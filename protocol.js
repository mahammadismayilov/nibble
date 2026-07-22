/**
 * Nibble HID protocol helpers (VID 248A/249A config interface).
 * Packets reconstructed from OEM tooling static analysis + USB captures.
 *
 * Buffer (Windows-style, report id at [0]):
 *   [0]=0x00  [1]=cmd  [2]=0x00  [3]=0x01  [4]=field meta  [5…]=payload  [0x20]=checksum
 * Transfer size: 0x21 (33) or 0x41 (65).
 */

export const CMD = {
  REPORT_RATE: 0x02,
  DPI: 0x03,
  DPI_RGB: 0x04,
  LIGHT: 0x05,
  SENSOR: 0x06,
  POWER: 0x07,
  /** Macro stream (chunked) — not used; risky */
  KEYS_MACRO: 0x08,
  /** Full 6-button keymap table — captures use this */
  KEYMAP: 0x09,
  STATUS: 0x10,
  RESET: 0x0f,
};

export const VIDS = [0x248a, 0x249a, 0x3151];
export const PIDS = [0x5c2e, 0x5d2e, 0x5e2e, 0x5c2f, 0x5007, 0x502d, 0x402d, 0x4026];

/** Checksum from FUN_00448e40 / FUN_00448d40 */
export function checksum(buf) {
  let sum = 0;
  // 9 triples covering offsets 5..31; pointer starts at offset 6
  for (let i = 0; i < 9; i++) {
    const p = 6 + i * 3;
    sum = (sum + buf[p - 1] + buf[p] + buf[p + 1]) & 0xff;
  }
  return sum;
}

export function finalize(buf) {
  buf[0x20] = checksum(buf);
  return buf;
}

function alloc(size = 0x41) {
  const b = new Uint8Array(size);
  return b;
}

/** Header 00 cmd 00 01  (dword 0x01_00_cmd_00 LE → actually 0x0100XX00) */
function setHeader(buf, cmd) {
  // Matches decompiler constants like 0x1000200 → bytes 00 02 00 01
  buf[0] = 0x00;
  buf[1] = cmd & 0xff;
  buf[2] = 0x00;
  buf[3] = 0x01;
}

/**
 * DPI wire encode FUN_0041da30(sensorType, dpi)
 * Default branch (most PAW3395 paths): dpi/50 - 1
 * Types 4, 0x10, 0x12, 0x35: dpi/100
 */
export function encodeDpi(dpi, sensorType = 0) {
  const d = Math.max(50, Math.min(26000, dpi | 0));
  if (sensorType === 0x11) {
    if (d > 10000) return ((d - 10000) / 100 + 200) | 0;
    return (d / 100) | 0;
  }
  if (sensorType === 0x25) {
    if (d > 5000) return ((d - 5000) / 1000 + 0x32) | 0;
    return (d / 100) | 0;
  }
  if (sensorType === 4 || sensorType === 0x12 || sensorType === 0x10 || sensorType === 0x35) {
    return (d / 100) | 0;
  }
  // common path
  return ((d / 50) | 0) - 1;
}

export function decodeDpi(wire, sensorType = 0) {
  const w = wire | 0;
  if (sensorType === 4 || sensorType === 0x12 || sensorType === 0x10 || sensorType === 0x35 || sensorType === 0x11) {
    return w * 100;
  }
  return (w + 1) * 50;
}

/** Status / battery query — FUN_00448d40 */
export function buildStatusQuery() {
  const buf = alloc(0x21);
  // *(uint16*)buf = 0x1000 → 00 10
  buf[0] = 0x00;
  buf[1] = CMD.STATUS;
  return finalize(buf);
}

/**
 * Report rate — FUN_004338d0
 * rateIndex: 0..3 for 125/250/500/1000
 */
export function encodeRateIndex(idx) {
  // rateIndex: 0=125, 1=250, 2=500, 3=1000
  // bInterval: 8=125, 4=250, 2=500, 1=1000
  const map = { 0: 8, 1: 4, 2: 2, 3: 1 };
  return map[idx] || 1;
}

export function decodeRateWire(wireValue) {
  const map = { 8: 0, 4: 1, 2: 2, 1: 3 };
  return map[wireValue] !== undefined ? map[wireValue] : 3;
}

export function buildReportRate(rateIndex) {
  const buf = alloc(0x41);
  setHeader(buf, CMD.REPORT_RATE);
  buf[4] = 1;
  buf[5] = encodeRateIndex(rateIndex);
  return finalize(buf);
}

/** GET report rate — load path uses *(uint16*) = 0x1200 */
export function buildReportRateGet() {
  const buf = alloc(0x21);
  buf[0] = 0x00;
  buf[1] = 0x12;
  return finalize(buf);
}

/**
 * Parse GET 0x12 response. SET body is [4]=1 [5]=rateIndex+1;
 * after success [1] is often 0 — accept either form.
 */
export function parseReportRateResponse(buf) {
  if (!buf || buf.length < 6) return null;
  if (buf[4] === 1) {
    const rateIndex = decodeRateWire(buf[5]);
    return { rateIndex, raw: buf };
  }
  return null;
}

/** GET DPI table — load path uses 0x1300 */
export function buildDpiGet() {
  const buf = alloc(0x21);
  buf[0] = 0x00;
  buf[1] = 0x13;
  return finalize(buf);
}

