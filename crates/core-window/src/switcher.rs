//! Independent Alt-Tab / Cmd-Tab window enumeration, live thumbnail caching, and window activation.

use crate::ffi::*;
use crate::models::OpenWindowItem;
use std::ffi::c_void;
use std::fs;
use std::process::Command;

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize]);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize]);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize]
        } else {
            b'='
        });
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Capture live preview thumbnail for a specific `CGWindowID`.
fn capture_window_thumbnail_data_url(window_id: u32) -> String {
    let tmp_dir = std::env::temp_dir().join("mac_omni_previews");
    let _ = fs::create_dir_all(&tmp_dir);
    let file_path = tmp_dir.join(format!("win_{}.jpg", window_id));

    // Reuse recent preview if < 3 seconds old for instant Alt-Tab response
    if let Ok(meta) = fs::metadata(&file_path) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().map(|d| d.as_secs() < 3).unwrap_or(false) {
                if let Ok(bytes) = fs::read(&file_path) {
                    if !bytes.is_empty() {
                        return format!("data:image/jpeg;base64,{}", base64_encode(&bytes));
                    }
                }
            }
        }
    }

    let status = Command::new("/usr/sbin/screencapture")
        .args([
            "-l",
            &window_id.to_string(),
            "-x",
            "-o",
            "-t",
            "jpg",
            &file_path.to_string_lossy(),
        ])
        .status();

    if status.map(|s| s.success()).unwrap_or(false) {
        if let Ok(bytes) = fs::read(&file_path) {
            if !bytes.is_empty() {
                return format!("data:image/jpeg;base64,{}", base64_encode(&bytes));
            }
        }
    }
    String::new()
}

/// Enumerate independent application windows for Alt-Tab replacement with optional Live Preview.
pub fn list_open_windows(with_previews: bool) -> Vec<OpenWindowItem> {
    let mut result = Vec::new();
    let my_pid = std::process::id() as i32;

    unsafe {
        // kCGWindowListOptionOnScreenOnly (1) | kCGWindowListExcludeDesktopElements (16) = 17
        let info_array = CGWindowListCopyWindowInfo(17, 0);
        if info_array.is_null() {
            return result;
        }

        let count = CFArrayGetCount(info_array);
        let k_win_num = make_cfstring("kCGWindowNumber");
        let k_owner_pid = make_cfstring("kCGWindowOwnerPID");
        let k_owner_name = make_cfstring("kCGWindowOwnerName");
        let k_win_name = make_cfstring("kCGWindowName");
        let k_win_layer = make_cfstring("kCGWindowLayer");
        let k_win_bounds = make_cfstring("kCGWindowBounds");

        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(info_array, i);
            if dict.is_null() {
                continue;
            }

            let mut layer: i32 = 0;
            let layer_ref = CFDictionaryGetValue(dict, k_win_layer);
            if !layer_ref.is_null() {
                CFNumberGetValue(
                    layer_ref,
                    K_CF_NUMBER_SINT32_TYPE,
                    &mut layer as *mut _ as *mut c_void,
                );
            }
            if layer != 0 {
                continue;
            }

            let mut owner_pid: i32 = 0;
            let pid_ref = CFDictionaryGetValue(dict, k_owner_pid);
            if !pid_ref.is_null() {
                CFNumberGetValue(
                    pid_ref,
                    K_CF_NUMBER_SINT32_TYPE,
                    &mut owner_pid as *mut _ as *mut c_void,
                );
            }
            if owner_pid == my_pid {
                continue;
            }

            let mut window_id: i64 = 0;
            let num_ref = CFDictionaryGetValue(dict, k_win_num);
            if !num_ref.is_null() {
                CFNumberGetValue(
                    num_ref,
                    K_CF_NUMBER_SINT64_TYPE,
                    &mut window_id as *mut _ as *mut c_void,
                );
            }

            let mut bounds = CGRect::default();
            let bounds_ref = CFDictionaryGetValue(dict, k_win_bounds);
            if !bounds_ref.is_null() {
                CGRectMakeWithDictionaryRepresentation(bounds_ref, &mut bounds);
            }

            if bounds.size.width < 120.0 || bounds.size.height < 80.0 {
                continue;
            }

            let app_name = cfstring_to_string(CFDictionaryGetValue(dict, k_owner_name));
            if app_name == "Window Server"
                || app_name == "Dock"
                || app_name == "SystemUIServer"
                || app_name == "Control Center"
            {
                continue;
            }

            let raw_title = cfstring_to_string(CFDictionaryGetValue(dict, k_win_name));
            let title = if raw_title.trim().is_empty() {
                format!("{} — Cửa sổ #{}", app_name, window_id)
            } else {
                raw_title
            };

            let preview_data_url = if with_previews && result.len() < 12 {
                capture_window_thumbnail_data_url(window_id as u32)
            } else {
                String::new()
            };

            result.push(OpenWindowItem {
                window_id: window_id as u32,
                owner_pid,
                app_name,
                title,
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
                preview_data_url,
            });
        }

        CFRelease(k_win_num);
        CFRelease(k_owner_pid);
        CFRelease(k_owner_name);
        CFRelease(k_win_name);
        CFRelease(k_win_layer);
        CFRelease(k_win_bounds);
        CFRelease(info_array);
    }

    result
}

/// Activate and bring a specific window to the front.
pub fn focus_window(window_id: u32, owner_pid: i32) -> Result<(), String> {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        // 1. Activate owning NSRunningApplication
        let cls = objc_getClass(c"NSRunningApplication".as_ptr());
        if !cls.is_null() {
            type MsgPid = unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> *mut c_void;
            type MsgAct = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> bool;
            let msg_pid: MsgPid = std::mem::transmute(objc_msgSend as *const c_void);
            let msg_act: MsgAct = std::mem::transmute(objc_msgSend as *const c_void);
            let sel_with_pid =
                sel_registerName(c"runningApplicationWithProcessIdentifier:".as_ptr());
            let sel_activate = sel_registerName(c"activateWithOptions:".as_ptr());
            let app = msg_pid(cls, sel_with_pid, owner_pid);
            if !app.is_null() {
                let _ = msg_act(app, sel_activate, 3);
            }
        }
        objc_autoreleasePoolPop(pool);

        // 2. Raise the specific window via Accessibility API
        if AXIsProcessTrusted() {
            let ax_app = AXUIElementCreateApplication(owner_pid);
            if !ax_app.is_null() {
                let attr_wins = make_cfstring("AXWindows");
                let action_raise = make_cfstring("AXRaise");
                let mut wins_ref: CFTypeRef = std::ptr::null();
                if AXUIElementCopyAttributeValue(ax_app, attr_wins, &mut wins_ref) == 0
                    && !wins_ref.is_null()
                {
                    let count = CFArrayGetCount(wins_ref);
                    for i in 0..count {
                        let win = CFArrayGetValueAtIndex(wins_ref, i);
                        if !win.is_null() {
                            AXUIElementPerformAction(win, action_raise);
                            break;
                        }
                    }
                    CFRelease(wins_ref);
                }
                CFRelease(attr_wins);
                CFRelease(action_raise);
                CFRelease(ax_app);
            }
        }
    }
    let _ = window_id;
    Ok(())
}

