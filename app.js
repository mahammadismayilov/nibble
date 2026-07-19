/**
 * Nibble — open WebHID mouse configurator
 * Protocol reverse-engineered from OEM firmware tooling; no unknown probes.
 */

import {
  buildStatusQuery,
  buildReportRate,
  buildReportRateGet,
  parseReportRateResponse,
  buildDpi,
  buildDpiGet,
  parseDpiResponse,
  buildDpiRgb,
  buildLight,
  buildLightGet,
  buildSolidCapture,
  buildOffCapture,
  buildBreatheCapture,
  parseLightResponse,
  buildSensor,
  buildPower,
  buildKeyMap,
  KEY_FUNC_PROVEN,
  lightIdToWire,
  wireToLightId,
  parseStatus,
  normalizeBatteryPercent,
  bufToHex,
  uiBrightnessToWire,
  uiSpeedToWire,
} from "./protocol.js";
import { NibbleHid, webHidSupported } from "./hid.js";

const APP_NAME = "Nibble";
const APP_VERSION = "1.0.0";
const STORAGE_KEY = "nibble-web-v1";
const THEME_KEY = "nibble-theme";
const LEGACY_STORAGE_KEY = "ajazz-driver-web-v1";
const LEGACY_THEME_KEY = "ajazz-theme";
const hid = new NibbleHid();

/** One-time migrate localStorage from older private builds */
function migrateLegacyStorage() {
  try {
    if (!localStorage.getItem(STORAGE_KEY) && localStorage.getItem(LEGACY_STORAGE_KEY)) {
      localStorage.setItem(STORAGE_KEY, localStorage.getItem(LEGACY_STORAGE_KEY));
    }
    if (!localStorage.getItem(THEME_KEY) && localStorage.getItem(LEGACY_THEME_KEY)) {
      localStorage.setItem(THEME_KEY, localStorage.getItem(LEGACY_THEME_KEY));
    }
  } catch {
    /* ignore */
  }
}
migrateLegacyStorage();

/* ---------- Theme (light default) ---------- */
function getTheme() {
  const t = document.documentElement.getAttribute("data-theme");
  return t === "dark" ? "dark" : "light";
}

function setTheme(theme) {
  const next = theme === "dark" ? "dark" : "light";
  document.documentElement.setAttribute("data-theme", next);
  try {
    localStorage.setItem(THEME_KEY, next);
  } catch {
    /* ignore */
  }
  const btn = document.getElementById("btn-theme");
  if (btn) {
    btn.title = next === "light" ? "Switch to dark theme" : "Switch to light theme";
    btn.setAttribute(
      "aria-label",
      next === "light" ? "Switch to dark theme" : "Switch to light theme"
    );
  }
  // theme-options only exists after DOM parse; safe no-op if missing
  const themeSeg = document.getElementById("theme-options");
  if (themeSeg) {
    themeSeg.querySelectorAll("button[data-value]").forEach((b) => {
      b.classList.toggle("active", b.dataset.value === next);
    });
  }
}

function toggleTheme() {
  setTheme(getTheme() === "light" ? "dark" : "light");
}

function initTheme() {
  let theme = "light";
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "light" || saved === "dark") theme = saved;
    else if (window.matchMedia("(prefers-color-scheme: dark)").matches) theme = "dark";
  } catch {
    theme = "light";
  }
  setTheme(theme);

  document.getElementById("btn-theme")?.addEventListener("click", () => {
    toggleTheme();
  });

  document.getElementById("theme-options")?.addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-value]");
    if (!btn) return;
    setTheme(btn.dataset.value);
  });
}

