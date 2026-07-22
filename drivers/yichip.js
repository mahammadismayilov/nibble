import { DriverPlugin } from "./base.js";

/**
 * Yichip / SinoWealth / YZW Micro-Controller Driver Plugin (VID 0x3151).
 * Encapsulates 65-byte feature reports, 8K Hz rate maps, and screen dock commands.
 */
export class YichipDriver extends DriverPlugin {
  constructor() {
    super("yichip", "Yichip Solution", [0x3151]);
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
    // 0=125Hz(8), 1=250Hz(4), 2=500Hz(2), 3=1000Hz(1), 4=2000Hz(4), 5=4000Hz(5), 6=8000Hz(6)
    const rateMap = { 0: 8, 1: 4, 2: 2, 3: 1, 4: 4, 5: 5, 6: 6 };
    buf[5] = rateMap[rateIndex] !== undefined ? rateMap[rateIndex] : rateIndex;
    return this.finalize(buf);
  }
}

export const yichipDriver = new YichipDriver();
