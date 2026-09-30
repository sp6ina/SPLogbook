# DESIGN_3_1_UPDATER_SIGNATURES.md

## 1. Streszczenie
Niniejszy dokument przedstawia analizę oraz projekt architektoniczny wdrożenia asymetrycznych podpisów cyfrowych (Ed25519) do mechanizmu automatycznych aktualizacji SPLogbook. Dodatkowo definiuje nowy cel architektoniczny: ścisły podział systemu na niezależny od platformy **wspólny rdzeń bezpieczeństwa** oraz wyizolowane **instalatory platformowe** dla systemów Windows i Linux.

Bezpieczeństwo autentyczności wydań zależy wyłącznie od odpowiedniej ochrony klucza prywatnego (np. użycia Protected Environments) oraz szczelności i audytowalności procesu podpisywania w GitHub Actions (aby wrogie skrypty nie uzyskały dostępu do załadowanego sekretu). Zabezpieczenia chronią zarówno przed wrogimi intruzami, jak i błędami platform pobierania.

## 2. Zweryfikowany stan obecny (Windows i Linux)
Na podstawie dogłębnej weryfikacji repozytorium ustalono następujący stan obecny:

**Ogólne:**
1. **Źródło metadanych i plików:** GitHub Releases API (`https://api.github.com/repos/sp6ina/SPLogbook/releases/latest`) oraz `browser_download_url`.
2. **Obecny błąd Windows:** Aktualizator pobiera paczkę `.zip`, nadpisuje nią bezpośrednio plik `SPLogbook.exe` i usiłuje uruchomić ZIP.
3. **Automatyczna instalacja:** Instalacja jest ręcznie inicjowana przez użytkownika z poziomu GUI, bez weryfikacji podpisów asymetrycznych (których brak).

**Obecne wsparcie Linux:**
1. **Generowane artefakty Linux:** Publikowany jest wyłącznie plik `SPLogbook-Linux-x86_64.tar.gz`. *(Potwierdzone w workflow: `build-and-release.yml`)*
2. **Formaty dystrybucji:** AppImage, deb, rpm, czy Flatpak **nie są** obecnie generowane. *(Potwierdzone w workflow)*
3. **Struktura archiwum tar.gz:** Archiwum zawiera cały katalog przenośny (portable) `SPLogbook-Linux-x86_64/`, w którym znajduje się binarka `splogbook`, skrypt `run.sh`, oraz katalogi m.in. `databases/`, `assets/`, `hamlib/`. To nie jest pojedynczy plik. *(Potwierdzone w workflow)*
4. **Obecny błąd aktualizatora Linux:** Podobnie jak w systemie Windows, `updater.rs` na Uniksach pobiera paczkę `tar.gz`, po czym funkcją `std::fs::rename` brutalnie nadpisuje plik bieżącej binarki pobranym archiwum `.tar.gz` i próbuje go uruchomić (`Command::new().spawn()`), niszcząc instalację. *(Potwierdzone w kodzie: `updater.rs:369`)*
5. **Lokalizacja i dane:** Użytkownik prawdopodobnie uruchamia aplikację w rozpakowanym katalogu w przestrzeni domowej. Bazy danych dołączone w `tar.gz` podpowiadają strukturę przenośną. Updater aktualnie nie weryfikuje środowiska uruchomienia ani praw dostępu. *(Potwierdzone w kodzie)*

## 3. Cel architektoniczny: Podział ról
System aktualizacji zostaje podzielony na dwie niezależne logicznie warstwy:

### 1. Wspólny Rdzeń Bezpieczeństwa (Security Core)
Odpowiada wyłącznie za kryptografię i integralność, niezależnie od systemu operacyjnego:
- Pobranie surowych bajtów manifestu oraz sygnatury.
- Dekodowanie podpisu Base64 i identyfikacja `key_id`.
- Weryfikacja kryptograficzna Ed25519 (odrzucenie, jeśli błąd).
- Parsowanie zaufanego manifestu (JSON), walidacja `manifest_version`, `product`, `channel`, `commit` (SemVer).
- Ochrona przed atakiem downgrade.
- Jednoznaczny wybór platformy, architektury i odpowiedniego typu paczki.
- Kontrola limitu `MAX_UPDATE_SIZE` i weryfikacja zgodności z `asset.size`.
- Strumieniowe pobieranie pliku przy jednoczesnym locie przeliczania SHA-256.
- Usuwanie niepełnych i uszkodzonych plików.
- Przekazanie zweryfikowanego w 100% zaufanego pliku tymczasowego do instalatora.

