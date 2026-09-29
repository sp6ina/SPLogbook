# 📋 PLAN — SPLogbook: plan naprawczy po audycie

**Podstawa:** `AUDIT_REPORT.md` (v1.0, 2026-09-29)
**Weryfikacja:** przeprowadzona względem aktualnego kodu (2026-09-29)
**Tryb pracy:** etapowy — jedno zadanie na sesję, wyłącznie na polecenie `Wykonaj etap X.Y`

---

## 0. WYNIK WERYFIKACJI RAPORTU

### 0.1 Odrzucone (fałszywe alarmy)

| Lp. | Ustalenie z raportu | Werdykt | Uzasadnienie |
|---|---|---|---|
| 1 | 🔴 „Hardcoded klucz P2P `"tajne-haslo"`" | **FAŁSZYWY ALARM** | Literał występuje **wyłącznie w testach** (`#[cfg(test)]` w `src/sync/p2p.rs:279-280`). Produkcyjna synchronizacja (`src/cluster/lan_sync.rs`) używa sekretu z konfiguracji użytkownika (`lan_sync_secret`), a serwer odmawia startu przy pustym haśle. |
| 2 | 🟡 „Brak eksportu Cabrillo" | **FAŁSZYWY ALARM** | Eksport **już istnieje** — `src/gui/app_export.rs::export_cabrillo()` („Cabrillo 3.0"), wywoływany z `src/gui/contest.rs:611`. |
| 3 | „Regex w `telnet.rs` powinny używać `LazyLock`" | **NIEAKTUALNE** | `src/cluster/telnet.rs` **już używa `OnceLock<Regex>`** (`spot_regex()`, `spot_regex_no_z()`, `sh_dx_regex()`). Pozostałe `.expect()` jest uzasadnione (regex statyczny). |
| 4 | 🔴 `sounds.rs` `thread::spawn().expect()` = „Wysokie ryzyko" | **ZANIŻONE SEVERITY** | Panic możliwy tylko przy wyczerpaniu zasobów OS (tworzenie wątku). Realnie 🟢 **Niskie**. |

### 0.2 Korekty metryk raportu

| Metryka z raportu | Wartość rzeczywista | Uwagi |
|---|---|---|
| `app.rs` = 5808 linii | **5355 linii** | God Object potwierdzony, ale już częściowo rozbity: istnieją `app_layout.rs`, `app_export.rs` itd. |
| `row_to_qso()` `.ok()` w 3 kolumnach | **4 kolumny** | Dodatkowo `journal_id` (indeks 53) używa `.ok()`. |
| SCP „bezużyteczny" | **Częściowo złagodzone** | Po zainicjowaniu 12 znakami kod ładuje do 50 000 unikalnych znaków z lokalnego dziennika (`main.rs`). Brak tylko importu `MASTER.DTA`. |

### 0.3 Potwierdzone (utrzymane w planie)

Wszystkie pozostałe ustalenia raportu potwierdzono w kodzie. Najważniejsze:

- `row_to_qso()`: `.ok()` zamiast `?` (ciche gubienie błędów odczytu) — `database.rs:113-117`
- Brak walidacji zakresu HH:MM / MM:SS — `qso.rs:216-234`
- `ADIF_VERSION = "3.1.7"` — `adif.rs:9`
- FT8/WSPR/JT65/JT9/MSK144 jako emisje główne (nie submode MFSK) — `adif.rs:227`
- Brak indeksów `country` i `freq` — `database.rs` (migracje)
- Auto-updater weryfikuje tylko SHA-256 — `updater.rs:234-251,347`
- `add_column_if_missing` z interpolacją `format!` — `database.rs:521`
- 4× duplikacja listy kolumn (INSERT/UPDATE) — `database.rs:687,776,869,1062`
- Powtórzony schemat `qso_records` w `migration_3` — `database.rs:368`
- `vacuum_if_needed` na `count % 1000` — `database.rs:1435`
- Cache prefiksów `Mutex<HashMap>` — `prefix.rs:39,340`
- `std::sync::Mutex<LogDatabase>` — `main.rs:211`
- Brak centralnego typu błędu (`AppError`), brak `tracing`, brak AccessKit, brak FTS5

