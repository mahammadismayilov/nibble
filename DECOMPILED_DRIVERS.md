# Decompiled OEM Drivers & Hardware Knowledge Base

> **LOCAL-ONLY REFERENCE FILE**: This document tracks all decompiled OEM mouse driver installers, extracted micro-controller firmware schemas, and protocol structures. **Do NOT commit or push to GitHub.**

---

## 🗂️ Registry of Decompiled OEM Drivers

### 1. 📂 **AJAZZ AJ179 / AJ179P Dual-Mode Driver (Old Version / 旧版)**
* **Local Source Path**: `C:\Users\Mehem\Downloads\AJAZZ_AJ179_AJ179P_Dual-mode_PAW3395_Windows_Only_Mouse_Driver\...`
* **Target Hardware**: AJAZZ AJ179 / AJ179P Dual-Mode (2.4G Wireless + Wired USB)
* **Micro-Controller**: **CompX Solution** (`VID 0x248A` / `0x249A`)
* **Product IDs (PIDs)**: `0x5C2E` (Wired USB), `0x5C2F` (2.4G Receiver)
* **Extracted Insights**:
  - Core CompX 33-byte HID packet layout.
  - Byte `0x20` checksum calculation formula (`sum of 9 triplets & 0xFF`).
  - 1000Hz polling rate bInterval register values (`0`: 125Hz, `1`: 250Hz, `2`: 500Hz, `3`: 1000Hz).

---

### 2. 📂 **AJAZZ AJ179 / AJ179P Tri-Mode Driver (New Version / 新版)**
* **Local Source Path**: `C:\Users\Mehem\Downloads\AJAZZ_AJ179_AJ179P_Dual-mode_PAW3395_Windows_Only_Mouse_Driver\AJAZZ_【AJ179】【AJ179P】(双模_PAW3395)_Win系统_鼠标驱动\【第一版(新版)】AJAZZ_【AJ179】【AJ179P】(三模_PAW3395)_Win系统_鼠标驱动`
* **Target Hardware**: AJAZZ AJ179 / AJ179P Tri-Mode (Bluetooth 5.3 + 2.4G Wireless + Wired USB)
* **Micro-Controller**: **CompX Solution** (`VID 0x248A` / `0x249A`)
* **Product IDs (PIDs)**: `0x5D2E` (Tri-mode Wired), `0x5E2F` (Tri-mode 2.4G)
* **Extracted Insights**:
  - Tri-mode Bluetooth 5.3 firmware register state reporting.
  - Extended 50-step DPI granularity registers up to 26,000 DPI.

---

### 3. 📦 **AJAZZ Driver (R) Setup 2.1.94 (April 2025 Release)**
* **Local Source Path**: `C:\Users\Mehem\Downloads\AJ159_PRO_PAW3395_Win_driver\【AJ159 PRO】(三模_PAW3395)_Win_鼠标驱动\AJAZZ Driver（R）_setup_2.1.94(WIN20250417).exe`
* **Extracted Scratch Path**: `scratch/app_extracted/resources/app/dist/static/js/main_ace02afb.js`
* **Target Hardware**: AJAZZ AJ159 PRO, AJ159 APEX, AJ179 APEX, Meetion 8K, YC3121 Hall Docks
* **Micro-Controller**: **Yichip / SinoWealth** (`VID 0x3151`), **YC3121** (`VID 0x3121`)
* **Product IDs (PIDs)**:
  - `0x402D`: AJ159 PRO 2.4G 8K Dongle
  - `0x4026`: AJ159 PRO Wired Cable
  - `0x5007`: AJ159 APEX / AJ179 APEX 8K Screen Dock
  - `0x502D`: AJ159 APEX / AJ179 APEX 8K Screen Dock (Alt)
  - `0x5008`: Magnetic Charging Base (Capped at 1000Hz)
* **Extracted Insights**:
  - Official 7-stage polling rate array: `[125, 250, 500, 1000, 2000, 4000, 8000]`.
  - 8K high-speed report rate index wire encoding (`4` = 2000Hz, `5` = 4000Hz, `6` = 8000Hz).
  - Explicit Charging Stand cap check: `pid === 20488 ? 1000Hz : 8000Hz`.
  - Full hardware configuration tables for 13 Vendor IDs (`VID 3151`, `VID 25A7`, `VID 3121`, `VID 347A`, `VID 0C45`).

---

## 🧬 Micro-Controller Architecture Reference

| Vendor ID (Hex) | Vendor ID (Dec) | Driver Engine | Known Brand Models |
| :--- | :--- | :--- | :--- |
| **`0x248A` / `0x249A`** | `9354` / `9370` | CompX | AJAZZ AJ179, AJ159, AJ139 Pro, Attack Shark X3 |
| **`0x3151`** | `12625` | Yichip / SinoWealth | AJAZZ AJ159 APEX, AJ159 PRO, AJ179 APEX |
| **`0x3121`** | `12577` | YC3121 MCU | AJAZZ Color Screen GIF Docks |
| **`0x25A7`** | `9639` | CompX / EVision | Attack Shark R1, VXE Dragonfly F1 |
| **`0x347A`** | `13434` | Telink | Ultra-low Latency 8K Dongles |
