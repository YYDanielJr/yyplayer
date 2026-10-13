param([Parameter(Mandatory = $true)][string]$NsisCompiler)

# Run the real installer code with a compile-time private HKCU ID and shortcuts.
# No admin request, production installation, APPDATA settings or real DLL is used.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$testRoot = Join-Path $root ('target/windows-installer-qa-' + [Guid]::NewGuid().ToString('N'))
$appId = 'YYPlayer-QA-' + [Guid]::NewGuid().ToString('N')
$stage = Join-Path $testRoot 'stage'
$output = Join-Path $testRoot 'packages'
$installed = Join-Path $testRoot 'installed with spaces'
$rules = Join-Path $testRoot 'files.nsh'
$utf8 = [Text.UTF8Encoding]::new($false)
$registry = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software', $true)
$keyPath = 'Microsoft\Windows\CurrentVersion\Uninstall\' + $appId
$cases = [Collections.Generic.List[string]]::new()
New-Item -ItemType Directory -Path $stage, $output -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination $stage
[IO.File]::WriteAllText($rules, @'
!macro YYPlayerInstallFiles
  Push "LICENSE"
  Call InstallFile
  Push "yyplayer.exe"
  Call InstallFile
  Push "runtime\fixture.dll"
  Call InstallFile
  Push "uninstall.exe"
  Call InstallFile
!macroend
!macro YYPlayerRollbackFiles
  Push "LICENSE"
  Call RollbackFile
  Push "yyplayer.exe"
  Call RollbackFile
  Push "runtime\fixture.dll"
  Call RollbackFile
  Push "uninstall.exe"
  Call RollbackFile
!macroend
!macro YYPlayerUninstallFiles
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\yyplayer.exe"
  Delete "$INSTDIR\runtime\fixture.dll"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR\runtime"
!macroend
'@, $utf8)

function Compile-Installer([string]$Version, [string]$Tag) {
    [IO.File]::WriteAllText((Join-Path $stage 'yyplayer.exe'), "fixture-$Tag", $utf8)
    New-Item -ItemType Directory -Path (Join-Path $stage 'runtime') -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $stage 'runtime/fixture.dll'), "fixture-dll-$Tag", $utf8)
    $env:YYPLAYER_PACKAGE_STAGE = $stage
    $env:YYPLAYER_PACKAGE_OUTPUT = $output
    & $NsisCompiler '/V2' '/DINSTALL_TEST' "/DAPP_ID=$appId" "/DAPP_VERSION=$Version" "/DAPP_COMMIT=$Tag" "/DAPP_FILE_RULES=$rules" "/DAPP_ICON=$(Join-Path $root 'assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico')" (Join-Path $root 'packaging/windows/yyplayer.nsi') *> (Join-Path $testRoot "compile-$Tag.log")
    if ($LASTEXITCODE -ne 0) { throw "Installer compilation failed: $Tag" }
    return Join-Path $output "YYPlayer-$Version-$Tag-windows-x64-setup.exe"
}
function Invoke-Installer([string]$Path, [string[]]$Arguments, [int]$Expected = 0) {
    $process = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(30000)) { throw 'Isolated installer timed out.' }
    if ($process.ExitCode -ne $Expected) { throw "Installer returned $($process.ExitCode), expected $Expected." }
}
function Read-InstallValue([string]$Name) {
    $key = $registry.OpenSubKey($keyPath)
    if (-not $key) { return $null }
    try { return $key.GetValue($Name) } finally { $key.Dispose() }
}
function Ensure([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
$previousStage = $env:YYPLAYER_PACKAGE_STAGE
$previousOutput = $env:YYPLAYER_PACKAGE_OUTPUT
try {
    $old = Compile-Installer '0.5.1' 'old'
    $previous = Compile-Installer '0.5.3' 'previous'
    $new = Compile-Installer '0.5.4' 'new'
    $older = Compile-Installer '0.5.0' 'downgrade'
    # Read the generated PE manifest, rather than matching the .nsi source.
    $binary = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($new))
    Ensure ($binary -match '<dpiAware[^>]*>true</dpiAware>') 'Generated Setup is not system-DPI aware.'
    $cases.Add('generated-setup-dpi-manifest')
    Invoke-Installer $old @('/S', "/D=$installed")
    Ensure ((Read-InstallValue 'DisplayVersion') -eq '0.5.1') 'Old fixture did not install.'
    Ensure ((Read-InstallValue 'LastInstallAction') -eq 'install') 'Fresh installation mode is wrong.'
    $uninstallBinary = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes((Join-Path $installed 'uninstall.exe')))
    Ensure ($uninstallBinary -match '<dpiAware[^>]*>true</dpiAware>') 'Generated uninstaller is not system-DPI aware.'
    $cases.Add('fresh-install')
    Invoke-Installer $previous @('/S')
    Ensure ((Read-InstallValue 'DisplayVersion') -eq '0.5.3') 'Previous release fixture did not upgrade.'
    Ensure ((Read-InstallValue 'LastInstallAction') -eq 'upgrade') 'Previous release was not detected as upgrade.'
    Invoke-Installer $new @('/S')
    Ensure ((Read-InstallValue 'DisplayVersion') -eq '0.5.4') 'Current release did not upgrade the previous release.'
    Ensure ((Read-InstallValue 'LastInstallAction') -eq 'upgrade') '0.5.3 to 0.5.4 was not detected as upgrade.'
    $cases.Add('0.5.3-to-0.5.4-upgrade')
    Invoke-Installer (Join-Path $installed 'uninstall.exe') @('/S', "_?=$installed")
    Invoke-Installer $old @('/S', "/D=$installed")
    $userFile = Join-Path $installed 'user-kept.txt'
    [IO.File]::WriteAllText($userFile, 'preserve me', $utf8)
    $externalConfig = Join-Path $testRoot 'settings.library.sqlite3'
    [IO.File]::WriteAllText($externalConfig, 'independent user data', $utf8)
    $configHash = (Get-FileHash -LiteralPath $externalConfig).Hash
    $oldExeHash = (Get-FileHash -LiteralPath (Join-Path $installed 'yyplayer.exe')).Hash
    $oldDllHash = (Get-FileHash -LiteralPath (Join-Path $installed 'runtime/fixture.dll')).Hash
    $lock = [IO.File]::Open((Join-Path $installed 'runtime/fixture.dll'), [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try { Invoke-Installer $new @('/S') 2 } finally { $lock.Dispose() }
    Ensure ((Read-InstallValue 'DisplayVersion') -eq '0.5.1') 'Failed upgrade changed registry version.'
    Ensure ((Get-FileHash -LiteralPath (Join-Path $installed 'yyplayer.exe')).Hash -eq $oldExeHash) 'Failed upgrade did not restore EXE.'
    Ensure ((Get-FileHash -LiteralPath (Join-Path $installed 'runtime/fixture.dll')).Hash -eq $oldDllHash) 'Failed upgrade changed the locked DLL.'
    $cases.Add('locked-dll-failure-rolls-back-exe-and-version')
    Invoke-Installer $new @('/S', "/D=$(Join-Path $testRoot 'wrong directory')")
    Ensure ((Read-InstallValue 'DisplayVersion') -eq '0.5.4') 'Upgrade did not update the version.'
    Ensure ((Read-InstallValue 'LastInstallAction') -eq 'upgrade') 'Existing install was not detected as upgrade.'
    Ensure ((Read-InstallValue 'InstallLocation') -eq $installed) 'Upgrade changed the original directory.'
    Ensure ([IO.File]::ReadAllText((Join-Path $installed 'yyplayer.exe')) -eq 'fixture-new') 'Upgrade used the wrong EXE.'
    Ensure (-not (Test-Path -LiteralPath (Join-Path $testRoot 'wrong directory'))) 'Upgrade created a duplicate install.'
    Ensure ((Get-FileHash -LiteralPath $externalConfig).Hash -eq $configHash) 'Upgrade changed independent settings.'
    Ensure ([IO.File]::ReadAllText($userFile) -eq 'preserve me') 'Upgrade deleted an unowned file.'
    $cases.Add('0.5.1-to-0.5.4-upgrade-keeps-location-and-user-files')
    Invoke-Installer $new @('/S')
    Ensure ((Read-InstallValue 'LastInstallAction') -eq 'repair') 'Same-version reinstall is not detected.'
    $cases.Add('same-version-reinstall')
    Invoke-Installer $older @('/S') 3
    Ensure ((Read-InstallValue 'DisplayVersion') -eq '0.5.4') 'Downgrade changed registry.'
    Ensure ([IO.File]::ReadAllText((Join-Path $installed 'yyplayer.exe')) -eq 'fixture-new') 'Downgrade overwrote files.'
    $cases.Add('downgrade-blocked')
    Invoke-Installer (Join-Path $installed 'uninstall.exe') @('/S', "_?=$installed")
    Ensure (-not (Read-InstallValue 'DisplayVersion')) 'Uninstall left the private registry entry.'
    Ensure (-not (Test-Path -LiteralPath (Join-Path $installed 'yyplayer.exe'))) 'Uninstall left owned program files.'
    Ensure (Test-Path -LiteralPath $userFile) 'Uninstall deleted an unowned file.'
    Ensure ((Get-FileHash -LiteralPath $externalConfig).Hash -eq $configHash) 'Uninstall changed user data.'
    $cases.Add('uninstall-keeps-unowned-files-and-settings')
    @{ result = 'PASS'; cases = $cases.ToArray(); fixture = $testRoot; registry_id = $appId } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $testRoot 'results.json') -Encoding UTF8
    Write-Output "PASS: $($cases.Count) isolated installer checks. Evidence: $testRoot"
} finally {
    $registry.DeleteSubKeyTree($keyPath, $false)
    $registry.Dispose()
    $env:YYPLAYER_PACKAGE_STAGE = $previousStage
    $env:YYPLAYER_PACKAGE_OUTPUT = $previousOutput
}
