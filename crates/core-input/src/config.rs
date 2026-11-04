//! Configuration models, shortcut canonicalization, Carbon keycode mapping, and JSON persistence.

use crate::carbon_hotkeys::sync_window_tiling_carbon_hotkeys;
use crate::ffi::{
    CARBON_CMD_KEY, CARBON_CONTROL_KEY, CARBON_OPTION_KEY, CARBON_SHIFT_KEY, FLAG_COMMAND,
    FLAG_CONTROL, FLAG_OPTION, FLAG_SHIFT,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppMouseProfile {
    /// Bundle ID của ứng dụng (VD: "com.google.Chrome", "com.apple.Safari")
    pub bundle_id: String,
    /// Tên hiển thị của ứng dụng (VD: "Google Chrome")
    pub app_name: String,
    /// Biểu tượng hiển thị trên giao diện
    #[serde(default = "default_app_icon")]
    pub icon: String,
    /// Bật/tắt quy tắc riêng cho ứng dụng này
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Hành động cho nút Tiến lên (Mouse 5 / Button 4)
    #[serde(default = "default_action_default")]
    pub forward_action: String,
    /// Hành động cho nút Trở về (Mouse 4 / Button 3)
    #[serde(default = "default_action_default")]
    pub back_action: String,
    /// Hành động cho nút Giữa / Con lăn (Mouse 3 / Button 2)
    #[serde(default = "default_action_default")]
    pub middle_action: String,
    /// Chế độ cuộn mượt cho ứng dụng này: "inherit" (theo cài đặt chung), "on" (luôn bật), "off" (tắt riêng)
    #[serde(default = "default_inherit")]
    pub smooth_scroll_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InputAutomationConfig {
    /// Bật/tắt tính năng quản lý Clipboard & Popup Cmd+Shift+V
    #[serde(default = "default_true")]
    pub clipboard_enabled: bool,
    /// Bật/tắt tính năng Chia cửa sổ bằng phím tắt
    #[serde(default = "default_true")]
    pub window_snapping_enabled: bool,
    /// Bật/tắt Cmd+X và Cmd+V để cắt & dán tệp trong Finder
    pub finder_cut_paste_enabled: bool,
    /// Trạng thái hiện tại của bộ nhớ đệm cắt tệp Finder
    pub finder_cut_active: bool,
    /// Bật/tắt ấn Chuột Trái + Chuột Phải cùng lúc
    pub mouse_chord_enabled: bool,
    /// Hành động khi ấn Chuột Trái + Chuột Phải cùng lúc
    pub mouse_chord_action: String,
    /// Độ nhạy đồng thời giữa 2 nút chuột (ms), mặc định 120ms
    pub mouse_chord_threshold_ms: i64,
    /// Phím tắt gọi Mission Control (F3) trên bàn phím thường
    pub mission_control_shortcut: String,
    /// Phím tắt gọi Show Desktop trên bàn phím thường
    pub show_desktop_shortcut: String,
    /// Phím tắt bật Popup Clipboard bên trái màn hình
    pub clipboard_popup_shortcut: String,
    /// Phím tắt bật cửa sổ Alt-Tab Live Preview
    pub alt_tab_shortcut: String,
    /// Thay thế Cmd+Tab bằng Live Preview độc lập
    #[serde(default = "default_true")]
    pub replace_macos_cmd_tab: bool,
    /// Bật/tắt tổng tính năng chụp màn hình (mặc định: Bật)
    #[serde(default = "default_true")]
    pub screenshot_enabled: bool,
    /// Bật/tắt chế độ 1: Chụp 1 vùng trên cả 2 màn hình cùng lúc
    #[serde(default = "default_true")]
    pub screenshot_region_enabled: bool,
    /// Phím tắt chế độ 1: Chụp 1 vùng (mặc định: Cmd+Shift+0)
    #[serde(default = "default_screenshot_shortcut")]
    pub screenshot_shortcut: String,
    /// Bật/tắt chế độ 2: Chụp toàn màn hình đã chọn + Mở công cụ vẽ chú thích
    #[serde(default = "default_true")]
    pub screenshot_fullscreen_edit_enabled: bool,
    /// Phím tắt chế độ 2: Toàn màn hình + Vẽ chú thích (mặc định: Cmd+Shift+9)
    #[serde(default = "default_screenshot_fullscreen_edit_shortcut")]
    pub screenshot_fullscreen_edit_shortcut: String,
    /// Bật/tắt chế độ 3: Chụp toàn màn hình đã chọn + Copy ngay vào Clipboard
    #[serde(default = "default_true")]
    pub screenshot_fullscreen_copy_enabled: bool,
    /// Phím tắt chế độ 3: Toàn màn hình + Copy ngay (mặc định: Cmd+Shift+8)
    #[serde(default = "default_screenshot_fullscreen_copy_shortcut")]
    pub screenshot_fullscreen_copy_shortcut: String,
    /// Chế độ mặc định khi gọi từ nút/menu: "region", "fullscreen_edit", "fullscreen_copy"
    #[serde(default = "default_screenshot_mode")]
    pub screenshot_mode: String,
    /// ID màn hình được chọn khi chụp toàn màn hình (0 = theo con trỏ chuột)
    #[serde(default)]
    pub screenshot_display_id: u32,

    // -------------------------------------------------------------------------
    // External Mouse Customization, Smooth Scrolling (120Hz) & Per-App Profiles
    // -------------------------------------------------------------------------
    /// Công tắc tổng cho toàn bộ tính năng Chuột rời (Smooth Scroll + Gán nút chuột)
    #[serde(default = "default_true")]
    pub external_mouse_enabled: bool,
    /// Bật/tắt Cuộn chuột mượt mà 120Hz cho con lăn chuột rời
    #[serde(default = "default_true")]
    pub smooth_scroll_enabled: bool,
    /// Quãng đường mỗi nấc cuộn vật lý (px), mặc định 45px (15..=120)
    #[serde(default = "default_scroll_step_px")]
    pub scroll_step_px: u32,
    /// Thời gian lướt / quán tính mỗi lần cuộn (ms), mặc định 260ms (80..=600)
    #[serde(default = "default_scroll_duration_ms")]
    pub scroll_duration_ms: u32,
    /// Hệ số gia tốc khi cuộn nhanh liên tiếp (1.0..=3.0), mặc định 1.6
    #[serde(default = "default_scroll_acceleration")]
    pub scroll_acceleration: f32,
    /// Đảo chiều cuộn riêng cho Chuột rời (không làm đảo chiều Trackpad MacBook)
    #[serde(default)]
    pub invert_external_mouse_scroll: bool,
    /// Giữ phím Shift + Cuộn dọc để cuộn ngang
    #[serde(default = "default_true")]
    pub shift_horizontal_scroll: bool,
    /// Bật/tắt tính năng gán hành động cho nút phụ trên chuột (Mouse 3 / 4 / 5)
    #[serde(default = "default_true")]
    pub mouse_buttons_enabled: bool,
    /// Hành động mặc định cho nút Tiến lên (Mouse 5 / Button 4) ở các App khác
    #[serde(default = "default_mouse_forward_action")]
    pub mouse_forward_action: String,
    /// Hành động mặc định cho nút Trở về (Mouse 4 / Button 3) ở các App khác
    #[serde(default = "default_mouse_back_action")]
    pub mouse_back_action: String,
    /// Hành động mặc định cho nút Giữa / Con lăn (Mouse 3 / Button 2) ở các App khác
    #[serde(default = "default_action_default")]
    pub mouse_middle_action: String,
    /// Xử lý khi nút được gán chuyển Desktop (Space) nhưng chỉ có 1 Desktop đang mở:
    /// "passthrough" (giữ mặc định của App), "ignore" (bỏ qua), "mission_control" (mở Mission Control)
    #[serde(default = "default_single_space_fallback")]
    pub single_space_fallback: String,
    /// Danh sách cấu hình riêng theo từng ứng dụng (Per-App Mouse Profiles)
    #[serde(default = "default_app_mouse_profiles")]
    pub app_mouse_profiles: Vec<AppMouseProfile>,
}

fn default_true() -> bool {
    true
}

fn default_app_icon() -> String {
    "🧩".to_string()
}

fn default_action_default() -> String {
    "default".to_string()
}

fn default_inherit() -> String {
    "inherit".to_string()
}

fn default_scroll_step_px() -> u32 {
    45
}

fn default_scroll_duration_ms() -> u32 {
    260
}

fn default_scroll_acceleration() -> f32 {
    1.6
}

fn default_mouse_forward_action() -> String {
    "space_right".to_string()
}

fn default_mouse_back_action() -> String {
    "space_left".to_string()
}

fn default_single_space_fallback() -> String {
    "passthrough".to_string()
}

pub fn default_app_mouse_profiles() -> Vec<AppMouseProfile> {
    vec![
        AppMouseProfile {
            bundle_id: "com.google.Chrome".to_string(),
            app_name: "Google Chrome".to_string(),
            icon: "🧭".to_string(),
            enabled: true,
            forward_action: "default".to_string(),
            back_action: "default".to_string(),
            middle_action: "default".to_string(),
            smooth_scroll_mode: "inherit".to_string(),
        },
        AppMouseProfile {
            bundle_id: "com.apple.Safari".to_string(),
            app_name: "Safari".to_string(),
            icon: "🧭".to_string(),
            enabled: true,
            forward_action: "browser_forward".to_string(),
            back_action: "browser_back".to_string(),
            middle_action: "default".to_string(),
            smooth_scroll_mode: "inherit".to_string(),
        },
        AppMouseProfile {
            bundle_id: "company.thebrowser.Browser".to_string(),
            app_name: "Arc Browser".to_string(),
            icon: "🌈".to_string(),
            enabled: true,
            forward_action: "default".to_string(),
            back_action: "default".to_string(),
            middle_action: "default".to_string(),
            smooth_scroll_mode: "inherit".to_string(),
        },
        AppMouseProfile {
            bundle_id: "com.brave.Browser".to_string(),
            app_name: "Brave Browser".to_string(),
            icon: "🦁".to_string(),
            enabled: true,
            forward_action: "default".to_string(),
            back_action: "default".to_string(),
            middle_action: "default".to_string(),
            smooth_scroll_mode: "inherit".to_string(),
        },
        AppMouseProfile {
            bundle_id: "com.microsoft.edgemac".to_string(),
            app_name: "Microsoft Edge".to_string(),
            icon: "🌊".to_string(),
            enabled: true,
            forward_action: "default".to_string(),
            back_action: "default".to_string(),
            middle_action: "default".to_string(),
            smooth_scroll_mode: "inherit".to_string(),
        },
        AppMouseProfile {
            bundle_id: "org.mozilla.firefox".to_string(),
            app_name: "Firefox".to_string(),
            icon: "🦊".to_string(),
            enabled: true,
            forward_action: "default".to_string(),
            back_action: "default".to_string(),
            middle_action: "default".to_string(),
            smooth_scroll_mode: "inherit".to_string(),
        },
    ]
}

fn default_screenshot_shortcut() -> String {
    "Cmd+Shift+0".to_string()
}

fn default_screenshot_fullscreen_edit_shortcut() -> String {
    "Cmd+Shift+9".to_string()
}

fn default_screenshot_fullscreen_copy_shortcut() -> String {
    "Cmd+Shift+8".to_string()
}

fn default_screenshot_mode() -> String {
    "region".to_string()
}

impl Default for InputAutomationConfig {
    fn default() -> Self {
        Self {
            clipboard_enabled: true,
            window_snapping_enabled: true,
            finder_cut_paste_enabled: true,
            finder_cut_active: false,
            mouse_chord_enabled: true,
            mouse_chord_action: "mission_control".to_string(),
            mouse_chord_threshold_ms: 120,
            mission_control_shortcut: "F3".to_string(),
            show_desktop_shortcut: "F11".to_string(),
            clipboard_popup_shortcut: "Cmd+Shift+V".to_string(),
            alt_tab_shortcut: "Cmd+Tab".to_string(),
            replace_macos_cmd_tab: true,
            screenshot_enabled: true,
            screenshot_region_enabled: true,
            screenshot_shortcut: default_screenshot_shortcut(),
            screenshot_fullscreen_edit_enabled: true,
            screenshot_fullscreen_edit_shortcut: default_screenshot_fullscreen_edit_shortcut(),
            screenshot_fullscreen_copy_enabled: true,
            screenshot_fullscreen_copy_shortcut: default_screenshot_fullscreen_copy_shortcut(),
            screenshot_mode: default_screenshot_mode(),
            screenshot_display_id: 0,
            external_mouse_enabled: true,
            smooth_scroll_enabled: true,
            scroll_step_px: default_scroll_step_px(),
            scroll_duration_ms: default_scroll_duration_ms(),
            scroll_acceleration: default_scroll_acceleration(),
            invert_external_mouse_scroll: false,
            shift_horizontal_scroll: true,
            mouse_buttons_enabled: true,
            mouse_forward_action: default_mouse_forward_action(),
            mouse_back_action: default_mouse_back_action(),
            mouse_middle_action: default_action_default(),
            single_space_fallback: default_single_space_fallback(),
            app_mouse_profiles: default_app_mouse_profiles(),
        }
    }
}

impl InputAutomationConfig {
    /// Resolve the action assigned to an external mouse button (`2` = Middle, `3` = Back/Mouse4, `4` = Forward/Mouse5)
    /// for the specified frontmost application `bundle_id`.
    pub fn resolve_mouse_button_action<'a>(&'a self, bundle_id: &str, button_number: i64) -> &'a str {
        if !self.external_mouse_enabled || !self.mouse_buttons_enabled {
            return "default";
        }
        if !bundle_id.is_empty() {
            if let Some(prof) = self
                .app_mouse_profiles
                .iter()
                .find(|p| p.enabled && p.bundle_id.eq_ignore_ascii_case(bundle_id))
            {
                let act = match button_number {
                    4 => prof.forward_action.as_str(),
                    3 => prof.back_action.as_str(),
                    2 => prof.middle_action.as_str(),
                    _ => "default",
                };
                if !act.is_empty() {
                    return act;
                }
            }
        }
        match button_number {
            4 => self.mouse_forward_action.as_str(),
            3 => self.mouse_back_action.as_str(),
            2 => self.mouse_middle_action.as_str(),
            _ => "default",
        }
    }

    /// Determine whether 120Hz smooth scrolling is active for the given frontmost `bundle_id`.
    pub fn is_smooth_scroll_enabled_for_app(&self, bundle_id: &str) -> bool {
        if !self.external_mouse_enabled || !self.smooth_scroll_enabled {
            return false;
        }
        if !bundle_id.is_empty() {
            if let Some(prof) = self
                .app_mouse_profiles
                .iter()
                .find(|p| p.enabled && p.bundle_id.eq_ignore_ascii_case(bundle_id))
            {
                match prof.smooth_scroll_mode.as_str() {
                    "off" => return false,
                    "on" => return true,
                    _ => {}
                }
            }
        }
        self.smooth_scroll_enabled
    }

    /// Return `true` when at least one feature that requires the global `CGEventTap` is enabled.
    pub fn needs_event_tap(&self) -> bool {
        let screenshot_any = self.screenshot_enabled
            && (self.screenshot_region_enabled
                || self.screenshot_fullscreen_edit_enabled
                || self.screenshot_fullscreen_copy_enabled);
        let mouse_any = self.external_mouse_enabled
            && (self.smooth_scroll_enabled
                || self.invert_external_mouse_scroll
                || self.shift_horizontal_scroll
                || self.mouse_buttons_enabled
                || self.mouse_chord_enabled);
        mouse_any
            || self.finder_cut_paste_enabled
            || self.replace_macos_cmd_tab
            || self.window_snapping_enabled
            || self.clipboard_enabled
            || screenshot_any
            || !self.mission_control_shortcut.trim().is_empty()
            || !self.show_desktop_shortcut.trim().is_empty()
    }

    /// Return `true` when at least one feature that requires macOS Accessibility (`AXIsProcessTrusted`) is enabled.
    pub fn needs_accessibility(&self) -> bool {
        let mouse_any = self.external_mouse_enabled
            && (self.smooth_scroll_enabled
                || self.invert_external_mouse_scroll
                || self.shift_horizontal_scroll
                || self.mouse_buttons_enabled
                || self.mouse_chord_enabled);
        mouse_any
            || self.finder_cut_paste_enabled
            || self.replace_macos_cmd_tab
            || self.window_snapping_enabled
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiOverlayEvent {
    ToggleClipboardLeftDrawer,
    ToggleAltTabOverlay,
    CommitAltTabSelection,
    TriggerScreenshotOverlay,
    TriggerFullscreenEditScreenshot,
    TriggerFullscreenCopyScreenshot,
    SnapOwnWindow(String),
    HideAllOverlays,
}

pub(crate) static CONFIG_STORE: OnceLock<Mutex<InputAutomationConfig>> = OnceLock::new();
pub(crate) static FINDER_CUT_PENDING: AtomicBool = AtomicBool::new(false);

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("MacOmniTool");
    let _ = fs::create_dir_all(&dir);
    dir.join("input_automation.json")
}

/// Canonicalize a shortcut string into standard modifier order (`Ctrl+Option+Cmd+Shift+Key`).
pub fn canonicalize_shortcut(s: &str) -> String {
    let mut ctrl = false;
    let mut opt = false;
    let mut cmd = false;
    let mut shift = false;
    let mut keys = Vec::new();
    for part in s.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "opt" | "option" | "alt" => opt = true,
            "cmd" | "command" | "meta" => cmd = true,
            "shift" => shift = true,
            _ => keys.push(part.to_string()),
        }
    }
    let mut out = Vec::new();
    if ctrl {
        out.push("Ctrl".to_string());
    }
    if opt {
        out.push("Option".to_string());
    }
    if cmd {
        out.push("Cmd".to_string());
    }
    if shift {
        out.push("Shift".to_string());
    }
    out.extend(keys);
    out.join("+")
}

