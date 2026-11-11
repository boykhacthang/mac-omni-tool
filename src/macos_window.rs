use egui_winit::winit::window::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_char, c_void};

pub const APP_ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Extract the underlying `NSWindow *` from a `winit::window::Window` via its `AppKit` `NSView` handle.
pub unsafe fn get_ns_window(window: &Window) -> *mut c_void {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
            let ns_view = appkit.ns_view.as_ptr() as *mut c_void;
            if !ns_view.is_null() {
                type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
                let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
                return msg0(ns_view, sel_registerName(c"window".as_ptr()));
            }
        }
    }
    std::ptr::null_mut()
}

/// NSWindowCollectionBehaviorCanJoinAllSpaces (1 << 0)
/// | NSWindowCollectionBehaviorStationary (1 << 4)
/// | NSWindowCollectionBehaviorIgnoresCycle (1 << 6)
/// | NSWindowCollectionBehaviorFullScreenAuxiliary (1 << 8)
/// | NSWindowCollectionBehaviorFullScreenDisallowsTiling (1 << 12)
/// | NSWindowCollectionBehaviorCanJoinAllApplications (1 << 18)
const OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR: isize =
    (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8) | (1 << 12) | (1 << 18);

/// Window level 2000 (above NSScreenSaverWindowLevel = 1000, Full Screen Spaces, Menu Bar, and Dock)
const OVERLAY_SCREENSHOT_WINDOW_LEVEL: isize = 2000;

/// NSPopUpMenuWindowLevel = 101 (renders floating popovers/drawers above Full Screen Spaces)
const OVERLAY_POPUP_WINDOW_LEVEL: isize = 101;

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub fn object_getClass(obj: *mut c_void) -> *mut c_void;
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
    pub fn class_replaceMethod(
        cls: *mut c_void,
        name: *mut c_void,
        imp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> bool,
        types: *const c_char,
    ) -> *mut c_void;
    pub fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

unsafe fn is_borderless_overlay_nswindow(this: *mut c_void) -> bool {
    if this.is_null() {
        return false;
    }
    type MsgStyleMask = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
    let msg_style: MsgStyleMask = std::mem::transmute(objc_msgSend as *const c_void);
    let mask = msg_style(this, sel_registerName(c"styleMask".as_ptr()));
    // NSWindowStyleMaskTitled = 1 << 0.
    // `main_window` is titled (mask & 1 != 0); all overlay windows are borderless (mask & 1 == 0).
    (mask & 1) == 0
}

unsafe extern "C" fn tao_is_nonactivating_panel_imp(this: *mut c_void, _sel: *mut c_void) -> bool {
    is_borderless_overlay_nswindow(this)
}

unsafe extern "C" fn tao_can_become_main_window_imp(this: *mut c_void, _sel: *mut c_void) -> bool {
    !is_borderless_overlay_nswindow(this)
}

