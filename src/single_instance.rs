fn instance_lock_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let dir = std::path::PathBuf::from(home).join("Library/Application Support/MacOmniTool");
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir.join("instance.lock");
        }
    }
    std::path::PathBuf::from("/tmp/com.khacthang.mac-omni-tool.instance.lock")
}

/// Ensure only a single instance of MacOmniTool can ever run at the same time.
/// Prevents duplicate instances when macOS Session Restore and LaunchAgent both fire on reboot.
pub fn ensure_single_instance(start_minimized: bool) {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::AsRawFd;

    let lock_path = instance_lock_path();
    let Ok(mut file) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&lock_path)
    else {
        return;
    };

    let fd = file.as_raw_fd();
    let my_pid = std::process::id() as i32;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let locked = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) == 0 };
    if !locked {
        let mut contents = String::new();
        let _ = file.seek(SeekFrom::Start(0));
        let _ = file.read_to_string(&mut contents);
        let mut parts = contents.split_whitespace();
        let existing_pid = parts
            .next()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let existing_ts = parts
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        // If launched in background (--minimized) or within 20s of the existing instance (boot/login race), exit immediately!
        if start_minimized || now_secs.saturating_sub(existing_ts) < 20 {
            std::process::exit(0);
        }

        // Otherwise, user manually launched a new instance: cleanly terminate the old instance and take over
        if existing_pid > 1 && existing_pid != my_pid {
            unsafe {
                let _ = libc::kill(existing_pid, libc::SIGTERM);
            }
            let mut acquired = false;
            for _ in 0..12 {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) == 0 } {
                    acquired = true;
                    break;
                }
            }
            if !acquired {
                unsafe {
                    let _ = libc::kill(existing_pid, libc::SIGKILL);
                }
                std::thread::sleep(std::time::Duration::from_millis(120));
                if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) != 0 } {
                    std::process::exit(0);
                }
            }
        } else {
            std::process::exit(0);
        }
    }

    let _ = file.set_len(0);
    let _ = file.seek(SeekFrom::Start(0));
    let _ = writeln!(file, "{} {}", my_pid, now_secs);
    let _ = file.flush();
    // Keep file descriptor open for the entire lifetime of the process so the flock remains active
    std::mem::forget(file);

    start_self_uninstall_watchdog();
}

extern "C" {
    fn proc_pidpath(pid: i32, buffer: *mut libc::c_void, buffersize: u32) -> i32;
}

fn current_live_executable_path() -> Option<String> {
    let mut buf = [0u8; 4096];
    let pid = std::process::id() as i32;
    let len = unsafe { proc_pidpath(pid, buf.as_mut_ptr() as *mut libc::c_void, buf.len() as u32) };
    if len > 0 {
        let slice = &buf[..len as usize];
        Some(String::from_utf8_lossy(slice).to_string())
    } else {
        None
    }
}

/// Watchdog that automatically stops `mac-omni-tool` and removes its LaunchAgent if the user
/// drags `MacOmniTool.app` into the macOS Trash (`~/.Trash`) or deletes the `.app` bundle while it is running.
fn start_self_uninstall_watchdog() {
    let Ok(initial_exe) = std::env::current_exe() else {
        return;
    };
    let initial_str = initial_exe.to_string_lossy().to_string();
    // Only watch when running inside a packaged `.app` bundle
    if !initial_str.contains(".app/Contents/MacOS/") {
        return;
    }

    std::thread::spawn(move || {
        let mut missing_streak = 0u8;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));

            let live_path = current_live_executable_path().unwrap_or_default();
            let moved_to_trash = live_path.contains("/.Trash/");
            let missing_on_disk = !initial_exe.exists()
                && (live_path.is_empty() || !std::path::Path::new(&live_path).exists());

            if moved_to_trash {
                let _ = core_cleaner::set_autostart_config(false, false);
                let _ = std::fs::remove_file(instance_lock_path());
                std::process::exit(0);
            } else if missing_on_disk {
                missing_streak = missing_streak.saturating_add(1);
                if missing_streak >= 2 {
                    let _ = core_cleaner::set_autostart_config(false, false);
                    let _ = std::fs::remove_file(instance_lock_path());
                    std::process::exit(0);
                }
            } else {
                missing_streak = 0;
            }
        }
    });
}

