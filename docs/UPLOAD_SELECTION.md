# LoTW / eQSL upload selection and transport contract

Status: **CURRENT** (`ADR-113`–`ADR-121`, updated 2026-11-17)
Scope: how SPLogbook decides *which* QSOs are uploaded, how the batch plan is built, what evidence
counts as acceptance, and what is deliberately **not** verified.

---

## 1. Problem this contract replaces

Previously, the upload path had four independent defects:

1. **Global selection.** The LoTW button used `db.get_recent_qsos(1000)` — no `journal_id`
   predicate at all — so a submission could contain QSOs from every journal in the database.
   LoTW submissions are irreversible ("Once submitted to LoTW, a QSO cannot be deleted"), so this
   was silent cross-journal disclosure.
2. **Silent truncation.** Every service applied `LIMIT N` *as the selection* (`1000` for
   LoTW/eQSL/QRZ, `5000` for Club Log, `100` for HamQTH/Cloudlog/HRDLog). Older QSOs of the
   active journal were never uploaded and nothing told the user.
3. **Boolean transport result.** TQSL's documented twelve-value exit-code contract was collapsed
   to `output.status.success()`, so the documented partial outcome (code `9`) and "no QSOs
   processed" (code `8`) were reported as generic errors.
4. **No write-back.** No production path ever wrote `lotw_qsl_sent` / `eqsl_qsl_sent`, so every
   click re-uploaded everything and a restart could not distinguish a completed batch from an
   unstarted one.

## 2. Selection contract

| Rule | Implementation |
|---|---|
| Identity | `journals.id` (TEXT), wrapped in a validated `JournalId`; a journal **name** or GUI index is never an identity |
| Missing journal | controlled refusal (`PlanError::MissingJournal` / `InvalidJournalId`) before any transport |
| Scope | exactly one journal per plan; `WHERE journal_id = ?1` in selection, payload read and status write |
| Completeness | **no `LIMIT`, no `OFFSET`, no pagination** — the whole eligible set is materialised |
| Eligibility | `COALESCE(<sent>, 'N') NOT IN ('Y', 'I')` |
| Order | canonical **ascending**: normalised `QSO_DATE`, normalised `TIME_ON` (right-padded to 6), `id` as tie-breaker |
| Determinism | the plan sorts internally, so the order does not depend on SQL, on input order, or on the GUI table sort |
| Plan identity | `plan_id = first 16 hex of SHA-256(service \| journal \| policy \| ordered ids)` — stable across restarts, no RNG |
| Immutability | switching the active journal, or adding/editing/deleting QSOs after plan creation, cannot change the plan |

### Why `Y` and `I` are excluded

ADIF 3.1.8 defines the `QSL Sent` enumeration used by `LOTW_QSL_SENT` and `EQSL_QSL_SENT`:

| Value | Official meaning | Selection |
|---|---|---|
| `Y` | "the QSO has been uploaded to, and accepted by, the online service" | excluded (already accepted) |
| `N` | "do not upload the QSO to the online service" (default) | eligible |
| `R` | "the contacted station has requested the QSO be uploaded" | eligible |
| `Q` | "a QSO has been selected to be uploaded" | eligible |
| `I` | "ignore or invalid" | excluded (explicitly ignored) |

`*_QSL_RCVD` (confirmation) is never used for eligibility: upload status and confirmation are
distinct, and confirmations are written only by the unrelated download path.

## 3. Batching contract

| Invariant | Statement |
|---|---|
| complete | `Σ batch.len() == plan.total_records()` |
| disjoint | `batch_i ∩ batch_j = ∅` |
| non-empty | a batch always contains at least one record |
| ordered | concatenating batches reproduces the plan order |
| bounded payload | each request carries at most `batch_size` records |
| controlled zero | `batch_size == 0` is an error, never "no batches" |
| empty plan | zero records → zero batches, reported as "nothing to upload" (not as success) |

### Batch sizes are client transport policy, not service limits

| Service | Size | Provenance |
|---|---|---|
| LoTW (TQSL) | 500 | No per-file record limit is documented for TQSL/LoTW; the documented constraints are per-QSO **content**. The size bounds one process invocation and one temp file. |
| eQSL (`ImportADIF`) | 250 | No maximum record count is documented for the real-time interface. The ≈1000 figure belongs to the **manual browser upload**, and the URL variant is documented as "normally be limited to a single QSO" for URL-length reasons only. |
| Club Log | 5000 | preserves the historical single-request size; >5000 now yields a second batch instead of truncation |
| QRZ.com | 1000 | as above |
| HamQTH / Cloudlog / HRDLog | 100 | one request per QSO already; the chunk only bounds how many rows are read at a time |

