//! macOS System Appearance (Light / Dark Mode) detection and switching via `SkyLight` or `System Events`.

use std::ffi::CString;
use std::process::Command;

/// Check if macOS is currently in Dark Mode.
pub fn is_dark_mode() -> bool {
    unsafe {
        if let Ok(path) =
            CString::new("/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight")
        {
            let handle = libc::dlopen(path.as_ptr(), libc::RTLD_LAZY);
            if !handle.is_null() {
                if let Ok(sym) = CString::new("SLSGetAppearanceThemeLegacy") {
                    let func_ptr = libc::dlsym(handle, sym.as_ptr());
                    if !func_ptr.is_null() {
                        type GetThemeFn = unsafe extern "C" fn() -> bool;
                        let func: GetThemeFn = std::mem::transmute(func_ptr);
                        return func();
                    }
                }
            }
        }
    }

    // Fallback via `defaults read -g AppleInterfaceStyle`
    if let Ok(out) = Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        return stdout.trim().eq_ignore_ascii_case("Dark");
    }
    false
}

/// Set macOS System Appearance to Light (`false`) or Dark (`true`).
pub fn set_dark_mode(dark: bool) -> bool {
    unsafe {
        if let Ok(path) =
            CString::new("/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight")
        {
            let handle = libc::dlopen(path.as_ptr(), libc::RTLD_LAZY);
            if !handle.is_null() {
                if let Ok(sym) = CString::new("SLSSetAppearanceThemeLegacy") {
                    let func_ptr = libc::dlsym(handle, sym.as_ptr());
                    if !func_ptr.is_null() {
                        type SetThemeFn = unsafe extern "C" fn(bool);
                        let func: SetThemeFn = std::mem::transmute(func_ptr);
                        func(dark);
                        return is_dark_mode();
                    }
                }
            }
        }
    }

    // Fallback via System Events AppleScript
    let script = format!(
        "tell application \"System Events\" to tell appearance preferences to set dark mode to {}",
        if dark { "true" } else { "false" }
    );
    let _ = Command::new("osascript").args(["-e", &script]).status();
    is_dark_mode()
}