/// Configure `TaoWindow` (`NSWindow`) so borderless overlay windows return `YES`
/// for `_isNonactivatingPanel`, `isFloatingPanel`, and `worksWhenModal`, and `NO` for `canBecomeMainWindow`,
/// without changing `TaoWindow`'s class hierarchy (`superclass` stays `NSWindow`).
///
/// In macOS AppKit (`-[NSWindow _allowedInOtherAppsFullScreenSpaceWithCollectionBehavior:]`),
/// windows belonging to a non-background-only app are forbidden from joining another app's
/// Full Screen Space (`_NSSetWindowAllowedOnSpecialSpaces` = 0) unless `[window _isNonactivatingPanel]`
/// returns `YES` and `FullScreenAuxiliary | CanJoinAllApplications` is set.
unsafe fn promote_to_nonactivating_overlay_panel(ns_win: *mut c_void) {
    if ns_win.is_null() {
        return;
    }
    let cur_cls = object_getClass(ns_win);
    if cur_cls.is_null() {
        return;
    }

    static SWIZZLED: std::sync::Once = std::sync::Once::new();
    SWIZZLED.call_once(|| {
        type ReplaceBoolNoArg = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool,
            *const c_char,
        ) -> *mut c_void;
        let replace_bool: ReplaceBoolNoArg =
            std::mem::transmute(class_replaceMethod as *const c_void);

        let _ = replace_bool(
            cur_cls,
            sel_registerName(c"_isNonactivatingPanel".as_ptr()),
            tao_is_nonactivating_panel_imp,
            c"B@:".as_ptr(),
        );
        let _ = replace_bool(
            cur_cls,
            sel_registerName(c"isFloatingPanel".as_ptr()),
            tao_is_nonactivating_panel_imp,
            c"B@:".as_ptr(),
        );
        let _ = replace_bool(
            cur_cls,
            sel_registerName(c"worksWhenModal".as_ptr()),
            tao_is_nonactivating_panel_imp,
            c"B@:".as_ptr(),
        );
        let _ = replace_bool(
            cur_cls,
            sel_registerName(c"canBecomeMainWindow".as_ptr()),
            tao_can_become_main_window_imp,
            c"B@:".as_ptr(),
        );
    });

    type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
    type MsgGetIsize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
    type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
    let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_get_isize: MsgGetIsize = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
    let sel_coll = sel_registerName(c"setCollectionBehavior:".as_ptr());
    let sel_get_coll = sel_registerName(c"collectionBehavior".as_ptr());

    msg1_bool(
        ns_win,
        sel_registerName(c"setCanBecomeVisibleWithoutLogin:".as_ptr()),
        true,
    );
    if msg_get_isize(ns_win, sel_get_coll) != OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR {
        msg1_isize(ns_win, sel_coll, 0);
        msg1_isize(ns_win, sel_coll, OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR);
    }
}

unsafe extern "C" fn accepts_first_mouse_imp(
    _this: *mut c_void,
    _sel: *mut c_void,
    _event: *mut c_void,
) -> bool {
    true
}

/// Enable `acceptsFirstMouse:` on the window's `contentView` and all its `WKWebView` subviews
/// so clicking on a secondary monitor's overlay window selects or drags on the 1st click instead of requiring 2 clicks.
pub unsafe fn enable_overlay_accepts_first_mouse(window: &Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgCount = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type MsgAtIdx = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_count: MsgCount = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_at_idx: MsgAtIdx = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_accepts = sel_registerName(c"acceptsFirstMouse:".as_ptr());
        let content_view = msg0(ns_win, sel_registerName(c"contentView".as_ptr()));
        if !content_view.is_null() {
            let cv_cls = object_getClass(content_view);
            if !cv_cls.is_null() {
                let _ = class_replaceMethod(
                    cv_cls,
                    sel_accepts,
                    accepts_first_mouse_imp,
                    c"B@:@".as_ptr(),
                );
            }
            let subviews = msg0(content_view, sel_registerName(c"subviews".as_ptr()));
            if !subviews.is_null() {
                let count = msg_count(subviews, sel_registerName(c"count".as_ptr()));
                for i in 0..count {
                    let sub = msg_at_idx(
                        subviews,
                        sel_registerName(c"objectAtIndex:".as_ptr()),
                        i,
                    );
                    if !sub.is_null() {
                        let sub_cls = object_getClass(sub);
                        if !sub_cls.is_null() {
                            let _ = class_replaceMethod(
                                sub_cls,
                                sel_accepts,
                                accepts_first_mouse_imp,
                                c"B@:@".as_ptr(),
                            );
                        }
                    }
                }
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Apply the embedded minimalist AppIcon directly to the macOS Dock & Cmd+Tab switcher
/// (only needed when running outside a `.app` bundle, as `.app` bundles use `Contents/Resources/AppIcon.icns` out-of-process).
pub unsafe fn set_macos_dock_icon() {
    static DOCK_ICON_ONCE: std::sync::Once = std::sync::Once::new();
    DOCK_ICON_ONCE.call_once(|| {
        if let Ok(exe) = std::env::current_exe() {
            if exe.to_string_lossy().contains(".app/Contents/MacOS/") {
                return;
            }
        }
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
                    let _ = msg0(img, sel_registerName(c"release".as_ptr()));
                }
            }
        }
        objc_autoreleasePoolPop(pool);
    });
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

pub unsafe fn set_nswindow_non_restorable(window: &Window) {
    let ns_win = get_ns_window(window);
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
pub unsafe fn set_nswindow_rounded_transparent(window: &Window, corner_radius: f64) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
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

        promote_to_nonactivating_overlay_panel(ns_win);
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
                type Msg0F64 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64;
                let msg0_f64: Msg0F64 = std::mem::transmute(objc_msgSend as *const c_void);
                let scale = msg0_f64(ns_win, sel_registerName(c"backingScaleFactor".as_ptr()));
                if scale > 0.5 {
                    msg1_f64(
                        layer,
                        sel_registerName(c"setContentsScale:".as_ptr()),
                        scale,
                    );
                }
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

/// Safely hide an overlay window, zero its opacity, lower its level,
/// and make it ignore all mouse events so a hidden overlay window can NEVER intercept clicks, flash stale frames,
/// or freeze the screen on any Space, while preserving its display frame so `WKWebView` never has to asynchronously
/// resize its viewport when revealing the next screenshot.
pub unsafe fn park_overlay_window_offscreen(window: &Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
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
        promote_to_nonactivating_overlay_panel(ns_win);
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

/// Safely hide a floating popup/drawer window (`mini_window`, `clip_window`, `alttab_window`)
/// and make it ignore all mouse events so it can never block clicks while hidden.
pub unsafe fn hide_floating_overlay_window(window: &Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
        type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);

        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            true,
        );
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 0.0);
        msg1_ptr(
            ns_win,
            sel_registerName(c"orderOut:".as_ptr()),
            std::ptr::null_mut(),
        );
    }
    objc_autoreleasePoolPop(pool);
    window.set_visible(false);
}

