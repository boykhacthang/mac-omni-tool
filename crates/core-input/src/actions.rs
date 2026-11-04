//! System actions (`Mission Control`, `App Exposé`, `Show Desktop`, `Launchpad`),
//! Finder frontmost check, and UI overlay callbacks.

use crate::config::{get_input_config, UiOverlayEvent};
use crate::ffi::*;
use std::ffi::{c_char, c_void, CStr, CString};
use std::process::Command;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) static OVERLAY_CALLBACK: OnceLock<
    Mutex<Option<Box<dyn Fn(UiOverlayEvent) + Send + Sync>>>,
> = OnceLock::new();
static LAST_OVERLAY_EVENT_MS: AtomicI64 = AtomicI64::new(0);

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn register_overlay_callback<F>(cb: F)
where
    F: Fn(UiOverlayEvent) + Send + Sync + 'static,
{
    let lock = OVERLAY_CALLBACK.get_or_init(|| Mutex::new(None));
    *lock.lock().unwrap() = Some(Box::new(cb));
}

pub(crate) fn notify_overlay_immediate(ev: UiOverlayEvent) {
    if let Some(lock) = OVERLAY_CALLBACK.get() {
        if let Ok(guard) = lock.try_lock() {
            if let Some(cb) = guard.as_ref() {
                cb(ev);
            }
        }
    }
}

pub(crate) fn notify_overlay(ev: UiOverlayEvent) {
    if ev == UiOverlayEvent::HideAllOverlays {
        notify_overlay_immediate(ev);
        return;
    }
    let t = now_ms();
    let prev = LAST_OVERLAY_EVENT_MS.load(Ordering::Relaxed);
    if (t - prev).abs() < 90 {
        return;
    }
    LAST_OVERLAY_EVENT_MS.store(t, Ordering::Relaxed);
    notify_overlay_immediate(ev);
}

/// Synthesize a hardware-level keyboard shortcut (tagged with `OMNI_SYNTHETIC_EVENT_TAG` so our own event tap ignores it).
pub fn post_synthetic_key_combo(keycode: u16, modifier_flags: u64) {
    std::thread::spawn(move || unsafe {
        let src = CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE);
        let down = CGEventCreateKeyboardEvent(src, keycode, true);
        let up = CGEventCreateKeyboardEvent(src, keycode, false);

        if !down.is_null() {
            // Preserve native SecondaryFn / NumericPad flags for arrow/function keys so macOS Dock SymbolicHotKeys (Spaces) match
            let native_bits = CGEventGetFlags(down) & (0x0080_0000 | 0x0020_0000);
            CGEventSetFlags(down, native_bits | modifier_flags);
            CGEventSetIntegerValueField(
                down,
                K_CG_EVENT_SOURCE_USER_DATA,
                OMNI_SYNTHETIC_EVENT_TAG,
            );
            CGEventPost(K_CG_HID_EVENT_TAP, down);
            CFRelease(down);
        }

        std::thread::sleep(std::time::Duration::from_millis(8));

        if !up.is_null() {
            let native_bits = CGEventGetFlags(up) & (0x0080_0000 | 0x0020_0000);
            CGEventSetFlags(up, native_bits | modifier_flags);
            CGEventSetIntegerValueField(up, K_CG_EVENT_SOURCE_USER_DATA, OMNI_SYNTHETIC_EVENT_TAG);
            CGEventPost(K_CG_HID_EVENT_TAP, up);
            CFRelease(up);
        }

        if !src.is_null() {
            CFRelease(src);
        }
    });
}

