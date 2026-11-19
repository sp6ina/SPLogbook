# LoTW / TQSL / eQSL.cc — official contract register (current as of 2026-11-17)

Status: **CURRENT** — established from official LoTW and eQSL documentation (`ADR-113`–`ADR-121`).
Purpose: the single place that records *what the services actually document*, so that no code
change ever has to rely on memory, on another logger, or on a guess.

Every fact below was read from the official document at the given URL during the gate's research
window (**2026-11-17T10:19Z – 2026-11-17T11:0xZ**). No account was created, no login was attempted,
no credential was sent, and no upload or download was performed against either service.

---

## 1. ADIF (the data format)

| Item | Value |
|---|---|
| Current stable version | **3.1.8** |
| Specification | https://www.adif.org/318/ADIF_318.htm — "(Released ADIF Version 3.1.8, updated 2026-10-12)" |
| Index | https://www.adif.org/ — lists only "ADIF V 3.1.8" as current |
| Newer / draft version | none published (`https://www.adif.org/319/…` → HTTP 404) |

### Fields used by SPLogbook

| Field | Type | Official definition |
|---|---|---|
| `LOTW_QSL_SENT` | Enumeration (`QSL Sent`) | "ARRL Logbook of the World QSL sent status", Default Value: `N` |
| `LOTW_QSLSDATE` | `Date` | "date QSL sent to ARRL Logbook of the World" — "(only valid if LOTW_QSL_SENT is Y, Q, or I)" |
| `LOTW_QSL_RCVD` | Enumeration (`QSL Rcvd`) | "ARRL Logbook of the World QSL received status", Default Value: `N` |
| `LOTW_QSLRDATE` | `Date` | "date QSL received from ARRL Logbook of the World" — "(only valid if LOTW_QSL_RCVD is Y, I, or V)", "(V import-only)" |
| `EQSL_QSL_SENT` | Enumeration (`QSL Sent`) | "eQSL.cc QSL sent status", Default Value: `N` |
| `EQSL_QSLSDATE` | `Date` | "date QSL sent to eQSL.cc" — "(only valid if EQSL_QSL_SENT is Y, Q, or I)" |
| `EQSL_QSL_RCVD` | Enumeration (`QSL Rcvd`) | "eQSL.cc QSL received status", Default Value: `N` |
| `EQSL_QSLRDATE` | `Date` | "date QSL received from eQSL.cc" — "(only valid if EQSL_QSL_RCVD is Y, I, or V)", "(V import-only)" |

`Date` = "8 Digits representing a UTC date in YYYYMMDD format", with `1930 <= YYYY`,
`1 <= MM <= 12`, `1 <= DD <= DaysInMonth(MM)`. No separators, no time, no timezone offset.
The sent/received dates have **no Default Value**, so an absent date stays unknown — it must never
be coerced to `""` or to an invented date.

`QSL Sent` enumeration (exactly five values): `Y` = "the QSO has been uploaded to, and accepted by,
the online service"; `N` = do not upload; `R` = the **contacted station** requested the upload;
`Q` = "a QSO has been selected to be uploaded to the online service"; `I` = ignore or invalid.

`QSL Rcvd` enumeration: `Y`, `N`, `R`, `I`, plus `V` (verified) which is **import-only** and must
never be emitted on export.

---

## 2. ARRL LoTW and TrustedQSL (TQSL)

| Item | Value |
|---|---|
| Current stable TQSL | **2.8.7** — https://www.arrl.org/tqsl ("The latest version of TQSL (version 2.8.7) can be downloaded here.") |
| Release notes | https://www.arrl.org/tqsl/TrustedQSL-2-8-7-release.pdf (PDF `/CreationDate` 2026-10-25 21:20:59 −04:00) |
| Command line | https://lotw.arrl.org/lotw-help/cmdline?lang=en — **stale**: still lists exit codes 0–11 |
| Duplicate handling | https://lotw.arrl.org/lotw-help/developer-submit-qsos/ |
| Download / query API | https://lotw.arrl.org/lotw-help/developer-query-qsos-qsls/ |
| Account activity (HTML) | https://lotw.arrl.org/lotw-help/view-accepted-qsos/?lang=en |

### Command line used by SPLogbook

```
tqsl -d -u -a all -x -l <StationLocation> <adif-file>
```

* `-d` suppress the date-range dialog, `-u` upload, `-a all` sign every QSO in the file,
  `-x` batch mode (status on stderr, exit code), `-l` station location.
* `-p` exists (passphrase for a protected Callsign Certificate) and is **not used**: the passphrase
  would appear in the process table and in TQSL's own diagnostic trace (`Command Line: …`).
* `-a new` — still shown on the website — is invalid on 2.8.7 (only `ask|abort|compliant|all` are
  accepted; anything else exits 10).

### Exit codes

The **current** table is 0–15 (TQSL 2.8.7 shipped `cmdline.htm` and `TQSL_EXIT_*` in
`apps/tqslapp.h`); the website still stops at 11.

