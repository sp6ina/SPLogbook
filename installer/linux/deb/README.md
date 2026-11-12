# SPLogbook Linux amd64 binary DEB (M3-H)

## 1. Scope

This directory contains the reviewed sources for the **binary** Debian package
`SPLogbook-Linux-amd64-<version>.deb` (canonical CI alias
`SPLogbook-Linux-amd64.deb`). The package is assembled with `dpkg-deb` from the
already verified portable payload; M3-H deliberately does **not** create a
Debian source package (`debian/rules`, `debian/changelog`, `.dsc`, `.changes`,
`orig.tar`). No APT repository and no package signature are produced in M3-H.

The DEB is a **CI artifact only**. It is never added to a public GitHub release
and no tag or release is created by this gate.

## 2. Supported system

The package targets 64-bit systems with glibc **2.43** (`GLIBC_2.43`) or newer.
The reference and only tested distribution is **Ubuntu 26.04 LTS**. Debian 13.7
(glibc 2.41) and openSUSE Leap 16.1 (glibc 2.40) do not satisfy the binary ABI
requirement and are not supported. RPM-based and Arch-based distributions must
use their own package formats; `dpkg -i` is not tested there.

## 3. Package identity

| Field | Value |
|---|---|
| `Package` | `splogbook` |
| `Version` | `1.3.0~alpha.1-1` (no epoch) |
| `Architecture` | `amd64` |
| `Section` / `Priority` | `hamradio` / `optional` |
| `Maintainer` | `Mariusz Woźniak (SP6INA) <contact@splogbook.org>` |
| `Homepage` | `https://github.com/sp6ina/SPLogbook` |
| `Standards-Version` | 4.7.4 (latest Debian Policy Manual) |

Version translation: the development SemVer `1.3.0-alpha.1` becomes
`1.3.0~alpha.1-1`. The tilde sorts the prerelease **before** `1.3.0`, and the
`-1` is the package revision. The builder and the verifier prove the ordering
with `dpkg --validate-version` and `dpkg --compare-versions`.

## 4. Filesystem layout

```text
/opt/SPLogbook/SPLogbook
/opt/SPLogbook/splogbook-updater
/opt/SPLogbook/lib/libhamlib.so.4
/opt/SPLogbook/locales/
/opt/SPLogbook/LICENSE
/opt/SPLogbook/THIRD_PARTY_LICENSES/
/opt/SPLogbook/.splogbook-deb-marker

/usr/bin/splogbook                            (POSIX sh launcher, 0755)
/usr/share/applications/splogbook.desktop
/usr/share/icons/hicolor/256x256/apps/splogbook.png
/usr/share/doc/splogbook/README.Debian
/usr/share/doc/splogbook/copyright
/usr/share/doc/splogbook/changelog.Debian.gz
```

Only `libhamlib.so.4` is bundled. It is a shared library, not an executable, and
is installed with mode `0644`; only `SPLogbook`, `splogbook-updater` and the
launcher are `0755`. The package ships the canonical LGPL-2.1
text for the bundled Hamlib as
`/opt/SPLogbook/THIRD_PARTY_LICENSES/HAMLIB_LGPL-2.1.txt`. The dynamic loader,
glibc, libm, libgcc_s, libstdc++, ALSA, X11, Xcursor, Xrandr, Xi, xkbcommon,
xkbcommon-x11, OpenGL, fontconfig, Wayland, GTK 3, libusb and libudev are
**not** bundled.

The launcher is a regular file, not a symlink:

```sh
#!/bin/sh
set -eu
exec /opt/SPLogbook/SPLogbook "$@"
```

It does not use `sudo`, `pkexec`, `doas`, `LD_LIBRARY_PATH`, `HOME` or XDG
overrides, does not write and does not mask the exit status.

## 5. Dependency mapping

`Depends` is derived from the real ELF/runtime requirements and verified in CI
with `readelf`, `ldd`, `dpkg -S` and `apt-cache` against the official Ubuntu
26.04 repositories. Every SONAME maps to exactly one package:

| SONAME | Package | Source of mapping |
|---|---|---|
| `libc.so.6`, `libm.so.6`, `libdl.so.2`, `libpthread.so.0`, `librt.so.1` | `libc6` (>= 2.43) | ELF `DT_NEEDED` + `GLIBC_2.43` policy |
| `libgcc_s.so.1` | `libgcc-s1` | ELF `DT_NEEDED` |
| `libasound.so.2` | `libasound2t64` | ELF `DT_NEEDED` (audio) |
| `libX11.so.6` | `libx11-6` | winit/X11 runtime |
| `libXcursor.so.1` | `libxcursor1` | winit/X11 runtime |
| `libXrandr.so.2` | `libxrandr2` | winit/X11 runtime |
| `libXi.so.6` | `libxi6` | winit/X11 runtime |
| `libxkbcommon.so.0` | `libxkbcommon0` | winit/X11 runtime |
| `libxkbcommon-x11.so.0` | `libxkbcommon-x11-0` | winit/X11 runtime |
| `libGL.so.1` | `libgl1` | eframe/glow runtime |
| `libfontconfig.so.1` | `libfontconfig1` | GUI text/font runtime |
| `libwayland-client.so.0` | `libwayland-client0` | winit/Wayland runtime |
| `libwayland-cursor.so.0` | `libwayland-cursor0` | winit/Wayland runtime |
| `libwayland-egl.so.1` | `libwayland-egl1` | winit/Wayland runtime |
| `libgtk-3.so.0` | `libgtk-3-0t64` | rfd/GTK dialog runtime |
| `libusb-1.0.so.0` | `libusb-1.0-0` | Hamlib USB runtime |
| `libudev.so.1` | `libudev1` | Hamlib/device runtime |

