# Native CAT — Architektura natywnych protokołów radiotelefonów SPLogbook

Stan odniesienia: SPLogbook v1.2.0, commit bazowy `ca125ec739821a1fad3e7e496732581c4ae67c59`, Rust Edition 2024.

---

## 1. Cele programu

Głównym celem programu **NATIVE-CAT** jest wdrożenie bezpośredniej, asynchronicznej i niezależnej od zewnętrznych bibliotek (takich jak Hamlib) komunikacji z radiotelefonami amatorskimi. Hamlib (`rigctld`) pozostaje w pełni wspieranym, uniwersalnym backendem CAT, jednak natywne protokoły pozwalają na:
1. Bezpośrednią pracę przez porty szeregowe / USB Virtual COM bez konieczności uruchamiania zewnętrznego procesu `rigctld`.
2. Dostęp do zaawansowanych możliwości producentów (Main/Sub VFO, Dualwatch, sterowanie filtrami DSP, nastawy RIT/XIT, precyzyjne odczyty mierników, zdarzenia transceive/broadcast), których uogólniony kontrakt Hamlib nie udostępnia lub udostępnia z opóźnieniem.
3. Wyeliminowanie narzutu i niestabilności pośrednich procesów w systemach Windows, Linux i macOS.
4. Zagwarantowanie najwyższego poziomu niezawodności, determinizmu i bezpieczeństwa operacji nadawczo-odbiorczych (w szczególności rygorystyczne bezpieczeństwo PTT).

---

## 2. Zakres

Program NATIVE-CAT obejmuje:
1. **Wspólny asynchroniczny transport szeregowy** (`tokio-serial`, obsługa baud rate, bitów danych, parzystości, bitów stopu, kontroli przepływu, bezpiecznych timeoutów, strumieniowego buforowania i odporności na fragmentację).
2. **Architekturę profili urządzeń i rejestru modeli** (`RigModelProfile`, `NativeCapabilities`, precyzyjne mapy trybów i ograniczeń pasm).
3. **Wdrożenie falami natywnych backendów**:
   - **NATIVE-1**: Icom CI-V (kodowanie binarne/BCD, ramki FE FE ... FD, obsługa ACK/NAK, filtracja echa, obsługa zdarzeń transceive),
   - **NATIVE-2**: Kenwood PC Control (komendy tekstowe ASCII zakończone średnikiem, Auto Information, obsługa portów szeregowych i LAN),
   - **NATIVE-3**: Yaesu CAT (rozróżnienie rodzin klasycznych binarnych i współczesnych tekstowych ASCII),
   - **NATIVE-4**: Elecraft (rodziny K3/KX oraz K4 LAN/Ethernet),
   - **NATIVE-5**: FlexRadio SmartSDR API (TCP discovery/command API, slices, metering).
4. **Rozszerzalny model domenowy**:
   - Zachowanie pełnej wstecznej zgodności z istniejącym kontraktem `RigBackend`, `RigCommand`, `RigState`, `RigCapabilities`, `RigError`,
   - Wprowadzenie dedykowanych komend i zdarzeń producentów (`NativeRigCommand`, `NativeRigEvent`).
5. **Pełny zestaw testów automatycznych**: kodeki, symulacje duplex stream, testy odporności na błędy transportu, bezkontaktowe testy PTT oraz przygotowanie pod testy Hardware-In-The-Loop (HIL).

---

## 3. Elementy poza zakresem

1. **SO2R jako RigBackend**: SO2R nie jest i nie będzie klasą `RigBackend`. SO2R to warstwa koordynacji wyższego rzędu nad dwiema niezależnymi instancjami `RigService`.
2. **Implementacja wczesnych atrap**: Żaden protokół (np. Xiegu, JRC, Ten-Tec) nie może zostać dodany na zasadzie zgadywania komend bez oficjalnej dokumentacji i profili.
3. **Modyfikacja schematu SQLite**: Baza danych pozostaje nienaruszona (zgodnie z AD-04).
4. **Wymuszanie fizycznego sprzętu w CI**: Wszystkie testy regresyjne i jednostkowe działają deterministycznie na strumieniach pamięciowych (`tokio::io::duplex`).

---

## 4. Inwentaryzacja aktualnego kodu

W stanie wejściowym repozytorium zidentyfikowano następujące komponenty powiązane z CAT:

| Plik | Stan wejściowy | Wnioski i decyzja audytu |
|---|---|---|
| `src/cat/backend.rs` | Definiuje `RigBackend`, `CatBackendKind`, stary `CatBackend` i `CatController` | Trait `RigBackend` stanowi stabilny fundament. `CatBackendKind` wymaga rozszerzenia o nowe natywne backendy. |
| `src/cat/rig.rs` | `RigState`, `RigCommand`, `RigCapabilities`, `RigError`, `RigStatus` | Zachowujemy 100% kompatybilność. Rozszerzenia natywne w osobnym module `src/cat/native`. |
| `src/cat/icom_ci_v.rs` | Pomocnicza struktura `CiV`: build_frame, BCD freq, lookup adresów modeli | Brak dekodera strumieniowego, brak ACK/NAK, brak transportu, brak `RigBackend`. Migracja przez `git mv` do `src/cat/native/icom/` w NATIVE-1. |
| `src/cat/kenwood.rs` | Statyczne formatowanie napisów FA, FB, MD, TX | Brak transportu, brak dekodera strumieniowego. Zachowany bez zmian w NATIVE-0/1, podstawa pod NATIVE-2. |
| `src/cat/so2r.rs` | `So2rState`, `RadioSlot`, `So2rError` (blokada TX, fokus) | Czysta domena dwu-radiowa. Zachowany bez modyfikacji. |
| `src/cat/hamlib.rs` | `HamlibClient`, trwała sesja TCP do `rigctld` | Działa stabilnie, testy przechodzą w 100%. |
| `src/cat/flrig.rs` | `FlrigClient`, XML-RPC po TCP | Działa stabilnie, testy przechodzą w 100%. |
| `src/cat/tci.rs` | `TciClient`, WebSocket | Działa stabilnie, testy przechodzą w 100%. |
| `src/cat/supervisor.rs` | Nadzorca procesu `rigctld` | Zarządza uruchamianiem Hamlib dla połączeń szeregowych. |
| `src/gui/cat_settings.rs` | UI konfiguracji CAT | Obecnie pozwala na wybór Hamlib, FLRig, TCI. Nie ruszane w NATIVE-0/1. |

