$base = Join-Path (Get-Location) 'target/resource-comparison'
$names = @('YYPLAYER_CONFIG','YYPLAYER_SMOKE_SECONDS','YYPLAYER_DIAGNOSTICS')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name,'Process') }
try {
 $env:YYPLAYER_CONFIG = Join-Path $base 'new-settings.json'
 $env:YYPLAYER_SMOKE_SECONDS = '8'
 $env:YYPLAYER_DIAGNOSTICS = Join-Path $base 'new-diagnostics.json'
 $player = Join-Path (Get-Location) 'target/release/yyplayer.exe'
 $sha = (Get-FileHash -LiteralPath $player).Hash
 $media = Join-Path $base 'silence.wav'
 $process = Start-Process -FilePath $player -ArgumentList ('"'+$media+'"') -WindowStyle Hidden -PassThru
 $samples=@();$start=[DateTime]::UtcNow
 while (-not $process.WaitForExit(500)) { $process.Refresh();$samples+=@{seconds=([DateTime]::UtcNow-$start).TotalSeconds;working_set=$process.WorkingSet64;private_bytes=$process.PrivateMemorySize64} }
 @{executable_sha256=$sha;exit=$process.ExitCode;samples=$samples;media_sha256=(Get-FileHash -LiteralPath $media).Hash} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $base 'new-memory.json') -Encoding UTF8
 Write-Output "New release sample exited $($process.ExitCode)"
} finally { foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name,$previous[$name],'Process') } }
