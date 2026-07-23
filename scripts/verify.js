import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { JSDOM } from "jsdom";
import { profileRegistry, PROFILES } from "../profiles/registry.js";
import { driverRegistry } from "../drivers/registry.js";
import { NibbleHid, webHidSupported } from "../hid.js";
import * as protocol from "../protocol.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appRoot = path.resolve(__dirname, "..");

let errors = 0;
let warnings = 0;

function logPass(msg) {
  console.log(`  ✓ ${msg}`);
}

function logFail(msg) {
  console.error(`  ❌ ${msg}`);
  errors++;
}

function logWarn(msg) {
  console.warn(`  ⚠️ ${msg}`);
  warnings++;
}

console.log("\n==================================================");
console.log(" 🧪 NIBBLE APP AUTOMATED VERIFICATION SUITE");
console.log("==================================================\n");

// --------------------------------------------------
// 1. Profile Schema & Asset Validation
// --------------------------------------------------
console.log("1. Validating Profile Definitions & Assets...");
const requiredProfileFields = ["id", "name", "driver", "image", "sensor", "maxDpi", "modes", "keys"];

for (const p of PROFILES) {
  let profileValid = true;
  for (const field of requiredProfileFields) {
    if (p[field] === undefined) {
      logFail(`Profile "${p.name || p.id}" is missing required field: "${field}"`);
      profileValid = false;
    }
  }

  if (p.image) {
    const imagePath = path.resolve(appRoot, p.image);
    if (!fs.existsSync(imagePath)) {
      logFail(`Profile "${p.name}" references non-existent image asset: "${p.image}"`);
      profileValid = false;
    }
  }

  if (Array.isArray(p.modes)) {
    for (const m of p.modes) {
      if (!m.vid || !m.pid) {
        logFail(`Profile "${p.name}" mode has invalid VID/PID: ${JSON.stringify(m)}`);
        profileValid = false;
      }
    }
  } else {
    logFail(`Profile "${p.name}" modes is not an array.`);
    profileValid = false;
  }

  if (profileValid) {
    logPass(`Profile "${p.name}" (${p.id}) verified - ${p.modes.length} mode(s)`);
  }
}

// --------------------------------------------------
// 2. Driver Interface & Plugin Validation
// --------------------------------------------------
console.log("\n2. Validating Hardware Driver Plugins...");
const requiredDriverMethods = [
  "supportsVendor",
  "getTransferOptions",
  "buildStatusQuery",
  "buildSetReportRate",
];

const compx = driverRegistry.getDriver("compx");
const yichip = driverRegistry.getDriver("yichip");

for (const drv of [compx, yichip]) {
  let drvValid = true;
  for (const method of requiredDriverMethods) {
    if (typeof drv[method] !== "function") {
      logFail(`Driver "${drv.name}" (${drv.id}) is missing method: "${method}"`);
      drvValid = false;
    }
  }

  const opts = drv.getTransferOptions("rate");
  if (typeof opts !== "object" || typeof opts.allowNoReply !== "boolean") {
    logFail(`Driver "${drv.name}" getTransferOptions() returned invalid options structure`);
    drvValid = false;
  }

  if (drvValid) {
    logPass(`Driver "${drv.name}" (${drv.id}) verified`);
  }
}

// --------------------------------------------------
// 3. Module Imports & Exports Verification
// --------------------------------------------------
console.log("\n3. Verifying ES Module Import/Export Integrity...");
const appJsPath = path.resolve(appRoot, "app.js");
const appJs = fs.readFileSync(appJsPath, "utf8");

const importRegex = /import\s+\{([^}]+)\}\s+from\s+["']([^"']+)["']/g;
const imports = [...appJs.matchAll(importRegex)];

const loadedModules = {
  "./protocol.js": protocol,
  "./hid.js": { NibbleHid, webHidSupported },
  "./profiles/registry.js": { profileRegistry },
  "./drivers/registry.js": { driverRegistry },
};

