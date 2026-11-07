# PROJEKT ARCHITEKTONICZNY: AKTUALIZATOR WINDOWS (PORTABLE ZIP)
# Dokument: DESIGN_3_0_A_WINDOWS_UPDATE.md

---

## 1. Status Dokumentu
- **Identyfikator:** `UPDATE-DESIGN-WIN`
- **Etap:** `STAGE-M2-INSTALLER-DESIGN-GATE`
- **Status dokumentu:** `REVIEWED` (Zgodnie z zasadami Sekcji 46; oczekuje na formalne zatwierdzenie otwartych decyzji wĹ‚aĹ›ciciela przez WĹ‚aĹ›ciciela repozytorium)
- **GotowoĹ›Ä‡ wykonawcza M3:** `BLOCKED_BY_OWNER_DECISIONS` (Sekcja 39 i 54)
- **Data opracowania:** 2026-11-07 (Rzeczywista data wykonania M2)
- **Strefa czasowa:** `+01:00` (CET / Europe/Warsaw)
- **Autor / Rola:** GĹ‚Ăłwny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Bezpiecznych AktualizatorĂłw

---

## 2. Wersja i Historia Zmian
- **Wersja:** `2.0.0`
- **Historia:**
  - `v1.0.0` (2026-09-29): Wczesny szkic koncepcyjny zablokowania bĹ‚Ä™du traktowania pliku ZIP jako EXE (historyczny commit w fazie pre-M1).
  - `v2.0.0` (2026-11-07): Kompletny projekt architektoniczny aktualizatora Windows Portable ZIP, integrujÄ…cy zweryfikowane mechanizmy manifest-first (Ed25519), silnik `UpdatePlan`, fail-closed ochronÄ™ baz SQLite i danych uĹĽytkownika, izolowany staging, model helpera, deterministyczny rollback oraz procedury odzyskiwania po awarii.

---

## 3. Baseline Git i Kontekst Techniczny
- **Baseline Git HEAD:** `5ddec14bd40b70ab099ba289ecdd952340c6da54`
- **GaĹ‚Ä…Ĺş referencyjna:** `main` / `repair/full-system-remediation`
- **Wersja oprogramowania:** SPLogbook `v1.2.0` (kod ĹşrĂłdĹ‚owy Rust 2024, MSRV 1.85+)
- **Relacja do Etapu M1:** Etap M1 zostaĹ‚ w peĹ‚ni zrealizowany i zweryfikowany testami oprogramowania (`STAGE_M1_COVERAGE_AND_STATUS_REPORT.md`, 36/36 kryteriĂłw `WIN-M1` speĹ‚nionych). Niniejszy dokument stanowi specyfikacjÄ™ docelowÄ… fazy M2 bez modyfikacji kodu.

---

## 4. Zakres Projektu (In-Scope)
Projekt obejmuje:
1. Oficjalnie wspierany model dystrybucji Windows: aplikacja przenoĹ›na (Portable) dostarczana w pĹ‚askim archiwum ZIP (`SPLogbook-Windows-x64.zip`).
2. Wykrywanie typu instalacji oraz walidacjÄ™ uprawnieĹ„ zapisu (Write Access Preflight).
3. ĹšcisĹ‚Ä… integracjÄ™ z kryptograficznym kontraktem zaufania manifest-first (`release-manifest.json` + podpis Ed25519).
4. InspekcjÄ™ struktury pakietu ZIP przed dekompresjÄ… (limity dekompresyjne, ochrona przed Zip-Slip, zakaz symlinkĂłw).
5. BezpiecznÄ… dekompresjÄ™ do izolowanego katalogu stagingu (`.staging`).
6. Generowanie i rygorystycznÄ… walidacjÄ™ planu aktualizacji (`UpdatePlan`) z zachowaniem zasady `UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE`.
7. OchronÄ™ danych uĹĽytkownika (`default_log.db`, logi Ĺ‚Ä…cznoĹ›ci, pliki ADIF/Cabrillo), konfiguracji (`station_config.json`) oraz baz referencyjnych (`databases/serviceLOG.db`, `databases/callbook.db`).
8. ArchitekturÄ™ procesu pomocniczego (Updater Helper) radzÄ…cego sobie z blokadÄ… wykonywalnego pliku PE przez system Windows (File Locking).
9. ProcedurÄ™ transakcyjnej podmiany plikĂłw z tworzeniem izolowanej kopii zapasowej (`.rollback`).
10. Uruchomienie kandydata aktualizacji z flagÄ… `--check-health-startup` oraz weryfikacjÄ™ zdrowia (Health Confirmation).
11. Automatyczny, deterministyczny i idempotentny mechanizm wycofania zmian (Rollback).
12. Procedury czyszczenia (Cleanup) oraz usuwania pozostaĹ‚oĹ›ci po przerwanych lub nieudanych prĂłbach (Crash Recovery).
13. Zabezpieczenia wspĂłĹ‚bieĹĽnoĹ›ci (Update Lock) oraz zasady bezpiecznego anulowania.
14. Macierz zagroĹĽeĹ„ bezpieczeĹ„stwa i macierz testĂłw akceptacyjnych dla Etapu M3.

---

## 5. Poza Zakresem Projektu (Out-of-Scope)
Projekt jawnie wyklucza i pozostawia jako przyszĹ‚e, odrÄ™bne inicjatywy:
1. ImplementacjÄ™ instalatora Microsoft Installer (MSI), WiX Toolset, Inno Setup czy NSIS.
2. AutomatycznÄ… eskalacjÄ™ uprawnieĹ„ przez User Account Control (UAC) / `runas`. W modelu Portable prĂłba aktualizacji w katalogu chronionym (np. `C:\Program Files`) koĹ„czy siÄ™ jawnÄ… odmowÄ…, a nie podnoszeniem uprawnieĹ„.
3. ModyfikacjÄ™ schematu manifestu wydaĹ„, wprowadzanie nowych pĂłl JSON bez zatwierdzenia.
4. ZmianÄ™ aktywnych kluczy zaufania oraz modyfikacjÄ™ polityki odwoĹ‚ania klucza `splogbook-release-2026-01`.
5. Nowe migracje schematĂłw baz danych SQLite.
6. DystrybucjÄ™ i aktualizacjÄ™ instalacji wielodostÄ™pnych (multi-user / service-level).