---

## ETAP 1 — Poprawność danych (Quick Wins, bez zmiany API)

### 1.1 `row_to_qso` — `.ok()` → `?`

**Status:** DONE ✅ (2026-09-29)
**Priorytet:** Krytyczny
**Złożoność:** Niska
**Opis:** W `row_to_qso()` kolumny `my_pota_ref` (49), `my_sota_ref` (50), `vucc_grids` (51) oraz `journal_id` (53) używają `row.get(N).ok()`, co cicho ignoruje **każdy** błąd odczytu (nie tylko NULL). Zamienić na `row.get(N)?` spójnie z pozostałymi kolumnami (wartości opcjonalne i tak zwracają `Option`).
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Niskie — zmiana czysto lokalna; może ujawnić błędy konwersji przy niespójnym schemacie (to jest pożądane).
**Zależności:** brak

**Podsumowanie zmian:** Zamieniono `row.get(N).ok()` → `row.get(N)?` dla `my_pota_ref` (49), `my_sota_ref` (50), `vucc_grids` (51), `journal_id` (53). Walidacja: `cargo check` ✅, `cargo clippy` ✅ (15 istniejących ostrzeżeń, zero nowych), `cargo test` ✅ (250 passed, 0 failed).

---

### 1.2 Walidacja zakresu czasu HH:MM / HH:MM:SS

**Status:** DONE ✅ (2026-09-29)
**Priorytet:** Wysoki
**Złożoność:** Niska
**Opis:** `QsoRecord::validate()` sprawdza jedynie długość (4/6 cyfr) i czy wszystkie znaki są cyframi. Dodać sprawdzenie: godzina 00–23, minuty 00–59 (oraz sekundy 00–59 dla HHMMSS). Dotyczy `time_on` i `time_off`.
**Zmodyfikowane pliki:** `src/core/qso.rs`
**Ryzyko:** Niskie — może odrzucić dotąd akceptowane (błędne) czasy przy importach; wymaga testu.
**Zależności:** brak

**Podsumowanie zmian:** Dodano prywatną funkcję `validate_time_field(raw, field_name)` (format + zakresy HH 00–23 / MM 00–59 / SS 00–59) i użyto jej dla `time_on` i `time_off` w `validate()`. Dodano 4 testy jednostkowe (godzina/minuty/sekundy poza zakresem + czasy graniczne 00:00 i 23:59:59). Walidacja: `cargo check` ✅, `cargo clippy` ✅ (15 istniejących ostrzeżeń, zero nowych), `cargo test` ✅ (254 passed, 0 failed).

---

### 1.3 Indeksy `country` i `freq` (nowa migracja)

**Status:** DONE ✅ (2026-09-29)
**Priorytet:** Wysoki
**Złożoność:** Niska
**Opis:** Dodać migrację (v4) tworzącą `CREATE INDEX idx_qso_country ON qso_records(country)` oraz `idx_qso_freq ON qso_records(freq)`.
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Niskie — migracja idempotentna (`IF NOT EXISTS`); krótki czas tworzenia indeksu na dużych bazach.
**Zależności:** brak

**Podsumowanie zmian:** Dodano `migration_4_indexes()` tworzącą indeksy `idx_qso_country` i `idx_qso_freq` (oba `IF NOT EXISTS`), zarejestrowano ją w liście migracji `init_schema()` i podniesiono `KNOWN_MAX_VERSION` 3 → 4 w `validate_schema()`. Zaktualizowano test `schema_migrations_record_versions_and_are_idempotent` (oczekiwana wersja 4 + asercja istnienia obu indeksów). Walidacja: `cargo check` ✅, `cargo clippy` ✅ (15 istniejących ostrzeżeń, zero nowych), `cargo test` ✅ (254 passed, 0 failed).

---

