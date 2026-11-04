// =============================================================================
// MAIN DASHBOARD SHELL, ABOUT MODAL & MULTI-WINDOW MODE ROUTER
// =============================================================================
function showAboutModalDialog() {
  const existing = document.getElementById("about-modal-backdrop");
  if (existing) {
    existing.remove();
  }

  const backdrop = document.createElement("div");
  backdrop.id = "about-modal-backdrop";
  backdrop.className = "about-modal-backdrop";
  backdrop.innerHTML = `
    <div class="about-modal-card" role="dialog" aria-modal="true">
      <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:14px;">
        <div style="display:flex;align-items:center;gap:12px;">
          <div class="brand-logo" style="width:44px;height:44px;border-radius:11px;font-size:20px;">M</div>
          <div>
            <div style="font-size:17px;font-weight:800;letter-spacing:-0.3px;">Mac Omni Tool</div>
            <div style="font-size:12px;color:var(--text-muted);display:flex;align-items:center;gap:6px;margin-top:2px;">
              <span class="badge-ok">v2.0.0</span>
              <span>Free Community Edition • MIT License</span>
            </div>
          </div>
        </div>
        <button class="btn btn-sm" id="btn-close-about-x" title="Close">✕</button>
      </div>

      <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:10px;padding:12px 14px;margin-bottom:14px;font-size:12.5px;line-height:1.55;">
        <div style="margin-bottom:8px;color:var(--text-main);">
          ${t("about.modal_desc")}
        </div>
        <div style="display:flex;flex-direction:column;gap:4px;font-size:12px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:8px;">
          <div>👨‍💻 <b>${t("settings.author_label")}</b> <span style="color:var(--text-main);font-weight:700;">boykhacthang</span></div>
          <div>📦 <b>GitHub:</b> <span style="color:var(--accent);font-family:monospace;">https://github.com/boykhacthang/mac-omni-tool</span></div>
          <div>⚡ <b>Engine:</b> <span style="color:var(--text-main);">100% Native Rust + Tao/WRY CoreGraphics & IOKit</span></div>
        </div>
      </div>

      <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap;">
        <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
          <button class="btn btn-sm btn-primary" id="btn-about-github">
            ⭐ GitHub (@boykhacthang)
          </button>
          <button
            class="btn btn-sm"
            id="btn-about-kofi"
            style="background:#72a4f2;border-color:#72a4f2;color:#ffffff;font-weight:700;"
          >
            ☕ Support on Ko-fi
          </button>
        </div>
        <button class="btn btn-sm" id="btn-close-about-ok">
          ${t("about.btn_close")}
        </button>
      </div>
    </div>
  `;

  document.body.appendChild(backdrop);

  const close = () => backdrop.remove();
  backdrop.addEventListener("click", (e) => {
    if (e.target === backdrop) close();
  });
  document.getElementById("btn-close-about-x").onclick = close;
  document.getElementById("btn-close-about-ok").onclick = close;
  document.getElementById("btn-about-github").onclick = () => {
    api("open_external_url", { url: "https://github.com/boykhacthang/mac-omni-tool" });
  };
  document.getElementById("btn-about-kofi").onclick = () => {
    api("open_external_url", { url: "https://ko-fi.com/I1R72A1475" });
  };
}

window.__showAboutModal = showAboutModalDialog;

