# Plan Normalizacji Jakości i Spójności Kodu (`SPLogbook`)

**Data audytu:** 2026-09-29  
**Zakres analizy:** `src/`, `build.rs`, `Cargo.toml`, skrypty pomocnicze, workflow GitHub Actions, dokumentację techniczną powiązaną z kodem.

# 1. Zakres i metodologia

Baza kodu SPLogbook była przedmiotem audytu pod kątem niespójności stylowych, usterek funkcjonalnych, duplikacji oraz naruszeń standardów domenowych (w tym ADIF 3.1.7). Ustalenia zostały rygorystycznie podzielone na błędy funkcjonalne i bezpieczeństwa (Część A), normalizację jakości kodu (Część B) oraz duże refaktoryzacje (Część C).

Każdy zidentyfikowany problem posiada "Status techniczny" oraz "Status wykonawczy". Żaden problem nie może zostać zaimplementowany, jeżeli ma status wykonawczy inny niż "GOTOWY DO IMPLEMENTACJI".

Odnośnie usuwania martwego kodu: publiczne symbole mogą nie generować takich samych ostrzeżeń o nieużyciu jak symbole prywatne, ale brak użycia wewnątrz repozytorium nie dowodzi, że symbol jest zbędny. Przed usunięciem należy sprawdzić API, dodatkowe binarki, testy, re-eksporty i planowane użycie, co znajduje odzwierciedlenie w przypisanym etapie analitycznym przed faktycznym usuwaniem.

# 2. Część A: Błędy funkcjonalne i bezpieczeństwa

Obejmuje możliwe przekłamanie danych, złamanie standardu lub protokołu, utratę danych, uszkodzenie instalacji, niepoprawną logikę domenową, oraz błędy bezpieczeństwa.

#### `PROB-P0-01` — Ciche podstawianie bieżącej daty i godziny (`Utc::now()`) w eksporcie SOTA CSV
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU P0
- **Priorytet:** P0 Krytyczny
- **Plik i zakres linii:** `src/core/sota_export.rs:34-55` (`format_sota_date`, `format_sota_time`)
- **Opis:** Funkcje z cichym fallbackiem na bieżący czas. Powoduje to ciche generowanie nieprawdziwych danych w eksportowanym logu CSV SOTA (utrata autentyczności łączności).
- **Rekomendowana poprawka:** Zwracać błąd (`Result<String, String>`) i go propagować, wycofując wymuszenie daty wyeksportowania.

#### `PROB-N1MM-A` — Niepoprawne usuwanie sufiksu pasma
- **Status techniczny:** ZAKOŃCZONY (DONE)
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU P0
- **Priorytet tymczasowy:** P1 Wysoki
- **Potencjalny priorytet:** P0 po potwierdzeniu specyfikacji i wpływu
- **Plik:** `src/digital/n1mm.rs`
- **Opis:** Usuwa sufiks 'm', z '70cm' powstaje '70c'.
- **Rekomendowana poprawka:** Najpierw zidentyfikować w dokumentacji protokołu N1MM poprawną wartość dla pasm ukf, i na jej podstawie poprawić kod po weryfikacji.

#### `PROB-N1MM-B` — Niespójne jednostki częstotliwości w ContactInfo i RadioInfo
- **Status techniczny:** ZAKOŃCZONY (DONE)
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU P0
- **Priorytet tymczasowy:** P1 Wysoki
- **Potencjalny priorytet:** P0 po potwierdzeniu specyfikacji i wpływu
- **Plik:** `src/digital/n1mm.rs`
- **Opis:** Mieszanie jednostek kHz i Hz w tych samych ramkach UDP N1MM.
- **Rekomendowana poprawka:** Zweryfikować oficjalną dokumentację dla ContactInfo i RadioInfo, zastosować się do niej.