const DEVICES = [
  {
    id: "aj179",
    name: "AJ179",
    type: 105,
    image: "assets/device/mouse_aj179.png",
    sensor: "PAW3395",
    modes: [
      { value: 0, desc: "USB", vid: "248A", pid: "5C2E", devId: "M179" },
      { value: 1, desc: "2.4G", vid: "248A", pid: "5C2F", devId: "M179" },
    ],
    keys: [
      { id: 201, keyValue: 0, direction: "right", x: 29.3, y: 29.7, defaultFunc: "left", lockedDefault: true },
      { id: 202, keyValue: 1, direction: "left", x: 70.4, y: 29.5, defaultFunc: "right" },
      { id: 203, keyValue: 2, direction: "bottom", x: 50.2, y: 16.5, defaultFunc: "middle" },
      { id: 204, keyValue: 4, direction: "right", x: 22.3, y: 40.9, defaultFunc: "forward" },
      { id: 205, keyValue: 3, direction: "right", x: 22, y: 52.6, defaultFunc: "backward" },
      { id: 206, keyValue: 5, direction: "left", x: 61.2, y: 54, defaultFunc: "dpi_loop" },
    ],
    dpiDefaults: [
      { value: 400, color: "#FF0000" },
      { value: 800, color: "#00FF00" },
      { value: 1200, color: "#0000FF" },
      { value: 1600, color: "#00FFFF" },
      { value: 2400, color: "#FFFF00" },
      { value: 3200, color: "#800080" },
    ],
    defaultDpiIndex: 1,
    reportRates: [125, 250, 500, 1000],
    defaultRateIndex: 3,
    lights: [
      { id: "flow", name: "Flowing light", enable: false },
      { id: "breathe", name: "Breathing", enable: true },
      { id: "solid", name: "Constant light", enable: true },
      { id: "neon", name: "Neon", enable: false },
      { id: "wave", name: "Colorful waves", enable: false },
      { id: "off", name: "Close", enable: true },
    ],
    defaultLight: "breathe",
  },
  {
    id: "aj139pro",
    name: "AJ139 Pro",
    type: 101,
    image: "assets/device/mouse_aj139pro.png",
    sensor: "PAW3395",
    modes: [
      { value: 0, desc: "USB", vid: "248A", pid: "5C2E", devId: "M129" },
      { value: 1, desc: "2.4G", vid: "248A", pid: "5C2F", devId: "M129" },
    ],
    keys: [
      { id: 201, keyValue: 0, direction: "right", x: 29.3, y: 29.7, defaultFunc: "left", lockedDefault: true },
      { id: 202, keyValue: 1, direction: "left", x: 70.4, y: 29.5, defaultFunc: "right" },
      { id: 203, keyValue: 2, direction: "bottom", x: 50.2, y: 16.5, defaultFunc: "middle" },
      { id: 204, keyValue: 4, direction: "right", x: 22.3, y: 40.9, defaultFunc: "forward" },
      { id: 205, keyValue: 3, direction: "right", x: 22, y: 52.6, defaultFunc: "backward" },
      { id: 206, keyValue: 5, direction: "left", x: 61.2, y: 54, defaultFunc: "dpi_loop" },
    ],
    dpiDefaults: [
      { value: 400, color: "#FF0000" },
      { value: 800, color: "#00FF00" },
      { value: 1200, color: "#0000FF" },
      { value: 1600, color: "#00FFFF" },
      { value: 2400, color: "#FFFF00" },
      { value: 3200, color: "#800080" },
    ],
    defaultDpiIndex: 1,
    reportRates: [125, 250, 500, 1000],
    defaultRateIndex: 3,
    lights: [
      { id: "flow", name: "Flowing light", enable: false },
      { id: "breathe", name: "Breathing", enable: true },
      { id: "solid", name: "Constant light", enable: true },
      { id: "neon", name: "Neon", enable: false },
      { id: "wave", name: "Colorful waves", enable: false },
      { id: "off", name: "Close", enable: true },
    ],
    defaultLight: "breathe",
  },
  {
    id: "aj159",
    name: "AJ159",
    type: 102,
    image: "assets/device/mouse_aj159.png",
    sensor: "PAW3395",
    modes: [
      { value: 0, desc: "USB", vid: "248A", pid: "5C2E", devId: "M620" },
      { value: 1, desc: "2.4G", vid: "248A", pid: "5C2F", devId: "M620" },
    ],
    keys: [
      { id: 201, keyValue: 0, direction: "right", x: 29.3, y: 29.7, defaultFunc: "left", lockedDefault: true },
      { id: 202, keyValue: 1, direction: "left", x: 70.4, y: 29.5, defaultFunc: "right" },
      { id: 203, keyValue: 2, direction: "bottom", x: 50.2, y: 16.5, defaultFunc: "middle" },
      { id: 204, keyValue: 4, direction: "right", x: 22.3, y: 40.9, defaultFunc: "forward" },
      { id: 205, keyValue: 3, direction: "right", x: 22, y: 52.6, defaultFunc: "backward" },
      { id: 206, keyValue: 5, direction: "left", x: 61.2, y: 54, defaultFunc: "dpi_loop" },
    ],
    dpiDefaults: [
      { value: 400, color: "#FF0000" },
      { value: 800, color: "#00FF00" },
      { value: 1200, color: "#0000FF" },
      { value: 1600, color: "#00FFFF" },
      { value: 2400, color: "#FFFF00" },
      { value: 3200, color: "#800080" },
    ],
    defaultDpiIndex: 1,
    reportRates: [125, 250, 500, 1000],
    defaultRateIndex: 3,
    lights: [
      { id: "flow", name: "Flowing light", enable: false },
      { id: "breathe", name: "Breathing", enable: true },
      { id: "solid", name: "Constant light", enable: true },
      { id: "neon", name: "Neon", enable: false },
      { id: "wave", name: "Colorful waves", enable: false },
      { id: "off", name: "Close", enable: true },
    ],
    defaultLight: "breathe",
  },
  {
    id: "aj159mc",
    name: "AJ159 MC",
    type: 103,
    image: "assets/device/mouse_aj159mc.png",
    sensor: "PAW3395",
    modes: [
      { value: 0, desc: "USB", vid: "248A", pid: "5C2E", devId: "M630" },
      { value: 1, desc: "2.4G", vid: "248A", pid: "5C2F", devId: "M630" },
    ],
    keys: [
      { id: 201, keyValue: 0, direction: "right", x: 29.3, y: 29.7, defaultFunc: "left", lockedDefault: true },
      { id: 202, keyValue: 1, direction: "left", x: 70.4, y: 29.5, defaultFunc: "right" },
      { id: 203, keyValue: 2, direction: "bottom", x: 50.2, y: 16.5, defaultFunc: "middle" },
      { id: 204, keyValue: 4, direction: "right", x: 22.3, y: 40.9, defaultFunc: "forward" },
      { id: 205, keyValue: 3, direction: "right", x: 22, y: 52.6, defaultFunc: "backward" },
      { id: 206, keyValue: 5, direction: "left", x: 61.2, y: 54, defaultFunc: "dpi_loop" },
    ],
    dpiDefaults: [
      { value: 400, color: "#FF0000" },
      { value: 800, color: "#00FF00" },
      { value: 1200, color: "#0000FF" },
      { value: 1600, color: "#00FFFF" },
      { value: 2400, color: "#FFFF00" },
      { value: 3200, color: "#800080" },
    ],
    defaultDpiIndex: 1,
    reportRates: [125, 250, 500, 1000],
    defaultRateIndex: 3,
    lights: [
      { id: "flow", name: "Flowing light", enable: false },
      { id: "breathe", name: "Breathing", enable: true },
      { id: "solid", name: "Constant light", enable: true },
      { id: "neon", name: "Neon", enable: false },
      { id: "wave", name: "Colorful waves", enable: false },
      { id: "off", name: "Close", enable: true },
    ],
    defaultLight: "breathe",
  },
];

const KEY_LABELS = {
  201: "Left Button",
  202: "Right Button",
  203: "Middle / Wheel",
  204: "Forward",
  205: "Backward",
  206: "DPI Button",
};

const FUNC_GROUPS = [
  {
    title: "Mouse Features",
    items: [
      { id: "left", label: "Left-Click" },
      { id: "right", label: "Right-Click" },
      { id: "middle", label: "Middle-Click" },
      { id: "forward", label: "Forward" },
      { id: "backward", label: "Backward" },
      { id: "double", label: "Double-Click" },
      { id: "scroll_up", label: "Scroll Up" },
      { id: "scroll_down", label: "Scroll Down" },
      { id: "dpi_loop", label: "DPI Loop" },
      { id: "fire", label: "Fire button" },
      { id: "disable", label: "Disable Key" },
    ],
  },
  {
    title: "Multimedia",
    items: [
      { id: "vol_up", label: "Volume +" },
      { id: "vol_down", label: "Volume -" },
      { id: "mute", label: "Mute" },
      { id: "play", label: "Play/Pause" },
      { id: "prev", label: "Previous" },
      { id: "next", label: "Next" },
      { id: "stop", label: "Stop" },
    ],
  },
  {
    title: "System Shortcut",
    items: [
      { id: "calc", label: "Calculator" },
      { id: "computer", label: "My Computer" },
      { id: "mail", label: "Mail" },
      { id: "refresh", label: "Refresh (F5)" },
      { id: "switch_app", label: "Switch Application" },
      { id: "copy", label: "Copy" },
      { id: "paste", label: "Paste" },
      { id: "brightness_up", label: "Screen Brightness +" },
      { id: "brightness_down", label: "Screen Brightness -" },
    ],
  },
  {
    title: "Lighting / Macro",
    items: [
      { id: "light_toggle", label: "Lighting on/off" },
      { id: "macro", label: "Macro" },
    ],
  },
];

const FUNC_LABEL = Object.fromEntries(
  FUNC_GROUPS.flatMap((g) => g.items.map((i) => [i.id, i.label]))
);

