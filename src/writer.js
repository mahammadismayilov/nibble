import {
  buildReportRate,
  buildDpi,
  buildDpiRgb,
  buildLight,
  buildSolidCapture,
  buildOffCapture,
  buildBreatheCapture,
  buildSensor,
  buildPower,
  buildKeyMap,
  buildStatusQuery,
  KEY_FUNC_PROVEN,
  lightIdToWire,
  wireToLightId,
  parseStatus,
  normalizeBatteryPercent,
  uiBrightnessToWire,
  uiSpeedToWire,
  buildReportRateGet,
  parseReportRateResponse,
  buildDpiGet,
  parseDpiResponse,
  parseLightResponse,
  buildSensorGet,
  parseSensorResponse,
  buildKeyMapGet,
  parseKeyMapResponse,
  buildLightGet,
} from "../protocol.js";

import { sendHardwareTelemetry } from "../telemetry.js";
import { profileRegistry } from "../profiles/registry.js";
import { driverRegistry } from "../drivers/registry.js";
import { hid } from "./hidInstance.js";
import {
  state,
  selectedDpiStage,
  setSelectedDpiStage,
  saveState,
  device,
  profile,
  toast,
  setStatus,
  clearBatteryRuntime,
} from "./state.js";
import { DEVICES, defaultProfile } from "./constants.js";
import { renderConnection, renderAll, renderDpi, renderHome, renderLight, renderSensor, renderSettings, renderKeys } from "./ui/navigation.js";

const APPLY_DEBOUNCE_MS = 280;
let _applyTimer = null;
let _applyBusy = false;
const _pendingScopes = new Set();

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

export function keyFuncsInWireOrder(p, d = device()) {
  const byKv = new Map();
  d.keys.forEach((k) => {
    byKv.set(k.keyValue, p.keys[k.id] || k.defaultFunc);
  });
  return [0, 1, 2, 3, 4, 5].map((kv) => byKv.get(kv) || "disable");
}

export function buildLightPacketFromUi(p) {
  const bri = uiBrightnessToWire(p.light.brightness);
  const spd = uiSpeedToWire(p.light.speed);
  const dpiColor = p.dpiStages[p.activeDpi]?.color || "#ff0000";
  const activeColor = (p.settings?.batteryRgbSync && p.light.color) ? p.light.color : dpiColor;

  if (p.light.mode === "solid") {
    return buildSolidCapture({
      brightness: bri,
      speed: spd,
      color: activeColor,
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
      color: activeColor,
      useRainbow: false,
      allowZero: true,
    });
  }
  return buildLight(lightIdToWire(p.light.mode), {
    brightness: bri,
    speed: spd,
    color: p.settings?.batteryRgbSync && p.light.color ? p.light.color : (p.light.color || activeColor),
    off: p.light.mode === "off",
    useRainbow: false,
    allowZero: true,
  });
}

/** Queue a scoped HID write. Always saves locally; writes mouse only when connected. */
export function queueDeviceWrite(...scopes) {
  for (const s of scopes) _pendingScopes.add(s);
  if (!hid.connected) return;
  clearTimeout(_applyTimer);
  _applyTimer = setTimeout(() => {
    flushDeviceWrites().catch((e) => toast(e.message || "Couldn't write to mouse"));
  }, APPLY_DEBOUNCE_MS);
}

