param(
    [Parameter(Mandatory = $true)]
    [string[]]$Files,
    [string]$ReportPath = 'target/release-security.json'
)

$ErrorActionPreference = 'Stop'
$status = Get-MpComputerStatus -ErrorAction Stop
if (!$status.AMServiceEnabled -or !$status.AntivirusEnabled) {
    throw 'Microsoft Defender is unavailable. Release files have not been checked.'
}
$platform = Join-Path $env:ProgramData 'Microsoft/Windows Defender/Platform'
$scannerCandidates = @(
    (Join-Path $platform 'CurrentVersion/MpCmdRun.exe')
    Get-ChildItem -LiteralPath $platform -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName 'MpCmdRun.exe' }
    (Join-Path $env:ProgramFiles 'Windows Defender/MpCmdRun.exe')
)
$scanner = $scannerCandidates | Where-Object {
    Test-Path -LiteralPath $_ -PathType Leaf
} | Select-Object -First 1
if (!$scanner) { throw 'The Microsoft Defender command-line scanner was not found.' }

$results = @(foreach ($path in $Files) {
    $file = Get-Item -LiteralPath $path -ErrorAction Stop
    if ($file.PSIsContainer) { throw 'Release security checks require explicit file paths.' }
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    # This diagnostic scan reports detections without changing files or protection settings.
    # It ignores exclusions and scans archive contents, including the installer payload.
    $output = @(& $scanner -Scan -ScanType 3 -File $file.FullName -DisableRemediation 2>&1)
    $scanExit = $LASTEXITCODE
    $output | ForEach-Object { Write-Host $_ }
    [PSCustomObject]@{
        File = $file.Name
        SHA256 = $hash
        AuthenticodeStatus = $signature.Status.ToString()
        ScannerExitCode = $scanExit
    }
})
$report = [PSCustomObject]@{
    CheckedAt = [DateTimeOffset]::Now.ToString('o')
    EngineVersion = $status.AMEngineVersion
    SignatureVersion = $status.AntivirusSignatureVersion
    Files = $results
}
$reportFile = [IO.Path]::GetFullPath($ReportPath)
[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($reportFile)) | Out-Null
[IO.File]::WriteAllText($reportFile, ($report | ConvertTo-Json -Depth 4))
if (@($results | Where-Object { $_.ScannerExitCode -ne 0 }).Count -gt 0) {
    throw 'Release publication is blocked: Defender detected a threat or could not complete a scan. See the scan output and security report.'
}
Write-Host 'Microsoft Defender completed the release scans without detections. This is a point-in-time check, not a safety guarantee.'