fn normalize_config(cfg: &mut InputAutomationConfig) {
    cfg.mission_control_shortcut = canonicalize_shortcut(&cfg.mission_control_shortcut);
    cfg.show_desktop_shortcut = canonicalize_shortcut(&cfg.show_desktop_shortcut);
    cfg.clipboard_popup_shortcut = canonicalize_shortcut(&cfg.clipboard_popup_shortcut);
    cfg.screenshot_shortcut = canonicalize_shortcut(&cfg.screenshot_shortcut);
    cfg.screenshot_fullscreen_edit_shortcut =
        canonicalize_shortcut(&cfg.screenshot_fullscreen_edit_shortcut);
    cfg.screenshot_fullscreen_copy_shortcut =
        canonicalize_shortcut(&cfg.screenshot_fullscreen_copy_shortcut);
    let canon_alt = canonicalize_shortcut(&cfg.alt_tab_shortcut);
    cfg.alt_tab_shortcut = if canon_alt.eq_ignore_ascii_case("Option+Tab") {
        "Cmd+Tab".to_string()
    } else {
        canon_alt
    };

    cfg.scroll_step_px = cfg.scroll_step_px.clamp(15, 120);
    cfg.scroll_duration_ms = cfg.scroll_duration_ms.clamp(80, 600);
    cfg.scroll_acceleration = cfg.scroll_acceleration.clamp(1.0, 3.0);
    if cfg.mouse_forward_action.trim().is_empty() {
        cfg.mouse_forward_action = default_mouse_forward_action();
    }
    if cfg.mouse_back_action.trim().is_empty() {
        cfg.mouse_back_action = default_mouse_back_action();
    }
    if cfg.mouse_middle_action.trim().is_empty() {
        cfg.mouse_middle_action = default_action_default();
    }
    if cfg.single_space_fallback.trim().is_empty() {
        cfg.single_space_fallback = default_single_space_fallback();
    }
}

