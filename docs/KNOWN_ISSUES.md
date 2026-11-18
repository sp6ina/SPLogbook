# SPLogbook — Known issues and open decisions

Public list of known defects, limitations and undecided questions that affect users,
packagers or maintainers. Findings already remediated in code are not repeated here.

Severity: **P0** — data loss, physical risk or crash on a normal path; **P1** — incorrect
behaviour with user-visible impact; **P2** — limitation with a workaround; **P3** — hygiene.

---

## 1. Open defects

### F-14 (P0) — panic after PTT is engaged — F-14 software remediation = CLOSED

A `Duration::clamp` computation in the HIL transmit path could receive an inverted range and
panic while PTT was keyed. The manifest accepted `max_test_duration_ms` in `1..=500`, while the
transmit path clamped against a hard-coded lower bound of 10 ms: a value in `1..=9` made
`min > max`, and `Ord::clamp` panics on that precondition. In `acquire_tx_guard_at` the clamp ran
**after** `SetPtt(true)` but **before** the `HilTxGuard` existed, so the panic left the
transmitter keyed with no release path; `panic = "abort"` in the release profile removed even the
unwinding `Drop`.

Software remediation (2026-11-15, `STAGE-P0-F14-SOFTWARE`, software-only verification):

- the range is now total and validated **before** PTT ON (`bounded_test_duration` returns
  `Result`; reversed/empty ranges are typed `SafetyViolation`, never a panic);
- the manifest rejects a maximum below the documented minimum (`HIL_MIN_TEST_DURATION_MS = 10`)
  and non-finite power/SWR limits;
- every controlled post-ON path attempts exactly one PTT OFF (ON error, arbiter-confirmation
  error, deadline, poll error, disconnect, cancellation), and the emergency `Drop` release is
  non-blocking and never fabricates `ConfirmedRx`;
- a failed, timed-out or unconfirmed release is reported as an error, never as success;
- the old panic path is guarded by `security_guard` plus the `cat_f14_fail_safe` suite
  (range matrix, fault injection, combinatorial invariants, deterministic stress).

Verification boundaries recorded by this gate:

- F-14 software remediation = CLOSED
- Protocol conformance software verification = COMPLETED
- Hardware validation = NOT_EXECUTED
- NATIVE-7B = NOT_STARTED

The software fix does **not** prove any physical behaviour: no transceiver, serial port, dummy
load, real PTT or RF transmission was used. The 10 hardware tests remain
`#[ignore = "hardware-in-the-loop"]`.

Files: `src/cat/hil/session.rs`, `src/cat/hil/manifest.rs`.

### CAT-018 / CAT-019 — protocol conformance (software remediation 2026-11-15)

Both discrepancies recorded during the F-14 closure were resolved by the dedicated gate
`STAGE-CAT-018-019-SOFTWARE`, against the **official model-specific** documents. Software-only
verification: no transceiver, serial port, real FLRig, PTT or RF was used.

- **CAT-018 — CLOSED, `COMPLETED_KENWOOD_PTT_PROTOCOL_CONFORMANCE_VERIFIED`.** The official
  PC CONTROL COMMAND references (`B5A-0316-20/01` TS-590S/SG, `B5A-2944-00/01` TS-890S,
  `B5A-3170-00` TS-990S) define the Answer `TX P1;` where `P1=0` is `SEND`, `P1=1` is
  `DATA SEND` and `P1=2` is `TX Tune` — **every** documented `P1` means the transceiver is
  transmitting. The production parser now matches that documented Answer model (`TX0;`/`TX1;`/
  `TX2;` ⇒ TX, `RX;` ⇒ RX, anything else — including a bare `TX;` — yields no state), the
  mislabelled "PTT read" helper that returned the transmit command `TX;` is gone, and the
  documented error frames `?;` / `E;` / `O;` fail the operation closed. The confirmation channel
  is **model-specific**: `IF` exists only in `B5A-0316-20/01` (TS-590S/SG); the TS-890S/TS-990S
  use only the documented Auto-Information Answer and no longer receive an undocumented `IF;`.
  The documented `ID;` command replaced `IF;` as the probe fallback. Write success is never a
  confirmed TX, and an accepted `TX;` keeps a release obligation for `disconnect()`.
- **CAT-019 — CLOSED, `COMPLETED_FLRIG_XMLRPC_PTT_CONFORMANCE_VERIFIED`.** FLRig **2.0.00**
  (`xml_server.cxx`, tag `v2.0.00`) documents `rig.set_ptt` as `n:i` — one **integer**
  parameter, nil return — and implements it as `PTT = int(params[0])`. The client now sends
  `<int>1</int>` / `<int>0</int>`, accepts the documented nil response, rejects any `<value>`
  (including the `<int>0</int>` "transceiver offline" answer), enforces the documented `<int>`
  type for `rig.get_ptt` and rejects non-200 HTTP statuses. No boolean/int fallback exists, and a
  PTT request is never retried.

`rig.set_verify_ptt` (`n:i`) and `rig.set_ptt_fast` (`n:i`, *"deprecated; use set_ptt"*) remain
documented in 2.0.00 but unused by SPLogbook; neither is required by the PTT path.

Status block:

```
CAT-018 = CLOSED / COMPLETED_KENWOOD_PTT_PROTOCOL_CONFORMANCE_VERIFIED
CAT-019 = CLOSED / COMPLETED_FLRIG_XMLRPC_PTT_CONFORMANCE_VERIFIED
SOFTWARE_ONLY_VERIFICATION=true
PROTOCOL_CONFORMANCE_SOFTWARE_VERIFIED=true
HARDWARE_BEHAVIOUR_NOT_VERIFIED=true
REAL_TRANSCEIVER_CONNECTED=false
REAL_PTT_EXECUTED=false
RF_TRANSMISSION_EXECUTED=false
HIL_EXECUTED=false
```

Tests: `cat_018_kenwood_conformance`, `cat_019_flrig_conformance`, the per-model rows in
`protocol_conformance`, and six `cat018_*` / `cat019_*` rules in `security_guard`. The synthetic
Kenwood wire (`tests/support/kenwood_wire.rs`) and the synthetic FLRig XML-RPC daemon
(`tests/support/flrig_wire.rs`) exist only in the test scope and are not an application feature.


### F-15 (P1) — HIL readiness is self-certifiable — F-15 software remediation = CLOSED

Readiness booleans default to true, so a manifest can declare itself HIL-READY without
independent evidence. Operator attestations (antenna disconnected, 50 Ω dummy load, PA
inhibited) are therefore the only real barrier.

Software remediation (2026-11-15, `STAGE-P1-F15-SOFTWARE`, software-only verification):

- the self-certifiable model is gone: every safety field is **required** and must be written
  explicitly, so a missing field is a parse error, never `true`;
- the test plan (`HilTestBenchManifest`), the operator declaration
  (`HilAttestationDocument`), the independent evidence (`HilIndependentEvidence`) and the
  authorization decision (`HilAuthorizationDecision`) are four separate layers;
- hardware execution is authorized only by `HilHardwareExecutionAuthorizer`, which requires an
  independent evidence document bound by SHA-256 to the plan **and** to the operator
  declaration, scoped to the test, rig, session, attempt and validity window, and covering every
  required measured fact;
- missing, malformed, expired, not-yet-valid, out-of-scope, replayed or provider-failed evidence
  is a controlled denial; a dry run never authorizes PTT;
- the evidence and decision types are deliberately **not** deserialisable, so an untrusted JSON
  document cannot forge an authorization;
- **no production evidence provider exists**: the test-only provider lives exclusively under
  `#[cfg(test)]`, and there is no CLI flag, GUI control, configuration key, environment variable
  or feature flag that can select it;
- the ten hardware tests remain `#[ignore = "hardware-in-the-loop"]`.

Verification boundaries recorded by this gate:

- F-15 software remediation = CLOSED
- HIL readiness self-certification = REMOVED
- Production default = DENIED
- Hardware readiness = NOT_VERIFIED
- SOFTWARE_ONLY_VERIFICATION
- F-14 software remediation = CLOSED
- Hardware validation = NOT_EXECUTED
- Protocol conformance software verification = COMPLETED
- F-17 (P1) = still open at this measurement (closed later by `STAGE-P1-F17-SOFTWARE`, see §1 F-17)
- NATIVE-7B = NOT_STARTED

SPLogbook cannot confirm the physical preparation of a test bench from a manifest or an operator
declaration alone. Without hardware in the current environment, HIL stays unavailable:
`HARDWARE_CONNECTED=false`, `REAL_TRANSCEIVER_CONNECTED=false`, `REAL_PTT_EXECUTED=false`,
`RF_TRANSMISSION_EXECUTED=false`, `HIL_EXECUTED=false`. The operator declaration describes what
the operator reports; it is never objective hardware evidence.

Files: `src/cat/hil/authorization.rs`, `src/cat/hil/manifest.rs`, `src/cat/hil/session.rs`,
`src/cat/hil/validator.rs`, `tests/cat_f15_independent_evidence.rs`,
`tests/cat_f14_fail_safe.rs`, `tests/security_guard.rs`.

### F-16 (P1) — GUI panic in logbook paging — F-16 software remediation = CLOSED

`src/gui/logbook_table.rs` computed the page start as `page * page_size` without clamping,
and the renderer sliced `cache.sorted_indices[page_start..page_end]` directly. The page index
was normalized only against the **unfiltered** row buffer and only when that buffer was
non-empty, while the renderer used the **filtered** index length. A stale page after a search,
an advanced filter, a journal switch, a deletion, an import or a sync could therefore produce
`start > end`, `start > len` or a multiplication overflow. The reproduced failures were
`range start index 100 out of range for slice of length 1`, `range start index 50 out of range
for slice of length 0` and `attempt to multiply with overflow` at
`src/gui/logbook_table.rs:442` / `:724`.

Software remediation (2026-11-16, `STAGE-P1-F16-SOFTWARE`, software-only verification):

- one pure, GUI-independent model (`src/gui/pagination.rs`) is the only place that computes
  the page count, the normalized page and the `start..end` window:
  `PaginationInput { requested_page, page_size, total_items }` →
  `PaginationWindow { normalized_page, page_count, start, end, effective_page_size, mode,
  was_clamped, arithmetic_guard_triggered }`;
- `LogViewState::pagination_window` normalizes the page before every slice and writes the
  normalized value back; `LogViewState::select_page_indices` obtains the page through
  `slice::get` and rebuilds an inconsistent cache once instead of panicking;
- the empty result has an explicit state (`page_count = 0`, page 0, range `0..0`); the existing
  `page_size = 0` mode (“all rows”) is a defined single-page window with `effective_page_size
  >= 1`, never a division by zero;
