use crate::gpu_ui::dashboard::{CleanerConfirmAction, CleanerSubTab, DashboardState};
use crate::gpu_ui::{
    badge_pill, card_frame, danger_button, format_bytes, palette, path_action_cell,
    pill_tab_button, pill_tabs_frame, primary_button, progress_bar_styled, secondary_button,
    setting_row_split, shorten_middle, sub_card_frame, thin_divider, tip_badge,
    truncated_name_label,
};
use crate::i18n::{tr, translate_cleaner_category, translate_scan_message};
use crate::macos_window::decode_data_url_or_path_to_rgba8_max_size;
use egui::{
    ColorImage, ComboBox, FontId, Margin, RichText, Rounding, Sense, Stroke, TextureOptions,
};
use std::time::{Duration, Instant};

impl DashboardState {
    pub(super) fn trigger_current_cleaner_subtab_scan(&mut self, force: bool) {
        match self.cleaner_tab {
            CleanerSubTab::Junk => {
                self.cleaner_status =
                    core_cleaner::start_cleaner_scan_async("junk", "all", 50, force);
            }
            CleanerSubTab::Duplicates => {
                self.cleaner_status = core_cleaner::start_cleaner_scan_async(
                    "duplicates",
                    &self.dup_scope,
                    self.dup_min_kb,
                    force,
                );
            }
            CleanerSubTab::Apps => {
                self.cleaner_status =
                    core_cleaner::start_cleaner_scan_async("uninstall", "all", 0, force);
                if force || !self.orphans_scanned {
                    self.orphans = core_cleaner::scan_orphaned_background_items();
                    self.orphans_scanned = true;
                }
            }
            CleanerSubTab::LargeFiles => {
                self.cleaner_status = core_cleaner::start_cleaner_scan_async(
                    "large",
                    &self.large_scope,
                    self.large_min_mb,
                    force,
                );
            }
        }
        self.last_cleaner_poll = Instant::now();
    }

    pub(super) fn render_tab_cleaner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Auto-start scan when opening a subtab that hasn't been scanned yet
        if !self.cleaner_status.running {
            let needs_initial_scan = match self.cleaner_tab {
                CleanerSubTab::Junk => self.cleaner_status.junk_result.is_none(),
                CleanerSubTab::Duplicates => self.cleaner_status.duplicates_result.is_none(),
                CleanerSubTab::Apps => self.cleaner_status.apps_result.is_none(),
                CleanerSubTab::LargeFiles => self.cleaner_status.large_result.is_none(),
            };
            if needs_initial_scan {
                self.trigger_current_cleaner_subtab_scan(false);
            } else if self.cleaner_tab == CleanerSubTab::Apps && !self.orphans_scanned {
                self.orphans = core_cleaner::scan_orphaned_background_items();
                self.orphans_scanned = true;
            }
        }

        if self.cleaner_status.running
            && self.last_cleaner_poll.elapsed() >= Duration::from_millis(160)
        {
            self.cleaner_status = core_cleaner::get_cleaner_scan_status();
            self.last_cleaner_poll = Instant::now();
            ctx.request_repaint_after(Duration::from_millis(160));
        }

        let p = palette();
        let mut copied_path_toast: Option<String> = None;

