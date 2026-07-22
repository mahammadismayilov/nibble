/**
 * Anonymous Hardware & Session Telemetry Emitter for Nibble.
 * 0 IP logging, 0 tracking cookies, 100% privacy compliant.
 */

const CONVEX_SITE_URL = "https://frugal-kookabura-259.eu-west-1.convex.site";
const TELEMETRY_OPT_OUT_KEY = "nibble_telemetry_opt_out";

function getSessionId() {
  try {
    let id = sessionStorage.getItem("nibble_session_id");
    if (!id) {
      id = "sess_" + Math.random().toString(36).substring(2, 10) + Date.now().toString(36);
      sessionStorage.setItem("nibble_session_id", id);
    }
    return id;
  } catch {
    return "sess_fallback";
  }
}

function detectOS() {
  if (typeof navigator === "undefined") return "Unknown";
  const ua = navigator.userAgent;
  if (/windows/i.test(ua)) return "Windows";
  if (/macintosh|mac os x/i.test(ua)) return "macOS";
  if (/cros/i.test(ua)) return "ChromeOS";
  if (/linux/i.test(ua)) return "Linux";
  if (/android/i.test(ua)) return "Android";
  return "Other";
}

function detectBrowser() {
  if (typeof navigator === "undefined") return "Unknown";
  const ua = navigator.userAgent;
  if (/edg/i.test(ua)) return "Edge";
  if (/chrome|crios/i.test(ua)) return "Chrome";
  if (/firefox/i.test(ua)) return "Firefox";
  return "Other";
}

export function isTelemetryOptedOut() {
  try {
    return localStorage.getItem(TELEMETRY_OPT_OUT_KEY) === "true";
  } catch {
    return false;
  }
}

export function setTelemetryOptOut(optOut) {
  try {
    localStorage.setItem(TELEMETRY_OPT_OUT_KEY, optOut ? "true" : "false");
  } catch {}
}

export async function pingActiveSession() {
  if (isTelemetryOptedOut()) return;
  try {
    await fetch(`${CONVEX_SITE_URL}/api/ping`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        sessionId: getSessionId(),
        operatingSystem: detectOS(),
        browser: detectBrowser(),
      }),
    });
  } catch (e) {
    // Silent catch — telemetry must never break user app flow
  }
}

export async function sendHardwareTelemetry(vidHex, pidHex, productName, isKnownModel) {
  if (isTelemetryOptedOut()) return;
  try {
    await fetch(`${CONVEX_SITE_URL}/api/telemetry`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        sessionId: getSessionId(),
        vid: vidHex,
        pid: pidHex,
        productName: productName || "Unknown HID Device",
        isKnownModel: !!isKnownModel,
      }),
    });
  } catch (e) {
    // Silent catch
  }
}

export async function sendErrorTelemetry(vidHex, pidHex, errorMessage, action) {
  if (isTelemetryOptedOut()) return;
  try {
    await fetch(`${CONVEX_SITE_URL}/api/error`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        sessionId: getSessionId(),
        vid: vidHex || "0x0000",
        pid: pidHex || "0x0000",
        errorMessage: String(errorMessage),
        action: action || "unknown",
      }),
    });
  } catch (e) {
    // Silent catch
  }
}
