# SPLogbook Linux x86_64 binary RPM (M3-I)

## 1. Scope

This directory contains the reviewed sources for the **binary** RPM package
`SPLogbook-Linux-x86_64-<version>.rpm` (canonical CI alias
`SPLogbook-Linux-x86_64.rpm`). The package is assembled with `rpmbuild -bb`
from the already verified portable payload. M3-I deliberately does **not**
create a source RPM, a COPR repository, a DNF/Zypper repository or a package
signature.

The RPM is a **CI artifact only**. It is never added to a public GitHub release
and no tag or release is created by this gate.

## 2. Supported systems

The package targets x86_64 systems with glibc **2.43** (`GLIBC_2.43`) or newer.

| System | Status in M3-I |
|---|---|
| Fedora Linux 45 (GA) | **native reference**, full build + lifecycle test |
| openSUSE Tumbleweed | **compatibility test**, same artifact installed and exercised |
| Fedora < 45 | not supported / not tested |
| RHEL, CentOS, Rocky, Alma 9/10 | not supported (glibc < 2.43) |
| openSUSE Leap 16.1 | not supported (glibc 2.40) |
| Debian/Ubuntu | not applicable (use DEB) |
| Arch Linux | not applicable (not RPM) |

The OCI test proves the distribution userspace, not the distribution kernel:
the container shares the CI host kernel. `uname -r` inside OCI is not native
kernel evidence.

## 3. Package identity

| Field | Value |
|---|---|
| `Name` | `splogbook` |
| `Version` | `1.3.0` |
| `Release` | `0.1.alpha.1` (no `%{?dist}`, no epoch) |
| `Architecture` | `x86_64` |
| `Vendor` | `SPLogbook` |
| `Packager` | `Mariusz Woźniak (SP6INA) <contact@splogbook.org>` |
| `License` | `GPL-3.0-or-later AND LGPL-2.1-or-later` |
| `URL` | `https://github.com/sp6ina/SPLogbook` |
| `Group` | `Applications/Communications` |
| `Summary` | `Amateur radio logging and station automation workstation` |

Version translation: SemVer `1.3.0-alpha.1` becomes RPM
`1.3.0-0.1.alpha.1`. The `0.*` release sorts the prerelease **before**
`1.3.0-1`; the builder proves the ordering with `rpmdev-vercmp`.

## 4. Filesystem layout

```text
/opt/SPLogbook/SPLogbook
/opt/SPLogbook/splogbook-updater
/opt/SPLogbook/lib/libhamlib.so.4
/opt/SPLogbook/locales/
/opt/SPLogbook/LICENSE
/opt/SPLogbook/THIRD_PARTY_LICENSES/
/opt/SPLogbook/.splogbook-rpm-marker

/usr/bin/splogbook
/usr/share/applications/splogbook.desktop
/usr/share/icons/hicolor/256x256/apps/splogbook.png
/usr/share/doc/splogbook/README.RPM
/usr/share/licenses/splogbook/LICENSE
/usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt
```

Only `libhamlib.so.4` is bundled. It is a shared library, not an executable,
and is installed with mode `0644`; only `SPLogbook`, `splogbook-updater` and
the launcher are `0755`. All entries are `root:root`.

The package deliberately does not copy `assets/`, `README.txt`,
`.splogbook-linux-portable-marker`, `rigctld`, `rotctld`, databases,
configuration, exports, backups or secrets.

## 5. Dependencies

`AutoReqProv: no` is used; the contract is explicit and distribution-neutral.
`Requires` uses SONAME/GLIBC capabilities, not Fedora-only package names:

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

`libhamlib.so.4` is bundled and therefore must **not** appear in `Requires`.
No `Recommends`, `Suggests`, `Supplements`, `Enhances`, `Conflicts`,
`Obsoletes` or `Provides` are declared beyond RPM's automatic
`splogbook = <evr>`.

## 6. RUNPATH and private Hamlib

- `SPLogbook` and `splogbook-updater`: exactly `RUNPATH=$ORIGIN/lib`.
- `libhamlib.so.4`: exactly `RUNPATH=$ORIGIN`.
- Legacy `RPATH`, absolute paths and `LD_LIBRARY_PATH` are rejected.
- The bundled library is `libhamlib.so.4` (SONAME check) and must be an
  ELF64 x86-64 shared object.
- `ldd` must resolve every library after installation and report no `not found`.
- Maximum required GLIBC symbol is `GLIBC_2.43`.

## 7. Scriptlets

