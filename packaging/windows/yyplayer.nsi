# -*- coding: utf-8 -*-
Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "FileFunc.nsh"
!include "WordFunc.nsh"
!include "WinMessages.nsh"
!include /CHARSET=UTF8 "${APP_FILE_RULES}"

!ifndef APP_VERSION
  !error "APP_VERSION must be supplied by scripts/Package-Windows.ps1"
!endif
!ifndef APP_COMMIT
  !define APP_COMMIT "local"
!endif
!define APP_STAGE "$%YYPLAYER_PACKAGE_STAGE%"
!define OUTPUT_DIR "$%YYPLAYER_PACKAGE_OUTPUT%"
!ifndef APP_ID
  !define APP_ID "{A858C6E1-6B63-4A1B-90EC-275C31F61D38}"
!endif
!ifdef INSTALL_TEST
  ; Compile-time isolated QA only; production always uses the machine registry.
  !define APP_REGISTRY HKCU
  !define APP_CONTEXT current
  !define SHORTCUT_DIRECTORY "YYPlayer-test-${APP_ID}"
!else
  !define APP_REGISTRY HKLM
  !define APP_CONTEXT all
  !define SHORTCUT_DIRECTORY "YYPlayer"
!endif
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}"
Var InstalledVersion
Var InstalledDirectory
Var InstallAction
Var WelcomeTitle
Var WelcomeText
Var InstallButton
Var InstallFailed
Var FilePath
Var ParentPath
!ifndef APP_ICON
  !error "APP_ICON must point to assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico"
!endif
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"

Name "YYPlayer ${APP_VERSION}"
OutFile "${OUTPUT_DIR}\YYPlayer-${APP_VERSION}-${APP_COMMIT}-windows-x64-setup.exe"
InstallDir "$PROGRAMFILES64\YYPlayer"
!ifdef INSTALL_TEST
  RequestExecutionLevel user
!else
  RequestExecutionLevel admin
!endif
ManifestDPIAware true
ShowInstDetails show
ShowUnInstDetails show
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "YYPlayer"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileDescription" "YYPlayer Windows x64 Setup"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-only"

!define MUI_ABORTWARNING
!define MUI_LICENSEPAGE_CHECKBOX
!define MUI_WELCOMEPAGE_TITLE "$WelcomeTitle"
!define MUI_WELCOMEPAGE_TEXT "$WelcomeText"
!insertmacro MUI_PAGE_WELCOME
!define MUI_PAGE_CUSTOMFUNCTION_SHOW ShowInstallButton
!insertmacro MUI_PAGE_LICENSE "${APP_STAGE}\LICENSE"
!define MUI_PAGE_CUSTOMFUNCTION_PRE DirectoryPre
!define MUI_PAGE_CUSTOMFUNCTION_SHOW ShowInstallButton
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\yyplayer.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch YYPlayer"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

