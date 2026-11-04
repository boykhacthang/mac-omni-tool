mod events;
mod macos_window;
mod menubar;
mod rpc;
mod single_instance;
mod ui_assets;
mod updater;
mod webview;

use crate::events::{screenshot_session, AppUserEvent, ScreenshotSlotData, MAX_SCREENSHOT_SLOTS};
use crate::macos_window::{
    disable_macos_window_restoration, focus_and_elevate_macos_window,
    park_overlay_window_offscreen, place_main_window_center_75_full_height,
    prepare_screenshot_window_hidden, restore_previous_frontmost_app, reveal_screenshot_window,
    set_macos_dock_icon_visible, set_nswindow_non_restorable, set_nswindow_rounded_transparent,
    snap_nswindow_directly,
};
use crate::menubar::{
    init_native_macos_menubar_item, setup_macos_application_menu, update_status_bar_title,
};
use crate::single_instance::ensure_single_instance;
use crate::webview::build_webview_with_api;

use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

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
    core_cleaner::prewarm_cleaner_cache();

    // 2. Build the native macOS event loop
    let event_loop = EventLoopBuilder::<AppUserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // 3. Create native macOS Menu Bar item right next to the MacBook Battery % icon!
    init_native_macos_menubar_item(proxy.clone());

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
        core_input::UiOverlayEvent::HideAllOverlays => {
            let _ = tap_proxy.send_event(AppUserEvent::HideAllOverlays);
        }
    });
    core_input::register_native_carbon_hotkeys();
    core_input::start_global_input_tap();

    let cur_disp = core_display::get_display_at_mouse_cursor();

    // Window 0: Menu Bar Dropdown Popover (hidden by default; drops down below the Menu Bar icon near Battery % when clicked)
    let mini_w = 384.0;
    let mut mini_h = 430.0;
    let mut mini_pos = (
        cur_disp.x as f64 + (cur_disp.width as f64 - (mini_w + 16.0)).max(20.0),
        cur_disp.y as f64 + 28.0,
    );
    let mini_window = WindowBuilder::new()
        .with_title("Mac Omni Menu Bar")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(mini_w, mini_h))
        .with_position(LogicalPosition::new(mini_pos.0, mini_pos.1))
        .with_visible(false)
        .build(&event_loop)
        .expect("Failed to create menu bar popover window");
    unsafe {
        set_nswindow_rounded_transparent(&mini_window, 14.0);
    }

    let mini_webview = build_webview_with_api(
        &mini_window,
        "omni://localhost/index.html?mode=mini_widget",
        proxy.clone(),
    )
    .expect("Failed to build menu bar popover WebView");
    unsafe {
        set_nswindow_rounded_transparent(&mini_window, 14.0);
    }

    // Window 1: Main Launcher Dashboard Window (Default: Center 75% width, full vertical height right against the Dock)
    let main_w_init = ((cur_disp.width as f64) * 0.75).round().max(900.0);
    let main_h_init = (cur_disp.height as f64 - 72.0).max(600.0);
    let main_window = WindowBuilder::new()
        .with_title("Mac Omni Tool")
        .with_inner_size(LogicalSize::new(main_w_init, main_h_init))
        .with_min_inner_size(LogicalSize::new(420.0, 320.0))
        .with_visible(!start_minimized)
        .build(&event_loop)
        .expect("Failed to create main launcher window");
    unsafe {
        set_nswindow_non_restorable(&main_window);
        place_main_window_center_75_full_height(&main_window);
        set_macos_dock_icon_visible(!start_minimized);
    }

    let main_webview = build_webview_with_api(
        &main_window,
        "omni://localhost/index.html?mode=main",
        proxy.clone(),
    )
    .expect("Failed to build main WebView");
    unsafe {
        place_main_window_center_75_full_height(&main_window);
        setup_macos_application_menu();
    }

    // Window 2: Left-Side Clipboard Drawer Popup (Cmd + Shift + V -> Docked on LEFT edge of current mouse monitor)
    let drawer_h = (cur_disp.height as f64 - 84.0).clamp(520.0, 900.0);
    let clip_window = WindowBuilder::new()
        .with_title("Clipboard Manager (Cmd+Shift+V)")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(450.0, drawer_h))
        .with_position(LogicalPosition::new(
            cur_disp.x as f64 + 12.0,
            cur_disp.y as f64 + 38.0,
        ))
        .with_visible(false)
        .build(&event_loop)
        .expect("Failed to create left clipboard drawer window");
    unsafe {
        set_nswindow_rounded_transparent(&clip_window, 14.0);
    }

    let clip_webview = build_webview_with_api(
        &clip_window,
        "omni://localhost/index.html?mode=clipboard_drawer",
        proxy.clone(),
    )
    .expect("Failed to build clipboard WebView");
    unsafe {
        set_nswindow_rounded_transparent(&clip_window, 14.0);
    }

    // Window 3: Alt-Tab Live Preview Overlay Window (Option + Tab / Cmd + Tab -> Centered on current mouse monitor)
    let alttab_w = 920.0;
    let alttab_h = 520.0;
    let alttab_window = WindowBuilder::new()
        .with_title("Alt-Tab Live Preview")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(alttab_w, alttab_h))
        .with_position(LogicalPosition::new(
            cur_disp.x as f64 + ((cur_disp.width as f64 - alttab_w) / 2.0).max(20.0),
            cur_disp.y as f64 + ((cur_disp.height as f64 - alttab_h) / 2.0).max(20.0),
        ))
        .with_visible(false)
        .build(&event_loop)
        .expect("Failed to create Alt-Tab overlay window");
    unsafe {
        set_nswindow_rounded_transparent(&alttab_window, 14.0);
    }

    let alttab_webview = build_webview_with_api(
        &alttab_window,
        "omni://localhost/index.html?mode=alttab",
        proxy.clone(),
    )
    .expect("Failed to build Alt-Tab WebView");
    unsafe {
        set_nswindow_rounded_transparent(&alttab_window, 14.0);
    }

    // Window Pool 4: Multi-Monitor Screenshot Overlay Windows
    let mut screenshot_windows = Vec::with_capacity(MAX_SCREENSHOT_SLOTS);
    let mut screenshot_webviews = Vec::with_capacity(MAX_SCREENSHOT_SLOTS);
    let mut screenshot_ids = Vec::with_capacity(MAX_SCREENSHOT_SLOTS);

    for slot in 0..MAX_SCREENSHOT_SLOTS {
        let win = WindowBuilder::new()
            .with_title(&format!("Mac Omni Screenshot #{}", slot + 1))
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(false)
            .with_inner_size(LogicalSize::new(
                cur_disp.width as f64,
                cur_disp.height as f64,
            ))
            .with_position(LogicalPosition::new(cur_disp.x as f64, cur_disp.y as f64))
            .with_visible(false)
            .build(&event_loop)
            .expect("Failed to create screenshot overlay window");

        unsafe {
            set_nswindow_rounded_transparent(&win, 0.0);
            park_overlay_window_offscreen(&win);
        }

        let url = format!(
            "omni://localhost/index.html?mode=screenshot_overlay&slot={}",
            slot
        );
        let wv = build_webview_with_api(&win, &url, proxy.clone())
            .expect("Failed to build Screenshot WebView");

        unsafe {
            set_nswindow_rounded_transparent(&win, 0.0);
            park_overlay_window_offscreen(&win);
        }

        screenshot_ids.push(win.id());
        screenshot_windows.push(win);
        screenshot_webviews.push(wv);
    }

    let main_id = main_window.id();
    let clip_id = clip_window.id();
    let alttab_id = alttab_window.id();
    let mini_id = mini_window.id();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(user_ev) => match user_ev {
                AppUserEvent::ToggleMenuBarPopover => {
                    let vis = mini_window.is_visible();
                    if vis {
                        mini_window.set_visible(false);
                    } else {
                        let (mx, _my) = core_display::get_mouse_cursor_position();
                        let disp = core_display::get_display_at_mouse_cursor();
                        let min_x = disp.x as f64 + 12.0;
                        let max_x = (disp.x as f64 + disp.width as f64 - (mini_w + 12.0)).max(min_x);
                        let px = (mx - (mini_w / 2.0)).clamp(min_x, max_x);
                        let py = disp.y as f64 + 28.0;
                        mini_pos = (px, py);
                        mini_window.set_inner_size(LogicalSize::new(mini_w, mini_h));
                        mini_window.set_outer_position(LogicalPosition::new(px, py));
                        unsafe {
                            set_nswindow_rounded_transparent(&mini_window, 14.0);
                        }
                        mini_window.set_visible(true);
                        mini_window.set_focus();
                        let _ = mini_webview.evaluate_script(
                            "window.__refreshMiniPopover && window.__refreshMiniPopover();",
                        );
                    }
                }
                AppUserEvent::SetMiniExpanded(expanded) => {
                    mini_window.set_visible(expanded);
                }
                AppUserEvent::SetMiniHeight(h) => {
                    let clamped_h = h.clamp(310.0, 640.0).round();
                    if (clamped_h - mini_h).abs() > 1.0 {
                        mini_h = clamped_h;
                        mini_window.set_inner_size(LogicalSize::new(mini_w, mini_h));
                        mini_window.set_outer_position(LogicalPosition::new(mini_pos.0, mini_pos.1));
                        unsafe {
                            set_nswindow_rounded_transparent(&mini_window, 14.0);
                        }
                    }
                }
                AppUserEvent::UpdateMenuBarTitle(title) => {
                    update_status_bar_title(&title);
                }
                AppUserEvent::SetAppTheme(theme) => {
                    let script = format!(
                        "window.__applyThemeFromRust && window.__applyThemeFromRust('{}');",
                        if theme == "light" { "light" } else { "dark" }
                    );
                    let _ = main_webview.evaluate_script(&script);
                    let _ = mini_webview.evaluate_script(&script);
                    let _ = clip_webview.evaluate_script(&script);
                    let _ = alttab_webview.evaluate_script(&script);
                }
                AppUserEvent::SetAppLang(lang) => {
                    let safe_lang = if lang == "vi" { "vi" } else { "en" };
                    let script = format!(
                        "window.__applyLangFromRust && window.__applyLangFromRust('{}');",
                        safe_lang
                    );
                    let _ = main_webview.evaluate_script(&script);
                    let _ = mini_webview.evaluate_script(&script);
                    let _ = clip_webview.evaluate_script(&script);
                    let _ = alttab_webview.evaluate_script(&script);
                    for wv in &screenshot_webviews {
                        let _ = wv.evaluate_script(&script);
                    }
                }
                AppUserEvent::ShowLauncher => {
                    unsafe {
                        set_macos_dock_icon_visible(true);
                        place_main_window_center_75_full_height(&main_window);
                        setup_macos_application_menu();
                    }
                    main_window.set_visible(true);
                    main_window.set_focus();
                }
                AppUserEvent::ShowAboutDialog => {
                    unsafe {
                        set_macos_dock_icon_visible(true);
                        setup_macos_application_menu();
                    }
                    if !main_window.is_visible() {
                        unsafe {
                            place_main_window_center_75_full_height(&main_window);
                        }
                        main_window.set_visible(true);
                    }
                    main_window.set_focus();
                    let _ = main_webview.evaluate_script(
                        "window.__showAboutModal && window.__showAboutModal();",
                    );
                }
                AppUserEvent::OpenDashboardTab(tab) => {
                    unsafe {
                        set_macos_dock_icon_visible(true);
                        setup_macos_application_menu();
                    }
                    if !main_window.is_visible() {
                        unsafe {
                            place_main_window_center_75_full_height(&main_window);
                        }
                        main_window.set_visible(true);
                    }
                    main_window.set_focus();
                    let safe_tab = serde_json::to_string(&tab).unwrap_or_else(|_| "\"settings\"".to_string());
                    let _ = main_webview.evaluate_script(&format!(
                        "window.switchMainTab && window.switchMainTab({});",
                        safe_tab
                    ));
                }
                AppUserEvent::HideLauncher => {
                    main_window.set_visible(false);
                    unsafe {
                        set_macos_dock_icon_visible(false);
                    }
                }
                AppUserEvent::QuitApp => {
                    *control_flow = ControlFlow::Exit;
                }
                AppUserEvent::ToggleClipboardDrawer => {
                    let vis = clip_window.is_visible();
                    if vis {
                        clip_window.set_visible(false);
                        if !alttab_window.is_visible()
                            && !screenshot_windows.iter().any(|w| w.is_visible())
                        {
                            core_input::set_overlay_escape_hotkey_active(false);
                        }
                    } else {
                        let disp = core_display::get_display_at_mouse_cursor();
                        let h = (disp.height as f64 - 84.0).clamp(520.0, 900.0);
                        clip_window.set_inner_size(LogicalSize::new(450.0, h));
                        clip_window.set_outer_position(LogicalPosition::new(
                            disp.x as f64 + 12.0,
                            disp.y as f64 + 38.0,
                        ));
                        clip_window.set_visible(true);
                        clip_window.set_focus();
                        core_input::set_overlay_escape_hotkey_active(true);
                        let _ = clip_webview
                            .evaluate_script("window.__refreshDrawer && window.__refreshDrawer();");
                    }
                }
                AppUserEvent::HideClipboardDrawer => {
                    clip_window.set_visible(false);
                    if !alttab_window.is_visible()
                        && !screenshot_windows.iter().any(|w| w.is_visible())
                    {
                        core_input::set_overlay_escape_hotkey_active(false);
                    }
                }
                AppUserEvent::ToggleAltTabOverlay => {
                    let vis = alttab_window.is_visible();
                    if vis {
                        let _ = alttab_webview.evaluate_script(
                            "window.__cycleAltTabNext && window.__cycleAltTabNext();",
                        );
                    } else {
                        let disp = core_display::get_display_at_mouse_cursor();
                        let ax = disp.x as f64 + ((disp.width as f64 - alttab_w) / 2.0).max(20.0);
                        let ay = disp.y as f64 + ((disp.height as f64 - alttab_h) / 2.0).max(20.0);
                        alttab_window.set_outer_position(LogicalPosition::new(ax, ay));
                        alttab_window.set_visible(true);
                        alttab_window.set_focus();
                        core_input::set_overlay_escape_hotkey_active(true);
                        let _ = alttab_webview
                            .evaluate_script("window.__refreshAltTab && window.__refreshAltTab();");
                    }
                }
                AppUserEvent::CommitAltTabSelection => {
                    if alttab_window.is_visible() {
                        let _ = alttab_webview.evaluate_script(
                            "window.__commitAltTabSelection && window.__commitAltTabSelection();",
                        );
                    }
                }
                AppUserEvent::HideAltTabOverlay => {
                    alttab_window.set_visible(false);
                    if !clip_window.is_visible()
                        && !screenshot_windows.iter().any(|w| w.is_visible())
                    {
                        core_input::set_overlay_escape_hotkey_active(false);
                    }
                }
                AppUserEvent::TriggerFullscreenScreenshot {
                    display_id,
                    instant_copy,
                } => {
                    let cfg = core_input::get_input_config();
                    if !cfg.screenshot_enabled {
                        return;
                    }
                    if screenshot_windows.iter().any(|w| w.is_visible()) {
                        return;
                    }
                    {
                        let mut sess = screenshot_session().lock().unwrap();
                        if sess.capturing {
                            return;
                        }
                        sess.capturing = true;
                    }
                    if mini_window.is_visible() {
                        mini_window.set_visible(false);
                    }
                    if clip_window.is_visible() {
                        clip_window.set_visible(false);
                    }
                    if alttab_window.is_visible() {
                        alttab_window.set_visible(false);
                    }

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

                    let bg_proxy = proxy.clone();
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
                                    let _ = bg_proxy.send_event(
                                        AppUserEvent::FullscreenInstantCopied {
                                            display_name: target_disp.name,
                                        },
                                    );
                                }
                                Err(_) => {
                                    let _ =
                                        bg_proxy.send_event(AppUserEvent::ScreenshotCaptureFailed);
                                }
                            }
                        });
                    } else {
                        std::thread::spawn(move || {
                            match core_display::capture_all_displays_png() {
                                Ok(captured_list) => {
                                    let seq = {
                                        let mut sess = screenshot_session().lock().unwrap();
                                        sess.capturing = false;
                                        sess.capture_seq += 1;
                                        let mut found_target = 0usize;
                                        sess.slots = captured_list
                                            .iter()
                                            .take(MAX_SCREENSHOT_SLOTS)
                                            .enumerate()
                                            .map(|(idx, (d, bytes))| {
                                                let is_target = d.id == target_disp.id;
                                                if is_target {
                                                    found_target = idx;
                                                }
                                                ScreenshotSlotData {
                                                    display_id: d.id,
                                                    display_name: d.name.clone(),
                                                    png_bytes: bytes.clone(),
                                                    x: d.x,
                                                    y: d.y,
                                                    width: d.width,
                                                    height: d.height,
                                                    preselect_fullscreen: is_target,
                                                }
                                            })
                                            .collect();
                                        let count = sess.slots.len();
                                        sess.expected_slots = count;
                                        sess.ready_slots = vec![false; count];
                                        sess.revealed = false;
                                        sess.target_slot = found_target;
                                        sess.active_slot = Some(found_target);
                                        sess.prev_frontmost_pid = prev_pid;
                                        sess.capture_seq
                                    };
                                    let _ = bg_proxy.send_event(
                                        AppUserEvent::PresentScreenshotSession {
                                            seq,
                                            fullscreen_picker_mode: true,
                                        },
                                    );
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
                    if screenshot_windows.iter().any(|w| w.is_visible()) {
                        return;
                    }
                    {
                        let mut sess = screenshot_session().lock().unwrap();
                        if sess.capturing {
                            return;
                        }
                        sess.capturing = true;
                    }
                    if mini_window.is_visible() {
                        mini_window.set_visible(false);
                    }
                    if clip_window.is_visible() {
                        clip_window.set_visible(false);
                    }
                    if alttab_window.is_visible() {
                        alttab_window.set_visible(false);
                    }

                    let prev_pid = core_window::frontmost_application_pid();
                    let mouse_disp = core_display::get_display_at_mouse_cursor();
                    let bg_proxy = proxy.clone();

                    std::thread::spawn(move || {
                        match core_display::capture_all_displays_png() {
                            Ok(captured_list) => {
                                let seq = {
                                    let mut sess = screenshot_session().lock().unwrap();
                                    sess.capturing = false;
                                    sess.capture_seq += 1;
                                    let mut found_target = 0usize;
                                    sess.slots = captured_list
                                        .iter()
                                        .take(MAX_SCREENSHOT_SLOTS)
                                        .enumerate()
                                        .map(|(idx, (d, bytes))| {
                                            if d.id == mouse_disp.id {
                                                found_target = idx;
                                            }
                                            ScreenshotSlotData {
                                                display_id: d.id,
                                                display_name: d.name.clone(),
                                                png_bytes: bytes.clone(),
                                                x: d.x,
                                                y: d.y,
                                                width: d.width,
                                                height: d.height,
                                                preselect_fullscreen: false,
                                            }
                                        })
                                        .collect();
                                    let count = sess.slots.len();
                                    sess.expected_slots = count;
                                    sess.ready_slots = vec![false; count];
                                    sess.revealed = false;
                                    sess.target_slot = found_target;
                                    sess.active_slot = Some(found_target);
                                    sess.prev_frontmost_pid = prev_pid;
                                    sess.capture_seq
                                };
                                let _ =
                                    bg_proxy.send_event(AppUserEvent::PresentScreenshotSession {
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
                    let slot_infos: Vec<(usize, i32, i32, usize, usize, String, bool)> = {
                        let sess = screenshot_session().lock().unwrap();
                        if sess.capture_seq != seq || sess.slots.is_empty() {
                            return;
                        }
                        sess.slots
                            .iter()
                            .enumerate()
                            .map(|(idx, s)| {
                                (
                                    idx,
                                    s.x,
                                    s.y,
                                    s.width,
                                    s.height,
                                    s.display_name.clone(),
                                    s.preselect_fullscreen,
                                )
                            })
                            .collect()
                    };

                    core_input::set_overlay_escape_hotkey_active(true);

                    for (slot_idx, x, y, width, height, disp_name, preselect_full) in slot_infos {
                        let win = &screenshot_windows[slot_idx];
                        let wv = &screenshot_webviews[slot_idx];

                        unsafe {
                            prepare_screenshot_window_hidden(win, x, y, width, height);
                        }

                        let safe_name = serde_json::to_string(&disp_name)
                            .unwrap_or_else(|_| "\"Display\"".to_string());
                        let script = format!(
                            "window.__startScreenshotSession && window.__startScreenshotSession({}, {}, {}, {}, {}, {}, {});",
                            width,
                            height,
                            seq,
                            slot_idx,
                            if preselect_full { "true" } else { "false" },
                            safe_name,
                            if fullscreen_picker_mode { "true" } else { "false" }
                        );
                        let _ = wv.evaluate_script(&script);
                    }

                    // Safety fallback: ensure overlay reveals within 120ms even if a secondary monitor's rAF is throttled
                    let fallback_proxy = proxy.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(120));
                        let _ = fallback_proxy.send_event(AppUserEvent::RevealScreenshotSlot {
                            slot: usize::MAX,
                            seq,
                        });
                    });
                }
                AppUserEvent::RevealScreenshotSlot { slot, seq } => {
                    let reveal_params = {
                        let mut sess = screenshot_session().lock().unwrap();
                        if sess.capture_seq != seq || sess.revealed || sess.slots.is_empty() {
                            None
                        } else {
                            if slot < sess.ready_slots.len() {
                                sess.ready_slots[slot] = true;
                            }
                            let all_ready =
                                slot == usize::MAX || sess.ready_slots.iter().all(|&r| r);
                            if all_ready {
                                sess.revealed = true;
                                Some((sess.expected_slots, sess.target_slot))
                            } else {
                                None
                            }
                        }
                    };

                    if let Some((num_slots, target_slot)) = reveal_params {
                        let count = num_slots.min(MAX_SCREENSHOT_SLOTS);
                        for idx in 0..count {
                            if idx != target_slot {
                                unsafe {
                                    reveal_screenshot_window(&screenshot_windows[idx], false);
                                }
                            }
                        }
                        if target_slot < count {
                            unsafe {
                                reveal_screenshot_window(&screenshot_windows[target_slot], true);
                            }
                        }
                    }
                }
                AppUserEvent::ScreenshotCaptureFailed => {
                    core_window::request_or_open_permission("screen_recording");
                }
                AppUserEvent::FullscreenInstantCopied { display_name } => {
                    let safe_en = serde_json::to_string(&format!(
                        "📸 Captured fullscreen ({}) & copied to Clipboard!",
                        display_name
                    ))
                    .unwrap_or_else(|_| {
                        "\"📸 Captured fullscreen & copied to Clipboard!\"".to_string()
                    });
                    let safe_vi = serde_json::to_string(&format!(
                        "📸 Đã chụp toàn màn hình ({}) & Copy vào Clipboard!",
                        display_name
                    ))
                    .unwrap_or_else(|_| {
                        "\"📸 Đã chụp toàn màn hình & Copy vào Clipboard!\"".to_string()
                    });
                    let _ = main_webview.evaluate_script(&format!(
                        "window.showToast && window.showToast(window.tr ? window.tr({}, {}) : {});",
                        safe_en, safe_vi, safe_en
                    ));
                }
                AppUserEvent::SetActiveScreenshotSlot(active_slot) => {
                    let changed = {
                        let mut sess = screenshot_session().lock().unwrap();
                        if sess.active_slot == Some(active_slot) {
                            false
                        } else {
                            sess.active_slot = Some(active_slot);
                            true
                        }
                    };
                    if !changed {
                        return;
                    }
                    for (idx, win) in screenshot_windows.iter().enumerate() {
                        if !win.is_visible() {
                            continue;
                        }
                        if idx == active_slot {
                            unsafe {
                                focus_and_elevate_macos_window(win, true, true);
                            }
                        } else if let Some(other_wv) = screenshot_webviews.get(idx) {
                            let _ = other_wv.evaluate_script(
                                "window.__clearScreenshotSelection && window.__clearScreenshotSelection();",
                            );
                        }
                    }
                }
                AppUserEvent::HideScreenshotOverlay => {
                    let prev_pid = {
                        let mut sess = screenshot_session().lock().unwrap();
                        sess.revealed = false;
                        sess.active_slot = None;
                        sess.capturing = false;
                        std::mem::replace(&mut sess.prev_frontmost_pid, 0)
                    };
                    for win in &screenshot_windows {
                        unsafe {
                            park_overlay_window_offscreen(win);
                        }
                    }
                    for wv in &screenshot_webviews {
                        let _ = wv.evaluate_script(
                            "window.__prepareScreenshotHide && window.__prepareScreenshotHide();",
                        );
                    }
                    unsafe {
                        restore_previous_frontmost_app(&main_window, prev_pid);
                    }
                    if !clip_window.is_visible() && !alttab_window.is_visible() {
                        core_input::set_overlay_escape_hotkey_active(false);
                    }
                }
                AppUserEvent::SnapOwnMainWindow(layout) => {
                    if main_window.is_visible() {
                        unsafe {
                            snap_nswindow_directly(&main_window, &layout);
                        }
                    }
                }
                AppUserEvent::HideAllOverlays => {
                    let prev_pid = {
                        let mut sess = screenshot_session().lock().unwrap();
                        sess.revealed = false;
                        sess.active_slot = None;
                        sess.capturing = false;
                        std::mem::replace(&mut sess.prev_frontmost_pid, 0)
                    };
                    mini_window.set_visible(false);
                    clip_window.set_visible(false);
                    alttab_window.set_visible(false);
                    for win in &screenshot_windows {
                        unsafe {
                            park_overlay_window_offscreen(win);
                        }
                    }
                    for wv in &screenshot_webviews {
                        let _ = wv.evaluate_script(
                            "window.__prepareScreenshotHide && window.__prepareScreenshotHide();",
                        );
                    }
                    unsafe {
                        restore_previous_frontmost_app(&main_window, prev_pid);
                    }
                    core_input::set_overlay_escape_hotkey_active(false);
                }
            },
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if window_id == clip_id {
                    clip_window.set_visible(false);
                    if !alttab_window.is_visible()
                        && !screenshot_windows.iter().any(|w| w.is_visible())
                    {
                        core_input::set_overlay_escape_hotkey_active(false);
                    }
                } else if window_id == alttab_id {
                    alttab_window.set_visible(false);
                    if !clip_window.is_visible()
                        && !screenshot_windows.iter().any(|w| w.is_visible())
                    {
                        core_input::set_overlay_escape_hotkey_active(false);
                    }
                } else if window_id == mini_id {
                    mini_window.set_visible(false);
                } else if screenshot_ids.contains(&window_id) {
                    let prev_pid = {
                        let mut sess = screenshot_session().lock().unwrap();
                        sess.revealed = false;
                        sess.active_slot = None;
                        sess.capturing = false;
                        std::mem::replace(&mut sess.prev_frontmost_pid, 0)
                    };
                    for win in &screenshot_windows {
                        unsafe {
                            park_overlay_window_offscreen(win);
                        }
                    }
                    for wv in &screenshot_webviews {
                        let _ = wv.evaluate_script(
                            "window.__prepareScreenshotHide && window.__prepareScreenshotHide();",
                        );
                    }
                    unsafe {
                        restore_previous_frontmost_app(&main_window, prev_pid);
                    }
                    if !clip_window.is_visible() && !alttab_window.is_visible() {
                        core_input::set_overlay_escape_hotkey_active(false);
                    }
                } else if window_id == main_id {
                    main_window.set_visible(false);
                    unsafe {
                        set_macos_dock_icon_visible(false);
                    }
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Focused(false),
                ..
            } => {
                if window_id == mini_id && mini_window.is_visible() {
                    mini_window.set_visible(false);
                }
            }
            _ => {}
        }
    });
}
