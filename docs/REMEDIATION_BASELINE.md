# BAZA POMIAROWA PROGRAMU NAPRAWCZEGO SPLOGBOOK (REMEDIATION BASELINE)

Data sporzÄ…dzenia: 2026-10-28
Ĺšrodowisko: Windows x86_64
Commit wejĹ›ciowy: `1bd21c443fd6ede231c2610c6cf1c7bb7962b44d` (HEAD / origin/main / main)
GaĹ‚Ä…Ĺş bazowa: `repair/full-system-remediation`

---

## 1. Wersje toolchaina i narzÄ™dzi audytowych

- `rustc`: `rustc 1.99.0 (b940084d7 2026-09-28)`
- `cargo`: `cargo 1.99.0 (5f94df478 2026-08-27)`
- `clippy`: `clippy 0.1.99 (b940084d7e 2026-09-28)`
- `rustfmt`: `rustfmt 1.10.0-stable (b940084d7e 2026-09-28)`
- `cargo-audit`: `cargo-audit-audit 0.22.2`

---

## 2. Pomiary poleceĹ„ weryfikacyjnych

| Lp. | DokĹ‚adna komenda | RozpoczÄ™cie (ISO 8601) | ZakoĹ„czenie (ISO 8601) | Czas (s) | Exit Code | Testy (passed/failed/ignored) | OstrzeĹĽenia | BĹ‚Ä™dy |
|---|---|---|---|---|---|---|---|---|
| 1 | `cargo +stable fmt --all -- --check` | 2026-10-28T20:25:45.5788106+01:00 | 2026-10-28T20:25:55.5705289+01:00 | 9.99 s | 0 | 0 / 0 / 0 | 0 | 0 |
| 2 | `cargo +stable check --workspace --all-targets --all-features` | 2026-10-28T20:25:55.7035220+01:00 | 2026-10-28T20:26:31.1754502+01:00 | 35.47 s | 0 | 0 / 0 / 0 | 6 (DATA-007) | 0 |
| 3 | `cargo +stable clippy --workspace --all-targets --all-features` | 2026-10-28T20:26:31.3437016+01:00 | 2026-10-28T20:26:36.6735819+01:00 | 5.33 s | 0 | 0 / 0 / 0 | 341 (340 lib + 1 bin) | 0 |
| 4 | `cargo +stable test --workspace --all-targets` | 2026-10-28T20:26:36.9096728+01:00 | 2026-10-28T20:35:54.8116470+01:00 | 557.90 s | 0 | 1519 passed / 0 failed / 10 ignored | 6 (lexer) | 0 |
| 5 | `cargo +stable test --doc` | 2026-10-28T20:35:55.1511000+01:00 | 2026-10-28T20:37:08.8393214+01:00 | 73.69 s | 0 | 0 passed / 0 failed / 0 ignored | 6 (lexer) | 0 |
| 6 | `cargo audit` | 2026-10-28T20:37:09.0658039+01:00 | 2026-10-28T20:37:20.3378674+01:00 | 11.27 s | 1 (vulnerabilities found) | 0 / 0 / 0 | 9 unmaintained/yanked | 7 podatnoĹ›ci (DEP-001) |
| 7 | `cargo tree -d` | 2026-10-28T20:37:20.5767873+01:00 | 2026-10-28T20:37:23.2226596+01:00 | 2.65 s | 0 | 0 / 0 / 0 | 0 | 0 |
| 8 | `cargo +stable test --lib` | 2026-10-28T20:37:23.4769481+01:00 | 2026-10-28T20:44:00.4743173+01:00 | 397.00 s | 0 | 983 passed / 0 failed / 0 ignored | 6 (lexer) | 0 |
| 9 | `cargo +stable test sync:: --lib` | 2026-10-28T20:44:00.7372822+01:00 | 2026-10-28T21:05:34.5708930+01:00 | 1293.83 s | 0 | 82 passed / 0 failed / 0 ignored | 6 (lexer) | 0 |

---

## 3. Pomiary ostrzeĹĽeĹ„ Clippy wedĹ‚ug celĂłw

| Lp. | Cel kompilacji / polecenie | Czas (s) | Exit Code | Liczba ostrzeĹĽeĹ„ w targecie | SzczegĂłĹ‚y |
|---|---|---|---|---|---|
| 1 | `cargo +stable clippy --lib` | 17.96 s | 0 | 340 | `splogbook` (lib) |
| 2 | `cargo +stable clippy --lib --tests` | 10.10 s | 0 | 417 | `splogbook` (lib test: 340 bazowych + 77 w testach) |
| 3 | `cargo +stable clippy --bin "SPLogbook"` | 9.12 s | 0 | 1 | `splogbook` (bin "SPLogbook": 1 ostrzeĹĽenie `if` can be collapsed) |
| 4 | `cargo +stable clippy --bin "SPLogbook" --tests` | 9.44 s | 0 | 1 | `splogbook` (bin "SPLogbook" test: 1 ostrzeĹĽenie `assert_is_empty`) |
| 5 | `cargo +stable clippy --bin splogbook-signer` | 8.24 s | 0 | 0 | `splogbook-signer` (0 warningĂłw w signerze) |
| 6 | `cargo +stable clippy --bin splogbook-signer --tests` | 10.21 s | 0 | 0 | `splogbook-signer` test (0 warningĂłw w signerze) |

---

## 4. Analiza wynikĂłw testĂłw

- ĹÄ…czna liczba testĂłw w workspace: **1519 passed**, **0 failed**, **10 ignored**.
- Wszystkie 10 ignorowanych testĂłw to testy Hardware-In-The-Loop w `tests/cat_native_7.rs`:
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
  SÄ… one poprawnie oznaczone bramkami sprzÄ™towymi i pozostajÄ… pominiÄ™te zgodnie z zasadÄ… HIL.

---

## 5. Analiza wydajnoĹ›ci (PERF-001 / TEST-001)

- Czas wykonania `cargo +stable test sync:: --lib`: **1293.83 s** (~21 minut i 33 sekundy).
- Przyczyna: 3 testy chunkowania operujÄ…ce na 15 000 wpisĂłw:
  - `sync::tests::each_inventory_chunk_is_at_most_one_mib`
  - `sync::tests::each_records_chunk_is_at_most_one_mib`
  - `sync::tests::each_request_chunk_is_at_most_one_mib`
  Testy te wykonujÄ… kwadratowÄ… liczbÄ™ operacji serializacji i alokacji `serde_json::to_vec(&chunk)` w kaĹĽdej iteracji, co dowodzi znaleziska **PERF-001** oraz **TEST-001**.

---

## 6. Analiza podatnoĹ›ci zaleĹĽnoĹ›ci (DEP-001)

Wynik polecenia `cargo audit`:
- **7 podatnoĹ›ci** (vulnerabilities):
  1. `ctutils 0.4.2` (RUSTSEC-2026-0385) â€“ Non-constant-time mask generation and u128 comparisons -> Wymagana aktualizacja do `>=0.4.4`
  2. `rustls 0.23.45` (RUSTSEC-2026-0346) â€“ Debug impls for client session values expose TLS secrets -> Upgrade do `>=0.23.46`
  3. `rustls 0.23.45` (RUSTSEC-2026-0347) â€“ Buffered application data sent to outer server after ECH rejection -> Upgrade do `>=0.23.46`
  4. `rustls 0.23.45` (RUSTSEC-2026-0348) â€“ Brotli certificate decompression can allocate up to 1 GiB -> Upgrade do `>=0.23.46`
  5. `rustls 0.23.45` (RUSTSEC-2026-0349) â€“ Unauthenticated ServerHello can downgrade later connections -> Upgrade do `>=0.23.46`
  6. `rustls-webpki 0.103.15` (RUSTSEC-2026-0382) â€“ Certificate verification may accept distrusted intermediates -> Upgrade do `>=0.103.16`
  7. `webpki-root-certs 1.0.9` (RUSTSEC-2026-0383) â€“ Bundled root certificates allow trust in distrusted emSign intermediates -> Upgrade do `>=1.0.10`
- **9 ostrzeĹĽeĹ„**:
  - Unmaintained crates: `bincode 1.3.3`, `paste 1.0.15`, `smartstring 1.0.1`, `zbus_names 4.3.4`, `zvariant 5.15.0`, `zvariant_derive 5.15.0`
  - Unsound constructor: `tokio 1.53.1` (RUSTSEC-2026-0386: `AsyncFd::new`)
  - Yanked releases: `clipboard-win 5.4.1`, `yoke-derive 0.8.3`

---

## 7. Analiza ostrzeĹĽeĹ„ leksera SQL (DATA-007)

W pliku `src/core/database/qso_store.rs`:
- 6 ostrzeĹĽeĹ„ kompilatora: `multiple lines skipped by escaped newline` (linie 310, 312 itp.) â€“ znalezisko **DATA-007** potwierdzone.

---

## 8. Pomiary po Etapie 1D-B (PERF-001, TEST-001)

- Czas wykonania `cargo +stable test sync:: --lib`: **35.70 s** (poprzednio 354.30 s bazowo, w audycie 1293.83 s â€“ przyspieszenie **10x** wzglÄ™dem bazy).
- Czas wykonania `cargo +stable test --lib`: **37.05 s** (poprzednio 353.36 s bazowo, w audycie 532.05 s â€“ przyspieszenie **9.5x** wzglÄ™dem bazy).
- Testy chunkowania 15 000 elementĂłw:
  - `sync::tests::each_inventory_chunk_is_at_most_one_mib`: **0.14 s** (poprzednio 254.09 s).
  - `sync::tests::each_request_chunk_is_at_most_one_mib`: **0.14 s** (poprzednio 256.84 s).
  - `sync::tests::each_records_chunk_is_at_most_one_mib`: **0.29 s** (poprzednio 45.85 s).