- multiplication and addition use `checked_mul` / `checked_add`; overflow is impossible for a
  normalized page (proof in the model report) and the defensive branch is signalled instead of
  wrapping, panicking or silently truncating;
- slicing goes through `slice::get`; the GUI no longer indexes `sorted_indices` or
  `recent_qsos` directly, and there is no `get_unchecked`, no `catch_unwind` and no `unwrap` /
  `expect` on a page range;
- selection remains keyed by the stable `id` (`INTEGER PRIMARY KEY AUTOINCREMENT`, IDs are not
  reused); deleting a record removes exactly that ID and never selects another QSO;
- filter contraction normalizes to the last existing page, filter expansion keeps a still-valid
  page, sorting keeps the page, page-size changes and empty journals are range-safe.

Verification boundaries recorded by this gate:

```
F-16 = CLOSED
COMPLETED_LOGBOOK_PAGINATION_PANIC_REMEDIATION_VERIFIED
Logbook pagination = PANIC_FREE_SOFTWARE_VERIFIED
Stale page normalization = VERIFIED
Filter transitions = VERIFIED
Selection stability = VERIFIED
Empty result = DEFINED
Page size zero = ALL_ROWS_SAFE
Checked arithmetic = VERIFIED
Safe slice access = VERIFIED
Real user data = NOT_USED
SOFTWARE_ONLY_VERIFICATION=true
```

Files: `src/gui/pagination.rs`, `src/gui/logbook_table.rs`,
`src/gui/state/log_view.rs`, `src/gui/app_qso.rs`, `src/gui/app_layout.rs`,
`tests/p1_f16_logbook_pagination.rs`, `tests/security_guard.rs`.

The verification is software-only and uses synthetic records only. It does **not** prove any
real user database, corrupted database or GUI session on real data: `REAL_USER_DATA_USED=false`,
no real log, callsign, journal or production profile was read. F-14, CAT-018, CAT-019, F-15 and
F-17 remain CLOSED, hardware readiness and hardware PTT remain NOT_VERIFIED, the ten
hardware-in-the-loop tests remain `#[ignore = "hardware-in-the-loop"]` and NATIVE-7B remains
NOT_STARTED. This defect must not be reopened without a reproducible regression.

### F-17 (P1) — single-key PTT toggle — F-17 software remediation = CLOSED

PTT was bound to a single key (F7) with `ConflictPolicy::AllowInTextEditing`, so a stray
keystroke — including one typed into a text field, a modal dialog, an active IME session or an
unfocused window — could key the transmitter, and the same event toggled TX/RX in either
direction.

Software remediation (2026-11-16, `STAGE-P1-F17-SOFTWARE`, software-only verification):

- the single-key toggle is gone: `F7` alone, `Shift+F7` and `Alt+F7` do nothing, and no other
  single key starts PTT;
- PTT is split into two disjoint, explicit intents: `Ctrl+Shift+F7` requests TX ON and `Ctrl+F7`
  requests TX OFF — the harder combination is the transmitting direction, so stopping is always
  the easier action;
- both bindings require **exact** modifiers (`Modifiers::matches_exact`), so an extra `Shift`,
  `Alt` or `Super` is an explicit mismatch, never a silent trigger;
- both bindings are blocked by text focus (`Context::egui_wants_keyboard_input()` **and**
  `Context::text_edit_focused()`), by an active IME composition, by a window without focus and
  by any open modal window; gates that cannot be read fail closed;
- key repeat never retriggers and a release event never produces an intent; the shortcut is
  consumed exactly once per frame;
- all three user-interaction surfaces (shortcut, VFO-panel buttons, command palette) go through
  one central dispatcher (`src/gui/ptt_input.rs`), and the GUI no longer contains a toggle entry
  point;
- PTT ON requires the F-15 authorization outcome `HardwareExecutionAuthorized`; a GUI session has
  no plan, attestation or independent evidence, so its production gate always returns
  `NotEvaluated` and refuses — `PTT_ON_COUNT_WITHOUT_AUTHORIZATION = 0`;
- PTT OFF never requires authorization (a blocked OFF could leave a transmitter keyed), a port
  error is reported as a failure, and neither a denial nor an arbiter error ever shows TX in the
  GUI;
- losing window focus while transmitting performs exactly one controlled OFF attempt through the
  existing TxArbiter path (never a raw `SetPtt`), without retries and without a wall-clock timer.

Verification boundaries recorded by this gate:

- F-17 software remediation = CLOSED
- Single-key PTT = REMOVED
- Text editing PTT = BLOCKED
- IME PTT = BLOCKED
- Window-unfocused PTT = BLOCKED
- Repeated-key PTT = BLOCKED
- PTT without F-15 authorization = BLOCKED
- Hardware interaction = NOT_VERIFIED
- NATIVE-7B = NOT_STARTED

The software fix does **not** prove any physical behaviour: no transceiver, serial port, dummy
load, real PTT or RF transmission was used. `HARDWARE_CONNECTED=false`,
`REAL_TRANSCEIVER_CONNECTED=false`, `REAL_PTT_EXECUTED=false`,
`RF_TRANSMISSION_EXECUTED=false`, `HIL_EXECUTED=false`. The ten hardware tests remain
`#[ignore = "hardware-in-the-loop"]`.

