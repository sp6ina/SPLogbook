> **ERRATA (2026-11-09):** Binding runtime policy is minimum **Linux 7.0** and **glibc 2.43**
> (`OWNER-M3-F-KERNEL-POLICY-OPTION-A`, `OWNER-M3-F-GLIBC-POLICY-01`). Sections describing the
> `glibc 2.35` build baseline apply only to future DEB/Flatpak packages and do not change the
> runtime minimum. Current snapshot: `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`.

# PROJEKT ARCHITEKTONICZNY: PAKIETY INSTALACYJNE I AKTUALIZATOR LINUX (PORTABLE TAR.GZ, APPIMAGE, DEB, RPM, FLATPAK)
# Dokument: DESIGN_3_0_B_LINUX_UPDATE.md

---

## 1. Status Dokumentu
- **Identyfikator:** `UPDATE-DESIGN-LINUX`
- **Etap:** `STAGE-M2-INSTALLER-DESIGN-GATE` / `STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE`
- **Status dokumentu:** `APPROVED` (Formalnie zatwierdzony po decyzjach Właściciela `OWNER-M2-04`, `OWNER-M2-05`, `OWNER-M2-06`, `OWNER-M2-07` w bramce zatwierdzenia decyzji)
- **Gotowość wykonawcza M3:** `READY_FOR_IMPLEMENTATION` (M3-BLOCKING DECISIONS: 0; prace implementacyjne M3 nierozpoczęte)
- **Data opracowania:** 2026-11-07 (Rzeczywista data wykonania bramki M2)
- **Strefa czasowa:** `+01:00` (CET / Central European Standard Time, UTC+01:00)
- **Autor / Rola:** Główny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Linux, AppImage, DEB, RPM, Flatpak, APT, DNF i Bezpiecznych Aktualizatorów

---

## 2. Wersja i Historia Zmian
- **Wersja:** `2.1.0`
- **Historia:**
  - `v1.0.0` (2026-09-29): Wczesny szkic koncepcyjny eliminujący niszczące zastępowanie binarki ELF surowym archiwum `.tar.gz` (historyczny commit pre-M1).
  - `v2.0.0` (2026-11-07): Kompletny projekt architektoniczny aktualizatora Linux Portable tar.gz, integrujący zweryfikowane mechanizmy manifest-first (Ed25519), bezpieczną dekompresję `extract_tar_gz_safe`, spłaszczanie pojedynczego katalogu nadrzędnego, silnik `UpdatePlan`, zachowanie bitu wykonywalnego `0o755`, hermetyzację bibliotek Hamlib i `RUNPATH`, atomowy mechanizm podmiany `rename()`, health check i rollback bez użycia uprawnień roota (`sudo`/`pkexec`) (`REVIEWED`).
  - `v2.1.0` (2026-11-07): Formalne zatwierdzenie rozszerzonego zakresu M3 przez Właściciela Projektu (`STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE`). Włączenie pakietów: Portable tar.gz, AppImage (`SPLogbook-Linux-x86_64.AppImage`), pakietów systemowych DEB (integracja z APT) i RPM (integracja z DNF) oraz Flatpak (`org.splogbook.SPLogbook`). Ustanowienie granicy menedżera pakietów (System Package Manager Boundary), bezwzględnego zakazu automatycznej eskalacji (`sudo`/`doas`/`pkexec`), formuły badania pojemności dysku, zakazu anulowania po wejściu w fazę `Applying` oraz pełnej macierzy testów natywnych Linux (`APPROVED`).

---

## 3. Baseline Git i Kontekst Techniczny
- **Baseline Git HEAD:** `7bb8a2ddeefb7b61c02318e82ce3a8a72216ba6b`
- **Gałąź referencyjna:** `main` / `repair/full-system-remediation`
- **Wersja oprogramowania:** SPLogbook `v1.2.0` (kod źródłowy Rust 2024, MSRV 1.85+)
- **Relacja do Etapu M1 i M2:** W Etapie M1 zweryfikowano pełne pokrycie 40/40 kryteriów `LINUX-M1`. Projekt bazowy instalatorów został zamknięty w bramce projektowej M2, a niniejsza wersja v2.1.0 formalizuje zatwierdzony rozszerzony zakres implementacji Etapu M3.

---

## 4. Zakres Projektu (In-Scope)
Projekt obejmuje pięć oficjalnych formatów dystrybucji Linux oraz precyzyjne reguły ich aktualizacji z podziałem na formaty transakcyjne (self-managed) oraz pakiety zarządzane systemowo (system-managed):
1. **Portable tar.gz (`SPLogbook-Linux-x86_64.tar.gz`):**
   - Oficjalne archiwum przenośne dla instalacji w przestrzeni użytkownika (`/home/*`).
   - Własny silnik transakcyjny SPLogbook: bezpieczna dekompresja do `.staging`, spłaszczanie katalogu nadrzędnego, walidacja `UpdatePlan`, zachowanie uprawnień `0o755`, hermetyzacja `libhamlib.so*` i relatywnego `RUNPATH`, atomowa podmiana `rename()`, backup w `.rollback`, weryfikacja zdrowia (`--check-health-startup` + `.health_ok`) i sprzątanie.
