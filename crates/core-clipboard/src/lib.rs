//! `core-clipboard`: Zero-leak 250ms `NSPasteboard` watcher, 1,000-item SQLite WAL ring-buffer,
//! image deduplication, Base64 codec, and simulated `Cmd+V` pasting.

pub mod codec;
pub mod ffi;
pub mod models;
pub mod pasteboard;
pub mod storage;

pub use codec::{base64_encode, decode_base64_png, fnv1a_hash, now_unix_ms};
pub use models::{ClipboardItem, ClipboardPageResult, ClipboardStats, MAX_CLIPBOARD_ITEMS};
pub use pasteboard::{
    activate_clipboard_item, copy_png_bytes_to_clipboard, copy_text_to_clipboard,
    poll_pasteboard_once, save_png_bytes_to_file, start_clipboard_watcher,
};
pub use storage::{
    app_data_dir, clear_unpinned_clipboard_items, db_path, delete_clipboard_item,
    get_clipboard_stats, images_dir, is_clipboard_enabled, query_clipboard_items,
    query_clipboard_page, record_clipboard_entry, set_clipboard_enabled, toggle_pin_clipboard_item,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_hash_deterministic() {
        let h1 = fnv1a_hash(b"hello world");
        let h2 = fnv1a_hash(b"hello world");
        let h3 = fnv1a_hash(b"hello world!");
        assert_eq!(h1.len(), 16);
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_base64_encode_and_decode_roundtrip() {
        let raw = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4, 5];
        let encoded = base64_encode(&raw);
        let data_url = format!("data:image/png;base64,{}", encoded);
        let decoded = decode_base64_png(&data_url).expect("Should decode valid base64 PNG");
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_decode_base64_invalid_errors() {
        assert!(decode_base64_png("").is_err());
        assert!(decode_base64_png("data:image/png;base64,@@@invalid@@@").is_err());
    }

    #[test]
    fn test_clipboard_sqlite_crud_and_pagination() {
        let unique_hash = format!("unit_test_hash_{}", now_unix_ms());
        record_clipboard_entry(
            "text",
            "Nội dung kiểm thử Unit Test Clipboard",
            "",
            37,
            &unique_hash,
            "UnitTest",
        );

        let page = query_clipboard_page("Kiểm thử Unit Test", "text", 1, 50);
        assert!(page.total_matching >= 1);
        let found = page
            .items
            .iter()
            .find(|i| i.content_hash == unique_hash)
            .expect("Inserted item should be found");

        // Toggle pin on -> off
        assert!(!found.is_pinned);
        assert!(toggle_pin_clipboard_item(found.id));
        let pinned_page = query_clipboard_page("", "pinned", 1, 50);
        assert!(pinned_page.items.iter().any(|i| i.id == found.id));

        // Clean up test entry
        assert!(delete_clipboard_item(found.id));
    }
}