---

## 5. Diagram architektury

```mermaid
graph TD
    UI[Warstwa Prezentacji: GUI / CatSettings] --> Hub[RigHub / RigService]
    Hub --> Backend[RigBackend Trait]
    
    subgraph "Istniejące Backendy"
        Backend --> Hamlib[HamlibBackend / rigctld TCP]
        Backend --> Flrig[FlrigBackend / XML-RPC TCP]
        Backend --> Tci[TciBackend / WebSocket]
    end

    subgraph "Natywna Architektura Native-CAT"
        Backend --> IcomBackend[IcomCiVBackend]
        Backend --> KenwoodBackend[KenwoodBackend (NATIVE-2)]
        Backend --> YaesuBackend[YaesuBackend (NATIVE-3)]
        Backend --> ElecraftBackend[ElecraftBackend (NATIVE-4)]
        Backend --> FlexBackend[FlexRadioBackend (NATIVE-5)]
        
        IcomBackend --> IcomCodec[IcomCiVCodec (FE FE .. FD)]
        IcomBackend --> ProfileRegistry[RigModelRegistry / ModelProfile]
        IcomBackend --> SerialTransport[Async Serial Transport (tokio-serial)]

        KenwoodBackend --> KenwoodCodec[KenwoodCodec (;)]
        KenwoodBackend --> SerialTransport

        YaesuBackend --> YaesuCodec[YaesuCodec]
        YaesuBackend --> SerialTransport

        ElecraftBackend --> ElecraftCodec[ElecraftCodec (;)]
        ElecraftBackend --> SerialTransport
        ElecraftBackend --> TcpTransport[Async TCP Transport (tokio::net::TcpStream)]
    end

    subgraph "Koordynacja Multi-Radio"
        So2rCoordinator[So2rCoordinator] --> HubA[RigService Radio A]
        So2rCoordinator --> HubB[RigService Radio B]
        So2rCoordinator --> Interlock[PTT Interlock Engine]
    end
```

---

## 6. Warstwy Native-CAT

Architektura jest ściśle podzielona na 7 warstw:
1. **Domena (`src/cat/rig.rs`, `src/cat/native/mod.rs`)**:
   - Podstawowe typy niezmienne: `RigCommand`, `RigState`, `RigCapabilities`, `RigError`.
   - Typy rozszerzone: `NativeRigCommand`, `NativeRigCapabilities`, `NativeRigEvent`.
2. **Wspólny transport szeregowy (`src/cat/native/transport.rs`)**:
   - `SerialConfig` (port, baud, bity danych, stop, parzystość, flow control, timeouty).
   - `AsyncSerialTransport` implementujący asynchroniczny odczyt/zapis z buforowaniem ramkowym i odpornością na częściowe pakiety.
3. **Kodek protokołu (`src/cat/native/<vendor>/codec.rs`)**:
   - Odpowiedzialny wyłącznie za kodowanie struktur domenowych do strumienia bajtów oraz dekodowanie strumienia do struktur odpowiedzi/zdarzeń.
   - Niezależny od wejścia/wyjścia sieciowego i sprzętowego (czysta logika bez I/O).
4. **Profil modelu (`src/cat/native/profile.rs`, `registry.rs`)**:
   - Metadane urządzeń: identyfikator modelu, domyślne parametry transmisji, obsługiwane komendy, ograniczenia sprzętowe, adresy CI-V / ID producenta.
5. **Backend (`src/cat/native/<vendor>/backend.rs`)**:
   - Implementuje `RigBackend` dla danego producenta.
   - Zarządza cyklem życia połączenia, obsługuje odpytywanie (`poll`), asynchroniczne zdarzenia transceive oraz wysyłanie komend (`execute`).
6. **Usługa (`RigService` / `RigHub`)**:
   - Aktor zarządzający pętlą I/O, kolejką komend i rozsyłaniem stanu do subskrybentów za pomocą `tokio::sync::watch`.
7. **Interfejs Użytkownika (`src/gui/cat_settings.rs`)**:
   - Formularze konfiguracji specyficzne dla danego backendu (port COM, prędkość, adres CI-V, podgląd możliwości).

---

## 7. Projekt wspólnego transportu szeregowego

Wspólny transport szeregowy zlokalizowany w `src/cat/native/transport.rs` opiera się na bibliotece `tokio-serial` (wersja 5.5.0) oraz abstrakcjach `AsyncRead + AsyncWrite`.

### 7.1. Konfiguracja portu
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: SerialDataBits,
    pub stop_bits: SerialStopBits,
    pub parity: SerialParity,
    pub flow_control: SerialFlowControl,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
}
```

### 7.2. Gwarancje transportu
- **Brak operacji blokujących**: Całość operacji I/O jest wykonywana asynchronicznie w środowisku Tokio.
- **Buforowanie strumieniowe**: Bufor kołowy/liniowy o konfigurowalnym maksymalnym rozmiarze (np. 4096 bajtów), zapobiegający wyciekom pamięci.
- **Odporność na fragmentację**: Kodek odpytuje bufor o obecność pełnej ramki; jeśli ramka jest niekompletna, transport kontynuuje asynchroniczny odczyt.
- **Bezpieczne anulowanie**: Wszystkie operacje I/O mogą zostać bezpiecznie przerwane przez timeout lub anulowanie future bez uszkodzenia stanu wewnętrznego.
- **Testowalność bez sprzętu**: Warstwa transportowa przyjmuje generyczny strumień `T: AsyncRead + AsyncWrite + Unpin + Send`, co umożliwia bezpośrednie testowanie na `tokio::io::duplex`.

---

## 8. Projekt rejestru protokołów i profili modeli

Transceivery jednego producenta różnią się między sobą zestawem instrukcji, formatem kodowania oraz zakresem obsługiwanych parametrów. Dlatego każdy model posiada swój formalny profil:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct RigModelProfile {
    pub model_id: &'static str,
    pub name: &'static str,
    pub vendor: RigVendor,
    pub default_baud_rate: u32,
    pub supported_baud_rates: &'static [u32],
    pub default_ci_v_address: Option<u8>,
    pub capabilities: RigCapabilities,
    pub extended_capabilities: NativeRigCapabilities,
    pub safe_poll_commands: &'static [&'static str],
}
```

