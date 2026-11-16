# SPLogbook — Verified baseline

Single source of truth for verified test counters and dependency-audit state. Counters appear
only here; every other document links to this file. Never update a counter without a real
measurement, and always record the measurement date.

**Data pomiaru:** 2026-11-16 (ostatni pomiar: po `STAGE-P1-F16-SOFTWARE`)
**Środowisko:** Windows 10/11 x86_64, `rustc 1.99.0`, `cargo 1.99.0`, `-j 1`
**HEAD pomiaru:** commit zawierający ten plik (`git log -1 --format=%H`)

---

## 1. Canonical counting command

```
cargo test --workspace --all-targets -j 1 -- --skip powershell_real_execution_full_cycle_success
```

- Registered = the number of tests reported by `cargo test --workspace --all-targets -- --list`.
- Executed = tests that ran (registered minus ignored minus filtered).
- Filtered = tests excluded on this host only; in CI nothing is filtered.
- Ignored = tests whose bodies are not run. All 10 are hardware-in-the-loop tests in
  `tests/cat_native_7.rs` marked `#[ignore = "hardware-in-the-loop"]`.

## 2. Workspace counters

| Metric | Value |
|---|---|
| TOTAL REGISTERED TESTS: | 3152 |
| TOTAL EXECUTED TESTS: | 3141 |
| TOTAL PASSED TESTS: | 3141 |
| TOTAL FAILED TESTS: | 0 |
| TOTAL IGNORED TESTS: | 10 |
| HIL IGNORED TESTS: | 10 |
| TOTAL FILTERED TESTS: | 1 |
| Doc-tests | 0 |
| Test binaries | 31 |

The single filtered test is `windows_helper_test::powershell_real_execution_full_cycle_success`.
It can block on a Windows security prompt in an interactive sandbox, so it is filtered in the
local baseline. It is **not** filtered in CI, where it passes; this is a host limitation, not
a product defect.

## 3. Per-suite counters

| Suite | Value |
|---|---|
| `security_guard` | 167 / 167 |
| `p1_f16_logbook_pagination` | 55 / 55 |
| pagination model (`gui::pagination`, lib) | 23 / 23 |
| logbook pagination state (`gui::state::log_view`, lib) | 6 / 6 |
| `p1_f17_ptt_gating` | 30 / 30 |
| `cat_f14_fail_safe` | 21 / 21 |
| `cat_f15_independent_evidence` | 9 / 9 |
| `protocol_conformance` | 11 / 11 |
| `cat_018_kenwood_conformance` | 22 / 22 |
| `cat_019_flrig_conformance` | 19 / 19 |
| `linux_flatpak_packaging_test` | 29 / 29 |
| `linux_xdg_data_preservation_test` | 7 / 7 |
| `process_environment_isolation_test` | 12 / 12 |
| `linux_deb_packaging_test` | 17 / 17 |
| `linux_rpm_packaging_test` | 17 / 17 |
| `linux_archive_security_test` | 13 / 13 |
| `linux_appimage_packaging_test` | 14 / 14 |
| `linux_portable_tar_gz_test` | 7 / 7 |
| `linux_multi_distribution_compatibility_test` | 5 / 5 |
| `splogbook-updater` unit tests | 5 / 5 |
| `windows_helper_test` | 33 / 33 executed (+1 host-filtered; 34 registered) |
| `windows_inno_setup_test` | 14 / 14 |
| `windows_msi_test` | 18 / 18 |
| `inno_user_data_safety_test` | 8 / 8 |
| `msi_user_data_safety_test` | 6 / 6 |
| `cat_native_6a` / `cat_native_6b` / `cat_native_7` | 165 / 99 / 307 registered (297 executed + 10 `#[ignore]`) |
| `i18n_guard` | 15 / 15 |
| `theme_guard` | 1 / 1 |

## 4. Static checks

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | EXIT 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | EXIT 0 |
| `cargo doc --no-deps` | EXIT 0 (pre-existing rustdoc warnings only) |
| `cargo tree -d` | 47 duplicate dependency versions |
| `git diff --check` | EXIT 0 |

## 5. Dependency audit

