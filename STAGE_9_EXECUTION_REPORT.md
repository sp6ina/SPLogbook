# RAPORT Z WYKONANIA ETAPU 9: ERGONOMIA KLAWIATUROWA, TRWAŁOŚĆ KONFIGURACJI I OPTYMALIZACJA RENDEROWANIA GUI

## 1. Metadane Wykonania
- **Rzeczywista data wykonania:** 2026-11-06
- **Deklarowana data robocza projektu:** 2026-11-06
- **Strefa czasowa (Timezone):** `Europe/Warsaw` (`+01:00`)
- **Status końcowy:** `COMPLETED_SOFTWARE_VERIFIED`

---

## 2. Tożsamość Git i Rewizje
- **Stan wejściowy repozytorium przed Etapem 9:** `eb3e7aa5992923769c09c3132bb82f508216c59b` (`docs(audit): close Stage 8 protocol and security gate`)
- **Kanoniczny punkt wejściowy Końcowej Bramki Etapu 9:** `e4114c05dbdf54e6fbd11d305af5519e51552dc9` (`docs(audit): close Stage 9 GUI ergonomics persistence and performance findings`)
- **Rozstrzygnięcie tożsamości Git (Ścieżka A):** Wcześniejszy kontrakt wejściowy promptu wskazywał błędnie rozszerzony hash `e4114c0a52df2ca7ee8478d8a7c29be1a7dc2612`. Weryfikacja `git cat-file -e` potwierdziła, że taki obiekt nie istnieje w Git (`OBJECT_NOT_FOUND`). Skrót `e4114c0` jednoznacznie i bezkolizyjnie (`git rev-parse --disambiguate=e4114c0`) rozwija się wyłącznie do kanonicznego commita `e4114c05dbdf54e6fbd11d305af5519e51552dc9`. Repozytorium jest w 100% spójne.
- **Gałąź robocza:** `repair/full-system-remediation`
- **Gałąź główna:** `main` (synchronizowana ściśle `--ff-only`)
- **Śledzenie zdalne:** `origin/main` (synchronizowane `--ff-only`)
- **Force-push:** NIE UŻYTO (0 zdarzeń)

---

## 3. Integralność Zależności i Hashe Plików Manifestu
Przed i po zakończeniu Etapu 9 pliki manifestów Cargo pozostały nienaruszone:
- `Cargo.toml` SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (niezmieniony)
- `Cargo.lock` SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (niezmieniony)
- **Nowe zależności zewnętrzne:** 0
- **Aktualizacje wersji crates:** 0

---

## 4. Stan Baseline i Weryfikacja Początkowa
Przed przystąpieniem do Etapu 9 zweryfikowano stan wyjściowy:
- Wszystkie testy programowe Etapów 1–8: 2071 passed, 0 failed, 10 ignored laboratoryjnych HIL.
- Brak wiszących procesów i czyste drzewo robocze.
- Bezpieczeństwo bazy danych użytkownika i sprzętu radiowego potwierdzone.

---

## 5. Zamknięte Znaleziska Audytu i Przyczyny Pierwotne (Root Causes)

