//! macOS Spaces (Multi-Desktop) inspection via `CGSCopyManagedDisplaySpaces`
//! and Frontmost / Running GUI Application discovery via `NSWorkspace`.

use crate::ffi::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicI64, Ordering};

pub(crate) static LAST_MOUSE_BUTTON: AtomicI64 = AtomicI64::new(0);
pub(crate) static LAST_MOUSE_BUTTON_AT_MS: AtomicI64 = AtomicI64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacesStatus {
    /// Total number of open Desktops (Spaces) across all displays
    pub total_spaces: usize,
    /// Number of Spaces on the active/primary display
    pub primary_display_spaces: usize,
    /// 1-based index of the active Space on the primary display
    pub current_space_index: usize,
    /// Frontmost application bundle identifier (e.g. "com.google.Chrome")
    pub frontmost_bundle_id: String,
    /// Frontmost application display name (e.g. "Google Chrome")
    pub frontmost_app_name: String,
    /// Last pressed external mouse button number (3 = Middle, 4 = Back, 5 = Forward)
    pub last_mouse_button: i64,
    /// Timestamp (ms) when the last external mouse button was pressed
    pub last_mouse_button_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAppItem {
    pub bundle_id: String,
    pub app_name: String,
}

unsafe fn nsstring_from_cstr(cstr: *const c_char) -> *mut c_void {
    let cls = objc_getClass(c"NSString".as_ptr());
    if cls.is_null() {
        return std::ptr::null_mut();
    }
    let sel = sel_registerName(c"stringWithUTF8String:".as_ptr());
    type MsgStr = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
    let msg: MsgStr = std::mem::transmute(objc_msgSend as *const c_void);
    msg(cls, sel, cstr)
}

unsafe fn nsstring_to_rust(ns_str: *mut c_void) -> Option<String> {
    if ns_str.is_null() {
        return None;
    }
    let sel_utf8 = sel_registerName(c"UTF8String".as_ptr());
    type MsgUtf8 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_char;
    let msg_utf8: MsgUtf8 = std::mem::transmute(objc_msgSend as *const c_void);
    let c_ptr = msg_utf8(ns_str, sel_utf8);
    if c_ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(c_ptr).to_string_lossy().into_owned())
    }
}

