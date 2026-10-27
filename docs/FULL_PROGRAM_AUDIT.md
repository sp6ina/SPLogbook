# Kompleksowy Audyt Techniczny i Architektoniczny Programu SPLogbook (`FULL_PROGRAM_AUDIT.md`)

**Dokument główny audytu:** `docs/FULL_PROGRAM_AUDIT.md`  
**Projekt:** SPLogbook (`v1.2.0`)  
**Autor projektu:** Mariusz Woźniak (SP6INA)  
**Gałąź referencyjna:** `main`  
**Punkt odniesienia Git (HEAD przed zapisem dokumentów audytu):** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` (`6f7d3aa`, tag `v1.2.0`)  
**Data audytu:** 2026-10-27  
**Tryb audytu:** Pełny, dowodowy, bez założeń, obejmujący 100% kodu źródłowego, testów, konfiguracji i historii Git

---

## 1. Streszczenie Wykonawcze (Executive Summary)

SPLogbook (`v1.2.0`) to rozbudowana, wieloplatformowa aplikacja desktopowa w języku Rust (Edition 2024, `eframe`/`egui` 0.36, `tokio` 1.53, `rusqlite` 0.40) przeznaczona do prowadzenia dziennika stacji krótkofalarskiej, obsługi zawodów, śledzenia dyplomów, synchronizacji chmurowej i LAN oraz bezpośredniego sterowania transceiverami (CAT).

Przeprowadzony audyt potwierdził, że projekt przeszedł rzeczywistą i szeroko zakrojoną transformację inżynierską (fale refaktoryzacyjne **W0–W8** oraz **NATIVE-0..7**):
- Wyeliminowano paniki w ścieżkach produkcyjnych i ujednolicono obsługę błędów domenowych (brak `Result<_, String>` chroniony przez test strażniczy `tests/i18n_guard.rs`).
- Wdrożono pełną lokalizację 7 języków (`pl`, `en`, `de`, `fr`, `es`, `it`, `ru`) z walidacją parametrów i zakazem zahardkodowanych napisów w GUI.
- Zbudowano nowoczesną architekturę wieloradiową (`RigHub`, `RigService`, `TxArbiter`) oraz 5 natywnych kodeków strumieniowych (Icom CI-V, Kenwood, Yaesu, Elecraft, FlexRadio SmartSDR) wraz z rozbudowaną domeną walidacji HIL (`src/cat/hil/`).
- Zaimplementowano szyfrowany protokół synchronizacji LAN Multi-Op (`Argon2id` + `ChaCha20-Poly1305` z ochroną AAD przed atakami replay i zmianą kolejności ramek).
- Wszystkie **1 519 aktywnych testów automatycznych** (983 jednostkowe + 536 integracyjnych w 5 zestawach) przechodzi z kodem wyjścia `0`.

Jednocześnie bezkompromisowa analiza kodu źródłowego ujawniła **74 konkretne problemy** (`1 BLOCKER`, `12 CRITICAL`, `29 HIGH`, `25 MEDIUM`, `6 LOW`, `1 INFO`), które sprawiają, że w obecnym stanie gałąź `main` **wymaga zamknięcia blokerów bezpieczeństwa, integralności danych i protokołów CAT przed uznaniem jej za w pełni gotową produkcyjnie i sprzętowo**:

1. **`SEC-001` [BLOCKER]**: W śledzonym przez Git pliku [`src/scratch.rs:4`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/scratch.rs#L4) znajduje się jawne ziarno klucza prywatnego Ed25519 `[1u8; 32]`, z którego wygenerowano jedyny aktywny klucz publiczny `splogbook-release-2026-01` w [`src/cloud/updater.rs:257-266`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L257-L266). Unieważnia to kryptograficzną ochronę auto-updatera.
2. **`REL-001` & `CI-001` [CRITICAL / HIGH]**: Instalator aktualizacji (`install_update`) nie działa ani na Windows (`Expand-Archive` odrzuca plik `.tmp` bez rozszerzenia `.zip`), ani na Linux (brak zdjęcia nadrzędnego katalogu `SPLogbook-Linux-x86_64/` z archiwum `.tar.gz`), a workflow `.github/workflows/build-and-release.yml` w ogóle nie generuje i nie dołącza pliku `release-manifest.json` ani `.sig`.
3. **`CAT-001`..`CAT-006` [CRITICAL / HIGH]**:
   - W `RigService::run_loop` brakuje automatycznego `reconnect` po przejściu w stan `Degraded` (`CAT-001`).
   - W 5 z 8 backendów (`IcomCiV`, `Yaesu`, `Elecraft`, `FlexRadio`, `TCI`) metoda `probe()` została napisana poza `impl RigBackend`, przez co polimorficzne `Box<dyn RigBackend>::probe()` wywołuje domyślne `connect()` wysyłające komendy mutujące `AI1;`/`AI2;`/`sub slice all` (`CAT-003`).
   - W `IcomCiVBackend` dekodowanie S-metra/mocy oraz kodowanie `SetPower` używa Little-Endian BCD zamiast Big-Endian BCD (`0000..0255`), przez co S-meter pokazuje `S9+60dB`, a `SetPower` jest odrzucane przez radio (`CAT-004`).
   - W `TciBackend` i `FlexRadioBackend` występuje wyścig potwierdzenia PTT powodujący fałszywą blokadę `TxArbiterState::Unknown` (`CAT-002`), natomiast `YaesuBackend` i `ElecraftBackend` optymistycznie ustawiają `confirmed_ptt = true` bez odczytu z radia (`CAT-005`).
   - `RigHub::resync_tx_arbiter` sprawdza `last_update` zamiast `last_state_update` (`CAT-006`), a `HilHardwareSession::execute_controlled_ptt` omija bramki `SPLOG_HIL_*` i `TxArbiter` (`CAT-015`).
4. **`DATA-001`..`DATA-005` [CRITICAL / HIGH]**:
   - Parser ADIF (`src/core/adif.rs`) fabrykuje dzisiejszą datę `Utc::now()` przy braku `<QSO_DATE>`/`<TIME_ON>` i przecina wielobajtowe polskie znaki UTF-8 (`DATA-001`).
   - `QsoIngestor::import_batch` pomija wywołanie `normalize_qso_fields` (`DATA-002`).
   - Usuwanie duplikatów (`find_duplicates.rs`) i `delete_selected_qso` działają bez potwierdzenia, bez zapisu na `undo_stack`, a szukanie duplikatów skanuje wszystkie dzienniki naraz bez filtra `journal_id` (`DATA-004`).
   - Przełączenie aktywnego dziennika nie przebudowuje `AwardsEngine`, a eksporty chmurowe i statystyki mieszają łączności ze wszystkich dzienników (`DATA-005`).
5. **`FUNC-001`..`FUNC-006` [CRITICAL / HIGH]**:
   - Eksporter Cabrillo 3.0 (`src/core/cabrillo.rs`) odwraca kolejność QSO, ignoruje `rst_sent`/`rst_rcvd`, eksportuje FT8/RTTY jako `"PH"` i gubi pola wymiany `srx_string`/`stx_string` (`FUNC-001`).
   - Endpoint `POST /api/v1/rig` mutuje tylko pamięć RAM i nigdy nie steruje radiem (`FUNC-003`).
   - Przycisk STOP nagrywania audio wywołuje `stop_audio()` zamiast `stop_recording()`, Terminal CW / WinKeyer nie ma warstwy portu szeregowego, `Js8CallListener` nigdy się nie uruchamia, a `src/core/clubs.rs` ma zahardkodowane tylko 11 znaków (`FUNC-004`, `FUNC-006`).
6. **`PERF-001` & `TEST-001` [CRITICAL / HIGH]**:
   - `chunk_inventory`, `chunk_requests` i `chunk_records` w `src/sync/catchup.rs` mają kwadratową złożoność $O(N^2)$ (klonowanie i serializacja całego wektora JSON w każdej iteracji), przez co 3 testy jednostkowe wydłużają czas `cargo test` z ~15 s do **532 s** (~9 minut).

---

## 2. Identyfikacja Środowiska i Weryfikacja Git (ETAP 0 & ETAP 1)

### 2.1. Środowisko wykonawcze i toolchain
- **Katalog główny repozytorium:** `c:\Users\sp6in\Documents\SPLogbook\SPLogbook`
- **System operacyjny i powłoka:** Windows 10/11 (`x86_64-pc-windows-msvc`), PowerShell
- **Aktywny toolchain Rust:** `stable-x86_64-pc-windows-msvc`
  - `rustc 1.99.0 (b940084d7 2026-09-28)`
  - `cargo 1.99.0 (5f94df478 2026-08-27)`
  - `clippy 0.1.99 (b940084d75 2026-09-28)`
  - `rustfmt 1.8.0-stable (b940084d75 2026-09-28)`
  - `cargo-audit 0.22.2`
- **Struktura Cargo:** Pojedynczy pakiet (`name = "splogbook"`, `version = "1.2.0"`, `edition = "2024"`), cele binarne: `SPLogbook` (`src/main.rs`) oraz `splogbook_signer` (`src/bin/splogbook_signer.rs`).
- **Pliki instrukcyjne i planistyczne w repozytorium:**
  - Obecne i przeanalizowane: [`README.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/README.md), [`Cargo.toml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Cargo.toml), [`Cargo.lock`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Cargo.lock), [`MASTER_IMPLEMENTATION_PLAN.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/MASTER_IMPLEMENTATION_PLAN.md), [`CODE_QUALITY_NORMALIZATION_PLAN.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/CODE_QUALITY_NORMALIZATION_PLAN.md), [`Native CAT.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Native%20CAT.md), `Refaktoryzacje.md` (plik lokalny zignorowany w `.gitignore`), [`.github/workflows/ci.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/ci.yml), [`.github/workflows/build-and-release.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/build-and-release.yml).
  - Nieobecne w repozytorium (odnotowane zgodnie z instrukcją): brak plików `AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, `CONTRIBUTING.md`, brak wcześniejszych plików w katalogu `docs/`.

### 2.2. Wyniki bezpiecznej weryfikacji Git przed audytem (ETAP 1)
| Polecenie | Wynik przed zmianami audytowymi |
|---|---|
| `git status --short --branch` | `## main...origin/main` (czyste drzewo robocze) |
| `git status --porcelain=v1` | *(pusty wynik — brak zmodyfikowanych i nieśledzonych plików)* |
| `git branch --show-current` | `main` |
| `git rev-parse HEAD` | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` |
| `git rev-parse --short HEAD` | `6f7d3aa` |
| `git log -1 --oneline --decorate` | `6f7d3aa (HEAD -> main, tag: v1.2.0, origin/main, origin/HEAD) docs(readme): update version 1.2.0 metadata, test counts, and module tree` |
| `git remote -v` | `origin https://github.com/sp6ina/SPLogbook.git (fetch / push)` |
| `git fetch --prune origin` | Kod wyjścia `0` |
| `git rev-parse refs/heads/main` | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` |
| `git rev-parse refs/remotes/origin/main` | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` |
| `git rev-list --left-right --count origin/main...main` | `0	0` |
| `git diff --check` / `git diff --cached --check` | Kod wyjścia `0` (brak błędów białych znaków) |
| `git submodule status` | Brak submodułów (kod wyjścia `0`) |
| `git tag --list` | `refactor-baseline`, `refactor-w0-done`..`refactor-w8-done`, `v1.0.0`, `v1.0.1`, `v1.0.2`, `v1.0.3`, `v1.0.4`, `v1.1.0`, `v1.2.0`, `v1.2.0-beta.1` |

