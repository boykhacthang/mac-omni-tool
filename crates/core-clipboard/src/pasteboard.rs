//! macOS `NSPasteboard` read/write operations and 250ms zero-leak background watcher thread.

use crate::codec::{fnv1a_hash, now_unix_ms};
use crate::ffi::*;
use crate::storage::{
    images_dir, is_clipboard_enabled, query_clipboard_items, record_clipboard_entry,
};
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::Duration;

static LAST_CHANGE_COUNT: AtomicIsize = AtomicIsize::new(-1);
static WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Synchronize `LAST_CHANGE_COUNT` with the current `NSPasteboard.generalPasteboard.changeCount`
/// so items copied while the Clipboard Manager was disabled are not retroactively captured upon re-enabling.
pub fn sync_pasteboard_change_count() {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let pb_cls = objc_getClass(c"NSPasteboard".as_ptr());
        if !pb_cls.is_null() {
            type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
            type Msg0Isize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
            let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
            let msg0_isize: Msg0Isize = std::mem::transmute(objc_msgSend as *const c_void);
            let pb = msg0(pb_cls, sel_registerName(c"generalPasteboard".as_ptr()));
            if !pb.is_null() {
                let current_count = msg0_isize(pb, sel_registerName(c"changeCount".as_ptr()));
                LAST_CHANGE_COUNT.store(current_count, Ordering::SeqCst);
            }
        }
        objc_autoreleasePoolPop(pool);
    }
}

/// Write arbitrary UTF-8 text directly to macOS `NSPasteboard`.
pub fn copy_text_to_clipboard(text: &str) -> bool {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let pb_cls = objc_getClass(c"NSPasteboard".as_ptr());
        let ns_str_cls = objc_getClass(c"NSString".as_ptr());

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg0Isize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
        type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        type Msg2 =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> bool;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_isize: Msg0Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
        let msg2: Msg2 = std::mem::transmute(objc_msgSend as *const c_void);

        let pb = msg0(pb_cls, sel_registerName(c"generalPasteboard".as_ptr()));
        let mut ok = false;
        if !pb.is_null() {
            let _ = msg0_isize(pb, sel_registerName(c"clearContents".as_ptr()));
            if let Ok(c_text) = CString::new(text) {
                let ns_val = msg1_str(
                    ns_str_cls,
                    sel_registerName(c"stringWithUTF8String:".as_ptr()),
                    c_text.as_ptr(),
                );
                let ns_type = msg1_str(
                    ns_str_cls,
                    sel_registerName(c"stringWithUTF8String:".as_ptr()),
                    c"public.utf8-plain-text".as_ptr(),
                );
                if !ns_val.is_null() && !ns_type.is_null() {
                    ok = msg2(
                        pb,
                        sel_registerName(c"setString:forType:".as_ptr()),
                        ns_val,
                        ns_type,
                    );
                }
            }
        }
        objc_autoreleasePoolPop(pool);
        ok
    }
}

