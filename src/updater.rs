use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LATEST_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/boykhacthang/mac-omni-tool/main/releases/latest.json";
pub const GITHUB_LATEST_API_URL: &str =
    "https://api.github.com/repos/boykhacthang/mac-omni-tool/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseManifest {
    pub version: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub zip_url: String,
    #[serde(default)]
    pub dmg_url: String,
    #[serde(default)]
    pub sha256_zip: String,
    #[serde(default)]
    pub notes_en: String,
    #[serde(default)]
    pub notes_vi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub ok: bool,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub published_at: String,
    pub zip_url: String,
    pub dmg_url: String,
    pub sha256_zip: String,
    pub notes_en: String,
    pub notes_vi: String,
    pub app_bundle_path: String,
    pub error: Option<String>,
}

/// Parses a version string like `"v2.1.0"` or `"2.1.0"` into `(major, minor, patch)`.
pub fn parse_semver(ver: &str) -> (u64, u64, u64) {
    let cleaned = ver.trim().trim_start_matches(['v', 'V']);
    let core = cleaned.split(['-', '+']).next().unwrap_or(cleaned);
    let mut parts = core
        .split('.')
        .map(|s| s.trim().parse::<u64>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    (major, minor, patch)
}

/// Compares two semantic version strings (`v1` vs `v2`).
pub fn compare_semver(v1: &str, v2: &str) -> Ordering {
    parse_semver(v1).cmp(&parse_semver(v2))
}

/// Detects the `.app` bundle directory of the currently running executable
/// (e.g. `/Applications/MacOmniTool.app` or `.../target/release/MacOmniTool.app`).
pub fn detect_current_app_bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    for ancestor in exe.ancestors() {
        if let Some(ext) = ancestor.extension() {
            if ext == "app" {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn fetch_manifest_from_raw_github() -> Result<ReleaseManifest, String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let url = format!("{}?t={}", LATEST_MANIFEST_URL, ts);

    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "8",
            "-H",
            "Cache-Control: no-cache",
            &url,
        ])
        .output()
        .map_err(|e| format!("Failed to invoke curl: {}", e))?;

    if !out.status.success() {
        return Err("Failed to fetch latest.json from GitHub".to_string());
    }

    serde_json::from_slice::<ReleaseManifest>(&out.stdout)
        .map_err(|e| format!("Invalid latest.json format: {}", e))
}

fn fetch_manifest_from_github_api() -> Result<ReleaseManifest, String> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "8",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: MacOmniTool-Updater",
            GITHUB_LATEST_API_URL,
        ])
        .output()
        .map_err(|e| format!("Failed to invoke curl for GitHub API: {}", e))?;

    if !out.status.success() {
        return Err("GitHub Releases API returned non-200 status".to_string());
    }

    let val: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Invalid GitHub API JSON: {}", e))?;

    let tag = val
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or(CURRENT_VERSION)
        .trim_start_matches(['v', 'V'])
        .to_string();

    let published_at = val
        .get("published_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let body = val
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut zip_url = format!(
        "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v{0}/MacOmniTool-v{0}-macOS.zip",
        tag
    );
    let mut dmg_url = format!(
        "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v{0}/MacOmniTool-v{0}-Installer.dmg",
        tag
    );

    if let Some(assets) = val.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            if let Some(dl) = asset.get("browser_download_url").and_then(|v| v.as_str()) {
                if dl.ends_with(".zip") {
                    zip_url = dl.to_string();
                } else if dl.ends_with(".dmg") {
                    dmg_url = dl.to_string();
                }
            }
        }
    }

    Ok(ReleaseManifest {
        version: tag,
        published_at,
        zip_url,
        dmg_url,
        sha256_zip: String::new(),
        notes_en: body.clone(),
        notes_vi: body,
    })
}

