//! 3-stage parallel Duplicate Files scanner (`Size` -> `8KB Edge Hash` -> `Full 128-bit Content Hash`)
//! with automatic internal DB engine file exclusion while checking normal `.sql` files.

use crate::models::{
    DuplicateExcludeConfig, DuplicateFileEntry, DuplicateGroup, DuplicateScanSummary,
};
use crate::scan_manager::update_scan_progress;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn default_duplicate_excludes() -> Vec<String> {
    vec![
        "node_modules".to_string(),
        "target".to_string(),
        "vendor".to_string(),
        "venv".to_string(),
        ".venv".to_string(),
        "Library".to_string(),
        "mysql".to_string(),
        "mariadb".to_string(),
        "postgres".to_string(),
        "postgresql".to_string(),
        "pg_wal".to_string(),
        "pg_xlog".to_string(),
        "pg_data".to_string(),
        "mongodb".to_string(),
        "redis".to_string(),
    ]
}

fn duplicate_exclude_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home).join("Library/Application Support/MacOmniTool");
    let _ = fs::create_dir_all(&dir);
    dir.join("duplicate_excludes.json")
}

pub fn get_duplicate_exclude_list() -> Vec<String> {
    let path = duplicate_exclude_config_path();
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<DuplicateExcludeConfig>(&raw) {
            if !cfg.excluded_dirs.is_empty() {
                return cfg.excluded_dirs;
            }
        }
    }
    default_duplicate_excludes()
}

fn save_duplicate_exclude_list(list: &[String]) {
    let path = duplicate_exclude_config_path();
    let cfg = DuplicateExcludeConfig {
        excluded_dirs: list.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(path, json);
    }
}

pub fn add_duplicate_exclude(pattern: &str) -> Vec<String> {
    let trimmed = pattern.trim().trim_end_matches('/').to_string();
    let mut list = get_duplicate_exclude_list();
    if !trimmed.is_empty()
        && !list
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&trimmed))
    {
        list.push(trimmed);
        save_duplicate_exclude_list(&list);
    }
    list
}

pub fn remove_duplicate_exclude(pattern: &str) -> Vec<String> {
    let trimmed = pattern.trim().trim_end_matches('/');
    let mut list = get_duplicate_exclude_list();
    list.retain(|item| !item.eq_ignore_ascii_case(trimmed));
    save_duplicate_exclude_list(&list);
    list
}

pub fn reset_duplicate_excludes() -> Vec<String> {
    let list = default_duplicate_excludes();
    save_duplicate_exclude_list(&list);
    list
}

/// Detect internal SQL / NoSQL database engine storage files that must NEVER be deleted as duplicates.
/// Normal `.sql` script/dump files (`*.sql`) return `false` so they are checked normally!
pub fn is_internal_db_engine_file(path: &Path) -> bool {
    let fname_lower = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if matches!(
        fname_lower.as_str(),
        "ibdata1"
            | "ib_logfile0"
            | "ib_logfile1"
            | "ibtmp1"
            | "auto.cnf"
            | "pg_version"
            | "pg_control"
            | "pg_filenode.map"
            | "pg_internal.init"
            | "postmaster.pid"
            | "postmaster.opts"
            | "wiredtiger"
            | "wiredtiger.wt"
            | "wiredtiger.turtle"
            | "wiredtiger.lock"
            | "mongod.lock"
            | "dump.rdb"
            | "appendonly.aof"
    ) || fname_lower.starts_with("ib_logfile")
        || fname_lower.starts_with("mysql-bin.")
        || fname_lower.starts_with("binlog.")
    {
        return true;
    }

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Normal .sql files are explicitly allowed!
    if ext == "sql" {
        return false;
    }

    matches!(
        ext.as_str(),
        "ibd"
            | "frm"
            | "myd"
            | "myi"
            | "opt"
            | "par"
            | "trn"
            | "trg"
            | "isl"
            | "csm"
            | "db-wal"
            | "db-shm"
            | "db-journal"
            | "sqlite-wal"
            | "sqlite-shm"
            | "sqlite-journal"
            | "wt"
            | "ldb"
            | "sst"
    )
}

