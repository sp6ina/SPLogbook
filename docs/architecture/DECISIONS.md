# SPLogbook — Architecture Decision Records (ADRs) and Binding Invariants

Consolidated register of architectural decision records (ADRs) that are still binding, together
with the architectural invariants they establish. Each row names where the decision is implemented
and which guard enforces it, so the decision can be traced to code and to a test.

Status vocabulary: **Implemented** (in code and guarded), **Open** (undecided, may block
work), **Superseded** (replaced by a later decision — kept for history only).

---

## 1. Binding architectural invariants

These are not preferences; they are contract. Weakening them requires a new decision.

1. **Single crate, single package.** One Cargo package `splogbook`, no `[workspace]`, three
   binaries (`SPLogbook`, `splogbook-updater`, `splogbook-signer`) plus a library with the
   modules `api, cat, cloud, cluster, core, digital, dsp, gui, media, network, plugins, sync`.
2. **Dependency direction.** `core` and `cat` must never depend on `gui`; `gui` may depend on
   all other modules.
3. **Single RF gate.** All transmit intent passes `TxArbiter`; PTT is never keyed during
   `connect()`, `probe()` or model detection, and `SetPtt` is never retried automatically.
   GUI transmit intent additionally passes one central dispatcher (`ADR-93`)
   and PTT ON requires the F-15 authorization outcome (`ADR-94`).
4. **Theme tokens only.** No raw `Color32` outside `src/gui/theme/**` and the isolated
   `qsl_colors` module — enforced by `tests/theme_guard.rs`.
5. **Data classification is absolute.** `UNKNOWN` data classification means *do not
   overwrite*. User databases, WAL/SHM files, configuration, exports and secrets are never
   packaged and never replaced.
6. **No privilege escalation.** No `sudo`, `doas`, `pkexec`, `runas` or UAC elevation
   anywhere; no writes to `/usr`; no direct replacement inside `Program Files`.
7. **Manifest-first trust.** An update asset exists only as part of a release whose manifest
   signature verified against a trusted, active key.
8. **Replacement semantics.**
   File replacement is **Journaled Transactional Replacement**:
   **`JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED`**, **`CRASH_RECOVERABLE_BY_JOURNAL`** and
   explicitly **Not Power-Loss Atomic** (**`NOT_POWER_LOSS_ATOMIC`**). The operation is
   **Crash Recoverable by Journal**, not atomic against power loss. Documentation and code must
   use these terms and must never claim hardware atomicity or "atomic swap".
9. **Linux support policy is derived, not fixed.** The kernel minimum is a
   **`DYNAMIC DISTRIBUTION-DERIVED KERNEL COMPATIBILITY FLOOR`**: the minimum is the oldest
   default kernel line among the officially supported distributions, resolved through a
   **`Common Compatibility Intersection`** of their glibc and kernel versions. It is
   recomputed per release and never raised without technical proof.
10. **Sendable but not shareable backends.** `RigBackend` is `Send` but not `Sync`; exactly
    one owner task holds a backend. The GUI never holds backends or transports.
11. **No `unsafe` in the updater**, and no `unwrap()`, `expect()` or `panic!` on production
    updater paths.
12. **Archive hardening.** Reject absolute paths, `..`, drive/UNC prefixes, alternate data
    streams, device names, symlinks, hardlinks, FIFOs and device nodes; bound entry count,
    per-file size and total extracted size.

## 2. Architectural Decision Records

### 2.1 Installer and updater scope

| ID | Decision | Status | Implemented in | Guarded by |
|---|---|---|---|---|
| `ADR-01` | Expanded Windows scope: portable ZIP + WiX MSI + Inno Setup; dual helper (native `splogbook-updater` primary, hardened PowerShell fallback) | Implemented | `src/cloud/updater/**` | `m3_b_*`, `m3_c_*` guards |
| `ADR-02` | Expanded Linux scope (portable tar.gz, AppImage, DEB, RPM, Flatpak declared); package-manager boundary; absolute ban on `sudo`/`doas`/`pkexec`/`runas` and on writes to `/usr` | Implemented | `src/cloud/updater/**`, `scripts/**` | `updater_does_not_use_sudo_guard`, `m3_f_*` |
| `ADR-03` | `.rollback` retention and backup-deletion policy | **Open** (non-blocking) | temporary conservative retention: `.rollback` is never auto-deleted and never contains user data | `m3_c_temporary_conservative_backup_retention_guard` |
| `ADR-04` | Classification of `databases/serviceLOG.db` | **Closed** (superseded) — `serviceLOG.db` and `FirstInstallSeed` were removed; all `.db`/`.sqlite` in `databases/` are `UserData` | `src/cloud/updater.rs` | `portable_database_files_are_protected_guard`, `r1c_r1d_legacy_databases_removed_and_xdg_precedence_guard` |
| `ADR-05` | Legacy bundled `databases/callbook.db` removed; all `*.db` in `databases/` protected as `UserData` | **Superseded** (`callbook.db` and `FirstInstallSeed` removed) | `src/cloud/updater.rs` | `m3_a_data_preservation_matrix_guard`, `r1c_r1d_legacy_databases_removed_and_xdg_precedence_guard` |
| `ADR-06` | Audit-log retention | **Open** | — | — |
| `ADR-07` | Cancellation is prohibited once `Applying` has started | Implemented | `src/cloud/updater/state.rs` | `cancellation_prohibited_from_applying_guard` |
| `ADR-08` | Native tests on Windows and Linux | Implemented | `tests/windows_*`, `tests/linux_*` | — |
| `ADR-09` | Native test matrix requirement for installers and packaging | Implemented | `.github/workflows/build-and-release.yml` | `m3_f_f03_fail_closed_verification_guard` |

