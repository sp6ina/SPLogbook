# STAGE-M3-I — RAPORT PRZEDIMPLEMENTACYJNY (Linux RPM, x86_64)

**Dokument ID:** DSH-SPLOGBOOK-M3I-PRE-001
**Bramka:** `STAGE-M3-I-LINUX-RPM-INTEGRATION-GATE`
**Status w chwili pomiaru:** `NOT_STARTED` → `IN_PROGRESS`
**Aktualna data projektu:** **2026-11-13**
**Czas i strefa pomiaru:** `2026-11-13T08:00+01:00`, `Europe/Warsaw`
**GO OWNER:** `OWNER_GO_M3_I_RPM=true` (niniejszy prompt)
**Jedna sesja zapisująca:** potwierdzona; brak drugiej sesji, brak blokady `.git/index.lock`
**Zakres:** wyłącznie M3-I. **M3-J pozostaje `NOT_STARTED`.**

> Raport powstaje **przed pierwszą zmianą kodu**. Wartości oznaczone `TBD_CI`
> mogą zostać rozstrzygnięte wyłącznie przez przypięty obraz OCI i są
> zapisywane w raporcie końcowym z logów CI. Host Windows nie ma `rpm`,
> `rpmbuild`, `rpmlint`, Dockera ani WSL — nie jest to ukrywane ani
> przedstawiane jako lokalny PASS.

---

## 1. Dokładny stan repozytorium na wejściu

### 1.1. Git — pomiar 2026-11-13

| Pole | Wartość |
|---|---|
| Gałąź | `main` |
| HEAD | `9427e5767f101d42347a5a004d7c309e1ec29783` |
| `main` | `9427e5767f101d42347a5a004d7c309e1ec29783` |
| `origin/main` po fetch | `9427e5767f101d42347a5a004d7c309e1ec29783` |
| ahead / behind | `0 / 0` |
| `HEAD == main == origin/main` | **TRUE** |
| `git diff --check` | **EXIT 0** |
| Zmiany śledzone (staged/unstaged) | **0 / 0** |
| `TRACKED_WORKTREE_CLEAN` | **TRUE** |
| Czystość ścisła (`CLEAN`) | **FALSE** — obecny `scratch/` |
| `git ls-files -o --exclude-standard` | `scratch/` i pliki w nim (m.in. dowody M3-G/M3-H, logi, skrypty, `scratch/tree_dupes.txt`) |
| `scratch/` | lokalny, **nie commitować, nie usuwać** |
| Stashe | 2 (`stash@{0}`, `stash@{1}`) — **nietknięte** |
| Submoduły | brak (`git submodule status` pusty) |
| Aktywna operacja Git | brak; `.git/index.lock` nie istnieje |

Fetch wykonano per-command z
`git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false fetch --prune origin`.
`scratch/` nie jest ignorowany jako katalog — ignorowane są tylko wybrane
wzorce plików; dlatego `?? scratch/` jest realnym stanem, a nie fantomem F-25.

### 1.2. Ostatnie commity

```text
9427e57 ci: pin the documented Rust toolchain in CI
a4a40ae docs(linux): close M3-H DEB integration gate
3171aa8 fix(linux): disable container doc exclusions in DEB lifecycle test
af04b13 fix(linux): install bundled Hamlib as a non-executable shared library
f9bab2e fix(linux): accept versioned Hamlib payload and normalize DEB tests
236b7ea feat(linux): add verified DEB packaging pipeline
cf127bb docs(linux): close M3-G AppImage integration gate
35464c7 fix(deps): remediate RUSTSEC-2026-0486 by updating rustix to 1.1.6
```

### 1.3. Tagi i release

- Tagi lokalne i zdalne są zgodne; **nie utworzono nowego tagu**.
- `v1.2.0` — obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`,
  peeled commit `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`, **VERIFIED**.
- Brak tagu `v1.3.0*`; brak GitHub Release dla `1.3.0-alpha.1`.
- Ostatni publiczny release: `SPLogbook v1.2.0` (2026-10-26).
- `Publish Release Assets` pozostaje wyłączone/skipped dla pushy na `main`.

### 1.4. Stan poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE` | `COMPLETED_APPIMAGE_INTEGRATION_VERIFIED` |
| `STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE` | `COMPLETED_DEB_INTEGRATION_VERIFIED` |
| `STAGE-M3-I-LINUX-RPM-INTEGRATION-GATE` | `NOT_STARTED` → `IN_PROGRESS` (ten prompt) |
| `M3-J` (Flatpak) | **`NOT_STARTED`** — nie rozpoczynać |

Ostatni zielony baseline przed tą bramką:

| Run | Workflow | HEAD | Wynik |
|---|---|---|---|
| `43377393318` | Build and Release SPLogbook | `9427e57` | `success`; `Publish Release Assets` = `skipped` |
| `43377393204` | CI | `9427e57` | `success` |

