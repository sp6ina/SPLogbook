# SPLogbook

<p align="center">
  <strong>Amateur radio logbook and station operating console</strong><br>
  <em>Written in Rust with an immediate-mode <code>egui</code> interface</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Version-1.3.0-brightgreen.svg" alt="Version 1.3.0">
  <img src="https://img.shields.io/badge/Language-Rust_2024-orange.svg" alt="Rust 2024">
  <img src="https://img.shields.io/badge/GUI-egui_%2F_eframe-blueviolet.svg" alt="egui / eframe">
  <img src="https://img.shields.io/badge/License-GPLv3-blue.svg" alt="License GPLv3">
  <img src="https://img.shields.io/badge/Platform-Windows_%7C_GNU%2FLinux-blue.svg" alt="Platform">
  <a href="https://github.com/sp6ina/SPLogbook/actions"><img src="https://github.com/sp6ina/SPLogbook/actions/workflows/build-and-release.yml/badge.svg" alt="Build and Release"></a>
  <img src="https://img.shields.io/badge/Tests-3705%2F3705_Passed-brightgreen.svg" alt="Tests">
  <img src="https://img.shields.io/badge/i18n-7_Languages-cyan.svg" alt="7 languages">
</p>

<img width="1917" height="991" alt="SPLogbook main window" src="https://github.com/user-attachments/assets/465c5eda-739d-4191-ad17-c6cc89a39347" />

SPLogbook is a cross-platform amateur radio logbook and station operating console for
Windows and GNU/Linux. It combines QSO logging with transceiver and rotator control (CAT),
digital-mode bridges, DX cluster, propagation, satellites, contests, award tracking and
online QSL services in one dockable workspace.

