$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This experimental build only runs in a disposable GitHub Actions checkout.' }

$project = Join-Path $env:GITHUB_WORKSPACE 'desktop/gpui-prototype'
$archive = Join-Path $env:RUNNER_TEMP 'gpui-0.2.2-performance.crate'
$vendor = Join-Path $project '.performance-vendor'
New-Item -ItemType Directory -Force -Path $vendor | Out-Null
Invoke-WebRequest -Uri 'https://static.crates.io/crates/gpui/gpui-0.2.2.crate' -OutFile $archive
$checksum = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($checksum -ne '979b45cfa6ec723b6f42330915a1b3769b930d02b2d505f9697f8ca602bee707') { throw 'The GPUI archive does not match Cargo.lock.' }
tar -xf $archive -C $vendor
if ($LASTEXITCODE -ne 0) { throw 'Could not unpack the GPUI archive.' }
$windowSource = Join-Path $vendor 'gpui-0.2.2/src/window.rs'
$original = [IO.File]::ReadAllText($windowSource)
$pattern = '\|\|\s*\(active\.get\(\)\s*&&\s*last_input_timestamp\.get\(\)\.elapsed\(\)\s*<\s*Duration::from_secs\(1\)\)'
$matcher = [regex]::new($pattern)
if ($matcher.Matches($original).Count -ne 1) { throw 'Could not identify the exact GPUI presentation rule.' }
$modified = $matcher.Replace($original, '', 1)
[IO.File]::WriteAllText($windowSource, $modified, [Text.UTF8Encoding]::new($false))
$cargoToml = Join-Path $project 'Cargo.toml'
if ([IO.File]::ReadAllText($cargoToml) -match '(?m)^\[patch\.crates-io\]') { throw 'The project already defines a crates.io patch.' }
Add-Content -LiteralPath $cargoToml -Value "`n[patch.crates-io]`ngpui = { path = `".performance-vendor/gpui-0.2.2`" }" -Encoding UTF8
Push-Location $project
try {
    cargo build --release --offline
    if ($LASTEXITCODE -ne 0) { throw 'The experimental GPUI build failed.' }
    cargo test --release --offline
    if ($LASTEXITCODE -ne 0) { throw 'The experimental GPUI tests failed.' }
} finally { Pop-Location }
