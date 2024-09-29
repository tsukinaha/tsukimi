; Define installer name
Name "Tsukimi Installer"

; Define output file
OutFile "tsukimi-x86_64-windows-gnu-installer.exe"

; Default installation directory (in user's personal folder)
InstallDir "$LOCALAPPDATA\Tsukimi"

; Store previous installation location
Var PreviousInstallDir

; Pages
Page directory
Page components
Page instfiles

; Uninstaller pages
UninstPage uninstConfirm
UninstPage instfiles

; Main installation section
Section "Tsukimi Main Program" SecMain
    SectionIn RO
    SetOutPath "$INSTDIR"
    
    ; Copy entire tsukimi folder
    File /r "tsukimi-x86_64-windows-gnu\*.*"
    
    ; Create start menu shortcut
    CreateDirectory "$SMPROGRAMS\Tsukimi"
    CreateShortCut "$SMPROGRAMS\Tsukimi\Tsukimi.lnk" "$INSTDIR\bin\tsukimi.exe"
    
    ; Create uninstaller
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    
    ; Write installation info to registry (current user only)
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "DisplayName" "Tsukimi"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "Publisher" "tsukinaha"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "DisplayVersion" "0.12.3"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "NoRepair" 1
SectionEnd

; Desktop shortcut section
Section "Create Desktop Shortcut" SecDesktopShortcut
    CreateShortCut "$DESKTOP\Tsukimi.lnk" "$INSTDIR\bin\tsukimi.exe"
SectionEnd

; Uninstall section
Section "Uninstall"
    ; Remove installed files
    RMDir /r "$INSTDIR"
    
    ; Remove start menu shortcut
    Delete "$SMPROGRAMS\Tsukimi\Tsukimi.lnk"
    RMDir "$SMPROGRAMS\Tsukimi"
    
    ; Remove desktop shortcut
    Delete "$DESKTOP\Tsukimi.lnk"
    
    ; Remove uninstaller
    Delete "$INSTDIR\Uninstall.exe"
    
    ; Remove registry keys (current user only)
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi"
SectionEnd

; Function to detect previous installation
Function .onInit
    ; Try to read previous installation directory
    ReadRegStr $PreviousInstallDir HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "InstallLocation"
    StrCmp $PreviousInstallDir "" done
    
    ; If a previous installation is found, set the installation directory to the previous one
    StrCpy $INSTDIR $PreviousInstallDir
    
    done:
FunctionEnd