W runie `43377393318` wszystkie istniejące joby były `success`:
Windows, Linux, 4× Verify Linux, Verify Linux negative, 2× Verify AppImage,
Build DEB, Verify DEB, Verify DEB negative. `Publish Release Assets` = `skipped`.

### 1.5. Środowisko lokalne

| Element | Wartość |
|---|---|
| Host | Windows, `C:\Users\sp6in\Documents\SPLogbook\SPLogbook` |
| Rust / Cargo | `rustc 1.99.0`, `cargo 1.99.0` (zgodne z pinem CI) |
| Git | `2.56.1.windows.1` |
| Docker / Podman / WSL | **brak** |
| `rpm` / `rpmbuild` / `rpmlint` | **brak** |
| POSIX shell | Git for Windows `sh.exe` / `bash.exe` (tylko składnia i testy statyczne) |
| `scratch/` | obecny, nieśledzony; nie commitować |

**Wniosek:** natywny build RPM, cykl życia, `rpmlint`, `ldd` i determinizm
binarny są wykonywane w CI na przypiętych obrazach OCI. Lokalnie można
wykonać wyłącznie testy statyczne, składnię powłoki i baseline Rust. Raport
końcowy rozdzieli te dwa rodzaje dowodów.

---

## 2. Analiza istniejącej implementacji

### 2.1. Portable tar.gz (M3-F)

- Builder: `scripts/build_linux_portable_tar_gz.sh`.
- Artefakt: `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz` + `.sha256`.
- Root archiwum: pojedynczy katalog `SPLogbook/`.
- Zawartość: `SPLogbook`, `splogbook-updater`, `lib/libhamlib.so.4`,
  `locales/`, `THIRD_PARTY_LICENSES/`, `LICENSE`, `assets/`, `README.txt`,
  `.splogbook-linux-portable-marker`.
- Brak symlinków, setuid/setgid; biblioteka Hamlib jest plikiem regularnym.
- RUNPATH: binaria `$ORIGIN/lib:$ORIGIN/../lib`, biblioteki `$ORIGIN`
  (RPM przyjmie **węższy, dokładny** kontrakt `$ORIGIN/lib` i `$ORIGIN`).
- Marker: `LinuxPortableTarGz`, `SelfManaged`, `build_commit`, target
  `linux-x86_64`, minimum kernel `7.0`, glibc family.
- To jest **jedyne wejście** dla buildera RPM; RPM nie kompiluje kodu ponownie.

### 2.2. DEB (M3-H)

- Builder: `scripts/build_linux_deb.sh`, `dpkg-deb`, podwójny build i SHA-256.
- Układ: `/opt/SPLogbook` + `/usr/bin/splogbook` + desktop + ikona.
- `Depends` z jawnych nazw pakietów Ubuntu (`libc6 (>= 2.43)`, `libgtk-3-0t64`, ...).
- Maintainer scripts: tylko `postinst`/`postrm`, odświeżanie cache pulpitu.
- Weryfikator: `scripts/verify_linux_deb.sh`, lintian z dwoma wyjątkami
  (`dir-or-file-in-opt`, `custom-library-search-path`) na podstawie
  `OWNER-M3-H-03`.
- Negatywne: 27/27 mutacji odrzuconych.
- Cykl życia: fresh install → upgrade z `1.3.0~alpha.0-1` → downgrade →
  re-upgrade → reinstall → remove → purge → fresh install → remove → purge.
- **Nie kopiujemy mechanicznie semantyki DEB.** RPM ma inne skrypty
  (`%post`, `%postun`), inne narzędzia (`rpm`, `rpmbuild`, `rpmlint`), inny
  model usuwania (`rpm -e`, bez `purge`) i inne nazwy zależności.

### 2.3. AppImage (M3-G)

- `scripts/build_linux_appimage.sh`, `installer/linux/appimage/AppRun`.
- Type 2, bundled Hamlib w `usr/lib`, marker AppImage, FUSE zweryfikowany.
- Nie dotykamy builderów, markerów ani jobów AppImage.

### 2.4. Windows

- `build-windows`, `build_linux`, Inno Setup i MSI pozostają bez zmian.
- RPM nie może wpływać na artefakty Windows ani na ich joby.

### 2.5. Security guards i workflowy

- `tests/security_guard.rs` zawiera strażników M3-F, F-03, M3-G, M3-H.
- `m3_h_deb_integration_guard` pilnuje DEB, 27 mutacji, workflow i braku publikacji.
- `.github/workflows/build-and-release.yml`:
  - `build-linux` buduje portable + AppImage,
  - `build-deb` / `verify-deb` / `verify-deb-negative-fixtures`,
  - `verify-linux-matrix` (4 profile),
  - `publish-release` jest tag-gated i obecnie nie publikuje DEB.
