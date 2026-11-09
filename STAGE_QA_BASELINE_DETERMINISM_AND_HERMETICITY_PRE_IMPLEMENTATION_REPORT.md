# RAPORT PRZEDIMPLEMENTACYJNY
# STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE

**Projekt:** SPLogbook
**Dokument ID:** DSH-SPLOGBOOK-QA-BASELINE-PRE-001
**Bramka:** `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE`
**Typ:** baseline regression diagnosis + installer test hermeticity + native installer failure analysis + time-based test determinism + CI contract hardening
**Priorytet:** P0 — krytyczny
**Status wejściowy bramki:** `BLOCKED_BY_BASELINE_REGRESSION`
**HARNESS:** DeepSeek Harness
**MODEL:** DeepSeek-V4.1-Flash-1017
**MIGRATION SOURCE:** Antigravity
**Rola:** Główny architekt techniczny, audytor jakości testów, specjalista Windows / Inno Setup / GitHub Actions

---

## 1. Data i czas

| Pole | Wartość |
|---|---|
| PROJECT DATE | **2026-11-09** |
| CONVERSATION DATE | **2026-11-09** |
| HOST WALL-CLOCK DATE | **2026-11-09** |
| HOST WALL-CLOCK TIME | **11:02:17 +01:00** (pierwszy odczyt); raport zamykany ok. 11:23 +01:00 |
| TIMEZONE | Central European Standard Time / `Europe/Warsaw` |
| UTC OFFSET | `+01:00` |
| DATE CONSISTENCY | **VERIFIED** (host = project date; data nie została cofnięta) |

Zegar systemowy nie został zmieniony. Daty istniejących commitów nie zostały zmodyfikowane.

---

## 2. Bramka Git (stan faktyczny, po fetch)

Wykonano `git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false fetch --prune origin` → **EXIT 0**.
Backend OpenSSL użyty per-command (schannel w sandboxie DSH nie pozyskuje poświadczeń); konfiguracja globalna Git **nie została zmieniona**.

| Kontrola | Wartość |
|---|---|
| `git rev-parse --show-toplevel` | `C:/Users/sp6in/Documents/SPLogbook/SPLogbook` |
| `git branch --show-current` | **`main`** (raport wejściowy podawał `repair/full-system-remediation`; obie gałęzie wskazują ten sam commit) |
| `git rev-parse HEAD` | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `git rev-parse main` | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `git rev-parse origin/main` | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `git rev-parse repair/full-system-remediation` | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `origin/main...main` | **0 / 0** |
| `origin/main...repair/full-system-remediation` | **0 / 0** |
| `git merge-base main origin/main` | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `MERGE_HEAD` / `REBASE_HEAD` / `CHERRY_PICK_HEAD` / `REVERT_HEAD` | **brak** (żadna operacja Git nie jest aktywna) |
| `git ls-files -o --exclude-standard` | wyłącznie `DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md` |
| `git status --porcelain=v1` (fsmonitor/untrackedCache off) | `?? DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md` |

**Klasyfikacja wejściowa: A. `LINEAR_CLEAN_WITH_HANDOFF_REPORT`.**
Historia liniowa, brak zmian śledzonych, dokładnie jeden nieśledzony raport przejęcia (zgodny z intencją).

### 2.1 Ochrona tagu `v1.2.0`

| Kontrola | Wartość | Zgodność |
|---|---|---|
| `git cat-file -t refs/tags/v1.2.0` | `tag` | ✅ |
| tag object | `3ea8d51aab436805107a1cf422e0b35cc301458f` | ✅ |
| peeled commit | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` | ✅ |
| `ls-remote` tag object | `3ea8d51aab436805107a1cf422e0b35cc301458f` | ✅ |
| `ls-remote` peeled | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` | ✅ |

**TAG V1.2.0: VERIFIED — brak findingu P0.** Tag nie będzie modyfikowany.

---

## 3. Tożsamość nieudanego testu