Rejestr profili (`RigModelRegistry`) udostępnia metody wyszukiwania po nazwie, producencie i identyfikatorze, umożliwiając automatyczne uzupełnianie parametrów w konfiguracji CAT.

---

## 9. Model capabilities podstawowych i rozszerzonych

### 9.1. Capabilities podstawowe (`RigCapabilities`)
Obsługuje wspólny kontrakt przenośny:
- `frequency`, `mode`, `passband`, `split`, `split_frequency`, `ptt`, `vfo`, `rit`, `xit`, `power`, `s_meter`.

### 9.2. Capabilities rozszerzone (`NativeRigCapabilities`)
Obsługuje specyficzne funkcje transceiverów:
- `dualwatch`: jednoczesny odbiór na dwóch częstotliwościach,
- `main_sub_tracking`: śledzenie VFO Main i Sub,
- `antenna_selection`: przełączanie złączy antenowych (ANT1, ANT2, RX ANT),
- `dsp_filter_shapes`: wybór kształtu filtru DSP (Sharp / Soft),
- `preamp_attenuator`: wielostopniowy przedwzmacniacz (Preamp 1/2) i tłumik (ATT 6/12/18 dB),
- `noise_reduction_level`: płynna regulacja redukcji szumów (NR 0-15),
- `notch_auto_manual`: sterowanie filtrem wycinającym (Auto Notch, Manual Notch),
- `transceive_broadcast`: obsługa spontanicznych ramek wysyłanych przez radio po poruszeniu gałką.

---

## 10. Mapa Icom CI-V

### 10.1. Struktura protokołu
Ramka CI-V:
```
[0xFE, 0xFE] [to_address] [from_address] [command] [subcommand (opcjonalne)] [dane BCD/bajty] [0xFD]
```
- Preambuła: `0xFE, 0xFE`
- Terminator: `0xFD`
- Kontroler SPLogbook: domyślnie `0xE0`
- Odpowiedź pozytywna (ACK): `FE FE <from> <to> FB FD`
- Odpowiedź negatywna (NAK): `FE FE <from> <to> FA FD`

### 10.2. Obsługa kodowania częstotliwości (BCD)
Częstotliwość w Hz jest kodowana jako 5 bajtów (10 cyfr) BCD w porządku little-endian (od najmniej znaczących dziesiątek/jednostek Hz do setek MHz i GHz).
Przykład: 14 074 000 Hz:
- Wartość 10-cyfrowa: `00 14 07 40 00`
- Bajty CI-V BCD (od prawej do lewej): `[0x00, 0x40, 0x07, 0x14, 0x00]`

### 10.3. Mapowanie podstawowych poleceń
| Funkcja | Komenda | Subkomenda | Format danych |
|---|---|---|---|
| Odczyt częstotliwości | `0x03` | brak | radio odpowiada ramką `0x03` + 5B BCD |
| Ustawienie częstotliwości | `0x05` | brak | 5 bajtów BCD |
| Odczyt trybu i filtru | `0x04` | brak | radio odpowiada ramką `0x04` + tryb (1B) + filtr (1B) |
| Ustawienie trybu i filtru | `0x06` | brak | tryb (1B) + opcjonalnie filtr (1B) |
| Wybór VFO | `0x07` | brak | `0x00` (VFO A), `0x01` (VFO B), `0xA0` (Main), `0xB0` (Sub) |
| Wybór Split | `0x0F` | brak | `0x00` (OFF), `0x01` (ON) |
| S-Meter | `0x15` | `0x02` | 2 bajty BCD (0000 - 0255) |
| PTT | `0x1C` | `0x00` | `0x00` (RX), `0x01` (TX) |
| RF Power | `0x14` | `0x0A` | 2 bajty BCD (0000 - 0255) |

---

## 11. Mapa Kenwood PC Control (Wdrożono w NATIVE-2)

1. **Format**: Tekst ASCII ze średnikiem `;` jako terminatorem każdej ramki.
2. **Kluczowe komendy**:
   - `FA<11 cyfr Hz>;` / `FB<11 cyfr Hz>;` — częstotliwość VFO A / VFO B (odczyt: `FA;` / `FB;`),
   - `MD<kod>;` — emisja (1: LSB, 2: USB, 3: CW, 4: FM, 5: AM, 6: FSK/RTTY, 7: CW-R, 8: FSK-R, 9: PSK),
   - `TX;` / `RX;` — bezpieczne sterowanie PTT bez ponawiania (odczyt: `TX;` zwraca `TX0;`/`TX1;`/`TX2;`),
   - `SM0;` — odczyt S-metra (odpowiedź `SM000xx;`, mapowana na dBm i jednostki S),
   - `FT0;` / `FT1;` — załączenie i wyłączenie pracy Split,
   - `IF;` — pełny 38-znakowy blok statusu transceivera (częstotliwość, RIT/XIT offset, stan TX/RX, emisja, VFO, Split).
3. **Tryb Auto Information (AI)**:
   - Włączenie `AI2;` przy nawiązywaniu połączenia eliminuje konieczność ciągłego odpytywania częstotliwości, emisji i PTT.
   - Radio samorzutnie rozsyła zmiany parametrów (np. obrót gałką VFO, zmiana pasma, aktywacja PTT), redukując nadmiarowy ruch odpytywania.
4. **Transport**:
   - Porty szeregowe / USB Virtual COM przez `tokio-serial`,
   - Kenwood Network Command System (KNS LAN): TCP port 60000 dla TS-890S i TS-990S, negocjacja `##CN` / `##ID` z bezpiecznym maskowaniem poświadczeń.
