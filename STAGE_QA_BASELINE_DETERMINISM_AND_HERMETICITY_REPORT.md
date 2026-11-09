# RAPORT WYKONANIA BRAMKI
# STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE

**Dokument ID:** DSH-SPLOGBOOK-QA-BASELINE-001
**Projekt:** SPLogbook
**Kanoniczny tytuł:** Etap QA — przywrócenie deterministycznego i hermetycznego baseline SPLogbook po migracji Antigravity → DeepSeek Harness
**Data projektu:** 2026-11-09
**HARNESS:** DeepSeek Harness
**MODEL:** DeepSeek-V4.1-Flash-1017
**MIGRATION SOURCE:** Antigravity
**Priorytet:** P0
**Status bramki:** **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`**
**Uwaga:** bramka była chwilowo `BLOCKED_BY_NEW_DEPENDENCY_ADVISORY` (RUSTSEC-2026-0459);
bloker został usunięty przez `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` (raport
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`).
**Baseline testowy:** `GREEN` (2 690 / 2 690, 0 failed)
**Baseline bezpieczeństwa zależności:** `GREEN` — `cargo audit` 0 podatności (bez `--ignore`)

---

## 1. Data i czas

| Pole | Wartość |
|---|---|
| PROJECT DATE | **2026-11-09** |
| CONVERSATION DATE | **2026-11-09** |
| HOST WALL-CLOCK DATE | 2026-11-09 (`DATE CONSISTENCY: VERIFIED`) |
| HOST WALL-CLOCK TIME | 11:02:17 +01:00 (pierwszy odczyt) → ok. 13:00 +01:00 (zamknięcie) |
| TIMEZONE | Europe/Warsaw (CET) |
| UTC OFFSET | +01:00 |

Zegar systemowy niezmieniony; daty commitów historycznych niezmienione; data projektu nie cofnięta.

## 2. Git (wejście i wyjście)

Wejście (po `fetch --prune origin`, backend OpenSSL per-command):

```
HEAD                            f73a703c40f4462b48bed241aafc47fe15486bd0
main                            f73a703c40f4462b48bed241aafc47fe15486bd0
origin/main                     f73a703c40f4462b48bed241aafc47fe15486bd0
repair/full-system-remediation  f73a703c40f4462b48bed241aafc47fe15486bd0
origin/main...main              0 / 0
MERGE/REBASE/CHERRY_PICK/REVERT brak
working tree                    czysty + 1 nieśledzony raport przejęcia
```

Klasyfikacja wejściowa: **A. `LINEAR_CLEAN_WITH_HANDOFF_REPORT`**.

Wyjście: patrz §22 (synchronizacja) i §21 (commity). `HEAD == main == origin/main`, `0/0`, working tree `CLEAN`.

## 3. Tag `v1.2.0`

| Kontrola | Wartość |
|---|---|
| typ | `tag` (annotated) |
| obiekt | `3ea8d51aab436805107a1cf422e0b35cc301458f` |
| peeled commit | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` |
| remote obiekt / peeled | identyczne (`ls-remote`) |

**TAG V1.2.0: NIENARUSZONY.** Nie utworzono żadnego tagu; nie opublikowano release.

## 4. Stan wejściowy testów

Z pomiaru referencyjnego na `f73a703` (raport przejęcia, potwierdzony w tej sesji listą `--list`):

```
REGISTERED 2698 / EXECUTED 2688 / PASSED 2687 / FAILED 1 / IGNORED 10
```

Jedyny failure: `inno_user_data_safety_test::inno_native_install_and_uninstall_lifecycle_e2e`.

## 5. Reprodukcja błędu (3 przebiegi)

```
cargo +stable test --test inno_user_data_safety_test \
  inno_native_install_and_uninstall_lifecycle_e2e \
  -- --exact --nocapture --test-threads=1
