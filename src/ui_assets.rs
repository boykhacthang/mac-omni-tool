/// Embedded UI assets served via the `omni://localhost/...` custom protocol.
/// Keeping all CSS, Locale dictionaries, and JS modularized in `ui/` while compiling
/// them into the Rust binary ensures `MacOmniTool.app` remains 100% self-contained.

pub const INDEX_HTML: &str = include_str!("../ui/index.html");

// Stylesheets
pub const CSS_BASE: &str = include_str!("../ui/styles/base.css");
pub const CSS_SCREENS: &str = include_str!("../ui/styles/screens.css");
pub const CSS_OVERLAYS: &str = include_str!("../ui/styles/overlays.css");

// Core RPC, Locale Dictionaries & Multi-Language (i18n) Engine
pub const JS_CORE_API: &str = include_str!("../ui/js/core/api.js");
pub const JS_LOCALE_EN: &str = include_str!("../ui/js/locales/en.js");
pub const JS_LOCALE_VI: &str = include_str!("../ui/js/locales/vi.js");
pub const JS_CORE_I18N: &str = include_str!("../ui/js/core/i18n.js");

// Reusable Components
pub const JS_COMP_CLIPBOARD_CARD: &str = include_str!("../ui/js/components/clipboard-card.js");
pub const JS_COMP_PATH_CELL: &str = include_str!("../ui/js/components/path-cell.js");

// Standalone Dashboard Screen Modules
pub const JS_SCREEN_DISPLAY_AUDIO: &str = include_str!("../ui/js/screens/display-audio.js");
pub const JS_SCREEN_HARDWARE_MONITOR: &str = include_str!("../ui/js/screens/hardware-monitor.js");
pub const JS_SCREEN_CLIPBOARD_TAB: &str = include_str!("../ui/js/screens/clipboard-tab.js");
pub const JS_SCREEN_WINDOW_TILING: &str = include_str!("../ui/js/screens/window-tiling.js");
pub const JS_SCREEN_MOUSE: &str = include_str!("../ui/js/screens/mouse.js");
pub const JS_SCREEN_SYSTEM_CLEANER: &str = include_str!("../ui/js/screens/system-cleaner.js");
pub const JS_SCREEN_SETTINGS: &str = include_str!("../ui/js/screens/settings.js");

// Standalone Overlay Window Modules
pub const JS_OVERLAY_MENUBAR: &str = include_str!("../ui/js/overlays/menubar-popover.js");
pub const JS_OVERLAY_CLIPBOARD_DRAWER: &str = include_str!("../ui/js/overlays/clipboard-drawer.js");
pub const JS_OVERLAY_ALTTAB: &str = include_str!("../ui/js/overlays/alttab-overlay.js");
pub const JS_OVERLAY_SCREENSHOT: &str = include_str!("../ui/js/overlays/screenshot-overlay.js");

// Main Dashboard Shell & Mode Router
pub const JS_APP_ROUTER: &str = include_str!("../ui/js/app.js");

