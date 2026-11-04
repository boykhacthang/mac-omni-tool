//! Apple Silicon GPU metrics reader via IOKit `IOAccelerator` (`ioreg`).

use crate::models::GpuInfo;
use std::process::Command;

/// Extract an unsigned 64-bit integer property from `ioreg` output text.
pub fn extract_ioreg_u64(text: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{}\"", key);
    let idx = text.find(&needle)?;
    let rest = &text[idx + needle.len()..];
    let eq = rest.find('=')?;
    let after_eq = rest[eq + 1..].trim_start();
    let digits: String = after_eq.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().ok()
}

/// Extract a quoted string property from `ioreg` output text.
pub fn extract_ioreg_str(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let idx = text.find(&needle)?;
    let rest = &text[idx + needle.len()..];
    let eq = rest.find('=')?;
    let after_eq = rest[eq + 1..].trim_start();
    let q_start = after_eq.find('"')?;
    let after_q = &after_eq[q_start + 1..];
    let q_end = after_q.find('"')?;
    Some(after_q[..q_end].trim().to_string())
}

/// Parse raw `ioreg -r -d 1 -w 0 -c IOAccelerator` output into `GpuInfo`.
pub fn parse_ioreg_gpu_output(
    text: &str,
    default_model: &str,
    gpu_temp_c: f32,
    total_ram_bytes: u64,
) -> GpuInfo {
    let mut name = default_model.to_string();
    let mut cores = 16u32;
    let mut metal_support = "Metal (Unified Memory)".to_string();
    let mut usage_percent = 0.0f32;
    let mut renderer_percent = 0.0f32;
    let mut tiler_percent = 0.0f32;
    let mut in_use_memory_bytes = 0u64;
    let mut alloc_memory_bytes = 0u64;

    if let Some(m) = extract_ioreg_str(text, "model") {
        if !m.is_empty() {
            name = m;
        }
    }
    if let Some(metal_plugin) = extract_ioreg_str(text, "MetalPluginName") {
        if metal_plugin.contains("G16") || metal_plugin.contains("G15") {
            metal_support = "Metal 4 • Unified Memory".to_string();
        } else if !metal_plugin.is_empty() {
            metal_support = format!("Metal ({})", metal_plugin);
        }
    }
    if let Some(c) = extract_ioreg_u64(text, "gpu-core-count") {
        cores = c as u32;
    }
    if let Some(dev_util) = extract_ioreg_u64(text, "Device Utilization %") {
        usage_percent = dev_util as f32;
    }
    if let Some(rend_util) = extract_ioreg_u64(text, "Renderer Utilization %") {
        renderer_percent = rend_util as f32;
        if renderer_percent > usage_percent {
            usage_percent = renderer_percent;
        }
    }
    if let Some(tile_util) = extract_ioreg_u64(text, "Tiler Utilization %") {
        tiler_percent = tile_util as f32;
    }
    if let Some(in_use) = extract_ioreg_u64(text, "In use system memory") {
        in_use_memory_bytes = in_use;
    }
    if let Some(alloc) = extract_ioreg_u64(text, "Alloc system memory") {
        alloc_memory_bytes = alloc;
    }

    // On Apple Silicon Unified Memory Architecture (UMA), Metal allows the GPU to dynamically use up to ~75% of system RAM
    let max_recommended_memory_bytes = (total_ram_bytes as f64 * 0.75) as u64;

    GpuInfo {
        name,
        cores,
        metal_support,
        usage_percent: (usage_percent * 10.0).round() / 10.0,
        renderer_percent: (renderer_percent * 10.0).round() / 10.0,
        tiler_percent: (tiler_percent * 10.0).round() / 10.0,
        in_use_memory_bytes,
        alloc_memory_bytes,
        max_recommended_memory_bytes,
        temp_c: gpu_temp_c.round(),
    }
}

/// Read Apple Silicon GPU metrics from IOKit `IOAccelerator` (`AGXAccelerator`).
pub fn read_gpu_info(default_model: &str, gpu_temp_c: f32, total_ram_bytes: u64) -> GpuInfo {
    if let Ok(out) = Command::new("/usr/sbin/ioreg")
        .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return parse_ioreg_gpu_output(&text, default_model, gpu_temp_c, total_ram_bytes);
        }
    }
    parse_ioreg_gpu_output("", default_model, gpu_temp_c, total_ram_bytes)
}

