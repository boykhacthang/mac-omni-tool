use crate::events::AppUserEvent;
use crate::macos_window::{
    class_addMethod, objc_allocateClassPair, objc_autoreleasePoolPop, objc_autoreleasePoolPush,
    objc_getClass, objc_msgSend, objc_registerClassPair, sel_registerName, set_macos_dock_icon,
};
use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use tao::event_loop::EventLoopProxy;

static MENUBAR_EVENT_PROXY: OnceLock<Mutex<Option<EventLoopProxy<AppUserEvent>>>> = OnceLock::new();
static STATUS_BUTTON_PTR: AtomicIsize = AtomicIsize::new(0);
static TARGET_OBJ_PTR: AtomicIsize = AtomicIsize::new(0);

unsafe extern "C" fn status_bar_item_clicked(
    _this: *mut c_void,
    _cmd: *mut c_void,
    _sender: *mut c_void,
) {
    if let Some(lock) = MENUBAR_EVENT_PROXY.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(AppUserEvent::ToggleMenuBarPopover);
            }
        }
    }
}

unsafe extern "C" fn about_menu_clicked(
    _this: *mut c_void,
    _cmd: *mut c_void,
    _sender: *mut c_void,
) {
    show_native_macos_about_panel();
    if let Some(lock) = MENUBAR_EVENT_PROXY.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(AppUserEvent::ShowAboutDialog);
            }
        }
    }
}

unsafe extern "C" fn settings_menu_clicked(
    _this: *mut c_void,
    _cmd: *mut c_void,
    _sender: *mut c_void,
) {
    if let Some(lock) = MENUBAR_EVENT_PROXY.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(AppUserEvent::OpenDashboardTab("settings".to_string()));
            }
        }
    }
}

unsafe extern "C" fn github_menu_clicked(
    _this: *mut c_void,
    _cmd: *mut c_void,
    _sender: *mut c_void,
) {
    let _ = std::process::Command::new("open")
        .arg("https://github.com/boykhacthang/mac-omni-tool")
        .spawn();
}

unsafe extern "C" fn quit_menu_clicked(
    _this: *mut c_void,
    _cmd: *mut c_void,
    _sender: *mut c_void,
) {
    if let Some(lock) = MENUBAR_EVENT_PROXY.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(AppUserEvent::QuitApp);
            }
        }
    }
}

unsafe fn make_nsstring(s: &str) -> *mut c_void {
    type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
    let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
    let ns_str_cls = objc_getClass(c"NSString".as_ptr());
    let sel_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
    if let Ok(c_str) = CString::new(s) {
        msg1_str(ns_str_cls, sel_utf8, c_str.as_ptr())
    } else {
        std::ptr::null_mut()
    }
}

/// Show the native macOS About panel (`orderFrontStandardAboutPanelWithOptions:`) with rich credits
pub unsafe fn show_native_macos_about_panel() {
    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    type Msg1Obj = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
    type MsgSetObj = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void);

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_obj: Msg1Obj = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_set_obj: MsgSetObj = std::mem::transmute(objc_msgSend as *const c_void);

    let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
    let dict_cls = objc_getClass(c"NSMutableDictionary".as_ptr());
    let attr_cls = objc_getClass(c"NSAttributedString".as_ptr());

    if !ns_app_cls.is_null() && !dict_cls.is_null() {
        let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
        let dict = msg0(dict_cls, sel_registerName(c"dictionary".as_ptr()));
        if !ns_app.is_null() && !dict.is_null() {
            let sel_set = sel_registerName(c"setObject:forKey:".as_ptr());

            let k_name = make_nsstring("ApplicationName");
            let v_name = make_nsstring("Mac Omni Tool");
            msg_set_obj(dict, sel_set, v_name, k_name);

            let k_ver = make_nsstring("ApplicationVersion");
            let v_ver = make_nsstring("2.0.0 — Free Community Edition");
            msg_set_obj(dict, sel_set, v_ver, k_ver);

            let k_build = make_nsstring("Version");
            let v_build = make_nsstring("MIT Open-Source");
            msg_set_obj(dict, sel_set, v_build, k_build);

            let k_copy = make_nsstring("Copyright");
            let v_copy = make_nsstring(
                "Copyright © 2026 boykhacthang\nhttps://github.com/boykhacthang/mac-omni-tool",
            );
            msg_set_obj(dict, sel_set, v_copy, k_copy);

            if !attr_cls.is_null() {
                let credits_str = make_nsstring(
                    "All-in-One Native macOS System Utility built in Rust.\n\
                     • Displays, Gamma Dimming, Warmth & Audio Switcher\n\
                     • Apple Silicon CPU/GPU/RAM Telemetry & Process Manager\n\
                     • 1,000-Item SQLite Clipboard Manager & Left Drawer\n\
                     • 13-Layout Window Tiling & Live Preview Alt-Tab\n\
                     • External Mouse Per-App Rules & 120Hz Smooth Scroll\n\
                     • 3-Mode Multi-Monitor Screenshot & Annotation\n\
                     • System Junk Cleaner, Duplicate Finder & Uninstaller\n\n\
                     Author: boykhacthang (github.com/boykhacthang/mac-omni-tool)\n\
                     Support: https://ko-fi.com/I1R72A1475",
                );
                let attr_alloc = msg0(attr_cls, sel_registerName(c"alloc".as_ptr()));
                let attr_obj = msg1_obj(
                    attr_alloc,
                    sel_registerName(c"initWithString:".as_ptr()),
                    credits_str,
                );
                if !attr_obj.is_null() {
                    let k_credits = make_nsstring("Credits");
                    msg_set_obj(dict, sel_set, attr_obj, k_credits);
                }
            }

            msg1_ptr(
                ns_app,
                sel_registerName(c"orderFrontStandardAboutPanelWithOptions:".as_ptr()),
                dict,
            );
        }
    }
    objc_autoreleasePoolPop(pool);
}

