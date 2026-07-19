# Nibble

Browser-based WebHID mouse configurator for PAW3395-class wireless mice.

Configure DPI, polling rate, lighting, keys, and power — no Windows installer, no OEM bloat.

## Download

[Download the latest release](https://github.com/mahammadismayilov/nibble/releases) — extract anywhere, no install.

Or clone the repo:

```bash
git clone https://github.com/mahammadismayilov/nibble.git
```

## Quick start

**You need Chrome or Edge** — WebHID doesn't work in Firefox or Safari.

### Windows

Double-click `start.bat` or run:

```bash
python -m http.server 8080
```

### macOS / Linux

```bash
chmod +x start.sh
./start.sh
# or: python3 -m http.server 8080
```

Then open **http://localhost:8080** in Chrome or Edge.

### First connection

1. Close any OEM mouse software (AJAZZ, etc.)
2. Plug in your wireless receiver
3. Click **Connect** in Nibble
4. Pick **Wireless-Receiver / config** (not mouse/keyboard)

### First connection

1. Close any OEM mouse software (AJAZZ, etc.)
2. Plug in your wireless receiver
3. Click **Connect** in Nibble
4. Pick **Wireless-Receiver / config** (not mouse/keyboard)

## Supported devices

Primarily AJ179 / AJ179P (PAW3395) and related 248A/249A receivers.

## Why a local server?

WebHID requires a secure context (HTTPS or localhost). You can't just open `index.html` from the file system — you need a local HTTP server. The scripts above handle that.

## License

MIT — see [LICENSE](./LICENSE).

---

Unofficial community project. Not affiliated with any mouse OEM.  
Protocol derived from **AJAZZ Driver (X) 1.0.1.4** (2024.01.04).
