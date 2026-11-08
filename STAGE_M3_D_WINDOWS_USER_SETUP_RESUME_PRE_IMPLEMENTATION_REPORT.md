# RAPORT PRZEDIMPLEMENTACYJNY WZNOWIENIA BRAMKI M3-D
## RESUME-STAGE-M3-D-WINDOWS-USER-SETUP-INTEGRATION-GATE
### Data projektu: 2026-11-08

---

## 1. Data i Czas Środowiska

- **DATA ROBOCZA PROJEKTU:** 2026-11-08
- **DATA ŚRODOWISKA:** 2026-11-08
- **CZAS ŚRODOWISKA:** 10:21:47
- **TIMEZONE:** Central European Standard Time (Warszawa, Polska)
- **UTC OFFSET:** +01:00

Zegar systemowy nie był modyfikowany. Daty istniejących commitów pozostają nienaruszone.

---

## 2. Bramka Git i Integralność Repozytorium

- **HEAD przed fetch:** `db4417e3b8acff9bb87aaf04fa5d703e5fec74bc`
- **HEAD po fetch:** `db4417e3b8acff9bb87aaf04fa5d703e5fec74bc`
- **main:** `db4417e3b8acff9bb87aaf04fa5d703e5fec74bc`
- **origin/main:** `db4417e3b8acff9bb87aaf04fa5d703e5fec74bc`
- **repair/full-system-remediation:** `db4417e3b8acff9bb87aaf04fa5d703e5fec74bc`
- **Ahead / Behind:** 0 / 0
- **Klasyfikacja baseline:** `EXACT_REPORTED_BASELINE` (HEAD == main == origin/main == db4417e)
- **Working Tree:** `CLEAN` (0 niezatwierdzonych zmian, brak aktywnych operacji merge/rebase/cherry-pick/revert)
- **Ochrona tagu v1.2.0:** nienaruszony (commit `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`, tag ref `3ea8d51aab436805107a1cf422e0b35cc301458f`)
- **Tag 1.3.0-alpha.1:** brak (nie tworzono tagu, brak publikacji release)

---

## 3. Stan Warunków Wstępnych (Prerequisites)

1. **Regresja i18n:** Usunięta i zabezpieczona (`i18n_guard` 15/15 passed).
2. **Strażnik terminologiczny:** `transactional_replacement` wdrożony, termin `atomic_swap` zastąpiony we wszystkich wymaganych modułach, testy terminologii zielone.
3. **Bramka bazowa kodu:**
   - `cargo fmt --check`: PASSED
   - `cargo check --workspace --all-targets --all-features`: PASSED (0 błędów)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASSED (0 ostrzeżeń)
   - `cargo test --workspace --all-targets`: PASSED (1862 jednostkowych, 96 security guards, 33 windows helper, 1 theme guard, 15 i18n, 165 cat_native_6a, cat_native_6b, 297 cat_native_7, 5 updater bin)
   - `cargo audit`: PASSED (0 podatności)
   - `cargo tree -d`: sprawdzone, brak nieoczekiwanych duplikatów zależności
4. **Decyzja Właściciela Projektu:** `OWNER-M3-D-TOOLCHAIN-01` w statusie `APPROVED` od 2026-11-08.
5. **Kompilator Inno Setup:** zlokalizowany w systemie i zweryfikowany.

---

## 4. Zatwierdzony Toolchain Inno Setup

- **DECYZJA:** `OWNER-M3-D-TOOLCHAIN-01`
- **STATUS DECYZJI:** `APPROVED`
- **KLASYFIKACJA:** `INNO_SETUP_7_1_1_APPROVED_FOR_M3_D`
- **APPROVED PATH:** `C:\Program Files\Inno Setup 7\ISCC.exe`
- **TEST-PATH:** `True`
- **ZWERYFIKOWANA WERSJA:** `7.1.1`
- **SHA-256 ISCC.exe:** `E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644`
- **WYMÓG OSTRZEŻEŃ:** 0 ostrzeżeń kompilatora (każdy warning traktowany jest jako błąd blokujący bramkę)
- **ZAKAZY TOOLCHAINU:** zakaz pobierania, instalowania, modyfikacji PATH, winget/choco/scoop, kopiowania kompilatora do repozytorium.

---

## 5. Tożsamość Wydania (Build Identity)

- **PACKAGE VERSION:** `1.3.0-alpha.1` (Cargo.toml oraz Cargo.lock)
- **BUILD COMMIT:** `db4417e3b8acff9bb87aaf04fa5d703e5fec74bc` (w trakcie implementacji uzupełniany bieżącym hash)
- **BUILD CHANNEL:** `alpha`
- **DISTRIBUTION STATUS:** `development`
- **PROFILE:** `release` (dla binariów do instalatora)
- **TARGET ARCHITECTURE:** `x86_64-pc-windows-msvc`
- **NOWE ZALEŻNOŚCI RUST:** 0 (zabronione dodawanie nowych crates)

