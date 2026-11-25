[Setup]
AppId=Nara.LinkReview
AppName=Nara
AppVersion=0.1.0
DefaultDirName={localappdata}\Programs\Nara
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes
OutputBaseFilename=nara-windows-x64-setup

[Files]
Source: "..\..\target\release\nara.exe"; DestDir: "{app}"; Flags: ignoreversion

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
