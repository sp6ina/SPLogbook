# STAGE-M3-H — RAPORT PRZEDIMPLEMENTACYJNY (Linux DEB, amd64)

**Dokument ID:** DSH-SPLOGBOOK-M3H-PRE-001
**Bramka:** `STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE`
**Status w chwili pomiaru:** `NOT_STARTED` → `IN_PROGRESS`
**Aktualna data projektu:** **2026-11-12**
**Czas i strefa:** 2026-11-12T12:14+01:00, `Europe/Warsaw`
**GO OWNER:** `OWNER-M3-H-01 … OWNER-M3-H-10` (niniejszy prompt)
**Jedna sesja:** potwierdzona

> Raport został przedstawiony w rozmowie **przed pierwszą edycją pliku**. Poniższe
> wartości są zmierzone w tej sesji. Pola `TBD_CI` są rozstrzygane wyłącznie przez
> przypięty obraz i są zapisywane w raporcie końcowym z logów CI — host Windows nie
> ma `dpkg`, `lintian` ani Dockera.

---

## 1–2. Data, czas, strefa

| Pole | Wartość |
|---|---|
| Aktualna data projektu | **2026-11-12** (host + `AGENTS.md:1`) |
| Data z promptu | `2026-11-11` — zapis wcześniejszy; zegar **nie** został cofnięty |
| Czas / strefa | `2026-11-12T12:14+01:00`, `Europe/Warsaw` |
| Zasada | nie zmieniano zegara hosta ani dat istniejących commitów |

## 3. Stabilizacja workspace

Jedno worktree `C:/Users/sp6in/Documents/SPLogbook/SPLogbook` (`main`), brak
`.git/index.lock`, brak procesów `cargo`/`rustc`/`git`/innych agentów, brak
aktywnej blokady `target/`, brak drugiej sesji, `git diff --check` = EXIT 0,
0 zmian staged i 0 unstaged. `TRACKED_WORKTREE_CLEAN=true`.

## 4. Końcowy workflow M3-G (commit `cf127bb`)

| Run | Workflow | Wynik |
|---|---|---|
| `43174714152` | Build and Release SPLogbook | **success** |
| `43174714030` | CI | **success** (`Format and Lint`) |

Joby runu `43174714152`: `Build Windows x64` **success**, `Build Linux x86_64`
success, 4× `Verify Linux` success, 2× `Verify AppImage` success,
`Publish Release Assets` **skipped**.
**`M3_G_FINAL_CLOSURE_WORKFLOW=SUCCESS`** — M3-H może się rozpocząć.

## 5–13. Git na wejściu

| Pole | Wartość |
|---|---|
| HEAD / main / origin/main | `cf127bbedae0b6b57efcce521a9f546f083b5406` (identyczne) |
| ahead / behind | `0 / 0` |
| `git merge-base --is-ancestor cf127bb origin/main` | EXIT 0 |
| Aktywna gałąź | `main` |
| `v1.2.0` | obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — **VERIFIED** |
| Stashe | 2 (`stash@{0}`, `stash@{1}`) — **nietknięte** |
| Submoduły | brak (`git submodule status` pusty) |
| `scratch/` | lokalny, nieśledzony (`?? scratch/`), częściowo ignorowany wzorcami; **nie commitować, nie usuwać** |
| Fetch | `git -c http.sslBackend=openssl … fetch --all --tags --prune` = EXIT 0 |

## 14. Przeczytane dokumenty

`AGENTS.md` (jedyne AGENTS.md), `docs/PROJECT_CURRENT_STATE.md`,
`docs/NEXT_WORK_GATE.md`, `MASTER_IMPLEMENTATION_PLAN.md` §10,
`docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/DOCUMENTATION_INDEX.md`,
`docs/DEEPSEEK_HARNESS_HANDOFF.md`, `DESIGN_3_0_B_LINUX_UPDATE.md`,
`DESIGN_3_0_C_PLATFORM_INSTALLERS.md` §9–16,
`STAGE_M3_G_LINUX_APPIMAGE_INTEGRATION_REPORT.md`,
`STAGE_M3_G_LINUX_APPIMAGE_BLOCKED_REPORT.md`,
`STAGE_DEPENDENCY_RUSTIX_RUSTSEC_2026_0486_REMEDIATION_REPORT.md`,
`STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_REPORT.md`, raporty portable
tar.gz i QA baseline, `scripts/verify_linux_container.sh`,
`scripts/install_linux_runtime_deps.sh`, `scripts/build_linux_portable_tar_gz.sh`,
`installer/linux/**`, `.github/workflows/build-and-release.yml`,
`.github/workflows/ci.yml`, `tests/security_guard.rs`,
`tests/linux_appimage_packaging_test.rs`,
`tests/linux_multi_distribution_compatibility_test.rs`,
`tests/linux_xdg_data_preservation_test.rs`,
`tests/linux_archive_security_test.rs`, `src/core/xdg.rs`,
`src/core/build_info.rs`, `src/main.rs`, `src/cloud/updater/installation.rs`,
`Cargo.toml`, `.gitignore`, `README.md` (§Author & Maintainer),
`assets/icon.png`.

