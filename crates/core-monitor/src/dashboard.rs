//! Stateful hardware monitor orchestrating CPU, RAM, Disk, Network, GPU, Sensors, Battery, and Processes
//! with bounded O(1) memory usage.

use crate::gpu::read_gpu_info;
use crate::models::{BatteryInfo, HardwareDashboardData, MenuBarQuickStats};
use crate::processes::{collect_top_processes, light_process_refresh_kind};
use crate::sensors::{read_battery_info, read_hardware_temperatures};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

struct MonitorState {
    sys: System,
    disks: Disks,
    networks: Networks,
    last_net_check: Instant,
    cached_net_rx_rate: u64,
    cached_net_tx_rate: u64,
    cached_total_rx: u64,
    cached_total_tx: u64,
    last_battery_check: Instant,
    cached_battery: Option<BatteryInfo>,
}

static MONITOR: OnceLock<Mutex<MonitorState>> = OnceLock::new();

fn get_monitor() -> &'static Mutex<MonitorState> {
    MONITOR.get_or_init(|| {
        // Initialize only CPU, RAM, and Networks on startup.
        // Do NOT populate the ~1,000-process table or disk list until the Hardware Monitor dashboard is actually opened!
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let disks = Disks::new();
        let networks = Networks::new_with_refreshed_list();
        Mutex::new(MonitorState {
            sys,
            disks,
            networks,
            last_net_check: Instant::now(),
            cached_net_rx_rate: 0,
            cached_net_tx_rate: 0,
            cached_total_rx: 0,
            cached_total_tx: 0,
            last_battery_check: Instant::now(),
            cached_battery: None,
        })
    })
}

/// Drop the cached ~1,000-process table and disk list when the main dashboard window is hidden to background,
/// keeping background Menu Bar telemetry (`get_menubar_stats`) ultra-lightweight.
pub fn release_dashboard_memory() {
    if let Some(lock) = MONITOR.get() {
        if let Ok(mut guard) = lock.lock() {
            if !guard.sys.processes().is_empty() {
                let mut light_sys = System::new();
                light_sys.refresh_cpu_usage();
                light_sys.refresh_memory();
                guard.sys = light_sys;
            }
            guard.disks = Disks::new();
            guard.cached_battery = None;
        }
    }
}

fn refresh_network_rates(guard: &mut MonitorState) -> (u64, u64, u64, u64) {
    let elapsed_secs = guard.last_net_check.elapsed().as_secs_f64();
    if elapsed_secs >= 0.75 || (guard.cached_total_rx == 0 && guard.cached_total_tx == 0) {
        let dt = elapsed_secs.max(0.25);
        guard.last_net_check = Instant::now();
        guard.networks.refresh();
        let mut rx_delta = 0u64;
        let mut tx_delta = 0u64;
        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        for (_name, net) in &guard.networks {
            rx_delta += net.received();
            tx_delta += net.transmitted();
            total_rx += net.total_received();
            total_tx += net.total_transmitted();
        }
        guard.cached_net_rx_rate = (rx_delta as f64 / dt) as u64;
        guard.cached_net_tx_rate = (tx_delta as f64 / dt) as u64;
        guard.cached_total_rx = total_rx;
        guard.cached_total_tx = total_tx;
    }
    (
        guard.cached_net_rx_rate,
        guard.cached_net_tx_rate,
        guard.cached_total_rx,
        guard.cached_total_tx,
    )
}

/// Ultra-lightweight stats poll for the Menu Bar status item (`❖`).
/// Refreshes only global CPU, RAM, Network I/O, and CPU/GPU temperature without enumerating processes or spawning external commands.
pub fn get_menubar_stats() -> MenuBarQuickStats {
    let (cpu_usage, used_mem, total_mem, mem_pct, net_rx, net_tx) = {
        let mut guard = get_monitor().lock().unwrap();
        guard.sys.refresh_cpu_usage();
        guard.sys.refresh_memory();
        let total_mem = guard.sys.total_memory().max(1);
        let avail_mem = guard.sys.available_memory();
        let used_mem = total_mem
            .saturating_sub(avail_mem)
            .max(guard.sys.used_memory());
        let mem_pct = ((used_mem as f64 / total_mem as f64) * 100.0) as f32;
        let (rx_rate, tx_rate, _, _) = refresh_network_rates(&mut guard);
        (
            guard.sys.global_cpu_usage(),
            used_mem,
            total_mem,
            mem_pct,
            rx_rate,
            tx_rate,
        )
    };
    let temps = read_hardware_temperatures();
    MenuBarQuickStats {
        cpu_usage_percent: (cpu_usage * 10.0).round() / 10.0,
        memory_usage_percent: (mem_pct * 10.0).round() / 10.0,
        used_memory_bytes: used_mem,
        total_memory_bytes: total_mem,
        cpu_temp_c: temps.cpu_temp_c,
        gpu_temp_c: temps.gpu_temp_c,
        net_rx_bytes_per_sec: net_rx,
        net_tx_bytes_per_sec: net_tx,
    }
}

