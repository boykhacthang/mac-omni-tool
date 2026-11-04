//! Window Tiling geometry calculator, self-window direct Cocoa snap callback, and external `AXUIElement` window snapping.

use crate::ffi::*;
use std::ffi::c_void;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static OWN_WINDOW_SNAP_CB: OnceLock<Mutex<Option<Box<dyn Fn(&str) + Send + Sync>>>> =
    OnceLock::new();

/// Register a callback invoked when the user triggers a window tiling layout (`Ctrl+Option+C`, etc.)
/// while `MacOmniTool` itself is the focused application (avoiding synchronous `AXUIElement` self-IPC deadlock).
pub fn register_own_window_snap_callback<F>(cb: F)
where
    F: Fn(&str) + Send + Sync + 'static,
{
    let lock = OWN_WINDOW_SNAP_CB.get_or_init(|| Mutex::new(None));
    *lock.lock().unwrap() = Some(Box::new(cb));
}

pub fn get_all_display_bounds() -> Vec<CGRect> {
    let mut ids = [0u32; 16];
    let mut count = 0u32;
    let mut rects = Vec::new();
    unsafe {
        if CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut count) == 0 {
            for &id in &ids[..count as usize] {
                rects.push(CGDisplayBounds(id));
            }
        }
    }
    if rects.is_empty() {
        rects.push(CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: 1440.0,
                height: 900.0,
            },
        });
    }
    rects
}

/// Return the height of the primary display (`origin == (0,0)`) for converting between Top-Left CG coordinates and Bottom-Left Cocoa coordinates.
pub fn primary_display_height() -> f64 {
    let displays = get_all_display_bounds();
    displays
        .iter()
        .find(|d| d.origin.x == 0.0 && d.origin.y == 0.0)
        .or_else(|| displays.first())
        .map(|d| d.size.height)
        .unwrap_or(900.0)
}

/// Pure function that computes `(nx, ny, nw, nh)` in top-left `CoreGraphics` coordinates for `layout` on `displays`.
pub fn compute_snap_rect_for_displays(
    displays: &[CGRect],
    layout: &str,
    cur_x: f64,
    cur_y: f64,
) -> (f64, f64, f64, f64) {
    let fallback = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 1440.0,
            height: 900.0,
        },
    };
    if displays.is_empty() {
        return compute_snap_rect_for_displays(&[fallback], layout, cur_x, cur_y);
    }

    let mut disp_idx = displays
        .iter()
        .position(|d| {
            cur_x >= d.origin.x
                && cur_x < d.origin.x + d.size.width
                && cur_y >= d.origin.y
                && cur_y < d.origin.y + d.size.height
        })
        .unwrap_or(0);

    if layout == "next_display" && displays.len() > 1 {
        disp_idx = (disp_idx + 1) % displays.len();
    }

    let screen = displays[disp_idx];
    let menu_bar_h = 25.0;
    let sx = screen.origin.x;
    let sy = screen.origin.y + menu_bar_h;
    let sw = screen.size.width;
    let sh = (screen.size.height - menu_bar_h).max(200.0);

    match layout {
        "left_half" => (sx, sy, (sw / 2.0).round(), sh),
        "right_half" => (
            sx + (sw / 2.0).round(),
            sy,
            (sw - (sw / 2.0).round()),
            sh,
        ),
        "top_half" => (sx, sy, sw, (sh / 2.0).round()),
        "bottom_half" => (
            sx,
            sy + (sh / 2.0).round(),
            sw,
            (sh - (sh / 2.0).round()),
        ),
        "top_left" => (sx, sy, (sw / 2.0).round(), (sh / 2.0).round()),
        "top_right" => (
            sx + (sw / 2.0).round(),
            sy,
            (sw - (sw / 2.0).round()),
            (sh / 2.0).round(),
        ),
        "bottom_left" => (
            sx,
            sy + (sh / 2.0).round(),
            (sw / 2.0).round(),
            (sh - (sh / 2.0).round()),
        ),
        "bottom_right" => (
            sx + (sw / 2.0).round(),
            sy + (sh / 2.0).round(),
            (sw - (sw / 2.0).round()),
            (sh - (sh / 2.0).round()),
        ),
        "two_thirds_left" => (sx, sy, (sw * 2.0 / 3.0).round(), sh),
        "one_third_right" => (
            sx + (sw * 2.0 / 3.0).round(),
            sy,
            (sw - (sw * 2.0 / 3.0).round()),
            sh,
        ),
        "center" => {
            let cw = (sw * 0.75).round();
            let ch = sh;
            (
                sx + ((sw - cw) / 2.0).round(),
                sy,
                cw,
                ch,
            )
        }
        "maximize" | "next_display" | _ => (sx, sy, sw, sh),
    }
}