- Pomiary celowane (3 serie pomiarowe, mediana):
  - `chunk_inventory` $N=15\,000$: **63.41 ms** (min 58.22 ms, max 69.29 ms, cel <200 ms speĹ‚niony z duĹĽym marginesem).
  - `chunk_requests` $N=15\,000$: **62.21 ms** (min 56.46 ms, max 77.21 ms).
  - `chunk_records` $N=3\,000$: **152.66 ms** (min 131.23 ms, max 185.83 ms).
- Liczba operacji serializacji: Ĺ›ciĹ›le liniowa $N + 1$ (potwierdzona testem deterministycznym `catchup_chunking_15k_identities_has_linear_serialization_count`).
- Parametry produkcyjne Argon2id: nienaruszone (19 MiB RAM, 2 iteracje, 1 wÄ…tek), zabezpieczone testem `production_argon2_parameters_remain_unchanged`.

---

## 9. Pomiary i weryfikacja bramki naprawczej PERF-001

- **Kontrakt limitĂłw protokoĹ‚u synchronizacji:**
  - `MAX_CHUNK_BYTES`: **1 MiB** (`1 048 576 B`, `src/sync/catchup.rs:18`) â€“ docelowy limit grupowania wielu rekordĂłw / identyfikatorĂłw.
  - `MAX_RECORD_BYTES`: **6 MiB** (`6 291 456 B`, `src/sync/catchup.rs:25`) â€“ twardy limit pojedynczego rekordu QSO w JSON.
  - `MAX_FRAME_BYTES`: **8 MiB** (`8 388 608 B`, `src/sync/frame.rs:10`) â€“ twardy limit warstwy ramkowania TCP (`write_frame`, `decode_length`).
  - `FRAME_OVERHEAD_BYTES`: **44 B** (`24 B` nonce + `16 B` Poly1305 tag + `4 B` prefiks dĹ‚ugoĹ›ci big-endian).
  - Narzut koperty JSON `Message::Records`: **33 B** (`last: true`) lub **34 B** (`last: false`).
  - CaĹ‚kowity narzut transportowy z kopertÄ…: **77â€“78 B**.
- **Weryfikacja duĹĽych rekordĂłw (SINGLE_RECORD_EXCEPTION_IS_VALID):**
  - Rekordy w przedziale $>1\text{ MiB}$ i $\le 6\text{ MiB}$ sÄ… emitowane w osobnym dedykowanym chunku `Message::Records` z 1 rekordem.
  - Maksymalny rozmiar zaszyfrowanej ramki dla rekordu 6 MiB: $\le 6\,291\,534\text{ B}$, z zapasem $>2{,}09\text{ MiB}$ poniĹĽej limitu transportowego `MAX_FRAME_BYTES` (8 MiB).
  - Roundtrip end-to-end potwierdzony testem `single_large_record_roundtrips_through_production_frame_codec` (dla 1 MiB+1, 2 MiB, 5.9 MiB, MAX_RECORD_BYTES - 500 B).
  - Rekordy $>6\text{ MiB}$ sÄ… odrzucane przez nadawcÄ™ (`RecordTooLarge`) bez przerywania sesji i bez skaĹĽenia bazy (`large_record_rejection_has_no_partial_side_effects`, `oversized_record_rejection_does_not_claim_full_catchup_success`).
- **Zabezpieczenie dekodera ramki:**
  - `decode_length` weryfikuje dĹ‚ugoĹ›Ä‡ przed zaalokowaniem bufora sterty.
  - Testy `frame_decoder_rejects_length_above_transport_limit` oraz `frame_decoder_rejects_length_overflow` potwierdzajÄ… natychmiastowe odrzucenie ramki na 4. bajcie bez alokacji.
- **PeĹ‚ne liczenie operacji:**
  - Test `catchup_chunking_15k_reports_complete_linear_operation_counts`: $1$ empty envelope + $15\,000$ item serializations + $3 \times C$ (final chunk, encryption, frame encoding) = Ĺ›ciĹ›le liniowy potok $O(N)$.
  - Test `estimated_chunk_size_matches_actual_serialized_size`: rzeczywisty rozmiar rĂłwny estymacji co do bajta dla `last: false` oraz estymacja - 1 B dla `last: true`.
  - Test `last_flag_size_delta_is_accounted_for`: 1-bajtowa delta `"last":false` vs `"last":true` w peĹ‚ni uwzglÄ™dniona w konserwatywnym oszacowaniu.
- **PamiÄ™Ä‡ i alokacje:**
  - Skumulowany wolumen alokacji tymczasowych poprzedniego algorytmu wynosiĹ‚ $>6\text{ GiB}$ dla $N=15\,000$.
  - Nowy algorytm utrzymuje w live heap wyĹ‚Ä…cznie dane wejĹ›ciowe i aktualny chunk.
  - Szczyt pamiÄ™ci rezydentnej (peak RSS) nie zostaĹ‚ bezpoĹ›rednio zmierzony.
- **Ryzyko resztkowe:** **NISKIE** (status ZEROWE sprostowany zgodnie z metodykÄ… inĹĽynierskÄ…).


---

## 10. Pomiary i weryfikacja po Etapie 3 (CAT-002, CAT-003, CAT-005, CAT-006, CAT-015)

