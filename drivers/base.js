/**
 * Base DriverPlugin interface for peripheral protocol engines.
 * Abstract driver class extended by CompX, Yichip, and future MCU drivers.
 */
export class DriverPlugin {
  constructor(id, name, vids) {
    this.id = id;
    this.name = name;
    this.vids = vids;
  }

  /**
   * Check if this driver supports the given vendor ID.
   */
  supportsVendor(vid) {
    const num = typeof vid === "string" ? parseInt(vid.replace("0x", ""), 16) : vid;
    return this.vids.includes(num);
  }

  /** Status / battery query packet */
  buildStatusQuery() {
    throw new Error("buildStatusQuery not implemented");
  }

  /** Set DPI & active stage packet */
  buildSetDpi(dpiStages, activeIndex, settings, sensorType) {
    throw new Error("buildSetDpi not implemented");
  }

  /** Set Report Rate / Polling Hz packet */
  buildSetReportRate(rateIndex) {
    throw new Error("buildSetReportRate not implemented");
  }

  /** Set Keymap table packet */
  buildSetKeymap(keyFuncs) {
    throw new Error("buildSetKeymap not implemented");
  }

  /** Set Light / RGB effect packet */
  buildSetLight(lightId, lightConfig) {
    throw new Error("buildSetLight not implemented");
  }

  /** Parse status response buffer */
  parseStatus(payload) {
    throw new Error("parseStatus not implemented");
  }
}
