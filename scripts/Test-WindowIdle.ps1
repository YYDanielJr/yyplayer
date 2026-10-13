param([Parameter(Mandatory = $true)][string]$Player)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $root ('target/window-idle-qa-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
$names = @('YYPLAYER_CONFIG','YYPLAYER_SMOKE_SECONDS','YYPLAYER_DIAGNOSTICS','YYPLAYER_SMOKE_SCRIPT','YYPLAYER_AUDIO_SCRIPT','YYPLAYER_UI_CAPTURE','YYPLAYER_RENDER_TRACE','YYPLAYER_MPV_LOG')
$previous = @{}
foreach ($name in $names) {
    $previous[$name] = [Environment]::GetEnvironmentVariable($name)
    Remove-Item -LiteralPath "Env:\$name" -ErrorAction SilentlyContinue
}
try {
    $env:YYPLAYER_CONFIG = Join-Path $fixture 'settings.json'
    $env:YYPLAYER_SMOKE_SECONDS = '8'
    $env:YYPLAYER_DIAGNOSTICS = Join-Path $fixture 'diagnostics.json'
    $process = Start-Process -FilePath $Player -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $fixture 'stdout.log') -RedirectStandardError (Join-Path $fixture 'stderr.log')
    if ($process.WaitForExit(3000)) { throw 'Idle player exited before the sampling window.' }
    $process.Refresh()
    $cpuBefore = $process.TotalProcessorTime.TotalMilliseconds
    $sample = [Diagnostics.Stopwatch]::StartNew()
    if ($process.WaitForExit(3000)) { throw 'Idle player exited before the sampling window ended.' }
    $process.Refresh()
    $cpuMs = $process.TotalProcessorTime.TotalMilliseconds - $cpuBefore
    $sample.Stop()
    if (-not $process.WaitForExit(20000)) { throw 'Idle player failed normal timed shutdown.' }
    if ($process.ExitCode -ne 0) { throw "Idle player exited with $($process.ExitCode)." }
    $data = Get-Content -LiteralPath $env:YYPLAYER_DIAGNOSTICS -Raw | ConvertFrom-Json
    if ($data.phase -ne 'Idle' -or $data.error -or $data.render_error) { throw 'Idle startup reported an error.' }
    if (-not $data.music_database_ready) { throw 'Independent music database did not initialize.' }
    if ($data.native_window_queries -lt 1 -or $data.native_window_queries -gt 8) { throw "Idle window kept querying native state: $($data.native_window_queries)." }
    if ($data.render_lifecycle.initializations -ne 0) { throw 'Idle music page initialized a libmpv renderer.' }
    @{ result = 'PASS'; seconds = 8; native_window_queries = $data.native_window_queries;
       renderer_initializations = $data.render_lifecycle.initializations; sample_ms = $sample.Elapsed.TotalMilliseconds;
       process_cpu_ms = $cpuMs; percent_of_one_logical_cpu = 100 * $cpuMs / $sample.Elapsed.TotalMilliseconds;
       executable_sha256 = (Get-FileHash -LiteralPath $Player).Hash.ToLowerInvariant(); fixture = $fixture } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $fixture 'results.json') -Encoding UTF8
    Write-Output "PASS: 8s idle, $($data.native_window_queries) native window queries. Evidence: $fixture"
} finally {
    foreach ($name in $names) {
        if ($null -eq $previous[$name]) { Remove-Item -LiteralPath "Env:\$name" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($name, $previous[$name]) }
    }
}
