param([string]$Player = '', [string]$Ffmpeg = 'ffmpeg', [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$evidence = Join-Path $projectRoot 'target/audio-validation'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
if (-not $Player) { $Player = Join-Path $projectRoot 'target/debug/yyplayer.exe' }
if (-not $SkipBuild) {
    Push-Location $projectRoot
    try { & cargo build --locked -p yyplayer-app; if ($LASTEXITCODE -ne 0) { throw 'Build failed' } } finally { Pop-Location }
}
$fixture = Join-Path $evidence 'song.flac'
$cover = Join-Path $evidence 'cover.png'
& $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'gradients=s=600x600:c0=0x223e38:c1=0x82b19a:x0=0:y0=0:x1=600:y1=600:type=linear' -frames:v 1 $cover
if ($LASTEXITCODE -ne 0) { throw 'Cover generation failed' }
& $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'sine=frequency=1000:sample_rate=44100' -i $cover -map 0:a -map 1:v -t 32 -ac 2 -sample_fmt s32 -c:a flac -c:v copy -disposition:v attached_pic -metadata title='Lake Lights' -metadata artist='YYPlayer Validation' -metadata album='Local Sessions' $fixture
if ($LASTEXITCODE -ne 0) { throw 'FLAC generation failed' }
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $evidence 'song.lrc'), "[00:00.00]A quiet place for your sound`n[00:03.00]Follow the lights along the lake`n[00:06.00]Let the evening breathe`n[00:09.00]Listen to the space between`n[00:12.00]Your music, close to you`n[00:18.00]The night unfolds", $utf8)
$names = @('YYPLAYER_CONFIG','YYPLAYER_SMOKE_SECONDS','YYPLAYER_SMOKE_SCRIPT','YYPLAYER_AUDIO_SCRIPT','YYPLAYER_DIAGNOSTICS')
$previous = @{}; foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name,'Process') }
function Invoke-AudioScenario([string]$name,[string]$mode,[double]$seconds,[bool]$scripted) {
    $config = Join-Path $evidence "$name-settings.json"
    [IO.File]::WriteAllText($config, (@{ version=1; device='auto'; volume=35; audio=@{mode=$mode} } | ConvertTo-Json -Depth 10),$utf8)
    $env:YYPLAYER_CONFIG=$config; $env:YYPLAYER_SMOKE_SECONDS="$seconds"; $env:YYPLAYER_SMOKE_SCRIPT=$null
    $env:YYPLAYER_AUDIO_SCRIPT=if ($scripted) {'1'} else {$null}
    $env:YYPLAYER_DIAGNOSTICS=Join-Path $evidence "$name.json"
    $started=[DateTime]::UtcNow
    $process=Start-Process -FilePath $Player -ArgumentList ('"'+$fixture+'"') -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(45000)) { $process.Kill(); throw "$name exit timed out" }
    if ($process.ExitCode -ne 0) {throw "$name exited with $($process.ExitCode)"}
    if (-not (Test-Path -LiteralPath $env:YYPLAYER_DIAGNOSTICS) -or (Get-Item -LiteralPath $env:YYPLAYER_DIAGNOSTICS).LastWriteTimeUtc -lt $started) {throw "$name missing fresh report"}
    $report=Get-Content -LiteralPath $env:YYPLAYER_DIAGNOSTICS -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($report.error -or $report.video -or $report.phase -ne 'Playing' -or $report.position -lt 2 -or $report.music_title -ne 'Lake Lights' -or $report.lyric_count -ne 6 -or $report.cover_width -lt 1) {throw "$name playback/assets failed: $($report.error)"}
    return $report
}
try {
    $shared=Invoke-AudioScenario 'shared' 'Shared' 5 $false
    if ($shared.exclusive -ne $false) {throw 'Shared WASAPI was not confirmed'}
    $exclusive=Invoke-AudioScenario 'exclusive' 'StrictExclusive' 5 $false
    if ($exclusive.exclusive -ne $true) {throw 'Exclusive WASAPI was not confirmed'}
    $controls=Invoke-AudioScenario 'controls' 'Shared' 14 $true
    if ($controls.script_steps -ne 7 -or $controls.eq_filter -or $controls.page -ne 0) {throw 'EQ cleanup/browse failed'}
    if (-not ($controls.checkpoints | Where-Object { $_.eq_filter -match 'g=4.500000' }) -or -not ($controls.checkpoints | Where-Object { $_.eq_filter -match 'g=-6.000000' }) -or -not ($controls.checkpoints | Where-Object { $_.exclusive -eq $true })) {throw 'Missing EQ precedence/exclusive checkpoints'}
    $saved=Get-Content -LiteralPath $env:YYPLAYER_CONFIG -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $saved.audio.presets.Validation) {throw 'Named EQ not persisted'}
    # A high-rate fixture exercises source-rate refusal, rather than forcing upsampling.
    $highRate=Join-Path $evidence 'high-rate.flac'
    & $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'sine=frequency=1000:sample_rate=768000' -t 16 -ac 2 -c:a flac $highRate
    if ($LASTEXITCODE -ne 0) {throw 'High-rate fixture generation failed'}
    $env:YYPLAYER_CONFIG=Join-Path $evidence 'high-rate-settings.json'
    [IO.File]::WriteAllText($env:YYPLAYER_CONFIG,(@{version=1;device='auto';volume=35;audio=@{mode='StrictExclusive';preserve_rate=$true}}|ConvertTo-Json -Depth 8),$utf8)
    $env:YYPLAYER_DIAGNOSTICS=Join-Path $evidence 'high-rate.json'; $env:YYPLAYER_AUDIO_SCRIPT=$null; $env:YYPLAYER_SMOKE_SECONDS='6'
    $process=Start-Process -FilePath $Player -ArgumentList ('"'+$highRate+'"') -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(30000) -or $process.ExitCode -ne 0) {throw 'High-rate source guard test did not exit'}
    $rate=Get-Content -LiteralPath $env:YYPLAYER_DIAGNOSTICS -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($rate.audio_source_rate -ne $rate.audio_output_rate) {
        if ($rate.phase -ne 'Paused' -or -not $rate.error) {throw 'Unsupported source rate was not paused'}
    } elseif ($rate.error -or $rate.exclusive -ne $true) {throw 'Native high-rate output failed'}
    Write-Output 'PASS: real FLAC, embedded artwork, automatic LRC, shared/exclusive WASAPI, EQ global/device/file, cleanup, persistence, safe exit'
    Write-Output "Source-rate guard: $($rate.audio_source_rate) -> $($rate.audio_output_rate) Hz; $($rate.phase)"
    Write-Output "Fixture SHA256: $((Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant())"
    Write-Output "Reports: $evidence"
} finally { foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name,$previous[$name],'Process') } }
