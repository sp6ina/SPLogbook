# DESIGN_3_0_A_WINDOWS_UPDATE.md

## 1. Cel dokumentu
Dokument stanowi formalny projekt mechanizmu instalatora aktualizacji dla środowiska Windows, gdzie aplikacja dystrybuowana jest jako archiwum przenośne `.zip`. Projekt ten realizuje wytyczne z `DESIGN_3_1_UPDATER_SIGNATURES.md` (zadanie 3.0.A) oddzielając etap weryfikacji kryptograficznej od samej procedury implementacji (rozpakowania paczki i podmiany binarki).

## 2. Diagnoza stanu początkowego
Do tej pory po pobraniu aktualizacji aktualizator zakładał naiwnie, że na systemie Windows plik `.zip` stanowi po prostu gotowy plik wykonywalny, w efekcie aplikując do niego operację `Move-Item -Force $nowy_zip $stary_exe`. Niszczyło to instalację. 

## 3. Wymagania i Architektura Instalatora Zip

### Krok 1: Wypakowanie poza obszarem operacyjnym (Staging)
1. Zaufany pakiet `.zip` (przekazany z mechanizmu bezpieczeństwa po sprawdzeniu SHA-256 i podpisu Ed25519) zostaje zapisany w bezpiecznym katalogu tymczasowym (np. w podkatalogu `.staging` obok programu głównego).
2. Proces Rusta uruchamia dekompresję (wykorzystując rygorystyczną bibliotekę, np. `zip`).
3. **Ochrona przed Zip-Slip**: Kod wykonujący wypakowanie absolutnie weryfikuje każdą podaną w archiwum ścieżkę. Ignoruje wpisy nawiązujące do rodzica (`../`) oraz ścieżki bezwzględne kierujące na zewnątrz katalogu izolowanego `.staging/extracted`.

### Krok 2: Przygotowanie Rollback'u
Zanim zostanie uruchomiony zewnętrzny proces wymuszający przerwę w działaniu, aplikacja samodzielnie (lub za pomocą dedykowanego helpera) wykonuje kopię bezpieczeństwa podmienianych plików. Katalog `.rollback` będzie przechowywał oryginały. Pliki baz danych SQLite (`*.db`) oraz logów użytkownika są całkowicie i celowo ignorowane w tym procesie z zastrzeżeniem, że instalator nigdy nie dokonuje operacji typu "Usuń cały stary katalog i wklej nowy". Operacja to tzw. "File merge" - nadpisywane są tylko i wyłącznie pliki wykonywalne Rusta oraz dodane ewentualne nowe aktywa z wypakowanego ZIPa.

### Krok 3: Atomowa podmiana - PowerShell
Plik `SPLogbook.exe` w systemie Windows posiada założoną blokadę plikową dopóki system operacyjny widzi uruchomiony z niego proces. Dlatego instalator Windowsa polegać będzie na ulepszonym skrypcie `updater.ps1`.
1. Skrypt czeka w pętli na wyłączenie PID głównego programu.
2. Wykonuje polecenie operujące na wypakowanych plikach: kopiuje zweryfikowaną treść struktury z `.staging/extracted/` w miejsce działania programu. 
3. Rozpoczyna nowy proces `SPLogbook.exe` przekazując z linii komend argument systemowy `--check-health-startup`.

### Krok 4: Potwierdzenie gotowości operacyjnej (Health Check)
Nowy proces (zaopatrzony w flagę `--check-health-startup`) w ciągu pierwszych kilku sekund podejmuje próbę:
- Deserializacji starego pliku konfiguracyjnego `config.json` w nowym układzie struktur z ewentualną migracją.
- Nawiązania połączenia (Open) do głównej bazy `sqlite`.
Jeżeli operacja powiedzie się (aplikacja renderuje klatkę eGUI i widzi poprawne pliki), loguje do watchera plik `.health_ok`. Watcher w PowerShell zamyka asystę uznając operację za udaną i czyści pliki `.staging`.

### Krok 5: Wycofanie po usterce (Rollback)
W wypadku, gdy po wykonaniu procesu Windowsowego nowa wersja ulegnie natychmiastowemu "wysypaniu" - okno zamknie się z kodem innym niż 0 lub minie 15-sekundowy timeout, w którym watcher nie odnotuje pliku `.health_ok`:
1. Watcher uaktywnia kod powrotu.
2. Odbudowuje pliki systemowe aplikacji z zapisanej kopii `.rollback`.
3. Ponownie uruchamia starą, bezpieczną i zweryfikowaną poprzednio wersję z informacją dla operatora (zostawiając ew. plik `error_log`).

## 4. Zadania wdrożeniowe (M3)
1. Dodanie niezbędnych zależności typu archiwum (np. `zip`) lub implementacja PowerShell oparta o `Expand-Archive`.
2. Implementacja ochrony ekstrakcji Zip-Slip w warstwie stagingu Rusta (aby nie rzucać bezpieczeństwa na barki powłoki systemowej).
3. Ewolucja skryptu ps1 wspierająca mechanizm `Rollback` połączona z modelem PID Watcher dla statusu wybudzenia.
4. Koniec naiwnego podmieniania pojedynczego pliku EXE plikiem zewnętrznym.