---

## 6. Obecny Format Wydania Windows
Zgodnie z potokiem `.github/workflows/build-and-release.yml` oraz audytem `UPDATE_ARTIFACT_INVENTORY_REPORT.md`:
- **Nazwa assetu:** `SPLogbook-Windows-x64.zip`
- **Format kontenera:** ZIP (DEFLATE / Store)
- **Docelowa platforma:** `windows` (`ReleasePlatform::Windows`)
- **Architektura:** `x86_64` (`ReleaseArchitecture::X86_64`)
- **Model archiwum:** `FLAT ARCHIVE` (pliki binarne, biblioteki i podkatalog `databases/` umieszczone bezpoĹ›rednio w korzeniu archiwum ZIP, bez wspĂłlnego katalogu nadrzÄ™dnego).
- **GĹ‚Ăłwny plik wykonywalny:** `SPLogbook.exe` (PE32+ executable, x86-64, subsystem Windows GUI).
- **ZaleĹĽnoĹ›ci wykonawcze:** Wbudowane biblioteki DLL kompilowane statycznie lub dystrybuowane w katalogu programu.
- **ZawartoĹ›Ä‡ podkatalogu `databases/`:** PoczÄ…tkowe ziarna referencyjne `serviceLOG.db` (baza prefiksĂłw, okrÄ™gĂłw i IOTA) oraz `callbook.db` (lokalny indeks znakĂłw).

---

## 7. Modele Instalacji (Installation Types)
Aktualizator definiuje Ĺ›cisĹ‚y typ wyliczeniowy rodzajĂłw instalacji Windows:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsInstallationType {
    /// Instalacja przenoĹ›na w katalogu z peĹ‚nymi prawami zapisu dla bieĹĽÄ…cego uĹĽytkownika
    /// (np. C:\SPLogbook, D:\HamRadio\SPLogbook, %USERPROFILE%\PortableApps\SPLogbook).
    PortableUserWritable,
    /// Instalacja w lokalizacji zablokowanej do zapisu dla bieĹĽÄ…cego procesu (np. brak uprawnieĹ„ NTFS).
    PortableReadOnly,
    /// Instalacja w chronionym katalogu systemowym (np. C:\Program Files, C:\Program Files (x86), C:\Windows).
    SystemProtectedDirectory,
    /// ĹšcieĹĽka sieciowa UNC (np. \\server\share\SPLogbook) â€” nieobsĹ‚ugiwana ze wzglÄ™dĂłw bezpieczeĹ„stwa.
    NetworkShareUnc,
    /// Stan nieznany lub bĹ‚Ä…d walidacji Ĺ›cieĹĽki.
    Unknown,
}
```

### ReguĹ‚y detekcji i zachowania:
1. **Weryfikacja Ĺ›cieĹĽki UNC:** JeĹ›li Ĺ›cieĹĽka pliku wykonywalnego zaczyna siÄ™ od `\\` lub prefiksu `\\?\UNC\`, instalacja jest klasyfikowana jako `NetworkShareUnc`. Aktualizacja zostaje natychmiast przerwana (fail-closed) z komunikatem ostrzegawczym.
2. **Weryfikacja katalogĂłw systemowych:** ĹšcieĹĽka bazowa jest porĂłwnywana z lokalizacjami `%ProgramFiles%`, `%ProgramFiles(x86)%` oraz `%SystemRoot%`. W przypadku wykrycia instalacji w tych obszarach:
   - Klasyfikacja: `SystemProtectedDirectory`.
   - Zachowanie: Aktualizator kategorycznie odmawia automatycznej podmiany w modelu portable. WyĹ›wietla monit: *"SPLogbook znajduje siÄ™ w katalogu chronionym systemowo. Automatyczna aktualizacja portable jest zablokowana. Pobierz i rozpakuj nowÄ… wersjÄ™ rÄ™cznie lub przenieĹ› program do katalogu uĹĽytkownika."*
3. **Preflight zapisu (Write Probe):** W katalogu programu podejmowana jest prĂłba utworzenia, zapisu i usuniÄ™cia pliku testowego `.update_probe_<uuid>.tmp`.
   - Sukces: `PortableUserWritable`.
   - BĹ‚Ä…d `PermissionDenied`: `PortableReadOnly` â€” aktualizacja zostaje zablokowana przed rozpoczÄ™ciem pobierania.

---

## 8. Kontrakt Zaufania (Trust Flow)
Aktualizator Windows bezwzglÄ™dnie egzekwuje model `manifest-first`:

```mermaid
flowchart TD
    Start["Ĺ»Ä…danie aktualizacji"] --> FetchM["Pobranie release-manifest.json oraz .sig"]
    FetchM --> SigVerify["Weryfikacja podpisu Ed25519 wzglÄ™dem TRUSTED_KEYS"]
    SigVerify -->|Podpis niepoprawny / brak| FailTrust["Odrzucenie: InvalidSignature / MissingSignature"]
    SigVerify --> KeyCheck["Weryfikacja identyfikatora i skrĂłtu klucza"]
    KeyCheck -->|Klucz na liĹ›cie REVOKED_KEY_IDS| FailRevoked["Odrzucenie: RevokedKeyRejected (fail-closed)"]
    KeyCheck -->|Klucz nieznany| FailUnknown["Odrzucenie: UntrustedKey"]
    KeyCheck --> ManifestValid["Walidacja semantyczna manifestu (SemVer, downgrade)"]
    ManifestValid --> SelectAsset["Selekcja assetu: Windows + x86_64 + zip"]
    SelectAsset --> VAsset["Utworzenie VerifiedReleaseAsset"]
    VAsset --> Download["Pobranie do .staging z kontrolÄ… rozmiaru"]
    Download --> HashCheck["Weryfikacja SHA-256 pobranego pliku"]
    HashCheck -->|Niezgodny| FailHash["Odrzucenie: HashMismatch (usuniÄ™cie pliku)"]
    HashCheck --> StagingPhase["PrzejĹ›cie do inspekcji ZIP i Stagingu"]
