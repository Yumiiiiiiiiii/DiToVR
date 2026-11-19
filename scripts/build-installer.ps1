param(
    [switch]$Sign,
    [string]$Makensis
)
# Builds the standard (non-Store) NSIS installer from the native desktop app.
# Output: target/installer/DiToVR_<version>_x64-setup.exe (and .sig with -Sign).
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $repoRoot
$cargoToml = Get-Content -Raw -Encoding UTF8 (Join-Path $repoRoot 'desktop/Cargo.toml')
if ($cargoToml -notmatch '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"') { throw 'Could not read the desktop version.' }
$version = $Matches[1]
if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    $env:CARGO_TARGET_DIR = Join-Path $repoRoot 'target/desktop'
} elseif (![IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    $env:CARGO_TARGET_DIR = [IO.Path]::GetFullPath((Join-Path $repoRoot $env:CARGO_TARGET_DIR))
}
cargo build --release --locked --manifest-path desktop/Cargo.toml
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$exe = Join-Path $env:CARGO_TARGET_DIR 'release/DiToVR.exe'
if (!(Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'DiToVR.exe was not built.' }
if (!$Makensis) {
    $found = Get-Command makensis -ErrorAction SilentlyContinue
    if ($found) { $Makensis = $found.Source }
    elseif (Test-Path 'C:/Program Files (x86)/NSIS/makensis.exe') { $Makensis = 'C:/Program Files (x86)/NSIS/makensis.exe' }
    elseif (Test-Path (Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe')) { $Makensis = Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe' }
    else { throw 'NSIS (makensis) is required. Install NSIS or pass -Makensis.' }
}
$outDir = Join-Path $repoRoot 'target/installer'
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
$setup = Join-Path $outDir "DiToVR_${version}_x64-setup.exe"
& $Makensis -V2 "-DVERSION=$version" "-DEXE=$exe" "-DOUT=$setup" (Join-Path $repoRoot 'desktop/installer/DiToVR.nsi')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($Sign) {
    if ([string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY)) {
        $key = Join-Path $env:LOCALAPPDATA 'DiToVR-release-signing/updater.key'
        if (!(Test-Path -LiteralPath $key -PathType Leaf)) { $key = Join-Path $env:LOCALAPPDATA 'DiscordToVR-release-signing/updater.key' }
        if (!(Test-Path -LiteralPath $key -PathType Leaf)) { throw 'Set TAURI_SIGNING_PRIVATE_KEY to the existing updater key. Do not regenerate it.' }
        $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -Raw -Encoding UTF8 $key).Trim()
    }
    $signTarget = Join-Path $repoRoot 'target/sign-update'
    cargo run --release --locked --manifest-path tools/sign-update/Cargo.toml --target-dir $signTarget -- $setup $version
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
Write-Host "Installer: $setup"