#### `PROB-P0-03` — Awaryjna poprawka updatera Linux
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU P0
- **Priorytet:** P0 Krytyczny
- **Plik i zakres linii:** `src/cloud/updater.rs:267-276, 368-375`
- **Opis:** Obecny updater może nadpisać instalacyjną binarkę archiwum gzip i trwale zepsuć aplikację.
- **Rekomendowana poprawka:** Zablokować nadpisywanie pliku. Przerwać aktualizację jeśli pobrano tar.gz/deb, wyświetlić komunikat o ręcznej aktualizacji. Zgłaszać error na brak tag_name zamiast wpisywać "nieznana". Żadnego sudo, instalowania z deb czy rozpakowywania. 

#### `PROB-P1-01` — Bezwarunkowa inkrementacja liczników zawodów w SpLogApp
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU P0
- **Priorytet:** P1 Wysoki
- **Plik:** `src/gui/app.rs:2081-2084`
- **Opis:** Zwykłe QSO podnosi `contest_qsos`. Wpływa to na bieżącą sesję GUI i eksport Cabrillo, ale oryginalne dane SQLite zachowują integralność i logikę da się przeliczyć.
- **Rekomendowana poprawka:** Ograniczyć zmianę statystyk do aktywnych zawodów.

#### `PROB-P1-02` — Błędna heurystyka w `ClubRegistry` oraz dane testowe w `callbook.rs`
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/core/clubs.rs`, `src/core/callbook.rs`
- **Opis:** Niepoprawne identyfikowanie klubów przez string `ends_with("CW")`. W callbook.rs wpisane stałe dane dla SP6INA.
- **Rekomendowana poprawka:** Usunąć mechaniczne testowanie końcówki callsign oraz wyprowadzić sztywne dane.

#### `PROB-P1-03A` — Duplikacja logiki prefiksu WPX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/core/awards.rs`, `src/core/prefix.rs`
- **Opis:** Duplikacja `extract_wpx_prefix`, `extract_wpx_base`, sufiksy operacyjne i rozbieżne przypadki brzegowe.

#### `PROB-P1-03B` — Duplikacja ekstrakcji polskiego okręgu SP
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/core/awards.rs`
- **Opis:** Wolna funkcja `extract_sp_district` i metoda `AwardsEngine::get_polish_district`.

#### `PROB-P1-03C` — Podwójna rejestracja danych dyplomowych
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/core/awards.rs`
- **Opis:** Wielokrotne wstawianie tych samych wartości pomiędzy `register_qso_record` a `register_qso_full`.

#### `PROB-P1-04` — Fałszywy fallback `127.0.0.1:7373` przy testowaniu sieci Multi-Op
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/gui/contest.rs`
- **Opis:** Cichy fallback przy błędach, a także używanie blokującego testu połączenia na wątku UI.

#### `PROB-P1-05` — Voice Keyer kluczujący PTT poza kanałem CAT
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/gui/voice_keyer.rs`
- **Opis:** Bezpośrednie omijanie włączonego backendu CAT.

#### `PROB-P1-06` — 5-krotna duplikacja potoku po zapisie QSO
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Priorytet:** P1 Wysoki
- **Plik:** `src/gui/app.rs`, `src/gui/contest.rs`
- **Opis:** Operacje typu wysłanie hooka, online, klaster itp po `save_qso`.

#### `PROB-P1-07` — Brak `+ Send + Sync` w typach błędów w async
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Plik:** `src/cloud/lotw.rs`, `src/cloud/qrz.rs`, itp.


# 3. Część B: Normalizacja jakości i spójności kodu

Obejmuje komentarze, nazewnictwo, lokalne duplikacje, nieidiomatyczne konstrukcje, małe abstrakcje, styl i utrzymywalność.

#### `PROB-P2-01` — Martwy kod GUI i nieosiągalne gałęzie
- **Status techniczny:** WYMAGA TESTU URUCHOMIENIOWEGO
- **Status wykonawczy:** ZABLOKOWANY DO CZASU WERYFIKACJI
- **Priorytet:** P2 Średni
- **Pliki:** `src/gui/app_layout.rs`
- **Opis:** Ok. 1000 linii nieosiągalnych z powodu `if ... return;` albo usunięcia dawnego układu z kolumnami.

