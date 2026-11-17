# SPLogbook — Reference data inventory and functional parity

Status: **R1A inventory (no file removed)**. Measured on 2026-11-17 against `c326572`.

This document maps every feature that reads bundled reference data to its current source and to
the planned replacement. A bundled data file may only be removed once every feature it serves
reaches `FUNCTIONAL_PARITY_VERIFIED`. A feature without a replacement stays
`BLOCKED_MISSING_REPLACEMENT` and its source file stays in the tree.

## 1. Bundled data files

| File | Size | Content | Read by |
|---|---|---|---|
| `databases/serviceLOG.db` | 3.4 MB | 9 tables, see §2 | `core::prefix`, `core::service_db`, `core::callbook`, `main.rs`, `gui::app` |
| `databases/callbook.db` | 4.4 MB | `Callbook` (51 955 rows: call, name, QTH, grid, state, manager, note), `inform` (1 row) | `core::callbook::LocalCallbook` |
| `databases/clubs.csv` | 0.9 KB | 17 club memberships with member numbers | `core::clubs::ClubRegistry` |

All three files contain records about identifiable third parties (callsigns combined with names,
locations, member numbers or QSL routes).

## 2. `serviceLOG.db` table map

| Table | Rows | Columns read by code | Code path | Feature |
|---|---|---|---|---|
| `UniqueCalls` | 4 310 | Callsign, Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude, Enable | `PrefixMatcher::load_from_db`, `ServiceDatabase::find_unique_call`, `search_unique_calls` | exact-callsign DXCC/zone exceptions; prefix browser window |
| `CountryDataEx` | 453 | Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude, PrefixList (regex), EndDate | `PrefixMatcher::load_from_db` | DXCC entity, zones, continent, coordinates by prefix |
| `Province` | 623 | same as above + Comment (province name), EndDate | `PrefixMatcher::load_from_db` | zone / coordinate overrides for sub-national prefixes; `PrefixInfo::province` is never displayed |
| `managers` | 29 102 | Call, Manager, Years, Notes | `ServiceDatabase::find_manager`, `search_managers`, `LocalCallbook::lookup` | QSL manager lookup and browser |
| `IOTA` | 1 190 | IOTA, Name, Prefix1 | `ServiceDatabase::search_iota`, `find_iota` | IOTA browser and validation (code fallback: 30-entry sample list) |
| `STATE` | 3 545 | State, Name, Country | `ServiceDatabase::search_states` | states / administrative subdivisions browser (code always adds the 50 US states) |
| `Modes` | 47 | — | none | not read; modes come from static code |
| `Bands` | 26 | — | none | not read; bands come from `core::bandplan` / `core::propagation` |
| `PropMode` | 18 | — | none | not read |
| `contest` | 227 | — | none | not read; contests come from static code |
| `Satellite` | 80 | — | none | not read; satellites come from static code |

## 3. Data written into code

| Location | Content | Classification |
|---|---|---|
| `src/core/clubs.rs` `default_registry()` | 7 real memberships with member numbers | personal data; to be emptied |
| `src/core/clubs.rs` tests | real callsigns asserted against the bundled CSV | to be replaced by synthetic CSV in a tempdir |
| `src/main.rs` SCP seed | 12 callsigns, several real private stations | to be removed; SCP is fed from the log and from `MASTER.SCP` |
| `src/core/callbook.rs` `test_local_callbook_lookup` | real station name and QTH asserted against `callbook.db` | to be replaced by a synthetic fixture |
| `src/core/service_db.rs` `fallback_iota` | 30 IOTA group names | public reference facts; kept |
| `src/core/service_db.rs` `US_STATES` | 50 US states | normative public data; kept |
| `tests/hil/ic7300_lab_bench.json` | real operator, callsign, serial number, port | to be renamed to `.example.json` with synthetic values |

## 4. Existing acquisition mechanisms

* `DatabaseUpdater::update_all_with_report` (menu action only) downloads `cty.dat`,
  `MASTER.SCP` and `lotw-user-activity.csv`. `cty.dat` is downloaded but **not parsed** anywhere.
* `trigger_database_update` writes into `./databases` when that directory exists in the current
  working directory; started from a source checkout this writes into the repository (R1D item).
* No import action exists for the local callbook, QSL managers, IOTA, states or clubs: the only
  way to supply them is to place the file at a fixed relative path.

## 5. Functional parity matrix

