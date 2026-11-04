//! Global `CGEventTap` listener thread for `Cmd+Tab` replacement, Finder `Cmd+X`/`Cmd+V`,
//! Simultaneous Left+Right Mouse Chords, and Custom Global Shortcuts.

use crate::actions::{
    is_finder_frontmost, notify_overlay, notify_overlay_immediate, now_ms, trigger_system_action,
};
use crate::carbon_hotkeys::{trigger_window_snap_debounced, OVERLAY_ESCAPE_ACTIVE};
use crate::config::{
    config_mutex, keycode_and_flags_to_shortcut, shortcuts_match, UiOverlayEvent,
    FINDER_CUT_PENDING,
};
use crate::ffi::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

static TAP_RUNNING: AtomicBool = AtomicBool::new(false);
static TAP_ATTEMPTED_WHILE_TRUSTED: AtomicBool = AtomicBool::new(false);
static STATE_CACHE_RUNNING: AtomicBool = AtomicBool::new(false);
static ALTTAB_MODIFIER_HELD: AtomicBool = AtomicBool::new(false);

static LEFT_DOWN_AT_MS: AtomicI64 = AtomicI64::new(0);
static RIGHT_DOWN_AT_MS: AtomicI64 = AtomicI64::new(0);
static LEFT_IS_HELD: AtomicBool = AtomicBool::new(false);
static RIGHT_IS_HELD: AtomicBool = AtomicBool::new(false);
static SUPPRESS_RIGHT_UP: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_OTHER_BTN_MASK: AtomicI64 = AtomicI64::new(0);
static ACTIVE_TAP_PORT: AtomicI64 = AtomicI64::new(0);

// Lock-free & non-blocking cached system state updated off the HID thread every 200ms
// so `event_tap_callback` never calls Objective-C `NSWorkspace` or WindowServer IPC synchronously.
static CACHED_FRONTMOST_IS_FINDER: AtomicBool = AtomicBool::new(false);
static CACHED_TOTAL_SPACES: AtomicI64 = AtomicI64::new(2);
static CACHED_FRONTMOST_BUNDLE: OnceLock<Mutex<String>> = OnceLock::new();
static CACHED_WINDOW_SHORTCUTS: OnceLock<Mutex<Vec<core_window::WindowShortcutBinding>>> =
    OnceLock::new();

