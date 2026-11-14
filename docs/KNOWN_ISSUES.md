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
| Flatpak packaging (`M3-J`) | not started; requires explicit authorization |
| Hardware-in-the-loop validation (NATIVE-7B) | not executed; 10 `#[ignore]` tests; blocked by F-14 |
| Package signing for DEB/RPM | not implemented; integrity comes from the release manifest and SHA-256 |
| Signed installers for Windows | not implemented; installer artifacts are unsigned test artifacts |

## 4. Environment limitations (not product defects)

- OCI containers share the host kernel, so native Linux verification is userspace and CI
  evidence — not proof of a distribution kernel.
- A low-integrity Windows sandbox cannot write `HKCU` or the Start Menu
  (`0x80070005`), so native Windows installer E2E cannot run there; it is explicit opt-in.
- The owner's build host requires `-j 1`; `-j 2` exhausts the page file (`os error 1455`).
- There is no local Linux or Docker environment for native distribution testing; those
  proofs come from CI.
