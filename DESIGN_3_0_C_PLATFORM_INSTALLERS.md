# PROJEKT ARCHITEKTONICZNY: WSPĂ“LNY INTERFEJS I RDZEĹ INSTALATORĂ“W PLATFORMOWYCH
# Dokument: DESIGN_3_0_C_PLATFORM_INSTALLERS.md

---

## 1. Status Dokumentu
- **Identyfikator:** `UPDATE-DESIGN-PLATFORM`
- **Etap:** `STAGE-M2-INSTALLER-DESIGN-GATE`
- **Status dokumentu:** `REVIEWED` (Oczekuje na formalne zatwierdzenie otwartych decyzji wĹ‚aĹ›ciciela przez WĹ‚aĹ›ciciela repozytorium)
- **GotowoĹ›Ä‡ wykonawcza M3:** `BLOCKED_BY_OWNER_DECISIONS` (Sekcja 39 i 54)
- **Data opracowania:** 2026-11-07 (Rzeczywista data wykonania M2)
- **Strefa czasowa:** `+01:00` (CET / Europe/Warsaw)
- **Autor / Rola:** GĹ‚Ăłwny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Bezpiecznych AktualizatorĂłw

---

## 2. Wersja i Historia Zmian
- **Wersja:** `2.0.0`
- **Historia:**
  - `v1.0.0` (2026-09-29): Wczesny szkic koncepcyjny podziaĹ‚u rĂłl miÄ™dzy rdzeĹ„ kryptograficzny a instalatory platformowe (historyczny commit pre-M1).
  - `v2.0.0` (2026-11-07): Kompletny projekt architektoniczny wspĂłlnego rdzenia aktualizacji SPLogbook, formalizujÄ…cy maszynÄ™ stanĂłw, transakcyjny model instalacji (`UpdateAttemptId`), abstrakcjÄ™ adapterĂłw platformowych (`PlatformInstallerAdapter`), klasyfikacjÄ™ plikĂłw, model wspĂłĹ‚bieĹĽnoĹ›ci, badanie pojemnoĹ›ci dysku, procedury odzyskiwania po awarii (Crash Recovery) oraz Ĺ›cisĹ‚Ä… piramidÄ™ testĂłw dla M3.

---

## 3. Baseline Git i Kontekst Techniczny
- **Baseline Git HEAD:** Commit po synchronizacji projektu Linux (`072f49e174d4e260ec9dfcfdca9b2f54190da831`).
- **GaĹ‚Ä…Ĺş referencyjna:** `main` / `repair/full-system-remediation`
- **Wersja oprogramowania:** SPLogbook `v1.2.0` (kod ĹşrĂłdĹ‚owy Rust 2024, MSRV 1.85+)
- **Relacja do dokumentĂłw platformowych:** Niniejszy dokument stanowi specyfikacjÄ™ nadrzÄ™dnÄ… dla `DESIGN_3_0_A_WINDOWS_UPDATE.md` oraz `DESIGN_3_0_B_LINUX_UPDATE.md`.

---

## 4. Architektura Warstwowa (Layered Architecture)
Architektura aktualizatora SPLogbook dzieli siÄ™ na cztery Ĺ›ciĹ›le odizolowane warstwy:

```mermaid
flowchart TD
    subgraph Layer1 [Warstwa 1: Prezentacja i Wyzwalanie]
        UI["GUI (app_services.rs / updates.rs)"]
    end

    subgraph Layer2 [Warstwa 2: WspĂłlny RdzeĹ„ Logiczny - Core Engine]
        Trust["Kryptografia Ed25519 (VerifiedReleaseManifest)"]
        AssetSel["Selekcja Assetu (VerifiedReleaseAsset)"]
        Downloader["Strumieniowe Pobieranie z kontrolÄ… SHA-256"]
        Planner["Silnik Planowania (UpdatePlan::generate)"]
        TxManager["MenedĹĽer Transakcji i Maszyna StanĂłw"]
    end

    subgraph Layer3 [Warstwa 3: Adaptery Platformowe - Platform Adapters]
        WinAdapter["Windows Adapter (ZIP, PowerShell Helper, PE Lock)"]
        LinuxAdapter["Linux Adapter (tar.gz, Root Flatten, 0o755, RUNPATH, Rename Swap)"]
    end

    subgraph Layer4 [Warstwa 4: System Operacyjny i Pliki]
        WinFS["Windows NTFS / Registry / Processes"]
        LinuxFS["Linux POSIX Filesystem / Shared Libs / Signals"]
    end

    UI --> TxManager
    TxManager --> Trust
    Trust --> AssetSel
    AssetSel --> Downloader
    Downloader --> TxManager
    TxManager --> WinAdapter
    TxManager --> LinuxAdapter
    WinAdapter --> Planner
    LinuxAdapter --> Planner
    WinAdapter --> WinFS
    LinuxAdapter --> LinuxFS
```

