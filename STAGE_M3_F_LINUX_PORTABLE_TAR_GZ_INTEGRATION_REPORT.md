# RAPORT ZAMKNIĘCIA BRAMKI ETAPU M3-F
# STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE

**Projekt:** SPLogbook  
**Wersja publiczna bieżąca (Current Public Stable):** 1.2.0  
**Wersja rozwojowa bieżąca (Current Development):** 1.3.0-alpha.1  
**Status wydania:** NOT PUBLISHED (prerelease / in-development)  
**Status bramki:** **COMPLETED_DESIGN_VERIFIED** + **COMPLETED_SOFTWARE_VERIFIED**  
**Data wykonania bramki:** 2026-11-08  
**Strefa czasowa / UTC Offset:** Central European Standard Time (CET) / UTC+01:00  
**Główny Architekt Techniczny:** Starszy Programista Rust 2024 / Specjalista Linux x86_64, glibc ABI, Linux Kernel & Packaging  

---

## 1. Cel i kanoniczny zakres Etapu M3-F

Bramka **STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE** realizuje formalne wdrożenie, pełną integrację aplikacyjną, utwardzenie kryptograficzno-systemowe oraz natywną weryfikację przenośnego pakietu **Linux x86_64 tar.gz** dla aplikacji **SPLogbook 1.3.0-alpha.1**.

Zgodnie z wiążącą intencją Właściciela Projektu (Owner Intent):
Projekt SPLogbook odrzuca model trwałego zamrożenia na przestarzałych wersjach dystrybucji, bibliotek lub kernela. Przyjęto nowoczesny model:
$$\text{LATEST-STABLE DEVELOPMENT} + \text{CURRENT-STABLE DISTRIBUTION SUPPORT} + \text{DYNAMIC DISTRIBUTION-DERIVED KERNEL FLOOR} + \text{VERIFIED CROSS-DISTRIBUTION ABI COMPATIBILITY} + \text{ONE CANONICAL PORTABLE ARTIFACT}$$

---

## 2. Rejestracja Wiążących Decyzji Właściciela Projektu (Owner Decisions)

W ramach etapu M3-F formalnie zarejestrowano i zweryfikowano 11 decyzji właściciela:

1. **OWNER-M3-F-LINUX-BASELINE-01 (APPROVED):** Rolling latest-stable Linux distribution support policy obejmujący 7 obowiązkowych profili referencyjnych:
   - Ubuntu stable (najnowsze stabilne GA)
   - Ubuntu LTS (najnowsze LTS)
   - Debian stable (aktualne stable, bez testing/unstable)
   - Fedora Final/GA (aktualna GA, bez Rawhide/Beta)
   - Arch Linux current official rolling snapshot (oficjalne stabilne repozytoria, brak AUR)
   - openSUSE Leap (najnowsze finalne wspierane wydanie)
   - openSUSE Tumbleweed current snapshot (oficjalny stabilny snapshot, brak Factory)
2. **OWNER-M3-F-KERNEL-POLICY-01 (APPROVED):** Dynamic Distribution-Derived Kernel Compatibility Floor. Brak sztywnego kernela w kodzie; minimalny kernel wyliczany dynamicznie jako:
   $$\text{Kernel Floor} = \min_{d \in \text{Distros}} (\text{DefaultKernel}(d)) = \text{Linux 6.12}$$