### 1.4 Walidacja formatu IOTA / SOTA / POTA

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Niska
**Opis:** Dodać walidację formatów: IOTA `XX-NNN`, SOTA `XX/YY-NNN`, POTA `XX-NNNN` w `validate()` (lub przy imporcie ADIF).
**Zmodyfikowane pliki:** `src/core/qso.rs` (ew. `src/core/adif.rs`)
**Ryzyko:** Niskie/Średnie — wymaga ustalenia, czy walidacja ma być twarda (odrzucenie) czy miękka (ostrzeżenie).
**Zależności:** brak

---

## ETAP 2 — Zgodność ADIF 3.1.8

### 2.1 Aktualizacja `ADIF_VERSION` do `"3.1.8"`

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Niska
**Opis:** Zmienić stałą `ADIF_VERSION` na `"3.1.8"` (specyfikacja z 26.09.2026) i zweryfikować listę pól/emisji względem 3.1.8.
**Zmodyfikowane pliki:** `src/core/adif.rs`
**Ryzyko:** Niskie — zmiana deklaratywna; wymaga przeglądu nowych pól ADIF 3.1.8 (osobne zadanie w razie potrzeby).
**Zależności:** brak

---

### 2.2 FT8/WSPR/JT65/JT9/MSK144 jako submode MFSK

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Średnia
**Opis:** Zgodnie z ADIF 3.1.8, FT8/FT4/Q65/WSPR/JT65/JT9/MSK144 to submodes `MFSK`. Obecnie zwracane jako emisje główne. Zaimplementować mapowanie `("MFSK", Some("FT8"))` **z opcją** zachowania `MODE=FT8` (wiele serwisów tego oczekuje) — decyzja konfiguracyjna.
**Zmodyfikowane pliki:** `src/core/adif.rs`
**Ryzyko:** Średnie — wpływa na eksport ADIF i kompatybilność z LoTW/eQSL; wymaga testów i decyzji produktowej.
**Zależności:** 2.1

---

