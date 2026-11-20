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
| Package version | `1.3.0` | `Cargo.toml` |
| Public stable version | `1.3.0` (release tag `v1.3.0`; previous stable `v1.2.0`, immutable annotated tag) | `git tag` / [`CHANGELOG.md`](../CHANGELOG.md) |
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
| Flatpak | `scripts/build_linux_flatpak.sh` | `scripts/verify_linux_flatpak.sh` | [`installer/linux/flatpak/README.md`](../installer/linux/flatpak/README.md) |
| Windows `ZIP` | `.github/workflows/build-and-release.yml` | — | [`README.md`](../README.md) |
| Windows Inno Setup | `scripts/build_windows_user_setup.ps1` | — | [`installer/windows/inno/README.md`](../installer/windows/inno/README.md) |
| Windows MSI (WiX) | `scripts/build_windows_msi.ps1` | — | [`installer/windows/msi/README.md`](../installer/windows/msi/README.md) |

Negative-fixture verifiers (`scripts/verify_*_negative_fixture.sh`) mutate disposable
copies of a real artifact and assert that every mutation is rejected. They are evidence
of fail-closed behaviour and never evidence of a successful installation.

The Flatpak is the only format that builds the Rust binary **inside** its own pinned
runtime (`org.freedesktop.Sdk` 26.08 plus the official Rust SDK extension), because a
Flatpak must link against the runtime's glibc rather than the host's. Its Cargo sources
are pinned by the checksums recorded in `Cargo.lock` and listed in
`installer/linux/flatpak/cargo-sources.json`; regenerate and check that file with

```bash
bash scripts/generate_flatpak_cargo_sources.sh
bash scripts/generate_flatpak_cargo_sources.sh --check
```

`tests/linux_flatpak_packaging_test.rs` fails when the committed list diverges from
`Cargo.lock`. Building or running the package needs Linux with bubblewrap, so it is
verified in CI only (see `docs/KNOWN_ISSUES.md` §4).

## 5. Repository state definitions

Contributors and maintainers must use precise language when describing the working tree.
Four states are distinguished:

| State | Definition |
|---|---|
| `TRACKED_CLEAN` | No modifications to tracked files (`git status --porcelain` empty for tracked paths) |
| `UNTRACKED_PRESENT` | Files exist that are neither tracked nor matched by an ignore rule |
| `IGNORED_PRESENT` | Files exist that match `.gitignore` or `.git/info/exclude` |
| `STRICT_WORKTREE_CLEAN` | `TRACKED_CLEAN` **and** no untracked and no ignored files |

`CLEAN` without a qualifier is ambiguous and must not be used. Local working files and
temporary drafts must stay outside the tracked repository tree.

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
- Always inspect `git clean -n` before running any destructive `git clean` command so
  untracked local files are not accidentally deleted.

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
4. Keep local working notes and temporary drafts outside the repository; only durable
   source code, tests, and technical documentation belong in the tracked tree.
5. Documents are written in UTF-8. Do not introduce re-encoded (mojibake) text.

## P0-F14 — PTT fail-safe invariants (software-only)

The HIL transmit path is required to hold these invariants, and they are enforced by
`tests/cat_f14_fail_safe.rs`, `tests/protocol_conformance.rs` and the `p0_f14_*` rules in
`tests/security_guard.rs`:

1. **No panicking range arithmetic.** The pulse bound is produced by
   `bounded_test_duration(requested, max_ms) -> Result<Duration, HilError>`, which rejects a
   maximum below `HIL_MIN_TEST_DURATION_MS` (10 ms) and clamps with explicit comparisons. The
   legacy `Duration::clamp(10 ms, max_ms)` call is forbidden (it panics on `min > max`).
2. **Validation before PTT ON.** The bounded duration is computed before the first
   `SetPtt(true)`; an invalid manifest or range yields a typed error and zero PTT ON.
