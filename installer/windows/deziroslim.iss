#ifndef AppVersion
  #error AppVersion must be provided with /DAppVersion=...
#endif
#ifndef SourceDir
  #error SourceDir must be provided with /DSourceDir=...
#endif
#ifndef OutputDir
  #error OutputDir must be provided with /DOutputDir=...
#endif
#ifndef Arch
  #error Arch must be provided with /DArch=amd64 or /DArch=arm64
#endif

#define AppName "deziroslim"
#define AppExeName "deziroslim.exe"
#define CliExeName "dzc-slim.exe"

[Setup]
AppId={{28D7A549-A69E-4344-8070-9124DCF4198E}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=deziroslim contributors
AppPublisherURL=https://github.com/qiyuanhuakai/deziroslim
AppSupportURL=https://github.com/qiyuanhuakai/deziroslim/issues
AppUpdatesURL=https://github.com/qiyuanhuakai/deziroslim/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir={#OutputDir}
OutputBaseFilename={#AppName}-{#AppVersion}-windows-{#Arch}-setup
SetupIconFile=..\..\assets\icons\deziroslim.ico
UninstallDisplayIcon={app}\{#AppExeName}
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog commandline
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
VersionInfoVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoProductVersion={#AppVersion}

#if Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\{#CliExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#AppExeName}"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExeName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#CliExeName}"; ValueType: string; ValueName: ""; ValueData: "{app}\{#CliExeName}"; Flags: uninsdeletekey

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