| Pole | Wartość |
|---|---|
| Test | `inno_user_data_safety_test::inno_native_install_and_uninstall_lifecycle_e2e` |
| Plik | `tests/inno_user_data_safety_test.rs:120-200` |
| Assertion | `tests/inno_user_data_safety_test.rs:139` — `assert!(status.success(), "Instalator musi zakończyć się kodem 0")` |
| Mechanizm wejściowy | `get_setup_exe_path()` (`:9-12`) autodetekuje `dist/SPLogbook-Windows-x64-Setup-1.3.0-alpha.1.exe`; przy braku pliku `return` (cichy skip) |
| Kontrakt wywołania | `/SILENT /NORESTART /DIR=<tempdir> /LOG=<tempdir>\setup_test.log` |
| Brak w kontrakcie | `/VERYSILENT`, `/SUPPRESSMSGBOXES`, `/SP-`, timeout procesu, izolacja `USERPROFILE`/`APPDATA`, jawna ścieżka artefaktu, weryfikacja wersji/hash |
| Efekt uboczny poza sandboxem | instalator tworzy skrót w **rzeczywistym** Menu Start i wpis odinstalowania w `HKCU` (test nie jest hermetyczny) |

---

## 4. Tożsamość lokalnego instalatora

| Pole | Wartość |
|---|---|
| Ścieżka | `dist/SPLogbook-Windows-x64-Setup-1.3.0-alpha.1.exe` |
| Istnieje / zwykły plik | TAK / TAK |
| Rozmiar | **13 345 389 B** |
| SHA-256 | **`08087B30F1E84C4490538DADC3AE109E5CCBB11606829D80053E59AC524E2869`** |
| Timestamp (mtime) | **2026-11-08 11:09:48 +01:00** (creation 11:09:38) |
| PE machine | `0x8664` (x64) |
| ProductName | `SPLogbook` |
| ProductVersion | `1.3.0-alpha.1` |
| FileVersion | *(pusty)* |
| CompanyName | `SPLogbook Project` |
| FileDescription | `SPLogbook Setup` |
| Comments | `This installation was built with Inno Setup.` |
| Authenticode | **NotSigned** (UNSIGNED TEST ARTIFACT) |
| Wydawca | brak podpisu → brak wydawcy |
| Zablokowany | NIE (brak blokady pliku, brak strefy MOTW) |
| Proces SPLogbook / updatera | BRAK |
| Wcześniejsza instalacja (uninstall entry HKCU/HKLM) | BRAK |
| AppMutex | brak aktywnego mutexu (brak procesu) |
| Pochodzenie z bieżącego HEAD | **NIE UDOWODNIONE** — artefakt jest zignorowany przez Git i zbudowany 2026-11-08 11:09:48, tuż przed commitem `9bead70` (`chore(installer): add approved Windows User Setup definition (M3-D)`, 11:10:13). `GitCommit` jest wkompilowany w skrypt Inno, ale marker zapisywany jest dopiero w `ssPostInstall`, do którego instalacja w tym środowisku nie dochodzi → dokładny commit budowy: `NOT_PROVEN`. |
| Wersja w instalatorze | `1.3.0-alpha.1` (PE `ProductVersion`; zgodna z `Cargo.toml:3`) |

**Kontrolowana diagnoza (dowód):** uruchomienie instalatora poza testem, w izolowanym katalogu, z pełnym logiem:

```
/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
/LOG="<workspace>\scratch\qa_gate\native_e2e_diag4\logs\setup_diag.log"
/DIR="<workspace>\scratch\qa_gate\native_e2e_diag4\install"
```

Z zachowaniem `USERPROFILE` (patrz §5.2) — **exit code instalatora: `4`**, czas 2,26 s.

---

## 5. Reprodukcja nieudanego testu (3 przebiegi, identyczna komenda)

```
cargo +stable test --test inno_user_data_safety_test \
    inno_native_install_and_uninstall_lifecycle_e2e \
    -- --exact --nocapture --test-threads=1
```

| Przebieg | Start (host) | Koniec | Czas testu | Exit code | Dialogi Inno | Wynik |
|---|---|---|---|---|---|---|
| RUN 1 | 2026-11-09 11:03:09 | 11:05:37 | **124,16 s** | **101** | 2 (skrót + rejestr), zamknięte diagnostycznie | FAILED @ `:139` |
| RUN 2 | 2026-11-09 11:17:18 | 11:20:02 | **15,70 s** | **101** | 0 (brak prezentacji) | FAILED @ `:139` |
| RUN 3 | 2026-11-09 11:20:03 | 11:22:18 | **15,57 s** | **101** | 0 (brak prezentacji) | FAILED @ `:139` |

Wszystkie trzy przebiegi: `0 passed; 1 failed; 6 filtered out`.
Wniosek z reprodukcji: **failure jest deterministyczny** (zawsze `exit != 0` instalatora → panika w `:139`), natomiast **prezentacja błędu jest niedeterministyczna** (message boxy Inno w trybie `/SILENT` potrafią zablokować proces bezterminowo — RUN 1 — albo zostać automatycznie pominięte — RUN 2/3). To dowód wady kontraktu testu (brak `/SUPPRESSMSGBOXES` i timeoutu), niezależnie od przyczyny exit code.