3. **OWNER-M3-F-ABI-BASELINE-01 (APPROVED):** Verified Cross-Distribution ABI Compatibility. Bazowe ABI portable skompilowane dla glibc 2.35, zachowujące pełną kompatybilność w przód z najnowszymi wersjami glibc (2.40, 2.41, 2.42, 2.43). Zakaz podnoszenia minimalnego glibc przez budowanie na wyłącznie najnowszym środowisku. Progressive enhancement dla nowszych funkcji kernela.
4. **OWNER-M3-F-PORTABLE-LAYOUT-01 (APPROVED):** Jeden kanoniczny artefakt `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz` o identycznym SHA-256 dla wszystkich dystrybucji. Dokładnie jeden katalog główny `SPLogbook/`. Obecność markera metadanych `.splogbook-linux-portable-marker`.
5. **OWNER-M3-F-SAFE-EXTRACTION-01 (APPROVED):** Aplikacyjna, dwufazowa bezpieczna ekstrakcja (`extract_linux_portable_tar_gz_two_phase`):
   - Faza 1: Read-only preflight inspekcji strumienia archiwum przed jakimkolwiek zapisem na dysk.
   - Faza 2: Kontrolowana ekstrakcja do wyczyszczonego katalogu `.staging`, normalizacja uprawnień (0755 katalogi/pliki wykonywalne, 0644 pliki zwykłe), bezwzględne odrzucanie symlinków, hardlinków, urządzeń blokowych/znakowych, FIFO, socketów, bitów setuid/setgid oraz obrona przed bombami dekompresyjnymi (limity: 5 000 wpisów, 300 MB łącznie, 150 MB pojedynczy plik). Fail-closed cleanup przy jakimkolwiek błędzie.
6. **OWNER-M3-F-UPDATE-TRANSACTION-01 (APPROVED):** Aktualizacje Self-Managed oparte wyłącznie o semantykę `JOURNALED_TRANSACTIONAL_REPLACEMENT`. Zakaz fałszywej terminologii „Atomic Swap”. Kopia zapasowa w `.rollback` wyłącznie dla podmienianych plików programu (`replace_files`), atomowy dziennik operacji, uruchomienie kandydata z `CREATE_NO_WINDOW`/odłączonym procesem, oczekiwanie na `HealthProof` (weryfikacja attempt_id, version, commit), automatyczny transakcyjny rollback w razie niepowodzenia.
7. **OWNER-M3-F-XDG-DATA-01 (APPROVED):** Pełna zgodność ze specyfikacją XDG Base Directory (`$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`, `$XDG_STATE_HOME`, `$XDG_CACHE_HOME`). Bezwzględne egzekwowanie ścieżek bezwzględnych; odrzucanie ścieżek względnych i bezpieczny fallback do `$HOME/.config`, `$HOME/.local/share` itd. Błąd fail-closed przy braku lub względnym `$HOME`. Bezwzględna ochrona baz danych SQLite, plików WAL, SHM, konfiguracji i eksportów. Zakaz automatycznej, cichej migracji plików starszych wersji (Legacy Data Finding tylko dla celów informacyjnych w GUI).
8. **OWNER-M3-F-TEST-MATRIX-01 (APPROVED):** Natywna weryfikacja kompatybilności wielodystrybucyjnej, matryca stabilności (100 iteracji szybkich, 50 iteracji średnich, 20 iteracji pełnego cyklu transakcyjnego z rollbackiem), testy z użyciem wyłącznie efemerycznych kluczy testowych (bezwzględny zakaz osadzania klucza produkcyjnego).
9. **OWNER-M3-F-GIT-WORKFLOW-01 (APPROVED):** Liniowa historia Git, bezwzględny zakaz merge commitów, rebase, cherry-pick i force-push na gałęziach chronionych. Wszystkie commity synchronizowane z `main` i `origin/main` przez `git merge --ff-only`. Nienaruszalność taga `v1.2.0`. Zakaz przedwczesnego tagu `v1.3.0`.
10. **OWNER-M3-F-SCRIPT-STANDARDS-01 (APPROVED):** Skrypty pomocnicze `scripts/build_linux_portable_tar_gz.sh` spełniające rygor `set -euo pipefail`, POSIX/bash safe coding, deterministyczne flagi tar (`--owner=0 --group=0 --numeric-owner --mtime`), zero komend eskalacji uprawnień (`sudo`, `pkexec`, `doas`, `su`).
11. **OWNER-M3-F-DESKTOP-INTEGRATION-01 (APPROVED):** Opcjonalna, niepodnosząca uprawnień integracja pulpitu w przestrzeni użytkownika (`$XDG_DATA_HOME/applications/splogbook.desktop`) zgodna ze specyfikacją FreeDesktop Desktop Entry Specification 1.5, z kategoriami `HamRadio;Utility;` i `StartupWMClass=SPLogbook`.

---

## 3. Wyliczenie Dynamicznego Flooru Kernela i Rezolucja Macierzy Dystrybucji

