//! `core-monitor`: Real-time Apple Silicon hardware telemetry, temperature sensors (`IOHIDEventSystemClient`),
//! GPU metrics (`IOAccelerator`), battery status, and zero-leak process management.

pub mod dashboard;
pub mod ffi;
pub mod gpu;
pub mod models;
pub mod processes;
pub mod sensors;

pub use dashboard::{get_hardware_dashboard, get_menubar_stats, release_dashboard_memory};
pub use gpu::{extract_ioreg_str, extract_ioreg_u64, parse_ioreg_gpu_output, read_gpu_info};
pub use models::{
    BatteryInfo, GpuInfo, HardwareDashboardData, MenuBarQuickStats, ProcessEntry,
    TempSensorReading, TemperatureSummary,
};
pub use processes::{
    collect_top_processes, kill_process, light_process_refresh_kind, normalize_process_sort_key,
    sort_process_entries,
};
pub use sensors::{
    classify_sensor_category, parse_pmset_battery_output, read_battery_info,
    read_hardware_temperatures, summarize_sensor_readings,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_sensor_category() {
        assert_eq!(classify_sensor_category("GPU MTR Temp Sensor0"), "GPU");
        assert_eq!(classify_sensor_category("PMU tdie1"), "CPU/SoC");
        assert_eq!(classify_sensor_category("pACC MTR Temp Sensor2"), "CPU/SoC");
        assert_eq!(classify_sensor_category("gas gauge battery"), "Battery/PMU");
        assert_eq!(classify_sensor_category("NAND CH0 temp"), "Storage/Other");
    }

    #[test]
    fn test_summarize_sensor_readings_dedup_and_avg() {
        let readings = vec![
            TempSensorReading {
                name: "PMU tdie1".to_string(),
                celsius: 50.0,
                category: "CPU/SoC".to_string(),
            },
            TempSensorReading {
                name: "PMU tdie1".to_string(), // duplicate name
                celsius: 48.0,
                category: "CPU/SoC".to_string(),
            },
            TempSensorReading {
                name: "PMU tdie2".to_string(),
                celsius: 60.0,
                category: "CPU/SoC".to_string(),
            },
            TempSensorReading {
                name: "GPU MTR Temp Sensor0".to_string(),
                celsius: 46.0,
                category: "GPU".to_string(),
            },
        ];
        let summary = summarize_sensor_readings(readings);
        assert_eq!(summary.sensors.len(), 3);
        assert_eq!(summary.cpu_temp_c, 55.0);
        assert_eq!(summary.gpu_temp_c, 46.0);
        assert_eq!(summary.max_temp_c, 60.0);
    }

    #[test]
    fn test_parse_pmset_battery_output() {
        let sample = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=123)\t84%; discharging; 4:12 remaining present: true\n";
        let batt = parse_pmset_battery_output(sample);
        assert!(batt.has_battery);
        assert_eq!(batt.percentage, 84);
        assert!(!batt.is_charging);
        assert!(batt.power_source.contains("Battery Power"));

        let sample_ac = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=123)\t100%; charged; 0:00 remaining present: true\n";
        let batt_ac = parse_pmset_battery_output(sample_ac);
        assert!(batt_ac.has_battery);
        assert_eq!(batt_ac.percentage, 100);
        assert!(batt_ac.is_charging);
    }

    #[test]
    fn test_parse_ioreg_gpu_output() {
        let sample = r#"
            "model" = "Apple M4"
            "MetalPluginName" = "AGXMetalG16G"
            "gpu-core-count" = 10
            "PerformanceStatistics" = {"Device Utilization %"=18,"Renderer Utilization %"=22,"Tiler Utilization %"=9,"In use system memory"=1073741824,"Alloc system memory"=2147483648}
        "#;
        let gpu = parse_ioreg_gpu_output(sample, "Apple Silicon", 45.0, 16 * 1024 * 1024 * 1024);
        assert_eq!(gpu.name, "Apple M4");
        assert_eq!(gpu.cores, 10);
        assert_eq!(gpu.metal_support, "Metal 4 • Unified Memory");
        assert_eq!(gpu.usage_percent, 22.0);
        assert_eq!(gpu.renderer_percent, 22.0);
        assert_eq!(gpu.tiler_percent, 9.0);
        assert_eq!(gpu.in_use_memory_bytes, 1_073_741_824);
        assert_eq!(gpu.alloc_memory_bytes, 2_147_483_648);
        assert_eq!(gpu.max_recommended_memory_bytes, 12 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_process_sorting_and_kill_guards() {
        let mut procs = vec![
            ProcessEntry {
                pid: 100,
                name: "beta".to_string(),
                exe_path: String::new(),
                memory_bytes: 1000,
                memory_mb: 1.0,
                memory_percent: 1.0,
                cpu_percent: 50.0,
                disk_read_bytes: 10,
                disk_written_bytes: 10,
                status: "Run".to_string(),
            },
            ProcessEntry {
                pid: 200,
                name: "alpha".to_string(),
                exe_path: String::new(),
                memory_bytes: 5000,
                memory_mb: 5.0,
                memory_percent: 5.0,
                cpu_percent: 5.0,
                disk_read_bytes: 1000,
                disk_written_bytes: 2000,
                status: "Run".to_string(),
            },
        ];

        sort_process_entries(&mut procs, "ram");
        assert_eq!(procs[0].pid, 200);

        sort_process_entries(&mut procs, "cpu");
        assert_eq!(procs[0].pid, 100);

        sort_process_entries(&mut procs, "disk");
        assert_eq!(procs[0].pid, 200);

        sort_process_entries(&mut procs, "name");
        assert_eq!(procs[0].name, "alpha");

        // Guard against killing PID <= 1 or self PID
        assert!(kill_process(0, true).is_err());
        assert!(kill_process(1, false).is_err());
        assert!(kill_process(std::process::id(), true).is_err());
    }
}
