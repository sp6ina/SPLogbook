# RAPORT PRZEDIMPLEMENTACYJNY: STAGE-M3-D-PREREQUISITES-REMEDIATION-GATE

**Data robocza projektu:** 2026-11-07
**Data środowiska:** 2026-11-08
**Czas środowiska:** 09:03:00
**Timezone:** Central European Standard Time ((UTC+01:00) Sarajewo, Skopie, Warszawa, Zagrzeb)
**UTC offset:** +01:00
**Etap:** STAGE-M3-D-PREREQUISITES-REMEDIATION-GATE
**Kanon:** Usunięcie regresji i18n, korekta terminologii transactional replacement oraz weryfikacja środowiska Inno Setup przed wznowieniem M3-D
**Status bramki wejściowy:** BLOCKED_OR_PARTIALLY_COMPLETED

---

## 1. Data i czas

- **DATA ROBOCZA PROJEKTU:** 2026-11-07
- **DATA ŚRODOWISKA:** 2026-11-08
- **CZAS ŚRODOWISKA:** 09:03:00
- **TIMEZONE:** Central European Standard Time
- **UTC OFFSET:** +01:00
- **Zegar systemowy:** Niezmodyfikowany.
- **Daty commitów historycznych:** Zachowane bez zmian.

---

## 2. Stan repozytorium Git

- **HEAD przed fetch:** `de2ef9bb26afbdf13df20bcde4a972902adc5e48`
- **HEAD po fetch:** `de2ef9bb26afbdf13df20bcde4a972902adc5e48`
- **main:** `de2ef9bb26afbdf13df20bcde4a972902adc5e48`
- **origin/main:** `de2ef9bb26afbdf13df20bcde4a972902adc5e48`
- **Branch roboczy:** `repair/full-system-remediation` (`de2ef9bb26afbdf13df20bcde4a972902adc5e48`)
- **Ahead / Behind:** 0 / 0
- **Working Tree:** CLEAN
- **Aktywne operacje Git:** Brak (brak MERGE_HEAD, REBASE_HEAD, CHERRY_PICK_HEAD, REVERT_HEAD)
- **Tag v1.2.0:**
  - Wskaźnik tagu: `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` (zgodność lokalna i zdalna `refs/tags/v1.2.0^{}`)
- **Klasyfikacja Git:** `EXACT_BASELINE`

---

## 3. Cargo i ochrona wersji

- **Wersja pakietu (Cargo.toml):** `1.3.0-alpha.1`
- **Wersja pakietu (Cargo.lock):** `1.3.0-alpha.1`
- **Cargo.toml hash:** `c2247198422ca6d52415d38efb6229b13a3ba9f6` (zgodny z HEAD, brak modyfikacji)
- **Cargo.lock hash:** `fe5f33bc1f40f9c6824760bbe06604590ab9a938` (zgodny z HEAD, brak modyfikacji)
- **Zależności:** 0 nowych, 0 zaktualizowanych, brak modyfikacji drzewa zależności.
- **EOL:** LF (`i/lf w/lf attr/`) dla obu plików manifestu.

---

## 4. Identyfikacja i analiza regresji I18N (Finding 1)

- **Identyfikator:** `BLK-M3-D-01-BASELINE-I18N-REGRESSION`
- **Plik źródłowy:** `src/gui/app_chrome.rs`
- **Linie:** 240 oraz 248
- **Niezlokalizowane literały:**
  - L240: `"Commit:"` (niezlokalizowany literał w wywołaniu widżetu `ui.label`)
  - L248: `"Kanał / Profil:"` (polski znak w literale stringowym oraz niezlokalizowany literał w wywołaniu widżetu `ui.label`)
- **Naruszone testy strażnicze:**
  - `tests/i18n_guard.rs::gui_has_no_hardcoded_text` (FAILED)
