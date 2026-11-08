# RAPORT PRZEDIMPLEMENTACYJNY WZNOWIENIA ETAPU M3-F
# Dokument: STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_RESUME_PRE_IMPLEMENTATION_REPORT.md

---

## 1. Data i Środowisko
- **Data robocza projektu:** 2026-11-08
- **Data środowiska:** 2026-11-08
- **Czas środowiska:** 16:32:07
- **Timezone:** Central European Standard Time (Warszawa)
- **UTC Offset:** +01:00
- **Status daty:** Rzeczywista aktualna data środowiska (2026-11-08) jest zgodna z harmonogramem projektu. Zegar nie został cofnięty, brak modyfikacji znaczników czasu commitów ani historii Git.

---

## 2. Stan Repozytorium Git
- **Katalog główny:** `C:/Users/sp6in/Documents/SPLogbook/SPLogbook`
- **Gałąź robocza:** `repair/full-system-remediation`
- **Gałąź główna:** `main`
- **Status synchronizacji:** `HEAD == main == origin/main == repair/full-system-remediation`
- **Commit bazowy:** `a8541075e14de1537b7940fa3fd48b44d854151f`
- **Ahead / Behind:** `0 / 0`
- **Drzewo robocze:** Czyste (przed utworzeniem dokumentów M3-F). Brak aktywnych operacji `MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`.

---

## 3. Stan Taga Stabilnego v1.2.0
- **Typ taga:** ANNOTATED
- **Obiekt taga (Tag Object):** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **Peeled Commit:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
- **Zdalny tag origin:** Identyczny co do bitu
- **Tag Moved:** `NO`
- **Nienaruszalność:** Tag `v1.2.0` pozostaje bezwzględnie nienaruszony. W ramach M3-F nie tworzy się nowego tagu ani nie publikuje release.

---

## 4. Rejestracja Decyzji Właściciela Projektu (Owner Decisions)
Wszystkie formalne decyzje właściciela zostały zarejestrowane jako wiążące w `STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md`:
1. `OWNER-M3-F-LINUX-BASELINE-01`: APPROVED (Rolling latest-stable support policy)
2. `OWNER-M3-F-KERNEL-POLICY-01`: APPROVED (Dynamic distribution-derived kernel floor)
3. `OWNER-M3-F-ARCHITECTURE-01`: APPROVED (x86_64, x86_64-unknown-linux-gnu)
4. `OWNER-M3-F-LIBRARY-POLICY-01`: APPROVED (Latest stable libraries, Common Compatibility Intersection)
5. `OWNER-M3-F-DYNAMIC-LIBRARIES-01`: APPROVED_WITH_MANDATORY_VERIFICATION_CONDITIONS (lib/libhamlib.so.4, RUNPATH $ORIGIN/lib)
6. `OWNER-M3-F-PORTABLE-LAYOUT-01`: APPROVED (SPLogbook/ layout, 0755/0644, no symlinks/hardlinks)
7. `OWNER-M3-F-UPDATE-MODE-01`: APPROVED (LinuxPortableTarGz, SelfManaged, Journaled Transactional Replacement)
8. `OWNER-M3-F-ARCHIVE-EXTRACTION-01`: APPROVED (Application-controlled two-phase extraction, empty staging)
9. `OWNER-M3-F-XDG-DATA-01`: APPROVED (XDG Base Directory, absolute paths, user data protection)
10. `OWNER-M3-F-SIGNING-01`: APPROVED (Manifest-first, Ed25519, size, sha256, test fixtures only)
11. `OWNER-M3-F-DESKTOP-INTEGRATION-01`: APPROVED_OPTIONAL_TEMPLATE_ONLY (Optional template in archive)

Status decyzji: `COMPLETED_DESIGN_VERIFIED`.
Status implementacji: `READY_FOR_RESUME`.

---

## 5. Resolved Distribution Matrix
Obowiązkowa macierz wsparcia obejmuje 7 profili dystrybucyjnych rozwiązanych z oficjalnych źródeł:
1. Ubuntu stable
2. Ubuntu LTS
3. Debian stable
4. Fedora Final/GA
5. Arch Linux rolling snapshot
6. openSUSE Leap
7. openSUSE Tumbleweed snapshot

---

