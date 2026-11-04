// =============================================================================
// SCREEN 5: SYSTEM CLEANER (JUNK, TRASH, DUPLICATE FILES, UNINSTALLER, LARGE FILES)
// =============================================================================
function createSystemCleanerScreen({ isTabActive }) {
  let cleanerSubTab = "junk";
  let largeFileScope = "all";
  let largeFileMinMb = 50;
  let duplicateScope = "all";
  let duplicateMinKb = 100;
  let alsoEmptyTrashOnClean = true;
  let showTrashItemsList = false;
  let showDupExcludeManager = false;
  let uninstallSearchQuery = "";
  let cleanerPollTimer = null;

  function stopPolling() {
    if (cleanerPollTimer) {
      clearTimeout(cleanerPollTimer);
      cleanerPollTimer = null;
    }
  }

  async function render(view, forceRescan = false) {
    stopPolling();

    view.innerHTML = `
      <div class="page-header">
        <div class="page-title-row">
          <div class="page-title">${t("cleaner.page_title")}</div>
          ${renderTip(t("cleaner.page_tip"))}
        </div>
        <div style="display:flex;align-items:center;gap:8px;">
          <button class="btn btn-sm btn-primary" id="btn-rescan-cleaner" title="${t('cleaner.tip_rescan')}">${t("cleaner.btn_rescan")}</button>
        </div>
      </div>

      <div class="subtab-bar">
        <div class="pill-tabs" id="cleaner-subtabs">
          <button class="pill-tab ${cleanerSubTab === 'junk' ? 'active' : ''}" data-sub="junk">${t("cleaner.subtab_junk")}</button>
          <button class="pill-tab ${cleanerSubTab === 'duplicates' ? 'active' : ''}" data-sub="duplicates">${t("cleaner.subtab_duplicates")}</button>
          <button class="pill-tab ${cleanerSubTab === 'uninstall' ? 'active' : ''}" data-sub="uninstall">${t("cleaner.subtab_uninstall")}</button>
          <button class="pill-tab ${cleanerSubTab === 'large' ? 'active' : ''}" data-sub="large">${t("cleaner.subtab_large")}</button>
        </div>
        <div class="subtab-bar-right" id="cleaner-subtab-status"></div>
      </div>
      <div id="cleaner-body"></div>
    `;

    view.querySelectorAll("#cleaner-subtabs .pill-tab").forEach(btn => {
      btn.addEventListener("click", () => {
        cleanerSubTab = btn.dataset.sub;
        render(view, false);
      });
    });

    const rescanBtn = document.getElementById("btn-rescan-cleaner");
    if (rescanBtn) {
      rescanBtn.onclick = () => render(view, true);
    }

    const body = document.getElementById("cleaner-body");
    if (!body) return;

    if (cleanerSubTab === "junk") {
      const scan = await api("scan_system_junk", { force: forceRescan });
      if (!isTabActive() || cleanerSubTab !== "junk") return;
      const items = scan.items || [];
      const trash = scan.trash || { item_count: 0, total_size_mb: 0, trash_path: "~/.Trash", items: [] };
      const trashItems = trash.items || [];

      body.innerHTML = `
        <!-- MACOS TRASH (EMPTY TRASH) -->
        <div class="card" style="border-color:rgba(245,158,11,0.35);padding:10px 14px;margin-bottom:10px;">
          <div class="card-header" style="margin-bottom:${showTrashItemsList && trashItems.length ? '8px' : '0'};flex-wrap:wrap;">
            <div class="card-title">
              ${t("cleaner.trash_card_title")}
              <span class="${trash.item_count > 0 ? 'badge-warn' : 'badge-ok'}">${trash.item_count} ${t("common.items")} • ${(trash.total_size_mb || 0).toFixed(1)} MB</span>
              ${renderTip(t("cleaner.trash_card_tip"))}
            </div>
            <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
              ${trashItems.length > 0 ? `
                <button class="btn btn-sm" id="btn-toggle-trash-items">
                  ${showTrashItemsList
                    ? t("cleaner.btn_hide_details")
                    : t("cleaner.btn_view_trash_items", { count: trashItems.length })}
                </button>
              ` : ''}
              <button class="btn btn-sm" id="btn-open-trash-finder" title="${t('cleaner.tip_open_trash')}">${t("cleaner.btn_open_trash")}</button>
              <button class="btn btn-sm btn-danger" id="btn-empty-trash-now" ${trash.item_count === 0 ? 'disabled' : ''}>
                ${t("cleaner.btn_empty_trash_now", { size: (trash.total_size_mb || 0).toFixed(1) })}
              </button>
            </div>
          </div>

          ${showTrashItemsList && trashItems.length > 0 ? `
            <div class="table-scroll-box" style="max-height:165px;">
              <table class="cleaner-table">
                <thead>
                  <tr>
                    <th style="width:36%;">${t("cleaner.col_trash_item")}</th>
                    <th style="width:14%;">${t("cleaner.col_type")}</th>
                    <th style="width:15%;">${t("cleaner.col_size")}</th>
                    <th style="width:35%;">${t("cleaner.col_path")}</th>
                  </tr>
                </thead>
                <tbody>
                  ${trashItems.map(trItem => `
                    <tr>
                      <td style="font-weight:600;" title="${escapeHtml(trItem.name)}">${trItem.is_dir ? '📁' : '📄'} ${escapeHtml(trItem.name)}</td>
                      <td><span class="badge-warn">${trItem.is_dir ? t("cleaner.kind_folder") : t("cleaner.kind_file")}</span></td>
                      <td style="font-weight:700;color:var(--warning);">${(trItem.size_mb || 0).toFixed(2)} MB</td>
                      <td class="col-path">${renderPathCell(trItem.path, true)}</td>
                    </tr>
                  `).join("")}
                </tbody>
              </table>
            </div>
          ` : ''}
        </div>

        <!-- SYSTEM CACHES & JUNK -->
        <div class="card" style="margin-bottom:0;">
          <div class="card-header" style="flex-wrap:wrap;">
            <div class="card-title">
              ${t("cleaner.junk_card_title")} <span style="color:var(--warning);">${scan.total_size_mb.toFixed(1)} MB</span>
            </div>
            <div style="display:flex;align-items:center;gap:10px;flex-wrap:wrap;">
              <label style="display:inline-flex;align-items:center;gap:5px;font-size:12px;font-weight:600;cursor:pointer;color:var(--text);">
                <input type="checkbox" id="chk-also-empty-trash" ${alsoEmptyTrashOnClean ? 'checked' : ''} />
                <span>${t("cleaner.also_empty_trash_perm")}</span>
              </label>
              <button class="btn btn-sm btn-primary" id="btn-clean-selected-junk">${t("cleaner.btn_clean_selected_count", { count: 0 })}</button>
            </div>
          </div>
          <div class="table-scroll-box" style="max-height:${showTrashItemsList && trashItems.length > 0 ? 'calc(100vh - 415px)' : 'calc(100vh - 245px)'};">
            <table class="cleaner-table">
              <thead>
                <tr>
                  <th style="width:32px;"><input type="checkbox" id="chk-all-junk" /></th>
                  <th style="width:24%;">${t("cleaner.kind_folder")}</th>
                  <th style="width:16%;">${t("cleaner.col_category_junk")}</th>
                  <th style="width:15%;">${t("cleaner.col_size")}</th>
                  <th style="width:45%;">${t("cleaner.col_path_hover")}</th>
                </tr>
              </thead>
              <tbody>
                ${items.length === 0 ? `<tr><td colspan="5" style="text-align:center;padding:20px;color:var(--text-muted);">${t("cleaner.junk_empty_row")}</td></tr>` : ""}
                ${items.map(item => `
                  <tr>
                    <td><input type="checkbox" class="chk-junk-item" value="${escapeHtml(item.path)}" /></td>
                    <td style="font-weight:600;" title="${escapeHtml(item.name)}">${escapeHtml(item.name)}</td>
                    <td><span class="badge-warn">${escapeHtml(tCleanerCategory(item.category))}</span></td>
                    <td style="font-weight:700;color:var(--accent);">${item.size_mb.toFixed(1)} MB</td>
                    <td class="col-path">${renderPathCell(item.path, true)}</td>
                  </tr>
                `).join("")}
              </tbody>
            </table>
          </div>
        </div>
      `;

      bindPathActions(body);

      const toggleTrashBtn = document.getElementById("btn-toggle-trash-items");
      if (toggleTrashBtn) {
        toggleTrashBtn.onclick = () => {
          showTrashItemsList = !showTrashItemsList;
          render(view, false);
        };
      }
      const openTrashBtn = document.getElementById("btn-open-trash-finder");
      if (openTrashBtn && trash.trash_path) {
        openTrashBtn.onclick = () => api("reveal_in_finder", { path: trash.trash_path });
      }
      const emptyTrashBtn = document.getElementById("btn-empty-trash-now");
      if (emptyTrashBtn) {
        emptyTrashBtn.onclick = async () => {
          emptyTrashBtn.disabled = true;
          emptyTrashBtn.textContent = t("cleaner.btn_emptying_trash");
          await api("empty_trash");
          showToast(t("cleaner.toast_trash_emptied"));
          render(view, true);
        };
      }

      const chkAlsoEmpty = document.getElementById("chk-also-empty-trash");
      if (chkAlsoEmpty) {
        chkAlsoEmpty.onchange = (e) => {
          alsoEmptyTrashOnClean = e.target.checked;
        };
      }

      const chkAll = document.getElementById("chk-all-junk");
      const cleanBtn = document.getElementById("btn-clean-selected-junk");
      const updateCount = () => {
        const count = body.querySelectorAll(".chk-junk-item:checked").length;
        cleanBtn.textContent = t("cleaner.btn_clean_selected_count", { count });
      };

      if (chkAll) {
        chkAll.onchange = () => {
          body.querySelectorAll(".chk-junk-item").forEach(c => c.checked = chkAll.checked);
          updateCount();
        };
      }
      body.querySelectorAll(".chk-junk-item").forEach(c => {
        c.onchange = updateCount;
      });

      cleanBtn.onclick = async () => {
        const selected = Array.from(body.querySelectorAll(".chk-junk-item:checked")).map(c => c.value);
        if (!selected.length) return showToast(t("cleaner.toast_select_to_clean"));
        await api("clean_selected_paths", {
          paths: selected,
          empty_trash: alsoEmptyTrashOnClean
        });
        showToast(t("cleaner.toast_cleaned_items", { count: selected.length }));
        render(view, true);
      };
    } else if (cleanerSubTab === "duplicates") {
      const renderDupProgressCard = (st) => {
        const pct = Math.max(4, Math.min(100, Number(st.percent || 5)));
        body.innerHTML = `
          <div class="card scan-progress-card">
            <div style="display:flex;align-items:center;justify-content:space-between;gap:10px;">
              <div style="font-weight:700;font-size:13.5px;display:flex;align-items:center;gap:8px;">
                <span>${t("cleaner.dup_progress_title")}</span>
              </div>
              <span class="badge-ok" style="font-size:12px;">${pct}%</span>
            </div>
            <div class="scan-progress-track">
              <div class="scan-progress-fill" style="width:${pct}%;"></div>
            </div>
            <div class="stat-sub" style="text-align:left;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
              ${escapeHtml(tScanMessage(st.message || t("cleaner.scan.comparing")))}
            </div>
          </div>
        `;
      };

      let st = await api("start_cleaner_scan", {
        kind: "duplicates",
        scope: duplicateScope,
        min_size_kb: duplicateMinKb,
        force: forceRescan
      });

      while (st && st.running) {
        if (!isTabActive() || cleanerSubTab !== "duplicates") return;
        renderDupProgressCard(st);
        await new Promise(r => {
          cleanerPollTimer = setTimeout(r, 140);
        });
        st = await api("get_cleaner_progress");
      }

      if (!isTabActive() || cleanerSubTab !== "duplicates" || !body) return;

      const dupData = (st && st.duplicates_result)
        ? st.duplicates_result
        : await api("scan_duplicate_files", { scope: duplicateScope, min_size_kb: duplicateMinKb });
      const groups = dupData.groups || [];
      const totalGroups = dupData.total_groups || groups.length;
      const totalDupFiles = dupData.total_duplicate_files || 0;
      const totalWastedMb = dupData.total_wasted_mb || 0;
      let excludedDirs = dupData.excluded_dirs;
      if (!Array.isArray(excludedDirs) || !excludedDirs.length) {
        const exRes = await api("get_duplicate_excludes");
        excludedDirs = (exRes && exRes.excludes) || [];
      }

      body.innerHTML = `
        ${showDupExcludeManager ? `
          <div class="card" style="margin-bottom:10px;padding:10px 14px;border:1px solid rgba(59,130,246,0.28);">
            <div class="card-header" style="margin-bottom:8px;flex-wrap:wrap;gap:8px;">
              <div class="card-title" style="display:flex;align-items:center;gap:8px;">
                <span>${t("cleaner.dup_exclude_card_title", { count: excludedDirs.length })}</span>
                ${renderTip(t("cleaner.dup_exclude_card_tip"))}
              </div>
              <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
                <span class="badge-ok">${t("cleaner.badge_scans_sql")}</span>
                <span class="badge-warn">${t("cleaner.badge_locks_sql_engine")}</span>
                <button class="btn btn-sm" id="btn-reset-dup-excludes" title="${t('cleaner.tip_reset_dup_excludes')}">${t("cleaner.btn_reset_defaults")}</button>
              </div>
            </div>

            <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px;flex-wrap:wrap;">
              <input
                type="text"
                id="inp-dup-exclude"
                class="input-box"
                style="flex:1;min-width:260px;"
                placeholder="${t('cleaner.dup_exclude_placeholder')}"
              />
              <button class="btn btn-sm btn-primary" id="btn-add-dup-exclude">${t("cleaner.btn_add_exclude_folder")}</button>
            </div>

            <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;max-height:76px;overflow-y:auto;">
              ${excludedDirs.map(dirPat => `
                <span style="display:inline-flex;align-items:center;gap:5px;padding:3px 9px;border-radius:99px;font-size:11.5px;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;background:var(--bg-input);border:1px solid var(--border);color:var(--text);">
                  <span>📁 ${escapeHtml(dirPat)}</span>
                  <button
                    class="btn-rm-dup-exclude"
                    data-pat="${escapeHtml(dirPat)}"
                    title="${t('cleaner.tip_rm_exclude')}"
                    style="border:none;background:transparent;color:var(--danger);cursor:pointer;font-weight:700;font-size:11px;padding:0 2px;"
                  >✕</button>
                </span>
              `).join("")}
            </div>
          </div>
        ` : ''}

        <div class="card" style="margin-bottom:0;">
          <div class="card-header" style="flex-wrap:wrap;gap:8px;">
            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
              <select class="select-box" id="sel-dup-scope">
                <option value="all" ${duplicateScope === 'all' ? 'selected' : ''}>${t("cleaner.scope_all_home")}</option>
                <option value="downloads" ${duplicateScope === 'downloads' ? 'selected' : ''}>~/Downloads</option>
                <option value="documents" ${duplicateScope === 'documents' ? 'selected' : ''}>~/Documents</option>
                <option value="desktop" ${duplicateScope === 'desktop' ? 'selected' : ''}>~/Desktop</option>
                <option value="pictures" ${duplicateScope === 'pictures' ? 'selected' : ''}>~/Pictures</option>
                <option value="movies" ${duplicateScope === 'movies' ? 'selected' : ''}>~/Movies</option>
              </select>
              <select class="select-box" id="sel-dup-min">
                <option value="10" ${duplicateMinKb === 10 ? 'selected' : ''}>&gt; 10 KB</option>
                <option value="100" ${duplicateMinKb === 100 ? 'selected' : ''}>&gt; 100 KB ${t("cleaner.recommended_suffix")}</option>
                <option value="1024" ${duplicateMinKb === 1024 ? 'selected' : ''}>&gt; 1 MB</option>
                <option value="10240" ${duplicateMinKb === 10240 ? 'selected' : ''}>&gt; 10 MB</option>
                <option value="51200" ${duplicateMinKb === 51200 ? 'selected' : ''}>&gt; 50 MB</option>
              </select>
              <span class="badge-warn">
                ${t("cleaner.dup_stats_badge", {
                  groups: totalGroups,
                  files: totalDupFiles,
                  wastedMb: totalWastedMb.toFixed(1)
                })}
              </span>
              <button class="btn btn-sm ${showDupExcludeManager ? 'btn-primary' : ''}" id="btn-toggle-dup-excludes" title="${t('cleaner.tip_manage_excludes')}">
                ${showDupExcludeManager
                  ? t("cleaner.btn_hide_excludes", { count: excludedDirs.length })
                  : t("cleaner.btn_show_excludes", { count: excludedDirs.length })}
              </button>
            </div>

            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
              <button class="btn btn-sm" id="btn-autoselect-dups" title="${t('cleaner.tip_autoselect_dups')}">
                ${t("cleaner.btn_autoselect_copies")}
              </button>
              <label style="display:inline-flex;align-items:center;gap:5px;font-size:12px;font-weight:600;cursor:pointer;">
                <input type="checkbox" id="chk-dup-empty-trash" ${alsoEmptyTrashOnClean ? 'checked' : ''} />
                <span>${t("cleaner.also_empty_trash_short")}</span>
              </label>
              <button class="btn btn-sm btn-danger" id="btn-clean-selected-dups">${t("cleaner.btn_delete_selected_dups", { count: 0 })}</button>
            </div>
          </div>

          ${groups.length === 0 ? `
            <div style="text-align:center;padding:28px;color:var(--text-muted);">
              ${t("cleaner.dup_empty_scope")}
            </div>
          ` : `
            <div class="table-scroll-box" style="max-height:${showDupExcludeManager ? 'calc(100vh - 375px)' : 'calc(100vh - 205px)'};">
              <table class="cleaner-table">
                <thead>
                  <tr>
                    <th style="width:32px;"><input type="checkbox" id="chk-uncheck-all-dups" title="${t('cleaner.tip_toggle_all_copies')}" /></th>
                    <th style="width:24%;">${t("cleaner.col_file_and_group")}</th>
                    <th style="width:15%;">${t("cleaner.col_status")}</th>
                    <th style="width:11%;">${t("cleaner.col_size")}</th>
                    <th style="width:46%;">${t("cleaner.col_path_and_exclude")}</th>
                  </tr>
                </thead>
                <tbody>
                  ${groups.map(g => g.files.map((f, fIdx) => {
                    const lastSlash = f.path.lastIndexOf("/");
                    const parentDir = lastSlash > 0 ? f.path.slice(0, lastSlash) : "";
                    return `
                    <tr style="${fIdx === 0 ? 'border-top:2px solid var(--border);' : ''}">
                      <td>
                        <input type="checkbox" class="chk-dup-item" data-original="${f.is_original ? '1' : '0'}" value="${escapeHtml(f.path)}" />
                      </td>
                      <td style="font-weight:600;" title="${escapeHtml(f.name)}">
                        <span style="color:var(--text-muted);font-size:11px;">#${g.group_id}</span> ${escapeHtml(f.name)}
                      </td>
                      <td>
                        ${f.is_original
                          ? `<span class="badge-ok" title="${t('cleaner.tip_original_keep')}">${t("cleaner.badge_original_keep")}</span>`
                          : `<span class="badge-warn">👯 ${t("cleaner.badge_duplicate")} (${escapeHtml(tCleanerCategory(g.kind))})</span>`
                        }
                      </td>
                      <td style="font-weight:700;color:${f.is_original ? 'var(--text-muted)' : 'var(--warning)'};">
                        ${f.size_mb >= 0.1 ? f.size_mb.toFixed(2) + ' MB' : Math.round(f.size_bytes / 1024) + ' KB'}
                      </td>
                      <td class="col-path">
                        <div style="display:flex;align-items:center;gap:6px;">
                          <div style="flex:1;min-width:0;">${renderPathCell(f.path, true)}</div>
                          ${parentDir ? `
                            <button
                              class="btn btn-sm btn-exclude-parent-dir"
                              data-dir="${escapeHtml(parentDir)}"
                              title="${escapeHtml(t('cleaner.tip_exclude_parent_dir', { dir: parentDir }))}"
                              style="padding:2px 7px;font-size:10.5px;flex-shrink:0;"
                            >${t("cleaner.btn_ignore_folder")}</button>
                          ` : ""}
                        </div>
                      </td>
                    </tr>
                  `}).join("")).join("")}
                </tbody>
              </table>
            </div>
          `}
        </div>
      `;

      bindPathActions(body);

      const toggleDupExcludesBtn = document.getElementById("btn-toggle-dup-excludes");
      if (toggleDupExcludesBtn) {
        toggleDupExcludesBtn.onclick = () => {
          showDupExcludeManager = !showDupExcludeManager;
          render(view, false);
        };
      }

      const inpExclude = document.getElementById("inp-dup-exclude");
      const btnAddExclude = document.getElementById("btn-add-dup-exclude");
      const submitAddExclude = async () => {
        if (!inpExclude) return;
        const val = (inpExclude.value || "").trim();
        if (!val) return showToast(t("cleaner.toast_enter_exclude"));
        await api("add_duplicate_exclude", { pattern: val });
        showToast(t("cleaner.toast_excluded_rescanned", { val }));
        render(view, true);
      };
      if (btnAddExclude) {
        btnAddExclude.onclick = submitAddExclude;
      }
      if (inpExclude) {
        inpExclude.onkeydown = (e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            submitAddExclude();
          }
        };
      }

      const btnResetExcludes = document.getElementById("btn-reset-dup-excludes");
      if (btnResetExcludes) {
        btnResetExcludes.onclick = async () => {
          await api("reset_duplicate_excludes");
          showToast(t("cleaner.toast_restored_excludes"));
          render(view, true);
        };
      }

      body.querySelectorAll(".btn-rm-dup-exclude").forEach(btn => {
        btn.onclick = async () => {
          const pat = btn.dataset.pat;
          if (!pat) return;
          await api("remove_duplicate_exclude", { pattern: pat });
          showToast(t("cleaner.toast_removed_exclude", { pat }));
          render(view, true);
        };
      });

      body.querySelectorAll(".btn-exclude-parent-dir").forEach(btn => {
        btn.onclick = async () => {
          const dir = btn.dataset.dir;
          if (!dir) return;
          await api("add_duplicate_exclude", { pattern: dir });
          showToast(t("cleaner.toast_excluded_dir_rescanned", { dir }));
          render(view, true);
        };
      });

      document.getElementById("sel-dup-scope").onchange = (e) => {
        duplicateScope = e.target.value;
        render(view, true);
      };
      document.getElementById("sel-dup-min").onchange = (e) => {
        duplicateMinKb = Number(e.target.value);
        render(view, true);
      };

      const chkDupEmpty = document.getElementById("chk-dup-empty-trash");
      if (chkDupEmpty) {
        chkDupEmpty.onchange = (e) => {
          alsoEmptyTrashOnClean = e.target.checked;
        };
      }

      const cleanDupBtn = document.getElementById("btn-clean-selected-dups");
      const updateDupCount = () => {
        const count = body.querySelectorAll(".chk-dup-item:checked").length;
        cleanDupBtn.textContent = t("cleaner.btn_delete_selected_dups", { count });
      };

      body.querySelectorAll(".chk-dup-item").forEach(c => {
        c.onchange = updateDupCount;
      });

      const autoSelectBtn = document.getElementById("btn-autoselect-dups");
      if (autoSelectBtn) {
        autoSelectBtn.onclick = () => {
          body.querySelectorAll(".chk-dup-item").forEach(c => {
            c.checked = c.dataset.original !== "1";
          });
          updateDupCount();
          showToast(t("cleaner.toast_autoselected_dups"));
        };
      }

      const chkToggleAllDups = document.getElementById("chk-uncheck-all-dups");
      if (chkToggleAllDups) {
        chkToggleAllDups.onchange = () => {
          body.querySelectorAll(".chk-dup-item").forEach(c => {
            c.checked = chkToggleAllDups.checked ? (c.dataset.original !== "1") : false;
          });
          updateDupCount();
        };
      }

      cleanDupBtn.onclick = async () => {
        const selected = Array.from(body.querySelectorAll(".chk-dup-item:checked")).map(c => c.value);
        if (!selected.length) {
          return showToast(t("cleaner.toast_select_dups_prompt"));
        }
        await api("clean_selected_paths", {
          paths: selected,
          empty_trash: alsoEmptyTrashOnClean
        });
        showToast(t("cleaner.toast_removed_dups", { count: selected.length }));
        render(view, true);
      };
    } else if (cleanerSubTab === "uninstall") {
      const res = await api("scan_installed_apps", { force: forceRescan });
      if (!isTabActive() || cleanerSubTab !== "uninstall") return;
      const apps = res.apps || [];
      const orphans = res.orphans || [];

      const qLower = (uninstallSearchQuery || "").trim().toLowerCase();
      const filteredApps = qLower
        ? apps.filter((a) =>
            (a.name || "").toLowerCase().includes(qLower) ||
            (a.bundle_id || "").toLowerCase().includes(qLower) ||
            (a.app_path || "").toLowerCase().includes(qLower)
          )
        : apps;

      body.innerHTML = `
        ${orphans.length > 0 ? `
          <div class="card" style="margin-bottom:10px;padding:10px 14px;border:1px solid rgba(239,68,68,0.45);background:rgba(239,68,68,0.06);">
            <div class="card-header" style="flex-wrap:wrap;gap:8px;margin-bottom:6px;">
              <div class="card-title" style="display:flex;align-items:center;gap:8px;">
                <span>${t("cleaner.orphans_card_title", { count: orphans.length })}</span>
                ${renderTip(t("cleaner.orphans_card_tip"))}
              </div>
              <button class="btn btn-sm btn-danger" id="btn-clean-orphaned-bg">
                ${t("cleaner.btn_kill_clean_orphans", { count: orphans.length })}
              </button>
            </div>
            <div class="table-scroll-box" style="max-height:140px;">
              <table class="cleaner-table">
                <thead>
                  <tr>
                    <th style="width:26%;">${t("cleaner.col_orphan_name")}</th>
                    <th style="width:18%;">${t("cleaner.col_type")}</th>
                    <th style="width:56%;">${t("cleaner.col_orphan_target")}</th>
                  </tr>
                </thead>
                <tbody>
                  ${orphans.map(o => `
                    <tr>
                      <td style="font-weight:700;color:var(--danger);">${escapeHtml(o.name)}${o.pid ? ` <span class="badge-warn">PID ${o.pid}</span>` : ''}</td>
                      <td>
                        <span class="badge-warn">
                          ${o.kind === 'Running Process'
                            ? t("cleaner.orphan_running")
                            : t("cleaner.orphan_launchagent")}
                        </span>
                      </td>
                      <td class="col-path">${renderPathCell(o.plist_path || o.target_path, true)}</td>
                    </tr>
                  `).join("")}
                </tbody>
              </table>
            </div>
          </div>
        ` : ''}

        <div class="card" style="margin-bottom:0;">
          <div class="card-header" style="flex-wrap:wrap;gap:10px;">
            <div style="display:flex;align-items:center;gap:10px;flex-wrap:wrap;">
              <div class="card-title">
                ${t("cleaner.apps_title_filtered", { shown: filteredApps.length, total: apps.length })}
                ${renderTip(t("cleaner.apps_card_tip"))}
              </div>
              <input
                type="text"
                id="inp-search-uninstall-apps"
                class="input-box"
                style="width:220px;"
                placeholder="${t('cleaner.search_apps_placeholder')}"
                value="${escapeHtml(uninstallSearchQuery)}"
              />
            </div>
            <label style="display:inline-flex;align-items:center;gap:5px;font-size:12px;font-weight:600;cursor:pointer;">
              <input type="checkbox" id="chk-uninstall-empty-trash" ${alsoEmptyTrashOnClean ? 'checked' : ''} />
              <span>${t("cleaner.also_empty_trash_perm")}</span>
            </label>
          </div>
          <div class="table-scroll-box" style="max-height:${orphans.length > 0 ? 'calc(100vh - 385px)' : 'calc(100vh - 205px)'};">
            <table class="cleaner-table">
              <thead>
                <tr>
                  <th style="width:34%;">${t("cleaner.col_app")}</th>
                  <th style="width:12%;">${t("cleaner.col_app_size")}</th>
                  <th style="width:12%;">${t("cleaner.col_leftovers_short")}</th>
                  <th style="width:26%;">${t("cleaner.col_path")}</th>
                  <th style="width:16%;text-align:right;">${t("cleaner.col_uninstall")}</th>
                </tr>
              </thead>
              <tbody id="uninstall-apps-tbody">
                ${filteredApps.map((app) => {
                  const idx = apps.indexOf(app);
                  const pidCount = (app.running_pids && app.running_pids.length) || 0;
                  return `
                  <tr class="uninstall-app-row" data-search="${escapeHtml((app.name + ' ' + app.bundle_id + ' ' + app.app_path).toLowerCase())}">
                    <td>
                      <div class="app-cell">
                        ${app.icon_data_url
                          ? `<img class="app-icon-img" src="${app.icon_data_url}" alt="" />`
                          : `<div class="app-icon-fallback">📦</div>`
                        }
                        <div class="app-name-wrap">
                          <div class="app-name-title" title="${escapeHtml(app.name)}" style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
                            <span>${escapeHtml(app.name)}</span>
                            ${pidCount > 0
                              ? `<span class="badge-ok" style="font-size:10.5px;padding:1px 6px;" title="PID: ${app.running_pids.join(', ')}">${t("cleaner.badge_app_running", { count: pidCount })}</span>`
                              : ''
                            }
                          </div>
                          <div class="app-name-sub">v${escapeHtml(app.version)} • ${t("cleaner.total_label")} ${app.total_size_mb.toFixed(1)} MB • ${app.related_files.length} ${t("common.items")}</div>
                        </div>
                      </div>
                    </td>
                    <td>${app.app_size_mb.toFixed(1)} MB</td>
                    <td style="color:var(--warning);font-weight:600;">${(app.leftovers_size_bytes / (1024 * 1024)).toFixed(1)} MB</td>
                    <td class="col-path">${renderPathCell(app.app_path, true)}</td>
                    <td style="text-align:right;">
                      <button class="btn btn-sm btn-danger btn-uninstall-app" data-idx="${idx}">${t("cleaner.btn_uninstall")}</button>
                    </td>
                  </tr>
                `;
                }).join("")}
              </tbody>
            </table>
          </div>
        </div>
      `;

      bindPathActions(body);

      const inpAppSearch = document.getElementById("inp-search-uninstall-apps");
      if (inpAppSearch) {
        inpAppSearch.oninput = (e) => {
          uninstallSearchQuery = e.target.value || "";
          const q = uninstallSearchQuery.trim().toLowerCase();
          body.querySelectorAll(".uninstall-app-row").forEach((row) => {
            const hay = row.dataset.search || "";
            row.style.display = !q || hay.includes(q) ? "" : "none";
          });
        };
      }

      const chkUninstallEmpty = document.getElementById("chk-uninstall-empty-trash");
      if (chkUninstallEmpty) {
        chkUninstallEmpty.onchange = (e) => {
          alsoEmptyTrashOnClean = e.target.checked;
        };
      }

      const btnCleanOrphans = document.getElementById("btn-clean-orphaned-bg");
      if (btnCleanOrphans) {
        btnCleanOrphans.onclick = async () => {
          btnCleanOrphans.disabled = true;
          await api("clean_orphaned_background");
          showToast(t("cleaner.toast_cleaned_orphans"));
          render(view, true);
        };
      }

      body.querySelectorAll(".btn-uninstall-app").forEach(btn => {
        btn.addEventListener("click", async () => {
          const app = apps[Number(btn.dataset.idx)];
          if (!app) return;
          btn.disabled = true;
          const paths = app.related_files.map(f => f.path);
          await api("uninstall_app", {
            app_path: app.app_path,
            app_name: app.name,
            bundle_id: app.bundle_id,
            paths,
            empty_trash: alsoEmptyTrashOnClean
          });
          showToast(t("cleaner.toast_uninstalled_app", { name: app.name }));
          render(view, true);
        });
      });
    } else if (cleanerSubTab === "large") {
      const renderProgressCard = (st) => {
        const pct = Math.max(4, Math.min(100, Number(st.percent || 5)));
        body.innerHTML = `
          <div class="card scan-progress-card">
            <div style="display:flex;align-items:center;justify-content:space-between;gap:10px;">
              <div style="font-weight:700;font-size:13.5px;display:flex;align-items:center;gap:8px;">
                <span>${t("cleaner.large_progress_title")}</span>
              </div>
              <span class="badge-ok" style="font-size:12px;">${pct}%</span>
            </div>
            <div class="scan-progress-track">
              <div class="scan-progress-fill" style="width:${pct}%;"></div>
            </div>
            <div class="stat-sub" style="text-align:left;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
              ${escapeHtml(tScanMessage(st.message || t("cleaner.scan.filesystem")))}
            </div>
          </div>
        `;
      };

      let st = await api("start_cleaner_scan", {
        kind: "large",
        scope: largeFileScope,
        min_size_mb: largeFileMinMb,
        force: forceRescan
      });

      while (st && st.running) {
        if (!isTabActive() || cleanerSubTab !== "large") return;
        renderProgressCard(st);
        await new Promise(r => {
          cleanerPollTimer = setTimeout(r, 140);
        });
        st = await api("get_cleaner_progress");
      }

      if (!isTabActive() || cleanerSubTab !== "large" || !body) return;

      const res = (st && st.large)
        ? { files: st.large }
        : await api("scan_large_files", { scope: largeFileScope, min_size_mb: largeFileMinMb });
      const files = res.files || [];
      body.innerHTML = `
        <div class="card" style="margin-bottom:0;">
          <div class="card-header" style="flex-wrap:wrap;">
            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
              <select class="select-box" id="sel-large-scope">
                <option value="all" ${largeFileScope === 'all' ? 'selected' : ''}>${t("cleaner.scope_user_home")}</option>
                <option value="downloads" ${largeFileScope === 'downloads' ? 'selected' : ''}>~/Downloads</option>
                <option value="documents" ${largeFileScope === 'documents' ? 'selected' : ''}>~/Documents</option>
                <option value="movies" ${largeFileScope === 'movies' ? 'selected' : ''}>~/Movies</option>
                <option value="desktop" ${largeFileScope === 'desktop' ? 'selected' : ''}>~/Desktop</option>
              </select>
              <select class="select-box" id="sel-large-min">
                <option value="20" ${largeFileMinMb === 20 ? 'selected' : ''}>&gt; 20 MB</option>
                <option value="50" ${largeFileMinMb === 50 ? 'selected' : ''}>&gt; 50 MB</option>
                <option value="100" ${largeFileMinMb === 100 ? 'selected' : ''}>&gt; 100 MB</option>
                <option value="500" ${largeFileMinMb === 500 ? 'selected' : ''}>&gt; 500 MB</option>
                <option value="1024" ${largeFileMinMb === 1024 ? 'selected' : ''}>&gt; 1 GB</option>
              </select>
            </div>
            <span class="stat-sub"><b>${files.length}</b> ${t("common.files")}</span>
          </div>

          <div class="table-scroll-box" style="max-height:calc(100vh - 205px);">
            <table class="cleaner-table">
              <thead>
                <tr>
                  <th style="width:25%;">${t("cleaner.col_file_name")}</th>
                  <th style="width:12%;">${t("cleaner.col_type")}</th>
                  <th style="width:14%;">${t("cleaner.col_size")}</th>
                  <th style="width:39%;">${t("cleaner.col_path_hover")}</th>
                  <th style="width:10%;text-align:right;">${t("cleaner.col_delete")}</th>
                </tr>
              </thead>
              <tbody>
                ${files.length === 0 ? `<tr><td colspan="5" style="text-align:center;padding:22px;color:var(--text-muted);">${t("cleaner.large_empty", { minMb: largeFileMinMb })}</td></tr>` : ''}
                ${files.map(f => `
                  <tr>
                    <td style="font-weight:600;" title="${escapeHtml(f.name)}">${escapeHtml(f.name)}</td>
                    <td><span class="badge-ok">${escapeHtml(tCleanerCategory(f.kind))}</span></td>
                    <td style="font-weight:700;color:var(--warning);">${f.size_mb.toFixed(1)} MB</td>
                    <td class="col-path">${renderPathCell(f.path, true)}</td>
                    <td style="text-align:right;">
                      <button class="btn btn-sm btn-danger btn-trash-file" data-path="${escapeHtml(f.path)}" title="${t('cleaner.tip_move_trash')}">🗑️</button>
                    </td>
                  </tr>
                `).join("")}
              </tbody>
            </table>
          </div>
        </div>
      `;

      bindPathActions(body);

      document.getElementById("sel-large-scope").onchange = (e) => {
        largeFileScope = e.target.value;
        render(view, true);
      };
      document.getElementById("sel-large-min").onchange = (e) => {
        largeFileMinMb = Number(e.target.value);
        render(view, true);
      };
      body.querySelectorAll(".btn-trash-file").forEach(btn => {
        btn.addEventListener("click", async () => {
          await api("clean_selected_paths", { paths: [btn.dataset.path] });
          showToast(t("cleaner.toast_moved_trash"));
          render(view, true);
        });
      });
    }
  }

  return {
    render,
    stopPolling
  };
}
