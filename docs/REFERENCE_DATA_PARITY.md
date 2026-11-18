# SPLogbook — Reference data inventory and functional parity

Status: **R1B CLOSED (R1B2 completed, D-R1B-FINAL §9.6) — every §7 row resolved; bundled files
still present until R1C.**
R1B measured on 2026-11-17, R1B2 on 2026-11-18. Earlier inventory (R1A) against `c326572`.

A bundled data file may only be removed once every feature it serves is
`FUNCTIONAL_PARITY_VERIFIED`. A feature without a complete replacement stays
`BLOCKED_MISSING_REPLACEMENT` and its source stays in the tree. Allowed statuses:
`FUNCTIONAL_PARITY_VERIFIED`, `BLOCKED_MISSING_REPLACEMENT`, `OWNER_DECISION_REQUIRED`,
`NOT_APPLICABLE`.

## 1. Bundled data files (unchanged in R1B)

| File | Content | Read by |
|---|---|---|
| `databases/serviceLOG.db` | 11 tables, see §2 | `core::prefix` (legacy layer), `core::service_db`, `core::callbook` |
| `databases/callbook.db` | `Callbook` (51 955 rows), `inform` | `core::callbook::LocalCallbook` |
| `databases/clubs.csv` | 17 club memberships | `core::clubs::ClubRegistry` (transitional source) |

## 2. `serviceLOG.db` table map

| Table | Rows | Used by | Replacement |
|---|---|---|---|
| `UniqueCalls` | 4 310 | exact-call exceptions, prefix browser | `=CALL` entries of the country files; user overrides |
| `CountryDataEx` | 453 | entity, zones, continent, coordinates by prefix | country files (`cty.dat` + `cty.csv`) |
| `Province` | 623 | sub-national zones, coordinates, region name | user regional rules (§5.2); no public source identified |
| `managers` | 29 102 | QSL manager lookup and browser | user table `qsl-managers` |
| `IOTA` | 1 190 | IOTA browser and validation | user table `iota` (+ built-in 30-entry sample) |
| `STATE` | 3 545 | states / subdivisions browser | user table `states` (+ built-in US list) |
| `Modes`, `Bands`, `PropMode`, `contest`, `Satellite` | — | not read by code | `NOT_APPLICABLE` |

## 3. External sources

### 3.1 Contest country files (`cty.dat`, `cty.csv`)

| Item | Finding |
|---|---|
| Data owner | Country Files project (`www.country-files.com`) |
| Format documentation | `cty.dat`: <https://www.country-files.com/cty-dat-format/>. `cty.csv`: no published description; it ships inside the release archives and carries the same entities in the same order with the ADIF DXCC entity code as third column |
| Terms | <https://www.country-files.com/copyright/> and `copyright.txt` inside every archive: MIT-style permission to use, copy, modify, merge, publish, distribute, sublicense; the notice must be kept in copies |
| Redistribution | permitted by those terms; **not used**: SPLogbook ships no country files (decision D1) |
| Download | user-triggered only. Default: `https://www.country-files.com/bigcty/download/bigcty.zip` ("Big CTY", complete zone overrides for everyday logging). This URL is not linked from the site pages; on 2026-11-17 it was byte-identical (SHA-256) to the dated release `bigcty-20261114.zip` linked from the Big CTY page |
| Update cadence | several releases per month; release marker `VERyyyymmdd` inside `cty.dat` |
| Limits | one entity per callsign; the first occurrence wins; coordinates are entity centres unless an alias carries `<lat/lon>` |

### 3.2 ADIF specification (entity names, decision E3)

