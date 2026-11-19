; DiToVR installer (NSIS, per-user). Replaces the Tauri-generated installer while keeping:
;  - install dir / uninstall registry key "DiToVR" (so the in-app updater finds the install)
;  - /P (passive) /UPDATE /R (relaunch) /ARGS used by the signed in-app updater
;  - migration from the pre-rename "Discord to VR" install, settings preserved
; Build: makensis /DVERSION=x.y.z /DEXE=<path>\DiToVR.exe /DOUT=<setup.exe> DiToVR.nsi
Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"
!include "nsDialogs.nsh"

!ifndef VERSION
  !error "Pass /DVERSION=x.y.z"
!endif
!ifndef EXE
  !error "Pass /DEXE=path to DiToVR.exe"
!endif
!ifndef OUT
  !define OUT "DiToVR_${VERSION}_x64-setup.exe"
!endif
!ifndef ROOT
  !define ROOT "..\.."
!endif

!define PRODUCT "DiToVR"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT}"
!define MANUFACTURER "DiToVR"

Name "${PRODUCT}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\${PRODUCT}"
InstallDirRegKey HKCU "Software\${MANUFACTURER}\${PRODUCT}" ""
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${PRODUCT} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "CompanyName" "${MANUFACTURER}"
VIAddVersionKey "FileDescription" "${PRODUCT} setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT"

Var UpdateMode
Var PassiveMode
Var Relaunch
Var RelaunchArgs
Var DeleteAppDataCheckboxState
Var DeleteAppDataCheckbox
Var PreviousInstallDir

!define MUI_ICON "${ROOT}\desktop\assets\icons\icon.ico"
!define MUI_UNICON "${ROOT}\desktop\assets\icons\icon.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\DiToVR.exe"
!insertmacro MUI_PAGE_LICENSE "${ROOT}\LICENSE"
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.ConfirmShow
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.ConfirmLeave
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Japanese"
!insertmacro MUI_LANGUAGE "English"

!include "cleanup.nsh"

Function .onInit
  ${GetParameters} $R0
  ${GetOptions} $R0 "/UPDATE" $R1
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/P" $R1
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/R" $R1
  ${IfNot} ${Errors}
    StrCpy $Relaunch 1
  ${EndIf}
  ClearErrors
  ; /ARGS takes the rest of the command line: the original app arguments to relaunch with.
  ${GetOptions} $R0 "/ARGS" $RelaunchArgs
  ClearErrors
  ${If} $PassiveMode = 1
    SetSilent silent
  ${EndIf}
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "このアプリは 64 ビット版の Windows が必要です。"
    Abort
  ${EndIf}
FunctionEnd

Function CloseRunningApp
  nsExec::ExecToStack 'cmd /c tasklist /NH /FI "IMAGENAME eq DiToVR.exe" | find /I "DiToVR.exe"'
  Pop $0
  Pop $1
  ${If} $0 = 0
    ${If} $UpdateMode = 1
      ; The updating app exits itself; give it a moment, then end it if still present.
      Sleep 1500
    ${Else}
      MessageBox MB_OKCANCEL|MB_ICONINFORMATION "DiToVR が実行中です。OK を押すと終了してインストールを続けます。" IDOK +2
      Abort
    ${EndIf}
    nsExec::Exec 'taskkill /F /IM DiToVR.exe'
    Sleep 800
  ${EndIf}
FunctionEnd

; Migrate from the pre-rename "Discord to VR" install: uninstall it in update mode so settings survive.
Function MigratePreviousInstall
  ReadRegStr $R8 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Discord to VR" "InstallLocation"
  ${If} $R8 != ""
    StrCpy $R9 $R8 1
    ${If} $R9 == '$\"'
      StrCpy $R8 $R8 -1 1
    ${EndIf}
    ${If} ${FileExists} "$R8\discord_To_VR.exe"
    ${AndIf} ${FileExists} "$R8\uninstall.exe"
      nsExec::Exec 'taskkill /F /IM discord_To_VR.exe'
      SetOutPath $TEMP
      ExecWait '$\"$R8\uninstall.exe$\" /S /UPDATE _?=$R8' $R9
      ${If} $R9 != 0
      ${OrIf} ${FileExists} "$R8\discord_To_VR.exe"
        MessageBox MB_ICONSTOP "以前のバージョンを終了・削除できませんでした。設定は保持されています。"
        Abort
      ${EndIf}
      StrCpy $PreviousInstallDir $R8
    ${EndIf}
  ${EndIf}
FunctionEnd

Section "Install"
  Call CloseRunningApp
  Call MigratePreviousInstall
  SetOutPath "$INSTDIR"
  File "/oname=DiToVR.exe" "${EXE}"
  File "${ROOT}\LICENSE"
  File "${ROOT}\THIRD_PARTY_AUDIO_NOTICES.txt"
  File "${ROOT}\THIRD_PARTY_UI_NOTICES.txt"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\${MANUFACTURER}\${PRODUCT}" "" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCT}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\DiToVR.exe$\""
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
  ; Shortcuts: an update keeps the user's existing choices; a fresh or migrated install creates them.
  ${If} $UpdateMode <> 1
  ${OrIf} $PreviousInstallDir != ""
    CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\DiToVR.exe"
    ${IfNot} ${FileExists} "$DESKTOP\${PRODUCT}.lnk"
      ${If} $PreviousInstallDir == ""
      ${OrIf} ${FileExists} "$DESKTOP\Discord to VR.lnk"
        CreateShortcut "$DESKTOP\${PRODUCT}.lnk" "$INSTDIR\DiToVR.exe"
      ${EndIf}
    ${EndIf}
    ${If} $PreviousInstallDir != ""
      Delete "$SMPROGRAMS\Discord to VR.lnk"
      Delete "$DESKTOP\Discord to VR.lnk"
    ${EndIf}
  ${EndIf}
  ${If} $Relaunch = 1
    Exec '"$INSTDIR\DiToVR.exe" $RelaunchArgs'
  ${EndIf}
SectionEnd

Function un.onInit
  ${GetParameters} $R0
  ${GetOptions} $R0 "/UPDATE" $R1
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}
  ClearErrors
  StrCpy $DeleteAppDataCheckboxState 0
FunctionEnd

Function un.ConfirmShow
  ${If} $UpdateMode <> 1
    ${NSD_CreateCheckbox} 0 150u 100% 12u "アプリの設定も削除する (Delete the application data)"
    Pop $DeleteAppDataCheckbox
  ${EndIf}
FunctionEnd
Function un.ConfirmLeave
  ${If} $UpdateMode <> 1
    ${NSD_GetState} $DeleteAppDataCheckbox $DeleteAppDataCheckboxState
  ${EndIf}
FunctionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /F /IM DiToVR.exe'
  Delete "$INSTDIR\DiToVR.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_AUDIO_NOTICES.txt"
  Delete "$INSTDIR\THIRD_PARTY_UI_NOTICES.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  ${If} $UpdateMode <> 1
    Delete "$SMPROGRAMS\${PRODUCT}.lnk"
    Delete "$DESKTOP\${PRODUCT}.lnk"
    DeleteRegKey HKCU "${UNINSTKEY}"
    DeleteRegKey HKCU "Software\${MANUFACTURER}\${PRODUCT}"
  ${EndIf}
  !insertmacro DITOVR_POSTUNINSTALL
SectionEnd
