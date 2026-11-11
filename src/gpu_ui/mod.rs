pub mod dashboard;
pub mod overlays;
pub mod screenshot;
mod tab_cleaner;
mod tab_mouse;
mod tab_settings;

use crate::i18n::{is_light_theme, tr};
use egui::{
    Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Rect, RichText, Rounding,
    Sense, Stroke, Style, Visuals,
};
use egui_wgpu::wgpu;
use egui_winit::winit::{event::WindowEvent, window::Window};
use std::sync::Arc;

/// Shared WGPU Metal device, queue, and Vietnamese-capable system font definitions.
/// All windows in the single process share this single `wgpu::Device` & `wgpu::Queue`
/// so there is zero duplicated shader/pipeline/font state across windows.
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub fonts: FontDefinitions,
    raw_queue_arc: Option<Arc<std::ffi::c_void>>,
}

impl GpuContext {
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("Failed to acquire Apple Metal GPU adapter");

        let mut raw_queue_arc: Option<Arc<std::ffi::c_void>> = None;
        let open_dev = unsafe {
            adapter.as_hal::<wgpu::hal::api::Metal, _, _>(|hal_adapter| {
                let hal_adapter = hal_adapter.expect("Metal HAL adapter unavailable");
                let mut open = wgpu::hal::Adapter::open(
                    hal_adapter,
                    wgpu::Features::empty(),
                    &wgpu::Limits::default(),
                    &wgpu::MemoryHints::MemoryUsage,
                )
                .expect("Failed to open Metal HAL adapter");
                let new_q = open
                    .device
                    .raw_device()
                    .lock()
                    .new_command_queue_with_max_command_buffer_count(16);
                open.queue = wgpu::hal::metal::Queue::queue_from_raw(new_q, 1.0);
                // `wgpu::hal::metal::Queue` is `{ raw: Arc<Mutex<metal::CommandQueue>>, timestamp_period: f32 }`.
                // Clone the `Arc` at offset 0 so we can swap the underlying `MTLCommandQueue` after rendering
                // to release Apple's ~165 MB `AGXMetal` kernel command-queue tile ringbuffers when idle.
                let arc_ref = &*(&open.queue as *const wgpu::hal::metal::Queue
                    as *const Arc<std::ffi::c_void>);
                raw_queue_arc = Some(arc_ref.clone());
                open
            })
        };

        let (device, queue) = unsafe {
            adapter
                .create_device_from_hal::<wgpu::hal::api::Metal>(
                    open_dev,
                    &wgpu::DeviceDescriptor {
                        label: Some("MacOmniTool Shared Metal Device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                    },
                    None,
                )
                .expect("Failed to create Apple Metal GPU device")
        };

        let fonts = load_macos_vietnamese_fonts();

        Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            fonts,
            raw_queue_arc,
        }
    }

    /// Reclaims ~165 MB of Apple Silicon `AGXMetal` kernel tile/scene ringbuffers (`Owned physical footprint (unmapped) (graphics)`)
    /// that macOS wires onto a `MTLCommandQueue` once a 3D render pass is committed.
    /// By swapping in a fresh lightweight `MTLCommandQueue` and flushing `wgpu`'s `signal_fence` after frame completion,
    /// the old `MTLCommandQueue` and its 165 MB kernel ringbuffer pool are immediately returned to macOS while keeping
    /// all `wgpu::Device` pipelines, shaders, font atlases, and `CAMetalLayer` drawables 100% intact.
    pub fn trim_command_queue(&self) {
        let _ = self.device.poll(wgpu::Maintain::Wait);
        unsafe {
            self.device.as_hal::<wgpu::hal::api::Metal, _, _>(|hal_dev| {
                if let (Some(d), Some(q_arc)) = (hal_dev, &self.raw_queue_arc) {
                    let new_q = d
                        .raw_device()
                        .lock()
                        .new_command_queue_with_max_command_buffer_count(16);
                    let dev_mutex_ref = d.raw_device();
                    let q_mutex_ref = &*(Arc::as_ptr(q_arc) as *const _);
                    if false {
                        let _ = [dev_mutex_ref, q_mutex_ref];
                    }
                    let mut q_guard = q_mutex_ref.lock();
                    let new_q_md = std::mem::ManuallyDrop::new(new_q);
                    let new_q_transmuted = std::ptr::read(&*new_q_md as *const _ as *const _);
                    let old_q = std::mem::replace(&mut *q_guard, new_q_transmuted);
                    drop(q_guard);
                    drop(old_q);
                }
            });
        }
        self.queue.submit(std::iter::empty());
        let _ = self.device.poll(wgpu::Maintain::Wait);
    }
}

/// Load lightweight macOS system fonts (< 2 MB total, shared via `Arc<FontData>` inside `GpuContext`)
/// so Vietnamese diacritics (`ắ, ề, ộ, ư, đ...`), macOS symbols (`⌘, ⌥, ⌃, ⇧, ❖, ✓, ✕, ✎, ❐`),
/// and emoji render with crisp, well-spaced glyphs and instant (< 10ms) startup.
pub fn load_macos_vietnamese_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    // Primary Proportional font: Verdana.ttf (241 KB, static screen-optimized font with wide letter-spacing
    // and full Vietnamese coverage so characters never collide or look cramped at 11pt-14pt),
    // with Arial.ttf as fallback.
    let proportional_candidates = [
        ("macos_verdana", "/System/Library/Fonts/Supplemental/Verdana.ttf"),
        ("macos_arial", "/System/Library/Fonts/Supplemental/Arial.ttf"),
    ];
    for (name, path) in proportional_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_string(), FontData::from_owned(bytes).into());
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
                fam.insert(0, name.to_string());
            }
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
                fam.push(name.to_string());
            }
            break;
        }
    }

    // Primary Monospace font: Monaco.ttf (668 KB, static regular-weight 400 font with full Vietnamese coverage,
    // unlike SFNSMono.ttf which is a variable font whose default unvaried master is thin weight 294.7).
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Monaco.ttf") {
        fonts
            .font_data
            .insert("macos_monaco".to_string(), FontData::from_owned(bytes).into());
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.insert(0, "macos_monaco".to_string());
        }
    }

    // Lightweight Dingbats fallback (150 KB: ✓, ✕, ✎, ❐, ❖, ✂, ✥)
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/ZapfDingbats.ttf") {
        fonts.font_data.insert(
            "macos_dingbats".to_string(),
            FontData::from_owned(bytes).into(),
        );
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.push("macos_dingbats".to_string());
        }
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.push("macos_dingbats".to_string());
        }
    }

    // Apple Symbols fallback (874 KB: ⌘, ⌥, ⌃, ⇧, technical & geometric symbols)
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Apple Symbols.ttf") {
        fonts.font_data.insert(
            "macos_symbols".to_string(),
            FontData::from_owned(bytes).into(),
        );
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.push("macos_symbols".to_string());
        }
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.push("macos_symbols".to_string());
        }
    }

    // Ensure built-in emoji fonts are also available in Monospace
    if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
        if !fam.iter().any(|f| f == "NotoEmoji-Regular") {
            fam.push("NotoEmoji-Regular".to_string());
        }
        if !fam.iter().any(|f| f == "emoji-icon-font") {
            fam.push("emoji-icon-font".to_string());
        }
    }

    fonts
}

