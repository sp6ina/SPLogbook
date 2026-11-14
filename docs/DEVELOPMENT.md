# SPLogbook — Development guide

Public, durable instructions for building, testing and maintaining SPLogbook.
For the product overview see [`README.md`](../README.md); for packaging see the
per-format documentation under [`installer/`](../installer); for security rules see
[`SECURITY.md`](../SECURITY.md) and [`SECURITY_PROCESS.md`](SECURITY_PROCESS.md).

---

## 1. Toolchain and platform pins

| Element | Value | Source of truth |
|---|---|---|
| Rust edition | `2024` | `Cargo.toml` |
| MSRV (minimum supported Rust version) | **1.99.0** | `Cargo.toml` (`rust-version`) |
| Pinned toolchain used by CI | **1.99.0** (components `rustfmt`, `clippy`) | `.github/workflows/ci.yml` |
| Package version (development) | `1.3.0-alpha.1` | `Cargo.toml` |
| Public stable version | `1.2.0` (annotated tag `v1.2.0`, immutable) | `git tag` / [`CHANGELOG.md`](../CHANGELOG.md) |
| Targets | Windows 10/11 x86_64, Linux x86_64 | [`LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md`](../LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md) |

`Cargo.toml` and `Cargo.lock` change **only** through a dedicated, justified change
(a version bump or a dependency security gate). Never hand-edit `Cargo.lock`; use
`cargo update -p <package> --precise <version>`. Never edit `Cargo.toml` merely to make
a build or a test pass.

## 2. Build

```bash
# Windows
cargo build --release            # target/release/SPLogbook.exe

# Linux (x86_64)
cargo build --release            # target/release/SPLogbook
```

Linux build dependencies (Debian/Ubuntu naming): `pkg-config`, `libasound2-dev`,
`libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libxi-dev`, `libxkbcommon-dev`,
`libxkbcommon-x11-dev`, `libgl1-mesa-dev`, `libfontconfig1-dev`, `libwayland-dev`,
`libgtk-3-dev`, `libusb-1.0-0-dev`, `libudev-dev`.

Three binaries are produced:

| Binary | Role |
|---|---|
| `SPLogbook` | GUI application (`--version`, `--build-info`, `--check-health-startup`) |
| `splogbook-updater` | Windows self-update helper (`--request`, `--validate-request-only`, `--diagnostic-version`) |
| `splogbook-signer` | Release manifest signing/verification (`keygen`, `fingerprint`, `sign`, `verify`) |

## 3. Test and verification baseline

Run the full baseline exactly as CI does:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo audit --deny warnings --no-yanked
cargo doc --no-deps
cargo tree -d
```

The single source of truth for the current verified counters, their measurement date and
the canonical counting method is [`VERIFICATION_BASELINE.md`](VERIFICATION_BASELINE.md).
Do not copy counters into other documents; link to that file instead.

Host notes (measured on the owner's Windows workstation, not a product limitation):

- Build with `-j 1` on this host; `-j 2` fails with page-file `os error 1455`.
- Long test runs should use `TMP`/`TEMP` inside `target/` (for example
  `target/m3isec-tmp`) when the system temp directory is subject to
  `PermissionDenied`.
- The Windows sandbox used for automated work runs at Low Mandatory Integrity Level
  (`S-1-16-4096`), so writes to `HKCU` and the Start Menu fail with `0x80070005`.
  Native Windows installer end-to-end tests are therefore explicit opt-in and are
  **not** evidence of a passing installer on that host.

### 3.1 Native installer end-to-end tests are explicit opt-in

| Test | Opt-in | Artifact path | Optional digest |
|---|---|---|---|
| Inno Setup user setup | `SPLOG_RUN_INNO_E2E=1` | `SPLOG_INNO_INSTALLER_PATH` (absolute) | `SPLOG_INNO_INSTALLER_SHA256` |
| WiX MSI | `SPLOG_RUN_MSI_E2E=1` | `SPLOG_MSI_INSTALLER_PATH` (absolute) | `SPLOG_MSI_INSTALLER_SHA256` |

Without opt-in the test reports `NATIVE INSTALLER E2E NOT REQUESTED` and passes as an
**explicitly unexecuted fixture** — never as a native PASS. With opt-in, any of: missing
artifact, wrong path, wrong extension, wrong name, wrong version, wrong digest, non-zero
exit code, missing log, missing install/uninstall proof, or changed user data is
**FAIL-CLOSED**. Never ignore an exit code, never append `|| true`, never replace a check
with an unconditional success.

The standard baseline must never depend on `dist/` or on any untracked artifact.

### 3.2 Hardware-in-the-loop tests

`tests/cat_native_7.rs` contains 10 tests marked `#[ignore = "hardware-in-the-loop"]`.
They are never executed by the standard baseline and require physical radio hardware,
an attested 50 Ω dummy load and explicit operator consent. See
[`Native CAT.md`](../Native%20CAT.md) and
[`HARDWARE_VALIDATION_MATRIX.md`](HARDWARE_VALIDATION_MATRIX.md).

### 3.3 Process environment in tests

Process environment variables are global to the test process and are unsafe to mutate while
another thread reads them (Rust 2024 marks `std::env::set_var`/`remove_var` as `unsafe`).
Tests must therefore never call them directly.

- All `HOME`, `XDG_*`, `TMP`, `TEMP`, `TMPDIR` and any other process-environment mutation in
  a test binary must go through the shared guard enabled with
  `#[path = "support/process_env.rs"] mod process_env;`.
