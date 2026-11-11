use crate::events::AppUserEvent;
use crate::gpu_ui::{
    badge_pill, card_frame, danger_button, draw_window_layout_icon, format_bytes,
    nav_sidebar_button, palette, pill_tab_button, pill_tabs_frame, primary_button,
    progress_bar_styled, render_floating_toast, sanitize_ui_text, secondary_button,
    setting_row_split, shortcut_recorder_button, sub_card_frame, thin_divider, tip_badge,
    toggle_switch, truncated_name_label, NavIconKind,
};
use crate::i18n::{
    is_light_theme, is_vi, set_lang, set_theme, tr, translate_audio_kind, translate_power_source,
    translate_window_action, translate_window_category,
};
use crate::macos_window::{
    decode_data_url_or_path_to_rgba8, decode_image_bytes_to_rgba8_max_size, APP_ICON_PNG,
};
use crate::updater::{UpdateCheckResult, CURRENT_VERSION};
use egui::{
    Color32, ColorImage, ComboBox, FontId, Key, Margin, RichText, Rounding, ScrollArea, Sense,
    Stroke, TextureHandle, TextureOptions,
};
use egui_winit::winit::event_loop::EventLoopProxy;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DashboardTab {
    Displays,
    Hardware,
    Clipboard,
    Tiling,
    Mouse,
    Cleaner,
    Settings,
}

