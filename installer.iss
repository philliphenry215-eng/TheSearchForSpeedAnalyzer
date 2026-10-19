; Script generated for Inno Setup Compiler
#ifndef MyAppName
  #define MyAppName "The Search For Speed Analyzer"
#endif
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
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

[Setup]
AppId={{9F82A4D1-5C82-4B65-9831-C68C65F8091E}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\SearchForSpeedAnalyzer
DisableProgramGroupPage=yes
; Per-user install requires no Windows administrator prompts
PrivilegesRequired=lowest
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
