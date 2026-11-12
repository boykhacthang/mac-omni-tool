use crate::events::{AppUserEvent, MAX_SCREENSHOT_SLOTS};
use crate::gpu_ui::{danger_button, palette, primary_button, secondary_button};
use crate::i18n::tr;
use crate::macos_window::encode_rgba8_to_png;
use egui::{
    Color32, ColorImage, FontId, Key, Margin, Pos2, Rect, Rounding, Sense, Stroke,
    TextureHandle, TextureOptions, Vec2,
};
use egui_winit::winit::event_loop::EventLoopProxy;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnnotationTool {
    Select,
    Rect,
    Arrow,
    Line,
    Pen,
    Text,
    Mosaic,
}

#[derive(Clone)]
pub enum AnnotationItem {
    Rect {
        rect: Rect,
        color: Color32,
        width: f32,
    },
    Line {
        start: Pos2,
        end: Pos2,
        color: Color32,
        width: f32,
    },
    Arrow {
        start: Pos2,
        end: Pos2,
        color: Color32,
        width: f32,
    },
    Pen {
        points: Vec<Pos2>,
        color: Color32,
        width: f32,
    },
    Mosaic {
        rect: Rect,
    },
    Text {
        pos: Pos2,
        text: String,
        color: Color32,
        size: f32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

#[derive(Clone, Copy)]
enum DragAction {
    CreateNew(Pos2),
    MoveSelection(Vec2),
    Resize { handle: ResizeHandle, initial: Rect },
}

pub struct ScreenshotSlotUi {
    pub slot_idx: usize,
    pub display_name: String,
    pub logical_w: f32,
    pub logical_h: f32,
    pub pixel_w: usize,
    pub pixel_h: usize,
    pub raw_rgba: Vec<u8>,
    pub raw_png_bytes: Vec<u8>,
    pub bg_texture: Option<TextureHandle>,
    pub selection: Option<Rect>,
    pub fullscreen_picker_mode: bool,
    pub tool: AnnotationTool,
    pub stroke_color: Color32,
    pub stroke_width: f32,
    pub annotations: Vec<AnnotationItem>,
    pub active_annotation: Option<AnnotationItem>,
    active_annotation_anchor: Option<Pos2>,
    drag_action: Option<DragAction>,
    pub text_input_pos: Option<Pos2>,
    pub text_input_buf: String,
    pub toolbar_size: Vec2,
    pub last_toolbar_rect: Option<Rect>,
}

impl ScreenshotSlotUi {
    pub fn new(slot_idx: usize) -> Self {
        Self {
            slot_idx,
            display_name: format!("Display #{}", slot_idx + 1),
            logical_w: 1440.0,
            logical_h: 900.0,
            pixel_w: 0,
            pixel_h: 0,
            raw_rgba: Vec::new(),
            raw_png_bytes: Vec::new(),
            bg_texture: None,
            selection: None,
            fullscreen_picker_mode: false,
            tool: AnnotationTool::Rect,
            stroke_color: Color32::from_rgb(239, 68, 68),
            stroke_width: 3.5,
            annotations: Vec::new(),
            active_annotation: None,
            active_annotation_anchor: None,
            drag_action: None,
            text_input_pos: None,
            text_input_buf: String::new(),
            toolbar_size: egui::vec2(760.0, 42.0),
            last_toolbar_rect: None,
        }
    }

    pub fn load_session_slot(
        &mut self,
        display_name: String,
        logical_w: usize,
        logical_h: usize,
        pixel_w: usize,
        pixel_h: usize,
        raw_rgba: Vec<u8>,
        preselect_fullscreen: bool,
        fullscreen_picker_mode: bool,
    ) {
        self.clear_memory();
        self.display_name = display_name;
        self.logical_w = logical_w.max(1) as f32;
        self.logical_h = logical_h.max(1) as f32;
        self.fullscreen_picker_mode = fullscreen_picker_mode;
        self.pixel_w = pixel_w;
        self.pixel_h = pixel_h;
        self.raw_rgba = raw_rgba;

        if preselect_fullscreen {
            self.selection = Some(Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(self.logical_w, self.logical_h),
            ));
            self.tool = AnnotationTool::Rect;
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.last_toolbar_rect = None;
        self.annotations.clear();
        self.active_annotation = None;
        self.active_annotation_anchor = None;
        self.drag_action = None;
        self.text_input_pos = None;
        self.text_input_buf.clear();
        self.tool = AnnotationTool::Rect;
    }

    /// Release all GPU textures and CPU RGBA/PNG buffers immediately when hidden.
    pub fn clear_memory(&mut self) {
        self.bg_texture = None;
        self.raw_rgba = Vec::new();
        self.raw_png_bytes = Vec::new();
        self.clear_selection();
    }

    pub fn render(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<AppUserEvent>) {
        // Keyboard shortcuts: Escape to close, Enter / Cmd+C to copy, Cmd+Z to undo
        let mut trigger_copy = false;
        let mut trigger_save = false;
        let mut trigger_save_as = false;
        ctx.input(|i| {
            if i.key_pressed(Key::Escape)
                || (i.modifiers.command && (i.key_pressed(Key::W) || i.key_pressed(Key::Q)))
            {
                if self.text_input_pos.is_some() {
                    self.text_input_pos = None;
                    self.text_input_buf.clear();
                } else {
                    let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
                }
            } else if i.key_pressed(Key::Enter) && self.text_input_pos.is_none() && self.selection.is_some() {
                trigger_copy = true;
            } else if i.modifiers.command && i.key_pressed(Key::C) && self.selection.is_some() {
                trigger_copy = true;
            } else if i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::S) && self.selection.is_some() {
                trigger_save_as = true;
            } else if i.modifiers.command && i.key_pressed(Key::S) && self.selection.is_some() {
                trigger_save = true;
            } else if i.modifiers.command && i.key_pressed(Key::Z) {
                self.annotations.pop();
            }
        });

        if trigger_copy {
            self.export_and_finish(true, false, proxy);
            return;
        }
        if trigger_save_as {
            self.export_and_finish(false, true, proxy);
            return;
        }
        if trigger_save {
            self.export_and_finish(false, false, proxy);
            return;
        }

        // Ensure background texture is uploaded to Metal
        if self.bg_texture.is_none() && self.pixel_w > 0 && self.pixel_h > 0 && !self.raw_rgba.is_empty() {
            let img = ColorImage::from_rgba_unmultiplied([self.pixel_w, self.pixel_h], &self.raw_rgba);
            self.bg_texture = Some(ctx.load_texture(
                format!("screenshot_bg_{}", self.slot_idx),
                img,
                TextureOptions::LINEAR,
            ));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let screen_rect = ui.max_rect();
                if screen_rect.width() > 10.0 && screen_rect.height() > 10.0 {
                    self.logical_w = screen_rect.width();
                    self.logical_h = screen_rect.height();
                }

                // 1. Draw full-screen captured desktop image
                if let Some(tex) = &self.bg_texture {
                    ui.painter().image(
                        tex.id(),
                        screen_rect,
                        Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // 2. Canvas pointer interaction:
                // While dragging a region or drawing an annotation, the toolbar is hidden and
                // `in_toolbar` is strictly false so mouse-move and mouse-release events are NEVER swallowed.
                let is_dragging_canvas =
                    self.drag_action.is_some() || self.active_annotation.is_some();
                if is_dragging_canvas {
                    self.last_toolbar_rect = None;
                }
                let toolbar_rect = self.compute_toolbar_rect(screen_rect);
                let pointer_pos =
                    ctx.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.latest_pos()));
                let in_toolbar = !is_dragging_canvas
                    && (pointer_pos
                        .map(|pt| {
                            toolbar_rect.map_or(false, |tr| tr.expand(3.0).contains(pt))
                                || self
                                    .last_toolbar_rect
                                    .map_or(false, |lr| lr.expand(3.0).contains(pt))
                        })
                        .unwrap_or(false)
                        || self.text_input_pos.is_some());

                if !in_toolbar {
                    self.handle_canvas_pointer(ctx, screen_rect, proxy);
                }

                // 3. Draw dark dimming mask outside selection
                let dim_color = Color32::from_rgba_unmultiplied(2, 6, 23, 145);
                if let Some(sel) = self.selection {
                    let s = sel.intersect(screen_rect);
                    // Top strip
                    if s.min.y > screen_rect.min.y {
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                screen_rect.min,
                                egui::pos2(screen_rect.max.x, s.min.y),
                            ),
                            Rounding::ZERO,
                            dim_color,
                        );
                    }
                    // Bottom strip
                    if s.max.y < screen_rect.max.y {
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                egui::pos2(screen_rect.min.x, s.max.y),
                                screen_rect.max,
                            ),
                            Rounding::ZERO,
                            dim_color,
                        );
                    }
                    // Left strip
                    if s.min.x > screen_rect.min.x {
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                egui::pos2(screen_rect.min.x, s.min.y),
                                egui::pos2(s.min.x, s.max.y),
                            ),
                            Rounding::ZERO,
                            dim_color,
                        );
                    }
                    // Right strip
                    if s.max.x < screen_rect.max.x {
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                egui::pos2(s.max.x, s.min.y),
                                egui::pos2(screen_rect.max.x, s.max.y),
                            ),
                            Rounding::ZERO,
                            dim_color,
                        );
                    }
                } else {
                    ui.painter()
                        .rect_filled(screen_rect, Rounding::ZERO, dim_color);
                    // Only show a compact top badge in fullscreen display-picker mode (no intrusive center banner during region selection)
                    if self.fullscreen_picker_mode {
                        let hint = format!(
                            "🖥 {} — {}",
                            self.display_name,
                            tr(
                                "Click to select Fullscreen (Esc to cancel)",
                                "Nhấp chuột để chọn Toàn màn hình (Esc để hủy)"
                            )
                        );
                        let top_center = egui::pos2(screen_rect.center().x, screen_rect.top() + 28.0);
                        let pill_rect =
                            Rect::from_center_size(top_center, egui::vec2(420.0, 32.0));
                        ui.painter().rect_filled(
                            pill_rect,
                            Rounding::same(8.0),
                            Color32::from_rgba_unmultiplied(15, 23, 42, 215),
                        );
                        ui.painter().text(
                            top_center,
                            egui::Align2::CENTER_CENTER,
                            hint,
                            FontId::proportional(12.5),
                            Color32::WHITE,
                        );
                    }
                }

                // 4. Draw committed annotations + active in-progress annotation
                for item in &self.annotations {
                    self.paint_annotation(ui.painter(), item);
                }
                if let Some(active) = &self.active_annotation {
                    self.paint_annotation(ui.painter(), active);
                }

                // 5. Draw selection border, 8 resize handles, and W×H dimension pill
                if let Some(sel) = self.selection {
                    ui.painter().rect_stroke(
                        sel,
                        Rounding::ZERO,
                        Stroke::new(2.0_f32, Color32::from_rgb(59, 130, 246)),
                    );

                    // 8 resize handles
                    for (_, h_rect) in Self::resize_handle_rects(sel) {
                        ui.painter().rect_filled(
                            h_rect,
                            Rounding::same(2.0),
                            Color32::WHITE,
                        );
                        ui.painter().rect_stroke(
                            h_rect,
                            Rounding::same(2.0),
                            Stroke::new(1.5_f32, Color32::from_rgb(37, 99, 235)),
                        );
                    }

                    // Dimension pill above top-left of selection
                    let dim_text = format!(
                        "{} × {} px  •  {}",
                        sel.width().round() as i32,
                        sel.height().round() as i32,
                        self.display_name
                    );
                    let pill_y = if sel.min.y > 32.0 {
                        sel.min.y - 26.0
                    } else {
                        sel.min.y + 6.0
                    };
                    let pill_pos = egui::pos2(sel.min.x.max(8.0), pill_y);
                    let pill_rect = Rect::from_min_size(pill_pos, egui::vec2(220.0, 22.0));
                    ui.painter().rect_filled(
                        pill_rect,
                        Rounding::same(6.0),
                        Color32::from_rgba_unmultiplied(15, 23, 42, 225),
                    );
                    ui.painter().text(
                        pill_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        dim_text,
                        FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                }
            });

        // 6. Floating Text Input Popover (when Text tool clicked on canvas)
        if let Some(tpos) = self.text_input_pos {
            let mut commit_text = false;
            let mut cancel_text = false;
            egui::Area::new(egui::Id::new(("screenshot_text_input", self.slot_idx)))
                .fixed_pos(tpos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 240))
                        .stroke(Stroke::new(1.5_f32, self.stroke_color))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let resp = ui.add_sized(
                                    [200.0, 24.0],
                                    egui::TextEdit::singleline(&mut self.text_input_buf)
                                        .hint_text(tr("Type annotation...", "Nhập chữ chú thích...")),
                                );
                                resp.request_focus();
                                if ui.input(|i| i.key_pressed(Key::Enter))
                                    || primary_button(ui, "OK").clicked()
                                {
                                    commit_text = true;
                                }
                                if secondary_button(ui, "✕").clicked() {
                                    cancel_text = true;
                                }
                            });
                        });
                });
            if commit_text {
                let txt = self.text_input_buf.trim().to_string();
                if !txt.is_empty() {
                    self.annotations.push(AnnotationItem::Text {
                        pos: tpos,
                        text: txt,
                        color: self.stroke_color,
                        size: (self.stroke_width * 5.0).clamp(14.0, 32.0),
                    });
                }
                self.text_input_pos = None;
                self.text_input_buf.clear();
            } else if cancel_text {
                self.text_input_pos = None;
                self.text_input_buf.clear();
            }
        }

        // 7. Floating Annotation & Export Toolbar (only shown AFTER region selection & any drag finishes)
        if self.drag_action.is_none() && self.active_annotation.is_none() {
            if let Some(tb_rect) = self.compute_toolbar_rect(ctx.screen_rect()) {
                let was_hidden = self.last_toolbar_rect.is_none();
                let mut do_copy = false;
                let mut do_save = false;
                let mut do_save_as = false;
                let mut do_cancel = false;

                let area_resp = egui::Area::new(egui::Id::new(("screenshot_toolbar", self.slot_idx)))
                    .fixed_pos(tb_rect.min)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let p = palette();
                        egui::Frame::none()
                            .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 245))
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(Margin::symmetric(10.0, 6.0))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                ui.spacing_mut().button_padding = egui::vec2(8.0, 5.0);
                                ui.horizontal(|ui| {
                                    // Tool buttons (selected AFTER choosing region; default = Rect)
                                    let tools = [
                                        (AnnotationTool::Rect, "▭"),
                                        (AnnotationTool::Arrow, "↗"),
                                        (AnnotationTool::Line, "╱"),
                                        (AnnotationTool::Pen, "✎"),
                                        (AnnotationTool::Text, "T"),
                                        (AnnotationTool::Mosaic, "▦"),
                                        (AnnotationTool::Select, "✥"),
                                    ];
                                    for (t, icon) in tools {
                                        let active = self.tool == t;
                                        let btn = if active {
                                            primary_button(ui, icon)
                                        } else {
                                            secondary_button(ui, icon)
                                        };
                                        if btn.clicked() {
                                            self.tool = t;
                                        }
                                    }

                                    ui.separator();

                                    // Color swatches
                                    let colors = [
                                        Color32::from_rgb(239, 68, 68),   // Red
                                        Color32::from_rgb(245, 158, 11),  // Amber
                                        Color32::from_rgb(34, 197, 94),   // Green
                                        Color32::from_rgb(59, 130, 246),  // Blue
                                        Color32::from_rgb(168, 85, 247),  // Purple
                                        Color32::WHITE,
                                    ];
                                    for col in colors {
                                        let (rect, resp) = ui.allocate_exact_size(
                                            egui::vec2(18.0, 18.0),
                                            Sense::click(),
                                        );
                                        ui.painter().circle_filled(rect.center(), 7.5, col);
                                        if self.stroke_color == col {
                                            ui.painter().circle_stroke(
                                                rect.center(),
                                                8.5,
                                                Stroke::new(2.0_f32, Color32::WHITE),
                                            );
                                        }
                                        if resp.clicked() {
                                            self.stroke_color = col;
                                        }
                                    }

                                    ui.separator();

                                    // Undo & Fullscreen
                                    if secondary_button(ui, "↶").clicked() {
                                        self.annotations.pop();
                                    }
                                    if secondary_button(ui, tr("⛶ Full", "⛶ Toàn màn")).clicked() {
                                        self.selection = Some(Rect::from_min_size(
                                            Pos2::ZERO,
                                            egui::vec2(self.logical_w, self.logical_h),
                                        ));
                                    }

                                    ui.separator();

                                    // Save PNG, Copy, Cancel
                                    let alt_held = ctx.input(|i| i.modifiers.alt);
                                    let cfg = core_input::get_input_config();
                                    let save_tip = if cfg.screenshot_ask_where_to_save {
                                        tr(
                                            "Lưu ảnh PNG (Cmd+S) • Mở hộp thoại chọn vị trí lưu và đặt tên",
                                            "Save PNG (Cmd+S) • Open dialog to choose location and name",
                                        )
                                        .to_string()
                                    } else {
                                        format!(
                                            "{} {} (Cmd+S) • {}",
                                            tr("Lưu vào", "Save to"),
                                            cfg.screenshot_save_directory,
                                            tr("Giữ Option hoặc Cmd+Shift+S để chọn vị trí", "Hold Option or Cmd+Shift+S to choose folder"),
                                        )
                                    };
                                    if secondary_button(ui, tr("💾 Save", "💾 Lưu PNG"))
                                        .on_hover_text(save_tip)
                                        .clicked()
                                    {
                                        if alt_held {
                                            do_save_as = true;
                                        } else {
                                            do_save = true;
                                        }
                                    }
                                    if primary_button(ui, tr("📋 Copy ↵", "📋 Copy ↵"))
                                        .on_hover_text(tr(
                                            "Copy vào Clipboard và đóng overlay (Enter / Cmd+C)",
                                            "Copy to Clipboard and close overlay (Enter / Cmd+C)",
                                        ))
                                        .clicked()
                                    {
                                        do_copy = true;
                                    }
                                    if danger_button(ui, "✕ Esc")
                                        .on_hover_text(tr(
                                            "Đóng (Esc / Cmd+W) • Nhấn Esc 3 lần hoặc Ctrl+Shift+Esc để tắt app khẩn cấp",
                                            "Close (Esc / Cmd+W) • Press Esc 3 times or Ctrl+Shift+Esc for emergency quit",
                                        ))
                                        .clicked()
                                    {
                                        do_cancel = true;
                                    }
                                });
                            });
                    });

                let actual_rect = area_resp.response.rect;
                if actual_rect.width() > 100.0 && actual_rect.height() > 20.0 {
                    let size_changed = (self.toolbar_size - actual_rect.size()).length_sq() > 1.0;
                    self.toolbar_size = actual_rect.size();
                    self.last_toolbar_rect = Some(actual_rect);
                    if was_hidden || size_changed {
                        ctx.request_repaint();
                    }
                } else if was_hidden {
                    ctx.request_repaint();
                }

                if do_copy {
                    self.export_and_finish(true, false, proxy);
                } else if do_save_as {
                    self.export_and_finish(false, true, proxy);
                } else if do_save {
                    self.export_and_finish(false, false, proxy);
                } else if do_cancel {
                    let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
                }
            }
        }
    }

    fn compute_toolbar_rect(&self, screen_rect: Rect) -> Option<Rect> {
        if self.drag_action.is_some() || self.active_annotation.is_some() {
            return None;
        }
        let sel = self.selection?;
        if sel.width() < 8.0 || sel.height() < 8.0 {
            return None;
        }
        let tb_w = self.toolbar_size.x.max(720.0);
        let tb_h = self.toolbar_size.y.max(40.0);
        let x = (sel.right().min(screen_rect.right() - 12.0) - tb_w).max(screen_rect.left() + 12.0);
        let y = if sel.bottom() + tb_h + 18.0 <= screen_rect.bottom() {
            sel.bottom() + 12.0
        } else if sel.top() - tb_h - 18.0 >= screen_rect.top() {
            sel.top() - tb_h - 12.0
        } else {
            (sel.bottom() - tb_h - 12.0).max(screen_rect.top() + 12.0)
        };
        Some(Rect::from_min_size(egui::pos2(x, y), egui::vec2(tb_w, tb_h)))
    }

    fn resize_handle_rects(sel: Rect) -> [(ResizeHandle, Rect); 8] {
        let sz = egui::vec2(9.0, 9.0);
        let c = sel.center();
        [
            (
                ResizeHandle::TopLeft,
                Rect::from_center_size(sel.left_top(), sz),
            ),
            (
                ResizeHandle::Top,
                Rect::from_center_size(egui::pos2(c.x, sel.top()), sz),
            ),
            (
                ResizeHandle::TopRight,
                Rect::from_center_size(sel.right_top(), sz),
            ),
            (
                ResizeHandle::Right,
                Rect::from_center_size(egui::pos2(sel.right(), c.y), sz),
            ),
            (
                ResizeHandle::BottomRight,
                Rect::from_center_size(sel.right_bottom(), sz),
            ),
            (
                ResizeHandle::Bottom,
                Rect::from_center_size(egui::pos2(c.x, sel.bottom()), sz),
            ),
            (
                ResizeHandle::BottomLeft,
                Rect::from_center_size(sel.left_bottom(), sz),
            ),
            (
                ResizeHandle::Left,
                Rect::from_center_size(egui::pos2(sel.left(), c.y), sz),
            ),
        ]
    }

    fn finalize_canvas_drag(&mut self, ctx: &egui::Context) {
        let mut changed = false;
        self.active_annotation_anchor = None;
        if let Some(finished) = self.active_annotation.take() {
            self.annotations.push(finished);
            changed = true;
        }
        if let Some(drag) = self.drag_action.take() {
            changed = true;
            if let DragAction::CreateNew(_) = drag {
                if let Some(sel) = self.selection {
                    if sel.width() < 8.0 || sel.height() < 8.0 {
                        // Click without drag selects fullscreen if in fullscreen_picker_mode, else clears
                        if self.fullscreen_picker_mode {
                            self.selection = Some(Rect::from_min_size(
                                Pos2::ZERO,
                                egui::vec2(self.logical_w, self.logical_h),
                            ));
                            self.tool = AnnotationTool::Rect;
                        } else {
                            self.selection = None;
                        }
                    } else {
                        // Region selection completed -> default active tool to Rect (drawing square/rectangle)
                        self.tool = AnnotationTool::Rect;
                    }
                }
            }
        }
        if changed {
            ctx.request_repaint();
        }
    }

    fn handle_canvas_pointer(
        &mut self,
        ctx: &egui::Context,
        screen_rect: Rect,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        let (pressed, down, released, pos) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.pointer.interact_pos().or_else(|| i.pointer.latest_pos()),
            )
        });
        let Some(pos) = pos else {
            if !pressed && !down && (self.drag_action.is_some() || self.active_annotation.is_some())
            {
                self.finalize_canvas_drag(ctx);
            }
            return;
        };

        if pressed {
            let _ = proxy.send_event(AppUserEvent::SetActiveScreenshotSlot(self.slot_idx));

            if self.fullscreen_picker_mode && self.selection.is_none() {
                self.selection = Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(self.logical_w, self.logical_h),
                ));
                self.tool = AnnotationTool::Rect;
                ctx.request_repaint();
                return;
            }

            if let Some(sel) = self.selection {
                // 1. Always allow grabbing the 8 border handles to resize the capture region
                for (handle, h_rect) in Self::resize_handle_rects(sel) {
                    if h_rect.expand(5.0).contains(pos) {
                        self.drag_action = Some(DragAction::Resize {
                            handle,
                            initial: sel,
                        });
                        return;
                    }
                }

                // 2. Inside selection: draw annotation (default = Rect) or move selection if Select tool is active
                if sel.contains(pos) {
                    if self.tool != AnnotationTool::Select {
                        self.active_annotation_anchor = Some(pos);
                        match self.tool {
                            AnnotationTool::Rect => {
                                self.active_annotation = Some(AnnotationItem::Rect {
                                    rect: Rect::from_two_pos(pos, pos),
                                    color: self.stroke_color,
                                    width: self.stroke_width,
                                });
                            }
                            AnnotationTool::Line => {
                                self.active_annotation = Some(AnnotationItem::Line {
                                    start: pos,
                                    end: pos,
                                    color: self.stroke_color,
                                    width: self.stroke_width,
                                });
                            }
                            AnnotationTool::Arrow => {
                                self.active_annotation = Some(AnnotationItem::Arrow {
                                    start: pos,
                                    end: pos,
                                    color: self.stroke_color,
                                    width: self.stroke_width,
                                });
                            }
                            AnnotationTool::Pen => {
                                self.active_annotation = Some(AnnotationItem::Pen {
                                    points: vec![pos],
                                    color: self.stroke_color,
                                    width: self.stroke_width,
                                });
                            }
                            AnnotationTool::Mosaic => {
                                self.active_annotation = Some(AnnotationItem::Mosaic {
                                    rect: Rect::from_two_pos(pos, pos),
                                });
                            }
                            AnnotationTool::Text => {
                                self.text_input_pos = Some(pos);
                                self.text_input_buf.clear();
                            }
                            AnnotationTool::Select => {}
                        }
                        return;
                    } else {
                        self.drag_action = Some(DragAction::MoveSelection(pos - sel.min));
                        return;
                    }
                }
            }

            // 3. Outside existing selection (or no selection yet): start selecting a new region
            self.tool = AnnotationTool::Rect;
            self.last_toolbar_rect = None;
            self.annotations.clear();
            self.drag_action = Some(DragAction::CreateNew(pos));
            self.selection = Some(Rect::from_two_pos(pos, pos));
        } else if down {
            if let Some(active) = &mut self.active_annotation {
                match active {
                    AnnotationItem::Rect { rect, .. } | AnnotationItem::Mosaic { rect } => {
                        let anchor = self.active_annotation_anchor.unwrap_or(rect.min);
                        *rect = Rect::from_two_pos(anchor, pos);
                    }
                    AnnotationItem::Line { end, .. } | AnnotationItem::Arrow { end, .. } => {
                        *end = pos;
                    }
                    AnnotationItem::Pen { points, .. } => {
                        if points.last().map_or(true, |last| last.distance(pos) > 2.0) {
                            points.push(pos);
                        }
                    }
                    AnnotationItem::Text { .. } => {}
                }
                return;
            }

            if let Some(drag) = self.drag_action {
                match drag {
                    DragAction::CreateNew(anchor) => {
                        let r = Rect::from_two_pos(anchor, pos).intersect(screen_rect);
                        self.selection = Some(r);
                    }
                    DragAction::MoveSelection(offset) => {
                        if let Some(sel) = self.selection {
                            let sz = sel.size();
                            let nx = (pos.x - offset.x)
                                .clamp(screen_rect.left(), (screen_rect.right() - sz.x).max(0.0));
                            let ny = (pos.y - offset.y)
                                .clamp(screen_rect.top(), (screen_rect.bottom() - sz.y).max(0.0));
                            self.selection = Some(Rect::from_min_size(egui::pos2(nx, ny), sz));
                        }
                    }
                    DragAction::Resize { handle, initial } => {
                        let mut min = initial.min;
                        let mut max = initial.max;
                        match handle {
                            ResizeHandle::TopLeft => {
                                min.x = pos.x;
                                min.y = pos.y;
                            }
                            ResizeHandle::Top => {
                                min.y = pos.y;
                            }
                            ResizeHandle::TopRight => {
                                max.x = pos.x;
                                min.y = pos.y;
                            }
                            ResizeHandle::Right => {
                                max.x = pos.x;
                            }
                            ResizeHandle::BottomRight => {
                                max.x = pos.x;
                                max.y = pos.y;
                            }
                            ResizeHandle::Bottom => {
                                max.y = pos.y;
                            }
                            ResizeHandle::BottomLeft => {
                                min.x = pos.x;
                                max.y = pos.y;
                            }
                            ResizeHandle::Left => {
                                min.x = pos.x;
                            }
                        }
                        self.selection = Some(Rect::from_two_pos(min, max).intersect(screen_rect));
                    }
                }
            }
        } else if released || self.drag_action.is_some() || self.active_annotation.is_some() {
            self.finalize_canvas_drag(ctx);
        }
    }

    fn paint_annotation(&self, painter: &egui::Painter, item: &AnnotationItem) {
        match item {
            AnnotationItem::Rect { rect, color, width } => {
                let r = Rect::from_two_pos(rect.min, rect.max);
                painter.rect_stroke(
                    r,
                    Rounding::same(2.0),
                    Stroke::new(*width, *color),
                );
            }
            AnnotationItem::Line {
                start,
                end,
                color,
                width,
            } => {
                painter.line_segment([*start, *end], Stroke::new(*width, *color));
            }
            AnnotationItem::Arrow {
                start,
                end,
                color,
                width,
            } => {
                painter.line_segment([*start, *end], Stroke::new(*width, *color));
                let dir = *end - *start;
                let len = dir.length();
                if len > 4.0 {
                    let u = dir / len;
                    let head_len = (width * 4.2).max(12.0);
                    let perp = egui::vec2(-u.y, u.x);
                    let p1 = *end - u * head_len + perp * (head_len * 0.45);
                    let p2 = *end - u * head_len - perp * (head_len * 0.45);
                    painter.line_segment([*end, p1], Stroke::new(*width, *color));
                    painter.line_segment([*end, p2], Stroke::new(*width, *color));
                }
            }
            AnnotationItem::Pen {
                points,
                color,
                width,
            } => {
                if points.len() >= 2 {
                    for w in points.windows(2) {
                        painter.line_segment([w[0], w[1]], Stroke::new(*width, *color));
                    }
                }
            }
            AnnotationItem::Mosaic { rect } => {
                let r = Rect::from_two_pos(rect.min, rect.max);
                let step = 10.0f32;
                let mut y = r.min.y;
                let mut row = 0usize;
                while y < r.max.y {
                    let mut x = r.min.x;
                    let mut col = 0usize;
                    while x < r.max.x {
                        let cell = Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2((x + step).min(r.max.x), (y + step).min(r.max.y)),
                        );
                        let shade = if (row + col) % 2 == 0 {
                            Color32::from_rgba_unmultiplied(30, 41, 59, 215)
                        } else {
                            Color32::from_rgba_unmultiplied(71, 85, 105, 215)
                        };
                        painter.rect_filled(cell, Rounding::ZERO, shade);
                        x += step;
                        col += 1;
                    }
                    y += step;
                    row += 1;
                }
            }
            AnnotationItem::Text {
                pos,
                text,
                color,
                size,
            } => {
                painter.text(
                    *pos,
                    egui::Align2::LEFT_TOP,
                    text,
                    FontId::proportional(*size),
                    *color,
                );
            }
        }
    }

    fn export_and_finish(
        &mut self,
        copy_to_clipboard: bool,
        ask_where_dialog: bool,
        proxy: &EventLoopProxy<AppUserEvent>,
    ) {
        let Some(sel) = self.selection else {
            let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);
            return;
        };

        // Fast path: full-screen unedited screenshot can reuse raw_png_bytes directly!
        let is_full = (sel.min.x.abs() < 1.5)
            && (sel.min.y.abs() < 1.5)
            && ((sel.width() - self.logical_w).abs() < 2.0)
            && ((sel.height() - self.logical_h).abs() < 2.0);

        let png_bytes = if is_full && self.annotations.is_empty() && !self.raw_png_bytes.is_empty()
        {
            Some(self.raw_png_bytes.clone())
        } else {
            self.rasterize_cropped_png(sel)
        };

        // CRITICAL: Dismiss the overlay IMMEDIATELY so the display is completely unlocked and responsive!
        self.clear_selection();
        let _ = proxy.send_event(AppUserEvent::HideScreenshotOverlay);

        if let Some(bytes) = png_bytes {
            std::thread::spawn(move || {
                if copy_to_clipboard {
                    let _ = core_clipboard::copy_png_bytes_to_clipboard(&bytes);
                } else {
                    let cfg = core_input::get_input_config();
                    let ask = ask_where_dialog || cfg.screenshot_ask_where_to_save;
                    let _ = core_clipboard::save_png_bytes_to_file(
                        &bytes,
                        Some(&cfg.screenshot_save_directory),
                        ask,
                    );
                }
            });
        }
    }

    /// Crop `self.raw_rgba` to `sel` at Retina pixel resolution, rasterize all annotations onto it,
    /// and encode to PNG via CoreGraphics `encode_rgba8_to_png`.
    fn rasterize_cropped_png(&self, sel: Rect) -> Option<Vec<u8>> {
        if self.pixel_w == 0 || self.pixel_h == 0 || self.raw_rgba.is_empty() {
            return None;
        }
        let sx = (self.pixel_w as f32 / self.logical_w.max(1.0)).max(1.0);
        let sy = (self.pixel_h as f32 / self.logical_h.max(1.0)).max(1.0);

        let x0 = ((sel.min.x * sx).round() as isize).clamp(0, self.pixel_w as isize - 1) as usize;
        let y0 = ((sel.min.y * sy).round() as isize).clamp(0, self.pixel_h as isize - 1) as usize;
        let x1 = ((sel.max.x * sx).round() as isize).clamp((x0 + 1) as isize, self.pixel_w as isize)
            as usize;
        let y1 = ((sel.max.y * sy).round() as isize).clamp((y0 + 1) as isize, self.pixel_h as isize)
            as usize;

        let cw = x1 - x0;
        let ch = y1 - y0;
        let mut cropped = vec![0u8; cw * ch * 4];

        for row in 0..ch {
            let src_off = ((y0 + row) * self.pixel_w + x0) * 4;
            let dst_off = row * cw * 4;
            cropped[dst_off..dst_off + cw * 4]
                .copy_from_slice(&self.raw_rgba[src_off..src_off + cw * 4]);
        }

        // Rasterize annotations into `cropped`
        for item in &self.annotations {
            match item {
                AnnotationItem::Mosaic { rect } => {
                    let r = Rect::from_two_pos(rect.min, rect.max);
                    let mx0 = (((r.min.x - sel.min.x) * sx).round() as isize).clamp(0, cw as isize)
                        as usize;
                    let my0 = (((r.min.y - sel.min.y) * sy).round() as isize).clamp(0, ch as isize)
                        as usize;
                    let mx1 = (((r.max.x - sel.min.x) * sx).round() as isize).clamp(0, cw as isize)
                        as usize;
                    let my1 = (((r.max.y - sel.min.y) * sy).round() as isize).clamp(0, ch as isize)
                        as usize;
                    let block = ((12.0 * sx).round() as usize).max(6);
                    let mut by = my0;
                    while by < my1 {
                        let by_end = (by + block).min(my1);
                        let mut bx = mx0;
                        while bx < mx1 {
                            let bx_end = (bx + block).min(mx1);
                            let mut sr = 0u64;
                            let mut sg = 0u64;
                            let mut sb = 0u64;
                            let mut cnt = 0u64;
                            for py in by..by_end {
                                for px in bx..bx_end {
                                    let idx = (py * cw + px) * 4;
                                    sr += cropped[idx] as u64;
                                    sg += cropped[idx + 1] as u64;
                                    sb += cropped[idx + 2] as u64;
                                    cnt += 1;
                                }
                            }
                            if cnt > 0 {
                                let ar = (sr / cnt) as u8;
                                let ag = (sg / cnt) as u8;
                                let ab = (sb / cnt) as u8;
                                for py in by..by_end {
                                    for px in bx..bx_end {
                                        let idx = (py * cw + px) * 4;
                                        cropped[idx] = ar;
                                        cropped[idx + 1] = ag;
                                        cropped[idx + 2] = ab;
                                        cropped[idx + 3] = 255;
                                    }
                                }
                            }
                            bx += block;
                        }
                        by += block;
                    }
                }
                AnnotationItem::Rect { rect, color, width } => {
                    let r = Rect::from_two_pos(rect.min, rect.max);
                    let p_tl = egui::pos2((r.left() - sel.left()) * sx, (r.top() - sel.top()) * sy);
                    let p_tr =
                        egui::pos2((r.right() - sel.left()) * sx, (r.top() - sel.top()) * sy);
                    let p_br =
                        egui::pos2((r.right() - sel.left()) * sx, (r.bottom() - sel.top()) * sy);
                    let p_bl =
                        egui::pos2((r.left() - sel.left()) * sx, (r.bottom() - sel.top()) * sy);
                    let w_px = (*width * sx).max(2.0);
                    draw_thick_line(&mut cropped, cw, ch, p_tl, p_tr, *color, w_px);
                    draw_thick_line(&mut cropped, cw, ch, p_tr, p_br, *color, w_px);
                    draw_thick_line(&mut cropped, cw, ch, p_br, p_bl, *color, w_px);
                    draw_thick_line(&mut cropped, cw, ch, p_bl, p_tl, *color, w_px);
                }
                AnnotationItem::Line {
                    start,
                    end,
                    color,
                    width,
                } => {
                    let a = egui::pos2((start.x - sel.left()) * sx, (start.y - sel.top()) * sy);
                    let b = egui::pos2((end.x - sel.left()) * sx, (end.y - sel.top()) * sy);
                    draw_thick_line(&mut cropped, cw, ch, a, b, *color, (*width * sx).max(2.0));
                }
                AnnotationItem::Arrow {
                    start,
                    end,
                    color,
                    width,
                } => {
                    let a = egui::pos2((start.x - sel.left()) * sx, (start.y - sel.top()) * sy);
                    let b = egui::pos2((end.x - sel.left()) * sx, (end.y - sel.top()) * sy);
                    let w_px = (*width * sx).max(2.0);
                    draw_thick_line(&mut cropped, cw, ch, a, b, *color, w_px);
                    let dir = b - a;
                    let len = dir.length();
                    if len > 4.0 {
                        let u = dir / len;
                        let head_len = (w_px * 4.2).max(16.0);
                        let perp = egui::vec2(-u.y, u.x);
                        let p1 = b - u * head_len + perp * (head_len * 0.45);
                        let p2 = b - u * head_len - perp * (head_len * 0.45);
                        draw_thick_line(&mut cropped, cw, ch, b, p1, *color, w_px);
                        draw_thick_line(&mut cropped, cw, ch, b, p2, *color, w_px);
                    }
                }
                AnnotationItem::Pen {
                    points,
                    color,
                    width,
                } => {
                    let w_px = (*width * sx).max(2.0);
                    for pair in points.windows(2) {
                        let a =
                            egui::pos2((pair[0].x - sel.left()) * sx, (pair[0].y - sel.top()) * sy);
                        let b =
                            egui::pos2((pair[1].x - sel.left()) * sx, (pair[1].y - sel.top()) * sy);
                        draw_thick_line(&mut cropped, cw, ch, a, b, *color, w_px);
                    }
                }
                AnnotationItem::Text { .. } => {}
            }
        }

        encode_rgba8_to_png(cw, ch, &cropped)
    }
}