/**
 * Parse GET 0x13 — same layout as SET cmd 0x03:
 *   [4]=0x25  [5]=(active<<4)|count  [6+]=6×(lo hi lo hi)
 */
export function parseDpiResponse(buf, sensorType = 0) {
  if (!buf || buf.length < 30) return null;
  
  const cmd = buf[0] === 0x00 ? buf[1] : buf[0];
  // We don't check cmd strictly because the mouse uses an unknown command byte for spontaneous DPI interrupts.
  // Instead, we structurally validate the packet below.
  
  let base = 0;
  // Find meta 0x25
  let metaAt = -1;
  for (let i = 0; i < 8; i++) {
    if (buf[i] === 0x25) {
      metaAt = i;
      break;
    }
  }
  if (metaAt < 0) {
    // fallback assume standard windows offsets
    metaAt = 4;
  }
  const packed = buf[metaAt + 1];
  if (packed == null) return null;
  const activeIndex = (packed >> 4) & 0x0f;
  const count = packed & 0x0f;
  
  // Structural validation: DPI packet must have a sane count and activeIndex!
  // If count is wildly off, it's a false positive (e.g. from a battery packet containing 0x25).
  if (count === 0 || count > 8 || activeIndex >= count) {
    return null;
  }
  
  const stages = [];
  let o = metaAt + 2;
  let validStages = 0;
  
  for (let i = 0; i < 6; i++) {
    if (o + 3 >= buf.length) break;
    const lo = buf[o];
    const hi = buf[o + 1];
    const wire = lo | (hi << 8);
    
    // Check if the wire value is somewhat sane for DPI (e.g., usually between 0 and 15000, and not totally random).
    // An empty stage might be 0, but if it's random garbage like 65000, reject it.
    // DPI step is usually 50, so wire is usually > 0 and < 300.
    // Actually AJ179 PAW3395 raw wire values are max ~520 for 26000 DPI.
    if (wire > 600) {
      return null; // Highly unlikely to be a real DPI packet!
    }
    
    stages.push({
      wire,
      value: decodeDpi(wire, sensorType),
    });
    o += 4; // lo hi lo hi
  }
  
  if (!stages.length) return null;
  return {
    activeIndex: Math.min(activeIndex, stages.length - 1),
    count: count || stages.length,
    stages,
    raw: buf,
  };
}

/**
 * DPI stages — FUN_00433230 + live captures
 *
 * HID: 03 00 01 25 (idx<<4|count) + 6×(lo hi lo hi) + pad + checksum
 *   count=6 active=1 → byte 0x16  (capture)
 *   count=2 active=1 → byte 0x12  (capture: only count nibble changes)
 *
 * Official always ships **all 6 stage slots**; lowering preset count does NOT
 * zero/truncate the table — unused slots keep their stored DPI values.
 *
 * stages: array of { value } (up to 6; missing slots pad from last known)
 * activeIndex: 0-based (clamped to count-1)
 * opts.count: enabled preset count 1..6 (default: min(6, stages.length))
 */
export function buildDpi(stages, activeIndex = 0, sensorType = 0, opts = {}) {
  const buf = alloc(0x41);
  setHeader(buf, CMD.DPI);
  buf[4] = 0x25;

  const list = Array.isArray(stages) ? stages : [];
  const enabled =
    opts.count != null
      ? Math.min(6, Math.max(1, opts.count | 0))
      : Math.min(6, Math.max(1, list.length || 1));
  const idx = Math.min(enabled - 1, Math.max(0, activeIndex | 0));
  buf[5] = ((idx & 0x0f) << 4) | (enabled & 0x0f);

  // Always 6 wire slots (capture keeps full table when count drops to 2)
  let last = list[0]?.value ?? 400;
  let o = 6;
  for (let i = 0; i < 6; i++) {
    if (list[i] && list[i].value != null) last = list[i].value;
    const enc = encodeDpi(last, sensorType) & 0xffff;
    const lo = enc & 0xff;
    const hi = (enc >> 8) & 0xff;
    // decompiler + capture: lo, hi, lo, hi per stage
    buf[o++] = lo;
    buf[o++] = hi;
    buf[o++] = lo;
    buf[o++] = hi;
  }
  return finalize(buf);
}

/**
 * DPI / LED color table — FUN_00433620 (cmd **0x04**)
 *
 * Capture (official color change):
 *   04 00 01 12 + 6×(R G B) + pad + checksum
 *   e.g. ff0000 00ff00 8f00af 00ffff ffff00 800080
 *
 * Multi-color LED: in **constant light** mode the official app has no color
 * picker — changing a DPI stage color (this packet) changes the constant LED.
 * colors: 0xRRGGBB numbers or #rrggbb strings (6 entries)
 */
export function buildDpiRgb(colors) {
  const buf = alloc(0x41);
  setHeader(buf, CMD.DPI_RGB);
  buf[4] = 0x12;
  let o = 5;
  for (let i = 0; i < 6; i++) {
    const c = parseColor(colors[i] || colors[0] || "#ffffff");
    buf[o++] = c.r;
    buf[o++] = c.g;
    buf[o++] = c.b;
  }
  return finalize(buf);
}