2. **Pakiet AppImage (`SPLogbook-Linux-x86_64.AppImage`):**
   - Samodzielny, uruchamialny obraz aplikacji zgodny ze specyfikacją AppImage Type 2.
   - Posiada osobny `package_type` (`linux-appimage`), niezależny asset i sumę SHA-256 w manifeście.
   - Własny mechanizm transakcyjnej podmiany obrazu: pobranie nowego pliku `.AppImage`, nadanie uprawnień wykonywalnych `0o755`, podmiana z zachowaniem kopii zapasowej poprzedniego obrazu, weryfikacja health check i rollback.
   - Ochrona danych użytkownika: dane i konfiguracja składowane są w standardowych ścieżkach XDG (`~/.local/share/SPLogbook`, `~/.config/SPLogbook`), całkowicie poza strukturą obrazu AppImage.
   - **AppImage nie może być traktowane jako zwykły tar.gz.**
3. **Pakiet DEB (`SPLogbook-Linux-x86_64.deb`):**
   - Oficjalny pakiet dla dystrybucji Debian, Ubuntu, Linux Mint i pochodnych.
   - Standardowa struktura `control.tar.xz` + `data.tar.xz`, metadane zależności bibliotecznych (`libc6`, `libgtk-3-0`, `libasound2`, itp.), skrypty `postinst`, `prerm`, `postrm`.
   - Instalacja binariów w `/usr/bin/splogbook`, bibliotek w `/usr/lib/splogbook`, ziaren baz w `/usr/share/splogbook/databases`.
   - Aktualizacja realizowana przez delegację do systemowego menedżera **APT** (`apt`, Software Center, PackageKit).
   - **Zakaz bezpośredniej podmiany plików systemowych przez SPLogbook.**
4. **Pakiet RPM (`SPLogbook-Linux-x86_64.rpm`):**
   - Oficjalny pakiet dla dystrybucji Fedora, RHEL, openSUSE, AlmaLinux i Rocky Linux.
   - Specyfikacja RPM (`.spec`), wersjonowanie SemVer, deklaracja zależności, maintainer scriptlets.
   - Instalacja systemowa w standardowych prefiksach.
   - Aktualizacja realizowana przez delegację do menedżera **DNF / Zypper**.
   - **Zakaz bezpośredniej podmiany plików RPM przez SPLogbook.**
5. **Pakiet Flatpak (`org.splogbook.SPLogbook`):**
   - Oficjalny sandboksowany format dystrybucji desktopowej.
   - Manifest `org.splogbook.SPLogbook.json`, runtime `org.freedesktop.Platform`, SDK `org.freedesktop.Sdk`.
   - Ściśle zdefiniowane uprawnienia: dostęp do portów USB/TTY (dla Native CAT i zewnętrznego Hamlib), sieć (dla synchronizacji LoTW/eQSL i klastrów DX), dostęp do dźwięku PulseAudio/PipeWire.
   - Rozróżnienie instalacji systemowej (`--system`) i użytkownika (`--user`).
   - Aktualizacja realizowana przez delegację do Flatpak CLI (`flatpak update org.splogbook.SPLogbook`) lub portalu aktualizacji GNOME/KDE.
   - **Zakaz bezpośredniej podmiany plików wewnątrz sandboxa `/app`.**
6. **Ustanowienie granicy menedżera pakietów (System Package Manager Boundary — `OWNER-M2-07`):**
   - Auto-updater SPLogbook wykrywa typ instalacji.
   - Dla formatów zarządzanych systemowo auto-updater:
     - sprawdza wersję w podpisanym manifeście wydań,
     - informuje użytkownika o dostępności aktualizacji,
     - wyświetla dedykowany monit z sugerowanym poleceniem systemowym lub uruchamia natywny interfejs menedżera pakietów (przez D-Bus / PackageKit / portal),
     - **kategorycznie zabrania używania poleceń `sudo`, `doas` i `pkexec` wewnątrz procesu SPLogbook.**
7. **Formuła preflightu dyskowego:**
   - Zastąpienie sztywnego 50 MB dynamiczną formułą uwzględniającą specyfikę systemu plików i narzut instalatorów.
8. **Polityka anulowania:**
   - Zezwolenie na anulowanie w fazach poprzedzających `Applying`; bezwzględna blokada po przejściu do fazy `Applying` (`OWNER-M2-13`).
9. **Kompleksowa macierz testów natywnych dla Linux CI runnera.**

---

## 5. Poza Zakresem Projektu (Out-of-Scope)
Projekt jawnie wyklucza:
1. Bezpośrednią podmianę plików w katalogach systemowych (`/usr/bin`, `/usr/lib`, `/opt`) należących do pakietów DEB, RPM lub Flatpak.
2. Jakiekolwiek mechanizmy eskalacji uprawnień wewnątrz kodu SPLogbook: wywoływanie `sudo`, `doas`, `pkexec`, używanie bitów SUID/SGID.
3. Obchodzenie solvera zależności menedżerów pakietów systemowych (APT, DNF).
4. Przekraczanie uprawnień przyznanych sandboxowi Flatpak (zakaz sandbox escape).
5. Modyfikację schematu manifestu wydań w niniejszej bramce (rozszerzenie schema nastąpi w M3).
6. Publikację repozytoriów PPA, DNF repos czy Flathub bez osobnej bramki wydawniczej i konfiguracji kluczy GPG.

---

## 6. Formaty Wydania Linux w Etapie M3
W ramach rozszerzonego zakresu M3 pipeline wydań Linux generuje pięć oficjalnych assetów:

| Asset | Format kontenera | Typ pakietu (`package_type`) | Mechanizm instalacji / aktualizacji |
|---|---|---|---|
| `SPLogbook-Linux-x86_64.tar.gz` | POSIX tar + gzip | `linux-portable-tar-gz` | Własna transakcja SPLogbook (atomowe `rename()`, staging, rollback) |
| `SPLogbook-Linux-x86_64.AppImage` | AppImage Type 2 (ELF + SquashFS) | `linux-appimage` | Samodzielna transakcja SPLogbook (podmiana obrazu, uprawnienia `0o755`, rollback) |
| `SPLogbook-Linux-x86_64.deb` | Debian Binary Package (ar) | `linux-deb` | Delegacja do systemowego menedżera APT (`apt install --only-upgrade`) |
| `SPLogbook-Linux-x86_64.rpm` | RPM v4 Package (cpio) | `linux-rpm` | Delegacja do systemowego menedżera DNF (`dnf upgrade`) |
| `org.splogbook.SPLogbook.flatpak` | Flatpak Bundle / Remote | `linux-flatpak` | Delegacja do mechanizmu Flatpak (`flatpak update`) / GNOME Software |

- **Wspólne cechy binarne:**
  - Kompilacja x86_64, dynamiczne linkowanie z bibliotekami systemowymi GLIBC (zgodność z bazowym Ubuntu 22.04 LTS).
  - Wersje portable i AppImage hermetyzują skompilowaną bibliotekę `libhamlib.so.4` w podkatalogu `lib/` z relatywnym `RUNPATH=$ORIGIN/lib`.
  - Pakiety DEB i RPM linkują do systemowych bibliotek dystrybucyjnych lub hermetyzowanych w `/usr/lib/splogbook`.

---

