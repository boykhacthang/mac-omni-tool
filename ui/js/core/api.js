// =============================================================================
// CORE RPC CLIENT, THEME SYNCHRONIZATION & SHARED FORMATTERS
// =============================================================================
const params = new URLSearchParams(window.location.search);
const APP_MODE = params.get("mode") || "main";

async function api(cmd, payload = {}) {
  const res = await fetch("omni://localhost/api", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ cmd, payload })
  });
  return await res.json();
}

// Theme Sync Across All Windows (Main, Menu Bar Popover, Clipboard Drawer, Alt-Tab)
function getAppTheme() {
  return localStorage.getItem("omni_app_theme") || "dark";
}

function applyAppTheme(theme, broadcast = false) {
  const t = theme === "light" ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", t);
  localStorage.setItem("omni_app_theme", t);
  if (broadcast) {
    api("set_app_theme", { theme: t });
  }
}

function toggleAppTheme() {
  const next = getAppTheme() === "dark" ? "light" : "dark";
  applyAppTheme(next, true);
  return next;
}

window.__applyThemeFromRust = (theme) => {
  applyAppTheme(theme, false);
  if (typeof window.__onThemeChanged === "function") {
    window.__onThemeChanged();
  }
};

applyAppTheme(getAppTheme(), false);

function renderTip(text) {
  return `
    <span class="tip-wrap">
      <span class="tip-btn">?</span>
      <span class="tip-pop">${escapeHtml(text)}</span>
    </span>
  `;
}

function showToast(msg) {
  const el = document.getElementById("toast");
  if (!el) return;
  el.textContent = msg;
  el.style.display = "block";
  clearTimeout(window.__toastTimer);
  window.__toastTimer = setTimeout(() => {
    el.style.display = "none";
  }, 2400);
}

function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 1 ? 1 : 0) + " " + units[i];
}

function formatPct(val) {
  return Number(val || 0).toFixed(1) + "%";
}

function formatTemp(val) {
  return Math.round(Number(val || 0)) + "°C";
}

function formatUptime(secs) {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${d > 0 ? d + "d " : ""}${h}h ${m}m`;
}

function escapeHtml(str) {
  return String(str || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function eventToShortcutString(e) {
  e.preventDefault();
  e.stopPropagation();
  const ignore = ["Control", "Shift", "Alt", "Meta"];
  if (ignore.includes(e.key)) return null;

  const parts = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Option");
  if (e.metaKey) parts.push("Cmd");
  if (e.shiftKey) parts.push("Shift");

  let key = e.key;
  if (key === "ArrowLeft") key = "Left";
  else if (key === "ArrowRight") key = "Right";
  else if (key === "ArrowUp") key = "Up";
  else if (key === "ArrowDown") key = "Down";
  else if (key === " ") key = "Space";
  else if (key.length === 1) key = key.toUpperCase();

  parts.push(key);
  return parts.join("+");
}

function formatCompactNetRate(bytesPerSec) {
  const b = Number(bytesPerSec || 0);
  if (b < 1024) return "0 KB/s";
  if (b < 1024 * 1024) return Math.round(b / 1024) + " KB/s";
  if (b < 1024 * 1024 * 1024) return (b / (1024 * 1024)).toFixed(1) + " MB/s";
  return (b / (1024 * 1024 * 1024)).toFixed(1) + " GB/s";
}

function formatMenuBarTitlePreview(cfg, hw) {
  const c = cfg || {};
  const h = hw || {};
  const rawIcon = String(c.menubar_icon || "❖").trim();
  const icon = rawIcon.toLowerCase() === "none" || !rawIcon ? "" : rawIcon;
  const withPrefix = (body) => {
    if (!body) return icon || "❖";
    return icon ? `${icon} ${body}` : body;
  };

  const rawTemp = h.cpu_temp_c ?? (h.temperatures && h.temperatures.cpu_temp_c) ?? 45;
  const tempStr = Math.max(30, Math.round(Number(rawTemp || 45))) + "°C";
  const cpuVal = Math.max(0, Number(h.cpu_usage_percent ?? 12.4));
  const cpuPctStr = cpuVal.toFixed(1) + "%";
  const ramVal = Math.max(0, Math.round(Number(h.memory_usage_percent ?? 54)));
  const ramPctStr = ramVal + "%";
  const usedGb = (Number(h.used_memory_bytes || (8.6 * 1073741824)) / 1073741824).toFixed(1);
  const totalGb = Math.max(1, Math.round(Number(h.total_memory_bytes || (16 * 1073741824)) / 1073741824));
  const rxStr = formatCompactNetRate(h.net_rx_bytes_per_sec || 0);
  const txStr = formatCompactNetRate(h.net_tx_bytes_per_sec || 0);

  const mode = String(c.menubar_display_mode || "temp_cpu");
  switch (mode) {
    case "icon_only":
      return withPrefix("");
    case "icon_name":
      return icon ? `${icon} Omni` : "Omni";
    case "temp_only":
      return withPrefix(tempStr);
    case "cpu_only":
      return withPrefix(cpuPctStr);
    case "ram_only":
      return withPrefix(`RAM ${ramPctStr}`);
    case "ram_gb":
      return withPrefix(`${usedGb}/${totalGb} GB`);
    case "cpu_ram":
      return withPrefix(`CPU ${cpuPctStr} • RAM ${ramPctStr}`);
    case "temp_cpu_ram":
      return withPrefix(`${tempStr} • CPU ${Math.round(cpuVal)}% • RAM ${ramPctStr}`);
    case "net_speed":
      return withPrefix(`↓ ${rxStr} ↑ ${txStr}`);
    case "temp_net":
      return withPrefix(`${tempStr} • ↓ ${rxStr}`);
    case "custom": {
      const parts = [];
      if (c.menubar_show_temp !== false) parts.push(tempStr);
      if (c.menubar_show_cpu !== false) {
        parts.push(c.menubar_show_ram ? `CPU ${Math.round(cpuVal)}%` : cpuPctStr);
      }
      if (c.menubar_show_ram) parts.push(`RAM ${ramPctStr}`);
      if (c.menubar_show_net) parts.push(`↓ ${rxStr} ↑ ${txStr}`);
      return withPrefix(parts.join(" • "));
    }
    default:
      return withPrefix(`${tempStr} • ${cpuPctStr}`);
  }
}