/// On-demand CJK (Japanese/Chinese/Korean) fallback font loader.
/// Attaches `Arial Unicode.ttf` to `ctx` only if `text` actually contains CJK characters,
/// keeping app startup and screenshot overlay initialization instant (< 10ms).
pub fn ensure_cjk_font_if_needed(ctx: &egui::Context, text: &str) {
    let has_cjk = text.chars().any(|c| {
        matches!(
            c as u32,
            0x3000..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF
        )
    });
    if !has_cjk {
        return;
    }
    let cjk_id = egui::Id::new("omni_cjk_font_loaded");
    let already_loaded = ctx.data(|d| d.get_temp::<bool>(cjk_id).unwrap_or(false));
    if already_loaded {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(cjk_id, true));

    static CJK_FONT_DATA: std::sync::OnceLock<Option<FontData>> = std::sync::OnceLock::new();
    let font_opt = CJK_FONT_DATA.get_or_init(|| {
        std::fs::read("/System/Library/Fonts/Supplemental/Arial Unicode.ttf")
            .ok()
            .map(FontData::from_owned)
    });
    if let Some(font_data) = font_opt {
        let mut defs = load_macos_vietnamese_fonts();
        defs.font_data
            .insert("macos_arial_unicode".to_string(), font_data.clone());
        if let Some(fam) = defs.families.get_mut(&FontFamily::Proportional) {
            fam.push("macos_arial_unicode".to_string());
        }
        if let Some(fam) = defs.families.get_mut(&FontFamily::Monospace) {
            fam.push("macos_arial_unicode".to_string());
        }
        ctx.set_fonts(defs);
    }
}

/// Sanitize UI and Clipboard strings for `egui`'s `ab_glyph` text shaper:
/// 1. Strips `\r`, Unicode Variation Selectors (`\u{FE0F}`, `\u{FE0E}`), ZWJ (`\u{200D}`), and zero-width spaces
///    which `ab_glyph` otherwise renders as a missing-glyph `□` box.
/// 2. Expands `\t` to 4 spaces and converts Regional Indicator flag codepoints (`\u{1F1E6}..=\u{1F1FF}`)
///    into readable ASCII country letters (`A..Z`) since `ab_glyph` does not perform flag ligature shaping.
/// 3. Maps unsupported emoji codepoints to crisp glyphs guaranteed to exist in `NotoEmoji-Regular.ttf` / `Arial Unicode.ttf`.
pub fn sanitize_ui_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\r' | '\u{FE0F}' | '\u{FE0E}' | '\u{200D}' | '\u{200B}' | '\u{FEFF}' => continue,
            '\t' => out.push_str("    "),
            '\u{1F1E6}'..='\u{1F1FF}' => {
                let ascii = (b'A' + (ch as u32 - 0x1F1E6) as u8) as char;
                out.push(ascii);
            }
            '🧹' => out.push('🗑'),
            '🧠' => out.push('⚡'),
            '🧭' => out.push('🌐'),
            '🦊' => out.push('🔥'),
            '🦁' => out.push('🛡'),
            '🧩' | '🎛' => out.push('❖'),
            '🧪' => out.push('⚡'),
            '👯' => out.push('📑'),
            '🗂' | '🪟' => out.push('❐'),
            '🐘' => out.push('📦'),
            '🟢' | '🔴' => out.push('●'),
            '👁' => out.push('🔍'),
            '🔼' => out.push('▲'),
            '🔽' => out.push('▼'),
            '🌡' => out.push('☀'),
            '🎙' => out.push('🔊'),
            '⏻' => out.push('⊗'),
            _ => out.push(ch),
        }
    }
    out
}

/// Per-window GPU rendering surface and `egui` state.
/// Supports suspending/dropping its `wgpu::Surface` (`CAMetalLayer` swapchain textures),
/// `egui_wgpu::Renderer` (GPU font atlas & buffers), and `ab_glyph` font caches
/// when the window is hidden to the background so hidden windows consume ~0 MB of VRAM/RAM.
pub struct WindowGpuSurface {
    pub window: Arc<Window>,
    pub surface: Option<wgpu::Surface<'static>>,
    pub config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub renderer: Option<egui_wgpu::Renderer>,
    pub fonts_loaded: bool,
    pub transparent: bool,
    pub last_theme_light: Option<bool>,
}

impl WindowGpuSurface {
    pub fn new(gpu: &GpuContext, window: Arc<Window>, transparent: bool, create_surface_now: bool) -> Self {
        let egui_ctx = egui::Context::default();
        let viewport_id = egui_ctx.viewport_id();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            Some(1024),
        );

        let surface_format = wgpu::TextureFormat::Bgra8Unorm;
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 1,
            alpha_mode: if transparent {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else {
                wgpu::CompositeAlphaMode::Auto
            },
            view_formats: vec![],
        };

        let mut this = Self {
            window,
            surface: None,
            config,
            surface_format,
            egui_ctx,
            egui_state,
            renderer: None,
            fonts_loaded: false,
            transparent,
            last_theme_light: None,
        };

        if create_surface_now {
            this.ensure_surface(gpu);
        }

