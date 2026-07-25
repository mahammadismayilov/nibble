import { APP_NAME, APP_VERSION, STORAGE_KEY, defaultProfile } from "./constants.js";
import { state, defaultState, setSelectedDpiStage, saveState, device, profile, toast } from "./state.js";
import { hid } from "./hidInstance.js";
import { queueDeviceWrite, connectHid, disconnectHid } from "./writer.js";
import { renderAll, renderKeys, renderDpi, renderHome, renderLight } from "./ui/navigation.js";

export function bindActions() {
  document.getElementById("btn-reset-keys")?.addEventListener("click", () => {
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

  document.getElementById("btn-reset-dpi")?.addEventListener("click", () => {
    if (!confirm("Reset DPI stages to defaults?")) return;
    const d = device();
    const p = profile();
    p.dpiStages = d.dpiDefaults.map((x) => ({ ...x }));
    p.activeDpi = d.defaultDpiIndex;
    p.reportRateIndex = d.defaultRateIndex;
    setSelectedDpiStage(p.activeDpi);
    saveState();
    renderDpi();
    renderHome();
    queueDeviceWrite("rate", "dpi", "light");
    toast("DPI reset");
  });

  document.getElementById("btn-reset-light")?.addEventListener("click", () => {
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

  document.getElementById("btn-factory")?.addEventListener("click", () => {
    if (!confirm("This will delete all saved profiles. Continue?")) return;
    localStorage.removeItem(STORAGE_KEY);
    const newState = defaultState();
    Object.assign(state, newState);
    setSelectedDpiStage(1);
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