/** Parse GET/echo of cmd 0x04 style buffer (or 32-byte HID). */
export function parseDpiRgbResponse(buf) {
  if (!buf || buf.length < 23) return null;
  let start = 0;
  for (let i = 0; i < 6; i++) {
    if (buf[i] === 0x04 && buf[i + 3] === 0x12) {
      start = i + 4;
      break;
    }
    if (buf[i] === 0x12 && i >= 3) {
      start = i + 1;
      break;
    }
  }
  if (start === 0 && buf[4] === 0x12) start = 5;
  if (start === 0 && buf[3] === 0x12) start = 4;
  const colors = [];
  for (let i = 0; i < 6; i++) {
    const o = start + i * 3;
    if (o + 2 >= buf.length) break;
    const r = buf[o];
    const g = buf[o + 1];
    const b = buf[o + 2];
    const hx = (n) => n.toString(16).padStart(2, "0");
    colors.push(`#${hx(r)}${hx(g)}${hx(b)}`);
  }
  return colors.length ? { colors, raw: buf } : null;
}

function parseColor(c) {
  if (typeof c === "number") {
    return { r: (c >> 16) & 0xff, g: (c >> 8) & 0xff, b: c & 0xff };
  }
  const s = String(c).replace("#", "");
  const n = parseInt(s.length === 3 ? s.split("").map((x) => x + x).join("") : s, 16);
  return { r: (n >> 16) & 0xff, g: (n >> 8) & 0xff, b: n & 0xff };
}

/**
 * Lighting — ground truth from official driver USB captures (HID data, 32 bytes).
 *
 * BREATHE:
 *   05 00 01 18 02 42 07 + 7×RGB rainbow + 00 00 00 3f
 * SOLID (red example):
 *   05 00 01 05 03 42 ff 00 00 + zeros + 44
 * OFF:
 *   05 00 01 02 00 00 + zeros + 00
 *   → meta=2, mode=0, packed=0  (off is mode 0, NOT 5)
 *
 * Packed byte (Ghidra FUN_00433000):  (brightness << 4) | speed
 * Wire scale is **0–4** (not 0–15). Official captures use 0x42 = bri 4, spd 2.
 * Values above 4 often make the LED look "off".
 *
 * Windows buffer = 00 + those 32; checksum at [0x20].
 */

/** Max brightness/speed nibble used by AJ179 firmware (capture 0x42). */
export const LIGHT_LEVEL_MAX = 4;

/**
 * Map UI brightness (0–100%) → wire 0–4.
 * Prefer calling this from the UI layer, then pass wire levels into builders.
 */
export function uiBrightnessToWire(percent) {
  if (percent == null || Number.isNaN(Number(percent))) return LIGHT_LEVEL_MAX;
  const n = Math.min(100, Math.max(0, Number(percent)));
  return Math.round((n / 100) * LIGHT_LEVEL_MAX);
}

/**
 * Map UI speed (1–10) → wire 0–4.
 * Default UI 5 → 2 (matches official capture low nibble).
 */
export function uiSpeedToWire(uiSpeed) {
  if (uiSpeed == null || Number.isNaN(Number(uiSpeed))) return 2;
  const n = Math.min(10, Math.max(1, Number(uiSpeed)));
  return Math.round(((n - 1) / 9) * LIGHT_LEVEL_MAX);
}

/**
 * Pack brightness/speed into the light config byte.
 * Expects **wire levels 0–4** (or pass opts.packed / percent via helpers above).
 * Ghidra: local_52 = (local_94 << 4) | local_90
 */
function lightPacked(opts = {}) {
  if (opts.packed != null) return opts.packed & 0xff;
  if (opts.forceZero) return 0x00;

  let bri = opts.brightness;
  let spd = opts.speed;

  // If caller still passed UI-scale values, normalize (percent > 4 or speed > 4).
  if (bri == null || Number.isNaN(Number(bri))) bri = LIGHT_LEVEL_MAX;
  else if (Number(bri) > LIGHT_LEVEL_MAX) bri = uiBrightnessToWire(bri);
  else bri = Math.max(0, Number(bri) | 0);

  if (spd == null || Number.isNaN(Number(spd))) spd = 2;
  else if (Number(spd) > LIGHT_LEVEL_MAX) spd = uiSpeedToWire(spd);
  else spd = Math.max(0, Number(spd) | 0);

  if (!opts.allowZero && opts.forceMinBright && bri === 0) bri = 1;
  if (!opts.allowZero && spd === 0) spd = 1;

  bri = Math.min(LIGHT_LEVEL_MAX, Math.max(0, bri | 0));
  spd = Math.min(LIGHT_LEVEL_MAX, Math.max(0, spd | 0));

  return ((bri & 0x0f) << 4) | (spd & 0x0f);
}

/** Official rainbow palette from breathe capture. */
export const LIGHT_RAINBOW_7 = [
  [0xff, 0x00, 0x00],
  [0x00, 0xff, 0x00],
  [0x00, 0x00, 0xff],
  [0x00, 0xff, 0xff],
  [0xff, 0xff, 0x00],
  [0xff, 0x00, 0xff],
  [0xff, 0xff, 0xff],
];

/**
 * Build SET light from captures + Ghidra.
 * UI ids map via lightIdToWire: breathe→2, solid→3, off→0, flow→1, …
 */