        this
    }

    /// Lazily create and configure the Metal `wgpu::Surface`, `egui_wgpu::Renderer`,
    /// and system font definitions when the window becomes visible.
    pub fn ensure_surface(&mut self, gpu: &GpuContext) {
        if !self.fonts_loaded {
            self.egui_ctx.set_fonts(gpu.fonts.clone());
            apply_omni_theme(&self.egui_ctx);
            self.last_theme_light = Some(is_light_theme());
            self.fonts_loaded = true;
        }
        if self.surface.is_none() {
            if let Ok(surface) = gpu.instance.create_surface(self.window.clone()) {
                let caps = surface.get_capabilities(&gpu.adapter);
                let format = caps
                    .formats
                    .iter()
                    .copied()
                    .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm || *f == wgpu::TextureFormat::Rgba8Unorm)
                    .or_else(|| caps.formats.first().copied())
                    .unwrap_or(self.surface_format);
                let alpha_mode = if self.transparent
                    && caps
                        .alpha_modes
                        .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
                {
                    wgpu::CompositeAlphaMode::PostMultiplied
                } else if self.transparent
                    && caps
                        .alpha_modes
                        .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
                {
                    wgpu::CompositeAlphaMode::PreMultiplied
                } else {
                    caps.alpha_modes
                        .first()
                        .copied()
                        .unwrap_or(wgpu::CompositeAlphaMode::Auto)
                };

                let size = self.window.inner_size();
                self.config.format = format;
                self.config.alpha_mode = alpha_mode;
                self.config.width = size.width.max(1);
                self.config.height = size.height.max(1);
                surface.configure(&gpu.device, &self.config);
                unsafe {
                    crate::macos_window::optimize_nswindow_metal_layer(&self.window);
                }
                self.surface = Some(surface);
            }
        }
        if self.renderer.is_none() {
            self.renderer = Some(egui_wgpu::Renderer::new(
                &gpu.device,
                self.config.format,
                None,
                1,
                false,
            ));
        }
    }

    /// Drop the Metal `wgpu::Surface` (`CAMetalLayer` swapchain drawables), `egui_wgpu::Renderer`
    /// (GPU textures/buffers), and `egui::Context` caches when the window is hidden to the background
    /// so hidden windows consume 0 MB of swapchain `IOSurface`s, 0 MB of `wgpu::Texture`s, and 0 MB of CPU caches.
    pub fn suspend_surface(&mut self, gpu: &GpuContext) {
        if self.surface.is_none() && self.renderer.is_none() && !self.fonts_loaded {
            return;
        }
        self.surface = None;
        self.renderer = None;
        self.egui_ctx = egui::Context::default();
        let viewport_id = self.egui_ctx.viewport_id();
        self.egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            viewport_id,
            self.window.as_ref(),
            Some(self.window.scale_factor() as f32),
            None,
            Some(1024),
        );
        self.fonts_loaded = false;
        self.last_theme_light = None;
        gpu.trim_command_queue();
        unsafe {
            crate::macos_window::release_nswindow_metal_layer(&self.window);
        }
        crate::macos_window::release_process_heap_memory();
    }

    pub fn resize(&mut self, gpu: &GpuContext, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        if let Some(surface) = &self.surface {
            surface.configure(&gpu.device, &self.config);
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        if matches!(event, WindowEvent::RedrawRequested) {
            return false;
        }
        let is_dpi_or_resize = matches!(
            event,
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
        );
        if !is_dpi_or_resize && !self.window.is_visible().unwrap_or(false) {
            return false;
        }
        let resp = self.egui_state.on_window_event(&self.window, event);
        if resp.repaint && self.window.is_visible().unwrap_or(false) {
            self.window.request_redraw();
        }
        resp.consumed
    }

    /// Render a single frame using `egui` + `egui-wgpu` onto the window's Metal `CAMetalLayer`.
    /// Returns the shortest requested `repaint_delay` so the `winit` event loop can schedule `ControlFlow::WaitUntil`.
    pub fn render_frame<F>(
        &mut self,
        gpu: &GpuContext,
        clear_rgba: [f32; 4],
        mut build_ui: F,
    ) -> std::time::Duration
    where
        F: FnMut(&egui::Context),
    {
        self.ensure_surface(gpu);
        let cur_light = is_light_theme();
        if self.last_theme_light != Some(cur_light) {
            apply_omni_theme(&self.egui_ctx);
            self.last_theme_light = Some(cur_light);
        }

        let win_size = self.window.inner_size();
        if win_size.width == 0 || win_size.height == 0 {
            return std::time::Duration::MAX;
        }
        if win_size.width != self.config.width || win_size.height != self.config.height {
            self.resize(gpu, win_size.width, win_size.height);
        }

        let (Some(surface), Some(renderer)) = (&self.surface, &mut self.renderer) else {
            return std::time::Duration::MAX;
        };

        let output_frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                surface.configure(&gpu.device, &self.config);
                match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => return std::time::Duration::MAX,
                }
            }
            Err(_) => return std::time::Duration::MAX,
        };

        let scale_factor = (self.window.scale_factor() as f32).max(1.0);
        let mut raw_input = self.egui_state.take_egui_input(&self.window);
        // Never let macOS system NSAppearance override our explicit app theme slot inside egui!
        raw_input.system_theme = None;
        if let Some(vp) = raw_input.viewports.get_mut(&self.egui_ctx.viewport_id()) {
            vp.native_pixels_per_point = Some(scale_factor);
        }
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            build_ui(ctx);
        });

        let min_repaint_delay = full_output
            .viewport_output
            .values()
            .map(|v| v.repaint_delay)
            .min()
            .unwrap_or(std::time::Duration::MAX);

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let ppp = full_output.pixels_per_point.max(1.0);
        let tris = self.egui_ctx.tessellate(full_output.shapes, ppp);

        for (id, image_delta) in &full_output.textures_delta.set {
            renderer.update_texture(&gpu.device, &gpu.queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: ppp,
        };

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MacOmniTool Egui Encoder"),
            });

        renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        let view = output_frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MacOmniTool Egui RenderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_rgba[0] as f64,
                            g: clear_rgba[1] as f64,
                            b: clear_rgba[2] as f64,
                            a: clear_rgba[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut rpass_static = rpass.forget_lifetime();
            renderer.render(&mut rpass_static, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            renderer.free_texture(id);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output_frame.present();
        gpu.trim_command_queue();

        if min_repaint_delay.is_zero() {
            self.window.request_redraw();
        }
        min_repaint_delay
    }
}

// ============================================================================
// Theme Palette & Custom Reusable Widgets (100% Solid High-Contrast Colors)
// ============================================================================

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg_primary: Color32,
    pub bg_sidebar: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_input: Color32,
    pub border: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub accent_border: Color32,
    pub success: Color32,
    pub success_soft: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
}

pub fn palette() -> Palette {
    if is_light_theme() {
        // High-contrast 100% solid Light Mode palette
        Palette {
            bg_primary: Color32::from_rgb(241, 245, 249),    // #f1f5f9
            bg_sidebar: Color32::from_rgb(255, 255, 255),    // #ffffff
            bg_card: Color32::from_rgb(255, 255, 255),       // #ffffff
            bg_card_hover: Color32::from_rgb(226, 232, 240), // #e2e8f0
            bg_input: Color32::from_rgb(241, 245, 249),      // #f1f5f9
            border: Color32::from_rgb(203, 213, 225),        // #cbd5e1
            text_primary: Color32::from_rgb(15, 23, 42),     // #0f172a
            text_secondary: Color32::from_rgb(51, 65, 85),   // #334155
            text_muted: Color32::from_rgb(100, 116, 139),    // #64748b
            accent: Color32::from_rgb(37, 99, 235),          // #2563eb
            accent_text: Color32::from_rgb(29, 78, 216),     // #1d4ed8
            accent_soft: Color32::from_rgb(219, 234, 254),   // #dbeafe (100% opaque)
            accent_border: Color32::from_rgb(96, 165, 250),  // #60a5fa
            success: Color32::from_rgb(5, 150, 105),         // #059669
            success_soft: Color32::from_rgb(209, 250, 229),  // #d1fae5 (100% opaque)
            warning: Color32::from_rgb(217, 119, 6),         // #d97706
            warning_soft: Color32::from_rgb(254, 243, 199),  // #fef3c7 (100% opaque)
            danger: Color32::from_rgb(220, 38, 38),          // #dc2626
            danger_soft: Color32::from_rgb(254, 226, 226),   // #fee2e2 (100% opaque)
        }
    } else {
        // High-contrast 100% solid Dark Mode palette (Zero washed-out transparency!)
        Palette {
            bg_primary: Color32::from_rgb(15, 17, 23),        // #0f1117
            bg_sidebar: Color32::from_rgb(20, 24, 33),        // #141821
            bg_card: Color32::from_rgb(28, 33, 46),           // #1c212e
            bg_card_hover: Color32::from_rgb(44, 52, 72),     // #2c3448
            bg_input: Color32::from_rgb(21, 25, 36),          // #151924 (solid & distinct)
            border: Color32::from_rgb(56, 66, 90),            // #38425a (crisp visible border)
            text_primary: Color32::from_rgb(243, 244, 246),   // #f3f4f6
            text_secondary: Color32::from_rgb(182, 190, 204), // #b6becc
            text_muted: Color32::from_rgb(126, 136, 155),     // #7e889b
            accent: Color32::from_rgb(59, 130, 246),          // #3b82f6
            accent_text: Color32::from_rgb(110, 176, 255),    // #6eb0ff
            accent_soft: Color32::from_rgb(28, 52, 98),       // #1c3462 (100% opaque navy tint)
            accent_border: Color32::from_rgb(59, 130, 246),   // #3b82f6
            success: Color32::from_rgb(16, 185, 129),         // #10b981
            success_soft: Color32::from_rgb(18, 62, 52),      // #123e34 (100% opaque emerald)
            warning: Color32::from_rgb(245, 158, 11),         // #f59e0b
            warning_soft: Color32::from_rgb(72, 50, 20),      // #483214 (100% opaque amber)
            danger: Color32::from_rgb(239, 68, 68),           // #ef4444
            danger_soft: Color32::from_rgb(74, 28, 34),       // #4a1c22 (100% opaque crimson)
        }
    }
}