### 2.2 Windows installer toolchains, CI publishing, and delegated update safety

| ID | Decision | Status | Implemented in | Guarded by |
|---|---|---|---|---|
| `ADR-10` | Inno Setup **7.1.1** at `C:\Program Files\Inno Setup 7\ISCC.exe` (SHA-256 `E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644`), zero compiler warnings tolerated; CI installs and verifies the pinned 7.1.1 installer (`F-05 = Resolved`) | Implemented | `installer/windows/inno/SPLogbook.iss`, `installer/windows/inno/README.md`, `.github/workflows/build-and-release.yml` | `m3_d_inno_setup_toolchain_decision_guard`, `m3_d_inno_setup_ci_build_and_publish_guard` |
| `ADR-11` | WiX Toolset SDK **7.0.1** with .NET 8 SDK / MSBuild 2022 SDK resolver discovered via `vswhere.exe` (fallback to VS 2022 editions); EULA accepted via `wix eula accept wix7` / `AcceptEula=true`; CI builds, E2E-verifies, and publishes `.msi` (`F-05 = Resolved`) | Implemented | `installer/windows/msi/SPLogbook.wixproj`, `scripts/build_windows_msi.ps1`, `.github/workflows/build-and-release.yml` | `m3_e_msi_toolchain_decision_and_identity_guard`, `m3_e_msi_ci_build_and_publish_guard` |
| `ADR-12` | MSI prerelease version mapping | **Open** — blocks MSI for `1.3.0-alpha.2` (error 1638) | — | `m3_e_*` version-mapping guards |
| `ADR-13` | Windows installer hardening (`F-05`, `F-12`, `F-13`, `F-22`): numeric SemVer 2.0.0 in Inno, explicit `AppMutexName` + Win32 named mutexes via `kernel32` FFI, lock + transaction journal + `HealthProof` + restart reconciliation for `WindowsInnoSetup`/`WindowsMsi`, and CI build/E2E/manifest/release publishing of ZIP + Inno `.exe` + WiX `.msi` | Implemented | `installer/windows/inno/SPLogbook.iss`, `src/main.rs`, `src/cloud/updater.rs`, `.github/workflows/build-and-release.yml` | `windows_inno_setup_test`, `windows_msi_test`, `release_workflow_rejects_incomplete_artifact_set`, `release_manifest_lists_every_expected_artifact_once` |

### 2.3 Linux support and packaging

All records below are **Implemented**; `ADR-14`, `ADR-15` and `ADR-18` were later **Superseded**
on their numeric values.

| ID | Decision |
|---|---|
| `ADR-14` | Rolling latest-stable Linux baseline. **Superseded** by `ADR-16` + `ADR-17` |
| `ADR-15` | Initial kernel floor model. **Superseded** by `ADR-16` |
| `ADR-16` | Kernel minimum is the oldest default kernel line among supported distributions — currently **Linux 7.0**, set by Ubuntu 26.04 LTS. Replaces both the dynamic `6.12` floor (`ADR-15`) and the fixed `7.2` minimum |
| `ADR-17` | glibc minimum **2.43**; maximum required symbol `GLIBC_2.43`; glibc is never bundled and is never rebuilt to an older symbol set |
| `ADR-18` | Cross-distribution ABI verification. **Superseded** on the numeric glibc baseline by `ADR-17` |
| `ADR-19` | Platform adapters and layered updater architecture |
| `ADR-20` | Only Hamlib may be bundled (`libhamlib.so.4`); all other runtime libraries come from the distribution |
| `ADR-21` | Private Hamlib with `RUNPATH=$ORIGIN/lib`; no `RPATH`; no system Hamlib dependency |
| `ADR-22` | One canonical portable artifact, a single root directory `SPLogbook/`, and an installation marker |
| `ADR-23` | Self-managed update for portable tar.gz and AppImage via journaled transactional replacement |
| `ADR-24` | Two-phase safe extraction: read-only preflight, then extraction into a cleaned staging directory, with entry/size/type limits |
| `ADR-25` | User data in XDG Base Directories; absolute `XDG_*` enforced with `$HOME` fallbacks; no silent migration of legacy portable data |
| `ADR-26` | Release integrity through the signed manifest and SHA-256; no package signing for DEB/RPM |
| `ADR-27` | Unprivileged, user-scope desktop integration (`share/applications/splogbook.desktop`) |
| `ADR-28` | Build and verify scripts are `set -euo pipefail`, deterministic, and free of `sudo`/`pkexec`/`doas`/`su` |
| `ADR-29` | Deterministic stability loops (100/50/20 iterations) and ephemeral test keys only; production keys are forbidden in tests |
| `ADR-30` | Fast-forward only; no rebase, merge commit or force-push; `v1.2.0` immutable; no premature `v1.3.0` |

Consequences of the Linux support policy:

- Supported: **Ubuntu 26.04 LTS** (kernel 7.0.0, glibc 2.43), **Fedora 45** (7.2, 2.44),
  **Arch Linux** (7.2, 2.44), **openSUSE Tumbleweed** (7.2, 2.44).
