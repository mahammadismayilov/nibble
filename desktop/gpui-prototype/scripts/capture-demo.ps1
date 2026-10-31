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
        $boundsDeadline = [DateTime]::UtcNow.AddSeconds(30)
        do {
            $process.Refresh()
            if ($process.HasExited) { throw "The $tab app exited before its window was ready." }
            if (-not [CaptureWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw "Could not get the $tab window bounds." }
            $width = $rect.Right - $rect.Left
            $height = $rect.Bottom - $rect.Top
            if ($width -ge 600 -and $height -ge 400) { break }
            Start-Sleep -Milliseconds 500
        } while ([DateTime]::UtcNow -lt $boundsDeadline)
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
            if ($tab -eq 'dpi-light' -and $width -eq 1040 -and $height -eq 711) {
                $before = $bitmap.GetPixel(230, 450).ToArgb()
                $beforeGap = $bitmap.GetPixel(720, 220).ToArgb()
                if (-not [CaptureWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus the GPUI demo for the stage-selection check.' }
                if (-not [CaptureWindow]::SetCursorPos($rect.Left + 300, $rect.Top + 455)) { throw 'Could not position the pointer for the stage-selection check.' }
                [CaptureWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
                [CaptureWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
                Start-Sleep -Seconds 2
                $afterBitmap = [System.Drawing.Bitmap]::new($width, $height)
                $afterGraphics = [System.Drawing.Graphics]::FromImage($afterBitmap)
                try {
                    $afterDc = $afterGraphics.GetHdc()
                    try { $afterPrinted = [CaptureWindow]::PrintWindow($process.MainWindowHandle, $afterDc, 2) }
                    finally { $afterGraphics.ReleaseHdc($afterDc) }
                    if (-not $afterPrinted) { throw 'Could not capture the GPUI demo after selecting stage 2.' }
                    $afterBitmap.Save((Join-Path $output 'dpi-light-stage2.png'), [System.Drawing.Imaging.ImageFormat]::Png)
                    if ($afterBitmap.GetPixel(230, 450).ToArgb() -eq $before) { throw 'Selecting stage 2 did not change its highlighted background.' }
                    if ($afterBitmap.GetPixel(720, 220).ToArgb() -ne $beforeGap) { throw 'Selecting stage 2 shifted the report-rate card.' }
                    Write-Output 'Demo stage-selection check: stage 2 highlight changed.'
                } finally {
                    $afterGraphics.Dispose()
                    $afterBitmap.Dispose()
                }
            } elseif ($tab -eq 'dpi-light') {
                throw "Stage-selection check requires the expected 1040x711 runner window, got ${width}x${height}."
            }
            if ($tab -eq 'settings-light') {
                if ($width -ne 1040 -or $height -ne 711) { throw "Settings interaction check requires a 1040x711 window, got ${width}x${height}." }
                if (-not [CaptureWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus the settings demo.' }
                foreach ($position in @(@(466, 230), @(966, 452))) {
                    if (-not [CaptureWindow]::SetCursorPos($rect.Left + $position[0], $rect.Top + $position[1])) { throw 'Could not position the pointer for the settings check.' }
                    [CaptureWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
                    [CaptureWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
                    Start-Sleep -Seconds 1
                }
                $afterSettings = [System.Drawing.Bitmap]::new($width, $height)
                $afterSettingsGraphics = [System.Drawing.Graphics]::FromImage($afterSettings)
                try {
                    $settingsDc = $afterSettingsGraphics.GetHdc()
                    try { $settingsPrinted = [CaptureWindow]::PrintWindow($process.MainWindowHandle, $settingsDc, 2) }
                    finally { $afterSettingsGraphics.ReleaseHdc($settingsDc) }
                    if (-not $settingsPrinted) { throw 'Could not capture the settings after selecting options.' }
                    $afterSettings.Save((Join-Path $output 'settings-light-selected.png'), [System.Drawing.Imaging.ImageFormat]::Png)
                    foreach ($area in @(@('LOD', 440, 210, 494, 249), @('connection', 940, 432, 992, 471))) {
                        $changed = 0
                        for ($y = $area[2]; $y -lt $area[4]; $y++) {
                            for ($x = $area[1]; $x -lt $area[3]; $x++) {
                                if ($bitmap.GetPixel($x, $y).ToArgb() -ne $afterSettings.GetPixel($x, $y).ToArgb()) { $changed++ }
                            }
                        }
                        if ($changed -lt 100) { throw "The $($area[0]) selection did not visibly change ($changed pixels)." }
                        Write-Output "Demo $($area[0]) selection changed $changed pixels."
                    }
                    if (-not [CaptureWindow]::SetCursorPos($rect.Left + 960, $rect.Top + 230)) { throw 'Could not position the pointer for the sleep menu.' }
                    [CaptureWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
                    [CaptureWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
                    Start-Sleep -Seconds 1
                    $menuBitmap = [System.Drawing.Bitmap]::new($width, $height)
                    $menuGraphics = [System.Drawing.Graphics]::FromImage($menuBitmap)
                    try {
                        $menuDc = $menuGraphics.GetHdc()
                        try { $menuPrinted = [CaptureWindow]::PrintWindow($process.MainWindowHandle, $menuDc, 2) }
                        finally { $menuGraphics.ReleaseHdc($menuDc) }
                        if (-not $menuPrinted) { throw 'Could not capture the sleep options.' }
                        $menuBitmap.Save((Join-Path $output 'settings-light-sleep-options.png'), [System.Drawing.Imaging.ImageFormat]::Png)
                        $menuChanged = 0
                        for ($y = 270; $y -lt 300; $y++) {
                            for ($x = 550; $x -lt 640; $x++) {
                                if ($afterSettings.GetPixel($x, $y).ToArgb() -ne $menuBitmap.GetPixel($x, $y).ToArgb()) { $menuChanged++ }
                            }
                        }
                        if ($menuChanged -lt 100) { throw 'The sleep options did not open.' }
                    } finally {
                        $menuGraphics.Dispose()
                        $menuBitmap.Dispose()
                    }
                    if (-not [CaptureWindow]::SetCursorPos($rect.Left + 590, $rect.Top + 280)) { throw 'Could not position the pointer for the sleep option.' }
                    [CaptureWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
                    [CaptureWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
                    Start-Sleep -Seconds 1
                    $sleepBitmap = [System.Drawing.Bitmap]::new($width, $height)
                    $sleepGraphics = [System.Drawing.Graphics]::FromImage($sleepBitmap)
                    try {
                        $sleepDc = $sleepGraphics.GetHdc()
                        try { $sleepPrinted = [CaptureWindow]::PrintWindow($process.MainWindowHandle, $sleepDc, 2) }
                        finally { $sleepGraphics.ReleaseHdc($sleepDc) }
                        if (-not $sleepPrinted) { throw 'Could not capture the selected sleep option.' }
                        $sleepBitmap.Save((Join-Path $output 'settings-light-sleep-selected.png'), [System.Drawing.Imaging.ImageFormat]::Png)
                        $labelChanged = 0
                        for ($y = 210; $y -lt 250; $y++) {
                            for ($x = 915; $x -lt 993; $x++) {
                                if ($afterSettings.GetPixel($x, $y).ToArgb() -ne $sleepBitmap.GetPixel($x, $y).ToArgb()) { $labelChanged++ }
                            }
                        }
                        if ($labelChanged -lt 20) { throw 'Selecting a sleep timeout did not change its displayed value.' }
                        Write-Output "Demo sleep selection changed $labelChanged pixels in the value."
                    } finally {
                        $sleepGraphics.Dispose()
                        $sleepBitmap.Dispose()
                    }
                } finally {
                    $afterSettingsGraphics.Dispose()
                    $afterSettings.Dispose()
                }
            }
        } finally {
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    } finally {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    }
}