export function buildLight(modeWire, opts = {}) {
  const buf = alloc(0x21);
  let mode = modeWire & 0xff;
  if (mode === 6) mode = 0; // XML light_6 "close" → firmware mode 0
  if (mode === 5) mode = 0; // legacy off mapping

  const isOff = mode === 0 && (opts.off === true || opts.forceZero || opts.packed === 0);
  const packed = isOff ? 0x00 : lightPacked({ allowZero: true, ...opts });
  const c = parseColor(opts.color || "#ff0000");

  setHeader(buf, CMD.LIGHT); // 00 05 00 01

  if (mode === 2) {
    // BREATHE capture
    buf[4] = 0x18;
    buf[5] = 0x02;
    buf[6] = packed;
    buf[7] = 0x07;
    const useColors =
      opts.useRainbow === true || opts.color == null
        ? LIGHT_RAINBOW_7
        : Array(7).fill([c.r, c.g, c.b]);
    // If UI passed a color for breathe, tint all slots; else rainbow
    const colors = opts.color && opts.useRainbow !== true ? Array(7).fill([c.r, c.g, c.b]) : LIGHT_RAINBOW_7;
    let o = 8;
    for (let i = 0; i < 7; i++) {
      const col = colors[i];
      buf[o++] = col[0];
      buf[o++] = col[1];
      buf[o++] = col[2];
    }
  } else if (mode === 3) {
    // SOLID: 05 00 01 05 03 PACKED R G B … (full RGB)
    buf[4] = 0x05;
    buf[5] = 0x03;
    buf[6] = packed;
    buf[7] = c.r;
    buf[8] = c.g;
    buf[9] = c.b;
  } else if (mode === 1) {
    buf[4] = 0x03;
    buf[5] = 0x01;
    buf[6] = packed;
  } else if (mode === 0 || isOff) {
    // OFF capture: 05 00 01 02 00 00 …
    buf[4] = 0x02;
    buf[5] = 0x00;
    buf[6] = 0x00;
  } else if (mode === 4) {
    // wave — same long form as breathe with mode id 4 (unverified)
    buf[4] = 0x18;
    buf[5] = 0x04;
    buf[6] = packed;
    buf[7] = 0x07;
    let o = 8;
    for (const col of LIGHT_RAINBOW_7) {
      buf[o++] = col[0];
      buf[o++] = col[1];
      buf[o++] = col[2];
    }
  } else {
    buf[4] = 0x02;
    buf[5] = mode;
    buf[6] = packed;
  }

  return finalize(buf);
}

/** Exact breathe rainbow from capture (packed default 0x42). */
export function buildLightFromCapture(opts = {}) {
  return buildLight(2, {
    useRainbow: true,
    brightness: opts.brightness ?? LIGHT_LEVEL_MAX,
    speed: opts.speed ?? 2,
    packed: opts.packed,
    allowZero: true,
  });
}

export function buildLightSetCandidates(modeWire, opts = {}) {
  const o = { brightness: LIGHT_LEVEL_MAX, speed: 2, color: "#ff0000", allowZero: true, ...opts };
  return [
    { name: "breathe capture", buf: buildLight(2, { ...o, useRainbow: true }) },
    { name: "solid red capture-style", buf: buildLight(3, { ...o, color: "#ff0000" }) },
    { name: "off capture", buf: buildLight(0, { off: true }) },
    { name: "buildLight mode", buf: buildLight(modeWire, o) },
  ];
}

/** GET light state — FUN_00430490 uses *(uint16*)buf = 0x1500 */
export function buildLightGet() {
  const buf = alloc(0x21);
  buf[0] = 0x00;
  buf[1] = 0x15;
  return finalize(buf);
}

/**
 * Parse GET 0x15 response.
 *
 * Mirrors SET body after header `00 15 00 01`:
 *   Solid:   05 03 PACKED R G B …   → meta@4 mode@5 packed@6
 *   Breathe: 18 02 PACKED 07 RGB… → meta@4 mode@5 packed@6
 *   Off:     02 00 00 …            → meta@4 mode@5 packed@6
 *
 * (Older short reads sometimes looked like mode@4 packed@5; prefer meta forms.)
 */
export function parseLightResponse(buf) {
  if (!buf || buf.length < 7) return null;
  const b4 = buf[4];
  const b5 = buf[5];
  const b6 = buf[6];

  // Known meta values from captures: 0x02 off/short, 0x03, 0x05 solid, 0x18 breathe
  const looksLikeMeta = b4 === 0x02 || b4 === 0x03 || b4 === 0x05 || b4 === 0x18;
  let meta = 0;
  let mode = 0;
  let packed = 0;
  if (looksLikeMeta) {
    meta = b4;
    mode = b5 & 0x0f;
    packed = b6;
  } else {
    // fallback
    mode = b4 & 0x0f;
    packed = b5;
  }
  let r = 0;
  let g = 0;
  let b = 0;
  if (looksLikeMeta && mode === 3 && buf.length > 9) {
    r = buf[7];
    g = buf[8];
    b = buf[9];
  } else if (looksLikeMeta && mode === 2 && buf.length > 10) {
    // first breathe slot
    r = buf[8];
    g = buf[9];
    b = buf[10];
  }
  const toHex2 = (n) => n.toString(16).padStart(2, "0");
  return {
    meta,
    mode,
    modeRaw: looksLikeMeta ? b5 : b4,
    packed,
    brightness: (packed >> 4) & 0x0f,
    speed: packed & 0x0f,
    r,
    g,
    b,
    color: `#${toHex2(r)}${toHex2(g)}${toHex2(b)}`,
    raw: buf,
  };
}

