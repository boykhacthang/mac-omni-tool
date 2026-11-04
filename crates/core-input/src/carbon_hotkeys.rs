//! OS-level Carbon `RegisterEventHotKey` manager for Clipboard (`Cmd+Shift+V`), Mission Control (`F3`),
//! Screenshot modes (`Cmd+Shift+0/9/8`), Escape dismissal, and all 13 Window Tiling shortcuts (`Ctrl+Option+C`, etc.).

use crate::actions::{
    notify_overlay, notify_overlay_immediate, now_ms, trigger_system_action,
};
use crate::config::{
    get_input_config, shortcut_to_carbon_keycode_and_modifiers, shortcuts_match, UiOverlayEvent,
};
use crate::ffi::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

pub(crate) static OVERLAY_ESCAPE_ACTIVE: AtomicBool = AtomicBool::new(false);
static ESCAPE_HOTKEY_REF: AtomicI64 = AtomicI64::new(0);
static LAST_WINDOW_SNAP_MS: AtomicI64 = AtomicI64::new(0);
static WINDOW_TILING_HOTKEY_REFS: OnceLock<Mutex<Vec<(usize, String)>>> = OnceLock::new();

fn window_tiling_refs() -> &'static Mutex<Vec<(usize, String)>> {
    WINDOW_TILING_HOTKEY_REFS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Trigger a window tiling layout with a 140ms debounce so simultaneous Carbon HotKey + CGEventTap never double-fire.
pub fn trigger_window_snap_debounced(action: &str) {
    let t = now_ms();
    let prev = LAST_WINDOW_SNAP_MS.load(Ordering::Relaxed);
    if (t - prev).abs() < 140 {
        return;
    }
    LAST_WINDOW_SNAP_MS.store(t, Ordering::Relaxed);
    let act = action.to_string();
    std::thread::spawn(move || {
        let my_pid = std::process::id() as i32;
        if core_window::frontmost_application_pid() == my_pid {
            notify_overlay_immediate(UiOverlayEvent::SnapOwnWindow(act));
        } else if core_window::snap_focused_window(&act).is_err() {
            notify_overlay_immediate(UiOverlayEvent::SnapOwnWindow(act));
        }
    });
}

/// Synchronize OS-level Carbon HotKeys for all enabled Window Tiling bindings (`id = 100..150`).
/// Guarantees `Ctrl+Option+C`, `Ctrl+Option+Left/Right/Up/Down/Enter`, etc. work even if `CGEventTap` is blocked.
pub fn sync_window_tiling_carbon_hotkeys() {
    unsafe {
        let target = GetApplicationEventTarget();
        if target.is_null() {
            return;
        }
        let mut guard = window_tiling_refs().lock().unwrap();
        for (hk_ptr, _) in guard.drain(..) {
            if hk_ptr != 0 {
                let _ = UnregisterEventHotKey(hk_ptr as *mut c_void);
            }
        }

        if !get_input_config().window_snapping_enabled {
            return;
        }

        let sig: u32 = 0x4F4D_4E49; // 'OMNI'
        let bindings = core_window::get_window_shortcuts();
        for (idx, b) in bindings.into_iter().enumerate() {
            if !b.enabled || b.shortcut.trim().is_empty() {
                continue;
            }
            if let Some((kc, mods)) = shortcut_to_carbon_keycode_and_modifiers(&b.shortcut) {
                let mut hk_ref: *mut c_void = std::ptr::null_mut();
                let id = 100 + (idx as u32);
                let status = RegisterEventHotKey(
                    kc,
                    mods,
                    EventHotKeyID { signature: sig, id },
                    target,
                    0,
                    &mut hk_ref,
                );
                if status == 0 && !hk_ref.is_null() {
                    guard.push((hk_ref as usize, b.action));
                } else {
                    guard.push((0, b.action));
                }
            } else {
                guard.push((0, b.action));
            }
        }
    }
}

/// Dynamically register/unregister OS-level Carbon HotKey for `Escape` (keycode 53) while any overlay is open.
pub fn set_overlay_escape_hotkey_active(active: bool) {
    OVERLAY_ESCAPE_ACTIVE.store(active, Ordering::SeqCst);
    unsafe {
        if active {
            if ESCAPE_HOTKEY_REF.load(Ordering::SeqCst) == 0 {
                let target = GetApplicationEventTarget();
                if !target.is_null() {
                    let mut hk_esc: *mut c_void = std::ptr::null_mut();
                    let status = RegisterEventHotKey(
                        53, // Escape key
                        0,
                        EventHotKeyID {
                            signature: 0x4F4D_4E49,
                            id: 99,
                        },
                        target,
                        0,
                        &mut hk_esc,
                    );
                    if status == 0 && !hk_esc.is_null() {
                        ESCAPE_HOTKEY_REF.store(hk_esc as i64, Ordering::SeqCst);
                    }
                }
            }
        } else {
            let old = ESCAPE_HOTKEY_REF.swap(0, Ordering::SeqCst) as *mut c_void;
            if !old.is_null() {
                let _ = UnregisterEventHotKey(old);
            }
        }
    }
}