The gate forbids both "assume 1000 from memory" and "remove the limit and send an unbounded
payload". Neither is done: no service record limit is encoded anywhere, and no single request is
unbounded, while the *plan* still carries every record.

## 4. LoTW transport contract (TQSL)

Invocation (CLI parameters unchanged):

```
tqsl -d -u -a all -x -l <StationLocation> <adif-file>
```

The certificate password is **never** passed on the command line (`-p` is not used): that would
leak the secret into the process table and logs. TQSL owns the certificate store.

### Documented exit codes → status

| Code | Meaning | Batch outcome | Marks records accepted |
|---|---|---|---|
| 0 | all QSOs signed and saved/uploaded | `Accepted` | **yes** |
| 1 | cancelled by user | `Cancelled` | no |
| 2 | rejected by LoTW | `Rejected` | no |
| 3 | unexpected response from TQSL server | `Unknown` | no |
| 4 / 5 | TQSL / TQSLlib error | `Failed` | no |
| 6 / 7 | cannot open input / output file | `Failed` | no |
| 8 | no QSOs processed (duplicates and/or out of date range) | `NoRecordsProcessed` | no |
| 9 | **some QSOs processed, some ignored** | `PartiallyAccepted` | no |
| 10 | command syntax error | `Failed` | no |
| 11 | LoTW connection error | `Failed` | no |
| other / missing | undocumented | `Unknown` | no |

The `Final Status: <description> (<code>)` stderr line is parsed as a **secondary** signal; the
numeric exit code is authoritative.

Code `9` is why a boolean result is structurally wrong: it is a non-zero exit that is *not* a
plain failure, and code `8` is an outcome in which nothing was processed but nothing failed
either. Code `8` conflates duplicates with "out of date range", so it cannot be used to mark a
QSO accepted.

## 5. eQSL transport contract (`ImportADIF`)

* Endpoint: `https://www.eqsl.cc/qslcard/ImportADIF.cfm` (**the processor**, not the
  `ImportADIF.txt` documentation page, which was the pre-gate target and could produce a false
  success because the documentation text contains words the old heuristic looked for).
* Method: **FORM POST** with `EQSL_USER`, `EQSL_PSWD` and `ADIFData`; credentials travel in the
  body, never in a URL.
* **No automatic retry** of an upload: an ambiguous outcome must not be laundered into success.
* The response body is redacted of the password before it leaves the client.

### Response classification

| Observed | Classification |
|---|---|
| HTTP status ≠ 200 | fatal, nothing imported |
| `Result: x out of y records added` with `x == y == records sent` | accepted |
| same line with `0 < x < y` | partial |
| same line with `x == 0` | no records processed |
| same line with `y != records sent` | failed — the payload did not arrive whole |
| documented `Error:` that guarantees nothing was imported | rejected |
| other `Error:` (e.g. stop-at-record) | ambiguous → `Unknown` |
| `Warning:` lines only | partial (documented: the record was **not** imported) |
| reply page without any `Result:` line | `Unknown` — since 02.03.2026 a payload without `<EOR>` terminates silently |

HTTP 200 alone is never acceptance.

## 6. Per-QSO status and partial success

| Batch outcome | Record status | Database write |
|---|---|---|
| `Accepted` | `Accepted` | `<service>_qsl_sent = 'Y'`, one transaction, row count must equal batch size |
| `PartiallyAccepted` | `Unknown` | none |
| `NoRecordsProcessed` | `Unknown` | none |
| `Rejected` | `Rejected` | none |
| `Unknown` / `TimedOut` | `Unknown` | none |
| `Failed` | `Failed` | none |
| `Cancelled` / not reached | `NotAttempted` | none |

Run status: `Empty`, `Complete`, `Partial`, `Failed`, `Cancelled`. **`Complete` is the only
success**, and it requires every batch to be accepted.

Transaction rules: the write happens *after* the transport call (never during), verifies the
journal and the exact id set, and rolls back on a row-count mismatch — so a partially written
status is impossible.

## 7. Retry, cancellation, recovery

* No automatic retry anywhere in the upload path.
* Retry is explicit and re-derives the eligible set, so records proven accepted (`Y`) cannot be
  included again. Ambiguous records remain eligible, and the resulting possible resend is a
  service-tolerated duplicate that is reported to the operator — never silently counted as a new
  success.
