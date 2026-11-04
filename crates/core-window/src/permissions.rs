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

/// Reset stale TCC entries (after rebuild) and prompt macOS for fresh permission.
pub fn reset_and_request_permissions() {
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
            let _ = CGRequestScreenCaptureAccess();
        }
    }
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// Open the corresponding macOS System Settings privacy pane (automatically clearing stale rebuild TCC cache if needed).
pub fn request_or_open_permission(kind: &str) {
    match kind {
        "accessibility" => unsafe {
            if !AXIsProcessTrusted() {
                let _ = Command::new("tccutil")
                    .args(["reset", "Accessibility", "com.khacthang.mac-omni-tool"])
                    .status();
            }
            let _ = prompt_accessibility_if_needed();
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                .spawn();
        },
        "screen_recording" => unsafe {
            if !CGPreflightScreenCaptureAccess() {
                let _ = Command::new("tccutil")
                    .args(["reset", "ScreenCapture", "com.khacthang.mac-omni-tool"])
                    .status();
            }
            let _ = CGRequestScreenCaptureAccess();
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                .spawn();
        },
        "full_disk" => {
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
                .spawn();
        }
        _ => {}
    }
}

