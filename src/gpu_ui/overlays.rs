use crate::events::AppUserEvent;
use crate::gpu_ui::{
    badge_pill, card_frame, danger_button, format_bytes, palette, primary_button, sanitize_ui_text,
    secondary_button, sub_card_frame, thin_divider,
};
use crate::i18n::{
    is_light_theme, is_vi, set_lang, set_theme, tr, translate_audio_kind, translate_menubar_mode,
    MENUBAR_MODE_OPTIONS,
};
use crate::macos_window::decode_data_url_or_path_to_rgba8;
use egui::{
    Color32, ColorImage, ComboBox, FontId, Key, Margin, RichText, Rounding, ScrollArea, Sense,
    Stroke, TextureHandle, TextureOptions,
};
use egui_winit::winit::event_loop::EventLoopProxy;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ============================================================================
// 1. Menu Bar Dropdown Popover (`mini_window` — Matching `ui/js/overlays/menubar-popover.js`)
// ============================================================================

pub struct MiniPopoverState {
    pub display_state: core_display::DisplaySystemState,
    pub audio_state: core_display::AudioSystemState,
    pub quick_stats: core_monitor::MenuBarQuickStats,
    pub input_cfg: core_input::InputAutomationConfig,
    pub permissions: core_window::PermissionStatus,
    pub last_refresh: Instant,
}

impl MiniPopoverState {
    pub fn new() -> Self {
        Self {
            display_state: core_display::DisplaySystemState {
                displays: Vec::new(),
                dark_mode: true,
            },
            audio_state: core_display::AudioSystemState {
                volume: 0.5,
                muted: false,
                current_output_id: 0,
                current_output_name: String::new(),
                output_devices: Vec::new(),
                input_devices: Vec::new(),
            },
            quick_stats: core_monitor::MenuBarQuickStats {
                cpu_usage_percent: 0.0,
                memory_usage_percent: 0.0,
                used_memory_bytes: 0,
                total_memory_bytes: 16 * 1024 * 1024 * 1024,
                cpu_temp_c: 42.0,
                gpu_temp_c: 40.0,
                net_rx_bytes_per_sec: 0,
                net_tx_bytes_per_sec: 0,
            },
            input_cfg: core_input::get_input_config(),
            permissions: core_window::get_permissions_status(),
            last_refresh: Instant::now() - Duration::from_secs(60),
        }
    }

    pub fn refresh_now(&mut self) {
        self.display_state = core_display::get_display_system_state();
        self.audio_state = core_display::get_audio_system_state();
        self.quick_stats = core_monitor::get_menubar_stats();
        self.input_cfg = core_input::get_input_config();
        self.permissions = core_window::get_permissions_status();
        self.last_refresh = Instant::now();
    }

    pub fn on_hidden(&mut self) {}

    pub fn render(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<AppUserEvent>) {
        if self.last_refresh.elapsed() >= Duration::from_secs(2) {
            self.quick_stats = core_monitor::get_menubar_stats();
            self.audio_state = core_display::get_audio_system_state();
            if self.display_state.displays.is_empty() {
                self.display_state = core_display::get_display_system_state();
            }
            self.last_refresh = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(2));

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
            return;
        }