/// Write raw PNG image bytes directly to macOS `NSPasteboard` and record into Mac Omni Clipboard history.
pub fn copy_png_bytes_to_clipboard(png_bytes: &[u8]) -> Result<(), String> {
    if png_bytes.is_empty() {
        return Err("Dữ liệu ảnh rỗng".to_string());
    }

    unsafe {
        let pool = objc_autoreleasePoolPush();
        let pb_cls = objc_getClass(c"NSPasteboard".as_ptr());
        let ns_str_cls = objc_getClass(c"NSString".as_ptr());
        let ns_data_cls = objc_getClass(c"NSData".as_ptr());

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg0Isize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
        type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        type Msg2 =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> bool;
        type MsgData =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, usize) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_isize: Msg0Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
        let msg2: Msg2 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_data: MsgData = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_general = sel_registerName(c"generalPasteboard".as_ptr());
        let sel_clear = sel_registerName(c"clearContents".as_ptr());
        let sel_change_count = sel_registerName(c"changeCount".as_ptr());
        let sel_str_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
        let sel_set_data = sel_registerName(c"setData:forType:".as_ptr());
        let sel_data_bytes = sel_registerName(c"dataWithBytes:length:".as_ptr());

        let pb = msg0(pb_cls, sel_general);
        if pb.is_null() {
            objc_autoreleasePoolPop(pool);
            return Err("Không thể truy cập NSPasteboard".to_string());
        }

        let _ = msg0_isize(pb, sel_clear);
        let ns_data = msg_data(
            ns_data_cls,
            sel_data_bytes,
            png_bytes.as_ptr() as *const c_void,
            png_bytes.len(),
        );
        let ns_type = msg1_str(ns_str_cls, sel_str_utf8, c"public.png".as_ptr());
        if !ns_data.is_null() && !ns_type.is_null() {
            let _ = msg2(pb, sel_set_data, ns_data, ns_type);
        }

        let new_count = msg0_isize(pb, sel_change_count);
        LAST_CHANGE_COUNT.store(new_count, Ordering::SeqCst);
        objc_autoreleasePoolPop(pool);
    }

    if is_clipboard_enabled() {
        let hash = fnv1a_hash(png_bytes);
        let file_path = images_dir().join(format!("clip_{}.png", hash));
        let _ = fs::write(&file_path, png_bytes);
        let kb = (png_bytes.len() as f64 / 1024.0).round() as i64;
        let label = format!("[Ảnh chụp màn hình • {} KB]", kb.max(1));
        record_clipboard_entry(
            "image",
            &label,
            &file_path.to_string_lossy(),
            png_bytes.len() as i64,
            &hash,
            "Mac Omni Screenshot",
        );
    }

    Ok(())
}

/// Prompt the user via native macOS Save Dialog and save PNG screenshot to disk.
pub fn save_png_bytes_to_file(png_bytes: &[u8]) -> Result<Option<String>, String> {
    if png_bytes.is_empty() {
        return Err("Dữ liệu ảnh rỗng".to_string());
    }

    let ts = now_unix_ms() / 1000;
    let default_name = format!("Screenshot_{}.png", ts);
    let script = format!(
        "POSIX path of (choose file name with prompt \"Lưu ảnh chụp màn hình (.png):\" default name \"{}\" default location (path to desktop folder))",
        default_name
    );

    let out = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Không thể mở hộp thoại lưu file: {}", e))?;

    if !out.status.success() {
        return Ok(None);
    }

    let mut chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if chosen.is_empty() {
        return Ok(None);
    }
    if !chosen.to_lowercase().ends_with(".png") {
        chosen.push_str(".png");
    }

    fs::write(&chosen, png_bytes).map_err(|e| format!("Lỗi ghi file '{}': {}", chosen, e))?;
    Ok(Some(chosen))
}

