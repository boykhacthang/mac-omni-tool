// =============================================================================
// MAIN DASHBOARD SHELL, ABOUT MODAL, UPDATE NOTIFIER & MULTI-WINDOW MODE ROUTER
// =============================================================================
let cachedUpdateState = null;
let dismissedUpdateBannerForSession = false;

function resetSidebarAboutButton() {
  const aboutBtn = document.getElementById("btn-sidebar-about");
  if (aboutBtn) {
    aboutBtn.classList.remove("btn-primary");
    aboutBtn.innerHTML = `ℹ️ ${t("about.nav_btn")}`;
  }
}

function hideUpdateNotificationBanner() {
  const banner = document.getElementById("update-notify-banner");
  if (banner) {
    banner.style.display = "none";
    banner.innerHTML = "";
  }
}

function renderUpdateNotificationBanner(res) {
  const banner = document.getElementById("update-notify-banner");
  if (!banner || !res || !res.has_update || dismissedUpdateBannerForSession) {
    return;
  }
  banner.style.display = "block";
  banner.innerHTML = `
    <div style="margin:0 0 12px 0;padding:10px 14px;border-radius:10px;background:rgba(59,130,246,0.12);border:1px solid rgba(59,130,246,0.42);display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap;">
      <div>
        <div style="font-weight:700;font-size:13px;color:var(--accent);">
          ${escapeHtml(t("update.banner_title", { latest: res.latest_version, current: res.current_version }))}
        </div>
        <div class="stat-sub" style="margin-top:2px;">
          ${escapeHtml(t("update.banner_sub"))}
        </div>
      </div>
      <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
        <button class="btn btn-sm btn-primary" id="btn-banner-open-update">
          ${t("update.banner_btn_view")}
        </button>
        <button class="btn btn-sm" id="btn-banner-skip-version">
          ${escapeHtml(t("update.btn_keep_current", { latest: res.latest_version }))}
        </button>
        <button class="btn btn-sm" id="btn-banner-remind-later">
          ${t("update.banner_btn_later")}
        </button>
      </div>
    </div>
  `;

  const viewBtn = document.getElementById("btn-banner-open-update");
  if (viewBtn) {
    viewBtn.onclick = () => showAboutModalDialog();
  }
  const laterBtn = document.getElementById("btn-banner-remind-later");
  if (laterBtn) {
    laterBtn.onclick = () => {
      dismissedUpdateBannerForSession = true;
      hideUpdateNotificationBanner();
    };
  }
  const skipBtn = document.getElementById("btn-banner-skip-version");
  if (skipBtn) {
    skipBtn.onclick = async () => {
      const latestCfg = await api("get_input_config");
      await api("update_input_config", {
        ...latestCfg,
        skipped_update_version: res.latest_version
      });
      dismissedUpdateBannerForSession = true;
      hideUpdateNotificationBanner();
      resetSidebarAboutButton();
      showToast(t("update.toast_kept_current", { latest: res.latest_version }));
    };
  }
}

function renderKeepCurrentModeInModal(curVer) {
  const box = document.getElementById("about-update-status-box");
  if (!box) return;
  box.innerHTML = `
    <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap;">
      <span class="badge-ok" style="font-size:11.5px;padding:3px 9px;">
        ${escapeHtml(t("update.keep_current_mode_badge", { version: curVer }))}
      </span>
      <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
        <button class="btn btn-sm btn-primary" id="btn-manual-check-once">${t("update.btn_check")}</button>
        <button class="btn btn-sm" id="btn-reenable-auto-check">${t("update.btn_enable_checks")}</button>
      </div>
    </div>
  `;
  const reenableBtn = document.getElementById("btn-reenable-auto-check");
  if (reenableBtn) {
    reenableBtn.onclick = async () => {
      const cfg = await api("get_input_config");
      await api("update_input_config", {
        ...cfg,
        auto_check_updates: true,
        skipped_update_version: ""
      });
      const chk = document.getElementById("chk-modal-auto-update");
      if (chk) chk.checked = true;
      showToast(t("update.toast_enabled_checks"));
      runUpdateCheckInModal(true);
    };
  }
  const manualBtn = document.getElementById("btn-manual-check-once");
  if (manualBtn) {
    manualBtn.onclick = () => runUpdateCheckInModal(true);
  }
}

