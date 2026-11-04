// =============================================================================
// SCREEN: EXTERNAL MOUSE CUSTOMIZATION, REVERSE SCROLL & 120HZ SMOOTH SCROLL
// - Sub-Tab 1 ("buttons"): Interactive SVG Mouse Diagram, Per-App Profiles & Button Mapping
// - Sub-Tab 2 ("scroll"): 120Hz Smooth Scroll Tuning, Reverse Scroll & Live Scroll Test Playground
// =============================================================================
function createMouseScreen({ isTabActive } = {}) {
  let mouseSubTab = "buttons"; // "buttons" | "scroll"
  let pollTimer = null;
  let selectedProfileKey = "__default__"; // "__default__" or app bundle_id
  let selectedDiagramBtn = "forward"; // "forward" | "back" | "middle" | "chord"
  let showAppPickerModal = false;
  let cachedInstalledApps = null;
  let modalSearchQuery = "";
  let lastSeenBtnAt = 0;

  function iconForBundleId(bundleId) {
    const b = (bundleId || "").toLowerCase();
    if (b.includes("chrome") || b.includes("safari")) return "🧭";
    if (b.includes("firefox")) return "🦊";
    if (b.includes("brave")) return "🦁";
    if (b.includes("edgemac")) return "🌊";
    if (b.includes("thebrowser")) return "🌈";
    if (b.includes("vscode") || b.includes("cursor") || b.includes("xcode") || b.includes("jetbrains")) return "💻";
    if (b.includes("terminal") || b.includes("iterm") || b.includes("warp")) return "⌨️";
    if (b.includes("finder")) return "📁";
    if (b.includes("figma") || b.includes("photoshop") || b.includes("illustrator")) return "🎨";
    if (b.includes("slack") || b.includes("discord") || b.includes("telegram") || b.includes("zalo")) return "💬";
    if (b.includes("spotify") || b.includes("music")) return "🎵";
    return "🧩";
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function getActionLabel(action) {
    return tMouseActionLabel(action);
  }

  function renderActionOptionsHtml(selectedValue) {
    const options = [
      "default",
      "space_right",
      "space_left",
      "browser_forward",
      "browser_back",
      "mission_control",
      "app_expose",
      "show_desktop",
      "launchpad",
      "tab_next",
      "tab_prev",
      "close_tab",
      "new_tab",
      "screenshot",
      "screenshot_full_edit",
      "screenshot_full_copy",
      "clipboard_popup",
      "alt_tab",
      "disabled"
    ];
    return options
      .map(val => `<option value="${val}" ${val === selectedValue ? "selected" : ""}>${escapeHtml(getActionLabel(val))}</option>`)
      .join("");
  }

  async function render(view) {
    stopPolling();
    const res = await api("get_mouse_state");
    const cfg = (res && res.config) ? res.config : await api("get_input_config");
    const spaces = (res && res.spaces) ? res.spaces : {
      total_spaces: 1,
      primary_display_spaces: 1,
      current_space_index: 1,
      frontmost_bundle_id: "",
      frontmost_app_name: ""
    };
    const runningApps = (res && res.running_apps) ? res.running_apps : [];
    const profiles = Array.isArray(cfg.app_mouse_profiles) ? cfg.app_mouse_profiles : [];

    if (
      selectedProfileKey !== "__default__" &&
      !profiles.some(p => p.bundle_id === selectedProfileKey)
    ) {
      selectedProfileKey = "__default__";
    }

    const activeProfile =
      selectedProfileKey === "__default__"
        ? null
        : profiles.find(p => p.bundle_id === selectedProfileKey) || null;

    const curForward = activeProfile ? activeProfile.forward_action : (cfg.mouse_forward_action || "space_right");
    const curBack = activeProfile ? activeProfile.back_action : (cfg.mouse_back_action || "space_left");
    const curMiddle = activeProfile ? activeProfile.middle_action : (cfg.mouse_middle_action || "default");

    const diagramBtnTitle = {
      forward: t("mouse.diagram_btn_forward"),
      back: t("mouse.diagram_btn_back"),
      middle: t("mouse.diagram_btn_middle"),
      chord: t("mouse.diagram_btn_chord")
    }[selectedDiagramBtn];

    const diagramBtnActionKey =
      selectedDiagramBtn === "forward"
        ? curForward
        : selectedDiagramBtn === "back"
          ? curBack
          : selectedDiagramBtn === "middle"
            ? curMiddle
            : (cfg.mouse_chord_action || "mission_control");

    const existingBundles = new Set(profiles.map(p => (p.bundle_id || "").toLowerCase()));
    const availableRunningApps = runningApps.filter(
      a => a.bundle_id && !existingBundles.has(a.bundle_id.toLowerCase())
    );

    const masterEnabled = cfg.external_mouse_enabled !== false;
    const buttonsEnabled = masterEnabled && cfg.mouse_buttons_enabled !== false;
    const smoothEnabled = masterEnabled && cfg.smooth_scroll_enabled !== false;
    const chordEnabled = masterEnabled && !!cfg.mouse_chord_enabled;
    const profileActive = !activeProfile || activeProfile.enabled !== false;
    const btnRowControlsEnabled = buttonsEnabled && profileActive;
    const diagramTestEnabled = selectedDiagramBtn === "chord" ? chordEnabled : btnRowControlsEnabled;

    view.innerHTML = `
      <div class="page-header">
        <div class="page-title-row">
          <div class="page-title">${t("mouse.page_title")}</div>
          ${renderTip(t("mouse.page_tip"))}
        </div>
        <div style="display:flex;align-items:center;gap:8px;">
          <button class="btn btn-sm" id="btn-reset-mouse-defaults">
            ${t("mouse.btn_restore_defaults")}
          </button>
          <button class="btn btn-sm btn-primary" id="btn-refresh-mouse">
            ${t("common.refresh")}
          </button>
        </div>
      </div>

      <div class="subtab-bar">
        <div class="pill-tabs" id="mouse-subtabs">
          <button class="pill-tab ${mouseSubTab === 'buttons' ? 'active' : ''}" data-sub="buttons">
            🎯 ${t("mouse.subtab_buttons")}
          </button>
          <button class="pill-tab ${mouseSubTab === 'scroll' ? 'active' : ''}" data-sub="scroll">
            🌊 ${t("mouse.subtab_scroll")}
          </button>
        </div>
        <div class="subtab-bar-right">
          <span class="badge-ok" id="lbl-spaces-live">
            🖥️ ${t("mouse.spaces_live", { current: spaces.current_space_index, total: spaces.total_spaces })}
          </span>
          <span class="stat-sub" id="lbl-last-mouse-btn" style="padding:2px 8px;border-radius:99px;background:var(--bg-input);border:1px solid var(--border);white-space:nowrap;">
            ${spaces.last_mouse_button >= 3
              ? t("mouse.last_pressed", { btn: spaces.last_mouse_button })
              : t("mouse.press_to_test")}
          </span>
        </div>
      </div>

      <!-- TOP 3 COMPACT QUICK SWITCH CARDS: MASTER, REVERSE SCROLL, SMOOTH SCROLL -->
      <div class="grid-3" style="margin-bottom:10px;">
        <!-- 1. Master Switch -->
        <div class="card" style="margin-bottom:0;padding:10px 13px;border-color:${cfg.external_mouse_enabled !== false ? 'rgba(59,130,246,0.35)' : 'var(--border)'};">
          <div class="card-header" style="margin-bottom:2px;">
            <div class="card-title" style="font-size:13px;">
              <span>🖱️ ${t("mouse.card_master_title")}</span>
            </div>
            <label class="switch">
              <input type="checkbox" id="chk-ext-mouse-master" ${cfg.external_mouse_enabled !== false ? "checked" : ""} />
              <span class="slider-toggle"></span>
            </label>
          </div>
          <div class="stat-sub" style="font-size:11px;">
            ${t("mouse.card_master_sub")}
          </div>
        </div>

        <!-- 2. Reverse Scroll (Invert External Mouse Scroll) -->
        <div class="card" style="margin-bottom:0;padding:10px 13px;border-color:${masterEnabled && cfg.invert_external_mouse_scroll ? 'rgba(16,185,129,0.4)' : 'var(--border)'};${masterEnabled ? '' : 'opacity:0.55;'}">
          <div class="card-header" style="margin-bottom:2px;">
            <div class="card-title" style="font-size:13px;">
              <span>🔄 ${t("mouse.card_reverse_title")}</span>
              ${masterEnabled && cfg.invert_external_mouse_scroll
                ? `<span class="badge-ok">${t("mouse.badge_reversed")}</span>`
                : ""}
            </div>
            <label class="switch">
              <input type="checkbox" id="chk-invert-scroll-top" ${cfg.invert_external_mouse_scroll ? "checked" : ""} ${masterEnabled ? "" : "disabled"} />
              <span class="slider-toggle"></span>
            </label>
          </div>
          <div class="stat-sub" style="font-size:11px;">
            ${t("mouse.card_reverse_sub")}
          </div>
        </div>

        <!-- 3. 120Hz Smooth Scroll -->
        <div class="card" style="margin-bottom:0;padding:10px 13px;border-color:${smoothEnabled ? 'rgba(59,130,246,0.35)' : 'var(--border)'};${masterEnabled ? '' : 'opacity:0.55;'}">
          <div class="card-header" style="margin-bottom:2px;">
            <div class="card-title" style="font-size:13px;">
              <span>🌊 ${t("mouse.card_smooth_title")}</span>
              ${smoothEnabled
                ? `<span class="badge-ok">120Hz</span>`
                : ""}
            </div>
            <label class="switch">
              <input type="checkbox" id="chk-smooth-scroll-top" ${cfg.smooth_scroll_enabled !== false ? "checked" : ""} ${masterEnabled ? "" : "disabled"} />
              <span class="slider-toggle"></span>
            </label>
          </div>
          <div class="stat-sub" style="font-size:11px;">
            ${t("mouse.card_smooth_sub")}
          </div>
        </div>
      </div>

      ${mouseSubTab === "buttons" ? `
        <!-- SUB-TAB 1: MOUSE BUTTONS & PER-APP PROFILES -->
        <div class="card" style="margin-bottom:0;${masterEnabled ? '' : 'opacity:0.6;'}">
          <div class="card-header" style="margin-bottom:8px;">
            <div class="card-title">
              <span>🎯 ${t("mouse.buttons_card_title")}</span>
              ${renderTip(t("mouse.buttons_card_tip"))}
            </div>
            <div style="display:flex;align-items:center;gap:8px;">
              <span class="stat-sub">${t("mouse.enable_btn_mapping")}</span>
              <label class="switch">
                <input type="checkbox" id="chk-mouse-buttons-enabled" ${cfg.mouse_buttons_enabled !== false ? "checked" : ""} ${masterEnabled ? "" : "disabled"} />
                <span class="slider-toggle"></span>
              </label>
            </div>
          </div>

          <div class="mouse-config-grid">
            <!-- Left: Interactive SVG Mouse Diagram -->
            <div class="mouse-diagram-box" style="padding:10px;">
              <div style="width:100%;display:flex;justify-content:space-between;align-items:center;">
                <span style="font-size:11.5px;font-weight:600;color:var(--text-muted);">${t("mouse.diagram_layout")}</span>
                <span style="font-size:10.5px;color:var(--accent);font-weight:600;">${t("mouse.diagram_click_btn")}</span>
              </div>

              <svg viewBox="0 0 210 235" style="width:145px;height:164px;">
                <!-- Outer Shell -->
                <rect x="48" y="16" width="116" height="202" rx="56" fill="rgba(148,163,184,0.10)" stroke="rgba(148,163,184,0.42)" stroke-width="2" />

                <!-- Left + Right Chord Area -->
                <path class="mouse-svg-hotspot ${selectedDiagramBtn === 'chord' ? 'active' : ''}" data-btn="chord"
                      d="M 50 76 L 50 92 L 104 92 L 104 18 C 72 18 50 44 50 76 Z"
                      fill="rgba(148,163,184,0.16)" stroke="rgba(148,163,184,0.48)" stroke-width="1.5" />
                <path class="mouse-svg-hotspot ${selectedDiagramBtn === 'chord' ? 'active' : ''}" data-btn="chord"
                      d="M 108 18 L 108 92 L 162 92 L 162 76 C 162 44 140 18 108 18 Z"
                      fill="rgba(148,163,184,0.16)" stroke="rgba(148,163,184,0.48)" stroke-width="1.5" />

                <!-- Middle Scroll Wheel (Mouse 3) -->
                <rect class="mouse-svg-hotspot ${selectedDiagramBtn === 'middle' ? 'active' : ''}" data-btn="middle"
                      x="97" y="36" width="18" height="40" rx="9"
                      fill="rgba(16,185,129,0.28)" stroke="#10b981" stroke-width="2" />

                <!-- Side Button 5: Forward (Top) -->
                <rect class="mouse-svg-hotspot ${selectedDiagramBtn === 'forward' ? 'active' : ''}" data-btn="forward"
                      x="30" y="84" width="16" height="36" rx="6"
                      fill="rgba(59,130,246,0.30)" stroke="#3b82f6" stroke-width="2" />
                <text x="24" y="106" text-anchor="end" fill="#3b82f6" font-size="11" font-weight="700">${t("mouse.svg_fwd")}</text>

                <!-- Side Button 4: Back (Bottom) -->
                <rect class="mouse-svg-hotspot ${selectedDiagramBtn === 'back' ? 'active' : ''}" data-btn="back"
                      x="30" y="126" width="16" height="36" rx="6"
                      fill="rgba(59,130,246,0.24)" stroke="#3b82f6" stroke-width="2" />
                <text x="24" y="148" text-anchor="end" fill="#3b82f6" font-size="11" font-weight="700">${t("mouse.svg_back")}</text>

                <text x="106" y="156" text-anchor="middle" fill="currentColor" opacity="0.48" font-size="11" font-weight="700">MAC OMNI</text>
                <text x="106" y="172" text-anchor="middle" fill="currentColor" opacity="0.35" font-size="10">5-Button Mouse</text>
              </svg>

              <div style="width:100%;background:var(--bg-card);border:1px solid var(--border);border-radius:8px;padding:7px 9px;text-align:center;">
                <div style="font-size:11px;font-weight:700;color:var(--accent);margin-bottom:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
                  ${escapeHtml(diagramBtnTitle)}
                </div>
                <div style="font-size:11px;color:var(--text-muted);margin-bottom:5px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
                  ${escapeHtml(getActionLabel(diagramBtnActionKey))}
                </div>
                <button class="btn btn-sm btn-primary" id="btn-test-diagram-action" style="width:100%;" ${diagramTestEnabled ? "" : "disabled"}>
                  ▶️ ${t("mouse.btn_test_action")}
                </button>
              </div>
            </div>

            <!-- Right: App Profiles + Button Assignment Rows -->
            <div style="display:flex;flex-direction:column;gap:8px;min-width:0;">
              <!-- App Selector Bar -->
              <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:10px;padding:9px 12px;">
                <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;margin-bottom:8px;flex-wrap:wrap;">
                  <span style="font-size:12px;font-weight:600;">
                    1. ${t("mouse.select_app_profile")}
                  </span>
                  <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
                    <!-- Option 1: Select directly from currently open apps -->
                    <select class="select-box" id="sel-add-running-app" style="min-width:195px;max-width:250px;font-size:11.5px;padding:4px 8px;" ${masterEnabled ? "" : "disabled"}>
                      <option value="">${t("mouse.pick_running_app")}</option>
                      ${availableRunningApps.map(a => `
                        <option value="${escapeHtml(a.bundle_id)}" data-name="${escapeHtml(a.app_name)}">
                          🟢 ${escapeHtml(a.app_name)}
                        </option>
                      `).join("")}
                    </select>
                    <!-- Option 2: Open App Picker Popup (Installed Apps + Finder .app dialog) -->
                    <button class="btn btn-sm" id="btn-open-app-picker-modal" style="color:var(--accent);border-color:rgba(59,130,246,0.4);" ${masterEnabled ? "" : "disabled"}>
                      ${t("mouse.btn_open_app_popup")}
                    </button>
                  </div>
                </div>

                <div class="app-profile-pills">
                  <button class="app-profile-pill ${selectedProfileKey === '__default__' ? 'active' : ''}" data-profile="__default__">
                    <span>🌐</span>
                    <span>${t("mouse.profile_default")}</span>
                  </button>
                  ${profiles.map(p => `
                    <button class="app-profile-pill ${selectedProfileKey === p.bundle_id ? 'active' : ''} ${p.enabled === false ? 'disabled-prof' : ''}" data-profile="${escapeHtml(p.bundle_id)}">
                      <span>${escapeHtml(p.icon || iconForBundleId(p.bundle_id))}</span>
                      <span>${escapeHtml(p.app_name)}</span>
                    </button>
                  `).join("")}
                </div>
              </div>

              <!-- Selected Profile Settings Card -->
              <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:10px;padding:10px 13px;flex:1;display:flex;flex-direction:column;justify-content:space-between;">
                <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;padding-bottom:6px;border-bottom:1px solid var(--border);flex-wrap:wrap;">
                  <div style="font-weight:700;font-size:12.5px;display:flex;align-items:center;gap:7px;">
                    <span>${activeProfile ? escapeHtml(activeProfile.icon || "🧩") : "🌐"}</span>
                    <span>
                      ${activeProfile
                        ? `${escapeHtml(activeProfile.app_name)} <span class="stat-sub">(${escapeHtml(activeProfile.bundle_id)})</span>`
                        : t("mouse.default_rules_header")}
                    </span>
                  </div>

                  ${activeProfile ? `
                    <div style="display:flex;align-items:center;gap:8px;">
                      <span class="stat-sub">${t("mouse.active_for_this_app")}</span>
                      <label class="switch">
                        <input type="checkbox" id="chk-profile-enabled" ${activeProfile.enabled !== false ? "checked" : ""} ${masterEnabled ? "" : "disabled"} />
                        <span class="slider-toggle"></span>
                      </label>
                      <button class="btn btn-sm btn-danger" id="btn-delete-profile" title="${t("mouse.tip_delete_profile")}" ${masterEnabled ? "" : "disabled"}>🗑️</button>
                    </div>
                  ` : `
                    <span class="badge-ok">
                      ${t("mouse.badge_auto_switch_space")}
                    </span>
                  `}
                </div>

                <!-- Row 1: Mouse 5 (Forward) -->
                <div class="mouse-setting-row" style="border-bottom:1px solid var(--border);${btnRowControlsEnabled ? '' : 'opacity:0.6;'}">
                  <div class="mouse-setting-info">
                    <span class="mouse-btn-badge">Mouse 5</span>
                    <div class="mouse-setting-text">
                      <div style="font-weight:600;">${t("mouse.row_forward_title")}</div>
                      <div class="stat-sub">
                        ${activeProfile
                          ? t("mouse.row_forward_sub_app")
                          : t("mouse.row_forward_sub_def")}
                      </div>
                    </div>
                  </div>
                  <select class="select-box mouse-action-select" id="sel-mouse-forward" ${btnRowControlsEnabled ? "" : "disabled"}>
                    ${renderActionOptionsHtml(curForward)}
                  </select>
                </div>

                <!-- Row 2: Mouse 4 (Back) -->
                <div class="mouse-setting-row" style="border-bottom:1px solid var(--border);${btnRowControlsEnabled ? '' : 'opacity:0.6;'}">
                  <div class="mouse-setting-info">
                    <span class="mouse-btn-badge">Mouse 4</span>
                    <div class="mouse-setting-text">
                      <div style="font-weight:600;">${t("mouse.row_back_title")}</div>
                      <div class="stat-sub">
                        ${activeProfile
                          ? t("mouse.row_back_sub_app")
                          : t("mouse.row_back_sub_def")}
                      </div>
                    </div>
                  </div>
                  <select class="select-box mouse-action-select" id="sel-mouse-back" ${btnRowControlsEnabled ? "" : "disabled"}>
                    ${renderActionOptionsHtml(curBack)}
                  </select>
                </div>

                <!-- Row 3: Mouse 3 (Middle Wheel Click) -->
                <div class="mouse-setting-row" style="border-bottom:1px solid var(--border);${btnRowControlsEnabled ? '' : 'opacity:0.6;'}">
                  <div class="mouse-setting-info">
                    <span class="mouse-btn-badge" style="color:var(--success);border-color:rgba(16,185,129,0.3);background:rgba(16,185,129,0.12);">Mouse 3</span>
                    <div class="mouse-setting-text">
                      <div style="font-weight:600;">${t("mouse.row_middle_title")}</div>
                    </div>
                  </div>
                  <select class="select-box mouse-action-select" id="sel-mouse-middle" ${btnRowControlsEnabled ? "" : "disabled"}>
                    ${renderActionOptionsHtml(curMiddle)}
                  </select>
                </div>

                <!-- Row 4: Context-specific row (Single-Space Fallback on Default vs Smooth Scroll Override on App Profile) -->
                ${!activeProfile ? `
                  <div class="mouse-setting-row" style="border-bottom:1px solid var(--border);${buttonsEnabled ? '' : 'opacity:0.6;'}">
                    <div class="mouse-setting-info">
                      <span class="mouse-btn-badge" style="color:var(--warning);border-color:rgba(245,158,11,0.35);background:rgba(245,158,11,0.14);">Spaces</span>
                      <div class="mouse-setting-text">
                        <div style="font-weight:600;">${t("mouse.row_single_space_title")}</div>
                        <div class="stat-sub">${t("mouse.row_single_space_sub")}</div>
                      </div>
                    </div>
                    <select class="select-box mouse-action-select" id="sel-single-space-fallback" ${buttonsEnabled ? "" : "disabled"}>
                      <option value="passthrough" ${cfg.single_space_fallback === 'passthrough' ? 'selected' : ''}>
                        ${t("mouse.fallback_passthrough")}
                      </option>
                      <option value="ignore" ${cfg.single_space_fallback === 'ignore' ? 'selected' : ''}>
                        ${t("mouse.fallback_ignore")}
                      </option>
                      <option value="mission_control" ${cfg.single_space_fallback === 'mission_control' ? 'selected' : ''}>
                        ${t("mouse.fallback_mc")}
                      </option>
                    </select>
                  </div>

                  <!-- Row 5: Simultaneous Left + Right Click Chord -->
                  <div class="mouse-setting-row" style="padding-bottom:2px;">
                    <div class="mouse-setting-info">
                      <span class="mouse-btn-badge">L + R</span>
                      <div class="mouse-setting-text">
                        <div style="font-weight:600;">${t("mouse.row_chord_title")}</div>
                        <div class="stat-sub">${t("mouse.row_chord_sub", { ms: cfg.mouse_chord_threshold_ms })}</div>
                      </div>
                    </div>
                    <div style="display:flex;align-items:center;gap:8px;flex-shrink:0;">
                      <select class="select-box" id="sel-chord-action-mouse" style="width:192px;font-size:12px;padding:5px 8px;" ${chordEnabled ? "" : "disabled"}>
                        <option value="mission_control" ${cfg.mouse_chord_action === 'mission_control' ? 'selected' : ''}>🚀 Mission Control (F3)</option>
                        <option value="app_expose" ${cfg.mouse_chord_action === 'app_expose' ? 'selected' : ''}>🪟 App Exposé</option>
                        <option value="show_desktop" ${cfg.mouse_chord_action === 'show_desktop' ? 'selected' : ''}>🖥️ ${t("mouse.chord_show_desktop")}</option>
                        <option value="launchpad" ${cfg.mouse_chord_action === 'launchpad' ? 'selected' : ''}>🎛️ Launchpad</option>
                        <option value="clipboard_popup" ${cfg.mouse_chord_action === 'clipboard_popup' ? 'selected' : ''}>📋 ${t("mouse.chord_clipboard")}</option>
                        <option value="alt_tab" ${cfg.mouse_chord_action === 'alt_tab' ? 'selected' : ''}>🪟 Alt-Tab</option>
                        <option value="screenshot" ${cfg.mouse_chord_action === 'screenshot' ? 'selected' : ''}>📸 ${t("mouse.chord_screenshot")}</option>
                      </select>
                      <label class="switch">
                        <input type="checkbox" id="chk-chord-enabled-mouse" ${cfg.mouse_chord_enabled ? 'checked' : ''} ${masterEnabled ? "" : "disabled"} />
                        <span class="slider-toggle"></span>
                      </label>
                    </div>
                  </div>
                ` : `
                  <div class="mouse-setting-row" style="padding-bottom:2px;${masterEnabled && profileActive && smoothEnabled ? '' : 'opacity:0.6;'}">
                    <div class="mouse-setting-info">
                      <span class="mouse-btn-badge">🌊 120Hz</span>
                      <div class="mouse-setting-text">
                        <div style="font-weight:600;">${t("mouse.prof_smooth_title")}</div>
                        <div class="stat-sub">${t("mouse.prof_smooth_sub")}</div>
                      </div>
                    </div>
                    <select class="select-box mouse-action-select" id="sel-profile-smooth-mode" ${masterEnabled && profileActive && smoothEnabled ? "" : "disabled"}>
                      <option value="inherit" ${(activeProfile.smooth_scroll_mode || 'inherit') === 'inherit' ? 'selected' : ''}>
                        ${t("mouse.prof_smooth_inherit")}
                      </option>
                      <option value="on" ${activeProfile.smooth_scroll_mode === 'on' ? 'selected' : ''}>
                        ${t("mouse.prof_smooth_on")}
                      </option>
                      <option value="off" ${activeProfile.smooth_scroll_mode === 'off' ? 'selected' : ''}>
                        ${t("mouse.prof_smooth_off")}
                      </option>
                    </select>
                  </div>
                `}
              </div>
            </div>
          </div>
        </div>
      ` : `
        <!-- SUB-TAB 2: 120HZ SMOOTH SCROLL & REVERSE SCROLL TUNING -->
        <div class="card" style="margin-bottom:0;${masterEnabled ? '' : 'opacity:0.6;'}">
          <div class="card-header">
            <div class="card-title">
              <span>🌊 ${t("mouse.scroll_card_title")}</span>
              ${renderTip(t("mouse.scroll_card_tip"))}
            </div>
            <div style="display:flex;gap:6px;flex-wrap:wrap;">
              <button class="btn btn-sm smooth-preset" data-step="35" data-dur="200" data-acc="1.4" ${masterEnabled ? "" : "disabled"}>${t("mouse.preset_snappy")}</button>
              <button class="btn btn-sm smooth-preset" data-step="45" data-dur="260" data-acc="1.6" ${masterEnabled ? "" : "disabled"}>${t("mouse.preset_balanced")}</button>
              <button class="btn btn-sm smooth-preset" data-step="65" data-dur="360" data-acc="2.0" ${masterEnabled ? "" : "disabled"}>${t("mouse.preset_silky")}</button>
            </div>
          </div>

          <div class="grid-3" style="margin-bottom:10px;${smoothEnabled ? '' : 'opacity:0.55;'}">
            <!-- Slider 1: Step Size -->
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;">
              <div style="display:flex;justify-content:space-between;margin-bottom:5px;">
                <span>📏 ${t("mouse.slider_step_title")}</span>
                <b id="lbl-scroll-step">${cfg.scroll_step_px || 45} px</b>
              </div>
              <input type="range" min="15" max="120" step="5" value="${cfg.scroll_step_px || 45}" id="slider-scroll-step" ${smoothEnabled ? "" : "disabled"} />
              <div class="stat-sub" style="margin-top:4px;">${t("mouse.slider_step_sub")}</div>
            </div>

            <!-- Slider 2: Smoothness / Glide Duration -->
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;">
              <div style="display:flex;justify-content:space-between;margin-bottom:5px;">
                <span>⏳ ${t("mouse.slider_dur_title")}</span>
                <b id="lbl-scroll-dur">${cfg.scroll_duration_ms || 260} ms</b>
              </div>
              <input type="range" min="80" max="600" step="20" value="${cfg.scroll_duration_ms || 260}" id="slider-scroll-dur" ${smoothEnabled ? "" : "disabled"} />
              <div class="stat-sub" style="margin-top:4px;">${t("mouse.slider_dur_sub")}</div>
            </div>

            <!-- Slider 3: Acceleration -->
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;">
              <div style="display:flex;justify-content:space-between;margin-bottom:5px;">
                <span>🚀 ${t("mouse.slider_acc_title")}</span>
                <b id="lbl-scroll-acc">${Number(cfg.scroll_acceleration || 1.6).toFixed(1)}x</b>
              </div>
              <input type="range" min="10" max="30" step="1" value="${Math.round((cfg.scroll_acceleration || 1.6) * 10)}" id="slider-scroll-acc" ${smoothEnabled ? "" : "disabled"} />
              <div class="stat-sub" style="margin-top:4px;">${t("mouse.slider_acc_sub")}</div>
            </div>
          </div>

          <div class="mouse-setting-row" style="border-top:1px solid var(--border);border-bottom:1px solid var(--border);">
            <div class="mouse-setting-text">
              <div style="font-weight:600;">🔄 ${t("mouse.row_reverse_bottom_title")}</div>
              <div class="stat-sub">${t("mouse.row_reverse_bottom_sub")}</div>
            </div>
            <label class="switch">
              <input type="checkbox" id="chk-invert-scroll-bottom" ${cfg.invert_external_mouse_scroll ? "checked" : ""} ${masterEnabled ? "" : "disabled"} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <div class="mouse-setting-row">
            <div class="mouse-setting-text">
              <div style="font-weight:600;">↔️ ${t("mouse.row_shift_horiz_title")}</div>
              <div class="stat-sub">${t("mouse.row_shift_horiz_sub")}</div>
            </div>
            <label class="switch">
              <input type="checkbox" id="chk-shift-horiz" ${cfg.shift_horizontal_scroll !== false ? "checked" : ""} ${masterEnabled ? "" : "disabled"} />
              <span class="slider-toggle"></span>
            </label>
          </div>

          <!-- Scroll Test Playground -->
          <div class="scroll-test-playground" style="max-height:145px;">
            <div style="font-weight:700;font-size:12px;color:var(--accent);margin-bottom:6px;">
              🧪 ${t("mouse.test_box_title")}
            </div>
            <div style="display:flex;flex-direction:column;gap:5px;font-size:12px;color:var(--text-muted);">
              <div>• ${t("mouse.test_line1")}</div>
              <div>• ${t("mouse.test_line2")}</div>
              <div>• ${t("mouse.test_line3")}</div>
              <div>• ${t("mouse.test_line4")}</div>
              <div>• ${t("mouse.test_line5")}</div>
              <div>• ${t("mouse.test_line6")}</div>
            </div>
          </div>
        </div>
      `}

      ${showAppPickerModal ? `
        <div id="mouse-app-picker-backdrop" style="position:fixed;inset:0;background:rgba(15,23,42,0.68);backdrop-filter:blur(6px);z-index:999;display:flex;align-items:center;justify-content:center;padding:18px;">
          <div style="width:100%;max-width:580px;max-height:82vh;background:var(--bg-card);border:1px solid var(--border);border-radius:14px;box-shadow:0 20px 50px rgba(0,0,0,0.45);display:flex;flex-direction:column;overflow:hidden;">
            <div style="padding:12px 16px;border-bottom:1px solid var(--border);display:flex;align-items:center;justify-content:space-between;gap:10px;">
              <div>
                <div style="font-weight:700;font-size:14px;">${t("mouse.modal_title")}</div>
                <div class="stat-sub" style="font-size:11.5px;margin-top:2px;">${t("mouse.modal_sub")}</div>
              </div>
              <button class="btn btn-sm" id="btn-close-app-picker-modal">${t("mouse.btn_close_add_app")}</button>
            </div>

            <div style="padding:10px 16px;border-bottom:1px solid var(--border);display:flex;align-items:center;gap:8px;flex-wrap:wrap;background:var(--bg-input);">
              <input type="text" class="input-box" id="inp-modal-app-search" value="${escapeHtml(modalSearchQuery)}" placeholder="${t("mouse.modal_search_placeholder")}" style="flex:1;min-width:220px;" />
              <button class="btn btn-sm btn-primary" id="btn-pick-finder-app">
                ${t("mouse.btn_pick_finder_app")}
              </button>
            </div>

            <div id="modal-app-picker-list" style="padding:10px 16px;overflow-y:auto;max-height:360px;display:flex;flex-direction:column;gap:6px;">
            </div>
          </div>
        </div>
      ` : ""}
    `;

    view.querySelectorAll("#mouse-subtabs .pill-tab").forEach(btn => {
      btn.addEventListener("click", () => {
        mouseSubTab = btn.dataset.sub || "buttons";
        render(view);
      });
    });

    const saveConfigPatch = async (patchFn, customToast) => {
      const latest = await api("get_input_config");
      patchFn(latest);
      await api("update_input_config", latest);
      showToast(customToast || t("mouse.toast_saved"));
      await render(view);
    };

    const addOrSelectAppProfile = async (bundleId, appName) => {
      const bTrim = (bundleId || "").trim();
      if (!bTrim) return;
      const aName = (appName || "").trim() || bTrim;
      showAppPickerModal = false;
      await saveConfigPatch(
        c => {
          c.app_mouse_profiles = Array.isArray(c.app_mouse_profiles) ? c.app_mouse_profiles : [];
          const existing = c.app_mouse_profiles.find(
            p => (p.bundle_id || "").toLowerCase() === bTrim.toLowerCase()
          );
          if (!existing) {
            c.app_mouse_profiles.push({
              bundle_id: bTrim,
              app_name: aName,
              icon: iconForBundleId(bTrim),
              enabled: true,
              forward_action: "default",
              back_action: "default",
              middle_action: "default",
              smooth_scroll_mode: "inherit"
            });
          } else {
            existing.enabled = true;
          }
          selectedProfileKey = existing ? existing.bundle_id : bTrim;
        },
        t("mouse.toast_added_app", { name: aName })
      );
    };

    // Header buttons
    document.getElementById("btn-refresh-mouse").onclick = () => {
      cachedInstalledApps = null;
      render(view);
    };
    document.getElementById("btn-reset-mouse-defaults").onclick = async () => {
      await api("reset_mouse_profiles");
      selectedProfileKey = "__default__";
      showToast(t("mouse.toast_restored_profiles"));
      await render(view);
    };

    // Top 3 Quick Switches
    document.getElementById("chk-ext-mouse-master").onchange = (e) =>
      saveConfigPatch(c => { c.external_mouse_enabled = e.target.checked; });

    document.getElementById("chk-invert-scroll-top").onchange = (e) =>
      saveConfigPatch(c => { c.invert_external_mouse_scroll = e.target.checked; });

    document.getElementById("chk-smooth-scroll-top").onchange = (e) =>
      saveConfigPatch(c => { c.smooth_scroll_enabled = e.target.checked; });

    if (mouseSubTab === "buttons") {
      document.getElementById("chk-mouse-buttons-enabled").onchange = (e) =>
        saveConfigPatch(c => { c.mouse_buttons_enabled = e.target.checked; });

      // SVG Mouse Diagram Hotspots
      view.querySelectorAll(".mouse-svg-hotspot").forEach(el => {
        el.addEventListener("click", () => {
          selectedDiagramBtn = el.dataset.btn || "forward";
          render(view);
        });
      });

      document.getElementById("btn-test-diagram-action").onclick = async () => {
        if (diagramBtnActionKey === "default" || diagramBtnActionKey === "disabled") {
          showToast(t("mouse.toast_passthrough"));
          return;
        }
        await api("trigger_system_action", { action: diagramBtnActionKey });
        showToast(t("mouse.toast_triggered", { action: getActionLabel(diagramBtnActionKey) }));
      };

      // App Profile Pills
      view.querySelectorAll(".app-profile-pill").forEach(pill => {
        pill.addEventListener("click", () => {
          selectedProfileKey = pill.dataset.profile || "__default__";
          render(view);
        });
      });

      // Option 1: Select directly from currently open/running apps dropdown
      const selRunning = document.getElementById("sel-add-running-app");
      if (selRunning) {
        selRunning.onchange = async () => {
          const opt = selRunning.options[selRunning.selectedIndex];
          if (opt && opt.value) {
            await addOrSelectAppProfile(opt.value, opt.dataset.name || opt.value);
          }
        };
      }

      // Option 2: Open App Picker Modal Popup
      const btnOpenPopup = document.getElementById("btn-open-app-picker-modal");
      if (btnOpenPopup) {
        btnOpenPopup.onclick = async () => {
          showAppPickerModal = true;
          modalSearchQuery = "";
          if (!cachedInstalledApps) {
            const appRes = await api("list_installed_mouse_apps");
            cachedInstalledApps = (appRes && Array.isArray(appRes.installed_apps))
              ? appRes.installed_apps
              : [];
          }
          await render(view);
        };
      }

      if (showAppPickerModal) {
        const runningSet = new Set(runningApps.map(a => (a.bundle_id || "").toLowerCase()));
        const allMap = new Map();
        for (const a of runningApps) {
          if (a.bundle_id) allMap.set(a.bundle_id.toLowerCase(), a);
        }
        for (const a of (cachedInstalledApps || [])) {
          if (a.bundle_id && !allMap.has(a.bundle_id.toLowerCase())) {
            allMap.set(a.bundle_id.toLowerCase(), a);
          }
        }
        const mergedApps = Array.from(allMap.values()).sort((a, b) => {
          const aRun = runningSet.has(a.bundle_id.toLowerCase()) ? 0 : 1;
          const bRun = runningSet.has(b.bundle_id.toLowerCase()) ? 0 : 1;
          if (aRun !== bRun) return aRun - bRun;
          return (a.app_name || "").localeCompare(b.app_name || "");
        });

        const renderModalList = (q) => {
          const listEl = document.getElementById("modal-app-picker-list");
          if (!listEl) return;
          const qNorm = (q || "").trim().toLowerCase();
          const filtered = mergedApps.filter(a => {
            if (!qNorm) return true;
            return (
              (a.app_name || "").toLowerCase().includes(qNorm) ||
              (a.bundle_id || "").toLowerCase().includes(qNorm)
            );
          });
          if (filtered.length === 0) {
            listEl.innerHTML = `<div class="stat-sub" style="padding:18px;text-align:center;">${t("mouse.modal_empty")}</div>`;
            return;
          }
          listEl.innerHTML = filtered.map(a => {
            const isRun = runningSet.has(a.bundle_id.toLowerCase());
            const isAdded = existingBundles.has(a.bundle_id.toLowerCase());
            return `
              <div class="modal-app-row" data-bundle="${escapeHtml(a.bundle_id)}" data-name="${escapeHtml(a.app_name)}"
                   style="display:flex;align-items:center;justify-content:space-between;gap:10px;padding:8px 11px;border-radius:9px;background:var(--bg-input);border:1px solid ${isAdded ? 'rgba(59,130,246,0.4)' : 'var(--border)'};cursor:pointer;">
                <div style="display:flex;align-items:center;gap:9px;min-width:0;">
                  <span style="font-size:16px;">${iconForBundleId(a.bundle_id)}</span>
                  <div style="min-width:0;">
                    <div style="font-weight:600;font-size:12.5px;display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
                      <span>${escapeHtml(a.app_name)}</span>
                      ${isRun ? `<span class="badge-ok" style="font-size:10px;padding:1px 6px;">${t("mouse.badge_running_app")}</span>` : ""}
                      ${isAdded ? `<span class="stat-sub" style="font-size:10.5px;color:var(--accent);font-weight:600;">• ${t("mouse.badge_already_added")}</span>` : ""}
                    </div>
                    <div class="stat-sub" style="font-size:11px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
                      ${escapeHtml(a.bundle_id)}
                    </div>
                  </div>
                </div>
                <button class="btn btn-sm ${isAdded ? '' : 'btn-primary'}" style="flex-shrink:0;">
                  ${isAdded ? "✓" : t("mouse.btn_select_this_app")}
                </button>
              </div>
            `;
          }).join("");

          listEl.querySelectorAll(".modal-app-row").forEach(row => {
            row.onclick = () => addOrSelectAppProfile(row.dataset.bundle, row.dataset.name);
          });
        };

        renderModalList(modalSearchQuery);

        const inpSearch = document.getElementById("inp-modal-app-search");
        if (inpSearch) {
          inpSearch.focus();
          inpSearch.oninput = (e) => {
            modalSearchQuery = e.target.value || "";
            renderModalList(modalSearchQuery);
          };
        }

        const btnCloseModal = document.getElementById("btn-close-app-picker-modal");
        if (btnCloseModal) {
          btnCloseModal.onclick = () => {
            showAppPickerModal = false;
            render(view);
          };
        }

        const backdrop = document.getElementById("mouse-app-picker-backdrop");
        if (backdrop) {
          backdrop.onclick = (e) => {
            if (e.target === backdrop) {
              showAppPickerModal = false;
              render(view);
            }
          };
        }

        const btnFinderPick = document.getElementById("btn-pick-finder-app");
        if (btnFinderPick) {
          btnFinderPick.onclick = async () => {
            const pickRes = await api("pick_mouse_app_dialog");
            if (pickRes && pickRes.ok && pickRes.app && pickRes.app.bundle_id) {
              await addOrSelectAppProfile(pickRes.app.bundle_id, pickRes.app.app_name);
            }
          };
        }
      }

      // Profile-specific controls vs Global controls
      const updateSelectedProfileField = (field, value) =>
        saveConfigPatch(c => {
          if (selectedProfileKey === "__default__") {
            if (field === "forward_action") c.mouse_forward_action = value;
            if (field === "back_action") c.mouse_back_action = value;
            if (field === "middle_action") c.mouse_middle_action = value;
          } else {
            const prof = (c.app_mouse_profiles || []).find(p => p.bundle_id === selectedProfileKey);
            if (prof) {
              prof[field] = value;
            }
          }
        });

      document.getElementById("sel-mouse-forward").onchange = (e) => {
        selectedDiagramBtn = "forward";
        updateSelectedProfileField("forward_action", e.target.value);
      };
      document.getElementById("sel-mouse-back").onchange = (e) => {
        selectedDiagramBtn = "back";
        updateSelectedProfileField("back_action", e.target.value);
      };
      document.getElementById("sel-mouse-middle").onchange = (e) => {
        selectedDiagramBtn = "middle";
        updateSelectedProfileField("middle_action", e.target.value);
      };

      const chkProfEnabled = document.getElementById("chk-profile-enabled");
      if (chkProfEnabled) {
        chkProfEnabled.onchange = (e) => updateSelectedProfileField("enabled", e.target.checked);
      }

      const selProfSmooth = document.getElementById("sel-profile-smooth-mode");
      if (selProfSmooth) {
        selProfSmooth.onchange = (e) => updateSelectedProfileField("smooth_scroll_mode", e.target.value);
      }

      const btnDelProf = document.getElementById("btn-delete-profile");
      if (btnDelProf) {
        btnDelProf.onclick = () =>
          saveConfigPatch(c => {
            c.app_mouse_profiles = (c.app_mouse_profiles || []).filter(
              p => p.bundle_id !== selectedProfileKey
            );
            selectedProfileKey = "__default__";
          });
      }

      const selFallback = document.getElementById("sel-single-space-fallback");
      if (selFallback) {
        selFallback.onchange = (e) =>
          saveConfigPatch(c => { c.single_space_fallback = e.target.value; });
      }

      const selChordAct = document.getElementById("sel-chord-action-mouse");
      if (selChordAct) {
        selChordAct.onchange = (e) => {
          selectedDiagramBtn = "chord";
          saveConfigPatch(c => { c.mouse_chord_action = e.target.value; });
        };
      }

      const chkChordEn = document.getElementById("chk-chord-enabled-mouse");
      if (chkChordEn) {
        chkChordEn.onchange = (e) =>
          saveConfigPatch(c => { c.mouse_chord_enabled = e.target.checked; });
      }
    } else {
      document.getElementById("chk-invert-scroll-bottom").onchange = (e) =>
        saveConfigPatch(c => { c.invert_external_mouse_scroll = e.target.checked; });

      document.getElementById("chk-shift-horiz").onchange = (e) =>
        saveConfigPatch(c => { c.shift_horizontal_scroll = e.target.checked; });

      // Smooth Scroll Sliders & Presets
      const stepSlider = document.getElementById("slider-scroll-step");
      const durSlider = document.getElementById("slider-scroll-dur");
      const accSlider = document.getElementById("slider-scroll-acc");

      stepSlider.oninput = (e) => {
        document.getElementById("lbl-scroll-step").textContent = e.target.value + " px";
      };
      stepSlider.onchange = (e) =>
        saveConfigPatch(c => { c.scroll_step_px = Number(e.target.value); });

      durSlider.oninput = (e) => {
        document.getElementById("lbl-scroll-dur").textContent = e.target.value + " ms";
      };
      durSlider.onchange = (e) =>
        saveConfigPatch(c => { c.scroll_duration_ms = Number(e.target.value); });

      accSlider.oninput = (e) => {
        document.getElementById("lbl-scroll-acc").textContent = (Number(e.target.value) / 10).toFixed(1) + "x";
      };
      accSlider.onchange = (e) =>
        saveConfigPatch(c => { c.scroll_acceleration = Number(e.target.value) / 10; });

      view.querySelectorAll(".smooth-preset").forEach(btn => {
        btn.addEventListener("click", () =>
          saveConfigPatch(c => {
            c.smooth_scroll_enabled = true;
            c.scroll_step_px = Number(btn.dataset.step);
            c.scroll_duration_ms = Number(btn.dataset.dur);
            c.scroll_acceleration = Number(btn.dataset.acc);
          })
        );
      });
    }

    // Lightweight live polling for active Desktop (Space) count & physical mouse button press indicator
    if (masterEnabled) {
      pollTimer = setInterval(async () => {
        if (typeof isTabActive === "function" && !isTabActive()) {
          stopPolling();
          return;
        }
        const stRes = await api("get_mouse_state");
        if (!stRes || !stRes.spaces) return;
        const sp = stRes.spaces;
        const spacesLbl = document.getElementById("lbl-spaces-live");
        if (spacesLbl) {
          spacesLbl.textContent = `🖥️ ${t("mouse.spaces_live", {
            current: sp.current_space_index,
            total: sp.total_spaces
          })}`;
        }
        const btnLbl = document.getElementById("lbl-last-mouse-btn");
        if (btnLbl && sp.last_mouse_button >= 3) {
          btnLbl.textContent = t("mouse.last_pressed", { btn: sp.last_mouse_button });
          if (sp.last_mouse_button_at_ms !== lastSeenBtnAt) {
            lastSeenBtnAt = sp.last_mouse_button_at_ms;
            btnLbl.style.borderColor = "var(--accent)";
            btnLbl.style.color = "var(--accent)";
            setTimeout(() => {
              if (btnLbl) {
                btnLbl.style.borderColor = "var(--border)";
                btnLbl.style.color = "var(--text-muted)";
              }
            }, 900);
          }
        }
      }, 1200);
    }
  }

  return { render, stopPolling };
}
