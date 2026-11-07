# RAPORT PRZEDIMPLEMENTACYJNY STAGE-M3-A-PLATFORM-CORE-IMPLEMENTATION-GATE

**Zadanie:** STAGE-M3-A-PLATFORM-CORE-IMPLEMENTATION-GATE
**Kanoniczny tytuł:** Etap M3-A — implementacja wspólnego rdzenia platformowego, modelu transakcji, automatu stanów i mechanizmów recovery aktualizatora SPLogbook
**Priorytet:** P0 — KRYTYCZNY
**Rola:** Główny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Bezpiecznych Aktualizatorów i Transakcyjnych Systemów Plików

---

## 1. Data i Czas Środowiska

- **Historyczna data poprzedniego raportu:** `2026-11-07`
- **Rzeczywista data wykonania M3-A:** `2026-11-07`
- **Rzeczywisty czas odczytany ze środowiska:** `12:32:42`
- **Strefa czasowa / przesunięcie UTC:** `+01:00` (Central European Standard Time / CET)
- **Rozbieżność chronologiczna:** Brak rozbieżności. Rzeczywista data środowiska wykonawczego (`2026-11-07`) w pełni pokrywa się z datą roboczą poprzedniego raportu (`2026-11-07`). Zegar systemowy oraz znaczniki czasu istniejących commitów pozostają nienaruszone.

---

## 2. Stan Repozytorium Git

- **Punkt wejściowy (Baseline HEAD):** `cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa`
- **Lokalna gałąź robocza:** `main` (`cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa`)
- **Gałąź `repair/full-system-remediation`:** `cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa`
- **Gałąź zdalna `origin/main`:** `cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa`
- **Status po `git fetch --prune origin`:**
  - `origin/main`: `cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa`
  - Licznik rozbieżności `origin/main...main`: `0 / 0`
  - Licznik rozbieżności `origin/main...repair/full-system-remediation`: `0 / 0`
  - Wspólny przodek (`merge-base`): `cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa`
- **Nowe commity zdalne:** 0
- **Operacje w toku (`MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`):** Czyste (brak jakichkolwiek aktywnych operacji).
- **Working Tree:** Czyste (`git status --porcelain=v1` zwraca pusty wynik).
- **Klasyfikacja Git:** `A. EXACT_BASELINE` — HEAD == main == origin/main == cf77f4a15b1aa872b1df7c6e0bdf84dbf168c7fa.

---

## 3. Ochrona Plików Cargo i Analiza Zależności

- **`Cargo.toml` working-tree hash:** `0cdf381165bfc3111fff15097007e4b235484ec8`
- **`Cargo.toml` HEAD blob:** `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`
- **`Cargo.lock` working-tree hash:** `52be66b2bc063c001d5c2e8556457fa2be04ff4f`
- **`Cargo.lock` HEAD blob:** `c569097f3ef69c66ec59b74c29c1eb29efce69ae`
- **EOL status:** `Cargo.toml` i `Cargo.lock` raportują `i/lf w/crlf attr/` (różnica hashy working tree vs repozytorium wynika w 100% z normalizacji LF w repozytorium Git do CRLF na dysku Windows).
- **Logical diff:** `EMPTY` (`git diff HEAD -- Cargo.toml Cargo.lock` zwraca pusty wynik).
- **Weryfikacja istniejących zależności:**
  - `uuid = { version = "1", features = ["v4"] }` — potwierdzona obecność w `Cargo.toml:53` oraz `Cargo.lock:5753`.
  - `serde = { version = "1.0", features = ["derive"] }` — potwierdzona obecność w `Cargo.toml:29`.
  - `serde_json = "1.0"` — potwierdzona obecność w `Cargo.toml:30`.
  - `tempfile = "3.27"` — potwierdzona obecność w `Cargo.toml:74`.
  - `fs2`, `file-lock`, `sysinfo` — brak w `Cargo.toml` i `Cargo.lock`.
- **Wymóg nowych zależności:** **BRAK (0 nowych zależności)**.
  - Blokada pojedynczej aktywnej aktualizacji zostanie zrealizowana w oparciu o atomową semantykę biblioteki standardowej: `std::fs::OpenOptions::new().write(true).create_new(true)` (`O_CREAT | O_EXCL` na POSIX, `CREATE_NEW` na Windows), co wyklucza wyścigi TOCTOU.
  - Badanie przestrzeni dyskowej oparte jest na czystym modelu matematycznym z wstrzykiwalnym `CapacityProvider` (oraz platformowej implementacji bez konieczności zewnętrznych bibliotek).
  - `Cargo.toml` i `Cargo.lock` pozostają bezwzględnie nienaruszone (`UNCHANGED`).