/// Trigger macOS Mission Control (F3), App Exposé, Show Desktop, Launchpad, Space Switching, Browser/Tab navigation, or internal overlays.
pub fn trigger_system_action(action: &str) {
    match action {
        "space_left" => {
            // Ctrl + Left Arrow (123) -> Move left a Space
            post_synthetic_key_combo(123, FLAG_CONTROL);
        }
        "space_right" => {
            // Ctrl + Right Arrow (124) -> Move right a Space
            post_synthetic_key_combo(124, FLAG_CONTROL);
        }
        "browser_back" => {
            // Cmd + [ (33) -> Back in Browsers & Finder
            post_synthetic_key_combo(33, FLAG_COMMAND);
        }
        "browser_forward" => {
            // Cmd + ] (30) -> Forward in Browsers & Finder
            post_synthetic_key_combo(30, FLAG_COMMAND);
        }
        "tab_prev" => {
            // Ctrl + Shift + Tab (48) -> Previous Tab
            post_synthetic_key_combo(48, FLAG_CONTROL | FLAG_SHIFT);
        }
        "tab_next" => {
            // Ctrl + Tab (48) -> Next Tab
            post_synthetic_key_combo(48, FLAG_CONTROL);
        }
        "close_tab" => {
            // Cmd + W (13) -> Close Tab / Window
            post_synthetic_key_combo(13, FLAG_COMMAND);
        }
        "new_tab" => {
            // Cmd + T (17) -> New Tab
            post_synthetic_key_combo(17, FLAG_COMMAND);
        }
        "mission_control" => {
            std::thread::spawn(|| {
                unsafe {
                    if let Ok(path) = CString::new(
                        "/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight",
                    ) {
                        let handle = libc::dlopen(path.as_ptr(), libc::RTLD_LAZY);
                        let _ = handle;
                    }
                }
                let _ = Command::new("open")
                    .args(["-a", "Mission Control"])
                    .status();
            });
        }
        "app_expose" => {
            std::thread::spawn(|| {
                let _ = Command::new("open")
                    .args(["-a", "Mission Control", "--args", "2"])
                    .status();
            });
        }
        "show_desktop" => {
            std::thread::spawn(|| {
                let _ = Command::new("open")
                    .args(["-a", "Mission Control", "--args", "1"])
                    .status();
            });
        }
        "launchpad" => {
            std::thread::spawn(|| {
                let _ = Command::new("open").args(["-a", "Launchpad"]).status();
            });
        }
        "clipboard_popup" => {
            notify_overlay(UiOverlayEvent::ToggleClipboardLeftDrawer);
        }
        "alt_tab" => {
            notify_overlay(UiOverlayEvent::ToggleAltTabOverlay);
        }
        "screenshot" | "screenshot_region" => {
            if get_input_config().screenshot_enabled {
                notify_overlay(UiOverlayEvent::TriggerScreenshotOverlay);
            }
        }
        "screenshot_full_edit" => {
            if get_input_config().screenshot_enabled {
                notify_overlay(UiOverlayEvent::TriggerFullscreenEditScreenshot);
            }
        }
        "screenshot_full_copy" => {
            if get_input_config().screenshot_enabled {
                notify_overlay(UiOverlayEvent::TriggerFullscreenCopyScreenshot);
            }
        }
        _ => {}
    }
}

/// Check if Finder (`com.apple.finder`) is currently the frontmost application.
pub fn is_finder_frontmost() -> bool {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let ws_cls = objc_getClass(c"NSWorkspace".as_ptr());
        if ws_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return false;
        }

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgUtf8 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_char;
        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_utf8: MsgUtf8 = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_shared = sel_registerName(c"sharedWorkspace".as_ptr());
        let sel_front = sel_registerName(c"frontmostApplication".as_ptr());
        let sel_bundle = sel_registerName(c"bundleIdentifier".as_ptr());
        let sel_utf8 = sel_registerName(c"UTF8String".as_ptr());

        let ws = msg0(ws_cls, sel_shared);
        if ws.is_null() {
            objc_autoreleasePoolPop(pool);
            return false;
        }
        let app = msg0(ws, sel_front);
        if app.is_null() {
            objc_autoreleasePoolPop(pool);
            return false;
        }
        let bundle_ns = msg0(app, sel_bundle);
        if bundle_ns.is_null() {
            objc_autoreleasePoolPop(pool);
            return false;
        }
        let c_ptr = msg_utf8(bundle_ns, sel_utf8);
        let is_finder = if !c_ptr.is_null() {
            CStr::from_ptr(c_ptr).to_bytes() == b"com.apple.finder"
        } else {
            false
        };

        objc_autoreleasePoolPop(pool);
        is_finder
    }
}

