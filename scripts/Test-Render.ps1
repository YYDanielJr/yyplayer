# Windows GL regression: static 4K HEVC, real hardware/software output, pixel comparison.
param([string]$Ffmpeg = 'ffmpeg', [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$evidence = Join-Path $projectRoot 'target/render-regression/static'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
Push-Location $projectRoot
$names = @('YYPLAYER_CONFIG','YYPLAYER_SMOKE_SECONDS','YYPLAYER_SMOKE_SCRIPT','YYPLAYER_DIAGNOSTICS','YYPLAYER_UI_CAPTURE','YYPLAYER_RENDER_TRACE')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
try {
    if (-not $SkipBuild) {
        & cargo build --locked -p yyplayer-app --bin yyplayer --example render-compare
        if ($LASTEXITCODE -ne 0) { throw 'Regression tools build failed' }
    }
    $player = Join-Path $projectRoot 'target/debug/yyplayer.exe'
    $compare = Join-Path $projectRoot 'target/debug/examples/render-compare.exe'
    if (-not (Test-Path -LiteralPath $player) -or -not (Test-Path -LiteralPath $compare)) { throw 'Build player and render-compare first' }
    $fixture = Join-Path $evidence 'static-hevc.mp4'
    if (-not (Test-Path -LiteralPath $fixture)) {
        & $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'smptebars=size=3840x2160:rate=30' -t 8 -c:v libx265 -preset ultrafast -x265-params 'log-level=error:pools=2:frame-threads=2' -pix_fmt yuv420p -an $fixture
        if ($LASTEXITCODE -ne 0) { throw 'Static HEVC fixture generation failed (libx265 required)' }
    }
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    $results = @()
    foreach ($case in @(@('hardware','Auto',$false), @('software','Software',$false), @('deband','Auto',$true))) {
        $caseName = $case[0]
        $env:YYPLAYER_CONFIG = Join-Path $evidence "$caseName-settings.json"
        [IO.File]::WriteAllText($env:YYPLAYER_CONFIG, (@{version=1;global=@{mode=$case[1];deband=$case[2]};volume=0;device='auto'} | ConvertTo-Json -Depth 8), $utf8)
        $env:YYPLAYER_SMOKE_SECONDS = '7'
        $env:YYPLAYER_SMOKE_SCRIPT = $null
        $env:YYPLAYER_RENDER_TRACE = $null
        $env:YYPLAYER_DIAGNOSTICS = Join-Path $evidence "$caseName.json"
        $env:YYPLAYER_UI_CAPTURE = Join-Path $evidence "$caseName.ppm"
        $started = [DateTime]::UtcNow
        $process = Start-Process -FilePath $player -ArgumentList ('"' + $fixture + '"') -WindowStyle Hidden -PassThru
        if (-not $process.WaitForExit(30000)) { $process.Kill(); throw "$caseName timed out" }
        if ($process.ExitCode -ne 0) { throw "$caseName exited with $($process.ExitCode)" }
        foreach ($path in @($env:YYPLAYER_DIAGNOSTICS,$env:YYPLAYER_UI_CAPTURE)) {
            if (-not (Test-Path -LiteralPath $path) -or (Get-Item -LiteralPath $path).LastWriteTimeUtc -lt $started) { throw "$caseName did not write fresh evidence" }
        }
        $state = Get-Content -LiteralPath $env:YYPLAYER_DIAGNOSTICS -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($state.error -or $state.render_error -or $state.render_frames -lt 60 -or -not $state.video) { throw "$caseName failed; inspect $evidence" }
        if ($caseName -ne 'software' -and (-not $state.hwdec -or $state.hwdec -eq 'no')) { throw 'Hardware regression unverified: no hardware decoder used' }
        & $Ffmpeg -hide_banner -loglevel error -y -i $env:YYPLAYER_UI_CAPTURE -frames:v 1 (Join-Path $evidence "$caseName.png")
        if ($LASTEXITCODE -ne 0) { throw 'Screenshot conversion failed' }
        $results += @{case=$caseName;hwdec=$state.hwdec;frames=$state.render_frames;exit_code=$process.ExitCode}
    }
    foreach ($caseName in @('hardware','deband')) {
        & $compare (Join-Path $evidence 'software.png') (Join-Path $evidence "$caseName.png") (Join-Path $evidence "$caseName-metrics.json")
        if ($LASTEXITCODE -ne 0) { throw "$caseName image regression failed" }
    }
    $summary = @{fixture_sha256=(Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant();results=$results;passed=$true}
    [IO.File]::WriteAllText((Join-Path $evidence 'summary.json'), ($summary | ConvertTo-Json -Depth 8), $utf8)
    Write-Output "PASS: real 4K HEVC hardware/deband output matches software reference; evidence: $evidence"
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
    Pop-Location
}