- `.github/workflows/ci.yml` pinuje Rust `1.99.0`, uruchamia `fmt`, `clippy
  -D warnings`, pełny `cargo test` i `cargo audit` z jawnymi baseline'owymi
  wyjątkami.
- Nie wolno wyciszać nowych lintów ani błędów CI.

### 2.6. Kontrakty payloadu i aplikacji

- `Cargo.toml:3` = `1.3.0-alpha.1`; `rust-version = 1.99.0`.
- `src/cloud/updater/installation.rs` już rozpoznaje `LinuxRpm` i plik
  `.splogbook-rpm-marker`; `management_mode = SystemManaged`.
- `src/cloud/updater/capabilities.rs` traktuje `LinuxRpm` jak `LinuxDeb`:
  delegacja do menedżera pakietów, brak self-apply, natywny rollback.
- **Nie ma potrzeby zmiany `src/**`.** Marker RPM jest już obsługiwany.
- Dane użytkownika: XDG (`config`, `data`, `state`, `cache`), bazy QSO,
  WAL/SHM, `station_config.json`, ADIF, Cabrillo, backupy i sekrety.

---

## 3. Proponowany kontrakt RPM

### 3.1. Tożsamość pakietu

| Pole | Wartość |
|---|---|
| `Name` | **`splogbook`** |
| `Epoch` | brak |
| `Version` | **`1.3.0`** |
| `Release` | **`0.1.alpha.1`** (bez `%{?dist}`) |
| `Architecture` | **`x86_64`** |
| `Vendor` | `SPLogbook` |
| `Packager` | `Mariusz Woźniak (SP6INA) <contact@splogbook.org>` |
| `License` | `GPL-3.0-or-later AND LGPL-2.1-or-later` |
| `URL` | `https://github.com/sp6ina/SPLogbook` |
| `Group` | `Applications/Communications` |
| `Summary` | `Amateur radio logging and station automation workstation` |
| `BuildArch` | `x86_64` |

### 3.2. Mapowanie wersji `1.3.0-alpha.1`

- SemVer `1.3.0-alpha.1` → RPM `Version: 1.3.0`, `Release: 0.1.alpha.1`.
- Prerelease jest **przed** `1.3.0-1` dzięki numerowi release `0.*`.
- Builder dowodzi porządku przez `rpmdev-vercmp`:
  - `1.3.0-0.1.alpha.1` < `1.3.0-1`,
  - `1.3.0-0.1.alpha.1` > `1.3.0-0.1.alpha.0`,
  - `1.3.0-0.1.alpha.1` < `1.3.0-0.1.alpha.2`,
  - równość z samym sobą.
- Brak epoch. Wersja `1.3.0-alpha.1` pozostaje niezmieniona w `Cargo.toml`.
- `%{?dist}` jest świadomie **wyłączony**: artefakt ma być identyczny na
  wspieranych systemach RPM i nie może zależeć od dystrybucyjnego sufiksu.

### 3.3. Zależności runtime

`AutoReqProv: no` — brak automatycznego generowania zależności przez
`find-requires`, ponieważ kontrakt ma być jawny i przenośny między Fedorą
a openSUSE. Wymagania są wyrażone przez capabilities soname/GLIBC, nie przez
nazwy pakietów jednej dystrybucji:

```text
glibc >= 2.43
libasound.so.2()(64bit)
libX11.so.6()(64bit)
libXcursor.so.1()(64bit)
libXrandr.so.2()(64bit)
libXi.so.6()(64bit)
libxkbcommon.so.0()(64bit)
libxkbcommon-x11.so.0()(64bit)
libGL.so.1()(64bit)
libfontconfig.so.1()(64bit)
libwayland-client.so.0()(64bit)
libwayland-cursor.so.0()(64bit)
libwayland-egl.so.1()(64bit)
libgtk-3.so.0()(64bit)
libusb-1.0.so.0()(64bit)
libudev.so.1()(64bit)
libgcc_s.so.1()(64bit)
libm.so.6()(64bit)
libc.so.6()(64bit)
libc.so.6(GLIBC_2.43)(64bit)
```

Zasady:

- `libhamlib.so.4` jest **prywatny i bundlowany**, więc **nie** występuje
  w `Requires`.
- Nie ma `Recommended`, `Suggests`, `Supplements`, `Enhances`, `Provides`
  ani `Obsoletes` poza automatycznym `splogbook = <evr>`.
- `rpmlib(...)` są standardowymi, technicznymi wymaganiami RPM i nie są
  traktowane jako zależności dystrybucyjne.
- Weryfikator odrzuci każdą zależność spoza zatwierdzonego zbioru, każdą
  zależność `hamlib`, każdą ścieżkę absolutną i każde `rpmlib` nowsze niż
  wymagane przez użyty format payloadu.

### 3.4. Docelowy układ plików

