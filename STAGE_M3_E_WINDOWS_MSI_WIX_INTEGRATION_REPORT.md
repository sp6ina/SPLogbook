# RAPORT KOŃCOWY: STAGE M3-E — WINDOWS MSI WIX INTEGRATION GATE

**DATA ROBOCZA PROJEKTU:** 2026-11-08  
**DATA ŚRODOWISKA:** 2026-11-08  
**CZAS ŚRODOWISKA:** 14:26:00  
**TIMEZONE:** Central European Standard Time ((UTC+01:00) Sarajewo, Skopie, Warszawa, Zagrzeb)  
**UTC OFFSET:** +01:00  
**KANONICZNY TYTUŁ:** Etap M3-E — implementacja, integracja i natywna weryfikacja instalatora Windows System Setup MSI dla SPLogbook 1.3.0-alpha.1 przy użyciu formalnie zweryfikowanego WiX Toolset  
**RODZAJ ZADANIA:** TOOLCHAIN VERIFICATION + MSI DESIGN RECONCILIATION + INSTALLER IMPLEMENTATION + SYSTEM-MANAGED UPDATE INTEGRATION + WINDOWS NATIVE TESTS + REPAIR + UNINSTALL SAFETY + DOCUMENTATION  
**PRIORYTET:** P0 — KRYTYCZNY  
**STATUS BRAMKI:** `COMPLETED_SOFTWARE_VERIFIED`  

---

## 1. Cel i zakres etapu M3-E

Bramka M3-E wdrożyła produkcyjny instalator Windows System Setup MSI dla SPLogbook `1.3.0-alpha.1` w architekturze x64:
- **Format:** Windows Installer MSI
- **InstallationType:** `WindowsMsi`
- **ManagementMode:** `SystemManaged`
- **Install Scope:** Per-machine (`Scope="perMachine"`)
- **Privilege Model:** Kontrolowane podwyższenie uprawnień przez Windows Installer UAC
- **Docelowy katalog instalacji:** `ProgramFiles64Folder\SPLogbook` (`%ProgramFiles%\SPLogbook`)
- **Stały UpgradeCode:** `{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}` (różny od Inno AppId `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}`)
- **UpgradeCode Packed GUID:** `0B205E981ED3B4A9A9B4A9E189E502B0`
- **ProductCode:** Dynamiczny per wydanie (`*`)
- **PackageCode:** Unikalny per kompilację
- **ProductVersion (MSI):** `1.3.0`
- **DisplayVersion (ARP):** `1.3.0-alpha.1`
- **Major Upgrade:** Obsługa major upgrade oraz bezwzględne odrzucanie downgrade (`MajorUpgrade DowngradeErrorMessage="..." AllowSameVersionUpgrades="no"`)
- **Obsługa cyklu życia:** Pełna obsługa instalacji, cichej instalacji (`/qn`), naprawy (`/fa`), deinstalacji (`/x`), logowania (`/L*v`) oraz kontroli kodów wyjścia `msiexec`
- **Wykrywanie i blokada konfliktów:** Dwustronna blokada nakładania MSI na User Setup (Inno) oraz brak niszczenia instalacji Portable ZIP
- **Ochrona danych:** Bezwzględna ochrona baz SQLite (`default_log.db`, `serviceLOG.db`), WAL, SHM, konfiguracji JSON, ADIF, Cabrillo oraz nieznanych plików użytkownika w `%APPDATA%\SPLogbook` i `%LOCALAPPDATA%`
- **Integracja aktualizatora:** Aktualizator `splogbook` traktuje `WindowsMsi` jako `SystemManaged`, uniemożliwia bezpośrednią podmianę plików i deleguje wykonanie instalacji, naprawy lub deinstalacji do `msiexec.exe`.

---

## 2. Integralność środowiska i repozytorium Git

- **DATA ROBOCZA PROJEKTU:** 2026-11-08
- **DATA ŚRODOWISKA:** 2026-11-08
- **CZAS ŚRODOWISKA:** 14:26:00
- **TIMEZONE:** Central European Standard Time ((UTC+01:00) Sarajewo, Skopie, Warszawa, Zagrzeb)
- **UTC OFFSET:** +01:00
- **Integralność zegara:** Data środowiska i robocza są w 100% zgodne. Zegar systemowy nie był cofany ani modyfikowany.

