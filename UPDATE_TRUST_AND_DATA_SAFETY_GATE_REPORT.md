# RAPORT WYKONAWCZY BRAMKI BEZPIECZEŃSTWA ZAUFANIA, MANIFESTU I DANYCH
## UPDATE-TRUST-AND-DATA-SAFETY-GATE

**Status końcowy bramki:** `COMPLETED_SOFTWARE_VERIFIED`  
**Projekt:** SPLogbook (`v1.2.0`)  
**Data robocza projektu (deklarowana):** 2026-11-06  
**Rzeczywista data wykonania w środowisku:** 2026-11-07 (UTC+01:00 / CET)  
**Rozbieżność chronologiczna:** Odnotowano przesunięcie doby środowiska wykonawczego (2026-11-07) względem deklarowanej daty roboczej projektu (2026-11-06). Zgodnie z zasadami bezpieczeństwa, zegar systemowy ani znaczniki wcześniejszych commitów nie były modyfikowane.

---

## 1. Identyfikacja Repozytorium i Stanu Git

- **Aktywna gałąź robocza:** `repair/full-system-remediation`
- **Gałąź główna:** `main`
- **Śledzenie zdalne:** `origin/main` (`https://github.com/sp6ina/SPLogbook.git`)
- **HEAD commit:** `c95172e518a9245edd95574031e543cc78342150`
- **main commit:** `c95172e518a9245edd95574031e543cc78342150`
- **origin/main commit:** `c95172e518a9245edd95574031e543cc78342150`
- **Merge-base (main ... origin/main):** `c95172e518a9245edd95574031e543cc78342150`
- **Relacja ahead / behind:** `0 / 0` (pełna synchronizacja fast-forward po każdym commitowaniu)
- **Status drzewa roboczego przed zatwierdzeniem raportu:** Czyste (z wyjątkiem raportów dokumentacyjnych).

### Historia commitów wdrożonych w ramach bramki (wyłącznie fast-forward):
1. `bc3e806`: `fix(updater): require verified manifest for GUI update flow (UPDATE-TRUST-001)`
2. `12c3b22`: `fix(updater): plan file updates with fail-closed data protection (UPDATE-DATA-001)`
3. `da34b4c`: `fix(updater): preserve portable data during Windows and Linux update and rollback (UPDATE-DATA-001)`
4. `c95172e`: `test(updater): harden trust data rollback and health safety matrix`
5. `[pending]`: `docs(updater): close trust and data safety gate`

---

## 2. Integralność Plików Zależności Cargo

Zgodnie z bezwzględnymi zasadami bramki (Zasady 38-42), pliki manifestu zależności nie były modyfikowane, żadne zależności nie zostały dodane ani zaktualizowane, a wersja projektu pozostała nienaruszona.

- **Cargo.toml:**
  - SHA-1: `0cdf381165bfc3111fff15097007e4b235484ec8`
  - Git blob object hash: `d8e3acd8f0c7b3307b3733c6c4a9a6cc57cb17c4`
  - Status: **100% UNTOUCHED / ZERO MODIFICATIONS**
- **Cargo.lock:**
  - SHA-1: `52be66b2bc063c001d5c2e8556457fa2be04ff4f`
  - Git blob object hash: `c569097f3ef69c66ec59b74c29c1eb29efce69ae`
  - Status: **100% UNTOUCHED / ZERO MODIFICATIONS**

---

## 3. Pomiary Jakości Kodu i Weryfikacji Bezpieczeństwa

Wszystkie operacje weryfikacyjne zrealizowano bez ostrzeżeń, błędów i z poszanowaniem ograniczeń pamięciowych środowiska (flaga `-j 2`).