fn is_internal_db_data_dir(dir: &Path) -> bool {
    dir.join("PG_VERSION").exists()
        || dir.join("ibdata1").exists()
        || dir.join("WiredTiger").exists()
        || dir.join("mysql_upgrade_info").exists()
}

/// Check if a directory matches any entry in the user's exclude list (by folder name or path prefix).
pub fn should_skip_duplicate_dir(dir: &Path, excludes: &[String], home: &str) -> bool {
    let fname_lower = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if fname_lower.starts_with('.') {
        return true;
    }

    let full_str = dir.to_string_lossy();
    let full_lower = full_str.to_lowercase();
    let tilde_lower = if !home.is_empty() && full_str.starts_with(home) {
        format!("~{}", &full_str[home.len()..]).to_lowercase()
    } else {
        String::new()
    };

    for raw_pat in excludes {
        let pat = raw_pat.trim().trim_end_matches('/').to_lowercase();
        if pat.is_empty() {
            continue;
        }
        if pat.contains('/') {
            let expanded = if pat.starts_with("~/") && !home.is_empty() {
                format!("{}{}", home.to_lowercase(), &pat[1..])
            } else {
                pat.clone()
            };
            if full_lower == expanded
                || full_lower.starts_with(&format!("{}/", expanded))
                || (!tilde_lower.is_empty()
                    && (tilde_lower == pat || tilde_lower.starts_with(&format!("{}/", pat))))
            {
                return true;
            }
        } else if fname_lower == pat {
            return true;
        }
    }

    is_internal_db_data_dir(dir)
}

pub fn classify_duplicate_kind(path: &Path) -> String {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "sql" => "File SQL (.sql)".to_string(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "svg" | "bmp" | "tiff" => {
            "Hình ảnh".to_string()
        }
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => "Video".to_string(),
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" => "Âm thanh".to_string(),
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "csv" => {
            "Tài liệu".to_string()
        }
        "dmg" | "iso" | "pkg" | "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" => {
            "Nén / Bộ cài".to_string()
        }
        _ => "Khác".to_string(),
    }
}

fn collect_candidate_files_recursive(
    dir: &Path,
    min_bytes: u64,
    depth: u32,
    excludes: &[String],
    home: &str,
    out: &mut Vec<(PathBuf, u64, u64)>,
) {
    if depth == 0 || out.len() >= 35_000 {
        return;
    }
    if should_skip_duplicate_dir(dir, excludes, home) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if out.len() >= 35_000 {
            break;
        }
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();
        if fname_str.starts_with('.') {
            continue;
        }
        let p = entry.path();
        let Ok(meta) = fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_file() {
            if is_internal_db_engine_file(&p) {
                continue;
            }
            let sz = meta.len();
            if sz >= min_bytes {
                let modified_unix = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                out.push((p, sz, modified_unix));
            }
        } else if meta.is_dir() && !should_skip_duplicate_dir(&p, excludes, home) {
            collect_candidate_files_recursive(&p, min_bytes, depth - 1, excludes, home, out);
        }
    }
}