- **Geneza wprowadzenia:** Wprowadzone w commicie `c2db075` podczas unifikacji modelu tożsamości `BuildIdentity`.
- **Klasyfikacja:** P0 — Regresja lokalizacyjna UI łamiąca wymaganie braku twardo zakodowanych tekstów w warstwie GUI.

---

## 5. Analiza lokalizacji i projekt kluczy I18N

- **Lista obsługiwanych języków:** 7 języków (Polski `pl`, Angielski `en`, Niemiecki `de`, Francuski `fr`, Hiszpański `es`, Włoski `it`, Rosyjski `ru`).
- **Istniejąca konwencja w sekcji okna About:**
  W oknie "O programie" w sekcji zwijanej diagnostyki (`CollapsingHeader`) wszystkie etykiety należą do grupy `app.diag_*`:
  - `app.diag_version` -> `Wersja:` / `Version:` / itp. (z dwukropkiem)
  - `app.diag_platform` -> `Platforma:` / `Platform:` / itp.
  - `app.diag_arch` -> `Architektura:` / `Architecture:` / itp.
  - `app.diag_theme` -> `Motyw:` / `Theme:` / itp.
  - `app.diag_data_dir` -> `Katalog danych:` / `Data directory:` / itp.
  - `app.diag_copy_btn` -> `📋 Kopiuj dane diagnostyczne`
- **Zatwierdzone klucze lokalizacyjne dla regresji:**
  Zgodnie z zasadą zachowania spójności z istniejącym prefiksem diagnostyki okna About:
  1. `app.diag_commit` -> generuje stałą `key::APP_DIAG_COMMIT`
  2. `app.diag_channel_profile` -> generuje stałą `key::APP_DIAG_CHANNEL_PROFILE`
- **Wymagane pliki do modyfikacji:**
  - `locales/pl.json`
  - `locales/en.json`
  - `locales/de.json`
  - `locales/fr.json`
  - `locales/es.json`
  - `locales/it.json`
  - `locales/ru.json`
  - `src/gui/app_chrome.rs`

---

## 6. Analiza błędu terminologicznego (Finding 3)

- **Identyfikator:** `TERM-M3-D-03-ATOMIC-SWAP-MISNOMER`
- **Stara nazwa testu:** `windows_helper_stability_20_iterations_database_preservation_and_atomic_swap` w `tests/windows_helper_test.rs:2013`
- **Wymagana nazwa:** `windows_helper_stability_20_iterations_database_preservation_and_transactional_replacement`
- **Wpływ architektoniczny:**
  Podmiana wieloplikowej instalacji na systemie Windows nie jest operacją atomową w sensie sprzętowym lub pojedynczego wywołania systemowego (brak gwarancji przy nagłej utracie zasilania: `NOT_POWER_LOSS_ATOMIC`). Została ona zaprojektowana i zweryfikowana jako transakcyjna podmiana dziennikowana z odzyskiwaniem stanu po awarii (`JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED`, `CRASH_RECOVERABLE_BY_JOURNAL`). Określenie `atomic swap` było historycznym skrótem myślowym i wprowadzało niespójność z kanonem architektonicznym M3-B/M3-C.
- **Wymagany strażnik:** Kontekstowy test w `tests/security_guard.rs` zabraniający deklarowania atomowości dla podmiany wieloplikowej w updaterze/helperze, dopuszczający poprawne określenia transakcyjne oraz atomowość pojedynczych plików (locki, pliki tymczasowe, typy atomowe).

---

## 7. Weryfikacja środowiska Inno Setup (Finding 2)

- **Zmienna `INNO_ISCC_PATH`:** Nie zdefiniowana.
- **Lokalizacja w systemowym PATH:** `where.exe ISCC.exe` -> Nie znaleziono.
- **Sprawdzenie ścieżek per-machine:**
  - `C:\Program Files (x86)\Inno Setup 6\ISCC.exe` -> Brak
  - `C:\Program Files\Inno Setup 6\ISCC.exe` -> Brak
  - `C:\Program Files\Inno Setup 7\ISCC.exe` -> **ZNALEZIONY**
