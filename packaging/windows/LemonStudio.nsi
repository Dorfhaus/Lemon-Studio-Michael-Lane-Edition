Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!ifndef PAYLOAD
!error "Pass /DPAYLOAD=staged payload directory"
!endif
!ifndef OUTPUT
!error "Pass /DOUTPUT=installer path"
!endif
Name "ML Studio - Michael Lane Edition"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\ML Studio"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ABORTWARNING
!define MUI_ICON "..\..\crates\os-app\assets\lemonstudio.ico"
!define MUI_WELCOMEPAGE_TEXT "Install the free Michael Lane customized edition of ML Studio.$\r$\n$\r$\nIncludes a native-only starter template, built-in instruments and effects, and the matching source code.$\r$\n$\r$\nClose ML Studio before installing. Existing projects and preferences are preserved."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!ifdef TEST_INSTALL
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\LemonStudio-Package-Test"
!define SHORTCUT "$INSTDIR\ML Studio.lnk"
!else
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\LemonStudio-MichaelLane"
!define SHORTCUT "$DESKTOP\ML Studio.lnk"
InstallDirRegKey HKCU "${REGKEY}" "InstallLocation"
!endif
Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "This installer requires 64-bit Windows."
    Abort
  ${EndIf}
!ifndef TEST_INSTALL
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -WindowStyle Hidden -Command "if (Get-Process lemonstudio -ErrorAction SilentlyContinue) { exit 1 } else { exit 0 }"'
  Pop $0
  Pop $1
  ${If} $0 != 0
    IfSilent +2
    MessageBox MB_ICONEXCLAMATION "Save your work and close ML Studio before installing."
    Abort
  ${EndIf}
!endif
FunctionEnd
Section "ML Studio"
  SetOutPath "$INSTDIR"
  File "${PAYLOAD}\lemonstudio.exe"
  File "${PAYLOAD}\LICENSE"
  File "${PAYLOAD}\THIRD-PARTY-NOTICES.txt"
  File "${PAYLOAD}\READ ME.txt"
  File "${PAYLOAD}\Lemon Studio Source.zip"
  File "${PAYLOAD}\Lemon Starter.lmn"
  File "${PAYLOAD}\runtime\*.dll"
  WriteRegStr HKCU "Software\Classes\LemonStudio.LmnProject" "" "ML Studio project"
  WriteRegStr HKCU "Software\Classes\LemonStudio.LmnProject\DefaultIcon" "" '$\"$INSTDIR\lemonstudio.exe$\",0'
  WriteRegStr HKCU "Software\Classes\LemonStudio.LmnProject\shell\open\command" "" '$\"$INSTDIR\lemonstudio.exe$\" $\"%1$\"'
  WriteRegStr HKCU "Software\Classes\.lmn\OpenWithProgids" "LemonStudio.LmnProject" ""
  ReadRegStr $0 HKCU "Software\Classes\.lmn" ""
  StrCmp $0 "" 0 +2
  WriteRegStr HKCU "Software\Classes\.lmn" "" "LemonStudio.LmnProject"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
!ifndef TEST_INSTALL
  ; Retire the old shortcut name when upgrading the same installation.
  ReadRegStr $0 HKCU "${REGKEY}" "InstallLocation"
  ${If} $0 == $INSTDIR
    ${AndIf} ${FileExists} "$DESKTOP\Lemon Studio.lnk"
    ${AndIfNot} ${FileExists} "${SHORTCUT}"
    Rename "$DESKTOP\Lemon Studio.lnk" "${SHORTCUT}"
  ${EndIf}
!endif
  CreateShortcut "${SHORTCUT}" "$INSTDIR\lemonstudio.exe"
  WriteRegStr HKCU "${REGKEY}" "DisplayName" "ML Studio - Michael Lane Edition"
  WriteRegStr HKCU "${REGKEY}" "DisplayVersion" "2026.11.13.3"
  WriteRegStr HKCU "${REGKEY}" "Publisher" "Michael Lane"
  WriteRegStr HKCU "${REGKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${REGKEY}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "${REGKEY}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegDWORD HKCU "${REGKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${REGKEY}" "NoRepair" 1
  ; Use the installed app's validated cleanup, including during silent installation.
  nsExec::ExecToLog '"$INSTDIR\lemonstudio.exe" --cleanup-updates'
SectionEnd
Section "Uninstall"
  ReadRegStr $0 HKCU "Software\Classes\LemonStudio.LmnProject\shell\open\command" ""
  StrCmp $0 '$\"$INSTDIR\lemonstudio.exe$\" $\"%1$\"' 0 lmn_assoc_done
  DeleteRegKey HKCU "Software\Classes\LemonStudio.LmnProject"
  DeleteRegValue HKCU "Software\Classes\.lmn\OpenWithProgids" "LemonStudio.LmnProject"
  ReadRegStr $0 HKCU "Software\Classes\.lmn" ""
  StrCmp $0 "LemonStudio.LmnProject" 0 lmn_assoc_done
  DeleteRegValue HKCU "Software\Classes\.lmn" ""
  lmn_assoc_done:
  Delete "${SHORTCUT}"
  Delete "$INSTDIR\lemonstudio.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD-PARTY-NOTICES.txt"
  Delete "$INSTDIR\READ ME.txt"
  Delete "$INSTDIR\Lemon Studio Source.zip"
  Delete "$INSTDIR\Lemon Starter.lmn"
  Delete "$INSTDIR\concrt140.dll"
  Delete "$INSTDIR\msvcp140.dll"
  Delete "$INSTDIR\msvcp140_1.dll"
  Delete "$INSTDIR\msvcp140_2.dll"
  Delete "$INSTDIR\msvcp140_atomic_wait.dll"
  Delete "$INSTDIR\msvcp140_codecvt_ids.dll"
  Delete "$INSTDIR\vccorlib140.dll"
  Delete "$INSTDIR\vcruntime140.dll"
  Delete "$INSTDIR\vcruntime140_1.dll"
  Delete "$INSTDIR\vcruntime140_threads.dll"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${REGKEY}"
SectionEnd
