# STAGE-M3-H — RAPORT KOŃCOWY (Linux DEB, amd64)

**Dokument ID:** DSH-SPLOGBOOK-M3H-FINAL-001
**Bramka:** `STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE`
**Status bramki:** **`COMPLETED_DEB_INTEGRATION_VERIFIED`**
**Aktualna data projektu:** **2026-11-12**
**Czas i strefa:** 2026-11-12 (sesja `Europe/Warsaw`, `+01:00`)
**Model / tryb:** DeepSeek-V4.1-Flash-1017 / autonomiczna, **jedna** sesja

---

## 1. Decyzje OWNER

| ID | Decyzja | Wykonanie |
|---|---|---|
| `OWNER-M3-H-01` | APPROVE BINARY DEB | binarny `dpkg-deb`; brak source package, `debian/rules`, `changelog`, `.dsc`, `.changes`, `orig.tar` |
| `OWNER-M3-H-02` | APPROVE DPKG-DEB | `dpkg 1.23.7` z oficjalnego repo obrazu `ubuntu:26.04@sha256:6a06111c…`; brak PPA/third-party |
| `OWNER-M3-H-03` | APPROVE `/opt/SPLogbook` LAYOUT | `opt/SPLogbook/{SPLogbook,splogbook-updater,lib/libhamlib.so.4,locales,LICENSE,THIRD_PARTY_LICENSES,.splogbook-deb-marker}` + launcher/desktop/ikona/doc |
| `OWNER-M3-H-04` | APPROVE FAIL-CLOSED LAUNCHER | `#!/bin/sh`, `set -eu`, `exec /opt/SPLogbook/SPLogbook "$@"`; bez sudo/pkexec/doas/LD_LIBRARY_PATH/HOME/XDG |
| `OWNER-M3-H-05` | APPROVE GENERATED-THEN-REVIEWED DEPENDS | 17 pakietów wyznaczonych z ELF/runtime i potwierdzonych w Ubuntu 26.04; `libhamlib.so.4` bundlowany, więc poza `Depends` |
| `OWNER-M3-H-06` | APPROVE MINIMAL MAINTAINER SCRIPTS | tylko `postinst`/`postrm`; idempotentny cache pulpitu, bez sieci/sudo/HOME/XDG/GUI/debconf |
| `OWNER-M3-H-07` | APPROVE SYSTEM-WIDE INSTALL WITH USER-DATA PRESERVATION | pełny cykl życia; dane XDG/QSO/WAL/SHM/konfiguracja/ADIF/Cabrillo/backupy SHA-256 identyczne |
| `OWNER-M3-H-08` | NO PACKAGE SIGNING IN M3-H | tylko SHA-256; brak APT/Release/InRelease/kluczy/podpisu |
| `OWNER-M3-H-09` | CI ARTIFACT ONLY | artefakt `SPLogbook-Linux-amd64-deb`; `Publish Release Assets` = `skipped`; brak tagu/release |
| `OWNER-M3-H-10` | APPROVE PACKAGE IDENTITY | `splogbook`, `amd64`, `1.3.0~alpha.1-1`, nazwa `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` + alias |

## 2. Wejście i ciągłość Git

| Pole | Wartość |
|---|---|
| Commit wejściowy | `cf127bbedae0b6b57efcce521a9f546f083b5406` (`HEAD = main = origin/main`, ahead/behind `0/0`) |
| Commit implementacyjny | `236b7ea4be70b028fd0f701143144ac6f0952262` — `feat(linux): add verified DEB packaging pipeline` |
| Korekty implementacyjne | `f9bab2e23951ba59bd7ca0ce7fed55f7e407ef37` — `fix(linux): accept versioned Hamlib payload and normalize DEB tests`; `af04b1300a6e34f8b5c572b68a5c9d6b5a6c93f5` — `fix(linux): install bundled Hamlib as a non-executable shared library`; `3171aa8d03444901c55be7aa1f134e297b061604` — `fix(linux): disable container doc exclusions in DEB lifecycle test` |
| Commit dokumentacyjny | `docs(linux): close M3-H DEB integration gate` (najnowszy commit `main` zawierający ten raport) |
| `v1.2.0` | obiekt `3ea8d51a…`, peeled `6f7d3aaa…` — **VERIFIED**, nietknięty |
| Stash / submoduły | 2 (nietknięte) / brak |
| Jedna sesja | potwierdzona; brak drugiego agenta, brak `.git/index.lock` |