5. **Profile referencyjne**:
   - `TS-590S` i `TS-590SG` (USB Virtual COM / RS-232C, brak wbudowanego Ethernetu),
   - `TS-890S` (Hybrid SDR, USB + KNS LAN),
   - `TS-990S` (Dual RX Main + Sub, USB + KNS LAN).

---

## 12. Mapa Yaesu CAT (Wdrożono w NATIVE-3, Audyt NATIVE-3A, Korekta Semantyki NATIVE-3B)

1. **Format protokołu**:
   - **Nowoczesna rodzina tekstowa ASCII** (FT-891, FT-991A, FT-710, FTDX10): komendy alfanumeryczne zakończone średnikiem `;`.
   - **Klasyczna rodzina binarna** (FT-817, FT-857, FT-897): LegacyBinary sklasyfikowano w enumie `YaesuProtocolFamily::LegacyBinary`, ale nie wdrożono w NATIVE-3 (brak kodeka binarnego, modele nie są rejestrowane jako aktywne profile).
2. **Kluczowe komendy ASCII**:
   - `FA<9 cyfr Hz>;` / `FB<9 cyfr Hz>;` — częstotliwość VFO A / VFO B (odczyt: `FA;` / `FB;`), np. `FA014074000;` dla 14.074 MHz (9 cyfr Hz). Formatter odrzuca wartości niepoprawne i przekraczające limit profilu.
   - `MD0<kod>;` — emisja (1: LSB, 2: USB, 3: CW-U, 4: FM, 5: AM, 6: RTTY-L, 7: CW-L, 8: DATA-L, 9: RTTY-U, A: DATA-FM, B: FM-N, C: DATA-U, D: AM-N, E: PSK na FT-710/FTDX10 lub C4FM na FT-991A; brak na FT-891). Brak cichego fallbacku do USB.
   - `TX1;` / `TX0;` — bezpieczne sterowanie PTT bez ponawiania (odczyt: `TX;` zwraca `TX0;`/`TX1;`/`TX2;`).
   - `SM0;` — odczyt S-metra (odpowiedź `SM0[000-255];`). Oficjalne instrukcje CAT Yaesu (FT-891, FT-991A, FT-710, FTDX10) definiują odpowiedź SM wyłącznie jako surowy 3-cyfrowy poziom defleksji wskaźnika (000..255), zależny od przedwzmacniacza, tłumika i AGC. Instrukcje nie podają wzorów, progów S0..S9, dB ponad S9 ani dBm. SPLogbook nie fabrykuje syntetycznych jednostek: `smeter_to_unit` jest transparentnym opakowaniem raw, `s_meter_dbm`/`s_meter_unit` nie są zniekształcane w `RigState`, a profile Yaesu deklarują `RigCapabilities::s_meter = false`. Odczyt surowy dostępny jest przez `YaesuBackend::raw_smeter()`.
   - `FT0;` / `FT1;` — załączenie i wyłączenie pracy Split.
   - `IF;` — 29-znakowy (lub 27-znakowy bez P11/P12) blok statusu transceivera (częstotliwość 9B, offset RIT 5B, flagi RIT/XIT, stan TX/RX, emisja, VFO, Split).
   - `AI1;` / `AI0;` — włączenie / wyłączenie trybu Auto Information (podczas rozłączania aktywnej sesji wysyłane jest `AI0;` z pojedynczą ograniczoną czasowo próbą).
   - `?;` — odpowiedź błędu radioodbiornika na nieprawidłowe lub niedozwolone polecenie.
3. **Tryb Auto Information (AI)**:
   - Włączenie `AI1;` następuje wyłącznie po potwierdzeniu sesji przez odczyt weryfikacyjny (probe).
   - Transceiver asynchronicznie rozsyła zmiany parametrów (ruch gałką, zmiana emisji, PTT), redukując liczbę zapytań pollingowych w stanie spoczynku do pojedynczego odczytu S-metra.
4. **Rozdzielenie Probe, Connect i Disconnect**:
   - **Probe**: ściśle read-only (`FA;`). Nigdy nie aktywuje AI ani PTT.
   - **Connect**: weryfikacja probe, a po jej sukcesie inicjalizacja zmieniająca konfigurację (`AI1;`). W razie niepowodzenia AI nie pozostaje aktywne.
   - **Disconnect**: bezpieczne wyłączenie nadawania (`TX0;` wysyłane tylko gdy transport istnieje, sesja jest aktywna, `confirmed_ptt == true` i profil potwierdza PTT) oraz wyłączenie AI (`AI0;` wysyłane tylko gdy transport istnieje, sesja jest aktywna i `ai_active == true`). Każda próba jest pojedyncza, ograniczona czasowo (500 ms) i nie blokuje zamknięcia transportu.
5. **Transport i porty COM per profil**:
   - Transceivery Yaesu ze zintegrowanym kontrolerem USB (Silicon Labs CP2105) udostępniają w systemie dwa wirtualne porty: **Enhanced COM Port** i **Standard COM Port**.
   - **Enhanced COM Port** jest preferowanym portem do sterowania CAT dla wszystkich nowoczesnych modeli.
   - **Standard COM Port**: na FT-710 port CAT-2 (Standard COM) może zostać w menu skonfigurowany do obsługi komend CAT. Na FT-891, FT-991A i FTDX10 port Standard COM służy wyłącznie do sprzętowego kluczowania PTT/CW/FSK przez linie RTS/DTR.
6. **Profile referencyjne**:
   - `FT-891` (fabrycznie 4800 baud, dozwolone: 4800, 9600, 19200, 38400 bps, 9 cyfr Hz, pojedynczy port CAT na Enhanced COM, `s_meter: false`),
   - `FT-991A` (fabrycznie 4800 baud [Menu 031 CAT RATE], dozwolone: 4800, 9600, 19200, 38400 bps, 9 cyfr Hz, Enhanced COM dla CAT, kod E = C4FM, `s_meter: false`),
   - `FT-710` (fabrycznie 38400 baud [CAT-1 RATE], dozwolone: 4800, 9600, 19200, 38400, 115200 bps, 9 cyfr Hz, Standard COM CAT-2 konfigurowalny w menu, `s_meter: false`),
   - `FTDX10` (fabrycznie 38400 baud, dozwolone: 4800, 9600, 19200, 38400, 115200 bps, 9 cyfr Hz, Enhanced COM dla CAT, kod E = PSK, `s_meter: false`).

