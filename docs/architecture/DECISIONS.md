# SPLogbook — Architecture decisions and binding invariants

Consolidated register of owner decisions that are still binding, together with the
architectural invariants they establish. Each row names where the decision is implemented
and which guard enforces it, so the decision can be traced to code and to a test.

Status vocabulary: **IMPLEMENTED** (in code and guarded), **OPEN** (undecided, may block
work), **SUPERSEDED** (replaced by a later decision — kept for history only).

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

## 2. Owner decisions

### 2.1 Installer and updater scope (M2)

| ID | Decision | Status | Implemented in | Guarded by |
|---|---|---|---|---|
| `OWNER-M2-01`, `OWNER-M2-02`, `OWNER-M2-03` | Expanded Windows scope: portable ZIP + WiX MSI + Inno Setup; dual helper (native `splogbook-updater` primary, hardened PowerShell fallback) | IMPLEMENTED | `src/cloud/updater/**` | `m3_b_*`, `m3_c_*` guards |
| `OWNER-M2-04`, `OWNER-M2-05`, `OWNER-M2-06`, `OWNER-M2-07` | Expanded Linux scope (portable tar.gz, AppImage, DEB, RPM, Flatpak declared); package-manager boundary; absolute ban on `sudo`/`doas`/`pkexec`/`runas` and on writes to `/usr` | IMPLEMENTED | `src/cloud/updater/**`, `scripts/**` | `updater_does_not_use_sudo_guard`, `m3_f_*` |
| `OWNER-M2-08`, `OWNER-M2-09` | `.rollback` retention and backup-deletion policy | **OPEN** (non-blocking) | temporary conservative retention: `.rollback` is never auto-deleted and never contains user data | `m3_c_temporary_conservative_backup_retention_guard` |
| `OWNER-M2-10` | Classification of `databases/serviceLOG.db` | **OPEN** (documentation dispute) — code treats it as a first-install seed | `src/cloud/updater/installation.rs` | `portable_database_files_are_protected_guard` |
| `OWNER-M2-11` | `databases/callbook.db` is a first-install seed (create only if absent) | IMPLEMENTED | `src/cloud/updater/**` | `m3_a_data_preservation_matrix_guard` |
| `OWNER-M2-12` | Audit-log retention | **OPEN** | — | — |
| `OWNER-M2-13` | Cancellation is prohibited once `Applying` has started | IMPLEMENTED | `src/cloud/updater/state.rs` | `cancellation_prohibited_from_applying_guard` |
| `OWNER-M2-14` | Native tests on Windows and Linux | IMPLEMENTED | `tests/windows_*`, `tests/linux_*` | — |
| `OWNER-M2-15` | Native test matrix requirement for M3 | IMPLEMENTED | `.github/workflows/build-and-release.yml` | `m3_f_f03_fail_closed_verification_guard` |

### 2.2 Windows installer toolchains

| ID | Decision | Status | Implemented in | Guarded by |
|---|---|---|---|---|
| `OWNER-M3-D-TOOLCHAIN-01` | Inno Setup **7.1.1** at `C:\Program Files\Inno Setup 7\ISCC.exe` (SHA-256 `E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644`), zero compiler warnings tolerated | IMPLEMENTED | `installer/windows/inno/SPLogbook.iss`, `installer/windows/inno/README.md` | `m3_d_inno_setup_toolchain_decision_guard` |
| `OWNER-M3-E-TOOLCHAIN-01` | WiX Toolset SDK **7.0.1** with .NET 8 SDK / MSBuild 2022 SDK resolver; EULA accepted once and recorded in-repo | IMPLEMENTED | `installer/windows/msi/SPLogbook.wixproj`, `installer/windows/msi/README.md` | `m3_e_msi_toolchain_decision_and_identity_guard` |
| `OWNER-M3-E-MSI-VERSIONING-01` | MSI prerelease version mapping | **OPEN** — blocks MSI for `1.3.0-alpha.2` (error 1638) | — | `m3_e_*` version-mapping guards |

### 2.3 Linux support and packaging (M3-F)

All eleven decisions below are **IMPLEMENTED**; `OWNER-M3-F-LINUX-BASELINE-01` and
`OWNER-M3-F-ABI-BASELINE-01` were later **SUPERSEDED** on their numeric values.

