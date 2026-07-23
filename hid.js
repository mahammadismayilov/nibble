/**
 * WebHID transport for Nibble (vendor config collection, typically MI_02).
 *
 * Composite dongle exposes several HID devices:
 *   usage 2  = mouse          (reject)
 *   usage 6  = keyboard       (reject)
 *   page 12  = consumer       (reject)
 *   usage 0  = vendor config  ← THIS ONE
 *
 * Windows/WebHID often reports that collection with in:[] out:[] even though
 * write/read work at 33 bytes. We still open it and try sendReport.
 */

import { VIDS, PIDS, bufToHex } from "./protocol.js";

export function webHidSupported() {
  return typeof navigator !== "undefined" && !!navigator.hid;
}

export function deviceFilters() {
  const filters = [];
  for (const vendorId of VIDS) {
    // Filter by Vendor ID (under Chrome's 64 filter limit)
    filters.push({ vendorId });
  }
  return filters;
}

export class NibbleHid {
  constructor() {
    this.device = null;
    this._onReport = null;
    this._waiters = [];
    this.lastError = null;
    /** @type {string|null} */
    this.frameMode = null;
    this.lastTx = null;
    this.lastRx = null;
  }

  get connected() {
    return !!(this.device && this.device.opened);
  }

  get info() {
    if (!this.device) return null;
    return {
      productName: this.device.productName,
      vendorId: this.device.vendorId,
      productId: this.device.productId,
      reports: describeReports(this.device),
      frameMode: this.frameMode,
      isConfigInterface: isConfigInterface(this.device),
      score: scoreDevice(this.device),
      hasListedOutputs: maxOutputDataSize(this.device) > 0,
    };
  }

  async requestAndOpen() {
    if (!webHidSupported()) {
      throw new Error("WebHID not supported — use Chrome/Edge on http://localhost");
    }

    const picked = await navigator.hid.requestDevice({ filters: deviceFilters() });
    if (!picked.length) throw new Error("No device selected");

    const granted = await navigator.hid.getDevices();
    const pool = uniqueDevices([...picked, ...granted].filter(isSupportedVidPid));

    const config = pickConfigDevice(pool);
    if (!config) {
      throw new Error(
        "Couldn’t find the mouse config interface. Pick the Wireless-Receiver entry, not mouse or keyboard."
      );
    }

    await this.openDevice(config);
    return this.info;
  }

  async openDevice(device) {
    if (this.device && this.device !== device) await this.close();

    if (isForbiddenInterface(device)) {
      throw new Error(
        "That’s the mouse or keyboard interface. Choose the Wireless-Receiver / config entry instead."
      );
    }

    this.device = device;
    this.frameMode = null;
    if (!device.opened) await device.open();
    device.addEventListener("inputreport", this._handleInput);
    device.addEventListener("disconnect", this._handleDisconnect);
  }

  async tryReopenGranted() {
    if (!webHidSupported()) return false;
    const list = (await navigator.hid.getDevices()).filter(isSupportedVidPid);
    const config = pickConfigDevice(list);
    if (!config || isForbiddenInterface(config)) return false;
    await this.openDevice(config);
    return true;
  }

  async getGrantedConfigDevices() {
    if (!webHidSupported()) return [];
    const list = (await navigator.hid.getDevices()).filter(isSupportedVidPid);
    // Group by some unique identifier to avoid returning multiple interfaces for the same dongle
    // if pickConfigDevice only picked one, but if they have multiple mice, we want to return
    // one config interface PER physical device.
    // For now, let's group by productName + vendorId + productId
    const groups = new Map();
    for (const d of list) {
      const key = `${d.vendorId}-${d.productId}-${d.productName}`;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(d);
    }
    const configs = [];
    for (const group of groups.values()) {
      const config = pickConfigDevice(group);
      if (config && !isForbiddenInterface(config)) configs.push(config);
    }
    return configs;
  }

  _handleInput = (ev) => {
    const data = new Uint8Array(ev.data.buffer);
    const payload = normalizeInput(ev.reportId, data);
    this.lastRx = payload;
    for (const w of this._waiters.splice(0)) {
      clearTimeout(w.timer);
      w.resolve(payload);
    }
    if (this._onReport) this._onReport(payload, ev.reportId);
  };

