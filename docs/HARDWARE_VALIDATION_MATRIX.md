# Macierz Walidacji Sprzętowej i Protokołowej (CAT / Native / HIL / Akcesoria)

**Zakres:** trwała wiedza protokołowa i wymagania sprzętowe dla 8 backendów CAT.
**Projekt:** SPLogbook (linia rozwojowa `1.3.0-alpha.1`)
**Gałąź referencyjna:** `main`
**Cel dokumentu:** Rozgraniczenie elementów podsystemu radiowego pomiędzy:
1. elementy zweryfikowane testami automatycznymi w repozytorium,
2. elementy zweryfikowane z użyciem mocków/emulatorów transportu (`MockSerialTransport`, lokalny serwer TCP/WebSocket),
3. elementy wymagające fizycznego sprzętu radiowego (Hardware-in-the-Loop, Stage 7B) oraz zidentyfikowane blokery uniemożliwiające obecnie poprawną walidację sprzętową.

---

## 0. Jak czytać tę macierz

Kolumna „Wykryte i naprawione defekty protokołu” jest **rekordem historycznym**: każdy
wymieniony defekt (`CAT-00x`) został naprawiony w kodzie i jest strażony testami w
`tests/cat_native_*` oraz `src/cat/**`. Nie opisuje ona bieżącego stanu blokad.

Pozostającą blokadą walidacji sprzętowej jest **dostępność fizycznych transceiverów**.
Otwarte znalezisko **F-14 (P0)** — panika w ścieżce HIL po załączeniu PTT — zostało
**zamknięte programowo** 2026-11-15 (`STAGE-P0-F14-SOFTWARE`): odwrócony zakres jest odrzucany
przed logicznym PTT ON, a każda kontrolowana ścieżka błędu po ON podejmuje próbę PTT OFF.
To **nie** jest walidacja sprzętowa: `HARDWARE_BEHAVIOUR_NOT_VERIFIED`, a wszystkie 10 testów
pozostaje `#[ignore = "hardware-in-the-loop"]`, dopóki NATIVE-7B nie zostanie jawnie
autoryzowane przez właściciela.

Kolumnę „Co wymaga fizycznego transceivera” należy czytać jako listę wymagań sprzętowych,
a nie jako listę zadań do wykonania w repozytorium.

---

## 1. Macierz Backendów CAT i Protokołów Radiowych (8 Backendów)

