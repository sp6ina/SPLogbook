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

- `CAT-018` (Kenwood PTT answer semantics): **CLOSED** (2026-11-15,
  `STAGE-CAT-018-019-SOFTWARE`, `COMPLETED_KENWOOD_PTT_PROTOCOL_CONFORMANCE_VERIFIED`). The
  official PC CONTROL COMMAND references define `TX P1;` with `P1=0` as `SEND`, so every
  documented `P1` means transmit; the parser, the model-specific confirmation channel (`IF` only
  for TS-590S/SG), the documented `?;`/`E;`/`O;` errors and the release obligation now match the
  documents per model. Software-only verification; hardware behaviour = NOT_VERIFIED.
- `CAT-019` (FLRig XML-RPC PTT type): **CLOSED** (2026-11-15, same gate,
  `COMPLETED_FLRIG_XMLRPC_PTT_CONFORMANCE_VERIFIED`). FLRig 2.0.00 documents `rig.set_ptt` as
  `n:i`; the client sends the documented `<int>`, accepts only the documented nil response,
  enforces the `<int>` type for `rig.get_ptt`, rejects non-200 HTTP statuses and has no
  boolean/int fallback. Software-only verification; real FLRig/hardware behaviour = NOT_VERIFIED.
- `F-14` (P0, panic after PTT is engaged): **F-14 software remediation = CLOSED** (2026-11-15,
  `STAGE-P0-F14-SOFTWARE`). The inverted-range panic was removed with a total, typed range
  validation executed before PTT ON, a manifest minimum (`HIL_MIN_TEST_DURATION_MS = 10`), and
  an explicit release attempt on every controlled post-ON path (including cancellation via a
  non-blocking `Drop` release). Verified software-only: protocol conformance software
  verification = COMPLETED, hardware validation = NOT_EXECUTED. It must not be reopened without
  a reproducible regression.
- `F-15` (P1, HIL readiness was self-certifiable): **F-15 software remediation = CLOSED**
  (2026-11-15, `STAGE-P1-F15-SOFTWARE`,
  `COMPLETED_HIL_READINESS_INDEPENDENT_EVIDENCE_REMEDIATION_VERIFIED`). No safety field defaults
  to `true` any more; the test plan, the operator declaration, the independent evidence and the
  authorization decision are separate layers, and hardware execution requires an evidence
  document bound by SHA-256 to the plan and the declaration, scoped to the test, rig, session,
  attempt and validity window. Missing/malformed/expired/out-of-scope/replayed evidence is a
  controlled denial, a dry run never keys PTT, and no production evidence provider exists
  (test-only provider confined to `#[cfg(test)]`). Production default = DENIED; hardware
  readiness = NOT_VERIFIED; NATIVE-7B remains NOT_STARTED. It must not be reopened without a
  reproducible regression.
- `F-17` (P1, PTT was a single-key toggle): **F-17 software remediation = CLOSED**
  (2026-11-16, `STAGE-P1-F17-SOFTWARE`,
  `COMPLETED_SAFE_PTT_USER_INTERACTION_REMEDIATION_VERIFIED`). A bare `F7` no longer keys the
  transmitter: PTT is split into two explicit intents (`Ctrl+Shift+F7` = TX ON, `Ctrl+F7` =
  TX OFF) with exact modifiers, and both are blocked during text editing, during IME
  composition, without window focus and while any modal window is open. Key repeat and release
  events never produce an intent, the shortcut is consumed once, and every user-interaction
  surface goes through one central dispatcher. PTT ON requires the F-15 authorization outcome;
  a denial, an arbiter error and a port error never show TX, and PTT OFF never requires
  authorization. Software-only verification: hardware interaction = **NOT_VERIFIED**; NATIVE-7B
  remains NOT_STARTED. It must not be reopened without a reproducible regression.
- `F-16` (P1, GUI panic in logbook paging): **F-16 software remediation = CLOSED**
  (2026-11-16, `STAGE-P1-F16-SOFTWARE`,
  `COMPLETED_LOGBOOK_PAGINATION_PANIC_REMEDIATION_VERIFIED`). Page count, page normalization
  and the `start..end` window now come from one pure model (`src/gui/pagination.rs`); a stale
  page after a filter, journal switch, deletion, import or sync is normalized before slicing,
  the empty result and `page_size = 0` are explicit safe states, arithmetic is checked, and
  access goes through `slice::get`. Selection stays keyed by the stable QSO `id`. Verified
  software-only with synthetic data: `REAL_USER_DATA_USED=false`, hardware behaviour remains
  NOT_VERIFIED and NATIVE-7B remains NOT_STARTED. It must not be reopened without a
  reproducible regression.