---

## 6. Projekt Architektoniczny Instalatora (Design)

- **KANONICZNY IDENTYFIKATOR APLIKACJI (AppId):**
  `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}`
- **TYP INSTALACJI (InstallationType):**
  `WindowsInnoSetup`
- **TRYB ZARZĄDZANIA (ManagementMode):**
  `SystemManaged`
- **ZAKRES INSTALACJI (Scope):**
  Per-user (`CurrentUser`)
- **UPRAWNIENIA (Privileges):**
  `PrivilegesRequired=lowest` (bezwzględny zakaz UAC, administratora i runas)
- **KATALOG DOMYŚLNY:**
  `{localappdata}\Programs\SPLogbook` (zakaz instalacji w `Program Files`)
- **ARCHITEKTURA INSTALATORA:**
  - `SetupArchitecture=x64`
  - `ArchitecturesAllowed=x64compatible`
  - `ArchitecturesInstallIn64BitMode=x64compatible`
- **REJESTR WINDOWS:**
  - Wyłącznie wpis w gałęzi `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}_is1`
  - Bezwzględny zakaz zapisu do `HKEY_LOCAL_MACHINE` (HKLM)
- **SKRÓTY:**
  - Menu Start bieżącego użytkownika: `{userprograms}\SPLogbook`
  - Opcjonalny skrót na pulpicie użytkownika: `{userdesktop}\SPLogbook` (zależny od taska `desktopicon`)
  - Zakaz skrótów ogólnosystemowych (`allusers`)
- **DELEGACJA UPDATERA:**
  - Auto-updater traktuje `WindowsInnoSetup` jako `SystemManaged`
  - Brak `self-apply`, brak `UpdatePlan` podmiany plików dla Inno Setup
  - Brak wykorzystania helpera Portable ZIP ani fallbacku PowerShell do instalacji
  - Pobranie i weryfikacja `VerifiedReleaseAsset` (wersja, commit, wielkość, SHA-256)
  - Bezpośrednie uruchomienie `Setup.exe` bez powłoki shell (`std::process::Command` bez cmd/powershell)
  - Argumenty wywołania: `/SILENT /NORESTART /LOG="<kontrolowana ścieżka>"`
  - Weryfikacja kodu wyjścia i późniejszy health check tożsamości

---

## 7. Konflikty Instalacyjne i Polityka Bezpieczeństwa

- **Współistnienie z Portable ZIP:**
  - Instalacje Portable ZIP i User Setup działają w odrębnych katalogach.
  - Instalator Inno Setup odmawia instalacji w katalogu zawierającym marker `.splogbook-portable`.
  - Instalator nie usuwa, nie modyfikuje ani nie konwertuje instalacji Portable ZIP.
- **Konflikt z MSI (WiX):**
  - Wykrycie obecności instalacji MSI (rejestr UpgradeCode `0B205E981ED3B4A9A9B4A9E189E502B0` lub deinstalator MSI) powoduje natychmiastowe przerwanie instalacji (`fail-closed`).
  - Instalator Inno Setup nie wywołuje `msiexec`, nie odinstalowuje MSI ani nie nadpisuje jego plików.
- **Działający proces programu (`SPLogbook.exe`):**
  - Obsługa przez `AppMutex=SPLogbook_Instance_Mutex_{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}` oraz `CloseApplications=yes`.
  - W trybie interaktywnym: monit do użytkownika o zamknięcie programu.
  - W trybie cichym (`/SILENT`): `fail-closed` w razie niemożności bezpiecznego zamknięcia — brak agresywnego ubijania procesu po nazwie (`taskkill`/`TerminateProcess` zabronione).

---

## 8. Polityka Ochrony Danych Użytkownika

Instalator, deinstalator, proces reinstalacji oraz aktualizacji podlegają bezwzględnej ochronie:
- Bazy danych SQLite: `station_config.json`, `default_log.db`, `serviceLOG.db`, `callbook.db`, `splogbook_contacts.db`, `*.db`.
- Pliki transakcyjne bazy: `*.db-wal`, `*.db-shm`, `*.db-journal`.
- Pliki eksportu i logów: `*.adi`, `*.adif`, `*.cbr`, `*.cabrillo`, katalogi `logs/`, `exports/`, `databases/`, `config/`.
- Nieznane pliki użytkownika umieszczone w katalogu aplikacji: pozostają nienaruszone.
- Reguły deinstalatora:
  - Brak wildcardów (`*.*`) w sekcji `[UninstallDelete]`.
  - Brak procedury `DelTree({app})`.
  - Użycie `dirifempty` wyłącznie dla katalogów programu po uprzednim usunięciu plików śledzonych.
  - Całkowity zakaz usuwania katalogu AppData użytkownika (`%APPDATA%\SPLogbook` oraz `%LOCALAPPDATA%\SPLogbook`).

