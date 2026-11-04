// =============================================================================
// REUSABLE COMPONENT: COMPACT PATH PILL WITH HOVER TOOLTIP, COPY & FINDER REVEAL
// =============================================================================
function shortenPath(fullPath) {
  if (!fullPath) return "";
  const s = String(fullPath).replace(/^\/Users\/[^/]+/, "~");
  const parts = s.split("/").filter(Boolean);
  if (parts.length <= 3) return s;
  const first = s.startsWith("~") ? "~" : "/" + parts[0];
  const lastTwo = parts.slice(-2).join("/");
  return `${first}/…/${lastTwo}`;
}

function renderPathCell(fullPath, includeFinderBtn = true) {
  const safeFull = escapeHtml(fullPath || "");
  const shortText = escapeHtml(shortenPath(fullPath || ""));
  return `
    <div class="path-action-row">
      <div class="path-pill-wrap">
        <span class="path-pill btn-copy-path" data-path="${safeFull}" title="${safeFull}">${shortText}</span>
        <div class="path-tooltip">${safeFull}<div style="margin-top:4px;color:#93c5fd;font-size:10px;">${t("path.click_to_copy")}</div></div>
      </div>
      <button class="btn-icon-xs btn-copy-path" data-path="${safeFull}" title="${t("path.copy")}">📋</button>
      ${includeFinderBtn ? `<button class="btn-icon-xs btn-reveal-file" data-path="${safeFull}" title="${t("path.reveal_finder")}">📂</button>` : ""}
    </div>
  `;
}

function bindPathActions(container) {
  container.querySelectorAll(".btn-copy-path").forEach(btn => {
    btn.addEventListener("click", async (e) => {
      e.stopPropagation();
      const p = btn.dataset.path;
      if (!p) return;
      await api("copy_text", { text: p });
      showToast(t("path.copied_toast", { path: shortenPath(p) }));
    });
  });
  container.querySelectorAll(".btn-reveal-file").forEach(btn => {
    btn.addEventListener("click", async (e) => {
      e.stopPropagation();
      const p = btn.dataset.path;
      if (!p) return;
      await api("reveal_in_finder", { path: p });
    });
  });
}