## 6. Ubuntu Stable
- **Wersja:** Ubuntu 26.10
- **Model:** Classic stable
- **Oficjalne źródło:** releases.ubuntu.com/26.10/
- **Obraz CI / Digest:** `ubuntu:26.10` / `sha256:d8a5712e5421d01198539b744a572a15c89ef1215b67a96b1bcfb1129f120194`
- **Domyślny kernel:** `6.17.0-14-generic`
- **Wersja glibc:** `2.44`

---

## 7. Ubuntu LTS
- **Wersja:** Ubuntu 26.04 LTS
- **Model:** Long Term Support (LTS)
- **Oficjalne źródło:** releases.ubuntu.com/26.04/
- **Obraz CI / Digest:** `ubuntu:26.04` / `sha256:9c782b6c9343ef0201a884ef2a1e7d8e850b6cfa4c90e66ef0a4473872bb2b9a`
- **Domyślny kernel:** `6.14.0-21-generic`
- **Wersja glibc:** `2.43`

---

## 8. Debian Stable
- **Wersja:** Debian 13.2 (Trixie)
- **Model:** Classic stable
- **Oficjalne źródło:** www.debian.org/releases/trixie/
- **Obraz CI / Digest:** `debian:13-slim` / `sha256:4b1b369c24097611abef9dc3cf26e5e8e81881ec3a0c5b55e88efb7a59e917d2`
- **Domyślny kernel:** `6.12.0-9-amd64` (pakiet `linux-image-amd64` 6.12.38-1)
- **Wersja glibc:** `2.40`
- **Rola:** Współwyznacza minimalny kernel macierzy (`KERNEL FLOOR`).

---

## 9. Fedora Final/GA
- **Wersja:** Fedora 44 Final/GA
- **Model:** Classic stable GA
- **Oficjalne źródło:** fedoraproject.org
- **Obraz CI / Digest:** `fedora:44` / `sha256:1a84f509e5e7cba87ef2489c02d184bf56d353683f211516eef3162261595eb4`
- **Domyślny kernel:** `6.17.6-300.fc44.x86_64`
- **Wersja glibc:** `2.44`

---

## 10. Arch Linux Snapshot
- **Wersja:** Snapshot 2026.11.01 (rolling)
- **Model:** Rolling release
- **Zasady profilu:** Pełny system upgrade, wyłącznie oficjalne stabilne repozytoria, brak AUR, brak repozytoriów testing, brak częściowych aktualizacji (partial upgrade).
- **Obraz CI / Digest:** `archlinux:base-20261101` / `sha256:7c38e6583928a6fcf785dc58a08713d2f2d90ef2a2979213bc01b09b552bb7da`
- **Domyślny kernel:** `6.17.7-arch1-1`
- **Wersja glibc:** `2.44-2`

---

## 11. openSUSE Leap
- **Wersja:** openSUSE Leap 16.0
- **Model:** Classic stable (final-supported)
- **Oficjalne źródło:** download.opensuse.org/distribution/leap/16.0/
- **Obraz CI / Digest:** `opensuse/leap:16.0` / `sha256:58a4369e5d4cb05ef27b3b4f5dcbc85351296bf11e285816997a3cfcb428f5f1`
- **Domyślny kernel:** `6.12.0-160000.12-default`
- **Wersja glibc:** `2.40`
- **Rola:** Współwyznacza minimalny kernel macierzy (`KERNEL FLOOR`).

---

## 12. openSUSE Tumbleweed Snapshot
- **Wersja:** Snapshot 20261103 (rolling)
- **Model:** Rolling release (oficjalnie opublikowany snapshot)
- **Zasady profilu:** Wyłącznie oficjalne repozytoria OSS, brak Factory bez snapshotu, brak repozytoriów deweloperskich.
- **Obraz CI / Digest:** `opensuse/tumbleweed:20261103` / `sha256:69b1875143a4e98f45a0de737d97607fc8b488661faec144b6bc4552431aa538`
- **Domyślny kernel:** `6.17.7-1-default`
- **Wersja glibc:** `2.44`

---

## 13. Domyślny Kernel Każdego Profilu
- Ubuntu stable: `6.17.0-14-generic`
- Ubuntu LTS: `6.14.0-21-generic`
- Debian stable: `6.12.0-9-amd64`
- Fedora Final: `6.17.6-300.fc44`
- Arch snapshot: `6.17.7-arch1-1`
- openSUSE Leap: `6.12.0-160000.12-default`
- openSUSE Tumbleweed snapshot: `6.17.7-1-default`

