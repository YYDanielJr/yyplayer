# Real contention test: only our three test processes and independent configs are used.
param([string]$Player='', [string]$DevicePattern='Realtek')
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$evidence=Join-Path $projectRoot 'target/audio-validation'
if (-not $Player) {$Player=Join-Path $projectRoot 'target/debug/yyplayer.exe'}
$shared=Get-Content (Join-Path $evidence 'shared.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$index=-1
for ($i=1; $i -lt $shared.devices.Count; $i++) {if ($shared.devices[$i] -match $DevicePattern) {$index=$i;break}}
if ($index -lt 0) {throw 'Choose an available exclusive-capable device with -DevicePattern'}
$device=$shared.device_ids[$index]
$fixture=Join-Path $evidence 'song.flac'
$utf8=New-Object System.Text.UTF8Encoding($false)
$names=@('YYPLAYER_CONFIG','YYPLAYER_SMOKE_SECONDS','YYPLAYER_DIAGNOSTICS','YYPLAYER_SMOKE_SCRIPT','YYPLAYER_AUDIO_SCRIPT')
$previous=@{};foreach($name in $names){$previous[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
$holder=$null
function Start-Audio([string]$name,[string]$mode,[string]$seconds) {
    $env:YYPLAYER_CONFIG=Join-Path $evidence "$name-settings.json"
    [IO.File]::WriteAllText($env:YYPLAYER_CONFIG,(@{version=1;volume=35;device=$device;audio=@{mode=$mode}}|ConvertTo-Json -Depth 8),$utf8)
    $env:YYPLAYER_SMOKE_SECONDS=$seconds;$env:YYPLAYER_DIAGNOSTICS=Join-Path $evidence "$name.json"
    $env:YYPLAYER_AUDIO_SCRIPT=$null;$env:YYPLAYER_SMOKE_SCRIPT=$null
    Start-Process -FilePath $Player -ArgumentList ('"'+$fixture+'"') -WindowStyle Hidden -PassThru
}
try {
    $holder=Start-Audio 'busy-holder' 'StrictExclusive' '20'
    Start-Sleep -Milliseconds 3500
    $strict=Start-Audio 'busy-strict' 'StrictExclusive' '5'
    if (-not $strict.WaitForExit(20000) -or $strict.ExitCode -ne 0) {throw 'Strict contender failed to exit'}
    $strictReport=Get-Content (Join-Path $evidence 'busy-strict.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $strictReport.error -or $strictReport.exclusive -eq $true -or $strictReport.audio_fallback) {throw 'Strict contention policy failed'}
    $prefer=Start-Audio 'busy-prefer' 'PreferExclusive' '5'
    if (-not $prefer.WaitForExit(20000) -or $prefer.ExitCode -ne 0) {throw 'Preferred contender failed to exit'}
    $preferReport=Get-Content (Join-Path $evidence 'busy-prefer.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $preferReport.audio_fallback -or -not $preferReport.error) {throw 'Expected same-device fallback attempt followed by shared-in-use failure'}
    if (-not $holder.WaitForExit(15000) -or $holder.ExitCode -ne 0) {throw 'Exclusive holder did not exit normally'}
    $holderReport=Get-Content (Join-Path $evidence 'busy-holder.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($holderReport.error -or $holderReport.exclusive -ne $true) {throw 'Exclusive holder did not retain its device'}
    Write-Output 'PASS: real exclusive contention; strict refused; preferred tried same-device shared then refused; holder remained exclusive; all exited normally'
} finally {
    if ($holder -and -not $holder.HasExited) {[void]$holder.CloseMainWindow();[void]$holder.WaitForExit(5000)}
    foreach($name in $names){[Environment]::SetEnvironmentVariable($name,$previous[$name],'Process')}
}
