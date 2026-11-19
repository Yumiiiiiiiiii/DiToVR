param()
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $repoRoot
$env:CARGO_TARGET_DIR = Join-Path $repoRoot 'target/store-build'
$makeappx = Get-ChildItem 'C:/Program Files (x86)/Windows Kits/10/bin/*/x64/makeappx.exe' |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (!$makeappx) { throw 'Install Windows 10/11 SDK (MakeAppx) before packaging.' }
cargo build --release --locked --features microsoft-store --manifest-path desktop/Cargo.toml
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Fresh isolated staging prevents stale binaries or private files entering MSIX.
$output = Join-Path $repoRoot ('target/store/' + [guid]::NewGuid().ToString('N'))
$icons = Join-Path $output 'icons'
$stage = Join-Path $output 'stage'
New-Item -ItemType Directory -Path $output | Out-Null
# Store logos are resized from the production icon (StoreLogo 50, Square150x150Logo 150, Square44x44Logo 44).
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Path $icons -Force | Out-Null
$source = [System.Drawing.Image]::FromFile((Join-Path $repoRoot 'desktop/assets/icons/icon.png'))
foreach ($logo in @(@('StoreLogo.png', 50), @('Square150x150Logo.png', 150), @('Square44x44Logo.png', 44))) {
    $size = $logo[1]
    $bitmap = New-Object System.Drawing.Bitmap $size, $size
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.DrawImage($source, 0, 0, $size, $size)
    $graphics.Dispose()
    $bitmap.Save((Join-Path $icons $logo[0]), [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}
$source.Dispose()
$exe = Join-Path $env:CARGO_TARGET_DIR 'release/DiToVR.exe'
& node scripts/store-manifest.mjs $stage $exe $icons
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ((Get-Content -Raw -Encoding UTF8 desktop/Cargo.toml) -notmatch '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"') { throw 'Could not read the desktop version.' }
$version = $Matches[1]
$package = Join-Path $output "DiToVR_${version}.0_x64.msix"
$securityReport = Join-Path $output 'security-report.json'
# Emit artifact paths before scanning so a failed scan report can be retained.
if ($env:GITHUB_OUTPUT) {
    Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "package_path=$package" -Encoding utf8
    Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "security_report_path=$securityReport" -Encoding utf8
}
# Do not pass /nv: SDK schema/semantic validation is required.
& $makeappx.FullName pack /d $stage /p $package /o
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$scanFiles = @((Join-Path $stage 'DiToVR.exe'), $package)
& (Join-Path $PSScriptRoot 'check-release-security.ps1') -Files $scanFiles -ReportPath $securityReport
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Store upload package (unsigned; Microsoft signs it after certification): $package"
Write-Host 'Do not distribute this unsigned MSIX directly to users.'
