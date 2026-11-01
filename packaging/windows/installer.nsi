Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
!include MUI2.nsh
!include LogicLib.nsh
!include WordFunc.nsh
!include FileFunc.nsh
!ifndef ARP
!define ARP "Software\Microsoft\Windows\CurrentVersion\Uninstall\ITGLA"
!endif
!ifndef SHORTCUT_NAME
!define SHORTCUT_NAME "ITGLA"
!endif
Var BackupDir
Name "ITGLA"
OutFile "${OUTPUT}"
!ifndef DEFAULT_INSTALL_DIR
!define DEFAULT_INSTALL_DIR "$LOCALAPPDATA\ITGLA"
!endif
InstallDir "${DEFAULT_INSTALL_DIR}"
InstallDirRegKey HKCU "${ARP}" "InstallLocation"
RequestExecutionLevel user
SetFont "Segoe UI" 9
BrandingText "IT in a glance"
VIProductVersion "${VERSION_QUAD}"
VIAddVersionKey /LANG=1033 "ProductName" "ITGLA"
VIAddVersionKey /LANG=1033 "CompanyName" "ITGLA"
VIAddVersionKey /LANG=1033 "FileDescription" "ITGLA Setup"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Copyright ITGLA"
!define MUI_ICON "${PAYLOAD}/itgla.ico"
!define MUI_UNICON "${PAYLOAD}/itgla.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "${ART}/header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "${ART}/sidebar.bmp"
!define MUI_HEADERIMAGE_BITMAP_STRETCH FitControl
!define MUI_WELCOMEFINISHPAGE_BITMAP_STRETCH FitControl
!define MUI_BGCOLOR "F4F6F1"
!define MUI_TEXTCOLOR "203C30"
!define MUI_INSTFILESPAGE_COLORS "203C30 F4F6F1"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "IT in a glance"
!define MUI_WELCOMEPAGE_TEXT "Keep servers, services, domains and their connections in one local workspace.$\r$\n$\r$\nSetup installs ITGLA for your Windows account. Existing workspaces and backups are kept.$\r$\n$\r$\nSelect Next to choose the installation folder."
!define MUI_FINISHPAGE_TITLE "ITGLA is ready"
!define MUI_FINISHPAGE_TEXT "Your local IT workspace is ready.$\r$\n$\r$\nOpen ITGLA to review your assets or explore the demo."
!define MUI_FINISHPAGE_RUN "$INSTDIR\itgla.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open ITGLA"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!define MUI_UNCONFIRMPAGE_TEXT_TOP "Remove ITGLA program files? Your workspaces, settings and backups will be kept."
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

!macro CheckWritable PREFIX
Function ${PREFIX}CheckWritable
  IfFileExists "$INSTDIR\itgla.exe" 0 done
  ; Write access fails when Windows has mapped this executable into a running process.
  System::Call 'kernel32::CreateFileW(w "$INSTDIR\itgla.exe", i 0xC0000000, i 0, p 0, i 3, i 0, p 0) p.r0'
  ${If} $0 == -1
    IfSilent +2
    MessageBox MB_OK|MB_ICONEXCLAMATION "Close ITGLA before continuing. Also check that you can write to the installation folder. Your data has not been changed."
    SetErrorLevel 2
    Abort
  ${EndIf}
  System::Call 'kernel32::CloseHandle(p r0)'
  done:
FunctionEnd
!macroend
!insertmacro CheckWritable ""
!insertmacro CheckWritable "un."

Function .onInit
  SetShellVarContext current
  System::Call 'kernel32::CreateMutexW(p 0, i 0, w "Local\ITGLA.Setup") p.r0 ?e'
  Pop $1
  ${If} $1 == 183
    SetErrorLevel 3
    Abort
  ${EndIf}
  ReadRegStr $0 HKCU "${ARP}" "DisplayVersion"
  ${If} $0 != ""
    ${VersionCompare} "$0" "${VERSION}" $1
    ${If} $1 == 1
      IfSilent +2
      MessageBox MB_OK|MB_ICONEXCLAMATION "A newer ITGLA version ($0) is installed. Setup will not downgrade it because older versions may not read your workspace safely."
      SetErrorLevel 4
      Abort
    ${EndIf}
  ${EndIf}
FunctionEnd

Function un.onInit
  SetShellVarContext current
  System::Call 'kernel32::CreateMutexW(p 0, i 0, w "Local\ITGLA.Setup") p.r0 ?e'
  Pop $1
  ${If} $1 == 183
    SetErrorLevel 3
    Abort
  ${EndIf}
  Call un.CheckWritable
FunctionEnd

!macro BackupFile NAME
  IfFileExists "$INSTDIR\${NAME}" 0 +4
    ClearErrors
    CopyFiles /SILENT "$INSTDIR\${NAME}" "$BackupDir\${NAME}"
    IfErrors backup_failed
!macroend
!macro RestoreFile NAME
  ClearErrors
  ${If} ${FileExists} "$BackupDir\${NAME}"
    CopyFiles /SILENT "$BackupDir\${NAME}" "$INSTDIR\${NAME}"
  ${Else}
    Delete "$INSTDIR\${NAME}"
  ${EndIf}
  IfErrors 0 +2
    StrCpy $9 1