/** Patch packed byte into a 32-byte HID capture (or 33-byte windows buf). */
export function patchLightPacked(hidBuf, packed) {
  const b = hidBuf.length === 32 ? new Uint8Array([0, ...hidBuf]) : new Uint8Array(hidBuf);
  // Windows layout: [4]=meta [5]=mode [6]=packed for solid/breathe/off captures
  b[6] = packed & 0xff;
  return finalize(b);
}

export function buildSolidCapture(opts = {}) {
  // Solid RGB: meta 05 mode 03 packed R G B (capture used ff 00 00; any RGB works)
  return buildLight(3, {
    brightness: opts.brightness ?? LIGHT_LEVEL_MAX,
    speed: opts.speed ?? 2,
    packed: opts.packed,
    color: opts.color || "#ff0000",
    allowZero: opts.allowZero !== false,
  });
}

export function buildOffCapture() {
  const buf = alloc(0x21);
  setHeader(buf, CMD.LIGHT);
  buf[4] = 0x02;
  buf[5] = 0x00;
  buf[6] = 0x00;
  return finalize(buf);
}

export function buildBreatheCapture(opts = {}) {
  // Rainbow if no color / useRainbow; single color fills all 7 slots
  const useRainbow = opts.useRainbow === true || opts.color == null;
  return buildLight(2, {
    useRainbow,
    brightness: opts.brightness ?? LIGHT_LEVEL_MAX,
    speed: opts.speed ?? 2,
    packed: opts.packed,
    color: opts.color,
    allowZero: opts.allowZero !== false,
  });
}

/**
 * Button keymap — captures + Ghidra encoder (cmd **0x09**).
 *
 * HID: 09 00 01 0f + 6×(type, val_lo, val_hi) + pad + checksum
 * Order = XML key_value 0..5: LMB, RMB, MMB, Back, Forward, DPI
 *
 * Type bytes (driver encoder + captures):
 *   0x10 mouse button bits / special mouse
 *   0x20 scroll wheel step
 *   0x40 DPI / mouse feature
 *   0x60 disable / light
 *   0x80 HID Consumer (USB HUT page 0x0C) — value is usage ID
 *   0x30 fire / advanced (partial)
 *
 * Media values match USB HID Consumer Usage Table and Ghidra
 * (vol+ 0xE9 proven on wire; others same table in EXE).
 */

/** UI function id → [b0, b1, b2] wire triple. */
export const KEY_FUNC_WIRE = {
  // --- mouse (type 0x10) — capture + Ghidra type==1 ---
  left: [0x10, 0x01, 0x00],
  middle: [0x10, 0x04, 0x00],
  right: [0x10, 0x02, 0x00],
  forward: [0x10, 0x10, 0x00],
  backward: [0x10, 0x08, 0x00],
  // Ghidra type==1 value 4 → LE 0x3230 + 0x03
  double: [0x30, 0x32, 0x03],
  // Ghidra type==1 value 7/8 → type becomes 0x20
  scroll_up: [0x20, 0x01, 0x00],
  scroll_down: [0x20, 0xff, 0x00],
  // Ghidra type==1 value 9/10 (fire-ish; third byte only)
  fire: [0x10, 0x00, 0xf5],

  // --- DPI feature (type 0x40) — capture D: dpi_loop = 40 01 ---
  dpi_loop: [0x40, 0x01, 0x00],
  // Ghidra type==5 also encodes 02 / 03 (likely DPI±; not in UI yet)
  dpi_plus: [0x40, 0x02, 0x00],
  dpi_minus: [0x40, 0x03, 0x00],

  // --- disable / light (type 0x60) ---
  disable: [0x60, 0x00, 0x00], // capture F
  light_toggle: [0x60, 0x04, 0x00], // Ghidra type==6 value 1

  // --- multimedia (type 0x80, HID Consumer) — Ghidra type==3 ---
  // USB HUT: CD play/pause, B5 next, B6 prev, B7 stop, E9/EA vol, E2 mute
  play: [0x80, 0xcd, 0x00],
  stop: [0x80, 0xb7, 0x00],
  prev: [0x80, 0xb6, 0x00],
  next: [0x80, 0xb5, 0x00],
  vol_up: [0x80, 0xe9, 0x00], // capture C
  vol_down: [0x80, 0xea, 0x00],
  mute: [0x80, 0xe2, 0x00],
  // Ghidra value 1 → LE u16 0x0183 at bytes 1..2
  media_player: [0x80, 0x83, 0x01],

  // --- system (type 0x80 / combos) — Ghidra type==8 ---
  brightness_up: [0x80, 0x6f, 0x00], // HUT Brightness Increment
  brightness_down: [0x80, 0x70, 0x00],
  calc: [0x80, 0x92, 0x01], // HUT AL Calculator 0x0192
  computer: [0x80, 0x94, 0x01], // HUT AL Local Machine Browser 0x0194
  // Ghidra 0x0223 WWW Home
  home: [0x80, 0x23, 0x02],
  mail: [0x80, 0x8a, 0x01], // HUT AL Email Reader 0x018A
  // Keyboard-style combos (type 0x70) — captures + Ghidra type==8
  // refresh F5, switch app (Alt+Tab-ish), copy, paste
  refresh: [0x70, 0x08, 0x07], // capture
  switch_app: [0x70, 0x08, 0x2b], // capture
  copy: [0x70, 0x01, 0x06], // capture (Ghidra 0x0170 + 0x06)
  paste: [0x70, 0x01, 0x19], // capture (Ghidra 0x0170 + 0x19)
};

