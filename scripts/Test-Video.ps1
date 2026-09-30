param([string]$Player = '', [string]$Ffmpeg = 'ffmpeg', [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$evidence = Join-Path $projectRoot 'docs/validation'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
if (-not $Player) { $Player = Join-Path $projectRoot 'target/debug/yyplayer.exe' }
if (-not $SkipBuild) {
    Push-Location $projectRoot
    try {
        & cargo build --locked -p yyplayer-app
        if ($LASTEXITCODE -ne 0) { throw 'Player build failed' }
    } finally { Pop-Location }
}
if (-not (Test-Path -LiteralPath $Player)) { throw "Player missing: $Player" }
$fixture = Join-Path $evidence 'video-fixture.mp4'
if (-not (Test-Path -LiteralPath $fixture)) {
    & $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'testsrc2=size=1280x720:rate=30' -f lavfi -i 'sine=frequency=440:sample_rate=48000' -t 24 -c:v libx264 -preset veryfast -pix_fmt yuv420p -c:a aac -shortest $fixture
    if ($LASTEXITCODE -ne 0) { throw 'Fixture generation failed' }
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
$names = @('YYPLAYER_CONFIG', 'YYPLAYER_SMOKE_SECONDS', 'YYPLAYER_SMOKE_SCRIPT', 'YYPLAYER_DIAGNOSTICS', 'YYPLAYER_UI_CAPTURE')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
function Invoke-Scenario([string]$name, [string]$mode, [double]$seconds, [bool]$scripted) {
    $config = Join-Path $evidence "$name-settings.json"
    [IO.File]::WriteAllText($config, (@{ version = 1; global = @{ mode = $mode }; volume = 70; device = 'auto' } | ConvertTo-Json -Depth 8), $utf8)
    $report = Join-Path $evidence "$name.json"
    $env:YYPLAYER_CONFIG = $config
    $env:YYPLAYER_SMOKE_SECONDS = "$seconds"
    $env:YYPLAYER_DIAGNOSTICS = $report
    $env:YYPLAYER_SMOKE_SCRIPT = if ($scripted) { '1' } else { $null }
    $env:YYPLAYER_UI_CAPTURE = if ($scripted) { Join-Path $evidence 'video-ui.ppm' } else { $null }
    $scenarioStarted = [DateTime]::UtcNow
    $process = Start-Process -FilePath $Player -ArgumentList ('"' + $fixture + '"') -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(60000)) { $process.Kill(); throw "$name did not exit within 60 seconds" }
    if ($process.ExitCode -ne 0) { throw "$name exited with $($process.ExitCode)" }
    if (-not (Test-Path -LiteralPath $report) -or (Get-Item -LiteralPath $report).LastWriteTimeUtc -lt $scenarioStarted) { throw "$name did not write a fresh report" }
    $result = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
    if ($result.error -or $result.render_error -or $result.render_frames -lt 60 -or -not $result.video -or $result.devices.Count -lt 1) { throw "$name playback failed; inspect $report" }
    return $result
}
try {
    $controlsStarted = [DateTime]::UtcNow
    $controls = Invoke-Scenario 'controls-playback' 'Auto' 14 $true
    if ($controls.script_steps -ne 11 -or $controls.phase -ne 'Playing' -or $controls.hwdec -ne 'no' -or $controls.volume -ne 35 -or -not $controls.muted) { throw 'Control/reload state did not match' }
    $points = $controls.checkpoints
    if (-not ($points | Where-Object { $_.speed -eq 1.5 }) -or -not ($points | Where-Object { $_.phase -eq 'Paused' }) -or -not ($points | Where-Object { $_.fullscreen }) -or -not ($points | Where-Object { $_.maximized -and -not $_.fullscreen })) { throw 'Missing speed/pause/fullscreen/maximize checkpoint' }
    $pausedBefore = $points | Where-Object { $_.phase -eq 'Paused' -and $_.seconds -gt 9 -and $_.seconds -lt 11 } | Select-Object -Last 1
    $pausedAfter = $points | Where-Object { $_.phase -eq 'Paused' -and $_.hwdec -eq 'no' -and $_.seconds -gt 11 -and $_.seconds -lt 12.5 } | Select-Object -Last 1
    if (-not $pausedBefore -or -not $pausedAfter -or [Math]::Abs($pausedAfter.position - $pausedBefore.position) -gt 0.2) { throw 'Decoder reload did not preserve paused position' }
    $saved = Get-Content -LiteralPath $env:YYPLAYER_CONFIG -Raw | ConvertFrom-Json
    if (-not ($saved.files.PSObject.Properties | Where-Object { $_.Value.mode -eq 'Software' })) { throw 'File decoder override was not persisted' }
    $software = Invoke-Scenario 'software-playback' 'Software' 7 $false
    if ($software.hwdec -ne 'no' -or $software.position -lt 2) { throw 'Software decoding scenario failed' }
    $capture = Join-Path $evidence 'video-ui.ppm'
    if (-not (Test-Path -LiteralPath $capture) -or (Get-Item -LiteralPath $capture).LastWriteTimeUtc -lt $controlsStarted) { throw 'No fresh debug UI capture; use a debug Player' }
    & $Ffmpeg -hide_banner -loglevel error -y -i $capture -frames:v 1 (Join-Path $projectRoot 'docs/video-ui.png')
    if ($LASTEXITCODE -ne 0) { throw 'UI evidence conversion failed' }
    Write-Output "PASS: controls, paused decoder reload, persistence, software decoding, safe exit"
    Write-Output "Fixture SHA256: $((Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant())"
    Write-Output "Runtime: $($controls.runtime); initial decoder: $(($points | Where-Object { $_.hwdec -and $_.hwdec -ne 'no' } | Select-Object -First 1).hwdec)"
    Write-Output "Reports: $evidence"
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}
