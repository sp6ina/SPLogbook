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