/**
 * Safe to Apply without extra captures.
 * Capture-proven OR (same Ghidra branch as capture-proven + standard HID).
 */
export const KEY_FUNC_PROVEN = new Set([
  // capture-proven
  "left",
  "right",
  "middle",
  "backward",
  "forward",
  "dpi_loop",
  "disable",
  "vol_up",
  // same Ghidra multimedia table as vol_up + USB HID Consumer
  "vol_down",
  "mute",
  "play",
  "stop",
  "prev",
  "next",
  // same Ghidra system table + standard HID AL usages
  "calc",
  "computer",
  "mail",
  "brightness_up",
  "brightness_down",
  // Ghidra mouse type==1 siblings of proven L/R/M/Back/Fwd
  "scroll_up",
  "scroll_down",
  "double",
  "light_toggle",
  // type 0x70 keyboard combos — captures
  "refresh",
  "switch_app",
  "copy",
  "paste",
]);

/** Still blocked (odd encodings / macros). */
export const KEY_FUNC_EXPERIMENTAL = new Set([
  "fire",
  "dpi_plus",
  "dpi_minus",
  "media_player",
  "home",
  "macro",
]);

export function keyFuncToWire(funcId) {
  const triple = KEY_FUNC_WIRE[funcId];
  if (!triple) return null;
  return [triple[0] & 0xff, triple[1] & 0xff, triple[2] & 0xff];
}

/**
 * @param {string[]} funcsByKeyValue - length 6, index = key_value 0..5
 * @param {{ allowUnproven?: boolean }} [opts]
 */
export function buildKeyMap(funcsByKeyValue, opts = {}) {
  const buf = alloc(0x21);
  setHeader(buf, CMD.KEYMAP);
  buf[4] = 0x0f; // meta from all captures
  const list = Array.isArray(funcsByKeyValue) ? funcsByKeyValue : [];
  let o = 5;
  for (let i = 0; i < 6; i++) {
    const id = list[i] || "disable";
    const wire = keyFuncToWire(id);
    if (!wire) {
      throw new Error(`Unknown key function: ${id}`);
    }
    if (!opts.allowUnproven && !KEY_FUNC_PROVEN.has(id)) {
      throw new Error(`Unproven / experimental key function: ${id}`);
    }
    buf[o++] = wire[0];
    buf[o++] = wire[1];
    buf[o++] = wire[2];
  }
  return finalize(buf);
}

export function wireToKeyFunc(b0, b1, b2) {
  for (const [id, triple] of Object.entries(KEY_FUNC_WIRE)) {
    if (triple[0] === b0 && triple[1] === b1 && triple[2] === b2) {
      return id;
    }
  }
  return null;
}

export function buildKeyMapGet() {
  const buf = alloc(0x21);
  buf[0] = 0x00;
  buf[1] = 0x19;
  return finalize(buf);
}

export function parseKeyMapResponse(buf) {
  if (!buf || buf.length < 23) return null;
  let start = -1;
  for (let i = 0; i < buf.length - 18; i++) {
    if ((buf[i] === 0x19 || buf[i] === 0x09) && (buf[i + 3] === 0x0f || buf[i + 4] === 0x0f)) {
      start = buf[i + 3] === 0x0f ? i + 4 : i + 5;
      break;
    }
  }
  if (start < 0) {
    if (buf[4] === 0x0f) start = 5;
    else if (buf[3] === 0x0f) start = 4;
    else start = 5; // fallback
  }

  const funcs = [];
  for (let i = 0; i < 6; i++) {
    const o = start + i * 3;
    if (o + 2 >= buf.length) break;
    const matched = wireToKeyFunc(buf[o], buf[o + 1], buf[o + 2]);
    funcs.push(matched || "disable");
  }
  return funcs.length === 6 ? funcs : null;
}

/** Defaults matching XML / capture D baseline for AJ179. */
export function defaultKeyMapFuncs() {
  return ["left", "right", "middle", "backward", "forward", "dpi_loop"];
}

/**
 * Sensor / LOD — FUN_004339e0 + captures (cmd 0x06)
 *
 * HID: 06 00 01 05  LOD  ANGLE  RIPPLE  …
 *   LOD: 1 = 1mm, 2 = 2mm (captures); 0 seen as prior default
 *   ANGLE: 1 when angle snap on   (capture @ [6])
 *   RIPPLE: 1 when ripple on      (capture @ [7])
 *
 * No "Motion Sync" control in the OEM Windows UI we mirrored.
 */
export function buildSensor({
  lod = 1,
  lodHigh = false,
  angleSnap = false,
  ripple = false,
} = {}) {
  const buf = alloc(0x41);
  setHeader(buf, CMD.SENSOR);
  buf[4] = 5;
  // Prefer explicit lod (1|2); lodHigh kept for older callers → 2mm
  let lift = lod;
  if (lift == null || Number.isNaN(Number(lift))) lift = lodHigh ? 2 : 1;
  lift = Math.min(2, Math.max(0, lift | 0));
  buf[5] = lift;
  buf[6] = angleSnap ? 1 : 0;
  buf[7] = ripple ? 1 : 0;
  return finalize(buf);
}

export function buildSensorGet() {
  const buf = alloc(0x41);
  setHeader(buf, CMD.SENSOR + 0x10); // 0x16
  return finalize(buf);
}

