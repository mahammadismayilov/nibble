param([switch]$Reference)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class CaptureWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
}
'@

$exe = if ($Reference) { Join-Path $env:GITHUB_WORKSPACE 'capture-input/nibble-reference.exe' }
    else { Join-Path $env:GITHUB_WORKSPACE 'capture-input/target/release/nibble-gpui-prototype.exe' }
$output = Join-Path $env:GITHUB_WORKSPACE 'visual-captures'
New-Item -ItemType Directory -Force -Path $output | Out-Null
if (-not (Test-Path -LiteralPath $exe)) { throw "Build artifact does not contain the GPUI executable at $exe" }

$tabs = if ($Reference) { @('tauri-home', 'tauri-keys', 'tauri-dpi', 'tauri-lighting', 'tauri-macro', 'tauri-settings', 'tauri-about') }
    else { @('home', 'keys', 'dpi', 'lighting', 'macro', 'settings', 'about', 'home-light', 'keys-light', 'dpi-light', 'lighting-light', 'macro-light', 'settings-light', 'about-light') }
foreach ($tab in $tabs) {
    $stdout = Join-Path $output "$tab.stdout.txt"
    $stderr = Join-Path $output "$tab.stderr.txt"
    if ($Reference) {
        $process = Start-Process -FilePath $exe -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    } else {
        $argsForDemo = @('--demo', "--demo-tab=$($tab -replace '-light$', '')")
        if ($tab.EndsWith('-light')) { $argsForDemo += '--demo-light' }
        $process = Start-Process -FilePath $exe -ArgumentList $argsForDemo -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    }
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        do {
            Start-Sleep -Milliseconds 500
            $process.Refresh()
            if ($process.HasExited) { throw "GPUI exited before opening the $tab window. See $tab.stderr.txt." }
        } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
        if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw "GPUI did not open the $tab window within 30 seconds." }
        if ($Reference) { Start-Sleep -Seconds 10 } else { Start-Sleep -Seconds 3 }
        $rect = New-Object CaptureWindow+Rect
        if (-not [CaptureWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw "Could not get the $tab window bounds." }
        $width = $rect.Right - $rect.Left
        $height = $rect.Bottom - $rect.Top
        if ($width -lt 600 -or $height -lt 400) { throw "Unexpected $tab window size: ${width}x${height}." }
        if ($Reference -and $tab -ne 'tauri-home') {
            $tabX = @{ 'tauri-keys' = 124; 'tauri-dpi' = 181; 'tauri-lighting' = 250; 'tauri-macro' = 323; 'tauri-settings' = 397; 'tauri-about' = 470 }[$tab]
            if (-not [CaptureWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw "Could not focus the Tauri window for $tab." }
            if (-not [CaptureWindow]::SetCursorPos($rect.Left + $tabX, $rect.Top + 119)) { throw "Could not move the pointer for $tab." }
            [CaptureWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
            [CaptureWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Seconds 2
        }
        $bitmap = [System.Drawing.Bitmap]::new($width, $height)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $dc = $graphics.GetHdc()
            try { $printed = [CaptureWindow]::PrintWindow($process.MainWindowHandle, $dc, 2) }
            finally { $graphics.ReleaseHdc($dc) }
            if (-not $printed) { throw "Windows could not capture the $tab window." }
            $colors = [System.Collections.Generic.HashSet[int]]::new()
            for ($y = 0; $y -lt $height; $y += 17) {
                for ($x = 0; $x -lt $width; $x += 17) { [void]$colors.Add($bitmap.GetPixel($x, $y).ToArgb()) }
            }
            $bitmap.Save((Join-Path $output "$tab.png"), [System.Drawing.Imaging.ImageFormat]::Png)
            if ($colors.Count -lt 8) { throw "The $tab capture appears blank ($($colors.Count) sampled colors)." }
            Write-Output "$tab capture: ${width}x${height}, $($colors.Count) sampled colors"
        } finally {
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    } finally {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    }
}

