use std::ffi::{c_char, c_void};
use tao::platform::macos::WindowExtMacOS;

const APP_ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub fn objc_allocateClassPair(
        superclass: *mut c_void,
        name: *const c_char,
        extra_bytes: usize,
    ) -> *mut c_void;
    pub fn objc_registerClassPair(cls: *mut c_void);
    pub fn class_addMethod(
        cls: *mut c_void,
        name: *mut c_void,
        imp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
        types: *const c_char,
    ) -> bool;
    pub fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

/// Apply the embedded minimalist AppIcon directly to the macOS Dock & Cmd+Tab switcher
pub unsafe fn set_macos_dock_icon() {
    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    type Msg1Obj = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
    type MsgData =
        unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8, usize) -> *mut c_void;

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_obj: Msg1Obj = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_data: MsgData = std::mem::transmute(objc_msgSend as *const c_void);

    let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
    let ns_data_cls = objc_getClass(c"NSData".as_ptr());
    let ns_img_cls = objc_getClass(c"NSImage".as_ptr());

    if !ns_app_cls.is_null() && !ns_data_cls.is_null() && !ns_img_cls.is_null() {
        let data = msg_data(
            ns_data_cls,
            sel_registerName(c"dataWithBytes:length:".as_ptr()),
            APP_ICON_PNG.as_ptr(),
            APP_ICON_PNG.len(),
        );
        if !data.is_null() {
            let img_alloc = msg0(ns_img_cls, sel_registerName(c"alloc".as_ptr()));
            let img = msg1_obj(
                img_alloc,
                sel_registerName(c"initWithData:".as_ptr()),
                data,
            );
            if !img.is_null() {
                let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
                if !ns_app.is_null() {
                    msg1_ptr(
                        ns_app,
                        sel_registerName(c"setApplicationIconImage:".as_ptr()),
                        img,
                    );
                }
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Show or hide the app icon in the macOS Dock (`NSApplicationActivationPolicyRegular` = 0 vs `Accessory` = 1)
pub unsafe fn set_macos_dock_icon_visible(visible: bool) {
    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> bool;
    type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, i8);

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);

    let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
    if !ns_app_cls.is_null() {
        let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
        if !ns_app.is_null() {
            // 0 = NSApplicationActivationPolicyRegular (shows on Dock)
            // 1 = NSApplicationActivationPolicyAccessory (hides from Dock, keeps Menu Bar item)
            let policy: isize = if visible { 0 } else { 1 };
            let _ = msg1_isize(
                ns_app,
                sel_registerName(c"setActivationPolicy:".as_ptr()),
                policy,
            );
            if visible {
                set_macos_dock_icon();
                msg1_bool(
                    ns_app,
                    sel_registerName(c"activateIgnoringOtherApps:".as_ptr()),
                    1,
                );
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Disable macOS Window Restoration ("Reopen windows when logging back in") for this app
/// so macOS loginwindow does not launch a second copy alongside our LaunchAgent on reboot.
pub unsafe fn disable_macos_window_restoration() {
    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
    type MsgSetBool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool, *mut c_void);

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_set_bool: MsgSetBool = std::mem::transmute(objc_msgSend as *const c_void);

    let defaults_cls = objc_getClass(c"NSUserDefaults".as_ptr());
    let ns_str_cls = objc_getClass(c"NSString".as_ptr());
    if !defaults_cls.is_null() && !ns_str_cls.is_null() {
        let std_defaults = msg0(
            defaults_cls,
            sel_registerName(c"standardUserDefaults".as_ptr()),
        );
        let key = msg1_str(
            ns_str_cls,
            sel_registerName(c"stringWithUTF8String:".as_ptr()),
            c"NSQuitAlwaysKeepsWindows".as_ptr(),
        );
        if !std_defaults.is_null() && !key.is_null() {
            msg_set_bool(
                std_defaults,
                sel_registerName(c"setBool:forKey:".as_ptr()),
                false,
                key,
            );
        }
    }
    objc_autoreleasePoolPop(pool);
}

pub unsafe fn set_nswindow_non_restorable(window: &tao::window::Window) {
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        msg1_bool(
            ns_win,
            sel_registerName(c"setRestorable:".as_ptr()),
            false,
        );
    }
}

/// Make a borderless popover/overlay NSWindow truly transparent around its rounded corners
/// by clearing the NSWindow background color and masking the contentView's CALayer to `corner_radius`.
pub unsafe fn set_nswindow_rounded_transparent(window: &tao::window::Window, corner_radius: f64) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
        type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
        type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);

        msg1_bool(
            ns_win,
            sel_registerName(c"setRestorable:".as_ptr()),
            false,
        );
        msg1_bool(ns_win, sel_registerName(c"setOpaque:".as_ptr()), false);
        // NSWindowAnimationBehaviorNone = 2 (eliminate macOS window fade/pop transitions)
        msg1_isize(
            ns_win,
            sel_registerName(c"setAnimationBehavior:".as_ptr()),
            2,
        );
        if corner_radius <= 0.0 {
            msg1_bool(ns_win, sel_registerName(c"setHasShadow:".as_ptr()), false);
        }

        let ns_color_cls = objc_getClass(c"NSColor".as_ptr());
        if !ns_color_cls.is_null() {
            let clear_color = msg0(ns_color_cls, sel_registerName(c"clearColor".as_ptr()));
            if !clear_color.is_null() {
                msg1_ptr(
                    ns_win,
                    sel_registerName(c"setBackgroundColor:".as_ptr()),
                    clear_color,
                );
            }
        }

        let content_view = msg0(ns_win, sel_registerName(c"contentView".as_ptr()));
        if !content_view.is_null() {
            msg1_bool(
                content_view,
                sel_registerName(c"setWantsLayer:".as_ptr()),
                true,
            );
            let layer = msg0(content_view, sel_registerName(c"layer".as_ptr()));
            if !layer.is_null() {
                msg1_f64(
                    layer,
                    sel_registerName(c"setCornerRadius:".as_ptr()),
                    corner_radius,
                );
                msg1_bool(
                    layer,
                    sel_registerName(c"setMasksToBounds:".as_ptr()),
                    true,
                );
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Safely hide an overlay window, zero its opacity, lower its level, and make it ignore all mouse events
/// so a hidden overlay window can NEVER intercept clicks, flash stale frames, or freeze the screen,
/// while keeping its Retina backing scale factor intact.
pub unsafe fn park_overlay_window_offscreen(window: &tao::window::Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
        type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
        type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);

        msg1_isize(
            ns_win,
            sel_registerName(c"setAnimationBehavior:".as_ptr()),
            2,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setRestorable:".as_ptr()),
            false,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            true,
        );
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 0.0);
        // NSNormalWindowLevel = 0
        msg1_isize(ns_win, sel_registerName(c"setLevel:".as_ptr()), 0);
        msg1_ptr(
            ns_win,
            sel_registerName(c"orderOut:".as_ptr()),
            std::ptr::null_mut(),
        );
    }
    objc_autoreleasePoolPop(pool);
    window.set_visible(false);
}

/// Position a borderless fullscreen overlay `NSWindow` directly onto the target display's exact
/// Cocoa frame (`setFrame:display:animate:`) so Retina backing scale factor (`2.0`) is preserved
/// without any cross-monitor `tao` logical-size scaling mismatch.
pub unsafe fn place_overlay_on_display(
    window: &tao::window::Window,
    disp_x: i32,
    disp_y: i32,
    disp_w: usize,
    disp_h: usize,
) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgRect = unsafe extern "C" fn(*mut c_void, *mut c_void) -> CocoaRect;
        type MsgSetFrame =
            unsafe extern "C" fn(*mut c_void, *mut c_void, CocoaRect, bool, bool);
        type MsgCount = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type MsgAtIndex =
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_rect: MsgRect = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_set_frame: MsgSetFrame = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_count: MsgCount = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_at_idx: MsgAtIndex = std::mem::transmute(objc_msgSend as *const c_void);

        let mut primary_h = disp_h as f64;
        let ns_screen_cls = objc_getClass(c"NSScreen".as_ptr());
        if !ns_screen_cls.is_null() {
            let screens = msg0(ns_screen_cls, sel_registerName(c"screens".as_ptr()));
            if !screens.is_null() {
                let count = msg_count(screens, sel_registerName(c"count".as_ptr()));
                if count > 0 {
                    // Index 0 in [NSScreen screens] is the primary display containing origin (0,0)
                    let primary = msg_at_idx(
                        screens,
                        sel_registerName(c"objectAtIndex:".as_ptr()),
                        0,
                    );
                    if !primary.is_null() {
                        let pf = msg_rect(primary, sel_registerName(c"frame".as_ptr()));
                        if pf.size.height > 0.0 {
                            primary_h = pf.size.height;
                        }
                    }
                }
            }
        }

        // Convert CoreGraphics top-left coordinates to Cocoa bottom-left coordinates
        let cocoa_x = disp_x as f64;
        let cocoa_y = primary_h - (disp_y as f64 + disp_h as f64);
        let rect = CocoaRect {
            origin: CocoaPoint {
                x: cocoa_x,
                y: cocoa_y,
            },
            size: CocoaSize {
                width: disp_w as f64,
                height: disp_h as f64,
            },
        };

        msg_set_frame(
            ns_win,
            sel_registerName(c"setFrame:display:animate:".as_ptr()),
            rect,
            true,
            false,
        );
    }
    objc_autoreleasePoolPop(pool);
}

