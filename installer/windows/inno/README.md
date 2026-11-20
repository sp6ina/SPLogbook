# SPLogbook Windows User Setup (Inno Setup 7.1.1)

Dokumentacja techniczna i architektoniczna instalatora **Windows User Setup** dla projektu SPLogbook (`ADR-10`).

---

## 1. Status Formalny i Zatwierdzony Toolchain

- **REKORD ARCHITEKTONICZNY:** `ADR-10`
- **STATUS:** `APPROVED` (2026-11-08)
- **ZATWIERDZONY KOMPILATOR:** `Inno Setup 7.1.1`
- **ŚCIEŻKA KOMPILATORA:** `C:\Program Files\Inno Setup 7\ISCC.exe`
- **SHA-256 ISCC.EXE:** `E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644`
- **WYMÓG OSTRZEŻEŃ:** 0 ostrzeżeń kompilatora (każdy warning traktowany jest jako błąd blokujący).

---

## 2. Kanoniczna Tożsamość i Parametry Instalatora

- **KANONICZNY APP ID:** `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}`
- **TYP INSTALACJI:** `WindowsInnoSetup`
- **TRYB ZARZĄDZANIA:** `SystemManaged`
- **ZAKRES INSTALACJI:** Per-user (`CurrentUser`)
- **POZIOM UPRAWNIEŃ:** `PrivilegesRequired=lowest` (bezwzględny brak wymogu uprawnień administratora, brak UAC, brak `runas`)
- **KATALOG DOMYŚLNY:** `{localappdata}\Programs\SPLogbook` (zakaz instalacji w `C:\Program Files`)
- **ARCHITEKTURA:**
  - `SetupArchitecture=x64`
  - `ArchitecturesAllowed=x64compatible`
  - `ArchitecturesInstallIn64BitMode=x64compatible`
- **REJESTR WINDOWS:**
  - Zapis wyłącznie do `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}_is1`
  - Bezwzględny zakaz modyfikacji kluczy w gałęzi `HKEY_LOCAL_MACHINE` (HKLM)
- **SKRÓTY:**
  - Menu Start użytkownika: `{userprograms}\SPLogbook`
  - Opcjonalny skrót na pulpicie użytkownika: `{userdesktop}\SPLogbook` (sterowany taskiem `desktopicon`)
  - Brak skrótów systemowych (`allusers`)

---

## 3. Allowlista Plików Wejściowych (Input Allowlist)

Pakiet instalatora zawiera wyłącznie ściśle zatwierdzone pliki:
1. **Binaria programu:**
   - `SPLogbook.exe` — główna binarka aplikacji (Rust 2024, x86_64 MSVC)
   - `splogbook-updater.exe` — dedykowany pomocnik aktualizatora
2. **Zasoby graficzne:**
   - `assets/icon.ico`
   - `assets/icon.png`
   - `assets/icon.rgba`
3. **Pliki lokalizacyjne:**
   - `locales/*.json`
   - `locales/manual/*.json` (jeśli obecne)

> **Brak preinstalowanych baz referencyjnych (`R1C`/`F-09`):** instalator nie pakuje już plików `databases/callbook.db`, `databases/serviceLOG.db` ani `databases/clubs.csv`. Dane użytkownika i opcjonalne tabele referencyjne żyją w kanonicznym katalogu `%APPDATA%\SPLogbook`.

### Bezwzględny zakaz pakowania (Forbidden Artifacts):
- Repozytorium `.git`, `.github`
- Cały katalog `target`
- Raporty markdown, testy, logi, artefakty CI
- Tokeny, dane uwierzytelniające, klucze prywatne Ed25519, certyfikaty Authenticode
- Produkcyjne bazy danych użytkownika (`default_log.db`, pliki `station_config.json`, QSO)
- Pliki tymczasowe, pliki `.env`, symbole PDB

---

## 4. Ochrona Danych Użytkownika i Bezpieczeństwo Deinstalacji

1. **Czysta pierwsza instalacja (`R1C`/`R1D`):**
   Instalator nie pakuje żadnych baz danych ani plików konfiguracyjnych. Istniejące pliki użytkownika (dzienniki QSO, konfiguracja, dane referencyjne) w `%APPDATA%\SPLogbook` oraz w katalogu instalacji **nigdy nie są nadpisywane ani usuwane**.