        let p = palette();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_primary)
                    .stroke(Stroke::new(1.5_f32, p.border))
                    .rounding(Rounding::same(14.0))
                    .inner_margin(Margin::symmetric(12.0, 10.0)),
            )
            .show(ctx, |ui| {
                let full_w = ui.available_width();
                ui.set_max_width(full_w);
                ui.spacing_mut().item_spacing.y = 6.0;

                // ------------------------------------------------------------
                // 1. Header Row: Brand [M] + Mac Omni + Telemetry | Lang + Theme + Close
                // ------------------------------------------------------------
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let (logo_rect, _) =
                        ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
                    ui.painter()
                        .rect_filled(logo_rect, Rounding::same(6.0), p.accent);
                    ui.painter().text(
                        logo_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "M",
                        FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                    ui.label(
                        RichText::new("Mac Omni")
                            .size(12.5)
                            .strong()
                            .color(p.text_primary),
                    );
                    ui.label(
                        RichText::new(format!(
                            "CPU {:.0}% • {:.0}°C • RAM {:.0}%",
                            self.quick_stats.cpu_usage_percent,
                            self.quick_stats.cpu_temp_c.max(30.0),
                            self.quick_stats.memory_usage_percent
                        ))
                        .size(10.5)
                        .color(p.text_secondary),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if secondary_button(ui, "✕").clicked() {
                            let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
                        }
                        let theme_lbl = if is_light_theme() { "🌙" } else { "☀" };
                        if secondary_button(ui, theme_lbl)
                            .on_hover_text(tr("Switch Dark/Light Theme", "Đổi giao diện Sáng/Tối"))
                            .clicked()
                        {
                            let next = if is_light_theme() { "dark" } else { "light" };
                            set_theme(next);
                            let _ = proxy.send_event(AppUserEvent::SetAppTheme(next.to_string()));
                        }
                        let lang_lbl = if is_vi() { "VI" } else { "EN" };
                        if secondary_button(ui, lang_lbl)
                            .on_hover_text(tr(
                                "Switch Language (EN/VI)",
                                "Chuyển ngôn ngữ (Tiếng Việt / English)",
                            ))
                            .clicked()
                        {
                            let next = if is_vi() { "en" } else { "vi" };
                            set_lang(next);
                            let _ = proxy.send_event(AppUserEvent::SetAppLang(next.to_string()));
                        }
                    });
                });

                // ------------------------------------------------------------
                // 2. Compact Display Settings Card
                // ------------------------------------------------------------
                card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("☀ Displays", "☀ Màn hình"))
                                .size(12.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let dark = self.display_state.dark_mode;
                            let lbl = if dark {
                                tr("🌙 macOS Dark", "🌙 macOS Tối")
                            } else {
                                tr("☀ macOS Light", "☀ macOS Sáng")
                            };
                            if secondary_button(ui, lbl).clicked() {
                                self.display_state.dark_mode = core_display::set_dark_mode(!dark);
                            }
                        });
                    });

                    for disp in &mut self.display_state.displays {
                        let is_hw = disp.supports_hardware_brightness;
                        let mut pct = if is_hw {
                            (disp.brightness * 100.0).round()
                        } else {
                            (disp.software_dimming * 100.0).round()
                        };
                        ui.horizontal(|ui| {
                            let name_short: String = disp.name.chars().take(28).collect();
                            ui.label(
                                RichText::new(name_short)
                                    .size(11.0)
                                    .color(p.text_secondary),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{:.0}%", pct))
                                            .size(11.0)
                                            .strong()
                                            .color(p.accent_text),
                                    );
                                },
                            );
                        });
                        let slider_w = ui.available_width().max(120.0);
                        ui.spacing_mut().slider_width = slider_w;
                        let range = if is_hw { 5.0..=100.0 } else { 20.0..=100.0 };
                        let slider = egui::Slider::new(&mut pct, range)
                            .show_value(false)
                            .trailing_fill(true);
                        if ui.add_sized([slider_w, 16.0], slider).changed() {
                            if is_hw {
                                disp.brightness = (pct / 100.0) as f32;
                                let _ =
                                    core_display::set_display_brightness(disp.id, disp.brightness);
                            } else {
                                disp.software_dimming = (pct / 100.0) as f32;
                                let _ = core_display::set_display_color_filter(
                                    disp.id,
                                    disp.software_dimming,
                                    disp.warmth,
                                );
                            }
                        }
                    }
                });

                // ------------------------------------------------------------
                // 3. Compact Audio & Output Device Switcher Card
                // ------------------------------------------------------------
                card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let muted = self.audio_state.muted;
                    let mut vol_pct = if muted {
                        0.0
                    } else {
                        (self.audio_state.volume * 100.0).round()
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("🔊 Volume", "🔊 Âm lượng"))
                                .size(12.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mute_icon = if muted { "🔇" } else { "🔊" };
                            if secondary_button(ui, mute_icon).clicked() {
                                self.audio_state =
                                    core_display::set_audio_volume(None, Some(!muted));
                            }
                            ui.label(
                                RichText::new(if muted {
                                    tr("Muted", "Đã tắt tiếng").to_string()
                                } else {
                                    format!("{:.0}%", vol_pct)
                                })
                                .size(11.5)
                                .strong()
                                .color(p.accent_text),
                            );
                        });
                    });

                    let slider_w = ui.available_width().max(120.0);
                    ui.spacing_mut().slider_width = slider_w;
                    let vol_slider = egui::Slider::new(&mut vol_pct, 0.0..=100.0)
                        .show_value(false)
                        .trailing_fill(true);
                    if ui.add_sized([slider_w, 16.0], vol_slider).changed() {
                        let v = (vol_pct / 100.0) as f32;
                        self.audio_state = core_display::set_audio_volume(Some(v), Some(false));
                    }

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} ({})",
                                tr("Audio Output", "Đầu ra âm thanh"),
                                self.audio_state.output_devices.len()
                            ))
                            .size(10.5)
                            .strong()
                            .color(p.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if secondary_button(ui, "🔄")
                                .on_hover_text(tr(
                                    "Refresh audio devices",
                                    "Làm mới danh sách thiết bị âm thanh",
                                ))
                                .clicked()
                            {
                                self.audio_state = core_display::get_audio_system_state();
                            }
                        });
                    });

                    let devs = self.audio_state.output_devices.clone();
                    let mut switch_dev_id: Option<u32> = None;
                    ScrollArea::vertical()
                        .id_salt("mini_audio_devs_scroll")
                        .max_height(88.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for dev in devs {
                                let is_active = dev.id == self.audio_state.current_output_id
                                    || dev.is_default_output;
                                let row_fill = if is_active { p.accent_soft } else { p.bg_input };
                                let row_stroke = if is_active {
                                    Stroke::new(1.0_f32, p.accent_border)
                                } else {
                                    Stroke::new(1.0_f32, p.border)
                                };
                                let resp = egui::Frame::none()
                                    .fill(row_fill)
                                    .stroke(row_stroke)
                                    .rounding(Rounding::same(6.0))
                                    .inner_margin(Margin::symmetric(8.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let dev_name_short: String =
                                                dev.name.chars().take(22).collect();
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} {}",
                                                    sanitize_ui_text(&dev.icon),
                                                    dev_name_short
                                                ))
                                                .size(11.5)
                                                .strong()
                                                .color(if is_active {
                                                    p.accent_text
                                                } else {
                                                    p.text_primary
                                                }),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "• {}",
                                                    translate_audio_kind(&dev.kind_label)
                                                ))
                                                .size(10.0)
                                                .color(p.text_muted),
                                            );
                                            if is_active {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        badge_pill(
                                                            ui,
                                                            tr("Active", "Đang dùng"),
                                                            p.success,
                                                            p.success_soft,
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());
                                if resp.clicked() && !is_active {
                                    switch_dev_id = Some(dev.id);
                                }
                            }
                        });
                    if let Some(did) = switch_dev_id {
                        if let Ok(st) = core_display::set_audio_device(did, true) {
                            self.audio_state = st;
                        }
                    }
                });

                // ------------------------------------------------------------
                // 4. Compact External Mouse Quick Toggles Card
                // ------------------------------------------------------------
                let ax_ok = self.permissions.accessibility_granted;
                let mouse_on = ax_ok && self.input_cfg.external_mouse_enabled;
                sub_card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr("🖱 Mouse", "🖱 Chuột rời"))
                                .size(11.5)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let smooth_on = mouse_on && self.input_cfg.smooth_scroll_enabled;
                            if (if smooth_on {
                                primary_button(ui, "🌊 120Hz")
                            } else {
                                secondary_button(ui, "🌊 120Hz")
                            })
                            .clicked()
                                && mouse_on
                            {
                                self.input_cfg.smooth_scroll_enabled =
                                    !self.input_cfg.smooth_scroll_enabled;
                                self.input_cfg =
                                    core_input::update_input_config(self.input_cfg.clone());
                            }

                            let inv_on = mouse_on && self.input_cfg.invert_external_mouse_scroll;
                            let inv_lbl = tr("🔄 Reverse", "🔄 Đảo cuộn");
                            if (if inv_on {
                                primary_button(ui, inv_lbl)
                            } else {
                                secondary_button(ui, inv_lbl)
                            })
                            .clicked()
                                && mouse_on
                            {
                                self.input_cfg.invert_external_mouse_scroll =
                                    !self.input_cfg.invert_external_mouse_scroll;
                                self.input_cfg =
                                    core_input::update_input_config(self.input_cfg.clone());
                            }

                            let master_lbl = if mouse_on {
                                tr("ON", "BẬT")
                            } else {
                                tr("OFF", "TẮT")
                            };
                            if (if mouse_on {
                                primary_button(ui, master_lbl)
                            } else {
                                secondary_button(ui, master_lbl)
                            })
                            .clicked()
                            {
                                if !ax_ok {
                                    let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
                                    let _ = proxy.send_event(AppUserEvent::ShowLauncher);
                                } else {
                                    self.input_cfg.external_mouse_enabled =
                                        !self.input_cfg.external_mouse_enabled;
                                    self.input_cfg =
                                        core_input::update_input_config(self.input_cfg.clone());
                                }
                            }
                        });
                    });
                });

                // ------------------------------------------------------------
                // 5. Compact Menu Bar Display Mode Selector Card
                // ------------------------------------------------------------
                sub_card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("❖ Menu Bar")
                                .size(11.5)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cur_mode = if !self.input_cfg.menubar_enabled {
                                "hidden".to_string()
                            } else {
                                self.input_cfg.menubar_display_mode.clone()
                            };
                            ComboBox::from_id_salt("mini_popover_menubar_mode")
                                .width(205.0)
                                .truncate()
                                .selected_text(translate_menubar_mode(&cur_mode))
                                .show_ui(ui, |ui| {
                                    ui.set_min_width(250.0);
                                    for &mode in MENUBAR_MODE_OPTIONS {
                                        let selected = cur_mode == mode;
                                        if ui
                                            .selectable_label(
                                                selected,
                                                translate_menubar_mode(mode),
                                            )
                                            .clicked()
                                            && !selected
                                        {
                                            let is_hidden = mode == "hidden";
                                            self.input_cfg.menubar_enabled = !is_hidden;
                                            self.input_cfg.menubar_display_mode = mode.to_string();
                                            self.input_cfg = core_input::update_input_config(
                                                self.input_cfg.clone(),
                                            );
                                            let _ =
                                                proxy.send_event(AppUserEvent::SyncMenuBarConfig);
                                            if is_hidden {
                                                let _ = proxy.send_event(
                                                    AppUserEvent::SetMiniExpanded(false),
                                                );
                                            }
                                        }
                                    }
                                });
                        });
                    });
                });

                // ------------------------------------------------------------
                // 6. 3 Equal Quick Action Buttons (Capture Region | Clipboard | F3)
                // ------------------------------------------------------------
                ui.columns(3, |cols| {
                    let w0 = cols[0].available_width();
                    if cols[0]
                        .add_sized(
                            [w0, 27.0],
                            egui::Button::new(
                                RichText::new(tr("📸 Capture", "📸 Chụp vùng"))
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_primary),
                            )
                            .fill(p.bg_card_hover)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .rounding(Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
                        let _ = proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
                    }

                    let w1 = cols[1].available_width();
                    if cols[1]
                        .add_sized(
                            [w1, 27.0],
                            egui::Button::new(
                                RichText::new("📋 Clipboard")
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_primary),
                            )
                            .fill(p.bg_card_hover)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .rounding(Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
                        let _ = proxy.send_event(AppUserEvent::ToggleClipboardDrawer);
                    }

                    let w2 = cols[2].available_width();
                    if cols[2]
                        .add_sized(
                            [w2, 27.0],
                            egui::Button::new(
                                RichText::new("🚀 F3")
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_primary),
                            )
                            .fill(p.bg_card_hover)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .rounding(Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
                        let _ = core_input::trigger_system_action("mission_control");
                    }
                });

                thin_divider(ui);

                // ------------------------------------------------------------
                // 7. Footer: Open Main Dashboard (flex 1) + Quit App (⏻)
                // ------------------------------------------------------------
                ui.horizontal(|ui| {
                    let quit_w = 34.0;
                    let open_w = (ui.available_width() - quit_w - 6.0).max(160.0);
                    if ui
                        .add_sized(
                            [open_w, 30.0],
                            egui::Button::new(
                                RichText::new(tr(
                                    "🚀 Open Mac Omni Tool",
                                    "🚀 Mở Bảng điều khiển (Dashboard)",
                                ))
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                            )
                            .fill(p.accent)
                            .stroke(Stroke::new(1.0_f32, p.accent))
                            .rounding(Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::SetMiniExpanded(false));
                        let _ = proxy.send_event(AppUserEvent::ShowLauncher);
                    }

                    if ui
                        .add_sized(
                            [quit_w, 30.0],
                            egui::Button::new(
                                RichText::new("⊗").size(14.0).strong().color(p.danger),
                            )
                            .fill(p.danger_soft)
                            .stroke(Stroke::new(1.0_f32, p.danger))
                            .rounding(Rounding::same(6.0)),
                        )
                        .on_hover_text(tr("Quit Mac Omni Tool", "Thoát hoàn toàn ứng dụng"))
                        .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::QuitApp);
                    }
                });

                // Auto-fit window height to content so there is never clipping or empty space
                let needed_h = (ui.min_rect().height() + 22.0).clamp(360.0, 620.0) as f64;
                let cur_h = ctx.screen_rect().height() as f64;
                if (needed_h - cur_h).abs() > 4.0 {
                    let _ = proxy.send_event(AppUserEvent::SetMiniHeight(needed_h));
                }
            });
    }
}