#### `PROB-P2-02` — Audyt `rows.flatten()` w SQLite
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni
- **Opis:** Należy przestać stosować mechaniczne `rows.flatten()`, lecz wdrożyć odpowiednie polityki obsługi po przetestowaniu w zadaniu `CQ-2.4.B`.
  - `database.rs`: Proponowana polityka, wymagająca zatwierdzenia w CQ-2.4.B: **Fail-fast**.
  - `database_stats.rs`: Proponowana polityka, wymagająca zatwierdzenia w CQ-2.4.B: **Fail-fast**.
  - `prefix.rs`: Proponowana polityka, wymagająca zatwierdzenia w CQ-2.4.B: **Partial result + warning**.
  - `service_db.rs`: Proponowana polityka, wymagająca zatwierdzenia w CQ-2.4.B: **Partial result + warning**.

#### `PROB-P2-03` — Wrappery viewportu (9x)
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni
- **Pliki:** Różne w `src/gui/`

#### `PROB-P2-03D` — Podejrzane nieużywane pola dyplomowe
- **Status techniczny:** WYMAGA TESTU UŻYCIA
- **Status wykonawczy:** ZABLOKOWANY DO CZASU WERYFIKACJI
- **Priorytet:** P2 Średni
- **Opis:** Identyfikacja i usunięcie nieużywanych w rzeczywistości pól dyplomowych i upewnienie się co do wpływu na dane.

#### `PROB-P2-04` — Nadmiarowe funkcje proxy i struktury jednostkowe
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni
- **Pliki:** `src/gui/command_palette.rs`, itp.

#### `PROB-P2-05` — 20 schematycznych kalkulatorów punktów w contestach
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni
- **Pliki:** `src/core/contest_rules.rs`

#### `PROB-P2-06` — Duplikaty helperów (XML, Text, Socket)
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni

#### `PROB-P2-07` — Optymalizacje w pętli 60Hz UI
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni
- **Opis:** Np. nagminne wyliczanie alokacyjne `pga.rs`, czytanie bazy.

#### `PROB-P2-08` — Niewłaściwe opisy API / mylące skutki
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni
- **Opis:** Zmiana nazw typu `vacuum_if_needed`.

#### `PROB-P2-09` — Niskiej jakości asercje w testach
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P2 Średni

#### `PROB-P3-01` — Komentarze nienaturalne i ślady AI
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P3 Niski
- **Pliki:** `src/core/adif.rs`
- **Opis:** Ślady promptów, narracja git, separatory. Zastąpienie zapisów sztucznych komentarzami opisującymi "dlaczego". 

#### `PROB-P3-02` — Brak diakrytyków i nagłówków SPDX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P3 Niski

#### `PROB-P3-03` — Jednoliterowe i ogólnikowe nazwy zmiennych
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P3 Niski

#### `PROB-P3-04` — Brud w katalogu z analizy i w CI
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P3 Niski
- **Opis:** Usunięcie skryptów do statystyk i ulepszenie github actions.

# 4. Część C: Duże refaktoryzacje wymagające DESIGN.md

Dla wszystkich punktów:
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

1. Dekompozycja SpLogApp (~7000 linii God Object).
2. Wspólny potok zapisu QSO.
3. Normalizacja 53-kolumnowego SQL.
4. Architektura CAT i WebSocket TCI.
5. Synchronizacja sieciowa LAN i P2P (do połączenia albo usunięcia).
6. Większa reorganizacja GUI z pełną lokalizacją (`i18n`).
7. Centralizacja motywów UI.

# 5. Elementy nietypowe, ale uzasadnione

