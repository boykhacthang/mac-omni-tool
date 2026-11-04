# 📡 Tài liệu Đặc tả JSON-RPC API (`docs/API_REFERENCE.md`)

Tất cả các cửa sổ WebView giao tiếp với tầng Rust thông qua giao thức tùy chỉnh `omni://localhost/api` bằng phương thức HTTP `POST`:

```javascript
const res = await fetch("omni://localhost/api", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ cmd: "<tên_lệnh>", payload: { ... } })
});
const data = await res.json();
```

Bộ xử lý trung tâm nằm tại [`src/rpc.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/src/rpc.rs). Dưới đây là danh mục đầy đủ các lệnh RPC theo từng nhóm chức năng.

---

## 1. Màn hình & Âm thanh (`core-display`)

| Lệnh (`cmd`) | Payload đầu vào | Kết quả trả về (JSON) |
| :--- | :--- | :--- |
| `get_display_state` | `{}` | `{ displays: DisplayInfo[], dark_mode: bool }` |
| `set_display_brightness` | `{ display_id: u32, brightness: f32 }` *(0.05..1.0)* | `{ ok: bool, display?: DisplayInfo, error?: string }` |
| `set_display_color_filter` | `{ display_id: u32, software_dimming: f32, warmth: f32 }` | `{ ok: bool, display?: DisplayInfo, error?: string }` |
| `reset_display_filters` | `{}` | `{ ok: true }` |
| `set_dark_mode` | `{ dark: bool }` | `{ ok: true, dark_mode: bool }` |
| `get_audio_state` | `{}` | `{ volume: f32, muted: bool, current_output_id: u32, current_output_name: string, output_devices: AudioDeviceInfo[], input_devices: AudioDeviceInfo[] }` |
| `set_audio_device` | `{ device_id: u32, is_output: bool }` | `AudioSystemState` hoặc `{ ok: false, error: string }` |
| `set_audio_volume` | `{ volume?: f32, muted?: bool }` | `AudioSystemState` |

---

## 2. Giám sát Phần cứng & Tiến trình (`core-monitor`)

| Lệnh (`cmd`) | Payload đầu vào | Kết quả trả về (JSON) |
| :--- | :--- | :--- |
| `get_hardware_dashboard` | `{ sort_by: "ram" \| "cpu" \| "disk" \| "name", limit: usize }` | `HardwareDashboardData` (CPU, Per-Core, GPU, Unified VRAM, RAM, Disk, Network I/O, Temperatures, Battery, Top Processes) |
| `get_menubar_stats` | `{}` | `MenuBarQuickStats { cpu_usage_percent, memory_usage_percent, used_memory_bytes, total_memory_bytes, cpu_temp_c }` *(Siêu nhẹ, không quét bảng tiến trình)* |
| `kill_process` | `{ pid: u32, force: bool }` | `{ ok: bool, message?: string, error?: string }` |

---

## 3. Quản lý Clipboard (`core-clipboard`)

| Lệnh (`cmd`) | Payload đầu vào | Kết quả trả về (JSON) |
| :--- | :--- | :--- |
| `query_clipboard` | `{ query: string, filter_kind: "all" \| "text" \| "image" \| "pinned", page: usize, page_size: usize }` | `{ ok: true, items: ClipboardEntry[], page: usize, page_size: usize, total_matching: usize, total_pages: usize, stats: ClipboardStats }` |
| `paste_clipboard_item` | `{ id: i64, simulate_paste: bool }` | `{ ok: bool, error?: string }` |
| `toggle_pin_clipboard` | `{ id: i64 }` | `{ ok: bool }` |
| `delete_clipboard_item` | `{ id: i64 }` | `{ ok: bool }` |
| `clear_clipboard` | `{}` | `{ ok: bool }` *(Xóa tất cả mục chưa ghim)* |
| `set_clipboard_enabled` | `{ enabled: bool }` | `{ ok: true, enabled: bool }` |
| `copy_text` | `{ text: string }` | `{ ok: bool }` |

---

## 4. Chia Cửa sổ & Chuyển đổi Cửa sổ Alt-Tab (`core-window`)

| Lệnh (`cmd`) | Payload đầu vào | Kết quả trả về (JSON) |
| :--- | :--- | :--- |
| `get_window_shortcuts` | `{}` | `{ ok: true, shortcuts: WindowShortcutBinding[] }` |
| `update_window_shortcut` | `{ action: string, shortcut: string, enabled: bool }` | `{ ok: bool, shortcuts?: WindowShortcutBinding[], error?: string }` *(Tự động đồng bộ lại Carbon HotKeys)* |
| `reset_window_shortcuts` | `{}` | `{ ok: true, shortcuts: WindowShortcutBinding[] }` *(Tự động đồng bộ lại Carbon HotKeys)* |
| `snap_window` | `{ layout: string }` *(vd: `"left_half"`, `"center"`, `"maximize"`, ...)* | `{ ok: bool, message?: string, error?: string }` |
| `list_open_windows` | `{ with_previews: bool }` | `{ ok: true, windows: OpenWindowEntry[] }` |
| `focus_window` | `{ window_id: u32, owner_pid: i32 }` | `{ ok: true }` |

---

## 5. Dọn dẹp Hệ thống, Thùng rác & Tệp trùng lặp (`core-cleaner`)

| Lệnh (`cmd`) | Payload đầu vào | Kết quả trả về (JSON) |
| :--- | :--- | :--- |
| `scan_system_junk` | `{ force: bool }` | `JunkScanSummary { items, total_size_bytes, total_size_mb, trash: TrashStatus }` |
| `scan_installed_apps` | `{ force: bool }` | `{ ok: true, apps: InstalledAppInfo[] }` |
| `scan_large_files` | `{ scope: string, min_size_mb: u64 }` | `{ ok: true, files: LargeFileEntry[] }` |
| `scan_duplicate_files` | `{ scope: string, min_size_kb: u64 }` | `DuplicateScanResult` |
| `start_cleaner_scan` | `{ kind: "junk" \| "uninstall" \| "large" \| "duplicates", scope: string, min_size_mb?: u64, min_size_kb?: u64, force: bool }` | `CleanerScanStatus` |
| `get_cleaner_progress` | `{}` | `CleanerScanStatus { running, kind, percent, message, ... }` |
| `get_duplicate_excludes` | `{}` | `{ ok: true, excludes: string[] }` |
| `add_duplicate_exclude` | `{ pattern: string }` | `{ ok: true, excludes: string[] }` |
| `remove_duplicate_exclude` | `{ pattern: string }` | `{ ok: true, excludes: string[] }` |
| `reset_duplicate_excludes` | `{}` | `{ ok: true, excludes: string[] }` |
| `get_trash_status` | `{}` | `TrashStatus` |
| `empty_trash` | `{}` | `{ ok: bool, message?: string, error?: string }` |
| `clean_selected_paths` | `{ paths: string[], empty_trash?: bool }` | `{ ok: bool, message?: string, error?: string }` |
| `reveal_in_finder` | `{ path: string }` | `{ ok: true }` |

---

## 6. Tự động hóa Đầu vào, Chụp màn hình & Cài đặt Hệ thống (`core-input` & App Shell)

| Lệnh (`cmd`) | Payload đầu vào | Kết quả trả về (JSON) |
| :--- | :--- | :--- |
| `get_input_config` | `{}` | `InputAutomationConfig` |
| `update_input_config` | `InputAutomationConfig` | `InputAutomationConfig` |
| `get_mouse_state` | `{}` | `{ ok: true, config: InputAutomationConfig, spaces: SpacesStatus, running_apps: RunningAppItem[] }` |
| `reset_mouse_profiles` | `{}` | `{ ok: true, config: InputAutomationConfig }` |
| `toggle_external_mouse` | `{ enabled?: bool }` | `{ ok: true, enabled: bool, config: InputAutomationConfig }` |
| `toggle_invert_mouse_scroll` | `{ inverted?: bool }` | `{ ok: true, inverted: bool, config: InputAutomationConfig }` |
| `toggle_smooth_scroll` | `{ enabled?: bool }` | `{ ok: true, enabled: bool, config: InputAutomationConfig }` |
| `trigger_system_action` | `{ action: "mission_control" \| "app_expose" \| "show_desktop" \| "launchpad" \| "space_left" \| "space_right" \| ... }` | `{ ok: true }` |
| `get_autostart_status` | `{}` | `AutostartStatus { enabled, start_minimized, plist_path, executable_path }` |
| `set_autostart_config` | `{ enabled: bool, start_minimized: bool }` | `AutostartStatus` |
| `get_permissions_status` | `{}` | `PermissionsStatus { accessibility_granted, screen_recording_granted, full_disk_access_granted }` |
| `open_permission` | `{ kind: "accessibility" \| "screen_recording" \| "full_disk" }` | `{ ok: true }` |
| `reset_permissions` | `{}` | `{ ok: true }` |
| `trigger_screenshot` | `{}` | `{ ok: true }` *(Mở lớp phủ chụp màn hình chọn vùng trên tất cả màn hình)* |
| `trigger_fullscreen_screenshot` | `{ display_id: u32, instant_copy: bool }` | `{ ok: true }` *(Chế độ 2 hoặc Chế độ 3 theo vị trí con trỏ chuột)* |
| `set_active_screenshot_slot` | `{ slot: usize }` | `{ ok: true }` |
| `get_screenshot_meta` | `{ slot: usize }` | `{ ok: bool, width, height, display_id, display_name, preselect_fullscreen, seq }` |
| `copy_screenshot_png` | `{ data_url: string }` | `{ ok: bool, error?: string }` |
| `save_screenshot_png` | `{ data_url: string }` | `{ ok: bool, saved?: true, error?: string }` |
| `update_menubar_title` | `{ title: string }` | `{ ok: true }` |
| `set_app_theme` | `{ theme: "dark" \| "light" }` | `{ ok: true }` *(Đồng bộ giao diện Sáng/Tối trên mọi cửa sổ)* |
| `set_app_lang` | `{ lang: "en" \| "vi" }` | `{ ok: true }` *(Đồng bộ ngôn ngữ trên toàn bộ 5 loại cửa sổ)* |
| `set_mini_expanded` | `{ expanded: bool }` | `{ ok: true }` |
| `show_launcher` | `{}` | `{ ok: true }` |
| `hide_launcher` | `{}` | `{ ok: true }` |
| `toggle_overlay` | `{ window: "clipboard" \| "alttab" \| "screenshot" }` | `{ ok: true }` |
| `hide_overlay` | `{ window: "clipboard" \| "alttab" \| "screenshot" }` | `{ ok: true }` |
| `quit_app` | `{}` | `{ ok: true }` |

