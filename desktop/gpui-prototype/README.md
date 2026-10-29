# Nibble GPUI prototype

An isolated interface preview for AJ179 / AJ179P CompX receivers. By default it
reads battery, DPI, polling rate, and supported lighting, keymap, and sensor states
without changing settings. It does not replace the desktop app. Close other mouse software before
testing because concurrent HID reads can interfere.
Some models share USB identifiers. When a status response identifies a different
model, the preview refuses to configure it. If the response contains no model
identity, the header shows that the model is unconfirmed.

Build and run on Windows with a Rust MSVC toolchain and the Windows SDK:

```powershell
cargo run --release
```

The prototype reports one snapshot and closes the HID handle. Use Refresh to
read again. It does not continuously poll or reconnect. Appearance selection is
saved separately from the original desktop app.

AJ179 / AJ179P profiles can be imported and exported in the original app's JSON
format. Profiles and local preferences are stored separately from the Tauri app.
Selecting or importing a profile currently updates local state only; it does not
apply the profile to the mouse. Profile settings are not yet synchronized with
all controls, and other mouse models are not supported by this preview.
The DPI color swatch edits the selected profile's color as #RRGGBB. In experimental
mode, Apply profile colors sends all six profile colors to the mouse and updates
the active lighting color. The receiver cannot reliably confirm the color table.

For visual comparisons without accessing a receiver, run the executable with
`--demo`. This mode displays sample data and disables receiver scans.

`--experimental-write` enables DPI, report-rate, sensor, keymap, lighting, and power
controls. For DPI, report rate, sensor, keymap, and lighting changes, the app
reads current settings before sending a change and checks the response afterward.
A stage switch and restoration were tested on an AJ179P;
other writes have packet tests but have not been tested on physical hardware.
Reset all keys requires confirmation and restores the default button map.
Left-click remains locked. Lighting may require a previously read color to turn
back on after the mouse reports an off state. Other settings are still previews.
Power settings are sent from the selected profile without device readback.