/// Compute a fast 128-bit edge hash from the first 4KB and last 4KB of a file.
pub fn compute_edge_hash_128(path: &Path, size: u64) -> Option<(u64, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = fs::File::open(path).ok()?;
    let mut buf = [0u8; 4096];

    let mut h1: u64 = 0xcbf2_9ce4_8422_2325 ^ size;
    let mut h2: u64 = 0x8422_2325_cbf2_9ce4 ^ size.rotate_left(17);

    let n1 = file.read(&mut buf).ok()?;
    for &b in &buf[..n1] {
        h1 ^= b as u64;
        h1 = h1.wrapping_mul(0x0000_0100_0000_01B3);
        h2 = h2.wrapping_add(b as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }

    if size > 4096 {
        let tail_offset = size.saturating_sub(4096);
        if file.seek(SeekFrom::Start(tail_offset)).is_ok() {
            if let Ok(n2) = file.read(&mut buf) {
                for &b in &buf[..n2] {
                    h1 ^= (b as u64).rotate_left(7);
                    h1 = h1.wrapping_mul(0x0000_0100_0000_01B3);
                    h2 ^= b as u64;
                    h2 = h2.wrapping_mul(0xBF58_476D_1CE4_E5B9);
                }
            }
        }
    }

    Some((h1, h2))
}

/// Compute a full 128-bit content hash across the entire file.
pub fn compute_full_hash_128(path: &Path, size: u64) -> Option<(u64, u64)> {
    use std::io::Read;
    let mut file = fs::File::open(path).ok()?;
    let mut buf = vec![0u8; 65536];

    let mut h1: u64 = 0xcbf2_9ce4_8422_2325 ^ size;
    let mut h2: u64 = 0x9E37_79B9_7F4A_7C15 ^ size.rotate_left(23);

    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            h1 ^= b as u64;
            h1 = h1.wrapping_mul(0x0000_0100_0000_01B3);
            h2 = (h2 ^ (b as u64)).rotate_left(5).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        }
    }

    Some((h1, h2))
}

/// 3-stage parallel duplicate file scanner (`Size` -> `Edge 8KB Hash` -> `Full Content Hash`).
pub fn scan_duplicate_files(scope: &str, min_size_kb: u64) -> DuplicateScanSummary {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_path = PathBuf::from(&home);

    let roots: Vec<PathBuf> = match scope {
        "downloads" => vec![home_path.join("Downloads")],
        "documents" => vec![home_path.join("Documents")],
        "desktop" => vec![home_path.join("Desktop")],
        "pictures" => vec![home_path.join("Pictures")],
        "movies" => vec![home_path.join("Movies")],
        _ => vec![
            home_path.join("Downloads"),
            home_path.join("Documents"),
            home_path.join("Desktop"),
            home_path.join("Pictures"),
            home_path.join("Movies"),
        ],
    };

    let excludes = get_duplicate_exclude_list();
    scan_duplicate_files_in_roots(&roots, min_size_kb, &excludes)
}