## 3. Narzędzia i obraz budujący

| Pole | Wartość |
|---|---|
| Obraz budujący | `ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c` |
| Digest | `sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c` |
| Dystrybucja | `Ubuntu 26.04.1 LTS` |
| Źródło pakietów | oficjalne repozytorium dystrybucji (`apt`), bez third-party |
| `dpkg --version` | `Debian 'dpkg' package management program version 1.23.7 (amd64).` |
| `dpkg-deb --version` | `Debian 'dpkg-deb' package archive backend version 1.23.7 (amd64).` |
| `dpkg-architecture` | `amd64` |
| Lintian | `2.129.0ubuntu2.2` |
| Kompresja | `xz` (`dpkg-deb -Zxz`) |
| `SOURCE_DATE_EPOCH` | `1794496300` (timestamp commita `3171aa8`) |

## 4. Wersja i tożsamość

| Pole | Wartość |
|---|---|
| Wersja aplikacji | **`1.3.0-alpha.1`** (`Cargo.toml:3`) |
| Wersja Debian | **`1.3.0~alpha.1-1`** |
| Epoch | **brak** |
| Rewizja pakietu | `-1` |
| `dpkg --validate-version` | EXIT 0 |
| `< 1.3.0-1` | `dpkg --compare-versions` = prawda |
| `> 1.3.0~alpha.0-1` | prawda (dowód także praktycznym downgrade/upgrade) |
| `< 1.3.0~alpha.2-1` | prawda |
| Nazwa wersjonowana | `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` |
| Alias kanoniczny | `SPLogbook-Linux-amd64.deb` |

## 5. Artefakt DEB

| Pole | Wartość |
|---|---|
| Nazwa | `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` |
| Rozmiar | **11 883 596 B** |
| SHA-256 | **`85a4ca9877305d97f7c31929eb6034bd36e88dc8d7fd208f9fab9cf7d8890061`** |
| Alias | `SPLogbook-Linux-amd64.deb` — identyczny rozmiar i SHA-256 |
| `build_commit` | `3171aa8d03444901c55be7aa1f134e297b061604` |
| Maksymalny GLIBC | `2.43` (SPLogbook), `2.39` (updater), `2.43` (libhamlib) |
| Pliki | **12 plików regularnych / 28 wpisów** |
| `Installed-Size` | **45 879** KiB |
| Kontener | `ar` / `control.tar.xz` + `data.tar.xz`, `xz` |
| Deterministyczność | dwie niezależne budowy, identyczny SHA-256 |
| Podpis / repozytorium APT | brak (M3-H) |

## 6. `DEBIAN/control`

| Pole | Wartość |
|---|---|
| Package | `splogbook` |
| Version | `1.3.0~alpha.1-1` |
| Section | `hamradio` |
| Priority | `optional` |
| Architecture | `amd64` |
| Maintainer | `Mariusz Woźniak (SP6INA) <contact@splogbook.org>` |
| Installed-Size | `45879` |
| Depends | `libc6 (>= 2.43), libasound2t64, libfontconfig1, libgcc-s1, libgl1, libgtk-3-0t64, libudev1, libusb-1.0-0, libwayland-client0, libwayland-cursor0, libwayland-egl1, libx11-6, libxcursor1, libxi6, libxkbcommon-x11-0, libxkbcommon0, libxrandr2` |
| Homepage | `https://github.com/sp6ina/SPLogbook` |
| Recommends / Suggests / Breaks / Conflicts / Provides / Replaces | brak |
| Standards-Version (dokumentacyjnie) | **4.7.4** (najnowszy Debian Policy Manual, marzec 2026) |

