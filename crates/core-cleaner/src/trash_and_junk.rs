//! System Junk scanner (`~/Library/Caches`, `~/Library/Logs`, Xcode/Cargo/npm caches) and macOS Trash (`~/.Trash`) manager.

use crate::models::{JunkItem, JunkScanSummary, TrashItem, TrashStatus};
use crate::scan_manager::{invalidate_cleaner_cache, update_scan_progress};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::UNIX_EPOCH;

pub fn compute_dir_size_fast(path: &Path, max_depth: u32, max_entries: &mut usize) -> u64 {
    if max_depth == 0 || *max_entries == 0 {
        return 0;
    }
    *max_entries = max_entries.saturating_sub(1);
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return 0;
        };
        let mut sum = 0u64;
        for entry in entries.flatten() {
            if *max_entries == 0 {
                break;
            }
            sum += compute_dir_size_fast(&entry.path(), max_depth - 1, max_entries);
        }
        return sum;
    }
    0
}

pub fn compute_dir_size(path: &Path, max_depth: u32) -> u64 {
    let mut budget = 1800usize;
    compute_dir_size_fast(path, max_depth.min(3), &mut budget)
}

pub fn get_trash_status() -> TrashStatus {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let trash_dir = PathBuf::from(&home).join(".Trash");
    let _ = fs::create_dir_all(&trash_dir);

    let mut entries_vec = Vec::new();
    if let Ok(entries) = fs::read_dir(&trash_dir) {
        for e in entries.flatten() {
            let p = e.path();
            let fname = e.file_name().to_string_lossy().to_string();
            if fname == ".DS_Store" || fname == ".localized" {
                continue;
            }
            entries_vec.push((fname, p));
        }
    }

    let mut items: Vec<TrashItem> = entries_vec
        .into_par_iter()
        .map(|(name, p)| {
            let is_dir = p.is_dir();
            let size_bytes = compute_dir_size(&p, 5);
            let size_mb = ((size_bytes as f64 / (1024.0 * 1024.0)) * 100.0).round() / 100.0;
            TrashItem {
                name,
                path: p.to_string_lossy().to_string(),
                is_dir,
                size_bytes,
                size_mb,
            }
        })
        .collect();

    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    let item_count = items.len();
    let total_size_bytes: u64 = items.iter().map(|i| i.size_bytes).sum();
    let total_size_mb = ((total_size_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0;
    items.truncate(100);

    TrashStatus {
        trash_path: trash_dir.to_string_lossy().to_string(),
        item_count,
        total_size_bytes,
        total_size_mb,
        items,
    }
}

/// Permanently empty macOS Trash (`~/.Trash`).
pub fn empty_macos_trash() -> Result<String, String> {
    let before = get_trash_status();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let trash_dir = PathBuf::from(&home).join(".Trash");

    let mut removed_count = 0usize;
    if let Ok(entries) = fs::read_dir(&trash_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname == ".localized" {
                continue;
            }
            let res = if p.is_dir() {
                fs::remove_dir_all(&p)
            } else {
                fs::remove_file(&p)
            };
            if res.is_ok() && fname != ".DS_Store" {
                removed_count += 1;
            }
        }
    }

    let after = get_trash_status();
    if after.item_count > 0 {
        let _ = Command::new("osascript")
            .args(["-e", "tell application \"Finder\" to empty trash"])
            .status();
    }

    invalidate_cleaner_cache();
    let count = removed_count.max(before.item_count);
    Ok(format!(
        "Đã dọn sạch Thùng rác ({} mục, giải phóng {:.1} MB vĩnh viễn).",
        count, before.total_size_mb
    ))
}

pub fn scan_system_junk() -> JunkScanSummary {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_path = PathBuf::from(&home);

    let mut targets: Vec<(String, PathBuf)> = Vec::new();

    if let Ok(entries) = fs::read_dir(home_path.join("Library/Caches")) {
        for e in entries.flatten() {
            targets.push(("User App Cache".to_string(), e.path()));
        }
    }

    if let Ok(entries) = fs::read_dir(home_path.join("Library/Logs")) {
        for e in entries.flatten() {
            targets.push(("App & System Logs".to_string(), e.path()));
        }
    }

    let dev_dirs = [
        (
            "Xcode DerivedData",
            home_path.join("Library/Developer/Xcode/DerivedData"),
        ),
        ("Cargo Cache", home_path.join(".cargo/registry/cache")),
        ("npm Cache", home_path.join(".npm/_cacache")),
        ("Trash (Thùng rác)", home_path.join(".Trash")),
    ];
    for (label, p) in dev_dirs {
        if p.exists() {
            targets.push((label.to_string(), p));
        }
    }

    let total_targets = targets.len().max(1);
    let done_counter = AtomicUsize::new(0);

    let mut items: Vec<JunkItem> = targets
        .into_par_iter()
        .filter_map(|(category, path)| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let done = done_counter.fetch_add(1, Ordering::Relaxed) + 1;
            let pct = ((done * 92) / total_targets).clamp(5, 95) as u8;
            update_scan_progress("junk", true, pct, &format!("Đang quét: {}", name));

            let size = compute_dir_size(&path, 5);
            if size < 100 * 1024 {
                return None;
            }
            Some(JunkItem {
                name,
                path: path.to_string_lossy().to_string(),
                category,
                size_bytes: size,
                size_mb: ((size as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
            })
        })
        .collect();

    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    items.truncate(80);

    let trash = get_trash_status();
    let total_size_bytes: u64 = items.iter().map(|i| i.size_bytes).sum();
    let total_size_mb = ((total_size_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0;

    JunkScanSummary {
        total_size_bytes,
        total_size_mb,
        items,
        trash,
    }
}

fn is_protected_system_or_root_path(path: &Path, home: &str) -> bool {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim_end_matches('/');
    if trimmed.is_empty() || raw.contains("..") {
        return true;
    }
    const SYSTEM_ROOTS: &[&str] = &[
        "/",
        "/System",
        "/Library",
        "/Applications",
        "/Users",
        "/bin",
        "/sbin",
        "/usr",
        "/var",
        "/private",
        "/etc",
        "/tmp",
        "/dev",
        "/Volumes",
        "/cores",
    ];
    if SYSTEM_ROOTS.iter().any(|&r| trimmed.eq_ignore_ascii_case(r))
        || trimmed.starts_with("/System/")
        || trimmed.starts_with("/usr/")
        || trimmed.starts_with("/bin/")
        || trimmed.starts_with("/sbin/")
    {
        return true;
    }
    let home_trimmed = home.trim_end_matches('/');
    if !home_trimmed.is_empty() {
        let protected_home_dirs = [
            home_trimmed.to_string(),
            format!("{}/Library", home_trimmed),
            format!("{}/Desktop", home_trimmed),
            format!("{}/Documents", home_trimmed),
            format!("{}/Downloads", home_trimmed),
            format!("{}/Pictures", home_trimmed),
            format!("{}/Movies", home_trimmed),
            format!("{}/Music", home_trimmed),
        ];
        if protected_home_dirs
            .iter()
            .any(|d| trimmed.eq_ignore_ascii_case(d))
        {
            return true;
        }
    }
    false
}

/// Safely move a file or folder into `~/.Trash` (or delete if already inside `~/.Trash`).
pub fn move_path_to_trash(target_path: &str) -> Result<u64, String> {
    let path = PathBuf::from(target_path);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    if is_protected_system_or_root_path(&path, &home) {
        return Err(format!(
            "Từ chối xóa đường dẫn hệ thống được bảo vệ: {}",
            target_path
        ));
    }
    if !path.exists() {
        return Err(format!("Đường dẫn không tồn tại: {}", target_path));
    }

    let size = compute_dir_size(&path, 6);
    let trash_dir = PathBuf::from(&home).join(".Trash");
    let _ = fs::create_dir_all(&trash_dir);

    if path.starts_with(&trash_dir) {
        if path == trash_dir {
            if let Ok(entries) = fs::read_dir(&trash_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let _ = fs::remove_dir_all(&p);
                    } else {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
            return Ok(size);
        } else if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(size);
    }

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "removed_item".to_string());

    let ts = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut dest = trash_dir.join(&file_name);
    if dest.exists() {
        dest = trash_dir.join(format!("{}_{}", ts, file_name));
    }

    fs::rename(&path, &dest)
        .map_err(|e| format!("Không thể chuyển '{}' vào Trash: {}", target_path, e))?;
    Ok(size)
}

pub fn clean_selected_paths(paths: &[String]) -> Result<String, String> {
    clean_selected_paths_ext(paths, false)
}

pub fn clean_selected_paths_ext(
    paths: &[String],
    also_empty_trash: bool,
) -> Result<String, String> {
    let _ = crate::uninstaller::stop_processes_and_agents_for_paths(paths);
    let mut freed_bytes = 0u64;
    let mut count = 0usize;
    for p in paths {
        if let Ok(sz) = move_path_to_trash(p) {
            freed_bytes += sz;
            count += 1;
        }
    }
    let mb = ((freed_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0;
    if also_empty_trash {
        let _ = empty_macos_trash();
        invalidate_cleaner_cache();
        return Ok(format!(
            "Đã dọn {} mục ({:.1} MB) và làm trống sạch Thùng rác vĩnh viễn!",
            count, mb
        ));
    }
    invalidate_cleaner_cache();
    Ok(format!(
        "Đã dọn dẹp {} mục (Giải phóng {:.1} MB vào Thùng rác).",
        count, mb
    ))
}