```

### Nienaruszalne reguĹ‚y zaufania:
- Klucz `splogbook-release-2026-01` pozostaje trwale i nieodwracalnie odwoĹ‚any (`REVOKED_KEY_IDS`). PrĂłba uĹĽycia tego klucza natychmiast przerywa proces.
- Wydanie `v1.2.0`, ktĂłrego manifest podpisano odwoĹ‚anym kluczem, jest weryfikowane jako niegodne zaufania.
- Pobranie pliku ZIP jest dopuszczalne **wyĹ‚Ä…cznie** po pomyĹ›lnej weryfikacji podpisanego manifestu. Brak manifestu lub bĹ‚Ä…d podpisu powoduje natychmiastowe zaniechanie poĹ‚Ä…czeĹ„ do pliku ZIP.

---

## 9. Selekcja Assetu Wydania (Asset Selection)
Asset instalacyjny Windows musi speĹ‚niaÄ‡ Ĺ›cisĹ‚Ä… triadÄ™ identyfikacyjnÄ…:
1. `platform == "windows"` (`ReleasePlatform::Windows`)
2. `arch == "x86_64"` (`ReleaseArchitecture::X86_64`)
3. `package_type == "zip"` (`ReleasePackageType::Zip`)
4. `filename` musi koĹ„czyÄ‡ siÄ™ na `.zip` (niewraĹĽliwe na wielkoĹ›Ä‡ liter) i odpowiadaÄ‡ wzorcowi `SPLogbook-Windows-x64.zip`.
5. W manifeĹ›cie musi znajdowaÄ‡ siÄ™ **dokĹ‚adnie jeden** asset speĹ‚niajÄ…cy powyĹĽsze kryteria. WystÄ…pienie zduplikowanych assetĂłw dla Windows skutkuje bĹ‚Ä™dem `UpdaterError::AmbiguousArtifact`.

---

## 10. Inspekcja Archiwum ZIP (Package Inspection)
Przed przystÄ…pieniem do jakiejkolwiek dekompresji, pobrany plik ZIP podlega rygorystycznej inspekcji nagĹ‚ĂłwkĂłw w strumieniu pamiÄ™ciowym lub plikowym:
- **Maksymalna liczba wpisĂłw w archiwum:** 10 000 wpisĂłw (`ArchiveLimits::MAX_ENTRIES`).
- **Maksymalny Ĺ‚Ä…czny rozmiar po rozpakowaniu:** 200 MB (`ArchiveLimits::MAX_TOTAL_BYTES`).
- **Maksymalny rozmiar pojedynczego pliku:** 100 MB (`ArchiveLimits::MAX_ENTRY_BYTES`).
- **WspĂłĹ‚czynnik kompresji (Decompression Bomb Protection):** Odrzucenie wpisĂłw o nienaturalnym stosunku rozmiaru skompresowanego do rozpakowanego (> 100:1 dla plikĂłw powyĹĽej 1 MB).
- **Blokada dowiÄ…zaĹ„ symbolicznych (Symlinki):** Wszelkie wpisy bÄ™dÄ…ce dowiÄ…zaniami symbolicznymi lub punktami reparse (NTFS Junctions) sÄ… bezwzglÄ™dnie odrzucane (`ArchiveLimits::allow_symlinks = false`).
- **Wykrywanie kolizji nazw i wielkoĹ›ci liter (Case-Collisions):** Normalizacja nazw do postaci maĹ‚oliterowej w strukturze indeksu â€” wykrycie prĂłb zduplikowania Ĺ›cieĹĽek rĂłĹĽniÄ…cych siÄ™ wyĹ‚Ä…cznie wielkoĹ›ciÄ… znakĂłw powoduje natychmiastowe przerwanie z bĹ‚Ä™dem `UpdaterError::UnsafeArchivePath`.

---

## 11. Bezpieczna Dekompresja ZIP (ZIP Extraction)
Dekompresja realizowana jest wyĹ‚Ä…cznie przez dedykowanÄ…, bezpiecznÄ… funkcjÄ™ `extract_zip_safe_windows`:
1. **Ochrona przed Zip-Slip:** KaĹĽda Ĺ›cieĹĽka wpisu w archiwum jest walidowana przez `validate_archive_relative_path`.
   - BezwzglÄ™dny zakaz wystÄ™powania separatorĂłw nadrzÄ™dnych (`..`).
   - BezwzglÄ™dny zakaz Ĺ›cieĹĽek zaczynajÄ…cych siÄ™ od separatora dysku lub katalogu gĹ‚Ăłwnego (`/`, `\`, `C:`).
   - BezwzglÄ™dny zakaz nazw zarezerwowanych DOS/Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`).
2. **Kanonikalizacja docelowa:** Po zbudowaniu peĹ‚nej Ĺ›cieĹĽki wewnÄ…trz folderu stagingu, weryfikowane jest, czy `dest_path.starts_with(&staging_canonical)`. WyjĹ›cie poza katalog stagingu powoduje rzucenie bĹ‚Ä™du `UpdaterError::ZipSlip`.
3. **Prawa dostÄ™pu:** Utworzone pliki otrzymujÄ… standardowe uprawnienia uĹĽytkownika; atrybuty `ReadOnly` lub `Hidden` z archiwum sÄ… czyszczone.

---

## 12. Izolowany Staging (Staging Architecture)
- Wszystkie pliki rozpakowywane sÄ… do tymczasowego podkatalogu `.staging` wewnÄ…trz katalogu instalacji aplikacji (`$baseDir\.staging`).
- **SpĹ‚aszczanie katalogu (Root Flattening):** JeĹĽeli caĹ‚a zawartoĹ›Ä‡ archiwum znajduje siÄ™ w pojedynczym katalogu nadrzÄ™dnym (np. `SPLogbook-Windows-x64/`), funkcja `flatten_single_root_directory` przesuwa zawartoĹ›Ä‡ poziom wyĹĽej. W obecnym formacie Windows (`FLAT ARCHIVE`) pliki trafiajÄ… bezpoĹ›rednio do `.staging`.
- **Walidacja ukĹ‚adu stagingu:** Funkcja `validate_staging_layout` sprawdza, czy w korzeniu `.staging` znajduje siÄ™ binarka `SPLogbook.exe`. Jej brak skutkuje przerwaniem procesu z bĹ‚Ä™dem `UpdaterError::InvalidStagingLayout`.

---

