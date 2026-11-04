//! Installed Application Uninstaller with leftover file detection, background process termination,
//! LaunchAgent cleanup, orphaned process detection, and `NSWorkspace` icon extraction.

use crate::models::{AppLeftoverFile, InstalledAppInfo, OrphanedBackgroundItem};
use crate::scan_manager::{invalidate_cleaner_cache, update_scan_progress};
use crate::trash_and_junk::{clean_selected_paths_ext, compute_dir_size, move_path_to_trash};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void, CString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[repr(C)]
#[derive(Copy, Clone)]
struct NSSize {
    width: f64,
    height: f64,
}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

static APP_ICON_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize]);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize]);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize]
        } else {
            b'='
        });
    }
    String::from_utf8(out).unwrap_or_default()
}

fn get_app_icon_data_url(app_path: &str) -> String {
    let cache = APP_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(existing) = guard.get(app_path) {
            return existing.clone();
        }
    }

    let data_url = unsafe {
        let pool = objc_autoreleasePoolPush();
        let mut result = String::new();

        type Msg0 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type Msg0Usize = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
        type Msg0Ptr = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const u8;
        type Msg1 = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
        type Msg1Str = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        type MsgSetSize = unsafe extern "C" fn(*mut c_void, *mut c_void, NSSize);
        type Msg2Repr =
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *mut c_void) -> *mut c_void;

        let msg0: Msg0 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_usize: Msg0Usize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg0_ptr: Msg0Ptr = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1: Msg1 = std::mem::transmute(objc_msgSend as *const c_void);
        let msg1_str: Msg1Str = std::mem::transmute(objc_msgSend as *const c_void);
        let msg_set_size: MsgSetSize = std::mem::transmute(objc_msgSend as *const c_void);
        let msg2_repr: Msg2Repr = std::mem::transmute(objc_msgSend as *const c_void);

        let ws_cls = objc_getClass(c"NSWorkspace".as_ptr());
        let ns_str_cls = objc_getClass(c"NSString".as_ptr());
        let rep_cls = objc_getClass(c"NSBitmapImageRep".as_ptr());

        if !ws_cls.is_null() && !ns_str_cls.is_null() && !rep_cls.is_null() {
            if let Ok(c_path) = CString::new(app_path) {
                let ws = msg0(ws_cls, sel_registerName(c"sharedWorkspace".as_ptr()));
                let ns_path = msg1_str(
                    ns_str_cls,
                    sel_registerName(c"stringWithUTF8String:".as_ptr()),
                    c_path.as_ptr(),
                );
                if !ws.is_null() && !ns_path.is_null() {
                    let img = msg1(ws, sel_registerName(c"iconForFile:".as_ptr()), ns_path);
                    if !img.is_null() {
                        msg_set_size(
                            img,
                            sel_registerName(c"setSize:".as_ptr()),
                            NSSize {
                                width: 40.0,
                                height: 40.0,
                            },
                        );
                        let tiff = msg0(img, sel_registerName(c"TIFFRepresentation".as_ptr()));
                        if !tiff.is_null() {
                            let rep = msg1(
                                rep_cls,
                                sel_registerName(c"imageRepWithData:".as_ptr()),
                                tiff,
                            );
                            if !rep.is_null() {
                                let png = msg2_repr(
                                    rep,
                                    sel_registerName(
                                        c"representationUsingType:properties:".as_ptr(),
                                    ),
                                    4,
                                    std::ptr::null_mut(),
                                );
                                if !png.is_null() {
                                    let len = msg0_usize(png, sel_registerName(c"length".as_ptr()));
                                    let ptr = msg0_ptr(png, sel_registerName(c"bytes".as_ptr()));
                                    if len > 0 && !ptr.is_null() {
                                        let slice = std::slice::from_raw_parts(ptr, len);
                                        result = format!(
                                            "data:image/png;base64,{}",
                                            base64_encode(slice)
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        objc_autoreleasePoolPop(pool);
        result
    };

    if !data_url.is_empty() {
        if let Ok(mut guard) = cache.lock() {
            guard.insert(app_path.to_string(), data_url.clone());
        }
    }
    data_url
}

/// Extract a `<string>` value for `<key>{key}</key>` in an XML `Info.plist`.
pub fn extract_plist_key(xml: &str, key: &str) -> Option<String> {
    let needle = format!("<key>{}</key>", key);
    let pos = xml.find(&needle)?;
    let rest = &xml[pos + needle.len()..];
    let s_start = rest.find("<string>")?;
    let after_s = &rest[s_start + 8..];
    let s_end = after_s.find("</string>")?;
    Some(after_s[..s_end].trim().to_string())
}

/// Extract the primary executable path from a LaunchAgent/LaunchDaemon XML plist (`Program` or `ProgramArguments[0]`).
pub fn extract_plist_program_path(xml: &str) -> Option<String> {
    if let Some(prog) = extract_plist_key(xml, "Program") {
        if !prog.is_empty() {
            return Some(prog);
        }
    }
    let needle = "<key>ProgramArguments</key>";
    let pos = xml.find(needle)?;
    let rest = &xml[pos + needle.len()..];
    let arr_start = rest.find("<array>")?;
    let after_arr = &rest[arr_start + 7..];
    let arr_end = after_arr.find("</array>")?;
    let arr_body = &after_arr[..arr_end];
    let s_start = arr_body.find("<string>")?;
    let after_s = &arr_body[s_start + 8..];
    let s_end = after_s.find("</string>")?;
    let val = after_s[..s_end].trim().to_string();
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

fn read_plist_xml_fast(plist_path: &Path) -> Option<String> {
    if let Ok(bytes) = fs::read(plist_path) {
        if !bytes.starts_with(b"bplist") {
            if let Ok(s) = String::from_utf8(bytes) {
                return Some(s);
            }
        }
    }
    if let Ok(out) = Command::new("/usr/bin/plutil")
        .args(["-convert", "xml1", "-o", "-", &plist_path.to_string_lossy()])
        .output()
    {
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }
    None
}

fn read_app_bundle_metadata(app_path: &Path) -> (String, String, String) {
    let default_name = app_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "App".to_string());
    let plist_path = app_path.join("Contents/Info.plist");

    if let Some(xml) = read_plist_xml_fast(&plist_path) {
        let bundle_id = extract_plist_key(&xml, "CFBundleIdentifier").unwrap_or_default();
        let name = extract_plist_key(&xml, "CFBundleDisplayName")
            .or_else(|| extract_plist_key(&xml, "CFBundleName"))
            .unwrap_or_else(|| default_name.clone());
        let version = extract_plist_key(&xml, "CFBundleShortVersionString")
            .unwrap_or_else(|| "1.0".to_string());
        return (name, bundle_id, version);
    }

    (default_name, String::new(), "1.0".to_string())
}

pub fn find_app_leftovers(app_name: &str, bundle_id: &str) -> Vec<AppLeftoverFile> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let lib = PathBuf::from(home).join("Library");
    let mut found = Vec::new();

    let has_full_disk_access = fs::read_dir(lib.join("Safari")).is_ok()
        || fs::read_dir(lib.join("Mail")).is_ok()
        || fs::read_dir("/Library/Application Support/com.apple.TCC").is_ok();

    let mut search_dirs = vec![
        ("LaunchAgents", lib.join("LaunchAgents")),
        ("Application Support", lib.join("Application Support")),
        ("Caches", lib.join("Caches")),
        ("Preferences", lib.join("Preferences")),
        ("Saved State", lib.join("Saved Application State")),
        ("HTTPStorages", lib.join("HTTPStorages")),
        ("WebKit", lib.join("WebKit")),
        ("Application Scripts", lib.join("Application Scripts")),
        ("Logs", lib.join("Logs")),
    ];
    // Only scan ~/Library/Containers & Group Containers when Full Disk Access is granted
    // so macOS never pops up "Mac Omni Tool would like to access data from other apps" automatically.
    if has_full_disk_access {
        search_dirs.push(("Containers", lib.join("Containers")));
        search_dirs.push(("Group Containers", lib.join("Group Containers")));
    }

    let name_clean: String = app_name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    let bid_lower = bundle_id.to_lowercase();
    let bid_prefix = {
        let parts: Vec<&str> = bid_lower.split('.').collect();
        if parts.len() >= 3 {
            format!("{}.{}.{}", parts[0], parts[1], parts[2])
        } else {
            String::new()
        }
    };

    for (kind, dir) in search_dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            let fname_alnum: String = fname.chars().filter(|c| c.is_alphanumeric()).collect();

            let matches_bid = !bid_lower.is_empty() && fname.contains(&bid_lower);
            let matches_bid_prefix =
                bid_prefix.len() >= 8 && fname.contains(&bid_prefix);
            let matches_name = name_clean.len() >= 4 && fname_alnum.contains(&name_clean);

            let mut matched = matches_bid || matches_bid_prefix || matches_name;

            if !matched && kind == "LaunchAgents" && fname.ends_with(".plist") {
                if let Some(xml) = read_plist_xml_fast(&p) {
                    let xml_lower = xml.to_lowercase();
                    if (!bid_lower.is_empty() && xml_lower.contains(&bid_lower))
                        || (app_name.len() >= 4
                            && (xml_lower.contains(&format!("/{}.app", app_name.to_lowercase()))
                                || xml_lower.contains(&format!(
                                    "/application support/{}",
                                    app_name.to_lowercase()
                                ))))
                    {
                        matched = true;
                    }
                }
            }

            if matched {
                let sz = compute_dir_size(&p, 4);
                found.push(AppLeftoverFile {
                    kind: kind.to_string(),
                    path: p.to_string_lossy().to_string(),
                    size_bytes: sz,
                    size_mb: ((sz as f64 / (1024.0 * 1024.0)) * 100.0).round() / 100.0,
                });
            }
        }
    }

    found
}

/// Return `(pid, executable_path)` for all user-space non-system processes.
pub fn list_user_running_processes() -> Vec<(u32, String)> {
    let my_pid = std::process::id();
    let mut list = Vec::new();
    let Ok(out) = Command::new("/bin/ps").args(["-axo", "pid=,comm="]).output() else {
        return list;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let Some(pid_str) = parts.next() else {
            continue;
        };
        let Some(comm_raw) = parts.next() else {
            continue;
        };
        let Ok(pid) = pid_str.trim().parse::<u32>() else {
            continue;
        };
        if pid <= 1 || pid == my_pid {
            continue;
        }
        let comm = comm_raw.trim();
        if comm.starts_with("/System/")
            || comm.starts_with("/usr/")
            || comm.starts_with("/bin/")
            || comm.starts_with("/sbin/")
        {
            continue;
        }
        list.push((pid, comm.to_string()));
    }
    list
}

/// Find all running process PIDs belonging to an application or its background helpers.
pub fn find_matching_pids_for_app(
    app_path: &str,
    app_name: &str,
    bundle_id: &str,
    related_paths: &[String],
    all_procs: &[(u32, String)],
) -> Vec<u32> {
    let mut pids = Vec::new();
    let app_path_lower = app_path.to_lowercase();
    let app_name_lower = app_name.trim().to_lowercase();
    let bid_lower = bundle_id.trim().to_lowercase();

    let support_prefixes: Vec<String> = related_paths
        .iter()
        .filter(|p| {
            p.ends_with(".app")
                || p.contains("/Application Support/")
                || p.contains("/Containers/")
        })
        .map(|p| p.to_lowercase())
        .collect();

    for (pid, comm) in all_procs {
        let comm_lower = comm.to_lowercase();
        let mut matched = false;

        if !app_path_lower.is_empty() && comm_lower.starts_with(&app_path_lower) {
            matched = true;
        }
        if !matched {
            for pref in &support_prefixes {
                if !pref.is_empty() && comm_lower.starts_with(pref) {
                    matched = true;
                    break;
                }
            }
        }
        if !matched && !bid_lower.is_empty() && comm_lower.contains(&bid_lower) {
            matched = true;
        }
        if !matched && app_name_lower.len() >= 3 {
            let pat_app = format!("/{}.app/", app_name_lower);
            let pat_helper = format!("/{} helper", app_name_lower);
            let pat_agent = format!("/{} agent", app_name_lower);
            let pat_menubar = format!("/{} menubar", app_name_lower);
            let pat_daemon = format!("/{} daemon", app_name_lower);
            let pat_support = format!("/application support/{}", app_name_lower);
            if comm_lower.contains(&pat_app)
                || comm_lower.contains(&pat_helper)
                || comm_lower.contains(&pat_agent)
                || comm_lower.contains(&pat_menubar)
                || comm_lower.contains(&pat_daemon)
                || comm_lower.contains(&pat_support)
            {
                matched = true;
            }
        }

        if matched {
            pids.push(*pid);
        }
    }

    pids.sort_unstable();
    pids.dedup();
    pids
}

/// Unload a LaunchAgent `.plist` from `launchd` so it does not respawn killed helper processes.
pub fn unload_launch_agent_plist(plist_path: &str) {
    let _ = Command::new("/bin/launchctl")
        .args(["unload", "-w", plist_path])
        .output();
}

/// Send `SIGTERM` followed by `SIGKILL` to a list of PIDs, returning how many were terminated.
pub fn terminate_pids(pids: &[u32]) -> usize {
    let my_pid = std::process::id();
    let valid_pids: Vec<String> = pids
        .iter()
        .copied()
        .filter(|&pid| pid > 1 && pid != my_pid)
        .map(|pid| pid.to_string())
        .collect();
    if valid_pids.is_empty() {
        return 0;
    }

    let mut term_args = vec!["-15".to_string()];
    term_args.extend(valid_pids.iter().cloned());
    let _ = Command::new("/bin/kill").args(&term_args).output();

    std::thread::sleep(Duration::from_millis(120));

    let mut kill_args = vec!["-9".to_string()];
    kill_args.extend(valid_pids.iter().cloned());
    let _ = Command::new("/bin/kill").args(&kill_args).output();

    valid_pids.len()
}

/// Terminate any running user processes whose executable lives inside the given paths,
/// and unload any LaunchAgent `.plist` files in the list before moving them to Trash.
pub fn stop_processes_and_agents_for_paths(paths: &[String]) -> usize {
    for p in paths {
        if p.ends_with(".plist") && p.contains("/LaunchAgents/") {
            unload_launch_agent_plist(p);
        }
    }

    let dir_prefixes: Vec<String> = paths
        .iter()
        .filter(|p| {
            p.ends_with(".app")
                || p.contains("/Application Support/")
                || p.contains("/Containers/")
        })
        .map(|p| p.to_lowercase())
        .collect();

    if dir_prefixes.is_empty() {
        return 0;
    }

    let all_procs = list_user_running_processes();
    let mut pids = Vec::new();
    for (pid, comm) in &all_procs {
        let comm_lower = comm.to_lowercase();
        if dir_prefixes.iter().any(|pref| comm_lower.starts_with(pref)) {
            pids.push(*pid);
        }
    }
    terminate_pids(&pids)
}

/// Completely uninstall an application:
/// 1. Unload its `~/Library/LaunchAgents/*.plist` files so `launchd` won't respawn helpers.
/// 2. Quit and force-kill the running application and all of its background Helper/Daemon/Agent processes.
/// 3. Move the `.app` bundle, `~/Library` leftovers, and LaunchAgents to `~/.Trash` (and optionally empty Trash).
pub fn uninstall_application(
    app_path: &str,
    app_name: &str,
    bundle_id: &str,
    related_paths: &[String],
    also_empty_trash: bool,
) -> Result<String, String> {
    let mut path_set: HashSet<String> = HashSet::new();
    if !app_path.is_empty() {
        path_set.insert(app_path.to_string());
    }
    for p in related_paths {
        if !p.is_empty() {
            path_set.insert(p.clone());
        }
    }
    for leftover in find_app_leftovers(app_name, bundle_id) {
        path_set.insert(leftover.path);
    }

    let all_paths: Vec<String> = path_set.into_iter().collect();

    // 1. Unload LaunchAgents first so launchd does not respawn background helpers
    for p in &all_paths {
        if p.ends_with(".plist") && p.contains("/LaunchAgents/") {
            unload_launch_agent_plist(p);
        }
    }

    // 2. Find and terminate all running processes & background helpers for this app
    let all_procs = list_user_running_processes();
    let pids = find_matching_pids_for_app(app_path, app_name, bundle_id, &all_paths, &all_procs);
    let killed_count = terminate_pids(&pids);

    // 3. Move app bundle + leftovers to Trash (and optionally empty Trash)
    let clean_msg = clean_selected_paths_ext(&all_paths, also_empty_trash)?;
    invalidate_cleaner_cache();

    if killed_count > 0 {
        Ok(format!(
            "Đã tắt {} tiến trình chạy ngầm của {} • {}",
            killed_count, app_name, clean_msg
        ))
    } else {
        Ok(format!("Đã gỡ sạch {} • {}", app_name, clean_msg))
    }
}

/// Scan for orphaned background processes (running from already-deleted `.app` bundles)
/// and orphaned `~/Library/LaunchAgents/*.plist` files pointing to missing executables.
pub fn scan_orphaned_background_items() -> Vec<OrphanedBackgroundItem> {
    let mut items = Vec::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let launch_agents_dir = PathBuf::from(&home).join("Library/LaunchAgents");

    // 1. Check ~/Library/LaunchAgents/*.plist for missing target binaries
    if let Ok(entries) = fs::read_dir(&launch_agents_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();
            if !fname.ends_with(".plist") {
                continue;
            }
            if let Some(xml) = read_plist_xml_fast(&p) {
                if let Some(prog) = extract_plist_program_path(&xml) {
                    let is_app_or_user_bin = prog.starts_with("/Applications/")
                        || prog.starts_with("/Users/")
                        || prog.starts_with("/Library/");
                    if is_app_or_user_bin
                        && (!Path::new(&prog).exists() || prog.contains("/.Trash/"))
                    {
                        items.push(OrphanedBackgroundItem {
                            kind: "LaunchAgent".to_string(),
                            name: fname,
                            pid: None,
                            target_path: prog,
                            plist_path: Some(p.to_string_lossy().to_string()),
                        });
                    }
                }
            }
        }
    }

    // 2. Check running user processes whose .app bundle no longer exists on disk (or is in Trash)
    let all_procs = list_user_running_processes();
    for (pid, comm) in all_procs {
        let is_user_or_app = comm.starts_with("/Applications/")
            || comm.starts_with("/Users/")
            || comm.starts_with("/Library/");
        if !is_user_or_app {
            continue;
        }
        if let Some(app_idx) = comm.find(".app/") {
            let app_bundle = &comm[..app_idx + 4];
            if !Path::new(app_bundle).exists() || comm.contains("/.Trash/") {
                let proc_name = Path::new(&comm)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| comm.clone());
                items.push(OrphanedBackgroundItem {
                    kind: "Running Process".to_string(),
                    name: proc_name,
                    pid: Some(pid),
                    target_path: comm,
                    plist_path: None,
                });
            }
        }
    }

    items
}

/// Kill all orphaned background processes and unload/trash all orphaned `LaunchAgents` plists.
pub fn clean_orphaned_background_items() -> Result<String, String> {
    let items = scan_orphaned_background_items();
    if items.is_empty() {
        return Ok("Không có tiến trình ngầm mồ côi nào đang chạy.".to_string());
    }

    let mut pids = Vec::new();
    let mut removed_agents = 0usize;

    for item in &items {
        if let Some(ref plist) = item.plist_path {
            unload_launch_agent_plist(plist);
            if move_path_to_trash(plist).is_ok() {
                removed_agents += 1;
            }
        }
        if let Some(pid) = item.pid {
            pids.push(pid);
        }
    }

    let killed_procs = terminate_pids(&pids);
    invalidate_cleaner_cache();

    Ok(format!(
        "Đã buộc dừng {} tiến trình ngầm mồ côi và gỡ {} LaunchAgent còn sót!",
        killed_procs, removed_agents
    ))
}

pub fn scan_installed_applications() -> Vec<InstalledAppInfo> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dirs = [
        PathBuf::from("/Applications"),
        PathBuf::from(home).join("Applications"),
    ];

    let mut app_paths = Vec::new();
    for d in dirs {
        if let Ok(entries) = fs::read_dir(d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|ext| ext == "app").unwrap_or(false) {
                    app_paths.push(p);
                }
            }
        }
    }

    let all_procs = list_user_running_processes();
    let total_apps = app_paths.len().max(1);
    let done_counter = AtomicUsize::new(0);

    let mut apps: Vec<InstalledAppInfo> = app_paths
        .into_par_iter()
        .map(|app_path| {
            let (name, bundle_id, version) = read_app_bundle_metadata(&app_path);
            let done = done_counter.fetch_add(1, Ordering::Relaxed) + 1;
            let pct = ((done * 88) / total_apps).clamp(5, 90) as u8;
            update_scan_progress(
                "uninstall",
                true,
                pct,
                &format!("Đang phân tích: {}.app", name),
            );

            let app_path_str = app_path.to_string_lossy().to_string();
            let app_size_bytes = compute_dir_size(&app_path, 4);
            let mut related_files = vec![AppLeftoverFile {
                kind: "App Bundle (.app)".to_string(),
                path: app_path_str.clone(),
                size_bytes: app_size_bytes,
                size_mb: ((app_size_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
            }];

            let leftovers = find_app_leftovers(&name, &bundle_id);
            let leftovers_size_bytes: u64 = leftovers.iter().map(|l| l.size_bytes).sum();
            related_files.extend(leftovers);

            let rel_paths: Vec<String> = related_files.iter().map(|f| f.path.clone()).collect();
            let running_pids = find_matching_pids_for_app(
                &app_path_str,
                &name,
                &bundle_id,
                &rel_paths,
                &all_procs,
            );

            let total_bytes = app_size_bytes + leftovers_size_bytes;
            InstalledAppInfo {
                name,
                bundle_id,
                version,
                app_path: app_path_str,
                icon_data_url: String::new(),
                app_size_bytes,
                app_size_mb: ((app_size_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
                leftovers_size_bytes,
                total_size_mb: ((total_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
                running_pids,
                related_files,
            }
        })
        .collect();

    apps.sort_by(|a, b| {
        (b.app_size_bytes + b.leftovers_size_bytes)
            .cmp(&(a.app_size_bytes + a.leftovers_size_bytes))
    });

    update_scan_progress(
        "uninstall",
        true,
        95,
        "Đang trích xuất biểu tượng ứng dụng...",
    );
    for app in &mut apps {
        app.icon_data_url = get_app_icon_data_url(&app.app_path);
    }

    apps
}