---

## 3. Audyt Architektury i Jakości Kodu Rust (ETAP 3 & ETAP 4)

Szczegółowa mapa systemu znajduje się w [`docs/ARCHITECTURE_CURRENT_STATE.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/ARCHITECTURE_CURRENT_STATE.md), natomiast architektura docelowa w [`docs/ARCHITECTURE_TARGET_STATE.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/ARCHITECTURE_TARGET_STATE.md).

### 3.1. Ocena realizacji etapów W0–W8 oraz NATIVE-0..7
| Fala / Milestone | Deklarowany rezultat w planach | Rzeczywisty stan w kodzie na gałęzi `main` | Werdykt dla fali |
|---|---|---|---|
| **W0–W1 (Error Normalization & Panic Removal)** | Usunięcie `Result<_, String>` z domeny i `.unwrap()` ze ścieżek produkcyjnych. | Zweryfikowano: `tests/i18n_guard.rs` blokuje `Result<_, String>`, produkcyjne blokady używają `.unwrap_or_else(PoisonError::into_inner)`. | **ZREALIZOWANO POPRAWNIE** |
| **W2 (i18n & Theme System)** | 7 pełnych katalogów językowych `locales/*.json` i system motywów bez zahardkodowanych tekstów PL w GUI. | Zweryfikowano: 7 plików JSON posiada identyczne klucze i parametry (15 testów w `i18n_guard.rs` i 1 w `theme_guard.rs`). | **ZREALIZOWANO POPRAWNIE** |
| **W3 (Config & Secret Store Decomposition)** | Dekompozycja `AppConfig` na podstruktury i sejf sekretów `SecretStore` (Keyring + Argon2id/ChaCha20-Poly1305). | Podstruktury i sejf działają, lecz `#[derive(Debug)]` na `AppConfig` wypisuje sekrety jawnie (`SEC-002`), a `snapshot_config()` gubi część podstruktur przy zapisie (`FUNC-005`). | **CZĘŚCIOWO (wymaga poprawek `SEC-002`, `FUNC-005`)** |
| **W4 (Unified QSO Ingest Pipeline)** | Jednolity potok `QsoIngestor` dla wszystkich źródeł QSO. | `QsoIngestor::ingest` działa poprawnie, ale `import_batch` pomija `normalize_qso_fields` (`DATA-002`), a edycja QSO (`update_qso`) omija `QsoIngestor` (`ARCH-002`). | **CZĘŚCIOWO (wymaga poprawek `DATA-002`, `ARCH-002`)** |
| **W5 (SpLogApp State Decomposition)** | Podział monolitu `SpLogApp` na podstruktury `src/gui/state/*`. | Stan został wydzielony do `src/gui/state/`, ale widoki GUI nadal bezpośrednio blokują `db.lock()` i pomijają `journal_id` (`ARCH-001`, `DATA-005`). | **CZĘŚCIOWO** |
| **W6 (CAT Runtime, RigHub & TxArbiter)** | Wieloradiowy `RigHub`, `RigService` i `TxArbiter`. | Rdzeń `RigHub`/`TxArbiter` jest solidny, lecz brakuje auto-reconnectu w `RigService` (`CAT-001`), `resync_tx_arbiter` używa złego pola czasu (`CAT-006`), a `POST /api/v1/rig` i `HamlibProxyServer` omijają `TxArbiter` (`FUNC-003`, `CAT-008`). | **CZĘŚCIOWO** |
| **W7 (LAN Sync Host/Peer v2)** | Szyfrowana synchronizacja TCP z procedurą Catch-Up. | Kryptografia i sesje działają poprawnie, lecz `catchup.rs` ma błąd wydajnościowy $O(N^2)$ (`PERF-001`), a `host_handshake` nie ma timeoutu (`SEC-003`). | **CZĘŚCIOWO** |
| **W8 (Signed Auto-Updater)** | Podpisywany Ed25519 manifest aktualizacji i instalator. | Klucz prywatny został pozostawiony w `src/scratch.rs` (`SEC-001`), instalator zawodzi na Windows i Linux (`REL-001`), a CI nie generuje manifestu (`CI-001`). | **WYMAGA PILNEJ NAPRAWY (`SEC-001`, `REL-001`, `CI-001`)** |
| **NATIVE-0..7 (Natywne protokoły CAT i HIL)** | Kodeki i backendy Icom, Kenwood, Yaesu, Elecraft, FlexRadio + domena HIL. | Kodeki ramkowania są wysokiej jakości, ale występują błędy w `RigBackend::probe` (`CAT-003`), BCD S-metra/mocy Icom (`CAT-004`), wyścigach PTT (`CAT-002`, `CAT-005`) oraz `execute_controlled_ptt` (`CAT-015`). | **WYMAGA NAPRAWY PRZED HIL STAGE 7B** |