| Narzędzie / Komenda | Parametry | Wynik | Szczegóły |
|---|---|---|---|
| `cargo fmt` | `--all -- --check` | **PASS (0 błędów)** | 100% zgodności ze standardem formatowania Rust |
| `cargo clippy` | `--workspace --all-targets --all-features -j 2 -- -D warnings` | **PASS (0 ostrzeżeń, 0 błędów)** | Czysty stan clippy we wszystkich celach |
| `cargo test --lib` | `-j 2` | **PASS (1694 passed, 0 failed, 0 ignored)** | Pełen zestaw testów jednostkowych biblioteki (+17 nowych testów planera i klasyfikacji) |
| `cargo test --bin SPLogbook` | `-j 2` | **PASS (0 passed, 0 failed, 0 ignored)** | Target binarny bez testów jednostkowych |
| `cargo test --test security_guard` | `-j 2` | **PASS (64 passed, 0 failed, 0 ignored)** | 7 nowych testów bramki bezpieczeństwa zaufania i danych |
| `cargo test --test i18n_guard` | `-j 2` | **PASS (15 passed, 0 failed, 0 ignored)** | Wszystkie reguły i18n spełnione |
| `cargo test --test theme_guard` | `-j 2` | **PASS (1 passed, 0 failed, 0 ignored)** | Bezpieczeństwo palety kolorów |
| `cargo test --test cat_native_6a` | `-j 2` | **PASS (165 passed, 0 failed, 0 ignored)** | Architektura i odporność CAT |
| `cargo test --test cat_native_6b` | `-j 2` | **PASS (99 passed, 0 failed, 0 ignored)** | Multi-radio i bezpieczeństwo TX |
| `cargo test --test cat_native_7` | `-j 2` | **PASS (297 passed, 10 ignored HIL, 0 failed)** | 10 testów HIL poprawnie zignorowanych (`SAFETY GATE CLOSED`) |
| `cargo audit` | domyślne | **PASS (0 vulnerabilities, 0 unsound)** | 7 dozwolonych ostrzeżeń tranzytywnych (`.cargo/audit.toml`) |

**Łączne statystyki testów w projekcie:**
- **Zarejestrowane testy:** **2345** (+24 nowe testy w bieżącej bramce)
- **Aktywne testy programowe (wykonane):** **2335 passed**, **0 failed**
- **Testy ignorowane:** **10** (wyłącznie fizyczny HIL w `tests/cat_native_7.rs`)
- **Fizyczne HIL / PTT / RF:** **ZERO** (brak emisji RF, brak manipulacji sprzętem)

---

## 4. Status Celów Kanonicznych Bramki

### 4.1. UPDATE-TRUST-001 — Weryfikacja Manifest-First i Bramka Zaufania
**Status:** `FIXED_SOFTWARE_VERIFIED`

1. **Eliminacja ominięcia manifestu w GUI:**
   - Ścieżka `trigger_update_install` w `src/gui/app_services.rs` nie wywołuje już funkcji selekcji opartej na rozszerzeniach pliku (`select_asset_for_platform`).
   - Wprowadzono silnie typowaną strukturę `VerifiedReleaseAsset` (oraz `VerifiedReleaseManifest`), która powstaje **wyłącznie** w wyniku przejścia pełnej weryfikacji kryptograficznej i metadanych.
   - `LatestRelease::new` zawiera pole `pub verified_asset: VerifiedReleaseAsset`.
   - GUI przekazuje `release.verified_asset` bezpośrednio do `updater::install_update(&verified_asset)`.
   - Bez poprawnego manifestu i dopasowanego w nim assetu obiekt `LatestRelease` nie może powstać.
2. **Klucze odwołane i polityka fail-closed:**
   - Klucz `splogbook-release-2026-01` pozostaje na stałe na liście `REVOKED_KEY_IDS` oraz `REVOKED_PUBLIC_KEYS`.
   - Każdy manifest podpisany tym kluczem (nawet jeśli podpis Ed25519 jest matematycznie poprawny, jak w historycznym wydaniu v1.2.0) zostaje natychmiast odrzucony statusem fail-closed (`RevokedKey`).
   - Klucze nieznane, puste identyfikatory `key_id`, brak sygnatury lub brak manifestu bezwzględnie blokują proces.