| Identyfikator | Tytuł kanoniczny | Przyczyna pierwotna (Root Cause) | Wdrożona remediacja |
|---|---|---|---|
| `GUI-001` | Blokada skrótów `Ctrl+...` przez aktywny `TextEdit`, brak `Enter`/`Esc` w zawodach i brak determinizmu fokusu | `egui_wants_keyboard_input()` zwracało `true` w polach tekstowych, blokując globalne skróty akcji (`Ctrl+W`, `Ctrl+L`). Brak obsługi `Key::Enter` i `Key::Escape` w oknie zawodów `contest.rs`. | Zaimplementowano centralny `ShortcutRegistry` rozróżniający konteksty (`Global`, `QsoEntry`, `Contest`, `LogTable`). Wyciągnięto skróty z modyfikatorem `Ctrl` przed blokadę `TextEdit`. Dodano obsługę `Enter` i `Esc` w `contest.rs` z deterministycznym powrotem fokusu na pole znaku. |
| `GUI-002` | Pływający widżet „Asystent Operatora” nie posiada przycisku zamknięcia, a jego stan nie zapisuje się w konfiguracji | Okno `egui::Window` nie miało parametru `.open(&mut ...)`, a flaga widoczności nie była utrwalana w konfiguracji. | Dodano `.open(&mut app.gui.windows.show_operator_assistant)` w nagłówku oraz pełną dwukierunkową synchronizację w `snapshot_config()`. |
| `GUI-003` | Fałszywy odczyt S-metra (`S9+20dB`, `-53 dBm`) i mocy (`100 W`) przed połączeniem radia | Zahardkodowane wartości testowe w `SpLogApp::build` dla początkowego stanu radia. | Zainicjalizowano spoczynkowy stan `RigState` kanonicznymi wartościami `s_meter_dbm: -127.0`, `s_meter_unit: "S0"`, `rf_power_watts: 0.0`. |
| `FUNC-005` | Utrata części ustawień po restarcie programu z powodu niekompletnego `SpLogApp::snapshot_config()` | `snapshot_config()` pomijało struktury CW, REST API, alerty solarne, satelity, layout/okna, motyw i Radio 2. Zapis pliku konfiguracyjnego nie był atomowy. | Uzupełniono pełną synchronizację w `snapshot_config()`, dodano walidację schematu, atomowy zapis pliku `.tmp` z bezpieczną zamianą `rename_atomic` i tworzeniem kopii `.bak`. |
| `PERF-002` | Niepotrzebny continuous repaint 250ms w GUI oraz kosztowne operacje $O(N)$ i $O(S \times N)$ w każdej klatce | Bezwarunkowe wywołanie `request_repaint_after(250ms)` w pętli renderowania; brak buforowania posortowanych rekordów i tabeli zawodów; $O(S \times N)$ duplikatów spotów. | Wprowadzono harmonogram zegara `until_next_second`, event-driven wake context z `EventBus` i `CatRuntime`, buforowanie `sorted_indices` i `worked_callsigns` z kluczem rewizji `LogViewCacheKey`, buforowanie scoreboardu z kluczem `ContestCacheKey`, weryfikację duplikatów w $O(1)$. |

---

## 6. Centralna Mapa Skrótów Klawiszowych (`ShortcutRegistry`)

### Rejestr Skrótów i Obsługiwane Konteksty
- **`Global`**: Aktywne w całej aplikacji, o ile nie zachodzi kolizja z edycją tekstu:
  - `Ctrl+N`: Ustawienie fokusu na pole znaku (Callsign)
  - `Ctrl+F`: Fokus na pole wyszukiwarki dziennika
  - `Ctrl+P`: Otwarcie palety poleceń
  - `Ctrl+K`: Otwarcie terminala CW / okna makr CW
  - `F1`–`F8`: Wysłanie skonfigurowanego makra CW przez podłączony klucz/CAT
- **`QsoEntry`** (Główny formularz łączności):
  - `Ctrl+L` / `Ctrl+S`: Zapis bieżącej łączności (dozwolone także podczas edycji tekstu w polach RST, Name, etc.)
  - `Ctrl+W`: Wycyszczenie formularza QSO (Wipe) i powrót fokusu na znak (dozwolone podczas edycji tekstu)
  - `Enter`: Zapis łączności (jeżeli fokus w polach formularza)
  - `Esc`: Wycyszczenie pól formularza (Wipe) i powrót fokusu na znak
  - `Tab` / `Shift+Tab`: Przejście w przód / w tył w deterministycznym cyklu pól: Callsign -> RST Sent -> RST Rcvd -> Name -> QTH -> Comment -> Callsign
- **`Contest`** (Okno zawodów):
  - `Enter`: Natychmiastowa walidacja, zapis łączności do logu i powrót fokusu na pole Call
  - `Esc`: Wycyszczenie formularza łączności konkursowej (Wipe) i powrót fokusu na pole Call
  - `Space`: Szybkie przełączenie między polem Callsign a polem Exchange (RST/Nr)
  - `Tab`: Cykliczna nawigacja po polach wymiany konkursowej