function defaultProfile(deviceId) {
  const device = DEVICES.find((d) => d.id === deviceId) || DEVICES[0];
  const keys = {};
  device.keys.forEach((k) => {
    keys[k.id] = k.defaultFunc;
  });
  return {
    name: "Standard",
    deviceId: device.id,
    keys,
    dpiStages: device.dpiDefaults.map((d) => ({ ...d })),
    activeDpi: device.defaultDpiIndex,
    reportRateIndex: device.defaultRateIndex,
    light: {
      mode: device.defaultLight,
      brightness: 100, // wire level 4 — official solid capture 0x42
      speed: 5, // wire level 2
      color: "#155dfc",
    },
    settings: {
      lod: "low", // 1mm → wire 1; high → 2mm wire 2
      angleSnap: false,
      ripple: false,
      debounce: 8,
      highSpeed: false,
      sleepWire: 30, // 5 min (10s units); official dropdown
      moveWake: false,
      moveCloseLight: false,
      mode: "usb",
      lmbLock: true,
    },
    macros: [],
  };
}

function defaultState() {
  const profiles = [defaultProfile("aj179")];
  profiles[0].name = "Profile 1";
  return {
    deviceId: "aj179",
    profileIndex: 0,
    profiles,
    selectedKey: 201,
    selectedMacro: null,
    recording: false,
    battery: null, // null until real HID status read
  };
}

let state = loadState();
let selectedDpiStage = state.profiles[state.profileIndex].activeDpi;

/** Runtime-only battery fields — never restore phantom 100%/charging from disk. */
const BATTERY_RUNTIME_KEYS = [
  "battery",
  "batteryCharging",
  "batteryOnline",
  "batteryIsLast",
  "batterySource",
  "batteryConfidence",
  "batteryDebug",
  "_lastDockFullAt",
  "_batterySeenLive",
];

function clearBatteryRuntime(target = state) {
  target.battery = null;
  target.batteryCharging = false;
  target.batteryOnline = true;
  target.batteryIsLast = false;
  target.batterySource = null;
  target.batteryConfidence = null;
  target.batteryDebug = "";
  target._lastDockFullAt = 0;
  target._batterySeenLive = false;
}

function loadState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultState();
    const parsed = JSON.parse(raw);
    if (!parsed.profiles?.length) return defaultState();
    const merged = { ...defaultState(), ...parsed };
    // Drop any persisted battery snapshot (old builds saved 100% + charging)
    for (const k of BATTERY_RUNTIME_KEYS) delete merged[k];
    clearBatteryRuntime(merged);
    return merged;
  } catch {
    return defaultState();
  }
}

function saveState() {
  const copy = { ...state };
  for (const k of BATTERY_RUNTIME_KEYS) delete copy[k];
  localStorage.setItem(STORAGE_KEY, JSON.stringify(copy));
}

function device() {
  return DEVICES.find((d) => d.id === state.deviceId) || DEVICES[0];
}

function profile() {
  return state.profiles[state.profileIndex];
}

function toast(msg) {
  const el = document.getElementById("toast");
  const exitMs = 220; // match --duration-toast
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

function setStatus(msg) {
  // Footer status bar removed — keep for optional future chrome
  const el = document.getElementById("status-msg");
  if (el) el.textContent = msg;
}

/* ---------- Auto-write to mouse (OEM-style: save on action) ---------- */
const APPLY_DEBOUNCE_MS = 280;
let _applyTimer = null;
let _applyBusy = false;
const _pendingScopes = new Set();

/** Queue a scoped HID write. Always saves locally; writes mouse only when connected. */
function queueDeviceWrite(...scopes) {
  for (const s of scopes) _pendingScopes.add(s);
  if (!hid.connected) return;
  clearTimeout(_applyTimer);
  _applyTimer = setTimeout(() => {
    flushDeviceWrites().catch((e) => toast(e.message || "Couldn't write to mouse"));
  }, APPLY_DEBOUNCE_MS);
}

async function flushDeviceWrites() {
  if (!hid.connected || _applyBusy) return;
  if (!_pendingScopes.size) return;
  _applyBusy = true;
  const scopes = new Set(_pendingScopes);
  _pendingScopes.clear();
  const p = profile();
  const d = device();

  const send = async (buf, opts = {}) => {
    await hid.xfer(buf, {
      timeoutMs: 900,
      retries: 2,
      preferStrip1: true,
      allowNoReply: true,
      ...opts,
    });
    await sleep(30);
  };

  try {
    if (scopes.has("all")) {
      scopes.add("rate");
      scopes.add("dpi");
      scopes.add("light");
      scopes.add("keys");
      scopes.add("sensor");
      scopes.add("power");
    }

    if (scopes.has("rate")) {
      await send(buildReportRate(p.reportRateIndex), { allowNoReply: false });
    }
    if (scopes.has("dpi") || scopes.has("light")) {
      if (scopes.has("dpi")) {
        await send(
          buildDpi(
            p.dpiStages.map((s) => ({ value: s.value })),
            p.activeDpi,
            0
          ),
          { allowNoReply: false }
        );
      }
      await send(
        buildDpiRgb(p.dpiStages.map((s) => s.color || "#ffffff")),
        { exact: true }
      );
    }
    if (scopes.has("light")) {
      await send(buildLightPacketFromUi(p), { exact: true });
    }
    if (scopes.has("keys")) {
      const funcs = keyFuncsInWireOrder(p, d);
      const unknown = funcs.filter((f) => !KEY_FUNC_PROVEN.has(f));
      if (!unknown.length) {
        const kbuf = buildKeyMap(funcs);
        await send(kbuf, { exact: true });
        await send(kbuf, { exact: true });
      }
    }
    if (scopes.has("sensor")) {
      await send(
        buildSensor({
          lod: p.settings.lod === "high" ? 2 : 1,
          angleSnap: p.settings.angleSnap,
          ripple: p.settings.ripple,
        }),
        { allowNoReply: false }
      );
    }
    if (scopes.has("power")) {
      await send(
        buildPower({
          sleepWire: p.settings.sleepWire ?? p.settings.sleepMin,
          moveWake: p.settings.moveWake,
          moveCloseLight: p.settings.moveCloseLight,
          debounce: p.settings.debounce,
        }),
        { allowNoReply: false }
      );
    }
  } finally {
    _applyBusy = false;
    if (_pendingScopes.size && hid.connected) {
      clearTimeout(_applyTimer);
      _applyTimer = setTimeout(() => {
        flushDeviceWrites().catch((e) => toast(e.message || "Couldn't write to mouse"));
      }, 80);
    }
  }
}

/* ---------- Tabs ---------- */
function showTab(name) {
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

function initTabs() {
  const nav = document.getElementById("tabs");
  if (!nav) {
    console.error("Nibble: #tabs nav not found");
    return;
  }

  // Event delegation — survives re-renders and clicks on inner <span>
  nav.addEventListener("click", (e) => {
    const tab = e.target.closest("button.tab, .tab");
    if (!tab || !nav.contains(tab)) return;
    e.preventDefault();
    const name = tab.getAttribute("data-tab") || tab.dataset.tab;
    if (!name) return;
    const panel = document.getElementById(`panel-${name}`);
    if (!panel) {
      console.error(`Nibble: missing panel for tab "${name}"`);
      return;
    }
    showTab(name);
  });
}

/* ---------- Device / profile selects ---------- */
function initDeviceSelect() {
  const sel = document.getElementById("device-select");
  sel.innerHTML = DEVICES.map((d) => `<option value="${d.id}">${d.name}</option>`).join("");
  sel.value = state.deviceId;
  sel.addEventListener("change", () => {
    state.deviceId = sel.value;
    // Prefer a profile for this device, else create one
    let idx = state.profiles.findIndex((p) => p.deviceId === state.deviceId);
    if (idx < 0) {
      const p = defaultProfile(state.deviceId);
      p.name = `Profile ${state.profiles.length + 1}`;
      state.profiles.push(p);
      idx = state.profiles.length - 1;
    }
    state.profileIndex = idx;
    selectedDpiStage = profile().activeDpi;
    state.selectedKey = device().keys[0].id;
    saveState();
    renderAll();
    toast(`Switched to ${device().name}`);
  });
}

function initProfileSelect() {
  const sel = document.getElementById("profile-select");
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
    document.getElementById("device-select").value = state.deviceId;
    selectedDpiStage = profile().activeDpi;
    saveState();
    renderAll();
  });
  // expose for other renders
  initProfileSelect.refresh = render;
}

