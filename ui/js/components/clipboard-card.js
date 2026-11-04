// =============================================================================
// REUSABLE COMPONENT: FULL CLIPBOARD CARD LIST (USED BY DRAWER & CLIPBOARD TAB)
// =============================================================================
function getSavedClipPageSize() {
  const val = Number(localStorage.getItem("omni_clip_page_size") || 50);
  return [50, 100, 200, 500, 1000].includes(val) ? val : 50;
}

function setSavedClipPageSize(val) {
  const n = Number(val);
  if ([50, 100, 200, 500, 1000].includes(n)) {
    localStorage.setItem("omni_clip_page_size", String(n));
  }
}

function renderFullClipboardCardsHtml(items, selectedIdx = -1, actionLabel = "", indexOffset = 0) {
  const btnText = actionLabel || t("clipboard.paste_copy");
  if (!items.length) {
    return `<div style="padding:28px;text-align:center;color:var(--text-muted);">${t("clipboard.empty")}</div>`;
  }
  return items.map((item, idx) => `
    <div class="clip-card ${idx === selectedIdx ? 'selected' : ''}" data-id="${item.id}" data-idx="${idx}">
      <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;">
        <div style="display:flex;align-items:center;gap:6px;">
          <span class="badge-ok">#${indexOffset + idx + 1}</span>
          <span style="font-size:11.5px;font-weight:600;color:var(--text-muted);">
            ${item.kind === 'image' ? t("clipboard.kind_image") : t("clipboard.kind_text")} • ${formatBytes(item.byte_size)}
          </span>
          ${item.is_pinned ? `<span class="badge-warn">${t("clipboard.pinned_badge")}</span>` : ''}
        </div>
        <div style="display:flex;gap:5px;">
          <button class="btn btn-sm btn-primary copy-btn" data-id="${item.id}">${btnText}</button>
          <button class="btn btn-sm pin-btn" data-id="${item.id}" title="${t("clipboard.tip_pin")}">${item.is_pinned ? '📌' : '📍'}</button>
          <button class="btn btn-sm btn-danger del-btn" data-id="${item.id}" title="${t("clipboard.tip_delete")}">✕</button>
        </div>
      </div>
      ${item.kind === "image" && item.image_data_url
      ? `<img class="clip-full-img" src="${item.image_data_url}" />`
      : `<div class="clip-full-text">${escapeHtml(item.full_text || item.preview_text)}</div>`
    }
    </div>
  `).join("");
}
