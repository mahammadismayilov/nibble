import { STORAGE_KEY, DEVICES, defaultProfile } from "./constants.js";

export function defaultState() {
  return {
    deviceId: DEVICES[0].id,
    profileIndex: 0,
    profiles: [defaultProfile(DEVICES[0].id)],
    selectedKey: 201,
    selectedMacro: null,
    recording: false,
    battery: null,
  };
}

export const BATTERY_RUNTIME_KEYS = [
  "battery",
  "batteryCharging",
  "batteryOnline",
  "batteryIsLast",
  "batterySource",
  "batteryConfidence",
  "batteryDebug",
  "_lastDockFullAt",
  "_batterySeenLive",
  "_lowBatteryWarned",
];

export function clearBatteryRuntime(target = state) {
  target.battery = null;
  target.batteryCharging = false;
  target.batteryOnline = true;
  target.batteryIsLast = false;
  target.batterySource = null;
  target.batteryConfidence = null;
  target.batteryDebug = "";
  target._lastDockFullAt = 0;
  target._batterySeenLive = false;
  target._lowBatteryWarned = false;
  target._currentBatteryRgbTier = null;
}

export function loadState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultState();
    const parsed = JSON.parse(raw);
    if (!parsed.profiles?.length) return defaultState();
    const merged = { ...defaultState(), ...parsed };
    for (const k of BATTERY_RUNTIME_KEYS) delete merged[k];
    clearBatteryRuntime(merged);
    merged.profiles?.forEach((p) => {
      if (p.keys) {
        const vals = Object.values(p.keys);
        if (vals.length > 0 && vals.every((v) => v === "disable")) {
          const dev = DEVICES.find((d) => d.id === p.deviceId) || DEVICES[0];
          dev.keys.forEach((k) => {
            p.keys[k.id] = k.defaultFunc;
          });
        }
      }
    });
    return merged;
  } catch {
    return defaultState();
  }
}

export const state = loadState();

export let selectedDpiStage = state.profiles[state.profileIndex]?.activeDpi ?? 0;
export function setSelectedDpiStage(idx) {
  selectedDpiStage = idx;
}

export function saveState() {
  const copy = { ...state };
  for (const k of BATTERY_RUNTIME_KEYS) delete copy[k];
  localStorage.setItem(STORAGE_KEY, JSON.stringify(copy));
}

export function device() {
  return DEVICES.find((d) => d.id === state.deviceId) || DEVICES[0];
}

export function profile() {
  return state.profiles[state.profileIndex];
}

export function toast(msg) {
  const el = document.getElementById("toast");
  if (!el) return;
  const exitMs = 220;
  el.textContent = msg;
  el.hidden = false;
  requestAnimationFrame(() => el.classList.add("show"));
  clearTimeout(toast._t);
  toast._t = setTimeout(() => {
    el.classList.remove("show");
    setTimeout(() => {
      el.hidden = true;
    }, exitMs);
  }, 2200);
}

export function setStatus(msg) {
  const el = document.getElementById("status-msg");
  if (el) el.textContent = msg;
}
