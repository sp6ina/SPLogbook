# RAPORT ZAMKNIĘCIA I WERYFIKACJI KOŃCOWEJ BRAMKI: STAGE-M3-B-CLOSURE-VERIFICATION-GATE

**Projekt:** SPLogbook  
**Identyfikator bramki:** `STAGE-M3-B-CLOSURE-VERIFICATION-GATE`  
**Kanoniczny tytuł:** Końcowa weryfikacja tożsamości procesów, semantyki podmiany plików, testów natywnych Windows i formalnego zamknięcia M3-B  
**Rodzaj zadania:** VERIFICATION + GAP REMEDIATION + TESTS + DOCUMENTATION  
**Priorytet:** P0 — KRYTYCZNY  
**Data robocza projektu:** 2026-11-07  
**Data środowiska:** 2026-11-07  
**Czas środowiska:** 18:42:39 CET (UTC+01:00)  
**Timezone:** Central European Standard Time (UTC+01:00)  
**Status końcowy bramki:** `COMPLETED_SOFTWARE_VERIFIED`  

---

## 1. Metadane Bramki i Środowisko

- **Identyfikator bramki:** `STAGE-M3-B-CLOSURE-VERIFICATION-GATE`
- **Tytuł kanoniczny:** Końcowa weryfikacja tożsamości procesów, semantyki podmiany plików, testów natywnych Windows i formalnego zamknięcia M3-B
- **Data robocza projektu:** `2026-11-07`
- **Rzeczywista data środowiska:** `2026-11-07`
- **Rzeczywisty czas środowiska:** `18:42:39+01:00`
- **Strefa czasowa środowiska:** `Central European Standard Time (UTC+01:00)`
- **Zgodność zegara:** Brak przesunięć kalendarzowych, brak manipulacji znacznikami czasu w Git.
- **Status bramki:** `COMPLETED_SOFTWARE_VERIFIED`

---

## 2. Podsumowanie Wykonawcze

Bramka weryfikacyjna `STAGE-M3-B-CLOSURE-VERIFICATION-GATE` przeprowadziła niezależną, rygorystyczną weryfikację techniczną całości implementacji etapu M3-B (pomocnicy Windows `splogbook-updater.exe` oraz utwardzony fallback PowerShell).

W ramach bramki:
1. **Zweryfikowano tożsamość i cykl życia procesów:**
   - Potwierdzono bezwzględny zakaz zabijania procesu macierzystego SPLogbook (`parent never killed`). Monitor procesu nadrzędnego wyłącznie oczekuje na jego dobrowolne zakończenie; po upływie timeoutu następuje bezpieczny fail-closed (`ParentWaitTimeout`) bez dotykania plików instalacji.
   - Wykluczono podatność na ponowne użycie PID przez weryfikację nazwy obrazu procesu nadrzędnego.
   - Zarządzanie procesem kandydata zrealizowano wyłącznie przez bezpośredni uchwyt potomka `HelperCandidateChild` (zakaz wyszukiwania i zabijania po nazwie obrazu lub PID-only).
2. **Zweryfikowano i sprecyzowano semantykę podmiany plików:**
   - Wyeliminowano fałszywe określenia typu "atomowe kopiowanie";
   - Formalnie sklasyfikowano i udowodniono semantykę jako:
     * `JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED`
     * `CRASH_RECOVERABLE_BY_JOURNAL`
     * `NOT_POWER_LOSS_ATOMIC`
   - Udowodniono odporność na awarie w każdym punkcie dziennika operacji z bezpiecznym zachowaniem kopii zapasowej `.rollback` (decyzja OWNER-M2-08/09).
3. **Zapewniono kompletną macierz testów natywnych Windows i PowerShell:**
   - Zwiększono zestaw testów w `tests/windows_helper_test.rs` z 9 do **28 testów** (100% PASS).
   - Zwiększono zestaw strażników w `tests/security_guard.rs` z 82 do **83 testów** (100% PASS).
   - Wdrożono i zweryfikowano rzeczywiste wykonanie kodu PowerShell przez natywny silnik `powershell.exe` na systemie Windows (w tym test pełnego cyklu oraz test rollbacku przy awarii kandydata).
   - Usunięto podatności platformowe PowerShell 5.1 (tokenizacja wielobajtowych znaków UTF-8 bez BOM w kodowaniu ANSI Windows-1250, cudzysłowy w argumentach `Start-Process`, obsługa BOM w JSON).
