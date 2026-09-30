# Prompt: Finalne uporządkowanie planu normalizacji jakości SPLogbook

Wciel się w rolę doświadczonego programisty Rust, maintenera dużych projektów open source oraz rygorystycznego recenzenta planów modernizacji oprogramowania.

Pracujesz nad projektem SPLogbook.

Przeczytaj dokładnie:

- `CODE_QUALITY_NORMALIZATION_PLAN.md`
- `PLAN.md`
- `DESIGN_2_2_FINAL.md`
- `DESIGN_3_1_UPDATER_SIGNATURES.md`

W razie konieczności sprawdź wskazane fragmenty aktualnego kodu, ale nie wykonuj ponownie kompletnego audytu całego repozytorium.

## ZADANIE

Popraw istniejący dokument:

`CODE_QUALITY_NORMALIZATION_PLAN.md`

Nie twórz nowego dokumentu.

Nie zmieniaj kodu programu.

Nie wykonuj żadnego zadania z planu.

Nie zmieniaj `PLAN.md`.

Nie zmieniaj dokumentów `DESIGN`.

Nie zmieniaj `Cargo.toml` ani `Cargo.lock`.

Nie zmieniaj workflow GitHub Actions.

Nie wykonuj commitów.

Celem jest usunięcie ostatnich niespójności organizacyjnych i przekształcenie dokumentu w jednoznaczny plan wykonawczy.

## 1. POPRAW TYTUŁ CZĘŚCI A

Zmień nagłówek:

„Część A: Potwierdzone błędy funkcjonalne i bezpieczeństwa”

na:

„Część A: Błędy funkcjonalne i bezpieczeństwa”

Uzasadnienie:

Część A zawiera również problemy wymagające:

- specyfikacji zewnętrznej,
- weryfikacji uruchomieniowej,
- raportu ETAPU 0,
- ponownego ustalenia priorytetu.

O tym, czy problem jest potwierdzony, ma decydować pole statusu, a nie tytuł sekcji.

## 2. ROZDZIEL STATUS TECHNICZNY OD GOTOWOŚCI WYKONAWCZEJ

Dla każdego problemu zastosuj dwa osobne pola:

- Status techniczny
- Status wykonawczy

Dozwolone wartości statusu technicznego:

- POTWIERDZONY W KODZIE
- CZĘŚCIOWO POTWIERDZONY
- WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ
- WYMAGA TESTU URUCHOMIENIOWEGO
- NIEPOTWIERDZONY
- FAŁSZYWY ALARM

Dozwolone wartości statusu wykonawczego:

- GOTOWY DO IMPLEMENTACJI
- ZABLOKOWANY DO CZASU RAPORTU P0
- ZABLOKOWANY DO CZASU WERYFIKACJI
- ZABLOKOWANY DO CZASU DESIGN.md
- TYLKO ZADANIE ANALITYCZNE
- NIE IMPLEMENTOWAĆ

Przykład:

```text
Status techniczny: POTWIERDZONY W KODZIE
Status wykonawczy: ZABLOKOWANY DO CZASU RAPORTU P0
```

Zastosuj ten model szczególnie dla:

- fallbacku `Utc::now()` w SOTA,
- aktualizatora Linux,
- liczników zawodów,
- problemów N1MM,
- podejrzanego martwego kodu GUI.

Problem nie może zostać wykonany, jeżeli status wykonawczy nie brzmi:

`GOTOWY DO IMPLEMENTACJI`

## 3. POPRAW PRIORYTETY N1MM

Problemy:

- `PROB-N1MM-A`
- `PROB-N1MM-B`

mają status:

`WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ`

Nie mogą więc posiadać ostatecznego priorytetu P0.

Zmień ich klasyfikację na:

```text
Priorytet tymczasowy: P1 Wysoki
Potencjalny priorytet: P0 po potwierdzeniu specyfikacji i wpływu
```

Warunki podniesienia do P0:

- oficjalna specyfikacja N1MM potwierdza niezgodność,
- niezgodność występuje w osiągalnej ścieżce produkcyjnej,
- wysyłane wartości powodują rzeczywiste przekłamanie danych lub złamanie protokołu,
- test regresyjny odtwarza problem.

ETAP 0 ma nadać ostateczny priorytet.

Nie zmieniaj kodu N1MM w ramach korekty dokumentu.

## 4. ROZDZIEL PROB-P1-03

Usuń zbiorczy problem:

`PROB-P1-03 — Duplikacja extract_wpx_prefix i ekstrakcji extract_sp_district`

Zastąp go czterema niezależnymi problemami:

### PROB-P1-03A — Duplikacja logiki prefiksu WPX

