//! Multi-threaded Large Files scanner (`Downloads`, `Documents`, `Desktop`, `Movies`, `Pictures`) and Finder reveal helper.

use crate::models::LargeFileEntry;
use crate::scan_manager::update_scan_progress;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::UNIX_EPOCH;

pub fn classify_file_kind(path: &Path) -> String {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => "Video".to_string(),
        "dmg" | "iso" | "pkg" | "ipsw" => "Disk Image (.dmg/.iso)".to_string(),
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "xz" => "Archive (.zip/.tar)".to_string(),
        "sqlite" | "db" | "vmdk" | "qcow2" | "bin" | "gguf" | "safetensors" => {
            "Data / Model / VM".to_string()
        }
        _ => "Other".to_string(),
    }
}

fn collect_large_files_recursive(
    dir: &Path,
    min_bytes: u64,
    depth: u32,
    out: &mut Vec<LargeFileEntry>,
) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let p = entry.path();
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();
        if fname_str == ".Trash" || fname_str == "node_modules" || fname_str == ".git" {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.is_file() {
            let sz = meta.len();
            if sz >= min_bytes {
                let modified_unix = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                out.push(LargeFileEntry {
                    name: fname_str.to_string(),
                    path: p.to_string_lossy().to_string(),
                    kind: classify_file_kind(&p),
                    size_bytes: sz,
                    size_mb: ((sz as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
                    modified_unix,
                });
            }
        } else if meta.is_dir() {
            collect_large_files_recursive(&p, min_bytes, depth - 1, out);
        }
    }
}

/// Scan user directories in parallel for files >= `min_size_mb` MB.
pub fn scan_large_files(scope: &str, min_size_mb: u64) -> Vec<LargeFileEntry> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_path = PathBuf::from(&home);
    let min_bytes = min_size_mb.max(10) * 1024 * 1024;

    let roots: Vec<PathBuf> = match scope {
        "downloads" => vec![home_path.join("Downloads")],
        "documents" => vec![home_path.join("Documents")],
        "desktop" => vec![home_path.join("Desktop")],
        "movies" => vec![home_path.join("Movies")],
        _ => vec![
            home_path.join("Downloads"),
            home_path.join("Documents"),
            home_path.join("Desktop"),
            home_path.join("Movies"),
            home_path.join("Pictures"),
            home_path.join("Library/Caches"),
        ],
    };

    let total_roots = roots.len().max(1);
    let done_counter = AtomicUsize::new(0);

    let mut results: Vec<LargeFileEntry> = roots
        .into_par_iter()
        .flat_map(|root| {
            let root_name = root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| root.to_string_lossy().to_string());
            update_scan_progress(
                "large",
                true,
                25,
                &format!("Đang quét thư mục ~/{} ...", root_name),
            );
            let mut local = Vec::new();
            collect_large_files_recursive(&root, min_bytes, 6, &mut local);
            let done = done_counter.fetch_add(1, Ordering::Relaxed) + 1;
            let pct = ((done * 92) / total_roots).clamp(15, 95) as u8;
            update_scan_progress("large", true, pct, &format!("Đã quét xong ~/{}", root_name));
            local
        })
        .collect();

    results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    results.truncate(100);
    results
}

/// Reveal a file or folder in macOS Finder (`open -R`).
pub fn reveal_in_finder(path: &str) -> Result<(), String> {
    Command::new("open")
        .args(["-R", path])
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