export function parseSensorResponse(buf) {
  if (!buf || buf.length < 8) return null;
  // Usually the GET returns the same format as SET body starting at index 5 or so.
  // We'll inspect buf[5], buf[6], buf[7] assuming it echoes the SET command.
  return {
    lod: buf[5],
    angleSnap: buf[6] === 1,
    ripple: buf[7] === 1,
    raw: Array.from(buf.slice(0, 16)),
  };
}

/**
 * Power / sleep / debounce — FUN_00433b40 + captures (cmd 0x07)
 *
 * HID: 07 00 01 04  SLEEP  MOVE_WAKE  MOVE_CLOSELIGHT  DEBOUNCE  …
 * Sleep wire = units of 10 seconds (official dropdown):
 *   10s=1, 30s=3, 1m=6, 2m=12, 5m=30, 10m=60, 20m=120, 30m=180
 * Capture with sleep byte 0x06 = 1 min.
 */
export const SLEEP_WIRE_OPTIONS = [
  { wire: 1, label: "10 sec", seconds: 10 },
  { wire: 3, label: "30 sec", seconds: 30 },
  { wire: 6, label: "1 min", seconds: 60 },
  { wire: 12, label: "2 min", seconds: 120 },
  { wire: 30, label: "5 min", seconds: 300 },
  { wire: 60, label: "10 min", seconds: 600 },
  { wire: 120, label: "20 min", seconds: 1200 },
  { wire: 180, label: "30 min", seconds: 1800 },
];

/** @param {number} wireOrMinutes - wire unit (preferred) or legacy minutes if 1–30 and not in table */
export function normalizeSleepWire(v) {
  const n = Number(v);
  if (SLEEP_WIRE_OPTIONS.some((o) => o.wire === n)) return n;
  // legacy: profile stored "minutes" as 1–30 → convert to wire (min * 6)
  if (n >= 1 && n <= 30 && Number.isInteger(n)) {
    const asMin = n * 6; // 1 min → 6, 5 min → 30
    if (SLEEP_WIRE_OPTIONS.some((o) => o.wire === asMin)) return asMin;
  }
  return 30; // default 5 min (XML sleep_light=30 in 10s units)
}

export function buildPower({
  sleepWire = 30,
  sleepMin, // legacy alias
  moveWake = false,
  moveCloseLight = false,
  debounce = 8,
} = {}) {
  const buf = alloc(0x41);
  setHeader(buf, CMD.POWER);
  buf[4] = 4;
  const sleep = normalizeSleepWire(sleepWire != null ? sleepWire : sleepMin);
  buf[5] = sleep & 0xff;
  buf[6] = moveWake ? 1 : 0;
  buf[7] = moveCloseLight ? 1 : 0;
  buf[8] = Math.min(30, Math.max(0, debounce | 0)) & 0xff;
  return finalize(buf);
}

/** Soft reset-ish — FUN_004345a0 */
export function buildReset() {
  const buf = alloc(0x21);
  setHeader(buf, CMD.RESET);
  buf[4] = 0x01;
  buf[5] = 0xff;
  return finalize(buf);
}

/**
 * Firmware mode ids from captures + XML:
 *   off = 0 (capture), flow ≈ 1, breathe = 2, solid = 3, neon/wave = 4
 */
export function lightIdToWire(id) {
  const map = { off: 0, flow: 1, breathe: 2, solid: 3, neon: 4, wave: 4 };
  return map[id] ?? 2;
}

export function lightIdToWire0(id) {
  return lightIdToWire(id);
}

export function lightIdToWireXml(id) {
  return lightIdToWire(id);
}

export function wireToLightId(n) {
  const map = {
    0: "off",
    1: "flow",
    2: "breathe",
    3: "solid",
    4: "wave",
    5: "off",
    6: "off",
  };
  return map[n] || "breathe";
}

/**
 * Status / battery — match Rockeyxx/AJ179-linux-battery:
 *   reply starts with C0 → battery = byte[2] (raw 0–100, or 0xFF dock)
 *   link byte[1] === 0 → sleep
 *   GET 0x10 fallback: [0x0c]=flag [0x0d]=level (Ghidra)
 *   short 00 ff ff → dock marker (not 100%)
 */