## 15. Hierarchia źródeł

`origin/main` → kod/skrypty/workflowy → `AGENTS.md` → decyzje OWNER
`OWNER-M3-H-01…10` → DESIGN → `PROJECT_CURRENT_STATE.md` → `NEXT_WORK_GATE.md` →
`MASTER_IMPLEMENTATION_PLAN.md` → `DECISION_TRACEABILITY_MATRIX.md` → raporty →
prompt.

## 16–17. Tożsamość bramki

`NEXT_GATE=M3-H — Integracja Linux DEB` (`docs/NEXT_WORK_GATE.md:63`),
`NEXT_GATE_STATUS=NOT_STARTED`, `NEXT_GATE_OWNER_GO=REQUIRED`; niniejszy prompt
udziela GO. **`M3_G_STATUS=CLOSED`**, `F03_STATUS=CLOSED`. Brak konfliktu bramki.

## 18–22. Istniejące artefakty i kontrakty

| Obszar | Stan faktyczny |
|---|---|
| Portable tar.gz | `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`; root `SPLogbook/`; `SPLogbook`, `splogbook-updater`, `lib/libhamlib.so.4`, `locales/`, `LICENSE`, `THIRD_PARTY_LICENSES/`, `assets/`, `README.txt`, marker `.splogbook-linux-portable-marker`; bez symlinków/setuid |
| AppImage | Type 2, `AppRun`, `usr/bin/{SPLogbook,splogbook-updater}`, `usr/lib/libhamlib.so.4`, marker AppImage, desktop + ikona; RUNPATH `$ORIGIN/../lib`; nie dotykamy |
| XDG | `$XDG_CONFIG_HOME/splogbook`, `$XDG_DATA_HOME/splogbook`, `$XDG_STATE_HOME/splogbook`, `$XDG_CACHE_HOME/splogbook`; ścieżki względne odrzucane; brak auto-migracji (`src/core/xdg.rs`) |
| Updater | `InstallationType::LinuxDeb` istnieje; marker `.splogbook-deb-marker` jest wykrywany (`src/cloud/updater/installation.rs:551`); `management_mode=SystemManaged`; **brak zmian kodu** |

## 23–28. Model pakietu i wersja

| Pole | Wartość |
|---|---|
| Typ | **binarny DEB** (`dpkg-deb`), **nie** source package; bez `debian/rules`, `changelog`, `.dsc`, `.changes`, `orig.tar` |
| Narzędzie | `dpkg-deb` z najnowszego `dpkg` w oficjalnym repo przypiętego obrazu `ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c`; brak PPA/third-party |
| dpkg / dpkg-deb / lintian | `TBD_CI` — builder i job zapisują `dpkg --version`, `dpkg-deb --version`, `lintian --version` |
| Źródło wersji | `Cargo.toml:3` = `1.3.0-alpha.1` (bez zmiany) |
| Wersja Debian | **`1.3.0~alpha.1-1`** (bez epoch; `~` przed `alpha`; rewizja `-1`) |
| Dowód porządku | `dpkg --validate-version`; `< 1.3.0-1`; `> 1.3.0~alpha.0-1`; `< 1.3.0~alpha.2-1`; `eq` sama ze sobą |
| Nazwa DEB | `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` + alias `SPLogbook-Linux-amd64.deb` |

## 29–38. `DEBIAN/control`