---

## 6. Pełny log Inno Setup — istotne fragmenty

Log z przebiegu kontrolnego (`native_e2e_diag4`), bez danych osobowych poza ścieżką profilu testowego:

```
Setup version: Inno Setup version 7.1.1 (64-bit)
User privileges: None
Administrative install mode: No
Install mode root key: HKEY_CURRENT_USER
64-bit install mode: Yes
Directory for uninstall files: <...>\native_e2e_diag4\install
Creating new uninstall log: <...>\install\unins000.dat
  ... (wszystkie pliki SPLogbook.exe, splogbook-updater.exe, assets, locales, seed DB)
  Successfully installed the file.   [dla każdego pliku allowlisty]
-- Icon entry --
Dest filename: C:\Users\<user>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\SPLogbook.lnk
Creating the icon.
  Exception message:
  Defaulting to OK for suppressed message box (OK):
      IPersistFile::Save failed; code 0x80070005.
      Odmowa dostępu.
Saving uninstall information.
Creating new uninstall key: HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}}_is1
  Fatal exception during installation process (Exception):
      Error creating registry key: ..._is1
      RegCreateKeyEx failed; code 5.
      Odmowa dostępu.
  Defaulting to OK for suppressed message box (OK): Setup was not completed.
  Rolling back changes.
  ... (rollback usuwa pliki programu)
  Uninstallation process succeeded.
Log closed.
```

Po rollbacku w katalogu instalacji pozostają wyłącznie pliki seed (`databases/callbook.db`, `serviceLOG.db`, `clubs.csv`) — zgodnie z `onlyifdoesntexist uninsneveruninstall`. Marker `.splogbook-inno-marker` **nie powstał**, `unins000.exe` został usunięty przez rollback, wpis rejestru **nie powstał**.

### 6.1 Ustalenie przyczyny — dowód środowiskowy

Sandbox DSH uruchamia procesy w **Low Mandatory Integrity Level** (`whoami /groups` → `S-1-16-4096`). Obiekty `HKCU` i Menu Start są Medium IL. ACL jawnie przyznaje `SP6INA_Matebook\sp6in` `FullControl` na `HKCU\...\Uninstall`, a mimo to:

```
New-Item HKCU:\Software\SPLogbookSandboxProbe           → Access denied
New-Item HKCU:\...\Uninstall\SPLogbookSandboxProbe      → Access denied
```

Jest to blokada integrity-level (no-write-up), a **nie** ACL, **nie** brak uprawnień konta, **nie** defekt instalatora.

Dodatkowo potwierdzono eksperymentalnie, że po przekierowaniu `USERPROFILE`/`APPDATA` instalator tworzy skrót **w izolowanym profilu testowym** (`<TEST_HOME>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\SPLogbook.lnk`) — czyli cała warstwa plikowa jest hermetyzowalna; jedynym niehermetyzowalnym elementem pozostaje zapis do `HKCU` (rejestr nie podlega przekierowaniu per-proces bez wirtualizacji).

### 6.2 Klasyfikacja (jedna główna)

**`D. SANDBOX_LIMITATION`**

Uzasadnienie:
- wszystkie operacje plikowe instalatora zakończyły się sukcesem (pliki allowlisty, poprawne rozmiary),
- pierwszy błąd to zapis skrótu poza workspace (denied przez Low IL),
- błąd fatalny to `RegCreateKeyEx` do `HKCU` (denied przez Low IL, ACL = FullControl),
- rollback zadziałał poprawnie i usunął pliki programu,
- brak jakiegokolwiek dowodu regresji M3-D (E).

**Wada wtórna (nie główna): `C. TEST_CONTRACT_DEFECT`** — test:
1. autodetekuje artefakt z `dist/` (naruszenie hermetyczności),
2. używa `/SILENT` bez `/SUPPRESSMSGBOXES` → błąd instalatora może zawiesić test bezterminowo (RUN 1: 124 s i oczekiwanie na kliknięcie),
3. nie ustawia timeoutu procesu,
4. nie izoluje `USERPROFILE`/`APPDATA` (w normalnym środowisku tworzy realny skrót w Menu Start),
5. nie weryfikuje wersji, tożsamości ani SHA-256 artefaktu.