/* ---------- Connection / battery ---------- */
function renderConnection() {
  const el = document.getElementById("connection-status");
  const text = el.querySelector(".conn-text");
  const btnC = document.getElementById("btn-connect");
  const btnD = document.getElementById("btn-disconnect");
  el.classList.remove("offline", "warn");

  if (hid.connected) {
    const info = hid.info;
    const pid = info.productId.toString(16).toUpperCase().padStart(4, "0");
    const vid = info.vendorId.toString(16).toUpperCase().padStart(4, "0");
    text.textContent = `Connected · ${info.productName || "Mouse"} (${vid}:${pid})`;
    btnC.hidden = true;
    btnD.hidden = false;
    const fw = document.getElementById("fw-ver");
    if (fw) fw.textContent = info.productName || "HID open";
    const vp = document.getElementById("vidpid");
    if (vp) vp.textContent = `${vid}:${pid}`;
  } else {
    el.classList.add("offline");
    text.textContent = webHidSupported() ? "Not connected · click Connect" : "WebHID unavailable";
    btnC.hidden = false;
    btnD.hidden = true;
    const fw = document.getElementById("fw-ver");
    if (fw) fw.textContent = "— not connected —";
    const vp = document.getElementById("vidpid");
    if (vp) vp.textContent = "—";
  }
  const pct = document.getElementById("battery-pct");
  const level = document.getElementById("battery-level");
  const bat = typeof state.battery === "number" ? state.battery : null;
  if (pct) {
    pct.textContent =
      bat != null
        ? `${bat}%${state.batteryIsLast || state.batteryOnline === false ? " (last)" : ""}`
        : "—";
  }
  if (level) level.style.width = bat != null ? `${Math.min(100, Math.max(0, bat))}%` : "0%";
  // Keep home bento battery in sync
  if (document.getElementById("stat-battery")) renderHome();
}

async function connectHid() {
  try {
    setStatus("Select your wireless receiver (not the mouse or keyboard entry)…");
    const info = await hid.requestAndOpen();
    clearBatteryRuntime();
    renderConnection();
    toast("Connected");
    setStatus(
      `Connected · ${info.productName || "mouse"} (${info.vendorId
        .toString(16)
        .toUpperCase()
        .padStart(4, "0")}:${info.productId.toString(16).toUpperCase().padStart(4, "0")})`
    );
    hid.onReport((payload) => {
      const st = parseStatus(payload);
      if (st?.kind === "status") applyBatteryFromStatus(st);
    });

    try {
      const st = await readStatusFromDevice();
      
      // Auto-detect device model based on devId from packet or vid/pid
      let match = null;
      if (st && st.devId) {
        match = DEVICES.find(d => d.modes.some(m => m.devId === st.devId));
      }
      if (!match) {
        const vidHex = info.vendorId.toString(16).toUpperCase().padStart(4, "0");
        const pidHex = info.productId.toString(16).toUpperCase().padStart(4, "0");
        match = DEVICES.find(d => d.modes.some(m => m.vid === vidHex && m.pid === pidHex));
      }
      
      if (match && match.id !== state.deviceId) {
        state.deviceId = match.id;
        let idx = state.profiles.findIndex((p) => p.deviceId === state.deviceId);
        if (idx < 0) {
          const p = defaultProfile(state.deviceId);
          state.profiles.push(p);
          idx = state.profiles.length - 1;
        }
        state.activeProfileIndex = idx;
        const sel = document.getElementById("device-select");
        if (sel) sel.value = state.deviceId;
        renderAll();
      }

      await sleep(50);
      await syncProfileFromDevice();
      startBatteryPoll();
      const bat = typeof state.battery === "number" ? ` · ${state.battery}%` : "";
      toast(`Connected${bat}`);
      setStatus(`Ready${bat}`);
    } catch (e) {
      setStatus(`Connected · could not sync settings (${e.message})`);
    }
  } catch (e) {
    stopBatteryPoll();
    toast(e.message || "Connect failed");
    setStatus(e.message || "Connect failed");
    renderConnection();
  }
}

async function disconnectHid() {
  stopBatteryPoll();
  await hid.close();
  renderConnection();
  toast("Disconnected");
  setStatus("Disconnected");
}

/** Keep Charging briefly after dock 0xFF so UI doesn't flicker. */
const DOCK_CHARGE_STICK_MS = 6000;

let _batteryPollTimer = null;
let _batteryReadBusy = false;

function stopBatteryPoll() {
  if (_batteryPollTimer) {
    clearInterval(_batteryPollTimer);
    _batteryPollTimer = null;
  }
}

function startBatteryPoll() {
  stopBatteryPoll();
  _batteryPollTimer = setInterval(() => {
    if (!hid.connected) {
      stopBatteryPoll();
      return;
    }
    if (_batteryReadBusy) return;
    readStatusFromDevice({ quiet: true }).catch(() => {});
  }, 2000);
}

/**
 * Simple apply (linux-compatible):
 *  - C0/% 1–100 → always show that %
 *  - 0xFF dock → Charging + hold last % (never 100)
 *  - GET low junk (≤20) only ignored if we already have a better last %
 *  - sleep → hold last, not charging
 */