pub(crate) fn config_mutex() -> &'static Mutex<InputAutomationConfig> {
    CONFIG_STORE.get_or_init(|| {
        let p = config_path();
        if let Ok(raw) = fs::read_to_string(&p) {
            if let Ok(mut cfg) = serde_json::from_str::<InputAutomationConfig>(&raw) {
                normalize_config(&mut cfg);
                if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                    let _ = fs::write(&p, json);
                }
                return Mutex::new(cfg);
            }
        }
        let def = InputAutomationConfig::default();
        if let Ok(json) = serde_json::to_string_pretty(&def) {
            let _ = fs::write(&p, json);
        }
        Mutex::new(def)
    })
}

pub fn get_input_config() -> InputAutomationConfig {
    let mut cfg = config_mutex().lock().unwrap().clone();
    cfg.finder_cut_active = FINDER_CUT_PENDING.load(Ordering::Relaxed);
    cfg
}

pub fn update_input_config(mut new_cfg: InputAutomationConfig) -> InputAutomationConfig {
    normalize_config(&mut new_cfg);
    if !new_cfg.finder_cut_paste_enabled {
        FINDER_CUT_PENDING.store(false, Ordering::SeqCst);
    }
    if !new_cfg.external_mouse_enabled || !new_cfg.smooth_scroll_enabled {
        crate::smooth_scroll::stop_smooth_scroll_momentum();
    }
    if !new_cfg.external_mouse_enabled || !new_cfg.mouse_buttons_enabled {
        crate::spaces::LAST_MOUSE_BUTTON.store(0, Ordering::Relaxed);
        crate::spaces::LAST_MOUSE_BUTTON_AT_MS.store(0, Ordering::Relaxed);
    }
    let mut guard = config_mutex().lock().unwrap();
    *guard = new_cfg;
    let mut cloned = guard.clone();
    cloned.finder_cut_active = FINDER_CUT_PENDING.load(Ordering::Relaxed);
    if let Ok(json) = serde_json::to_string_pretty(&cloned) {
        let _ = fs::write(config_path(), json);
    }
    drop(guard);
    sync_window_tiling_carbon_hotkeys();
    crate::event_tap::sync_event_tap_enabled(cloned.needs_event_tap());
    cloned
}

