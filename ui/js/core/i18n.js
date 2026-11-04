// =============================================================================
// MULTI-LANGUAGE (i18n) ENGINE — POWERED BY MODULAR LOCALE FILES (`ui/js/locales/*.js`)
// Default Language: English ("en"). Currently supports English ("en") & Vietnamese ("vi").
// Synchronizes language state across Main Dashboard, Menu Bar Popover,
// Clipboard Drawer, Alt-Tab Overlay, and Multi-Monitor Screenshot Overlays.
// =============================================================================
const DEFAULT_LANG = "en";

function getAvailableLocales() {
  const locales = window.OMNI_LOCALES || {};
  const keys = Object.keys(locales);
  return keys.length > 0 ? keys : ["en", "vi"];
}

function getLocaleDict(lang) {
  const locales = window.OMNI_LOCALES || {};
  return locales[lang] || locales[DEFAULT_LANG] || {};
}

function getAppLang() {
  const saved = localStorage.getItem("omni_app_lang");
  const supported = getAvailableLocales();
  return supported.includes(saved) ? saved : DEFAULT_LANG;
}

function isVi() {
  return getAppLang() === "vi";
}

function applyAppLang(lang, broadcast = false) {
  const supported = getAvailableLocales();
  const normalized = supported.includes(lang) ? lang : DEFAULT_LANG;
  document.documentElement.setAttribute("lang", normalized);
  localStorage.setItem("omni_app_lang", normalized);
  if (broadcast && typeof api === "function") {
    api("set_app_lang", { lang: normalized });
  }
  return normalized;
}

function toggleAppLang() {
  const supported = getAvailableLocales();
  const cur = getAppLang();
  const idx = supported.indexOf(cur);
  const next = supported[(idx + 1) % supported.length] || DEFAULT_LANG;
  applyAppLang(next, true);
  if (typeof window.__onLangChanged === "function") {
    window.__onLangChanged();
  }
  return next;
}

/**
 * Translate by dictionary key from `ui/js/locales/<lang>.js` with optional `{var}` interpolation.
 * @param {string} key - Dictionary key in `OMNI_LOCALES[lang].phrases`
 * @param {Record<string, string|number>} [vars] - Optional variables to interpolate
 * @returns {string}
 */
function t(key, vars = {}) {
  const lang = getAppLang();
  const dict = getLocaleDict(lang);
  const fallbackDict = getLocaleDict(DEFAULT_LANG);
  let template =
    (dict.phrases && dict.phrases[key]) ??
    (fallbackDict.phrases && fallbackDict.phrases[key]) ??
    key;

  for (const [k, v] of Object.entries(vars)) {
    template = String(template).replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
  }
  return template;
}

/**
 * Legacy helper: translate using locale dictionary lookup first, falling back to inline `(enText, viText)`.
 * All UI files now use `t(key, vars)` directly so new languages only require a `ui/js/locales/<code.js>` file.
 * @param {string} enText - Phrase key or fallback English string
 * @param {string} [viText] - Fallback Vietnamese string
 * @param {Record<string, string|number>} [vars] - Optional variables to interpolate
 * @returns {string}
 */
function tr(enText, viText, vars = {}) {
  const lang = getAppLang();
  const dict = getLocaleDict(lang);
  let template =
    (dict.phrases && dict.phrases[enText] !== undefined)
      ? dict.phrases[enText]
      : (lang === "vi" && viText !== undefined ? viText : enText);

  for (const [k, v] of Object.entries(vars)) {
    template = String(template).replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
  }
  return template;
}

/**
 * Render a reusable `<select>` dropdown populated dynamically from `window.OMNI_LOCALES`.
 * Adding a new locale file in `ui/js/locales/*.js` automatically adds an `<option>` here.
 */
function renderLanguageSelectHtml(id = "sel-app-lang", style = "", compact = false) {
  const cur = getAppLang();
  const locales = window.OMNI_LOCALES || {};
  const codes = getAvailableLocales();
  const optionsHtml = codes
    .map((code) => {
      const meta = locales[code] || { code, name: code.toUpperCase(), shortLabel: code.toUpperCase(), flag: "🌐" };
      const label = compact
        ? `${meta.flag || "🌐"} ${meta.shortLabel || code.toUpperCase()}`
        : `${meta.flag || "🌐"} ${meta.name || code}`;
      return `<option value="${code}" ${code === cur ? "selected" : ""}>${label}</option>`;
    })
    .join("");

  return `<select class="select-box lang-select-box" id="${id}" style="${style}" title="${t("common.select_language")}">${optionsHtml}</select>`;
}

/**
 * Bind change handler to a language `<select>` element.
 */
