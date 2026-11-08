# RAPORT PRZEDIMPLEMENTACYJNY: STAGE M3-E — WINDOWS MSI WIX INTEGRATION GATE

**Data robocza projektu:** 2026-11-08  
**Data środowiska:** 2026-11-08  
**Czas środowiska:** 13:08:00  
**Timezone:** Central European Standard Time ((UTC+01:00) Sarajewo, Skopie, Warszawa, Zagrzeb)  
**UTC offset:** +01:00  
**Kanon bramki:** Implementacja, integracja i natywna weryfikacja instalatora Windows System Setup MSI dla SPLogbook 1.3.0-alpha.1 przy użyciu formalnie zweryfikowanego WiX Toolset  
**Status bramki:** `BLOCKED_BY_TOOLCHAIN_APPROVAL` / `ENVIRONMENT_BLOCKED`  

---

## 1. Data i czas

- **DATA ROBOCZA PROJEKTU:** 2026-11-08
- **DATA ŚRODOWISKA:** 2026-11-08
- **CZAS ŚRODOWISKA:** 13:08:00
- **TIMEZONE:** Central European Standard Time ((UTC+01:00) Sarajewo, Skopie, Warszawa, Zagrzeb)
- **UTC OFFSET:** +01:00
- **Integralność zegara:** Data robocza i data środowiska są w 100% zbieżne (2026-11-08). Zegar systemowy nie był modyfikowany.

---

## 2. Stan repozytorium Git

- **HEAD przed fetch:** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **HEAD po fetch (`git fetch --prune origin`):** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **main:** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **origin/main:** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **repair/full-system-remediation:** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **Tag v1.2.0:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` (nienaruszony, zweryfikowany lokalnie i zdalnie)
- **Ahead / Behind:** 0 / 0
- **Working Tree:** CLEAN (poza utworzonymi raportami przedimplementacyjnymi)
- **Operacje Git w toku:** Brak (`MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD` nieaktywne)
- **Klasyfikacja Git:** `A. EXACT_REPORTED_BASELINE`

---

## 3. Toolchain WiX

- **Znalezione wersje WiX:** Brak zainstalowanych narzędzi w systemie Windows.
- **Weryfikacja CLI:**
  - `wix.exe`: NOT FOUND
  - `candle.exe`: NOT FOUND
  - `light.exe`: NOT FOUND
  - `dark.exe`: NOT FOUND
  - `torch.exe`: NOT FOUND
  - `insignia.exe`: NOT FOUND
  - `lessmsi.exe`: NOT FOUND
- **Środowisko .NET / MSBuild:**
  - `dotnet.exe`: NOT FOUND (brak .NET SDK w PATH oraz `C:\Program Files\dotnet`)
  - `MSBuild.exe`: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe` (v17.14.23)
  - `msiexec.exe`: `C:\Windows\System32\msiexec.exe` (v5.0.26100.9949, SHA256: `6C63CFD4CB36998E9FF763117D4A74193BA72A5FC7F51EA7210F9065C2188452`)
- **Metoda uruchamiania:** Niedostępna do czasu autoryzacji i udostępnienia toolchainu.
- **Extensions:** Wymagane `WixUtilExtension` / `WixUIExtension`.
- **Status decyzji właściciela:**
  - `git grep -n "OWNER-M3-E" -- .` -> BRAK (0 dopasowań).
  - Wymagana rejestracja formalnej decyzji: `OWNER-M3-E-TOOLCHAIN-01`.
  - Szczegółowa analiza wariantu v3 vs v4/v5 zawarta w: `STAGE_M3_E_WIX_TOOLCHAIN_DECISION_PROPOSAL.md`.

---

## 4. BuildIdentity

- **Package Version:** `1.3.0-alpha.1`
- **Commit:** `954fca396b2986abe783f5cc8b0d1aca1bf19be1`
- **Channel:** `alpha`
- **Distribution Status:** `development`
- **Profile:** `release`
- **Target:** `windows-x86_64` (x64)
- **Status zależności:** Brak zmian w `Cargo.toml` i `Cargo.lock` (hash `c2247198422ca6d52415d38efb6229b13a3ba9f6` oraz `fe5f33bc1f40f9c6824760bbe06604590ab9a938`).

---

## 5. MSI Design

