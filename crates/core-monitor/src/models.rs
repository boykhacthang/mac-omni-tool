//! Data models for hardware telemetry, temperatures, GPU, battery, and running processes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TempSensorReading {
    pub name: String,
    pub celsius: f32,
    pub category: String, // "CPU/SoC", "GPU", "Battery/PMU", "Storage/Other"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemperatureSummary {
    pub cpu_temp_c: f32,
    pub gpu_temp_c: f32,
    pub battery_temp_c: f32,
    pub max_temp_c: f32,
    pub sensors: Vec<TempSensorReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub memory_bytes: u64,
    pub memory_mb: f64,
    pub memory_percent: f32,
    pub cpu_percent: f32,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatteryInfo {
    pub has_battery: bool,
    pub percentage: u8,
    pub is_charging: bool,
    pub power_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuInfo {
    pub name: String,
    pub cores: u32,
    pub metal_support: String,
    pub usage_percent: f32,
    pub renderer_percent: f32,
    pub tiler_percent: f32,
    pub in_use_memory_bytes: u64,
    pub alloc_memory_bytes: u64,
    pub max_recommended_memory_bytes: u64,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareDashboardData {
    pub host_name: String,
    pub os_version: String,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub cpu_usage_percent: f32,
    pub per_core_usage: Vec<f32>,
    pub gpu: GpuInfo,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub memory_usage_percent: f32,
    pub total_swap_bytes: u64,
    pub used_swap_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_available_bytes: u64,
    pub disk_usage_percent: f32,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub net_total_rx_bytes: u64,
    pub net_total_tx_bytes: u64,
    pub uptime_seconds: u64,
    pub temperatures: TemperatureSummary,
    pub battery: BatteryInfo,
    pub top_processes: Vec<ProcessEntry>,
    pub active_sort: String,
}

/// Ultra-lightweight stats payload for the Menu Bar status item (`❖`).
/// Avoids enumerating processes or spawning external commands when the dashboard is hidden.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MenuBarQuickStats {
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub used_memory_bytes: u64,
    pub total_memory_bytes: u64,
    pub cpu_temp_c: f32,
    #[serde(default)]
    pub gpu_temp_c: f32,
    #[serde(default)]
    pub net_rx_bytes_per_sec: u64,
    #[serde(default)]
    pub net_tx_bytes_per_sec: u64,
}

