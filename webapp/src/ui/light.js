import { saveState, device, profile } from "../state.js";
import { queueDeviceWrite } from "../writer.js";
import { renderHome } from "./home.js";
import { toHex } from "./dpi.js";

export function renderLight() {
  const d = device();
  const p = profile();
  const grid = document.getElementById("effect-grid");
  if (!grid) return;
  grid.innerHTML = "";
  d.lights.forEach((fx) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = fx.name;
    btn.disabled = !fx.enable;
    if (p.light.mode === fx.id) btn.classList.add("active");
    btn.addEventListener("click", () => {
      if (!fx.enable) return;
      p.light.mode = fx.id;
      saveState();
      renderLight();
      renderHome();
      queueDeviceWrite("light");
    });
    grid.appendChild(btn);
  });

  const briEl = document.getElementById("light-brightness");
  const briVal = document.getElementById("light-brightness-val");
  const spdEl = document.getElementById("light-speed");
  const spdVal = document.getElementById("light-speed-val");

  if (briEl) briEl.value = p.light.brightness;
  if (briVal) briVal.textContent = `${p.light.brightness}%`;
  if (spdEl) spdEl.value = p.light.speed;
  if (spdVal) spdVal.textContent = p.light.speed;

  const colorRow = document.getElementById("light-color-row");
  const speedRow = document.getElementById("light-speed-row");
  const note = document.getElementById("light-mode-note");
  const colorInput = document.getElementById("light-color");
  const dpiCol = p.dpiStages[p.activeDpi]?.color || "#ff0000";

  if (p.light.mode === "solid" || p.light.mode === "breathe") {
    if (p.settings?.batteryRgbSync) {
      if (colorInput) {
        colorInput.value = toHex(p.light.color || "#00ff00");
        colorInput.disabled = true;
      }
      if (colorRow) colorRow.style.opacity = "0.55";
      if (speedRow) speedRow.style.display = p.light.mode === "solid" ? "none" : "";
      if (note) {
        note.textContent = "Battery-Sync RGB Mode Active — LED color reflects current battery level (Green > 50%, Yellow 25-50%, Red < 25%).";
      }
    } else {
      p.light.color = dpiCol;
      if (colorInput) {
        colorInput.value = toHex(dpiCol);
        colorInput.disabled = true;
      }
      if (colorRow) colorRow.style.opacity = "0.55";
      if (speedRow) speedRow.style.display = p.light.mode === "solid" ? "none" : "";
      if (note) {
        const modeName = p.light.mode === "solid" ? "Constant" : "Breathing";
        const extraSpeed = p.light.mode === "breathe" ? " Speed & brightness apply." : "";
        note.textContent =
          `${modeName} mode has no separate color (same as official app). LED color = active DPI stage color — change it on the DPI tab.${extraSpeed}`;
      }
    }
  } else if (p.light.mode === "off") {
    if (colorInput) colorInput.disabled = true;
    if (colorRow) colorRow.style.opacity = "0.4";
    if (speedRow) speedRow.style.display = "none";
    if (note) note.textContent = "Lighting off.";
  } else {
    if (colorInput) {
      colorInput.disabled = false;
      colorInput.value = toHex(p.light.color || dpiCol);
    }
    if (colorRow) colorRow.style.opacity = "1";
    if (speedRow) speedRow.style.display = "";
    if (note) note.textContent = "";
  }
  updateLightPreview();
}

export function bindLightEditors() {
  document.getElementById("light-brightness")?.addEventListener("input", (e) => {
    profile().light.brightness = Number(e.target.value);
    const briVal = document.getElementById("light-brightness-val");
    if (briVal) briVal.textContent = `${e.target.value}%`;
    saveState();
    updateLightPreview();
    renderHome();
    queueDeviceWrite("light");
  });
  document.getElementById("light-speed")?.addEventListener("input", (e) => {
    profile().light.speed = Number(e.target.value);
    const spdVal = document.getElementById("light-speed-val");
    if (spdVal) spdVal.textContent = e.target.value;
    saveState();
    updateLightPreview();
    queueDeviceWrite("light");
  });
  document.getElementById("light-color")?.addEventListener("input", (e) => {
    profile().light.color = e.target.value;
    saveState();
    updateLightPreview();
    renderHome();
    queueDeviceWrite("light");
  });
}

export function updateLightPreview() {
  const p = profile();
  const orb = document.getElementById("light-orb");
  const stage = document.getElementById("stage-glow");
  if (!orb && !stage) return;
  const dpiCol = p.dpiStages[p.activeDpi]?.color || "#ff0000";
  const color =
    p.light.mode === "off"
      ? "#000000"
      : p.settings?.batteryRgbSync && p.light.color
        ? p.light.color
        : p.light.mode === "solid"
          ? dpiCol
          : p.light.color || dpiCol;
  const bright = p.light.brightness / 100;

  if (p.light.mode === "off") {
    if (orb) {
      orb.style.opacity = "0";
      orb.style.animation = "none";
    }
    if (stage) {
      stage.style.opacity = "0.12";
      stage.style.background = "radial-gradient(circle, rgba(120,120,140,0.14), transparent 72%)";
      stage.style.animation = "none";
    }
    return;
  }

  const modeMul =
    p.light.mode === "breathe" || p.light.mode === "flow"
      ? 0.85
      : p.light.mode === "solid"
        ? 1
        : 0.9;
  const alpha = (0.18 + bright * 0.4) * modeMul;
  const bg = `radial-gradient(circle, ${hexToRgba(color, alpha)}, transparent 72%)`;
  if (orb) {
    orb.style.background = bg;
    orb.style.opacity = String((0.3 + bright * 0.4) * modeMul);
    orb.style.animation = "none";
  }
  if (stage) {
    stage.style.background = bg;
    stage.style.opacity = String((0.28 + bright * 0.35) * modeMul);
    stage.style.animation = "none";
  }
}

export function hexToRgba(hex, a) {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return `rgba(${r},${g},${b},${a})`;
}
