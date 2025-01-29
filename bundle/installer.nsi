!define APPNAME "gwatson"
!define APPVERSION "1.0.0"
!define COMPANY "YourCompany"
!define INSTALLDIR "$PROGRAMFILES64\${APPNAME}"
!define OUTFILE "gwatson-installer.exe"
!define EXECUTABLE "bin\gwatson.exe"

Outfile ${OUTFILE}
InstallDir ${INSTALLDIR}
RequestExecutionLevel user
Icon "gwatson\icon.ico"


Page components
Page directory
Page instfiles

Section "GWatson Game" SEC00
    SetOutPath $INSTDIR
    File /r "gwatson\*.*"

    ; Create Start Menu shortcut
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortCut "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\${EXECUTABLE}"

    ; Optionally create a desktop shortcut
    SectionIn RO
    CreateShortCut "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\${EXECUTABLE}"

    WriteUninstaller $INSTDIR\uninstall.exe
    WriteRegStr HKLM "Software\${COMPANY}\${APPNAME}" "InstallDir" "$INSTDIR"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayName" "${APPNAME} ${APPVERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall" SEC02
    Delete "$INSTDIR\*.*"
    RMDir /r "$INSTDIR"

    Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
    RMDir "$SMPROGRAMS\${APPNAME}"

    Delete "$DESKTOP\${APPNAME}.lnk"

    DeleteRegKey HKLM "Software\${COMPANY}\${APPNAME}"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
SectionEnd