---

## 4. Audyt Współbieżności i Systemów Asynchronicznych (ETAP 5)

1. **Interakcja wątku GUI (`eframe`) z runtime'em Tokio**:
   - W `src/main.rs:41-47` tworzony jest wielowątkowy runtime Tokio, a wejście `_guard = rt.enter()` pozwala wywoływać `tokio::spawn` z dowolnej metody `SpLogApp`.
   - Komunikacja powrotna z zadań tła do wątku GUI odbywa się poprawnie przez nieblokujące wywołania `try_recv()` i `watch::Receiver::borrow()` w `SpLogApp::poll_async_events()` (`src/gui/app_async.rs:14-42`).
2. **Wykryte problemy współbieżności**:
   - **Blokowanie wątków roboczych Tokio przez synchroniczny SQLite (`CONC-001`)**: Podczas gdy `src/api/server.rs` poprawnie używa `tokio::task::spawn_blocking` (funkcja `with_db`), plik [`src/gui/online_sync.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L112-L491) wykonuje ciężkie zapytania SQLite (`get_recent_qsos(10000)` oraz pętlę pojedynczych `UPDATE` przy potwierdzeniach LoTW/eQSL) bezpośrednio wewnątrz bloków `async move` na wątkach roboczych Tokio.
   - **Blokowanie wątku GUI przez `std::thread::sleep` w `RigctldSupervisor` (`CAT-017`)**: Start i zatrzymanie procesu `rigctld` wykonuje synchroniczne uśpienia do 255 ms na głównym wątku renderowania.
   - **Brak timeoutu na `host_handshake` w `SyncHost` (`SEC-003`)**: Pozwala zablokować wszystkie 5 permitów semafora `handshake_sem`.

---

## 5. Kompletna Macierz Funkcjonalności Programu (ETAP 6)

Każda funkcja w repozytorium została zweryfikowana wzdłuż całego łańcucha: **wejście -> walidacja -> logika -> zapis/efekt -> odczyt -> prezentacja -> obsługa błędów -> test**:

| Obszar / Funkcja | Klasyfikacja stanu | Lokalizacja główna | Uzasadnienie i dowody |
|---|---|---|---|
| Ręczny zapis łączności (QSO Entry) | `implemented` / `tested` | `src/gui/qso_entry.rs`, `src/gui/app_qso.rs`, `src/core/ingest/mod.rs` | Pełny przepływ przez `QsoIngestor::ingest` -> SQLite -> `AwardsEngine` -> `EventBus`. |
| Edycja istniejącej łączności (`editing_qso_id`) | `partially wired` | `src/gui/app_qso.rs:116-146`, `src/api/server.rs:811-840` | Działa, lecz w GUI omija `normalize_qso_fields`/`validate()`, a w REST API nie przebudowuje `AwardsEngine` (`ARCH-002`). |
| Usuwanie zaznaczonych QSO + Undo (`Ctrl+Z`) | `partially wired` | `src/gui/app_qso.rs:530-640` | Pojedyncze usunięcie obsługuje Undo, lecz `delete_selected_qso` (`:564`) i `find_duplicates.rs` (`:297`) nie zapisują rekordów na `undo_stack` i nie pytają o potwierdzenie (`DATA-004`). |
| Wyszukiwanie duplikatów (`find_duplicates.rs`) | `broken` (multi-journal) | `src/gui/find_duplicates.rs`, `src/core/database/search.rs:27-81` | Brak filtra `journal_id` — łączy i usuwa łączności z różnych dzienników (`DATA-004`). |
| Zarządzanie wieloma dziennikami (`JournalProfile`) | `partially wired` | `src/core/database/journals.rs`, `src/gui/app_layout.rs:602-618` | Tworzenie i przełączanie tabeli działa, ale statystyki, chmura, QSL i `AwardsEngine` przy zmianie dziennika nie izolują `journal_id` (`DATA-005`). |
| Import i eksport ADIF 3.1.4 | `partially wired` | `src/core/adif.rs`, `src/gui/adif_dialog.rs` | Eksport działa kompletnie; import fabrykuje `Utc::now()` przy braku daty/czasu, psuje wielobajtowy UTF-8 przy długości znakowej (`DATA-001`) i pomija `normalize_qso_fields` w `import_batch` (`DATA-002`). |
| Eksport Cabrillo 3.0 (`cabrillo.rs`) | `broken` | `src/core/cabrillo.rs:27-88` | Odwraca kolejność QSO, wymusza stałe `599`/`59`, mapuje FT8/RTTY na `PH` i gubi `srx_string`/`stx_string` (`FUNC-001`). |
| Eksport EDI REG1TEST (VHF/UHF) i CSV | `implemented` / `tested` | `src/core/edi.rs`, `src/core/csv_export.rs` | Pełny przepływ eksportu z wyliczeniem QRB i punktów kilometrowych w EDI. |
| Silnik zawodów (`ContestEngine` i `contest.rs`) | `partially wired` | `src/core/contest.rs`, `src/core/contest_rules.rs`, `src/gui/contest.rs` | Błędy w regułach mnożników SP DX / CQ WW (`FUNC-002`), brak `Enter`/`Esc` w GUI (`GUI-001`) i 5 skanów $O(N)$ na klatkę (`PERF-002`). |
| Silnik dyplomowy (`AwardsEngine`: DXCC, WAZ, WAS, WAC, WPX, SPPA, PGA, SOTA, POTA, IOTA, Grids) | `implemented` / `tested` | `src/core/awards.rs`, `src/gui/awards_dialog.rs` | Działa dla 11 programów dyplomowych; kolekcje `worked_wwff` i `worked_rda` w `AwardsEngine` są martwe (`FUNC-007`). |
| Rozpoznawanie klubów (`ClubRegistry`) | `UI-only` (makieta) | `src/core/clubs.rs:24-84` | Zwraca zahardkodowane odznaki wyłącznie dla 11 wpisanych na sztywno znaków ze współdzielonymi numerami (`FUNC-006`). |
| Sterowanie CAT — Hamlib (`rigctld`) | `partially wired` / `tested` | `src/cat/hamlib.rs`, `src/cat/supervisor.rs` | Działa po TCP, lecz brak auto-reconnectu w `RigService` (`CAT-001`) i `auto_start_rigctld` z nowego panelu nie startuje `RigctldSupervisor` (`CAT-007`). |
| Sterowanie CAT — FLRig (XML-RPC) | `partially wired` | `src/cat/flrig.rs` | Brak HTTP Keep-Alive (20 gniazd TCP/s) i wyłączone możliwości Split/VFO/Power (`CAT-010`). |
| Sterowanie CAT — TCI (WebSocket) | `broken` (PTT) | `src/cat/tci.rs` | Odczyt częstotliwości/trybu działa, lecz `SetPtt(true)` ma wyścig potwierdzenia (okno 5 ms) blokujący `TxArbiter` w `Unknown` (`CAT-002`). |
| Sterowanie CAT — Natywny Icom CI-V | `partially wired` / `hardware-pending` | `src/cat/native/icom/` | Częstotliwość, tryb i PTT działają, lecz S-meter/moc mają odwrócone BCD (`CAT-004`), `SetRit`/`SetXit`/`SetSplitFrequency` wysyłają złe subkomendy (`CAT-011`), a `probe()` nie jest w `impl RigBackend` (`CAT-003`). |
| Sterowanie CAT — Natywny Kenwood | `partially wired` / `hardware-pending` | `src/cat/native/kenwood/` | Odczyt i `probe()` działają poprawnie, lecz settery czekają na echo `AI2;` (ryzyko timeoutu 1500 ms) i brak implementacji `SetPower`/`SetRit`/`SetXit` (`CAT-012`). |
| Sterowanie CAT — Natywny Yaesu CAT | `partially wired` / `hardware-pending` | `src/cat/native/yaesu/` | Odczyt/zapis częstotliwości i trybu działa, lecz `SetPtt` jest optymistyczne bez weryfikacji `TX;` (`CAT-005`), a `probe()` nie jest w `impl RigBackend` (`CAT-003`). |
| Sterowanie CAT — Natywny Elecraft | `partially wired` / `hardware-pending` | `src/cat/native/elecraft/` | Kodek K3/K4/KX działa, lecz `SetPtt` jest optymistyczne (`CAT-005`), a `probe()` nie jest w `impl RigBackend` (`CAT-003`). |
| Sterowanie CAT — Natywny FlexRadio SmartSDR | `broken` (PTT & Split) / `hardware-pending` | `src/cat/native/flexradio/` | Wyścig PTT z linią `interlock` (`CAT-002`), brak `wait_for_response` w setterach i pusta gałąź `SetSplit` (`CAT-013`). |
| Panel VFO w GUI (`vfo_panel.rs`) | `partially wired` | `src/gui/vfo_panel.rs` | Strojenie VFO A, zmiana pasma/trybu i PTT działają; przyciski Split `+1k`/`+5k`, Preamp/ATT/NB/NR/AGC i SWR `1.2` są atrapami (`CAT-007`). |
| Koordynacja SO2R (`src/cat/so2r.rs`) | `backend-only` | `src/cat/so2r.rs` | Przetestowana maszyna stanów w izolacji, niepodłączona do `RigHub` i GUI (`CAT-014`). |
| Kluczowanie CW / WinKeyer (`winkeyer.rs`, `cw_terminal.rs`) | `UI-only` (symulacja) | `src/cat/winkeyer.rs`, `src/gui/cw_terminal.rs`, `src/gui/app_plugins.rs:215` | Brak sterownika portu szeregowego; okno terminala i makra CW jedynie animują tekst na ekranie (`FUNC-004`). |
| Voice Keyer (`voice_keyer.rs`) | `partially wired` | `src/gui/voice_keyer.rs` | Odtwarza pliki WAV i wysyła `request_tx`, ale nie czeka na potwierdzenie `TxArbiterState::Owned` (`CAT-008`). |
| Nagrywanie i odtwarzanie audio QSO (`audio.rs`) | `partially wired` | `src/media/audio.rs`, `src/gui/qso_entry.rs:646` | Nagrywanie WAV działa, lecz przycisk `⏹ STOP` w `qso_entry.rs:646` wywołuje `stop_audio()` zamiast `stop_recording()` (`FUNC-004`). |
| Integracja WSJT-X / JTDX UDP (`:2237`) | `implemented` / `tested` | `src/digital/wsjtx.rs`, `src/gui/app_async.rs:78-193` | Pełny odbiór dekodowań, podświetlanie duplikatów/ATNO i automatyczne logowanie `QSO Logged` przez `QsoIngestor`. |
| Integracja JS8Call UDP (`:2242`) | `dead` (nieosiągalny) | `src/digital/js8call.rs`, `src/gui/app.rs:495` | `js8call_enabled` jest zawsze `false` w `start_background()` i nie jest zapisywane w konfiguracji (`FUNC-004`). |
| Integracja FLDigi XML-RPC (`:7362`) | `backend-only` | `src/digital/fldigi.rs`, `src/gui/online_sync.rs:543-580` | Metody `get_qso_data`/`get_frequency` nigdy nie są wywoływane, a przycisk testu gubi wynik (`FUNC-004`). |
| Klient Telnet DX Cluster (`telnet.rs`, `spot.rs`) | `implemented` / `tested` | `src/cluster/telnet.rs`, `src/gui/cluster_panel.rs` | Pełny odbiór spotów, filtrowanie, alerty pasmowe i klikalne QSY. |
| Synchronizacja chmurowa (LoTW, eQSL, QRZ, ClubLog, HRDLog, Cloudlog) | `partially wired` | `src/cloud/*`, `src/gui/online_sync.rs` | Komunikacja HTTP/ADIF działa, lecz pobiera dane z wszystkich dzienników naraz (`DATA-005`), blokuje wątki Tokio pętlą pojedynczych `UPDATE` (`CONC-001`) i może ujawnić hasło z URL przy błędzie sieci (`SEC-002`). |
| Synchronizacja LAN Multi-Op (`src/sync/*`) | `implemented` / `tested` | `src/sync/*`, `src/gui/multi_op_dialog.rs` | Szyfrowany transport i Catch-Up działają, ale `catchup.rs` ma złożoność $O(N^2)$ (`PERF-001`), `import_batch` pomija `normalize_qso_fields` (`DATA-002`), a `host_handshake` nie ma timeoutu (`SEC-003`). |
| Lokalny serwer REST API + SSE (`:8420`) | `partially wired` / `tested` | `src/api/server.rs` | 15 z 16 endpointów działa poprawnie; `POST /api/v1/rig` jest atrapą modyfikującą tylko pamięć RAM (`FUNC-003`). |
| Auto-Updater (`src/cloud/updater.rs`) | `broken` | `src/cloud/updater.rs`, `src/scratch.rs` | Skompromitowany klucz prywatny w `src/scratch.rs` (`SEC-001`), błąd `.zip`/katalogu w instalatorze (`REL-001`) i brak generowania manifestu w CI (`CI-001`). |

---

## 6. Wyniki Uruchomienia Narzędzi Walidacyjnych (ETAP 17)

| Narzędzie / Polecenie | Kod wyjścia | Czas | Wynik szczegółowy |
|---|---:|---:|---|
| `cargo +stable fmt --all -- --check` | **0** | 1,89 s | Formatowanie kodu w 100% zgodne ze standardem `rustfmt`. |
| `cargo +stable check --workspace --all-targets` | **0** | 14,10 s | Kompilacja czysta; 6 ostrzeżeń leksera `multiple lines skipped by escaped newline` w `src/core/database/qso_store.rs`. |
| `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | **101** | 29,68 s | **340 ostrzeżeń w `lib`**, **1 w `bin`**, **417 w `lib test`** (`RUST-001`). |
| `cargo +stable test --workspace --all-targets` | **0** | ~566,6 s | **1 519 passed, 0 failed, 10 ignored** (983 unit + 536 integration). |
| `cargo +stable test --doc` | **0** | 6,91 s | 0 doctestów. |
| `cargo audit` | **1** | 3,12 s | **7 podatności (`RUSTSEC-2026-*`)** w `rustls`, `rustls-webpki`, `webpki-root-certs`, `ctutils` + **9 ostrzeżeń** (w tym `tokio 1.53.1` unsound `RUSTSEC-2026-0386`) (`DEP-001`). |
| `cargo tree -d` | **0** | 1,45 s | Zduplikowane wersje przejściowe: `base64` (0.22/0.23), `bitflags` (1.3/2.13), `thiserror` (1.0/2.0), `windows-sys` (0.52/0.60/0.61), `rand_core` (0.6/0.10). |

---

## 7. Samokontrola Audytu (ETAP 23)

Przed sformułowaniem wniosków końcowych przeprowadzono drugą weryfikację każdego znaleziska o poziomie `BLOCKER`, `CRITICAL` i `HIGH`:
1. Wszystkie ścieżki plików i numery linii zostały sprawdzone bezpośrednio w drzewie roboczym na commicie `6f7d3aa`.
2. Rozróżniono moduły rzeczywiście aktywne (np. `ClubRegistry::check` w `src/core/clubs.rs`, `PropagationHistory::record_*` w `src/core/propagation_history.rs`) od modułów nieużywanych (`src/core/qsl_print.rs`, `src/cat/so2r.rs`, `WinkeyerProtocol` w `src/cat/winkeyer.rs`).
3. Potwierdzono, że żadne istniejące pliki kodu źródłowego `src/**` nie zostały zmodyfikowane podczas audytu — jedynymi zmianami w repozytorium są wygenerowane dokumenty audytowe w katalogu `docs/`.

---

## 8. Końcowy Werdykt Gotowości Programu

| Obszar oceny | Werdykt cząstkowy | Uzasadnienie |
|---|---|---|
| **Podstawowe logowanie QSO, baza SQLite, WSJT-X UDP, DX Cluster, i18n, motywy** | **READY (Gotowy)** | Stabilny potok `QsoIngestor::ingest`, pełna lokalizacja 7 języków, 1 519 przechodzących testów. |
| **Bezpieczeństwo wydawnicze i Auto-Updater** | **NOT READY (Bloker `SEC-001`, `REL-001`, `CI-001`)** | Publiczny klucz prywatny Ed25519 w `src/scratch.rs`, niedziałający instalator na Windows/Linux i brak podpisywania manifestu w GitHub Actions. |
| **Praca wielodziennikowa (`JournalProfile`) i operacje usuwania** | **CONDITIONAL (`DATA-004`, `DATA-005`)** | Wymaga dodania filtra `journal_id` do duplikatów/statystyk/chmury oraz obsługi `undo_stack` w `delete_selected_qso`. |
| **Obsługa zawodów i eksport Cabrillo 3.0** | **NOT READY (`FUNC-001`, `FUNC-002`, `GUI-001`)** | Wymaga naprawy formatowania Cabrillo 3.0, mnożników SP DX / CQ WW oraz obsługi `Enter`/`Esc` i skrótów `Ctrl+...`. |
| **Podsystem CAT i walidacja sprzętowa HIL Stage 7B** | **NOT READY FOR HARDWARE (`CAT-001`..`CAT-006`, `CAT-015`)** | Wymaga naprawy polimorficznego `RigBackend::probe`, Big-Endian BCD w Icom CI-V, potwierdzeń PTT w TCI/FlexRadio/Yaesu/Elecraft, auto-reconnectu w `RigService` oraz `execute_controlled_ptt` przed podłączeniem fizycznych transceiverów. |
| **WERDYKT CAŁOŚCIOWY DLA GAŁĘZI `main` (`v1.2.0`)** | **WARUNKOWY / NOT READY DO WYDANIA BEZ ZAMKNIĘCIA ETAPÓW 1–6 PLANU NAPRAWCZEGO** | Projekt posiada bardzo silne fundamenty architektoniczne i domenowe, lecz przed publikacją kolejnego wydania produkcyjnego i rozpoczęciem testów sprzętowych HIL Stage 7B konieczne jest wdrożenie poprawek Kategorii A zgodnie z [`docs/IMPROVEMENT_ROADMAP.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/IMPROVEMENT_ROADMAP.md). |