The current specification is ADIF 3.1.8 (updated 2026-10-12, <https://adif.org/318/ADIF_318.htm>).
The document contains no licence or redistribution statement. Canonical ADIF entity names are
therefore **not** embedded; see §7.

## 4. File formats introduced in R1B

All formats are UTF-8 CSV (RFC 4180 quoting), start with a format marker line, allow `#`
comments, identify columns by header name and are validated **as a whole**: one invalid row
rejects the file and the installed data stays in use. Exact duplicates are kept once with a
warning; same key with different content is a conflict.

| Data | Marker | Columns (required in bold) | Code |
|---|---|---|---|
| regional rules, user overrides | `#splogbook-rules,1` | **`rule_type`** (`exact`, `prefix`, `pattern`), **`callsign_or_prefix`**, `dxcc_entity_code`, `subdivision_code`, `subdivision_name`, `cq_zone`, `itu_zone`, `continent`, `latitude`, `longitude`, `priority`, `source`, `source_version`, `source_date`, `notes` | `core::reference::rules` |
| IOTA | `#splogbook-iota,1` | **`reference`**, **`name`**, `prefix` | `core::reference::tables` |
| states / subdivisions | `#splogbook-states,1` | **`code`**, **`name`**, **`country`** | `core::reference::tables` |
| QSL managers | `#splogbook-qsl-managers,1` | **`callsign`**, **`manager`**, `years`, `notes` | `core::reference::tables` |
| local callbook | `#splogbook-callbook,1` | **`callsign`**, `name`, `qth`, `grid`, `state`, `manager`, `notes` | `core::reference::tables` |
| clubs | header `callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b` (unchanged format) | all seven | `core::clubs` |

Rules: a rule must set at least one attribute; zones 1–40 / 1–90; coordinates in degrees,
+North/+East, both or neither; priority −1000…1000; `source_date` is `YYYY-MM-DD`. Within one
rule book `exact` (full call, then the call without operational suffixes), then the longest
`prefix`, then `pattern`; equal-priority matching patterns are a reported conflict and none is
applied.

## 5. Storage, installation and resolution

### 5.1 Storage

Everything lives under `<data dir>/reference/` (next to `databases/`, so a portable
installation keeps it beside the executable). Each source is an independent set
(`country-files`, `regional-rules`, `user-overrides`, `iota`, `states`, `qsl-managers`,
`callbook`, `clubs`) with a hashed `manifest.json`. Installation stages and verifies the files,
lets the caller validate them, then swaps `current → .prev` and `staging → current`; an
interrupted swap is completed or rolled back on the next access. Updating country files never
touches user rules or tables.

### 5.2 Prefix resolution layers (decisions E2, E4)

1. explicit user override;
2. exact call from the country files (also without `/P`, `/M`, `/QRP`…);
3. exact user regional rule;
4. longest country-file prefix, refined by a user regional rule of the same entity, or during
   the transition by a legacy `Province` rule of the same entity;
5. legacy database as compatibility layer when the country files give nothing;
6. a bare user rule with an entity code;
7. no result.

A regional rule naming a different entity than the country files is reported
(`LayerConflict::EntityMismatch`) and not applied.

### 5.3 Source precision

Every `PrefixInfo` carries `location_precision`: `ExactQsoLocation` (set only by the QSO layer),
`UserOverride`, `UserRegionalRule`, `LegacyRegionalRule`, `CountryFileOverride`,
`EntityCentroid`, `LegacyEntity`, `Unknown`. A rule that sets no coordinates never raises the
declared precision.

### 5.4 User flows

References menu: status of every source (not installed / version and date / cannot be loaded);
download current Big CTY after confirming the source; import country files from a ZIP or from
`cty.dat` + `cty.csv`; restore previous country files; import and export regional rules, user
overrides, IOTA, states, QSL managers and the local callbook; club membership editor (filter,
add, edit, delete, save, import, export, reload). Every success reloads the shared data without a
restart; every failure keeps the installed data and shows a translated message. Browsers show a
neutral hint when no data is installed. Nothing is downloaded automatically.

### 5.5 Archive safety (decision E5)

`zip` 9.6.1 (MIT, MSRV 1.88), default features off, only `deflate-flate2`. The archive is read in
memory: archive ≤ 16 MiB, ≤ 256 entries, each wanted entry ≤ 16 MiB with the declared size never
trusted (read stops one byte past the limit); absolute, drive, `..` and control-character names
reject the whole archive; duplicated wanted entries, encryption and methods other than Stored and
Deflate are rejected. The pair is installed only if every `cty.dat` entity has an entity code in
`cty.csv`. The download is bounded in memory independently of `Content-Length`.

## 6. Differential comparison (Level C)

Inputs: 64 524 calls — every legacy exact call, every country-file exact call, generated calls for
every prefix of both sources, and `/P`, `/MM`, guest-prefix and `/9` variants. Country files:
Big CTY of 2026-11-14 (346 entities, 6 331 prefixes, 23 155 exact calls, all entity codes
matched). The comparison runs locally against the bundled database; no record of it is stored in
the repository.

### 6.1 Legacy defect corrected first (E1)

203 of 941 active legacy prefix rules (49 entity rules, among them United States, England, Italy,
Japan, Spain, France, European Russia, Australia, Argentina and Brazil, and 154 regional rules)
used the end anchor `\Z`, which the regex crate rejects, and were silently dropped. After the
fix all 347 entity rules and 594 regional rules compile; the fix alone changes 21 927 of the
64 524 results (`LEGACY_DEFECT_CORRECTED`). One unreadable `UniqueCalls` row is now reported.

### 6.2 Fixed legacy versus country files

| Class | Inputs | Status |
|---|---|---|
| identical | 12 027 | — |
| resolved only by the country files (prefix) | 2 262 | `EXPECTED_SOURCE_IMPROVEMENT` |
| resolved only by a country-file exact entry | 3 951 | `EXPECTED_SOURCE_IMPROVEMENT` |
| different result from a country-file exact entry | 21 593 | `EXPECTED_SOURCE_IMPROVEMENT` |
| entity centre differs, same zones | 16 353 | `SOURCE_DATA_DIFFERENCE` (most legacy coordinates are regional) |
| entity-level zone/continent differs | 6 222 | `SOURCE_DATA_DIFFERENCE` / `REGIONAL_DATA_NOT_AVAILABLE` |
| legacy-only exact data differs | 1 683 | `CUSTOM_OVERRIDE_REQUIRED` |
| different entity for a prefix (37 groups) | 252 | `SOURCE_DATA_DIFFERENCE` → `OWNER_DECISION_REQUIRED` |
| legacy-only result (43 groups) | 181 | `OWNER_DECISION_REQUIRED` |

Entity groups (C3), legacy → country files: South Shetland Is. → Antarctica (LU1–LU8 Z),
Argentina regional → Antarctica (AY1–AY8 Z), Antarctica → United States (KC4), S. Cook Is. → New
Zealand (ZK1), Antarctica → Chile (CA9–CD9, XQ9, 3G9), Bouvet → Antarctica (3Y/), Taiwan →
Pratas Island (BU9/BW9), Antarctica → South Shetland Islands (CE9, XR9), USA → Guantanamo Bay
(KG4), New Zealand → Antarctica (ZM5), Swains Is. → American Samoa (KH8), Alaska → Republic of
Korea (KL9), France → Wallis & Futuna (TW), Heard Is. → Antarctica (VK0), Russian regions →
European Russia / Kaliningrad / Franz Josef Land, Macquarie Is. → Antarctica, Lord Howe Is. →
Norfolk Island, ITU HQ → Italy, Antarctic stations → their national entity, Sardinia → Italy,
East/West Malaysia, Southern Sudan → Sudan (ST0), Sudan → Egypt (SS0), New Caledonia → France
(TX8), Christmas Is. → Norfolk Island, NZ Sub-Antarctic Is. → New Zealand, Conway Reef / Rotuma
→ Fiji, Austria → Italy (4U).

Legacy-only groups (C4): former colonial prefixes mapped to England (VQ1, VQ6, VQ8, VS2, VS4,
VS5, ZD4, ZE1) — `INVALID_LEGACY_RULE`; VS9, ZC5, ZC6, JZ0, VO0, VO9 — `OBSOLETE_PREFIX`; deleted
entities (St. Maarten, Desroches) — `HISTORICAL_ENTITY`; unofficial prefixes (1B, 1S, 1C,
regional political entities) and legacy-only exact calls — `USER_OVERRIDE_CANDIDATE`; England
`2O…`, Scarborough Reef `BS7…`, St. Kitts `VP2`, Minami Torishima `JD/M` — `OWNER_DECISION_REQUIRED`.

### 6.3 User tables

`core::service_db` and `core::callbook` differential tests serve the same synthetic rows from
the legacy SQLite layout and from imported user tables and require identical results for every
query (case, `%` and `_`, ordering, 100-row limit, base-call fallback, manager merge).

## 7. Functional parity matrix

Columns: FUNCTION · OLD_SOURCE · NEW_SOURCE · FLOW_BEFORE · FLOW_AFTER · OFFLINE_BEFORE ·
OFFLINE_AFTER · TEST_BEFORE · TEST_AFTER · DIFFERENCES · PARITY_STATUS · RESIDUAL_RISK.

| Function | Old source | New source | Flow before | Flow after | Offline before | Offline after | Test before | Test after | Differences | Status | Residual risk |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Local callbook | `callbook.db` | user table `callbook` | data out of the box | import, then same lookups | yes | yes after import | real-record test | unit, integration, differential | none on identical data | FUNCTIONAL_PARITY_VERIFIED | user must supply data |
| QRZ | network | unchanged | unchanged | unchanged | no | no | existing | existing | none | NOT_APPLICABLE | — |
| HamQTH | network | unchanged | unchanged | unchanged | no | no | existing | existing | none | NOT_APPLICABLE | — |
| Callbook cache | user cache | unchanged | unchanged | unchanged | yes | yes | existing | existing | none | NOT_APPLICABLE | — |
| DXCC entity | `CountryDataEx`, `UniqueCalls` | country files | out of the box | download or import | yes | yes after install | synthetic | unit, corpus, differential | §9.4: 33 C3 / 22 C4 groups, all classified (D-C3, D-C4, D-ENT, I6) | FUNCTIONAL_PARITY_VERIFIED | country files decide (D-C3); legacy-only rules retired with R1C (§9.6) |
| ADIF entity code | same | `cty.csv` | same | same | yes | yes | synthetic | unit, consistency check, ADIF 3.1.8 code check (I4) | none at entity level | FUNCTIONAL_PARITY_VERIFIED | follows D-C3 |
| CQ zone | legacy incl. `Province` | country files + regional rules | same | same | yes | yes | synthetic | unit, differential | legacy regional zones are a call-area heuristic (D-R1), retired with R1C (§9.6) | FUNCTIONAL_PARITY_VERIFIED | Big CTY zone overrides; user rules for regional detail |
| ITU zone | same | same | same | same | yes | yes | synthetic | unit, differential | same; D-ENT corrections (South Cook 62, Cocos I. 12) | FUNCTIONAL_PARITY_VERIFIED | same |
| Continent | same | same | same | same | yes | yes | synthetic | unit, differential | D-ENT corrections (Peter 1 I., Antarctica AN) | FUNCTIONAL_PARITY_VERIFIED | same |
| Prefixes | regex rules | longest prefix + rules | same | same | yes | yes | synthetic | corpus, differential | §9.4 (I6 implicit primaries) | FUNCTIONAL_PARITY_VERIFIED | as DXCC |
| Exact calls | `UniqueCalls` | `=CALL` + overrides | same | same | yes | yes | synthetic | unit, differential | 1 087 legacy-only: 597 redundant, 490 retired (D-LO, §9.6) | FUNCTIONAL_PARITY_VERIFIED | retired calls resolve by prefix after R1C |
| Coordinates | legacy regional | entity centre / regional rules | regional | entity unless rule | yes | yes | none | unit, precision tests | entity centre unless a rule gives coordinates (D-R1B-FINAL) | FUNCTIONAL_PARITY_VERIFIED | precision is reported, not hidden |
| Province / state / oblast level | `Province` | user regional rules | out of the box | import only | yes | yes after import | none | unit, layer tests (I3) | no authoritative prefix→region mapping exists (§9.1); legacy heuristic not exported (D-R1, D-R1B-FINAL) | FUNCTIONAL_PARITY_VERIFIED | regional names only from user rules after R1C |
| Prefix browser | `UniqueCalls` | country-file exceptions | out of the box | after install | yes | yes | none | integration | country-file exceptions (D-C3) | FUNCTIONAL_PARITY_VERIFIED | follows D-C3 |
| IOTA | `IOTA` | user table `iota` | 1 190 groups | import | yes | yes after import | none | unit, integration, differential | none on identical data | FUNCTIONAL_PARITY_VERIFIED | user must supply data |
| States / subdivisions | `STATE` | user table `states` | 3 545 rows | import (+ US list) | yes | yes after import | none | unit, integration, differential | none on identical data | FUNCTIONAL_PARITY_VERIFIED | same |
| QSL managers | `managers` | user table `qsl-managers` | 29 102 rows | import | yes | yes after import | none | unit, integration, differential | none on identical data | FUNCTIONAL_PARITY_VERIFIED | same |
| Clubs | `clubs.csv` + built-in | user clubs file | file or built-in | import or editor | yes | yes | partly real | unit, store tests | none | FUNCTIONAL_PARITY_VERIFIED | built-in seed removed in R1C |
| SCP | seed + log + `MASTER.SCP` | unchanged | unchanged | unchanged | yes | yes | existing | existing | seed removal in R1C | NOT_APPLICABLE | — |
| Modes, bands, contests, satellites | static code | unchanged | unchanged | unchanged | yes | yes | existing | existing | none | NOT_APPLICABLE | — |
| Import | fixed file paths | per-source import actions | copy files | menu actions | — | — | none | unit, integration | added, nothing removed | FUNCTIONAL_PARITY_VERIFIED | — |
| Update | `cty.dat` written to CWD, unused | Big CTY install + reload | unused download | validated install | — | — | existing | unit, archive tests | old action kept | FUNCTIONAL_PARITY_VERIFIED | old action still writes to CWD (R1D) |
| Offline operation | bundled files | installed user data | full | full after install | yes | yes | none | loader tests | Big CTY + user tables work offline once installed; first run has no data (R1D) | FUNCTIONAL_PARITY_VERIFIED | follows rows above |
| Clean first run | — | — | — | — | — | — | — | — | — | NOT_APPLICABLE (R1D) | — |
| Portable | implicit | reference root beside data | — | unchanged | yes | yes | updater tests | root test | none | NOT_APPLICABLE (R1D) | — |
| Installers | ship the three files | unchanged in R1B | seeded | seeded | — | — | installer tests | unchanged | none yet | NOT_APPLICABLE (R1C) | — |
| Updater | first-install seeds | unchanged | preserve | preserve | — | — | existing | existing | none | NOT_APPLICABLE | — |
| API | `PrefixMatcher`, `ClubRegistry` | same, layered | — | — | — | — | server tests | unchanged | follows prefix rows; `province` carries the region name or `null` (I3) | FUNCTIONAL_PARITY_VERIFIED | follows prefix rows |
| GUI | browsers | same windows + import actions + hints | — | — | — | — | — | integration | options added, none removed | FUNCTIONAL_PARITY_VERIFIED | — |

Regional detail: entity-level fallback — implemented, precision `EntityCentroid`;
province-level and state-level data — user regional rules (`#splogbook-rules,1`), the legacy heuristic
is retired with R1C (D-R1B-FINAL, §9.6); exact-call overrides — mechanism implemented, legacy-only
data `RETIRED_UNLICENSED_LEGACY_DATA` (§9.6);
source precision — reported on every result; coordinate precision — never raised without
coordinates.

## 8. Open decisions

All decided (2026-11-18): C3/C4 acceptance, regional data, legacy-only exact calls and entity
attributes — see §9.2 and §9.6. The items below are kept as recorded by R1B.

* **C3 / C4 acceptance** — confirm the country files as the deciding source for the 37 entity
  groups, and the proposed handling of the 43 legacy-only groups.
* **Regional data source** — the province, state and oblast rules (`Province`) have no identified
  public source with clear terms. Options: the owner supplies a regional rules file under the
  documented format; an approved external source is evaluated; `serviceLOG.db` stays as the
  compatibility layer.
* **Legacy-only exact calls** — 1 683 inputs depend on callsign data that exists only in the
  bundled database; they can be preserved only as user overrides supplied by the owner.
* **ADIF entity names (E3)** — the ADIF specification carries no redistribution statement, so
  canonical names are not embedded. Stored COUNTRY values are never rewritten; new QSOs get the
  country-file name and the entity code.

R1C may start only when no feature that depends on a bundled file is
`BLOCKED_MISSING_REPLACEMENT`.

## 9. R1B2 — C3/C4, regional data and legacy-only calls (2026-11-18)

### 9.1 Source register

All sources were opened and read between 2026-11-18T05:14Z and 05:23Z. SHA-256 values were
computed locally; none of the publishers below publishes a checksum or signature.

| Source | Owner | URL | Version / date | SHA-256 | Licence / redistribution | Use |
|---|---|---|---|---|---|---|
| Big CTY | Country Files project (AD1C) | <https://www.country-files.com/bigcty/download/2026/bigcty-20261114.zip> (release post <https://www.country-files.com/big-cty-14-november-2026/>) | released 2026-11-14; 331 142 B; undated `bigcty.zip` byte-identical on 2026-11-18 | archive `c899ffc4461998b4688945b8413dc2da530f525269d8a408f8011539d5e47bc3`; `cty.dat` `edeb0c8e81e0671d7c48e87e512705781ff2e9a7965d1202508d587d5820fac1`; `cty.csv` `5a5f13a673a2d42ffcd1d9eb298e6aa773463a3a224a6e67ad1f924bc7cf32bc` | MIT-style (<https://www.country-files.com/copyright/>); not shipped (D1) | replacement source: entities, zones, exact calls |
| `cty.dat` format | same | <https://www.country-files.com/cty-dat-format/> | read 2026-11-18 | — | — | "Alias DXCC prefixes (including the primary one) follow" |
| ARRL DXCC List | ARRL | <https://www.arrl.org/country-lists-prefixes>, <https://www.arrl.org/files/file/DXCC/Current_Deleted.txt> | "January 2026"; 340 current, 62 deleted | `892c719f2fbb66f27a2c7ca023518718c4f0599c7aa66f60364b916e3d5dc26d` | personal use only; ARRL permission needed otherwise → **not shipped**, used as a cross-check | official entity status and attributes |
| ADIF 3.1.8 | ADIF Development Group | <https://adif.org/318/ADIF_318.htm>; exports <https://adif.org.uk/318/resources> (`ADIF_318_resources_2026_10_12.zip`) | 3.1.8, updated 2026-10-12 | spec `9d130f558125fbbf4d1da93c4fb8d63dda859914ffeb54416627f093afcf7fe6`; exports `d211e1d41adaf7fc341c73a8a3a56c343534aac57043b7474f7df2d528e56d8c` | no licence statement → names not embedded (E3); the test suite holds the entity **codes** only | entity codes, deleted flags, region lists (no callsign mapping) |
| ITU Appendix 42 RR | ITU-R | <https://www.itu.int/en/ITU-R/terrestrial/fmd/Pages/call_sign_series.aspx> (table <https://www.itu.int/gladapp/Allocation/CallSigns>) | read 2026-11-18 | `1633ea603597a3e477fa38df51b3427c93089da592bf1852ca5cae735805cd00` | all rights reserved → reference only | whether a prefix is allocated |
| Poland | Minister Cyfryzacji | Dz.U. 2026 poz. 406, <https://eli.gov.pl/api/acts/DU/2026/406/text/I/D20260406.pdf> | 19 March 2026 | `cff94713a772447aa08337e3ad8e320d410a49d2bdde409a16e42b4ae89fcf1c` | public law | §21–22: the digit has no regional meaning |
| Canada | ISED | RIC-9, <https://ised-isde.canada.ca/site/spectrum-management-telecommunications/en/node/314> | Issue 2 (Provisional), October 2005; page modified 2016-04-25 | `e0961d6c6099a482acd2f245d1bc3ac3f8cf10fc842a883313aa25866c226c3a` | public | prefix = province of the address at issuance; calls are for life; VE9 = New Brunswick |
| USA | FCC | 47 CFR 97.17(d), <https://www.ecfr.gov/current/title-47/chapter-I/subchapter-D/part-97/subpart-A> | current eCFR | — | public | "selected by the sequential call sign system"; the FCC page on that system returned HTTP 403 |

### 9.2 Owner decisions

| ID | Decision |
|---|---|
| D-R1 | Legacy regional rules are `CALL_AREA_HEURISTIC_DEFAULT`: a traditional call-area heuristic and fallback, **not** an authoritative address register (Dz.U. 2026 poz. 406 §21–22, RIC-9). They stay in R1B2, with the region name read correctly (I3). Known differences: legacy `VE1` = "N. Brunswick, Nova Scotia", while RIC-9 gives VE1 = Nova Scotia and VE9 = New Brunswick; 225 rules in 33 entities have no region list in ADIF 3.1.8. |
| D-C3 | Big CTY 2026-11-14 decides over the legacy database for all prefix-versus-prefix entity conflicts, except for the entity attributes of D-ENT. |
| D-C4 | `1S`, `BS7`, `TN/`, `TU/`: resolved by I6. `VQ3…ZQ3` → England, `/ZC6` → UK SBA Cyprus, `VP2`/`VP2K` → St. Kitts & Nevis, `3D6` → Fiji / Conway Reef, `VP2` → Anguilla: `INVALID_LEGACY_RULE`. `/1`, `/CHE`, `JD/`, `/2CS`, `/C`, `/JD`, `/T`, `/D`, `/Z`: `INVALID_UNSCOPED_LEGACY_RULE` (Big CTY supplies the context-specific forms). `1P0`, `T0`, `D1`, `0T6`, `1C0`, `O1`: `REJECTED_UNALLOCATED_ITU_PREFIX` (not in ITU Appendix 42, not an ARRL DXCC prefix). `Y8`/`Y9` → Germany, `ZQ2`/`ZQ3` → Gibraltar, `PJ/` → St. Maarten (deleted): `HISTORICAL_INACTIVE_PREFIX`. |
| D-ENT | For the entity continent and ITU zone, the ARRL DXCC List / ADIF 3.1.8 standard decides over Big CTY: Peter 1 I. (199) continent AN; Antarctica (13) continent AN; South Cook Is. (234) ITU 62; Cocos I. (37, TI9) ITU 12. Code 37 is the entity whose ITU zone conflicts (ARRL 12, Big CTY 11); Cocos (Keeling) Is. (38, VK9C) agrees in both sources (ITU 54) and is not changed. |
| D-LO | Legacy-only exact calls, after I1/I2: `MALFORMED_REJECT` 1 (`R155??P`); `HISTORICAL_DELETED_ENTITY` 36; `HISTORICAL_DATED_OPERATION` 260; `HISTORICAL_SPECIAL_EVENT` 418 (numeral of two or more digits); `HISTORICAL_UNVERIFIED_LEGACY_EXCEPTION` 373 — 1 087 calls in total. The bundled files stay in `databases/` (removal belongs to R1C). |
| D-IMP | I1–I6 approved and implemented (§9.3). |

The C4 classes are recorded classifications. They do not remove rules from the legacy
compatibility layer, which still answers when the country files give nothing, until R1C.

### 9.3 Code changes

| Item | Change | Code |
|---|---|---|
| I1 | `UniqueCalls` keys are trimmed and upper-cased; empty keys and keys with characters other than `A–Z`, `0–9`, `/` are rejected and reported (`LegacyRuleIssueKind::InvalidCallsign`). Nine calls with a trailing space can now be found. | `core::prefix::PrefixMatcher::load_from_db_with_report` |
| I2 | A repeated call keeps the row with the latest known activity date (the later of `StartDate` / `EndDate`, as `YYYY-MM-DD`; a row without a valid date counts as undated); ties go to the higher `rowid` (`_id`). Previously the last row in scan order won regardless of dates. Tables without date columns (older layouts) are still read. | same |
| I3 | The region name is read from `Province.Country` (trimmed; `None` when empty) instead of the always-empty `Comment`, so `PrefixInfo.province` no longer carries `Some("")`. | same |
| I6 | After the whole `cty.dat` is read, the primary prefix of each entity becomes an implicit prefix alias with the entity defaults, unless any entity already lists it explicitly. Not for WAE-only entities (`*`); not for primaries containing `/` (such as `3Y/p`), which a normalised call can never match. Big CTY 2026-11-14: 16 primaries are missing from their alias lists, 15 are added (`CE9` is an explicit alias of South Shetland Is.). | `core::reference::cty::CtyData` |
| D-ENT | `OFFICIAL_ENTITY_ATTRIBUTES` replaces the entity defaults after `cty.csv` codes are joined; explicit per-alias overrides stay as published. Reported in `CountryFileReport::official_attribute_changes`. | `core::reference::cty`, `PrefixMatcher::load_from_country_files` |
| I4 | Deterministic tests for I1, I2, I3, I6, D-ENT, `cty.dat`/`cty.csv` consistency and entity codes checked against the ADIF 3.1.8 code list (`CtyData::entities_with_unknown_code`); synthetic data only. | `core::prefix` tests, `core::reference::cty` tests |

### 9.4 Parity matrix

Method: the production resolver in three configurations (legacy database only; Big CTY
2026-11-14 with `cty.csv`; production layering of country files plus the legacy compatibility
layer) over a fixed corpus of 60 876 calls. The corpus holds every legacy exact call (4 201), every
Big CTY exact call (23 155), and one synthetic call under every Big CTY prefix and every legacy
seed prefix (`ARRLPrefix`, `Province.Prefix`), with `/P`, `/9` and the guest form `PFX/SP1AA`, plus
`/P` and `/MM` for every legacy call. The corpus is sorted and has no random part. Two runs on the
same code gave byte-identical output. Before = `68fdd92`; after = I1–I3, I6 and D-ENT. The
comparison runs locally; nothing of it is stored in the repository.

| Class | Before | After |
|---|---|---|
| identical entity, zones, continent | 27 326 | 27 505 |
| same entity, zones differ | 9 877 | 9 812 |
| same entity, continent differs | 151 | 67 |
| different entity | 14 617 | 14 600 |
| — C3, prefix against prefix (groups / inputs) | 36 / 493 | 33 / 477 |
| — explained by an exact-call entry | 14 124 | 14 123 |
| legacy only, C4 (groups / inputs) | 26 / 219 | 22 / 210 |
| replacement only | 8 573 | 8 570 |
| neither | 113 | 112 |
| layered result differs from legacy only | 30 313 | 30 228 |

Load counts after: legacy 2 278 exact calls (one rejected as malformed, one row unreadable because
its ITU zone is stored as text), 347 entity rules, 594 regional rules, 203 `\Z` anchors normalised;
Big CTY 346 entities, 6 346 prefixes (6 331 listed + 15 implicit primaries), 23 155 exact calls,
0 issues, every entity matched to a `cty.csv` code.

Effects of the changes:

* **I6.** C4 groups `1S`, `BS7`, `TN/`, `TU/` now resolve. Previous C3 conflicts disappear for
  Heard I., Macquarie I., Lord Howe I., Franz Josef Land, ITU HQ and UN HQ, whose primaries are
  now matched. `IS…` calls outside the listed `IS0` block now resolve to Sardinia instead of Italy.
* **D-ENT.** The "continent differs" class falls from 151 to 67: Antarctica and Peter 1 I. are AN.
  59 South Cook inputs now carry ITU 62 and 8 Cocos I. inputs ITU 12.
* **I1 / I2.** Trimmed keys bring back nine calls; three new single-input C3 groups follow from
  them: `E51AND` North Cook → South Cook, `VK9KS` Territory of New Guinea (deleted) → Norfolk I.,
  `FO8AJ` Clipperton → French Polynesia. I2 changes the result of conflicting dated duplicates:
  for example the entity of `FO/KH0PR`, `FO0MAC`, `FO0PAP`, `JF1LGD/JD1` and `VP8AEN`, and the
  zones of `R66IOTA`.
* **I3.** Every regional result now carries its region name (25 550 legacy and 18 209 layered
  inputs changed only in `province`).
* The remaining C3 groups (33) and C4 groups (22) follow the classes in §9.2.

### 9.5 Residual risks

* Big CTY has no published checksum or signature; integrity rests on TLS, local validation and
  the recorded SHA-256.
* The regional heuristic is not a location: it can be wrong for any station that moved or chose
  its call freely (Poland), and the USA rules could not be checked against the FCC text.
* "Special event" is a format rule (multi-digit numeral); no official register was found.
* The corpus measures rule behaviour, not real log contents.

### 9.6 Owner decision D-R1B-FINAL — closure of R1B (2026-11-18)

1. **No export of unlicensed legacy data.** `databases/serviceLOG.db` has no open licence and
   no documented origin, and SPLogbook is GPL-3.0-or-later. Its 594 regional rules and its
   non-redundant `UniqueCalls` entries are therefore **not** copied into tracked files. They
   are retired together with `serviceLOG.db` in R1C (`RETIRED_UNLICENSED_LEGACY_DATA`).
2. **Legacy-only exact calls.** Of the 1 087 calls that exist only in the legacy table (after
   I1/I2), 597 give exactly the same entity code, CQ zone, ITU zone and continent from Big CTY
   (after I6 and D-ENT) and are redundant. The other 490 are retired:

   | D-LO class | Calls | Redundant | Retired |
   |---|---|---|---|
   | `HISTORICAL_SPECIAL_EVENT` | 418 | 335 | 83 |
   | `HISTORICAL_UNVERIFIED_LEGACY_EXCEPTION` | 373 | 150 | 223 |
   | `HISTORICAL_DATED_OPERATION` | 260 | 112 | 148 |
   | `HISTORICAL_DELETED_ENTITY` | 36 | 0 | 36 |
   | total | 1 087 | 597 | 490 |

   Of the 490, 415 resolve to another entity through Big CTY, 55 to the same entity with other
   zones, and 20 to nothing (among them the prefixes rejected in D-C4 as not allocated by ITU,
   e.g. `0T6C`, `1C0ZZ`, `1P0U`, `D1DX`). These 20 are part of the four classes above, not a
   separate group.
3. **Regional rules.** The official sources read in §9.1 (Dz.U. 2026 poz. 406, ISED RIC-9,
   47 CFR 97, ADIF 3.1.8) give no authoritative mapping from a callsign prefix to an
   administrative region. The legacy rules never delivered a region name: v1.2.0 shipped the
   same `serviceLOG.db` (blob `6d701c1`) and read the always-empty `Province.Comment`, so every
   regional result carried `Some("")`; I3 fixes the reading for the transition period. Regional
   detail is provided by user files in `#splogbook-rules,1` (`core::reference::rules`: `exact`,
   `prefix` and `pattern` rules with entity code, subdivision code and name, CQ/ITU zone,
   continent, coordinates, priority and provenance; no validity dates) and by the local
   callbook (`#splogbook-callbook,1`).
4. **Default offline source.** For entity, CQ/ITU zone, continent, coordinates and exact calls
   the default source is Big CTY (`cty.dat` / `cty.csv`, MIT-style licence), with the D-ENT
   corrections (ARRL DXCC List / ADIF 3.1.8) and the user rules on top.
5. **Result.** No row of §7 is `BLOCKED_MISSING_REPLACEMENT` or `OWNER_DECISION_REQUIRED`, which
   meets the entry condition of R1C. R1C is **not started** and needs its own owner GO.

Notes for R1C:

* `test_local_callbook_lookup` (`src/core/callbook.rs`, "Test stacji SP1ADT z bazy callbook.db")
  reads the real `databases/callbook.db` and only asserts when the file exists. It must be
  replaced by a synthetic test before `databases/callbook.db` is removed.
* The three files are still shipped by the portable ZIP step of
  `.github/workflows/build-and-release.yml`, the Inno Setup script and the MSI components, and
  are preserved as first-install seeds by the updater; the legacy readers in `main.rs`,
  `gui/app.rs`, `core::service_db`, `core::callbook`, `core::clubs` (built-in registry) and the
  legacy compatibility layer of the prefix loader still look for them.
