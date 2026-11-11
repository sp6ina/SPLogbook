# SPLogbook Linux AppImage Packaging (M3-G)

---

## 1. Przeznaczenie i zakres

Katalog `installer/linux/appimage/` zawiera trwałe definicje pakunku AppImage
(`SPLogbook-Linux-x86_64.AppImage`) dla SPLogbook Linux x86_64. Bramka:
`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`.

Polityka wsparcia pozostaje niezmieniona (M3-F): **kernel >= 7.0**, **glibc 2.43**,
cztery wspierane dystrybucje (Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed).
AppImage nie zmienia tej polityki i nie podnosi minimum.

---

## 2. Zatwierdzony układ AppDir

```text
AppDir/
├── AppRun                                  # POSIX sh, 0755 (installer/linux/appimage/AppRun)
├── splogbook.desktop                       # kanoniczny template, 0644
├── splogbook.png                           # assets/icon.png (256x256), 0644
├── .DirIcon                                # regularny plik, identyczne bajty jak splogbook.png
└── usr/
    ├── bin/
    │   ├── SPLogbook                       # 0755, RUNPATH=$ORIGIN/../lib
    │   └── splogbook-updater               # 0755, RUNPATH=$ORIGIN/../lib
    ├── lib/
    │   └── libhamlib.so.4                  # jedyna bundlowana biblioteka, 0755
    └── share/
        ├── applications/
        │   └── splogbook.desktop           # identyczne bajty jak AppDir/splogbook.desktop
        ├── icons/
        │   └── hicolor/
        │       └── 256x256/
        │           └── apps/
        │               └── splogbook.png   # identyczne bajty jak AppDir/splogbook.png
        ├── doc/
        │   └── splogbook/
        │       ├── LICENSE
        │       ├── README.txt
        │       └── THIRD_PARTY_LICENSES/
        └── splogbook/
            └── .splogbook-appimage-marker  # deterministyczny JSON, 0644
```

Zasady:

- brak symlinków i hardlinków w całym AppDir;
- brak metadanych AppStream (niezatwierdzone przez DESIGN) — `appimagetool` działa z `--no-appstream`;
- brak danych użytkownika (`*.db`, `*.db-wal`, `*.db-shm`, `*.adi`, `*.adif`, `*.cbr`,
  `station_config.json`, tokenów, kluczy);
- brak bitów `setuid`/`setgid` i uprawnień world-writable.

---

## 3. Polityka bibliotek

Bundlowana jest **wyłącznie** `libhamlib.so.4` (pochodząca z zweryfikowanego
portable tar.gz z tego samego commita). Wszystkie pozostałe biblioteki pozostają
wymaganiami runtime dystrybucji:

- glibc, loader dynamiczny, `libm`, `libgcc_s`, `libstdc++`;
- ALSA, X11, Xcursor, Xrandr, Xi, xkbcommon, xkbcommon-x11;
- OpenGL, fontconfig, Wayland, GTK 3, libusb, libudev;
- sterowniki i pluginy jądra.

Zakazane jest automatyczne kopiowanie wszystkiego z `ldd` oraz maskowanie
zależności przez `LD_LIBRARY_PATH`.

---

## 4. AppRun

`AppRun` jest skryptem POSIX `sh`:

```sh
set -eu
APPDIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec "${APPDIR}/usr/bin/SPLogbook" "$@"
```

Nie używa `sudo`, `pkexec`, `doas`, nie zmienia `HOME` ani `XDG_*`, nie ustawia
`LD_LIBRARY_PATH` i nie zapisuje do AppDir.

---

## 5. Marker i dane użytkownika

Marker `usr/share/splogbook/.splogbook-appimage-marker` zawiera co najmniej:
`package_version`, `build_commit`, `installation_type=LinuxAppImage`,
`resolved_minimum_kernel=7.0`, `maximum_glibc=2.43`, `artifact_format=AppImage`,
`architecture=x86_64`, `appimagetool_channel=continuous`,
`appimagetool_source_commit`, `appimagetool_sha256`, `runtime_release_or_commit`,
`runtime_sha256`. Jest deterministyczny, bez lokalnego czasu, nazwy użytkownika,
ścieżek hosta, tokenów i sekretów.

Dane użytkownika pozostają poza obrazem, w ścieżkach XDG:

- konfiguracja: `$XDG_CONFIG_HOME/splogbook`
- dane i bazy: `$XDG_DATA_HOME/splogbook`
- stan: `$XDG_STATE_HOME/splogbook`
- cache: `$XDG_CACHE_HOME/splogbook`

---

## 6. Narzędzia budowy

- `appimagetool`: najnowszy **oficjalny** Continuous build z
  `https://github.com/AppImage/appimagetool`, rozwiązany raz, zweryfikowany przez
  SHA-256 i przypięty na czas bramki (`scripts/install_appimage_toolchain.sh`).
- type-2 runtime: najnowszy zweryfikowany oficjalny release z
  `https://github.com/AppImage/type2-runtime`, przekazany jawnie przez `--runtime-file`
  (appimagetool nie pobiera runtime w tle).

Narzędzia **nie są** commitowane do repozytorium.

---

## 7. Skrypty

| Skrypt | Rola |
|---|---|
| `scripts/install_appimage_toolchain.sh` | rozwiązanie + weryfikacja + pin `appimagetool` i runtime |
| `scripts/build_linux_appimage.sh` | budowa AppDir i AppImage z zweryfikowanego portable tar.gz |
| `scripts/verify_appimage.sh` | fail-closed weryfikator obrazu i AppDir |
| `scripts/verify_appimage_negative_fixture.sh` | dowód odrzucenia 12 mutacji |

Publikacja release pozostaje poza zakresem tej bramki.