```text
/opt/SPLogbook/SPLogbook
/opt/SPLogbook/splogbook-updater
/opt/SPLogbook/lib/libhamlib.so.4
/opt/SPLogbook/locales/...
/opt/SPLogbook/LICENSE
/opt/SPLogbook/THIRD_PARTY_LICENSES/...
/opt/SPLogbook/.splogbook-rpm-marker

/usr/bin/splogbook                                  (POSIX sh, 0755)
/usr/share/applications/splogbook.desktop
/usr/share/icons/hicolor/256x256/apps/splogbook.png
/usr/share/doc/splogbook/README.RPM
/usr/share/licenses/splogbook/LICENSE
/usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt
```

Świadomie **nie** kopiujemy do pakietu: `assets/`, `README.txt`,
`.splogbook-linux-portable-marker`, `rigctld`, `rotctld`, żadnych baz,
konfiguracji, eksportów, backupów ani sekretów.

### 3.5. Właściciele, grupy i tryby

- Wszystkie wpisy: **`root:root`**.
- Katalogi: `0755`.
- `SPLogbook`, `splogbook-updater`, `/usr/bin/splogbook`: `0755`.
- `libhamlib.so.4`: `0644` (biblioteka współdzielona, **nie** wykonywalna).
- `LICENSE`, `THIRD_PARTY_LICENSES/*`, marker, desktop, ikona, dokumentacja:
  `0644`.
- Zakaz symlinków, setuid, setgid, sticky bit i world-writable.

### 3.6. Launcher

Plik `installer/linux/rpm/launcher`, instalowany jako `/usr/bin/splogbook`:

```sh
#!/bin/sh
set -eu
exec /opt/SPLogbook/SPLogbook "$@"
```

Bez `sudo`, `pkexec`, `doas`, `LD_LIBRARY_PATH`, `HOME`, XDG, sieci, zapisu
i maskowania exit code. Działa z dowolnego CWD.

### 3.7. Desktop i ikony

- `installer/linux/rpm/splogbook.desktop`:
  `Exec=splogbook`, `Icon=splogbook`, `StartupWMClass=SPLogbook`,
  `Terminal=false`, `Categories=HamRadio;Utility;`.
- Walidacja `desktop-file-validate`.
- Ikona: `assets/icon.png` (256×256 PNG), bajtowo identyczna,
  `/usr/share/icons/hicolor/256x256/apps/splogbook.png`.

### 3.8. Prywatny Hamlib i RUNPATH

- Dokładnie jeden plik: `lib/libhamlib.so.4`, SONAME `libhamlib.so.4`.
- `SPLogbook` i `splogbook-updater`: **dokładnie** `RUNPATH=$ORIGIN/lib`.
- `libhamlib.so.4`: **dokładnie** `RUNPATH=$ORIGIN`.
- Zakaz `RPATH` (starego znacznika), zakaz ścieżek absolutnych, zakaz
  `LD_LIBRARY_PATH`.
- `ldd` po instalacji: zero `not found`, exit 0.
- Maksymalny wymagany symbol GLIBC ≤ `2.43`.

### 3.9. Skrypty pakietu

Tylko dwa scriptlety:

- `%post` — po install/upgrade odświeża `update-desktop-database` i
  `gtk-update-icon-cache`, **tylko jeśli narzędzie już istnieje**; błąd
  narzędzia jest ostrzeżeniem, nie fail.
- `%postun` — po remove/upgrade robi to samo, nie usuwa danych użytkownika.

Zakazane i odrzucane: `%pre`, `%preun`, `%pretrans`, `%posttrans`,
`%trigger*`, `%verifyscript`, `%config`, `%ghost`. Brak sieci, `sudo`,
`pkexec`, `doas`, `systemctl`, `crontab`, `HOME`, XDG, GUI, PTT/RF.

### 3.10. Ochrona danych użytkownika

- Pakiet nie zawiera i nie tworzy plików w `$HOME` ani XDG.
- `%post`/`%postun` nie odwołują się do `HOME`/XDG.
- `rpm -e` usuwa wyłącznie pliki z manifestu RPM; nie ma „purge”, który
  mógłby usunąć dane użytkownika.
- Testy cyklu życia używają izolowanego `HOME` i porównują SHA-256 wszystkich
  plików przed/po każdej operacji.

### 3.11. Marker RPM

`/opt/SPLogbook/.splogbook-rpm-marker` — deterministyczny JSON:

```json
{
  "marker_version": "1.0",
  "installation_type": "LinuxRpm",
  "management_mode": "SystemManaged",
  "package_version": "1.3.0-alpha.1",
  "rpm_version": "1.3.0",
  "rpm_release": "0.1.alpha.1",
  "build_commit": "<40 hex>",
  "resolved_minimum_kernel": "7.0",
  "maximum_glibc": "2.43",
  "artifact_format": "rpm",
  "architecture": "x86_64",
  "package_name": "splogbook",
  "target": "linux-x86_64",
  "created_by": "scripts/build_linux_rpm.sh"
}
```