// ============================================================================
// 2. Left-Side Clipboard Drawer (`clip_window` — Cmd+Shift+V)
// ============================================================================

pub struct ClipboardDrawerState {
    pub query: String,
    pub filter_kind: String, // "all", "text", "image", "pinned"
    pub page: usize,
    pub page_size: usize,
    pub page_result: core_clipboard::ClipboardPageResult,
    pub stats: core_clipboard::ClipboardStats,
    pub selected_idx: usize,
    pub request_search_focus: bool,
    pub image_textures: HashMap<i64, TextureHandle>,
    pub last_poll: Instant,
}

impl ClipboardDrawerState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            filter_kind: "all".to_string(),
            page: 1,
            page_size: 25,
            page_result: core_clipboard::ClipboardPageResult {
                items: Vec::new(),
                page: 1,
                page_size: 25,
                total_matching: 0,
                total_pages: 1,
            },
            stats: core_clipboard::ClipboardStats {
                enabled: true,
                total_items: 0,
                pinned_items: 0,
                image_items: 0,
                max_capacity: 1000,
                disk_usage_bytes: 0,
                polling_interval_ms: 250,
            },
            selected_idx: 0,
            request_search_focus: false,
            image_textures: HashMap::new(),
            last_poll: Instant::now() - Duration::from_secs(10),
        }
    }

    pub fn reload(&mut self) {
        self.page_result = core_clipboard::query_clipboard_page(
            &self.query,
            &self.filter_kind,
            self.page,
            self.page_size,
        );
        self.stats = core_clipboard::get_clipboard_stats();
        if self.selected_idx >= self.page_result.items.len() {
            self.selected_idx = 0;
        }
        self.last_poll = Instant::now();
    }

    pub fn on_shown(&mut self) {
        self.page = 1;
        self.selected_idx = 0;
        self.request_search_focus = true;
        self.reload();
    }

    /// Release all cached image textures and item vectors when the drawer is hidden to the background.
    pub fn on_hidden(&mut self) {
        self.image_textures.clear();
        self.page_result.items.clear();
    }

    pub fn render(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<AppUserEvent>) {
        if self.last_poll.elapsed() >= Duration::from_millis(1200) {
            self.reload();
        }

        // Keyboard shortcuts: Escape, ArrowDown, ArrowUp, Enter
        let mut paste_selected = false;
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
            } else if i.key_pressed(Key::ArrowDown) && !self.page_result.items.is_empty() {
                self.selected_idx = (self.selected_idx + 1) % self.page_result.items.len();
            } else if i.key_pressed(Key::ArrowUp) && !self.page_result.items.is_empty() {
                if self.selected_idx == 0 {
                    self.selected_idx = self.page_result.items.len() - 1;
                } else {
                    self.selected_idx -= 1;
                }
            } else if i.key_pressed(Key::Enter) && !self.page_result.items.is_empty() {
                paste_selected = true;
            }
        });

        if paste_selected {
            if let Some(item) = self.page_result.items.get(self.selected_idx) {
                let id = item.id;
                let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(45));
                    let _ = core_clipboard::activate_clipboard_item(id, true);
                });
                return;
            }
        }

        let p = palette();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_primary)
                    .stroke(Stroke::new(1.5_f32, p.border))
                    .rounding(Rounding::same(14.0))
                    .inner_margin(Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr("📋 Clipboard Drawer", "📋 Ngăn kéo Clipboard"))
                            .size(14.5)
                            .strong()
                            .color(p.text_primary),
                    );
                    badge_pill(
                        ui,
                        &format!("{}/{}", self.stats.total_items, self.stats.max_capacity),
                        p.accent_text,
                        p.accent_soft,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(ui, "✕").clicked() {
                            let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
                        }
                        if danger_button(ui, tr("Clear", "Xóa")).clicked() {
                            let _ = core_clipboard::clear_unpinned_clipboard_items();
                            self.image_textures.clear();
                            self.reload();
                        }
                    });
                });

                // Search input
                ui.horizontal(|ui| {
                    let search_resp = ui.add_sized(
                        [ui.available_width(), 28.0],
                        egui::TextEdit::singleline(&mut self.query).hint_text(tr(
                            "🔍 Search clipboard history (↑/↓/Enter to Paste)...",
                            "🔍 Tìm lịch sử Clipboard (↑/↓/Enter để Dán)...",
                        )),
                    );
                    if self.request_search_focus {
                        search_resp.request_focus();
                        self.request_search_focus = false;
                    }
                    if search_resp.changed() {
                        self.page = 1;
                        self.selected_idx = 0;
                        self.reload();
                    }
                });

                // Filter tabs
                ui.horizontal(|ui| {
                    let filters = [
                        ("all", tr("All", "Tất cả")),
                        ("text", tr("Text", "Văn bản")),
                        ("image", tr("Images", "Hình ảnh")),
                        ("pinned", tr("📌 Pinned", "📌 Đã ghim")),
                    ];
                    for (key, lbl) in filters {
                        let active = self.filter_kind == key;
                        let btn = if active {
                            primary_button(ui, lbl)
                        } else {
                            secondary_button(ui, lbl)
                        };
                        if btn.clicked() && !active {
                            self.filter_kind = key.to_string();
                            self.page = 1;
                            self.selected_idx = 0;
                            self.reload();
                        }
                    }
                });

                ui.separator();

                // Items ScrollArea
                let mut action_paste: Option<i64> = None;
                let mut action_pin: Option<i64> = None;
                let mut action_delete: Option<i64> = None;

                ScrollArea::vertical()
                    .max_height(ui.available_height() - 36.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.page_result.items.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    RichText::new(tr(
                                        "No matching clipboard items found.",
                                        "Không có mục Clipboard nào phù hợp.",
                                    ))
                                    .size(13.0)
                                    .color(p.text_muted),
                                );
                            });
                        } else {
                            for (idx, item) in self.page_result.items.iter().enumerate() {
                                let is_sel = idx == self.selected_idx;
                                let border_col = if is_sel { p.accent } else { p.border };
                                let fill_col = if is_sel { p.bg_card_hover } else { p.bg_card };

                                egui::Frame::none()
                                    .fill(fill_col)
                                    .stroke(Stroke::new(
                                        if is_sel { 1.5_f32 } else { 1.0_f32 },
                                        border_col,
                                    ))
                                    .rounding(Rounding::same(10.0))
                                    .inner_margin(Margin::same(10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let kind_badge = if item.kind == "image" {
                                                tr("🖼 Image", "🖼 Ảnh")
                                            } else {
                                                tr("📝 Text", "📝 Văn bản")
                                            };
                                            badge_pill(
                                                ui,
                                                kind_badge,
                                                p.accent_text,
                                                p.accent_soft,
                                            );
                                            if !item.source_app.is_empty() {
                                                ui.label(
                                                    RichText::new(sanitize_ui_text(
                                                        &item.source_app,
                                                    ))
                                                    .size(11.0)
                                                    .color(p.text_muted),
                                                );
                                            }
                                            ui.label(
                                                RichText::new(format_bytes(
                                                    item.byte_size.max(0) as u64,
                                                ))
                                                .size(11.0)
                                                .color(p.text_muted),
                                            );

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if danger_button(ui, "🗑").clicked() {
                                                        action_delete = Some(item.id);
                                                    }
                                                    let pin_lbl =
                                                        if item.is_pinned { "📌" } else { "📍" };
                                                    if secondary_button(ui, pin_lbl).clicked() {
                                                        action_pin = Some(item.id);
                                                    }
                                                    if primary_button(
                                                        ui,
                                                        tr("Paste ↵", "Dán ↵"),
                                                    )
                                                    .clicked()
                                                    {
                                                        action_paste = Some(item.id);
                                                    }
                                                },
                                            );
                                        });

                                        if item.kind == "image" {
                                            if self.image_textures.len() > 30 {
                                                self.image_textures.clear();
                                            }
                                            let tex = self
                                                .image_textures
                                                .entry(item.id)
                                                .or_insert_with(|| {
                                                    let src = if !item.image_path.is_empty() {
                                                        &item.image_path
                                                    } else {
                                                        &item.image_data_url
                                                    };
                                                    if let Some((w, h, rgba)) =
                                                        decode_data_url_or_path_to_rgba8(src)
                                                    {
                                                        let img = ColorImage::from_rgba_unmultiplied(
                                                            [w, h],
                                                            &rgba,
                                                        );
                                                        ctx.load_texture(
                                                            format!("clip_img_{}", item.id),
                                                            img,
                                                            TextureOptions::LINEAR,
                                                        )
                                                    } else {
                                                        let empty = ColorImage::new(
                                                            [1, 1],
                                                            Color32::TRANSPARENT,
                                                        );
                                                        ctx.load_texture(
                                                            format!("clip_empty_{}", item.id),
                                                            empty,
                                                            TextureOptions::NEAREST,
                                                        )
                                                    }
                                                });
                                            let sz = tex.size_vec2();
                                            if sz.x > 1.0 && sz.y > 1.0 {
                                                let max_w = (ui.available_width() - 8.0).max(120.0);
                                                let max_h = 110.0;
                                                let scale =
                                                    (max_w / sz.x).min(max_h / sz.y).min(1.0);
                                                let resp = ui.add(
                                                    egui::Image::new((
                                                        tex.id(),
                                                        egui::vec2(sz.x * scale, sz.y * scale),
                                                    ))
                                                    .rounding(Rounding::same(6.0))
                                                    .sense(egui::Sense::click()),
                                                );
                                                if resp.clicked() {
                                                    self.selected_idx = idx;
                                                    action_paste = Some(item.id);
                                                }
                                            }
                                        } else {
                                            let preview_raw: String =
                                                item.preview_text.chars().take(220).collect();
                                            let preview = sanitize_ui_text(&preview_raw);
                                            crate::gpu_ui::ensure_cjk_font_if_needed(ctx, &preview);
                                            let resp = ui.add(
                                                egui::Label::new(
                                                    RichText::new(preview)
                                                        .size(12.5)
                                                        .color(p.text_primary),
                                                )
                                                .sense(egui::Sense::click()),
                                            );
                                            if resp.clicked() {
                                                self.selected_idx = idx;
                                                action_paste = Some(item.id);
                                            }
                                        }
                                    });
                                ui.add_space(4.0);
                            }
                        }
                    });

                if let Some(id) = action_pin {
                    let _ = core_clipboard::toggle_pin_clipboard_item(id);
                    self.reload();
                }
                if let Some(id) = action_delete {
                    let _ = core_clipboard::delete_clipboard_item(id);
                    self.image_textures.remove(&id);
                    self.reload();
                }
                if let Some(id) = action_paste {
                    let _ = proxy.send_event(AppUserEvent::HideClipboardDrawer);
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(45));
                        let _ = core_clipboard::activate_clipboard_item(id, true);
                    });
                }

                // Footer Pagination
                ui.horizontal(|ui| {
                    let can_prev = self.page > 1;
                    let can_next = self.page < self.page_result.total_pages;
                    if ui
                        .add_enabled(can_prev, egui::Button::new(tr("◀ Prev", "◀ Trước")))
                        .clicked()
                    {
                        self.page = self.page.saturating_sub(1).max(1);
                        self.selected_idx = 0;
                        self.reload();
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} {} / {}",
                            tr("Page", "Trang"),
                            self.page_result.page,
                            self.page_result.total_pages
                        ))
                        .size(11.5)
                        .color(p.text_secondary),
                    );
                    if ui
                        .add_enabled(can_next, egui::Button::new(tr("Next ▶", "Tiếp ▶")))
                        .clicked()
                    {
                        self.page += 1;
                        self.selected_idx = 0;
                        self.reload();
                    }
                });
            });
    }
}

