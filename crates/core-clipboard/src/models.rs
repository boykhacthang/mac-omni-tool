//! Data models for 1,000-item SQLite Clipboard Manager & Pagination.

use serde::{Deserialize, Serialize};

pub const MAX_CLIPBOARD_ITEMS: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardItem {
    pub id: i64,
    pub kind: String, // "text", "image", "file"
    pub preview_text: String,
    pub full_text: String,
    pub image_path: String,
    pub image_data_url: String,
    pub byte_size: i64,
    pub content_hash: String,
    pub source_app: String,
    pub is_pinned: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardStats {
    pub enabled: bool,
    pub total_items: usize,
    pub pinned_items: usize,
    pub image_items: usize,
    pub max_capacity: usize,
    pub disk_usage_bytes: u64,
    pub polling_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardPageResult {
    pub items: Vec<ClipboardItem>,
    pub page: usize,
    pub page_size: usize,
    pub total_matching: usize,
    pub total_pages: usize,
}