Zakres:

- `extract_wpx_prefix`,
- `extract_wpx_base`,
- sufiksy operacyjne,
- rozbieżne przypadki brzegowe.

### PROB-P1-03B — Duplikacja ekstrakcji polskiego okręgu SP

Zakres:

- wolna funkcja `extract_sp_district`,
- metoda `AwardsEngine::get_polish_district`,
- ustalenie jednej kanonicznej implementacji.

### PROB-P1-03C — Podwójna rejestracja danych dyplomowych

Zakres:

- `register_qso_record`,
- `register_qso_full`,
- wielokrotne wstawianie tych samych wartości.

### PROB-P2-03D — Podejrzane nieużywane pola dyplomowe

Zakres:

- identyfikacja pól,
- sprawdzenie rzeczywistego użycia,
- sprawdzenie serializacji,
- sprawdzenie testów,
- decyzja o pozostawieniu lub usunięciu.

Dla `PROB-P2-03D` ustaw:

```text
Status techniczny: WYMAGA TESTU UŻYCIA
Status wykonawczy: ZABLOKOWANY DO CZASU WERYFIKACJI
Priorytet: P2 Średni
```

Powiąż te problemy odpowiednio z zadaniami:

- `CQ-2.2.A`
- `CQ-2.2.B`
- `CQ-2.2.C`
- `CQ-2.2.D`

## 5. OZNACZ POLITYKI rows.flatten() JAKO PROPOZYCJE

Nie przedstawiaj obecnych decyzji:

- fail-fast,
- partial result + warning

jako ostatecznie zatwierdzonych.

Dla każdego wystąpienia `rows.flatten()` dodaj:

- proponowana polityka,
- status decyzji,
- miejsce raportowania błędu,
- wpływ pominięcia rekordu,
- możliwość propagacji błędu,
- wymagany test.

Użyj sformułowania:

„Proponowana polityka, wymagająca zatwierdzenia w CQ-2.4.B”.

Zadanie `CQ-2.4.B` ma ustalić ostateczną politykę osobno dla:

- `database.rs`,
- `database_stats.rs`,
- `prefix.rs`,
- `service_db.rs`.

Nie implementuj zmian.

## 6. NADAJ WSZYSTKIM ZADANIOM PREFIKS CQ

Aby uniknąć kolizji z Etapem 3.1 dotyczącym podpisów aktualizatora, nadaj wszystkim zadaniom tego dokumentu prefiks:

`CQ-`

Przykłady:

- `CQ-0`
- `CQ-1.1`
- `CQ-1.2.A`
- `CQ-2.2.A`
- `CQ-3.1.A`
- `CQ-4.1.A`

Zmień również nazwy wynikowych raportów tak, aby jasno wskazywały audyt jakości:

- `REPORT_CODE_QUALITY_P0_VERIFICATION.md`
- `DEAD_CODE_VERIFICATION_REPORT.md`

Nie zmieniaj numeracji zadań w `PLAN.md`.

Prefiks `CQ` obowiązuje wyłącznie wewnątrz:

`CODE_QUALITY_NORMALIZATION_PLAN.md`

## 7. ROZBIJ CQ-2.1

Obecne zadanie 2.1 łączy:

- błędną heurystykę `ends_with("CW")`,
- zahardkodowane dane demonstracyjne w callbooku.

Podziel je na:

### CQ-2.1.A — Naprawa heurystyki członkostwa klubowego

Plik:

- `src/core/clubs.rs`

Zakres:

- usunięcie błędnego `ends_with("CW")`,
- zachowanie prawidłowych danych klubowych,
- testy pozytywne i negatywne.

### CQ-2.1.B — Usunięcie danych demonstracyjnych z produkcyjnego callbooka

Plik:

- `src/core/callbook.rs`

Zakres:

- usunięcie testowych danych osobowych z produkcyjnej ścieżki,
- przeniesienie danych testowych do `#[cfg(test)]`, jeśli są potrzebne,
- sprawdzenie zachowania bez skonfigurowanego dostawcy callbooka.

Każde zadanie ma otrzymać osobne kryteria ukończenia i osobny commit.

## 8. ROZBIJ IMPLEMENTACJĘ BŁĘDÓW SQLITE

Zastąp ogólne zadanie:

`CQ-2.4.C — Implementacja poprawionej propagacji błędów SQLite małymi grupami`

następującymi zadaniami:

- `CQ-2.4.C1 — Obsługa błędów w database.rs`
- `CQ-2.4.C2 — Obsługa błędów w database_stats.rs`
- `CQ-2.4.C3 — Obsługa błędów w prefix.rs`
- `CQ-2.4.C4 — Obsługa błędów w service_db.rs`

