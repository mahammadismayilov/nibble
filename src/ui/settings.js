import { saveState, profile } from "../state.js";
import { defaultProfile } from "../constants.js";
import { getTheme } from "../theme.js";
import { queueDeviceWrite } from "../writer.js";
import { renderHome } from "./home.js";
import { renderConnection } from "./navigation.js";

export function renderSensor() {
  /* Sensor settings are rendered in renderSettings() */
}

export function renderSettings() {
  const p = profile();
  if (!p.settings) p.settings = defaultProfile(p.deviceId || state.deviceId).settings;
  const s = p.settings;
  setSegmented("lod-options", s.lod);
  setSegmented("mode-options", s.mode);
  setSegmented("theme-options", getTheme());
  const setCheck = (id, val) => {
    const el = document.getElementById(id);
    if (el) el.checked = !!val;
  };
  const setVal = (id, val) => {
    const el = document.getElementById(id);
    if (el) el.value = val;
  };
  setCheck("opt-angle-snap", s.angleSnap);
  setCheck("opt-ripple", s.ripple);
  setVal("opt-debounce", s.debounce);
  setCheck("opt-highspeed", s.highSpeed);
  {
    let w = s.sleepWire;
    if (w == null && s.sleepMin != null) {
      w = s.sleepMin <= 30 && s.sleepMin >= 1 ? s.sleepMin * 6 : 30;
      s.sleepWire = w;
    }
    setVal("opt-sleep", w ?? 30);
  }
  setCheck("opt-move-wake", s.moveWake);
  setCheck("opt-move-light", s.moveCloseLight);
  setCheck("opt-lmb-lock", s.lmbLock !== false);
  setCheck("opt-fine-dpi", s.fineDpi);
  setCheck("opt-low-battery-warn", s.lowBatteryWarn);
  setCheck("opt-battery-rgb-sync", s.batteryRgbSync);
}

export function setSegmented(id, value) {
  document.querySelectorAll(`#${id} button`).forEach((b) => {
    b.classList.toggle("active", b.dataset.value === value);
  });
}

export function bindSettingsEditors() {
  const bindSeg = (id, key, scope) => {
    document.querySelectorAll(`#${id} button`).forEach((b) => {
      b.addEventListener("click", () => {
        profile().settings[key] = b.dataset.value;
        saveState();
        renderSettings();
        renderConnection();
        renderHome();
        if (scope) queueDeviceWrite(scope);
      });
    });
  };
  bindSeg("lod-options", "lod", "sensor");
  bindSeg("mode-options", "mode", null);

  if (profile().settings.mode === "bt") {
    profile().settings.mode = "usb";
    saveState();
  }

  const bindCheck = (id, key, scope) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener("change", (e) => {
      profile().settings[key] = e.target.checked;
      saveState();
      if (scope) queueDeviceWrite(scope);
    });
  };
  bindCheck("opt-angle-snap", "angleSnap", "sensor");
  bindCheck("opt-ripple", "ripple", "sensor");
  bindCheck("opt-highspeed", "highSpeed", null);
  bindCheck("opt-move-wake", "moveWake", "power");
  bindCheck("opt-move-light", "moveCloseLight", "power");
  bindCheck("opt-lmb-lock", "lmbLock", null);
  
  bindCheck("opt-fine-dpi", "fineDpi", null);
  bindCheck("opt-low-battery-warn", "lowBatteryWarn", null);
  bindCheck("opt-battery-rgb-sync", "batteryRgbSync", "light");

  document.getElementById("opt-debounce")?.addEventListener("change", (e) => {
    profile().settings.debounce = Number(e.target.value) || 0;
    saveState();
    queueDeviceWrite("power");
  });
  document.getElementById("opt-sleep")?.addEventListener("change", (e) => {
    profile().settings.sleepWire = Number(e.target.value) || 30;
    delete profile().settings.sleepMin;
    saveState();
    queueDeviceWrite("power");
  });
}
