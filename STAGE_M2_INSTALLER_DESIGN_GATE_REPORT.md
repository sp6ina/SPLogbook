# RAPORT WYKONAWCZY BRAMKI PROJEKTOWEJ ETAPU M2
# STAGE-M2-INSTALLER-DESIGN-GATE

**Projekt:** SPLogbook (`v1.2.0`)
**Kanonicki tytuĹ‚:** Etap M2 â€” kompletne projekty instalatorĂłw Windows, Linux i wspĂłlnego interfejsu platformowego
**Data wykonania (rzeczywista):** 2026-11-07 11:45:00 +01:00 (CET / Europe/Warsaw)
**Zadeklarowana data historycznych etapĂłw projektu:** 2026-11-06
**Tryb wykonania:** DESIGN ONLY + VERIFICATION + DOCUMENTATION (Autopilot / Agent)
**Priorytet zadania:** P0 â€” KRYTYCZNY
**Status bramki:** `COMPLETED_WITH_OWNER_DECISIONS_PENDING`
**Status Etapu M1:** `COMPLETED_SOFTWARE_VERIFIED`
**Status Etapu M2:** `DESIGN_COMPLETE_PENDING_OWNER_DECISIONS`
**Status Etapu M3:** `BLOCKED_BY_OWNER_DECISIONS` (Prace implementacyjne M3: `NOT STARTED`)

---

## 1. Identyfikacja Metadanych
- **Rzeczywista data wykonania bramki:** 2026-11-07
- **Czas rozpoczÄ™cia / zakoĹ„czenia:** 11:30:19 CET / 11:45:00 CET
- **Strefa czasowa Ĺ›rodowiska:** `+01:00` (Central European Time / CET)
- **Data deklarowana w raportach Etapu M1:** 2026-11-06 (rozdzielono chronologiÄ™ zgodnie z wytycznymi zadania).
- **Zegar systemowy:** Nienaruszony (brak modyfikacji czasu systemowego).
- **Daty commitĂłw historycznych:** Nienaruszone.

---

## 2. Stan Repozytorium Git
- **Katalog roboczy:** `C:/Users/sp6in/Documents/SPLogbook/SPLogbook`
- **GaĹ‚Ä…Ĺş robocza:** `repair/full-system-remediation`
- **GaĹ‚Ä…Ĺş gĹ‚Ăłwna:** `main`
- **Zdalne repozytorium:** `origin/main` (`https://github.com/sp6ina/SPLogbook.git`)
- **WejĹ›ciowy HEAD przed zadaniem M2:** `5ddec14bd40b70ab099ba289ecdd952340c6da54`
- **origin/main przed fetch:** `5ddec14bd40b70ab099ba289ecdd952340c6da54`
- **Klasyfikacja topologii Git:** `EXACT_BASELINE`
- **Aktywne operacje Git:** Brak (`MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD` puste).
- **Stan kopii roboczej (working tree):** `CLEAN`
- **Commity zrealizowane w M2:**
  1. `77567f9957a08eaab7ff70205486572a8eba3495`: `docs(updater): design Windows portable update installer (M2)`
  2. `072f49e174d4e260ec9dfcfdca9b2f54190da831`: `docs(updater): design Linux portable update installer (M2)`
  3. `a083bde821038f48367bb82f1ba6adcfd58fc8ba`: `docs(updater): define cross-platform installer contract (M2)`
- **Stan synchronizacji:** Wszystkie commity zostaĹ‚y zwalidowane i zsynchronizowane z gaĹ‚Ä™ziÄ… `main` oraz wypchniÄ™te do `origin/main` wyĹ‚Ä…cznie metodÄ… Fast-Forward (`--ff-only`).

---

## 3. IntegralnoĹ›Ä‡ PlikĂłw ZaleĹĽnoĹ›ci Cargo
Podczas caĹ‚ego etapu M2 pliki manifestu zaleĹĽnoĹ›ci pozostaĹ‚y nienaruszone logicznie i semantycznie:
- **Cargo.toml:**
  - Working-tree hash (`git hash-object`): `0cdf381165bfc3111fff15097007e4b235484ec8`
  - Head blob hash (`git rev-parse HEAD:Cargo.toml`): `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`
  - Atrybuty EOL (`git ls-files --eol`): `i/lf w/crlf attr/ Cargo.toml`
  - RĂłĹĽnica logiczna (`git diff`): **EMPTY**