Files: `src/gui/ptt_input.rs`, `src/gui/shortcuts.rs`, `src/gui/app_input.rs`,
`src/gui/app_cat.rs`, `src/gui/vfo_panel.rs`, `src/gui/command_palette.rs`,
`src/cat/hil/authorization.rs`, `tests/p1_f17_ptt_gating.rs`, `tests/security_guard.rs`.

This defect must not be reopened without a reproducible regression.

### F-19 (P1) — SOTA/POTA reference fields inverted — F-19 = CLOSED

Exported spot/activation records could place the operator's own reference in the correspondent
field and the correspondent's reference in the local field. Impact: invalid uploads to SOTA/POTA,
wrong activator/hunter attribution and lost references on round-trip.

Root cause: the reference model had no notion of a **side**. The station profile's own reference
was used to prefill the QSO entry field, a cluster spot of another activator wrote its reference
into the same single field per program, and the persistence step mapped that one field to the
**correspondent** column only (`src/gui/state/entry.rs`, `src/gui/app_config.rs`,
`src/gui/app_qso.rs`). `MY_SOTA_REF`/`MY_POTA_REF` were therefore never written by any UI path, and
the official POTA `MY_SIG`/`SIG` representation was neither read nor emitted
(`src/core/adif.rs`). `MY_*` had effectively been read as "my contact" instead of "my station".

Software remediation (2026-11-16, `STAGE-P1-F19-SOFTWARE`, software-only verification):

- the entry state has an explicit local pair (`my_sota`, `my_pota` — the activation context,
  prefilled from the station profile and surviving the per-QSO reset) and an explicit correspondent
  pair (`sota`, `pota` — filled by a spot or typed); `SpLogApp::draft_qso_from_entry` is the single
  pure mapping from the form to the record and writes each pair to its own column;
- one central module `src/core/award_refs.rs` (`StationSide`, `AwardProgram`, `AwardRefPair`,
  `AwardReferences`, `PotaRepresentation`) is the only place that turns references into ADIF
  fields; the ADIF engine delegates both import and export to it;
- SOTA: `MY_SOTA_REF` ← own summit, `SOTA_REF` ← correspondent summit; activator, chaser and
  summit-to-summit all keep their perspective, and a missing side stays missing;
- POTA production contract: `MY_SIG=POTA` + `MY_SIG_INFO` for the activator's own park and
  `SIG=POTA` + `SIG_INFO` for the other activator's park, per the official POTA documentation;
  `MY_SIG` never appears without `MY_SIG_INFO` and `SIG` never without `SIG_INFO`;
- the ADIF 3.1.8 `MY_POTA_REF`/`POTA_REF` (`POTARefList`) fields are imported and are used for
  multi-park values, which the POTA SIG contract cannot express; exactly **one** POTA representation
  is emitted per record, so two contradictory representations can never be produced;
- consistent POTA representations are merged without loss; contradictory ones are a controlled,
  explicit per-record rejection (`AdifRecordError::ConflictingPotaRepresentation`) that names both
  values — never an arbitrary choice and never a side swap; uninterpretable SIG data is reported as
  an import notice instead of being silently dropped;
- SOTA CSV V2 uses each record's own `MY_SOTA_REF` for column 3 instead of one dialog value for the
  whole file, and leaves column 3 empty for a chaser;
- no database migration and no historical record rewrite: each role already had its own column.

Verification boundaries recorded by this gate:

```
F-19 = CLOSED
COMPLETED_SOTA_POTA_REFERENCE_SEMANTICS_REMEDIATION_VERIFIED
SOTA local/remote mapping = VERIFIED
POTA local/remote mapping = VERIFIED
S2S mapping = VERIFIED
P2P mapping = VERIFIED
Import role preservation = VERIFIED
Export role preservation = VERIFIED
ADIF round-trip = SOFTWARE_VERIFIED
Conflicting POTA representations = REJECTED
External SOTA/POTA upload = NOT_EXECUTED
Real user data = NOT_USED
SOFTWARE_ONLY_VERIFICATION=true
F-14 / CAT-018 / CAT-019 / F-15 / F-16 / F-17 = remain CLOSED
NATIVE-7B = NOT_STARTED
```

The verification is software-only and uses synthetic records only. It does **not** claim that any
public system accepted a file: no upload to SOTA or POTA was performed, no account was used and no
real log, callsign or database was read. It also does not claim that every historical user record is
correct — rows written before this gate keep the values they were stored with, and no automatic
repair or irreversible migration was performed. Hardware readiness and hardware PTT remain
NOT_VERIFIED, the ten hardware-in-the-loop tests remain `#[ignore = "hardware-in-the-loop"]` and
NATIVE-7B remains NOT_STARTED.

Files: `src/core/award_refs.rs`, `src/core/adif.rs`, `src/core/qso.rs`, `src/core/sota_export.rs`,
`src/gui/state/entry.rs`, `src/gui/app_qso.rs`, `src/gui/app_config.rs`, `src/gui/qso_entry.rs`,
`src/gui/cluster_panel.rs`, `src/gui/app_lookup.rs`, `src/gui/app_plugins.rs`,
`src/plugins/bridge.rs`, `tests/p1_f19_award_references.rs`, `tests/security_guard.rs`,
`docs/ADIF_AWARD_REFERENCES.md`.

This defect must not be reopened without a reproducible regression.

### F-20 (P1) — LoTW/eQSL upload selection was global and truncated — F-20 = CLOSED