3. **Every controlled post-ON path attempts exactly one PTT OFF**: ON command error, arbiter
   confirmation error, pulse deadline, poll error, transport loss, and cancellation. A
   cancellation or abandoned guard triggers one non-blocking emergency release from `Drop`
   (single `poll` with `Waker::noop()`; no runtime, no blocking I/O, no fabricated `ConfirmedRx`).
4. **Failures are never success.** A failed, timed-out or unconfirmed release returns an error;
   `HilReleaseOutcome` records `Failed`/`Unconfirmed` and the session state stays
   `FailedOrUnknown`.

Test-only protocol doubles live **only** in `tests/support/protocol_double.rs` and are included
explicitly with `#[path = "support/protocol_double.rs"]`. They implement the real `RigBackend`
trait and simulate only documented device behaviour; they never replace the production parser,
frame builder, validation, `TxArbiter`, cleanup, timeout or cancellation logic. There is no
`CatBackendKind` variant, feature flag, GUI/CLI entry, configuration key, environment variable
or automatic fallback that can select them, and `cargo build --release` cannot compile them.

Run the focused gates with:

```
cargo test --test cat_f14_fail_safe
cargo test --test protocol_conformance
cargo test --test security_guard p0_f14
```

Hardware-in-the-loop validation remains **NOT_EXECUTED**: the ten
`#[ignore = "hardware-in-the-loop"]` tests are never run by CI and no real PTT, RF transmission
or transceiver connection is part of the baseline.

## P1-F15 — independent HIL readiness evidence (software-only)

HIL readiness used to be self-certifiable: the safety booleans of the manifest defaulted to
`true`, so the same hand-written document that declared the bench ready was also treated as the
proof. The F-15 remediation separates four layers and makes the production default a denial.

1. **Configuration** — `HilTestBenchManifest` describes the planned test (identity, transport,
   matrix, RF parameters and limits). Every safety flag is required; a missing field is a parse
   error, never `true`. `schema_version` is mandatory for the current format; a document without
   it is legacy v1, requires an explicit migration and stays denied.
2. **Operator attestation** — `HilAttestationDocument` carries the operator's statements. It is a
   declaration, not objective evidence, and it never authorizes hardware execution on its own.
3. **Independent evidence** — `HilIndependentEvidence` comes from a separate document issued by
   an authority outside the test plan. It is bound by SHA-256 to the plan **and** to the
   attestation, and it names the test case, rig, session, attempt, scope and validity window. The
   digest is an integrity hash, **not** a cryptographic signature.
4. **Authorization decision** — `HilHardwareExecutionAuthorizer` evaluates the three inputs and
   returns a closed `HilAuthorizationDecision`. The default is `NotEvaluated`; missing,
   malformed, expired, not-yet-valid, out-of-scope, replayed or provider-failed evidence is a
   controlled denial. `DryRunOnly` never keys PTT.

`HilReadinessValidator::authorize_hardware_execution_at` is the only path that can produce
`HardwareExecutionAuthorized`. The evidence and decision types are deliberately **not**
`Deserialize`, so an untrusted JSON document cannot forge an authorization, and there is no
production evidence provider at all: the test-only provider lives exclusively under
`#[cfg(test)]` in `src/cat/hil/authorization.rs`. No CLI flag, GUI control, configuration key,
environment variable or feature flag can select it.

Focused gates:

```
cargo test --test cat_f15_independent_evidence
cargo test --test cat_f14_fail_safe
cargo test --test security_guard p1_f15
```

`cat_f14_fail_safe` now asserts the fail-closed contract on the production API (a complete plan
and declaration without evidence still produce zero PTT), while the F-14 mechanical invariants
are verified in `src/cat/hil/session.rs` under `#[cfg(test)]`, where test-scope evidence is
available. Hardware behaviour remains **NOT_VERIFIED**.

## CAT-018 / CAT-019 — protocol-conformance suites (software-only)

Two dedicated suites verify the production PTT frames against the official documentation without
any hardware:

- `tests/cat_018_kenwood_conformance.rs` drives the real `KenwoodBackend`/`KenwoodCodec` over the
  test-only in-memory wire double `tests/support/kenwood_wire.rs`. It asserts, **per model**
  (TS-590S, TS-590SG, TS-890S, TS-990S), the exact `TX;`/`RX;` bytes, the documented `TX P1;`
  answer semantics, the model-specific confirmation channel (`IF` only for TS-590S/SG), the
  documented error frames `?;`/`E;`/`O;`, the no-retry rule for PTT ON, the release obligation on
  disconnect, fault injection, a combinatorial matrix and deterministic stress
  (1000 ON + 1000 OFF per profile, 500 errors/timeouts/transport losses per profile).
- `tests/cat_019_flrig_conformance.rs` drives the real `FlrigClient` against the test-only
  loopback XML-RPC daemon `tests/support/flrig_wire.rs`. It asserts the byte-exact
  `rig.set_ptt` payload (`<int>1</int>` / `<int>0</int>`, one method, one parameter, documented
  endpoint), the documented nil response, the rejection of any `<value>` (including the
  `<int>0</int>` "transceiver offline" answer), the strict `<int>` type of `rig.get_ptt`, the
  HTTP-status and malformed/truncated/oversized response handling, no boolean/int fallback, a
  combinatorial matrix and deterministic stress (`1000 × (ON + OFF)`, 1000 faults, HTTP matrix).

Both doubles are compiled only into test binaries, are never referenced from `src/`, and cannot
be selected from the GUI, the CLI, the configuration or the environment. The focused commands are:

```
cargo test --test cat_018_kenwood_conformance
cargo test --test cat_019_flrig_conformance
cargo test --test security_guard cat018
cargo test --test security_guard cat019
```

## P1-F17 — explicitly gated PTT interaction (software-only)

PTT used to be a single-key global toggle (`F7`) that also fired while the operator was typing
in a text field. F-17 removes that interaction entirely and replaces it with two disjoint,
explicit intents behind one central dispatcher.

1. **Two intents, never a toggle.** `PttIntent::RequestOn` (`Ctrl+Shift+F7`) and
   `PttIntent::RequestOff` (`Ctrl+F7`) are separate actions. A bare `F7`, `Shift+F7` and
   `Alt+F7` do nothing at all, and no other single key starts PTT. The transmitting direction
   deliberately needs the harder combination, so stopping transmission is always the easier
   action.
2. **Exact modifier contract.** Both bindings use `ModifierPolicy::ExactRequired`, which
   consumes only `Event::Key { pressed: true, repeat: false, .. }` with
   `Modifiers::matches_exact`. egui's default `matches_logically` ignores extra `Shift`/`Alt`,
   so it is not used for PTT.
3. **Four input gates.** Text focus (`Context::egui_wants_keyboard_input()` **and**
   `Context::text_edit_focused()`), active IME composition, window focus and any open modal
   window each block both intents. The gates are computed once per frame from
   `ShortcutGates`; a gate that cannot be read fails closed
   (`ShortcutGates::fail_closed()`).
4. **Repeat and release are inert.** A `repeat` event never creates a new intent, a release
   event never creates one either, and the matching press is removed from the frame buffer so a
   second read cannot repeat the action.
5. **One dispatcher.** `src/gui/ptt_input.rs` is the only place a GUI intent becomes a transmit
   request. The shortcut, the two VFO-panel buttons and the two palette commands all call
   `SpLogApp::dispatch_ptt`. The GUI contains no `toggle_ptt`, no `RigCommand::SetPtt` and no
   `SetPtt(true)`; the production port calls `CatRuntime::request_tx`/`release_tx`, i.e. the
   existing `TxArbiter` path.
6. **F-15 authorization for ON only.** PTT ON is forwarded only for
   `HilAuthorizationOutcome::HardwareExecutionAuthorized`.
   `splogbook::cat::hil::authorization::evaluate_gui_ptt_authorization()` is the production gate
   for a GUI session and always returns `NotEvaluated` (a GUI session has no plan, attestation
   or independent evidence), so `PTT_ON_COUNT_WITHOUT_AUTHORIZATION = 0`. PTT OFF is **never**
   authorization-gated: a blocked OFF could leave a transmitter keyed.