- **Licence:** GNU General Public License v3.0 or later
- **Website:** [splogbook.org](https://splogbook.org)
- **Languages:** English, Polish, German, Spanish, French, Italian, Russian
- **Current stable release:** `1.3.0` (2026-11-20). See [`CHANGELOG.md`](CHANGELOG.md).

---

## Contents

1. [What is new in 1.3.0](#what-is-new-in-130)
2. [Operating features](#operating-features)
3. [Keyboard shortcuts](#keyboard-shortcuts)
4. [Download and install](#download-and-install)
5. [Upgrading from v1.2.0](#upgrading-from-v120)
6. [Data, configuration and credentials](#data-configuration-and-credentials)
7. [Local REST API](#local-rest-api)
8. [Building from source and running the tests](#building-from-source-and-running-the-tests)
9. [Documentation](#documentation)
10. [Contributing](#contributing)
11. [Licence, data attribution and security](#licence-data-attribution-and-security)
12. [Author and support](#author-and-support)

---

## What is new in 1.3.0

The full list is in [`CHANGELOG.md`](CHANGELOG.md). The main user-visible changes:

### Interface and ergonomics

- Shared design tokens (spacing, radii, type scale, monospace callsigns and frequencies)
  used by every panel.
- Three themes, **Operator Dark** (default), **Daylight** and **High Contrast**
  (colour-blind safe), with a palette checked against WCAG contrast ratios. `Ctrl+T`
  cycles through them.
- A typed notification queue: info, success, warning and error toasts with severity icons,
  time-to-live, de-duplication and dismiss.
- A status bar with a clear **TX/RX** indicator, plus CAT, cluster and UTC status.
- Keyboard workflow in the QSO form: `Enter` saves, `Esc` clears, a stable `Tab` order,
  and a contest mode with its own save and wipe keys. Clicking a spot focuses the QSO form.
- Every panel can be undocked into its own window, for example on a second monitor, and
  docked back, using consistent vector icons.

### World map

- Land outlines from **Natural Earth 110m Land** (public domain), embedded at build time.
- A smooth day/night shadow with a twilight band (grayline), and sun and moon markers.
- An optional Maidenhead field grid.
- A great-circle short path to the DX station, split correctly at the antimeridian.
- Pan and zoom stay within the map. The map keeps a 2:1 aspect ratio with letterboxing in
  docked tiles and in floating windows.

### Audio (AF) waterfall

- Built on `cpal` (audio capture) and `rustfft` (FFT sizes 512 to 4096).
- A spectrum plot above the waterfall, sharing one Hz axis, with a dB grid.
- Peak Hold, five colour palettes and Auto-Level.
- A crosshair that reads out AF frequency, level and the resulting RF frequency derived
  from the VFO and the mode, plus passband and target-tone markers.
- **Click-to-QSY:** clicking a signal retunes the VFO through the existing CAT path. Without
  CAT, the click only moves the marker.

### Reference data and packaging

- Country files, DXCC entity attributes, regional rules and club memberships are loaded from
  the user data directory and managed from the application, with atomic installs and
  rollback. This data is not bundled with the application.
- Time handling moved from `chrono` to `jiff`. LoTW and eQSL received dates are never
  invented.
- Windows Inno Setup and MSI installers are built, verified and published by the release
  pipeline. DEB, RPM, AppImage and Flatpak packaging scripts are verified in CI (see
  [Download and install](#download-and-install)).

---

## Operating features

| Area | What SPLogbook does |
|---|---|
| **Logbook** | SQLite in WAL mode (bundled SQLite, never the system library), multiple journals, a virtualised and paginated log table with sorting, filtering and column presets, undo/redo of deletions, duplicate finder, ADIF 3.1.8 import/export, ADX and CSV export. |
| **CAT and rotators** | Native CAT for Icom CI-V, Kenwood, Yaesu, Elecraft and FlexRadio (SmartSDR), plus Hamlib `rigctld`, FLRig (XML-RPC) and TCI (WebSocket). Multi-radio core with a single-TX interlock. Hamlib `rotctld` rotator control. A Hamlib-compatible CAT sharing server for other applications. WinKeyer CW keying. |
| **PTT safety** | PTT is never bound to a single key. `Ctrl+Shift+F7` requests PTT ON and `Ctrl+F7` requests PTT OFF. Both are blocked during text editing, IME composition, in modal windows and without window focus; key repeat and key release never create a TX intent. |
| **Digital modes** | WSJT-X / JTDX UDP bridge (port 2237), JS8Call TCP API (port 2442), FLDigi XML-RPC, N1MM Logger+ UDP broadcast (port 12060). |
| **DX cluster** | Telnet cluster client with spot de-duplication, ATNO / new-band / new-mode highlighting, audible alerts, a band map and click-to-tune via CAT. |
| **Propagation and solar** | Built-in VOACAP-lite HF model (MUF, LUF, FOT, reliability and band-opening matrix) fed by space weather data from HamQTH (SFI, SSN, A and K indices, X-ray flux); grayline status. |
| **Satellites** | SGP4 orbit propagation from TLE data, pass tracking, Doppler correction through CAT and az/el rotator steering. |
| **Contests** | Contest rules with exchange parsing, live score, rate meter, multiplier matrix and dupe warnings; Cabrillo 3.0 export whose claimed score matches the on-screen score. |
| **Awards** | DXCC, WAZ, WAS, WAE, IOTA, SOTA, POTA, WWFF, RDA, PGA and SP award tracking in an awards matrix. |
| **Online services** | LoTW (via TQSL), eQSL, Club Log, QRZ.com, HamQTH, HRDLog, Cloudlog, PSK Reporter and WSPR, with a shared upload scheduler (offline queue, backoff, rate limits) and a callbook cache. |
| **Multi-Op LAN sync** | Host/peer QSO replication over LAN or VPN (TCP port 7373), encrypted with XChaCha20-Poly1305 using an Argon2id-derived key, with replay protection and bidirectional catch-up of the active journal. New QSOs only; edits and deletions are not propagated. |
| **QSL** | QSL manager, QSL card designer and A4 vector PDF label sheets (Avery layouts). |
| **Extensibility** | Sandboxed Rhai plugins (no filesystem, network or process access) with lifecycle hooks, a plugin catalogue with SHA-256-verified installs, an embedded REST API and a WebSocket event stream. |
| **Workspace** | `egui_dock` workspace with floating windows, saved layout profiles, built-in presets and `.spws` layout export/import; command palette (`Ctrl+Shift+P`); built-in manual in all seven languages. |

Native CAT architecture, PTT invariants and RF safety rules are described in
[`Native CAT.md`](Native%20CAT.md).

---

## Keyboard shortcuts

Press `F1` in the application for the full list. Main bindings:

| Shortcut | Context | Action |
|---|---|---|
| `Enter` | QSO form | Save the QSO |
| `Esc` | QSO form | Clear the QSO form |
| `F2` / `Ctrl+L` | Global | Save the QSO |
| `F3` / `Ctrl+W` | Global / QSO form | Clear the QSO form |
| `Ctrl+N` | Global | New QSO (clear the form) |
| `F4` | Global | Callbook lookup for the entered callsign |
| `F5` | Global | Reload the log table |
| `F6` | Global | Send a DX cluster spot |
| `Ctrl+Shift+F7` | Global | Request PTT ON through CAT (TX intent gate, see above) |
| `Ctrl+F7` | Global | Request PTT OFF through CAT (always available while transmitting) |
| `F8` | Global | Voice keyer |
| `F11` | Global | Toggle full screen |
| `F12` / `Ctrl+H` | Global | User manual |
| `F1` | Global | Keyboard shortcut reference |
| `Ctrl+F` | Global | Advanced log filter |
| `Ctrl+D` / `Ctrl+B` / `Ctrl+M` | Global | DX cluster / band map / world map |
| `Ctrl+K` | Global | CW keyer terminal |
| `Ctrl+P` | Global | Station profiles |
| `Ctrl+E` / `Ctrl+I` | Global | CSV export / ADIF import |
| `Ctrl+T` | Global | Cycle the colour theme |
| `Ctrl+Z` / `Ctrl+Y` | Global | Undo / redo deleting QSOs |
| `Ctrl+S` | Global | Save the station configuration |
| `Ctrl+Shift+P` | Global | Command palette |
| `Ctrl+Shift+S` | Global | Statistics |
| `Ctrl+Q` | Global | Quit |

`F7` on its own does nothing. In contest mode, `Enter` and `Esc` save and clear the contest
entry.

---

## Download and install

### Release assets

Each GitHub release is produced by CI from its version tag. The
[Releases](https://github.com/sp6ina/SPLogbook/releases) page carries exactly these files:

| File | Platform | Description |
|---|---|---|
| `SPLogbook-Windows-x64-Setup-1.3.0.exe` | Windows x64 | Inno Setup per-user installer (no administrator rights) |
| `SPLogbook-Windows-x64-MSI-1.3.0.msi` | Windows x64 | WiX MSI per-machine installer |
| `SPLogbook-Windows-x64.zip` | Windows x64 | Portable archive |
| `SPLogbook-Linux-x86_64.tar.gz` | Linux x86_64 | Portable archive |
| `SHA256SUMS.txt` | all | SHA-256 checksums of the four packages |
| `release-manifest.json` | all | Release manifest: version, commit, size and SHA-256 of each package |
| `release-manifest.json.sig` | all | Ed25519 signature of the manifest |

DEB, RPM, AppImage and Flatpak packages are **not** release assets. Their packaging
scripts are verified in CI; you can build them from source (see
[Linux distribution packages](#linux-distribution-packages)).

### Verifying a download

Check the SHA-256 checksum of the file you downloaded:

```bash
sha256sum -c --ignore-missing SHA256SUMS.txt
```

```powershell
Get-FileHash .\SPLogbook-Windows-x64-Setup-1.3.0.exe -Algorithm SHA256
```

On Windows, compare the printed hash with the matching line in `SHA256SUMS.txt`.

To check the signed manifest, build the verifier from the source tree of the same release:

```bash
cargo build --release --bin splogbook-signer
./target/release/splogbook-signer verify release-manifest.json release-manifest.json.sig
```

Without a key argument, `verify` trusts only the release key built into SPLogbook,
`splogbook-release-2026-02` (public-key fingerprint
`2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a`), and exits with an
error when the signature does not match it. The key `splogbook-release-2026-01` is revoked
and is always rejected. The built-in updater applies
the same checks (manifest first, then platform, package type, file name, size and SHA-256)
before it installs anything. The full specification is in
[`docs/security/RELEASE_SECURITY.md`](docs/security/RELEASE_SECURITY.md).

### Windows

Requirements: Windows 10 or Windows 11, 64-bit.

Install **one** of the following:

- **Inno Setup installer (recommended):** run `SPLogbook-Windows-x64-Setup-1.3.0.exe`. It
  installs per user into `%LOCALAPPDATA%\Programs\SPLogbook` without administrator rights.
  Updating or uninstalling keeps your data.
- **MSI:** `SPLogbook-Windows-x64-MSI-1.3.0.msi` installs per machine into
  `C:\Program Files\SPLogbook` and needs elevation. It refuses to install when the per-user
  Inno Setup installation is present.
- **Portable ZIP:** extract `SPLogbook-Windows-x64.zip` into a folder and run
  `SPLogbook.exe`.

The ZIP bundles Hamlib 4.7.2 (`rigctld`, `rotctld`). Details:
[`installer/windows/inno/README.md`](installer/windows/inno/README.md),
[`installer/windows/msi/README.md`](installer/windows/msi/README.md).

### Linux

Requirements:

- x86_64, **Linux kernel 7.0 or newer** and **glibc 2.43 or newer**.
- Supported distributions: **Ubuntu 26.04 LTS**, **Fedora 45**, **Arch Linux** and
  **openSUSE Tumbleweed**. Debian 13 (glibc 2.41) and openSUSE Leap 16.1 (glibc 2.40) are
  not supported.
- X11 or Wayland.
- For Linux credential storage, a running and unlocked Secret Service provider (for example
  GNOME Keyring or KWallet).

The policy is recorded in
[`LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md`](LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md), a
point-in-time record made during the `1.3.0-alpha.1` measurement.

Portable archive:

```bash
tar -xzf SPLogbook-Linux-x86_64.tar.gz
cd SPLogbook
./SPLogbook
```

The archive contains the application, the updater, Hamlib 4.7.2 (`rigctld`, `rotctld` and
`libhamlib`) and a desktop entry template in `assets/`. It needs no root privileges. For
serial CAT and WinKeyer, add your user to the group that owns serial devices (`dialout` on
Ubuntu, Fedora and openSUSE; `uucp` on Arch Linux), then log in again. Details:
[`installer/linux/portable/README.md`](installer/linux/portable/README.md).

### Linux distribution packages

These packages are built from the portable archive by the scripts in `scripts/` and are
verified in CI on the pinned distribution images. They are not attached to GitHub releases.
Build commands are in [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md).

| Format | Script | Notes |
|---|---|---|
| DEB (Ubuntu 26.04 LTS) | `scripts/build_linux_deb.sh` | `/opt/SPLogbook` and the launcher `/usr/bin/splogbook`; [`installer/linux/deb/README.md`](installer/linux/deb/README.md) |
| RPM (Fedora 45, openSUSE Tumbleweed) | `scripts/build_linux_rpm.sh` | same layout, private `libhamlib.so.4`; [`installer/linux/rpm/README.md`](installer/linux/rpm/README.md) |
| AppImage | `scripts/build_linux_appimage.sh` | needs a working FUSE mount; [`installer/linux/appimage/README.md`](installer/linux/appimage/README.md) |
| Flatpak (`io.github.sp6ina.SPLogbook`) | `scripts/build_linux_flatpak.sh` | sandboxed, not on Flathub; native serial CAT is unavailable by default; [`installer/linux/flatpak/README.md`](installer/linux/flatpak/README.md) |

Installing, upgrading or removing any of these packages does not touch user data.

---

## Upgrading from v1.2.0

`v1.2.0` is no longer supported, and 1.3.0 contains no compatibility code for its data:

- Nothing is migrated automatically. A clean 1.3.0 profile starts with an empty logbook.
- To carry your log over, export it to ADIF in v1.2.0 and import the file in 1.3.0
  (`Ctrl+I`).
- Reference data (country files, DXCC attributes, regional rules, club memberships) is not
  bundled any more. Download or import it from the application; it is stored under
  `reference/` in the user data directory.
- Re-enter service credentials if the application asks for them (see below).

---

## Data, configuration and credentials

| | Windows | Linux |
|---|---|---|
| Configuration | `%APPDATA%\SPLogbook\` | `$XDG_CONFIG_HOME/splogbook` (default `~/.config/splogbook`) |
| Logbook database | `%APPDATA%\SPLogbook\databases\default_log.db` | `$XDG_DATA_HOME/splogbook/databases/default_log.db` (default `~/.local/share/splogbook`) |
| Reference data | `%APPDATA%\SPLogbook\reference\` | `$XDG_DATA_HOME/splogbook/reference/` |

- User data is kept outside the program directory. Installers, the updater and
  uninstallers leave it in place.
- Service passwords, API keys, the LAN sync secret and the local REST API key are kept in the
  operating system credential store (Windows Credential Manager or a Linux Secret Service
  provider), not in `station_config.json`. Existing plaintext credentials move into the store
  on first launch. If the move or access fails, startup stops with an error and the original
  configuration is left unchanged.
- Copying a configuration to another computer does not copy its credentials. Clear the
  `secret_store_id` value in the copied JSON file and enter the credentials again. Older
  plaintext copies or backups of `station_config.json` stay sensitive until you delete them.
- Backup and recovery of the SQLite database are described in
  [`docs/DATABASE_BACKUP_RECOVERY.md`](docs/DATABASE_BACKUP_RECOVERY.md).

---

## Local REST API

The embedded Axum server listens on the loopback address `127.0.0.1:8420` by default and
accepts browser requests only from loopback origins. Every endpoint except
`/api/v1/status` and `/api/v1/endpoints` requires the API key, sent as the `X-Api-Key`
header or as `Authorization: Bearer`; the key is shown and can be regenerated in the
application settings. `GET /api/v1/endpoints` returns the
full catalogue. Main groups:

| Endpoints | Purpose |
|---|---|
| `GET /api/v1/status`, `GET /api/v1/endpoints` | health, version and endpoint catalogue (public) |
| `GET/POST /api/v1/qsos`, `GET/PUT/DELETE /api/v1/qsos/{id}`, `GET /api/v1/qsos/callsign/{call}` | QSO list, create, read, update, delete, worked-before and dupe check |
| `GET /api/v1/lookup/{call}` | DXCC entity, zones, award needs and club memberships |
| `GET /api/v1/adif/export`, `POST /api/v1/adif/import` | ADIF export and batch import |
| `GET/POST /api/v1/rig`, `/api/v1/rigs/...` | CAT state and control for one or more radios |
| `GET /api/v1/awards`, `/api/v1/journals`, `/api/v1/stats`, `/api/v1/cluster/spots` | awards, journals, statistics and cluster spots |
| `GET /api/v1/ws` | WebSocket stream of application events |

QSOs created through the API go through the same ingest pipeline as the QSO form.

```bash
curl http://127.0.0.1:8420/api/v1/status
curl -H "X-Api-Key: <YOUR_API_KEY>" "http://127.0.0.1:8420/api/v1/lookup/DL1ABC"
```

---

## Building from source and running the tests

Requirements: Rust **1.99.0** or newer (edition 2024). `rusqlite` is always built with the
bundled SQLite.

### Windows

```powershell
git clone https://github.com/sp6ina/SPLogbook.git
cd SPLogbook
cargo build --release
```

### Linux (Ubuntu 26.04 LTS)

```bash
sudo apt update && sudo apt install -y \
  pkg-config libasound2-dev libx11-dev libxcursor-dev libxrandr-dev \
  libxi-dev libxkbcommon-dev libxkbcommon-x11-dev libgl1-mesa-dev \
  libfontconfig1-dev libwayland-dev libgtk-3-dev libusb-1.0-0-dev libudev-dev
git clone https://github.com/sp6ina/SPLogbook.git
cd SPLogbook
cargo build --release
```

The binaries are `target/release/SPLogbook.exe` on Windows and `target/release/SPLogbook`
on Linux. Toolchain pins, packaging commands and the git workflow are in
[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md).

### Tests

The canonical baseline command is:

```bash
cargo test --workspace --all-targets -j 1 -- --skip powershell_real_execution_full_cycle_success
```

The `--skip` filter applies only to local interactive Windows hosts, where that one test can
block on a security prompt; CI runs it unfiltered. Last measured baseline (counters are kept
in [`docs/VERIFICATION_BASELINE.md`](docs/VERIFICATION_BASELINE.md)):

- **Total Registered Tests**: 3716
- **Total Executed Tests**: 3705
- **Total Passed Tests**: 3705
- **Total Failed Tests**: 0
- **Total Ignored Tests**: 10 (all hardware-in-the-loop)
- **Total Filtered Tests**: 1 (local host only)

Static checks used before every change:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Hardware-in-the-loop tests

The 10 tests in `tests/cat_native_7.rs` marked `#[ignore = "hardware-in-the-loop"]` key a
real transmitter. They never run in CI or in the baseline, and hardware validation
(NATIVE-7B) has not been executed. Run them only on a laboratory bench with a 50 Ω dummy
load, never on a station connected to an antenna. Requirements and RF safety rules are in
[`Native CAT.md`](Native%20CAT.md) and
[`docs/HARDWARE_VALIDATION_MATRIX.md`](docs/HARDWARE_VALIDATION_MATRIX.md).

---

## Documentation

| Document | Contents |
|---|---|
| [`CHANGELOG.md`](CHANGELOG.md) | release history |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | contribution workflow, required checks, test and code rules |
| [`SECURITY.md`](SECURITY.md) | security policy, vulnerability reporting and updater guarantees |
| [`docs/DOCUMENTATION_INDEX.md`](docs/DOCUMENTATION_INDEX.md) | index of every tracked document |
| [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) | toolchain pins, build, test and packaging commands, git workflow |
| [`docs/VERIFICATION_BASELINE.md`](docs/VERIFICATION_BASELINE.md) | verified test counters and dependency-audit state |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | delivered milestones and planned work |
| [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) | open defects and open decisions |
| [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md) | module map, data flows, security boundaries |
| [`docs/architecture/DECISIONS.md`](docs/architecture/DECISIONS.md) | architectural decision records |
| [`docs/architecture/THEMES.md`](docs/architecture/THEMES.md) | theme and semantic palette architecture |
| [`docs/SECURITY_PROCESS.md`](docs/SECURITY_PROCESS.md) | dependency-audit policy, key handling, fail-closed rules |
| [`docs/security/RELEASE_SECURITY.md`](docs/security/RELEASE_SECURITY.md) | release supply chain and update trust specification |
| [`docs/security/SECURITY_KEY_ROTATION.md`](docs/security/SECURITY_KEY_ROTATION.md) | key rotation runbook |
| [`docs/DATABASE_BACKUP_RECOVERY.md`](docs/DATABASE_BACKUP_RECOVERY.md) | SQLite backup, restore and recovery |
| [`docs/HARDWARE_VALIDATION_MATRIX.md`](docs/HARDWARE_VALIDATION_MATRIX.md) | CAT protocol knowledge and hardware requirements |
| [`Native CAT.md`](Native%20CAT.md) | native CAT architecture, PTT invariants, HIL and RF safety |
| [`LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md`](LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md) | Linux kernel/glibc policy and supported distributions |
| [`locales/README.md`](locales/README.md) | translation catalogue and how to add strings |
| [`installer/`](installer) | per-format packaging documentation for Windows and Linux |

---

## Contributing

Issue reports and pull requests are welcome. Before opening a pull request:

1. Read [`CONTRIBUTING.md`](CONTRIBUTING.md).
2. Run the test baseline and the static checks listed above; all must pass.
3. Do not change dependencies or `Cargo.lock` as a side effect of another change.
4. Report security problems privately as described in [`SECURITY.md`](SECURITY.md), not in a
   public issue.

---

## Licence, data attribution and security

- SPLogbook is licensed under the **GNU General Public License v3.0 or later**. See
  [`LICENSE`](LICENSE).
- World map land outlines: **Natural Earth 110m Land**, public domain,
  [naturalearthdata.com](https://www.naturalearthdata.com/). The data is embedded as
  `assets/maps/ne_110m_land.bin`.
- The Windows ZIP and the Linux archive bundle Hamlib 4.7.2, distributed under its own
  licence (LGPL/GPL).
- Security policy and reporting: [`SECURITY.md`](SECURITY.md). Release signing and update
  trust: [`docs/security/RELEASE_SECURITY.md`](docs/security/RELEASE_SECURITY.md).

---

## Author and support

**Mariusz Woźniak (SP6INA)**

- E-mail: [contact@splogbook.org](mailto:contact@splogbook.org)
- Website: [splogbook.org](https://splogbook.org)
- QRZ: [SP6INA on QRZ.com](https://www.qrz.com/db/SP6INA)
- GitHub: [@sp6ina](https://github.com/sp6ina)

SPLogbook is free software with no advertising, telemetry or paid tiers. If you want to
support its development, you can
[buy the author a coffee at buycoffee.to/sp6ina](https://buycoffee.to/sp6ina).

73 de SP6INA
