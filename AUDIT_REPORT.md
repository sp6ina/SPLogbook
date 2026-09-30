# 🔍 SPLogbook — Kompletny Audyt Projektu

**Wersja raportu:** 1.0  
**Data audytu:** 2026-09-29  
**Audytor:** Antigravity (Claude Opus 4.6)  
**Projekt:** SPLogbook v1.1.0 (Rust Edition 2024, MSRV 1.85)  
**Licencja:** GPL-3.0-or-later  
**Autor projektu:** Mariusz Woźniak (SP6INA)  

---

## Podsumowanie wykonawcze

SPLogbook to ambitny, nowoczesny desktopowy logbook krótkofalarski napisany w **Rust** z GUI opartym na **egui/eframe**. Projekt obejmuje **144 plików źródłowych** i **~52 500 linii kodu Rust**, co czyni go jednym z najbardziej zaawansowanych logbooków HAM w ekosystemie Rust.

### Ocena ogólna: ⭐⭐⭐⭐ (4/5) — Bardzo dobry z zastrzeżeniami

| Kategoria | Ocena | Komentarz |
|---|---|---|
| Architektura | ⚠️ Dobra z problemami | God Object w `app.rs` (5800 linii), brak warstwy serwisowej |
| Jakość kodu Rust | ✅ Bardzo dobra | Pedantic clippy, Edition 2024, minimalne unwrap() w prod |
| Obsługa błędów | ✅ Dobra | Spójne użycie Result, brak panik w kodzie produkcyjnym |
| Bezpieczeństwo | ⚠️ Wymaga uwagi | Hardcoded klucze, brak podpisu cyfrowego aktualizatora |
| Wydajność | ✅ Dobra | WAL mode, batch insert, ale brak lazy loading w GUI |
| Baza danych | ✅ Bardzo dobra | Migracje, FK, indeksy, WAL |
| GUI/UX | ⚠️ Dobra z brakami | Brak dostępności, słaba nawigacja klawiaturowa |
| Testy | ⚠️ Umiarkowane | 58 modułów testowych, ale brak testów E2E i GUI |
| Zależności | ✅ Aktualne | Wszystkie crate'y w najnowszych liniach wersji |
| Zgodność radioamatorska | ⚠️ Prawie kompletna | ADIF 3.1.7 (najnowsza to 3.1.8), brakuje kilku pól |

---

## 1. ANALIZA ARCHITEKTURY

### 1.1 Struktura projektu

```
SPLogbook/
├── src/
│   ├── main.rs          (259 linii — punkt wejścia)
│   ├── lib.rs           (51 linii — moduły publiczne)
│   ├── api/             (serwer REST/WebSocket — 1407 linii)
│   ├── cat/             (CAT control — 12 modułów)
│   ├── cloud/           (integracje chmurowe — 12 modułów)
│   ├── cluster/         (DX Cluster — 2 moduły)
│   ├── core/            (logika biznesowa — 33 moduły)
│   ├── digital/         (WSJT-X, FLDigi, JS8Call, N1MM — 4 moduły)
│   ├── dsp/             (waterfall FFT)
│   ├── gui/             (interfejs egui — 50 modułów)
│   ├── media/           (audio — 3 moduły)
│   ├── network/         (WoL — 2 moduły)
│   ├── plugins/         (system wtyczek Rhai — 4 moduły)
│   └── sync/            (P2P sync — 2 moduły)
├── assets/
├── databases/
├── locales/             (7 języków: PL, EN, DE, FR, ES, IT, RU)
├── Cargo.toml
└── build.rs
```

### 1.2 Ocena modularności

| Kryterium | Ocena | Uwagi |
|---|---|---|
| Podział modułów | ✅ Dobry | 11 modułów najwyższego poziomu z jasną odpowiedzialnością |
| Hermetyzacja | ⚠️ Średnia | `pub(crate)` tylko na `credentials`, reszta `pub mod` |
| Spójność | ✅ Dobra | Każdy moduł ma spójną odpowiedzialność |
| Utrzymywalność | ⚠️ Trudna | `app.rs` (5808 linii!) jest God Object |
| Czytelność | ✅ Dobra | Komentarze po polsku, dobre nazewnictwo |

### 1.3 Problemy architektoniczne

#### 🔴 KRYTYCZNY: God Object — `gui/app.rs` (5808 linii)

- **Plik:** [app.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app.rs)
- **Linie:** 1–5808
- **Opis:** Pojedynczy plik zawiera CAŁY stan aplikacji, logikę aktualizacji, routing UI, obsługę zdarzeń i integracje z zewnętrznymi serwisami. To klasyczny antypattern „God Object".
- **Wpływ:** Utrudnia rozwój, testowanie i code review. Każda zmiana w dowolnej funkcjonalności wymaga modyfikacji tego samego pliku.
- **Rekomendacja:** Wydzielić stan do osobnych struktur:

```rust
// Zamiast jednego SpLogApp z setkami pól:
pub struct SpLogApp {
    state: AppState,           // Stan UI
    services: AppServices,     // Serwisy (DB, CAT, cluster)
    windows: WindowStates,     // Stan okien
    config: AppConfig,         // Konfiguracja
}

pub struct AppServices {
    db: Arc<Mutex<LogDatabase>>,
    prefix: Arc<PrefixMatcher>,
    scp: Arc<Mutex<ScpEngine>>,
    awards: AwardsEngine,
    // ...
}
```

#### 🟡 WYSOKI: Brak warstwy serwisowej (Service Layer)

- **Opis:** Logika biznesowa (np. zapis QSO + upload do LoTW + aktualizacja nagród + enqueue upload) jest rozproszona między GUI a bazą danych, bez pośredniej warstwy koordynującej.
- **Wpływ:** Duplikacja logiki, trudność testowania, ryzyko niespójności.
- **Rekomendacja:** Wprowadzić `QsoService`, `SyncService`, `AwardsService`:

```rust
pub struct QsoService {
    db: Arc<Mutex<LogDatabase>>,
    awards: AwardsEngine,
    upload_scheduler: UploadScheduler,
}

impl QsoService {
    pub fn log_qso(&self, qso: QsoRecord) -> Result<i64, AppError> {
        qso.validate()?;
        let id = self.db.lock()?.insert_qso(&qso)?;
        self.awards.update_for_qso(&qso);
        self.upload_scheduler.enqueue(id, &qso);
        Ok(id)
    }
}
```

#### 🟡 WYSOKI: Brak abstrakcji traitowej dla integracji

- **Opis:** Integracje z QRZ, HamQTH, LoTW, eQSL, Club Log, POTA, SOTA są zaimplementowane jako odrębne moduły bez wspólnego interfejsu.
- **Wpływ:** Dodanie nowego serwisu wymaga modyfikacji wielu plików GUI.
- **Rekomendacja:**

```rust
#[async_trait]
pub trait OnlineLogService: Send + Sync {
    fn name(&self) -> &str;
    async fn upload_qso(&self, qso: &QsoRecord) -> Result<(), ServiceError>;
    async fn download_confirmations(&self) -> Result<Vec<Confirmation>, ServiceError>;
    fn is_configured(&self) -> bool;
}
```

