# PROJEKT ARCHITEKTONICZNY: AKTUALIZATOR LINUX (PORTABLE TAR.GZ)
# Dokument: DESIGN_3_0_B_LINUX_UPDATE.md

---

## 1. Status Dokumentu
- **Identyfikator:** `UPDATE-DESIGN-LINUX`
- **Etap:** `STAGE-M2-INSTALLER-DESIGN-GATE`
- **Status dokumentu:** `REVIEWED` (Oczekuje na formalne zatwierdzenie otwartych decyzji wĹ‚aĹ›ciciela przez WĹ‚aĹ›ciciela repozytorium)
- **GotowoĹ›Ä‡ wykonawcza M3:** `BLOCKED_BY_OWNER_DECISIONS` (Sekcja 39 i 54)
- **Data opracowania:** 2026-11-07 (Rzeczywista data wykonania M2)
- **Strefa czasowa:** `+01:00` (CET / Europe/Warsaw)
- **Autor / Rola:** GĹ‚Ăłwny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Bezpiecznych AktualizatorĂłw

---

## 2. Wersja i Historia Zmian
- **Wersja:** `2.0.0`
- **Historia:**
  - `v1.0.0` (2026-09-29): Wczesny szkic koncepcyjny eliminujÄ…cy niszczÄ…ce zastÄ™powanie binarki ELF surowym archiwum `.tar.gz` (historyczny commit pre-M1).
  - `v2.0.0` (2026-11-07): Kompletny projekt architektoniczny aktualizatora Linux Portable tar.gz, integrujÄ…cy zweryfikowane mechanizmy manifest-first (Ed25519), bezpiecznÄ… dekompresjÄ™ `extract_tar_gz_safe`, spĹ‚aszczanie pojedynczego katalogu nadrzÄ™dnego, silnik `UpdatePlan`, zachowanie bitu wykonywalnego `0o755`, hermetyzacjÄ™ bibliotek Hamlib i `RUNPATH`, atomowy mechanizm podmiany `rename()`, health check i rollback bez uĹĽycia uprawnieĹ„ roota (`sudo`/`pkexec`).

---

## 3. Baseline Git i Kontekst Techniczny
- **Baseline Git HEAD:** Commit po synchronizacji projektu Windows (`77567f9957a08eaab7ff70205486572a8eba3495`).
- **GaĹ‚Ä…Ĺş referencyjna:** `main` / `repair/full-system-remediation`
- **Wersja oprogramowania:** SPLogbook `v1.2.0` (kod ĹşrĂłdĹ‚owy Rust 2024, MSRV 1.85+)
- **Relacja do Etapu M1:** W Etapie M1 zweryfikowano peĹ‚ne pokrycie 40/40 kryteriĂłw `LINUX-M1` (`STAGE_M1_COVERAGE_AND_STATUS_REPORT.md`). Niniejszy dokument stanowi specyfikacjÄ™ docelowej architektury wykonawczej dla M3.

---

## 4. Zakres Projektu (In-Scope)
Projekt obejmuje:
1. Oficjalnie wspierany model dystrybucji Linux: archiwum przenoĹ›ne (Portable) skompresowane w formacie tar.gz (`SPLogbook-Linux-x86_64.tar.gz`).
2. Rozpoznawanie typu instalacji: instalacja przenoĹ›na w katalogu domowym uĹĽytkownika vs instalacja w Ĺ›cieĹĽkach systemowych (`/usr/bin`, `/usr/local/bin`, `/opt`).
3. WeryfikacjÄ™ praw zapisu (Write Access Preflight) bez eskalacji uprawnieĹ„ (`sudo` = 0, `pkexec` = 0).
4. Kontrakt zaufania manifest-first (Ed25519) i selekcjÄ™ assetu na podstawie platformy, architektury i package type.
5. InspekcjÄ™ nagĹ‚ĂłwkĂłw tar przed i w trakcie dekompresji (kontrola Zip-Slip, odrzucanie Ĺ›cieĹĽek bezwzglÄ™dnych, kontrola symlinkĂłw, blokada FIFO i nodĂłw urzÄ…dzeĹ„).
6. BezpiecznÄ… dekompresjÄ™ do izolowanego stagingu (`.staging`).
7. SpĹ‚aszczanie pojedynczego katalogu nadrzÄ™dnego (`SPLogbook-Linux-x86_64/`).
8. Generowanie i walidacjÄ™ `UpdatePlan` z zachowaniem ochrony baz danych SQLite i konfiguracji.
9. Zachowanie atrybutĂłw uprawnieĹ„ POSIX (`0o755` dla binarki i skryptĂłw pomocniczych).
10. OchronÄ™ spĂłjnoĹ›ci bibliotek wspĂłĹ‚dzielonych (`lib/libhamlib.so*`) oraz relatywnego `RUNPATH`.
11. TransakcyjnÄ… procedurÄ™ podmiany plikĂłw za pomocÄ… atomowych wywoĹ‚aĹ„ `rename()` z bezpiecznÄ… kopiÄ… zapasowÄ… w `.rollback`.
12. Uruchomienie kandydata w trybie `--check-health-startup` oraz monitorowanie pliku `.health_ok`.
13. Automatyczny rollback w przypadku awarii procesu kandydata (crash/segfault) lub przekroczenia 15-sekundowego limitu czasu.
14. SprzÄ…tanie plikĂłw tymczasowych oraz procedury odzyskiwania po przerwaniu zasilania.
15. Macierz testĂłw akceptacyjnych dla runnerĂłw Linux CI.