impl DashboardTab {
    pub fn from_name(name: &str) -> Self {
        match name {
            "displays" | "display" => Self::Displays,
            "hardware" | "monitor" => Self::Hardware,
            "clipboard" => Self::Clipboard,
            "tiling" | "window" | "windows" => Self::Tiling,
            "mouse" | "input" => Self::Mouse,
            "cleaner" => Self::Cleaner,
            "settings" => Self::Settings,
            _ => Self::Displays,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DisplaySubTab {
    Displays,
    Audio,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HardwareSubTab {
    Overview,
    Processes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseSubTab {
    Buttons,
    SmoothScroll,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CleanerSubTab {
    Junk,
    Duplicates,
    Apps,
    LargeFiles,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsSubTab {
    Shortcuts,
    System,
}

#[derive(Clone)]
pub enum ShortcutRecordTarget {
    WindowTiling(String),
    ConfigField(String),
}

#[derive(Clone, Debug)]
pub enum CleanerConfirmAction {
    EmptyTrash {
        item_count: usize,
        total_mb: f64,
    },
    CleanJunk {
        paths: Vec<String>,
        total_mb: f64,
        also_empty_trash: bool,
    },
    DeleteDuplicates {
        paths: Vec<String>,
        total_mb: f64,
        also_empty_trash: bool,
    },
    DeleteLargeFiles {
        paths: Vec<String>,
        total_mb: f64,
        also_empty_trash: bool,
    },
    UninstallApp {
        app: core_cleaner::InstalledAppInfo,
        also_empty_trash: bool,
    },
    CleanOrphans {
        count: usize,
    },
}

#[allow(dead_code)]
pub struct DashboardState {
    pub active_tab: DashboardTab,
    pub app_icon_tex: Option<TextureHandle>,
    pub toast: Option<(String, Instant)>,
    pub show_about_modal: bool,

    // Tab 1: Displays & Audio
    pub display_subtab: DisplaySubTab,
    pub display_state: core_display::DisplaySystemState,
    pub audio_state: core_display::AudioSystemState,
    pub last_display_refresh: Instant,
    pub last_audio_refresh: Instant,

    // Tab 2: Hardware & Processes
    pub hw_subtab: HardwareSubTab,
    pub hw_data: Option<core_monitor::HardwareDashboardData>,
    pub hw_sort_by: String,
    pub hw_limit: usize,
    pub hw_process_filter: String,
    pub hw_show_all_sensors: bool,
    pub hw_auto_refresh: bool,
    pub hw_interval_sec: u64,
    pub last_hw_refresh: Instant,

    // Tab 3: Clipboard
    pub clip_query: String,
    pub clip_filter: String,
    pub clip_page: usize,
    pub clip_page_size: usize,
    pub clip_result: core_clipboard::ClipboardPageResult,
    pub clip_stats: core_clipboard::ClipboardStats,
    pub clip_textures: HashMap<i64, TextureHandle>,
    pub last_clip_refresh: Instant,

    // Tab 4: Window Tiling
    pub tiling_category: String,
    pub window_shortcuts: Vec<core_window::WindowShortcutBinding>,

    // Tab 5: Mouse & Shortcuts
    pub mouse_subtab: MouseSubTab,
    pub mouse_selected_profile: String, // "global" or bundle_id
    pub mouse_selected_btn: String,     // "forward", "back", "middle", "chord"
    pub show_app_picker_modal: bool,
    pub installed_gui_apps: Vec<core_input::RunningAppItem>,
    pub app_picker_filter: String,
    pub input_cfg: core_input::InputAutomationConfig,
    pub permissions: core_window::PermissionStatus,
    pub running_apps: Vec<core_input::RunningAppItem>,
    pub spaces_status: core_input::SpacesStatus,
    pub last_spaces_refresh: Instant,
    pub recording_shortcut: Option<ShortcutRecordTarget>,
    pub selected_running_app_idx: usize,

    // Tab 6: System Cleaner
    pub cleaner_tab: CleanerSubTab,
    pub cleaner_status: core_cleaner::CleanerScanStatus,
    pub selected_junk_paths: HashSet<String>,
    pub selected_large_paths: HashSet<String>,
    pub selected_dup_paths: HashSet<String>,
    pub cleaner_also_empty_trash: bool,
    pub cleaner_show_trash_details: bool,
    pub cleaner_show_dup_excludes: bool,
    pub cleaner_expanded_app_bundle: Option<String>,
    pub app_icon_textures: HashMap<String, Option<TextureHandle>>,
    pub app_search_filter: String,
    pub orphans: Vec<core_cleaner::OrphanedBackgroundItem>,
    pub orphans_scanned: bool,
    pub large_scope: String,
    pub large_min_mb: u64,
    pub dup_scope: String,
    pub dup_min_kb: u64,
    pub dup_excludes: Vec<String>,
    pub new_exclude_pattern: String,
    pub last_cleaner_poll: Instant,
    pub cleaner_confirm_action: Option<CleanerConfirmAction>,

    // Tab 7: Settings & Updater
    pub settings_subtab: SettingsSubTab,
    pub autostart: core_cleaner::AutostartConfig,
    pub update_result: Arc<Mutex<Option<UpdateCheckResult>>>,
    pub update_checking: Arc<Mutex<bool>>,
    pub update_installing: Arc<Mutex<bool>>,
    pub confirm_install_update: bool,
}

impl DashboardState {
    pub fn new() -> Self {
        let input_cfg = core_input::get_input_config();
        let permissions = core_window::get_permissions_status();
        let autostart = core_cleaner::get_autostart_status();
        let dup_excludes = core_cleaner::get_duplicate_exclude_list();
        let now_past = Instant::now() - Duration::from_secs(60);

        let mut this = Self {
            active_tab: DashboardTab::Displays,
            app_icon_tex: None,
            toast: None,
            show_about_modal: false,
            display_subtab: DisplaySubTab::Displays,
            display_state: core_display::get_display_system_state(),
            audio_state: core_display::get_audio_system_state(),
            last_display_refresh: Instant::now(),
            last_audio_refresh: Instant::now(),
            hw_subtab: HardwareSubTab::Overview,
            hw_data: None,
            hw_sort_by: "ram".to_string(),
            hw_limit: 20,
            hw_process_filter: String::new(),
            hw_show_all_sensors: false,
            hw_auto_refresh: true,
            hw_interval_sec: 3,
            last_hw_refresh: now_past,
            clip_query: String::new(),
            clip_filter: "all".to_string(),
            clip_page: 1,
            clip_page_size: 25,
            clip_result: core_clipboard::ClipboardPageResult {
                items: Vec::new(),
                page: 1,
                page_size: 25,
                total_matching: 0,
                total_pages: 1,
            },
            clip_stats: core_clipboard::get_clipboard_stats(),
            clip_textures: HashMap::new(),
            last_clip_refresh: now_past,
            tiling_category: "all".to_string(),
            window_shortcuts: core_window::get_window_shortcuts(),
            mouse_subtab: MouseSubTab::Buttons,
            mouse_selected_profile: "global".to_string(),
            mouse_selected_btn: "forward".to_string(),
            show_app_picker_modal: false,
            installed_gui_apps: Vec::new(),
            app_picker_filter: String::new(),
            input_cfg,
            permissions,
            running_apps: Vec::new(),
            spaces_status: core_input::get_spaces_status(),
            last_spaces_refresh: now_past,
            recording_shortcut: None,
            selected_running_app_idx: 0,
            cleaner_tab: CleanerSubTab::Junk,
            cleaner_status: core_cleaner::CleanerScanStatus::default(),
            selected_junk_paths: HashSet::new(),
            selected_large_paths: HashSet::new(),
            selected_dup_paths: HashSet::new(),
            cleaner_also_empty_trash: true,
            cleaner_show_trash_details: false,
            cleaner_show_dup_excludes: false,
            cleaner_expanded_app_bundle: None,
            app_icon_textures: HashMap::new(),
            app_search_filter: String::new(),
            orphans: Vec::new(),
            orphans_scanned: false,
            large_scope: "all".to_string(),
            large_min_mb: 50,
            dup_scope: "all".to_string(),
            dup_min_kb: 100,
            dup_excludes,
            new_exclude_pattern: String::new(),
            last_cleaner_poll: now_past,
            cleaner_confirm_action: None,
            settings_subtab: SettingsSubTab::Shortcuts,
            autostart,
            update_result: Arc::new(Mutex::new(None)),
            update_checking: Arc::new(Mutex::new(false)),
            update_installing: Arc::new(Mutex::new(false)),
            confirm_install_update: false,
        };

        if this.input_cfg.auto_check_updates {
            this.trigger_update_check_async(false);
        }

        this
    }

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((sanitize_ui_text(&msg.into()), Instant::now()));
    }

    pub fn switch_tab(&mut self, tab: DashboardTab) {
        if self.active_tab != tab {
            if self.active_tab == DashboardTab::Hardware {
                self.hw_data = None;
                core_monitor::release_dashboard_memory();
            }
            if self.active_tab == DashboardTab::Clipboard {
                self.clip_textures.clear();
            }
            if self.active_tab == DashboardTab::Cleaner {
                self.app_icon_textures.clear();
            }
            if self.active_tab == DashboardTab::Mouse {
                self.running_apps.clear();
                self.installed_gui_apps.clear();
            }
            self.active_tab = tab;
            self.recording_shortcut = None;
            self.refresh_active_tab_now();
        }
    }

    pub fn on_focused(&mut self) {
        self.permissions = core_window::get_permissions_status();
        self.input_cfg = core_input::get_input_config();
        self.refresh_active_tab_now();
    }

    /// Release cached textures and large dashboard vectors when hidden to the background.
    pub fn on_hidden(&mut self) {
        self.app_icon_tex = None;
        self.clip_textures.clear();
        self.app_icon_textures.clear();
        self.clip_result.items.clear();
        self.hw_data = None;
        self.cleaner_status = core_cleaner::CleanerScanStatus::default();
        self.orphans_scanned = false;
        self.running_apps.clear();
        self.installed_gui_apps.clear();
        self.show_app_picker_modal = false;
        self.recording_shortcut = None;
    }

    pub fn refresh_active_tab_now(&mut self) {
        match self.active_tab {
            DashboardTab::Displays => {
                self.display_state = core_display::get_display_system_state();
                self.audio_state = core_display::get_audio_system_state();
                self.last_display_refresh = Instant::now();
            }
            DashboardTab::Hardware => {
                self.hw_data = Some(core_monitor::get_hardware_dashboard(
                    &self.hw_sort_by,
                    self.hw_limit,
                ));
                self.last_hw_refresh = Instant::now();
            }
            DashboardTab::Clipboard => {
                self.clip_result = core_clipboard::query_clipboard_page(
                    &self.clip_query,
                    &self.clip_filter,
                    self.clip_page,
                    self.clip_page_size,
                );
                self.clip_stats = core_clipboard::get_clipboard_stats();
                self.last_clip_refresh = Instant::now();
            }
            DashboardTab::Tiling => {
                self.window_shortcuts = core_window::get_window_shortcuts();
                self.input_cfg = core_input::get_input_config();
                self.permissions = core_window::get_permissions_status();
            }
            DashboardTab::Mouse => {
                self.input_cfg = core_input::get_input_config();
                self.permissions = core_window::get_permissions_status();
                self.running_apps = core_input::list_running_gui_apps();
                self.spaces_status = core_input::get_spaces_status();
                self.last_spaces_refresh = Instant::now();
            }
            DashboardTab::Cleaner => {
                self.cleaner_status = core_cleaner::get_cleaner_scan_status();
                self.dup_excludes = core_cleaner::get_duplicate_exclude_list();
                self.last_cleaner_poll = Instant::now();
            }
            DashboardTab::Settings => {
                self.input_cfg = core_input::get_input_config();
                self.permissions = core_window::get_permissions_status();
                self.autostart = core_cleaner::get_autostart_status();
            }
        }
    }

    pub fn trigger_update_check_async(&mut self, manual: bool) {
        {
            let mut checking = self.update_checking.lock().unwrap();
            if *checking {
                return;
            }
            *checking = true;
        }
        if manual {
            self.confirm_install_update = false;
        }
        let res_arc = self.update_result.clone();
        let chk_arc = self.update_checking.clone();
        std::thread::spawn(move || {
            let res = crate::updater::check_for_updates();
            *res_arc.lock().unwrap() = Some(res);
            *chk_arc.lock().unwrap() = false;
        });
    }

    pub fn render(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<AppUserEvent>) {
        // Load embedded App Icon texture once (downscaled to 96x96 = 36 KB)
        if self.app_icon_tex.is_none() {
            if let Some((w, h, rgba)) = decode_image_bytes_to_rgba8_max_size(APP_ICON_PNG, 96) {
                let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                self.app_icon_tex = Some(ctx.load_texture(
                    "omni_app_icon",
                    img,
                    TextureOptions::LINEAR,
                ));
            }
        }

        // Capture interactive shortcut recording if active
        if self.recording_shortcut.is_some() {
            self.handle_shortcut_recording(ctx, proxy);
        }

        let p = palette();

        // ====================================================================
        // Left Navigation Sidebar (Exact match with `.sidebar` in `base.css`)
        // ====================================================================
        egui::SidePanel::left("omni_sidebar")
            .resizable(false)
            .exact_width(210.0)
            .frame(
                egui::Frame::none()
                    .fill(p.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .inner_margin(Margin::symmetric(12.0, 14.0)),
            )
            .show(ctx, |ui| {
                // Brand Header (`.brand` + `.brand-badge`)
                ui.horizontal(|ui| {
                    let (badge_rect, _) =
                        ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
                    if let Some(tex) = &self.app_icon_tex {
                        ui.painter().image(
                            tex.id(),
                            badge_rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().rect_filled(
                            badge_rect,
                            Rounding::same(7.0),
                            p.accent,
                        );
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "M",
                            FontId::proportional(13.5),
                            Color32::WHITE,
                        );
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("Mac Omni")
                                .size(14.5)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new(format!("v{} • Metal GPU", CURRENT_VERSION))
                                .size(10.5)
                                .color(p.text_muted),
                        );
                    });
                });

                ui.add_space(10.0);
                thin_divider(ui);
                ui.add_space(4.0);

                let nav_items = [
                    (
                        DashboardTab::Displays,
                        NavIconKind::Displays,
                        tr("Displays & Audio", "Màn hình & Âm thanh"),
                    ),
                    (
                        DashboardTab::Hardware,
                        NavIconKind::Hardware,
                        tr("Hardware & Processes", "Phần cứng & Tiến trình"),
                    ),
                    (
                        DashboardTab::Clipboard,
                        NavIconKind::Clipboard,
                        tr("Clipboard (1,000)", "Clipboard (1,000)"),
                    ),
                    (
                        DashboardTab::Tiling,
                        NavIconKind::Tiling,
                        tr("Window Tiling", "Chia Cửa sổ"),
                    ),
                    (
                        DashboardTab::Mouse,
                        NavIconKind::Mouse,
                        tr("External Mouse", "Chuột rời"),
                    ),
                    (
                        DashboardTab::Cleaner,
                        NavIconKind::Cleaner,
                        tr("System Cleaner", "Dọn Rác & Gỡ App"),
                    ),
                    (
                        DashboardTab::Settings,
                        NavIconKind::Settings,
                        tr("Settings & Permissions", "Cài đặt & Quyền"),
                    ),
                ];

                for (tab, icon_kind, label) in nav_items {
                    let active = self.active_tab == tab;
                    if nav_sidebar_button(ui, active, icon_kind, label).clicked() {
                        self.switch_tab(tab);
                    }
                    ui.add_space(2.0);
                }

                // Bottom-aligned Sidebar Footer in standard TopDown layout (never clipped vertically or horizontally)
                let footer_reserved_h = 92.0;
                let spacer_h = (ui.available_height() - footer_reserved_h).max(8.0);
                ui.add_space(spacer_h);

                thin_divider(ui);
                ui.add_space(4.0);

                // Footer Row 1: Language Selector + Theme Toggle Button
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let theme_btn_w = 34.0;
                    let combo_w = (ui.available_width() - theme_btn_w - 6.0).max(108.0);

                    ComboBox::from_id_salt("sidebar_lang_select")
                        .width(combo_w)
                        .selected_text(
                            RichText::new(if is_vi() {
                                "🌐 Tiếng Việt"
                            } else {
                                "🌐 English"
                            })
                            .size(11.5)
                            .strong()
                            .color(p.text_primary),
                        )
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(is_vi(), "🌐 Tiếng Việt (VI)")
                                .clicked()
                                && !is_vi()
                            {
                                set_lang("vi");
                                let _ = proxy.send_event(AppUserEvent::SetAppLang("vi".to_string()));
                            }
                            if ui
                                .selectable_label(!is_vi(), "🌐 English (EN)")
                                .clicked()
                                && is_vi()
                            {
                                set_lang("en");
                                let _ = proxy.send_event(AppUserEvent::SetAppLang("en".to_string()));
                            }
                        });

                    let theme_icon = if is_light_theme() { "☀" } else { "🌙" };
                    let theme_btn = egui::Button::new(
                        RichText::new(theme_icon)
                            .size(13.5)
                            .color(p.text_primary),
                    )
                    .fill(p.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .rounding(Rounding::same(6.0));
                    if ui
                        .add_sized([theme_btn_w, 28.0], theme_btn)
                        .on_hover_text(tr(
                            "Switch Dark / Light App Theme",
                            "Đổi giao diện Sáng / Tối",
                        ))
                        .clicked()
                    {
                        let next = if is_light_theme() { "dark" } else { "light" };
                        set_theme(next);
                        let _ = proxy.send_event(AppUserEvent::SetAppTheme(next.to_string()));
                    }
                });

                ui.add_space(4.0);

                // Footer Row 2: About (or Update badge) + Hide App
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let half_w = ((ui.available_width() - 6.0) * 0.5).max(76.0);

                    let upd_ver = self
                        .update_result
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|r| {
                            if r.has_update
                                && r.latest_version != self.input_cfg.skipped_update_version
                            {
                                Some(r.latest_version.clone())
                            } else {
                                None
                            }
                        });

                    if let Some(ver) = upd_ver {
                        let btn = egui::Button::new(
                            RichText::new(format!("🔔 v{}", ver))
                                .size(11.5)
                                .strong()
                                .color(p.warning),
                        )
                        .fill(p.warning_soft)
                        .stroke(Stroke::new(1.0_f32, p.warning))
                        .rounding(Rounding::same(6.0));
                        if ui.add_sized([half_w, 28.0], btn).clicked() {
                            self.show_about_modal = true;
                        }
                    } else {
                        let btn = egui::Button::new(
                            RichText::new(tr("ℹ About", "ℹ Giới thiệu"))
                                .size(11.5)
                                .strong()
                                .color(p.text_primary),
                        )
                        .fill(p.bg_card_hover)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .rounding(Rounding::same(6.0));
                        if ui.add_sized([half_w, 28.0], btn).clicked() {
                            self.show_about_modal = true;
                        }
                    }

                    let right_w = ui.available_width().max(76.0);
                    let hide_btn = egui::Button::new(
                        RichText::new(tr("Hide App", "Ẩn App"))
                            .size(11.5)
                            .strong()
                            .color(p.text_primary),
                    )
                    .fill(p.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .rounding(Rounding::same(6.0));
                    if ui.add_sized([right_w, 28.0], hide_btn).clicked() {
                        let _ = proxy.send_event(AppUserEvent::HideLauncher);
                    }
                });
            });

        // ====================================================================
        // Main Content Area (`.main-content` in `base.css`)
        // ====================================================================
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_primary)
                    .inner_margin(Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                // Missing Accessibility Permission Alert Banner on Tiling / Mouse tabs
                if (self.active_tab == DashboardTab::Tiling || self.active_tab == DashboardTab::Mouse)
                    && !self.permissions.accessibility_granted
                {
                    egui::Frame::none()
                        .fill(p.warning_soft)
                        .stroke(Stroke::new(1.0_f32, p.warning))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tr(
                                        "⚠ Accessibility Permission is required for Window Tiling, Alt-Tab & External Mouse features.",
                                        "⚠ Cần cấp quyền Accessibility (Trợ năng) để sử dụng Chia cửa sổ, Alt-Tab và Chuột rời.",
                                    ))
                                    .size(12.0)
                                    .strong()
                                    .color(p.warning),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if primary_button(
                                            ui,
                                            tr("Grant Accessibility", "Cấp quyền Accessibility"),
                                        )
                                        .clicked()
                                        {
                                            core_window::request_or_open_permission("accessibility");
                                            core_input::notify_permissions_changed();
                                            self.permissions = core_window::get_permissions_status();
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(6.0);
                }

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        DashboardTab::Displays => self.render_tab_displays(ui, ctx, proxy),
                        DashboardTab::Hardware => self.render_tab_hardware(ui, ctx),
                        DashboardTab::Clipboard => self.render_tab_clipboard(ui, ctx, proxy),
                        DashboardTab::Tiling => self.render_tab_tiling(ui, proxy),
                        DashboardTab::Mouse => self.render_tab_mouse(ui, proxy),
                        DashboardTab::Cleaner => self.render_tab_cleaner(ui, ctx),
                        DashboardTab::Settings => self.render_tab_settings(ui, proxy),
                    });
            });

        if self.show_about_modal {
            self.render_about_modal(ctx, proxy);
        }
        if self.show_app_picker_modal {
            self.render_app_picker_modal(ctx);
        }

        // Floating Toast Notification Overlay (bottom-right, never pushes UI layout down)
        render_floating_toast(ctx, &mut self.toast);
    }

    // ========================================================================
    // Screen 1: Displays & Audio (Matching `ui/js/screens/display-audio.js`)
    // ========================================================================
    fn render_tab_displays(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        if self.last_display_refresh.elapsed() >= Duration::from_secs(5) {
            self.display_state = core_display::get_display_system_state();
            self.audio_state = core_display::get_audio_system_state();
            self.last_display_refresh = Instant::now();
            self.last_audio_refresh = Instant::now();
        } else if self.last_audio_refresh.elapsed() >= Duration::from_millis(1500) {
            self.audio_state = core_display::get_audio_system_state();
            self.last_audio_refresh = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_millis(1500));

        let p = palette();

        // 1. Page Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr("Displays & Audio", "Màn hình & Âm thanh"))
                    .size(16.5)
                    .strong()
                    .color(p.text_primary),
            );
            tip_badge(
                ui,
                tr(
                    "Manage multi-monitor brightness, Night Shift warmth, and switch Speakers / Headphones / Microphones.",
                    "Quản lý độ sáng màn hình, độ ấm màu bảo vệ mắt (Night Shift) và chuyển đổi nhanh Loa / Tai nghe / Micro.",
                ),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if secondary_button(ui, tr("Refresh", "Làm mới")).clicked() {
                    self.display_state = core_display::get_display_system_state();
                    self.audio_state = core_display::get_audio_system_state();
                }
                if secondary_button(ui, tr("Reset Color Filters", "Đặt lại màu")).clicked() {
                    core_display::reset_all_display_filters();
                    self.display_state = core_display::get_display_system_state();
                    self.show_toast(tr(
                        "Reset all display color filters to 100%.",
                        "Đã đặt lại toàn bộ bộ lọc màu màn hình.",
                    ));
                }
            });
        });

        ui.add_space(6.0);

        // 2. Sub-tab Bar: Displays | Audio + Quick Actions
        ui.horizontal(|ui| {
            pill_tabs_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let disp_lbl = format!(
                        "☀ {} ({})",
                        tr("Displays", "Màn hình"),
                        self.display_state.displays.len()
                    );
                    if pill_tab_button(
                        ui,
                        self.display_subtab == DisplaySubTab::Displays,
                        &disp_lbl,
                    )
                    .clicked()
                    {
                        self.display_subtab = DisplaySubTab::Displays;
                    }
                    if pill_tab_button(
                        ui,
                        self.display_subtab == DisplaySubTab::Audio,
                        tr("🔊 Audio Devices", "🔊 Âm thanh"),
                    )
                    .clicked()
                    {
                        self.display_subtab = DisplaySubTab::Audio;
                        self.audio_state = core_display::get_audio_system_state();
                        self.last_audio_refresh = Instant::now();
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.display_subtab == DisplaySubTab::Displays {
                    if secondary_button(ui, tr("⚡ Full + Copy", "⚡ Chụp + Copy"))
                        .on_hover_text(&self.input_cfg.screenshot_fullscreen_copy_shortcut)
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                            display_id: 0,
                            instant_copy: true,
                        });
                    }
                    if secondary_button(ui, tr("🖼 Full + Edit", "🖼 Chụp màn + Sửa"))
                        .on_hover_text(&self.input_cfg.screenshot_fullscreen_edit_shortcut)
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                            display_id: 0,
                            instant_copy: false,
                        });
                    }
                    if primary_button(ui, tr("✂ Capture Region", "✂ Chụp vùng"))
                        .on_hover_text(&self.input_cfg.screenshot_shortcut)
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
                    }
                } else {
                    let out_name = if self.audio_state.current_output_name.is_empty() {
                        tr("System Default", "Mặc định")
                    } else {
                        &self.audio_state.current_output_name
                    };
                    badge_pill(
                        ui,
                        &format!("🔊 {}", out_name),
                        p.accent_text,
                        p.accent_soft,
                    );
                }
            });
        });

        thin_divider(ui);
        ui.add_space(4.0);

        if self.display_subtab == DisplaySubTab::Displays {
            // Top 3 Quick Appearance Cards (macOS Mode | App Theme | Language)
            ui.columns(3, |cols| {
                // Card 1: macOS System Appearance
                card_frame().show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("macOS Appearance", "Giao diện macOS"))
                                .size(12.0)
                                .strong(),
                        );
                        tip_badge(
                            ui,
                            tr(
                                "Switch system-wide macOS appearance between Light and Dark mode.",
                                "Chuyển đổi chế độ Sáng / Tối của toàn hệ điều hành macOS.",
                            ),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let is_dark = self.display_state.dark_mode;
                            if (if is_dark {
                                primary_button(ui, tr("🌙 Dark", "🌙 Tối"))
                            } else {
                                secondary_button(ui, tr("🌙 Dark", "🌙 Tối"))
                            })
                            .clicked()
                                && !is_dark
                            {
                                self.display_state.dark_mode = core_display::set_dark_mode(true);
                            }
                            if (if !is_dark {
                                primary_button(ui, tr("☀ Light", "☀ Sáng"))
                            } else {
                                secondary_button(ui, tr("☀ Light", "☀ Sáng"))
                            })
                            .clicked()
                                && is_dark
                            {
                                self.display_state.dark_mode = core_display::set_dark_mode(false);
                            }
                        });
                    });
                });

                // Card 2: App Theme
                card_frame().show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("App Theme", "Giao diện App"))
                                .size(12.0)
                                .strong(),
                        );
                        tip_badge(
                            ui,
                            tr(
                                "Customize Mac Omni Tool's own window theme independently.",
                                "Tùy chỉnh màu nền riêng của ứng dụng Mac Omni Tool.",
                            ),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let light = is_light_theme();
                            if (if !light {
                                primary_button(ui, tr("🌙 Dark", "🌙 Tối"))
                            } else {
                                secondary_button(ui, tr("🌙 Dark", "🌙 Tối"))
                            })
                            .clicked()
                                && light
                            {
                                set_theme("dark");
                                let _ =
                                    proxy.send_event(AppUserEvent::SetAppTheme("dark".to_string()));
                            }
                            if (if light {
                                primary_button(ui, tr("☀ Light", "☀ Sáng"))
                            } else {
                                secondary_button(ui, tr("☀ Light", "☀ Sáng"))
                            })
                            .clicked()
                                && !light
                            {
                                set_theme("light");
                                let _ = proxy
                                    .send_event(AppUserEvent::SetAppTheme("light".to_string()));
                            }
                        });
                    });
                });

                // Card 3: Language
                card_frame().show(&mut cols[2], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("Language", "Ngôn ngữ"))
                                .size(12.0)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let vi = is_vi();
                            if (if !vi {
                                primary_button(ui, "EN")
                            } else {
                                secondary_button(ui, "EN")
                            })
                            .clicked()
                                && vi
                            {
                                set_lang("en");
                                let _ =
                                    proxy.send_event(AppUserEvent::SetAppLang("en".to_string()));
                            }
                            if (if vi {
                                primary_button(ui, "VN")
                            } else {
                                secondary_button(ui, "VN")
                            })
                            .clicked()
                                && !vi
                            {
                                set_lang("vi");
                                let _ =
                                    proxy.send_event(AppUserEvent::SetAppLang("vi".to_string()));
                            }
                        });
                    });
                });
            });

            ui.add_space(8.0);

            // Connected Displays Cards
            for disp in &mut self.display_state.displays {
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let icon = if disp.is_builtin { "💻" } else { "🖥" };
                        ui.label(
                            RichText::new(format!("{} {}", icon, disp.name))
                                .size(14.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        badge_pill(
                            ui,
                            &format!("{}×{}", disp.width, disp.height),
                            p.text_secondary,
                            p.bg_input,
                        );
                        if disp.is_main {
                            badge_pill(
                                ui,
                                tr("Main", "Chính"),
                                p.accent_text,
                                p.accent_soft,
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(
                                ui,
                                if disp.supports_hardware_brightness {
                                    "DDC/CI & Native"
                                } else {
                                    "Gamma Dimming"
                                },
                                p.success,
                                p.success_soft,
                            );
                        });
                    });

                    ui.add_space(6.0);

                    // 3-Column Sliders: Brightness | Extra Dimming | Night Shift Warmth
                    ui.columns(3, |cols| {
                        // Col 1: Brightness
                        cols[0].horizontal(|ui| {
                            ui.label(
                                RichText::new(tr("☀ Brightness", "☀ Độ sáng"))
                                    .size(11.5)
                                    .color(p.text_secondary),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{:.0}%",
                                            (disp.brightness * 100.0).round()
                                        ))
                                        .font(FontId::monospace(11.5))
                                        .strong()
                                        .color(p.accent_text),
                                    );
                                },
                            );
                        });
                        let mut br_pct = (disp.brightness * 100.0).round();
                        if cols[0]
                            .add_sized(
                                [cols[0].available_width(), 18.0],
                                egui::Slider::new(&mut br_pct, 5.0..=100.0).show_value(false),
                            )
                            .changed()
                        {
                            disp.brightness = (br_pct / 100.0) as f32;
                            let _ = core_display::set_display_brightness(disp.id, disp.brightness);
                        }

                        // Col 2: Software Extra Dimming
                        cols[1].horizontal(|ui| {
                            ui.label(
                                RichText::new(tr("🔅 Extra Dimming", "🔅 Làm tối thêm"))
                                    .size(11.5)
                                    .color(p.text_secondary),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{:.0}%",
                                            (disp.software_dimming * 100.0).round()
                                        ))
                                        .font(FontId::monospace(11.5))
                                        .strong()
                                        .color(p.accent_text),
                                    );
                                },
                            );
                        });
                        let mut dim_pct = (disp.software_dimming * 100.0).round();
                        if cols[1]
                            .add_sized(
                                [cols[1].available_width(), 18.0],
                                egui::Slider::new(&mut dim_pct, 20.0..=100.0).show_value(false),
                            )
                            .changed()
                        {
                            disp.software_dimming = (dim_pct / 100.0) as f32;
                            let _ = core_display::set_display_color_filter(
                                disp.id,
                                disp.software_dimming,
                                disp.warmth,
                            );
                        }

                        // Col 3: Night Shift Warmth
                        cols[2].horizontal(|ui| {
                            ui.label(
                                RichText::new(tr("🌅 Warmth (Night Shift)", "🌅 Độ ấm màu (Night Shift)"))
                                    .size(11.5)
                                    .color(p.text_secondary),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{:.0}%",
                                            (disp.warmth * 100.0).round()
                                        ))
                                        .font(FontId::monospace(11.5))
                                        .strong()
                                        .color(p.warning),
                                    );
                                },
                            );
                        });
                        let mut warm_pct = (disp.warmth * 100.0).round();
                        if cols[2]
                            .add_sized(
                                [cols[2].available_width(), 18.0],
                                egui::Slider::new(&mut warm_pct, 0.0..=100.0).show_value(false),
                            )
                            .changed()
                        {
                            disp.warmth = (warm_pct / 100.0) as f32;
                            let _ = core_display::set_display_color_filter(
                                disp.id,
                                disp.software_dimming,
                                disp.warmth,
                            );
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        for preset in [25u32, 50, 75, 100] {
                            if secondary_button(ui, &format!("{}%", preset)).clicked() {
                                disp.brightness = preset as f32 / 100.0;
                                let _ =
                                    core_display::set_display_brightness(disp.id, disp.brightness);
                            }
                        }
                        if secondary_button(ui, tr("↺ Default", "↺ Mặc định")).clicked() {
                            disp.software_dimming = 1.0;
                            disp.warmth = 0.0;
                            let _ = core_display::set_display_color_filter(disp.id, 1.0, 0.0);
                        }
                    });
                });
                ui.add_space(8.0);
            }
        } else {
            // Audio Sub-Tab
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr("🔊 Master Output Volume", "🔊 Âm lượng Đầu ra (Master Volume)"))
                            .size(14.0)
                            .strong(),
                    );
                    tip_badge(
                        ui,
                        tr(
                            "Control master output volume and mute state directly.",
                            "Điều chỉnh âm lượng tổng của thiết bị phát âm thanh đang chọn.",
                        ),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let vol_lbl = if self.audio_state.muted {
                            tr("🔇 Muted", "🔇 Đã tắt tiếng").to_string()
                        } else {
                            format!("{:.0}%", (self.audio_state.volume * 100.0).round())
                        };
                        badge_pill(ui, &vol_lbl, p.accent_text, p.accent_soft);
                    });
                });

                ui.add_space(6.0);
                let mut vol_pct = (self.audio_state.volume * 100.0).round();
                if ui
                    .add_sized(
                        [ui.available_width(), 20.0],
                        egui::Slider::new(&mut vol_pct, 0.0..=100.0).suffix("%"),
                    )
                    .changed()
                {
                    let v = (vol_pct / 100.0) as f32;
                    self.audio_state = core_display::set_audio_volume(Some(v), Some(false));
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mute_btn_lbl = if self.audio_state.muted {
                        tr("🔊 Unmute", "🔊 Bật tiếng")
                    } else {
                        tr("🔇 Mute", "🔇 Tắt tiếng")
                    };
                    if (if self.audio_state.muted {
                        danger_button(ui, mute_btn_lbl)
                    } else {
                        secondary_button(ui, mute_btn_lbl)
                    })
                    .clicked()
                    {
                        let next_m = !self.audio_state.muted;
                        self.audio_state = core_display::set_audio_volume(None, Some(next_m));
                    }
                    for v in [25u32, 50, 75, 100] {
                        if secondary_button(ui, &format!("{}%", v)).clicked() {
                            self.audio_state = core_display::set_audio_volume(
                                Some(v as f32 / 100.0),
                                Some(false),
                            );
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // Output & Input Devices Card
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(
                            "🎧 Output & Input Audio Devices",
                            "🎧 Thiết bị Đầu ra & Đầu vào (Speakers / AirPods / Mic)",
                        ))
                        .size(14.0)
                        .strong(),
                    );
                    tip_badge(
                        ui,
                        tr(
                            "Click any speaker or headphone card below to switch macOS audio output immediately.",
                            "Nhấp vào tên Loa, Tai nghe Bluetooth, AirPods hoặc Màn hình rời để chuyển âm thanh ngay lập tức.",
                        ),
                    );
                });
                ui.add_space(6.0);

                let out_devs = self.audio_state.output_devices.clone();
                for pair in out_devs.chunks(2) {
                    ui.columns(2, |cols| {
                        for (idx, dev) in pair.iter().enumerate() {
                            let active = dev.id == self.audio_state.current_output_id
                                || dev.is_default_output;
                            let stroke_col = if active { p.accent } else { p.border };
                            let fill_col = if active { p.accent_soft } else { p.bg_input };
                            let resp = egui::Frame::none()
                                .fill(fill_col)
                                .stroke(Stroke::new(
                                    if active { 1.5_f32 } else { 1.0_f32 },
                                    stroke_col,
                                ))
                                .rounding(Rounding::same(8.0))
                                .inner_margin(Margin::symmetric(10.0, 8.0))
                                .show(&mut cols[idx], |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(sanitize_ui_text(&dev.icon)).size(16.0),
                                        );
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            ui.label(
                                                RichText::new(&dev.name)
                                                    .size(12.5)
                                                    .strong()
                                                    .color(if active {
                                                        p.accent_text
                                                    } else {
                                                        p.text_primary
                                                    }),
                                            );
                                            ui.label(
                                                RichText::new(translate_audio_kind(
                                                    &dev.kind_label,
                                                ))
                                                .size(11.0)
                                                .color(p.text_muted),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if active {
                                                    ui.label(
                                                        RichText::new("●")
                                                            .size(14.0)
                                                            .color(p.success),
                                                    );
                                                }
                                            },
                                        );
                                    });
                                })
                                .response
                                .interact(Sense::click());
                            if resp.clicked() && !active {
                                if let Ok(st) = core_display::set_audio_device(dev.id, true) {
                                    self.audio_state = st;
                                }
                            }
                        }
                    });
                    ui.add_space(4.0);
                }

                if !self.audio_state.input_devices.is_empty() {
                    thin_divider(ui);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("🎙 Input Microphone:", "🎙 Micro Đầu vào:"))
                                .size(12.5)
                                .strong(),
                        );
                        let cur_in_name = self
                            .audio_state
                            .input_devices
                            .iter()
                            .find(|d| d.is_default_input)
                            .map(|d| {
                                format!("{} ({})", d.name, translate_audio_kind(&d.kind_label))
                            })
                            .unwrap_or_else(|| tr("Default Microphone", "Micro mặc định").to_string());
                        let in_devs = self.audio_state.input_devices.clone();
                        let mic_combo_w = (ui.available_width() * 0.45).clamp(150.0, 260.0);
                        ComboBox::from_id_salt("audio_input_mic_combo")
                            .width(mic_combo_w)
                            .truncate()
                            .selected_text(cur_in_name)
                            .show_ui(ui, |ui| {
                                ui.set_min_width(280.0);
                                for dev in in_devs {
                                    let lbl = format!(
                                        "{} ({})",
                                        dev.name,
                                        translate_audio_kind(&dev.kind_label)
                                    );
                                    if ui.selectable_label(dev.is_default_input, lbl).clicked()
                                        && !dev.is_default_input
                                    {
                                        if let Ok(st) =
                                            core_display::set_audio_device(dev.id, false)
                                        {
                                            self.audio_state = st;
                                        }
                                    }
                                }
                            });
                    });
                }
            });
        }
    }

    // ========================================================================
    // Screen 2: Hardware & Processes (Matching `ui/js/screens/hardware-monitor.js`)
    // ========================================================================
    fn render_tab_hardware(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let interval = Duration::from_secs(self.hw_interval_sec.max(2));
        if self.hw_data.is_none()
            || (self.hw_auto_refresh && self.last_hw_refresh.elapsed() >= interval)
        {
            self.hw_data = Some(core_monitor::get_hardware_dashboard(
                &self.hw_sort_by,
                self.hw_limit,
            ));
            self.last_hw_refresh = Instant::now();
        }
        if self.hw_auto_refresh {
            ctx.request_repaint_after(interval);
        }

        let p = palette();

        // 1. Page Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr("Hardware & Processes", "Phần cứng & Tiến trình"))
                    .size(16.5)
                    .strong()
                    .color(p.text_primary),
            );
            tip_badge(
                ui,
                tr(
                    "Monitor real-time CPU, GPU, RAM, SSD, Network I/O, Apple Silicon thermal sensors, and manage running processes.",
                    "Theo dõi trực tiếp CPU, GPU, RAM, Nhiệt độ cảm biến Apple Silicon, Pin, Ổ cứng, Mạng và đóng các tiến trình treo.",
                ),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if secondary_button(ui, tr("Refresh", "Làm mới")).clicked() {
                    self.hw_data = Some(core_monitor::get_hardware_dashboard(
                        &self.hw_sort_by,
                        self.hw_limit,
                    ));
                    self.last_hw_refresh = Instant::now();
                }

                let auto_lbl = if self.hw_auto_refresh {
                    tr("● Auto", "● Tự động")
                } else {
                    tr("⏸ Paused", "⏸ Tạm dừng")
                };
                if (if self.hw_auto_refresh {
                    primary_button(ui, auto_lbl)
                } else {
                    secondary_button(ui, auto_lbl)
                })
                .clicked()
                {
                    self.hw_auto_refresh = !self.hw_auto_refresh;
                }

                ComboBox::from_id_salt("hw_interval_combo")
                    .width(64.0)
                    .selected_text(format!("{}s", self.hw_interval_sec))
                    .show_ui(ui, |ui| {
                        for sec in [2u64, 3, 5, 10] {
                            ui.selectable_value(&mut self.hw_interval_sec, sec, format!("{}s", sec));
                        }
                    });
            });
        });

        ui.add_space(6.0);

        let Some(hw) = self.hw_data.clone() else {
            return;
        };

        // 2. Sub-tab Bar: Overview | Processes
        ui.horizontal(|ui| {
            pill_tabs_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if pill_tab_button(
                        ui,
                        self.hw_subtab == HardwareSubTab::Overview,
                        tr("📊 Hardware Overview", "📊 Tổng quan Phần cứng"),
                    )
                    .clicked()
                    {
                        self.hw_subtab = HardwareSubTab::Overview;
                    }
                    let proc_lbl = format!(
                        "⚙ {} ({})",
                        tr("Processes", "Tiến trình"),
                        hw.top_processes.len()
                    );
                    if pill_tab_button(
                        ui,
                        self.hw_subtab == HardwareSubTab::Processes,
                        &proc_lbl,
                    )
                    .clicked()
                    {
                        self.hw_subtab = HardwareSubTab::Processes;
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.hw_subtab == HardwareSubTab::Overview {
                    badge_pill(
                        ui,
                        &format!(
                            "↓ {}/s · ↑ {}/s",
                            format_bytes(hw.net_rx_bytes_per_sec),
                            format_bytes(hw.net_tx_bytes_per_sec)
                        ),
                        p.text_secondary,
                        p.bg_card,
                    );
                    badge_pill(
                        ui,
                        &format!(
                            "RAM: {} / {}",
                            format_bytes(hw.used_memory_bytes),
                            format_bytes(hw.total_memory_bytes)
                        ),
                        p.accent_text,
                        p.accent_soft,
                    );
                    badge_pill(
                        ui,
                        &format!("GPU: {:.1}%", hw.gpu.usage_percent),
                        p.success,
                        p.success_soft,
                    );
                    badge_pill(
                        ui,
                        &format!(
                            "CPU: {:.1}% · {:.0}°C",
                            hw.cpu_usage_percent, hw.temperatures.cpu_temp_c
                        ),
                        p.warning,
                        p.warning_soft,
                    );
                } else {
                    ComboBox::from_id_salt("hw_limit_combo")
                        .width(68.0)
                        .selected_text(format!("Top {}", self.hw_limit))
                        .show_ui(ui, |ui| {
                            for lim in [15usize, 20, 30, 50] {
                                if ui
                                    .selectable_value(
                                        &mut self.hw_limit,
                                        lim,
                                        format!("Top {}", lim),
                                    )
                                    .changed()
                                {
                                    self.hw_data = Some(core_monitor::get_hardware_dashboard(
                                        &self.hw_sort_by,
                                        self.hw_limit,
                                    ));
                                }
                            }
                        });

                    pill_tabs_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (k, lbl) in [
                                ("ram", "RAM"),
                                ("cpu", "CPU"),
                                ("disk", tr("Disk", "Ổ đĩa")),
                                ("name", tr("Name", "Tên")),
                            ] {
                                if pill_tab_button(ui, self.hw_sort_by == k, lbl).clicked()
                                    && self.hw_sort_by != k
                                {
                                    self.hw_sort_by = k.to_string();
                                    self.hw_data = Some(core_monitor::get_hardware_dashboard(
                                        &self.hw_sort_by,
                                        self.hw_limit,
                                    ));
                                }
                            }
                        });
                    });
                }
            });
        });

        thin_divider(ui);
        ui.add_space(4.0);

        if self.hw_subtab == HardwareSubTab::Overview {
            // Row 1: 3 Primary Cards (CPU | GPU | RAM)
            ui.columns(3, |cols| {
                // Card 1: CPU
                card_frame().show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new("CPU").size(13.5).strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} ({} {})",
                                    hw.cpu_brand,
                                    hw.cpu_cores,
                                    tr("cores", "nhân")
                                ))
                                .size(11.0)
                                .color(p.text_muted),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(
                                ui,
                                &format!("{:.1}°C", hw.temperatures.cpu_temp_c),
                                if hw.temperatures.cpu_temp_c >= 80.0 {
                                    p.danger
                                } else if hw.temperatures.cpu_temp_c >= 65.0 {
                                    p.warning
                                } else {
                                    p.success
                                },
                                p.bg_input,
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("{:.1}%", hw.cpu_usage_percent))
                            .size(21.0)
                            .strong()
                            .color(p.text_primary),
                    );
                    progress_bar_styled(ui, hw.cpu_usage_percent / 100.0, p.accent, 6.0);
                    ui.add_space(6.0);
                    ui.columns(4, |core_cols| {
                        for (idx, &u) in hw.per_core_usage.iter().take(12).enumerate() {
                            let col = &mut core_cols[idx % 4];
                            let core_fg = if u >= 75.0 {
                                p.danger
                            } else if u >= 40.0 {
                                p.warning
                            } else {
                                p.text_secondary
                            };
                            let w = col.available_width();
                            col.add_sized(
                                [w, 20.0],
                                egui::Button::new(
                                    RichText::new(format!("C{} {:.0}%", idx + 1, u))
                                        .font(FontId::monospace(11.0))
                                        .strong()
                                        .color(core_fg),
                                )
                                .fill(p.bg_input)
                                .stroke(Stroke::new(1.0_f32, p.border))
                                .rounding(Rounding::same(5.0))
                                .sense(Sense::hover()),
                            );
                        }
                    });
                });

                // Card 2: GPU
                card_frame().show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new("GPU").size(13.5).strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} ({} {})",
                                    hw.gpu.name,
                                    hw.gpu.cores,
                                    tr("cores", "nhân")
                                ))
                                .size(11.0)
                                .color(p.text_muted),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(
                                ui,
                                &format!("{:.1}°C", hw.gpu.temp_c),
                                p.success,
                                p.bg_input,
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("{:.1}%", hw.gpu.usage_percent))
                            .size(21.0)
                            .strong()
                            .color(p.text_primary),
                    );
                    progress_bar_styled(ui, hw.gpu.usage_percent / 100.0, p.success, 6.0);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "Render {:.0}% • Tiler {:.0}% • VRAM {} / {}",
                            hw.gpu.renderer_percent,
                            hw.gpu.tiler_percent,
                            format_bytes(hw.gpu.in_use_memory_bytes),
                            format_bytes(hw.gpu.max_recommended_memory_bytes)
                        ))
                        .size(10.5)
                        .color(p.text_muted),
                    );
                });

                // Card 3: RAM
                card_frame().show(&mut cols[2], |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new("RAM").size(13.5).strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} {}",
                                    format_bytes(hw.total_memory_bytes),
                                    tr("Unified", "Thống nhất")
                                ))
                                .size(11.0)
                                .color(p.text_muted),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(
                                ui,
                                &format!("Swap {}", format_bytes(hw.used_swap_bytes)),
                                p.text_secondary,
                                p.bg_input,
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("{:.1}%", hw.memory_usage_percent))
                            .size(21.0)
                            .strong()
                            .color(p.text_primary),
                    );
                    progress_bar_styled(ui, hw.memory_usage_percent / 100.0, p.accent, 6.0);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{}: {} • {}: {}",
                            tr("Used", "Đang dùng"),
                            format_bytes(hw.used_memory_bytes),
                            tr("Free", "Trống"),
                            format_bytes(hw.available_memory_bytes)
                        ))
                        .size(10.5)
                        .color(p.text_muted),
                    );
                });
            });

            ui.add_space(8.0);

            // Row 2: 3 Secondary Cards (Sensors & Battery | SSD Storage | Network I/O)
            ui.columns(3, |cols| {
                // Col 1: Thermal Sensors & Battery
                card_frame().show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("🌡 Thermal & Battery", "🌡 Nhiệt độ & Pin"))
                                .size(13.0)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(
                                ui,
                                &format!(
                                    "{} {:.1}°C",
                                    tr("Max", "Đỉnh"),
                                    hw.temperatures.max_temp_c
                                ),
                                p.warning,
                                p.warning_soft,
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "CPU: {:.1}°C  •  GPU: {:.1}°C",
                            hw.temperatures.cpu_temp_c, hw.temperatures.gpu_temp_c
                        ))
                        .size(12.0)
                        .color(p.text_secondary),
                    );
                    if hw.battery.has_battery {
                        ui.label(
                            RichText::new(format!(
                                "🔋 {}% ({})",
                                hw.battery.percentage,
                                translate_power_source(&hw.battery.power_source)
                            ))
                            .size(12.0)
                            .strong()
                            .color(p.success),
                        );
                    }
                    let toggle_lbl = if self.hw_show_all_sensors {
                        tr("Hide sensors ▲", "Ẩn cảm biến ▲")
                    } else {
                        tr("All sensors ▼", "Xem cảm biến ▼")
                    };
                    if secondary_button(ui, toggle_lbl).clicked() {
                        self.hw_show_all_sensors = !self.hw_show_all_sensors;
                    }
                });

                // Col 2: SSD Storage
                card_frame().show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("💿 SSD Storage", "💿 Ổ cứng SSD"))
                                .size(13.0)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(
                                ui,
                                &format!("{:.1}%", hw.disk_usage_percent),
                                p.accent_text,
                                p.accent_soft,
                            );
                        });
                    });
                    ui.add_space(4.0);
                    progress_bar_styled(ui, hw.disk_usage_percent / 100.0, p.accent, 6.0);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{}: {} / {}",
                            tr("Used", "Đã dùng"),
                            format_bytes(hw.disk_used_bytes),
                            format_bytes(hw.disk_total_bytes)
                        ))
                        .size(11.5)
                        .color(p.text_secondary),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            tr("Available", "Còn trống"),
                            format_bytes(hw.disk_available_bytes)
                        ))
                        .size(11.5)
                        .color(p.success),
                    );
                });

                // Col 3: Network I/O
                card_frame().show(&mut cols[2], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("🌐 Network I/O", "🌐 Mạng (Network I/O)"))
                                .size(13.0)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_pill(ui, &hw.host_name, p.text_secondary, p.bg_input);
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "⬇ {}: {}/s ({}: {})",
                            tr("Download", "Tải xuống"),
                            format_bytes(hw.net_rx_bytes_per_sec),
                            tr("Total", "Tổng"),
                            format_bytes(hw.net_total_rx_bytes)
                        ))
                        .size(11.5)
                        .color(p.success),
                    );
                    ui.label(
                        RichText::new(format!(
                            "⬆ {}: {}/s ({}: {})",
                            tr("Upload", "Tải lên"),
                            format_bytes(hw.net_tx_bytes_per_sec),
                            tr("Total", "Tổng"),
                            format_bytes(hw.net_total_tx_bytes)
                        ))
                        .size(11.5)
                        .color(p.accent_text),
                    );
                });
            });

            if self.hw_show_all_sensors && !hw.temperatures.sensors.is_empty() {
                ui.add_space(8.0);
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "☀ {} ({})",
                                tr("Hardware Temperature Sensors", "Chi tiết Cảm biến Nhiệt độ"),
                                hw.temperatures.sensors.len()
                            ))
                            .size(13.0)
                            .strong()
                            .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if secondary_button(ui, tr("Hide sensors ▲", "Ẩn cảm biến ▲")).clicked()
                            {
                                self.hw_show_all_sensors = false;
                            }
                        });
                    });
                    thin_divider(ui);
                    ui.columns(3, |scols| {
                        for (idx, s) in hw.temperatures.sensors.iter().enumerate() {
                            let col = &mut scols[idx % 3];
                            let temp_col = if s.celsius >= 80.0 {
                                p.danger
                            } else if s.celsius >= 65.0 {
                                p.warning
                            } else {
                                p.success
                            };
                            sub_card_frame().show(col, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(sanitize_ui_text(&s.name))
                                            .size(11.5)
                                            .strong()
                                            .color(p.text_primary),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            badge_pill(
                                                ui,
                                                &format!("{:.1}°C", s.celsius),
                                                temp_col,
                                                p.bg_card,
                                            );
                                        },
                                    );
                                });
                            });
                            col.add_space(4.0);
                        }
                    });
                });
            }

            ui.add_space(8.0);

            // Row 3: Top 5 Processes Quick Card
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(
                            "⚙ Top 5 Resource-Intensive Processes",
                            "⚙ Top 5 Tiến trình Tiêu thụ Tài nguyên",
                        ))
                        .size(13.5)
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(
                            ui,
                            tr("View All Processes →", "Xem tất cả tiến trình →"),
                        )
                        .clicked()
                        {
                            self.hw_subtab = HardwareSubTab::Processes;
                        }
                    });
                });
                thin_divider(ui);

                let mut kill_target: Option<(u32, String)> = None;
                for proc in hw.top_processes.iter().take(5) {
                    setting_row_split(
                        ui,
                        |ui| {
                            if danger_button(ui, tr("Force Quit", "Buộc thoát")).clicked() {
                                kill_target = Some((proc.pid, proc.name.clone()));
                            }
                            badge_pill(
                                ui,
                                &format!("{:.1} MB", proc.memory_mb),
                                p.accent_text,
                                p.accent_soft,
                            );
                            badge_pill(
                                ui,
                                &format!("CPU {:.1}%", proc.cpu_percent),
                                p.warning,
                                p.warning_soft,
                            );
                        },
                        |ui| {
                            badge_pill(
                                ui,
                                &format!("PID {}", proc.pid),
                                p.text_muted,
                                p.bg_input,
                            );
                            truncated_name_label(ui, &proc.name, 28, 12.5, p.text_primary);
                        },
                    );
                    thin_divider(ui);
                }
                if let Some((pid, name)) = kill_target {
                    match core_monitor::kill_process(pid, true) {
                        Ok(msg) => {
                            self.show_toast(format!("✓ {} ({})", msg, name));
                            self.hw_data = Some(core_monitor::get_hardware_dashboard(
                                &self.hw_sort_by,
                                self.hw_limit,
                            ));
                        }
                        Err(err) => self.show_toast(format!("✕ {}", err)),
                    }
                }
            });
        } else {
            // Processes Sub-Tab
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [320.0, 26.0],
                        egui::TextEdit::singleline(&mut self.hw_process_filter).hint_text(tr(
                            "🔍 Filter processes by name or PID...",
                            "🔍 Lọc tiến trình theo tên hoặc PID...",
                        )),
                    );
                });
                thin_divider(ui);

                let filter_lc = self.hw_process_filter.trim().to_lowercase();
                let mut kill_target: Option<(u32, String)> = None;

                for proc in &hw.top_processes {
                    if !filter_lc.is_empty()
                        && !proc.name.to_lowercase().contains(&filter_lc)
                        && !proc.pid.to_string().contains(&filter_lc)
                    {
                        continue;
                    }
                    setting_row_split(
                        ui,
                        |ui| {
                            if danger_button(ui, tr("Force Quit", "Buộc thoát")).clicked() {
                                kill_target = Some((proc.pid, proc.name.clone()));
                            }
                            ui.label(
                                RichText::new(format!(
                                    "R: {} / W: {}",
                                    format_bytes(proc.disk_read_bytes),
                                    format_bytes(proc.disk_written_bytes)
                                ))
                                .font(FontId::monospace(10.5))
                                .color(p.text_muted),
                            );
                            badge_pill(
                                ui,
                                &format!("{:.1} MB ({:.1}%)", proc.memory_mb, proc.memory_percent),
                                p.accent_text,
                                p.accent_soft,
                            );
                            badge_pill(
                                ui,
                                &format!("CPU {:.1}%", proc.cpu_percent),
                                p.warning,
                                p.warning_soft,
                            );
                        },
                        |ui| {
                            badge_pill(
                                ui,
                                &format!("PID {}", proc.pid),
                                p.text_muted,
                                p.bg_input,
                            );
                            truncated_name_label(ui, &proc.name, 28, 12.5, p.text_primary);
                        },
                    );
                    thin_divider(ui);
                }

                if let Some((pid, name)) = kill_target {
                    match core_monitor::kill_process(pid, true) {
                        Ok(msg) => {
                            self.show_toast(format!("✓ {} ({})", msg, name));
                            self.hw_data = Some(core_monitor::get_hardware_dashboard(
                                &self.hw_sort_by,
                                self.hw_limit,
                            ));
                        }
                        Err(err) => self.show_toast(format!("✕ {}", err)),
                    }
                }
            });
        }
    }

    // ========================================================================
    // Screen 3: Clipboard (Matching `ui/js/screens/clipboard-tab.js`)
    // ========================================================================
    fn render_tab_clipboard(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        if self.last_clip_refresh.elapsed() >= Duration::from_secs(2) {
            self.clip_result = core_clipboard::query_clipboard_page(
                &self.clip_query,
                &self.clip_filter,
                self.clip_page,
                self.clip_page_size,
            );
            self.clip_stats = core_clipboard::get_clipboard_stats();
            self.last_clip_refresh = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(2));

        let p = palette();

        // 1. Page Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr("Clipboard History (SQLite)", "Lịch sử Clipboard (SQLite)"))
                    .size(16.5)
                    .strong()
                    .color(p.text_primary),
            );
            tip_badge(
                ui,
                tr(
                    "Automatically saves up to 1,000 copied texts, links, and screenshots in local SQLite.",
                    "Tự động lưu trữ tối đa 1.000 đoạn văn bản, liên kết và hình ảnh chụp màn hình vào cơ sở dữ liệu SQLite cục bộ.",
                ),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if danger_button(ui, tr("🗑 Clear Unpinned", "🗑 Xóa mục chưa ghim"))
                    .clicked()
                {
                    let _ = core_clipboard::clear_unpinned_clipboard_items();
                    self.clip_textures.clear();
                    self.refresh_active_tab_now();
                }
                if primary_button(
                    ui,
                    &format!(
                        "⚡ {} ({})",
                        tr("Open Left Drawer", "Mở Ngăn kéo Trái"),
                        self.input_cfg.clipboard_popup_shortcut
                    ),
                )
                .clicked()
                {
                    let _ = proxy.send_event(AppUserEvent::ToggleClipboardDrawer);
                }

                let mut enabled = self.input_cfg.clipboard_enabled;
                if toggle_switch(ui, &mut enabled).changed() {
                    self.input_cfg.clipboard_enabled = enabled;
                    core_clipboard::set_clipboard_enabled(enabled);
                    self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
                }
                ui.label(
                    RichText::new(if enabled {
                        tr("Recording", "Đang ghi")
                    } else {
                        tr("Paused", "Tạm dừng")
                    })
                    .size(11.5)
                    .color(p.text_secondary),
                );
            });
        });

        ui.add_space(6.0);

        // 2. Sub-tab Bar: Filter Pills + Stat Pills
        ui.horizontal(|ui| {
            pill_tabs_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let all_lbl = format!(
                        "{} ({})",
                        tr("All", "Tất cả"),
                        self.clip_stats.total_items
                    );
                    let img_lbl = format!(
                        "🖼 {} ({})",
                        tr("Images", "Hình ảnh"),
                        self.clip_stats.image_items
                    );
                    let pin_lbl = format!(
                        "📌 {} ({})",
                        tr("Pinned", "Đã ghim"),
                        self.clip_stats.pinned_items
                    );
                    for (fk, lbl) in [
                        ("all", all_lbl.as_str()),
                        ("text", tr("📝 Text", "📝 Văn bản")),
                        ("image", img_lbl.as_str()),
                        ("pinned", pin_lbl.as_str()),
                    ] {
                        if pill_tab_button(ui, self.clip_filter == fk, lbl).clicked()
                            && self.clip_filter != fk
                        {
                            self.clip_filter = fk.to_string();
                            self.clip_page = 1;
                            self.refresh_active_tab_now();
                        }
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                badge_pill(
                    ui,
                    &format!("💾 {}", format_bytes(self.clip_stats.disk_usage_bytes)),
                    p.text_secondary,
                    p.bg_card,
                );
                badge_pill(
                    ui,
                    &format!(
                        "{}: {}/{}",
                        tr("Stored", "Lưu trữ"),
                        self.clip_stats.total_items,
                        self.clip_stats.max_capacity
                    ),
                    p.accent_text,
                    p.accent_soft,
                );
            });
        });

        thin_divider(ui);
        ui.add_space(4.0);

        // 3. Search & Pagination Toolbar Card
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                let resp = ui.add_sized(
                    [320.0, 26.0],
                    egui::TextEdit::singleline(&mut self.clip_query).hint_text(tr(
                        "🔍 Search text, path, or source app...",
                        "🔍 Tìm kiếm văn bản, đường dẫn, ứng dụng nguồn...",
                    )),
                );
                if resp.changed() {
                    self.clip_page = 1;
                    self.refresh_active_tab_now();
                }

                ComboBox::from_id_salt("clip_page_size_combo")
                    .width(95.0)
                    .selected_text(format!("{} / {}", self.clip_page_size, tr("page", "trang")))
                    .show_ui(ui, |ui| {
                        for sz in [15usize, 25, 50] {
                            if ui
                                .selectable_value(
                                    &mut self.clip_page_size,
                                    sz,
                                    format!("{} / {}", sz, tr("page", "trang")),
                                )
                                .changed()
                            {
                                self.clip_page = 1;
                                self.refresh_active_tab_now();
                            }
                        }
                    });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            self.clip_page < self.clip_result.total_pages,
                            egui::Button::new(tr("Next ▶", "Sau ▶")),
                        )
                        .clicked()
                    {
                        self.clip_page += 1;
                        self.refresh_active_tab_now();
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} {} / {} ({} {})",
                            tr("Page", "Trang"),
                            self.clip_result.page,
                            self.clip_result.total_pages,
                            self.clip_result.total_matching,
                            tr("items", "mục")
                        ))
                        .size(11.5)
                        .color(p.text_secondary),
                    );
                    if ui
                        .add_enabled(
                            self.clip_page > 1,
                            egui::Button::new(tr("◀ Prev", "◀ Trước")),
                        )
                        .clicked()
                    {
                        self.clip_page = self.clip_page.saturating_sub(1).max(1);
                        self.refresh_active_tab_now();
                    }
                });
            });
        });

        ui.add_space(6.0);

        let mut act_copy: Option<i64> = None;
        let mut act_pin: Option<i64> = None;
        let mut act_del: Option<i64> = None;

        for item in &self.clip_result.items {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    badge_pill(
                        ui,
                        if item.kind == "image" {
                            tr("🖼 Image", "🖼 Hình ảnh")
                        } else {
                            tr("📝 Text", "📝 Văn bản")
                        },
                        p.accent_text,
                        p.accent_soft,
                    );
                    if !item.source_app.is_empty() {
                        badge_pill(ui, &item.source_app, p.text_secondary, p.bg_input);
                    }
                    ui.label(
                        RichText::new(format_bytes(item.byte_size.max(0) as u64))
                            .size(11.0)
                            .color(p.text_muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if danger_button(ui, "🗑").clicked() {
                            act_del = Some(item.id);
                        }
                        let pin_lbl = if item.is_pinned {
                            tr("📌 Pinned", "📌 Đã ghim")
                        } else {
                            tr("📍 Pin", "📍 Ghim")
                        };
                        if secondary_button(ui, pin_lbl).clicked() {
                            act_pin = Some(item.id);
                        }
                        if primary_button(ui, tr("📋 Copy", "📋 Sao chép")).clicked() {
                            act_copy = Some(item.id);
                        }
                    });
                });

                if item.kind == "image" {
                    if self.clip_textures.len() > 30 {
                        self.clip_textures.clear();
                    }
                    let tex = self.clip_textures.entry(item.id).or_insert_with(|| {
                        let src = if !item.image_path.is_empty() {
                            &item.image_path
                        } else {
                            &item.image_data_url
                        };
                        if let Some((w, h, rgba)) = decode_data_url_or_path_to_rgba8(src) {
                            let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                            ctx.load_texture(
                                format!("dash_clip_{}", item.id),
                                img,
                                TextureOptions::LINEAR,
                            )
                        } else {
                            let empty = ColorImage::new([1, 1], Color32::TRANSPARENT);
                            ctx.load_texture(
                                format!("dash_clip_empty_{}", item.id),
                                empty,
                                TextureOptions::NEAREST,
                            )
                        }
                    });
                    let sz = tex.size_vec2();
                    if sz.x > 1.0 && sz.y > 1.0 {
                        let scale = (360.0 / sz.x).min(130.0 / sz.y).min(1.0);
                        ui.add(
                            egui::Image::new((tex.id(), egui::vec2(sz.x * scale, sz.y * scale)))
                                .rounding(Rounding::same(6.0)),
                        );
                    }
                } else {
                    let preview_raw: String = item.preview_text.chars().take(360).collect();
                    let preview = sanitize_ui_text(&preview_raw);
                    crate::gpu_ui::ensure_cjk_font_if_needed(ctx, &preview);
                    ui.label(RichText::new(preview).size(12.5).color(p.text_primary));
                }
            });
            ui.add_space(5.0);
        }

        if let Some(id) = act_copy {
            let _ = core_clipboard::activate_clipboard_item(id, false);
            self.show_toast(tr(
                "Copied item to macOS Clipboard!",
                "Đã sao chép mục này vào Clipboard!",
            ));
            self.refresh_active_tab_now();
        }
        if let Some(id) = act_pin {
            let _ = core_clipboard::toggle_pin_clipboard_item(id);
            self.refresh_active_tab_now();
        }
        if let Some(id) = act_del {
            let _ = core_clipboard::delete_clipboard_item(id);
            self.clip_textures.remove(&id);
            self.refresh_active_tab_now();
        }
    }

    // ========================================================================
    // Screen 4: Window Tiling (Matching `ui/js/screens/window-tiling.js`)
    // ========================================================================
    fn render_tab_tiling(&mut self, ui: &mut egui::Ui, proxy: &EventLoopProxy<AppUserEvent>) {
        let p = palette();

        // 1. Page Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr(
                    "Window Tiling & Snapping",
                    "Chia Cửa sổ (Window Tiling)",
                ))
                .size(16.5)
                .strong()
                .color(p.text_primary),
            );
            tip_badge(
                ui,
                tr(
                    "Snap windows to halves, quarters, thirds, center 75%, maximize, or move across displays via hotkeys or screen edges.",
                    "Chia nhanh cửa sổ theo Nửa màn hình, Góc 1/4, Chia 3 (1/3 & 2/3), Căn giữa 75% hoặc đẩy sang màn hình phụ.",
                ),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if secondary_button(ui, tr("↺ Restore Defaults", "↺ Khôi phục mặc định"))
                    .clicked()
                {
                    self.window_shortcuts = core_window::reset_window_shortcuts();
                    core_input::sync_window_tiling_carbon_hotkeys();
                    self.show_toast(tr(
                        "Restored all 13 default window tiling shortcuts.",
                        "Đã khôi phục toàn bộ phím tắt chia cửa sổ mặc định.",
                    ));
                }

                let mut snap_en = self.input_cfg.window_snapping_enabled;
                if toggle_switch(ui, &mut snap_en).changed() {
                    self.input_cfg.window_snapping_enabled = snap_en;
                    self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
                }
                ui.label(
                    RichText::new(if snap_en {
                        tr("Enabled", "Đã bật")
                    } else {
                        tr("Disabled", "Đã tắt")
                    })
                    .size(11.5)
                    .color(p.text_secondary),
                );
            });
        });

        ui.add_space(6.0);

        // Collect unique categories
        let mut categories: Vec<String> = Vec::new();
        for b in &self.window_shortcuts {
            if !categories.contains(&b.category) {
                categories.push(b.category.clone());
            }
        }

        // 2. Sub-tab Bar: Category Filter Pills
        ui.horizontal(|ui| {
            pill_tabs_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let all_lbl =
                        format!("{} ({})", tr("All", "Tất cả"), self.window_shortcuts.len());
                    if pill_tab_button(ui, self.tiling_category == "all", &all_lbl).clicked() {
                        self.tiling_category = "all".to_string();
                    }
                    for cat in &categories {
                        if pill_tab_button(
                            ui,
                            &self.tiling_category == cat,
                            translate_window_category(cat),
                        )
                        .clicked()
                        {
                            self.tiling_category = cat.clone();
                        }
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(tr(
                        "💡 Drag windows to screen edges or press shortcuts below",
                        "💡 Kéo cửa sổ sát mép màn hình hoặc nhấn phím tắt bên dưới",
                    ))
                    .size(11.0)
                    .color(p.text_muted),
                );
            });
        });

        thin_divider(ui);
        ui.add_space(4.0);

        // 3. Window Shortcuts Table Card
        card_frame().show(ui, |ui| {
            let shortcuts_snapshot = self.window_shortcuts.clone();
            for b in shortcuts_snapshot {
                if self.tiling_category != "all" && b.category != self.tiling_category {
                    continue;
                }
                ui.horizontal(|ui| {
                    draw_window_layout_icon(ui, &b.action);
                    ui.label(
                        RichText::new(translate_window_action(&b.action, &b.label))
                            .size(13.0)
                            .strong()
                            .color(p.text_primary),
                    );
                    badge_pill(
                        ui,
                        translate_window_category(&b.category),
                        p.text_secondary,
                        p.bg_input,
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(ui, "▶")
                            .on_hover_text(tr("Test on this window", "Thử trên cửa sổ này"))
                            .clicked()
                        {
                            let _ =
                                proxy.send_event(AppUserEvent::SnapOwnMainWindow(b.action.clone()));
                        }

                        let mut en = b.enabled;
                        if toggle_switch(ui, &mut en).changed() {
                            if let Ok(list) =
                                core_window::update_window_shortcut(&b.action, &b.shortcut, en)
                            {
                                self.window_shortcuts = list;
                                core_input::sync_window_tiling_carbon_hotkeys();
                            }
                        }

                        let is_recording = matches!(
                            &self.recording_shortcut,
                            Some(ShortcutRecordTarget::WindowTiling(act)) if act == &b.action
                        );
                        if shortcut_recorder_button(ui, &b.shortcut, is_recording).clicked() {
                            self.recording_shortcut =
                                Some(ShortcutRecordTarget::WindowTiling(b.action.clone()));
                        }
                    });
                });
                thin_divider(ui);
            }
        });
    }

    pub(super) fn add_or_select_mouse_profile(&mut self, bundle_id: &str, app_name: &str) {
        if !self
            .input_cfg
            .app_mouse_profiles
            .iter()
            .any(|p| p.bundle_id == bundle_id)
        {
            let b = bundle_id.to_lowercase();
            let icon = if b.contains("chrome") || b.contains("safari") || b.contains("arc") {
                "🌐"
            } else if b.contains("code") || b.contains("cursor") || b.contains("zed") {
                "💻"
            } else {
                "❖"
            };
            let is_browser =
                b.contains("chrome") || b.contains("safari") || b.contains("firefox") || b.contains("arc");
            self.input_cfg
                .app_mouse_profiles
                .push(core_input::AppMouseProfile {
                    bundle_id: bundle_id.to_string(),
                    app_name: app_name.to_string(),
                    icon: icon.to_string(),
                    enabled: true,
                    forward_action: if is_browser {
                        "default".to_string()
                    } else {
                        "space_right".to_string()
                    },
                    back_action: if is_browser {
                        "default".to_string()
                    } else {
                        "space_left".to_string()
                    },
                    middle_action: "default".to_string(),
                    smooth_scroll_mode: "inherit".to_string(),
                });
            self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
        }
        self.mouse_selected_profile = bundle_id.to_string();
    }

    // ========================================================================
    // Application Picker Modal (for Per-App Mouse Rules)
    // ========================================================================
    fn render_app_picker_modal(&mut self, ctx: &egui::Context) {
        let p = palette();
        let mut open = self.show_app_picker_modal;
        let mut picked: Option<(String, String)> = None;
        let mut pick_from_finder = false;

        egui::Window::new(tr(
            "📁 Select Application for Custom Mouse Profile",
            "📁 Chọn Ứng dụng để gán cấu hình Chuột riêng",
        ))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(460.0, 380.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [280.0, 26.0],
                    egui::TextEdit::singleline(&mut self.app_picker_filter).hint_text(tr(
                        "🔍 Search /Applications...",
                        "🔍 Tìm ứng dụng đã cài trên máy...",
                    )),
                );
                if secondary_button(ui, tr("📂 Browse Finder...", "📂 Duyệt Finder...")).clicked()
                {
                    pick_from_finder = true;
                }
            });
            thin_divider(ui);

            let q = self.app_picker_filter.trim().to_lowercase();
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for app in &self.installed_gui_apps {
                    if !q.is_empty()
                        && !app.app_name.to_lowercase().contains(&q)
                        && !app.bundle_id.to_lowercase().contains(&q)
                    {
                        continue;
                    }
                    let already = self
                        .input_cfg
                        .app_mouse_profiles
                        .iter()
                        .any(|prof| prof.bundle_id == app.bundle_id);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(
                                RichText::new(&app.app_name)
                                    .size(12.5)
                                    .strong()
                                    .color(p.text_primary),
                            );
                            ui.label(
                                RichText::new(&app.bundle_id)
                                    .font(FontId::monospace(10.5))
                                    .color(p.text_muted),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if already {
                                badge_pill(
                                    ui,
                                    tr("✓ Added", "✓ Đã có"),
                                    p.success,
                                    p.success_soft,
                                );
                            } else if primary_button(ui, tr("+ Select", "+ Chọn")).clicked() {
                                picked = Some((app.bundle_id.clone(), app.app_name.clone()));
                            }
                        });
                    });
                    thin_divider(ui);
                }
            });
        });

        if pick_from_finder {
            if let Some(app) = core_input::pick_application_bundle_dialog() {
                picked = Some((app.bundle_id, app.app_name));
            }
        }
        if let Some((bid, name)) = picked {
            self.add_or_select_mouse_profile(&bid, &name);
            open = false;
        }
        self.show_app_picker_modal = open;
    }

    // ========================================================================
    // About & Self-Updater Modal
    // ========================================================================
    fn render_about_modal(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<AppUserEvent>) {
        let p = palette();
        let mut open = self.show_about_modal;

        egui::Window::new(tr("ℹ About Mac Omni Tool", "ℹ Giới thiệu Mac Omni Tool"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .fixed_size(egui::vec2(500.0, 380.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.app_icon_tex {
                        ui.add(
                            egui::Image::new((tex.id(), egui::vec2(48.0, 48.0)))
                                .rounding(Rounding::same(10.0)),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Mac Omni Tool — Pure Rust Metal GPU Edition")
                                .size(15.5)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Version v{} • Single-Process egui + wgpu (Metal) • MIT License",
                                CURRENT_VERSION
                            ))
                            .size(12.0)
                            .color(p.text_secondary),
                        );
                    });
                });

                thin_divider(ui);

                let checking = *self.update_checking.lock().unwrap();
                let installing = *self.update_installing.lock().unwrap();
                let upd_opt = self.update_result.lock().unwrap().clone();

                sub_card_frame().show(ui, |ui| {
                    if checking {
                        ui.label(
                            RichText::new(tr(
                                "⏳ Checking GitHub for updates...",
                                "⏳ Đang kiểm tra bản cập nhật trên GitHub...",
                            ))
                            .size(13.0),
                        );
                        ctx.request_repaint_after(Duration::from_millis(200));
                    } else if installing {
                        ui.label(
                            RichText::new(tr(
                                "⏳ Downloading, verifying SHA-256 & installing update...",
                                "⏳ Đang tải về, kiểm tra SHA-256 và cài đặt bản cập nhật...",
                            ))
                            .size(13.0)
                            .strong()
                            .color(p.accent),
                        );
                        ctx.request_repaint_after(Duration::from_millis(200));
                    } else if let Some(res) = upd_opt {
                        if !res.ok {
                            ui.label(
                                RichText::new(format!(
                                    "⚠ {}",
                                    res.error.unwrap_or_else(|| "Update check failed".to_string())
                                ))
                                .color(p.warning),
                            );
                        } else if res.has_update {
                            ui.label(
                                RichText::new(format!(
                                    "🚀 {} v{} (Current: v{})",
                                    tr("New version available:", "Đã có phiên bản mới:"),
                                    res.latest_version,
                                    res.current_version
                                ))
                                .size(13.5)
                                .strong()
                                .color(p.success),
                            );
                            let notes = if is_vi() && !res.notes_vi.is_empty() {
                                &res.notes_vi
                            } else {
                                &res.notes_en
                            };
                            if !notes.is_empty() {
                                ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                                    ui.label(
                                        RichText::new(notes).size(11.5).color(p.text_secondary),
                                    );
                                });
                            }

                            ui.horizontal(|ui| {
                                if !self.confirm_install_update {
                                    if primary_button(ui, tr("⬇ Update Now", "⬇ Cập nhật ngay"))
                                        .clicked()
                                    {
                                        self.confirm_install_update = true;
                                    }
                                    if secondary_button(
                                        ui,
                                        tr("Skip This Version", "Giữ nguyên bản hiện tại"),
                                    )
                                    .clicked()
                                    {
                                        self.input_cfg.skipped_update_version =
                                            res.latest_version.clone();
                                        self.input_cfg = core_input::update_input_config(
                                            self.input_cfg.clone(),
                                        );
                                        self.show_toast(tr(
                                            "Kept current version; skipped this update.",
                                            "Đã chọn giữ nguyên phiên bản hiện tại.",
                                        ));
                                    }
                                } else {
                                    if primary_button(
                                        ui,
                                        tr(
                                            "✓ Confirm Download & Restart",
                                            "✓ Xác nhận Tải & Khởi động lại",
                                        ),
                                    )
                                    .clicked()
                                    {
                                        let zip_url = res.zip_url.clone();
                                        let sha = res.sha256_zip.clone();
                                        let inst_arc = self.update_installing.clone();
                                        *inst_arc.lock().unwrap() = true;
                                        let p_clone = proxy.clone();
                                        std::thread::spawn(move || {
                                            if crate::updater::download_and_install_update(
                                                &zip_url, &sha,
                                            )
                                            .is_ok()
                                            {
                                                std::thread::sleep(Duration::from_millis(250));
                                                let _ = p_clone.send_event(AppUserEvent::QuitApp);
                                            }
                                            *inst_arc.lock().unwrap() = false;
                                        });
                                    }
                                    if secondary_button(ui, tr("Cancel", "Hủy")).clicked() {
                                        self.confirm_install_update = false;
                                    }
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "✓ {} (v{})",
                                    tr(
                                        "You are running the latest version!",
                                        "Bạn đang sử dụng phiên bản mới nhất!"
                                    ),
                                    res.current_version
                                ))
                                .size(13.0)
                                .strong()
                                .color(p.success),
                            );
                        }
                    } else if primary_button(
                        ui,
                        tr("🔍 Check for Updates Now", "🔍 Kiểm tra Cập nhật ngay"),
                    )
                    .clicked()
                    {
                        self.trigger_update_check_async(true);
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if secondary_button(ui, "🌐 GitHub: boykhacthang/mac-omni-tool").clicked() {
                        let _ = std::process::Command::new("open")
                            .arg("https://github.com/boykhacthang/mac-omni-tool")
                            .spawn();
                    }
                    if secondary_button(ui, "☕ Support on Ko-fi").clicked() {
                        let _ = std::process::Command::new("open")
                            .arg("https://ko-fi.com/I1R72A1475")
                            .spawn();
                    }
                });
            });

        self.show_about_modal = open;
    }

    // ========================================================================
    // Interactive Keyboard Shortcut Recorder
    // ========================================================================
    fn handle_shortcut_recording(
        &mut self,
        ctx: &egui::Context,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        let mut cancel = false;
        let mut recorded_combo: Option<String> = None;

        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                cancel = true;
                return;
            }
            for ev in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = ev
                {
                    let key_name = match key {
                        Key::A => Some("A"),
                        Key::B => Some("B"),
                        Key::C => Some("C"),
                        Key::D => Some("D"),
                        Key::E => Some("E"),
                        Key::F => Some("F"),
                        Key::G => Some("G"),
                        Key::H => Some("H"),
                        Key::I => Some("I"),
                        Key::J => Some("J"),
                        Key::K => Some("K"),
                        Key::L => Some("L"),
                        Key::M => Some("M"),
                        Key::N => Some("N"),
                        Key::O => Some("O"),
                        Key::P => Some("P"),
                        Key::Q => Some("Q"),
                        Key::R => Some("R"),
                        Key::S => Some("S"),
                        Key::T => Some("T"),
                        Key::U => Some("U"),
                        Key::V => Some("V"),
                        Key::W => Some("W"),
                        Key::X => Some("X"),
                        Key::Y => Some("Y"),
                        Key::Z => Some("Z"),
                        Key::Num0 => Some("0"),
                        Key::Num1 => Some("1"),
                        Key::Num2 => Some("2"),
                        Key::Num3 => Some("3"),
                        Key::Num4 => Some("4"),
                        Key::Num5 => Some("5"),
                        Key::Num6 => Some("6"),
                        Key::Num7 => Some("7"),
                        Key::Num8 => Some("8"),
                        Key::Num9 => Some("9"),
                        Key::ArrowLeft => Some("Left"),
                        Key::ArrowRight => Some("Right"),
                        Key::ArrowUp => Some("Up"),
                        Key::ArrowDown => Some("Down"),
                        Key::Enter => Some("Return"),
                        Key::Space => Some("Space"),
                        Key::Tab => Some("Tab"),
                        Key::F1 => Some("F1"),
                        Key::F2 => Some("F2"),
                        Key::F3 => Some("F3"),
                        Key::F4 => Some("F4"),
                        Key::F5 => Some("F5"),
                        Key::F6 => Some("F6"),
                        Key::F7 => Some("F7"),
                        Key::F8 => Some("F8"),
                        Key::F9 => Some("F9"),
                        Key::F10 => Some("F10"),
                        Key::F11 => Some("F11"),
                        Key::F12 => Some("F12"),
                        _ => None,
                    };

                    if let Some(k) = key_name {
                        let mut parts = Vec::new();
                        if modifiers.ctrl {
                            parts.push("Ctrl");
                        }
                        if modifiers.alt {
                            parts.push("Option");
                        }
                        if modifiers.shift {
                            parts.push("Shift");
                        }
                        if modifiers.command || modifiers.mac_cmd {
                            parts.push("Cmd");
                        }
                        if !parts.is_empty() || k.starts_with('F') {
                            parts.push(k);
                            recorded_combo =
                                Some(core_input::canonicalize_shortcut(&parts.join("+")));
                            break;
                        }
                    }
                }
            }
        });

        if cancel {
            self.recording_shortcut = None;
            return;
        }

        if let Some(combo) = recorded_combo {
            if let Some(target) = self.recording_shortcut.take() {
                match target {
                    ShortcutRecordTarget::WindowTiling(action) => {
                        let cur_en = self
                            .window_shortcuts
                            .iter()
                            .find(|b| b.action == action)
                            .map(|b| b.enabled)
                            .unwrap_or(true);
                        if let Ok(list) =
                            core_window::update_window_shortcut(&action, &combo, cur_en)
                        {
                            self.window_shortcuts = list;
                            core_input::sync_window_tiling_carbon_hotkeys();
                            self.show_toast(format!("✓ {}: {}", action, combo));
                        }
                    }
                    ShortcutRecordTarget::ConfigField(field) => {
                        match field.as_str() {
                            "screenshot_shortcut" => self.input_cfg.screenshot_shortcut = combo,
                            "screenshot_fullscreen_edit_shortcut" => {
                                self.input_cfg.screenshot_fullscreen_edit_shortcut = combo
                            }
                            "screenshot_fullscreen_copy_shortcut" => {
                                self.input_cfg.screenshot_fullscreen_copy_shortcut = combo
                            }
                            "clipboard_popup_shortcut" => {
                                self.input_cfg.clipboard_popup_shortcut = combo
                            }
                            "alt_tab_shortcut" => self.input_cfg.alt_tab_shortcut = combo,
                            "dashboard_shortcut" => self.input_cfg.dashboard_shortcut = combo,
                            _ => {}
                        }
                        self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
                        let _ = proxy.send_event(AppUserEvent::SyncMenuBarConfig);
                    }
                }
            }
        }
    }
}
