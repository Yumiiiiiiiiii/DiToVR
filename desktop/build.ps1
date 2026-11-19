param([switch]$Release, [switch]$Check, [switch]$Clippy, [switch]$Test, [switch]$Store)
$ErrorActionPreference = 'Stop'
# Standard and Store builds share the exe name, so use separate target directories.
$kind = if ($Store) { 'store' } else { 'standard' }
$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA "DiToVRBuildCache/desktop-$kind"
$vsLocator = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path -LiteralPath $vsLocator) {
    $vsInstall = & $vsLocator -latest -products '*' -property installationPath
    if (!$vsInstall) { throw 'Visual Studio C++ Build Tools is required.' }
    $devCommand = Join-Path $vsInstall 'Common7/Tools/VsDevCmd.bat'
    $environmentLines = & cmd.exe /d /c "call `"$devCommand`" -arch=x64 -host_arch=x64 >nul && set"
    foreach ($environmentLine in $environmentLines) {
        if ($environmentLine -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') }
    }
}
$verb = if ($Check) { 'check' } elseif ($Clippy) { 'clippy' } elseif ($Test) { 'test' } else { 'build' }
$buildArgs = @($verb, '--locked', '--manifest-path', (Join-Path $PSScriptRoot 'Cargo.toml'))
if ($Release) { $buildArgs += '--release' }
if ($Store) { $buildArgs += @('--features', 'microsoft-store') }
if ($Clippy) { $buildArgs += @('--', '-D', 'warnings') }
& cargo @buildArgs
exit $LASTEXITCODE
