# Nibble GPUI prototype

An isolated, read-only experiment for the AJ179P receiver `249A:5C2F`.
It opens the configuration interface once and sends the existing status query.
It does not change mouse settings, replace the desktop app, or save profiles.
Close other mouse software before testing because concurrent HID reads can interfere.

Build and run on Windows with a Rust MSVC toolchain and the Windows SDK:

```powershell
cargo run --release
```

The prototype reports a single status reading. It does not poll or reconnect.