7. **No optimistic GUI state.** `app.cat.ptt_active` has exactly one writer,
   `update_cat_state_from_telemetry`, and is derived from `TxArbiterState::Owned`. A denial, an
   arbiter error and a port error never show TX; a port error is reported as
   `PttDispatchOutcome::Failed`, never as success.
8. **Release on focus loss.** `PttFocusReleaseGuard` emits exactly one
   `PttIntent::RequestOff` per `focused → unfocused` transition while the arbiter confirms local
   transmission — no retries, no wall-clock timer, no dependence on a key-up event.

Focused gates:

```
cargo test --test p1_f17_ptt_gating
cargo test --lib f17
cargo test --test security_guard p1_f17
```

The dispatcher is exercised through the production entry point `egui::Context::run_ui` (the same
path the application uses) plus a pure decision function, so the whole matrix — gates, arbiter
states, authorization outcomes, port failures, repeat/release — is deterministic and
sleep-free. The recording port double exists only under `#[cfg(test)]` and in `tests/`; it is
never compiled into the release binary and cannot be selected from the GUI, the CLI, the
configuration or the environment. Hardware behaviour remains **NOT_VERIFIED**: no transceiver,
CAT transport, PTT line or RF was used.

## P1-F16 — panic-free logbook pagination (software-only)

The logbook table used to compute `start = page * page_size` in the renderer and to slice the
filtered index list with `sorted_indices[start..end]`. A stale page after a search, an
advanced filter, a journal switch, a deletion, an import or a sync could produce a reversed or
out-of-bounds range, and `page * page_size` could overflow. Pagination is now a pure,
GUI-independent model.

1. **One model.** `src/gui/pagination.rs` exposes
   `PaginationInput { requested_page, page_size, total_items }` and a total
   `paginate()` returning `PaginationWindow { total_items, normalized_page, page_count, start,
   end, effective_page_size, mode, was_clamped, arithmetic_guard_triggered }`. No other place
   computes a page count or a page range.
2. **Normalization.** `LogViewState::pagination_window` clamps the requested page to
   `0..page_count` and writes the normalized value back before anything is sliced. A stale page
   becomes the last existing page; a still-valid page is preserved (a filtering expansion does
   not jump to page 0). Filter contraction, journal switch, deletion, import and sync all pass
   through this method.
3. **Empty result and `page_size = 0`.** An empty result is `page_count = 0`, page 0, range
   `0..0`, and the GUI shows the existing “no results” state. The existing “all rows” mode
   (`page_size = 0`) is a defined single-page window with `effective_page_size >= 1`; it never
   calls `div_ceil(0)` and never divides by zero.
4. **Checked arithmetic.** `checked_mul` and `checked_add` replace the raw multiplication and
   addition. For a normalized page `start < total_items`, so overflow is mathematically
   impossible; a defensive `None` branch produces a safe window with
   `arithmetic_guard_triggered = true` instead of a panic or a wrapped index. `saturating_*` is
   not used to hide an invalid state.
5. **Safe slice access.** `PaginationWindow::slice_of` is `items.get(start..end)` (Rust 1.99.0
   `slice::get` returns `Option` and never panics). `LogViewState::select_page_indices` rebuilds
   an inconsistent cache once and otherwise returns a controlled empty view; the table uses
   `page_indices.get(row.index())` and `recent_qsos.get(idx)`, never direct indexing,
   `get_unchecked` or `catch_unwind`.
6. **Selection.** Selection is keyed by the stable QSO `id` (`INTEGER PRIMARY KEY
   AUTOINCREMENT`), never by a page or filtered index. Deleting a record removes exactly that
   ID; a filtered-out record stays selected by ID and can never be replaced by another QSO.