### 2.3 Fallback pasma `"20m"` → błąd importu

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Niska
**Opis:** Gdy import ADIF nie ma BAND ani rozpoznawalnej FREQ, obecnie cicho ustawia `"20m"`. Zmienić na odrzucenie rekordu z raportem w `AdifImportResult.errors`.
**Zmodyfikowane pliki:** `src/core/adif.rs`
**Ryzyko:** Niskie — zmiana zachowania importu (dotąd „cichy" fallback).
**Zależności:** brak

---

## ETAP 3 — Bezpieczeństwo

### 3.1 Podpis cyfrowy ed25519 dla auto-updatera

**Status:** TODO
**Priorytet:** Krytyczny
**Złożoność:** Wysoka
**Opis:** Aktualizator weryfikuje wyłącznie SHA-256. Dodać weryfikację podpisu ed25519 release'ów (klucz publiczny osadzony w buildzie, podpisy w metadanych wydania GitHub).
**Zmodyfikowane pliki:** `src/cloud/updater.rs`, proces wydawniczy (CI), `Cargo.toml` (ew. `ed25519-dalek`)
**Ryzyko:** Wysokie — dotyka procesu dystrybucji; wymaga infrastruktury podpisywania.
**Zależności:** brak (ale **wymaga DESIGN.md** przed implementacją)

---

### 3.2 Walidacja identyfikatorów w `add_column_if_missing`

**Status:** TODO
**Priorytet:** Niski
**Złożoność:** Niska
**Opis:** `format!("ALTER TABLE {table} ADD COLUMN {column} {decl}")` — dodać walidację nazw regexem `^[a-zA-Z_][a-zA-Z0-9_]*$` (defense-in-depth; identyfikatory pochodzą z kodu, nie od użytkownika).
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Niskie — czysto obronne.
**Zależności:** brak

---

## ETAP 4 — Utrzymywalność bazy danych

### 4.1 Eliminacja 4× duplikacji SQL (INSERT/UPDATE)

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Lista 53 kolumn QSO jest ręcznie powielana w `insert_qso`, `restore_qso`, `batch_insert_qsos`, `update_qso` oraz stałej `QSO_COLUMNS`. Ujednolicić (makro `stringify_fields!`/`placeholders!` lub trait `to_params()`).
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Średnie — duża powierzchnia zmiany; **wymaga DESIGN.md** i testów regresyjnych.
**Zależności:** brak

---

### 4.2 Wspólny schemat tabeli (migration_3)

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Niska
**Opis:** `migration_3_foreign_keys` kopiuje cały schemat `qso_records` z `migration_1`. Wydzielić definicję schematu do wspólnej stałej.
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Niskie — refaktoryzacja deklaratywna.
**Zależności:** 4.1 (kolejność: po ujednoliceniu)

---

### 4.3 Determinizm `vacuum_if_needed`

**Status:** TODO
**Priorytet:** Niski
**Złożoność:** Niska
**Opis:** Zastąpić heurystykę `count % 1000 == 0` sprawdzeniem rozmiaru WAL lub `PRAGMA wal_autocheckpoint`.
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Niskie.
**Zależności:** brak

---

## ETAP 5 — Wydajność

### 5.1 Repaint on-demand (ograniczenie ciągłego 60 fps)

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** `request_repaint()` przy podłączonym CAT powoduje ciągły render. Zastąpić `request_repaint_after(...)` / repaint tylko przy realnej zmianie.
**Zmodyfikowane pliki:** `src/gui/app.rs` (lub wydzielony moduł)
**Ryzyko:** Średnie — dotyka pętli GUI; wymaga testów manualnych (płynność VFO).
**Zależności:** brak

---

### 5.2 `prepare_cached()` dla hot-path

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Niska
**Opis:** `find_previous_qsos` i inne zapytania wywoływane przy każdej zmianie znaku używać `prepare_cached()`.
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Niskie.
**Zależności:** brak

---

### 5.3 Cache prefiksów — `DashMap`/`RwLock`

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Niska
**Opis:** Zastąpić `Mutex<HashMap>` cache prefiksów (`prefix.rs`) strukturą optymalizującą odczyty.
**Zmodyfikowane pliki:** `src/core/prefix.rs`
**Ryzyko:** Niskie/Średnie — zmiana modelu współbieżności; wymaga testów.
**Zależności:** brak

---

### 5.4 Full-text search (FTS5)

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Średnia
**Opis:** Dodać wirtualną tabelę FTS5 (`callsign, name, comment`) z synchronizacją i triggerami.
**Zmodyfikowane pliki:** `src/core/database.rs`
**Ryzyko:** Średnie — nowa tabela + triggery; wymaga migracji i testów.
**Zależności:** brak

---

## ETAP 6 — Architektura (duże refaktoryzacje — każda wymaga DESIGN.md + podzadań)

### 6.1 Centralny typ błędu `AppError`

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Wprowadzić `thiserror` i ujednolicić `rusqlite::Result`, `Result<(), String>`, `&'static str`.
**Zmodyfikowane pliki:** cały projekt (`src/core/error.rs` nowy + usunięcie `String`-errors)
**Ryzyko:** Średnie/Wysokie — szeroka zmiana API wewnętrznego; **wymaga DESIGN.md** i planu migracji.
**Zależności:** brak (ale warunkuje 6.3)

---

### 6.2 Rozbicie `app.rs` (God Object) — meta-zadanie

**Status:** TODO
**Priorytet:** Krytyczny (utrzymywalność)
**Złożoność:** Wysoka
**Opis:** `app.rs` (5355 linii) podzielić na `AppState` / `AppServices` / `WindowStates` / `AppConfig` i wydzielić moduły. **Wymaga najpierw DESIGN.md** (mapa pól → struktury, kolejność ekstrakcji, plan migracji) i realizacji jako sekwencja podzadań 6.2.1–6.2.7, każde zatwierdzane osobno.
**Zmodyfikowane pliki:** `src/gui/app.rs` + nowe moduły `src/gui/`
**Ryzyko:** Wysokie — największe ryzyko regresji.
**Zależności:** brak (meta-zadanie realizowane przez podzadania)

---

### 6.2.1 Analiza modułów

**Status:** TODO
**Priorytet:** Krytyczny
**Złożoność:** Średnia
**Opis:** Inventaryzacja `app.rs`: zmapowanie pól/struktur i metod do obszarów funkcjonalnych (workspace, export, contest, CAT, cloud, reszta). Określenie granic ekstrakcji i zależności między modułami. Wynik: podstawa do DESIGN.md i planu migracji.
**Zmodyfikowane pliki:** brak zmian w kodzie (analiza + DESIGN.md)
**Ryzyko:** Niskie — faza analityczna, bez ryzyka regresji.
**Zależności:** brak

---

### 6.2.2 Workspace extraction

**Status:** TODO
**Priorytet:** Krytyczny
**Złożoność:** Średnia
**Opis:** Wydzielenie logiki workspace'ów / paneli / docking (`DockState`, `ViewPanelConfig`, zapis/przywracanie layoutu) z `app.rs` do dedykowanego modułu.
**Zmodyfikowane pliki:** `src/gui/app.rs`, nowy `src/gui/workspace*.rs`
**Ryzyko:** Średnie — dotyka startu GUI i serializacji layoutu.
**Zależności:** 6.2.1

---

### 6.2.3 Export extraction

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Niska
**Opis:** Wydzielenie logiki eksportu (ADIF/Cabrillo/PDF/GPX) z `app.rs`. Uwaga: `app_export.rs` już istnieje — zweryfikować, co jeszcze pozostaje w `app.rs` i domknąć.
**Zmodyfikowane pliki:** `src/gui/app.rs`, `src/gui/app_export.rs`
**Ryzyko:** Niskie — kontynuacja istniejącej ekstrakcji.
**Zależności:** 6.2.1

---

### 6.2.4 Contest extraction

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Wydzielenie logiki contestowej (stan contestu, punktacja, okno contestu) z `app.rs`.
**Zmodyfikowane pliki:** `src/gui/app.rs`, `src/gui/contest.rs`
**Ryzyko:** Średnie — powiązane z logowaniem QSO i punktacją.
**Zależności:** 6.2.1

---

### 6.2.5 CAT extraction

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Wydzielenie obsługi CAT (stan riga, VFO, połączenia, poll) z `app.rs`.
**Zmodyfikowane pliki:** `src/gui/app.rs`, nowy moduł `src/gui/` (lub `src/cat/`)
**Ryzyko:** Średnie — powiązane z pętlą repaint i wątkami CAT.
**Zależności:** 6.2.1

---

### 6.2.6 Cloud extraction

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Wydzielenie obsługi integracji chmurowych (upload, statusy, online sync) z `app.rs`.
**Zmodyfikowane pliki:** `src/gui/app.rs`, `src/gui/online_sync.rs`
**Ryzyko:** Średnie — operacje async i kolejki upload.
**Zależności:** 6.2.1

---

### 6.2.7 Final cleanup

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Końcowe porządkowanie: usunięcie martwego kodu, ujednolicenie granic modułów, doprowadzenie `app.rs` do roli cienkiego „composition root" (składanie stanu i serwisów, bez logiki biznesowej).
**Zmodyfikowane pliki:** `src/gui/app.rs` + moduły z 6.2.2–6.2.6
**Ryzyko:** Średnie — finalna konsolidacja po ekstrakcjach.
**Zależności:** 6.2.2, 6.2.3, 6.2.4, 6.2.5, 6.2.6

---

### 6.3 Warstwa serwisowa (QsoService, SyncService, AwardsService)

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Wysoka
**Opis:** Wydzielić logikę biznesową (zapis QSO + upload + nagrody) z GUI/DB do warstwy koordynującej.
**Zmodyfikowane pliki:** nowe `src/services/`, `src/gui/app.rs`
**Ryzyko:** Wysokie; **wymaga DESIGN.md**; zależne od 6.1 i 6.2.
**Zależności:** 6.1, 6.2

---

### 6.4 Trait `OnlineLogService` dla integracji

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Średnia
**Opis:** Wspólny trait dla QRZ/HamQTH/LoTW/eQSL/Club Log/POTA/SOTA.
**Zmodyfikowane pliki:** `src/cloud/*`, nowy trait
**Ryzyko:** Średnie; **wymaga DESIGN.md**.
**Zależności:** 6.3 (wskazane)

---

## ETAP 7 — Modernizacja stacku

### 7.1 Migracja `log` → `tracing`

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Zastąpić `log`+`env_logger` przez `tracing`+`tracing-subscriber` (structured logging, spans, kontekst callsign/QSO).
**Zmodyfikowane pliki:** cały projekt, `Cargo.toml`
**Ryzyko:** Średnie; **wymaga DESIGN.md** (hurtowa zmiana makr).
**Zależności:** brak

---

### 7.2 `std::sync::Mutex` → `parking_lot::Mutex`

**Status:** TODO
**Priorytet:** Średni
**Złożoność:** Niska
**Opis:** Zamienić `std::sync::Mutex` (nie-poisonable, szybszy). Dotyczy `Arc<Mutex<LogDatabase>>` i innych.
**Zmodyfikowane pliki:** `src/main.rs`, `src/core/*`, `Cargo.toml`
**Ryzyko:** Niskie/Średnie — zmiana semantyki poisoningu; wymaga audytu wszystkich `.lock().unwrap()`.
**Zależności:** brak

---

## ETAP 8 — Nowe funkcje

### 8.1 ESM / sekwencyjna nawigacja klawiaturowa QSO

**Status:** TODO
**Priorytet:** Krytyczny (UX contestowe)
**Złożoność:** Wysoka
**Opis:** Implementacja ESM (Enter Sends Message) i nawigacji TAB/ENTER przez pola formularza QSO (Callsign → RST → Exchange → Log).
**Zmodyfikowane pliki:** `src/gui/qso_entry.rs`, `src/gui/app.rs`
**Ryzyko:** Wysokie — dotyka interakcji formularza; wymaga testów UX.
**Zależności:** brak (wskazane po 6.2)

---

### 8.2 Import MASTER.DTA dla SCP

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Zastąpić/uzupełnić 12 hardcoded znaków importem plików `MASTER.DTA` (supercheckpartial.com) — lokalny import i opcjonalne pobieranie.
**Zmodyfikowane pliki:** `src/main.rs`, `src/core/scp.rs` (lub odpowiednik), `src/gui/`
**Ryzyko:** Średnie — nowe źródło danych + format pliku.
**Zależności:** brak

---

### 8.3 AccessKit (dostępność)

**Status:** TODO
**Priorytet:** Wysoki
**Złożoność:** Średnia
**Opis:** Włączyć feature `accesskit` w eframe (dostępny w 0.36).
**Zmodyfikowane pliki:** `Cargo.toml`
**Ryzyko:** Średnie — zmiana builda i potencjalny wpływ na rozmiar binarki.
**Zależności:** brak

---

## 0.4 Uwagi i zasady wykonania

1. **Jedno zadanie = jedna sesja.** Nigdy nie wykonuj więcej niż jedno zadanie z tego planu.
2. **Refaktoryzacje duże** (3.1, 4.1, 6.1–6.4, 7.1) **wymagają najpierw DESIGN.md** + planu migracji + podziału na podzadania, **bez zmiany kodu** do czasu zatwierdzenia.
3. **Zmiany minimalne**, bez hurtowego search/replace, bez przebudowy architektury poza zakresem, bez aktualizacji bibliotek poza zakresem.
4. Po każdym zadaniu: aktualizacja `PLAN.md` (status → DONE, data, podsumowanie) + raport oraz walidacja `cargo check`, `cargo clippy`, `cargo test`.