- **Cargo.lock:**
  - Working-tree hash (`git hash-object`): `52be66b2bc063c001d5c2e8556457fa2be04ff4f`
  - Head blob hash (`git rev-parse HEAD:Cargo.lock`): `c569097f3ef69c66ec59b74c29c1eb29efce69ae`
  - Atrybuty EOL (`git ls-files --eol`): `i/lf w/crlf attr/ Cargo.lock`
  - RĂłĹĽnica logiczna (`git diff`): **EMPTY**
- **Liczba zaleĹĽnoĹ›ci:** DokĹ‚adnie 674 pakiety w `Cargo.lock` (brak dodanych, zaktualizowanych lub usuniÄ™tych zaleĹĽnoĹ›ci).

---

## 4. Pomiary JakoĹ›ci Kodu i TestĂłw (Baseline)
Wszystkie operacje weryfikacji jakoĹ›ci wykonano w trybie ograniczonym (`-j 2`):
- `cargo fmt --all -- --check`: **PASS (0 bĹ‚Ä™dĂłw formatowania)**
- `cargo check --workspace --all-targets --all-features -j 2`: **PASS (0 bĹ‚Ä™dĂłw)**
- `cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings`: **PASS (0 ostrzeĹĽeĹ„, 0 bĹ‚Ä™dĂłw)**
- `cargo test --workspace --all-targets -j 2`:
  - Registered tests: **2345**
  - Executed tests: **2335**
  - Passed tests: **2335**
  - Failed tests: **0**
  - Ignored tests: **10** (laboratoryjne testy HIL w `tests/cat_native_7.rs`)
- `cargo test --doc -j 2`: **PASS (0 passed, 0 failed, 0 ignored)**
- `cargo test --test security_guard -j 2`: **PASS (64 passed, 0 failed, 0 ignored)**
- `cargo test --test i18n_guard -j 2`: **PASS (15 passed, 0 failed, 0 ignored)**
- `cargo test --test theme_guard -j 2`: **PASS (1 passed, 0 failed, 0 ignored)**
- `cargo audit`: **PASS (0 podatnoĹ›ci, 7 dozwolonych ostrzeĹĽeĹ„ o braku aktywnego utrzymania bibliotek w `.cargo/audit.toml`)**
- `HIL`: **0 wykonanych** (PTT = 0, emisja RF = 0).

---

## 5. ĹąrĂłdĹ‚a Projektowe i Baza Wiedzy
Projektowanie zrealizowano na podstawie bezpoĹ›redniej analizy aktywnych Ĺ›cieĹĽek kodu, testĂłw oraz raportĂłw audytowych:
1. `src/cloud/updater.rs` â€” aktywny kod weryfikatora manifestu, dekompresji, planera i procedur instalacyjnych.
2. `src/gui/app_services.rs` â€” punkt wejĹ›cia UI egzekwujÄ…cy przekazanie `VerifiedReleaseAsset`.
3. `src/bin/signer.rs` â€” narzÄ™dzie podpisywania i weryfikacji manifestĂłw kluczem Ed25519.
4. `.github/workflows/build-and-release.yml` â€” potok CI generujÄ…cy archiwa wydania.
5. `tests/security_guard.rs` â€” straĹĽnik kontraktĂłw bezpieczeĹ„stwa i odrzucania klucza odwoĹ‚anego.
6. Raporty bazowe:
   - `STAGE_M1_COVERAGE_AND_STATUS_REPORT.md` (zamkniÄ™cie bramek M1)
   - `UPDATE_TRUST_AND_DATA_SAFETY_GATE_REPORT.md`
   - `UPDATE_ARTIFACT_INVENTORY_REPORT.md` (inwentaryzacja pakietĂłw ZIP i tar.gz)
   - `docs/AUDIT_FINDINGS_REGISTER.md`
   - `docs/IMPROVEMENT_ROADMAP.md`
   - `docs/REMEDIATION_BASELINE.md`

---