- **`LogTable`** (Tabela logbooka):
  - `Delete`: Usunięcie zaznaczonej łączności (wyłącznie gdy fokus NIE znajduje się w `TextEdit`)
  - `Up` / `Down`: Nawigacja po wierszach tabeli łączności

### Ochrona Edycji Tekstu (`TextEditShortcutProtection`)
- Zdarzenia jednoliterowe (np. litery `D`, `W`, cyfry `1`-`8`, klawisz `Delete`, `Backspace`) są w pełni przekazywane do aktywnego widżetu `TextEdit` bez przechwytywania przez skróty nawigacyjne aplikacji.
- Skróty funkcyjne z modyfikatorem `Ctrl` (`Ctrl+W`, `Ctrl+L`, `Ctrl+S`, `Ctrl+N`, `Ctrl+F`) są bezpiecznie przepuszczane i obsługiwane bez niszczenia wprowadzanego tekstu.

---

## 7. Kontrakt Fokusu (`FocusContract`)
- **Determinizm:** Fokus nie jest wymuszany bezwarunkowo w każdej klatce renderowania.
- **Żądania sterowane zdarzeniami (`FocusRequest`):** Fokus zmienia się wyłącznie w odpowiedzi na konkretne zdarzenie:
  - Po pomyślnym zapisie łączności (`Enter`, `Ctrl+L`, kliknięcie „Zapisz QSO”) fokus wraca na pole Callsign.
  - Po wyczyszczeniu formularza (`Esc`, `Ctrl+W`) fokus wraca na pole Callsign.
  - Po kliknięciu `Ctrl+N` fokus ustawia się na pole Callsign.
  - Po kliknięciu `Ctrl+F` fokus ustawia się na pole wyszukiwarki.
- **Ochrona danych przy błędzie zapisu:** Jeżeli łączność nie przejdzie walidacji (np. pusty znak, nieprawidłowy format wymiany), dane wprowadzone przez operatora NIE są czyszczone, a kursor pozostaje w edytowanym polu z komunikatem błędu.

---

## 8. Przepływ Pracy QSO (QSO Workflow)
- **Zapisywanie w pileupie:** Operator wpisuje znak, naciska `Space` lub `Tab`, wpisuje raport/wymianę, naciska `Enter`. Łączność zostaje zapisana do bazy SQLite, tabela dziennika i licznik łączności zostają zaktualizowane, a kursor automatycznie znajduje się w polu znaku gotowy do kolejnej łączności.
- **Routing klawisza Escape:**
  1. Jeżeli otwarte jest modalne okno dialogowe lub paleta poleceń, `Esc` zamyka to okno w pierwszej kolejności, bez modyfikowania danych formularza QSO.
  2. Jeżeli brak okien modalnych, `Esc` wykonuje operację Wipe na formularzu łączności i przywraca fokus na znak.

---

## 9. Konfiguracja Aplikacji: Schema, Zapis Atomowy i Migracje
- **Schema i synchronizacja:** Struktura `AppConfig` odzwierciedla wszystkie aspekty działania aplikacji:
  - `station`: znak stacji, lokator QTH, imię operatora
  - `ui`: motyw (`theme_preset`), widoczność asystenta (`show_operator_assistant`), układ paneli
  - `cw`: prędkość WPM, makra funkcyjne `macro_f1`..`macro_f8`
  - `runtime`: parametry REST API (`enabled`, `port`, `token`)
  - `solar`: alerty pasmowe (`enabled`, moc, emisje)
  - `satellites`: parametry śledzenia i TLE
  - `rig`: konfiguracja profilu Radio 1 i Radio 2 (`active_profile`, `profiles`)
- **Atomowość zapisu (`rename_atomic`):**
  1. Serializacja konfiguracji do JSON i walidacja schematu.
  2. Zapis do unikalnego pliku tymczasowego w tym samym katalogu: `config.json.tmp.<uuid>`.
  3. Wywołanie `sync_all()` w celu wymuszenia opróżnienia buforów systemu operacyjnego na dysk.
  4. Rotacja istniejącego poprawnego pliku do kopii bezpieczeństwa: `config.json.bak`.
  5. Atomowa zamiana pliku tymczasowego na plik docelowy `config.json`.
  6. W przypadku jakiegokolwiek błędu I/O w trakcie zapisu, poprzedni plik konfiguracyjny pozostaje w 100% nienaruszony.
