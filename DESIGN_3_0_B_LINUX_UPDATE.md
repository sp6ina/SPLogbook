# DESIGN_3_0_B_LINUX_UPDATE.md

## 1. Cel dokumentu
Dokument stanowi projekt koncepcyjny instalatora uaktualnień przenośnej wersji programu SPLogbook na systemy Unix/Linux, dla formatu bazowego `.tar.gz`. Został utworzony w ramach wykonania kroku 3.0.B na podstawie nadrzędnych specyfikacji `DESIGN_3_1_UPDATER_SIGNATURES.md`.

## 2. Diagnoza stanu początkowego
Do tej pory automatyczna funkcja używała metody `std::fs::rename` podmieniając fizycznie natywny plik wykonywalny ELF surową paczką archiwum `.tar.gz`, co uniemożliwiało ponowny rozruch aplikacji i wymagało uciążliwej, całkowicie manualnej ingerencji ze strony radioamatora. Krok awaryjny (M1 - Safety Gates) zablokował te operacje.

## 3. Wymagania i Architektura Instalatora Tar.Gz (Portable)

Na systemach linuksowych SPLogbook znajduje się najczęściej w postaci tzw. katalogu z binarką, bibliotekami i assetsami. Instalator nie będzie wykorzystywał poleceń powiązanych z prawami Root'a (`sudo`), aby nie narażać bezpieczeństwa. Działa bezwzględnie w modelu ograniczonych uprawnień (User Space).

### Krok 1: Weryfikacja Uprawnień (Target Validation)
Zanim proces pobierania zostanie w ogóle autoryzowany na warstwę GUI:
1. Skrypt instalatora wywołuje wbudowany moduł Rusta, by upewnić się, czy folder macierzysty z którego został podniesiony proces, ma w ogóle prawo R/W dla bieżącego usera (uid). 
2. Instalacje wykonane z roota (np. do zablokowanego `/opt/`) natychmiast kończą procedurę wyrzuceniem monitu w okienku: "Katalog aplikacji jest chroniony przed zapisem (np. użyto dpkg/apt). Auto-updater wstrzymany. Zaktualizuj program używając menedżera pakietów lub jako root".

### Krok 2: Ekstrakcja poza strefą działania (Staging / ZipSlip Protection)
1. Bezpieczny plik `.tar.gz` z rdzenia zabezpieczeń, wpada do lokalizacji obok działającej instalacji (np. `tmp_update/`).
2. Instalator uruchamia zaufaną bibliotekę `tar` (w połączeniu z `flate2`).
3. **Restrykcyjny filtr (Linux "Zip Slip" Guard):**
   - Ścieżki zawierające sekwencję `..` rzucają `Err`.
   - Ścieżki startujące od korzenia np. `/etc/` rzucają `Err`.
   - Ścieżki nawiązujące do zasobów IPC, device nodes (np. blokowych, character devices, fifo, unix sockets) ulegają całkowitemu zignorowaniu bez wypakowania.
   - Prawa dostępu (permissions mask) zawarte z archiwum zostają nienaruszone, chyba że godzą w ogólną własność użytkownika docelowego.

### Krok 3: Kopia Zapozasowa (Staging Backup / Rollback)
1. Instalator wykonuje kopię głównych komponentów aplikacji z użyciem twardych linków (jeśli wspierane na partycji) lub płytkiego klonowania plików (shallow copy) do ustronnego katalogu `.rollback`.
2. Omijaniu ulegają całkowicie pliki SQLite użytkownika - zabezpieczając cenne zbiory styków radiowych.

### Krok 4: Atomowa podmiana - Rename Swap
Na Uniksach, działający i "zmapowany" w pamięci (mapped executable) plik można odczepić (unlink) poprzez `rename`, co jest bezpieczniejsze od Windowsa.
1. Kod Rusta iteruje po rozpakowanej paczce z wnętrza katalogu tymczasowego.
2. Stosuje nadpisywanie starych plików poprzez `rename()` w jednoznacznym systemie plików.
3. Gdy główne binary się podmienią, instalator (z poziomu starego wciąż biegnącego kodu ładuje) polecenie w dół z użyciem `fork` & `exec` zdejmując bieżące okno aplikacji i wzbudzając nowe SPLogbook.exe (`--check-health-startup`).

### Krok 5: Health Confirmation & Ping
Nowy uruchomiony daemon logbooka, wykonuje start. Jeśli bazy wczytają się poprawnie i brak błędów krytycznych – proces upuszcza wyznaczony sygnał (np. wpis do pliku lub nazwanego potoku), a uśpiony w tle stary demon aktualizatora przed swoim całkowitym `exit()` potwierdza wchłonięcie aktualizacji z usunięciem `.rollback` ze śmieci. Jeżeli nowy program zakończy się "Segmentation fault" z powodu niekompatybilnych bibliotek `.so`, watchdog odpali przywrócenie.

## 4. Zadania Implementacyjne dla Linux
1. Odizolowanie pakietów dystrybucyjnych od natywnego `.tar.gz`.
2. Dołączenie zaufanej, bezpiecznej biblioteki `tar` dla instalatora bez wywołań bocznych polecenia CLI. 
3. Wdrożenie warunków `Unix-only` w architekturze chroniącej Zip-Slip.
4. Adaptacja flag i argumentów na wywołaniach `.spawn()`.
