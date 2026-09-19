; ./src-tauri/windows/hooks.nsh
; NSIS hooks for Genomics Caddy.
; Reads uninstall-library.txt (the resolved library, including a custom folder).
; Current-user installs already remove $INSTDIR; extra prompt only if the library
; lives outside the install folder. Everyone installs never touch other accounts.

!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $R7 ""
  IfFileExists "$LOCALAPPDATA\Genomics Caddy\uninstall-library.txt" 0 try_instdir_hint
    FileOpen $R8 "$LOCALAPPDATA\Genomics Caddy\uninstall-library.txt" r
    FileRead $R8 $R7
    FileClose $R8
    Goto have_library_hint
  try_instdir_hint:
  IfFileExists "$INSTDIR\uninstall-library.txt" 0 have_library_hint
    FileOpen $R8 "$INSTDIR\uninstall-library.txt" r
    FileRead $R8 $R7
    FileClose $R8
  have_library_hint:
  StrCpy $R9 $R7 "" -1
  ${If} $R9 == "$\r"
  ${OrIf} $R9 == "$\n"
    StrCpy $R7 $R7 -1
  ${EndIf}
  StrCpy $R9 $R7 "" -1
  ${If} $R9 == "$\r"
  ${OrIf} $R9 == "$\n"
    StrCpy $R7 $R7 -1
  ${EndIf}

  ${If} $MultiUser.InstallMode == "AllUsers"
    MessageBox MB_YESNO|MB_ICONQUESTION "Also delete this Windows account's genomes and downloads? Other accounts on this PC are left alone." IDNO skip_allusers_library
      StrLen $R5 $R7
      ${If} $R5 > 8
        RMDir /r "$R7"
      ${EndIf}
      RMDir /r "$LOCALAPPDATA\Genomics Caddy\Data"
      Delete "$LOCALAPPDATA\Genomics Caddy\data-location.json"
      Delete "$LOCALAPPDATA\Genomics Caddy\uninstall-library.txt"
      RMDir "$LOCALAPPDATA\Genomics Caddy"
    skip_allusers_library:
  ${Else}
    StrLen $R5 $INSTDIR
    StrCpy $R6 $R7 $R5
    ${If} $R7 != ""
    ${AndIf} $R6 != $INSTDIR
      MessageBox MB_YESNO|MB_ICONQUESTION "Also delete the custom library folder?$\r$\n$R7" IDNO skip_custom_library
        StrLen $R9 $R7
        ${If} $R9 > 8
          RMDir /r "$R7"
        ${EndIf}
      skip_custom_library:
    ${ElseIf} $R7 != ""
      StrLen $R9 $R7
      ${If} $R9 > 8
        RMDir /r "$R7"
      ${EndIf}
    ${EndIf}
  ${EndIf}
  RMDir /r "$LOCALAPPDATA\com.dna.explorer"
!macroend