- Not supported: **Debian 13.7** (glibc 2.41) and **openSUSE Leap 16.1** (glibc 2.40) — they
  do not meet the ABI requirement. **Ubuntu 26.10** interim is out of scope.
- Snapshot contract: `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json` (`splogbook-linux-support-snapshot-v2`).

### 2.4 Verification and repository hygiene (F-03)

| ID | Decision | Status |
|---|---|---|
| `ADR-31` | Repository state is described with four explicit classes: tracked, untracked, ignored, strict | Implemented |
| `ADR-32` | Local working directories stay out of the public tree | Implemented |
| `ADR-33` | A run with 7 profiles is never described as 4 profiles; false-PASS evidence is classified as such | Implemented |
| `ADR-34` | One documentation commit per consistency change | Implemented |
| `ADR-35` | Fast-forward-only push after a green baseline | Implemented |
| `ADR-36` | Completed milestones are not reopened without reproducible regression | Implemented |
| `ADR-37` | Native evidence is classified by origin (userspace/CI) and never claimed as kernel proof | Implemented |
| `ADR-38` | Fail-closed native Linux verification (F-03 **Resolved**) | Implemented |

### 2.5 AppImage toolchain pins

| ID | Decision | Status |
|---|---|---|
| `ADR-39` | AppImage packaging integration contract | Implemented |
| `ADR-40` | `appimagetool` = official `appimagetool` repository, channel `continuous`, commit `11d54c9acc7f246df1398050a4b880314ac76862`, SHA-256 `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` | Implemented |
| `ADR-41` | Type-2 runtime = channel `continuous`, commit `fedfad51acc2860b2916a1eef32ba4a500ff31ef`, SHA-256 `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46`, passed via explicit `--runtime-file` | Implemented |
| `ADR-42` | Dynamic resolution of latest verified official builds through the GitHub REST API | **Superseded** 2026-11-18 by `ADR-44` |
| `ADR-43` | No fallback to an older tool release (appimagetool 1.9.1 stays forbidden) | Implemented |
| `ADR-44` | Static pin, no REST API, no cache. The pins live in `installer/linux/appimage/toolchain-pins` and keep the builds of `ADR-40` / `ADR-41`. appimagetool: size `15133176`, SHA-256 `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03`, URL `https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage`. Runtime: size `944632`, SHA-256 `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46`, URL `https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64`. Each asset is downloaded directly and must match the exact size and SHA-256 (fail closed, `PIN_STALE` when upstream replaced the `continuous` asset); updating a pin requires an explicit ADR revision | Implemented |

### 2.6 DEB package contract

All ten decisions are **Implemented** and verified by `tests/linux_deb_packaging_test.rs` and
`m3_h_deb_integration_guard`.

| ID | Decision |
|---|---|
| `ADR-45` | Binary package only (no source package), built with `dpkg-deb -Zxz` from the pinned `ubuntu:26.04` image |
| `ADR-46` | Identity `splogbook`, version `1.3.0~alpha.1-1`, architecture `amd64`, section `hamradio` |
| `ADR-47` | Layout `/opt/SPLogbook` plus the launcher `/usr/bin/splogbook`; lintian suppression limited to `dir-or-file-in-opt,custom-library-search-path` (and rpmlint equivalent for RPM) |
| `ADR-48` | Desktop entry and 256×256 icon installed to the standard XDG locations |
| `ADR-49` | Installation marker `.splogbook-deb-marker` carrying the build identity |
| `ADR-50` | Fail-closed launcher: no `sudo`, no `LD_LIBRARY_PATH` override, absolute `exec` |
| `ADR-51` | `Depends` generated from the ELF closure, then reviewed (17 packages); bundled Hamlib is not a dependency |
| `ADR-52` | Minimal maintainer scripts; no `preinst`/`prerm`; user data preserved across the whole lifecycle |
| `ADR-53` | No package signing; integrity via the release manifest and SHA-256 |
| `ADR-54` | Artifact is a CI artifact only; publishing requires a separate release step. **Superseded by `ADR-72`** (published from v1.4.0) |

### 2.7 Flatpak package contract

All decisions are **Implemented** and verified by `tests/linux_flatpak_packaging_test.rs`,
`m3_j_flatpak_sandbox_and_packaging_guard` and the Flatpak CI jobs.