pub fn apply_omni_theme(ctx: &egui::Context) {
    let p = palette();
    let mut style = Style::default();
    let mut visuals = if is_light_theme() {
        Visuals::light()
    } else {
        Visuals::dark()
    };

    visuals.panel_fill = p.bg_primary;
    visuals.window_fill = p.bg_card;
    visuals.window_stroke = Stroke::new(1.0_f32, p.border);
    visuals.faint_bg_color = p.bg_input;
    visuals.extreme_bg_color = p.bg_input;
    visuals.override_text_color = Some(p.text_primary);
    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent_text);

    let r = Rounding::same(6.0);
    visuals.widgets.noninteractive.bg_fill = p.bg_card;
    visuals.widgets.noninteractive.weak_bg_fill = p.bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.noninteractive.rounding = r;

    // Inactive widgets (ComboBox, Slider rail, Checkbox, Button) have solid fill & crisp border
    visuals.widgets.inactive.bg_fill = p.bg_card_hover;
    visuals.widgets.inactive.weak_bg_fill = p.bg_card_hover;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.inactive.rounding = r;

    visuals.widgets.hovered.bg_fill = p.accent_soft;
    visuals.widgets.hovered.weak_bg_fill = p.accent_soft;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.hovered.rounding = r;

    // IMPORTANT: `visuals.widgets.active.fg_stroke.color` is used by `egui` as `visuals.strong_text_color()`
    // for all `RichText::new(...).strong()` calls! It MUST be `p.text_primary` (never hardcoded WHITE),
    // otherwise `.strong()` text becomes white-on-white in Light Mode!
    visuals.widgets.active.bg_fill = p.accent_soft;
    visuals.widgets.active.weak_bg_fill = p.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.active.rounding = r;

    visuals.widgets.open.bg_fill = p.accent_soft;
    visuals.widgets.open.weak_bg_fill = p.accent_soft;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.open.rounding = r;

    style.visuals = visuals;
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(17.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(12.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(11.5, FontFamily::Proportional),
    );
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.5);
    style.spacing.window_margin = Margin::same(14.0);
    // Require holding cursor still for 0.5s before showing tooltips
    style.interaction.tooltip_delay = 0.5;
    style.interaction.show_tooltips_only_when_still = true;
    // Apply to BOTH Dark and Light style slots in egui::Context so no widget ever falls back to black text on dark cards!
    ctx.all_styles_mut(|s| *s = style.clone());
}

/// Standard rounded card container frame (`.card` in `base.css`).
pub fn card_frame() -> egui::Frame {
    let p = palette();
    egui::Frame::none()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0_f32, p.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(14.0, 12.0))
}

/// Compact inner card container frame.
pub fn sub_card_frame() -> egui::Frame {
    let p = palette();
    egui::Frame::none()
        .fill(p.bg_input)
        .stroke(Stroke::new(1.0_f32, p.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(10.0, 8.0))
}

/// Container frame for `.pill-tabs` bar.
pub fn pill_tabs_frame() -> egui::Frame {
    let p = palette();
    egui::Frame::none()
        .fill(p.bg_input)
        .stroke(Stroke::new(1.0_f32, p.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(3.0))
}

/// Render a `.pill-tab` button inside a subtab bar.
pub fn pill_tab_button(ui: &mut egui::Ui, active: bool, label: &str) -> egui::Response {
    let p = palette();
    let clean = sanitize_ui_text(label);
    let fg = if active { p.accent_text } else { p.text_secondary };
    let fill = if active { p.accent_soft } else { p.bg_input };
    let stroke = if active {
        Stroke::new(1.0_f32, p.accent_border)
    } else {
        Stroke::NONE
    };
    let btn = egui::Button::new(RichText::new(clean).size(12.0).strong().color(fg))
        .fill(fill)
        .stroke(stroke)
        .rounding(Rounding::same(6.0));
    ui.add(btn)
}

/// Render an `.app-profile-pill` button (solid background & border even when not hovered).
pub fn profile_pill_button(ui: &mut egui::Ui, active: bool, label: &str) -> egui::Response {
    let p = palette();
    let clean = sanitize_ui_text(label);
    let fg = if active { p.accent_text } else { p.text_primary };
    let fill = if active { p.accent_soft } else { p.bg_card };
    let stroke = if active {
        Stroke::new(1.5_f32, p.accent_border)
    } else {
        Stroke::new(1.0_f32, p.border)
    };
    let btn = egui::Button::new(RichText::new(clean).size(12.0).strong().color(fg))
        .fill(fill)
        .stroke(stroke)
        .rounding(Rounding::same(7.0));
    ui.add(btn)
}

/// Render the circular `?` tooltip badge (`.tip-badge` in `base.css`).
/// Hovering `?` immediately opens a crisp floating tooltip popup.
pub fn tip_badge(ui: &mut egui::Ui, tooltip: &str) {
    if tooltip.is_empty() {
        return;
    }
    let p = palette();
    let clean_tip = sanitize_ui_text(tooltip);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
    let hovered = resp.hovered();
    if ui.is_rect_visible(rect) {
        let bg = if hovered { p.accent } else { p.bg_card_hover };
        let fg = if hovered { Color32::WHITE } else { p.text_secondary };
        let border = if hovered { p.accent } else { p.border };
        ui.painter().circle_filled(rect.center(), 8.5, bg);
        ui.painter()
            .circle_stroke(rect.center(), 8.5, Stroke::new(1.0_f32, border));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "?",
            FontId::proportional(11.0),
            fg,
        );
    }
    resp.on_hover_ui_at_pointer(|ui| {
        ui.set_max_width(300.0);
        ui.label(
            RichText::new(&clean_tip)
                .size(12.0)
                .color(p.text_primary),
        );
    });
}

/// Middle-truncate a long filename, process name, or identifier by keeping its prefix and suffix
/// (including the file extension) and replacing the middle with `...`.
/// Example: `"D1DB8E47-2561-4067-ADE4-789782839658.VMRS"` -> `"D1DB8E47-256...39658.VMRS"`.
pub fn shorten_middle(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars || max_chars <= 6 {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let tail_len = (keep * 2 / 5).max(4);
    let head_len = keep.saturating_sub(tail_len);
    let head: String = chars[..head_len].iter().collect();
    let tail: String = chars[chars.len() - tail_len..].iter().collect();
    format!("{}...{}", head, tail)
}

/// Render a bold filename / item name label in a table row using middle-truncation (`head...tail`)
/// and attach a hover tooltip showing the full original name whenever truncated.
pub fn truncated_name_label(
    ui: &mut egui::Ui,
    full_name: &str,
    max_chars: usize,
    font_size: f32,
    color: Color32,
) {
    let short = shorten_middle(full_name, max_chars);
    let resp = ui.label(RichText::new(&short).size(font_size).strong().color(color));
    if short != full_name {
        resp.on_hover_text(full_name);
    }
}

/// Shorten a filesystem path like `ui/js/components/path-cell.js`:
/// `/Users/khacthang/Library/Caches/com.apple.Safari` -> `~/…/Caches/com.apple.Safari`
/// Long folder names or filenames are middle-truncated (`head...tail`) so path pills stay compact.
pub fn shorten_path(full_path: &str) -> String {
    if full_path.is_empty() {
        return String::new();
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let s = if !home.is_empty() && full_path.starts_with(&home) {
        format!("~{}", &full_path[home.len()..])
    } else {
        full_path.to_string()
    };
    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() <= 3 {
        if parts.is_empty() {
            return s;
        }
        let mut short_parts: Vec<String> = parts
            .iter()
            .enumerate()
            .map(|(idx, seg)| {
                if idx + 1 == parts.len() {
                    shorten_middle(seg, 20)
                } else {
                    shorten_middle(seg, 14)
                }
            })
            .collect();
        if s.starts_with('/') {
            short_parts[0] = format!("/{}", short_parts[0]);
        }
        return short_parts.join("/");
    }
    let first = if s.starts_with('~') {
        "~".to_string()
    } else {
        format!("/{}", shorten_middle(parts[0], 12))
    };
    let parent = shorten_middle(parts[parts.len() - 2], 14);
    let leaf = shorten_middle(parts[parts.len() - 1], 20);
    format!("{}/…/{}/{}", first, parent, leaf)
}

/// Reusable Path Cell component matching `ui/js/components/path-cell.js`:
/// Renders a compact monospace path pill (hover shows full path, click copies path) + `📋` Copy + `📂` Reveal in Finder.
/// Returns `Some(copied_short_path)` if the user copied the path so the caller can show a toast.
pub fn path_action_cell(ui: &mut egui::Ui, full_path: &str, include_finder: bool) -> Option<String> {
    let p = palette();
    let short = shorten_path(full_path);
    let mut copied: Option<String> = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let pill = egui::Button::new(
            RichText::new(&short)
                .font(FontId::monospace(10.5))
                .color(p.text_secondary),
        )
        .fill(p.bg_input)
        .stroke(Stroke::new(1.0_f32, p.border))
        .rounding(Rounding::same(5.0))
        .min_size(egui::vec2(215.0, 22.0));

        if ui
            .add(pill)
            .on_hover_ui_at_pointer(|ui| {
                ui.set_max_width(380.0);
                ui.label(
                    RichText::new(full_path)
                        .font(FontId::monospace(11.0))
                        .color(p.text_primary),
                );
                ui.label(
                    RichText::new(tr("Click to copy path", "Nhấn để copy đường dẫn"))
                        .size(10.5)
                        .color(p.accent_text),
                );
            })
            .clicked()
        {
            ui.ctx().copy_text(full_path.to_string());
            copied = Some(short.clone());
        }

        let copy_btn = egui::Button::new(RichText::new("📋").size(11.0).color(p.text_secondary))
            .fill(p.bg_card_hover)
            .stroke(Stroke::new(1.0_f32, p.border))
            .rounding(Rounding::same(5.0));
        if ui
            .add(copy_btn)
            .on_hover_text(tr("Copy path", "Copy đường dẫn"))
            .clicked()
        {
            ui.ctx().copy_text(full_path.to_string());
            copied = Some(short);
        }

        if include_finder {
            let open_btn =
                egui::Button::new(RichText::new("📂").size(11.0).color(p.text_secondary))
                    .fill(p.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .rounding(Rounding::same(5.0));
            if ui
                .add(open_btn)
                .on_hover_text(tr("Reveal in Finder", "Mở trong Finder"))
                .clicked()
            {
                let _ = core_cleaner::reveal_in_finder(full_path);
            }
        }
    });

    copied
}

/// Interactive keyboard shortcut recorder pill (`.shortcut-recorder` in `base.css`).
pub fn shortcut_recorder_button(
    ui: &mut egui::Ui,
    shortcut: &str,
    is_recording: bool,
) -> egui::Response {
    let p = palette();
    let text = if is_recording {
        tr("Press keys... (Esc)", "Nhấn phím... (Esc)").to_string()
    } else if shortcut.trim().is_empty() {
        tr("Assign key...", "Gán phím...").to_string()
    } else {
        shortcut.to_string()
    };
    let fg = if is_recording {
        p.warning
    } else if shortcut.trim().is_empty() {
        p.text_muted
    } else {
        p.accent_text
    };
    let fill = if is_recording {
        p.warning_soft
    } else {
        p.bg_input
    };
    let border_col = if is_recording {
        p.warning
    } else if !shortcut.trim().is_empty() {
        p.accent_border
    } else {
        p.border
    };
    let btn = egui::Button::new(
        RichText::new(text)
            .font(FontId::monospace(11.5))
            .strong()
            .color(fg),
    )
    .fill(fill)
    .stroke(Stroke::new(1.0_f32, border_col))
    .rounding(Rounding::same(6.0))
    .min_size(egui::vec2(92.0, 24.0));
    ui.add(btn).on_hover_text(tr(
        "Click to record a new keyboard shortcut (Press Esc to cancel)",
        "Nhấp vào rồi nhấn tổ hợp phím mới trên bàn phím (Nhấn Esc để hủy)",
    ))
}

/// Custom macOS-style pill toggle switch (`true` / `false`).
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = egui::vec2(36.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let p = palette();
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let off_bg = if is_light_theme() {
            Color32::from_rgb(148, 163, 184)
        } else {
            Color32::from_rgb(64, 75, 98)
        };
        let bg_color = if *on { p.accent } else { off_bg };
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect_filled(rect, Rounding::same(radius), bg_color);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle_filled(center, radius - 2.5, Color32::WHITE);
    }
    response
}

/// Render a colored pill badge (`text` on `bg` with `fg` text color).
/// Uses an atomic non-interactive `egui::Button` so it never breaks `horizontal_wrapped` or `right_to_left` layouts.
pub fn badge_pill(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    let clean = sanitize_ui_text(text);
    let pill = egui::Button::new(egui::RichText::new(clean).size(11.5).strong().color(fg))
        .fill(bg)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(6.0))
        .sense(Sense::hover());
    let _ = ui.add(pill);
}