### 2. Instalatory Platformowe (Platform Installers)
Odpowiadają wyłącznie za wdrożenie w system operacyjny:
- Instalator **nie może samodzielnie pobierać** niezaufanego artefaktu.
- Nie może pomijać weryfikacji podpisu ani liczenia SHA-256 (dostaje tylko gotowy produkt od rdzenia).
- Nie może zmieniać decyzji rdzenia dotyczącej kanałów.
- Posiada wdrożoną logikę bezpiecznej aplikacji paczki w dane środowisko (Windows, Linux) zależnie od typu formatu i uprawnień systemowych.

## 4. Nowy podział zadania 3.0
Zastąpiono dotychczasowe jedno zadanie naprawy błędu następującym podziałem na fazę koncepcyjną i przygotowawczą, która musi odbyć się **przed** napisaniem jakiegokolwiek kodu instalatorów:

- **3.0.A Projekt aktualizacji Windows** -> Wymaga powstania `DESIGN_3_0_A_WINDOWS_UPDATE.md`.
- **3.0.B Projekt aktualizacji Linux** -> Wymaga powstania `DESIGN_3_0_B_LINUX_UPDATE.md`.
- **3.0.C Wspólny interfejs instalatorów platformowych** -> Wymaga powstania `DESIGN_3_0_C_PLATFORM_INSTALLERS.md`.

## 5. Model formatów dystrybucji (`package_type`)
Element `asset` w manifeście zostaje rozszerzony o obowiązkowe pole: `package_type`.
Nieznany `package_type` ma natychmiast powodować odrzucenie artefaktu z puli wyboru. 

**Formaty obecnie wspierane (generowane przez CI):**
- Windows: `zip`
- Linux: `tar.gz`

**Formaty planowane na przyszłość (nieaktywne w walidacji pobierania):**
- Windows: `exe`, `msi`
- Linux: `appimage`, `deb`, `rpm`, `flatpak`

## 6. Jednoznaczny wybór artefaktu (Selektor)
Reguła wyboru zostaje zmieniona na precyzyjną triadę:
`platform + arch + package_type`

**Polityka wyboru i preferencji dla wielu formatów (jeśli wprowadzone):**
Aplikacja sama musi wykryć, z jakiego typu pakietu pracuje.
*Linux (przykład logiki):*
1. Preferowane `appimage`, jeśli aplikacja wykryje, że uruchomiona jest w formacie AppImage.
2. Preferowane `tar.gz`, jeśli uruchomiona jest z katalogu w trybie portable.
3. Dla środowiska zainstalowanego menedżerem (np. `/usr/bin` poprzez `deb` / `rpm` / `flatpak`), updater **nie wykonuje** samodzielnej podmiany. Informuje o wydaniu nowej wersji.
4. Jeżeli instalacja nie jest przez aplikację rozpoznana w żaden sposób, updater nie zgaduje – odmawia auto-aktualizacji podając komunikat ostrzegawczy o nierozpoznanej strukturze systemu.

**Wymagania ścisłe dla wyboru (Core):**
- Musi zostać wybrany **dokładnie jeden** artefakt z manifestu.
- Zero dopasowań oznacza brak aktualizacji (odpowiedniej paczki) dla danego systemu.
- Wiele dopasowań bez precyzyjnej jednoznacznej preferencji (np. dwa ZIPy dla Windows 64-bit w jednym manifeście) powoduje uznanie JSON-a za niejednoznaczny i jego odrzucenie.
- Nazwa `filename` musi być unikalna w całym dokumencie JSON.
- Kombinacja `platform + arch + package_type` musi być unikalna.
- Updater pod żadnym pozorem nie może wybierać pierwszego pasującego pliku polegając na naiwnym filtrowaniu po jego rozszerzeniu (musi analizować flagi JSON).

## 7. Granica zaufania