- **ADIF 3.1.7 (MODE i SUBMODE):**
Wewnętrzna reprezentacja FT4 i Q65 jest tłumaczona podczas eksportu na standardową reprezentację ADIF 3.1.7:

MODE=MFSK + SUBMODE=FT4
MODE=MFSK + SUBMODE=Q65

Importer toleruje również spotykane w praktyce bezpośrednie wartości MODE=FT4 i MODE=Q65, aby nie tracić danych z zewnętrznych logów.

FT8, JT65, JT9, WSPR i MSK144 pozostają głównymi wartościami MODE.
- **Wyjątek pasma 60m (USB)**.
- **Sufiksy dla obszarów rosyjskich w DXCC**.
- **Miękka walidacja IOTA/SOTA (UX dla szybkiego logowania w pile-up)**.
- **Rozwiązywanie sprzętu Icom CI-V**.
- **Okno deduplikacji czasowej WSJT-X (5 sekund)**.
- **Odczyt error page serwerów eqsl z weryfikacją JPEG**.
- **Zastępowanie stanu GUI (borrow checker workarounds dla egui_dock)**.

# 6. Fałszywe alarmy

Dla wszystkich poniższych alarmów:
- **Status techniczny:** FAŁSZYWY ALARM
- **Status wykonawczy:** NIE IMPLEMENTOWAĆ

1. Ostrzeżenie przed tajnymi kluczami ("tajne-haslo") w testach `p2p.rs`.
2. `expect()` na `OnceLock<Regex>`.
3. Globalne `clippy::pedantic` disallows na rzutowaniu i limitach liniowych (niezbędne do matematyki GUI w egui).
4. Użycie patternów failsafe jak w przypadku `PoisonError` muteksów.

# 7. Zadania weryfikacyjne

#### CQ-0: Weryfikacja problemów krytycznych
Wynikiem tej analizy jest plik `REPORT_CODE_QUALITY_P0_VERIFICATION.md` służący potwierdzeniu i analizie wpływu problemów:
- fallback Utc::now() podczas eksportu SOTA,
- jednostki częstotliwości N1MM i konwersja pasm centymetrowych,
- zachowanie aktualizatora Linux,
- wpływ zwykłego QSO na liczniki zawodów.

#### CQ-3.1.A: Raport użycia symboli i nieosiągalnych ścieżek
Wynik ma trafić do dokumentu `DEAD_CODE_VERIFICATION_REPORT.md` (bez modyfikacji kodu). Należy potwierdzić wszystkie odniesienia cross-module.

# 8. Plan implementacji małymi krokami

Każde z poniższych zadań nadaje się do osobnego commita, osobnej sesji. 