---

## 4. Weryfikacja Bazowa (Baseline Gates)

- **`cargo +stable fmt --all -- --check`:** PASSED (kod sformatowany, brak naruszeń stylu).
- **`cargo +stable check --workspace --all-targets --all-features -j 2`:** PASSED (0 błędów kompilacji).
- **`cargo +stable clippy --workspace --all-targets --all-features -j 2 -- -D warnings`:** PASSED (0 ostrzeżeń Clippy).
- **`cargo +stable test --workspace --all-targets -j 2`:**
  - `splogbook` (lib): 1694 passed, 0 failed, 0 ignored
  - `SPLogbook` (bin): 0 passed, 0 failed, 0 ignored
  - `splogbook-signer` (bin): 0 passed, 0 failed, 0 ignored
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed, 0 ignored
  - `tests/cat_native_6b.rs`: 99 passed, 0 failed, 0 ignored
  - `tests/cat_native_7.rs`: 297 passed, 0 failed, 10 ignored (wszystkie 10 testów to odizolowane testy HIL fizycznego sprzętu radiowego)
  - `tests/i18n_guard.rs`: 15 passed, 0 failed, 0 ignored
  - `tests/security_guard.rs`: 64 passed, 0 failed, 0 ignored
  - `tests/theme_guard.rs`: 1 passed, 0 failed, 0 ignored
- **`cargo +stable test --doc -j 2`:** PASSED (0 passed, 0 failed).
- **Statystyki testów bazowych:**
  - **TOTAL REGISTERED:** 2345
  - **TOTAL EXECUTED:** 2335
  - **TOTAL PASSED:** 2335
  - **TOTAL FAILED:** 0
  - **TOTAL IGNORED:** 10 (wyłącznie HIL)
  - **HIL EXECUTED:** 0
  - **HIL IGNORED:** 10
- **`cargo audit`:** PASSED (0 podatności, 7 dozwolonych ostrzeżeń unmaintained transitives zgodnych z udokumentowaną białą listą).
- **`git diff --check`:** Czyste (brak spacji na końcach linii i problemów z EOL).

---

## 5. Analiza Istniejącej Architektury

- **Moduł updatera (`src/cloud/updater.rs`):**
  - Duży plik (~6206 linii kodu) zawierający zarówno pobieranie danych referencyjnych (cty.dat, master.scp, lotw), parsowanie i weryfikację podpisu Ed25519 manifestu wydań, klasyfikację ścieżek `classify_update_path`, jak i procedury instalacji ZIP oraz tar.gz.
  - Wykorzystuje typy `UpdatePlan` (z polami `replace_files`, `create_files`, `preserve_files`, `rejected_files`), `UpdateFileClass` oraz `AtomicInstallationOptions`.
- **Integracja GUI (`src/gui/app_services.rs`, `src/gui/state/updates.rs`):**
  - Obsługa sprawdzania aktualizacji w tle za pośrednictwem kanału mpsc zwracającego `UpdateCheckOutcome`.
  - `UpdateState` w GUI przechowuje jedynie status sprawdzenia, informację o nowej wersji i stan pobierania/błędu, bez deterministycznej orkiestracji transakcyjnej.
- **Wykrywanie instalacji:**
  - Obecnie ograniczone do formatów `zip` oraz `tar.gz` wywodzonych z konfiguracji platformy i manifestu. Brak modelu instalacji systemowych vs samodzielnych.
- **Katalogi stagingu i rollbacku:**
  - Staging tworzony pod `.staging`, rollback pod `.rollback` w katalogu programu.
  - Brak powiązania rollbacku i weryfikacji zdrowia z unikalnym identyfikatorem próby.
- **Weryfikacja zdrowia (`.health_ok`):**
  - Sprawdza istnienie pliku `.health_ok` w `base_dir`, bez walidacji, czy należy on do bieżącej próby aktualizacji (podatność na odczyt starych artefaktów).

---

## 6. Luki Względem Zatwierdzonego Projektu DESIGN 3.0 C