| Metric | Value |
|---|---|
| `cargo audit` | **EXIT 0 — 0 vulnerabilities** |
| Advisory database size at measurement | 1509 advisories |
| Packages scanned | 663 |
| Command | `cargo audit --deny warnings --no-yanked --ignore …` (the explicit baseline in `.github/workflows/ci.yml`); the local re-measurement used `--no-fetch` against the cached database because this host cannot write the advisory-db lock file, CI always fetches |
| Approved ignores | 7 (`unmaintained` / `yanked` transitive entries only) |
| New `--ignore` entries in this measurement | none |
| Offline re-check | `cargo audit --no-fetch …` reuses the cached advisory database and is useful when the environment forbids writing the advisory-db lock file; CI always fetches |

Resolved advisories (do not reintroduce):

| Advisory | Crate | Resolution |
|---|---|---|
| `RUSTSEC-2026-0459` (CVE-2026-119375) | `libsqlite3-sys` | `rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`, bundled SQLite 3.54.1 |
| `RUSTSEC-2026-0486` | `rustix` | `rustix 1.1.6` |
| `RUSTSEC-2026-0523`, `RUSTSEC-2026-0524` | `quinn-proto` | `quinn-proto 0.11.19` |
| `RUSTSEC-2026-0538` | `quinn-udp` | `quinn-udp 0.5.16` |

If a new advisory appears, the procedure in
[`SECURITY_PROCESS.md`](SECURITY_PROCESS.md) §2 applies: stop, record the ID, and fix it only
in a dedicated dependency-security change.

## 6. Environment-determinism verification (2026-11-14)