Only `%post` and `%postun` exist. Both are POSIX `/bin/sh`, non-interactive and
idempotent. They refresh `update-desktop-database` and
`gtk-update-icon-cache` only when the official helper is already present.
They never use the network, `sudo`, `pkexec`, `doas`, `systemctl`, `crontab`,
`HOME`, XDG variables, GUI, PTT or RF.

There are no `%pre`, `%preun`, `%pretrans`, `%posttrans`, `%trigger*`,
`%verifyscript`, `%config` or `%ghost` entries.

## 8. User data

The package never installs user data into `/opt/SPLogbook`. SPLogbook keeps its
data in `$XDG_CONFIG_HOME/splogbook`, `$XDG_DATA_HOME/splogbook`,
`$XDG_STATE_HOME/splogbook` and `$XDG_CACHE_HOME/splogbook` (with the standard
`$HOME` fallbacks). Install, upgrade, downgrade, reinstall and `rpm -e` must
not modify or delete QSO databases, WAL/SHM files, `station_config.json`,
ADIF/Cabrillo exports, backups, device configuration or secrets.

RPM has no `purge` equivalent: full removal is `rpm -e`, and it removes only
package-owned files.

## 9. Marker

`/opt/SPLogbook/.splogbook-rpm-marker` is deterministic JSON:

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

The existing updater already detects `.splogbook-rpm-marker` as
`InstallationType::LinuxRpm`; M3-I does not change updater code.

## 10. Build and verification

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)" \
BUILD_IMAGE="fedora:45@sha256:..." \
BUILD_IMAGE_DIGEST="sha256:..." \
  bash scripts/build_linux_rpm.sh dist dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz
```

The builder rebuilds the package twice and fails unless both builds have the
same SHA-256. It records `rpm --version`, `rpmbuild --version`,
`rpmspec --version`, the distribution and the image digest in
`dist/rpm-toolchain.json`, and writes `rpm-info.txt`, `rpm-contents.txt`,
`rpm-fields.txt`, `rpm-scripts.txt`, `rpm-requires.txt` and
`<artifact>.rpm.manifest` next to the artifact. It never installs the RPM.

Static + native verification:

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
EXPECTED_SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)" \
EXPECTED_ICON_SHA256="$(sha256sum assets/icon.png | awk '{print $1}')" \
EXPECTED_DESKTOP_SHA256="$(sha256sum installer/linux/rpm/splogbook.desktop | awk '{print $1}')" \
  sh scripts/verify_linux_rpm.sh dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm
```

The verifier checks the checksum sidecar, manifest, package type, metadata,
`Requires`, `SIZE`, exact layout, modes and ownership, absence of
symlinks/setuid/setgid/world-writable entries, absence of user data and
secrets, ELF identity, RUNPATH, GLIBC <= 2.43, `ldd`, launcher, desktop entry,
icon, marker, `build_commit`, `rpmlint` (0 errors), then performs native
install, upgrade, downgrade, re-upgrade, reinstall and `rpm -e` with isolated
user-data fixtures. It prints `RPM VERIFICATION COMPLETED SUCCESSFULLY` only at
the end.

`SPLOG_RPM_INSPECTION_ONLY=1` stops after the static checks and prints
`RPM INSPECTION COMPLETED SUCCESSFULLY`; it is used only by the negative
fixture harness and is never evidence of installation.

## 11. rpmlint policy

Fedora's `rpmlint` is the reference validator. `dir-or-file-in-opt` and
`custom-library-search-path` are suppressed because they are a direct
consequence of the approved `/opt/SPLogbook` layout and the private bundled
Hamlib (`OWNER-M3-H-03`). No other tag is suppressed. The verifier requires
**0 errors** and reports every warning. A new warning that would need a
suppression requires an explicit owner decision.

## 12. Negative fixtures

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
  sh scripts/verify_linux_rpm_negative_fixture.sh \
    scripts/verify_linux_rpm.sh dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm
```

The harness mutates disposable copies of the real RPM and requires every
mutation to be rejected with a non-zero verifier exit and without a success
message. It covers package-integrity, metadata, payload, Hamlib, RUNPATH,
launcher, desktop, scriptlet, user-data, manifest and checksum mutations.

## 13. CI

`.github/workflows/build-and-release.yml` builds the RPM once in the pinned
Fedora 45 image from the verified portable artifact, uploads the
`SPLogbook-Linux-x86_64-rpm` CI artifact, verifies it natively in Fedora 45
and openSUSE Tumbleweed, and runs the negative fixture job in Fedora 45. The
`publish-release` job remains tag-gated, does not copy any RPM and contains an
explicit assertion that no `.rpm` is staged for release.