1. **Brak domenowego identyfikatora `UpdateAttemptId`:** Brak bezpiecznego, losowego identyfikatora próby opartego o UUID v4 uniemożliwiającego kolizje i porzucanie osieroconych artefaktów.
2. **Brak trwałego modelu transakcji (`UpdateTransaction`):** Brak struktury przechowującej pełen kontekst transakcji, wersję formatu rekordu, sumy kontrolne, stan bieżący i bezpieczną serializację/deserializację do pliku.
3. **Brak kompletnego automatu 20 stanów:** Brak deterministycznej maszyny stanów egzekwującej prawidłową kolejność faz (od `Idle` przez inspekcję, staging, plan, `AwaitingApply`, `Applying` aż po `Committed` lub stany rollbacku/recovery).
4. **Brak podziału na `SelfManaged` i `SystemManaged` oraz macierzy możliwości (Capabilities):** Brak abstrakcji rozróżniającej instalacje przenośne od zarządzanych przez system operacyjny.
5. **Brak wspólnego kontraktu adaptera platformowego (`PlatformInstallerAdapter`):** Brak ujednoliconego interfejsu dla operacji wykrywania, inspekcji, stagingu, planowania, delegacji, uruchamiania kandydata i wycofywania zmian.
6. **Brak atomowej blokady współbieżności aktualizacji (`.update.lock`):** Możliwość uruchomienia równoległych procesów aktualizacji; brak rekordu właściciela blokady (PID, attempt ID, start time).
7. **Brak parametrycznej formuły preflightu dyskowego:** Obecność arbitralnego założenia o stałej wielkości bufora zamiast ścisłej formuły sumującej pobranie, rozpakowanie, backup, narzut menedżera pakietów, metadane i konfigurowalny margines 25–100 MiB (domyślnie 32 MiB).
8. **Brak rygorystycznej polityki anulowania (`OWNER-M2-13`):** Brak rozróżnienia fazy anulowalnej (przed `Applying`) od fazy nieanulowalnej (od momentu wejścia w `Applying`).
9. **Brak silnika decyzji recovery (Crash Recovery Decision Engine):** Brak deterministycznej analizy niedokończonych transakcji przy starcie i bezwzględnego zakazu automatycznego re-apply po awarii.
10. **Brak powiązania Health Proof z próbą aktualizacji:** Brak struktury `HealthProof` weryfikującej zgodność `attempt_id`, wersji, commit SHA i czasu utworzenia.
11. **Brak typowanych modeli błędów transakcyjnych:** Brak szczegółowych, bezpiecznych (pozbawionych sekretów) wariantów błędów transakcji, locka, dysku, przejść stanów i uprawnień.

---

## 7. Zakres Implementacji M3-A

W celu zachowania przejrzystości, wysokiej spójności i modułowości (zgodnie z sekcją 11 wytycznych), implementacja wspólnego rdzenia platformowego zostanie zorganizowana w nowym podkatalogu `src/cloud/updater/` z re-eksportem przez `src/cloud/updater.rs`:

### Pliki produkcyjne:
1. `src/cloud/updater/attempt_id.rs` — domenowy typ `UpdateAttemptId` oparty na UUID v4, bezpieczne parsowanie, Display, walidacja składowych systemu plików.
2. `src/cloud/updater/state.rs` — kompletny 20-stanowy automat stanów (`UpdateStateId`, `UpdateStateMachine`), dozwolone i niedozwolone przejścia.
3. `src/cloud/updater/transaction.rs` — model `UpdateTransaction`, wersjonowanie rekordu (v1), bezpieczny crash-safe zapis transakcji (tempfile + flush + sync + atomic rename), brak sekretów.
4. `src/cloud/updater/installation.rs` — typy `InstallationType`, `ManagementMode`, fail-closed dla `UnknownInstallation`, detekcja oparta o wiarygodne markery.
5. `src/cloud/updater/capabilities.rs` — model `InstallationCapabilities` z centralną macierzą możliwości formatów.
6. `src/cloud/updater/platform.rs` — cecha `PlatformInstallerAdapter`, pomocniczy `FakePlatformAdapter` dla testów.
7. `src/cloud/updater/preflight.rs` — model obliczeń `DiskPreflightRequirements`, arytmetyka sprawdzana (checked), margines 25–100 MiB (domyślnie 32 MiB), cecha `CapacityProvider`.
8. `src/cloud/updater/lock.rs` — atomowa blokada `.update.lock` via `create_new(true)`, identyfikacja właściciela (attempt ID, PID, start time), fail-closed na uszkodzonym locku, guard zwalniający wyłącznie własną blokadę.
9. `src/cloud/updater/cancellation.rs` — polityka anulowania transakcji (`can_cancel`), zakaz anulowania od `Applying`.
10. `src/cloud/updater/recovery.rs` — silnik decyzji recovery (`RecoveryDecisionEngine`, `RecoveryDecision`), idempotencja, zakaz automatycznego re-apply, ochrona danych użytkownika.
11. `src/cloud/updater/health.rs` — model `HealthProof` powiązany z `UpdateAttemptId`, wersją, commitem i czasem utworzenia, odrzucanie starych dowodów.
12. `src/cloud/updater/error.rs` — typowane błędy rdzenia platformowego z bezpiecznym formatowaniem diagnostycznym i brakiem wycieku danych wrażliwych.
13. `src/cloud/updater.rs` — deklaracja podmodułów i publiczny re-eksport typów wspólnego rdzenia.