| ID | Decision |
|---|---|
| `OWNER-M3-F-LINUX-BASELINE-01` | Rolling latest-stable Linux baseline. **SUPERSEDED** by `OWNER-M3-F-KERNEL-POLICY-OPTION-A` + `OWNER-M3-F-GLIBC-POLICY-01` |
| `OWNER-M3-F-KERNEL-POLICY-01` | Kernel floor model. **SUPERSEDED** by `OWNER-M3-F-KERNEL-POLICY-OPTION-A` |
| `OWNER-M3-F-KERNEL-POLICY-OPTION-A` | Kernel minimum is the oldest default kernel line among supported distributions — currently **Linux 7.0**, set by Ubuntu 26.04 LTS. Replaces both the dynamic `6.12` floor and the fixed `7.2` |
| `OWNER-M3-F-GLIBC-POLICY-01` | glibc minimum **2.43**; maximum required symbol `GLIBC_2.43`; glibc is never bundled and is never rebuilt to an older symbol set |
| `OWNER-M3-F-ABI-BASELINE-01` | Cross-distribution ABI verification. **SUPERSEDED** on the numeric glibc baseline by `OWNER-M3-F-GLIBC-POLICY-01` |
| `OWNER-M3-F-ARCHITECTURE-01` | Platform adapters and layered updater architecture |
| `OWNER-M3-F-LIBRARY-POLICY-01` | Only Hamlib may be bundled (`libhamlib.so.4`); all other runtime libraries come from the distribution |
| `OWNER-M3-F-DYNAMIC-LIBRARIES-01` | Private Hamlib with `RUNPATH=$ORIGIN/lib`; no `RPATH`; no system Hamlib dependency |
| `OWNER-M3-F-PORTABLE-LAYOUT-01` | One canonical portable artifact, a single root directory `SPLogbook/`, and an installation marker |
| `OWNER-M3-F-UPDATE-MODE-01` | Self-managed update for portable tar.gz and AppImage via journaled transactional replacement |
| `OWNER-M3-F-ARCHIVE-EXTRACTION-01` | Two-phase safe extraction: read-only preflight, then extraction into a cleaned staging directory, with entry/size/type limits |
| `OWNER-M3-F-XDG-DATA-01` | User data in XDG Base Directories; absolute `XDG_*` enforced with `$HOME` fallbacks; no silent migration of legacy portable data |
| `OWNER-M3-F-SIGNING-01` | Release integrity through the signed manifest and SHA-256; no package signing for DEB/RPM |
| `OWNER-M3-F-DESKTOP-INTEGRATION-01` | Unprivileged, user-scope desktop integration (`share/applications/splogbook.desktop`) |
| `OWNER-M3-F-SCRIPT-STANDARDS-01` | Build and verify scripts are `set -euo pipefail`, deterministic, and free of `sudo`/`pkexec`/`doas`/`su` |
| `OWNER-M3-F-TEST-MATRIX-01` | Deterministic stability loops (100/50/20 iterations) and ephemeral test keys only; production keys are forbidden in tests |
| `OWNER-M3-F-GIT-WORKFLOW-01` | Fast-forward only; no rebase, merge commit or force-push; `v1.2.0` immutable; no premature `v1.3.0` |

Consequences of the Linux support policy:

- Supported: **Ubuntu 26.04 LTS** (kernel 7.0.0, glibc 2.43), **Fedora 45** (7.2, 2.44),
  **Arch Linux** (7.2, 2.44), **openSUSE Tumbleweed** (7.2, 2.44).
- Not supported: **Debian 13.7** (glibc 2.41) and **openSUSE Leap 16.1** (glibc 2.40) — they
  do not meet the ABI requirement. **Ubuntu 26.10** interim is out of scope.
- Snapshot contract: `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json` (`splogbook-linux-support-snapshot-v2`).

### 2.4 Verify-and-close decisions (M3-F PCV / F-03)

| ID | Decision | Status |
|---|---|---|
| `OWNER-M3-F-PCV-01` | Repository state is described with four explicit classes: tracked, untracked, ignored, strict | IMPLEMENTED |
| `OWNER-M3-F-PCV-02` | The local working directories are owner-local and must stay out of the public tree | IMPLEMENTED |
| `OWNER-M3-F-PCV-03` | A run with 7 profiles is never described as 4 profiles; false-PASS evidence is classified as such | IMPLEMENTED |
| `OWNER-M3-F-PCV-04` | One documentation commit per consistency change | IMPLEMENTED |
| `OWNER-M3-F-PCV-05` | Fast-forward-only push after a green baseline | IMPLEMENTED |
| `OWNER-M3-F-PCV-06` | Completed milestones are not reopened without reproducible regression | IMPLEMENTED |
| `OWNER-M3-F-PCV-07` | Native evidence is classified by origin (userspace/CI) and never claimed as kernel proof | IMPLEMENTED |
| `OWNER-M3-F-F03-01` | Authorization for the fail-closed native Linux verification gate | IMPLEMENTED (F-03 **CLOSED**) |

