//! macOS TCC permissions checker and System Settings deep-linker (`Accessibility`, `Screen Recording`, `Full Disk Access`).

use crate::ffi::*;
use crate::models::PermissionStatus;
use std::ffi::c_void;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Silent check whether macOS Accessibility (`AXIsProcessTrusted`) is currently granted.
/// Never triggers any macOS TCC prompt.
pub fn is_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Silent check whether macOS Screen Recording (`CGPreflightScreenCaptureAccess`) is currently granted.
/// Never triggers any macOS TCC prompt.
pub fn is_screen_recording_granted() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Silent check whether macOS Full Disk Access is currently granted.
/// Checks protected directories (`~/Library/Safari`, `~/Library/Mail`, `/Library/Application Support/com.apple.TCC`)
/// that return `EPERM` silently without triggering any macOS TCC popup (unlike `~/Library/Containers`).
pub fn is_full_disk_access_granted() -> bool {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let home_path = PathBuf::from(&home);
    fs::read_dir(home_path.join("Library/Safari")).is_ok()
        || fs::read_dir(home_path.join("Library/Mail")).is_ok()
        || fs::read_dir("/Library/Application Support/com.apple.TCC").is_ok()
}

/// Check Accessibility, Screen Recording, and Full Disk Access permissions silently without triggering any prompts.
pub fn get_permissions_status() -> PermissionStatus {
    PermissionStatus {
        accessibility_granted: is_accessibility_trusted(),
        screen_recording_granted: is_screen_recording_granted(),
        full_disk_access_granted: is_full_disk_access_granted(),
    }
}

/// Prompt macOS to register this app in Accessibility permissions if not yet trusted.
pub fn prompt_accessibility_if_needed() -> bool {
    unsafe {
        if AXIsProcessTrusted() {
            return true;
        }
        let keys = [kAXTrustedCheckOptionPrompt as *const c_void];
        let values = [kCFBooleanTrue];
        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            std::ptr::null(),
            std::ptr::null(),
        );
        let trusted = AXIsProcessTrustedWithOptions(dict);
        if !dict.is_null() {
            CFRelease(dict);
        }
        trusted
    }
}

/// Locate the current `.app` bundle path (or `/Applications/MacOmniTool.app`) and register it
/// with macOS LaunchServices (`lsregister -f`) so `tccd` always resolves the "Mac Omni Tool" app name and icon.
pub fn register_current_app_with_launch_services() {
    let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
    if !std::path::Path::new(lsregister).exists() {
        return;
    }

    let mut candidate_apps: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let resolved = fs::canonicalize(&exe).unwrap_or(exe);
        for ancestor in resolved.ancestors() {
            if ancestor.extension().and_then(|s| s.to_str()) == Some("app") {
                candidate_apps.push(ancestor.to_path_buf());
                break;
            }
        }
    }
    let sys_app = PathBuf::from("/Applications/MacOmniTool.app");
    if sys_app.exists() && !candidate_apps.contains(&sys_app) {
        candidate_apps.push(sys_app);
    }

    for app_path in candidate_apps {
        let _ = Command::new(lsregister)
            .arg("-f")
            .arg(&app_path)
            .status();
    }
}

/// Request Screen Recording registration with `tccd` / `WindowServer` when explicitly requested by user in App Settings.
fn ensure_screen_recording_entry_registered() {
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            return;
        }
        let _ = CGRequestScreenCaptureAccess();
    }
}

/// Reset stale TCC entries (only when explicitly requested by user) and re-register fresh permission prompts.
pub fn reset_and_request_permissions() {
    register_current_app_with_launch_services();
    if !is_accessibility_trusted() {
        let _ = Command::new("tccutil")
            .args(["reset", "Accessibility", "com.khacthang.mac-omni-tool"])
            .status();
        let _ = prompt_accessibility_if_needed();
    }
    if !is_screen_recording_granted() {
        let _ = Command::new("tccutil")
            .args(["reset", "ScreenCapture", "com.khacthang.mac-omni-tool"])
            .status();
        ensure_screen_recording_entry_registered();
    }
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// Open the corresponding macOS System Settings privacy pane when explicitly requested by the user in App Settings.
/// If the permission is not currently granted (e.g. first time or stale `CDHash` after an app rebuild/update where the toggle
/// in System Settings still appears ON), automatically resets the stale TCC entry for `com.khacthang.mac-omni-tool` first
/// so the user can toggle it ON cleanly without getting stuck.
pub fn request_or_open_permission(kind: &str) {
    register_current_app_with_launch_services();
    match kind {
        "accessibility" => {
            if !is_accessibility_trusted() {
                let _ = Command::new("tccutil")
                    .args(["reset", "Accessibility", "com.khacthang.mac-omni-tool"])
                    .status();
                let _ = prompt_accessibility_if_needed();
            }
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                .spawn();
        }
        "screen_recording" => {
            if !is_screen_recording_granted() {
                let _ = Command::new("tccutil")
                    .args(["reset", "ScreenCapture", "com.khacthang.mac-omni-tool"])
                    .status();
                ensure_screen_recording_entry_registered();
            }
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                .spawn();
        }
        "full_disk" => {
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
                .spawn();
        }
        _ => {}
    }
}

