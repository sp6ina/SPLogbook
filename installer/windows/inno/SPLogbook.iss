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

#define AppMutexName "SPLogbook_Instance_Mutex_{{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}"

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

AppMutex={#AppMutexName},SPLogbook
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

; Brak preinstalowanych baz referencyjnych (R1C/F-09): dane użytkownika i referencje w %APPDATA%\SPLogbook.

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
function ExtractNextToken(var S: String; const Delim: String): String;
var
  DelimPos: Integer;
begin
  DelimPos := Pos(Delim, S);
  if DelimPos > 0 then
  begin
    Result := Copy(S, 1, DelimPos - 1);
    S := Copy(S, DelimPos + Length(Delim), Length(S) - DelimPos - Length(Delim) + 1);
  end
  else
  begin
    Result := S;
    S := '';
  end;
end;

function IsAllDigits(const S: String): Boolean;
var
  I: Integer;
begin
  if S = '' then
  begin
    Result := False;
    Exit;
  end;
  for I := 1 to Length(S) do
  begin
    if (S[I] < '0') or (S[I] > '9') then
    begin
      Result := False;
      Exit;
    end;
  end;
  Result := True;
end;

function CompareNumericStrings(const A, B: String): Integer;
var
  NA, NB: String;
  Cmp: Integer;
begin
  NA := A;
  NB := B;
  while (Length(NA) > 1) and (NA[1] = '0') do
    NA := Copy(NA, 2, Length(NA) - 1);
  while (Length(NB) > 1) and (NB[1] = '0') do
    NB := Copy(NB, 2, Length(NB) - 1);

  if Length(NA) < Length(NB) then
    Result := -1
  else if Length(NA) > Length(NB) then
    Result := 1
  else
  begin
    Cmp := CompareStr(NA, NB);
    if Cmp < 0 then
      Result := -1
    else if Cmp > 0 then
      Result := 1
    else
      Result := 0;
  end;
end;

function ComparePreRelease(const Pre1, Pre2: String): Integer;
var
  Rest1, Rest2, Tok1, Tok2: String;
  Num1, Num2: Boolean;
  Cmp: Integer;
begin
  if (Pre1 = '') and (Pre2 = '') then
  begin
    Result := 0;
    Exit;
  end;
  if Pre1 = '' then
  begin
    Result := 1;
    Exit;
  end;
  if Pre2 = '' then
  begin
    Result := -1;
    Exit;
  end;

  Rest1 := Pre1;
  Rest2 := Pre2;
  while (Rest1 <> '') or (Rest2 <> '') do
  begin
    if Rest1 = '' then
    begin
      Result := -1;
      Exit;
    end;
    if Rest2 = '' then
    begin
      Result := 1;
      Exit;
    end;

    Tok1 := ExtractNextToken(Rest1, '.');
    Tok2 := ExtractNextToken(Rest2, '.');
    Num1 := IsAllDigits(Tok1);
    Num2 := IsAllDigits(Tok2);

    if Num1 and Num2 then
      Cmp := CompareNumericStrings(Tok1, Tok2)
    else if Num1 and (not Num2) then
      Cmp := -1
    else if (not Num1) and Num2 then
      Cmp := 1
    else
    begin
      Cmp := CompareStr(Tok1, Tok2);
      if Cmp < 0 then
        Cmp := -1
      else if Cmp > 0 then
        Cmp := 1;
    end;

    if Cmp <> 0 then
    begin
      Result := Cmp;
      Exit;
    end;
  end;
  Result := 0;
end;

function CompareSemVer(const V1, V2: String): Integer;
var
  S1, S2, Core1, Core2, Pre1, Pre2, Tok1, Tok2: String;
  P, I, Cmp: Integer;
begin
  S1 := Trim(V1);
  S2 := Trim(V2);

  if (Length(S1) > 0) and ((S1[1] = 'v') or (S1[1] = 'V')) then
    S1 := Copy(S1, 2, Length(S1) - 1);
  if (Length(S2) > 0) and ((S2[1] = 'v') or (S2[1] = 'V')) then
    S2 := Copy(S2, 2, Length(S2) - 1);

  P := Pos('+', S1);
  if P > 0 then
    S1 := Copy(S1, 1, P - 1);
  P := Pos('+', S2);
  if P > 0 then
    S2 := Copy(S2, 1, P - 1);

  P := Pos('-', S1);
  if P > 0 then
  begin
    Core1 := Copy(S1, 1, P - 1);
    Pre1 := Copy(S1, P + 1, Length(S1) - P);
  end
  else
  begin
    Core1 := S1;
    Pre1 := '';
  end;

  P := Pos('-', S2);
  if P > 0 then
  begin
    Core2 := Copy(S2, 1, P - 1);
    Pre2 := Copy(S2, P + 1, Length(S2) - P);
  end
  else
  begin
    Core2 := S2;
    Pre2 := '';
  end;

  for I := 1 to 3 do
  begin
    Tok1 := ExtractNextToken(Core1, '.');
    Tok2 := ExtractNextToken(Core2, '.');
    if not IsAllDigits(Tok1) then
      Tok1 := '0';
    if not IsAllDigits(Tok2) then
      Tok2 := '0';
    Cmp := CompareNumericStrings(Tok1, Tok2);
    if Cmp <> 0 then
    begin
      Result := Cmp;
      Exit;
    end;
  end;

  Result := ComparePreRelease(Pre1, Pre2);
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

// Escapes a value for use inside a JSON string literal: backslash first,
// then double quote (Windows paths contain backslashes).
function EscapeJsonString(const S: String): String;
begin
  Result := S;
  StringChangeEx(Result, '\', '\\', True);
  StringChangeEx(Result, '"', '\"', True);
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
      '  "package_version": "' + EscapeJsonString(ExpandConstant('{#AppVersion}')) + '",' + #13#10 +
      '  "app_id": "{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}",' + #13#10 +
      '  "installation_scope": "CurrentUser",' + #13#10 +
      '  "installation_root": "' + EscapeJsonString(ExpandConstant('{app}')) + '",' + #13#10 +
      '  "build_commit": "' + EscapeJsonString(ExpandConstant('{#GitCommit}')) + '",' + #13#10 +
      '  "installer_engine": "InnoSetup"' + #13#10 +
      '}';
    SaveStringToFile(MarkerPath, MarkerContent, False);
  end;
end;
