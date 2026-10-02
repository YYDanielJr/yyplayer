param([switch]$SkipBuild, [string]$Ffmpeg = 'ffmpeg')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$previous = $env:YYPLAYER_CONFIG
try {
    $base = Join-Path $root 'target/video-library'
    New-Item -ItemType Directory -Path (Join-Path $base 'library/A'), (Join-Path $base 'library/B') -Force | Out-Null
    $video = Join-Path $base 'library/A/湖边.mp4'
    & $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'smptebars=size=1280x720:rate=30' -t 24 -c:v libx264 -preset ultrafast -pix_fmt yuv420p -an $video
    if ($LASTEXITCODE -ne 0) { throw 'Video fixture failed' }
    Copy-Item -LiteralPath $video -Destination (Join-Path $base 'library/B/天空.mp4') -Force
    Copy-Item -LiteralPath $video -Destination (Join-Path $base 'loose.mp4') -Force
    & $Ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'anullsrc=r=44100:cl=stereo' -t 24 (Join-Path $base 'silence.wav')
    if ($LASTEXITCODE -ne 0) { throw 'Audio fixture failed' }
    if (-not $SkipBuild) {
        cargo build --locked --offline -p yyplayer-app --example video-library
        if ($LASTEXITCODE -ne 0) { throw 'Example build failed' }
    }
    $env:YYPLAYER_CONFIG = Join-Path $base 'settings.json'
    $process = Start-Process -FilePath (Join-Path $root 'target/debug/examples/video-library.exe') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $base 'stdout.log') -RedirectStandardError (Join-Path $base 'stderr.log')
    $samples = @()
    while (-not $process.WaitForExit(500)) {
        $process.Refresh()
        $samples += @{time=[DateTime]::UtcNow.ToString('o');working_set=$process.WorkingSet64;private_bytes=$process.PrivateMemorySize64}
    }
    $samples | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $base 'memory.json') -Encoding UTF8
    Get-Content -LiteralPath (Join-Path $base 'stdout.log')
    if ($process.ExitCode -ne 0) { Get-Content -LiteralPath (Join-Path $base 'stderr.log'); throw "Video library qualification exited $($process.ExitCode)" }
} finally { $env:YYPLAYER_CONFIG = $previous; Pop-Location }