fn draw_thick_line(
    rgba: &mut [u8],
    w: usize,
    h: usize,
    a: Pos2,
    b: Pos2,
    color: Color32,
    thickness: f32,
) {
    let dist = a.distance(b).max(1.0);
    let steps = (dist * 1.5).ceil() as usize;
    let r = (thickness * 0.5).max(1.0);
    let r_sq = r * r;
    let ir = r.ceil() as isize;
    let [cr, cg, cb, _] = color.to_array();

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let cx = a.x + (b.x - a.x) * t;
        let cy = a.y + (b.y - a.y) * t;
        let ix = cx.round() as isize;
        let iy = cy.round() as isize;
        for dy in -ir..=ir {
            let py = iy + dy;
            if py < 0 || py >= h as isize {
                continue;
            }
            for dx in -ir..=ir {
                let px = ix + dx;
                if px < 0 || px >= w as isize {
                    continue;
                }
                if (dx * dx + dy * dy) as f32 <= r_sq {
                    let idx = (py as usize * w + px as usize) * 4;
                    rgba[idx] = cr;
                    rgba[idx + 1] = cg;
                    rgba[idx + 2] = cb;
                    rgba[idx + 3] = 255;
                }
            }
        }
    }
}

pub struct ScreenshotOverlayManager {
    pub slots: Vec<ScreenshotSlotUi>,
}