Marker jest zgodny z istniejącym `InstallationType::LinuxRpm`
(`.splogbook-rpm-marker`); **nie zmieniamy `src/**`**.

### 3.12. Manifest i sumy kontrolne

- `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm.sha256`
- `SPLogbook-Linux-x86_64.rpm.sha256` (alias)
- `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm.manifest` — deterministyczny
  manifest tekstowy: tożsamość, SHA-256 artefaktu, rozmiar, liczba plików
  i wpisów, oraz dla każdego wpisu typ, tryb, UID/GID i SHA-256 pliku.
- Manifest jest generowany z **tego samego** package-root, z którego powstał
  RPM; weryfikator porównuje go z rzeczywistą zawartością RPM.

### 3.13. Nazwa artefaktu i alias

| Rola | Nazwa |
|---|---|
| Wersjonowany RPM | `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` |
| Alias CI | `SPLogbook-Linux-x86_64.rpm` |
| Wewnętrzna nazwa RPM | `splogbook-1.3.0-0.1.alpha.1.x86_64.rpm` |

Alias jest kopią bajtowo identyczną; oba mają sidecar SHA-256.

---

## 4. Macierz wspieranych systemów RPM

### 4.1. System referencyjny

**Fedora Linux 45 (GA)** — natywny system referencyjny dla builda i
weryfikacji RPM.

Uzasadnienie:

- Fedora 45 ma glibc **2.44** (potwierdzone w runie `43377393318`), więc
  spełnia politykę `GLIBC_2.43` z zapasem.
- Jest to natywny system RPM z `rpmbuild`, `rpmlint`, `rpmdevtools` i
  oficjalnym repozytorium dystrybucji.
- Obraz jest już przypięty w istniejącej macierzy:
  `fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c`.
- Fedora 45 jest jedną z czterech dystrybucji wspieranych przez politykę
  M3-F; nie wprowadzamy nowej dystrybucji.

### 4.2. System kompatybilnościowy

**openSUSE Tumbleweed** — test instalacji i cyklu życia tym samym artefaktem
Fedora, o ile test przejdzie.

- glibc **2.44** (potwierdzone w runie `43377393318`).
- Obraz:
  `opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1`.
- Test jest dowodem kompatybilności ABI/userspace, a nie deklaracją, że
  pakiet został zbudowany natywnie na openSUSE.
- Jeżeli `rpm -i`/cykl życia nie przejdzie, openSUSE zostanie jawnie
  oznaczone jako **niewspierane dla tego RPM w M3-I**, bez ukrywania błędu.

### 4.3. Wspierane / niewspierane

| System | Status w M3-I | Podstawa |
|---|---|---|
| Fedora Linux 45 | **wspierany i testowany** (referencyjny) | glibc 2.44, natywny RPM, pełny cykl życia |
| openSUSE Tumbleweed | **testowany kompatybilnościowo**; status zależy od wyniku | glibc 2.44, ten sam artefakt RPM |
| Fedora < 45 | **niewspierany** | brak testu i ryzyko glibc < 2.43 |
| RHEL / CentOS / Rocky / Alma 9/10 | **niewspierany** | glibc < 2.43; brak testu |
| openSUSE Leap 16.1 | **niewspierany** | glibc 2.40 |
| Debian / Ubuntu | **nie dotyczy** | używają DEB; RPM nie jest testowany |
| Arch Linux | **nie dotyczy** | nie używa RPM; wsparcie przez portable/AppImage |

**Nie deklarujemy wsparcia bez testu.** Kontener OCI dowodzi userspace
dystrybucji, ale **nie** dowodzi jądra dystrybucji — jądro jest współdzielone
z hostem CI. To ograniczenie jest przenoszone do raportu końcowego.

---

## 5. Plan testów

### 5.1. Build

1. Sprawdzenie wymaganych narzędzi: `rpm`, `rpmbuild`, `rpmspec`,
   `rpmdev-vercmp`, `readelf`, `file`, `sha256sum`, `tar`, `gzip`.
2. Odczyt wersji z `Cargo.toml`; mapowanie `1.3.0-alpha.1` →
   `1.3.0-0.1.alpha.1`; brak epoch.
3. Weryfikacja sidecara portable payloadu i `build_commit`.
4. Ekstrakcja payloadu do izolowanego katalogu; odrzucenie symlinków,
   wpisów poza `SPLogbook/`, nieoczekiwanych bibliotek.
5. Preflight ELF, dokładny RUNPATH, brak RPATH, GLIBC ≤ 2.43.
6. Budowa package-root z allowlisty; zakaz user data i sekretów.
7. Dwie niezależne budowy `rpmbuild -bb`; porównanie SHA-256, rozmiaru,
   listy plików, metadanych, trybów, właścicieli, scriptletów, payloadu,
   zależności i wyników inspekcji binariów.