---

## 13. Natywna obsługa Elecraft (Wdrożenie NATIVE-4)

### 13.1. Oficjalne źródła referencyjne
1. **Elecraft K3S, K3, KX3, and KX2 Programmer's Reference**, Rev G5, February 20, 2019.
2. **Elecraft K4 Programmer's Reference**, Rev D12, May 5, 2026.

### 13.2. Rozdzielenie rodzin protokołów
Protokół Elecraft NIE jest traktowany jako alias Kenwooda. Zaimplementowano jawny podział:
- `ElecraftProtocolFamily::K3Compatible`:
  - Modele: K3, K3S, KX2, KX3.
  - Architektura oparta o rozszerzenia K31 / K22 na porcie szeregowym / USB.
  - Parametry: 8N1, domyślnie 38400 baud, 11 cyfr częstotliwości w Hz.
- `ElecraftProtocolFamily::K4`:
  - Modele: K4, K4D (z drugim fizycznym odbiornikiem KRX4).
  - Architektura direct-sampling SDR z obsługą USB oraz Ethernet TCP (port 9200).
  - Wsparcie dla wielu równorzędnych klientów sterujących.

### 13.3. Różnice sprzętowe i ograniczenia profili
- **KX2**: Urządzenie jednoukładowe bez subreceivera (`supports_subreceiver: false`, `supports_vfo_b_modifier: false`, `dualwatch: false`, `antenna_select: false`).
- **KX3**: Pojedynczy tor RF z programowym dual-watch w tym samym paśmie (`supports_subreceiver: false`, `dualwatch: true`, `antenna_select: false`).
- **K3 / K3S**: Podwójny odbiornik z modułem KRX3 (`supports_subreceiver: true`, `supports_vfo_b_modifier: true`, `dualwatch: true`, `antenna_select: true`).
- **K4 / K4D**: Pełny dwuodbiornikowy SDR z obsługą TCP port 9200 (`supports_tcp: true`, `supports_subreceiver: true`, `supports_vfo_b_modifier: true`).

### 13.4. Kodek strumieniowy (ElecraftCodec)
- Terminator `;`.
- Zmienna długość prefiksów: 2 znaki (`FA`, `FB`, `MD`, `IF`, `TQ`, `PC`), 3 znaki (`SMH`, `SWT`, `SWH`), 4 znaki (`#RVM`).
- Modyfikator `$` kierujący komendy do VFO B / Sub-RX (`MD$`, `BW$`, `SMH$`, `DT$`).
- Rozróżnienie odpowiedzi na zapytania GET od asynchronicznych powiadomień Auto-Info (`AI1`, `AI2`, `AI4`).
- Obsługa ramek błędu `?;` oraz bezpieczny wariant `ElecraftMessage::Unknown` dla nieznanych poleceń.

### 13.5. Semantyka mierników (Meters)
- **K3 / K3S / KX2 / KX3**: Komenda `SM;` zwraca surową liczbę segmentów LCD (0..15 / 0..21). Brak oficjalnej kalibracji na dBm czy S-unit w dokumentacji. Backend udostępnia odczyt surowy przez `raw_smeter()`, a `capabilities.s_meter = false` (brak sztucznych jednostek w `RigState`).
- **K4 / K4D**: Komenda `SMH;` / `SMH$;` udostępnia skalibrowaną moc sygnału w dBm (`+nnn` / `-nnn`), co pozwala na włączenie `capabilities.s_meter = true`.

### 13.6. Bezpieczeństwo PTT i sesji
- `probe()` jest całkowicie read-only (wysyła wyłącznie `ID;`, `FA;`, `MD;`, zamykając transport).
- `connect()` jest dwufazowy: faza 1 weryfikuje radio w trybie read-only, faza 2 aktywuje `AI2;`.
- `SetPtt(true)` jest komendą jednokrotną — brak retry w przypadku błędu.
- `disconnect()` wykonuje pojedynczą, ograniczoną czasowo (200ms) próbę wysłania `RX;` tylko przy potwierdzonym PTT oraz `AI0;` tylko przy aktywnym Auto-Info.

### 13.7. Architektura przyszłego Data Plane K4 (ElecraftStreamService)
W NATIVE-4 zaimplementowano wyłącznie **Control Plane** (CAT, status, Auto-Info przez Serial/TCP).
Obsługa **Data Plane** (strumienie szerokopasmowe) zostanie zaimplementowana w przyszłości jako odrębna usługa:

```text
+------------------------------------------------------------------------+
|                              SPLogbook                                 |
+------------------------------------+-----------------------------------+
|            CONTROL PLANE           |             DATA PLANE            |
|       (Wdrożone w NATIVE-4)        |      (Projekt przyszłościowy)     |
+------------------------------------+-----------------------------------+
|         ElecraftBackend            |       ElecraftStreamService       |
|    - Port szeregowy / USB          |    - Odrębne gniazda TCP/UDP      |
|    - Port TCP 9200 (ASCII CAT)     |    - Strumień panadaptera (#PAN)  |
|    - FA, FB, MD, IF, AI, TQ        |    - Strumień widma dB/bin (#DBS) |
|    - RigState & NativeRigEvent     |    - Strumień próbek I/Q (#IQ)    |
|    - Pętla poll() i zdarzenia AI   |    - Cyfrowy strumień audio (#AUD)|
+------------------------------------+-----------------------------------+
```
Dane strumieniowe Data Plane pod żadnym pozorem nie będą mieszane ze strukturą `RigState` ani wspólnym transportem ramek CAT.

---

## 14. Plan FlexRadio SmartSDR (Plan NATIVE-5)

1. Odkrywanie w sieci lokalnej przez broadcast UDP 4992.
2. Połączenie sterujące TCP 4992 z protokołem asynchronicznym: `C<seq>|<cmd>`, odpowiedzi `R<seq>|...`, statusy `S<handle>|...`.
3. Model Slice Receivers: odrębne odbiorniki programowe przypisywane do VFO.