3. **Ścisłe dopasowanie metadanych assetu:**
   - Selekcja assetu bazuje na enumach: `ReleasePlatform` (`windows`, `linux`), `ReleaseArchitecture` (`x86_64`) oraz `ReleasePackageType` (`zip` dla Windows, `tar.gz` dla Linux).
   - Wymuszone jest dokładne sprawdzenie nazwy pliku, typu paczki, oczekiwanego rozmiaru bajtowego oraz skrótu SHA-256. W przypadku pobrania innego rozmiaru lub niezgodnego SHA-256 proces kończy się błędem przed przejściem do ekstrakcji i stagingu.

---

### 4.2. UPDATE-DATA-001 — Ochrona Danych Użytkownika, Konfiguracji i Baz Portable
**Status:** `FIXED_SOFTWARE_VERIFIED`

1. **Reguła UNKNOWN DATA CLASSIFICATION → DO NOT OVERWRITE:**
   - Zaimplementowano klasyfikator ścieżek `classify_update_path(&Path) -> UpdateFileClass`.
   - Każdy plik niebędący jednoznacznie zaklasyfikowany jako bezpieczny składnik programu (`UpdateFileClass::ProgramFile`) jest traktowany jako podlegający bezwzględnej ochronie:
     - `UpdateFileClass::UserData` (np. `default_log.db`, pliki `*.adi`, `*.adif`, `*.db` w katalogu głównym lub podkatalogu `logs/`);
     - `UpdateFileClass::UserConfiguration` (np. `station_config.json`, `*.conf`, `*.toml`, `*.yaml`);
     - `UpdateFileClass::FirstInstallSeed` (`databases/serviceLOG.db`, `databases/callbook.db`);
     - `UpdateFileClass::ReferenceData` (`databases/clubs.csv`, dane prefiksów DXCC);
     - `UpdateFileClass::Unknown` (wszelkie nieznane pliki i katalogi) -> domyślna ochrona przed nadpisaniem.
2. **Ochrona baz danych w trybie Portable:**
   - Pliki `databases/serviceLOG.db` oraz `databases/callbook.db` zostały zaklasyfikowane jako `FirstInstallSeed`.
   - Jeśli plik bazy istnieje już w docelowej instalacji użytkownika (aktywny katalog aplikacji), aktualizator **nigdy go nie nadpisuje** ani nie modyfikuje. Nowy plik ze stagingu jest ignorowany (`preserve_files`).
   - Zastąpienie następuje wyłącznie wtedy, gdy docelowy plik w ogóle nie istnieje (pierwsza inicjalizacja programu).
3. **Likwidacja dzikich kart (wildcards) na Windows:**
   - W procedurze generowania skryptu PowerShell (`generate_powershell_update_script`) całkowicie usunięto kopiowanie zbiorcze `Copy-Item -Path "$staging\*"` oraz `$rollback\*`.
   - Zaimplementowano generator jawnych list: `$replaceFiles` (wyłącznie pliki programu, które istniały wcześniej i podlegają zastąpieniu) oraz `$createFiles` (nowe pliki programu dodane w danym wydaniu).
   - W przypadku rollbacku przywracane są wyłącznie pliki z `$replaceFiles`, a nowo utworzone pliki programu z `$createFiles` są bezpiecznie usuwane, pozostawiając dane użytkownika, bazy SQLite i konfigurację w stanie nienaruszonym.
4. **Zabezpieczenie procedury aktualizacji na Linux:**
   - W procedurze `install_via_tar_gz` zaimplementowano generowanie i weryfikację `UpdatePlan::generate(&staging_dir, base_dir)` oraz `plan.validate_safety("splogbook")`.
   - Backup tworzony jest wyłącznie dla plików zakwalifikowanych do podmiany (`plan.replace_files`).
   - Podmianie i ewentualnemu rollbackowi podlegają wyłącznie zatwierdzone składniki binarne i zasoby programu.
   - W procedurze unika się wszelkich uprawnień administratora (`sudo`, `pkexec`), a pliki otrzymują standardowe uprawnienia użytkownika uruchamiającego aplikację.