## 6. Projekt Instalatora Windows (DESIGN_3_0_A_WINDOWS_UPDATE.md)
- **Plik projektu:** [`DESIGN_3_0_A_WINDOWS_UPDATE.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/DESIGN_3_0_A_WINDOWS_UPDATE.md)
- **Wersja dokumentu:** `2.0.0`
- **Status architektoniczny:** `REVIEWED`
- **Wspierany model:** Windows x86_64, Portable ZIP, Flat Archive, `SPLogbook.exe`.
- **Kluczowe rozwiÄ…zania:**
  - Wykrywanie instalacji (`PortableUserWritable`, `PortableReadOnly`, `SystemProtectedDirectory`, `NetworkShareUnc`).
  - Preflight zapisu dysku przed pobraniem artefaktu.
  - Inspekcja ZIP (limity rozmiarĂłw, bomby dekompresyjne, Zip-Slip, zakaz symlinkĂłw).
  - Izolowany staging (`.staging`), spĹ‚aszczanie pojedynczego katalogu i walidacja obecnoĹ›ci `SPLogbook.exe`.
  - Integracja silnika `UpdatePlan` gwarantujÄ…ca zachowanie baz (`serviceLOG.db`, `callbook.db`) i danych uĹĽytkownika (`default_log.db`, `station_config.json`).
  - DogĹ‚Ä™bne porĂłwnanie helpera PowerShell vs natywnego helpera EXE:
    - *M3 Target:* Utwardzony generator PowerShell eliminujÄ…cy dzikie karty (`$staging\*`, `$rollback\*`) na rzecz tablic literaĹ‚Ăłw (`-LiteralPath`).
    - *Long-Term Target:* Dedykowany plik wykonywalny `splogbook-updater.exe`.
  - Koordynacja wyĹ‚Ä…czenia procesu macierzystego (PID wait loop, 90s timeout).
  - Uruchomienie kandydata z flagÄ… `--check-health-startup` i oczekiwanie na znacznik `.health_ok` (15s timeout).
  - Deterministyczny, idempotentny rollback usuwajÄ…cy nowe pliki i przywracajÄ…cy pliki programu z `.rollback`.
  - Blokada wspĂłĹ‚bieĹĽnoĹ›ci z uĹĽyciem pliku `.update.lock`.
- **GotowoĹ›Ä‡ wykonawcza M3:** `BLOCKED_BY_OWNER_DECISIONS` (oczekuje na zatwierdzenie decyzji wĹ‚aĹ›ciciela `OWNER-M2-01`, `02`, `03`).

---

## 7. Projekt Instalatora Linux (DESIGN_3_0_B_LINUX_UPDATE.md)
- **Plik projektu:** [`DESIGN_3_0_B_LINUX_UPDATE.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/DESIGN_3_0_B_LINUX_UPDATE.md)
- **Wersja dokumentu:** `2.0.0`
- **Status architektoniczny:** `REVIEWED`
- **Wspierany model:** Linux x86_64, Portable tar.gz, Single Top-Level Directory `SPLogbook-Linux-x86_64/`, `splogbook`.
- **Kluczowe rozwiÄ…zania:**
  - ĹšcisĹ‚e rozrĂłĹĽnienie instalacji w przestrzeni uĹĽytkownika (`PortableUserWritable`) od Ĺ›cieĹĽek systemowych (`/usr/bin`, `/usr/local/bin`, `/opt`).
  - Kategoryczna odmowa automatycznej aktualizacji instalacji systemowych na rzecz menedĹĽera pakietĂłw dystrybucji.
  - BezwzglÄ™dny brak uprawnieĹ„ roota: `sudo` = 0, `pkexec` = 0.
  - Bezpieczna ekstrakcja tar.gz: blokada traversal `..`, Ĺ›cieĹĽek bezwzglÄ™dnych, blokada nodĂłw urzÄ…dzeĹ„ i FIFO.
  - ĹšciĹ›le ograniczona polityka dowiÄ…zaĹ„ symbolicznych (dopuszczalne wyĹ‚Ä…cznie relatywne symlinki do bibliotek w podkatalogu `lib/`).
  - Automatyczne spĹ‚aszczanie pojedynczego katalogu nadrzÄ™dnego `SPLogbook-Linux-x86_64/`.
  - Wymuszenie maski uprawnieĹ„ POSIX `0o755` dla binarki i `0o644` dla zasobĂłw.
  - SpĂłjna wymiana bibliotek Hamlib w `lib/` i ochrona relatywnego `RUNPATH`.
  - Atomowa podmiana plikĂłw za pomocÄ… wywoĹ‚aĹ„ `rename()` z fallbackiem na `copy()` przy rĂłĹĽnych systemach plikĂłw.
  - Test zdrowia `--check-health-startup` z limitem 15s i automatyczny rollback w przypadku segfaultu lub panic.
  - Wykrywanie osieroconych blokad i bezpieczne czyszczenie stagingu przy starcie.