**M3-D: NIE zostaje otwarte** (`COMPLETED_SOFTWARE_VERIFIED` pozostaje w mocy).
**Uwaga poboczna (P3, poza zakresem bramki):** log Inno pokazuje klucz `...{C8E28541-...613C1}}_is1` z podwójnym `}`; wymaga odrębnego potwierdzenia, czy `AppId` jest poprawnie escapowany (nie naprawiane w tej bramce).

---

## 7. Testy zależne od czasu (F-23) — semantyka obecna i docelowa

| # | Test | Lokalizacja | Obecna semantyka (wadliwa) | Docelowa semantyka deterministyczna |
|---|---|---|---|---|
| 1 | `sync::tests::idle_peer_timeout_uses_virtual_time` | `src/sync/mod.rs:1344-1406` | mierzy `std::time::Instant` i wymaga `wall_elapsed < 5 s` — czysty wall-clock | wyłącznie czas wirtualny Tokio (`start_paused`), jawny `advance(IDLE_TIMEOUT+1ms)`, asercja `virt_elapsed >= IDLE_TIMEOUT` i górny bound **wirtualny**; zero `Instant::now()` w kryterium PASS |
| 2 | `sync::tests::handshake_parallelism_is_limited` | `src/sync/mod.rs:952-991` | otwiera 5 połączeń i czeka maks. 1 s (50 × 20 ms) aż `available_handshake_permits()==0`; pod obciążeniem limit nie zostaje osiągnięty → fałszywy FAIL | jawny mechanizm gotowości: oczekiwanie na **licznik atomowy** `available_handshake_permits()` z kontrolowanym budżetem iteracji (safety-net, nie kryterium wydajności); asercje: limit osiągnięty, brak przekroczenia, zwolnienie permita po zamknięciu połączenia, brak deadlocka |
| 3 | `cloud::updater::lock::tests::concurrent_lock_stress_has_single_winner` | `src/cloud/updater/lock.rs:389-426` | wymaga `winners == 1` w całej historii; zwycięzca zwalnia blokadę po 50 ms, więc drugi wątek może legalnie przejąć → `winners == 2` | niezmiennik **AT MOST ONE SIMULTANEOUS HOLDER**: licznik bieżących posiadaczy + `max_simultaneous_holders == 1`, liczba przejęć ≥ 1, suma przejęć i odrzuceń = liczba wątków; brak zakazu legalnego ponownego przejęcia |

**Uwaga o zakresie:** testy 1-3 są modułami `#[cfg(test)]` **wewnątrz `src/`** (`src/sync/mod.rs`, `src/cloud/updater/lock.rs`). Nie istnieją ich odpowiedniki w `tests/`. Remediacja F-23 wymaga więc edycji **wyłącznie bloków testowych**, bez zmiany jednej linii kodu produkcyjnego. Patrz §10 (dewiacja jawna).

---

## 8. Pliki do zmiany

| Plik | Rodzaj zmiany | Commit |
|---|---|---|
| `tests/support/native_artifact.rs` | NOWY — wspólny, fail-closed kontrakt opt-in + SHA-256 + runner z timeoutem | 1 |
| `tests/inno_user_data_safety_test.rs` | przepisanie E2E na opt-in/jawną ścieżkę/izolację/timeout/marker; + testy kontraktu fail-closed | 1 |
| `tests/msi_user_data_safety_test.rs` | usunięcie autodetekcji `dist/*.msi`, opt-in + jawna ścieżka + SHA-256 | 1 |
| `src/sync/mod.rs` | **tylko `#[cfg(test)]`** — 2 testy | 2 |
| `src/cloud/updater/lock.rs` | **tylko `#[cfg(test)]`** — 1 test | 2 |
| `tests/security_guard.rs` | nowy strażnik hermetyczności (statyczny + behawioralny) | 3 |
| `.github/workflows/ci.yml` | krok strażnika + komentarz liczników; bez opt-in w standardowym jobie | 3 |
| `AGENTS.md`, `docs/DEEPSEEK_HARNESS_*.md`, `docs/PROJECT_*.md`, `docs/DOCUMENTATION_INDEX.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/NEXT_WORK_GATE.md` | NOWE dokumenty przejęcia | 4 |
| `DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md` | dodanie do repo + korekta formalna nagłówka | 4 |
| `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_PRE_IMPLEMENTATION_REPORT.md` / `..._REPORT.md` | raporty bramki | 4 |