async function runUpdateCheckInModal(forceManual = false) {
  const box = document.getElementById("about-update-status-box");
  if (!box) return;

  const cfg = await api("get_input_config");
  const curVerFallback = (cachedUpdateState && cachedUpdateState.current_version) || "2.3.1";
  const chk = document.getElementById("chk-modal-auto-update");
  if (chk && cfg) {
    chk.checked = cfg.auto_check_updates !== false;
  }

  if (!forceManual && cfg && cfg.auto_check_updates === false) {
    renderKeepCurrentModeInModal(curVerFallback);
    return;
  }

  let res = (!forceManual && cachedUpdateState && cachedUpdateState.ok) ? cachedUpdateState : null;
  if (!res) {
    box.innerHTML = `<div class="stat-sub">${t("update.checking")}</div>`;
    res = await api("check_for_updates");
    cachedUpdateState = res;
  }
  if (!document.getElementById("about-update-status-box")) return;

  if (!res || !res.ok) {
    const errMsg = (res && res.error) ? res.error : "Network error";
    box.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap;">
        <span class="badge-warn">${escapeHtml(t("update.toast_failed", { error: errMsg }))}</span>
        <button class="btn btn-sm" id="btn-retry-update-check">${t("update.btn_check")}</button>
      </div>
    `;
    const retryBtn = document.getElementById("btn-retry-update-check");
    if (retryBtn) retryBtn.onclick = () => runUpdateCheckInModal(true);
    return;
  }

  if (res.has_update) {
    const isSkipped = cfg && cfg.skipped_update_version === res.latest_version;
    const notes = getAppLang() === "vi" ? (res.notes_vi || res.notes_en || "") : (res.notes_en || res.notes_vi || "");
    box.innerHTML = `
      <div style="display:flex;flex-direction:column;gap:8px;">
        <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap;">
          <span class="badge-ok" style="font-size:12px;padding:3px 9px;">
            ${escapeHtml(isSkipped
      ? t("update.skipped_badge", { latest: res.latest_version, current: res.current_version })
      : t("update.available_badge", { latest: res.latest_version, current: res.current_version }))}
          </span>
          <div style="display:flex;align-items:center;gap:6px;">
            ${res.published_at ? `<span class="stat-sub">${escapeHtml(res.published_at)}</span>` : ""}
            <button class="btn btn-sm" id="btn-recheck-update-when-avail" style="padding:2px 7px;font-size:11px;">${t("update.btn_check")}</button>
          </div>
        </div>
        ${notes ? `
          <div style="font-size:11.5px;color:var(--text-muted);background:var(--bg-card);border:1px solid var(--border);border-radius:7px;padding:7px 10px;max-height:85px;overflow-y:auto;white-space:pre-line;">${escapeHtml(notes)}</div>
        ` : ""}
        <div id="update-confirm-box" style="display:none;padding:9px 11px;border-radius:8px;background:rgba(245,158,11,0.12);border:1px solid rgba(245,158,11,0.45);">
          <div style="font-size:12px;font-weight:600;margin-bottom:8px;color:var(--text-main);">
            ${escapeHtml(t("update.confirm_prompt", { latest: res.latest_version }))}
          </div>
          <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
            <button class="btn btn-sm btn-primary" id="btn-confirm-self-update">
              ${t("update.btn_confirm_yes")}
            </button>
            <button class="btn btn-sm" id="btn-cancel-self-update">
              ${t("update.btn_confirm_cancel")}
            </button>
          </div>
        </div>
        <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;" id="update-action-buttons-row">
          <button class="btn btn-sm btn-primary" id="btn-perform-self-update">
            ${escapeHtml(t("update.btn_install_now", { latest: res.latest_version }))}
          </button>
          <button class="btn btn-sm" id="btn-download-update-dmg">
            ${t("update.btn_download_dmg")}
          </button>
          ${!isSkipped ? `
            <button class="btn btn-sm" id="btn-keep-current-version">
              ${escapeHtml(t("update.btn_keep_current", { latest: res.latest_version }))}
            </button>
          ` : ""}
          <button class="btn btn-sm" id="btn-disable-update-checks">
            ${t("update.btn_disable_checks")}
          </button>
        </div>
      </div>
    `;

    const recheckAvailBtn = document.getElementById("btn-recheck-update-when-avail");
    if (recheckAvailBtn) {
      recheckAvailBtn.onclick = () => runUpdateCheckInModal(true);
    }

    const installBtn = document.getElementById("btn-perform-self-update");
    const confirmBox = document.getElementById("update-confirm-box");
    const actionsRow = document.getElementById("update-action-buttons-row");
    const confirmYesBtn = document.getElementById("btn-confirm-self-update");
    const confirmCancelBtn = document.getElementById("btn-cancel-self-update");

    if (installBtn && confirmBox && actionsRow) {
      installBtn.onclick = () => {
        actionsRow.style.display = "none";
        confirmBox.style.display = "block";
      };
    }

    if (confirmCancelBtn && confirmBox && actionsRow) {
      confirmCancelBtn.onclick = () => {
        confirmBox.style.display = "none";
        actionsRow.style.display = "flex";
      };
    }

    if (confirmYesBtn) {
      confirmYesBtn.onclick = async () => {
        if (confirmBox) {
          confirmBox.innerHTML = `<span class="badge-ok">${escapeHtml(t("update.installing", { latest: res.latest_version }))}</span>`;
        }
        const updRes = await api("perform_self_update", {
          zip_url: res.zip_url,
          sha256_zip: res.sha256_zip || ""
        });
        if (!updRes || !updRes.ok) {
          showToast(t("update.toast_failed", { error: (updRes && updRes.error) || "Update failed" }));
          runUpdateCheckInModal(true);
        }
      };
    }

    const dmgBtn = document.getElementById("btn-download-update-dmg");
    if (dmgBtn && res.dmg_url) {
      dmgBtn.onclick = () => api("open_external_url", { url: res.dmg_url });
    }

    const keepBtn = document.getElementById("btn-keep-current-version");
    if (keepBtn) {
      keepBtn.onclick = async () => {
        const latestCfg = await api("get_input_config");
        await api("update_input_config", {
          ...latestCfg,
          skipped_update_version: res.latest_version
        });
        hideUpdateNotificationBanner();
        resetSidebarAboutButton();
        showToast(t("update.toast_kept_current", { latest: res.latest_version }));
        runUpdateCheckInModal(false);
      };
    }

    const disableBtn = document.getElementById("btn-disable-update-checks");
    if (disableBtn) {
      disableBtn.onclick = async () => {
        const latestCfg = await api("get_input_config");
        await api("update_input_config", {
          ...latestCfg,
          auto_check_updates: false,
          skipped_update_version: res.latest_version
        });
        const modalChk = document.getElementById("chk-modal-auto-update");
        if (modalChk) modalChk.checked = false;
        hideUpdateNotificationBanner();
        resetSidebarAboutButton();
        showToast(t("update.toast_disabled_checks"));
        renderKeepCurrentModeInModal(res.current_version);
      };
    }
  } else {
    box.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap;">
        <span class="badge-ok" style="font-size:11.5px;padding:3px 9px;">
          ${escapeHtml(t("update.up_to_date", { version: res.current_version }))}
        </span>
        <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
          <button class="btn btn-sm" id="btn-recheck-update-modal">${t("update.btn_check")}</button>
          <button class="btn btn-sm" id="btn-lock-current-version">${t("update.btn_disable_checks")}</button>
        </div>
      </div>
    `;
    const recheckBtn = document.getElementById("btn-recheck-update-modal");
    if (recheckBtn) recheckBtn.onclick = () => runUpdateCheckInModal(true);
    const lockBtn = document.getElementById("btn-lock-current-version");
    if (lockBtn) {
      lockBtn.onclick = async () => {
        const latestCfg = await api("get_input_config");
        await api("update_input_config", {
          ...latestCfg,
          auto_check_updates: false
        });
        const modalChk = document.getElementById("chk-modal-auto-update");
        if (modalChk) modalChk.checked = false;
        hideUpdateNotificationBanner();
        resetSidebarAboutButton();
        showToast(t("update.toast_disabled_checks"));
        renderKeepCurrentModeInModal(res.current_version);
      };
    }
  }
}