**Zaufane (Trusted Boundary):**
- Wbudowany w kod aplikacji klucz publiczny Ed25519 (oraz jawna lista jego parametrów i wersji wbudowana w kod źródłowy).
- Surowe bajty weryfikowanego dokumentu, jeśli funkcja biblioteki kryptograficznej potwierdziła ich zgodność z sygnaturą.

**Niezaufane (Untrusted Input):**
- Odpowiedź API GitHub, metadane wydania, logi.
- Wartość `digest` z GH (służy tylko transportowi).
- Niesprawdzony podpis cyfrowy i niezweryfikowany manifest.
- **Lokalna konfiguracja użytkownika:** Traktowana bezwarunkowo jako wejście niezaufane. Konfiguracja może określać wyłącznie bezpieczne preferencje (np. opcje GUI, włącz/wyłącz auto-check, wybór kanału "stable"). Lokalna konfiguracja **nie może**: dodawać kluczy publicznych, zastępować kluczy, wyłączać weryfikacji Ed25519/SHA-256, zezwalać na downgrade, omijać polityki kanałów, zmieniać product, `manifest_version`, `minimum_updater_version`, ani oczekiwanej platformy / arch. Niepoprawne wartości pliku konfiguracji powodują zablokowanie operacji złośliwych i powrót na bezpieczne ustawienia domyślne.

## 8. Format Manifestu (Kanonikalizacja i Pola)

Podpis dotyczy precyzyjnie **dokładnych bajtów pliku** `release-manifest.json`, które opublikowano bez wprowadzania jakichkolwiek modyfikacji. Zrezygnowano z wymogu kanonikalizacji JSON-a.

Brak obowiązkowego pola powoduje odrzucenie. Domyślne wartości z atrybutów `#[serde(default)]` są zabronione na polach chronionych.

Nazwa pliku z treścią: `release-manifest.json`
Nazwa pliku podpisu: `release-manifest.json.sig`

**Format struktury (Przykład Linux):**
```json
{
  "manifest_version": 1,
  "key_id": "splogbook-release-2026-01",
  "product": "SPLogbook",
  "version": "1.2.0",
  "channel": "stable",
  "minimum_updater_version": "1.2.0",
  "commit": "a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f0",
  "assets": [
    {
      "platform": "linux",
      "arch": "x86_64",
      "package_type": "tar.gz",
      "filename": "SPLogbook-Linux-x86_64.tar.gz",
      "size": 18233441,
      "sha256": "3d5f0e4c2f76c58916ec258f246851bea091d14d4247a2fc3e18694461b1816e"
    }
  ]
}
```

**`minimum_updater_version`:** Egzekwowane dopiero przez **nowy updater** (od 1.2.0). Starszy klient nie potrafi parsnąć ani zrozumieć bezpiecznego JSON-a, co samoistnie uchroni nowy układ przed złamaniem wstecznym.

**`commit`:** Dla `manifest_version = 1`, pole to musi zawierać **dokładnie 40 znaków** używając wyłącznie małych znaków szesnastkowych (`0-9`, `a-f`). Reprezentuje pełny identyfikator SHA-1 Git. Skrócone hashe zabronione. Klient odczytuje go wyłącznie jako referencję bezpieczeństwa (nie musi pobierać kodu z Git by go ocenić).

## 9. Format klucza, podpisu i SHA-256 (Bootstrap)

* **Klucz publiczny Ed25519:** Zaufane klucze publiczne muszą być osadzone w kodzie bez użycia Base64, jako natywne tablice bajtów, co gwarantuje ich wymiar 32 już na etapie kompilacji:
  `const PUBLIC_KEY: [u8; 32] = [...];`
* **Podpis:** Zapisany w pliku `.sig` w formacie Base64 (po zdekodowaniu dokładnie 64 bajty).
* **SHA-256:** Dokładnie 64 małe znaki szesnastkowe.

**Tabela zaufanych kluczy (Wpis):**
Zawiera: `key_id`, `public_key: [u8; 32]`, `status` (aktywny/wycofany), `min_version`, oraz `max_version` (opcjonalną maksymalną dozwoloną wersję).