/// Compute target `(x, y, width, height)` in top-left CoreGraphics screen coordinates for a given `layout`
/// starting from current point `(cur_x, cur_y)` on the active system displays.
pub fn compute_snap_rect(layout: &str, cur_x: f64, cur_y: f64) -> (f64, f64, f64, f64) {
    let displays = get_all_display_bounds();
    compute_snap_rect_for_displays(&displays, layout, cur_x, cur_y)
}

/// Return PID of the currently frontmost macOS application via `NSWorkspace.sharedWorkspace.frontmostApplication`.
pub fn frontmost_application_pid() -> i32 {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let ws_cls = objc_getClass(c"NSWorkspace".as_ptr());
        if ws_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return 0;
        }
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgPid = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_pid: MsgPid = std::mem::transmute(objc_msgSend as *const c_void);

        let ws = msg0(ws_cls, sel_registerName(c"sharedWorkspace".as_ptr()));
        if ws.is_null() {
            objc_autoreleasePoolPop(pool);
            return 0;
        }
        let app = msg0(ws, sel_registerName(c"frontmostApplication".as_ptr()));
        if app.is_null() {
            objc_autoreleasePoolPop(pool);
            return 0;
        }
        let pid = msg_pid(app, sel_registerName(c"processIdentifier".as_ptr()));
        objc_autoreleasePoolPop(pool);
        pid
    }
}

/// Fallback via AppleScript (`System Events`) when `AXIsProcessTrusted` is blocked after a binary rebuild.
fn snap_window_via_osascript_fallback(layout: &str) -> Result<String, String> {
    let (nx, ny, nw, nh) = compute_snap_rect(layout, 100.0, 100.0);
    let script = format!(
        r#"tell application "System Events"
            set frontProc to first application process whose frontmost is true
            tell front window of frontProc
                set position to {{{}, {}}}
                set size to {{{}, {}}}
            end tell
        end tell"#,
        nx as i32, ny as i32, nw as i32, nh as i32
    );
    if let Ok(out) = Command::new("osascript").args(["-e", &script]).output() {
        if out.status.success() {
            return Ok(format!(
                "Đã áp dụng bố cục '{}' ({}x{} tại {},{})",
                layout, nw as i32, nh as i32, nx as i32, ny as i32
            ));
        }
    }
    Err("Chưa cấp quyền Accessibility (Trợ năng) cho phiên bản mới. Vui lòng bấm 'Làm mới quyền' trong tab Cài đặt.".to_string())
}