| ID | Decision |
|---|---|
| `ADR-55` | Application ID `io.github.sp6ina.SPLogbook`. Replaces the earlier design-time placeholder `org.splogbook.SPLogbook`. The `io.github.<owner>.<App>` reverse-DNS form matches the repository namespace and needs no domain registration; every exported name (desktop entry, AppStream metadata, icons, marker) uses it. |
| `ADR-56` | Runtime `org.freedesktop.Platform` and SDK `org.freedesktop.Sdk`, branch `26.08`. The SDK Rust extension is not used because branch 26.08 ships rustc 1.98.0, below the project MSRV 1.99.0 (`ADR-58`). The Freedesktop runtime is the smallest runtime that provides every actual dependency: the lockfile contains no GTK, GLib or Pango crate, so the GNOME runtime would only add an unused stack. |
| `ADR-57` | Default sandbox without `--device=all`. Native serial CAT (`/dev/ttyUSB*`, `/dev/ttyACM*`) is unavailable and fails closed with a diagnostic; network CAT backends are fully supported. A documented user-side opt-in `flatpak override --user --device=all io.github.sp6ina.SPLogbook` exists and states explicitly that it widens access to all devices visible to Flatpak. Selective USB portal support is a separate future improvement that requires new device-access architecture. |
| `ADR-58` | The Flatpak build uses the official `rust-1.99.0-x86_64-unknown-linux-gnu.tar.gz` archive pinned by SHA-256 `de0581ca9d732295a6474cfbd02461db27d69acd5050a8206523a8d6fa1599db` instead of `org.freedesktop.Sdk.Extension.rust-stable`, whose 26.08 branch ships rustc 1.98.0 and would violate the project MSRV 1.99.0. The build asserts `rustc --version` and the `Resolve Rust toolchain pin` CI job re-verifies the digest against the official distribution on every run. |
| `ADR-59` | The permission set is exactly `--share=network`, `--share=ipc`, `--socket=wayland`, `--socket=fallback-x11`, `--socket=pulseaudio`, `--device=dri`, `--talk-name=org.freedesktop.secrets`. No filesystem permission, no persistent path, no environment override and no wildcard bus name. |
| `ADR-60` | The Rust build is offline and locked: 662 crates pinned by the `Cargo.lock` SHA-256 checksums in `cargo-sources.json`, a vendored-source replacement, `CARGO_NET_OFFLINE=1`, `cargo build --locked --offline`, one `--download-only` network phase and a real build with `--disable-download`, a bwrap sandbox with no network namespace, vendored Cargo sources and `CARGO_NET_OFFLINE=1`. `--sandbox` is not passed because flatpak-builder 1.4.x rejects a `dir` source located above the manifest directory in a sandboxed build; the manifest declares no `build-args`, so nothing can escape through them. |
| `ADR-61` | The Flatpak links and bundles no Hamlib. The only Hamlib integration is the `rigctld` TCP protocol, so the DEB/RPM private-library exception has no Flatpak counterpart; the sandbox already guarantees that no host library is reachable. Verified by the absence of `libhamlib*` in the bundle and in `ldd`. |
| `ADR-62` | A Flatpak is updated by Flatpak only. The bundle ships no `splogbook-updater`, the application classifies itself as `LinuxFlatpak` from the `/.flatpak-info` marker with `supports_self_apply = false` and `requires_delegation = true`, and `/app` is read-only inside the sandbox. |
| `ADR-63` | **Superseded by `ADR-72`** for the GitHub release (the bundle is published from v1.4.0; still no Flathub submission). Original decision: the bundle is a local/CI artifact only. No Flathub submission, no Flathub repository, no GitHub release, no tag, and a permanent assertion that no `.flatpak` may be staged into `release-staging`. |
| `ADR-64` | Two independent clean builds must produce an identical `/app` payload (paths, modes, sizes, per-file SHA-256). Bit-identical bundles or OSTree commits are reported as evidence and never claimed when they differ. |

### 2.8 Dependency and security decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-65` | Use the latest stable compatible dependency versions; never `cargo audit --ignore` as a shortcut | Implemented |
| `ADR-66` | `rusqlite` keeps the `bundled` feature; system SQLite is forbidden | Implemented |
| `ADR-67` | Lockfile changes only through cargo (`cargo update -p <pkg> --precise <ver>`), push only after a green `cargo audit` | Implemented |
| `ADR-68` | `rustix 1.1.5 → 1.1.6` (targeted) to remove `RUSTSEC-2026-0486` | Implemented |
| `ADR-69` | `quinn-proto 0.11.19` / `quinn-udp 0.5.16` to remove `RUSTSEC-2026-0523`, `-0524`, `-0538` | Implemented |
| `ADR-70` | `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` to remove `RUSTSEC-2026-0459` | Implemented |
| `ADR-71` | `Publish Release Assets` runs only for a pushed `v*` tag: `if: startsWith(github.ref, 'refs/tags/v') && github.event_name == 'push'`. Every `workflow_dispatch` (any branch, any `dry_run`) skips it and never reads the signing secret; build and verification jobs keep no job-level `if:` and still run on dispatch. Supersedes the dispatch dry-run signing path of `docs/security/RELEASE_SECURITY.md` §9. Guard: `m3_f_f03_fail_closed_verification_guard` (release gating) in `tests/security_guard.rs` | Implemented |
| `ADR-72` | From v1.4.0 a GitHub release publishes the AppImage, DEB, RPM and Flatpak bundle next to the portable `tar.gz` and the Windows ZIP/EXE/MSI (owner decision 2026-11-27; supersedes the CI-only rule of `ADR-54`, `ADR-63` and M3-I). `Publish Release Assets` needs every build and verification job of the workflow; each Linux package is taken from the artifact its verification jobs downloaded, must be found exactly once and must match the SHA-256 written by its build job. The staging holds exactly eight assets, `SHA256SUMS.txt` lists all eight and the signed release manifest carries one entry per asset (`appimage`, `deb`, `rpm`, `flatpak`). Released updaters keep selecting `tar.gz`/`zip`/`inno-exe`/`msi`; package installs are not updated in the application (DEB/RPM by the package manager, Flatpak by Flatpak, AppImage by replacing the file). No package signature, no APT/DNF repository and no Flathub submission. Guards: `release_workflow_manifest_template_is_accepted_by_the_updater`, `linux_in_app_update_is_limited_to_the_portable_installation_guard` and the package guards in `tests/security_guard.rs` and `tests/linux_*_packaging_test.rs` | Implemented |