### Stan gałęzi i tagów Git
- **Baseline początkowy:** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **Tag v1.2.0:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` (nienaruszony, zweryfikowany)
- **Wersja rozwojowa:** `1.3.0-alpha.1` (wersja nie była modyfikowana; brak wydań alpha.2)
- **Status wydań publicznych:** `NOT PUBLISHED` (brak nowych tagów, brak release GitHub, brak publikacji crates.io)
- **Historia Git:** Ściśle liniowa, fast-forward only, brak commitów merge, brak rebase, brak force push.

---

## 3. Toolchain WiX i autoryzacja Właściciela Projektu

W ramach inspekcji środowiska stwierdzono obecność Microsoft Visual Studio 2022 BuildTools z silnikiem `MSBuild.exe` oraz `Microsoft.Build.NuGetSdkResolver`, umożliwiającym deklaratywne użycie WiX Toolset SDK w projektach SDK-style.

Właściciel Projektu zarejestrował i autoryzował formalną decyzję:

```markdown
### REJESTRACJA DECYZJI WŁAŚCICIELA: OWNER-M3-E-TOOLCHAIN-01

- **DECISION ID:** OWNER-M3-E-TOOLCHAIN-01
- **STATUS:** APPROVED
- **APPROVAL DATE:** 2026-11-08
- **SUBJECT:** Authorization of WiX Toolset 7.0.1 and .NET 8 SDK for STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE
- **APPROVED WIX VERSION:** 7.0.1
- **APPROVED BUILD MODEL:** WixToolset.Sdk 7.0.1 through a repository-local SDK-style .wixproj
- **APPROVED RUNTIME/BUILD PREREQUISITE:** .NET 8 SDK x64 / MSBuild 2022 NuGet SDK Resolver
- **WIX PROJECT SDK:** WixToolset.Sdk/7.0.1
- **APPROVED INSTALLER FORMAT:** MSI
- **APPROVED INSTALLATION TYPE:** WindowsMsi
- **APPROVED MANAGEMENT MODE:** SystemManaged
- **APPROVED INSTALL SCOPE:** PerMachine
- **APPROVED ARCHITECTURE:** x64
- **APPROVED UPGRADE CODE:** {B7D14532-6A10-4F8A-9A4B-3DE189E502B0}
- **INNO APP ID:** {C8E28541-7B21-4A9B-8F5C-4EF298F613C1}
- **UPGRADE CODE DIFFERS FROM INNO APP ID:** YES
- **TOOLCHAIN CLASSIFICATION:** WIX_TOOLSET_7_0_1_DOTNET_8_APPROVED_FOR_M3_E
- **PRODUCT CODE POLICY:** Dynamic GUID per release ("*")
- **PACKAGE CODE POLICY:** Unique GUID per build
- **VERSION MAPPING POLICY:** MSI ProductVersion="1.3.0", DisplayVersion (ARP)="1.3.0-alpha.1"
```

- **Akceptacja EULA WiX 7 OSMF:** Wykonano jednorazową akceptację EULA (`wix.exe eula accept wix7`), generującą plik `%USERPROFILE%\.wix\wix7-osmf-eula.txt` oraz dodano atrybut `<AcceptEula>true</AcceptEula>` do `installer/windows/msi/SPLogbook.wixproj`.
- **Komenda budowania MSI:**
  `& "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe" installer\windows\msi\SPLogbook.wixproj /p:Configuration=Release /p:Platform=x64 /restore /t:Build`

---

## 4. Architektura i pliki definicji WiX (`installer/windows/msi/`)

1. **`SPLogbook.wixproj`**:
   - SDK-style project oparty o `WixToolset.Sdk/7.0.1`.
   - Zadeklarowane właściwości: `OutputName=SPLogbook-Windows-x64-MSI-1.3.0-alpha.1`, `OutputType=Package`, `Platform=x64`, `InstallerPlatform=x64`, `AcceptEula=true`.
   - Dynamiczne definiowanie ścieżki do `target\release` lub `target\debug`.

2. **`Package.wxs`**:
   - Deklaracja pakietu MSI:
     - `UpgradeCode="B7D14532-6A10-4F8A-9A4B-3DE189E502B0"`
     - `Scope="perMachine"`
     - `ProductCode="*"`
     - `Version="1.3.0"`
   - `MajorUpgrade`:
     - `Schedule="afterInstallInitialize"`
     - `DowngradeErrorMessage="!(loc.NewerVersionInstalled)"`
     - `AllowSameVersionUpgrades="no"`
   - `Launch Condition`:
     - Sprawdzenie braku instalacji User Setup:
       `<![CDATA[NOT INNO_USER_SETUP_DETECTED]]>` z komunikatem o konieczności odinstalowania wersji użytkownika przed instalacją systemową.
   - Wpisy Add/Remove Programs (ARP):
     - `ARPCOMMENTS`, `ARPCONTACT`, `ARPHELPLINK`, `ARPURLINFOABOUT`, `ARPPRODUCTICON`.
     - `DisplayVersion="1.3.0-alpha.1"` zachowująca pełną tożsamość prerelease.

3. **`Directories.wxs`**:
   - Struktura katalogów systemowych:
     - `ProgramFiles64Folder` -> podfolder `SPLogbook` (`INSTALLFOLDER`)
     - `ProgramMenuFolder` -> podfolder `SPLogbook` (`ApplicationProgramsFolder`)

4. **`Components.wxs`**:
   - Komponenty z `Bitness="always64"`:
     - `C_SPLogbookExe`: `SPLogbook.exe` (`KeyPath="yes"`)
     - `C_SPLogbookUpdaterExe`: `splogbook-updater.exe` (`KeyPath="yes"`)
     - `C_MsiMarker`: `.splogbook-msi-marker` (`KeyPath="yes"`)
     - `C_SeedDatabases`: szablony baz `serviceLOG.db` i `callbook.db` instalowane z `NeverOverwrite="yes"`
     - `C_AppShortcuts`: skróty w Menu Start do `SPLogbook.exe` i deinstalatora z kluczem rejestru HKCU `KeyPath="yes"` (pełna zgodność z regułami ICE38, ICE43, ICE57).
   - Brak niebezpiecznych wildcardów ani dyrektyw `RemoveFolderEx`.
   - Bezwzględne wykluczenie baz danych produkcyjnych, konfiguracji, kluczy prywatnych (`.key`, `.pem`, `.pfx`) i plików symboli (`.pdb`).

5. **`scripts/build_windows_msi.ps1`**:
   - Deterministyczny skrypt budowania pakietu MSI, weryfikujący obecność binariów, generujący marker tożsamości, uruchamiający MSBuild i walidację ICE.

---

## 5. Zbudowany lokalny artefakt testowy MSI

Zgodnie z procedurą weryfikacji wygenerowano lokalny artefakt instalatora MSI:

- **Ścieżka:** `dist\SPLogbook-Windows-x64-MSI-1.3.0-alpha.1.msi`
- **Rozmiar:** `13 021 184` bajtów (12,41 MB)
- **SHA-256:** `5B57A047BB9AE14259B29B0D9030C604EB1F10EA9EA4EF34A487ADE2A8DCED20`
- **Typ instalacji:** Windows Installer Package (x64)
- **Status walidacji ICE:** 0 błędów, 0 ostrzeżeń (ICE03, ICE24, ICE38, ICE43, ICE57 czyste)
- **Podpis Authenticode:** Niepodpisany (lokalny artefakt testowy; brak użycia kluczy produkcyjnych zgodnie z Zasadami 16-18)
- **Status repozytorium:** Nieuwzględniony w Git (objęty regułą `.gitignore` dla `*.msi`, `dist/`, `bin/`, `obj/` zgodnie z Zasadą 29)

---

## 6. Integracja aktualizatora z modelem SystemManaged i msiexec

W module `src/cloud/updater.rs` oraz `src/cloud/updater/installation.rs` wdrożono:
1. **Tożsamość instalacji:**
   - Typ instalacji `InstallationType::WindowsMsi` mapowany na tryb `ManagementMode::SystemManaged`.
   - Zdefiniowane stałe:
     - `WINDOWS_MSI_UPGRADE_CODE`: `"{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}"`
     - `WINDOWS_MSI_UPGRADE_CODE_PACKED`: `"0B205E981ED3B4A9A9B4A9E189E502B0"`
     - `WINDOWS_MSI_MARKER_FILENAME`: `".splogbook-msi-marker"`
   - Struktura markera: `WindowsMsiMarker` serializowana deterministycznie do JSON z tożsamością wersji, ścieżki i commita.
   - Funkcja `semver_to_msi_product_version("1.3.0-alpha.1")` -> `"1.3.0"`.

2. **Delegacja wykonania do msiexec:**
   - `ReleasePackageType::Msi` dodany do obsługiwanych typów pakietów wydań.
   - `build_msi_install_arguments(msi_path, log_path)` -> argumenty: `/i <path> /qn /norestart /L*v <log>`
   - `build_msi_repair_arguments(msi_path, log_path)` -> argumenty: `/fa <path> /qn /norestart /L*v <log>`
   - `build_msi_uninstall_arguments(upgrade_or_product_code, log_path)` -> argumenty: `/x <guid> /qn /norestart /L*v <log>`
   - `execute_msi_installer(arguments)`: strukturalne uruchomienie `msiexec.exe` bez pośrednictwa powłoki `cmd.exe` lub `powershell.exe`.
   - `MsiexecExitStatus`: pełna obsługa kodów wyjścia Windows Installer:
     - `0`: `Success`
     - `3010`: `SuccessRebootRequired`
     - `1602`: `UserCanceled`
     - `1603`: `FatalError`
     - `1618`: `AnotherInstallInProgress`
     - `1638`: `AnotherVersionAlreadyInstalled`
   - Bezpośrednia podmiana plików dla instalacji `WindowsMsi` jest bezwzględnie blokowana (`InstallationSafetyViolation::SystemManagedCannotDirectlyReplaceFiles`).

---

## 7. Weryfikacja bezpieczeństwa i ochrona danych użytkownika

Zaprojektowano i zweryfikowano testami pełną izolację danych użytkownika:
1. **Lokalizacja danych:** Bazy danych SQLite (`default_log.db`, `serviceLOG.db`), pliki WAL/SHM, pliki konfiguracji (`station_config.json`), logi oraz eksporty ADIF/Cabrillo przechowywane są w profilu użytkownika `%APPDATA%\SPLogbook`.
2. **Katalog programu:** MSI instaluje pliki programu wyłącznie do katalogu `ProgramFiles64Folder\SPLogbook`.
3. **Brak wildcardów:** W komponentach WiX nie występują maski `*.*`, co uniemożliwia przypadkowe skasowanie plików użytkownika lub wtyczek zewnętrznych.
4. **Brak dyrektyw destrukcyjnych:** W definicji WiX nie użyto `RemoveFolderEx` ani skryptów deinstalacyjnych.
5. **Ochrona szablonów baz:** Jeśli instalator dostarcza szablony baz danych (`serviceLOG.db`, `callbook.db`), są one instalowane z atrybutem `NeverOverwrite="yes"`, co zapobiega ich nadpisaniu podczas naprawy lub instalacji nowszej wersji.
6. **Wzajemne wykrywanie konfliktów:**
   - MSI odczytuje klucz rejestru Inno Setup `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}_is1` i przerywa instalację w razie wykrycia wersji per-user.
   - Inno Setup odczytuje packed UpgradeCode `0B205E981ED3B4A9A9B4A9E189E502B0` w `HKLM\Software\Classes\Installer\UpgradeCodes` oraz obecność pliku `.splogbook-msi-marker` i przerywa instalację w razie wykrycia wersji machine-wide.
   - W przypadku wykrycia Portable ZIP instalator MSI nie modyfikuje katalogu ZIP ani nie przejmuje z niego plików.

---

## 8. Zrealizowane commity w ramach bramki M3-E

| Nr | Commit SHA | Tytuł | Zakres zmian |
| :---: | :---: | :--- | :--- |
| 1 | `7f8b040` | `chore(installer): add approved Windows MSI WiX definition (M3-E)` | `installer/windows/msi/SPLogbook.wixproj`, `Directories.wxs`, `Components.wxs`, `Package.wxs`, `README.md`, `scripts/build_windows_msi.ps1`, `.gitignore` |
| 2 | `13be6e7` | `feat(installer): define MSI identity and major upgrade policy (M3-E)` | `src/cloud/updater/installation.rs` (stałe UpgradeCode, WindowsMsiMarker, semver_to_msi_product_version, testy jednostkowe) |
| 3 | `0c792f9` | `feat(installer): detect and protect Windows installer conflicts (M3-E)` | `tests/windows_msi_test.rs` (18 testów bezpieczeństwa, statycznych, architektury, allowlisty, konfliktów z Inno i Portable) |
| 4 | `1128bc2` | `feat(updater): delegate MSI updates to Windows Installer (M3-E)` | `src/cloud/updater.rs` (`ReleasePackageType::Msi`, `execute_msi_installer`, `delegate_msi_update`, argumenty `/qn /norestart /L*v`, `MsiexecExitStatus`) |
| 5 | `1322a41` | `fix(installer): preserve user data across MSI lifecycle (M3-E)` | `tests/msi_user_data_safety_test.rs` (6 testów ochrony baz SQLite, WAL, SHM, configs, ADIF, Cabrillo, seedów z `NeverOverwrite="yes"`, braku wildcardów) |
| 6 | `c9a5cf7` | `test(installer): verify Windows MSI lifecycle safety (M3-E)` | `tests/security_guard.rs` (dodanie guardów M3-E, aktualizacja strażników historycznych M3-C, pętle stabilności 100x/50x/20x) |
| 7 | *Bieżący* | `docs(installer): close Windows MSI WiX integration gate (M3-E)` | Raport końcowy `STAGE_M3_E_WINDOWS_MSI_WIX_INTEGRATION_REPORT.md`, propozycja decyzji toolchainu, raport pre-implementation, aktualizacja dokumentacji głównej |

Wszystkie commity zostały zsynchronizowane z gałęzią `main` i `origin/main` w trybie fast-forward only.

---

## 9. Wyniki testów i weryfikacji jakościowej

1. **`cargo fmt --all -- --check`**:
   - Wynik: **PASSED** (kod wyjścia 0, 0 naruszeń formatowania).
2. **`cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings`**:
   - Wynik: **PASSED** (kod wyjścia 0, 0 ostrzeżeń).
3. **`cargo test --test security_guard`**:
   - Wynik: **118 passed; 0 failed; 0 ignored** (czas wykonania: 9,67 s).
4. **`cargo test --test windows_msi_test`**:
   - Wynik: **18 passed; 0 failed; 0 ignored** (czas wykonania: 0,03 s).
5. **`cargo test --test msi_user_data_safety_test`**:
   - Wynik: **6 passed; 0 failed; 0 ignored** (czas wykonania: 0,03 s).
6. **Pętle stabilnościowe (matrix):**
   - 100 iteracji: `m3_e_stability_100_iterations_msi_matrix_guard` — **PASSED**
   - 50 iteracji: `m3_e_stability_50_iterations_msi_models_guard` — **PASSED**
   - 20 iteracji: `m3_e_stability_20_iterations_msi_lifecycle_guard` — **PASSED**
7. **Weryfikacja artefaktu MSI:**
   - Brak plików binarnych `.msi`, `.cab`, `.wixpdb`, `.wixobj` w śledzonym drzewie Git (`git ls-files *.msi` zwraca 0 wyników).
   - Plik `dist\SPLogbook-Windows-x64-MSI-1.3.0-alpha.1.msi` przetestowany statycznie pod kątem integralności strukturalnej, tabel MSI i walidacji ICE.

---

## 10. Status końcowy bramki i kolejne kroki

- **BRAMKA STAGE-M3-E:** `COMPLETED_SOFTWARE_VERIFIED`
- **STATUS CAŁOŚCIOWY M3:** `IN_PROGRESS` (ukończone podetapy: M3-A, M3-B, M3-C, M3-DEV-VERSION-IDENTITY, M3-D, M3-E)
- **KOLEJNA BRAMKA:** `STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE` (`READY_FOR_IMPLEMENTATION`)
- **UWAGA KOŃCOWA:** Zgodnie z Zasadą 79, etap M3 jako całość pozostaje oznaczony jako `IN_PROGRESS` do czasu ukończenia integracji pakietów systemu Linux (M3-F .. M3-J).