fn cached_frontmost_bundle() -> String {
    CACHED_FRONTMOST_BUNDLE
        .get_or_init(|| Mutex::new(String::new()))
        .try_lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

fn cached_window_shortcuts() -> Vec<core_window::WindowShortcutBinding> {
    CACHED_WINDOW_SHORTCUTS
        .get_or_init(|| Mutex::new(Vec::new()))
        .try_lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

pub(crate) fn reset_all_tap_held_states() {
    ALTTAB_MODIFIER_HELD.store(false, Ordering::SeqCst);
    LEFT_IS_HELD.store(false, Ordering::Relaxed);
    RIGHT_IS_HELD.store(false, Ordering::Relaxed);
    SUPPRESS_RIGHT_UP.store(false, Ordering::Relaxed);
    SUPPRESSED_OTHER_BTN_MASK.store(0, Ordering::Relaxed);
    crate::smooth_scroll::stop_smooth_scroll_momentum();
}

fn start_background_state_cache() {
    if STATE_CACHE_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("omni-input-state-cache".to_string())
        .spawn(|| {
            let mut last_space_index: usize = 0;
            loop {
                let (bundle_id, _) = crate::spaces::frontmost_app_info();
                let is_finder = bundle_id == "com.apple.finder" || is_finder_frontmost();
                CACHED_FRONTMOST_IS_FINDER.store(is_finder, Ordering::Relaxed);
                if let Ok(mut guard) = CACHED_FRONTMOST_BUNDLE
                    .get_or_init(|| Mutex::new(String::new()))
                    .try_lock()
                {
                    *guard = bundle_id;
                }

                let (total_spaces, _, cur_space) = crate::spaces::query_macos_spaces_counts();
                CACHED_TOTAL_SPACES.store(total_spaces.max(1) as i64, Ordering::Relaxed);

                // If user switched macOS Desktop Space (e.g. via physical F3 Mission Control or trackpad swipe)
                // while an overlay was active, immediately dismiss all overlays so no window is left stranded.
                if last_space_index != 0 && cur_space != 0 && cur_space != last_space_index {
                    if OVERLAY_ESCAPE_ACTIVE.load(Ordering::SeqCst) {
                        reset_all_tap_held_states();
                        notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
                    }
                }
                if cur_space != 0 {
                    last_space_index = cur_space;
                }

                let shortcuts = core_window::get_window_shortcuts();
                if let Ok(mut guard) = CACHED_WINDOW_SHORTCUTS
                    .get_or_init(|| Mutex::new(Vec::new()))
                    .try_lock()
                {
                    *guard = shortcuts;
                }

                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        });
}

/// Dynamically enable or disable the macOS `CGEventTap` port based on whether Accessibility is trusted
/// and whether any tap-dependent feature is active.
pub fn sync_event_tap_enabled(enabled: bool) {
    let trusted = core_window::is_accessibility_trusted();
    if !enabled || !trusted {
        reset_all_tap_held_states();
    }
    let port = ACTIVE_TAP_PORT.load(Ordering::Relaxed) as CFMachPortRef;
    if !port.is_null() {
        let active = trusted && (enabled || OVERLAY_ESCAPE_ACTIVE.load(Ordering::SeqCst));
        unsafe {
            CGEventTapEnable(port, active);
        }
    }
}

/// Notify `core-input` that macOS permissions may have changed (e.g. user returned from System Settings or clicked "Làm mới quyền").
/// Re-evaluates `CGEventTap` state and Carbon HotKeys without ever triggering any OS prompt.
pub fn notify_permissions_changed() {
    TAP_ATTEMPTED_WHILE_TRUSTED.store(false, Ordering::SeqCst);
    let cfg = crate::config::get_input_config();
    sync_event_tap_enabled(cfg.needs_event_tap());
    crate::carbon_hotkeys::sync_window_tiling_carbon_hotkeys();
}

unsafe extern "C" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    let trusted = core_window::is_accessibility_trusted();
    if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT {
        reset_all_tap_held_states();
        let port = ACTIVE_TAP_PORT.load(Ordering::Relaxed) as CFMachPortRef;
        if !port.is_null() {
            let should_enable = trusted
                && (config_mutex()
                    .try_lock()
                    .map(|g| g.needs_event_tap())
                    .unwrap_or(true)
                    || OVERLAY_ESCAPE_ACTIVE.load(Ordering::SeqCst));
            CGEventTapEnable(port, should_enable);
        }
        return event;
    }

    if event.is_null() || !trusted {
        return event;
    }

    // Ignore synthetic events generated by Mac Omni Tool itself (Smooth Scroll & Synthetic Key Combos)
    if CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA) == OMNI_SYNTHETIC_EVENT_TAG {
        return event;
    }

    // Unconditional, lock-free Global Emergency Escape Kill-Switch (runs BEFORE any mutex lock!)
    if event_type == K_CG_EVENT_KEY_DOWN {
        let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
        if keycode == 53 {
            let flags = CGEventGetFlags(event);
            reset_all_tap_held_states();
            let was_overlay_active = OVERLAY_ESCAPE_ACTIVE.swap(false, Ordering::SeqCst);
            let has_emergency_mod = (flags & (FLAG_OPTION | FLAG_CONTROL | FLAG_COMMAND)) != 0;
            notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
            if was_overlay_active || has_emergency_mod {
                return std::ptr::null_mut();
            }
            return event;
        }
        // Physical F3 / Mission Control keycodes (99 = F3, 160 = Mission Control): always dismiss overlays first
        if keycode == 99 || keycode == 160 {
            if OVERLAY_ESCAPE_ACTIVE.swap(false, Ordering::SeqCst) {
                reset_all_tap_held_states();
                notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
            }
        }
    }

    // Never block the HID event tap thread on a mutex; pass events through immediately if busy
    let cfg = match config_mutex().try_lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return event,
    };

    if !cfg.needs_event_tap() && !OVERLAY_ESCAPE_ACTIVE.load(Ordering::SeqCst) {
        return event;
    }

    match event_type {
        K_CG_EVENT_LEFT_MOUSE_DOWN => {
            if !cfg.external_mouse_enabled || !cfg.mouse_chord_enabled {
                LEFT_IS_HELD.store(false, Ordering::Relaxed);
                return event;
            }
            let t = now_ms();
            LEFT_DOWN_AT_MS.store(t, Ordering::Relaxed);
            LEFT_IS_HELD.store(true, Ordering::Relaxed);

            if RIGHT_IS_HELD.load(Ordering::Relaxed) {
                let dt = (t - RIGHT_DOWN_AT_MS.load(Ordering::Relaxed)).abs();
                if dt > 500 {
                    RIGHT_IS_HELD.store(false, Ordering::Relaxed);
                } else if dt <= cfg.mouse_chord_threshold_ms {
                    LEFT_IS_HELD.store(false, Ordering::Relaxed);
                    RIGHT_IS_HELD.store(false, Ordering::Relaxed);
                    SUPPRESS_RIGHT_UP.store(true, Ordering::Relaxed);
                    trigger_system_action(&cfg.mouse_chord_action);
                    return std::ptr::null_mut();
                }
            }
        }
        K_CG_EVENT_LEFT_MOUSE_UP => {
            LEFT_IS_HELD.store(false, Ordering::Relaxed);
        }
        K_CG_EVENT_RIGHT_MOUSE_DOWN => {
            if !cfg.external_mouse_enabled || !cfg.mouse_chord_enabled {
                RIGHT_IS_HELD.store(false, Ordering::Relaxed);
                return event;
            }
            let t = now_ms();
            RIGHT_DOWN_AT_MS.store(t, Ordering::Relaxed);
            RIGHT_IS_HELD.store(true, Ordering::Relaxed);

            if LEFT_IS_HELD.load(Ordering::Relaxed) {
                let dt = (t - LEFT_DOWN_AT_MS.load(Ordering::Relaxed)).abs();
                if dt > 500 {
                    LEFT_IS_HELD.store(false, Ordering::Relaxed);
                } else if dt <= cfg.mouse_chord_threshold_ms.max(250) {
                    LEFT_IS_HELD.store(false, Ordering::Relaxed);
                    RIGHT_IS_HELD.store(false, Ordering::Relaxed);
                    SUPPRESS_RIGHT_UP.store(true, Ordering::Relaxed);
                    trigger_system_action(&cfg.mouse_chord_action);
                    return std::ptr::null_mut();
                }
            }
        }
        K_CG_EVENT_RIGHT_MOUSE_UP => {
            RIGHT_IS_HELD.store(false, Ordering::Relaxed);
            if SUPPRESS_RIGHT_UP.swap(false, Ordering::Relaxed) {
                let dt = (now_ms() - RIGHT_DOWN_AT_MS.load(Ordering::Relaxed)).abs();
                if dt <= 600 {
                    return std::ptr::null_mut();
                }
            }
        }
        K_CG_EVENT_SCROLL_WHEEL => {
            if cfg.external_mouse_enabled
                && (cfg.smooth_scroll_enabled
                    || cfg.invert_external_mouse_scroll
                    || cfg.shift_horizontal_scroll)
            {
                let needs_bundle_check = cfg
                    .app_mouse_profiles
                    .iter()
                    .any(|p| p.enabled && p.smooth_scroll_mode != "inherit");
                let front_bundle = if needs_bundle_check {
                    cached_frontmost_bundle()
                } else {
                    String::new()
                };
                return crate::smooth_scroll::handle_scroll_wheel_event(event, &cfg, &front_bundle);
            }
        }
        K_CG_EVENT_OTHER_MOUSE_DOWN => {
            if cfg.external_mouse_enabled && cfg.mouse_buttons_enabled {
                let btn = CGEventGetIntegerValueField(event, K_CG_MOUSE_EVENT_BUTTON_NUMBER);
                // CoreGraphics buttonNumber: 2 = Mouse 3 (Middle), 3 = Mouse 4 (Back), 4 = Mouse 5 (Forward)
                if (2..=8).contains(&btn) {
                    crate::spaces::LAST_MOUSE_BUTTON.store(btn + 1, Ordering::Relaxed);
                    crate::spaces::LAST_MOUSE_BUTTON_AT_MS.store(now_ms(), Ordering::Relaxed);
                }

                if (2..=4).contains(&btn) {
                    let bundle_id = cached_frontmost_bundle();
                    let action = cfg.resolve_mouse_button_action(&bundle_id, btn);

                    match action {
                        "default" => return event,
                        "disabled" => {
                            SUPPRESSED_OTHER_BTN_MASK.fetch_or(1 << btn, Ordering::Relaxed);
                            return std::ptr::null_mut();
                        }
                        "space_left" | "space_right" => {
                            let total_spaces = CACHED_TOTAL_SPACES.load(Ordering::Relaxed);
                            if total_spaces <= 1 {
                                match cfg.single_space_fallback.as_str() {
                                    "passthrough" => return event,
                                    "mission_control" => {
                                        SUPPRESSED_OTHER_BTN_MASK
                                            .fetch_or(1 << btn, Ordering::Relaxed);
                                        trigger_system_action("mission_control");
                                        return std::ptr::null_mut();
                                    }
                                    _ => {
                                        SUPPRESSED_OTHER_BTN_MASK
                                            .fetch_or(1 << btn, Ordering::Relaxed);
                                        return std::ptr::null_mut();
                                    }
                                }
                            } else {
                                SUPPRESSED_OTHER_BTN_MASK.fetch_or(1 << btn, Ordering::Relaxed);
                                trigger_system_action(action);
                                return std::ptr::null_mut();
                            }
                        }
                        other_act => {
                            SUPPRESSED_OTHER_BTN_MASK.fetch_or(1 << btn, Ordering::Relaxed);
                            trigger_system_action(other_act);
                            return std::ptr::null_mut();
                        }
                    }
                }
            }
        }
        K_CG_EVENT_OTHER_MOUSE_UP => {
            let btn = CGEventGetIntegerValueField(event, K_CG_MOUSE_EVENT_BUTTON_NUMBER);
            if (2..=8).contains(&btn) {
                let bit = 1i64 << btn;
                let prev = SUPPRESSED_OTHER_BTN_MASK.fetch_and(!bit, Ordering::Relaxed);
                if (prev & bit) != 0 {
                    return std::ptr::null_mut();
                }
            }
        }
        K_CG_EVENT_KEY_DOWN => {
            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
            let flags = CGEventGetFlags(event);

            // Feature 5: Finder Cut & Paste (Cmd+X -> Cmd+C with cut flag, Cmd+V -> Cmd+Option+V)
            let only_cmd = (flags & FLAG_COMMAND) != 0
                && (flags & (FLAG_SHIFT | FLAG_OPTION | FLAG_CONTROL)) == 0;
            if cfg.finder_cut_paste_enabled
                && only_cmd
                && CACHED_FRONTMOST_IS_FINDER.load(Ordering::Relaxed)
            {
                if keycode == 7 {
                    FINDER_CUT_PENDING.store(true, Ordering::SeqCst);
                    CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, 8);
                    return event;
                } else if keycode == 8 {
                    FINDER_CUT_PENDING.store(false, Ordering::SeqCst);
                } else if keycode == 9 && FINDER_CUT_PENDING.swap(false, Ordering::SeqCst) {
                    CGEventSetFlags(event, flags | FLAG_OPTION);
                    return event;
                }
            }

            if let Some(combo) = keycode_and_flags_to_shortcut(keycode, flags) {
                if cfg.screenshot_enabled && core_window::is_screen_recording_granted() {
                    if cfg.screenshot_region_enabled
                        && shortcuts_match(&combo, &cfg.screenshot_shortcut)
                    {
                        notify_overlay(UiOverlayEvent::TriggerScreenshotOverlay);
                        return std::ptr::null_mut();
                    }
                    if cfg.screenshot_fullscreen_edit_enabled
                        && shortcuts_match(&combo, &cfg.screenshot_fullscreen_edit_shortcut)
                    {
                        notify_overlay(UiOverlayEvent::TriggerFullscreenEditScreenshot);
                        return std::ptr::null_mut();
                    }
                    if cfg.screenshot_fullscreen_copy_enabled
                        && shortcuts_match(&combo, &cfg.screenshot_fullscreen_copy_shortcut)
                    {
                        notify_overlay(UiOverlayEvent::TriggerFullscreenCopyScreenshot);
                        return std::ptr::null_mut();
                    }
                }

                if cfg.clipboard_enabled && shortcuts_match(&combo, &cfg.clipboard_popup_shortcut) {
                    notify_overlay(UiOverlayEvent::ToggleClipboardLeftDrawer);
                    return std::ptr::null_mut();
                }

                if cfg.replace_macos_cmd_tab && !shortcuts_match(&combo, "Option+Tab") {
                    let is_alt_tab_custom = !cfg.alt_tab_shortcut.is_empty()
                        && shortcuts_match(&combo, &cfg.alt_tab_shortcut);
                    let is_cmd_tab = shortcuts_match(&combo, "Cmd+Tab");

                    if is_alt_tab_custom || is_cmd_tab {
                        ALTTAB_MODIFIER_HELD.store(true, Ordering::SeqCst);
                        notify_overlay(UiOverlayEvent::ToggleAltTabOverlay);
                        return std::ptr::null_mut();
                    }
                }

                if shortcuts_match(&combo, &cfg.mission_control_shortcut) {
                    notify_overlay_immediate(UiOverlayEvent::HideAllOverlays);
                    trigger_system_action("mission_control");
                    return std::ptr::null_mut();
                }

                if shortcuts_match(&combo, &cfg.show_desktop_shortcut) {
                    trigger_system_action("show_desktop");
                    return std::ptr::null_mut();
                }

                if !cfg.dashboard_shortcut.trim().is_empty()
                    && shortcuts_match(&combo, &cfg.dashboard_shortcut)
                {
                    notify_overlay(UiOverlayEvent::ToggleMainDashboard);
                    return std::ptr::null_mut();
                }

                if cfg.window_snapping_enabled {
                    let bindings = cached_window_shortcuts();
                    for b in bindings {
                        if b.enabled && shortcuts_match(&combo, &b.shortcut) {
                            trigger_window_snap_debounced(&b.action);
                            return std::ptr::null_mut();
                        }
                    }
                }
            }
        }
        K_CG_EVENT_FLAGS_CHANGED => {
            if !cfg.replace_macos_cmd_tab {
                ALTTAB_MODIFIER_HELD.store(false, Ordering::SeqCst);
                return event;
            }
            if ALTTAB_MODIFIER_HELD.load(Ordering::SeqCst) {
                let flags = CGEventGetFlags(event);
                if (flags & (FLAG_OPTION | FLAG_COMMAND)) == 0 {
                    ALTTAB_MODIFIER_HELD.store(false, Ordering::SeqCst);
                    notify_overlay_immediate(UiOverlayEvent::CommitAltTabSelection);
                }
            }
        }
        _ => {}
    }

    event
}

