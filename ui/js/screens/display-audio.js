// =============================================================================
// SCREEN 1: DISPLAY & AUDIO CONTROL (SUB-TABS: DISPLAYS & APPEARANCE | AUDIO & MIC)
// =============================================================================
let displayAudioSubTab = "displays"; // "displays" | "audio"

async function renderDisplayAndAudioScreen(view, { onThemeChanged } = {}) {
  const [state, audio] = await Promise.all([
    api("get_display_state"),
    api("get_audio_state")
  ]);
  const displays = state.displays || [];
  const darkMode = !!state.dark_mode;
  const appDark = getAppTheme() === "dark";
  const volPct = Math.round((audio.volume || 0) * 100);
  const muted = !!audio.muted;
  const outDevices = audio.output_devices || [];
  const inDevices = audio.input_devices || [];

  view.innerHTML = `
    <div class="page-header">
      <div class="page-title-row">
        <div class="page-title">${t("display.page_title")}</div>
        ${renderTip(t("display.page_tip"))}
      </div>
      <div style="display:flex;align-items:center;gap:8px;">
        ${displayAudioSubTab === "displays" ? `
          <button class="btn btn-sm" id="btn-reset-gamma">${t("display.btn_reset_color")}</button>
        ` : ""}
        <button class="btn btn-sm btn-primary" id="btn-refresh-displays">${t("common.refresh")}</button>
      </div>
    </div>

    <div class="subtab-bar">
      <div class="pill-tabs" id="display-audio-subtabs">
        <button class="pill-tab ${displayAudioSubTab === 'displays' ? 'active' : ''}" data-sub="displays">
          🖥️ ${t("display.subtab_displays", { count: displays.length })}
        </button>
        <button class="pill-tab ${displayAudioSubTab === 'audio' ? 'active' : ''}" data-sub="audio">
          🔊 ${t("display.subtab_audio", { count: outDevices.length })}
        </button>
      </div>
      <div class="subtab-bar-right">
        <span class="stat-sub">
          ${displayAudioSubTab === "displays"
            ? `🖥️ ${displays.length} • 🔊 ${escapeHtml(audio.current_output_name || t("audio.default_device"))}`
            : `🔊 ${escapeHtml(audio.current_output_name || t("audio.default_device"))} (${muted ? t("audio.zero_muted") : volPct + "%"})`}
        </span>
      </div>
    </div>

    ${displayAudioSubTab === "displays" ? `
      <div class="grid-3" style="margin-bottom:12px;">
        <div class="card" style="margin-bottom:0;padding:11px 14px;">
          <div class="card-header" style="margin-bottom:0;">
            <div class="card-title" style="font-size:13px;">
              ${t("display.macos_theme_title")}
              ${renderTip(t("display.macos_theme_tip"))}
            </div>
            <div style="display:flex;gap:5px;">
              <button class="btn btn-sm ${!darkMode ? 'btn-primary' : ''}" id="btn-mode-light">${t("nav.theme_light")}</button>
              <button class="btn btn-sm ${darkMode ? 'btn-primary' : ''}" id="btn-mode-dark">${t("nav.theme_dark")}</button>
            </div>
          </div>
        </div>

        <div class="card" style="margin-bottom:0;padding:11px 14px;">
          <div class="card-header" style="margin-bottom:0;">
            <div class="card-title" style="font-size:13px;">
              ${t("display.app_theme_title")}
              ${renderTip(t("display.app_theme_tip"))}
            </div>
            <div style="display:flex;gap:5px;">
              <button class="btn btn-sm ${!appDark ? 'btn-primary' : ''}" id="btn-app-light">${t("nav.theme_light")}</button>
              <button class="btn btn-sm ${appDark ? 'btn-primary' : ''}" id="btn-app-dark">${t("nav.theme_dark")}</button>
            </div>
          </div>
        </div>

        <div class="card" style="margin-bottom:0;padding:11px 14px;">
          <div class="card-header" style="margin-bottom:0;">
            <div class="card-title" style="font-size:13px;">
              ${t("display.lang_title")}
              ${renderTip(t("display.lang_tip"))}
            </div>
            ${renderLanguageSelectHtml("sel-display-lang", "min-width:130px;padding:4px 8px;font-size:12px;")}
          </div>
        </div>
      </div>

      <div id="display-cards" style="max-height:calc(100vh - 195px);overflow-y:auto;overflow-x:hidden;padding-right:2px;">
        ${displays.map(d => `
          <div class="card" style="margin-bottom:12px;">
            <div class="card-header">
              <div class="card-title">
                🖥️ ${escapeHtml(d.name)}
                ${d.is_main ? `<span class="badge-ok">${t("display.badge_main")}</span>` : ''}
                <span class="stat-sub">(${d.width}×${d.height})</span>
              </div>
              <div style="display:flex;gap:5px;">
                <button class="btn btn-sm quick-br" data-id="${d.id}" data-val="0.25">25%</button>
                <button class="btn btn-sm quick-br" data-id="${d.id}" data-val="0.50">50%</button>
                <button class="btn btn-sm quick-br" data-id="${d.id}" data-val="0.75">75%</button>
                <button class="btn btn-sm quick-br" data-id="${d.id}" data-val="1.0">100%</button>
              </div>
            </div>

            <div class="grid-3">
              <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;">
                <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
                  <span style="font-weight:600;">${t("display.lbl_brightness")}</span>
                  <b id="lbl-br-${d.id}" style="color:var(--accent);">${Math.round(d.brightness * 100)}%</b>
                </div>
                <input type="range" min="5" max="100" value="${Math.round(d.brightness * 100)}" class="slider-br" data-id="${d.id}" />
              </div>

              <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;">
                <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
                  <span style="display:inline-flex;align-items:center;gap:4px;font-weight:600;">${t("display.lbl_extra_dimming")} ${renderTip(t("display.tip_extra_dimming"))}</span>
                  <b id="lbl-dim-${d.id}">${Math.round(d.software_dimming * 100)}%</b>
                </div>
                <input type="range" min="20" max="100" value="${Math.round(d.software_dimming * 100)}" class="slider-filter" data-id="${d.id}" />
              </div>

              <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;">
                <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
                  <span style="display:inline-flex;align-items:center;gap:4px;font-weight:600;">${t("display.lbl_color_warmth")} ${renderTip(t("display.tip_color_warmth"))}</span>
                  <b id="lbl-warm-${d.id}" style="color:var(--warning);">${Math.round(d.warmth * 100)}%</b>
                </div>
                <input type="range" min="0" max="100" value="${Math.round(d.warmth * 100)}" class="slider-warmth" data-id="${d.id}" />
              </div>
            </div>
          </div>
        `).join("")}
      </div>
    ` : `
      <!-- AUDIO OUTPUT & INPUT SWITCHER (AIRPODS / SPEAKERS / HEADPHONES) -->
      <div class="card" style="margin-bottom:0;">
        <div class="card-header" style="flex-wrap:wrap;">
          <div class="card-title">
            ${t("audio.card_title")}
            ${renderTip(t("audio.card_tip"))}
          </div>
          <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
            <button class="btn btn-sm ${muted ? 'btn-danger' : ''}" id="btn-audio-mute">
              ${muted ? t("audio.btn_muted") : t("audio.btn_unmuted")}
            </button>
            <button class="btn btn-sm quick-vol" data-val="0.25">25%</button>
            <button class="btn btn-sm quick-vol" data-val="0.50">50%</button>
            <button class="btn btn-sm quick-vol" data-val="0.75">75%</button>
            <button class="btn btn-sm quick-vol" data-val="1.0">100%</button>
          </div>
        </div>

        <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:9px;padding:10px 12px;margin-bottom:12px;">
          <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
            <span style="font-weight:600;">${t("audio.master_volume")} (${escapeHtml(audio.current_output_name || t("audio.default_device"))})</span>
            <b id="lbl-audio-vol" style="color:var(--accent);">${muted ? t("audio.zero_muted") : volPct + "%"}</b>
          </div>
          <input type="range" min="0" max="100" value="${muted ? 0 : volPct}" id="slider-audio-vol" />
        </div>

        <div style="font-size:12px;font-weight:600;color:var(--text-muted);margin-bottom:6px;">
          ${t("audio.quick_switcher_label", { count: outDevices.length })}
        </div>
        <div class="audio-device-grid" style="max-height:calc(100vh - 310px);overflow-y:auto;padding-right:2px;">
          ${outDevices.map(dev => `
            <div class="audio-device-card out-dev-card ${dev.is_default_output ? 'active' : ''}" data-id="${dev.id}">
              <div style="font-size:20px;">${dev.icon}</div>
              <div style="flex:1;min-width:0;">
                <div style="font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${escapeHtml(dev.name)}</div>
                <div class="stat-sub">${escapeHtml(tAudioKindLabel(dev.kind_label))}</div>
              </div>
              ${dev.is_default_output ? `<span class="badge-ok">${t("common.active")}</span>` : ''}
            </div>
          `).join("")}
        </div>

        ${inDevices.length > 0 ? `
          <div class="setting-row" style="margin-top:12px;padding-top:12px;border-top:1px solid var(--border);">
            <span style="font-weight:600;">${t("audio.input_mic_label")}</span>
            <select class="select-box" id="sel-audio-input" style="min-width:240px;">
              ${inDevices.map(dev => `
                <option value="${dev.id}" ${dev.is_default_input ? 'selected' : ''}>
                  ${escapeHtml(dev.name)} (${escapeHtml(tAudioKindLabel(dev.kind_label))})
                </option>
              `).join("")}
            </select>
          </div>
        ` : ''}
      </div>
    `}
  `;

  view.querySelectorAll("#display-audio-subtabs .pill-tab").forEach(btn => {
    btn.addEventListener("click", () => {
      displayAudioSubTab = btn.dataset.sub || "displays";
      renderDisplayAndAudioScreen(view, { onThemeChanged });
    });
  });

  document.getElementById("btn-refresh-displays").onclick = () =>
    renderDisplayAndAudioScreen(view, { onThemeChanged });

  if (displayAudioSubTab === "displays") {
    const resetGammaBtn = document.getElementById("btn-reset-gamma");
    if (resetGammaBtn) {
      resetGammaBtn.onclick = async () => {
        await api("reset_display_filters");
        showToast(t("display.toast_reset_color"));
        renderDisplayAndAudioScreen(view, { onThemeChanged });
      };
    }

    document.getElementById("btn-mode-light").onclick = async () => {
      await api("set_dark_mode", { dark: false });
      renderDisplayAndAudioScreen(view, { onThemeChanged });
    };
    document.getElementById("btn-mode-dark").onclick = async () => {
      await api("set_dark_mode", { dark: true });
      renderDisplayAndAudioScreen(view, { onThemeChanged });
    };
    document.getElementById("btn-app-light").onclick = () => {
      applyAppTheme("light", true);
      if (typeof onThemeChanged === "function") onThemeChanged();
    };
    document.getElementById("btn-app-dark").onclick = () => {
      applyAppTheme("dark", true);
      if (typeof onThemeChanged === "function") onThemeChanged();
    };
    bindLanguageSelect("sel-display-lang");

    view.querySelectorAll(".slider-br").forEach(slider => {
      slider.addEventListener("input", async (e) => {
        const id = Number(slider.dataset.id);
        const pct = Number(e.target.value);
        document.getElementById(`lbl-br-${id}`).textContent = pct + "%";
        await api("set_display_brightness", { display_id: id, brightness: pct / 100 });
      });
    });

    view.querySelectorAll(".quick-br").forEach(btn => {
      btn.addEventListener("click", async () => {
        await api("set_display_brightness", {
          display_id: Number(btn.dataset.id),
          brightness: Number(btn.dataset.val)
        });
        renderDisplayAndAudioScreen(view, { onThemeChanged });
      });
    });

    displays.forEach(d => {
      const dimEl = view.querySelector(`.slider-filter[data-id="${d.id}"]`);
      const warmEl = view.querySelector(`.slider-warmth[data-id="${d.id}"]`);
      const updateFilter = async () => {
        const dim = Number(dimEl.value);
        const warm = Number(warmEl.value);
        document.getElementById(`lbl-dim-${d.id}`).textContent = dim + "%";
        document.getElementById(`lbl-warm-${d.id}`).textContent = warm + "%";
        await api("set_display_color_filter", {
          display_id: d.id,
          software_dimming: dim / 100,
          warmth: warm / 100
        });
      };
      if (dimEl) dimEl.addEventListener("input", updateFilter);
      if (warmEl) warmEl.addEventListener("input", updateFilter);
    });
  } else {
    document.getElementById("slider-audio-vol").addEventListener("input", async (e) => {
      const pct = Number(e.target.value);
      document.getElementById("lbl-audio-vol").textContent = pct + "%";
      await api("set_audio_volume", { volume: pct / 100 });
    });
    document.getElementById("btn-audio-mute").onclick = async () => {
      await api("set_audio_volume", { muted: !muted });
      renderDisplayAndAudioScreen(view, { onThemeChanged });
    };
    view.querySelectorAll(".quick-vol").forEach(btn => {
      btn.addEventListener("click", async () => {
        await api("set_audio_volume", { volume: Number(btn.dataset.val), muted: false });
        renderDisplayAndAudioScreen(view, { onThemeChanged });
      });
    });
    view.querySelectorAll(".out-dev-card").forEach(card => {
      card.addEventListener("click", async () => {
        await api("set_audio_device", { device_id: Number(card.dataset.id), is_output: true });
        showToast(t("audio.toast_switched_output"));
        renderDisplayAndAudioScreen(view, { onThemeChanged });
      });
    });
    const selIn = document.getElementById("sel-audio-input");
    if (selIn) {
      selIn.onchange = async (e) => {
        await api("set_audio_device", { device_id: Number(e.target.value), is_output: false });
        showToast(t("audio.toast_switched_input"));
      };
    }
  }
}
