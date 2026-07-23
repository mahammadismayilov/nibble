/**
 * Nibble — open WebHID mouse configurator
 * Modular ES entry point.
 */

import { webHidSupported } from "./hid.js";
import { hid } from "./src/hidInstance.js";
import { pingActiveSession } from "./telemetry.js";
import { migrateLegacyStorage, initTheme } from "./src/theme.js";
import { setStatus } from "./src/state.js";
import { connectHid } from "./src/writer.js";
import { APP_VERSION } from "./src/constants.js";
import {
  initTabs,
  initDeviceSelect,
  initProfileSelect,
  renderAll,
} from "./src/ui/navigation.js";
import { bindDpiEditors } from "./src/ui/dpi.js";
import { bindLightEditors } from "./src/ui/light.js";
import { bindSettingsEditors } from "./src/ui/settings.js";
import { bindActions } from "./src/profileManager.js";

migrateLegacyStorage();

function init() {
  const isDesktop = typeof window !== "undefined" && !!(window.__TAURI_INTERNALS__ || window.__TAURI__);
  const badge = document.getElementById("app-type-badge");
  if (badge) badge.textContent = isDesktop ? "Desktop" : "Web";

  const platformText = document.getElementById("platform-badge-text");
  if (platformText) platformText.textContent = isDesktop ? "Desktop App (Native HID)" : "Web Companion (WebHID)";

  const softVer = document.getElementById("soft-version");
  if (softVer) softVer.textContent = `v${APP_VERSION}`;
  const aboutVer = document.getElementById("about-version");
  if (aboutVer) aboutVer.textContent = APP_VERSION;

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
    bindSettingsEditors();
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
      btn.title = "Requires Chrome, Edge, or Nibble Desktop";
    }
  }

  async function startupCheck() {
    const main = document.getElementById("main-app");
    const landing = document.getElementById("landing-screen");

    // Desktop Tauri: auto-connect via native Rust HID (no WebHID needed)
    if (isDesktop && window.__TAURI__ && window.__TAURI__.core) {
      try {
        if (main) main.style.display = "";
        if (landing) landing.style.display = "none";
        await connectHid(false); // requestAndOpen() handles native HID internally
      } catch (e) {
        console.error("Desktop auto-connect failed:", e);
        if (main) main.style.display = "";
        if (landing) landing.style.display = "none";
      }
      return;
    }

    // Web: requires WebHID support
    if (!webHidSupported()) return;
    try {
      const devices = await hid.getGrantedConfigDevices();

      if (devices.length >= 1) {
        try {
          if (main) main.style.display = "";
          if (landing) landing.style.display = "none";
          await hid.openDevice(devices[0]);
          await connectHid(true);
        } catch (e) {
          console.error(e);
          showLandingScreen(devices);
        }
      } else {
        showLandingScreen([]);
      }
    } catch (e) {
      console.error(e);
      showLandingScreen([]);
    }
  }

  startupCheck();
}

function showLandingScreen(devices) {
  const landing = document.getElementById("landing-screen");
  const main = document.getElementById("main-app");
  if (!landing || !main) return;

  landing.style.display = "flex";
  main.style.display = "none";

  const grid = document.getElementById("landing-grid");
  if (!grid) return;
  grid.innerHTML = "";

  if (devices.length > 0) {
    devices.forEach((dev) => {
      const card = document.createElement("div");
      card.className = "landing-card";
      let img = "assets/device/mouse_aj179.png";
      const name = dev.productName || "AJAZZ Mouse";

      card.innerHTML = `
        <img src="${img}" alt="${name}" />
        <h3>${name}</h3>
      `;
      card.addEventListener("click", async () => {
        try {
          landing.style.display = "none";
          main.style.display = "";
          await hid.openDevice(dev);
          await connectHid(true);
        } catch (e) {
          console.error(e);
        }
      });
      grid.appendChild(card);
    });
  }
}

document.getElementById("btn-landing-connect")?.addEventListener("click", async () => {
  try {
    const landing = document.getElementById("landing-screen");
    const main = document.getElementById("main-app");
    if (landing && main) {
      landing.style.display = "none";
      main.style.display = "";
    }
    await connectHid(false);
  } catch (e) {
    console.error(e);
  }
});

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => {
    init();
    pingActiveSession();
  });
} else {
  init();
  pingActiveSession();
}