for (const [, specifiers, modulePath] of imports) {
  const names = specifiers.split(",").map((s) => s.trim()).filter(Boolean);
  let targetModule = loadedModules[modulePath];

  if (!targetModule && (modulePath.startsWith("./src/") || modulePath.startsWith("../src/"))) {
    try {
      const fullPath = path.resolve(appRoot, modulePath);
      targetModule = await import(`file:///${fullPath.replace(/\\/g, "/")}`);
    } catch (e) {
      logFail(`Failed to dynamically import module "${modulePath}": ${e.message}`);
    }
  }

  if (!targetModule && !modulePath.includes("telemetry")) {
    logWarn(`Unchecked module import path: "${modulePath}"`);
    continue;
  }

  if (targetModule) {
    for (const name of names) {
      if (!(name in targetModule)) {
        logFail(`Import error in app.js: "${name}" is imported from "${modulePath}", but does not exist!`);
      } else {
        logPass(`Import "${name}" from "${modulePath}" verified`);
      }
    }
  }
}

// --------------------------------------------------
// 4. Registry Method Invocations Check
// --------------------------------------------------
console.log("\n4. Checking Registry & HID Class Method Invocations...");

const profileRegistryCalls = [...new Set([...appJs.matchAll(/profileRegistry\.([a-zA-Z0-9_$]+)\(/g)].map((m) => m[1]))];
for (const m of profileRegistryCalls) {
  if (typeof profileRegistry[m] !== "function") {
    logFail(`Method profileRegistry.${m}() referenced in app.js does not exist on ProfileRegistry!`);
  } else {
    logPass(`profileRegistry.${m}() exists`);
  }
}

const driverRegistryCalls = [...new Set([...appJs.matchAll(/driverRegistry\.([a-zA-Z0-9_$]+)\(/g)].map((m) => m[1]))];
for (const m of driverRegistryCalls) {
  if (typeof driverRegistry[m] !== "function") {
    logFail(`Method driverRegistry.${m}() referenced in app.js does not exist on DriverRegistry!`);
  } else {
    logPass(`driverRegistry.${m}() exists`);
  }
}

const hidInst = new NibbleHid();
const hidCalls = [...new Set([...appJs.matchAll(/hid\.([a-zA-Z0-9_$]+)\(/g)].map((m) => m[1]))];
for (const m of hidCalls) {
  if (typeof hidInst[m] !== "function") {
    logFail(`Method hid.${m}() referenced in app.js does not exist on NibbleHid!`);
  } else {
    logPass(`hid.${m}() exists`);
  }
}

// --------------------------------------------------
// 5. HTML DOM Element ID Alignment
// --------------------------------------------------
console.log("\n5. Verifying DOM Element IDs in index.html...");
const indexHtmlPath = path.resolve(appRoot, "index.html");
const indexHtml = fs.readFileSync(indexHtmlPath, "utf8");

const htmlIds = new Set([...indexHtml.matchAll(/id=["']([^"']+)["']/g)].map((m) => m[1]));
const jsGetIdCalls = [...new Set([...appJs.matchAll(/getElementById\(["']([^"']+)["']\)/g)].map((m) => m[1]))];

for (const id of jsGetIdCalls) {
  if (!htmlIds.has(id)) {
    logWarn(`app.js references DOM element ID "#${id}", but it is not in index.html`);
  } else {
    logPass(`DOM element ID "#${id}" verified in HTML`);
  }
}

// --------------------------------------------------
// 6. Protocol Packet Builders & Parsers Sanity Tests
// --------------------------------------------------
console.log("\n6. Running Protocol Packet Builders & Parsers Sanity Tests...");

try {
  const statusBuf = protocol.buildStatusQuery();
  if (!(statusBuf instanceof Uint8Array) || statusBuf.length < 32) {
    logFail("buildStatusQuery() returned invalid payload");
  } else {
    logPass(`buildStatusQuery() generated ${statusBuf.length}-byte packet`);
  }

  const rateBuf = protocol.buildReportRate(3);
  if (!(rateBuf instanceof Uint8Array)) {
    logFail("buildReportRate() failed");
  } else {
    logPass(`buildReportRate() generated ${rateBuf.length}-byte packet`);
  }

  const dpiBuf = protocol.buildDpi([{ value: 800 }, { value: 1600 }], 0, 0);
  if (!(dpiBuf instanceof Uint8Array)) {
    logFail("buildDpi() failed");
  } else {
    logPass(`buildDpi() generated ${dpiBuf.length}-byte packet`);
  }

  const lightBuf = protocol.buildLight(1, { brightness: 3, speed: 2, color: "#ff0000" });
  if (!(lightBuf instanceof Uint8Array)) {
    logFail("buildLight() failed");
  } else {
    logPass(`buildLight() generated ${lightBuf.length}-byte packet`);
  }

  const mockStatusReport = new Uint8Array(33);
  mockStatusReport[0] = 0x00;
  mockStatusReport[1] = 0xc0;
  mockStatusReport[2] = 1;
  mockStatusReport[3] = 85;
  const parsed = protocol.parseStatus(mockStatusReport);
  if (!parsed || parsed.kind !== "status" || parsed.battery !== 85) {
    logFail(`parseStatus() failed to parse mock status report: ${JSON.stringify(parsed)}`);
  } else {
    logPass(`parseStatus() correctly parsed 85% battery status report`);
  }
} catch (err) {
  logFail(`Protocol sanity test exception: ${err.message}`);
}

// --------------------------------------------------
// 7. Mock WebHID Device Transfer Simulation
// --------------------------------------------------
console.log("\n7. Simulating Mock WebHID Device Transfers...");

async function testMockHidTransfers() {
  const mockDevice = {
    opened: true,
    async sendReport(reportId, data) {
      return Promise.resolve();
    },
  };

  const testHid = new NibbleHid();
  testHid.device = mockDevice;

  try {
    const testBuf = new Uint8Array(33);
    const result = await testHid.xfer(testBuf, { allowNoReply: true, timeoutMs: 200 });
    if (result && result.length === 33) {
      logPass("Mock WebHID transfer with allowNoReply: true succeeded");
    } else {
      logFail("Mock WebHID transfer with allowNoReply: true returned invalid result");
    }
  } catch (err) {
    logFail(`Mock WebHID transfer allowNoReply failed: ${err.message}`);
  }

  try {
    const testBuf = new Uint8Array(33);
    testBuf[1] = 0x10;

    const xferPromise = testHid.xfer(testBuf, { allowNoReply: false, timeoutMs: 500 });

    setTimeout(() => {
      const mockRx = new Uint8Array(33);
      mockRx[1] = 0x10;
      testHid._handleInput({ reportId: 0, data: { buffer: mockRx.buffer } });
    }, 50);

    const rxResult = await xferPromise;
    if (rxResult && rxResult.length === 33) {
      logPass("Mock WebHID transfer with ACK report response succeeded");
    } else {
      logFail("Mock WebHID transfer with ACK report response failed");
    }
  } catch (err) {
    logFail(`Mock WebHID transfer with ACK failed: ${err.message}`);
  }
}

await testMockHidTransfers();

// --------------------------------------------------
// 8. Full JSDOM UI Interaction & Event Handler Test
// --------------------------------------------------
console.log("\n8. Executing JSDOM Browser UI Interaction Stress Test...");

try {
  const dom = new JSDOM(indexHtml, {
    url: "http://localhost:8080/",
    runScripts: "outside-only",
    resources: "usable",
  });

  const { window } = dom;
  global.window = window;
  global.document = window.document;
  global.HTMLElement = window.HTMLElement;
  global.HTMLSelectElement = window.HTMLSelectElement;
  global.HTMLInputElement = window.HTMLInputElement;
  global.Blob = window.Blob;

  // Mock browser globals needed by UI handlers
  global.confirm = window.confirm = () => true;
  global.requestAnimationFrame = window.requestAnimationFrame = (cb) => setTimeout(cb, 0);
  global.cancelAnimationFrame = window.cancelAnimationFrame = (id) => clearTimeout(id);
  global.URL.createObjectURL = () => "blob:mock-url";
  global.URL.revokeObjectURL = () => {};

  // Mock WebHID on window.navigator
  Object.defineProperty(window.navigator, "hid", {
    value: {
      async getDevices() { return []; },
      async requestDevice() { return []; },
      addEventListener() {},
      removeEventListener() {},
    },
    writable: true,
    configurable: true,
  });

  // Mock localStorage
  const localStore = new Map();
  global.localStorage = {
    getItem(k) { return localStore.get(k) || null; },
    setItem(k, v) { localStore.set(k, String(v)); },
    removeItem(k) { localStore.delete(k); },
    clear() { localStore.clear(); },
  };

  // Import app.js in simulated JSDOM environment
  await import(`file:///${appJsPath.replace(/\\/g, "/")}`);

  // Trigger DOMContentLoaded
  window.document.dispatchEvent(new window.Event("DOMContentLoaded"));

  logPass("JSDOM initialization & DOMContentLoaded fired cleanly");

  // Test UI Element Clicks & Interactions
  const clickableIds = [
    "btn-connect",
    "btn-disconnect",
    "btn-landing-connect",
    "btn-theme",
    "btn-reset-keys",
    "btn-reset-dpi",
    "btn-reset-light",
    "btn-export-profile",
  ];

  for (const id of clickableIds) {
    const el = window.document.getElementById(id);
    if (el) {
      el.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
      logPass(`Simulated click on #${id}`);
    }
  }

  // Test Tab Switching
  const tabs = window.document.querySelectorAll("#tabs .tab");
  for (const tab of tabs) {
    tab.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
    const tabName = tab.getAttribute("data-tab");
    logPass(`Simulated click on tab [data-tab="${tabName}"]`);
  }

  // Test Select Dropdowns
  const devSelect = window.document.getElementById("device-select");
  if (devSelect && devSelect.options.length > 0) {
    devSelect.selectedIndex = 1;
    devSelect.dispatchEvent(new window.Event("change", { bubbles: true }));
    logPass("Simulated change on #device-select");
  }

  const profSelect = window.document.getElementById("profile-select");
  if (profSelect && profSelect.options.length > 0) {
    profSelect.selectedIndex = 0;
    profSelect.dispatchEvent(new window.Event("change", { bubbles: true }));
    logPass("Simulated change on #profile-select");
  }

  // Test LOD Segmented Active Button Rendering across state representations
  const { renderSettings } = await import(`file:///${path.resolve(appRoot, "src/ui/settings.js").replace(/\\/g, "/")}`);
  const { state, profile } = await import(`file:///${path.resolve(appRoot, "src/state.js").replace(/\\/g, "/")}`);

  for (const lodVal of ["low", "high", 1, 2]) {
    profile().settings.lod = lodVal;
    renderSettings();
    const activeBtn = window.document.querySelector("#lod-options button.active");
    if (!activeBtn) {
      logFail(`renderSettings() failed to render an active button for lod = ${JSON.stringify(lodVal)}`);
    } else {
      const expected = (lodVal === "high" || lodVal === 2) ? "high" : "low";
      if (activeBtn.dataset.value !== expected) {
        logFail(`renderSettings() active button mismatch for lod = ${JSON.stringify(lodVal)}: got "${activeBtn.dataset.value}", expected "${expected}"`);
      } else {
        logPass(`renderSettings() active button correctly highlighted "${expected}" for lod = ${JSON.stringify(lodVal)}`);
      }
    }
  }

} catch (err) {
  logFail(`JSDOM UI Stress Test Exception: ${err.stack || err.message}`);
}

// --------------------------------------------------
// Summary Report
// --------------------------------------------------
console.log("\n==================================================");
if (errors === 0) {
  console.log(` 🎉 ALL VERIFICATION CHECKS PASSED SUCCESSFULLY! (${warnings} warning(s))`);
  console.log("==================================================\n");
  process.exit(0);
} else {
  console.error(` ❌ VERIFICATION FAILED WITH ${errors} ERROR(S) AND ${warnings} WARNING(S)!`);
  console.log("==================================================\n");
  process.exit(1);
}