LangString UpgradeTitle ${LANG_SIMPCHINESE} "升级 YYPlayer 至 ${APP_VERSION}"
LangString UpgradeTitle ${LANG_ENGLISH} "Upgrade YYPlayer to ${APP_VERSION}"
LangString UpgradeText ${LANG_SIMPCHINESE} "检测到已安装的 YYPlayer $InstalledVersion。$\r$\n$\r$\n将升级原目录中的程序：$\r$\n$INSTDIR$\r$\n$\r$\n设置、音乐库和歌单将保留。请先关闭 YYPlayer。"
LangString UpgradeText ${LANG_ENGLISH} "YYPlayer $InstalledVersion is already installed.$\r$\n$\r$\nUpgrade location:$\r$\n$INSTDIR$\r$\n$\r$\nSettings, libraries and playlists are preserved. Close YYPlayer before continuing."
LangString UpgradeButton ${LANG_SIMPCHINESE} "升级(&U)"
LangString UpgradeButton ${LANG_ENGLISH} "&Upgrade"
LangString RepairTitle ${LANG_SIMPCHINESE} "重新安装 YYPlayer ${APP_VERSION}"
LangString RepairTitle ${LANG_ENGLISH} "Reinstall YYPlayer ${APP_VERSION}"
LangString RepairButton ${LANG_SIMPCHINESE} "重新安装(&R)"
LangString RepairButton ${LANG_ENGLISH} "&Reinstall"
LangString RepairText ${LANG_SIMPCHINESE} "YYPlayer ${APP_VERSION} 已安装。$\r$\n$\r$\n将重新安装原目录中的程序：$\r$\n$INSTDIR$\r$\n$\r$\n设置、音乐库和歌单将保留。请先关闭 YYPlayer。"
LangString RepairText ${LANG_ENGLISH} "YYPlayer ${APP_VERSION} is already installed.$\r$\n$\r$\nReinstall location:$\r$\n$INSTDIR$\r$\n$\r$\nSettings, libraries and playlists are preserved. Close YYPlayer before continuing."
LangString DowngradeError ${LANG_SIMPCHINESE} "已安装更新的 YYPlayer $InstalledVersion，不能用 ${APP_VERSION} 覆盖。"
LangString DowngradeError ${LANG_ENGLISH} "YYPlayer $InstalledVersion is newer than ${APP_VERSION}. Downgrade is blocked."
LangString CopyError ${LANG_SIMPCHINESE} "无法更新程序文件，已尝试恢复原文件。请关闭 YYPlayer，检查磁盘空间与目录权限后重试。"
LangString CopyError ${LANG_ENGLISH} "Could not update program files. Original files were restored where possible. Close YYPlayer and check disk space and directory permissions before retrying."

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "This package requires 64-bit Windows."
    Abort
  ${EndIf}
  SetRegView 64
  SetShellVarContext ${APP_CONTEXT}
  StrCpy $InstallAction "install"
  StrCpy $WelcomeTitle "$(MUI_TEXT_WELCOME_INFO_TITLE)"
  StrCpy $WelcomeText "$(MUI_TEXT_WELCOME_INFO_TEXT)"
  StrCpy $InstallButton "$(^InstallBtn)"
  ReadRegStr $InstalledDirectory ${APP_REGISTRY} "${UNINSTALL_KEY}" "InstallLocation"
  ReadRegStr $InstalledVersion ${APP_REGISTRY} "${UNINSTALL_KEY}" "DisplayVersion"
  ${If} $InstalledDirectory != ""
    ; Keep the original install even when /D= or the default directory differs.
    StrCpy $INSTDIR $InstalledDirectory
    StrCpy $InstallAction "upgrade"
    StrCpy $WelcomeTitle "$(UpgradeTitle)"
    StrCpy $WelcomeText "$(UpgradeText)"
    StrCpy $InstallButton "$(UpgradeButton)"
    ${If} $InstalledVersion != ""
      ${VersionCompare} $InstalledVersion "${APP_VERSION}" $0
      ${If} $0 == 1
        SetErrorLevel 3
        MessageBox MB_ICONSTOP "$(DowngradeError)" /SD IDOK
        Abort
      ${ElseIf} $0 == 0
        StrCpy $InstallAction "repair"
        StrCpy $WelcomeTitle "$(RepairTitle)"
        StrCpy $WelcomeText "$(RepairText)"
        StrCpy $InstallButton "$(RepairButton)"
      ${EndIf}
    ${EndIf}
  ${EndIf}
FunctionEnd

Function DirectoryPre
  ${If} $InstallAction != "install"
    Abort
  ${EndIf}
FunctionEnd

Function ShowInstallButton
  ${If} $InstallAction != "install"
    GetDlgItem $0 $HWNDPARENT 1
    SendMessage $0 ${WM_SETTEXT} 0 "STR:$InstallButton"
  ${EndIf}
FunctionEnd