/// Render a floating toast notification overlay (`position: fixed; bottom: 22px; right: 22px; z-index: 9999`)
/// so notifications float cleanly above the UI without ever shifting or pushing down page content.
pub fn render_floating_toast(
    ctx: &egui::Context,
    toast: &mut Option<(String, std::time::Instant)>,
) {
    let Some((msg, ts)) = toast.clone() else {
        return;
    };
    if ts.elapsed() >= std::time::Duration::from_secs(4) {
        *toast = None;
        return;
    }

    let p = palette();
    let mut dismiss = false;
    egui::Area::new(egui::Id::new("omni_floating_toast_overlay"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-22.0, -22.0))
        .interactable(true)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(p.bg_card)
                .stroke(Stroke::new(1.5_f32, p.accent))
                .rounding(Rounding::same(10.0))
                .inner_margin(Margin::symmetric(14.0, 10.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 4.0),
                    blur: 14.0,
                    spread: 1.0,
                    color: Color32::from_black_alpha(110),
                })
                .show(ui, |ui| {
                    ui.set_max_width(460.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        badge_pill(ui, "🔔", p.accent_text, p.accent_soft);
                        ui.label(
                            RichText::new(&msg)
                                .size(12.5)
                                .strong()
                                .color(p.text_primary),
                        );
                        if secondary_button(ui, "✕").clicked() {
                            dismiss = true;
                        }
                    });
                });
        });

    if dismiss {
        *toast = None;
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

/// Primary accent button.
pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = palette();
    let clean = sanitize_ui_text(label);
    let btn = egui::Button::new(
        egui::RichText::new(clean)
            .size(12.0)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(p.accent)
    .stroke(Stroke::new(1.0_f32, p.accent))
    .rounding(Rounding::same(6.0));
    ui.add(btn)
}

/// Secondary subtle button.
pub fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = palette();
    let clean = sanitize_ui_text(label);
    let btn = egui::Button::new(
        egui::RichText::new(clean)
            .size(12.0)
            .strong()
            .color(p.text_primary),
    )
    .fill(p.bg_card_hover)
    .stroke(Stroke::new(1.0_f32, p.border))
    .rounding(Rounding::same(6.0));
    ui.add(btn)
}

/// Danger red button.
pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = palette();
    let clean = sanitize_ui_text(label);
    let btn = egui::Button::new(
        egui::RichText::new(clean)
            .size(12.0)
            .strong()
            .color(p.danger),
    )
    .fill(p.danger_soft)
    .stroke(Stroke::new(1.0_f32, p.danger))
    .rounding(Rounding::same(6.0));
    ui.add(btn)
}

/// Styled horizontal progress bar (`0.0..=1.0`).
pub fn progress_bar_styled(ui: &mut egui::Ui, fraction: f32, fill_color: Color32, height: f32) {
    let p = palette();
    let desired = egui::vec2(ui.available_width().max(40.0), height);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let r = Rounding::same((height * 0.5).clamp(2.0, 8.0));
        ui.painter().rect_filled(rect, r, p.bg_input);
        let clamped = fraction.clamp(0.0, 1.0);
        if clamped > 0.0 {
            let mut fill_rect = rect;
            fill_rect.set_width((rect.width() * clamped).max(height));
            ui.painter().rect_filled(fill_rect, r, fill_color);
        }
    }
}