/// Resolve a request path on `omni://localhost` into `(content_type, bytes)`.
pub fn resolve_ui_asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    let clean = path.split('?').next().unwrap_or(path);
    match clean {
        "/" | "/index.html" => Some(("text/html; charset=utf-8", INDEX_HTML.as_bytes())),
        "/styles/base.css" => Some(("text/css; charset=utf-8", CSS_BASE.as_bytes())),
        "/styles/screens.css" => Some(("text/css; charset=utf-8", CSS_SCREENS.as_bytes())),
        "/styles/overlays.css" => Some(("text/css; charset=utf-8", CSS_OVERLAYS.as_bytes())),
        "/js/core/api.js" => Some(("application/javascript; charset=utf-8", JS_CORE_API.as_bytes())),
        "/js/locales/en.js" => Some((
            "application/javascript; charset=utf-8",
            JS_LOCALE_EN.as_bytes(),
        )),
        "/js/locales/vi.js" => Some((
            "application/javascript; charset=utf-8",
            JS_LOCALE_VI.as_bytes(),
        )),
        "/js/core/i18n.js" => Some((
            "application/javascript; charset=utf-8",
            JS_CORE_I18N.as_bytes(),
        )),
        "/js/components/clipboard-card.js" => Some((
            "application/javascript; charset=utf-8",
            JS_COMP_CLIPBOARD_CARD.as_bytes(),
        )),
        "/js/components/path-cell.js" => Some((
            "application/javascript; charset=utf-8",
            JS_COMP_PATH_CELL.as_bytes(),
        )),
        "/js/screens/display-audio.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_DISPLAY_AUDIO.as_bytes(),
        )),
        "/js/screens/hardware-monitor.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_HARDWARE_MONITOR.as_bytes(),
        )),
        "/js/screens/clipboard-tab.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_CLIPBOARD_TAB.as_bytes(),
        )),
        "/js/screens/window-tiling.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_WINDOW_TILING.as_bytes(),
        )),
        "/js/screens/mouse.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_MOUSE.as_bytes(),
        )),
        "/js/screens/system-cleaner.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_SYSTEM_CLEANER.as_bytes(),
        )),
        "/js/screens/settings.js" => Some((
            "application/javascript; charset=utf-8",
            JS_SCREEN_SETTINGS.as_bytes(),
        )),
        "/js/overlays/menubar-popover.js" => Some((
            "application/javascript; charset=utf-8",
            JS_OVERLAY_MENUBAR.as_bytes(),
        )),
        "/js/overlays/clipboard-drawer.js" => Some((
            "application/javascript; charset=utf-8",
            JS_OVERLAY_CLIPBOARD_DRAWER.as_bytes(),
        )),
        "/js/overlays/alttab-overlay.js" => Some((
            "application/javascript; charset=utf-8",
            JS_OVERLAY_ALTTAB.as_bytes(),
        )),
        "/js/overlays/screenshot-overlay.js" => Some((
            "application/javascript; charset=utf-8",
            JS_OVERLAY_SCREENSHOT.as_bytes(),
        )),
        "/js/app.js" => Some(("application/javascript; charset=utf-8", JS_APP_ROUTER.as_bytes())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_ui_assets_referenced_in_index_html_resolve() {
        let (mime, index_bytes) = resolve_ui_asset("/index.html").expect("index.html must resolve");
        assert!(mime.starts_with("text/html"));
        assert!(!index_bytes.is_empty());

        let expected_paths = [
            "/styles/base.css",
            "/styles/screens.css",
            "/styles/overlays.css",
            "/js/core/api.js",
            "/js/locales/en.js",
            "/js/locales/vi.js",
            "/js/core/i18n.js",
            "/js/components/clipboard-card.js",
            "/js/components/path-cell.js",
            "/js/screens/display-audio.js",
            "/js/screens/hardware-monitor.js",
            "/js/screens/clipboard-tab.js",
            "/js/screens/window-tiling.js",
            "/js/screens/mouse.js",
            "/js/screens/system-cleaner.js",
            "/js/screens/settings.js",
            "/js/overlays/menubar-popover.js",
            "/js/overlays/clipboard-drawer.js",
            "/js/overlays/alttab-overlay.js",
            "/js/overlays/screenshot-overlay.js",
            "/js/app.js",
        ];

        for path in expected_paths {
            assert!(
                INDEX_HTML.contains(path),
                "ui/index.html must reference asset {}",
                path
            );
            let (content_type, bytes) =
                resolve_ui_asset(path).unwrap_or_else(|| panic!("Asset {} must resolve", path));
            assert!(
                !bytes.is_empty(),
                "Asset {} must not be empty (content_type={})",
                path,
                content_type
            );
        }
    }

    #[test]
    fn test_locale_dictionaries_contain_matching_keys() {
        use std::collections::BTreeSet;

        assert!(JS_LOCALE_EN.contains("window.OMNI_LOCALES.en"));
        assert!(JS_LOCALE_VI.contains("window.OMNI_LOCALES.vi"));
        assert!(JS_CORE_I18N.contains("DEFAULT_LANG = \"en\""));

        fn extract_section_entries(js: &str, section: &str) -> std::collections::BTreeMap<String, String> {
            let marker = format!("  {}: {{", section);
            let start = js
                .find(&marker)
                .unwrap_or_else(|| panic!("Missing section {}", section));
            let after = &js[start + marker.len()..];
            let end = after
                .find("\n  }")
                .unwrap_or_else(|| panic!("Missing end of section {}", section));
            let block = &after[..end];
            let mut entries = std::collections::BTreeMap::new();
            for line in block.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with("//") {
                    continue;
                }
                if let Some((lhs, rhs)) = trimmed.split_once(':') {
                    let key = lhs.trim().trim_matches('"').trim_matches('\'');
                    let val = rhs.trim().trim_end_matches(',').trim().trim_matches('"');
                    if !key.is_empty() {
                        entries.insert(key.to_string(), val.to_string());
                    }
                }
            }
            entries
        }

        fn extract_placeholders(val: &str) -> BTreeSet<String> {
            let mut vars = BTreeSet::new();
            let mut rest = val;
            while let Some(open) = rest.find('{') {
                let after_open = &rest[open + 1..];
                if let Some(close) = after_open.find('}') {
                    let var = &after_open[..close];
                    if !var.is_empty() && var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        vars.insert(var.to_string());
                    }
                    rest = &after_open[close + 1..];
                } else {
                    break;
                }
            }
            vars
        }

        for section in [
            "windowActions",
            "windowCategories",
            "mouseActions",
            "audioKinds",
            "cleanerCategories",
            "phrases",
        ] {
            let en_entries = extract_section_entries(JS_LOCALE_EN, section);
            let vi_entries = extract_section_entries(JS_LOCALE_VI, section);
            let en_keys: BTreeSet<_> = en_entries.keys().cloned().collect();
            let vi_keys: BTreeSet<_> = vi_entries.keys().cloned().collect();
            assert!(!en_keys.is_empty(), "Section {} must not be empty", section);
            let missing_in_vi: Vec<_> = en_keys.difference(&vi_keys).collect();
            let missing_in_en: Vec<_> = vi_keys.difference(&en_keys).collect();
            assert!(
                missing_in_vi.is_empty() && missing_in_en.is_empty(),
                "Key mismatch in section `{}`: missing_in_vi={:?}, missing_in_en={:?}",
                section,
                missing_in_vi,
                missing_in_en
            );

            for (key, en_val) in &en_entries {
                let vi_val = &vi_entries[key];
                let en_vars = extract_placeholders(en_val);
                let vi_vars = extract_placeholders(vi_val);
                assert_eq!(
                    en_vars, vi_vars,
                    "Placeholder mismatch for key `{}` in section `{}`: en={:?}, vi={:?}",
                    key, section, en_vars, vi_vars
                );
            }
        }

        // Verify all t("...") / t('...') keys across UI modules exist in locales
        let en_phrases: BTreeSet<_> = extract_section_entries(JS_LOCALE_EN, "phrases")
            .keys()
            .cloned()
            .collect();
        let ui_modules = [
            ("i18n.js", JS_CORE_I18N),
            ("clipboard-card.js", JS_COMP_CLIPBOARD_CARD),
            ("path-cell.js", JS_COMP_PATH_CELL),
            ("display-audio.js", JS_SCREEN_DISPLAY_AUDIO),
            ("hardware-monitor.js", JS_SCREEN_HARDWARE_MONITOR),
            ("clipboard-tab.js", JS_SCREEN_CLIPBOARD_TAB),
            ("window-tiling.js", JS_SCREEN_WINDOW_TILING),
            ("mouse.js", JS_SCREEN_MOUSE),
            ("system-cleaner.js", JS_SCREEN_SYSTEM_CLEANER),
            ("settings.js", JS_SCREEN_SETTINGS),
            ("menubar-popover.js", JS_OVERLAY_MENUBAR),
            ("clipboard-drawer.js", JS_OVERLAY_CLIPBOARD_DRAWER),
            ("alttab-overlay.js", JS_OVERLAY_ALTTAB),
            ("screenshot-overlay.js", JS_OVERLAY_SCREENSHOT),
            ("app.js", JS_APP_ROUTER),
        ];

        for (name, src) in ui_modules {
            // Check no inline tr( or isVi( remains outside i18n.js
            if name != "i18n.js" {
                assert!(
                    !src.contains("tr("),
                    "Module {} still contains inline tr(...) call",
                    name
                );
                assert!(
                    !src.contains("isVi("),
                    "Module {} still contains inline isVi(...) call",
                    name
                );
            }

            // Extract literal keys passed to t("...") or t('...')
            let bytes = src.as_bytes();
            let mut i = 0;
            while i + 3 < bytes.len() {
                let prev_ok = i == 0 || {
                    let c = bytes[i - 1] as char;
                    !c.is_ascii_alphanumeric() && c != '_' && c != '$'
                };
                if prev_ok && bytes[i] == b't' && bytes[i + 1] == b'(' {
                    let quote = bytes[i + 2];
                    if quote == b'"' || quote == b'\'' {
                        let start = i + 3;
                        let mut end = start;
                        while end < bytes.len() && bytes[end] != quote {
                            end += 1;
                        }
                        if end < bytes.len() {
                            let key = &src[start..end];
                            if key.contains('.') {
                                assert!(
                                    en_phrases.contains(key),
                                    "Module {} references missing locale key `{}`",
                                    name,
                                    key
                                );
                            }
                        }
                    }
                }
                i += 1;
            }
        }
    }
}