- **ProductName:** `SPLogbook`
- **Manufacturer:** `SP6INA & SPLogbook Team`
- **ProductVersion (MSI):** `1.3.0` (zgodnie ze specyfikacją Windows Installer wymagającą formatu liczbowego `major.minor.build` bez przyrostków tekstowych)
- **DisplayVersion (ARP):** `1.3.0-alpha.1` (pełny SemVer 2.0 z zachowaniem identyfikatora prerelease)
- **UpgradeCode:** `B7D14532-6A10-4F8A-9A4B-3DE189E502B0` (stały GUID rodziny MSI SPLogbook, zatwierdzony w `DESIGN_3_0_A_WINDOWS_UPDATE.md` L140 oraz sprawdzany w sekcji `[Code]` Inno Setup)
- **Inno Setup AppId:** `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}` (jawnie różny od UpgradeCode)
- **ProductCode policy:** Generowany dynamicznie per release (`*` w WiX) w celu wsparcia transakcyjnego `MajorUpgrade`
- **PackageCode policy:** Unikalny GUID generowany dla każdego buildu MSI
- **Architecture:** `x64`
- **Scope:** `Per-machine` (`ALLUSERS=1`, instalacja systemowa)
- **Target Directory:** `ProgramFiles64Folder\SPLogbook` (`%ProgramFiles%\SPLogbook`)
- **Major Upgrade:** Wdrożenie schematu `<MajorUpgrade>` z harmonogramem `Schedule="afterInstallInitialize"` lub `Schedule="afterInstallExecute"`
- **Downgrade Rejection:** Aktywna reguła `DowngradeErrorMessage` blokująca instalację starszej wersji na nowszą
- **Repair:** Standardowa obsługa Windows Installer (`msiexec /fa`), z wyłączną naprawą plików programu należących do MSI
- **Uninstall:** Usunięcie wyłącznie zasobów MSI, wpisów rejestru HKLM oraz skrótów bez dotykania danych użytkownika

---

## 6. Konflikty instalatorów

1. **Konflikt z User Setup (Inno Setup):**
   - Weryfikacja klucza deinstalatora per-user: `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}_is1`.
   - Weryfikacja katalogu domyślnego User Setup: `%LOCALAPPDATA%\Programs\SPLogbook`.
   - Weryfikacja markera `WindowsInnoSetup`.
   - **Polityka:** `fail-closed` (`ManagedInstallerConflict`). Instalator MSI odmawia instalacji, jeśli w systemie istnieje instalacja per-user SPLogbook.
2. **Konflikt z Portable ZIP:**
   - Weryfikacja obecności markera Portable ZIP lub plików aplikacji w katalogu innym niż systemowy `Program Files`.
   - **Polityka:** `non-destructive`. MSI nie instaluje się do katalogu Portable ZIP, nie modyfikuje go i nie przejmuje jego plików.
3. **Konflikt z nieznaną instalacją (`Unknown`):**
   - **Polityka:** `fail-closed`. Odmowa instalacji przy wykryciu niespójnego stanu.

---

## 7. Dane chronione użytkownika

Bezwzględny zakaz włączania poniższych plików do komponentów MSI, nadpisywania, naprawiania lub usuwania podczas instalacji, upgrade, repair i uninstall:
- `station_config.json` (oraz katalog `%APPDATA%\SPLogbook\config`)
- `default_log.db`, `serviceLOG.db`, `callbook.db`, `splogbook_contacts.db` (oraz katalog `%APPDATA%\SPLogbook\databases`)
- Pliki transakcyjne baz danych SQLite: `*.db-wal`, `*.db-shm`, `*.db-journal`
- Pliki eksportów: `*.adi`, `*.adif`, `*.cbr`, `*.cabrillo` (katalog `%APPDATA%\SPLogbook\exports`)
- Logi diagnostyczne aplikacji: `%APPDATA%\SPLogbook\logs`
- Pliki nieznane (`unknown files`) pozostawione w katalogu aplikacji.

Komponenty MSI obejmują **wyłącznie** binaria aplikacji i zasoby statyczne:
- `SPLogbook.exe` (KeyPath)
- `splogbook-updater.exe` (KeyPath)
- Ikona aplikacji `app.ico`
- Pliki licencji i dokumentacji (`LICENSE`, `README.md`)

---

## 8. Integracja z Updaterem (`SystemManaged`)

Dla instalacji `InstallationType::WindowsMsi`:
- **ManagementMode:** `ManagementMode::SystemManaged`
- **Strategia:** Pełna delegacja do silnika Windows Installer (`msiexec.exe`)
- **Zakazy:**
  - Brak `self-apply` (aplikacja nie podmienia własnych plików w `%ProgramFiles%`).
  - Brak `portable helper apply` (pomocnik nie wykonuje podmiany wieloplikowej w Program Files).
  - Brak `PowerShell fallback` (zakaz używania skryptów PowerShell do podmiany plików MSI).