**Kolejność weryfikacji i bootstrap:**
Niezachwiana, twarda kolejność walidacji, chroniąca przed przedwczesnym zaufaniem do zawartości:
1. Pobierz surowe bajty manifestu i pliku `.sig`.
2. Zdekoduj podpis Base64.
3. Zweryfikuj podpis **kolejno** małą listą aktywnych zaufanych kluczy publicznych na tych samych surowych bajtach manifestu.
4. Jeżeli żaden z kluczy nie odniesie sukcesu weryfikacyjnego na pakiecie z sygnaturą - odrzuć natychmiast i zniszcz bufory pobierania.
5. Zapamiętaj precyzyjnie wpis klucza (parametry i progi `min/max`), który poprawnie go zweryfikował.
6. Dopiero teraz sparsuj autoryzowany bajtowo manifest JSON do wewnętrznej struktury aplikacji.
7. Sprawdź, czy `manifest.key_id` z JSONa, jest co do litery zgodny z używanym zapamiętanym wcześniej ID.
8. Zweryfikuj `manifest_version`, `product`, `channel` i resztę twardych pól.
9. Sparsuj pole `manifest.version` przez parser `SemVer`.
10. Sprawdź, czy `manifest.version` łapie się pomiędzy limit `min_version` i `max_version` nałożony na użyty przed sekundą fizyczny klucz publiczny. Niezgodność z wersją klucza odrzuca pakiet.
11. Dopiero po tym etapie przejdź do: sprawdzania downgrade'u na obiekcie i uruchomienia selekcji na polach paczek z architektury.

## 10. Dokładny rozmiar artefaktu

- Liczba pobranych bajtów pliku musi być **dokładnie równa** wartości określonej w `asset.size` z podpisanego manifestu. Marginesy błędu są niedozwolone.
- Przy przekroczeniu wielkości w locie (więcej bajtów niż uważa manifest), transfer zrywa się z błędem.
- W razie urwania transferu ze strony repozytorium (mniej bajtów niż uważa manifest), artefakt oznaczony jest jako śmieć i pobieranie zawodzi.
- Częściowy plik po operacjach powyżej usuwa się od razu (brak jakichkolwiek prób podawania go na wejście instalatorom platformowym).
- **Limit MAX_UPDATE_SIZE:** Odgórny, wkompilowany limit, uniemożliwiający żądanie `asset.size <= MAX_UPDATE_SIZE` blokujące wyczerpanie dysku systemowego i pamięci. Wartość tego limitu musi zostać wyznaczona na podstawie obserwacji największych pakietów `SPLogbook` doliczając rozsądny bufor dla plików dodatkowych w przyszłości przy uwzględnieniu standardu partycji root Linuksa. Nagłówek `Content-Length` może zostać użyty do natychmiastowego zamknięcia łącza jako heurystyka, ale to pętla zliczająca fizyczne bloki od strony TCP jest gwarantem zatrzymania pobierania.

## 11. Polityka Instalacji Linux

*(Dla 3.0.B DESIGN LINUX UPDATE)* Właściwa instalacja jest uzależniona od paczki uruchomieniowej.

**5.1 AppImage (Format planowany na przyszłość)**
Jeśli instalacja to zapisywalny AppImage: pobierz nową binarkę do `tmp`, zweryfikuj z rdzeniem. Nadaj chmod executable (wymagane wewnątrz), zachowaj obecny do rollback, wykonaj `rename` na tej samej partycji i uruchom.
Jeśli niezapisywalny (np. readonly montaż) - wstrzymaj i nakaż aktualizację manualną bez hasła.

**5.2 Portable tar.gz (Format obecnie używany)**
Jeśli to rozpakowany folder: pobierz `.tar.gz`. Wspólny rdzeń akceptuje paczkę. Rozpakuj go do całkowicie nowego folderu `tmp`. Zabronione jest bezpośrednie sypanie wypakowanym materiałem do działającego aktualnie folderu z racji groźby nadpisania plików `.db`.
Sprawdź w `tmp` poprawne wyjście binarki. Przygotuj nowy katalog instalacyjny, zamień kontrolowanym przejściem powłoki katalogi na ten sam mountpoint z zabezpieczeniem do tyłu (rollback directory).
Auto-updater użytkownika bezwzględnie nie nadpisuje logów bazy danych użytkownika `serviceLOG.db` ani poświadczeń użytkownika. Zostawia je bezpiecznie nietknięte.

