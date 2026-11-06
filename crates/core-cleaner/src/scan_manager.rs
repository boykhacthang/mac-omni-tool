//! Asynchronous background scan runner, progress state tracker, and result cache.

use crate::duplicates::scan_duplicate_files;
use crate::large_files::scan_large_files;
use crate::models::{CleanerScanStatus, InstalledAppInfo, JunkScanSummary};
use crate::trash_and_junk::scan_system_junk;
use crate::uninstaller::scan_installed_applications;
use std::sync::{Mutex, OnceLock};

static SCAN_STATUS: OnceLock<Mutex<CleanerScanStatus>> = OnceLock::new();

fn status_mutex() -> &'static Mutex<CleanerScanStatus> {
    SCAN_STATUS.get_or_init(|| Mutex::new(CleanerScanStatus::default()))
}

pub(crate) fn update_scan_progress(kind: &str, running: bool, percent: u8, message: &str) {
    if let Ok(mut guard) = status_mutex().lock() {
        guard.running = running;
        guard.kind = kind.to_string();
        guard.percent = percent;
        guard.message = message.to_string();
    }
}

pub fn get_cleaner_scan_status() -> CleanerScanStatus {
    status_mutex().lock().unwrap().clone()
}

pub fn invalidate_cleaner_cache() {
    if let Ok(mut guard) = status_mutex().lock() {
        guard.junk_result = None;
        guard.apps_result = None;
        guard.large_result = None;
        guard.duplicates_result = None;
    }
    crate::uninstaller::clear_app_icon_cache();
}

/// Free cached scan results and Base64 app icons when the main window is hidden to background
/// (unless a background scan is actively in progress).
pub fn release_cleaner_memory() {
    if let Ok(mut guard) = status_mutex().lock() {
        if !guard.running {
            guard.junk_result = None;
            guard.apps_result = None;
            guard.large_result = None;
            guard.duplicates_result = None;
            crate::uninstaller::clear_app_icon_cache();
        }
    }
}

pub fn scan_system_junk_cached(force: bool) -> JunkScanSummary {
    if !force {
        if let Ok(guard) = status_mutex().lock() {
            if let Some(ref res) = guard.junk_result {
                return res.clone();
            }
        }
    }
    let res = scan_system_junk();
    if let Ok(mut guard) = status_mutex().lock() {
        guard.junk_result = Some(res.clone());
    }
    res
}

pub fn scan_installed_applications_cached(force: bool) -> Vec<InstalledAppInfo> {
    if !force {
        if let Ok(guard) = status_mutex().lock() {
            if let Some(ref res) = guard.apps_result {
                return res.clone();
            }
        }
    }
    let res = scan_installed_applications();
    if let Ok(mut guard) = status_mutex().lock() {
        guard.apps_result = Some(res.clone());
    }
    res
}

pub fn prewarm_cleaner_cache() {
    std::thread::spawn(|| {
        let junk = scan_system_junk();
        if let Ok(mut guard) = status_mutex().lock() {
            if guard.junk_result.is_none() {
                guard.junk_result = Some(junk);
            }
        }
        let apps = scan_installed_applications();
        if let Ok(mut guard) = status_mutex().lock() {
            if guard.apps_result.is_none() {
                guard.apps_result = Some(apps);
            }
        }
    });
}

pub fn start_cleaner_scan_async(
    kind: &str,
    scope: &str,
    min_size_val: u64,
    force: bool,
) -> CleanerScanStatus {
    {
        let mut guard = status_mutex().lock().unwrap();
        if guard.running && guard.kind == kind {
            return guard.clone();
        }
        if !force {
            match kind {
                "junk" if guard.junk_result.is_some() => return guard.clone(),
                "uninstall" if guard.apps_result.is_some() => return guard.clone(),
                "large"
                    if guard.large_result.is_some()
                        && guard.large_scope == scope
                        && guard.large_min_mb == min_size_val =>
                {
                    return guard.clone();
                }
                "duplicates"
                    if guard.duplicates_result.is_some()
                        && guard.duplicates_scope == scope
                        && guard.duplicates_min_kb == min_size_val =>
                {
                    return guard.clone();
                }
                _ => {}
            }
        }
        guard.running = true;
        guard.kind = kind.to_string();
        guard.percent = 6;
        guard.message = "Đang khởi tạo trình quét đa luồng...".to_string();
    }

    let kind_owned = kind.to_string();
    let scope_owned = scope.to_string();
    std::thread::spawn(move || {
        match kind_owned.as_str() {
            "junk" => {
                let res = scan_system_junk();
                if let Ok(mut guard) = status_mutex().lock() {
                    guard.junk_result = Some(res);
                    guard.running = false;
                    guard.percent = 100;
                    guard.message = "Đã quét xong!".to_string();
                }
            }
            "uninstall" => {
                let res = scan_installed_applications();
                if let Ok(mut guard) = status_mutex().lock() {
                    guard.apps_result = Some(res);
                    guard.running = false;
                    guard.percent = 100;
                    guard.message = "Đã quét xong danh sách ứng dụng!".to_string();
                }
            }
            "large" => {
                let res = scan_large_files(&scope_owned, min_size_val);
                if let Ok(mut guard) = status_mutex().lock() {
                    guard.large_result = Some(res);
                    guard.large_scope = scope_owned;
                    guard.large_min_mb = min_size_val;
                    guard.running = false;
                    guard.percent = 100;
                    guard.message = "Đã quét xong tệp dung lượng lớn!".to_string();
                }
            }
            "duplicates" => {
                let res = scan_duplicate_files(&scope_owned, min_size_val);
                if let Ok(mut guard) = status_mutex().lock() {
                    guard.duplicates_result = Some(res);
                    guard.duplicates_scope = scope_owned;
                    guard.duplicates_min_kb = min_size_val;
                    guard.running = false;
                    guard.percent = 100;
                    guard.message = "Đã quét xong các tệp bị trùng lặp!".to_string();
                }
            }
            _ => {
                if let Ok(mut guard) = status_mutex().lock() {
                    guard.running = false;
                    guard.percent = 100;
                }
            }
        }
    });

    get_cleaner_scan_status()
}