- **Proces aktualizacji:**
  1. Updater weryfikuje podpisany manifest wydania (`VerifiedReleaseManifest`).
  2. Pobiera i weryfikuje zasób MSI (`VerifiedReleaseAsset` typu `windows-msi`).
  3. Sprawdza hash SHA-256 oraz rozmiar pliku.
  4. Uruchamia `msiexec.exe` bez użycia powłoki (`shell=false`), z parametrami:
     `msiexec.exe /i "<ścieżka_do_msi>" /qn /norestart /L*v "<ścieżka_do_logu>"`
  5. Weryfikuje kod wyjścia (`exit code`):
     - `0`: Sukces (`SUCCESS`)
     - `3010`: Sukces wymagający restartu systemu (`SUCCESS_REBOOT_REQUIRED`)
     - `1602`: Anulowanie przez użytkownika (`USER_CANCELLED`)
     - `1603`: Błąd krytyczny instalatora (`FATAL_INSTALL_ERROR`)
     - `1618`: Inna instalacja w toku (`ANOTHER_INSTALLATION_IN_PROGRESS`)
     - `1638`: Nowsza wersja już zainstalowana (`ANOTHER_VERSION_ALREADY_INSTALLED`)
  6. Wymusza brak automatycznego restartu systemu (`/norestart`).
  7. Po restarcie aplikacji weryfikuje tożsamość wersji (`BuildIdentity`).

---

## 9. Analiza ryzyk architektonicznych

1. **Brak autoryzowanego toolchainu WiX (Ryzyko P0):**
   - Środowisko nie posiada zainstalowanego WiX ani .NET SDK.
   - Próba automatycznej instalacji naruszyłaby zasady bezwzględne 20-28.
   - Wymagane zatwierdzenie propozycji `STAGE_M3_E_WIX_TOOLCHAIN_DECISION_PROPOSAL.md`.
2. **Podwyższenie uprawnień (Elevation / UAC):**
   - Instalacja per-machine wymaga podwyższonych uprawnień. Windows Installer zarządza podwyższeniem uprawnień w sposób natywny i transakcyjny. Zabrania się własnych implementacji `runas`.
3. **Ograniczenia ProductVersion w Windows Installer:**
   - Wersja SemVer `1.3.0-alpha.1` zawiera człon prerelease, którego `ProductVersion` w MSI nie dopuszcza.
   - Zastosowanie rozdzielenia: `ProductVersion = "1.3.0"` oraz `DisplayVersion = "1.3.0-alpha.1"`.
4. **Usuwanie danych użytkownika podczas deinstalacji:**
   - Zastosowanie ścisłej allowlisty komponentów gwarantuje, że mechanizm deinstalacji Windows Installer usuwa wyłącznie zarejestrowane pliki programu.

---

## 10. Plan commitów po odblokowaniu bramki

1. **Commit 1:** `chore(installer): add approved Windows MSI WiX definition (M3-E)`
   - Dodanie definicji projektu WiX, konfiguracji x64 per-machine, allowlisty plików i skryptu budującego `scripts/build_windows_msi.ps1`.
2. **Commit 2:** `feat(installer): define MSI identity and major upgrade policy (M3-E)`
   - Integracja stałego `UpgradeCode`, dynamicznego `ProductCode`, mapowania wersji oraz reguł blokady downgrade.
3. **Commit 3:** `feat(installer): detect and protect Windows installer conflicts (M3-E)`
   - Implementacja markera `WindowsMsi`, mechanizmu wykrywania konfliktu z Inno Setup (`fail-closed`) i ochrony Portable ZIP.
4. **Commit 4:** `feat(updater): delegate MSI updates to Windows Installer (M3-E)`
   - Integracja updater'a z `msiexec`, obsługa ustrukturyzowanych argumentów, logowania `/L*v` i mapowania kodów wyjścia.
5. **Commit 5:** `fix(installer): preserve user data across MSI lifecycle (M3-E)`
   - Ochrona baz danych SQLite, konfiguracji, logów i plików nieznanych podczas cyklu życia instalatora MSI.
6. **Commit 6:** `test(installer): verify Windows MSI lifecycle safety (M3-E)`
   - Wdrożenie kompleksowych testów jednostkowych, integracyjnych i strażników bezpieczeństwa w `tests/security_guard.rs` i dedykowanych testach MSI.
7. **Commit 7:** `docs(installer): close Windows MSI WiX integration gate (M3-E)`
   - Aktualizacja dokumentacji projektu, raportu końcowego, README, Roadmapy i rejestru audytu.

---

## 11. Podsumowanie i werdykt wstępny

- **Baza kodu Rust:** W 100% zielona (2573 testy passed, 0 failed, 0 warnings w clippy, binaria release zbudowane pomyślnie).
- **Stan repozytorium Git:** Czysty, nienaruszony, w pełni zsynchronizowany (`main == origin/main == 954fca396b2986abe783f5cc8b0d1aca1bf19be1`).
- **Warunek blokujący:** Brak zainstalowanego narzędzia WiX Toolset oraz brak formalnej decyzji właściciela projektu `OWNER-M3-E-TOOLCHAIN-01`.
- **Rekomendowane działanie:** Przedstawienie dokumentu `STAGE_M3_E_WIX_TOOLCHAIN_DECISION_PROPOSAL.md` Właścicielowi Projektu w celu formalnego podjęcia decyzji o wyborze i instalacji toolchainu WiX Toolset.