- **Status Etapu 3:** **SOFTWARE VERIFIED**
- **Status fizycznego HIL:** **NOT EXECUTED â€” SAFETY GATE CLOSED** (10 fizycznych testĂłw sprzÄ™towych pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`).
- **Semantyka zwolnienia PTT:**
  - `Drop`: inicjuje synchroniczne best-effort wysĹ‚anie komendy TX OFF, lecz nie gwarantuje asynchronicznego potwierdzenia powrotu radia do RX.
  - Jawne zamkniÄ™cie sesji (`release_and_confirm_rx` / `disconnect`): wysyĹ‚a komendÄ™ TX OFF i asynchronicznie oczekuje na potwierdzenie stanu RX z transceivera.
- **Ujednolicone mapowanie znalezisk i commitĂłw (Bramka Dokumentacyjna):**
  - `CAT-002`: WyĹ›cig potwierdzenia PTT w TCI i FlexRadio -> Commity `31ab74e` *(w msg tag CAT-003)* i `ce73b8c` *(w msg tag CAT-005)*. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-003`: Polimorficzny read-only `probe()` dla wszystkich 8 backendĂłw -> Commit `3e907aa` *(w msg tag CAT-002)*. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-005`: WymĂłg odpytania `TX;`/`TQ;` bez optymizmu w Yaesu i Elecraft -> Commit `1fce33a` *(w msg tag CAT-006)*. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-006`: `RigHub::resync_tx_arbiter` (`last_state_update`) i oznaczanie `StateFreshness::Stale` -> Commit `1fce33a`. Status: `FIXED_SOFTWARE_VERIFIED`.
  - `CAT-015`: Cykl ĹĽycia sesji HIL i bramki Ĺ›rodowiskowe `SPLOG_HIL_*` -> Commit `ddc1c0c`. Status: `FIXED_SOFTWARE_VERIFIED` (fizyczny HIL: NOT EXECUTED â€” SAFETY GATE CLOSED).

- **Stan gaĹ‚Ä™zi po Etapie 3:**
  - Aktywna gaĹ‚Ä…Ĺş: `repair/full-system-remediation` (zsynchronizowana z `main` i `origin/main` przez fast-forward).
  - Wszystkie commity Etapu 3:
    1. `3e907aa`: `fix(cat): make backend probing strictly read-only (CAT-002)`
    2. `31ab74e`: `fix(cat): require confirmed PTT ownership for TCI (CAT-003)`
    3. `ce73b8c`: `fix(cat): require confirmed PTT ownership for FlexRadio (CAT-005)`
    4. `1fce33a`: `fix(cat): enforce fail-safe PTT cleanup (CAT-006)`
    5. `ddc1c0c`: `fix(hil): protect hardware PTT session lifecycle (CAT-015)`
- **Zabezpieczenie Cargo:**
  - `Cargo.toml`: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (nienaruszone)
  - `Cargo.lock`: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (nienaruszone)
  - Nowe zaleĹĽnoĹ›ci: 0 (zakaz Ĺ›ciĹ›le dotrzymany).
- **Wyniki testĂłw domeny CAT i HIL:**
  - `tests/cat_native_6a.rs`: 165 testĂłw PASSED (w tym 5 nowych deterministycznych testĂłw synchronizacji i wyĹ›cigĂłw PTT).
  - `tests/cat_native_6b.rs`: 87 testĂłw PASSED (w tym 12 nowych testĂłw kontraktowych read-only probe dla 8 backendĂłw).
  - `tests/cat_native_7.rs`: 297 testĂłw PASSED, 10 ignored (w tym 14 nowych testĂłw cyklu ĹĽycia sesji HIL i potwierdzeĹ„ powrotu do RX).
  - Wszystkie 10 fizycznych testĂłw sprzÄ™towych NATIVE-7B pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]` (bramka fizycznego nadawania zamkniÄ™ta).

---

## 5. Status po zakoĹ„czeniu Etapu 4 (SOFTWARE VERIFIED)

Data sporzÄ…dzenia: 2026-11-03
Ĺšrodowisko: Windows x86_64
Commit koĹ„cowy Etapu 4: `HEAD` (repair/full-system-remediation / main / origin/main)

### Podsumowanie wdroĹĽenia Etapu 4:
- **Zrealizowany zakres audytowy:**
  - `CAT-001`: Poprawne kodowanie BCD Little-Endian (czÄ™stotliwoĹ›Ä‡) i Big-Endian (poziomy) dla Icom CI-V oraz pÄ™tla reconnectu z exponential backoff i jitterem w `RigService`.
  - `CAT-004`: Walidacja granic ramek CI-V (min 6 B), odrzucanie echa, NAK, broadcastu, obcego adresu, obsĹ‚uga komend split `0x25` i RIT/XIT `0x21`.
  - `CAT-009`: Odpytywanie komendÄ… `i` w `HamlibBackend::poll_internal` przy wĹ‚Ä…czonym split i aktualizacja czÄ™stotliwoĹ›ci TX.
  - `CAT-010`: TrwaĹ‚y strumieĹ„ TCP HTTP Keep-Alive w `FlrigClient` oraz obsĹ‚uga Split, VFO i Power.
  - `CAT-011`: Poprawne parsowanie i kodowanie parametrĂłw RIT/XIT oraz passband w backendach natywnych.
  - `CAT-012`: Fallback getter query po 150 ms w `KenwoodBackend` przy braku echa AI, odczyt mocy `PC` i obsĹ‚uga `SetPower`/`SetRit`/`SetXit`.
  - `CAT-013`: Oczekiwanie na potwierdzenie `R<seq>|0|` przez `send_and_await_ack` w `FlexRadioBackend` i komenda `slice set tx=1` w `SetSplit`.
  - `CAT-016`: Dynamiczny lookup domyĹ›lnego adresu CI-V z profilu modelu w `HilTestBenchManifest::to_rig_config` oraz `ReconnectPolicy` w `RigService`.
  - `CAT-017`:
    1. Rozdzielenie sprzÄ™towego stanu PTT (`state.ptt`) od lokalnej wĹ‚asnoĹ›ci `TxOwned`, zachowanie External TX po reconnect bez maskowania przez `state.ptt = false`, obsĹ‚uga `PttState::Unknown`, blokada `SetPtt(true)` przy aktywnym External TX.
    2. PeĹ‚na izolacja generacji sesji (`SessionOperationResult`, sprawdzanie `session_generation` przed mutacjÄ… stanu wspĂłĹ‚dzielonego w `apply_poll_result`, `apply_connect_result`, `apply_disconnect_result`), odrzucanie spĂłĹşnionych wynikĂłw asynchronicznych, powiÄ…zanie timera reconnectu z generacjÄ…, anulowanie operacji asynchronicznych przy shutdown.
    3. Rygorystyczny framing odpowiedzi HTTP/1.1 keep-alive FLRig (`MAX_FLRIG_HTTP_HEADER_BYTES = 8192`, `MAX_FLRIG_XMLRPC_BODY_BYTES = 65536`, trwaĹ‚y bufor odbiorczy, walidacja nagĹ‚ĂłwkĂłw Content-Length, peer close, clean reset on reconnect).

- **Zrealizowane commity Etapu 4:**
  1. `a5f2a8c`: `docs: align Stage 3 CAT finding mappings and HIL status` (Commit 0 â€” Bramka dokumentacyjna)
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

- **Klasyfikacja zaleĹĽnoĹ›ci Cargo Audit:**
  - `Vulnerabilities`: 0
  - `Unsound advisories`: 0
  - `Yanked crates`: 1 (`async-executor 1.14.0`, tranzytywna przez `keyring` -> `zbus`, tymczasowo zaakceptowana bez modyfikacji Cargo.lock)
  - `Unmaintained crates`: 6 (osobna kategoria informacyjna, nie Ĺ‚Ä…czyÄ‡ z yanked)

- **Nienaruszone sumy kontrolne Cargo:**
  - `Cargo.toml`: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`
  - `Cargo.lock`: `c569097f3ef69c66ec59b74c29c1eb29efce69ae`
  - Nowe zaleĹĽnoĹ›ci: 0 (bezwzglÄ™dny zakaz dotrzymany).

- **BezpieczeĹ„stwo RF i HIL:**
  - Emisja RF: 0.
  - Wszystkie 10 fizycznych testĂłw HIL pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`.
  - Status HIL: `NOT EXECUTED â€” SAFETY GATE CLOSED`.
  - Status programowy Etapu 4: `SOFTWARE VERIFIED`.
  - Status bramki CAT-017: `FIXED_SOFTWARE_VERIFIED`.
  - GotowoĹ›Ä‡ do Etapu 5: `READY`.


---

## 6. Status po zakoĹ„czeniu KoĹ„cowej Bramki Naprawczej Etapu 5 â€” IntegralnoĹ›Ä‡ Danych (SOFTWARE VERIFIED)

Data sporzÄ…dzenia: 2026-11-03
Ĺšrodowisko: Windows x86_64
Aktywna gaĹ‚Ä…Ĺş: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main` i `origin/main` (ahead/behind: 0 0)
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres audytowy:
- `DATA-001`: Jednoznaczna polityka dĹ‚ugoĹ›ci ADIF. Wprowadzenie `AdifParseMode::Strict` (rygorystyczne ciÄ™cie bajtowe UTF-8, odrzucanie rekordĂłw z przeciÄ™tym znakiem) oraz `AdifParseMode::Compatibility` (fallback na liczbÄ™ znakĂłw Unicode wyĹ‚Ä…cznie na granicy znaku i przed nastÄ™pnym tagiem/EOR, z jawnym raportowaniem ostrzeĹĽeĹ„ `AdifCompatibilityWarning`). Eksport ADIF oparty o kanonicznÄ… dĹ‚ugoĹ›Ä‡ bajtowÄ… UTF-8.
- `DATA-002`: Dowodowa atomowoĹ›Ä‡ zapisu pojedynczego QSO (`ingest`) oraz wsadowego (`import_batch`). KaĹĽda operacja zapisu wykonuje `tx.commit()` w SQLite przed emisjÄ… zdarzeĹ„ i mutacjÄ… stanĂłw pochodnych. Przy jakimkolwiek bĹ‚Ä™dzie (w Ĺ›rodku lub na koĹ„cu batcha) nastÄ™puje peĹ‚ny ROLLBACK w SQLite, 0 rekordĂłw w bazie, 0 modyfikacji w AwardsEngine, 0 w ScpEngine, 0 wyemitowanych AppEvent. Potwierdzona moĹĽliwoĹ›Ä‡ idempotentnego ponowienia operacji.
- `DATA-003`: Stabilny, kanoniczny klucz toĹĽsamoĹ›ci `QsoIdentity` (`{CALL}|{BAND}|{MODE}|{DATE}|{MINUTE}`), pomijajÄ…cy znaki specjalne w datach i normalizujÄ…cy minuty. Migracja 6 schematu SQLite dodaje kolumnÄ™ `identity_key` i tworzy `CREATE UNIQUE INDEX idx_qso_identity_unique ON qso_records(journal_id, identity_key)`. Migracja wykonuje uprzedni audyt duplikatĂłw: w razie ich znalezienia przerywa dziaĹ‚anie i cofa transakcjÄ™, bez usuwania ani scalania rekordĂłw uĹĽytkownika. Eliminacja wyĹ›cigĂłw check-then-insert takĹĽe pomiÄ™dzy rĂłĹĽnymi poĹ‚Ä…czeniami SQLite i instancjami repozytorium.
- `ARCH-002`: SQLite jako pojedyncze ĹşrĂłdĹ‚o prawdy (Single Source of Truth). Dodanie metody `QsoIngestor::rebuild_derived_state()`, ktĂłra w razie awarii, restartu lub rozbieĹĽnoĹ›ci w 100% deterministycznie odbudowuje `AwardsEngine` i `ScpEngine` z bazy danych. Emisja `AppEvent::QsoLogged` nastÄ™puje wyĹ‚Ä…cznie po potwierdzonym commit SQLite; bĹ‚Ä…d odbiorcy zdarzeĹ„ nie powoduje utraty zapisanego rekordu.

