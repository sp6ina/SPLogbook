# RZECZYWISTY RAPORT KOŃCOWEJ BRAMKI ETAPU 10

## 1. Data

- **Data robocza projektu:** 2026-11-06
- **Data środowiska wykonawczego:** 2026-11-07
- **Timezone:** Europe/Warsaw (UTC+01:00)

---

## 2. Git

- **HEAD przed fetch:** `e41e5381488692bfe83257ab7a1e4e4fcda4e836`
- **origin/main przed fetch:** `e41e5381488692bfe83257ab7a1e4e4fcda4e836`
- **Pobrane nowe commity z origin:** 0 (historia w 100% zsynchronizowana)
- **HEAD wejściowy bramki:** `e41e5381488692bfe83257ab7a1e4e4fcda4e836`
- **Gałąź robocza:** `repair/full-system-remediation`
- **Gałąź główna:** `main`
- **Śledzenie zdalne:** `origin/main`
- **Ahead / Behind:** `0 / 0`
- **Working tree:** Czyste (`git status --porcelain=v1` zwraca stan pusty)
- **Operacje zabronione (rebase, force-push, merge bez ff):** NOT EXECUTED

---

## 3. Cargo i Integralność Repozytorium

- **`Cargo.toml` SHA-1 (blob Git LF):** `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (niezmieniony)
- **`Cargo.lock` SHA-1 (blob Git LF):** `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (niezmieniony)
- **`Cargo.toml` working-copy hash (CRLF Windows):** `0cdf381165bfc3111fff15097007e4b235484ec8`
- **`Cargo.lock` working-copy hash (CRLF Windows):** `52be66b2bc063c001d5c2e8556457fa2be04ff4f`
- **`git diff HEAD -- Cargo.toml Cargo.lock`:** Pusty (0 bajtów zmian)
- **Nowe zależności:** 0
- **Zaktualizowane zależności:** 0
- **Zmiany wersji projektu:** 0 (projekt pozostaje na wersji `1.2.0`)

---

## 4. Test Counts (Kanoniczny pomiar deterministyczny)

- **TOTAL REGISTERED TESTS:** 2289
- **TOTAL EXECUTED TESTS:** 2279
- **TOTAL PASSED TESTS:** 2279
- **TOTAL FAILED TESTS:** 0
- **TOTAL IGNORED TESTS:** 10
- **HIL IGNORED TESTS:** 10
- **DOC TESTS:** 0
- **FLAKY TESTS:** 0

### Podział celów testowych (per target):
| Cel testowy | Kategoria | Registered | Executed | Passed | Failed | Ignored |
|---|---|---|---|---|---|---|
| `src/lib.rs` | Unit / Module | 1645 | 1645 | 1645 | 0 | 0 |
| `src/main.rs` | Binary | 0 | 0 | 0 | 0 | 0 |
| `src/bin/signer.rs` | Binary | 0 | 0 | 0 | 0 | 0 |
| `tests/cat_native_6a.rs` | Integration (CAT 6A) | 165 | 165 | 165 | 0 | 0 |
| `tests/cat_native_6b.rs` | Integration (CAT 6B) | 99 | 99 | 99 | 0 | 0 |
| `tests/cat_native_7.rs` | Integration / HIL Offline | 307 | 297 | 297 | 0 | 10 (HIL) |
| `tests/i18n_guard.rs` | Guard (Arch/i18n) | 15 | 15 | 15 | 0 | 0 |
| `tests/security_guard.rs` | Guard (Security/CI) | 57 | 57 | 57 | 0 | 0 |
| `tests/theme_guard.rs` | Guard (Theme) | 1 | 1 | 1 | 0 | 0 |
| `Doc-tests splogbook` | Doc Tests | 0 | 0 | 0 | 0 | 0 |
| **SUMA CANONICAL** | | **2289** | **2279** | **2279** | **0** | **10** |

---

## 5. ClubRegistry (FUNC-006)

