# SPLogbook — Known issues and open decisions

Public list of known defects, limitations and undecided questions that affect users,
packagers or maintainers. Findings already remediated in code are not repeated here.

Severity: **P0** — data loss, physical risk or crash on a normal path; **P1** — incorrect
behaviour with user-visible impact; **P2** — limitation with a workaround; **P3** — hygiene.

---

## 1. Open defects

### F-14 (P0) — panic after PTT is engaged

A clamp computation in the HIL transmit path can receive an inverted range and panic while
PTT is keyed. This is the highest-severity open item and a **hard prerequisite** for any
hardware-in-the-loop (NATIVE-7B) validation: transmitting hardware must never be left keyed
because of a host-side panic.

- Impact: physical — keyed transmitter without a software-confirmed release.
- Status: open. HIL tests remain not executed (10 `#[ignore = "hardware-in-the-loop"]`).
- Files: `src/cat/hil/session.rs`, `src/cat/hil/manifest.rs`.

### F-15 (P1) — HIL readiness is self-certifiable

Readiness booleans default to true, so a manifest can declare itself HIL-READY without
independent evidence. Operator attestations (antenna disconnected, 50 Ω dummy load, PA
inhibited) are therefore the only real barrier.

### F-16 (P1) — GUI panic in logbook paging

`src/gui/logbook_table.rs` computes the page start without clamping, so a stale page index
after a filter change can produce an invalid range and panic.

### F-17 (P1) — single-key PTT toggle

PTT is bound to a single key (F7) that also fires while a text field has focus. A stray
keystroke can key the transmitter. A modifier or an explicit confirm step is required.

### F-19 (P1) — SOTA/POTA reference fields inverted

Exported spot/activation records can place the operator reference in the correspondent
fields. Impact: invalid uploads to SOTA/POTA.

### F-20 (P1) — LoTW/eQSL upload selection is global and truncated

The upload path selects the most recent 1000 QSOs without a journal scope, so uploads can
mix journals and silently omit older QSOs. Files: `src/gui/online_sync.rs`.

### F-21 (P1) — Cabrillo export caps at 10 000 QSOs

The exporter truncates at 10 000 QSOs and derives `CLAIMED-SCORE` from the truncated set,
which misrepresents large contest entries. Files: `src/gui/app_export.rs`.

### F-05 (P2) — MSI and Inno Setup installers are not published by CI

Both installers are implemented and locally verified, but CI builds and publishes only the
portable Windows ZIP. Users cannot obtain an installer from a release.

### F-06 (P2) — reference-data downloads are unsigned and non-atomic

Reference database refreshes (callbook, prefixes, satellite elements) are fetched without a
signature and written without a temp-file-plus-rename step, so an interrupted refresh can
leave a partially written file.

### F-07 (P2) — XDG precedence can be shadowed by a portable database

A `databases/` directory next to the binary takes precedence over the XDG data directory,
which can silently shadow user data after an unpack-anywhere install.

### F-22 (P2) — system-managed updates lack a lock, journal and health proof

The Inno/MSI delegation path relies on the package manager for transactionality and does not
create `.update.lock`, an update journal or a health proof of its own.

### F-18 (P2) — redaction can panic on non-ASCII input

`redact_sensitive_info` slices at offsets computed from a lowercased copy, which can land on
a non-char boundary and panic for non-ASCII input. Files: `src/cat/tx_arbiter.rs`.

### F-11 (P3) — hard-coded seed callsigns in the SCP seed

A small number of literal callsigns are injected into the SCP seed at startup. Functional,
but not what a reference database should contain. Files: `src/main.rs`.

### F-12 (P3) — Inno Setup version comparison is lexicographic

`CompareSemVer` in `installer/windows/inno/SPLogbook.iss` compares versions as strings, so
`1.10.0` sorts below `1.3.0`. Affects the downgrade guard for future minor versions ≥ 10.

### F-13 (P3) — AppMutex mixes Inno escaping with a literal GUID

The `AppMutex` value combines `{{` escaping with a literal GUID, producing a mutex name that
does not match a naive expectation. Harmless today; fragile if the GUID is ever templated.

### F-09 (P3) — reference databases are committed as binaries

`databases/callbook.db` and `databases/serviceLOG.db` are tracked binaries grown through
commits. They work as first-install seeds, but they bloat the repository and cannot be
reviewed as text.

## 2. Open decisions

| Decision | State | Effect |
|---|---|---|
| `OWNER-M2-08`, `OWNER-M2-09` | **OPEN** (non-blocking) | `.rollback` retention stays on the temporary conservative policy: never auto-deleted, never contains user data |
| `OWNER-M2-10` | **OPEN** (documentation dispute) | the design text and the implementation disagree about the classification of `serviceLOG.db`; the code treats it as a first-install seed |
| `OWNER-M2-12` | **OPEN** | audit-log retention policy undefined |
| `OWNER-M3-E-MSI-VERSIONING-01` | **OPEN** | blocks MSI for `1.3.0-alpha.2` and later (Windows Installer error 1638) |

Recorded decision identifiers and where each one is implemented are listed in
[`architecture/DECISIONS.md`](architecture/DECISIONS.md).

## 3. Not implemented