## 13. Budowanie Planu Aktualizacji (UpdatePlan Contract)
Plan aktualizacji jest wyliczany w sposĂłb czysto funkcyjny przez `UpdatePlan::generate(&staging, base_dir)` bez wykonywania operacji I/O modyfikujÄ…cych stan programu.

```rust
pub struct UpdatePlan {
    pub replace_files: Vec<(PathBuf, PathBuf)>, // (staging_rel, active_abs)
    pub create_files: Vec<(PathBuf, PathBuf)>,  // (staging_rel, active_abs)
    pub preserve_files: Vec<(PathBuf, String)>, // (active_abs, reason)
    pub rejected_files: Vec<(PathBuf, String)>, // (staging_rel, reason)
}
```

### Kryteria podziaĹ‚u:
1. `replace_files`: Pliki programu (`SPLogbook.exe`, biblioteki DLL, oficjalne zasoby w `resources/`), ktĂłre juĹĽ istniejÄ… w `$baseDir`.
2. `create_files`: Nowe pliki programu, nowe biblioteki DLL lub nowe ziarna baz danych (`FirstInstallSeed`), jeĹ›li dany plik docelowy **nie istnieje** w aktywnej instalacji.
3. `preserve_files`: Wszystkie istniejÄ…ce pliki uĹĽytkownika, konfiguracja oraz istniejÄ…ce bazy danych referencyjnych/seedĂłw.
4. `rejected_files`: Wszelkie nierozpoznane pliki w paczce aktualizacyjnej (zasada fail-closed).

---

## 14. Klasyfikacja PlikĂłw i Ochrona Danych (File Classification Policy)
Klasyfikator `classify_update_path(&Path)` przypisuje kaĹĽdÄ… Ĺ›cieĹĽkÄ™ do jednej z poniĹĽszych klas:

| Klasa pliku (`UpdateFileClass`) | Wzorzec Ĺ›cieĹĽki / nazwy | Polityka w Stagingu | Czy moĹĽe nadpisaÄ‡ istniejÄ…cy plik? | Polityka Rollbacku |
|---|---|---|---|---|
| **ProgramFile** | `SPLogbook.exe`, `*.dll`, `resources/**`, `assets/**` | `replace_files` lub `create_files` | **TAK** (wyĹ‚Ä…cznie po verified backupie) | Przywracany z kopii w `.rollback` |
| **RuntimeLibrary** | Biblioteki dynamiczne DLL | `replace_files` lub `create_files` | **TAK** (po backupie) | Przywracana z `.rollback` |
| **FirstInstallSeed** | `databases/serviceLOG.db`, `databases/callbook.db` | Dozwolony jako `create_files` tylko przy nowej instalacji | **BEZWZGLÄDNIE NIE** (jeĹ›li plik istnieje lokalnie, trafia do `preserve_files`) | Nienaruszany przez rollback |
| **UserConfiguration** | `station_config.json`, `config/**`, `*.conf`, `*.cfg` | Wykrycie w ZIP trafia do `rejected_files` | **BEZWZGLÄDNIE NIE** (nigdy nie nadpisywaÄ‡) | Nienaruszany przez rollback |
| **UserData** | `default_log.db`, `*.db-wal`, `*.db-shm`, `*.adi`, `logs/**` | Wykrycie w ZIP trafia do `rejected_files` | **BEZWZGLÄDNIE NIE** (nigdy nie nadpisywaÄ‡) | Nienaruszany przez rollback |
| **ReferenceData** | Bazy referencyjne | Chronione domyĹ›lnie | **NIE** (wymaga dedykowanego mechanizmu migracji) | Nienaruszany przez rollback |
| **ReleaseMetadata** | `manifest.json`, `changelog.md`, `license.txt` | Informacyjne | **NIE** (nie kopiowaÄ‡ do korzenia aplikacji) | Pomijany |
| **Unknown** | KaĹĽdy inny plik nierozpoznany | Odrzucany (`rejected_files`) | **BEZWZGLÄDNIE NIE** (fail-closed) | Pomijany |

---

## 15. Architektura Procesu Pomocniczego (Helper Model Decision)
System Windows zakĹ‚ada wyĹ‚Ä…cznÄ… blokadÄ™ zapisu (Exclusive Execute/Write Lock) na uruchomionym pliku `SPLogbook.exe`. Podmiana pliku binarnego wymaga zewnÄ™trznego procesu pomocniczego.

### PorĂłwnanie wariantĂłw:

| Kryterium oceny | Opcja A: PowerShell Helper (Aktualna) | Opcja B: Natywny Helper EXE (`splogbook-updater.exe`) |
|---|---|---|
| **DostÄ™pnoĹ›Ä‡ w OS** | DostÄ™pny domyĹ›lnie w Windows 10 i 11 | Wymaga skompilowania i dystrybucji osobnego pliku PE |
| **ZaleĹĽnoĹ›ci wykonawcze** | Wymaga Ĺ›rodowiska PowerShell 5.1+ | Samodzielny plik binarny Rust (brak zewnÄ™trznych interpreterĂłw) |
| **BezpieczeĹ„stwo parametrĂłw** | Wymaga rygorystycznego cytowania Ĺ›cieĹĽek (`replace('\'', "''")`) | BezpoĹ›rednie przekazywanie argumentĂłw przez API procesĂłw OS |
| **OdpornoĹ›Ä‡ na wstrzykiwanie** | WraĹĽliwy na bĹ‚Ä™dy parsowania, jeĹ›li Ĺ›cieĹĽka zawiera znaki specjalne | 100% odporny na command injection (przekazywanie strukturalne) |
| **Polityka ExecutionPolicy** | Wymaga flagi `-ExecutionPolicy Bypass` | Nie podlega politykom ExecutionPolicy |
| **Ryzyko heurystyki antywirusowej (AMSI)** | PodwyĹĽszone: skrypty PowerShell modyfikujÄ…ce pliki EXE bywajÄ… flagowane | Niskie: standardowy proces pomocniczy podpisany cyfrowo |
| **ObsĹ‚uga bĹ‚Ä™dĂłw i diagnostyka** | Trudniejsza w debugowaniu, ograniczona do strumieni PowerShell | PeĹ‚ne, strukturalne logowanie do pliku diagnostycznego JSON |
| **Rozmiar dystrybucji** | 0 bajtĂłw w archiwum (generowany w locie) | ~1-2 MB dodatkowego rozmiaru w archiwum ZIP |