## 7. Mapowanie `Depends` (SONAME → pakiet)

| SONAME | Pakiet | Źródło / weryfikacja |
|---|---|---|
| `libc.so.6`, `libm.so.6`, `libdl.so.2`, `libpthread.so.0`, `librt.so.1` | `libc6 (>= 2.43)` | `DT_NEEDED` + wymagany `GLIBC_2.43`; Ubuntu 26.04 ma 2.43 |
| `libgcc_s.so.1` | `libgcc-s1` | `DT_NEEDED` |
| `libasound.so.2` | `libasound2t64` | `DT_NEEDED` |
| `libX11.so.6` | `libx11-6` | winit/X11 |
| `libXcursor.so.1` | `libxcursor1` | winit/X11 |
| `libXrandr.so.2` | `libxrandr2` | winit/X11 |
| `libXi.so.6` | `libxi6` | winit/X11 |
| `libxkbcommon.so.0` | `libxkbcommon0` | winit/X11 |
| `libxkbcommon-x11.so.0` | `libxkbcommon-x11-0` | winit/X11 |
| `libGL.so.1` | `libgl1` | eframe/glow |
| `libfontconfig.so.1` | `libfontconfig1` | GUI text/font |
| `libwayland-client.so.0` | `libwayland-client0` | winit/Wayland |
| `libwayland-cursor.so.0` | `libwayland-cursor0` | winit/Wayland |
| `libwayland-egl.so.1` | `libwayland-egl1` | winit/Wayland |
| `libgtk-3.so.0` | `libgtk-3-0t64` | rfd/GTK |
| `libusb-1.0.so.0` | `libusb-1.0-0` | Hamlib USB |
| `libudev.so.1` | `libudev1` | Hamlib/device |

Wszystkie pakiety są dostępne w oficjalnym repozytorium Ubuntu 26.04 (potwierdzone przez
`apt-get -f install` w jobie `Verify DEB`; profil Ubuntu M3-F instalował tę samą listę).
`libhamlib.so.4` jest bundlowany i **nie** występuje w `Depends`.

## 8. Struktura, prawa i własność

| Ścieżka | Typ / prawa |
|---|---|
| `opt/SPLogbook/SPLogbook` | ELF 0755 |
| `opt/SPLogbook/splogbook-updater` | ELF 0755 |
| `opt/SPLogbook/lib/libhamlib.so.4` | ELF shared object **0644** |
| `opt/SPLogbook/locales/` | katalog 0755 (tłumaczenia są kompilowane do binarki; katalog zgodny z zatwierdzonym układem) |
| `opt/SPLogbook/LICENSE` | 0644 |
| `opt/SPLogbook/THIRD_PARTY_LICENSES/HAMLIB_LGPL-2.1.txt` | 0644 (LGPL-2.1 dla Hamlib) |
| `opt/SPLogbook/.splogbook-deb-marker` | 0644 |
| `usr/bin/splogbook` | POSIX sh 0755 |
| `usr/share/applications/splogbook.desktop` | 0644 |
| `usr/share/icons/hicolor/256x256/apps/splogbook.png` | PNG 256×256, 0644, bajtowo identyczna z `assets/icon.png` |
| `usr/share/doc/splogbook/{README.Debian,copyright,changelog.Debian.gz}` | 0644 |

Wszystkie wpisy `root:root`; brak symlinków, setuid/setgid/sticky, world-writable i plików
specjalnych. Nie ma `conffiles`, `preinst` ani `prerm`.

## 9. Launcher, desktop, ikona, marker

- Launcher: `#!/bin/sh`, `set -eu`, `exec /opt/SPLogbook/SPLogbook "$@"`; działa z dowolnego CWD,
  przekazuje argumenty i kod wyjścia, nie maskuje błędów, nie zapisuje do `/opt`.