**5.3 Pakiet DEB / RPM (Format planowany)**
Dla programów z menedżerów pakietów Linuksowych (ścieżki ustandaryzowane np. `/usr/bin`), auto-updater nigdy nie podmienia samodzielnie plików systemowych, ani **nie uruchamia niejawnego sudo**, by zmusić się do przepchnięcia praw Root'a. Proces ten ma na celu pobranie paczki z weryfikacją (o ile jest chęć ze strony admina) lub tylko zasygnalizowanie użytkownikowi o nowej wersji wydanej w deb-repozytorium z przekazaniem poleceń komend na ekranie. 

**5.4 Flatpak (Format planowany)**
Całkowicie izolowany system, w którym updater nie robi nic oprócz poinformowania o aktualizacji flatpaka (zakazuje podrzucania własnego `tar.gz`). Nie ma wsparcia mieszania kanałów.

## 12. Ochrona archiwów TAR (Linux "Zip Slip")

Dla formatu `tar.gz`, wypakowywacz musi stosować surowe reguły bezpieczeństwa w trakcie ekstrakcji:
- Bezwzględnie odrzucać ścieżki zawierające `..` lub nawiązujące do `/` (bezwzględne).
- Odrzucać wpisy usiłujące wrzucić plik poza wskazany katalog tymczasowy paczki (`tmp`).
- Odrzucać w ogólności wpisy FIFO, dowiązania symlink kierujące się na tyły systemu, sockety i specjalne pliki blokowe/urządzeń.
- Updater nie może automatycznie przywracać użytkownika `uid`, `gid` oraz 100% zrzutu permmisions binarnych widniejących w paczce u dystrybutora. Prawa muszą wkomponowywać się w system usera.
- Po operacji wypakowania weryfikuje się zrębę i odnajduje binarkę.

## 13. Uprawnienia, Sudo, Atomowość i Rollback

**Brak niejawnego Sudo na Linux:** Auto-updater nie pobiera i nie nasłuchuje hasła wpisywanego do root'a. Nie wykorzystuje go do zmiennych, i nie konkatenacji w shellu. Sytuacja zapisu do readonly partycji odrzuca automatyzm na starcie z informacją tekstową, ratując środowisko przed wyciekiem uprawnień.

**Atomowość (Stan maszyny stanu instalacji):**
Zmiana (podmiana) systemowa dla wygody operacyjnej (`rename`) dokonywana jest przeważnie tylko w promieniu systemu plików. Staging i proces aktualizacyjny winien wykorzystywać to samo urządzenie fizyczne, by usunięcie i przeniesienie wykonały się jako jedna instrukcja logiczna systemu (atomic mv).
Stany: `Downloaded`, `Verified`, `Staged`, `SwapPending`, `Swapped`, `LaunchPending`, `Healthy`, `RollbackRequired`, `RolledBack`, `Failed`.

**Potwierdzenie prawidłowego uruchomienia (Health Check):**
Samo spawnnięcie się powłoki (pid nowej aplikacji) z podmienionego folderu nie kończy zadania. SPLogbook (nowy proces) musi mieć zaimplementowaną komunikację sygnałową (np. specjalny argument na pierwszy start `--check-health-startup` lub mały plik IPC/socket). Jeśli odpalona aplikacja zawiedzie lub zamknie kodem zepsutym zanim przekaże pinga ratunkowego (stan `Healthy`), instalator/helper reanimuje oryginalną apkę operacją przywracania `Rollback`. Brak arbitralnych przerw na sleep'y bez pomiarów statystycznych ładowania SPLogbooka.

## 14. Wspólny Interfejs Instalatora Platformowego (PlatformInstaller)

Bez implementacji w kodzie Rust, koncepcja wymaga utworzenia wyizolowanej struktury odpowiedzialności.
Rdzeń bezpieczeństwa pcha zaufane informacje poprzez abstrakcję `PlatformInstaller`:

- `detect_installation_type()` -> Zwraca enum typu wdrożenia (np. `PortableTarGz`, `AppImage`, `WindowsZip`).
- `validate_installation_target()` -> Sprawdza prawa Read-Write foldera głównego programu.
- `stage_verified_artifact(Path)` -> Przyjmuje w 100% ufny artefakt i uwalnia do katalogu bocznego (rozpakowanie z obroną ZipSlip).
- `prepare_rollback()` -> Klonuje current_dir na boczny tor.
- `apply_update()` -> Atomowy Swap folderów.
- `launch_updated_application()` -> Wybudza podmieniony silnik do życia.
- `confirm_health()` -> Konkluzja i ping od nowego okna aplikacji z wynikiem boolean.
- `rollback()` -> Zamiana stanów i folderów do tyłu po usterce z czyszczeniem błędu dla użytkownika.
- `cleanup()` -> Likwidacja plików `.tar.gz` /. `tmp` jeśli wszystko działa (lub nic nie zadziałało po cofnięciu).

## 15. Zaktualizowana Kolejność Wdrożenia

Kolejność ścisła (jedno zadanie to jedna sesja dla chatbota i jeden wydzielony Pull Request z commitem).
Nie zaczynamy implementacji bez stworzenia dokumentów!

*Faza projektowa systemu:*
- **3.0.A** Opracowanie dokumentu `DESIGN_3_0_A_WINDOWS_UPDATE.md` (szczegółowy proces Zip).
- **3.0.B** Opracowanie dokumentu `DESIGN_3_0_B_LINUX_UPDATE.md` (szczegółowe tar.gz z Zip Slip).
- **3.0.C** Opracowanie dokumentu `DESIGN_3_0_C_PLATFORM_INSTALLERS.md` (stanówka, rollback, powiązanie).

*Faza wdrożeniowa logiki (Instalacja bez kryptografii):*
- **3.0.D** Implementacja instalatora Windows (zgodnie z 3.0.A).
- **3.0.E** Implementacja instalatora Linux **tylko** dla wspieranego formatu (Portable Tar.gz z ochroną).
- **3.0.F** Wdrożenie mechanizmów z testami dla powrotów awaryjnych `rollback` oraz sygnałów okienkowych `health confirmation` na Windows.
- **3.0.G** Wdrożenie testów `rollback` i `health` dla Linux.

*Faza Bezpieczeństwa (Crypto i Core):*
- **3.1.A** Wygenerowanie `REPORT_3_1_A_GITHUB_DIGEST.md` jako praca analityczna braku wartości digest API z zakazem kodowania poprawek w tej sesji (jeśli wada = wymóg podzadania).
- **3.1.B** Finalizacja wewnętrzna struktur Manifestu zawierających wymóg na string: `package_type`.
- **3.1.C** Izolowany weryfikator `Ed25519` w warstwie logicznej na tablicy u8.
- **3.1.D** Zbudowanie ścisłej, liniowej, odpornej 12-krokowej walidacji parsera (z zachowaniem tablic Key).
- **3.1.E** Utworzenie dokładnego selektora sprawdzającego unikalność triad z JSONa (platform+arch+package_type).
- **3.1.F** Ochrona downgrade'owa pod parser biblioteki SemVer i zablokowanie równego numerowania.
- **3.1.G** Integracja całości - Wspólny rdzeń uderza nowym pobieraniem (Stream+max_size) z podaniem zaufanego pakietu do wcześniej powstałych instalatorów.
- **3.1.H** Osobne CLI narzędzie offline (generator wydań `SIG`) na kluczu prywatnym.
- **3.1.I** Integracja logiki z Workflow GitHub Actions (Protected Env. przy deployu JSONa z plikami).
- **3.1.J** Izolowane wydanie testowe Windows i Linux do piaskownicy.
- **3.1.K** Testy podmiany - weryfikacja czy jeden ukradziony bajt paraliżuje całą rurę operacji.
- **3.1.L** Pierwsze publiczne wydanie v1.2.0.

## 16. Plan Testów Linux

Podział dla zabezpieczenia operacyjności wersji portable:
- **Jednostkowe:**
  - Odrzucenie paczek Linux bez pasującego `package_type` w parserze JSON.
  - Skrypty filtrujące złośliwe i dziwne ścieżki w archiwach paczek `tar`.
- **Integracyjne:**
  - Sprawdzenie logiki błędnego dopasowania rur streamingowych dla SHA z zerwaniem ułamka rozmiaru wielkości przy sztucznym opóźnieniu i ucięciu gniazda HTTP.
