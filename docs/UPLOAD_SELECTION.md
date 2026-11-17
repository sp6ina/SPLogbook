# LoTW / eQSL upload selection and transport contract

Status: **CURRENT** (introduced by `STAGE-P1-F20-SOFTWARE`, 2026-11-17)
Scope: how SPLogbook decides *which* QSOs are uploaded, how the batch plan is built, what evidence
counts as acceptance, and what is deliberately **not** verified.

---

## 1. Problem this contract replaces

Before this gate the upload path had four independent defects:

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

Invocation (parameters unchanged by this gate):

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

* No real LoTW or eQSL submission was performed by this gate; no real account, credential, token
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
