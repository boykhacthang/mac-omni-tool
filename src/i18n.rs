use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static IS_VI: AtomicBool = AtomicBool::new(true);
static IS_LIGHT: AtomicBool = AtomicBool::new(false);
static PREFS_LOCK: OnceLock<Mutex<UiPrefs>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPrefs {
    pub lang: String,
    pub theme: String,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            lang: "vi".to_string(),
            theme: "dark".to_string(),
        }
    }
}

fn prefs_path() -> PathBuf {
    core_clipboard::app_data_dir().join("ui_prefs.json")
}

pub fn init_ui_prefs() -> UiPrefs {
    let loaded = std::fs::read_to_string(prefs_path())
        .ok()
        .and_then(|s| serde_json::from_str::<UiPrefs>(&s).ok())
        .unwrap_or_default();
    IS_VI.store(loaded.lang == "vi", Ordering::Relaxed);
    IS_LIGHT.store(loaded.theme == "light", Ordering::Relaxed);
    let lock = PREFS_LOCK.get_or_init(|| Mutex::new(loaded.clone()));
    if let Ok(mut guard) = lock.lock() {
        *guard = loaded.clone();
    }
    loaded
}

fn save_ui_prefs(prefs: &UiPrefs) {
    if let Ok(json) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(prefs_path(), json);
    }
}

pub fn is_vi() -> bool {
    IS_VI.load(Ordering::Relaxed)
}

#[allow(dead_code)]
pub fn current_lang() -> &'static str {
    if is_vi() {
        "vi"
    } else {
        "en"
    }
}

pub fn set_lang(lang: &str) {
    let vi = lang.eq_ignore_ascii_case("vi");
    IS_VI.store(vi, Ordering::Relaxed);
    let lock = PREFS_LOCK.get_or_init(|| Mutex::new(UiPrefs::default()));
    if let Ok(mut guard) = lock.lock() {
        guard.lang = if vi { "vi".to_string() } else { "en".to_string() };
        save_ui_prefs(&guard);
    }
}

pub fn is_light_theme() -> bool {
    IS_LIGHT.load(Ordering::Relaxed)
}

#[allow(dead_code)]
pub fn current_theme() -> &'static str {
    if is_light_theme() {
        "light"
    } else {
        "dark"
    }
}

pub fn set_theme(theme: &str) {
    let light = theme.eq_ignore_ascii_case("light");
    IS_LIGHT.store(light, Ordering::Relaxed);
    let lock = PREFS_LOCK.get_or_init(|| Mutex::new(UiPrefs::default()));
    if let Ok(mut guard) = lock.lock() {
        guard.theme = if light {
            "light".to_string()
        } else {
            "dark".to_string()
        };
        save_ui_prefs(&guard);
    }
}

/// Fast inline bilingual lookup (`en` vs `vi`).
#[inline]
pub fn tr<'a>(en: &'a str, vi: &'a str) -> &'a str {
    if is_vi() {
        vi
    } else {
        en
    }
}