7. **Coverage.** Pages partition the filtered result: no duplicated rows, no omitted rows, no
   overlapping ranges, and the union of all pages equals the filtered result.

Invariants (software-verified):
`effective_page_size >= 1`; `start <= end`; `end <= total_items`; `start <= total_items`;
`page_count == 0` implies `start == end == 0`; `page_count > 0` implies
`normalized_page < page_count`; in paged mode `start == normalized_page * page_size` and
`end == min(start + page_size, total_items)`; an empty result is `0..0`; the model is pure,
deterministic, time-independent and never mutates user data.

Focused gates:

```
cargo test --test p1_f16_logbook_pagination
cargo test --lib gui::pagination
cargo test --lib gui::state::log_view
cargo test --test security_guard p1_f16
```

The integration suite drives the production `SpLogApp` + `render_logbook_body` through
`egui::Context::run_ui` with an in-memory database and synthetic `TST…` records only. It covers
the empty dataset, one row, one full page, a partial page, an exact multiple, `usize::MAX` page
and page size, filter/sort/journal/deletion/import/sync transitions, selection stability,
fault injection, the coverage proof and deterministic stress (5000/5000 stale-page and empty
series, 3000/3000 deletion and filter series, 200/200 GUI frames, plus a 100 000+ combination
model matrix). No real user data, hardware, PTT or RF is used. F-14, CAT-018, CAT-019, F-15 and
F-17 remain CLOSED; hardware readiness and hardware PTT remain NOT_VERIFIED and NATIVE-7B
remains NOT_STARTED.

## P1-F19 — SOTA/POTA operator–correspondent reference semantics (software-only)

The reference model had no side. The station profile's own summit/park was used to prefill one QSO
entry field per program, a cluster spot of another activator wrote its reference into that same
field, and the save path mapped the field to the **correspondent** ADIF column only. `MY_SOTA_REF`
and `MY_POTA_REF` were never written, and the official POTA `MY_SIG`/`SIG` representation was
neither read nor emitted.

1. **One role-aware model.** `src/core/award_refs.rs` defines `StationSide { Local, Remote }`,
   `AwardProgram { Sota, Pota }`, `AwardRefPair { local, remote }`, `AwardReferences { sota, pota }`
   and `PotaRepresentation { Absent, Sig, DirectFields }`. `local` always means the SPLogbook
   station, `remote` always the correspondent; a missing reference is an explicit `None`; neither
   side is ever derived from the other.
2. **One central mapping.** `AwardReferences::adif_fields()` is the only producer of
   `MY_SOTA_REF`, `SOTA_REF`, `MY_POTA_REF`, `POTA_REF`, `MY_SIG`, `MY_SIG_INFO`, `SIG`, `SIG_INFO`;
   `import_adif_references()` is the only consumer. `src/core/adif.rs` contains no reference field
   name any more and delegates both directions.
3. **SOTA.** own summit → `MY_SOTA_REF`, correspondent summit → `SOTA_REF`, both for S2S, neither
   copied. A summit is never carried in SIG fields.
4. **POTA.** The production contract is the documented POTA pair — `MY_SIG=POTA` + `MY_SIG_INFO`
   for the activator's own park and `SIG=POTA` + `SIG_INFO` for the other activator's park. A
   marker and its `_INFO` partner are always emitted together, and exactly one representation is
   emitted per record. The ADIF 3.1.8 `MY_POTA_REF`/`POTA_REF` list is imported and is used for a
   multi-park value, which the POTA SIG contract cannot express.
5. **Conflicts are explicit.** Consistent representations are merged without loss; contradictory
   ones reject that record with `AdifRecordError::ConflictingPotaRepresentation` naming both
   values. An uninterpretable `SIG` value is reported as an import notice, never guessed.
6. **The form has sides.** `QsoEntryState` carries `my_sota`/`my_pota` (activation context,
   prefilled from the station profile, preserved across the per-QSO reset) and `sota`/`pota`
   (correspondent, filled by a spot or typed).
   `SpLogApp::draft_qso_from_entry` is the single, pure, clock-free form→record mapping.
