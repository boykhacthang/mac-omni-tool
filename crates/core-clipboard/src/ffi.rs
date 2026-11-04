//! Native macOS SQLite3, Objective-C (`NSPasteboard`, `NSBitmapImageRep`),
//! CoreGraphics (`CGEventPost`), and CoreFoundation FFI declarations.

use std::ffi::{c_char, c_int, c_void};

pub type Sqlite3 = c_void;
pub type Sqlite3Stmt = c_void;
pub const SQLITE_OK: c_int = 0;
pub const SQLITE_ROW: c_int = 100;
pub const SQLITE_TRANSIENT: isize = -1;

#[link(name = "sqlite3")]
extern "C" {
    pub fn sqlite3_open(filename: *const c_char, pp_db: *mut *mut Sqlite3) -> c_int;
    pub fn sqlite3_close(db: *mut Sqlite3) -> c_int;
    pub fn sqlite3_exec(
        db: *mut Sqlite3,
        sql: *const c_char,
        callback: *const c_void,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    pub fn sqlite3_prepare_v2(
        db: *mut Sqlite3,
        z_sql: *const c_char,
        n_byte: c_int,
        pp_stmt: *mut *mut Sqlite3Stmt,
        pz_tail: *mut *const c_char,
    ) -> c_int;
    pub fn sqlite3_bind_text(
        stmt: *mut Sqlite3Stmt,
        idx: c_int,
        val: *const c_char,
        n: c_int,
        destructor: isize,
    ) -> c_int;
    pub fn sqlite3_bind_int64(stmt: *mut Sqlite3Stmt, idx: c_int, val: i64) -> c_int;
    pub fn sqlite3_step(stmt: *mut Sqlite3Stmt) -> c_int;
    pub fn sqlite3_column_int64(stmt: *mut Sqlite3Stmt, i_col: c_int) -> i64;
    pub fn sqlite3_column_text(stmt: *mut Sqlite3Stmt, i_col: c_int) -> *const u8;
    pub fn sqlite3_finalize(stmt: *mut Sqlite3Stmt) -> c_int;
}

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    pub fn CGEventSetFlags(event: *mut c_void, flags: u64);
    pub fn CGEventPost(tap: u32, event: *mut c_void);
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFRelease(cf: *const c_void);
}