- **Migracje ze starszych wersji:** W starszych konfiguracjach jednoradiowych konfiguracja `rig` jest transparentnie migrowana do słownika profili `profiles` z zachowaniem identyfikatorów, portów szeregowych i prędkości baud.

---

## 10. Trwałość Konfiguracji Multi-Radio
- Profile radiowe Radio 1 i Radio 2 posiadają niezależną tożsamość i parametry.
- Zmiana aktywnego radia w interfejsie graficznym natychmiast synchronizuje dane aktywnego profilu.
- Zapis konfiguracji zachowuje parametry obu transceiverów (model, port UART, baudrate, model CI-V, ustawienia Hamlib/TCI/FLRig).
- Przy ponownym uruchomieniu aplikacji oba profile są poprawnie wczytywane bez nadpisywania jednego radia drugim.

---

## 11. Audyt i Eliminacja Niepotrzebnego Repaintu GUI
- **Stan przed zmianą:** W `src/gui/app.rs` w metodzie `render_main_ui` znajdowało się wywołanie:
  `ui.ctx().request_repaint_after(Duration::from_millis(250));`
  Powodowało to, że aplikacja w stanie spoczynku (bez aktywności użytkownika i bez telemetrii z radia) wybudzała pętlę renderowania 4 razy na sekundę, zużywając cykle CPU.
- **Stan po zmianie:**
  1. Usunięto stałe wywołanie 250ms.
  2. Wprowadzono precyzyjny harmonogram zegara czasu rzeczywistego `until_next_second`, planujący kolejne wybudzenie dokładnie na początek kolejnej sekundy UTC (opóźnienie spoczynkowe ~600–1000 ms w zależności od ułamka sekundy).
  3. Komponenty `EventBus` i `CatRuntime` zostały wyposażone w mechanizm powiadamiania kontekstu `egui::Context::request_repaint()`. Każde zdarzenie asynchroniczne (nowy pakiet częstotliwości CAT, zmiana PTT, odebrany spot klastra DX) budzi interfejs natychmiast bez żadnego opóźnienia.
- **Weryfikacja testowa:** Test `idle_gui_does_not_request_continuous_repaint` potwierdził, że w stanie spoczynku żądanie odświeżenia nie jest zgłaszane z interwałem 250 ms, a wybudzenie następuje na początku sekundy UTC lub po zdarzeniu telemetrii.

---

## 12. Pomiary Wydajności i Czasów Renderowania
- **Środowisko testowe:** System Windows x86_64, profil testowy Cargo (unoptimized + debuginfo).
- **Klasyfikacja pomiarów i unikanie nieuprawnionych uproszczeń:**
  - Pomiary dotyczą wyselekcjonowanych procedur CPU, a nie pełnego czasu ramki egui/eframe.
  - `FULL EGUI FRAME: NOT MEASURED` (pełny czas klatki zależy od backendu wgpu/glow i środowiska OS).
  - Usunięto nieuzasadnione twierdzenia „400x faster frame rendering”, gdyż porównywały one cold cache build z warm cache hit.
- **Wyniki pomiarów CPU według kanonicznej klasyfikacji:**
  - `CACHE_BUILD_TIME` (tabela 25 000 rekordów QSO przy zimnym buforze): ~48–65 ms CPU time.
  - `CACHE_HIT_TIME` (`sorted_indices` przy braku mutacji danych i filtra): **0.08–0.14 ms** CPU time.
  - `DUPLICATE_LOOKUP_TIME` ($O(1)$ set lookup w `worked_callsigns`): **< 0.05 ms** CPU time (wcześniej $O(S \times N)$ iteracyjne przeszukiwanie ~18 ms).
  - `VISIBLE_ROWS_RENDER_CPU_TIME` (renderowanie wierszy widocznych w oknie): **0.12–0.25 ms** CPU time.