| Code | Official meaning |
|---|---|
| 0 | all QSOs submitted were signed and saved or signed and uploaded |
| 1 | cancelled by user |
| 2 | rejected by LoTW |
| 3 | unexpected response from the TQSL server |
| 4 | TQSL error |
| 5 | TQSLlib error |
| 6 | unable to open input file |
| 7 | unable to open output file |
| 8 | no QSOs processed (already uploaded and/or out of date range) |
| 9 | some QSOs processed, some ignored (already uploaded and/or out of date range) |
| 10 | command syntax error |
| 11 | LoTW connection error |
| 12 | unknown error |
| 13 | the TQSL upload-tracking database was locked |
| 14 | some QSOs in a log being signed were already uploaded |
| 15 | invalid passphrase given when importing a P12 file |

SPLogbook classifies **0–11 exactly as documented** (F-20 contract, unchanged) and routes
**12–15 to `Unknown`**, which is fail-closed: never `Accepted`, never auto-retried. Codes 12–15 were
not in the F-20 baseline, so recognising them as documented values while keeping the fail-closed
outcome preserves the closed contract without reopening it.

**Exit 0 does not mean LoTW accepted the QSO.** The documented example is
`Log uploaded successfully with result "File queued for processing"!` followed by
`Final Status: Success (0)`.

### Duplicates and idempotency

TQSL detects duplicates **locally** (CALL/BAND/MODE/PROP_MODE/SAT_NAME + Station Location unchanged),
and the documentation states that "any digitally-signed log file generated by TrustedQSL that
contains one or more duplicate QSOs will be recognized by the LoTW Server as containing duplicate
QSOs". It **never** says whether the server rejects, silently ignores or inserts them.

**No idempotency is promised.** Therefore `Unknown` is never auto-retried, and "exactly-once" is
never claimed anywhere in the code or documentation.

The only documented update path is a correction: if callsign, date, time-to-second, band and mode
match, an existing record is replaced when the Station Location changed; otherwise "LoTW will create
a new QSO". "Once submitted to LoTW, a QSO cannot be deleted; it can only be corrected."

### Download API

```
https://lotw.arrl.org/lotwuser/lotwreport.adi?login=<user>&password=<pass>&qso_query=1&…
```

* "Each query is an HTTP \"GET\" string, formed in accordance with standard URL requirements
  (e.g., RFC1738)."
* Required: `login`, `password`, `qso_query=1`. Filters include `qso_qsl`, `qso_qslsince`,
  `qso_qsorxsince`, `qso_owncall`, `qso_callsign`, `qso_mode`, `qso_band`, `qso_dxcc`,
  `qso_startdate/time`, `qso_enddate/time`, `qso_mydetail`, `qso_qsldetail`, `qso_withown`.
* The documentation is **silent** on POST, HTTP Basic, session cookies, API keys and OAuth for this
  endpoint. Cookies are documented only for web-account login; RFC1867 form upload only for
  `https://lotw.arrl.org/lotw/upload`.
* There is **no** documented safer download variant and **no** warning about credentials in URLs.

**SPLogbook decision:** keep `GET` and the query credentials, because changing them would invent a
protocol. Confine the exposure (`SecretUrl`, no redirects, sanitised errors, no logging, no
persistence). Status: `OFFICIAL_ENDPOINT_REQUIRES_QUERY_CREDENTIALS=true`.

### Post-hoc acceptance evidence

* Machine-readable receipt: `qso_qsl=no` plus `APP_LoTW_RXQSO` / `APP_LoTW_LASTQSORX`.
* "Your Activity" is login-gated HTML; `lotw-user-activity.csv` is weekly and gives only the last
  upload per callsign.
* Upload responses are HTML comments (`<!-- .UPL. accepted|rejected -->`).

SPLogbook does **not** use any of these to infer per-record acceptance; a batch status is derived
only from the documented TQSL exit code.

### Official "date sent"

There is none. `QSO_DATE`/`TIME_ON` are the on-air UTC start; `APP_LoTW_RXQSO` is when LoTW
inserted/updated the record; the Activity page shows file processing and signing times. Nothing is
a QSL-sent date, so SPLogbook derives `LOTW_QSLSDATE` from its own durable acceptance instant and
documents that as an interpretation.

---

## 3. eQSL.cc

| Item | Value |
|---|---|
| Upload interface | https://www.eqsl.cc/qslcard/ImportADIF.txt — "EQSL.CC REAL-TIME LOGGING SOFTWARE INTERFACE", revised **01 March 2026** |
| Inbox download | https://www.eqsl.cc/qslcard/DownloadInBox.txt — "EQSL.CC INBOX DOWNLOADING PROCEDURE", revised **12 Oct 2025** |
| Outbox download | https://www.eqsl.cc/qslcard/DownloadADIF.txt — revised **13 Nov 2026** |
| API index | https://www.eqsl.cc/qslcard/programming.cfm — "Logger Interface Info (API)" |
| Accepted ADIF | https://www.eqsl.cc/qslcard/ADIFContentSpecs.cfm — "We accept up to ADIF V3.1.8 logs" |
| Duplicates FAQ | eQSL FAQ topic 1053 |

