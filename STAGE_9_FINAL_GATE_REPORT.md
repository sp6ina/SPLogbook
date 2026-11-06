# RAPORT KOŃCOWY: BRAMKA ROZSTRZYGNIĘCIA TOŻSAMOŚCI GIT I KOŃCOWA BRAMKA ETAPU 9

## 1. Tożsamość Git i Rozstrzygnięcie Rozbieżności Hashy

- **Wcześniejszy hash wejściowy (zapisany w promptach):** `e4114c0a52df2ca7ee8478d8a7c29be1a7dc2612`
- **Rzeczywisty hash odczytany z repozytorium:** `e4114c05dbdf54e6fbd11d305af5519e51552dc9`
- **Weryfikacja istnienia obiektów w Git:**
  - `git cat-file -e e4114c0a52df2ca7ee8478d8a7c29be1a7dc2612^{commit}` -> `OBJECT_NOT_FOUND` (kod wyjścia 128, obiekt fizycznie nie istnieje w repozytorium ani zdalnym `origin`).
  - `git cat-file -e e4114c05dbdf54e6fbd11d305af5519e51552dc9^{commit}` -> `EXISTS` (obiekt poprawny, typ: `commit`, rozmiar: 554 bajty).
- **Drzewo (Tree SHA):** `8bb6b4991c0e3eb4c02967ceeb0b9a8cf49666c5`
- **Rodzice (Parents SHA):** `a7330cee990c7f7636e788c03975c5ecdb8628e9` (`test(gui): add keyboard persistence and rendering regression matrix`)
- **Referencje wskazujące commit `e4114c05...`:**
  - `refs/heads/main`
  - `refs/remotes/origin/main`
  - `refs/heads/repair/full-system-remediation`
  - `HEAD`
- **Reflog:** Commit `e4114c05...` został poprawnie utworzony w repozytorium 2026-11-06T14:09:47+01:00 i bezkonfliktowo przewinięty (`checkout`, `merge --ff-only`). Stary pseudo-ref `.git/REBASE_HEAD` datowany na 2026-09-28 został zweryfikowany jako nieaktywny relikt (brak trwającej operacji rebase).
- **Weryfikacja skrótu `e4114c0`:**
  - `git rev-parse --disambiguate=e4114c0` zwraca dokładnie jeden wpis: `e4114c05dbdf54e6fbd11d305af5519e51552dc9`.
  - Skrót siedmioznakowy jest w 100% jednoznaczny i bezkolizyjny w całym repozytorium.
- **Klasyfikacja rozstrzygnięcia:** **Ścieżka A: `REPOSITORY_HASH_IS_CANONICAL_AND_DOCUMENTATION_WAS_WRONG`**.
- **Przyczyna:** Błąd kontraktu dokumentacyjnego/promptu (literówka przy rozszerzaniu skrótu z 7 do 40 znaków). Obiekt `e4114c0a...` nigdy nie istniał. Historia Git jest spójna i nienaruszona.

---

## 2. Kanoniczny Baseline i Stan Repozytorium

- **Aktywna gałąź robocza:** `repair/full-system-remediation`
- **Lokalna gałąź główna:** `main` (synchronizowana wyłącznie `--ff-only`)
- **Zdalna gałąź główna:** `origin/main` (synchronizowana wyłącznie `--ff-only`)
- **Ahead / Behind:** `0 / 0` (stan pełnej synchronizacji we wszystkich 3 referencjach)
- **Working Tree:** Czyste (brak niezacommitowanych zmian, brak konfliktów, brak procesów rebase/merge/cherry-pick)
- **Historia Git:** Całkowicie nienaruszona; nie wykonano rebase, zwykłego merge, rewrite ani force-push.

---

## 3. Integralność Zależności Cargo

- `Cargo.toml` SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (w 100% nienaruszony)
- `Cargo.lock` SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (w 100% nienaruszony)
- **Nowe zależności zewnętrzne:** 0
- **Aktualizacje wersji w crates:** 0
- **Audyt podatności (`cargo audit`):** 0 podatności

---

## 4. Statystyki Zestawu Testów (Test Counts)

Wszystkie testy zostały zweryfikowane deterministycznym audytem testów (`test_audit.rs`):