8. Zapis `rpm-toolchain.json`, `rpm-info.txt`, `rpm-contents.txt`,
   `rpm-fields.txt`, `rpm-scripts.txt`, `rpm-requires.txt`, manifestu.

### 5.2. Kontrola metadanych

- `Name`, `Version`, `Release`, `Arch`, `Vendor`, `License`, `URL`,
  `Summary`, `Group`, `BuildHost`, `BuildTime`, `Size`.
- Brak epoch; brak `%{?dist}`.
- `Requires` dokładnie zgodne z kontraktem; brak `hamlib`.
- `rpm -qp --scripts`: tylko `%post` i `%postun`.
- `rpm -qp --dump`: tryby, UID/GID, brak symlinków.
- `rpm -qp --qf '%{PAYLOADFORMAT} %{PAYLOADCOMPRESSOR}'`:
  jawny, deterministyczny payload (`cpio`, `xz`).
- `Installed Size` zgodny z sumą rozmiarów plików.

### 5.3. Kontrola zawartości

- Dokładna lista wpisów równa manifestowi.
- Wymagane pliki obecne; żadnych dodatkowych.
- Brak `assets/`, `README.txt`, portable markera, `rigctld`, `rotctld`.
- Brak baz, WAL/SHM, konfiguracji, ADIF/Cabrillo, backupów, sekretów.
- Brak bibliotek systemowych (`libc`, `libm`, `libgcc_s`, `libstdc++`, ...).
- Tylko `libhamlib.so.4` w `lib/`.

### 5.4. Walidator

- `rpmlint` na artefakcie w Fedorze 45; 0 errors.
- Dopuszczalne wyłącznie tagi wynikające z dziedziczonego
  `OWNER-M3-H-03`: układ `/opt` i prywatny Hamlib z `$ORIGIN`.
- Każdy nowy tag/error jest raportowany; **nie wyciszamy** go bez decyzji.
- `desktop-file-validate` dla pliku desktop.
- `rpm -K`/`rpm --checksig` tylko jako kontrola integralności (bez wymogu
  podpisu; pakiet nie jest podpisywany w M3-I).

### 5.5. Instalacja i cykl życia

Kolejność:

1. fresh install bieżącego RPM,
2. upgrade z kontrolowanego fixture `1.3.0-0.1.alpha.0`,
3. downgrade do fixture,
4. re-upgrade,
5. reinstall (`rpm -U --replacepkgs`),
6. remove (`rpm -e`),
7. pełne usunięcie i sprawdzenie braku wpisu w RPM DB,
8. fresh install,
9. remove,
10. pełne usunięcie.

Przed/po każdej operacji: snapshot SHA-256 wszystkich plików w izolowanym
`HOME`, w tym XDG config/data/state/cache, QSO DB, WAL, SHM,
`station_config.json`, ADIF, Cabrillo, backupy i token. Porównanie musi być
identyczne. `rpm -e` nie jest upoważnieniem do kasowania danych użytkownika.

### 5.6. Testy funkcjonalne po instalacji

- `splogbook --version` = `SPLogbook 1.3.0-alpha.1`.
- `splogbook --build-info`: `Version`, `Commit`, `Target: linux-x86_64`.
- `--check-health-startup --health-proof-path`: proof `OK`.
- Uruchomienie launchera z `/`, `/tmp` i innego CWD.
- Brak modyfikacji `/opt/SPLogbook` przez uruchomienie.
- `ldd` na wszystkich trzech ELF: exit 0, brak `not found`.
- `desktop-file-validate` na zainstalowanym pliku.
- Ikona i desktop bajtowo zgodne z template'ami.

### 5.7. Negatywne fixture'y

Każdy fixture:

- odrzucony niezerowym exit code,
- bez komunikatu sukcesu,
- z kontrolowanym komunikatem przyczyny.

Zakres:

1. zła nazwa pakietu,
2. zła wersja,
3. zły release,
4. zła architektura,
5. zły `build_commit`,
6. zły `target`,
7. brak binarki głównej,
8. brak updatera,
9. brak launchera,
10. dodatkowy niedozwolony plik,
11. złe prawa launchera,
12. zły właściciel/grupa,
13. symlink zamiast pliku,
14. brak prywatnego Hamlib,
15. zły SONAME Hamlib,
16. Hamlib nie jest ELF/shared object,
17. niedozwolony RUNPATH/RPATH,
18. nierozwiązane biblioteki,
19. launcher z `sudo`,
20. launcher z `LD_LIBRARY_PATH`,
21. zły `Exec` desktop,
22. brak ikony,
23. brak markera,
24. zły marker (glibc > 2.43),
25. niedozwolony `%pre`,
26. niedozwolony scriptlet z siecią,
27. scriptlet usuwający dane użytkownika,
28. niezgodność manifestu,
29. niezgodność sumy kontrolnej,
30. uszkodzony/nie-RPM artefakt,
31. brak sidecara,
32. dodatkowy plik w `lib/`.

