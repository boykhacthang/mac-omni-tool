//! macOS LaunchAgent (`~/Library/LaunchAgents/com.khacthang.mac-omni-tool.plist`) management.

use crate::models::AutostartConfig;
use std::fs;
use std::path::PathBuf;

pub fn launch_agent_plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home).join("Library").join("LaunchAgents");
    let _ = fs::create_dir_all(&dir);
    dir.join("com.khacthang.mac-omni-tool.plist")
}

pub fn get_autostart_status() -> AutostartConfig {
    let plist = launch_agent_plist_path();
    let exe = std::env::current_exe()
        .unwrap_or_else(|_| {
            PathBuf::from("/Applications/MacOmniTool.app/Contents/MacOS/mac-omni-tool")
        })
        .to_string_lossy()
        .to_string();

    if let Ok(content) = fs::read_to_string(&plist) {
        let enabled = content.contains("<true/>");
        let start_minimized = content.contains("--minimized");
        AutostartConfig {
            enabled,
            start_minimized,
            plist_path: plist.to_string_lossy().to_string(),
            exe_path: exe,
        }
    } else {
        AutostartConfig {
            enabled: false,
            start_minimized: false,
            plist_path: plist.to_string_lossy().to_string(),
            exe_path: exe,
        }
    }
}

pub fn set_autostart_config(
    enabled: bool,
    start_minimized: bool,
) -> Result<AutostartConfig, String> {
    let plist = launch_agent_plist_path();
    let exe = std::env::current_exe()
        .unwrap_or_else(|_| {
            PathBuf::from("/Applications/MacOmniTool.app/Contents/MacOS/mac-omni-tool")
        })
        .to_string_lossy()
        .to_string();

    if !enabled {
        if plist.exists() {
            let _ = fs::remove_file(&plist);
        }
        return Ok(get_autostart_status());
    }

    let args_xml = if start_minimized {
        format!(
            "        <string>{}</string>\n        <string>--minimized</string>",
            exe
        )
    } else {
        format!("        <string>{}</string>", exe)
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.khacthang.mac-omni-tool</string>
    <key>AssociatedBundleIdentifiers</key>
    <string>com.khacthang.mac-omni-tool</string>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
    <key>ProgramArguments</key>
    <array>
{}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        args_xml
    );

    fs::write(&plist, xml).map_err(|e| format!("Không thể ghi LaunchAgent plist: {}", e))?;
    Ok(get_autostart_status())
}

