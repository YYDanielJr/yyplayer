param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$fixture = Join-Path $root 'target/ui-customization'
New-Item -ItemType Directory -Force -Path (Join-Path $fixture 'library') | Out-Null
# Self-owned media, local fonts, independent config; no user media / APPDATA writes.
& ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'sine=frequency=440:duration=4' -metadata 'artist=YYPlayer Studio' -metadata 'album=Glass Sessions' -c:a pcm_s16le (Join-Path $fixture 'tone.wav')
if ($LASTEXITCODE -ne 0) { throw 'Audio fixture generation failed; ffmpeg required' }
@'
from pathlib import Path
import shutil
from PIL import Image, ImageDraw
root=Path('target/ui-customization')
for i in range(1,81):
    stem=f'{i:02} - Glass Session'
    shutil.copyfile(root/'tone.wav',root/'library'/(stem+'.wav'))
    (root/'library'/(stem+'.lrc')).write_text('[00:00.00]Lyrics font \u00b7 Aa 012345\n[00:02.00]Local lyrics \u00b7 Glass Sessions\n',encoding='utf-8')
for i in range(1,7):
    art=Image.new('RGB',(256,256),(24+i*14,60+i*10,120+i*12))
    draw=ImageDraw.Draw(art)
    draw.ellipse((36,36,220,220),fill=(170,200,225))
    draw.ellipse((80,80,176,176),fill=(40,60,110))
    art.save(root/'library'/f'{i:02} - Glass Session.png')
(root/'subtitle.srt').write_text('1\n00:00:00,000 --> 00:00:29,000\nSubtitle font \u00b7 Aa 012345\n',encoding='utf-8')
(root/'subtitle.ass').write_text('''[Script Info]
ScriptType: v4.00+
PlayResX: 1280
PlayResY: 720
[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,36,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,0,2,20,20,40,1
[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.00,0:00:29.00,Default,,0,0,0,,ASS font \u00b7 Aa 012345
''',encoding='utf-8')
'@ | python -
if ($LASTEXITCODE -ne 0) { throw 'Local subtitle/lyrics fixtures failed; Python required' }
& ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'color=c=0x20304a:s=1280x720:r=24:d=30' -c:v libx264 -pix_fmt yuv420p (Join-Path $fixture 'subtitle.mp4')
if ($LASTEXITCODE -ne 0) { throw 'Video fixture generation failed' }
& ffmpeg -hide_banner -loglevel error -y -i (Join-Path $fixture 'tone.wav') -i (Join-Path $fixture 'library/01 - Glass Session.png') -map 0:a -map 1 -metadata 'artist=YYPlayer Studio' -metadata 'album=Glass Sessions' -c:a flac -c:v png -disposition:v attached_pic (Join-Path $fixture 'library/01 - Glass Session.flac')
if ($LASTEXITCODE -ne 0) { throw 'Embedded cover fixture generation failed' }
Remove-Item -LiteralPath (Join-Path $fixture 'library/01 - Glass Session.wav')
# Remove the sidecar so the first row must decode its embedded FLAC artwork.
Remove-Item -LiteralPath (Join-Path $fixture 'library/01 - Glass Session.png')
$previous = $env:YYPLAYER_CONFIG
try {
    $env:YYPLAYER_CONFIG = Join-Path $fixture 'settings.json'
    if ($SkipBuild) { & (Join-Path $root 'target/debug/examples/ui-customization.exe') }
    else { cargo run --locked --offline -p yyplayer-app --example ui-customization --config profile.dev.debug=0 --config profile.dev.incremental=false }
    if ($LASTEXITCODE -ne 0) { throw 'UI customization qualification failed' }
} finally { $env:YYPLAYER_CONFIG = $previous }