/// Wire custom "About Mac Omni Tool", "Settings...", and "GitHub Repository" items into the top-left
/// macOS Application Menu (next to the Apple  logo) so clicking the app name shows full About info.
pub unsafe fn setup_macos_application_menu() {
    let target_obj = TARGET_OBJ_PTR.load(Ordering::SeqCst) as *mut c_void;
    if target_obj.is_null() {
        return;
    }

    let pool = objc_autoreleasePoolPush();
    type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    type Msg1Isize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize);
    type Msg1IsizeRet = unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> *mut c_void;
    type MsgInsertItem =
        unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, isize);
    type MsgInitMenuItem = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
    ) -> *mut c_void;

    let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_isize: Msg1Isize = std::mem::transmute(objc_msgSend as *const c_void);
    let msg1_isize_ret: Msg1IsizeRet = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_insert: MsgInsertItem = std::mem::transmute(objc_msgSend as *const c_void);
    let msg_init_item: MsgInitMenuItem = std::mem::transmute(objc_msgSend as *const c_void);

    let ns_app_cls = objc_getClass(c"NSApplication".as_ptr());
    let ns_menu_cls = objc_getClass(c"NSMenu".as_ptr());
    let ns_item_cls = objc_getClass(c"NSMenuItem".as_ptr());

    if !ns_app_cls.is_null() && !ns_menu_cls.is_null() && !ns_item_cls.is_null() {
        let ns_app = msg0(ns_app_cls, sel_registerName(c"sharedApplication".as_ptr()));
        if !ns_app.is_null() {
            let mut main_menu = msg0(ns_app, sel_registerName(c"mainMenu".as_ptr()));
            if main_menu.is_null() {
                let m_alloc = msg0(ns_menu_cls, sel_registerName(c"alloc".as_ptr()));
                main_menu = msg0(m_alloc, sel_registerName(c"init".as_ptr()));
                let app_item_alloc = msg0(ns_item_cls, sel_registerName(c"alloc".as_ptr()));
                let app_item = msg0(app_item_alloc, sel_registerName(c"init".as_ptr()));
                let sub_alloc = msg0(ns_menu_cls, sel_registerName(c"alloc".as_ptr()));
                let sub_menu = msg0(sub_alloc, sel_registerName(c"init".as_ptr()));
                msg1_ptr(
                    app_item,
                    sel_registerName(c"setSubmenu:".as_ptr()),
                    sub_menu,
                );
                msg1_ptr(main_menu, sel_registerName(c"addItem:".as_ptr()), app_item);
                msg1_ptr(
                    ns_app,
                    sel_registerName(c"setMainMenu:".as_ptr()),
                    main_menu,
                );
            }

            let first_item = msg1_isize_ret(
                main_menu,
                sel_registerName(c"itemAtIndex:".as_ptr()),
                0,
            );
            if !first_item.is_null() {
                let app_submenu = msg0(first_item, sel_registerName(c"submenu".as_ptr()));
                if !app_submenu.is_null() {
                    let existing = msg1_isize_ret(
                        app_submenu,
                        sel_registerName(c"itemWithTag:".as_ptr()),
                        9901,
                    );
                    if existing.is_null() {
                        // Remove default stock "About" item at index 0 if present so ours replaces it cleanly
                        let count: isize = {
                            type MsgCount = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
                            let msg_cnt: MsgCount =
                                std::mem::transmute(objc_msgSend as *const c_void);
                            msg_cnt(app_submenu, sel_registerName(c"numberOfItems".as_ptr()))
                        };
                        if count > 0 {
                            msg1_isize(
                                app_submenu,
                                sel_registerName(c"removeItemAtIndex:".as_ptr()),
                                0,
                            );
                        }

                        let empty_key = make_nsstring("");
                        let comma_key = make_nsstring(",");

                        // 0. About Mac Omni Tool
                        let about_title = make_nsstring("About Mac Omni Tool");
                        let about_alloc = msg0(ns_item_cls, sel_registerName(c"alloc".as_ptr()));
                        let about_item = msg_init_item(
                            about_alloc,
                            sel_registerName(
                                c"initWithTitle:action:keyEquivalent:".as_ptr(),
                            ),
                            about_title,
                            sel_registerName(c"aboutMenuClicked:".as_ptr()),
                            empty_key,
                        );
                        msg1_ptr(
                            about_item,
                            sel_registerName(c"setTarget:".as_ptr()),
                            target_obj,
                        );
                        msg1_isize(about_item, sel_registerName(c"setTag:".as_ptr()), 9901);
                        msg_insert(
                            app_submenu,
                            sel_registerName(c"insertItem:atIndex:".as_ptr()),
                            about_item,
                            0,
                        );

                        // 1. Check for Updates…
                        let upd_title = make_nsstring("Check for Updates…");
                        let upd_alloc = msg0(ns_item_cls, sel_registerName(c"alloc".as_ptr()));
                        let upd_item = msg_init_item(
                            upd_alloc,
                            sel_registerName(
                                c"initWithTitle:action:keyEquivalent:".as_ptr(),
                            ),
                            upd_title,
                            sel_registerName(c"aboutMenuClicked:".as_ptr()),
                            empty_key,
                        );
                        msg1_ptr(
                            upd_item,
                            sel_registerName(c"setTarget:".as_ptr()),
                            target_obj,
                        );
                        msg_insert(
                            app_submenu,
                            sel_registerName(c"insertItem:atIndex:".as_ptr()),
                            upd_item,
                            1,
                        );

                        // 2. GitHub Repository
                        let gh_title =
                            make_nsstring("GitHub: boykhacthang/mac-omni-tool");
                        let gh_alloc = msg0(ns_item_cls, sel_registerName(c"alloc".as_ptr()));
                        let gh_item = msg_init_item(
                            gh_alloc,
                            sel_registerName(
                                c"initWithTitle:action:keyEquivalent:".as_ptr(),
                            ),
                            gh_title,
                            sel_registerName(c"githubMenuClicked:".as_ptr()),
                            empty_key,
                        );
                        msg1_ptr(
                            gh_item,
                            sel_registerName(c"setTarget:".as_ptr()),
                            target_obj,
                        );
                        msg_insert(
                            app_submenu,
                            sel_registerName(c"insertItem:atIndex:".as_ptr()),
                            gh_item,
                            2,
                        );

                        // 3. Settings... (Cmd+,)
                        let set_title = make_nsstring("Settings…");
                        let set_alloc = msg0(ns_item_cls, sel_registerName(c"alloc".as_ptr()));
                        let set_item = msg_init_item(
                            set_alloc,
                            sel_registerName(
                                c"initWithTitle:action:keyEquivalent:".as_ptr(),
                            ),
                            set_title,
                            sel_registerName(c"settingsMenuClicked:".as_ptr()),
                            comma_key,
                        );
                        msg1_ptr(
                            set_item,
                            sel_registerName(c"setTarget:".as_ptr()),
                            target_obj,
                        );
                        msg_insert(
                            app_submenu,
                            sel_registerName(c"insertItem:atIndex:".as_ptr()),
                            set_item,
                            3,
                        );

                        // 4. Separator
                        let sep = msg0(ns_item_cls, sel_registerName(c"separatorItem".as_ptr()));
                        msg_insert(
                            app_submenu,
                            sel_registerName(c"insertItem:atIndex:".as_ptr()),
                            sep,
                            4,
                        );

                        // Ensure Quit item exists if menu was empty
                        if count == 0 {
                            let q_key = make_nsstring("q");
                            let q_title = make_nsstring("Quit Mac Omni Tool");
                            let q_alloc = msg0(ns_item_cls, sel_registerName(c"alloc".as_ptr()));
                            let q_item = msg_init_item(
                                q_alloc,
                                sel_registerName(
                                    c"initWithTitle:action:keyEquivalent:".as_ptr(),
                                ),
                                q_title,
                                sel_registerName(c"quitMenuClicked:".as_ptr()),
                                q_key,
                            );
                            msg1_ptr(
                                q_item,
                                sel_registerName(c"setTarget:".as_ptr()),
                                target_obj,
                            );
                            msg1_ptr(app_submenu, sel_registerName(c"addItem:".as_ptr()), q_item);
                        }
                    }
                }
            }
        }
    }
    objc_autoreleasePoolPop(pool);
}