**Nie zmieniamy:** `src/**` poza blokami `#[cfg(test)]`, `Cargo.toml`, `Cargo.lock`, `installer/**`, `build.rs`, wersji `1.3.0-alpha.1`, `README.md` (poza ewentualnym wskaźnikiem — nie planowane), `.github/workflows/build-and-release.yml` (bez zmian), tagów, release'ów.

---

## 9. Kontrakt natywnego E2E (projekt)

### 9.1 Opt-in

- `SPLOG_RUN_INNO_E2E=1` — uruchomienie natywnego cyklu Inno Setup,
- `SPLOG_INNO_INSTALLER_PATH=<ścieżka absolutna>` — jawny artefakt,
- `SPLOG_RUN_MSI_E2E=1` + `SPLOG_MSI_INSTALLER_PATH=<ścieżka absolutna>` — kontrola artefaktu MSI (model inny niż E2E wykonawczy, stąd osobne zmienne; uzasadnione).

Gdy opt-in wyłączony: test **nie wykonuje** instalatora, wypisuje `NATIVE INSTALLER E2E NOT REQUESTED` (widoczne przy `--nocapture`) i kończy się PASS jako jawnie niewykonany fixture. **Nie** używamy `#[ignore]` (kanoniczna liczba 10 HIL pozostaje bez zmian), **nie** traktujemy braku opt-in jako natywnego PASS.

### 9.2 Fail-closed (gdy opt-in włączony)

- brak zmiennej ścieżki → FAIL,
- ścieżka względna / nieabsolutna → FAIL,
- plik nie istnieje / nie jest regular file / jest symlinkiem → FAIL,
- złe rozszerzenie → FAIL,
- nazwa pliku niezgodna z `SPLogbook-Windows-x64-Setup-<CARGO_PKG_VERSION>.exe` → FAIL,
- brak/niezerowy exit code → FAIL,
- brak logu, brak dowodu instalacji, brak dowodu deinstalacji → FAIL,
- zmiana danych użytkownika (byte-identical) → FAIL.

### 9.3 Hermetyczność wykonania

