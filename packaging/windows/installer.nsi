Name "EmojiClu"
!define APPNAME "emojiclu"
!define APPVERSION 1.0.1
!define COMPANY "Tim Harper"
!define OUTFILE emojiclu-installer-1.0.1.exe
!define EXECUTABLE "bin\emojiclu.exe"

!include "MUI2.nsh"
!include "LogicLib.nsh"

RequestExecutionLevel highest

!define MUI_ABORTWARNING
!define MUI_ICON "emojiclu\icon.ico"
!define MUI_UNICON "emojiclu\icon.ico"

Outfile ${OUTFILE}
Icon "emojiclu\icon.ico"

Var INSTALL_TYPE

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
Page custom InstallTypePageCreate InstallTypePageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Function InstallTypePageCreate
    !insertmacro MUI_HEADER_TEXT "Choose Installation Type" "Install for all users or only for current user."
    nsDialogs::Create 1018
    Pop $0
    ${NSD_CreateRadioButton} 10 30 100% 10u "Current user only"
    Pop $1
    ${NSD_CreateRadioButton} 10 50 100% 10u "All users (requires admin)"
    Pop $2
    ${NSD_SetState} $1 ${BST_CHECKED}
    nsDialogs::Show
FunctionEnd

Function InstallTypePageLeave
    ${NSD_GetState} $2 $INSTALL_TYPE
    ${If} $INSTALL_TYPE == ${BST_CHECKED}
        UserInfo::GetAccountType
        Pop $0
        ${If} $0 != "Admin"
            MessageBox MB_OK|MB_ICONSTOP "Administrator rights required for all-users installation."
            Abort
        ${EndIf}
        SetShellVarContext all
        StrCpy $INSTDIR "$PROGRAMFILES64\${APPNAME}"
        SetRegView 64
    ${Else}
        SetShellVarContext current
        StrCpy $INSTDIR "$LOCALAPPDATA\${APPNAME}"
    ${EndIf}
FunctionEnd

Section "Install EmojiClu" SecCore
    SectionIn RO
    SetOutPath $INSTDIR
    File /r "emojiclu\*.*"
    WriteUninstaller $INSTDIR\uninstall.exe
    
    ${If} $INSTALL_TYPE == ${BST_CHECKED}
        WriteRegStr HKLM "Software\${COMPANY}\${APPNAME}" "InstallDir" "$INSTDIR"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayName" "${APPNAME} ${APPVERSION}"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "UninstallString" "$INSTDIR\uninstall.exe"
    ${Else}
        WriteRegStr HKCU "Software\${COMPANY}\${APPNAME}" "InstallDir" "$INSTDIR"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayName" "${APPNAME} ${APPVERSION}"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "UninstallString" "$INSTDIR\uninstall.exe"
    ${EndIf}
SectionEnd

Section "Create Shortcuts" SecShortcuts
    SectionIn 1
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortCut "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\${EXECUTABLE}"
    CreateShortCut "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\${EXECUTABLE}"
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} "Installs the core game files."
    !insertmacro MUI_DESCRIPTION_TEXT ${SecShortcuts} "Creates Start Menu and Desktop shortcuts."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
    ReadRegStr $0 HKLM "Software\${COMPANY}\${APPNAME}" "InstallDir"
    ${If} $0 != ""
        SetShellVarContext all
        DeleteRegKey HKLM "Software\${COMPANY}\${APPNAME}"
        DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
    ${Else}
        SetShellVarContext current
        DeleteRegKey HKCU "Software\${COMPANY}\${APPNAME}"
        DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
    ${EndIf}
    
    Delete "$INSTDIR\*.*"
    RMDir /r "$INSTDIR"
    Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
    RMDir "$SMPROGRAMS\${APPNAME}"
    Delete "$DESKTOP\${APPNAME}.lnk"
SectionEnd