pub fn keycode_and_flags_to_shortcut(keycode: i64, flags: u64) -> Option<String> {
    let key_name = match keycode {
        0 => "A",
        1 => "S",
        2 => "D",
        3 => "F",
        4 => "H",
        5 => "G",
        6 => "Z",
        7 => "X",
        8 => "C",
        9 => "V",
        11 => "B",
        12 => "Q",
        13 => "W",
        14 => "E",
        15 => "R",
        16 => "Y",
        17 => "T",
        18 => "1",
        19 => "2",
        20 => "3",
        21 => "4",
        22 => "6",
        23 => "5",
        24 => "=",
        25 => "9",
        26 => "7",
        27 => "-",
        28 => "8",
        29 => "0",
        30 => "]",
        31 => "O",
        32 => "U",
        33 => "[",
        34 => "I",
        35 => "P",
        36 => "Enter",
        37 => "L",
        38 => "J",
        40 => "K",
        45 => "N",
        46 => "M",
        48 => "Tab",
        49 => "Space",
        53 => "Escape",
        96 => "F5",
        97 => "F6",
        98 => "F7",
        99 => "F3",
        100 => "F8",
        101 => "F9",
        103 => "F11",
        105 => "F13",
        107 => "F14",
        109 => "F10",
        111 => "F12",
        113 => "F15",
        118 => "F4",
        120 => "F2",
        122 => "F1",
        123 => "Left",
        124 => "Right",
        125 => "Down",
        126 => "Up",
        160 => "F3",
        _ => return None,
    };

    let mut parts = Vec::new();
    if (flags & FLAG_CONTROL) != 0 {
        parts.push("Ctrl");
    }
    if (flags & FLAG_OPTION) != 0 {
        parts.push("Option");
    }
    if (flags & FLAG_COMMAND) != 0 {
        parts.push("Cmd");
    }
    if (flags & FLAG_SHIFT) != 0 {
        parts.push("Shift");
    }
    parts.push(key_name);
    Some(parts.join("+"))
}