### 2.9 P0-F14 safety decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-72` | The HIL transmit pulse range is validated by a total, `Result`-returning helper **before** PTT ON. An inverted or below-minimum range is a typed `SafetyViolation`, never a `clamp` panic. `HIL_MIN_TEST_DURATION_MS = 10` and `HIL_MAX_TEST_DURATION_MS = 500` are manifest constants, and the manifest rejects values outside them plus non-finite power/SWR limits. | Implemented |
| `ADR-73` | Every controlled path after logical PTT ON attempts exactly one PTT OFF. A failed, timed-out or unconfirmed release is reported as an error (`HilReleaseOutcome::{Failed,Unconfirmed}`), never as success. A cancelled/abandoned transmit guard performs one non-blocking emergency release in `Drop` (single poll with `Waker::noop()`; no runtime creation, no blocking I/O, no fabricated `ConfirmedRx`), because `panic = "abort"` makes unwind-based cleanup unavailable in release. | Implemented |
| `ADR-74` | Test-only protocol doubles are confined to `tests/`/`tests/support/` and `#[cfg(test)]`. No production enum variant, feature flag, GUI/CLI entry, configuration key, environment variable or fallback may select them; they cannot be compiled into a release artifact. | Implemented |

### 2.10 CAT-018 / CAT-019 protocol-conformance decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-75` | Kenwood PTT is resolved **per model** through an explicit documented contract (`KenwoodPttContract`): ON `TX;`, OFF `RX;`, and a status-read channel that is `IF` only for TS-590S/TS-590SG. There is no brand-wide Kenwood command assumption; a model without a document is refused before any byte is written. | Implemented |
| `ADR-76` | `TX P1;` is decoded as the documented **transmission-mode** answer: `P1=0` (`SEND`), `1` (`DATA SEND`) and `2` (`TX Tune`) all mean transmit; `RX;` means receive; any other frame (including a bare `TX;`) produces no PTT state. The former `TX0; = receive` mapping and the "PTT read" helper that returned the transmit command `TX;` are removed. | Implemented |
| `ADR-77` | Write success is never a confirmed TX. `state.ptt`/`confirmed_ptt` change only on a documented answer (`TX P1;`, `RX;` or the documented `IF` P8 status read for the models that define it). An accepted `TX;` leaves a release obligation (`ptt_commanded`) that `disconnect()` discharges with exactly one documented `RX;`. | Implemented |
| `ADR-78` | The documented error messages `?;`, `E;`, `O;` (p. 3 of every reference) abort the operation immediately with a typed protocol error; PTT ON is never retried. The documented `ID;` command replaces `IF;` as the connection-probe fallback, and the S-meter read form is model-specific (`SM0;` vs `SM;`). | Implemented |
| `ADR-79` | FLRig 2.0.00 is the version contract. `rig.set_ptt` (`n:i`) is sent with one `<int>` parameter (`1`/`0`); the documented nil response is the only success shape; any `<value>` (including the `<int>0</int>` "transceiver offline" answer), a fault, malformed XML, a non-200 HTTP status or a timeout is an error. No boolean/int fallback and no type-based retry exist. | Implemented |
| `ADR-80` | `rig.get_ptt` (`i:n`) accepts only the documented `<int>` `0`/`1`; other XML-RPC types are rejected as protocol errors (no implicit coercion). `rig.set_verify_ptt` (`n:i`) and the deprecated `rig.set_ptt_fast` (`n:i`) are recorded but not adopted. | Implemented |

### 2.11 P1-F15 HIL readiness and independent-evidence decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-81` | No HIL safety field may default to `true`. Every safety boolean is a required field, so an absent value is a deserialisation error, never an affirmative assertion. `#[serde(default = "default_true")]`, `unwrap_or(true)` and equivalent fallbacks are forbidden in the HIL safety types and pinned by `security_guard`. | Implemented |
| `ADR-82` | The test plan (`HilTestBenchManifest`), the operator declaration (`HilAttestationDocument`), the independent evidence (`HilIndependentEvidence`) and the authorization decision (`HilAuthorizationDecision`) are four separate layers. The plan cannot create, hold or validate evidence; the operator declaration is never objective evidence. | Implemented |
| `ADR-83` | Hardware execution is authorized only by `HilHardwareExecutionAuthorizer::authorize_at`, which requires an evidence document bound by SHA-256 to the plan **and** to the declaration, scoped to the test case, rig, session, attempt and validity window, covering every `HIL_REQUIRED_MEASURED_FACTS` entry, and issued by a listed authority. A SHA-256 digest is an integrity hash, never a signature. | Implemented |
| `ADR-84` | The authorization default is `NotEvaluated`. Missing, malformed, expired, not-yet-valid, out-of-scope, replayed, provider-error, provider-timeout and cancelled evidence are controlled denials. `DryRunOnly` never keys PTT, and a readiness report never authorizes PTT. | Implemented |
| `ADR-85` | `HilIndependentEvidence` and `HilAuthorizationDecision` are deliberately **not** `Deserialize`, so an untrusted document cannot forge an authorization. No production evidence provider exists; the test-only evidence constructor lives exclusively under `#[cfg(test)]`, and no CLI flag, GUI control, configuration key, environment variable or feature flag can select it. | Implemented |
| `ADR-86` | A document without `schema_version` is legacy v1: it still parses as a plan, requires an explicit migration and stays denied. Migration never promotes a missing field to `true`, never creates evidence and never converts a historical `true` into a current consent. | Implemented |
| `ADR-87` | The remediation is software-only. Production default = DENIED, hardware readiness = NOT_VERIFIED, `NATIVE-7B = NOT_STARTED`; the ten hardware-in-the-loop tests stay `#[ignore = "hardware-in-the-loop"]` and no real PTT or RF is executed. | Implemented |