* Cancellation stops subsequent batches, keeps accepted batches, and can never produce a success
  status.
* There is no durable per-batch checkpoint (the schema has no upload-attempt table). Recovery is
  idempotent re-derivation: a crash between service acceptance and the status commit merely makes
  those records eligible again. This is a documented residual gap, not a silent behaviour.

## 8. Deliberate exclusions

* No real LoTW or eQSL submission was performed during software verification; no real account, credential, token
  or TQSL certificate was used.
* No service response is claimed to be empirically verified; classifiers are pinned to official
  documentation and to test doubles.
* No schema migration: `LOTW_QSLSDATE` / `EQSL_QSLSDATE` are still not persisted, so ADIF export
  omits those optional dates.
* Test transports (loopback HTTP server, endpoint override) exist only under `#[cfg(test)]` and
  are unreachable from the GUI, CLI, configuration, environment or a release artefact.

## 9. Traceability

| Contract element | Code | Guard / test |
|---|---|---|
| journal identity + plan | `src/cloud/upload_plan.rs` | `tests/p1_f20_upload_selection.rs`, `security_guard::p1_f20_upload_selection_is_journal_scoped_guard` |
| journal-scoped, complete selection | `src/core/database/upload_selection.rs` | `p1_f20_no_global_or_truncated_upload_selector_guard` |
| lossless batching | `src/cloud/upload_plan.rs` | `p1_f20_lossless_batching_invariants_guard`, batching matrix |
| TQSL exit-code contract | `src/cloud/lotw.rs` | `cloud::lotw::tqsl_contract_tests` |
| eQSL response contract | `src/cloud/eqsl.rs` | `cloud::eqsl::tests` (incl. loopback) |
| status only on evidence | `src/core/database/upload_selection.rs` | `p1_f20_status_update_is_journal_scoped_guard` |
| partial / timeout / cancel | `src/cloud/upload_plan.rs` | `p1_f20_partial_timeout_cancellation_are_not_success_guard` |
| credential safety | `src/cloud/eqsl.rs`, `src/gui/online_sync_upload.rs` | `p1_f20_credentials_never_in_upload_url_or_logs_guard` |
| test-scope-only transports | `src/cloud/eqsl.rs` | `p1_f20_test_transport_is_test_scope_only_guard` |
| no real endpoints in tests | `tests/**` | `p1_f20_no_real_endpoints_in_tests_guard` |

---

# Part II — F-20 residual hardening (2026-11-17)

This part closes the three risks that F-20 left open. It does **not** reopen anything F-20
closed: selection, batching, plan identity, journal-scoped status updates, LoTW/eQSL partial-result
interpretation, `ImportADIF.cfm` and upload-path credential redaction are unchanged, and the F-20
suites still pass unmodified.

## 10. Persistent upload checkpoints (F20-R1)

Previously, the only durable trace of an upload was `{service}_qsl_sent = 'Y'` on the QSO row.
The plan, the batch split, the attempt id, the per-batch outcome and the acceptance instant existed
only in memory, so 8 of the 10 crash windows were structurally unrecoverable.

Three additive tables now make the plan durable (schema version **8**):

| Table | Purpose | Deliberately absent |
|---|---|---|
| `upload_plans` | `plan_id`, `schema_version`, `service`, `journal_id`, `selection_policy` (+ version), `canonical_order_version`, `batch_policy_version`, `plan_digest`, `total_qsos`, `total_batches`, `created_at`, `state`, `updated_at`, `error_class` | no credentials, no URL, no payload |
| `upload_batches` | `(plan_id, ordinal)` PK, `batch_id` UNIQUE per plan, `service`, `journal_id`, `record_count`, `batch_digest`, `attempt_id`, `state`, `started_at`, **`accepted_at`**, `completed_at`, `accepted_count`, `rejected_count`, `unknown_count`, `evidence_digest`, `error_class`, `retry_eligible` | no response body, no URL, no certificate |
| `upload_batch_members` | `(plan_id, ordinal, position)` PK → `qso_id`: the **ordered, durable membership** | — |

`accepted_at` is the instant at which durable acceptance evidence was obtained. It is the **only**
source of the ADIF sent date (§11); neither the commit time, nor the migration time, nor `QSO_DATE`
is used.

### States