### Decyzja architektoniczna:
- **Cel bieĹĽÄ…cy M3 (M3 Current Target):** Ulepszony, w peĹ‚ni utwardzony skrypt PowerShell (`generate_powershell_update_script`), eliminujÄ…cy wszelkie dzikie karty, stosujÄ…cy literaĹ‚y Ĺ›cieĹĽek (`-LiteralPath`), izolacjÄ™ backupu i bezpoĹ›rednie tablice plikĂłw z `UpdatePlan`.
- **Cel dĹ‚ugoterminowy (Long-Term Target):** Kompilacja natywnego programu pomocniczego `splogbook-updater.exe` (wymaga formalnej decyzji wĹ‚aĹ›ciciela `OWNER-M2-03`).

---

## 16. ZamkniÄ™cie Starego Procesu (Process Shutdown Coordination)
1. GĹ‚Ăłwny proces SPLogbook przygotowuje Ĺ›rodowisko, zapisuje plan, generuje skrypt pomocniczy w `%TEMP%\SPLogbook_update_<pid>.ps1` i uruchamia go w tle.
2. Helper otrzymuje identyfikator PID procesu macierzystego (`$pidToWait = <pid>`).
3. GĹ‚Ăłwny proces SPLogbook dokonuje czystego zamkniÄ™cia bazy SQLite (checkpoint WAL), zapisuje stan konfiguracji i koĹ„czy pracÄ™ (`std::process::exit(0)`).
4. Helper monitoruje proces macierzysty za pomocÄ… pÄ™tli:
   ```powershell
   $deadline = (Get-Date).AddSeconds(90)
   while (Get-Process -Id $pidToWait -ErrorAction SilentlyContinue) {
       if ((Get-Date) -gt $deadline) {
           # Przekroczono limit czasu oczekiwania na zamkniÄ™cie
           exit 1
       }
       Start-Sleep -Milliseconds 250
   }
   Start-Sleep -Milliseconds 500
   ```
5. JeĹ›li proces nie zamknie siÄ™ w ciÄ…gu 90 sekund, proces aktualizacji zostaje bezpiecznie przerwany bez dotykania plikĂłw programu.

---

## 17. Kopia Zapasowa Przed Zastosowaniem (Backup Contract)
1. Przed wykonaniem jakiejkolwiek modyfikacji w katalogu instalacji, helper tworzy dedykowany podkatalog kopii zapasowej: `$baseDir\.rollback`.
2. Stary katalog `.rollback` (jeĹ›li istniaĹ‚) jest bezwzglÄ™dnie usuwany.
3. Kopia obejmuje **wyĹ‚Ä…cznie** pliki ze zbioru `replace_files` zdefiniowane w `UpdatePlan`:
   ```powershell
   foreach ($rel in $replaceFiles) {
       $orig = Join-Path $baseDir $rel
       if (Test-Path -LiteralPath $orig) {
           $dest = Join-Path $rollback $rel
           $destDir = Split-Path $dest -Parent
           if (-not (Test-Path -LiteralPath $destDir)) {
               New-Item -ItemType Directory -Force -Path $destDir | Out-Null
           }
           Copy-Item -LiteralPath $orig -Destination $dest -Force
       }
   }
   ```
4. **Zasada nadrzÄ™dna:** Bazy danych uĹĽytkownika (`default_log.db`), konfiguracja (`station_config.json`) oraz bazy referencyjne (`serviceLOG.db`) **nie sÄ… kopiowane** do `.rollback`, co gwarantuje, ĹĽe proces rollbacku w ĹĽadnych okolicznoĹ›ciach nie nadpisze nowszych danych uĹĽytkownika.

---

## 18. Zastosowanie Aktualizacji (Apply Phase)
Podmiana plikĂłw odbywa siÄ™ w dwĂłch precyzyjnych krokach bez uĹĽycia jakichkolwiek masek wieloznacznych (`*`):
1. **Podmiana istniejÄ…cych plikĂłw programu (`replaceFiles`):**
   ```powershell
   foreach ($rel in $replaceFiles) {
       $src = Join-Path $staging $rel
       $dst = Join-Path $baseDir $rel
       Copy-Item -LiteralPath $src -Destination $dst -Force
   }
   ```
2. **Utworzenie nowych dopuszczonych plikĂłw (`createFiles`):**
   ```powershell
   foreach ($rel in $createFiles) {
       $src = Join-Path $staging $rel
       $dst = Join-Path $baseDir $rel
       $dstDir = Split-Path $dst -Parent
       if (-not (Test-Path -LiteralPath $dstDir)) {
           New-Item -ItemType Directory -Force -Path $dstDir | Out-Null
       }
       Copy-Item -LiteralPath $src -Destination $dst -Force
   }
   ```
3. Zastosowanie operacji `Copy-Item -LiteralPath` zapobiega interpretacji znakĂłw specjalnych (np. nawiasĂłw kwadratowych w Ĺ›cieĹĽkach).

---

## 19. Uruchomienie Kandydata (Launch Candidate)
1. Przed uruchomieniem kandydata usuwany jest ewentualny stary plik potwierdzenia:
   ```powershell
   if (Test-Path -LiteralPath $healthOk) {
       Remove-Item -Force -LiteralPath $healthOk
   }
   ```
2. Helper uruchamia zaktualizowany proces z flagÄ… diagnostycznÄ…:
   ```powershell
   $newProc = Start-Process -FilePath $exe -ArgumentList "--check-health-startup" -PassThru
   ```
3. Uchwyt `$newProc` pozwala na monitorowanie, czy proces nie zakoĹ„czyĹ‚ siÄ™ przedwczeĹ›nie awariÄ… (crash/segfault).

---

## 20. Potwierdzenie Zdrowia (Health Confirmation Contract)
1. Nowo uruchomiony proces w trybie `--check-health-startup` wykonuje:
   - InicjalizacjÄ™ podstawowych podsystemĂłw Rusta.
   - Odczyt i deserializacjÄ™ pliku konfiguracyjnego `station_config.json`.
   - PrĂłbÄ™ otwarcia bazy danych SQLite w trybie odczytu.
   - WeryfikacjÄ™ obecnoĹ›ci zasobĂłw GUI.