| Backend / Rodzina | Implementacja w kodzie | Pokrycie testami jednostkowymi i mockami | Wykryte defekty protokołowe / bezpieczeństwa (Blokery przed HIL) | Co wymaga fizycznego transceivera (Stage 7B HIL) | Status walidacji sprzętowej |
|---|---|---|---|---|---|
| **1. Hamlib (`rigctld`)** | `src/cat/hamlib.rs`<br>`src/cat/supervisor.rs` | Wysokie (mock TCP serwera `rigctld`, parsowanie `RPRT`, `f`, `m`, `s`, `t`, `l STRENGTH`, `l RFPOWER`). | - `CAT-001`: Brak automatycznego reconnectu w `RigService::run_loop` po zerwaniu gniazda TCP.<br>- `CAT-009`: `poll_internal` nie odpytuje `v` (VFO), `i` (Split TX freq) ani `j`/`z` (RIT/XIT).<br>- `CAT-007`: `start_cat_service` ignoruje `HamlibConfig::auto_start_rigctld` z `station_cfg.rigs`. | - Czas startu procesu `rigctld` z fizycznym konwerterem USB-UART.<br>- Zachowanie `rigctld` przy odłączeniu kabla USB w trakcie sesji. | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **2. FLRig (XML-RPC)** | `src/cat/flrig.rs` | Średnie (testy budowania/parsowania XML-RPC i mock HTTP). | - `CAT-010`: Brak HTTP Keep-Alive (`Connection: close` + `TcpStream::connect` na każde wywołanie XML-RPC = 20 gniazd TCP/s, ryzyko wyczerpania portów `TIME_WAIT` na Windows).<br>- Okrojone `RigCapabilities` (wyłączone `split`, `vfo`, `passband`, `power`). | - Rzeczywista współpraca z uruchomioną aplikacją FLRig 2.0.x pod obciążeniem ciągłego pollingu (200 ms). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **3. TCI (WebSocket)** | `src/cat/tci.rs` | Wysokie na poziomie parsera ramek (`vfo:`, `modulation:`, `trx:`, `rx_smeter:`, `tx_power:`). | - `CAT-002` **[KRYTYCZNY]**: Wyścig potwierdzenia PTT — `SetPtt(true)` nie czeka na echo `trx:0,true;`, a kolejne `poll()` czeka tylko `5 ms` (`short_poll_window`), przez co odpowiedź serwera TCI po >5 ms powoduje natychmiastowe przejście `TxArbiter` w `Unknown` i awaryjne zrzucenie PTT.<br>- `CAT-003`: `TciBackend::probe` nie jest w `impl RigBackend`.<br>- `SetVfo` nie wysyła komendy sieciowej. | - Latencja odpowiedzi `trx:0,true;` w ExpertSDR3 (SunSDR2 DX/PRO) oraz Thetis (ANAN/Hermes-Lite 2). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **4. Icom CI-V (Natywny)** | `src/cat/native/icom/codec.rs`<br>`src/cat/native/icom/backend.rs`<br>`src/cat/native/icom/models.rs` | Wysokie dla ramkowania `0xFE 0xFE ... 0xFD`, kolizji, echa Half-Duplex i częstotliwości 5-bajtowej BCD. | - `CAT-004` **[KRYTYCZNY]**: Odwrócona kolejność bajtów BCD (Little-Endian zamiast Big-Endian) w `decode_smeter` (`0x15 0x02`), `decode_power_watts` (`0x15 0x11`) oraz `SetPower` (`0x14 0x0A`) — S-meter/moc zawyżane do `255` (`S9+60dB`), a `SetPower` wysyła niepoprawne BCD odrzucane ramką `NAK 0xFA`.<br>- `CAT-011`: `SetRit`/`SetXit` wysyłają 1 bajt `[0x01]` do subkomendy `0x21 0x00` (oczekującej 3 bajtów offsetu) zamiast `0x21 0x01`/`0x21 0x02` i ignorują offset `hz`.<br>- `CAT-011`: `SetSplitFrequency` wysyła `0x05` (aktywne VFO RX) zamiast VFO nadawczego.<br>- `CAT-003`: `probe()` poza `impl RigBackend`.<br>- `CAT-016`: `HilTestBenchManifest::to_rig_config` wymusza adres `0x94` dla wszystkich modeli Icom. | - Fizyczny kabel CI-V z echem sprzętowym (1-wire CI-V) oraz port USB w IC-7300 (`0x94`), IC-7610 (`0x98`), IC-705 (`0xA4`), IC-9700 (`0xA2`), IC-706MKIIG (`0x58` — 4-bajtowy brak odczytu PTT). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **5. Kenwood (Natywny)** | `src/cat/native/kenwood/codec.rs`<br>`src/cat/native/kenwood/backend.rs`<br>`src/cat/native/kenwood/models.rs` | Wysokie dla `FA;`, `FB;`, `MD;`, `IF;`, `SM0;`, `TX;`, `RX;` oraz read-only `probe()` (jedyny natywny backend z `probe` w `impl RigBackend`). | - `CAT-012`: `roundtrip_command` w setterach (`FA`, `MD`, `TX`, `RX`, `FT`, `FR`) czeka na echo z radia. W Kenwood settery nie odsyłają odpowiedzi, gdy `AI2;` nie zaemituje zmiany (np. ustawienie tej samej emisji lub wyłączone AI), co powoduje timeout `1500 ms` i blokadę `TxArbiter` przy `SetPtt`.<br>- `CAT-012`: Profile deklarują `passband: true, rit: true, xit: true, power: true`, ale `execute()` zwraca dla nich `Err(RigError::Unsupported)`. | - Weryfikacja na TS-590S/SG, TS-890S, TS-990S (różnice w formacie `IF;`, `MD` vs `OM` w TS-890S). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **6. Yaesu CAT (Natywny)** | `src/cat/native/yaesu/codec.rs`<br>`src/cat/native/yaesu/backend.rs`<br>`src/cat/native/yaesu/models.rs` | Wysokie dla ramek ASCII `;` (`FA`, `FB`, `MD0`, `IF`, `SM0`, `TX`, `FT`, `?;`) i profili `FT-891`, `FT-991A`, `FT-710`, `FTDX10`. | - `CAT-003` **[KRYTYCZNY]**: `YaesuBackend::probe` nie nadpisuje `RigBackend::probe`, więc polimorficzny Probe wywołuje `connect()` i wysyła `AI1;` (naruszenie zasady Read-Only Probe R10).<br>- `CAT-005` **[WYSOKI]**: `SetPtt` optymistycznie ustawia `confirmed_ptt = *ptt` bez odpytania `TX;`, a `poll()` przy `ai_active == true` odpytuje tylko `SM0;`, zwracając niepotwierdzone PTT do `TxArbiter`!<br>- `SetRit`/`SetXit` ignorują offset `hz`. | - Rozróżnienie `Enhanced COM Port` vs `Standard COM Port` (CP2105) na fizycznym FT-891, FT-991A, FT-710, FTDX10.<br>- Zachowanie przy odrzuceniu `TX1;` (`?;` poza pasmem). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **7. Elecraft (Natywny)** | `src/cat/native/elecraft/codec.rs`<br>`src/cat/native/elecraft/backend.rs`<br>`src/cat/native/elecraft/models.rs` | Wysokie dla `K3;`, `K31;`, `K4;`, `OM;`, `FA`, `FB`, `MD`, `DT`, `TQ`, `PC`, `BW`, `SM`, `FT`, `FR`, `SB`, `RO`. | - `CAT-003` **[KRYTYCZNY]**: `ElecraftBackend::probe` nie nadpisuje `RigBackend::probe`, więc polimorficzny Probe wysyła `AI2;`/`K31;`/`K41;`.<br>- `CAT-005` **[WYSOKI]**: `SetPtt` optymistycznie ustawia `confirmed_ptt = *ptt` bez weryfikacji `TQ;`, a `poll()` ignoruje błąd `let _ = self.query_and_await_response("TQ;", "TQ").await;`. | - Walidacja na fizycznych K3, K3S, K4, KX3, KX2 (w tym obsługa sub-odbiornika `KRX3` i trybów `DATA A` / `AFSK A`). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |
| **8. FlexRadio SmartSDR** | `src/cat/native/flexradio/codec.rs`<br>`src/cat/native/flexradio/backend.rs`<br>`src/cat/native/flexradio/models.rs` | Wysokie dla parsera linii `V`, `H`, `S<handle>|<topic>`, `R<seq>|<code>|`, `M` oraz maszyny stanów slice'ów i interlocka. | - `CAT-002` **[KRYTYCZNY]**: Wyścig potwierdzenia PTT pomiędzy `R<seq>|0|` (odpowiedź na `xmit 1`) a asynchronicznym statusem `S...|interlock state=TRANSMITTING` — jeśli `poll()` zakończy się przed nadejściem linii `interlock`, `TxArbiter` wchodzi w `Unknown`.<br>- `CAT-003`: `FlexRadioBackend::probe` nie nadpisuje `RigBackend::probe` (wysyła `sub slice all`).<br>- `CAT-013`: Settery nie wywołują `wait_for_response(seq)`, a `SetSplit` jest pustym blokiem (`no-op`). | - Rzeczywisty serwer SmartSDR TCP (`:4992`) na serii FLEX-6400 / 6600 / 6700 / 8400 / 8600 w środowisku MultiFLEX (weryfikacja `tx_client_handle`). | **NIEWYKONANA** (wymaga fizycznego transceivera; HIL Stage 7B nieuruchomiony) |

