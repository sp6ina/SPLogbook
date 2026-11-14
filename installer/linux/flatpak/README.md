# SPLogbook Linux x86_64 Flatpak (M3-J)

## 1. Scope and status

This directory contains the reviewed sources of the sandboxed Flatpak package
`SPLogbook-Linux-x86_64-<version>.flatpak` (canonical CI alias
`SPLogbook-Linux-x86_64.flatpak`).

| Item | Value |
|---|---|
| Status | implemented, verified in CI, **not published** |
| Application ID | `io.github.sp6ina.SPLogbook` |
| Runtime | `org.freedesktop.Platform` branch **26.08** |
| SDK | `org.freedesktop.Sdk` branch **26.08** |
| Rust toolchain | `org.freedesktop.Sdk.Extension.rust-stable` (branch 26.08) |
| Architecture | `x86_64` |
| Branch | `master` |
| Command | `SPLogbook` |

The Flatpak is an **additional** delivery format. It does not replace the
portable `tar.gz`, the AppImage, the DEB or the RPM, and it changes none of
their contracts.

The bundle is a **local and CI artifact only**:

- it is **not** submitted to Flathub and no Flathub repository exists;
- it is **not** attached to a GitHub release and no tag or release is created
  by M3-J;
- `Publish Release Assets` carries a permanent assertion that no `.flatpak`
  file may be staged into `release-staging`.

## 2. Why `org.freedesktop.Platform` and not `org.gnome.Platform`

The application is an `egui`/`eframe` GUI built with `glow` (OpenGL). It links
`libX11`, `libxkbcommon`, Wayland client libraries, `libGL`, `libasound`,
`libfontconfig` and nothing from GTK: `Cargo.lock` contains no `gtk-sys`,
`glib-sys`, `gdk-sys` or `pango` crate, because `rfd` 0.17 uses the
`xdg-portal` backend for file dialogs. The FREEDESKTOP runtime therefore
provides every runtime dependency, and the significantly larger GNOME runtime
would only add an unused GTK stack.

The runtime and SDK are pinned by branch (`26.08`), which is the current stable
freedesktop-sdk series (`freedesktop-sdk-26.08.4` at the time of this gate).
Beta, release-candidate and nightly series are never used, and the branch is
never replaced by an unpinned `master`.

## 3. Files in this directory

| File | Role |
|---|---|
| `io.github.sp6ina.SPLogbook.yml` | the Flatpak manifest (identity, runtime, permissions, modules) |
| `io.github.sp6ina.SPLogbook.metainfo.xml` | AppStream metadata (`appstreamcli` validated) |
| `io.github.sp6ina.SPLogbook.desktop` | desktop entry (`desktop-file-validate` validated) |
| `icons/64x64/…png`, `icons/128x128/…png`, `icons/256x256/…png` | icons exported under the application ID |
| `cargo-sources.json` | generated, reviewable list of the 662 pinned crates |

## 4. Sandbox permissions (exactly)

```yaml
finish-args:
  - --share=network
  - --share=ipc
  - --socket=wayland
  - --socket=fallback-x11
  - --socket=pulseaudio
  - --device=dri
  - --talk-name=org.freedesktop.secrets
```

| Permission | Reason | Negative proof |
|---|---|---|
| `--share=network` | CAT over TCP (`rigctld`, `rotctld`, TCI, FlexRadio, Kenwood LAN, Elecraft K4 TCP, `flrig`, WinKeyer), DX cluster, callbook, LoTW/eQSL/ClubLog uploads, LAN sync, loopback REST server | removing it fails `verify-flatpak-permissions` (exact contract) |
| `--share=ipc` | MIT-SHM for the X11 fallback path | an extra shared subsystem fails the exact contract |
| `--socket=wayland` | native Wayland windowing | — |
| `--socket=fallback-x11` | X11 only when Wayland is unavailable | — |
| `--socket=pulseaudio` | CW/DX side tone through `rodio`/`cpal` | — |
| `--device=dri` | hardware OpenGL for `eframe`/`glow` | `--device=all` is rejected |
| `--talk-name=org.freedesktop.secrets` | `keyring` stores QRZ/eQSL/ClubLog/LAN credentials in the Secret Service | `--talk-name=*` is rejected |

The following are **deliberately absent** and asserted absent by the verifier
and by `security_guard`:

`--filesystem=host`, `--filesystem=home`, `--filesystem=/dev`,
`--filesystem=/etc`, `--filesystem=/usr`, `--filesystem=/opt`,
`--filesystem=/run`, `--device=all`, `--socket=system-bus`,
`--socket=session-bus`, `--talk-name=*`, `--own-name=*`,
`--system-talk-name=*`, `--allow=devel`, `--allow=multiarch`.

There is **no** persistent path, no `--env=` override and no extra
`[Session Bus Policy]` entry. `flatpak info --show-permissions
io.github.sp6ina.SPLogbook` must produce exactly this normalised set:

```text
context.devices          dri
context.shared           ipc
context.shared           network
context.sockets          fallback-x11
context.sockets          pulseaudio
context.sockets          wayland
session-bus.…secrets     talk
```

## 5. Sources, offline build and reproducibility

The Rust dependency set is the **662 crates of `Cargo.lock`**, pinned by the
SHA-256 checksums Cargo itself recorded. They are listed in
`cargo-sources.json`, generated deterministically by

```sh
bash scripts/generate_flatpak_cargo_sources.sh          # rewrite
bash scripts/generate_flatpak_cargo_sources.sh --check  # verify (CI)
```

`tests/linux_flatpak_packaging_test.rs` re-implements the projection in Rust
and fails if the committed file diverges from `Cargo.lock` in any crate name,
version or digest. There is no vendored `vendor/` directory in the repository:
only the generated, reviewable source list is committed.

The build is controlled and fail-closed:

1. `flatpak-builder --download-only` performs the **only** network phase
   (crates from `static.crates.io`, pinned by SHA-256; runtime and SDK from
   the pinned Flathub branch).
2. `flatpak-builder --disable-download --sandbox` performs the real build with
   no source download and no build-arg escape. The module sandbox has no
   network namespace.
3. Cargo runs with `CARGO_NET_OFFLINE=1`, a vendored-source replacement and
   `--locked --offline`, so a missing or mismatched crate fails the build
   instead of being downloaded.
4. `scripts/build_linux_flatpak.sh` refuses to run unless the tracked worktree
   is clean, `HEAD` equals `EXPECTED_BUILD_COMMIT` and `target/` does **not**
   exist, so no previous Cargo build can become an implicit input.

Two independent clean builds from the same inputs are compared in the
`Verify Flatpak determinism` job: the complete `/app` payload (paths, modes,
sizes and per-file SHA-256) must be identical. Whether the OSTree commit and
the bundle file are also bit-identical is recorded as evidence and reported
separately; the gate never claims bit-identical bundles when OSTree metadata
differs.

The build identity (`--build-info` and the in-bundle marker) comes from the
`.git` directory of the source tree, which is the single source of truth for
the commit. The builder never injects a synthetic commit.

## 6. Sandboxed functionality and hardware limits

| Capability | In the Flatpak sandbox |
|---|---|
| QSO logging, ADIF/Cabrillo, awards, statistics, backups | **supported** (XDG data inside the sandbox) |
| CAT over TCP: `rigctld`, `rotctld`, TCI, FlexRadio, Kenwood LAN, Elecraft K4 TCP, `flrig`, WinKeyer | **supported** |
| Loopback REST server, LAN sync, DX cluster, callbook, LoTW/eQSL/ClubLog | **supported** |
| File open/save dialogs | **supported** through XDG Desktop Portal (no `--filesystem` needed) |
| Opening URLs and the logbook folder | **supported** through the portal |
| Audio side tone | **supported** (`--socket=pulseaudio`) |
| Native CAT over `/dev/ttyUSB*`, `/dev/ttyACM*`, `/dev/ttyS*` (Icom CI-V, Kenwood, Yaesu, Elecraft serial) | **not available** in the default sandbox |
| Direct USB device access, SDR via `libusb` | **not available** |
| Automatic local `rigctld` start | **not available** (no `rigctld` binary in the bundle) |

### 6.1 Owner decision and the documented opt-in

`OWNER-M3-J-DEVICE-01` approved this model: the default sandbox stays minimal
and native serial CAT is **fail-closed** with a clear diagnostic instead of
silently failing. Flatpak has no per-port device permission — `--device=dri`,
`input`, `usb`, `kvm`, `shm`, `all` are the only values, and `--device=usb`
exposes only `/dev/bus/usb`, not the `/dev/tty*` nodes a serial CAT interface
needs.

A user who needs native serial CAT can widen the sandbox for this application
only, with an explicit, reversible command:

```sh
flatpak override --user --device=all io.github.sp6ina.SPLogbook
```

