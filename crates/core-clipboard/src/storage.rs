//! SQLite WAL persistence layer for the 1,000-item ring-buffer Clipboard Manager.

use crate::codec::{base64_encode, now_unix_ms};
use crate::ffi::*;
use crate::models::{ClipboardItem, ClipboardPageResult, ClipboardStats, MAX_CLIPBOARD_ITEMS};
use std::ffi::{c_char, c_int, CStr, CString};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static CLIPBOARD_ENABLED: AtomicBool = AtomicBool::new(true);
static DB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn is_clipboard_enabled() -> bool {
    CLIPBOARD_ENABLED.load(Ordering::Relaxed)
}

pub fn set_clipboard_enabled(enabled: bool) {
    let was_enabled = CLIPBOARD_ENABLED.swap(enabled, Ordering::SeqCst);
    if enabled && !was_enabled {
        crate::pasteboard::sync_pasteboard_change_count();
    }
}

fn db_mutex() -> &'static Mutex<()> {
    DB_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn app_data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("MacOmniTool");
    let _ = fs::create_dir_all(dir.join("clipboard_images"));
    dir
}

pub fn db_path() -> PathBuf {
    app_data_dir().join("clipboard_history.sqlite3")
}

pub fn images_dir() -> PathBuf {
    app_data_dir().join("clipboard_images")
}

