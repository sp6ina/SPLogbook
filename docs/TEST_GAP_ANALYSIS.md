# Analiza Pokrycia Testami i Luk Testowych (`TEST_GAP_ANALYSIS.md`)

**Dokument audytowy:** `docs/TEST_GAP_ANALYSIS.md`  
**Projekt:** SPLogbook (`v1.2.0`)  
**Gałąź referencyjna:** `main` (`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`)

---

## 1. Wyniki Faktycznego Wykonania Zestawu Testów podczas Audytu

Wszystkie zestawy testów w repozytorium zostały uruchomione na aktywnym łańcuchu narzędziowym `rustc 1.99.0 (stable-x86_64-pc-windows-msvc)`:

| Cel testowy | Liczba testów uruchomionych | Passed | Failed | Ignored | Czas wykonania | Uwagi audytowe |
|---|---:|---:|---:|---:|---:|---|
| `splogbook` (`lib` unit tests) | 983 | **983** | 0 | 0 | **532,05 s** | 979 testów wykonuje się w ~15 s; 4 testy w `sync::tests` trwają >515 s (`PERF-001`, `TEST-001`). |
| `tests/cat_native_6a.rs` | 160 | **160** | 0 | 0 | **0,54 s** | Testy `RigHub`, `RigService`, `TxArbiter` i transportów natywnych. |
| `tests/cat_native_6b.rs` | 75 | **75** | 0 | 0 | **2,09 s** | Testy integracji GUI/CAT, migracji konfiguracji i `RigProbeRunner`. |
| `tests/cat_native_7.rs` | 295 | **285** | 0 | **10** | **0,08 s** | Testy domeny HIL Stage 7A (285) + 10 przypadków sprzętowych Stage 7B (`#[ignore]`). |
| `tests/i18n_guard.rs` | 15 | **15** | 0 | 0 | **24,54 s** | Walidacja 7 plików `locales/*.json`, brak zahardkodowanych tekstów PL w GUI i brak `Result<_, String>`. |
| `tests/theme_guard.rs` | 1 | **1** | 0 | 0 | **0,41 s** | Walidacja spójności palet motywów w `src/gui/theme.rs`. |
| `SPLogbook` (`bin`) | 0 | **0** | 0 | 0 | **0,00 s** | Brak testów w `src/main.rs`. |
| `splogbook_signer` (`bin`) | 0 | **0** | 0 | 0 | **0,00 s** | Brak testów w `src/bin/splogbook_signer.rs`. |
| Doctesty (`cargo test --doc`) | 0 | **0** | 0 | 0 | **6,91 s** | Brak doctestów w komentarzach `///`. |
| **RAZEM** | **1 529** | **1 519** | **0** | **10** | **~566,62 s** | 100% aktywnych testów przechodzi (`EXIT=0`). |

---

## 2. Krytyczna Ocena Jakości Istniejących Testów

### 2.1. Mocne strony obecnego zestawu testowego
1. **Strażnicy architektoniczni i lokalizacyjni (`tests/i18n_guard.rs`, `tests/theme_guard.rs`)**:
   - Skutecznie blokują dodanie klucza tłumaczeń tylko do jednego języka, niezbalansowanych nawiasów `{param}`, polskich znaków diakrytycznych w kodzie `src/gui/` oraz powrotu do `Result<T, String>`.
2. **Testy maszyn stanowych `TxArbiter`, `RigHub` i protokołu `sync`**:
   - Bardzo dobre pokrycie złośliwych ramek kryptograficznych (manipulacja AAD, replay licznika, odwrócenie kierunku Host/Peer, przekroczenie rozmiaru 6 MiB) oraz wirtualnego czasu (`#[tokio::test(start_paused = true)]`) dla timeoutów bezczynności.
3. **Testy strumieniowe kodeków CAT (`icom`, `kenwood`, `yaesu`, `elecraft`, `flexradio`)**:
   - Sprawdzają fragmentację bajt po bajcie, sklejone ramki, błędy `NAK`/`?;` oraz nieprawidłowy UTF-8.