> **Warning.** This override grants the application access to **all devices
> visible to Flatpak** — not only to the serial port you intend to use. It
> includes every `/dev` node the Flatpak runtime exposes (audio, input, USB,
> block and character devices, DRM and others). Apply it only if you accept
> that scope, and remove it with
> `flatpak override --user --reset io.github.sp6ina.SPLogbook` when you no
> longer need it. The shipped manifest never contains `--device=all`, and
> `--filesystem=/dev` is never used as a workaround.

Selective USB device access through `org.freedesktop.portal.Usb` is recorded as
a **separate future improvement**: it requires new device-access architecture
in the serial backends and is deliberately not part of this change.

## 7. User data location and preservation

Inside the sandbox the application uses the standard XDG base directories,
which Flatpak maps below `~/.var/app/io.github.sp6ina.SPLogbook/`:

| Data | Host path |
|---|---|
| QSO database, exports, backups | `~/.var/app/io.github.sp6ina.SPLogbook/data/splogbook/` |
| configuration, tokens | `~/.var/app/io.github.sp6ina.SPLogbook/config/splogbook/` |
| state, journals | `~/.var/app/io.github.sp6ina.SPLogbook/state/splogbook/` |
| caches | `~/.var/app/io.github.sp6ina.SPLogbook/cache/splogbook/` |

The application never writes to `/app`, `/usr`, the installation directory or
the current working directory. `/app` is read-only inside the sandbox and the
verifier proves that a write attempt fails.

`flatpak uninstall io.github.sp6ina.SPLogbook` removes the application only.
`~/.var/app/io.github.sp6ina.SPLogbook/` is **not** removed, so a later
reinstall finds the logbook, configuration and backups unchanged. Deleting
that directory is a separate, explicit user action.

Data from an unsandboxed installation (portable `tar.gz`, AppImage, DEB, RPM)
is **never** copied automatically. There is no manifest permission that would
allow such a copy, and no silent import exists. Migration is a manual,
user-driven export/import through ADIF, exactly as for any other format
change.

## 8. Self-update

A Flatpak is updated by Flatpak, never by the application:

- the bundle does **not** contain `splogbook-updater`;
- the application classifies itself as `InstallationType::LinuxFlatpak` from
  the stable `/.flatpak-info` sandbox marker, which is system-managed
  (`supports_self_apply = false`, `requires_delegation = true`);
- no code path writes into `/app`, and the verifier asserts that `/app` is
  read-only;
- the update instruction for users is `flatpak update io.github.sp6ina.SPLogbook`
  or the desktop software centre.

## 9. Lifecycle verified in CI

The `Verify Flatpak lifecycle` job performs, in a throw-away user
installation and with isolated fixtures (never real user data):

1. fresh install of the bundle;
2. first run (`--version`, `--build-info`, `--check-health-startup`);
3. creation of fixture data in the sandbox XDG directories;
4. `flatpak update` at the same ref;
5. a forced re-deploy of the installed OSTree commit, which is the deployment
   path a real downgrade uses (`flatpak update --commit=<commit>`);
6. reinstall of the same bundle;
7. uninstall — the data directory and its content must be byte-identical;
8. reinstall and a second run;
9. final uninstall — the data must still be identical;
10. explicit, separate removal of the fixture data.

Every step compares a SHA-256 digest of the complete data tree (paths, modes,
sizes and per-file digests), so any rewrite, deletion or truncation fails the
job.

A SemVer **version** downgrade is not exercised because only one Flatpak build
exists: the development line `1.3.0-alpha.1` has no earlier Flatpak artifact,
and the public `1.2.0` release predates this format. A version downgrade
requires a second published build and therefore remains a future scenario; the
deployment-level downgrade path is covered as described above.

## 10. Build and verify

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
FLATPAK_BUILD_IMAGE="ubuntu:26.04@sha256:…" \
  bash scripts/build_linux_flatpak.sh dist
```

The builder validates the toolchain, the identity, the exported file names,
the icons, the AppStream metadata, the desktop entry, the pinned source list,
the forbidden permissions and the manifest projection
(`flatpak-builder --show-manifest`), installs the pinned runtime/SDK into an
isolated `FLATPAK_USER_DIR`, and only then builds. It writes the bundle, the
`.sha256` sidecars, the per-file payload manifest, `flatpak-toolchain.json`,
`flatpak-determinism.txt`, the runtime/SDK information and the build logs. It
prints `FLATPAK BUILD COMPLETED SUCCESSFULLY` only as its last statement.

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
  bash scripts/verify_linux_flatpak.sh dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.flatpak
```