Plan: `Planned → Running → {Completed | Cancelled | RecoveryRequired}`.
Batch: `Planned → Prepared → InFlight → {Accepted | PartiallyAccepted | Rejected | Unknown |
Cancelled}`; `Accepted → Committing → Completed`; anything invariant-violating → `RecoveryRequired`.

`Committing` is written **inside** the same transaction that writes `Completed`. A crash therefore
exposes either `Accepted` (transaction rolled back) or `Completed` (committed) — never `Committing`.
That non-observability *is* the proof of atomicity; an observed `Committing` is treated as
`RecoveryRequired`.

### Recovery matrix (no transport is ever executed during recovery)

| Durable state after crash | Action | Result |
|---|---|---|
| `Planned` | resume allowed | batch may start |
| `Prepared` | resume allowed | batch may start |
| `InFlight` | **fail closed** | → `Unknown` (`interrupted_in_flight`), never `Accepted`, never auto-retried |
| `Accepted` | local commit only | status + date written from the durable `accepted_at`; **no re-upload** |
| `Committing` | invariant violation | → `RecoveryRequired` |
| `PartiallyAccepted` | manual | no automatic action |
| `Rejected` | manual | no automatic action |
| `Unknown` | manual | never auto-retried |
| `Cancelled` | terminal | never auto-sent |
| `Completed` | terminal | **never resent** |

Recovery runs once at application start (`SpLogApp::build` → `recover_interrupted_uploads`) and is
purely local.

### Idempotency and transport model

* A batch carries a `batch_digest` over `(service, journal, plan_id, ordinal, ordered ids)`; a
  changed composition, order or size is detected and rejected (`MembershipMismatch`).
* `open_or_create_upload_checkpoint` reuses an existing checkpoint only when `plan_digest` and the
  counts match; otherwise it returns `PlanConflict`. A different batch policy for the same record
  set produces the same F-20 `plan_id` but a different digest, so the durable plan is **not**
  rewritten.
* The model is **at-most-once for `Completed`** and **unknown for `InFlight`**. The words
  "exactly-once" are not used anywhere, because neither LoTW nor eQSL documents idempotency.

## 11. ADIF sent dates (F20-R2)

Schema version **9** adds the two nullable `TEXT` columns `lotw_qslsdate` and `eqsl_qslsdate`.
They are **not** backfilled: a historical `Y` row keeps `Y + NULL`, because no documented source for
that date exists. The migration contains no `UPDATE`, no `datetime()` and no `DEFAULT`.

Per the official ADIF 3.1.8 specification:

* `LOTW_QSLSDATE` — `Date` — "date QSL sent to ARRL Logbook of the World (only valid if
  LOTW_QSL_SENT is Y, Q, or I)".
* `EQSL_QSLSDATE` — `Date` — "date QSL sent to eQSL.cc (only valid if EQSL_QSL_SENT is Y, Q, or I)".
* `Date` is exactly 8 digits, UTC, `YYYYMMDD`, with `1930 <= YYYY`.

Implementation rules:

* the date is written **only** in the same transaction that writes `{service}_qsl_sent = 'Y'`, and
  only from a confirmed `Accepted` batch;
* the column is written as `COALESCE(<col>, ?date)`, so a later attempt cannot overwrite the
  originally accepted date;
* export is gated on the status (`Y`/`Q`/`I`) and on a valid 8-digit, ≥ 1930 date; an unusable value
  is **omitted**, never repaired;
* import keeps whatever the file contains and never invents a date;
* rejected / timed-out / cancelled / partially accepted / unknown batches write **no** date;
* the date is never derived from `QSO_DATE`.

The ADIF specification calls the field the *sent* date while `Y` means *accepted*; neither LoTW nor
eQSL publishes an official "date sent". The UTC date of confirmed acceptance is therefore the only
value for which this application holds proof, and it is documented as an interpretation rather than
as a restatement of the specification.

## 12. Download credential transport (F20-R3)

Every download path was inventoried against the current official documentation:

| Path | Endpoint | Method | Credential location | Safer documented variant |
|---|---|---|---|---|
| LoTW confirmation report | `https://lotw.arrl.org/lotwuser/lotwreport.adi` | **GET** ("Each query is an HTTP \"GET\" string") | `login`, `password` in the query | **none documented** (POST, Basic, session, cookie, API key and OAuth are all undocumented) |
| eQSL inbox download | `https://www.eqsl.cc/qslcard/DownloadInBox.cfm` | **GET** ("call the program with the following parameters in the URL") | `UserName`, `Password` in the query | **none documented** (POST undocumented; a login/session flow is mentioned without any mechanism) |
| eQSL outbox download | `DownloadADIF.cfm` | GET | same | same (not used by this build) |
| eQSL archive | **no such endpoint** — `DownloadArchive.cfm` returns 404 | — | `Archive` is a *parameter* of `DownloadInBox.cfm` | do not invent an endpoint |