Każde zadanie może zostać wykonane dopiero po zakończeniu `CQ-2.4.B`.

Dla każdego zadania określ:

- ostateczną politykę,
- sposób propagacji lub raportowania,
- zmianę sygnatur funkcji,
- wpływ na wywołujących,
- test błędnego rekordu,
- test poprawnych rekordów,
- ryzyko częściowego wyniku.

## 9. ROZBIJ CQ-4.2

Podziel zadanie dotyczące diakrytyki i SPDX na:

### CQ-4.2.A — Korekta polskich komunikatów i komentarzy

Zakres:

- wyłącznie tekst widoczny dla użytkownika,
- komentarze,
- logi,
- napisy interfejsu.

Nie zmieniaj kluczy lokalizacyjnych ani formatów protokołów.

### CQ-4.2.B — Uzupełnienie nagłówków SPDX

Zakres:

- dodanie poprawnych nagłówków licencyjnych,
- bez zmian w logice,
- sprawdzenie zgodności z licencją repozytorium.

Każde zadanie jako osobny commit.

## 10. ROZBIJ CQ-4.3

Podziel normalizację nazw na grupy:

- `CQ-4.3.A — Nazewnictwo w core`
- `CQ-4.3.B — Nazewnictwo w cat`
- `CQ-4.3.C — Nazewnictwo w gui`
- `CQ-4.3.D — Nazewnictwo w digital, dsp i pozostałych modułach`

Zasady:

- zmieniaj wyłącznie nazwy lokalne i prywatne,
- zmiana publicznego API wymaga osobnego uzasadnienia,
- nie łącz zmiany nazw ze zmianą zachowania,
- nie wykonuj masowego search/replace,
- każdy commit ma dotyczyć jednej grupy modułów.

## 11. DOPRECYZUJ OPIS ADIF

Zastąp niejasny opis:

„FT4/Q65 zgłaszają niestandardowe ułożenia wg starszych API”

następującym jednoznacznym opisem:

```text
Wewnętrzna reprezentacja FT4 i Q65 jest tłumaczona podczas eksportu
na standardową reprezentację ADIF 3.1.7:

MODE=MFSK + SUBMODE=FT4
MODE=MFSK + SUBMODE=Q65

Importer toleruje również spotykane w praktyce bezpośrednie wartości
MODE=FT4 i MODE=Q65, aby nie tracić danych z zewnętrznych logów.

FT8, JT65, JT9, WSPR i MSK144 pozostają głównymi wartościami MODE.
```

Nie odwołuj się do ADIF 3.1.8 jako wydanego standardu.

## 12. ZŁAGODŹ SPEKULACYJNE NOWE OBSERWACJE

W sekcji:

„Nowe obserwacje wymagające osobnego audytu”

usuń sformułowania sugerujące potwierdzone błędy.

Dla GUI zapisz:

```text
Obserwacja: potencjalne opóźnienia renderowania w układzie dokowalnym.
Nie potwierdzono usterki. Wymaga profilowania czasu klatki i pomiaru
wywołań wykonywanych podczas renderowania.
```

Dla CAT zapisz:

```text
Obserwacja: zachowanie tokio::select! wymaga analizy anulowalności
operacji, czasu wykonywania handlerów i przepływu zdarzeń.
Nie potwierdzono nieskończonej pętli ani deadlocka.
```

Dla każdej obserwacji ustaw:

```text
Status techniczny: NIEPOTWIERDZONY
Status wykonawczy: NIE IMPLEMENTOWAĆ
```

Nie przypisuj priorytetu przed osobnym audytem.

## 13. DODAJ MACIERZ ŚLEDZENIA

Dodaj sekcję:

## Macierz: problem → weryfikacja → implementacja

Macierz nie musi być tabelą Markdown. Może być listą.

Dla każdego problemu zapisz:

- identyfikator problemu,
- bieżący status techniczny,
- zadanie weryfikacyjne,
- zadanie implementacyjne,
- zależności,
- kryterium zamknięcia.

Przykład:

```text
PROB-P0-01
→ weryfikacja: CQ-0 / REPORT_CODE_QUALITY_P0_VERIFICATION.md
→ implementacja: CQ-1.1
→ zależność: raport potwierdzający osiągalność i wpływ
→ zamknięcie: test błędnej daty SOTA + pełna walidacja

PROB-N1MM-A
→ weryfikacja: CQ-0-N1MM-A
→ implementacja: CQ-1.2.A
→ zależność: oficjalna specyfikacja konkretnego pola N1MM
→ zamknięcie: test wartości pasma zgodnej ze specyfikacją

PROB-P2-01
→ weryfikacja: CQ-3.1.A / DEAD_CODE_VERIFICATION_REPORT.md
→ implementacja: CQ-3.1.B1, CQ-3.1.B2 itd.
→ zależność: dowód braku użycia
→ zamknięcie: osobne usunięcie każdej grupy symboli
```