- **Złożoność obliczeniowa:** Notacja matematyczna KaTeX inline: $O(N)$ dla zimnego budowania indeksu, $O(1)$ dla wyszukiwania duplikatów i trafień pamięci podręcznej, $O(S \times N)$ wyeliminowane z pętli odświeżania spotów.

---

## 13. Pamięć Podręczna (Cache) i Zasady Unieważniania (Invalidation)
- **Klucz rewizji `LogViewCacheKey`:**
  - Składniki: `log_revision`, `filter_hash`, `sort_column`, `sort_ascending`, `journal_id`.
  - Unieważnienie: Dodanie nowego QSO, usunięcie, edycja, zmiana dziennika, wpisanie nowego znaku w filtrze lub kliknięcie nagłówka kolumny powoduje natychmiastowe przeliczenie indeksów.
- **Klucz rewizji `ContestCacheKey`:**
  - Składniki: `log_revision`, `contest_id`, `category_operator`, `band_filter`.
  - Unieważnienie: Każda nowa łączność zalogowana w zawodach unieważnia pamięć podręczną scoreboardu i matrycy mnożników.
- **Pamięć podręczna radia `rig_cache`:**
  - Powiązana ściśle z tożsamością `RigId` oraz rewizją telemetrii. Przełączenie aktywnego profilu radia natychmiast unieważnia pamięć podręczną, zapobiegając przenikaniu wskazań częstotliwości i mocy między radiami (`stale_cache_never_crosses_profiles`).
  - Zaimplementowana w `FormVariantCache` z metodami dostępowymi w `CatState`, co zagwarantowało zachowanie limitu poniżej 30 pól w strukturze `CatState`.

---

## 14. Ochrona przed Regresją Funkcjonalności Etapu 8
- Pełen zestaw testów protokołów wieloradiowych, rejestru `PhysicalEndpointRegistry`, menedżera portów `ProxyPortManager`, endpointów REST API i mostu rozkazów natywnych został uruchomiony.
- **Wynik:** 0 regresji funkcjonalnych, 0 konfliktów portów.

---

## 15. Macierz Stabilności (Stability Matrix)
Wykonano 5-krotne powtórzenie zestawu 12 krytycznych testów integracyjnych i wydajnościowych Etapu 9:
1. `keyboard_shortcuts_ctrl_w_and_ctrl_l_work_when_input_focused`
2. `contest_enter_logs_and_returns_focus_to_callsign`
3. `escape_clears_qso_fields_in_contest_and_main_entry`
4. `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`
5. `config_atomic_write_preserves_previous_file_on_write_failure`
6. `multi_radio_settings_are_preserved_across_save_and_reload`
7. `idle_gui_does_not_request_continuous_repaint`
8. `incoming_telemetry_event_requests_gui_repaint`
9. `cache_is_reused_when_inputs_are_unchanged`
10. `gui_state_lock_is_released_before_render`
11. `gui_does_not_hold_lock_during_config_write`
12. `slow_background_task_does_not_block_gui_state_access`

**Wyniki:**
- Próba 1: 12/12 PASSED (0 failures, 0 flaky)
- Próba 2: 12/12 PASSED (0 failures, 0 flaky)
- Próba 3: 12/12 PASSED (0 failures, 0 flaky)
- Próba 4: 12/12 PASSED (0 failures, 0 flaky)
- Próba 5: 12/12 PASSED (0 failures, 0 flaky)
- **Łącznie:** 60/60 PASSED, 0 failures, 0 flaky.

---

## 16. Weryfikacja Statyczna Clippy i Formatowanie
- `cargo fmt --check`: ZALICZONE (0 błędów formatowania)
- `cargo +stable clippy --workspace --all-targets --all-features -j 2`: ZALICZONE (0 ostrzeżeń w kodzie Etapu 9)
- Brak `#[allow(...)]`, brak `unwrap()` w kodzie produkcyjnym, brak bloków `unsafe`.

---

