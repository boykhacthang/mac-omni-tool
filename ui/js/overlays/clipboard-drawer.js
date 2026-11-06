// =============================================================================
// OVERLAY MODE 1: LEFT-SIDE CLIPBOARD DRAWER (Cmd+Shift+V)
// =============================================================================
async function initClipboardDrawer() {
  const root = document.getElementById("root");
  let items = [];
  let filterKind = "all";
  let searchQuery = "";
  let selectedIdx = 0;
  let drawerPage = 1;
  let drawerTotalPages = 1;
  let drawerTotalMatching = 0;
  let drawerPageSize = getSavedClipPageSize();

  function renderDrawerShell() {
    root.innerHTML = `
      <div class="drawer-shell">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px;">
          <div style="display:flex;align-items:center;gap:6px;">
            <div style="font-weight:700;font-size:14px;color:var(--accent);">📋 Clipboard</div>
            <span id="drawer-sub" class="badge-ok">0 / 1000</span>
          </div>
          <div style="display:flex;align-items:center;gap:5px;">
            <button class="btn btn-sm" id="btn-drawer-open-dashboard" title="${t("popover.open_dashboard")}" style="padding:3px 7px;">🚀</button>
            <button class="btn btn-sm" id="btn-close-drawer">✕ Esc</button>
          </div>
        </div>
        <input id="drawer-search" class="input-text" placeholder="${t("clipboard.search_placeholder")}" value="${escapeHtml(searchQuery)}" style="width:100%;margin-bottom:7px;" autofocus />
        <div class="pill-tabs" style="width:100%;justify-content:space-between;">
          <button class="pill-tab ${filterKind === 'all' ? 'active' : ''}" data-kind="all">${t("clipboard.filter_all")}</button>
          <button class="pill-tab ${filterKind === 'text' ? 'active' : ''}" data-kind="text">${t("clipboard.filter_text")}</button>
          <button class="pill-tab ${filterKind === 'image' ? 'active' : ''}" data-kind="image">${t("clipboard.filter_image")}</button>
          <button class="pill-tab ${filterKind === 'pinned' ? 'active' : ''}" data-kind="pinned">${t("clipboard.filter_pinned")}</button>
        </div>
        <div id="drawer-list" class="clip-list-viewport"></div>
        <div style="display:flex;align-items:center;justify-content:space-between;gap:6px;padding-top:8px;border-top:1px solid var(--border);margin-top:6px;">
          <div style="display:flex;align-items:center;gap:4px;">
            <button class="btn btn-sm" id="drawer-prev-page" style="padding:3px 7px;">◀</button>
            <span id="drawer-page-label" style="font-size:11px;color:var(--text-muted);font-weight:600;">${t("common.page")} 1/1</span>
            <button class="btn btn-sm" id="drawer-next-page" style="padding:3px 7px;">▶</button>
          </div>
          <select class="select-box" id="drawer-page-size" style="padding:3px 7px;font-size:11px;">
            <option value="50" ${drawerPageSize === 50 ? 'selected' : ''}>${t("common.per_page", { size: 50 })}</option>
            <option value="100" ${drawerPageSize === 100 ? 'selected' : ''}>${t("common.per_page", { size: 100 })}</option>
            <option value="200" ${drawerPageSize === 200 ? 'selected' : ''}>${t("common.per_page", { size: 200 })}</option>
            <option value="500" ${drawerPageSize === 500 ? 'selected' : ''}>${t("common.per_page", { size: 500 })}</option>
            <option value="1000" ${drawerPageSize === 1000 ? 'selected' : ''}>${t("common.per_page", { size: 1000 })}</option>
          </select>
        </div>
      </div>
    `;

    document.getElementById("btn-close-drawer").onclick = () => api("hide_overlay", { window: "clipboard" });
    const openDashBtn = document.getElementById("btn-drawer-open-dashboard");
    if (openDashBtn) {
      openDashBtn.onclick = async () => {
        await api("hide_overlay", { window: "clipboard" });
        await api("show_launcher");
      };
    }
    document.getElementById("drawer-prev-page").onclick = () => {
      if (drawerPage > 1) {
        drawerPage -= 1;
        selectedIdx = 0;
        refreshList();
      }
    };
    document.getElementById("drawer-next-page").onclick = () => {
      if (drawerPage < drawerTotalPages) {
        drawerPage += 1;
        selectedIdx = 0;
        refreshList();
      }
    };
    document.getElementById("drawer-page-size").onchange = (e) => {
      drawerPageSize = Number(e.target.value);
      setSavedClipPageSize(drawerPageSize);
      drawerPage = 1;
      selectedIdx = 0;
      refreshList();
    };

    document.getElementById("drawer-search").addEventListener("input", (e) => {
      searchQuery = e.target.value;
      drawerPage = 1;
      selectedIdx = 0;
      refreshList();
    });

    root.querySelectorAll(".pill-tab").forEach(btn => {
      btn.addEventListener("click", () => {
        root.querySelectorAll(".pill-tab").forEach(b => b.classList.remove("active"));
        btn.classList.add("active");
        filterKind = btn.dataset.kind;
        drawerPage = 1;
        selectedIdx = 0;
        refreshList();
      });
    });
  }

  async function refreshList() {
    const res = await api("query_clipboard", {
      query: searchQuery,
      filter_kind: filterKind,
      page: drawerPage,
      page_size: drawerPageSize
    });
    items = res.items || [];
    drawerPage = res.page || 1;
    drawerTotalPages = res.total_pages || 1;
    drawerTotalMatching = res.total_matching || 0;
    const stats = res.stats || {};
    const subEl = document.getElementById("drawer-sub");
    if (subEl) {
      subEl.textContent = `${stats.total_items || drawerTotalMatching}/${stats.max_capacity || 1000}`;
    }

    const pageLbl = document.getElementById("drawer-page-label");
    if (pageLbl) {
      pageLbl.textContent = t("clipboard.page_status", {
        page: drawerPage,
        totalPages: drawerTotalPages,
        shown: items.length,
        total: drawerTotalMatching
      });
    }
    const prevBtn = document.getElementById("drawer-prev-page");
    const nextBtn = document.getElementById("drawer-next-page");
    if (prevBtn) prevBtn.disabled = drawerPage <= 1;
    if (nextBtn) nextBtn.disabled = drawerPage >= drawerTotalPages;

    renderRows();
  }

  function renderRows() {
    const listEl = document.getElementById("drawer-list");
    if (!listEl) return;
    const offset = (drawerPage - 1) * drawerPageSize;
    listEl.innerHTML = renderFullClipboardCardsHtml(items, selectedIdx, t("clipboard.paste"), offset);

    listEl.querySelectorAll(".clip-card").forEach(card => {
      card.addEventListener("click", async (e) => {
        if (e.target.classList.contains("pin-btn") || e.target.classList.contains("del-btn")) return;
        const id = Number(card.dataset.id);
        await api("paste_clipboard_item", { id, simulate_paste: true });
        await api("hide_overlay", { window: "clipboard" });
      });
    });

    listEl.querySelectorAll(".pin-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        await api("toggle_pin_clipboard", { id: Number(btn.dataset.id) });
        refreshList();
      });
    });

    listEl.querySelectorAll(".del-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        await api("delete_clipboard_item", { id: Number(btn.dataset.id) });
        refreshList();
      });
    });
  }

  window.addEventListener("keydown", async (e) => {
    if (e.key === "Escape") {
      await api("hide_overlay", { window: "clipboard" });
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIdx = Math.min(items.length - 1, selectedIdx + 1);
      renderRows();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIdx = Math.max(0, selectedIdx - 1);
      renderRows();
    } else if (e.key === "Enter" && items[selectedIdx]) {
      e.preventDefault();
      await api("paste_clipboard_item", { id: items[selectedIdx].id, simulate_paste: true });
      await api("hide_overlay", { window: "clipboard" });
    }
  });

  window.__onThemeChanged = () => applyAppTheme(getAppTheme(), false);
  window.__onLangChanged = () => {
    renderDrawerShell();
    refreshList();
  };
  window.__refreshDrawer = () => {
    applyAppTheme(getAppTheme(), false);
    applyAppLang(getAppLang(), false);
    drawerPageSize = getSavedClipPageSize();
    renderDrawerShell();
    drawerPage = 1;
    selectedIdx = 0;
    const inp = document.getElementById("drawer-search");
    if (inp) inp.focus();
    refreshList();
  };
  window.__onDrawerHidden = () => {
    items = [];
    const listEl = document.getElementById("drawer-list");
    if (listEl) listEl.innerHTML = "";
  };

  renderDrawerShell();
}
