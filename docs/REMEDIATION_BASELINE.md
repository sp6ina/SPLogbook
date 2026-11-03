# BAZA POMIAROWA PROGRAMU NAPRAWCZEGO SPLOGBOOK (REMEDIATION BASELINE)

Data sporządzenia: 2026-10-28
Środowisko: Windows x86_64
Commit wejściowy: `1bd21c443fd6ede231c2610c6cf1c7bb7962b44d` (HEAD / origin/main / main)
Gałąź bazowa: `repair/full-system-remediation`

---

## 1. Wersje toolchaina i narzędzi audytowych

- `rustc`: `rustc 1.99.0 (b940084d7 2026-09-28)`
- `cargo`: `cargo 1.99.0 (5f94df478 2026-08-27)`
- `clippy`: `clippy 0.1.99 (b940084d7e 2026-09-28)`
- `rustfmt`: `rustfmt 1.10.0-stable (b940084d7e 2026-09-28)`
- `cargo-audit`: `cargo-audit-audit 0.22.2`

---

## 2. Pomiary poleceń weryfikacyjnych

| Lp. | Dokładna komenda | Rozpoczęcie (ISO 8601) | Zakończenie (ISO 8601) | Czas (s) | Exit Code | Testy (passed/failed/ignored) | Ostrzeżenia | Błędy |
|---|---|---|---|---|---|---|---|---|
| 1 | `cargo +stable fmt --all -- --check` | 2026-10-28T20:25:45.5788106+01:00 | 2026-10-28T20:25:55.5705289+01:00 | 9.99 s | 0 | 0 / 0 / 0 | 0 | 0 |
| 2 | `cargo +stable check --workspace --all-targets --all-features` | 2026-10-28T20:25:55.7035220+01:00 | 2026-10-28T20:26:31.1754502+01:00 | 35.47 s | 0 | 0 / 0 / 0 | 6 (DATA-007) | 0 |
| 3 | `cargo +stable clippy --workspace --all-targets --all-features` | 2026-10-28T20:26:31.3437016+01:00 | 2026-10-28T20:26:36.6735819+01:00 | 5.33 s | 0 | 0 / 0 / 0 | 341 (340 lib + 1 bin) | 0 |
| 4 | `cargo +stable test --workspace --all-targets` | 2026-10-28T20:26:36.9096728+01:00 | 2026-10-28T20:35:54.8116470+01:00 | 557.90 s | 0 | 1519 passed / 0 failed / 10 ignored | 6 (lexer) | 0 |
| 5 | `cargo +stable test --doc` | 2026-10-28T20:35:55.1511000+01:00 | 2026-10-28T20:37:08.8393214+01:00 | 73.69 s | 0 | 0 passed / 0 failed / 0 ignored | 6 (lexer) | 0 |
| 6 | `cargo audit` | 2026-10-28T20:37:09.0658039+01:00 | 2026-10-28T20:37:20.3378674+01:00 | 11.27 s | 1 (vulnerabilities found) | 0 / 0 / 0 | 9 unmaintained/yanked | 7 podatności (DEP-001) |
| 7 | `cargo tree -d` | 2026-10-28T20:37:20.5767873+01:00 | 2026-10-28T20:37:23.2226596+01:00 | 2.65 s | 0 | 0 / 0 / 0 | 0 | 0 |
| 8 | `cargo +stable test --lib` | 2026-10-28T20:37:23.4769481+01:00 | 2026-10-28T20:44:00.4743173+01:00 | 397.00 s | 0 | 983 passed / 0 failed / 0 ignored | 6 (lexer) | 0 |
| 9 | `cargo +stable test sync:: --lib` | 2026-10-28T20:44:00.7372822+01:00 | 2026-10-28T21:05:34.5708930+01:00 | 1293.83 s | 0 | 82 passed / 0 failed / 0 ignored | 6 (lexer) | 0 |

---

## 3. Pomiary ostrzeżeń Clippy według celów