unsafe extern "C" fn carbon_hotkey_handler(
    _next_handler: *mut c_void,
    the_event: *mut c_void,
    _user_data: *mut c_void,
) -> i32 {
    let mut hk_id = EventHotKeyID {
        signature: 0,
        id: 0,
    };
    let status = GetEventParameter(
        the_event,
        K_EVENT_PARAM_DIRECT_OBJECT,
        TYPE_EVENT_HOT_KEY_ID,
        std::ptr::null_mut(),
        std::mem::size_of::<EventHotKeyID>(),
        std::ptr::null_mut(),
        &mut hk_id as *mut _ as *mut c_void,
    );
    if status == 0 {
        let cfg = get_input_config();
        match hk_id.id {
            2 => {
                if cfg.clipboard_enabled {
                    notify_overlay(UiOverlayEvent::ToggleClipboardLeftDrawer);
                }
            }
            3 => {
                notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
                trigger_system_action("mission_control");
            }
            4 => {
                if cfg.screenshot_enabled
                    && cfg.screenshot_region_enabled
                    && shortcuts_match(&cfg.screenshot_shortcut, "Cmd+Shift+0")
                {
                    notify_overlay(UiOverlayEvent::TriggerScreenshotOverlay);
                }
            }
            5 => {
                if cfg.screenshot_enabled
                    && cfg.screenshot_fullscreen_edit_enabled
                    && shortcuts_match(&cfg.screenshot_fullscreen_edit_shortcut, "Cmd+Shift+9")
                {
                    notify_overlay(UiOverlayEvent::TriggerFullscreenEditScreenshot);
                }
            }
            6 => {
                if cfg.screenshot_enabled
                    && cfg.screenshot_fullscreen_copy_enabled
                    && shortcuts_match(&cfg.screenshot_fullscreen_copy_shortcut, "Cmd+Shift+8")
                {
                    notify_overlay(UiOverlayEvent::TriggerFullscreenCopyScreenshot);
                }
            }
            99 => {
                notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
            }
            id if (100..150).contains(&id) => {
                if cfg.window_snapping_enabled {
                    let idx = (id - 100) as usize;
                    let bindings = core_window::get_window_shortcuts();
                    if let Some(b) = bindings.get(idx) {
                        if b.enabled {
                            trigger_window_snap_debounced(&b.action);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    0
}

/// Register native Carbon EventHotKeys on main thread (`Cmd+Shift+V`, `F3`, `Cmd+Shift+0/9/8`, and Window Tiling shortcuts).
pub fn register_native_carbon_hotkeys() {
    unsafe {
        let target = GetApplicationEventTarget();
        if target.is_null() {
            return;
        }
        let spec = EventTypeSpec {
            event_class: K_EVENT_CLASS_KEYBOARD,
            event_kind: K_EVENT_HOT_KEY_PRESSED,
        };
        let mut handler_ref: *mut c_void = std::ptr::null_mut();
        InstallEventHandler(
            target,
            carbon_hotkey_handler,
            1,
            &spec,
            std::ptr::null_mut(),
            &mut handler_ref,
        );

        let sig: u32 = 0x4F4D_4E49; // 'OMNI'
        let mut hk2: *mut c_void = std::ptr::null_mut();
        let mut hk3: *mut c_void = std::ptr::null_mut();
        let mut hk4: *mut c_void = std::ptr::null_mut();
        let mut hk5: *mut c_void = std::ptr::null_mut();
        let mut hk6: *mut c_void = std::ptr::null_mut();

        // Hotkey 2: Cmd + Shift + V -> keycode 9 ('V')
        RegisterEventHotKey(
            9,
            CARBON_CMD_KEY | CARBON_SHIFT_KEY,
            EventHotKeyID { signature: sig, id: 2 },
            target,
            0,
            &mut hk2,
        );

        // Hotkey 3: F3 -> keycode 99
        RegisterEventHotKey(
            99,
            0,
            EventHotKeyID { signature: sig, id: 3 },
            target,
            0,
            &mut hk3,
        );

        // Hotkey 4: Cmd + Shift + 0 -> keycode 29 ('0')
        RegisterEventHotKey(
            29,
            CARBON_CMD_KEY | CARBON_SHIFT_KEY,
            EventHotKeyID { signature: sig, id: 4 },
            target,
            0,
            &mut hk4,
        );

        // Hotkey 5: Cmd + Shift + 9 -> keycode 25 ('9')
        RegisterEventHotKey(
            25,
            CARBON_CMD_KEY | CARBON_SHIFT_KEY,
            EventHotKeyID { signature: sig, id: 5 },
            target,
            0,
            &mut hk5,
        );

        // Hotkey 6: Cmd + Shift + 8 -> keycode 28 ('8')
        RegisterEventHotKey(
            28,
            CARBON_CMD_KEY | CARBON_SHIFT_KEY,
            EventHotKeyID { signature: sig, id: 6 },
            target,
            0,
            &mut hk6,
        );
    }

    sync_window_tiling_carbon_hotkeys();
}