/// Enable Apple Silicon lossless GPU `IOSurface` compression on the window's `CAMetalLayer`
/// and ensure its `frame` and `contentsScale` match `contentView.bounds`.
pub unsafe fn optimize_nswindow_metal_layer(window: &Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        type Msg1BoolRet = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> bool;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_bool_ret: Msg1BoolRet = std::mem::transmute(objc_msgSend as *const c_void);

        let content_view = msg0(ns_win, sel_registerName(c"contentView".as_ptr()));
        if !content_view.is_null() {
            let layer = msg0(content_view, sel_registerName(c"layer".as_ptr()));
            let metal_cls = objc_getClass(c"CAMetalLayer".as_ptr());
            if !layer.is_null()
                && !metal_cls.is_null()
                && msg1_bool_ret(
                    layer,
                    sel_registerName(c"isKindOfClass:".as_ptr()),
                    metal_cls,
                )
            {
                let sel_comp = sel_registerName(c"setWantsIOSurfaceCompression:".as_ptr());
                if msg1_bool_ret(
                    layer,
                    sel_registerName(c"respondsToSelector:".as_ptr()),
                    sel_comp,
                ) {
                    msg1_bool(layer, sel_comp, true);
                }
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Purge all `IOSurface` backbuffers and cached drawable contents from `window.contentView`'s
/// `CAMetalLayer` in-place when the window is hidden to the background.
/// Keeps the `CAMetalLayer` instance attached to `contentView` so `wgpu` reuses it cleanly on reopen
/// without leaking layers or throwing Objective-C selector exceptions on non-Metal backing layers.
pub unsafe fn release_nswindow_metal_layer(window: &Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct ObjcSize {
            width: f64,
            height: f64,
        }
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg0Void = unsafe extern "C" fn(*mut c_void, *mut c_void);
        type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
        type Msg1Size = unsafe extern "C" fn(*mut c_void, *mut c_void, ObjcSize);
        type Msg1BoolRet = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> bool;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_void: Msg0Void = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_size: Msg1Size = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_bool_ret: Msg1BoolRet = std::mem::transmute(objc_msgSend as *const c_void);

        let content_view = msg0(ns_win, sel_registerName(c"contentView".as_ptr()));
        if !content_view.is_null() {
            let layer = msg0(content_view, sel_registerName(c"layer".as_ptr()));
            let metal_cls = objc_getClass(c"CAMetalLayer".as_ptr());
            if !layer.is_null()
                && !metal_cls.is_null()
                && msg1_bool_ret(
                    layer,
                    sel_registerName(c"isKindOfClass:".as_ptr()),
                    metal_cls,
                )
            {
                msg1_size(
                    layer,
                    sel_registerName(c"setDrawableSize:".as_ptr()),
                    ObjcSize {
                        width: 1.0,
                        height: 1.0,
                    },
                );
                let sel_next = sel_registerName(c"nextDrawable".as_ptr());
                if msg1_bool_ret(
                    layer,
                    sel_registerName(c"respondsToSelector:".as_ptr()),
                    sel_next,
                ) {
                    let tiny_drawable = msg0(layer, sel_next);
                    if !tiny_drawable.is_null() {
                        msg0_void(tiny_drawable, sel_registerName(c"present".as_ptr()));
                    }
                }
                msg1_ptr(
                    layer,
                    sel_registerName(c"setContents:".as_ptr()),
                    std::ptr::null_mut(),
                );
                let sel_remove_bb = sel_registerName(c"removeBackBuffers".as_ptr());
                if msg1_bool_ret(
                    layer,
                    sel_registerName(c"respondsToSelector:".as_ptr()),
                    sel_remove_bb,
                ) {
                    msg0_void(layer, sel_remove_bb);
                }
                let sel_discard = sel_registerName(c"discardContents".as_ptr());
                if msg1_bool_ret(
                    layer,
                    sel_registerName(c"respondsToSelector:".as_ptr()),
                    sel_discard,
                ) {
                    msg0_void(layer, sel_discard);
                }
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Show a floating popup/drawer window (`mini_window`, `clip_window`, `alttab_window`)
/// and restore its opacity and mouse event handling.
pub unsafe fn show_floating_overlay_window(window: &Window, corner_radius: f64) {
    set_nswindow_rounded_transparent(window, corner_radius);
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
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
                let _ = msg0(ns_app, sel_registerName(c"unhideWithoutActivation".as_ptr()));
                msg1_bool(
                    ns_app,
                    sel_registerName(c"activateIgnoringOtherApps:".as_ptr()),
                    true,
                );
            }
        }

        promote_to_nonactivating_overlay_panel(ns_win);
        msg1_bool(
            ns_win,
            sel_registerName(c"setHidesOnDeactivate:".as_ptr()),
            false,
        );
        msg1_isize(
            ns_win,
            sel_registerName(c"setCollectionBehavior:".as_ptr()),
            OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR,
        );
        msg1_isize(
            ns_win,
            sel_registerName(c"setLevel:".as_ptr()),
            OVERLAY_POPUP_WINDOW_LEVEL,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            false,
        );
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 1.0);
        let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));
        msg1_ptr(
            ns_win,
            sel_registerName(c"makeKeyAndOrderFront:".as_ptr()),
            std::ptr::null_mut(),
        );
    }
    objc_autoreleasePoolPop(pool);
    window.set_visible(true);
    window.focus_window();
}

