# Nibble GPUI prototype

An isolated interface preview for AJ179 / AJ179P CompX receivers. By default it
reads battery, DPI, and polling rate without changing settings. It does not
replace the desktop app or save profiles. Close other mouse software before
testing because concurrent HID reads can interfere.

Build and run on Windows with a Rust MSVC toolchain and the Windows SDK:

```powershell
cargo run --release
```

The prototype reports one snapshot and closes the HID handle. Use Refresh to
read again. It does not continuously poll or reconnect.

For visual comparisons without accessing a receiver, run the executable with
`--demo`. This mode displays sample data and disables receiver scans.

`--experimental-write` enables the DPI-stage and report-rate selectors. The
app reads the current settings before sending a change and checks the response
afterward. These writes have packet tests but have not yet been tested on a
physical mouse. Other settings are still previews.