2. Po pomyĹ›lnym teĹ›cie proces zapisuje plik `$baseDir\.health_ok`.
3. Helper oczekuje na pojawienie siÄ™ pliku przez maksymalnie 15 sekund:
   ```powershell
   $healthDeadline = (Get-Date).AddSeconds(15)
   $success = $false
   while ((Get-Date) -lt $healthDeadline) {
       if (Test-Path -LiteralPath $healthOk) {
           $success = $true
           break
       }
       if ($newProc.HasExited) {
           # Kandydat ulegĹ‚ natychmiastowej awarii
           break
       }
       Start-Sleep -Milliseconds 250
   }
   ```
4. JeĹ›li w ciÄ…gu 15 sekund plik `.health_ok` nie powstanie lub proces kandydata zakoĹ„czy siÄ™ kodem bĹ‚Ä™du, aktualizacja jest uznawana za nieudanÄ… i natychmiast przechodzi do fazy Rollbacku.

---

## 21. Wycofanie Zmian (Rollback Contract)
Procedura rollbacku jest w 100% deterministyczna i idempotentna:
```powershell
if (-not $success) {
    # 1. Zatrzymanie wiszÄ…cego lub wadliwego procesu kandydata
    Stop-Process -Id $newProc.Id -Force -ErrorAction SilentlyContinue

    # 2. UsuniÄ™cie wszystkich nowo utworzonych plikĂłw programu
    foreach ($rel in $createFiles) {
        $target = Join-Path $baseDir $rel
        if (Test-Path -LiteralPath $target) {
            Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue
        }
    }

    # 3. PrzywrĂłcenie oryginalnych plikĂłw z katalogu .rollback
    foreach ($rel in $replaceFiles) {
        $bkp = Join-Path $rollback $rel
        if (Test-Path -LiteralPath $bkp) {
            Copy-Item -LiteralPath $bkp -Destination (Join-Path $baseDir $rel) -Force
        }
    }

    # 4. Ponowne uruchomienie poprzedniej, sprawdzonej wersji programu
    Start-Process -FilePath $exe
}
```
### WĹ‚aĹ›ciwoĹ›ci gwarantowane:
- Ĺ»adne pliki z `preserve_files` (bazy, konfiguracja) nie sÄ… dotykane.
- Nowe pliki programu, ktĂłre mogĹ‚yby powodowaÄ‡ konflikty wersji, sÄ… usuwane.
- Poprzednia wersja binarki `SPLogbook.exe` i bibliotek DLL powraca na swoje miejsce.

---

## 22. Czyszczenie (Cleanup Contract)
W przypadku pomyĹ›lnego potwierdzenia zdrowia (`$success == $true`):
1. Helper usuwa katalog stagingu: `Remove-Item -Recurse -Force -LiteralPath $staging`.
2. Helper usuwa katalog kopii zapasowej: `Remove-Item -Recurse -Force -LiteralPath $rollback`.
3. Helper usuwa znacznik zdrowia: `Remove-Item -Force -LiteralPath $healthOk`.
4. Helper usuwa swĂłj wĹ‚asny skrypt z katalogu `%TEMP%`.
5. Helper koĹ„czy dziaĹ‚anie kodem 0.

---

## 23. Odzyskiwanie po Awarii Zasilania / Crashu (Crash Recovery)
JeĹĽeli nastÄ…pi nagĹ‚a utrata zasilania lub zabicie procesu helpera podczas operacji kopiowania:
1. **Wykrywanie przy starcie:** Podczas standardowego startu `SPLogbook.exe` sprawdza obecnoĹ›Ä‡ katalogu `.rollback` oraz brak flagi aktywnego procesu helpera.
2. **ReguĹ‚a rozstrzygania:**
   - JeĹ›li w katalogu `$baseDir` istnieje plik `.rollback` zawierajÄ…cy poprawny plik `SPLogbook.exe`, a gĹ‚Ăłwny plik wykonywalny jest uszkodzony lub nie uruchamia siÄ™, operator otrzymuje instrukcjÄ™ ratunkowÄ… lub aplikacja wykonuje procedurÄ™ automatycznego odtworzenia z `.rollback`.
   - NiedokoĹ„czona prĂłba aktualizacji **nigdy** nie jest wznawiana automatycznie w przĂłd (zakaz ponawiania podmiany bez ponownej peĹ‚nej weryfikacji manifestu i SHA-256).

---

## 24. Blokada WspĂłĹ‚bieĹĽnoĹ›ci (Concurrency Lock)
1. Aktualizator tworzy plik blokady `$baseDir\.update.lock` przed rozpoczÄ™ciem pobierania.
2. Struktura pliku blokady (JSON):
   ```json
   {
       "attempt_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
       "pid": 12345,
       "started_at": 1799276400,
       "version": "1.2.1"
   }
   ```
3. Przed rozpoczÄ™ciem nowej prĂłby badane jest, czy proces o wskazanym PID istnieje. JeĹ›li PID nie istnieje w systemie (Stale Lock), plik blokady jest bezpiecznie zwalniany.
4. Blokada uniemoĹĽliwia rĂłwnolegĹ‚e uruchomienie dwĂłch instancji procedury aktualizacji w tym samym katalogu.

---

## 25. Polityka Anulowania (Cancellation Policy)
1. **Faza bezpieczna (Pre-Apply):**
   - Podczas sprawdzania manifestu, pobierania pliku ZIP oraz rozpakowywania do `.staging`, uĹĽytkownik moĹĽe w kaĹĽdej chwili anulowaÄ‡ operacjÄ™.
   - Skutek: Natychmiastowe przerwanie pobierania, usuniÄ™cie pliku tymczasowego `.SPLogbook_update_*.zip`, usuniÄ™cie katalogu `.staging`, zwolnienie blokady `.update.lock`. Stan programu pozostaje w 100% nienaruszony.
2. **Faza krytyczna (Post-Apply / Swap):**
   - Od momentu przekazania sterowania do helpera i rozpoczÄ™cia podmiany plikĂłw, anulowanie operacji przez uĹĽytkownika jest programowo zablokowane w GUI.
   - Przerwanie procesu helpera wymusza wykonanie procedury Rollbacku.

---