- **CQ-1.1**: Zwracanie błędu w SOTA CSV zamiast `Utc::now()`.
- **CQ-1.2.A**: N1MM - usuwanie liter przy sufiksach centymetrowych (ZABLOKOWANE, po teście specyfikacji).
- **CQ-1.2.B**: N1MM - wyrównanie jednostek między ContactInfo a RadioInfo (ZABLOKOWANE, po teście specyfikacji).
- **CQ-1.3**: Awaryjna blokada niszczącej auto-instalacji nieobsługiwanych artefaktów Linux.
- **CQ-2.1.A**: Naprawa heurystyki członkostwa klubowego.
- **CQ-2.1.B**: Usunięcie danych demonstracyjnych z produkcyjnego callbooka.
- **CQ-2.2.A**: Konsolidacja extract_wpx_prefix i sufiksów operacyjnych.
- **CQ-2.2.B**: Konsolidacja ekstrakcji polskiego okręgu SP.
- **CQ-2.2.C**: Usunięcie podwójnej rejestracji dyplomów w AwardsEngine.
- **CQ-2.2.D**: Weryfikacja i ewentualne usunięcie martwych pól dyplomowych (ZABLOKOWANE do testów).
- **CQ-2.3.A**: Naprawa testu połączenia Multi-Op.
- **CQ-2.3.B**: Podłączenie Voice Keyera do aktywnego backendu CAT.
- **CQ-2.4.A**: Dodanie Send + Sync do błędów funkcji async.
- **CQ-2.4.B**: Audyt rows.flatten() i wybór polityki obsługi błędów SQLite dla poszczególnych modułów.
- **CQ-2.4.C1**: Obsługa błędów w database.rs.
- **CQ-2.4.C2**: Obsługa błędów w database_stats.rs.
- **CQ-2.4.C3**: Obsługa błędów w prefix.rs.
- **CQ-2.4.C4**: Obsługa błędów w service_db.rs.
- **CQ-3.1.B1**: Nieosiągalne gałęzie wewnątrz funkcji.
- **CQ-3.1.B2**: Prywatne funkcje GUI bez wywołań.
- **CQ-3.1.B3**: Stary układ kolumnowy.
- **CQ-3.1.B4**: Podejrzane publiczne symbole po dodatkowej analizie API (`CQ-3.1.B4` nie może zostać wykonane tylko dlatego, że nie znaleziono wewnętrznego wywołania publicznego symbolu).
- **CQ-3.3.A**: Usunięcie nieużywanych wrapperów i funkcji.
- **CQ-3.3.B**: Uproszczenie command_palette.rs.
- **CQ-3.3.C**: Konsolidacja StateMutation i RigServerCommand.
- **CQ-3.3.D**: Uporządkowanie aliasów eQSL.
- **CQ-3.3.E**: Redukcja duplikacji reguł zawodów.
- **CQ-3.4.A**: Helpery XML.
- **CQ-3.4.B**: Helpery tekstowe i SQL LIKE.
- **CQ-3.4.C**: Klucz deduplikacji QSO.
- **CQ-3.4.D**: Wspólny transport TCP CAT.
- **CQ-3.4.E**: Pętla zapisu LAN Sync.
- **CQ-3.4.F**: Rejestracja getterów Rhai.
- **CQ-3.4.G**: Wspólne komponenty wykresów i statusów GUI.
- **CQ-3.5.A**: Optymalizacja pracy wykonywanej w każdej klatce GUI.
- **CQ-3.5.B**: Optymalizacja awards_matrix.
- **CQ-3.5.C**: Optymalizacja LocalCallbook i SQLite.
- **CQ-3.5.D**: Optymalizacja pga.rs.
- **CQ-3.5.E**: Ograniczenie klonowania w fields_to_qso.
- **CQ-3.5.F**: Zmiana mylących nazw metod.
- **CQ-4.1.A**: Czyszczenie komentarzy ADIF (z powoływaniem się na zachowanie domeny).
- **CQ-4.1.B**: Czyszczenie komentarzy core i cloud.
- **CQ-4.1.C**: Czyszczenie komentarzy CAT i protokoły.
- **CQ-4.1.D**: Czyszczenie komentarzy GUI.
- **CQ-4.1.E**: Separatory i banery ASCII.
- **CQ-4.2.A**: Korekta polskich komunikatów i komentarzy.
- **CQ-4.2.B**: Uzupełnienie nagłówków SPDX.
- **CQ-4.3.A**: Nazewnictwo w core.
- **CQ-4.3.B**: Nazewnictwo w cat.
- **CQ-4.3.C**: Nazewnictwo w gui.
- **CQ-4.3.D**: Nazewnictwo w digital, dsp i pozostałych modułach.
- **CQ-4.4**: CI/CD check.

# 9. Kolejność realizacji i zależności

Ustalona ogólna kolejność wykonania:
1. `CQ-0`: raport P0.
2. Awaryjna blokada updatera Linux, jeśli `CQ-0` potwierdzi osiągalność.
3. Potwierdzone poprawki P0.
4. Zadania P1 gotowe do implementacji.
5. Raport `CQ-3.1.A` o martwym kodzie.
6. Małe zadania P2.
7. Zadania P3.
8. Duże refaktoryzacje dopiero po osobnych `DESIGN.md`.