- Desktop: `Exec=splogbook`, `Icon=splogbook`, `StartupWMClass=SPLogbook`;
  `desktop-file-validate` PASS (hint kategorii `HamRadio` jest informacyjny).
- Ikona: `assets/icon.png` 256×256, SHA-256 `6f1a31d13626a3a00a50a6e731d01e672d2b8fc485bdc207a2e022c635ea509e`,
  identyczna w pakiecie i po instalacji.
- Marker: `marker_version=1.0`, `installation_type=LinuxDeb`, `management_mode=SystemManaged`,
  `package_version=1.3.0-alpha.1`, `debian_version=1.3.0~alpha.1-1`,
  `build_commit=3171aa8…`, `resolved_minimum_kernel=7.0`, `maximum_glibc=2.43`,
  `artifact_format=deb`, `architecture=amd64`, `package_name=splogbook`; poprawny JSON,
  bez czasu lokalnego/użytkownika/ścieżki runnera.

## 10. Maintainer scripts

`postinst` i `postrm`: `#!/bin/sh`, `set -e`, idempotentne, nieinteraktywne, obsługują argumenty
dpkg; odświeżają wyłącznie `update-desktop-database` i `gtk-update-icon-cache`, jeśli narzędzie
istnieje. Brak sieci, `sudo`/`pkexec`/`doas`, `HOME`/XDG, GUI, updatera, debconf, systemd, cron,
PTT/RF i usuwania danych. Brak `|| true` na operacjach krytycznych.

## 11. Builder i weryfikator

**Builder** (`scripts/build_linux_deb.sh`): weryfikuje sidecar payloadu, `build_commit`,
`package_version`, `installation_type=LinuxPortableTarGz`, ELF, RUNPATH, GLIBC ≤ 2.43; składa
package-root z allowlisty; instaluje `libhamlib.so.4` jako 0644; tworzy marker, `control`,
`postinst`, `postrm`, `README.Debian`, `copyright`, `changelog.Debian.gz` i LGPL-2.1; skanuje
zabronione pliki/tokeny; ustawia deterministyczne mtime i `root:root`; buduje **dwa razy**
i wymaga identycznego SHA-256; zapisuje `deb-toolchain.json`, `deb-info.txt`,
`deb-contents.txt`, `deb-fields.txt`. Nie instaluje DEB na hoście i nie publikuje.

**Weryfikator** (`scripts/verify_linux_deb.sh`): fail-closed; sprawdza sidecar, typ, control,
`Depends`, `Installed-Size`, układ, prawa/własność, brak symlinków/setuid/setgid/world-writable,
brak danych użytkownika i sekretów, ELF/RUNPATH/GLIBC ≤ 2.43, `ldd`, launcher, desktop, ikonę,
marker/build_commit, lintian (0 errors), następnie natywny cykl: poprzedni fixture
`1.3.0~alpha.0-1` → upgrade → downgrade → re-upgrade → reinstall → remove → purge → fresh install
→ remove → purge, z izolowanym `HOME`/XDG i porównaniem SHA-256 wszystkich plików użytkownika.
Komunikat `DEB VERIFICATION COMPLETED SUCCESSFULLY` jest ostatnią instrukcją.

## 12. Testy negatywne — 27/27

| Mutacja | Exit | Mutacja | Exit |
|---|---|---|---|
| brak `.sha256` | 1 | user data w pakiecie | 1 |
| błędny SHA-256 | 1 | setuid | 1 |
| obcięty DEB | 1 | world-writable | 1 |
| brak `control.tar` | 1 | symlink absolutny | 1 |
| zła Architecture | 1 | symlink traversal | 1 |
| niepełne Depends | 1 | `postinst` z `HOME` | 1 |
| zły Installed-Size | 1 | `postinst` z siecią | 1 |
| brak Maintainer | 1 | brak binarki | 1 |
| brak updatera | 1 | brak libhamlib | 1 |
| brak launchera | 1 | launcher 0644 | 1 |
| launcher z sudo | 1 | launcher z LD_LIBRARY_PATH | 1 |
| niebezpieczny Exec | 1 | brak ikony | 1 |
| brak markera | 1 | zły build_commit | 1 |
| GLIBC > 2.43 (marker) | 1 | | |