  _handleDisconnect = () => {
    this.device = null;
    this.frameMode = null;
    for (const w of this._waiters.splice(0)) {
      clearTimeout(w.timer);
      w.reject(new Error("Device disconnected"));
    }
  };

  onReport(cb) {
    this._onReport = cb;
  }

  async close() {
    if (this.device) {
      try {
        this.device.removeEventListener("inputreport", this._handleInput);
        if (this.device.opened) await this.device.close();
      } catch (_) {}
    }
    this.device = null;
    this.frameMode = null;
  }

  /**
   * Send protocol buffer; wait for input report.
   * @param {Uint8Array} windowsBuf 33-byte buffer (leading 00) OR 32-byte HID data
   * @param {{timeoutMs?:number, retries?:number, exact?:boolean, preferStrip1?:boolean}} opts
   *   exact: do not recompute checksum; send bytes as-is (for capture replay)
   */
  async xfer(windowsBuf, opts = {}) {
    if (!this.connected) throw new Error("Not connected");

    const timeoutMs = opts.timeoutMs ?? 1200;
    const retries = opts.retries ?? 2;
    const exact = opts.exact === true;

    // Normalize to 33-byte Windows form (leading 0x00)
    let packet = new Uint8Array(33);
    if (windowsBuf.length === 32) {
      packet[0] = 0x00;
      packet.set(windowsBuf, 1);
    } else {
      packet.set(windowsBuf.subarray(0, 33));
    }
    if (!exact) {
      packet[0x20] = checksum33(packet);
    }
    this.lastTx = packet;

    // Capture-proven path: strip1 only (32-byte HID data like Wireshark "HID Data")
    // full33 as fallback if strip1 throws
    const preferStrip =
      opts.preferStrip1 !== false &&
      (this.frameMode === "strip1" || this.frameMode === null || exact);

    const attempts = preferStrip
      ? [
          { name: "strip1", reportId: 0, body: packet.slice(1) },
          { name: "full33", reportId: 0, body: packet },
        ]
      : [
          { name: "full33", reportId: 0, body: packet },
          { name: "strip1", reportId: 0, body: packet.slice(1) },
        ];

    let lastErr = null;
    let lastRx = null;
    let lastMode = null;

    for (let r = 0; r < retries; r++) {
      for (const att of attempts) {
        // Fast-path: Fire-and-forget write (no ACK report expected from device)
        if (opts.allowNoReply) {
          try {
            await this.device.sendReport(att.reportId, att.body);
            this.frameMode = att.name;
            return new Uint8Array(33);
          } catch (e) {
            lastErr = e;
            this.lastError = e;
            continue;
          }
        }

        // Waiter path for commands requiring ACK response
        let timer = null;
        let settled = false;

        const wait = new Promise((resolve, reject) => {
          timer = setTimeout(() => {
            if (!settled) {
              settled = true;
              this._waiters = this._waiters.filter((w) => w.timer !== timer);
              reject(new Error("HID read timeout"));
            }
          }, timeoutMs);

          this._waiters.push({
            resolve: (v) => {
              if (!settled) {
                settled = true;
                clearTimeout(timer);
                resolve(v);
              }
            },
            reject: (err) => {
              if (!settled) {
                settled = true;
                clearTimeout(timer);
                reject(err);
              }
            },
            timer,
          });
        });

        try {
          await this.device.sendReport(att.reportId, att.body);
        } catch (e) {
          lastErr = e;
          this.lastError = e;
          if (timer) clearTimeout(timer);
          this._waiters = this._waiters.filter((w) => w.timer !== timer);
          continue;
        }

        try {
          const rx = await wait;
          lastRx = rx;
          lastMode = att.name;
          this.frameMode = att.name;
          // Always accept a response for exact capture replay; device often echoes SET
          if (exact || isPlausibleAck(rx, packet)) {
            return rx;
          }
        } catch (e) {
          lastErr = e;
          this.lastError = e;
          // Write succeeded even if read timed out — fire-and-forget for exact writes
          if (exact) {
            this.frameMode = att.name;
            return new Uint8Array(33);
          }
        }
      }
    }

    if (lastRx) {
      this.frameMode = lastMode;
      return lastRx;
    }

    // Last resort: fire-and-forget strip1
    if (exact || opts.allowNoReply) {
      try {
        await this.device.sendReport(0, packet.slice(1));
        this.frameMode = "strip1";
        return new Uint8Array(33); // empty-ish success
      } catch (e) {
        lastErr = e;
      }
    }

    throw new Error(lastErr?.message || "No response from mouse");
  }