- `F-19` (P1, SOTA/POTA operator–correspondent reference fields were inverted): **F-19 = CLOSED**
  (2026-11-16, `STAGE-P1-F19-SOFTWARE`,
  `COMPLETED_SOTA_POTA_REFERENCE_SEMANTICS_REMEDIATION_VERIFIED`). The ambiguous single reference
  field per program is gone: the entry state now has an explicit local pair (`my_sota`, `my_pota`,
  the activation context, prefilled from the station profile) and an explicit correspondent pair
  (`sota`, `pota`, filled by a spot or by typing), and `SpLogApp::draft_qso_from_entry` maps each
  pair to its own column. One central module (`src/core/award_refs.rs`) is the only place that turns
  references into ADIF fields: `MY_SOTA_REF`/`SOTA_REF` for SOTA and the POTA-documented
  `MY_SIG`/`MY_SIG_INFO` + `SIG`/`SIG_INFO` pair for POTA, with the ADIF 3.1.8
  `MY_POTA_REF`/`POTA_REF` list used only when a side carries multiple parks. Import merges
  consistent representations, rejects contradictory ones explicitly and preserves the value; S2S
  and P2P keep both sides. `SOTA local/remote mapping = VERIFIED`,
  `POTA local/remote mapping = VERIFIED`, `S2S mapping = VERIFIED`, `P2P mapping = VERIFIED`,
  `ADIF round-trip = SOFTWARE_VERIFIED`. No schema migration, no historical record rewritten,
  `Real user data = NOT_USED`, `External SOTA/POTA upload = NOT_EXECUTED`, hardware remains
  NOT_VERIFIED and NATIVE-7B remains NOT_STARTED. It must not be reopened without a reproducible
  regression.
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

- `F-20` (LoTW/eQSL upload selection was global and truncated): **CLOSED** — the upload scope is a
  journal-scoped (`JOURNAL_SCOPED`) materialised plan; silent truncation is removed and batching
  is lossless (`LOSSLESS`) and deterministic; partial, timeout and cancelled outcomes are modelled and never counted as
  success; status is written only after batch-level service evidence. Software-only verification:
  no real LoTW or eQSL upload, no real credentials and no real user data were used, so real
  submission remains NOT_EXECUTED and production credentials remain unverified.
- `F-21` (P1, Cabrillo export and contest scoring capped at 10 000 QSOs): **F-21 = CLOSED**
  (2026-11-18, `STAGE-P1-F21-SOFTWARE`,
  `COMPLETED_CABRILLO_UNTRUNCATED_EXPORT_AND_SCORE_REMEDIATION_VERIFIED`). `export_cabrillo`,
  `export_pdf_log`, `export_gpx_log` and `ContestCache` query the full active journal via
  `get_all_qsos_for_journal`, `cabrillo::build` and `compute_scoreboard` share the canonical
  ascending key `canonical_qso_sort_key`, and wall-clock timestamp fabrication is removed
  (fail-closed `ContestError::InvalidQsoTimestamp`). Software-only verification; NATIVE-7B remains
  NOT_STARTED.
- `F-18` (P2, redaction panic on non-ASCII UTF-8 input): **F-18 = CLOSED** (2026-11-18).
  `redact_sensitive_info` in `src/cat/tx_arbiter.rs` matches ASCII secret markers directly on byte
  windows with `eq_ignore_ascii_case` (without `.to_lowercase()`) and aligns slice bounds with
  `ceil_char_boundary` / `floor_char_boundary`.