/// Query `(total_spaces, primary_display_spaces, current_space_index)` from macOS WindowServer.
pub fn query_macos_spaces_counts() -> (usize, usize, usize) {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let conn = CGSMainConnectionID();
        if conn == 0 {
            objc_autoreleasePoolPop(pool);
            return (1, 1, 1);
        }

        let arr = CGSCopyManagedDisplaySpaces(conn);
        if arr.is_null() {
            objc_autoreleasePoolPop(pool);
            return (1, 1, 1);
        }

        type MsgCount = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type MsgIdx = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void;
        type MsgObj = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
        type MsgI64 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i64;

        let msg_count: MsgCount = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_idx: MsgIdx = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_obj: MsgObj = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_i64: MsgI64 = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_count = sel_registerName(c"count".as_ptr());
        let sel_at = sel_registerName(c"objectAtIndex:".as_ptr());
        let sel_for_key = sel_registerName(c"objectForKey:".as_ptr());
        let sel_long = sel_registerName(c"longLongValue".as_ptr());

        let key_spaces = nsstring_from_cstr(c"Spaces".as_ptr());
        let key_cur_space = nsstring_from_cstr(c"Current Space".as_ptr());
        let key_id64 = nsstring_from_cstr(c"id64".as_ptr());
        let key_managed_id = nsstring_from_cstr(c"ManagedSpaceID".as_ptr());

        let disp_count = msg_count(arr, sel_count);
        let mut total_spaces = 0usize;
        let mut primary_spaces = 0usize;
        let mut current_idx = 1usize;

        for d in 0..disp_count {
            let disp_dict = msg_idx(arr, sel_at, d);
            if disp_dict.is_null() {
                continue;
            }
            let spaces_arr = msg_obj(disp_dict, sel_for_key, key_spaces);
            if spaces_arr.is_null() {
                continue;
            }
            let s_count = msg_count(spaces_arr, sel_count);
            total_spaces += s_count;

            if d == 0 {
                primary_spaces = s_count.max(1);
                let cur_dict = msg_obj(disp_dict, sel_for_key, key_cur_space);
                if !cur_dict.is_null() {
                    let mut cur_id_num = msg_obj(cur_dict, sel_for_key, key_id64);
                    if cur_id_num.is_null() {
                        cur_id_num = msg_obj(cur_dict, sel_for_key, key_managed_id);
                    }
                    let cur_id = if !cur_id_num.is_null() {
                        msg_i64(cur_id_num, sel_long)
                    } else {
                        -1
                    };

                    if cur_id >= 0 {
                        for s_idx in 0..s_count {
                            let s_dict = msg_idx(spaces_arr, sel_at, s_idx);
                            if !s_dict.is_null() {
                                let mut s_id_num = msg_obj(s_dict, sel_for_key, key_id64);
                                if s_id_num.is_null() {
                                    s_id_num = msg_obj(s_dict, sel_for_key, key_managed_id);
                                }
                                if !s_id_num.is_null() && msg_i64(s_id_num, sel_long) == cur_id {
                                    current_idx = s_idx + 1;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        CFRelease(arr);
        objc_autoreleasePoolPop(pool);

        (
            total_spaces.max(1),
            primary_spaces.max(1),
            current_idx.max(1),
        )
    }
}

/// Return the `(bundle_id, localized_name)` of the currently frontmost application.
pub fn frontmost_app_info() -> (String, String) {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let ws_cls = objc_getClass(c"NSWorkspace".as_ptr());
        if ws_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return (String::new(), String::new());
        }

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_shared = sel_registerName(c"sharedWorkspace".as_ptr());
        let sel_front = sel_registerName(c"frontmostApplication".as_ptr());
        let sel_bundle = sel_registerName(c"bundleIdentifier".as_ptr());
        let sel_name = sel_registerName(c"localizedName".as_ptr());

        let ws = msg0(ws_cls, sel_shared);
        if ws.is_null() {
            objc_autoreleasePoolPop(pool);
            return (String::new(), String::new());
        }
        let app = msg0(ws, sel_front);
        if app.is_null() {
            objc_autoreleasePoolPop(pool);
            return (String::new(), String::new());
        }

        let bundle_id = nsstring_to_rust(msg0(app, sel_bundle)).unwrap_or_default();
        let app_name = nsstring_to_rust(msg0(app, sel_name)).unwrap_or_else(|| bundle_id.clone());

        objc_autoreleasePoolPop(pool);
        (bundle_id, app_name)
    }
}

/// Get complete Spaces & Frontmost Application status for the External Mouse UI.
pub fn get_spaces_status() -> SpacesStatus {
    let (total_spaces, primary_display_spaces, current_space_index) = query_macos_spaces_counts();
    let (frontmost_bundle_id, frontmost_app_name) = frontmost_app_info();
    SpacesStatus {
        total_spaces,
        primary_display_spaces,
        current_space_index,
        frontmost_bundle_id,
        frontmost_app_name,
        last_mouse_button: LAST_MOUSE_BUTTON.load(Ordering::Relaxed),
        last_mouse_button_at_ms: LAST_MOUSE_BUTTON_AT_MS.load(Ordering::Relaxed),
    }
}

/// List all currently running regular GUI applications (`NSApplicationActivationPolicyRegular`).
pub fn list_running_gui_apps() -> Vec<RunningAppItem> {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let ws_cls = objc_getClass(c"NSWorkspace".as_ptr());
        if ws_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return Vec::new();
        }

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgCount = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type MsgIdx = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void;
        type MsgPolicy = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_count: MsgCount = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_idx: MsgIdx = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_policy: MsgPolicy = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_shared = sel_registerName(c"sharedWorkspace".as_ptr());
        let sel_running = sel_registerName(c"runningApplications".as_ptr());
        let sel_count = sel_registerName(c"count".as_ptr());
        let sel_at = sel_registerName(c"objectAtIndex:".as_ptr());
        let sel_policy = sel_registerName(c"activationPolicy".as_ptr());
        let sel_bundle = sel_registerName(c"bundleIdentifier".as_ptr());
        let sel_name = sel_registerName(c"localizedName".as_ptr());

        let ws = msg0(ws_cls, sel_shared);
        if ws.is_null() {
            objc_autoreleasePoolPop(pool);
            return Vec::new();
        }
        let apps_arr = msg0(ws, sel_running);
        if apps_arr.is_null() {
            objc_autoreleasePoolPop(pool);
            return Vec::new();
        }

        let n = msg_count(apps_arr, sel_count);
        let mut by_bundle: BTreeMap<String, String> = BTreeMap::new();

        for i in 0..n {
            let app = msg_idx(apps_arr, sel_at, i);
            if app.is_null() {
                continue;
            }
            // NSApplicationActivationPolicyRegular == 0
            if msg_policy(app, sel_policy) != 0 {
                continue;
            }
            if let Some(bundle_id) = nsstring_to_rust(msg0(app, sel_bundle)) {
                let b_trim = bundle_id.trim();
                if !b_trim.is_empty() {
                    let name = nsstring_to_rust(msg0(app, sel_name))
                        .unwrap_or_else(|| b_trim.to_string());
                    by_bundle.insert(b_trim.to_string(), name);
                }
            }
        }

        objc_autoreleasePoolPop(pool);

        let mut list: Vec<RunningAppItem> = by_bundle
            .into_iter()
            .map(|(bundle_id, app_name)| RunningAppItem {
                bundle_id,
                app_name,
            })
            .collect();
        list.sort_by(|a, b| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()));
        list
    }
}

/// Read `bundle_id` and display `app_name` from a `.app` bundle directory on disk via `NSBundle`.
pub fn read_app_bundle_info(app_path: &std::path::Path) -> Option<RunningAppItem> {
    if !app_path.exists() {
        return None;
    }
    let path_str = app_path.to_str()?;
    let c_path = std::ffi::CString::new(path_str).ok()?;
    let fallback_name = app_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("App")
        .to_string();

    unsafe {
        let pool = objc_autoreleasePoolPush();
        let bundle_cls = objc_getClass(c"NSBundle".as_ptr());
        if bundle_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }
        type Msg1 = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        let msg1: Msg1 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);

        let ns_path = nsstring_from_cstr(c_path.as_ptr());
        if ns_path.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }
        let sel_bundle_with_path = sel_registerName(c"bundleWithPath:".as_ptr());
        let bundle = msg1(bundle_cls, sel_bundle_with_path, ns_path);
        if bundle.is_null() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let sel_id = sel_registerName(c"bundleIdentifier".as_ptr());
        let bundle_id = nsstring_to_rust(msg0(bundle, sel_id)).unwrap_or_default();
        let b_trim = bundle_id.trim().to_string();
        if b_trim.is_empty() {
            objc_autoreleasePoolPop(pool);
            return None;
        }

        let sel_obj_for_info = sel_registerName(c"objectForInfoDictionaryKey:".as_ptr());
        let key_disp = nsstring_from_cstr(c"CFBundleDisplayName".as_ptr());
        let key_name = nsstring_from_cstr(c"CFBundleName".as_ptr());

        let mut app_name = nsstring_to_rust(msg1(bundle, sel_obj_for_info, key_disp))
            .filter(|s| !s.trim().is_empty());
        if app_name.is_none() {
            app_name = nsstring_to_rust(msg1(bundle, sel_obj_for_info, key_name))
                .filter(|s| !s.trim().is_empty());
        }
        let final_name = app_name.unwrap_or(fallback_name);

        objc_autoreleasePoolPop(pool);
        Some(RunningAppItem {
            bundle_id: b_trim,
            app_name: final_name,
        })
    }
}