### 5.8. Determinizm

- Dwie niezależne budowy z tych samych wejść i tego samego
  `SOURCE_DATE_EPOCH`.
- Porównanie: SHA-256, rozmiar, lista plików, metadane RPM, tryby,
  właściciele, scriptlety, payload, zależności, inspekcja binariów.
- Brak identyczności bitowej = **brak deklaracji determinizmu**; raport
  wskazuje źródło różnicy albo bramka jest blokowana.

### 5.9. Regresja

- DEB (M3-H), portable tar.gz, AppImage, Windows.
- F-03 fail-closed, `security_guard`, pełny baseline.
- `cargo audit`, `cargo doc`, `cargo tree -d`, `cargo fmt --check`,
  `cargo clippy` na pinie `1.99.0`.
- `git diff --check`, YAML, JSON.

---

## 6. Rejestr ryzyk i decyzji właściciela

### 6.1. Decyzje odziedziczone

| Decyzja | Zastosowanie w M3-I |
|---|---|
| `OWNER-M3-H-03` — układ `/opt/SPLogbook` + launcher `/usr/bin/splogbook` | **dziedziczony bez zmian**; ten sam layout w RPM |
| `OWNER-M3-H-03` — prywatny `libhamlib.so.4` i `RUNPATH=$ORIGIN/lib` | **dziedziczony**; RPM wymaga węższego, dokładnego RUNPATH |
| `OWNER-M3-F-KERNEL-POLICY-OPTION-A` — minimum Linux 7.0 | marker RPM `resolved_minimum_kernel=7.0` |
| `OWNER-M3-F-GLIBC-POLICY-01` — glibc 2.43 | `Requires: glibc >= 2.43` i `GLIBC_2.43` |
| `OWNER-M3-F-XDG-DATA-01` — dane w XDG | brak zapisu do XDG w pakiecie; testy zachowania danych |
| `OWNER-M3-F-SIGNING-01` — brak publikacji bez podpisu | RPM nie jest publikowany ani podpisywany w M3-I |

**Brak nowych decyzji OWNER na wejściu.** Jeżeli walidator ujawni nowy
tag/error, którego nie da się naprawić bez zmiany zatwierdzonego kontraktu,
bramka zatrzyma się przed zmianą i poprosi o decyzję.

### 6.2. Różnice semantyczne DEB → RPM

| Obszar | DEB (M3-H) | RPM (M3-I) |
|---|---|---|
| Budowa | `dpkg-deb` | `rpmbuild -bb` |
| Wersja | `1.3.0~alpha.1-1` | `1.3.0-0.1.alpha.1` |
| Zależności | nazwy pakietów Ubuntu | capabilities soname/GLIBC |
| Instalacja | `dpkg -i` | `rpm -i` |
| Upgrade | `dpkg -i` | `rpm -U` |
| Downgrade | `dpkg -i` starszej wersji | `rpm -U --oldpackage` |
| Reinstall | `dpkg -i` | `rpm -U --replacepkgs` |
| Remove | `dpkg --remove` | `rpm -e` |
| Purge | `dpkg --purge` | **brak odpowiednika**; `rpm -e` |
| Skrypty | `postinst`/`postrm` | `%post`/`%postun` |
| Walidator | `lintian` | `rpmlint` |
| Manifest | `dpkg-deb --contents` | `rpm -qpl` + manifest SHA-256 |

Nie kopiujemy `postinst`/`postrm` jeden do jednego; implementujemy
odpowiedniki RPM z zachowaniem tego samego kontraktu bezpieczeństwa.

### 6.3. Dopuszczalne ostrzeżenia walidatora

Wstępnie dopuszczalne, **tylko** jeśli wynikają z dziedziczonego układu
`/opt/SPLogbook` i prywatnego Hamlib:

- `dir-or-file-in-opt`,
- `custom-library-search-path` / `binary-or-shlib-defines-rpath`.

Wszystkie inne warningi są raportowane. `rpmlint` musi mieć **0 errors**.
Jeżeli pojawi się nowy warning wymagający suppression, wymagana jest decyzja
OWNER; nie wyciszamy go „na zapas”.

### 6.4. HIL / PTT / RF

- Poza zakresem M3-I.
- Nie uruchamiamy 10 testów `#[ignore = "hardware-in-the-loop"]`.
- Nie wykonujemy PTT ani emisji RF.
- Testy RPM są wyłącznie software/userspace.

### 6.5. Publikacja