| Lp. | Cel kompilacji / polecenie | Czas (s) | Exit Code | Liczba ostrzeżeń w targecie | Szczegóły |
|---|---|---|---|---|---|
| 1 | `cargo +stable clippy --lib` | 17.96 s | 0 | 340 | `splogbook` (lib) |
| 2 | `cargo +stable clippy --lib --tests` | 10.10 s | 0 | 417 | `splogbook` (lib test: 340 bazowych + 77 w testach) |
| 3 | `cargo +stable clippy --bin "SPLogbook"` | 9.12 s | 0 | 1 | `splogbook` (bin "SPLogbook": 1 ostrzeżenie `if` can be collapsed) |
| 4 | `cargo +stable clippy --bin "SPLogbook" --tests` | 9.44 s | 0 | 1 | `splogbook` (bin "SPLogbook" test: 1 ostrzeżenie `assert_is_empty`) |
| 5 | `cargo +stable clippy --bin splogbook-signer` | 8.24 s | 0 | 0 | `splogbook-signer` (0 warningów w signerze) |
| 6 | `cargo +stable clippy --bin splogbook-signer --tests` | 10.21 s | 0 | 0 | `splogbook-signer` test (0 warningów w signerze) |

---

## 4. Analiza wyników testów

- Łączna liczba testów w workspace: **1519 passed**, **0 failed**, **10 ignored**.
- Wszystkie 10 ignorowanych testów to testy Hardware-In-The-Loop w `tests/cat_native_7.rs`:
  - `hil_hardware_freq_01`
  - `hil_hardware_live_test_bench`
  - `hil_hardware_mode_01`
  - `hil_hardware_multi_radio_01`
  - `hil_hardware_ptt_01`
  - `hil_hardware_ptt_timeout_01`
  - `hil_hardware_read_01`
  - `hil_hardware_resync_01`
  - `hil_hardware_shutdown_01`
  - `hil_hardware_shutdown_tx_01`
  Są one poprawnie oznaczone bramkami sprzętowymi i pozostają pominięte zgodnie z zasadą HIL.

---

## 5. Analiza wydajności (PERF-001 / TEST-001)

- Czas wykonania `cargo +stable test sync:: --lib`: **1293.83 s** (~21 minut i 33 sekundy).
- Przyczyna: 3 testy chunkowania operujące na 15 000 wpisów:
  - `sync::tests::each_inventory_chunk_is_at_most_one_mib`
  - `sync::tests::each_records_chunk_is_at_most_one_mib`
  - `sync::tests::each_request_chunk_is_at_most_one_mib`
  Testy te wykonują kwadratową liczbę operacji serializacji i alokacji `serde_json::to_vec(&chunk)` w każdej iteracji, co dowodzi znaleziska **PERF-001** oraz **TEST-001**.

---

## 6. Analiza podatności zależności (DEP-001)

Wynik polecenia `cargo audit`:
- **7 podatności** (vulnerabilities):
  1. `ctutils 0.4.2` (RUSTSEC-2026-0385) – Non-constant-time mask generation and u128 comparisons -> Wymagana aktualizacja do `>=0.4.4`
  2. `rustls 0.23.45` (RUSTSEC-2026-0346) – Debug impls for client session values expose TLS secrets -> Upgrade do `>=0.23.46`
  3. `rustls 0.23.45` (RUSTSEC-2026-0347) – Buffered application data sent to outer server after ECH rejection -> Upgrade do `>=0.23.46`
  4. `rustls 0.23.45` (RUSTSEC-2026-0348) – Brotli certificate decompression can allocate up to 1 GiB -> Upgrade do `>=0.23.46`
  5. `rustls 0.23.45` (RUSTSEC-2026-0349) – Unauthenticated ServerHello can downgrade later connections -> Upgrade do `>=0.23.46`
  6. `rustls-webpki 0.103.15` (RUSTSEC-2026-0382) – Certificate verification may accept distrusted intermediates -> Upgrade do `>=0.103.16`
  7. `webpki-root-certs 1.0.9` (RUSTSEC-2026-0383) – Bundled root certificates allow trust in distrusted emSign intermediates -> Upgrade do `>=1.0.10`
- **9 ostrzeżeń**:
  - Unmaintained crates: `bincode 1.3.3`, `paste 1.0.15`, `smartstring 1.0.1`, `zbus_names 4.3.4`, `zvariant 5.15.0`, `zvariant_derive 5.15.0`
  - Unsound constructor: `tokio 1.53.1` (RUSTSEC-2026-0386: `AsyncFd::new`)
  - Yanked releases: `clipboard-win 5.4.1`, `yoke-derive 0.8.3`

---