N1MM może przejść do implementacji dopiero po potwierdzeniu specyfikacji (Raport P0).
Nie rozpoczynać od czyszczenia komentarzy, jeżeli pozostają otwarte potwierdzone błędy P0/P1.

# 10. Kryteria akceptacji

Dla każdego zadania implementacyjnego wymaga się:
- określenia zachowania przed zmianą,
- określenia zachowania po zmianie,
- testu regresyjnego,
- listy zmodyfikowanych plików,
- oceny wpływu na publiczne API,
- oceny wpływu na format danych,
- wyniku `cargo fmt --check`,
- wyniku `cargo check`,
- wyniku `cargo clippy --all-targets --all-features`,
- wyniku `cargo test`,
- testu Windows, jeśli zmiana jest platformowa,
- testu Linux, jeśli zmiana jest platformowa,
- testu ręcznego GUI, jeśli automatyczny test nie wystarcza.

Dla zmian wyłącznie redakcyjnych:
- brak zmiany zachowania,
- brak zmiany publicznego API,
- kompilacja i testy wystarczają.

# 11. Nowe obserwacje wymagające osobnego audytu

1. Obserwacja: potencjalne opóźnienia renderowania w układzie dokowalnym.
Nie potwierdzono usterki. Wymaga profilowania czasu klatki i pomiaru wywołań wykonywanych podczas renderowania.
- **Status techniczny:** NIEPOTWIERDZONY
- **Status wykonawczy:** NIE IMPLEMENTOWAĆ

2. Obserwacja: zachowanie tokio::select! wymaga analizy anulowalności operacji, czasu wykonywania handlerów i przepływu zdarzeń.
Nie potwierdzono nieskończonej pętli ani deadlocka.
- **Status techniczny:** NIEPOTWIERDZONY
- **Status wykonawczy:** NIE IMPLEMENTOWAĆ

## Macierz: problem → weryfikacja → implementacja

* PROB-P0-01 
  → weryfikacja: CQ-0 / REPORT_CODE_QUALITY_P0_VERIFICATION.md
  → implementacja: CQ-1.1
  → zależność: raport potwierdzający osiągalność i wpływ
  → zamknięcie: test błędnej daty SOTA + pełna walidacja

* PROB-N1MM-A
  → weryfikacja: CQ-0
  → implementacja: CQ-1.2.A
  → zależność: oficjalna specyfikacja konkretnego pola N1MM
  → zamknięcie: test wartości pasma zgodnej ze specyfikacją

* PROB-N1MM-B
  → weryfikacja: CQ-0
  → implementacja: CQ-1.2.B
  → zależność: oficjalna specyfikacja N1MM
  → zamknięcie: test jednostek częstotliwości w ContactInfo i RadioInfo

* PROB-P0-03
  → weryfikacja: CQ-0 / REPORT_CODE_QUALITY_P0_VERIFICATION.md
  → implementacja: CQ-1.3
  → zależność: raport potwierdzający osiągalność
  → zamknięcie: test błędu przy tar.gz dla updatera

* PROB-P1-01
  → weryfikacja: CQ-0 / REPORT_CODE_QUALITY_P0_VERIFICATION.md
  → implementacja: CQ-1.4 (lub zależne zadanie implementacyjne)
  → zależność: raport wpływu na produkcyjne UI/eksport
  → zamknięcie: test statystyk na normalnym QSO

* PROB-P1-02
  → weryfikacja: brak dodatkowej (TYLKO ZADANIE ANALITYCZNE)
  → implementacja: CQ-2.1.A, CQ-2.1.B
  → zależność: brak
  → zamknięcie: test heurystyki, test braku callbooka

* PROB-P1-03A
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-2.2.A
  → zależność: brak
  → zamknięcie: test ekstrakcji WPX

* PROB-P1-03B
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-2.2.B
  → zależność: brak
  → zamknięcie: test okręgu SP