/// Snapshot hardware stats and top processes sorted by `"ram"` (default), `"cpu"`, `"disk"`, or `"name"`.
/// Automatically purges dead processes on every tick so RAM usage never grows over time.
pub fn get_hardware_dashboard(sort_by: &str, limit: usize) -> HardwareDashboardData {
    let mut guard = get_monitor().lock().unwrap();

    guard.sys.refresh_cpu_all();
    guard.sys.refresh_memory();
    // `ProcessesToUpdate::All` automatically removes terminated processes from `sysinfo`'s map
    guard
        .sys
        .refresh_processes_specifics(ProcessesToUpdate::All, light_process_refresh_kind());
    guard.disks.refresh_list();
    guard.disks.refresh();

    let total_mem = guard.sys.total_memory().max(1);
    let avail_mem = guard.sys.available_memory();
    let used_mem = total_mem
        .saturating_sub(avail_mem)
        .max(guard.sys.used_memory());
    let mem_pct = ((used_mem as f64 / total_mem as f64) * 100.0) as f32;

    let cpu_brand = guard
        .sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Apple Silicon".to_string());
    let cpu_cores = guard.sys.cpus().len();
    let cpu_usage = guard.sys.global_cpu_usage();
    let per_core_usage: Vec<f32> = guard.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

    // Primary disk (/)
    let (disk_total, disk_avail) = guard
        .disks
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"))
        .or_else(|| guard.disks.iter().next())
        .map(|d| (d.total_space(), d.available_space()))
        .unwrap_or((1, 0));
    let disk_used = disk_total.saturating_sub(disk_avail);
    let disk_pct = ((disk_used as f64 / disk_total.max(1) as f64) * 100.0) as f32;

    // Network totals & rates
    let (net_rx_rate, net_tx_rate, total_rx, total_tx) = refresh_network_rates(&mut guard);

    let (processes, normalized_sort) =
        collect_top_processes(&guard.sys, total_mem, sort_by, limit);

    let host_name = System::host_name().unwrap_or_else(|| "Mac".to_string());
    let os_version = format!(
        "macOS {}",
        System::os_version().unwrap_or_else(|| "".to_string())
    );
    let uptime = System::uptime();

    // Cache battery info for 10 seconds to avoid spawning `pmset` on every 2-second poll
    let battery = if guard.cached_battery.is_none()
        || guard.last_battery_check.elapsed() >= Duration::from_secs(10)
    {
        let b = read_battery_info();
        guard.cached_battery = Some(b.clone());
        guard.last_battery_check = Instant::now();
        b
    } else {
        guard.cached_battery.clone().unwrap()
    };

    drop(guard);

    let temperatures = read_hardware_temperatures();
    let gpu = read_gpu_info(&cpu_brand, temperatures.gpu_temp_c, total_mem);

    HardwareDashboardData {
        host_name,
        os_version,
        cpu_brand,
        cpu_cores,
        cpu_usage_percent: (cpu_usage * 10.0).round() / 10.0,
        per_core_usage,
        gpu,
        total_memory_bytes: total_mem,
        used_memory_bytes: used_mem,
        available_memory_bytes: avail_mem,
        memory_usage_percent: (mem_pct * 10.0).round() / 10.0,
        total_swap_bytes: 0,
        used_swap_bytes: 0,
        disk_total_bytes: disk_total,
        disk_used_bytes: disk_used,
        disk_available_bytes: disk_avail,
        disk_usage_percent: (disk_pct * 10.0).round() / 10.0,
        net_rx_bytes_per_sec: net_rx_rate,
        net_tx_bytes_per_sec: net_tx_rate,
        net_total_rx_bytes: total_rx,
        net_total_tx_bytes: total_tx,
        uptime_seconds: uptime,
        temperatures,
        battery,
        top_processes: processes,
        active_sort: normalized_sort.to_string(),
    }
}