/// Copy a saved clipboard item back to macOS `generalPasteboard` and optionally simulate `Cmd+V`.
pub fn activate_clipboard_item(id: i64, simulate_paste: bool) -> Result<(), String> {
    let item = {
        let all = query_clipboard_items("", "all", 1000);
        all.into_iter()
            .find(|i| i.id == id)
            .ok_or_else(|| "Không tìm thấy mục clipboard".to_string())?
    };

    unsafe {
        let pool = objc_autoreleasePoolPush();
        let pb_cls = objc_getClass(c"NSPasteboard".as_ptr());
        let ns_str_cls = objc_getClass(c"NSString".as_ptr());
        let ns_data_cls = objc_getClass(c"NSData".as_ptr());

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg0Isize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
        type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        type Msg2 =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> bool;
        type MsgData =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, usize) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_isize: Msg0Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
        let msg2: Msg2 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_data: MsgData = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_general = sel_registerName(c"generalPasteboard".as_ptr());
        let sel_clear = sel_registerName(c"clearContents".as_ptr());
        let sel_change_count = sel_registerName(c"changeCount".as_ptr());
        let sel_str_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
        let sel_set_str = sel_registerName(c"setString:forType:".as_ptr());
        let sel_set_data = sel_registerName(c"setData:forType:".as_ptr());
        let sel_data_bytes = sel_registerName(c"dataWithBytes:length:".as_ptr());

        let pb = msg0(pb_cls, sel_general);
        if !pb.is_null() {
            let _ = msg0_isize(pb, sel_clear);

            if item.kind == "image" && !item.image_path.is_empty() {
                if let Ok(bytes) = fs::read(&item.image_path) {
                    let ns_data = msg_data(
                        ns_data_cls,
                        sel_data_bytes,
                        bytes.as_ptr() as *const c_void,
                        bytes.len(),
                    );
                    let ns_type = msg1_str(ns_str_cls, sel_str_utf8, c"public.png".as_ptr());
                    if !ns_data.is_null() && !ns_type.is_null() {
                        let _ = msg2(pb, sel_set_data, ns_data, ns_type);
                    }
                }
            } else if let Ok(c_text) = CString::new(item.full_text.as_str()) {
                let ns_val = msg1_str(ns_str_cls, sel_str_utf8, c_text.as_ptr());
                let ns_type =
                    msg1_str(ns_str_cls, sel_str_utf8, c"public.utf8-plain-text".as_ptr());
                if !ns_val.is_null() && !ns_type.is_null() {
                    let _ = msg2(pb, sel_set_str, ns_val, ns_type);
                }
            }

            let new_count = msg0_isize(pb, sel_change_count);
            LAST_CHANGE_COUNT.store(new_count, Ordering::SeqCst);
        }
        objc_autoreleasePoolPop(pool);
    }

    if simulate_paste && unsafe { AXIsProcessTrusted() } {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(80));
            unsafe {
                // Virtual keycode 9 = 'V', Flag 0x100000 = Command
                let down = CGEventCreateKeyboardEvent(std::ptr::null(), 9, true);
                let up = CGEventCreateKeyboardEvent(std::ptr::null(), 9, false);
                if !down.is_null() && !up.is_null() {
                    CGEventSetFlags(down, 0x0010_0000);
                    CGEventSetFlags(up, 0x0010_0000);
                    CGEventPost(0, down);
                    CGEventPost(0, up);
                    CFRelease(down);
                    CFRelease(up);
                }
            }
        });
    }

    Ok(())
}