### 2.2. Słabości i antywzorce w istniejących testach
1. **Testy utrwalające błędną specyfikację domeny lub protokołu („bug-locking tests”)**:
   - **Icom CI-V BCD (`src/cat/native/icom/codec.rs`)**: Istniejące testy S-metra i mocy zostały napisane pod błędne założenie Little-Endian BCD (`[0x20, 0x01]` zamiast rzeczywistego `[0x01, 0x20]` wysyłanego przez transceivery Icom), przez co testy przechodzą, ale prawdziwe radio Icom pokazuje `S9+60dB` (`CAT-004`).
   - **Cabrillo (`src/core/cabrillo.rs:106-140`)**: Test jednostkowy sprawdza wygenerowany nagłówek i obecność znaku, ale nie weryfikuje zachowania `rst_sent`, emisji cyfrowych (`RY`/`DG`) ani wymiany `srx_string` (`FUNC-001`).
2. **Testowanie metod inherentnych zamiast polimorficznego interfejsu `dyn RigBackend`**:
   - Testy `probe()` dla `IcomCiVBackend`, `YaesuBackend`, `ElecraftBackend`, `FlexRadioBackend` i `TciBackend` wywołują bezpośrednio metodę struktury `backend.probe().await`, nie wykrywając, że wywołanie przez obiekt traitu `(Box::new(backend) as Box<dyn RigBackend>).probe().await` (używane produkcyjnie w `RigProbeRunner`) uruchamia domyślną metodę `connect()` wysyłającą `AI1;`/`AI2;` (`CAT-003`)!
3. **Wąskie gardło wydajnościowe w `src/sync/mod.rs` (`TEST-001` / `PERF-001`)**:
   - 4 testy w `src/sync/mod.rs` zajmują **97% czasu wykonania wszystkich testów jednostkowych** (~517 s z 532 s) z powodu kwadratowej serializacji JSON w `src/sync/catchup.rs` oraz 33 wywołań Argon2id w trybie `opt-level = 0`.
4. **Zahardkodowane liczniki i SHA w `src/cat/hil/evidence.rs:339-366`**:
   - `HilSoftwareBaselineEvidence::canonical_current()` sprawdza w testach zahardkodowany commit `e1ff373b5d264f3f3de2383c19936f45b2befe6a` i statyczne liczby testów zamiast aktualnego stanu repozytorium.

---

## 3. Specyfikacja Brakujących Testów Regresyjnych (Do Wdrożenia)

Poniższa tabela definiuje konkretne testy regresyjne wymagane do zamknięcia wykrytych defektów:

| ID Znaleziska | Nazwa testu | Docelowa lokalizacja | Wejście / Warunki początkowe | Oczekiwany rezultat (Kryterium „Pass”) |
|---|---|---|---|---|
| `SEC-001` | `updater_trusted_keys_reject_known_test_and_trivial_seeds` | `src/cloud/updater.rs` | Wyliczenie kluczy publicznych Ed25519 z ziaren `[0u8; 32]`, `[1u8; 32]`, `[42u8; 32]`, `[0xFFu8; 32]`. | Żaden klucz w `TRUSTED_KEYS` nie ma bajtów równych kluczom z trywialnych ziaren; plik `src/scratch.rs` nie istnieje. |
| `REL-001` | `updater_install_flattens_single_root_directory_and_uses_zip_on_windows` | `src/cloud/updater.rs` | Tymczasowe archiwum zawierające `SPLogbook-Linux-x86_64/splogbook` oraz ścieżka tymczasowa na Windows. | Plik wykonywalny trafia bezpośrednio do `base_dir/splogbook`, a plik tymczasowy na Windows kończy się na `.zip`. |
| `SEC-002` | `app_config_debug_redacts_all_eleven_secret_fields` | `src/core/station.rs` | `AppConfig` z unikalnymi markerami `"SECRET_MARKER_1..11"` we wszystkich polach haseł/tokenów. | `format!("{cfg:?}")` nie zawiera podciągu `"SECRET_MARKER_"`, lecz zawiera `"[REDACTED]"`. |
| `SEC-003` | `sync_host_times_out_silent_tcp_client_and_frees_handshake_permit` | `src/sync/mod.rs` (`start_paused = true`) | `SyncHost` + 5 połączeń `TcpStream`, które nic nie wysyłają; przesunięcie czasu o `11 s`. | `host.available_handshake_permits()` wraca z `0` do `MAX_HANDSHAKES` (`5`). |
| `DATA-001` | `adif_import_rejects_missing_date_or_time_and_preserves_utf8_char_len` | `src/core/adif.rs` | 1) `<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<EOR>`; 2) `<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20261018<TIME_ON:6>120000<QTH:6>Kraków<EOR>`. | 1) Rekord 1 trafia do `report.rejected == 1` (brak fabrykowania `Utc::now()`); 2) Rekord 2 ma `qth == Some("Kraków".into())`. |
| `DATA-002` | `import_batch_normalizes_fields_and_infers_band_from_frequency` | `src/core/ingest/mod.rs` | `QsoRecord` z `callsign: " sp6ina "`, `band: ""`, `freq: Some(14.074)`, `qso_date: "2026-10-18"`, `time_on: "12:30:00"`, `ImportOptions { enrich: false, .. }`. | `import_batch` wstawia 1 rekord ze znormalizowanym `callsign == "SP6INA"`, `band == "20m"`, `qso_date == "20261018"`, `time_on == "123000"`. |
| `DATA-003` | `search_qsos_advanced_handles_hyphenated_date_range` | `src/core/database/mod.rs` | Rekord w bazie z `qso_date = "20261018"`; filtr `date_from = Some("2026-10-01")`, `date_to = Some("2026-10-31")`. | `search_qsos_advanced` zwraca 1 rekord. |
| `DATA-004` | `find_duplicates_respects_journal_id_and_delete_supports_undo` | `src/gui/app_tests.rs` | Identyczne QSO w dziennikach `"DEFAULT"` i `"POTA"`; usunięcie zaznaczonego QSO przez `delete_selected_qso()` + `perform_undo()`. | `find_duplicate_qsos("DEFAULT")` zwraca `0` duplikatów; po `perform_undo()` usunięte QSO wraca do bazy i `AwardsEngine`. |
| `DATA-005` | `switching_active_journal_rebuilds_awards_engine` | `src/gui/app_tests.rs` | Dziennik `"J1"` z łącznością `W1AW` (DXCC 291) i pusty dziennik `"J2"`. Przełączenie na `"J2"`. | W dzienniku `"J2"` sprawdzenie `W1AW` w `AwardsEngine` zwraca `is_new_dxcc == true` (ATNO). |
| `FUNC-001` | `cabrillo_export_renders_actual_rst_digital_modes_and_srx_string` | `src/core/cabrillo.rs` | Dwa QSO: 1) `mode: "FT8"`, `rst_sent: "-10"`, `rst_rcvd: "-05"`, `stx_string: "15"`, `srx_string: "14"`; 2) `mode: "CW"`, `rst_sent: "579"`, `srx_string: "D"`. | Wyjście Cabrillo zawiera tryb `DG` z raportami `-10` / `-05` i wymianą `15` / `14` oraz tryb `CW` z `579` i wymianą `D` w porządku chronologicznym. |
| `FUNC-003` | `rest_api_post_rig_control_forwards_commands_to_cat_runtime` | `src/api/server.rs` | Żądanie `POST /api/v1/rig` z `{"frequency_hz": 7074000, "mode": "FT8", "ptt": true}` przy podłączonym mockowanym `CatRuntime`. | Mockowany `RigBackend` odbiera `SetFrequency(7_074_000)`, `SetMode("FT8")` oraz `SetPtt(true)` zaaprobowane przez `TxArbiter`. |
| `FUNC-005` | `snapshot_config_preserves_all_modified_sections` | `src/gui/app_tests.rs` | Zmiana `gui.theme.preset`, `gui.windows.show_operator_assistant`, `config.cw.wpm`, `config.runtime.rest_api_port`, `config.solar.band_alert_enabled`. | `app.snapshot_config()` zwraca `AppConfig` zawierający wszystkie zmodyfikowane wartości. |
| `CAT-001` | `rig_service_reconnects_after_poll_error_in_degraded_state` | `tests/cat_native_6a.rs` | Mockowany `RigBackend`, który w drugim `poll()` zwraca `Err(RigError::ConnectionLost)`, ale akceptuje kolejne `connect()`. | `RigService` przechodzi `Ready -> Degraded -> Ready`, a licznik wywołań `connect()` wynosi `2`. |
| `CAT-002` | `tci_and_flexradio_set_ptt_await_delayed_tx_confirmation` | `tests/cat_native_6a.rs` | Serwer TCI / SmartSDR odsyłający potwierdzenie `trx:0,true;` / `interlock state=TRANSMITTING` po 25 ms od otrzymania komendy. | `RigHub::request_tx` kończy się sukcesem i ustawia `TxArbiterState::Owned` (brak fałszywego `Unknown`). |
| `CAT-003` | `polymorphic_box_dyn_rig_backend_probe_is_read_only_for_all_backends` | `tests/cat_native_6b.rs` | Wywołanie `(Box::new(backend) as Box<dyn RigBackend>).probe().await` dla wszystkich 8 backendów na transporcie rejestrującym zapisane bajty. | Zapisane bajty nie zawierają `AI1;`, `AI2;`, `K31;`, `K41;` ani `sub `. |
| `CAT-004` | `icom_civ_smeter_and_power_bcd_roundtrip_big_endian` | `src/cat/native/icom/codec.rs` | Ramka S-metra `[0x01, 0x20]` (`0120`) i `SetPower(50)` (`0128`). | `decode_smeter` zwraca `120` (`S9`), a `SetPower(50)` generuje bajty `[0x01, 0x28]`. |
| `CAT-005` | `yaesu_and_elecraft_set_ptt_require_hardware_tx_query_response` | `tests/cat_native_6a.rs` | Mockowany transport Yaesu/Elecraft zwracający `TX0;` / `TQ0;` (lub `?;`) w odpowiedzi na `TX1;` / `TX;`. | `execute(SetPtt(true))` zwraca `Err` lub `poll()` zwraca `ptt == false`, uniemożliwiając przejście `TxArbiter` w `Owned`. |
| `CAT-006` | `resync_tx_arbiter_rejects_stale_state_without_new_poll` | `tests/cat_native_6a.rs` | Wymuszenie `TxArbiterState::Unknown`, wywołanie nieudanej komendy podbijającej `last_update` bez nowego `last_state_update`, wywołanie `resync_tx_arbiter`. | `resync_tx_arbiter` zwraca `Err(RigError::SafetyInterlock)` i utrzymuje stan `Unknown`. |
| `CAT-015` | `hil_execute_controlled_ptt_enforces_env_gates_and_confirmed_rx` | `tests/cat_native_7.rs` | Wywołanie `HilHardwareSession::execute_controlled_ptt` przy braku `SPLOG_HIL_ALLOW_TX=1` oraz przy backendzie pozostającym w `ptt: true` po `SetPtt(false)`. | W pierwszym przypadku blokada autoryzacji; w drugim błąd `SafetyInterlock` i `rx_restored_cleanly == false`. |
| `PERF-001` | `catchup_chunking_15k_identities_runs_in_linear_time_under_200ms` | `src/sync/mod.rs` | 15 000 rekordów `QsoIdentity` przekazywanych do `chunk_inventory` i `chunk_requests`. | Czas wykonania w profilu `test` poniżej `200 ms`, każdy chunk $\le 1\text{ MiB}$. |
| `GUI-001` | `ctrl_shortcuts_trigger_while_callsign_input_has_focus` | `src/gui/app_tests.rs` | Skupienie fokusu na polu `callsign` i symulacja zdarzenia `Ctrl+W` oraz `Ctrl+L`. | `Ctrl+W` czyści formularz `entry`, a `Ctrl+L` wywołuje zapis łączności. |