/// Queries GitHub for the latest release manifest and compares it against `CURRENT_VERSION`.
pub fn check_for_updates() -> UpdateCheckResult {
    let app_bundle_path = detect_current_app_bundle_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/Applications/MacOmniTool.app".to_string());

    let manifest_res = fetch_manifest_from_raw_github().or_else(|_| fetch_manifest_from_github_api());

    match manifest_res {
        Ok(m) => {
            let latest_clean = m.version.trim().trim_start_matches(['v', 'V']).to_string();
            let has_update = compare_semver(&latest_clean, CURRENT_VERSION) == Ordering::Greater;
            let zip_url = if m.zip_url.is_empty() {
                format!(
                    "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v{0}/MacOmniTool-v{0}-macOS.zip",
                    latest_clean
                )
            } else {
                m.zip_url
            };
            let dmg_url = if m.dmg_url.is_empty() {
                format!(
                    "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v{0}/MacOmniTool-v{0}-Installer.dmg",
                    latest_clean
                )
            } else {
                m.dmg_url
            };

            UpdateCheckResult {
                ok: true,
                current_version: CURRENT_VERSION.to_string(),
                latest_version: latest_clean,
                has_update,
                published_at: m.published_at,
                zip_url,
                dmg_url,
                sha256_zip: m.sha256_zip,
                notes_en: m.notes_en,
                notes_vi: m.notes_vi,
                app_bundle_path,
                error: None,
            }
        }
        Err(err) => UpdateCheckResult {
            ok: false,
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            has_update: false,
            published_at: String::new(),
            zip_url: String::new(),
            dmg_url: String::new(),
            sha256_zip: String::new(),
            notes_en: String::new(),
            notes_vi: String::new(),
            app_bundle_path,
            error: Some(err),
        },
    }
}

fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let out = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to run shasum: {}", e))?;
    if !out.status.success() {
        return Err("shasum command failed".to_string());
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.split_whitespace()
        .next()
        .map(|h| h.to_lowercase())
        .ok_or_else(|| "Empty shasum output".to_string())
}

fn is_allowed_update_url(url: &str) -> bool {
    if url.contains("..") || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    url.starts_with("https://github.com/boykhacthang/mac-omni-tool/")
        || url.starts_with("https://raw.githubusercontent.com/boykhacthang/mac-omni-tool/")
}

