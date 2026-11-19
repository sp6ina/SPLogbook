# ADIF award-reference semantics (SOTA and POTA)

Durable technical contract for the role-aware SOTA/POTA reference mapping (`ADR-103`–`ADR-112`).
It records what the program writes, what it reads, and why — with the
official source of every rule.

ADIF version: **3.1.8** (`src/core/adif.rs`, `ADIF_VERSION`), the current released specification
("Released ADIF Version 3.1.8, updated 2026-10-12", <https://www.adif.org/318/ADIF_318.htm>).

## 1. The one rule

`MY_*` always means **this station** (the station doing the logging). The un-prefixed partner always
means the **contacted station**. The ADIF 3.1.8 QSO-field table states it per field:

| Field | Official description (ADIF 3.1.8 §III.C.1.b) |
|---|---|
| `MY_SOTA_REF` | "the logging station's International SOTA Reference." |
| `SOTA_REF` | "the contacted station's International SOTA Reference." |
| `MY_POTA_REF` | "a comma-delimited list of one or more of the logging station's POTA (Parks on the Air) reference(s)." |
| `POTA_REF` | "a comma-delimited list of one or more of the contacted station's POTA (Parks on the Air) reference(s)." |
| `MY_SIG` / `MY_SIG_INFO` | special-interest activity / information of the logging station |
| `SIG` / `SIG_INFO` | "the name of the contacted station's special activity or interest group" / "information associated with the contacted station's activity or interest group" |
| `STATION_CALLSIGN`, `OPERATOR` | own station / own operator (no `MY_` prefix) |
| `CALL` | "the contacted station's callsign" |

`MY_` therefore means "my station", **not** "my contact". Roles are never inferred from the mere
presence of a field, from a GUI label, from column order or from a variable name.

## 2. Model

`src/core/award_refs.rs` is the only module that knows the sides:

```text
StationSide  { Local, Remote }          // Local = SPLogbook station, Remote = correspondent
AwardProgram { Sota, Pota }
AwardRefPair { local: Option<String>, remote: Option<String> }
AwardReferences { sota: AwardRefPair, pota: AwardRefPair }
PotaRepresentation { Absent, Sig, DirectFields }
```

- `local` is **always** the SPLogbook station; `remote` is **always** the correspondent.
- A missing reference is an explicit `None`; an empty string is normalised to `None`.
- QSO storage keeps one column per role (`my_sota_ref`, `sota_ref`, `my_pota_ref`, `pota_ref`), so no
  database migration is required and no historical row is rewritten.
- The reference is **not** part of `QsoIdentity`; duplicate detection is unchanged.

## 3. SOTA mapping

| Model | ADIF export | ADIF import |
|---|---|---|
| `sota.local` | `MY_SOTA_REF` | `MY_SOTA_REF` |
| `sota.remote` | `SOTA_REF` | `SOTA_REF` |
| `None` | field omitted | field absent → `None` |

- SOTA activator: own summit → `MY_SOTA_REF`; no correspondent summit → no `SOTA_REF`.
- SOTA chaser: no local activation reference → no `MY_SOTA_REF`; the chased summit → `SOTA_REF`.
- Summit-to-summit: both fields, with **different** roles; neither side is copied into the other.
- `SOTARef` is single-valued (ADIF defines no SOTA list type); a comma is a malformed value, kept
  verbatim and only reported by the existing soft-format warning.
- A summit is never carried in `MY_SIG_INFO`/`SIG_INFO`. SOTA's own ADIF importer uses `MY_SOTA_REF`
  and `SOTA_REF` for summit identity and reads the SIG pair only as a location hint.

SOTA CSV V2 (`export_sota_csv`) follows the same sides: column 3 `[My Summit]` is the record's
`my_sota_ref` (falling back to the activation-context value passed by the export dialog only when the
record has none), and column 9 `[His Summit]` is `sota_ref`. A chaser row leaves column 3 empty.

## 4. POTA mapping

ADIF 3.1.8 defines `MY_POTA_REF`/`POTA_REF` as `POTARefList` (since ADIF 3.1.4). POTA's own
documentation does **not** mention those fields; it documents the SIG pair and states that only its
documented fields are recognised:

> "MY_SIG: Special Interest Group. Insert "POTA" in this field."
> "MY_SIG_INFO: Specify the POTA park reference number for the activator's park …"
> "SIG: Special Interest Group. Specify "POTA" in this field."
> "SIG_INFO: The park reference number of the other activator's park."
> — <https://docs.pota.app/docs/activator_reference/ADIF_for_POTA_reference.html>

> "DO NOT USE Any other field names! … The ONLY fields recognized are those described above."
> — <https://docs.pota.app/docs/activator_reference/logging_made_easy.html>

### 4.1 Production export contract

| Model | ADIF export | ADIF import accepted |
|---|---|---|
| `pota.local = Some(r)` | `MY_SIG=POTA`, `MY_SIG_INFO=r` | `MY_SIG`+`MY_SIG_INFO` (`MY_SIG` = POTA, case-insensitive) and/or `MY_POTA_REF` |
| `pota.remote = Some(r)` | `SIG=POTA`, `SIG_INFO=r` | `SIG`+`SIG_INFO` (`SIG` = POTA) and/or `POTA_REF` |
| `None` | no SIG pair for that side | absent → `None` |

Invariants:

- `MY_SIG` is never emitted without `MY_SIG_INFO`, and `SIG` never without `SIG_INFO`.
- `SIG=POTA` is never emitted without a remote reference, `MY_SIG=POTA` never without a local one.
- A record never carries both representations; exactly one is chosen per record.

### 4.2 Multiple parks (`POTARefList`)

POTA documents one reference per field and represents multi-park activations by separate log files
and by repeating the QSO row per reference — it documents no list separator. ADIF, in contrast,
defines `POTARefList` as a comma-delimited list. Therefore:

- a record with a comma list on either POTA side is exported with the **direct fields**
  (`MY_POTA_REF`/`POTA_REF`), the only lossless, spec-defined carrier;
- such a record is **not** claimed to be uploadable to POTA as one file, and the program emits no
  SIG pair for it (that would be an invalid value for POTA);
- a record with one reference per side uses the SIG pair only.

The choice is a deterministic function of the record, so `export → import → export` is a fixed point.

### 4.3 Conflicting representations on import

| Input | Result |
|---|---|
| only a direct field | value taken |
| only a SIG pair | value taken from `_INFO` |
| both, equal (case-insensitive, list order ignored) | merged, no conflict, no loss |
| both, different | record **rejected**: `AdifRecordError::ConflictingPotaRepresentation`, message naming both values and the side |
| `SIG` value that is not `POTA` | not interpreted as POTA; reported as `AwardRefNotice::UnknownSig` |
| `SIG_INFO` without `SIG` | not interpreted; reported as `AwardRefNotice::ReferenceWithoutSig` |
| `SIG=POTA` without `SIG_INFO` | no reference; reported as `AwardRefNotice::PotaSigWithoutReference` |

Rejection is per record; the rest of the file still imports (the existing per-record contract). The
importer never chooses one value arbitrarily and never swaps sides. Notices are returned in
`AdifImportResult::reference_notices` and are therefore visible to every caller instead of being
dropped silently.

## 5. Where the sides come from

| Source | Side | Behaviour |
|---|---|---|
| Station profile (`sota_ref`, `pota_ref`) | **local** | pre-fills the form's `my_sota`/`my_pota`; describes the current activation |
| Activation context after saving a QSO | **local** | `my_sota`/`my_pota` survive the per-QSO form reset |
| Cluster spot of another activator | **remote** | fills `entry.sota`/`entry.pota` only |
| Self-spot / own activation | **local** | belongs to `my_*`; a spot never overwrites `my_*` |
| QSO form's "SOTA Summit"/"POTA Park" | **remote** | saved into `sota_ref`/`pota_ref` |
| QSO edit dialog | both | does not expose reference fields, so both sides pass through unchanged |
| Script/plugin `qso_field` | both | `sota_ref`/`pota_ref` = remote, `my_sota_ref`/`my_pota_ref` = local |

A spot's reference is the *spotted* station's reference. The program never transfers a spot value to
the local side and never copies a local value to the remote side.

## 6. Case and validation

- ADIF declares `POTARef` case-insensitive and gives no case rule for `SOTARef`; POTA's docs give no
  case rule either. The importer therefore **preserves the value verbatim** (trim only), so no case is
  guessed. Equality checks are case-insensitive.
- SOTA CSV export upper-cases references because the SOTA database validator requires upper case
  (`^[A-Z0-9]+/[A-Z][A-Z]-[0-9][0-9][0-9]$`); that is a CSV formatting rule, not an ADIF rule.
- Soft format warnings (`QsoRecord::warnings`) now cover **both** sides. They never block a save.
- The program validates **syntax only**. It does not claim that a reference exists:
  `syntactically valid` ≠ `externally verified`. No online lookup is performed by this mapping and no
  data is uploaded to SOTA or POTA.

## 7. ADI length and encoding (unchanged by F-19)

The writer emits `<TAG:<byte length>>` and the parser accepts byte lengths with a documented
character-count compatibility fallback. ADIF 3.1.8 defines `Character` as ASCII 32–126 and forbids
`Intl*` types in ADI, so for the ASCII data that ADIF permits a character is one byte. Empty values
are omitted (ADIF permits omission; it does not require it).

## 8. Non-goals

- No historical record is rewritten, migrated or repaired automatically.
- No schema change: each role already has its own nullable column.
- No generic `SIG`/`MY_SIG` storage is added; an uninterpretable SIG value is reported, not guessed.
- WWFF/IOTA/PGA/`GRIDSQUARE`/`STATE` handling is unchanged.

## 9. Official sources

| Source | URL |
|---|---|
| ADIF 3.1.8 (current released specification) | <https://www.adif.org/318/ADIF_318.htm> |
| ADIF index / version list | <https://www.adif.org/> |
| POTA — ADIF for POTA, technical reference (`MY_SIG`, `MY_SIG_INFO`, `SIG`, `SIG_INFO`) | <https://docs.pota.app/docs/activator_reference/ADIF_for_POTA_reference.html> |
| POTA — Logging Made Easy ("DO NOT USE Any other field names") | <https://docs.pota.app/docs/activator_reference/logging_made_easy.html> |
| POTA — Rules (activator, hunter, park-to-park, optional field list) | <https://docs.pota.app/docs/rules.html> |
| POTA — Park-to-Park guide (`SIG_INFO`, multi-park rows) | <https://docs.pota.app/docs/activator_reference/park_2_park.html> |
| SOTA — General Rules v1.21 (activator, chaser, reference system) | <https://storage.sota.org.uk/docs/SOTA-General-Rules-June-2022.pdf> |
| SOTA — Joining In | <https://www.sota.org.uk/Joining-In/General-Rules> |
| SOTA — Awards incl. Summit-to-Summit | <https://www.sota.org.uk/Joining-In/Awards> |
| SOTA Database (upload help is behind SOTA SSO) | <https://www.sotadata.org.uk/en/upload> |

The SOTA CSV V2 column semantics and the SOTA ADIF importer's `MY_SOTA_REF`/`SOTA_REF` usage were
read from SOTA's own Sotadata3 application served by `www.sotadata.org.uk`; that help text is not
available as an anonymous public page, which is recorded as a documentation-access limitation rather
than replaced by a secondary source.