---

## 15. Plan innych producentów

Kolejni producenci (m.in. Xiegu X6100/G90, Lab599 TX-500, Expert Electronics, Ten-Tec) będą dołączani wyłącznie po skompletowaniu oficjalnej dokumentacji, zdefiniowaniu profilu w rejestrze i napisaniu dedykowanego zestawu testów kodeka i transportu.

---

## 16. Bezpieczeństwo PTT (Rygor krytyczny)

Zasady bezpieczeństwa operacji nadawczych:
1. **Brak automatycznego PTT podczas inicjalizacji**: Metody `connect()`, `probe()`, detekcja modelu i odpytywanie wstępne **pod żadnym pozorem** nie wysyłają rozkazu przejścia na nadawanie.
2. **Zakaz PTT w testach jednostkowych**: Żaden automatyczny test nie może aktywować nadawania na podłączonym fizycznym sprzęcie.
3. **Brak retry dla PTT**: Jeżeli rozkaz `SetPtt(true)` lub `SetPtt(false)` zakończy się błędem, timeoutem lub brakiem potwierdzenia, backend **nie ponawia** próby automatycznie, lecz natychmiast raportuje błąd krytyczny `RigError::Protocol` do operatora.
4. **Idempotentne rozłączanie (Fail-Safe)**: W przypadku wywołania `disconnect()` lub zerwania sesji następuje bezwarunkowa próba wysłania rozkazu wyłączenia PTT (`SetPtt(false)`), a następnie zamknięcie portu.
5. **Blokada komend inwazyjnych**: Podczas `probe()` zabronione jest strojenie tunera, nadawanie nośnej, zapisywanie komórek pamięci i skanowanie pasma.

---

## 17. Model testów bez sprzętu

Testowanie transportów szeregowych i kodeków odbywa się w pełni deterministycznie w środowisku CI:
- **`tokio::io::duplex`**: Emuluje dwukierunkowy kanał szeregowy (strumień bajtów).
- **Testy fragmentacji**: Wstrzykiwanie danych po 1 bajcie z opóźnieniami, weryfikacja czy kodek poprawnie scala ramki.
- **Testy sklejania**: Przesyłanie wielu pełnych ramek w jednym pakiecie I/O, weryfikacja kolejności przetwarzania.
- **Testy zakłóceń**: Wstrzykiwanie śmieciowych bajtów przed prawidłową preambułą `0xFE, 0xFE` lub po terminatorze `0xFD`.
- **Testy błędów protokołu**: Weryfikacja reakcji na NAK (`0xFA`), błędny BCD, przekroczenie maksymalnego rozmiaru ramki.

---

## 18. Model testów Hardware-In-The-Loop (HIL)

Dla laboratoriów wyposażonych w fizyczne urządzenia lub mikrokontrolery symulujące radio (np. Teensy/RP2040 udające IC-7300/TS-590):
- Dedykowana flaga testowa `#[ignore = "hardware-in-the-loop"]`,
- Testy HIL uruchamiane wyłącznie ręcznie z jawną konfiguracją portu COM i modelu przez zmienne środowiskowe (np. `SPLOG_HIL_PORT=COM3`, `SPLOG_HIL_MODEL=IC-7300`).
- Testy HIL podlegają tym samym rygorom bezpieczeństwa PTT (brak nadawania RF bez sztucznego obciążenia i zgody operatora).

---

## 19. Plan migracji konfiguracji

Konfiguracja stacji (`AppConfig` / `station_config.json`) zachowuje wsteczną kompatybilność:
- Pole `cat_backend` przyjmuje nowe wartości: `"icom_ci_v"`, `"kenwood"`, `"yaesu"`, `"elecraft"`, `"flexradio"`,
- Zachowane zostają dotychczasowe pola `cat_serial_port` i `cat_baud_rate`,
- Dodane zostaną opcjonalne pola dla zaawansowanych ustawień CI-V / sieciowych (`cat_ci_v_address: Option<u8>`).

---

## 20. Plan interfejsu użytkownika (UI)

W fali NATIVE-6 okno `cat_settings.rs` zostanie wzbogacone o:
1. Dynamiczne formularze zależne od wybranego backendu (np. pole adresu CI-V widoczne tylko dla Icom, konfiguracja IP/portu dla FlexRadio i Kenwood LAN).
2. Przycisk bezpiecznego testu połączenia („Testuj połączenie”) wykonujący wyłącznie operację `probe()` (odczyt wersji/częstotliwości, bez PTT).
3. Wyświetlanie wykrytych możliwości transceivera (`RigCapabilities`).

W falach NATIVE-0 i NATIVE-1 interfejs GUI nie ulega modyfikacji runtime, co gwarantuje pełną stabilność aplikacji.

---

## 21. Rozstrzygnięcie dotyczące SO2R

1. **SO2R nie jest backendem radiowym**: Moduł `src/cat/so2r.rs` zostaje zachowany w dotychczasowej postaci jako czysta domena stanu dwóch transceiverów.
2. **Architektura przyszłości**: Koordynator SO2R (`So2rCoordinator`) będzie zarządzać dwiema niezależnymi instancjami `RigService` (Radio A i Radio B), odpowiadając za:
   - PTT Interlock (sprzętową i logiczną blokadę nadawania na dwóch radiach jednocześnie),
   - Fokus odbiorczy i nadawczy (RX/TX focus),
   - Przełączanie audio (lewe ucho / prawe ucho).

---

## 22. Fale realizacyjne (NATIVE-0 do NATIVE-7)

