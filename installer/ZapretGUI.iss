#define AppName "Zapret GUI"
#define AppVersion GetEnv("APP_VERSION")
#define AppPublisher "plutskiy"
#define AppExeName "zapret_gui.exe"

[Setup]
AppId={{B1A2E3D4-5F60-4789-8A1B-2C3D4E5F6071}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={localappdata}\Programs\Zapret GUI
DefaultGroupName={#AppName}
OutputDir=..\dist
OutputBaseFilename=Zapret-GUI-Setup
Compression=lzma
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64
SetupIconFile=zapret.ico
UninstallDisplayIcon={app}\zapret.ico

[Files]
Source: "zapret.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\data\*"; DestDir: "{app}\data"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "lists\*.backup,lists\*.test-backup.txt,test results\*"

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\zapret.ico"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\zapret.ico"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Запустить {#AppName}"; Flags: postinstall nowait skipifsilent