export async function flushDeviceWrites() {
  if (!hid.connected || _applyBusy) return;
  if (!_pendingScopes.size) return;
  _applyBusy = true;
  const scopes = new Set(_pendingScopes);
  _pendingScopes.clear();
  const p = profile();
  const d = device();
  const activeDriver = driverRegistry.getDriverForDevice(d) || driverRegistry.getDriverForVendor(hid.info?.vendorId);

  const send = async (buf, opts = {}, scope = "general") => {
    const xferOpts = activeDriver
      ? activeDriver.getTransferOptions(scope, opts)
      : {
          timeoutMs: 900,
          retries: 2,
          preferStrip1: true,
          allowNoReply: true,
          ...opts,
        };
    await hid.xfer(buf, xferOpts);
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
      await send(buildReportRate(p.reportRateIndex), {}, "rate");
    }
    if (scopes.has("dpi") || scopes.has("light")) {
      if (scopes.has("dpi")) {
        await send(
          buildDpi(
            p.dpiStages.map((s) => ({ value: s.value })),
            p.activeDpi,
            0
          ),
          {},
          "dpi"
        );
      }
      await send(
        buildDpiRgb(p.dpiStages.map((s) => s.color || "#ffffff")),
        { exact: true },
        "dpiRgb"
      );
    }
    if (scopes.has("light")) {
      await send(buildLightPacketFromUi(p), { exact: true }, "light");
    }
    if (scopes.has("keys")) {
      const funcs = keyFuncsInWireOrder(p, d);
      const unknown = funcs.filter((f) => !KEY_FUNC_PROVEN.has(f));
      if (!unknown.length) {
        const kbuf = buildKeyMap(funcs);
        await send(kbuf, { exact: true }, "keys");
        await send(kbuf, { exact: true }, "keys");
      }
    }
    if (scopes.has("sensor")) {
      await send(
        buildSensor({
          lod: p.settings.lod === "high" ? 2 : 1,
          angleSnap: p.settings.angleSnap,
          ripple: p.settings.ripple,
        }),
        {},
        "sensor"
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
        {},
        "power"
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

export async function applyLightOnly() {
  queueDeviceWrite("light");
  await flushDeviceWrites();
}

const DOCK_CHARGE_STICK_MS = 6000;
let _batteryPollTimer = null;
let _batteryReadBusy = false;

export function stopBatteryPoll() {
  if (_batteryPollTimer) {
    clearInterval(_batteryPollTimer);
    _batteryPollTimer = null;
  }
}

export function startBatteryPoll() {
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

export function evaluateBatteryRgbSync(force = false) {
  const p = profile();
  if (!p) return;
  const pct = state.battery;

  if (p.settings?.batteryRgbSync && !state.batteryCharging) {
    const val = typeof pct === "number" ? pct : 100;
    let tier = "green";
    let hex = "#00ff00";
    if (val <= 50 && val > 25) {
      tier = "yellow";
      hex = "#ffff00";
    } else if (val <= 25) {
      tier = "red";
      hex = "#ff0000";
    }

    if (force || state._currentBatteryRgbTier !== tier) {
      state._currentBatteryRgbTier = tier;
      p.dpiStages.forEach((s) => {
        if (!s.userColor) s.userColor = s.color || "#ffffff";
      });
      if (p.dpiStages[p.activeDpi]) {
        p.dpiStages[p.activeDpi].color = hex;
      }
      p.light.color = hex;
      saveState();
      renderDpi();
      renderHome();
      renderLight();
      queueDeviceWrite("dpi", "light");
    }
  } else {
    if (state._currentBatteryRgbTier !== null || force) {
      state._currentBatteryRgbTier = null;
      let changed = false;
      p.dpiStages.forEach((s) => {
        if (s.userColor && s.color !== s.userColor) {
          s.color = s.userColor;
          changed = true;
        }
      });
      if (changed || force) {
        saveState();
        renderDpi();
        renderHome();
        renderLight();
        queueDeviceWrite("dpi", "light");
      }
    }
  }
}

export function applyBatteryFromStatus(st) {
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

  if (dockMarker) {
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
    }
    renderConnection();
    return false;
  }

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

  const isSuddenJumpTo100 = pct >= 100 && typeof state.battery === "number" && state.battery < 95;
  const chargingNow = flagOn || recentDock || state.batteryCharging || !!st.charging || isSuddenJumpTo100;
  
  if (chargingNow && pct >= 100) {
    state._lastDockFullAt = now;
    state.batteryOnline = true;
    state.batteryCharging = true;
    state.batteryIsLast = state.battery != null;
    state.batterySource = state.battery != null ? "charge-hold" : "charge-wait";
    state.batteryDebug = st.debug || "hold last (no 100% while charging)";
    renderConnection();
    return true;
  }

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

  const p = profile();
  if (p?.settings?.batteryRgbSync && typeof pct === "number" && !state.batteryCharging) {
    let tier = "green";
    let hex = "#00ff00";
    if (pct <= 50 && pct > 25) {
      tier = "yellow";
      hex = "#ffff00";
    } else if (pct <= 25) {
      tier = "red";
      hex = "#ff0000";
    }
    
    if (state._currentBatteryRgbTier !== tier) {
      state._currentBatteryRgbTier = tier;
      if (p.light.mode !== "off") {
        p.light.color = hex;
        saveState();
        renderLight();
        queueDeviceWrite("light");
      }
    }
  } else if (state.batteryCharging) {
    state._currentBatteryRgbTier = null;
  }

  if (p?.settings?.lowBatteryWarn && pct <= 15 && !state.batteryCharging && !state._lowBatteryWarned) {
    state._lowBatteryWarned = true;
    p.light.mode = "breathe";
    p.light.color = "#ff0000";
    saveState();
    renderLight();
    queueDeviceWrite("light");
    toast("Low battery warning triggered (Blinking Red)");
  } else if (state.batteryCharging) {
    state._lowBatteryWarned = false;
  }

  renderConnection();
  return true;
}

export async function readStatusFromDevice(opts = {}) {
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
      toast(`Status · ${pct}${ch}${sl}`);
    }
    return bestC0 || bestAny || (lastResp ? parseStatus(lastResp) : null);
  } finally {
    _batteryReadBusy = false;
    hid.onReport(prev);
  }
}

export async function connectHid(pickerChosen = false) {
  try {
    let info = null;
    if (pickerChosen) {
      info = hid.info;
    } else {
      setStatus("Select your wireless receiver (not the mouse or keyboard entry)…");
      info = await hid.requestAndOpen();
    }
    
    clearBatteryRuntime();
    renderConnection();
    toast("Connected");

    if (info?.vendorId && info?.productId) {
      const vHex = `0x${info.vendorId.toString(16).toUpperCase().padStart(4, "0")}`;
      const pHex = `0x${info.productId.toString(16).toUpperCase().padStart(4, "0")}`;
      const match = profileRegistry?.findProfile?.(info.vendorId, info.productId);
      sendHardwareTelemetry(vHex, pHex, info.productName || "HID Mouse", !!match);
    }
    setStatus(
      `Connected · ${info.productName || "mouse"} (${info.vendorId
        .toString(16)
        .toUpperCase()
        .padStart(4, "0")}:${info.productId.toString(16).toUpperCase().padStart(4, "0")})`
    );
    hid.onReport((payload) => {
      const u8 = payload instanceof Uint8Array ? payload : new Uint8Array(payload);
      const st = parseStatus(u8);
      if (st?.kind === "status") applyBatteryFromStatus(st);

      const isC2 = (u8.length >= 4) && (u8[0] === 0xc2 || (u8[0] === 0x00 && u8[1] === 0xc2));
      
      if (isC2) {
        const offset = u8[0] === 0x00 ? 1 : 0;
        const packed = u8[offset + 1];
        
        if ((packed & 0x0f) === 0x06) {
          const activeIndex = (packed >> 4) & 0x0f;
          const p = profile();
          
          if (typeof activeIndex === "number" && activeIndex >= 0 && activeIndex < p.dpiStages.length) {
            if (p.activeDpi !== activeIndex) {
              p.activeDpi = activeIndex;
              setSelectedDpiStage(activeIndex);
              saveState();
              renderDpi();
              renderHome();
            }
          }
        }
      }
    });

    try {
      const st = await readStatusFromDevice();
      
      let match = null;
      if (st && st.devId) {
        match = DEVICES.find(d => d.modes.some(m => m.devId === st.devId));
      }
      
      const currentDev = device();
      const vidHex = info.vendorId.toString(16).toUpperCase().padStart(4, "0");
      const pidHex = info.productId.toString(16).toUpperCase().padStart(4, "0");
      const currentMatchesVidPid = currentDev.modes.some(m => m.vid === vidHex && m.pid === pidHex);

      if (!match && !currentMatchesVidPid) {
        match = DEVICES.find(d => d.modes.some(m => m.vid === vidHex && m.pid === pidHex));
      }
      
      if (match && match.id !== state.deviceId) {
        state.deviceId = match.id;
        let idx = state.profiles.findIndex((p) => p.deviceId === state.deviceId);
        if (idx < 0) {
          const p = defaultProfile(state.deviceId);
          p.name = `Profile ${state.profiles.length + 1}`;
          state.profiles.push(p);
          idx = state.profiles.length - 1;
        }
        state.profileIndex = idx;
        saveState();
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
      const landing = document.getElementById("landing-screen");
      const main = document.getElementById("main-app");
      if (landing && main) {
        landing.style.display = "none";
        main.style.display = "";
      }
    } catch (err) {
      toast("Connected (status sync skipped)");
      setStatus("Connected");
    }
  } catch (err) {
    if (err.name === "NotFoundError" || err.message?.includes("User cancelled")) {
      setStatus("Device picker cancelled");
      toast("Cancelled");
      return;
    }
    toast(`Connection failed: ${err.message || err}`);
    setStatus(`Connection error: ${err.message || err}`);
  }
}

export async function disconnectHid() {
  stopBatteryPoll();
  await hid.close();
  renderConnection();
  toast("Disconnected");
  setStatus("Disconnected");
}

export async function syncProfileFromDevice() {
  if (!hid.connected) throw new Error("Not connected");
  const p = profile();
  const log = [];

  const xferGet = async (buf) => {
    return hid.xfer(buf, {
      timeoutMs: 900,
      retries: 2,
      preferStrip1: true,
    });
  };

  try {
    const raw = await xferGet(buildReportRateGet());
    const rateIdx = parseReportRateResponse(raw);
    if (rateIdx != null && rateIdx >= 0) {
      p.reportRateIndex = rateIdx;
      log.push(`Rate: ${device().reportRates[rateIdx] || rateIdx}Hz`);
    }
  } catch {
    /* optional */
  }
  await sleep(40);

  try {
    const raw = await xferGet(buildDpiGet());
    const dpiInfo = parseDpiResponse(raw);
    if (dpiInfo) {
      if (typeof dpiInfo.activeIndex === "number" && dpiInfo.activeIndex < p.dpiStages.length) {
        p.activeDpi = dpiInfo.activeIndex;
        setSelectedDpiStage(dpiInfo.activeIndex);
      }
      log.push(`DPI: ${p.dpiStages[p.activeDpi]?.value || "ok"}`);
    }
  } catch {
    /* optional */
  }
  await sleep(40);

  try {
    const raw = await xferGet(buildKeyMapGet());
    const keys = parseKeyMapResponse(raw);
    if (keys && Object.keys(keys).length > 0) {
      const vals = Object.values(keys);
      if (!(vals.length > 0 && vals.every((v) => v === "disable"))) {
        p.keys = { ...p.keys, ...keys };
        log.push("Keys synced");
      }
    }
  } catch {
    /* optional */
  }
  await sleep(40);

  try {
    const raw = await xferGet(buildSensorGet());
    const sensor = parseSensorResponse(raw);
    if (sensor) {
      p.settings = { ...p.settings, ...sensor };
      log.push("Sensor synced");
    }
  } catch {
    /* optional */
  }
  await sleep(40);

  try {
    const raw = await xferGet(buildLightGet());
    const lt = parseLightResponse(raw);
    if (lt) {
      let modeId = wireToLightId(lt.mode);
      if (lt.meta === 0x05) modeId = "solid";
      else if (lt.meta === 0x18) modeId = "breathe";
      else if (lt.meta === 0x02 && lt.mode === 0) modeId = "off";
      else if (lt.mode === 3) modeId = "solid";
      else if (lt.mode === 2) modeId = "breathe";
      else if (lt.mode === 0) modeId = "off";

      p.light.mode = modeId;
      p.light.brightness = Math.round(((lt.brightness || 0) / 4) * 100);
      p.light.speed = Math.max(1, Math.min(10, Math.round(1 + ((lt.speed || 2) / 4) * 9)));
      if (lt.color && lt.color !== "#000000") p.light.color = lt.color;
      log.push(modeId);
    }
  } catch {
    /* optional */
  }
  await sleep(40);

  saveState();
  renderAll();
  setStatus(log.length ? `Synced · ${log.join(" · ")}` : "Synced");
  return log;
}