| Fala | Nazwa fali | Zakres i cel |
|---|---|---|
| **NATIVE-0** | Fundament architektury i transport szeregowy | Audyt, dokument `Native CAT.md`, struktura `src/cat/native/`, wspólny transport `tokio-serial`, model profili i testy transportu. |
| **NATIVE-1** | Pełny Icom CI-V Backend | Migracja `icom_ci_v.rs` przez `git mv`, dekoder strumieniowy, obsługa ACK/NAK/echo/transceive, profil IC-7300/7610/705, implementacja `RigBackend`, testy. |
| **NATIVE-2** | Pełny Kenwood PC Control | Implementacja KenwoodCodec, Auto Information, obsługa portów szeregowych i LAN, profile TS-590/890/990. |
| **NATIVE-3** | Pełny Yaesu CAT | Implementacja YaesuCodec (rodziny tekstowe i binarne), obsługa portów Enhanced/Standard. |
| **NATIVE-4** | Elecraft Backend | K3/KX oraz K4 Ethernet, multisesyjność. |
| **NATIVE-5** | FlexRadio SmartSDR | Protokół TCP/UDP SmartSDR, obsługa slice receivers i subskrypcji. |
| **NATIVE-6** | Fabryka backendów, migracja i UI | Integracja z `cat_settings.rs`, dynamiczne panele konfiguracji, walidacja i18n. |
| **NATIVE-7** | Testy HIL, dokumentacja i audyt końcowy | Procedury testów sprzętowych, aktualizacja podręcznika i audyt bezpieczeństwa. |

---

## 23. Kryteria ukończenia fal NATIVE-0, NATIVE-1, NATIVE-2 i NATIVE-3

### Kryteria NATIVE-0:
- [x] Pełny audyt istniejących modułów CAT.
- [x] Opracowanie kompletnego dokumentu `Native CAT.md`.
- [x] Dodanie zależności `tokio-serial` z czystym `cargo tree -d`.
- [x] Utworzenie modułów `src/cat/native/` (`transport.rs`, `profile.rs`, `registry.rs`).
- [x] Zapewnienie asynchronicznego transportu opartego na `tokio-serial` z testami na `tokio::io::duplex`.
- [x] Wszystkie dotychczasowe 419 testów przechodzą, brak nowych ostrzeżeń clippy.

### Kryteria NATIVE-1:
- [x] Migracja `icom_ci_v.rs` przez `git mv` z zachowaniem historii.
- [x] Implementacja `IcomCiVCodec` z obsługą fragmentacji, ACK, NAK, BCD i echa.
- [x] Implementacja `IcomCiVBackend` spełniającego kontrakt `RigBackend`.
- [x] Utworzenie profili dla referencyjnych transceiverów Icom (IC-7300, IC-7610, IC-705, IC-706MKIIG).
- [x] Zestaw 20 deterministycznych testów Icom.
- [x] Pełna walidacja kompilatora i testów (fmt, check, clippy, testy CAT i pełne).

### Kryteria NATIVE-2:
- [x] Migracja `kenwood.rs` przez `git mv` do `src/cat/native/kenwood/codec.rs` z zachowaniem historii.
- [x] Implementacja strumieniowego `KenwoodCodec` z buforowaniem ramkowym i wykrywaniem fragmentacji.
- [x] Implementacja parsera stanu transceivera `IF;` (`KenwoodIfInfo`).
- [x] Implementacja `KenwoodBackend` spełniającego kontrakt `RigBackend`.
- [x] Obsługa transportu szeregowego RS-232C i USB Virtual COM.
- [x] Obsługa asynchronicznego Auto Information (`AI2;`) z redukcją nadmiarowego pollingu.
- [x] Implementacja dedykowanego transportu sieciowego Kenwood Network Command System (`KenwoodLanTransport`) z protokołem handshake `##CN`, uwierzytelnianiem `##ID` i bezpiecznym maskowaniem poświadczeń.
- [x] Utworzenie profili dla referencyjnych transceiverów Kenwood (`TS-590S`, `TS-590SG`, `TS-890S`, `TS-990S`).
- [x] Rygorystyczne bezpieczeństwo PTT (brak PTT podczas connect/probe, brak ponawiania SetPtt(true), bezpieczne idempotentne rozłączanie).
- [x] Kompletny zestaw 39 deterministycznych testów jednostkowych (kodek, backend, LAN KNS, profile).
- [x] Pełna walidacja (fmt, check, clippy 218 bez nowych ostrzeżeń, 483 testy przechodzące bez błędów).

### Kryteria NATIVE-3:
- [x] Rygorystyczny audyt oficjalnych instrukcji Yaesu CAT (FT-891, FT-991A, FT-710, FTDX10).
- [x] Rozdzielenie rodzin protokołów Yaesu na nowoczesną tekstową (`YaesuAsciiCat`) i klasyczną binarną (`YaesuLegacyBinaryCat`).
- [x] Implementacja strumieniowego `YaesuAsciiCodec` z buforowaniem ramkowym zakończonym `;`, obsługą fragmentacji, `?;` i Auto Information.
- [x] Implementacja `YaesuBackend` implementującego kontrakt `RigBackend`.
- [x] Obsługa podwójnych portów COM wirtualnych urządzeń Silicon Labs CP2105 (Enhanced vs Standard) z dedykowaną heurystyką `is_enhanced_port_recommended`.
- [x] Obsługa asynchronicznego Auto Information (`AI1;`) redukującego polling.
- [x] Utworzenie profili dla referencyjnych transceiverów Yaesu (`FT-891`, `FT-991A`, `FT-710`, `FTDX10`) z kodowaniem 9-cyfrowym częstotliwości i poprawnym mapowaniem emisji.
- [x] Rygorystyczne bezpieczeństwo PTT (brak PTT podczas connect/probe, probe wyłącznie tylko do odczytu, brak retry dla SetPtt(true), bezpieczne idempotentne rozłączanie).
- [x] Kompletny zestaw deterministycznych testów jednostkowych (kodek, backend na dupleksie, profile).
- [x] Pełna walidacja kompilatora i testów (fmt, check, clippy 218 bez nowych ostrzeżeń, 513 testów przechodzących bez błędów).

### Kryteria NATIVE-3A:
- [x] Precyzyjny audyt profili FT-891, FT-991A, FT-710, FTDX10 na podstawie oficjalnych instrukcji CAT.
- [x] Korekta baud rate (FT-891 i FT-991A fabrycznie 4800 baud).
- [x] Usunięcie nieudokumentowanego przeliczania wartości SM na dBm.
- [x] Walidacja długości ramek IF per rodzina modeli.
- [x] Zestaw testów audytowych Yaesu.