pub fn scan_duplicate_files_in_roots(
    roots: &[PathBuf],
    min_size_kb: u64,
    excludes: &[String],
) -> DuplicateScanSummary {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let min_bytes = min_size_kb.max(1) * 1024;

    update_scan_progress(
        "duplicates",
        true,
        15,
        "Bước 1/3: Đang thu thập danh sách tệp theo kích thước...",
    );

    let all_files: Vec<(PathBuf, u64, u64)> = roots
        .par_iter()
        .flat_map(|root| {
            let mut local = Vec::new();
            collect_candidate_files_recursive(root, min_bytes, 6, excludes, &home, &mut local);
            local
        })
        .collect();

    let mut size_groups: HashMap<u64, Vec<(PathBuf, u64)>> = HashMap::new();
    for (p, sz, mtime) in all_files {
        size_groups.entry(sz).or_default().push((p, mtime));
    }

    let size_candidates: Vec<(u64, PathBuf, u64)> = size_groups
        .into_iter()
        .filter(|(_, list)| list.len() >= 2)
        .flat_map(|(sz, list)| list.into_iter().map(move |(p, mtime)| (sz, p, mtime)))
        .collect();

    if size_candidates.is_empty() {
        return DuplicateScanSummary {
            excluded_dirs: excludes.to_vec(),
            ..DuplicateScanSummary::default()
        };
    }

    update_scan_progress(
        "duplicates",
        true,
        48,
        &format!(
            "Bước 2/3: Đang so khớp nhanh đầu/cuối {} tệp cùng dung lượng...",
            size_candidates.len()
        ),
    );

    let edge_hashed: Vec<((u64, u64, u64), (PathBuf, u64))> = size_candidates
        .into_par_iter()
        .filter_map(|(sz, p, mtime)| {
            let (h1, h2) = compute_edge_hash_128(&p, sz)?;
            Some(((sz, h1, h2), (p, mtime)))
        })
        .collect();

    let mut edge_groups: HashMap<(u64, u64, u64), Vec<(PathBuf, u64)>> = HashMap::new();
    for (key, val) in edge_hashed {
        edge_groups.entry(key).or_default().push(val);
    }

    let final_candidates: Vec<(u64, PathBuf, u64)> = edge_groups
        .into_iter()
        .filter(|(_, list)| list.len() >= 2)
        .flat_map(|((sz, _, _), list)| list.into_iter().map(move |(p, mtime)| (sz, p, mtime)))
        .collect();

    if final_candidates.is_empty() {
        return DuplicateScanSummary {
            excluded_dirs: excludes.to_vec(),
            ..DuplicateScanSummary::default()
        };
    }

    update_scan_progress(
        "duplicates",
        true,
        78,
        &format!(
            "Bước 3/3: Đang băm kiểm tra 100% nội dung {} tệp nghi trùng...",
            final_candidates.len()
        ),
    );

    let full_hashed: Vec<((u64, u64, u64), (PathBuf, u64))> = final_candidates
        .into_par_iter()
        .filter_map(|(sz, p, mtime)| {
            let (h1, h2) = compute_full_hash_128(&p, sz)?;
            Some(((sz, h1, h2), (p, mtime)))
        })
        .collect();

    let mut full_groups: HashMap<(u64, u64, u64), Vec<(PathBuf, u64)>> = HashMap::new();
    for (key, val) in full_hashed {
        full_groups.entry(key).or_default().push(val);
    }

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    for ((sz, h1, h2), mut list) in full_groups {
        if list.len() < 2 {
            continue;
        }
        list.sort_by(|(pa, ma), (pb, mb)| {
            ma.cmp(mb)
                .then_with(|| pa.as_os_str().len().cmp(&pb.as_os_str().len()))
                .then_with(|| pa.cmp(pb))
        });

        let first_path = &list[0].0;
        let representative_name = first_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let kind = classify_duplicate_kind(first_path);
        let file_size_mb = ((sz as f64 / (1024.0 * 1024.0)) * 100.0).round() / 100.0;
        let count = list.len();
        let wasted_bytes = sz * (count as u64 - 1);
        let wasted_mb = ((wasted_bytes as f64 / (1024.0 * 1024.0)) * 100.0).round() / 100.0;

        let files: Vec<DuplicateFileEntry> = list
            .into_iter()
            .enumerate()
            .map(|(idx, (p, mtime))| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                DuplicateFileEntry {
                    name,
                    path: p.to_string_lossy().to_string(),
                    size_bytes: sz,
                    size_mb: file_size_mb,
                    modified_unix: mtime,
                    is_original: idx == 0,
                }
            })
            .collect();

        groups.push(DuplicateGroup {
            group_id: 0,
            hash: format!("{:016x}{:016x}", h1, h2),
            representative_name,
            kind,
            file_size_bytes: sz,
            file_size_mb,
            count,
            wasted_bytes,
            wasted_mb,
            files,
        });
    }

    groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    groups.truncate(100);
    for (idx, g) in groups.iter_mut().enumerate() {
        g.group_id = idx + 1;
    }

    let total_groups = groups.len();
    let total_duplicate_files: usize = groups.iter().map(|g| g.count.saturating_sub(1)).sum();
    let total_wasted_bytes: u64 = groups.iter().map(|g| g.wasted_bytes).sum();
    let total_wasted_mb = ((total_wasted_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0;

    DuplicateScanSummary {
        total_groups,
        total_duplicate_files,
        total_wasted_bytes,
        total_wasted_mb,
        groups,
        excluded_dirs: excludes.to_vec(),
    }
}

