//! `core-window`: Window Tiling (`1/2`, `1/4`, `2/3`, `Center`, `Maximize`, `Next Display`),
//! Alt-Tab Live Preview window enumeration, and macOS TCC permission helpers.

pub mod ffi;
pub mod models;
pub mod permissions;
pub mod shortcuts;
pub mod switcher;
pub mod tiling;

pub use ffi::{CGPoint, CGRect, CGSize};
pub use models::{OpenWindowItem, PermissionStatus, WindowShortcutBinding};
pub use permissions::{
    get_permissions_status, prompt_accessibility_if_needed, request_or_open_permission,
    reset_and_request_permissions,
};
pub use shortcuts::{
    default_window_shortcuts, get_window_shortcuts, reset_window_shortcuts, update_window_shortcut,
};
pub use switcher::{focus_window, list_open_windows};
pub use tiling::{
    compute_snap_rect, compute_snap_rect_for_displays, frontmost_application_pid,
    get_all_display_bounds, primary_display_height, register_own_window_snap_callback,
    snap_focused_window,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_snap_rect_layouts() {
        let displays = vec![
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: 1440.0,
                    height: 925.0, // usable sh = 900.0 after 25px menu bar
                },
            },
            CGRect {
                origin: CGPoint { x: 1440.0, y: 0.0 },
                size: CGSize {
                    width: 1920.0,
                    height: 1105.0, // usable sh = 1080.0
                },
            },
        ];

        // Left half on display 0
        let (x, y, w, h) = compute_snap_rect_for_displays(&displays, "left_half", 200.0, 200.0);
        assert_eq!((x, y, w, h), (0.0, 25.0, 720.0, 900.0));

        // Right half on display 0
        let (x, y, w, h) = compute_snap_rect_for_displays(&displays, "right_half", 200.0, 200.0);
        assert_eq!((x, y, w, h), (720.0, 25.0, 720.0, 900.0));

        // Center (75% width, full usable height) on display 0 (Ctrl+Option+C)
        let (cx, cy, cw, ch) = compute_snap_rect_for_displays(&displays, "center", 200.0, 200.0);
        assert_eq!(cw, 1080.0);
        assert_eq!(ch, 900.0);
        assert_eq!(cx, 180.0);
        assert_eq!(cy, 25.0);

        // Next display from display 0 -> moves to display 1
        let (nx, ny, nw, nh) =
            compute_snap_rect_for_displays(&displays, "next_display", 200.0, 200.0);
        assert_eq!((nx, ny, nw, nh), (1440.0, 25.0, 1920.0, 1080.0));
    }

    #[test]
    fn test_window_shortcuts_conflict_detection() {
        let defaults = default_window_shortcuts();
        assert_eq!(defaults.len(), 13);
        let center_binding = defaults
            .iter()
            .find(|b| b.action == "center")
            .expect("center action must exist");
        assert_eq!(center_binding.shortcut, "Ctrl+Option+C");

        // Trying to assign Ctrl+Option+Left (already used by left_half) to right_half should fail
        let _ = reset_window_shortcuts();
        let err = update_window_shortcut("right_half", "Ctrl+Option+Left", true);
        assert!(err.is_err());
    }
}