/// Start the global `CGEventTap` listener thread for `Cmd+Tab` replacement, Finder Cut/Paste, Mouse Chords, and Global Shortcuts.
/// Never prompts for Accessibility permission automatically on startup or in background retry loops:
/// `CGEventTapCreate` is only invoked after `core_window::is_accessibility_trusted()` is `true`, and at most once per trust session.
pub fn start_global_input_tap() {
    start_background_state_cache();
    if TAP_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::Builder::new()
        .name("global-cgeventtap".to_string())
        .spawn(|| unsafe {
            let mask: u64 = (1 << K_CG_EVENT_LEFT_MOUSE_DOWN)
                | (1 << K_CG_EVENT_LEFT_MOUSE_UP)
                | (1 << K_CG_EVENT_RIGHT_MOUSE_DOWN)
                | (1 << K_CG_EVENT_RIGHT_MOUSE_UP)
                | (1 << K_CG_EVENT_KEY_DOWN)
                | (1 << K_CG_EVENT_FLAGS_CHANGED)
                | (1 << K_CG_EVENT_SCROLL_WHEEL)
                | (1 << K_CG_EVENT_OTHER_MOUSE_DOWN)
                | (1 << K_CG_EVENT_OTHER_MOUSE_UP);

            loop {
                let trusted = core_window::is_accessibility_trusted();
                if !trusted {
                    TAP_ATTEMPTED_WHILE_TRUSTED.store(false, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    continue;
                }
                if !crate::config::get_input_config().needs_event_tap() {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    continue;
                }
                // Only attempt CGEventTapCreate once per trust session so we NEVER trigger an endless OS permission dialog loop.
                if TAP_ATTEMPTED_WHILE_TRUSTED.swap(true, Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    continue;
                }

                let tap = CGEventTapCreate(
                    K_CG_HID_EVENT_TAP,
                    K_CG_HEAD_INSERT_EVENT_TAP,
                    K_CG_EVENT_TAP_OPTION_DEFAULT,
                    mask,
                    event_tap_callback,
                    std::ptr::null_mut(),
                );

                if !tap.is_null() {
                    ACTIVE_TAP_PORT.store(tap as i64, Ordering::Relaxed);
                    crate::carbon_hotkeys::sync_window_tiling_carbon_hotkeys();
                    let src = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                    let rl = CFRunLoopGetCurrent();
                    CFRunLoopAddSource(rl, src, kCFRunLoopCommonModes);
                    let should_enable = core_window::is_accessibility_trusted()
                        && crate::config::get_input_config().needs_event_tap();
                    CGEventTapEnable(tap, should_enable);
                    CFRunLoopRun();
                    break;
                }
            }
        })
        .expect("Failed to spawn CGEventTap thread");
}