Macierz ma obejmować wszystkie problemy P0-P3.

Jeżeli problem nie ma obecnie przypisanego zadania, dodaj zadanie analityczne albo implementacyjne.

## 14. DOPRECYZUJ CQ-3.1.B

Nie pozostawiaj ogólnego zadania:

„Usuwanie martwego kodu małymi grupami modułów”.

Zaprojektuj podział, który zostanie ostatecznie ustalony przez raport `CQ-3.1.A`.

Przykładowy przyszły podział:

- `CQ-3.1.B1 — nieosiągalne gałęzie wewnątrz funkcji`,
- `CQ-3.1.B2 — prywatne funkcje GUI bez wywołań`,
- `CQ-3.1.B3 — stary układ kolumnowy`,
- `CQ-3.1.B4 — podejrzane publiczne symbole po dodatkowej analizie API`.

Zaznacz:

`CQ-3.1.B4` nie może zostać wykonane tylko dlatego, że nie znaleziono wewnętrznego wywołania publicznego symbolu.

## 15. DOPRECYZUJ KRYTERIA AKCEPTACJI

Dla każdego zadania implementacyjnego wymagaj:

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

## 16. POPRAW KOLEJNOŚĆ REALIZACJI

Ustal następującą ogólną kolejność:

1. `CQ-0`: raport P0.
2. Awaryjna blokada updatera Linux, jeśli `CQ-0` potwierdzi osiągalność.
3. Potwierdzone poprawki P0.
4. Zadania P1 gotowe do implementacji.
5. Raport `CQ-3.1.A` o martwym kodzie.
6. Małe zadania P2.
7. Zadania P3.
8. Duże refaktoryzacje dopiero po osobnych `DESIGN.md`.

N1MM może przejść do implementacji dopiero po potwierdzeniu specyfikacji.

Nie rozpoczynaj od czyszczenia komentarzy, jeżeli pozostają otwarte potwierdzone błędy P0/P1.

## OGRANICZENIA

Nie zmieniaj kodu.

Nie wykonuj raportu `CQ-0`.

Nie twórz `REPORT_CODE_QUALITY_P0_VERIFICATION.md`.

Nie twórz `DEAD_CODE_VERIFICATION_REPORT.md`.

Nie zmieniaj `PLAN.md`.

Nie zmieniaj dokumentów `DESIGN`.

Nie zmieniaj `Cargo.toml` ani `Cargo.lock`.

Nie zmieniaj workflow GitHub Actions.

Nie wykonuj commitów.

Nie dodawaj nowych problemów poza koniecznymi pozycjami organizacyjnymi.

Zaktualizuj wyłącznie:

`CODE_QUALITY_NORMALIZATION_PLAN.md`

## KONTROLA KOŃCOWA

Przed zakończeniem sprawdź:

1. Czy Część A nie sugeruje, że wszystkie problemy są potwierdzone.
2. Czy każdy problem ma status techniczny i wykonawczy.
3. Czy N1MM ma priorytet tymczasowy, a nie końcowy P0.
4. Czy `PROB-P1-03` został rozdzielony.
5. Czy `rows.flatten()` zawiera propozycje polityk, a nie decyzje ostateczne.
6. Czy wszystkie zadania mają prefiks `CQ`.
7. Czy `CQ-2.1` jest rozdzielone na kluby i callbook.
8. Czy implementacja SQLite jest podzielona per moduł.
9. Czy diakrytyka i SPDX są osobnymi zadaniami.
10. Czy zmiany nazw są podzielone na grupy modułów.
11. Czy opis ADIF 3.1.7 jest jednoznaczny.
12. Czy nowe obserwacje są wyraźnie oznaczone jako niepotwierdzone.
13. Czy istnieje kompletna macierz problem → weryfikacja → implementacja.
14. Czy numeracja nie koliduje z Etapem 3.1 updatera.
15. Czy każdy etap nadaje się do osobnej sesji i osobnego commita.

Po zapisaniu dokumentu:

1. Podsumuj poprawki organizacyjne.
2. Wymień zmienione priorytety.
3. Wymień rozdzielone problemy i zadania.
4. Potwierdź dodanie prefiksu `CQ`.
5. Potwierdź dodanie macierzy śledzenia.
6. Nie implementuj kodu.
7. Zatrzymaj się.