- **Status findingu:** `FIXED_SOFTWARE_VERIFIED`
- **Primary membership key:** `(club_id, normalized_callsign)` — tożsamość członkostwa stacji w klubie
- **Member-number conflict key:** `(club_id, normalized_member_number)` — odrębny indeks unikalności numeru w klubie
- **Cross-club collisions:** Legalne powtórzenia tego samego numeru w różnych klubach są w pełni dopuszczone (`club_registry_allows_same_member_number_in_different_clubs`).
- **Intra-club conflicts:** Sprzeczny duplikat tego samego numeru dla innego znaku w tym samym klubie jest bezwzględnie odrzucany (`club_registry_rejects_same_number_for_different_callsigns_in_same_club`).
- **Idempotent duplicate policy:** Identyczny rekord jest akceptowany idempotentnie (dokładnie 1 wpis bez błędu, `club_registry_accepts_idempotent_duplicate`).
- **Sprzeczność tożsamości:** Ten sam znak w tym samym klubie z innym numerem jest odrzucany jako konflikt (`club_registry_rejects_same_membership_with_conflicting_number`).
- **Przypisanie numerów:** Rejestr nigdy nie wymyśla ani nie generuje numerów automatycznie (brak numeru w CSV = `None`).
- **Wymóg normalizacji:** Lookup i kluczowanie używają `clean = callsign.trim().to_uppercase()`.
- **Audyt datasetu `databases/clubs.csv`:**
  - `INPUT ROWS`: 17
  - `VALID ROWS`: 17
  - `REJECTED ROWS`: 0
  - `IDEMPOTENT DUPLICATES`: 0
  - `CROSS-CLUB SAME-NUMBER OCCURRENCES`: 0
  - `INTRA-CLUB NUMBER CONFLICTS`: 0
  - `MEMBERSHIP KEY CONFLICTS`: 0
  - `EMPTY CALLSIGNS`: 0
  - `UNKNOWN CLUB IDS`: 0
  - `MALFORMED ROWS`: 0
- **Fallback:** `ClubRegistry::default_registry()` ze sprawdzonymi, unikalnymi numerami wbudowanymi w binarkę.

---

## 6. PropagationHistory (FUNC-007)

- **Status findingu:** `FIXED_SOFTWARE_VERIFIED`
- **Typ zwracany API:** `Option<f64>` dla `accuracy_pct()` oraz `band_accuracy_pct(band: &str)`
- **Stan braku próbek:** `None`
- **Prezentacja w GUI:** `"N/A"` z kolorem `text_muted` (brak próbek nigdy nie jest pokazywany jako 0.0%)
- **Stan zmierzonego zera:** `Some(0.0)`
- **Prezentacja zmierzonego zera w GUI:** `"0.0% (N)"` z kolorem `palette.warning`
- **Zakres wartości:** Ściśle ograniczony `.clamp(0.0, 100.0)`
- **Dzielenie przez zero:** Całkowicie wyeliminowane (funkcja sprawdza `if verified.is_empty() { return None; }`)
- **Unieważnianie cache:** Klucz pamięci podręcznej `SolarCacheKey` jest natychmiast unieważniany przy zmianie parametrów solarnych lub godziny UTC.
- **Weryfikacja testowa:** 13 dedykowanych testów sprawdzających stany N/A, 0.0%, 100.0%, brak próbek pasmowych oraz prezentację w egui.

---

## 7. cargo-audit i Polityka Bezpieczeństwa CI (CI-002)

- **Status findingu:** `FIXED_SOFTWARE_VERIFIED`
- **Przypięta wersja cargo-audit:** `0.22.2`
- **Polecenie instalacji w CI:** `cargo install cargo-audit --version 0.22.2 --locked`
- **Polecenie audytu w CI:**
  ```bash
  cargo audit --deny warnings --no-yanked --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2025-0141 --ignore RUSTSEC-2026-0249 --ignore RUSTSEC-2026-0370 --ignore RUSTSEC-2026-0371 --ignore RUSTSEC-2026-0372 --ignore RUSTSEC-2026-0097
  ```
