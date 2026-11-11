use crate::events::AppUserEvent;
use crate::gpu_ui::dashboard::{DashboardState, MouseSubTab};
use crate::gpu_ui::{
    badge_pill, card_frame, danger_button, draw_interactive_mouse_diagram, label_with_tip, palette,
    pill_tab_button, pill_tabs_frame, primary_button, profile_pill_button, sanitize_ui_text,
    secondary_button, setting_row_split, sub_card_frame, thin_divider, toggle_switch,
};
use crate::i18n::{
    tr, translate_chord_action, translate_mouse_action, translate_single_space_fallback,
    CHORD_ACTION_OPTIONS, MOUSE_ACTION_OPTIONS, SINGLE_SPACE_FALLBACK_OPTIONS,
};
use egui::{Color32, ComboBox, FontId, Margin, RichText, Rounding, ScrollArea, Stroke};
use egui_winit::winit::event_loop::EventLoopProxy;
use std::time::Duration;

impl DashboardState {
    pub(super) fn render_tab_mouse(
        &mut self,
        ui: &mut egui::Ui,
        _proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        if self.last_spaces_refresh.elapsed() >= Duration::from_secs(3) {
            self.spaces_status = core_input::get_spaces_status();
            self.last_spaces_refresh = std::time::Instant::now();
        }

        let p = palette();
        let avail_w = ui.available_width();

        // 1. Page Header
        setting_row_split(
            ui,
            |ui| {
                if self.input_cfg.external_mouse_enabled {
                    badge_pill(
                        ui,
                        tr("✨ Mouse Optimization ON", "✨ Đang bật tối ưu Chuột rời"),
                        p.success,
                        p.success_soft,
                    );
                } else {
                    badge_pill(
                        ui,
                        tr("⚪ Mouse Optimization OFF", "⚪ Đang tắt Chuột rời"),
                        p.warning,
                        p.warning_soft,
                    );
                }
                if secondary_button(ui, tr("🔄 Refresh Apps", "🔄 Làm mới App"))
                    .on_hover_text(tr(
                        "Refresh running apps & macOS Desktop Spaces count",
                        "Làm mới danh sách ứng dụng đang chạy & số lượng Desktop",
                    ))
                    .clicked()
                {
                    self.running_apps = core_input::list_running_gui_apps();
                    self.spaces_status = core_input::get_spaces_status();
                }
            },
            |ui| {
                label_with_tip(
                    ui,
                    RichText::new(tr(
                        "External Mouse & 120Hz Smooth Scroll",
                        "Chuột rời & Cuộn mượt 120Hz",
                    ))
                    .size(16.5)
                    .strong()
                    .color(p.text_primary),
                    tr(
                        "Optimize Logitech, Razer, Microsoft & gaming mice on macOS: independent reverse scroll, 120Hz smooth inertia, and per-app button actions without affecting MacBook Trackpad.",
                        "Tối ưu hóa toàn diện cho chuột Logitech, Razer, Microsoft và chuột Gaming trên macOS: Đảo chiều cuộn độc lập (không ảnh hưởng Trackpad), nội suy cuộn mượt 120Hz và gán nút phụ theo từng ứng dụng.",
                    ),
                );
            },
        );

        ui.add_space(6.0);

        // 2. Sub-tab Bar: Pill Tabs + Live macOS Spaces Status Pill
        let show_diagram_hint = avail_w >= 820.0;
        setting_row_split(
            ui,
            |ui| {
                let total_sp = self.spaces_status.total_spaces.max(1);
                let cur_sp = self.spaces_status.current_space_index.clamp(1, total_sp);
                let (sp_fg, sp_bg) = if total_sp <= 1 {
                    (p.warning, p.warning_soft)
                } else {
                    (p.success, p.success_soft)
                };
                badge_pill(
                    ui,
                    &format!("🖥 Desktop {} / {}", cur_sp, total_sp),
                    sp_fg,
                    sp_bg,
                );
                if show_diagram_hint {
                    badge_pill(
                        ui,
                        tr(
                            "Click diagram button to inspect",
                            "Nhấp nút trên sơ đồ để kiểm tra",
                        ),
                        p.text_secondary,
                        p.bg_input,
                    );
                }
            },
            |ui| {
                pill_tabs_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if pill_tab_button(
                            ui,
                            self.mouse_subtab == MouseSubTab::Buttons,
                            tr(
                                "🎯 Buttons & Reverse Scroll",
                                "🎯 Nút bấm & Đảo chiều cuộn",
                            ),
                        )
                        .clicked()
                        {
                            self.mouse_subtab = MouseSubTab::Buttons;
                        }
                        if pill_tab_button(
                            ui,
                            self.mouse_subtab == MouseSubTab::SmoothScroll,
                            tr(
                                "🌊 120Hz Smooth Scroll Tuning",
                                "🌊 Tinh chỉnh Cuộn mượt 120Hz",
                            ),
                        )
                        .clicked()
                        {
                            self.mouse_subtab = MouseSubTab::SmoothScroll;
                        }
                    });
                });
            },
        );

        thin_divider(ui);
        ui.add_space(4.0);

        if self.mouse_subtab == MouseSubTab::Buttons {
            // 3. Top 3 Quick Switch Cards (Master | Reverse Scroll | Smooth 120Hz)
            let mut top_changed = false;
            if ui.available_width() >= 680.0 {
                ui.columns(3, |cols| {
                    self.render_mouse_top_card_master(&mut cols[0], &mut top_changed);
                    self.render_mouse_top_card_reverse(&mut cols[1], &mut top_changed);
                    self.render_mouse_top_card_smooth(&mut cols[2], &mut top_changed);
                });
            } else {
                self.render_mouse_top_card_master(ui, &mut top_changed);
                ui.add_space(6.0);
                self.render_mouse_top_card_reverse(ui, &mut top_changed);
                ui.add_space(6.0);
                self.render_mouse_top_card_smooth(ui, &mut top_changed);
            }

            if top_changed {
                self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
            }

            ui.add_space(8.0);

            // 4. Unified Button Mapping Card
            let mut cfg_changed = false;
            let mut remove_bundle: Option<String> = None;

            card_frame().show(ui, |ui| {
                // Card Header: Title + Tip on left | Enable Side Buttons switch on right
                setting_row_split(
                    ui,
                    |ui| {
                        if toggle_switch(ui, &mut self.input_cfg.mouse_buttons_enabled).changed() {
                            cfg_changed = true;
                        }
                        ui.label(
                            RichText::new(tr("Enable Side Buttons", "Bật gán nút phụ"))
                                .size(12.0)
                                .strong()
                                .color(p.text_secondary),
                        );
                    },
                    |ui| {
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "🎯 Side & Middle Mouse Button Actions (Per-App)",
                                "🎯 Gán hành động cho Nút phụ Chuột (Theo từng App)",
                            ))
                            .size(14.0)
                            .strong()
                            .color(p.text_primary),
                            tr(
                                "Click any button on the 5-button mouse diagram on the left to inspect or test it. Configure default actions or add per-application profiles on the right.",
                                "Nhấp vào từng nút trên sơ đồ chuột 5 nút bên trái để xem nhanh. Mặc định Nút Hông chuyển Desktop trái/phải; riêng trong trình duyệt (Chrome) tự động giữ chức năng Tiến/Lùi trang.",
                            ),
                        );
                    },
                );

                thin_divider(ui);
                ui.add_space(2.0);

                // Resolve current actions for the selected profile so the diagram box can display & test them
                let (fwd_act, back_act, mid_act) = if self.mouse_selected_profile == "global" {
                    (
                        self.input_cfg.mouse_forward_action.clone(),
                        self.input_cfg.mouse_back_action.clone(),
                        self.input_cfg.mouse_middle_action.clone(),
                    )
                } else if let Some(prof) = self
                    .input_cfg
                    .app_mouse_profiles
                    .iter()
                    .find(|p| p.bundle_id == self.mouse_selected_profile)
                {
                    (
                        prof.forward_action.clone(),
                        prof.back_action.clone(),
                        prof.middle_action.clone(),
                    )
                } else {
                    (
                        self.input_cfg.mouse_forward_action.clone(),
                        self.input_cfg.mouse_back_action.clone(),
                        self.input_cfg.mouse_middle_action.clone(),
                    )
                };

                if ui.available_width() >= 680.0 {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(198.0);
                            self.render_mouse_diagram_panel(ui, &fwd_act, &back_act, &mid_act);
                        });
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            self.render_mouse_rules_panel(
                                ui,
                                &mut cfg_changed,
                                &mut remove_bundle,
                            );
                        });
                    });
                } else {
                    self.render_mouse_diagram_panel(ui, &fwd_act, &back_act, &mid_act);
                    ui.add_space(6.0);
                    self.render_mouse_rules_panel(ui, &mut cfg_changed, &mut remove_bundle);
                }
            });

            if let Some(bid) = remove_bundle {
                self.input_cfg
                    .app_mouse_profiles
                    .retain(|p| p.bundle_id != bid);
                self.mouse_selected_profile = "global".to_string();
                cfg_changed = true;
            }
            if cfg_changed {
                self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
            }
        } else {
            // ================================================================
            // SUBTAB 2: 120Hz Smooth Scroll Tuning (`MouseSubTab::SmoothScroll`)
            // ================================================================
            let mut sc_changed = false;
            card_frame().show(ui, |ui| {
                if ui.available_width() >= 820.0 {
                    setting_row_split(
                        ui,
                        |ui| {
                            self.render_smooth_scroll_preset_buttons(ui, &mut sc_changed);
                        },
                        |ui| {
                            label_with_tip(
                                ui,
                                RichText::new(tr(
                                    "🌊 120Hz Smooth Scroll Engine & Presets",
                                    "🌊 Bộ nội suy Cuộn mượt 120Hz (Smooth Scroll Engine)",
                                ))
                                .size(14.0)
                                .strong(),
                                tr(
                                    "Customize step distance, glide duration, and acceleration curve for external mouse wheels.",
                                    "Tùy chỉnh tốc độ, độ trượt quán tính (inertia) và gia tốc cho con lăn chuột rời.",
                                ),
                            );
                        },
                    );
                } else {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "🌊 120Hz Smooth Scroll Engine & Presets",
                            "🌊 Bộ nội suy Cuộn mượt 120Hz (Smooth Scroll Engine)",
                        ))
                        .size(14.0)
                        .strong(),
                        tr(
                            "Customize step distance, glide duration, and acceleration curve for external mouse wheels.",
                            "Tùy chỉnh tốc độ, độ trượt quán tính (inertia) và gia tốc cho con lăn chuột rời.",
                        ),
                    );
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        self.render_smooth_scroll_preset_buttons(ui, &mut sc_changed);
                    });
                }

                ui.add_space(8.0);
                if ui.available_width() >= 660.0 {
                    ui.columns(3, |cols| {
                        self.render_scroll_slider_step(&mut cols[0], &mut sc_changed);
                        self.render_scroll_slider_duration(&mut cols[1], &mut sc_changed);
                        self.render_scroll_slider_accel(&mut cols[2], &mut sc_changed);
                    });
                } else {
                    self.render_scroll_slider_step(ui, &mut sc_changed);
                    ui.add_space(6.0);
                    self.render_scroll_slider_duration(ui, &mut sc_changed);
                    ui.add_space(6.0);
                    self.render_scroll_slider_accel(ui, &mut sc_changed);
                }

                thin_divider(ui);
                ui.horizontal(|ui| {
                    if toggle_switch(ui, &mut self.input_cfg.shift_horizontal_scroll).changed() {
                        sc_changed = true;
                    }
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "Hold Shift + Vertical Wheel = Horizontal Scroll",
                            "Giữ phím Shift + Con lăn dọc = Cuộn ngang (Excel, Figma, Code...)",
                        ))
                        .size(12.5)
                        .strong(),
                        "",
                    );
                });
            });

            if sc_changed {
                self.input_cfg = core_input::update_input_config(self.input_cfg.clone());
            }

            ui.add_space(8.0);

            // Scroll Test Playground Card
            card_frame().show(ui, |ui| {
                setting_row_split(
                    ui,
                    |ui| {
                        ui.label(
                            RichText::new(tr(
                                "Place cursor below and roll mouse wheel",
                                "Đặt con trỏ vào khung dưới và lăn chuột",
                            ))
                            .size(11.0)
                            .color(p.text_muted),
                        );
                    },
                    |ui| {
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "⚡ Smooth Scroll Test Playground",
                                "⚡ Khu vực Cuộn thử nghiệm (Scroll Test Playground)",
                            ))
                            .size(13.5)
                            .strong(),
                            "",
                        );
                    },
                );
                ui.add_space(4.0);
                sub_card_frame().show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_salt("smooth_scroll_playground")
                        .max_height(155.0)
                        .show(ui, |ui| {
                            for i in 1..=16 {
                                setting_row_split(
                                    ui,
                                    |ui| {
                                        ui.label(
                                            RichText::new("120 FPS • Cubic-Out")
                                                .font(FontId::monospace(10.5))
                                                .color(Color32::from_rgb(16, 185, 129)),
                                        );
                                    },
                                    |ui| {
                                        badge_pill(
                                            ui,
                                            &format!("#{:02}", i),
                                            p.accent_text,
                                            p.accent_soft,
                                        );
                                        label_with_tip(
                                            ui,
                                            RichText::new(tr(
                                                "120Hz Smooth Scroll test line — roll your mouse wheel to feel the fluid inertia",
                                                "Dòng thử nghiệm Cuộn mượt 120Hz — lăn bánh xe chuột để cảm nhận độ mượt",
                                            ))
                                            .size(12.0),
                                            "",
                                        );
                                    },
                                );
                                thin_divider(ui);
                            }
                        });
                });
            });
        }
    }

    fn render_mouse_top_card_master(&mut self, ui: &mut egui::Ui, top_changed: &mut bool) {
        let p = palette();
        card_frame().show(ui, |ui| {
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.external_mouse_enabled).changed() {
                        *top_changed = true;
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "🖱 External Mouse (Master)",
                            "🖱 Tối ưu Chuột rời (Master)",
                        ))
                        .size(12.5)
                        .strong()
                        .color(p.text_primary),
                        tr(
                            "Master switch for all side/middle button remapping and mouse scroll enhancements.",
                            "Công tắc tổng bật/tắt toàn bộ tính năng tối ưu hóa nút phụ và cuộn chuột rời.",
                        ),
                    );
                },
            );
            ui.label(
                RichText::new(tr(
                    "Enable/disable all external mouse features",
                    "Bật/tắt toàn bộ tính năng chuột rời",
                ))
                .size(11.0)
                .color(p.text_secondary),
            );
        });
    }

    fn render_mouse_top_card_reverse(&mut self, ui: &mut egui::Ui, top_changed: &mut bool) {
        let p = palette();
        card_frame().show(ui, |ui| {
            let show_badge = ui.available_width() >= 270.0;
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.invert_external_mouse_scroll).changed()
                    {
                        *top_changed = true;
                    }
                    if show_badge {
                        if self.input_cfg.invert_external_mouse_scroll {
                            badge_pill(
                                ui,
                                tr("Reversed", "Đã đảo chiều"),
                                p.success,
                                p.success_soft,
                            );
                        } else {
                            badge_pill(
                                ui,
                                tr("Default", "Mặc định"),
                                p.text_secondary,
                                p.bg_input,
                            );
                        }
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "🔄 Reverse Mouse Scroll",
                            "🔄 Đảo chiều cuộn Chuột rời",
                        ))
                        .size(12.5)
                        .strong()
                        .color(p.text_primary),
                        "",
                    );
                },
            );
            ui.label(
                RichText::new(tr(
                    "MacBook Trackpad always keeps natural direction",
                    "Trackpad MacBook luôn giữ nguyên chiều tự nhiên",
                ))
                .size(11.0)
                .color(p.text_secondary),
            );
        });
    }

    fn render_mouse_top_card_smooth(&mut self, ui: &mut egui::Ui, top_changed: &mut bool) {
        let p = palette();
        card_frame().show(ui, |ui| {
            let show_badge = ui.available_width() >= 260.0;
            setting_row_split(
                ui,
                |ui| {
                    if toggle_switch(ui, &mut self.input_cfg.smooth_scroll_enabled).changed() {
                        *top_changed = true;
                    }
                    if show_badge {
                        if self.input_cfg.smooth_scroll_enabled {
                            badge_pill(ui, "120Hz", p.success, p.success_soft);
                        } else {
                            badge_pill(ui, tr("Off", "Tắt"), p.text_secondary, p.bg_input);
                        }
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("🌊 120Hz Smooth Scroll", "🌊 Cuộn mượt 120Hz"))
                            .size(12.5)
                            .strong()
                            .color(p.text_primary),
                        "",
                    );
                },
            );
            ui.label(
                RichText::new(tr(
                    "120fps fluid inertia for notched mouse wheels",
                    "Nội suy quán tính 120fps cho con lăn chuột rời",
                ))
                .size(11.0)
                .color(p.text_secondary),
            );
        });
    }

    fn render_mouse_diagram_panel(
        &mut self,
        ui: &mut egui::Ui,
        fwd_act: &str,
        back_act: &str,
        mid_act: &str,
    ) {
        let p = palette();
        sub_card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            setting_row_split(
                ui,
                |ui| {
                    ui.label(
                        RichText::new(tr("Click button", "Nhấp nút để chọn"))
                            .size(10.0)
                            .color(p.text_muted),
                    );
                },
                |ui| {
                    ui.label(
                        RichText::new(tr("Button Map", "Sơ đồ nút"))
                            .size(11.0)
                            .strong()
                            .color(p.text_secondary),
                    );
                },
            );

            ui.vertical_centered(|ui| {
                if let Some(clicked_btn) =
                    draw_interactive_mouse_diagram(ui, &self.mouse_selected_btn)
                {
                    self.mouse_selected_btn = clicked_btn.to_string();
                }
            });

            let (btn_title, btn_act_key, btn_act_label) = match self.mouse_selected_btn.as_str() {
                "back" => (
                    tr("Side Back (Mouse 4)", "Nút Hông Sau (Mouse 4 — Back)"),
                    back_act.to_string(),
                    translate_mouse_action(back_act),
                ),
                "middle" => (
                    tr(
                        "Middle Wheel (Mouse 3)",
                        "Nút Giữa (Mouse 3 — Middle Click)",
                    ),
                    mid_act.to_string(),
                    translate_mouse_action(mid_act),
                ),
                "chord" => (
                    tr(
                        "Left + Right (L+R Chord)",
                        "Chuột Trái + Phải (L+R Chord)",
                    ),
                    self.input_cfg.mouse_chord_action.clone(),
                    if self.input_cfg.mouse_chord_enabled {
                        translate_chord_action(&self.input_cfg.mouse_chord_action)
                    } else {
                        tr("🚫 Disabled", "🚫 Đang tắt")
                    },
                ),
                _ => (
                    tr(
                        "Side Forward (Mouse 5)",
                        "Nút Hông Trước (Mouse 5 — Forward)",
                    ),
                    fwd_act.to_string(),
                    translate_mouse_action(fwd_act),
                ),
            };

            egui::Frame::none()
                .fill(p.bg_card)
                .stroke(Stroke::new(1.0_f32, p.border))
                .rounding(Rounding::same(7.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(btn_title)
                            .size(11.0)
                            .strong()
                            .color(p.text_primary),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(sanitize_ui_text(btn_act_label))
                                .size(10.5)
                                .color(p.accent_text),
                        )
                        .truncate(),
                    );
                    ui.add_space(3.0);
                    let btn_w = ui.available_width();
                    if ui
                        .add_sized(
                            [btn_w, 24.0],
                            egui::Button::new(
                                RichText::new(tr("▶ Test This Action", "▶ Thử hành động này"))
                                    .size(11.0)
                                    .strong()
                                    .color(p.text_primary),
                            )
                            .fill(p.bg_card_hover)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .rounding(Rounding::same(5.0)),
                        )
                        .clicked()
                    {
                        let _ = core_input::trigger_system_action(&btn_act_key);
                    }
                });
        });
    }

    fn render_mouse_rules_panel(
        &mut self,
        ui: &mut egui::Ui,
        cfg_changed: &mut bool,
        remove_bundle: &mut Option<String>,
    ) {
        let p = palette();

        // Box 1: Application Profile Selector
        sub_card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let box_w = ui.available_width();
            let quick_combo_w = (box_w * 0.28).clamp(120.0, 175.0);

            setting_row_split(
                ui,
                |ui| {
                    if secondary_button(ui, tr("➕ Choose App...", "➕ Chọn App...")).clicked() {
                        self.installed_gui_apps = core_input::list_installed_gui_apps();
                        self.app_picker_filter.clear();
                        self.show_app_picker_modal = true;
                    }

                    if !self.running_apps.is_empty() {
                        ComboBox::from_id_salt("mouse_quick_add_running_combo")
                            .width(quick_combo_w)
                            .truncate()
                            .selected_text(tr("+ Running App...", "+ App đang chạy..."))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(250.0);
                                let running = self.running_apps.clone();
                                for r in running {
                                    let exists = self
                                        .input_cfg
                                        .app_mouse_profiles
                                        .iter()
                                        .any(|p| p.bundle_id == r.bundle_id);
                                    if !exists
                                        && ui
                                            .selectable_label(
                                                false,
                                                format!("{} ({})", r.app_name, r.bundle_id),
                                            )
                                            .clicked()
                                    {
                                        self.add_or_select_mouse_profile(&r.bundle_id, &r.app_name);
                                    }
                                }
                            });
                    }
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr(
                            "1. Select Application for Mouse Button Rules",
                            "1. Chọn Ứng dụng cần cấu hình nút chuột",
                        ))
                        .size(12.0)
                        .strong()
                        .color(p.text_primary),
                        "",
                    );
                },
            );

            ui.add_space(4.0);

            // Solid high-contrast App Profile Pills
            ui.horizontal_wrapped(|ui| {
                if profile_pill_button(
                    ui,
                    self.mouse_selected_profile == "global",
                    tr("🌐 Default (All Apps)", "🌐 Mặc định (Mọi App)"),
                )
                .clicked()
                {
                    self.mouse_selected_profile = "global".to_string();
                }

                let profiles_snap = self.input_cfg.app_mouse_profiles.clone();
                for prof in profiles_snap {
                    let active = self.mouse_selected_profile == prof.bundle_id;
                    let lbl = format!("{} {}", sanitize_ui_text(&prof.icon), prof.app_name);
                    if profile_pill_button(ui, active, &lbl).clicked() {
                        self.mouse_selected_profile = prof.bundle_id.clone();
                    }
                }
            });
        });

        ui.add_space(6.0);

        // Box 2: Selected Profile Rules
        sub_card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let row_w = ui.available_width();
            let combo_w = (row_w * 0.40).clamp(140.0, 230.0);
            let chord_combo_w = (row_w * 0.33).clamp(120.0, 185.0);

            if self.mouse_selected_profile == "global" {
                // Header for Global Default Rules
                setting_row_split(
                    ui,
                    |ui| {
                        badge_pill(
                            ui,
                            tr("Auto Desktop Switch", "Tự động chuyển Desktop"),
                            p.success,
                            p.success_soft,
                        );
                    },
                    |ui| {
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "🌐 Default Rules (Applied to Finder & Other Apps)",
                                "🌐 Quy tắc Mặc định (Áp dụng cho Finder & các App khác)",
                            ))
                            .size(12.5)
                            .strong()
                            .color(p.text_primary),
                            tr(
                                "Default rules apply to all applications that do not have a dedicated profile above.",
                                "Quy tắc mặc định áp dụng cho mọi ứng dụng không có cấu hình riêng ở danh sách phía trên.",
                            ),
                        );
                    },
                );

                thin_divider(ui);

                let sel_fwd = self.mouse_selected_btn == "forward";
                let sel_back = self.mouse_selected_btn == "back";
                let sel_mid = self.mouse_selected_btn == "middle";
                let sel_chord = self.mouse_selected_btn == "chord";

                // Row 1: Mouse 5 (Forward)
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt("global_mouse_row_fwd")
                            .width(combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_mouse_action(
                                &self.input_cfg.mouse_forward_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in MOUSE_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut self.input_cfg.mouse_forward_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_mouse_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "forward".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Mouse 5", p.accent_text, p.accent_soft);
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "Side Forward Button (Thumb Upper)",
                                "Nút Hông Trước (Forward / Tiến lên)",
                            ))
                            .size(12.5)
                            .strong()
                            .color(if sel_fwd {
                                p.accent_text
                            } else {
                                p.text_primary
                            }),
                            "",
                        );
                    },
                );
                thin_divider(ui);

                // Row 2: Mouse 4 (Back)
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt("global_mouse_row_back")
                            .width(combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_mouse_action(
                                &self.input_cfg.mouse_back_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in MOUSE_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut self.input_cfg.mouse_back_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_mouse_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "back".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Mouse 4", p.accent_text, p.accent_soft);
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "Side Back Button (Thumb Lower)",
                                "Nút Hông Sau (Back / Trở về)",
                            ))
                            .size(12.5)
                            .strong()
                            .color(if sel_back {
                                p.accent_text
                            } else {
                                p.text_primary
                            }),
                            "",
                        );
                    },
                );
                thin_divider(ui);

                // Row 3: Mouse 3 (Middle Wheel Click)
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt("global_mouse_row_mid")
                            .width(combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_mouse_action(
                                &self.input_cfg.mouse_middle_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in MOUSE_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut self.input_cfg.mouse_middle_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_mouse_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "middle".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Mouse 3", p.success, p.success_soft);
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "Middle Button (Scroll Wheel Click)",
                                "Nút Giữa (Nhấn con lăn / Middle Click)",
                            ))
                            .size(12.5)
                            .strong()
                            .color(if sel_mid {
                                p.success
                            } else {
                                p.text_primary
                            }),
                            "",
                        );
                    },
                );
                thin_divider(ui);

                // Row 4: Single Space Fallback (`single_space_fallback`)
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt("global_mouse_single_space")
                            .width(combo_w)
                            .truncate()
                            .selected_text(translate_single_space_fallback(
                                &self.input_cfg.single_space_fallback,
                            ))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in SINGLE_SPACE_FALLBACK_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut self.input_cfg.single_space_fallback,
                                            opt.to_string(),
                                            translate_single_space_fallback(opt),
                                        )
                                        .changed()
                                    {
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Spaces", p.warning, p.warning_soft);
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "When Only 1 Desktop (Space) Exists",
                                "Khi chỉ có 1 Desktop (Space) duy nhất",
                            ))
                            .size(12.5)
                            .strong()
                            .color(p.text_primary),
                            tr(
                                "Choose what happens when a button is set to switch Desktops, but macOS currently has only 1 Desktop open.",
                                "Chọn hành vi khi nút chuột được gán chuyển Desktop nhưng hiện tại macOS chỉ mở 1 Desktop duy nhất.",
                            ),
                        );
                    },
                );
                thin_divider(ui);

                // Row 5: L + R Simultaneous Click (`mouse_chord_enabled` + `mouse_chord_action`)
                setting_row_split(
                    ui,
                    |ui| {
                        if toggle_switch(ui, &mut self.input_cfg.mouse_chord_enabled).changed() {
                            self.mouse_selected_btn = "chord".to_string();
                            *cfg_changed = true;
                        }
                        ComboBox::from_id_salt("global_mouse_chord_action")
                            .width(chord_combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_chord_action(
                                &self.input_cfg.mouse_chord_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(250.0);
                                for &opt in CHORD_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut self.input_cfg.mouse_chord_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_chord_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "chord".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "L + R", p.accent_text, p.accent_soft);
                        label_with_tip(
                            ui,
                            RichText::new(tr(
                                "Left + Right Simultaneous Click (Chord)",
                                "Nhấn Chuột Trái + Phải cùng lúc (Chord)",
                            ))
                            .size(12.5)
                            .strong()
                            .color(if sel_chord {
                                p.accent_text
                            } else {
                                p.text_primary
                            }),
                            "",
                        );
                    },
                );
            } else if let Some(prof) = self
                .input_cfg
                .app_mouse_profiles
                .iter_mut()
                .find(|p| p.bundle_id == self.mouse_selected_profile)
            {
                // Selected Per-App Profile Rules
                let app_name_disp = prof.app_name.clone();
                let bundle_id_disp = prof.bundle_id.clone();
                let icon_disp = sanitize_ui_text(&prof.icon);

                setting_row_split(
                    ui,
                    |ui| {
                        if danger_button(ui, "🗑")
                            .on_hover_text(tr(
                                "Remove this app profile",
                                "Xóa cấu hình riêng của App này",
                            ))
                            .clicked()
                        {
                            *remove_bundle = Some(bundle_id_disp.clone());
                        }
                        if toggle_switch(ui, &mut prof.enabled).changed() {
                            *cfg_changed = true;
                        }
                        ui.label(
                            RichText::new(tr(
                                "Enable for this App",
                                "Áp dụng riêng cho App này",
                            ))
                            .size(11.5)
                            .strong()
                            .color(p.text_secondary),
                        );
                    },
                    |ui| {
                        ui.label(
                            RichText::new(format!("{} {}", icon_disp, app_name_disp))
                                .size(13.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new(format!("({})", bundle_id_disp))
                                .font(FontId::monospace(11.0))
                                .color(p.text_muted),
                        );
                    },
                );

                thin_divider(ui);

                // Row 1: App Mouse 5
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt(("prof_fwd", &bundle_id_disp))
                            .width(combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_mouse_action(
                                &prof.forward_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in MOUSE_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut prof.forward_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_mouse_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "forward".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Mouse 5", p.accent_text, p.accent_soft);
                        label_with_tip(
                            ui,
                            RichText::new(format!(
                                "{} {}",
                                tr("Side Forward Button in", "Nút Hông Trước (Forward) khi dùng"),
                                app_name_disp
                            ))
                            .size(12.5)
                            .strong(),
                            "",
                        );
                    },
                );
                thin_divider(ui);

                // Row 2: App Mouse 4
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt(("prof_back", &bundle_id_disp))
                            .width(combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_mouse_action(
                                &prof.back_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in MOUSE_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut prof.back_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_mouse_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "back".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Mouse 4", p.accent_text, p.accent_soft);
                        label_with_tip(
                            ui,
                            RichText::new(format!(
                                "{} {}",
                                tr("Side Back Button in", "Nút Hông Sau (Back) khi dùng"),
                                app_name_disp
                            ))
                            .size(12.5)
                            .strong(),
                            "",
                        );
                    },
                );
                thin_divider(ui);

                // Row 3: App Mouse 3
                setting_row_split(
                    ui,
                    |ui| {
                        ComboBox::from_id_salt(("prof_mid", &bundle_id_disp))
                            .width(combo_w)
                            .truncate()
                            .selected_text(sanitize_ui_text(translate_mouse_action(
                                &prof.middle_action,
                            )))
                            .show_ui(ui, |ui| {
                                ui.set_min_width(270.0);
                                for &opt in MOUSE_ACTION_OPTIONS {
                                    if ui
                                        .selectable_value(
                                            &mut prof.middle_action,
                                            opt.to_string(),
                                            sanitize_ui_text(translate_mouse_action(opt)),
                                        )
                                        .changed()
                                    {
                                        self.mouse_selected_btn = "middle".to_string();
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "Mouse 3", p.success, p.success_soft);
                        label_with_tip(
                            ui,
                            RichText::new(format!(
                                "{} {}",
                                tr("Middle Wheel Button in", "Nút Giữa (Middle Click) khi dùng"),
                                app_name_disp
                            ))
                            .size(12.5)
                            .strong(),
                            "",
                        );
                    },
                );
                thin_divider(ui);

                // Row 4: App 120Hz Smooth Scroll Mode
                setting_row_split(
                    ui,
                    |ui| {
                        let modes = [
                            (
                                "inherit",
                                tr("Follow Global Setting", "Theo cài đặt chung (Mặc định)"),
                            ),
                            (
                                "on",
                                tr(
                                    "Always Enable Smooth Scroll",
                                    "Luôn Bật cuộn mượt trong App này",
                                ),
                            ),
                            (
                                "off",
                                tr("Disable Smooth Scroll Here", "Tắt cuộn mượt trong App này"),
                            ),
                        ];
                        ComboBox::from_id_salt(("prof_smooth", &bundle_id_disp))
                            .width(combo_w)
                            .truncate()
                            .selected_text(
                                modes
                                    .iter()
                                    .find(|(k, _)| *k == prof.smooth_scroll_mode)
                                    .map(|(_, v)| *v)
                                    .unwrap_or("Follow Global Setting"),
                            )
                            .show_ui(ui, |ui| {
                                ui.set_min_width(250.0);
                                for (k, lbl) in modes {
                                    if ui
                                        .selectable_value(
                                            &mut prof.smooth_scroll_mode,
                                            k.to_string(),
                                            lbl,
                                        )
                                        .changed()
                                    {
                                        *cfg_changed = true;
                                    }
                                }
                            });
                    },
                    |ui| {
                        badge_pill(ui, "🌊 120Hz", p.accent_text, p.accent_soft);
                        label_with_tip(
                            ui,
                            RichText::new(format!(
                                "{} {}",
                                tr("120Hz Smooth Scroll in", "Cuộn mượt 120Hz cho riêng"),
                                app_name_disp
                            ))
                            .size(12.5)
                            .strong(),
                            "",
                        );
                    },
                );
            }
        });
    }

    fn render_smooth_scroll_preset_buttons(&mut self, ui: &mut egui::Ui, sc_changed: &mut bool) {
        if secondary_button(ui, tr("🌊 Silky", "🌊 Siêu mượt")).clicked() {
            self.input_cfg.scroll_step_px = 55;
            self.input_cfg.scroll_duration_ms = 360;
            self.input_cfg.scroll_acceleration = 1.8;
            *sc_changed = true;
        }
        if primary_button(
            ui,
            tr("✨ Balanced (Default)", "✨ Chuẩn Mac (Khuyên dùng)"),
        )
        .clicked()
        {
            self.input_cfg.scroll_step_px = 45;
            self.input_cfg.scroll_duration_ms = 260;
            self.input_cfg.scroll_acceleration = 1.6;
            *sc_changed = true;
        }
        if secondary_button(ui, tr("⚡ Snappy", "⚡ Nhanh & Dứt khoát")).clicked() {
            self.input_cfg.scroll_step_px = 50;
            self.input_cfg.scroll_duration_ms = 150;
            self.input_cfg.scroll_acceleration = 1.4;
            *sc_changed = true;
        }
    }

    fn render_scroll_slider_step(&mut self, ui: &mut egui::Ui, sc_changed: &mut bool) {
        let p = palette();
        sub_card_frame().show(ui, |ui| {
            setting_row_split(
                ui,
                |ui| {
                    ui.label(
                        RichText::new(format!("{} px", self.input_cfg.scroll_step_px))
                            .font(FontId::monospace(11.5))
                            .strong()
                            .color(p.accent_text),
                    );
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("📏 Step Distance", "📏 Quãng đường mỗi nấc"))
                            .size(12.0)
                            .strong(),
                        "",
                    );
                },
            );
            let w = ui.available_width().max(80.0);
            ui.spacing_mut().slider_width = w;
            if ui
                .add_sized(
                    [w, 18.0],
                    egui::Slider::new(&mut self.input_cfg.scroll_step_px, 15..=120)
                        .show_value(false)
                        .trailing_fill(true),
                )
                .changed()
            {
                *sc_changed = true;
            }
        });
    }

    fn render_scroll_slider_duration(&mut self, ui: &mut egui::Ui, sc_changed: &mut bool) {
        let p = palette();
        sub_card_frame().show(ui, |ui| {
            setting_row_split(
                ui,
                |ui| {
                    ui.label(
                        RichText::new(format!("{} ms", self.input_cfg.scroll_duration_ms))
                            .font(FontId::monospace(11.5))
                            .strong()
                            .color(p.accent_text),
                    );
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("⏱ Glide Duration", "⏱ Thời gian lướt"))
                            .size(12.0)
                            .strong(),
                        "",
                    );
                },
            );
            let w = ui.available_width().max(80.0);
            ui.spacing_mut().slider_width = w;
            if ui
                .add_sized(
                    [w, 18.0],
                    egui::Slider::new(&mut self.input_cfg.scroll_duration_ms, 80..=600)
                        .show_value(false)
                        .trailing_fill(true),
                )
                .changed()
            {
                *sc_changed = true;
            }
        });
    }

    fn render_scroll_slider_accel(&mut self, ui: &mut egui::Ui, sc_changed: &mut bool) {
        let p = palette();
        sub_card_frame().show(ui, |ui| {
            setting_row_split(
                ui,
                |ui| {
                    ui.label(
                        RichText::new(format!("{:.1}x", self.input_cfg.scroll_acceleration))
                            .font(FontId::monospace(11.5))
                            .strong()
                            .color(p.accent_text),
                    );
                },
                |ui| {
                    label_with_tip(
                        ui,
                        RichText::new(tr("🚀 Acceleration", "🚀 Hệ số Gia tốc"))
                            .size(12.0)
                            .strong(),
                        "",
                    );
                },
            );
            let w = ui.available_width().max(80.0);
            ui.spacing_mut().slider_width = w;
            if ui
                .add_sized(
                    [w, 18.0],
                    egui::Slider::new(&mut self.input_cfg.scroll_acceleration, 1.0..=3.0)
                        .show_value(false)
                        .trailing_fill(true),
                )
                .changed()
            {
                *sc_changed = true;
            }
        });
    }
}