function bindLanguageSelect(selectElOrId) {
  const el = typeof selectElOrId === "string" ? document.getElementById(selectElOrId) : selectElOrId;
  if (!el) return;
  el.onchange = (e) => {
    const next = applyAppLang(e.target.value, true);
    if (typeof window.__onLangChanged === "function") {
      window.__onLangChanged();
    }
    const meta = getLocaleDict(next);
    if (typeof showToast === "function") {
      showToast(
        t("common.lang_switched", {
          flag: meta.flag || "🌐",
          name: meta.name || next.toUpperCase()
        })
      );
    }
  };
}

window.__applyLangFromRust = (lang) => {
  applyAppLang(lang, false);
  if (typeof window.__onLangChanged === "function") {
    window.__onLangChanged();
  }
};

// Initialize <html lang="en|vi"> immediately on load (default: "en")
applyAppLang(getAppLang(), false);

/**
 * Translate window tiling layout label by action key via `ui/js/locales/<lang>.js`.
 */
function tWindowActionLabel(action, fallbackLabel) {
  const dict = getLocaleDict(getAppLang());
  const fallbackDict = getLocaleDict(DEFAULT_LANG);
  return (
    (dict.windowActions && dict.windowActions[action]) ||
    (fallbackDict.windowActions && fallbackDict.windowActions[action]) ||
    fallbackLabel ||
    action
  );
}

/**
 * Translate window tiling category name via `ui/js/locales/<lang>.js`.
 */
function tWindowCategory(category) {
  const dict = getLocaleDict(getAppLang());
  const fallbackDict = getLocaleDict(DEFAULT_LANG);
  return (
    (dict.windowCategories && dict.windowCategories[category]) ||
    (fallbackDict.windowCategories && fallbackDict.windowCategories[category]) ||
    category ||
    ""
  );
}

/**
 * Translate external mouse action label by action ID via `ui/js/locales/<lang>.js`.
 */
function tMouseActionLabel(action) {
  const dict = getLocaleDict(getAppLang());
  const fallbackDict = getLocaleDict(DEFAULT_LANG);
  return (
    (dict.mouseActions && dict.mouseActions[action]) ||
    (fallbackDict.mouseActions && fallbackDict.mouseActions[action]) ||
    action ||
    ""
  );
}

/**
 * Translate CoreAudio device kind label via `ui/js/locales/<lang>.js`.
 */
function tAudioKindLabel(kindLabel) {
  const dict = getLocaleDict(getAppLang());
  const fallbackDict = getLocaleDict(DEFAULT_LANG);
  return (
    (dict.audioKinds && dict.audioKinds[kindLabel]) ||
    (fallbackDict.audioKinds && fallbackDict.audioKinds[kindLabel]) ||
    kindLabel ||
    ""
  );
}

/**
 * Translate file/junk category returned from core-cleaner via `ui/js/locales/<lang>.js`.
 */
function tCleanerCategory(cat) {
  const dict = getLocaleDict(getAppLang());
  const fallbackDict = getLocaleDict(DEFAULT_LANG);
  return (
    (dict.cleanerCategories && dict.cleanerCategories[cat]) ||
    (fallbackDict.cleanerCategories && fallbackDict.cleanerCategories[cat]) ||
    cat ||
    ""
  );
}

/**
 * Translate background scan progress messages from core-cleaner via locale dictionary keys.
 */
function tScanMessage(msg) {
  if (!msg) return "";
  const s = String(msg);
  if (s.startsWith("Đang quét thư mục ~/")) {
    const folder = s.slice("Đang quét thư mục ~/".length).replace(/\s*\.\.\.$/, "");
    return t("cleaner.scan.folder", { folder });
  }
  if (s.startsWith("Đã quét xong ~/")) {
    const folder = s.slice("Đã quét xong ~/".length);
    return t("cleaner.scan.folder_done", { folder });
  }
  if (s.startsWith("Đang quét: ")) {
    const name = s.slice("Đang quét: ".length);
    return t("cleaner.scan.item", { name });
  }
  if (s.includes("Bước 1/3:")) {
    return t("cleaner.scan.dup_step1");
  }
  if (s.includes("Bước 2/3:")) {
    const m = s.match(/(\d+)/g);
    const count = m && m.length >= 3 ? m[2] : null;
    return count ? t("cleaner.scan.dup_step2_count", { count }) : t("cleaner.scan.dup_step2");
  }
  if (s.includes("Bước 3/3:")) {
    const m = s.match(/(\d+)/g);
    const count = m && m.length >= 4 ? m[3] : null;
    return count ? t("cleaner.scan.dup_step3_count", { count }) : t("cleaner.scan.dup_step3");
  }
  if (s.includes("Đang đối chiếu nội dung tệp")) {
    return t("cleaner.scan.comparing");
  }
  if (s.includes("Đang quét hệ thống")) {
    return t("cleaner.scan.filesystem");
  }
  return s;
}