async function initMainDashboard() {
  const root = document.getElementById("root");
  let activeTab = "display";

  const hardwareMonitorScreen = createHardwareMonitorScreen({
    isTabActive: () => activeTab === "monitor"
  });
  const clipboardScreen = createClipboardScreen();
  const mouseScreen = createMouseScreen({
    isTabActive: () => activeTab === "mouse"
  });
  const systemCleanerScreen = createSystemCleanerScreen({
    isTabActive: () => activeTab === "cleaner"
  });

  function renderShell() {
    const isDark = getAppTheme() === "dark";
    root.innerHTML = `
      <div class="app-shell">
        <aside class="sidebar">
          <div class="brand" id="sidebar-brand-about" title="${t("about.btn_open")}">
            <div class="brand-logo">M</div>
            <div class="brand-title">Mac Omni</div>
          </div>

          <div class="nav-item ${activeTab === 'display' ? 'active' : ''}" data-tab="display">
            <span class="nav-icon">☀️</span> <span>${t("nav.displays")}</span>
          </div>
          <div class="nav-item ${activeTab === 'monitor' ? 'active' : ''}" data-tab="monitor">
            <span class="nav-icon">🌡️</span> <span>${t("nav.hardware")}</span>
          </div>
          <div class="nav-item ${activeTab === 'clipboard' ? 'active' : ''}" data-tab="clipboard">
            <span class="nav-icon">📋</span> <span>${t("nav.clipboard")}</span>
          </div>
          <div class="nav-item ${activeTab === 'tiling' ? 'active' : ''}" data-tab="tiling">
            <span class="nav-icon">📐</span> <span>${t("nav.windows")}</span>
          </div>
          <div class="nav-item ${activeTab === 'mouse' ? 'active' : ''}" data-tab="mouse">
            <span class="nav-icon">🖱️</span> <span>${t("nav.mouse")}</span>
          </div>
          <div class="nav-item ${activeTab === 'cleaner' ? 'active' : ''}" data-tab="cleaner">
            <span class="nav-icon">🧹</span> <span>${t("nav.cleaner")}</span>
          </div>
          <div class="nav-item ${activeTab === 'settings' ? 'active' : ''}" data-tab="settings">
            <span class="nav-icon">⚙️</span> <span>${t("nav.settings")}</span>
          </div>

          <div class="sidebar-footer">
            ${renderLanguageSelectHtml("sel-sidebar-lang", "width:100%;padding:5px 8px;font-size:12px;")}
            <button class="btn btn-sm" id="btn-toggle-app-theme" style="width:100%;">
              ${isDark ? t("nav.theme_light") : t("nav.theme_dark")}
            </button>
            <button class="btn btn-sm" id="btn-sidebar-about" style="width:100%;">
              ℹ️ ${t("about.nav_btn")}
            </button>
            <button class="btn btn-sm" id="btn-hide-to-mini" style="width:100%;">
              ${t("nav.hide_app")}
            </button>
          </div>
        </aside>

        <main class="content-area" id="main-view"></main>
      </div>
    `;

    bindLanguageSelect("sel-sidebar-lang");
    document.getElementById("sidebar-brand-about").onclick = () => showAboutModalDialog();
    document.getElementById("btn-sidebar-about").onclick = () => showAboutModalDialog();
    document.getElementById("btn-toggle-app-theme").onclick = () => {
      toggleAppTheme();
      renderShell();
      renderActiveTab();
    };
    document.getElementById("btn-hide-to-mini").onclick = () => api("hide_launcher");

    root.querySelectorAll(".nav-item").forEach(item => {
      item.addEventListener("click", () => {
        activeTab = item.dataset.tab;
        root.querySelectorAll(".nav-item").forEach(i => i.classList.remove("active"));
        item.classList.add("active");
        renderActiveTab();
      });
    });
  }

  async function renderActiveTab() {
    const view = document.getElementById("main-view");
    if (!view) return;
    if (activeTab !== "monitor") {
      hardwareMonitorScreen.stopPolling();
    }
    if (activeTab !== "mouse") {
      mouseScreen.stopPolling();
    }
    if (activeTab !== "cleaner") {
      systemCleanerScreen.stopPolling();
    }

    if (activeTab === "display") {
      await renderDisplayAndAudioScreen(view, {
        onThemeChanged: () => {
          renderShell();
          renderActiveTab();
        }
      });
    } else if (activeTab === "monitor") {
      await hardwareMonitorScreen.render(view);
    } else if (activeTab === "clipboard") {
      await clipboardScreen.render(view);
    } else if (activeTab === "tiling") {
      await renderWindowTilingScreen(view);
    } else if (activeTab === "mouse") {
      await mouseScreen.render(view);
    } else if (activeTab === "cleaner") {
      await systemCleanerScreen.render(view);
    } else if (activeTab === "settings") {
      await renderUnifiedSettingsScreen(view);
    }
  }

  window.__openDashboardTab = (tabName) => {
    activeTab = tabName;
    renderShell();
    renderActiveTab();
  };
  window.switchMainTab = window.__openDashboardTab;

  window.__onThemeChanged = () => {
    renderShell();
    renderActiveTab();
  };

  window.__onLangChanged = () => {
    renderShell();
    renderActiveTab();
  };

  renderShell();
  renderActiveTab();
}

// =============================================================================
// APPLICATION BOOTSTRAP ROUTER
// =============================================================================
if (APP_MODE === "mini_widget") {
  initMiniLeftWidget();
} else if (APP_MODE === "clipboard_drawer") {
  initClipboardDrawer();
} else if (APP_MODE === "alttab") {
  initAltTabOverlay();
} else if (APP_MODE === "screenshot_overlay") {
  initScreenshotOverlay();
} else {
  initMainDashboard();
}