### Pliki testowe:
- Testy jednostkowe w każdym z dedykowanych podmodułów `src/cloud/updater/*.rs`.
- `tests/security_guard.rs` — rozszerzenie o statyczne i dynamiczne strażniki bezpieczeństwa M3-A (brak publicznego niezwalidowanego konstruktora, brak swobodnego mutowania stanu, zakaz anulowania po Applying, atomowość locka, zakaz auto-apply w recovery, fail-closed nieznanych instalacji, ochrona baz i brak komend eskalacji).

### Pliki dokumentacyjne:
- `STAGE_M3_A_PRE_IMPLEMENTATION_REPORT.md` (niniejszy dokument).
- `STAGE_M3_A_PLATFORM_CORE_IMPLEMENTATION_REPORT.md` (końcowy raport wykonawczy zamykający bramkę).
- Aktualizacje rejestrów roadmapy i planu implementacji: `MASTER_IMPLEMENTATION_PLAN.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`, `docs/AUDIT_FINDINGS_REGISTER.md`.

---

## 8. Plan Commitów

Zgodnie z sekcją 28 specyfikacji bramki zaplanowano 6 atomowych, precyzyjnych commitów:

### COMMIT 1:
- **Komunikat:** `feat(updater): add typed update transaction and state machine (M3-A)`
- **Zakres:** `UpdateAttemptId`, automat 20 stanów (`UpdateStateId`, `UpdateStateMachine`), `UpdateTransaction`, wersjonowanie i crash-safe zapis rekordu transakcji, testy poprawnych i zabronionych przejść.

### COMMIT 2:
- **Komunikat:** `feat(updater): classify installation management capabilities (M3-A)`
- **Zakres:** `InstallationType`, `ManagementMode` (`SelfManaged` / `SystemManaged`), macierz `InstallationCapabilities`, interfejs `PlatformInstallerAdapter`, `FakePlatformAdapter`, testy klasyfikacji i zdolności.

### COMMIT 3:
- **Komunikat:** `feat(updater): add disk preflight and update concurrency lock (M3-A)`
- **Zakres:** Formuła `RequiredSpace`, arytmetyka checked, walidacja marginesu 25–100 MiB, cecha `CapacityProvider`, atomowa blokada `.update.lock` via `create_new(true)`, lock record i guard, testy rywalizacji i wyłącznego zwycięzcy.

### COMMIT 4:
- **Komunikat:** `feat(updater): recover interrupted update transactions safely (M3-A)`
- **Zakres:** `RecoveryDecisionEngine`, polityka anulowania przed i po `Applying`, model `HealthProof` powiązany z attempt ID, czyszczenie artefaktów bez naruszania danych użytkownika, testy idempotencji i zakazu re-apply.

### COMMIT 5:
- **Komunikat:** `test(updater): guard M3 platform core safety contracts`
- **Zakres:** Rozszerzenie `tests/security_guard.rs` o guardy M3-A, testy przekrojowe nienaruszalności danych użytkownika (`serviceLOG.db`, `callbook.db`, `station_config.json`, `default_log.db`), testy stabilności i współbieżności.

### COMMIT 6:
- **Komunikat:** `docs(updater): close M3-A platform core implementation gate`
- **Zakres:** Końcowy raport wykonawczy `STAGE_M3_A_PLATFORM_CORE_IMPLEMENTATION_REPORT.md`, synchronizacja `MASTER_IMPLEMENTATION_PLAN.md`, `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`.

---

## 9. Warunek Przystąpienia do Implementacji

Wszystkie warunki wejściowe zostały zweryfikowane:
- Środowisko czyste i zsynchronizowane (`EXACT_BASELINE`),
- Zależności Cargo nienaruszone i kompletne (obecność `uuid v4`),
- 2335 testów jednostkowych i integracyjnych przeszło pomyślnie,
- 0 ostrzeżeń Clippy, 0 podatności `cargo audit`,
- 0 wykonanych fizycznych testów HIL.

Raport przedimplementacyjny został pomyślnie utworzony. Przystępuję do realizacji zaplanowanych commitów.
