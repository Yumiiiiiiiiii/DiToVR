; Hooks are included before NSIS defines MANUFACTURER. Keep this name aligned
; with bundle.publisher in tauri.conf.json.
VIAddVersionKey "CompanyName" "DiToVR"
Var DiToVRPreviousInstallDir

; Tauri keys installations by productName. Remove the previous named install
; using its own updater mode so settings survive and two app entries do not remain.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R8 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Discord to VR" "InstallLocation"
  ${If} $R8 != ""
    StrCpy $R9 $R8 1
    ${If} $R9 == '$\"'
      StrCpy $R8 $R8 -1 1
    ${EndIf}
    ${If} ${FileExists} "$R8\discord_To_VR.exe"
    ${AndIf} ${FileExists} "$R8\uninstall.exe"
      !insertmacro CheckIfAppIsRunning "$R8\discord_To_VR.exe" "Discord to VR"
      SetOutPath $TEMP
      ExecWait '$\"$R8\uninstall.exe$\" /S /UPDATE _?=$R8' $R9
      ${If} $R9 != 0
      ${OrIf} ${FileExists} "$R8\discord_To_VR.exe"
        MessageBox MB_ICONSTOP "以前のバージョンを終了・削除できませんでした。設定は保持されています。"
        Abort
      ${EndIf}
      StrCpy $DiToVRPreviousInstallDir $R8
      SetOutPath $INSTDIR
    ${EndIf}
  ${EndIf}
!macroend

; /UPDATE preserves the old shortcuts. Recreate only shortcuts that point to
; the verified previous executable; preserve the user's choice not to have one.
!macro DiToVRMigrateShortcut oldShortcut newShortcut
  !insertmacro IsShortcutTarget "${oldShortcut}" "$DiToVRPreviousInstallDir\discord_To_VR.exe"
  Pop $0
  ${If} $0 = 1
    ${IfNot} ${FileExists} "${newShortcut}"
      CreateShortcut "${newShortcut}" "$INSTDIR\DiToVR.exe"
      !insertmacro SetLnkAppUserModelId "${newShortcut}"
    ${EndIf}
    !insertmacro IsShortcutTarget "${newShortcut}" "$INSTDIR\DiToVR.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "${oldShortcut}"
      Delete "${oldShortcut}"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $DiToVRPreviousInstallDir != ""
    !insertmacro DiToVRMigrateShortcut "$SMPROGRAMS\Discord to VR.lnk" "$SMPROGRAMS\DiToVR.lnk"
    !insertmacro DiToVRMigrateShortcut "$DESKTOP\Discord to VR.lnk" "$DESKTOP\DiToVR.lnk"
  ${EndIf}
!macroend

; The application stores settings outside Tauri's bundle-ID directory.
; Match Tauri's explicit data-removal choice and preserve settings on updates.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "$APPDATA\DiToVR"
    RmDir /r "$APPDATA\discord-to-vr"
  ${EndIf}
!macroend
