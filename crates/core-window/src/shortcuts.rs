//! Customizable Window Tiling shortcut configuration, validation, and JSON persistence.

use crate::models::WindowShortcutBinding;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static SHORTCUTS_CACHE: OnceLock<Mutex<Vec<WindowShortcutBinding>>> = OnceLock::new();

fn config_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("MacOmniTool");
    let _ = fs::create_dir_all(&dir);
    dir.join("window_shortcuts.json")
}

pub fn default_window_shortcuts() -> Vec<WindowShortcutBinding> {
    vec![
        WindowShortcutBinding {
            action: "left_half".to_string(),
            label: "Nửa Trái (1/2 Left)".to_string(),
            category: "Chia 1/2".to_string(),
            shortcut: "Ctrl+Option+Left".to_string(),
            default_shortcut: "Ctrl+Option+Left".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "right_half".to_string(),
            label: "Nửa Phải (1/2 Right)".to_string(),
            category: "Chia 1/2".to_string(),
            shortcut: "Ctrl+Option+Right".to_string(),
            default_shortcut: "Ctrl+Option+Right".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "top_half".to_string(),
            label: "Nửa Trên (1/2 Top)".to_string(),
            category: "Chia 1/2".to_string(),
            shortcut: "Ctrl+Option+Up".to_string(),
            default_shortcut: "Ctrl+Option+Up".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "bottom_half".to_string(),
            label: "Nửa Dưới (1/2 Bottom)".to_string(),
            category: "Chia 1/2".to_string(),
            shortcut: "Ctrl+Option+Down".to_string(),
            default_shortcut: "Ctrl+Option+Down".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "maximize".to_string(),
            label: "Toàn Màn Hình (Maximize)".to_string(),
            category: "Bố Cục Lớn".to_string(),
            shortcut: "Ctrl+Option+Enter".to_string(),
            default_shortcut: "Ctrl+Option+Enter".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "center".to_string(),
            label: "Căn Giữa (Center 75%)".to_string(),
            category: "Bố Cục Lớn".to_string(),
            shortcut: "Ctrl+Option+C".to_string(),
            default_shortcut: "Ctrl+Option+C".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "top_left".to_string(),
            label: "Góc Trên Trái (1/4)".to_string(),
            category: "Chia 1/4 Góc".to_string(),
            shortcut: "Ctrl+Option+U".to_string(),
            default_shortcut: "Ctrl+Option+U".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "top_right".to_string(),
            label: "Góc Trên Phải (1/4)".to_string(),
            category: "Chia 1/4 Góc".to_string(),
            shortcut: "Ctrl+Option+I".to_string(),
            default_shortcut: "Ctrl+Option+I".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "bottom_left".to_string(),
            label: "Góc Dưới Trái (1/4)".to_string(),
            category: "Chia 1/4 Góc".to_string(),
            shortcut: "Ctrl+Option+J".to_string(),
            default_shortcut: "Ctrl+Option+J".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "bottom_right".to_string(),
            label: "Góc Dưới Phải (1/4)".to_string(),
            category: "Chia 1/4 Góc".to_string(),
            shortcut: "Ctrl+Option+K".to_string(),
            default_shortcut: "Ctrl+Option+K".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "two_thirds_left".to_string(),
            label: "2/3 Bên Trái".to_string(),
            category: "Tỷ Lệ 2/3 & Màn Hình".to_string(),
            shortcut: "Ctrl+Option+D".to_string(),
            default_shortcut: "Ctrl+Option+D".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "one_third_right".to_string(),
            label: "1/3 Bên Phải".to_string(),
            category: "Tỷ Lệ 2/3 & Màn Hình".to_string(),
            shortcut: "Ctrl+Option+F".to_string(),
            default_shortcut: "Ctrl+Option+F".to_string(),
            enabled: true,
        },
        WindowShortcutBinding {
            action: "next_display".to_string(),
            label: "Chuyển Sang Màn Hình Kế Tiếp".to_string(),
            category: "Tỷ Lệ 2/3 & Màn Hình".to_string(),
            shortcut: "Ctrl+Option+N".to_string(),
            default_shortcut: "Ctrl+Option+N".to_string(),
            enabled: true,
        },
    ]
}

fn shortcuts_store() -> &'static Mutex<Vec<WindowShortcutBinding>> {
    SHORTCUTS_CACHE.get_or_init(|| {
        let path = config_file_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<Vec<WindowShortcutBinding>>(&data) {
                if !parsed.is_empty() {
                    return Mutex::new(parsed);
                }
            }
        }
        let defaults = default_window_shortcuts();
        if let Ok(json) = serde_json::to_string_pretty(&defaults) {
            let _ = fs::write(&path, json);
        }
        Mutex::new(defaults)
    })
}

pub fn get_window_shortcuts() -> Vec<WindowShortcutBinding> {
    shortcuts_store().lock().unwrap().clone()
}

pub fn update_window_shortcut(
    action: &str,
    new_shortcut: &str,
    enabled: bool,
) -> Result<Vec<WindowShortcutBinding>, String> {
    let mut guard = shortcuts_store().lock().unwrap();
    let trimmed = new_shortcut.trim();

    if !trimmed.is_empty() {
        for item in guard.iter() {
            if item.action != action
                && item.enabled
                && item.shortcut.eq_ignore_ascii_case(trimmed)
            {
                return Err(format!(
                    "Phím tắt '{}' đã được gán cho bố cục '{}'. Vui lòng chọn tổ hợp phím khác!",
                    trimmed, item.label
                ));
            }
        }
    }

    if let Some(target) = guard.iter_mut().find(|i| i.action == action) {
        target.shortcut = trimmed.to_string();
        target.enabled = enabled;
    }

    let cloned = guard.clone();
    if let Ok(json) = serde_json::to_string_pretty(&cloned) {
        let _ = fs::write(config_file_path(), json);
    }
    Ok(cloned)
}

pub fn reset_window_shortcuts() -> Vec<WindowShortcutBinding> {
    let mut guard = shortcuts_store().lock().unwrap();
    *guard = default_window_shortcuts();
    let cloned = guard.clone();
    if let Ok(json) = serde_json::to_string_pretty(&cloned) {
        let _ = fs::write(config_file_path(), json);
    }
    cloned
}