/// Snap or tile the currently focused window to the specified layout.
pub fn snap_focused_window(layout: &str) -> Result<String, String> {
    let front_pid = frontmost_application_pid();
    let my_pid = std::process::id() as i32;

    // 1. If MacOmniTool itself is the frontmost app, snap our own main window directly via Cocoa!
    if front_pid == my_pid {
        if let Some(lock) = OWN_WINDOW_SNAP_CB.get() {
            if let Ok(guard) = lock.lock() {
                if let Some(cb) = guard.as_ref() {
                    cb(layout);
                    let (nx, ny, nw, nh) = compute_snap_rect(layout, 100.0, 100.0);
                    return Ok(format!(
                        "Đã căn chỉnh cửa sổ Mac Omni theo bố cục '{}' ({}x{} tại {},{})",
                        layout, nw as i32, nh as i32, nx as i32, ny as i32
                    ));
                }
            }
        }
    }

    unsafe {
        if !AXIsProcessTrusted() {
            return snap_window_via_osascript_fallback(layout);
        }

        // 2. Obtain AXUIElement for the frontmost application directly by PID (reliable even while Ctrl+Option is held)
        let focused_app: CFTypeRef = if front_pid > 0 {
            AXUIElementCreateApplication(front_pid)
        } else {
            let sys_wide = AXUIElementCreateSystemWide();
            let attr_focused_app = make_cfstring("AXFocusedApplication");
            let mut app_ref: CFTypeRef = std::ptr::null();
            let _ = AXUIElementCopyAttributeValue(sys_wide, attr_focused_app, &mut app_ref);
            CFRelease(attr_focused_app);
            CFRelease(sys_wide);
            app_ref
        };

        if focused_app.is_null() {
            return Err("Không tìm thấy ứng dụng đang active để chia cửa sổ.".to_string());
        }

        // Try AXFocusedWindow first, then fallback to AXMainWindow, then first item in AXWindows
        let attr_focused_win = make_cfstring("AXFocusedWindow");
        let mut focused_win: CFTypeRef = std::ptr::null();
        let err_win =
            AXUIElementCopyAttributeValue(focused_app, attr_focused_win, &mut focused_win);
        CFRelease(attr_focused_win);

        if err_win != 0 || focused_win.is_null() {
            let attr_main_win = make_cfstring("AXMainWindow");
            let _ = AXUIElementCopyAttributeValue(focused_app, attr_main_win, &mut focused_win);
            CFRelease(attr_main_win);
        }

        if focused_win.is_null() {
            let attr_wins = make_cfstring("AXWindows");
            let mut wins_arr: CFTypeRef = std::ptr::null();
            if AXUIElementCopyAttributeValue(focused_app, attr_wins, &mut wins_arr) == 0
                && !wins_arr.is_null()
            {
                if CFArrayGetCount(wins_arr) > 0 {
                    let first = CFArrayGetValueAtIndex(wins_arr, 0);
                    if !first.is_null() {
                        #[link(name = "CoreFoundation", kind = "framework")]
                        extern "C" {
                            fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
                        }
                        focused_win = CFRetain(first);
                    }
                }
                CFRelease(wins_arr);
            }
            CFRelease(attr_wins);
        }

        CFRelease(focused_app);

        if focused_win.is_null() {
            return snap_window_via_osascript_fallback(layout);
        }

        // Read current window position + center point to determine which display it sits on
        let attr_pos = make_cfstring("AXPosition");
        let attr_size = make_cfstring("AXSize");
        let mut cur_pos_val: CFTypeRef = std::ptr::null();
        let mut cur_size_val: CFTypeRef = std::ptr::null();
        let mut cur_pt = CGPoint { x: 100.0, y: 100.0 };
        let mut cur_sz = CGSize {
            width: 800.0,
            height: 600.0,
        };
        if AXUIElementCopyAttributeValue(focused_win, attr_pos, &mut cur_pos_val) == 0
            && !cur_pos_val.is_null()
        {
            AXValueGetValue(
                cur_pos_val,
                K_AX_VALUE_CGPOINT_TYPE,
                &mut cur_pt as *mut _ as *mut c_void,
            );
            CFRelease(cur_pos_val);
        }
        if AXUIElementCopyAttributeValue(focused_win, attr_size, &mut cur_size_val) == 0
            && !cur_size_val.is_null()
        {
            AXValueGetValue(
                cur_size_val,
                K_AX_VALUE_CGSIZE_TYPE,
                &mut cur_sz as *mut _ as *mut c_void,
            );
            CFRelease(cur_size_val);
        }

        let center_x = cur_pt.x + cur_sz.width / 2.0;
        let center_y = cur_pt.y + cur_sz.height / 2.0;
        let (nx, ny, nw, nh) = compute_snap_rect(layout, center_x, center_y);

        let new_pt = CGPoint { x: nx, y: ny };
        let new_sz = CGSize {
            width: nw,
            height: nh,
        };

        let ax_pos = AXValueCreate(K_AX_VALUE_CGPOINT_TYPE, &new_pt as *const _ as *const c_void);
        let ax_sz = AXValueCreate(K_AX_VALUE_CGSIZE_TYPE, &new_sz as *const _ as *const c_void);

        if !ax_pos.is_null() {
            AXUIElementSetAttributeValue(focused_win, attr_pos, ax_pos);
        }
        if !ax_sz.is_null() {
            AXUIElementSetAttributeValue(focused_win, attr_size, ax_sz);
        }
        if !ax_pos.is_null() {
            AXUIElementSetAttributeValue(focused_win, attr_pos, ax_pos);
            CFRelease(ax_pos);
        }
        if !ax_sz.is_null() {
            CFRelease(ax_sz);
        }

        CFRelease(attr_pos);
        CFRelease(attr_size);
        CFRelease(focused_win);

        Ok(format!(
            "Đã áp dụng bố cục '{}' ({}x{} tại {},{})",
            layout, nw as i32, nh as i32, nx as i32, ny as i32
        ))
    }
}

