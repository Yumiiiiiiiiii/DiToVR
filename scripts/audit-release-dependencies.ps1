$ErrorActionPreference = 'Stop'
npm audit --omit=dev
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$auditDir = Join-Path $env:RUNNER_TEMP 'cargo-audit'
New-Item -ItemType Directory -Path $auditDir -Force | Out-Null
$archive = Join-Path $auditDir 'cargo-audit.zip'
Invoke-WebRequest -Uri 'https://github.com/rustsec/rustsec/releases/download/cargo-audit/v0.22.2/cargo-audit-x86_64-pc-windows-msvc-v0.22.2.zip' -OutFile $archive
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne '0a7316540862c13d954f648917ceacca593747baed6eec180fafa590be2710ab') {
    throw 'The cargo-audit download does not match the pinned SHA-256.'
}
Expand-Archive -LiteralPath $archive -DestinationPath $auditDir -Force
$audit = Join-Path $auditDir 'cargo-audit-x86_64-pc-windows-msvc-v0.22.2/cargo-audit.exe'
& $audit audit --file Cargo.lock
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& $audit audit --file src-tauri/Cargo.lock
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