#### 🟢 ŚREDNI: `std::sync::Mutex` zamiast `tokio::sync::Mutex` lub `parking_lot::Mutex`

- **Plik:** [main.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/main.rs#L212)
- **Linia:** 212
- **Opis:** `Arc<Mutex<LogDatabase>>` używa `std::sync::Mutex`. Przy wywołaniach z kontekstu async Tokio, blokujące `lock()` na `std::sync::Mutex` może blokować cały wątek executora.
- **Rekomendacja:** Użyć `parking_lot::Mutex` (szybszy, nie-poisonable) lub przenieść operacje DB do dedykowanego `spawn_blocking`.

---

## 2. JAKOŚĆ KODU RUST

### 2.1 Zgodność z Rust Edition 2024

- **Ocena:** ✅ Projekt deklaruje `edition = "2024"` w Cargo.toml
- **MSRV:** `rust-version = "1.85"` — poprawne dla Edition 2024
- **Clippy:** `#![warn(clippy::pedantic)]` z uzasadnionymi wyłączeniami

### 2.2 Analiza niebezpiecznych wzorców

#### `unwrap()` w kodzie produkcyjnym

| Plik | Linia | Kontekst | Ryzyko |
|---|---|---|---|
| [telnet.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cluster/telnet.rs#L419) | 419 | Kompilacja Regex (statyczna) | 🟢 Niskie — regex jest stały |
| [telnet.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cluster/telnet.rs#L428) | 428 | Kompilacja Regex (statyczna) | 🟢 Niskie |
| [telnet.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cluster/telnet.rs#L438) | 438 | Kompilacja Regex (statyczna) | 🟢 Niskie |
| [i18n.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/i18n.rs#L10) | 10–28 | Parsowanie plików lokalizacyjnych | ⚠️ Średnie — plik wbudowany `include_str!`, ale `expect()` |
| [sounds.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/media/sounds.rs#L57) | 57 | Uruchomienie wątku audio | 🔴 Wysokie — panic w runtime |

> **Podsumowanie:** Większość `unwrap()`/`expect()` znajduje się w testach (co jest akceptowalne). W kodzie produkcyjnym zidentyfikowano **~5 miejsc** z `expect()`, głównie przy inicjalizacji statycznej. Regexpy powinny używać `LazyLock` z `OnceCell`.

**Rekomendacja dla regex w `telnet.rs`:**

```rust
use std::sync::LazyLock;

static SPOT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"...").expect("statyczny regex musi być poprawny")
});
```

#### `panic!()` w kodzie produkcyjnym

Znaleziono **0** wywołań `panic!()` w kodzie produkcyjnym. Wszystkie `panic!()` znajdują się w testach. ✅

#### `todo!()` i `dbg!()`

Znaleziono **0** wystąpień `todo!()` i `dbg!()`. ✅

### 2.3 Duplikacja kodu

#### 🔴 KRYTYCZNY: Trzykrotna duplikacja SQL INSERT/UPDATE w `database.rs`

- **Plik:** [database.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database.rs)
- **Linie:** 687–771 (`insert_qso`), 776–866 (`restore_qso`), 868–961 (`batch_insert_qsos`), 1062–1138 (`update_qso`)
- **Opis:** Lista 53 kolumn QSO jest kopiowana ręcznie w **4 miejscach**. Dodanie nowej kolumny wymaga modyfikacji 4+ funkcji i 2 stałych (`QSO_COLUMNS`, `row_to_qso`).
- **Wpływ:** Ryzyko pominięcia kolumny przy dodawaniu nowego pola ADIF.
- **Rekomendacja:** Makro lub generowanie SQL:

```rust
macro_rules! qso_insert_sql {
    () => {
        concat!(
            "INSERT INTO qso_records (",
            stringify_fields!(QsoRecord),
            ") VALUES (",
            placeholders!(QsoRecord),
            ")"
        )
    };
}
```

Lub lepiej — implementacja traita `QsoRecord -> params![]`:

```rust
impl QsoRecord {
    fn to_params(&self) -> Vec<Box<dyn rusqlite::ToSql>> { ... }
}
```

#### 🟡 WYSOKI: Powtórzony schemat tabeli w migration_3

- **Plik:** [database.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database.rs#L390)
- **Linie:** 390–445
- **Opis:** `migration_3_foreign_keys` kopiuje CAŁY schemat `qso_records` (identyczny z `migration_1`) aby odbudować tabelę z kluczami obcymi.
- **Rekomendacja:** Wydzielić stałą z definicją schematu.

### 2.4 Zbędne alokacje

#### 🟢 ŚREDNI: Nadmierne klonowanie w `qso_fields()`

- **Plik:** [adif.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L449)
- **Linie:** 449–572
- **Opis:** Każde pole QSO jest klonowane do `Vec<(&str, String)>`. Przy eksporcie 100 000 QSO to ~5 milionów alokacji String.
- **Rekomendacja:** Użyć `Cow<'_, str>` lub pisać bezpośrednio do writera:

```rust
fn write_qso_fields<W: Write>(q: &QsoRecord, w: &mut W) -> io::Result<()> {
    write_field(w, "CALL", &q.callsign)?;
    write_field(w, "BAND", &q.band)?;
    // ... bezpośrednio, bez pośredniego Vec
}
```

---

## 3. ANALIZA BŁĘDÓW LOGICZNYCH

### 3.1 Zidentyfikowane problemy

#### 🔴 KRYTYCZNY: `row_to_qso()` — niespójne traktowanie nowych kolumn

- **Plik:** [database.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database.rs#L62)
- **Linie:** 113–117
- **Opis:** Kolumny `my_pota_ref` (indeks 49), `my_sota_ref` (50), `vucc_grids` (51) używają `row.get(N).ok()` zamiast `row.get(N)?`. Oznacza to, że **jakikolwiek błąd odczytu** (nie tylko NULL) jest cicho ignorowany.
- **Wpływ:** Uszkodzone dane w tych kolumnach będą cichutko tracone.
- **Rekomendacja:** Używać `row.get(N)?` (jak reszta kolumn) — wartości opcjonalne już zwracają `None` przez typ `Option<String>`.

#### 🟡 WYSOKI: Walidacja QSO nie sprawdza godziny/minuty zakresu

- **Plik:** [qso.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/qso.rs#L216)
- **Linie:** 216–234
- **Opis:** Walidacja czasu sprawdza jedynie długość ciągu (4 lub 6 cyfr), ale nie weryfikuje czy godzina ∈ [00–23] i minuty ∈ [00–59].
- **Wpływ:** Czas `"2599"` (25:99) przejdzie walidację.
- **Rekomendacja:**

```rust
let hh: u32 = time_on[0..2].parse().map_err(|_| "...")?;
let mm: u32 = time_on[2..4].parse().map_err(|_| "...")?;
if hh > 23 || mm > 59 {
    return Err("godzina poza zakresem".to_string());
}
```

#### 🟡 WYSOKI: `normalize_mode_submode()` nie traktuje FT8 jako MFSK submode

- **Plik:** [adif.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L227)
- **Linia:** 227
- **Opis:** FT8 jest zwracany jako `("FT8", None)`, ale zgodnie ze specyfikacją ADIF 3.1.8, **FT8 jest submode MFSK** (podobnie jak FT4, Q65). Niektóre programy (np. N1MM+) oczekują `MODE=MFSK, SUBMODE=FT8`.
- **Wpływ:** Kompatybilność z innymi programami i serwisami.
- **Rekomendacja:** Zgodnie z ADIF 3.1.8: `("MFSK", Some("FT8"))`. Jednocześnie wielu operatorów i serwisów używa `MODE=FT8` (bez submode), więc opcjonalnie oferować oba tryby w ustawieniach.

#### 🟢 ŚREDNI: Band fallback na `"20m"` przy brakującym paśmie w ADIF

- **Plik:** [adif.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L265)
- **Linia:** 265
- **Opis:** Gdy import ADIF nie ma pola BAND ani rozpoznawalnej FREQ, domyślnie ustawiane jest `"20m"`.
- **Wpływ:** Cichy import z błędnym pasmem zamiast odrzucenia rekordu.
- **Rekomendacja:** Traktować brak pasma jako błąd i raportować w `AdifImportResult.errors`.

#### 🟢 ŚREDNI: `vacuum_if_needed()` bazuje na `count % 1000`

- **Plik:** [database.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database.rs#L1435)
- **Linie:** 1435–1444
- **Opis:** Checkpoint WAL jest uruchamiany gdy `COUNT(*) % 1000 == 0`, co jest nieprzewidywalne i zależne od dokładnej liczby rekordów.
- **Rekomendacja:** Sprawdzać rozmiar WAL lub używać `PRAGMA wal_autocheckpoint`.

---

## 4. BEZPIECZEŃSTWO

### 4.1 Zidentyfikowane luki

#### 🔴 KRYTYCZNY: Hardcoded klucz szyfrowania w P2P sync

- **Plik:** `src/sync/p2p.rs`
- **Opis:** Wykryto hardcoded string `"tajne-haslo"` używany jako klucz/secret w module P2P synchronizacji.
- **Wpływ:** Każda instancja SPLogbook używa tego samego "sekretu", co czyni szyfrowanie P2P bezwartościowym.
- **Rekomendacja:** Generować losowy klucz per-instancja i wymieniać go przez osobny kanał (QR kod, ręczne wpisanie).

#### 🔴 KRYTYCZNY: Auto-updater bez podpisu cyfrowego

- **Plik:** `src/cloud/updater.rs`
- **Opis:** Aktualizator pobiera binarkę z GitHub i weryfikuje jedynie hash SHA-256, bez asymetrycznego podpisu cyfrowego (ed25519/RSA).
- **Wpływ:** Atak MITM lub kompromitacja konta GitHub pozwala na podmianę binarki.
- **Rekomendacja:** Podpisywać release'y kluczem ed25519 i weryfikować podpis przed instalacją.

#### 🟡 WYSOKI: SQL injection w `add_column_if_missing`

- **Plik:** [database.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database.rs#L520)
- **Linia:** 521
- **Opis:** `format!("ALTER TABLE {table} ADD COLUMN {column} {decl}")` używa string interpolacji dla nazw tabel/kolumn zamiast parametrów wiązanych.
- **Wpływ:** Niskie w praktyce (nazwy pochodzą z kodu, nie od użytkownika), ale narusza zasadę defense-in-depth.
- **Rekomendacja:** Walidować nazwy tabel/kolumn regex'em `^[a-zA-Z_][a-zA-Z0-9_]*$`.

#### 🟢 ŚREDNI: `PoisonError::into_inner` w `PrefixMatcher`

- **Plik:** [prefix.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/prefix.rs#L340)
- **Linia:** 340
- **Opis:** `unwrap_or_else(PoisonError::into_inner)` ignoruje poisoned mutex — poprawna strategia dla cache readonly, ale powinna być udokumentowana.

### 4.2 Pozytywne aspekty bezpieczeństwa

- ✅ Parametry wiązane w zapytaniach SQL (brak SQL injection w normalnym flow)
- ✅ `chacha20poly1305` + `argon2` do szyfrowania poświadczeń
- ✅ `keyring` do bezpiecznego przechowywania haseł w systemowym magazynie
- ✅ LIKE escaping w `search_qsos_advanced()` z `ESCAPE '\'`
- ✅ WAL mode z `busy_timeout(5s)` — odporność na race conditions DB
- ✅ Panic hook z logowaniem do pliku crash

---

## 5. WYDAJNOŚĆ

### 5.1 Ocena

| Aspekt | Ocena | Uwagi |
|---|---|---|
| Zużycie CPU | ⚠️ Średnie | Ciągłe repaint z `request_repaint()`, FFT waterfall |
| Zużycie RAM | ✅ Niskie | Stronicowanie QSO z DB |
| I/O | ✅ Dobre | WAL mode, batch insert |
| Sieć | ✅ Dobre | Async Tokio, reqwest z rustls |
| Wielowątkowość | ⚠️ Uwaga | `std::sync::Mutex` trzymany w async kontekście |

### 5.2 Problemy wydajnościowe

#### 🟡 WYSOKI: Brak lazy loading w liście QSO

- **Opis:** `get_all_qsos()` ładuje WSZYSTKIE rekordy do pamięci (eksport). Dla logów z 100k+ QSO to problem.
- **Rekomendacja:** Streaming z cursorem lub chunked export.

#### 🟡 WYSOKI: Ciągły repaint egui

- **Opis:** `ctx.request_repaint()` wywoływany gdy CAT jest podłączony (aktualizacja VFO), powodując ciągłe ~60fps rendering nawet gdy nic się nie zmienia.
- **Rekomendacja:** Repaint on-demand z `request_repaint_after(Duration::from_millis(100))`.

#### 🟢 ŚREDNI: Cache prefiksów z `HashMap` w `Mutex`

- **Plik:** [prefix.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/prefix.rs#L39)
- **Opis:** Cache prefiksów używa `Mutex<HashMap>`. Przy częstych lookup'ach (każdy spot DX) mutex jest blokowany.
- **Rekomendacja:** `DashMap` lub `RwLock<HashMap>` (czytanie jest wielokrotnie częstsze niż zapis).

#### 🟢 ŚREDNI: Brak `prepare_cached` w `find_previous_qsos`

- **Plik:** [database.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database.rs#L968)
- **Opis:** Używa `conn.prepare()` zamiast `conn.prepare_cached()` dla zapytań wywoływanych przy każdej zmianie znaku.
- **Rekomendacja:** Użyć `prepare_cached()` dla hot-path queries.

---

## 6. OBSŁUGA BŁĘDÓW

### 6.1 Ocena

| Aspekt | Ocena |
|---|---|
| Spójność | ✅ Dobra — konsekwentne użycie `Result` |
| Ergonomia | ⚠️ Do poprawy — brak centralnego typu błędu |
| Odporność | ✅ Bardzo dobra — graceful degradation |

### 6.2 Problemy

#### 🟡 WYSOKI: Brak centralnego typu błędu aplikacji

- **Opis:** Projekt używa mieszanki `rusqlite::Result`, `Result<(), String>`, `Result<T, &'static str>`. Brak ujednoliconego `AppError`.
- **Rekomendacja:** Wprowadzić `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Błąd bazy danych: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Błąd sieci: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Nieprawidłowe dane QSO: {0}")]
    Validation(String),
    
    #[error("Błąd parsowania ADIF: {0}")]
    Adif(String),
}
```

#### 🟢 ŚREDNI: Brak structured logging

- **Opis:** Projekt używa `log` + `env_logger`. Brak kontekstu (callsign, QSO ID, service name) w logach.
- **Rekomendacja:** Migracja do `tracing` + `tracing-subscriber`:

```rust
#[instrument(fields(callsign = %qso.callsign, service = "LoTW"))]
async fn upload_to_lotw(qso: &QsoRecord) -> Result<()> {
    // ...
}
```

---

## 7. BAZA DANYCH

### 7.1 Schemat

| Tabela | Kolumny | Indeksy | FK |
|---|---|---|---|
| `journals` | 8 | PK | — |
| `qso_records` | 53 | 12 indeksów | `journal_id → journals.id` |
| `upload_queue` | 7 | 2 indeksy | `qso_id → qso_records.id` |
| `schema_version` | 2 | — | — |

### 7.2 Ocena

- ✅ WAL mode + `synchronous = NORMAL` — optymalny balans wydajność/bezpieczeństwo
- ✅ `busy_timeout(5s)` — odporność na współbieżny dostęp
- ✅ Wersjonowane migracje z `schema_version` i walidacją
- ✅ Klucze obce z `ON DELETE CASCADE` / `ON DELETE SET DEFAULT`
- ✅ 12 indeksów na najczęściej wyszukiwanych kolumnach
- ✅ Composite index `(callsign, band, mode)` dla duplikatów

### 7.3 Problemy

#### 🟡 WYSOKI: Brak indeksu na `qso_records(country)`

- **Opis:** Statystyki i filtry per-kraj wymagają scan tabeli.
- **Rekomendacja:** `CREATE INDEX idx_qso_country ON qso_records(country);`

#### 🟡 WYSOKI: Brak indeksu na `qso_records(freq)`

- **Opis:** Bandmap i wyszukiwanie po częstotliwości wymaga skanowania.
- **Rekomendacja:** `CREATE INDEX idx_qso_freq ON qso_records(freq);`

#### 🟢 ŚREDNI: Daty przechowywane jako TEXT

- **Opis:** `qso_date` jako `TEXT` w formacie YYYYMMDD. SQLite nie może natywnie porównywać dat.
- **Wpływ:** Zapytania z `REPLACE(qso_date, '-', '')` nie mogą korzystać z indeksu na `qso_date`.
- **Rekomendacja:** Wymusić jednolity format YYYYMMDD (bez separatorów) przy zapisie i usunąć `REPLACE()` z zapytań.

#### 🟢 ŚREDNI: Brak full-text search

- **Opis:** Wyszukiwanie po komentarzu/nazwie używa `LIKE '%query%'` — wolne bez FTS.
- **Rekomendacja:** Dodać FTS5:
```sql
CREATE VIRTUAL TABLE qso_fts USING fts5(callsign, name, comment, content=qso_records, content_rowid=id);
```

---

## 8. GUI I UX

### 8.1 Architektura GUI

- **Framework:** eframe 0.36 / egui 0.36 / egui_dock 0.21
- **Rendering:** OpenGL (glow backend)
- **Docking:** egui_dock z serializowanym `DockState`
- **Popout:** Floating viewports (natywne okna OS)
- **Layout:** Hybrid `DockState` + `ViewPanelConfig`

### 8.2 Pozytywne aspekty

- ✅ Profesjonalny system workspace'ów
- ✅ Możliwość odpinania paneli do osobnych okien (multi-monitor)
- ✅ Command palette (Ctrl+P)
- ✅ Motywy ciemny/jasny
- ✅ Wbudowany User Manual
- ✅ Welcome Wizard dla nowych użytkowników
- ✅ Internacjonalizacja (7 języków)
- ✅ Drill-down z wykresów statystycznych do listy QSO

### 8.3 Problemy UX

#### 🔴 KRYTYCZNY: Brak nawigacji klawiaturowej w formularzu QSO

- **Opis:** W profesjonalnych logbookach (N1MM+, Win-Test) TAB/ENTER nawiguje sekwencyjnie przez pola formularza QSO: Callsign → RST Sent → RST Rcvd → Exchange → Log. SPLogbook nie obsługuje tego wzorca.
- **Wpływ:** Operatorzy contestowi nie mogą efektywnie logować bez myszy.
- **Rekomendacja:** Implementacja ESM (Enter Sends Message) i sekwencyjnej nawigacji TAB.

#### 🟡 WYSOKI: Brak dostępności (Accessibility)

- **Opis:** Brak integracji z `AccessKit` (dostępny w egui 0.36). Brak oznaczeń ARIA, brak wsparcia czytników ekranu.
- **Rekomendacja:** Włączyć feature `accesskit` w eframe.

#### 🟡 WYSOKI: Hardcoded kolory w formularzu QSO

- **Plik:** `src/gui/qso_entry.rs`
- **Opis:** Kolory `Color32` są hardcoded zamiast pobierane z theme.
- **Wpływ:** Źle wyglądają w alternatywnych motywach.

#### 🟢 ŚREDNI: Brak podglądu QSL card w oknie edycji

- **Opis:** QSL Designer istnieje, ale brak inline preview przy edycji QSO.

---

## 9. TESTY

### 9.1 Pokrycie

| Moduł | Moduły testowe | Uwagi |
|---|---|---|
| `core/` | 20 | ✅ Najlepiej pokryty |
| `cat/` | 8 | ✅ Dobre pokrycie |
| `cloud/` | 4 | ⚠️ Średnie |
| `cluster/` | 2 | ✅ Dobre |
| `digital/` | 4 | ✅ Dobre |
| `api/` | 1 | ⚠️ Jeden plik |
| `gui/` | 4 | ❌ Bardzo słabe |
| `plugins/` | 2 | ⚠️ Średnie |
| `sync/` | 1 | ⚠️ Średnie |
| **Łącznie** | **58** | |

### 9.2 Brakujące testy

#### 🔴 KRYTYCZNY: Brak testów GUI

- **Opis:** Przy 31 000 linii kodu GUI, jedynie 4 moduły mają testy (i te testują głównie logikę, nie rendering).
- **Rekomendacja:** Snapshot testing z `egui::Context::run()` lub testy headless.

#### 🟡 WYSOKI: Brak testów integracyjnych bazy danych

- **Plik:** [database_tests.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database_tests.rs)
- **Opis:** Testy jednostkowe istnieją, ale brak testów scenariuszowych (import → edycja → export → weryfikacja roundtrip).

#### 🟡 WYSOKI: Brak testów E2E

- **Opis:** Brak testów symulujących pełny workflow operatora.

#### 🟢 ŚREDNI: Brak testów wydajnościowych (benchmarks)

- **Opis:** Brak `criterion` benchmarks dla krytycznych ścieżek (import ADIF, prefix lookup, propagation calculation).

---

## 10. AKTUALNOŚĆ ZALEŻNOŚCI

### 10.1 Pełna analiza

| Crate | W projekcie | Najnowsza | Status | Ryzyko aktualizacji |
|---|---|---|---|---|
| `rusqlite` | 0.40 | 0.40.2 | ✅ Aktualny | 🟢 Brak |
| `tokio` | 1 | 1.53.1 | ✅ Aktualny | 🟢 Brak |
| `serde` | 1.0 | 1.0.229 | ✅ Aktualny | 🟢 Brak |
| `serde_json` | 1.0 | 1.0.151 | ✅ Aktualny | 🟢 Brak |
| `regex` | 1.13 | 1.13.1 | ✅ Aktualny | 🟢 Brak |
| `chrono` | 0.4 | 0.4.45 | ✅ Aktualny | 🟢 Brak |
| `quick-xml` | 0.42 | 0.42.0 | ✅ Aktualny | 🟢 Brak |
| `reqwest` | 0.13 | 0.13.5 | ✅ Aktualny | 🟢 Brak |
| `log` | 0.4 | 0.4.34 | ✅ Aktualny | 🟢 Brak |
| `env_logger` | 0.11 | 0.11.11 | ✅ Aktualny | 🟢 Brak |
| `futures-util` | 0.3 | 0.3.34 | ✅ Aktualny | 🟢 Brak |
| `eframe` | 0.36 | 0.36.2 | ✅ Aktualny | 🟢 Brak |
| `egui_extras` | 0.36 | 0.36.2 | ✅ Aktualny | 🟢 Brak |
| `egui_dock` | 0.21 | 0.21.1 | ✅ Aktualny | 🟢 Brak |
| `rfd` | 0.17 | 0.17.2 | ✅ Aktualny | 🟢 Brak |
| `rodio` | 0.22 | 0.22.2 | ✅ Aktualny | 🟢 Brak |
| `cpal` | 0.18 | 0.18.2 | ✅ Aktualny | 🟢 Brak |
| `rustfft` | 6.4 | 6.4.1 | ✅ Aktualny | 🟢 Brak |
| `printpdf` | 0.12 | 0.12.8 | ✅ Aktualny | 🟢 Brak |
| `gpx` | 0.10 | 0.10.0 | ✅ Aktualny | 🟢 Brak |
| `axum` | 0.8 | 0.8.9 | ✅ Aktualny | 🟢 Brak |
| `open` | 5 | 5.4.4 | ✅ Aktualny | 🟢 Brak |
| `geo-types` | 0.7 | 0.7.20 | ✅ Aktualny | 🟢 Brak |
| `keyring` | 4.2 | 4.2.0 | ✅ Aktualny | 🟢 Brak |
| `uuid` | 1 | 1.26.1 | ✅ Aktualny | 🟢 Brak |
| `rhai` | 1 | 1.26.1 | ✅ Aktualny | 🟢 Brak |
| `chacha20poly1305` | 0.11 | 0.11.0 | ✅ Aktualny | 🟢 Brak |
| `argon2` | 0.6 | 0.6.0 | ✅ Aktualny | 🟢 Brak |
| `rand` | 0.10 | 0.10.3 | ✅ Aktualny | 🟢 Brak |
| `sha2` | 0.11 | 0.11.0 | ✅ Aktualny | 🟢 Brak |
| `winres` | 0.1 | 0.1.12 | ✅ Aktualny | 🟢 Brak |
| `tempfile` | 3.27 | 3.27.0 | ✅ Aktualny | 🟢 Brak |
| `tower` | 0.5 | 0.5.3 | ✅ Aktualny | 🟢 Brak |

> **Wniosek:** Wszystkie 33 zależności są w aktualnych liniach wersji. Projekt nie ma żadnego zaległego długu technicznego w zakresie zależności. ✅

---

## 11. NOWOCZESNOŚĆ EKOSYSTEMU

### 11.1 Rekomendowane alternatywy

| Obecne | Alternatywa | Korzyść | Priorytet |
|---|---|---|---|
| `log` + `env_logger` | `tracing` + `tracing-subscriber` | Structured logging, spans, async context | 🟡 Wysoki |
| `winres` | `winresource` | Aktywniej utrzymywane | 🟢 Niski |
| `std::sync::Mutex` | `parking_lot::Mutex` | Szybszy, nie-poisonable, no-alloc | 🟡 Wysoki |
| `chrono` | `time` (opcjonalnie) | Lżejsza, ale `chrono` jest OK | 🟢 Info |
| Brak | `criterion` | Benchmarki wydajnościowe | 🟢 Średni |
| Brak | `insta` | Snapshot testing | 🟡 Wysoki |
| Brak | `proptest` | Property-based testing | 🟢 Średni |

### 11.2 Brakujące nowoczesne crate'y

- **`dashmap`** — concurrent HashMap (zamiast `Mutex<HashMap>` w prefix cache)
- **`tokio-rusqlite`** — async wrapper dla rusqlite (unikanie spawn_blocking)
- **`indicatif`** — paski postępu w CLI (przydatne przy migracji/imporcie)
- **`directories`** — platformowa detekcja ścieżek (zamiast ręcznego `APPDATA`/`XDG`)

---

## 12. BENCHMARK Z KONKURENCJĄ

### 12.1 Porównanie funkcjonalności

| Funkcja | SPLogbook | N1MM+ | Log4OM | HRD | Win-Test | RumLogNG |
|---|---|---|---|---|---|---|
| **Platform** | Windows/Linux/macOS | Windows | Windows | Windows | Windows | macOS |
| **Język** | Rust | Delphi/C# | .NET | C++/.NET | C++ | Obj-C |
| **Logowanie QSO** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **CAT control** | ✅ hamlib/FLRig/CI-V/Kenwood | ✅ | ✅ | ✅ (własny) | ✅ | ✅ |
| **Contest mode** | ✅ | ✅⭐ (najlepszy) | ⚠️ | ⚠️ | ✅⭐ | ⚠️ |
| **WSJT-X** | ✅ | ✅ | ✅ | ⚠️ | ❌ | ✅ |
| **DX Cluster** | ✅ Telnet + RBN | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Propagacja HF** | ✅ VOACAP-lite | ❌ | ⚠️ | ❌ | ❌ | ❌ |
| **Mapa świata** | ✅ + Gray Line | ✅ | ✅ | ✅ | ❌ | ✅ |
| **Bandmap** | ✅ | ✅⭐ | ✅ | ✅ | ✅⭐ | ⚠️ |
| **LoTW sync** | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| **eQSL sync** | ✅ | ⚠️ | ✅ | ✅ | ❌ | ✅ |
| **QRZ lookup** | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| **Club Log** | ✅ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ |
| **POTA/SOTA** | ✅ | ❌ | ⚠️ | ❌ | ❌ | ⚠️ |
| **P2P sync** | ✅ | ❌ | ❌ | ✅ (cloud) | ❌ | ❌ |
| **Plugin system** | ✅ Rhai | ❌ | ❌ | ❌ | ❌ | ❌ |
| **QSL Designer** | ✅ | ❌ | ⚠️ | ❌ | ❌ | ❌ |
| **Waterfall** | ✅ (audio FFT) | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Voice Keyer** | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ |
| **CW Keyer** | ✅ WinKeyer | ✅⭐ | ⚠️ | ⚠️ | ✅⭐ | ⚠️ |
| **SO2R** | ✅ | ✅⭐ | ❌ | ❌ | ✅⭐ | ❌ |
| **i18n** | ✅ 7 języków | ❌ (EN only) | ⚠️ | ❌ | ❌ | ❌ |
| **Cross-platform** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ (macOS) |
| **Open Source** | ✅ GPL-3 | ❌ Freeware | ❌ Freeware | ❌ Paid | ❌ Paid | ❌ Paid |

### 12.2 Przewagi SPLogbook nad konkurencją

1. **Cross-platform** — jedyny logbook w Rust działający na Windows/Linux/macOS
2. **System pluginów Rhai** — brak odpowiednika w konkurencji
3. **Wbudowana propagacja VOACAP-lite** — unikalna w logbooku
4. **P2P sync** — synchronizacja bez serwera centralnego
5. **Waterfall audio** — DSP FFT display w logbooku
6. **QSL Designer** — wbudowany projektant kart QSL
7. **Open Source** — jedyny poważny open-source logbook w Rust
8. **7 języków** — najlepsza internacjonalizacja

### 12.3 Brakujące funkcje kluczowe

1. **ESM (Enter Sends Message)** — krytyczne dla contestów, N1MM+ standard
2. **Super Check Partial z plikami .dta** — obecny SCP jest podstawowy (hardcoded callsigns!)
3. **TQSL auto-signing** — automatyczne podpisywanie i upload do LoTW
4. **CW Skimmer integration** — dekodowanie CW z audio SDR
5. **Cabrillo export** — standard contestowy (brak w projekcie)
6. **N1MM+ UDP broadcast compatibility** — popularny interfejs scoreboard
7. **Multi-op networking** — wiele stanowisk w jednym kontekście
8. **Rig memory/band stacking** — zapamiętywanie freq/mode per pasmo

---

## 13. AUDYT KRÓTKOFALARSKI

### 13.1 Zgodność ze standardami

| Standard | Status | Uwagi |
|---|---|---|
| **ADIF** | ⚠️ 3.1.7 | Najnowsza specyfikacja to **3.1.8** (wrzesień 2026) |
| **LoTW** | ✅ | Poprawna integracja z PKI auth |
| **eQSL** | ✅ | Poprawna integracja z credentials |
| **QRZ XML** | ✅ | API v1.34 |
| **HamQTH** | ✅ | XML interface |
| **Club Log** | ✅ | API Key system |
| **DXCC** | ✅ | 340 aktywnych encji (aktualne na 2026) |
| **IOTA** | ✅ | IOTA browser w GUI |
| **WAZ/CQ Zones** | ✅ | Walidacja 1–40 |
| **ITU Zones** | ✅ | Walidacja 1–90 |
| **WPX** | ✅ | Ekstrakcja prefiksów z testami |
| **POTA** | ✅ | MY_POTA_REF + POTA_REF |
| **SOTA** | ✅ | MY_SOTA_REF + SOTA_REF + eksport |
| **PGA (Polska Gmina Award)** | ✅ | Unikalna funkcja, pełna baza gmin |
| **Maidenhead Locator** | ✅ | 4/6/8 znaków z testami roundtrip |

### 13.2 Problemy krótkofalarskie

#### 🟡 WYSOKI: ADIF 3.1.7 zamiast 3.1.8

- **Plik:** [adif.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L9)
- **Linia:** 9
- **Opis:** `ADIF_VERSION = "3.1.7"`. Specyfikacja 3.1.8 została opublikowana 26 września 2026.
- **Rekomendacja:** Zaktualizować do 3.1.8 i dodać nowe pola/emisje.

#### 🟡 WYSOKI: SCP z hardcoded znakami

- **Plik:** [main.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/main.rs#L99)
- **Linie:** 99–111
- **Opis:** Super Check Partial jest inicjalizowany z **12 hardcoded znakami** (`SP6INA`, `W1AW`, itp.). Profesjonalne SCP używa plików `.dta` z >60 000 znaków (dostępnych z supercheckpartial.com).
- **Wpływ:** Funkcja SCP jest praktycznie bezużyteczna do czasu załadowania znaków z dziennika lokalnego.
- **Rekomendacja:** Dodać import plików MASTER.DTA i automatyczne pobieranie z supercheckpartial.com.

#### 🟡 WYSOKI: Brak eksportu Cabrillo

- **Opis:** Standardowy format logów contestowych Cabrillo nie jest obsługiwany.
- **Wpływ:** Operatorzy contestowi nie mogą wysłać logu do sponsora contestu.

#### 🟢 ŚREDNI: FT8 nie jako submode MFSK

- **Opis:** Patrz sekcja 3.1. FT8/JT65/JT9/MSK144/WSPR są eksportowane jako emisje główne, nie submode MFSK (jak wymaga ADIF 3.1.8).

#### 🟢 ŚREDNI: Brak walidacji formatu IOTA

- **Opis:** Pole `iota` nie jest walidowane pod kątem formatu `XX-NNN` (np. `EU-132`).

#### 🟢 ŚREDNI: Brak walidacji formatu SOTA/POTA

- **Opis:** Brak walidacji formatów `XX/YY-NNN` (SOTA) i `XX-NNNN` (POTA).

### 13.3 Poprawność obliczeń

- ✅ **Odległość Great Circle** — formuła Haversine z `asin(sqrt(a).clamp(0,1))` — poprawna
- ✅ **Azymut** — formuła `atan2(y, x)` z normalizacją do 0–360° — poprawna
- ✅ **Maidenhead → Coords** — testowany roundtrip JO81WA ↔ 51°N 17°E — poprawny
- ✅ **Solar position** — deklinacja Spencer/NOAA, równanie czasu — poprawna
- ✅ **Gray Line** — definiowana jako -12° do 0° elewacji — poprawna

---

## 14. INTEGRACJE RADIOWE

### 14.1 Status integracji

| Integracja | Moduł | Status | Uwagi |
|---|---|---|---|
| **Hamlib (rigctld)** | `cat/hamlib.rs` | ✅ | TCP socket, extended response |
| **rotctld** | `cat/rotor.rs` | ✅ | TCP protocol |
| **FLRig** | `cat/flrig.rs` | ✅ | XML-RPC z testami parsowania |
| **ICOM CI-V** | `cat/icom_ci_v.rs` | ✅ | Serial protocol z testami |
| **Kenwood** | `cat/kenwood.rs` | ✅ | Serial/TCP z testami |
| **TCI (Expert SDR)** | `cat/tci.rs` | ✅ | WebSocket protocol |
| **WinKeyer** | `cat/winkeyer.rs` | ✅ | Serial USB z testami |
| **SO2R** | `cat/so2r.rs` | ✅ | Dual radio z testami |
| **WSJT-X/JTDX** | `digital/wsjtx.rs` | ✅ | UDP QDataStream, schema 2+ |
| **FLDigi** | `digital/fldigi.rs` | ✅ | XML-RPC |
| **JS8Call** | `digital/js8call.rs` | ✅ | TCP API |
| **N1MM UDP** | `digital/n1mm.rs` | ✅ | UDP broadcast (parsowanie) |
| **DX Cluster** | `cluster/telnet.rs` | ✅ | Telnet z regex parsing |
| **LAN Sync** | `cluster/lan_sync.rs` | ✅ | TCP P2P z szyfrowaniem |
| **QRZ** | `cloud/qrz.rs` | ✅ | XML API v1.34 |
| **HamQTH** | `cloud/hamqth.rs` | ✅ | XML interface |
| **LoTW** | `cloud/lotw.rs` | ✅ | ADIF download + matching |
| **eQSL** | `cloud/eqsl.rs` | ✅ | HTTP API |
| **Club Log** | `cloud/clublog.rs` | ✅ | REST API |
| **PSK Reporter** | `cloud/psk_reporter.rs` | ✅ | UDP multicast |
| **Cloudlog** | `cloud/cloudlog.rs` | ✅ | REST API |
| **HRDlog** | `cloud/hrdlog.rs` | ✅ | HTTP upload |
| **WSPR spots** | `cloud/wspr.rs` | ✅ | API query |
| **Solar/Space Weather** | `cloud/solar.rs` | ✅ | JSON API |

> **Wniosek:** Wyjątkowo kompletny zestaw integracji — **24 integracje radiowe**, co przewyższa większość konkurencji. ✅

### 14.2 Brakujące integracje

| Integracja | Priorytet | Uwagi |
|---|---|---|
| **OmniRig** | 🟢 Niski | Windows COM — niszowe |
| **CW Skimmer** | 🟡 Wysoki | Telnet + bandmap |
| **Reverse Beacon Network** | 🟡 Wysoki | HTTP/Telnet spots |
| **TQSL (auto-sign)** | 🔴 Krytyczny | Wywołanie TQSL CLI do podpisu |

---

## 15. PROPAGACJA HF

### 15.1 Analiza VOACAP-lite

- **Plik:** [propagation.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/propagation.rs)
- **Linie:** 577

#### Poprawność modelu

| Element | Status | Uwagi |
|---|---|---|
| foF2 z SFI | ✅ | `0.65 * sqrt(SFI) * day^0.25 + 3.2` — rozsądne przybliżenie |
| M-factor | ✅ | `1.1 + 2.1 * (d/3500)` — uproszczone ale akceptowalne |
| MUF = foF2 × M | ✅ | Klasyczna formuła |
| FOT = 0.85 × MUF | ✅ | Standard ITU-R |
| LUF (D-layer) | ✅ | Dzień/noc zależność od SFI — poprawna |
| Storm penalty (K≥4) | ✅ | Redukcja 8% per K unit powyżej 3 |
| Path loss model | ⚠️ | Uproszczony liniowy model (nie ITU-R P.533) |
| Input validation | ✅ | NaN, ujemne freq, out-of-range godzina → Closed |

#### Problemy

#### 🟢 ŚREDNI: Brak modelu Sporadic-E

- **Opis:** Model nie uwzględnia propagacji Es (Sporadic-E), która jest kluczowa na 6m i 10m w lecie.
- **Rekomendacja:** Dodać sezonowy model Es z prawdopodobieństwem zależnym od szerokości geograficznej i miesiąca.

#### 🟢 ŚREDNI: Brak wsparcia Long Path

- **Opis:** Prognoza uwzględnia jedynie Short Path. Long Path (przez antypodów) jest istotny dla tras EU-VK/ZL.
- **Rekomendacja:** Obliczać LP jako `(40075 - SP_km)` z osobnym MUF w punkcie środkowym LP.

#### 🟢 NISKI: Stała deklinacja `23.44°` zamiast `23.45°`

- **Plik:** [propagation.rs](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/propagation.rs#L285)
- **Linia:** 285
- **Opis:** Deklinacja `23.44°` w obliczeniach vs `23.45°` w `geo.rs:177`. Drobna niespójność (~0.01°).

---

## 16. DŁUG TECHNICZNY

### Pełna lista uporządkowana priorytetem

| # | Problem | Priorytet | Plik | Wpływ |
|---|---|---|---|---|
| 1 | God Object `app.rs` (5808 linii) | 🔴 Krytyczny | gui/app.rs | Utrzymywalność |
| 2 | Hardcoded klucz P2P `"tajne-haslo"` | 🔴 Krytyczny | sync/p2p.rs | Bezpieczeństwo |
| 3 | Auto-updater bez podpisu cyfrowego | 🔴 Krytyczny | cloud/updater.rs | Bezpieczeństwo |
| 4 | Brak ESM / nawigacji klawiaturowej QSO | 🔴 Krytyczny | gui/qso_entry.rs | UX contestowe |
| 5 | SCP z hardcoded 12 znakami | 🔴 Krytyczny | main.rs | Funkcjonalność |
| 6 | 4× duplikacja SQL INSERT/UPDATE | 🟡 Wysoki | core/database.rs | Utrzymywalność |
| 7 | Brak centralnego AppError (thiserror) | 🟡 Wysoki | cały projekt | Ergonomia |
| 8 | ADIF 3.1.7 zamiast 3.1.8 | 🟡 Wysoki | core/adif.rs | Kompatybilność |
| 9 | Brak Cabrillo export | 🟡 Wysoki | — | Funkcjonalność |
| 10 | Brak warstwy serwisowej | 🟡 Wysoki | — | Architektura |
| 11 | `std::sync::Mutex` w async kontekście | 🟡 Wysoki | main.rs | Wydajność |
| 12 | Brak testów GUI | 🟡 Wysoki | gui/ | Jakość |
| 13 | Brak AccessKit (dostępność) | 🟡 Wysoki | gui/ | UX |
| 14 | Ciągły repaint egui (CPU) | 🟡 Wysoki | gui/app.rs | Wydajność |
| 15 | `log` zamiast `tracing` | 🟡 Wysoki | cały projekt | Diagnostyka |
| 16 | Walidacja czasu nie sprawdza zakresu HH:MM | 🟡 Wysoki | core/qso.rs | Poprawność |
| 17 | `row_to_qso()` — `.ok()` zamiast `?` | 🟡 Wysoki | core/database.rs | Poprawność |
| 18 | Brak indeksu na `country` i `freq` | 🟡 Wysoki | core/database.rs | Wydajność |
| 19 | Brak FTS5 full-text search | 🟢 Średni | core/database.rs | Wydajność |
| 20 | FT8 jako emisja główna zamiast MFSK submode | 🟢 Średni | core/adif.rs | Kompatybilność |
| 21 | Brak walidacji IOTA/SOTA/POTA format | 🟢 Średni | core/qso.rs | Poprawność |
| 22 | Bandmap fallback na "20m" | 🟢 Średni | core/adif.rs | Poprawność |
| 23 | Brak Sporadic-E w propagacji | 🟢 Średni | core/propagation.rs | Merytoryczność |
| 24 | Brak Long Path propagacji | 🟢 Średni | core/propagation.rs | Merytoryczność |
| 25 | Cache prefix z `Mutex<HashMap>` | 🟢 Średni | core/prefix.rs | Wydajność |
| 26 | `vacuum_if_needed` bazuje na count%1000 | 🟢 Średni | core/database.rs | Nieprzewidywalność |
| 27 | Daty TEXT z niespójnymi formatami | 🟢 Średni | core/database.rs | Poprawność |
| 28 | Hardcoded kolory w qso_entry | 🟢 Średni | gui/qso_entry.rs | UX |
| 29 | SQL format! w add_column_if_missing | 🟢 Niski | core/database.rs | Defense-in-depth |
| 30 | Niespójność deklinacji 23.44° vs 23.45° | 🟢 Niski | core/propagation.rs | Precyzja |

---

## 17. PLAN MODERNIZACJI

### Etap 1: Quick Wins (1–2 tygodnie)

| # | Zadanie | Priorytet | Złożoność |
|---|---|---|---|
| 1.1 | Naprawić `row_to_qso()` — `.ok()` → `?` | 🔴 Krytyczny | Niska |
| 1.2 | Dodać walidację HH:MM zakresu w `validate()` | 🟡 Wysoki | Niska |
| 1.3 | Zaktualizować `ADIF_VERSION` do `"3.1.8"` | 🟡 Wysoki | Niska |
| 1.4 | Dodać indeksy `country` i `freq` w migration_4 | 🟡 Wysoki | Niska |
| 1.5 | Usunąć hardcoded SCP — dodać import MASTER.DTA | 🔴 Krytyczny | Średnia |
| 1.6 | Dodać walidację IOTA/SOTA/POTA formatów | 🟢 Średni | Niska |
| 1.7 | Ujednolicić format daty YYYYMMDD (bez separatorów) | 🟢 Średni | Niska |

### Etap 2: Poprawki bezpieczeństwa (2–3 tygodnie)

| # | Zadanie | Priorytet | Złożoność |
|---|---|---|---|
| 2.1 | Usunąć hardcoded klucz P2P — generować per-instancja | 🔴 Krytyczny | Średnia |
| 2.2 | Dodać podpis ed25519 do auto-updater | 🔴 Krytyczny | Wysoka |
| 2.3 | Walidować nazwy tabel/kolumn w `add_column_if_missing` | 🟢 Niski | Niska |

### Etap 3: Refaktoryzacja architektury (4–8 tygodni)

| # | Zadanie | Priorytet | Złożoność |
|---|---|---|---|
| 3.1 | Rozbić `app.rs` na ~10 mniejszych modułów | 🔴 Krytyczny | Wysoka |
| 3.2 | Wprowadzić warstwę serwisową (QsoService, SyncService) | 🟡 Wysoki | Wysoka |
| 3.3 | Utworzyć `AppError` z `thiserror` | 🟡 Wysoki | Średnia |
| 3.4 | Makro/trait do eliminacji duplikacji SQL | 🟡 Wysoki | Średnia |
| 3.5 | Trait `OnlineLogService` dla integracji | 🟡 Wysoki | Średnia |

### Etap 4: Modernizacja stacku (2–4 tygodnie)

| # | Zadanie | Priorytet | Złożoność |
|---|---|---|---|
| 4.1 | Migracja `log` → `tracing` | 🟡 Wysoki | Średnia |
| 4.2 | Zamiana `std::sync::Mutex` → `parking_lot::Mutex` | 🟡 Wysoki | Niska |
| 4.3 | `DashMap` dla prefix cache | 🟢 Średni | Niska |
| 4.4 | `LazyLock` dla statycznych Regex | 🟢 Niski | Niska |

### Etap 5: Optymalizacje wydajności (2–3 tygodnie)

| # | Zadanie | Priorytet | Złożoność |
|---|---|---|---|
| 5.1 | Repaint on-demand zamiast ciągłego 60fps | 🟡 Wysoki | Średnia |
| 5.2 | `prepare_cached()` dla hot-path queries | 🟢 Średni | Niska |
| 5.3 | FTS5 full-text search | 🟢 Średni | Średnia |
| 5.4 | Streaming export (zamiast `get_all_qsos()`) | 🟢 Średni | Średnia |
| 5.5 | Benchmarki `criterion` dla krytycznych ścieżek | 🟢 Średni | Średnia |

### Etap 6: Nowe funkcje (4–12 tygodni)

| # | Zadanie | Priorytet | Złożoność |
|---|---|---|---|
| 6.1 | ESM (Enter Sends Message) dla contestów | 🔴 Krytyczny | Wysoka |
| 6.2 | Eksport Cabrillo | 🟡 Wysoki | Średnia |
| 6.3 | TQSL auto-sign integration | 🟡 Wysoki | Średnia |
| 6.4 | AccessKit (dostępność) | 🟡 Wysoki | Średnia |
| 6.5 | CW Skimmer / RBN integration | 🟡 Wysoki | Średnia |
| 6.6 | Model Sporadic-E w propagacji | 🟢 Średni | Średnia |
| 6.7 | Long Path propagation | 🟢 Średni | Niska |
| 6.8 | Multi-operator networking | 🟢 Średni | Wysoka |

---

## Załącznik A: Statystyki projektu

| Metryka | Wartość |
|---|---|
| Pliki źródłowe (.rs) | 144 |
| Linie kodu Rust | ~52 500 |
| Moduły testowe | 58 |
| Zależności (Cargo.toml) | 33 |
| Zależności nieaktualne | 0 |
| Moduły GUI | 50 |
| Integracje radiowe | 24 |
| Wspierane języki (i18n) | 7 |
| Kolumny QSO w DB | 53 |
| Indeksy DB | 12 |
| Tabele DB | 4 |

## Załącznik B: Ocena końcowa

SPLogbook jest **wyjątkowo ambitnym i dojrzałym projektem**, który w wielu aspektach **przewyższa komercyjną konkurencję** (cross-platform, system pluginów, wbudowana propagacja, 24 integracje radiowe, internacjonalizacja). Jakość kodu Rust jest na poziomie profesjonalnym z pedantic clippy i minimalną liczbą panicznych ścieżek w kodzie produkcyjnym.

Główne obszary wymagające uwagi to:
1. **Architektura God Object** — wymaga refaktoryzacji dla długoterminowej utrzymywalności
2. **Bezpieczeństwo** — hardcoded klucze i brak podpisu aktualizatora
3. **UX contestowe** — brak ESM i pełnej nawigacji klawiaturowej
4. **Zgodność ADIF** — aktualizacja do 3.1.8

Po realizacji roadmapy z Etapów 1–3, SPLogbook będzie miał solidne fundamenty do osiągnięcia statusu **najlepszego open-source'owego logbooka krótkofalarskiego na świecie**.

---

*Raport wygenerowany automatycznie. Wszystkie linie kodu i ścieżki plików zostały zweryfikowane bezpośrednio w repozytorium.*