!macroend

!macro CleanBackup
  Delete "$BackupDir\itgla.exe"
  Delete "$BackupDir\itgla.ico"
  Delete "$BackupDir\README.md"
  Delete "$BackupDir\CHANGELOG.md"
  Delete "$BackupDir\WINDOWS-README.md"
  Delete "$BackupDir\Uninstall.exe"
  RMDir "$BackupDir"
!macroend

Section "ITGLA (required)" Main
  SectionIn RO
  Call CheckWritable
  InitPluginsDir
  ClearErrors
  SetOutPath "$PLUGINSDIR\payload"
  File "${PAYLOAD}/itgla.exe"
  File "${PAYLOAD}/itgla.ico"
  File "${PAYLOAD}/README.md"
  File "${PAYLOAD}/CHANGELOG.md"
  File "${PAYLOAD}/WINDOWS-README.md"
  IfErrors backup_failed
  StrCpy $BackupDir "$INSTDIR\.itgla-setup-backup"
  ${If} ${FileExists} "$BackupDir"
    SetErrorLevel 6
    Abort "A previous recovery directory exists. Review .itgla-setup-backup before installing again."
  ${EndIf}
  ClearErrors
  CreateDirectory "$BackupDir"
  IfErrors backup_failed
  !insertmacro BackupFile "itgla.exe"
  !insertmacro BackupFile "itgla.ico"
  !insertmacro BackupFile "README.md"
  !insertmacro BackupFile "CHANGELOG.md"
  !insertmacro BackupFile "WINDOWS-README.md"
  !insertmacro BackupFile "Uninstall.exe"
  ClearErrors
  SetOutPath "$INSTDIR"
  CopyFiles /SILENT "$PLUGINSDIR\payload\*.*" "$INSTDIR"
  IfErrors install_failed
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  IfErrors install_failed
  CreateDirectory "$SMPROGRAMS\${SHORTCUT_NAME}"
  CreateShortcut "$SMPROGRAMS\${SHORTCUT_NAME}\ITGLA.lnk" "$INSTDIR\itgla.exe" "" "$INSTDIR\itgla.ico" 0
  CreateShortcut "$SMPROGRAMS\${SHORTCUT_NAME}\Uninstall ITGLA.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${ARP}" "DisplayName" "ITGLA"
  WriteRegStr HKCU "${ARP}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${ARP}" "Publisher" "ITGLA"
  WriteRegStr HKCU "${ARP}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${ARP}" "DisplayIcon" "$INSTDIR\itgla.exe"
  WriteRegStr HKCU "${ARP}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "${ARP}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegStr HKCU "${ARP}" "URLInfoAbout" "https://itgla.com"
  WriteRegStr HKCU "${ARP}" "HelpLink" "https://itgla.com/"
  WriteRegDWORD HKCU "${ARP}" "NoModify" 1
  WriteRegDWORD HKCU "${ARP}" "NoRepair" 1
  !insertmacro CleanBackup
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${ARP}" "EstimatedSize" $0
  Goto done
  install_failed:
    StrCpy $9 0
    !insertmacro RestoreFile "itgla.exe"
    !insertmacro RestoreFile "itgla.ico"
    !insertmacro RestoreFile "README.md"
    !insertmacro RestoreFile "CHANGELOG.md"
    !insertmacro RestoreFile "WINDOWS-README.md"
    !insertmacro RestoreFile "Uninstall.exe"
    ${If} $9 == 0
      !insertmacro CleanBackup
      IfSilent +2
      MessageBox MB_OK|MB_ICONSTOP "Installation failed. Previous program files were restored. Your workspace data was not changed."
    ${Else}
      IfSilent +2
      MessageBox MB_OK|MB_ICONSTOP "Installation failed and some program files could not be restored. Recovery copies remain in $BackupDir. Your workspace data was not changed."
    ${EndIf}
    SetErrorLevel 5
    Abort
  backup_failed:
    SetErrorLevel 6
    Abort "Cannot back up the existing installation. No program files were replaced."
  done:
SectionEnd

Section /o "Desktop shortcut" DesktopShortcut
  CreateShortcut "$DESKTOP\${SHORTCUT_NAME}.lnk" "$INSTDIR\itgla.exe" "" "$INSTDIR\itgla.ico" 0
SectionEnd

Section "Uninstall"
  Call un.CheckWritable
  ClearErrors
  Delete "$INSTDIR\itgla.exe"
  IfErrors 0 +3
    SetErrorLevel 2
    Abort "Cannot remove ITGLA. Close the application and try again."
  Delete "$DESKTOP\${SHORTCUT_NAME}.lnk"
  Delete "$SMPROGRAMS\${SHORTCUT_NAME}\ITGLA.lnk"
  Delete "$SMPROGRAMS\${SHORTCUT_NAME}\Uninstall ITGLA.lnk"
  RMDir "$SMPROGRAMS\${SHORTCUT_NAME}"
  Delete "$INSTDIR\itgla.ico"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\CHANGELOG.md"
  Delete "$INSTDIR\WINDOWS-README.md"
  Delete "$INSTDIR\Uninstall.exe"
  ; Non-recursive only: never delete unknown files, workspaces or backups.
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${ARP}"
SectionEnd