pub fn translate_window_action(action: &str, fallback_label: &str) -> String {
    match action {
        "left_half" => tr("Left Half (1/2 Left)", "Nửa Trái (1/2 Left)").to_string(),
        "right_half" => tr("Right Half (1/2 Right)", "Nửa Phải (1/2 Right)").to_string(),
        "top_half" => tr("Top Half (1/2 Top)", "Nửa Trên (1/2 Top)").to_string(),
        "bottom_half" => tr("Bottom Half (1/2 Bottom)", "Nửa Dưới (1/2 Bottom)").to_string(),
        "maximize" => tr("Full Screen (Maximize)", "Toàn Màn Hình (Maximize)").to_string(),
        "center" => tr("Center (75%)", "Căn Giữa (Center 75%)").to_string(),
        "top_left" => tr("Top Left Quarter (1/4)", "Góc Trên Trái (1/4)").to_string(),
        "top_right" => tr("Top Right Quarter (1/4)", "Góc Trên Phải (1/4)").to_string(),
        "bottom_left" => tr("Bottom Left Quarter (1/4)", "Góc Dưới Trái (1/4)").to_string(),
        "bottom_right" => tr("Bottom Right Quarter (1/4)", "Góc Dưới Phải (1/4)").to_string(),
        "first_third" => tr("Left One-Third (1/3)", "1/3 Bên Trái").to_string(),
        "center_third" => tr("Center One-Third (1/3)", "1/3 Ở Giữa").to_string(),
        "last_third" | "one_third_right" => {
            tr("Right One-Third (1/3)", "1/3 Bên Phải").to_string()
        }
        "first_two_thirds" | "two_thirds_left" => {
            tr("Left Two-Thirds (2/3)", "2/3 Bên Trái").to_string()
        }
        "last_two_thirds" => tr("Right Two-Thirds (2/3)", "2/3 Bên Phải").to_string(),
        "next_display" => tr(
            "Move to Next Display",
            "Chuyển Sang Màn Hình Kế Tiếp",
        )
        .to_string(),
        _ => fallback_label.to_string(),
    }
}

pub fn translate_window_category(cat: &str) -> &'static str {
    match cat {
        "Chia 1/2" => tr("Halves (1/2)", "Chia 1/2"),
        "Bố Cục Lớn" => tr("Large Layouts", "Bố Cục Lớn"),
        "Chia 1/4 Góc" => tr("Quarters (1/4)", "Chia 1/4 Góc"),
        "Tỷ Lệ 2/3 & Màn Hình" => tr("2/3 Ratio & Displays", "Tỷ Lệ 2/3 & Màn Hình"),
        _ => tr("All Layouts", "Tất cả bố cục"),
    }
}

/// Complete 20 mouse button action options matching `ui/js/locales/en.js` & `vi.js`.
pub const MOUSE_ACTION_OPTIONS: &[&str] = &[
    "default",
    "space_right",
    "space_left",
    "browser_forward",
    "browser_back",
    "mission_control",
    "app_expose",
    "show_desktop",
    "launchpad",
    "tab_next",
    "tab_prev",
    "close_tab",
    "new_tab",
    "screenshot",
    "screenshot_full_edit",
    "screenshot_full_copy",
    "clipboard_popup",
    "alt_tab",
    "open_dashboard",
    "disabled",
];

