# AGENT.md — Nibble Long-Term Memory & Guidelines

> **IMPORTANT FOR ALL AGENTS**: Read this file at the start of every session. Maintain and update this memory log whenever new hardware IDs, protocol quirks, user preferences, or architecture decisions are established.

---

## 🧠 Long-Term Memory & Technical Knowledge Base

### 1. Hardware & WebHID Protocols
- **CompX Micro-Controllers**:
  - `VID_248A` (`0x248A`) & `VID_249A` (`0x249A`)
  - Product IDs (PIDs): `0x5C2E`, `0x5D2E`, `0x5E2E`, `0x5C2F`
- **AJAZZ AJ159 Series (Yichip / SinoWealth Micro-Controllers)**:
  - `VID_3151` (`0x3151`)
  - Product IDs (PIDs):
    - `0x5007` (AJ159 APEX 2.4G 8K Screen Dock)
    - `0x502D` (AJ159 APEX 2.4G 8K Dock Alt)
    - `0x402D` (AJ159 PRO 2.4G 8K Dongle)
    - `0x4026` (AJ159 PRO Wired Cable)
  - *Note*: Keyboard/Trackpad HID entries like `VID_3537` (`0x3537` / `PID 1093`) are NOT mouse hardware IDs and must be excluded from mouse protocol filters.
- **WebHID Rules**:
  - Always add both Vendor IDs (`VIDS`) and Product IDs (`PIDS`) to `protocol.js` array exports.
  - Do NOT filter out wired or wireless mode interfaces when users report connectivity issues.
  - Do NOT enforce "Wireless Receiver only" popups; mice work via direct USB cable or 2.4G wireless dongle/dock.

### 2. Supported Mouse Profiles & Specs
- **AJ159 APEX**: PAW3950 APEX, 8K Wireless Polling Rate (125-8000Hz), 30K/42K OC DPI, Color Screen GIF Dock.
- **AJ159 PRO**: PAW3395, 8K Wireless Polling Rate, 26K DPI, Magnetic 8K Dock.
- **AJ159 / AJ159P / AJ159 MC**: PAW3395, 1K Wireless Polling Rate, 26K DPI, Nano Receiver / Charging Stand.
- **AJ179 APEX**: Ergonomic right-handed PAW3950 APEX, 8K Wireless, Color Screen GIF Dock.
- **AJ179 / AJ179P**: Ergonomic right-handed PAW3395, 1K Wireless, Magnetic Charging Dock.
- **AJ139 Pro**: Symmetrical PAW3395, 1K Wireless, Standard Nano Receiver.

### 3. Verification & Deployment Workflow Rules
- **DO NOT `git push` prematurely!**
  - Always ask or confirm before pushing commits to GitHub/Vercel (`nibble` app repo or `nibble-website` repo).
  - Test and verify changes locally first.
- **Database Status Tags**:
  - `Verified Working` (Green): Reserved ONLY for models physically verified by developers or users with confirmed hardware IDs.
  - `Protocol Supported` (Blue): Default for models supported by protocol specs but awaiting physical user testing.
  - `Community Test Pending` (Amber): Used for models actively under community hardware ID validation (e.g. AJ159 APEX).

### 4. Website & UX Design Rules
- **No Custom Cursors**:
  - Never add custom/gooey cursor JavaScript loops (`requestAnimationFrame`) or `cursor: none !important` CSS rules. Keep native OS cursors clean across all pages.
- **Database Catalog Layout (`/devices`)**:
  - Use a spacious **Vertical List Layout** (`devices-vertical-list`) with large 140px x 140px sharp mouse thumbnails (`device-image-col`).
  - Do NOT use 3D grid cards or `transform: translateY(-4px)` hover animations on catalog items.
- **Container Centering**:
  - Ensure all layout sections preserve explicit horizontal auto margins (`margin-left: auto; margin-right: auto;`) on `.container` elements to avoid left-alignment bugs.