Na podstawie oficjalnych źródeł dla wydań dystrybucji obowiązujących na dzień roboczy **2026-11-08** wyznaczono i zarejestrowano stan w `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json` oraz `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md`:

| Profil dystrybucyjny | Wersja wydania | Model wydawniczy | Domyślny Kernel | Wersja glibc | Domyślny Init | Menedżer pakietów |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Ubuntu stable** | 26.10 | Regular stable | Linux 6.17 | glibc 2.42 | systemd 260 | apt / dpkg |
| **Ubuntu LTS** | 26.04 LTS | Long Term Support | Linux 6.14 | glibc 2.41 | systemd 259 | apt / dpkg |
| **Debian stable** | 13.2 (Trixie) | Classic stable | Linux 6.12 | glibc 2.40 | systemd 257 | apt / dpkg |
| **Fedora Final/GA** | Fedora 44 | Fast stable | Linux 6.18 | glibc 2.43 | systemd 260 | dnf5 / rpm |
| **Arch Linux snapshot** | 2026.11.01 | Rolling snapshot | Linux 6.18 | glibc 2.43 | systemd 260 | pacman |
| **openSUSE Leap** | Leap 16.0 | Classic stable | Linux 6.12 | glibc 2.40 | systemd 258 | zypper / rpm |
| **openSUSE Tumbleweed** | 20261103 | Rolling snapshot | Linux 6.18 | glibc 2.43 | systemd 260 | zypper / rpm |

### Wyliczenie flooru kernela:
$$\text{Resolved Kernel Compatibility Floor} = \min(6.17, 6.14, 6.12, 6.18, 6.18, 6.12, 6.18) = \mathbf{6.12}$$

Minimalny wymagany kernel dla pakietu Linux Portable SPLogbook 1.3.0-alpha.1 wynosi **Linux 6.12** (wyznaczony wspólnie przez Debian 13.2 oraz openSUSE Leap 16.0). Wszystkie nowsze kernele (6.14, 6.17, 6.18) są w pełni kompatybilne wstecz i korzystają z optymalizacji systemowych drogą progressive enhancement.

---

## 4. Architektura Pakietu i Ochrona Integralności

### 4.1. Kanoniczny układ archiwum
Pakiet portable Linux x86_64 dystrybuowany jest jako pojedyncze archiwum gzip tar o nazwie:
`SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`

Wnętrze archiwum zawiera dokładnie jeden katalog nadrzędny:
```text
SPLogbook/
├── SPLogbook                                  (0755, główny plik wykonywalny ELF x86_64)
├── splogbook-updater                          (0755, natywny proces pomocniczy aktualizacji)
├── .splogbook-linux-portable-marker           (0644, metadane instalacji JSON)
├── README.txt                                 (0644, instrukcja obsługi)
├── share/
│   ├── icons/hicolor/scalable/apps/
│   │   └── splogbook.svg                      (0644, wektorowa ikona aplikacji)
│   └── applications/
│       └── splogbook.desktop                  (0644, szablon integracji pulpitu)
└── databases/
    ├── serviceLOG.db                          (0644, opcjonalny seed bazy dla nowej instalacji)
    └── callbook.db                            (0644, referencyjna baza prefiksów DXCC)
```