pub unsafe fn set_status_button_title(button: *mut c_void, text: &str) {
    if button.is_null() {
        return;
    }
    let pool = objc_autoreleasePoolPush();
    type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);

    let sel_set_title = sel_registerName(c"setTitle:".as_ptr());
    let ns_title = make_nsstring(text);
    if !ns_title.is_null() {
        msg1_ptr(button, sel_set_title, ns_title);
    }
    objc_autoreleasePoolPop(pool);
}

pub fn update_status_bar_title(title: &str) {
    let ptr = STATUS_BUTTON_PTR.load(Ordering::SeqCst) as *mut c_void;
    unsafe {
        set_status_button_title(ptr, title);
    }
}

/// Create a native macOS NSStatusItem in the top Menu Bar right next to the Battery % icon
pub fn init_native_macos_menubar_item(proxy: EventLoopProxy<AppUserEvent>) {
    unsafe {
        set_macos_dock_icon();
    }
    let lock = MENUBAR_EVENT_PROXY.get_or_init(|| Mutex::new(None));
    *lock.lock().unwrap() = Some(proxy);

    unsafe {
        let pool = objc_autoreleasePoolPush();

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg1F64 = unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void;
        type Msg1Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_f64: Msg1F64 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_ptr: Msg1Ptr = std::mem::transmute(objc_msgSend as *const c_void);

        let ns_object = objc_getClass(c"NSObject".as_ptr());
        let mut target_cls = objc_getClass(c"OmniMenuBarTarget".as_ptr());
        if target_cls.is_null() {
            target_cls = objc_allocateClassPair(ns_object, c"OmniMenuBarTarget".as_ptr(), 0);
            if !target_cls.is_null() {
                class_addMethod(
                    target_cls,
                    sel_registerName(c"statusItemClicked:".as_ptr()),
                    status_bar_item_clicked,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    target_cls,
                    sel_registerName(c"aboutMenuClicked:".as_ptr()),
                    about_menu_clicked,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    target_cls,
                    sel_registerName(c"settingsMenuClicked:".as_ptr()),
                    settings_menu_clicked,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    target_cls,
                    sel_registerName(c"githubMenuClicked:".as_ptr()),
                    github_menu_clicked,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    target_cls,
                    sel_registerName(c"quitMenuClicked:".as_ptr()),
                    quit_menu_clicked,
                    c"v@:@".as_ptr(),
                );
                objc_registerClassPair(target_cls);
            }
        }

        let target_obj = msg0(
            msg0(target_cls, sel_registerName(c"alloc".as_ptr())),
            sel_registerName(c"init".as_ptr()),
        );
        let _ = msg0(target_obj, sel_registerName(c"retain".as_ptr()));
        TARGET_OBJ_PTR.store(target_obj as isize, Ordering::SeqCst);

        let status_bar_cls = objc_getClass(c"NSStatusBar".as_ptr());
        let sys_bar = msg0(status_bar_cls, sel_registerName(c"systemStatusBar".as_ptr()));
        // NSVariableStatusItemLength = -1.0
        let status_item = msg1_f64(
            sys_bar,
            sel_registerName(c"statusItemWithLength:".as_ptr()),
            -1.0,
        );
        let _ = msg0(status_item, sel_registerName(c"retain".as_ptr()));

        let button = msg0(status_item, sel_registerName(c"button".as_ptr()));
        if !button.is_null() {
            STATUS_BUTTON_PTR.store(button as isize, Ordering::SeqCst);
            set_status_button_title(button, "❖ Omni");
            msg1_ptr(
                button,
                sel_registerName(c"setTarget:".as_ptr()),
                target_obj,
            );
            msg1_ptr(
                button,
                sel_registerName(c"setAction:".as_ptr()),
                sel_registerName(c"statusItemClicked:".as_ptr()),
            );
        }

        setup_macos_application_menu();
        objc_autoreleasePoolPop(pool);
    }
}