Każda mutacja: exit niezerowy, **brak** komunikatu sukcesu. Job `Verify DEB (negative fixtures)`
= `success` wyłącznie dlatego, że wszystkie 27 mutacji zostało odrzuconych.

## 13. Natywny cykl życia (dowód CI)

| Krok | Wynik |
|---|---|
| Fresh install poprzedniego fixture `1.3.0~alpha.0-1` | PASS, dane użytkownika bez zmian |
| Upgrade do `1.3.0~alpha.1-1` | PASS, dane bez zmian |
| Downgrade do `1.3.0~alpha.0-1` i re-upgrade | PASS, dane bez zmian |
| Reinstall `1.3.0~alpha.1-1` | PASS, dane bez zmian |
| `dpkg-query` | `splogbook 1.3.0~alpha.1-1 amd64 install ok installed` |
| Pliki / desktop / ikona | wszystkie obecne, poprawne, `desktop-file-validate` PASS |
| `ldd` po instalacji | bez `not found` (libasound, libgcc_s, libm, libc, libusb, libudev) |
| Launcher `--version` | `SPLogbook 1.3.0-alpha.1` |
| `--build-info` | `Version: 1.3.0-alpha.1`, `Commit: 3171aa8…`, `Target: linux-x86_64` |
| Health check | exit 0, proof `OK`, `/opt/SPLogbook` niezmienione |
| Remove | PASS, dane bez zmian |
| Purge | PASS, dane bez zmian |
| Fresh install bieżącej wersji | PASS, dane bez zmian |
| Final remove + purge | PASS, dane bez zmian, `dpkg --audit` czysty |

Dane fixture: `default_log.db`, `-wal`, `-shm`, `station_config.json`, `log.adi`, `contest.cbr`,
`backups/backup.db`, `user.token`; manifest SHA-256 identyczny przed/po każdej operacji.

## 14. Lintian

| Pole | Wartość |
|---|---|
| Wersja | `2.129.0ubuntu2.2` |
| Errors | **0** |
| Warnings | **2** — `initial-upload-closes-no-bugs` (pierwszy upload, brak `Closes:`), `no-manual-page` (manpage poza zatwierdzonym układem) |
| Suppression | wyłącznie `dir-or-file-in-opt,custom-library-search-path` — bezpośredni skutek `OWNER-M3-H-03` (`/opt` + prywatny `libhamlib.so.4` z `RUNPATH=$ORIGIN/lib`); każdy inny tag jest raportowany |
| Wynik | `PASS: lintian reported 0 errors (2 warnings; …)` |

## 15. Baseline lokalny (2026-11-12)

| Komenda | Wynik |
|---|---|
| `cargo +stable fmt --all -- --check` | EXIT 0 |
| `cargo +stable check --workspace --all-targets -j 1` | EXIT 0 |
| `cargo +stable clippy --workspace --all-targets -j 1 -- -D warnings` | EXIT 0, 0 ostrzeżeń |
| `cargo +stable test --workspace --all-targets -j 1` | EXIT 0 — **2735** zarejestrowanych, **2725** wykonanych, **2725** zaliczonych, **0** nieudanych, **10** ignorowanych |
| `cargo +stable doc --workspace --no-deps` | EXIT 0 (21 pre-existing warningów dokumentacji) |
| `cargo +stable tree -d` | EXIT 0 |
| `cargo audit --deny warnings --no-yanked <7 dozwolonych ignore>` | **EXIT 0** (1471 advisory, 674 zależności, 0 znalezisk) |
| `git diff --check` | EXIT 0 |
| Walidacja YAML / JSON | PASS / PASS |

