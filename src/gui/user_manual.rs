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

pub const MANUAL_SECTIONS_EN: &[ManualSection] = &[
    ManualSection {
        id: "start",
        title: "1. Getting Started",
        body: "Welcome to SPLogbook — an advanced, modern amateur radio logging and station automation environment built in Rust.\n\n\
1. Initial Setup: Open Help → Welcome Wizard (or Station Profile Settings) to enter your station callsign (e.g. SP6INA), operator name, Maidenhead grid locator (e.g. JO81), and optional CQ/ITU zones.\n\n\
2. Connecting Your Radio (CAT): Open Settings → CAT / Rotator Setup and choose the backend matching your transceiver (Hamlib rigctld, Kenwood/Yaesu/Elecraft serial, Icom CI-V, FLRig, or TCI for SDR). Once connected, VFO frequency, mode, and signal reports synchronize in real time.\n\n\
3. Connecting to a DX Cluster: Open the DX Cluster tab or floating window, select a server preset (e.g. dxcluster.pl:8000, dxfun.com:8000, w3lpl.net:7373), and click 'Connect'. Clicking any DX spot automatically tunes your radio and populates the QSO entry form.\n\n\
4. Logging a QSO: Enter the correspondent's callsign in the Callsign field and press Enter or F2. SPLogbook immediately resolves the DXCC prefix, calculates bearing and distance, queries online Callbooks (QRZ/HamQTH), and checks award status (ATNO / New Band / New Mode).\n\n\
Tip: Press Ctrl+Shift+P at any time to open the Command Palette and invoke any application feature from the keyboard.",
    },
    ManualSection {
        id: "station",
        title: "2. Station & CAT Setup",
        body: "SPLogbook provides a universal CAT abstraction layer supporting a wide range of transceivers and protocols:\n\n\
- Hamlib (rigctld): Standard TCP network backend (default port 4532) or automatic background supervisor starting bundled/system rigctld on your chosen serial COM/tty port.\n\
- Kenwood / Elecraft / Yaesu: Direct serial ASCII CAT protocol support (FA, FB, MD, TX, RX, IF) without requiring external daemons.\n\
- Icom CI-V: Direct binary CI-V communication with Icom transceivers (configurable hex address, e.g. 0x94 for IC-7300, 0x88 for IC-7100).\n\
- TCI (Transceiver Control Interface): High-speed network integration with SDR transceivers (Expert Electronics SunSDR2 PRO/DX, Thetis, ExpertSDR3).\n\
- FLRig (XML-RPC): Seamless integration with the popular FLRig transceiver control server (default port 12345).\n\
- SO2R: Single-Operator Two-Radio operation with automatic simultaneous transmit lockout (TX Lockout).\n\n\
Antenna Rotator Control: The built-in rotctld client (TCP port 4533) rotates your antenna with a single click to the Short Path or Long Path azimuth from any DX spot, World Map, or QSO entry.\n\n\
CAT Port Sharing: SPLogbook includes a built-in Hamlib proxy server (port 4534) so external applications (WSJT-X, JTDX, FLDigi) can share the same radio simultaneously without COM port conflicts.",
    },
    ManualSection {
        id: "logging",
        title: "3. QSO Logging & SQLite DB",
        body: "At the core of SPLogbook is a high-performance SQLite database engine running in WAL (Write-Ahead Logging) mode:\n\n\
- Data Integrity & Safety: WAL mode with transactional commits protects your log against corruption even during sudden power loss. Rolling automatic backups keep up to 10 historical revisions.\n\
- High-Speed Indexing: Specialized SQLite indexes guarantee instantaneous searching and filtering even with logs exceeding ratusan thousands of QSOs.\n\
- Smart QSO Entry Form: Automatic uppercase callsign normalization, Super Check Partial (SCP) suggestions, ITU prefix resolution, and CTY.DAT entity matching.\n\
- Duplicate Detection: Instant visual [DUPE!] warning whenever a station has already been worked on the current band and mode.\n\
- QSO Audio Recorder: Record live receiver audio from your soundcard and attach WAV recordings directly to individual QSO records.\n\
- Multi-Journal Support: Maintain multiple independent logs (Home station, Club callsign, /P field activations, Contest logs).",
    },
    ManualSection {
        id: "shortcuts",
        title: "4. Keyboard Shortcuts",
        body: "SPLogbook is designed for fast, hands-on-keyboard operation without reaching for the mouse:\n\n\
Function Keys (Primary):\n\
- F1 — Help window and complete keyboard shortcuts reference\n\
- F2 — Save current QSO (Contest & DX standard)\n\
- F3 — Wipe QSO entry form and focus Callsign input\n\
- F4 — Force immediate online Callbook lookup (QRZ / HamQTH)\n\
- F5 — Refresh logbook table from database\n\
- F6 — Open Send DX Spot dialog\n\
- F7 — Toggle transceiver PTT (TX / RX) via CAT\n\
- F8 — Open Voice Keyer panel\n\
- F11 — Toggle Fullscreen mode\n\
- F12 — Open built-in User Manual (this window)\n\n\
Modifier Shortcuts (Ctrl / Cmd):\n\
- Ctrl+N — New QSO (clears form and focuses Callsign)\n\
- Ctrl+W — Wipe QSO form\n\
- Ctrl+S — Save station configuration and database\n\
- Ctrl+F — Open Advanced Logbook Filter\n\
- Ctrl+L — Focus / toggle Logbook table\n\
- Ctrl+D — Open / toggle DX Cluster panel\n\
- Ctrl+B — Open Bandmap window\n\
- Ctrl+M — Open World Map & Grayline Terminator\n\
- Ctrl+K — Open CW Keyer & Macros terminal\n\
- Ctrl+P — Open Station Profiles manager\n\
- Ctrl+E — Open configurable CSV Exporter\n\
- Ctrl+I — Open ADIF Import dialog\n\
- Ctrl+T — Cycle UI color theme (Dark / Daylight / High-Contrast)\n\
- Ctrl+Z — Undo last deleted QSO\n\
- Ctrl+Y / Ctrl+Shift+Z — Redo\n\
- Ctrl+Shift+S — Open Statistics & Charts window\n\
- Ctrl+Shift+P — Open Command Palette\n\
- Ctrl+Q — Safely exit application",
    },
    ManualSection {
        id: "cluster",
        title: "5. DX Cluster & Bandmap",
        body: "The DX Cluster module delivers real-time DX spots and award alerts:\n\n\
- Built-in Telnet Client: Connects to any DXSpider, AR-Cluster, or CC-Cluster node with automatic reconnection and live command input (e.g. sh/dx 25).\n\
- Smart Deduplication: Sliding window filter prevents duplicate spots from flooding your table.\n\
- Visual & Audio Alerts:\n\
  * Magenta Star (ATNO): All-Time New One DXCC entity with optional audio chime.\n\
  * Emerald Badge: New Band or New Mode for a worked DXCC country.\n\
- Advanced Filtering: Filter by HF / WARC / VHF bands, modes (CW, SSB, FT8/FT4, Digi), continent, or hide automated RBN skimmer spots.\n\
- Click-to-Tune: Clicking any spot tunes your transceiver VFO to the spot frequency, sets the mode, and populates the QSO entry form.\n\
- Visual Bandmap: Real-time frequency scale displaying active stations with age-based color fading.",
    },
    ManualSection {
        id: "propagation",
        title: "6. HF Propagation, Solar & EME",
        body: "SPLogbook combines real-time ionospheric modeling with live geophysical telemetry:\n\n\
- Space Weather Telemetry (NOAA SWPC): Live Solar Flux Index (SFI), Sunspot Number (SSN), geomagnetic A and Kp indices, X-ray background flux, and solar wind speed.\n\
- VOACAP-lite HF Engine: Computes great-circle midpoint, solar zenith angle, D-layer absorption, critical/maximum usable frequencies (foF2, MUF, LUF, FOT), and band opening probability (0-100%) with estimated S-meter signal level.\n\
- Dynamic Propagation Badge: Entering a callsign immediately displays predicted circuit reliability on all HF bands.\n\
- EME (Earth-Moon-Earth) Planner: Real-time topocentric Moon azimuth, elevation, range, Doppler shift, path loss, degradation, and mutual visibility windows.",
    },
    ManualSection {
        id: "digital",
        title: "7. Digital Modes (WSJT-X, JS8, FLDigi)",
        body: "SPLogbook integrates directly with external digital mode suites:\n\n\
- WSJT-X / JTDX UDP Bridge (port 2237):\n\
  * Decodes binary UDP frames: Heartbeat, Status, Decode, QSO Logged, and Clear.\n\
  * Automatically logs completed FT8/FT4/JT65/Q65 QSOs with exact SNR dB reports and grid locators.\n\
  * Highlights calling stations in real time when they represent an ATNO or new band/mode.\n\
- JS8Call (JSON TCP/UDP port 2442): Captures RX.ACTIVITY, RIG.FREQ, and LOG.QSO frames.\n\
- FLDigi (XML-RPC port 7362): Reads frequency, mode, and QSO fields for PSK31, RTTY, Olivia, and Contestia.\n\
- N1MM UDP Broadcasts: Emits standard XML ContactInfo packets so tools like GridTracker track your logged QSOs.",
    },
    ManualSection {
        id: "satellites",
        title: "8. Satellite Tracking & Doppler",
        body: "Dedicated tools for LEO amateur satellites and the QO-100 geostationary transponder:\n\n\
- SGP4/SDP4 Orbital Propagator: Uses NORAD Two-Line Elements (TLE) with one-click online updates.\n\
- Live Orbital Telemetry: Computes azimuth, elevation, slant range, radial velocity, and ground footprint.\n\
- Automatic CAT Doppler Correction: Continuously calculates uplink and downlink Doppler shift:\n\
  Δf = -f0 * (v_slant / c)\n\
  and updates transceiver VFO frequencies via CAT.\n\
- Azimuth/Elevation Rotator Tracking: Drives 2-axis satellite rotators via rotctld.",
    },
    ManualSection {
        id: "waterfall",
        title: "9. SDR Audio Spectrum & Waterfall",
        body: "Real-time audio spectrum analyzer and waterfall display:\n\n\
- Cross-Platform Audio Capture (cpal): Captures audio directly from your transceiver USB audio codec, line-in, or virtual audio cable.\n\
- RustFFT Engine: Fast Hann-windowed FFT computation minimizing spectral leakage.\n\
- Configurable Controls:\n\
  * FFT sizes: 512, 1024, 2048, or 4096 bins.\n\
  * Adjustable Gain (dB), Noise Floor (dB), and Pause/Resume inspection.\n\
  * High-contrast color gradient for spotting weak CW and digital signals.\n\
- Flexible Docking: Use inline inside the main workspace or undock to a second monitor.",
    },
    ManualSection {
        id: "contest",
        title: "10. Contest Engine & Cabrillo",
        body: "High-speed contesting module compliant with international contest rules:\n\n\
- Supported Contests: CQ WW DX, CQ WPX, ARRL DX, IARU HF Championship, SP DX Contest, VHF/UHF contests, and custom user-defined rules.\n\
- Smart Exchange Parser: Single-line exchange entry (e.g. '599 001' or '599 15') automatically splits RST, serial number, CQ/ITU zone, or province.\n\
- Live Score & Rate Sheet: Real-time QSO/hour rate (10-min and 60-min), multiplier matrix by band, and total score calculation.\n\
- Export Formats: Generates official Cabrillo 3.0 logs, EDI (VHF/UHF) logs, and ADIF/ADX files.",
    },
    ManualSection {
        id: "awards",
        title: "11. Awards & Field Programs",
        body: "Automatic progress tracking across major international and national award programs:\n\n\
- DXCC: Mixed, Band (160m–6m), Mode (CW, Phone, Digital), and DXCC Challenge matrix with Worked vs. Confirmed status.\n\
- WAZ, WAS & WAE: Tracks all 40 CQ Zones, 50 US States (with built-in States Browser), and European WAE entities.\n\
- Polska Gmina Award (PGA): Complete database of all 2,477 Polish municipalities.\n\
- SOTA, POTA, WWFF & IOTA: Dedicated reference fields, built-in IOTA Island Browser, and compliant SOTA CSV export.",
    },
    ManualSection {
        id: "qsl",
        title: "12. QSL Designer & Avery PDF Labels",
        body: "Manage paper QSL confirmations and print address/QSO labels:\n\n\
- Vector PDF Label Exporter: Generates print-ready A4 PDF sheets for standard Avery templates (3x8, 3x7, 2x8, 2x7 labels per page).\n\
- Smart Print Queue: Filter QSOs waiting for paper QSLs by band, mode, or date range, with automatic multi-page pagination.\n\
- Margin Calibration: Fine-tune horizontal and vertical offsets in millimeters for any laser or inkjet printer.\n\
- Visual QSL Card Designer: Customize background, callsign typography, QSO data table, and station notes.",
    },
    ManualSection {
        id: "export",
        title: "13. Cloud Sync, ADIF & REST API",
        body: "Comprehensive data exchange and cloud logbook synchronization:\n\n\
- ADIF 3.1.5 & ADX: Full import/export compatibility with standard .adi and XML .adx files.\n\
- Online Logbooks: Direct two-way or upload synchronization with ARRL LoTW (via TQSL), eQSL.cc, Club Log, QRZ.com Logbook, HamQTH, Cloudlog, and HRDLog.net.\n\
- Resilient Upload Scheduler: Persistent queue with automatic retry and exponential backoff.\n\
- Online Database Updater: One-click download and live reload of CTY.DAT (DXCC prefixes), MASTER.SCP (Super Check Partial), and LoTW user activity lists.\n\
- Built-in REST & WebSocket API (port 8080): Local Axum server for third-party integrations.",
    },
    ManualSection {
        id: "plugins",
        title: "14. User Plugins (Rhai) & Marketplace",
        body: "Extend SPLogbook safely with sandboxed user scripts:\n\n\
- Rhai Scripting Sandbox: Write custom .rhai plugins without recompiling the application. Strict operation and memory limits keep the UI responsive.\n\
- Lifecycle Hooks: on_startup(), on_qso_logged(call, band, mode, freq, is_atno), on_dx_spot(spotter, call, freq, comment), on_rig_state(freq, mode, connected), and on_band_opened(band).\n\
- Command Bridge: Scripts can trigger CW macros, voice keyer messages, rotator headings, audio alerts, and QSO fields.\n\
- Built-in Plugin Marketplace: One-click installation of curated plugins (POTA Helper, CW Macros, Contest Assistant, Rotor Assistant, Band Activity Logger).",
    },
    ManualSection {
        id: "p2p",
        title: "15. Encrypted LAN / P2P Sync",
        body: "Multi-operator collaboration without relying on external cloud servers:\n\n\
- End-to-End Encryption: Peer-to-peer synchronization over LAN or VPN secured with XChaCha20-Poly1305 authenticated encryption and Argon2id key derivation.\n\
- Real-Time Multi-Op TCP Server: Broadcasts newly logged QSOs and live operator band/mode status across all connected operating positions.\n\
- Field-Ready: Works completely offline over a local Wi-Fi router during Field Day or DXpeditions.",
    },
    ManualSection {
        id: "os_setup",
        title: "16. Windows & Linux Setup",
        body: "SPLogbook runs natively on 64-bit Windows 10/11 and GNU/Linux. Key platform notes:\n\n\
GNU/Linux (x86_64):\n\
1. USB Serial Port Permissions:\n\
   Add your user account to the serial group to access /dev/ttyUSB* and /dev/ttyACM*:\n\
   - Debian / Ubuntu / Mint / Raspberry Pi OS:\n\
     sudo usermod -aG dialout $USER\n\
   - Arch / Manjaro / Fedora / openSUSE:\n\
     sudo usermod -aG uucp $USER\n\
   Log out and log back in after running the command.\n\
2. Data Directories (XDG Standard):\n\
   - Databases & logs: ~/.local/share/splogbook/\n\
   - Configuration: ~/.config/splogbook/\n\n\
Microsoft Windows (10 / 11 64-bit):\n\
1. Virtual COM Ports: Install the transceiver manufacturer's USB UART driver (Silicon Labs CP210x, FTDI, or CH340) and select the detected COM port in CAT Settings.\n\
2. Windows Firewall: Allow SPLogbook on private networks when using the WSJT-X UDP bridge (2237), CAT proxy (4534), or Multi-Op LAN sync.\n\
3. Data Directory: Stored in %APPDATA%\\SPLogbook\\",
    },
    ManualSection {
        id: "troubleshooting",
        title: "17. Troubleshooting FAQ",
        body: "Solutions to common technical questions:\n\n\
Q: My transceiver does not connect via CAT.\n\
A: 1. Ensure no other program (WSJT-X, OmniRig, Putty) is holding the same COM/tty serial port open.\n\
   2. Verify that the Baud Rate in CAT Settings matches your radio's menu setting (e.g. 38400 or 115200).\n\
   3. In Serial mode, enable 'Auto-start rigctld' or choose the direct Kenwood/Icom backend.\n\n\
Q: Clicking 'Update Online Database' — how do I know it finished?\n\
A: SPLogbook downloads the latest CTY.DAT, MASTER.SCP, and LoTW user activity list in the background, reloads them immediately into memory, and shows a confirmation toast notification.\n\n\
Q: DX Cluster disconnects immediately after connecting.\n\
A: DXSpider and AR-Cluster nodes require a valid amateur callsign at login. Set your callsign in Station Settings (if left as N0CALL, SPLogbook automatically uses a fallback guest callsign).\n\n\
Q: Can I move my logbook database to another computer?\n\
A: Yes! Click 'Open Data Directory' below and copy your .db file (or create a ZIP backup in the Backup Manager). SQLite files are 100% compatible across Windows and Linux.",
    },
];