- RPM jest **artefaktem CI**.
- Brak nowego tagu, GitHub Release, publicznego RPM, nightly i kanału
  dystrybucji.
- `Publish Release Assets` pozostaje `skipped` na pushu `main`.
- Job publikacji dostanie trwałą asercję, że `.rpm` nie trafia do
  `release-staging`.

### 6.6. Ograniczenia OCI

- Kontener współdzieli jądro hosta; `uname -r` w OCI nie jest dowodem
  natywnego jądra dystrybucji.
- Dowód dotyczy userspace, ABI, instalatora i menedżera pakietów.
- Brak lokalnego Dockera/WSL oznacza, że natywne dowody pochodzą z CI;
  raport końcowy nie nazwie ich „lokalnym PASS”.

### 6.7. Ryzyka rezydualne

1. `rpmbuild` może wprowadzić niedeterministyczny cookie/nagłówek —
   wtedy determinizm jest **FAIL**, nie „z zastrzeżeniem”.
2. Różnice `rpmlint` Fedora vs openSUSE — walidator referencyjny to Fedora;
   openSUSE może mieć inne warningi, raportowane osobno.
3. Różnice wersji RPM między Fedorą a openSUSE mogą zmienić `rpmlib(...)`
   i payload; porównujemy metadane, nie tylko SHA.
4. `rpm -U --oldpackage` może wymagać `--nodeps` w specyficznych stanach;
   użycie `--nodeps` w teście cyklu życia jest zabronione.
5. Brak podpisu RPM — zgodne z M3-H; publikacja i tak wyłączona.
6. HIL/PTT/RF niewykonane — poza zakresem.
7. OpenSUSE może nie mieć dokładnie tych samych capability soname;
   wtedy status „testowany kompatybilnościowo” zmienia się na „niewspierany
   w M3-I”, bez fałszywego PASS.

---

## 7. Plan implementacji i commitów

### 7.1. Pliki nowe

- `installer/linux/rpm/README.md`
- `installer/linux/rpm/splogbook.spec`
- `installer/linux/rpm/launcher`
- `installer/linux/rpm/splogbook.desktop`
- `installer/linux/rpm/templates/README.RPM`
- `installer/linux/rpm/templates/LGPL-2.1.txt`
- `scripts/build_linux_rpm.sh`
- `scripts/verify_linux_rpm.sh`
- `scripts/verify_linux_rpm_negative_fixture.sh`
- `scripts/linux_rpm_manifest.sh`
- `scripts/build_linux_rpm_fixture.sh`
- `tests/linux_rpm_packaging_test.rs`
- raport przedimplementacyjny i końcowy

### 7.2. Pliki modyfikowane

- `.github/workflows/build-and-release.yml` — joby RPM, macierz Fedora/openSUSE,
  asercja braku publikacji RPM.
- `.github/workflows/ci.yml` — wyłącznie jeśli wymagane przez pin/komentarze
  liczników; bez zmiany toolchainu.
- `tests/security_guard.rs` — `m3_i_rpm_integration_guard`.
- `.gitignore` — `*.rpm`, `*.rpm.sha256`, `*.rpm.manifest`.
- dokumentacja stanu: `AGENTS.md`, `docs/NEXT_WORK_GATE.md`,
  `docs/PROJECT_CURRENT_STATE.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`,
  `docs/DOCUMENTATION_INDEX.md`, `docs/DEEPSEEK_HARNESS_HANDOFF.md`.

### 7.3. Commity

1. `docs(linux): add M3-I RPM pre-implementation report`
2. `feat(linux): add verified RPM packaging pipeline`
3. ewentualne `fix(linux): ...` ujawnione przez testy (bez squasha)
4. `docs(linux): close M3-I RPM integration gate`

Push wyłącznie fast-forward; bez rebase, merge commitów i force-push.

---

## 8. Kryteria wejścia do implementacji

- [x] `HEAD == main == origin/main == 9427e57...`
- [x] ahead/behind `0/0`
- [x] tracked worktree clean
- [x] `scratch/` niecommitowany
- [x] M3-H `COMPLETED_DEB_INTEGRATION_VERIFIED`
- [x] M3-J `NOT_STARTED`
- [x] brak nowego tagu/release
- [x] `Publish Release Assets` skipped na `main`
- [x] Rust 1.99.0 w CI
- [x] GO OWNER `OWNER_GO_M3_I_RPM=true`
- [x] analiza istniejących pipeline'ów i kontraktów zakończona
- [x] brak potrzeby zmiany `src/**`
- [x] plan testów i macierz systemów zdefiniowane
- [x] ryzyka i decyzje OWNER zapisane

**Status raportu:** `COMPLETE_PRE_IMPLEMENTATION_ANALYSIS`.
**Następny krok:** commit dokumentacyjny, następnie implementacja M3-I.
**M3-J:** `NOT_STARTED` — nie rozpoczynać.
