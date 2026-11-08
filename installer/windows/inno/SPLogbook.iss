; ==============================================================================
; SPLogbook Windows User Setup Script (Inno Setup 7.1.1)
; Canonically defined for STAGE-M3-D-WINDOWS-USER-SETUP-INTEGRATION-GATE
; Conforms to OWNER-M3-D-TOOLCHAIN-01 (APPROVED 2026-11-08)
; ==============================================================================

#ifndef AppVersion
  #define AppVersion "1.3.0-alpha.1"
#endif

#ifndef GitCommit
  #define GitCommit "unknown"
#endif

#ifndef BuildChannel
  #define BuildChannel "alpha"
#endif

#ifndef DistributionStatus
  #define DistributionStatus "development"
#endif

#ifndef StagingDir
  #define StagingDir "staging"
#endif

#ifndef OutputDir
  #define OutputDir "dist"
#endif

#ifndef OutputBaseFilename
  #define OutputBaseFilename "SPLogbook-Windows-x64-Setup-1.3.0-alpha.1"
#endif

[Setup]
AppName=SPLogbook
AppId={{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}}
AppVersion={#AppVersion}
AppVerName=SPLogbook {#AppVersion}
AppPublisher=SPLogbook Project
DefaultDirName={localappdata}\Programs\SPLogbook
DefaultGroupName=SPLogbook

SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

PrivilegesRequired=lowest

Compression=lzma2
SolidCompression=yes

Uninstallable=yes
RestartIfNeededByRun=no
AlwaysRestart=no

DisableProgramGroupPage=yes
UsePreviousAppDir=yes

OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseFilename}

AppMutex=SPLogbook_Instance_Mutex_{{C8E28541-7B21-4A9B-8F5C-4EF298F613C1},SPLogbook
CloseApplications=yes
CloseApplicationsFilter=SPLogbook.exe

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Binaries (strictly allowlisted)
Source: "{#StagingDir}\SPLogbook.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StagingDir}\splogbook-updater.exe"; DestDir: "{app}"; Flags: ignoreversion

; Assets (strictly allowlisted)
Source: "{#StagingDir}\assets\icon.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "{#StagingDir}\assets\icon.png"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "{#StagingDir}\assets\icon.rgba"; DestDir: "{app}\assets"; Flags: ignoreversion

; Locales (strictly allowlisted)
Source: "{#StagingDir}\locales\*"; DestDir: "{app}\locales"; Flags: ignoreversion
Source: "{#StagingDir}\locales\manual\*"; DestDir: "{app}\locales\manual"; Flags: ignoreversion skipifsourcedoesntexist

; Seed databases (FirstInstallSeed policy: never overwrite, never uninstall)
Source: "{#StagingDir}\databases\callbook.db"; DestDir: "{app}\databases"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "{#StagingDir}\databases\serviceLOG.db"; DestDir: "{app}\databases"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "{#StagingDir}\databases\clubs.csv"; DestDir: "{app}\databases"; Flags: onlyifdoesntexist uninsneveruninstall

[Icons]
Name: "{userprograms}\SPLogbook"; Filename: "{app}\SPLogbook.exe"; IconFilename: "{app}\assets\icon.ico"
Name: "{userdesktop}\SPLogbook"; Filename: "{app}\SPLogbook.exe"; IconFilename: "{app}\assets\icon.ico"; Tasks: desktopicon

[UninstallDelete]
; Precise file deletions — strictly NO wildcards and NO DelTree
Type: files; Name: "{app}\.splogbook-inno-marker"
; Clean up directories only if they are empty (preserves user data, SQLite WAL/SHM, unknown files)
Type: dirifempty; Name: "{app}\locales\manual"
Type: dirifempty; Name: "{app}\locales"
Type: dirifempty; Name: "{app}\assets"
Type: dirifempty; Name: "{app}\databases"
Type: dirifempty; Name: "{app}"

[Code]
function CompareSemVer(const V1, V2: String): Integer;
var
  Clean1, Clean2: String;
  P1, P2: Integer;
begin
  Clean1 := Trim(V1);
  Clean2 := Trim(V2);
  P1 := Pos('-', Clean1);
  if P1 > 0 then Clean1 := Copy(Clean1, 1, P1 - 1);
  P2 := Pos('-', Clean2);
  if P2 > 0 then Clean2 := Copy(Clean2, 1, P2 - 1);
  Result := CompareText(Clean1, Clean2);
end;

function InitializeSetup(): Boolean;
var
  i: Integer;
  InstalledVer: String;
begin
  Result := True;

  // 1. Prohibit /ALLUSERS override - per-user mode is enforced by script
  for i := 1 to ParamCount do
  begin
    if CompareText(ParamStr(i), '/ALLUSERS') = 0 then
    begin
      MsgBox('Instalator SPLogbook obsługuje wyłącznie instalację per-user. Parametr /ALLUSERS jest niedozwolony.', mbError, MB_OK);
      Result := False;
      Exit;
    end;
  end;

  // 2. Fail-closed on existing MSI installation conflict
  if RegKeyExists(HKLM, 'Software\Classes\Installer\UpgradeCodes\0B205E981ED3B4A9A9B4A9E189E502B0') or
     RegKeyExists(HKCU, 'Software\Classes\Installer\UpgradeCodes\0B205E981ED3B4A9A9B4A9E189E502B0') or
     RegKeyExists(HKLM, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}') or
     RegKeyExists(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}_is1') then
  begin
    MsgBox('Wykryto instalację SPLogbook zarządzaną przez MSI. Instalator User Setup nie może współdzielić instalacji z MSI.', mbError, MB_OK);
    Result := False;
    Exit;
  end;

  // 3. Reject downgrade if older version is being installed over newer
  if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}_is1', 'DisplayVersion', InstalledVer) then
  begin
    if (InstalledVer <> '') and (CompareSemVer(InstalledVer, '{#AppVersion}') > 0) then
    begin
      MsgBox('Wykryto nowszą wersję SPLogbook (' + InstalledVer + '). Próba downgrade do {#AppVersion} została zablokowana.', mbError, MB_OK);
      Result := False;
      Exit;
    end;
  end;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if CurPageID = wpSelectDir then
  begin
    // Refuse to install into a directory containing Portable ZIP marker
    if FileExists(ExpandConstant('{app}\.splogbook-portable')) then
    begin
      MsgBox('Wybrany katalog zawiera instalację SPLogbook Portable ZIP. Wybierz inny katalog dla instalatora User Setup.', mbError, MB_OK);
      Result := False;
      Exit;
    end;

    // Refuse to install into a directory containing MSI marker
    if FileExists(ExpandConstant('{app}\.splogbook-msi-marker')) then
    begin
      MsgBox('Wybrany katalog zawiera instalację SPLogbook MSI. Wybierz inny katalog dla instalatora User Setup.', mbError, MB_OK);
      Result := False;
      Exit;
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  MarkerPath: String;
  MarkerContent: String;
begin
  if CurStep = ssPostInstall then
  begin
    MarkerPath := ExpandConstant('{app}\.splogbook-inno-marker');
    MarkerContent := '{' + #13#10 +
      '  "marker_version": "1.0",' + #13#10 +
      '  "installation_type": "WindowsInnoSetup",' + #13#10 +
      '  "management_mode": "SystemManaged",' + #13#10 +
      '  "package_version": "' + ExpandConstant('{#AppVersion}') + '",' + #13#10 +
      '  "app_id": "{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}",' + #13#10 +
      '  "installation_scope": "CurrentUser",' + #13#10 +
      '  "installation_root": "' + ExpandConstant('{app}') + '",' + #13#10 +
      '  "build_commit": "' + ExpandConstant('{#GitCommit}') + '",' + #13#10 +
      '  "installer_engine": "InnoSetup"' + #13#10 +
      '}';
    SaveStringToFile(MarkerPath, MarkerContent, False);
  end;
end;