---

## 2. Macierz Podsystemów Towarzyszących (Arbitraż TX, HIL, Proxy, Rotator, WinKeyer, Audio)

| Komponent | Lokalizacja | Co zostało przetestowane automatycznie | Wykryte luki i błędy w kodzie | Co wymaga walidacji ze sprzętem / środowiskiem zewnętrznym |
|---|---|---|---|---|
| **`TxArbiter` i `RigHub::resync_tx_arbiter`** | `src/cat/tx_arbiter.rs`<br>`src/cat/hub.rs:888-908` | Pełne testy automatów stanowych (`Idle -> Claiming -> Owned -> Releasing -> Idle` oraz wejścia w `Unknown`). | - `CAT-006` **[WYSOKI]**: `RigHub::resync_tx_arbiter` przekazuje `snapshot.last_update` (aktualizowane przy przejściu stanu lub odrzuconej komendzie) zamiast `snapshot.last_state_update`, co pozwala odblokować `Unknown` na podstawie przestarzałego stanu sprzed awarii.<br>- Wariant `StateFreshness::Stale` nigdy nie jest ustawiany w `RigService`. | - Testy odcięcia zasilania/kabla USB w trakcie nadawania (`Owned -> Unknown`) i ręcznego odzyskania po przywróceniu odbioru (`ConfirmedRx`). |
| **Wykonawca HIL (`HilHardwareSession`)** | `src/cat/hil/session.rs`<br>`src/cat/hil/manifest.rs` | Testy bramek środowiskowych `SPLOG_HIL_*`, `HilTxSafetyGuard` (`acquire_tx_guard`), macierzy `HilCheckId` i generowania raportów. | - `CAT-015` **[WYSOKI]**: `HilHardwareSession::execute_controlled_ptt` (`session.rs:630-684`) omija `verify_hardware_access_authorization()`, omija `TxArbiter`, nie sprawdza początkowego RX i nie wywołuje `poll()` po `SetPtt(false)`, bezwarunkowo ustawiając `rx_restored_cleanly = true`.<br>- `run_isolated_hardware_dispatcher_at` (`session.rs:730-844`) jest zaślepką zwracającą `Blocked` / `NotRun` nawet dla `Stage7BHardware`. | - Fizyczne wykonanie 10 przypadków testowych Stage 7B ze sztucznym obciążeniem 50 Ω (`Dummy Load`) po naprawie `execute_controlled_ptt`. |
| **Hamlib TCP Proxy (`:4534`)** | `src/cat/server.rs`<br>`src/gui/app_async.rs:258-307` | Testy komend protokołu `rigctld` (`f`, `F`, `m`, `M`, `t`, `T`, `s`, `S`, `\dump_state`, `\chk_vfo`). | - `CAT-008` **[WYSOKI]**: Optymistyczna mutacja `shared_state` i zwrot `RPRT 0` przed przekazaniem `T 1` do `TxArbiter` przez pętlę GUI.<br>- Pusta gałąź passbandu w `M <mode> <passband>` (`server.rs:165`). | - Równoczesna praca z WSJT-X / JTDX podłączonym jako `Hamlib NET rigctl` do `127.0.0.1:4534`. |
| **Rotator (`RotorClient` / `rotctld`)** | `src/cat/rotor.rs`<br>`src/gui/app_cat.rs:10-46`<br>`src/gui/astronomy_dialog.rs:233-236` | Testy formatowania komend `P <az> <el>`, `S`, `p` na mocku TCP. | - `RotorClient::get_position` nigdy nie jest wywoływany cyklicznie (brak pollera pozycji anteny).<br>- Przyciski śledzenia Księżyca/Słońca w `astronomy_dialog.rs:233-236` mutują tylko pola w pamięci i nie wywołują `rotate_antenna_to_el`. | - Współpraca z `rotctld` i fizycznym sterownikiem rotatora (Yaesu GS-232A/B, AlfaSpid, Prosistel). |
| **Kluczowanie CW (`Winkeyer` / Terminal CW / Makra)** | `src/cat/winkeyer.rs`<br>`src/gui/cw_terminal.rs`<br>`src/gui/app_plugins.rs:215-225` | Tylko testy jednostkowe kodowania tablic bajtów `WinkeyerProtocol` i bufora `CwTerminalBuffer`. | - `FUNC-004` **[WYSOKI]**: Całkowity brak sterownika I/O portu szeregowego dla WinKeyera (`WinkeyerProtocol` nie jest nigdzie wywoływany poza testem jednostkowym). Terminal CW i makra F1–F8 w GUI to wyłącznie wizualna symulacja. | - Po dopisaniu transportu `tokio-serial`: walidacja na fizycznym kluczu K1EL WinKeyer USB (WK2 / WK3). |
| **Voice Keyer i Nagrywanie Audio** | `src/gui/voice_keyer.rs`<br>`src/media/audio.rs`<br>`src/gui/qso_entry.rs:646` | Testy ścieżek plików WAV i inicjalizacji stanu. | - `CAT-008`: `VoiceKeyer::transmit_message` nie czeka na potwierdzenie `TxArbiterState::Owned` przed odtworzeniem pliku WAV.<br>- `FUNC-004`: Przycisk STOP nagrywania w `qso_entry.rs:646` wywołuje `stop_audio()` zamiast `stop_recording()`. | - Dobór karty dźwiękowej TX/RX (kodek USB w transceiverze vs głośniki systemowe) — obecnie `rodio`/`cpal` otwiera wyłącznie domyślne urządzenie systemowe. |

