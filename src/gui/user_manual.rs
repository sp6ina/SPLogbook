// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wbudowany podręcznik użytkownika (Instrukcja obsługi).

use crate::gui::app::SpLogApp;
use eframe::egui;

/// Sekcja podręcznika. Treść trzymana w kodzie (offline), gotowa na
/// późniejszą migrację do plików `.md` w `assets/docs/` per język.
pub struct ManualSection {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

pub const MANUAL_SECTIONS: &[ManualSection] = &[
    ManualSection {
        id: "start",
        title: "1. Pierwsze kroki",
        body: "Witaj w SPLogbook — zaawansowanym, nowoczesnym środowisku logowania i automatyzacji stacji krótkofalarskiej stworzonym w języku Rust.\n\n\
1. Konfiguracja początkowa: Wybierz menu Pomoc → Kreator pierwszego uruchomienia (Welcome Wizard), aby wprowadzić swój znak stacji (np. SP6INA), imię, locator Maidenhead (np. JO81) oraz opcjonalny kod gminy PGA i strefy CQ/ITU.\n\n\
2. Podłączenie radia (CAT): W menu Ustawienia → Konfiguracja CAT / Rotora wybierz backend odpowiedni dla Twojego sprzętu (Hamlib, Kenwood/Yaesu, Icom CI-V, FLRig lub TCI dla SDR). Po połączeniu częstotliwość, emisja i raporty będą synchronizowane w czasie rzeczywistym.\n\n\
3. Podłączenie klastra DX: Przejdź do zakładki lub pływającego okna DX Cluster. Kliknij 'Połącz' (domyślny węzeł: cluster.sp7pka.ampr.org:7300 lub własny serwer Telnet). Kliknięcie dowolnego spotu automatycznie przestraja radio i wypełnia formularz logowania.\n\n\
4. Logowanie QSO: Wpisz znak korespondenta w polu Callsign i naciśnij Enter lub F2. SPLogbook natychmiast zweryfikuje prefiks, wyznaczy azymut i odległość, pobierze dane z bazy Callbook/QRZ oraz sprawdzi status dyplomowy (ATNO / nowe pasmo).\n\n\
Wskazówka: Naciśnij Ctrl+Shift+P w dowolnym momencie, aby otworzyć Paletę Poleceń (Command Palette) i natychmiast wywołać dowolną funkcję programu z klawiatury.",
    },
    ManualSection {
        id: "station",
        title: "2. Konfiguracja stacji i CAT",
        body: "SPLogbook oferuje uniwersalną warstwę abstrakcji CAT (CatBackend), obsługującą szeroki wachlarz protokołów:\n\n\
- Hamlib (rigctld): Domyślny backend sieciowy TCP (port 4533/4534). Możesz korzystać z wbudowanej binarki (Bundled Hamlib 4.7.2) lub zainstalowanej w systemie (/usr/bin/rigctld lub systemowy PATH na Windowsie).\n\
- Kenwood / Elecraft / Yaesu: Bezpośrednia obsługa szeregowego protokołu ASCII CAT (komendy FA, FB, MD, TX, RX, IF) bez konieczności uruchamiania zewnętrznych demonów.\n\
- Icom CI-V: Bezpośrednia komunikacja binarna z radiami Icom (wybór adresu szesnastkowego CI-V, np. 0x94 dla IC-7300, 0x88 dla IC-7100).\n\
- TCI (Transceiver Control Interface): Błyskawiczna integracja z transceiverami SDR firm Expert Electronics (SunSDR2 PRO/DX), Thetis, ExpertSDR3 przez protokół WebSocket.\n\
- FLRig (XML-RPC): Integracja z popularnym serwerem sterowania FLRig.\n\
- SO2R: Tryb pracy na dwa radia (Single-Operator Two-Radio) z automatyczną blokadą jednoczesnego nadawania (TX Lockout).\n\n\
Sterowanie rotorem antenowym: Wbudowany klient rotctld (TCP port 4533) umożliwia obrót anteny jednym kliknięciem na azymut ortodromy (Short Path) ze wskazanego spotu, mapy lub formularza QSO.\n\n\
Współdzielenie portu CAT: SPLogbook posiada wbudowany serwer proxy Hamlib (port 4534), dzięki czemu programy zewnętrzne (np. WSJT-X, FLDigi, JTDX) mogą współdzielić to samo radio bez konfliktów portu szeregowego.",
    },
    ManualSection {
        id: "logging",
        title: "3. Logowanie łączności i baza SQLite",
        body: "Sercem SPLogbook jest silnik bazy danych oparty na SQLite działający w trybie WAL (Write-Ahead Logging):\n\n\
- Niezawodność i bezpieczeństwo danych: Tryb WAL z synchronicznością NORMAL i transakcjami chroni log przed uszkodzeniem nawet przy nagłym zaniku zasilania. Baza tworzy automatyczne, rotacyjne kopie zapasowe (do 10 rewizji wstecz).\n\
- Indeksy wydajnościowe: 10 wyspecjalizowanych indeksów gwarantuje natychmiastowe wyszukiwanie i filtrowanie nawet przy bazach liczących setki tysięcy QSO.\n\
- Inteligentny formularz QSO: Automatyczna konwersja znaków do wielkich liter, dynamiczna korekta błędów pisowni (Levenshtein), weryfikacja prefiksów ITU i dopasowanie do bazy CTY.DAT.\n\
- Wykrywanie duplikatów: Natychmiastowe, pogrubione ostrzeżenie [DUPE!] w przypadku próby ponownego zalogowania korespondenta na tym samym paśmie i w tej samej emisji.\n\
- Nagrywanie audio łączności: Możliwość rejestracji sygnału audio z mikrofonu/karty dźwiękowej i przypięcia pliku WAV do rekordu QSO.\n\
- Multi-Journal: Obsługa wielu niezależnych dzienników (np. dziennik domowy, stacja klubowa, log wyprawowy /P).",
    },
    ManualSection {
        id: "shortcuts",
        title: "4. Skróty klawiszowe i ergonomia",
        body: "SPLogbook został zaprojektowany z myślą o szybkiej obsłudze bez konieczności sięgania po myszkę:\n\n\
Klawisze funkcyjne (Główne):\n\
- F1 — Okno pomocy i pełnej ściągawki skrótów klawiszowych\n\
- F2 — Zapisz bieżące QSO (Save QSO) — standard kontestowy\n\
- F3 — Wyczyść formularz QSO (Wipe) i ustaw kursor na polu znaku\n\
- F4 — Wymuś natychmiastowe wyszukanie znaku w Callbooku / QRZ.com\n\
- F5 — Odśwież widok dziennika (przeładuj tabelę QSO z bazy)\n\
- F6 — Otwórz okno wysyłania spotu do klastra DX (Send Spot)\n\
- F7 — Przełącz nadawanie PTT (TX/RX) przez CAT\n\
- F8 — Otwórz okno odtwarzacza komunikatów głosowych (Voice Keyer)\n\
- F11 — Przełącz tryb pełnoekranowy (Toggle Fullscreen)\n\
- F12 — Otwórz wbudowaną instrukcję obsługi (ten podręcznik)\n\n\
Skróty kombinowane (Ctrl / Cmd):\n\
- Ctrl+N — Nowe QSO (czyści formularz i aktywuje pole znaku)\n\
- Ctrl+W — Wyczyść formularz QSO (Wipe)\n\
- Ctrl+S — Zapisz konfigurację stacji i dziennik\n\
- Ctrl+F — Otwórz zaawansowany filtr wyszukiwania w logbooku\n\
- Ctrl+L — Przełącz / aktywuj tabelę logbooka\n\
- Ctrl+D — Otwórz / przełącz panel DX Cluster\n\
- Ctrl+B — Otwórz okno Bandmapy (mapa aktywności pasma)\n\
- Ctrl+M — Otwórz interaktywną mapę świata z linią Greyline\n\
- Ctrl+K — Otwórz terminal i makra telegraficzne CW\n\
- Ctrl+P — Otwórz menedżer profili stacji roboczej\n\
- Ctrl+E — Otwórz konfigurowalny eksporter CSV\n\
- Ctrl+I — Otwórz okno importu pliku ADIF\n\
- Ctrl+T — Przełącz motyw kolorystyczny (Dark / Daylight / High-Contrast)\n\
- Ctrl+Z — Cofnij ostatnio usunięte QSO (Undo)\n\
- Ctrl+Y / Ctrl+Shift+Z — Ponów operację (Redo)\n\
- Ctrl+Shift+S — Otwórz okno statystyk i analizy wykresów\n\
- Ctrl+Shift+P — Otwórz paletę szybkiego wyszukiwania poleceń\n\
- Ctrl+Q — Bezpieczne zamknięcie programu",
    },
    ManualSection {
        id: "cluster",
        title: "5. Klaster DX i Bandmapa",
        body: "Moduł DX Cluster dostarcza meldunki o stacjach DX w czasie rzeczywistym:\n\n\
- Wbudowany klient Telnet: Łączy się z dowolnym węzłem klastra DX (np. serwery AR-Cluster, CC-Cluster, DXSpider). Posiada mechanizm automatycznego wznawiania połączenia po zerwaniu sieci.\n\
- Inteligentna deduplikacja: Okno przesuwne 2 kHz / 30 spotów eliminuje powtarzające się meldunki, zapobiegając zaśmiecaniu listy.\n\
- Wyróżnienia graficzne i akustyczne:\n\
  * Gwiazdka Magenta (ATNO): Nowy kraj na liście życiowej (All-Time New One) z opcjonalnym sygnałem dźwiękowym.\n\
  * Iskra Szmaragdowa: Nowe pasmo lub nowa emisja dla danego kraju DXCC.\n\
- Filtrowanie zaawansowane: Wybór pasm HF / WARC / VHF, filtrowanie po emisjach (CW, SSB, cyfrowe), eliminacja spotów od skimmerów automatycznych (RBN) lub praca tylko z meldunkami od operatorów.\n\
- Kliknij i pracuj (Click-to-Tune): Kliknięcie spotu automatycznie przestawia VFO transceivera na częstotliwość stacji DX i wpisuje znak do formularza łączności.\n\
- Bandmapa graficzna: Wizualna reprezentacja pasma z gasnącymi w czasie znacznikami stacji DX.",
    },
    ManualSection {
        id: "propagation",
        title: "6. Propagacja HF, pogoda kosmiczna & EME",
        body: "SPLogbook łączy modelowanie jonosferyczne w czasie rzeczywistym z pobieraniem aktualnych danych geofizycznych:\n\n\
- Dane pogody kosmicznej (NOAA SWPC): Automatyczne pobieranie wskaźników strumienia słonecznego (SFI), liczby plam (SSN), indeksu geomagnetycznego A i K, strumienia promieniowania rentgenowskiego oraz prędkości wiatru słonecznego.\n\
- Silnik VOACAP-lite HF: Oblicza punkt środkowy ortodromy, kąt zenitalny Słońca, tłumienie warstwy D, częstotliwości graniczne i krytyczne (foF2, MUF, LUF, FOT) oraz prawdopodobieństwo otwarcia pasma (0-100%) wraz z szacowaną siłą sygnału na skali S-metra.\n\
- Dynamiczna plakietka propagacji: Przy wpisywaniu znaku korespondenta w formularzu QSO natychmiast wyświetla się wskaźnik prawdopodobieństwa łączności z danym krajem na bieżącym paśmie.\n\
- Kalkulator łączności odbiciowych od Księżyca (EME): Wyznacza w czasie rzeczywistym azymut i elewację Księżyca, odległość Ziemia-Księżyc, fazę lunarną, tłumienie trasy, przesunięcie polaryzacyjne oraz okna widoczności wspólnej (Mutual Window) z wybranym locatorem korespondenta.",
    },
    ManualSection {
        id: "digital",
        title: "7. Emisje cyfrowe (WSJT-X, JS8Call, FLDigi)",
        body: "SPLogbook bezproblemowo integruje się z popularnymi programami do emisji cyfrowych:\n\n\
- Mostek UDP WSJT-X / JTDX (port 2237):\n\
  * Odbiera ramki binarne UDP: Heartbeat, Status, Decode, QSO Logged oraz Clear.\n\
  * Automatycznie zapisuje ukończone łączności FT8 i FT4 w bazie SPLogbook z dokładnymi raportami SNR (dB).\n\
  * Weryfikuje w locie znak wzywającej stacji i informuje, czy jest to nowy kraj, pasmo lub emisja.\n\
- JS8Call (protokół JSON TCP port 2237): Przechwytuje pakiety RX.ACTIVITY, RX.DIRECTED i RIG.FREQ, umożliwiając bezpośrednie logowanie łączności klawiaturowych.\n\
- FLDigi (XML-RPC): Pozwala na płynne logowanie emisji PSK31, RTTY, Olivia oraz odbiór częstotliwości.\n\
- Emisja ramek rozgłoszeniowych N1MM: SPLogbook wysyła pakiety XML N1MM Logger+ przez UDP, co pozwala narzędziom takim jak GridTracker czy nakładkom mapowym na natychmiastowe śledzenie pracy.",
    },
    ManualSection {
        id: "satellites",
        title: "8. Śledzenie satelitów i Doppler CAT",
        body: "Moduł satelitarny dedykowany jest miłośnikom łączności przez satelity LEO oraz transponder geostacjonarny QO-100:\n\n\
- Propagator orbitalny SGP4/SDP4: Wykorzystuje dwuliniowe elementy orbitalne NORAD (TLE) z możliwością automatycznej aktualizacji przez internet.\n\
- Parametry orbity na żywo: Wylicza azymut, elewację, odległość skośną (Slant Range), prędkość radialną oraz footprint satelity.\n\
- Automatyczna kompensacja efektu Dopplera: W czasie rzeczywistym oblicza przesunięcie częstotliwości odbiorczej i nadawczej:\n\
  Δf = -f0 * (v_slant / c)\n\
  i wysyła poprawki do transceivera przez CAT w podziale na Uplink i Downlink.\n\
- Sterowanie rotorem azymut-elewacja: Wysyła współrzędne śledzenia anteny do dwuosiowych rotorów satelitarnych przez protokół rotctld.",
    },
    ManualSection {
        id: "waterfall",
        title: "9. Analizator widma SDR (FFT) i wodospad",
        body: "SPLogbook posiada wbudowany analizator widma audio i wodospad (Waterfall) w czasie rzeczywistym:\n\n\
- Przechwytywanie audio przez cpal: Pobiera strumień audio bezpośrednio z wyjścia karty dźwiękowej transceivera (lub dowolnego wejścia liniowego / wirtualnego kabla audio VB-Cable).\n\
- Silnik transformaty Fouriera (RustFFT): Zoptymalizowana, wielowątkowa transformata FFT z oknem Hanna, minimalizującym wyciek widmowy.\n\
- Konfigurowalne parametry:\n\
  * Rozmiar FFT: 512, 1024, 2048 lub 4096 punktów (wybór między rozdzielczością czasową a częstotliwościową).\n\
  * Regulacja wzmocnienia (Gain dB) oraz podłogi szumowej (Floor dB).\n\
  * Skala barwna: Płynna mapa kolorów ułatwiająca wyławianie słabych sygnałów CW i emisji cyfrowych.\n\
- Uniwersalne dokowanie: Panel widma może pracować jako kafelek w oknie głównym lub jako niezależne okno pływające na drugim monitorze.",
    },
    ManualSection {
        id: "contest",
        title: "10. Zawody krótkofalarskie (Contest Engine)",
        body: "Moduł zawodów zapewnia maksymalną szybkość pracy i zgodność z regulaminami międzynarodowymi:\n\n\
- Obsługiwane zawody: SP DX Contest, CQ World Wide DX (CQ WW), CQ WPX Contest, ARRL International DX, IARU HF World Championship oraz zawody krajowe VHF/UHF.\n\
- Analizator raportów (Exchange Parser): Rozpoznaje ciągi znaków wpisane w jednym polu (np. '599 001 WR' lub '599 15') i automatycznie przypisuje je do raportu RST, numeru seryjnego, strefy CQ/ITU lub województwa.\n\
- Statystyki na żywo: Wskaźnik tempa pracy (QSO/h z ostatnich 10 minut i godziny), historia tempa, licznik mnożników oraz macierz mnożników w podziale na pasma.\n\
- Błyskawiczny zapis z klawiatury: Klawisz Enter lub F2 natychmiast rejestruje łączność, inkrementuje numer seryjny i przywraca fokus na pole znaku korespondenta.\n\
- Eksport logu: Generowanie w pełni poprawnych plików Cabrillo 3.0 zgodnych ze standardami komisji zawodów oraz eksport do formatu ADX (XML ADIF 3.1.5).",
    },
    ManualSection {
        id: "awards",
        title: "11. Dyplomy i programy terenowe (PGA, SOTA, POTA)",
        body: "SPLogbook automatycznie kalkuluje i monitoruje postęp w prestiżowych programach dyplomowych:\n\n\
- DXCC: Zestawienie Mixed, Band (160m-10m), Mode (CW, Phone, Digital) oraz DXCC Challenge. Pełna obsługa statusu ATNO.\n\
- WAZ i WAS: Monitorowanie 40 stref CQ oraz wszystkich 50 stanów USA z wbudowaną przeglądarką stanów.\n\
- Polska Gmina Award (PGA): Kompletna baza wszystkich 2477 polskich gmin z wyszukiwarką i automatycznym rozpoznawaniem kodu gminy z komentarza lub bazy SP.\n\
- Aktywacje terenowe SOTA & POTA: Śledzenie referencji szczytów górskich (Summits on the Air) i parków (Parks on the Air) z dedykowanym eksporterem plików CSV zgodnych z systemem SOTA.\n\
- IOTA & WWFF: Baza wysp morskich i obszarów chronionych florystyczno-faunistycznych.",
    },
    ManualSection {
        id: "qsl",
        title: "12. Projektant QSL i etykiety Avery A4 PDF",
        body: "Zarządzanie potwierdzeniami łączności i wydruk naklejek:\n\n\
- Wektorowy eksporter etykiet PDF: Generuje gotowe do druku arkusze w formacie A4 dla standardowych szablonów Avery (3x8, 3x7, 2x8, 2x7 etykiet na stronę).\n\
- Inteligentna kolejka do druku: Filtrowanie QSO oczekujących na potwierdzenie papierowe, według pasma, emisji lub zakresu dat.\n\
- Precyzyjna kalibracja marginesów: Możliwość dopasowania marginesów poziomych i pionowych z dokładnością do milimetra pod dowolną drukarkę laserową lub atramentową.\n\
- Wizualny projektant kart QSL: Umożliwia dodawanie tła, tekstu stacji, tabeli danych łączności oraz logotypów klubowych.",
    },
    ManualSection {
        id: "export",
        title: "13. Eksporty, chmura i REST API",
        body: "Szerokie możliwości wymiany danych i integracji zewnętrznych:\n\n\
- ADIF 3.1.5 i ADX: Pełna zgodność ze standardem ADIF (pliki .adi oraz XML .adx), w tym obsługa najnowszych podemisji cyfrowych.\n\
- Konfigurowalny eksporter CSV: Wybór dowolnych kolumn z rekordu QSO, separatora (przecinek, średnik, tabulator) i kodowania UTF-8.\n\
- Usługi online: Bezpośrednia synchronizacja z LoTW (certyfikacja cyfrowa TQSL), eQSL.cc, Club Log (w tym OQRS), QRZ.com XML, HamQTH, Cloudlog i HRDLog.net.\n\
- Deterministyczny harmonogram uploadów (Scheduler): Odporna na awarie kolejka offline w SQLite z wykładniczym czasem oczekiwania (exponential backoff) chroniąca limity zapytań API.\n\
- Wbudowany serwer REST API (port 8080): Nowoczesny serwer oparty na bibliotece Axum umożliwia odczyt i dodawanie QSO, pobieranie statystyk oraz nasłuch zdarzeń aplikacji przez WebSocket (/api/v1/ws).",
    },
    ManualSection {
        id: "plugins",
        title: "14. Wtyczki użytkownika (Rhai) i Marketplace",
        body: "Możliwość rozszerzania funkcjonalności programu za pomocą bezpiecznych skryptów:\n\n\
- Piaskownica skryptowa Rhai: Użytkownicy mogą pisać własne wtyczki w plikach .rhai bez konieczności rekompilacji programu. Skrypty są domyślnie izolowane od systemu plików i sieci.\n\
- Zdarzenia i hooki cyklu życia: on_startup(), on_qso_logged(call, band, mode, freq, is_atno), on_dx_spot(spotter, call, freq, comment), on_rig_state(freq, mode, connected) oraz on_band_opened(band).\n\
- Bezpieczny mostek komend (Command Bridge): Skrypty mogą wysyłać makra CW, uruchamiać komunikaty głosowe, sterować rotorem, odtwarzać dźwięki systemowe i wstawiać dane do formularza QSO.\n\
- Wbudowany Marketplace wtyczek: Katalog dodatków z instalacją jednym kliknięciem (POTA Helper, CW Macros, Contest Assistant, Rotor Assistant, Band Activity Logger). Weryfikacja sum kontrolnych SHA256 gwarantuje integralność instalowanych dodatków.",
    },
    ManualSection {
        id: "p2p",
        title: "15. Szyfrowana synchronizacja P2P (LAN/VPN)",
        body: "Bezpieczna praca wielostanowiskowa bez pośrednictwa serwerów w chmurze:\n\n\
- Szyfrowanie end-to-end: Połączenia bezpośrednie między stacjami w sieci lokalnej LAN lub tunelu VPN zabezpieczone są kryptograficznie szyfrem strumieniowym XChaCha20-Poly1305 z wyprowadzaniem klucza za pomocą algorytmu Argon2id.\n\
- Dwukierunkowa replikacja: Automatyczne wykrywanie brakujących rekordów i bezkolizyjne scalanie dzienników podczas pracy ekspedycyjnej lub zawodów Multi-Operator.\n\
- Niezależność od internetu: Zapewnia pełną synchronizację logów w warunkach polowych, gdzie dostępny jest wyłącznie lokalny punkt dostępowy Wi-Fi.",
    },
    ManualSection {
        id: "os_setup",
        title: "16. Specyfika Windows i Linux (Porty, Grupy, Zapora)",
        body: "SPLogbook jest aplikacją w pełni wieloplatformową (Windows 10/11 x64 oraz 64-bitowy Linux). Poniżej zestawiono kluczowe różnice konfiguracyjne:\n\n\
GNU/Linux (x86_64):\n\
1. Uprawnienia do portów szeregowych USB:\n\
   Domyślnie zwykły użytkownik nie ma praw zapisu do portów /dev/ttyUSB* ani /dev/ttyACM*. Należy dodać swoje konto do odpowiedniej grupy systemowej:\n\
   - Ubuntu / Debian / Mint / Raspberry Pi OS:\n\
     sudo usermod -aG dialout $USER\n\
   - Arch Linux / Manjaro / Fedora / openSUSE:\n\
     sudo usermod -aG uucp $USER\n\
   Po wykonaniu komendy należy wylogować się i zalogować ponownie.\n\
2. Środowiska graficzne: Aplikacja wspiera natywnie zarówno serwery X11, jak i Wayland. W razie potrzeby można wymusić backend poleceniem: WINIT_UNIX_BACKEND=x11 ./SPLogbook\n\
3. Ścieżki danych: Zgodne ze standardem XDG Base Directory:\n\
   - Baza danych i logi: ~/.local/share/splogbook/\n\
   - Konfiguracja stacji: ~/.config/splogbook/\n\n\
Microsoft Windows (10 / 11 64-bit):\n\
1. Porty COM: Transceivery z wbudowanym interfejsem USB (np. IC-7300, FT-710, TS-590) tworzą wirtualny port COM (np. COM3, COM4). Upewnij się, że zainstalowano oficjalny sterownik USB UART producenta (Silicon Labs CP210x, FTDI lub CH340).\n\
2. Zapora Windows Defender Firewall: Przy pierwszym uruchomieniu serwera REST API (port 8080), proxy CAT (port 4534) lub mostka WSJT-X (port 2237) zezwól aplikacji SPLogbook na komunikację w sieciach prywatnych.\n\
3. Ścieżki danych: Dane i konfiguracja przechowywane są w profilu użytkownika: %APPDATA%\\SPLogbook\\",
    },
    ManualSection {
        id: "troubleshooting",
        title: "17. Rozwiązywanie problemów (Troubleshooting FAQ)",
        body: "Odpowiedzi na najczęściej spotykane pytania techniczne:\n\n\
P: Transceiver nie łączy się przez CAT (błąd połączenia).\n\
O: 1. Upewnij się, że żadna inna aplikacja (OmniRig, WSJT-X, N1MM) nie blokuje tego samego portu szeregowego COM/tty.\n\
   2. Sprawdź poprawność prędkości Baudrate w menu radia i w SPLogbook (najczęściej 19200 lub 38400, a dla nowszych transceiverów 115200).\n\
   3. Na Linuksie upewnij się, że Twoje konto należy do grupy dialout lub uucp.\n\n\
P: Nie słychać alertów dźwiękowych ATNO lub Voice Keyer nie odtwarza plików.\n\
O: Sprawdź w mikserze głośności systemu, czy SPLogbook ma włączony dźwięk oraz czy domyślne urządzenie wyjściowe audio jest aktywne. Pliki komunikatów głosowych powinny być w standardowym formacie WAV.\n\n\
P: Połączenie z klastrem Telnet zostaje natychmiast przerwane.\n\
O: Niektóre węzły klastra wymagają natychmiastowego podania znaku wywoławczego (Login callsign). Upewnij się, że w konfiguracji klastra wpisano Twój znak stacji.\n\n\
P: Czy mogę przenieść bazę danych na inny komputer?\n\
O: Tak! Wystarczy skopiować plik default_log.db (znajdziesz go klikając menu Pomoc → Otwórz katalog danych programu). Baza w formacie SQLite jest w 100% zgodna binarnie między systemami Windows i Linux.",
    },
];

/// Kontekstowa ikona pomocy „?” — otwiera podręcznik na wskazanej sekcji.
pub fn help_button(app: &mut SpLogApp, ui: &mut egui::Ui, section: &'static str) {
    if ui
        .small_button("?")
        .on_hover_text("Pomoc — otwórz podręcznik na tej sekcji")
        .clicked()
    {
        app.show_user_manual = true;
        app.manual_section = Some(section.to_string());
    }
}

fn section_by_id(id: &str) -> usize {
    MANUAL_SECTIONS.iter().position(|s| s.id == id).unwrap_or(0)
}

/// Okno „Instrukcja obsługi” — panel boczny z sekcjami + przewijana treść.
pub fn render_user_manual_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_user_manual {
        return;
    }
    let mut is_open = app.show_user_manual;