## 26. Badanie PojemnoĹ›ci Dysku (Disk Space Preflight)
Przed zatwierdzeniem pobierania wyliczane jest wymagane miejsce na dysku docelowym:
$$\text{RequiredSpace} = \text{DeclaredDownloadSize} + \text{MaxExtractedSize} + \text{ReplacedFilesSize} + \text{SafetyMargin}$$
- `SafetyMargin` wynosi sztywno **50 MB**.
- Dla typowego wydania SPLogbook: $15\text{ MB (ZIP)} + 60\text{ MB (staging)} + 40\text{ MB (backup)} + 50\text{ MB} = 165\text{ MB}$.
- JeĹ›li wolne miejsce na dysku partycji docelowej (oraz partycji `%TEMP%`) jest mniejsze niĹĽ wymagany prĂłg, aktualizacja jest wstrzymywana z komunikatem `UpdaterError::Io("NiewystarczajÄ…ca iloĹ›Ä‡ wolnego miejsca na dysku.")`.

---

## 27. Uprawnienia i Ĺšrodowisko Windows
- Aktualizator dziaĹ‚a wyĹ‚Ä…cznie w kontekĹ›cie uprawnieĹ„ zalogowanego uĹĽytkownika (Standard User).
- W przypadku wykrycia braku praw zapisu do katalogu docelowego, aktualizator **nie uruchamia** mechanizmu UAC (`runas`), lecz zgĹ‚asza bĹ‚Ä…d uprawnieĹ„.
- ObsĹ‚uga dĹ‚ugich Ĺ›cieĹĽek (`\\?\` prefix) jest stosowana wewnÄ…trz kodu Rust dla zapewnienia odpornoĹ›ci na przekroczenie limitu `MAX_PATH` (260 znakĂłw).

---

## 28. Model BĹ‚Ä™dĂłw (Windows Error Hierarchy)
BĹ‚Ä™dy aktualizatora mapowane sÄ… na precyzyjne warianty `UpdaterError`:
- `UpdaterError::SafetyGate`: PrĂłba uĹĽycia nieobsĹ‚ugiwanego pakietu (np. instalatora MSI).
- `UpdaterError::InvalidStagingLayout`: Brak pliku `SPLogbook.exe` w rozpakowanym archiwum.
- `UpdaterError::ZipSlip`: Wykrycie prĂłby wyjĹ›cia poza katalog stagingu w nagĹ‚Ăłwku ZIP.
- `UpdaterError::ArchiveLimitExceeded`: Przekroczenie limitu dekompresji (bomba dekompresyjna).
- `UpdaterError::HealthCheckFailed`: Niepowodzenie testu zdrowia kandydata i wykonanie rollbacku.
- `UpdaterError::Io`: BĹ‚Ä™dy blokady plikowej NTFS, brak uprawnieĹ„ lub bĹ‚Ä™dy kopiowania.

---

## 29. Logowanie Audytowe i Telemetria Lokalna
- Rejestr zdarzeĹ„ aktualizatora zapisywany jest do lokalnego pliku tekstowego `$baseDir\update_audit.log`.
- **Zasady redakcji danych poufnych:**
  - BezwzglÄ™dny zakaz logowania kluczy prywatnych, tokenĂłw sesyjnych, danych QSO, danych stacji i zawartoĹ›ci konfiguracji.
  - Logowane sÄ… wyĹ‚Ä…cznie: identyfikator prĂłby (`attempt_id`), wersja docelowa, skrĂłt SHA-256 manifestu, skrĂłt assetu, kody etapĂłw oraz czas trwania operacji.

---

## 30. Macierz ZagroĹĽeĹ„ BezpieczeĹ„stwa Windows (Security Matrix)

| ID ZagroĹĽenia | Wektor ataku / BĹ‚Ä…d | Ĺšrodowisko | Mechanizm mitygacji w projekcie | Ryzyko szczÄ…tkowe |
|---|---|---|---|---|
| **THR-WIN-01** | Zip-Slip (traversal `..\`) | Ekstrakcja ZIP | `validate_archive_relative_path` + canonicalize prefix check | Brak (fail-closed) |
| **THR-WIN-02** | Command Injection w PowerShell | Helper skrypt | Przekazywanie parametrĂłw przez precyzyjne tablice literaĹ‚Ăłw, brak `Invoke-Expression` | Zminimalizowane |
| **THR-WIN-03** | Nadpisanie bazy Ĺ‚Ä…cznoĹ›ci | Podmiana plikĂłw | `UpdatePlan` klasyfikuje `default_log.db` jako `UserData` (pomijane) | Wyeliminowane |
| **THR-WIN-04** | Nadpisanie bazy `serviceLOG.db` | Podmiana plikĂłw | Klasyfikacja jako `FirstInstallSeed` (kopiowane tylko gdy brak pliku) | Wyeliminowane |
| **THR-WIN-05** | Podmiana niezweryfikowanego EXE | Pobieranie | WymĂłg `VerifiedReleaseAsset` z podpisem Ed25519 i SHA-256 | Wyeliminowane |
| **THR-WIN-06** | DLL Search-Order Hijacking | Uruchomienie kandydata | Sprawdzanie spĂłjnoĹ›ci bibliotek w katalogu aplikacji, brak szukania w CWD | Niskie |
| **THR-WIN-07** | Zablokowanie programu przez crash nowej wersji | Uruchomienie kandydata | Health confirmation (`.health_ok`) + automatyczny rollback | Niskie |
| **THR-WIN-08** | Stary znacznik zdrowia | Health check | UsuniÄ™cie `.health_ok` przed uruchomieniem kandydata | Wyeliminowane |

---

## 31. Macierz TestĂłw Implementacyjnych dla Etapu M3 (Windows Test Matrix)

| ID Testu | Zakres testu | Warunki wejĹ›ciowe / Fixture | Oczekiwany rezultat | Poziom testu |
|---|---|---|---|---|
| **TEST-WIN-01** | Bezpieczna ekstrakcja ZIP | Archiwum ZIP ze Ĺ›cieĹĽkÄ… `..\..\Windows\system32\malicious.dll` | `Err(UpdaterError::ZipSlip)` | Unit (Rust) |
| **TEST-WIN-02** | Odrzucenie symlinkĂłw | Archiwum ZIP zawierajÄ…ce wpis symlinka | `Err(UpdaterError::UnsafeArchivePath)` | Unit (Rust) |
| **TEST-WIN-03** | Ochrona baz danych w UpdatePlan | Staging z nowym `serviceLOG.db`, active z istniejÄ…cym | `preserve_files` zawiera bazÄ™, brak w `replace_files` | Unit (Rust) |
| **TEST-WIN-04** | Generowanie skryptu PowerShell | `UpdatePlan` z plikami programu i danymi | Brak `$staging\*`, brak `$rollback\*`, obecne `$replaceFiles` | Unit (Rust) |
| **TEST-WIN-05** | Rollback przy awarii zdrowia | Nowa binarka koĹ„czÄ…ca siÄ™ kodem 1 (crash) | PrzywrĂłcenie oryginalnego `SPLogbook.exe`, usuniÄ™cie nowych plikĂłw | Integration (OS) |
| **TEST-WIN-06** | ĹšcieĹĽki ze spacjami | ĹšcieĹĽka instalacji: `C:\My Logbook\SPLogbook\` | Poprawne cytowanie, brak bĹ‚Ä™dĂłw skĹ‚adni PowerShell | Integration (OS) |
| **TEST-WIN-07** | Odmowa dla Program Files | ĹšcieĹĽka: `C:\Program Files\SPLogbook` | `Err(UpdaterError::SafetyGate)` przed pobraniem | Unit / Integration |

---

## 32. KompatybilnoĹ›Ä‡ Wsteczna i Migracje
- Projekt w peĹ‚ni obsĹ‚uguje istniejÄ…cÄ… strukturÄ™ katalogĂłw wersji `v1.2.0`.
- Wszystkie istniejÄ…ce pliki uĹĽytkownika (`default_log.db`, `station_config.json`, `databases/serviceLOG.db`) pozostajÄ… na swoich standardowych miejscach.
- Zmiana nie wymaga modyfikacji struktur bazodanowych ani instalowania zewnÄ™trznych sterownikĂłw.

---

## 33. Rejestr Decyzji WĹ‚aĹ›ciciela dla Windows
W toku projektowania zidentyfikowano nastÄ™pujÄ…ce decyzje leĹĽÄ…ce w gestii WĹ‚aĹ›ciciela:
- **`OWNER-M2-01`:** Potwierdzenie, ĹĽe Windows pozostaje wyĹ‚Ä…cznie w formacie Portable ZIP w M3. (Rekomendacja architektoniczna: **APPROVED**).
- **`OWNER-M2-02`:** Potwierdzenie, ĹĽe MSI / Inno Setup stanowi odrÄ™bny, przyszĹ‚y etap. (Rekomendacja architektoniczna: **APPROVED**).
- **`OWNER-M2-03`:** WybĂłr helpera dla M3: ulepszony PowerShell czy kompilowany EXE `splogbook-updater.exe`. (Rekomendacja architektoniczna: **PowerShell w M3, natywny EXE w M4**).
- **`OWNER-M2-09`:** UsuniÄ™cie katalogu `.rollback` po pomyĹ›lnym potwierdzeniu zdrowia. (Rekomendacja architektoniczna: **APPROVED â€” natychmiastowe usuwanie w celu oszczÄ™dnoĹ›ci miejsca**).

---

## 34. Kryteria Akceptacji Implementacji M3 (Definition of Done)
Etap M3 dla platformy Windows bÄ™dzie mĂłgĹ‚ zostaÄ‡ uznany za zrealizowany wyĹ‚Ä…cznie po speĹ‚nieniu:
1. Wszystkie testy jednostkowe planera i rozpakowywania ZIP przechodzÄ… w 100% na Windows CI runnerze.
2. Generator skryptu PowerShell produkuje wyĹ‚Ä…cznie skrypty uĹĽywajÄ…ce `-LiteralPath` i jawnych list plikĂłw z `UpdatePlan`.
3. Wyeliminowano wszelkie wystÄ…pienia poleceĹ„ z maskami wieloznacznymi `$staging\*` i `$rollback\*`.
4. Test symulacyjny nieudanego health checku wykazuje peĹ‚ne, automatyczne przywrĂłcenie binarki z `.rollback` bez uszczerbku dla danych QSO i pliku konfiguracyjnego.
5. Kod nie wymaga uprawnieĹ„ administratora ani polecenia `runas`.

---

## 35. Odrzucone Alternatywy Architektoniczne
1. **Odrzucono: BezpoĹ›rednie nadpisywanie plikĂłw przez PowerShell bez planowania:** PowodowaĹ‚o niszczenie baz danych uĹĽytkownika i konfiguracji.
2. **Odrzucono: Automatyczne podnoszenie uprawnieĹ„ UAC (`runas`):** W modelu aplikacji przenoĹ›nej instalowanej w folderze uĹĽytkownika UAC jest zbÄ™dne, a jego wymuszanie rodzi zagroĹĽenia bezpieczeĹ„stwa i komplikuje automatyzacjÄ™.
3. **Odrzucono: Traktowanie archiwum ZIP jako pliku wykonywalnego:** Zidentyfikowany i wyeliminowany bĹ‚Ä…d z wersji legacy.

---

## 36. Ryzyka SzczÄ…tkowe (Residual Risks)
1. **Blokada przez oprogramowanie antywirusowe (Antivirus Lock):** NiektĂłre programy antywirusowe mogÄ… zablokowaÄ‡ nowo rozpakowany plik PE na czas skanowania w locie, powodujÄ…c chwilowy bĹ‚Ä…d `AccessDenied`.
   - *Mitygacja w M3:* Helper wprowadzi 3 prĂłby kopiowania z 500 ms odstÄ™pem czasowym przed zgĹ‚oszeniem bĹ‚Ä™du.
2. **Polityka ExecutionPolicy Restricted w Ĺ›rodowiskach domenowych:** W rzadkich konfiguracjach korporacyjnych parametr `-ExecutionPolicy Bypass` moĹĽe zostaÄ‡ zablokowany przez GPO.
   - *Mitygacja:* ZgĹ‚oszenie bĹ‚Ä™du z instrukcjÄ… dla uĹĽytkownika lub przejĹ›cie na natywny helper EXE w kolejnym wydaniu.