5. **Gwarancja atomowości i rollbacku (`apply_atomic_installation_with_options`):**
   - W przypadku symulacji błędu podmiany (rename failure) procedura rollbacku przywraca wyłącznie pliki programu, nie naruszając istniejących baz danych i plików konfiguracyjnych użytkownika.

---

## 5. Macierz Weryfikacji i Ochrony Platformowej

| Platforma | Format paczki | Główna binarka | Model archiwum | Zasada selekcji | Zabezpieczenie danych | Mechanizm rollbacku |
|---|---|---|---|---|---|---|
| **Windows** | ZIP (`.zip`) | `SPLogbook.exe` | Flat archive | Manifest (`windows` / `x86_64` / `zip` + SHA-256) | `UpdatePlan`: ochrona `*.db`, `station_config.json`, seeds `databases/*.db`; brak `$staging\*` | Wyłącznie `$replaceFiles` i usunięcie `$createFiles` |
| **Linux** | TAR.GZ (`.tar.gz`) | `splogbook` | Pojedynczy katalog (`SPLogbook-Linux-x86_64/`) | Manifest (`linux` / `x86_64` / `tar.gz` + SHA-256) | Spłaszczenie katalogu + `UpdatePlan::validate_safety`; pomijanie chronionych baz | Podmiana wyłącznie `replace_files` z katalogu `.backup_<pid>` |

---

## 6. Zakres Celowo Nieobjęty Bieżącą Bramką

Zgodnie z poleceniem i zakresem P0 zadania:
1. Nie zaimplementowano instalatora Windows (`UPDATE-WIN-INSTALLER` / MSI / WiX / InnoSetup) — realizacja zaplanowana w kolejnym dedykowanym zadaniu instalatora Windows.
2. Nie zaimplementowano instalatorów linuksowych (`UPDATE-LINUX-INSTALLER` / AppImage / DEB / RPM / Flatpak) — realizacja w kolejnym dedykowanym zadaniu instalatora Linux.
3. Nie wdrożono kamieni milowych M2 (rozproszona synchronizacja chmurowa) ani M3 (zaawansowane funkcje CAT multi-rig).
4. Nie wygenerowano nowego wydania (release) ani tagu Git v1.2.1.
5. Nie modyfikowano wydania v1.2.0 ani nie podpisywano ponownie jego zasobów (historyczne v1.2.0 poprawnie pozostaje zablokowane ze względu na odwołany klucz).

---

## 7. Rekomendacje dla Kolejnych Kroków

1. **Przejście do zadań instalatorów platformowych:**
   - `UPDATE-WIN-INSTALLER`: Opracowanie dedykowanego instalatora dla platformy Windows z zachowaniem izolacji danych użytkownika (`%APPDATA%` vs katalog instalacyjny).
   - `UPDATE-LINUX-INSTALLER`: Opracowanie natywnej paczki lub instalatora dla systemu Linux (np. `.tar.gz` z instalatorem desktop entry lub AppImage).
2. **Rotacja i publikacja nowego wydania v1.2.1:**
   - Wygenerowanie nowego wydania v1.2.1 z manifestem podpisanym aktywnym kluczem `splogbook-release-2026-02`.
   - Zapewnienie, że dystrybucja v1.2.1 korzysta w pełni z wdrożonej bramki `VerifiedReleaseManifest`.
3. **Utrzymanie testów regresyjnych:**
   - Zestaw 64 testów w `tests/security_guard.rs` oraz 1694 testów jednostkowych stanowi nienaruszalną bazę ochronną dla wszelkich przyszłych modyfikacji modułu `updater.rs`.