unsafe fn open_db() -> Option<*mut Sqlite3> {
    let c_path = CString::new(db_path().to_string_lossy().as_bytes()).ok()?;
    let mut db: *mut Sqlite3 = std::ptr::null_mut();
    if sqlite3_open(c_path.as_ptr(), &mut db) != SQLITE_OK {
        if !db.is_null() {
            sqlite3_close(db);
        }
        return None;
    }

    let init_sql = CString::new(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS clipboard_items (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             kind TEXT NOT NULL,
             preview_text TEXT NOT NULL,
             full_text TEXT NOT NULL,
             image_path TEXT NOT NULL DEFAULT '',
             byte_size INTEGER NOT NULL DEFAULT 0,
             content_hash TEXT NOT NULL UNIQUE,
             source_app TEXT NOT NULL DEFAULT 'macOS',
             is_pinned INTEGER NOT NULL DEFAULT 0,
             created_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_clip_created ON clipboard_items(is_pinned DESC, created_at DESC);",
    )
    .ok()?;
    sqlite3_exec(
        db,
        init_sql.as_ptr(),
        std::ptr::null(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    Some(db)
}

unsafe fn col_string(stmt: *mut Sqlite3Stmt, col: c_int) -> String {
    let ptr = sqlite3_column_text(stmt, col);
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr as *const c_char)
            .to_string_lossy()
            .into_owned()
    }
}

/// Insert or bump an item in SQLite and prune oldest unpinned items beyond 1,000.
pub fn record_clipboard_entry(
    kind: &str,
    full_text: &str,
    image_path: &str,
    byte_size: i64,
    content_hash: &str,
    source_app: &str,
) {
    let _guard = db_mutex().lock().unwrap();
    let preview: String = full_text.chars().take(280).collect();
    let now = now_unix_ms();

    unsafe {
        let Some(db) = open_db() else { return };

        let upsert_sql = CString::new(
            "INSERT INTO clipboard_items (kind, preview_text, full_text, image_path, byte_size, content_hash, source_app, is_pinned, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)
             ON CONFLICT(content_hash) DO UPDATE SET
                 created_at = excluded.created_at,
                 source_app = excluded.source_app;",
        )
        .unwrap();

        let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
        if sqlite3_prepare_v2(
            db,
            upsert_sql.as_ptr(),
            -1,
            &mut stmt,
            std::ptr::null_mut(),
        ) == SQLITE_OK
        {
            let c_kind = CString::new(kind).unwrap_or_default();
            let c_prev = CString::new(preview).unwrap_or_default();
            let c_full = CString::new(full_text).unwrap_or_default();
            let c_img = CString::new(image_path).unwrap_or_default();
            let c_hash = CString::new(content_hash).unwrap_or_default();
            let c_src = CString::new(source_app).unwrap_or_default();

            sqlite3_bind_text(stmt, 1, c_kind.as_ptr(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_text(stmt, 2, c_prev.as_ptr(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_text(stmt, 3, c_full.as_ptr(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_text(stmt, 4, c_img.as_ptr(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_int64(stmt, 5, byte_size);
            sqlite3_bind_text(stmt, 6, c_hash.as_ptr(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_text(stmt, 7, c_src.as_ptr(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_int64(stmt, 8, now);

            let _ = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        }

        // Enforce 1,000 unpinned item limit & delete stale image files from disk
        let prune_select_sql = CString::new(format!(
            "SELECT id, image_path FROM clipboard_items
             WHERE is_pinned = 0
             ORDER BY created_at DESC
             LIMIT -1 OFFSET {};",
            MAX_CLIPBOARD_ITEMS
        ))
        .unwrap();

        let mut p_stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
        let mut ids_to_delete = Vec::new();
        if sqlite3_prepare_v2(
            db,
            prune_select_sql.as_ptr(),
            -1,
            &mut p_stmt,
            std::ptr::null_mut(),
        ) == SQLITE_OK
        {
            while sqlite3_step(p_stmt) == SQLITE_ROW {
                let old_id = sqlite3_column_int64(p_stmt, 0);
                let old_img = col_string(p_stmt, 1);
                if !old_img.is_empty() {
                    let _ = fs::remove_file(&old_img);
                }
                ids_to_delete.push(old_id);
            }
            sqlite3_finalize(p_stmt);
        }

        for old_id in ids_to_delete {
            if let Ok(del_sql) =
                CString::new(format!("DELETE FROM clipboard_items WHERE id = {};", old_id))
            {
                sqlite3_exec(
                    db,
                    del_sql.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
        }

        sqlite3_close(db);
    }
}

/// Query clipboard items with pagination (50 / 100 / 200 / 500 / 1000 per page).
pub fn query_clipboard_page(
    query: &str,
    filter_kind: &str,
    page: usize,
    page_size: usize,
) -> ClipboardPageResult {
    let _guard = db_mutex().lock().unwrap();
    let per_page = if page_size == 0 {
        50
    } else {
        page_size.clamp(1, MAX_CLIPBOARD_ITEMS)
    };
    let current_page = page.max(1);
    let offset = (current_page - 1) * per_page;
    let q_lower = query.trim().to_lowercase();

    let mut page_items = Vec::new();
    let mut total_matching = 0usize;

    unsafe {
        let Some(db) = open_db() else {
            return ClipboardPageResult {
                items: page_items,
                page: current_page,
                page_size: per_page,
                total_matching: 0,
                total_pages: 1,
            };
        };

        let sql = CString::new(
            "SELECT id, kind, preview_text, full_text, image_path, byte_size, content_hash, source_app, is_pinned, created_at
             FROM clipboard_items
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT 1000;",
        )
        .unwrap();

        let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
        if sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) == SQLITE_OK {
            while sqlite3_step(stmt) == SQLITE_ROW {
                let id = sqlite3_column_int64(stmt, 0);
                let kind = col_string(stmt, 1);
                let preview_text = col_string(stmt, 2);
                let full_text = col_string(stmt, 3);
                let image_path = col_string(stmt, 4);
                let byte_size = sqlite3_column_int64(stmt, 5);
                let content_hash = col_string(stmt, 6);
                let source_app = col_string(stmt, 7);
                let is_pinned = sqlite3_column_int64(stmt, 8) != 0;
                let created_at = sqlite3_column_int64(stmt, 9);

                match filter_kind {
                    "pinned" if !is_pinned => continue,
                    "text" if kind != "text" => continue,
                    "image" if kind != "image" => continue,
                    _ => {}
                }

                if !q_lower.is_empty()
                    && !full_text.to_lowercase().contains(&q_lower)
                    && !source_app.to_lowercase().contains(&q_lower)
                {
                    continue;
                }

                let idx = total_matching;
                total_matching += 1;

                if idx >= offset && page_items.len() < per_page {
                    let image_data_url = if kind == "image" && !image_path.is_empty() {
                        if let Ok(bytes) = fs::read(&image_path) {
                            format!("data:image/png;base64,{}", base64_encode(&bytes))
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    page_items.push(ClipboardItem {
                        id,
                        kind,
                        preview_text,
                        full_text,
                        image_path,
                        image_data_url,
                        byte_size,
                        content_hash,
                        source_app,
                        is_pinned,
                        created_at,
                    });
                }
            }
            sqlite3_finalize(stmt);
        }

        sqlite3_close(db);
    }

    let total_pages = ((total_matching + per_page - 1) / per_page).max(1);
    ClipboardPageResult {
        items: page_items,
        page: current_page.min(total_pages),
        page_size: per_page,
        total_matching,
        total_pages,
    }
}

/// Query clipboard items (with search query, category filter, and limit up to 1,000).
pub fn query_clipboard_items(query: &str, filter_kind: &str, limit: usize) -> Vec<ClipboardItem> {
    let per_page = if limit == 0 { 50 } else { limit };
    query_clipboard_page(query, filter_kind, 1, per_page).items
}

pub fn toggle_pin_clipboard_item(id: i64) -> bool {
    let _guard = db_mutex().lock().unwrap();
    unsafe {
        let Some(db) = open_db() else { return false };
        if let Ok(sql) = CString::new(format!(
            "UPDATE clipboard_items SET is_pinned = CASE WHEN is_pinned = 1 THEN 0 ELSE 1 END WHERE id = {};",
            id
        )) {
            sqlite3_exec(
                db,
                sql.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
        sqlite3_close(db);
    }
    true
}

pub fn delete_clipboard_item(id: i64) -> bool {
    let _guard = db_mutex().lock().unwrap();
    unsafe {
        let Some(db) = open_db() else { return false };
        if let Ok(sel_sql) = CString::new(format!(
            "SELECT image_path FROM clipboard_items WHERE id = {};",
            id
        )) {
            let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
            if sqlite3_prepare_v2(db, sel_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut())
                == SQLITE_OK
            {
                if sqlite3_step(stmt) == SQLITE_ROW {
                    let img = col_string(stmt, 0);
                    if !img.is_empty() {
                        let _ = fs::remove_file(&img);
                    }
                }
                sqlite3_finalize(stmt);
            }
        }
        if let Ok(sql) = CString::new(format!("DELETE FROM clipboard_items WHERE id = {};", id)) {
            sqlite3_exec(
                db,
                sql.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
        sqlite3_close(db);
    }
    true
}

pub fn clear_unpinned_clipboard_items() -> bool {
    let _guard = db_mutex().lock().unwrap();
    unsafe {
        let Some(db) = open_db() else { return false };
        if let Ok(sel_sql) = CString::new(
            "SELECT image_path FROM clipboard_items WHERE is_pinned = 0 AND image_path != '';",
        ) {
            let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
            if sqlite3_prepare_v2(db, sel_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut())
                == SQLITE_OK
            {
                while sqlite3_step(stmt) == SQLITE_ROW {
                    let img = col_string(stmt, 0);
                    if !img.is_empty() {
                        let _ = fs::remove_file(&img);
                    }
                }
                sqlite3_finalize(stmt);
            }
        }
        if let Ok(sql) = CString::new("DELETE FROM clipboard_items WHERE is_pinned = 0;") {
            sqlite3_exec(
                db,
                sql.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
        sqlite3_close(db);
    }
    true
}

pub fn get_clipboard_stats() -> ClipboardStats {
    let _guard = db_mutex().lock().unwrap();
    let mut total_items = 0usize;
    let mut pinned_items = 0usize;
    let mut image_items = 0usize;

    unsafe {
        if let Some(db) = open_db() {
            if let Ok(sql) = CString::new(
                "SELECT COUNT(*), SUM(CASE WHEN is_pinned = 1 THEN 1 ELSE 0 END), SUM(CASE WHEN kind = 'image' THEN 1 ELSE 0 END) FROM clipboard_items;",
            ) {
                let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
                if sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut())
                    == SQLITE_OK
                {
                    if sqlite3_step(stmt) == SQLITE_ROW {
                        total_items = sqlite3_column_int64(stmt, 0) as usize;
                        pinned_items = sqlite3_column_int64(stmt, 1) as usize;
                        image_items = sqlite3_column_int64(stmt, 2) as usize;
                    }
                    sqlite3_finalize(stmt);
                }
            }
            sqlite3_close(db);
        }
    }

    let mut disk_usage_bytes = 0u64;
    if let Ok(entries) = fs::read_dir(images_dir()) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                disk_usage_bytes += meta.len();
            }
        }
    }

    ClipboardStats {
        enabled: is_clipboard_enabled(),
        total_items,
        pinned_items,
        image_items,
        max_capacity: MAX_CLIPBOARD_ITEMS,
        disk_usage_bytes,
        polling_interval_ms: 250,
    }
}