function showAboutModalDialog(forceManualCheck = false) {
  const existing = document.getElementById("about-modal-backdrop");
  if (existing) {
    existing.remove();
  }

  const curVer = (cachedUpdateState && cachedUpdateState.current_version) || "2.3.1";
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
              <span class="badge-ok">v${escapeHtml(curVer)}</span>
              <span>Free Community Edition • MIT License</span>
            </div>
          </div>
        </div>
        <button class="btn btn-sm" id="btn-close-about-x" title="Close">✕</button>
      </div>

      <!-- AUTO-UPDATER SECTION -->
      <div style="background:var(--bg-input);border:1px solid rgba(59,130,246,0.32);border-radius:10px;padding:10px 13px;margin-bottom:12px;">
        <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;margin-bottom:8px;padding-bottom:8px;border-bottom:1px solid var(--border);">
          <div>
            <div style="font-size:12px;font-weight:700;color:var(--accent);">
              ${t("update.section_title")}
            </div>
            <div class="stat-sub" style="margin-top:2px;">
              ${t("update.auto_check_sub")}
            </div>
          </div>
          <label class="switch" title="${t("update.auto_check_label")}">
            <input type="checkbox" id="chk-modal-auto-update" checked />
            <span class="slider"></span>
          </label>
        </div>
        <div id="about-update-status-box">
          <div class="stat-sub">${t("update.checking")}</div>
        </div>
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
  const modalAutoChk = document.getElementById("chk-modal-auto-update");
  if (modalAutoChk) {
    modalAutoChk.onchange = async () => {
      const enabled = modalAutoChk.checked;
      const latestCfg = await api("get_input_config");
      await api("update_input_config", {
        ...latestCfg,
        auto_check_updates: enabled,
        skipped_update_version: enabled ? "" : (latestCfg.skipped_update_version || "")
      });
      if (enabled) {
        showToast(t("update.toast_enabled_checks"));
        runUpdateCheckInModal(true);
      } else {
        hideUpdateNotificationBanner();
        resetSidebarAboutButton();
        showToast(t("update.toast_disabled_checks"));
        renderKeepCurrentModeInModal(curVer);
      }
    };
  }

  runUpdateCheckInModal(!!forceManualCheck);
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

        <main class="content-area">
          <div id="update-notify-banner" style="display:none;"></div>
          <div id="main-view"></div>
        </main>
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

    if (cachedUpdateState && cachedUpdateState.ok && cachedUpdateState.has_update && !dismissedUpdateBannerForSession) {
      renderUpdateNotificationBanner(cachedUpdateState);
      const aboutBtn = document.getElementById("btn-sidebar-about");
      if (aboutBtn) {
        aboutBtn.classList.add("btn-primary");
        aboutBtn.textContent = `🔔 v${cachedUpdateState.latest_version}`;
      }
    }
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

  window.openPermissionsSettings = (missingKind) => {
    window.__settingsInitialSubTab = "system";
    window.__highlightedPermKind = missingKind || "";
    activeTab = "settings";
    renderShell();
    renderActiveTab();
    if (missingKind === "accessibility") {
      showToast(t("perm.toast_need_accessibility"));
    } else if (missingKind === "screen_recording") {
      showToast(t("perm.toast_need_screen_recording"));
    } else {
      showToast(t("perm.toast_need_generic"));
    }
  };

  window.__onMainWindowFocused = () => {
    if (
      activeTab === "settings" ||
      activeTab === "mouse" ||
      activeTab === "tiling" ||
      activeTab === "monitor" ||
      activeTab === "cleaner"
    ) {
      renderActiveTab();
    }
  };

  window.__onMainWindowHidden = () => {
    hardwareMonitorScreen.stopPolling();
    mouseScreen.stopPolling();
    systemCleanerScreen.stopPolling();
  };

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

  // Silent background check 3.5s after launch: ONLY notifies user (never updates automatically)
  setTimeout(async () => {
    try {
      const cfg = await api("get_input_config");
      if (cfg && cfg.auto_check_updates === false) {
        return;
      }
      const res = await api("check_for_updates");
      cachedUpdateState = res;
      if (res && res.ok && res.has_update) {
        if (cfg && cfg.skipped_update_version === res.latest_version) {
          return;
        }
        renderUpdateNotificationBanner(res);
        showToast(t("update.toast_new_version", { latest: res.latest_version }));
        const aboutBtn = document.getElementById("btn-sidebar-about");
        if (aboutBtn) {
          aboutBtn.classList.add("btn-primary");
          aboutBtn.textContent = `🔔 v${res.latest_version}`;
        }
      }
    } catch (_e) {
      // ignore offline
    }
  }, 3500);
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
