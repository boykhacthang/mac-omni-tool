mod events;
mod gpu_ui;
mod i18n;
mod macos_window;
mod menubar;
#[allow(dead_code)]
mod rpc;
mod single_instance;
#[allow(dead_code)]
mod ui_assets;
mod updater;

use crate::events::{screenshot_session, AppUserEvent, ScreenshotSlotData, MAX_SCREENSHOT_SLOTS};
use crate::gpu_ui::{
    dashboard::{DashboardState, DashboardTab},
    overlays::{AltTabOverlayState, ClipboardDrawerState, MiniPopoverState},
    screenshot::ScreenshotOverlayManager,
    GpuContext, WindowGpuSurface,
};
use crate::i18n::{init_ui_prefs, set_lang, set_theme, tr};
use crate::macos_window::{
    disable_macos_window_restoration, focus_and_elevate_macos_window,
    hide_floating_overlay_window, park_overlay_window_offscreen,
    place_main_window_center_75_full_height, place_overlay_on_display,
    prepare_screenshot_window_hidden, release_process_heap_memory, restore_previous_frontmost_app,
    reveal_screenshot_window, set_macos_dock_icon_visible, set_nswindow_non_restorable,
    set_nswindow_rounded_transparent, show_floating_overlay_window, snap_nswindow_directly,
};
use crate::menubar::{
    init_native_macos_menubar_item, setup_macos_application_menu, update_status_bar_title,
};
use crate::single_instance::ensure_single_instance;

use egui_winit::winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId, WindowLevel},
};
use std::sync::Arc;

struct OmniGpuApp {
    start_minimized: bool,
    proxy: EventLoopProxy<AppUserEvent>,
    gpu: GpuContext,

    // Window GPU surfaces (initialized in `resumed`)
    main_surface: Option<WindowGpuSurface>,
    mini_surface: Option<WindowGpuSurface>,
    clip_surface: Option<WindowGpuSurface>,
    alttab_surface: Option<WindowGpuSurface>,
    screenshot_surfaces: Vec<WindowGpuSurface>,

    // UI states
    dashboard_ui: DashboardState,
    mini_ui: MiniPopoverState,
    clip_ui: ClipboardDrawerState,
    alttab_ui: AltTabOverlayState,
    screenshot_ui: ScreenshotOverlayManager,

    mini_w: f64,
    mini_h: f64,
    mini_pos: (f64, f64),
    alttab_w: f64,
    alttab_h: f64,
    next_repaint_deadline: Option<std::time::Instant>,
}

impl OmniGpuApp {
    fn new(start_minimized: bool, proxy: EventLoopProxy<AppUserEvent>) -> Self {
        let _ = init_ui_prefs();
        let gpu = GpuContext::new();
        let cur_disp = core_display::get_display_at_mouse_cursor();
        let mini_w = 384.0;
        let mini_h = 430.0;
        let mini_pos = (
            cur_disp.x as f64 + (cur_disp.width as f64 - (mini_w + 16.0)).max(20.0),
            cur_disp.y as f64 + 28.0,
        );

        Self {
            start_minimized,
            proxy,
            gpu,
            main_surface: None,
            mini_surface: None,
            clip_surface: None,
            alttab_surface: None,
            screenshot_surfaces: Vec::with_capacity(MAX_SCREENSHOT_SLOTS),
            dashboard_ui: DashboardState::new(),
            mini_ui: MiniPopoverState::new(),
            clip_ui: ClipboardDrawerState::new(),
            alttab_ui: AltTabOverlayState::new(),
            screenshot_ui: ScreenshotOverlayManager::new(),
            mini_w,
            mini_h,
            mini_pos,
            alttab_w: 920.0,
            alttab_h: 520.0,
            next_repaint_deadline: None,
        }
    }

    fn record_repaint_delay(&mut self, delay: std::time::Duration) {
        if delay.is_zero() || delay > std::time::Duration::from_secs(3600) {
            return;
        }
        let candidate = std::time::Instant::now() + delay;
        match self.next_repaint_deadline {
            Some(cur) if cur <= candidate => {}
            _ => self.next_repaint_deadline = Some(candidate),
        }
    }

    fn create_screenshot_slot_surface(
        &self,
        event_loop: &ActiveEventLoop,
        slot: usize,
        x: i32,
        y: i32,
        width: usize,
        height: usize,
    ) -> WindowGpuSurface {
        let attrs = WindowAttributes::default()
            .with_title(format!("Mac Omni Screenshot #{}", slot + 1))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(width as f64, height as f64))
            .with_position(LogicalPosition::new(x as f64, y as f64))
            .with_visible(false);

