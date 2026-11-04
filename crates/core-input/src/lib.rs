//! `core-input`: Global Carbon HotKeys (`RegisterEventHotKey`), HID `CGEventTap` listener,
//! External Mouse Customization (120Hz Smooth Scroll, Independent Reverse Scroll, Per-App Button Profiles),
//! Finder Cut/Paste (`Cmd+X` -> `Cmd+Option+V`), Mouse Chords, and Shortcut canonicalization.

pub mod actions;
pub mod carbon_hotkeys;
pub mod config;
pub mod event_tap;
pub mod ffi;
pub mod smooth_scroll;
pub mod spaces;

pub use actions::{
    is_finder_frontmost, post_synthetic_key_combo, register_overlay_callback, trigger_system_action,
};
pub use carbon_hotkeys::{
    register_native_carbon_hotkeys, set_overlay_escape_hotkey_active, sync_feature_carbon_hotkeys,
    sync_window_tiling_carbon_hotkeys, trigger_window_snap_debounced,
};
pub use config::{
    canonicalize_shortcut, default_app_mouse_profiles, get_input_config,
    keycode_and_flags_to_shortcut, shortcut_to_carbon_keycode_and_modifiers, shortcuts_match,
    update_input_config, AppMouseProfile, InputAutomationConfig, UiOverlayEvent,
};
pub use event_tap::{start_global_input_tap, sync_event_tap_enabled};
pub use spaces::{
    frontmost_app_info, get_spaces_status, list_running_gui_apps, query_macos_spaces_counts,
    RunningAppItem, SpacesStatus,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::{CARBON_CONTROL_KEY, CARBON_OPTION_KEY, FLAG_CONTROL, FLAG_OPTION};

    #[test]
    fn test_canonicalize_and_shortcuts_match() {
        assert_eq!(canonicalize_shortcut("alt + ctrl + c"), "Ctrl+Option+c");
        assert!(shortcuts_match("Ctrl+Option+C", "Alt+Control+c"));
        assert!(shortcuts_match("Cmd+Shift+V", "Shift+Command+v"));
        assert!(!shortcuts_match("Ctrl+Option+C", "Ctrl+Cmd+C"));
    }

    #[test]
    fn test_shortcut_to_carbon_keycode_and_modifiers() {
        let (kc, mods) = shortcut_to_carbon_keycode_and_modifiers("Ctrl+Option+C")
            .expect("Ctrl+Option+C should map to Carbon keycode");
        assert_eq!(kc, 8); // 'C' = 8
        assert_eq!(mods, CARBON_CONTROL_KEY | CARBON_OPTION_KEY);

        let (kc_left, mods_left) = shortcut_to_carbon_keycode_and_modifiers("Ctrl+Option+Left")
            .expect("Ctrl+Option+Left should map to Carbon keycode");
        assert_eq!(kc_left, 123);
        assert_eq!(mods_left, CARBON_CONTROL_KEY | CARBON_OPTION_KEY);
    }

    #[test]
    fn test_keycode_and_flags_to_shortcut() {
        let combo = keycode_and_flags_to_shortcut(8, FLAG_CONTROL | FLAG_OPTION)
            .expect("Keycode 8 with Ctrl+Option should produce Ctrl+Option+C");
        assert_eq!(combo, "Ctrl+Option+C");
    }

    #[test]
    fn test_per_app_mouse_button_resolution_and_smooth_scroll() {
        let mut cfg = InputAutomationConfig::default();
        // Default: Chrome uses "default" (page forward/back), while other apps use "space_right" / "space_left"
        assert_eq!(
            cfg.resolve_mouse_button_action("com.google.Chrome", 4),
            "default"
        );
        assert_eq!(
            cfg.resolve_mouse_button_action("com.google.Chrome", 3),
            "default"
        );
        assert_eq!(
            cfg.resolve_mouse_button_action("com.apple.Terminal", 4),
            "space_right"
        );
        assert_eq!(
            cfg.resolve_mouse_button_action("com.apple.Terminal", 3),
            "space_left"
        );

        // When global smooth_scroll_enabled is false, even an app with smooth_scroll_mode = "on" must return false
        cfg.external_mouse_enabled = true;
        cfg.smooth_scroll_enabled = false;
        if let Some(p) = cfg.app_mouse_profiles.first_mut() {
            p.enabled = true;
            p.smooth_scroll_mode = "on".to_string();
        }
        assert!(!cfg.is_smooth_scroll_enabled_for_app("com.google.Chrome"));
        assert!(!cfg.is_smooth_scroll_enabled_for_app("com.apple.Terminal"));

        // When master switch is off, all buttons fall back to "default" and smooth scroll is false
        cfg.external_mouse_enabled = false;
        cfg.smooth_scroll_enabled = true;
        assert_eq!(
            cfg.resolve_mouse_button_action("com.apple.Terminal", 4),
            "default"
        );
        assert!(!cfg.is_smooth_scroll_enabled_for_app("com.apple.Terminal"));
    }

    #[test]
    fn test_needs_event_tap_when_all_features_disabled() {
        let mut cfg = InputAutomationConfig::default();
        assert!(cfg.needs_event_tap());

        cfg.finder_cut_paste_enabled = false;
        cfg.replace_macos_cmd_tab = false;
        cfg.external_mouse_enabled = false;
        cfg.clipboard_enabled = false;
        cfg.screenshot_enabled = false;
        cfg.window_snapping_enabled = false;
        cfg.mission_control_shortcut = String::new();
        cfg.show_desktop_shortcut = String::new();

        assert!(!cfg.needs_event_tap());
        assert!(!cfg.needs_accessibility());
    }

    #[test]
    fn test_spaces_status_and_running_apps_query() {
        let st = get_spaces_status();
        assert!(st.total_spaces >= 1);
        assert!(st.primary_display_spaces >= 1);
        assert!(st.current_space_index >= 1);

        let _apps = list_running_gui_apps();
    }
}