- `tempfile::tempdir()` jako `TEST_ROOT`,
- `TEST_INSTALL_DIR`, `TEST_LOG_DIR`, `TEST_HOME`, `TEST_APPDATA`, `TEST_LOCALAPPDATA`, `TEST_TEMP` **wewnątrz** `TEST_ROOT`,
- proces instalatora dostaje `USERPROFILE=TEST_HOME`, `APPDATA=TEST_HOME\AppData\Roaming`, `LOCALAPPDATA=...`, `TEMP`/`TMP` → dowiedziono, że `{userprograms}` trafia do izolowanego profilu,
- argumenty: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /LOG=<abs> /DIR=<abs>`,
- timeout procesu (safety-net, nie kryterium PASS),
- SHA-256 artefaktu liczony i raportowany; opcjonalna asercja `SPLOG_*_SHA256`,
- brak globów, brak `|| true`, brak ignorowania exit code.

---

## 10. Potwierdzenie stanu `src/` i dewiacja jawna

**Przed rozpoczęciem zmian:** `git diff --stat -- src` = **0 plików, 0 linii**; `src/` jest czyste.

**Dewiacja jawna (konflikt wymagań):** §5 pkt 1 i kryterium 31 zabraniają zmian w `src/**`, natomiast §18 nakazuje poprawić trzy testy deterministyczne, które **znajdują się w `#[cfg(test)]` wewnątrz `src/`**. Nie istnieje ścieżka spełnienia §18 bez dotknięcia tych dwóch plików. Przyjęta interpretacja: zakaz dotyczy **kodu produkcyjnego**; zmieniane będą wyłącznie bloki `#[cfg(test)]`.

Zobowiązania:
- zero zmian w kodzie produkcyjnym (`#[cfg(not(test))]` / kod wykonywalny),
- `git diff` dołączony do raportu końcowego z zaznaczeniem, że wszystkie zmiany w `src/` leżą w modułach testowych,
- jeśli OWNER uzna to za naruszenie, commit 2 jest wydzielony i odwracalny bez wpływu na commity 1, 3, 4.

W raporcie końcowym raportowane będą **dwie** liczby: `PRODUCTION CODE CHANGES IN src/` = **0** oraz `TEST-ONLY CHANGES IN src/` = **2 pliki**.

---

## 11. Kanoniczne liczniki (punkt wyjścia)

Liczniki zostaną ponownie zmierzone poleceniem `cargo +stable test --workspace --all-targets -- --list` **przed** zmianami i **po** zmianach (nie na podstawie nazw plików).

Oczekiwany punkt odniesienia (raport przejęcia, do potwierdzenia pomiarem):

```
REGISTERED: 2698
EXECUTED:   2688
PASSED:     2687
FAILED:     1
IGNORED:    10  (wyłącznie #[ignore = "hardware-in-the-loop"] w tests/cat_native_7.rs)
```

Wymagania po bramce: `FAILED = 0`, `IGNORED HIL = 10`, `NEW IGNORED = 0`, `HIL EXECUTED = 0`.
Dodanie testów kontraktu fail-closed oraz strażnika hermetyczności może zwiększyć `REGISTERED`/`EXECUTED` — jest dozwolone i raportowane.

---

## 12. Plan commitów

1. `test(qa): make native installer E2E explicit and hermetic`
   — `tests/support/native_artifact.rs`, `tests/inno_user_data_safety_test.rs`, `tests/msi_user_data_safety_test.rs`, oba raporty QA (wstępny).
2. `test(qa): remove wall-clock assumptions from concurrency tests`
   — wyłącznie `#[cfg(test)]` w `src/sync/mod.rs` i `src/cloud/updater/lock.rs`.
3. `ci(qa): enforce deterministic hermetic baseline`
   — `tests/security_guard.rs`, `.github/workflows/ci.yml`.
4. `docs(agent): add DeepSeek Harness project handoff`
   — `AGENTS.md`, `docs/DEEPSEEK_HARNESS_*`, `docs/PROJECT_*`, `docs/DOCUMENTATION_INDEX.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/NEXT_WORK_GATE.md`, raport przejęcia, raport końcowy QA.

Po każdym commicie: `fmt --check`, `check --all-targets --all-features`, `clippy -D warnings`, pełny `cargo test`, `security_guard`, `git diff --check`, `git status --porcelain=v1`.

---

## 13. Kryteria ukończenia

Bramka może otrzymać `COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED` wyłącznie gdy:

1. Git liniowy; 2. working tree czyste; 3. `v1.2.0` nietknięty; 4. wersja `1.3.0-alpha.1`;
5. przyczyna nieudanego testu sklasyfikowana (`D. SANDBOX_LIMITATION`); 6. pełny log bez sekretów zachowany;
7. standardowy baseline nie wykonuje lokalnego instalatora; 8. obecność `dist/*.exe` nie zmienia wyniku;
9. obecność `dist/*.msi` nie zmienia wyniku; 10. natywny E2E wymaga opt-in; 11. wymaga jawnej ścieżki;
12. brak artefaktu przy opt-in = FAIL; 13. niezerowy exit code = FAIL; 14. brak produkcyjnego `APPDATA`;
15. brak produkcyjnego katalogu instalacji; 16-17. trzy testy czasowe deterministyczne i niezmiennik
„max 1 jednoczesny posiadacz”; 18. trzy pełne przebiegi 0 failures; 19. przebieg pod obciążeniem 0 failures;
20. 20 iteracji celowanych 0 flaky; 21-26. fmt/check/clippy/audit/doc/security guards zielone;
27-29. HIL/PTT/RF = 0; 30. ignored HIL = 10; 31-32. `src/` produkcyjnie i Cargo nietknięte;
33-34. raport przejęcia i dokumenty DSH w repo; 35-36. brak polityki Linux i M3-G; 37-38. brak release/taga;
39. `main == origin/main`.

---

## 14. Warunki zatrzymania

Zatrzymanie i raport `BLOCKED_*` nastąpi, jeżeli: Git rozbieżny; nieoczekiwane zmiany śledzone; aktywna operacja Git; `origin/main` ruszy się równolegle; tag `v1.2.0` inny; nie da się ustalić tożsamości instalatora; test dotknie produkcyjnych danych; instalator zmodyfikuje realną instalację użytkownika; log ujawni realny defekt instalatora (→ `REOPENED_BY_CONFIRMED_REGRESSION`); konieczna zmiana `src/` produkcyjnie / `Cargo.toml` / `Cargo.lock`; konieczne HIL/PTT/RF; test pozostaje flaky; trzy pełne przebiegi nie są zielone; konieczne maskowanie exit code, osłabienie asercji bezpieczeństwa, rebase, zwykły merge, force-push, publikacja lub tag.

W obecnym stanie **żaden** z warunków zatrzymania nie zachodzi: przyczyna jest udowodniona jako ograniczenie sandboxu (D), a nie regresja M3-D.

---

*Raport sporządzono przed modyfikacją jakiegokolwiek testu lub workflow, zgodnie z §10 zadania.*