        let win = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create screenshot overlay window"),
        );

        unsafe {
            set_nswindow_rounded_transparent(&win, 0.0);
            place_overlay_on_display(&win, x, y, width, height);
            park_overlay_window_offscreen(&win);
        }

        // Do not allocate Metal surface until screenshot session is actually triggered
        WindowGpuSurface::new(&self.gpu, win, true, false)
    }

    fn ensure_windows_initialized(&mut self, event_loop: &ActiveEventLoop) {
        if self.main_surface.is_some() {
            return;
        }

        let cur_disp = core_display::get_display_at_mouse_cursor();

        // 1. Main Launcher Dashboard Window (`main_window`) — created FIRST for instant startup!
        let main_w_init = ((cur_disp.width as f64) * 0.75).round().max(900.0);
        let main_h_init = (cur_disp.height as f64 - 72.0).max(600.0);
        let main_attrs = WindowAttributes::default()
            .with_title("Mac Omni Tool")
            .with_inner_size(LogicalSize::new(main_w_init, main_h_init))
            .with_min_inner_size(LogicalSize::new(420.0, 320.0))
            .with_visible(!self.start_minimized);
        let main_win = Arc::new(
            event_loop
                .create_window(main_attrs)
                .expect("Failed to create main launcher window"),
        );
        unsafe {
            set_nswindow_non_restorable(&main_win);
            place_main_window_center_75_full_height(&main_win);
            set_macos_dock_icon_visible(!self.start_minimized);
            setup_macos_application_menu();
        }
        let main_surf = WindowGpuSurface::new(&self.gpu, main_win.clone(), false, !self.start_minimized);
        if !self.start_minimized {
            main_win.request_redraw();
        }
        self.main_surface = Some(main_surf);

        // 2. Menu Bar Dropdown Popover (`mini_window`)
        let mini_attrs = WindowAttributes::default()
            .with_title("Mac Omni Menu Bar")
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(self.mini_w, self.mini_h))
            .with_position(LogicalPosition::new(self.mini_pos.0, self.mini_pos.1))
            .with_visible(false);
        let mini_win = Arc::new(
            event_loop
                .create_window(mini_attrs)
                .expect("Failed to create menu bar popover window"),
        );
        unsafe {
            set_nswindow_rounded_transparent(&mini_win, 14.0);
            hide_floating_overlay_window(&mini_win);
        }
        self.mini_surface = Some(WindowGpuSurface::new(&self.gpu, mini_win, true, false));

        // 3. Left-Side Clipboard Drawer (`clip_window`)
        let drawer_h = (cur_disp.height as f64 - 84.0).clamp(520.0, 900.0);
        let clip_attrs = WindowAttributes::default()
            .with_title("Clipboard Manager (Cmd+Shift+V)")
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(450.0, drawer_h))
            .with_position(LogicalPosition::new(
                cur_disp.x as f64 + 12.0,
                cur_disp.y as f64 + 38.0,
            ))
            .with_visible(false);
        let clip_win = Arc::new(
            event_loop
                .create_window(clip_attrs)
                .expect("Failed to create left clipboard drawer window"),
        );
        unsafe {
            set_nswindow_rounded_transparent(&clip_win, 14.0);
            hide_floating_overlay_window(&clip_win);
        }
        self.clip_surface = Some(WindowGpuSurface::new(&self.gpu, clip_win, true, false));

        // 4. Alt-Tab Live Preview Overlay (`alttab_window`)
        let alttab_attrs = WindowAttributes::default()
            .with_title("Alt-Tab Live Preview")
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(self.alttab_w, self.alttab_h))
            .with_position(LogicalPosition::new(
                cur_disp.x as f64 + ((cur_disp.width as f64 - self.alttab_w) / 2.0).max(20.0),
                cur_disp.y as f64 + ((cur_disp.height as f64 - self.alttab_h) / 2.0).max(20.0),
            ))
            .with_visible(false);
        let alttab_win = Arc::new(
            event_loop
                .create_window(alttab_attrs)
                .expect("Failed to create Alt-Tab overlay window"),
        );
        unsafe {
            set_nswindow_rounded_transparent(&alttab_win, 14.0);
            hide_floating_overlay_window(&alttab_win);
        }
        self.alttab_surface = Some(WindowGpuSurface::new(&self.gpu, alttab_win, true, false));

        // 5. Pre-create Screenshot Overlay Windows (parked offscreen without Metal swapchain)
        // so the very first Cmd+Shift+4 opens in < 70ms!
        for (idx, disp) in core_display::list_displays()
            .into_iter()
            .take(MAX_SCREENSHOT_SLOTS)
            .enumerate()
        {
            let surf = self.create_screenshot_slot_surface(
                event_loop,
                idx,
                disp.x,
                disp.y,
                disp.width,
                disp.height,
            );
            self.screenshot_surfaces.push(surf);
        }
    }

    fn any_screenshot_visible(&self) -> bool {
        self.screenshot_surfaces
            .iter()
            .any(|s| s.window.is_visible().unwrap_or(false))
    }

    fn hide_mini_popover(&mut self) {
        if let Some(mini) = &mut self.mini_surface {
            if mini.window.is_visible().unwrap_or(false) {
                unsafe {
                    hide_floating_overlay_window(&mini.window);
                }
                mini.suspend_surface(&self.gpu);
                self.mini_ui.on_hidden();
            }
        }
    }

    fn hide_clipboard_drawer(&mut self) {
        if let Some(clip) = &mut self.clip_surface {
            if clip.window.is_visible().unwrap_or(false) {
                unsafe {
                    hide_floating_overlay_window(&clip.window);
                }
                clip.suspend_surface(&self.gpu);
                self.clip_ui.on_hidden();
            }
        }
        let alttab_vis = self
            .alttab_surface
            .as_ref()
            .and_then(|s| s.window.is_visible())
            .unwrap_or(false);
        if !alttab_vis && !self.any_screenshot_visible() {
            core_input::set_overlay_escape_hotkey_active(false);
        }
    }

    fn hide_alttab_overlay(&mut self) {
        if let Some(alttab) = &mut self.alttab_surface {
            if alttab.window.is_visible().unwrap_or(false) {
                unsafe {
                    hide_floating_overlay_window(&alttab.window);
                }
                alttab.suspend_surface(&self.gpu);
                self.alttab_ui.on_hidden();
            }
        }
        let clip_vis = self
            .clip_surface
            .as_ref()
            .and_then(|s| s.window.is_visible())
            .unwrap_or(false);
        if !clip_vis && !self.any_screenshot_visible() {
            core_input::set_overlay_escape_hotkey_active(false);
        }
    }

    fn hide_screenshot_overlays(&mut self) {
        let (was_revealed, prev_pid) = {
            let mut sess = screenshot_session().lock().unwrap();
            let rev = sess.revealed;
            sess.revealed = false;
            sess.active_slot = None;
            sess.capturing = false;
            sess.slots.clear();
            (rev, std::mem::replace(&mut sess.prev_frontmost_pid, 0))
        };
        let any_vis = was_revealed || self.any_screenshot_visible();
        for surf in &mut self.screenshot_surfaces {
            if surf.window.is_visible().unwrap_or(false) || surf.surface.is_some() {
                unsafe {
                    park_overlay_window_offscreen(&surf.window);
                }
                surf.suspend_surface(&self.gpu);
            }
        }
        self.screenshot_ui.clear_all();
        if any_vis {
            if let Some(main) = &self.main_surface {
                unsafe {
                    restore_previous_frontmost_app(&main.window, prev_pid);
                }
            }
            release_process_heap_memory();
        }
        let clip_vis = self
            .clip_surface
            .as_ref()
            .and_then(|s| s.window.is_visible())
            .unwrap_or(false);
        let alttab_vis = self
            .alttab_surface
            .as_ref()
            .and_then(|s| s.window.is_visible())
            .unwrap_or(false);
        if !clip_vis && !alttab_vis {
            core_input::set_overlay_escape_hotkey_active(false);
        }
    }

    fn show_main_dashboard(&mut self) {
        if let Some(main) = &mut self.main_surface {
            unsafe {
                set_macos_dock_icon_visible(true);
                if !main.window.is_visible().unwrap_or(false) {
                    place_main_window_center_75_full_height(&main.window);
                }
                setup_macos_application_menu();
            }
            main.ensure_surface(&self.gpu);
            main.window.set_visible(true);
            main.window.focus_window();
            self.dashboard_ui.on_focused();
            main.window.request_redraw();
        }
    }

    fn hide_main_dashboard(&mut self) {
        if let Some(main) = &mut self.main_surface {
            main.window.set_visible(false);
            main.suspend_surface(&self.gpu);
            unsafe {
                set_macos_dock_icon_visible(false);
            }
            self.dashboard_ui.on_hidden();
            release_process_heap_memory();
        }
    }

    fn request_redraw_all_visible(&self) {
        if let Some(s) = &self.main_surface {
            if s.window.is_visible().unwrap_or(false) {
                s.window.request_redraw();
            }
        }
        if let Some(s) = &self.mini_surface {
            if s.window.is_visible().unwrap_or(false) {
                s.window.request_redraw();
            }
        }
        if let Some(s) = &self.clip_surface {
            if s.window.is_visible().unwrap_or(false) {
                s.window.request_redraw();
            }
        }
        if let Some(s) = &self.alttab_surface {
            if s.window.is_visible().unwrap_or(false) {
                s.window.request_redraw();
            }
        }
        for s in &self.screenshot_surfaces {
            if s.window.is_visible().unwrap_or(false) {
                s.window.request_redraw();
            }
        }
    }
}

