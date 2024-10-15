Unicode True

; Tsukimi Installer Script
; 基本设置
!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "Tsukimi"
OutFile "tsukimi-x86_64-windows-gnu-installer.exe"
InstallDir "$LOCALAPPDATA\Tsukimi"
RequestExecutionLevel user

; 获取安装包元数据
!define /file VERSION "version.txt"
VIProductVersion "${VERSION}"
VIAddVersionKey "ProductName" "Tsukimi"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "© 2024 tsukinaha"
VIAddVersionKey "FileDescription" "Tsukimi Installer"

; 定义宏以检查并卸载之前的安装
!macro CheckAndUninstallPrevious
    ; 初始化一个变量来存储旧安装路径
    StrCpy $R1 ""

    ; 尝试从注册表读取旧安装路径
    ReadRegStr $R1 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "InstallLocation"
    ${If} $R1 == ""
        ; 如果注册表中没有，尝试使用默认路径
        StrCpy $R1 "$LOCALAPPDATA\Tsukimi"
    ${EndIf}

    ; 检查卸载程序是否存在
    IfFileExists "$R1\uninstall.exe" 0 continue_install
        MessageBox MB_OKCANCEL|MB_ICONINFORMATION "$(PreviousInstallDetected)" IDOK uninstall
        Abort ; 如果用户选择取消，则中止安装
    uninstall:
        ; 执行卸载操作
        ExecWait '"$R1\uninstall.exe" /S _?=$R1' $0
        ${If} $0 != 0
            MessageBox MB_OK|MB_ICONSTOP "$(UninstallFailed)"
            Abort
        ${EndIf}
        ; 将安装目录设置为旧版本的目录
        StrCpy $INSTDIR $R1
    continue_install:
!macroend

; 界面设置
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; 语言文件
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

; 自定义字符串
LangString PreviousInstallDetected ${LANG_ENGLISH} "An old version of Tsukimi has been detected. Click 'OK' to upgrade, or 'Cancel' to exit."
LangString PreviousInstallDetected ${LANG_SIMPCHINESE} "检测到旧版本的Tsukimi。点击'确定'进行升级安装，或点击'取消'退出。"

LangString UninstallFailed ${LANG_ENGLISH} "Uninstallation of the old version failed. Please uninstall manually and try again."
LangString UninstallFailed ${LANG_SIMPCHINESE} "卸载旧版本失败。请手动卸载后再试。"

LangString SecTsukimiName ${LANG_ENGLISH} "Tsukimi"
LangString SecTsukimiName ${LANG_SIMPCHINESE} "Tsukimi"

LangString SecTsukimiDesc ${LANG_ENGLISH} "Install Tsukimi application."
LangString SecTsukimiDesc ${LANG_SIMPCHINESE} "安装Tsukimi应用程序。"

LangString SecDesktopShortcutName ${LANG_ENGLISH} "Desktop Shortcut"
LangString SecDesktopShortcutName ${LANG_SIMPCHINESE} "桌面快捷方式"

LangString SecDesktopShortcutDesc ${LANG_ENGLISH} "Create a shortcut for Tsukimi on the desktop."
LangString SecDesktopShortcutDesc ${LANG_SIMPCHINESE} "在桌面上创建Tsukimi的快捷方式。"

LangString UninstallQuestion ${LANG_ENGLISH} "Are you sure you want to completely remove Tsukimi and all of its components?"
LangString UninstallQuestion ${LANG_SIMPCHINESE} "您确定要完全移除 Tsukimi 及其所有组件吗？"

; 安装区段
Section "$(SecTsukimiName)" SecTsukimi
    SectionIn RO
    SetOutPath "$INSTDIR"
    
    ; 复制所有文件
    File /r "tsukimi-x86_64-windows-gnu\*.*"
    
    ; 创建卸载程序
    WriteUninstaller "$INSTDIR\uninstall.exe"
    
    ; 创建开始菜单快捷方式
    CreateDirectory "$SMPROGRAMS\Tsukimi"
    CreateShortcut "$SMPROGRAMS\Tsukimi\Tsukimi.lnk" "$INSTDIR\bin\tsukimi.exe"
    CreateShortcut "$SMPROGRAMS\Tsukimi\Uninstall Tsukimi.lnk" "$INSTDIR\uninstall.exe"

    ; 写入卸载信息到注册表
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "DisplayName" "Tsukimi"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "DisplayIcon" "$INSTDIR\bin\tsukimi.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "Publisher" "tsukinaha"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "DisplayVersion" "${VERSION}"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "NoRepair" 1

    ; 获取安装大小
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi" "EstimatedSize" "$0"
SectionEnd

Section "$(SecDesktopShortcutName)" SecDesktopShortcut
    SectionIn 1
    CreateShortcut "$DESKTOP\Tsukimi.lnk" "$INSTDIR\bin\tsukimi.exe"
SectionEnd

; 描述
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecTsukimi} $(SecTsukimiDesc)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktopShortcut} $(SecDesktopShortcutDesc)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; 卸载区段
Section "Uninstall"
    ; 删除安装的文件
    RMDir /r "$INSTDIR"
    
    ; 删除开始菜单快捷方式
    Delete "$SMPROGRAMS\Tsukimi\Tsukimi.lnk"
    Delete "$SMPROGRAMS\Tsukimi\Uninstall Tsukimi.lnk"
    RMDir "$SMPROGRAMS\Tsukimi"

    ; 删除桌面快捷方式
    Delete "$DESKTOP\Tsukimi.lnk"

    ; 删除注册表项
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Tsukimi"
SectionEnd

; 安装程序初始化函数
Function .onInit
    ; 设置默认语言
    !insertmacro MUI_LANGDLL_DISPLAY

    ; 设置默认安装目录
    StrCpy $INSTDIR "$LOCALAPPDATA\Tsukimi"

    ; 检查并卸载之前的安装
    !insertmacro CheckAndUninstallPrevious
FunctionEnd

; 卸载程序初始化函数
Function un.onInit
    !insertmacro MUI_UNGETLANGUAGE
    MessageBox MB_YESNO|MB_ICONQUESTION "$(UninstallQuestion)" IDYES +2
    Abort
FunctionEnd