4. **Uzgodniono hierarchię i roadmapę M3:**
   - Potwierdzono, że **M3-C to wyłącznie integracja Windows Portable ZIP** (`STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE`);
   - Potwierdzono, że pakiety Linux (tar.gz, AppImage, DEB, RPM, Flatpak) stanowią dalsze etapy M3 (M3-F..M3-J) i nie wchodzą w zakres M3-C;
   - Potwierdzono, że implementacja M3-C NIE została rozpoczęta w tej bramce.

---

## 3. Tożsamość Procesów i Cykl Życia

### A. Monitor Procesu Macierzystego (SPLogbook)
- **Mechanizm oczekiwania:** Helper otrzymuje PID procesu macierzystego w podpisanym i zweryfikowanym kryptograficznie obiekcie `WindowsHelperRequest`.
- **Zakaz zabijania procesu nadrzędnego:** Helper pod żadnym pozorem nie wywołuje `TerminateProcess`, `process.kill()` ani żadnych poleceń powłoki typu `taskkill`.
- **Weryfikacja tożsamości PID:** Helper sprawdza tożsamość procesu przez otwarcie uchwytu `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION` i potwierdzenie nazwy obrazu (`splogbook.exe`). Zapobiega to przypadkowemu oczekiwaniu na obcy proces w razie ponownego użycia PID przez system operacyjny.
- **Obsługa przekroczenia limitu czasu (Timeout):** W przypadku, gdy proces macierzysty nie zakończy się w wyznaczonym czasie (`parent_wait_timeout_seconds`), helper przerywa pracę z kodem błędu `WindowsHelperExitCode::ParentWaitTimeout`, przed rozpoczęciem jakichkolwiek modyfikacji plików. Pliki stagingu ani instalacji nie zostają naruszone.

### B. Zarządzanie Procesem Kandydata
- **Uruchomienie strukturalne (No-Shell):** Kandydat jest uruchamiany bezpośrednio za pomocą Win32 API (`CreateProcessW` / `std::process::Command`) z flagą `CREATE_NO_WINDOW`. Brak powłok pośredniczących (`cmd.exe`, PowerShell).
- **Identyfikacja wyłącznie przez uchwyt potomka:** Obiekt `HelperCandidateChild` zachowuje bezpośredni uchwyt procesu potomnego (`std::process::Child`).
- **Zakaz zabijania po nazwie obrazu lub PID-only:** Zabicie kandydata w razie awarii walidacji dowodu zdrowia (`HealthProof`) następuje wyłącznie poprzez wywołanie metody `child_handle.terminate()` (lub `Stop-Process -InputObject $proc` w PowerShell) na bezpośrednim uchwycie potomka. Nigdy nie jest wywoływane zabijanie procesów po nazwie obrazu ani po gołym identyfikatorze PID.

---

## 4. Semantyka Podmiany Plików i Transakcyjność

### A. Precyzyjna Klasyfikacja Techniczna
Operacja podmiany plików w instalacji Windows została formalnie i precyzyjnie sklasyfikowana jako:
- **`JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED`**
- **`CRASH_RECOVERABLE_BY_JOURNAL`**
- **`NOT_POWER_LOSS_ATOMIC`**

Wycofano wszelkie nieprecyzyjne deklaracje sugerujące "atomowe kopiowanie" (które w systemach plików NTFS bez transakcji TxF nie istnieje dla wielu plików jednocześnie).

### B. Architektura Dziennika Operacji (`HelperOperationJournal`)
1. **Faza weryfikacji wstępnej (Pre-apply Validation):**
   - Sprawdzenie istnienia katalogów, uprawnień zapisu, poprawności ścieżek bez traversal i ADS.
   - Weryfikacja integralności każdego pliku w stagingu przed wykonaniem jakiejkolwiek modyfikacji (`validate_staging_files_exist_and_match`).