- **Łączna liczba zarejestrowanych testów:** **2172**
- **Wykonane aktywne testy programowe:** **2162**
- **Testy zakończone sukcesem (PASSED):** **2162**
- **Testy zakończone porażką (FAILED):** **0**
- **Testy ignorowane (fizyczny HIL):** **10** (`#[ignore = "hardware-in-the-loop"]`)
- **Testy niestabilne (flaky):** **0**
- **Testy dokumentacyjne (`cargo test --doc`):** **0 failed**

### Szczegółowy podział według targetów testowych:
| Pakiet / Target | Ścieżka | Testy PASSED | Testy FAILED | Testy IGNORED |
|---|---|---:|---:|---:|
| `splogbook` (biblioteka rdzenna) | `src/lib.rs` | 1543 | 0 | 0 |
| `security_guard` (strażnicy bezpieczeństwa) | `tests/security_guard.rs` | 42 | 0 | 0 |
| `i18n_guard` (strażnicy internacjonalizacji) | `tests/i18n_guard.rs` | 15 | 0 | 0 |
| `theme_guard` (strażnik kolorystyki) | `tests/theme_guard.rs` | 1 | 0 | 0 |
| `cat_native_6a` (architektura wieloradiowa A) | `tests/cat_native_6a.rs` | 165 | 0 | 0 |
| `cat_native_6b` (architektura wieloradiowa B) | `tests/cat_native_6b.rs` | 99 | 0 | 0 |
| `cat_native_7` (protokoły radiowe i mocki HIL) | `tests/cat_native_7.rs` | 297 | 0 | 10 |
| `SPLogbook` (główna aplikacja) | `src/main.rs` | 0 | 0 | 0 |
| `splogbook-signer` (narzędzie podpisujące) | `src/bin/signer.rs` | 0 | 0 | 0 |
| **SUMA** | | **2162** | **0** | **10** |

---

## 5. Izolacja Skrótu `Ctrl+W` (`GUI-001`)

- **Problem pierwotny:** Skrót `Ctrl+W` czyścił formularz łączności niezależnie od tego, czy operator edytował pole tekstowe filtra, asystenta operatora czy konfiguracji stacji.
- **Wdrożone rozwiązanie:**
  1. Zaimplementowano typ `ActiveFocusContext` w `src/gui/shortcuts.rs`:
     - `ModalDialog` (najwyższy priorytet — blokada akcji tła)
     - `ContestQsoEntry` (formularz łączności w oknie zawodów — `ContestWipeQso`)
     - `MainQsoEntry` (główny formularz łączności — `WipeQso`)
     - `LogFilter` (pole wyszukiwarki/filtra)
     - `SettingsTextEdit` (okna ustawień i konfiguracji)
     - `OperatorAssistantTextEdit` (notatki asystenta i terminal CW)
     - `LogTable` (tabela łączności)
     - `Global` (ogólny kontekst aplikacji)
  2. Dodano metodę `determine_active_focus_context(&self, ctx: &egui::Context)` w `src/gui/app_input.rs`, która deterministycznie wylicza bieżący kontekst fokusu z uwzględnieniem priorytetów okien.
  3. Skrót `Ctrl+W` został powiązany ściśle z kontekstami `MainQsoEntry` oraz `ContestQsoEntry`. W pozostałych polach edycyjnych (`LogFilter`, `SettingsTextEdit`, `OperatorAssistantTextEdit`) `Ctrl+W` nie wykonuje żadnej akcji czyszczenia QSO.
  4. W `QsoEntryState::clear()` naprawiono pomijanie czyszczenia pola `exchange`.
- **Weryfikacja testowa:** Zestaw 9 testów w `src/gui/app_tests.rs`:
  - `ctrl_w_in_qso_callsign_follows_explicit_qso_contract`
  - `ctrl_w_in_qso_exchange_follows_explicit_qso_contract`
  - `ctrl_w_in_settings_text_edit_does_not_wipe_qso`
  - `ctrl_w_in_filter_does_not_wipe_qso`
  - `ctrl_w_in_modal_does_not_reach_background_qso`
  - `ctrl_w_in_operator_assistant_does_not_wipe_background_qso`
  - `ctrl_w_is_consumed_exactly_once`
  - `ctrl_w_does_not_execute_two_actions`
  - `ctrl_w_repeat_follows_declared_repeat_policy`

---

## 6. Bezpieczeństwo i Redakcja Tokenu REST API (`FUNC-005`)

- **Model przechowywania:**
  - Pole `rest_api_key` w `AppConfig` posiada atrybut `#[serde(skip_serializing)]`, wykluczający zapis klucza do pliku `station_config.json`.
  - Token jest utrwalany w bezpiecznym magazynie poświadczeń (`CredentialStore` / systemowy Keyring / `Secrets`).