/// Check if any of the given `Window`s is currently the active key window (`[ns_win isKeyWindow]`).
#[allow(dead_code)]
pub unsafe fn is_any_nswindow_focused(windows: &[Window]) -> bool {
    let pool = objc_autoreleasePoolPush();
    type MsgBool = unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool;
    let msg_bool: MsgBool = std::mem::transmute(objc_msgSend as *const c_void);
    let sel_is_key = sel_registerName(c"isKeyWindow".as_ptr());
    let mut any_key = false;
    for w in windows {
        let ns_win = get_ns_window(w);
        if !ns_win.is_null() && msg_bool(ns_win, sel_is_key) {
            any_key = true;
            break;
        }
    }
    objc_autoreleasePoolPop(pool);
    any_key
}

/// Position a borderless fullscreen overlay `NSWindow` directly onto the target display's exact
/// Cocoa frame (`setFrame:display:animate:`) so Retina backing scale factor (`2.0`) is preserved
/// without any cross-monitor logical-size scaling mismatch.
pub unsafe fn place_overlay_on_display(
    window: &Window,
    disp_x: i32,
    disp_y: i32,
    disp_w: usize,
    disp_h: usize,
) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
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

        let cur = msg_rect(ns_win, sel_registerName(c"frame".as_ptr()));
        let needs_resize = (cur.origin.x - rect.origin.x).abs() > 0.5
            || (cur.origin.y - rect.origin.y).abs() > 0.5
            || (cur.size.width - rect.size.width).abs() > 0.5
            || (cur.size.height - rect.size.height).abs() > 0.5;
        if needs_resize {
            msg_set_frame(
                ns_win,
                sel_registerName(c"setFrame:display:animate:".as_ptr()),
                rect,
                false,
                false,
            );
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Prepare a screenshot overlay window on the target display while keeping it visually transparent (`alpha = 0.004`)
/// and mouse-passthrough (`ignoresMouseEvents = true`) so its `WKWebView` can decode and composite the new
/// screenshot onto its GPU `CAMetalLayer` before the window becomes visible (`alpha = 1.0`).
pub unsafe fn prepare_screenshot_window_hidden(
    window: &Window,
    disp_x: i32,
    disp_y: i32,
    disp_w: usize,
    disp_h: usize,
) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Bool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
        type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
        type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_bool: Msg1Bool = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);

        promote_to_nonactivating_overlay_panel(ns_win);
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
        msg1_bool(
            ns_win,
            sel_registerName(c"setHidesOnDeactivate:".as_ptr()),
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
        // MoveToActiveSpace | Transient | FullScreenAuxiliary (266)
        msg1_isize(
            ns_win,
            sel_registerName(c"setCollectionBehavior:".as_ptr()),
            OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR,
        );
        // 0.004 < 1/255 is completely invisible on screen, while keeping WindowServer occlusion
        // state visible at target level so WKWebView renders requestAnimationFrame + CAMetalLayer immediately.
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 0.004);
        msg1_isize(
            ns_win,
            sel_registerName(c"setLevel:".as_ptr()),
            OVERLAY_SCREENSHOT_WINDOW_LEVEL,
        );

        place_overlay_on_display(window, disp_x, disp_y, disp_w, disp_h);
        enable_overlay_accepts_first_mouse(window);
        let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));
    }
    objc_autoreleasePoolPop(pool);
}

