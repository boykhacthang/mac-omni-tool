//! Data structures for Displays, Color Filters, and CoreAudio Devices.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioDeviceInfo {
    pub id: u32,
    pub name: String,
    pub kind_label: String, // "AirPods", "Tai nghe Bluetooth", "Loa ngoài MacBook", "Loa Màn hình", "USB Audio / DAC"
    pub icon: String,
    pub is_output: bool,
    pub is_input: bool,
    pub is_default_output: bool,
    pub is_default_input: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioSystemState {
    pub volume: f32, // 0.0 .. 1.0
    pub muted: bool,
    pub current_output_id: u32,
    pub current_output_name: String,
    pub output_devices: Vec<AudioDeviceInfo>,
    pub input_devices: Vec<AudioDeviceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub x: i32,
    pub y: i32,
    pub is_main: bool,
    pub is_builtin: bool,
    pub vendor_id: u32,
    pub model_id: u32,
    /// Brightness from 0.0 to 1.0 (100%)
    pub brightness: f32,
    /// Software dimming level from 0.15 to 1.0 (for external monitors or extra-dark mode)
    pub software_dimming: f32,
    /// Color warmth from 0.0 (normal) to 1.0 (warmest / Night Shift)
    pub warmth: f32,
    pub supports_hardware_brightness: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySystemState {
    pub displays: Vec<DisplayInfo>,
    pub dark_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayColorState {
    pub software_dimming: f32,
    pub warmth: f32,
}

impl Default for DisplayColorState {
    fn default() -> Self {
        Self {
            software_dimming: 1.0,
            warmth: 0.0,
        }
    }
}