Process-environment isolation was audited and repaired after CI run `43571747469` attempt 1
failed on `tests/linux_xdg_data_preservation_test.rs:47` (see
[`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) §5). The measurement below is a real local run of the
final tree.

| Item | Value |
|---|---|
| Shared test guard | `tests/support/process_env.rs` (one process-wide `Mutex`, RAII, poison recovery, nested-guard rejection) |
| Tests migrated to the guard | `tests/linux_xdg_data_preservation_test.rs` (7 tests, including 2 moved from `src/core/xdg.rs`) |
| Helper contract + negative tests | `process_environment_isolation_test` — 12 / 12 |
| Security guard for the rule | `qa_process_environment_mutation_is_serialized_and_guarded` (part of `security_guard` 127 / 127) |
| Iterations of the XDG failure test (both `test_xdg_relative_paths_are_rejected_and_fallback_to_home` and `test_xdg_base_directory_resolution_enforces_absolute_paths`) | 100 + 100 — all passed |
| Iterations of the whole `linux_xdg_data_preservation_test` binary (default parallelism) | 50 — all passed |
| Iterations of the helper suite | 100 — all passed |
| Iterations of the mixed HOME/XDG set (both binaries, concurrent) | 100 — all passed |
| Iterations with `RUST_TEST_THREADS` = 1 / 2 / 8 | 20 + 20 + 20 per binary — all passed |
| Iterations with varied `HOME` / `XDG_*` (absolute, relative, empty, missing) | 5 variants × 20 — all passed |
| Liczba iteracji stresowych | 670 — 0 failures |
| Full workspace runs on the final tree | 3 consecutive runs, all EXIT 0 (see the gate report for timings) |
| New dependencies for test serialization | none (`Cargo.toml`/`Cargo.lock` unchanged) |
| `--test-threads=1`, `#[ignore]`, filtering, retry, sleep as synchronization | none |

The only test code that still mutates process environment outside the shared guard is the
already serialized HIL block in `tests/cat_native_7.rs`: it uses its own `tokio::sync::Mutex`
for the disjoint `SPLOG_HIL_*` namespace in a separate test process. `security_guard` pins and
documents that exception.

## 7. Non-counters that must stay true

| Invariant | Value |
|---|---|
| `Cargo.toml` / `Cargo.lock` | unchanged by documentation, cleanup or test-determinism work |
| Test environment isolation | all `HOME`/`XDG_*` mutations go through `tests/support/process_env.rs` |
| Pinned CI toolchain | `1.99.0` |
| Package version | `1.3.0-alpha.1` |
| Public stable | `1.2.0` (tag `v1.2.0`, immutable) |
| Hardware-in-the-loop tests executed | 0 |
| PTT / RF operations | 0 / 0 |
| Flatpak (`M3-J`) | delivered and verified in CI: pinned `org.freedesktop` 26.08 runtime/SDK, exact minimal sandbox, offline locked Cargo build from 662 pinned crates, 48 negative fixtures, full lifecycle and payload-reproducible builds (see `installer/linux/flatpak/README.md`) |

## 8. P0-F14 software verification (2026-11-15)

Measured on the P0-F14 tree with the canonical command from section 1 (local -j 1, one host
filter as documented above).

| Item | Value |
|---|---|
| Registered / executed / passed | 2845 / 2834 / 2834 |
| Failed | 0 |
| Ignored (all hardware-in-the-loop) | 10 |
| Filtered locally | 1 (windows_helper_test::powershell_real_execution_full_cycle_success) |
| Test binaries | 26 |
| New F-14 suite | cat_f14_fail_safe 28 / 28 |
| Protocol conformance suite | protocol_conformance 11 / 11 |
| Security guard (incl. 5 p0_f14_* rules) | 133 / 133 |
| F-14 reproduction before the fix | reproduced: ssertion failed: min <= max after PTT ON |
| F-14 after the fix | controlled Result::Err, zero PTT ON, zero panic |
| SOFTWARE_ONLY_VERIFICATION | true |
| HARDWARE_CONNECTED / REAL_TRANSCEIVER_CONNECTED | false / false |
| REAL_PTT_EXECUTED / RF_TRANSMISSION_EXECUTED | false / false |
| HIL_EXECUTED | false (10 #[ignore = "hardware-in-the-loop"] unchanged) |
| TEST_DOUBLE_PRODUCTION_EXCLUSION_VERIFIED | true (release binary scanned: no test-double symbols) |
| Cargo.toml / Cargo.lock | unchanged |

The P0-F14 tests drive the real production HilHardwareSession against test-only protocol
doubles in 	ests/support/protocol_double.rs; they never open a port, a device or a real

igctld, never key a transmitter and never emit RF.

## 9. CAT-018 / CAT-019 protocol conformance (2026-11-15)

Measured on the `STAGE-CAT-018-019-SOFTWARE` tree with the canonical command from section 1.
Software-only verification: no transceiver, serial port, real FLRig, PTT or RF was used.

| Item | Value |
|---|---|
| Registered / executed / passed | 2894 / 2883 / 2883 |
| Failed | 0 |
| Ignored (all hardware-in-the-loop) | 10 |
| Filtered locally | 1 (`windows_helper_test::powershell_real_execution_full_cycle_success`) |
| Test binaries | 28 |
| Kenwood conformance suite | `cat_018_kenwood_conformance` 22 / 22 |
| FLRig conformance suite | `cat_019_flrig_conformance` 19 / 19 |
| Protocol conformance suite | `protocol_conformance` 11 / 11 |
| Security guard (incl. 6 `cat018_*`/`cat019_*` rules) | 139 / 139 |
| F-14 fail-safe suite (unchanged) | `cat_f14_fail_safe` 28 / 28 |
| CAT-018 reproduction before the fix | `TX0;` parsed as RX, disconnect fail-safe skipped, `IF;` sent to TS-890S — 3 tests failed |
| CAT-019 reproduction before the fix | `<boolean>1</boolean>` on the wire, nil response rejected, `<int>0</int>` treated as success — 4 tests failed |
| Kenwood PTT stress | 1000 ON + 1000 OFF per profile (4 profiles), 500 rejections + 500 timeouts + 500 transport losses per profile |
| FLRig PTT stress | 1000 ON + 1000 OFF, 1000 faults, 1000 malformed responses, 1000 HTTP 500, 5 HTTP statuses × 200, 1000 client timeouts + 1000 peer disconnects |
| `Cargo.toml` / `Cargo.lock` | unchanged |
| `SOFTWARE_ONLY_VERIFICATION` | true |
| `HARDWARE_CONNECTED` / `REAL_TRANSCEIVER_CONNECTED` | false / false |
| `REAL_PTT_EXECUTED` / `RF_TRANSMISSION_EXECUTED` / `HIL_EXECUTED` | false / false / false |
| `PROTOCOL_CONFORMANCE_SOFTWARE_VERIFIED` | true |
| `HARDWARE_BEHAVIOUR_NOT_VERIFIED` | true |
| `TEST_DOUBLE_PRODUCTION_EXCLUSION_VERIFIED` | true (release binary scanned: no test-double symbols) |

The new suites drive the real production codecs/clients through test-only doubles
(`tests/support/kenwood_wire.rs` in-memory `tokio::io::duplex`, `tests/support/flrig_wire.rs`
loopback `127.0.0.1:0` XML-RPC daemon). They never open a serial port, never connect to a real
FLRig, never key a transmitter and never emit RF.

## 10. P1-F15 HIL readiness independent-evidence remediation (2026-11-15)

Measured on the `STAGE-P1-F15-SOFTWARE` tree with the canonical command from section 1 and
`cargo test --workspace --all-targets -j 1 -- --list` for the registered count. Software-only
verification: no transceiver, serial port, dummy load, real PTT or RF was used, and no evidence
provider exists in the production code.

| Item | Value |
|---|---|
| Registered / executed / passed | 2961 / 2950 / 2950 |
| Failed | 0 |
| Ignored (all hardware-in-the-loop) | 10 |
| Filtered locally | 1 (`windows_helper_test::powershell_real_execution_full_cycle_success`) |
| Test binaries | 29 |
| New F-15 integration suite | `cat_f15_independent_evidence` 9 / 9 |
| F-14 fail-safe suite (fail-closed contract) | `cat_f14_fail_safe` 21 / 21 |
| F-14/F-15 mechanical, stress and canonical-matrix tests under `cfg(test)` | 59 / 59 (`cargo test --lib hil::`) |
| Native CAT offline suite | `cat_native_7` 297 executed + 10 `#[ignore]` |
| Security guard (incl. 7 `p1_f15_*` rules and 15 `p1_f17_*` rules) | 161 / 161 |
| Deserialisation matrix | 24 mandatory cases (empty, missing, false, true-without-evidence, null, wrong types, duplicate, unknown, alias/case, empty/partial object, legacy schema, future schema, malformed JSON, missing nested objects, partial attestations, `serde(default)`, `Default::default()`, custom defaults, migration) |
| Self-certification reproduction before the fix | manifest with default-true booleans produced `overall_status = Passed` and reachable PTT |
| Self-certification after the fix | every self-certified case → controlled denial, `PTT_ON_COUNT = 0` |
| Evidence matrix (test scope only) | valid, missing, malformed, expired, not-yet-valid, wrong scope/digest/test/rig/session/attempt, replay, unsupported schema/issuer, provider error/timeout/cancellation |
| Combinatorial matrix | 48 deterministic combinations, every one denied |
| Stress | 2000 missing-evidence PTT attempts, 2000 missing fields, 2000 self-certification, 2000 all-booleans-true, 2000 missing evidence, 1000 malformed/expired/wrong-scope/wrong-digest/replay/provider-timeout/provider-error/cancellation/migration, 500 test-only-provider issuances, 1000 malformed manifests; deterministic, no sleep |
| `SOFTWARE_ONLY_VERIFICATION` | true |
| `PRODUCTION_HIL_DEFAULT_DENIED` | true |
| `HARDWARE_READINESS_NOT_VERIFIED` | true |
| `HARDWARE_CONNECTED` / `REAL_TRANSCEIVER_CONNECTED` | false / false |
| `REAL_PTT_EXECUTED` / `RF_TRANSMISSION_EXECUTED` / `HIL_EXECUTED` | false / false / false |
| `HIL_TESTS_REMAIN_IGNORED` | true (10 `#[ignore = "hardware-in-the-loop"]`) |
| `TEST_EVIDENCE_PROVIDER_TEST_SCOPE_ONLY` | true (`#[cfg(test)]` only) |
| `TEST_EVIDENCE_PROVIDER_COMPILED_IN_PRODUCTION` | false |
| `Cargo.toml` / `Cargo.lock` | unchanged |
| `F-14` / `CAT-018` / `CAT-019` | remain CLOSED |
| `F-17` / `NATIVE-7B` | OPEN (at this measurement) / NOT_STARTED |

The F-15 suites drive the **production** authorization chain. Without an independent evidence
document every entry point (controlled PTT, TX guard, explicit arbiter, verification APIs) is a
controlled denial with `SetPtt(true)` count 0; the mechanical F-14 invariants are additionally
verified in `src/cat/hil/session.rs` under `#[cfg(test)]`, where a test-scope evidence document is
available. The ten hardware-in-the-loop tests stay ignored and are never run by CI.

## 11. P1-F17 safe PTT interaction (2026-11-16)

Measured on the `STAGE-P1-F17-SOFTWARE` tree with the canonical command from section 1 (`-j 1`,
`--skip powershell_real_execution_full_cycle_success`). Software-only verification: no
transceiver, CAT transport, PTT line or RF was used.

| Item | Value |
|---|---|
| `p1_f17_ptt_gating` | 30 / 30 |
| `security_guard` (incl. 15 `p1_f17_*` rules) | 161 / 161 |
| Unsafe bare-`F7` contract reproduced before the fix | yes — the production resolver returned `AppAction::TogglePtt` for a bare `F7` press while a text field had focus |
| Unsafe bare-`F7` after the fix | 0 PTT requests (`PTT_REQUEST_COUNT = 0`, `PTT_ON_COUNT = 0`, `BACKEND_CALL_COUNT = 0`) |
| `F17_ROOT_CAUSE_CONFIRMED` | true |
| `F17_SINGLE_KEY_PTT_REMOVED` / `F17_NO_OTHER_SINGLE_KEY_PTT` | true / true |
| `F17_TEXT_FOCUS_BLOCKS_PTT` / `F17_IME_BLOCKS_PTT` | true / true |
| `F17_WINDOW_UNFOCUSED_BLOCKS_PTT` / `F17_MODAL_BLOCKS_PTT` | true / true |
| `F17_REPEAT_BLOCKS_PTT` / `F17_RELEASE_DOES_NOT_TRIGGER_ON` | true / true |
| `F17_EVENT_CONSUMED_ONCE` / `F17_CENTRAL_DISPATCHER_USED` | true / true |
| `F17_GUI_DIRECT_SET_PTT_REMOVED` | true (no `toggle_ptt`, no `RigCommand::SetPtt`, no `SetPtt(true)` in `src/gui`) |
| `F17_F15_AUTHORIZATION_REQUIRED` / `F17_DRY_RUN_BLOCKS_PTT` / `F17_DENIED_BLOCKS_PTT` | true / true / true |
| `F17_GUI_STATE_NOT_OPTIMISTIC` | true (single telemetry-derived write of `cat.ptt_active`) |
| `PTT_ON_COUNT_WITHOUT_AUTHORIZATION` | 0 |
| Deterministic combinatorial matrix | 26 880 combinations, every safety invariant held |
| Stress (deterministic, sleep-free) | 5 000 bare-key, 5 000 text-focus, 5 000 repeat, 5 005 unauthorized, 2 000 focus-loss, 2 000 IME, 2 000 modal, 2 000 multi-event frames, 1 000 cancellation, 1 000 OFF error, 500 test-double port, plus a 1 920-case panic-free sweep |
| Fault injection | port failure on ON and OFF, closed gates, every arbitration state, every authorization outcome — all controlled denials, no panic |
| `TEST_INPUT_PROVIDER_TEST_SCOPE_ONLY` | true (`#[cfg(test)]` and `tests/` only) |
| `TEST_INPUT_PROVIDER_COMPILED_IN_PRODUCTION` | false (release binary scanned: no test-double symbols or strings) |
| `TEST_INPUT_PROVIDER_PRESENT_IN_RELEASE_ARTIFACTS` | false |
| `SOFTWARE_ONLY_VERIFICATION` | true |
| `HARDWARE_PTT_NOT_VERIFIED` | true |
| `HARDWARE_CONNECTED` / `REAL_TRANSCEIVER_CONNECTED` | false / false |
| `REAL_PTT_EXECUTED` / `RF_TRANSMISSION_EXECUTED` / `HIL_EXECUTED` | false / false / false |
| `HIL_TESTS_REMAIN_IGNORED` | true (10 `#[ignore = "hardware-in-the-loop"]`) |
| `Cargo.toml` / `Cargo.lock` | unchanged |
| `F-14` / `CAT-018` / `CAT-019` / `F-15` | remain CLOSED |
| `F-17` | CLOSED (software) |
| `NATIVE-7B` | NOT_STARTED |

The F-17 suites drive the **production** GUI input path
(`egui::Context::run_ui` → `SpLogApp::handle_input_and_shortcuts` → `ShortcutRegistry::resolve` →
`SpLogApp::dispatch_ptt` → `gui::ptt_input::dispatch`) and the production arbitration path is left
unchanged. Without an F-15 authorization decision every PTT ON is a controlled denial with
`SetPtt(true)` count 0; PTT OFF never requires authorization and remains available while the
arbiter confirms local transmission. The recording port double exists only in test scope and is
absent from the release binary. Hardware behaviour remains **NOT_VERIFIED**.
## 12. P1-F16 panic-free logbook pagination (2026-11-16)

Measured on the `STAGE-P1-F16-SOFTWARE` tree with the canonical command from section 1
(`-j 1`, `--skip powershell_real_execution_full_cycle_success`) and
`cargo test --workspace --all-targets -j 1 -- --list` for the registered count. Software-only
verification: the suite uses an in-memory database and synthetic `TST…` records; no real user
log, callsign, journal, production profile, hardware, PTT or RF was used.

| Item | Value |
|---|---|
| `p1_f16_logbook_pagination` | 55 / 55 |
| Pagination model unit tests (`gui::pagination`, lib) | 23 / 23 |
| Logbook pagination state tests (`gui::state::log_view`, lib) | 6 / 6 |
| `security_guard` (incl. 6 `p1_f16_*` rules) | 167 / 167 |
| Panic reproduced before the fix | 3 / 3 controlled reproductions: `range start index 100 out of range for slice of length 1` and `range start index 50 out of range for slice of length 0` at `src/gui/logbook_table.rs:442`; `attempt to multiply with overflow` at `src/gui/logbook_table.rs:724` |
| Panic after the fix | 0 panic; every window satisfies `start <= end <= total` |
| Deterministic combinatorial matrix | 162 408 model combinations + 1 000 transition combinations, all invariants held |
| Stress (deterministic, sleep-free) | 5 000 stale-page, 5 000 empty-dataset, 5 000 page-size, 3 000 deletion-on-last-page, 3 000 rapid-filter, 3 000 filter+sort, 2 000 journal-switch, 2 000 selection-stabilization, 5 000 arithmetic-boundary iterations and 200 GUI frames |
| Coverage proof | page ranges partition the filtered result: no duplicates, no omitted rows, no overlapping ranges |
| Selection | stable QSO `id` (`AUTOINCREMENT`); deleting a record removes exactly that ID |
| `SOFTWARE_ONLY_VERIFICATION` | true |
| `REAL_USER_DATA_USED` | false |
| `HARDWARE_CONNECTED` / `REAL_TRANSCEIVER_CONNECTED` | false / false |
| `REAL_PTT_EXECUTED` / `RF_TRANSMISSION_EXECUTED` / `HIL_EXECUTED` | false / false / false |
| `HIL_TESTS_REMAIN_IGNORED` | true (10 `#[ignore = "hardware-in-the-loop"]`) |
| `CARGO_TOML_UNCHANGED` / `CARGO_LOCK_UNCHANGED` | true / true |
| `F-14` / `CAT-018` / `CAT-019` / `F-15` / `F-17` | remain CLOSED |
| `F-16` | CLOSED — `COMPLETED_LOGBOOK_PAGINATION_PANIC_REMEDIATION_VERIFIED` |
| `NATIVE-7B` | NOT_STARTED |

Logbook pagination is `PANIC_FREE_SOFTWARE_VERIFIED` under the tested synthetic conditions:
stale page normalization, filter transitions, selection stability and the documented
`F-16 = CLOSED` status are verified. This does **not** verify any real user database or any
corrupted production database.