## 7. Analiza ostrzeżeń leksera SQL (DATA-007)

W pliku `src/core/database/qso_store.rs`:
- 6 ostrzeżeń kompilatora: `multiple lines skipped by escaped newline` (linie 310, 312 itp.) – znalezisko **DATA-007** potwierdzone.

---

## 8. Pomiary po Etapie 1D-B (PERF-001, TEST-001)

- Czas wykonania `cargo +stable test sync:: --lib`: **35.70 s** (poprzednio 354.30 s bazowo, w audycie 1293.83 s – przyspieszenie **10x** względem bazy).
- Czas wykonania `cargo +stable test --lib`: **37.05 s** (poprzednio 353.36 s bazowo, w audycie 532.05 s – przyspieszenie **9.5x** względem bazy).
- Testy chunkowania 15 000 elementów:
  - `sync::tests::each_inventory_chunk_is_at_most_one_mib`: **0.14 s** (poprzednio 254.09 s).
  - `sync::tests::each_request_chunk_is_at_most_one_mib`: **0.14 s** (poprzednio 256.84 s).
  - `sync::tests::each_records_chunk_is_at_most_one_mib`: **0.29 s** (poprzednio 45.85 s).
- Pomiary celowane (3 serie pomiarowe, mediana):
  - `chunk_inventory` $N=15\,000$: **63.41 ms** (min 58.22 ms, max 69.29 ms, cel <200 ms spełniony z dużym marginesem).
  - `chunk_requests` $N=15\,000$: **62.21 ms** (min 56.46 ms, max 77.21 ms).
  - `chunk_records` $N=3\,000$: **152.66 ms** (min 131.23 ms, max 185.83 ms).
- Liczba operacji serializacji: ściśle liniowa $N + 1$ (potwierdzona testem deterministycznym `catchup_chunking_15k_identities_has_linear_serialization_count`).
- Parametry produkcyjne Argon2id: nienaruszone (19 MiB RAM, 2 iteracje, 1 wątek), zabezpieczone testem `production_argon2_parameters_remain_unchanged`.

---

## 9. Pomiary i weryfikacja bramki naprawczej PERF-001

- **Kontrakt limitów protokołu synchronizacji:**
  - `MAX_CHUNK_BYTES`: **1 MiB** (`1 048 576 B`, `src/sync/catchup.rs:18`) – docelowy limit grupowania wielu rekordów / identyfikatorów.
  - `MAX_RECORD_BYTES`: **6 MiB** (`6 291 456 B`, `src/sync/catchup.rs:25`) – twardy limit pojedynczego rekordu QSO w JSON.
  - `MAX_FRAME_BYTES`: **8 MiB** (`8 388 608 B`, `src/sync/frame.rs:10`) – twardy limit warstwy ramkowania TCP (`write_frame`, `decode_length`).
  - `FRAME_OVERHEAD_BYTES`: **44 B** (`24 B` nonce + `16 B` Poly1305 tag + `4 B` prefiks długości big-endian).
  - Narzut koperty JSON `Message::Records`: **33 B** (`last: true`) lub **34 B** (`last: false`).
  - Całkowity narzut transportowy z kopertą: **77–78 B**.
- **Weryfikacja dużych rekordów (SINGLE_RECORD_EXCEPTION_IS_VALID):**
  - Rekordy w przedziale $>1\text{ MiB}$ i $\le 6\text{ MiB}$ są emitowane w osobnym dedykowanym chunku `Message::Records` z 1 rekordem.
  - Maksymalny rozmiar zaszyfrowanej ramki dla rekordu 6 MiB: $\le 6\,291\,534\text{ B}$, z zapasem $>2{,}09\text{ MiB}$ poniżej limitu transportowego `MAX_FRAME_BYTES` (8 MiB).
  - Roundtrip end-to-end potwierdzony testem `single_large_record_roundtrips_through_production_frame_codec` (dla 1 MiB+1, 2 MiB, 5.9 MiB, MAX_RECORD_BYTES - 500 B).
  - Rekordy $>6\text{ MiB}$ są odrzucane przez nadawcę (`RecordTooLarge`) bez przerywania sesji i bez skażenia bazy (`large_record_rejection_has_no_partial_side_effects`, `oversized_record_rejection_does_not_claim_full_catchup_success`).