The verifier checks the bundle type, magic, size, both checksum sidecars, the
payload export allowlist (no symlink, no special bit, no world-writable file),
the manifest contract, the pinned sources, AppStream, the desktop entry, the
icon sizes, then installs the bundle into a private user installation and
compares `flatpak info --show-permissions` with the exact expected set, the
ref/arch/branch/commit, the metadata runtime and SDK, the payload allowlist,
the marker, the absence of Hamlib, of the self-update helper and of user data,
then executes `--version`, `--build-info`, `--check-health-startup`, an
arbitrary working directory run, `ldd` inside the runtime, the read-only `/app`
proof, the XDG mapping, the full lifecycle and a headless X11 GUI smoke test.
It prints `FLATPAK VERIFICATION COMPLETED SUCCESSFULLY` only at the end.

`SPLOG_FLATPAK_STAGE=permissions|lifecycle` restricts the run to the
corresponding stage and prints a stage-specific banner.
`SPLOG_FLATPAK_INSPECTION_ONLY=1` stops after the static checks and prints
`FLATPAK INSPECTION COMPLETED SUCCESSFULLY`; it is used only by the negative
harness and is never evidence of a successful installation.

## 11. Negative fixtures

```sh
EXPECTED_BUILD_COMMIT="$(git rev-parse HEAD)" \
  bash scripts/verify_linux_flatpak_negative_fixture.sh \
    dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.flatpak
```

The harness mutates disposable copies of the repository contract, the bundle
and its sidecars, and the CI release-staging contract. Every mutation must be
rejected with a non-zero exit code, a controlled `FAIL:` reason and no success
banner. It covers the wrong app ID, runtime, runtime version, SDK,
architecture, command, build commit, version, a missing or invalid AppStream
file, a missing or invalid desktop entry, a missing icon, a wrong export name,
`--filesystem=host`, `--filesystem=home`, `--device=all`,
`--socket=system-bus`, a wildcard bus name, an unpinned git source, an archive
source without SHA-256, an absolute source path, a `file:///` path, a networked
build stage, a missing `--locked`, missing offline enforcement, a
`cargo-sources.json` divergence, the Hamlib model, a wrong Hamlib SONAME, an
unexpected bundled library, an `/opt` dependency, a self-update helper, a
user-data file in the working directory or in `/app`, user-data loss on
uninstall, a corrupt bundle, a non-Flatpak file, a wrong SHA-256, an extra
file, a symlink, a setuid and a world-writable mode, a wrong metadata licence,
a mismatching launchable, a missing or unjustified network declaration and a
publication into `release-staging`.

## 12. CI

`.github/workflows/build-and-release.yml` contains:

| Job | Purpose |
|---|---|
| `Build Flatpak` | one clean build inside the pinned privileged container, artifact upload |
| `Verify Flatpak` | full static + install + execution + lifecycle + GUI verification |
| `Verify Flatpak permissions` | exact `flatpak info --show-permissions` contract and payload allowlist |
| `Verify Flatpak lifecycle` | data preservation across the whole lifecycle plus GUI smoke |
| `Verify Flatpak determinism` | second independent clean build and payload comparison |
| `Verify Flatpak negative fixtures` | all negative fixtures |

No job uses a secret, publishes anything, needs interactive input, requires
hardware or touches PTT/RF. Logs are kept as CI artifacts, and no failure is
retried or masked.

## 13. Known limitations

- Native serial CAT and direct USB device access are unavailable in the
  default sandbox (see section 6).
- A local `rigctld` cannot be spawned by the application because the bundle
  contains no Hamlib binaries; run `rigctld` on the host and connect over
  `127.0.0.1` (the sandbox shares the host network namespace).
- The bundle links no Hamlib: the application's only Hamlib integration is the
  `rigctld` TCP protocol, so the DEB/RPM private-library exception has no
  Flatpak counterpart. This is verified: the bundle contains no `libhamlib*`
  and `ldd` reports no Hamlib.
- A SemVer downgrade is not exercised (see section 9).
- Graphical verification is headless (Xvfb); it proves that the process
  survives, creates no fatal error and exits only on timeout, not that a human
  sees a correct window on a physical display.
- The Flatpak is verified on hosted `ubuntu-latest` runners with a pinned
  `ubuntu:26.04` container. That proves userspace behaviour, not a specific
  distribution kernel.
- No Flathub submission exists, so the package is installed from a local
  bundle (`flatpak install ./SPLogbook-Linux-x86_64.flatpak`).
