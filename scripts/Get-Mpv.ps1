param([string]$Destination = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'third_party/mpv/manifest.json') -Raw | ConvertFrom-Json
if (-not $Destination) { $Destination = Join-Path $projectRoot 'third_party/mpv/windows-x64' }
$runtimeDirectory = [IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $runtimeDirectory -Force | Out-Null
$archivePath = Join-Path $runtimeDirectory 'runtime.7z'
Invoke-WebRequest -Uri $manifest.archive_url -OutFile $archivePath
if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.archive_sha256) { throw 'libmpv archive checksum mismatch' }
& tar.exe -xf $archivePath -C $runtimeDirectory
if ($LASTEXITCODE -ne 0) { throw 'Extraction failed. Use a current Windows tar.exe or extract the verified archive with 7-Zip.' }
$libraryPath = Join-Path $runtimeDirectory $manifest.library
if (-not (Test-Path -LiteralPath $libraryPath)) { throw 'Expected library missing from archive' }
$libraryHash = (Get-FileHash -LiteralPath $libraryPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($libraryHash -ne $manifest.library_sha256) { throw 'libmpv DLL checksum mismatch' }
Write-Output "Runtime ready: $libraryPath"
Write-Output "DLL SHA256: $libraryHash"