- The guard acquires one per-process `Mutex`, snapshots the original state (including
  "the variable did not exist"), applies the requested values, and restores everything in
  `Drop` — also while unwinding after a panic. The mutex is recovered from a poisoned state
  so one panicking test cannot cascade into unrelated failures.
- A guard is not re-entrant: `EnvGuard::try_new` returns `EnvGuardError::NestedGuard` for the
  same thread and `EnvGuardError::Busy` when another thread holds the lock. Do not work around
  this with `sleep`, retries or `--test-threads=1`.
- Tests that do not touch the environment keep running in parallel; the lock covers only the
  guarded section.
- `tests/cat_native_7.rs` is the single documented exception: its offline HIL tests serialize
  the disjoint `SPLOG_HIL_*` namespace with their own `tokio::sync::Mutex`, in a separate test
  process. Both mechanisms are enforced by `security_guard`.
- `std::env::set_current_dir` is a process-global mutation as well. No test may call it
  without a restoring guard; `security_guard` rejects new occurrences.

The contract and negative tests for the guard live in
`tests/process_environment_isolation_test.rs`; the verified iteration counts are recorded in
[`VERIFICATION_BASELINE.md`](VERIFICATION_BASELINE.md) §6.

## 4. Local Linux packaging builds

Each package has its own build and verify scripts; every verifier is fail-closed and
prints its success banner only after all assertions have passed.

| Format | Build | Verify | Documentation |
|---|---|---|---|
| Portable `tar.gz` | `scripts/build_linux_portable_tar_gz.sh` | `scripts/verify_linux_container.sh` | [`installer/linux/portable/README.md`](../installer/linux/portable/README.md) |
| AppImage | `scripts/build_linux_appimage.sh` | `scripts/verify_appimage.sh` | [`installer/linux/appimage/README.md`](../installer/linux/appimage/README.md) |
| DEB | `scripts/build_linux_deb.sh` | `scripts/verify_linux_deb.sh` | [`installer/linux/deb/README.md`](../installer/linux/deb/README.md) |
| RPM | `scripts/build_linux_rpm.sh` | `scripts/verify_linux_rpm.sh` | [`installer/linux/rpm/README.md`](../installer/linux/rpm/README.md) |
| Windows `ZIP` | `.github/workflows/build-and-release.yml` | — | [`README.md`](../README.md) |
| Windows Inno Setup | `scripts/build_windows_user_setup.ps1` | — | [`installer/windows/inno/README.md`](../installer/windows/inno/README.md) |
| Windows MSI (WiX) | `scripts/build_windows_msi.ps1` | — | [`installer/windows/msi/README.md`](../installer/windows/msi/README.md) |

Negative-fixture verifiers (`scripts/verify_*_negative_fixture.sh`) mutate disposable
copies of a real artifact and assert that every mutation is rejected. They are evidence
of fail-closed behaviour and never evidence of a successful installation.

## 5. Repository state definitions

Contributors and maintainers must use precise language when describing the working tree.
Four states are distinguished:

| State | Definition |
|---|---|
| `TRACKED_CLEAN` | No modifications to tracked files (`git status --porcelain` empty for tracked paths) |
| `UNTRACKED_PRESENT` | Files exist that are neither tracked nor matched by an ignore rule |
| `IGNORED_PRESENT` | Files exist that match `.gitignore` or `.git/info/exclude` |
| `STRICT_WORKTREE_CLEAN` | `TRACKED_CLEAN` **and** no untracked and no ignored files |

`CLEAN` without a qualifier is ambiguous and must not be used. Local working directories
such as `scratch/` and `.local-work/` are owner-local, are excluded through
`.git/info/exclude` (which is never tracked), and never appear in the public tree.

## 6. Git workflow

- Fetch before any work: `git fetch origin --prune`.
- `origin/main` after fetch is the reference point; code in `origin/main` is the factual state.
- Synchronization is **fast-forward only** (`git merge --ff-only`, `git pull --ff-only`).
- **No rebase**, **no merge commits**, **no force-push**, **no history reset**, no
  `git filter-repo`, no BFG.
- Tags are immutable: never create, move or delete a tag. `v1.2.0` is a protected
  annotated tag — tag object `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled commit
  `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`.
- Releases are published only from a `v*` tag and only through the `publish-release` job
  in `.github/workflows/build-and-release.yml`; publishing requires a dedicated gate.
- Do not use `git clean -x`, `git clean -X`, `git clean -fdx` or `git clean -fdX` when the
  dry run (`git clean -n`) lists owner-local files.

## 7. Documentation policy

Public documentation is limited to material needed to build, test, use, package or
maintain the program:

| Document | Purpose |
|---|---|
| [`README.md`](../README.md) | product overview, features, requirements, installation |
| [`CHANGELOG.md`](../CHANGELOG.md) | public release history |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | contribution workflow |
| [`SECURITY.md`](../SECURITY.md) | security policy and reporting |
| [`DOCUMENTATION_INDEX.md`](DOCUMENTATION_INDEX.md) | index of every tracked document |

Rules:

1. Every tracked Markdown document must be listed in
   [`DOCUMENTATION_INDEX.md`](DOCUMENTATION_INDEX.md) **in the same commit** that adds it.
   `security_guard` enforces this.
2. Counters come only from a measurement and always carry the measurement date.
3. A document must not describe a removed file as existing, and must not link to a path
   that is not tracked.
4. Working notes, prompts, session handoffs, gate evidence logs and owner-local plans
   belong outside the repository, not in the public tree.
5. Documents are written in UTF-8. Do not introduce re-encoded (mojibake) text.