- `R1C`, `R1D`, `F-07`, `F-09`, `F-11` (`STAGE-R1C-R1D-LEGACY-DATA-REMOVAL-AND-CLEAN-FIRST-RUN`):
  **F-07 = CLOSED**, **F-09 = CLOSED**, **F-11 = CLOSED** (2026-11-18). Removed legacy binary/CSV
  reference databases (`databases/serviceLOG.db`, `databases/callbook.db`, `databases/clubs.csv`),
  `src/core/service_db.rs`, SQLite `v1.2.0` legacy lookup compatibility, hardcoded `scp.insert(`
  seed callsigns, `UpdateFileClass::FirstInstallSeed`, installer/CI bundling of seed databases, and
  portable `exe_dir`/`CWD` shadowing of XDG/`%APPDATA%` paths. All reference data is now supplied via
  `UserTables`, Big CTY (`cty.dat`/`cty.csv`), and user-installed files in the canonical data
  directory, with built-in IOTA samples (`fallback_iota`) and `US_STATES` retained in
  `src/core/reference/tables.rs`.

## 5. Not started / not delivered

| Item | Status | Note |
|---|---|---|
| Flathub submission of the Flatpak | **NOT_STARTED** | deliberately out of scope; the package is a local/CI artifact only and is installed from a bundle |
| Publishing of Windows Inno Setup and MSI installers | **OPEN** (finding `F-05`) | implemented and locally verified; CI builds and publishes only the portable ZIP |
| MSI prerelease versioning for `1.3.0-alpha.2`+ | **OPEN** (decision `OWNER-M3-E-MSI-VERSIONING-01`) | Windows Installer rejects the prerelease version mapping with error 1638 |
| Hardware-in-the-loop validation (NATIVE-7B) | **NOT_STARTED** | 10 tests remain `#[ignore = "hardware-in-the-loop"]`; the software blockers `F-14` and `F-15` are closed, but hardware validation is still NOT_EXECUTED and NATIVE-7B = NOT_STARTED requires a separate owner authorisation and a future approved independent evidence provider |
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

## 8. F-20 residual hardening — `STAGE-P1-F20R-SOFTWARE` (2026-11-17 / CLOSED 2026-11-18)

| Item | Status |
|---|---|
| F20R gate (`STAGE-P1-F20R-SOFTWARE`) | **F20R = CLOSED** — unblocked by `STAGE-RUSTSEC-2026-0543-CHRONO-REPLACEMENT` (`jiff 1.0.1`); verified on `8169c97`: `CI` run `44348963039` = `success`, `Build and Release` run `44348972587` = `success` (23/23 jobs green, `Publish Release Assets` = `skipped`) |
| F-20 | remains **CLOSED** — nothing F-20 closed was reopened; all 29 F-20 tests and all 12 F-20 guards pass unmodified |
| F20-R1 persistent upload checkpoints | **CLOSED** — `IMPLEMENTED_AND_SOFTWARE_VERIFIED` |
| F20-R2 `LOTW_QSLSDATE` / `EQSL_QSLSDATE` | **CLOSED** — `IMPLEMENTED` |
| F20-R3 download credential transport | **CONFINED** — `OFFICIAL_ENDPOINT_REQUIRES_QUERY_CREDENTIALS=true`; the official contract offers no safer mechanism, so the limitation is documented instead of being falsely closed |
| F20-R4 invented received date on the LoTW / eQSL download path | **F20-R4 = CLOSED** (2026-11-18) — LoTW received date comes only from `LOTW_QSLRDATE`, `APP_LoTW_RXQSL` or `QSLRDATE` (`QSL_RCVD=Y`), and eQSL received date comes only from `EQSL_QSLRDATE` or `QSLRDATE` (`EQSL_QSL_RCVD=Y` / `QSL_RCVD=Y`), otherwise `None` with `COALESCE` protection; see `KNOWN_ISSUES.md` §6 |
| `RUSTSEC-2026-0543` (`chrono` unmaintained) | **CLOSED** (2026-11-18) — resolved by full migration to `jiff 1.0.1` without adding an advisory ignore |
| F-21 | **F-21 = CLOSED** (2026-11-18, `STAGE-P1-F21-SOFTWARE`) |
| F-18 | **F-18 = CLOSED** (2026-11-18, UTF-8 byte-window redaction without `.to_lowercase()`) |
| NATIVE-7B | remains **NOT_STARTED** |

Storage: schema version **9** (`upload_plans`, `upload_batches`, `upload_batch_members`,
`qso_records.lotw_qslsdate`, `qso_records.eqsl_qslsdate`). Migration is additive and reversible in
test scope; no historical row is rewritten.