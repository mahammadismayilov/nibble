import fs from "fs";
import path from "path";

console.log("\n==================================================");
console.log(" 🧪 NIBBLE DESKTOP (TAURI V2) VERIFICATION SUITE");
console.log("==================================================\n");

let errors = 0;
let passes = 0;

function logPass(msg) {
  passes++;
  console.log(`  ✓ ${msg}`);
}

function logFail(msg) {
  errors++;
  console.error(`  ❌ ${msg}`);
}

const desktopDir = path.resolve(process.cwd());

// 1. Verify desktop/package.json
try {
  const pkgPath = path.join(desktopDir, "package.json");
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));
  if (pkg.name === "nibble-desktop" && pkg.dependencies["@tauri-apps/api"]) {
    logPass(`package.json verified: ${pkg.name} v${pkg.version}`);
  } else {
    logFail("package.json invalid name or missing @tauri-apps/api dependency");
  }
} catch (err) {
  logFail(`Failed to parse desktop/package.json: ${err.message}`);
}

// 2. Verify desktop/src-tauri/tauri.conf.json
try {
  const confPath = path.join(desktopDir, "src-tauri", "tauri.conf.json");
  const conf = JSON.parse(fs.readFileSync(confPath, "utf-8"));
  if (conf.identifier === "com.nibble.desktop" && conf.productName === "Nibble") {
    logPass(`tauri.conf.json verified: ID=${conf.identifier}, Product=${conf.productName}`);
  } else {
    logFail("tauri.conf.json identifier or productName mismatch");
  }
} catch (err) {
  logFail(`Failed to parse tauri.conf.json: ${err.message}`);
}

// 3. Verify desktop/src-tauri/Cargo.toml
try {
  const cargoPath = path.join(desktopDir, "src-tauri", "Cargo.toml");
  const cargoStr = fs.readFileSync(cargoPath, "utf-8");
  if (cargoStr.includes('name = "nibble-desktop"') && cargoStr.includes("tray-icon")) {
    logPass(`Cargo.toml verified: name="nibble-desktop" with tray-icon feature enabled`);
  } else {
    logFail("Cargo.toml missing package name or tray-icon feature");
  }
} catch (err) {
  logFail(`Failed to read Cargo.toml: ${err.message}`);
}

// 4. Verify desktop/src-tauri/src/main.rs
try {
  const mainRsPath = path.join(desktopDir, "src-tauri", "src", "main.rs");
  const mainRsStr = fs.readFileSync(mainRsPath, "utf-8");
  if (mainRsStr.includes("TrayIconBuilder") && mainRsStr.includes("on_window_event")) {
    logPass(`main.rs verified: TrayIconBuilder and WindowEvent close-prevention implemented`);
  } else {
    logFail("main.rs missing TrayIconBuilder or close event listener");
  }
} catch (err) {
  logFail(`Failed to read main.rs: ${err.message}`);
}

// 5. Verify index.html in desktop
try {
  const htmlPath = path.join(desktopDir, "index.html");
  const htmlStr = fs.readFileSync(htmlPath, "utf-8");
  if (htmlStr.includes('<span class="badge">Desktop</span>') && htmlStr.includes('src="/app.js"')) {
    logPass(`index.html verified: Desktop badge & /app.js module link present`);
  } else {
    logFail("index.html missing Desktop badge or /app.js link");
  }
} catch (err) {
  logFail(`Failed to read index.html: ${err.message}`);
}

console.log("\n==================================================");
if (errors === 0) {
  console.log(` 🎉 DESKTOP VERIFICATION PASSED (${passes} check(s) passed)`);
  console.log("==================================================\n");
  process.exit(0);
} else {
  console.error(` ❌ DESKTOP VERIFICATION FAILED WITH ${errors} ERROR(S)`);
  console.log("==================================================\n");
  process.exit(1);
}
