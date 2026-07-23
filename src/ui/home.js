import { state, device, profile } from "../state.js";
import { hid } from "../hidInstance.js";
import { updateLightPreview } from "./light.js";

export function renderHome() {
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

  const formatImgSrc = (src) => {
    if (!src) return "assets/device/mouse_aj179.png";
    return src.replace(/^\/+/, "");
  };

  const mouseImg = document.getElementById("home-mouse-img") || document.getElementById("mouse-image");
  if (mouseImg) mouseImg.src = formatImgSrc(d.image);
  const keysImg = document.getElementById("keys-mouse-img") || document.getElementById("keys-mouse-image");
  if (keysImg) keysImg.src = formatImgSrc(d.image);
  const lightImg = document.getElementById("light-mouse-image");
  if (lightImg) lightImg.src = formatImgSrc(d.image);

  const subText = hid.connected
    ? `Connected · ${d.sensor ? d.sensor + " sensor" : "Gaming mouse"}`
    : d.sensor
      ? `${d.sensor} sensor`
      : "Gaming mouse";

  setTxt("home-dev-name", d.name);
  setTxt("home-device-name", d.name);
  setTxt("home-dev-sub", subText);
  setTxt("home-device-sub", subText);

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
    const chgPrefix = state.batteryCharging ? "⚡ " : "";
    setTxt("stat-battery", `${chgPrefix}${Math.min(100, bat)}%${held ? " (last)" : ""}`);
    let sub = "Discharging";
    if (state.batteryOnline === false) sub = "Sleeping";
    else if (state.batteryCharging) sub = "Charging";
    else if (bat <= 20) sub = "Low";
    setTxt("stat-battery-sub", sub);
  } else {
    let sub = "Connect to read";
    if (hid.connected) {
      if (state.batteryCharging) {
        setTxt("stat-battery", "⚡ Charging");
        sub = "Charging";
      } else if (state.batteryOnline === false) {
        setTxt("stat-battery", "Sleeping");
        sub = "Sleeping";
      } else {
        setTxt("stat-battery", "—");
        sub = "Unavailable";
      }
    } else {
      setTxt("stat-battery", "—");
    }
    setTxt("stat-battery-sub", sub);
  }
  updateLightPreview();
}
