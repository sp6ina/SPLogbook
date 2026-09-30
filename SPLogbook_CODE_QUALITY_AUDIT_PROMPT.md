# Prompt: Audyt jakości i normalizacja kodu SPLogbook

Wciel się w rolę doświadczonego programisty Rust, maintenera dużych projektów open source oraz rygorystycznego recenzenta kodu.

Pracujesz nad projektem SPLogbook.

Masz dostęp do całego repozytorium.

## CEL

Przeprowadź kompletny przegląd kodu pod kątem fragmentów, które:

- są niespójne ze stylem pozostałej części projektu,
- wyglądają na wygenerowane automatycznie bez pełnego zrozumienia kontekstu,
- zawierają sztuczne, nadmierne lub nieprzydatne komentarze,
- używają ogólnikowych nazw,
- są przesadnie rozbudowane,
- zawierają niepotrzebne abstrakcje,
- duplikują istniejące rozwiązania,
- są trudne w utrzymaniu,
- mają nierówny poziom jakości,
- nie są zgodne z idiomami Rust,
- nie pasują do przyjętej architektury SPLogbook.

Celem nie jest ukrywanie historii ani pochodzenia kodu.

Celem jest doprowadzenie projektu do spójnego, profesjonalnego i utrzymywalnego standardu jakości, tak aby cały kod wyglądał jak świadomie zaprojektowana, konsekwentnie rozwijana baza kodu Rust.

## TRYB PRACY

Pracuj etapowo.

W pierwszym kroku **NIE ZMIENIAJ KODU**.

Najpierw wykonaj audyt i utwórz plik:

`CODE_QUALITY_NORMALIZATION_PLAN.md`

Po utworzeniu planu zatrzymaj się.

Nie rozpoczynaj automatycznie implementacji.

## ZAKRES ANALIZY

Przeanalizuj wszystkie pliki projektu, w szczególności:

- `src/`,
- `tests/`,
- `benches/`,
- `build.rs`,
- `Cargo.toml`,
- skrypty pomocnicze,
- workflow GitHub Actions,
- dokumentację techniczną powiązaną z kodem.

Nie analizuj plików wygenerowanych automatycznie, katalogu `target` ani zależności zewnętrznych.

## 1. KOMENTARZE

Znajdź komentarze, które:

- powtarzają dokładnie to, co robi kod,
- opisują oczywiste operacje,
- brzmią jak instrukcja lub odpowiedź modelu językowego,
- używają sztucznego albo przesadnie formalnego języka,
- zawierają niepotrzebne nagłówki sekcji,
- są rozwlekłe,
- są nieaktualne,
- nie wyjaśniają przyczyny decyzji,
- zawierają teksty robocze, TODO lub notatki bez wartości.

Nie usuwaj komentarzy wyjaśniających:

- dlaczego zastosowano konkretne rozwiązanie,
- wymagania protokołów,
- nietypowe zachowanie sprzętu,
- ograniczenia ADIF,
- zasady krótkofalarskie,
- bezpieczeństwo,
- kompatybilność,
- przypadki brzegowe.

Zaproponuj pozostawienie komentarzy opisujących „dlaczego”, a nie oczywiste „co”.

## 2. NAZEWNICTWO

Znajdź:

- ogólne nazwy typu `data`, `result`, `response`, `temp`, `item`, `handler`, `manager`,
- nazwy niezgodne z terminologią projektu,
- niespójne skróty,
- nazwy funkcji, które nie opisują skutków ubocznych,
- polskie i angielskie nazwy mieszane w kodzie,
- nazwy sugerujące inne zachowanie niż rzeczywiste,
- nazwy nadmiernie długie lub sztucznie szczegółowe.

Nie zmieniaj nazw publicznego API bez wyraźnej potrzeby.

Każdą proponowaną zmianę nazwy uzasadnij.

## 3. NADMIERNE ABSTRAKCJE

Znajdź:

- funkcje używane tylko raz bez poprawy czytelności,
- wrappery bez rzeczywistej wartości,
- struktury z jednym polem bez uzasadnienia,
- traity mające tylko jedną implementację bez perspektywy rozwoju,
- nadmiarowe buildery,
- zbyt wiele warstw przekazywania danych,
- abstrakcje wprowadzone „na przyszłość”, ale obecnie niewykorzystywane,
- zbyt skomplikowane rozwiązania prostych problemów.

