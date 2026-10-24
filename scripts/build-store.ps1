param([string]$CompilerRoot)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $repoRoot
$env:CARGO_TARGET_DIR = Join-Path $repoRoot 'target/store-build'
if ($CompilerRoot) {
    $compiler = Get-ChildItem -LiteralPath $CompilerRoot -Directory |
        Where-Object { $_.Name -like 'llvm-mingw-*-msvcrt-x86_64' } |
        Sort-Object Name -Descending | Select-Object -First 1
    if (!$compiler) { throw 'Portable LLVM-MinGW compiler was not found.' }
    $compilerBin = Join-Path $compiler.FullName 'bin'
    $runtime = Join-Path $CompilerRoot 'gcc-runtime'
    foreach ($file in @('libgcc.a', 'libgcc_eh.a')) {
        if (!(Test-Path -LiteralPath (Join-Path $runtime $file))) { throw "Missing GNU runtime: $file" }
    }
    $env:PATH = $compilerBin + ';' + $env:PATH
    $env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
    $env:CARGO_BUILD_TARGET = $null
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $compilerBin 'x86_64-w64-mingw32-clang.exe'
    $env:CC_x86_64_pc_windows_gnu = $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER
    $env:RUSTFLAGS = $null
    $env:CARGO_ENCODED_RUSTFLAGS = '-L' + [char]31 + 'native=' + $runtime
}
$makeappx = Get-ChildItem 'C:/Program Files (x86)/Windows Kits/10/bin/*/x64/makeappx.exe' |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (!$makeappx) { throw 'Install Windows 10/11 SDK (MakeAppx) before packaging.' }
& node node_modules/@tauri-apps/cli/tauri.js build --ci --no-bundle --features microsoft-store -- --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Fresh isolated staging prevents stale binaries or private files entering MSIX.
$output = Join-Path $repoRoot ('target/store/' + [guid]::NewGuid().ToString('N'))
$icons = Join-Path $output 'icons'
$stage = Join-Path $output 'stage'
New-Item -ItemType Directory -Path $output | Out-Null
& node node_modules/@tauri-apps/cli/tauri.js icon src-tauri/icons/icon.svg --output $icons
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$exe = Join-Path $env:CARGO_TARGET_DIR 'release/discord_To_VR.exe'
if ($CompilerRoot -and !(Test-Path -LiteralPath (Join-Path (Split-Path $exe -Parent) 'WebView2Loader.dll'))) {
    throw 'The GNU build requires WebView2Loader.dll beside the executable.'
}
& node scripts/store-manifest.mjs $stage $exe $icons
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$version = (Get-Content -Raw -Encoding UTF8 src-tauri/tauri.conf.json | ConvertFrom-Json).version
$package = Join-Path $output "DiscordToVR_${version}.0_x64.msix"
$securityReport = Join-Path $output 'security-report.json'
# Emit artifact paths before scanning so a failed scan report can be retained.
if ($env:GITHUB_OUTPUT) {
    Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "package_path=$package" -Encoding utf8
    Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "security_report_path=$securityReport" -Encoding utf8
}
# Do not pass /nv: SDK schema/semantic validation is required.
& $makeappx.FullName pack /d $stage /p $package /o
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$scanFiles = @((Join-Path $stage 'discord_To_VR.exe'), $package)
$loader = Join-Path $stage 'WebView2Loader.dll'
if (Test-Path -LiteralPath $loader) { $scanFiles += $loader }
& (Join-Path $PSScriptRoot 'check-release-security.ps1') -Files $scanFiles -ReportPath $securityReport
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Store upload package (unsigned; Microsoft signs it after certification): $package"
Write-Host 'Do not distribute this unsigned MSIX directly to users.'
