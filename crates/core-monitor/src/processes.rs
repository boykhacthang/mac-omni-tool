//! Process enumeration, sorting, and safe `SIGTERM`/`SIGKILL` termination.

use crate::models::ProcessEntry;
use sysinfo::{ProcessRefreshKind, System, UpdateKind};

/// Lightweight process refresh kind that only updates CPU, RAM, Disk I/O, and caches `exe` path once,
/// avoiding heavy per-tick allocations of `cmd` or `environ` vectors.
pub fn light_process_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::new()
        .with_cpu()
        .with_memory()
        .with_disk_usage()
        .with_exe(UpdateKind::OnlyIfNotSet)
}

/// Normalize sort key to `"ram"`, `"cpu"`, `"disk"`, or `"name"`.
pub fn normalize_process_sort_key(sort_by: &str) -> &'static str {
    match sort_by.to_lowercase().as_str() {
        "cpu" => "cpu",
        "disk" => "disk",
        "name" => "name",
        _ => "ram",
    }
}

/// Sort a slice of `ProcessEntry` in-place according to `sort_by` (`"ram"`, `"cpu"`, `"disk"`, `"name"`).
pub fn sort_process_entries(processes: &mut [ProcessEntry], sort_by: &str) {
    match normalize_process_sort_key(sort_by) {
        "cpu" => processes.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
        }),
        "disk" => processes.sort_by(|a, b| {
            let a_io = a.disk_read_bytes + a.disk_written_bytes;
            let b_io = b.disk_read_bytes + b.disk_written_bytes;
            b_io.cmp(&a_io).then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
        }),
        "name" => processes.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
        }),
        _ => processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
    }
}

/// Query `ri_phys_footprint` via `proc_pid_rusage(RUSAGE_INFO_V2)` on macOS so process RAM
/// matches macOS Activity Monitor's "Memory" column (`phys_footprint`) instead of raw RSS
/// (which inflates GUI apps by ~55 MB of shared read-only dyld system framework pages).
#[cfg(target_os = "macos")]
fn query_macos_phys_footprint(pid: u32) -> Option<u64> {
    let mut rusage = std::mem::MaybeUninit::<libc::rusage_info_v2>::uninit();
    let ret = unsafe {
        libc::proc_pid_rusage(
            pid as i32,
            libc::RUSAGE_INFO_V2,
            rusage.as_mut_ptr() as *mut libc::rusage_info_t,
        )
    };
    if ret == 0 {
        let info = unsafe { rusage.assume_init() };
        if info.ri_phys_footprint > 0 {
            return Some(info.ri_phys_footprint);
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn query_macos_phys_footprint(_pid: u32) -> Option<u64> {
    None
}

/// Collect and sort the top `limit` processes from `sysinfo::System`.
pub fn collect_top_processes(
    sys: &System,
    total_mem: u64,
    sort_by: &str,
    limit: usize,
) -> (Vec<ProcessEntry>, &'static str) {
    let total_mem_safe = total_mem.max(1);
    let mut processes: Vec<ProcessEntry> = sys
        .processes()
        .iter()
        .filter_map(|(pid, proc_info)| {
            let pid_u32 = pid.as_u32();
            if pid_u32 == 0 {
                return None;
            }
            let name = proc_info.name().to_string_lossy().to_string();
            if name.is_empty() {
                return None;
            }
            let mem = query_macos_phys_footprint(pid_u32).unwrap_or_else(|| proc_info.memory());
            let disk = proc_info.disk_usage();
            let exe_path = proc_info
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            Some(ProcessEntry {
                pid: pid_u32,
                name,
                exe_path,
                memory_bytes: mem,
                memory_mb: ((mem as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
                memory_percent: (((mem as f64 / total_mem_safe as f64) * 100.0) * 10.0).round()
                    as f32
                    / 10.0,
                cpu_percent: (proc_info.cpu_usage() * 10.0).round() / 10.0,
                disk_read_bytes: disk.total_read_bytes,
                disk_written_bytes: disk.total_written_bytes,
                status: format!("{:?}", proc_info.status()),
            })
        })
        .collect();

    let normalized_sort = normalize_process_sort_key(sort_by);
    sort_process_entries(&mut processes, normalized_sort);

    let max_items = if limit == 0 { 10 } else { limit.min(100) };
    processes.truncate(max_items);
    (processes, normalized_sort)
}

/// Force quit (`SIGKILL`) or gracefully quit (`SIGTERM`) a running process by PID.
pub fn kill_process(pid: u32, force: bool) -> Result<String, String> {
    if pid <= 1 {
        return Err("Không thể tắt tiến trình cốt lõi của hệ thống (PID <= 1).".to_string());
    }
    let my_pid = std::process::id();
    if pid == my_pid {
        return Err("Không thể tự Force Quit chính ứng dụng Mac Omni Tool.".to_string());
    }

    let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
    let res = unsafe { libc::kill(pid as i32, sig) };
    if res == 0 {
        Ok(format!(
            "Đã gửi lệnh {} tới tiến trình PID {}.",
            if force {
                "Force Quit (SIGKILL)"
            } else {
                "Quit (SIGTERM)"
            },
            pid
        ))
    } else {
        let err = std::io::Error::last_os_error();
        Err(format!(
            "Không thể tắt PID {}: {} (Có thể là tiến trình root hệ thống).",
            pid, err
        ))
    }
}