### Upload (`ImportADIF.cfm`) — unchanged by F-20R

* Endpoint `https://www.EQSL.cc/qslcard/ImportADIF.cfm` ("The URL is case-insensitive").
* "You can transmit the data as an FORM POST, or with the data in the URL". POST is
  `multipart/form-data` with field `Filename`; credentials as form fields `EQSL_USER`/`EQSL_PSWD`
  or as ADIF header tags; the GET variant uses `ADIFData`.
* Response marker `<!-- Reply form eQSL.cc ADIF Real-time Interface -->`; success string
  `Result: x out of y records added` "(where x and y should be equal if all records were accepted)".
* `<EOR>`: "As of 02March2026 any record or file that does not contain at least one `<EOR>` tag will
  terminate the program without any warning message" (the body says 02March2026, the changelog says
  01 Mar 2026 — an internal inconsistency of the source, recorded not resolved).
* Severity model: `Error:` fatal to results, `Error:` per record ("previous records were imported
  OK"), `Warning:` record not imported, `Caution:` imported anyway. **No documented error guarantees
  an empty import.**

### Download — the F-20R decision point

```
https://www.eQSL.cc/qslcard/DownloadInBox.cfm?UserName=<call>&Password=<pass>
```

* Parameters: `UserName`, `Password`, optional `QTHNickname`, `LimitDateLo`/`LimitDateHi`
  (MM/DD/YYYY), `RcvdSince` (YYYYMMDDHHMM, 1900–2078), `ConfirmedOnly`, `UnconfirmedOnly`,
  `HamOnly`, and **`Archive` = 1|0 (omitted = Inbox AND Archive)**.
* Outbox: `DownloadADIF.cfm` with `UserName`, `Password`, `QTHNickname`, `LimitDateLo`,
  `LimitDateHi`, `HamOnly`.
* **POST is not documented** (word-boundary scan of both download documents for
  POST/GET/METHOD/COOKIE/SESSION/LOGIN yields zero hits). Not forbidden — undocumented.
* A login/session flow is **mentioned but undocumented**: "UserName = Callsign (mandatory if not
  logged in; Ignored if already logged in)" and "(Note that if you are already logged in, Username
  and Password will be ignored…)". There is no login URL, cookie name or token lifetime anywhere in
  the API documentation; only the homepage HTML shows `LoginFinish.cfm`, which is not part of the
  API.
* **There is no `DownloadArchive.cfm`** — it returns HTTP 404. `Archive` is a parameter of
  `DownloadInBox.cfm`. Do not invent an archive endpoint.
* **HTTP 200 is not evidence**: an unauthenticated `GET DownloadInBox.cfm` returns **200** with
  "You are not yet logged in". Success requires the body string "Your ADIF log file has been built";
  the outbox adds "You have no log entries".
* Never construct the downloaded-file URL: "We moved the DownloadedFiles folder … Any external
  programs that had hardcoded the folder structure might not find the files!", and the links expire
  after "a few hours".
* Rate limits: **none documented**. Only a URL-length caution, field caps and a download cap
  ("100,000 eQSLs").
* Credential warning exists on the **upload** side only ("PLEASE USE https instead of http, since
  you will be sending the user's password in plain text!", added 19 Feb 2026). The **download** side
  contains no warning of any kind.

**SPLogbook decision:** keep `GET` and the query credentials. Confine the exposure exactly as for
LoTW. Status: `OFFICIAL_ENDPOINT_REQUIRES_QUERY_CREDENTIALS=true`.

### Duplicates

FAQ 1053: a record matching Callsigns, Dates, Times (down to hours and minutes), Band and Mode
identically with one already in the Outbox "will be rejected as a duplicate and will not be imported
into the database"; `ImportADIF` returns `Warning: … Bad record: Duplicate`. The Foreground Uploader
"may allow some duplicates if your logger has output duplicates within the ADIF file itself".

### Post-hoc result check

Only the OutBox download and `DisplayLastUploadDate.cfm` (last-upload timestamp). There is no
upload-history endpoint, so no invented endpoint is added.

---

## 4. Sources that are stale, and why this matters

| Source | Stale claim | Correct current fact |
|---|---|---|
| https://lotw.arrl.org/lotw-help/cmdline?lang=en | exit codes 0–11 | TQSL 2.8.7 documents 0–15 |
| https://lotw.arrl.org/lotw-help/installation/ | "current version of TQSL is 2.7.2" | 2.8.7 per https://www.arrl.org/tqsl |
| LoTW website command-line page | `-a new` is valid | 2.8.7 accepts only `ask|abort|compliant|all` |

Rule for future gates: when a service's own help pages disagree with the shipped release notes or
with the shipped binary's documentation, the **newest document** wins, and both are recorded here.

---

## 5. What is deliberately still unverified

* No real LoTW or eQSL submission, download or login was executed.
* Server-side duplicate behaviour (reject / ignore / duplicate) is **undocumented** and therefore
  **unverified**; the code is fail-closed around it.
* Real acceptance timing, queueing delays and rate limiting are unverified.
* The TQSL certificate store was never touched; no certificate exists in any test fixture.