### Granice odpowiedzialnoĹ›ci warstw:
1. **Warstwa Prezentacji:** Odpowiada wyĹ‚Ä…cznie za asynchroniczne wyzwolenie operacji, odbiĂłr zdarzeĹ„ postÄ™pu i prezentacjÄ™ statusu uĹĽytkownikowi. Nie podejmuje decyzji o zaufaniu ani nie dotyka plikĂłw na dysku.
2. **WspĂłlny RdzeĹ„ Logiczny (Core Engine):** Odpowiada za weryfikacjÄ™ podpisu Ed25519, odrzucanie odwoĹ‚anych kluczy, selekcjÄ™ assetu, kontrolÄ™ limitĂłw pobierania, weryfikacjÄ™ SHA-256, generowanie `UpdatePlan` oraz zarzÄ…dzanie cyklem ĹĽycia transakcji.
3. **Adaptery Platformowe:** OdpowiadajÄ… za specyficzne dla danego systemu operacyjnego operacje I/O: formaty archiwĂłw (ZIP vs tar.gz), uprawnienia POSIX, procesy pomocnicze, radzenie sobie z blokadami plikĂłw i wywoĹ‚ania testĂłw zdrowia.
4. **System Operacyjny:** ZarzÄ…dza fizycznym zapisem na dysku i procesami.

---

## 5. Maszyna StanĂłw Aktualizacji (Update State Machine)
Proces aktualizacji jest realizowany jako deterministyczny automat stanĂłw. KaĹĽda zmiana stanu jest jawna i rejestrowana w dzienniku transakcji.

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> CheckingRelease : SprawdĹş aktualizacje
    CheckingRelease --> FetchingManifest : Wykryto nowszÄ… wersjÄ™
    CheckingRelease --> Idle : Brak nowszej wersji / BĹ‚Ä…d sieci
    FetchingManifest --> VerifyingManifest : Pobrano manifest i podpis
    FetchingManifest --> FailedPreApply : BĹ‚Ä…d pobierania / limit
    VerifyingManifest --> SelectingAsset : Podpis poprawny, klucz zaufany
    VerifyingManifest --> FailedPreApply : ZĹ‚y podpis / odwoĹ‚any klucz / downgrade
    SelectingAsset --> Downloading : Wybrano dokĹ‚adnie 1 asset
    SelectingAsset --> FailedPreApply : Brak assetu / niejednoznacznoĹ›Ä‡
    Downloading --> VerifyingDownload : Pobrano caĹ‚oĹ›Ä‡
    Downloading --> FailedPreApply : BĹ‚Ä…d sieci / limit rozmiaru / anulowanie
    VerifyingDownload --> InspectingPackage : Rozmiar i SHA-256 poprawne
    VerifyingDownload --> FailedPreApply : Niezgodny SHA-256 lub rozmiar
    InspectingPackage --> Staging : NagĹ‚Ăłwki bezpieczne, brak ZipSlip
    InspectingPackage --> FailedPreApply : Wykryto ZipSlip / nody specjalne
    Staging --> BuildingUpdatePlan : Rozpakowano i spĹ‚aszczono katalog
    Staging --> FailedPreApply : BĹ‚Ä…d I/O / brak binarki
    BuildingUpdatePlan --> AwaitingApply : Plan poprawny, dane chronione
    BuildingUpdatePlan --> FailedPreApply : BĹ‚Ä…d walidacji bezpieczeĹ„stwa
    AwaitingApply --> Applying : Potwierdzenie instalacji
    AwaitingApply --> Cleanup : Anulowanie przez uĹĽytkownika
    Applying --> LaunchingCandidate : Podmieniono pliki programu
    Applying --> FailedPostApply : BĹ‚Ä…d zapisu / blokada pliku
    LaunchingCandidate --> AwaitingHealth : Uruchomiono proces potomny
    LaunchingCandidate --> FailedPostApply : Nie moĹĽna uruchomiÄ‡ procesu
    AwaitingHealth --> Committed : Wykryto .health_ok w ciÄ…gu 15s
    AwaitingHealth --> FailedPostApply : Timeout 15s / crash procesu
    Committed --> Cleanup : Sukces
    FailedPreApply --> Cleanup : BĹ‚Ä…d przed podmianÄ…
    FailedPostApply --> Rollback : BĹ‚Ä…d w trakcie / po podmianie
    Rollback --> RollbackVerified : PrzywrĂłcono pliki z backupu
    Rollback --> RollbackFailed : BĹ‚Ä…d przywracania (interwencja manualna)
    RollbackVerified --> Cleanup : Sukces rollbacku
    Cleanup --> Idle : Zwolniono zasoby i blokadÄ™