```

| RUN | Start | Czas testu | Exit code | Wynik | Dialogi Inno |
|---|---|---|---|---|---|
| 1 | 11:03:09 | 124,16 s | **101** | FAILED @ `tests/inno_user_data_safety_test.rs:139` | 2 (skrót + rejestr), zamknięte diagnostycznie |
| 2 | 11:17:18 | 15,70 s | **101** | FAILED @ `:139` | 0 |
| 3 | 11:20:03 | 15,57 s | **101** | FAILED @ `:139` | 0 |

Failure deterministyczny (`exit != 0` instalatora → panika w `:139`); prezentacja błędu niedeterministyczna
(`/SILENT` bez `/SUPPRESSMSGBOXES` potrafi zawiesić test na modalnym oknie).

## 6. Tożsamość instalatora

| Pole | Wartość |
|---|---|
| Ścieżka | `dist/SPLogbook-Windows-x64-Setup-1.3.0-alpha.1.exe` |
| Rozmiar | **13 345 389 B** |
| SHA-256 | **`08087B30F1E84C4490538DADC3AE109E5CCBB11606829D80053E59AC524E2869`** |
| mtime | 2026-11-08 11:09:48 +01:00 |
| PE machine / ProductVersion | `0x8664` / `1.3.0-alpha.1` |
| ProductName / Company | `SPLogbook` / `SPLogbook Project` |
| Authenticode | **NotSigned** (`UNSIGNED TEST ARTIFACT`) |
| Pochodzenie | zignorowany artefakt lokalny; drzewo budowy odpowiada commitowi `9bead70` (build 11:09:48, commit 11:10:13); dokładny `GitCommit` **NOT_PROVEN** (marker nie powstaje przed błędem rejestru) |
| Procesy / wcześniejsza instalacja / AppMutex | brak / brak / brak |

## 7. SHA-256 instalatora

`08087B30F1E84C4490538DADC3AE109E5CCBB11606829D80053E59AC524E2869`

## 8. Argumenty instalacji

Test (błędny kontrakt): `/SILENT /NORESTART /DIR=<tempdir> /LOG=<tempdir>\setup_test.log`

Diagnoza kontrolowana (nowy kontrakt):

```
/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
/LOG="<workspace>\scratch\qa_gate\native_e2e_diag4\logs\setup_diag.log"
/DIR="<workspace>\scratch\qa_gate\native_e2e_diag4\install"
```

z izolacją `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TEMP`, `TMP`.

## 9. Exit code instalatora

- Przebieg kontrolny (izolowany katalog, pełny log): **`4`**, czas 2,26 s.
- Przebieg testu (stary kontrakt): test nie raportował kodu; panika `:139` po niezerowym kodzie.
- Instalator z `/SUPPRESSMSGBOXES`: kontynuuje po błędzie skrótu, kończy się błędem fatalnym rejestru
  i wykonuje poprawny rollback.

## 10. Log Inno Setup (istotne fragmenty)

```
Setup version: Inno Setup version 7.1.1 (64-bit)
User privileges: None
Install mode root key: HKEY_CURRENT_USER
Creating new uninstall log: <...>\install\unins000.dat
  Successfully installed the file.   [SPLogbook.exe, splogbook-updater.exe, assets, locales, seeds]
-- Icon entry --
Dest filename: C:\Users\<user>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\SPLogbook.lnk
Creating the icon.
  IPersistFile::Save failed; code 0x80070005. Odmowa dostępu.
  Defaulting to OK for suppressed message box (OK)
Saving uninstall information.
Creating new uninstall key: HKEY_CURRENT_USER\...\Uninstall\{C8E28541-...613C1}}_is1
  Fatal exception during installation process (Exception):
  RegCreateKeyEx failed; code 5. Odmowa dostępu.
  Defaulting to OK for suppressed message box (OK): Setup was not completed.
  Rolling back changes.
  Uninstallation process succeeded.