---

## 5. Poza Zakresem Projektu (Out-of-Scope)
Projekt jawnie wyklucza:
1. Budowanie, dystrybucjÄ™ lub automatycznÄ… aktualizacjÄ™ formatĂłw pakietĂłw systemowych: DEB, RPM, Flatpak, Snap, AppImage.
2. Jakiekolwiek mechanizmy eskalacji uprawnieĹ„: `sudo`, `doas`, `pkexec`, SUID binaries.
3. ObsĹ‚ugÄ™ instalacji w systemowych katalogach chronionych (`/usr/bin`, `/bin`, `/opt`) w trybie auto-updatera (aplikacja w tym trybie odsyĹ‚a uĹĽytkownika do menedĹĽera pakietĂłw dystrybucji).
4. ModyfikacjÄ™ bibliotek systemowych GLIBC lub instalacjÄ™ zaleĹĽnoĹ›ci przez `apt`/`dnf`/`pacman`.
5. ModyfikacjÄ™ schematu manifestu wydaĹ„.

---

## 6. Obecny Format Wydania Linux
Zgodnie z potokiem `.github/workflows/build-and-release.yml`:
- **Nazwa assetu:** `SPLogbook-Linux-x86_64.tar.gz`
- **Format kontenera:** POSIX tar archiwizowany i skompresowany gzip (`.tar.gz`).
- **Platforma:** `linux` (`ReleasePlatform::Linux`).
- **Architektura:** `x86_64` (`ReleaseArchitecture::X86_64`).
- **Model archiwum:** `SINGLE TOP-LEVEL DIRECTORY` (wszystkie pliki znajdujÄ… siÄ™ wewnÄ…trz folderu `SPLogbook-Linux-x86_64/`).
- **GĹ‚Ăłwny plik wykonywalny:** `splogbook` (ELF 64-bit LSB pie executable, x86-64, dynamicznie linkowany).
- **Biblioteki doĹ‚Ä…czone:** Podkatalog `lib/` zawierajÄ…cy biblioteki Hamlib (`libhamlib.so.4`, dowiÄ…zania symboliczne `libhamlib.so`).
- **Konfiguracja linkera ELF:** WartoĹ›Ä‡ `RUNPATH` ustawiona relatywnie wzglÄ™dem binarki (`$ORIGIN/lib` lub `$ORIGIN/../lib`).
- **Ziarna baz danych:** Podkatalog `databases/` zawierajÄ…cy `serviceLOG.db` oraz `callbook.db`.

---