### PeĹ‚ne wyniki weryfikacji bramki:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 bĹ‚Ä™dĂłw)
- `cargo +stable test --workspace --all-targets -- --list`:
  - 1874 aktywne testy programowe
  - 10 fizycznych testĂłw sprzetowych HIL (`#[ignore = "hardware-in-the-loop"]`)
  - 1884 wszystkich zarejestrowanych przypadkĂłw testowych (+49 wzglÄ™dem stanu wejĹ›ciowego)
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy domenowe i straĹĽnicy:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 285 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatnoĹ›ci (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained
- BezpieczeĹ„stwo RF i HIL:
  - Emisja RF: 0
  - Fizyczne testy CAT/HIL: odizolowane za `#[ignore = "hardware-in-the-loop"]`
  - Status HIL: `NOT EXECUTED â€” SAFETY GATE CLOSED`
- Prawdziwe bazy uĹĽytkownika: nienaruszone (testy wyĹ‚Ä…cznie w RAM i katalogach temp).
- Status Etapu 5: **SOFTWARE VERIFIED**.
- Status Etapu 6: **UNBLOCKED**.

## 7. Status po Ostatecznej Bramce Precyzji ToĹĽsamoĹ›ci QSO (DATA-003, SOFTWARE VERIFIED)

Data sporzÄ…dzenia: 2026-11-03
Ĺšrodowisko: Windows x86_64
Aktywna gaĹ‚Ä…Ĺş: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main`
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres Bramki Precyzji ToĹĽsamoĹ›ci QSO:
- **DATA-003 (Status: FIXED_SOFTWARE_VERIFIED):**
  - **Klucz toĹĽsamoĹ›ci v2:** `v2|{CALL}|{BAND}|{FREQ_HZ}|{MODE}|{SUBMODE}|{DATE}|{TIME_TAG}`.
  - **RozdzielczoĹ›Ä‡ sekundowa i minutowa:** Jawny enum `QsoTimePrecision` (`Minute` z tagiem `M{HHMM}`, `Second` z tagiem `S{HHMMSS}`). Brak fabrykowania sekund "00".
  - **RozrĂłĹĽnianie czÄ™stotliwoĹ›ci:** Stabilna reprezentacja `freq_hz` w hercach (`u64`), brak false-positive deduplikacji dla stacji w tej samej minucie na rĂłĹĽnych czÄ™stotliwoĹ›ciach.
  - **Normalizacja emisji i subemisji:** Funkcja `normalize_mode_and_submode` (USB/LSB -> SSB, FT8/FT4 -> cyfrowe, CW-R -> CW).
  - **Izolacja per-journal:** UnikalnoĹ›Ä‡ wymuszana Ĺ›ciĹ›le per `(journal_id, identity_key)`.
  - **Polityka rekordĂłw niekompletnych (MODEL B):** `identity_key = NULL`, SQLite UNIQUE traktuje kaĹĽdy NULL jako odrÄ™bny wpis, automatyczna deduplikacja dla niekompletnych rekordĂłw jest wyĹ‚Ä…czona.
  - **Migracja schematu 7 (`SCHEMA_VERSION` 6 -> 7):** W peĹ‚ni atomowa migracja z uprzednim audytem duplikatĂłw. W razie konfliktu nastÄ™puje natychmiastowy ROLLBACK, 0 rekordĂłw usuniÄ™tych, 0 scalonych, wersja schematu pozostaje bez zmian.
  - **Weryfikacja wielopoĹ‚Ä…czeniowa:** Potwierdzona stabilnoĹ›Ä‡ wyĹ›cigĂłw wspĂłĹ‚bieĹĽnych dla wielu poĹ‚Ä…czeĹ„ i repozytoriĂłw (5 powtĂłrzeĹ„ testu obciÄ…ĹĽeniowego bez ĹĽadnego bĹ‚Ä™du).

### PeĹ‚ne wyniki weryfikacji bramki:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 bĹ‚Ä™dĂłw)
- `cargo +stable test --workspace --all-targets -- --list`:
  - 1915 aktywnych testĂłw programowych (0 failed)
  - 10 fizycznych testĂłw sprzÄ™towych HIL (`#[ignore = "hardware-in-the-loop"]`)
  - 1925 wszystkich zarejestrowanych przypadkĂłw testowych (+41 wzglÄ™dem stanu wejĹ›ciowego)
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy domenowe i straĹĽnicy:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 297 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatnoĹ›ci (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained
- BezpieczeĹ„stwo RF i bazy danych:
  - Prawdziwa baza danych uĹĽytkownika nie zostaĹ‚a otwarta.
  - Ĺ»adne dane uĹĽytkownika nie zostaĹ‚y zmodyfikowane.

## 8. Status po ZakoĹ„czeniu Etapu 6 (Izolacja Wielu DziennikĂłw, Bezpieczne Undo, Atomowy Backup i Restore, SOFTWARE VERIFIED)

Data sporzÄ…dzenia: 2026-11-03
Ĺšrodowisko: Windows x86_64
Aktywna gaĹ‚Ä…Ĺş: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main` i `origin/main`
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres Etapu 6:
- **DATA-004 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Wszystkie operacje usuwania QSO (pojedyncze, zaznaczone, duplikaty) sÄ… zakresowane przez `journal_id`.
  - Typowana historia operacji Undo/Redo (`UndoAction::{DeleteSingle, DeleteBatch}`) ze Ĺ›cisĹ‚ym ograniczeniem do 20 wpisĂłw.
  - Przywracanie Ĺ‚Ä…cznoĹ›ci zachowuje oryginalne `id` oraz `journal_id` (`restore_qso_scoped`, `restore_multiple_qsos_scoped`).
  - Modalne okno potwierdzenia usuniÄ™cia zaznaczonych Ĺ‚Ä…cznoĹ›ci z peĹ‚nÄ… lokalizacjÄ… i18n (`Msg::new(key::MSG_QSO_DELETED_MULTIPLE)`).
  - UsuniÄ™cie duplikatĂłw w oknie â€žZnajdĹş duplikatyâ€ť rejestruje operacjÄ™ na stosie Undo i automatycznie odbudowuje `AwardsEngine`.
- **DATA-005 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Zmiana aktywnego dziennika w `switch_active_journal` uniewaĹĽnia cache statystyk, czyĹ›ci stosy Undo/Redo, selekcjÄ™ tabeli oraz w peĹ‚ni odbudowuje `AwardsEngine` i `ScpEngine` dla nowego dziennika.
  - Wszystkie zapytania agregujÄ…ce w `src/core/database/stats.rs` otrzymaĹ‚y warianty `stats_*_for_journal(journal_id)`.
  - Zadania asynchronicznej synchronizacji chmurowej (`online_sync.rs`) przechwytujÄ… `journal_id` przed zespawnowaniem zadania i pobierajÄ… rekordy wyĹ‚Ä…cznie dla wskazanego dziennika.
  - Automatyczna kopia zapasowa ADIF przy zamykaniu programu (`Drop for SpLogApp`) eksportuje wszystkie Ĺ‚Ä…cznoĹ›ci danego dziennika bez arbitralnego obcinania do 5000 rekordĂłw.
- **DATA-006 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Wyeliminowanie fallbacku `fs::copy` na ĹĽywej bazie WAL w `src/core/backup.rs`. Kopia tworzona jest wyĹ‚Ä…cznie przez SQLite Online Backup API (`rusqlite::backup::Backup`) z `busy_timeout(5s)`.
  - Walidacja spĂłjnoĹ›ci kopii zapasowej przed zatwierdzeniem: sprawdzenie rozmiaru, `PRAGMA quick_check(1)` oraz odczyt wersji schematu.
  - Zabezpieczenie przed utworzeniem kopii w tej samej Ĺ›cieĹĽce co aktywna baza (`BackupError::SamePath`).
  - Poprawa rotacji starych kopii (`prune_old_backups`) przez Ĺ›cisĹ‚e dopasowanie prefiksu `{stem}_backup_*.db`, eliminujÄ…ca usuwanie kopii innych baz danych w tym samym katalogu.
  - PeĹ‚na implementacja `BackupManager::restore_database` ze stagingiem w `.staging_restore.tmp`, automatycznÄ… migracjÄ… schematĂłw < 7, odrzuceniem schematĂłw > 7 (`SchemaMismatch`), migawkÄ… bezpieczeĹ„stwa do rollbacku `.pre_restore_bak` i czyszczeniem starych plikĂłw `-wal`/`-shm`.
- **CONC-001 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Wyeliminowanie blokowania wÄ…tkĂłw roboczych Tokio przez synchroniczny `db.lock()` w `online_sync.rs` za pomocÄ… `tokio::task::spawn_blocking`.
  - Wprowadzenie transakcyjnych metod wsadowych `mark_lotw_confirmed_batch_for_journal` i `mark_eqsl_confirmed_batch_for_journal`, zastÄ™pujÄ…cych pÄ™tle autocommit jednym atomowym commitem na tysiÄ…ce potwierdzeĹ„.

### Wyniki peĹ‚nej bramki weryfikacyjnej:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 bĹ‚Ä™dĂłw)
- Aktywne testy programowe: **1938 passed**, **0 failed** (+23 nowe testy w Etapie 6)
- Zignorowane testy fizycznego HIL: **10 ignored** (`#[ignore = "hardware-in-the-loop"]`)
- Wszystkie zarejestrowane przypadki testowe: **1948**
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy straĹĽnikĂłw i domenowe:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed (w tym `gui_has_no_hardcoded_text`)
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 297 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatnoĹ›ci (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained
- BezpieczeĹ„stwo danych:
  - Prawdziwa baza danych uĹĽytkownika nie zostaĹ‚a otwarta.
  - Ĺ»adne dane uĹĽytkownika nie zostaĹ‚y zmodyfikowane.

## 9. Status po ZakoĹ„czeniu Etapu 7 i Bramki Naprawczej (Domena ZawodĂłw KrĂłtkofalarskich: Cabrillo 3.0 i Regulaminy Punktacji, COMPLETED_SOFTWARE_VERIFIED)

Data sporzÄ…dzenia: 2026-11-05
Ĺšrodowisko: Windows x86_64
Aktywna gaĹ‚Ä…Ĺş: `repair/full-system-remediation`
Stan synchronizacji: fast-forward z `main` i `origin/main`
Cargo.toml SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (bez zmian)
Cargo.lock SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (bez zmian)

### Zrealizowany zakres Etapu 7 i Bramki Naprawczej:
- **FUNC-001 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Eksporter Cabrillo 3.0 w `src/core/cabrillo.rs` generuje peĹ‚ny, zgodny ze standardem nagĹ‚Ăłwek Cabrillo v3.0 (`START-OF-LOG: 3.0`, `CALLSIGN:`, `CONTEST:`, `CATEGORY-OPERATOR:`, `CATEGORY-TRANSMITTER:`, `CLAIMED-SCORE: <int>`, `CREATED-BY: SPLogbook`, `END-OF-LOG:`).
  - Poprawne kody emisji Cabrillo: `CW` (`CW`), `PH` (`SSB, USB, LSB, AM`), `FM` (`FM`), `RY` (`RTTY`), `DG` (`FT8, FT4, PSK, etc.`).
  - Zachowanie rzeczywistych raportĂłw RST (`rst_sent`, `rst_rcvd`) z mode-aware fallbackiem `599`/`59` stosowanym wyĹ‚Ä…cznie dla pustych pĂłl.
  - Dynamiczny eksport wymiany z `stx_string` (oraz fallback `stx`) i `srx_string` (oraz fallback `srx`).
  - Monotoniczne serie numerĂłw seryjnych dla Single-Op i Multi-One (`validate_cqwpx_serial_policy`).
  - ObsĹ‚uga kategorii `CabrilloTransmitterCategory::One` (kolumna `t` pomijana); odrzucanie kategorii wielonadajnikowych `Two`, `Multi`, `Unlimited` z `ContestError::UnsupportedContestCategory`.
  - PeĹ‚na retencja Ĺ‚Ä…cznoĹ›ci zduplikowanych (duplikatĂłw) w wyjĹ›ciowym pliku Cabrillo (0 punktĂłw).
- **FUNC-002 (Status: FIXED_SOFTWARE_VERIFIED):**
  - SP DX Contest (`score_sp_dx`): peĹ‚na obsĹ‚uga reguĹ‚ PZK dla stacji polskich (`my_dxcc == 269`, EU 1 pkt, DX 3 pkt, SP-SP 0 pkt, mnoĹĽnik DXCC per pasmo) oraz stacji zagranicznych (`my_dxcc != 269`, Ĺ‚Ä…cznoĹ›ci tylko z SP po 3 pkt, mnoĹĽnik 16 wojewĂłdztw wedĹ‚ug kanonicznej mapy: B = lubuskie, C = Ĺ‚Ăłdzkie, D = dolnoĹ›lÄ…skie, F = pomorskie, G = Ĺ›lÄ…skie, J = warmiĹ„sko-mazurskie, K = podkarpackie, L = lubelskie, M = maĹ‚opolskie, O = podlaskie, P = kujawsko-pomorskie, R = mazowieckie, S = Ĺ›wiÄ™tokrzyskie, U = opolskie, W = wielkopolskie, Z = zachodniopomorskie per pasmo). Dozwolone pasma 160mâ€“10m, emisje CW i SSB.
- **DOC-001 (Status: FIXED_SOFTWARE_VERIFIED):**
  - Korekta arytmetyki manualnego fixture CQ WW: 10 punktĂłw QSO Ă— 10 mnoĹĽnikĂłw = 100 (CLAIMED-SCORE: 100). Duplikaty wnoszÄ… 0 dodatkowych punktĂłw i 0 mnoĹĽnikĂłw.
  - Korekta nazw wojewĂłdztw SP DX i kanoniczna mapa 16 kodĂłw jednoliterowych: R = mazowieckie, C = Ĺ‚Ăłdzkie, K = podkarpackie, P = kujawsko-pomorskie.
  - UsuniÄ™cie artefaktĂłw zapisu matematycznego i peĹ‚na weryfikacja zgodnoĹ›ci dokumentacji z kodem i testami automatycznymi.
  - CQ WW Contest (`score_cqww`): wĹ‚asny DXCC 0 pkt, ten sam kontynent 1 pkt (wewnÄ…trz NA 2 pkt), inny kontynent 3 pkt; strefy CQ (1..40) oraz DXCC per pasmo.
  - CQ WPX Contest (`score_wpx`):
    - **Ten sam kraj (same-country): 1 punkt na kaĹĽdym paĹ›mie** (brak podwojenia na pasmach niskich!).
    - Ten sam kontynent, inny kraj: 1 pkt (wysokie pasma 10m/15m/20m), 2 pkt (niskie pasma 40m/80m/160m).
    - Ameryka PĂłĹ‚nocna inny kraj: 2 pkt (wysokie), 4 pkt (niskie).
    - Inny kontynent: 3 pkt (wysokie), 6 pkt (niskie).
    - MnoĹĽnik prefiksĂłw WPX: unikalny prefiks zaliczany dokĹ‚adnie raz na caĹ‚e zawody.
  - ĹšcisĹ‚e odseparowanie profili RTTY (`CqWpxRtty`, `CqWwRtty`) jako jawnie nieobsĹ‚ugiwanych wariantĂłw zwracajÄ…cych `ContestError::UnsupportedContestVariant`.
  - Walidacja pasm (`validate_band`), emisji (`validate_mode`) i wymian (`validate_exchange`) per profil zawodĂłw.
  - NiezaleĹĽne, rÄ™cznie wyliczone fixtures (`scoreboard_matches_manual_spdx_fixture`, `scoreboard_matches_manual_cqww_fixture`, `scoreboard_matches_manual_cqwpx_fixture`, `claimed_score_matches_manual_fixture`).

### Wyniki peĹ‚nej bramki weryfikacyjnej:
- `cargo +stable fmt --all -- --check`: 0 diff (formatowanie czyste)
- `cargo +stable check --workspace --all-targets --all-features`: EXIT 0 (0 bĹ‚Ä™dĂłw)
- `cargo +stable clippy --lib`: 382 warnings (Baseline: 383, spadek o 1 ostrzeĹĽenie, 0 nowych)
- `cargo +stable clippy --lib --tests`: 483 warnings (Baseline: 489, spadek o 6 ostrzeĹĽeĹ„, 0 nowych)
- Aktywne testy programowe: **2029 passed**, **0 failed** (+29 nowych testĂłw w Ostatecznej Bramce Korekcyjnej Etapu 7)
- Zignorowane testy fizycznego HIL: **10 ignored** (`#[ignore = "hardware-in-the-loop"]`)
- Wszystkie zarejestrowane przypadki testowe: **2039**
- `cargo +stable test --doc`: EXIT 0 (0 failed)
- Testy straĹĽnikĂłw i domenowe:
  - `tests/security_guard.rs`: 32 passed, 0 failed
  - `tests/i18n_guard.rs`: 15 passed, 0 failed
  - `tests/theme_guard.rs`: 1 passed, 0 failed
  - `tests/cat_native_6a.rs`: 165 passed, 0 failed
  - `tests/cat_native_6b.rs`: 87 passed, 0 failed
  - `tests/cat_native_7.rs`: 297 passed, 10 ignored HIL, 0 failed
- `cargo audit`:
  - 0 podatnoĹ›ci (vulnerabilities: 0)
  - 0 unsound
  - 1 yanked (`async-executor 1.14.0`)
  - 6 unmaintained (`bincode`, `paste`, `smartstring`, `zbus_names`, `zvariant`, `zvariant_derive`)
- BezpieczeĹ„stwo danych:
  - Prawdziwa baza danych uĹĽytkownika nie zostaĹ‚a otwarta.
  - Ĺ»adne dane uĹĽytkownika nie zostaĹ‚y zmodyfikowane.
- Ostateczna Bramka Dokumentacyjna Etapu 7 (2026-11-05):
  - Poprawa fixture i testĂłw CQ WW DX: potwierdzenie 3 punktĂłw na wszystkich pasmach (160mâ€“10m), prawidĹ‚owy wynik: 10 punktĂłw QSO Ă— 10 mnoĹĽnikĂłw = 100, CLAIMED-SCORE: 100.
  - Poprawa fixture i walidatora SP DX: Ĺ›cisĹ‚a egzekucja 16 jednoliterowych symboli wojewĂłdztw wedle mapy kanonicznej (B, C, D, F, G, J, K, L, M, O, P, R, S, U, W, Z; m.in. R = mazowieckie, C = Ĺ‚Ăłdzkie, K = podkarpackie, P = kujawsko-pomorskie), odrzucanie kodĂłw administracyjnych (`MA`, `LD`, etc.), CLAIMED-SCORE: 27.
  - Cabrillo 3.0: rozdzielenie `CATEGORY-OPERATOR` (`SINGLE-OP`, `MULTI-OP`, `CHECKLOG`) od `CATEGORY-TRANSMITTER` (`ONE`), usuniÄ™cie bĹ‚Ä™dnego `MULTI` w `CATEGORY-TRANSMITTER`.
  - Wyniki punktacji CQ WPX CW/SSB (144 pkt) w peĹ‚ni zachowane i zabezpieczone.
- UkoĹ„czenie i weryfikacja Etapu 8 (Rzeczywista data wykonania: 2026-11-06, Deklarowana data robocza projektu: 2026-11-05):
  - Etap 8 (Integracja Multi-Radio w GUI, REST API Rig i Hamlib Proxy) wdroĹĽony, utwardzony i zweryfikowany w 100% ze statusem `COMPLETED_SOFTWARE_VERIFIED`.
  - ZamkniÄ™te i zweryfikowane kanoniczne findings: `FUNC-003`, `CAT-007`, `CAT-008`, `CAT-014`.
  - WdroĹĽone commity Etapu 8:
    1. `8002667`: `refactor(cat): add native command bridge and clean up legacy controller (CAT-014)`
    2. `fd43ed2`: `fix(cat): enforce TxArbiter PTT confirmation in proxy and voice keyer (CAT-008)`
    3. `fe7be53`: `fix(gui): eliminate single-rig fallbacks and support radio 2 configuration (CAT-007)`
    4. `36d1dee`: `feat(api): wire REST rig control through CatRuntime and add multi-rig endpoints (FUNC-003)`
    5. `26096de`: `test(cat): add multi-radio concurrency, isolation, and proxy integration matrix`
    6. `3cff368`: `fix(cat): correct Hamlib error mapping for TxArbiter failures (CAT-008)`
    7. `833f28d`: `fix(api): harden REST rig control security boundary (FUNC-003)`
    8. `c11d0b1`: `fix(cat): bind Hamlib proxy sessions to explicit rig ports (CAT-007)`
    9. `cc7f36e`: `fix(cat): enforce exclusive physical rig endpoint ownership`
  - RozwiÄ…zane obszary koĹ„cowej bramki korekcyjno-bezpieczeĹ„stwowej Etapu 8:
    1. Kanoniczna semantyka kodĂłw Hamlib `rig.h` (`-11 = RIG_ENAVAIL`, `-14 = RIG_BUSBUSY`), centralny enum `HamlibErrorCode`, brak optymistycznego `RPRT 0` przy odrzuceniu rozkazu mutujÄ…cego lub PTT.
    2. Utwardzenie REST API: domyĹ›lny bind loopback `127.0.0.1:8420`, zakaz bindu publicznego bez auth, Ĺ›cisĹ‚a walidacja formatu `RigId` (400 Bad Request), walidacja czÄ™stotliwoĹ›ci i emisji, `deny_unknown_fields`, 404 dla nieznanego RigId bez fallbacku, routing PTT przez `TxArbiter` (409 Conflict).
    3. Jednoznaczne mapowanie portĂłw Hamlib Proxy (`ProxyPortManager`): port bazowy 4534 per radio 1, unikalne parzyste porty (4536, 4538...) per kolejne radia, zakaz kolizji z REST (8420), niezmiennoĹ›Ä‡ sesji TCP po zmianie aktywnego radia w GUI.
    4. WyĹ‚Ä…czna wĹ‚asnoĹ›Ä‡ fizycznego portu CAT (`PhysicalEndpointRegistry`): pojedynczy wĹ‚aĹ›ciciel UART, normalizacja portĂłw Windows (`COM3`, `\\.\COM3`), wykluczenie kolizji Native CAT z zewnÄ™trznym rigctld, brak otwierania portĂłw UART przez Hamlib Proxy i Hamlib Client.
  - Statystyki testĂłw po KoĹ„cowej Bramce Etapu 8:
    - ĹÄ…cznie zarejestrowanych testĂłw we workspace: 2081
    - Aktywne testy programowe: 2071 passed, 0 failed
    - Fizyczny HIL: 10 ignored (`#[ignore = "hardware-in-the-loop"]`), emisja RF = 0, bramka bezpieczeĹ„stwa HIL zamkniÄ™ta.
    - `cargo test --doc`: ok (0 doc testĂłw)
    - `cargo clippy --lib`: 0 ostrzeĹĽeĹ„ w nowym kodzie Etapu 8
    - `cargo fmt --all -- --check`: 0 odchyleĹ„ formatowania
    - `security_guard`: 32 passed, 0 failed
    - `i18n_guard`: 15 passed, 0 failed
    - `theme_guard`: 1 passed, 0 failed
  - IntegralnoĹ›Ä‡ zaleĹĽnoĹ›ci i plikĂłw konfiguracyjnych:
    - `Cargo.toml` SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (niezmieniony)
    - `Cargo.lock` SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (niezmieniony)
  - Prawdziwa baza danych uĹĽytkownika: nieotwierana, dane nienaruszone.
- UkoĹ„czenie i weryfikacja Etapu 9 (Rzeczywista data wykonania: 2026-11-06, Deklarowana data robocza projektu: 2026-11-06):
  - Etap 9 (Ergonomia Klawiaturowa, TrwaĹ‚oĹ›Ä‡ Konfiguracji i Optymalizacja Renderowania GUI) wdroĹĽony, utwardzony i zweryfikowany w 100% ze statusem `COMPLETED_SOFTWARE_VERIFIED`.
  - ZamkniÄ™te i zweryfikowane kanoniczne findings: `GUI-001`, `GUI-002`, `GUI-003`, `FUNC-005`, `PERF-002`.
  - WdroĹĽone commity Etapu 9 (Faza bazowa oraz KoĹ„cowa Bramka Remediacji):
    1. `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
    2. `229dea5`: `fix(gui): make QSO focus and keyboard workflow deterministic`
    3. `acecd8c`: `fix(config): make application settings durable and atomic`
    4. `c4cf893`: `fix(config): migrate and preserve multi-radio settings`
    5. `8397351`: `perf(gui): replace unnecessary continuous repaint with event-driven updates`
    6. `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
    7. `a7330ce`: `test(gui): add keyboard persistence and rendering regression matrix`
    8. `e4114c0` (`e4114c05dbdf54e6fbd11d305af5519e51552dc9`): `docs(audit): close Stage 9 GUI ergonomics persistence and performance findings`
    9. `e8580be` (`e8580beefd0c9ce1a54cf5f1ed7d6f7fd954deb8`): `test(audit): make workspace test counting deterministic`
    10. `60d43e4` (`60d43e44cacf6aa46beb7bbaf836fd4c427d349f`): `fix(gui): isolate Ctrl+W wipe to explicit QSO contexts (GUI-001)`
    11. `d564272` (`d5642725f4eff3cf536e2573058f545e646ba490`): `fix(config): redact REST token and harden secret handling (FUNC-005)`
    12. `665183b` (`665183b79b76da7f8072d5dd383287ebed4ce685`): `test(gui): make Stage 9 performance measurements reproducible (PERF-002)`
    13. `docs(audit): correct Stage 9 hashes counts benchmarks and chronology` (domkniÄ™cie bramki Etapu 9)
  - RozwiÄ…zane obszary Etapu 9:
    1. Ergonomia klawiaturowa i ochrona tekstu (`GUI-001`): centralny rejestr skrĂłtĂłw `ShortcutRegistry`, obsĹ‚uga kontekstĂłw (`Global`, `QsoEntry`, `Contest`, `LogTable`), odblokowanie skrĂłtĂłw z `Ctrl` podczas edycji tekstu z jednoczesnym zabezpieczeniem pĂłl wprowadzania przed jednoliterowymi skrĂłtami nawigacji, deterministyczna obsĹ‚uga `Enter` (zapis Ĺ‚Ä…cznoĹ›ci + powrĂłt fokusu na znak) oraz `Esc` (anulowanie/czyszczenie) w oknie zawodĂłw i formularzu gĹ‚Ăłwnym, deterministyczny cykliczny `Tab` order.
    2. PĹ‚ywajÄ…cy widĹĽet Asystenta Operatora (`GUI-002`): dodanie przycisku zamkniÄ™cia `âś•` w nagĹ‚Ăłwku okna oraz peĹ‚na trwaĹ‚oĹ›Ä‡ flagi `show_operator_assistant` w konfiguracji.
    3. Spoczynkowy odczyt telemetrii radia (`GUI-003`): eliminacja faĹ‚szywych odczytĂłw `S9+20dB` / `100 W` w stanie rozĹ‚Ä…czonym; inicjalizacja wartoĹ›ciami spoczynkowymi `S0` (`-127 dBm`) i `0.0 W`.
    4. TrwaĹ‚oĹ›Ä‡ i atomowoĹ›Ä‡ konfiguracji (`FUNC-005`): kompletna dwukierunkowa synchronizacja pĂłl w `SpLogApp::snapshot_config()` (CW, REST API, alerty solarne, satelity, layout/okna, motyw graficzny, profile multi-radio Radio 1 i Radio 2), walidacja schematu, bezpieczny atomowy zapis przez plik tymczasowy `.tmp` z rotacjÄ… `.bak` (`rename_atomic`) i zachowaniem poprzedniego sprawnego pliku w razie awarii I/O lub walidacji.
    5. Eliminacja niepotrzebnego repaintu i buforowanie GUI (`PERF-002`): usuniÄ™cie ciÄ…gĹ‚ego ĹĽÄ…dania odĹ›wieĹĽania 250ms w pÄ™tli renderowania, wprowadzenie harmonogramu `until_next_second`, event-driven wake context z `EventBus` i `CatRuntime` budzÄ…cy GUI natychmiast po nadejĹ›ciu telemetrii, buforowanie `sorted_indices` i `worked_callsigns` w `LogViewState` z kluczem rewizji `LogViewCacheKey`, buforowanie metryk zawodĂłw w `ContestState` z kluczem `ContestCacheKey`, weryfikacja spotĂłw klastra DX zoptymalizowana do $O(1)$ set lookup, buforowanie telemetrii radia `rig_cache` per `RigId` z zachowaniem limitu 30 pĂłl `CatState`.
    6. Macierz stabilnoĹ›ci i brak blokad: 5-krotne powtĂłrzenie 12 krytycznych testĂłw integracyjnych i wydajnoĹ›ciowych (60/60 zaliczonych, 0 flaky), brak blokad wÄ…tku GUI przez zadania w tle i operacje I/O.
  - Statystyki testĂłw po KoĹ„cowej Bramce Etapu 9:
    - ĹÄ…cznie zarejestrowanych testĂłw we workspace: 2172
    - Aktywne testy programowe: 2162 passed, 0 failed
    - Fizyczny HIL: 10 ignored (`#[ignore = "hardware-in-the-loop"]`), emisja RF = 0, bramka bezpieczeĹ„stwa HIL zamkniÄ™ta.
    - `cargo test --doc`: ok (0 doc testĂłw)
    - `cargo clippy`: 0 ostrzeĹĽeĹ„ (brak reguĹ‚ blokujÄ…cych)
    - `cargo fmt --all -- --check`: 0 odchyleĹ„ formatowania
    - `security_guard`: 42 passed, 0 failed
    - `i18n_guard`: 15 passed, 0 failed
    - `theme_guard`: 1 passed, 0 failed
    - `cargo audit`: 0 podatnoĹ›ci
  - IntegralnoĹ›Ä‡ zaleĹĽnoĹ›ci i plikĂłw konfiguracyjnych:
    - `Cargo.toml` SHA-1: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4` (niezmieniony)
    - `Cargo.lock` SHA-1: `c569097f3ef69c66ec59b74c29c1eb29efce69ae` (niezmieniony)
  - Prawdziwa baza danych uĹĽytkownika: nieotwierana, dane nienaruszone.
- GotowoĹ›Ä‡ do kolejnego etapu:
  - Etap 9 definitywnie i bezwarunkowo zamkniÄ™ty ze statusem `COMPLETED_SOFTWARE_VERIFIED`.
- UkoĹ„czenie i weryfikacja Etapu 10 (Rzeczywista data wykonania: 2026-11-06, Deklarowana data robocza projektu: 2026-11-06):
  - Etap 10 (PodĹ‚Ä…czenie Funkcji Pozornych, CzystoĹ›Ä‡ `cargo clippy -D warnings` i Dokumentacja) wdroĹĽony, utwardzony i zweryfikowany w 100% ze statusem `COMPLETED_SOFTWARE_VERIFIED`.
  - ZamkniÄ™te i zweryfikowane kanoniczne findings: `FUNC-004`, `FUNC-006`, `FUNC-007`, `FUNC-008`, `RUST-001`, `CI-002`, `DOC-001`.
  - WdroĹĽone commity Etapu 10:
    1. `9cb49dc`: `fix(digital): connect js8call listener, fldigi test and winkeyer frames (FUNC-004)`
    2. `85104a1`: `fix(gui): wire audio recording stop photo preview and EME rotor (FUNC-004)`
    3. `062d65c`: `fix(core): load club affiliations dynamically from reference database (FUNC-006)`
    4. `a5d9bde`: `fix(digital): implement winkeyer frames fldigi xmlrpc test and js8call listener (FUNC-004)`
    5. `53d532a`: `fix(awards): support wwff references, display propagation accuracy and fix lotw since date (FUNC-007, FUNC-008)`
    6. `caf72d1`: `fix(awards): enforce club uniqueness, complete awards engine and propagation UI (FUNC-006, FUNC-007)`
    7. `d2c6970`: `refactor(clippy): clean core and database warnings (RUST-001)`
    8. `95d77cb`: `refactor(clippy): clean CAT and digital warnings (RUST-001)`
    9. `1ae0eae`: `refactor(clippy): clean GUI warnings (RUST-001)`
    10. `21d3bf3`: `refactor(clippy): clean cloud media and plugin warnings (RUST-001)`
    11. `efaece4`: `refactor(clippy): clean binaries and integration test warnings (RUST-001)`
    12. `037d4c5`: `fix(test): preserve UTF-8 encoding in app_config, psk_reporter and security_guard (RUST-001)`
    13. `7466bdd`: `ci(quality): enforce Clippy deny warnings and dependency audit (CI-002)`
  - RozwiÄ…zane obszary Etapu 10:
    1. PeĹ‚ny przepĹ‚yw produkcyjny funkcji pozornych (`FUNC-004`):
       - `AudioRecorder::stop_recording`: idempotentne zatrzymanie workera, oprĂłĹĽnienie bufora i finalizacja nagĹ‚Ăłwka WAV przez wewnÄ™trzny trait `AudioSink` bez otwierania fizycznego urzÄ…dzenia audio w testach.
       - `PhotoViewerDialog`: przekazanie wybranego ĹşrĂłdĹ‚a z galerii QSL, odrzucenie pustych Ĺ›cieĹĽek, jawne raportowanie brakujÄ…cego pliku w GUI i czyszczenie stanu przejĹ›ciowego przy zamkniÄ™ciu.
       - Rotator EME (`rotate_antenna_to_el`): walidacja zakresu azymutu (`0..=360`) i elewacji (`-90..=90`), odrzucenie `NaN`/`Infinity`, eliminacja niejawnego fallbacku na pierwsze radio, uĹĽycie abstrakcji `RotatorTransport` i bounded timeout.
       - `Js8CallListener`: ustawienia `js8call_enabled` i `js8call_port` w `AppConfig`, dynamiczny start/stop, obsĹ‚uga ramek TCP JSON Lines i odpornoĹ›Ä‡ na czÄ™Ĺ›ciowe/uszkodzone ramki.
       - `FldigiClient::test_connection`: zapytanie XML-RPC `fldigi.version` + `main.get_frequency` z aktualizacjÄ… stanu `Online`/`Error` w `online_sync.rs`.
       - `WinkeyerProtocol` i `WinkeyerSession`: generowanie binarnych ramek K1EL (`HostOpen`, `SetWpm`, `Abort`, tekst CW), abstrakcja `WinkeyerTransport` z `MockWinkeyerTransport`, brak cichego sukcesu bez skonfigurowanego transportu.
    2. Rejestr klubĂłw krĂłtkofalarskich (`FUNC-006`): usuniÄ™cie zahardkodowanych 11 znakĂłw ze wspĂłĹ‚dzielonymi numerami czĹ‚onkowskimi, dynamiczne Ĺ‚adowanie z `databases/clubs.csv` (lub konfiguracji uĹĽytkownika) z rygorystycznÄ… walidacjÄ… unikalnoĹ›ci par `(club, member_number)` i normalizacjÄ… znakĂłw bazowych (`SP9ABC/P` -> `SP9ABC`).
    3. DomkniÄ™cie `AwardsEngine`, `PropagationHistory` i `qsl_print` (`FUNC-007`): zliczanie i prezentacja referencji `WWFF` i `RDA` w silniku i oknie dyplomĂłw, wyĹ›wietlanie skutecznoĹ›ci prognoz (`accuracy_rate`, `band_accuracy`) w panelu solarnym oraz integracja kalkulacji arkuszy `qsl_print::calculate_total_sheets`.
    4. Przyrostowe pobieranie potwierdzeĹ„ LoTW (`FUNC-008`): przekazywanie i walidacja parametru `qso_qslsince` (`YYYY-MM-DD`) w `LotwClient::fetch_confirmations` z redakcjÄ… poĹ›wiadczeĹ„ w diagnostyce.
    5. CzystoĹ›Ä‡ `cargo clippy -- -D warnings` (`RUST-001`): redukcja ostrzeĹĽeĹ„ z 389 unikalnych (768 Ĺ‚Ä…cznie we wszystkich targetach) do **0 ostrzeĹĽeĹ„** we wszystkich targetach workspace'u bez dodawania nowych `#[allow(...)]`.
    6. Utwardzenie potoku CI (`CI-002`): wymuszenie `cargo clippy --workspace --all-targets --all-features -- -D warnings` oraz `cargo audit --deny warnings` (z jawnÄ… listÄ… 7 zaakceptowanych tranzytywnych advisories) w `.github/workflows/ci.yml`, zabezpieczone 7 testami w `tests/security_guard.rs`.
    7. Synchronizacja dokumentacji (`DOC-001`): aktualizacja `README.md`, `Native CAT.md`, `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md` i `docs/REMEDIATION_BASELINE.md`.
  - Statystyki testĂłw po KoĹ„cowej Bramce Etapu 10:
    - ĹÄ…cznie zarejestrowanych testĂłw we workspace: **2289** (+117 nowych testĂłw w Etapie 10)
    - Aktywne testy programowe: **2279 passed**, **0 failed**
    - Fizyczny HIL: **10 ignored** (`#[ignore = "hardware-in-the-loop"]`), emisja RF = 0, fizyczne PTT = 0, bramka bezpieczeĹ„stwa HIL zamkniÄ™ta (`NOT EXECUTED â€” SAFETY GATE CLOSED`).
    - `cargo test --doc`: ok (0 doc testĂłw)
    - `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings`: **EXIT 0 (0 warnings, 0 errors)**
    - `cargo +stable fmt --all -- --check`: **0 odchyleĹ„ formatowania**
    - `security_guard`: **57 passed, 0 failed**
    - `i18n_guard`: **15 passed, 0 failed**
    - `theme_guard`: **1 passed, 0 failed**
    - `cat_native_6a`: **165 passed, 0 failed**
    - `cat_native_6b`: **99 passed, 0 failed**
    - `cat_native_7`: **297 passed, 10 ignored HIL, 0 failed**
    - `cargo audit`: **0 vulnerabilities, 0 unsound** (1 yanked + 6 unmaintained tranzytywnych, udokumentowanych)
  - IntegralnoĹ›Ä‡ zaleĹĽnoĹ›ci i plikĂłw konfiguracyjnych:
    - `Cargo.toml` SHA-1: `0cdf381165bfc3111fff15097007e4b235484ec8` (blob Git LF: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`, niezmieniony)
    - `Cargo.lock` SHA-1: `52be66b2bc063c001d5c2e8556457fa2be04ff4f` (blob Git LF: `c569097f3ef69c66ec59b74c29c1eb29efce69ae`, niezmieniony)
  - Prawdziwa baza danych i konfiguracja uĹĽytkownika: nieotwierane, dane nienaruszone.
- GotowoĹ›Ä‡ do kolejnego etapu:
  - Etap 10 definitywnie i bezwarunkowo zamkniÄ™ty ze statusem `COMPLETED_SOFTWARE_VERIFIED`.
  - Etap 11 (fizyczne testy sprzÄ™towe HIL) pozostaje nierozpoczÄ™ty (`NOT EXECUTED â€” SAFETY GATE CLOSED`).

---

## 7. Bramka BezpieczeĹ„stwa Zaufania, Manifestu i Danych (UPDATE-TRUST-AND-DATA-SAFETY-GATE)

- **Data robocza projektu:** 2026-11-06
- **Rzeczywista data wykonania w Ĺ›rodowisku:** 2026-11-07 (UTC+01:00 / CET)
- **Status bramki:** `COMPLETED_SOFTWARE_VERIFIED`
- **Commit wejĹ›ciowy:** `a82df1fc104ad12c3c4a4c784191ecca8c385f52`
- **GaĹ‚Ä…Ĺş robocza:** `repair/full-system-remediation` (zsynchronizowana fast-forward z `main` i `origin/main`)
- **WdroĹĽone commity bramki:**
  1. `bc3e806`: `fix(updater): require verified manifest for GUI update flow (UPDATE-TRUST-001)`
  2. `12c3b22`: `fix(updater): plan file updates with fail-closed data protection (UPDATE-DATA-001)`
  3. `da34b4c`: `fix(updater): preserve portable data during Windows and Linux update and rollback (UPDATE-DATA-001)`
  4. `c95172e`: `test(updater): harden trust data rollback and health safety matrix`
- **RozwiÄ…zane obszary:**
  1. **UPDATE-TRUST-001 (`FIXED_SOFTWARE_VERIFIED`):** Wymuszenie `VerifiedReleaseManifest` i `VerifiedReleaseAsset` w Ĺ›cieĹĽce GUI (`trigger_update_install`), eliminacja doboru wyĹ‚Ä…cznie po rozszerzeniach, fail-closed w przypadku odwoĹ‚anych kluczy (`REVOKED_KEY_IDS`).
  2. **UPDATE-DATA-001 (`FIXED_SOFTWARE_VERIFIED`):** ReguĹ‚a `UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE`, klasyfikacja `UpdateFileClass`, ochrona `databases/serviceLOG.db`, `databases/callbook.db` (`FirstInstallSeed`), `station_config.json`, `default_log.db`, likwidacja kopiowania dzikimi kartami (`$staging\*`, `$rollback\*`) w skrypcie PowerShell, precyzyjne `$replaceFiles` i `$createFiles`, walidacja `UpdatePlan` na Linuksie.
- **Pomiary po wdroĹĽeniu bramki:**
  - `cargo fmt --all -- --check`: **0 bĹ‚Ä™dĂłw**
  - `cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings`: **0 ostrzeĹĽeĹ„, 0 bĹ‚Ä™dĂłw**
  - ĹÄ…cznie zarejestrowanych testĂłw: **2345** (+56 nowych testĂłw zarejestrowanych wzglÄ™dem baseline 2289: +49 jednostkowych, +7 security guard)
  - Aktywne testy programowe: **2335 passed, 0 failed** (+56 nowych testĂłw wykonanych wzglÄ™dem baseline 2279)
  - HIL ignorowane: **10 ignored** (emisja RF = 0, fizyczne PTT = 0, safety gate closed)
  - `cargo audit`: **0 vulnerabilities, 0 unsound**
  - `Cargo.toml` SHA-1: `0cdf381165bfc3111fff15097007e4b235484ec8` (blob: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`, niezmieniony)
  - `Cargo.lock` SHA-1: `52be66b2bc063c001d5c2e8556457fa2be04ff4f` (blob: `c569097f3ef69c66ec59b74c29c1eb29efce69ae`, niezmieniony)
- **Status ZamkniÄ™cia Bramek Etapu M1 i Wyniki Etapu M2:**
  - `UPDATE-WIN-SAFETY-GATE`: **SATISFIED_BY_UPDATE_TRUST_AND_DATA_SAFETY_GATE** (`COMPLETED_SOFTWARE_VERIFIED`)
  - `UPDATE-LINUX-SAFETY-GATE`: **SATISFIED_BY_UPDATE_TRUST_AND_DATA_SAFETY_GATE** (`COMPLETED_SOFTWARE_VERIFIED`)
  - `ETAP M1`: **COMPLETED_SOFTWARE_VERIFIED**
  - ETAP M2: **COMPLETED_DESIGN_VERIFIED** (STAGE-M2-INSTALLER-DESIGN-GATE oraz STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE zakończone, raporty STAGE_M2_INSTALLER_DESIGN_GATE_REPORT.md i STAGE_M2_OWNER_DECISIONS_APPROVAL_REPORT.md)
    - UPDATE-DESIGN-WIN: APPROVED (DESIGN_3_0_A_WINDOWS_UPDATE.md v2.1.0)
    - UPDATE-DESIGN-LINUX: APPROVED (DESIGN_3_0_B_LINUX_UPDATE.md v2.1.0)
    - UPDATE-DESIGN-PLATFORM: APPROVED (DESIGN_3_0_C_PLATFORM_INSTALLERS.md v2.1.0)
  - ETAP M3: **IN_PROGRESS** (M3-A zrealizowany w STAGE-M3-A-PLATFORM-CORE-IMPLEMENTATION-GATE jako COMPLETED_SOFTWARE_VERIFIED; M3-B zrealizowany w STAGE-M3-B-WINDOWS-HELPERS-IMPLEMENTATION-GATE i formalnie zamkniety w STAGE-M3-B-CLOSURE-VERIFICATION-GATE jako COMPLETED_SOFTWARE_VERIFIED; M3-C zrealizowany w STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE jako COMPLETED_SOFTWARE_VERIFIED; M3-DEV-VERSION-IDENTITY zrealizowany w STAGE-M3-DEV-VERSION-IDENTITY-GATE jako COMPLETED_SOFTWARE_VERIFIED; M3-D Windows User Setup: COMPLETED_SOFTWARE_VERIFIED; M3-E Windows MSI: COMPLETED_SOFTWARE_VERIFIED; M3-F Linux Portable tar.gz: COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED)
  - CURRENT PUBLIC STABLE: 1.2.0 (reprezentowane przez niezmieniony tag v1.2.0)
  - CURRENT DEVELOPMENT: 1.3.0-alpha.1 (centralny BuildIdentity, aktywny kod źródłowy)
  - PUBLIC RELEASE STATUS: NOT PUBLISHED (nieopublikowane)
  - STAGE M3-D STATUS: STAGE-M3-D-WINDOWS-USER-SETUP-INTEGRATION-GATE: COMPLETED_SOFTWARE_VERIFIED
  - STAGE M3-E STATUS: STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE: COMPLETED_SOFTWARE_VERIFIED
  - STAGE M3-F STATUS: STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE: COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED
- **Raporty:**
  - Analiza wejĹ›ciowa: `UPDATE_PRE_IMPLEMENTATION_REPORT.md`
  - Raport wykonawczy bramki bezpieczeĹ„stwa: `UPDATE_TRUST_AND_DATA_SAFETY_GATE_REPORT.md`
  - KoĹ„cowy raport pokrycia i zamkniÄ™cia M1: `STAGE_M1_COVERAGE_AND_STATUS_REPORT.md`
  - Raport wykonawczy bramki projektowej M2: `STAGE_M2_INSTALLER_DESIGN_GATE_REPORT.md`
  - Raport wykonawczy bramki rdzenia platformowego M3-A: `STAGE_M3_A_PLATFORM_CORE_IMPLEMENTATION_REPORT.md`
  - Raport wykonawczy bramki pomocnikow Windows M3-B: `STAGE_M3_B_WINDOWS_HELPERS_IMPLEMENTATION_REPORT.md`
  - Raport zamkniecia i weryfikacji bramki pomocnikow Windows M3-B: `STAGE_M3_B_CLOSURE_VERIFICATION_REPORT.md`
  - Raport wykonawczy bramki integracji Windows Portable ZIP M3-C: `STAGE_M3_C_WINDOWS_PORTABLE_ZIP_INTEGRATION_REPORT.md`
  - Raport wykonawczy bramki tożsamości wersji rozwojowej M3: `STAGE_M3_DEV_VERSION_IDENTITY_REPORT.md`
  - Raport wstępny wznowienia bramki M3-D: `STAGE_M3_D_WINDOWS_USER_SETUP_RESUME_PRE_IMPLEMENTATION_REPORT.md`
  - Raport wykonawczy bramki integracji Windows User Setup M3-D: `STAGE_M3_D_WINDOWS_USER_SETUP_INTEGRATION_REPORT.md`
  - Raport propozycji decyzji toolchainu WiX M3-E: `STAGE_M3_E_WIX_TOOLCHAIN_DECISION_PROPOSAL.md`
  - Raport wstępny bramki Windows MSI WiX M3-E: `STAGE_M3_E_WINDOWS_MSI_WIX_PRE_IMPLEMENTATION_REPORT.md`
  - Raport wykonawczy bramki integracji Windows MSI WiX M3-E: `STAGE_M3_E_WINDOWS_MSI_WIX_INTEGRATION_REPORT.md`
