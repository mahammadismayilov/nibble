# Nibble GPUI prototype

An isolated interface preview for AJ179 / AJ179P CompX receivers. By default it
reads battery, DPI, polling rate, and supported lighting, keymap, and sensor states
without changing settings. It does not replace the desktop app or save profiles. Close other mouse software before
testing because concurrent HID reads can interfere.

Build and run on Windows with a Rust MSVC toolchain and the Windows SDK:

```powershell
cargo run --release
```

The prototype reports one snapshot and closes the HID handle. Use Refresh to
read again. It does not continuously poll or reconnect. Appearance selection is
saved separately from the original desktop app.

For visual comparisons without accessing a receiver, run the executable with
`--demo`. This mode displays sample data and disables receiver scans.

`--experimental-write` enables DPI, report-rate, sensor, keymap, and lighting
controls. The app reads current settings before sending a change and checks the
response afterward. A stage switch and restoration were tested on an AJ179P;
other writes have packet tests but have not been tested on physical hardware.
Left-click remains locked. Lighting may require a previously read color to turn
back on after the mouse reports an off state. Other settings are still previews.