---

## 3. Minimalny Zestaw Warunków Wejścia do Fizycznych Testów Sprzętowych (Stage 7B)

Przed podłączeniem fizycznego transceivera do procedury HIL należy obowiązkowo zamknąć następujące zadania programowe:
1. Przenieść implementacje `probe()` do `impl RigBackend` w backendach `IcomCiV`, `Yaesu`, `Elecraft`, `FlexRadio` oraz `Tci` (`CAT-003`).
2. Naprawić potwierdzanie PTT w `TciBackend`, `FlexRadioBackend`, `YaesuBackend`, `ElecraftBackend` i `KenwoodBackend` (`CAT-002`, `CAT-005`, `CAT-012`).
3. Naprawić kodowanie Big-Endian BCD dla S-metra/mocy oraz komendy `0x21` (RIT/XIT) w `IcomCiVBackend` (`CAT-004`, `CAT-011`) oraz rezolucję adresu CI-V w `HilTestBenchManifest::to_rig_config` (`CAT-016`).
4. Naprawić `RigHub::resync_tx_arbiter` (`last_state_update` + `StateFreshness::Stale`, `CAT-006`).
5. Zabezpieczyć `HilHardwareSession::execute_controlled_ptt` poprzez pełne użycie `acquire_tx_guard` i `release_and_confirm_rx` (`CAT-015`).