// ============================================================================
// 3. Independent Alt-Tab Live Preview Overlay (`alttab_window` — Option+Tab / Cmd+Tab)
// ============================================================================

pub struct AltTabOverlayState {
    pub windows: Vec<core_window::OpenWindowItem>,
    pub selected_idx: usize,
    pub preview_textures: HashMap<u32, TextureHandle>,
}

impl AltTabOverlayState {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            selected_idx: 0,
            preview_textures: HashMap::new(),
        }
    }

    pub fn refresh_on_open(&mut self) {
        self.preview_textures.clear();
        self.windows = core_window::list_open_windows(true);
        self.selected_idx = if self.windows.len() > 1 { 1 } else { 0 };
    }

    pub fn cycle_next(&mut self) {
        if !self.windows.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.windows.len();
        }
    }

    pub fn commit_selection(&mut self, proxy: &EventLoopProxy<AppUserEvent>) {
        if let Some(w) = self.windows.get(self.selected_idx).cloned() {
            let _ = core_window::focus_window(w.window_id, w.owner_pid);
        }
        let _ = proxy.send_event(AppUserEvent::HideAltTabOverlay);
    }

    pub fn on_hidden(&mut self) {
        self.preview_textures.clear();
        self.windows.clear();
    }

    pub fn render(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<AppUserEvent>) {
        let mut commit_now = false;
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                let _ = proxy.send_event(AppUserEvent::HideAltTabOverlay);
            } else if i.key_pressed(Key::Tab) {
                self.cycle_next();
            } else if i.key_pressed(Key::ArrowRight) && !self.windows.is_empty() {
                self.selected_idx = (self.selected_idx + 1) % self.windows.len();
            } else if i.key_pressed(Key::ArrowLeft) && !self.windows.is_empty() {
                if self.selected_idx == 0 {
                    self.selected_idx = self.windows.len() - 1;
                } else {
                    self.selected_idx -= 1;
                }
            } else if i.key_pressed(Key::Enter) {
                commit_now = true;
            }
        });

        if commit_now {
            self.commit_selection(proxy);
            return;
        }

        let p = palette();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_primary)
                    .stroke(Stroke::new(1.5_f32, p.border))
                    .rounding(Rounding::same(14.0))
                    .inner_margin(Margin::same(14.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(
                            "❐ Window Switcher (Alt-Tab Live Preview)",
                            "❐ Chuyển đổi Cửa sổ độc lập (Alt-Tab Live Preview)",
                        ))
                        .size(14.5)
                        .strong()
                        .color(p.text_primary),
                    );
                    badge_pill(
                        ui,
                        &format!("{} {}", self.windows.len(), tr("Windows", "Cửa sổ")),
                        p.accent_text,
                        p.accent_soft,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(ui, "✕ Esc").clicked() {
                            let _ = proxy.send_event(AppUserEvent::HideAltTabOverlay);
                        }
                    });
                });
                ui.separator();

                if self.windows.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(120.0);
                        ui.label(
                            RichText::new(tr(
                                "No open application windows on this Space.",
                                "Không có cửa sổ ứng dụng nào đang mở.",
                            ))
                            .size(14.0)
                            .color(p.text_muted),
                        );
                    });
                    return;
                }

                let mut clicked_win: Option<(u32, i32)> = None;

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let cols = 3usize;
                        let card_w = ((ui.available_width() - 24.0) / cols as f32).max(240.0);

                        for (row_idx, chunk) in self.windows.chunks(cols).enumerate() {
                            ui.horizontal(|ui| {
                                for (col_idx, w) in chunk.iter().enumerate() {
                                    let idx = row_idx * cols + col_idx;
                                    let is_sel = idx == self.selected_idx;
                                    let stroke_col = if is_sel { p.accent } else { p.border };
                                    let bg_col = if is_sel { p.bg_card_hover } else { p.bg_card };

                                    let resp = egui::Frame::none()
                                        .fill(bg_col)
                                        .stroke(Stroke::new(
                                            if is_sel { 2.5_f32 } else { 1.0_f32 },
                                            stroke_col,
                                        ))
                                        .rounding(Rounding::same(10.0))
                                        .inner_margin(Margin::same(10.0))
                                        .show(ui, |ui| {
                                            ui.set_width(card_w - 20.0);
                                            ui.horizontal(|ui| {
                                                badge_pill(
                                                    ui,
                                                    &w.app_name,
                                                    p.accent_text,
                                                    p.accent_soft,
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{}×{}",
                                                                w.width, w.height
                                                            ))
                                                            .size(10.5)
                                                            .color(p.text_muted),
                                                        );
                                                    },
                                                );
                                            });

                                            let title_short: String =
                                                w.title.chars().take(42).collect();
                                            ui.label(
                                                RichText::new(title_short)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(p.text_primary),
                                            );

                                            // Thumbnail Preview
                                            let thumb_h = 128.0;
                                            if !w.preview_data_url.is_empty() {
                                                let tex = self
                                                    .preview_textures
                                                    .entry(w.window_id)
                                                    .or_insert_with(|| {
                                                        if let Some((iw, ih, rgba)) =
                                                            decode_data_url_or_path_to_rgba8(
                                                                &w.preview_data_url,
                                                            )
                                                        {
                                                            let img =
                                                                ColorImage::from_rgba_unmultiplied(
                                                                    [iw, ih],
                                                                    &rgba,
                                                                );
                                                            ctx.load_texture(
                                                                format!("alttab_{}", w.window_id),
                                                                img,
                                                                TextureOptions::LINEAR,
                                                            )
                                                        } else {
                                                            let empty = ColorImage::new(
                                                                [1, 1],
                                                                Color32::TRANSPARENT,
                                                            );
                                                            ctx.load_texture(
                                                                format!(
                                                                    "alttab_empty_{}",
                                                                    w.window_id
                                                                ),
                                                                empty,
                                                                TextureOptions::NEAREST,
                                                            )
                                                        }
                                                    });
                                                let sz = tex.size_vec2();
                                                if sz.x > 1.0 && sz.y > 1.0 {
                                                    let max_w = (card_w - 24.0).max(160.0);
                                                    let scale = (max_w / sz.x)
                                                        .min(thumb_h / sz.y)
                                                        .min(1.0);
                                                    ui.vertical_centered(|ui| {
                                                        ui.add(
                                                            egui::Image::new((
                                                                tex.id(),
                                                                egui::vec2(
                                                                    sz.x * scale,
                                                                    sz.y * scale,
                                                                ),
                                                            ))
                                                            .rounding(Rounding::same(6.0)),
                                                        );
                                                    });
                                                }
                                            } else {
                                                let (rect, _) = ui.allocate_exact_size(
                                                    egui::vec2(card_w - 24.0, thumb_h),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().rect_filled(
                                                    rect,
                                                    Rounding::same(6.0),
                                                    p.bg_input,
                                                );
                                                ui.painter().text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    &w.app_name,
                                                    egui::FontId::proportional(14.0),
                                                    p.text_muted,
                                                );
                                            }
                                        })
                                        .response
                                        .interact(egui::Sense::click());

                                    if resp.hovered() {
                                        self.selected_idx = idx;
                                    }
                                    if resp.clicked() {
                                        clicked_win = Some((w.window_id, w.owner_pid));
                                    }
                                }
                            });
                            ui.add_space(6.0);
                        }
                    });

                if let Some((wid, pid)) = clicked_win {
                    let _ = core_window::focus_window(wid, pid);
                    let _ = proxy.send_event(AppUserEvent::HideAltTabOverlay);
                }
            });
    }
}