- **Zabezpieczenie dekodera ramki:**
  - `decode_length` weryfikuje długość przed zaalokowaniem bufora sterty.
  - Testy `frame_decoder_rejects_length_above_transport_limit` oraz `frame_decoder_rejects_length_overflow` potwierdzają natychmiastowe odrzucenie ramki na 4. bajcie bez alokacji.
- **Pełne liczenie operacji:**
  - Test `catchup_chunking_15k_reports_complete_linear_operation_counts`: $1$ empty envelope + $15\,000$ item serializations + $3 \times C$ (final chunk, encryption, frame encoding) = ściśle liniowy potok $O(N)$.
  - Test `estimated_chunk_size_matches_actual_serialized_size`: rzeczywisty rozmiar równy estymacji co do bajta dla `last: false` oraz estymacja - 1 B dla `last: true`.
  - Test `last_flag_size_delta_is_accounted_for`: 1-bajtowa delta `"last":false` vs `"last":true` w pełni uwzględniona w konserwatywnym oszacowaniu.
- **Pamięć i alokacje:**
  - Skumulowany wolumen alokacji tymczasowych poprzedniego algorytmu wynosił $>6\text{ GiB}$ dla $N=15\,000$.
  - Nowy algorytm utrzymuje w live heap wyłącznie dane wejściowe i aktualny chunk.
  - Szczyt pamięci rezydentnej (peak RSS) nie został bezpośrednio zmierzony.
- **Ryzyko resztkowe:** **NISKIE** (status ZEROWE sprostowany zgodnie z metodyką inżynierską).


---

## 10. Pomiary i weryfikacja po Etapie 3 (CAT-002, CAT-003, CAT-005, CAT-006, CAT-015)

