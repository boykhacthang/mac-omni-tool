//! Data models for Window Tiling Shortcuts, Open Window Switcher Items, and TCC Permission Status.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowShortcutBinding {
    pub action: String,
    pub label: String,
    pub category: String,
    pub shortcut: String,
    pub default_shortcut: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenWindowItem {
    pub window_id: u32,
    pub owner_pid: i32,
    pub app_name: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub preview_data_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionStatus {
    pub accessibility_granted: bool,
    pub screen_recording_granted: bool,
    pub full_disk_access_granted: bool,
}

