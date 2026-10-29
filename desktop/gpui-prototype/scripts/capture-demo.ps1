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
}
'@

$exe = if ($Reference) { Join-Path $env:GITHUB_WORKSPACE 'capture-input/nibble-reference.exe' }
    else { Join-Path $env:GITHUB_WORKSPACE 'capture-input/target/release/nibble-gpui-prototype.exe' }
$output = Join-Path $env:GITHUB_WORKSPACE 'visual-captures'
New-Item -ItemType Directory -Force -Path $output | Out-Null
if (-not (Test-Path -LiteralPath $exe)) { throw "Build artifact does not contain the GPUI executable at $exe" }

$tabs = if ($Reference) { @('tauri-home') } else { @('home', 'keys', 'dpi', 'lighting', 'macro', 'settings', 'about') }
foreach ($tab in $tabs) {
    $stdout = Join-Path $output "$tab.stdout.txt"
    $stderr = Join-Path $output "$tab.stderr.txt"
    if ($Reference) {
        $process = Start-Process -FilePath $exe -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    } else {
        $process = Start-Process -FilePath $exe -ArgumentList @('--demo', "--demo-tab=$tab") -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
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