function applyBatteryFromStatus(st) {
  if (!st || st.kind !== "status") return false;
  const now = Date.now();
  const recentDock =
    typeof state._lastDockFullAt === "number" &&
    now - state._lastDockFullAt < DOCK_CHARGE_STICK_MS;
  const flagOn = (st.chargeFlag | 0) > 0;
  const fromC0 = String(st.source || "").startsWith("c0");
  const fromGet = String(st.source || "").startsWith("get");
  const dockMarker =
    st.confidence === "dock-marker" ||
    st.batteryRaw === 0xff ||
    st.source === "short-ff";

  if (st.sleeping || st.online === false) {
    state.batteryOnline = false;
    state.batteryCharging = false;
    state.batteryIsLast = state.battery != null;
    state.batterySource = "sleep";
    state.batteryDebug = st.debug || "";
    state._lastDockFullAt = 0;
    renderConnection();
    return true;
  }

  // Dock 0xFF — charging, keep last known % (do not invent 100)
  if (dockMarker) {
    // Ignore cold GET 0xFF when we have never seen a real % (padding → fake 100% bug)
    if (state.battery == null && fromGet) {
      state.batteryDebug = "ignore cold get-ff";
      return false;
    }
    state._lastDockFullAt = now;
    state.batteryOnline = true;
    state.batteryCharging = true;
    state.batteryIsLast = state.battery != null;
    state.batterySource = state.battery != null ? "dock-last" : "dock-wait";
    state.batteryDebug = st.debug || "dock";
    renderConnection();
    return true;
  }

  const pct =
    typeof st.battery === "number" && st.battery >= 1 && st.battery <= 100
      ? st.battery
      : normalizeBatteryPercent(st.batteryRaw);

  if (pct == null) {
    if (flagOn) {
      state.batteryCharging = true;
      state._lastDockFullAt = now;
      state.batteryOnline = true;
      state.batteryIsLast = state.battery != null;
      state.batterySource = "flag";
      state.batteryDebug = st.debug || "";
      renderConnection();
      return true;
    }
    return false;
  }

  // GET-only junk ≤20: don't replace a better known % (dock noise). C0 always trusted.
  if (
    fromGet &&
    !fromC0 &&
    pct <= 20 &&
    typeof state.battery === "number" &&
    state.battery > 20
  ) {
    if (flagOn || recentDock || state.batteryCharging) {
      state.batteryCharging = true;
      state.batteryIsLast = true;
      state.batteryOnline = true;
      state.batterySource = "dock-hold";
      state.batteryDebug = `hold ${state.battery} vs get ${pct}`;
      renderConnection();
      return true;
    }
    return false;
  }

  // While charging, firmware often reports 100% / "full" — keep last real SOC instead.
  // Some docks send 100% with no charge flag, so catch sudden jumps to 100 as a dock marker.
  const isSuddenJumpTo100 = pct >= 100 && typeof state.battery === "number" && state.battery < 95;
  const chargingNow = flagOn || recentDock || state.batteryCharging || !!st.charging || isSuddenJumpTo100;
  
  if (chargingNow && pct >= 100) {
    state._lastDockFullAt = now;
    state.batteryOnline = true;
    state.batteryCharging = true;
    state.batteryIsLast = state.battery != null;
    state.batterySource = state.battery != null ? "charge-hold" : "charge-wait";
    state.batteryDebug = st.debug || "hold last (no 100% while charging)";
    // do not write state.battery = 100
    renderConnection();
    return true;
  }

  // Accept percent (C0 always; GET when not junk-overwriting)
  state._batterySeenLive = true;
  state.battery = pct;
  state.batteryOnline = true;
  state.batteryIsLast = false;
  if (fromC0 && !flagOn) {
    state.batteryCharging = false;
    state._lastDockFullAt = 0;
  } else {
    state.batteryCharging = flagOn || recentDock;
    if (!state.batteryCharging) state._lastDockFullAt = 0;
  }
  state.batterySource = st.source || "status";
  state.batteryConfidence = st.confidence || "";
  state.batteryDebug = st.debug || "";
  renderConnection();
  return true;
}

/**
 * Poll status + listen for C0. Prefer C0 % over GET.
 */
async function readStatusFromDevice(opts = {}) {
  if (!hid.connected) throw new Error("Not connected");
  if (_batteryReadBusy) return null;
  const quiet = !!opts.quiet;
  _batteryReadBusy = true;

  let bestC0 = null;
  let bestAny = null;
  let lastResp = null;

  const consider = (st) => {
    if (!st || st.kind !== "status") return;
    bestAny = st;
    applyBatteryFromStatus(st);
    if (typeof st.battery === "number" && String(st.source || "").startsWith("c0")) {
      bestC0 = st;
    }
  };

  const prev = hid._onReport;
  hid.onReport((payload) => {
    consider(parseStatus(payload));
    if (typeof prev === "function") prev(payload);
  });

  try {
    for (let attempt = 0; attempt < 4; attempt++) {
      try {
        const resp = await hid.xfer(buildStatusQuery(), {
          timeoutMs: 900,
          retries: 2,
          preferStrip1: true,
        });
        lastResp = resp;
        consider(parseStatus(resp));
        if (bestC0 || (state.battery != null && state._batterySeenLive && !state.batteryCharging))
          break;
      } catch (_) {
        /* retry */
      }
      await sleep(100);
    }

    // C0 can arrive slightly after the GET write
    const t0 = Date.now();
    while (Date.now() - t0 < 800) {
      if (bestC0 || (state.battery != null && state._batterySeenLive)) break;
      await sleep(40);
    }

    if (bestC0) applyBatteryFromStatus(bestC0);
    renderConnection();

    if (!quiet) {
      const pct = state.battery != null ? `${state.battery}%` : "—";
      const ch = state.batteryCharging ? " · charging" : "";
      const sl = state.batteryOnline === false ? " · sleeping" : "";
      setStatus(`Battery ${pct}${ch}${sl}`);
    }
    return bestC0 || bestAny || (lastResp ? parseStatus(lastResp) : null);
  } finally {
    _batteryReadBusy = false;
    hid.onReport((payload) => {
      const st = parseStatus(payload);
      if (st?.kind === "status") applyBatteryFromStatus(st);
      if (typeof prev === "function") prev(payload);
    });
  }
}

/**
 * Pull rate / DPI / light from the mouse (GET 0x12 / 0x13 / 0x15) into the profile
 * so Home shows device state, not stale localStorage defaults.
 */