2. **Faza kopii zapasowej (Backup Phase):**
   - Tworzenie katalogu `.rollback`.
   - Przeniesienie/skopiowanie każdego zastępowanego pliku do `.rollback`.
   - Zapis rekordu do dziennika operacji z wymuszeniem zapisu buforów (`flush` + `sync_all`).
3. **Faza podmiany (Apply Phase):**
   - Transakcyjne wdrożenie plików ze stagingu do katalogu instalacji.
   - Zapis rekordu operacji i oznaczenie fazy apply w dzienniku (`flush` + `sync_all`).
4. **Faza weryfikacji zdrowia (Health Verification):**
   - Uruchomienie kandydata w trybie sprawdzania zdrowia z unikalnym `attempt_id`.
   - Oczekiwanie na pojawienie się podpisanego dowodu zdrowia `.health_ok`.
5. **Faza wycofania (Rollback):**
   - W razie timeoutu, niezerowego kodu wyjścia lub braku dowodu zdrowia — natychmiastowe zatrzymanie kandydata za pomocą uchwytu potomka.
   - Automatyczne odtworzenie plików z katalogu `.rollback`.
   - Zapewnienie idempotencji rollbacku (wielokrotne wywołanie nie niszczy przywróconego stanu).
   - Bezpieczne zachowanie kopii zapasowej `.rollback` na dysku do celów diagnostycznych, zgodnie z decyzją Właściciela `OWNER-M2-08/09`.

---

## 5. Kompletność i Wyniki Testów

### A. Statystyki Ogólne Projektu
- **Zarejestrowane testy we workspace:** `2522`
- **Wykonane testy programowe:** `2512`
- **Zdane testy (Passed):** `2512` (100% sukcesu)
- **Niezdane testy (Failed):** `0`
- **Zignorowane testy (Ignored):** `10` (wyłącznie laboratoryjne testy sprzętowe HIL)
- **Wykonane testy HIL:** `0` (zgodnie z bezwzględną zasadą bezpieczeństwa)
- **Emisja RF:** `0`
- **Użycie fizycznego PTT:** `0`

### B. Dedykowane Testy Windows Helper (`tests/windows_helper_test.rs`)
Łącznie **28 testów** (wszystkie zaliczone):
1. `windows_helper_path_normalization_and_unicode_spaces`
2. `windows_helper_forged_attempt_id_rejection`
3. `windows_helper_tampered_plan_digest_fails_pre_apply`
4. `windows_helper_parent_process_timeout_aborts`
5. `windows_helper_candidate_failure_triggers_deterministic_rollback`
6. `windows_helper_candidate_health_proof_timeout_triggers_rollback`
7. `windows_helper_never_touches_user_databases_or_wal`
8. `windows_helper_powershell_generator_parity_and_no_wildcards`
9. `windows_helper_stability_multi_iteration_protocol_and_journal`
10. `windows_helper_process_identity_parent_never_killed`
11. `windows_helper_candidate_termination_only_via_child_handle`
12. `windows_helper_staging_tampering_fails_pre_apply`
13. `windows_helper_backup_tampering_aborts_without_overwrite`
14. `windows_helper_journal_crash_recovery_before_apply`
15. `windows_helper_journal_crash_recovery_during_apply`
16. `windows_helper_powershell_real_execution_full_cycle_success` (rzeczywiste wykonanie `powershell.exe`)
17. `windows_helper_powershell_real_execution_candidate_failure_triggers_rollback` (rzeczywiste wykonanie `powershell.exe`)
18. `windows_helper_powershell_static_no_iex_no_wildcards_no_eval`
19. `windows_helper_powershell_static_no_network_no_runas`
20. `windows_helper_rust_vs_powershell_exit_codes_parity`
21. `windows_helper_rust_vs_powershell_result_json_schema_parity`
22. `windows_helper_fault_injection_pre_apply_missing_staging_file`
23. `windows_helper_fault_injection_apply_failure_triggers_full_rollback`
24. `windows_helper_fault_injection_rollback_idempotency`
25. `windows_helper_candidate_kill_by_name_strictly_forbidden`
26. `windows_helper_pid_reuse_guard_checks_process_image_name`
27. `windows_helper_candidate_args_preserve_spaces_in_powershell`
28. `windows_helper_file_replacement_semantics_journaled_not_atomic_copy`