/// Subtle horizontal row divider inside settings/table cards.
pub fn thin_divider(ui: &mut egui::Ui) {
    let p = palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 5.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        let y = rect.center().y;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            Stroke::new(1.0_f32, p.border),
        );
    }
}

/// Responsive split row layout that renders `right_ui` (controls, ComboBox, switches, buttons) first
/// in `Layout::right_to_left`, then allocates the exact remaining horizontal width `rem_w` for `left_ui`
/// (title, badges, tooltips) in `Layout::left_to_right` with text wrapping enabled.
/// This guarantees right-side widgets can NEVER overlap or cover left-side titles on narrow windows.
pub fn setting_row_split(
    ui: &mut egui::Ui,
    right_ui: impl FnOnce(&mut egui::Ui),
    left_ui: impl FnOnce(&mut egui::Ui),
) {
    let avail_w = ui.available_width();
    let row_h = ui.spacing().interact_size.y.max(24.0);
    ui.allocate_ui_with_layout(
        egui::vec2(avail_w, row_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            right_ui(ui);
            let rem_w = ui.available_width().max(36.0);
            ui.allocate_ui_with_layout(
                egui::vec2(rem_w, row_h),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_max_width(rem_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    left_ui(ui);
                },
            );
        },
    );
}

/// Render a setting row title label followed by an optional `?` tooltip badge,
/// automatically reserving space for the `?` badge so neither the text nor the badge
/// ever overflows into the right-side controls.
pub fn label_with_tip(ui: &mut egui::Ui, text: impl Into<egui::RichText>, tooltip: &str) {
    let reserve_for_tip = if tooltip.is_empty() {
        0.0
    } else {
        18.0 + ui.spacing().item_spacing.x
    };
    let max_lbl_w = (ui.available_width() - reserve_for_tip).max(24.0);
    ui.scope(|ui| {
        ui.set_max_width(max_lbl_w);
        ui.add(egui::Label::new(text.into()).wrap());
    });
    if !tooltip.is_empty() {
        tip_badge(ui, tooltip);
    }
}

// ============================================================================
// Vector Icons for Sidebar Navigation, Window Tiling & 5-Button Mouse Diagram
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavIconKind {
    Displays,
    Hardware,
    Clipboard,
    Tiling,
    Mouse,
    Cleaner,
    Settings,
}

/// Draw crisp anti-aliased vector icons for the 7 sidebar navigation tabs so they never
/// suffer from missing font glyphs or blurry bitmap scaling.
pub fn draw_nav_vector_icon(painter: &egui::Painter, rect: Rect, kind: NavIconKind, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.6_f32, color);
    match kind {
        NavIconKind::Displays => {
            painter.circle_stroke(c, 3.6, s);
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI * 0.25;
                let dir = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment([c + dir * 5.2, c + dir * 7.2], s);
            }
        }
        NavIconKind::Hardware => {
            let b1 = Rect::from_min_max(
                egui::pos2(c.x - 5.5, c.y - 1.0),
                egui::pos2(c.x - 3.0, c.y + 6.0),
            );
            let b2 = Rect::from_min_max(
                egui::pos2(c.x - 1.2, c.y - 5.5),
                egui::pos2(c.x + 1.3, c.y + 6.0),
            );
            let b3 = Rect::from_min_max(
                egui::pos2(c.x + 3.0, c.y - 3.0),
                egui::pos2(c.x + 5.5, c.y + 6.0),
            );
            painter.rect_filled(b1, Rounding::same(1.0), color);
            painter.rect_filled(b2, Rounding::same(1.0), color);
            painter.rect_filled(b3, Rounding::same(1.0), color);
        }
        NavIconKind::Clipboard => {
            let board = Rect::from_center_size(c + egui::vec2(0.0, 1.0), egui::vec2(11.0, 12.5));
            painter.rect_stroke(board, Rounding::same(2.0), s);
            let clip = Rect::from_center_size(c + egui::vec2(0.0, -5.2), egui::vec2(5.5, 2.6));
            painter.rect_filled(clip, Rounding::same(1.0), color);
            painter.line_segment(
                [egui::pos2(c.x - 3.0, c.y - 1.0), egui::pos2(c.x + 3.0, c.y - 1.0)],
                Stroke::new(1.3_f32, color),
            );
            painter.line_segment(
                [egui::pos2(c.x - 3.0, c.y + 2.2), egui::pos2(c.x + 2.0, c.y + 2.2)],
                Stroke::new(1.3_f32, color),
            );
        }
        NavIconKind::Tiling => {
            let r = Rect::from_center_size(c, egui::vec2(13.0, 11.0));
            painter.rect_stroke(r, Rounding::same(2.0), s);
            painter.line_segment([egui::pos2(c.x, r.top()), egui::pos2(c.x, r.bottom())], s);
            painter.line_segment([egui::pos2(c.x, c.y), egui::pos2(r.right(), c.y)], s);
        }
        NavIconKind::Mouse => {
            let body = Rect::from_center_size(c, egui::vec2(10.0, 14.0));
            painter.rect_stroke(body, Rounding::same(5.0), s);
            painter.line_segment(
                [egui::pos2(c.x, body.top() + 2.5), egui::pos2(c.x, c.y - 1.0)],
                Stroke::new(1.8_f32, color),
            );
        }
        NavIconKind::Cleaner => {
            let lid_y = c.y - 4.2;
            painter.line_segment(
                [egui::pos2(c.x - 5.5, lid_y), egui::pos2(c.x + 5.5, lid_y)],
                s,
            );
            painter.line_segment(
                [egui::pos2(c.x - 2.2, lid_y - 1.6), egui::pos2(c.x + 2.2, lid_y - 1.6)],
                s,
            );
            let bin = Rect::from_min_max(
                egui::pos2(c.x - 4.2, lid_y + 1.0),
                egui::pos2(c.x + 4.2, c.y + 6.0),
            );
            painter.rect_stroke(bin, Rounding::same(1.5), s);
            painter.line_segment(
                [egui::pos2(c.x - 1.4, lid_y + 3.0), egui::pos2(c.x - 1.4, c.y + 4.0)],
                Stroke::new(1.2_f32, color),
            );
            painter.line_segment(
                [egui::pos2(c.x + 1.4, lid_y + 3.0), egui::pos2(c.x + 1.4, c.y + 4.0)],
                Stroke::new(1.2_f32, color),
            );
        }
        NavIconKind::Settings => {
            painter.circle_stroke(c, 3.2, s);
            for i in 0..6 {
                let angle = (i as f32) * std::f32::consts::PI / 3.0;
                let dir = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment([c + dir * 4.2, c + dir * 6.4], Stroke::new(2.0_f32, color));
            }
        }
    }
}

/// Sidebar navigation item matching `.nav-item` and `.nav-item.active` in `ui/styles/base.css`.
pub fn nav_sidebar_button(
    ui: &mut egui::Ui,
    active: bool,
    icon_kind: NavIconKind,
    label: &str,
) -> egui::Response {
    let p = palette();
    let desired = egui::vec2(ui.available_width(), 34.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let bg = if active {
            p.accent_soft
        } else if hovered {
            p.bg_card_hover
        } else {
            Color32::TRANSPARENT
        };
        let border = if active {
            Stroke::new(1.0_f32, p.accent_border)
        } else {
            Stroke::NONE
        };
        let fg = if active {
            p.accent_text
        } else if hovered {
            p.text_primary
        } else {
            p.text_secondary
        };

        ui.painter()
            .rect_filled(rect, Rounding::same(8.0), bg);
        if border != Stroke::NONE {
            ui.painter()
                .rect_stroke(rect, Rounding::same(8.0), border);
        }

        let icon_rect = Rect::from_center_size(
            egui::pos2(rect.left() + 18.0, rect.center().y),
            egui::vec2(16.0, 16.0),
        );
        draw_nav_vector_icon(ui.painter(), icon_rect, icon_kind, fg);

        ui.painter().text(
            egui::pos2(rect.left() + 34.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.5),
            fg,
        );
    }
    resp
}