| Area | State |
|---|---|
| Flathub submission of the Flatpak | not submitted; the package is a local/CI artifact installed from a bundle |
| Hardware-in-the-loop validation (NATIVE-7B) | not executed; 10 `#[ignore]` tests; blocked by F-14 |
| Package signing for DEB/RPM | not implemented; integrity comes from the release manifest and SHA-256 |
| Package signing for the Flatpak | not applicable to a local bundle; integrity comes from the bundle SHA-256 sidecar and the per-file payload manifest |
| Signed installers for Windows | not implemented; installer artifacts are unsigned test artifacts |

## 3.1 Flatpak sandbox limitations (by design)

`OWNER-M3-J-DEVICE-01` fixed the default sandbox of the Flatpak package at the
minimal set `--share=network`, `--share=ipc`, `--socket=wayland`,
`--socket=fallback-x11`, `--socket=pulseaudio`, `--device=dri`,
`--talk-name=org.freedesktop.secrets`. Consequences:

| Limit | Impact | Workaround |
|---|---|---|
| Native serial CAT (`/dev/ttyUSB*`, `/dev/ttyACM*`, `/dev/ttyS*`) | Icom CI-V, Kenwood, Yaesu and Elecraft serial backends cannot open the port inside the sandbox; the failure is fail-closed with a diagnostic | documented opt-in `flatpak override --user --device=all io.github.sp6ina.SPLogbook`; the override grants **all** devices visible to Flatpak, not only the serial port |
| Direct USB device access (SDR, `libusb`) | not available | run the host-side daemon and connect over TCP |
| Local `rigctld` auto-start | the bundle contains no Hamlib binary | run `rigctld` on the host and connect to `127.0.0.1` over the shared network namespace |
| Selective USB portal (`org.freedesktop.portal.Usb`) | not implemented | recorded as a separate future improvement; it needs new device-access architecture in the serial backends |
| SemVer downgrade of the Flatpak | not exercised: only one Flatpak build exists | a real version downgrade requires a second published build; the deployment-level downgrade path is verified with `flatpak update --commit=<commit>` |

Network CAT backends (`rigctld`, `rotctld`, TCI, FlexRadio, Kenwood LAN,
Elecraft K4 TCP, `flrig`, WinKeyer) and all logging, export, portal and audio
features work in the default sandbox.

## 4. Environment limitations (not product defects)

- OCI containers share the host kernel, so native Linux verification is userspace and CI
  evidence — not proof of a distribution kernel.
- A low-integrity Windows sandbox cannot write `HKCU` or the Start Menu
  (`0x80070005`), so native Windows installer E2E cannot run there; it is explicit opt-in.
- The owner's build host requires `-j 1`; `-j 2` exhausts the page file (`os error 1455`).
- There is no local Linux or Docker environment for native distribution testing; those
  proofs come from CI. The same applies to the Flatpak: `flatpak-builder` and `flatpak run`
  require Linux with bubblewrap, so the build and every sandbox proof come from hosted CI
  runners with a pinned `ubuntu:26.04` container.

## 5. Resolved test-infrastructure issues (history)

Kept so that a recurrence is recognised instead of being rediscovered. These are not
product defects and no user-facing behaviour is affected.

### QA-XDG-01 — parallel process-environment mutation in XDG tests (resolved 2026-11-14)

CI run `43571747469` attempt 1 failed on `tests/linux_xdg_data_preservation_test.rs:47`
(`test_xdg_base_directory_resolution_enforces_absolute_paths`):

```
assertion `left == right` failed
  left: "/tmp/.tmpTUCjtX/home_user/.config/splogbook"
 right: "/tmp/.tmpTUCjtX/custom_config/splogbook"
```

Three tests in the same test binary mutated process-global `HOME`/`XDG_*` without mutual
exclusion, and `XdgPaths::resolve()` reads `HOME` first and `XDG_CONFIG_HOME` later, so a
concurrent test could replace the values between those reads. Attempt 2 of the same run
passed with no code change, which confirmed a race rather than a logic error.

Resolution: every `HOME`/`XDG_*` mutation in tests goes through one shared RAII guard
(`tests/support/process_env.rs`) — a single per-process mutex, snapshot/restore that
distinguishes an absent variable from an empty one, restoration during unwinding, poisoned
mutex recovery, and explicit rejection of nested guards. The two env-mutating unit tests
moved from `src/core/xdg.rs` to the integration suite, so the library test binary no longer
mutates the environment. The HIL test binary keeps its own lock for the disjoint
`SPLOG_HIL_*` namespace and no longer writes an unsynchronised reference-time seed (the
deterministic time is passed explicitly). `security_guard` enforces all of these rules.
Measured evidence: `security_guard` 127/127, `linux_xdg_data_preservation_test` 7/7,
`process_environment_isolation_test` 12/12 and 670 deterministic stress iterations with
0 failures — see [`VERIFICATION_BASELINE.md`](VERIFICATION_BASELINE.md) §6. No retry,
`#[ignore]`, test filtering or global `--test-threads=1` was used.

### QA-RANDOM-01 — random temp-directory name could trip the `iex` denial assertion (resolved 2026-11-14)

`m3_b_stability_20_iterations_helper_parity_and_e2e_cycle` generates a PowerShell fallback
script from a request whose paths point into a `tempfile::tempdir()`. The assertion that the
script contains no `iex` alias was applied to the whole rendered script, so a random temp
directory name that happened to contain the letters `iex` made the test fail with no product
defect. The assertion now strips the random installation-root prefix before checking the
forbidden tokens; every security assertion is unchanged. Found during the final verification
of the environment-determinism gate.
