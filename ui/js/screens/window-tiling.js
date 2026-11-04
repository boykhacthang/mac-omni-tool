// =============================================================================
// SCREEN 4: WINDOW TILING & CUSTOM KEYBOARD SHORTCUTS (CATEGORY TABS & SCROLLABLE TABLE)
// =============================================================================
let windowTilingCatFilter = "all";

async function renderWindowTilingScreen(view) {
  const [res, cfg] = await Promise.all([
    api("get_window_shortcuts"),
    api("get_input_config")
  ]);
  const bindings = res.shortcuts || [];
  const perms = (res && res.permissions) ? res.permissions : await api("get_permissions_status");
  const axGranted = !!(perms && perms.accessibility_granted);
  const snappingEnabled = axGranted && cfg.window_snapping_enabled !== false;

  const categories = ["all", ...Array.from(new Set(bindings.map(b => b.category)))];
  if (!categories.includes(windowTilingCatFilter)) {
    windowTilingCatFilter = "all";
  }
  const filteredBindings =
    windowTilingCatFilter === "all"
      ? bindings
      : bindings.filter(b => b.category === windowTilingCatFilter);

  view.innerHTML = `
    <div class="page-header">
      <div class="page-title-row">
        <div class="page-title">${t("tiling.page_title")}</div>
        ${renderTip(t("tiling.page_tip"))}
      </div>
      <div style="display:flex;align-items:center;gap:10px;">
        <div style="display:flex;align-items:center;gap:6px;">
          <span style="font-size:12px;font-weight:600;">${snappingEnabled ? t("common.enabled") : t("common.disabled")}</span>
          <label class="switch">
            <input type="checkbox" id="chk-snapping-master" ${snappingEnabled ? 'checked' : ''} />
            <span class="slider-toggle"></span>
          </label>
        </div>
        <button class="btn btn-sm" id="btn-reset-shortcuts">${t("tiling.btn_reset_defaults")}</button>
      </div>
    </div>

    ${!axGranted ? `
      <div class="card" style="margin-bottom:10px;padding:11px 14px;border:1px solid rgba(245,158,11,0.55);background:rgba(245,158,11,0.08);display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap;">
        <div style="display:flex;flex-direction:column;gap:3px;">
          <div style="font-weight:700;font-size:13px;color:#f59e0b;">
            ${t("perm.tiling_banner_title")}
          </div>
          <div class="stat-sub" style="font-size:11.5px;">
            ${t("perm.tiling_banner_sub")}
          </div>
        </div>
        <button class="btn btn-sm btn-primary" id="btn-tiling-goto-perms">
          ${t("perm.btn_open_app_perms")}
        </button>
      </div>
    ` : ""}

    <div class="subtab-bar">
      <div class="pill-tabs" id="tiling-cat-tabs">
        ${categories.map(cat => `
          <button class="pill-tab ${windowTilingCatFilter === cat ? 'active' : ''}" data-cat="${escapeHtml(cat)}">
            ${cat === "all"
              ? t("tiling.cat_all", { count: bindings.length })
              : escapeHtml(tWindowCategory(cat))}
          </button>
        `).join("")}
      </div>
      <div class="subtab-bar-right">
        <span class="stat-sub"><b>${filteredBindings.length}</b> / ${bindings.length}</span>
      </div>
    </div>

    <div class="card" style="margin-bottom:0;${snappingEnabled ? '' : 'opacity:0.55;'}">
      <div class="table-scroll-box" style="max-height:calc(100vh - 175px);">
        <table>
          <thead>
            <tr>
              <th>${t("tiling.col_layout")}</th>
              <th>${t("tiling.col_category")}</th>
              <th>${t("tiling.col_shortcut")}</th>
              <th>${t("tiling.col_enable")}</th>
              <th style="text-align:right;">${t("tiling.col_test")}</th>
            </tr>
          </thead>
          <tbody>
            ${filteredBindings.map(b => `
              <tr style="${!b.enabled ? 'opacity:0.65;' : ''}">
                <td style="font-weight:600;">${escapeHtml(tWindowActionLabel(b.action, b.label))}</td>
                <td><span class="badge-ok">${escapeHtml(tWindowCategory(b.category))}</span></td>
                <td>
                  <div class="shortcut-recorder" data-action="${b.action}" style="${snappingEnabled ? '' : 'pointer-events:none;opacity:0.6;'}">
                    ${b.shortcut ? escapeHtml(b.shortcut) : t("tiling.set_shortcut")}
                  </div>
                </td>
                <td>
                  <label class="switch">
                    <input type="checkbox" class="toggle-shortcut-enable" data-action="${b.action}" data-shortcut="${escapeHtml(b.shortcut)}" ${b.enabled ? 'checked' : ''} ${snappingEnabled ? '' : 'disabled'} />
                    <span class="slider-toggle"></span>
                  </label>
                </td>
                <td style="text-align:right;">
                  <button class="btn btn-sm btn-test-snap" data-action="${b.action}" ${snappingEnabled && b.enabled ? '' : 'disabled'}>▶️</button>
                </td>
              </tr>
            `).join("")}
          </tbody>
        </table>
      </div>
    </div>
  `;

  view.querySelectorAll("#tiling-cat-tabs .pill-tab").forEach(btn => {
    btn.addEventListener("click", () => {
      windowTilingCatFilter = btn.dataset.cat || "all";
      renderWindowTilingScreen(view);
    });
  });

  const gotoPermsBtn = document.getElementById("btn-tiling-goto-perms");
  if (gotoPermsBtn) {
    gotoPermsBtn.onclick = () => {
      if (typeof window.openPermissionsSettings === "function") {
        window.openPermissionsSettings("accessibility");
      }
    };
  }

  document.getElementById("chk-snapping-master").onchange = async (e) => {
    if (!axGranted) {
      e.target.checked = false;
      if (typeof window.openPermissionsSettings === "function") {
        window.openPermissionsSettings("accessibility");
      }
      return;
    }
    await api("update_input_config", { ...cfg, window_snapping_enabled: e.target.checked });
    showToast(e.target.checked
      ? t("tiling.toast_enabled")
      : t("tiling.toast_disabled")
    );
    renderWindowTilingScreen(view);
  };

  document.getElementById("btn-reset-shortcuts").onclick = async () => {
    await api("reset_window_shortcuts");
    showToast(t("tiling.toast_reset"));
    renderWindowTilingScreen(view);
  };

  view.querySelectorAll(".shortcut-recorder").forEach(box => {
    box.addEventListener("click", () => {
      if (!snappingEnabled) return;
      view.querySelectorAll(".shortcut-recorder").forEach(el => el.classList.remove("recording"));
      box.classList.add("recording");
      box.textContent = t("common.press_keys");

      const handler = async (e) => {
        if (e.key === "Escape") {
          window.removeEventListener("keydown", handler, true);
          renderWindowTilingScreen(view);
          return;
        }
        if (e.key === "Backspace" || e.key === "Delete") {
          e.preventDefault();
          window.removeEventListener("keydown", handler, true);
          await api("update_window_shortcut", { action: box.dataset.action, shortcut: "", enabled: false });
          renderWindowTilingScreen(view);
          return;
        }
        const combo = eventToShortcutString(e);
        if (!combo) return;
        window.removeEventListener("keydown", handler, true);

        const resp = await api("update_window_shortcut", {
          action: box.dataset.action,
          shortcut: combo,
          enabled: true
        });
        showToast(resp.ok
          ? t("tiling.toast_saved_shortcut", { combo })
          : (resp.error || t("tiling.toast_conflict"))
        );
        renderWindowTilingScreen(view);
      };
      window.addEventListener("keydown", handler, true);
    });
  });

  view.querySelectorAll(".toggle-shortcut-enable").forEach(chk => {
    chk.addEventListener("change", async () => {
      await api("update_window_shortcut", {
        action: chk.dataset.action,
        shortcut: chk.dataset.shortcut,
        enabled: chk.checked
      });
      renderWindowTilingScreen(view);
    });
  });

  view.querySelectorAll(".btn-test-snap").forEach(btn => {
    btn.addEventListener("click", async () => {
      if (!axGranted) {
        if (typeof window.openPermissionsSettings === "function") {
          window.openPermissionsSettings("accessibility");
        }
        return;
      }
      const resp = await api("snap_window", { layout: btn.dataset.action });
      if (resp && resp.missing_permission && typeof window.openPermissionsSettings === "function") {
        window.openPermissionsSettings(resp.missing_permission);
        return;
      }
      showToast(resp.ok
        ? t("tiling.toast_snapped_action", { action: tWindowActionLabel(btn.dataset.action) })
        : (resp.error || t("tiling.toast_snap_failed"))
      );
    });
  });
}
