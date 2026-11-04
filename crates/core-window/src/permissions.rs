//! macOS TCC permissions checker and System Settings deep-linker (`Accessibility`, `Screen Recording`, `Full Disk Access`).

use crate::ffi::*;
use crate::models::PermissionStatus;
use std::ffi::c_void;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Check Accessibility, Screen Recording, and Full Disk Access permissions.
pub fn get_permissions_status() -> PermissionStatus {
    let accessibility_granted = unsafe { AXIsProcessTrusted() };
    let screen_recording_granted = unsafe { CGPreflightScreenCaptureAccess() };
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let full_disk_access_granted =
        fs::read_dir(PathBuf::from(&home).join("Library/Safari")).is_ok()
            || fs::read_dir(PathBuf::from(&home).join("Library/Containers")).is_ok();

    PermissionStatus {
        accessibility_granted,
        screen_recording_granted,
        full_disk_access_granted,
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

/// Trigger a 1x1 pixel screen capture probe so `tccd` / `WindowServer` guarantees that
/// `Mac Omni Tool` is listed inside `System Settings -> Privacy & Security -> Screen Recording`.
fn ensure_screen_recording_entry_registered() {
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            return;
        }
        let _ = CGRequestScreenCaptureAccess();
    }
    let probe_path = std::env::temp_dir().join(format!(
        "mac_omni_tcc_probe_{}.png",
        std::process::id()
    ));
    let _ = Command::new("screencapture")
        .args(["-x", "-R", "0,0,1,1"])
        .arg(&probe_path)
        .status();
    let _ = fs::remove_file(&probe_path);
}

/// Reset stale TCC entries (only when explicitly requested by user) and re-register fresh permission prompts.
pub fn reset_and_request_permissions() {
    register_current_app_with_launch_services();
    unsafe {
        if !AXIsProcessTrusted() {
            let _ = Command::new("tccutil")
                .args(["reset", "Accessibility", "com.khacthang.mac-omni-tool"])
                .status();
            let _ = prompt_accessibility_if_needed();
        }
        if !CGPreflightScreenCaptureAccess() {
            let _ = Command::new("tccutil")
                .args(["reset", "ScreenCapture", "com.khacthang.mac-omni-tool"])
                .status();
            ensure_screen_recording_entry_registered();
        }
    }
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// Open the corresponding macOS System Settings privacy pane without wiping existing TCC entries,
/// while ensuring LaunchServices & `tccd` have registered "Mac Omni Tool" in the list.
pub fn request_or_open_permission(kind: &str) {
    register_current_app_with_launch_services();
    match kind {
        "accessibility" => {
            let _ = prompt_accessibility_if_needed();
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                .spawn();
        }
        "screen_recording" => {
            ensure_screen_recording_entry_registered();
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

