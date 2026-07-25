# Nibble Apps

This repository contains both Nibble applications:

```text
webapp/   Browser-based WebHID mouse configurator
desktop/  Native Tauri companion, built from the webapp frontend
```

## Web app

```bash
cd webapp
npm test
python -m http.server 8080
```

Open `http://localhost:8080` in Chrome or Edge.

## Desktop app

```bash
cd desktop
npm install
npm run tauri dev
```

The desktop app's Vite configuration deliberately uses `../webapp`, so the
browser and native releases always run the same configurator code.
