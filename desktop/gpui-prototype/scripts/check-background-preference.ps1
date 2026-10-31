$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PreferenceWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
}
'@

$exe = Join-Path $env:GITHUB_WORKSPACE 'capture-input/target/release/nibble-gpui-prototype.exe'
$originalLocalAppData = $env:LOCALAPPDATA
$env:LOCALAPPDATA = Join-Path $env:RUNNER_TEMP ('nibble-preference-' + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $env:LOCALAPPDATA | Out-Null
New-Item -ItemType Directory -Path (Join-Path $env:LOCALAPPDATA 'Nibble-gpui') | Out-Null
[IO.File]::WriteAllText((Join-Path $env:LOCALAPPDATA 'Nibble-gpui/theme'), 'light')

function Start-PreferenceApp {
    param([string]$Name)
    $output = Join-Path $env:GITHUB_WORKSPACE 'visual-captures'
    $proc = Start-Process -FilePath $exe -PassThru -WindowStyle Normal -RedirectStandardOutput (Join-Path $output "$Name.stdout.txt") -RedirectStandardError (Join-Path $output "$Name.stderr.txt")
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) { throw 'Preference test app exited early.' }
    } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Preference test app did not open.' }
    Start-Sleep -Seconds 3
    return $proc
}

try {
    $process = Start-PreferenceApp -Name 'preference-first'
    try {
        $rect = New-Object PreferenceWindow+Rect
        if (-not [PreferenceWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw 'Could not read preference test window bounds.' }
        if (($rect.Right - $rect.Left) -ne 1040 -or ($rect.Bottom - $rect.Top) -ne 711) { throw 'Preference test requires the expected runner window size.' }
        if (-not [PreferenceWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus preference test window.' }
        if (-not [PreferenceWindow]::SetCursorPos($rect.Left + 425, $rect.Top + 120)) { throw 'Could not select Settings.' }
        [PreferenceWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        [PreferenceWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Seconds 1
        if (-not [PreferenceWindow]::SetCursorPos($rect.Left + 800, $rect.Top + 550)) { throw 'Could not position the pointer for scrolling.' }
        $wheelDown = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]-480), 0)
        [PreferenceWindow]::mouse_event(0x0800, 0, 0, $wheelDown, [UIntPtr]::Zero)
        Start-Sleep -Seconds 1
        if (-not [PreferenceWindow]::SetCursorPos($rect.Left + 960, $rect.Top + 582)) { throw 'Could not select Clear.' }
        [PreferenceWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        [PreferenceWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Seconds 1
        $preferencePath = Join-Path $env:LOCALAPPDATA 'Nibble-gpui/background'
        if (-not (Test-Path -LiteralPath $preferencePath)) { throw 'Background preference was not saved.' }
        if ((Get-Content -LiteralPath $preferencePath -Raw).Trim() -ne 'clear') { throw 'The saved background preference is not Clear.' }
    } finally {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    }
    $process = Start-PreferenceApp -Name 'preference-restart'
    try {
        if ((Get-Content -LiteralPath $preferencePath -Raw).Trim() -ne 'clear') { throw 'The saved preference changed after restarting.' }
        $rect = New-Object PreferenceWindow+Rect
        if (-not [PreferenceWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw 'Could not read the restarted window bounds.' }
        if (-not [PreferenceWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw 'Could not focus the restarted window.' }
        if (-not [PreferenceWindow]::SetCursorPos($rect.Left + 425, $rect.Top + 120)) { throw 'Could not reopen Settings.' }
        [PreferenceWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        [PreferenceWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Seconds 1
        if (-not [PreferenceWindow]::SetCursorPos($rect.Left + 800, $rect.Top + 550)) { throw 'Could not scroll the restarted window.' }
        [PreferenceWindow]::mouse_event(0x0800, 0, 0, $wheelDown, [UIntPtr]::Zero)
        Start-Sleep -Seconds 1
        $bitmap = [System.Drawing.Bitmap]::new(1040, 711)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $dc = $graphics.GetHdc()
            try { $printed = [PreferenceWindow]::PrintWindow($process.MainWindowHandle, $dc, 2) }
            finally { $graphics.ReleaseHdc($dc) }
            if (-not $printed) { throw 'Could not capture the restarted preference.' }
            $bitmap.Save((Join-Path $env:GITHUB_WORKSPACE 'visual-captures/preference-restarted.png'), [System.Drawing.Imaging.ImageFormat]::Png)
            if ($bitmap.GetPixel(980, 580).ToArgb() -ne [System.Drawing.Color]::FromArgb(255, 228, 229, 232).ToArgb()) {
                throw 'Clear was not selected after restarting.'
            }
        } finally {
            $graphics.Dispose()
            $bitmap.Dispose()
        }
        Write-Output 'Background preference saved and Clear was selected after restart.'
    } finally {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    }
} finally {
    $env:LOCALAPPDATA = $originalLocalAppData
}
