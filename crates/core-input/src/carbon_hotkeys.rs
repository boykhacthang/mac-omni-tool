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
static CARBON_DISP_HANDLER_REF: AtomicI64 = AtomicI64::new(0);
static CARBON_APP_HANDLER_REF: AtomicI64 = AtomicI64::new(0);
static LAST_WINDOW_SNAP_MS: AtomicI64 = AtomicI64::new(0);
static WINDOW_TILING_HOTKEY_REFS: OnceLock<Mutex<Vec<(usize, String)>>> = OnceLock::new();
static FEATURE_HOTKEY_REFS: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();

unsafe fn primary_carbon_target() -> *mut c_void {
    let disp = GetEventDispatcherTarget();
    if !disp.is_null() {
        disp
    } else {
        GetApplicationEventTarget()
    }
}

fn window_tiling_refs() -> &'static Mutex<Vec<(usize, String)>> {
    WINDOW_TILING_HOTKEY_REFS.get_or_init(|| Mutex::new(Vec::new()))
}

fn feature_hotkey_refs() -> &'static Mutex<Vec<usize>> {
    FEATURE_HOTKEY_REFS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Synchronize OS-level Carbon HotKeys for Clipboard (`id = 2`), Mission Control (`id = 3`),
/// Screenshot modes (`id = 4, 5, 6`), and Show Desktop (`id = 7`).
/// Immediately unregisters (`UnregisterEventHotKey`) any hotkey whose feature toggle is OFF.
pub fn sync_feature_carbon_hotkeys() {
    unsafe {
        let target = primary_carbon_target();
        if target.is_null() {
            return;
        }
        let mut guard = feature_hotkey_refs().lock().unwrap();
        for hk_ptr in guard.drain(..) {
            if hk_ptr != 0 {
                let _ = UnregisterEventHotKey(hk_ptr as *mut c_void);
            }
        }

        let cfg = get_input_config();
        let sig: u32 = 0x4F4D_4E49; // 'OMNI'

        let mut register_if_active = |id: u32, active: bool, shortcut: &str| {
            if !active || shortcut.trim().is_empty() {
                return;
            }
            if let Some((kc, mods)) = shortcut_to_carbon_keycode_and_modifiers(shortcut) {
                let mut hk_ref: *mut c_void = std::ptr::null_mut();
                let status = RegisterEventHotKey(
                    kc,
                    mods,
                    EventHotKeyID { signature: sig, id },
                    target,
                    0,
                    &mut hk_ref,
                );
                if status == 0 && !hk_ref.is_null() {
                    guard.push(hk_ref as usize);
                }
            }
        };

        // Hotkey 2: Clipboard Left Drawer
        register_if_active(2, cfg.clipboard_enabled, &cfg.clipboard_popup_shortcut);

        // Hotkey 3: Mission Control (F3)
        register_if_active(
            3,
            !cfg.mission_control_shortcut.trim().is_empty(),
            &cfg.mission_control_shortcut,
        );

        // Hotkey 4 & 14: Screenshot Mode 1 (Region) — always registered when feature is enabled;
        // supports both Cmd+Shift+0 and Ctrl+Shift+0 when default shortcut is active.
        let region_active = cfg.screenshot_enabled && cfg.screenshot_region_enabled;
        register_if_active(4, region_active, &cfg.screenshot_shortcut);
        if shortcuts_match(&cfg.screenshot_shortcut, "Cmd+Shift+0") {
            register_if_active(14, region_active, "Ctrl+Shift+0");
        } else if shortcuts_match(&cfg.screenshot_shortcut, "Ctrl+Shift+0") {
            register_if_active(14, region_active, "Cmd+Shift+0");
        }

        // Hotkey 5 & 15: Screenshot Mode 2 (Fullscreen + Annotation)
        let fs_edit_active = cfg.screenshot_enabled && cfg.screenshot_fullscreen_edit_enabled;
        register_if_active(5, fs_edit_active, &cfg.screenshot_fullscreen_edit_shortcut);
        if shortcuts_match(&cfg.screenshot_fullscreen_edit_shortcut, "Cmd+Shift+9") {
            register_if_active(15, fs_edit_active, "Ctrl+Shift+9");
        } else if shortcuts_match(&cfg.screenshot_fullscreen_edit_shortcut, "Ctrl+Shift+9") {
            register_if_active(15, fs_edit_active, "Cmd+Shift+9");
        }

        // Hotkey 6 & 16: Screenshot Mode 3 (Fullscreen + Instant Copy)
        let fs_copy_active = cfg.screenshot_enabled && cfg.screenshot_fullscreen_copy_enabled;
        register_if_active(6, fs_copy_active, &cfg.screenshot_fullscreen_copy_shortcut);
        if shortcuts_match(&cfg.screenshot_fullscreen_copy_shortcut, "Cmd+Shift+8") {
            register_if_active(16, fs_copy_active, "Ctrl+Shift+8");
        } else if shortcuts_match(&cfg.screenshot_fullscreen_copy_shortcut, "Ctrl+Shift+8") {
            register_if_active(16, fs_copy_active, "Cmd+Shift+8");
        }

        // Hotkey 7: Show Desktop (F11)
        register_if_active(
            7,
            !cfg.show_desktop_shortcut.trim().is_empty(),
            &cfg.show_desktop_shortcut,
        );

        // Hotkey 8: Open / Toggle Main Dashboard (especially when Menu Bar is hidden in Background Mode)
        register_if_active(
            8,
            !cfg.dashboard_shortcut.trim().is_empty(),
            &cfg.dashboard_shortcut,
        );

        // Permanent Emergency Unfreeze Hotkeys (Option+Esc = 98, Ctrl+Esc = 97) always active at OS Carbon level
        register_if_active(98, true, "Option+Escape");
        register_if_active(97, true, "Ctrl+Escape");

        // Permanent Emergency Force-Quit Hotkeys (Ctrl+Shift+Esc = 999, Ctrl+Option+Cmd+Q = 998, Cmd+Option+Shift+Esc = 997)
        register_if_active(999, true, "Ctrl+Shift+Escape");
        register_if_active(998, true, "Ctrl+Option+Cmd+Q");
        register_if_active(997, true, "Cmd+Option+Shift+Escape");
    }
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

/// Synchronize OS-level Carbon HotKeys for all enabled features and Window Tiling bindings (`id = 100..150`).
/// Guarantees `Ctrl+Option+C`, `Ctrl+Option+Left/Right/Up/Down/Enter`, etc. work even if `CGEventTap` is blocked,
/// and unregisters any hotkey whose toggle is OFF.
pub fn sync_window_tiling_carbon_hotkeys() {
    sync_feature_carbon_hotkeys();
    unsafe {
        let target = primary_carbon_target();
        if target.is_null() {
            return;
        }
        let mut guard = window_tiling_refs().lock().unwrap();
        for (hk_ptr, _) in guard.drain(..) {
            if hk_ptr != 0 {
                let _ = UnregisterEventHotKey(hk_ptr as *mut c_void);
            }
        }

        if !get_input_config().window_snapping_enabled || !core_window::is_accessibility_trusted() {
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
                let target = primary_carbon_target();
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
    if let Ok(cfg) = crate::config::config_mutex().try_lock() {
        crate::event_tap::sync_event_tap_enabled(cfg.needs_event_tap());
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
                if cfg.clipboard_enabled && !cfg.clipboard_popup_shortcut.trim().is_empty() {
                    notify_overlay(UiOverlayEvent::ToggleClipboardLeftDrawer);
                }
            }
            3 => {
                if !cfg.mission_control_shortcut.trim().is_empty() {
                    notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
                    trigger_system_action("mission_control");
                }
            }
            4 | 14 => {
                if cfg.screenshot_enabled
                    && cfg.screenshot_region_enabled
                    && !cfg.screenshot_shortcut.trim().is_empty()
                {
                    notify_overlay(UiOverlayEvent::TriggerScreenshotOverlay);
                }
            }
            5 | 15 => {
                if cfg.screenshot_enabled
                    && cfg.screenshot_fullscreen_edit_enabled
                    && !cfg.screenshot_fullscreen_edit_shortcut.trim().is_empty()
                {
                    notify_overlay(UiOverlayEvent::TriggerFullscreenEditScreenshot);
                }
            }
            6 | 16 => {
                if cfg.screenshot_enabled
                    && cfg.screenshot_fullscreen_copy_enabled
                    && !cfg.screenshot_fullscreen_copy_shortcut.trim().is_empty()
                {
                    notify_overlay(UiOverlayEvent::TriggerFullscreenCopyScreenshot);
                }
            }
            7 => {
                if !cfg.show_desktop_shortcut.trim().is_empty() {
                    trigger_system_action("show_desktop");
                }
            }
            8 => {
                if !cfg.dashboard_shortcut.trim().is_empty() {
                    notify_overlay(UiOverlayEvent::ToggleMainDashboard);
                }
            }
            97..=99 => {
                crate::event_tap::reset_all_tap_held_states();
                notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
            }
            997..=999 => {
                eprintln!("[EMERGENCY] Global emergency force-quit hotkey triggered! Exiting process immediately.");
                std::process::exit(0);
            }
            id if (100..150).contains(&id) => {
                if cfg.window_snapping_enabled && core_window::is_accessibility_trusted() {
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

/// Register native Carbon EventHotKeys on main thread (`Cmd+Shift+V`, `F3`, `Cmd+Shift+0/9/8`, `Ctrl+Shift+0/9/8`, and Window Tiling shortcuts).
pub fn register_native_carbon_hotkeys() {
    unsafe {
        let spec = EventTypeSpec {
            event_class: K_EVENT_CLASS_KEYBOARD,
            event_kind: K_EVENT_HOT_KEY_PRESSED,
        };
        let app_target = GetApplicationEventTarget();
        if !app_target.is_null() && CARBON_APP_HANDLER_REF.load(Ordering::SeqCst) == 0 {
            let mut handler_ref: *mut c_void = std::ptr::null_mut();
            let status = InstallEventHandler(
                app_target,
                carbon_hotkey_handler,
                1,
                &spec,
                std::ptr::null_mut(),
                &mut handler_ref,
            );
            if status == 0 && !handler_ref.is_null() {
                CARBON_APP_HANDLER_REF.store(handler_ref as i64, Ordering::SeqCst);
            }
        }
        let disp_target = GetEventDispatcherTarget();
        if !disp_target.is_null()
            && disp_target != app_target
            && CARBON_DISP_HANDLER_REF.load(Ordering::SeqCst) == 0
        {
            let mut handler_ref: *mut c_void = std::ptr::null_mut();
            let status = InstallEventHandler(
                disp_target,
                carbon_hotkey_handler,
                1,
                &spec,
                std::ptr::null_mut(),
                &mut handler_ref,
            );
            if status == 0 && !handler_ref.is_null() {
                CARBON_DISP_HANDLER_REF.store(handler_ref as i64, Ordering::SeqCst);
            }
        }
    }

    sync_window_tiling_carbon_hotkeys();
}
