# SPLogbook Windows System Setup (MSI / WiX Toolset 7.0.1)

Dokumentacja techniczna i architektoniczna instalatora **Windows System Setup (MSI)** dla projektu SPLogbook w ramach bramki:
`STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE`.

---

## 1. Status Formalny i Zatwierdzony Toolchain

- **DECYZJA WŁAŚCICIELA:** `OWNER-M3-E-TOOLCHAIN-01`
- **STATUS DECYZJI:** `APPROVED` (2026-11-08)
- **ZATWIERDZONY KOMPILATOR / SDK:** `WiX Toolset SDK 7.0.1` (`WixToolset.Sdk/7.0.1`)
- **SILNIK KOMPILACJI:** MSBuild 2022 wyszukiwany dynamicznie przez `vswhere.exe` (`-latest -products * -requires Microsoft.Component.MSBuild`) z fallbackiem do standardowych ścieżek Visual Studio 2022 (`Enterprise`, `Professional`, `Community`, `BuildTools`) oraz wbudowanym `NuGetSdkResolver` i silnikiem `wix.exe` (7.0.1)
- **AKCEPTACJA EULA:** Jawna akceptacja EULA WiX 7 (`wix eula accept wix7`, tworząca plik `%USERPROFILE%\.wix\wix7-osmf-eula.txt`); na świeżych runnerach CI skrypt wykonuje ją celem MSBuild `-t:AcceptEula -p:EulaId=wix7` (sam `-p:AcceptEula=true` nie wystarcza — `wix.exe` zgłasza WIX7015)
- **WYMÓG OSTRZEŻEŃ:** 0 błędów i 0 nieprzejrzanych ostrzeżeń (ICE Errors = 0, ICE Warnings = 0).

---

## 2. Kanoniczna Tożsamość i Parametry Instalatora

- **KANONICZNY UPGRADE CODE:** `{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}`
- **INNO SETUP APP ID (DLA PORÓWNANIA):** `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}`
- **UPGRADE CODE DIFFERS FROM INNO APP ID:** `TAK` (bezwzględna rozłączność tożsamości)
- **PRODUCT CODE:** Dynamiczny identyfikator GUID generowany dla każdego wydania SemVer (`*`) w celu zapewnienia pełnego wsparcia `MajorUpgrade`
- **PACKAGE CODE:** Unikalny identyfikator GUID generowany dla każdego pakietu
- **TYP INSTALACJI:** `WindowsMsi`
- **TRYB ZARZĄDZANIA:** `SystemManaged`
- **ZAKRES INSTALACJI:** Per-machine (`Scope="perMachine"`, `ALLUSERS=1`)
- **POZIOM UPRAWNIEŃ:** Kontrolowany przez Windows Installer model UAC / elevation
- **KATALOG DOMYŚLNY:** `ProgramFiles64Folder\SPLogbook` (`C:\Program Files\SPLogbook`)
- **ARCHITEKTURA:** `x64`
- **WERSJE:**
  - `MSI ProductVersion`: `1.3.0` (format `major.minor.build` wymagany przez specyfikację Windows Installer)
  - `DisplayVersion (ARP)`: `1.3.0-alpha.1` (pełny SemVer 2.0 z identyfikatorem prerelease)
- **SKRÓTY:**
  - Menu Start: `ProgramMenuFolder\SPLogbook`
  - Rejestr KeyPath: `HKCU\Software\SPLogbook\MsiInstalled` (zgodność z ICE38/ICE43/ICE57)

---

## 3. Allowlista Plików Wejściowych (Input Allowlist)

Pakiet instalatora MSI zawiera wyłącznie ściśle zatwierdzone pliki:
1. **Binaria programu:**
   - `SPLogbook.exe` — główna binarka aplikacji (Rust 2024, x86_64 MSVC)
   - `splogbook-updater.exe` — dedykowany pomocnik aktualizatora
2. **Kanoniczny marker instalacji:**
   - `.splogbook-msi-marker` — metadane instalatora w formacie JSON
3. **Zasoby graficzne:**
   - `assets/icon.ico`
   - `assets/icon.png`
   - `assets/icon.rgba`
4. **Pliki lokalizacyjne:**
   - `locales/*.json` (pl, en, de, fr, es, it, ru)
   - `locales/manual/*.json` (jeśli obecne)

> **Brak preinstalowanych baz referencyjnych (`R1C`/`F-09`):** pakiet MSI nie zawiera już `databases/callbook.db`, `databases/serviceLOG.db` ani `databases/clubs.csv`. Dane użytkownika i referencje żyją wyłącznie w `%APPDATA%\SPLogbook`.