        // 1. Page Header (Exact match with `ui/js/screens/system-cleaner.js`)
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr(
                    "System Cleaner, Duplicates & App Uninstaller",
                    "Dọn Rác, File Trùng & Gỡ Cài đặt App",
                ))
                .size(16.5)
                .strong()
                .color(p.text_primary),
            );
            tip_badge(
                ui,
                tr(
                    "Safe multi-threaded scanner to clean caches & logs, empty macOS Trash, find byte-identical duplicate files, and completely uninstall applications with all leftover Library files.",
                    "Quét đa luồng an toàn để dọn bộ nhớ đệm, dọn Thùng rác macOS, tìm tệp trùng lặp chính xác từng byte và gỡ sạch tận gốc các file rác còn sót khi xóa ứng dụng.",
                ),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, tr("🔄 Rescan Data", "🔄 Quét lại dữ liệu"))
                    .on_hover_text(tr(
                        "Force rescan of the current tab",
                        "Quét lại dữ liệu mới nhất cho mục đang mở",
                    ))
                    .clicked()
                {
                    self.trigger_current_cleaner_subtab_scan(true);
                }
            });
        });

        ui.add_space(6.0);

        // 2. Sub-tab Bar: 4 Pill Tabs + Scan Status Badge
        ui.horizontal(|ui| {
            pill_tabs_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (st, lbl) in [
                        (
                            CleanerSubTab::Junk,
                            tr("🗑 Junk & Caches", "🗑 Dọn Rác & Cache"),
                        ),
                        (
                            CleanerSubTab::Duplicates,
                            tr("📑 Duplicate Files", "📑 File Trùng lặp"),
                        ),
                        (
                            CleanerSubTab::Apps,
                            tr("📦 App Uninstaller", "📦 Gỡ Cài đặt App"),
                        ),
                        (
                            CleanerSubTab::LargeFiles,
                            tr("📦 Large Files", "📦 File Lớn"),
                        ),
                    ] {
                        if pill_tab_button(ui, self.cleaner_tab == st, lbl).clicked()
                            && self.cleaner_tab != st
                        {
                            self.cleaner_tab = st;
                            self.cleaner_status = core_cleaner::get_cleaner_scan_status();
                        }
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.cleaner_status.running {
                    badge_pill(
                        ui,
                        &format!(
                            "⏳ {} ({}%)",
                            translate_scan_message(&self.cleaner_status.message),
                            self.cleaner_status.percent
                        ),
                        p.warning,
                        p.warning_soft,
                    );
                } else {
                    badge_pill(ui, tr("Ready", "Sẵn sàng"), p.success, p.success_soft);
                }
            });
        });

        thin_divider(ui);
        ui.add_space(4.0);

        let has_current_tab_data = match self.cleaner_tab {
            CleanerSubTab::Junk => self.cleaner_status.junk_result.is_some(),
            CleanerSubTab::Duplicates => self.cleaner_status.duplicates_result.is_some(),
            CleanerSubTab::Apps => self.cleaner_status.apps_result.is_some(),
            CleanerSubTab::LargeFiles => self.cleaner_status.large_result.is_some(),
        };

        if self.cleaner_status.running && !has_current_tab_data {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(translate_scan_message(&self.cleaner_status.message))
                            .size(13.0)
                            .strong()
                            .color(p.text_primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        badge_pill(
                            ui,
                            &format!("{}%", self.cleaner_status.percent.max(5)),
                            p.success,
                            p.success_soft,
                        );
                    });
                });
                ui.add_space(4.0);
                progress_bar_styled(
                    ui,
                    (self.cleaner_status.percent.max(5) as f32) / 100.0,
                    p.accent,
                    8.0,
                );
            });
            ui.add_space(6.0);
        }

        match self.cleaner_tab {
            // ================================================================
            // SUBTAB 1: SYSTEM JUNK & MACOS TRASH (`CleanerSubTab::Junk`)
            // ================================================================
            CleanerSubTab::Junk => {
                if let Some(junk) = self.cleaner_status.junk_result.clone() {
                    // Card 1: macOS Trash (`~/.Trash`) with Amber Border
                    egui::Frame::none()
                        .fill(p.bg_card)
                        .stroke(Stroke::new(1.2_f32, p.warning))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tr(
                                        "🗑 macOS Trash (~/.Trash)",
                                        "🗑 Thùng rác macOS (~/.Trash)",
                                    ))
                                    .size(13.5)
                                    .strong()
                                    .color(p.text_primary),
                                );
                                let (tr_fg, tr_bg) = if junk.trash.item_count > 0 {
                                    (p.warning, p.warning_soft)
                                } else {
                                    (p.success, p.success_soft)
                                };
                                badge_pill(
                                    ui,
                                    &format!(
                                        "{} {} • {:.1} MB",
                                        junk.trash.item_count,
                                        tr("items", "mục"),
                                        junk.trash.total_size_mb
                                    ),
                                    tr_fg,
                                    tr_bg,
                                );
                                tip_badge(
                                    ui,
                                    tr(
                                        "Inspect files currently inside your macOS Trash (~/.Trash), open Trash in Finder, or permanently empty it.",
                                        "Xem danh sách các tệp đang nằm trong Thùng rác macOS (~/.Trash), mở trong Finder hoặc đổ sạch vĩnh viễn.",
                                    ),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_enabled(
                                                junk.trash.item_count > 0,
                                                egui::Button::new(
                                                    RichText::new(format!(
                                                        "🔥 {} ({:.1} MB)",
                                                        tr("Empty Trash Now", "Đổ Thùng rác ngay"),
                                                        junk.trash.total_size_mb
                                                    ))
                                                    .size(12.0)
                                                    .strong()
                                                    .color(p.danger),
                                                )
                                                .fill(p.danger_soft)
                                                .stroke(Stroke::new(1.0_f32, p.danger))
                                                .rounding(Rounding::same(6.0)),
                                            )
                                            .clicked()
                                        {
                                            self.cleaner_confirm_action =
                                                Some(CleanerConfirmAction::EmptyTrash {
                                                    item_count: junk.trash.item_count,
                                                    total_mb: junk.trash.total_size_mb,
                                                });
                                        }

                                        if secondary_button(
                                            ui,
                                            tr("📂 Open Trash", "📂 Mở Trash"),
                                        )
                                        .clicked()
                                        {
                                            let _ = core_cleaner::reveal_in_finder(
                                                &junk.trash.trash_path,
                                            );
                                        }

                                        if !junk.trash.items.is_empty() {
                                            let toggle_lbl = if self.cleaner_show_trash_details {
                                                tr("▲ Hide Details", "▲ Ẩn chi tiết").to_string()
                                            } else {
                                                format!(
                                                    "🔍 {} ({})",
                                                    tr("View Trash Items", "Xem mục trong Trash"),
                                                    junk.trash.items.len()
                                                )
                                            };
                                            if secondary_button(ui, &toggle_lbl).clicked() {
                                                self.cleaner_show_trash_details =
                                                    !self.cleaner_show_trash_details;
                                            }
                                        }
                                    },
                                );
                            });

                            if self.cleaner_show_trash_details && !junk.trash.items.is_empty() {
                                thin_divider(ui);
                                for tr_item in junk.trash.items.iter().take(40) {
                                    setting_row_split(
                                        ui,
                                        |ui| {
                                            if let Some(cp) =
                                                path_action_cell(ui, &tr_item.path, true)
                                            {
                                                copied_path_toast = Some(cp);
                                            }
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.2} MB",
                                                    tr_item.size_mb
                                                ))
                                                .size(12.0)
                                                .strong()
                                                .color(p.warning),
                                            );
                                        },
                                        |ui| {
                                            ui.label(
                                                RichText::new(if tr_item.is_dir {
                                                    "📁"
                                                } else {
                                                    "📄"
                                                })
                                                .size(12.5),
                                            );
                                            truncated_name_label(
                                                ui,
                                                &tr_item.name,
                                                26,
                                                12.0,
                                                p.text_primary,
                                            );
                                            badge_pill(
                                                ui,
                                                if tr_item.is_dir {
                                                    tr("Folder", "Thư mục")
                                                } else {
                                                    tr("File", "Tệp tin")
                                                },
                                                p.warning,
                                                p.warning_soft,
                                            );
                                        },
                                    );
                                    thin_divider(ui);
                                }
                            }
                        });

                    ui.add_space(8.0);

                    // Card 2: System Caches & Junk Table
                    card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr(
                                    "🗑 System Caches & Application Logs",
                                    "🗑 Bộ nhớ đệm & File Rác Hệ thống",
                                ))
                                .size(13.5)
                                .strong()
                                .color(p.text_primary),
                            );
                            badge_pill(
                                ui,
                                &format!("{:.1} MB", junk.total_size_mb),
                                p.warning,
                                p.warning_soft,
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let sel_count = self.selected_junk_paths.len();
                                    let clean_lbl = format!(
                                        "🚀 {} ({})",
                                        tr("Clean Selected", "Dọn sạch mục đã chọn"),
                                        sel_count
                                    );
                                    if primary_button(ui, &clean_lbl).clicked() {
                                        if sel_count == 0 {
                                            self.show_toast(tr(
                                                "Please select at least 1 item to clean.",
                                                "Vui lòng tích chọn ít nhất 1 mục rác để dọn.",
                                            ));
                                        } else {
                                            let paths: Vec<String> =
                                                self.selected_junk_paths.iter().cloned().collect();
                                            let total_mb: f64 = junk
                                                .items
                                                .iter()
                                                .filter(|it| {
                                                    self.selected_junk_paths.contains(&it.path)
                                                })
                                                .map(|it| it.size_mb)
                                                .sum();
                                            self.cleaner_confirm_action =
                                                Some(CleanerConfirmAction::CleanJunk {
                                                    paths,
                                                    total_mb,
                                                    also_empty_trash: self.cleaner_also_empty_trash,
                                                });
                                        }
                                    }

                                    ui.checkbox(
                                        &mut self.cleaner_also_empty_trash,
                                        RichText::new(tr(
                                            "Also empty Trash permanently",
                                            "Kết hợp đổ sạch Thùng rác (Xóa vĩnh viễn)",
                                        ))
                                        .size(11.5)
                                        .strong()
                                        .color(p.text_secondary),
                                    );
                                },
                            );
                        });

                        thin_divider(ui);

                        // Table Header Row with Master Checkbox
                        let all_checked = !junk.items.is_empty()
                            && junk
                                .items
                                .iter()
                                .all(|it| self.selected_junk_paths.contains(&it.path));
                        let mut toggle_all = all_checked;
                        ui.horizontal(|ui| {
                            if ui
                                .checkbox(
                                    &mut toggle_all,
                                    RichText::new(tr("Folder", "Thư mục"))
                                        .size(11.5)
                                        .strong()
                                        .color(p.text_muted),
                                )
                                .changed()
                            {
                                if toggle_all {
                                    for it in &junk.items {
                                        self.selected_junk_paths.insert(it.path.clone());
                                    }
                                } else {
                                    self.selected_junk_paths.clear();
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(tr(
                                            "Category  •  Size  •  Path (Click to copy / Open Finder)",
                                            "Phân loại  •  Dung lượng  •  Đường dẫn (Copy / Mở Finder)",
                                        ))
                                        .size(11.0)
                                        .color(p.text_muted),
                                    );
                                },
                            );
                        });
                        thin_divider(ui);

                        if junk.items.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new(tr(
                                        "✨ Your system caches and logs are clean!",
                                        "✨ Hệ thống của bạn hiện rất sạch sẽ, không có mục rác lớn!",
                                    ))
                                    .size(12.5)
                                    .color(p.text_secondary),
                                );
                                ui.add_space(16.0);
                            });
                        } else {
                            for item in &junk.items {
                                setting_row_split(
                                    ui,
                                    |ui| {
                                        if let Some(cp) = path_action_cell(ui, &item.path, true) {
                                            copied_path_toast = Some(cp);
                                        }
                                        ui.label(
                                            RichText::new(format!("{:.1} MB", item.size_mb))
                                                .size(12.0)
                                                .strong()
                                                .color(p.accent_text),
                                        );
                                        badge_pill(
                                            ui,
                                            &translate_cleaner_category(&item.category),
                                            p.warning,
                                            p.warning_soft,
                                        );
                                    },
                                    |ui| {
                                        let mut checked =
                                            self.selected_junk_paths.contains(&item.path);
                                        if ui.checkbox(&mut checked, "").changed() {
                                            if checked {
                                                self.selected_junk_paths.insert(item.path.clone());
                                            } else {
                                                self.selected_junk_paths.remove(&item.path);
                                            }
                                        }
                                        truncated_name_label(
                                            ui,
                                            &item.name,
                                            28,
                                            12.5,
                                            p.text_primary,
                                        );
                                    },
                                );
                                thin_divider(ui);
                            }
                        }
                    });
                }
            }

            // ================================================================
            // SUBTAB 2: DUPLICATE FILES (`CleanerSubTab::Duplicates`)
            // ================================================================
            CleanerSubTab::Duplicates => {
                // Optional Expandable Exclude Manager Card
                if self.cleaner_show_dup_excludes {
                    egui::Frame::none()
                        .fill(p.bg_card)
                        .stroke(Stroke::new(1.2_f32, p.accent_border))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "🛡 {} ({})",
                                        tr(
                                            "Excluded Folders & Patterns",
                                            "Danh sách loại trừ khi quét trùng"
                                        ),
                                        self.dup_excludes.len()
                                    ))
                                    .size(13.0)
                                    .strong()
                                    .color(p.text_primary),
                                );
                                tip_badge(
                                    ui,
                                    tr(
                                        "Folders and patterns listed here are skipped during duplicate scanning. Internal SQL database engines (.ibd, PG_VERSION, .sqlite-wal) are automatically protected.",
                                        "Các thư mục trong danh sách này sẽ được bỏ qua khi quét trùng. Các vùng lưu trữ SQL Engine nội bộ luôn được bảo vệ tự động.",
                                    ),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if secondary_button(
                                            ui,
                                            tr("↺ Restore Defaults", "↺ Khôi phục mặc định"),
                                        )
                                        .clicked()
                                        {
                                            self.dup_excludes =
                                                core_cleaner::reset_duplicate_excludes();
                                            self.trigger_current_cleaner_subtab_scan(true);
                                        }
                                        badge_pill(
                                            ui,
                                            tr(
                                                "🔒 Protects Live SQL Engines",
                                                "🔒 Khoá bảo vệ vùng chạy SQL",
                                            ),
                                            p.warning,
                                            p.warning_soft,
                                        );
                                        badge_pill(
                                            ui,
                                            tr("⚡ Scans .sql Dumps", "⚡ Quét cả file .sql"),
                                            p.success,
                                            p.success_soft,
                                        );
                                    },
                                );
                            });

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [320.0, 26.0],
                                    egui::TextEdit::singleline(&mut self.new_exclude_pattern)
                                        .hint_text(tr(
                                            "Folder name or path (e.g. node_modules, ~/Work)...",
                                            "Nhập tên thư mục hoặc đường dẫn (VD: node_modules, ~/Work)...",
                                        )),
                                );
                                if primary_button(
                                    ui,
                                    tr("➕ Add Exclude Folder", "➕ Thêm thư mục loại trừ"),
                                )
                                .clicked()
                                    && !self.new_exclude_pattern.trim().is_empty()
                                {
                                    let pat = self.new_exclude_pattern.trim().to_string();
                                    self.dup_excludes = core_cleaner::add_duplicate_exclude(&pat);
                                    self.new_exclude_pattern.clear();
                                    self.cleaner_status =
                                        core_cleaner::filter_duplicates_by_excludes_in_cached_scan(
                                        );
                                }
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                let snap = self.dup_excludes.clone();
                                for pat in snap {
                                    if secondary_button(ui, &format!("📁 {}  ✕", pat))
                                        .on_hover_text(tr(
                                            "Click to remove from exclude list",
                                            "Nhấp để bỏ khỏi danh sách loại trừ",
                                        ))
                                        .clicked()
                                    {
                                        self.dup_excludes =
                                            core_cleaner::remove_duplicate_exclude(&pat);
                                        self.trigger_current_cleaner_subtab_scan(true);
                                    }
                                }
                            });
                        });
                    ui.add_space(8.0);
                }

                // Main Duplicates Card
                let mut rescan_dups = false;
                let mut exclude_dir_to_add: Option<String> = None;

                card_frame().show(ui, |ui| {
                    let dup_opt = self.cleaner_status.duplicates_result.clone();
                    let (total_groups, total_files, total_wasted) = if let Some(d) = &dup_opt {
                        (d.total_groups, d.total_duplicate_files, d.total_wasted_mb)
                    } else {
                        (0, 0, 0.0)
                    };

                    // Header Toolbar
                    ui.horizontal_wrapped(|ui| {
                        let scopes = [
                            ("all", tr("All Home (~)", "Toàn bộ thư mục cá nhân (~)")),
                            ("downloads", "~/Downloads"),
                            ("documents", "~/Documents"),
                            ("desktop", "~/Desktop"),
                            ("pictures", "~/Pictures"),
                            ("movies", "~/Movies"),
                        ];
                        ComboBox::from_id_salt("dup_scope_combo")
                            .width(175.0)
                            .truncate()
                            .selected_text(
                                scopes
                                    .iter()
                                    .find(|(k, _)| *k == self.dup_scope)
                                    .map(|(_, v)| *v)
                                    .unwrap_or("~/"),
                            )
                            .show_ui(ui, |ui| {
                                ui.set_min_width(220.0);
                                for (k, lbl) in scopes {
                                    if ui
                                        .selectable_value(
                                            &mut self.dup_scope,
                                            k.to_string(),
                                            lbl,
                                        )
                                        .changed()
                                    {
                                        rescan_dups = true;
                                    }
                                }
                            });

                        let min_sizes = [
                            (10u64, "> 10 KB".to_string()),
                            (
                                100,
                                format!("> 100 KB ({})", tr("Recommended", "Khuyên dùng")),
                            ),
                            (1024, "> 1 MB".to_string()),
                            (10240, "> 10 MB".to_string()),
                            (51200, "> 50 MB".to_string()),
                        ];
                        ComboBox::from_id_salt("dup_min_kb_combo")
                            .width(145.0)
                            .truncate()
                            .selected_text(
                                min_sizes
                                    .iter()
                                    .find(|(k, _)| *k == self.dup_min_kb)
                                    .map(|(_, v)| v.as_str())
                                    .unwrap_or("> 100 KB"),
                            )
                            .show_ui(ui, |ui| {
                                ui.set_min_width(185.0);
                                for (k, lbl) in &min_sizes {
                                    if ui
                                        .selectable_value(&mut self.dup_min_kb, *k, lbl)
                                        .changed()
                                    {
                                        rescan_dups = true;
                                    }
                                }
                            });

                        badge_pill(
                            ui,
                            &format!(
                                "{} {} • {} {} • {} {:.1} MB",
                                total_groups,
                                tr("groups", "nhóm"),
                                total_files,
                                tr("copies", "bản sao"),
                                tr("Wasted", "Phí"),
                                total_wasted
                            ),
                            p.warning,
                            p.warning_soft,
                        );

                        let ex_btn_lbl = format!(
                            "🛡 {} ({})",
                            tr("Excludes", "Loại trừ"),
                            self.dup_excludes.len()
                        );
                        if (if self.cleaner_show_dup_excludes {
                            primary_button(ui, &ex_btn_lbl)
                        } else {
                            secondary_button(ui, &ex_btn_lbl)
                        })
                        .clicked()
                        {
                            self.cleaner_show_dup_excludes = !self.cleaner_show_dup_excludes;
                        }

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let sel_count = self.selected_dup_paths.len();
                                if danger_button(
                                    ui,
                                    &format!(
                                        "🗑 {} ({})",
                                        tr("Delete Selected", "Xóa file đã chọn"),
                                        sel_count
                                    ),
                                )
                                .clicked()
                                {
                                    if sel_count == 0 {
                                        self.show_toast(tr(
                                            "Select duplicate copies to delete first.",
                                            "Hãy chọn các bản sao trùng lặp cần xóa trước.",
                                        ));
                                    } else {
                                        let paths: Vec<String> =
                                            self.selected_dup_paths.iter().cloned().collect();
                                        let mut total_mb = 0.0;
                                        if let Some(d) = &dup_opt {
                                            for g in &d.groups {
                                                for f in &g.files {
                                                    if self.selected_dup_paths.contains(&f.path) {
                                                        total_mb += f.size_mb;
                                                    }
                                                }
                                            }
                                        }
                                        self.cleaner_confirm_action =
                                            Some(CleanerConfirmAction::DeleteDuplicates {
                                                paths,
                                                total_mb,
                                                also_empty_trash: self.cleaner_also_empty_trash,
                                            });
                                    }
                                }

                                ui.checkbox(
                                    &mut self.cleaner_also_empty_trash,
                                    RichText::new(tr("Empty Trash", "Đổ luôn Trash"))
                                        .size(11.5)
                                        .strong()
                                        .color(p.text_secondary),
                                );

                                if secondary_button(
                                    ui,
                                    tr("✨ Auto-select Copies", "✨ Tự động chọn các bản sao"),
                                )
                                .clicked()
                                {
                                    if let Some(d) = &dup_opt {
                                        self.selected_dup_paths.clear();
                                        for g in &d.groups {
                                            for f in &g.files {
                                                if !f.is_original {
                                                    self.selected_dup_paths.insert(f.path.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        );
                    });

                    thin_divider(ui);

                    if let Some(dups) = dup_opt {
                        if dups.groups.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(
                                    RichText::new(tr(
                                        "✨ No duplicate files found in this scope!",
                                        "✨ Không tìm thấy tệp trùng lặp nào trong phạm vi này!",
                                    ))
                                    .size(12.5)
                                    .color(p.text_secondary),
                                );
                                ui.add_space(20.0);
                            });
                        } else {
                            for grp in &dups.groups {
                                sub_card_frame().show(ui, |ui| {
                                    setting_row_split(
                                        ui,
                                        |ui| {
                                            badge_pill(
                                                ui,
                                                &format!(
                                                    "{} {} • {} {:.2} MB",
                                                    grp.count,
                                                    tr("files", "tệp"),
                                                    tr("Wasted", "Phí"),
                                                    grp.wasted_mb
                                                ),
                                                p.warning,
                                                p.warning_soft,
                                            );
                                        },
                                        |ui| {
                                            badge_pill(
                                                ui,
                                                &format!("#{}", grp.group_id),
                                                p.accent_text,
                                                p.accent_soft,
                                            );
                                            truncated_name_label(
                                                ui,
                                                &grp.representative_name,
                                                28,
                                                12.5,
                                                p.text_primary,
                                            );
                                            badge_pill(
                                                ui,
                                                &translate_cleaner_category(&grp.kind),
                                                p.text_secondary,
                                                p.bg_card,
                                            );
                                        },
                                    );
                                    thin_divider(ui);

                                    for f in &grp.files {
                                        let parent_dir = f
                                            .path
                                            .rfind('/')
                                            .filter(|&i| i > 0)
                                            .map(|i| f.path[..i].to_string());
                                        setting_row_split(
                                            ui,
                                            |ui| {
                                                if let Some(dir) = parent_dir {
                                                    if secondary_button(
                                                        ui,
                                                        tr(
                                                            "🚫 Ignore Folder",
                                                            "🚫 Bỏ qua thư mục",
                                                        ),
                                                    )
                                                    .on_hover_text(&dir)
                                                    .clicked()
                                                    {
                                                        exclude_dir_to_add = Some(dir);
                                                    }
                                                }
                                                if let Some(cp) =
                                                    path_action_cell(ui, &f.path, true)
                                                {
                                                    copied_path_toast = Some(cp);
                                                }
                                            },
                                            |ui| {
                                                let mut checked =
                                                    self.selected_dup_paths.contains(&f.path);
                                                if ui.checkbox(&mut checked, "").changed() {
                                                    if checked {
                                                        self.selected_dup_paths
                                                            .insert(f.path.clone());
                                                    } else {
                                                        self.selected_dup_paths.remove(&f.path);
                                                    }
                                                }
                                                if f.is_original {
                                                    badge_pill(
                                                        ui,
                                                        tr(
                                                            "🔒 Original (Keep)",
                                                            "🔒 Bản gốc (Giữ)",
                                                        ),
                                                        p.success,
                                                        p.success_soft,
                                                    );
                                                } else {
                                                    badge_pill(
                                                        ui,
                                                        tr("📑 Duplicate", "📑 Bản sao"),
                                                        p.warning,
                                                        p.warning_soft,
                                                    );
                                                }
                                                let size_lbl = if f.size_mb >= 0.1 {
                                                    format!("{:.2} MB", f.size_mb)
                                                } else {
                                                    format!("{} KB", (f.size_bytes / 1024).max(1))
                                                };
                                                ui.label(
                                                    RichText::new(size_lbl)
                                                        .size(11.5)
                                                        .strong()
                                                        .color(if f.is_original {
                                                            p.text_muted
                                                        } else {
                                                            p.warning
                                                        }),
                                                );
                                            },
                                        );
                                    }
                                });
                                ui.add_space(5.0);
                            }
                        }
                    }
                });

                if let Some(dir) = exclude_dir_to_add {
                    self.dup_excludes = core_cleaner::add_duplicate_exclude(&dir);
                    self.cleaner_status =
                        core_cleaner::filter_duplicates_by_excludes_in_cached_scan();
                    self.show_toast(format!(
                        "🛡 {}: {}",
                        tr("Added to duplicate excludes", "Đã thêm vào danh sách loại trừ"),
                        dir
                    ));
                }
                if rescan_dups {
                    self.trigger_current_cleaner_subtab_scan(true);
                }
            }

            // ================================================================
            // SUBTAB 3: APP UNINSTALLER & ORPHANS (`CleanerSubTab::Apps`)
            // ================================================================
            CleanerSubTab::Apps => {
                // Card 1: Orphaned Background Daemons & LaunchAgents (if any)
                if !self.orphans.is_empty() {
                    egui::Frame::none()
                        .fill(p.bg_card)
                        .stroke(Stroke::new(1.2_f32, p.danger))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "👻 {} ({})",
                                        tr(
                                            "Orphaned Background Processes & LaunchAgents",
                                            "Phát hiện Tiến trình / LaunchAgent mồ côi"
                                        ),
                                        self.orphans.len()
                                    ))
                                    .size(13.5)
                                    .strong()
                                    .color(p.danger),
                                );
                                tip_badge(
                                    ui,
                                    tr(
                                        "Background helpers or LaunchAgent plists whose parent .app bundle was already deleted.",
                                        "Các tiến trình chạy ngầm hoặc LaunchAgent tự khởi động mà ứng dụng gốc trong /Applications đã bị xóa.",
                                    ),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if danger_button(
                                            ui,
                                            &format!(
                                                "🔥 {} ({})",
                                                tr(
                                                    "Kill & Clean Orphans",
                                                    "Diệt & Dọn sạch mục mồ côi"
                                                ),
                                                self.orphans.len()
                                            ),
                                        )
                                        .clicked()
                                        {
                                            self.cleaner_confirm_action =
                                                Some(CleanerConfirmAction::CleanOrphans {
                                                    count: self.orphans.len(),
                                                });
                                        }
                                    },
                                );
                            });
                            thin_divider(ui);
                            for o in &self.orphans {
                                setting_row_split(
                                    ui,
                                    |ui| {
                                        let path_str = o
                                            .plist_path
                                            .as_deref()
                                            .filter(|s| !s.is_empty())
                                            .unwrap_or(&o.target_path);
                                        if let Some(cp) = path_action_cell(ui, path_str, true) {
                                            copied_path_toast = Some(cp);
                                        }
                                    },
                                    |ui| {
                                        truncated_name_label(ui, &o.name, 28, 12.0, p.danger);
                                        if let Some(pid) = o.pid {
                                            badge_pill(
                                                ui,
                                                &format!("PID {}", pid),
                                                p.warning,
                                                p.warning_soft,
                                            );
                                        }
                                        badge_pill(
                                            ui,
                                            &translate_cleaner_category(&o.kind),
                                            p.warning,
                                            p.warning_soft,
                                        );
                                    },
                                );
                                thin_divider(ui);
                            }
                        });
                    ui.add_space(8.0);
                }

                // Card 2: Installed Applications Table
                card_frame().show(ui, |ui| {
                    let apps = self
                        .cleaner_status
                        .apps_result
                        .clone()
                        .unwrap_or_default();
                    let q = self.app_search_filter.trim().to_lowercase();
                    let filtered_apps: Vec<core_cleaner::InstalledAppInfo> = apps
                        .iter()
                        .filter(|a| {
                            q.is_empty()
                                || a.name.to_lowercase().contains(&q)
                                || a.bundle_id.to_lowercase().contains(&q)
                                || a.app_path.to_lowercase().contains(&q)
                        })
                        .cloned()
                        .collect();

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "📦 {} ({}/{})",
                                tr("Installed Applications", "Ứng dụng đã cài đặt"),
                                filtered_apps.len(),
                                apps.len()
                            ))
                            .size(13.5)
                            .strong()
                            .color(p.text_primary),
                        );
                        tip_badge(
                            ui,
                            tr(
                                "Uninstalling an app here automatically quits running processes, removes the .app bundle, and cleans all associated Library caches, preferences, and containers.",
                                "Gỡ cài đặt tại đây sẽ tự động đóng tiến trình đang chạy, xóa gói .app và dọn sạch toàn bộ dữ liệu phụ (Application Support, Caches, Preferences, Containers) trong ~/Library.",
                            ),
                        );
                        ui.add_sized(
                            [240.0, 26.0],
                            egui::TextEdit::singleline(&mut self.app_search_filter).hint_text(tr(
                                "🔍 Search app name, Bundle ID...",
                                "🔍 Tìm tên App, Bundle ID...",
                            )),
                        );

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.checkbox(
                                    &mut self.cleaner_also_empty_trash,
                                    RichText::new(tr(
                                        "Also empty Trash permanently",
                                        "Kết hợp đổ sạch Thùng rác (Xóa vĩnh viễn)",
                                    ))
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_secondary),
                                );
                            },
                        );
                    });

                    thin_divider(ui);

                    let mut uninstall_target: Option<core_cleaner::InstalledAppInfo> = None;

                    for app in filtered_apps {
                        if !self.app_icon_textures.contains_key(&app.bundle_id) {
                            if self.app_icon_textures.len() > 400 {
                                self.app_icon_textures.clear();
                            }
                            let loaded_tex = if !app.icon_data_url.is_empty() {
                                decode_data_url_or_path_to_rgba8_max_size(&app.icon_data_url, 64)
                                    .map(|(iw, ih, rgba)| {
                                        let img =
                                            ColorImage::from_rgba_unmultiplied([iw, ih], &rgba);
                                        ctx.load_texture(
                                            format!("app_icon_{}", app.bundle_id),
                                            img,
                                            TextureOptions::LINEAR,
                                        )
                                    })
                            } else {
                                None
                            };
                            self.app_icon_textures
                                .insert(app.bundle_id.clone(), loaded_tex);
                        }

                        ui.horizontal(|ui| {
                            if let Some(Some(tex)) = self.app_icon_textures.get(&app.bundle_id) {
                                ui.add(
                                    egui::Image::new((tex.id(), egui::vec2(32.0, 32.0)))
                                        .rounding(Rounding::same(7.0)),
                                );
                            } else {
                                let (r, _) = ui.allocate_exact_size(
                                    egui::vec2(32.0, 32.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(r, Rounding::same(7.0), p.bg_input);
                                ui.painter().text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "📦",
                                    FontId::proportional(15.0),
                                    p.accent_text,
                                );
                            }

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.5;
                                ui.horizontal(|ui| {
                                    truncated_name_label(ui, &app.name, 26, 13.0, p.text_primary);
                                    if !app.running_pids.is_empty() {
                                        badge_pill(
                                            ui,
                                            &format!(
                                                "● {} ({})",
                                                tr("Running", "Đang chạy"),
                                                app.running_pids.len()
                                            ),
                                            p.success,
                                            p.success_soft,
                                        );
                                    }
                                });
                                ui.label(
                                    RichText::new(format!(
                                        "v{} • {} {:.1} MB • {} {}",
                                        app.version,
                                        tr("Total", "Tổng"),
                                        app.total_size_mb,
                                        app.related_files.len(),
                                        tr("items", "mục")
                                    ))
                                    .size(11.0)
                                    .color(p.text_muted),
                                );
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if danger_button(ui, tr("Uninstall", "Gỡ bỏ")).clicked() {
                                        uninstall_target = Some(app.clone());
                                    }
                                    if let Some(cp) = path_action_cell(ui, &app.app_path, true) {
                                        copied_path_toast = Some(cp);
                                    }
                                    if secondary_button(
                                        ui,
                                        &format!(
                                            "{}: {:.1} MB ▾",
                                            tr("Leftovers", "Rác phụ"),
                                            (app.leftovers_size_bytes as f64) / (1024.0 * 1024.0)
                                        ),
                                    )
                                    .on_hover_text(tr(
                                        "Click to view related Library leftover files",
                                        "Nhấn để xem danh sách file rác đi kèm trong ~/Library",
                                    ))
                                    .clicked()
                                    {
                                        if self.cleaner_expanded_app_bundle.as_deref()
                                            == Some(&app.bundle_id)
                                        {
                                            self.cleaner_expanded_app_bundle = None;
                                        } else {
                                            self.cleaner_expanded_app_bundle =
                                                Some(app.bundle_id.clone());
                                        }
                                    }
                                    badge_pill(
                                        ui,
                                        &format!("App: {:.1} MB", app.app_size_mb),
                                        p.text_secondary,
                                        p.bg_input,
                                    );
                                },
                            );
                        });

                        if self.cleaner_expanded_app_bundle.as_deref() == Some(&app.bundle_id) {
                            sub_card_frame().show(ui, |ui| {
                                for rf in &app.related_files {
                                    ui.horizontal(|ui| {
                                        badge_pill(
                                            ui,
                                            &translate_cleaner_category(&rf.kind),
                                            p.accent_text,
                                            p.accent_soft,
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if let Some(cp) =
                                                    path_action_cell(ui, &rf.path, true)
                                                {
                                                    copied_path_toast = Some(cp);
                                                }
                                                ui.label(
                                                    RichText::new(format_bytes(rf.size_bytes))
                                                        .size(11.0)
                                                        .strong()
                                                        .color(p.warning),
                                                );
                                            },
                                        );
                                    });
                                }
                            });
                        }

                        thin_divider(ui);
                    }

                    if let Some(target) = uninstall_target {
                        self.cleaner_confirm_action = Some(CleanerConfirmAction::UninstallApp {
                            app: target,
                            also_empty_trash: self.cleaner_also_empty_trash,
                        });
                    }
                });
            }

            // ================================================================
            // SUBTAB 4: LARGE FILES (`CleanerSubTab::LargeFiles`)
            // ================================================================
            CleanerSubTab::LargeFiles => {
                let mut rescan_large = false;
                let mut single_delete_target: Option<(String, f64)> = None;

                card_frame().show(ui, |ui| {
                    let files = self
                        .cleaner_status
                        .large_result
                        .clone()
                        .unwrap_or_default();

                    ui.horizontal_wrapped(|ui| {
                        let scopes = [
                            ("all", tr("User Home (~)", "Thư mục người dùng (~)")),
                            ("downloads", "~/Downloads"),
                            ("documents", "~/Documents"),
                            ("movies", "~/Movies"),
                            ("desktop", "~/Desktop"),
                        ];
                        ComboBox::from_id_salt("large_scope_combo")
                            .width(175.0)
                            .truncate()
                            .selected_text(
                                scopes
                                    .iter()
                                    .find(|(k, _)| *k == self.large_scope)
                                    .map(|(_, v)| *v)
                                    .unwrap_or("~/"),
                            )
                            .show_ui(ui, |ui| {
                                ui.set_min_width(220.0);
                                for (k, lbl) in scopes {
                                    if ui
                                        .selectable_value(
                                            &mut self.large_scope,
                                            k.to_string(),
                                            lbl,
                                        )
                                        .changed()
                                    {
                                        self.selected_large_paths.clear();
                                        rescan_large = true;
                                    }
                                }
                            });

                        let min_mbs = [
                            (20u64, "> 20 MB"),
                            (50, "> 50 MB"),
                            (100, "> 100 MB"),
                            (500, "> 500 MB"),
                            (1024, "> 1 GB"),
                        ];
                        ComboBox::from_id_salt("large_min_mb_combo")
                            .width(115.0)
                            .truncate()
                            .selected_text(
                                min_mbs
                                    .iter()
                                    .find(|(k, _)| *k == self.large_min_mb)
                                    .map(|(_, v)| *v)
                                    .unwrap_or("> 50 MB"),
                            )
                            .show_ui(ui, |ui| {
                                ui.set_min_width(140.0);
                                for (k, lbl) in min_mbs {
                                    if ui
                                        .selectable_value(&mut self.large_min_mb, k, lbl)
                                        .changed()
                                    {
                                        self.selected_large_paths.clear();
                                        rescan_large = true;
                                    }
                                }
                            });

                        badge_pill(
                            ui,
                            &format!("{} {}", files.len(), tr("files", "tệp tin")),
                            p.accent_text,
                            p.accent_soft,
                        );

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let sel_count = self.selected_large_paths.len();
                                let sel_mb: f64 = files
                                    .iter()
                                    .filter(|f| self.selected_large_paths.contains(&f.path))
                                    .map(|f| f.size_mb)
                                    .sum();
                                let del_btn_lbl = if sel_count > 0 {
                                    format!(
                                        "🗑 {} ({} • {:.1} MB)",
                                        tr("Delete Selected", "Xóa mục đã chọn"),
                                        sel_count,
                                        sel_mb
                                    )
                                } else {
                                    format!(
                                        "🗑 {} (0)",
                                        tr("Delete Selected", "Xóa mục đã chọn")
                                    )
                                };
                                if danger_button(ui, &del_btn_lbl).clicked() {
                                    if sel_count == 0 {
                                        self.show_toast(tr(
                                            "Please select at least 1 large file to delete.",
                                            "Vui lòng tích chọn ít nhất 1 tệp lớn để xóa.",
                                        ));
                                    } else {
                                        let paths: Vec<String> =
                                            self.selected_large_paths.iter().cloned().collect();
                                        self.cleaner_confirm_action =
                                            Some(CleanerConfirmAction::DeleteLargeFiles {
                                                paths,
                                                total_mb: sel_mb,
                                                also_empty_trash: self.cleaner_also_empty_trash,
                                            });
                                    }
                                }

                                ui.checkbox(
                                    &mut self.cleaner_also_empty_trash,
                                    RichText::new(tr(
                                        "Empty Trash (Permanent)",
                                        "Đổ luôn Trash (Xóa vĩnh viễn)",
                                    ))
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_secondary),
                                );
                            },
                        );
                    });

                    thin_divider(ui);

                    // Master Checkbox Header Row
                    let all_checked = !files.is_empty()
                        && files
                            .iter()
                            .all(|f| self.selected_large_paths.contains(&f.path));
                    let mut toggle_all = all_checked;
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(
                                &mut toggle_all,
                                RichText::new(tr("Select All Files", "Chọn tất cả tệp"))
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_muted),
                            )
                            .changed()
                        {
                            if toggle_all {
                                for f in &files {
                                    self.selected_large_paths.insert(f.path.clone());
                                }
                            } else {
                                self.selected_large_paths.clear();
                            }
                        }
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(tr(
                                        "Size  •  Path (Copy / Finder)  •  Delete",
                                        "Dung lượng  •  Đường dẫn (Copy / Mở Finder)  •  Xóa",
                                    ))
                                    .size(11.0)
                                    .color(p.text_muted),
                                );
                            },
                        );
                    });

                    thin_divider(ui);

                    if files.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                RichText::new(format!(
                                    "✨ {} (> {} MB)",
                                    tr(
                                        "No large files found in this folder",
                                        "Không tìm thấy tệp lớn nào trong phạm vi này"
                                    ),
                                    self.large_min_mb
                                ))
                                .size(12.5)
                                .color(p.text_secondary),
                            );
                            ui.add_space(20.0);
                        });
                    } else {
                        for f in &files {
                            setting_row_split(
                                ui,
                                |ui| {
                                    if danger_button(ui, "🗑")
                                        .on_hover_text(tr(
                                            "Delete file (with confirmation)",
                                            "Xóa tệp này (có xác nhận trước khi xóa)",
                                        ))
                                        .clicked()
                                    {
                                        single_delete_target = Some((f.path.clone(), f.size_mb));
                                    }
                                    if let Some(cp) = path_action_cell(ui, &f.path, true) {
                                        copied_path_toast = Some(cp);
                                    }
                                    ui.label(
                                        RichText::new(format!("{:.1} MB", f.size_mb))
                                            .size(12.0)
                                            .strong()
                                            .color(p.warning),
                                    );
                                },
                                |ui| {
                                    let mut checked =
                                        self.selected_large_paths.contains(&f.path);
                                    if ui.checkbox(&mut checked, "").changed() {
                                        if checked {
                                            self.selected_large_paths.insert(f.path.clone());
                                        } else {
                                            self.selected_large_paths.remove(&f.path);
                                        }
                                    }
                                    truncated_name_label(ui, &f.name, 26, 12.5, p.text_primary);
                                    badge_pill(
                                        ui,
                                        &translate_cleaner_category(&f.kind),
                                        p.success,
                                        p.success_soft,
                                    );
                                },
                            );
                            thin_divider(ui);
                        }
                    }
                });

                if let Some((path, size_mb)) = single_delete_target {
                    self.cleaner_confirm_action =
                        Some(CleanerConfirmAction::DeleteLargeFiles {
                            paths: vec![path],
                            total_mb: size_mb,
                            also_empty_trash: self.cleaner_also_empty_trash,
                        });
                }
                if rescan_large {
                    self.trigger_current_cleaner_subtab_scan(true);
                }
            }
        }

        if let Some(cp) = copied_path_toast {
            self.show_toast(format!("📋 {}: {}", tr("Copied path", "Đã copy"), cp));
        }

        self.render_cleaner_confirm_modal(ctx);
    }

    fn render_cleaner_confirm_modal(&mut self, ctx: &egui::Context) {
        let Some(action) = self.cleaner_confirm_action.clone() else {
            return;
        };

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cleaner_confirm_action = None;
            return;
        }

        let p = palette();

        // Subtle backdrop dimmer
        let screen_rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("cleaner_confirm_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (rect, resp) =
                    ui.allocate_exact_size(screen_rect.size(), Sense::click());
                ui.painter()
                    .rect_filled(rect, Rounding::ZERO, egui::Color32::from_black_alpha(110));
                if resp.clicked() {
                    self.cleaner_confirm_action = None;
                }
            });

        let mut confirm_now = false;
        let mut cancel_now = false;
        let mut updated_also_empty = self.cleaner_also_empty_trash;

        egui::Window::new(tr("⚠ Confirm Action", "⚠ Xác nhận thao tác xóa"))
            .id(egui::Id::new("cleaner_confirm_modal_window"))
            .order(egui::Order::Tooltip)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .fixed_size(egui::vec2(450.0, 0.0))
            .frame(
                egui::Frame::none()
                    .fill(p.bg_card)
                    .stroke(Stroke::new(1.5_f32, p.danger))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::symmetric(18.0, 16.0)),
            )
            .show(ctx, |ui| {
                ui.set_width(414.0);
                match &action {
                    CleanerConfirmAction::EmptyTrash {
                        item_count,
                        total_mb,
                    } => {
                        ui.label(
                            RichText::new(tr(
                                "🔥 Permanently Empty macOS Trash?",
                                "🔥 Xác nhận đổ sạch Thùng rác macOS?",
                            ))
                            .size(15.0)
                            .strong()
                            .color(p.danger),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "{} {} ({:.1} MB) {}",
                                tr("All", "Toàn bộ"),
                                item_count,
                                total_mb,
                                tr(
                                    "items in ~/.Trash will be permanently deleted and cannot be recovered.",
                                    "mục trong ~/.Trash sẽ bị xóa vĩnh viễn và không thể khôi phục."
                                )
                            ))
                            .size(12.5)
                            .color(p.text_primary),
                        );
                    }
                    CleanerConfirmAction::CleanJunk {
                        paths,
                        total_mb,
                        also_empty_trash,
                    } => {
                        updated_also_empty = *also_empty_trash;
                        ui.label(
                            RichText::new(tr(
                                "🗑 Confirm Cleaning System Junk & Caches",
                                "🗑 Xác nhận dọn Bộ nhớ đệm & File Rác",
                            ))
                            .size(15.0)
                            .strong()
                            .color(p.text_primary),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "{}: {} {} • {:.1} MB",
                                tr("Selected to clean", "Đã chọn dọn dẹp"),
                                paths.len(),
                                tr("items", "mục"),
                                total_mb
                            ))
                            .size(12.5)
                            .strong()
                            .color(p.warning),
                        );
                        ui.add_space(6.0);
                        Self::render_preview_paths_list(ui, paths);
                        ui.add_space(8.0);
                        if ui
                            .checkbox(
                                &mut updated_also_empty,
                                RichText::new(tr(
                                    "Also empty macOS Trash permanently",
                                    "Kết hợp đổ sạch Thùng rác (Xóa vĩnh viễn)",
                                ))
                                .size(12.0)
                                .strong()
                                .color(p.text_primary),
                            )
                            .changed()
                        {
                            self.cleaner_also_empty_trash = updated_also_empty;
                            self.cleaner_confirm_action = Some(CleanerConfirmAction::CleanJunk {
                                paths: paths.clone(),
                                total_mb: *total_mb,
                                also_empty_trash: updated_also_empty,
                            });
                        }
                    }
                    CleanerConfirmAction::DeleteDuplicates {
                        paths,
                        total_mb,
                        also_empty_trash,
                    } => {
                        updated_also_empty = *also_empty_trash;
                        ui.label(
                            RichText::new(tr(
                                "🗑 Confirm Deleting Duplicate Files",
                                "🗑 Xác nhận xóa các Tệp trùng lặp đã chọn",
                            ))
                            .size(15.0)
                            .strong()
                            .color(p.text_primary),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "{}: {} {} • {:.1} MB",
                                tr("Selected copies", "Số bản sao sẽ xóa"),
                                paths.len(),
                                tr("files", "tệp"),
                                total_mb
                            ))
                            .size(12.5)
                            .strong()
                            .color(p.warning),
                        );
                        ui.add_space(6.0);
                        Self::render_preview_paths_list(ui, paths);
                        ui.add_space(8.0);
                        if ui
                            .checkbox(
                                &mut updated_also_empty,
                                RichText::new(tr(
                                    "Also empty macOS Trash permanently",
                                    "Kết hợp đổ sạch Thùng rác (Xóa vĩnh viễn)",
                                ))
                                .size(12.0)
                                .strong()
                                .color(p.text_primary),
                            )
                            .changed()
                        {
                            self.cleaner_also_empty_trash = updated_also_empty;
                            self.cleaner_confirm_action =
                                Some(CleanerConfirmAction::DeleteDuplicates {
                                    paths: paths.clone(),
                                    total_mb: *total_mb,
                                    also_empty_trash: updated_also_empty,
                                });
                        }
                    }
                    CleanerConfirmAction::DeleteLargeFiles {
                        paths,
                        total_mb,
                        also_empty_trash,
                    } => {
                        updated_also_empty = *also_empty_trash;
                        ui.label(
                            RichText::new(if paths.len() == 1 {
                                tr(
                                    "🗑 Confirm Deleting Large File",
                                    "🗑 Xác nhận xóa Tệp dung lượng lớn",
                                )
                            } else {
                                tr(
                                    "🗑 Confirm Deleting Selected Large Files",
                                    "🗑 Xác nhận xóa các Tệp lớn đã chọn",
                                )
                            })
                            .size(15.0)
                            .strong()
                            .color(p.text_primary),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "{}: {} {} • {:.1} MB",
                                tr("Selected", "Đã chọn"),
                                paths.len(),
                                tr("files", "tệp"),
                                total_mb
                            ))
                            .size(12.5)
                            .strong()
                            .color(p.warning),
                        );
                        ui.add_space(6.0);
                        Self::render_preview_paths_list(ui, paths);
                        ui.add_space(8.0);
                        if ui
                            .checkbox(
                                &mut updated_also_empty,
                                RichText::new(tr(
                                    "Also empty macOS Trash permanently",
                                    "Kết hợp đổ sạch Thùng rác (Xóa vĩnh viễn)",
                                ))
                                .size(12.0)
                                .strong()
                                .color(p.text_primary),
                            )
                            .changed()
                        {
                            self.cleaner_also_empty_trash = updated_also_empty;
                            self.cleaner_confirm_action =
                                Some(CleanerConfirmAction::DeleteLargeFiles {
                                    paths: paths.clone(),
                                    total_mb: *total_mb,
                                    also_empty_trash: updated_also_empty,
                                });
                        }
                    }
                    CleanerConfirmAction::UninstallApp {
                        app,
                        also_empty_trash,
                    } => {
                        updated_also_empty = *also_empty_trash;
                        ui.label(
                            RichText::new(format!(
                                "📦 {} \"{}\"?",
                                tr("Completely Uninstall", "Gỡ cài đặt tận gốc"),
                                app.name
                            ))
                            .size(15.0)
                            .strong()
                            .color(p.danger),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "• Bundle ID: {}\n• {}: {:.1} MB ({} {})",
                                app.bundle_id,
                                tr("Total size", "Tổng dung lượng"),
                                app.total_size_mb,
                                app.related_files.len(),
                                tr("related items", "mục liên quan")
                            ))
                            .size(12.0)
                            .color(p.text_primary),
                        );
                        ui.add_space(8.0);
                        if ui
                            .checkbox(
                                &mut updated_also_empty,
                                RichText::new(tr(
                                    "Also empty macOS Trash permanently",
                                    "Kết hợp đổ sạch Thùng rác (Xóa vĩnh viễn)",
                                ))
                                .size(12.0)
                                .strong()
                                .color(p.text_primary),
                            )
                            .changed()
                        {
                            self.cleaner_also_empty_trash = updated_also_empty;
                            self.cleaner_confirm_action =
                                Some(CleanerConfirmAction::UninstallApp {
                                    app: app.clone(),
                                    also_empty_trash: updated_also_empty,
                                });
                        }
                    }
                    CleanerConfirmAction::CleanOrphans { count } => {
                        ui.label(
                            RichText::new(tr(
                                "👻 Kill & Clean Orphaned Background Items?",
                                "👻 Xác nhận diệt & dọn sạch Tiến trình mồ côi?",
                            ))
                            .size(15.0)
                            .strong()
                            .color(p.danger),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "{} {} {}",
                                tr("Will terminate and remove", "Sẽ dừng và dọn sạch"),
                                count,
                                tr(
                                    "orphaned background processes / LaunchAgents.",
                                    "tiến trình ngầm / LaunchAgent mồ côi."
                                )
                            ))
                            .size(12.5)
                            .color(p.text_primary),
                        );
                    }
                }

                ui.add_space(12.0);
                thin_divider(ui);
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if danger_button(
                            ui,
                            tr("🗑 Confirm Delete", "🗑 Xác nhận xóa ngay"),
                        )
                        .clicked()
                        {
                            confirm_now = true;
                        }
                        ui.add_space(6.0);
                        if secondary_button(ui, tr("Cancel", "Hủy bỏ")).clicked() {
                            cancel_now = true;
                        }
                    });
                });
            });

        if cancel_now {
            self.cleaner_confirm_action = None;
            return;
        }

        if confirm_now {
            self.cleaner_confirm_action = None;
            match action {
                CleanerConfirmAction::EmptyTrash { .. } => {
                    match core_cleaner::empty_macos_trash() {
                        Ok(msg) => {
                            self.cleaner_status =
                                core_cleaner::remove_deleted_paths_from_cached_scan(&[], true);
                            self.show_toast(format!("✓ {}", msg));
                        }
                        Err(err) => self.show_toast(format!("✕ {}", err)),
                    }
                }
                CleanerConfirmAction::CleanJunk {
                    paths,
                    also_empty_trash,
                    ..
                } => match core_cleaner::clean_selected_paths_ext(&paths, also_empty_trash) {
                    Ok(msg) => {
                        for p_str in &paths {
                            self.selected_junk_paths.remove(p_str);
                        }
                        self.cleaner_status = core_cleaner::remove_deleted_paths_from_cached_scan(
                            &paths,
                            also_empty_trash,
                        );
                        self.show_toast(format!("✓ {}", msg));
                    }
                    Err(err) => self.show_toast(format!("✕ {}", err)),
                },
                CleanerConfirmAction::DeleteDuplicates {
                    paths,
                    also_empty_trash,
                    ..
                } => match core_cleaner::clean_selected_paths_ext(&paths, also_empty_trash) {
                    Ok(msg) => {
                        for p_str in &paths {
                            self.selected_dup_paths.remove(p_str);
                        }
                        self.cleaner_status = core_cleaner::remove_deleted_paths_from_cached_scan(
                            &paths,
                            also_empty_trash,
                        );
                        self.show_toast(format!("✓ {}", msg));
                    }
                    Err(err) => self.show_toast(format!("✕ {}", err)),
                },
                CleanerConfirmAction::DeleteLargeFiles {
                    paths,
                    also_empty_trash,
                    ..
                } => match core_cleaner::clean_selected_paths_ext(&paths, also_empty_trash) {
                    Ok(msg) => {
                        for p_str in &paths {
                            self.selected_large_paths.remove(p_str);
                        }
                        self.cleaner_status = core_cleaner::remove_deleted_paths_from_cached_scan(
                            &paths,
                            also_empty_trash,
                        );
                        self.show_toast(format!("✓ {}", msg));
                    }
                    Err(err) => self.show_toast(format!("✕ {}", err)),
                },
                CleanerConfirmAction::UninstallApp {
                    app,
                    also_empty_trash,
                } => {
                    let paths: Vec<String> = app
                        .related_files
                        .iter()
                        .map(|f| f.path.clone())
                        .collect();
                    match core_cleaner::uninstall_application(
                        &app.app_path,
                        &app.name,
                        &app.bundle_id,
                        &paths,
                        also_empty_trash,
                    ) {
                        Ok(msg) => {
                            self.cleaner_status =
                                core_cleaner::remove_uninstalled_app_from_cached_scan(
                                    &app.bundle_id,
                                    &app.app_path,
                                    also_empty_trash,
                                );
                            self.show_toast(format!("✓ {}", msg));
                        }
                        Err(err) => self.show_toast(format!("✕ {}", err)),
                    }
                }
                CleanerConfirmAction::CleanOrphans { .. } => {
                    match core_cleaner::clean_orphaned_background_items() {
                        Ok(msg) => self.show_toast(format!("✓ {}", msg)),
                        Err(err) => self.show_toast(format!("✕ {}", err)),
                    }
                    self.orphans = core_cleaner::scan_orphaned_background_items();
                }
            }
        }
    }

    fn render_preview_paths_list(ui: &mut egui::Ui, paths: &[String]) {
        let p = palette();
        sub_card_frame().show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for path_str in paths.iter().take(5) {
                let file_name = path_str
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(path_str);
                let short_name = shorten_middle(file_name, 38);
                ui.label(
                    RichText::new(format!("• {}", short_name))
                        .size(11.5)
                        .color(p.text_secondary),
                )
                .on_hover_text(path_str);
            }
            if paths.len() > 5 {
                ui.label(
                    RichText::new(format!(
                        "... {} {} {}",
                        tr("and", "và"),
                        paths.len() - 5,
                        tr("more items", "mục khác")
                    ))
                    .size(11.0)
                    .italics()
                    .color(p.text_muted),
                );
            }
        });
    }
}