Original defect: the LoTW upload button selected the most recent 1000 QSOs with **no journal
scope** (`db.get_recent_qsos(1000)`), so one submission could mix every journal in the database,
omit older QSOs of the active journal, and never record which QSOs were actually uploaded. The
eQSL button was journal-scoped but still applied `LIMIT 1000` as a silent truncation, posted to
`.../ImportADIF.txt` (the documentation page, not the processor) and classified success from loose
body substrings without ever looking at the HTTP status. Files: `src/gui/online_sync.rs`,
`src/cloud/lotw.rs`, `src/cloud/eqsl.rs`.

Root causes (four independent defects):

1. a **global** selector (`get_recent_qsos`) used as an upload selection;
2. `LIMIT N` used as the selection instead of as a batch size, in all seven services;
3. TQSL's documented twelve-value exit-code contract collapsed to a boolean, so the documented
   partial outcome (code `9`) and the "no QSOs processed" outcome (code `8`) were reported as a
   generic error;
4. no write-back of the upload result, so nothing prevented resubmitting accepted QSOs and nothing
   distinguished a completed batch from an unstarted one after a restart.

Software remediation (2026-11-17, `STAGE-P1-F20-SOFTWARE`, software-only verification):

- one central, pure model `src/cloud/upload_plan.rs` owns the upload **scope**: a validated
  `JournalId` taken from `journals.id` (never a journal name, never a GUI index), a materialised
  and canonically ordered QSO identity set, a deterministic `plan_id` (SHA-256 over service,
  journal, policy and ordered ids), and a lossless, disjoint batch partition — every record in
  exactly one batch, the sum of batch lengths equal to the plan size, no empty batch, and
  `batch_size == 0` as a controlled error;
- `src/core/database/upload_selection.rs` selects **all** eligible records of exactly one journal
  with no `LIMIT` and no pagination, using the officially documented ADIF `QSL Sent` semantics
  (`Y` = "uploaded to, and accepted by, the online service" and `I` = "ignore or invalid" are
  excluded; `N`/`R`/`Q` stay eligible);
- status is written only by `commit_batch_acceptance`, which sets `<service>_qsl_sent = 'Y'` in one
  transaction **after** the service returned batch-level proof of acceptance, asserts that the
  number of updated rows equals the batch size, and rolls back on any mismatch;
- LoTW uses the documented TQSL exit-code table (`classify_tqsl_exit`): only code `0` is
  acceptance, code `9` is partial, code `8` means nothing was processed, code `1` is cancellation
  and code `11` a connection error; the `Final Status:` stderr line is parsed as a secondary
  signal only;
- eQSL posts to the documented processor `https://www.eqsl.cc/qslcard/ImportADIF.cfm` (FORM POST
  carrying `EQSL_USER`, `EQSL_PSWD`, `ADIFData`), never retries an upload automatically, checks the
  HTTP status, and requires the documented `Result: x out of y records added` line with
  `x == y == records sent` before any acceptance is recorded; the response is redacted of the
  password before it leaves the client;
- every online upload (LoTW, eQSL, Club Log, QRZ.com, HamQTH, Cloudlog, HRDLog) now goes through
  the same plan, so no service can silently truncate;
- cancellation is explicit: it stops subsequent batches, keeps already accepted batches and can
  never produce a success status.

Verification boundaries recorded by this gate:

```
F-20 = CLOSED
COMPLETED_JOURNAL_SCOPED_UPLOAD_SELECTION_REMEDIATION_VERIFIED
LoTW selection = JOURNAL_SCOPED
eQSL selection = JOURNAL_SCOPED
Silent truncation = REMOVED
Batching = LOSSLESS_SOFTWARE_VERIFIED
Partial success = MODELED
Real LoTW upload = NOT_EXECUTED
Real eQSL upload = NOT_EXECUTED
Real credentials = NOT_USED
SOFTWARE_ONLY_VERIFICATION=true
F-14 / CAT-018 / CAT-019 / F-15 / F-16 / F-17 / F-19 = remain CLOSED
F-21 = CLOSED (closed by STAGE-P1-F21-SOFTWARE, see §1 F-21)
NATIVE-7B = NOT_STARTED
```

This is a software-only verification. It does **not** claim that LoTW or eQSL accepted a real
submission: no real upload was performed, no account was used, no real credentials or TQSL
certificate were handled, and every fixture is synthetic. It also does not claim empirical
verification of every service response — the classifiers are pinned to the official documented
contract and exercised with test doubles. Hardware readiness and hardware PTT remain NOT_VERIFIED,
the ten hardware-in-the-loop tests remain `#[ignore = "hardware-in-the-loop"]`, and NATIVE-7B
remains NOT_STARTED. No database migration was required: the existing `*_qsl_sent` columns make the
selection idempotent, and `LOTW_QSLSDATE` / `EQSL_QSLSDATE` are still not persisted (documented
residual gap).

Files: `src/cloud/upload_plan.rs`, `src/cloud/eqsl.rs`, `src/cloud/lotw.rs`,
`src/cloud/scheduler.rs`, `src/core/database/upload_selection.rs`, `src/core/database/mod.rs`,
`src/gui/online_sync.rs`, `src/gui/online_sync_upload.rs`, `src/gui/state/online_sync.rs`,
`src/gui/mod.rs`, `locales/*.json`, `tests/p1_f20_upload_selection.rs`, `tests/security_guard.rs`,
`docs/UPLOAD_SELECTION.md`.

This defect must not be reopened without a reproducible regression.