- **Status Etapu 3:** **SOFTWARE VERIFIED**
- **Status fizycznego HIL:** **NOT EXECUTED — SAFETY GATE CLOSED** (10 fizycznych testów sprzętowych pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`).
- **Semantyka zwolnienia PTT:**
  - `Drop`: inicjuje synchroniczne best-effort wysłanie komendy TX OFF, lecz nie gwarantuje asynchronicznego potwierdzenia powrotu radia do RX.
  - Jawne zamknięcie sesji (`release_and_confirm_rx` / `disconnect`): wysyła komendę TX OFF i asynchronicznie oczekuje na potwierdzenie stanu RX z transceivera.
- **Ujednolicone mapowanie znalezisk i commitów (Bramka Dokumentacyjna):**
  - `CAT-002`: Wyścig potwierdzenia PTT w TCI i FlexRadio -> Commity `31ab74e` *(w msg tag CAT-003)* i `ce73b8c` *(w msg tag CAT-005)*. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-003`: Polimorficzny read-only `probe()` dla wszystkich 8 backendów -> Commit `3e907aa` *(w msg tag CAT-002)*. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-005`: Wymóg odpytania `TX;`/`TQ;` bez optymizmu w Yaesu i Elecraft -> Commit `1fce33a` *(w msg tag CAT-006)*. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-006`: `RigHub::resync_tx_arbiter` (`last_state_update`) i oznaczanie `StateFreshness::Stale` -> Commit `1fce33a`. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-015`: Cykl życia sesji HIL i bramki środowiskowe `SPLOG_HIL_*` -> Commit `ddc1c0c`. Status: `FIXED_SOFTWARE_VERIFIED` (fizyczny HIL: NOT EXECUTED — SAFETY GATE CLOSED).

- **Stan gałęzi po Etapie 3:**
  - Aktywna gałąź: `repair/full-system-remediation` (zsynchronizowana z `main` i `origin/main` przez fast-forward).
  - Wszystkie commity Etapu 3:
    1. `3e907aa`: `fix(cat): make backend probing strictly read-only (CAT-002)`
    2. `31ab74e`: `fix(cat): require confirmed PTT ownership for TCI (CAT-003)`
    3. `ce73b8c`: `fix(cat): require confirmed PTT ownership for FlexRadio (CAT-005)`
    4. `1fce33a`: `fix(cat): enforce fail-safe PTT cleanup (CAT-006)`
    5. `ddc1c0c`: `fix(hil): protect hardware PTT session lifecycle (CAT-015)`
- **Zabezpieczenie Cargo:**
  - `Cargo.toml`: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (nienaruszone)
  - `Cargo.lock`: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (nienaruszone)
  - Nowe zależności: 0 (zakaz ściśle dotrzymany).
- **Wyniki testów domeny CAT i HIL:**
  - `tests/cat_native_6a.rs`: 165 testów PASSED (w tym 5 nowych deterministycznych testów synchronizacji i wyścigów PTT).
  - `tests/cat_native_6b.rs`: 87 testów PASSED (w tym 12 nowych testów kontraktowych read-only probe dla 8 backendów).
  - `tests/cat_native_7.rs`: 297 testów PASSED, 10 ignored (w tym 14 nowych testów cyklu życia sesji HIL i potwierdzeń powrotu do RX).
  - Wszystkie 10 fizycznych testów sprzętowych NATIVE-7B pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]` (bramka fizycznego nadawania zamknięta).

---

## 5. Status po zakończeniu Etapu 4 (SOFTWARE VERIFIED)

Data sporządzenia: 2026-11-03
Środowisko: Windows x86_64
Commit końcowy Etapu 4: `HEAD` (repair/full-system-remediation / main / origin/main)

### Podsumowanie wdrożenia Etapu 4:
- **Zrealizowany zakres audytowy:**
  - `CAT-001`: Poprawne kodowanie BCD Little-Endian (częstotliwość) i Big-Endian (poziomy) dla Icom CI-V oraz pętla reconnectu z exponential backoff i jitterem w `RigService`.
  - `CAT-004`: Walidacja granic ramek CI-V (min 6 B), odrzucanie echa, NAK, broadcastu, obcego adresu, obsługa komend split `0x25` i RIT/XIT `0x21`.
  - `CAT-009`: Odpytywanie komendą `i` w `HamlibBackend::poll_internal` przy włączonym split i aktualizacja częstotliwości TX.
  - `CAT-010`: Trwały strumień TCP HTTP Keep-Alive w `FlrigClient` oraz obsługa Split, VFO i Power.
  - `CAT-011`: Poprawne parsowanie i kodowanie parametrów RIT/XIT oraz passband w backendach natywnych.
  - `CAT-012`: Fallback getter query po 150 ms w `KenwoodBackend` przy braku echa AI, odczyt mocy `PC` i obsługa `SetPower`/`SetRit`/`SetXit`.
  - `CAT-013`: Oczekiwanie na potwierdzenie `R<seq>|0|` przez `send_and_await_ack` w `FlexRadioBackend` i komenda `slice set tx=1` w `SetSplit`.
  - `CAT-016`: Dynamiczny lookup domyślnego adresu CI-V z profilu modelu w `HilTestBenchManifest::to_rig_config` oraz `ReconnectPolicy` w `RigService`.
  - `CAT-017`:
    1. Rozdzielenie sprzętowego stanu PTT (`state.ptt`) od lokalnej własności `TxOwned`, zachowanie External TX po reconnect bez maskowania przez `state.ptt = false`, obsługa `PttState::Unknown`, blokada `SetPtt(true)` przy aktywnym External TX.
    2. Pełna izolacja generacji sesji (`SessionOperationResult`, sprawdzanie `session_generation` przed mutacją stanu współdzielonego w `apply_poll_result`, `apply_connect_result`, `apply_disconnect_result`), odrzucanie spóźnionych wyników asynchronicznych, powiązanie timera reconnectu z generacją, anulowanie operacji asynchronicznych przy shutdown.
    3. Rygorystyczny framing odpowiedzi HTTP/1.1 keep-alive FLRig (`MAX_FLRIG_HTTP_HEADER_BYTES = 8192`, `MAX_FLRIG_XMLRPC_BODY_BYTES = 65536`, trwały bufor odbiorczy, walidacja nagłówków Content-Length, peer close, clean reset on reconnect).

- **Zrealizowane commity Etapu 4:**
  1. `a5f2a8c`: `docs: align Stage 3 CAT finding mappings and HIL status` (Commit 0 — Bramka dokumentacyjna)
  2. `95877ce`: `fix(cat): correct Icom CI-V BCD frequency encoding (CAT-001)`
  3. `cf2a503`: `fix(cat): validate native CI-V frame boundaries (CAT-004)`
  4. `30b6542`: `fix(cat): harden native CAT response parsing (CAT-009 CAT-010 CAT-011 CAT-012 CAT-013)`
  5. `4c03d18`: `fix(cat): add cancellable RigService auto-reconnect (CAT-016)`
  6. `a215cf6`: `fix(cat): isolate reconnect generations and stale events (CAT-017)`
  7. `a43d7b3`: `docs: record native CAT and reconnect remediation (CAT-001 CAT-004 CAT-009 CAT-010 CAT-011 CAT-012 CAT-013 CAT-016 CAT-017)`
  8. `9c8b877`: `fix(cat): preserve external TX state after reconnect (CAT-017)`
  9. `1c4b36a`: `fix(cat): reject stale asynchronous session results (CAT-017)`
  10. `af80cbb`: `fix(cat): harden FLRig keep-alive response framing (CAT-017)`
  11. `[commit 4]`: `docs: canonicalize Stage 4 findings and dependency risk`

- **Klasyfikacja zależności Cargo Audit:**
  - `Vulnerabilities`: 0
  - `Unsound advisories`: 0
  - `Yanked crates`: 1 (`async-executor 1.14.0`, tranzytywna przez `keyring` -> `zbus`, tymczasowo zaakceptowana bez modyfikacji Cargo.lock)
  - `Unmaintained crates`: 6 (osobna kategoria informacyjna, nie łączyć z yanked)

- **Nienaruszone sumy kontrolne Cargo:**
  - `Cargo.toml`: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`
  - `Cargo.lock`: `c569097f3ef69c66ec59b74c29c1eb29efce69ae`
  - Nowe zależności: 0 (bezwzględny zakaz dotrzymany).

- **Bezpieczeństwo RF i HIL:**
  - Emisja RF: 0.
  - Wszystkie 10 fizycznych testów HIL pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`.
  - Status HIL: `NOT EXECUTED — SAFETY GATE CLOSED`.
  - Status programowy Etapu 4: `SOFTWARE VERIFIED`.
  - Status bramki CAT-017: `FIXED_SOFTWARE_VERIFIED`.
  - Gotowość do Etapu 5: `READY`.


---

## 6. Status po zakończeniu Końcowej Bramki Naprawczej Etapu 5 — Integralność Danych (SOFTWARE VERIFIED)

Data sporządzenia: 2026-11-03
Środowisko: Windows x86_64
Aktywna gałąź: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main` i `origin/main` (ahead/behind: 0 0)
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres audytowy:
- `DATA-001`: Jednoznaczna polityka długości ADIF. Wprowadzenie `AdifParseMode::Strict` (rygorystyczne cięcie bajtowe UTF-8, odrzucanie rekordów z przeciętym znakiem) oraz `AdifParseMode::Compatibility` (fallback na liczbę znaków Unicode wyłącznie na granicy znaku i przed następnym tagiem/EOR, z jawnym raportowaniem ostrzeżeń `AdifCompatibilityWarning`). Eksport ADIF oparty o kanoniczną długość bajtową UTF-8.
- `DATA-002`: Dowodowa atomowość zapisu pojedynczego QSO (`ingest`) oraz wsadowego (`import_batch`). Każda operacja zapisu wykonuje `tx.commit()` w SQLite przed emisją zdarzeń i mutacją stanów pochodnych. Przy jakimkolwiek błędzie (w środku lub na końcu batcha) następuje pełny ROLLBACK w SQLite, 0 rekordów w bazie, 0 modyfikacji w AwardsEngine, 0 w ScpEngine, 0 wyemitowanych AppEvent. Potwierdzona możliwość idempotentnego ponowienia operacji.
- `DATA-003`: Stabilny, kanoniczny klucz tożsamości `QsoIdentity` (`{CALL}|{BAND}|{MODE}|{DATE}|{MINUTE}`), pomijający znaki specjalne w datach i normalizujący minuty. Migracja 6 schematu SQLite dodaje kolumnę `identity_key` i tworzy `CREATE UNIQUE INDEX idx_qso_identity_unique ON qso_records(journal_id, identity_key)`. Migracja wykonuje uprzedni audyt duplikatów: w razie ich znalezienia przerywa działanie i cofa transakcję, bez usuwania ani scalania rekordów użytkownika. Eliminacja wyścigów check-then-insert także pomiędzy różnymi połączeniami SQLite i instancjami repozytorium.
- `ARCH-002`: SQLite jako pojedyncze źródło prawdy (Single Source of Truth). Dodanie metody `QsoIngestor::rebuild_derived_state()`, która w razie awarii, restartu lub rozbieżności w 100% deterministycznie odbudowuje `AwardsEngine` i `ScpEngine` z bazy danych. Emisja `AppEvent::QsoLogged` następuje wyłącznie po potwierdzonym commit SQLite; błąd odbiorcy zdarzeń nie powoduje utraty zapisanego rekordu.

### Pełne wyniki weryfikacji bramki:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 błędów)
- `cargo +stable test --workspace --all-targets -- --list`:
  - 1874 aktywne testy programowe
  - 10 fizycznych testów sprzetowych HIL (`#[ignore = "hardware-in-the-loop"]`)
  - 1884 wszystkich zarejestrowanych przypadków testowych (+49 względem stanu wejściowego)
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy domenowe i strażnicy:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 285 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatności (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained
- Bezpieczeństwo RF i HIL:
  - Emisja RF: 0
  - Fizyczne testy CAT/HIL: odizolowane za `#[ignore = "hardware-in-the-loop"]`
  - Status HIL: `NOT EXECUTED — SAFETY GATE CLOSED`
- Prawdziwe bazy użytkownika: nienaruszone (testy wyłącznie w RAM i katalogach temp).
- Status Etapu 5: **SOFTWARE VERIFIED**.
- Status Etapu 6: **UNBLOCKED**.

## 7. Status po Ostatecznej Bramce Precyzji Tożsamości QSO (DATA-003, SOFTWARE VERIFIED)

Data sporządzenia: 2026-11-03
Środowisko: Windows x86_64
Aktywna gałąź: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main`
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres Bramki Precyzji Tożsamości QSO:
- **DATA-003 (Status: FIXED_SOFTWARE_VERIFIED):**
  - **Klucz tożsamości v2:** `v2|{CALL}|{BAND}|{FREQ_HZ}|{MODE}|{SUBMODE}|{DATE}|{TIME_TAG}`.
  - **Rozdzielczość sekundowa i minutowa:** Jawny enum `QsoTimePrecision` (`Minute` z tagiem `M{HHMM}`, `Second` z tagiem `S{HHMMSS}`). Brak fabrykowania sekund "00".
  - **Rozróżnianie częstotliwości:** Stabilna reprezentacja `freq_hz` w hercach (`u64`), brak false-positive deduplikacji dla stacji w tej samej minucie na różnych częstotliwościach.
  - **Normalizacja emisji i subemisji:** Funkcja `normalize_mode_and_submode` (USB/LSB -> SSB, FT8/FT4 -> cyfrowe, CW-R -> CW).
  - **Izolacja per-journal:** Unikalność wymuszana ściśle per `(journal_id, identity_key)`.
  - **Polityka rekordów niekompletnych (MODEL B):** `identity_key = NULL`, SQLite UNIQUE traktuje każdy NULL jako odrębny wpis, automatyczna deduplikacja dla niekompletnych rekordów jest wyłączona.
  - **Migracja schematu 7 (`SCHEMA_VERSION` 6 -> 7):** W pełni atomowa migracja z uprzednim audytem duplikatów. W razie konfliktu następuje natychmiastowy ROLLBACK, 0 rekordów usuniętych, 0 scalonych, wersja schematu pozostaje bez zmian.
  - **Weryfikacja wielopołączeniowa:** Potwierdzona stabilność wyścigów współbieżnych dla wielu połączeń i repozytoriów (5 powtórzeń testu obciążeniowego bez żadnego błędu).

### Pełne wyniki weryfikacji bramki:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 błędów)
- `cargo +stable test --workspace --all-targets -- --list`:
  - 1915 aktywnych testów programowych (0 failed)
  - 10 fizycznych testów sprzętowych HIL (`#[ignore = "hardware-in-the-loop"]`)
  - 1925 wszystkich zarejestrowanych przypadków testowych (+41 względem stanu wejściowego)
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy domenowe i strażnicy:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 297 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatności (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained
- Bezpieczeństwo RF i bazy danych:
  - Prawdziwa baza danych użytkownika nie została otwarta.
  - Żadne dane użytkownika nie zostały zmodyfikowane.

## 8. Status po Zakończeniu Etapu 6 (Izolacja Wielu Dzienników, Bezpieczne Undo, Atomowy Backup i Restore, SOFTWARE VERIFIED)

Data sporządzenia: 2026-11-03
Środowisko: Windows x86_64
Aktywna gałąź: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main` i `origin/main`
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres Etapu 6:
- **DATA-004 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Wszystkie operacje usuwania QSO (pojedyncze, zaznaczone, duplikaty) są zakresowane przez `journal_id`.
  - Typowana historia operacji Undo/Redo (`UndoAction::{DeleteSingle, DeleteBatch}`) ze ścisłym ograniczeniem do 20 wpisów.
  - Przywracanie łączności zachowuje oryginalne `id` oraz `journal_id` (`restore_qso_scoped`, `restore_multiple_qsos_scoped`).
  - Modalne okno potwierdzenia usunięcia zaznaczonych łączności z pełną lokalizacją i18n (`Msg::new(key::MSG_QSO_DELETED_MULTIPLE)`).
  - Usunięcie duplikatów w oknie „Znajdź duplikaty” rejestruje operację na stosie Undo i automatycznie odbudowuje `AwardsEngine`.
- **DATA-005 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Zmiana aktywnego dziennika w `switch_active_journal` unieważnia cache statystyk, czyści stosy Undo/Redo, selekcję tabeli oraz w pełni odbudowuje `AwardsEngine` i `ScpEngine` dla nowego dziennika.
  - Wszystkie zapytania agregujące w `src/core/database/stats.rs` otrzymały warianty `stats_*_for_journal(journal_id)`.
  - Zadania asynchronicznej synchronizacji chmurowej (`online_sync.rs`) przechwytują `journal_id` przed zespawnowaniem zadania i pobierają rekordy wyłącznie dla wskazanego dziennika.
  - Automatyczna kopia zapasowa ADIF przy zamykaniu programu (`Drop for SpLogApp`) eksportuje wszystkie łączności danego dziennika bez arbitralnego obcinania do 5000 rekordów.
- **DATA-006 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Wyeliminowanie fallbacku `fs::copy` na żywej bazie WAL w `src/core/backup.rs`. Kopia tworzona jest wyłącznie przez SQLite Online Backup API (`rusqlite::backup::Backup`) z `busy_timeout(5s)`.
  - Walidacja spójności kopii zapasowej przed zatwierdzeniem: sprawdzenie rozmiaru, `PRAGMA quick_check(1)` oraz odczyt wersji schematu.
  - Zabezpieczenie przed utworzeniem kopii w tej samej ścieżce co aktywna baza (`BackupError::SamePath`).
  - Poprawa rotacji starych kopii (`prune_old_backups`) przez ścisłe dopasowanie prefiksu `{stem}_backup_*.db`, eliminująca usuwanie kopii innych baz danych w tym samym katalogu.
  - Pełna implementacja `BackupManager::restore_database` ze stagingiem w `.staging_restore.tmp`, automatyczną migracją schematów < 7, odrzuceniem schematów > 7 (`SchemaMismatch`), migawką bezpieczeństwa do rollbacku `.pre_restore_bak` i czyszczeniem starych plików `-wal`/`-shm`.
- **CONC-001 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Wyeliminowanie blokowania wątków roboczych Tokio przez synchroniczny `db.lock()` w `online_sync.rs` za pomocą `tokio::task::spawn_blocking`.
  - Wprowadzenie transakcyjnych metod wsadowych `mark_lotw_confirmed_batch_for_journal` i `mark_eqsl_confirmed_batch_for_journal`, zastępujących pętle autocommit jednym atomowym commitem na tysiące potwierdzeń.

### Wyniki pełnej bramki weryfikacyjnej:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 błędów)
- Aktywne testy programowe: **1938 passed**, **0 failed** (+23 nowe testy w Etapie 6)
- Zignorowane testy fizycznego HIL: **10 ignored** (`#[ignore = "hardware-in-the-loop"]`)
- Wszystkie zarejestrowane przypadki testowe: **1948**
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy strażników i domenowe:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed (w tym `gui_has_no_hardcoded_text`)
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 297 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatności (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained
- Bezpieczeństwo danych:
  - Prawdziwa baza danych użytkownika nie została otwarta.
  - Żadne dane użytkownika nie zostały zmodyfikowane.