`security_guard` = **124/124**; `linux_deb_packaging_test` = **17/17**.
Po ostatnich poprawkach (tryb `libhamlib` 0644, wyłączenie `path-exclude` w kontenerze)
powtórzono pełny przebieg z jawnym filtrem pre-existing testu blokowanego przez sandbox
Windows: **2724/2724**, 0 failed, 10 ignored, 1 filtered. Ten sam test przechodzi w CI.
`RUSTSEC-2026-0486=ABSENT`, `NEW_VULNERABILITIES=0`, `NEW_UNAPPROVED_WARNINGS=0`.

## 16. Dowody CI

| Run | Workflow | head_sha | Wynik |
|---|---|---|---|
| `43245181469` | `CI` | `3171aa8…` | **`success`** (fmt, clippy, testy, `cargo audit`) |
| `43245181565` | `Build and Release SPLogbook` | `3171aa8…` | **`success`** |

Joby runu `43245181565`: `Build Linux x86_64` = `success`, `Build Windows x64` = `success`,
`Build DEB (Ubuntu 26.04 amd64)` = **`success`**, `Verify DEB (Ubuntu 26.04 LTS native)` =
**`success`**, `Verify DEB (negative fixtures)` = **`success`** (27/27),
`Verify Linux (Ubuntu/Fedora/Arch/openSUSE)` = **4 × `success`**,
`Verify Linux (negative fixture)` = `success`, `Verify AppImage (negative fixtures)` =
`success`, `Verify AppImage (real FUSE)` = `success`, `Publish Release Assets` = **`skipped`**.

Artefakt CI `SPLogbook-Linux-amd64-deb`: DEB + alias + sidecary + `deb-toolchain.json` +
`deb-info.txt` + `deb-contents.txt` + `deb-fields.txt` + `deb-dpkg-version.txt` + log budowy.
Artefakty dowodowe: `deb-evidence-verify`, `deb-evidence-negative-fixtures`.

## 17. Brak regresji istniejących artefaktów

- Portable tar.gz: 4/4 profile `success`, `NATIVE VERIFICATION COMPLETED SUCCESSFULLY`, SHA-256
  `d0c626cd36e71a2a0fdb03a5eae4961d70b2af1f9d6678a4012aeab2a920fed8`.
- AppImage: 2 joby (`negative fixtures`, `real FUSE`) + weryfikacja w 4 profilach = `success`;
  AppImage zbudowany tym samym przypiętym toolchainem; FUSE `VERIFIED`.
- F-03: `Verify Linux (negative fixture)` = `success`; brak biblioteki → exit 1, bez komunikatu
  sukcesu; `scripts/verify_linux_container.sh` nietknięty.
- Publikacja: `Publish Release Assets` = `skipped`; brak nowego tagu, brak release, brak
  publicznego DEB; `v1.2.0` nietknięty.

## 18. Kryteria zamknięcia (53/53)