2. **Deinstalacja programu:**
   - Usuwane są wyłącznie pliki zarejestrowane w sekcji `[Files]` oraz skróty z sekcji `[Icons]`.
   - Usunięcie markera instalacji: `Type: files; Name: "{app}\.splogbook-inno-marker"`.
   - Czyszczenie katalogów: wyłącznie `Type: dirifempty` — jeśli w katalogu aplikacji znajdują się pliki użytkownika (bazy QSO, WAL, SHM, konfiguracja, eksporty ADIF/Cabrillo, nieznane pliki), katalog pozostaje nienaruszony.
   - **Zakazy bezwzględne:** zakaz stosowania wildcardów (`*.*`) w `[UninstallDelete]`, zakaz wywoływania procedury `DelTree({app})`, całkowity zakaz usuwania katalogów `%APPDATA%\SPLogbook` i `%LOCALAPPDATA%\SPLogbook`.

---

## 5. Wykrywanie Konfliktów Instalacyjnych

1. **Konflikt z Portable ZIP:**
   Instalator weryfikuje obecność pliku `.splogbook-portable`. W razie wykrycia próby instalacji w katalogu Portable ZIP, instalator odmawia kontynuacji i wymaga wskazania innego katalogu.
2. **Konflikt z MSI (WiX):**
   Weryfikacja kluczy rejestru `UpgradeCode` MSI oraz wpisów deinstalatora MSI. Wykrycie instalacji MSI skutkuje natychmiastowym przerwaniem instalacji (`fail-closed`).
3. **Konflikt z uruchomionym procesem (`F-13 = CLOSED`):**
   Zarządzany przez `AppMutex={#AppMutexName},SPLogbook` (`#define AppMutexName "SPLogbook_Instance_Mutex_{{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}"`) oraz `CloseApplications=yes`. Główny proces `SPLogbook.exe` utrzymuje oba nazwane muteksy Win32 przez `win32_instance_mutex::InstanceMutexGuard` (po wczesnych flagach CLI `--version`, `--build-info`, `--check-health-startup`). W trybie cichym (`/SILENT`) instalator kończy działanie błędem (`fail-closed`), nie modyfikując plików. Zakaz agresywnego zabijania procesu (`taskkill`/`TerminateProcess po nazwie`).
4. **Próba downgrade (`F-12 = CLOSED`):**
   Porównanie wersji w rejestrze HKCU za pomocą pełnego numerycznego parsera SemVer 2.0.0 (`ExtractNextToken`, `IsAllDigits`, `CompareNumericStrings`, `ComparePreRelease`, `CompareSemVer`). Próba zainstalowania starszej wersji na nowszą jest blokowana.

---

## 6. Procedura Budowy Lokalnego Instalatora

Do budowy służy skrypt:
`scripts/build_windows_user_setup.ps1`

Wymagania:
- System operacyjny: Windows 10/11 x86_64
- Zainstalowany Inno Setup 7.1.1 pod zatwierdzoną ścieżką `C:\Program Files\Inno Setup 7\ISCC.exe`
- Rust 2024 toolchain (stable)

Uruchomienie:
```powershell
powershell -ExecutionPolicy Bypass -File scripts\build_windows_user_setup.ps1
```

Skrypt automatycznie:
1. Weryfikuje wersję w `Cargo.toml` (`1.3.0`).
2. Sprawdza czystość repozytorium Git i pobiera hash commita.
3. Sprawdza obecność i wersję `ISCC.exe` (7.1.1).
4. Kompiluje binaria w profilu release (`SPLogbook.exe`, `splogbook-updater.exe`).
5. Tworzy izolowany katalog stagingu i kopiuje wyłącznie pliki z allowlisty.
6. Weryfikuje brak zakazanych plików w stagingu.
7. Kompiluje instalator za pomocą zatwierdzonego ISCC z flagą `--no-signing`.
8. Oblicza SHA-256 wynikowego instalatora `SPLogbook-Windows-x64-Setup-1.3.0.exe`.

---

## 7. Status Podpisu i Publikacji (`F-05 = CLOSED`, `F-22 = CLOSED`)

- **STATUS PODPISU:** `UNSIGNED TEST ARTIFACT` (brak certyfikatu Authenticode)
- **STATUS PUBLIKACJI CI:** `PUBLISHED_IN_RELEASE_PIPELINE` — potok `.github/workflows/build-and-release.yml` pobiera i weryfikuje SHA-256 przypiętego Inno Setup 7.1.1, buduje `SPLogbook-Windows-x64-Setup-<version>.exe` (+ `.sha256`), wykonuje test E2E (`SPLOG_RUN_INNO_E2E=1`) i dołącza pakiet `inno-exe` do `SHA256SUMS.txt`, podpisanego `release-manifest.json` oraz GitHub Release.
- **TRANSAKCYJNOŚĆ UPDATERA:** `F-22 = CLOSED` — delegowana aktualizacja przez `WindowsInnoSetup` używa blokady `.update.lock`, trwałego journala `.update_transaction.json` oraz walidacji `HealthProof` (w tym po restarcie procesu w `consume_update_result_on_startup`).
