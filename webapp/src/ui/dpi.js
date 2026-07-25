import { state, selectedDpiStage, setSelectedDpiStage, saveState, device, profile } from "../state.js";
import { queueDeviceWrite } from "../writer.js";
import { renderHome } from "./home.js";

export function toHex(c) {
  if (!c) return "#ffffff";
  if (c.startsWith("#") && c.length === 7) return c;
  return c;
}

export function dpiToPercent(dpi, maxDpi = 26000) {
  const minDpi = 100;
  const clamped = Math.max(minDpi, Math.min(maxDpi, dpi));
  const minLog = Math.log(minDpi);
  const maxLog = Math.log(maxDpi);
  const valLog = Math.log(clamped);
  return Math.max(3, Math.min(97, ((valLog - minLog) / (maxLog - minLog)) * 100));
}

export function percentToDpi(pct, maxDpi = 26000, step = 100) {
  const minDpi = 100;
  const clampedPct = Math.max(0, Math.min(100, pct));
  const minLog = Math.log(minDpi);
  const maxLog = Math.log(maxDpi);
  const valLog = minLog + (clampedPct / 100) * (maxLog - minLog);
  const raw = Math.exp(valLog);
  const rounded = Math.round(raw / step) * step;
  return Math.max(50, Math.min(maxDpi, rounded));
}

export function renderDpi() {
  const d = device();
  const p = profile();
  const maxDpiLimit = d.maxDpi || 26000;

  // Active DPI Stage Count (1..6 preset levels)
  const stageCount = Math.min(6, Math.max(1, p.dpiStageCount || p.dpiStages.length || 6));
  p.dpiStageCount = stageCount;
  if (p.activeDpi >= stageCount) p.activeDpi = stageCount - 1;
  if (selectedDpiStage >= stageCount) setSelectedDpiStage(stageCount - 1);

  document.querySelectorAll("#dpi-level-options button").forEach((btn) => {
    btn.classList.toggle("active", Number(btn.dataset.count) === stageCount);
  });

  const enabledStages = p.dpiStages.slice(0, stageCount);

  // Active Stage Diamond Header Swatch
  const activeDiamond = document.getElementById("dpi-active-diamond");
  const activeStage = p.dpiStages[p.activeDpi] || p.dpiStages[0];
  if (activeDiamond && activeStage) {
    activeDiamond.style.color = activeStage.color || "#f5a623";
  }

  // Render G HUB Interactive DPI Timeline Track
  const valContainer = document.getElementById("dpi-timeline-values");
  const nodesContainer = document.getElementById("dpi-timeline-nodes");

  if (valContainer && nodesContainer) {
    valContainer.innerHTML = "";
    nodesContainer.innerHTML = "";

    enabledStages.forEach((stage, i) => {
      const pct = dpiToPercent(stage.value, maxDpiLimit);
      const isActive = i === p.activeDpi;
      const isSelected = i === selectedDpiStage;

      // 1. DPI Value Number above line
      const valNode = document.createElement("div");
      valNode.className = `dpi-val-node${isActive ? " active" : ""}${isSelected ? " selected" : ""}`;
      valNode.style.left = `${pct}%`;
      valNode.style.setProperty("--node-color", stage.color || "#f5a623");
      valNode.innerHTML = `<span class="dpi-val-num">${stage.value}</span>`;

      valNode.addEventListener("click", (e) => {
        e.stopPropagation();
        setSelectedDpiStage(i);
        p.activeDpi = i;
        saveState();
        renderDpi();
        renderHome();
        queueDeviceWrite("dpi", "light");
      });
      valContainer.appendChild(valNode);

      // 2. Track Node Marker (Diamond if active, Dot if inactive)
      const trackNode = document.createElement("div");
      trackNode.className = `dpi-track-node${isActive ? " active" : ""}`;
      trackNode.style.left = `${pct}%`;
      trackNode.style.setProperty("--node-color", stage.color || "#f5a623");
      trackNode.innerHTML = `<div class="node-dot"></div>`;

      trackNode.addEventListener("click", (e) => {
        e.stopPropagation();
        setSelectedDpiStage(i);
        p.activeDpi = i;
        saveState();
        renderDpi();
        renderHome();
        queueDeviceWrite("dpi", "light");
      });
      nodesContainer.appendChild(trackNode);
    });
  }

  // Render Scale Ticks (200, 1000, 2000, 3000, 4000, 6000, 8000...)
  const scaleEl = document.getElementById("dpi-timeline-scale");
  if (scaleEl) {
    scaleEl.innerHTML = "";
    const scaleValues = [200, 1000, 2000, 3000, 4000, 6000, 8000];
    if (maxDpiLimit > 8000) scaleValues.push(maxDpiLimit);

    scaleValues.forEach((val) => {
      const pct = dpiToPercent(val, maxDpiLimit);
      const tick = document.createElement("div");
      tick.className = "scale-tick";
      tick.style.left = `${pct}%`;
      tick.innerHTML = `<span>${val}</span>`;
      scaleEl.appendChild(tick);
    });
  }

  // Render Compact Stage Selector Chips
  const list = document.getElementById("dpi-stages");
  if (list) {
    list.innerHTML = "";
    enabledStages.forEach((stage, i) => {
      const row = document.createElement("div");
      row.className = `dpi-stage${i === selectedDpiStage ? " active" : ""}`;
      row.innerHTML = `
        <span class="swatch" style="background:${stage.color}"></span>
        <span class="name">Stage ${i + 1}</span>
        <span class="value">${stage.value} DPI</span>
        ${i === p.activeDpi ? '<span class="active-tag">ACTIVE</span>' : ''}
      `;
      row.addEventListener("click", () => {
        setSelectedDpiStage(i);
        p.activeDpi = i;
        saveState();
        renderDpi();
        renderHome();
        queueDeviceWrite("dpi", "light");
      });
      list.appendChild(row);
    });
  }

  // Update Inputs
  const stage = p.dpiStages[selectedDpiStage] || p.dpiStages[0];
  const input = document.getElementById("dpi-input");
  const color = document.getElementById("dpi-color");
  if (input && color && stage) {
    input.value = Math.min(maxDpiLimit, stage.value);
    color.value = toHex(stage.color);
  }

  // Render Report Rate Options
  const rates = document.getElementById("rate-options");
  if (rates) {
    rates.innerHTML = "";
    d.reportRates.forEach((hz, i) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = i === p.reportRateIndex ? "active" : "";
      btn.innerHTML = `${hz} Hz<span>${i === d.defaultRateIndex ? "Default" : "Report rate"}</span>`;
      btn.addEventListener("click", () => {
        p.reportRateIndex = i;
        saveState();
        renderDpi();
        renderHome();
        queueDeviceWrite("rate");
      });
      rates.appendChild(btn);
    });
  }
}