; Files are extracted before any old file is touched. Each replaced file has an
; independent backup, so a locked DLL / copy failure can roll back this run.
Function InstallFile
  Pop $FilePath
  ${If} $InstallFailed == 1
    Return
  ${EndIf}
  ClearErrors
  ${GetParent} "$INSTDIR\$FilePath" $ParentPath
  CreateDirectory "$ParentPath"
  ${GetParent} "$PLUGINSDIR\backup\$FilePath" $ParentPath
  CreateDirectory "$ParentPath"
  IfFileExists "$INSTDIR\$FilePath" 0 no_backup
  CopyFiles /SILENT "$INSTDIR\$FilePath" "$PLUGINSDIR\backup\$FilePath"
  IfErrors copy_failed
no_backup:
  ${GetParent} "$PLUGINSDIR\attempted\$FilePath" $ParentPath
  CreateDirectory "$ParentPath"
  FileOpen $0 "$PLUGINSDIR\attempted\$FilePath" w
  FileClose $0
  IfErrors copy_failed
  CopyFiles /SILENT "$PLUGINSDIR\payload\$FilePath" "$INSTDIR\$FilePath"
  IfErrors copy_failed
  Return
copy_failed:
  StrCpy $InstallFailed 1
FunctionEnd

Function RollbackFile
  Pop $FilePath
  IfFileExists "$PLUGINSDIR\attempted\$FilePath" 0 rollback_done
  IfFileExists "$PLUGINSDIR\backup\$FilePath" 0 remove_new
  ClearErrors
  CopyFiles /SILENT "$PLUGINSDIR\backup\$FilePath" "$INSTDIR\$FilePath"
  IfErrors 0 rollback_done
  DetailPrint "Could not restore $FilePath (the file may still be in use)."
  Return
remove_new:
  Delete "$INSTDIR\$FilePath"
rollback_done:
FunctionEnd

Section "YYPlayer" MainSection
  SectionIn RO
  SetShellVarContext ${APP_CONTEXT}
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\payload"
  File /r "${APP_STAGE}\*"
  WriteUninstaller "$PLUGINSDIR\payload\uninstall.exe"
  StrCpy $InstallFailed 0
  !insertmacro YYPlayerInstallFiles
  ${If} $InstallFailed == 1
    !insertmacro YYPlayerRollbackFiles
    SetErrorLevel 2
    MessageBox MB_ICONSTOP "$(CopyError)" /SD IDOK
    Abort
  ${EndIf}
  SetOutPath "$INSTDIR"
  CreateDirectory "$SMPROGRAMS\${SHORTCUT_DIRECTORY}"
  CreateShortcut "$SMPROGRAMS\${SHORTCUT_DIRECTORY}\YYPlayer.lnk" "$INSTDIR\yyplayer.exe" "" "$INSTDIR\yyplayer.exe" 0
  CreateShortcut "$SMPROGRAMS\${SHORTCUT_DIRECTORY}\Uninstall YYPlayer.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\yyplayer.exe" 0
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "DisplayName" "YYPlayer"
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\yyplayer.exe,0"
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
  WriteRegStr ${APP_REGISTRY} "${UNINSTALL_KEY}" "LastInstallAction" "$InstallAction"
  WriteRegDWORD ${APP_REGISTRY} "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD ${APP_REGISTRY} "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Function un.onInit
  SetRegView 64
  SetShellVarContext ${APP_CONTEXT}
FunctionEnd

Section "Uninstall"
  SetShellVarContext ${APP_CONTEXT}
  Delete "$SMPROGRAMS\${SHORTCUT_DIRECTORY}\YYPlayer.lnk"
  Delete "$SMPROGRAMS\${SHORTCUT_DIRECTORY}\Uninstall YYPlayer.lnk"
  RMDir "$SMPROGRAMS\${SHORTCUT_DIRECTORY}"
  DeleteRegKey ${APP_REGISTRY} "${UNINSTALL_KEY}"
  !insertmacro YYPlayerUninstallFiles
  RMDir "$INSTDIR"
SectionEnd