| Pole | Wartość |
|---|---|
| Package | `splogbook` |
| Version | `1.3.0~alpha.1-1` |
| Architecture | `amd64` |
| Maintainer | **`Mariusz Woźniak (SP6INA) <contact@splogbook.org>`** — kanoniczne, publiczne (`Cargo.toml:6`, `README.md:926–929`) |
| Section / Priority | `hamradio` / `optional` |
| Homepage | `https://github.com/sp6ina/SPLogbook` (jedyny weryfikowalny adres projektu; brak `splogbook.org` w repo) |
| Description | synopsis `Amateur radio logging and station automation workstation` + opis wielowierszowy; bez marketingu i bez deklaracji „oficjalny Debian” |
| Installed-Size | wyliczany deterministycznie z bajtów payloadu (KiB w górę) |
| Depends | `libc6 (>= 2.43)`, `libasound2t64`, `libfontconfig1`, `libgcc-s1`, `libgl1`, `libgtk-3-0t64`, `libudev1`, `libusb-1.0-0`, `libwayland-client0`, `libwayland-cursor0`, `libwayland-egl1`, `libx11-6`, `libxcursor1`, `libxi6`, `libxkbcommon-x11-0`, `libxkbcommon0`, `libxrandr2` |
| Mapowanie | SONAME→pakiet weryfikowane w CI przez `dpkg -S`/`readelf`/`ldd`; wszystkie nazwy są już instalowane z oficjalnego repo w profilu Ubuntu 26.04 (run `43174714152`) |
| Recommends/Suggests/Breaks/Conflicts/Provides/Replaces | **brak** (brak dowodu potrzeby; `libhamlib.so.4` jest bundlowany, więc **nie** w Depends) |
| Standards-Version | **`4.7.4`** — najnowszy Debian Policy Manual (4.7.4, marzec 2026; pakiet `debian-policy 4.7.4.1`); pole dokumentacyjne, nie jest polem binarnym |

## 39–45. Struktura, launcher, desktop, ikona, marker, skrypty

| Element | Wartość |
|---|---|
| `opt/SPLogbook/` | `SPLogbook`, `splogbook-updater`, `lib/libhamlib.so.4`, `locales/`, `LICENSE`, `THIRD_PARTY_LICENSES/`, `.splogbook-deb-marker` — **bez** `assets/`, `README.txt`, baz i konfiguracji; wyłącznie `libhamlib.so.4` |
| `usr/bin/splogbook` | `#!/bin/sh`, `set -eu`, `exec /opt/SPLogbook/SPLogbook "$@"`, 0755 root:root, bez sudo/pkexec/doas/LD_LIBRARY_PATH/HOME/XDG |
| `usr/share/applications/splogbook.desktop` | `Exec=splogbook`, `Icon=splogbook`, `StartupWMClass=SPLogbook`; `desktop-file-validate` |
| Ikona | `assets/icon.png` (zmierzone 256×256 PNG, SHA-256 `6f1a31d1…a509e`), bajtowo identyczna, 0644, `hicolor/256x256/apps/splogbook.png` |
| Marker | JSON: `marker_version=1.0`, `installation_type=LinuxDeb`, `management_mode=SystemManaged`, `package_version=1.3.0-alpha.1`, `debian_version=1.3.0~alpha.1-1`, `build_commit=<40 hex>`, `resolved_minimum_kernel=7.0`, `maximum_glibc=2.43`, `artifact_format=deb`, `architecture=amd64`, `package_name=splogbook` |
| Maintainer scripts | **tylko `postinst` i `postrm`** (idempotentny, nieinteraktywny refresh cache pulpitu, jeśli narzędzie istnieje); bez preinst/prerm; bez conffiles; bez sieci, sudo, HOME/XDG, GUI, debconf |
| Własność/prawa | katalogi 0755, binaria 0755, dane/licencje/desktop/ikona 0644, wszystko `root:root`, bez setuid/setgid/world-writable/symlinków |

## 46–57. Polityka danych i plany

| Obszar | Plan |
|---|---|
| Dane użytkownika | pakiet nie dotyka `$HOME`/XDG/baz/WAL/SHM/konfiguracji/ADIF/Cabrillo/backupów; purge ich nie usuwa; testy na tymczasowych HOME/XDG |
| Instalacja | `dpkg -i` + oficjalny `apt-get -f install` w jednorazowym `ubuntu:26.04`; `dpkg-query`, pliki, launcher, desktop, ikona, `--version`, `--build-info`, health check |
| Upgrade | kontrolowany fixture `1.3.0~alpha.0-1` zbudowany z tego samego payloadu → upgrade do `1.3.0~alpha.1-1`; dane XDG bez zmian |
| Downgrade | `dpkg -i` fixture, powrót do bieżącej wersji; dowód porównania wersji w praktyce |
| Reinstall | ponowna instalacja tej samej wersji; brak zmian danych i plików spoza pakietu |
| Remove / purge | pliki pakietu usunięte; dane XDG/bazy/backupy zachowane (SHA-256 identyczne) |
| Rollback/failure | uszkodzony DEB, brak zależności, zły skrypt → oczekiwany FAIL, bez fałszywego PASS |
| Testy pozytywne | 4 profile M3-F/M3-G + nowy natywny Ubuntu 26.04 DEB + reinstall/upgrade/downgrade/remove/purge + health |
| Testy negatywne | 27 mutacji, każda odrzucona niezerowym exit i **bez** komunikatu sukcesu |
| CI | nowe joby `Build DEB`, `Verify DEB`, `Verify DEB (negative fixtures)`; artefakt `SPLogbook-Linux-amd64-deb`; `Publish Release Assets` bez DEB |
| Licencje | `LICENSE` (GPL-3.0) w `/opt/SPLogbook` + `/usr/share/doc/splogbook/copyright` (DEP-5) + `README.Debian`; `THIRD_PARTY_LICENSES/` |
| SHA-256 | sidecar dla wersjonowanego DEB i aliasu; weryfikacja fail-closed; brak podpisu/repozytorium APT |