/// Convert a shortcut string (e.g. `"Ctrl+Option+C"`, `"Ctrl+Option+Left"`) into a Carbon `(keycode, modifiers)` pair.
pub fn shortcut_to_carbon_keycode_and_modifiers(shortcut: &str) -> Option<(u32, u32)> {
    let mut modifiers = 0u32;
    let mut keycode_opt = None;

    for part in shortcut
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= CARBON_CONTROL_KEY,
            "opt" | "option" | "alt" => modifiers |= CARBON_OPTION_KEY,
            "cmd" | "command" | "meta" => modifiers |= CARBON_CMD_KEY,
            "shift" => modifiers |= CARBON_SHIFT_KEY,
            k => {
                let code = match k {
                    "a" => 0,
                    "s" => 1,
                    "d" => 2,
                    "f" => 3,
                    "h" => 4,
                    "g" => 5,
                    "z" => 6,
                    "x" => 7,
                    "c" => 8,
                    "v" => 9,
                    "b" => 11,
                    "q" => 12,
                    "w" => 13,
                    "e" => 14,
                    "r" => 15,
                    "y" => 16,
                    "t" => 17,
                    "1" => 18,
                    "2" => 19,
                    "3" => 20,
                    "4" => 21,
                    "6" => 22,
                    "5" => 23,
                    "=" => 24,
                    "9" => 25,
                    "7" => 26,
                    "-" => 27,
                    "8" => 28,
                    "0" => 29,
                    "]" => 30,
                    "o" => 31,
                    "u" => 32,
                    "[" => 33,
                    "i" => 34,
                    "p" => 35,
                    "enter" | "return" => 36,
                    "l" => 37,
                    "j" => 38,
                    "k" => 40,
                    "n" => 45,
                    "m" => 46,
                    "tab" => 48,
                    "space" => 49,
                    "escape" | "esc" => 53,
                    "f5" => 96,
                    "f6" => 97,
                    "f7" => 98,
                    "f3" => 99,
                    "f8" => 100,
                    "f9" => 101,
                    "f11" => 103,
                    "f13" => 105,
                    "f14" => 107,
                    "f10" => 109,
                    "f12" => 111,
                    "f15" => 113,
                    "f4" => 118,
                    "f2" => 120,
                    "f1" => 122,
                    "left" => 123,
                    "right" => 124,
                    "down" => 125,
                    "up" => 126,
                    _ => return None,
                };
                keycode_opt = Some(code);
            }
        }
    }

    keycode_opt.map(|kc| (kc, modifiers))
}

/// Compare two shortcut strings regardless of modifier order or `Option`/`Alt` naming.
pub fn shortcuts_match(a: &str, b: &str) -> bool {
    if a.trim().is_empty() || b.trim().is_empty() {
        return false;
    }
    let norm = |s: &str| -> Vec<String> {
        let mut v: Vec<String> = s
            .split('+')
            .map(|p| {
                let t = p.trim().to_lowercase();
                if t == "alt" {
                    "option".to_string()
                } else if t == "command" || t == "meta" {
                    "cmd".to_string()
                } else if t == "control" {
                    "ctrl".to_string()
                } else {
                    t
                }
            })
            .filter(|t| !t.is_empty())
            .collect();
        v.sort();
        v
    };
    norm(a) == norm(b)
}