### 2.12 P1-F17 explicit PTT interaction decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-88` | PTT is never bound to a single key. `F7` alone, `Shift+F7` and `Alt+F7` perform no action; the transmit directions are two explicit intents with at least one modifier: `Ctrl+Shift+F7` requests TX ON and `Ctrl+F7` requests TX OFF. No `Alt`-based combination is used, so no `AltGr`/`Ctrl+Alt+F*` collision exists on Polish or Central-European layouts. | Implemented |
| `ADR-89` | PTT is not an ambiguous toggle. `PttIntent::RequestOn` and `PttIntent::RequestOff` are disjoint actions; the transmitting direction deliberately needs the harder combination, so stopping transmission is always easier. | Implemented |
| `ADR-90` | Both PTT bindings require `Modifiers::matches_exact` and a non-repeat `pressed: true` event. egui's default `matches_logically` (which ignores extra `Shift`/`Alt`) is not used for PTT, so an unexpected modifier is an explicit mismatch. | Implemented |
| `ADR-91` | Both PTT intents are blocked by text focus (`egui_wants_keyboard_input` and `text_edit_focused`), by an active IME composition, by a window without focus and by any open modal window. The gates are computed once per frame, and an unreadable gate fails closed. | Implemented |
| `ADR-92` | Key repeat never creates a new intent, a release event never creates an intent, and the matching press is removed from the frame buffer so it is consumed exactly once. | Implemented |
| `ADR-93` | Every PTT user interaction (shortcut, on-screen buttons, command palette) goes through one central dispatcher (`src/gui/ptt_input.rs`). The GUI contains no toggle entry point, no `RigCommand::SetPtt` and no direct `SetPtt(true)`; the direct runtime transmit call sites in `src/gui` are a closed, guard-pinned set of programmatic transmitters. | Implemented |
| `ADR-94` | PTT ON is forwarded only for the F-15 outcome `HardwareExecutionAuthorized`. The production GUI gate `evaluate_gui_ptt_authorization()` lives inside the F-15 module, takes no input and always returns `NotEvaluated`; `src/gui` may never name the authorized outcome. PTT OFF is never authorization-gated, because a blocked OFF could leave a transmitter keyed. | Implemented |
| `ADR-95` | The GUI TX indicator is telemetry-derived (`TxArbiterState::Owned`), never optimistic. A denial, an arbiter error or a port error never presents TX and is never reported as success. Losing window focus while transmitting performs exactly one controlled OFF attempt through the existing `TxArbiter` path, with no retries and no wall-clock timer. | Implemented |
| `ADR-96` | The remediation is software-only. Single-key PTT = REMOVED, text/IME/window-unfocused/repeat PTT = BLOCKED, PTT without F-15 authorization = BLOCKED, `PTT_ON_COUNT_WITHOUT_AUTHORIZATION = 0`, hardware interaction = NOT_VERIFIED, `NATIVE-7B = NOT_STARTED`; the ten hardware-in-the-loop tests stay `#[ignore = "hardware-in-the-loop"]` and no real PTT or RF is executed. | Implemented |

## 3. Update state machine and data classification (reference)

- 20 states with strict transitions; `Committed` is the only terminal-success state. A
  failure before `Applying` leads to `FailedPreApply` (no file rollback); a failure at or
  after `Applying` requires rollback.
- Installation types: `WindowsPortableZip`, `WindowsInnoSetup`, `WindowsMsi`,
  `LinuxPortableTarGz`, `LinuxAppImage`, `LinuxDeb`, `LinuxRpm`, `LinuxFlatpak`,
  `UnknownInstallation`. Self-managed: ZIP, tar.gz, AppImage. System-managed: MSI, Inno, DEB,
  RPM, Flatpak. No system-managed format supports self-apply.
- Data classes: `ProgramFile`, `RuntimeLibrary`, `UserConfiguration`, `UserData`, `ReferenceData`, `ReleaseMetadata`, `Unknown` (`FirstInstallSeed` removed in `R1C`).

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the module map and
[`../security/RELEASE_SECURITY.md`](../security/RELEASE_SECURITY.md) for the release
security contract.

