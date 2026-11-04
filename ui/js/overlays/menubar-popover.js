// =============================================================================
// OVERLAY MODE 0: MENU BAR POPOVER (LIGHTWEIGHT POLLING & QUICK AUDIO/DISPLAY)
// =============================================================================
async function initMiniLeftWidget() {
  const root = document.getElementById("root");
  let hwData = null;
  let dispState = null;
  let audioState = null;
  let inputCfg = null;
  let lastAudioSig = "";
  let isDraggingSlider = false;

  function computeAudioSignature(aud) {
    if (!aud) return "";
    const outList = (aud.output_devices || [])
      .map(d => `${d.id}:${d.name}:${d.is_default_output ? 1 : 0}`)
      .join("|");
    return `${aud.current_output_id}|${aud.muted ? 1 : 0}|${Math.round((aud.volume || 0) * 100)}|${outList}`;
  }

  async function pollMiniStats() {
    applyAppTheme(getAppTheme(), false);
    applyAppLang(getAppLang(), false);
    const [hw, disp, aud, inp] = await Promise.all([
      api("get_menubar_stats"),
      api("get_display_state"),
      api("get_audio_state"),
      api("get_input_config")
    ]);
    hwData = hw;
    dispState = disp;
    audioState = aud;
    inputCfg = inp;
    lastAudioSig = computeAudioSignature(aud);

    const cpuPct = hwData ? formatPct(hwData.cpu_usage_percent) : "0.0%";
    const rawTemp = hwData
      ? (hwData.cpu_temp_c ?? (hwData.temperatures && hwData.temperatures.cpu_temp_c) ?? 45)
      : 45;
    const cpuTemp = formatTemp(rawTemp);
    await api("update_menubar_title", { title: `❖ ${cpuTemp} • ${cpuPct}` });
    if (!isDraggingSlider) {
      renderMini();
    }
  }

  // Auto-detect headphones / AirPods / speakers plug/unplug every 1.2s
  setInterval(async () => {
    if (isDraggingSlider || document.hidden) return;
    const aud = await api("get_audio_state");
    const newSig = computeAudioSignature(aud);
    if (newSig && newSig !== lastAudioSig) {
      lastAudioSig = newSig;
      audioState = aud;
      renderMini();
    }
  }, 1200);

  // Update Menu Bar temperature & CPU % every 4s (lightweight, skips process table)
  setInterval(async () => {
    if (isDraggingSlider) return;
    const hw = await api("get_menubar_stats");
    if (hw) {
      hwData = hw;
      const cpuPct = formatPct(hwData.cpu_usage_percent);
      const rawTemp = hwData.cpu_temp_c ?? (hwData.temperatures && hwData.temperatures.cpu_temp_c) ?? 45;
      const cpuTemp = formatTemp(rawTemp);
      await api("update_menubar_title", { title: `❖ ${cpuTemp} • ${cpuPct}` });
      const subEl = document.getElementById("mini-hw-sub");
      if (subEl) {
        subEl.textContent = `CPU ${cpuPct} • ${cpuTemp} • RAM ${formatPct(hwData.memory_usage_percent)}`;
      }
    }
  }, 4000);

  async function closePopover() {
    await api("set_mini_expanded", { expanded: false });
  }

  function renderMini() {
    const cpuPct = hwData ? formatPct(hwData.cpu_usage_percent) : "0.0%";
    const ramPct = hwData ? formatPct(hwData.memory_usage_percent) : "0.0%";
    const rawTemp = hwData
      ? (hwData.cpu_temp_c ?? (hwData.temperatures && hwData.temperatures.cpu_temp_c) ?? 45)
      : 45;
    const cpuTemp = formatTemp(rawTemp);
    const displays = (dispState && dispState.displays) ? dispState.displays : [];
    const darkMode = dispState ? !!dispState.dark_mode : true;
    const volPct = audioState ? Math.round((audioState.volume || 0) * 100) : 50;
    const muted = audioState ? !!audioState.muted : false;
    const outDevs = (audioState && audioState.output_devices) ? audioState.output_devices : [];
    const curLang = getAppLang();

    root.innerHTML = `
      <div class="mini-dropdown-panel">
        <div class="mini-header-row">
          <div style="display:flex;align-items:center;gap:6px;min-width:0;flex:1;">
            <div class="brand-logo" style="width:22px;height:22px;font-size:11.5px;border-radius:6px;">M</div>
            <div style="font-weight:700;font-size:12.5px;white-space:nowrap;flex-shrink:0;">Mac Omni</div>
            <span class="stat-sub" id="mini-hw-sub" style="font-size:10.8px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">CPU ${cpuPct} • ${cpuTemp} • RAM ${ramPct}</span>
          </div>
          <div style="display:flex;gap:4px;align-items:center;flex-shrink:0;">
            ${renderLanguageSelectHtml("sel-mini-lang", "padding:1px 5px;font-size:11px;font-weight:600;height:24px;", true)}
            <button class="btn btn-sm" id="mini-btn-theme" style="padding:2px 6px;height:24px;font-size:11.5px;" title="${t("popover.tip_app_theme")}">${getAppTheme() === 'dark' ? '☀️' : '🌙'}</button>
            <button class="btn btn-sm" id="btn-collapse-top" style="padding:2px 6px;height:24px;font-size:11.5px;">✕</button>
          </div>
        </div>

        <!-- Compact Display Settings -->
        <div class="mini-section-card">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px;">
            <span style="font-size:11.8px;font-weight:600;white-space:nowrap;">${t("popover.displays")}</span>
            <button class="btn btn-sm" id="mini-btn-dark" style="padding:1px 7px;height:22px;font-size:10.8px;">
              ${darkMode ? t("popover.macos_dark") : t("popover.macos_light")}
            </button>
          </div>
          <div style="display:flex;flex-direction:column;gap:4px;">
            ${displays.map(d => `
              <div>
                <div style="display:flex;justify-content:space-between;align-items:center;font-size:11px;margin-bottom:2px;">
                  <span style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:255px;">${escapeHtml(d.name)}</span>
                  <b id="mini-br-lbl-${d.id}">${Math.round(d.brightness * 100)}%</b>
                </div>
                <input type="range" min="5" max="100" value="${Math.round(d.brightness * 100)}" class="mini-slider-br mini-range" data-id="${d.id}" />
              </div>
            `).join("")}
          </div>
        </div>

        <!-- Compact Audio & AirPods / Speakers Switcher -->
        <div class="mini-section-card">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:3px;">
            <span style="font-size:11.8px;font-weight:600;white-space:nowrap;">${t("popover.volume")}</span>
            <div style="display:flex;align-items:center;gap:5px;">
              <b id="mini-vol-lbl" style="font-size:11.5px;">${muted ? t("popover.muted") : volPct + "%"}</b>
              <button class="btn btn-sm" id="mini-btn-mute" style="padding:1px 6px;height:22px;font-size:11px;">${muted ? '🔇' : '🔊'}</button>
            </div>
          </div>
          <input type="range" min="0" max="100" value="${muted ? 0 : volPct}" id="mini-slider-vol" class="mini-range" />

          <div style="display:flex;align-items:center;justify-content:space-between;margin:5px 0 3px;">
            <span style="font-size:10.8px;font-weight:600;color:var(--text-muted);white-space:nowrap;">${t("popover.audio_output_count", { count: outDevs.length })}</span>
            <button class="btn-icon-xs" id="mini-btn-refresh-audio" style="padding:1px 5px;font-size:10.5px;" title="${t("popover.tip_refresh_audio")}">🔄</button>
          </div>
          <div class="mini-audio-list">
            ${outDevs.map(dev => `
              <div class="audio-device-card mini-audio-dev ${dev.is_default_output ? 'active' : ''}" data-id="${dev.id}">
                <span style="font-size:13.5px;flex-shrink:0;">${dev.icon}</span>
                <div style="flex:1;min-width:0;display:flex;align-items:center;gap:5px;">
                  <span style="font-size:11.5px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${escapeHtml(dev.name)}</span>
                  <span style="font-size:10px;color:var(--text-muted);white-space:nowrap;flex-shrink:0;">• ${escapeHtml(tAudioKindLabel(dev.kind_label))}</span>
                </div>
                ${dev.is_default_output ? `<span class="badge-ok" style="padding:1px 6px;font-size:10px;flex-shrink:0;">${t("common.active")}</span>` : ''}
              </div>
            `).join("")}
          </div>
        </div>

        <!-- Compact External Mouse Quick Toggles (Master / Reverse Scroll / Smooth 120Hz) -->
        <div class="mini-section-card mini-mouse-row">
          <span class="mini-mouse-label">🖱️ ${t("popover.mouse_label")}</span>
          <div class="mini-mouse-actions">
            <button class="btn btn-sm ${inputCfg && inputCfg.external_mouse_enabled !== false ? 'btn-primary' : ''}" id="mini-btn-mouse-master" title="${t("popover.tip_mouse_master")}">
              ${inputCfg && inputCfg.external_mouse_enabled !== false ? t("common.on") : t("common.off")}
            </button>
            <button class="btn btn-sm ${inputCfg && inputCfg.invert_external_mouse_scroll ? 'btn-primary' : ''}" id="mini-btn-mouse-invert" title="${t("popover.tip_mouse_invert")}">
              🔄 ${t("popover.mouse_reverse")}
            </button>
            <button class="btn btn-sm ${inputCfg && inputCfg.smooth_scroll_enabled !== false ? 'btn-primary' : ''}" id="mini-btn-mouse-smooth" title="${t("popover.tip_mouse_smooth")}">
              🌊 120Hz
            </button>
          </div>
        </div>

        <div style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:5px;flex-shrink:0;">
          <button class="btn btn-sm" id="mini-btn-shot" style="height:27px;font-size:11.2px;" title="${t("popover.tip_capture")}">${t("popover.btn_capture")}</button>
          <button class="btn btn-sm" id="mini-btn-clip" style="height:27px;font-size:11.2px;">📋 Clipboard</button>
          <button class="btn btn-sm" id="mini-btn-mc" style="height:27px;font-size:11.2px;">🚀 F3</button>
        </div>

        <div style="display:flex;gap:5px;margin-top:auto;flex-shrink:0;">
          <button class="btn btn-primary" id="btn-open-launcher" style="flex:1;height:30px;padding:0 10px;font-size:12px;font-weight:600;">
            ${t("popover.open_dashboard")}
          </button>
          <button class="btn btn-sm btn-danger" id="mini-btn-quit-app" style="height:30px;padding:0 9px;" title="${t("popover.tip_quit")}">⏻</button>
        </div>
      </div>
    `;

    const panelEl = root.querySelector(".mini-dropdown-panel");
    if (panelEl) {
      panelEl.style.height = "auto";
      const neededHeight = Math.ceil(panelEl.getBoundingClientRect().height);
      panelEl.style.height = "100vh";
      if (neededHeight > 200) {
        api("set_mini_height", { height: neededHeight });
      }
    }

    bindLanguageSelect("sel-mini-lang");
    document.getElementById("mini-btn-theme").onclick = () => {
      toggleAppTheme();
      renderMini();
    };
    document.getElementById("btn-collapse-top").onclick = closePopover;
    document.getElementById("btn-open-launcher").onclick = async () => {
      await api("show_launcher");
      await closePopover();
    };
    document.getElementById("mini-btn-shot").onclick = async () => {
      await closePopover();
      setTimeout(() => {
        api("trigger_screenshot");
      }, 80);
    };
    document.getElementById("mini-btn-clip").onclick = async () => {
      await closePopover();
      await api("toggle_overlay", { window: "clipboard" });
    };
    document.getElementById("mini-btn-mc").onclick = async () => {
      await closePopover();
      await api("trigger_system_action", { action: "mission_control" });
    };
    document.getElementById("mini-btn-mouse-master").onclick = async () => {
      const res = await api("toggle_external_mouse");
      if (res && res.config) inputCfg = res.config;
      renderMini();
    };
    document.getElementById("mini-btn-mouse-invert").onclick = async () => {
      const res = await api("toggle_invert_mouse_scroll");
      if (res && res.config) inputCfg = res.config;
      renderMini();
    };
    document.getElementById("mini-btn-mouse-smooth").onclick = async () => {
      const res = await api("toggle_smooth_scroll");
      if (res && res.config) inputCfg = res.config;
      renderMini();
    };
    document.getElementById("mini-btn-dark").onclick = async () => {
      await api("set_dark_mode", { dark: !darkMode });
      pollMiniStats();
    };
    document.getElementById("mini-btn-quit-app").onclick = () => api("quit_app");

    const refreshAudioBtn = document.getElementById("mini-btn-refresh-audio");
    if (refreshAudioBtn) {
      refreshAudioBtn.onclick = async () => {
        audioState = await api("get_audio_state");
        lastAudioSig = computeAudioSignature(audioState);
        renderMini();
      };
    }

    root.querySelectorAll(".mini-slider-br").forEach(slider => {
      slider.addEventListener("mousedown", () => { isDraggingSlider = true; });
      slider.addEventListener("mouseup", () => { isDraggingSlider = false; });
      slider.addEventListener("input", async (e) => {
        const id = Number(slider.dataset.id);
        const pct = Number(e.target.value);
        const lbl = document.getElementById(`mini-br-lbl-${id}`);
        if (lbl) lbl.textContent = pct + "%";
        await api("set_display_brightness", { display_id: id, brightness: pct / 100 });
      });
    });

    const volSlider = document.getElementById("mini-slider-vol");
    if (volSlider) {
      volSlider.addEventListener("mousedown", () => { isDraggingSlider = true; });
      volSlider.addEventListener("mouseup", () => { isDraggingSlider = false; });
      volSlider.addEventListener("input", async (e) => {
        const pct = Number(e.target.value);
        const lbl = document.getElementById("mini-vol-lbl");
        if (lbl) lbl.textContent = pct + "%";
        audioState = await api("set_audio_volume", { volume: pct / 100 });
        lastAudioSig = computeAudioSignature(audioState);
      });
    }

    const muteBtn = document.getElementById("mini-btn-mute");
    if (muteBtn) {
      muteBtn.onclick = async () => {
        audioState = await api("set_audio_volume", { muted: !muted });
        lastAudioSig = computeAudioSignature(audioState);
        renderMini();
      };
    }

    root.querySelectorAll(".mini-audio-dev").forEach(card => {
      card.addEventListener("click", async () => {
        audioState = await api("set_audio_device", {
          device_id: Number(card.dataset.id),
          is_output: true
        });
        lastAudioSig = computeAudioSignature(audioState);
        renderMini();
      });
    });
  }

  window.__onThemeChanged = renderMini;
  window.__onLangChanged = renderMini;
  window.__refreshMiniPopover = pollMiniStats;
  pollMiniStats();
}
