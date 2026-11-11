use crate::events::AppUserEvent;
use crate::gpu_ui::dashboard::{DashboardState, DashboardTab, SettingsSubTab, ShortcutRecordTarget};
use crate::gpu_ui::{
    badge_pill, card_frame, label_with_tip, palette, pill_tab_button, pill_tabs_frame,
    primary_button, sanitize_ui_text, secondary_button, setting_row_split,
    shortcut_recorder_button, sub_card_frame, thin_divider, toggle_switch,
};
use crate::i18n::{
    is_light_theme, is_vi, set_lang, set_theme, tr, translate_chord_action, translate_menubar_mode,
    translate_mouse_action, translate_single_space_fallback, CHORD_ACTION_OPTIONS,
    MENUBAR_MODE_OPTIONS, MOUSE_ACTION_OPTIONS, SINGLE_SPACE_FALLBACK_OPTIONS,
};
use crate::updater::CURRENT_VERSION;
use egui::{ComboBox, FontId, Margin, RichText, Rounding, Stroke};
use egui_winit::winit::event_loop::EventLoopProxy;

impl DashboardState {
    pub(super) fn render_tab_settings(
        &mut self,
        ui: &mut egui::Ui,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        let p = palette();
        let avail_w = ui.available_width();

        // 1. Page Header
        setting_row_split(
            ui,
            |ui| {
                if secondary_button(ui, "⭐ GitHub (@boykhacthang)").clicked() {
                    let _ = std::process::Command::new("open")
                        .arg("https://github.com/boykhacthang/mac-omni-tool")
                        .spawn();
                }
                if secondary_button(ui, tr(" About", " Giới thiệu")).clicked() {
                    self.show_about_modal = true;
                }
            },
            |ui| {
                label_with_tip(
                    ui,
                    RichText::new(tr(
                        "Settings, Shortcuts & System Permissions",
                        "Cài đặt, Phím tắt & Quyền Hệ thống",
                    ))
                    .size(16.5)
                    .strong()
                    .color(p.text_primary),
                    tr(
                        "Configure global shortcuts, Menu Bar status item, background mode, Launch at Login, and macOS TCC permissions.",
                        "Quản lý tập trung toàn bộ phím tắt, thanh Menu Bar, tự khởi động cùng macOS và quyền hệ thống.",
                    ),
                );
            },
        );

        ui.add_space(6.0);

        // 2. Sub-tab Bar: Shortcuts & Features | System, Permissions & About
        let show_full_quick_btns = avail_w >= 820.0;
        let is_shortcuts_subtab = self.settings_subtab == SettingsSubTab::Shortcuts;
        let all_perms_ok = self.permissions.accessibility_granted
            && self.permissions.screen_recording_granted;
        setting_row_split(
            ui,
            |ui| {
                if is_shortcuts_subtab {
                    if secondary_button(
                        ui,
                        if show_full_quick_btns {
                            tr("📋 Open Clipboard", "📋 Mở Clipboard")
                        } else {
                            "📋 Clipboard"
                        },
                    )
                    .clicked()
                    {
                        let _ = proxy.send_event(AppUserEvent::ToggleClipboardDrawer);
                    }
                    if show_full_quick_btns {
                        if secondary_button(ui, tr("⚡ Full + Copy", "⚡ Chụp + Copy")).clicked() {
                            let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                                display_id: 0,
                                instant_copy: true,
                            });
                        }
                        if secondary_button(ui, tr("🖼 Full + Edit", "🖼 Chụp màn + Sửa"))
                            .clicked()
                        {
                            let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                                display_id: 0,
                                instant_copy: false,
                            });
                        }
                    }
                    if primary_button(ui, tr("✂ Capture Region", "✂ Chụp vùng")).clicked() {
                        let _ = proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
                    }
                } else if all_perms_ok {
                    badge_pill(
                        ui,
                        tr("✓ All Permissions Granted", "✓ Đã cấp đủ quyền"),
                        p.success,
                        p.success_soft,
                    );
                } else {
                    badge_pill(
                        ui,
                        tr("⚠ Missing Permissions", "⚠ Thiếu quyền hệ thống"),
                        p.warning,
                        p.warning_soft,
                    );
                }
            },
            |ui| {
                pill_tabs_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if pill_tab_button(
                            ui,
                            self.settings_subtab == SettingsSubTab::Shortcuts,
                            tr("⌨ Shortcuts & Features", "⌨ Phím tắt & Tính năng"),
                        )
                        .clicked()
                        {
                            self.settings_subtab = SettingsSubTab::Shortcuts;
                        }
                        if pill_tab_button(
                            ui,
                            self.settings_subtab == SettingsSubTab::System,
                            tr(
                                "🛡 System, Permissions & About",
                                "🛡 Hệ thống, Quyền & Giới thiệu",
                            ),
                        )
                        .clicked()
                        {
                            self.settings_subtab = SettingsSubTab::System;
                        }
                    });
                });
            },
        );

        thin_divider(ui);
        ui.add_space(4.0);

        if self.settings_subtab == SettingsSubTab::Shortcuts {
            // Card 1: Menu Bar & Background Mode (Responsive 2-Column Grid + Live Menu Bar Preview Badge)
            let mut mb_changed = false;
            card_frame().show(ui, |ui| {
                let icon_prefix = if self.input_cfg.menubar_icon == "none"
                    || self.input_cfg.menubar_icon.is_empty()
                {
                    String::new()
                } else {
                    format!("{} ", sanitize_ui_text(&self.input_cfg.menubar_icon))
                };
                let preview_text = if !self.input_cfg.menubar_enabled
                    || self.input_cfg.menubar_display_mode == "hidden"
                {
                    tr(
                        "Hidden (Running silently)",
                        "Đang ẩn trên Menu Bar",
                    )
                    .to_string()
                } else {
                    match self.input_cfg.menubar_display_mode.as_str() {
                        "temp_cpu" => format!("{}45°C • 12%", icon_prefix),
                        "temp_cpu_ram" => format!("{}45°C • 12% • 54%", icon_prefix),
                        "cpu_ram" => format!("{}CPU 12% • RAM 54%", icon_prefix),
                        "temp_only" => format!("{}45°C", icon_prefix),
                        "cpu_only" => format!("{}12%", icon_prefix),
                        "ram_only" => format!("{}54%", icon_prefix),
                        "ram_gb" => format!("{}8.6/16GB", icon_prefix),
                        "net_speed" => format!("{}↓1.2M ↑120K", icon_prefix),
                        "icon_only" => sanitize_ui_text(if self.input_cfg.menubar_icon == "none" {
                            "❖"
                        } else {
                            &self.input_cfg.menubar_icon
                        }),
                        "icon_name" => format!("{}Omni", icon_prefix),
                        "custom" => {
                            let mut parts = Vec::new();
                            if self.input_cfg.menubar_show_temp {
                                parts.push("45°C");
                            }
                            if self.input_cfg.menubar_show_cpu {
                                parts.push("12%");
                            }
                            if self.input_cfg.menubar_show_ram {
                                parts.push("54%");
                            }
                            if self.input_cfg.menubar_show_net {
                                parts.push("↓1.2M");
                            }
                            if parts.is_empty() {
                                format!("{}Omni", icon_prefix)
                            } else {
                                format!("{}{}", icon_prefix, parts.join(" • "))
                            }
                        }
                        _ => format!("{}45°C • 12%", icon_prefix),
                    }
                };

                setting_row_split(
                    ui,
                    |ui| {
                        egui::Frame::none()
                            .fill(p.bg_input)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::symmetric(10.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&preview_text)
                                        .font(FontId::monospace(11.5))
                                        .strong()
                                        .color(p.text_primary),
                                );
                            });
                        ui.label(
                            RichText::new(tr("Preview:", "Xem trước:"))
                                .size(11.0)
                                .color(p.text_muted),
                        );
                    },
                    |ui| {
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "❖ Menu Bar (Status Bar) & Background Mode",
                                "❖ Thanh Menu Bar (Status Bar) & Chạy ngầm",
                            ))
                            .size(14.0)
                            .strong(),
                            tr(
                                "Customize the icon and live hardware telemetry displayed next to the macOS clock, or hide the icon completely while keeping hotkeys active.",
                                "Tùy chỉnh biểu tượng, thông số hiển thị trên thanh Menu Bar góc phải trên màn hình (cạnh Đồng hồ), hoặc ẩn hoàn toàn.",
                            ),
                        );
                    },
                );

                thin_divider(ui);

                if ui.available_width() >= 700.0 {
                    ui.columns(2, |cols| {
                        self.render_menubar_left_rows(&mut cols[0], &mut mb_changed);
                        self.render_menubar_right_rows(&mut cols[1], &mut mb_changed);
                    });
                } else {
                    self.render_menubar_left_rows(ui, &mut mb_changed);
                    thin_divider(ui);
                    self.render_menubar_right_rows(ui, &mut mb_changed);
                }

                if self.input_cfg.menubar_display_mode == "custom" {
                    thin_divider(ui);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(tr("Custom metrics:", "Chọn thành phần:"))
                                .size(11.5)
                                .strong()
                                .color(p.accent_text),
                        );
                        if ui
                            .checkbox(
                                &mut self.input_cfg.menubar_show_temp,
                                tr("🌡 Temp (°C)", "🌡 Nhiệt độ (°C)"),
                            )
                            .changed()
                        {
                            mb_changed = true;
                        }
                        if ui
                            .checkbox(&mut self.input_cfg.menubar_show_cpu, "⚡ % CPU")
                            .changed()
                        {
                            mb_changed = true;
                        }
                        if ui
                            .checkbox(&mut self.input_cfg.menubar_show_ram, "💾 % RAM")
                            .changed()
                        {
                            mb_changed = true;
                        }
                        if ui
                            .checkbox(
                                &mut self.input_cfg.menubar_show_net,
                                tr("🌐 Net Speed", "🌐 Tốc độ mạng"),
                            )
                            .changed()
                        {
                            mb_changed = true;
                        }
                    });
                }
            });

            if mb_changed {
                self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
                let _ = proxy.send_event(AppUserEvent::SyncMenuBarConfig);
            }

            ui.add_space(8.0);

            // Row 2: Responsive Layout for Features & Global Shortcuts | External Mouse, Scroll & F3
            // Uses 2 columns on wide windows (>= 920px available width) and stacks vertically on compact windows.
            let mut feat_changed = false;
            let mut jump_to_mouse = false;

            if ui.available_width() >= 920.0 {
                ui.columns(2, |cols| {
                    self.render_settings_shortcuts_card(&mut cols[0], proxy, &mut feat_changed);
                    self.render_settings_mouse_card(
                        &mut cols[1],
                        &mut feat_changed,
                        &mut jump_to_mouse,
                    );
                });
            } else {
                self.render_settings_shortcuts_card(ui, proxy, &mut feat_changed);
                ui.add_space(8.0);
                self.render_settings_mouse_card(ui, &mut feat_changed, &mut jump_to_mouse);
            }

            if jump_to_mouse {
                self.switch_tab(DashboardTab::Mouse);
            }

            if feat_changed {
                self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
                let _ = proxy.send_event(AppUserEvent::SyncMenuBarConfig);
            }
        } else {
            // Sub-tab 2: System, Permissions & About (`SettingsSubTab::System`)
            if ui.available_width() >= 880.0 {
                ui.columns(2, |cols| {
                    self.render_settings_autostart_card(&mut cols[0], proxy);
                    self.render_settings_permissions_card(&mut cols[1]);
                });
            } else {
                self.render_settings_autostart_card(ui, proxy);
                ui.add_space(8.0);
                self.render_settings_permissions_card(ui);
            }

            ui.add_space(8.0);

            // Bottom Card: About Application, Author & Free Community Edition (Ko-fi)
            card_frame().show(ui, |ui| {
                if ui.available_width() >= 760.0 {
                    ui.columns(2, |cols| {
                        self.render_settings_about_left(&mut cols[0]);
                        self.render_settings_about_right(&mut cols[1]);
                    });
                } else {
                    self.render_settings_about_left(ui);
                    ui.add_space(8.0);
                    self.render_settings_about_right(ui);
                }
            });
        }
    }

    fn render_menubar_left_rows(&mut self, ui: &mut egui::Ui, mb_changed: &mut bool) {
        // 1. Show on Menu Bar
        setting_row_split(
            ui,
            |ui| {
                if toggle_switch(ui, &mut self.input_cfg.menubar_enabled).changed() {
                    if self.input_cfg.menubar_enabled
                        && self.input_cfg.menubar_display_mode == "hidden"
                    {
                        self.input_cfg.menubar_display_mode = "temp_cpu".to_string();
                    } else if !self.input_cfg.menubar_enabled {
                        self.input_cfg.menubar_display_mode = "hidden".to_string();
                    }
                    *mb_changed = true;
                }
            },
            |ui| {
                label_with_tip(
                    ui,
                    RichText::new(tr(
                        "Show on macOS Menu Bar",
                        "Hiển thị trên thanh Menu Bar",
                    ))
                    .size(12.5)
                    .strong(),
                    tr(
                        "Turn off to hide the Menu Bar icon while keeping background shortcuts active.",
                        "Tắt để ẩn hoàn toàn icon trên thanh Menu Bar (cạnh Đồng hồ) nhưng vẫn chạy ngầm.",
                    ),
                );
            },
        );

        thin_divider(ui);

        // 2. Menu Bar Telemetry Mode
        let combo_w = (ui.available_width() * 0.42).clamp(130.0, 195.0);
        setting_row_split(
            ui,
            |ui| {
                ComboBox::from_id_salt("settings_mb_mode_combo")
                    .width(combo_w)
                    .truncate()
                    .selected_text(translate_menubar_mode(
                        &self.input_cfg.menubar_display_mode,
                    ))
                    .show_ui(ui, |ui| {
                        ui.set_min_width(250.0);
                        for &k in MENUBAR_MODE_OPTIONS {
                            if ui
                                .selectable_value(
                                    &mut self.input_cfg.menubar_display_mode,
                                    k.to_string(),
                                    translate_menubar_mode(k),
                                )
                                .changed()
                            {
                                self.input_cfg.menubar_enabled = k != "hidden";
                                *mb_changed = true;
                            }
                        }
                    });
            },
            |ui| {
                label_with_tip(
                    ui,
                    RichText::new(tr(
                        "Menu Bar Telemetry Mode",
                        "Chế độ nội dung Menu Bar",
                    ))
                    .size(12.5)
                    .strong(),
                    tr(
                        "Select which hardware metrics to display next to the Menu Bar icon.",
                        "Chọn thông số muốn theo dõi nhanh trên thanh Menu Bar.",
                    ),
                );
            },
        );
    }

    fn render_menubar_right_rows(&mut self, ui: &mut egui::Ui, mb_changed: &mut bool) {
        // 1. Menu Bar Status Icon
        let combo_w = (ui.available_width() * 0.40).clamp(125.0, 175.0);
        setting_row_split(
            ui,
            |ui| {
                let icons = [
                    ("❖", tr("❖ Diamond (Omni)", "❖ Kim cương (Omni)")),
                    ("⌘", "⌘ Command"),
                    ("⚡", tr("⚡ Bolt", "⚡ Tia chớp")),
                    ("🖥", tr("🖥 Display", "🖥 Màn hình")),
                    ("🎛", tr("🎛 Sliders", "🎛 Điều khiển")),
                    ("●", tr("● Dot", "● Chấm tròn")),
                    ("none", tr("No Icon (Text Only)", "Không hiện icon")),
                ];
                ComboBox::from_id_salt("settings_mb_icon_combo")
                    .width(combo_w)
                    .truncate()
                    .selected_text(
                        icons
                            .iter()
                            .find(|(k, _)| {
                                sanitize_ui_text(k)
                                    == sanitize_ui_text(&self.input_cfg.menubar_icon)
                            })
                            .map(|(_, v)| *v)
                            .unwrap_or("❖ Diamond"),
                    )
                    .show_ui(ui, |ui| {
                        ui.set_min_width(200.0);
                        for (k, lbl) in icons {
                            if ui
                                .selectable_value(
                                    &mut self.input_cfg.menubar_icon,
                                    k.to_string(),
                                    lbl,
                                )
                                .changed()
                            {
                                *mb_changed = true;
                            }
                        }
                    });
            },
            |ui| {
                label_with_tip(
                    ui,
                    RichText::new(tr("Menu Bar Status Icon", "Biểu tượng trên Menu Bar"))
                        .size(12.5)
                        .strong(),
                    tr(
                        "Prefix symbol displayed before telemetry text on the macOS Menu Bar.",
                        "Ký hiệu biểu tượng đứng trước thông số trên thanh Menu Bar.",
                    ),
                );
            },
        );

        thin_divider(ui);

        // 2. Keep Running When Closing Window
        setting_row_split(
            ui,
            |ui| {
                if toggle_switch(ui, &mut self.input_cfg.keep_running_in_background).changed() {
                    *mb_changed = true;
                }
            },
            |ui| {
                label_with_tip(
                    ui,
                    RichText::new(tr(
                        "Keep Running When Closing Window (🔴)",
                        "Thu về chạy ngầm khi bấm Đóng (🔴)",
                    ))
                    .size(12.5)
                    .strong(),
                    tr(
                        "Closing the main window hides the app and suspends GPU memory (~26 MB RAM) while keeping all hotkeys, Mouse & Clipboard active.",
                        "Đóng cửa sổ chính sẽ ẩn App và giải phóng GPU (~26 MB RAM), giữ nguyên mọi phím tắt & chuột rời.",
                    ),
                );
            },
        );
    }

    fn render_settings_shortcuts_card(
        &mut self,
        ui: &mut egui::Ui,
        proxy: &EventLoopProxy<AppUserEvent>,
        feat_changed: &mut bool,
    ) {
        card_frame().show(ui, |ui| {
            label_with_tip(
                ui,
                RichText::new(tr(
                    "🚀 Features & Global Shortcuts",
                    "🚀 Tính năng & Phím tắt Toàn cục",
                ))
                .size(14.0)
                .strong(),
                tr(
                    "Click any shortcut pill and press a new key combination on your keyboard (Press Esc to cancel).",
                    "Nhấp vào ô phím tắt bất kỳ rồi nhấn tổ hợp phím mới trên bàn phím để thay đổi (Nhấn Esc để hủy).",
                ),
            );
            thin_divider(ui);

            // 1. Toggle Main Dashboard
            setting_row_split(
                ui,
                |ui| {
                    let is_rec = matches!(
                        &self.recording_shortcut,
                        Some(ShortcutRecordTarget::ConfigField(f)) if f == "dashboard_shortcut"
                    );
                    if shortcut_recorder_button(ui, &self.input_cfg.dashboard_shortcut, is_rec)
                        .clicked()
                    {
                        self.recording_shortcut = Some(ShortcutRecordTarget::ConfigField(
                            "dashboard_shortcut".to_string(),
                        ));
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Toggle Main Window (Dashboard)",
                            "Mở / Ẩn Cửa sổ Chính (Dashboard)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Global shortcut to summon or hide the main Mac Omni Tool window anytime, even when the Menu Bar icon is hidden.",
                            "Gọi nhanh cửa sổ chính bất kỳ lúc nào ngay cả khi đã ẩn biểu tượng trên thanh Menu Bar.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 2. Screenshot Mode 1: Region Capture
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.screenshot_enabled).changed() {
                        *feat_changed = true;
                    }
                    let is_rec = matches!(
                        &self.recording_shortcut,
                        Some(ShortcutRecordTarget::ConfigField(f)) if f == "screenshot_shortcut"
                    );
                    if shortcut_recorder_button(ui, &self.input_cfg.screenshot_shortcut, is_rec)
                        .clicked()
                    {
                        self.recording_shortcut = Some(ShortcutRecordTarget::ConfigField(
                            "screenshot_shortcut".to_string(),
                        ));
                    }
                    if secondary_button(ui, tr("✂ Test", "✂ Thử")).clicked() {
                        let _ = proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Region Screenshot (Multi-Monitor)",
                            "Chụp màn hình Vùng chọn (Đa màn hình)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Drag a custom region on any display and annotate with arrows, rectangles, text, or pixelate blur.",
                            "Kéo chọn vùng bất kỳ trên mọi màn hình, hỗ trợ vẽ mũi tên, khung chữ nhật, viết chữ và làm mờ.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 3. Screenshot Mode 2: Fullscreen + Edit
            setting_row_split(
                ui,
                |ui| {
                    let is_rec = matches!(
                        &self.recording_shortcut,
                        Some(ShortcutRecordTarget::ConfigField(f)) if f == "screenshot_fullscreen_edit_shortcut"
                    );
                    if shortcut_recorder_button(
                        ui,
                        &self.input_cfg.screenshot_fullscreen_edit_shortcut,
                        is_rec,
                    )
                    .clicked()
                    {
                        self.recording_shortcut = Some(ShortcutRecordTarget::ConfigField(
                            "screenshot_fullscreen_edit_shortcut".to_string(),
                        ));
                    }
                    if secondary_button(ui, tr("🖼 Test", "🖼 Thử")).clicked() {
                        let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                            display_id: 0,
                            instant_copy: false,
                        });
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Fullscreen Screenshot + Annotate",
                            "Chụp Toàn màn hình + Chú thích",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Captures the entire active display and opens the annotation toolbar immediately.",
                            "Chụp toàn bộ màn hình tại vị trí chuột và mở ngay thanh công cụ vẽ chú thích.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 4. Screenshot Mode 3: Fullscreen + Instant Copy
            setting_row_split(
                ui,
                |ui| {
                    let is_rec = matches!(
                        &self.recording_shortcut,
                        Some(ShortcutRecordTarget::ConfigField(f)) if f == "screenshot_fullscreen_copy_shortcut"
                    );
                    if shortcut_recorder_button(
                        ui,
                        &self.input_cfg.screenshot_fullscreen_copy_shortcut,
                        is_rec,
                    )
                    .clicked()
                    {
                        self.recording_shortcut = Some(ShortcutRecordTarget::ConfigField(
                            "screenshot_fullscreen_copy_shortcut".to_string(),
                        ));
                    }
                    if secondary_button(ui, tr("⚡ Test", "⚡ Thử")).clicked() {
                        let _ = proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                            display_id: 0,
                            instant_copy: true,
                        });
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Fullscreen Screenshot + Instant Copy",
                            "Chụp Toàn màn hình + Copy ngay",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Captures the active display and copies the PNG image directly into your Clipboard.",
                            "Chụp tức thì toàn màn hình đang làm việc và lưu thẳng ảnh PNG vào Clipboard.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 5. Clipboard Drawer
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.clipboard_enabled).changed() {
                        core_clipboard::set_clipboard_enabled(self.input_cfg.clipboard_enabled);
                        *feat_changed = true;
                    }
                    let is_rec = matches!(
                        &self.recording_shortcut,
                        Some(ShortcutRecordTarget::ConfigField(f)) if f == "clipboard_popup_shortcut"
                    );
                    if shortcut_recorder_button(
                        ui,
                        &self.input_cfg.clipboard_popup_shortcut,
                        is_rec,
                    )
                    .clicked()
                    {
                        self.recording_shortcut = Some(ShortcutRecordTarget::ConfigField(
                            "clipboard_popup_shortcut".to_string(),
                        ));
                    }
                    if secondary_button(ui, tr("📋 Open", "📋 Mở")).clicked() {
                        let _ = proxy.send_event(AppUserEvent::ToggleClipboardDrawer);
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Clipboard History Drawer (1,000 items)",
                            "Ngăn kéo Lịch sử Clipboard (1,000 mục)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Slide-out left drawer with instant search, pinned items, and click-to-paste.",
                            "Ngăn kéo trượt bên trái màn hình giúp tìm kiếm và dán nhanh văn bản/hình ảnh đã copy.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 6. Alt-Tab Live Preview
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.replace_macos_cmd_tab).changed() {
                        *feat_changed = true;
                    }
                    let is_rec = matches!(
                        &self.recording_shortcut,
                        Some(ShortcutRecordTarget::ConfigField(f)) if f == "alt_tab_shortcut"
                    );
                    if shortcut_recorder_button(ui, &self.input_cfg.alt_tab_shortcut, is_rec)
                        .clicked()
                    {
                        self.recording_shortcut = Some(ShortcutRecordTarget::ConfigField(
                            "alt_tab_shortcut".to_string(),
                        ));
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Alt-Tab Window Switcher (Thumbnails)",
                            "Alt-Tab (Chuyển cửa sổ kèm ảnh thu nhỏ)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Replaces Cmd+Tab / Option+Tab with a visual window switcher showing live window previews.",
                            "Thay thế Cmd+Tab mặc định bằng bộ chuyển đổi từng cửa sổ độc lập có hình thu nhỏ trực quan.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 7. Finder Cmd+X -> Cmd+V Cut & Paste
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.finder_cut_paste_enabled).changed() {
                        *feat_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Finder File Cut & Paste (Cmd+X → Cmd+V)",
                            "Cắt & Dán File trong Finder (Cmd+X → Cmd+V)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Press Cmd+X on files in Finder, then Cmd+V in destination folder to move them (automatically translates to Cmd+Option+V).",
                            "Nhấn Cmd+X trên tệp trong Finder rồi nhấn Cmd+V ở thư mục đích để di chuyển tệp tự nhiên như Windows.",
                        ),
                    );
                },
            );
        });
    }

    fn render_settings_mouse_card(
        &mut self,
        ui: &mut egui::Ui,
        feat_changed: &mut bool,
        jump_to_mouse: &mut bool,
    ) {
        card_frame().show(ui, |ui| {
            setting_row_split(
                ui,
                |ui| {
                    if secondary_button(
                        ui,
                        tr("Detailed Mouse Settings →", "Cấu hình chi tiết →"),
                    )
                    .clicked()
                    {
                        *jump_to_mouse = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "🖱 External Mouse, Scroll & F3 Key",
                            "🖱 Chuột rời, Cuộn trang & Phím F3",
                        ))
                        .size(14.0)
                        .strong(),
                        "",
                    );
                },
            );
            thin_divider(ui);

            let row_w = ui.available_width();
            let combo_w = (row_w * 0.42).clamp(140.0, 235.0);
            let chord_combo_w = (row_w * 0.35).clamp(125.0, 190.0);

            // 1. External Mouse Master
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.external_mouse_enabled).changed() {
                        *feat_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "External Mouse Optimization (Master)",
                            "Tối ưu hóa Chuột rời (Công tắc tổng)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Master toggle for all external mouse button mappings and scroll enhancements.",
                            "Công tắc tổng bật/tắt các tính năng hỗ trợ chuột gắn ngoài.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 2. Independent Reverse Scroll
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.invert_external_mouse_scroll).changed()
                    {
                        *feat_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Reverse Scroll (External Mouse Only)",
                            "Đảo chiều cuộn riêng cho Chuột rời",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Inverts wheel scroll direction for external mice without affecting MacBook Trackpad natural gestures.",
                            "Đảo chiều con lăn chuột chuẩn Windows mà KHÔNG làm ngược chiều vuốt tự nhiên của Trackpad.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 3. 120Hz Smooth Scroll
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.smooth_scroll_enabled).changed() {
                        *feat_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "120Hz Smooth Scrolling for Mouse Wheel",
                            "Cuộn mượt 120Hz cho Chuột rời",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Eliminates notched wheel stutter on macOS with 120fps inertia interpolation.",
                            "Khử giật khấc khi lăn bánh xe chuột thông thường với nội suy quán tính 120fps.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 4. L+R Chord Click
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.mouse_chord_enabled).changed() {
                        *feat_changed = true;
                    }
                    ComboBox::from_id_salt("settings_mouse_chord_act")
                        .width(chord_combo_w)
                        .truncate()
                        .selected_text(translate_chord_action(&self.input_cfg.mouse_chord_action))
                        .show_ui(ui, |ui| {
                            ui.set_min_width(240.0);
                            for &opt in CHORD_ACTION_OPTIONS {
                                if ui
                                    .selectable_value(
                                        &mut self.input_cfg.mouse_chord_action,
                                        opt.to_string(),
                                        translate_chord_action(opt),
                                    )
                                    .changed()
                                {
                                    *feat_changed = true;
                                }
                            }
                        });
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Left + Right Chord Click",
                            "Nhấn đồng thời Chuột Trái + Phải",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Trigger Mission Control or another action by pressing Left & Right mouse buttons simultaneously.",
                            "Gọi nhanh Mission Control hoặc hành động tùy chọn khi bấm cùng lúc 2 nút chuột Trái & Phải.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 5. Default Forward Button
            setting_row_split(
                ui,
                |ui| {
                    ComboBox::from_id_salt("settings_mouse_fwd")
                        .width(combo_w)
                        .truncate()
                        .selected_text(translate_mouse_action(
                            &self.input_cfg.mouse_forward_action,
                        ))
                        .show_ui(ui, |ui| {
                            ui.set_min_width(265.0);
                            for &opt in MOUSE_ACTION_OPTIONS {
                                if ui
                                    .selectable_value(
                                        &mut self.input_cfg.mouse_forward_action,
                                        opt.to_string(),
                                        translate_mouse_action(opt),
                                    )
                                    .changed()
                                {
                                    *feat_changed = true;
                                }
                            }
                        });
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Default Forward Button (Mouse 5)",
                            "Nút Hông Trước (Mouse 5 — Tiến)",
                        ))
                        .size(12.5)
                        .strong(),
                        "",
                    );
                },
            );
            thin_divider(ui);

            // 6. Default Back Button
            setting_row_split(
                ui,
                |ui| {
                    ComboBox::from_id_salt("settings_mouse_back")
                        .width(combo_w)
                        .truncate()
                        .selected_text(translate_mouse_action(&self.input_cfg.mouse_back_action))
                        .show_ui(ui, |ui| {
                            ui.set_min_width(265.0);
                            for &opt in MOUSE_ACTION_OPTIONS {
                                if ui
                                    .selectable_value(
                                        &mut self.input_cfg.mouse_back_action,
                                        opt.to_string(),
                                        translate_mouse_action(opt),
                                    )
                                    .changed()
                                {
                                    *feat_changed = true;
                                }
                            }
                        });
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Default Back Button (Mouse 4)",
                            "Nút Hông Sau (Mouse 4 — Lùi)",
                        ))
                        .size(12.5)
                        .strong(),
                        "",
                    );
                },
            );
            thin_divider(ui);

            // 7. Single Space Fallback
            setting_row_split(
                ui,
                |ui| {
                    ComboBox::from_id_salt("settings_single_space_fb")
                        .width(combo_w)
                        .truncate()
                        .selected_text(translate_single_space_fallback(
                            &self.input_cfg.single_space_fallback,
                        ))
                        .show_ui(ui, |ui| {
                            ui.set_min_width(265.0);
                            for &opt in SINGLE_SPACE_FALLBACK_OPTIONS {
                                if ui
                                    .selectable_value(
                                        &mut self.input_cfg.single_space_fallback,
                                        opt.to_string(),
                                        translate_single_space_fallback(opt),
                                    )
                                    .changed()
                                {
                                    *feat_changed = true;
                                }
                            }
                        });
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "When Only 1 Desktop Space Is Open",
                            "Khi chỉ có 1 Màn hình (1 Space)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Fallback behavior when pressing Switch Desktop Left/Right while macOS has only 1 Desktop Space.",
                            "Hành động tự động khi nhấn nút chuyển Desktop nhưng macOS đang chỉ có 1 Màn hình.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 8. F3 Key -> Mission Control
            setting_row_split(
                ui,
                |ui| {
                    let mut f3_on = !self.input_cfg.mission_control_shortcut.trim().is_empty();
                    if toggle_switch(ui, &mut f3_on).changed() {
                        self.input_cfg.mission_control_shortcut = if f3_on {
                            "F3".to_string()
                        } else {
                            String::new()
                        };
                        *feat_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "F3 Key Opens Mission Control",
                            "Phím F3 mở Mission Control",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Ideal for Windows/PC mechanical keyboards where F3 does not trigger Mission Control natively.",
                            "Hỗ trợ bàn phím cơ Windows/PC chưa ánh xạ phím F3 sang Mission Control.",
                        ),
                    );
                },
            );
        });
    }

    fn render_settings_autostart_card(
        &mut self,
        ui: &mut egui::Ui,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        card_frame().show(ui, |ui| {
            label_with_tip(
                ui,
                RichText::new(tr(
                    "⚙ Launch at Login & Appearance",
                    "⚙ Khởi động cùng hệ thống & Ngôn ngữ",
                ))
                .size(14.0)
                .strong(),
                tr(
                    "Uses a standard macOS LaunchAgent plist in ~/Library/LaunchAgents.",
                    "Cấu hình tự động chạy Mac Omni Tool ngầm ngay khi bạn đăng nhập vào macOS.",
                ),
            );
            thin_divider(ui);

            let mut auto_en = self.autostart.enabled;
            let mut auto_min = self.autostart.start_minimized;
            let mut auto_changed = false;

            // 1. Launch at Login
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut auto_en).changed() {
                        auto_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Start Automatically on macOS Login",
                            "Tự động chạy khi khởi động macOS",
                        ))
                        .size(12.5)
                        .strong(),
                        "~/Library/LaunchAgents/com.macomnitool.app.plist",
                    );
                },
            );
            thin_divider(ui);

            // 2. Start Minimized
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut auto_min).changed() {
                        auto_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Start Hidden in Background (--minimized)",
                            "Khởi động ẩn ở chế độ chạy ngầm (--minimized)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Only shows on Menu Bar without opening main window on boot.",
                            "Chỉ hiện thanh Menu Bar, không bật cửa sổ lớn khi vừa mở máy.",
                        ),
                    );
                },
            );
            if auto_changed {
                if let Ok(st) = core_cleaner::set_autostart_config(auto_en, auto_min) {
                    self.autostart = st;
                }
            }
            thin_divider(ui);

            let combo_w = (ui.available_width() * 0.38).clamp(130.0, 175.0);

            // 3. Language
            setting_row_split(
                ui,
                |ui| {
                    ComboBox::from_id_salt("settings_lang_combo")
                        .width(combo_w)
                        .truncate()
                        .selected_text(if is_vi() {
                            "🇻🇳 Tiếng Việt"
                        } else {
                            "🇬🇧 English"
                        })
                        .show_ui(ui, |ui| {
                            ui.set_min_width(175.0);
                            if ui.selectable_label(is_vi(), "🇻🇳 Tiếng Việt").clicked()
                                && !is_vi()
                            {
                                set_lang("vi");
                                let _ = proxy.send_event(AppUserEvent::SetAppLang(
                                    "vi".to_string(),
                                ));
                            }
                            if ui.selectable_label(!is_vi(), "🇬🇧 English").clicked()
                                && is_vi()
                            {
                                set_lang("en");
                                let _ = proxy.send_event(AppUserEvent::SetAppLang(
                                    "en".to_string(),
                                ));
                            }
                        });
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("Display Language", "Ngôn ngữ hiển thị (Language)"))
                            .size(12.5)
                            .strong(),
                        tr(
                            "Instant bilingual switch across Dashboard, Menu Bar & Tray.",
                            "Chuyển đổi tức thì giữa Tiếng Việt và Tiếng Anh trên toàn bộ ứng dụng.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 4. App Theme
            setting_row_split(
                ui,
                |ui| {
                    ComboBox::from_id_salt("settings_theme_combo")
                        .width(combo_w)
                        .truncate()
                        .selected_text(if is_light_theme() {
                            tr("☀ Light Theme", "☀ Giao diện Sáng")
                        } else {
                            tr("🌙 Dark Theme", "🌙 Giao diện Tối")
                        })
                        .show_ui(ui, |ui| {
                            ui.set_min_width(185.0);
                            if ui
                                .selectable_label(
                                    !is_light_theme(),
                                    tr("🌙 Dark Theme", "🌙 Giao diện Tối"),
                                )
                                .clicked()
                                && is_light_theme()
                            {
                                set_theme("dark");
                                let _ = proxy.send_event(AppUserEvent::SetAppTheme(
                                    "dark".to_string(),
                                ));
                            }
                            if ui
                                .selectable_label(
                                    is_light_theme(),
                                    tr("☀ Light Theme", "☀ Giao diện Sáng"),
                                )
                                .clicked()
                                && !is_light_theme()
                            {
                                set_theme("light");
                                let _ = proxy.send_event(AppUserEvent::SetAppTheme(
                                    "light".to_string(),
                                ));
                            }
                        });
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("Application Theme", "Giao diện Ứng dụng (Theme)"))
                            .size(12.5)
                            .strong(),
                        tr(
                            "Switch between High-Contrast Dark and Light themes.",
                            "Chuyển đổi giữa giao diện Tối và Sáng độ tương phản cao.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 5. Auto-check updates
            setting_row_split(
                ui,
                |ui| {
                    if secondary_button(ui, tr("🔍 Check Now", "🔍 Kiểm tra")).clicked() {
                        self.trigger_update_check_async(true);
                        self.show_about_modal = true;
                    }
                    if toggle_switch(ui, &mut self.input_cfg.auto_check_updates).changed() {
                        self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Auto-check GitHub for Updates",
                            "Tự động kiểm tra cập nhật mới",
                        ))
                        .size(12.5)
                        .strong(),
                        "",
                    );
                },
            );
        });
    }

    fn render_settings_permissions_card(&mut self, ui: &mut egui::Ui) {
        let p = palette();
        card_frame().show(ui, |ui| {
            setting_row_split(
                ui,
                |ui| {
                    if secondary_button(ui, tr("↺ Reset TCC", "↺ Reset Quyền"))
                        .on_hover_text(tr(
                            "Reset stale macOS TCC entries after reinstalling or updating the app",
                            "Làm mới danh sách quyền TCC khi vừa cài đè phiên bản mới",
                        ))
                        .clicked()
                    {
                        core_window::reset_and_request_permissions();
                        core_input::notify_permissions_changed();
                        self.permissions = core_window::get_permissions_status();
                    }
                    if secondary_button(ui, tr("🔄 Re-check", "🔄 Kiểm tra lại")).clicked() {
                        core_input::notify_permissions_changed();
                        self.permissions = core_window::get_permissions_status();
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "🛡 macOS System Permissions",
                            "🛡 Quyền Hệ thống macOS (Permissions)",
                        ))
                        .size(14.0)
                        .strong(),
                        "",
                    );
                },
            );
            thin_divider(ui);

            // 1. Accessibility
            setting_row_split(
                ui,
                |ui| {
                    if !self.permissions.accessibility_granted
                        && primary_button(ui, tr("Grant", "Cấp quyền")).clicked()
                    {
                        core_window::request_or_open_permission("accessibility");
                    }
                    badge_pill(
                        ui,
                        if self.permissions.accessibility_granted {
                            tr("✓ Granted", "✓ Đã cấp")
                        } else {
                            tr("⚠ Not Granted", "⚠ Chưa cấp")
                        },
                        if self.permissions.accessibility_granted {
                            p.success
                        } else {
                            p.warning
                        },
                        if self.permissions.accessibility_granted {
                            p.success_soft
                        } else {
                            p.warning_soft
                        },
                    );
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("Accessibility", "Trợ năng (Accessibility)"))
                            .size(12.5)
                            .strong(),
                        tr(
                            "Required for Window Tiling, Alt-Tab, Global Shortcuts & External Mouse.",
                            "Cần thiết cho Chia cửa sổ, Alt-Tab, Phím tắt toàn cục và Chuột rời.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 2. Screen Recording
            setting_row_split(
                ui,
                |ui| {
                    if !self.permissions.screen_recording_granted
                        && primary_button(ui, tr("Grant", "Cấp quyền")).clicked()
                    {
                        core_window::request_or_open_permission("screen_recording");
                    }
                    badge_pill(
                        ui,
                        if self.permissions.screen_recording_granted {
                            tr("✓ Granted", "✓ Đã cấp")
                        } else {
                            tr("⚠ Not Granted", "⚠ Chưa cấp")
                        },
                        if self.permissions.screen_recording_granted {
                            p.success
                        } else {
                            p.warning
                        },
                        if self.permissions.screen_recording_granted {
                            p.success_soft
                        } else {
                            p.warning_soft
                        },
                    );
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("Screen Recording", "Ghi màn hình (Screen Recording)"))
                            .size(12.5)
                            .strong(),
                        tr(
                            "Required for Multi-monitor Screenshots & Alt-Tab window thumbnails.",
                            "Cần thiết cho Chụp ảnh màn hình đa màn hình & ảnh thu nhỏ cửa sổ Alt-Tab.",
                        ),
                    );
                },
            );
            thin_divider(ui);

            // 3. Full Disk Access (Optional)
            setting_row_split(
                ui,
                |ui| {
                    if secondary_button(ui, tr("Open Settings", "Mở Cài đặt")).clicked() {
                        core_window::request_or_open_permission("full_disk");
                    }
                    if self.permissions.full_disk_access_granted {
                        badge_pill(
                            ui,
                            tr("✓ Granted", "✓ Đã cấp"),
                            p.success,
                            p.success_soft,
                        );
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Full Disk Access (Optional)",
                            "Toàn quyền Ổ đĩa (Tùy chọn)",
                        ))
                        .size(12.5)
                        .strong(),
                        tr(
                            "Optional: enables deep system cache cleaning & emptying macOS Trash.",
                            "Tùy chọn: giúp quét rác sâu trong thư mục hệ thống & làm sạch Thùng rác.",
                        ),
                    );
                },
            );

            ui.add_space(6.0);
            sub_card_frame().show(ui, |ui| {
                ui.label(
                    RichText::new(tr(
                        "💡 Tip: If permissions stop working after installing an update, click \"↺ Reset TCC\" above to refresh macOS permission signatures.",
                        "💡 Mẹo: Nếu vừa cài đè bản mới mà tính năng Chia cửa sổ / Chuột rời chưa nhận quyền, hãy nhấn nút \"↺ Reset Quyền\" ở góc trên.",
                    ))
                    .size(11.0)
                    .color(p.text_secondary),
                );
            });
        });
    }

    fn render_settings_about_left(&self, ui: &mut egui::Ui) {
        let p = palette();
        ui.horizontal(|ui| {
            if let Some(tex) = &self.app_icon_tex {
                ui.add(
                    egui::Image::new((tex.id(), egui::vec2(42.0, 42.0)))
                        .rounding(Rounding::same(10.0)),
                );
            }
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Mac Omni Tool")
                            .size(15.5)
                            .strong()
                            .color(p.text_primary),
                    );
                    badge_pill(
                        ui,
                        &format!("v{}", CURRENT_VERSION),
                        p.accent_text,
                        p.accent_soft,
                    );
                    badge_pill(
                        ui,
                        tr("MIT License · 100% Free", "Giấy phép MIT · Miễn phí 100%"),
                        p.success,
                        p.success_soft,
                    );
                });
                ui.label(
                    RichText::new(tr(
                        "7-in-1 Native Rust + Metal GPU Utility for macOS (Apple Silicon & Intel).",
                        "Bộ công cụ All-in-One 7-trong-1 siêu nhẹ viết bằng Rust + Metal GPU cho macOS.",
                    ))
                    .size(11.5)
                    .color(p.text_secondary),
                );
            });
        });

        ui.add_space(6.0);
        ui.columns(2, |sub| {
            sub_card_frame().show(&mut sub[0], |ui| {
                ui.label(
                    RichText::new(tr("👤 Author / Developer", "👤 Tác giả / Phát triển"))
                        .size(10.5)
                        .color(p.text_muted),
                );
                ui.label(
                    RichText::new("Khắc Thắng (@boykhacthang)")
                        .size(12.0)
                        .strong(),
                );
            });
            sub_card_frame().show(&mut sub[1], |ui| {
                ui.label(
                    RichText::new(tr("⚡ Architecture", "⚡ Kiến trúc"))
                        .size(10.5)
                        .color(p.text_muted),
                );
                ui.label(
                    RichText::new("Pure Rust + Metal GPU (egui)")
                        .size(12.0)
                        .strong()
                        .color(p.accent_text),
                );
            });
        });
    }

    fn render_settings_about_right(&mut self, ui: &mut egui::Ui) {
        let p = palette();
        egui::Frame::none()
            .fill(p.accent_soft)
            .stroke(Stroke::new(1.0_f32, p.accent_border))
            .rounding(Rounding::same(9.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(tr(
                        "☕ Free Community Edition",
                        "☕ Phiên bản Miễn phí Cộng đồng (Free Community Edition)",
                    ))
                    .size(13.0)
                    .strong()
                    .color(p.text_primary),
                );
                ui.label(
                    RichText::new(tr(
                        "Every feature is 100% unlocked with zero ads or telemetry. If Mac Omni Tool helps your daily workflow, consider supporting the author!",
                        "Mọi tính năng đều được mở khóa 100%, không quảng cáo, không thu thập dữ liệu. Nếu ứng dụng hữu ích cho công việc của bạn, hãy ủng hộ tác giả nhé!",
                    ))
                    .size(11.5)
                    .color(p.text_secondary),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if primary_button(
                        ui,
                        tr("☕ Support on Ko-fi", "☕ Ủng hộ tác giả ly cà phê (Ko-fi)"),
                    )
                    .clicked()
                    {
                        let _ = std::process::Command::new("open")
                            .arg("https://ko-fi.com/I1R72A1475")
                            .spawn();
                    }
                    if secondary_button(ui, tr("⭐ Star on GitHub", "⭐ Tặng sao GitHub"))
                        .clicked()
                    {
                        let _ = std::process::Command::new("open")
                            .arg("https://github.com/boykhacthang/mac-omni-tool")
                            .spawn();
                    }
                    if secondary_button(
                        ui,
                        tr(" About & Updates", " Giới thiệu & Cập nhật"),
                    )
                    .clicked()
                    {
                        self.show_about_modal = true;
                    }
                });
            });
    }
}
