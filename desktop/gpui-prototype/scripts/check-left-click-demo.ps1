$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class LeftClickDemoWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
}
'@
$exe = Join-Path $env:GITHUB_WORKSPACE 'capture-input/target/release/nibble-gpui-prototype.exe'
$output = Join-Path $env:GITHUB_WORKSPACE 'visual-captures'
$process = Start-Process -FilePath $exe -ArgumentList @('--demo', '--demo-light', '--demo-tab=settings') -PassThru -WindowStyle Normal -RedirectStandardOutput (Join-Path $output 'left-click.stdout.txt') -RedirectStandardError (Join-Path $output 'left-click.stderr.txt')
function Click-Point([int]$x, [int]$y) {
    if (-not [LeftClickDemoWindow]::SetCursorPos($rect.Left + $x, $rect.Top + $y)) { throw 'Could not position the pointer.' }
    [LeftClickDemoWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    [LeftClickDemoWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Seconds 1
}
function Capture([string]$name) {
    $bitmap = [System.Drawing.Bitmap]::new(1040, 711)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $dc = $graphics.GetHdc()
        try { $ok = [LeftClickDemoWindow]::PrintWindow($process.MainWindowHandle, $dc, 2) }
        finally { $graphics.ReleaseHdc($dc) }
        if (-not $ok) { throw "Could not capture $name." }
        $bitmap.Save((Join-Path $output "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
        return $bitmap
    } finally { $graphics.Dispose() }
}
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do { Start-Sleep -Milliseconds 500; $process.Refresh(); if ($process.HasExited) { throw 'Demo exited early.' } }
    while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Demo did not open.' }
    Start-Sleep -Seconds 3
    $rect = New-Object LeftClickDemoWindow+Rect
    if (-not [LeftClickDemoWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw 'Could not get window bounds.' }
    if (($rect.Right - $rect.Left) -ne 1040 -or ($rect.Bottom - $rect.Top) -ne 711) { throw 'Unexpected demo window size.' }
    if (-not [LeftClickDemoWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus demo.' }
    [void][LeftClickDemoWindow]::SetCursorPos($rect.Left + 800, $rect.Top + 550)
    $wheelDown = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]-480), 0)
    [LeftClickDemoWindow]::mouse_event(0x0800, 0, 0, $wheelDown, [UIntPtr]::Zero)
    Start-Sleep -Seconds 1
    Click-Point 968 655
    Click-Point 130 119
    $before = Capture 'left-click-before'
    Click-Point 850 310
    $pending = Capture 'left-click-pending'
    try {
        $labelChanged = 0; $dialogChanged = 0
        for ($y = 205; $y -lt 250; $y += 2) { for ($x = 540; $x -lt 740; $x += 2) { if ($before.GetPixel($x, $y).ToArgb() -ne $pending.GetPixel($x, $y).ToArgb()) { $labelChanged++ } } }
        for ($y = 255; $y -lt 340; $y += 2) { for ($x = 540; $x -lt 990; $x += 2) { if ($before.GetPixel($x, $y).ToArgb() -ne $pending.GetPixel($x, $y).ToArgb()) { $dialogChanged++ } } }
        if ($labelChanged -gt 5 -or $dialogChanged -lt 100) { throw "Left-click confirmation did not appear without changing the assignment: label=$labelChanged dialog=$dialogChanged." }
        Click-Point 590 316
        $after = Capture 'left-click-confirmed'
        try {
            $labelChanged = 0
            for ($y = 205; $y -lt 250; $y += 2) { for ($x = 540; $x -lt 740; $x += 2) { if ($pending.GetPixel($x, $y).ToArgb() -ne $after.GetPixel($x, $y).ToArgb()) { $labelChanged++ } } }
            if ($labelChanged -lt 10) { throw 'Confirming left-click reassignment did not change the selected assignment.' }
            Write-Output "Demo left-click assignment changed after confirmation ($labelChanged sampled pixels)."
        } finally { $after.Dispose() }
    } finally { $before.Dispose(); $pending.Dispose() }
} finally { if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force } }