pub fn translate_mouse_action(action: &str) -> &'static str {
    match action {
        "default" => tr(
            "🌐 Default App Behavior (Pass-through)",
            "🌐 Mặc định của App (Tiến/Lùi trang)",
        ),
        "space_right" => tr(
            "🖥 Next Desktop / Space → (If >1 Space)",
            "🖥 Chuyển sang Desktop phải → (Nếu >1 Desktop)",
        ),
        "space_left" => tr(
            "🖥 ← Previous Desktop / Space (If >1 Space)",
            "🖥 ← Chuyển về Desktop trái (Nếu >1 Desktop)",
        ),
        "browser_forward" => tr(
            "🌐 Browser / Finder Forward (Cmd+])",
            "🌐 Tiến tới trang / thư mục sau (Cmd+])",
        ),
        "browser_back" => tr(
            "🌐 Browser / Finder Back (Cmd+[)",
            "🌐 Quay về trang / thư mục trước (Cmd+[)",
        ),
        "mission_control" => tr("🚀 Mission Control (F3)", "🚀 Mở Mission Control (F3)"),
        "app_expose" => tr("❐ App Exposé", "❐ Mở App Exposé"),
        "show_desktop" => tr("🖥 Show Desktop (F11)", "🖥 Hiện Desktop (F11)"),
        "launchpad" => tr("🎛 Launchpad", "🎛 Mở Launchpad"),
        "tab_next" => tr(
            "📑 Next Tab (Ctrl+Tab)",
            "📑 Chuyển sang Tab kế tiếp (Ctrl+Tab)",
        ),
        "tab_prev" => tr(
            "📑 Previous Tab (Ctrl+Shift+Tab)",
            "📑 Quay lại Tab trước (Ctrl+Shift+Tab)",
        ),
        "close_tab" => tr("✕ Close Tab (Cmd+W)", "✕ Đóng Tab hiện tại (Cmd+W)"),
        "new_tab" => tr("➕ New Tab (Cmd+T)", "➕ Mở Tab mới (Cmd+T)"),
        "screenshot" => tr(
            "📸 Screenshot (Region Capture)",
            "📸 Chụp màn hình (Chọn vùng)",
        ),
        "screenshot_full_edit" => tr(
            "🖼 Fullscreen Screenshot + Annotate",
            "🖼 Chụp toàn màn hình + Vẽ chú thích",
        ),
        "screenshot_full_copy" => tr(
            "⚡ Fullscreen Screenshot + Instant Copy",
            "⚡ Chụp toàn màn hình + Copy ngay",
        ),
        "clipboard_popup" => tr("📋 Clipboard Drawer", "📋 Mở ngăn kéo Clipboard"),
        "alt_tab" => tr(
            "❐ Alt-Tab Window Switcher",
            "❐ Mở chuyển cửa sổ Alt-Tab",
        ),
        "open_dashboard" => tr(
            "🚀 Open Mac Omni Tool Dashboard",
            "🚀 Mở Bảng điều khiển Mac Omni Tool",
        ),
        "copy" => tr("📋 Copy (Cmd+C)", "📋 Sao chép (Cmd+C)"),
        "paste" => tr("📋 Paste (Cmd+V)", "📋 Dán (Cmd+V)"),
        "disabled" | "none" => tr(
            "🚫 Disabled (Ignore Button)",
            "🚫 Tắt nút này (Không làm gì)",
        ),
        _ => tr(
            "🌐 Default App Behavior",
            "🌐 Mặc định của App",
        ),
    }
}

pub const CHORD_ACTION_OPTIONS: &[&str] = &[
    "mission_control",
    "app_expose",
    "show_desktop",
    "launchpad",
    "clipboard_popup",
    "alt_tab",
    "screenshot",
];

pub fn translate_chord_action(action: &str) -> &'static str {
    match action {
        "mission_control" => tr("🚀 Mission Control", "🚀 Mở Mission Control"),
        "app_expose" => tr("❐ App Exposé", "❐ Mở App Exposé"),
        "show_desktop" => tr("🖥 Show Desktop", "🖥 Hiện Màn hình chính (Desktop)"),
        "launchpad" => tr("🎛 Launchpad", "🎛 Mở Launchpad"),
        "clipboard_popup" => tr("📋 Clipboard Drawer", "📋 Mở ngăn kéo Clipboard"),
        "alt_tab" => tr("❐ Alt-Tab Switcher", "❐ Mở chuyển cửa sổ Alt-Tab"),
        "screenshot" => tr("📸 Region Screenshot", "📸 Chụp màn hình vùng chọn"),
        _ => tr("🚀 Mission Control", "🚀 Mở Mission Control"),
    }
}

pub const SINGLE_SPACE_FALLBACK_OPTIONS: &[&str] = &[
    "passthrough",
    "ignore",
    "mission_control",
];

pub fn translate_single_space_fallback(mode: &str) -> &'static str {
    match mode {
        "passthrough" => tr(
            "Pass-through to App (Browser Back/Forward)",
            "Giữ nguyên nút gốc cho App (Tiến/Lùi trang)",
        ),
        "ignore" => tr(
            "Ignore (Do Nothing When Only 1 Space)",
            "Bỏ qua (Không làm gì khi chỉ có 1 Desktop)",
        ),
        "mission_control" => tr(
            "Open Mission Control Instead",
            "Mở Mission Control để tạo thêm Desktop",
        ),
        _ => tr(
            "Pass-through to App (Browser Back/Forward)",
            "Giữ nguyên nút gốc cho App (Tiến/Lùi trang)",
        ),
    }
}

