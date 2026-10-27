; Script generated for Inno Setup Compiler
#ifndef MyAppName
  #define MyAppName "The Search For Speed Analyzer"
#endif
#ifndef MyAppVersion
  #define MyAppVersion "0.3.9"
#endif
#ifndef MyAppPublisher
  #define MyAppPublisher "The Henry Boys"
#endif
#ifndef MyAppURL
  #define MyAppURL "https://github.com/philliphenry215-eng/TheSearchForSpeedAnalyzer"
#endif
#ifndef MyAppExeName
  #define MyAppExeName "SearchForSpeedAnalyzer.exe"
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "SearchForSpeedAnalyzer-Setup"
#endif
; Purely numeric version (e.g. 0.3.9) for the Windows file "Details" tab, which rejects text like "v0.3.9-beta.1".
#ifndef MyAppNumericVersion
  #define MyAppNumericVersion "0.3.9"
#endif

; Beta and stable are separate apps in Windows: own AppId (uninstall entry) and own folder,
; so installing/uninstalling one never touches the other. Pass /DBetaBuild for beta builds.
; The stable AppId must never change, or existing installs won't upgrade in place.
#ifdef BetaBuild
  #define MyAppId "{{D69B32D8-DB42-4871-9607-5818DF1E63A7}"
  #define MyAppDirName "SearchForSpeedAnalyzer-Beta"
#else
  #define MyAppId "{{9F82A4D1-5C82-4B65-9831-C68C65F8091E}"
  #define MyAppDirName "SearchForSpeedAnalyzer"
#endif

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
ArchitecturesInstallIn64BitMode=x64compatible
; Default to user's Documents folder: familiar, accessible, zero admin rights needed, no Error 5
DefaultDirName={userdocs}\{#MyAppDirName}
DisableProgramGroupPage=yes
; Per-user install into Documents requires no Windows administrator prompts or elevation.
PrivilegesRequired=lowest
; No privilege dialog is shown. The in-app updater passes /ALLUSERS only for legacy (v0.3.6)
; installs in C:\Program Files, so they keep updating in place under their original uninstall entry.
PrivilegesRequiredOverridesAllowed=commandline
; Close a still-running copy of the app (Windows Restart Manager) before replacing its files.
; The app is relaunched by the [Run] section instead of Restart Manager.
CloseApplications=force
RestartApplications=no
; File "Details" metadata shown in Explorer and read by antivirus / SmartScreen reputation checks.
VersionInfoVersion={#MyAppNumericVersion}
VersionInfoProductVersion={#MyAppNumericVersion}
VersionInfoTextVersion={#MyAppVersion}
VersionInfoProductTextVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoProductName={#MyAppName}
VersionInfoDescription={#MyAppName} Setup
VersionInfoCopyright=Copyright (C) 2026 {#MyAppPublisher}
OutputDir=.
OutputBaseFilename={#OutputBaseFilename}
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\icon.ico"; DestDir: "{app}\assets"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\icon.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\icon.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
; Used by the in-app updater, which runs this installer with /SILENT /RELAUNCH.
; runasoriginaluser: even when an admin update was approved via UAC, the app restarts with normal rights.
Filename: "{app}\{#MyAppExeName}"; Flags: nowait runasoriginaluser; Check: ShouldRelaunch

[Code]
function ShouldRelaunch: Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/RELAUNCH') = 0 then
      Result := True;
end;