---

## 9. Planowane Pliki

1. **Definicja instalatora Inno Setup:**
   - `installer/windows/inno/SPLogbook.iss` (główny skrypt kompilacji instalatora)
   - `installer/windows/inno/README.md` (kompletna dokumentacja architektoniczna instalatora)
2. **Skrypt automatyzacji budowy:**
   - `scripts/build_windows_user_setup.ps1` (skrypt weryfikujący clean working tree, kompilujący binarki Rust, przygotowujący staging allowlisty i uruchamiający ISCC 7.1.1 z wymogiem 0 ostrzeżeń)
3. **Kod produkcyjny Rust (detekcja i integracja aktualizatora):**
   - `src/cloud/updater/installation.rs` (rozszerzenie struktury markera User Setup `.splogbook-inno-marker` w formacie JSON oraz detekcja konfliktów)
   - `src/cloud/updater.rs` oraz `src/cloud/updater/platform.rs` (zarządzanie procesem aktualizacji `WindowsInnoSetup` w trybie `SystemManaged`, obsługa zasobu `inno-exe`, strukturalne parametry wywołania)
4. **Testy jednostkowe, integracyjne i strażnicy:**
   - `tests/security_guard.rs` (rozszerzenie o komplet strażników M3-D)
   - `tests/windows_inno_setup_test.rs` (testy struktury `.iss`, allowlisty, braku wildcardów, braku eskalacji uprawnień, integracji z updaterem)
   - Testy natywne instalacji i deinstalacji na Windows (weryfikacja E2E instalatora w izolowanym katalogu testowym)
5. **Dokumentacja i rejestry:**
   - `STAGE_M3_D_WINDOWS_USER_SETUP_INTEGRATION_REPORT.md`
   - Aktualizacja: `README.md`, `MASTER_IMPLEMENTATION_PLAN.md`, `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`

---

## 10. Plan Commitów i Synchronizacji

Zgodnie z Section 40 zlecenia, zaplanowano 6 atomowych, liniowych commitów:

1. **COMMIT 1:**
   `chore(installer): add approved Windows User Setup definition (M3-D)`
   - Skrypt `installer/windows/inno/SPLogbook.iss`
   - Stały AppId `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}`
   - Konfiguracja x64, per-user, `PrivilegesRequired=lowest`, `{localappdata}\Programs\SPLogbook`
   - Allowlista plików stagingu i skrypt `scripts/build_windows_user_setup.ps1`

2. **COMMIT 2:**
   `feat(installer): detect and protect Windows installation types (M3-D)`
   - Implementacja markera User Setup z metadanymi (JSON)
   - Integracja z `InstallationType::WindowsInnoSetup` i `ManagementMode::SystemManaged`
   - Detekcja konfliktów z Portable ZIP i MSI (fail-closed)

3. **COMMIT 3:**
   `feat(updater): delegate User Setup updates to verified installer (M3-D)`
   - Obsługa formatu `inno-exe` w `VerifiedReleaseAsset`
   - Delegacja aktualizacji do instalatora z argumentami `/SILENT /NORESTART /LOG=...`
   - Weryfikacja kodu powrotu i zakaz bezpośredniej podmiany plików w katalogu programu

4. **COMMIT 4:**
   `fix(installer): preserve user data during setup lifecycle (M3-D)`
   - Wdrożenie polityki seed files (`FirstInstallSeed`)
   - Ochrona baz danych, konfiguracji, logów i plików nieznanych podczas deinstalacji
   - Brak wildcard uninstall delete

5. **COMMIT 5:**
   `test(installer): verify Windows User Setup lifecycle safety (M3-D)`
   - Strażniki architektury i bezpieczeństwa w `tests/security_guard.rs`
   - Testy statyczne skryptu `.iss` i integracji updatera
   - Testy kompilacji i natywnej instalacji/deinstalacji E2E

6. **COMMIT 6:**
   `docs(installer): close Windows User Setup integration gate (M3-D)`
   - Raport końcowy `STAGE_M3_D_WINDOWS_USER_SETUP_INTEGRATION_REPORT.md`
   - Aktualizacja `README.md`, `MASTER_IMPLEMENTATION_PLAN.md`, `installer/windows/inno/README.md`, rejestrów audytowych

Po każdym commicie następuje pełna walidacja (`cargo check`, `cargo clippy`, testy strażników) oraz synchronizacja `fast-forward` z gałęzią `main` i `origin/main`.

---

Raport sporządził: Główny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024
Status: ZATWIERDZONY DO IMPLEMENTACJI