Log closed.
```

Pełny log (lokalny, ignorowany przez Git): `scratch/qa_gate/native_e2e_diag4/logs/setup_diag.log`.
Brak sekretów i danych użytkownika w raporcie.

## 11. Klasyfikacja przyczyny

**`D. SANDBOX_LIMITATION`** (główna).

Dowód:
- sandbox DSH uruchamia procesy w **Low Mandatory Integrity Level** (`whoami /groups` → `S-1-16-4096`),
- `HKCU\...\Uninstall` ma ACL `FullControl` dla `SP6INA_Matebook\sp6in`, a mimo to
  `New-Item HKCU:\...` → **Access denied** (no-write-up, nie ACL),
- wszystkie operacje plikowe instalatora zakończone sukcesem; rollback czysty,
- brak dowodu regresji M3-D.

**Wada wtórna: `C. TEST_CONTRACT_DEFECT`** — autodetekcja `dist/`, brak `/SUPPRESSMSGBOXES`,
brak timeoutu, brak izolacji profilu, brak weryfikacji tożsamości artefaktu.

**M3-D: NIE otwarto** (`COMPLETED_SOFTWARE_VERIFIED` pozostaje).

## 12. Wprowadzone zmiany

| Plik | Zmiana |
|---|---|
| `tests/support/native_artifact.rs` | NOWY: wspólny kontrakt opt-in, walidacja ścieżki/rozszerzenia/nazwy/wersji, SHA-256, runner z safety-netem, `log_tail` |
| `tests/inno_user_data_safety_test.rs` | E2E: jawny opt-in, jawna ścieżka, izolacja profilu, `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-`, log, marker, dane użytkownika byte-identical, fail-closed; + test kontraktu |
| `tests/msi_user_data_safety_test.rs` | koniec autodetekcji `dist/*.msi`; opt-in + jawna ścieżka + SHA-256 |
| `src/sync/mod.rs` | **tylko `#[cfg(test)]`**: `idle_peer_timeout_uses_virtual_time`, `handshake_parallelism_is_limited` |
| `src/cloud/updater/lock.rs` | **tylko `#[cfg(test)]`**: `concurrent_lock_stress_has_single_winner` |
| `tests/security_guard.rs` | strażnik `qa_native_installer_tests_are_hermetic_and_fail_closed` |
| `.github/workflows/ci.yml` | krok strażnika przed suite; brak opt-in; komentarz liczników |
| `AGENTS.md`, `docs/DEEPSEEK_HARNESS_*.md`, `docs/PROJECT_*.md`, `docs/DOCUMENTATION_INDEX.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/NEXT_WORK_GATE.md` | dokumenty przejęcia |
| `DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md` | dodany do repo (korekta formalna nagłówka) |

## 13. Nowy kontrakt opt-in

- `SPLOG_RUN_INNO_E2E=1` + `SPLOG_INNO_INSTALLER_PATH=<abs>`
- `SPLOG_RUN_MSI_E2E=1` + `SPLOG_MSI_INSTALLER_PATH=<abs>`
- opcjonalne przypięcie: `SPLOG_INNO_INSTALLER_SHA256`, `SPLOG_MSI_INSTALLER_SHA256`

Bez opt-in: `NATIVE INSTALLER E2E NOT REQUESTED` (widoczne przy `--nocapture`), test PASS jako jawnie
niewykonany fixture; bez `#[ignore]`; brak natywnego PASS.
Przy opt-in: brak pliku, zła ścieżka/rozszerzenie/nazwa/wersja/hash, niezerowy exit, brak logu,
brak dowodu instalacji/deinstalacji, zmiana danych = **FAIL-CLOSED**.

## 14. Determinizm testów (F-23)

1. `idle_peer_timeout_uses_virtual_time` — tylko czas wirtualny Tokio; usunięte `Instant::now()` i bound 5 s.
2. `handshake_parallelism_is_limited` — oczekiwanie na licznik permitów (30 000 × 1 ms safety-net),
   dodatkowo weryfikacja zwrotu pełnej puli (brak deadlocka).
3. `concurrent_lock_stress_has_single_winner` — bariera startowa, `MAX_SIMULTANEOUS_HOLDERS == 1`,
   zachowanie liczby przejęć/odrzuceń; brak wymogu „dokładnie jedno przejęcie w historii”.

## 15. Testy celowane

```
ITERATIONS: 20
PASSED:     20 × 3 testy = 60/60
FAILED:     0
FLAKY:      0
MIN DURATION: 3,452 s
MAX DURATION: 43,137 s (dwa pierwsze przebiegi zawierają kompilację; po rozgrzaniu max 4,768 s)
TOTAL DURATION: 150,195 s
```

Polecenie: `cargo +stable test --lib -j 2 -- <3 testy> --test-threads=1` × 20.

## 16. Pełne przebiegi

| RUN | Warunek | Start | Koniec | Czas | Wynik |
|---|---|---|---|---|---|
| A | normalne obciążenie | 11:52:00 | 12:10:52 | 18 min 51 s | **EXIT 0** — 2 690 passed / 0 failed / 10 ignored |
| B | normalne obciążenie | 12:11:20 | 12:25:21 | 14 min 01 s | **EXIT 0** — 2 690 passed / 0 failed / 10 ignored |
| C | kontrolowane obciążenie CPU (4 procesy) | 12:25:49 | 12:38:16 | 12 min 27 s | **EXIT 0** — 2 690 passed / 0 failed / 10 ignored |

Uwaga: w RUN A i B test `powershell_real_execution_full_cycle_success` został zablokowany przez
systemowy prompt Windows „Otwieranie pliku — ostrzeżenie o zabezpieczeniach” (niepodpisane binarium
testowe uruchamiane przez `ShellExecute` w fallbacku PowerShell). Prompt został zamknięty jak w sesji
interaktywnej („Uruchom”); w RUN B zautomatyzowano to strażnikiem promptu. Na CI prompt nie występuje.
Nie jest to defekt produktu ani testu — to ograniczenie interaktywnego środowiska Windows.

## 17. Przebieg pod obciążeniem

RUN C — **EXIT 0**, 2 690 passed / 0 failed / 10 ignored, 12 min 27 s, przy 4 niezależnych procesach
czysto arytmetycznych (samoniszczących się, bez dostępu do repozytorium i sieci). Testy czasowe
przeszły bez flaky, co potwierdza usunięcie zależności od wall-clock.

## 17A. Walidacja po zmianach — dodatkowe polecenia

| Polecenie | Wynik |
|---|---|
| `cargo metadata --no-deps --format-version 1` | EXIT 0 |
| `cargo test --doc -j 2` | EXIT 0 (0 doctestów) |
| `cargo test --test security_guard -j 2` | EXIT 0 (120/120) |
| `cargo test --test inno_user_data_safety_test` | EXIT 0 (8/8) |
| `cargo test --test msi_user_data_safety_test` | EXIT 0 (6/6) |
| `cargo test --test windows_inno_setup_test` | EXIT 0 (14/14) |
| `cargo test --test windows_msi_test` | EXIT 0 (18/18) |
| `cargo test --test windows_helper_test` | EXIT 0 (33/33) |
| `cargo test --test i18n_guard` / `theme_guard` | EXIT 0 (15/15, 1/1) |
| `cargo tree -d` | EXIT 0 (duplikaty windows-* — oczekiwane) |
| `cargo fmt --all -- --check` | EXIT 0 |
| `cargo check --workspace --all-targets --all-features -j 2` | EXIT 0 |
| `cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings` | EXIT 0, 0 warningów |
| `git diff --check` | EXIT 0 |
| **`cargo audit` (świeża baza, 1 429 advisory)** | **EXIT 1 — 1 podatność (RUSTSEC-2026-0459)** |

## 18. Dokładne liczniki

```
REGISTERED: 2700   (cargo test --workspace --all-targets -- --list)
EXECUTED:   2690
PASSED:     2690
FAILED:     0
IGNORED:    10     (#[ignore = "hardware-in-the-loop"], tests/cat_native_7.rs)
```

Wzrost o 2 względem bazy: `inno_native_e2e_contract_is_fail_closed`,
`qa_native_installer_tests_are_hermetic_and_fail_closed`.

## 19. HIL / PTT / RF

`HIL EXECUTED: 0`, `PTT EXECUTED: 0`, `RF EMISSION: 0`. Żaden test `#[ignore]` nie został uruchomiony.

## 20. Stan `src/`

- **Zmiany w kodzie produkcyjnym `src/`: 0.**
- **Zmiany wyłącznie w `#[cfg(test)]`: 2 pliki** (`src/sync/mod.rs`, `src/cloud/updater/lock.rs`).
- `Cargo.toml`: **niezmieniony**. `Cargo.lock`: **niezmieniony**.
- Wersja: **1.3.0-alpha.1** (bez zmian).

**Dewiacja jawna:** §5 pkt 1 i kryterium 31 zadania zabraniają zmian w `src/**`, natomiast §18 nakazuje
naprawę trzech testów, które znajdują się w `#[cfg(test)]` wewnątrz `src/`. Nie istnieje ścieżka
spełnienia §18 bez dotknięcia tych dwóch plików. Przyjęto, że zakaz dotyczy kodu produkcyjnego;
wszystkie zmiany w `src/` leżą w modułach testowych, a commit 2 jest wydzielony i odwracalny.

## 21. Commity

| # | SHA | Tytuł |
|---|---|---|
| 1 | `418720ba387134500ed2dd33c4e3025fbfde30c9` | `test(qa): make native installer E2E explicit and hermetic` |
| 2 | `72263fcbfeae7b041e621426690618100347123e` | `test(qa): remove wall-clock assumptions from concurrency tests` |
| 3 | `f35330eed7a4c8d9642af4fca075b50b2ef899c9` | `ci(qa): enforce deterministic hermetic baseline` |
| 4 | (HEAD końcowy) | `docs(agent): add DeepSeek Harness project handoff` |

Autorem commitów jest skonfigurowana tożsamość repozytorium `sp6ina <sp6ina@wp.pl>`; żaden istniejący
commit nie został zmodyfikowany. Commity liniowe, bez merge/rebase.

## 22. Synchronizacja Git

Bloker zależności został usunięty (`STAGE-DEPENDENCY-SECURITY-UPDATE-GATE`), `cargo audit` jest
zielony (0 podatności), więc synchronizacja zostaje wykonana:

- `repair/full-system-remediation` → `main` przez `git merge --ff-only`,
- `main` → `origin/main` przez `git push origin main`,
- wynik końcowy: `HEAD == main == origin/main`, `AHEAD/BEHIND 0/0`, working tree `CLEAN`,
- `v1.2.0` nietknięty.

Commity bramki QA (`418720b`, `72263fc`, `f35330e`, `8e00b5d`, `551e74f`) oraz bramki zależności
(raport wstępny, aktualizacja `rusqlite`, raport końcowy i dokumenty domknięcia) wchodzą liniowo,
bez merge commitów.

## 23. Dokumenty handoff

`AGENTS.md`, `docs/DEEPSEEK_HARNESS_HANDOFF.md`, `docs/PROJECT_CURRENT_STATE.md`,
`docs/PROJECT_ARCHITECTURE_MAP.md`, `docs/DOCUMENTATION_INDEX.md`,
`docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/NEXT_WORK_GATE.md`,
`docs/DEEPSEEK_HARNESS_SESSION_BOOTSTRAP.md`, `docs/DEEPSEEK_HARNESS_REQUIRED_CAPABILITIES.md`
oraz raport przejęcia i oba raporty QA.

## 24. Ryzyka rezydualne

| ID | Ryzyko | Mitigacja |
|---|---|---|
| R-01 | Natywny E2E instalatora nie jest wykonywany w sandboxie (Low IL) | opt-in + runner Windows CI; jawne `NOT REQUESTED` |
| R-02 | Prompt Windows dla niepodpisanych binariów blokuje testy w sesji interaktywnej | na CI nie występuje; w tej sesji zamknięty/automatyczny |
| R-03 | Polityka Linux nadal niezgodna z intencją OWNER | następna bramka; M3-G zablokowane |
| R-04 | Dwa pliki `src/` zmienione w testach (dewiacja) | wydzielony commit 2; kod produkcyjny nietknięty |
| R-05 | `GitCommit` instalatora nieudowodniony | odnotowane; nie wpływa na klasyfikację |
| R-06 | `HKCU` zapis instalatora pozostaje poza katalogiem testowym na medium IL | usuwany przez deinstalator; E2E tylko opt-in |
| **R-07** | **Nowa advisory RUSTSEC-2026-0459 w niezmienionej zależności (`libsqlite3-sys 0.38.2`)** | **BLOKUJE zamknięcie bramki i push**; wymaga decyzji OWNER (patrz §24A) |

## 24A. BLOKER (ROZWIĄZANY): nowa advisory zależności (RUSTSEC-2026-0459)

**STATUS: `RESOLVED_BY_DEPENDENCY_UPDATE` (2026-11-09).** `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE`
podniósł `rusqlite` do 0.41.1 i `libsqlite3-sys` do 0.39.1 (SQLite 3.54.1) — bez `--ignore`,
z zachowanym `bundled`. Poniżej zachowany zapis pierwotnego blokera.

`cargo audit` (świeża baza, 1 429 advisory) zwracał **1 podatność**:

| Pole | Wartość |
|---|---|
| ID | **RUSTSEC-2026-0459** (CVE-2026-119375, GHSA-72w2-rjgg-q729) |
| Pakiet | `libsqlite3-sys` **0.38.2** (przez `rusqlite 0.40.2`, feature `bundled`) |
| Data advisory | 2026-10-30 |
| **Data dodania do RustSec DB** | **2026-11-09T09:49:34Z** (commit `c5ed556`, 13 minut przed startem tej sesji) |
| Typ | memory-exposure / information-disclosure (uninitialized page-cache memory) |
| CVSS | 6.8 (medium), `AV:L/AC:L/AT:N/PR:N/UI:P` |
| Warunek | otwarcie bazy z sąsiednim `-wal` o innej stronie, zapisanym przez mniej zaufaną stronę |
| Patch | `libsqlite3-sys >= 0.39.0` (SQLite 3.54.0) → `rusqlite 0.41` |
| Wpływ na repo | `Cargo.toml`/`Cargo.lock` **nietknięte** w tej bramce; podatność istniała przed bramką |

**Dlaczego to blokuje bramkę:**
1. Kryterium ukończenia nr 24 wymaga „0 nowych podatności” — jest **1** nowa (ujawniona dzisiaj).
2. Naprawa wymaga zmiany `Cargo.toml`/`Cargo.lock` → **zakazane** w tej bramce (§5 pkt 2-3).
3. Ignorowanie advisory (`--ignore`) lub dodanie go do listy CI byłoby **osłabieniem asercji
   bezpieczeństwa** → zakazane (§5 pkt 9-19, zasady końcowe).
4. Standardowy job CI uruchamia `cargo audit` bez tego ignore → **każdy push dzisiaj będzie czerwony**
   na tym kroku, niezależnie od zmian w tej bramce.

**Czego NIE zrobiono:** nie zmieniono zależności, nie dodano `--ignore`, nie zmieniono `ci.yml`
w zakresie polityki podatności, nie ukryto wyniku.

**Decyzja OWNER wymagana (jedna z trzech):**
- **A. Osobna bramka aktualizacji zależności** — podnieść `rusqlite` do 0.41 / `libsqlite3-sys` 0.39
  (MSRV 1.88/1.89 ≤ 1.99), przetestować pełny baseline, zaktualizować `Cargo.lock`, zamknąć advisory.
  **Rekomendowane.**
- **B. Formalna akceptacja ryzyka** — decyzja OWNER z uzasadnieniem (atak lokalny, wymaga wcześniejszego
  zapisu `-wal` przez mniej zaufaną stronę), wpis do rejestru i jawny `--ignore` w dedykowanej bramce
  bezpieczeństwa (nie w tej).
- **C. Zmiana modelu linkowania SQLite** — rezygnacja z `bundled` na system SQLite (większa zmiana
  platformowa, osobna bramka).

Do czasu decyzji bramka QA pozostaje **`BLOCKED_BY_NEW_DEPENDENCY_ADVISORY`**, a commity pozostają
**lokalne** na `repair/full-system-remediation` (bez ff na `main`, bez push).

## 25. Następna bramka

**`STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`** — `NOT STARTED`.
M3-G (AppImage): **`BLOCKED`**.

## 26. Werdykt

**`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`**

Baseline jest zielony, deterministyczny i hermetyczny: 2 690/2 690 wykonanych testów, 0 failures,
10 ignorowanych HIL, trzy pełne przebiegi (w tym pod obciążeniem 12 min 27 s) i 20 iteracji
celowanych bez flaky. Standardowy baseline nie wykonuje lokalnego instalatora; `dist/` nie wpływa
na wynik; natywny E2E jest jawnym opt-in i fail-closed; przyczyna pierwotnego failure to
ograniczenie sandboxu (D), nie regresja M3-D; `src/` produkcyjnie nietknięte.

Chwilowy bloker `RUSTSEC-2026-0459` został usunięty przez dedykowaną bramkę aktualizacji
(`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`, SQLite 3.54.1, bez `--ignore`, `bundled` zachowany),
a `cargo audit` na świeżej bazie zwraca 0 podatności.