### C. Strażnicy Bezpieczeństwa (`tests/security_guard.rs`)
Łącznie **83 testy strażników** (wszystkie zaliczone), w tym nowo dodany:
- `m3_b_process_identity_and_closure_guard`:
  * Potwierdza brak `kill` / `TerminateProcess` wobec procesu nadrzędnego.
  * Potwierdza, że zakończenie kandydata odbywa się wyłącznie na uchwycie `child_handle.terminate()`.
  * Potwierdza obecność dedykowanego celu binarnego `splogbook-updater` w `Cargo.toml`.

---

## 6. Minimalna Macierz Wymagań Windows (WIN-HELPER-01..36)

| Identyfikator | Wymaganie techniczne | Status | Potwierdzenie testowe |
|---|---|---|---|
| `WIN-HELPER-01` | Dedykowana binarka pomocnika `splogbook-updater.exe` | IMPLEMENTED | `Cargo.toml`, `splogbook-updater.rs` |
| `WIN-HELPER-02` | Wersjonowany kontrakt IPC JSON (`WindowsHelperRequest`/`Result`) | IMPLEMENTED | `windows_helper_path_normalization_and_unicode_spaces` |
| `WIN-HELPER-03` | Kryptograficzna tożsamość transakcji `UpdateAttemptId` | IMPLEMENTED | `windows_helper_forged_attempt_id_rejection` |
| `WIN-HELPER-04` | Weryfikacja skrótu planu SHA-256 (`plan_digest`) | IMPLEMENTED | `windows_helper_tampered_plan_digest_fails_pre_apply` |
| `WIN-HELPER-05` | Monitorowanie procesu macierzystego z limitem czasu | IMPLEMENTED | `windows_helper_parent_process_timeout_aborts` |
| `WIN-HELPER-06` | Bezwzględny zakaz zabijania procesu macierzystego | IMPLEMENTED | `windows_helper_process_identity_parent_never_killed` |
| `WIN-HELPER-07` | Detekcja ponownego użycia PID procesu nadrzędnego | IMPLEMENTED | `windows_helper_pid_reuse_guard_checks_process_image_name` |
| `WIN-HELPER-08` | Uruchamianie kandydata w trybie `CREATE_NO_WINDOW` bez powłoki | IMPLEMENTED | `windows_helper_candidate_failure_triggers_deterministic_rollback` |
| `WIN-HELPER-09` | Zarządzanie kandydatem wyłącznie przez uchwyt potomka | IMPLEMENTED | `windows_helper_candidate_termination_only_via_child_handle` |
| `WIN-HELPER-10` | Bezwzględny zakaz zabijania procesów po nazwie obrazu | IMPLEMENTED | `windows_helper_candidate_kill_by_name_strictly_forbidden` |
| `WIN-HELPER-11` | Weryfikacja dowodu zdrowia kandydata (`HealthProof` `.health_ok`) | IMPLEMENTED | `windows_helper_candidate_health_proof_timeout_triggers_rollback` |
| `WIN-HELPER-12` | Transakcyjny dziennik operacji `HelperOperationJournal` | IMPLEMENTED | `windows_helper_stability_multi_iteration_protocol_and_journal` |
| `WIN-HELPER-13` | Semantyka `JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED` | IMPLEMENTED | `windows_helper_file_replacement_semantics_journaled_not_atomic_copy` |
| `WIN-HELPER-14` | Weryfikacja integralności stagingu przed podmianą | IMPLEMENTED | `windows_helper_staging_tampering_fails_pre_apply` |
| `WIN-HELPER-15` | Tworzenie kopii zapasowej `.rollback` przed modyfikacją | IMPLEMENTED | `windows_helper_backup_tampering_aborts_without_overwrite` |
| `WIN-HELPER-16` | Wycofanie zmian (rollback) przy awarii weryfikacji kandydata | IMPLEMENTED | `windows_helper_candidate_failure_triggers_deterministic_rollback` |
| `WIN-HELPER-17` | Wycofanie zmian przy awarii fazy apply | IMPLEMENTED | `windows_helper_fault_injection_apply_failure_triggers_full_rollback` |
| `WIN-HELPER-18` | Idempotencja procedury rollbacku | IMPLEMENTED | `windows_helper_fault_injection_rollback_idempotency` |
| `WIN-HELPER-19` | Bezpieczne zachowanie katalogu `.rollback` (decyzja OWNER-M2-08/09) | IMPLEMENTED | `windows_helper_never_touches_user_databases_or_wal` |
| `WIN-HELPER-20` | Bezwzględna ochrona baz SQLite użytkownika i plików WAL/SHM | IMPLEMENTED | `windows_helper_never_touches_user_databases_or_wal` |
| `WIN-HELPER-21` | Ochrona konfiguracji użytkownika i logów | IMPLEMENTED | `windows_helper_never_touches_user_databases_or_wal` |
| `WIN-HELPER-22` | Obsługa spacji i znaków Unicode w ścieżkach instalacji | IMPLEMENTED | `windows_helper_path_normalization_and_unicode_spaces` |
| `WIN-HELPER-23` | Zachowanie spacji w argumentach `Start-Process` PowerShell | IMPLEMENTED | `windows_helper_candidate_args_preserve_spaces_in_powershell` |
| `WIN-HELPER-24` | Utwardzony generator fallbacku PowerShell | IMPLEMENTED | `windows_helper_powershell_generator_parity_and_no_wildcards` |
| `WIN-HELPER-25` | Bezwzględny zakaz wildcardów w PowerShell | IMPLEMENTED | `windows_helper_powershell_static_no_iex_no_wildcards_no_eval` |
| `WIN-HELPER-26` | Bezwzględny zakaz `Invoke-Expression` / `iex` / `eval` | IMPLEMENTED | `windows_helper_powershell_static_no_iex_no_wildcards_no_eval` |
| `WIN-HELPER-27` | Bezwzględny zakaz pobierania plików przez PowerShell | IMPLEMENTED | `windows_helper_powershell_static_no_network_no_runas` |
| `WIN-HELPER-28` | Bezwzględny zakaz eskalacji uprawnień (`runas`, UAC) | IMPLEMENTED | `windows_helper_powershell_static_no_network_no_runas` |
| `WIN-HELPER-29` | Bezpieczna tokenizacja PowerShell 5.1 (czyste ASCII) | IMPLEMENTED | `windows_helper_powershell_real_execution_full_cycle_success` |
| `WIN-HELPER-30` | Obsługa UTF-8 BOM generowanego przez PowerShell 5.1 | IMPLEMENTED | `windows_helper_powershell_real_execution_candidate_failure_triggers_rollback` |
| `WIN-HELPER-31` | Rzeczywiste wykonanie sukcesu w `powershell.exe` | IMPLEMENTED | `windows_helper_powershell_real_execution_full_cycle_success` |
| `WIN-HELPER-32` | Rzeczywiste wykonanie rollbacku w `powershell.exe` | IMPLEMENTED | `windows_helper_powershell_real_execution_candidate_failure_triggers_rollback` |
| `WIN-HELPER-33` | Pełny parytet kodów wyjścia Rust vs PowerShell | IMPLEMENTED | `windows_helper_rust_vs_powershell_exit_codes_parity` |
| `WIN-HELPER-34` | Pełny parytet struktury JSON wyniku Rust vs PowerShell | IMPLEMENTED | `windows_helper_rust_vs_powershell_result_json_schema_parity` |
| `WIN-HELPER-35` | Odporność na awarię przed i w trakcie fazy apply w journalu | IMPLEMENTED | `windows_helper_journal_crash_recovery_before_apply` |
| `WIN-HELPER-36` | Dedykowane strażniki architektury w `security_guard.rs` | IMPLEMENTED | `m3_b_process_identity_and_closure_guard` (83 testy) |