    // Ustalenie aktywnej sekcji (z ewentualnego linku kontekstowego „?”).
    let selected = if let Some(section) = app.manual_section.take() {
        section_by_id(&section)
    } else {
        app.manual_selected
    };

    let mut new_selected = selected;
    let mut close_requested = false;

    egui::Window::new("📖 Instrukcja obsługi i podręcznik użytkownika")
        .open(&mut is_open)
        .default_size([880.0, 580.0])
        .min_width(650.0)
        .min_height(400.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Panel boczny z listą sekcji.
                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        for (i, sec) in MANUAL_SECTIONS.iter().enumerate() {
                            if ui.selectable_label(new_selected == i, sec.title).clicked() {
                                new_selected = i;
                            }
                        }
                    });

                ui.separator();

                // Treść aktywnej sekcji.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let sec = &MANUAL_SECTIONS[new_selected.min(MANUAL_SECTIONS.len() - 1)];
                        ui.heading(egui::RichText::new(sec.title).size(18.0).strong());
                        ui.separator();
                        for paragraph in sec.body.split("\n\n") {
                            ui.label(paragraph);
                            ui.add_space(4.0);
                        }
                    });
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button("📁 Otwórz katalog danych programu")
                    .on_hover_text("Otwiera folder z bazą danych i plikami konfiguracyjnymi")
                    .clicked()
                {
                    if let Some(parent) = app.config_file_path.parent() {
                        let _ = open::that(parent);
                    }
                }
                if ui.button("🌐 Zgłoś uwagę lub błąd na GitHubie").clicked() {
                    let _ = open::that("https://github.com/sp6ina/SPLogbook/issues/new");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Zamknij (Esc)").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    app.manual_selected = new_selected;
    app.show_user_manual = is_open && !close_requested;
}