pub const MENUBAR_MODE_OPTIONS: &[&str] = &[
    "temp_cpu",
    "temp_cpu_ram",
    "cpu_ram",
    "cpu_only",
    "temp_only",
    "ram_only",
    "ram_gb",
    "net_speed",
    "temp_net",
    "icon_only",
    "icon_name",
    "custom",
    "hidden",
];

pub fn translate_menubar_mode(mode: &str) -> &'static str {
    match mode {
        "temp_cpu" => tr(
            "🌡 Temp + CPU % (Default)",
            "🌡 Nhiệt độ + % CPU (Mặc định)",
        ),
        "temp_cpu_ram" => tr(
            "📊 Temp + CPU % + RAM %",
            "📊 Nhiệt độ + % CPU + % RAM",
        ),
        "cpu_ram" => tr("⚡ CPU % + RAM %", "⚡ % CPU + % RAM"),
        "cpu_only" => tr("⚡ CPU % Only", "⚡ Chỉ hiện % CPU"),
        "temp_only" => tr("🌡 CPU Temp Only (°C)", "🌡 Chỉ hiện Nhiệt độ (°C)"),
        "ram_only" => tr("💾 RAM % Only", "💾 Chỉ hiện % RAM"),
        "ram_gb" => tr(
            "💾 RAM Used / Total (GB)",
            "💾 Dung lượng RAM (Đang dùng/Tổng GB)",
        ),
        "net_speed" => tr(
            "🌐 Network Speed (↓/↑)",
            "🌐 Tốc độ Mạng (↓ Tải xuống / ↑ Tải lên)",
        ),
        "temp_net" => tr(
            "🌡 Temp + 🌐 Net Speed",
            "🌡 Nhiệt độ + 🌐 Tốc độ Mạng",
        ),
        "icon_only" => tr(
            "❖ Icon Only (Minimal)",
            "❖ Chỉ hiện Biểu tượng (Tối giản)",
        ),
        "icon_name" => tr("❖ Icon + Omni", "❖ Biểu tượng + chữ Omni"),
        "custom" => tr(
            "⚙ Custom Metrics...",
            "⚙ Tự chọn thành phần hiển thị...",
        ),
        "hidden" => tr(
            "🙈 Hide Menu Bar Icon",
            "🙈 Ẩn khỏi thanh Menu Bar (Chạy ngầm)",
        ),
        _ => tr(
            "🌡 Temp + CPU % (Default)",
            "🌡 Nhiệt độ + % CPU (Mặc định)",
        ),
    }
}

pub fn translate_audio_kind(kind: &str) -> String {
    match kind {
        "Loa ngoài MacBook" => tr("MacBook Speakers", "Loa ngoài MacBook").to_string(),
        "Micro MacBook" => tr("MacBook Microphone", "Micro MacBook").to_string(),
        "Tai nghe Bluetooth" => tr("Bluetooth Headphones", "Tai nghe Bluetooth").to_string(),
        "Loa Màn hình" => tr("Display Audio", "Loa Màn hình").to_string(),
        "Tai nghe có dây" => tr("Wired Headphones", "Tai nghe có dây").to_string(),
        "Thiết bị Âm thanh" | "Thiết bị âm thanh" => {
            tr("Audio Device", "Thiết bị âm thanh").to_string()
        }
        other => other.to_string(),
    }
}

