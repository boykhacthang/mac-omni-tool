use crate::events::{screenshot_session, AppUserEvent};
use serde::Deserialize;
use serde_json::{json, Value};
use egui_winit::winit::event_loop::EventLoopProxy;

#[derive(Debug, Deserialize)]
pub struct ApiRequest {
    pub cmd: String,
    #[serde(default)]
    pub payload: Value,
}

pub fn handle_api_request(body: &[u8], proxy: &EventLoopProxy<AppUserEvent>) -> Value {
    let Ok(req) = serde_json::from_slice::<ApiRequest>(body) else {
        return json!({ "ok": false, "error": "Invalid JSON request" });
    };
    let p = &req.payload;

    match req.cmd.as_str() {
        // --------------------------------------------------------------------
        // Tab 1: Display Management & System Light/Dark Mode + Audio Devices
        // --------------------------------------------------------------------
        "get_display_state" => {
            let st = core_display::get_display_system_state();
            serde_json::to_value(st).unwrap_or_else(|_| json!({}))
        }
        "set_display_brightness" => {
            let id = p.get("display_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let br = p.get("brightness").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            match core_display::set_display_brightness(id, br) {
                Ok(info) => json!({ "ok": true, "display": info }),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "set_display_color_filter" => {
            let id = p.get("display_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let dim = p
                .get("software_dimming")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            let warm = p.get("warmth").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            match core_display::set_display_color_filter(id, dim, warm) {
                Ok(info) => json!({ "ok": true, "display": info }),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "reset_display_filters" => {
            core_display::reset_all_display_filters();
            json!({ "ok": true })
        }
        "set_dark_mode" => {
            let dark = p.get("dark").and_then(|v| v.as_bool()).unwrap_or(true);
            let now_dark = core_display::set_dark_mode(dark);
            json!({ "ok": true, "dark_mode": now_dark })
        }
        "get_audio_state" => {
            let st = core_display::get_audio_system_state();
            serde_json::to_value(st).unwrap_or_else(|_| json!({}))
        }
        "set_audio_device" => {
            let id = p.get("device_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let is_output = p.get("is_output").and_then(|v| v.as_bool()).unwrap_or(true);
            match core_display::set_audio_device(id, is_output) {
                Ok(st) => serde_json::to_value(st).unwrap_or_else(|_| json!({ "ok": true })),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "set_audio_volume" => {
            let vol = p.get("volume").and_then(|v| v.as_f64()).map(|v| v as f32);
            let muted = p.get("muted").and_then(|v| v.as_bool());
            let st = core_display::set_audio_volume(vol, muted);
            serde_json::to_value(st).unwrap_or_else(|_| json!({ "ok": true }))
        }

        // --------------------------------------------------------------------
        // Tab 2: Hardware Monitoring, Temperatures & Top Processes + Force Quit
        // --------------------------------------------------------------------
        "get_hardware_dashboard" => {
            let sort_by = p.get("sort_by").and_then(|v| v.as_str()).unwrap_or("ram");
            let limit = p.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let data = core_monitor::get_hardware_dashboard(sort_by, limit);
            serde_json::to_value(data).unwrap_or_else(|_| json!({}))
        }
        "get_menubar_stats" => {
            let data = core_monitor::get_menubar_stats();
            serde_json::to_value(data).unwrap_or_else(|_| json!({}))
        }
        "kill_process" => {
            let pid = p.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let force = p.get("force").and_then(|v| v.as_bool()).unwrap_or(true);
            match core_monitor::kill_process(pid, force) {
                Ok(msg) => json!({ "ok": true, "message": msg }),
                Err(err) => json!({ "ok": false, "error": err }),
            }
        }

        // --------------------------------------------------------------------
        // Tab 3: Clipboard Manager (1,000 items + Images + Left-Side Drawer)
        // --------------------------------------------------------------------
        "query_clipboard" => {
            let query = p.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let filter_kind = p
                .get("filter_kind")
                .and_then(|v| v.as_str())
                .unwrap_or("all");
            let page = p.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let page_size = p
                .get("page_size")
                .or_else(|| p.get("limit"))
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;
            let page_res = core_clipboard::query_clipboard_page(query, filter_kind, page, page_size);
            let stats = core_clipboard::get_clipboard_stats();
            json!({
                "ok": true,
                "items": page_res.items,
                "page": page_res.page,
                "page_size": page_res.page_size,
                "total_matching": page_res.total_matching,
                "total_pages": page_res.total_pages,
                "stats": stats
            })
        }
        "paste_clipboard_item" => {
            let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let simulate = p
                .get("simulate_paste")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match core_clipboard::activate_clipboard_item(id, simulate) {
                Ok(()) => json!({ "ok": true }),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "toggle_pin_clipboard" => {
            let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let ok = core_clipboard::toggle_pin_clipboard_item(id);
            json!({ "ok": ok })
        }
        "delete_clipboard_item" => {
            let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let ok = core_clipboard::delete_clipboard_item(id);
            json!({ "ok": ok })
        }
        "clear_clipboard" => {
            let ok = core_clipboard::clear_unpinned_clipboard_items();
            json!({ "ok": ok })
        }
        "set_clipboard_enabled" => {
            let enabled = p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            core_clipboard::set_clipboard_enabled(enabled);
            let mut cfg = core_input::get_input_config();
            cfg.clipboard_enabled = enabled;
            let saved = core_input::update_input_config(cfg);
            if !saved.clipboard_enabled {
                let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
            }
            json!({ "ok": true, "enabled": saved.clipboard_enabled })
        }
        "copy_text" => {
            let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let ok = core_clipboard::copy_text_to_clipboard(text);
            json!({ "ok": ok })
        }

        // --------------------------------------------------------------------
        // Tab 4 & 5: Window Snapping/Tiling & Independent Alt-Tab Live Preview
        // --------------------------------------------------------------------
        "get_window_shortcuts" => {
            let shortcuts = core_window::get_window_shortcuts();
            let permissions = core_window::get_permissions_status();
            json!({ "ok": true, "shortcuts": shortcuts, "permissions": permissions })
        }
        "update_window_shortcut" => {
            let action = p.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let shortcut = p.get("shortcut").and_then(|v| v.as_str()).unwrap_or("");
            let enabled = p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            match core_window::update_window_shortcut(action, shortcut, enabled) {
                Ok(list) => {
                    core_input::sync_window_tiling_carbon_hotkeys();
                    json!({ "ok": true, "shortcuts": list })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "reset_window_shortcuts" => {
            let list = core_window::reset_window_shortcuts();
            core_input::sync_window_tiling_carbon_hotkeys();
            json!({ "ok": true, "shortcuts": list })
        }
        "snap_window" => {
            if !core_window::is_accessibility_trusted() {
                return json!({
                    "ok": false,
                    "missing_permission": "accessibility",
                    "error": "Cần cấp quyền Accessibility (Trợ năng) trong Cài đặt → Hệ thống & Quyền trên App trước khi sử dụng Chia cửa sổ."
                });
            }
            let cfg = core_input::get_input_config();
            if !cfg.window_snapping_enabled {
                return json!({ "ok": false, "error": "Tính năng Chia & Gắn cửa sổ đang tắt" });
            }
            let layout = p
                .get("layout")
                .and_then(|v| v.as_str())
                .unwrap_or("left_half");
            if core_window::get_window_shortcuts()
                .iter()
                .any(|b| b.action == layout && !b.enabled)
            {
                return json!({ "ok": false, "error": "Bố cục cửa sổ này đang tắt" });
            }
            let my_pid = std::process::id() as i32;
            if core_window::frontmost_application_pid() == my_pid {
                let _ = proxy.send_event(AppUserEvent::SnapOwnMainWindow(layout.to_string()));
                json!({ "ok": true, "message": format!("Đã chia cửa sổ ({})", layout) })
            } else {
                match core_window::snap_focused_window(layout) {
                    Ok(msg) => json!({ "ok": true, "message": msg }),
                    Err(_) => {
                        let _ =
                            proxy.send_event(AppUserEvent::SnapOwnMainWindow(layout.to_string()));
                        json!({ "ok": true, "message": format!("Đã chia cửa sổ ({})", layout) })
                    }
                }
            }
        }
        "list_open_windows" => {
            let with_previews = p
                .get("with_previews")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let wins = core_window::list_open_windows(with_previews);
            json!({ "ok": true, "windows": wins })
        }
        "focus_window" => {
            let window_id = p.get("window_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let owner_pid = p.get("owner_pid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let _ = core_window::focus_window(window_id, owner_pid);
            json!({ "ok": true })
        }

        // --------------------------------------------------------------------
        // Tab 6: External Mouse, Smooth Scroll, Reverse Scroll, Per-App Buttons & Input
        // --------------------------------------------------------------------
        "get_input_config" => {
            let cfg = core_input::get_input_config();
            serde_json::to_value(cfg).unwrap_or_else(|_| json!({}))
        }
        "update_input_config" => {
            if let Ok(new_cfg) =
                serde_json::from_value::<core_input::InputAutomationConfig>(p.clone())
            {
                core_clipboard::set_clipboard_enabled(new_cfg.clipboard_enabled);
                let saved = core_input::update_input_config(new_cfg);
                if !saved.clipboard_enabled {
                    let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
                }
                if !saved.replace_macos_cmd_tab {
                    let _ = proxy.send_event(AppUserEvent::HideAltTabOverlay);
                }
                if !saved.screenshot_enabled {
                    let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
                }
                let _ = proxy.send_event(AppUserEvent::SyncMenuBarConfig);
                serde_json::to_value(saved).unwrap_or_else(|_| json!({ "ok": true }))
            } else {
                json!({ "ok": false, "error": "Invalid input config payload" })
            }
        }
        "update_menubar_config" => {
            let mut cfg = core_input::get_input_config();
            if let Some(enabled) = p.get("menubar_enabled").and_then(|v| v.as_bool()) {
                cfg.menubar_enabled = enabled;
                if enabled && cfg.menubar_display_mode == "hidden" {
                    cfg.menubar_display_mode = "temp_cpu".to_string();
                } else if !enabled {
                    cfg.menubar_display_mode = "hidden".to_string();
                }
            }
            if let Some(mode) = p.get("menubar_display_mode").and_then(|v| v.as_str()) {
                cfg.menubar_display_mode = mode.to_string();
                if mode == "hidden" {
                    cfg.menubar_enabled = false;
                } else if p.get("menubar_enabled").is_none() {
                    cfg.menubar_enabled = true;
                }
            }
            if let Some(icon) = p.get("menubar_icon").and_then(|v| v.as_str()) {
                cfg.menubar_icon = icon.to_string();
            }
            if let Some(v) = p.get("menubar_show_temp").and_then(|v| v.as_bool()) {
                cfg.menubar_show_temp = v;
            }
            if let Some(v) = p.get("menubar_show_cpu").and_then(|v| v.as_bool()) {
                cfg.menubar_show_cpu = v;
            }
            if let Some(v) = p.get("menubar_show_ram").and_then(|v| v.as_bool()) {
                cfg.menubar_show_ram = v;
            }
            if let Some(v) = p.get("menubar_show_net").and_then(|v| v.as_bool()) {
                cfg.menubar_show_net = v;
            }
            if let Some(sec) = p.get("menubar_refresh_interval_sec").and_then(|v| v.as_u64()) {
                cfg.menubar_refresh_interval_sec = (sec as u32).clamp(1, 30);
            }
            if let Some(bg) = p.get("keep_running_in_background").and_then(|v| v.as_bool()) {
                cfg.keep_running_in_background = bg;
            }
            if let Some(sc) = p.get("dashboard_shortcut").and_then(|v| v.as_str()) {
                cfg.dashboard_shortcut = sc.to_string();
            }
            let saved = core_input::update_input_config(cfg);
            let _ = proxy.send_event(AppUserEvent::SyncMenuBarConfig);
            json!({ "ok": true, "config": saved })
        }
        "get_mouse_state" => {
            let cfg = core_input::get_input_config();
            let spaces = core_input::get_spaces_status();
            let running_apps = core_input::list_running_gui_apps();
            let permissions = core_window::get_permissions_status();
            json!({
                "ok": true,
                "config": cfg,
                "spaces": spaces,
                "running_apps": running_apps,
                "permissions": permissions
            })
        }
        "list_installed_mouse_apps" => {
            let installed_apps = core_input::list_installed_gui_apps();
            let running_apps = core_input::list_running_gui_apps();
            json!({
                "ok": true,
                "installed_apps": installed_apps,
                "running_apps": running_apps
            })
        }
        "pick_mouse_app_dialog" => {
            if let Some(app) = core_input::pick_application_bundle_dialog() {
                json!({ "ok": true, "app": app })
            } else {
                json!({ "ok": false, "cancelled": true })
            }
        }
        "reset_mouse_profiles" => {
            let mut cfg = core_input::get_input_config();
            cfg.app_mouse_profiles = core_input::default_app_mouse_profiles();
            cfg.mouse_profiles_v2_chrome_only = true;
            cfg.mouse_forward_action = "space_right".to_string();
            cfg.mouse_back_action = "space_left".to_string();
            cfg.mouse_middle_action = "default".to_string();
            cfg.single_space_fallback = "passthrough".to_string();
            let saved = core_input::update_input_config(cfg);
            json!({ "ok": true, "config": saved })
        }
        "toggle_external_mouse" => {
            let mut cfg = core_input::get_input_config();
            let next = p
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(!cfg.external_mouse_enabled);
            if next && !core_window::is_accessibility_trusted() {
                return json!({
                    "ok": false,
                    "missing_permission": "accessibility",
                    "error": "Cần cấp quyền Accessibility (Trợ năng) trong Cài đặt → Hệ thống & Quyền trên App trước khi bật Chuột rời.",
                    "enabled": cfg.external_mouse_enabled,
                    "config": cfg
                });
            }
            cfg.external_mouse_enabled = next;
            let saved = core_input::update_input_config(cfg);
            json!({ "ok": true, "enabled": saved.external_mouse_enabled, "config": saved })
        }
        "toggle_invert_mouse_scroll" => {
            let mut cfg = core_input::get_input_config();
            if !core_window::is_accessibility_trusted() {
                return json!({
                    "ok": false,
                    "missing_permission": "accessibility",
                    "error": "Cần cấp quyền Accessibility (Trợ năng) trong Cài đặt → Hệ thống & Quyền trên App.",
                    "inverted": cfg.invert_external_mouse_scroll,
                    "config": cfg
                });
            }
            if !cfg.external_mouse_enabled {
                return json!({ "ok": false, "error": "Tối ưu Chuột rời đang tắt", "inverted": cfg.invert_external_mouse_scroll, "config": cfg });
            }
            let next = p
                .get("inverted")
                .and_then(|v| v.as_bool())
                .unwrap_or(!cfg.invert_external_mouse_scroll);
            cfg.invert_external_mouse_scroll = next;
            let saved = core_input::update_input_config(cfg);
            json!({ "ok": true, "inverted": saved.invert_external_mouse_scroll, "config": saved })
        }
        "toggle_smooth_scroll" => {
            let mut cfg = core_input::get_input_config();
            if !core_window::is_accessibility_trusted() {
                return json!({
                    "ok": false,
                    "missing_permission": "accessibility",
                    "error": "Cần cấp quyền Accessibility (Trợ năng) trong Cài đặt → Hệ thống & Quyền trên App.",
                    "enabled": cfg.smooth_scroll_enabled,
                    "config": cfg
                });
            }
            if !cfg.external_mouse_enabled {
                return json!({ "ok": false, "error": "Tối ưu Chuột rời đang tắt", "enabled": cfg.smooth_scroll_enabled, "config": cfg });
            }
            let next = p
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(!cfg.smooth_scroll_enabled);
            cfg.smooth_scroll_enabled = next;
            let saved = core_input::update_input_config(cfg);
            json!({ "ok": true, "enabled": saved.smooth_scroll_enabled, "config": saved })
        }
        "trigger_system_action" => {
            let act = p
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("mission_control");
            core_input::trigger_system_action(act);
            json!({ "ok": true })
        }

        // --------------------------------------------------------------------
        // Tab 7: Junk Cleaner, App Uninstaller & Large Files
        // --------------------------------------------------------------------
        "scan_system_junk" => {
            let force = p.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            let res = core_cleaner::scan_system_junk_cached(force);
            serde_json::to_value(res).unwrap_or_else(|_| json!({}))
        }
        "scan_installed_apps" => {
            let force = p.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            let apps = core_cleaner::scan_installed_applications_cached(force);
            let orphans = core_cleaner::scan_orphaned_background_items();
            json!({ "ok": true, "apps": apps, "orphans": orphans })
        }
        "uninstall_app" => {
            let app_path = p.get("app_path").and_then(|v| v.as_str()).unwrap_or("");
            let app_name = p.get("app_name").and_then(|v| v.as_str()).unwrap_or("App");
            let bundle_id = p.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
            let paths: Vec<String> = p
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let also_empty_trash = p
                .get("empty_trash")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match core_cleaner::uninstall_application(
                app_path,
                app_name,
                bundle_id,
                &paths,
                also_empty_trash,
            ) {
                Ok(msg) => json!({ "ok": true, "message": msg }),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "clean_orphaned_background" => match core_cleaner::clean_orphaned_background_items() {
            Ok(msg) => json!({ "ok": true, "message": msg }),
            Err(e) => json!({ "ok": false, "error": e }),
        },
        "scan_large_files" => {
            let scope = p.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
            let min_mb = p.get("min_size_mb").and_then(|v| v.as_u64()).unwrap_or(50);
            let files = core_cleaner::scan_large_files(scope, min_mb);
            json!({ "ok": true, "files": files })
        }
        "scan_duplicate_files" => {
            let scope = p.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
            let min_kb = p.get("min_size_kb").and_then(|v| v.as_u64()).unwrap_or(100);
            let res = core_cleaner::scan_duplicate_files(scope, min_kb);
            serde_json::to_value(res).unwrap_or_else(|_| json!({}))
        }
        "get_duplicate_excludes" => {
            let list = core_cleaner::get_duplicate_exclude_list();
            json!({ "ok": true, "excludes": list })
        }
        "add_duplicate_exclude" => {
            let pattern = p.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let list = core_cleaner::add_duplicate_exclude(pattern);
            core_cleaner::invalidate_cleaner_cache();
            json!({ "ok": true, "excludes": list })
        }
        "remove_duplicate_exclude" => {
            let pattern = p.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let list = core_cleaner::remove_duplicate_exclude(pattern);
            core_cleaner::invalidate_cleaner_cache();
            json!({ "ok": true, "excludes": list })
        }
        "reset_duplicate_excludes" => {
            let list = core_cleaner::reset_duplicate_excludes();
            core_cleaner::invalidate_cleaner_cache();
            json!({ "ok": true, "excludes": list })
        }
        "get_trash_status" => {
            let st = core_cleaner::get_trash_status();
            serde_json::to_value(st).unwrap_or_else(|_| json!({}))
        }
        "empty_trash" => match core_cleaner::empty_macos_trash() {
            Ok(msg) => json!({ "ok": true, "message": msg }),
            Err(e) => json!({ "ok": false, "error": e }),
        },
        "start_cleaner_scan" => {
            let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("junk");
            let scope = p.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
            let min_val = if kind == "duplicates" {
                p.get("min_size_kb").and_then(|v| v.as_u64()).unwrap_or(100)
            } else {
                p.get("min_size_mb").and_then(|v| v.as_u64()).unwrap_or(50)
            };
            let force = p.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            let st = core_cleaner::start_cleaner_scan_async(kind, scope, min_val, force);
            serde_json::to_value(st).unwrap_or_else(|_| json!({}))
        }
        "get_cleaner_progress" => {
            let st = core_cleaner::get_cleaner_scan_status();
            serde_json::to_value(st).unwrap_or_else(|_| json!({}))
        }
        "clean_selected_paths" => {
            let paths: Vec<String> = p
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let also_empty_trash = p
                .get("empty_trash")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match core_cleaner::clean_selected_paths_ext(&paths, also_empty_trash) {
                Ok(msg) => json!({ "ok": true, "message": msg }),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "reveal_in_finder" => {
            if let Some(path) = p.get("path").and_then(|v| v.as_str()) {
                let _ = core_cleaner::reveal_in_finder(path);
            }
            json!({ "ok": true })
        }

        // --------------------------------------------------------------------
        // Tab 8: Launch at Login & System Permissions
        // --------------------------------------------------------------------
        "get_autostart_status" => {
            let st = core_cleaner::get_autostart_status();
            serde_json::to_value(st).unwrap_or_else(|_| json!({}))
        }
        "set_autostart_config" => {
            let enabled = p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let min = p
                .get("start_minimized")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match core_cleaner::set_autostart_config(enabled, min) {
                Ok(st) => serde_json::to_value(st).unwrap_or_else(|_| json!({ "ok": true })),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "get_permissions_status" => {
            core_input::notify_permissions_changed();
            let perms = core_window::get_permissions_status();
            serde_json::to_value(perms).unwrap_or_else(|_| json!({}))
        }
        "open_permission" => {
            let kind = p
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("accessibility");
            core_window::request_or_open_permission(kind);
            core_input::notify_permissions_changed();
            json!({ "ok": true })
        }
        "reset_permissions" => {
            core_window::reset_and_request_permissions();
            core_input::notify_permissions_changed();
            json!({ "ok": true })
        }

        // --------------------------------------------------------------------
        // Menu Bar Status Item, Popover, Launcher & Multi-Monitor Overlays
        // --------------------------------------------------------------------
        "update_menubar_title" => {
            if let Some(title) = p.get("title").and_then(|v| v.as_str()) {
                let _ = proxy.send_event(AppUserEvent::UpdateMenuBarTitle(title.to_string()));
            }
            json!({ "ok": true })
        }
        "set_app_theme" => {
            if let Some(theme) = p.get("theme").and_then(|v| v.as_str()) {
                let _ = proxy.send_event(AppUserEvent::SetAppTheme(theme.to_string()));
            }
            json!({ "ok": true })
        }
        "set_app_lang" => {
            if let Some(lang) = p.get("lang").and_then(|v| v.as_str()) {
                let _ = proxy.send_event(AppUserEvent::SetAppLang(lang.to_string()));
            }
            json!({ "ok": true })
        }
        "open_external_url" => {
            if let Some(url) = p.get("url").and_then(|v| v.as_str()) {
                if url.starts_with("https://") || url.starts_with("http://") {
                    let _ = std::process::Command::new("open").arg(url).spawn();
                    return json!({ "ok": true });
                }
            }
            json!({ "ok": false, "error": "Invalid URL" })
        }
        "check_for_updates" => {
            let res = crate::updater::check_for_updates();
            serde_json::to_value(res).unwrap_or_else(|_| json!({ "ok": false }))
        }
        "perform_self_update" => {
            let zip_url = p.get("zip_url").and_then(|v| v.as_str()).unwrap_or("");
            let sha256_zip = p.get("sha256_zip").and_then(|v| v.as_str()).unwrap_or("");
            match crate::updater::download_and_install_update(zip_url, sha256_zip) {
                Ok(target_path) => {
                    let quit_proxy = proxy.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(250));
                        let _ = quit_proxy.send_event(AppUserEvent::QuitApp);
                    });
                    json!({
                        "ok": true,
                        "target_path": target_path
                    })
                }
                Err(err) => json!({
                    "ok": false,
                    "error": err
                }),
            }
        }
        "set_mini_expanded" => {
            let expanded = p.get("expanded").and_then(|v| v.as_bool()).unwrap_or(false);
            let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(expanded));
            json!({ "ok": true })
        }
        "set_mini_height" => {
            if let Some(h) = p.get("height").and_then(|v| v.as_f64()) {
                let _ = proxy.send_event(AppUserEvent::SetMiniHeight(h));
            }
            json!({ "ok": true })
        }
        "show_launcher" => {
            let _ = proxy.send_event(AppUserEvent::ShowLauncher);
            json!({ "ok": true })
        }
        "hide_launcher" => {
            let _ = proxy.send_event(AppUserEvent::HideLauncher);
            json!({ "ok": true })
        }
        "quit_app" => {
            let _ = proxy.send_event(AppUserEvent::QuitApp);
            json!({ "ok": true })
        }
        "toggle_overlay" => {
            let cfg = core_input::get_input_config();
            let w = p
                .get("window")
                .and_then(|v| v.as_str())
                .unwrap_or("clipboard");
            if w == "alttab" {
                if !core_window::is_accessibility_trusted() {
                    return json!({
                        "ok": false,
                        "missing_permission": "accessibility",
                        "error": "Cần cấp quyền Accessibility (Trợ năng) trong Cài đặt → Hệ thống & Quyền trên App trước khi sử dụng Alt-Tab."
                    });
                }
                if !cfg.replace_macos_cmd_tab {
                    return json!({ "ok": false, "error": "Alt-Tab đang tắt" });
                }
                let _ = proxy.send_event(AppUserEvent::ToggleAltTabOverlay);
            } else if w == "screenshot" {
                if !core_window::is_screen_recording_granted() {
                    return json!({
                        "ok": false,
                        "missing_permission": "screen_recording",
                        "error": "Cần cấp quyền Ghi màn hình (Screen Recording) trong Cài đặt → Hệ thống & Quyền trên App trước khi chụp màn hình."
                    });
                }
                if !cfg.screenshot_enabled || !cfg.screenshot_region_enabled {
                    return json!({ "ok": false, "error": "Chụp vùng chọn đang tắt" });
                }
                let _ = proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
            } else {
                if !cfg.clipboard_enabled {
                    return json!({ "ok": false, "error": "Lịch sử Clipboard đang tắt" });
                }
                let _ = proxy.send_event(AppUserEvent::ToggleClipboardDrawer);
            }
            json!({ "ok": true })
        }
        "hide_overlay" => {
            let w = p
                .get("window")
                .and_then(|v| v.as_str())
                .unwrap_or("clipboard");
            if w == "alttab" {
                let _ = proxy.send_event(AppUserEvent::HideAltTabOverlay);
            } else if w == "screenshot" {
                let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
            } else {
                let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
            }
            json!({ "ok": true })
        }

        // --------------------------------------------------------------------
        // Region Screenshot (Dual-Monitor), Fullscreen & Annotation
        // --------------------------------------------------------------------
        "trigger_screenshot" => {
            if !core_window::is_screen_recording_granted() {
                return json!({
                    "ok": false,
                    "missing_permission": "screen_recording",
                    "error": "Cần cấp quyền Ghi màn hình (Screen Recording) trong Cài đặt → Hệ thống & Quyền trên App trước khi chụp màn hình."
                });
            }
            let cfg = core_input::get_input_config();
            if !cfg.screenshot_enabled || !cfg.screenshot_region_enabled {
                return json!({ "ok": false, "error": "Chụp vùng chọn đang tắt" });
            }
            let _ = proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
            json!({ "ok": true })
        }
        "trigger_fullscreen_screenshot" => {
            if !core_window::is_screen_recording_granted() {
                return json!({
                    "ok": false,
                    "missing_permission": "screen_recording",
                    "error": "Cần cấp quyền Ghi màn hình (Screen Recording) trong Cài đặt → Hệ thống & Quyền trên App trước khi chụp màn hình."
                });
            }
            let cfg = core_input::get_input_config();
            let display_id = p.get("display_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let instant_copy = p
                .get("instant_copy")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !cfg.screenshot_enabled
                || (instant_copy && !cfg.screenshot_fullscreen_copy_enabled)
                || (!instant_copy && !cfg.screenshot_fullscreen_edit_enabled)
            {
                return json!({ "ok": false, "error": "Chế độ chụp màn hình này đang tắt" });
            }
            let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                display_id,
                instant_copy,
            });
            json!({ "ok": true })
        }
        "set_active_screenshot_slot" => {
            let slot = p.get("slot").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let _ = proxy.send_event(AppUserEvent::SetActiveScreenshotSlot(slot));
            json!({ "ok": true })
        }
        "reveal_screenshot_slot" => {
            let slot = p.get("slot").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let seq = p.get("seq").and_then(|v| v.as_u64()).unwrap_or(0);
            let _ = proxy.send_event(AppUserEvent::RevealScreenshotSlot { slot, seq });
            json!({ "ok": true })
        }
        "get_screenshot_meta" => {
            let slot = p.get("slot").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let sess = screenshot_session().lock().unwrap();
            if let Some(s) = sess.slots.get(slot) {
                json!({
                    "ok": !s.png_bytes.is_empty(),
                    "width": s.width,
                    "height": s.height,
                    "display_id": s.display_id,
                    "display_name": s.display_name,
                    "preselect_fullscreen": s.preselect_fullscreen,
                    "seq": sess.capture_seq,
                })
            } else {
                json!({ "ok": false })
            }
        }
        "copy_screenshot_png" => {
            let raw_slot = p.get("raw_slot").and_then(|v| v.as_u64()).map(|v| v as usize);
            let data_url = p.get("data_url").and_then(|v| v.as_str()).unwrap_or("");
            let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
            let bytes_res = if let Some(slot_idx) = raw_slot {
                let sess = screenshot_session().lock().unwrap();
                sess.slots
                    .get(slot_idx)
                    .filter(|s| !s.png_bytes.is_empty())
                    .map(|s| s.png_bytes.clone())
                    .ok_or_else(|| "Empty screenshot slot".to_string())
            } else {
                core_clipboard::decode_base64_png(data_url)
            };
            match bytes_res.and_then(|bytes| core_clipboard::copy_png_bytes_to_clipboard(&bytes)) {
                Ok(()) => json!({ "ok": true }),
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "save_screenshot_png" => {
            let raw_slot = p.get("raw_slot").and_then(|v| v.as_u64()).map(|v| v as usize);
            let data_url = p.get("data_url").and_then(|v| v.as_str()).unwrap_or("");
            let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
            let bytes_res = if let Some(slot_idx) = raw_slot {
                let sess = screenshot_session().lock().unwrap();
                sess.slots
                    .get(slot_idx)
                    .filter(|s| !s.png_bytes.is_empty())
                    .map(|s| s.png_bytes.clone())
                    .ok_or_else(|| "Empty screenshot slot".to_string())
            } else {
                core_clipboard::decode_base64_png(data_url)
            };
            match bytes_res {
                Ok(bytes) => {
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(75));
                        let cfg = core_input::get_input_config();
                        let _ = core_clipboard::save_png_bytes_to_file(
                            &bytes,
                            Some(&cfg.screenshot_save_directory),
                            cfg.screenshot_ask_where_to_save,
                        );
                    });
                    json!({ "ok": true, "saved": true })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }

        _ => json!({ "ok": false, "error": format!("Unknown command: {}", req.cmd) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_api_request_with_and_without_payload() {
        let req1: ApiRequest =
            serde_json::from_slice(br#"{"cmd":"get_menubar_stats"}"#).expect("valid json");
        assert_eq!(req1.cmd, "get_menubar_stats");
        assert!(req1.payload.is_null());

        let req2: ApiRequest = serde_json::from_slice(
            br#"{"cmd":"get_hardware_dashboard","payload":{"sort_by":"cpu","limit":15}}"#,
        )
        .expect("valid json");
        assert_eq!(req2.cmd, "get_hardware_dashboard");
        assert_eq!(req2.payload["sort_by"], "cpu");
        assert_eq!(req2.payload["limit"], 15);
    }
}