### F-21 (P1) — Cabrillo export caps at 10 000 QSOs — F-21 = CLOSED

Original defect: `SpLogApp::export_cabrillo` fetched QSOs through
`db.get_recent_qsos_for_journal(&self.active_journal.id, 10000)` (`ORDER BY ... DESC LIMIT 10000`),
silently dropping the oldest contacts of any contest log above 10 000 QSOs, and passed that
descending, truncated slice to `compute_scoreboard` to compute `CLAIMED-SCORE`. In addition,
`cabrillo::build` sorted raw date/time strings without separator normalization or 6-digit time
padding and fell back to the wall clock (`jiff::Timestamp::now()`) when a record had a missing or
hyphenated date or a malformed time.

Software remediation (2026-11-18, `STAGE-P1-F21-SOFTWARE`, software-only verification):

- `export_cabrillo`, `export_pdf_log` and `export_gpx_log` in `src/gui/app_export.rs` now query the
  full journal via `db.get_all_qsos_for_journal(&self.active_journal.id)` (`WHERE journal_id = ?1
  ORDER BY ORDER_CHRONO_ASC`, no `LIMIT`);
- `render_contest_window` in `src/gui/contest.rs` refreshes `ContestCache` from
  `db.get_all_qsos_for_journal(&app.active_journal.id)` keyed by `ContestCacheKey` (including
  `journal_qso_count`), so the live Contest window scoreboard and multiplier matrix use the same
  complete journal set as Cabrillo export rather than the paginated/filtered table view buffer;
- `canonical_qso_sort_key` in `src/core/cabrillo.rs` normalizes date (`YYYYMMDD`), 6-digit padded
  time (`HHMMSS`) and `id`, matching SQLite `ORDER_CHRONO_ASC`, and is used by both
  `cabrillo::build` and `compute_scoreboard` (`src/core/contest_stats.rs`) so duplicate detection
  always credits the earliest chronological contact regardless of input slice order;
- `jiff::Timestamp::now()` is removed from `src/core/cabrillo.rs`:
  `format_and_validate_cabrillo_date` (`YYYYMMDD` / `YYYY-MM-DD` validated via `jiff::civil::Date`
  → `"YYYY-MM-DD"`) and `format_and_validate_cabrillo_time` (`HHMM` / `HHMMSS` / `HH:MM` /
  `HH:MM:SS` with range checks → `"HHMM"`) fail closed with `ContestError::InvalidQsoTimestamp`.

Verification boundaries recorded by this gate:

```
F-21 = CLOSED
COMPLETED_CABRILLO_UNTRUNCATED_EXPORT_AND_SCORE_REMEDIATION_VERIFIED
Cabrillo selection = JOURNAL_SCOPED_UNTRUNCATED
Claimed score = FULL_CANONICAL_SET_VERIFIED
Timestamp fallback to wall clock = REMOVED_FAIL_CLOSED
Same-minute ordering = CANONICAL_DETERMINISTIC_VERIFIED
Real user data = NOT_USED
SOFTWARE_ONLY_VERIFICATION=true
F-14 / CAT-018 / CAT-019 / F-15 / F-16 / F-17 / F-19 / F-20 = remain CLOSED
NATIVE-7B = NOT_STARTED
```

Files: `src/core/cabrillo.rs`, `src/core/contest_rules.rs`, `src/core/contest_stats.rs`,
`src/gui/app_export.rs`, `src/gui/contest.rs`, `src/gui/state/contest.rs`,
`tests/p1_f21_cabrillo_export.rs`, `tests/security_guard.rs`, `docs/KNOWN_ISSUES.md`.

This defect must not be reopened without a reproducible regression.

### F-05 (P2) — MSI and Inno Setup installers are not published by CI

Both installers are implemented and locally verified, but CI builds and publishes only the
portable Windows ZIP. Users cannot obtain an installer from a release.

### F-06 (P2) — reference-data downloads are unsigned (atomic write and payload validation implemented)

Third-party reference feeds (`cty.dat`, `MASTER.SCP`, `lotw-user-activity.csv`, country-file
archives) do not publish cryptographic signatures upstream. All reference download paths now enforce
a streaming `BoundedBuffer` size limit (`MAX_REFERENCE_FILE_SIZE = 32 MiB`), reject empty/whitespace
and HTML error payloads (`<!DOCTYPE html` / `<html`), and install via same-directory atomic
temp-file-plus-rename (`write_file_atomic_sync` in `src/cloud/updater.rs` and `install_set` in
`src/core/reference/country_files.rs`), while `trigger_database_update` in
`src/gui/app_services.rs` reloads `MASTER.SCP` only when the update succeeded and the file is
non-empty.

### F-07 (P2) — XDG precedence can be shadowed by a portable database

A `databases/` directory next to the binary takes precedence over the XDG data directory,
which can silently shadow user data after an unpack-anywhere install.

### F-22 (P2) — system-managed updates lack a lock, journal and health proof

The Inno/MSI delegation path relies on the package manager for transactionality and does not
create `.update.lock`, an update journal or a health proof of its own.

### F-18 (P2) — redaction can panic on non-ASCII input — F-18 = CLOSED

Original defect: `redact_sensitive_info` in `src/cat/tx_arbiter.rs` searched a lowercased copy
(`out.to_lowercase()`) and sliced `out` at byte offsets computed from that copy, which could land
inside a multibyte UTF-8 scalar or panic when a character changed byte length on lowercase
conversion (e.g. `İ` U+0130 or `Ⱥ` U+023A).