---

## 4. Macierz zgodności protokołów — weryfikacja programowa (P0-F14, 2026-11-15)

Sekcja dodana przez `STAGE-P0-F14-SOFTWARE`. Zapisuje **programową** weryfikację ramek PTT
wobec oficjalnej dokumentacji producentów. Nie jest dowodem zachowania sprzętu.

Flagi tej sekcji:

- `SOFTWARE_ONLY_VERIFICATION=true`
- `HARDWARE_BEHAVIOUR_NOT_VERIFIED=true`
- `TEST_DOUBLE_PRODUCTION_EXCLUSION_VERIFIED=true`
- `REAL_PTT_EXECUTED=false`, `RF_TRANSMISSION_EXECUTED=false`, `HIL_EXECUTED=false`

Legenda statusów: `OFFICIAL_MODEL_SPECIFIC_VERIFIED`, `OFFICIAL_FAMILY_LEVEL_VERIFIED`,
`OFFICIAL_PROTOCOL_LEVEL_VERIFIED`, `DOCUMENTATION_NOT_FOUND`.

| Producent | Model / rodzina | Backend | Transport | PTT ON | PTT OFF | Dokument oficjalny (rewizja) | Odpowiedź / potwierdzenie | Timeout | Cleanup po ON | Test | Status dokumentacyjny |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Icom | IC-7610 | `IcomCiV` | Serial CI-V | `1C 00 01` | `1C 00 00` | CI-V Reference Guide **A7380-7EX-4**, Sep. 2025 | ramka statusu `1C 00 <00/01>`; `*1` auto-output tylko gdy `1C 04`=ON | `write_timeout`/`read_timeout` konfiguracji | `RX` + `poll()` (ConfirmedRx) | `protocol_conformance`, `cat_f14_fail_safe` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Icom | IC-7300 | `IcomCiV` | Serial CI-V | `1C 00 01` | `1C 00 00` | 補足説明書 CI-V **A7290-2J-8**, 2023-12 | ramka `1C` (00=受信, 01=送信) | j.w. | j.w. | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Icom | IC-705 | `IcomCiV` | Serial CI-V | `1C 00 01` | `1C 00 00` | CI-V Reference Guide **A7560-8EX-4b**, Mar. 2021 (dokument pobrany i zidentyfikowany; wiersz tabeli nieodczytany — PDF AES-256) | j.w. (poziom protokołu CI-V) | j.w. | j.w. | `protocol_conformance` | `DOCUMENTATION_NOT_FOUND` (treść wiersza) / protokół zweryfikowany na IC-7610 i IC-7300 |
| Icom | IC-706MKIIG | `IcomCiV` | Serial CI-V | `1C 00 01` (protokół CI-V) | `1C 00 00` | instrukcja modelu niezlokalizowana w tej bramce | j.w. | j.w. | j.w. | — | `DOCUMENTATION_NOT_FOUND` |
| Yaesu | FT-891 | `Yaesu` | Serial | `TX1;` | `TX0;` | CAT Operation Reference **1909-C** (2019) | Answer `TX P1;` (P1=1 TX, 0 RX, 2 „RADIO TX ON, CAT TX OFF”) | brak reguły w dokumencie; `CAT TOT` 10 ms (menu) | `TX0;` + `TX;` (ConfirmedRx) | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Yaesu | FT-991A | `Yaesu` | Serial | `TX1;` | `TX0;` | CAT Operation Reference **1711-D** (2017) / **1808-D** (DG-ID) | j.w. | j.w. | j.w. | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Yaesu | FT-710 | `Yaesu` | Serial (CAT-1) | `TX1;` | `TX0;` | CAT Operation Reference **2306-C** (2023) | j.w. + AI push po zmianie stanu | j.w. | j.w. | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Yaesu | FTDX10 | `Yaesu` | Serial (Enhanced COM) | `TX1;` | `TX0;` | CAT Operation Reference **2308-F** (2023) | j.w. | j.w. | j.w. | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Kenwood | TS-590S / TS-590SG | `Kenwood` | Serial / USB | `TX;` | `RX;` | PC CONTROL COMMAND **B5A-0316-20/01** (2019-01-30) | odpowiedź tylko gdy AI ON; status `IF` P8 (0=RX, 1=TX) | brak reguły w dokumencie | `RX;` + status | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` |
| Kenwood | TS-890S | `Kenwood` | Serial / LAN | `TX;` | `RX;` | PC CONTROL COMMAND **B5A-2944-00/01** (2019-01-30) | odpowiedź tylko gdy AI ON; brak `IF` w referencji | brak reguły | `RX;` | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` (keying) / `DOCUMENTATION_NOT_FOUND` (odczyt statusu) |
| Kenwood | TS-990S | `Kenwood` | Serial / LAN | `TX;` | `RX;` | PC CONTROL COMMAND **B5A-3170-00** (2019-01-30) | j.w. | brak reguły | `RX;` | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` (keying) / `DOCUMENTATION_NOT_FOUND` (odczyt statusu) |
| Elecraft | K3 / K3S / KX3 / KX2 | `Elecraft` | Serial | `TX;` | `RX;` | Programmer's Reference **Rev. G5** (2019-02-20) | `TQ;` → `TQ0;`/`TQ1;` | dokument: typowa odpowiedź <10 ms, najgorsza ~100 ms; brak reguły retry | `RX;` + `TQ;` | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` (K3) / `OFFICIAL_FAMILY_LEVEL_VERIFIED` (K3S/KX3/KX2) |
| Elecraft | K4 / K4D / K4HD | `Elecraft` | TCP (K4) | `TX;` | `RX;` | K4 Programmer's Reference **Rev. D12** (2026-05-04) | `TQ;` → `TQ1;` (TQX od Rev. C10) | PING 1 s / drop po 10 s | `RX;` + `TQ;` | `protocol_conformance` | `OFFICIAL_MODEL_SPECIFIC_VERIFIED` (K4) / `OFFICIAL_FAMILY_LEVEL_VERIFIED` (K4D/K4HD) |
| FlexRadio | FLEX-6400/6600/8400/8600M | `FlexRadio` | TCP 4992 | `xmit 1` | `xmit 0` | SmartSDR TCP/IP API, `TCPIP xmit` rev `177f09f` (2024-07-01) + `interlock` status (2026-10-06) | `R<seq>|0|`; stan wyłącznie przez `S…|interlock state=… tx_allowed=…` | keepalive 1 s / 15 s | `xmit 0` + interlock RX | `protocol_conformance` | `OFFICIAL_FAMILY_LEVEL_VERIFIED` |
| Expert Electronics | TCI (SunSDR / Thetis) | `Tci` | WebSocket | `trx:0,true;` | `trx:0,false;` | TCI Protocol **2.1** (Sep. 2026) | `TRX:arg1,arg2;` (broadcast 30 ms tick; 200 ms monopoly) | brak wymaganego ack/timeout w specyfikacji | `trx:…,false;` + potwierdzenie | `protocol_conformance` | `OFFICIAL_PROTOCOL_LEVEL_VERIFIED` |
| Hamlib | `rigctld` (dowolny model backendu) | `Hamlib` | TCP | `T 1` | `T 0` | `rigctld(1)` @ **4.7.2** + `rig.h` | `RPRT x` (0 = OK, ujemne = kod błędu); `t` → 0/1/2/3 | brak timeoutu po stronie klienta w dokumentacji | `T 0` + `t` | `protocol_conformance` | `OFFICIAL_PROTOCOL_LEVEL_VERIFIED` |
| FLRig | FLRig 2.0.00 | `Flrig` | TCP XML-RPC 12345 | `rig.set_ptt` `<int>1</int>` | `rig.set_ptt` `<int>0</int>` | `xml_server.cxx` (metoda `n:i`), `rig.get_ptt` `i:n` | wynik `rig.set_ptt` nieokreślony; `rig.get_ptt` → `<int>` | brak timeoutu XML-RPC w dokumentacji | `rig.set_ptt 0` + `rig.get_ptt` | `protocol_conformance` (dokument) + `CAT-019` (rozbieżność) | `OFFICIAL_PROTOCOL_LEVEL_VERIFIED` |
| K1EL | WinKeyer USB (WK3) | `Winkeyer` | Serial (brak sterownika I/O — `FUNC-004`) | nie dotyczy: PTT sterowane sprzętowo przez klawer | `PTT On/Off 0x18` tylko gdy PTT wyłączone | WinKeyer3 IC Manual **Rev. 1.3** (2019-03-19), WKUSB **Rev. 1.4** (2023-09-02) | brak komendy hosta bezpośrednio załączającej PTT | tune watchdog 100 s | n/d (brak ścieżki hosta) | — | `OFFICIAL_PROTOCOL_LEVEL_VERIFIED` / `PTT_NOT_SUPPORTED` (host) |

Uwagi:

1. Parser Kenwooda mapuje `TX0;` na odbiór, podczas gdy oficjalne referencje definiują `P1=0`
   jako `SEND` (nadawanie) — zapisane jako `CAT-018` w [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md);
   ścieżka jest nieosiągalna, bo backend wysyła `RX;`.
2. Klient FLRig wysyła parametr `<boolean>` zamiast udokumentowanego `<int>` — zapisane jako
   `CAT-019`; błąd typu w FLRig kończy się błędem (fail-closed), nie transmisją.
3. Testowe implementacje protokołów istnieją wyłącznie w `tests/support/protocol_double.rs`
   i `#[cfg(test)]`; nie są kompilowane do wersji release, nie są dostępne z GUI/CLI/konfiguracji/
   środowiska i nie ma na nie automatycznego fallbacku
   (`TEST_DOUBLE_PRODUCTION_EXCLUSION_VERIFIED=true`).
