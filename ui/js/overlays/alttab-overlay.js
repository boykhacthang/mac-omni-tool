// =============================================================================
// OVERLAY MODE 2: WINDOWS-STYLE ALT-TAB LIVE PREVIEW OVERLAY (Cmd+Tab)
// =============================================================================
async function initAltTabOverlay() {
  const root = document.getElementById("root");
  let windows = [];
  let selectedIdx = 0;

  function renderAltTabShell() {
    root.innerHTML = `
      <div class="win-alttab-backdrop">
        <div id="alttab-selected-label" class="win-alttab-selected-title">${t("alttab.loading")}</div>
        <div id="alttab-grid" class="win-alttab-grid"></div>
        <div style="display:flex;justify-content:center;gap:14px;margin-top:8px;font-size:11.5px;color:var(--text-muted);">
          <span>${t("alttab.hint_cycle")}</span>
          <span>• ${t("alttab.hint_esc")}</span>
        </div>
      </div>
    `;
  }

  async function commitSelection(idx = selectedIdx) {
    const w = windows[idx];
    if (w) {
      await api("focus_window", { window_id: w.window_id, owner_pid: w.owner_pid });
    }
    await api("hide_overlay", { window: "alttab" });
  }

  async function loadWindows() {
    applyAppTheme(getAppTheme(), false);
    applyAppLang(getAppLang(), false);
    const res = await api("list_open_windows", { with_previews: true });
    windows = res.windows || [];
    selectedIdx = windows.length > 1 ? 1 : 0;
    renderGrid();
  }

  function renderGrid() {
    const grid = document.getElementById("alttab-grid");
    const titleEl = document.getElementById("alttab-selected-label");
    if (!grid || !titleEl) return;
    if (!windows.length) {
      titleEl.textContent = t("alttab.empty");
      grid.innerHTML = "";
      return;
    }
    const cur = windows[selectedIdx] || windows[0];
    titleEl.textContent = `${cur.app_name} — ${cur.title || cur.app_name}`;

    grid.innerHTML = windows.map((w, idx) => {
      const initial = (w.app_name || "W").trim().charAt(0).toUpperCase();
      return `
        <div class="win-alttab-card ${idx === selectedIdx ? 'selected' : ''}" data-idx="${idx}">
          <div class="win-alttab-card-header">
            <div class="win-alttab-app-badge">${escapeHtml(initial)}</div>
            <div style="flex:1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
              ${escapeHtml(w.title || w.app_name)}
            </div>
            <span style="font-size:10.5px;color:var(--text-muted);">${escapeHtml(w.app_name)}</span>
          </div>
          ${w.preview_data_url
            ? `<img class="win-alttab-thumb" src="${w.preview_data_url}" />`
            : `<div class="win-alttab-thumb" style="display:flex;align-items:center;justify-content:center;color:var(--text-muted);font-size:12px;">🪟 ${escapeHtml(w.app_name)}</div>`
          }
        </div>
      `;
    }).join("");

    grid.querySelectorAll(".win-alttab-card").forEach(card => {
      card.addEventListener("click", () => {
        commitSelection(Number(card.dataset.idx));
      });
    });

    const activeCard = grid.querySelector(".win-alttab-card.selected");
    if (activeCard) {
      activeCard.scrollIntoView({ block: "nearest", inline: "nearest" });
    }
  }

  window.addEventListener("keydown", async (e) => {
    if (e.key === "Escape") {
      await api("hide_overlay", { window: "alttab" });
    } else if (e.key === "ArrowRight" || e.key === "Tab") {
      e.preventDefault();
      if (windows.length) {
        selectedIdx = e.shiftKey
          ? (selectedIdx - 1 + windows.length) % windows.length
          : (selectedIdx + 1) % windows.length;
        renderGrid();
      }
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (windows.length) {
        selectedIdx = (selectedIdx - 1 + windows.length) % windows.length;
        renderGrid();
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      await commitSelection(selectedIdx);
    }
  });

  window.addEventListener("keyup", async (e) => {
    if (e.key === "Alt" || e.key === "Meta") {
      await commitSelection(selectedIdx);
    }
  });

  window.__onThemeChanged = () => applyAppTheme(getAppTheme(), false);
  window.__onLangChanged = () => {
    renderAltTabShell();
    renderGrid();
  };
  window.__refreshAltTab = () => {
    applyAppTheme(getAppTheme(), false);
    applyAppLang(getAppLang(), false);
    renderAltTabShell();
    loadWindows();
  };
  window.__cycleAltTabNext = () => {
    if (windows.length) {
      selectedIdx = (selectedIdx + 1) % windows.length;
      renderGrid();
    }
  };
  window.__commitAltTabSelection = () => {
    commitSelection(selectedIdx);
  };
  window.__onAltTabHidden = () => {
    windows = [];
    const grid = document.getElementById("alttab-grid");
    if (grid) grid.innerHTML = "";
  };

  renderAltTabShell();
}