fn is_valid_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Downloads the release `.zip` archive, verifies its SHA-256 (if provided),
/// extracts `MacOmniTool.app`, clears Gatekeeper quarantine, re-signs with the
/// stable identifier `com.khacthang.mac-omni-tool`, and spawns a background
/// script that replaces the current `.app` bundle and relaunches it.
pub fn download_and_install_update(zip_url: &str, expected_sha256: &str) -> Result<String, String> {
    if !is_allowed_update_url(zip_url) {
        return Err("Untrusted update URL. Only official GitHub URLs are allowed.".to_string());
    }

    let expected_clean = expected_sha256.trim().to_lowercase();
    if !expected_clean.is_empty() && !is_valid_sha256_hex(&expected_clean) {
        return Err("Invalid SHA-256 checksum format.".to_string());
    }

    let target_app = detect_current_app_bundle_path()
        .unwrap_or_else(|| PathBuf::from("/Applications/MacOmniTool.app"));

    let pid = std::process::id();
    let staging_dir = std::env::temp_dir().join(format!("mac_omni_update_{}", pid));
    let _ = std::fs::remove_dir_all(&staging_dir);
    std::fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create temp update dir: {}", e))?;

    let zip_path = staging_dir.join("MacOmniTool-update.zip");
    let extract_dir = staging_dir.join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    // 1. Download the .zip bundle
    let dl_status = Command::new("curl")
        .args(["-fL", "--max-time", "120", "-o"])
        .arg(&zip_path)
        .arg(zip_url)
        .status()
        .map_err(|e| format!("Failed to execute curl to download update: {}", e))?;

    if !dl_status.success() {
        let _ = std::fs::remove_dir_all(&staging_dir);
        return Err("Failed to download update package from GitHub.".to_string());
    }

    // 2. Verify SHA-256 checksum if provided
    if !expected_clean.is_empty() {
        let actual_sha = compute_file_sha256(&zip_path)?;
        if actual_sha != expected_clean {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(format!(
                "SHA-256 checksum mismatch (expected {}, got {}). Update aborted.",
                expected_clean, actual_sha
            ));
        }
    }

    // 3. Extract the .zip archive using macOS native `ditto`
    let ditto_status = Command::new("ditto")
        .args(["-x", "-k"])
        .arg(&zip_path)
        .arg(&extract_dir)
        .status()
        .map_err(|e| format!("Failed to extract update zip via ditto: {}", e))?;

    if !ditto_status.success() {
        let _ = std::fs::remove_dir_all(&staging_dir);
        return Err("Failed to unpack MacOmniTool update archive.".to_string());
    }

    let extracted_app = extract_dir.join("MacOmniTool.app");
    if !extracted_app.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
        return Err("Extracted archive does not contain MacOmniTool.app.".to_string());
    }

    // 4. Remove quarantine flags & re-sign with stable bundle identifier + designated requirement
    let _ = Command::new("xattr")
        .arg("-cr")
        .arg(&extracted_app)
        .status();
    let _ = Command::new("codesign")
        .args([
            "--force",
            "--deep",
            "--sign",
            "-",
            "--identifier",
            "com.khacthang.mac-omni-tool",
            "-r=designated => identifier \"com.khacthang.mac-omni-tool\"",
        ])
        .arg(&extracted_app)
        .status();

    // 5. Spawn detached shell script using positional arguments ($1, $2, $3) to prevent shell injection
    let script_path = staging_dir.join("apply_update.sh");
    let script_content = "\
#!/usr/bin/env zsh\n\
TARGET_APP=\"$1\"\n\
SOURCE_APP=\"$2\"\n\
STAGING_DIR=\"$3\"\n\
if [[ -z \"$TARGET_APP\" || -z \"$SOURCE_APP\" || -z \"$STAGING_DIR\" ]]; then\n\
  exit 1\n\
fi\n\
sleep 0.6\n\
mkdir -p \"$TARGET_APP\"\n\
ditto \"$SOURCE_APP\" \"$TARGET_APP\"\n\
xattr -cr \"$TARGET_APP\" 2>/dev/null || true\n\
codesign --force --deep --sign - --identifier \"com.khacthang.mac-omni-tool\" -r='designated => identifier \"com.khacthang.mac-omni-tool\"' \"$TARGET_APP\" 2>/dev/null || true\n\
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f \"$TARGET_APP\" 2>/dev/null || true\n\
touch \"$TARGET_APP\"\n\
open \"$TARGET_APP\"\n\
rm -rf \"$STAGING_DIR\" 2>/dev/null || true\n";
    std::fs::write(&script_path, script_content)
        .map_err(|e| format!("Failed to write update helper script: {}", e))?;

    Command::new("/bin/zsh")
        .arg(&script_path)
        .arg(&target_app)
        .arg(&extracted_app)
        .arg(&staging_dir)
        .spawn()
        .map_err(|e| format!("Failed to launch update helper script: {}", e))?;

    Ok(target_app.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_compare_semver() {
        assert_eq!(parse_semver("v2.0.0"), (2, 0, 0));
        assert_eq!(parse_semver("2.1.3-beta"), (2, 1, 3));
        assert_eq!(compare_semver("2.1.0", "2.0.0"), Ordering::Greater);
        assert_eq!(compare_semver("v2.0.0", "2.0.0"), Ordering::Equal);
        assert_eq!(compare_semver("1.9.9", "2.0.0"), Ordering::Less);
        assert_eq!(compare_semver("2.0.1", "2.0.0"), Ordering::Greater);
    }

    #[test]
    fn test_allowed_update_urls() {
        assert!(is_allowed_update_url(
            "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.0.0/MacOmniTool-v2.0.0-macOS.zip"
        ));
        assert!(is_allowed_update_url(
            "https://raw.githubusercontent.com/boykhacthang/mac-omni-tool/main/releases/v2.0.0/MacOmniTool-v2.0.0-macOS.zip"
        ));
        assert!(!is_allowed_update_url("https://evil.example.com/MacOmniTool.zip"));
        assert!(!is_allowed_update_url("https://objects.githubusercontent.com/arbitrary-repo/file.zip"));
        assert!(!is_allowed_update_url("https://github.com/boykhacthang/mac-omni-tool/../other/file.zip"));
        assert!(is_valid_sha256_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
        assert!(!is_valid_sha256_hex("not-a-valid-sha256"));
    }
}