Nie upraszczaj elementów, jeżeli abstrakcja:

- umożliwia testowanie,
- oddziela logikę od GUI,
- chroni granice modułów,
- jest potrzebna dla wielu platform,
- obsługuje wiele integracji,
- jest uzasadniona bezpieczeństwem.

## 4. POWTARZALNE WZORCE

Znajdź fragmenty o charakterystycznej, mechanicznej konstrukcji:

- identyczne bloki `match`,
- podobne konwersje błędów,
- powtarzane walidacje,
- duplikowane parsowanie,
- podobne funkcje różniące się jednym parametrem,
- kopiowane zapytania SQL,
- kopiowane struktury obsługi GUI,
- ręcznie powtarzane mapowania.

Nie zastępuj duplikacji makrem ani generyczną abstrakcją, jeżeli wynik będzie trudniejszy do zrozumienia niż obecny kod.

## 5. OBSŁUGA BŁĘDÓW

Sprawdź:

- `Result<T, String>`,
- nieprecyzyjne komunikaty błędów,
- ignorowanie błędów przez `.ok()`,
- `unwrap()` i `expect()` w ścieżkach produkcyjnych,
- błędy cicho zastępowane wartościami domyślnymi,
- błędy logowane, ale nieprzekazywane użytkownikowi,
- przypadki powodujące utratę danych,
- nadmierne `map_err` bez dodania kontekstu.

Nie zamieniaj automatycznie wszystkich błędów na jeden centralny typ.

Wskaż, gdzie zmiana rzeczywiście poprawi diagnostykę i utrzymywalność.

## 6. IDIOMY RUST

Sprawdź zgodność z:

- Rust Edition 2024,
- aktualnym MSRV projektu,
- idiomatycznym użyciem `Option` i `Result`,
- borrowing zamiast niepotrzebnego `clone()`,
- iteratorami tam, gdzie poprawiają czytelność,
- prostymi pętlami tam, gdzie iterator byłby mniej czytelny,
- `From` i `TryFrom`,
- newtype tylko tam, gdzie daje bezpieczeństwo typów,
- RAII,
- bezpieczną współbieżnością,
- właściwym użyciem `Arc`, `Mutex` i `RwLock`.

Nie wykonuj mechanicznego „unowocześniania” kodu.

Każda zmiana musi poprawiać co najmniej jeden z aspektów:

- poprawność,
- czytelność,
- testowalność,
- bezpieczeństwo,
- wydajność,
- utrzymywalność.

## 7. SPÓJNOŚĆ ARCHITEKTURY

Znajdź fragmenty, które:

- obchodzą istniejące warstwy projektu,
- umieszczają logikę biznesową w GUI,
- wykonują operacje sieciowe bezpośrednio w kodzie widoku,
- duplikują funkcjonalność istniejącego modułu,
- wprowadzają drugi sposób realizacji tego samego zadania,
- mają niezgodny model błędów,
- mają niezgodne nazewnictwo,
- nie respektują istniejących granic modułów.

Nie przebudowuj całej architektury.

Duże problemy architektoniczne zapisz jako osobne propozycje wymagające `DESIGN.md`.

## 8. TESTY

Sprawdź, czy testy:

- rzeczywiście sprawdzają zachowanie,
- nie powtarzają implementacji produkcyjnej,
- nie są nadmiernie rozbudowane,
- nie testują oczywistych szczegółów implementacyjnych,
- mają czytelne nazwy,
- obejmują przypadki brzegowe,
- nie używają sztucznych, nierealistycznych danych,
- nie maskują błędów przez `unwrap` bez uzasadnienia.

Nie usuwaj testów tylko dlatego, że wydają się obszerne.

## 9. KOD DOMENOWY KRÓTKOFALARSKI

Zachowaj szczególną ostrożność przy kodzie dotyczącym:

- ADIF,
- MODE i SUBMODE,
- QSO,
- pasm,
- emisji,
- DXCC,
- stref CQ i ITU,
- Maidenhead Locator,
- propagacji,
- CAT,
- Hamlib,
- rigctld,
- rotctld,
- WSJT-X,
- LoTW,
- eQSL,
- Club Log,
- QRZ,
- POTA,
- SOTA,
- IOTA,
- zawodów i Cabrillo.

