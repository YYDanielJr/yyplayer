param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
# Self-owned tones and independent config; no APPDATA or OS personalization writes.
@'
from pathlib import Path
import wave, array, math
root=Path('target/theme-validation/library')
root.mkdir(parents=True, exist_ok=True)
samples=array.array('h')
for i in range(44100*32):
    v=int(900*math.sin(2*math.pi*440*i/44100))
    samples.extend([v,v])
for folder,name in [('Quiet Hours','Evening Air'),('Quiet Hours','Still Water'),('Open Skies','Golden Light'),('Open Skies','A Little Further')]:
    p=root/folder/(name+'.wav');p.parent.mkdir(parents=True,exist_ok=True)
    with wave.open(str(p),'wb') as f:
        f.setnchannels(2);f.setsampwidth(2);f.setframerate(44100);f.writeframes(samples.tobytes())
'@ | python -
if ($LASTEXITCODE -ne 0) { throw 'Fixture generation failed' }
$previous = $env:YYPLAYER_CONFIG
try {
    $env:YYPLAYER_CONFIG = Join-Path $root 'target/theme-validation/settings.json'
    if ($SkipBuild) { & (Join-Path $root 'target/debug/examples/library-theme.exe') }
    else { cargo run --locked -p yyplayer-app --example library-theme --offline }
    if ($LASTEXITCODE -ne 0) { throw 'Library/theme qualification failed' }
} finally { $env:YYPLAYER_CONFIG = $previous }