## 7. RozrĂłĹĽnienie TypĂłw Instalacji (Portable vs System-Managed)
Na systemach Unix/Linux kluczowe jest jednoznaczne rozrĂłĹĽnienie Ĺ›rodowiska uruchomieniowego:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxInstallationType {
    /// Instalacja przenoĹ›na w katalogu domowym uĹĽytkownika z prawami zapisu
    /// (np. /home/user/SPLogbook, ~/.local/share/SPLogbook).
    PortableUserWritable,
    /// Katalog instalacji na systemie plikĂłw zamontowanym w trybie tylko do odczytu (read-only filesystem).
    ReadOnlyFilesystem,
    /// Instalacja w standardowych Ĺ›cieĹĽkach systemowych (/usr/bin, /usr/local/bin, /opt).
    SystemManagedLocation,
    /// Wykrycie Ĺ›rodowiska kontenerowego lub sandboxa (Flatpak, Snap).
    SandboxManaged,
    /// Stan nieznany lub brak dostÄ™pu do metadanych Ĺ›cieĹĽki.
    Unknown,
}
```

### ReguĹ‚y detekcji i granica odpowiedzialnoĹ›ci:
1. **Wykrywanie Ĺ›cieĹĽek systemowych:** JeĹ›li Ĺ›cieĹĽka binarki `current_exe()` rozpoczyna siÄ™ od:
   - `/usr/bin/`
   - `/usr/local/bin/`
   - `/opt/`
   - `/bin/`
   klasyfikacja to `SystemManagedLocation`.
   - **Zachowanie:** Auto-updater **bezwzglÄ™dnie odmawia** samodzielnej prĂłby aktualizacji. W GUI wyĹ›wietlany jest monit: *"SPLogbook zostaĹ‚ zainstalowany w Ĺ›cieĹĽce systemowej (%s). Automatyczna aktualizacja jest wyĹ‚Ä…czona. UĹĽyj menedĹĽera pakietĂłw dystrybucji (np. apt, dnf) lub zaktualizuj aplikacjÄ™ jako administrator."*
2. **Wykrywanie praw zapisu (Write Probe):** JeĹ›li Ĺ›cieĹĽka znajduje siÄ™ w katalogu uĹĽytkownika, wykonywana jest weryfikacja metadanych `metadata.permissions().readonly()` oraz testowy zapis pliku `.update_probe_<uuid>.tmp`.
   - JeĹ›li zapis siÄ™ powiedzie: `PortableUserWritable`.
   - JeĹ›li wystÄ…pi bĹ‚Ä…d `PermissionDenied` lub `EROFS` (Read-only file system): zgĹ‚aszany jest bĹ‚Ä…d `UpdaterError::Io`, a proces aktualizacji zostaje przerwany przed pobraniem pakietu.
3. **Brak eskalacji:** Zgodnie z zasadÄ… `updater_does_not_use_sudo_guard`, kod SPLogbook nie zawiera i nigdy nie wywoĹ‚uje poleceĹ„ `sudo`, `doas` ani `pkexec`.

---

## 8. Kontrakt Zaufania (Linux Trust Flow)
PrzepĹ‚yw weryfikacji kryptograficznej na Linuksie jest identyczny jak na Windows i wymusza model `manifest-first`:
1. Pobranie pliku `release-manifest.json` oraz podpisu `release-manifest.json.sig`.
2. Weryfikacja podpisu Ed25519 wzglÄ™dem wbudowanych `TRUSTED_KEYS`.
3. Sprawdzenie statusu odwoĹ‚ania klucza (`REVOKED_KEY_IDS`). Klucz `splogbook-release-2026-01` jest trwale odrzucony (fail-closed).
4. Weryfikacja ochrony przed atakiem downgrade (`manifest.version` vs `current_version`).
5. Selekcja assetu speĹ‚niajÄ…cego wymagania: `platform == linux`, `arch == x86_64`, `package_type == tar.gz`.
6. Pobranie zweryfikowanego assetu do pliku tymczasowego `.SPLogbook_update_<pid>.tar.gz`.
7. Weryfikacja rozmiaru bajtowego oraz skrĂłtu SHA-256 pobranego pliku.

---

## 9. Selekcja Assetu Linux
Kryteria wyboru assetu Linux:
- `platform`: `ReleasePlatform::Linux` (`"linux"`).
- `arch`: `ReleaseArchitecture::X86_64` (`"x86_64"`).
- `package_type`: `ReleasePackageType::TarGz` (`"tar.gz"`).
- `filename`: musi koĹ„czyÄ‡ siÄ™ na `.tar.gz` i odpowiadaÄ‡ `SPLogbook-Linux-x86_64.tar.gz`.
- Liczba pasujÄ…cych assetĂłw w manifeĹ›cie: **dokĹ‚adnie 1**. Wszelkie odchylenia skutkujÄ… bĹ‚Ä™dem `UpdaterError::AmbiguousArtifact` lub `UpdaterError::NoMatchingAssets`.

---

## 10. Inspekcja Archiwum tar.gz (Tar Inspection)
Dekompresja archiwum tar.gz podlega Ĺ›cisĹ‚ej kontroli limitĂłw i wĹ‚aĹ›ciwoĹ›ci wpisĂłw:
- **Maksymalna liczba wpisĂłw:** 10 000 wpisĂłw (`ArchiveLimits::MAX_ENTRIES`).
- **Maksymalny Ĺ‚Ä…czny rozmiar po rozpakowaniu:** 250 MB (`ArchiveLimits::MAX_TOTAL_BYTES`).
- **Maksymalny rozmiar pojedynczego pliku:** 150 MB (`ArchiveLimits::MAX_ENTRY_BYTES`).
- **WspĂłĹ‚czynnik kompresji (Decompression Bomb):** Kontrola strumieniowa; przekroczenie limitu rozmiaru rozpakowanego natychmiast przerywa strumieĹ„ gzip z bĹ‚Ä™dem `ArchiveLimitExceeded`.
- **Weryfikacja typĂłw wpisĂłw tar (`tar::EntryType`):**
  - Dozwolone: `Regular`, `Continuous`, `Directory`, `Symlink` (pod Ĺ›cisĹ‚ym nadzorem).
  - **Zabronione i odrzucane:** `Link` (hardlink), `Fifo`, `Char`, `Block`. Wykrycie pliku specjalnego lub urzÄ…dzenia natychmiast przerywa operacjÄ™ z bĹ‚Ä™dem `UpdaterError::UnsafeArchivePath`.

---

## 11. Bezpieczna Ekstrakcja (Tar Extraction)
Dekompresja realizowana jest przez `extract_tar_gz_safe`:
1. **Ochrona przed Path Traversal:** ĹšcieĹĽka kaĹĽdego wpisu w archiwum jest walidowana przez `validate_archive_relative_path`.
   - Zakaz wystÄ™powania komponentĂłw `..` (rodzic).
   - Zakaz Ĺ›cieĹĽek zaczynajÄ…cych siÄ™ od `/` (Ĺ›cieĹĽki bezwzglÄ™dne).
   - Zakaz wystÄ™powania bajtu zerowego (`\0`) w nazwie Ĺ›cieĹĽki.
2. **Kanonikalizacja docelowa:** Po doĹ‚Ä…czeniu Ĺ›cieĹĽki do katalogu `.staging`, sprawdzane jest, czy docelowa lokalizacja mieĹ›ci siÄ™ wewnÄ…trz katalogu stagingu.
3. Wszystkie pliki rozpakowywane sÄ… do odizolowanego podkatalogu `.staging` obok programu.

---

## 12. Polityka DowiÄ…zaĹ„ Symbolicznych i PlikĂłw Specjalnych
- **DomyĹ›lna polityka:** DowiÄ…zania symboliczne do celĂłw poza katalogiem stagingu sÄ… traktowane jako wektor ataku (Symlink Escape) i bezwzglÄ™dnie blokowane.
- **WyjÄ…tek dla bibliotek Hamlib:** W archiwach Linux katalog `lib/` zawiera tradycyjne dowiÄ…zania biblioteczne (np. `libhamlib.so -> libhamlib.so.4.0.0`).
  - *Zasada dopuszczenia:* DowiÄ…zanie symboliczne jest akceptowane **wyĹ‚Ä…cznie wtedy**, gdy jego cel (target) jest Ĺ›cieĹĽkÄ… wzglÄ™dnÄ…, nie zawiera `..` i wskazuje na plik w tym samym podkatalogu `lib/`.
  - DowiÄ…zania bezwzglÄ™dne (np. `-> /lib/x86_64-linux-gnu/...`) lub dowiÄ…zania wskazujÄ…ce na zewnÄ…trz pakietu rzucajÄ… bĹ‚Ä…d `UpdaterError::UnsafeArchivePath`.

---

## 13. SpĹ‚aszczanie Katalogu NadrzÄ™dnego (Root Flattening)
Paczka `SPLogbook-Linux-x86_64.tar.gz` generowana przez GitHub Actions zawiera caĹ‚Ä… zawartoĹ›Ä‡ w pojedynczym katalogu korzenia `SPLogbook-Linux-x86_64/`.
- Funkcja `flatten_single_root_directory(&staging)` sprawdza, czy korzeĹ„ `.staging` zawiera dokĹ‚adnie jeden podkatalog i zero plikĂłw regularnych.
- JeĹ›li tak, zawartoĹ›Ä‡ tego katalogu jest bezpiecznie przesuwana bezpoĹ›rednio do korzenia `.staging`, a pusty katalog nadrzÄ™dny jest usuwany.
- Funkcja `validate_staging_layout` weryfikuje, czy w korzeniu `.staging` znajduje siÄ™ binarka `splogbook`.

---

## 14. UkĹ‚ad Stagingu i Uprawnienia POSIX
- WewnÄ…trz `.staging` sprawdzane sÄ… uprawnienia plikĂłw.
- Plik `splogbook` oraz ewentualne skrypty pomocnicze (np. `run.sh`) muszÄ… posiadaÄ‡ bit wykonywalny (`0o755`). JeĹ›li archiwum nie miaĹ‚o ustawionego bitu, funkcja `extract_tar_gz_safe` ustawia maskÄ™ `0o755` dla plikĂłw binarnych programu.
- Pliki danych i bazy SQLite otrzymujÄ… maskÄ™ `0o644` (z uwzglÄ™dnieniem umask uĹĽytkownika).

---

## 15. Budowanie i Walidacja Planu Aktualizacji (UpdatePlan on Linux)
1. `UpdatePlan::generate(&staging, base_dir)` przeszukuje deterministycznie drzewo rozpakowanego stagingu.
2. KaĹĽdy plik jest klasyfikowany przez `classify_update_path`.
3. Plan jest walidowany przez `plan.validate_safety("splogbook")`.
   - Walidacja upewnia siÄ™, ĹĽe `splogbook` znajduje siÄ™ w `replace_files` lub `create_files`.
   - Walidacja gwarantuje, ĹĽe ĹĽaden plik oznaczony jako `UserData` (`default_log.db`), `UserConfiguration` (`station_config.json`) ani istniejÄ…cy `FirstInstallSeed` (`serviceLOG.db`, `callbook.db`) nie trafiĹ‚ do `replace_files`.

---

## 16. ZarzÄ…dzanie Bibliotekami WspĂłĹ‚dzielonymi (Libraries & Hamlib)
W archiwum dystrybucyjnym Linux kluczowe biblioteki (m.in. Hamlib i backendy radiowe) umieszczone sÄ… w podkatalogu `lib/`.
- Wszystkie pliki `lib/*.so*` sÄ… klasyfikowane jako `ProgramFile` / `RuntimeLibrary`.
- Podczas aktualizacji caĹ‚y zestaw bibliotek w `lib/` jest wymieniany w sposĂłb spĂłjny z gĹ‚ĂłwnÄ… binarkÄ… `splogbook`, co zapobiega bĹ‚Ä™dom niedopasowania symboli ABI (`undefined symbol`).
- Stare wersje bibliotek trafiajÄ… do `.rollback/lib/`.

---

## 17. Ochrona RUNPATH Linkera ELF
- Binarka `splogbook` jest linkowana z flagÄ… `-Wl,-rpath,$ORIGIN/lib` lub `$ORIGIN/../lib`.
- Struktura katalogĂłw w `.staging` oraz po wdroĹĽeniu do `$baseDir` zachowuje dokĹ‚adnÄ… relatywnÄ… pozycjÄ™ pomiÄ™dzy plikiem `splogbook` a katalogiem `lib/`.
- Zabronione jest przesuwanie binarki do innego podfolderu bez zachowania relacji z bibliotekami.

---

## 18. Kopia Zapasowa Przed PodmianÄ… (Rollback Preparation)
1. Utworzenie katalogu `$baseDir/.rollback`.
2. Dla kaĹĽdego pliku `(rel, orig)` ze zbioru `plan.replace_files`:
   - Utworzenie odpowiedniego podkatalogu w `.rollback`.
   - Wykonanie kopii pliku `fs::copy(orig, &dest)`.
3. **Zasada nadrzÄ™dna:** Bazy danych uĹĽytkownika (`default_log.db`), pliki Ĺ‚Ä…cznoĹ›ci i konfiguracja **nie sÄ… kopiowane** do `.rollback`.

---

## 19. Zastosowanie Aktualizacji (Apply Phase & Atomic Rename)
Na systemach Unix dziaĹ‚ajÄ…cy proces zmapowany w pamiÄ™ci moĹĽe zostaÄ‡ bezpiecznie odlinkowany (unlinked) przez operacjÄ™ `rename()`.
1. **PÄ™tla podmiany plikĂłw (`replace_files`):**
   ```rust
   for (rel, orig) in &plan.replace_files {
       let src = staging_dir.join(rel);
       if let Some(p) = orig.parent() {
           let _ = fs::create_dir_all(p);
       }
       // PrĂłba atomowego rename() w obrÄ™bie tego samego filesystemu
       if fs::rename(&src, orig).is_err() {
           // Fallback na copy w przypadku rĂłĹĽnych punktĂłw montowania
           fs::copy(&src, orig)?;
       }
   }
   ```
2. **PÄ™tla tworzenia nowych plikĂłw (`create_files`):**
   ```rust
   for (rel, orig) in &plan.create_files {
       let src = staging_dir.join(rel);
       if let Some(p) = orig.parent() {
           let _ = fs::create_dir_all(p);
       }
       if fs::rename(&src, orig).is_err() {
           fs::copy(&src, orig)?;
       }
   }
   ```
3. Wymuszenie synchronizacji metadanych katalogu (`sync` / `fsync`).

---

## 20. Uruchomienie Kandydata i Test Zdrowia (Launch & Health Check)
1. UsuniÄ™cie starego pliku potwierdzenia: `let _ = fs::remove_file(&health_ok);`.
2. Uruchomienie zaktualizowanego procesu jako procesu potomnego:
   ```rust
   let mut child = std::process::Command::new(current)
       .arg("--check-health-startup")
       .spawn()?;
   ```
3. Oczekiwanie na utworzenie pliku `.health_ok` z limitem czasu 15 sekund:
   ```rust
   let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
   let mut success = false;
   while std::time::Instant::now() < deadline {
       if health_ok.exists() {
           success = true;
           break
       }
       if let Ok(Some(_status)) = child.try_wait() {
           // Proces kandydata ulegĹ‚ przedwczesnemu zamkniÄ™ciu (crash / panic)
           break;
       }
       std::thread::sleep(std::time::Duration::from_millis(250));
   }
   ```

---

## 21. Deterministyczny Rollback na Linuksie
JeĹĽeli test zdrowia nie powiĂłdĹ‚ siÄ™ (`success == false`):
1. Natychmiastowe zabicie procesu kandydata: `let _ = child.kill();`.
2. UsuniÄ™cie wszystkich nowo utworzonych plikĂłw (`create_files`):
   ```rust
   for (_rel, orig) in &plan.create_files {
       let _ = fs::remove_file(orig);
   }
   ```
3. PrzywrĂłcenie oryginalnych plikĂłw programu z katalogu `.rollback`:
   ```rust
   for (rel, orig) in &plan.replace_files {
       let bkp = rollback_dir.join(rel);
       let _ = fs::copy(&bkp, orig);
   }
   ```
4. Ponowne uruchomienie poprzedniej wersji programu:
   ```rust
   let _ = std::process::Command::new(current).spawn();
   ```
5. ZwrĂłcenie bĹ‚Ä™du `UpdaterError::HealthCheckFailed` i powrĂłt do stanu bezpiecznego.

---

## 22. Czyszczenie (Cleanup)
W przypadku pomyĹ›lnego testu zdrowia (`success == true`):
1. UsuniÄ™cie katalogu `.staging`: `let _ = fs::remove_dir_all(&staging_dir);`.
2. UsuniÄ™cie katalogu `.rollback`: `let _ = fs::remove_dir_all(&rollback_dir);`.
3. UsuniÄ™cie znacznika `.health_ok`: `let _ = fs::remove_file(&health_ok);`.
4. Czyste zakoĹ„czenie starego procesu z kodem 0 (`std::process::exit(0)`).

---

## 23. Odzyskiwanie po Awarii Zasilania (Crash Recovery on Linux)
1. Podczas uruchamiania aplikacji sprawdzana jest obecnoĹ›Ä‡ pozostaĹ‚oĹ›ci po przerwanej aktualizacji (`.rollback`, `.staging`, `.update.lock`).
2. JeĹ›li plik blokady `.update.lock` wskazuje na PID procesu, ktĂłry juĹĽ nie istnieje w `/proc/<pid>`, blokada jest uznawana za osieroconÄ… (Stale Lock) i zwalniana.
3. Wszelkie niedokoĹ„czone pliki w `.staging` sÄ… bezpiecznie usuwane przed podjÄ™ciem nowej prĂłby.

---

## 24. Blokada WspĂłĹ‚bieĹĽnoĹ›ci (Linux Update Lock)
- Plik blokady `$baseDir/.update.lock` tworzony jest z uĹĽyciem `O_CREAT | O_EXCL` (lub odpowiednika w Rust).
- Zapisywane sÄ… metadane: PID procesu aktualizatora, znacznik czasu Unix oraz `attempt_id`.
- Przed operacjÄ… badane jest istnienie procesu za pomocÄ… `kill(pid, 0)` lub analizy `/proc/<pid>`. Zapobiega to deadlockom w przypadku nagĹ‚ego zabicia aplikacji sygnaĹ‚em `SIGKILL`.

---

## 25. Polityka Anulowania
- **Przed fazÄ… podmiany:** Anulowanie natychmiast przerywa pobieranie, usuwa plik tymczasowy `.tar.gz`, czyĹ›ci `.staging` i zwalnia `.update.lock`.
- **W trakcie fazy podmiany (`apply`):** Anulowanie przez UI jest zablokowane. W razie wymuszonego sygnaĹ‚u `SIGINT`/`SIGTERM`, proces prĂłbuje wykonaÄ‡ natychmiastowy rollback przywracajÄ…cy pliki z `.rollback`.

---

## 26. Badanie PojemnoĹ›ci Dysku (Disk Space Preflight on Linux)
Przed rozpoczÄ™ciem pobierania sprawdzana jest dostÄ™pnoĹ›Ä‡ miejsca za pomocÄ… wywoĹ‚ania systemowego `statvfs`:
$$\text{FreeSpace}(\text{base\_dir}) \ge \text{DownloadSize} + \text{ExtractedSize} + \text{BackupSize} + 50\text{ MB}$$
- JeĹ›li katalog tymczasowy `/tmp` znajduje siÄ™ na osobnym punkcie montowania (np. `tmpfs`), wolne miejsce sprawdzane jest niezaleĹĽnie dla `/tmp` oraz `$baseDir`.
- W razie niedoboru miejsca aktualizacja zostaje przerwana z bĹ‚Ä™dem `UpdaterError::Io("Brak wymaganego miejsca na dysku.")`.

---

## 27. ObsĹ‚uga BĹ‚Ä™dĂłw POSIX (Linux Error Model)
BĹ‚Ä™dy mapowane sÄ… na `UpdaterError`:
- `EACCES` / `EPERM`: `UpdaterError::Io("Brak uprawnieĹ„ do zapisu w katalogu programu")`.
- `EROFS`: `UpdaterError::Io("System plikĂłw zamontowany w trybie tylko do odczytu")`.
- `ENOSPC`: `UpdaterError::Io("Brak wolnego miejsca na partycji")`.
- `ENOENT` dla bibliotek: `UpdaterError::InvalidStagingLayout("Brak wymaganych bibliotek w lib/")`.

---

## 28. Logowanie Audytowe i PrywatnoĹ›Ä‡
- Dziennik operacji zapisywany jest w `$baseDir/update_audit.log`.
- Redagowane sÄ… wszelkie dane stacji, znaki wywoĹ‚awcze, identyfikatory QSO oraz Ĺ›cieĹĽki prywatne wykraczajÄ…ce poza katalog domowy.
- Uprawnienia pliku logu: `0o600` (dostÄ™p wyĹ‚Ä…cznie dla bieĹĽÄ…cego uĹĽytkownika).

---

## 29. Macierz ZagroĹĽeĹ„ BezpieczeĹ„stwa Linux (Security Matrix)

| ID ZagroĹĽenia | Wektor ataku | Ĺšrodowisko | Mechanizm mitygacji w projekcie | Ryzyko szczÄ…tkowe |
|---|---|---|---|---|
| **THR-LNX-01** | Tar Traversal (`../../`) | Ekstrakcja tar | `validate_archive_relative_path` odrzuca `..` i Ĺ›cieĹĽki bezwzglÄ™dne | Wyeliminowane |
| **THR-LNX-02** | Symlink Escape (`-> /etc/passwd`) | Ekstrakcja tar | Blokada symlinkĂłw zewnÄ™trznych; dozwolone tylko lokalne w `lib/` | Wyeliminowane |
| **THR-LNX-03** | Device Node Injection (tworzenie `/dev/*`) | Ekstrakcja tar | Odrzucanie wpisĂłw `Block`, `Char`, `Fifo` w `extract_tar_gz_safe` | Wyeliminowane |
| **THR-LNX-04** | Eskalacja uprawnieĹ„ przez `sudo` | Instalacja | Kod nie posiada wywoĹ‚aĹ„ `sudo`/`pkexec`; dziaĹ‚a w przestrzeni uĹĽytkownika | Wyeliminowane |
| **THR-LNX-05** | Nadpisanie bazy Ĺ‚Ä…cznoĹ›ci | Podmiana | `default_log.db` klasyfikowane jako `UserData` (pomijane w `UpdatePlan`) | Wyeliminowane |
| **THR-LNX-06** | Uszkodzenie bibliotek Hamlib | Podmiana | SpĂłjna podmiana caĹ‚ego katalogu `lib/` w jednej transakcji | Niskie |
| **THR-LNX-07** | Segfault nowej wersji z powodu ABI | Uruchomienie | Test zdrowia `--check-health-startup` + automatyczny rollback | Niskie |
| **THR-LNX-08** | Zmiana uprawnieĹ„ plikĂłw uĹĽytkownika | Ekstrakcja | Wymuszenie bezpiecznej maski `0o755` dla binariĂłw i `0o644` dla danych | Niskie |

---

## 30. Macierz TestĂłw Implementacyjnych M3 (Linux Test Matrix)

| ID Testu | Zakres testu | Warunki wejĹ›ciowe / Fixture | Oczekiwany rezultat | Ĺšrodowisko |
|---|---|---|---|---|
| **TEST-LNX-01** | Bezpieczna ekstrakcja tar.gz | Archiwum tar.gz ze Ĺ›cieĹĽkÄ… `/etc/shadow` | `Err(UpdaterError::UnsafeArchivePath)` | CI Linux |
| **TEST-LNX-02** | Odrzucenie FIFO i nodĂłw blokowych | Archiwum tar.gz zawierajÄ…ce nody `Fifo` | `Err(UpdaterError::UnsafeArchivePath)` | CI Linux |
| **TEST-LNX-03** | SpĹ‚aszczanie korzenia `SPLogbook-Linux-x86_64/` | Archiwum z pojedynczym katalogiem nadrzÄ™dnym | Pliki trafiajÄ… bezpoĹ›rednio do `.staging` | CI Linux |
| **TEST-LNX-04** | Zachowanie bitu `0o755` | Weryfikacja uprawnieĹ„ binarki `splogbook` po ekstrakcji | `mode & 0o111 != 0` | CI Linux |
| **TEST-LNX-05** | Ochrona baz danych w `UpdatePlan` | Staging zawiera nowe `serviceLOG.db`, active ma istniejÄ…ce | `preserve_files` chroni bazÄ™, brak w `replace_files` | CI Linux |
| **TEST-LNX-06** | Odmowa dla `/usr/bin` | ĹšcieĹĽka programu: `/usr/bin/splogbook` | `Err(UpdaterError::SafetyGate)` przed pobraniem | CI Linux |
| **TEST-LNX-07** | Rollback po awarii zdrowia | Nowa binarka koĹ„czÄ…ca siÄ™ `SIGSEGV` lub kodem 1 | PrzywrĂłcenie starej binarki z `.rollback`, czysty powrĂłt | CI Linux |

---

## 31. Rejestr Decyzji WĹ‚aĹ›ciciela dla Linux
- **`OWNER-M2-04`:** Linux M3 obejmuje wyĹ‚Ä…cznie oficjalny pakiet Portable tar.gz. (Rekomendacja: **APPROVED**).
- **`OWNER-M2-05`:** AppImage jako osobny etap w przyszĹ‚oĹ›ci. (Rekomendacja: **DEFERRED**).
- **`OWNER-M2-06`:** Pakiety systemowe DEB/RPM/Flatpak pozostajÄ… poza auto-updaterem SPLogbook. (Rekomendacja: **APPROVED â€” instalacje systemowe zarzÄ…dzane przez systemowe menedĹĽery pakietĂłw**).
- **`OWNER-M2-07`:** Instalacje w `/usr/bin` odrzucane z monitem o uĹĽycie menedĹĽera pakietĂłw. (Rekomendacja: **APPROVED**).

---

## 32. Kryteria Akceptacji Implementacji M3 (Linux Definition of Done)
1. 100% testĂłw jednostkowych i integracyjnych przechodzi na natywnym runnerze Ubuntu w potoku GitHub Actions.
2. Funkcja `extract_tar_gz_safe` bezwzglÄ™dnie blokuje path traversal, Ĺ›cieĹĽki bezwzglÄ™dne oraz nody specjalne.
3. Podmiana plikĂłw zachowuje uprawnienia wykonywalne `0o755` oraz strukturÄ™ `lib/`.
4. Test symulacji segfaultu kandydata koĹ„czy siÄ™ pomyĹ›lnym przywrĂłceniem poprzedniej wersji bez utraty danych logu.
5. Kod nie zawiera ĹĽadnych odwoĹ‚aĹ„ do poleceĹ„ `sudo` lub `pkexec`.

---

## 33. Odrzucone Alternatywy
1. **Odrzucono: WywoĹ‚ywanie `sudo` lub `pkexec`:** Narusza podstawowe zasady bezpieczeĹ„stwa aplikacji desktopowych; grozi eskalacjÄ… uprawnieĹ„ i uszkodzeniem uprawnieĹ„ plikĂłw uĹĽytkownika w katalogu domowym.
2. **Odrzucono: ZastÄ™powanie binarki surowym plikiem `.tar.gz`:** Historyczny bĹ‚Ä…d wersji legacy caĹ‚kowicie niszczÄ…cy instalacjÄ™.
3. **Odrzucono: PĹ‚aska struktura bez katalogu nadrzÄ™dnego w tar.gz:** Sprzeczne z dotychczasowym oficjalnym formatem wydania `SPLogbook-Linux-x86_64.tar.gz`.

---

## 34. Ryzyka SzczÄ…tkowe
1. **RĂłĹĽne systemy plikĂłw (Cross-Filesystem Staging):** JeĹ›li katalog `.staging` lub `%TEMP%` znajdzie siÄ™ na innym woluminie niĹĽ katalog programu, wywoĹ‚anie `rename()` zwrĂłci bĹ‚Ä…d `EXDEV`.
   - *Mitygacja:* Projekt przewiduje automatyczny fallback na `copy()` z zachowaniem uprawnieĹ„ i fsync.
2. **NiezgodnoĹ›Ä‡ wersji GLIBC:** JeĹ›li nowa wersja zostaĹ‚a skompilowana na nowszym systemie niĹĽ ten, na ktĂłrym uruchamia jÄ… uĹĽytkownik, kandydat nie wystartuje (`version GLIBC_X.XX not found`).
   - *Mitygacja:* Health check wykryje bĹ‚Ä…d linkera dynamicznego podczas prĂłby startu kandydata i automatycznie przywrĂłci dziaĹ‚ajÄ…cÄ… poprzedniÄ… wersjÄ™.