  /** Send raw 32-byte HID data exactly as Wireshark "HID Data" field. */
  async sendHidData32(hid32, opts = {}) {
    const u8 =
      typeof hid32 === "string"
        ? hexToBytes(hid32)
        : hid32 instanceof Uint8Array
          ? hid32
          : new Uint8Array(hid32);
    if (u8.length !== 32) {
      throw new Error(`Expected 32-byte HID data, got ${u8.length}`);
    }
    return this.xfer(u8, { ...opts, exact: true, preferStrip1: true, allowNoReply: true });
  }

  _cancelWaiter(promiseIgnored) {
    // no-op placeholder; waiters self-timeout
  }

  _waitReport(timeoutMs) {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this._waiters = this._waiters.filter((w) => w.resolve !== resolve);
        reject(new Error("HID read timeout"));
      }, timeoutMs);
      this._waiters.push({ resolve, reject, timer });
    });
  }
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function buildSendAttempts(packet, listedOut, learned) {
  /** @type {{name:string, reportId:number, body:Uint8Array}[]} */
  const list = [];

  const add = (name, reportId, body) => {
    list.push({ name, reportId, body });
  };

  // A: full 33-byte blob as report id 0 (Windows WriteFile style, no separate report id)
  add("full33", 0, packet);

  // B: strip leading 0x00, 32-byte data, reportId 0
  add("strip1", 0, packet.slice(1));

  // C: strip leading 0x00, reportId = 0 explicitly with padded 32
  {
    const b = new Uint8Array(32);
    b.set(packet.slice(1, 33));
    add("pad32", 0, b);
  }

  // D: some descriptors use report id matching first data byte after zero
  if (packet[1] !== 0) {
    add("rid=cmd", packet[1], packet.slice(2));
  }

  // E: if listed size is 64, pad to 64
  if (listedOut === 64 || listedOut === 0) {
    const b64 = new Uint8Array(64);
    b64.set(packet);
    add("pad64", 0, b64);
    const b64s = new Uint8Array(64);
    b64s.set(packet.slice(1));
    add("pad64strip", 0, b64s);
  }

  // Put learned mode first
  if (learned) {
    list.sort((a, b) => (a.name === learned ? -1 : b.name === learned ? 1 : 0));
  }

  // Dedupe
  const seen = new Set();
  return list.filter((a) => {
    const k = a.reportId + ":" + a.body.length + ":" + a.body[0] + ":" + a.body[1];
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}

function isSupportedVidPid(d) {
  return VIDS.includes(d.vendorId) && PIDS.includes(d.productId);
}

function uniqueDevices(list) {
  const seen = new Set();
  const out = [];
  for (const d of list) {
    const key = `${d.vendorId}:${d.productId}:${JSON.stringify(describeReports(d))}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(d);
  }
  return out;
}

function reportSize(r) {
  if (!r?.items?.length) return 0;
  return r.items.reduce((n, it) => n + ((it.reportSize * it.reportCount) >> 3), 0);
}

export function describeReports(device) {
  return (device.collections || []).map((c) => ({
    usagePage: c.usagePage,
    usage: c.usage,
    in: (c.inputReports || []).map((r) => ({ id: r.reportId, size: reportSize(r) })),
    out: (c.outputReports || []).map((r) => ({ id: r.reportId, size: reportSize(r) })),
  }));
}

function maxOutputDataSize(device) {
  let best = 0;
  for (const c of device.collections || []) {
    for (const r of c.outputReports || []) best = Math.max(best, reportSize(r));
  }
  return best;
}

function maxInputDataSize(device) {
  let best = 0;
  for (const c of device.collections || []) {
    for (const r of c.inputReports || []) best = Math.max(best, reportSize(r));
  }
  return best;
}

/** Mouse / keyboard / consumer — never use for config. */
function isForbiddenInterface(device) {
  const cols = device.collections || [];
  if (!cols.length) return false;
  // Pure mouse
  if (cols.every((c) => c.usagePage === 0x01 && c.usage === 0x02)) return true;
  // Keyboard and/or consumer only
  const onlyKbConsumer = cols.every(
    (c) =>
      (c.usagePage === 0x01 && (c.usage === 0x06 || c.usage === 0x07)) ||
      c.usagePage === 0x0c
  );
  if (onlyKbConsumer) return true;
  return false;
}

/**
 * Config interface if:
 *  - has 32/33/64/65 output, OR
 *  - usagePage 1 usage 0 (even with empty report lists — Windows quirk)
 */
export function isConfigInterface(device) {
  if (isForbiddenInterface(device)) return false;
  const out = maxOutputDataSize(device);
  if (out === 32 || out === 33 || out === 64 || out === 65) return true;
  for (const c of device.collections || []) {
    if (c.usagePage === 0x01 && c.usage === 0x00) return true;
    if (c.usagePage >= 0xff00) return true;
  }
  return false;
}

export function scoreDevice(device) {
  let score = 0;
  for (const c of device.collections || []) {
    if (c.usagePage === 0x01 && c.usage === 0x00) score += 50; // config
    if (c.usagePage >= 0xff00) score += 25;
    if (c.usagePage === 0x01 && c.usage === 0x06) score -= 40; // keyboard
    if (c.usagePage === 0x01 && c.usage === 0x02) score -= 40; // mouse
    if (c.usagePage === 0x0c) score -= 30; // consumer
    for (const r of c.outputReports || []) {
      const s = reportSize(r);
      if (s === 32 || s === 33) score += 40;
      if (s === 64 || s === 65) score += 15;
      if (s > 0) score += 5;
    }
    for (const r of c.inputReports || []) {
      const s = reportSize(r);
      if (s === 32 || s === 33) score += 20;
    }
  }
  return score;
}

function pickConfigDevice(devices) {
  if (!devices?.length) return null;
  const ranked = [...devices]
    .filter((d) => !isForbiddenInterface(d))
    .map((d) => ({ d, score: scoreDevice(d) }));
  ranked.sort((a, b) => b.score - a.score);
  if (!ranked.length) return null;
  // Accept usage 0 config even with empty out[] (score 50 in your log)
  if (ranked[0].score >= 40 || isConfigInterface(ranked[0].d)) return ranked[0].d;
  return null;
}

function normalizeInput(reportId, data) {
  if (data.length >= 33) return data.slice(0, 33);
  if (data.length === 32) {
    const p = new Uint8Array(33);
    p[0] = reportId || 0;
    p.set(data, 1);
    return p;
  }
  if (reportId) {
    const p = new Uint8Array(33);
    p[0] = reportId & 0xff;
    p.set(data.subarray(0, 32), 1);
    return p;
  }
  const p = new Uint8Array(33);
  p.set(data.subarray(0, 33));
  return p;
}

function isPlausibleAck(rx, tx) {
  if (!rx || rx.length < 2) return false;
  if (rx[0] === 0xc0) return true;
  if (rx[1] === 0x00) return true;
  const cmd = tx[1];
  if (rx[0] === cmd || rx[1] === cmd) return true;
  // GET light / rate / dpi — accept any non-empty payload
  if ((tx[1] === 0x15 || tx[1] === 0x12 || tx[1] === 0x13) && rx.some((b, i) => i > 2 && b !== 0))
    return true;
  if (tx[1] === 0x05) return true;
  // any non-trivial response after our write
  if (rx.some((b) => b !== 0)) return true;
  return false;
}

function checksum33(buf) {
  let sum = 0;
  for (let i = 0; i < 9; i++) {
    const p = 6 + i * 3;
    sum = (sum + buf[p - 1] + buf[p] + buf[p + 1]) & 0xff;
  }
  return sum;
}

function hexToBytes(hex) {
  const clean = hex.replace(/\s+/g, "");
  if (clean.length % 2) throw new Error("Invalid hex");
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

// describeReports / scoreDevice already exported above
export { bufToHex };