---

## 7. Spójność Roadmapy i Hierarchii Etapu M3

Zgodnie z wymaganiami bramki zweryfikowano i uzgodniono hierarchię podetapów M3 w dokumentacji projektu (`MASTER_IMPLEMENTATION_PLAN.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`, `docs/AUDIT_FINDINGS_REGISTER.md`):

1. **Etap M3-A:** Rdzeń platformowy aktualizatora (`COMPLETED_SOFTWARE_VERIFIED`).
2. **Etap M3-B:** Pomocnik Windows i PowerShell (`COMPLETED_SOFTWARE_VERIFIED` — niniejsza bramka zamykająca).
3. **Etap M3-C:** Integracja Windows Portable ZIP (`READY_FOR_IMPLEMENTATION`, `NOT STARTED` — bramka `STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE`).
4. **Dalsze etapy M3 (instalatory Windows i pakiety Linux):**
   - **M3-D:** Integracja Windows User Setup / Inno Setup (gdy dotyczy)
   - **M3-E:** Integracja Windows System Setup / MSI (gdy dotyczy)
   - **M3-F:** Integracja Linux Portable tar.gz
   - **M3-G:** Integracja Linux AppImage
   - **M3-H:** Integracja Linux DEB
   - **M3-I:** Integracja Linux RPM
   - **M3-J:** Integracja Linux Flatpak

