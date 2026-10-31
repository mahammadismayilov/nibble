param([string]$InputDirectory = 'capture-input', [string]$OutputDirectory = 'performance-results')
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
public static class PerfWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    public static Task<int> MovePointer(int left, int top, int width, int height, int durationMs) {
        return Task.Run(() => {
            int count = 0;
            Stopwatch watch = Stopwatch.StartNew();
            while (watch.ElapsedMilliseconds < durationMs) {
                int x = left + 100 + (count % 30) * Math.Max(1, (width - 200) / 30);
                int y = top + (count % 2 == 0 ? 240 : Math.Min(height - 100, 400));
                if (SetCursorPos(x, y)) count++;
                Thread.Sleep(16);
            }
            return count;
        });
    }
}
'@

$exe = Join-Path $env:GITHUB_WORKSPACE "$InputDirectory/target/release/nibble-gpui-prototype.exe"
$output = Join-Path $env:GITHUB_WORKSPACE $OutputDirectory
New-Item -ItemType Directory -Force -Path $output | Out-Null
if (-not (Test-Path -LiteralPath $exe)) { throw "Missing demo executable at $exe" }
$gpuDescription = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Select-Object Name, DriverVersion)
$processorDescription = @(Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue | Select-Object Name, NumberOfLogicalProcessors)
$rows = [System.Collections.Generic.List[object]]::new()

foreach ($style in @('solid', 'frosted', 'clear')) {
    $stdout = Join-Path $output "$style.stdout.txt"
    $stderr = Join-Path $output "$style.stderr.txt"
    $process = Start-Process -FilePath $exe -ArgumentList @('--demo', '--demo-tab=home', "--demo-background=$style") -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        do {
            Start-Sleep -Milliseconds 500
            $process.Refresh()
            if ($process.HasExited) { throw "The $style demo exited early." }
        } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
        if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw "The $style demo did not open." }
        $rect = New-Object PerfWindow+Rect
        if (-not [PerfWindow]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) { throw "Could not get the $style window bounds." }
        $width = $rect.Right - $rect.Left
        $height = $rect.Bottom - $rect.Top
        if ($width -lt 600 -or $height -lt 400) { throw "Unexpected $style window size: ${width}x${height}." }
        if (-not [PerfWindow]::SetForegroundWindow($process.MainWindowHandle)) { throw "Could not focus the $style demo." }
        Start-Sleep -Seconds 3
        foreach ($trial in 1..2) {
        foreach ($phase in @('idle', 'pointer')) {
            if ($phase -eq 'idle') { Start-Sleep -Seconds 2 }
            $process.Refresh()
            $startCpu = $process.TotalProcessorTime.TotalSeconds
            $desktopProcess = Get-Process -Name dwm -ErrorAction SilentlyContinue | Select-Object -First 1
            $desktopStartCpu = if ($desktopProcess -and $null -ne $desktopProcess.CPU) { $desktopProcess.CPU } else { $null }
            $timer = [System.Diagnostics.Stopwatch]::StartNew()
            $moveTask = if ($phase -eq 'pointer') { [PerfWindow]::MovePointer($rect.Left, $rect.Top, $width, $height, 8000) } else { $null }
            $counterSets = @()
            try { $counterSets = @(Get-Counter '\GPU Engine(*)\Utilization Percentage' -SampleInterval 1 -MaxSamples 8 -ErrorAction SilentlyContinue) } catch { }
            $remainingMs = 8000 - [int]$timer.ElapsedMilliseconds
            if ($remainingMs -gt 0) { Start-Sleep -Milliseconds $remainingMs }
            if ($moveTask) { $moveCount = $moveTask.GetAwaiter().GetResult() } else { $moveCount = 0 }
            $timer.Stop()
            $process.Refresh()
            if ($process.HasExited) { throw "The $style demo exited during $phase." }
            $appCpu = 100 * ($process.TotalProcessorTime.TotalSeconds - $startCpu) / $timer.Elapsed.TotalSeconds
            $desktopCpu = $null
            if ($desktopProcess -and $null -ne $desktopStartCpu) {
                $desktopAfter = Get-Process -Id $desktopProcess.Id -ErrorAction SilentlyContinue
                if ($desktopAfter -and $null -ne $desktopAfter.CPU) { $desktopCpu = [math]::Round(100 * ($desktopAfter.CPU - $desktopStartCpu) / $timer.Elapsed.TotalSeconds, 2) }
            }
            $gpuValues = @($counterSets.CounterSamples | Where-Object {
                $_.Status -eq 0 -and $_.InstanceName -match ('^pid_' + $process.Id + '_') -and $_.InstanceName -match 'engtype_3d'
            } | Group-Object Timestamp | ForEach-Object { ($_.Group | Measure-Object CookedValue -Sum).Sum })
            $rows.Add([pscustomobject]@{
                Style = $style; Trial = $trial; Phase = $phase; DurationSeconds = [math]::Round($timer.Elapsed.TotalSeconds, 2)
                AppCpuPercentOfOneCore = [math]::Round($appCpu, 2); DesktopCompositorCpuPercentOfOneCore = $desktopCpu
                AppMean3DEnginePercent = if ($gpuValues.Count) { [math]::Round(($gpuValues | Measure-Object -Average).Average, 2) } else { $null }
                AppPeak3DEnginePercent = if ($gpuValues.Count) { [math]::Round(($gpuValues | Measure-Object -Maximum).Maximum, 2) } else { $null }
                ValidGpuSamples = $gpuValues.Count; PointerMoves = $moveCount
            })
        }
        }
    } finally {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    }
}
$report = [pscustomobject]@{ Runner = $env:RUNNER_OS; Gpu = $gpuDescription; Processor = $processorDescription; Samples = $rows }
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'performance-report.json') -Encoding UTF8
$report | ConvertTo-Json -Depth 8
