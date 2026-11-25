#ifndef AppArch
  #define AppArch "x86_64"
#endif
#ifndef AppBinary
  #define AppBinary "..\..\target\release\nara.exe"
#endif
#if AppArch == "arm64"
  #define InstallerArchitecture "arm64"
#else
  #define InstallerArchitecture "x64compatible"
#endif

[Setup]
AppId=Nara.LinkReview
AppName=Nara
AppVersion=0.1.3
DefaultDirName={localappdata}\Programs\Nara
PrivilegesRequired=lowest
ArchitecturesAllowed={#InstallerArchitecture}
ArchitecturesInstallIn64BitMode={#InstallerArchitecture}
ChangesAssociations=yes
OutputBaseFilename=nara-windows-{#AppArch}-setup

[Files]
Source: "{#AppBinary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Nara"; Filename: "{app}\nara.exe"
Name: "{userprograms}\Nara Settings"; Filename: "{app}\nara.exe"; Parameters: "--settings"

[Registry]
Root: HKCU; Subkey: "Software\Classes\Nara.WebLink"; ValueType: string; ValueData: "Nara Web Link"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Nara.WebLink\DefaultIcon"; ValueType: string; ValueData: "{app}\nara.exe,0"
Root: HKCU; Subkey: "Software\Classes\Nara.WebLink\shell\open\command"; ValueType: string; ValueData: """{app}\nara.exe"" ""%1"""
Root: HKCU; Subkey: "Software\Nara\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Nara"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Nara\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Review web links before opening them in your browser."
Root: HKCU; Subkey: "Software\Nara\Capabilities\URLAssociations"; ValueType: string; ValueName: "http"; ValueData: "Nara.WebLink"
Root: HKCU; Subkey: "Software\Nara\Capabilities\URLAssociations"; ValueType: string; ValueName: "https"; ValueData: "Nara.WebLink"
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Nara"; ValueData: "Software\Nara\Capabilities"; Flags: uninsdeletevalue