### 2.5 AppImage toolchain pins (M3-G)

| ID | Decision | Status |
|---|---|---|
| `OWNER-M3-G-01` | Authorization for the AppImage integration milestone | IMPLEMENTED |
| `OWNER-M3-G-03-R1` | `appimagetool` = official `appimagetool` repository, channel `continuous`, commit `11d54c9acc7f246df1398050a4b880314ac76862`, SHA-256 `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` | IMPLEMENTED |
| `OWNER-M3-G-04-R1` | Type-2 runtime = channel `continuous`, commit `fedfad51acc2860b2916a1eef32ba4a500ff31ef`, SHA-256 `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46`, passed via explicit `--runtime-file` | IMPLEMENTED |
| `OWNER-M3-G-08`, `OWNER-M3-G-09` | Use the latest *verified* official builds; no fallback to an older tool release | IMPLEMENTED |

### 2.6 DEB package contract (M3-H)

All ten decisions are **IMPLEMENTED** and verified by `tests/linux_deb_packaging_test.rs` and
`m3_h_deb_integration_guard`.

| ID | Decision |
|---|---|
| `OWNER-M3-H-01` | Binary package only (no source package), built with `dpkg-deb -Zxz` from the pinned `ubuntu:26.04` image |
| `OWNER-M3-H-02` | Identity `splogbook`, version `1.3.0~alpha.1-1`, architecture `amd64`, section `hamradio` |
| `OWNER-M3-H-03` | Layout `/opt/SPLogbook` plus the launcher `/usr/bin/splogbook`; lintian suppression limited to `dir-or-file-in-opt,custom-library-search-path` |
| `OWNER-M3-H-04` | Desktop entry and 256×256 icon installed to the standard XDG locations |
| `OWNER-M3-H-05` | Installation marker `.splogbook-deb-marker` carrying the build identity |
| `OWNER-M3-H-06` | Fail-closed launcher: no `sudo`, no `LD_LIBRARY_PATH` override, absolute `exec` |
| `OWNER-M3-H-07` | `Depends` generated from the ELF closure, then reviewed (17 packages); bundled Hamlib is not a dependency |
| `OWNER-M3-H-08` | Minimal maintainer scripts; no `preinst`/`prerm`; user data preserved across the whole lifecycle |
| `OWNER-M3-H-09` | No package signing; integrity via the release manifest and SHA-256 |
| `OWNER-M3-H-10` | Artifact is a CI artifact only; publishing requires a separate release gate |

### 2.7 Dependency and security decisions

| ID | Decision | Status |
|---|---|---|
| `OWNER-DEP-SECURITY-01` | Use the latest stable compatible dependency versions; never `cargo audit --ignore` as a shortcut | IMPLEMENTED |
| `OWNER-DEP-SECURITY-02` | `rusqlite` keeps the `bundled` feature; system SQLite is forbidden | IMPLEMENTED |
| `OWNER-DEP-SECURITY-03` | Lockfile changes only through cargo (`cargo update -p <pkg> --precise <ver>`), push only after a green `cargo audit` | IMPLEMENTED |
| `OWNER-DEP-RUSTIX-01` | `rustix 1.1.5 → 1.1.6` (targeted) to remove `RUSTSEC-2026-0486` | IMPLEMENTED |
| — | `quinn-proto 0.11.19` / `quinn-udp 0.5.16` to remove `RUSTSEC-2026-0523`, `-0524`, `-0538` | IMPLEMENTED |
| — | `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` to remove `RUSTSEC-2026-0459` | IMPLEMENTED |

## 3. Update state machine and data classification (reference)

- 20 states with strict transitions; `Committed` is the only terminal-success state. A
  failure before `Applying` leads to `FailedPreApply` (no file rollback); a failure at or
  after `Applying` requires rollback.
- Installation types: `WindowsPortableZip`, `WindowsInnoSetup`, `WindowsMsi`,
  `LinuxPortableTarGz`, `LinuxAppImage`, `LinuxDeb`, `LinuxRpm`, `LinuxFlatpak`,
  `UnknownInstallation`. Self-managed: ZIP, tar.gz, AppImage. System-managed: MSI, Inno, DEB,
  RPM, Flatpak. No system-managed format supports self-apply.
- Data classes: `ProgramFile`, `RuntimeLibrary`, `FirstInstallSeed`, `UserConfiguration`,
  `UserData`, `ReferenceData`, `ReleaseMetadata`, `Unknown`.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the module map and
[`../security/RELEASE_SECURITY.md`](../security/RELEASE_SECURITY.md) for the release
security contract.
