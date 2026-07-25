import {
  buildReportRate,
  buildDpi,
  buildDpiRgb,
  buildLight,
  buildSolidCapture,
  buildOffCapture,
  buildBreatheCapture,
  buildSensor,
  buildPower,
  buildKeyMap,
  buildStatusQuery,
  buildReportRateGet,
  parseReportRateResponse,
  buildDpiGet,
  parseDpiResponse,
  buildSensorGet,
  parseSensorResponse,
  buildKeyMapGet,
  parseKeyMapResponse,
  buildLightGet,
  parseLightResponse,
  parseStatus,
  lightIdToWire,
  uiBrightnessToWire,
  uiSpeedToWire,
} from "../protocol.js";

/**
 * Base DriverPlugin interface for peripheral protocol engines.
 * Abstract driver class extended by CompX, Yichip, and future MCU drivers.
 */
export class DriverPlugin {
  constructor(id, name, vids, options = {}) {
    this.id = id;
    this.name = name;
    this.vids = vids;
    this.options = {
      allowNoReply: options.allowNoReply ?? true,
      timeoutMs: options.timeoutMs ?? 900,
      retries: options.retries ?? 2,
      preferStrip1: options.preferStrip1 ?? true,
      ...options,
    };
  }

  /**
   * Check if this driver supports the given vendor ID.
   */
  supportsVendor(vid) {
    const num = typeof vid === "string" ? parseInt(vid.replace("0x", ""), 16) : vid;
    return this.vids.includes(num);
  }

  /**
   * Returns WebHID transfer options tailored to this hardware driver & scope.
   */
  getTransferOptions(scope = "general", overrides = {}) {
    return {
      timeoutMs: this.options.timeoutMs,
      retries: this.options.retries,
      preferStrip1: this.options.preferStrip1,
      allowNoReply: this.options.allowNoReply,
      ...overrides,
    };
  }

  /** Status / battery query packet */
  buildStatusQuery() {
    throw new Error("buildStatusQuery not implemented");
  }

  /** Set DPI & active stage packet */
  buildSetDpi(dpiStages, activeIndex, settings, sensorType) {
    return buildDpi(dpiStages, activeIndex, sensorType);
  }

  buildSetDpiRgb(colors) { return buildDpiRgb(colors); }

  /** Set Report Rate / Polling Hz packet */
  buildSetReportRate(rateIndex) {
    return buildReportRate(rateIndex);
  }

  /** Set Keymap table packet */
  buildSetKeymap(keyFuncs) {
    return buildKeyMap(keyFuncs);
  }

  /** Set Light / RGB effect packet */
  buildSetLight(lightId, lightConfig) {
    return buildLight(lightId, lightConfig);
  }

  buildLightFromUi(profile) {
    const brightness = uiBrightnessToWire(profile.light.brightness);
    const speed = uiSpeedToWire(profile.light.speed);
    const dpiColor = profile.dpiStages[profile.activeDpi]?.color || "#ff0000";
    const color = profile.settings?.batteryRgbSync && profile.light.color
      ? profile.light.color
      : (profile.light.color || dpiColor);
    if (profile.light.mode === "solid") return buildSolidCapture({ brightness, speed, color, allowZero: true });
    if (profile.light.mode === "off") return buildOffCapture();
    if (profile.light.mode === "breathe") return buildBreatheCapture({ brightness, speed, color, useRainbow: false, allowZero: true });
    return this.buildSetLight(lightIdToWire(profile.light.mode), { brightness, speed, color, off: false, useRainbow: false, allowZero: true });
  }

  buildSetSensor(settings) { return buildSensor(settings); }
  buildSetPower(settings) { return buildPower(settings); }
  buildGetReportRate() { return buildReportRateGet(); }
  buildGetDpi() { return buildDpiGet(); }
  buildGetKeymap() { return buildKeyMapGet(); }
  buildGetSensor() { return buildSensorGet(); }
  buildGetLight() { return buildLightGet(); }

  /** Parse status response buffer */
  parseStatus(payload) {
    return parseStatus(payload);
  }
  parseReportRate(payload) { return parseReportRateResponse(payload); }
  parseDpi(payload, sensorType) { return parseDpiResponse(payload, sensorType); }
  parseKeymap(payload) { return parseKeyMapResponse(payload); }
  parseSensor(payload) { return parseSensorResponse(payload); }
  parseLight(payload) { return parseLightResponse(payload); }
}