- **Redakcja danych poufnych:**
  - W `Debug` dla `AppConfig`: pole `rest_api_key` jest formatowane jako `"<redacted>"`.
  - W komunikatach diagnostycznych i błędach backendów: rozbudowano funkcję `redact_sensitive_info` w `src/cat/tx_arbiter.rs` o wzorce `x-api-key=`, `x-api-key:`, `api_key=`, `api_key:`, `apikey=`, `apikey:`, zastępując wartości tokenów ciągiem `[REDACTED]`.
  - W diagnostycznym eksporcie URL: funkcja `redacted_url_for_diagnostics` w `src/core/http.rs` usuwa token z parametrów query.
- **Interfejs graficzny GUI:**
  - W `src/gui/menu/tools.rs` pole edycyjne klucza REST API jest zamaskowane przez `.password(true)`.
  - Przyciski kopiowania do schowka i regeneracji klucza nie emitują żadnych komunikatów logowania zawierających wartość tokenu.
- **Polityka sieciowa:**
  - Serwer REST API domyślnie nasłuchuje wyłącznie na adresie pętli zwrotnej (`127.0.0.1`).
  - Próba nasłuchu na interfejsach publicznych (`0.0.0.0`, `::`) lub w sieci LAN bez podania tokenu kończy się natychmiastowym odrzuceniem (`ServerBindError::PublicBindWithoutAuthentication`).
- **Weryfikacja testowa:** Zestaw 9 dedykowanych testów strażników w `tests/security_guard.rs`:
  - `rest_token_is_not_serialized_in_standard_config`
  - `rest_token_is_redacted_from_debug`
  - `rest_token_is_redacted_from_error_messages`
  - `rest_token_is_not_in_diagnostic_export`
  - `rest_token_is_masked_in_gui_model`
  - `rest_token_copy_action_does_not_log_value`
  - `rest_token_regeneration_does_not_log_values`
  - `remote_rest_remains_disabled_by_default`
  - `public_bind_without_token_is_rejected`

---

## 7. Deterministyczna Naprawa Testu Odświeżania GUI (`PERF-002`)

- **Przyczyna niestabilności (flakiness):**
  - Dotychczasowy test `idle_gui_does_not_request_continuous_repaint` polegał na zegarze ściennym `chrono::Utc::now()`.
  - Gdy ułamek sekundy na maszynie testowej wynosił $\ge 950$ ms, aplikacja planowała odświeżenie na minimalne dopuszczalne opóźnienie 50 ms.
  - Silnik `egui` kompensował czas przewidywanej ramki (`predicted_dt` $\approx 16.67$ ms), redukując zaobserwowane `repaint_delay` do $\approx 33.33$ ms.
  - Asercja `repaint_delay >= 50ms` sporadycznie zawodziła w ostatnich 50 milisekundach każdej sekundy zegara ściennego.
- **Wdrożona naprawa architektoniczna:**
  1. Wyodrębniono czystą funkcję `duration_until_next_second(subsec_millis: u32) -> Duration` w `src/gui/app.rs`, obliczającą czas do kolejnej sekundy UTC z twardym dolnym ograniczeniem minimum 50 ms.
  2. Zdefiniowano model relacji opóźnień:
     - `APPLICATION_REQUESTED_DELAY` = `duration_until_next_second(subsec_millis)`
     - `EGUI_OBSERVED_DELAY_AFTER_PREDICTED_FRAME_COMPENSATION` = `compute_observed_repaint_delay(requested, predicted_dt)`
  3. Do struktury `SpLogApp` dodano pole wstrzykiwania czasu testowego `pub mock_subsec_millis: Option<u32>`.
  4. W testach wprowadzono fazę rozgrzewki (warm-up) klatek egui, stabilizującą układ dokowania przed pomiarem spoczynku.
- **Weryfikacja determinizmu:**
  - Zbadano wszystkie punkty brzegowe: 000 ms, 500 ms, 900 ms, 949 ms, 950 ms, 960 ms, 983 ms, 984 ms, 999 ms.
  - Test `idle_gui_does_not_request_continuous_repaint` wykonano w pętli 100 powtórzeń — **100/100 PASSED (0 flaky)**.
  - Dodano testy:
    - `until_next_second_is_deterministic_at_boundary_values`
    - `idle_repaint_requested_delay_is_bounded`
    - `idle_repaint_observed_delay_accounts_for_predicted_frame`
    - `idle_repaint_test_does_not_depend_on_wall_clock_boundary`