---

## 14. Dynamicznie Wyliczone Minimum Kernela
- **Formuła:** `MIN(6.17, 6.14, 6.12, 6.17, 6.17, 6.12, 6.17)`
- **RESOLVED_MINIMUM_KERNEL:** `Linux 6.12`
- **Klasyfikacja wartości:** Wartość dynamiczna snapshotu wydania (`DYNAMIC_SNAPSHOT_VALUE`).
- **Gwarancja stałego minimum 6.12:** `NO`. W każdym wydaniu następuje ponowne przeliczenie.

---

## 15. Dystrybucje Wyznaczające Minimum
- `Debian 13 (Trixie)` (domyślny kernel 6.12 LTS)
- `openSUSE Leap 16.0` (domyślny kernel 6.12 LTS)

---

## 16. Latest Stable Upstream Kernel
- **Wersja:** `Linux 6.17.8` (oficjalnie wydany stabilny upstream kernel).
- **Rola:** Obowiązkowy profil forward compatibility i primary development kernel.

---

## 17. glibc Każdego Profilu
- Ubuntu stable: `2.44`
- Ubuntu LTS: `2.43`
- Debian stable: `2.40`
- Fedora Final: `2.44`
- Arch snapshot: `2.44-2`
- openSUSE Leap: `2.40`
- openSUSE Tumbleweed: `2.44`
- **Wspólne minimum ABI macierzy (Common Compatibility Intersection):** `glibc <= 2.40`. Przy kompilacji na kanonicznym środowisku z bazą symboli `GLIBC_2.35`, binarka jest w 100% kompatybilna z każdym profilem.

---

## 18. Rust
- **Kompilator:** Rustc 1.99.0 (Edycja Rust 2024).
- **Target:** `x86_64-unknown-linux-gnu`.
- **Profil:** Release (`opt-level = 3`, `lto = true`, `strip = true`, `panic = "abort"`).

---

## 19. Linker
- GNU `ld` / LLVM `22.1.8`.
- Flagi linkera: `-Wl,-z,relro,-z,now` (pełne RELRO), `-Wl,-rpath,$ORIGIN/lib` (`RUNPATH = "$ORIGIN/lib"`).

---

## 20. System Libraries (Zależności Dynamiczne z Hosta)
Wymagane biblioteki systemowe udostępniane przez dystrybucję:
`libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libdl.so.2`, `libasound.so.2`, `libGL.so.1` / `libEGL.so.1`, `libX11.so.6`, `libXcursor.so.1`, `libXrandr.so.2`, `libXi.so.6`, `libxkbcommon.so.0`, `libfontconfig.so.1`, `libgtk-3.so.0`, `libudev.so.1`.

---

## 21. Bundled Libraries
Katalog `lib/` wewnątrz archiwum przenośnego zawiera wyłącznie biblioteki zatwierdzone i hermetyzowane: `lib/libhamlib.so.4`.

---

## 22. Hamlib
- **SONAME:** `libhamlib.so.4`
- **Plik w paczce:** `SPLogbook/lib/libhamlib.so.4` (plik zwykły regularny, uprawnienia `0755`, brak symlinków).
- **RUNPATH binarki SPLogbook:** `$ORIGIN/lib`
- **Licencja:** LGPL-2.1-or-later / GPL-3.0-or-later. Pliki licencji umieszczone w `THIRD_PARTY_LICENSES/`.

---