Columns: FUNCTION · CURRENT_IMPLEMENTATION · CURRENT_DATA_SOURCE · REPLACEMENT_IMPLEMENTATION ·
REPLACEMENT_DATA_SOURCE · FLOW_BEFORE · FLOW_AFTER · TEST_BEFORE · TEST_AFTER · OFFLINE · ONLINE ·
PARITY_STATUS · RESIDUAL_RISK.

| Function | Current impl. | Current source | Replacement impl. | Replacement source | Flow before | Flow after | Test before | Test after | Offline | Online | Parity | Residual risk |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Local callbook | `LocalCallbook::lookup` | `callbook.db` | same API, user data dir path + import action | user-supplied SQLite/CSV | data out of the box | empty until import; neutral notice | `test_local_callbook_lookup` (real record, silently passes without file) | synthetic fixture in tempdir | yes after import | n/a | BLOCKED_MISSING_REPLACEMENT | flow change requires owner acceptance (D4) |
| QRZ | `CallbookSource::Qrz` | network | unchanged | network | unchanged | unchanged | existing unit tests | unchanged | no | yes | not affected | none |
| HamQTH | `CallbookSource::HamQth` | network | unchanged | network | unchanged | unchanged | existing | unchanged | no | yes | not affected | none |
| Callbook cache | `cache_lookup/cache_store` | user cache DB | unchanged | user cache DB | unchanged | unchanged | `test_callbook_cache_roundtrip` | unchanged | yes | n/a | not affected | test fixture uses owner data (R1C) |
| DXCC entity number | `PrefixMatcher` | `CountryDataEx.DXCC`, `UniqueCalls.DXCC` | cty parser + entity-code table | `cty.dat` has **no DXCC numbers** | out of the box | after user download | `app_tests` synthetic schema | synthetic cty in tempdir | yes after download | download | BLOCKED_MISSING_REPLACEMENT | needs D1 |
| CQ zone | `PrefixMatcher` | `CountryDataEx`, `Province`, `UniqueCalls` | cty parser incl. `(n)` overrides | `cty.dat` | out of the box | after user download | synthetic | synthetic | yes | download | planned R1B | rule-order differences vs regex rules |
| ITU zone | `PrefixMatcher` | same | cty parser incl. `[n]` overrides | `cty.dat` | same | same | synthetic | synthetic | yes | download | planned R1B | same |
| Continent | `PrefixMatcher` | same | cty parser incl. `{XX}` overrides | `cty.dat` | same | same | synthetic | synthetic | yes | download | planned R1B | none known |
| Prefixes | `PrefixMatcher` regex rules | `CountryDataEx.PrefixList` | longest-prefix match | `cty.dat` | same | same | synthetic | synthetic | yes | download | planned R1B | regex semantics differ from longest match; parity tests required |
| Full-call exceptions | `UniqueCalls` | serviceLOG | `=CALL` entries | `cty.dat` | same | same | synthetic | synthetic | yes | download | planned R1B | cty exception set differs in content from `UniqueCalls` |
| Coordinates | `PrefixMatcher` lat/lon | serviceLOG | cty lat/lon (longitude sign inverted), `<lat/lon>` overrides | `cty.dat` | same | same | none | synthetic | yes | download | planned R1B | sign convention must be tested |
| Prefix browser | `PrefixManagerWindow` | `UniqueCalls` | search over loaded `=CALL` exceptions | `cty.dat` | data out of the box | after download | none | synthetic | yes | download | planned R1B | column content changes (no ARRL "QSL" column used) |
| IOTA | `ServiceDatabase::search_iota/find_iota` | `IOTA` table + code sample | same API over neutral user DB | user import | 1 190 groups | 30-entry sample until import | none | synthetic | yes | n/a | BLOCKED_MISSING_REPLACEMENT | no licence-clear redistributable source identified (D3) |
| States / admin. units | `ServiceDatabase::search_states` | `STATE` + code US list | same API over neutral user DB + US list | user import | 3 545 rows | 50 US states until import | none | synthetic | yes | n/a | BLOCKED_MISSING_REPLACEMENT | D3 |
| QSL managers | `find_manager`, `search_managers`, callbook fallback | `managers` | same API over neutral user DB | user import; QRZ/HamQTH manager field | 29 102 rows | empty until import | none | synthetic | yes | QRZ/HamQTH | BLOCKED_MISSING_REPLACEMENT | D3 |
| Clubs | `ClubRegistry` | `clubs.csv` + hard-coded registry | same API; empty default; CSV from user data dir | user CSV | 7–17 memberships | empty until user CSV | unit tests (part real) | synthetic tempdir CSV | yes | n/a | planned R1B | none; format unchanged |
| Modes | static code | — | unchanged | — | unchanged | unchanged | existing | unchanged | yes | n/a | not affected | `Modes` table unused |
| Bands | `core::bandplan` | — | unchanged | — | unchanged | unchanged | existing | unchanged | yes | n/a | not affected | `Bands` table unused |
| Contests | static code | — | unchanged | — | unchanged | unchanged | existing | unchanged | yes | n/a | not affected | `contest` table unused |
| Satellites | static code | — | unchanged | — | unchanged | unchanged | existing | unchanged | yes | n/a | not affected | `Satellite` table unused |
| SCP | `ScpEngine` | hard-coded seed + log + `MASTER.SCP` | log + `MASTER.SCP` | user download | seed visible at start | log-derived until download | `scp.rs` tests | synthetic calls | yes | download | planned R1C | none |
| Data import | none for reference data | fixed file paths | import actions per category | user files | copy file manually | menu action | none | synthetic | yes | n/a | planned R1B | GUI surface added, none removed |
| Data export | ADIF/Cabrillo/CSV | log DB | unchanged | log DB | unchanged | unchanged | existing | unchanged | yes | n/a | not affected | none |
| User-triggered download | `DatabaseUpdater` | network | unchanged + parse after download | network | unchanged | unchanged | existing | + reload test | n/a | yes | planned R1B | must write to user data dir, not CWD |
| Data update | same menu action | network | same | network | unchanged | unchanged | existing | + reload test | n/a | yes | planned R1B | none |
| Offline operation | all lookups local | bundled files | all lookups local | installed user data | full | full after data install | none | first-run test | yes | n/a | planned R1D | first run has no reference data |
| Clean first run | `main.rs` path search | CWD / exe dir / app data | user data layer | user data dir | may read repo `databases/` | user data dir only | none | isolated-process test | yes | n/a | planned R1D | — |
| Portable mode | implicit (`exe_dir/databases`) | exe dir | explicit marker | exe dir | implicit | explicit | updater tests | + first-run test | yes | n/a | planned R1D | migration for existing implicit portable installs |
| Installers | Inno/MSI/workflow copy the 3 files | repo files | no reference files shipped | — | seeded | empty, import later | installer tests | updated tests | yes | n/a | planned R1C | existing installs keep their files (never uninstalled) |
| Updater | classifies the 3 files as first-install seeds | — | classification kept for existing installs | — | preserve | preserve | updater tests | unchanged | yes | n/a | not affected | none |
| API | `/lookup` uses `PrefixMatcher`, `ClubRegistry` | serviceLOG, clubs | same | cty, user CSV | data | data after install | server tests | unchanged | yes | n/a | follows rows above | none |
| GUI | IOTA, states, QSL manager, prefix browsers | serviceLOG | same windows, same options | user data | populated | empty state + import hint | none | snapshot of options | yes | n/a | follows rows above | no option removed |
| Tests | some tests read bundled files | repo files | synthetic tempdir fixtures | synthetic | — | — | — | — | — | — | planned R1C | none |

## 6. Owner decisions required before R1B

* **D1 — DXCC entity numbers.** `cty.dat` carries no ADIF DXCC entity code. Options: (a) add the
  same provider's `cty.csv` (contains entity codes) as a second user-triggered download and
  parser; (b) ship the ADIF DXCC entity enumeration as static normative data, subject to licence
  confirmation; (c) user-imported mapping. Recommended: (a).
* **D2 — Prefix rule semantics.** Current matching is regex-based (`PrefixList`); `cty.dat` uses
  longest-prefix matching with exact-call entries. Parity is defined as identical results on a
  synthetic corpus, not on the removed private data.
* **D3 — IOTA, states and QSL managers.** No redistributable, licence-clear source has been
  identified. Proposed parity definition: identical API, windows, search, filter and sort over a
  neutral user database filled by an explicit import; the bundled content is not replaced.
* **D4 — Local callbook.** Same parity definition as D3 (import of a user-supplied file).
* **D5 — Clubs.** Empty default registry; CSV read from the user data directory, same format.
* **D6 — Unused tables.** `Modes`, `Bands`, `PropMode`, `contest`, `Satellite`, `inform` need
  no replacement because no code reads them.