- **Polityka wyjątków:**
  - 0 dozwolonych luk bezpieczeństwa (`vulnerabilities`)
  - 0 niezaakceptowanych ostrzeżeń `unsound`
  - Brak szerokich masek (`wildcards` zabronione)
  - Brak opcji typu `ignore all warnings`
  - Każde nowe advisory spoza jawnej listy powoduje natychmiastowe zatrzymanie potoku CI (exit code 1)
- **Lista zaakceptowanych tranzytywnych advisories (7):**
  1. `RUSTSEC-2024-0436` (paste 1.0.15, unmaintained) — tranzytywna przez `egui_dock`
  2. `RUSTSEC-2025-0141` (bincode 1.3.3, unmaintained) — tranzytywna przez `printpdf` -> `azul-layout` -> `hyphenation`
  3. `RUSTSEC-2026-0249` (smartstring 1.0.1, unmaintained) — tranzytywna przez `rhai`
  4. `RUSTSEC-2026-0370` (zvariant 5.15.0, unmaintained) — tranzytywna przez `keyring` -> `zbus`
  5. `RUSTSEC-2026-0371` (zbus_names 4.3.4, unmaintained) — tranzytywna przez `keyring` -> `zbus`
  6. `RUSTSEC-2026-0372` (zvariant_derive 5.15.0, unmaintained) — tranzytywna przez `keyring` -> `zbus`
  7. `async-executor 1.14.0` (`RUSTSEC-2026-0097` / yanked warning) — tranzytywna przez `keyring` -> `zbus`
- **Strażnicy bezpieczeństwa w `tests/security_guard.rs` (8 nowych guardów):**
  - `ci_uses_pinned_cargo_audit_version`
  - `ci_runs_cargo_audit`
  - `ci_audit_does_not_ignore_all_warnings`
  - `ci_audit_uses_explicit_advisory_ids`
  - `ci_audit_allowed_advisories_match_documented_baseline`
  - `ci_audit_new_unknown_advisory_would_fail`
  - `ci_audit_does_not_disable_vulnerability_failures`
  - `ci_audit_does_not_modify_cargo_manifests`

---

## 8. Jakość Dokumentacji i Notacji Matematycznej (DOC-001)

- **Status findingu:** `FIXED_SOFTWARE_VERIFIED`
- **Zduplikowane artefakty KaTeX:** `0` (wyszukiwania `O(N)O(N)O(N)`, `O(1)O(1)O(1)`, `O(S×N)O(S \times N)O(S×N)` w plikach markdown zwracają 0 wyników)
- **Spójność dokumentacji:**
  - `README.md`: Zaktualizowany badge `Tests-2279/2279_Passed`, sekcja testów z pełnym zestawieniem metryk (2289 registered, 2279 passed, 10 ignored HIL).
  - `Native CAT.md`: Zaktualizowany bilans testów (2279 aktywnych passed, 2289 registered, 10 ignored HIL).
  - `docs/AUDIT_FINDINGS_REGISTER.md`: Zaktualizowany rejestr z prawidłowymi kluczami ClubRegistry, brakiem próbek propagacji i polityką cargo-audit.
  - `docs/IMPROVEMENT_ROADMAP.md`: Zgodny z rzeczywistym bilansem testów i zamknięciem Etapu 10.
  - `docs/REMEDIATION_BASELINE.md`: Zsynchronizowany z kanonicznymi metrykami bramki końcowej.
  - `STAGE_10_FINAL_GATE_REPORT.md`: Utworzony niniejszy raport źródłowy.
  - `tests/security_guard.rs`: Dodano guard `documentation_test_counts_match_canonical_workspace_counts` weryfikujący automatycznie spójność dokumentacji.

---

## 9. Regresja i Stabilność

- **Regresja Etapu 10:** 0
- **Regresja Etapów 1–9:** 0
- **Clippy `-D warnings`:** EXIT 0 (0 ostrzeżeń, 0 błędów)
- **Fmt check:** 0 odchyleń formatowania
- **Stabilność testów:** 10/10 powtórzeń testów `ClubRegistry`, `test_backup_database_concurrent_writer`, `propagation_accuracy` i `security_guard` zakończonych sukcesem (0 flaky).

