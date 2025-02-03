!define APPNAME "mindhunt"
!define APPVERSION "1.0.0"
!define COMPANY "YourCompany"
!define OUTFILE "mindhunt-installer.exe"
!define EXECUTABLE "bin\mindhunt.exe"

; Product name for installer
!define PRODUCT_NAME "mindhunt"
!define MUI_PRODUCT ${PRODUCT_NAME}

; Include Modern UI
!include "MUI2.nsh"
!include "LogicLib.nsh"

; Request the highest execution level for the installer
RequestExecutionLevel highest

; MUI Settings
!define MUI_ABORTWARNING
!define MUI_ICON "mindhunt\icon.ico"
!define MUI_UNICON "mindhunt\icon.ico"

; Welcome page settings
!define MUI_WELCOMEPAGE_TITLE "Welcome to ${PRODUCT_NAME} Setup"
!define MUI_WELCOMEPAGE_TEXT "Setup will guide you through the installation of ${PRODUCT_NAME}.$\r$\n$\r$\nIt is recommended that you close all other applications before starting Setup. This will make it possible to update relevant system files without having to reboot your computer.$\r$\n$\r$\nClick Next to continue."

; License page settings
!define MUI_LICENSEPAGE_TEXT_TOP "Please review the license terms before installing ${PRODUCT_NAME}."

; Finish page settings
!define MUI_FINISHPAGE_TITLE "Completing the ${PRODUCT_NAME} Setup Wizard"
!define MUI_FINISHPAGE_TEXT "${PRODUCT_NAME} has been installed on your computer.$\r$\n$\r$\nClick Finish to close Setup."

Outfile ${OUTFILE}
Icon "mindhunt\icon.ico"

Var INSTALL_TYPE ; Variable to store installation type (0=current user, 1=all users)

; Installation types
InstType "Full"
InstType "Minimal"

; Modern UI pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
Page custom InstallTypePageCreate InstallTypePageLeave
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Language files - must come after pages
!insertmacro MUI_LANGUAGE "English"

; Custom page function for install type selection
Function InstallTypePageCreate
    !insertmacro MUI_HEADER_TEXT "Choose Installation Type" "Choose whether to install for all users or current user only."
    nsDialogs::Create 1018
    Pop $0
    
    ${NSD_CreateRadioButton} 10 30 100% 10u "Install for current user only"
    Pop $1
    ${NSD_CreateRadioButton} 10 50 100% 10u "Install for all users (requires administrator privileges)"
    Pop $2
    
    ${NSD_SetState} $1 ${BST_CHECKED} ; Default to current user
    nsDialogs::Show
FunctionEnd

Function InstallTypePageLeave
    ${NSD_GetState} $2 $INSTALL_TYPE
    ${If} $INSTALL_TYPE == ${BST_CHECKED}
        ; Check if we have admin rights
        UserInfo::GetAccountType
        Pop $0
        ${If} $0 != "Admin"
            MessageBox MB_OK|MB_ICONSTOP "Administrator rights required for all-users installation. Please run the installer with administrator privileges."
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

Section "mindhunt Game" SecCore
    SectionIn RO ; Main application is required
    SetOutPath $INSTDIR
    File /r "mindhunt\*.*"
    WriteUninstaller $INSTDIR\uninstall.exe
    
    ; Write registry keys for uninstaller
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

Section "Start Menu Shortcut" SecStartMenu
    SectionIn 1 ; Include in Full installation
    ${If} $INSTALL_TYPE == ${BST_CHECKED}
        SetShellVarContext all
    ${Else}
        SetShellVarContext current
    ${EndIf}
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortCut "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\${EXECUTABLE}"
SectionEnd

Section "Desktop Shortcut" SecDesktop
    SectionIn 1 ; Include in Full installation
    ${If} $INSTALL_TYPE == ${BST_CHECKED}
        SetShellVarContext all
    ${Else}
        SetShellVarContext current
    ${EndIf}
    CreateShortCut "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\${EXECUTABLE}"
SectionEnd

; Component descriptions
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} "Install ${APPNAME} core files"
    !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Create Start Menu shortcut"
    !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create Desktop shortcut"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
    ; Set correct context for uninstallation
    ReadRegStr $0 HKLM "Software\${COMPANY}\${APPNAME}" "InstallDir"
    ${If} $0 != ""
        SetShellVarContext all
    ${Else}
        SetShellVarContext current
    ${EndIf}

    Delete "$INSTDIR\*.*"
    RMDir /r "$INSTDIR"

    Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
    RMDir "$SMPROGRAMS\${APPNAME}"

    Delete "$DESKTOP\${APPNAME}.lnk"

    ; Check if we're uninstalling a per-user or all-users installation
    ${If} $0 != ""
        DeleteRegKey HKLM "Software\${COMPANY}\${APPNAME}"
        DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
    ${Else}
        DeleteRegKey HKCU "Software\${COMPANY}\${APPNAME}"
        DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
    ${EndIf}
SectionEnd
