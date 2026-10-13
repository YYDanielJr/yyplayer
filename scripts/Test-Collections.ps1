param([switch]$SkipBuild, [switch]$Release)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$base = Join-Path $root 'target/collections'
New-Item -ItemType Directory -Force -Path (Join-Path $base 'music'), (Join-Path $base 'video') | Out-Null
for ($i = 0; $i -lt 4; $i++) {
    $artist = if ($i -lt 2) { 'YYPlayer A' } else { 'YYPlayer B' }
    & ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'anullsrc=r=48000:cl=stereo:d=8' -metadata "title=Night $i" -metadata "artist=$artist" -metadata 'album=Live' -c:a flac (Join-Path $base "music/$i.flac")
    if ($LASTEXITCODE -ne 0) { throw 'Music fixture generation failed' }
}
for ($i = 0; $i -lt 2; $i++) {
    & ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'color=c=0x25394e:s=640x360:r=24:d=12' -c:v libx264 -pix_fmt yuv420p (Join-Path $base "video/$i.mp4")
    if ($LASTEXITCODE -ne 0) { throw 'Video fixture generation failed' }
}
$previous = $env:YYPLAYER_CONFIG
try {
    $env:YYPLAYER_CONFIG = Join-Path $base ('settings-sqlite-' + [Guid]::NewGuid().ToString('N') + '.json')
    $profile = if ($Release) { 'release' } else { 'debug' }
    if ($SkipBuild) { & (Join-Path $root "target/$profile/examples/collections.exe") }
    elseif ($Release) { cargo run --locked --offline --release -p yyplayer-app --example collections }
    else { cargo run --locked --offline -p yyplayer-app --example collections --config profile.dev.debug=0 --config profile.dev.incremental=false }
    if ($LASTEXITCODE -ne 0) { throw 'Collections qualification failed' }
} finally { $env:YYPLAYER_CONFIG = $previous }