/// Single 250ms poll cycle wrapped in an Objective-C autoreleasepool (0% CPU, 0 memory leak).
pub fn poll_pasteboard_once() {
    if !is_clipboard_enabled() {
        return;
    }
    unsafe {
        let pool = objc_autoreleasePoolPush();

        let pb_cls = objc_getClass(c"NSPasteboard".as_ptr());
        if pb_cls.is_null() {
            objc_autoreleasePoolPop(pool);
            return;
        }

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg0Isize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;
        type Msg0Usize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type Msg0Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const u8;
        type Msg1 = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        type Msg2Repr = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *mut c_void,
        ) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_isize: Msg0Isize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_usize: Msg0Usize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_ptr: Msg0Ptr = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1: Msg1 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
        let msg2_repr: Msg2Repr = std::mem::transmute(objc_msgSend as *const c_void);

        let sel_general = sel_registerName(c"generalPasteboard".as_ptr());
        let sel_change_count = sel_registerName(c"changeCount".as_ptr());

        let pb = msg0(pb_cls, sel_general);
        if pb.is_null() {
            objc_autoreleasePoolPop(pool);
            return;
        }

        let current_count = msg0_isize(pb, sel_change_count);
        let prev_count = LAST_CHANGE_COUNT.load(Ordering::Relaxed);
        if current_count == prev_count {
            objc_autoreleasePoolPop(pool);
            return;
        }
        LAST_CHANGE_COUNT.store(current_count, Ordering::Relaxed);

        let ns_str_cls = objc_getClass(c"NSString".as_ptr());
        let sel_str_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
        let sel_data_for_type = sel_registerName(c"dataForType:".as_ptr());
        let sel_str_for_type = sel_registerName(c"stringForType:".as_ptr());
        let sel_utf8 = sel_registerName(c"UTF8String".as_ptr());
        let sel_bytes = sel_registerName(c"bytes".as_ptr());
        let sel_length = sel_registerName(c"length".as_ptr());

        // Check for concealed/transient types (password managers)
        let concealed_type = msg1_str(
            ns_str_cls,
            sel_str_utf8,
            c"org.nspasteboard.ConcealedType".as_ptr(),
        );
        if !concealed_type.is_null() && !msg1(pb, sel_data_for_type, concealed_type).is_null() {
            objc_autoreleasePoolPop(pool);
            return;
        }

        // 1. Check for PNG or TIFF Image on clipboard
        let png_type = msg1_str(ns_str_cls, sel_str_utf8, c"public.png".as_ptr());
        let tiff_type = msg1_str(ns_str_cls, sel_str_utf8, c"public.tiff".as_ptr());
        let mut png_data = if !png_type.is_null() {
            msg1(pb, sel_data_for_type, png_type)
        } else {
            std::ptr::null_mut()
        };

        if png_data.is_null() && !tiff_type.is_null() {
            let tiff_data = msg1(pb, sel_data_for_type, tiff_type);
            if !tiff_data.is_null() {
                let rep_cls = objc_getClass(c"NSBitmapImageRep".as_ptr());
                let sel_rep_data = sel_registerName(c"imageRepWithData:".as_ptr());
                let sel_repr = sel_registerName(c"representationUsingType:properties:".as_ptr());
                let rep = msg1(rep_cls, sel_rep_data, tiff_data);
                if !rep.is_null() {
                    png_data = msg2_repr(rep, sel_repr, 4, std::ptr::null_mut());
                }
            }
        }

        if !png_data.is_null() {
            let len = msg0_usize(png_data, sel_length);
            let ptr = msg0_ptr(png_data, sel_bytes);
            if len > 0 && !ptr.is_null() {
                let slice = std::slice::from_raw_parts(ptr, len);
                let hash = fnv1a_hash(slice);
                let file_path = images_dir().join(format!("clip_{}.png", hash));
                if !file_path.exists() {
                    let _ = fs::write(&file_path, slice);
                }
                let kb = (len as f64 / 1024.0).round() as i64;
                let label = format!("[Hình ảnh Clipboard • {} KB]", kb.max(1));
                record_clipboard_entry(
                    "image",
                    &label,
                    &file_path.to_string_lossy(),
                    len as i64,
                    &hash,
                    "macOS Clipboard",
                );
                objc_autoreleasePoolPop(pool);
                return;
            }
        }

        // 2. Check for UTF-8 Text on clipboard
        let text_type = msg1_str(
            ns_str_cls,
            sel_str_utf8,
            c"public.utf8-plain-text".as_ptr(),
        );
        if !text_type.is_null() {
            let ns_str = msg1(pb, sel_str_for_type, text_type);
            if !ns_str.is_null() {
                let c_ptr = msg0_ptr(ns_str, sel_utf8) as *const c_char;
                if !c_ptr.is_null() {
                    let text = CStr::from_ptr(c_ptr).to_string_lossy().into_owned();
                    if !text.trim().is_empty() {
                        let hash = fnv1a_hash(text.as_bytes());
                        record_clipboard_entry(
                            "text",
                            &text,
                            "",
                            text.len() as i64,
                            &hash,
                            "macOS Clipboard",
                        );
                    }
                }
            }
        }

        objc_autoreleasePoolPop(pool);
    }
}

/// Start the lightweight 250ms `NSPasteboard` watcher thread.
pub fn start_clipboard_watcher() {
    if WATCHER_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::Builder::new()
        .name("clipboard-watcher-250ms".to_string())
        .spawn(|| loop {
            poll_pasteboard_once();
            std::thread::sleep(Duration::from_millis(250));
        })
        .expect("Failed to spawn clipboard watcher thread");
}

