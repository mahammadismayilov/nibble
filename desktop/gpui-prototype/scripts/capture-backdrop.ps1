$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class BackdropCaptureWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
}
'@

$exe = Join-Path $env:GITHUB_WORKSPACE 'capture-input/target/release/nibble-gpui-prototype.exe'
$output = Join-Path $env:GITHUB_WORKSPACE 'visual-captures'
$backdrop = [System.Windows.Forms.Form]::new()
$backdrop.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$backdrop.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
$backdrop.Bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$backdrop.ShowInTaskbar = $false
try {
    $backdrop.BackColor = [System.Drawing.Color]::Red
    $backdrop.Show()
    [System.Windows.Forms.Application]::DoEvents()
    foreach ($style in @('solid', 'frosted', 'clear')) {
        $stdout = Join-Path $output "backdrop-$style.stdout.txt"
        $stderr = Join-Path $output "backdrop-$style.stderr.txt"
        $process = Start-Process -FilePath $exe -ArgumentList @('--demo', '--demo-light', '--demo-tab=macro', "--demo-background=$style") -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(30)
            do {
                Start-Sleep -Milliseconds 500
                $process.Refresh()
                if ($process.HasExited) { throw "The $style demo exited before opening." }
            } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
            if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw "The $style demo did not open." }
            Start-Sleep -Seconds 2
            $rect = New-Object BackdropCaptureWindow+Rect
            if (-not [BackdropCaptureWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw "Could not read the $style window bounds." }
            $width = $rect.Right - $rect.Left
            $height = $rect.Bottom - $rect.Top
            if ($width -lt 600 -or $height -lt 400) { throw "Unexpected $style window bounds." }
            $samples = @()
            foreach ($colorName in @('Red', 'Blue')) {
                $backdrop.BackColor = [System.Drawing.Color]::$colorName
                $backdrop.Refresh()
                [System.Windows.Forms.Application]::DoEvents()
                if (-not [BackdropCaptureWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw "Could not focus the $style demo." }
                Start-Sleep -Seconds 1
                $bitmap = [System.Drawing.Bitmap]::new($width, $height)
                $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
                try {
                    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, [System.Drawing.Size]::new($width, $height))
                    $bitmap.Save((Join-Path $output "backdrop-$style-$colorName.png"), [System.Drawing.Imaging.ImageFormat]::Png)
                    $samples += $bitmap.GetPixel(30, 170).ToArgb()
                } finally {
                    $graphics.Dispose()
                    $bitmap.Dispose()
                }
            }
            Write-Output "Backdrop ${style}: red=$($samples[0]), blue=$($samples[1]), desktop influences window=$($samples[0] -ne $samples[1])"
        } finally {
            if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
        }
    }
} finally {
    $backdrop.Close()
    $backdrop.Dispose()
}