### Kryteria NATIVE-3B (Finalna korekta semantyki pomiarów i sesji Yaesu):
- [x] Weryfikacja komendy SM: oficjalne instrukcje FT-891, FT-991A, FT-710, FTDX10 definiują wyłącznie 000..255 (defleksja wskaźnika) bez progów S0..S9, dB ponad S9 ani dBm.
- [x] Eliminacja syntetycznych jednostek S (`smeter_to_unit` jest transparentnym opakowaniem raw, usunięto `smeter_to_dbm_and_unit`).
- [x] `RigCapabilities::s_meter = false` dla profili Yaesu (brak skalibrowanego dBm w RigState); surowy odczyt dostępny jako `raw_smeter()`.
- [x] Audyt `disconnect()`: warunkowe wysyłanie `TX0;` i `AI0;`, pojedyncza ograniczona czasowo próba (500 ms), brak retry, brak blokowania zamknięcia transportu, bezpieczne czyszczenie stanu.
- [x] Audyt `connect()` i `probe()`: ścisły podział na read-only `probe()` (`FA;`, brak modyfikacji stanu) i `connect()` (weryfikacja + potwierdzona inicjalizacja `AI1;`).
- [x] Rozstrzygnięcie niespójności raportowych i oparcie metryk wyłącznie na bezpośrednich wynikach narzędzi.
- [x] Pełna walidacja: 46 testów modułu Yaesu, 215 testów CAT, 543 testy pełnego zestawu — wszystkie passed, 0 błędów, 0 nowych ostrzeżeń clippy.

### Kryteria NATIVE-4:
- [x] Implementacja natywnego backendu Elecraft CAT (`K3`, `K3S`, `KX2`, `KX3`, `K4`, `K4D`).
- [x] Rozróżnienie rodzin `ElecraftProtocolFamily::K3Compatible` oraz `ElecraftProtocolFamily::K4`.
- [x] Strumieniowy tekstowy `ElecraftCodec` z terminatorem `;` i modyfikatorem `$`.
- [x] Obsługa asynchronicznego Auto Information (`AI2;`).
- [x] Dedykowany transport sieciowy TCP dla K4 (port 9200) z obsługą wielu klientów.
- [x] Rygorystyczne bezpieczeństwo PTT (read-only probe, brak retry SetPtt(true), bezpieczny disconnect).
- [x] Zestaw testów deterministycznych na strumieniach dupleksowych i socketach testowych.

### Kryteria NATIVE-4A (Niezależny audyt zgodności z dokumentacją Elecraft):
- [x] Audyt profili z *K3S, K3, KX3, and KX2 Programmer's Reference* Rev G5 i *K4 Programmer's Reference* Rev D12.
- [x] Precyzyjna obsługa modyfikatora `$` oraz weryfikacja DSP dual-watch w KX2 i KX3 (`extended_capabilities.dualwatch = true`, `supports_vfo_b_modifier = true`).
- [x] Filtrowanie komend z modyfikatorem `$` per model (`supports_dollar_command()`) — odrzucenie `AN$` i `SMH$` na urządzeniach bez odpowiedniego sprzętu.
- [x] Rozróżnienie hardware subreceivera na K3/K3S: `static_subreceiver_capable: bool` vs runtime detekcja modułu opcjonalnego KRX3 za pomocą zapytania `OM;` (flaga `'S'`).
- [x] Deterministyczny parser prefiksów poleceń z tabelami `KNOWN_PREFIXES_4`, `KNOWN_PREFIXES_3`, `KNOWN_PREFIXES_2` eliminujący błędne dopasowania prefiksów w komendach o alfabetycznych parametrach (np. `OMAP1;`).
- [x] Precyzyjna semantyka S-metra: surowe słupki `SM;` (0000..0015/0021) na K3/KX oraz skalibrowany odczyt w dBm (`SMH;`) na K4.
- [x] Weryfikacja formatów mocy `PC` i `PO`: skala `L` (0.1W) dla QRP i K4 (<10W), skala `H` (1W) oraz odczyt rzeczywistej mocy nadawania `PO` w 0.1W.
- [x] Rejestracja profilu `ELECRAFT_K4HD` (KDDC4 dual superhet front-end) oraz uzupełnienie prędkości 57600 baud dla rodziny K4.
- [x] Weryfikacja stałej portu TCP `K4_DEFAULT_TCP_PORT = 9200` i zapewnienie dynamicznych portów (`127.0.0.1:0`) we wszystkich testach asynchronicznych.

---

## 24. Strategia wycofania (Rollback)

W razie niepowodzenia lub nieprzewidzianej regresji:
1. Każda fala jest zamykana atomowym commitem.
2. Gałęzie `main` i `refactor/w4-arch-cat` są synchronizowane wyłącznie fast-forward.
3. Wycofanie fali sprowadza się do `git revert` commita fali, co przywraca w 100% poprzedni stabilny stan bez konfliktów.

---

## 25. Rejestr zmian funkcjonalnych

W falach NATIVE-0 i NATIVE-1 nie wprowadza się żadnych zmian łamiących dotychczasowe zachowanie runtime aplikacji:
- Hamlib, FLRig i TCI zachowują 100% dotychczasowego kodu i funkcjonalności.
- Istniejące profile konfiguracji nie ulegają uszkodzeniu.
- Nowe backendy są dodawane jako niezależne, modularne komponenty.

---

## 26. Ryzyka licencyjne dokumentacji i implementacji

1. **Licencja kodu**: Całość nowego kodu SPLogbook powstaje na licencji GPL-3.0-or-later.
2. **Dokumentacja producentów**: Wszystkie stałe protokołów (adresy, kody komend) stanowią fakty techniczne konieczne do interoperacyjności (zgodnie z prawem autorskim i orzecznictwem dotyczącym inżynierii wstecznej i protokołów komunikacyjnych). Żadne zastrzeżone fragmenty tabel producentów nie są kopiowane dosłownie do repozytorium.
3. **Izolacja czystego pokoju**: Implementacje ramek i kodeków bazują wyłącznie na własnej logice parsowania binarnego i tekstowego.
