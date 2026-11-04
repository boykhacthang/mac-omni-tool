// =============================================================================
// OVERLAY MODE 4: MULTI-MONITOR SCREENSHOT & ANNOTATION OVERLAY
// =============================================================================
function initScreenshotOverlay() {
  const SLOT_INDEX = Number(params.get("slot") || 0);
  const root = document.getElementById("root");
  root.innerHTML = `
    <div class="ls-root" id="ls-root">
      <canvas class="ls-canvas" id="ls-canvas"></canvas>
      <div class="ls-top-hint" id="ls-top-hint">
        <span id="ls-monitor-badge" style="color:#60a5fa;font-weight:700;">${t("screenshot.display_badge", { num: SLOT_INDEX + 1 })}</span>
        <span id="ls-top-instruction">${t("screenshot.hint_region")}</span>
        <button class="ls-action-btn" id="ls-btn-select-full" style="height:24px;padding:0 9px;font-size:11px;">
          ${t("screenshot.select_full")}
        </button>
        <button class="ls-action-btn" id="ls-btn-copy-full-instant" style="height:24px;padding:0 9px;font-size:11px;">
          ${t("screenshot.copy_instant")}
        </button>
        <button class="ls-action-btn danger" id="ls-btn-cancel-top" style="height:24px;padding:0 9px;font-size:11px;">
          ${t("screenshot.exit_esc")}
        </button>
      </div>
      <div class="ls-size-pill" id="ls-size-pill">0 × 0</div>

      <!-- Vertical Annotation Toolbar (Rect, Pen, Line, Arrow, Circle, Text, Blur, Move, Colors, Stroke) -->
      <div class="ls-toolbar-vert" id="ls-toolbar-vert">
        <button class="ls-tool-btn active" data-tool="rect" title="${t('screenshot.tool_rect')}">▭</button>
        <button class="ls-tool-btn" data-tool="pen" title="${t('screenshot.tool_pen')}">✏️</button>
        <button class="ls-tool-btn" data-tool="line" title="${t('screenshot.tool_line')}">📏</button>
        <button class="ls-tool-btn" data-tool="arrow" title="${t('screenshot.tool_arrow')}">↗️</button>
        <button class="ls-tool-btn" data-tool="circle" title="${t('screenshot.tool_circle')}">◯</button>
        <button class="ls-tool-btn" data-tool="text" title="${t('screenshot.tool_text')}">🔤</button>
        <button class="ls-tool-btn" data-tool="blur" title="${t('screenshot.tool_blur')}">💧</button>
        <button class="ls-tool-btn" data-tool="move" title="${t('screenshot.tool_move')}">✥</button>

        <div style="width:20px;height:1px;background:rgba(255,255,255,0.16);margin:3px 0;"></div>

        <!-- Quick Color Picker (Default: Red #ef4444) -->
        <div class="ls-color-dot active" data-color="#ef4444" style="background:#ef4444;" title="${t('screenshot.color_red')}"></div>
        <div class="ls-color-dot" data-color="#f59e0b" style="background:#f59e0b;" title="${t('screenshot.color_amber')}"></div>
        <div class="ls-color-dot" data-color="#10b981" style="background:#10b981;" title="${t('screenshot.color_green')}"></div>
        <div class="ls-color-dot" data-color="#3b82f6" style="background:#3b82f6;" title="${t('screenshot.color_blue')}"></div>
        <div class="ls-color-dot" data-color="#ffffff" style="background:#ffffff;" title="${t('screenshot.color_white')}"></div>

        <div style="width:20px;height:1px;background:rgba(255,255,255,0.16);margin:3px 0;"></div>

        <!-- Stroke Thickness -->
        <button class="ls-stroke-dot" data-size="2" title="${t('screenshot.stroke_thin')}">
          <span style="width:5px;height:5px;border-radius:50%;background:#fff;display:block;"></span>
        </button>
        <button class="ls-stroke-dot active" data-size="4" title="${t('screenshot.stroke_medium')}">
          <span style="width:8px;height:8px;border-radius:50%;background:#fff;display:block;"></span>
        </button>
        <button class="ls-stroke-dot" data-size="7" title="${t('screenshot.stroke_thick')}">
          <span style="width:12px;height:12px;border-radius:50%;background:#fff;display:block;"></span>
        </button>
      </div>

      <!-- Horizontal Action Toolbar (Undo, Clear, Save, Copy, Close — uniform icon buttons, red Close button) -->
      <div class="ls-toolbar-horiz" id="ls-toolbar-horiz">
        <button class="ls-action-btn ls-icon-btn" id="ls-btn-undo" title="${t('screenshot.tip_undo')}" aria-label="${t('screenshot.tip_undo')}">
          <svg viewBox="0 0 24 24"><path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11"/></svg>
        </button>
        <button class="ls-action-btn ls-icon-btn" id="ls-btn-clear" title="${t('screenshot.tip_clear')}" aria-label="${t('screenshot.tip_clear')}">
          <svg viewBox="0 0 24 24"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>
        </button>
        <button class="ls-action-btn ls-icon-btn" id="ls-btn-save" title="${t('screenshot.tip_save')}" aria-label="${t('screenshot.tip_save')}">
          <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        </button>
        <button class="ls-action-btn ls-icon-btn" id="ls-btn-copy" title="${t('screenshot.tip_copy')}" aria-label="${t('screenshot.tip_copy')}">
          <svg viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        </button>
        <button class="ls-action-btn ls-icon-btn danger" id="ls-btn-close" title="${t('screenshot.tip_close')}" aria-label="${t('screenshot.tip_close')}">
          <svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>

      <!-- Inline Text Input for Text Tool -->
      <input type="text" class="ls-text-editor" id="ls-text-editor" placeholder="${t('screenshot.text_placeholder')}" />
    </div>
  `;

  function getWideGamut2dContext(canvasEl) {
    try {
      const c = canvasEl.getContext("2d", { colorSpace: "display-p3" });
      if (c) return c;
    } catch (_) { }
    return canvasEl.getContext("2d");
  }

  const canvas = document.getElementById("ls-canvas");
  const ctx = getWideGamut2dContext(canvas);
  const hintEl = document.getElementById("ls-top-hint");
  const monitorBadgeEl = document.getElementById("ls-monitor-badge");
  const btnSelectFull = document.getElementById("ls-btn-select-full");
  const sizePill = document.getElementById("ls-size-pill");
  const vertBar = document.getElementById("ls-toolbar-vert");
  const horizBar = document.getElementById("ls-toolbar-horiz");
  const textEditor = document.getElementById("ls-text-editor");
  const mosaicCanvas = document.createElement("canvas");
  const mosaicCtx = getWideGamut2dContext(mosaicCanvas);

  let bgImage = new Image();
  let bgLoaded = false;
  let screenW = window.innerWidth || 1440;
  let screenH = window.innerHeight || 900;
  let dpr = Math.max(window.devicePixelRatio || 2, 2);

  let sel = null; // { x, y, w, h }
  let annotations = [];
  let draftAnnotation = null;
  let isFullscreenPickerMode = false;

  let activeTool = "rect"; // Default tool is Red Rectangle (for quick screen highlighting)
  let strokeColor = "#ef4444"; // Default color is Red
  let strokeSize = 4;

  let interactionMode = null; // 'selecting' | 'moving' | 'resizing' | 'drawing'
  let resizeDir = null;
  let dragStart = { x: 0, y: 0 };
  let origSel = null;
  let textTargetPos = null;
  let rafDrawPending = false;
  let cachedVertW = 0;
  let cachedVertH = 0;
  let cachedHorizW = 0;
  let cachedHorizH = 0;

  function setupCanvasDimensions(w, h) {
    screenW = w || window.innerWidth || 1440;
    screenH = h || window.innerHeight || 900;
    let targetPhysW;
    let targetPhysH;
    if (bgLoaded && bgImage.naturalWidth > 0 && bgImage.naturalHeight > 0) {
      // Lock canvas backing store 1:1 to the physical pixel dimensions of the captured PNG
      targetPhysW = bgImage.naturalWidth;
      targetPhysH = bgImage.naturalHeight;
      dpr = bgImage.naturalWidth / screenW;
    } else {
      dpr = Math.max(window.devicePixelRatio || 2, 2);
      targetPhysW = Math.round(screenW * dpr);
      targetPhysH = Math.round(screenH * dpr);
    }
    // Only mutate canvas.width / canvas.height when dimensions actually change
    // so the GPU backing store is never unnecessarily destroyed or cleared!
    if (canvas.width !== targetPhysW) {
      canvas.width = targetPhysW;
    }
    if (canvas.height !== targetPhysH) {
      canvas.height = targetPhysH;
    }
    const cssW = screenW + "px";
    const cssH = screenH + "px";
    if (canvas.style.width !== cssW) canvas.style.width = cssW;
    if (canvas.style.height !== cssH) canvas.style.height = cssH;
  }

  function normalizeRect(x1, y1, x2, y2) {
    const x = Math.max(0, Math.min(x1, x2));
    const y = Math.max(0, Math.min(y1, y2));
    const right = Math.min(screenW, Math.max(x1, x2));
    const bottom = Math.min(screenH, Math.max(y1, y2));
    return { x, y, w: Math.max(0, right - x), h: Math.max(0, bottom - y) };
  }

  function getHandles(r) {
    const cx = r.x + r.w / 2;
    const cy = r.y + r.h / 2;
    return [
      { dir: "nw", x: r.x, y: r.y, cursor: "nwse-resize" },
      { dir: "n", x: cx, y: r.y, cursor: "ns-resize" },
      { dir: "ne", x: r.x + r.w, y: r.y, cursor: "nesw-resize" },
      { dir: "e", x: r.x + r.w, y: cy, cursor: "ew-resize" },
      { dir: "se", x: r.x + r.w, y: r.y + r.h, cursor: "nwse-resize" },
      { dir: "s", x: cx, y: r.y + r.h, cursor: "ns-resize" },
      { dir: "sw", x: r.x, y: r.y + r.h, cursor: "nesw-resize" },
      { dir: "w", x: r.x, y: cy, cursor: "ew-resize" }
    ];
  }

  function hitTestHandle(mx, my, r) {
    if (!r || r.w < 4 || r.h < 4) return null;
    for (const h of getHandles(r)) {
      if (Math.abs(mx - h.x) <= 7 && Math.abs(my - h.y) <= 7) {
        return h;
      }
    }
    return null;
  }

  function pointInRect(mx, my, r) {
    return r && mx >= r.x && mx <= r.x + r.w && my >= r.y && my <= r.y + r.h;
  }

  function drawSingleAnnotation(targetCtx, item, imgSource) {
    if (!item) return;
    targetCtx.save();
    targetCtx.strokeStyle = item.color || "#ef4444";
    targetCtx.fillStyle = item.color || "#ef4444";
    targetCtx.lineWidth = item.size || 4;
    targetCtx.lineCap = "round";
    targetCtx.lineJoin = "round";

    if (item.type === "pen") {
      const pts = item.points || [];
      if (pts.length === 1) {
        targetCtx.beginPath();
        targetCtx.arc(pts[0].x, pts[0].y, (item.size || 4) / 2, 0, Math.PI * 2);
        targetCtx.fill();
      } else if (pts.length > 1) {
        targetCtx.beginPath();
        targetCtx.moveTo(pts[0].x, pts[0].y);
        for (let i = 1; i < pts.length; i++) {
          targetCtx.lineTo(pts[i].x, pts[i].y);
        }
        targetCtx.stroke();
      }
    } else if (item.type === "line") {
      targetCtx.beginPath();
      targetCtx.moveTo(item.x1, item.y1);
      targetCtx.lineTo(item.x2, item.y2);
      targetCtx.stroke();
    } else if (item.type === "arrow") {
      const dx = item.x2 - item.x1;
      const dy = item.y2 - item.y1;
      const angle = Math.atan2(dy, dx);
      const headLen = Math.max(12, (item.size || 4) * 3.6);

      targetCtx.beginPath();
      targetCtx.moveTo(item.x1, item.y1);
      targetCtx.lineTo(item.x2, item.y2);
      targetCtx.stroke();

      targetCtx.beginPath();
      targetCtx.moveTo(item.x2, item.y2);
      targetCtx.lineTo(
        item.x2 - headLen * Math.cos(angle - Math.PI / 6),
        item.y2 - headLen * Math.sin(angle - Math.PI / 6)
      );
      targetCtx.lineTo(
        item.x2 - headLen * Math.cos(angle + Math.PI / 6),
        item.y2 - headLen * Math.sin(angle + Math.PI / 6)
      );
      targetCtx.closePath();
      targetCtx.fill();
    } else if (item.type === "rect") {
      const rx = Math.min(item.x1, item.x2);
      const ry = Math.min(item.y1, item.y2);
      const rw = Math.abs(item.x2 - item.x1);
      const rh = Math.abs(item.y2 - item.y1);
      targetCtx.strokeRect(rx, ry, rw, rh);
    } else if (item.type === "circle") {
      const cx = (item.x1 + item.x2) / 2;
      const cy = (item.y1 + item.y2) / 2;
      const rx = Math.abs(item.x2 - item.x1) / 2;
      const ry = Math.abs(item.y2 - item.y1) / 2;
      targetCtx.beginPath();
      targetCtx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
      targetCtx.stroke();
    } else if (item.type === "text" && item.text) {
      const fontPx = Math.max(15, (item.size || 4) * 4.5);
      targetCtx.font = `700 ${fontPx}px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif`;
      targetCtx.textBaseline = "top";
      targetCtx.shadowColor = "rgba(0,0,0,0.65)";
      targetCtx.shadowBlur = 3;
      targetCtx.fillText(item.text, item.x, item.y);
    } else if (item.type === "blur" && imgSource && bgLoaded) {
      const rx = Math.min(item.x1, item.x2);
      const ry = Math.min(item.y1, item.y2);
      const rw = Math.abs(item.x2 - item.x1);
      const rh = Math.abs(item.y2 - item.y1);
      if (rw > 4 && rh > 4) {
        const scaleX = imgSource.naturalWidth / screenW;
        const scaleY = imgSource.naturalHeight / screenH;
        const sampleW = Math.max(2, Math.round(rw / 10));
        const sampleH = Math.max(2, Math.round(rh / 10));
        mosaicCanvas.width = sampleW;
        mosaicCanvas.height = sampleH;
        mosaicCtx.imageSmoothingEnabled = true;
        mosaicCtx.drawImage(
          imgSource,
          rx * scaleX,
          ry * scaleY,
          rw * scaleX,
          rh * scaleY,
          0,
          0,
          sampleW,
          sampleH
        );
        targetCtx.imageSmoothingEnabled = false;
        targetCtx.drawImage(mosaicCanvas, 0, 0, sampleW, sampleH, rx, ry, rw, rh);
      }
    }
    targetCtx.restore();
  }

  function updateFloatingToolbars() {
    if (interactionMode === "drawing") {
      return;
    }

    if (
      !sel ||
      sel.w < 6 ||
      sel.h < 6 ||
      interactionMode === "selecting" ||
      interactionMode === "moving" ||
      interactionMode === "resizing"
    ) {
      vertBar.style.display = "none";
      horizBar.style.display = "none";
      if (!sel || sel.w < 2 || sel.h < 2) {
        sizePill.style.display = "none";
      } else {
        sizePill.style.display = "block";
        sizePill.textContent = `${Math.round(sel.w)} × ${Math.round(sel.h)} px`;
        const pillY = sel.y >= 28 ? sel.y - 26 : sel.y + 6;
        const pillX = Math.min(screenW - 110, Math.max(6, sel.x));
        sizePill.style.left = `${pillX}px`;
        sizePill.style.top = `${pillY}px`;
      }
      return;
    }

    sizePill.style.display = "block";
    sizePill.textContent = `${Math.round(sel.w)} × ${Math.round(sel.h)} px`;
    const pillY = sel.y >= 28 ? sel.y - 26 : sel.y + 6;
    const pillX = Math.min(screenW - 110, Math.max(6, sel.x));
    sizePill.style.left = `${pillX}px`;
    sizePill.style.top = `${pillY}px`;

    vertBar.style.display = "flex";
    if (!cachedVertW || !cachedVertH) {
      const vRect = vertBar.getBoundingClientRect();
      cachedVertW = vRect.width || 44;
      cachedVertH = vRect.height || 380;
    }
    const vW = cachedVertW;
    const vH = cachedVertH;
    let vx = sel.x + sel.w + 8;
    if (vx + vW > screenW - 6) {
      vx = sel.x - vW - 8;
    }
    if (vx < 6) {
      vx = Math.max(6, sel.x + sel.w - vW - 8);
    }
    let vy = Math.min(screenH - vH - 48, Math.max(8, sel.y + sel.h - vH));
    vertBar.style.left = `${vx}px`;
    vertBar.style.top = `${vy}px`;

    horizBar.style.display = "flex";
    if (!cachedHorizW || !cachedHorizH) {
      const hRect = horizBar.getBoundingClientRect();
      cachedHorizW = hRect.width || 196;
      cachedHorizH = hRect.height || 42;
    }
    const hW = cachedHorizW;
    const hH = cachedHorizH;
    let hx = Math.min(screenW - hW - 8, Math.max(8, sel.x + sel.w - hW));
    let hy = sel.y + sel.h + 8;
    if (hy + hH > screenH - 6) {
      hy = sel.y - hH - 8;
    }
    if (hy < 6) {
      hy = Math.max(6, sel.y + sel.h - hH - 8);
    }
    horizBar.style.left = `${hx}px`;
    horizBar.style.top = `${hy}px`;
  }

  function scheduleDrawScene() {
    if (rafDrawPending) return;
    rafDrawPending = requestAnimationFrame(() => {
      rafDrawPending = 0;
      drawScene();
    });
  }

  function drawScene() {
    if (rafDrawPending) {
      cancelAnimationFrame(rafDrawPending);
      rafDrawPending = 0;
    }

    if (
      bgLoaded &&
      bgImage.naturalWidth > 0 &&
      (canvas.width !== bgImage.naturalWidth || canvas.height !== bgImage.naturalHeight)
    ) {
      setupCanvasDimensions(screenW, screenH);
    }

    // 1. Draw the captured screen 1:1 in physical device pixels with zero interpolation/blur
    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    if (bgLoaded) {
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(bgImage, 0, 0, canvas.width, canvas.height);
    }
    ctx.restore();

    // 2. Draw selection mask & annotations in logical screen coordinates
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    if (sel && sel.w > 1 && sel.h > 1) {
      // Dim ONLY the area outside the selection box so the selected region stays 100% sharp & true-to-color
      ctx.save();
      ctx.fillStyle = "rgba(0, 0, 0, 0.22)";
      if (sel.y > 0) {
        ctx.fillRect(0, 0, screenW, sel.y);
      }
      const selBottom = sel.y + sel.h;
      if (selBottom < screenH) {
        ctx.fillRect(0, selBottom, screenW, screenH - selBottom);
      }
      if (sel.x > 0) {
        ctx.fillRect(0, sel.y, sel.x, sel.h);
      }
      const selRight = sel.x + sel.w;
      if (selRight < screenW) {
        ctx.fillRect(selRight, sel.y, screenW - selRight, sel.h);
      }
      ctx.restore();

      ctx.save();
      ctx.beginPath();
      ctx.rect(sel.x, sel.y, sel.w, sel.h);
      ctx.clip();

      for (const item of annotations) {
        drawSingleAnnotation(ctx, item, bgImage);
      }
      if (draftAnnotation) {
        drawSingleAnnotation(ctx, draftAnnotation, bgImage);
      }
      ctx.restore();

      ctx.save();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.5;
      ctx.strokeRect(sel.x, sel.y, sel.w, sel.h);

      ctx.strokeStyle = "#ef4444";
      ctx.lineWidth = 1.5;
      ctx.setLineDash([5, 4]);
      ctx.strokeRect(sel.x, sel.y, sel.w, sel.h);
      ctx.restore();

      ctx.save();
      ctx.fillStyle = "#ffffff";
      ctx.strokeStyle = "#ef4444";
      ctx.lineWidth = 1.5;
      for (const h of getHandles(sel)) {
        ctx.fillRect(h.x - 4, h.y - 4, 8, 8);
        ctx.strokeRect(h.x - 4, h.y - 4, 8, 8);
      }
      ctx.restore();
    } else if (bgLoaded) {
      // Keep a uniform subtle dim backdrop while waiting for region drag so clicking never flashes the screen
      ctx.save();
      ctx.fillStyle = "rgba(0, 0, 0, 0.22)";
      ctx.fillRect(0, 0, screenW, screenH);
      ctx.restore();
    }

    ctx.restore();
    updateFloatingToolbars();
  }

  function commitOpenTextEditor() {
    if (textEditor.style.display === "block") {
      const val = textEditor.value.trim();
      if (val && textTargetPos) {
        annotations.push({
          type: "text",
          color: strokeColor,
          size: strokeSize,
          x: textTargetPos.x,
          y: textTargetPos.y,
          text: val
        });
      }
      textEditor.value = "";
      textEditor.style.display = "none";
      textTargetPos = null;
      drawScene();
    }
  }

  function openTextEditorAt(x, y) {
    commitOpenTextEditor();
    textTargetPos = { x, y };
    const fontPx = Math.max(15, strokeSize * 4.5);
    textEditor.style.left = `${x}px`;
    textEditor.style.top = `${y}px`;
    textEditor.style.color = strokeColor;
    textEditor.style.borderColor = strokeColor;
    textEditor.style.fontSize = `${fontPx}px`;
    textEditor.style.display = "block";
    textEditor.value = "";
    setTimeout(() => textEditor.focus(), 10);
  }

  textEditor.addEventListener("keydown", (e) => {
    e.stopPropagation();
    if (e.key === "Enter") {
      e.preventDefault();
      commitOpenTextEditor();
    } else if (e.key === "Escape") {
      e.preventDefault();
      textEditor.value = "";
      textEditor.style.display = "none";
      textTargetPos = null;
    }
  });

  function isUnannotatedFullscreenSelection() {
    if (annotations.length > 0 || textEditor.style.display === "block") {
      return false;
    }
    if (!sel || sel.w < 4 || sel.h < 4) {
      return true;
    }
    return (
      sel.x <= 1 &&
      sel.y <= 1 &&
      Math.abs(sel.w - screenW) <= 2 &&
      Math.abs(sel.h - screenH) <= 2
    );
  }

  function exportCroppedDataUrl() {
    commitOpenTextEditor();
    if (!bgLoaded) return null;
    const region = (sel && sel.w >= 4 && sel.h >= 4)
      ? sel
      : { x: 0, y: 0, w: screenW, h: screenH };

    const scaleX = bgImage.naturalWidth / screenW;
    const scaleY = bgImage.naturalHeight / screenH;
    const sx = Math.max(0, Math.round(region.x * scaleX));
    const sy = Math.max(0, Math.round(region.y * scaleY));
    const outW = Math.max(1, Math.min(bgImage.naturalWidth - sx, Math.round(region.w * scaleX)));
    const outH = Math.max(1, Math.min(bgImage.naturalHeight - sy, Math.round(region.h * scaleY)));

    const outCanvas = document.createElement("canvas");
    outCanvas.width = outW;
    outCanvas.height = outH;
    const outCtx = getWideGamut2dContext(outCanvas);
    outCtx.imageSmoothingEnabled = false;

    outCtx.drawImage(
      bgImage,
      sx,
      sy,
      outW,
      outH,
      0,
      0,
      outW,
      outH
    );

    if (annotations.length > 0) {
      outCtx.save();
      outCtx.imageSmoothingEnabled = true;
      outCtx.scale(scaleX, scaleY);
      outCtx.translate(-sx / scaleX, -sy / scaleY);
      for (const item of annotations) {
        drawSingleAnnotation(outCtx, item, bgImage);
      }
      outCtx.restore();
    }

    return outCanvas.toDataURL("image/png");
  }

  async function copySelectionToClipboard() {
    commitOpenTextEditor();
    if (isUnannotatedFullscreenSelection()) {
      await api("copy_screenshot_png", { raw_slot: SLOT_INDEX });
      return;
    }
    const dataUrl = exportCroppedDataUrl();
    if (!dataUrl) return;
    await api("copy_screenshot_png", { data_url: dataUrl });
  }

  async function saveSelectionToFile() {
    commitOpenTextEditor();
    if (isUnannotatedFullscreenSelection()) {
      await api("save_screenshot_png", { raw_slot: SLOT_INDEX });
      return;
    }
    const dataUrl = exportCroppedDataUrl();
    if (!dataUrl) return;
    await api("save_screenshot_png", { data_url: dataUrl });
  }

  async function cancelScreenshot() {
    commitOpenTextEditor();
    await api("hide_overlay", { window: "screenshot" });
  }

  btnSelectFull.onclick = (e) => {
    e.stopPropagation();
    api("set_active_screenshot_slot", { slot: SLOT_INDEX });
    sel = { x: 0, y: 0, w: screenW, h: screenH };
    hintEl.style.display = "none";
    drawScene();
  };

  const btnCopyFullInstant = document.getElementById("ls-btn-copy-full-instant");
  if (btnCopyFullInstant) {
    btnCopyFullInstant.onclick = async (e) => {
      e.stopPropagation();
      api("set_active_screenshot_slot", { slot: SLOT_INDEX });
      sel = { x: 0, y: 0, w: screenW, h: screenH };
      await copySelectionToClipboard();
    };
  }

  const btnCancelTop = document.getElementById("ls-btn-cancel-top");
  if (btnCancelTop) {
    btnCancelTop.onclick = (e) => {
      e.stopPropagation();
      cancelScreenshot();
    };
  }

  window.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    cancelScreenshot();
  });

  // Mouse Events on Canvas
  canvas.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    api("set_active_screenshot_slot", { slot: SLOT_INDEX });
    commitOpenTextEditor();
    const mx = e.clientX;
    const my = e.clientY;

    if (isFullscreenPickerMode && (!sel || sel.w < 6 || sel.h < 6)) {
      sel = { x: 0, y: 0, w: screenW, h: screenH };
      annotations = [];
      draftAnnotation = null;
      interactionMode = null;
      hintEl.style.display = "none";
      drawScene();
      return;
    }

    const handle = hitTestHandle(mx, my, sel);
    if (handle) {
      interactionMode = "resizing";
      resizeDir = handle.dir;
      dragStart = { x: mx, y: my };
      origSel = { ...sel };
      return;
    }

    if (sel && pointInRect(mx, my, sel)) {
      if (activeTool === "move") {
        interactionMode = "moving";
        dragStart = { x: mx, y: my };
        origSel = { ...sel };
      } else if (activeTool === "text") {
        openTextEditorAt(mx, my);
      } else if (activeTool === "pen") {
        interactionMode = "drawing";
        draftAnnotation = {
          type: "pen",
          color: strokeColor,
          size: strokeSize,
          points: [{ x: mx, y: my }]
        };
        scheduleDrawScene();
      } else {
        interactionMode = "drawing";
        draftAnnotation = {
          type: activeTool,
          color: strokeColor,
          size: strokeSize,
          x1: mx,
          y1: my,
          x2: mx,
          y2: my
        };
        scheduleDrawScene();
      }
      return;
    }

    if (!sel || annotations.length === 0) {
      interactionMode = "selecting";
      dragStart = { x: mx, y: my };
      sel = { x: mx, y: my, w: 0, h: 0 };
      annotations = [];
      draftAnnotation = null;
      hintEl.style.display = "none";
      scheduleDrawScene();
    }
  });

  window.addEventListener("mousemove", (e) => {
    const mx = e.clientX;
    const my = e.clientY;

    if (!interactionMode) {
      const h = hitTestHandle(mx, my, sel);
      if (h) {
        canvas.style.cursor = h.cursor;
      } else if (sel && pointInRect(mx, my, sel)) {
        if (activeTool === "move") canvas.style.cursor = "move";
        else if (activeTool === "text") canvas.style.cursor = "text";
        else canvas.style.cursor = "crosshair";
      } else {
        canvas.style.cursor = "crosshair";
      }
      return;
    }

    if (interactionMode === "selecting") {
      sel = normalizeRect(dragStart.x, dragStart.y, mx, my);
      scheduleDrawScene();
    } else if (interactionMode === "moving" && origSel) {
      const dx = mx - dragStart.x;
      const dy = my - dragStart.y;
      const nx = Math.max(0, Math.min(screenW - origSel.w, origSel.x + dx));
      const ny = Math.max(0, Math.min(screenH - origSel.h, origSel.y + dy));
      sel = { x: nx, y: ny, w: origSel.w, h: origSel.h };
      scheduleDrawScene();
    } else if (interactionMode === "resizing" && origSel) {
      let x1 = origSel.x;
      let y1 = origSel.y;
      let x2 = origSel.x + origSel.w;
      let y2 = origSel.y + origSel.h;
      if (resizeDir.includes("w")) x1 = mx;
      if (resizeDir.includes("e")) x2 = mx;
      if (resizeDir.includes("n")) y1 = my;
      if (resizeDir.includes("s")) y2 = my;
      sel = normalizeRect(x1, y1, x2, y2);
      scheduleDrawScene();
    } else if (interactionMode === "drawing" && draftAnnotation) {
      if (draftAnnotation.type === "pen") {
        draftAnnotation.points.push({ x: mx, y: my });
      } else {
        draftAnnotation.x2 = mx;
        draftAnnotation.y2 = my;
      }
      scheduleDrawScene();
    }
  });

  window.addEventListener("mouseup", () => {
    if (!interactionMode) return;
    if (interactionMode === "selecting") {
      if (!sel || sel.w < 6 || sel.h < 6) {
        sel = null;
        hintEl.style.display = "inline-flex";
      }
    } else if (interactionMode === "drawing" && draftAnnotation) {
      if (draftAnnotation.type === "pen") {
        annotations.push(draftAnnotation);
      } else {
        const dist = Math.hypot(
          draftAnnotation.x2 - draftAnnotation.x1,
          draftAnnotation.y2 - draftAnnotation.y1
        );
        if (dist >= 3) {
          annotations.push(draftAnnotation);
        }
      }
      draftAnnotation = null;
    }
    interactionMode = null;
    resizeDir = null;
    drawScene();
  });

  vertBar.querySelectorAll(".ls-tool-btn").forEach(btn => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      commitOpenTextEditor();
      vertBar.querySelectorAll(".ls-tool-btn").forEach(b => b.classList.remove("active"));
      btn.classList.add("active");
      activeTool = btn.dataset.tool;
    });
  });

  vertBar.querySelectorAll(".ls-color-dot").forEach(dot => {
    dot.addEventListener("click", (e) => {
      e.stopPropagation();
      vertBar.querySelectorAll(".ls-color-dot").forEach(d => d.classList.remove("active"));
      dot.classList.add("active");
      strokeColor = dot.dataset.color;
    });
  });

  vertBar.querySelectorAll(".ls-stroke-dot").forEach(dot => {
    dot.addEventListener("click", (e) => {
      e.stopPropagation();
      vertBar.querySelectorAll(".ls-stroke-dot").forEach(d => d.classList.remove("active"));
      dot.classList.add("active");
      strokeSize = Number(dot.dataset.size) || 4;
    });
  });

  document.getElementById("ls-btn-undo").onclick = (e) => {
    e.stopPropagation();
    commitOpenTextEditor();
    annotations.pop();
    drawScene();
  };
  document.getElementById("ls-btn-clear").onclick = (e) => {
    e.stopPropagation();
    commitOpenTextEditor();
    annotations = [];
    drawScene();
  };
  document.getElementById("ls-btn-save").onclick = (e) => {
    e.stopPropagation();
    saveSelectionToFile();
  };
  document.getElementById("ls-btn-copy").onclick = (e) => {
    e.stopPropagation();
    copySelectionToClipboard();
  };
  document.getElementById("ls-btn-close").onclick = (e) => {
    e.stopPropagation();
    cancelScreenshot();
  };

  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      cancelScreenshot();
      return;
    }
    if (textEditor.style.display === "block") return;
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "c") {
      e.preventDefault();
      copySelectionToClipboard();
    } else if (e.key === "Enter" && sel && sel.w >= 6 && sel.h >= 6) {
      e.preventDefault();
      copySelectionToClipboard();
    } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      saveSelectionToFile();
    } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z") {
      e.preventDefault();
      annotations.pop();
      drawScene();
    }
  });

  window.__clearScreenshotSelection = () => {
    commitOpenTextEditor();
    sel = null;
    annotations = [];
    draftAnnotation = null;
    interactionMode = null;
    hintEl.style.display = "inline-flex";
    vertBar.style.display = "none";
    horizBar.style.display = "none";
    sizePill.style.display = "none";
    drawScene();
  };

  window.__prepareScreenshotHide = () => {
    if (rafDrawPending) {
      cancelAnimationFrame(rafDrawPending);
      rafDrawPending = 0;
    }
    textEditor.value = "";
    textEditor.style.display = "none";
    textTargetPos = null;
    bgLoaded = false;
    sel = null;
    annotations = [];
    draftAnnotation = null;
    interactionMode = null;
    vertBar.style.display = "none";
    horizBar.style.display = "none";
    sizePill.style.display = "none";
    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.restore();
  };

  function syncScreenshotLabels(targetSlot = SLOT_INDEX, labelName = "") {
    if (monitorBadgeEl) {
      monitorBadgeEl.textContent = labelName
        ? t("screenshot.display_badge_named", { num: targetSlot + 1, name: labelName })
        : t("screenshot.display_badge", { num: targetSlot + 1 });
    }
    const topInstr = document.getElementById("ls-top-instruction");
    if (topInstr) {
      topInstr.innerHTML = isFullscreenPickerMode
        ? t("screenshot.hint_picker")
        : t("screenshot.hint_region");
    }
    if (btnSelectFull) btnSelectFull.textContent = t("screenshot.select_full");
    const btnCopyInstant = document.getElementById("ls-btn-copy-full-instant");
    if (btnCopyInstant) btnCopyInstant.textContent = t("screenshot.copy_instant");
    const btnCancel = document.getElementById("ls-btn-cancel-top");
    if (btnCancel) btnCancel.textContent = t("screenshot.exit_esc");

    const undoBtn = document.getElementById("ls-btn-undo");
    if (undoBtn) {
      undoBtn.title = t("screenshot.tip_undo");
      undoBtn.setAttribute("aria-label", t("screenshot.tip_undo"));
    }
    const clearBtn = document.getElementById("ls-btn-clear");
    if (clearBtn) {
      clearBtn.title = t("screenshot.tip_clear");
      clearBtn.setAttribute("aria-label", t("screenshot.tip_clear"));
    }
    const saveBtn = document.getElementById("ls-btn-save");
    if (saveBtn) {
      saveBtn.title = t("screenshot.tip_save");
      saveBtn.setAttribute("aria-label", t("screenshot.tip_save"));
    }
    const copyBtn = document.getElementById("ls-btn-copy");
    if (copyBtn) {
      copyBtn.title = t("screenshot.tip_copy");
      copyBtn.setAttribute("aria-label", t("screenshot.tip_copy"));
    }
    const closeBtn = document.getElementById("ls-btn-close");
    if (closeBtn) {
      closeBtn.title = t("screenshot.tip_close");
      closeBtn.setAttribute("aria-label", t("screenshot.tip_close"));
    }
    if (textEditor) textEditor.placeholder = t("screenshot.text_placeholder");
  }

  window.__onLangChanged = () => syncScreenshotLabels(SLOT_INDEX, "");

  window.__startScreenshotSession = async (
    w,
    h,
    seq,
    slotIdx = SLOT_INDEX,
    preselectFullscreen = false,
    dispName = "",
    fullscreenPickerMode = false
  ) => {
    applyAppLang(getAppLang(), false);
    commitOpenTextEditor();
    sel = null;
    annotations = [];
    draftAnnotation = null;
    interactionMode = null;
    isFullscreenPickerMode = !!fullscreenPickerMode;
    activeTool = "rect";
    strokeColor = "#ef4444";
    strokeSize = 4;

    vertBar.querySelectorAll(".ls-tool-btn").forEach(b => {
      b.classList.toggle("active", b.dataset.tool === "rect");
    });
    vertBar.querySelectorAll(".ls-color-dot").forEach(d => {
      d.classList.toggle("active", d.dataset.color === "#ef4444");
    });
    vertBar.querySelectorAll(".ls-stroke-dot").forEach(d => {
      d.classList.toggle("active", d.dataset.size === "4");
    });

    let targetW = w;
    let targetH = h;
    let targetSeq = seq;
    let targetSlot = (slotIdx !== undefined && slotIdx !== null) ? slotIdx : SLOT_INDEX;
    let isFull = !!preselectFullscreen;
    let labelName = dispName;

    if (!targetW || !targetH) {
      const meta = await api("get_screenshot_meta", { slot: targetSlot });
      targetW = meta.width || window.innerWidth;
      targetH = meta.height || window.innerHeight;
      targetSeq = meta.seq || Date.now();
      if (meta.preselect_fullscreen) isFull = true;
      if (meta.display_name) labelName = meta.display_name;
    }

    syncScreenshotLabels(targetSlot, labelName);

    if (isFull) {
      sel = { x: 0, y: 0, w: targetW, h: targetH };
      hintEl.style.display = "inline-flex";
    } else {
      hintEl.style.display = "inline-flex";
      vertBar.style.display = "none";
      horizBar.style.display = "none";
      sizePill.style.display = "none";
    }

    const imgUrl = `omni://localhost/screenshot_${targetSlot}.png?seq=${targetSeq}&t=${Date.now()}`;
    const nextImg = new Image();
    nextImg.decoding = "sync";

    const finalizeAndReveal = () => {
      bgImage = nextImg;
      bgLoaded = true;
      setupCanvasDimensions(targetW, targetH);
      drawScene();

      let revealed = false;
      const notifyReady = () => {
        if (revealed) return;
        revealed = true;
        api("reveal_screenshot_slot", { slot: targetSlot, seq: targetSeq });
      };
      requestAnimationFrame(() => {
        notifyReady();
      });
      setTimeout(notifyReady, 18);
    };

    nextImg.onload = () => {
      if (typeof nextImg.decode === "function") {
        nextImg.decode().then(finalizeAndReveal).catch(finalizeAndReveal);
      } else {
        finalizeAndReveal();
      }
    };
    nextImg.onerror = () => {
      cancelScreenshot();
    };
    nextImg.src = imgUrl;
  };

  setupCanvasDimensions(window.innerWidth, window.innerHeight);
}

