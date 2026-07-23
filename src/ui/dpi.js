import { state, selectedDpiStage, setSelectedDpiStage, saveState, device, profile } from "../state.js";
import { queueDeviceWrite } from "../writer.js";
import { renderHome } from "./home.js";

export function toHex(c) {
  if (!c) return "#ffffff";
  if (c.startsWith("#") && c.length === 7) return c;
  return c;
}

export function renderDpi() {
  const d = device();
  const p = profile();
  const list = document.getElementById("dpi-stages");
  if (!list) return;
  list.innerHTML = "";
  p.dpiStages.forEach((stage, i) => {
    const row = document.createElement("div");
    row.className = `dpi-stage${i === selectedDpiStage ? " active" : ""}`;
    row.innerHTML = `
      <span class="swatch" style="background:${stage.color}"></span>
      <span class="name">DPI Stage ${i + 1}</span>
      <span class="value">${stage.value}</span>
      <span class="active-tag">${i === p.activeDpi ? "ACTIVE" : "Select"}</span>
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

  const stage = p.dpiStages[selectedDpiStage] || p.dpiStages[0];
  const slider = document.getElementById("dpi-slider");
  const input = document.getElementById("dpi-input");
  const color = document.getElementById("dpi-color");
  if (slider && input && color && stage) {
    const maxDpiLimit = d.maxDpi || 26000;
    slider.max = String(maxDpiLimit);
    slider.step = p.settings.fineDpi ? "50" : "100";
    slider.value = Math.min(maxDpiLimit, stage.value);
    input.value = Math.min(maxDpiLimit, stage.value);
    color.value = toHex(stage.color);
  }

  const rates = document.getElementById("rate-options");
  if (!rates) return;
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

export function bindDpiEditors() {
  const apply = (val) => {
    const d = device();
    const p = profile();
    const step = p.settings.fineDpi ? 50 : 100;
    const maxDpiLimit = d.maxDpi || 26000;
    const n = Math.min(maxDpiLimit, Math.max(50, Math.round(Number(val) / step) * step));
    p.dpiStages[selectedDpiStage].value = n;
    const slider = document.getElementById("dpi-slider");
    const input = document.getElementById("dpi-input");
    if (slider) slider.value = n;
    if (input) input.value = n;
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("dpi");
  };

  document.getElementById("dpi-slider")?.addEventListener("input", (e) => apply(e.target.value));
  document.getElementById("dpi-input")?.addEventListener("change", (e) => apply(e.target.value));
  document.getElementById("dpi-color")?.addEventListener("input", (e) => {
    profile().dpiStages[selectedDpiStage].color = e.target.value;
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("dpi", "light");
  });
}
