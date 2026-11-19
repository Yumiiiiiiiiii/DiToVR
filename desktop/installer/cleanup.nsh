; Uninstall data cleanup, kept in its own file so scripts/test-installer-hooks.ps1 can run it in isolation.
; Settings are removed only when the user explicitly asked for it and this is not an update.
!macro DITOVR_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "$APPDATA\DiToVR"
    RmDir /r "$APPDATA\discord-to-vr"
  ${EndIf}
!macroend