export function parseStatus(buf) {
  if (!buf || buf.length < 2) return null;
  const u8 = buf instanceof Uint8Array ? buf : new Uint8Array(buf);

  const isChargeBit = (flag) => {
    const f = flag == null ? 0 : flag | 0;
    return f === 1 || f === 2 || f === 3;
  };

  const make = (level, chargeFlag, link, source, extra = {}) => {
    if (link === 0x00) {
      return {
        kind: "status",
        online: false,
        sleeping: true,
        charging: false,
        chargeFlag: 0,
        battery: null,
        batteryRaw: null,
        confidence: "sleep",
        link: 0,
        raw: u8,
        source,
        debug: extra.debug || "sleep",
      };
    }
    if (level == null || level < 0 || level > 255) return null;

    const dockMarker = level === 0xff;
    // 0xFF is dock/full marker — NOT UI 100%
    const pct = dockMarker ? null : normalizeBatteryPercent(level);
    if (!dockMarker && pct == null) return null;

    const flag = isChargeBit(chargeFlag) ? chargeFlag | 0 : 0;
    return {
      kind: "status",
      online: true,
      sleeping: false,
      charging: dockMarker || flag > 0,
      chargeFlag: flag,
      battery: pct,
      batteryRaw: level,
      devId: extra.devId || null,
      confidence: dockMarker
        ? "dock-marker"
        : extra.confidence || (String(source).startsWith("c0") ? "c0" : "get"),
      link: link ?? 1,
      raw: u8,
      source,
      debug:
        extra.debug ||
        (dockMarker
          ? `dock-marker flag=${flag}`
          : `raw=${level} → ${pct}% flag=${flag}`),
    };
  };

  // Dock short (capture)
  if (u8.length >= 3 && u8[0] === 0x00 && u8[1] === 0xff && u8[2] === 0xff) {
    return make(0xff, 1, 1, "short-ff");
  }
  if (u8.length >= 2 && u8[0] === 0xff && u8[1] === 0xff) {
    return make(0xff, 1, 1, "short-ff");
  }

  // C0 — same as linux daemon: if (buf[0]==0xC0) return buf[2]
  // Also allow C0 after a leading 0x00 report-id byte (WebHID 33-byte form)
  for (let i = 0; i < Math.min(3, u8.length - 2); i++) {
    if (u8[i] !== 0xc0) continue;
    const link = u8[i + 1];
    const level = u8[i + 2];
    const flag = u8.length > i + 3 ? u8[i + 3] : 0;

    if (link === 0x00) {
      return make(null, 0, 0, `c0-sleep@${i}`, {
        debug: `c0 sleep @${i} l=${level} f=${flag}`,
      });
    }

    // Trust C0[2] exactly like linux — any 1–254 / 0xFF
    if (level === 0x00) continue;
    return make(level, flag, link, `c0@${i}`, {
      confidence: level === 0xff ? "dock-marker" : "c0",
      debug: `c0@${i} link=${link} bat=${level} flag=${flag}`,
    });
  }

  // GET 0x10 reply — Ghidra SetBatteryInfo(level=buf[0x0d], flag=buf[0x0c])
  // is relative to the 32-byte HID payload where byte0 = 0x10.
  //
  // User capture (WebHID 33-byte / leading 00):
  //   00 10 00 01 0b 4d 31 37 39 84 01 95 01 00 59 01 …
  //   "M179" in the middle; level=0x59 (89%) at index 0x0e, flag at 0x0d
  //   (= Ghidra 0x0c/0x0d shifted +1 by the leading report byte).
  //
  // Wrong old code read [0x0d]=0x00 → "Unavailable".
  const tryGet10 = (cmdAt, tag) => {
    // flag/level relative to HID payload start (cmd byte)
    const flagOff = cmdAt + 0x0c;
    const levelOff = cmdAt + 0x0d;
    if (u8.length <= levelOff) return null;
    
    let devId = null;
    if (u8.length >= cmdAt + 8) {
      const b1 = u8[cmdAt + 4], b2 = u8[cmdAt + 5], b3 = u8[cmdAt + 6], b4 = u8[cmdAt + 7];
      if (b1 === 0x4d) devId = String.fromCharCode(b1, b2, b3, b4);
    }

    const flag = u8[flagOff];
    const level = u8[levelOff];
    if (level === 0xff) {
      return make(0xff, flag, 1, tag, {
        devId,
        debug: `${tag} flag@${flagOff}=${flag} level@${levelOff}=ff`,
      });
    }
    if (level > 0 && level <= 100) {
      return make(level, flag, 1, tag, {
        devId,
        confidence: "get",
        debug: `${tag} flag@${flagOff}=${flag} level@${levelOff}=${level} → ${level}%`,
      });
    }
    // 101–254 rare 0–255 scale
    if (level > 100 && level < 255) {
      return make(level, flag, 1, tag, {
        devId,
        confidence: "get",
        debug: `${tag} flag@${flagOff}=${flag} level@${levelOff}=${level}`,
      });
    }
    return null;
  };

  // 33-byte Windows form: [0]=00 [1]=10 → level at 0x0e
  if (u8[0] === 0x00 && u8[1] === 0x10) {
    const st = tryGet10(1, "get10/win33");
    if (st) return st;
  }
  // 32-byte HID form: [0]=10 → level at 0x0d (Ghidra as written)
  if (u8[0] === 0x10) {
    const st = tryGet10(0, "get10/hid32");
    if (st) return st;
  }
  // Fallback: scan for cmd 0x10 near start
  for (let i = 0; i < Math.min(3, u8.length); i++) {
    if (u8[i] === 0x10) {
      const st = tryGet10(i, `get10@${i}`);
      if (st) return st;
    }
  }

  return {
    kind: "ack",
    ok: u8[1] === 0x00 || u8[0] === 0xc0 || u8[0] === 0xff,
    cmd: u8[0] === 0 ? u8[1] : u8[0],
    raw: u8,
  };
}

/**
 * Wire byte → UI percent.
 * 1–100 direct; 0xFF = not a percent (dock); 101–254 ≈ 0–255 scale.
 */
export function normalizeBatteryPercent(v) {
  if (v == null || Number.isNaN(Number(v))) return null;
  const n = Number(v) | 0;
  if (n <= 0) return null;
  if (n === 0xff || n === 255) return null;
  if (n <= 100) return n;
  if (n < 255) return Math.max(1, Math.min(100, Math.round((n / 255) * 100)));
  return null;
}

export function bufToHex(buf) {
  return [...buf].map((x) => x.toString(16).padStart(2, "0")).join(" ");
}