async function syncProfileFromDevice() {
  if (!hid.connected) throw new Error("Not connected");
  const p = profile();
  const d = device();
  const log = [];
  const xferGet = async (buf) => {
    const got = await hid.xfer(buf, {
      timeoutMs: 900,
      retries: 2,
      preferStrip1: true,
    });
    return got;
  };

  // Report rate
  try {
    const raw = await xferGet(buildReportRateGet());
    const rr = parseReportRateResponse(raw);
    if (rr && rr.rateIndex >= 0 && rr.rateIndex < d.reportRates.length) {
      p.reportRateIndex = rr.rateIndex;
      log.push(`${d.reportRates[rr.rateIndex]} Hz`);
    }
  } catch (e) {
    console.log("Error getting report rate:", e);
  }
  await sleep(40);

  // DPI table + active stage
  try {
    const raw = await xferGet(buildDpiGet());
    const dpi = parseDpiResponse(raw, 0);
    if (dpi?.stages?.length) {
      const n = Math.min(6, dpi.stages.length);
      for (let i = 0; i < n; i++) {
        if (!p.dpiStages[i]) p.dpiStages[i] = { value: 800, color: "#ffffff" };
        p.dpiStages[i].value = dpi.stages[i].value;
      }
      p.activeDpi = Math.min(dpi.activeIndex, p.dpiStages.length - 1);
      log.push(`${p.dpiStages[p.activeDpi].value} DPI`);
    }
  } catch {
    /* optional */
  }
  await sleep(40);

  // Lighting
  try {
    const raw = await xferGet(buildLightGet());
    const lt = parseLightResponse(raw);
    if (lt) {
      const id = wireToLightId(lt.mode);
      // Prefer meta-based mode when wire mode looks solid/breathe
      let modeId = id;
      if (lt.meta === 0x05) modeId = "solid";
      else if (lt.meta === 0x18) modeId = "breathe";
      else if (lt.meta === 0x02 && lt.mode === 0) modeId = "off";
      else if (lt.mode === 3) modeId = "solid";
      else if (lt.mode === 2) modeId = "breathe";
      else if (lt.mode === 0) modeId = "off";

      p.light.mode = modeId;
      // wire bri 0–4 → UI percent
      p.light.brightness = Math.round(((lt.brightness || 0) / 4) * 100);
      p.light.speed = Math.max(1, Math.min(10, Math.round(1 + ((lt.speed || 2) / 4) * 9)));
      if (lt.color && lt.color !== "#000000") p.light.color = lt.color;
      log.push(modeId);
    }
  } catch {
    /* optional */
  }

  saveState();
  renderHome();
  renderDpi();
  renderLight();
  renderConnection();
  setStatus(log.length ? `Synced · ${log.join(" · ")}` : "Synced");
  return log;
}

/** Full write (used after connect + factory-style resets). Quiet unless error. */
async function applyToMouse() {
  if (!hid.connected) return;
  _pendingScopes.add("all");
  await flushDeviceWrites();
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

/**
 * Observed behavior (AJ179-class):
 * - Constant light has NO independent color in the driver UI.
 * - LED color in constant mode = active DPI stage color (cmd 0x04 table).
 * - Light cmd 0x05 sets mode + brightness; solid capture still carries RGB but
 *   hardware follows the DPI color table when in constant mode.
 */
function buildLightPacketFromUi(p) {
  const bri = uiBrightnessToWire(p.light.brightness);
  const spd = uiSpeedToWire(p.light.speed);
  // Prefer active DPI color for solid (matches official)
  const dpiColor = p.dpiStages[p.activeDpi]?.color || p.light.color || "#ff0000";
  const color = p.light.color || dpiColor;
  if (p.light.mode === "solid") {
    return buildSolidCapture({
      brightness: bri,
      speed: spd,
      color: dpiColor,
      allowZero: true,
    });
  }
  if (p.light.mode === "off") {
    return buildOffCapture();
  }
  if (p.light.mode === "breathe") {
    return buildBreatheCapture({
      brightness: bri,
      speed: spd,
      color,
      useRainbow: p.light.breatheRainbow !== false && !p.light.color,
      allowZero: true,
    });
  }
  return buildLight(lightIdToWire(p.light.mode), {
    brightness: bri,
    speed: spd,
    color: dpiColor,
    off: p.light.mode === "off",
    useRainbow: false,
    allowZero: true,
  });
}

async function applyLightOnly() {
  queueDeviceWrite("light");
  await flushDeviceWrites();
}

/* ---------- Home ---------- */
function renderHome() {
  const d = device();
  const p = profile();
  const dpi = p.dpiStages[p.activeDpi] || p.dpiStages[0] || { value: "—", color: "#888" };
  const rate = d.reportRates[p.reportRateIndex] ?? "—";
  const light = d.lights.find((l) => l.id === p.light.mode);
  const dpiCol = dpi.color || "#888888";
  const ledColor = p.light.mode === "solid" ? dpiCol : p.light.color || dpiCol;

  const setTxt = (id, t) => {
    const el = document.getElementById(id);
    if (el) el.textContent = t;
  };

  const mouseImg = document.getElementById("mouse-image");
  if (mouseImg) mouseImg.src = d.image;
  const keysImg = document.getElementById("keys-mouse-image");
  if (keysImg) keysImg.src = d.image;
  const lightImg = document.getElementById("light-mouse-image");
  if (lightImg) lightImg.src = d.image;

  setTxt("home-device-name", d.name);
  setTxt("home-device-sub", d.sensor ? `${d.sensor} sensor` : "Gaming mouse");

  setTxt("stat-dpi", String(dpi.value));
  setTxt("stat-dpi-color", `Stage ${p.activeDpi + 1}`);
  const dpiSw = document.getElementById("stat-dpi-swatch");
  if (dpiSw) dpiSw.style.background = dpiCol;

  setTxt("stat-rate", String(rate));

  setTxt("stat-light", light?.name || p.light.mode);
  const lightSub =
    p.light.mode === "off"
      ? "Off"
      : p.light.mode === "solid"
        ? `Stage ${p.activeDpi + 1} color · ${p.light.brightness ?? 0}%`
        : `${p.light.brightness ?? 0}% brightness`;
  setTxt("stat-light-color", lightSub);
  const lightSw = document.getElementById("stat-light-swatch");
  if (lightSw) {
    lightSw.style.background = p.light.mode === "off" ? "#444" : ledColor;
    lightSw.style.opacity = p.light.mode === "off" ? "0.35" : "1";
  }

  setTxt("stat-mode", { usb: "USB", "24g": "2.4G" }[p.settings.mode] || "USB");
  setTxt("stat-link", hid.connected ? "Connected" : "Not connected");

  const bat = state.battery;
  const batTile = document.querySelector(".bento-battery");
  if (batTile) batTile.classList.toggle("is-charging", !!state.batteryCharging);
  if (hid.connected && typeof bat === "number" && bat >= 0) {
    const held = !!(state.batteryIsLast || state.batteryOnline === false);
    setTxt("stat-battery", `${Math.min(100, bat)}%${held ? " (last)" : ""}`);
    let sub = "Discharging";
    if (state.batteryOnline === false) sub = "Sleeping";
    else if (state.batteryCharging) sub = "Charging";
    else if (bat <= 20) sub = "Low";
    setTxt("stat-battery-sub", sub);
  } else {
    setTxt("stat-battery", "—");
    let sub = "Connect to read";
    if (hid.connected) {
      if (state.batteryCharging) sub = "Charging";
      else if (state.batteryOnline === false) sub = "Sleeping";
      else sub = "Unavailable";
    }
    setTxt("stat-battery-sub", sub);
  }
  updateLightPreview();
}

/* ---------- Keys ---------- */

/** Profile keys → wire order (key_value 0..5). */
function keyFuncsInWireOrder(p, d = device()) {
  const byKv = new Map();
  d.keys.forEach((k) => {
    byKv.set(k.keyValue, p.keys[k.id] || k.defaultFunc);
  });
  return [0, 1, 2, 3, 4, 5].map((kv) => byKv.get(kv) || "disable");
}

function renderKeys() {
  const d = device();
  const p = profile();
  const host = document.getElementById("key-hotspots");
  host.innerHTML = "";
  d.keys.forEach((k) => {
    const func = p.keys[k.id] || k.defaultFunc;
    const div = document.createElement("div");
    div.className = `hotspot ${k.direction || "left"}${
      state.selectedKey === k.id ? " active" : ""
    }`;
    div.style.left = `${k.x}%`;
    div.style.top = `${k.y}%`;
    div.innerHTML = `<span class="pin"></span><span class="label">${KEY_LABELS[k.id]} · ${
      FUNC_LABEL[func] || func
    }</span>`;
    div.addEventListener("click", () => {
      state.selectedKey = k.id;
      renderKeys();
    });
    host.appendChild(div);
  });

  const keyMeta = d.keys.find((k) => k.id === state.selectedKey) || d.keys[0];
  const current = p.keys[keyMeta.id];
  document.getElementById("selected-key-label").textContent = `${
    KEY_LABELS[keyMeta.id]
  } → ${FUNC_LABEL[current] || current}`;

  const groups = document.getElementById("func-groups");
  groups.innerHTML = "";
  FUNC_GROUPS.forEach((g) => {
    const wrap = document.createElement("div");
    wrap.className = "func-group";
    wrap.innerHTML = `<h4>${g.title}</h4>`;
    const opts = document.createElement("div");
    opts.className = "func-options";
    g.items.forEach((item) => {
      const btn = document.createElement("button");
      btn.type = "button";
      const proven = KEY_FUNC_PROVEN.has(item.id);
      btn.textContent = item.label;
      btn.title = proven ? item.label : "Not available yet";
      if (!proven) btn.classList.add("unproven");
      if (item.id === current) btn.classList.add("active");
      btn.addEventListener("click", () => assignKey(keyMeta, item.id));
      opts.appendChild(btn);
    });
    wrap.appendChild(opts);
    groups.appendChild(wrap);
  });
}

function assignKey(keyMeta, funcId) {
  const p = profile();
  if (keyMeta.defaultFunc === "left" && p.settings.lmbLock && funcId !== "left") {
    toast("Left-click is locked. Unlock it in Settings first.");
    return;
  }
  if (keyMeta.defaultFunc === "left" && funcId !== "left") {
    if (!confirm("Changing left-click may make UI hard to use. Continue?")) return;
  }
  if (!KEY_FUNC_PROVEN.has(funcId)) {
    toast("That function isn’t available yet — pick another");
    return;
  }
  p.keys[keyMeta.id] = funcId;
  saveState();
  renderKeys();
  queueDeviceWrite("keys");
}

/** Write full 6-key table (cmd 0x09). */
async function applyKeysOnly() {
  queueDeviceWrite("keys");
  await flushDeviceWrites();
}

/* ---------- DPI ---------- */
function renderDpi() {
  const d = device();
  const p = profile();
  const list = document.getElementById("dpi-stages");
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
      selectedDpiStage = i;
      p.activeDpi = i;
      saveState();
      renderDpi();
      renderHome();
      queueDeviceWrite("dpi", "light");
    });
    list.appendChild(row);
  });

  const stage = p.dpiStages[selectedDpiStage];
  const slider = document.getElementById("dpi-slider");
  const input = document.getElementById("dpi-input");
  const color = document.getElementById("dpi-color");
  slider.value = stage.value;
  input.value = stage.value;
  color.value = toHex(stage.color);

  const rates = document.getElementById("rate-options");
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