impl ScreenshotOverlayManager {
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(MAX_SCREENSHOT_SLOTS);
        for idx in 0..MAX_SCREENSHOT_SLOTS {
            slots.push(ScreenshotSlotUi::new(idx));
        }
        Self { slots }
    }

    pub fn clear_all(&mut self) {
        for s in &mut self.slots {
            s.clear_memory();
        }
    }

    pub fn clear_other_selections(&mut self, active_slot: usize) {
        for (idx, s) in self.slots.iter_mut().enumerate() {
            if idx != active_slot {
                s.clear_selection();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rasterize_cropped_png_and_toolbar_rect_cover_copy_button() {
        let mut slot = ScreenshotSlotUi::new(0);
        let logical_w = 200usize;
        let logical_h = 150usize;
        let pixel_w = 400usize;
        let pixel_h = 300usize;
        let mut rgba = vec![0u8; pixel_w * pixel_h * 4];
        for px in rgba.chunks_exact_mut(4) {
            px[0] = 37;
            px[1] = 99;
            px[2] = 235;
            px[3] = 255;
        }
        slot.load_session_slot(
            "Built-in Retina Display".to_string(),
            logical_w,
            logical_h,
            pixel_w,
            pixel_h,
            rgba,
            false,
            false,
        );
        slot.selection = Some(Rect::from_min_max(
            egui::pos2(20.0, 20.0),
            egui::pos2(120.0, 100.0),
        ));

        let png = slot
            .rasterize_cropped_png(slot.selection.unwrap())
            .expect("rasterize_cropped_png should return valid PNG bytes");
        assert!(png.len() > 64);
        assert_eq!(&png[0..4], &[0x89, b'P', b'N', b'G']);

        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0));
        // While dragging a region, toolbar must NOT be computed or intercept pointer release
        slot.drag_action = Some(DragAction::CreateNew(egui::pos2(20.0, 20.0)));
        assert!(slot.compute_toolbar_rect(screen).is_none());

        // Once mouse is released (`finalize_canvas_drag`), toolbar immediately appears
        let ctx = egui::Context::default();
        slot.finalize_canvas_drag(&ctx);
        assert!(slot.drag_action.is_none());
        let tb_rect = slot
            .compute_toolbar_rect(screen)
            .expect("compute_toolbar_rect should return Some after drag finishes");
        assert!(tb_rect.width() >= 720.0);
        assert!(tb_rect.right() <= screen.right());
        assert!(!tb_rect.expand(3.0).contains(egui::pos2(120.0, 100.0)));
    }
}
