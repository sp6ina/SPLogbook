# Raport Weryfikacji Problemów P0 (VERIFY-P0)

## 1. PROB-P0-01: Ciche podstawianie bieżącej daty w SOTA CSV

- **Status:** POTWIERDZONE W KODZIE
- **Priorytet końcowy:** P0 Krytyczny
- **Plik i zakres:** `src/core/sota_export.rs:34-55` (`format_sota_date`, `format_sota_time`)
- **Stan obecny:** Kod analizuje długość przekazanego tekstu z datą/czasem (oczekując 8 znaków dla daty i 4 dla czasu). Jeśli wartość jest niekompletna (np. błędna data w bazie, brak wartości), program cicho używa `chrono::Utc::now()`.
- **Wpływ:** Wysłanie nieprawdziwych danych łączności (czasu i daty wygenerowania pliku) bez wiedzy użytkownika, co stanowi sfałszowanie logu zgłaszanego do programu dyplomowego.
- **Rekomendacja:** Zmienić funkcje pomocnicze tak, aby zwracały `Result<String, String>`. Błędy propagować w górę do funkcji eksportującej, przerywając eksportowanie i zwracając użytkownikowi jasny komunikat o wadliwym wpisie.

## 2. PROB-N1MM-A: Niepoprawne usuwanie sufiksu pasma

- **Status:** POTWIERDZONE W KODZIE / ZAKOŃCZONY (DONE)
- **Priorytet końcowy:** P1 Wysoki (nie osiąga poziomu P0, ponieważ uszkodzona jest tylko integracja N1MM z określonymi narzędziami innych firm dla pasm UKF, nie dotyka integralności głównego dziennika w SQLite).
- **Plik i zakres:** `src/digital/n1mm.rs:186-189` (`band_to_meters`)
- **Stan obecny:** Funkcja konwertuje ciągi poprzez proste wywołanie `band.trim_end_matches('m')`. Prowadzi to do rezultatu `"70c"` dla wejściowego `"70cm"` zamiast wymaganej wartości dla pasma UKF.
- **Wpływ:** Odbiorcy UDP nie rozpoznają poprawnie pasma "70c", co może skutkować błędem parsowania na urządzeniu integrującym (np. GridTracker).
- **Rekomendacja:** Przygotować test jednostkowy walidujący mapowanie, sprawdzić oficjalną dokumentację formatu XML dla N1MM, po czym napisać sztywne mapowanie wartości.

## 3. PROB-N1MM-B: Niespójne jednostki częstotliwości w N1MM XML

- **Status:** POTWIERDZONE W KODZIE / ZAKOŃCZONY (DONE)
- **Priorytet końcowy:** P1 Wysoki (zniekształcenie pakietów sieciowych, brak utraty danych w systemie lokalnym).
- **Plik i zakres:** `src/digital/n1mm.rs:83, 98-99, 159`
- **Stan obecny:** W ramce `<contactinfo>` pole `txfreq`/`rxfreq` wypełniane jest wartością przemnożoną przez 1000 i zaokrągloną (traktowaną jako kHz), podczas gdy w ramce `<radioinfo>` pole `Freq`/`TXFreq` operuje na innej wielkości (Hz, wejściowy parametr `freq_hz`).
- **Wpływ:** Skutkuje to wyświetlaniem w obcych aplikacjach niepoprawnych częstotliwości lub błędów przeskalowania. 
- **Rekomendacja:** Znalezienie poprawnego mnożnika wg oficjalnej dokumentacji N1MM Broadcast, zmiana kodu w obu funkcjach generujących ramki, dodanie testu z potwierdzonymi wartościami.

## 4. PROB-P0-03 (oraz przypadek Windows): Awaryjna poprawka updatera

- **Status:** POTWIERDZONE W KODZIE
- **Priorytet końcowy:** P0 Krytyczny
- **Plik i zakres:** `src/cloud/updater.rs:361-419`
- **Stan obecny:** Proces aplikacji po pobraniu uaktualnienia (zawsze w postaci archiwum) stosuje standardowy mechanizm self-update, przenosząc plik archiwum `.zip` lub `.tar.gz` wprost pod starą ścieżkę pliku binarnego/wykonywalnego.
- **Wpływ:** Po uruchomieniu aktualizacji użytkownik trwale niszczy binarkę programu. Przy próbie następnego włączenia, system (Windows lub Linux) nie rozpozna pliku jako wykonywalnego.
- **Rekomendacja:** Natychmiastowe zaimplementowanie zabezpieczenia typu Safety Gate blokującego te typy archiwów przed podmienianiem głównego programu. Wdrożenie awaryjnego komunikatu.

## 5. PROB-P1-01: Bezwarunkowa inkrementacja liczników zawodów w SpLogApp

- **Status:** POTWIERDZONE W KODZIE
- **Priorytet końcowy:** P1 Wysoki (wpływa jedynie na stan UI/pamięci krótkotrwałej, lecz nie niszczy bazy, a logikę zawodów da się przeliczyć od nowa na podstawie SQL).
- **Plik i zakres:** `src/gui/app.rs:2081-2084`
- **Stan obecny:** Liczniki `contest_qsos`, `contest_points`, `contest_mults`, `contest_stx` są inkrementowane bezwzględnie przy zapisie każdego pojedynczego QSO, bez uwzględniania czy okno zawodów jest otwarte/aktywne.
- **Wpływ:** Mieszanie statystyk zwykłych łączności DX-owych z trybem Contest (zawyżanie wyniku bieżącego u operatora). 
- **Rekomendacja:** Owarunkować blok kodu podniesieniem statusu aktywnych zawodów (`if self.show_contest_window`).