7. **SOTA CSV V2** uses each record's own `MY_SOTA_REF` for column 3 instead of one dialog value
   for the whole file, and leaves column 3 empty for a chaser.

Focused gates:

```
cargo test --test p1_f19_award_references
cargo test --lib core::award_refs
cargo test --lib core::adif
cargo test --lib core::sota_export
cargo test --lib gui::state::entry
cargo test --test security_guard p1_f19
```

The integration suite drives the production form mapping, the ADIF engine and the SOTA CSV exporter
with synthetic `TST…` callsigns and synthetic references (`AA/BB-001`, `CC/DD-999`, `XX-0001`,
`YY-9999`) only: SOTA activator/chaser/S2S, POTA activator/hunter/P2P, single and dual POTA
representations, conflicts, spot and activation perspective, import/export/DB round trips, fault
injection, a 100 000-case combinatorial matrix and deterministic sleep-free stress. No real user
data, no account, no hardware, no PTT, no RF and no upload to SOTA or POTA is used. F-14, CAT-018,
CAT-019, F-15, F-16 and F-17 remain CLOSED; hardware readiness and hardware PTT remain NOT_VERIFIED
and NATIVE-7B remains NOT_STARTED.
## P1-F20 — journal-scoped, lossless upload selection (software-only)

Every online upload is driven by one pure model, `src/cloud/upload_plan.rs`. To add or change a
service you must:

1. add the target to `UploadTarget` (id, label, batch size, whether it has a sent-status column);
2. select records only through `LogDatabase::select_upload_candidates_for_journal`
   (`src/core/database/upload_selection.rs`) — never with `get_recent_qsos*`, never with `LIMIT`;
3. build an `UploadPlan` (it sorts canonically and validates the batch size) and run it with
   `run_plan`, injecting the transport;
4. write the result through `commit_batch_acceptance` (accepted batches only, row count asserted,
   rollback on mismatch).

Rules that the guard enforces (`security_guard::p1_f20_*`):

- no upload code path may call the global `get_recent_qsos(` selector;
- no upload selection may use `LIMIT`/`OFFSET` or a numeric "recent N" call;
- no service may build an ADIF payload outside the plan orchestrator;
- a test transport must live in `#[cfg(test)]` or `tests/` and must never be reachable from the
  GUI, CLI, config, environment or a release artefact;
- no test may reference a real service endpoint;
- credential-bearing fields never travel in a URL.

Batch sizes are **client transport policy**, not service limits. No documented service record
limit exists for TQSL or for the eQSL real-time interface, so none is encoded; the only hard rule
is that a plan must never lose a record.

When adding an i18n key for upload progress, add it to all seven catalogues at once —
`tests/i18n_guard.rs` requires identical key sets, identical parameter sets and no dead keys.

## Persistent upload checkpoints (F-20R)

The upload path now has a durable state machine. When changing anything in
`src/gui/online_sync_upload.rs`, `src/cloud/upload_checkpoint.rs` or
`src/core/database/upload_checkpoint.rs`, respect these rules:

1. never send a batch whose durable state is `Completed`, `Cancelled`, `Unknown`, `PartiallyAccepted`,
   `Rejected`, `Committing` or `RecoveryRequired`;
2. never treat `InFlight` as `Accepted` after a restart — it becomes `Unknown`;
3. never retry `Unknown` automatically; there is no documented idempotency;
4. never write `{service}_qsl_sent` or `{service}_qslsdate` outside
   `commit_batch_acceptance_with_dates`;
5. never derive the sent date from `QSO_DATE`, the wall clock at commit time, the migration time or
   the recovery time — it comes from the batch's durable `accepted_at`;
6. never persist a credential, a URL or a response body in a checkpoint table.

Schema changes require a new numbered migration, a transaction, a rollback test in
`src/core/database/tests.rs`, and a re-run of `tests/p1_f20r_recovery.rs`.