/// Prepare a screenshot overlay window on the target display while keeping it visually transparent (`alpha = 0.004`)
/// and mouse-passthrough (`ignoresMouseEvents = true`) so its `WKWebView` can decode and composite the new
/// screenshot onto its GPU `CAMetalLayer` before the window becomes visible (`alpha = 1.0`).
pub unsafe fn prepare_screenshot_window_hidden(
    window: &tao::window::Window,
    disp_x: i32,
    disp_y: i32,
    disp_w: usize,
    disp_h: usize,
) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
        type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);

        msg1_bool(
            ns_win,
            sel_registerName(c"setRestorable:".as_ptr()),
            false,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setHasShadow:".as_ptr()),
            false,
        );
        // NSWindowAnimationBehaviorNone = 2
        msg1_isize(
            ns_win,
            sel_registerName(c"setAnimationBehavior:".as_ptr()),
            2,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            true,
        );
        // 0.004 < 1/255 is completely invisible on screen, while keeping WindowServer occlusion
        // state visible so WKWebView renders requestAnimationFrame + CAMetalLayer immediately.
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 0.004);
        // NSStatusWindowLevel = 25 (above Menu Bar & Dock)
        msg1_isize(ns_win, sel_registerName(c"setLevel:".as_ptr()), 25);

        place_overlay_on_display(window, disp_x, disp_y, disp_w, disp_h);
        let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));
    }
    objc_autoreleasePoolPop(pool);
}

