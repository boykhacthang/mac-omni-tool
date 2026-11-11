//! Asynchronous background scan runner, progress state tracker, and result cache.

use crate::duplicates::{get_duplicate_exclude_list, scan_duplicate_files, should_skip_duplicate_dir};
use crate::large_files::scan_large_files;
use crate::models::{CleanerScanStatus, InstalledAppInfo, JunkScanSummary};
use crate::trash_and_junk::{get_trash_status, scan_system_junk};
use crate::uninstaller::scan_installed_applications;
use std::collections::HashSet;
use std::path::Path;
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
                "uninstall" | "apps" if guard.apps_result.is_some() => return guard.clone(),
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
            "uninstall" | "apps" => {
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

/// Immediately remove deleted file/folder paths from the in-memory `CleanerScanStatus` cache
/// (Large Files, Duplicates, Junk & Trash) without triggering a blocking loading bar rescan.
/// Also refreshes macOS Trash stats in a non-blocking background thread if files were moved to Trash.
pub fn remove_deleted_paths_from_cached_scan(
    deleted_paths: &[String],
    emptied_trash: bool,
) -> CleanerScanStatus {
    let deleted_set: HashSet<&str> = deleted_paths.iter().map(|s| s.as_str()).collect();
    if let Ok(mut guard) = status_mutex().lock() {
        // 1. Update Large Files
        if let Some(ref mut large) = guard.large_result {
            large.retain(|entry| !deleted_set.contains(entry.path.as_str()));
        }

        // 2. Update Duplicate Files
        if let Some(ref mut dups) = guard.duplicates_result {
            for grp in &mut dups.groups {
                grp.files.retain(|f| !deleted_set.contains(f.path.as_str()));
                if !grp.files.is_empty() && !grp.files.iter().any(|f| f.is_original) {
                    grp.files[0].is_original = true;
                }
                grp.count = grp.files.len();
                grp.wasted_bytes = grp
                    .file_size_bytes
                    .saturating_mul(grp.count.saturating_sub(1) as u64);
                grp.wasted_mb = (grp.wasted_bytes as f64) / (1024.0 * 1024.0);
            }
            dups.groups.retain(|grp| grp.files.len() >= 2);
            dups.total_groups = dups.groups.len();
            dups.total_duplicate_files = dups
                .groups
                .iter()
                .map(|g| g.count.saturating_sub(1))
                .sum();
            dups.total_wasted_bytes = dups.groups.iter().map(|g| g.wasted_bytes).sum();
            dups.total_wasted_mb = (dups.total_wasted_bytes as f64) / (1024.0 * 1024.0);
        }

        // 3. Update System Junk & Trash
        if let Some(ref mut junk) = guard.junk_result {
            junk.items.retain(|it| !deleted_set.contains(it.path.as_str()));
            junk.total_size_bytes = junk.items.iter().map(|it| it.size_bytes).sum();
            junk.total_size_mb = (junk.total_size_bytes as f64) / (1024.0 * 1024.0);
            if emptied_trash {
                junk.trash.item_count = 0;
                junk.trash.total_size_bytes = 0;
                junk.trash.total_size_mb = 0.0;
                junk.trash.items.clear();
            }
        }
    }

    if !emptied_trash {
        std::thread::spawn(|| {
            let trash = get_trash_status();
            if let Ok(mut guard) = status_mutex().lock() {
                if let Some(ref mut junk) = guard.junk_result {
                    junk.trash = trash;
                }
            }
        });
    }

    get_cleaner_scan_status()
}

/// Immediately remove an uninstalled application from the cached `apps_result` without rescanning.
pub fn remove_uninstalled_app_from_cached_scan(
    bundle_id: &str,
    app_path: &str,
    emptied_trash: bool,
) -> CleanerScanStatus {
    if let Ok(mut guard) = status_mutex().lock() {
        if let Some(ref mut apps) = guard.apps_result {
            apps.retain(|a| a.bundle_id != bundle_id && a.app_path != app_path);
        }
        if emptied_trash {
            if let Some(ref mut junk) = guard.junk_result {
                junk.trash.item_count = 0;
                junk.trash.total_size_bytes = 0;
                junk.trash.total_size_mb = 0.0;
                junk.trash.items.clear();
            }
        }
    }

    if !emptied_trash {
        std::thread::spawn(|| {
            let trash = get_trash_status();
            if let Ok(mut guard) = status_mutex().lock() {
                if let Some(ref mut junk) = guard.junk_result {
                    junk.trash = trash;
                }
            }
        });
    }

    get_cleaner_scan_status()
}

/// Immediately filter cached duplicate groups against the current exclude list in-place.
pub fn filter_duplicates_by_excludes_in_cached_scan() -> CleanerScanStatus {
    let excludes = get_duplicate_exclude_list();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    if let Ok(mut guard) = status_mutex().lock() {
        if let Some(ref mut dups) = guard.duplicates_result {
            dups.excluded_dirs = excludes.clone();
            for grp in &mut dups.groups {
                grp.files.retain(|f| {
                    let mut cur = Path::new(&f.path).parent();
                    while let Some(dir) = cur {
                        if should_skip_duplicate_dir(dir, &excludes, &home) {
                            return false;
                        }
                        cur = dir.parent();
                    }
                    true
                });
                if !grp.files.is_empty() && !grp.files.iter().any(|f| f.is_original) {
                    grp.files[0].is_original = true;
                }
                grp.count = grp.files.len();
                grp.wasted_bytes = grp
                    .file_size_bytes
                    .saturating_mul(grp.count.saturating_sub(1) as u64);
                grp.wasted_mb = (grp.wasted_bytes as f64) / (1024.0 * 1024.0);
            }
            dups.groups.retain(|grp| grp.files.len() >= 2);
            dups.total_groups = dups.groups.len();
            dups.total_duplicate_files = dups
                .groups
                .iter()
                .map(|g| g.count.saturating_sub(1))
                .sum();
            dups.total_wasted_bytes = dups.groups.iter().map(|g| g.wasted_bytes).sum();
            dups.total_wasted_mb = (dups.total_wasted_bytes as f64) / (1024.0 * 1024.0);
        }
    }
    get_cleaner_scan_status()
}


