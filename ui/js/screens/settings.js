// =============================================================================
// SCREEN 6: UNIFIED SETTINGS (SUB-TABS: SHORTCUTS & FEATURES | SYSTEM, PERMISSIONS & ABOUT)
// =============================================================================
let settingsSubTab = "shortcuts";

async function renderUnifiedSettingsScreen(view) {
  const [cfg, auto, perms, dispState] = await Promise.all([
    api("get_input_config"),
    api("get_autostart_status"),
    api("get_permissions_status"),
    api("get_display_state")
  ]);
  const displays = (dispState && dispState.displays) ? dispState.displays : [];

  view.innerHTML = `
    <div class="page-header">
      <div class="page-title-row">
        <div class="page-title">${t("settings.page_title")}</div>
        ${renderTip(t("settings.page_tip"))}
      </div>
      <div style="display:flex;align-items:center;gap:8px;">
        <button class="btn btn-sm" id="btn-show-about-dialog">
           ${t("about.btn_open")}
        </button>
        <button class="btn btn-sm btn-primary" id="btn-header-github" title="https://github.com/boykhacthang/mac-omni-tool">
          ⭐ GitHub (@boykhacthang)
        </button>
      </div>
    </div>

    <div class="subtab-bar">
      <div class="pill-tabs" id="settings-subtabs">
        <button class="pill-tab ${settingsSubTab === 'shortcuts' ? 'active' : ''}" data-sub="shortcuts">
          ${t("settings.tab_shortcuts")}
        </button>
        <button class="pill-tab ${settingsSubTab === 'system' ? 'active' : ''}" data-sub="system">
          ${t("settings.tab_system")}
        </button>
      </div>
      <div class="subtab-bar-right">
        ${settingsSubTab === 'shortcuts' ? `
          <button class="btn btn-sm btn-primary" id="btn-test-screenshot">
            ${t("settings.btn_test_screenshot", { shortcut: escapeHtml(cfg.screenshot_shortcut || 'Cmd+Shift+0') })}
          </button>
          <button class="btn btn-sm" id="btn-test-mc">${t("settings.btn_test_f3")}</button>
          <button class="btn btn-sm" id="btn-launch-alttab-hud">${t("settings.btn_test_alttab")}</button>
        ` : `
          <button class="btn btn-sm btn-primary" id="btn-reset-tcc">${t("settings.btn_refresh_perms")}</button>
        `}
      </div>
    </div>

    ${settingsSubTab === 'shortcuts' ? `
      <div class="grid-2">
        <!-- FEATURES & KEYBOARD SHORTCUTS -->
        <div class="card" style="margin-bottom:0;">
          <div class="card-header">
            <div class="card-title">
              ${t("settings.card_features_title")}
              ${renderTip(t("settings.card_features_tip"))}
            </div>
          </div>

          <!-- LANGUAGE SELECTOR (DROPDOWN) -->
          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="display:inline-flex;align-items:center;gap:5px;font-weight:600;">
              🌐 ${t("settings.lang_row_label")}
            </span>
            ${renderLanguageSelectHtml("sel-settings-lang", "min-width:155px;")}
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="display:inline-flex;align-items:center;gap:5px;font-weight:600;color:var(--accent);">
              ${t("settings.feat_screenshot_title")}
              ${renderTip(t("settings.feat_screenshot_tip"))}
            </span>
            <div style="display:flex;align-items:center;gap:8px;">
              <span class="stat-sub">${t("settings.master_label")}</span>
              <label class="switch">
                <input type="checkbox" id="chk-screenshot-enabled" ${cfg.screenshot_enabled !== false ? 'checked' : ''} />
                <span class="slider-toggle"></span>
              </label>
            </div>
          </div>

          <div style="padding:6px 10px;margin:6px 0;border-radius:8px;background:var(--bg-input);border:1px solid var(--border);">
            <!-- Mode 1: Region Capture -->
            <div class="setting-row" style="padding:4px 0;border-bottom:1px solid var(--border);">
              <div style="display:flex;flex-direction:column;gap:1px;">
                <span style="font-size:12px;font-weight:600;">${t("settings.mode1_title")}</span>
                <span style="font-size:10.5px;color:var(--text-muted);">${t("settings.mode1_sub")}</span>
              </div>
              <div style="display:flex;align-items:center;gap:6px;">
                <button class="btn btn-sm" id="btn-try-shot-region" title="${t('settings.tip_test_mode1')}">${t("common.test")}</button>
                <div class="shortcut-recorder sys-shortcut" data-field="screenshot_shortcut">${escapeHtml(cfg.screenshot_shortcut || 'Cmd+Shift+0')}</div>
                <label class="switch">
                  <input type="checkbox" id="chk-shot-region-enabled" ${cfg.screenshot_region_enabled !== false ? 'checked' : ''} />
                  <span class="slider-toggle"></span>
                </label>
              </div>
            </div>

            <!-- Mode 2: Fullscreen + Annotation -->
            <div class="setting-row" style="padding:4px 0;border-bottom:1px solid var(--border);">
              <div style="display:flex;flex-direction:column;gap:1px;">
                <span style="font-size:12px;font-weight:600;">${t("settings.mode2_title")}</span>
                <span style="font-size:10.5px;color:var(--text-muted);">${t("settings.mode2_sub")}</span>
              </div>
              <div style="display:flex;align-items:center;gap:6px;">
                <button class="btn btn-sm" id="btn-try-shot-fulledit" title="${t('settings.tip_test_mode2')}">${t("common.test")}</button>
                <div class="shortcut-recorder sys-shortcut" data-field="screenshot_fullscreen_edit_shortcut">${escapeHtml(cfg.screenshot_fullscreen_edit_shortcut || 'Cmd+Shift+9')}</div>
                <label class="switch">
                  <input type="checkbox" id="chk-shot-fulledit-enabled" ${cfg.screenshot_fullscreen_edit_enabled !== false ? 'checked' : ''} />
                  <span class="slider-toggle"></span>
                </label>
              </div>
            </div>

            <!-- Mode 3: Fullscreen + Instant Copy -->
            <div class="setting-row" style="padding:4px 0;border-bottom:1px solid var(--border);">
              <div style="display:flex;flex-direction:column;gap:1px;">
                <span style="font-size:12px;font-weight:600;">${t("settings.mode3_title")}</span>
                <span style="font-size:10.5px;color:var(--text-muted);">${t("settings.mode3_sub")}</span>
              </div>
              <div style="display:flex;align-items:center;gap:6px;">
                <button class="btn btn-sm" id="btn-try-shot-fullcopy" title="${t('settings.tip_test_mode3')}">${t("common.test")}</button>
                <div class="shortcut-recorder sys-shortcut" data-field="screenshot_fullscreen_copy_shortcut">${escapeHtml(cfg.screenshot_fullscreen_copy_shortcut || 'Cmd+Shift+8')}</div>
                <label class="switch">
                  <input type="checkbox" id="chk-shot-fullcopy-enabled" ${cfg.screenshot_fullscreen_copy_enabled !== false ? 'checked' : ''} />
                  <span class="slider-toggle"></span>
                </label>
              </div>
            </div>

            <!-- Cursor-based Display Detection -->
            <div class="setting-row" style="padding:4px 0 2px;">
              <span style="font-size:11.5px;color:var(--text-muted);font-weight:600;">${t("settings.target_monitor_label")}</span>
              <span class="badge-ok" style="font-size:11px;">${t("settings.target_monitor_badge", { count: displays.length })}</span>
            </div>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="font-weight:500;">${t("tiling.page_title")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-snapping-enabled" ${cfg.window_snapping_enabled !== false ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="font-weight:500;">📋 Clipboard</span>
            <div style="display:flex;align-items:center;gap:8px;">
              <div class="shortcut-recorder sys-shortcut" data-field="clipboard_popup_shortcut">${escapeHtml(cfg.clipboard_popup_shortcut || 'Cmd+Shift+V')}</div>
              <label class="switch">
                <input type="checkbox" id="chk-clipboard-enabled" ${cfg.clipboard_enabled !== false ? 'checked' : ''} />
                <span class="slider-toggle"></span>
              </label>
            </div>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="display:inline-flex;align-items:center;gap:5px;font-weight:500;">
              🪟 Alt-Tab
              ${renderTip(t("settings.alttab_tip"))}
            </span>
            <div style="display:flex;align-items:center;gap:8px;">
              <div class="shortcut-recorder sys-shortcut" data-field="alt_tab_shortcut">${escapeHtml(cfg.alt_tab_shortcut || 'Cmd+Tab')}</div>
              <label class="switch">
                <input type="checkbox" id="chk-replace-cmdtab" ${cfg.replace_macos_cmd_tab !== false ? 'checked' : ''} />
                <span class="slider-toggle"></span>
              </label>
            </div>
          </div>

          <div class="setting-row">
            <span style="display:inline-flex;align-items:center;gap:5px;font-weight:500;">
              ${t("settings.finder_cut_title")}
              ${renderTip(t("settings.finder_cut_tip"))}
            </span>
            <label class="switch">
              <input type="checkbox" id="chk-finder-cut" ${cfg.finder_cut_paste_enabled ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>
        </div>

        <!-- EXTERNAL MOUSE, REVERSE SCROLL, SMOOTH SCROLL & F3 MISSION CONTROL -->
        <div class="card" style="margin-bottom:0;">
          <div class="card-header">
            <div class="card-title">
              ${t("settings.card_mouse_f3_title")}
              ${renderTip(t("settings.card_mouse_f3_tip"))}
            </div>
            <button class="btn btn-sm btn-primary" id="btn-open-mouse-tab">
              ${t("settings.btn_per_app_mouse")}
            </button>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="font-weight:600;color:var(--accent);">${t("settings.ext_mouse_master")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-settings-ext-mouse" ${cfg.external_mouse_enabled !== false ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span>${t("settings.reverse_mouse_scroll")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-settings-invert-scroll" ${cfg.invert_external_mouse_scroll ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span>${t("settings.smooth_scroll_120hz")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-settings-smooth-scroll" ${cfg.smooth_scroll_enabled !== false ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span>${t("settings.left_right_chord")}</span>
            <div style="display:flex;align-items:center;gap:8px;">
              <select class="select-box" id="sel-mouse-action" style="min-width:150px;">
                <option value="mission_control" ${cfg.mouse_chord_action === 'mission_control' ? 'selected' : ''}>🚀 Mission Control (F3)</option>
                <option value="app_expose" ${cfg.mouse_chord_action === 'app_expose' ? 'selected' : ''}>🪟 App Exposé</option>
                <option value="show_desktop" ${cfg.mouse_chord_action === 'show_desktop' ? 'selected' : ''}>${t("settings.chord_show_desktop")}</option>
                <option value="launchpad" ${cfg.mouse_chord_action === 'launchpad' ? 'selected' : ''}>🎛️ Launchpad</option>
                <option value="clipboard_popup" ${cfg.mouse_chord_action === 'clipboard_popup' ? 'selected' : ''}>${t("settings.chord_clipboard")}</option>
                <option value="alt_tab" ${cfg.mouse_chord_action === 'alt_tab' ? 'selected' : ''}>🪟 Alt-Tab</option>
                <option value="screenshot" ${cfg.mouse_chord_action === 'screenshot' ? 'selected' : ''}>${t("settings.chord_screenshot")}</option>
              </select>
              <label class="switch">
                <input type="checkbox" id="chk-mouse-chord" ${cfg.mouse_chord_enabled ? 'checked' : ''} />
                <span class="slider-toggle"></span>
              </label>
            </div>
          </div>

          <div style="padding:8px 0;border-bottom:1px solid var(--border);">
            <div style="display:flex;justify-content:space-between;margin-bottom:4px;">
              <span>${t("settings.chord_sensitivity")}</span>
              <b id="lbl-chord-ms">${cfg.mouse_chord_threshold_ms} ms</b>
            </div>
            <input type="range" min="50" max="250" step="10" value="${cfg.mouse_chord_threshold_ms}" id="slider-chord-ms" />
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span>${t("settings.f3_key_label")}</span>
            <div class="shortcut-recorder sys-shortcut" data-field="mission_control_shortcut">${escapeHtml(cfg.mission_control_shortcut)}</div>
          </div>

          <div class="setting-row">
            <span>${t("settings.show_desktop_key_label")}</span>
            <div class="shortcut-recorder sys-shortcut" data-field="show_desktop_shortcut">${escapeHtml(cfg.show_desktop_shortcut)}</div>
          </div>
        </div>
      </div>
    ` : `
      <div class="grid-2">
        <!-- LAUNCH AT LOGIN & LANGUAGE -->
        <div class="card">
          <div class="card-header">
            <div class="card-title">${t("settings.card_startup_lang_title")}</div>
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span style="display:inline-flex;align-items:center;gap:5px;font-weight:600;">
              🌐 ${t("settings.lang_row_label")}
            </span>
            ${renderLanguageSelectHtml("sel-settings-lang", "min-width:155px;")}
          </div>

          <div class="setting-row" style="border-bottom:1px solid var(--border);">
            <span>${t("settings.launch_at_login")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-autostart" ${auto.enabled ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <div class="setting-row">
            <span>${t("settings.start_minimized")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-autostart-min" ${auto.start_minimized ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>
        </div>

        <!-- SYSTEM PERMISSIONS -->
        <div class="card">
          <div class="card-header">
            <div class="card-title">
              ${t("settings.card_perms_title")}
              ${renderTip(t("settings.card_perms_tip"))}
            </div>
            <button class="btn btn-sm" id="btn-recheck-perms">${t("settings.btn_recheck")}</button>
          </div>

          <table>
            <tbody>
              <tr>
                <td style="font-weight:600;">Accessibility</td>
                <td><span class="${perms.accessibility_granted ? 'badge-ok' : 'badge-warn'}">${perms.accessibility_granted ? t("settings.perm_granted") : t("settings.perm_not_granted")}</span></td>
                <td style="text-align:right;"><button class="btn btn-sm perm-btn" data-kind="accessibility">${t("settings.btn_grant")}</button></td>
              </tr>
              <tr>
                <td style="font-weight:600;">Screen Recording</td>
                <td><span class="${perms.screen_recording_granted ? 'badge-ok' : 'badge-warn'}">${perms.screen_recording_granted ? t("settings.perm_granted") : t("settings.perm_not_granted")}</span></td>
                <td style="text-align:right;"><button class="btn btn-sm perm-btn" data-kind="screen_recording">${t("settings.btn_grant")}</button></td>
              </tr>
              <tr>
                <td style="font-weight:600;">Full Disk Access</td>
                <td><span class="${perms.full_disk_access_granted ? 'badge-ok' : 'badge-warn'}">${perms.full_disk_access_granted ? t("settings.perm_granted") : t("settings.perm_optional")}</span></td>
                <td style="text-align:right;"><button class="btn btn-sm perm-btn" data-kind="full_disk">${t("settings.btn_open_settings")}</button></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- ABOUT, AUTHOR & FREE COMMUNITY EDITION / DONATE -->
      <div class="card" style="margin-bottom:0;border:1px solid rgba(59,130,246,0.28);">
        <div class="card-header" style="flex-wrap:wrap;gap:10px;margin-bottom:6px;">
          <div class="card-title" style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
            <span>💡 ${t("settings.about_community_title")}</span>
            <span class="badge-ok">v2.0.0 • MIT License</span>
          </div>
          <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
            <button class="btn btn-sm btn-primary" id="btn-check-update-settings">
              ${t("update.btn_check")}
            </button>
            <button class="btn btn-sm" id="btn-open-github" title="https://github.com/boykhacthang/mac-omni-tool">
              ⭐ GitHub (boykhacthang/mac-omni-tool)
            </button>
            <button
              class="btn btn-sm"
              id="btn-kofi-support"
              title="https://ko-fi.com/I1R72A1475"
              style="background:#72a4f2;border-color:#72a4f2;color:#ffffff;font-weight:700;padding:5px 12px;border-radius:7px;display:inline-flex;align-items:center;gap:6px;box-shadow:0 2px 8px rgba(114,164,242,0.35);"
            >
              <img src="https://storage.ko-fi.com/cdn/cup-border.png" alt="Ko-fi" style="height:15px;width:auto;" onerror="this.style.display='none'" />
              <span>Support me on Ko-fi</span>
            </button>
          </div>
        </div>
        <div style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px;font-size:12px;color:var(--text-muted);">
          <div>
            👨‍💻 ${t("settings.author_label")} <b style="color:var(--text-main);">boykhacthang</b> •
            ${t("settings.about_community_desc")}
          </div>
          <div>
            ${t("settings.about_kofi_cta")}
          </div>
        </div>
      </div>
    `}
  `;

  view.querySelectorAll("#settings-subtabs .pill-tab").forEach((btn) => {
    btn.addEventListener("click", () => {
      settingsSubTab = btn.dataset.sub;
      renderUnifiedSettingsScreen(view);
    });
  });

  bindLanguageSelect("sel-settings-lang");
  const aboutBtn = document.getElementById("btn-show-about-dialog");
  if (aboutBtn) {
    aboutBtn.onclick = () => {
      if (typeof window.__showAboutModal === "function") {
        window.__showAboutModal();
      }
    };
  }
  const checkUpdSettingsBtn = document.getElementById("btn-check-update-settings");
  if (checkUpdSettingsBtn) {
    checkUpdSettingsBtn.onclick = () => {
      if (typeof window.__showAboutModal === "function") {
        window.__showAboutModal();
      }
    };
  }
  const headerGhBtn = document.getElementById("btn-header-github");
  if (headerGhBtn) {
    headerGhBtn.onclick = () => api("open_external_url", { url: "https://github.com/boykhacthang/mac-omni-tool" });
  }
  const githubBtn = document.getElementById("btn-open-github");
  if (githubBtn) {
    githubBtn.onclick = () => api("open_external_url", { url: "https://github.com/boykhacthang/mac-omni-tool" });
  }
  const kofiBtn = document.getElementById("btn-kofi-support");
  if (kofiBtn) {
    kofiBtn.onclick = () => api("open_external_url", { url: "https://ko-fi.com/I1R72A1475" });
  }

  const saveCfg = async (patch = {}) => {
    const chkVal = (id, fallback) => {
      const el = document.getElementById(id);
      return el ? el.checked : fallback;
    };
    const selVal = (id, fallback) => {
      const el = document.getElementById(id);
      return el ? el.value : fallback;
    };
    const numVal = (id, fallback) => {
      const el = document.getElementById(id);
      return el ? Number(el.value) : fallback;
    };

    const updated = {
      ...cfg,
      screenshot_enabled: chkVal("chk-screenshot-enabled", cfg.screenshot_enabled !== false),
      screenshot_region_enabled: chkVal("chk-shot-region-enabled", cfg.screenshot_region_enabled !== false),
      screenshot_fullscreen_edit_enabled: chkVal("chk-shot-fulledit-enabled", cfg.screenshot_fullscreen_edit_enabled !== false),
      screenshot_fullscreen_copy_enabled: chkVal("chk-shot-fullcopy-enabled", cfg.screenshot_fullscreen_copy_enabled !== false),
      screenshot_display_id: 0,
      window_snapping_enabled: chkVal("chk-snapping-enabled", cfg.window_snapping_enabled !== false),
      clipboard_enabled: chkVal("chk-clipboard-enabled", cfg.clipboard_enabled !== false),
      replace_macos_cmd_tab: chkVal("chk-replace-cmdtab", cfg.replace_macos_cmd_tab !== false),
      finder_cut_paste_enabled: chkVal("chk-finder-cut", !!cfg.finder_cut_paste_enabled),
      external_mouse_enabled: chkVal("chk-settings-ext-mouse", cfg.external_mouse_enabled !== false),
      invert_external_mouse_scroll: chkVal("chk-settings-invert-scroll", !!cfg.invert_external_mouse_scroll),
      smooth_scroll_enabled: chkVal("chk-settings-smooth-scroll", cfg.smooth_scroll_enabled !== false),
      mouse_chord_enabled: chkVal("chk-mouse-chord", !!cfg.mouse_chord_enabled),
      mouse_chord_action: selVal("sel-mouse-action", cfg.mouse_chord_action),
      mouse_chord_threshold_ms: numVal("slider-chord-ms", cfg.mouse_chord_threshold_ms),
      ...patch
    };
    await api("update_input_config", updated);
    showToast(t("settings.toast_saved"));
    renderUnifiedSettingsScreen(view);
  };

  const openMouseBtn = document.getElementById("btn-open-mouse-tab");
  if (openMouseBtn) {
    openMouseBtn.onclick = () => {
      if (typeof window.__openDashboardTab === "function") {
        window.__openDashboardTab("mouse");
      }
    };
  }

  const bindChange = (id, fn) => {
    const el = document.getElementById(id);
    if (el) el.onchange = fn;
  };
  const bindClick = (id, fn) => {
    const el = document.getElementById(id);
    if (el) el.onclick = fn;
  };

  bindChange("chk-settings-ext-mouse", () => saveCfg());
  bindChange("chk-settings-invert-scroll", () => saveCfg());
  bindChange("chk-settings-smooth-scroll", () => saveCfg());

  bindClick("btn-test-screenshot", () => api("trigger_screenshot"));
  bindClick("btn-try-shot-region", () => api("trigger_screenshot"));
  bindClick("btn-try-shot-fulledit", () => {
    api("trigger_fullscreen_screenshot", { display_id: 0, instant_copy: false });
  });
  bindClick("btn-try-shot-fullcopy", () => {
    api("trigger_fullscreen_screenshot", { display_id: 0, instant_copy: true });
  });
  bindClick("btn-test-mc", () => api("trigger_system_action", { action: "mission_control" }));
  bindClick("btn-launch-alttab-hud", () => api("toggle_overlay", { window: "alttab" }));

  bindChange("chk-screenshot-enabled", () => saveCfg());
  bindChange("chk-shot-region-enabled", () => saveCfg());
  bindChange("chk-shot-fulledit-enabled", () => saveCfg());
  bindChange("chk-shot-fullcopy-enabled", () => saveCfg());
  bindChange("chk-snapping-enabled", () => saveCfg());
  bindChange("chk-clipboard-enabled", () => saveCfg());
  bindChange("chk-replace-cmdtab", () => saveCfg());
  bindChange("chk-finder-cut", () => saveCfg());
  bindChange("chk-mouse-chord", () => saveCfg());
  bindChange("sel-mouse-action", () => saveCfg());

  const sliderChord = document.getElementById("slider-chord-ms");
  if (sliderChord) {
    sliderChord.oninput = (e) => {
      const lbl = document.getElementById("lbl-chord-ms");
      if (lbl) lbl.textContent = e.target.value + " ms";
    };
    sliderChord.onchange = () => saveCfg();
  }

  view.querySelectorAll(".sys-shortcut").forEach(box => {
    box.addEventListener("click", () => {
      box.classList.add("recording");
      box.textContent = t("common.press_keys");
      const handler = async (e) => {
        if (e.key === "Escape") {
          window.removeEventListener("keydown", handler, true);
          renderUnifiedSettingsScreen(view);
          return;
        }
        const combo = eventToShortcutString(e);
        if (!combo) return;
        window.removeEventListener("keydown", handler, true);
        await saveCfg({ [box.dataset.field]: combo });
      };
      window.addEventListener("keydown", handler, true);
    });
  });

  const updateAuto = async () => {
    const chkAuto = document.getElementById("chk-autostart");
    const chkMin = document.getElementById("chk-autostart-min");
    const enabled = chkAuto ? chkAuto.checked : !!auto.enabled;
    const start_minimized = chkMin ? chkMin.checked : !!auto.start_minimized;
    await api("set_autostart_config", { enabled, start_minimized });
    showToast(enabled
      ? t("settings.toast_launch_on")
      : t("settings.toast_launch_off")
    );
    renderUnifiedSettingsScreen(view);
  };

  bindChange("chk-autostart", updateAuto);
  bindChange("chk-autostart-min", updateAuto);
  bindClick("btn-recheck-perms", () => renderUnifiedSettingsScreen(view));
  bindClick("btn-reset-tcc", async () => {
    await api("reset_permissions");
    showToast(t("settings.toast_refreshed_tcc"));
    setTimeout(() => renderUnifiedSettingsScreen(view), 1000);
  });

  view.querySelectorAll(".perm-btn").forEach(btn => {
    btn.addEventListener("click", () => api("open_permission", { kind: btn.dataset.kind }));
  });
}