/// Scan standard macOS application directories (`/Applications`, `/System/Applications`, `~/Applications`)
/// and return all installed GUI `.app` bundles with their real `bundle_id` and `app_name`.
pub fn list_installed_gui_apps() -> Vec<RunningAppItem> {
    let mut dirs = vec![
        std::path::PathBuf::from("/Applications"),
        std::path::PathBuf::from("/Applications/Utilities"),
        std::path::PathBuf::from("/System/Applications"),
        std::path::PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(std::path::PathBuf::from(home).join("Applications"));
    }

    let mut by_bundle: BTreeMap<String, RunningAppItem> = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                if let Some(item) = read_app_bundle_info(&path) {
                    by_bundle
                        .entry(item.bundle_id.to_lowercase())
                        .or_insert(item);
                }
            }
        }
    }

    let mut list: Vec<RunningAppItem> = by_bundle.into_values().collect();
    list.sort_by(|a, b| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()));
    list
}

/// Open the native macOS Finder file dialog to pick an installed `.app` bundle from `/Applications`.
pub fn pick_application_bundle_dialog() -> Option<RunningAppItem> {
    let out = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"POSIX path of (choose file of type {"app", "com.apple.application-bundle"} with prompt "Chọn ứng dụng (.app):" default location (POSIX file "/Applications"))"#,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw_path.is_empty() {
        return None;
    }
    let clean_path = raw_path.trim_end_matches('/');
    read_app_bundle_info(std::path::Path::new(clean_path))
}
