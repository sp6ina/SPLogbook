# SPLogbook — Public Roadmap

This roadmap states what is delivered, what is not, and what is planned. It contains no
working plans, prompts or session notes; those stay outside the repository.

Status vocabulary: **DELIVERED** (implemented and verified), **CLOSED** (a defect or risk
that has been remediated and must not be reopened without a reproducible regression),
**OPEN** (known and tracked), **NOT_STARTED** (no implementation work exists).

---

## 1. Current version state

| Element | Value |
|---|---|
| Public stable | **1.2.0** — annotated tag `v1.2.0`, immutable (object `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled commit `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`) |
| Development | **1.3.0-alpha.1** — **not published** (no tag, no release, no assets) |
| Release pipeline | artifacts are built and verified by CI; publication is gated on a `v*` tag and a separate authorization |
| Public release audit | [`docs/security/HISTORICAL_RELEASE_AUDIT.md`](security/HISTORICAL_RELEASE_AUDIT.md) |

## 2. Supported platforms

| Platform | Minimum | Packages |
|---|---|---|
| Windows 10/11 x86_64 | — | portable ZIP (published path), Inno Setup user setup, WiX MSI (both implemented, not CI-published) |
| Linux x86_64 | kernel **7.0**, glibc **2.43** | portable `tar.gz`, AppImage, DEB, RPM, Flatpak (`io.github.sp6ina.SPLogbook`, runtime `org.freedesktop.Platform` 26.08) |
| Linux distributions | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed | — |
| Not supported | Debian 13 (glibc 2.41), openSUSE Leap 16.1 (glibc 2.40), Ubuntu interim 26.10 | — |

Binding source of truth: `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`. The minimum is
recomputed per release from the oldest default kernel line among the supported
distributions and is never raised without technical proof. Unstable channels (`testing`,
Rawhide, backports, release candidates) never set the minimum.

## 3. Delivered milestones

| ID | Scope | Status | Durable outcome |
|---|---|---|---|
| M1 | Update safety and data preservation | **DELIVERED** | manifest-first asset selection; update-plan data classification; `UNKNOWN` never overwritten |
| M2 | Installer design and owner decisions | **DELIVERED** | three approved design documents (Windows, Linux, platform contract) |
| M3-A | Platform updater core | **DELIVERED** | attempt IDs, 20-state machine, transaction, installation detection, preflight, lock, cancellation, recovery, health proof |
| M3-B | Windows update helper | **DELIVERED** | native `splogbook-updater` protocol v1, hardened PowerShell fallback, journaled replacement, health/rollback |
| M3-C | Windows portable ZIP integration | **DELIVERED** | central-directory ZIP inspection, staging, plan, helper delegation, result consumption on startup |
| M3-D | Windows Inno Setup user setup | **DELIVERED** | per-user install in `%LOCALAPPDATA%`, no elevation, user data preserved |
| M3-E | Windows MSI (WiX) | **DELIVERED** | per-machine MSI, fixed `UpgradeCode`, `msiexec` delegation, conflict detection |
| M3-DEV | Development version identity | **DELIVERED** | `BuildIdentity`, update channels, `--version`/`--build-info`, SemVer downgrade policy |
| M3-F | Linux portable `tar.gz` + support policy | **DELIVERED** | deterministic archive, two-phase extraction, XDG placement, kernel 7.0 / glibc 2.43 policy |
| M3-G | Linux AppImage | **DELIVERED** | pinned official `appimagetool` and type-2 runtime, verified FUSE, 12 negative fixtures |
| M3-H | Linux DEB | **DELIVERED** | `splogbook` DEB under `/opt/SPLogbook` + `/usr/bin/splogbook`, 27 negative fixtures, full lifecycle |
| M3-I | Linux RPM | **DELIVERED** | deterministic RPM, bundled Hamlib with `RUNPATH`, 34 negative fixtures, lifecycle on Fedora 45 and Tumbleweed |
| M3-J | Linux Flatpak | **DELIVERED** | `io.github.sp6ina.SPLogbook`, pinned `org.freedesktop` 26.08 runtime/SDK, 662 SHA-256 pinned crates, offline locked build, exact minimal sandbox, 48 negative fixtures, full lifecycle |
| — | Dependency security remediation | **DELIVERED** | `RUSTSEC-2026-0459`, `-0486`, `-0523`, `-0524`, `-0538` resolved without new `--ignore` entries |

## 4. Closed risks

- `F-03` (fail-closed native Linux verification): **CLOSED**. `scripts/verify_linux_container.sh`
  treats ELF type, `ldd`, `--version`, `--build-info` and the required `GLIBC_2.43` symbol as
  assertions, installs runtime libraries from the official package manager of each pinned
  image before verifying, and prints `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` only after
  every check passed. A negative CI job proves that a missing library aborts with a non-zero
  exit code and no success message. It must not be reopened without a reproducible
  regression.
- `F-23` (non-deterministic tests): **CLOSED** — virtual time and permit counters replace
  wall-clock dependencies.
- `F-24` (non-hermetic installer tests): **CLOSED** — native E2E is explicit opt-in,
  path- and digest-pinned, and fail-closed.

## 5. Not started / not delivered

| Item | Status | Note |
|---|---|---|
| Flathub submission of the Flatpak | **NOT_STARTED** | deliberately out of scope; the package is a local/CI artifact only and is installed from a bundle |
| Publishing of Windows Inno Setup and MSI installers | **OPEN** (finding `F-05`) | implemented and locally verified; CI builds and publishes only the portable ZIP |
| MSI prerelease versioning for `1.3.0-alpha.2`+ | **OPEN** (decision `OWNER-M3-E-MSI-VERSIONING-01`) | Windows Installer rejects the prerelease version mapping with error 1638 |
| Hardware-in-the-loop validation (NATIVE-7B) | **NOT_STARTED** | 10 tests remain `#[ignore = "hardware-in-the-loop"]`; blocked by the open P0 finding `F-14` |
| Signed packages for DEB/RPM | **NOT_STARTED** | integrity is provided by the release manifest and SHA-256, not by package signatures |

Open defects and limitations are tracked in [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md).

## 6. Governance rules

1. Exactly one milestone is worked on at a time; a milestone starts only after explicit
   authorization.
2. Completed milestones are not reopened without a confirmed, reproducible regression.
3. Code is the factual source of state; a document never overrides code, and a comment never
   overrides execution.
4. Tags and published releases are immutable. A defect is fixed by a new version, not by
   rewriting a published one.
5. Dependency updates and security remediations happen only in dedicated changes with a green
   `cargo audit` before and after.