fn active_sections(lang: crate::core::i18n::Language) -> &'static [ManualSection] {
    match lang {
        crate::core::i18n::Language::Pl => MANUAL_SECTIONS,
        _ => MANUAL_SECTIONS_EN,
    }
}

/// Kontekstowa ikona pomocy „?” — otwiera podręcznik na wskazanej sekcji.
pub fn help_button(app: &mut SpLogApp, ui: &mut egui::Ui, section: &'static str) {
    let tip = crate::core::i18n::tr_or(
        app.current_language,
        "Pomoc — otwórz podręcznik na tej sekcji",
        "Help — open User Manual at this section",
    );
    if ui.small_button("?").on_hover_text(tip).clicked() {
        app.show_user_manual = true;
        app.manual_section = Some(section.to_string());
    }
}

fn section_by_id(id: &str) -> usize {
    MANUAL_SECTIONS.iter().position(|s| s.id == id).unwrap_or(0)
}

/// Okno „Instrukcja obsługi” — panel boczny z sekcjami + przewijana, czytelnie sformatowana treść.
pub fn render_user_manual_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_user_manual {
        return;
    }
    let lang = app.current_language;
    let sections = active_sections(lang);
    let mut is_open = app.show_user_manual;

    let selected = if let Some(section) = app.manual_section.take() {
        section_by_id(&section)
    } else {
        app.manual_selected.min(sections.len().saturating_sub(1))
    };

    let mut new_selected = selected;
    let mut close_requested = false;

    let screen = ctx.content_rect();
    let max_w = (screen.width() - 32.0).clamp(640.0, 940.0);
    let max_h = (screen.height() - 48.0).clamp(420.0, 680.0);
    let default_w = 860.0_f32.min(max_w);
    let default_h = 560.0_f32.min(max_h);

    let win_title = crate::core::i18n::tr_or(
        lang,
        "📖 Instrukcja obsługi i podręcznik użytkownika",
        "📖 User Manual & Operator Guide",
    );

    let search_id = egui::Id::new("user_manual_search_filter");
    let mut search_query: String = ctx.data_mut(|d| d.get_temp(search_id).unwrap_or_default());

    egui::Window::new(win_title)
        .id(egui::Id::new("splogbook_user_manual_window"))
        .open(&mut is_open)
        .default_size([default_w, default_h])
        .min_size([620.0, 380.0])
        .max_size([max_w, max_h])
        .constrain_to(screen)
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.set_max_width(max_w - 20.0);

            // Górny pasek nawigacji i wyszukiwarki
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🔍")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );
                let hint = crate::core::i18n::tr_or(
                    lang,
                    "Szukaj w podręczniku (np. CAT, WSJT-X, skróty)...",
                    "Search manual (e.g. CAT, WSJT-X, shortcuts)...",
                );
                ui.add(
                    egui::TextEdit::singleline(&mut search_query)
                        .hint_text(hint)
                        .desired_width(260.0),
                );
                if !search_query.is_empty()
                    && ui
                        .small_button("✕")
                        .on_hover_text(crate::core::i18n::tr_or(lang, "Wyczyść", "Clear"))
                        .clicked()
                {
                    search_query.clear();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let next_label = crate::core::i18n::tr_or(lang, "Następny ▶", "Next ▶");
                    if ui
                        .add_enabled(
                            new_selected + 1 < sections.len(),
                            egui::Button::new(next_label),
                        )
                        .clicked()
                    {
                        new_selected = (new_selected + 1).min(sections.len() - 1);
                    }
                    ui.label(
                        egui::RichText::new(format!("{} / {}", new_selected + 1, sections.len()))
                            .size(11.5)
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );
                    let prev_label = crate::core::i18n::tr_or(lang, "◀ Poprzedni", "◀ Prev");
                    if ui
                        .add_enabled(new_selected > 0, egui::Button::new(prev_label))
                        .clicked()
                    {
                        new_selected = new_selected.saturating_sub(1);
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let body_height = (ui.available_height() - 44.0).clamp(260.0, 540.0);
            let q_lower = search_query.trim().to_lowercase();

            ui.horizontal(|ui| {
                // Lewy panel boczny: spis treści w układzie pionowym
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(15, 23, 42, 180))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(6))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85)))
                    .show(ui, |ui| {
                        ui.set_width(235.0);
                        ui.set_height(body_height);
                        egui::ScrollArea::vertical()
                            .id_salt("manual_toc_scroll")
                            .max_height(body_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(220.0);
                                    for (i, sec) in sections.iter().enumerate() {
                                        if !q_lower.is_empty()
                                            && !sec.title.to_lowercase().contains(&q_lower)
                                            && !sec.body.to_lowercase().contains(&q_lower)
                                        {
                                            continue;
                                        }
                                        let is_sel = new_selected == i;
                                        let text = egui::RichText::new(sec.title)
                                            .size(12.5)
                                            .color(if is_sel {
                                                egui::Color32::from_rgb(56, 189, 248)
                                            } else {
                                                egui::Color32::from_rgb(226, 232, 240)
                                            })
                                            .strong();
                                        let resp = ui.add_sized(
                                            [218.0, 26.0],
                                            egui::Button::selectable(is_sel, text),
                                        );
                                        if resp.clicked() {
                                            new_selected = i;
                                        }
                                    }
                                });
                            });
                    });

                ui.add_space(6.0);

                // Prawy panel: treść rozdziału z zawijaniem wierszy w układzie pionowym
                let content_width = (ui.available_width() - 4.0).max(340.0);
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 41, 59, 140))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(12))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85)))
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.set_height(body_height);
                        egui::ScrollArea::vertical()
                            .id_salt("manual_content_scroll")
                            .max_height(body_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let inner_w = (content_width - 28.0).max(300.0);
                                    ui.set_max_width(inner_w);

                                    let sec = &sections[new_selected.min(sections.len() - 1)];
                                    ui.label(
                                        egui::RichText::new(sec.title)
                                            .size(17.5)
                                            .strong()
                                            .color(egui::Color32::from_rgb(56, 189, 248)),
                                    );
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(8.0);

                                    for paragraph in sec.body.split("\n\n") {
                                        for line in paragraph.lines() {
                                            let trimmed = line.trim();
                                            if trimmed.is_empty() {
                                                continue;
                                            }
                                            if let Some(rest) = trimmed.strip_prefix("- ") {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.set_max_width(inner_w);
                                                    ui.label(
                                                        egui::RichText::new("•")
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(
                                                                56, 189, 248,
                                                            )),
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(rest).size(13.0),
                                                        )
                                                        .wrap(),
                                                    );
                                                });
                                            } else if let Some(rest) = trimmed.strip_prefix("* ") {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.set_max_width(inner_w);
                                                    ui.add_space(12.0);
                                                    ui.label(
                                                        egui::RichText::new("◦").color(
                                                            egui::Color32::from_rgb(52, 211, 153),
                                                        ),
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(rest).size(12.5),
                                                        )
                                                        .wrap(),
                                                    );
                                                });
                                            } else {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(trimmed).size(13.0),
                                                    )
                                                    .wrap(),
                                                );
                                            }
                                            ui.add_space(3.0);
                                        }
                                        ui.add_space(8.0);
                                    }
                                });
                            });
                    });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                let open_dir_label = crate::core::i18n::tr_or(
                    lang,
                    "📁 Otwórz katalog danych programu",
                    "📁 Open Data Directory",
                );
                let open_dir_tip = crate::core::i18n::tr_or(
                    lang,
                    "Otwiera folder z bazą danych i plikami konfiguracyjnymi",
                    "Opens folder containing database and station configuration files",
                );
                if ui.button(open_dir_label).on_hover_text(open_dir_tip).clicked() {
                    if let Some(parent) = app.config_file_path.parent() {
                        let _ = open::that(parent);
                    }
                }
                let issue_label = crate::core::i18n::tr_or(
                    lang,
                    "🌐 Zgłoś uwagę lub błąd na GitHubie",
                    "🌐 Report Issue on GitHub",
                );
                if ui.button(issue_label).clicked() {
                    let _ = open::that("https://github.com/sp6ina/SPLogbook/issues/new");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_label =
                        crate::core::i18n::tr_or(lang, "Zamknij (Esc)", "Close (Esc)");
                    if ui.button(close_label).clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    ctx.data_mut(|d| d.insert_temp(search_id, search_query));
    app.manual_selected = new_selected;
    app.show_user_manual = is_open && !close_requested;
}
