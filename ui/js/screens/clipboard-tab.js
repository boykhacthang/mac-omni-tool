// =============================================================================
// SCREEN 3: CLIPBOARD HISTORY & PINNED ITEMS TAB (COMPACT STAT STRIP & SCROLLABLE LIST)
// =============================================================================
function createClipboardScreen() {
  let clipFilter = "all";
  let clipQuery = "";
  let clipPage = 1;
  let clipPageSize = getSavedClipPageSize();

  async function render(view) {
    clipPageSize = getSavedClipPageSize();
    const [res, cfg] = await Promise.all([
      api("query_clipboard", {
        query: clipQuery,
        filter_kind: clipFilter,
        page: clipPage,
        page_size: clipPageSize
      }),
      api("get_input_config")
    ]);
    const items = res.items || [];
    clipPage = res.page || 1;
    const totalPages = res.total_pages || 1;
    const totalMatching = res.total_matching || 0;
    const stats = res.stats || {};
    const enabled = cfg.clipboard_enabled !== false;
    const offset = (clipPage - 1) * clipPageSize;

    view.innerHTML = `
      <div class="page-header">
        <div class="page-title-row">
          <div class="page-title">📋 ${t("nav.clipboard")}</div>
          ${renderTip(t("clipboard.page_tip"))}
        </div>
        <div style="display:flex;align-items:center;gap:10px;">
          <div style="display:flex;align-items:center;gap:6px;">
            <span style="font-size:12px;font-weight:600;">${enabled ? t("common.enabled") : t("common.disabled")}</span>
            <label class="switch">
              <input type="checkbox" id="chk-clip-enable" ${enabled ? 'checked' : ''} />
              <span class="slider-toggle"></span>
            </label>
          </div>
          <button class="btn btn-sm btn-primary" id="btn-open-left-drawer" ${enabled ? '' : 'disabled'}>
            ${t("clipboard.btn_open_drawer", { shortcut: escapeHtml(cfg.clipboard_popup_shortcut || 'Cmd+Shift+V') })}
          </button>
          <button class="btn btn-sm btn-danger" id="btn-clear-clips">${t("clipboard.clear_unpinned_btn")}</button>
        </div>
      </div>

      <div class="subtab-bar">
        <div class="pill-tabs" id="clip-filter-tabs">
          <button class="pill-tab ${clipFilter === 'all' ? 'active' : ''}" data-kind="all">${t("clipboard.filter_all")}</button>
          <button class="pill-tab ${clipFilter === 'text' ? 'active' : ''}" data-kind="text">${t("clipboard.filter_text")}</button>
          <button class="pill-tab ${clipFilter === 'image' ? 'active' : ''}" data-kind="image">${t("clipboard.filter_image")}</button>
          <button class="pill-tab ${clipFilter === 'pinned' ? 'active' : ''}" data-kind="pinned">${t("clipboard.filter_pinned_tab")}</button>
        </div>
        <div class="subtab-bar-right">
          <div class="stat-pill">
            <span>📦 ${t("clipboard.stat_stored")}</span>
            <b style="color:var(--accent);">${stats.total_items || 0} / ${stats.max_capacity || 1000}</b>
          </div>
          <div class="stat-pill">
            <span>🖼️ ${t("clipboard.stat_images")}</span>
            <b>${stats.image_items || 0}</b>
            <span>(${formatBytes(stats.disk_usage_bytes || 0)})</span>
          </div>
          <div class="stat-pill">
            <span>📌 ${t("clipboard.stat_pinned")}</span>
            <b style="color:var(--success);">${stats.pinned_items || 0}</b>
          </div>
        </div>
      </div>

      <div class="card" style="margin-bottom:0;">
        <div class="card-header" style="flex-wrap:wrap;gap:8px;">
          <input id="clip-search-input" class="input-text" style="width:280px;" placeholder="${t("clipboard.search_tab_placeholder")}" value="${escapeHtml(clipQuery)}" />
          <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
            <select class="select-box" id="clip-page-size-select" title="${t("clipboard.tip_per_page")}" style="padding:4px 8px;font-size:12px;">
              <option value="50" ${clipPageSize === 50 ? 'selected' : ''}>${t("common.per_page", { size: 50 })}</option>
              <option value="100" ${clipPageSize === 100 ? 'selected' : ''}>${t("common.per_page", { size: 100 })}</option>
              <option value="200" ${clipPageSize === 200 ? 'selected' : ''}>${t("common.per_page", { size: 200 })}</option>
              <option value="500" ${clipPageSize === 500 ? 'selected' : ''}>${t("common.per_page", { size: 500 })}</option>
              <option value="1000" ${clipPageSize === 1000 ? 'selected' : ''}>${t("common.per_page", { size: 1000 })}</option>
            </select>
            <div style="display:flex;align-items:center;gap:5px;">
              <button class="btn btn-sm" id="clip-btn-prev" ${clipPage <= 1 ? 'disabled' : ''}>${t("clipboard.btn_prev")}</button>
              <span class="badge-ok">${t("clipboard.page_status", { page: clipPage, totalPages, shown: items.length, total: totalMatching })}</span>
              <button class="btn btn-sm" id="clip-btn-next" ${clipPage >= totalPages ? 'disabled' : ''}>${t("clipboard.btn_next")}</button>
            </div>
          </div>
        </div>

        <div id="main-clip-list" style="max-height:calc(100vh - 190px);overflow-y:auto;padding-right:2px;">
          ${renderFullClipboardCardsHtml(items, -1, t("clipboard.btn_copy_item"), offset)}
        </div>
      </div>
    `;

    document.getElementById("chk-clip-enable").onchange = async (e) => {
      await api("set_clipboard_enabled", { enabled: e.target.checked });
      showToast(e.target.checked
        ? t("clipboard.toast_enabled")
        : t("clipboard.toast_paused")
      );
      render(view);
    };
    document.getElementById("btn-open-left-drawer").onclick = () => api("toggle_overlay", { window: "clipboard" });
    document.getElementById("btn-clear-clips").onclick = async () => {
      await api("clear_clipboard");
      clipPage = 1;
      showToast(t("clipboard.toast_cleared_history"));
      render(view);
    };

    document.getElementById("clip-page-size-select").onchange = (e) => {
      clipPageSize = Number(e.target.value);
      setSavedClipPageSize(clipPageSize);
      clipPage = 1;
      render(view);
    };
    document.getElementById("clip-btn-prev").onclick = () => {
      if (clipPage > 1) {
        clipPage -= 1;
        render(view);
      }
    };
    document.getElementById("clip-btn-next").onclick = () => {
      if (clipPage < totalPages) {
        clipPage += 1;
        render(view);
      }
    };

    document.getElementById("clip-search-input").addEventListener("input", (e) => {
      clipQuery = e.target.value;
      clipPage = 1;
      render(view);
    });

    view.querySelectorAll("#clip-filter-tabs .pill-tab").forEach(btn => {
      btn.addEventListener("click", () => {
        clipFilter = btn.dataset.kind;
        clipPage = 1;
        render(view);
      });
    });

    const listEl = document.getElementById("main-clip-list");
    listEl.querySelectorAll(".copy-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        await api("paste_clipboard_item", { id: Number(btn.dataset.id), simulate_paste: false });
        showToast(t("clipboard.toast_copied_item"));
      });
    });
    listEl.querySelectorAll(".pin-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        await api("toggle_pin_clipboard", { id: Number(btn.dataset.id) });
        render(view);
      });
    });
    listEl.querySelectorAll(".del-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        await api("delete_clipboard_item", { id: Number(btn.dataset.id) });
        render(view);
      });
    });
  }

  return { render };
}
