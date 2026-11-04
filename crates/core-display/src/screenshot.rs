//! Mouse cursor display detection and multi-monitor parallel PNG screenshot capture.

use crate::brightness::list_displays;
use crate::ffi::{CFRelease, CGEventCreate, CGEventGetLocation};
use crate::models::DisplayInfo;
use std::process::Command;

/// Get current global mouse cursor position in CoreGraphics logical display coordinates.
pub fn get_mouse_cursor_position() -> (f64, f64) {
    unsafe {
        let ev = CGEventCreate(std::ptr::null());
        if ev.is_null() {
            return (0.0, 0.0);
        }
        let pt = CGEventGetLocation(ev);
        CFRelease(ev);
        (pt.x, pt.y)
    }
}

/// Find which display in `displays` contains `(mx, my)`, falling back to the main display.
pub fn find_display_containing_point(displays: &[DisplayInfo], mx: f64, my: f64) -> DisplayInfo {
    for d in displays {
        let dx = d.x as f64;
        let dy = d.y as f64;
        let dw = d.width as f64;
        let dh = d.height as f64;
        if mx >= dx && mx < dx + dw && my >= dy && my < dy + dh {
            return d.clone();
        }
    }
    displays
        .iter()
        .find(|d| d.is_main)
        .cloned()
        .or_else(|| displays.first().cloned())
        .unwrap_or(DisplayInfo {
            id: 1,
            name: "Main Display".to_string(),
            width: 1440,
            height: 900,
            x: 0,
            y: 0,
            is_main: true,
            is_builtin: true,
            vendor_id: 0,
            model_id: 0,
            brightness: 1.0,
            software_dimming: 1.0,
            warmth: 0.0,
            supports_hardware_brightness: true,
        })
}

/// Return the `DisplayInfo` of the monitor that currently contains the mouse cursor.
pub fn get_display_at_mouse_cursor() -> DisplayInfo {
    let (mx, my) = get_mouse_cursor_position();
    let displays = list_displays();
    find_display_containing_point(&displays, mx, my)
}

/// Capture an instant silent PNG screenshot of the specified display (Retina native resolution).
pub fn capture_display_png(display: &DisplayInfo) -> Result<Vec<u8>, String> {
    let tmp_dir = std::env::temp_dir().join("mac_omni_screenshots");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let tmp_file = tmp_dir.join(format!("active_screen_capture_{}.png", display.id));
    let _ = std::fs::remove_file(&tmp_file);

    let rect_arg = format!(
        "{},{},{},{}",
        display.x, display.y, display.width, display.height
    );

    let status = Command::new("/usr/sbin/screencapture")
        .args([
            "-x",
            "-R",
            &rect_arg,
            "-t",
            "png",
            &tmp_file.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("Không thể gọi screencapture: {}", e))?;

    if !status.success() {
        return Err(
            "Không thể chụp màn hình. Vui lòng cấp quyền Screen Recording trong Cài đặt."
                .to_string(),
        );
    }

    let bytes = std::fs::read(&tmp_file)
        .map_err(|e| format!("Không đọc được dữ liệu ảnh chụp màn hình: {}", e))?;
    let _ = std::fs::remove_file(&tmp_file);

    if bytes.is_empty() {
        return Err("Ảnh chụp màn hình trống (kiểm tra quyền Screen Recording).".to_string());
    }
    Ok(bytes)
}

/// Simultaneously capture silent PNG screenshots of all connected displays in parallel.
pub fn capture_all_displays_png() -> Result<Vec<(DisplayInfo, Vec<u8>)>, String> {
    let displays = list_displays();
    if displays.is_empty() {
        let fallback = get_display_at_mouse_cursor();
        let bytes = capture_display_png(&fallback)?;
        return Ok(vec![(fallback, bytes)]);
    }

    let results: Vec<Result<(DisplayInfo, Vec<u8>), String>> = std::thread::scope(|s| {
        let handles: Vec<_> = displays
            .iter()
            .map(|disp| {
                let d = disp.clone();
                s.spawn(move || {
                    let bytes = capture_display_png(&d)?;
                    Ok((d, bytes))
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("Lỗi luồng chụp màn hình".to_string()))
            })
            .collect()
    });

    let mut out = Vec::with_capacity(results.len());
    let mut last_err = String::new();
    for res in results {
        match res {
            Ok(pair) => out.push(pair),
            Err(e) => last_err = e,
        }
    }

    if out.is_empty() {
        return Err(if last_err.is_empty() {
            "Không thể chụp màn hình. Vui lòng kiểm tra quyền Screen Recording.".to_string()
        } else {
            last_err
        });
    }
    Ok(out)
}