**Potwierdzenie:** Pakiety Linux nie są i nie mogą być przypisane do etapu M3-C. Wdrożenie etapu M3-C NIE zostało rozpoczęte.

---

## 8. Status Plików Zarządczych Cargo, EOL i Audytu Bezpieczeństwa

- **Plik Cargo.toml:** Zawiera wyłącznie cel binarny `splogbook-updater`. Brak nowych zależności, brak zmian wersji bibliotek.
- **Plik Cargo.lock:** Nienaruszony (`52be66b2bc063c001d5c2e8556457fa2be04ff4f`).
- **Walidacja binarnego pomocnika:**
  - `cargo check --bin splogbook-updater`: SUCCESS
  - `cargo test --bin splogbook-updater`: SUCCESS
  - `cargo build --bin splogbook-updater`: SUCCESS
- **Formatowanie i statyczna analiza kodu:**
  - `cargo fmt --all -- --check`: 0 błędów
  - `cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings`: 0 ostrzeżeń, 0 błędów
- **Audyt bezpieczeństwa zależności:**
  - `cargo audit`: 0 podatności (brak nieznanych podatności)
- **Zgodność formatu linii (EOL):** Brak niedozwolonych zmian CRLF/LF w plikach roboczych.

---

## 9. Raport Git i Liniowość Historii

- **Baseline HEAD przed etapem M3-B:** `84676d57a9b4ed4c30f61d7cef52624f70189f6e`
- **HEAD wejściowy przed bramką weryfikacyjną:** `5e5b37932aa664520909cf5b927ebe3f6fce443b`
- **Commity zrealizowane w ramach bramki `STAGE-M3-B-CLOSURE-VERIFICATION-GATE`:**
  1. `a276da6` — `fix(updater): harden Windows helper pre-apply and fault injection capabilities (M3-B)`
  2. `7c25e0a` — `test(updater): add native Windows helper process, tampering, fault and PowerShell execution tests (M3-B)`
  3. `7b88e1a` — `docs(updater): reconcile M3-B process and replacement guarantees`
  4. (Bieżący commit formalnego zamknięcia i reconciliacji dokumentacji)
- **Metoda scalania:** Wyłącznie `fast-forward` (`git merge --ff-only`).
- **Stan drzewa roboczego:** `CLEAN` (brak niezatwierdzonych zmian).
- **Synchronizacja ze zdalnym repozytorium:** `origin/main` jest w 100% zsynchronizowane z gałęzią lokalną `main` (ahead 0, behind 0).

---

## 10. Rekomendacja i Następny Krok

### Rekomendacja
Etap **STAGE-M3-B-WINDOWS-HELPERS-IMPLEMENTATION-GATE** spełnia w 100% wszystkie rygorystyczne wymagania bezpieczeństwa, stabilności, tożsamości procesów, transakcyjnej semantyki podmiany plików oraz pokrycia testami natywnymi Windows i PowerShell.

Etap M3-B uzyskuje definitywny, bezwarunkowy status:
**`COMPLETED_SOFTWARE_VERIFIED`**

### Następny Krok
Kolejnym kanonicznym zadaniem w roadmapie projektu jest:
**`STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE`**  
(Gotowy do rozpoczęcia w osobnej, dedykowanej sesji roboczej; w bieżącej sesji nie rozpoczęty).