```

---

## 6. WspĂłlne Typy Danych (Common Data Types)
RdzeĹ„ aktualizatora operuje na Ĺ›cisĹ‚ych, niemutowalnych typach danych domenowych:

### 1. `UpdateAttemptId`
Unikalny identyfikator prĂłby aktualizacji (UUID v4), zapobiegajÄ…cy konfliktom i wiÄ…ĹĽÄ…cy wszystkie logi, pliki tymczasowe oraz znaczniki zdrowia.

### 2. `VerifiedReleaseManifest`
Manifest, ktĂłrego sygnatura Ed25519 zostaĹ‚a matematycznie zweryfikowana wzglÄ™dem listy `TRUSTED_KEYS` i ktĂłry nie zostaĹ‚ podpisany odwoĹ‚anym kluczem (`REVOKED_KEY_IDS`).

### 3. `VerifiedReleaseAsset`
Pojedynczy plik instalacyjny jednoznacznie wyselekcjonowany z manifestu na podstawie triady `(platform, arch, package_type)` z powiÄ…zanÄ… sumÄ… SHA-256 i zadeklarowanym rozmiarem.

### 4. `UpdatePlan`
Deterministyczny plan zmian na systemie plikĂłw:
- `replace_files`: pliki programu podlegajÄ…ce podmianie.
- `create_files`: nowe pliki programu lub ziarna instalacji.
- `preserve_files`: chronione dane uĹĽytkownika, bazy i konfiguracja.
- `rejected_files`: odrzucone pliki nierozpoznane (fail-closed).

### 5. `UpdateFileClass`
Klasyfikacja bezpieczeĹ„stwa pliku: `ProgramFile`, `RuntimeLibrary`, `FirstInstallSeed`, `UserData`, `UserConfiguration`, `ReferenceData`, `Cache`, `ReleaseMetadata`, `Unknown`.

---

## 7. Interfejs Adaptera Platformowego (Platform Adapter Trait)
Wszystkie specyficzne dla systemu operacyjnego operacje definiowane sÄ… przez wspĂłlny interfejs Rusta:

```rust
#[async_trait::async_trait]
pub trait PlatformInstallerAdapter: Send + Sync {
    /// Wykrywa typ bieĹĽÄ…cej instalacji i prawa zapisu.
    fn detect_installation_type(&self, base_dir: &Path) -> Result<InstallationType, UpdaterError>;

    /// Bada dostÄ™pnoĹ›Ä‡ wolnego miejsca na dysku partycji docelowej i tymczasowej.
    fn check_disk_space(&self, base_dir: &Path, required_bytes: u64) -> Result<(), UpdaterError>;

    /// Dokonuje inspekcji nagĹ‚ĂłwkĂłw pobranego archiwum przed peĹ‚nÄ… ekstrakcjÄ….
    fn inspect_archive(&self, archive_path: &Path, limits: &ArchiveLimits) -> Result<PackageInspection, UpdaterError>;

    /// Bezpiecznie rozpakowuje archiwum do wskazanego katalogu stagingu.
    fn extract_archive(&self, archive_path: &Path, staging_dir: &Path, limits: &ArchiveLimits) -> Result<(), UpdaterError>;