### 2.13 P1-F16 panic-free logbook pagination decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-97` | Page count, normalized page and the `start..end` window are produced by one pure, GUI-independent model (`PaginationInput` → `PaginationWindow` in `src/gui/pagination.rs`). No renderer, reload path or filter path computes its own page count or range. | Implemented |
| `ADR-98` | `LogViewState::pagination_window` normalizes the page before slicing and writes it back. A stale page becomes the last existing page; a still-valid page is preserved (filtering expansion does not reset to page 0); an empty result is page 0 with `page_count = 0` and range `0..0`; `page_size = 0` is the defined all-rows mode with `effective_page_size >= 1`. Existing explicit resets (`drill_down`, `clear_drill_down`, page-size change) stay explicit. | Implemented |
| `ADR-99` | Page arithmetic uses `checked_mul` / `checked_add`. Overflow is impossible for a normalized page; the defensive `None` branch returns a safe window with `arithmetic_guard_triggered = true`, never a panic, a wrapped index or a silent truncation. `saturating_*` is not used as a substitute for a correct state. | Implemented |
| `ADR-100` | All page access uses `slice::get` (`PaginationWindow::slice_of`, `page_indices.get(row.index())`, `recent_qsos.get(idx)`). Direct indexing of `sorted_indices`/`recent_qsos` with a page range, `get_unchecked`, `catch_unwind` and `unwrap`/`expect` on a checked access are forbidden. An inconsistent cache is rebuilt once and otherwise yields a controlled empty view with a diagnostic. | Implemented |
| `ADR-101` | Selection is keyed by the stable QSO `id` (`INTEGER PRIMARY KEY AUTOINCREMENT`), never by a page or filtered index. Deleting a record removes exactly that ID; a filtered-out selection is preserved by ID and is never replaced by a different QSO. | Implemented |
| `ADR-102` | The remediation is software-only and uses synthetic data only. `REAL_USER_DATA_USED=false`; hardware readiness and hardware PTT remain NOT_VERIFIED; the ten HIL tests stay `#[ignore = "hardware-in-the-loop"]`; `NATIVE-7B = NOT_STARTED`; F-14, CAT-018, CAT-019, F-15 and F-17 stay CLOSED. Verified by `tests/p1_f16_logbook_pagination.rs` and the `p1_f16_*` guards. | Implemented |

### 2.14 P1-F19 role-aware SOTA/POTA reference decisions

| ID | Decision | Status |
|---|---|---|
| `ADR-103` | A reference always has an explicit side. `core::award_refs::StationSide` fixes `Local` = the SPLogbook station and `Remote` = the correspondent; the model `AwardReferences { sota: AwardRefPair, pota: AwardRefPair }` keeps both sides as independent `Option` values. No importer, exporter or form derives one side from the other, and a missing side stays `None`. | Implemented |
| `ADR-104` | Exactly one mapping exists: `AwardReferences::adif_fields()` produces every reference field and `import_adif_references()` consumes every reference field. `src/core/adif.rs` names no reference field and delegates both directions; a second parallel mapping is forbidden and pinned by `security_guard`. | Implemented |
| `ADR-105` | SOTA follows ADIF 3.1.8: the logging station's summit is `MY_SOTA_REF`, the contacted station's summit is `SOTA_REF`. Activator, chaser and summit-to-summit keep their perspective; a summit is never carried in `MY_SIG_INFO`/`SIG_INFO` (SOTA itself uses those only as a location hint). | Implemented |
| `ADR-106` | The POTA production contract is the representation POTA documents: `MY_SIG=POTA` + `MY_SIG_INFO=<own park>` and `SIG=POTA` + `SIG_INFO=<other park>`. A marker is never emitted without its `_INFO` partner, and `SIG=POTA` is never emitted without a remote reference. The ADIF 3.1.8 `MY_POTA_REF`/`POTA_REF` (`POTARefList`, since ADIF 3.1.4) remain importable and are the only carrier for a multi-park value. | Implemented |
| `ADR-107` | Exactly one POTA representation is emitted per record, chosen deterministically by the data: a comma list on either side selects the direct `POTARefList` fields, otherwise the SIG pair. Two representations could contradict each other, so they are never emitted together; this makes `export → import → export` a fixed point. | Implemented |
| `ADR-108` | Consistent POTA representations (direct field and SIG pair) are merged without loss, comparing case-insensitively and ignoring list order. Contradictory ones reject that record with `AdifRecordError::ConflictingPotaRepresentation`, which names both values and the side. No value is chosen arbitrarily, no side is swapped, and the source file keeps the original data. An uninterpretable `SIG` value is reported as an import notice, never guessed. | Implemented |
| `ADR-109` | The QSO entry state carries the sides explicitly: `my_sota`/`my_pota` are the activation context, prefilled from the station profile and preserved across the per-QSO reset; `sota`/`pota` describe the correspondent and are filled by a spot or by typing. `SpLogApp::draft_qso_from_entry` is the single pure, clock-free form→record mapping used by both the save path and the tests. A spot of another activator never writes the local side. | Implemented |
| `ADR-110` | The SOTA CSV V2 export uses each record's own `MY_SOTA_REF` for column 3 (`[My Summit]`) and falls back to the activation-context value only when the record has none; column 9 (`[His Summit]`) is `SOTA_REF`. A chaser row leaves column 3 empty. One dialog value can no longer overwrite a per-record summit. | Implemented |
| `ADR-111` | No schema change and no historical rewrite: each role already has its own nullable column (`my_sota_ref`, `sota_ref`, `my_pota_ref`, `pota_ref`). Rows written before the fix keep their stored values; no automatic repair or irreversible migration is performed. | Implemented |
| `ADR-112` | The remediation is software-only and uses synthetic data only. `REAL_USER_DATA_USED=false`, `EXTERNAL_SOTA_UPLOAD_EXECUTED=false`, `EXTERNAL_POTA_UPLOAD_EXECUTED=false`; no account is used and no file is uploaded. Hardware readiness and hardware PTT remain NOT_VERIFIED; the ten HIL tests stay `#[ignore = "hardware-in-the-loop"]`; `NATIVE-7B = NOT_STARTED`; F-14, CAT-018, CAT-019, F-15, F-16 and F-17 stay CLOSED. Verified by `tests/p1_f19_award_references.rs` and the `p1_f19_*` guards. | Implemented |