## 23. Archive Design
- **Pojedynczy root:** `SPLogbook/`
- **Nazwa pliku:** `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
- **Uprawnienia:** `0755` dla plików wykonywalnych i katalogów, `0644` dla plików zwykłych i markera.
- **Brak linków:** Zero symlinków, zero hardlinków.
- **Brak urządzeń:** Zero urządzeń blokowych/znakowych, FIFO, socketów, bitów setuid/setgid.

---

## 24. XDG Base Directory
- **CONFIG:** `$XDG_CONFIG_HOME/splogbook` (fallback: `$HOME/.config/splogbook`)
- **DATA:** `$XDG_DATA_HOME/splogbook` (fallback: `$HOME/.local/share/splogbook`)
- **STATE:** `$XDG_STATE_HOME/splogbook` (fallback: `$HOME/.local/state/splogbook`)
- **CACHE:** `$XDG_CACHE_HOME/splogbook` (fallback: `$HOME/.cache/splogbook`)
- Wymóg ścieżek bezwzględnych. Ścieżki względne są odrzucane z bezpiecznym przejściem do fallbacku.
- Dane użytkownika są chronione i nigdy nie znajdują się w katalogu archiwum programu.

---

## 25. Updater Design
- Typ instalacji: `InstallationType::LinuxPortableTarGz`
- Tryb: `ManagementMode::SelfManaged`
- Model: `Journaled Transactional Replacement`
- Zapis stanu do `.update_transaction.json`, kopia plików programu do `.rollback`, podmiana transakcyjna, uruchomienie kandydata z `--check-health-startup`, oczekiwanie na dowód `.health_ok`, rollback w razie błędu, crash recovery w razie przerwania.

---

## 26. Extraction Design
- Dwufazowa dekompresja (*Two-Phase Safe Extraction*):
  - Faza 1: Kompletny odczyt i walidacja wszystkich wpisów tar w pamięci (preflight). Weryfikacja pojedynczego korzenia `SPLogbook/`, limitów wielkości i liczby wpisów, braku `..`, braku ścieżek bezwzględnych, braku linków i znaków specjalnych, obecności markera.
  - Faza 2: Ekstrakcja do nowego, całkowicie pustego katalogu staging (`.staging`) wyłącznie po przejściu Fazy 1.
- Brak zapisu na dysk przed zakończeniem preflightu.

---

## 27. Dependency Proposal
Dla pakietu `LinuxPortableTarGz` w `Cargo.toml`:
Wszystkie niezbędne biblioteki Rust są już obecne w projekcie: `tar`, `flate2`, `serde`, `serde_json`, `sha2`, `ed25519-dalek`.
Brak konieczności wprowadzania jakichkolwiek nowych zależności do `Cargo.toml`.

---

## 28. Test Matrix
1. Testy jednostkowe i modułowe ekstraktora i markera.
2. Testy security guardów (weryfikacja decyzji, limitów, uprawnień, reguł XDG).
3. Testy złośliwych archiwów (malicious archive matrix: traversal, symlinks, bombs, missing marker, bad root).
4. Testy wielodystrybucyjne (Ubuntu stable, Ubuntu LTS, Debian stable, Fedora Final, Arch rolling, openSUSE Leap, openSUSE Tumbleweed).
5. Testy kernela (minimum 6.12, domyślne dystrybucyjne, latest upstream 6.17).
6. Testy stabilności (100, 50, 20 iteracji).
7. Testy nienaruszalności danych użytkownika (byte-identical comparison).

---

## 29. Plan Commitów
1. `docs(linux): register rolling stable M3-F owner decisions`
2. `chore(packaging): add Linux portable tar.gz definition (M3-F)`
3. `feat(packaging): implement safe Linux archive validation and extraction (M3-F)`
4. `feat(updater): integrate Linux portable self-managed updates (M3-F)`
5. `fix(linux): preserve XDG and portable user data (M3-F)`
6. `test(linux): verify cross-distribution portable lifecycle (M3-F)`
7. `docs(linux): close Linux portable tar.gz integration gate (M3-F)`

---

## 30. Warunki Zatrzymania (Stop Conditions)
Prace zostaną zatrzymane natychmiast w przypadku wystąpienia:
1. Rozbieżności w historii Git lub braku możliwości wykonania operacji fast-forward (`ff-only`).
2. Brudnego drzewa roboczego przed zatwierdzeniem etapu.
3. Wymogu uprawnień roota / sudo / pkexec do działania programu lub testów.
4. Niezgodności ABI glibc uniemożliwiającej działanie tego samego artefaktu na wszystkich 7 profilach.
5. Awarii testu bezpiecznej ekstrakcji lub możliwości ucieczki poza staging.
6. Modyfikacji lub utraty jakichkolwiek danych użytkownika podczas aktualizacji/rollbacku.
7. Wystąpienia testów niestabilnych (flaky > 0) lub ostrzeżeń kompilatora/Clippy.
