param([string]$Makensis = (Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe'))
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
$fixture = Join-Path $workspace ('target/rename-installer-fixture-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
$fixture = (Resolve-Path -LiteralPath $fixture).Path
if (!$fixture.StartsWith($workspace + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe fixture directory.' }
$oldDir = Join-Path $fixture 'old app with spaces'
$newDir = Join-Path $fixture 'DiToVR'
$shortcutDir = Join-Path $fixture 'shortcuts'
New-Item -ItemType Directory -Path $oldDir, $newDir, $shortcutDir | Out-Null
[IO.File]::WriteAllText((Join-Path $oldDir 'discord_To_VR.exe'), 'synthetic old app')
[IO.File]::WriteAllText((Join-Path $oldDir 'config.json'), 'keep settings')
$shell = New-Object -ComObject WScript.Shell
$oldLink = $shell.CreateShortcut((Join-Path $shortcutDir 'Discord to VR.lnk'))
$oldLink.TargetPath = Join-Path $oldDir 'discord_To_VR.exe'
$oldLink.Save()
$fake = @'
Unicode true
!include "FileFunc.nsh"
Name "Isolated previous uninstaller"
OutFile "@OLD@\uninstall.exe"
RequestExecutionLevel user
SilentInstall silent
Section
  ${GetParameters} $0
  FileOpen $1 "@ROOT@\uninstall-arguments.txt" w
  FileWrite $1 $0
  FileClose $1
  Delete "@OLD@\discord_To_VR.exe"
SectionEnd
'@
$fakeScript = Join-Path $fixture 'old.nsi'
[IO.File]::WriteAllText($fakeScript, $fake.Replace('@OLD@', $oldDir).Replace('@ROOT@', $fixture), [Text.UTF8Encoding]::new($false))
& $Makensis /V2 $fakeScript
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$hook = Get-Content -Raw -Encoding UTF8 (Join-Path $workspace 'src-tauri/installer-hooks.nsh')
$read = 'ReadRegStr $R8 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Discord to VR" "InstallLocation"'
if ($hook.Split(@($read), [StringSplitOptions]::None).Count -ne 2) { throw 'Review the rename hook registry lookup before running this fixture.' }
# Substitute only the old-install lookup. Never create or touch real app registry keys.
$hook = $hook.Replace($read, 'StrCpy $R8 "$\"' + $oldDir + '$\""')
$hook = $hook.Replace('RmDir /r "$APPDATA\DiToVR"', 'RmDir /r "' + (Join-Path $fixture 'unused-current-settings') + '"').
    Replace('RmDir /r "$APPDATA\discord-to-vr"', 'RmDir /r "' + (Join-Path $fixture 'unused-old-settings') + '"')
$hook = $hook.Replace('$SMPROGRAMS\', $shortcutDir + '\').Replace('$DESKTOP\', (Join-Path $fixture 'absent-desktop') + '\')
# Reuse Tauri's actual shortcut macros. Keep process termination a fixture stub.
$utilsPath = Join-Path $workspace 'target/tauri/release/nsis/x64/utils.nsh'
$utils = Get-Content -Raw -Encoding UTF8 $utilsPath
$utils = [regex]::Replace($utils, '(?ms)^!macro CheckIfAppIsRunning .*?^!macroend', '')
$utilsFile = Join-Path $fixture 'utils.nsh'
[IO.File]::WriteAllText($utilsFile, $utils, [Text.UTF8Encoding]::new($false))
$hookFile = Join-Path $fixture 'hook.nsh'
[IO.File]::WriteAllText($hookFile, $hook, [Text.UTF8Encoding]::new($false))
$script = @'
Unicode true
!include "LogicLib.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "@UTILS@"
!define BUNDLEID "isolated.rename.fixture"
Name "DiToVR isolated rename fixture"
VIProductVersion "0.0.0.0"
VIAddVersionKey "FileVersion" "0.0.0.0"
VIAddVersionKey "FileDescription" "Isolated rename test"
VIAddVersionKey "LegalCopyright" "Test fixture"
OutFile "@ROOT@\fixture.exe"
RequestExecutionLevel user
SilentInstall silent
Var DeleteAppDataCheckboxState
Var UpdateMode
!macro CheckIfAppIsRunning executablePath productName
  ; A harmless synthetic file is never running. The shipping hook uses Tauri's
  ; Restart Manager macro instead of this fixture-only replacement.
!macroend
!include "@HOOK@"
Section
  StrCpy $INSTDIR "@NEW@"
  !insertmacro NSIS_HOOK_PREINSTALL
  FileOpen $0 "$INSTDIR\DiToVR.exe" w
  FileWrite $0 "synthetic new app"
  FileClose $0
  !insertmacro NSIS_HOOK_POSTINSTALL
  FileOpen $0 "@ROOT@\completed.txt" w
  FileWrite $0 "completed"
  FileClose $0
SectionEnd
'@
$mainScript = Join-Path $fixture 'fixture.nsi'
[IO.File]::WriteAllText($mainScript, $script.Replace('@ROOT@', $fixture).Replace('@HOOK@', $hookFile).Replace('@NEW@', $newDir).Replace('@UTILS@', $utilsFile), [Text.UTF8Encoding]::new($false))
& $Makensis /V2 $mainScript
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$process = Start-Process -FilePath (Join-Path $fixture 'fixture.exe') -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0 -or !(Test-Path -LiteralPath (Join-Path $fixture 'completed.txt'))) { throw 'Rename fixture did not complete.' }
if (Test-Path -LiteralPath (Join-Path $oldDir 'discord_To_VR.exe')) { throw 'Previous executable was not removed by its uninstaller.' }
if ([IO.File]::ReadAllText((Join-Path $oldDir 'config.json')) -ne 'keep settings') { throw 'Settings were changed.' }
$arguments = [IO.File]::ReadAllText((Join-Path $fixture 'uninstall-arguments.txt'))
if (!$arguments.Contains('/UPDATE') -or !$arguments.Contains('_?=' + $oldDir)) { throw 'The old uninstaller must preserve data and receive the full path including spaces.' }
if (Test-Path -LiteralPath (Join-Path $shortcutDir 'Discord to VR.lnk')) { throw 'Old shortcut was not renamed.' }
$newLink = $shell.CreateShortcut((Join-Path $shortcutDir 'DiToVR.lnk'))
if ($newLink.TargetPath -ne (Join-Path $newDir 'DiToVR.exe')) { throw 'Renamed shortcut does not target the new executable.' }
if (Test-Path -LiteralPath (Join-Path $fixture 'absent-desktop/DiToVR.lnk')) { throw 'A desktop shortcut was created without a previous shortcut.' }
Write-Host 'Installer rename fixture passed: update mode preserved settings and migrated the existing shortcut to the new executable.'