## 7. Rozróżnienie Typów Instalacji i Granica Menedżera Pakietów
Na systemach Linux kluczowe jest jednoznaczne rozpoznanie środowiska uruchomieniowego:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinuxInstallationType {
    /// Instalacja przenośna tar.gz w katalogu domowym użytkownika z pełnymi prawami zapisu
    /// (np. /home/user/SPLogbook, ~/.local/share/SPLogbook).
    LinuxPortableTarGz,
    /// Samodzielny obraz wykonywalny AppImage (wykrywany przez zmienną środowiskową $APPIMAGE).
    LinuxAppImage,
    /// Instalacja systemowa z pakietu DEB (wykrywana baza dpkg w /var/lib/dpkg/info/splogbook.list).
    LinuxDebInstalled,
    /// Instalacja systemowa z pakietu RPM (wykrywana w bazie rpm /var/lib/rpm).
    LinuxRpmInstalled,
    /// Instalacja w środowisku kontenerowym Flatpak (wykrywany plik /.flatpak-info).
    LinuxFlatpak,
    /// Ścieżka systemowa bez rozpoznanego pakietu lub system plików tylko do odczytu.
    LinuxUnknownInstallation,
}
```

### Reguły detekcji środowiska Linux:
1. **Weryfikacja środowiska Flatpak:**
   - Sprawdzenie obecności pliku `/.flatpak-info`.
   - Jeśli plik istnieje: klasyfikacja jako `LinuxFlatpak`.
2. **Weryfikacja środowiska AppImage:**
   - Sprawdzenie obecności zmiennej środowiskowej `std::env::var("APPIMAGE")`.
   - Jeśli zmienna wskazuje na poprawną ścieżkę pliku obrazu: klasyfikacja jako `LinuxAppImage`.
3. **Inspekcja bazy pakietów DEB:**
   - Weryfikacja, czy binarka programu znajduje się w `/usr/bin/splogbook` oraz czy plik `/var/lib/dpkg/info/splogbook.list` istnieje.
   - Jeśli warunki są spełnione: klasyfikacja jako `LinuxDebInstalled`.
4. **Inspekcja bazy pakietów RPM:**
   - Weryfikacja, czy binarka programu znajduje się w `/usr/bin/splogbook` i program jest zarejestrowany w bazie RPM (`rpm -qf /usr/bin/splogbook` lub odczyt bazy sqlite/bdb librpm).
   - Jeśli warunki są spełnione: klasyfikacja jako `LinuxRpmInstalled`.
5. **Weryfikacja instalacji przenośnej (Portable tar.gz):**
   - Jeśli program nie jest Flatpakiem, AppImage ani pakietem DEB/RPM, a katalog bazowy znajduje się w katalogu domowym użytkownika (`$HOME`) i posiada zweryfikowane uprawnienia zapisu bez roota: klasyfikacja jako `LinuxPortableTarGz`.
6. **Inne lokalizacje / brak uprawnień:**
   - Instalacje w `/usr/local/bin`, `/opt` bez praw zapisu lub zablokowane: klasyfikacja jako `LinuxUnknownInstallation`.

### Granica menedżera pakietów (System Package Manager Boundary):
- **Dla `LinuxPortableTarGz` oraz `LinuxAppImage`:** Aplikacja realizuje transakcyjną aktualizację w przestrzeni użytkownika za pomocą własnego mechanizmu SPLogbook.
- **Dla `LinuxDebInstalled`:** SPLogbook informuje o dostępności nowej wersji z manifestu i wyświetla monit: *"Dostępna jest nowa wersja SPLogbook. Zaktualizuj system za pomocą: sudo apt update && sudo apt install --only-upgrade splogbook lub użyj menedżera oprogramowania."* SPLogbook nie modyfikuje żadnych plików w `/usr`.
- **Dla `LinuxRpmInstalled`:** SPLogbook informuje o aktualizacji z monitem: *"Dostępna jest nowa wersja SPLogbook. Zaktualizuj system za pomocą: sudo dnf upgrade splogbook."*
- **Dla `LinuxFlatpak`:** SPLogbook deleguje wywołanie do Portalu Aktualizacji XDG (`org.freedesktop.portal.Flatpak`) lub informuje o poleceniu `flatpak update org.splogbook.SPLogbook`.
- **Zasada nadrzędna:** W kodzie SPLogbook **nie ma i nie będzie** żadnych wywołań `sudo`, `doas` ani `pkexec`.

---

## 8. Architektura Aktualizacji Pakietu AppImage
AppImage wymaga dedykowanej procedury aktualizacji ze względu na swoją strukturę:
1. **Identyfikacja obrazu:**
   - Ścieżka aktywnego obrazu pobierana jest ze zmiennej `$APPIMAGE`.
   - Ścieżka katalogu zawierającego obraz: `appimage_dir = active_appimage.parent()`.
2. **Preflight uprawnień i miejsca:**
   - Weryfikacja praw zapisu w `appimage_dir`.
   - Badanie wolnego miejsca na partycji obrazu (wymagany rozmiar: $2 	imes 	ext{Rozmiar nowego AppImage} + 	ext{FilesystemSafetyMargin}$).
3. **Pobieranie i weryfikacja:**
   - Pobranie assetu `SPLogbook-Linux-x86_64.AppImage` do pliku tymczasowego `.SPLogbook_candidate.AppImage` w tym samym katalogu (na tym samym systemie plików).
   - Strumieniowa i finalna weryfikacja sumy SHA-256 względem podpisanego manifestu Ed25519.
4. **Nadanie uprawnień wykonywalnych:**
   - Wywołanie `std::fs::set_permissions(path, Permissions::from_mode(0o755))` na pobranym pliku kandydata.
5. **Transakcyjna podmiana (Atomic Rename):**
   - Utworzenie kopii zapasowej poprzedniego obrazu: `rename(active_appimage, active_appimage.with_extension("rollback"))`.
   - Podmiana na nowego kandydata: `rename(".SPLogbook_candidate.AppImage", active_appimage)`.
6. **Weryfikacja zdrowia i Rollback:**
   - Uruchomienie nowego obrazu kandydata z flagą `--check-health-startup`.
   - Jeśli proces potwierdzi zdrowie (`.health_ok` w ciągu 15s): usunięcie pliku `.rollback`.
   - Jeśli proces ulegnie awarii (segfault/timeout): przywrócenie poprzedniego obrazu z `.rollback` przez atomowe `rename()`.
7. **Ochrona danych:** Dane użytkownika i logi łączności (`default_log.db`, `station_config.json`) składowane są w `~/.local/share/SPLogbook` i nie podlegają modyfikacji podczas aktualizacji obrazu.

---

## 9. Architektura Pakietu DEB (Debian / Ubuntu)
1. **Metadane pakietu (`debian/control`):**
   - `Package: splogbook`
   - `Architecture: amd64`
   - `Depends: libc6 (>= 2.35), libgtk-3-0 (>= 3.24), libasound2 (>= 1.2), libgl1-mesa-glx, libusb-1.0-0, libudev1`
   - `Recommends: hamlib`
   - `Section: hamradio`
   - `Priority: optional`
2. **Układ plików w pakiecie:**
   - `/usr/bin/splogbook` (główny plik wykonywalny ELF)
   - `/usr/lib/splogbook/libhamlib.so.4` (hermetyzowana biblioteka)
   - `/usr/share/splogbook/databases/serviceLOG.db` (ziarno referencyjne)
   - `/usr/share/splogbook/databases/callbook.db` (ziarno callbooka)
   - `/usr/share/applications/splogbook.desktop` (plik integracji desktopowej)
   - `/usr/share/icons/hicolor/scalable/apps/splogbook.svg` (ikona)
3. **Maintainer Scripts:**
   - `postinst`: rejestracja ikon i menu desktopowego (`update-desktop-database`, `gtk-update-icon-cache`). Nadanie odpowiednich grup uprawnień (np. informacja o konieczności dodania użytkownika do grupy `dialout` w celu dostępu do portu szeregowego).
   - `prerm`: brak destrukcyjnych akcji na danych użytkownika.
   - `postrm`: przy `purge` usunięcie plików z `/etc/splogbook`, ale **nigdy z `/home/*`**.

---

## 10. Architektura Pakietu RPM (Fedora / RHEL)
1. **Specyfikacja RPM (`splogbook.spec`):**
   - `Name: splogbook`
   - `Version: 1.2.0`
   - `Release: 1%{?dist}`
   - `Summary: Modern Amateur Radio Logbook for Linux`
   - `License: MIT OR Apache-2.0`
   - `Requires: gtk3, alsa-lib, mesa-libGL, libusb1, systemd-libs`
2. **Instalacja i makra:**
   - Standardowe makra `%make_install`, `%files`, `%defattr(-,root,root,-)`.
   - Ochrona baz użytkownika: program uruchamiany przez użytkownika kopiuje brakujące ziarna baz z `/usr/share/splogbook/databases` do `~/.local/share/SPLogbook/databases` przy pierwszym uruchomieniu (`FirstInstallSeed`).

---

## 11. Architektura Pakietu Flatpak
1. **Identyfikator aplikacji:** `org.splogbook.SPLogbook`.
2. **Środowisko uruchomieniowe (Runtime):**
   - Runtime: `org.freedesktop.Platform` (wersja 24.08).
   - SDK: `org.freedesktop.Sdk` (wersja 24.08).
3. **Uprawnienia sandboxa (`finish-args`):**
   - `--share=ipc`, `--socket=x11`, `--socket=wayland`, `--socket=pulseaudio`.
   - `--share=network` (dla LoTW, eQSL, HamQTH, QRZ, klastrów DX).
   - `--device=all` (lub `--filesystem=/dev`) dla dostępu do fizycznych portów USB/UART (np. `/dev/ttyUSB0`, `/dev/ttyACM0`) w sterowaniu radiostacją Native CAT i Hamlib.
   - `--filesystem=xdg-documents:ro` (opcjonalny dostęp do eksportu/importu ADIF).
4. **Katalogi danych sandboxa:**
   - Dane aplikacji i bazy SQLite: `~/.var/app/org.splogbook.SPLogbook/data/SPLogbook/`.
   - Konfiguracja: `~/.var/app/org.splogbook.SPLogbook/config/SPLogbook/`.
5. **Mechanizm aktualizacji:**
   - Aktualizacje dostarczane są w standardowym potoku Flatpak.
   - Auto-updater wewnątrz sandboxa wykrywa aktualizację z manifestu i wywołuje akcję przez Portal Aktualizacji XDG (`org.freedesktop.portal.Updates`), nie próbując modyfikować systemu plików `/app`.

---

## 12. Bezpieczna Ekstrakcja tar.gz dla Portable
Dekompresja archiwum tar.gz realizowana jest przez `extract_tar_gz_safe`:
1. **Inspekcja limitów archiwum:**
   - Maksymalna liczba wpisów: 10 000 (`ArchiveLimits::MAX_ENTRIES`).
   - Maksymalny łączny rozmiar: 250 MB (`ArchiveLimits::MAX_TOTAL_BYTES`).
   - Maksymalny rozmiar pliku: 150 MB (`ArchiveLimits::MAX_ENTRY_BYTES`).
2. **Ochrona przed Tar-Traversal i Zip-Slip:**
   - Weryfikacja `validate_archive_relative_path`: odrzucenie ścieżek z `..`, ścieżek bezwzględnych (`/etc/passwd`), pustych komponentów.
3. **Pliki specjalne i dowiązania:**
   - Bezwzględny zakaz plików FIFO (named pipes), urządzeń blokowych i znakowych (`mknod`), gniazd UNIX.
   - Dowiązania symboliczne dozwolone **wyłącznie** w podkatalogu `lib/` dla bibliotek współdzielonych (np. `libhamlib.so -> libhamlib.so.4`), o ile cel dowiązania pozostaje wewnątrz stagingu (`ArchiveLimits::allow_symlinks = true` z rygorystyczną kontrolą celu).

---

## 13. Spłaszczanie Katalogu Nadrzędnego (Root Flattening)
Oficjalne archiwum Linux tworzone przez potok wydań zawiera katalog nadrzędny `SPLogbook-Linux-x86_64/`.
- Funkcja `flatten_single_root_directory` po rozpakowaniu przesuwa wszystkie pliki i podkatalogi (`binarka`, `lib/`, `databases/`) bezpośrednio do korzenia katalogu `.staging`.
- Funkcja `validate_staging_layout` weryfikuje obecność pliku wykonywalnego `splogbook` w korzeniu stagingu.

---

## 14. Układ Stagingu i Uprawnienia POSIX
W środowiskach Unix/Linux uprawnienia plików są kluczowym elementem bezpieczeństwa:
1. Plik wykonywalny `splogbook` oraz skrypty pomocnicze otrzymują uprawnienia `0o755` (`rwxr-xr-x`).
2. Pliki danych, biblioteki i bazy otrzymują uprawnienia `0o644` (`rw-r--r--`).
3. Uprawnienia są jawnie aplikowane przez `std::fs::set_permissions` z użyciem `PermissionsExt::from_mode`.

---

## 15. Budowanie i Walidacja Planu Aktualizacji (UpdatePlan on Linux)
Plan aktualizacji jest generowany w sposób czysty przez `UpdatePlan::generate`:
- `replace_files`: pliki programu `splogbook`, biblioteki `lib/libhamlib.so*`.
- `create_files`: nowe pliki programu lub ziarna baz, jeśli nie istnieją w aktywnej instalacji.
- `preserve_files`: istniejące bazy danych (`default_log.db`, `serviceLOG.db`, `callbook.db`), pliki konfiguracyjne (`station_config.json`), logi i pliki ADIF.
- `rejected_files`: nierozpoznane pliki w archiwum (fail-closed).

---

## 16. Zarządzanie Bibliotekami Współdzielonymi i RUNPATH Linkera ELF
1. SPLogbook na Linuksie hermetyzuje biblioteki Hamlib w folderze `$baseDir/lib/`.
2. Plik binarny kompilowany jest z relatywną ścieżką przeszukiwania linkera: `RUNPATH = "$ORIGIN/lib"`.
3. Podczas aktualizacji cały katalog `lib/` jest podmieniany atomowo w ramach planu aktualizacji, co zapobiega rozspójnieniu wersji binarki i bibliotek Hamlib.

---

## 17. Atomowa Podmiana Plików (Apply Phase & Atomic Rename)
W systemach POSIX podmiana plików wykonywalnych może odbywać się bez restartu do helpera dzięki właściwościom wywołania `rename(2)`:
1. Działający proces SPLogbook może podmienić swój własny plik binarny na systemie plików: stary i-węzeł pozostaje otwarty w pamięci jądra, podczas gdy ścieżka w katalogu wskazuje na nowy i-węzeł.
2. Przed podmianą tworzona jest kopia zapasowa w `.rollback`.
3. Podmiana każdego pliku odbywa się za pomocą atomowego `std::fs::rename`. Jeśli staging znajduje się na tym samym systemie plików, operacja jest natychmiastowa i odporna na awarie zasilania.

---

## 18. Uruchomienie Kandydata i Test Zdrowia
1. Nowa binarka uruchamiana jest jako proces potomny z flagą `--check-health-startup`.
2. Usunięcie starego pliku `.health_ok`.
3. Monitorowanie procesu kandydata przez maksymalnie 15 sekund.
4. Jeśli kandydat zgłosi błąd, ulegnie awarii (SIGSEGV, SIGABRT) lub przekroczy timeout 15s — natychmiastowe uruchomienie procedury Rollbacku.

---

## 19. Deterministyczny Rollback na Linuksie
W przypadku niepowodzenia testu zdrowia:
1. Zabicie wiszącego procesu kandydata (`SIGKILL`).
2. Usunięcie nowo utworzonych plików programu.
3. Przywrócenie starych plików z katalogu `.rollback` za pomocą atomowych wywołań `rename()`.
4. Uprawnienia `0o755` są weryfikowane i przywracane.
5. Poprzednia wersja binarki powraca na swoje miejsce; dane użytkownika pozostają w 100% nienaruszone.

---

## 20. Badanie Pojemności Dysku (Disk Space Preflight on Linux)
Arbitralny margines 50 MB został kategorycznie zastąpiony ścisłą formułą preflightu dyskowego:

$$	ext{RequiredSpace} = 	ext{DeclaredDownloadSize} + 	ext{MaxExtractedSize} + 	ext{BackupSize} + 	ext{PackageManagerOverhead} + 	ext{TransactionMetadataSize} + 	ext{FilesystemSafetyMargin}$$

### Parametry dla środowiska Linux:
- `DeclaredDownloadSize`: rozmiar zdeklarowany w manifeście (`size`).
- `MaxExtractedSize`: maksymalny rozmiar po rozpakowaniu (dla tar.gz / AppImage).
- `BackupSize`: rozmiar plików z `replace_files` podlegających kopii zapasowej.
- `PackageManagerOverhead`: dla APT / DNF — narzut bufora pakietów w `/var/cache/apt` lub `/var/cache/dnf` szacowany na $2 	imes 	ext{Rozmiar pakietu}$. Dla Flatpak — bufor pobierania ostree. Dla tar.gz i AppImage — 0.
- `TransactionMetadataSize`: narzut metadanych (~1 MB).
- `FilesystemSafetyMargin`: konfigurowalny bufor bezpieczeństwa systemu plików (ext4, btrfs, xfs):
  - Wartość minimalna: **25 MB**.
  - Wartość maksymalna: **100 MB**.
  - Wartość domyślna: **32 MB**.

---

## 21. Polityka Anulowania Transakcji (`OWNER-M2-13`)
- **Faza bezpieczna (Pre-Apply):** Anulowanie jest w pełni dozwolone w stanach od sprawdzania wydania do oczekiwania na akceptację (`AwaitingApply`). Wszystkie pliki tymczasowe są usuwane, a stan aplikacji pozostaje nienaruszony.
- **Faza krytyczna (Post-Apply / `Applying`):** Po wejściu w fazę podmiany plików (`Applying`), **anulowanie jest bezwzględnie zablokowane**. Żądania użytkownika w GUI są blokowane, a w razie błędu I/O transakcja przechodzi do procedury automatycznego rollbacku.

---

## 22. Macierz Zagrożeń Bezpieczeństwa Linux (Security Matrix)

| ID Zagrożenia | Wektor ataku / Błąd | Środowisko | Mechanizm mitygacji w projekcie | Ryzyko szczątkowe |
|---|---|---|---|---|
| **THR-LNX-01** | Tar-Traversal / Zip-Slip (`../`) | Ekstrakcja tar | `validate_archive_relative_path` + canonicalize check | Brak (fail-closed) |
| **THR-LNX-02** | Pliki specjalne (FIFO, mknod, dev) | Ekstrakcja tar | `ArchiveLimits::allow_special_files = false` | Wyeliminowane |
| **THR-LNX-03** | Złośliwe symlinki poza staging | Ekstrakcja tar | Kontrola celów symlinków, dopuszczenie wyłącznie w `lib/` | Wyeliminowane |
| **THR-LNX-04** | Eskalacja uprawnień przez `sudo` | Aktualizacja | Bezwzględny zakaz wywoływania `sudo`/`pkexec`; praca w user-space | Wyeliminowane |
| **THR-LNX-05** | Utrata bitu wykonywalnego binarki | Ekstrakcja tar | Jawne `set_permissions(0o755)` po ekstrakcji i po rollbacku | Wyeliminowane |
| **THR-LNX-06** | Uszkodzenie bibliotek Hamlib i RUNPATH | Podmiana | Atomowa podmiana całego katalogu `lib/`, stały `RUNPATH=$ORIGIN/lib` | Wyeliminowane |
| **THR-LNX-07** | Podmiana plików DEB/RPM przez auto-updater | Pakiety systemowe | Delegacja do APT/DNF; zakaz bezpośredniej podmiany w `/usr` | Wyeliminowane |
| **THR-LNX-08** | Złośliwy maintainer script DEB/RPM | Pakiety systemowe | Pakiety budowane wyłącznie w bezpiecznym CI z audytem skryptów | Wyeliminowane |
| **THR-LNX-09** | Nadmierne uprawnienia sandboxa Flatpak | Flatpak | Ściśle minimalne permissions w manifeście; brak dostępu do roota | Wyeliminowane |
| **THR-LNX-10** | Podmiana obrazu AppImage na niezweryfikowany | AppImage | Wymóg `VerifiedReleaseAsset` z Ed25519 i SHA-256 przed podmianą | Wyeliminowane |
| **THR-LNX-11** | Usunięcie danych w `/home` przy deinstalacji | Deinstalacja | Maintainer scripts wykluczają usuwanie danych użytkownika | Wyeliminowane |

---

## 23. Macierz Testów Implementacyjnych M3 dla Linux (Linux Test Matrix)

| ID Testu | Zakres testu | Fixture / Warunki wejściowe | Oczekiwany rezultat | Poziom testu |
|---|---|---|---|---|
| **TEST-LNX-01** | Bezpieczna ekstrakcja tar.gz | Archiwum tar z wpisem `../../etc/shadow` | `Err(UpdaterError::ZipSlip)` | Unit (Rust) |
| **TEST-LNX-02** | Blokada plików FIFO i urządzeń | Archiwum tar z wpisem `dev/zero` lub FIFO | `Err(UpdaterError::UnsafeArchivePath)` | Unit (Rust) |
| **TEST-LNX-03** | Spłaszczanie folderu nadrzędnego | Archiwum z `SPLogbook-Linux-x86_64/splogbook` | Binarka umieszczona w korzeniu `.staging` | Unit (Rust) |
| **TEST-LNX-04** | Uprawnienia POSIX `0o755` | Rozpakowana binarka `splogbook` | Weryfikacja `mode & 0o777 == 0o755` | Unit (Unix) |
| **TEST-LNX-05** | Atomowy rollback na Linuksie | Symulacja błędu health checku | Przywrócenie starej binarki z `.rollback` przez rename | Integration (OS) |
| **TEST-LNX-06** | Odmowa dla `/usr/bin` bez roota | Ścieżka instalacji w `/usr/bin` | Wykrycie `LinuxDebInstalled`/`LinuxRpmInstalled`, delegacja | Unit / Integration |
| **TEST-LNX-07** | Ochrona bibliotek Hamlib i RUNPATH | Test uruchomienia z lokalnym `lib/libhamlib.so` | Poprawne rozwiązanie symboli dynamicznych przez ld.so | Integration (OS) |
| **TEST-LNX-08** | AppImage — transakcyjna aktualizacja | Samodzielny obraz AppImage w `$HOME/bin` | Pobranie nowego `.AppImage`, uprawnienia 0o755, atomowa podmiana | Native CI (Linux) |
| **TEST-LNX-09** | AppImage — rollback przy awarii | Uszkodzony kandydat AppImage | Atomowe przywrócenie poprzedniego obrazu z `.rollback` | Native CI (Linux) |
| **TEST-LNX-10** | DEB — budowanie i weryfikacja pakietu | Źródła M3, narzędzie `dpkg-deb` | Poprawny pakiet DEB, bez błędów `lintian` | Native CI (Linux) |
| **TEST-LNX-11** | DEB — instalacja, upgrade i zachowanie `/home` | Środowisko testowe Ubuntu, `apt install` | Poprawna instalacja w `/usr`, dane w `/home` nienaruszone | Native CI (Linux) |
| **TEST-LNX-12** | RPM — budowanie i weryfikacja pakietu | Źródła M3, narzędzie `rpmbuild` | Poprawny pakiet RPM, bez błędów `rpmlint` | Native CI (Linux) |
| **TEST-LNX-13** | RPM — instalacja, upgrade i odinstalowanie | Środowisko testowe Fedora, `dnf install` | Poprawna instalacja, zachowane bazy użytkownika | Native CI (Linux) |
| **TEST-LNX-14** | Flatpak — budowa sandboxa z manifestu | Narzędzie `flatpak-builder` | Sukces kompilacji, poprawny App ID i uprawnienia | Native CI (Linux) |
| **TEST-LNX-15** | Flatpak — weryfikacja uprawnień i portów | Uruchomienie w sandboxie Flatpak | Dostęp do USB/TTY bez naruszania sandboxa | Native CI (Linux) |
| **TEST-LNX-16** | Zakaz anulowania po wejściu w Applying | Próba anulowania w trakcie `rename()` | Blokada w UI, dokończenie operacji lub rollback | Integration (OS) |
| **TEST-LNX-17** | Preflight dyskowy z dynamiczną formułą | Symulacja partycji z miejscem poniżej `RequiredSpace` | Wstrzymanie aktualizacji z komunikatem o braku miejsca | Unit (Rust) |

---

## 24. Rejestr Decyzji Właściciela dla Linux
W toku projektowania podjęto i formalnie zatwierdzono następujące decyzje leżące w gestii Właściciela Projektu:

| ID Decyzji | Treść decyzji | Zatwierdzony wariant | Status formalny | Blokuje M3? |
|---|---|---|---|---|
| **`OWNER-M2-04`** | Zakres formatów wydań Linux w M3 | **APPROVED — EXPANDED OPTION A** (Portable tar.gz, pakiety DEB i RPM) | **APPROVED** | **NIE (0)** |
| **`OWNER-M2-05`** | Wdrożenie pakietu AppImage w M3 | **APPROVED — APPIMAGE REQUIRED** (Dedykowany format obrazu `SPLogbook-Linux-x86_64.AppImage`) | **APPROVED** | **NIE (0)** |
| **`OWNER-M2-06`** | Integracja pakietów DEB, RPM i Flatpak z auto-updaterem | **APPROVED — MANAGED PACKAGE INTEGRATION** (Wykrywanie, monity, delegacja do APT/DNF/Flatpak) | **APPROVED** | **NIE (0)** |
| **`OWNER-M2-07`** | Granica menedżera pakietów i zakaz eskalacji | **APPROVED — SYSTEM PACKAGE MANAGER BOUNDARY** (Zakaz `sudo`/`doas`/`pkexec`; brak podmiany w `/usr`) | **APPROVED** | **NIE (0)** |

**Podsumowanie decyzji blokujących M3 dla Linux:**
- Liczba decyzji blokujących M3 przed zatwierdzeniem: **4** (`OWNER-M2-04`, `OWNER-M2-05`, `OWNER-M2-06`, `OWNER-M2-07`).
- Liczba decyzji blokujących M3 po zatwierdzeniu: **0**.

---

## 25. Kryteria Akceptacji Implementacji M3 (Linux Definition of Done)
Etap M3 dla platformy Linux zostanie uznany za zrealizowany wyłącznie po spełnieniu:
1. Wszystkie testy jednostkowe planera, bezpiecznej dekompresji tar.gz i zachowania uprawnień `0o755` przechodzą na Linux CI runnerze.
2. Zaimplementowano obsługę AppImage jako niezależnego formatu z transakcyjną podmianą pliku obrazu i rollbackiem.
3. Utworzono i przetestowano definicje budowania pakietów DEB i RPM z zachowaniem maintainer scripts nienaruszających katalogu domowego użytkownika.
4. Opracowano manifest Flatpak `org.splogbook.SPLogbook` ze ścisłymi uprawnieniami sprzętowymi USB/serial.
5. Auto-updater bezbłędnie wykrywa typ instalacji i dla instalacji zarządzanych deleguje operację do właściwego menedżera pakietów bez prób bezpośredniej podmiany w `/usr`.
6. Kod aktualizatora nie zawiera żadnych odwołań do poleceń `sudo`, `doas` ani `pkexec`.
7. Preflight dyskowy egzekwuje uzasadnioną formułę pojemności z buforem bezpieczeństwa.
8. Anulowanie po wejściu w fazę `Applying` jest programowo zablokowane.
9. Wszystkie testy z macierzy Linux (`TEST-LNX-01` do `TEST-LNX-17`) przechodzą pomyślnie.

---

## 26. Odrzucone Alternatywy
1. **Odrzucono: Wywoływanie `sudo` lub `pkexec` w auto-updaterze:** Narusza podstawowe zasady bezpieczeństwa aplikacji desktopowych; grozi eskalacją uprawnień i uszkodzeniem bazy uprawnień plików w katalogu domowym.
2. **Odrzucono: Bezpośrednie kopiowanie plików DEB/RPM do `/usr/bin`:** Powoduje ominięcie bazy danych menedżera pakietów dystrybucji (dpkg/rpm), uniemożliwia czyste usunięcie i rodzi konflikty przy aktualizacjach systemowych.
3. **Odrzucono: Traktowanie AppImage jako zwykłego archiwum tar.gz:** AppImage jest pojedynczym plikiem wykonywalnym i wymaga podmiany całego pliku obrazu, a nie rozpakowywania jego składników do katalogu.
4. **Odrzucono: Arbitralny margines dyskowy 50 MB:** Niewystarczający dla instalatorów systemowych z buforami pamięci podręcznej pakietów.

---

## 27. Ryzyka Szczątkowe (Residual Risks)
1. **Różnice biblioteczne GLIBC między dystrybucjami:** Kompilacja na nowszym systemie może uniemożliwić uruchomienie binarki na starszej dystrybucji.
   - *Mitygacja w M3:* Kompilacja pakietów w CI na bazowym obrazie Ubuntu 22.04 LTS (GLIBC 2.35), a dla Flatpak i AppImage — hermetyzacja wewnątrz standardowego runtime.
2. **Uprawnienia do portu szeregowego USB/UART (`dialout` / `uucp`):** Nowo zainstalowany pakiet może nie mieć dostępu do `/dev/ttyUSB0` bez dodania użytkownika do odpowiedniej grupy systemowej.
   - *Mitygacja:* Skrypt instalacyjny DEB/RPM oraz komunikat w aplikacji informują użytkownika o konieczności przypisania do grupy `dialout`.
