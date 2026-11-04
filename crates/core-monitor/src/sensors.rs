//! Apple Silicon thermal sensor (`IOHIDEventSystemClient`) and battery (`pmset`) readers.

use crate::ffi::*;
use crate::models::{BatteryInfo, TempSensorReading, TemperatureSummary};
use std::ffi::CString;
use std::process::Command;

/// Categorize an Apple Silicon sensor name into `"GPU"`, `"CPU/SoC"`, `"Battery/PMU"`, or `"Storage/Other"`.
pub fn classify_sensor_category(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("gpu")
        || lower.contains("gxf")
        || lower.starts_with("tg")
        || lower.contains(" tg")
    {
        "GPU"
    } else if lower.contains("tdie")
        || lower.contains("pacc")
        || lower.contains("eacc")
        || lower.contains("soc")
        || lower.contains("cpu")
        || lower.contains("mtr")
        || lower.starts_with("tp")
    {
        "CPU/SoC"
    } else if lower.contains("gas gauge") || lower.contains("batt") || lower.contains("pmu") {
        "Battery/PMU"
    } else {
        "Storage/Other"
    }
}

/// Read real hardware temperatures from Apple Silicon `IOHIDEventSystemClient` sensors.
/// Wrapped in an Objective-C autorelease pool so background threads never leak temporary CF/ObjC objects.
pub fn read_hardware_temperatures() -> TemperatureSummary {
    let mut sensors = Vec::new();

    unsafe {
        let pool = objc_autoreleasePoolPush();
        let client = IOHIDEventSystemClientCreate(std::ptr::null());
        if !client.is_null() {
            let services = IOHIDEventSystemClientCopyServices(client);
            if !services.is_null() {
                let count = CFArrayGetCount(services);
                let key_cstr = CString::new("Product").unwrap();
                let key_cf = CFStringCreateWithCString(
                    std::ptr::null(),
                    key_cstr.as_ptr(),
                    K_CF_STRING_ENCODING_UTF8,
                );

                for i in 0..count {
                    let service = CFArrayGetValueAtIndex(services, i);
                    if service.is_null() {
                        continue;
                    }
                    let event =
                        IOHIDServiceClientCopyEvent(service, K_IOHID_EVENT_TYPE_TEMPERATURE, 0, 0);
                    if !event.is_null() {
                        let temp =
                            IOHIDEventGetFloatValue(event, K_IOHID_EVENT_FIELD_TEMPERATURE_LEVEL)
                                as f32;
                        CFRelease(event);

                        if (15.0..=125.0).contains(&temp) {
                            let prop = IOHIDServiceClientCopyProperty(service, key_cf);
                            let name = cfstring_to_rust(prop)
                                .unwrap_or_else(|| format!("Sensor #{}", i + 1));
                            if !prop.is_null() {
                                CFRelease(prop);
                            }

                            let category = classify_sensor_category(&name).to_string();
                            sensors.push(TempSensorReading {
                                name,
                                celsius: temp.round(),
                                category,
                            });
                        }
                    }
                }

                if !key_cf.is_null() {
                    CFRelease(key_cf);
                }
                CFRelease(services);
            }
            CFRelease(client);
        }
        objc_autoreleasePoolPop(pool);
    }

    summarize_sensor_readings(sensors)
}

/// Deduplicate sensor names, sort descending by temperature, and compute category averages.
pub fn summarize_sensor_readings(mut sensors: Vec<TempSensorReading>) -> TemperatureSummary {
    sensors.sort_by(|a, b| {
        b.celsius
            .partial_cmp(&a.celsius)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut unique_sensors: Vec<TempSensorReading> = Vec::new();
    for s in sensors {
        if !unique_sensors.iter().any(|u| u.name == s.name) {
            unique_sensors.push(s);
        }
    }

    let avg_for = |cat: &str| -> Option<f32> {
        let vals: Vec<f32> = unique_sensors
            .iter()
            .filter(|s| s.category == cat)
            .map(|s| s.celsius)
            .collect();
        if vals.is_empty() {
            None
        } else {
            let sum: f32 = vals.iter().sum();
            Some((sum / vals.len() as f32).round())
        }
    };

    let overall_avg = if unique_sensors.is_empty() {
        42.0
    } else {
        let sum: f32 = unique_sensors.iter().map(|s| s.celsius).sum();
        (sum / unique_sensors.len() as f32).round()
    };

    let cpu_temp_c = avg_for("CPU/SoC").unwrap_or(overall_avg).round();
    let gpu_temp_c = avg_for("GPU").unwrap_or((cpu_temp_c - 2.0).max(30.0)).round();
    let battery_temp_c = avg_for("Battery/PMU").unwrap_or(34.0).round();
    let max_temp_c = unique_sensors
        .first()
        .map(|s| s.celsius.round())
        .unwrap_or(cpu_temp_c);

    unique_sensors.truncate(16);

    TemperatureSummary {
        cpu_temp_c,
        gpu_temp_c,
        battery_temp_c,
        max_temp_c,
        sensors: unique_sensors,
    }
}

/// Parse `pmset -g batt` stdout into `BatteryInfo`.
pub fn parse_pmset_battery_output(text: &str) -> BatteryInfo {
    let power_source = if text.contains("AC Power") {
        "AC Power (Đang cắm sạc)".to_string()
    } else {
        "Battery Power (Dùng Pin)".to_string()
    };
    for line in text.lines() {
        if let Some(pct_idx) = line.find('%') {
            let prefix = &line[..pct_idx];
            let digits: String = prefix
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if let Ok(pct) = digits.parse::<u8>() {
                let is_charging = line.contains("; charging") || line.contains("; charged");
                return BatteryInfo {
                    has_battery: true,
                    percentage: pct,
                    is_charging,
                    power_source,
                };
            }
        }
    }
    BatteryInfo {
        has_battery: false,
        percentage: 100,
        is_charging: true,
        power_source: "AC Power".to_string(),
    }
}

/// Read battery status via `pmset -g batt`.
pub fn read_battery_info() -> BatteryInfo {
    if let Ok(out) = Command::new("pmset").args(["-g", "batt"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        return parse_pmset_battery_output(&text);
    }
    BatteryInfo {
        has_battery: false,
        percentage: 100,
        is_charging: true,
        power_source: "AC Power".to_string(),
    }
}