    /// Normalizuje ukĹ‚ad katalogĂłw stagingu (np. spĹ‚aszcza pojedynczy katalog nadrzÄ™dny).
    fn normalize_staging(&self, staging_dir: &Path) -> Result<(), UpdaterError>;

    /// Waliduje, czy w stagingu znajduje siÄ™ oczekiwany plik wykonywalny programu.
    fn validate_staging_executable(&self, staging_dir: &Path) -> Result<(), UpdaterError>;

    /// Tworzy kopiÄ™ zapasowÄ… podmienianych plikĂłw programu w katalogu rollback.
    fn prepare_backup(&self, base_dir: &Path, rollback_dir: &Path, plan: &UpdatePlan) -> Result<(), UpdaterError>;

    /// Aplikuje plan aktualizacji (podmienia pliki programu i tworzy nowe).
    fn apply_update(&self, base_dir: &Path, staging_dir: &Path, plan: &UpdatePlan, attempt_id: &UpdateAttemptId) -> Result<(), UpdaterError>;

    /// Uruchamia proces kandydata z flagÄ… diagnostycznÄ… --check-health-startup.
    fn launch_candidate(&self, exe_path: &Path, attempt_id: &UpdateAttemptId) -> Result<CandidateProcessHandle, UpdaterError>;

    /// Oczekuje na potwierdzenie zdrowia (.health_ok) kandydata.
    fn wait_for_health(&self, base_dir: &Path, handle: &mut CandidateProcessHandle, timeout: Duration) -> Result<bool, UpdaterError>;

    /// Wykonuje operacjÄ™ wycofania zmian (Rollback).
    fn execute_rollback(&self, base_dir: &Path, rollback_dir: &Path, plan: &UpdatePlan) -> Result<RollbackResult, UpdaterError>;