### Bezwzględny zakaz pakowania (Forbidden Artifacts):
- Repozytorium `.git`, `.github`
- Katalog `target`
- Raporty markdown, testy, logi, artefakty CI
- Klucze prywatne Ed25519, certyfikaty Authenticode
- Produkcyjne bazy danych użytkownika (`default_log.db`, pliki `station_config.json`, QSO)
- Pliki tymczasowe, pliki `.env`, symbole PDB

---

## 4. Ochrona Danych Użytkownika i Bezpieczeństwo Cyklu Życia

1. **Ochrona danych użytkownika (`%APPDATA%\SPLogbook`):**
   Katalog danych użytkownika oraz pliki baz roboczych (`default_log.db`, `station_config.json`, WAL, SHM, journal, ADIF, Cabrillo) NIE NALEŻĄ do żadnego komponentu MSI. Podczas instalacji, major upgrade, repair oraz deinstalacji Windows Installer pod żadnym pozorem nie dotyka ani nie usuwa katalogu `%APPDATA%`.
2. **Deinstalacja programu:**
   - Usuwane są wyłącznie pliki zarejestrowane w tabeli `File` komponentów MSI.
   - Usunięcie markera instalacji `.splogbook-msi-marker`.
   - Pliki nieznane (`unknown files`) utworzone w katalogu instalacji pozostają nienaruszone.
   - Brak wildcard deletion (`*.*`).
3. **Funkcja naprawy (Repair - `msiexec /fa`):**
   - Naprawia wyłącznie brakujące lub uszkodzone pliki programu należące do komponentów MSI.
   - Nie dotyka danych użytkownika ani plików w `%APPDATA%\SPLogbook`.

---

## 5. Wykrywanie Konfliktów Instalacyjnych

1. **Konflikt z Inno Setup (User Setup):**
   Reguła `<Launch Condition="NOT INNO_USER_SETUP_DETECTED" ... />` weryfikuje obecność wpisu deinstalatora per-user w `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}_is1`. Wykrycie instalacji per-user skutkuje natychmiastowym przerwaniem instalacji (`fail-closed`).
2. **Konflikt z Portable ZIP:**
   MSI instaluje się wyłącznie do `C:\Program Files\SPLogbook`, nie przejmuje i nie nadpisuje katalogu Portable ZIP (`non-destructive`).
3. **Próba downgrade:**
   Reguła `<MajorUpgrade DowngradeErrorMessage="..." Schedule="afterInstallInitialize" />` blokuje instalację starszej wersji na nowszą.

---

## 6. Procedura Budowy Lokalnego Instalatora

Do budowy służy skrypt:
`scripts/build_windows_msi.ps1`

Wymagania:
- System operacyjny: Windows 10/11 x86_64
- MSBuild 2022 (Visual Studio 2022 Enterprise / Professional / Community / BuildTools)
- WiX Toolset SDK 7.0.1 (automatycznie rozwiązywany przez wbudowany resolver NuGet MSBuild)
- Akceptacja EULA WiX 7 (`wix eula accept wix7` lub, w CI, `msbuild -t:AcceptEula -p:EulaId=wix7`)

---

## 7. Status Podpisu i Publikacji (`F-05 = CLOSED`, `F-22 = CLOSED`)

- **STATUS PODPISU:** `UNSIGNED TEST ARTIFACT` (brak certyfikatu Authenticode)
- **STATUS PUBLIKACJI CI:** `PUBLISHED_IN_RELEASE_PIPELINE` — potok `.github/workflows/build-and-release.yml` buduje `SPLogbook-Windows-x64-MSI-<version>.msi` (+ `.sha256`) przez `scripts/build_windows_msi.ps1 -OutputDir dist -SkipCargoBuild`, uruchamia weryfikację E2E (`SPLOG_RUN_MSI_E2E=1`) i publikuje pakiet `msi` w `SHA256SUMS.txt`, podpisanym `release-manifest.json` oraz GitHub Release.
- **TRANSAKCYJNOŚĆ UPDATERA:** `F-22 = CLOSED` — delegowana aktualizacja `WindowsMsi` utrzymuje `.update.lock`, trwały journal `.update_transaction.json` (z fallbackiem do `%LOCALAPPDATA%\SPLogbook\updater_state` / `%APPDATA%\SPLogbook\updater_state` przy braku uprawnień zapisu w `Program Files`) oraz walidację `HealthProof` (również po restarcie procesu w `consume_update_result_on_startup`).
