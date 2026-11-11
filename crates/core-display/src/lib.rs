//! `core-display`: Multi-monitor hardware/software brightness, Night Shift warmth filters,
//! macOS Dark/Light appearance switching, parallel multi-monitor screenshots, and CoreAudio management.

pub mod appearance;
pub mod audio;
pub mod brightness;
pub mod ffi;
pub mod models;
pub mod screenshot;

pub use appearance::{is_dark_mode, set_dark_mode};
pub use audio::{
    classify_audio_device, get_audio_system_state, set_audio_device, set_audio_volume,
};
pub use brightness::{
    compute_gamma_channels, get_display_system_state, list_displays, reset_all_display_filters,
    set_display_brightness, set_display_color_filter,
};
pub use ffi::{CGDirectDisplayID, CGError, CGGammaValue, CGPoint, CGRect, CGSize};
pub use models::{
    AudioDeviceInfo, AudioSystemState, DisplayColorState, DisplayInfo, DisplaySystemState,
};
pub use screenshot::{
    capture_all_displays_png, capture_all_displays_rgba8, capture_display_png,
    capture_display_rgba8, find_display_containing_point, get_display_at_mouse_cursor,
    get_mouse_cursor_position, CapturedDisplayRgba,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_gamma_channels_bounds_and_warmth() {
        let (r, g, b) = compute_gamma_channels(1.0, 0.0);
        assert_eq!((r, g, b), (1.0, 1.0, 1.0));

        // High warmth reduces blue more than green while preserving red
        let (rw, gw, bw) = compute_gamma_channels(0.8, 0.5);
        assert!((rw - 0.8).abs() < 1e-4);
        assert!(gw < rw);
        assert!(bw < gw);

        // Extreme minimum clamping never blacks out the screen (< 0.15)
        let (r_min, g_min, b_min) = compute_gamma_channels(0.0, 1.0);
        assert!(r_min >= 0.15);
        assert!(g_min >= 0.10);
        assert!(b_min >= 0.05);
    }

    #[test]
    fn test_classify_audio_device_types() {
        let (label, icon) = classify_audio_device("Thang's AirPods Pro", u32::from_be_bytes(*b"blut"));
        assert_eq!(label, "AirPods");
        assert_eq!(icon, "🎧");

        let (label2, icon2) = classify_audio_device("MacBook Pro Speakers", u32::from_be_bytes(*b"bltn"));
        assert_eq!(label2, "Loa ngoài MacBook");
        assert_eq!(icon2, "🔊");

        let (label3, icon3) = classify_audio_device("LG UltraFine 4K", u32::from_be_bytes(*b"dprt"));
        assert_eq!(label3, "Loa Màn hình");
        assert_eq!(icon3, "🖥️");

        let (label4, icon4) = classify_audio_device("FiiO K7 DAC", u32::from_be_bytes(*b"usb "));
        assert_eq!(label4, "USB Audio / DAC");
        assert_eq!(icon4, "🎛️");
    }

    #[test]
    fn test_find_display_containing_point_multi_monitor() {
        let displays = vec![
            DisplayInfo {
                id: 1,
                name: "Built-in".to_string(),
                width: 1512,
                height: 982,
                x: 0,
                y: 0,
                is_main: true,
                is_builtin: true,
                vendor_id: 1,
                model_id: 1,
                brightness: 1.0,
                software_dimming: 1.0,
                warmth: 0.0,
                supports_hardware_brightness: true,
            },
            DisplayInfo {
                id: 2,
                name: "External".to_string(),
                width: 2560,
                height: 1440,
                x: 1512,
                y: 0,
                is_main: false,
                is_builtin: false,
                vendor_id: 2,
                model_id: 2,
                brightness: 0.8,
                software_dimming: 0.8,
                warmth: 0.2,
                supports_hardware_brightness: false,
            },
        ];

        let d1 = find_display_containing_point(&displays, 500.0, 400.0);
        assert_eq!(d1.id, 1);

        let d2 = find_display_containing_point(&displays, 2000.0, 800.0);
        assert_eq!(d2.id, 2);

        // Out-of-bounds falls back to main display (id=1)
        let d_fallback = find_display_containing_point(&displays, -500.0, -500.0);
        assert_eq!(d_fallback.id, 1);
    }
}
