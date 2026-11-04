#[test]
fn test_display_and_hardware_and_clipboard_and_cleaner() {
    // 1. Display & Audio state
    let disp_state = core_display::get_display_system_state();
    println!(
        "Detected {} display(s), dark_mode = {}",
        disp_state.displays.len(),
        disp_state.dark_mode
    );
    let cursor_disp = core_display::get_display_at_mouse_cursor();
    assert!(cursor_disp.width > 0 && cursor_disp.height > 0);

    let audio_state = core_display::get_audio_system_state();
    println!(
        "Audio output: '{}' (vol={:.0}%, muted={}), {} output devices, {} input devices",
        audio_state.current_output_name,
        audio_state.volume * 100.0,
        audio_state.muted,
        audio_state.output_devices.len(),
        audio_state.input_devices.len()
    );
    assert!(!audio_state.output_devices.is_empty());

    // 2. Hardware monitor & Apple Silicon temperatures & Top 10 RAM tasks
    let hw = core_monitor::get_hardware_dashboard("ram", 10);
    println!(
        "CPU: {} ({} cores) - Usage: {}% - CPU Temp: {}°C, GPU Temp: {}°C",
        hw.cpu_brand,
        hw.cpu_cores,
        hw.cpu_usage_percent,
        hw.temperatures.cpu_temp_c,
        hw.temperatures.gpu_temp_c
    );
    assert!(hw.cpu_cores > 0);
    assert!(hw.total_memory_bytes > 0);
    assert!(!hw.top_processes.is_empty());
    assert!(hw.top_processes.len() <= 10);

    // Verify Top 10 RAM tasks are sorted descending by memory_bytes
    for pair in hw.top_processes.windows(2) {
        assert!(pair[0].memory_bytes >= pair[1].memory_bytes);
    }

    // 3. Clipboard 1000-item SQLite storage
    core_clipboard::record_clipboard_entry(
        "text",
        "Xin chào từ Mac Omni Tool Rust!",
        "",
        32,
        "test_hash_001",
        "CargoTest",
    );
    let clips = core_clipboard::query_clipboard_items("Mac Omni Tool", "all", 50);
    assert!(!clips.is_empty());
    let stats = core_clipboard::get_clipboard_stats();
    assert_eq!(stats.max_capacity, 1000);
    assert_eq!(stats.polling_interval_ms, 250);

    // 4. Customizable window shortcuts
    let shortcuts = core_window::get_window_shortcuts();
    assert!(shortcuts.len() >= 12);

    // 5. Input config, External Mouse & Screenshot config (per-mode shortcuts & toggles)
    let input_cfg = core_input::get_input_config();
    assert_eq!(input_cfg.clipboard_popup_shortcut, "Cmd+Shift+V");
    assert_eq!(input_cfg.mouse_chord_action, "mission_control");
    assert!(input_cfg.screenshot_enabled);
    assert!(input_cfg.screenshot_region_enabled);
    assert_eq!(input_cfg.screenshot_shortcut, "Cmd+Shift+0");
    assert!(input_cfg.screenshot_fullscreen_edit_enabled);
    assert!(!input_cfg.screenshot_fullscreen_edit_shortcut.is_empty());
    assert!(input_cfg.screenshot_fullscreen_copy_enabled);
    assert!(!input_cfg.screenshot_fullscreen_copy_shortcut.is_empty());
    assert!(input_cfg.scroll_step_px >= 15);
    assert!(input_cfg.scroll_duration_ms >= 80);
    assert!(!input_cfg.app_mouse_profiles.is_empty());
    assert!(!input_cfg.menubar_display_mode.is_empty());
    assert!(input_cfg.menubar_refresh_interval_sec >= 1 && input_cfg.menubar_refresh_interval_sec <= 60);
    assert!(!input_cfg.dashboard_shortcut.is_empty());
    assert_eq!(
        input_cfg.resolve_mouse_button_action("com.google.Chrome", 4),
        "default"
    );

    // Verify 1x1 transparent PNG base64 decode & clipboard copy
    let sample_png_data_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let decoded = core_clipboard::decode_base64_png(sample_png_data_url)
        .expect("Failed to decode sample PNG data URL");
    assert!(decoded.starts_with(&[0x89, b'P', b'N', b'G']));
    core_clipboard::copy_png_bytes_to_clipboard(&decoded)
        .expect("Failed to copy PNG bytes to clipboard");

    // 6. Cleaner, Trash Status, Duplicate Files Scanner & Autostart
    let autostart = core_cleaner::get_autostart_status();
    assert!(autostart.plist_path.contains("com.khacthang.mac-omni-tool.plist"));
    let trash = core_cleaner::get_trash_status();
    assert!(trash.trash_path.contains(".Trash"));
    let junk = core_cleaner::scan_system_junk();
    println!(
        "Scanned junk: {:.1} MB across {} items (Trash: {} items, {:.1} MB)",
        junk.total_size_mb,
        junk.items.len(),
        junk.trash.item_count,
        junk.trash.total_size_mb
    );
    let dups = core_cleaner::scan_duplicate_files("desktop", 1024);
    println!(
        "Scanned duplicates: {} groups, {} duplicate files ({:.1} MB wasted)",
        dups.total_groups, dups.total_duplicate_files, dups.total_wasted_mb
    );

    // 7. Verify SQL script (.sql) duplicate detection + Internal SQL/DB engine exclusion + Custom excluded folders
    let test_root = std::env::temp_dir().join(format!(
        "omni_dup_sql_test_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&test_root);
    std::fs::create_dir_all(&test_root).unwrap();

    // (a) Create two identical normal .sql files (> 1KB) -> MUST be detected as duplicates!
    let sql_content = "-- SQL Dump\nCREATE TABLE users (id INT PRIMARY KEY, name TEXT);\n"
        .repeat(40);
    std::fs::write(test_root.join("backup_1.sql"), &sql_content).unwrap();
    std::fs::write(test_root.join("backup_2.sql"), &sql_content).unwrap();

    // (b) Create two identical internal MySQL InnoDB (.ibd) files -> MUST be excluded!
    let ibd_content = vec![0x42u8; 4096];
    std::fs::write(test_root.join("users_a.ibd"), &ibd_content).unwrap();
    std::fs::write(test_root.join("users_b.ibd"), &ibd_content).unwrap();

    // (c) Create a directory inside an excluded folder ("ignore_me") with duplicate .sql files -> MUST be skipped!
    let ignored_dir = test_root.join("ignore_me");
    std::fs::create_dir_all(&ignored_dir).unwrap();
    std::fs::write(ignored_dir.join("ignored_1.sql"), &sql_content).unwrap();
    std::fs::write(ignored_dir.join("ignored_2.sql"), &sql_content).unwrap();

    // (d) Create an internal Postgres data directory (contains PG_VERSION) -> MUST be skipped automatically!
    let pg_cluster = test_root.join("my_pg_cluster");
    std::fs::create_dir_all(&pg_cluster).unwrap();
    std::fs::write(pg_cluster.join("PG_VERSION"), "18\n").unwrap();
    std::fs::write(pg_cluster.join("inside_pg_1.sql"), &sql_content).unwrap();
    std::fs::write(pg_cluster.join("inside_pg_2.sql"), &sql_content).unwrap();

    let excludes = vec!["ignore_me".to_string(), "mysql".to_string()];
    let scan_res = core_cleaner::scan_duplicate_files_in_roots(
        std::slice::from_ref(&test_root),
        1,
        &excludes,
    );

    // Exactly 1 duplicate group (for backup_1.sql and backup_2.sql), and 0 from .ibd, ignore_me, or my_pg_cluster!
    assert_eq!(scan_res.total_groups, 1);
    assert_eq!(scan_res.groups[0].kind, "File SQL (.sql)");
    assert_eq!(scan_res.groups[0].count, 2);

    let _ = std::fs::remove_dir_all(&test_root);
}

#[test]
fn test_menubar_stats_and_repeated_hardware_polling() {
    let mb = core_monitor::get_menubar_stats();
    assert!(mb.cpu_usage_percent >= 0.0);
    assert!(mb.memory_usage_percent > 0.0);
    assert!(mb.cpu_temp_c > 15.0);

    for sort in ["ram", "cpu", "disk", "name"] {
        let dash = core_monitor::get_hardware_dashboard(sort, 5);
        assert!(dash.top_processes.len() <= 5);
    }
}

#[test]
fn test_window_shortcut_conflict_and_reset() {
    let initial = core_window::get_window_shortcuts();
    assert!(!initial.is_empty());

    // Attempting to assign left_half's shortcut ("Ctrl+Option+Left") to center should fail conflict check
    let conflict = core_window::update_window_shortcut("center", "Ctrl+Option+Left", true);
    assert!(conflict.is_err());

    // Resetting restores default shortcuts cleanly
    let reset = core_window::reset_window_shortcuts();
    let center = reset.iter().find(|s| s.action == "center").expect("center exists");
    assert_eq!(center.shortcut, "Ctrl+Option+C");
    assert!(center.enabled);
}
