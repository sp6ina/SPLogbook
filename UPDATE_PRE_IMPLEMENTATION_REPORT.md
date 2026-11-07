# RAPORT PRZEDIMPLEMENTACYJNY UPDATE-TRUST-AND-DATA-SAFETY-GATE

**Data robocza projektu:** 2026-11-06  
**Rzeczywista data środowiska:** 2026-11-07T09:16:50+01:00 (Central European Standard Time)  
**Rozbieżność chronologiczna:** Odnotowana zgodnie z wytycznymi — zegar systemowy i daty istniejących commitów pozostają nienaruszone.  
**Zadanie:** UPDATE-TRUST-AND-DATA-SAFETY-GATE  
**Priorytet:** P0 — KRYTYCZNY  
**Architekt:** Główny Architekt Techniczny SPLogbook & Senior Rust Developer  

---

## 1. Git

- **Punkt wejściowy (Baseline HEAD):** `a82df1fc104ad12c3c4a4c784191ecca8c385f52`
- **Lokalna gałąź robocza:** `repair/full-system-remediation` (`a82df1fc104ad12c3c4a4c784191ecca8c385f52`)
- **Gałąź `main`:** `a82df1fc104ad12c3c4a4c784191ecca8c385f52`
- **Gałąź zdalna `origin/main`:** `a82df1fc104ad12c3c4a4c784191ecca8c385f52`
- **Operacja `git fetch --prune origin`:** Wykonana pomyślnie.
- **Nowe commity na `origin/main`:** 0 (brak nowych commitów po stronie zdalnej).
- **Ahead / Behind:** `0 / 0`
- **Wspólny przodek (`merge-base`):** `a82df1fc104ad12c3c4a4c784191ecca8c385f52`
- **Operacje w toku (`MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`):**
  - Czyste. Weryfikacja wykazała archiwalny plik `.git/REBASE_HEAD` z 28.09.2026 (`git rebase --abort` zwróciło `fatal: no rebase in progress`); plik usunięto, weryfikacja wszystkich wskaźników operacji zwraca status pusty.
- **Working Tree:** Czyste (`git status --porcelain=v1` zwraca pusty ciąg).
- **Klasyfikacja Git:** `A. EXACT_BASELINE`.

---

## 2. Cargo

- **CARGO.TOML WORKING-TREE HASH:** `0cdf381165bfc3111fff15097007e4b235484ec8`
- **CARGO.TOML HEAD BLOB:** `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`
- **CARGO.LOCK WORKING-TREE HASH:** `52be66b2bc063c001d5c2e8556457fa2be04ff4f`
- **CARGO.LOCK HEAD BLOB:** `c569097f3ef69c66ec59b74c29c1eb29efce69ae`
- **LOGICAL DIFF:** EMPTY (`git diff HEAD -- Cargo.toml Cargo.lock` jest pusty).
- **EOL:** `Cargo.lock` i `Cargo.toml` mają `i/lf w/crlf attr/` (różnica skrótów wynika w 100% z normalizacji LF w bazie Git do CRLF na dysku Windows).
- **Zależności:** 674 skrzynki. Brak nowych zależności. Zakaz jakichkolwiek modyfikacji `Cargo.toml` i `Cargo.lock`.

---

## 3. Test baseline