    /// SprzÄ…ta katalogi stagingu, kopii zapasowej i pliki tymczasowe.
    fn cleanup(&self, base_dir: &Path, staging_dir: &Path, rollback_dir: &Path) -> Result<(), UpdaterError>;
}
```

---

## 8. Wykrywanie Rodzaju Instalacji (Installation Detection)
Aktualizator wspiera ujednolicone rozpoznawanie instalacji:
- `PortableUserWritable`: peĹ‚ne wsparcie automatycznej aktualizacji.
- `PortableReadOnly`: blokada aktualizacji z typowanym komunikatem bĹ‚Ä™du przed rozpoczÄ™ciem pobierania.
- `SystemManagedLocation`: kategoryczna odmowa automatycznej aktualizacji; odesĹ‚anie do menedĹĽera pakietĂłw dystrybucji lub rÄ™cznej instalacji.
- `Unknown`: zachowanie fail-closed.

---

## 9. Granica Zaufania (Trust Boundary)
Zaufanie bazuje na asymetrycznej kryptografii Ed25519:
- **Elementy Zaufane:** Wkompilowane w kod aplikacji klucze publiczne `TRUSTED_KEYS` oraz logika weryfikatora.
- **Elementy Niezaufane:** GitHub API, nagĹ‚Ăłwki HTTP, serwery lustrzane CDN, lokalna konfiguracja uĹĽytkownika `station_config.json`, zmienne Ĺ›rodowiskowe oraz zawartoĹ›Ä‡ pobranych archiwĂłw.
- Zmiana w lokalnej konfiguracji nie moĹĽe wymusiÄ‡ akceptacji nieznanego lub odwoĹ‚anego klucza.

---

## 10. Abstrakcja Inspekcji PakietĂłw (Package Inspection)
Wynik inspekcji pakietu (`PackageInspection`) reprezentuje zbadane wĹ‚aĹ›ciwoĹ›ci archiwum:
- Format kontenera (`Zip` / `TarGz`).
- Model korzenia (`Flat` / `SingleDirectory`).
- Liczba wpisĂłw (musi byÄ‡ $\le 10\,000$).
- ĹÄ…czny rozmiar po dekompresji (musi byÄ‡ $\le 200\text{ MB}$ dla Windows, $\le 250\text{ MB}$ dla Linux).
- Flaga wykrycia prĂłb traversal (`..` lub Ĺ›cieĹĽek bezwzglÄ™dnych).
- Flaga wykrycia niedozwolonych plikĂłw specjalnych (FIFO, nody urzÄ…dzeĹ„, hardlinki).
- Flaga bezpieczeĹ„stwa symlinkĂłw (dopuszczalne wyĹ‚Ä…cznie relatywne symlinki w `lib/` na Linuksie).

---

## 11. Izolowany Staging (Staging Contract)
1. Dekompresja odbywa siÄ™ zawsze do dedykowanego podkatalogu `.staging` wewnÄ…trz katalogu instalacji.
2. Katalog `.staging` przed rozpoczÄ™ciem nowej dekompresji jest czyszczony z wszelkich pozostaĹ‚oĹ›ci po poprzednich prĂłbach.
3. Wszelkie bĹ‚Ä™dy dekompresji natychmiast usuwajÄ… katalog `.staging`.

---

## 12. Kontrakt Planu Aktualizacji (UpdatePlan Contract)
1. Plan generowany jest deterministycznie: lista plikĂłw wejĹ›ciowych jest sortowana leksykograficznie przed przetwarzaniem.
2. Plan nie modyfikuje ĹĽadnych plikĂłw w trakcie generowania (pure function / dry-run).
3. KaĹĽda Ĺ›cieĹĽka podlega rygorystycznemu badaniu bezpieczeĹ„stwa (brak separatorĂłw `..`, brak znakĂłw wieloznacznych).

---

## 13. Polityka Klasyfikacji i Ochrony Danych
WspĂłlna zasada architektoniczna: **`UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE`**.
- Pliki baz danych uĹĽytkownika (`default_log.db`), pliki logĂłw i kopii zapasowych QSO sÄ… traktowane jako nienaruszalne i wykluczone z podmiany.
- Plik konfiguracji `station_config.json` jest nienaruszalny.
- Pliki poczÄ…tkowych baz danych (`serviceLOG.db`, `callbook.db`) podlegajÄ… regule `FirstInstallSeed`: sÄ… kopiowane do instalacji wyĹ‚Ä…cznie wtedy, gdy dany plik docelowy **nie istnieje**. JeĹ›li plik istnieje, jest bezwzglÄ™dnie zachowywany (`preserve_files`).
- Wszelkie nierozpoznane pliki w paczce aktualizacyjnej trafiajÄ… do `rejected_files` i nie sÄ… kopiowane.

---

## 14. Transakcyjny Model Aktualizacji (Transaction Model)
KaĹĽda operacja aktualizacji jest wiÄ…zana z transakcjÄ… o unikalnym identyfikatorze:
```rust
pub struct UpdateTransaction {
    pub attempt_id: UpdateAttemptId,
    pub target_version: String,
    pub target_commit: String,
    pub platform: ReleasePlatform,
    pub arch: ReleaseArchitecture,
    pub package_type: ReleasePackageType,
    pub manifest_sha256: String,
    pub asset_sha256: String,
    pub started_at: u64,
    pub current_phase: UpdatePhase,
    pub rollback_state: RollbackState,
}
```
Stan transakcji nie zawiera ĹĽadnych sekretĂłw, tokenĂłw ani danych QSO.

---

## 15. Kontrakt Kopii Zapasowej (Backup Contract)
- Kopia zapasowa w katalogu `.rollback` obejmuje **wyĹ‚Ä…cznie** pliki ze zbioru `replace_files` zdefiniowane w `UpdatePlan`.
- Kopia zapasowa nigdy nie dubluje plikĂłw uĹĽytkownika ani konfiguracji, co eliminuje ryzyko przywrĂłcenia przestarzaĹ‚ej bazy podczas rollbacku.

---

## 16. Kontrakt Aplikowania Zmian (Apply Contract)
- Podmiana nastÄ™puje bez uĹĽycia masek wieloznacznych (`*`).
- KaĹĽdy plik ze zbioru `replace_files` oraz `create_files` jest kopiowany/przenoszony z uĹĽyciem precyzyjnej, pojedynczej Ĺ›cieĹĽki literaĹ‚owej.
- Na platformie Linux stosowane jest atomowe `rename()`; na Windows bezpieczne kopiowanie sterowane procesem pomocniczym po zamkniÄ™ciu programu gĹ‚Ăłwnego.

---

## 17. Kontrakt Uruchomienia Kandydata (Launch Contract)
- Kandydat uruchamiany jest z flagÄ… diagnostycznÄ… `--check-health-startup`.
- Nowy proces otrzymuje identyfikator transakcji w Ĺ›rodowisku lub parametrze wywoĹ‚ania.
- Czas oczekiwania na potwierdzenie wynosi maksymalnie 15 sekund.

---

## 18. Kontrakt Potwierdzenia Zdrowia (Health Contract)
- Potwierdzenie polega na utworzeniu przez proces kandydata pliku `.health_ok`.
- Plik `.health_ok` zawiera zserializowany rekord JSON: wersjÄ™, commit oraz `attempt_id`.
- Stary plik `.health_ok` jest usuwany przed uruchomieniem kandydata, co zapobiega akceptacji przestarzaĹ‚ego znacznika.

---

## 19. Kontrakt Wycofania Zmian (Rollback Contract)
W razie bĹ‚Ä™du testu zdrowia lub awarii kandydata:
1. Proces kandydata jest natychmiast zatrzymywany (`kill`).
2. Wszystkie nowo utworzone pliki (`create_files`) sÄ… usuwane.
3. Wszystkie pliki podmienione (`replace_files`) sÄ… przywracane z katalogu `.rollback`.
4. Poprzednia wersja programu jest ponownie uruchamiana.
5. Zwracany jest precyzyjny status: `RollbackResult::Success`, `RollbackResult::Partial` lub `RollbackResult::Failed`.

---

## 20. Kontrakt Czyszczenia (Cleanup Contract)
- Po pomyĹ›lnym potwierdzeniu zdrowia katalogi `.staging`, `.rollback` oraz znacznik `.health_ok` sÄ… bezwzglÄ™dnie usuwane.
- Po udanym wycofaniu zmian (rollback) katalog `.staging` i znacznik sÄ… usuwane, a katalog `.rollback` jest zwalniany po przywrĂłceniu plikĂłw.

---

## 21. Odzyskiwanie po Awarii Zasilania (Crash Recovery)
- Przy kaĹĽdym starcie aplikacji SPLogbook bada obecnoĹ›Ä‡ pozostaĹ‚oĹ›ci po przerwanych transakcjach.
- JeĹ›li wykryto porzucony katalog `.staging` lub `.rollback`, aplikacja weryfikuje spĂłjnoĹ›Ä‡ plikĂłw programu i usuwa niekompletne pozostaĹ‚oĹ›ci.
- Przerwana transakcja nigdy nie jest wznawiana automatycznie w przĂłd.

---

## 22. Blokada WspĂłĹ‚bieĹĽnoĹ›ci (Concurrency Control)
- Plik blokady `$baseDir/.update.lock` gwarantuje, ĹĽe w danym katalogu instalacji moĹĽe toczyÄ‡ siÄ™ wyĹ‚Ä…cznie jedna operacja aktualizacji naraz.
- Blokada zawiera PID i czas rozpoczÄ™cia; w przypadku wykrycia martwego procesu (Stale Lock) blokada jest bezpiecznie przejmowana po zalogowaniu ostrzeĹĽenia.

---

## 23. Polityka Anulowania (Cancellation Policy)
- Anulowanie jest dozwolone w fazach: sprawdzania, pobierania, rozpakowywania i planowania.
- Po wejĹ›ciu w fazÄ™ `Applying` (fizyczna podmiana plikĂłw) anulowanie przez uĹĽytkownika jest blokowane w interfejsie graficznym; ewentualny bĹ‚Ä…d wykonania wyzwala deterministyczny rollback.

---

## 24. Diagnostyka i Telemetria Lokalna
- Dziennik operacji zapisywany jest w pliku `update_audit.log`.
- Format wpisĂłw: ISO 8601 UTC, poziom, `attempt_id`, faza, wynik, kod bĹ‚Ä™du.
- BezwzglÄ™dny zakaz umieszczania w logu kluczy prywatnych, danych QSO, poĹ›wiadczeĹ„ i tokenĂłw.

---

## 25. Hierarchia BĹ‚Ä™dĂłw (Error Hierarchy)
BĹ‚Ä™dy aktualizatora sÄ… silnie typowane w enumie `UpdaterError`:
- **BĹ‚Ä™dy Zaufania i Kryptografii:** `MissingSignature`, `InvalidSignature`, `UntrustedKey`, `RevokedKeyRejected`, `DowngradeRejected`.
- **BĹ‚Ä™dy Manifestu i Assetu:** `ManifestParse`, `ManifestValidation`, `NoMatchingAssets`, `AmbiguousArtifact`, `SizeMismatch`, `HashMismatch`.
- **BĹ‚Ä™dy Archiwum i Stagingu:** `ZipSlip`, `UnsafeArchivePath`, `ArchiveLimitExceeded`, `InvalidStagingLayout`.
- **BĹ‚Ä™dy Wykonawcze i Systemowe:** `Io`, `SafetyGate`, `HealthCheckFailed`, `RollbackFailed`.

---

## 26. Macierz ZagroĹĽeĹ„ BezpieczeĹ„stwa WspĂłlnego Rdzenia

| ZagroĹĽenie | Wektor | Mitygacja Architektoniczna | Status |
|---|---|---|---|
| **ZĹ‚oĹ›liwy manifest** | Podmiana odpowiedzi API | Weryfikacja podpisu Ed25519 z `TRUSTED_KEYS` | Wyeliminowane |
| **UĹĽycie skompromitowanego klucza** | Podpisanie starym kluczem 2026-01 | Rejestr `REVOKED_KEY_IDS` (fail-closed) | Wyeliminowane |
| **Atak Downgrade** | Wymuszenie starszej wersji | Weryfikacja SemVer (`verify_downgrade_protection`) | Wyeliminowane |
| **Podmiana pliku w locie (MITM)** | Modyfikacja strumienia pobierania | Strumieniowa weryfikacja SHA-256 z podpisanego manifestu | Wyeliminowane |
| **Uszkodzenie danych uĹĽytkownika** | Kopiowanie zawartoĹ›ci archiwum | Klasyfikacja `UpdateFileClass` + deterministyczny `UpdatePlan` | Wyeliminowane |
| **RĂłwnolegĹ‚e aktualizacje** | PodwĂłjne klikniÄ™cie / dwa procesy | WyĹ‚Ä…czna blokada plikowa `.update.lock` | Wyeliminowane |

---

## 27. ZgodnoĹ›Ä‡ Formatu i Wersjonowanie
- ZgodnoĹ›Ä‡ z SemVer 2.0.
- Schemat manifestu wydaĹ„ pozostaje niezmieniony w fazie M2.
- Wsparcie dla wydaĹ„ oficjalnych z archiwami `SPLogbook-Windows-x64.zip` oraz `SPLogbook-Linux-x86_64.tar.gz`.

---

## 28. Piramida TestĂłw dla Etapu M3 (Test Pyramid)
Architektura testowa M3 opiera siÄ™ na trzech poziomach:
1. **Testy Jednostkowe (Unit Tests):** Weryfikacja logiki czystej (klasyfikator Ĺ›cieĹĽek, planer `UpdatePlan`, walidacja limitĂłw archiwĂłw, detekcja ZipSlip) â€” szybkie, w 100% deterministyczne, wykonywane w pamiÄ™ci i katalogach tymczasowych.
2. **Testy Integracyjne (Integration Tests):** Symulacja peĹ‚nego cyklu transakcji na makietach katalogĂłw (tworzenie stagingu, generowanie backupu, podmiana, symulacja awarii kandydata, weryfikacja rollbacku).
3. **Natywne Testy Platformowe (CI Runners):** Wykonywanie testĂłw na fizycznych/wirtualnych runnerach Windows i Ubuntu w potoku GitHub Actions.

---

## 29. Wymagania DotyczÄ…ce RunnerĂłw Platformowych CI
- **Windows Runner:** `windows-latest` (Windows Server z PowerShell 5.1+).
- **Linux Runner:** `ubuntu-latest` (Ĺ›rodowisko x86_64 z bibliotekami GLIBC).
- Testy nie mogÄ… wymagaÄ‡ uprawnieĹ„ administratora ani polecenia `sudo`.
- Testy HIL pozostajÄ… wyĹ‚Ä…czone (`#[ignore = "hardware-in-the-loop"]`).