/// Draw a miniature visual window-tiling preview icon (`24x16`) for a given window tiling action.
pub fn draw_window_layout_icon(ui: &mut egui::Ui, action: &str) {
    let p = palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 17.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    ui.painter()
        .rect_filled(rect, Rounding::same(3.0), p.bg_input);
    ui.painter()
        .rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0_f32, p.border));

    let inner = rect.shrink(2.0);
    let w = inner.width();
    let h = inner.height();
    let l = inner.left();
    let t = inner.top();

    let highlight = match action {
        "left_half" => Rect::from_min_size(egui::pos2(l, t), egui::vec2(w * 0.5, h)),
        "right_half" => Rect::from_min_size(egui::pos2(l + w * 0.5, t), egui::vec2(w * 0.5, h)),
        "top_half" => Rect::from_min_size(egui::pos2(l, t), egui::vec2(w, h * 0.5)),
        "bottom_half" => Rect::from_min_size(egui::pos2(l, t + h * 0.5), egui::vec2(w, h * 0.5)),
        "top_left" => Rect::from_min_size(egui::pos2(l, t), egui::vec2(w * 0.5, h * 0.5)),
        "top_right" => {
            Rect::from_min_size(egui::pos2(l + w * 0.5, t), egui::vec2(w * 0.5, h * 0.5))
        }
        "bottom_left" => {
            Rect::from_min_size(egui::pos2(l, t + h * 0.5), egui::vec2(w * 0.5, h * 0.5))
        }
        "bottom_right" => Rect::from_min_size(
            egui::pos2(l + w * 0.5, t + h * 0.5),
            egui::vec2(w * 0.5, h * 0.5),
        ),
        "first_third" => Rect::from_min_size(egui::pos2(l, t), egui::vec2(w * 0.34, h)),
        "center_third" => {
            Rect::from_min_size(egui::pos2(l + w * 0.33, t), egui::vec2(w * 0.34, h))
        }
        "last_third" | "one_third_right" => {
            Rect::from_min_size(egui::pos2(l + w * 0.66, t), egui::vec2(w * 0.34, h))
        }
        "first_two_thirds" | "two_thirds_left" => {
            Rect::from_min_size(egui::pos2(l, t), egui::vec2(w * 0.66, h))
        }
        "last_two_thirds" => {
            Rect::from_min_size(egui::pos2(l + w * 0.34, t), egui::vec2(w * 0.66, h))
        }
        "center" => Rect::from_center_size(inner.center(), egui::vec2(w * 0.72, h * 0.85)),
        "maximize" => inner,
        _ => inner.shrink(2.0),
    };

    ui.painter()
        .rect_filled(highlight, Rounding::same(1.5), p.accent);
}