- **TOTAL REGISTERED TESTS:** 2025 (1645 lib + 307 integration cat_native + 15 i18n_guard + 57 security_guard + 1 theme_guard)
- **TOTAL EXECUTED TESTS:** 2015
- **TOTAL PASSED TESTS:** 2015
- **TOTAL FAILED TESTS:** 0
- **TOTAL IGNORED TESTS:** 10
- **HIL IGNORED TESTS:** 10 (odizolowane w `tests/cat_native_7.rs`, brak flagi `--ignored`)
- **CLIPPY WARNINGS:** 0 (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
- **CARGO AUDIT:** 0 podatności, 7 dozwolonych ostrzeżeń unmaintained (zgodne z białą listą w `security_guard.rs`)
- **DOC TESTS:** 0, ok.

---

## 4. Aktywna ścieżka aktualizacji

1. **GUI Entry (`trigger_update_check` w `src/gui/app_services.rs`):**
   - Wywołuje `latest_release()`.
   - `latest_release()` pobiera metadane JSON z GitHub API, a następnie wywołuje `fetch_and_verify_manifest(&latest, current_version)`.
   - Wynik trafia do `self.updates.update_available = Some(release)`.
2. **GUI Install Entry (`trigger_update_install` w `src/gui/app_services.rs`):**
   - Pobiera `release` ze stanu `self.updates.update_available`.
   - **LUKA ARCHITEKTONICZNA:** Wywołuje `select_asset_for_platform(&release.assets)`, dobierając plik wyłącznie po rozszerzeniu (`.zip`, `.exe`, `.msi` / `.tar.gz`, `.appimage`, `.deb`).
   - Przekazuje surowy `&ReleaseAsset` do `crate::cloud::updater::install_update(&asset)`.
3. **Instalacja (`install_update` w `src/cloud/updater.rs`):**
   - Wywołuje `check_safety_gate(asset)`, która odrzuca wyłącznie `.msi` (Windows) lub `.deb`/`.rpm` (Linux).
   - Pobiera plik z `asset.browser_download_url` do pliku tymczasowego `.zip` / `.tar.gz`.
   - Sprawdza sumę SHA-256 tylko jeśli pole `digest` jest obecne.
   - Wywołuje `self_replace(&current, &tmp_path)`.
4. **Windows Apply (`install_via_powershell`):**
   - Rozpakowuje ZIP do `.staging` przez `extract_zip_safe_windows`.
   - Spłaszcza ewentualny pojedynczy folder nadrzędny.
   - Generuje i odpala skrypt PowerShell, który:
     - Kopiuje istniejące pliki do `.rollback`;
     - Wykonuje bezwarunkowe `Copy-Item -Path "$staging\*" -Destination $baseDir -Recurse -Force`;
     - Sprawdza `--check-health-startup` (oczekiwanie na `.health_ok` do 15 s);
     - Przy sukcesie usuwa `.staging` i `.rollback`;
     - Przy awarii wykonuje `Copy-Item -Path "$rollback\*" -Destination $baseDir -Recurse -Force` i uruchamia stary proces.
5. **Linux Apply (`install_via_tar_gz`):**
   - Sprawdza czy `base_dir` jest readonly.
   - Rozpakowuje tar.gz do `.staging` przez `extract_tar_gz_safe`.
   - Spłaszcza nadrzędny folder.
   - Kopiuje istniejące pliki do `.rollback`.
   - Atomowo podmienia wszystkie pliki ze stagingu do `base_dir` (`fs::rename`).
   - Sprawdza `--check-health-startup` (oczekiwanie na `.health_ok`).
   - W razie sukcesu usuwa staging i rollback.
   - W razie awarii przywraca pliki z rollbacku.

---

## 5. Model zaufania

- **Zaufane klucze (`TRUSTED_KEYS` w `src/cloud/updater.rs`):**
  - `splogbook-release-2026-02`: aktywny (`active: true`), klucz publiczny `[120, 98, 36, 60...]`, fingerprint `2a6277b7...`.
  - `splogbook-release-2026-01`: odwołany (`active: false`), klucz publiczny `[109, 72, 1, 34...]`.
- **Rejestr odwołań:**
  - `REVOKED_KEY_IDS`: `["splogbook-release-2026-01"]`.
  - `REVOKED_PUBLIC_KEYS`: 5 odwołanych fingerprintów.
- **Klucze nieznane / puste / brak klucza:**
  - Zwracają `UpdaterError::UntrustedKey` lub `UpdaterError::KeyMismatch` (fail-closed).
- **Status wydania v1.2.0:**
  - Manifest v1.2.0 deklaruje `key_id: "splogbook-release-2026-01"`.
  - Podpis jest matematycznie poprawny dla klucza `splogbook-release-2026-01`.
  - Bieżąca polityka zaufania trwale odrzuca manifest v1.2.0 z powodu obecności identyfikatora i klucza na listach odwołań.
  - v1.2.0 nie może zostać pobrane ani zainstalowane przez bezpieczny aktualizator.

---

## 6. Klasyfikacja danych

| Plik | Lokalizacja runtime | Lokalizacja w paczce | Tryb otwarcia w kodzie | Mutacja w runtime | Ryzyko dla danych użytkownika | Polityka nadpisywania | Dowód |
|---|---|---|---|---|---|---|---|
| `SPLogbook.exe` / `splogbook` | `base_dir` | root / `SPLogbook-Linux-x86_64/` | Executable | NIE (zastępowany w całości) | BRAK | `ProgramFile`: ALLOW (wymagany backup i health check) | [WORKFLOW][CODE] |
| `*.dll` / `*.so*` | `base_dir` | root / `hamlib/` | Library | NIE | BRAK | `ProgramFile`: ALLOW | [WORKFLOW][CODE] |
| Narzędzia Hamlib (`rigctl*`, `ampctl*` itp.) | `base_dir` | root | Executable | NIE | BRAK | `ProgramFile`: ALLOW | [WORKFLOW][CODE] |
| Skrypty pomocnicze (`run.sh`, `.desktop`) | `base_dir` | root | Shell/Config | NIE | BRAK | `ProgramFile`: ALLOW (zachowanie bitu 0o755) | [ARCHIVE] |
| `databases/serviceLOG.db` | `base_dir/databases` lub AppData | `databases/serviceLOG.db` | Read-only SQLite | NIE | Utrata ewentualnych lokalnych wpisów IOTA/prefiksów | `FirstInstallSeed`: DENY OVERWRITE IF EXISTS (PRESERVE) | [CODE] `service_db.rs:89` |
| `databases/callbook.db` | `base_dir/databases` lub AppData | `databases/callbook.db` | Read-only SQLite | NIE | Utrata danych | `FirstInstallSeed`: DENY OVERWRITE IF EXISTS (PRESERVE) | [CODE] `callbook.rs:94` |
| `databases/default_log.db` | `base_dir/databases` lub AppData | BRAK w paczce | Read-write SQLite (WAL) | **TAK (ciągła - QSO)** | **KRYTYCZNE (utrata logu)** | `UserData`: **DENY OVERWRITE (CRITICAL PRESERVE)** | [CODE] `main.rs:136` |
| `station_config.json` | `base_dir/databases` lub ConfigDir | BRAK w paczce | Read-write JSON | **TAK** | **KRYTYCZNE (utrata poświadczeń)** | `UserConfiguration`: **DENY OVERWRITE (CRITICAL PRESERVE)** | [CODE] `main.rs:186` |
| `databases/*.db` (nieznane bazy) | `base_dir/databases` | Dowolne | SQLite | Potencjalna | WYSOKIE | `Unknown`: DENY OVERWRITE (FAIL-CLOSED PRESERVE) | Reguła bezpieczeństwa |
| Pliki nieznane w paczce | `staging` | Dowolne | Nieznane | Nieznana | Potencjalny Zip Slip/Malware | `Unknown`: REJECT UPDATE | Reguła bezpieczeństwa |

---

## 7. Potwierdzone defekty

### Defekt 1: UPDATE-TRUST-001
- **Root Cause:** W `src/gui/app_services.rs` funkcja `trigger_update_install` dobiera asset przez `select_asset_for_platform(&release.assets)` (oparte na rozszerzeniach), zamiast wymagać zweryfikowanego manifestu i wyboru assetu przez `select_manifest_asset`. Funkcja `install_update` przyjmuje surowy `ReleaseAsset` bez gwarancji typu zweryfikowanego manifestu.
- **Symbole:** `app_services::trigger_update_install`, `updater::select_asset_for_platform`, `updater::install_update`, `updater::check_safety_gate`.
- **Platforma:** Windows & Linux.
- **Wpływ:** Użytkownik GUI może pobrać i uruchomić podmianę niezweryfikowanego pakietu, omijając weryfikację kryptograficzną Ed25519.
- **Bezpieczeństwo:** P0 — obejście łańcucha zaufania aktualizacji.
- **Test reprodukcyjny:** Próba wywołania instalacji bez zweryfikowanego manifestu lub z odwołanym kluczem musi być bezwzględnie zablokowana na poziomie typu i wykonania przed etapem pobierania / apply.

### Defekt 2: UPDATE-DATA-001
- **Root Cause:** W `install_via_powershell` (Windows) skrypt wykonuje `Copy-Item -Path "$staging\*" -Destination $baseDir -Recurse -Force`, co bezwarunkowo nadpisuje całą zawartość katalogu `databases/` w instalacji portable (w tym `serviceLOG.db`, `callbook.db`, a w razie błędu paczki także `default_log.db` i `station_config.json`). W Linuksie `install_via_tar_gz` oraz `apply_atomic_installation` również sekwencyjnie nadpisują wszystkie pliki ze stagingu.
- **Symbole:** `updater::install_via_powershell`, `updater::install_via_tar_gz`, `updater::apply_atomic_installation_with_options`.
- **Platforma:** Windows & Linux (ze szczególnym uwzględnieniem trybu portable).
- **Wpływ:** Nadpisanie baz danych referencyjnych lub baz użytkownika w katalogu portable, potencjalna utrata łączności lub konfiguracji stacji.
- **Bezpieczeństwo:** P0 — integralność i nienaruszalność danych użytkownika.
- **Test reprodukcyjny:** Zainstalowana baza `serviceLOG.db`, `callbook.db`, `default_log.db` oraz `station_config.json` w katalogu docelowym po aktualizacji ze stagingu zawierającego nowsze wersje tych plików muszą pozostać w 100% bajtowo identyczne.

---

## 8. Zakres implementacji

- **Dozwolone pliki do modyfikacji:**
  - `src/cloud/updater.rs` — typowany kontrakt `VerifiedReleaseManifest`, `VerifiedReleaseAsset`, funkcja `classify_update_path`, `UpdatePlan`, bezpieczne planowanie i wykonanie apply/rollback dla Windows i Linux bez nadpisywania chronionych danych.
  - `src/gui/app_services.rs` — modyfikacja `trigger_update_check` i `trigger_update_install` tak, aby operowały wyłącznie na typowanym `VerifiedReleaseManifest` i `VerifiedReleaseAsset`, bez bezpośredniego wywołania selekcji po rozszerzeniach.
  - `src/gui/state/updates.rs` — przechowywanie zweryfikowanego stanu aktualizacji w stanie aplikacji.
  - `tests/security_guard.rs` — guardy bezpieczeństwa sprawdzające wymóg manifest-first, odrzucenie odwołanych kluczy, ochronę baz i brak `sudo`.
  - Nowe testy jednostkowe i integracyjne w `src/cloud/updater.rs` oraz dedykowany moduł testowy.
  - Dokumenty w commicie dokumentacyjnym: `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`, `UPDATE_TRUST_AND_DATA_SAFETY_GATE_REPORT.md`.
- **Warunkowo dozwolone pliki:**
  - `src/gui/app_async.rs` oraz `src/gui/changelog.rs` — wyłącznie jeśli zmiana typu `UpdateCheckOutcome` wymaga adaptacji obsługi komunikatów UI (bez zmian wizualnych).
- **Niedozwolone pliki:**
  - `Cargo.toml` (BEZWZGLĘDNIE ZAKAZANE)
  - `Cargo.lock` (BEZWZGLĘDNIE ZAKAZANE)
  - Pliki potoków `.github/workflows/*` (chyba że uzasadnione guardem, preferowany brak zmian)
  - Pliki baz danych produkcyjnych `databases/*.db`
  - Pliki konfiguracyjne użytkownika

---

## 9. Plan commitów

- **COMMIT 1:**
  `fix(updater): require verified manifest for GUI update flow (UPDATE-TRUST-001)`
  - Wprowadzenie typowanego modelu `VerifiedReleaseManifest` oraz `VerifiedReleaseAsset`.
  - Sprawdzenie polityki kluczy (aktywny, nieodwołany, dopasowany key_id).
  - Wymuszenie selekcji assetu ściśle na podstawie zweryfikowanego manifestu (`platform`, `arch`, `package_type`, `filename`, `size`, `sha256`).
  - Podłączenie GUI (`trigger_update_check`, `trigger_update_install`) do kontraktu manifest-first.
  - Usunięcie aktywnego użycia selekcji po rozszerzeniach w ścieżce instalacji.
  - Testy celowane zaufania i weryfikacji manifestu.
- **COMMIT 2:**
  `fix(updater): plan file updates with fail-closed data protection (UPDATE-DATA-001)`
  - Centralna funkcja klasyfikacji ścieżek `classify_update_path(relative_path) -> UpdateFileClass`.
  - Deterministyczny `UpdatePlan` (kategorie: replace, create, preserve, reject).
  - Ochrona fail-closed dla `databases/*.db`, `station_config.json`, `default_log.db` oraz plików nieznanych.
  - Testy determinizmu planu, braku I/O w fazie planowania, ochrony baz.
- **COMMIT 3:**
  `fix(updater): preserve portable data during Windows and Linux update and rollback (UPDATE-DATA-001)`
  - Naprawa generatora PowerShell dla Windows: wyeliminowanie `Copy-Item "$staging\*" -Recurse -Force`, wdrożenie selektywnej podmiany i selektywnego rollbacku opartego na zatwierdzonym planie plików.
  - Naprawa instalatora Linux: weryfikacja praw zapisu, wdrożenie selektywnego swapu i rollbacku opartego na planie plików.
  - Idempotentny rollback pozostawiający bazy i konfigurację użytkownika w 100% bajtowo nienaruszone.
  - Testy temp-dir dla Windows i Linux (bez uprawnień administratora, bez sudo).
- **COMMIT 4:**
  `test(updater): harden trust data rollback and health safety matrix`
  - Rozszerzone testy stabilności (min. 20-100 iteracji kluczowych funkcji).
  - Nowe security guardy w `tests/security_guard.rs`.
  - Pełna weryfikacja braku regresji dla Etapów 1–10.
- **COMMIT 5:**
  `docs(updater): close trust and data safety gate`
  - Utworzenie `UPDATE_TRUST_AND_DATA_SAFETY_GATE_REPORT.md`.
  - Aktualizacja rejestrów i statusów roadmapy.

---

## 10. Warunki zatrzymania

Prace zostaną natychmiast zatrzymane w przypadku:
1. Rozbieżności w historii Git lub braku możliwości wykonania fast-forward.
2. Jakiejkolwiek konieczności modyfikacji `Cargo.toml` lub `Cargo.lock` bądź dodania zewnętrznych zależności.
3. Potrzeby użycia prawdziwego klucza prywatnego wydania.
4. Próby reaktywacji odwołanego klucza `splogbook-release-2026-01`.
5. Wymogu uruchomienia pobranych plików binarnych w testach.
6. Wymogu uprawnień administratora lub `sudo`.
7. Wykrycia niestabilności (flaky test) w testach bezpieczeństwa.
8. Jakiejkolwiek próby dostępu do produkcyjnych plików danych użytkownika.
9. Naruszenia reguł bezpieczeństwa sprzętowego radiostacji (HIL, PTT, RF).
