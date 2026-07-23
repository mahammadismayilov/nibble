import { state, saveState, device, profile, setSelectedDpiStage, toast } from "../state.js";
import { DEVICES, defaultProfile } from "../constants.js";
import { hid } from "../hidInstance.js";
import { renderHome } from "./home.js";
import { renderKeys } from "./keys.js";
import { renderDpi } from "./dpi.js";
import { renderLight } from "./light.js";
import { renderSensor, renderSettings } from "./settings.js";

export { renderHome, renderKeys, renderDpi, renderLight, renderSensor, renderSettings };

export function showTab(name) {
  const tabName = name || "home";
  document.querySelectorAll("#tabs .tab").forEach((t) => {
    const on = t.dataset.tab === tabName;
    t.classList.toggle("active", on);
    t.setAttribute("aria-selected", on ? "true" : "false");
  });
  document.querySelectorAll("main.content > section.panel").forEach((p) => {
    p.classList.toggle("active", p.id === `panel-${tabName}`);
  });
}

export function initTabs() {
  const nav = document.getElementById("tabs");
  if (!nav) return;

  nav.addEventListener("click", (e) => {
    const tab = e.target.closest("button.tab, .tab");
    if (!tab || !nav.contains(tab)) return;
    e.preventDefault();
    const name = tab.getAttribute("data-tab") || tab.dataset.tab;
    if (!name) return;
    showTab(name);
  });
}

export function initDeviceSelect() {
  const sel = document.getElementById("device-select");
  if (!sel) return;
  sel.innerHTML = DEVICES.map((d) => `<option value="${d.id}">${d.name}</option>`).join("");
  sel.value = state.deviceId;
  sel.addEventListener("change", () => {
    state.deviceId = sel.value;
    let idx = state.profiles.findIndex((p) => p.deviceId === state.deviceId);
    if (idx < 0) {
      const p = defaultProfile(state.deviceId);
      p.name = `Profile ${state.profiles.length + 1}`;
      state.profiles.push(p);
      idx = state.profiles.length - 1;
    }
    state.profileIndex = idx;
    setSelectedDpiStage(profile().activeDpi);
    state.selectedKey = device().keys[0].id;
    saveState();
    renderAll();
    toast(`Switched to ${device().name}`);
  });
}

export function initProfileSelect() {
  const sel = document.getElementById("profile-select");
  if (!sel) return;
  const render = () => {
    sel.innerHTML = state.profiles
      .map((p, i) => `<option value="${i}">${p.name}</option>`)
      .join("");
    sel.value = String(state.profileIndex);
  };
  render();
  sel.addEventListener("change", () => {
    state.profileIndex = Number(sel.value);
    state.deviceId = profile().deviceId;
    const devSel = document.getElementById("device-select");
    if (devSel) devSel.value = state.deviceId;
    setSelectedDpiStage(profile().activeDpi);
    saveState();
    renderAll();
  });
  initProfileSelect.refresh = render;
}

export function renderConnection() {
  const el = document.getElementById("connection-status");
  if (!el) return;
  const text = el.querySelector(".conn-text");
  const btnC = document.getElementById("btn-connect");
  const btnD = document.getElementById("btn-disconnect");
  el.classList.remove("offline", "warn");

  if (hid.connected) {
    const info = hid.info;
    const pid = info.productId.toString(16).toUpperCase().padStart(4, "0");
    const vid = info.vendorId.toString(16).toUpperCase().padStart(4, "0");
    const devIdStr = state.devId ? ` (${state.devId})` : "";
    
    let sub = "Online";
    let extra = "";
    if (state.batteryOnline === false) {
      sub = "Mouse sleeping";
    } else if (state.batteryCharging) {
      sub = "Charging";
    } else if (state.batteryIsLast && typeof state.battery === "number") {
      sub = "Last read";
      extra = ` (${state.battery}%)`;
    }

    text.innerHTML = `<strong>${device().name}</strong> · ${vid}:${pid}${devIdStr} · ${sub}${extra}`;
    btnC.style.display = "none";
    btnD.style.display = "";

    const fwEl = document.getElementById("fw-ver");
    if (fwEl) fwEl.textContent = state.fwVersion || "v1.0.3";
    const vpEl = document.getElementById("vidpid");
    if (vpEl) vpEl.textContent = `${vid}:${pid}`;

    const batPct = document.getElementById("battery-pct");
    const batLvl = document.getElementById("battery-level");
    if (batPct && batLvl) {
      if (typeof state.battery === "number" && state.battery >= 0) {
        batPct.textContent = `${Math.min(100, state.battery)}%`;
        batLvl.style.width = `${Math.min(100, state.battery)}%`;
      } else {
        batPct.textContent = "—";
        batLvl.style.width = "0%";
      }
    }
  } else {
    el.classList.add("offline");
    text.innerHTML = "<strong>Disconnected</strong> · Connect mouse receiver via WebHID";
    btnC.style.display = "";
    btnD.style.display = "none";
    const batPct = document.getElementById("battery-pct");
    const batLvl = document.getElementById("battery-level");
    if (batPct) batPct.textContent = "—";
    if (batLvl) batLvl.style.width = "0%";
  }
}

export function renderAll() {
  const devSel = document.getElementById("device-select");
  if (devSel) devSel.value = state.deviceId;
  if (initProfileSelect.refresh) initProfileSelect.refresh();
  renderConnection();
  renderHome();
  renderKeys();
  renderDpi();
  renderLight();
  renderSettings();
}