## 17. Pełna Weryfikacja Zestawu Testów
- **Łączna liczba zarejestrowanych testów:** 2172
- **Aktywne testy programowe:** 2162 PASSED, 0 FAILED
- **Testy ignorowane (fizyczny HIL):** 10 IGNORED (`#[ignore = "hardware-in-the-loop"]`)
- **Podział na pakiety i targety:**
  - `splogbook` (biblioteka rdzenna): 1543 passed, 0 failed
  - `security_guard` (testy strażników bezpieczeństwa): 42 passed, 0 failed
  - `i18n_guard` (testy strażników internacjonalizacji): 15 passed, 0 failed
  - `theme_guard` (test strażnika motywów graficznych): 1 passed, 0 failed
  - `cat_native_6a` (architektura wieloradiowa A): 165 passed, 0 failed
  - `cat_native_6b` (architektura wieloradiowa B): 99 passed, 0 failed
  - `cat_native_7` (protokoły sprzętowe i HIL mock): 297 passed, 0 failed, 10 ignored
  - Binaria aplikacji (`SPLogbook`, `splogbook-signer`): 0 passed (brak testów binarnych)
  - Testy dokumentacyjne (`cargo test --doc`): 0 failed
- **Testy flaky:** 0 (test `idle_gui_does_not_request_continuous_repaint` w pełni ustabilizowany deterministycznie)
- **Cargo Audit:** 0 podatności

---

## 18. Wykaz Commitów Etapu 9

### Faza Wdrożeniowa Etapu 9
1. `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
2. `229dea5`: `fix(gui): make QSO focus and keyboard workflow deterministic`
3. `acecd8c`: `fix(config): make application settings durable and atomic`
4. `c4cf893`: `fix(config): migrate and preserve multi-radio settings`
5. `8397351`: `perf(gui): replace unnecessary continuous repaint with event-driven updates`
6. `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
7. `a7330ce`: `test(gui): add keyboard persistence and rendering regression matrix`
8. `e4114c0` (`e4114c05dbdf54e6fbd11d305af5519e51552dc9`): `docs(audit): close Stage 9 GUI ergonomics persistence and performance findings`

### Końcowa Bramka Remediacji i Rozstrzygnięcia Tożsamości Git
9. `e8580be` (`e8580beefd0c9ce1a54cf5f1ed7d6f7fd954deb8`): `test(audit): make workspace test counting deterministic`
10. `60d43e4` (`60d43e44cacf6aa46beb7bbaf836fd4c427d349f`): `fix(gui): isolate Ctrl+W wipe to explicit QSO contexts (GUI-001)`
11. `d564272` (`d5642725f4eff3cf536e2573058f545e646ba490`): `fix(config): redact REST token and harden secret handling (FUNC-005)`
12. `665183b` (`665183b79b76da7f8072d5dd383287ebed4ce685`): `test(gui): make Stage 9 performance measurements reproducible (PERF-002)`
13. `docs(audit): correct Stage 9 hashes counts benchmarks and chronology` (zamknięcie Bramki Końcowej)

---

## 19. Synchronizacja Gałęzi Git
- Wszystkie commity zostały scalone do gałęzi `main` w trybie fast-forward (`--ff-only`).
- Gałąź zdalna `origin/main` została zaktualizowana bez użycia force-push.
- Stan roboczy `repair/full-system-remediation` jest w 100% zsynchronizowany z `main` i `origin/main`.

---

## 20. Ochrona Danych Użytkownika i Środowiska Sprzętowego
- Prawdziwa baza danych SQLite użytkownika nie została otwarta ani zmodyfikowana.
- Żadne fizyczne operacje HIL nie zostały uruchomione.
- Fizyczne PTT nie zostało aktywowane (0 operacji nadawania).
- Emisja fal elektromagnetycznych (RF emission): **0 mW / 0 dBm**.

---

## 21. Końcowy Werdykt
Etap 9 został w pełni zrealizowany, zweryfikowany testami programowymi i macierzą stabilności, spełniając wszystkie wymagania architektoniczne, wydajnościowe i bezpieczeństwa.

**STATUS ETAPU 9:** `COMPLETED_SOFTWARE_VERIFIED`