- **Platformowe:**
  - `Portable tar.gz`: Rozpakowywanie i przenoszenie binarki do docelowej przestrzeni montażowej na ext4 z ominięciem nadpisu dla `bazy_danych.db`.
- **Manualne:**
  - Sprawdzenie Rollbacku na żywym organizmie wyciągając prądy po aktualizacji w katalogu zapisywalnym.
  - Sprawdzenie zachowania dla uruchomienia SPLogbook spod `read-only` gdzie uprawnienia dla `/opt` i polecenia blokują dostęp systemowy.
- **Przyszłe Kryteria (CI/CD):**
  - Ochrona rozpakowania Flatpak lub DEB/AppImage (Nieaktywne testy dopóki nie zostaną wdrożone w workflow). Zgodność Aarch64 również po implementacji w CI.

## 17. Kryteria Akceptacji (Etap 3.1 & 3.0)
1. Pierwsza kompletna wersja wydana (np. v1.2.0). Błąd kryptograficzny, błąd klucza czy JSON'a twardo blokuje instalację danego wydania i nie może zostać pominięty przez użytkownika. Kolejna próba jest możliwa dopiero po opublikowaniu poprawnego wydania albo usunięciu problemu po stronie źródła.
2. Windows i Linux używają jednego, spójnego monolitycznego rdzenia bezpieczeństwa logiki Ed25519 z blokowaniem.
3. Instalatory platformowe **przyjmują wyłącznie** podany autoryzowany już i przefiltrowany artefakt od Rdzenia.
4. Manifest określa precyzyjny obowiązkowy string `package_type` podlegający walidacji (np. `"zip"`, `"tar.gz"`). Selekcja narzuca ujęcie kanału instalacyjnego wykrytego u użytkownika.
5. Updater pod żadnym pozorem nie miesza typów (np. próba rozpakowania `.tar.gz` przy instalacji w `AppImage`).
6. Lokalna konfiguracja klienta nie stanowi zaufanego decydenta co do logiki odblokowania usterki podpisu czy klucza głównego podanego jako `[u8; 32]`.
7. Rozmiar ostatecznie pobranego z dysku artefaktu z powrotem zgadza się **dokładnie** z wartością `asset.size`, przy jednoczesnym uwzględnieniu zakazu dla transferów ponad `MAX_UPDATE_SIZE`.
8. Każdy `filename` oraz triad systemowych paczek jest w dokumencie weryfikowana na czysto (bez zduplikowanych par pakietu). Manifest zawierający wariantowy bałagan (więcej niż jedno dopasowanie) blokuje się natychmiast, by rdzeń nie mógł pociągnąć na oślep pierwszej pasującej paczki o dobrym rozszerzeniu.
9. Wpis pole `commit` opublikowane w manifeście zawiera precyzyjne 40 znaków małego hexu.
10. Zakres dopuszczonej ważności na skojarzony publiczny klucz ujęto tuż za potwierdzeniem 100% autentyczności bajtów podpisu i całego bloku JSON z dekoderem manifestu Ed25519.
11. Archiwa instalacyjne Linuxowego środowiska (tar.gz) ulegają bezpiecznemu filtrowaniu i rozpakowaniu bez cienia uprawnień dla `SUDO`.
12. Aktualizacja (szczególnie folderze na Linuxie) nie nadpisuje logbooków baz operacyjnych i konfiguracji użytkownika z wnętrza paczek.
13. Nowy proces logbooka potrafi "potwierdzić swój zdrowy start" instalatorowi platformowemu, w przeciwnym razie odwraca całą transakcję w trybie szybkiego przywrócenia kopii.
14. Skrajny przypadek (na Unix) z brakiem praw zapisu wyrzuca kulturalną komendę aktualizacji ręcznej.
15. Testowe wydanie (3.1.J) ukryte za osobnym weryfikatorem omija zwykłych obserwatorów.
16. Pomyślnie utworzono analityczny `REPORT_3_1_A` nie pisząc linijki kodu w swoim obwodzie roboczym, jak i podziały dla DESIGNU Windowsa i Linuxa na oddzielne pliki specyfikacji (zgodnie z ułożoną listą).
17. Zadanie zostało zakończone testami podmiany pakietu.