---

## 30. WpĹ‚yw na Publiczne API
- Zmiany architektoniczne sÄ… hermetycznie zamkniÄ™te w module `src/cloud/updater.rs`.
- Interfejs wejĹ›ciowy GUI w `src/gui/app_services.rs` (`trigger_update_install`) operuje na bezpiecznym, zweryfikowanym typie `VerifiedReleaseAsset`.

---

## 31. WpĹ‚yw na Formaty Danych i Bazy SQLite
- **Brak jakiegokolwiek wpĹ‚ywu:** Schematy baz SQLite (`serviceLOG.db`, `callbook.db`, `default_log.db`) pozostajÄ… nienaruszone.
- Struktura pliku konfiguracyjnego `station_config.json` pozostaje nienaruszona.

---

## 32. WpĹ‚yw na ZaleĹĽnoĹ›ci Cargo
- Projekt M2 nie dodaje ani nie modyfikuje ĹĽadnych zaleĹĽnoĹ›ci w `Cargo.toml` ani `Cargo.lock`.
- Wszelkie operacje bazujÄ… na istniejÄ…cych pakietach (`zip`, `tar`, `flate2`, `ed25519-dalek`, `sha2`, `walkdir`, `tempfile`).

---

## 33. Sekwencja Implementacji dla Etapu M3 (Implementation Sequence M3)
WdroĹĽenie M3 powinno nastÄ…piÄ‡ w nastÄ™pujÄ…cych, Ĺ›cisĹ‚ych krokach inĹĽynierskich:
1. **Krok M3-1 (Core Adapter Trait):** Zdefiniowanie cechy `PlatformInstallerAdapter` oraz unifikacja struktury `UpdateTransaction` w `src/cloud/updater.rs`.
2. **Krok M3-2 (Windows Hardening):** Utwardzenie generatora PowerShell w oparciu o precyzyjnÄ… listÄ™ plikĂłw z `UpdatePlan` oraz dodanie obsĹ‚ugi kodĂłw powrotu health checku.
3. **Krok M3-3 (Linux In-Process / Rename):** Refaktoryzacja `install_via_tar_gz` do peĹ‚nej zgodnoĹ›ci z `UpdatePlan` i atomowym `rename()`.
4. **Krok M3-4 (Health & Rollback Integration):** Zintegrowanie obsĹ‚ugi znacznika `.health_ok` z attempt ID w `main.rs`.
5. **Krok M3-5 (Crash Recovery & Lock):** Implementacja mechanizmu blokady wspĂłĹ‚bieĹĽnoĹ›ci `.update.lock` oraz usuwania porzuconych stagingĂłw przy starcie.
6. **Krok M3-6 (Platform CI Validation):** WĹ‚Ä…czenie natywnych testĂłw integracyjnych w potoku GitHub Actions.