## 58–59. Pliki i commity

**Nowe:** `scripts/build_linux_deb.sh`, `scripts/verify_linux_deb.sh`,
`scripts/verify_linux_deb_negative_fixture.sh`,
`installer/linux/deb/{README.md,launcher,splogbook.desktop,templates/{control,postinst,postrm,README.Debian,copyright}}`,
`tests/linux_deb_packaging_test.rs`, raport przedimplementacyjny i końcowy.
**Modyfikowane:** `.github/workflows/build-and-release.yml`,
`.github/workflows/ci.yml`, `tests/security_guard.rs`, `.gitignore`,
`AGENTS.md`, `MASTER_IMPLEMENTATION_PLAN.md`, `docs/PROJECT_CURRENT_STATE.md`,
`docs/NEXT_WORK_GATE.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`,
`docs/DOCUMENTATION_INDEX.md`, `docs/DEEPSEEK_HARNESS_HANDOFF.md`.
**Commity:** `feat(linux): add verified DEB packaging pipeline` → push → CI;
po dowodach `docs(linux): close M3-H DEB integration gate` → push → CI.

## 60. Kryteria sukcesu

Wszystkie 53 kryteria §37 promptu, w tym: DEB tylko artefakt CI, brak
release/tagu, `v1.2.0` nietknięty, portable/AppImage/F-03 bez regresji,
lintian 0 errors, dane użytkownika nienaruszone, `HEAD==main==origin/main`,
`0/0`, tracked clean, `scratch/` niecommitowany, następna bramka nierozpoczęta.

## 61. Warunki zatrzymania

Każdy warunek §39 promptu: ruch `origin/main`, regresja M3-G/F-03, brak
mapowania zależności, GLIBC > 2.43, błąd lintian, nowe advisory, konieczność
force/rebase/reset, wykrycie sekretu lub nowa decyzja OWNER.

---

## Konflikty źródłowe rozstrzygnięte (nie arbitralnie)

1. **`DESIGN_3_0_B_LINUX_UPDATE.md` §9** opisuje układ `/usr/bin` +
   `/usr/lib/splogbook` + `/usr/share/splogbook/databases`, `Recommends: hamlib`
   i `libc6 (>= 2.35)`. Jest to **starszy** dokument; nowsze, jawne decyzje
   `OWNER-M3-H-01…10` ustanawiają układ `/opt/SPLogbook` + launcher,
   GLIBC 2.43 i `Depends` generowane z payloadu. Zgodnie z hierarchią źródeł
   decyzje OWNER wygrywają; DESIGN pozostaje źródłem architektury tam, gdzie
   nie jest sprzeczny. Brak potrzeby debhelper/source package.
2. **`DESIGN_3_0_B` §4** wymienia Debian/Ubuntu/Mint; obowiązująca polityka
   Linux (M3-F) uznaje Debian 13.7 za niewspierany (glibc 2.41), więc kontrakt
   DEB to **Ubuntu 26.04 LTS (glibc 2.43)**. Nie testujemy DEB na Fedorze,
   Arch ani Tumbleweed.
3. **Data promptu `2026-11-11`** vs host/AGENTS `2026-11-12` — przyjmuję
   późniejszą, zmierzoną datę; zegar i daty commitów nietknięte.

## Ograniczenie środowiska (jawne)

Host Windows nie ma Dockera/WSL ani `dpkg`/`lintian`. Lokalnie wykonano:
składnię POSIX (`sh -n`), statyczne testy Rust, pełny baseline `cargo` oraz
przegląd plików. **Natywny build/instalacja/upgrade/purge i lintian są
wykonywane wyłącznie w CI** na przypiętym `ubuntu:26.04`; nie są przedstawiane
jako lokalny PASS.