Because the official contract provides **no** safer mechanism, the method and the endpoint are
unchanged. Instead the exposure is confined:

* a `SecretUrl` wrapper renders only `scheme://host/path` from `Debug` and `Display`; the full
  address is not reachable through any public accessor that returns `&str`/`String`;
* the download clients use `http_client_no_redirect` (`reqwest::redirect::Policy::none()`), so a
  `3xx` can never replay the query to another address; a redirect is returned as an ordinary
  response without evidence of success;
* every transport error is sanitised through `sanitize_network_error` before it leaves the client;
* `build_report_url` — the only helper that ever produced a credential-bearing `String` — now exists
  **only** under `#[cfg(test)]`;
* no URL is logged, persisted, put in a checkpoint, put in telemetry or emitted in an error.

The honest status vocabulary is therefore:

```
Persistent upload checkpoints = IMPLEMENTED_AND_SOFTWARE_VERIFIED
Crash recovery = SOFTWARE_VERIFIED
Unknown remote outcome = FAIL_CLOSED_MANUAL_RECONCILIATION_REQUIRED
LOTW_QSLSDATE = IMPLEMENTED
EQSL_QSLSDATE = IMPLEMENTED
Download credential transport = OFFICIAL_CONTRACT_VERIFIED
Real service login = NOT_EXECUTED
Real upload/download = NOT_EXECUTED

OFFICIAL_ENDPOINT_REQUIRES_QUERY_CREDENTIALS=true
QUERY_CREDENTIALS_NOT_LOGGED=true
QUERY_CREDENTIALS_NOT_PERSISTED=true
QUERY_CREDENTIALS_NOT_EXPOSED_IN_ERRORS=true
```

`CREDENTIALS_NOT_IN_QUERY` is **false** and is never claimed.

## 13. Traceability (F-20R)

| Contract element | Code | Guard / test |
|---|---|---|
| durable plan and batch model | `src/cloud/upload_checkpoint.rs` | `cloud::upload_checkpoint` unit tests, `p1_f20r_persistent_checkpoint_schema_guard` |
| persistence, transitions, atomic commit | `src/core/database/upload_checkpoint.rs` | `p1_f20r_recovery`, `p1_f20r_sent_dates` |
| checkpoint-driven orchestration | `src/gui/online_sync_upload.rs` | `p1_f20r_orchestration_is_checkpoint_driven_guard` |
| startup recovery | `src/gui/app.rs` | `crash_*` / `restart_*` tests |
| InFlight is never Accepted | `src/cloud/upload_checkpoint.rs` | `p1_f20r_in_flight_never_becomes_accepted_guard`, `stress_3000_in_flight_restarts_all_become_unknown` |
| Unknown not retried, Completed not resent | `src/cloud/upload_checkpoint.rs` | `p1_f20r_unknown_not_retried_and_completed_not_resent_guard` |
| status + date atomicity | `src/core/database/upload_checkpoint.rs` | `p1_f20r_status_and_sent_date_are_atomic_guard`, `a_failed_row_count_rolls_back_both_status_and_date` |
| `LOTW_QSLSDATE` / `EQSL_QSLSDATE` | `src/core/qso.rs`, `src/core/adif.rs`, schema v9 | `p1_f20r_sent_date_fields_exist_and_round_trip_guard`, `p1_f20r_sent_dates` |
| no invented historical dates | schema v9, export gate | `p1_f20r_no_invented_historical_dates_guard`, `legacy_accepted_rows_keep_status_without_a_sent_date` |
| download credential confinement | `src/core/http.rs`, `src/cloud/lotw.rs`, `src/cloud/eqsl.rs` | `p1_f20r_download_credentials_are_confined_guard`, `p1_f20r_credential_safety` |
| no secret in checkpoints | schema v8 tables | `p1_f20r_checkpoints_store_no_secret_guard`, `f20r_checkpoint_tables_contain_no_secret_or_url` |
| test-scope-only hooks | `src/core/database/mod.rs` | `p1_f20r_test_only_helpers_never_reach_production_guard` |