pub fn translate_cleaner_category(cat: &str) -> String {
    match cat {
        "User App Cache" => tr("User App Cache", "Cache ứng dụng người dùng").to_string(),
        "App & System Logs" => tr(
            "App & System Logs",
            "Nhật ký hệ thống & ứng dụng (Logs)",
        )
        .to_string(),
        "Xcode DerivedData" => "Xcode DerivedData".to_string(),
        "Cargo Cache" => tr("Cargo Cache (Rust)", "Cargo Cache (Rust)").to_string(),
        "npm Cache" => tr("npm Cache (Node.js)", "npm Cache (Node.js)").to_string(),
        "Trash (Thùng rác)" => tr("Trash (~/.Trash)", "Trash (Thùng rác)").to_string(),
        "File SQL (.sql)" => tr("SQL File (.sql)", "File SQL (.sql)").to_string(),
        "Hình ảnh" | "Image" => tr("Image", "Hình ảnh").to_string(),
        "Video" => "Video".to_string(),
        "Âm thanh" | "Audio" => tr("Audio", "Âm thanh").to_string(),
        "Tài liệu" | "Document" => tr("Document", "Tài liệu").to_string(),
        "Nén / Bộ cài" => tr("Archive / Installer", "Nén / Bộ cài").to_string(),
        "Khác" | "Other" => tr("Other", "Khác").to_string(),
        "Phần mềm / Bộ cài" => tr("Installer / Disk Image", "Phần mềm / Bộ cài").to_string(),
        "Tệp nén" => tr("Archive", "Tệp nén").to_string(),
        "Dữ liệu" => tr("Data File", "Dữ liệu").to_string(),
        "Disk Image (.dmg/.iso)" => {
            tr("Disk Image (.dmg/.iso)", "Bộ cài đĩa ảo (.dmg/.iso)").to_string()
        }
        "Archive (.zip/.tar)" => tr("Archive (.zip/.tar)", "Tệp nén (.zip/.tar)").to_string(),
        "Data / Model / VM" => {
            tr("Data / Model / VM", "Dữ liệu / Model / Máy ảo").to_string()
        }
        "Application Support" => {
            tr("Application Support", "Dữ liệu hỗ trợ (App Support)").to_string()
        }
        "Preferences" => tr("Preferences (.plist)", "Tệp cấu hình (Preferences)").to_string(),
        "Caches" => tr("Caches", "Bộ nhớ đệm (Caches)").to_string(),
        "Saved State" => tr("Saved State", "Trạng thái lưu (Saved State)").to_string(),
        "Logs" => tr("Logs", "Nhật ký (Logs)").to_string(),
        "Containers" => tr("Sandbox Container", "Vùng chứa (Container)").to_string(),
        "LaunchAgents" => tr("LaunchAgent", "Tác vụ chạy ngầm (LaunchAgent)").to_string(),
        "Running Process" => tr("Running Process", "Đang chạy ngầm").to_string(),
        "LaunchAgent" => tr("Orphaned LaunchAgent", "LaunchAgent rác").to_string(),
        other => other.to_string(),
    }
}

pub fn translate_scan_message(msg: &str) -> String {
    if msg.is_empty() {
        return tr("Ready", "Sẵn sàng").to_string();
    }
    if is_vi() {
        return msg.to_string();
    }
    if let Some(folder) = msg.strip_prefix("Đang quét thư mục ~/") {
        let f = folder.trim_end_matches('.').trim();
        return format!("Scanning folder ~/{}...", f);
    }
    if let Some(folder) = msg.strip_prefix("Đã quét xong ~/") {
        return format!("Finished scanning ~/{}", folder);
    }
    if let Some(item) = msg.strip_prefix("Đang quét: ") {
        return format!("Scanning: {}", item);
    }
    if msg.contains("Bước 1/3:") {
        return "Step 1/3: Scanning file sizes across directory tree...".to_string();
    }
    if msg.contains("Bước 2/3:") {
        return "Step 2/3: Comparing 128-bit edge hashes...".to_string();
    }
    if msg.contains("Bước 3/3:") {
        return "Step 3/3: Verifying full byte-for-byte file hashes...".to_string();
    }
    if msg.contains("Đang đối chiếu nội dung tệp") {
        return "Comparing file contents...".to_string();
    }
    if msg.contains("Đang quét hệ thống") {
        return "Scanning filesystem...".to_string();
    }
    if msg.contains("Hoàn tất") {
        return "Scan completed.".to_string();
    }
    msg.to_string()
}

pub fn translate_power_source(src: &str) -> String {
    match src {
        "AC Power" | "Sạc điện (AC)" => tr("AC Power", "Đang cắm sạc (AC)").to_string(),
        "Battery Power" | "Pin" => tr("Battery Power", "Đang dùng Pin").to_string(),
        other => other.to_string(),
    }
}
