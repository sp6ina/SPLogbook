# DESIGN_3_0_C_PLATFORM_INSTALLERS.md

## 1. Cel dokumentu
Dokument określa wspólny interfejs programistyczny (trait/abstrakcję) Rusta dla instalatorów platformowych. Realizuje krok 3.0.C opisany w nadrzędnym `DESIGN_3_1_UPDATER_SIGNATURES.md`, spinając różnice projektowe między systemem Windows (`DESIGN_3_0_A_WINDOWS_UPDATE.md`) i Linux (`DESIGN_3_0_B_LINUX_UPDATE.md`) we wspólny rdzeń zarządzania procesem aktualizacji.

## 2. Abstrahowanie ról rdzenia bezpieczeństwa a instalatora
Wspólny Rdzeń Bezpieczeństwa (Security Core) SPLogbook odpowiedzialny jest za uwierzytelnienie pobierania, weryfikację podpisu Ed25519 oraz pobranie pliku z limitem `MAX_UPDATE_SIZE` i weryfikacją `SHA-256`. Po tych operacjach dysponuje on lokalną ścieżką w 100% zaufanego pliku dystrybucyjnego (np. w katalogu `%TEMP%` lub `tmp/`). Od tego momentu przekazuje on pałeczkę instalatorowi, wywołując na nim poszczególne metody wspólnego interfejsu (w modelu State Machine). 

## 3. Wspólny Interfejs: `trait PlatformInstaller`

Mechanizm aktualizacji musi implementować lub logicznie realizować poniższe kroki na odpowiedniej dla systemu operacyjnego strukturze (np. `WindowsZipInstaller` lub `LinuxPortableInstaller`):

### `detect_installation_type() -> Result<InstallationType, Error>`
Metoda służy do upewnienia się, w jakiej formie zainstalowano program na systemie.
- Typy: `WindowsPortableZip`, `LinuxPortableTarGz`, `LinuxAppImage`, `ManagedPackage` (DEB/RPM/Flatpak - brak nadpisywania).
- Zwraca typ instalacji na podstawie heurystyk ścieżek uruchomienia oraz specyfiki OS.

### `validate_installation_target() -> Result<(), Error>`
- Weryfikacja praw dostępu Read/Write katalogu macierzystego aplikacji. Zabezpiecza np. na systemach Linuksowych przed wyrzuceniem awarii w połowie z braku podniesionych praw (co od razu przełącza system na instrukcję instalacji manualnej przez usera).

### `stage_verified_artifact(artifact: &Path) -> Result<PathBuf, Error>`
- Przyjmuje sprawdzony artefakt kryptograficzny (Zip lub Tar.gz).
- Tworzy katalog `.staging` (z odpowiednimi prawami dla lokalnego usera).
- Przeprowadza dekompresję (chronioną na obostrzeniach ZipSlip) do wyizolowanego katalogu, przygotowując pliki binarne nowej wersji do ostatecznego wdrożenia. Zwraca ścieżkę do rozpakowanych plików.

### `prepare_rollback() -> Result<PathBuf, Error>`
- Klonuje oryginalne pliki binarki i niezbędne asety do ustronnego katalogu np. `.rollback`.
- Metoda gwarantuje, że nie modyfikuje, ani nie zgarnia bezcennego folderu bazy danych (`databases/`) i logów użytkownika (`serviceLOG.db`). 

### `apply_update(staged_dir: &Path) -> Result<(), Error>`
- Skonkretyzowana realizacja właściwej operacji "Swap/Merge".
- **Na Linux**: wykonuje `std::fs::rename()` nadpisując na żywo rozpakowane pliki i odczepiając stare inode od drzewa.
- **Na Windows**: tworzy awaryjnego watchera w PowerShell (np. w `%TEMP%`), wywołuje go by zapadł w letarg oczekiwania, po czym zgłasza gotowość zamknięcia natywnego programu na głównym rdzeniu. Powłoka PowerShell zrealizuje przenosiny omijając błąd zablokowanego pliku `exe`.

### `launch_updated_application() -> Result<(), Error>`
- Formuje instrukcję budzącą z powrotem aplikację SPLogbook. Przekazuje nowemu wywołaniu flagę systemową `--check-health-startup`.
- Na Uniksach odbywa się to za pomocą operacji powłoki uruchamiającej niezależny detach (np. `Command::new().spawn()`). Na Windowsie proces ten jest wpisany bezpośrednio w końcówkę skryptu zapuszczonego w powłoce PowerShell.

### `confirm_health() -> Result<bool, Error>`
- Nowe uruchomione okno Rusta (odbierające flagę health_startup) próbuje przeczytać konfigurację i wczytać bazę bez błędów typu *Panic* oraz *Segfault*.
- Przy sukcesie, nowy proces emituje sygnał np. poprzez dedykowany plik `.health_ok`, a następnie natychmiast odpina flagę trybu diagnostycznego przechodząc do normalnej pracy logbooka.
- Stary watcher procesowy (w skrypcie/procesie) obserwuje obecność pliku na systemie - jego pojawienie się w określonym timeoucie definiuje powodzenie aktualizacji.

### `rollback() -> Result<(), Error>`
- W przypadku wyrzucenia usterki (braku sygnału w timeoucie 15 sekund lub awarii): watcher odwraca instalację kopiując z `.rollback` z powrotem pliki na katalog produkcyjny i zmuszając do odpalenia z powrotem stary proces. (Zabezpieczenie przed twardym wyłączeniem przez niekompatybilność DLL czy SO). Zapisuje log `update_error.log`.

### `cleanup() -> Result<(), Error>`
- Realizowana bezwarunkowo na zakończenie lub anulowanie (jeżeli update udany lub przywrócono stan). Likwiduje złoża z `.staging`, pliki autoryzowane (tar, zip), usypia lub kasuje stare struktury awaryjne z dysku pozostawiając porządek.

## 4. Architektura wspólna procesu
Interfejs musi pozwalać na odwrócenie odpowiedzialności. Kod Rusta obsługujący pobieranie i uwierzytelnianie (Crypto Core) jest nieświadomy wewnętrznych zjawisk systemu. Odpala jedynie metody: `validate() -> stage() -> prepare_rollback() -> apply_update() -> launch()`. 

Plik stanowi finalizację planu projektowego Etapu M2, przygotowując oprogramowanie do pisania czystego, wyizolowanego dla obu systemów kodu pod proces instalacji nowej generacji.
