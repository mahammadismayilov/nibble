# Nibble GPUI prototype

An isolated, read-only interface preview for AJ179 / AJ179P CompX receivers.
It recreates the existing desktop layout and reads battery, DPI, and polling rate
from the receiver. Mouse setting controls are visual previews and cannot write.
It does not replace the desktop app or save profiles. Close other mouse software
before testing because concurrent HID reads can interfere.

Build and run on Windows with a Rust MSVC toolchain and the Windows SDK:

```powershell
cargo run --release
```

The prototype reports one snapshot and closes the HID handle. Use Refresh to
read again. It does not continuously poll or reconnect.

