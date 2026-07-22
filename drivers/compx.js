import { DriverPlugin } from "./base.js";

/**
 * CompX Micro-Controller Driver Plugin (VID 0x248A / 0x249A).
 * Encapsulates CompX 33-byte HID packets & checksum logic.
 */
export class CompXDriver extends DriverPlugin {
  constructor(options = {}) {
    super("compx", "CompX Solution", [0x248a, 0x249a], {
      allowNoReply: true,
      timeoutMs: 900,
      retries: 2,
      preferStrip1: true,
      ...options,
    });
  }

  checksum(buf) {
    let sum = 0;
    for (let i = 0; i < 9; i++) {
      const p = 6 + i * 3;
      sum = (sum + buf[p - 1] + buf[p] + buf[p + 1]) & 0xff;
    }
    return sum;
  }

  finalize(buf) {
    buf[0x20] = this.checksum(buf);
    return buf;
  }

  setHeader(buf, cmd) {
    buf[0] = 0x00;
    buf[1] = cmd & 0xff;
    buf[2] = 0x00;
    buf[3] = 0x01;
  }

  buildStatusQuery() {
    const buf = new Uint8Array(0x21);
    buf[0] = 0x00;
    buf[1] = 0x10; // STATUS CMD
    return this.finalize(buf);
  }

  buildSetReportRate(rateIndex) {
    const buf = new Uint8Array(0x41);
    this.setHeader(buf, 0x02);
    buf[4] = 1;
    const rateMap = { 0: 8, 1: 4, 2: 2, 3: 1 };
    buf[5] = rateMap[rateIndex] !== undefined ? rateMap[rateIndex] : 1;
    return this.finalize(buf);
  }
}

export const compxDriver = new CompXDriver();
