# SPLogbook Windows System Setup (MSI / WiX Toolset 7.0.1)

Dokumentacja techniczna i architektoniczna instalatora **Windows System Setup (MSI)** dla projektu SPLogbook w ramach bramki:
`STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE`.

---

## 1. Status Formalny i Zatwierdzony Toolchain

- **DECYZJA WŁAŚCICIELA:** `OWNER-M3-E-TOOLCHAIN-01`
- **STATUS DECYZJI:** `APPROVED` (2026-11-08)
- **ZATWIERDZONY KOMPILATOR / SDK:** `WiX Toolset SDK 7.0.1` (`WixToolset.Sdk/7.0.1`)
- **SILNIK KOMPILACJI:** MSBuild 2022 (`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe`) z wbudowanym `NuGetSdkResolver` oraz silnikiem `wix.exe` (7.0.1)
- **AKCEPTACJA EULA:** Jawna akceptacja EULA WiX 7 (`wix eula accept wix7`, tworząca plik `%USERPROFILE%\.wix\wix7-osmf-eula.txt`)
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
5. **Bazy referencyjne (polityka FirstInstallSeed):**
   - `databases/callbook.db` (`NeverOverwrite="yes"`)
   - `databases/serviceLOG.db` (`NeverOverwrite="yes"`)
   - `databases/clubs.csv` (`NeverOverwrite="yes"`)

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
   - Nie przywraca i nie nadpisuje baz referencyjnych (`NeverOverwrite="yes"`).

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
- MSBuild 2022 w Visual Studio BuildTools
- WiX Toolset SDK 7.0.1 (automatycznie rozwiązywany przez wbudowany resolver NuGet MSBuild)
- Akceptacja EULA WiX 7 (`wix eula accept wix7`)