### 2.15 P1-F20 journal-scoped upload selection decisions

| ID | Value | Rationale |
|---|---|---|
| `ADR-113` | Upload scope is defined by a materialised `UploadPlan` carrying `journal_id`, a deterministic `plan_id` and a lossless batch partition | A selector cannot express "exactly these records"; a plan can, and it is immutable against journal switches and later edits |
| `ADR-114` | Journal identity is `journals.id`; journal names and GUI indices are not identities | Names are user-editable and indices are display state |
| `ADR-115` | Eligibility is `sent NOT IN ('Y','I')` per the ADIF `QSL Sent` enumeration | `Y` officially means "uploaded to, and accepted by, the online service"; `I` means ignore/invalid |
| `ADR-116` | Batch sizes are client transport policy, not service limits (LoTW 500, eQSL 250, Club Log 5000, QRZ 1000, per-QSO 100) | No record-count limit is documented for TQSL or the eQSL real-time interface; the ≈1000 figure belongs to the manual browser upload |
| `ADR-117` | TQSL exit codes are mapped explicitly; only `0` is acceptance, `9` is partial, `8` is no-records-processed, `1` is cancellation | Based on ARRL's documented `Status Information` table |
| `ADR-118` | eQSL acceptance requires `Result: x out of y records added` with `x == y == records sent`; HTTP 200 alone is not acceptance | Based on eQSL's documented `RETURN PAGE FORMAT` section |
| `ADR-119` | No database migration: `<service>_qsl_sent = 'Y'` is the acceptance marker; `LOTW_QSLSDATE`/`EQSL_QSLSDATE` stay unpersisted | Avoids an unnecessary schema migration; the existing columns already make selection idempotent |
| `ADR-120` | No automatic upload retry; recovery is idempotent re-derivation of not-yet-accepted records | Both services tolerate duplicates, but an ambiguous outcome must never be reported as success |
| `ADR-121` | Test transports live only in `#[cfg(test)]`; no env/CLI/config/GUI path may select them | A production-reachable test transport is forbidden |

Traceability: `src/cloud/upload_plan.rs`, `src/core/database/upload_selection.rs`,
`src/cloud/lotw.rs`, `src/cloud/eqsl.rs`, `src/gui/online_sync_upload.rs`,
`tests/p1_f20_upload_selection.rs`, `tests/security_guard.rs` (`p1_f20_*`),
`docs/UPLOAD_SELECTION.md`.

## D-20R-01 — durable upload checkpoints instead of re-deriving the eligible set

**Context.** F-20 documented "recovery is idempotent re-derivation". That cannot distinguish *never
sent* from *sent and accepted but not recorded*, and offers no evidence for `Unknown`.

**Decision.** Persist plan identity, ordered batch membership, per-batch state and the acceptance
instant. Recovery is driven exclusively by durable state. `InFlight` becomes `Unknown`; `Accepted`
completes the local commit without re-uploading; `Completed` is never resent.

**Alternatives rejected.** (a) Re-derivation — cannot answer the recovery question. (b) A single
`last_plan` row — cannot represent concurrent plans or partial progress. (c) Storing the ADIF payload
in the checkpoint — unnecessary, and it would duplicate user data outside the QSO table.

## D-20R-02 — at-most-once / unknown instead of "exactly once"

**Context.** Neither LoTW nor eQSL documents upload idempotency; LoTW explicitly cannot delete a
submitted QSO, and eQSL's duplicate rejection is documented as a `Warning`, not as an idempotent
guarantee.

**Decision.** Model the upload as *at-most-once for `Completed`* and *unknown for `InFlight`*. Never
retry `Unknown` automatically. Never use the term "exactly-once".

## D-20R-03 — the sent date is the acceptance date, not the QSO date

**Context.** ADIF defines `*_QSLSDATE` as the date the QSL was *sent* and gates it on status
`Y`/`Q`/`I`; no service documents a "date sent". SPLogbook holds exactly one provable instant: the
moment it obtained durable acceptance evidence (`accepted_at`).

**Decision.** Write `*_QSLSDATE` from `accepted_at` in the same transaction as the status, using
`COALESCE` so a retry cannot overwrite the original. Never derive it from `QSO_DATE`, from the commit
clock, from the migration time or from the recovery time. Historical `Y` rows keep `Y + NULL`.

## D-20R-04 — keep query credentials where the official contract requires them

**Context.** Both documented download interfaces accept credentials only in the query string. POST,
Basic auth, sessions, cookies and API keys are undocumented for those endpoints.

**Decision.** Do not change the method and do not invent an endpoint. Confine the exposure
(`SecretUrl` with redacted `Debug`/`Display`, redirects disabled, sanitised errors, no logging, no
persistence) and publish the limitation with the precise status vocabulary. `CREDENTIALS_NOT_IN_QUERY`
must never be claimed.

**Revisit trigger.** A new official document that specifies POST, a session/cookie flow or an API
key for either download endpoint.

## D-20R-05 — migration transactions use `BEGIN IMMEDIATE`

**Context.** The previous loop used a deferred transaction, so a concurrent writer could raise
`SQLITE_BUSY` after earlier DDL had already executed inside the same migration.

**Decision.** Use `TransactionBehavior::Immediate`. The official SQLite documentation guarantees that
after `BEGIN IMMEDIATE` no subsequent statement returns `SQLITE_BUSY`, closing the partially-applied
migration window.