- **Właściwości wykrytego pliku:**
  - Ścieżka: `C:\Program Files\Inno Setup 7\ISCC.exe`
  - Rozmiar: 2 136 992 B
  - Wersja kompilatora (`--version`): `7.1.1`
- **Klasyfikacja środowiska:**
  `INNO_SETUP_COMPATIBLE_NEWER_REQUIRES_REVIEW` (Kategoria B)
  Wykryto oficjalny kompilator Inno Setup w standardowej lokalizacji systemowej, jednak w wersji 7.1.1 (wyższej niż pierwotnie testowana rodzina 6.x). Kanoniczne wymaganie projektowe `Inno Setup 6+` dopuszcza wersje 6 i nowsze, lecz ze względu na zmianę wersji głównej (major version 7) wymagane jest zatwierdzenie zgodności wstecznej składni skryptów instalatora przed przejściem do fazy implementacji M3-D.
- **Polityka bezpieczeństwa agenta:**
  Agent NIE instalował, NIE pobierał i NIE modyfikował Inno Setup ani zmiennej PATH systemowego.

---

## 8. Stan baseline przed modyfikacjami

| Narzędzie / Test | Oczekiwane | Rzeczywiste przed zmianami | Status |
| :--- | :--- | :--- | :--- |
| `cargo fmt --all -- --check` | 0 format errors | 0 errors | **PASS** |
| `cargo check --workspace --all-targets` | 0 errors | 0 errors | **PASS** |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings | 0 warnings | **PASS** |
| `cargo audit` | 0 vulnerabilities, <= 7 allowed | 0 vulnerabilities, 7 warnings | **PASS** |
| `tests/i18n_guard.rs` | 15 passed, 0 failed | 14 passed, 1 failed | **FAILED** (`gui_has_no_hardcoded_text`) |
| `tests/security_guard.rs` | 95 passed, 0 failed | 95 passed, 0 failed | **PASS** |
| `tests/windows_helper_test.rs` | 33 passed, 0 failed | 33 passed, 0 failed | **PASS** |

---

## 9. Szczegółowy plan zmian

1. **Commit 1 (`fix(i18n)`):**
   - Dodanie `"app.diag_commit"` oraz `"app.diag_channel_profile"` do 7 plików językowych w `locales/`.
   - Zastąpienie literałów w `src/gui/app_chrome.rs` wywołaniami `tr(key::APP_DIAG_COMMIT, lang)` oraz `tr(key::APP_DIAG_CHANNEL_PROFILE, lang)`.
   - Weryfikacja: `cargo test --test i18n_guard` (15/15 passed).
   - Fast-forward merge do `main` i push do `origin/main`.
2. **Commit 2 (`test(updater)`):**
   - Zmiana nazwy funkcji w `tests/windows_helper_test.rs` na `windows_helper_stability_20_iterations_database_preservation_and_transactional_replacement`.
   - Dodanie trwałego strażnika terminologicznego w `tests/security_guard.rs`.
   - Weryfikacja: `cargo test --test windows_helper_test` oraz `cargo test --test security_guard`.
   - Fast-forward merge do `main` i push do `origin/main`.
3. **Commit 3 (`docs(installer)`):**
   - Utworzenie `STAGE_M3_D_PREREQUISITES_REMEDIATION_REPORT.md`.
   - Aktualizacja rejestru statusów i roadmapy.
   - Fast-forward merge do `main` i push do `origin/main`.

---

## 10. Kryteria wznowienia etapu M3-D

- Pełna eliminacja regresji i18n (15/15 w `i18n_guard`).
- Spójność terminologiczna (`transactional replacement`, brak zakazanych deklaracji atomowości).
- Zweryfikowanie kompilatora ISCC (znaleziony 7.1.1 z rekomendacją autoryzacji zgodności ze składnią M3-D).
- Czyste drzewo robocze Git, zsynchronizowane z `origin/main`.