| # | Kryterium | Wynik |
|---|---|---|
| 1 | origin/main odtworzony | ✅ `cf127bb` → `3171aa8` |
| 2 | końcowy workflow M3-G sklasyfikowany | ✅ run `43174714152` = `success` |
| 3 | dokumenty kanoniczne przeczytane | ✅ |
| 4 | M3-G CLOSED | ✅ |
| 5 | M3-H właściwą bramką | ✅ |
| 6 | zatwierdzony binarny model DEB | ✅ OWNER-M3-H-01 |
| 7 | oficjalny dpkg-deb | ✅ 1.23.7 |
| 8 | wersja dpkg zapisana | ✅ |
| 9 | poprawna nazwa pakietu | ✅ `splogbook` |
| 10 | poprawny porządek wersji | ✅ `1.3.0~alpha.1-1` |
| 11 | Architecture=amd64 | ✅ |
| 12 | control poprawny | ✅ |
| 13 | Depends potwierdzone | ✅ 17 pakietów |
| 14 | struktura pakietu poprawna | ✅ |
| 15 | launcher działa | ✅ |
| 16 | desktop poprawny | ✅ |
| 17 | ikona poprawna | ✅ |
| 18 | marker poprawny | ✅ |
| 19 | prawa i własność | ✅ root:root |
| 20 | brak setuid/setgid | ✅ |
| 21 | brak niebezpiecznych symlinków | ✅ |
| 22 | brak danych użytkownika | ✅ |
| 23 | GLIBC ≤ 2.43 | ✅ |
| 24 | ldd bez `not found` | ✅ |
| 25 | SHA-256 poprawny | ✅ `85a4ca98…` |
| 26 | lintian bez błędów | ✅ 0 errors |
| 27 | fresh install | ✅ |
| 28 | reinstall | ✅ |
| 29 | upgrade | ✅ |
| 30 | remove | ✅ |
| 31 | purge | ✅ |
| 32 | dane użytkownika nienaruszone | ✅ SHA-256 |
| 33 | testy negatywne odrzucają mutacje | ✅ 27/27 |
| 34 | portable tar.gz działa | ✅ |
| 35 | AppImage działa | ✅ |
| 36 | F-03 fail-closed | ✅ |
| 37 | pełny baseline zielony | ✅ |
| 38 | clippy 0 ostrzeżeń | ✅ |
| 39 | security_guard | ✅ 124/124 |
| 40 | strażnik M3-H | ✅ |
| 41 | cargo audit | ✅ EXIT 0 |
| 42 | brak nowego advisory | ✅ |
| 43 | git diff --check | ✅ |
| 44 | DEB tylko artefakt CI | ✅ |
| 45 | brak publicznego release | ✅ |
| 46 | brak nowego tagu | ✅ |
| 47 | v1.2.0 nietknięty | ✅ |
| 48 | commit dokumentacyjny wypchnięty | ✅ |
| 49 | HEAD == main == origin/main | ✅ |
| 50 | ahead/behind = 0/0 | ✅ |
| 51 | TRACKED_WORKTREE_CLEAN=true | ✅ |
| 52 | scratch/ niecommitowany | ✅ |
| 53 | kolejna bramka nierozpoczęta | ✅ M3-I `NOT_STARTED` |

## 19. Ryzyka rezydualne

1. Kontenery OCI współdzielą jądro hosta — dowód jest dowodem userspace/CI, nie dowodem jądra
   dystrybucji.
2. Lintian: 2 warnings (`initial-upload-closes-no-bugs`, `no-manual-page`) są świadomie
   zaakceptowane; manpage i `Closes:` nie należą do zatwierdzonego układu M3-H.
3. Suppression `dir-or-file-in-opt,custom-library-search-path` jest bezpośrednim skutkiem
   `OWNER-M3-H-03`; każdy inny tag lintian jest raportowany i nie jest wyciszany.
4. Debian 13.7 i Leap 16.1 pozostają niewspierane (glibc < 2.43); DEB nie jest testowany na
   dystrybucjach RPM/Arch.
5. Katalog `locales/` jest pusty w payloadzie portable (tłumaczenia są kompilowane do binarki);
   lintian zgłasza to jako `info` (`package-contains-empty-directory`), nie error.
6. Fizyczny HIL/PTT/RF pozostają poza zakresem (nieuruchamiane).

## 20. Następna bramka

NEXT_GATE: **`M3-I`** — Integracja Linux RPM (`MASTER_IMPLEMENTATION_PLAN.md` §10)
NEXT_GATE_STATUS: `NOT_STARTED`
NEXT_GATE_OWNER_GO: `REQUIRED`
M3_J_STATUS: `NOT_STARTED`

Bramka `M3-I` **nie została rozpoczęta**; `M3-J` (Flatpak) pozostaje `NOT_STARTED`.

**Ostateczny status M3-H:** **`COMPLETED_DEB_INTEGRATION_VERIFIED`**.
