//! Data models for Autostart, System Junk, Trash, App Uninstaller, Large Files, Duplicate Files, and Async Scan Progress.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutostartConfig {
    pub enabled: bool,
    pub start_minimized: bool,
    pub plist_path: String,
    pub exe_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JunkItem {
    pub name: String,
    pub path: String,
    pub category: String,
    pub size_bytes: u64,
    pub size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TrashItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TrashStatus {
    pub trash_path: String,
    pub item_count: usize,
    pub total_size_bytes: u64,
    pub total_size_mb: f64,
    pub items: Vec<TrashItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct JunkScanSummary {
    pub total_size_bytes: u64,
    pub total_size_mb: f64,
    pub items: Vec<JunkItem>,
    #[serde(default)]
    pub trash: TrashStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppLeftoverFile {
    pub kind: String, // "App Bundle (.app)", "Application Support", "Caches", "Preferences", "Containers", "Saved State", "Logs"
    pub path: String,
    pub size_bytes: u64,
    pub size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledAppInfo {
    pub name: String,
    pub bundle_id: String,
    pub version: String,
    pub app_path: String,
    pub icon_data_url: String,
    pub app_size_bytes: u64,
    pub app_size_mb: f64,
    pub leftovers_size_bytes: u64,
    pub total_size_mb: f64,
    #[serde(default)]
    pub running_pids: Vec<u32>,
    pub related_files: Vec<AppLeftoverFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrphanedBackgroundItem {
    pub kind: String, // "Running Process" | "LaunchAgent"
    pub name: String,
    pub pid: Option<u32>,
    pub target_path: String,
    pub plist_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LargeFileEntry {
    pub name: String,
    pub path: String,
    pub kind: String, // "Video", "Disk Image (.dmg/.iso)", "Archive (.zip/.tar)", "Data / Model / VM", "Other"
    pub size_bytes: u64,
    pub size_mb: f64,
    pub modified_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateFileEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_mb: f64,
    pub modified_unix: u64,
    pub is_original: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateGroup {
    pub group_id: usize,
    pub hash: String,
    pub representative_name: String,
    pub kind: String,
    pub file_size_bytes: u64,
    pub file_size_mb: f64,
    pub count: usize,
    pub wasted_bytes: u64,
    pub wasted_mb: f64,
    pub files: Vec<DuplicateFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DuplicateScanSummary {
    pub total_groups: usize,
    pub total_duplicate_files: usize,
    pub total_wasted_bytes: u64,
    pub total_wasted_mb: f64,
    pub groups: Vec<DuplicateGroup>,
    #[serde(default)]
    pub excluded_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateExcludeConfig {
    pub excluded_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanerScanStatus {
    pub running: bool,
    pub kind: String,
    pub percent: u8,
    pub message: String,
    pub junk_result: Option<JunkScanSummary>,
    pub apps_result: Option<Vec<InstalledAppInfo>>,
    pub large_result: Option<Vec<LargeFileEntry>>,
    pub large_scope: String,
    pub large_min_mb: u64,
    pub duplicates_result: Option<DuplicateScanSummary>,
    pub duplicates_scope: String,
    pub duplicates_min_kb: u64,
}

