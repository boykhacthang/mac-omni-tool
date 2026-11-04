// =============================================================================
// SCREEN 2: HARDWARE MONITOR & REAL-TIME PROCESS MANAGER
// - Sub-Tab 1 ("overview"): 6 Live Hardware Cards (CPU/Cores, GPU/VRAM, RAM, Sensors, SSD, Net) + Top 5 preview
// - Sub-Tab 2 ("processes"): Dedicated Scrollable Process Table with Sticky Header & Zero-Leak DOM Recycling
// =============================================================================
function createHardwareMonitorScreen({ isTabActive }) {
  let hwSubTab = "overview"; // "overview" | "processes"
  let monitorSort = "ram";
  let monitorLimit = 25;
  let monitorAutoRefresh = true;
  let monitorIntervalMs = 2000;
  let monitorSearchQuery = "";
  let monitorPollTimer = null;
  let monitorIsFetching = false;

  function stopPolling() {
    if (monitorPollTimer) {
      clearInterval(monitorPollTimer);
      monitorPollTimer = null;
    }
  }

  function startPolling(view) {
    stopPolling();
    if (!monitorAutoRefresh || !isTabActive()) return;
    monitorPollTimer = setInterval(() => {
      if (!isTabActive() || document.hidden) return;
      refreshMonitorDataInPlace(view);
    }, monitorIntervalMs);
  }

  function renderCompactHwStripHtml(data) {
    const temps = data.temperatures || {};
    const gpu = data.gpu || {};
    const cpuPct = Number(data.cpu_usage_percent || 0);
    const gpuPct = Number(gpu.usage_percent || 0);
    const memPct = Number(data.memory_usage_percent || 0);
    return `
      <div class="stat-strip" style="margin-bottom:10px;">
        <div class="stat-pill">
          <span>⚡ CPU:</span>
          <b style="color:var(--accent);">${formatPct(cpuPct)}</b>
          <span>(${formatTemp(temps.cpu_temp_c)})</span>
        </div>
        <div class="stat-pill">
          <span>🎮 GPU:</span>
          <b>${formatPct(gpuPct)}</b>
          <span>(${formatBytes(gpu.in_use_memory_bytes || 0)} VRAM)</span>
        </div>
        <div class="stat-pill">
          <span>🧠 RAM:</span>
          <b style="color:${memPct > 85 ? 'var(--warning)' : 'var(--success)'};">${formatPct(memPct)}</b>
          <span>(${formatBytes(data.used_memory_bytes)} / ${formatBytes(data.total_memory_bytes)})</span>
        </div>
        <div class="stat-pill">
          <span>🌐 Net:</span>
          <b style="color:var(--success);">⬇️ ${formatBytes(data.net_rx_bytes_per_sec)}/s</b>
          <b style="color:var(--accent);">⬆️ ${formatBytes(data.net_tx_bytes_per_sec)}/s</b>
        </div>
      </div>
    `;
  }

  function renderTop5PreviewHtml(procs) {
    const top5 = (procs || []).slice(0, 5);
    return `
      <div class="card" style="margin-bottom:0;padding:11px 14px;">
        <div class="card-header" style="margin-bottom:8px;flex-wrap:wrap;gap:8px;">
          <div class="card-title" style="font-size:13px;">
            <span>🔥 ${t("hardware.top5_title")}</span>
          </div>
          <button class="btn btn-sm btn-primary" id="btn-jump-proc-tab">
            ⚙️ ${t("hardware.btn_open_proc_table")}
          </button>
        </div>
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:8px;">
          ${top5.map((p, idx) => `
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:8px;padding:7px 10px;display:flex;flex-direction:column;gap:3px;min-width:0;">
              <div style="display:flex;align-items:center;justify-content:space-between;gap:4px;">
                <span style="font-weight:700;font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title="${escapeHtml(p.name)}">
                  #${idx + 1} ${escapeHtml(p.name)}
                </span>
                <span style="font-size:10px;color:var(--text-muted);font-family:monospace;">PID ${p.pid}</span>
              </div>
              <div style="display:flex;justify-content:space-between;font-size:11px;">
                <span style="color:var(--accent);font-weight:600;">${p.memory_mb >= 1024 ? (p.memory_mb / 1024).toFixed(2) + " GB" : p.memory_mb.toFixed(0) + " MB"}</span>
                <span style="color:var(--text-muted);">CPU <b style="color:var(--text-main);">${formatPct(p.cpu_percent)}</b></span>
              </div>
            </div>
          `).join("")}
        </div>
      </div>
    `;
  }

  function renderHardwareCardsHtml(data) {
    const temps = data.temperatures || {};
    const gpu = data.gpu || {};
    const batt = data.battery || {};
    const gpuPct = Number(gpu.usage_percent || 0);
    const cpuPct = Number(data.cpu_usage_percent || 0);
    const memPct = Number(data.memory_usage_percent || 0);
    const diskPct = Number(data.disk_usage_percent || 0);
    const cores = Array.isArray(data.per_core_usage) ? data.per_core_usage : [];

    const vramUsed = Number(gpu.in_use_memory_bytes || 0);
    const vramAlloc = Number(gpu.alloc_memory_bytes || 0);
    const vramMax = Math.max(1, Number(gpu.max_recommended_memory_bytes || data.total_memory_bytes || 1));
    const vramUsedPct = Math.min(100, (vramUsed / vramMax) * 100);
    const vramAllocPct = Math.min(100, (vramAlloc / vramMax) * 100);

    const cpuTempColor = temps.cpu_temp_c > 80 ? "var(--danger)" : temps.cpu_temp_c > 65 ? "var(--warning)" : "var(--success)";
    const gpuTempVal = gpu.temp_c || temps.gpu_temp_c || 0;
    const gpuTempColor = gpuTempVal > 80 ? "var(--danger)" : gpuTempVal > 65 ? "var(--warning)" : "var(--success)";

    return `
      <!-- ROW 1: 3 PRIMARY CARDS (CPU • GPU • RAM) -->
      <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:10px;margin-bottom:10px;">
        <!-- CPU CARD -->
        <div class="card" style="margin-bottom:0;padding:12px 14px;display:flex;flex-direction:column;justify-content:space-between;">
          <div>
            <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:5px;">
              <div class="card-title" style="display:flex;align-items:center;gap:6px;">
                <span>${t("hardware.cpu_card_title")}</span>
              </div>
              <span class="badge-ok" style="font-size:11px;color:${cpuTempColor};border-color:${cpuTempColor}44;">
                🌡️ ${formatTemp(temps.cpu_temp_c)}
              </span>
            </div>

            <div style="display:flex;align-items:baseline;justify-content:space-between;gap:8px;">
              <div class="stat-value" style="font-size:22px;">${formatPct(cpuPct)}</div>
              <span style="font-size:11px;color:var(--text-muted);font-weight:600;">
                ${escapeHtml(data.cpu_brand)} • ${data.cpu_cores} ${t("hardware.cores_unit")}
              </span>
            </div>

            <div class="progress-track" style="height:6px;margin:6px 0 8px;">
              <div class="progress-fill ${cpuPct > 80 ? 'danger' : cpuPct > 60 ? 'warn' : ''}" style="width:${Math.min(100, cpuPct)}%;transition:width 0.35s ease;"></div>
            </div>
          </div>

          <!-- Per-Core Mini Bars -->
          <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:8px;padding:6px 8px;">
            <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text-muted);margin-bottom:4px;">
              <span>${t("hardware.per_core_load", { count: cores.length || data.cpu_cores })}</span>
              <span>${t("hardware.peak_label")} <b style="color:var(--text-main);">${cores.length ? formatPct(Math.max(...cores)) : '0%'}</b></span>
            </div>
            <div style="display:grid;grid-template-columns:repeat(${Math.max(1, cores.length || 12)},1fr);gap:3px;height:24px;align-items:end;">
              ${(cores.length ? cores : Array(12).fill(0)).map((c, i) => {
                const h = Math.max(8, Math.min(100, Math.round(c)));
                const barClr = c > 80 ? 'var(--danger)' : c > 50 ? 'var(--warning)' : 'var(--accent)';
                return `
                  <div title="${t("hardware.core_tip", { num: i + 1, pct: c.toFixed(1) })}" style="height:100%;background:rgba(128,128,128,0.14);border-radius:3px;display:flex;align-items:flex-end;overflow:hidden;">
                    <div style="width:100%;height:${h}%;background:${barClr};border-radius:3px;transition:height 0.35s ease;"></div>
                  </div>
                `;
              }).join("")}
            </div>
          </div>
        </div>

        <!-- GPU CARD -->
        <div class="card" style="margin-bottom:0;padding:12px 14px;display:flex;flex-direction:column;justify-content:space-between;">
          <div>
            <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:5px;">
              <div class="card-title" style="display:inline-flex;align-items:center;gap:5px;">
                <span>${t("hardware.gpu_card_title")}</span>
                ${renderTip(t("hardware.gpu_card_tip"))}
              </div>
              <span class="badge-ok" style="font-size:11px;color:${gpuTempColor};border-color:${gpuTempColor}44;">
                🌡️ ${formatTemp(gpuTempVal)}
              </span>
            </div>

            <div style="display:flex;align-items:baseline;justify-content:space-between;gap:8px;">
              <div class="stat-value" style="font-size:22px;">${formatPct(gpuPct)}</div>
              <span style="font-size:11px;color:var(--text-muted);font-weight:600;">
                ${gpu.cores || 16} ${t("hardware.cores_unit")} • ${escapeHtml(gpu.metal_support || 'Metal')}
              </span>
            </div>

            <div class="progress-track" style="height:6px;margin:6px 0 8px;">
              <div class="progress-fill ${gpuPct > 85 ? 'danger' : gpuPct > 60 ? 'warn' : ''}" style="width:${Math.min(100, gpuPct)}%;transition:width 0.35s ease;"></div>
            </div>
          </div>

          <!-- Render / Tiler & Unified VRAM -->
          <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:8px;padding:6px 8px;font-size:11px;">
            <div style="display:flex;justify-content:space-between;margin-bottom:4px;">
              <span style="color:var(--text-muted);">Render: <b style="color:var(--text-main);">${formatPct(gpu.renderer_percent || 0)}</b></span>
              <span style="color:var(--text-muted);">Tiler: <b style="color:var(--text-main);">${formatPct(gpu.tiler_percent || 0)}</b></span>
              <span style="color:var(--accent);font-weight:600;">VRAM: ${formatBytes(vramUsed)}</span>
            </div>
            <div style="height:5px;background:rgba(128,128,128,0.16);border-radius:999px;overflow:hidden;position:relative;margin-bottom:4px;">
              <div style="position:absolute;left:0;top:0;bottom:0;width:${vramAllocPct}%;background:rgba(59,130,246,0.32);"></div>
              <div style="position:absolute;left:0;top:0;bottom:0;width:${vramUsedPct}%;background:var(--accent);"></div>
            </div>
            <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text-muted);">
              <span>${t("hardware.gpu_allocated")} <b style="color:var(--text-main);">${formatBytes(vramAlloc)}</b></span>
              <span>${t("hardware.gpu_max_ram")} <b style="color:var(--text-main);">${formatBytes(vramMax)}</b></span>
            </div>
          </div>
        </div>

        <!-- RAM CARD -->
        <div class="card" style="margin-bottom:0;padding:12px 14px;display:flex;flex-direction:column;justify-content:space-between;">
          <div>
            <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:5px;">
              <div class="card-title">${t("hardware.ram_card_title")}</div>
              <span class="${memPct > 85 ? 'badge-warn' : 'badge-ok'}" style="font-size:11px;">
                ${memPct > 85 ? t("hardware.ram_high_pressure") : t("hardware.ram_healthy")}
              </span>
            </div>

            <div style="display:flex;align-items:baseline;justify-content:space-between;gap:8px;">
              <div class="stat-value" style="font-size:22px;">${formatPct(memPct)}</div>
              <span style="font-size:11px;color:var(--text-muted);font-weight:600;">
                ${formatBytes(data.used_memory_bytes)} / ${formatBytes(data.total_memory_bytes)}
              </span>
            </div>

            <div class="progress-track" style="height:6px;margin:6px 0 8px;">
              <div class="progress-fill ${memPct > 88 ? 'danger' : memPct > 75 ? 'warn' : ''}" style="width:${Math.min(100, memPct)}%;transition:width 0.35s ease;"></div>
            </div>
          </div>

          <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:6px;background:var(--bg-input);border:1px solid var(--border);border-radius:8px;padding:6px 8px;text-align:center;">
            <div>
              <div style="font-size:10px;color:var(--text-muted);">${t("hardware.ram_used")}</div>
              <div style="font-size:11.5px;font-weight:700;color:var(--accent);">${formatBytes(data.used_memory_bytes)}</div>
            </div>
            <div style="border-left:1px solid var(--border);border-right:1px solid var(--border);">
              <div style="font-size:10px;color:var(--text-muted);">${t("hardware.ram_available")}</div>
              <div style="font-size:11.5px;font-weight:700;color:var(--success);">${formatBytes(data.available_memory_bytes)}</div>
            </div>
            <div>
              <div style="font-size:10px;color:var(--text-muted);">${t("hardware.ram_total")}</div>
              <div style="font-size:11.5px;font-weight:700;">${formatBytes(data.total_memory_bytes)}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- ROW 2: SENSORS & BATTERY • SSD STORAGE • LIVE NETWORK I/O -->
      <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:10px;margin-bottom:10px;">
        <!-- TEMPERATURES & POWER -->
        <div class="card" style="margin-bottom:0;padding:10px 13px;">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:7px;">
            <div class="card-title" style="font-size:13px;">${t("hardware.sensors_card_title")}</div>
            <span style="font-size:11px;font-weight:600;color:var(--text-muted);">
              ${batt.is_charging ? t("hardware.battery_charging") : "🔌 " + escapeHtml(batt.power_source || "AC Power")}
              ${batt.has_battery ? ` (${Math.round(batt.percentage || 100)}%)` : ''}
            </span>
          </div>
          <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:6px;text-align:center;">
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:7px;padding:5px 4px;">
              <div style="font-size:10px;color:var(--text-muted);">CPU SoC</div>
              <div style="font-size:13px;font-weight:700;color:${cpuTempColor};">${formatTemp(temps.cpu_temp_c)}</div>
            </div>
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:7px;padding:5px 4px;">
              <div style="font-size:10px;color:var(--text-muted);">GPU</div>
              <div style="font-size:13px;font-weight:700;color:${gpuTempColor};">${formatTemp(gpuTempVal)}</div>
            </div>
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:7px;padding:5px 4px;">
              <div style="font-size:10px;color:var(--text-muted);">${t("hardware.battery_pmu")}</div>
              <div style="font-size:13px;font-weight:700;color:var(--text-main);">${formatTemp(temps.battery_temp_c)}</div>
            </div>
          </div>
        </div>

        <!-- SSD STORAGE -->
        <div class="card" style="margin-bottom:0;padding:10px 13px;">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:5px;">
            <div class="card-title" style="font-size:13px;">${t("hardware.ssd_card_title")}</div>
            <span style="font-size:12.5px;font-weight:700;color:var(--accent);">${formatPct(diskPct)}</span>
          </div>
          <div class="progress-track" style="height:6px;margin:4px 0 8px;">
            <div class="progress-fill ${diskPct > 90 ? 'danger' : diskPct > 75 ? 'warn' : ''}" style="width:${Math.min(100, diskPct)}%;"></div>
          </div>
          <div style="display:flex;justify-content:space-between;font-size:11px;color:var(--text-muted);">
            <span>${t("hardware.ssd_used")} <b style="color:var(--text-main);">${formatBytes(data.disk_used_bytes)}</b></span>
            <span>${t("hardware.ssd_free")} <b style="color:var(--success);">${formatBytes(data.disk_available_bytes)}</b></span>
            <span>${t("hardware.ssd_total")} <b>${formatBytes(data.disk_total_bytes)}</b></span>
          </div>
        </div>

        <!-- NETWORK I/O -->
        <div class="card" style="margin-bottom:0;padding:10px 13px;">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px;">
            <div class="card-title" style="font-size:13px;">${t("hardware.net_card_title")}</div>
            <span style="font-size:10.5px;color:var(--text-muted);">⏱️ Uptime: <b>${formatUptime(data.uptime_seconds)}</b></span>
          </div>
          <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:7px;padding:5px 8px;display:flex;align-items:center;justify-content:space-between;">
              <div>
                <div style="font-size:10px;color:var(--text-muted);">${t("hardware.net_download")}</div>
                <div style="font-size:12.5px;font-weight:700;color:var(--success);">${formatBytes(data.net_rx_bytes_per_sec)}/s</div>
              </div>
              <div style="font-size:9.5px;color:var(--text-muted);text-align:right;">
                ${t("hardware.net_total_rx")}<br><b>${formatBytes(data.net_total_rx_bytes)}</b>
              </div>
            </div>
            <div style="background:var(--bg-input);border:1px solid var(--border);border-radius:7px;padding:5px 8px;display:flex;align-items:center;justify-content:space-between;">
              <div>
                <div style="font-size:10px;color:var(--text-muted);">${t("hardware.net_upload")}</div>
                <div style="font-size:12.5px;font-weight:700;color:var(--accent);">${formatBytes(data.net_tx_bytes_per_sec)}/s</div>
              </div>
              <div style="font-size:9.5px;color:var(--text-muted);text-align:right;">
                ${t("hardware.net_total_tx")}<br><b>${formatBytes(data.net_total_tx_bytes)}</b>
              </div>
            </div>
          </div>
        </div>
      </div>

      ${renderTop5PreviewHtml(data.top_processes || [])}
    `;
  }

  function filterProcessList(procs) {
    const q = (monitorSearchQuery || "").trim().toLowerCase();
    return q
      ? procs.filter(p =>
          (p.name || "").toLowerCase().includes(q) ||
          String(p.pid).includes(q) ||
          (p.exe_path || "").toLowerCase().includes(q)
        )
      : procs.slice(0, monitorLimit);
  }

  function createProcessRowElement() {
    const trEl = document.createElement("tr");
    trEl.className = "proc-row";
    trEl.innerHTML = `
      <td class="col-idx" style="color:var(--text-muted);"></td>
      <td class="col-pid" style="font-family:monospace;color:var(--text-muted);"></td>
      <td class="col-name">
        <div class="proc-name-main" style="font-weight:600;color:var(--text-main);"></div>
        <div class="proc-name-sub" style="font-size:10px;color:var(--text-muted);"></div>
      </td>
      <td class="col-ram">
        <div style="display:flex;justify-content:space-between;font-size:11.5px;margin-bottom:3px;gap:4px;">
          <b class="proc-ram-val" style="color:var(--accent);"></b>
          <span class="proc-ram-pct" style="color:var(--text-muted);font-size:10.5px;"></span>
        </div>
        <div style="height:4px;background:rgba(128,128,128,0.15);border-radius:999px;overflow:hidden;">
          <div class="proc-ram-bar" style="height:100%;background:var(--accent);border-radius:999px;transition:width 0.3s ease;"></div>
        </div>
      </td>
      <td class="col-cpu">
        <div style="display:flex;justify-content:space-between;font-size:11.5px;margin-bottom:3px;">
          <b class="proc-cpu-val"></b>
        </div>
        <div style="height:4px;background:rgba(128,128,128,0.15);border-radius:999px;overflow:hidden;">
          <div class="proc-cpu-bar" style="height:100%;border-radius:999px;transition:width 0.3s ease;"></div>
        </div>
      </td>
      <td class="col-io" style="font-size:11px;color:var(--text-muted);"></td>
      <td class="col-act">
        <button class="btn btn-sm btn-danger btn-force-quit">${t("hardware.btn_force_quit_sm")}</button>
      </td>
    `;
    return trEl;
  }

  function updateProcessRowElement(trEl, p, idx) {
    const ramBarW = Math.min(100, Math.max(3, p.memory_percent * 2.5));
    const cpuBarW = Math.min(100, Math.max(2, p.cpu_percent));
    const cpuColor = p.cpu_percent > 50 ? "var(--danger)" : p.cpu_percent > 15 ? "var(--warning)" : "var(--success)";
    const ioTotal = (p.disk_read_bytes || 0) + (p.disk_written_bytes || 0);

    trEl.children[0].textContent = String(idx + 1);
    trEl.children[1].textContent = String(p.pid);

    const nameCell = trEl.children[2];
    nameCell.title = p.exe_path || p.name || "";
    nameCell.querySelector(".proc-name-main").textContent = p.name || "";
    const subEl = nameCell.querySelector(".proc-name-sub");
    if (p.exe_path) {
      subEl.textContent = p.exe_path;
      subEl.style.display = "block";
    } else {
      subEl.textContent = "";
      subEl.style.display = "none";
    }

    const ramCell = trEl.children[3];
    ramCell.querySelector(".proc-ram-val").textContent =
      p.memory_mb >= 1024 ? (p.memory_mb / 1024).toFixed(2) + " GB" : p.memory_mb.toFixed(1) + " MB";
    ramCell.querySelector(".proc-ram-pct").textContent = formatPct(p.memory_percent);
    ramCell.querySelector(".proc-ram-bar").style.width = `${ramBarW}%`;

    const cpuCell = trEl.children[4];
    const cpuValEl = cpuCell.querySelector(".proc-cpu-val");
    cpuValEl.textContent = formatPct(p.cpu_percent);
    cpuValEl.style.color = p.cpu_percent > 15 ? cpuColor : "var(--text-main)";
    const cpuBarEl = cpuCell.querySelector(".proc-cpu-bar");
    cpuBarEl.style.width = `${cpuBarW}%`;
    cpuBarEl.style.background = cpuColor;

    const ioCell = trEl.children[5];
    ioCell.textContent = ioTotal > 0
      ? `📖 ${formatBytes(p.disk_read_bytes || 0)} • ✍️ ${formatBytes(p.disk_written_bytes || 0)}`
      : "0 B";
    ioCell.style.opacity = ioTotal > 0 ? "1" : "0.5";

    const btn = trEl.children[6].querySelector(".btn-force-quit");
    btn.dataset.pid = String(p.pid);
    btn.dataset.name = p.name || "";
  }

  function syncProcessRowsDom(tbodyEl, procs) {
    if (!tbodyEl) return;
    const filtered = filterProcessList(procs);

    if (filtered.length === 0) {
      while (tbodyEl.firstChild) {
        tbodyEl.removeChild(tbodyEl.firstChild);
      }
      const emptyTr = document.createElement("tr");
      emptyTr.className = "proc-empty-row";
      const td = document.createElement("td");
      td.colSpan = 7;
      td.style.cssText = "text-align:center;padding:24px;color:var(--text-muted);";
      td.textContent = t("hardware.proc_empty_search", { query: monitorSearchQuery });
      emptyTr.appendChild(td);
      tbodyEl.appendChild(emptyTr);
      return;
    }

    const emptyRow = tbodyEl.querySelector(".proc-empty-row");
    if (emptyRow) {
      tbodyEl.removeChild(emptyRow);
    }

    while (tbodyEl.children.length > filtered.length) {
      tbodyEl.removeChild(tbodyEl.lastElementChild);
    }

    for (let i = 0; i < filtered.length; i++) {
      let trEl = tbodyEl.children[i];
      if (!trEl || !trEl.classList.contains("proc-row")) {
        const newTr = createProcessRowElement();
        if (trEl) {
          tbodyEl.replaceChild(newTr, trEl);
        } else {
          tbodyEl.appendChild(newTr);
        }
        trEl = newTr;
      }
      updateProcessRowElement(trEl, filtered[i], i);
    }
  }

  function bindOverviewJumpBtn(view) {
    const jumpBtn = document.getElementById("btn-jump-proc-tab");
    if (jumpBtn) {
      jumpBtn.onclick = () => {
        hwSubTab = "processes";
        render(view);
      };
    }
  }

  async function refreshMonitorDataInPlace(view) {
    if (monitorIsFetching) return;
    monitorIsFetching = true;
    try {
      const fetchLimit = (monitorSearchQuery || "").trim() ? 100 : monitorLimit;
      const data = await api("get_hardware_dashboard", { sort_by: monitorSort, limit: fetchLimit });
      if (!isTabActive()) return;

      const cardsEl = document.getElementById("hw-cards-container");
      if (cardsEl) {
        cardsEl.innerHTML = renderHardwareCardsHtml(data);
        bindOverviewJumpBtn(view);
      }

      const stripEl = document.getElementById("hw-compact-strip");
      if (stripEl) {
        stripEl.innerHTML = renderCompactHwStripHtml(data);
      }

      const tbodyEl = document.getElementById("proc-table-body");
      if (tbodyEl) {
        syncProcessRowsDom(tbodyEl, data.top_processes || []);
      }

      const countEl = document.getElementById("proc-count-badge");
      if (countEl) {
        const totalShown = (data.top_processes || []).length;
        const shownCount = Math.min(monitorLimit, totalShown);
        countEl.textContent = t("hardware.showing_procs", { count: shownCount });
      }
    } finally {
      monitorIsFetching = false;
    }
  }

  async function render(view) {
    const fetchLimit = (monitorSearchQuery || "").trim() ? 100 : monitorLimit;
    const data = await api("get_hardware_dashboard", { sort_by: monitorSort, limit: fetchLimit });
    const gpu = data.gpu || {};
    const batt = data.battery || {};
    const procs = data.top_processes || [];
    const shownCount = Math.min(monitorLimit, procs.length);

    view.innerHTML = `
      <div class="page-header">
        <div class="page-title-row">
          <div class="page-title">${t("hardware.page_title")}</div>
          ${renderTip(t("hardware.page_tip", {
            brand: data.cpu_brand,
            cpuCores: data.cpu_cores,
            gpuCores: gpu.cores || 16,
            os: data.os_version,
            power: batt.power_source || 'AC',
            uptime: formatUptime(data.uptime_seconds)
          }))}
        </div>

        <div style="display:flex;align-items:center;gap:7px;">
          <button class="btn btn-sm ${monitorAutoRefresh ? '' : 'btn-primary'}" id="btn-toggle-hw-auto" title="${t("hardware.tip_toggle_auto")}">
            ${monitorAutoRefresh ? t("hardware.btn_pause") : t("hardware.btn_auto")}
          </button>
          <select class="select-box" id="sel-hw-interval" title="${t("hardware.tip_interval")}" style="padding:4px 8px;font-size:11.5px;">
            <option value="1000" ${monitorIntervalMs === 1000 ? 'selected' : ''}>${t("hardware.interval_1s")}</option>
            <option value="2000" ${monitorIntervalMs === 2000 ? 'selected' : ''}>${t("hardware.interval_2s")}</option>
            <option value="5000" ${monitorIntervalMs === 5000 ? 'selected' : ''}>${t("hardware.interval_5s")}</option>
          </select>
          <button class="btn btn-sm btn-primary" id="btn-refresh-hw">${t("common.refresh")}</button>
        </div>
      </div>

      <div class="subtab-bar">
        <div class="pill-tabs" id="hw-subtabs">
          <button class="pill-tab ${hwSubTab === 'overview' ? 'active' : ''}" data-sub="overview">
            📊 ${t("hardware.subtab_overview")}
          </button>
          <button class="pill-tab ${hwSubTab === 'processes' ? 'active' : ''}" data-sub="processes">
            ⚙️ ${t("hardware.subtab_processes")}
          </button>
        </div>
        <div class="subtab-bar-right">
          <span class="badge-ok" id="hw-live-status-badge" style="font-size:11px;display:inline-flex;align-items:center;gap:5px;">
            ${monitorAutoRefresh
              ? t("hardware.status_auto", { sec: monitorIntervalMs / 1000 })
              : t("hardware.status_paused")}
          </span>
        </div>
      </div>

      ${hwSubTab === "overview" ? `
        <div id="hw-cards-container">
          ${renderHardwareCardsHtml(data)}
        </div>
      ` : `
        <div id="hw-compact-strip">
          ${renderCompactHwStripHtml(data)}
        </div>

        <div class="card" style="margin-bottom:0;">
          <div class="card-header" style="flex-wrap:wrap;gap:10px;">
            <div class="card-title" style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
              <span>${t("hardware.running_procs_title")}</span>
              <span class="stat-sub" id="proc-count-badge" style="font-weight:500;">
                ${t("hardware.showing_procs", { count: shownCount })}
              </span>
              ${renderTip(t("hardware.running_procs_tip"))}
            </div>

            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
              <input
                type="text"
                class="input-box"
                id="proc-search-input"
                placeholder="${t("hardware.search_placeholder")}"
                value="${escapeHtml(monitorSearchQuery)}"
                style="width:200px;padding:4px 9px;font-size:12px;"
              />

              <div class="pill-tabs" id="proc-sort-tabs">
                <button class="pill-tab ${monitorSort === 'ram' ? 'active' : ''}" data-sort="ram">RAM</button>
                <button class="pill-tab ${monitorSort === 'cpu' ? 'active' : ''}" data-sort="cpu">CPU</button>
                <button class="pill-tab ${monitorSort === 'disk' ? 'active' : ''}" data-sort="disk">${t("hardware.sort_disk")}</button>
                <button class="pill-tab ${monitorSort === 'name' ? 'active' : ''}" data-sort="name">${t("hardware.sort_name")}</button>
              </div>

              <select class="select-box" id="proc-limit-select" style="padding:4px 8px;font-size:11.5px;">
                <option value="15" ${monitorLimit === 15 ? 'selected' : ''}>Top 15</option>
                <option value="25" ${monitorLimit === 25 ? 'selected' : ''}>Top 25</option>
                <option value="50" ${monitorLimit === 50 ? 'selected' : ''}>Top 50</option>
                <option value="100" ${monitorLimit === 100 ? 'selected' : ''}>Top 100</option>
              </select>
            </div>
          </div>

          <div class="table-scroll-box" style="max-height:calc(100vh - 205px);">
            <table class="proc-table">
              <thead>
                <tr>
                  <th class="col-idx">#</th>
                  <th class="col-pid">PID</th>
                  <th class="col-name">${t("hardware.col_process")}</th>
                  <th class="col-ram">${t("hardware.col_ram")}</th>
                  <th class="col-cpu">% CPU</th>
                  <th class="col-io">${t("hardware.col_disk_rw")}</th>
                  <th class="col-act">${t("hardware.col_action")}</th>
                </tr>
              </thead>
              <tbody id="proc-table-body"></tbody>
            </table>
          </div>
        </div>
      `}
    `;

    view.querySelectorAll("#hw-subtabs .pill-tab").forEach(btn => {
      btn.addEventListener("click", () => {
        hwSubTab = btn.dataset.sub || "overview";
        render(view);
      });
    });

    bindOverviewJumpBtn(view);

    document.getElementById("btn-refresh-hw").onclick = () => refreshMonitorDataInPlace(view);

    document.getElementById("btn-toggle-hw-auto").onclick = () => {
      monitorAutoRefresh = !monitorAutoRefresh;
      const badge = document.getElementById("hw-live-status-badge");
      const btn = document.getElementById("btn-toggle-hw-auto");
      if (badge) {
        badge.textContent = monitorAutoRefresh
          ? t("hardware.status_auto", { sec: monitorIntervalMs / 1000 })
          : t("hardware.status_paused");
      }
      if (btn) {
        btn.textContent = monitorAutoRefresh ? t("hardware.btn_pause") : t("hardware.btn_auto");
        btn.classList.toggle("btn-primary", !monitorAutoRefresh);
      }
      if (monitorAutoRefresh) {
        refreshMonitorDataInPlace(view);
        startPolling(view);
      } else {
        stopPolling();
      }
    };

    document.getElementById("sel-hw-interval").onchange = (e) => {
      monitorIntervalMs = Number(e.target.value) || 2000;
      const badge = document.getElementById("hw-live-status-badge");
      if (badge && monitorAutoRefresh) {
        badge.textContent = t("hardware.status_auto", { sec: monitorIntervalMs / 1000 });
      }
      startPolling(view);
    };

    const limitSel = document.getElementById("proc-limit-select");
    if (limitSel) {
      limitSel.onchange = (e) => {
        monitorLimit = Number(e.target.value);
        refreshMonitorDataInPlace(view);
      };
    }

    const searchInput = document.getElementById("proc-search-input");
    if (searchInput) {
      searchInput.oninput = (e) => {
        monitorSearchQuery = e.target.value || "";
        refreshMonitorDataInPlace(view);
      };
    }

    view.querySelectorAll("#proc-sort-tabs .pill-tab").forEach(btn => {
      btn.addEventListener("click", () => {
        monitorSort = btn.dataset.sort;
        view.querySelectorAll("#proc-sort-tabs .pill-tab").forEach(b =>
          b.classList.toggle("active", b.dataset.sort === monitorSort)
        );
        refreshMonitorDataInPlace(view);
      });
    });

    const tbodyEl = document.getElementById("proc-table-body");
    if (tbodyEl) {
      tbodyEl.onclick = async (e) => {
        const btn = e.target.closest(".btn-force-quit");
        if (!btn) return;
        const pid = Number(btn.dataset.pid);
        const name = btn.dataset.name || "";
        const res = await api("kill_process", { pid, force: true });
        showToast(res.ok
          ? t("hardware.toast_terminated", { name, pid })
          : t("hardware.toast_error", { error: res.error })
        );
        refreshMonitorDataInPlace(view);
      };
      syncProcessRowsDom(tbodyEl, procs);
    }

    startPolling(view);
  }

  return {
    render,
    stopPolling
  };
}