`libhamlib.so.4` is bundled in `/opt/SPLogbook/lib` and therefore must **not**
appear in `Depends`. No `Recommends`, `Suggests`, `Breaks`, `Conflicts`,
`Provides` or `Replaces` are declared in M3-H.

The complete list matches the runtime sonames that the M3-F/M3-G verification
already installs from the official repository in the Ubuntu 26.04 profile.

## 6. Maintainer scripts

Only `postinst` and `postrm` exist. Both are `#!/bin/sh` + `set -e`, are
idempotent and non-interactive, and only refresh the desktop database and the
icon cache when the official helper tool is already installed. They never use
the network, `sudo`, `pkexec`, `doas`, `HOME`, XDG variables, GUI, updater,
debconf, systemd, cron, PTT or RF.

There are no conffiles, no `preinst` and no `prerm`.

## 7. User data

The package never installs user data into `/opt/SPLogbook`. SPLogbook keeps its
data in `$XDG_CONFIG_HOME/splogbook`, `$XDG_DATA_HOME/splogbook`,
`$XDG_STATE_HOME/splogbook` and `$XDG_CACHE_HOME/splogbook` (with the standard
`$HOME` fallbacks). Install, upgrade, downgrade, reinstall, remove and purge
must not modify or delete QSO databases, WAL/SHM files, `station_config.json`,
ADIF/Cabrillo exports, backups, device configuration or secrets.

## 8. Marker

`/opt/SPLogbook/.splogbook-deb-marker` is deterministic JSON:

```json
{
  "marker_version": "1.0",
  "installation_type": "LinuxDeb",
  "management_mode": "SystemManaged",
  "package_version": "1.3.0-alpha.1",
  "debian_version": "1.3.0~alpha.1-1",
  "build_commit": "<40 hex>",
  "resolved_minimum_kernel": "7.0",
  "maximum_glibc": "2.43",
  "artifact_format": "deb",
  "architecture": "amd64",
  "package_name": "splogbook",
  "created_by": "scripts/build_linux_deb.sh"
}
```

The existing updater already detects `.splogbook-deb-marker` as
`InstallationType::LinuxDeb`; M3-H does not change updater code.

## 9. Build and verification

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)" \
BUILD_IMAGE="ubuntu:26.04@sha256:..." \
BUILD_IMAGE_DIGEST="sha256:..." \
  bash scripts/build_linux_deb.sh dist dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz
```

The builder rebuilds the package twice and fails unless both builds have the
same SHA-256. It records `dpkg --version`, `dpkg-deb --version`,
`dpkg-architecture`, the distribution and the image digest in
`dist/deb-toolchain.json`, and writes `deb-info.txt`, `deb-contents.txt` and
`deb-fields.txt` next to the artifact. It never installs the DEB.

Static + native verification:

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
EXPECTED_ICON_SHA256="$(sha256sum assets/icon.png | awk '{print $1}')" \
EXPECTED_DESKTOP_SHA256="$(sha256sum installer/linux/deb/splogbook.desktop | awk '{print $1}')" \
  sh scripts/verify_linux_deb.sh dist/SPLogbook-Linux-amd64-1.3.0-alpha.1.deb
```

The verifier checks the checksum sidecar, package type, control fields,
`Depends`, `Installed-Size`, the exact layout, modes and ownership, absence of
symlinks/setuid/setgid/world-writable entries, absence of user data and secrets,
ELF identity, RUNPATH, GLIBC <= 2.43, `ldd`, launcher, desktop entry, icon,
marker, `build_commit`, lintian (0 errors), then performs native install,
upgrade, downgrade, reinstall, remove and purge with isolated user-data
fixtures. It prints `DEB VERIFICATION COMPLETED SUCCESSFULLY` only at the end.

Lintian reports `dir-or-file-in-opt` for the `/opt/SPLogbook` layout and
`custom-library-search-path` for the relative `$ORIGIN/lib` RUNPATH required by
the private bundled `libhamlib.so.4`. Both are explicitly approved by
`OWNER-M3-H-03`; the verifier therefore suppresses **exactly these two tags**
and prints the suppression in the evidence. No other lintian tag is suppressed.

`SPLOG_DEB_INSPECTION_ONLY=1` stops after the static checks and prints
`DEB INSPECTION COMPLETED SUCCESSFULLY`; it is used only by the negative
fixture harness and is never evidence of installation.

## 10. Negative fixtures

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
  sh scripts/verify_linux_deb_negative_fixture.sh \
    scripts/verify_linux_deb.sh dist/SPLogbook-Linux-amd64-1.3.0-alpha.1.deb
```

The harness mutates disposable copies of the real DEB and requires every
mutation to be rejected with a non-zero verifier exit and without a success
message. It covers missing/broken checksum, truncated archive, missing control,
wrong architecture/depends/installed-size/maintainer, missing binary/updater/
library/launcher/icon/marker, wrong launcher mode, `sudo`/`LD_LIBRARY_PATH` in
the launcher, dangerous desktop `Exec`, wrong `build_commit`, GLIBC above 2.43,
user data, setuid, world-writable files, absolute/traversal symlinks and
maintainer scripts touching `HOME` or the network.

## 11. CI

`.github/workflows/build-and-release.yml` builds the DEB once in the pinned
`ubuntu:26.04` image from the verified portable artifact, uploads the
`SPLogbook-Linux-amd64-deb` CI artifact, verifies the package natively in a
disposable Ubuntu 26.04 container and runs the negative fixture job. The
`publish-release` job remains tag-gated, does not copy any DEB and contains an
explicit assertion that no `.deb` is staged for release.
