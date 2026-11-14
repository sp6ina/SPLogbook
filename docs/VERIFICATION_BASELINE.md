# SPLogbook — Verified baseline

Single source of truth for verified test counters and dependency-audit state. Counters appear
only here; every other document links to this file. Never update a counter without a real
measurement, and always record the measurement date.

**Data pomiaru:** 2026-11-14
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
| TOTAL REGISTERED TESTS: | 2754 |
| TOTAL EXECUTED TESTS: | 2743 |
| TOTAL PASSED TESTS: | 2743 |
| TOTAL FAILED TESTS: | 0 |
| TOTAL IGNORED TESTS: | 10 |
| HIL IGNORED TESTS: | 10 |
| TOTAL FILTERED TESTS: | 1 |
| Doc-tests | 0 |
| Test binaries | 22 |

The single filtered test is `windows_helper_test::powershell_real_execution_full_cycle_success`.
It can block on a Windows security prompt in an interactive sandbox, so it is filtered in the
local baseline. It is **not** filtered in CI, where it passes; this is a host limitation, not
a product defect.

## 3. Per-suite counters

| Suite | Value |
|---|---|
| `security_guard` | 126 / 126 |
| `linux_deb_packaging_test` | 17 / 17 |
| `linux_rpm_packaging_test` | 17 / 17 |
| `linux_archive_security_test` | 13 / 13 |
| `linux_appimage_packaging_test` | 14 / 14 |
| `linux_portable_tar_gz_test` | 7 / 7 |
| `linux_xdg_data_preservation_test` | 5 / 5 |
| `linux_multi_distribution_compatibility_test` | 5 / 5 |
| `windows_helper_test` | 33 / 33 |
| `windows_inno_setup_test` | 14 / 14 |
| `windows_msi_test` | 18 / 18 |
| `inno_user_data_safety_test` | 8 / 8 |
| `msi_user_data_safety_test` | 6 / 6 |
| `cat_native_6a` / `cat_native_6b` / `cat_native_7` | 165 / 99 / 307 (10 `#[ignore]` in `cat_native_7`) |
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
| Command | `cargo audit --deny warnings --no-yanked --ignore …` (the explicit baseline in `.github/workflows/ci.yml`) |
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

## 6. Non-counters that must stay true

| Invariant | Value |
|---|---|
| `Cargo.toml` / `Cargo.lock` | unchanged by documentation or cleanup work |
| Pinned CI toolchain | `1.99.0` |
| Package version | `1.3.0-alpha.1` |
| Public stable | `1.2.0` (tag `v1.2.0`, immutable) |
| Hardware-in-the-loop tests executed | 0 |
| PTT / RF operations | 0 / 0 |
| Flatpak (`M3-J`) | not started |