/// Interactive 5-button mouse diagram matching the exact SVG in `ui/js/screens/mouse.js`.
/// Returns `Some(button_key)` if the user clicked one of the buttons (`"forward"`, `"back"`, `"middle"`, `"chord"`).
pub fn draw_interactive_mouse_diagram(
    ui: &mut egui::Ui,
    selected_btn: &str,
) -> Option<&'static str> {
    let p = palette();
    let desired = egui::vec2(176.0, 174.0);
    let (canvas, _) = ui.allocate_exact_size(desired, Sense::hover());
    if !ui.is_rect_visible(canvas) {
        return None;
    }

    let mut clicked: Option<&'static str> = None;
    let c = canvas.center() + egui::vec2(14.0, 0.0);

    // 1. Outer Mouse Body Shell (`width = 104, height = 160, rx = 48`)
    let body = Rect::from_center_size(c, egui::vec2(104.0, 160.0));
    let body_fill = if is_light_theme() {
        Color32::from_rgb(226, 232, 240)
    } else {
        Color32::from_rgb(38, 46, 66)
    };
    let body_border = if is_light_theme() {
        Color32::from_rgb(148, 163, 184)
    } else {
        Color32::from_rgb(82, 96, 126)
    };
    ui.painter()
        .rect_filled(body, Rounding::same(48.0), body_fill);
    ui.painter().rect_stroke(
        body,
        Rounding::same(48.0),
        Stroke::new(2.0_f32, body_border),
    );

    // 2. Left Click Cap & Right Click Cap (Upper third of mouse body)
    let cap_h = 64.0;
    let left_cap = Rect::from_min_max(
        egui::pos2(body.left() + 6.0, body.top() + 6.0),
        egui::pos2(body.center().x - 2.0, body.top() + cap_h),
    );
    let right_cap = Rect::from_min_max(
        egui::pos2(body.center().x + 2.0, body.top() + 6.0),
        egui::pos2(body.right() - 6.0, body.top() + cap_h),
    );

    let left_resp = ui.interact(left_cap, ui.id().with("mouse_cap_left"), Sense::click());
    let right_resp = ui.interact(right_cap, ui.id().with("mouse_cap_right"), Sense::click());
    if left_resp.clicked() || right_resp.clicked() {
        clicked = Some("chord");
    }

    let chord_active =
        selected_btn == "chord" || left_resp.hovered() || right_resp.hovered();
    let cap_fill = if chord_active {
        p.accent_soft
    } else if is_light_theme() {
        Color32::from_rgb(203, 213, 225)
    } else {
        Color32::from_rgb(52, 62, 86)
    };
    let cap_stroke = if chord_active {
        Stroke::new(1.6_f32, p.accent)
    } else {
        Stroke::new(1.2_f32, body_border)
    };

    let left_rounding = Rounding {
        nw: 40.0,
        ne: 6.0,
        sw: 6.0,
        se: 6.0,
    };
    let right_rounding = Rounding {
        nw: 6.0,
        ne: 40.0,
        sw: 6.0,
        se: 6.0,
    };

    ui.painter().rect_filled(left_cap, left_rounding, cap_fill);
    ui.painter()
        .rect_stroke(left_cap, left_rounding, cap_stroke);
    ui.painter()
        .rect_filled(right_cap, right_rounding, cap_fill);
    ui.painter()
        .rect_stroke(right_cap, right_rounding, cap_stroke);

    ui.painter().text(
        left_cap.center() + egui::vec2(-4.0, 4.0),
        egui::Align2::CENTER_CENTER,
        "L",
        FontId::proportional(11.0),
        if chord_active { p.accent_text } else { p.text_secondary },
    );
    ui.painter().text(
        right_cap.center() + egui::vec2(4.0, 4.0),
        egui::Align2::CENTER_CENTER,
        "R",
        FontId::proportional(11.0),
        if chord_active { p.accent_text } else { p.text_secondary },
    );

    // 3. Middle Scroll Wheel (Mouse 3 — Solid Emerald Green `#10b981`)
    let mid_rect = Rect::from_center_size(
        egui::pos2(body.center().x, body.top() + 38.0),
        egui::vec2(18.0, 38.0),
    );
    let mid_resp = ui.interact(mid_rect, ui.id().with("mouse_hs_mid"), Sense::click());
    if mid_resp.clicked() {
        clicked = Some("middle");
    }
    let mid_sel = selected_btn == "middle" || mid_resp.hovered();
    let mid_fill = if mid_sel {
        Color32::from_rgb(16, 185, 129)
    } else {
        Color32::from_rgb(14, 92, 68)
    };
    let mid_border = if mid_sel {
        Color32::WHITE
    } else {
        Color32::from_rgb(16, 185, 129)
    };
    ui.painter()
        .rect_filled(mid_rect, Rounding::same(9.0), mid_fill);
    ui.painter().rect_stroke(
        mid_rect,
        Rounding::same(9.0),
        Stroke::new(2.0_f32, mid_border),
    );
    // Scroll wheel ridges
    for dy in [-6.0_f32, 0.0, 6.0] {
        let ry = mid_rect.center().y + dy;
        ui.painter().line_segment(
            [
                egui::pos2(mid_rect.left() + 4.0, ry),
                egui::pos2(mid_rect.right() - 4.0, ry),
            ],
            Stroke::new(1.2_f32, Color32::WHITE),
        );
    }

    // 4. Side Button 5 (Forward / Tiến — Upper Left Thumb Button)
    let fwd_rect = Rect::from_center_size(
        egui::pos2(body.left() - 5.0, body.top() + 80.0),
        egui::vec2(16.0, 34.0),
    );
    let fwd_resp = ui.interact(fwd_rect, ui.id().with("mouse_hs_fwd"), Sense::click());
    if fwd_resp.clicked() {
        clicked = Some("forward");
    }
    let fwd_sel = selected_btn == "forward" || fwd_resp.hovered();
    let fwd_fill = if fwd_sel {
        Color32::from_rgb(59, 130, 246)
    } else {
        Color32::from_rgb(30, 74, 154)
    };
    let fwd_border = if fwd_sel {
        Color32::WHITE
    } else {
        Color32::from_rgb(96, 165, 250)
    };
    ui.painter()
        .rect_filled(fwd_rect, Rounding::same(6.0), fwd_fill);
    ui.painter().rect_stroke(
        fwd_rect,
        Rounding::same(6.0),
        Stroke::new(2.0_f32, fwd_border),
    );
    ui.painter().text(
        egui::pos2(fwd_rect.left() - 4.0, fwd_rect.center().y),
        egui::Align2::RIGHT_CENTER,
        tr("Fwd", "Tiến"),
        FontId::proportional(10.5),
        p.accent_text,
    );

    // 5. Side Button 4 (Back / Lùi — Lower Left Thumb Button)
    let back_rect = Rect::from_center_size(
        egui::pos2(body.left() - 5.0, body.top() + 120.0),
        egui::vec2(16.0, 34.0),
    );
    let back_resp = ui.interact(back_rect, ui.id().with("mouse_hs_back"), Sense::click());
    if back_resp.clicked() {
        clicked = Some("back");
    }
    let back_sel = selected_btn == "back" || back_resp.hovered();
    let back_fill = if back_sel {
        Color32::from_rgb(59, 130, 246)
    } else {
        Color32::from_rgb(30, 74, 154)
    };
    let back_border = if back_sel {
        Color32::WHITE
    } else {
        Color32::from_rgb(96, 165, 250)
    };
    ui.painter()
        .rect_filled(back_rect, Rounding::same(6.0), back_fill);
    ui.painter().rect_stroke(
        back_rect,
        Rounding::same(6.0),
        Stroke::new(2.0_f32, back_border),
    );
    ui.painter().text(
        egui::pos2(back_rect.left() - 4.0, back_rect.center().y),
        egui::Align2::RIGHT_CENTER,
        tr("Back", "Lùi"),
        FontId::proportional(10.5),
        p.accent_text,
    );

    // 6. Palm Branding (`MAC OMNI` / `5-Button Mouse`)
    ui.painter().text(
        egui::pos2(body.center().x, body.top() + 106.0),
        egui::Align2::CENTER_CENTER,
        "MAC OMNI",
        FontId::proportional(10.0),
        p.text_secondary,
    );
    ui.painter().text(
        egui::pos2(body.center().x, body.top() + 120.0),
        egui::Align2::CENTER_CENTER,
        "5-Button Mouse",
        FontId::proportional(8.5),
        p.text_muted,
    );

    clicked
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_ui_text_strips_variation_selectors_and_maps_unicode11_emojis() {
        let dirty = "☀️ Màn hình 🌡️ Nhiệt độ 🖱️ Chuột 🧹 Dọn rác ⚙️ Cài đặt 🧠 RAM 🧭 Safari 👯 Trùng 🇻🇳 🇯🇵 新DBHUB\r\ncol1\tcol2";
        let cleaned = sanitize_ui_text(dirty);
        assert!(!cleaned.contains('\u{FE0F}'));
        assert!(!cleaned.contains('\r'));
        assert!(!cleaned.contains('\t'));
        assert!(!cleaned.contains('🧹'));
        assert!(!cleaned.contains('🧠'));
        assert!(!cleaned.contains('🧭'));
        assert!(!cleaned.contains('👯'));
        assert!(!cleaned.contains('🌡'));
        assert!(cleaned.contains("☀ Màn hình"));
        assert!(cleaned.contains("☀ Nhiệt độ"));
        assert!(cleaned.contains("🗑 Dọn rác"));
        assert!(cleaned.contains("⚙ Cài đặt"));
        assert!(cleaned.contains("VN JP 新DBHUB"));
        assert!(cleaned.contains("col1    col2"));
    }

    #[test]
    fn test_load_macos_vietnamese_fonts_includes_dingbats_and_symbol_fallbacks() {
        let fonts = load_macos_vietnamese_fonts();
        assert!(
            fonts.font_data.contains_key("macos_verdana")
                || fonts.font_data.contains_key("macos_arial")
        );
        assert!(fonts.font_data.contains_key("macos_monaco"));
        assert!(fonts.font_data.contains_key("macos_dingbats"));
        assert!(fonts.font_data.contains_key("macos_symbols"));

        let ctx = egui::Context::default();
        ensure_cjk_font_if_needed(&ctx, "JP 新DBHUB");
        let loaded = ctx.data(|d| {
            d.get_temp::<bool>(egui::Id::new("omni_cjk_font_loaded"))
                .unwrap_or(false)
        });
        assert!(loaded);
    }

    #[test]
    fn test_shorten_path_formats_home_and_deep_paths() {
        let s = shorten_path("/Applications/Safari.app");
        assert_eq!(s, "/Applications/Safari.app");
        let deep = shorten_path("/Library/Application Support/SomeVendor/SubFolder/CacheFile.db");
        assert_eq!(deep, "/Library/…/SubFolder/CacheFile.db");

        let long_name = shorten_middle("D1DB8E47-2561-4067-ADE4-789782839658.VMRS", 26);
        assert!(long_name.starts_with("D1DB8E47"));
        assert!(long_name.ends_with(".VMRS"));
        assert!(long_name.contains("..."));
        assert!(long_name.chars().count() <= 26);
    }

    #[test]
    fn test_metal_command_queue_trim_reclaims_ringbuffer_memory() {
        let gpu = GpuContext::new();
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Bgra8Unorm,
            None,
            1,
            false,
        );
        let ctx = egui::Context::default();
        ctx.set_fonts(gpu.fonts.clone());
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1440,
                height: 900,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

        for frame_idx in 0..2 {
            let mut raw_input = egui::RawInput {
                max_texture_side: Some(1024),
                ..Default::default()
            };
            raw_input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            ));
            let full_output = ctx.run(raw_input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label(format!("Xin chào Việt Nam frame {}", frame_idx));
                });
            });
            for (id, delta) in &full_output.textures_delta.set {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
            let tris = ctx.tessellate(full_output.shapes, 1.0);
            let sd = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [1440, 900],
                pixels_per_point: 1.0,
            };
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &tris, &sd);
            {
                let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let mut rpass_static = rpass.forget_lifetime();
                renderer.render(&mut rpass_static, &tris, &sd);
            }
            gpu.queue.submit(std::iter::once(encoder.finish()));
            gpu.trim_command_queue();
        }
    }
}