/// Atomically reveal an already-positioned and GPU-composited screenshot overlay window (`alpha = 1.0`).
pub unsafe fn reveal_screenshot_window(window: &tao::window::Window, make_key: bool) {
    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
    type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
    type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);

    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        msg1_isize(
            ns_win,
            sel_registerName(c"setAnimationBehavior:".as_ptr()),
            2,
        );
        msg1_isize(ns_win, sel_registerName(c"setLevel:".as_ptr()), 25);
        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            false,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setAcceptsMouseMovedEvents:".as_ptr()),
            true,
        );
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 1.0);

        if make_key {
            let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
            if !ns_app_cls.is_null() {
                let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
                if !ns_app.is_null() {
                    msg1_bool(
                        ns_app,
                        sel_registerName(c"activateIgnoringOtherApps:".as_ptr()),
                        true,
                    );
                }
            }
            msg1_ptr(
                ns_win,
                sel_registerName(c"makeKeyAndOrderFront:".as_ptr()),
                std::ptr::null_mut(),
            );
        } else {
            let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// When closing the screenshot overlay, if another application was frontmost before the screenshot began,
/// keep `main_window` ordered behind and restore active focus to that previous application so `main_window`
/// never unexpectedly pops in front of the user's workspace.
pub unsafe fn restore_previous_frontmost_app(
    main_window: &tao::window::Window,
    prev_pid: i32,
) {
    let my_pid = std::process::id() as i32;
    if prev_pid <= 0 || prev_pid == my_pid {
        return;
    }
    let pool = objc_autoreleasePoolPush();
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    type MsgPid = unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> *mut c_void;
    type MsgAct = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> bool;

    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_pid: MsgPid = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_act: MsgAct = std::mem::transmute(objc_msgSend as *const c_void);

    if main_window.is_visible() {
        let main_ns = main_window.ns_window();
        if !main_ns.is_null() {
            msg1_ptr(
                main_ns,
                sel_registerName(c"orderBack:".as_ptr()),
                std::ptr::null_mut(),
            );
        }
    }

    let cls = objc_getClass(c"NSRunningApplication".as_ptr());
    if !cls.is_null() {
        let app = msg_pid(
            cls,
            sel_registerName(c"runningApplicationWithProcessIdentifier:".as_ptr()),
            prev_pid,
        );
        if !app.is_null() {
            // NSApplicationActivateIgnoringOtherApps = 1 << 1 (2)
            let _ = msg_act(app, sel_registerName(c"activateWithOptions:".as_ptr()), 2);
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Activate the macOS application and optionally raise an overlay NSWindow above the Menu Bar & Dock
pub unsafe fn focus_and_elevate_macos_window(
    window: &tao::window::Window,
    above_menubar: bool,
    make_key: bool,
) {
    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
    type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
    type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);

    let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
    if !ns_app_cls.is_null() {
        let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
        if !ns_app.is_null() {
            msg1_bool(
                ns_app,
                sel_registerName(c"activateIgnoringOtherApps:".as_ptr()),
                true,
            );
        }
    }

    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        msg1_bool(
            ns_win,
            sel_registerName(c"setRestorable:".as_ptr()),
            false,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            false,
        );
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 1.0);
        if above_menubar {
            // NSStatusWindowLevel = 25 (covers Menu Bar & Dock for fullscreen screenshot selection)
            msg1_isize(ns_win, sel_registerName(c"setLevel:".as_ptr()), 25);
        }
        msg1_bool(
            ns_win,
            sel_registerName(c"setAcceptsMouseMovedEvents:".as_ptr()),
            true,
        );
        if make_key {
            msg1_ptr(
                ns_win,
                sel_registerName(c"makeKeyAndOrderFront:".as_ptr()),
                std::ptr::null_mut(),
            );
        } else {
            let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));
        }
    }

    objc_autoreleasePoolPop(pool);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CocoaPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CocoaSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CocoaRect {
    origin: CocoaPoint,
    size: CocoaSize,
}