* PROB-P1-03C
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-2.2.C
  → zależność: brak
  → zamknięcie: test dyplomów

* PROB-P2-03D
  → weryfikacja: CQ-2.2.D
  → implementacja: opcjonalna po wynikach weryfikacji
  → zależność: weryfikacja użycia i testów
  → zamknięcie: raport analityczny lub usunięcie

* PROB-P1-04
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-2.3.A
  → zależność: brak
  → zamknięcie: test obsługi błędów ToSocketAddrs

* PROB-P1-05
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-2.3.B
  → zależność: brak
  → zamknięcie: test sterowania z Voice Keyera przez kanał

* PROB-P1-06
  → weryfikacja: brak dodatkowej
  → implementacja: Osobne zadanie / DESIGN.md
  → zależność: wspólny potok
  → zamknięcie: redukcja wywołań w GUI do jednej uniwersalnej funkcji

* PROB-P1-07
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-2.4.A
  → zależność: brak
  → zamknięcie: kompilacja po wymuszeniu Send+Sync w boksowanych typach błędów

* PROB-P2-01
  → weryfikacja: CQ-3.1.A / DEAD_CODE_VERIFICATION_REPORT.md
  → implementacja: CQ-3.1.B1, CQ-3.1.B2, CQ-3.1.B3, CQ-3.1.B4
  → zależność: dowód braku użycia z CQ-3.1.A
  → zamknięcie: usunięcie poszczególnych partii martwego kodu w oddzielnych commitach

* PROB-P2-02
  → weryfikacja: CQ-2.4.B (Audyt)
  → implementacja: CQ-2.4.C1, CQ-2.4.C2, CQ-2.4.C3, CQ-2.4.C4
  → zależność: zatwierdzenie proponowanych polityk w CQ-2.4.B
  → zamknięcie: ostateczne poprawki SQLite per plik

* PROB-P2-03
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-3.2 (lub pochodne)
  → zależność: brak
  → zamknięcie: wrappery wyeliminowane z 9 paneli GUI

* PROB-P2-04
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-3.3.A, CQ-3.3.B, CQ-3.3.C, CQ-3.3.D
  → zależność: brak
  → zamknięcie: konsolidacja struktur/funkcji Proxy

* PROB-P2-05
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-3.3.E
  → zależność: brak
  → zamknięcie: unifikacja wzorców zawodów (test po zmianie z zachowaniem wyniku)

* PROB-P2-06
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-3.4.A do CQ-3.4.G
  → zależność: brak
  → zamknięcie: ujednolicenie XML/helperów/TCP

* PROB-P2-07
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-3.5.A do CQ-3.5.E
  → zależność: brak
  → zamknięcie: redukcja zbędnych alokacji egui

* PROB-P2-08
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-3.5.F
  → zależność: brak
  → zamknięcie: spójne nazwy i parametry

* PROB-P2-09
  → weryfikacja: brak dodatkowej
  → implementacja: (w ramach innych commitów)
  → zależność: naprawione wywołania
  → zamknięcie: brak asercji asertujących błędy logiczne

* PROB-P3-01
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-4.1.A do CQ-4.1.E
  → zależność: zachowanie objaśnień (sekcja dlaczego)
  → zamknięcie: brak sztucznych nagłówków i numerów punktów z promptów

* PROB-P3-02
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-4.2.A, CQ-4.2.B
  → zależność: SPDX i prawidłowa pisownia widoczna
  → zamknięcie: weryfikacja wizualna znaków

* PROB-P3-03
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-4.3.A do CQ-4.3.D
  → zależność: brak zmiany API publicznego bez powodu
  → zamknięcie: logiczniejsze nazwy w kodzie lokalnym

* PROB-P3-04
  → weryfikacja: brak dodatkowej
  → implementacja: CQ-4.4
  → zależność: brak
  → zamknięcie: udany przebieg Github Actions z rustfmt i clippy
