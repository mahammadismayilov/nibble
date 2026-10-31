$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProfileResetWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extraInfo);
}
'@
$exe = Join-Path $env:GITHUB_WORKSPACE 'capture-input/target/release/nibble-gpui-prototype.exe'
$output = Join-Path $env:GITHUB_WORKSPACE 'visual-captures'
$originalLocalAppData = $env:LOCALAPPDATA
$env:LOCALAPPDATA = Join-Path $env:RUNNER_TEMP ('nibble-profile-reset-' + [guid]::NewGuid().ToString())
$profileDir = Join-Path $env:LOCALAPPDATA 'Nibble-gpui'
New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $profileDir 'theme'), 'light')
$profile = @{
    name = 'First Profile'; deviceId = 'aj179'; activeDpi = 1; dpiStageCount = 6; reportRateIndex = 3
    dpiStages = @(@{value=400;color='#FF0000'}, @{value=800;color='#00FF00'}, @{value=1200;color='#0000FF'}, @{value=1600;color='#00FFFF'}, @{value=2400;color='#FFFF00'}, @{value=3200;color='#800080'})
    light = @{mode='breathe';brightness=100;speed=5;color='#00FF00'}
    keys = @{'201'='left';'202'='right';'203'='middle';'204'='forward';'205'='backward';'206'='dpi_loop'}
    settings = @{lod='low';angleSnap=$false;ripple=$false;sleepWire=30;moveWake=$true;moveCloseLight=$false;debounce=4;lmbLock=$true;fineDpi=$false}
}
$second = $profile.Clone(); $second.name = 'Second Profile'; $second.activeDpi = 2; $second.reportRateIndex = 2
$profilePath = Join-Path $profileDir 'profiles.json'
$original = @{profileIndex=1;profiles=@($profile,$second)} | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($profilePath, $original, [Text.UTF8Encoding]::new($false))
function Start-TestApp([string]$name) {
    $proc = Start-Process -FilePath $exe -PassThru -WindowStyle Normal -RedirectStandardOutput (Join-Path $output "$name.stdout.txt") -RedirectStandardError (Join-Path $output "$name.stderr.txt")
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do { Start-Sleep -Milliseconds 500; $proc.Refresh(); if ($proc.HasExited) { throw 'Profile test app exited early.' } }
    while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Profile test app did not open.' }
    Start-Sleep -Seconds 4
    return $proc
}
function Click-Point([int]$x, [int]$y) {
    if (-not [ProfileResetWindow]::SetCursorPos($rect.Left + $x, $rect.Top + $y)) { throw 'Could not position the pointer.' }
    [ProfileResetWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    [ProfileResetWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Seconds 1
}
function Capture-ProfileWindow([string]$name) {
    $bitmap = [System.Drawing.Bitmap]::new(1040, 711)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $dc = $graphics.GetHdc()
        try { $printed = [ProfileResetWindow]::PrintWindow($process.MainWindowHandle, $dc, 2) }
        finally { $graphics.ReleaseHdc($dc) }
        if (-not $printed) { throw "Could not capture $name." }
        $bitmap.Save((Join-Path $output "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
        return $bitmap
    } finally { $graphics.Dispose() }
}
try {
    $process = Start-TestApp 'profile-reset-first'
    try {
        $rect = New-Object ProfileResetWindow+Rect
        if (-not [ProfileResetWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw 'Could not read profile test bounds.' }
        if (($rect.Right - $rect.Left) -ne 1040 -or ($rect.Bottom - $rect.Top) -ne 711) { throw 'Unexpected profile test window size.' }
        if (-not [ProfileResetWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus profile test window.' }
        $initial = Capture-ProfileWindow 'profile-reset-before'
        Click-Point 420 119
        [void][ProfileResetWindow]::SetCursorPos($rect.Left + 800, $rect.Top + 550)
        $wheelDown = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]-1200), 0)
        [ProfileResetWindow]::mouse_event(0x0800, 0, 0, $wheelDown, [UIntPtr]::Zero)
        Start-Sleep -Seconds 1
        Click-Point 935 452
        if ([IO.File]::ReadAllText($profilePath) -ne $original) { throw 'Opening confirmation changed saved profiles.' }
        Click-Point 835 452
        if ([IO.File]::ReadAllText($profilePath) -ne $original) { throw 'Cancelling confirmation changed saved profiles.' }
        Click-Point 935 452
        Click-Point 935 452
        $saved = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
        if ($saved.profileIndex -ne 0 -or $saved.profiles.Count -ne 1 -or $saved.profiles[0].name -ne 'Default Profile') { throw 'Confirmed reset did not save the default profile.' }
    } finally { if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force } }
    $process = Start-TestApp 'profile-reset-restart'
    try {
        $saved = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
        if ($saved.profileIndex -ne 0 -or $saved.profiles.Count -ne 1 -or $saved.profiles[0].name -ne 'Default Profile') { throw 'Reset profiles did not survive restart.' }
        $restarted = Capture-ProfileWindow 'profile-reset-after-restart'
        try {
            $changes = 0; $savedValuesChanged = 0
            for ($y = 57; $y -lt 78; $y++) { for ($x = 850; $x -lt 980; $x++) {
                if ($initial.GetPixel($x, $y).ToArgb() -ne $restarted.GetPixel($x, $y).ToArgb()) { $changes++ }
            } }
            for ($y = 220; $y -lt 300; $y += 2) { for ($x = 520; $x -lt 850; $x += 2) {
                if ($initial.GetPixel($x, $y).ToArgb() -ne $restarted.GetPixel($x, $y).ToArgb()) { $savedValuesChanged++ }
            } }
            if ($changes -lt 30) { throw 'Restart did not display the reset profile.' }
            if ($savedValuesChanged -lt 20) { throw 'Offline home values did not follow the saved profile.' }
        } finally { $restarted.Dispose(); $initial.Dispose() }
        Write-Output 'Profile reset preserved the file until confirmation and persisted the default profile after restart.'
        $rect = New-Object ProfileResetWindow+Rect
        if (-not [ProfileResetWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw 'Could not read the restarted window bounds.' }
        if (-not [ProfileResetWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus the restarted window.' }
        Click-Point 190 119
        Click-Point 140 500
        Click-Point 810 370
        Click-Point 160 602
        foreach ($key in @(0x31, 0x34, 0x30, 0x30, 0x0d)) {
            [ProfileResetWindow]::keybd_event([byte]$key, 0, 0, [UIntPtr]::Zero)
            [ProfileResetWindow]::keybd_event([byte]$key, 0, 2, [UIntPtr]::Zero)
        }
        Start-Sleep -Seconds 1
        Click-Point 337 197
        $edited = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
        if ($edited.profiles[0].activeDpi -ne 2 -or $edited.profiles[0].dpiStageCount -ne 3 -or $edited.profiles[0].reportRateIndex -ne 2 -or $edited.profiles[0].dpiStages[2].value -ne 1400) { throw 'Offline DPI or report-rate edits were not saved.' }
        if ($edited.profiles[0].dpiStages[0].value -ne 400 -or $edited.profiles[0].settings.debounce -ne 4) { throw 'Offline editing changed unrelated profile values.' }
        $offlineShot = Capture-ProfileWindow 'profile-offline-dpi-edited'
        $offlineShot.Dispose()
        Write-Output 'Offline DPI stage, value, count, and report rate were saved without changing unrelated profile values.'
        Click-Point 420 119
        Click-Point 460 230
        Click-Point 470 303
        Click-Point 480 452
        Click-Point 970 304
        $edited = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
        if ($edited.profiles[0].settings.lod -ne 'high' -or -not $edited.profiles[0].settings.angleSnap) { throw 'Offline sensor settings were not saved.' }
        if ($edited.profiles[0].settings.debounce -ne 5 -or $edited.profiles[0].settings.moveWake -ne $false) { throw 'Offline power settings were not saved.' }
        if ($edited.profiles[0].dpiStages[2].value -ne 1400) { throw 'Offline settings editing changed an unrelated DPI value.' }
        Write-Output 'Offline sensor and power settings were saved without changing DPI.'
        Click-Point 260 119
        Click-Point 900 255
        Click-Point 735 410
        $edited = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
        if ($edited.profiles[0].light.mode -ne 'solid' -or $edited.profiles[0].light.brightness -ne 50) { throw 'Offline lighting edits were not saved.' }
        if ($edited.profiles[0].settings.debounce -ne 5 -or $edited.profiles[0].dpiStages[2].value -ne 1400) { throw 'Offline lighting editing changed unrelated profile values.' }
        Write-Output 'Offline lighting mode and brightness were saved without changing other settings.'
    } finally { if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force } }
} finally { $env:LOCALAPPDATA = $originalLocalAppData }