/// Atomically reveal an already-positioned and GPU-composited screenshot overlay window (`alpha = 1.0`).
pub unsafe fn reveal_screenshot_window(window: &Window, make_key: bool) {
    enable_overlay_accepts_first_mouse(window);
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

    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
        if !ns_app_cls.is_null() {
            let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
            if !ns_app.is_null() {
                let _ = msg0(ns_app, sel_registerName(c"unhideWithoutActivation".as_ptr()));
                if make_key {
                    msg1_bool(
                        ns_app,
                        sel_registerName(c"activateIgnoringOtherApps:".as_ptr()),
                        true,
                    );
                }
            }
        }

        promote_to_nonactivating_overlay_panel(ns_win);
        msg1_bool(
            ns_win,
            sel_registerName(c"setHidesOnDeactivate:".as_ptr()),
            false,
        );
        msg1_isize(
            ns_win,
            sel_registerName(c"setAnimationBehavior:".as_ptr()),
            2,
        );
        msg1_isize(
            ns_win,
            sel_registerName(c"setCollectionBehavior:".as_ptr()),
            OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR,
        );
        msg1_isize(
            ns_win,
            sel_registerName(c"setLevel:".as_ptr()),
            OVERLAY_SCREENSHOT_WINDOW_LEVEL,
        );
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
        let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));

        if make_key {
            msg1_ptr(
                ns_win,
                sel_registerName(c"makeKeyAndOrderFront:".as_ptr()),
                std::ptr::null_mut(),
            );
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// When closing the screenshot overlay, if another application was frontmost before the screenshot began,
/// keep `main_window` ordered behind and restore active focus to that previous application so `main_window`
/// never unexpectedly pops in front of the user's workspace.
pub unsafe fn restore_previous_frontmost_app(
    main_window: &Window,
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

    if main_window.is_visible().unwrap_or(false) {
        let main_ns = get_ns_window(main_window);
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
    window: &Window,
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

    let ns_win = get_ns_window(window);
    if !ns_win.is_null() {
        promote_to_nonactivating_overlay_panel(ns_win);
        msg1_bool(
            ns_win,
            sel_registerName(c"setRestorable:".as_ptr()),
            false,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setHidesOnDeactivate:".as_ptr()),
            false,
        );
        msg1_isize(
            ns_win,
            sel_registerName(c"setCollectionBehavior:".as_ptr()),
            OVERLAY_FULLSCREEN_COLLECTION_BEHAVIOR,
        );
        msg1_bool(
            ns_win,
            sel_registerName(c"setIgnoresMouseEvents:".as_ptr()),
            false,
        );
        msg1_f64(ns_win, sel_registerName(c"setAlphaValue:".as_ptr()), 1.0);
        let level = if above_menubar {
            OVERLAY_SCREENSHOT_WINDOW_LEVEL
        } else {
            OVERLAY_POPUP_WINDOW_LEVEL
        };
        msg1_isize(ns_win, sel_registerName(c"setLevel:".as_ptr()), level);
        msg1_bool(
            ns_win,
            sel_registerName(c"setAcceptsMouseMovedEvents:".as_ptr()),
            true,
        );
        let _ = msg0(ns_win, sel_registerName(c"orderFrontRegardless".as_ptr()));
        if make_key {
            msg1_ptr(
                ns_win,
                sel_registerName(c"makeKeyAndOrderFront:".as_ptr()),
                std::ptr::null_mut(),
            );
        }
    }

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
pub unsafe fn snap_nswindow_directly(window: &Window, layout: &str) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
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
pub unsafe fn place_main_window_center_75_full_height(window: &Window) {
    let pool = objc_autoreleasePoolPush();
    let ns_win = get_ns_window(window);
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

extern "C" {
    fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    fn CGColorSpaceRelease(space: *mut c_void);
    fn CGImageGetWidth(image: *mut c_void) -> usize;
    fn CGImageGetHeight(image: *mut c_void) -> usize;
    fn CGImageRelease(image: *mut c_void);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    fn CGBitmapContextCreateImage(context: *mut c_void) -> *mut c_void;
    fn CGContextDrawImage(c: *mut c_void, rect: CocoaRect, image: *mut c_void);
    fn CGContextRelease(c: *mut c_void);
}

/// kCGImageAlphaPremultipliedLast (1) | kCGBitmapByteOrder32Big (4 << 12) => RGBA8 byte order in memory
const CG_RGBA8_BITMAP_INFO: u32 = 1 | (4 << 12);

/// Hardware-accelerated PNG / JPEG / TIFF / ICNS decoding into raw `RGBA8` `(width, height, Vec<u8>)`
/// using macOS `NSBitmapImageRep` + `CoreGraphics`.
#[allow(dead_code)]
pub fn decode_image_bytes_to_rgba8(bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    decode_image_bytes_to_rgba8_max_size(bytes, 0)
}

/// Hardware-accelerated PNG / JPEG / TIFF / ICNS decoding into raw `RGBA8` `(width, height, Vec<u8>)`,
/// optionally downscaled in CoreGraphics so `width <= max_dim` and `height <= max_dim` when `max_dim > 0`.
pub fn decode_image_bytes_to_rgba8_max_size(
    bytes: &[u8],
    max_dim: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    if bytes.is_empty() {
        return None;
    }
    unsafe {
        let pool = objc_autoreleasePoolPush();
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Obj = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
        type MsgData =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8, usize) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_obj: Msg1Obj = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_data: MsgData = std::mem::transmute(objc_msgSend as *const c_void);

        let ns_data_cls = objc_getClass(c"NSData".as_ptr());
        let ns_rep_cls = objc_getClass(c"NSBitmapImageRep".as_ptr());
        if ns_data_cls.is_null() || ns_rep_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let data = msg_data(
            ns_data_cls,
            sel_registerName(c"dataWithBytes:length:".as_ptr()),
            bytes.as_ptr(),
            bytes.len(),
        );
        if data.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let rep = msg1_obj(
            ns_rep_cls,
            sel_registerName(c"imageRepWithData:".as_ptr()),
            data,
        );
        if rep.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let cg_img = msg0(rep, sel_registerName(c"CGImage".as_ptr()));
        if cg_img.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let src_w = CGImageGetWidth(cg_img);
        let src_h = CGImageGetHeight(cg_img);
        if src_w == 0 || src_h == 0 || src_w > 16384 || src_h > 16384 {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let (width, height) = if max_dim > 0 && (src_w > max_dim || src_h > max_dim) {
            let scale = (max_dim as f64 / src_w as f64).min(max_dim as f64 / src_h as f64);
            (
                ((src_w as f64 * scale).round() as usize).max(1),
                ((src_h as f64 * scale).round() as usize).max(1),
            )
        } else {
            (src_w, src_h)
        };

        let mut rgba = vec![0u8; width * height * 4];
        let color_space = CGColorSpaceCreateDeviceRGB();
        let ctx = CGBitmapContextCreate(
            rgba.as_mut_ptr() as *mut c_void,
            width,
            height,
            8,
            width * 4,
            color_space,
            CG_RGBA8_BITMAP_INFO,
        );
        if ctx.is_null() {
            if !color_space.is_null() {
                CGColorSpaceRelease(color_space);
            }
            objc_autoreleasePoolPop(pool);
            return None;
        }

        CGContextDrawImage(
            ctx,
            CocoaRect {
                origin: CocoaPoint { x: 0.0, y: 0.0 },
                size: CocoaSize {
                    width: width as f64,
                    height: height as f64,
                },
            },
            cg_img,
        );

        CGContextRelease(ctx);
        if !color_space.is_null() {
            CGColorSpaceRelease(color_space);
        }
        objc_autoreleasePoolPop(pool);
        Some((width, height, rgba))
    }
}

/// Decode a `data:image/...;base64,...` URL or a local file path into `(width, height, Vec<u8>)` RGBA8.
pub fn decode_data_url_or_path_to_rgba8(input: &str) -> Option<(usize, usize, Vec<u8>)> {
    decode_data_url_or_path_to_rgba8_max_size(input, 640)
}

/// Decode a `data:image/...;base64,...` URL or a local file path into `(width, height, Vec<u8>)` RGBA8
/// with optional downscaling (`max_dim`).
pub fn decode_data_url_or_path_to_rgba8_max_size(
    input: &str,
    max_dim: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("data:image/") {
        if let Ok(bytes) = core_clipboard::decode_base64_png(trimmed) {
            return decode_image_bytes_to_rgba8_max_size(&bytes, max_dim);
        }
        return None;
    }
    if let Ok(bytes) = std::fs::read(trimmed) {
        return decode_image_bytes_to_rgba8_max_size(&bytes, max_dim);
    }
    None
}

/// Hardware-accelerated encoding of raw `RGBA8` pixels `(width, height, &[u8])` into PNG bytes
/// using macOS `CoreGraphics` + `NSBitmapImageRep`.
pub fn encode_rgba8_to_png(width: usize, height: usize, rgba: &[u8]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() < width.saturating_mul(height).saturating_mul(4) {
        return None;
    }
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let color_space = CGColorSpaceCreateDeviceRGB();
        let ctx = CGBitmapContextCreate(
            rgba.as_ptr() as *mut c_void,
            width,
            height,
            8,
            width * 4,
            color_space,
            CG_RGBA8_BITMAP_INFO,
        );
        if ctx.is_null() {
            if !color_space.is_null() {
                CGColorSpaceRelease(color_space);
            }
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let cg_img = CGBitmapContextCreateImage(ctx);
        CGContextRelease(ctx);
        if !color_space.is_null() {
            CGColorSpaceRelease(color_space);
        }
        if cg_img.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Obj = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
        type MsgRepPng = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *mut c_void,
        ) -> *mut c_void;
        type MsgLen = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type MsgBytes = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const u8;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_obj: Msg1Obj = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_rep_png: MsgRepPng = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_len: MsgLen = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_bytes: MsgBytes = std::mem::transmute(objc_msgSend as *const c_void);

        let ns_rep_cls = objc_getClass(c"NSBitmapImageRep".as_ptr());
        let ns_dict_cls = objc_getClass(c"NSDictionary".as_ptr());
        let mut out_vec = None;
        if !ns_rep_cls.is_null() {
            let empty_dict = if !ns_dict_cls.is_null() {
                msg0(ns_dict_cls, sel_registerName(c"dictionary".as_ptr()))
            } else {
                std::ptr::null_mut()
            };
            let rep_alloc = msg0(ns_rep_cls, sel_registerName(c"alloc".as_ptr()));
            let rep = msg1_obj(
                rep_alloc,
                sel_registerName(c"initWithCGImage:".as_ptr()),
                cg_img,
            );
            if !rep.is_null() {
                // NSBitmapImageFileTypePNG = 4
                let png_data = msg_rep_png(
                    rep,
                    sel_registerName(c"representationUsingType:properties:".as_ptr()),
                    4,
                    empty_dict,
                );
                if !png_data.is_null() {
                    let len = msg_len(png_data, sel_registerName(c"length".as_ptr()));
                    let ptr = msg_bytes(png_data, sel_registerName(c"bytes".as_ptr()));
                    if len > 0 && !ptr.is_null() {
                        let slice = std::slice::from_raw_parts(ptr, len);
                        out_vec = Some(slice.to_vec());
                    }
                }
                let _ = msg0(rep, sel_registerName(c"release".as_ptr()));
            }
        }

        CGImageRelease(cg_img);
        objc_autoreleasePoolPop(pool);
        out_vec
    }
}

/// Release cached dashboard/cleaner heap allocations and advise macOS `libmalloc`
/// to return free magazine pages to the kernel when windows are hidden to the background.
pub fn release_process_heap_memory() {
    core_monitor::release_dashboard_memory();
    core_cleaner::release_cleaner_memory();
    unsafe {
        let _ = malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coregraphics_rgba8_png_roundtrip_and_app_icon() {
        let (icon_w, icon_h, icon_rgba) =
            decode_image_bytes_to_rgba8(APP_ICON_PNG).expect("Should decode embedded APP_ICON_PNG");
        assert!(icon_w > 0 && icon_h > 0);
        assert_eq!(icon_rgba.len(), icon_w * icon_h * 4);

        let w = 4usize;
        let h = 3usize;
        let mut raw = Vec::with_capacity(w * h * 4);
        for i in 0..(w * h) {
            raw.push(((i * 37) & 0xFF) as u8);
            raw.push(((i * 73) & 0xFF) as u8);
            raw.push(((i * 113) & 0xFF) as u8);
            raw.push(255);
        }
        let png = encode_rgba8_to_png(w, h, &raw).expect("Should encode RGBA8 to PNG");
        assert!(png.starts_with(b"\x89PNG"));
        let (dw, dh, decoded) =
            decode_image_bytes_to_rgba8(&png).expect("Should decode encoded PNG");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(decoded, raw);
    }
}