Nie upraszczaj kodu domenowego tylko dlatego, że wygląda nietypowo.

Najpierw ustal, czy nietypowy fragment wynika z wymagań standardu, protokołu lub kompatybilności.

## PLAN WYNIKOWY

Utwórz:

`CODE_QUALITY_NORMALIZATION_PLAN.md`

Plan ma zawierać:

### 1. Podsumowanie

- ogólna ocena spójności kodu,
- najważniejsze problemy,
- obszary o najwyższym ryzyku.

### 2. Potwierdzone problemy

Dla każdego problemu:

- identyfikator,
- priorytet,
- plik i zakres linii,
- opis,
- dlaczego wygląda niespójnie lub mechanicznie,
- wpływ na projekt,
- rekomendowana poprawka,
- ryzyko zmiany,
- wymagane testy.

### 3. Elementy podejrzane, ale uzasadnione

Wymień fragmenty, które początkowo wyglądają nietypowo, ale są uzasadnione przez:

- domenę krótkofalarską,
- kompatybilność,
- bezpieczeństwo,
- wydajność,
- ograniczenia biblioteki,
- wymagania platformowe.

### 4. Fałszywe alarmy

Wymień elementy, których nie należy zmieniać wraz z uzasadnieniem.

### 5. Plan etapowy

Podziel poprawki na małe zadania:

- jedno zadanie na sesję,
- jedno zadanie na commit,
- minimalny zakres plików,
- jasno określone kryteria ukończenia.

### 6. Duże refaktoryzacje

Dla większych zmian wymagaj osobnego `DESIGN.md`.

## PRIORYTETY

Nadaj problemom priorytety:

### P0 Krytyczny

- utrata danych,
- błąd bezpieczeństwa,
- złamanie standardu,
- crash w typowym użyciu.

### P1 Wysoki

- duże ryzyko regresji,
- poważna niespójność,
- duplikacja powodująca błędy,
- trudna diagnostyka.

### P2 Średni

- czytelność,
- utrzymywalność,
- lokalne niespójności,
- nadmiarowe abstrakcje.

### P3 Niski

- styl,
- nazewnictwo,
- komentarze,
- drobne uproszczenia.

Nie oznaczaj problemów stylistycznych jako krytyczne.

## ZASADY IMPLEMENTACJI

Na tym etapie nie implementuj żadnych poprawek.

Po późniejszym poleceniu:

`Wykonaj zadanie X.Y`

wykonaj wyłącznie wskazane zadanie.

Nie przechodź automatycznie dalej.

Nie wykonuj poprawek „przy okazji”.

Po każdym zadaniu:

- uruchom `cargo fmt --check`,
- uruchom `cargo check`,
- uruchom `cargo clippy --all-targets --all-features`,
- uruchom `cargo test`,
- przedstaw zmodyfikowane pliki,
- przedstaw ryzyka,
- zaktualizuj wyłącznie status wskazanego zadania.

## OGRANICZENIA

Nie zmieniaj kodu w pierwszym kroku.

Nie zmieniaj historii Git.

Nie usuwaj informacji o współautorach.

Nie usuwaj informacji licencyjnych.

Nie usuwaj prawdziwych informacji o wykorzystanych narzędziach.

Nie fałszuj autorstwa ani pochodzenia kodu.

Nie dodawaj sztucznych komentarzy mających sugerować określone autorstwo.

Nie zastępuj działającego kodu innym tylko po to, aby wyglądał inaczej.

Nie wykonuj masowego `search/replace`.

Nie zmieniaj publicznego API bez konkretnego uzasadnienia.

Nie zmieniaj zachowania programu w ramach poprawek stylistycznych.

## ZAKOŃCZENIE

Po utworzeniu `CODE_QUALITY_NORMALIZATION_PLAN.md`:

1. Nie zmieniaj kodu.
2. Nie aktualizuj innych dokumentów.
3. Podaj krótkie podsumowanie ustaleń.
4. Wymień maksymalnie 10 najważniejszych zadań.
5. Zatrzymaj się i czekaj na polecenie wykonania konkretnego zadania.