function bindDpiEditors() {
  const apply = (val) => {
    const p = profile();
    const n = Math.min(26000, Math.max(50, Math.round(Number(val) / 50) * 50));
    p.dpiStages[selectedDpiStage].value = n;
    document.getElementById("dpi-slider").value = n;
    document.getElementById("dpi-input").value = n;
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("dpi");
  };
  document.getElementById("dpi-slider").addEventListener("input", (e) => apply(e.target.value));
  document.getElementById("dpi-input").addEventListener("change", (e) => apply(e.target.value));
  document.getElementById("dpi-color").addEventListener("input", (e) => {
    profile().dpiStages[selectedDpiStage].color = e.target.value;
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("dpi", "light");
  });
}

function toHex(c) {
  if (!c) return "#ffffff";
  if (c.startsWith("#") && c.length === 7) return c;
  return c;
}

/* ---------- Lighting ---------- */
function renderLight() {
  const d = device();
  const p = profile();
  const grid = document.getElementById("effect-grid");
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
  document.getElementById("light-brightness").value = p.light.brightness;
  document.getElementById("light-brightness-val").textContent = `${p.light.brightness}%`;
  document.getElementById("light-speed").value = p.light.speed;
  document.getElementById("light-speed-val").textContent = p.light.speed;

  const colorRow = document.getElementById("light-color-row");
  const speedRow = document.getElementById("light-speed-row");
  const note = document.getElementById("light-mode-note");
  const colorInput = document.getElementById("light-color");
  const dpiCol = p.dpiStages[p.activeDpi]?.color || "#ff0000";

  if (p.light.mode === "solid") {
    // Constant mode: color from active DPI stage (cmd 0x04)
    p.light.color = dpiCol;
    if (colorInput) {
      colorInput.value = toHex(dpiCol);
      colorInput.disabled = true;
    }
    if (colorRow) colorRow.style.opacity = "0.55";
    if (speedRow) speedRow.style.display = "none";
    if (note) {
      note.textContent =
        "Constant mode has no separate color (same as official app). LED color = active DPI stage color — change it on the DPI tab.";
    }
  } else if (p.light.mode === "off") {
    if (colorInput) colorInput.disabled = true;
    if (colorRow) colorRow.style.opacity = "0.4";
    if (speedRow) speedRow.style.display = "none";
    if (note) note.textContent = "Lighting off.";
  } else if (p.light.mode === "breathe") {
    if (colorInput) {
      colorInput.disabled = false;
      colorInput.value = toHex(p.light.color || dpiCol);
    }
    if (colorRow) colorRow.style.opacity = "1";
    if (speedRow) speedRow.style.display = "";
    if (note) {
      note.textContent =
        "Breathing · optional single color, or leave default. Speed & brightness apply.";
    }
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

function bindLightEditors() {
  document.getElementById("light-brightness").addEventListener("input", (e) => {
    profile().light.brightness = Number(e.target.value);
    document.getElementById("light-brightness-val").textContent = `${e.target.value}%`;
    saveState();
    updateLightPreview();
    renderHome();
    queueDeviceWrite("light");
  });
  document.getElementById("light-speed").addEventListener("input", (e) => {
    profile().light.speed = Number(e.target.value);
    document.getElementById("light-speed-val").textContent = e.target.value;
    saveState();
    updateLightPreview();
    queueDeviceWrite("light");
  });
  document.getElementById("light-color").addEventListener("input", (e) => {
    profile().light.color = e.target.value;
    saveState();
    updateLightPreview();
    renderHome();
    queueDeviceWrite("light");
  });
}

function updateLightPreview() {
  const p = profile();
  const orb = document.getElementById("light-orb");
  const stage = document.getElementById("stage-glow");
  if (!orb && !stage) return;
  const dpiCol = p.dpiStages[p.activeDpi]?.color || "#ff0000";
  // Constant mode: LED follows DPI stage color (official behavior)
  const color =
    p.light.mode === "off"
      ? "#000000"
      : p.light.mode === "solid"
        ? dpiCol
        : p.light.color || dpiCol;
  const bright = p.light.brightness / 100;

  // Static preview only — no idle CSS loops on a functional configurator
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

  // Breathe/flow: slightly softer static glow so mode is still legible without looping
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

function hexToRgba(hex, a) {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return `rgba(${r},${g},${b},${a})`;
}

/* ---------- Macros (not planned — static panel only) ---------- */
function renderMacros() {
  /* Panel is static HTML: "Not planned". No editor / no HID. */
}

function bindMacros() {
  /* no-op */
}

/* ---------- Settings ---------- */
function renderSettings() {
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
    // Migrate legacy sleepMin (minutes) → sleepWire (10s units)
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
}

function setSegmented(id, value) {
  document.querySelectorAll(`#${id} button`).forEach((b) => {
    b.classList.toggle("active", b.dataset.value === value);
  });
}

function bindSettings() {
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
  bindSeg("mode-options", "mode", null); // UI-only display

  // AJ179 is cable + 2.4G only — drop stale bluetooth profile values
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
  bindCheck("opt-highspeed", "highSpeed", null); // local only
  bindCheck("opt-move-wake", "moveWake", "power");
  bindCheck("opt-move-light", "moveCloseLight", "power");
  bindCheck("opt-lmb-lock", "lmbLock", null); // local only

  document.getElementById("opt-debounce").addEventListener("change", (e) => {
    profile().settings.debounce = Number(e.target.value) || 0;
    saveState();
    queueDeviceWrite("power");
  });
  document.getElementById("opt-sleep").addEventListener("change", (e) => {
    profile().settings.sleepWire = Number(e.target.value) || 30;
    delete profile().settings.sleepMin;
    saveState();
    queueDeviceWrite("power");
  });
}

/* ---------- Actions ---------- */
function bindActions() {
  document.getElementById("btn-reset-keys").addEventListener("click", () => {
    if (!confirm("Reset all button configuration data?")) return;
    const d = device();
    const p = profile();
    d.keys.forEach((k) => {
      p.keys[k.id] = k.defaultFunc;
    });
    saveState();
    renderKeys();
    queueDeviceWrite("keys");
    toast("Keys reset");
  });

  document.getElementById("btn-reset-dpi").addEventListener("click", () => {
    if (!confirm("Reset DPI stages to defaults?")) return;
    const d = device();
    const p = profile();
    p.dpiStages = d.dpiDefaults.map((x) => ({ ...x }));
    p.activeDpi = d.defaultDpiIndex;
    p.reportRateIndex = d.defaultRateIndex;
    selectedDpiStage = p.activeDpi;
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("rate", "dpi", "light");
    toast("DPI reset");
  });

  document.getElementById("btn-reset-light").addEventListener("click", () => {
    if (!confirm("Reset lighting configuration?")) return;
    const d = device();
    profile().light = {
      mode: d.defaultLight,
      brightness: 100,
      speed: 5,
      color: "#155dfc",
    };
    saveState();
    renderLight();
    renderHome();
    queueDeviceWrite("light");
    toast("Lighting reset");
  });

  document.getElementById("btn-factory").addEventListener("click", () => {
    if (!confirm("This will delete all saved profiles. Continue?")) return;
    localStorage.removeItem(STORAGE_KEY);
    state = defaultState();
    selectedDpiStage = 1;
    renderAll();
    if (hid.connected) queueDeviceWrite("all");
    toast("Factory settings restored");
  });

  document.getElementById("btn-connect")?.addEventListener("click", () => connectHid());
  document.getElementById("btn-disconnect")?.addEventListener("click", () => disconnectHid());

  document.getElementById("btn-export-profile")?.addEventListener("click", () => {
    const blob = new Blob(
      [JSON.stringify({ app: APP_NAME, version: APP_VERSION, profile: profile() }, null, 2)],
      { type: "application/json" }
    );
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `${profile().name.replace(/\s+/g, "_")}_${device().id}.json`;
    a.click();
    URL.revokeObjectURL(a.href);
    toast("Profile exported");
  });

  document.getElementById("btn-import-profile")?.addEventListener("click", () => {
    document.getElementById("import-file")?.click();
  });
  document.getElementById("import-file")?.addEventListener("change", async (e) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const data = JSON.parse(await file.text());
      const p = data.profile || data;
      if (!p.keys || !p.dpiStages) throw new Error("Invalid profile");
      state.profiles.push({
        ...defaultProfile(p.deviceId || state.deviceId),
        ...p,
        name: p.name || "Imported",
      });
      state.profileIndex = state.profiles.length - 1;
      state.deviceId = profile().deviceId;
      saveState();
      renderAll();
      if (hid.connected) queueDeviceWrite("all");
      toast("Profile imported");
    } catch {
      toast("Could not import profile");
    }
    e.target.value = "";
  });
}

/* ---------- Render all ---------- */
function renderAll() {
  document.getElementById("device-select").value = state.deviceId;
  if (initProfileSelect.refresh) initProfileSelect.refresh();
  renderConnection();
  renderHome();
  renderKeys();
  renderDpi();
  renderLight();
  renderMacros();
  renderSettings();
}

function init() {
  // Tabs first so navigation works even if later setup throws
  try {
    initTabs();
  } catch (e) {
    console.error("Nibble: initTabs failed", e);
  }
  try {
    initTheme();
  } catch (e) {
    console.error("Nibble: initTheme failed", e);
  }

  try {
    initDeviceSelect();
    initProfileSelect();
    bindDpiEditors();
    bindLightEditors();
    bindMacros();
    bindSettings();
    bindActions();
    renderAll();
  } catch (e) {
    console.error("Nibble: init failed", e);
    setStatus("Something went wrong loading the app.");
  }

  if (!webHidSupported()) {
    const btn = document.getElementById("btn-connect");
    if (btn) {
      btn.disabled = true;
      btn.title = "WebHID needs Chrome or Edge";
    }
  }

  // Re-open previously permitted device (no chooser) if available
  hid.tryReopenGranted().then((ok) => {
    if (ok) {
      renderConnection();
      setStatus("Reconnected");
    }
  }).catch(() => {});
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
