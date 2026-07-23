import { profileRegistry } from "../profiles/registry.js";

export const APP_NAME = "Nibble";
export const APP_VERSION = "1.0.4";
export const STORAGE_KEY = "nibble-web-v1";
export const THEME_KEY = "nibble-theme";
export const LEGACY_STORAGE_KEY = "ajazz-driver-web-v1";
export const LEGACY_THEME_KEY = "ajazz-theme";

export const DEVICES = profileRegistry.getAllProfiles();

export const KEY_LABELS = {
  201: "Left Button",
  202: "Right Button",
  203: "Middle / Wheel",
  204: "Forward",
  205: "Backward",
  206: "DPI Button",
};

export const FUNC_GROUPS = [
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

export const FUNC_LABEL = Object.fromEntries(
  FUNC_GROUPS.flatMap((g) => g.items.map((i) => [i.id, i.label]))
);

export function defaultProfile(deviceId) {
  const device = DEVICES.find((d) => d.id === deviceId) || DEVICES[0];
  const keys = {};
  device.keys.forEach((k) => {
    keys[k.id] = k.defaultFunc;
  });
  const activeDpi = device.defaultDpiIndex ?? 1;
  const dpiStages = (device.dpiDefaults || [
    { value: 400, color: "#FF0000" },
    { value: 800, color: "#00FF00" },
    { value: 1200, color: "#0000FF" },
    { value: 1600, color: "#00FFFF" },
    { value: 2400, color: "#FFFF00" },
    { value: 3200, color: "#800080" },
  ]).map((s) => ({ value: s.value, color: s.color }));

  const reportRates = device.reportRates || [125, 250, 500, 1000];
  const defaultRateIdx = reportRates.includes(1000) ? reportRates.indexOf(1000) : reportRates.length - 1;

  return {
    name: "Default Profile",
    deviceId: device.id,
    activeDpi,
    dpiStages,
    reportRateIndex: defaultRateIdx,
    light: {
      mode: device.defaultLight || "solid",
      brightness: 100,
      speed: 5,
      color: dpiStages[activeDpi]?.color || "#00FF00",
    },
    keys,
    settings: {
      lod: "low",
      angleSnap: false,
      ripple: false,
      sleepWire: 5,
      moveWake: true,
      moveCloseLight: false,
      debounce: 4,
    },
  };
}