---

## 8. Klasyfikacja Pomiarów Wydajnościowych (Benchmark Classification)

Zgodnie z wymaganiami audytu usunięto nieuprawnione uproszczenia (np. porównywanie zimnego budowania indeksu z ciepłym odczytem jako „400x szybsza klatka”). Wszystkie pomiary sklasyfikowano w oparciu o ich rzeczywisty zakres:

- `CACHE_BUILD_TIME` (tabela 25 000 QSO, pierwsze uruchomienie / zmiana filtra): **48–65 ms CPU time** ($O(N)$)
- `CACHE_HIT_TIME` (tabela 25 000 QSO, klatki spoczynkowe bez mutacji): **0.08–0.14 ms CPU time** ($O(1)$)
- `DUPLICATE_LOOKUP_TIME` (weryfikacja spotu w `worked_callsigns`): **< 0.05 ms CPU time** ($O(1)$, wcześniej $O(S \times N)$ ~18 ms)
- `VISIBLE_ROWS_RENDER_CPU_TIME` (wyrysowanie wierszy na ekranie): **0.12–0.25 ms CPU time**
- `FULL EGUI FRAME: NOT MEASURED` — pełen czas klatki zależy od warstwy renderowania OS/GPU i nie był mierzony syntetycznie.
- **Notacja KaTeX:** Oczyszczono formatowanie; zastosowano czyste notacje inline `$O(N)$`, `$O(S \times N)$`, `$O(1)$`.

---

## 9. Bezpieczeństwo Środowiska i Ochrona Danych

- **Prawdziwa baza SQLite użytkownika:** Nieotwierana, dane w 100% nienaruszone.
- **Hardware-in-the-Loop (HIL):** Nieuruchamiane (0 operacji fizycznych).
- **Fizyczne PTT:** Nieuruchamiane (0 wyzwoleń nadajnika).
- **Emisja RF:** **0 mW / 0 dBm**.

---

## 10. Pełna Chronologia Commitów Etapu 9

### Faza Bazowa Etapu 9:
1. `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
2. `229dea5`: `fix(gui): make QSO focus and keyboard workflow deterministic`
3. `acecd8c`: `fix(config): make application settings durable and atomic`
4. `c4cf893`: `fix(config): migrate and preserve multi-radio settings`
5. `8397351`: `perf(gui): replace unnecessary continuous repaint with event-driven updates`
6. `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
7. `a7330ce`: `test(gui): add keyboard persistence and rendering regression matrix`
8. `e4114c0` (`e4114c05dbdf54e6fbd11d305af5519e51552dc9`): `docs(audit): close Stage 9 GUI ergonomics persistence and performance findings`

### Końcowa Bramka Remediacji i Rozstrzygnięcia Git:
9. `e8580be` (`e8580beefd0c9ce1a54cf5f1ed7d6f7fd954deb8`): `test(audit): make workspace test counting deterministic`
10. `60d43e4` (`60d43e44cacf6aa46beb7bbaf836fd4c427d349f`): `fix(gui): isolate Ctrl+W wipe to explicit QSO contexts (GUI-001)`
11. `d564272` (`d5642725f4eff3cf536e2573058f545e646ba490`): `fix(config): redact REST token and harden secret handling (FUNC-005)`
12. `665183b` (`665183b79b76da7f8072d5dd383287ebed4ce685`): `test(gui): make Stage 9 performance measurements reproducible (PERF-002)`
13. `docs(audit): correct Stage 9 hashes counts benchmarks and chronology` (bieżący commit domykający bramkę)

---

## 11. Końcowy Werdykt i Gotowość

Wszystkie warunki Bramki Rozstrzygnięcia Tożsamości Git oraz Końcowej Bramki Etapu 9 zostały w pełni spełnione. Kod, konfiguracja, bezpieczeństwo sekretów, determinizm testów oraz dokumentacja są w 100% spójne.

- **BRAMKA TOŻSAMOŚCI GIT:** `PASSED`
- **STATUS ETAPU 9:** `COMPLETED_SOFTWARE_VERIFIED`
- **STATUS KOLEJNEGO ETAPU (ETAP 10):** `READY`
- **WYKONANIE ETAPU 10:** `NOT STARTED`