Remediation (2026-11-18): `redact_sensitive_info` matches ASCII secret prefixes (`password=`,
`token:`, `bearer `, `api_key=`) directly on `out.as_bytes()` windows with `eq_ignore_ascii_case`
(without calling `.to_lowercase()`), scans the token extent on bytes, and clamps replacement offsets
with `ceil_char_boundary` / `floor_char_boundary`. Files: `src/cat/tx_arbiter.rs`,
`tests/security_guard.rs`.

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
| Hardware-in-the-loop validation (NATIVE-7B) | not executed; 10 `#[ignore]` tests; F-14 software remediation is closed but hardware validation remains NOT_EXECUTED and requires a separate owner authorisation |
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

---

## 6. F-20 residual risks after `STAGE-P1-F20R-SOFTWARE` (2026-11-17)

### F20-R1 — no durable checkpoint of plans and batches (CLOSED, software verified)

Up to F-20 the only durable trace of an upload was `{service}_qsl_sent = 'Y'`. Plan, batch split,
attempt id, per-batch outcome and the acceptance instant were in memory, so 8 of 10 crash windows
were structurally unrecoverable. Schema version 8 adds `upload_plans`, `upload_batches` and
`upload_batch_members`; recovery at startup is unambiguous and never executes a transport. See
[`UPLOAD_SELECTION.md`](UPLOAD_SELECTION.md) §10. Closed as
`IMPLEMENTED_AND_SOFTWARE_VERIFIED`.

### F20-R2 — no `LOTW_QSLSDATE` / `EQSL_QSLSDATE` (CLOSED, software verified)

Schema version 9 adds both ADIF `Date` columns, written only in the same transaction as the
`Y` status and only from a confirmed `Accepted` batch. Historical `Y` rows keep `Y + NULL`; nothing
is backfilled. See [`UPLOAD_SELECTION.md`](UPLOAD_SELECTION.md) §11.

### F20-R3 — credentials in the query string of download paths (CONFINED, residual official limitation)

**Status: `OFFICIAL_ENDPOINT_REQUIRES_QUERY_CREDENTIALS=true`.** This is **not** a closed risk in the
sense "credentials are no longer in the query"; it cannot be, without inventing a protocol.

The current official documentation documents **only** query credentials for both download endpoints:

* LoTW `https://lotw.arrl.org/lotwuser/lotwreport.adi` — *"Each query is an HTTP \"GET\" string"*,
  parameters `login`, `password`; POST, HTTP Basic, session cookies, API keys and OAuth are
  **undocumented**; no safer variant is documented and there is no warning about credentials in
  URLs.
* eQSL `https://www.eqsl.cc/qslcard/DownloadInBox.cfm` — *"You can call the program with the
  following parameters in the URL"*, parameters `UserName`, `Password`; POST is **undocumented**; a
  session/login flow is mentioned (`"mandatory if not logged in"`) but no mechanism, cookie name or
  token lifetime is documented. There is **no** `DownloadArchive.cfm` (HTTP 404); `Archive` is a
  parameter of `DownloadInBox.cfm`.

What this gate guarantees instead:

```text
QUERY_CREDENTIALS_NOT_LOGGED=true
QUERY_CREDENTIALS_NOT_PERSISTED=true
QUERY_CREDENTIALS_NOT_EXPOSED_IN_ERRORS=true
```

`CREDENTIALS_NOT_IN_QUERY` is **false** and is never claimed anywhere.

Residual exposure that remains, by design of the remote service:

1. the credential is present in the outbound request line and therefore visible to anything that can
   observe the TLS-terminated request at the service, to the local network stack, and to any
   intermediary that terminates TLS;
2. changing the method to POST, or introducing a session, would be an **undocumented protocol
   invention** and is forbidden by the gate; it must not be attempted without a new official
   contract;
3. URL-based credentials are incompatible with HTTP Basic-style per-request scoping, so there is no
   application-level way to reduce the exposure further.

Mitigations that **are** implemented: `SecretUrl` with redacted `Debug`/`Display`, redirects
disabled, all transport errors sanitised, the only credential-bearing `String` helper gated to
`#[cfg(test)]`, and no URL persisted in the checkpoint tables
(`f20r_checkpoint_tables_contain_no_secret_or_url`).

### F20-R4 — invented received date on the LoTW download path (CLOSED 2026-11-18, software verified)

`parse_lotw_confirmations` used to fall back to the wall clock when a confirmation had no
`LOTW_QSLRDATE`. The official LoTW report (`lotwreport.adi`, developer page re-read 2026-11-18)
sends `QSL_RCVD`, `QSLRDATE` and `APP_LoTW_RXQSL` (`YYYY-MM-DD HH:MM:SS`) but never
`LOTW_QSLRDATE`. The ADIF import also mapped `QSLRDATE` only to the paper-QSL field. So a
fabricated *received* date was written for practically every confirmation.

Fixed in `STAGE-RUSTSEC-2026-0543-CHRONO-REPLACEMENT` (batch T3) under the owner decision for
this gate. The received date is, in order:

1. `LOTW_QSLRDATE`,
2. the date part of `APP_LoTW_RXQSL`,
3. `QSLRDATE`, only when the LoTW report says `QSL_RCVD=Y`.