---

## 34. Sekwencja CommitĂłw dla Etapu M3 (Commit Sequence M3)
- `feat(updater): introduce platform installer adapter and transaction state machine`
- `fix(updater): harden Windows update helper with strict UpdatePlan file lists`
- `fix(updater): implement atomic rename and rollback semantics for Linux tar.gz`
- `feat(updater): bind health confirmation marker with update attempt identity`
- `feat(updater): add update concurrency lock and crash recovery routines`
- `test(updater): add cross-platform integration test suite for update transactions`

---

## 35. Kryteria UkoĹ„czenia M3 (Definition of Done)
Etap M3 zostanie uznany za ukoĹ„czony, gdy:
1. Kod ĹşrĂłdĹ‚owy w `src/cloud/updater.rs` w peĹ‚ni realizuje powyĹĽszÄ… architekturÄ™.
2. Wszystkie testy automatyczne przechodzÄ… na Windows i Linux runnerach bez ostrzeĹĽeĹ„ Clippy.
3. BezpieczeĹ„stwo danych uĹĽytkownika i baz danych jest potwierdzone testami guard w `tests/security_guard.rs`.
4. W repozytorium nie ma ĹĽadnych wywoĹ‚aĹ„ `sudo`, `pkexec` ani niebezpiecznych masek wieloznacznych.

---

## 36. ZaleĹĽnoĹ›ci od Decyzji WĹ‚aĹ›ciciela (Owner Decision Dependencies)
RozpoczÄ™cie implementacji M3 jest zablokowane do czasu formalnego zatwierdzenia decyzji przez WĹ‚aĹ›ciciela repozytorium:
- `OWNER-M2-01` do `OWNER-M2-15` (szczegĂłĹ‚owo opisane w rejestrze decyzji).
