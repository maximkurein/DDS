; DDS Studio — установщик Inno Setup 6. Собирает build_windows.ps1.
; UTF-8 с BOM. Версия: ISCC /DAppVersion=0.3.1 /Orelease\v0.3.1 /Fимя

#ifndef AppVersion
  #define AppVersion "0.3.1"
#endif

#define AppName "DDS Studio"
#define AppPublisher "Kurein M. N."
#define AppExe "DDS Studio.exe"

[Setup]
AppId={{B6E2A148-4C7D-4F1A-9E55-0A8D3C71F2B4}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=release
OutputBaseFilename=DDS-Studio-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
SetupIconFile=target\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
CloseApplications=force

[Languages]
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "target\win-bundle\DDS Studio\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent