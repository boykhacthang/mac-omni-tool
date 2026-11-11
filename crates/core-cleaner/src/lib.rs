//! `core-cleaner`: Multi-threaded System Junk Cleaner, macOS Trash manager, App Uninstaller with Leftover detection,
//! Large Files scanner, 3-stage Duplicate Files scanner, and LaunchAgent Autostart configuration.

pub mod autostart;
pub mod duplicates;
pub mod large_files;
pub mod models;
pub mod scan_manager;
pub mod trash_and_junk;
pub mod uninstaller;

pub use autostart::{get_autostart_status, launch_agent_plist_path, set_autostart_config};
pub use duplicates::{
    add_duplicate_exclude, classify_duplicate_kind, compute_edge_hash_128, compute_full_hash_128,
    default_duplicate_excludes, get_duplicate_exclude_list, is_internal_db_engine_file,
    remove_duplicate_exclude, reset_duplicate_excludes, scan_duplicate_files,
    scan_duplicate_files_in_roots, should_skip_duplicate_dir,
};
pub use large_files::{classify_file_kind, reveal_in_finder, scan_large_files};
pub use models::{
    AppLeftoverFile, AutostartConfig, CleanerScanStatus, DuplicateExcludeConfig,
    DuplicateFileEntry, DuplicateGroup, DuplicateScanSummary, InstalledAppInfo, JunkItem,
    JunkScanSummary, LargeFileEntry, OrphanedBackgroundItem, TrashItem, TrashStatus,
};
pub use scan_manager::{
    filter_duplicates_by_excludes_in_cached_scan, get_cleaner_scan_status,
    invalidate_cleaner_cache, prewarm_cleaner_cache, release_cleaner_memory,
    remove_deleted_paths_from_cached_scan, remove_uninstalled_app_from_cached_scan,
    scan_installed_applications_cached, scan_system_junk_cached, start_cleaner_scan_async,
};
pub use trash_and_junk::{
    clean_selected_paths, clean_selected_paths_ext, compute_dir_size, compute_dir_size_fast,
    empty_macos_trash, get_trash_status, move_path_to_trash, scan_system_junk,
};
pub use uninstaller::{
    clean_orphaned_background_items, extract_plist_key, extract_plist_program_path,
    find_app_leftovers, find_matching_pids_for_app, scan_installed_applications,
    scan_orphaned_background_items, uninstall_application,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_plist_key() {
        let xml = r#"
            <plist version="1.0">
            <dict>
                <key>CFBundleIdentifier</key>
                <string>com.khacthang.mac-omni-tool</string>
                <key>CFBundleDisplayName</key>
                <string>Mac Omni</string>
                <key>ProgramArguments</key>
                <array>
                    <string>/Users/me/Library/Application Support/iStat Menus 7/iStat Menus Helper.app/Contents/MacOS/iStat Menus Helper</string>
                </array>
            </dict>
            </plist>
        "#;
        assert_eq!(
            extract_plist_key(xml, "CFBundleIdentifier").as_deref(),
            Some("com.khacthang.mac-omni-tool")
        );
        assert_eq!(
            extract_plist_key(xml, "CFBundleDisplayName").as_deref(),
            Some("Mac Omni")
        );
        assert_eq!(extract_plist_key(xml, "NonExistent"), None);
        assert_eq!(
            extract_plist_program_path(xml).as_deref(),
            Some("/Users/me/Library/Application Support/iStat Menus 7/iStat Menus Helper.app/Contents/MacOS/iStat Menus Helper")
        );
    }

    #[test]
    fn test_find_matching_pids_for_app() {
        let procs = vec![
            (
                4673,
                "/Users/khacthang/Library/Application Support/iStat Menus 7/iStat Menus Helper.app/Contents/MacOS/iStat Menus Helper".to_string(),
            ),
            (
                8000,
                "/Applications/Spotify.app/Contents/MacOS/Spotify".to_string(),
            ),
        ];
        let related = vec![
            "/Users/khacthang/Library/Application Support/iStat Menus 7".to_string(),
        ];
        let matched = find_matching_pids_for_app(
            "/Applications/iStat Menus.app",
            "iStat Menus",
            "com.bjango.istatmenus",
            &related,
            &procs,
        );
        assert_eq!(matched, vec![4673]);
    }

    #[test]
    fn test_internal_db_engine_vs_normal_sql_file() {
        // Normal .sql dump/script files MUST be allowed (return false)
        assert!(!is_internal_db_engine_file(Path::new("/tmp/schema_backup.sql")));
        assert!(!is_internal_db_engine_file(Path::new("/Users/me/dump.SQL")));

        // Internal database engine storage files MUST be blocked (return true)
        assert!(is_internal_db_engine_file(Path::new("/var/mysql/users.ibd")));
        assert!(is_internal_db_engine_file(Path::new("/var/mysql/ibdata1")));
        assert!(is_internal_db_engine_file(Path::new("/var/pg/PG_VERSION")));
        assert!(is_internal_db_engine_file(Path::new("/var/db/app.sqlite-wal")));
        assert!(is_internal_db_engine_file(Path::new("/var/mongo/WiredTiger.wt")));
    }

    #[test]
    fn test_should_skip_duplicate_dir_rules() {
        let excludes = vec![
            "node_modules".to_string(),
            "~/Documents/secret_project".to_string(),
        ];
        let home = "/Users/khacthang";

        assert!(should_skip_duplicate_dir(
            Path::new("/Users/khacthang/proj/node_modules"),
            &excludes,
            home
        ));
        assert!(should_skip_duplicate_dir(
            Path::new("/Users/khacthang/.git"),
            &excludes,
            home
        ));
        assert!(should_skip_duplicate_dir(
            Path::new("/Users/khacthang/Documents/secret_project/sub"),
            &excludes,
            home
        ));
        assert!(!should_skip_duplicate_dir(
            Path::new("/Users/khacthang/Documents/normal_project"),
            &excludes,
            home
        ));
    }

    #[test]
    fn test_classify_file_kinds() {
        assert_eq!(classify_file_kind(Path::new("movie.mp4")), "Video");
        assert_eq!(
            classify_file_kind(Path::new("installer.dmg")),
            "Disk Image (.dmg/.iso)"
        );
        assert_eq!(
            classify_duplicate_kind(Path::new("backup.sql")),
            "File SQL (.sql)"
        );
        assert_eq!(classify_duplicate_kind(Path::new("photo.png")), "Hình ảnh");
    }
}