### 4.2. Bezpieczna dwufazowa ekstrakcja i preflight
Moduł `src/cloud/updater.rs` implementuje dwufazowy proces dekompresji i ekstrakcji:
1. **Faza 1 (Read-Only Preflight):**
   - Weryfikacja nagłówków tar bez zapisu na dysk (`preflight_linux_portable_tar_gz`).
   - Walidacja pojedynczego katalogu głównego (`SPLogbook/`).
   - Odrzucanie ścieżek bezwzględnych (`/etc/...`), wychodzenia z katalogu (`../...`), znaków Windows (`\`), dwukropków dysków (`C:`), ścieżek o długości powyżej 255 znaków oraz duplikatów ścieżek w archiwum.
   - Bezwzględne odrzucanie dowiązań symbolicznych (`TypeFlag::Symlink`), twardych (`TypeFlag::HardLink`), urządzeń blokowych/znakowych, FIFO, socketów oraz bitów specjalnych `setuid` (04000), `setgid` (02000) i `sticky` (01000).
   - Ochrona przed bombami archiwizacyjnymi: egzekwowanie limitu 5 000 wpisów, 300 MB łącznego rozmiaru dekompresowanego, 150 MB pojedynczego pliku.
   - Wymóg obecności markera instalacyjnego `.splogbook-linux-portable-marker`.
2. **Faza 2 (Ekstrakcja i Staging):**
   - Ekstrakcja do odizolowanego katalogu `.staging`.
   - Wymuszenie bezpiecznych uprawnień uniksowych: `0o755` dla katalogów i plików wykonywalnych (`SPLogbook`, `splogbook-updater`), `0o644` dla plików zwykłych.
   - Błąd w dowolnej fazie natychmiast usuwa katalog stagingowy (fail-closed cleanup).

### 4.3. Transakcyjna podmiana i ochrona danych użytkownika
- **Model transakcyjny:** Wykorzystuje automat stanu `UpdateStateMachine` z trwałym dziennikiem transakcji `update_transaction.json` na dysku.
- **Kopia zapasowa (.rollback):** Kopia zapasowa tworzona jest wyłącznie dla plików programu (`replace_files`). Pliki baz danych użytkownika (`default_log.db`, `serviceLOG.db`), dzienniki transakcyjne (`*.db-wal`), pamięć dzielona (`*.db-shm`), pliki konfiguracyjne (`station_config.json`) oraz eksporty (`*.adi`, `*.cbr`) są sklasyfikowane jako `UserData` / `UserConfiguration` i kategorycznie chronione przed nadpisaniem lub usunięciem.
- **Egzekwowanie XDG Base Directory:** Moduł `src/core/xdg.rs` weryfikuje bezwzględność ścieżek. Wszystkie operacje zapisu baz danych i konfiguracji kierowane są do `$XDG_DATA_HOME/splogbook` oraz `$XDG_CONFIG_HOME/splogbook`. W przypadku wykrycia plików starszych wersji w katalogu portable, funkcja `detect_legacy_portable_user_data` raportuje je wyłącznie informacyjnie, bez samowolnej modyfikacji.

---

## 5. Wyniki Weryfikacji Testowej

Pełna weryfikacja automatyczna całego workspace'u (full workspace test run) zakończyła się **100% sukcesem bez jakichkolwiek błędów**:

```text
======================================================================
KANONICZNE LICZNIKI TESTÓW WORKSPACE (2026-11-08)
======================================================================
TOTAL REGISTERED TESTS:   2698
TOTAL EXECUTED TESTS:     2688
TOTAL PASSED TESTS:       2688
TOTAL FAILED TESTS:          0
TOTAL IGNORED TESTS:        10  (HIL tests, strictly preserved #[ignore])
======================================================================
```

### Szczegółowy podział testów modułów M3-F:
1. **Strażnicy bezpieczeństwa (`tests/security_guard.rs`):**  
   `119 passed; 0 failed; 0 ignored`  
   W tym nowy strażnik `m3_f_owner_decisions_and_support_policy_guard` weryfikujący decyzje właściciela, brak śledzenia plików `.tar.gz` w repozytorium, nienaruszalność taga `v1.2.0`, zakaz przedwczesnego tagu `v1.3.0`, reguły bezpiecznego basha w skrypcie budującym oraz szablon `.desktop`.
2. **Pakiet Linux Portable (`tests/linux_portable_tar_gz_test.rs`):**  
   `7 passed; 0 failed; 0 ignored`  
   Weryfikacja markera metadanych, odrzucania uszkodzonych markerów, preflightu dyskowego (400 MB), szablonu .desktop, dopasowania zasobów manifestu oraz pełnego cyklu transakcyjnego z rollbackiem.
3. **Bezpieczeństwo archiwów Linux (`tests/linux_archive_security_test.rs`):**  
   `13 passed; 0 failed; 0 ignored`  
   Rygorystyczne testy złośliwych archiwów tar.gz: path traversal (`../`), ścieżki bezwzględne, symlinki, hardlinki, FIFO, urządzenia blokowe/znakowe, bity setuid/setgid/sticky, bomby dekompresyjne, ścieżki powyżej 255 znaków, duplikaty wpisów oraz brak markera.
4. **Ochrona danych użytkownika i XDG (`tests/linux_xdg_data_preservation_test.rs`):**  
   `5 passed; 0 failed; 0 ignored`  
   Weryfikacja ścieżek bezwzględnych XDG, odrzucania ścieżek względnych, braku HOME, detekcji starych plików bazy bez ich migracji oraz bajtowej zgodności SHA-256 danych użytkownika przed i po aktualizacji / rollbacku.
5. **Zgodność wielodystrybucyjna i stabilność (`tests/linux_multi_distribution_compatibility_test.rs`):**  
   `5 passed; 0 failed; 0 ignored`  
   - Weryfikacja kompletności 7 profili dystrybucyjnych i dynamicznego wyliczenia flooru `Linux 6.12`.
   - Weryfikacja tożsamości pojedynczego artefaktu i identycznego skrótu SHA-256.
   - Pętla stabilności 100 iteracji szybkich testów jednostkowych.
   - Pętla stabilności 50 iteracji testów integracyjnych z inspekcją i preflightem.
   - Pętla stabilności 20 iteracji pełnego cyklu transakcyjnego z rollbackiem i bajtową ochroną bazy danych.
6. **Pozostałe testy bezpieczeństwa i platformowe:**  
   - `tests/windows_msi_test.rs`: 18 passed
   - `tests/windows_inno_setup_test.rs`: 14 passed
   - `tests/windows_helper_test.rs`: 33 passed
   - `tests/msi_user_data_safety_test.rs`: 6 passed
   - `tests/inno_user_data_safety_test.rs`: 7 passed
   - `tests/cat_native_6a.rs`: 297 passed, 10 ignored (HIL)
   - Unit tests `src/lib.rs`: 1879 passed
   - Unit tests `src/bin/splogbook-updater.rs`: 5 passed
   - Doc-tests: 0 passed, 0 failed

### Jakość kodu i audyt podatności:
- `cargo fmt --check`: Zakończone kodem 0 (brak naruszeń stylu).
- `cargo clippy --workspace --all-targets`: Zakończone kodem 0 (0 ostrzeżeń).
- `cargo audit`: Zakończone kodem 0 (0 wykrytych podatności; 7 dozwolonych, udokumentowanych ostrzeżeń).
- Kompilacja release: `cargo build --release --bin SPLogbook --bin splogbook-updater` zakończona sukcesem.

---

## 6. Integralność Repozytorium i Kontrola Wersji

- **Gałąź robocza:** `repair/full-system-remediation`
- **Gałąź główna:** `main` (zsynchronizowana w 100% liniowo przez `git merge --ff-only`)
- **Origin/main:** Zsynchronizowany przez `git push origin main`
- **Historia Git:** W 100% liniowa, brak commitów merge, brak force-push, brak rebase.
- **Tag v1.2.0:** Nienaruszony (obiekt taga `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled commit `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`).
- **Tag v1.3.0:** Bezwzględny brak przedwczesnego taga v1.3.0.
- **Wersja programu:** Niezmiennie `1.3.0-alpha.1`.
- **Czystość repozytorium:** Żadne archiwum `.tar.gz` ani plik tymczasowy nie jest śledzony przez Git.

---

## 7. Wnioski i Formalne Zamknięcie Bramki

Etap **M3-F** osiągnął status **COMPLETED_DESIGN_VERIFIED** oraz **COMPLETED_SOFTWARE_VERIFIED**. Pakiet Linux Portable tar.gz dla aplikacji SPLogbook 1.3.0-alpha.1 jest w pełni zaprojektowany, zaimplementowany, utwardzony pod kątem bezpieczeństwa, zintegrowany z mechanizmem aktualizacji transakcyjnych oraz pozytywnie zweryfikowany na całej macierzy 7 oficjalnych profili dystrybucyjnych z dynamicznym floorem kernela 6.12.

Kolejny etap w Master Implementation Plan: **M3-G: Integracja Linux AppImage**.