Each candidate must be a valid ADIF 3.1.8 `Date` (`adif::adif_date_part`). Without one the date
stays `None` (`LotwConfirmation::qsl_rdate: Option<String>`). The confirmation `UPDATE` uses
`COALESCE`, so a report without a date never erases a stored date.

Evidence:
* `cloud::lotw` tests `f20r4_received_date_*` (4 cases);
* `core::database` test `f20r4_lotw_confirmation_without_date_keeps_the_stored_date`;
* guard `f20r4_lotw_received_date_is_never_invented_guard`.

Software-only: no real LoTW or eQSL report was downloaded. The eQSL confirmation path
(`src/core/adif.rs`, `src/core/database/qsl.rs` `mark_eqsl_confirmed_batch_for_journal`) applies the
same discipline: `eqsl_qslrdate` is resolved from `EQSL_QSLRDATE` (validated by `adif_date_part`) or
`QSLRDATE` (validated by `adif_date_part` only when `EQSL_QSL_RCVD=Y` or `QSL_RCVD=Y`), otherwise
`None` with SQLite `COALESCE(?1, eqsl_qslrdate)` preserving any previously stored date and never
falling back to `QSO_DATE`.
---

## 7. `RUSTSEC-2026-0543` — new advisory blocking CI (2026-11-17) — RESOLVED 2026-11-18

**Resolution (2026-11-18, `STAGE-RUSTSEC-2026-0543-CHRONO-REPLACEMENT`):**
* `chrono` is removed from `Cargo.toml`, `Cargo.lock` and every source file. Time handling uses
  `jiff 1.0.1` (feature `serde`), the only jiff in the graph.
* `env_logger` runs with `default-features = false` (`auto-color`, `regex`): its `humantime`
  feature would pull a second jiff line (0.2). Log timestamps are formatted by jiff in
  `src/main.rs`.
* `cargo audit --deny warnings --no-yanked` with the unchanged seven-entry ignore list exits 0.
  `RUSTSEC-2026-0543` was never added to the ignore list.
* Guards `chrono_replacement_*` block a return of `chrono`, any ignore of `RUSTSEC-2026-0543`,
  and a second or 0.x jiff line.

The record below is kept as history.

`cargo audit --deny warnings` in CI now fails on **`RUSTSEC-2026-0543`**: "chrono is unmaintained",
type INFO / Unmaintained, with **no patched versions**. Source read 2026-11-17T15:4xZ at
`https://rustsec.org/advisories/RUSTSEC-2026-0543` (HTTP 200).

Facts:

* `chrono 0.4.45` is an existing direct dependency; `Cargo.lock` is byte-identical to the F-20
  baseline (`git diff 70b91d2 HEAD -- Cargo.lock` is empty).
* The advisory is **not** in the approved `--ignore` list in `.github/workflows/ci.yml`, and adding it
  requires a formal owner decision (`.github/workflows/ci.yml` §audit, `AGENTS.md` §7).
* The CI advisory database contained 1527 entries at 2026-11-17T14:36Z; the local cache has 1509, so
  a local `cargo audit --no-fetch` still passes. The CI run is authoritative.
* The previous CI run on the F-20 baseline (`44050036049`, 2026-11-17T08:49Z) was `success`, so the
  advisory entered the database during 2026-11-17.
* There is no patched version, so the only code-level remedy is to stop depending on `chrono`
  entirely — a dependency migration that this gate explicitly forbids.

Consequences:

| Effect | Status |
|---|---|
| `STAGE-P1-F20R-SOFTWARE` closure | **BLOCKED_BY_NEW_RUSTSEC_ADVISORY** |
| Software work (checkpoints, sent dates, credential confinement) | complete and verified locally — 3457/3457 tests, clippy clean, fmt clean |
| CI | red on the audit step only; every other CI step is green |
| Unblocking decision needed | owner decision: either approve `RUSTSEC-2026-0543` as an accepted `unmaintained` entry (with the same justification style as the existing seven), or authorise a dedicated dependency gate that removes `chrono` |

Do **not** add the `--ignore` entry without that decision, and do **not** treat this gate as closed.

---

## 8. CI: AppImage toolchain blocked by HTTP 403 (2026-11-17) — RESOLVED 2026-11-18

**Symptom:** Build and Release runs `44102699069` and `44108875925` failed at the AppImage
toolchain step. The very first request, `api.github.com/repos/AppImage/appimagetool/releases/tags/continuous`,
returned `curl: (22) … 403`.

**Cause:** the resolver made four unauthenticated GitHub REST API calls per run. The official
limit for unauthenticated requests is 60 per hour per originating IP, and exceeding it returns
403 or 429. Rate limiting is the most likely cause; the response body was not logged.

**Fix (`STAGE-CI-APPIMAGE-TOOLCHAIN-PINNING`, decision `OWNER-M3-G-10`):**
* The toolchain is pinned statically in `installer/linux/appimage/toolchain-pins`, with the same
  builds as `OWNER-M3-G-03-R1` / `-04-R1`.
* `scripts/install_appimage_toolchain.sh` makes no REST API call and uses no cache. It downloads
  both assets straight from their official release URLs, logs the HTTP status, and fails closed
  on size or SHA-256 mismatch before the file is moved into place or made executable.

**Residual risk:** the `continuous` assets are replaced upstream on every new build. The job then
stops with `PIN_STALE` (size or SHA-256 mismatch, or HTTP 404), and the pinned file can no longer
be downloaded. A new pin needs a new owner decision.