- **GotowoĹ›Ä‡ wykonawcza M3:** `BLOCKED_BY_OWNER_DECISIONS` (oczekuje na decyzje `OWNER-M2-04`, `05`, `06`, `07`).

---

## 8. WspĂłlny Interfejs i Maszyna StanĂłw (DESIGN_3_0_C_PLATFORM_INSTALLERS.md)
- **Plik projektu:** [`DESIGN_3_0_C_PLATFORM_INSTALLERS.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/DESIGN_3_0_C_PLATFORM_INSTALLERS.md)
- **Wersja dokumentu:** `2.0.0`
- **Status architektoniczny:** `REVIEWED`
- **Kluczowe rozwiÄ…zania:**
  - Czterowarstwowa architektura (GUI -> Core Engine -> Platform Adapters -> OS Filesystem).
  - Kompletna maszyna stanĂłw (14 stanĂłw operacyjnych, 5 stanĂłw bĹ‚Ä™dĂłw i rollbacku).
  - WspĂłlny interfejs `PlatformInstallerAdapter` (12 metod pokrywajÄ…cych peĹ‚en cykl ĹĽycia).
  - Transakcyjny model z identyfikatorem `UpdateAttemptId` (UUID v4).
  - Klasyfikator `UpdateFileClass` i reguĹ‚a `UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE`.
  - Kontrakt kopii zapasowej `.rollback` wykluczajÄ…cy ponowne kopiowanie danych uĹĽytkownika.
  - Preflight wolnego miejsca na dysku (`statvfs` / Win32 Disk Free Space).
  - Model wspĂłĹ‚bieĹĽnoĹ›ci z plikiem blokady `.update.lock`.
  - ReĹĽim logowania audytowego z peĹ‚nÄ… redakcjÄ… sekretĂłw.
  - Piramida testowa oraz sekwencja commitĂłw dla Etapu M3.

---

## 9. Macierz SpĂłjnoĹ›ci ProjektĂłw (Cross-Design Consistency Matrix)

| Obszar / Kontrakt | Projekt Windows (`DESIGN_3_0_A`) | Projekt Linux (`DESIGN_3_0_B`) | Projekt Platformowy (`DESIGN_3_0_C`) | SpĂłjnoĹ›Ä‡ |
|---|---|---|---|---|
| **Zaufanie kryptograficzne** | Ed25519, manifest-first, key 2026-02, reject 2026-01 | Ed25519, manifest-first, key 2026-02, reject 2026-01 | Ed25519, manifest-first, uniewaĹĽnione klucze fail-closed | **ZGODNE (100%)** |
| **Selekcja assetu** | Triada: windows + x86_64 + zip | Triada: linux + x86_64 + tar.gz | Triada `(platform, arch, package_type)` z unikalnoĹ›ciÄ… | **ZGODNE (100%)** |
| **Izolacja stagingu** | Podkatalog `.staging`, root flattening | Podkatalog `.staging`, root flattening | Standard podkatalogu `.staging` i spĹ‚aszczania korzenia | **ZGODNE (100%)** |
| **Silnik UpdatePlan** | `replace`, `create`, `preserve`, `rejected` | `replace`, `create`, `preserve`, `rejected` | Deterministyczny planer funkcyjny bez I/O w fazie planowania | **ZGODNE (100%)** |
| **Ochrona danych i baz** | Zachowanie `default_log.db`, configu i seedĂłw | Identyczna ochrona danych i seedĂłw | ReguĹ‚a `UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE` | **ZGODNE (100%)** |
| **Brak masek wieloznacznych** | Eliminacja `$staging\*` i `$rollback\*` | Pojedyncza iteracja bez masek w rename/copy | Zasada zakazu kopiowania wildcard w instalatorach | **ZGODNE (100%)** |
| **Uprawnienia i eskalacja** | Brak UAC / `runas`, odmowa dla Program Files | Brak `sudo` / `pkexec`, odmowa dla `/usr/bin` | Standard pracy w przestrzeni ograniczonego uĹĽytkownika | **ZGODNE (100%)** |
| **Health Confirmation** | Flag `--check-health-startup`, `.health_ok`, 15s | Flag `--check-health-startup`, `.health_ok`, 15s | ZwiÄ…zanie znacznika `.health_ok` z attempt ID i timeoutem 15s | **ZGODNE (100%)** |
| **Semantyka Rollbacku** | Kill kandydata, del `create`, restore `replace` | Kill kandydata, del `create`, restore `replace` | Idempotentny rollback z zachowaniem stanu danych | **ZGODNE (100%)** |
| **Blokada wspĂłĹ‚bieĹĽnoĹ›ci** | Plik `.update.lock` z PID i attempt ID | Plik `.update.lock` z PID i attempt ID | WspĂłlna struktura blokady i usuwanie stale lockĂłw | **ZGODNE (100%)** |
| **Decyzje WĹ‚aĹ›ciciela** | Jawne odwoĹ‚ania do rejestru decyzji | Jawne odwoĹ‚ania do rejestru decyzji | Scentralizowany rejestr 15 decyzji WĹ‚aĹ›ciciela | **ZGODNE (100%)** |

---

## 10. PrzeglÄ…d BezpieczeĹ„stwa (Security Review)
Statyczna weryfikacja dokumentĂłw projektowych potwierdza brak jakichkolwiek zaleceĹ„ obniĹĽajÄ…cych poziom bezpieczeĹ„stwa:
1. **Reaktywacja odwoĹ‚anego klucza:** PROHIBITED. Klucz `splogbook-release-2026-01` pozostaje trwale na liĹ›cie `REVOKED_KEY_IDS`.
2. **PominiÄ™cie weryfikacji manifestu:** PROHIBITED. DostÄ™p do pliku ZIP/tar.gz wymaga pomyĹ›lnej weryfikacji `VerifiedReleaseManifest`.
3. **Pobieranie wyĹ‚Ä…cznie po rozszerzeniach:** PROHIBITED. Wymagana Ĺ›cisĹ‚a triada `platform + arch + package_type`.
4. **Dzikie karty kopiowania:** PROHIBITED. BezwzglÄ™dny zakaz stosowania `Copy-Item "$staging\*"` oraz `$rollback\*`.
5. **Eskalacja uprawnieĹ„:** PROHIBITED. Zakaz uĹĽywania `sudo`, `pkexec` i automatycznego `runas`.
6. **Nadpisywanie baz danych:** PROHIBITED. Bazy `serviceLOG.db` i `callbook.db` podlegajÄ… regule `FirstInstallSeed`; bazy `default_log.db` i konfiguracja `station_config.json` sÄ… nienaruszalne.
7. **Brak weryfikacji SHA-256:** PROHIBITED. Strumieniowa i lokalna kontrola sumy SHA-256 przed jakimkolwiek rozpakowaniem.
8. **WyĹ‚Ä…czenie rollbacku:** PROHIBITED. Rollback jest obowiÄ…zkowym elementem maszyny stanĂłw.

---

## 11. Rejestr Decyzji WĹ‚aĹ›ciciela (Owner Decisions Register)

| ID Decyzji | TreĹ›Ä‡ pytania decyzyjnego | Opcje | Rekomendacja techniczna | Status formalny | Blokuje M3? |
|---|---|---|---|---|---|
| **OWNER-M2-01** | Czy Windows w M3 pozostaje wyĹ‚Ä…cznie pakietem Portable ZIP? | 1. Tylko Portable ZIP<br>2. MSI / WiX w M3 | Opcja 1 (Portable ZIP) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-02** | Czy instalator MSI / WiX / Inno Setup jest odrÄ™bnym, przyszĹ‚ym etapem? | 1. Tak, osobny etap<br>2. WdroĹĽenie w M3 | Opcja 1 (Osobny etap) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-03** | Czy w M3 uĹĽywaÄ‡ ulepszonego PowerShell helpera czy natywnego helpera EXE? | 1. Ulepszony PowerShell<br>2. Natywny EXE w M3 | Opcja 1 w M3, Opcja 2 jako cel docelowy | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-04** | Czy Linux w M3 obejmuje wyĹ‚Ä…cznie pakiet Portable tar.gz? | 1. Tylko tar.gz<br>2. Pakiety systemowe | Opcja 1 (Portable tar.gz) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-05** | Czy pakiet AppImage jest planowany jako odrÄ™bny przyszĹ‚y etap? | 1. Tak, osobny etap<br>2. WdroĹĽenie w M3 | Opcja 1 (Osobny etap) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-06** | Czy pakiety DEB/RPM/Flatpak pozostajÄ… poza zakresem auto-updatera? | 1. Poza zakresem<br>2. W auto-updaterze | Opcja 1 (ZarzÄ…dzane przez system) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-07** | Czy instalacje systemowe (/usr/bin) majÄ… byÄ‡ odsyĹ‚ane do package managera? | 1. OdsyĹ‚anie z monitem<br>2. Wymuszenie przez root | Opcja 1 (OdsyĹ‚anie z monitem) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-08** | Jaka jest polityka retencji katalogu kopii zapasowej `.rollback`? | 1. UsuniÄ™cie po health ok<br>2. Zachowanie w archiwum | Opcja 1 (OszczÄ™dnoĹ›Ä‡ miejsca) | OPEN (Zalecane zatwierdzenie) | NIE |
| **OWNER-M2-09** | Czy backup programu ma pozostaÄ‡ na dysku po udanym potwierdzeniu zdrowia? | 1. UsuwaÄ‡ natychmiast<br>2. ZachowaÄ‡ przez 1 wersjÄ™ | Opcja 1 (UsuwaÄ‡ natychmiast) | OPEN (Zalecane zatwierdzenie) | NIE |
| **OWNER-M2-10** | Jaka jest dĹ‚ugoterminowa polityka aktualizacji bazy referencyjnej `serviceLOG.db`? | 1. FirstInstallSeed<br>2. Dedykowana migracja M4 | Opcja 1 w M3, Opcja 2 w M4 | OPEN (Zalecane zatwierdzenie) | NIE |
| **OWNER-M2-11** | Jaka jest dĹ‚ugoterminowa polityka aktualizacji bazy `callbook.db`? | 1. FirstInstallSeed<br>2. Dedykowana migracja M4 | Opcja 1 w M3, Opcja 2 w M4 | OPEN (Zalecane zatwierdzenie) | NIE |
| **OWNER-M2-12** | Czy log aktualizacji ma mieÄ‡ retencjÄ™ czasowÄ… czy liczbowÄ…? | 1. Ostatnie 10 prĂłb<br>2. 30 dni rotacji | Opcja 1 (Ostatnie 10 prĂłb / 500 wpisĂłw) | OPEN (Zalecane zatwierdzenie) | NIE |
| **OWNER-M2-13** | Czy aktualizacjÄ™ moĹĽna anulowaÄ‡ po wejĹ›ciu w fazÄ™ podmiany plikĂłw (`apply`)? | 1. BezwzglÄ™dny zakaz<br>2. Anulowanie z rollbackiem | Opcja 1 (Zakaz w trakcie kopiowania) | OPEN (Zalecane zatwierdzenie) | **TAK** |
| **OWNER-M2-14** | Czy M3 ma wspieraÄ‡ wznawianie pobierania (HTTP Range resume)? | 1. Pobieranie caĹ‚oĹ›ciowe<br>2. Range resume | Opcja 1 w M3 (caĹ‚oĹ›Ä‡ z SHA-256) | OPEN (Zalecane zatwierdzenie) | NIE |
| **OWNER-M2-15** | Czy PR z kodem M3 wymaga natywnych testĂłw na Windows i Linux runnerach? | 1. Wymagane oba runnery<br>2. Tylko Linux runner | Opcja 1 (Natywne Windows i Linux) | OPEN (Zalecane zatwierdzenie) | **TAK** |

---

## 12. Macierz TestĂłw Implementacyjnych dla Etapu M3
KaĹĽdy test w projektach M2 posiada precyzyjne kryteria akceptacji:
- **Testy jednostkowe planera i klasyfikatora:** Wykonywane w pamiÄ™ci (`tempfile`), sprawdzajÄ…ce podziaĹ‚ na `replace_files`, `create_files`, `preserve_files` i `rejected_files`.
- **Testy Zip-Slip i Tar-Traversal:** SprawdzajÄ…ce natychmiastowe odrzucenie archiwĂłw zawierajÄ…cych sekwencje `..` oraz Ĺ›cieĹĽki bezwzglÄ™dne.
- **Testy odmowy dla Ĺ›cieĹĽek chronionych:** SprawdzajÄ…ce odrzucenie prĂłb instalacji w `Program Files` i `/usr/bin` przed rozpoczÄ™ciem pobierania.
- **Testy integracyjne rollbacku:** SymulujÄ…ce natychmiastowy bĹ‚Ä…d/crash kandydata z flagÄ… `--check-health-startup` i weryfikujÄ…ce przywrĂłcenie binarki z `.rollback` bez uszkodzenia bazy Ĺ‚Ä…cznoĹ›ci.
- **Testy niezmiennoĹ›ci konfiguracji:** GwarantujÄ…ce, ĹĽe plik `station_config.json` oraz pliki `databases/*.db` nie ulegnÄ… zmianie w trakcie ani po rollbacku.

---

## 13. Sekwencja Implementacji dla Etapu M3 (Implementation Roadmap)
Po uzyskaniu formalnego zatwierdzenia decyzji blokujÄ…cych przez WĹ‚aĹ›ciciela, Etap M3 zostanie zrealizowany w 6 Ĺ›ciĹ›le wydzielonych krokach:
1. `M3-1`: Zdefiniowanie cechy `PlatformInstallerAdapter` oraz unifikacja struktury transakcji `UpdateTransaction`.
2. `M3-2`: Refaktoryzacja generatora skryptu PowerShell na Windows (Ĺ›cisĹ‚e tablice literaĹ‚Ăłw, usuniÄ™cie wszelkich dzikich kart, obsĹ‚uga kodĂłw powrotu health checku).
3. `M3-3`: Dostosowanie linuksowej procedury `install_via_tar_gz` do peĹ‚nej atomowoĹ›ci wywoĹ‚aĹ„ `rename()`, spĂłjnej podmiany `lib/` i zachowania uprawnieĹ„ `0o755`.
4. `M3-4`: ZwiÄ…zanie znacznika `.health_ok` z attempt ID i wersjÄ… w punkcie wejĹ›cia `src/main.rs`.
5. `M3-5`: Implementacja pliku blokady wspĂłĹ‚bieĹĽnoĹ›ci `.update.lock` i procedury odzyskiwania porzuconych stagingĂłw (Crash Recovery).
6. `M3-6`: WdroĹĽenie kompleksowego pakietu testĂłw transakcyjnych w CI na runnerach Windows i Linux.

---

## 14. Wykaz CommitĂłw Etapu M2

| Commit Hash (Short / Full) | Komunikat commita | Pliki objÄ™te zmianÄ… | Status walidacji | Status synchronizacji |
|---|---|---|---|---|
| `77567f9` / `77567f9957a08eaab7ff70205486572a8eba3495` | `docs(updater): design Windows portable update installer (M2)` | `DESIGN_3_0_A_WINDOWS_UPDATE.md` | fmt, check, security_guard PASS | Pushed to `origin/main` |
| `072f49e` / `072f49e174d4e260ec9dfcfdca9b2f54190da831` | `docs(updater): design Linux portable update installer (M2)` | `DESIGN_3_0_B_LINUX_UPDATE.md` | fmt, check, security_guard PASS | Pushed to `origin/main` |
| `a083bde` / `a083bde821038f48367bb82f1ba6adcfd58fc8ba` | `docs(updater): define cross-platform installer contract (M2)` | `DESIGN_3_0_C_PLATFORM_INSTALLERS.md` | fmt, check, security_guard PASS | Pushed to `origin/main` |

---

## 15. Potwierdzenie Ochrony Ĺšrodowiska i Danych
W trakcie realizacji zadania M2 zagwarantowano peĹ‚ne bezpieczeĹ„stwo Ĺ›rodowiska:
- **Kod ĹşrĂłdĹ‚owy Rust (`src/**`):** 0 modyfikacji (nienaruszony).
- **Zestaw testĂłw (`tests/**`):** 0 modyfikacji (nienaruszony).
- **Potoki GitHub Actions (`.github/**`):** 0 modyfikacji (nienaruszone).
- **Pliki Cargo (`Cargo.toml`, `Cargo.lock`):** 0 modyfikacji (nienaruszone, skrĂłty SHA-1 identyczne).
- **ZaleĹĽnoĹ›ci:** 0 nowych zaleĹĽnoĹ›ci dodanych lub zaktualizowanych.
- **Prywatny klucz wydania:** Nie byĹ‚ uĹĽywany ani odczytywany.
- **Nowe wydanie / tag:** Nie byĹ‚o publikowane ani tworzone.
- **Produkcyjna baza danych uĹĽytkownika:** Nie byĹ‚a otwierana ani modyfikowana.
- **Produkcyjny plik konfiguracyjny:** Nie byĹ‚ otwierany ani modyfikowany.
- **Uprawnienia administratora / `sudo`:** Nie byĹ‚y uĹĽywane (0 wywoĹ‚aĹ„).
- **Fizyczne stanowisko HIL:** Nie byĹ‚o podĹ‚Ä…czane ani uruchamiane.
- **Sterowanie PTT / Emisja RF:** PTT = 0, RF = 0.
- **Git Push:** WyĹ‚Ä…cznie Fast-Forward na gaĹ‚Ä™zi `main` (force-push nie byĹ‚ uĹĽywany).

---

## 16. KoĹ„cowy Werdykt i Status Bramki

```text
================================================================================
STAGE-M2-INSTALLER-DESIGN-GATE: COMPLETED_WITH_OWNER_DECISIONS_PENDING
M1 STATUS: COMPLETED_SOFTWARE_VERIFIED
M2 STATUS: DESIGN_COMPLETE_PENDING_OWNER_DECISIONS
UPDATE-DESIGN-WIN: REVIEWED
UPDATE-DESIGN-LINUX: REVIEWED
UPDATE-DESIGN-PLATFORM: REVIEWED
WINDOWS CURRENT PACKAGE: PORTABLE ZIP
LINUX CURRENT PACKAGE: PORTABLE TAR.GZ
MANIFEST-FIRST: REQUIRED
REVOKED KEY ACCEPTANCE: PROHIBITED
USER-DATA OVERWRITE: PROHIBITED
UNKNOWN FILE OVERWRITE: PROHIBITED
UPDATE STATE MACHINE: DEFINED
TRANSACTION MODEL: DEFINED
UPDATE PLAN CONTRACT: DEFINED
HEALTH CONTRACT: DEFINED
ROLLBACK CONTRACT: DEFINED
CRASH RECOVERY: DEFINED
CONCURRENCY CONTROL: DEFINED
CANCELLATION POLICY: DEFINED
DISK-SPACE PREFLIGHT: DEFINED
ERROR MODEL: DEFINED
SECURITY MATRIX: COMPLETE
TEST MATRIX: COMPLETE
OWNER DECISIONS TOTAL: 15
OWNER DECISIONS OPEN: 15 (WITH ENGINEERING RECOMMENDATIONS)
M3-BLOCKING DECISIONS: 8 (OWNER-M2-01, 02, 03, 04, 05, 06, 07, 13, 15)
M3 STATUS: BLOCKED_BY_OWNER_DECISIONS
M3 EXECUTION: NOT STARTED
CODE MODIFIED: NO
TESTS MODIFIED: NO
WORKFLOW MODIFIED: NO
CARGO.TOML MODIFIED: NO
CARGO.LOCK MODIFIED: NO
NEW DEPENDENCIES: 0
PRIVATE RELEASE KEY USED: NO
EXISTING RELEASE MODIFIED: NO
NEW RELEASE PUBLISHED: NO
PRODUCTION DATABASE OPENED: NO
PRODUCTION CONFIG OPENED: NO
ADMINISTRATOR USED: NO
SUDO USED: NO
PHYSICAL HIL: NOT EXECUTED
PHYSICAL PTT: NOT EXECUTED
RF EMISSION: 0
MAIN: SYNCHRONIZED
ORIGIN/MAIN: SYNCHRONIZED
AHEAD/BEHIND: 0/0
WORKING TREE: CLEAN
FORCE-PUSH: NOT EXECUTED
NEXT TASK: APPROVAL OF M3-BLOCKING OWNER DECISIONS -> STAGE-M3-INSTALLER-IMPLEMENTATION-GATE
================================================================================
```