export function bindDpiEditors() {
  // DPI Stage Count Selector (1..6 levels)
  document.querySelectorAll("#dpi-level-options button").forEach((btn) => {
    btn.addEventListener("click", () => {
      const count = Number(btn.dataset.count) || 6;
      const p = profile();
      p.dpiStageCount = count;
      if (p.activeDpi >= count) p.activeDpi = count - 1;
      if (selectedDpiStage >= count) setSelectedDpiStage(count - 1);
      saveState();
      renderDpi();
      renderHome();
      queueDeviceWrite("dpi", "light");
    });
  });

  const apply = (val) => {
    const d = device();
    const p = profile();
    const step = p.settings.fineDpi ? 50 : 100;
    const maxDpiLimit = d.maxDpi || 26000;
    const n = Math.min(maxDpiLimit, Math.max(50, Math.round(Number(val) / step) * step));
    p.dpiStages[selectedDpiStage].value = n;
    const input = document.getElementById("dpi-input");
    if (input) input.value = n;
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("dpi");
  };

  document.getElementById("dpi-input")?.addEventListener("change", (e) => apply(e.target.value));
  document.getElementById("dpi-color")?.addEventListener("input", (e) => {
    const stage = profile().dpiStages[selectedDpiStage];
    if (stage) {
      stage.color = e.target.value;
      stage.userColor = e.target.value;
    }
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("dpi", "light");
  });

  // Track Line Click & Drag Interaction
  const track = document.getElementById("dpi-timeline-track");
  if (track && !track.dataset.bound) {
    track.dataset.bound = "true";
    const handleTrackMove = (e) => {
      const rect = track.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const pct = Math.max(0, Math.min(100, (clickX / rect.width) * 100));
      const d = device();
      const p = profile();
      const step = p.settings.fineDpi ? 50 : 100;
      const maxDpiLimit = d.maxDpi || 26000;
      const newDpi = percentToDpi(pct, maxDpiLimit, step);
      p.dpiStages[selectedDpiStage].value = newDpi;
      saveState();
      renderDpi();
      renderHome();
      queueDeviceWrite("dpi");
    };

    let isDragging = false;
    track.addEventListener("mousedown", (e) => {
      isDragging = true;
      handleTrackMove(e);
    });
    window.addEventListener("mousemove", (e) => {
      if (isDragging) handleTrackMove(e);
    });
    window.addEventListener("mouseup", () => {
      if (isDragging) isDragging = false;
    });
  }
}
