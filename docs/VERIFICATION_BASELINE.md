# SPLogbook — Verified baseline

Single source of truth for verified test counters and dependency-audit state. Counters appear
only here; every other document links to this file. Never update a counter without a real
measurement, and always record the measurement date.

**Data pomiaru:** 2026-11-15 (ostatni pomiar: po `STAGE-CAT-018-019-SOFTWARE`)
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
| TOTAL REGISTERED TESTS: | 2894 |
| TOTAL EXECUTED TESTS: | 2883 |
| TOTAL PASSED TESTS: | 2883 |
| TOTAL FAILED TESTS: | 0 |
| TOTAL IGNORED TESTS: | 10 |
| HIL IGNORED TESTS: | 10 |
| TOTAL FILTERED TESTS: | 1 |
| Doc-tests | 0 |
| Test binaries | 28 |

The single filtered test is `windows_helper_test::powershell_real_execution_full_cycle_success`.
It can block on a Windows security prompt in an interactive sandbox, so it is filtered in the
local baseline. It is **not** filtered in CI, where it passes; this is a host limitation, not
a product defect.

## 3. Per-suite counters

| Suite | Value |
|---|---|
| `security_guard` | 139 / 139 |
| `cat_f14_fail_safe` | 28 / 28 |
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
| `windows_helper_test` | 33 / 33 executed (+1 host-filtered; 34 registered) |
| `windows_inno_setup_test` | 14 / 14 |
| `windows_msi_test` | 18 / 18 |
| `inno_user_data_safety_test` | 8 / 8 |
| `msi_user_data_safety_test` | 6 / 6 |
| `cat_native_6a` / `cat_native_6b` / `cat_native_7` | 165 / 99 / 307 executed + 10 `#[ignore]` w `cat_native_7` |
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