/// Directly snap a `tao::window::Window` using Cocoa `[NSWindow setFrame:display:animate:]`
/// so snapping works reliably even when our own app window is frontmost (or before Accessibility is granted).
pub unsafe fn snap_nswindow_directly(window: &tao::window::Window, layout: &str) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgRect = unsafe extern "C" fn(*mut c_void, *mut c_void) -> CocoaRect;
        type MsgSetFrame =
            unsafe extern "C" fn(*mut c_void, *mut c_void, CocoaRect, bool, bool);
        type MsgCount = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type MsgAtIndex =
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_rect: MsgRect = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_set_frame: MsgSetFrame = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_count: MsgCount = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_at_idx: MsgAtIndex = std::mem::transmute(objc_msgSend as *const c_void);

        let mut screen = msg0(ns_win, sel_registerName(c"screen".as_ptr()));
        let ns_screen_cls = objc_getClass(c"NSScreen".as_ptr());
        if screen.is_null() && !ns_screen_cls.is_null() {
            screen = msg0(ns_screen_cls, sel_registerName(c"mainScreen".as_ptr()));
        }
        if !screen.is_null() {
            if layout == "next_display" && !ns_screen_cls.is_null() {
                let screens = msg0(ns_screen_cls, sel_registerName(c"screens".as_ptr()));
                if !screens.is_null() {
                    let count = msg_count(screens, sel_registerName(c"count".as_ptr()));
                    if count > 1 {
                        let mut cur_idx = 0usize;
                        for idx in 0..count {
                            let s = msg_at_idx(
                                screens,
                                sel_registerName(c"objectAtIndex:".as_ptr()),
                                idx,
                            );
                            if s == screen {
                                cur_idx = idx;
                                break;
                            }
                        }
                        let next_screen = msg_at_idx(
                            screens,
                            sel_registerName(c"objectAtIndex:".as_ptr()),
                            (cur_idx + 1) % count,
                        );
                        if !next_screen.is_null() {
                            screen = next_screen;
                        }
                    }
                }
            }

            let vf = msg_rect(screen, sel_registerName(c"visibleFrame".as_ptr()));
            let sx = vf.origin.x;
            let sy = vf.origin.y;
            let sw = vf.size.width;
            let sh = vf.size.height;

            // Note: Cocoa NSWindow coordinates have origin (0,0) at the BOTTOM-LEFT of the primary screen!
            let target = match layout {
                "left_half" => CocoaRect {
                    origin: CocoaPoint { x: sx, y: sy },
                    size: CocoaSize {
                        width: sw / 2.0,
                        height: sh,
                    },
                },
                "right_half" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx + sw / 2.0,
                        y: sy,
                    },
                    size: CocoaSize {
                        width: sw / 2.0,
                        height: sh,
                    },
                },
                "top_half" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx,
                        y: sy + sh / 2.0,
                    },
                    size: CocoaSize {
                        width: sw,
                        height: sh / 2.0,
                    },
                },
                "bottom_half" => CocoaRect {
                    origin: CocoaPoint { x: sx, y: sy },
                    size: CocoaSize {
                        width: sw,
                        height: sh / 2.0,
                    },
                },
                "top_left" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx,
                        y: sy + sh / 2.0,
                    },
                    size: CocoaSize {
                        width: sw / 2.0,
                        height: sh / 2.0,
                    },
                },
                "top_right" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx + sw / 2.0,
                        y: sy + sh / 2.0,
                    },
                    size: CocoaSize {
                        width: sw / 2.0,
                        height: sh / 2.0,
                    },
                },
                "bottom_left" => CocoaRect {
                    origin: CocoaPoint { x: sx, y: sy },
                    size: CocoaSize {
                        width: sw / 2.0,
                        height: sh / 2.0,
                    },
                },
                "bottom_right" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx + sw / 2.0,
                        y: sy,
                    },
                    size: CocoaSize {
                        width: sw / 2.0,
                        height: sh / 2.0,
                    },
                },
                "first_third" => CocoaRect {
                    origin: CocoaPoint { x: sx, y: sy },
                    size: CocoaSize {
                        width: sw / 3.0,
                        height: sh,
                    },
                },
                "center_third" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx + sw / 3.0,
                        y: sy,
                    },
                    size: CocoaSize {
                        width: sw / 3.0,
                        height: sh,
                    },
                },
                "last_third" | "one_third_right" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx + (sw * 2.0 / 3.0),
                        y: sy,
                    },
                    size: CocoaSize {
                        width: sw / 3.0,
                        height: sh,
                    },
                },
                "first_two_thirds" | "two_thirds_left" => CocoaRect {
                    origin: CocoaPoint { x: sx, y: sy },
                    size: CocoaSize {
                        width: sw * 2.0 / 3.0,
                        height: sh,
                    },
                },
                "last_two_thirds" => CocoaRect {
                    origin: CocoaPoint {
                        x: sx + sw / 3.0,
                        y: sy,
                    },
                    size: CocoaSize {
                        width: sw * 2.0 / 3.0,
                        height: sh,
                    },
                },
                "center" => {
                    let w = (sw * 0.75).round();
                    CocoaRect {
                        origin: CocoaPoint {
                            x: sx + ((sw - w) / 2.0).round(),
                            y: sy,
                        },
                        size: CocoaSize {
                            width: w,
                            height: sh,
                        },
                    }
                }
                "maximize" => CocoaRect {
                    origin: CocoaPoint { x: sx, y: sy },
                    size: CocoaSize {
                        width: sw,
                        height: sh,
                    },
                },
                _ => {
                    let w = (sw * 0.75).round();
                    CocoaRect {
                        origin: CocoaPoint {
                            x: sx + ((sw - w) / 2.0).round(),
                            y: sy,
                        },
                        size: CocoaSize {
                            width: w,
                            height: sh,
                        },
                    }
                }
            };

            msg_set_frame(
                ns_win,
                sel_registerName(c"setFrame:display:animate:".as_ptr()),
                target,
                true,
                true,
            );
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Position the main launcher window at **Center 75% width, 100% full usable vertical height**
/// (`visibleFrame`, from right below the macOS Menu Bar down to right against the Dock).
pub unsafe fn place_main_window_center_75_full_height(window: &tao::window::Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = window.ns_window();
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgRect = unsafe extern "C" fn(*mut c_void, *mut c_void) -> CocoaRect;
        type MsgSetFrame =
            unsafe extern "C" fn(*mut c_void, *mut c_void, CocoaRect, bool, bool);

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_rect: MsgRect = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_set_frame: MsgSetFrame = std::mem::transmute(objc_msgSend as *const c_void);

        let ns_screen_cls = objc_getClass(c"NSScreen".as_ptr());
        let mut screen = msg0(ns_win, sel_registerName(c"screen".as_ptr()));
        if screen.is_null() && !ns_screen_cls.is_null() {
            screen = msg0(ns_screen_cls, sel_registerName(c"mainScreen".as_ptr()));
        }
        if !screen.is_null() {
            let vf = msg_rect(screen, sel_registerName(c"visibleFrame".as_ptr()));
            let sx = vf.origin.x;
            let sy = vf.origin.y; // Exact top edge of the macOS Dock
            let sw = vf.size.width;
            let sh = vf.size.height; // Full vertical height between Menu Bar and Dock
            let w = (sw * 0.75).round();
            let target = CocoaRect {
                origin: CocoaPoint {
                    x: sx + ((sw - w) / 2.0).round(),
                    y: sy,
                },
                size: CocoaSize {
                    width: w,
                    height: sh,
                },
            };
            msg_set_frame(
                ns_win,
                sel_registerName(c"setFrame:display:animate:".as_ptr()),
                target,
                true,
                false,
            );
        }
    }
    objc_autoreleasePoolPop(pool);
}

