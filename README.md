# Nibble

Browser-based WebHID mouse configurator for PAW3395-class wireless mice.

Configure DPI, polling rate, lighting, keys, and power — no Windows installer, no OEM bloat.

## Usage

Open **`index.html`** in **Chrome or Edge** (WebHID required).  
Close other mouse drivers first. Connect your receiver and pick **Wireless-Receiver / config** (not mouse/keyboard).

### Run locally

```bash
python -m http.server 8080
```

Open http://localhost:8080.

## Supported devices

Primarily AJ179 / AJ179P (PAW3395) and related 248A/249A receivers.

## License

MIT — see [LICENSE](./LICENSE).

---

Unofficial community project. Not affiliated with any mouse OEM.  
Protocol derived from **AJAZZ Driver (X) 1.0.1.4** (2024.01.04).