impl ApplicationHandler<AppUserEvent> for OmniGpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        let first_init = self.main_surface.is_none();
        self.ensure_windows_initialized(event_loop);
        if !first_init {
            // Reopen from Dock / Spotlight when already running
            self.show_main_dashboard();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, user_ev: AppUserEvent) {
        event_loop.set_control_flow(ControlFlow::Wait);
        self.ensure_windows_initialized(event_loop);

        match user_ev {
            AppUserEvent::ToggleMenuBarPopover => {
                let cfg = core_input::get_input_config();
                if !cfg.menubar_enabled || cfg.menubar_display_mode == "hidden" {
                    self.hide_mini_popover();
                    return;
                }
                let is_vis = self
                    .mini_surface
                    .as_ref()
                    .and_then(|s| s.window.is_visible())
                    .unwrap_or(false);
                if is_vis {
                    self.hide_mini_popover();
                } else {
                    let (mx, _my) = core_display::get_mouse_cursor_position();
                    let disp = core_display::get_display_at_mouse_cursor();
                    let min_x = disp.x as f64 + 12.0;
                    let max_x =
                        (disp.x as f64 + disp.width as f64 - (self.mini_w + 12.0)).max(min_x);
                    let px = (mx - (self.mini_w / 2.0)).clamp(min_x, max_x);
                    let py = disp.y as f64 + 28.0;
                    self.mini_pos = (px, py);
                    self.mini_ui.refresh_now();
                    if let Some(mini) = &mut self.mini_surface {
                        let _ = mini
                            .window
                            .request_inner_size(LogicalSize::new(self.mini_w, self.mini_h));
                        mini.window
                            .set_outer_position(LogicalPosition::new(px, py));
                        mini.ensure_surface(&self.gpu);
                        unsafe {
                            show_floating_overlay_window(&mini.window, 14.0);
                        }
                        mini.window.request_redraw();
                    }
                }
            }
            AppUserEvent::SetMiniExpanded(expanded) => {
                if expanded {
                    self.mini_ui.refresh_now();
                    if let Some(mini) = &mut self.mini_surface {
                        mini.ensure_surface(&self.gpu);
                        unsafe {
                            show_floating_overlay_window(&mini.window, 14.0);
                        }
                        mini.window.request_redraw();
                    }
                } else {
                    self.hide_mini_popover();
                }
            }
            AppUserEvent::SetMiniHeight(h) => {
                let clamped_h = h.clamp(310.0, 640.0).round();
                if (clamped_h - self.mini_h).abs() > 1.0 {
                    self.mini_h = clamped_h;
                    if let Some(mini) = &mut self.mini_surface {
                        let _ = mini
                            .window
                            .request_inner_size(LogicalSize::new(self.mini_w, self.mini_h));
                        mini.window.set_outer_position(LogicalPosition::new(
                            self.mini_pos.0,
                            self.mini_pos.1,
                        ));
                        unsafe {
                            set_nswindow_rounded_transparent(&mini.window, 14.0);
                        }
                        mini.window.request_redraw();
                    }
                }
            }
            AppUserEvent::UpdateMenuBarTitle(title) => {
                let cfg = core_input::get_input_config();
                if cfg.menubar_enabled && cfg.menubar_display_mode != "hidden" {
                    update_status_bar_title(&title);
                }
            }
            AppUserEvent::SyncMenuBarConfig => {
                let cfg = core_input::get_input_config();
                let (visible, _) = crate::menubar::sync_menubar_from_config(&cfg);
                if !visible {
                    self.hide_mini_popover();
                } else if self
                    .mini_surface
                    .as_ref()
                    .and_then(|s| s.window.is_visible())
                    .unwrap_or(false)
                {
                    self.mini_ui.refresh_now();
                    if let Some(mini) = &self.mini_surface {
                        mini.window.request_redraw();
                    }
                }
            }
            AppUserEvent::SetAppTheme(theme) => {
                set_theme(&theme);
                self.request_redraw_all_visible();
            }
            AppUserEvent::SetAppLang(lang) => {
                set_lang(&lang);
                self.request_redraw_all_visible();
            }
            AppUserEvent::ShowLauncher => {
                self.show_main_dashboard();
            }
            AppUserEvent::ToggleLauncher => {
                let my_pid = std::process::id() as i32;
                let is_front = core_window::frontmost_application_pid() == my_pid;
                let is_vis = self
                    .main_surface
                    .as_ref()
                    .and_then(|s| s.window.is_visible())
                    .unwrap_or(false);
                if is_vis && is_front {
                    self.hide_main_dashboard();
                } else {
                    self.show_main_dashboard();
                }
            }
            AppUserEvent::ShowAboutDialog => {
                self.dashboard_ui.show_about_modal = true;
                self.show_main_dashboard();
            }
            AppUserEvent::OpenDashboardTab(tab) => {
                self.dashboard_ui.switch_tab(DashboardTab::from_name(&tab));
                self.show_main_dashboard();
            }
            AppUserEvent::HideLauncher => {
                self.hide_main_dashboard();
            }
            AppUserEvent::QuitApp => {
                event_loop.exit();
            }
            AppUserEvent::ToggleClipboardDrawer => {
                if !core_input::get_input_config().clipboard_enabled {
                    self.hide_clipboard_drawer();
                    return;
                }
                let is_vis = self
                    .clip_surface
                    .as_ref()
                    .and_then(|s| s.window.is_visible())
                    .unwrap_or(false);
                if is_vis {
                    self.hide_clipboard_drawer();
                } else {
                    let disp = core_display::get_display_at_mouse_cursor();
                    let h = (disp.height as f64 - 84.0).clamp(520.0, 900.0);
                    self.clip_ui.on_shown();
                    if let Some(clip) = &mut self.clip_surface {
                        let _ = clip.window.request_inner_size(LogicalSize::new(450.0, h));
                        clip.window.set_outer_position(LogicalPosition::new(
                            disp.x as f64 + 12.0,
                            disp.y as f64 + 38.0,
                        ));
                        clip.ensure_surface(&self.gpu);
                        unsafe {
                            show_floating_overlay_window(&clip.window, 14.0);
                        }
                        core_input::set_overlay_escape_hotkey_active(true);
                        clip.window.request_redraw();
                    }
                }
            }
            AppUserEvent::HideClipboardDrawer => {
                self.hide_clipboard_drawer();
            }
            AppUserEvent::ToggleAltTabOverlay => {
                if !core_input::get_input_config().replace_macos_cmd_tab
                    || !core_window::is_accessibility_trusted()
                {
                    self.hide_alttab_overlay();
                    return;
                }
                let is_vis = self
                    .alttab_surface
                    .as_ref()
                    .and_then(|s| s.window.is_visible())
                    .unwrap_or(false);
                if is_vis {
                    self.alttab_ui.cycle_next();
                    if let Some(alttab) = &self.alttab_surface {
                        alttab.window.request_redraw();
                    }
                } else {
                    let disp = core_display::get_display_at_mouse_cursor();
                    let ax = disp.x as f64 + ((disp.width as f64 - self.alttab_w) / 2.0).max(20.0);
                    let ay = disp.y as f64 + ((disp.height as f64 - self.alttab_h) / 2.0).max(20.0);
                    self.alttab_ui.refresh_on_open();
                    if let Some(alttab) = &mut self.alttab_surface {
                        alttab
                            .window
                            .set_outer_position(LogicalPosition::new(ax, ay));
                        alttab.ensure_surface(&self.gpu);
                        unsafe {
                            show_floating_overlay_window(&alttab.window, 14.0);
                        }
                        core_input::set_overlay_escape_hotkey_active(true);
                        alttab.window.request_redraw();
                    }
                }
            }
            AppUserEvent::CommitAltTabSelection => {
                let is_vis = self
                    .alttab_surface
                    .as_ref()
                    .and_then(|s| s.window.is_visible())
                    .unwrap_or(false);
                if is_vis {
                    self.alttab_ui.commit_selection(&self.proxy);
                }
            }
            AppUserEvent::HideAltTabOverlay => {
                self.hide_alttab_overlay();
            }
            AppUserEvent::TriggerFullscreenScreenshot {
                display_id,
                instant_copy,
            } => {
                let cfg = core_input::get_input_config();
                if !cfg.screenshot_enabled
                    || (instant_copy && !cfg.screenshot_fullscreen_copy_enabled)
                    || (!instant_copy && !cfg.screenshot_fullscreen_edit_enabled)
                {
                    return;
                }
                if !core_window::is_screen_recording_granted() {
                    let _ = self.proxy.send_event(AppUserEvent::ScreenshotCaptureFailed);
                    return;
                }
                {
                    let mut sess = screenshot_session().lock().unwrap();
                    if sess.capturing || sess.revealed {
                        return;
                    }
                    sess.capturing = true;
                }
                self.hide_mini_popover();
                self.hide_clipboard_drawer();
                self.hide_alttab_overlay();

                let prev_pid = core_window::frontmost_application_pid();
                let mouse_disp = core_display::get_display_at_mouse_cursor();
                let target_disp = if display_id != 0 {
                    core_display::list_displays()
                        .into_iter()
                        .find(|d| d.id == display_id)
                        .unwrap_or_else(|| mouse_disp.clone())
                } else {
                    mouse_disp.clone()
                };

                let bg_proxy = self.proxy.clone();
                if instant_copy {
                    std::thread::spawn(move || {
                        let res = core_display::capture_display_png(&target_disp);
                        {
                            let mut sess = screenshot_session().lock().unwrap();
                            sess.capturing = false;
                        }
                        match res {
                            Ok(png_bytes) => {
                                let _ = core_clipboard::copy_png_bytes_to_clipboard(&png_bytes);
                                let _ =
                                    bg_proxy.send_event(AppUserEvent::FullscreenInstantCopied {
                                        display_name: target_disp.name,
                                    });
                            }
                            Err(_) => {
                                let _ =
                                    bg_proxy.send_event(AppUserEvent::ScreenshotCaptureFailed);
                            }
                        }
                    });
                } else {
                    std::thread::spawn(move || {
                        match core_display::capture_all_displays_rgba8() {
                            Ok(captured_list) => {
                                let seq = {
                                    let mut sess = screenshot_session().lock().unwrap();
                                    sess.capturing = false;
                                    sess.capture_seq += 1;
                                    let mut found_target = 0usize;
                                    sess.slots = captured_list
                                        .into_iter()
                                        .take(MAX_SCREENSHOT_SLOTS)
                                        .enumerate()
                                        .map(|(idx, cap)| {
                                            let is_target = cap.display.id == target_disp.id;
                                            if is_target {
                                                found_target = idx;
                                            }
                                            ScreenshotSlotData {
                                                display_id: cap.display.id,
                                                display_name: cap.display.name,
                                                png_bytes: Vec::new(),
                                                pixel_w: cap.pixel_w,
                                                pixel_h: cap.pixel_h,
                                                raw_rgba: cap.rgba,
                                                x: cap.display.x,
                                                y: cap.display.y,
                                                width: cap.display.width,
                                                height: cap.display.height,
                                                preselect_fullscreen: is_target,
                                            }
                                        })
                                        .collect();
                                    let count = sess.slots.len();
                                    sess.expected_slots = count;
                                    sess.ready_slots = vec![true; count];
                                    sess.revealed = false;
                                    sess.target_slot = found_target;
                                    sess.active_slot = Some(found_target);
                                    sess.prev_frontmost_pid = prev_pid;
                                    sess.capture_seq
                                };
                                let _ =
                                    bg_proxy.send_event(AppUserEvent::PresentScreenshotSession {
                                        seq,
                                        fullscreen_picker_mode: true,
                                    });
                            }
                            Err(_) => {
                                {
                                    let mut sess = screenshot_session().lock().unwrap();
                                    sess.capturing = false;
                                }
                                let _ =
                                    bg_proxy.send_event(AppUserEvent::ScreenshotCaptureFailed);
                            }
                        }
                    });
                }
            }
            AppUserEvent::TriggerScreenshotOverlay => {
                let cfg = core_input::get_input_config();
                if !cfg.screenshot_enabled || !cfg.screenshot_region_enabled {
                    return;
                }
                if !core_window::is_screen_recording_granted() {
                    let _ = self.proxy.send_event(AppUserEvent::ScreenshotCaptureFailed);
                    return;
                }
                {
                    let mut sess = screenshot_session().lock().unwrap();
                    if sess.capturing || sess.revealed {
                        return;
                    }
                    sess.capturing = true;
                }
                self.hide_mini_popover();
                self.hide_clipboard_drawer();
                self.hide_alttab_overlay();

                let prev_pid = core_window::frontmost_application_pid();
                let mouse_disp = core_display::get_display_at_mouse_cursor();
                let bg_proxy = self.proxy.clone();

                std::thread::spawn(move || {
                    match core_display::capture_all_displays_rgba8() {
                        Ok(captured_list) => {
                            let seq = {
                                let mut sess = screenshot_session().lock().unwrap();
                                sess.capturing = false;
                                sess.capture_seq += 1;
                                let mut found_target = 0usize;
                                sess.slots = captured_list
                                    .into_iter()
                                    .take(MAX_SCREENSHOT_SLOTS)
                                    .enumerate()
                                    .map(|(idx, cap)| {
                                        if cap.display.id == mouse_disp.id {
                                            found_target = idx;
                                        }
                                        ScreenshotSlotData {
                                            display_id: cap.display.id,
                                            display_name: cap.display.name,
                                            png_bytes: Vec::new(),
                                            pixel_w: cap.pixel_w,
                                            pixel_h: cap.pixel_h,
                                            raw_rgba: cap.rgba,
                                            x: cap.display.x,
                                            y: cap.display.y,
                                            width: cap.display.width,
                                            height: cap.display.height,
                                            preselect_fullscreen: false,
                                        }
                                    })
                                    .collect();
                                let count = sess.slots.len();
                                sess.expected_slots = count;
                                sess.ready_slots = vec![true; count];
                                sess.revealed = false;
                                sess.target_slot = found_target;
                                sess.active_slot = Some(found_target);
                                sess.prev_frontmost_pid = prev_pid;
                                sess.capture_seq
                            };
                            let _ = bg_proxy.send_event(AppUserEvent::PresentScreenshotSession {
                                seq,
                                fullscreen_picker_mode: false,
                            });
                        }
                        Err(_) => {
                            {
                                let mut sess = screenshot_session().lock().unwrap();
                                sess.capturing = false;
                            }
                            let _ = bg_proxy.send_event(AppUserEvent::ScreenshotCaptureFailed);
                        }
                    }
                });
            }
            AppUserEvent::PresentScreenshotSession {
                seq,
                fullscreen_picker_mode,
            } => {
                let (slot_data, target_slot) = {
                    let mut sess = screenshot_session().lock().unwrap();
                    if sess.capture_seq != seq || sess.slots.is_empty() {
                        return;
                    }
                    sess.revealed = true;
                    (std::mem::take(&mut sess.slots), sess.target_slot)
                };

                while self.screenshot_surfaces.len() < slot_data.len().min(MAX_SCREENSHOT_SLOTS) {
                    let next_slot = self.screenshot_surfaces.len();
                    let s = &slot_data[next_slot];
                    let surf = self.create_screenshot_slot_surface(
                        event_loop,
                        next_slot,
                        s.x,
                        s.y,
                        s.width,
                        s.height,
                    );
                    self.screenshot_surfaces.push(surf);
                }

                core_input::set_overlay_escape_hotkey_active(true);

                for (slot_idx, s) in slot_data.into_iter().enumerate() {
                    if let Some(ui_slot) = self.screenshot_ui.slots.get_mut(slot_idx) {
                        ui_slot.load_session_slot(
                            s.display_name,
                            s.width,
                            s.height,
                            s.pixel_w,
                            s.pixel_h,
                            s.raw_rgba,
                            s.preselect_fullscreen,
                            fullscreen_picker_mode,
                        );
                    }
                    if let Some(surf) = self.screenshot_surfaces.get_mut(slot_idx) {
                        unsafe {
                            prepare_screenshot_window_hidden(
                                &surf.window,
                                s.x,
                                s.y,
                                s.width,
                                s.height,
                            );
                        }
                        surf.window.set_visible(true);
                        surf.ensure_surface(&self.gpu);
                        if let Some(ui_slot) = self.screenshot_ui.slots.get_mut(slot_idx) {
                            let proxy = &self.proxy;
                            surf.render_frame(&self.gpu, [0.0, 0.0, 0.0, 1.0], |ctx| {
                                ui_slot.render(ctx, proxy);
                            });
                        }
                    }
                }
                release_process_heap_memory();

                // Atomically reveal all displays now that their Metal textures are rendered
                let count = self.screenshot_surfaces.len();
                for idx in 0..count {
                    if idx != target_slot {
                        unsafe {
                            reveal_screenshot_window(&self.screenshot_surfaces[idx].window, false);
                        }
                    }
                }
                if target_slot < count {
                    unsafe {
                        reveal_screenshot_window(
                            &self.screenshot_surfaces[target_slot].window,
                            true,
                        );
                    }
                }
            }
            AppUserEvent::RevealScreenshotSlot { .. }
            | AppUserEvent::ScreenshotRevealWatchdog(_) => {}
            AppUserEvent::CheckScreenshotFocusLost(seq) => {
                let should_check = {
                    let sess = screenshot_session().lock().unwrap();
                    sess.capture_seq == seq && sess.revealed
                };
                if should_check {
                    let wins: Vec<Window> = Vec::new();
                    let _ = wins;
                    // Focus is managed directly via WindowEvent::Focused
                }
            }
            AppUserEvent::ScreenshotCaptureFailed => {
                self.hide_mini_popover();
                self.dashboard_ui.switch_tab(DashboardTab::Settings);
                self.dashboard_ui.show_toast(tr(
                    "⚠️ Please grant Screen Recording permission in Settings → macOS Permissions.",
                    "⚠️ Vui lòng cấp quyền Ghi màn hình (Screen Recording) trong mục Cài đặt.",
                ));
                self.show_main_dashboard();
            }
            AppUserEvent::FullscreenInstantCopied { display_name } => {
                self.dashboard_ui.show_toast(format!(
                    "📸 {} ({})",
                    tr(
                        "Captured fullscreen & copied to Clipboard!",
                        "Đã chụp toàn màn hình & Copy vào Clipboard!"
                    ),
                    display_name
                ));
                if let Some(main) = &self.main_surface {
                    if main.window.is_visible().unwrap_or(false) {
                        main.window.request_redraw();
                    }
                }
            }
            AppUserEvent::SetActiveScreenshotSlot(active_slot) => {
                let (changed, active_count) = {
                    let mut sess = screenshot_session().lock().unwrap();
                    let count = sess.expected_slots.min(self.screenshot_surfaces.len());
                    if !sess.revealed || sess.active_slot == Some(active_slot) {
                        (false, count)
                    } else {
                        sess.active_slot = Some(active_slot);
                        (true, count)
                    }
                };
                if !changed {
                    return;
                }
                self.screenshot_ui.clear_other_selections(active_slot);
                for (idx, surf) in self
                    .screenshot_surfaces
                    .iter()
                    .enumerate()
                    .take(active_count)
                {
                    if idx == active_slot {
                        unsafe {
                            focus_and_elevate_macos_window(&surf.window, true, true);
                        }
                    }
                    surf.window.request_redraw();
                }
            }
            AppUserEvent::HideScreenshotOverlay => {
                self.hide_screenshot_overlays();
            }
            AppUserEvent::SnapOwnMainWindow(layout) => {
                if let Some(main) = &self.main_surface {
                    if core_input::get_input_config().window_snapping_enabled
                        && main.window.is_visible().unwrap_or(false)
                    {
                        unsafe {
                            snap_nswindow_directly(&main.window, &layout);
                        }
                        main.window.request_redraw();
                    }
                }
            }
            AppUserEvent::HideAllOverlays => {
                self.hide_mini_popover();
                self.hide_clipboard_drawer();
                self.hide_alttab_overlay();
                self.hide_screenshot_overlays();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Wait);

        let main_id = self.main_surface.as_ref().map(|s| s.window.id());
        let mini_id = self.mini_surface.as_ref().map(|s| s.window.id());
        let clip_id = self.clip_surface.as_ref().map(|s| s.window.id());
        let alttab_id = self.alttab_surface.as_ref().map(|s| s.window.id());
        let screenshot_idx = self
            .screenshot_surfaces
            .iter()
            .position(|s| s.window.id() == window_id);

        // Pass input event to the matching window's egui state
        if Some(window_id) == main_id {
            if let Some(surf) = &mut self.main_surface {
                surf.handle_window_event(&event);
            }
        } else if Some(window_id) == mini_id {
            if let Some(surf) = &mut self.mini_surface {
                surf.handle_window_event(&event);
            }
        } else if Some(window_id) == clip_id {
            if let Some(surf) = &mut self.clip_surface {
                surf.handle_window_event(&event);
            }
        } else if Some(window_id) == alttab_id {
            if let Some(surf) = &mut self.alttab_surface {
                surf.handle_window_event(&event);
            }
        } else if let Some(idx) = screenshot_idx {
            if let Some(surf) = self.screenshot_surfaces.get_mut(idx) {
                surf.handle_window_event(&event);
            }
        }

        match event {
            WindowEvent::Resized(new_size) => {
                if Some(window_id) == main_id {
                    if let Some(surf) = &mut self.main_surface {
                        surf.resize(&self.gpu, new_size.width, new_size.height);
                        surf.window.request_redraw();
                    }
                } else if Some(window_id) == mini_id {
                    if let Some(surf) = &mut self.mini_surface {
                        surf.resize(&self.gpu, new_size.width, new_size.height);
                        surf.window.request_redraw();
                    }
                } else if Some(window_id) == clip_id {
                    if let Some(surf) = &mut self.clip_surface {
                        surf.resize(&self.gpu, new_size.width, new_size.height);
                        surf.window.request_redraw();
                    }
                } else if Some(window_id) == alttab_id {
                    if let Some(surf) = &mut self.alttab_surface {
                        surf.resize(&self.gpu, new_size.width, new_size.height);
                        surf.window.request_redraw();
                    }
                } else if let Some(idx) = screenshot_idx {
                    if let Some(surf) = self.screenshot_surfaces.get_mut(idx) {
                        surf.resize(&self.gpu, new_size.width, new_size.height);
                        surf.window.request_redraw();
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if Some(window_id) == main_id {
                    if let Some(surf) = &mut self.main_surface {
                        let sz = surf.window.inner_size();
                        surf.resize(&self.gpu, sz.width, sz.height);
                        surf.window.request_redraw();
                    }
                } else if Some(window_id) == mini_id {
                    if let Some(surf) = &mut self.mini_surface {
                        let sz = surf.window.inner_size();
                        surf.resize(&self.gpu, sz.width, sz.height);
                        surf.window.request_redraw();
                    }
                } else if Some(window_id) == clip_id {
                    if let Some(surf) = &mut self.clip_surface {
                        let sz = surf.window.inner_size();
                        surf.resize(&self.gpu, sz.width, sz.height);
                        surf.window.request_redraw();
                    }
                } else if Some(window_id) == alttab_id {
                    if let Some(surf) = &mut self.alttab_surface {
                        let sz = surf.window.inner_size();
                        surf.resize(&self.gpu, sz.width, sz.height);
                        surf.window.request_redraw();
                    }
                } else if let Some(idx) = screenshot_idx {
                    if let Some(surf) = self.screenshot_surfaces.get_mut(idx) {
                        let sz = surf.window.inner_size();
                        surf.resize(&self.gpu, sz.width, sz.height);
                        surf.window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let proxy = self.proxy.clone();
                let mut delay = std::time::Duration::MAX;
                if Some(window_id) == main_id {
                    if let Some(surf) = &mut self.main_surface {
                        let ui_state = &mut self.dashboard_ui;
                        delay = surf.render_frame(&self.gpu, [0.06, 0.09, 0.16, 1.0], |ctx| {
                            ui_state.render(ctx, &proxy);
                        });
                    }
                } else if Some(window_id) == mini_id {
                    if let Some(surf) = &mut self.mini_surface {
                        let ui_state = &mut self.mini_ui;
                        delay = surf.render_frame(&self.gpu, [0.0, 0.0, 0.0, 0.0], |ctx| {
                            ui_state.render(ctx, &proxy);
                        });
                    }
                } else if Some(window_id) == clip_id {
                    if let Some(surf) = &mut self.clip_surface {
                        let ui_state = &mut self.clip_ui;
                        delay = surf.render_frame(&self.gpu, [0.0, 0.0, 0.0, 0.0], |ctx| {
                            ui_state.render(ctx, &proxy);
                        });
                    }
                } else if Some(window_id) == alttab_id {
                    if let Some(surf) = &mut self.alttab_surface {
                        let ui_state = &mut self.alttab_ui;
                        delay = surf.render_frame(&self.gpu, [0.0, 0.0, 0.0, 0.0], |ctx| {
                            ui_state.render(ctx, &proxy);
                        });
                    }
                } else if let Some(idx) = screenshot_idx {
                    if let (Some(surf), Some(ui_slot)) = (
                        self.screenshot_surfaces.get_mut(idx),
                        self.screenshot_ui.slots.get_mut(idx),
                    ) {
                        delay = surf.render_frame(&self.gpu, [0.0, 0.0, 0.0, 1.0], |ctx| {
                            ui_slot.render(ctx, &proxy);
                        });
                    }
                }
                self.record_repaint_delay(delay);
            }
            WindowEvent::CloseRequested => {
                if Some(window_id) == clip_id {
                    self.hide_clipboard_drawer();
                } else if Some(window_id) == alttab_id {
                    self.hide_alttab_overlay();
                } else if Some(window_id) == mini_id {
                    self.hide_mini_popover();
                } else if screenshot_idx.is_some() {
                    self.hide_screenshot_overlays();
                } else if Some(window_id) == main_id {
                    let cfg = core_input::get_input_config();
                    if cfg.keep_running_in_background {
                        self.hide_main_dashboard();
                    } else {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Focused(true) => {
                if Some(window_id) == main_id {
                    core_input::notify_permissions_changed();
                    self.dashboard_ui.on_focused();
                    if let Some(surf) = &self.main_surface {
                        surf.window.request_redraw();
                    }
                }
            }
            WindowEvent::Focused(false) => {
                if Some(window_id) == mini_id {
                    self.hide_mini_popover();
                } else if Some(window_id) == clip_id {
                    self.hide_clipboard_drawer();
                } else if Some(window_id) == alttab_id {
                    self.hide_alttab_overlay();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.next_repaint_deadline {
            if std::time::Instant::now() >= deadline {
                self.next_repaint_deadline = None;
                self.request_redraw_all_visible();
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() {
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    // 0. Enforce single instance & disable macOS reboot window restoration
    ensure_single_instance(start_minimized);
    unsafe {
        disable_macos_window_restoration();
    }

    // 1. Sync clipboard enabled state & start the 250ms zero-leak NSPasteboard clipboard watcher thread
    let initial_cfg = core_input::get_input_config();
    core_clipboard::set_clipboard_enabled(initial_cfg.clipboard_enabled);
    core_clipboard::start_clipboard_watcher();
    std::thread::spawn(|| {
        core_window::register_current_app_with_launch_services();
    });

    // 2. Build the native macOS Winit event loop
    let event_loop = EventLoop::<AppUserEvent>::with_user_event()
        .build()
        .expect("Failed to create Winit EventLoop");
    let proxy = event_loop.create_proxy();

    // 3. Create native macOS Menu Bar item right next to the MacBook Battery % icon!
    init_native_macos_menubar_item(proxy.clone());
    let _ = crate::menubar::sync_menubar_from_config(&initial_cfg);

    let menubar_ticker_proxy = proxy.clone();
    std::thread::Builder::new()
        .name("menubar-status-ticker".to_string())
        .spawn(move || loop {
            let cfg = core_input::get_input_config();
            let interval_secs = cfg.menubar_refresh_interval_sec.clamp(1, 30) as u64;
            if cfg.menubar_enabled
                && cfg.menubar_display_mode != "hidden"
                && crate::menubar::menubar_mode_needs_live_stats(&cfg)
            {
                let stats = core_monitor::get_menubar_stats();
                let title = crate::menubar::format_menubar_title(&cfg, &stats);
                let _ = menubar_ticker_proxy.send_event(AppUserEvent::UpdateMenuBarTitle(title));
            }
            std::thread::sleep(std::time::Duration::from_secs(interval_secs));
        })
        .expect("Failed to spawn menubar status ticker thread");

    // 4. Register Carbon Global HotKeys (Option+Tab, Cmd+Shift+V, F3, Window Tiling) & HID CGEventTap
    let tap_proxy = proxy.clone();
    core_input::register_overlay_callback(move |ev| match ev {
        core_input::UiOverlayEvent::ToggleClipboardLeftDrawer => {
            let _ = tap_proxy.send_event(AppUserEvent::ToggleClipboardDrawer);
        }
        core_input::UiOverlayEvent::ToggleAltTabOverlay => {
            let _ = tap_proxy.send_event(AppUserEvent::ToggleAltTabOverlay);
        }
        core_input::UiOverlayEvent::CommitAltTabSelection => {
            let _ = tap_proxy.send_event(AppUserEvent::CommitAltTabSelection);
        }
        core_input::UiOverlayEvent::TriggerScreenshotOverlay => {
            let _ = tap_proxy.send_event(AppUserEvent::TriggerScreenshotOverlay);
        }
        core_input::UiOverlayEvent::TriggerFullscreenEditScreenshot => {
            let _ = tap_proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                display_id: 0,
                instant_copy: false,
            });
        }
        core_input::UiOverlayEvent::TriggerFullscreenCopyScreenshot => {
            let _ = tap_proxy.send_event(AppUserEvent::TriggerFullscreenScreenshot {
                display_id: 0,
                instant_copy: true,
            });
        }
        core_input::UiOverlayEvent::SnapOwnWindow(layout) => {
            let _ = tap_proxy.send_event(AppUserEvent::SnapOwnMainWindow(layout));
        }
        core_input::UiOverlayEvent::ToggleMainDashboard => {
            let _ = tap_proxy.send_event(AppUserEvent::ToggleLauncher);
        }
        core_input::UiOverlayEvent::HideAllOverlays => {
            let _ = tap_proxy.send_event(AppUserEvent::HideAllOverlays);
        }
    });
    core_input::register_native_carbon_hotkeys();
    core_input::start_global_input_tap();

    // 5. Run the Pure Rust GPU Application
    let mut app = OmniGpuApp::new(start_minimized, proxy);
    event_loop
        .run_app(&mut app)
        .expect("Failed to run Pure Rust GPU EventLoop");
}