---

## 10. Bezpieczeństwo Danych i Środowiska Fizycznego

- **Baza danych użytkownika:** NIE OTWIERANA (0 operacji)
- **Konfiguracja użytkownika:** NIE OTWIERANA (0 modyfikacji)
- **Fizyczny HIL:** NIE WYKONYWANY (`NOT EXECUTED — SAFETY GATE CLOSED`, 10 testów pozostaje w stanie `#[ignore]`)
- **Fizyczne PTT:** NIE WYKONYWANE (0 cykli)
- **Emisja RF:** 0.0 W (brak promieniowania)

---

## 11. Ostateczny Werdykt Bramki Końcowej

DATA ROBOCZA PROJEKTU: 2026-11-06
KOŃCOWA BRAMKA ETAPU 10: PASSED
FUNC-004: FIXED_SOFTWARE_VERIFIED
FUNC-006: FIXED_SOFTWARE_VERIFIED
FUNC-007: FIXED_SOFTWARE_VERIFIED
FUNC-008: FIXED_SOFTWARE_VERIFIED
RUST-001: FIXED_SOFTWARE_VERIFIED
CI-002: FIXED_SOFTWARE_VERIFIED
DOC-001: FIXED_SOFTWARE_VERIFIED
TOTAL REGISTERED TESTS: 2289
TOTAL EXECUTED TESTS: 2279
TOTAL PASSED TESTS: 2279
TOTAL FAILED TESTS: 0
TOTAL IGNORED TESTS: 10
HIL IGNORED TESTS: 10
DOCUMENTATION TEST COUNTS: CONSISTENT
CLUB MEMBERSHIP PRIMARY KEY: CLUB_ID + NORMALIZED_CALLSIGN
CLUB MEMBER NUMBER CONFLICT KEY: CLUB_ID + NORMALIZED_MEMBER_NUMBER
CROSS-CLUB SAME MEMBER NUMBER: ALLOWED
UNRESOLVED INTRA-CLUB NUMBER CONFLICTS: 0
INVENTED MEMBER NUMBERS: 0
PROPAGATION WITHOUT SAMPLES: N/A
PROPAGATION MEASURED ZERO: 0.0%
PROPAGATION NO-DATA AMBIGUITY: 0
CARGO-AUDIT VERSION: PINNED (0.22.2)
CARGO-AUDIT POLICY: EXPLICIT ADVISORY IDS
BROAD AUDIT IGNORE: 0
UNKNOWN ADVISORY WOULD FAIL: VERIFIED
UNACCEPTED VULNERABILITIES: 0
UNACCEPTED UNSOUND ADVISORIES: 0
CLIPPY DENY WARNINGS: PASSED
CLIPPY WARNINGS: 0
DOCUMENTATION KATEX ARTIFACTS: 0
STALE TEST COUNTS: 0
FLAKY TESTS: 0
STAGE 10 REGRESSION: 0
STAGE 1-9 REGRESSION: 0
CARGO.TOML: UNCHANGED
CARGO.LOCK: UNCHANGED
NEW DEPENDENCIES: 0
PRODUCTION DATABASE OPENED: NO
PRODUCTION CONFIG OPENED: NO
USER DATA ACCESSED: NO
USER DATA MODIFIED: NO
PHYSICAL HIL: NOT EXECUTED
PHYSICAL PTT: NOT EXECUTED
RF EMISSION: 0
MAIN: SYNCHRONIZED
ORIGIN/MAIN: SYNCHRONIZED
AHEAD/BEHIND: 0/0
WORKING TREE: CLEAN
FORCE-PUSH: NOT EXECUTED
ETAP 10: COMPLETED_SOFTWARE_VERIFIED
ETAP 11: READY
ETAP 11 EXECUTION: NOT STARTED
