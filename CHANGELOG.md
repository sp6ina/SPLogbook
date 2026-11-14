# Changelog

All notable public changes to SPLogbook. This file records what users and packagers need;
it is not a development log.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Current state: **public stable `1.2.0`**, development line **`1.3.0-alpha.1` (not
published)**. No release newer than `1.2.0` exists: there is no `v1.3.0*` tag and no
GitHub release for the development line.

---

## [Unreleased] — 1.3.0-alpha.1

Development version. **Not published**: no tag, no GitHub release, no release assets.
Artifacts are produced and verified by CI only.

### Added

- **Linux portable `tar.gz`** — deterministic single-root archive
  (`SPLogbook-Linux-x86_64-<version>.tar.gz`), two-phase safe extraction with entry/size
  bounds, rejection of links and device nodes, and XDG Base Directory data placement.
- **Linux AppImage** — `SPLogbook-Linux-x86_64.AppImage`, built with a pinned official
  `appimagetool` and a pinned type-2 runtime, with verified FUSE mounting and 12 negative
  fixtures.
- **Linux DEB** — binary `splogbook` package, `1.3.0~alpha.1-1`, `amd64`, installed under
  `/opt/SPLogbook` with the launcher `/usr/bin/splogbook`, desktop entry and icon; 27
  negative fixtures; verified install/upgrade/downgrade/reinstall/remove/purge lifecycle.
- **Linux RPM** — `splogbook-1.3.0-0.1.alpha.1.x86_64`, same `/opt/SPLogbook` layout,
  bundled `libhamlib.so.4` with `RUNPATH=$ORIGIN/lib`; 34 negative fixtures; lifecycle
  verified on Fedora 45 and openSUSE Tumbleweed.
- **Windows Inno Setup user setup** — per-user installer in
  `%LOCALAPPDATA%\Programs\SPLogbook`, no elevation, user data preserved on update and
  uninstall. Implemented and locally verified; **not built or published by CI** (see
  Known limitations).
- **Windows WiX MSI** — per-machine MSI with a fixed `UpgradeCode`, delegated updates
  through `msiexec`, and mutual-exclusion detection against the Inno installer.
  Implemented and locally verified; **not built or published by CI**.
- **Build identity and channels** — `BuildIdentity`, `UpdateChannel`, `--build-info`,
  `--version` and `--diagnostic-version` output; SemVer channel and downgrade policy.
- **Linux support policy snapshot** — `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`
  declaring kernel and glibc minimums and the supported distribution matrix.

### Changed

- **Linux runtime requirement** is now **kernel 7.0** and **glibc 2.43**
  (`GLIBC_2.43` is the maximum required symbol). Supported distributions: Ubuntu 26.04 LTS,
  Fedora 45, Arch Linux, openSUSE Tumbleweed. Debian 13 (glibc 2.41) and openSUSE Leap 16.1
  (glibc 2.40) are not supported.
- Native Linux CI verification is **fail-closed**: `ldd`, `--version`, `--build-info` and
  the required `GLIBC_2.43` symbol are assertions, runtime libraries are installed from the
  official package manager of each pinned image before verification, and a negative job
  proves that a missing library aborts with a non-zero exit code and no success message.
- Native Windows installer end-to-end tests became explicit opt-in and hermetic: they no
  longer auto-detect artifacts in `dist/`, and without opt-in they are an explicitly
  unexecuted fixture rather than a PASS.

### Security

- `RUSTSEC-2026-0459` (CVE-2026-119375) removed: `rusqlite 0.41.1` /
  `libsqlite3-sys 0.39.1`, bundled SQLite 3.54.1.
- `RUSTSEC-2026-0486` removed: `rustix 1.1.6`.
- `RUSTSEC-2026-0523`, `RUSTSEC-2026-0524` removed: `quinn-proto 0.11.19`.
- `RUSTSEC-2026-0538` removed: `quinn-udp 0.5.16`.
- `cargo audit` exits 0 with no new `--ignore` entries; the previously approved
  `unmaintained`/`yanked` entries are unchanged.
- Update trust chain hardened: manifest-first asset selection, permanent rejection of the
  revoked `splogbook-release-2026-01` key, integrity binding on
  (platform, architecture, package type) + filename + size + SHA-256, downgrade protection,
  and archive path/entry hardening.
- File replacement semantics are documented precisely as
  `JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED`, `CRASH_RECOVERABLE_BY_JOURNAL` and
  **`NOT_POWER_LOSS_ATOMIC`**.

### Fixed

- Idle peer timeout, handshake parallelism limit and the concurrent update-lock test are
  now deterministic (virtual time, permit counters) instead of wall-clock dependent.
- The updater journal, lock and transaction records are validated on load and fail closed
  on an unknown version.

### Known limitations

- MSI and Inno Setup installers are **not** built or published by CI; only the portable
  Windows ZIP is. Tracked as finding **F-05**.
- MSI prerelease versioning for `1.3.0-alpha.2` and later is blocked by the open decision
  `OWNER-M3-E-MSI-VERSIONING-01` (Windows Installer error 1638).
- Hardware-in-the-loop validation (NATIVE-7B) is **not executed**: 10 tests remain
  `#[ignore = "hardware-in-the-loop"]`. The open P0 finding **F-14** must be resolved first.
- Flatpak packaging (`M3-J`) is **not started**.

---

## [1.2.0] — 2026-10-26

Public stable release. Annotated tag `v1.2.0` (tag object
`3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled commit
`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`). Immutable.

Release assets:

| Asset | Size | SHA-256 |
|---|---|---|
| `release-manifest.json` | 755 B | `3e29b633db1545a307a6c588abe9f739c50ec500c37434c8c555b0e58e956cb4` |
| `release-manifest.json.sig` | 88 B | `4698556ab52f682d2c2692da1cd34e75ccdd95737b99326adb286a584a58e84f` |
| `SPLogbook-Linux-x86_64.tar.gz` | 20 572 398 B | `3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93` |
| `SPLogbook-Windows-x64.zip` | 20 831 936 B | `bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0` |

Notes:

- The manifest of this release was signed with the key `splogbook-release-2026-01`, which
  was later **revoked** after a key-material incident. The current updater therefore
  rejects `v1.2.0` as `UntrustedKey`; this is intended fail-closed behaviour, not a
  regression. The incident, the rotation and the continuing rejection are documented in
  [`docs/security/SECURITY_KEY_ROTATION.md`](docs/security/SECURITY_KEY_ROTATION.md) and
  [`docs/security/HISTORICAL_RELEASE_AUDIT.md`](docs/security/HISTORICAL_RELEASE_AUDIT.md).
- This release does not include a `SHA256SUMS.txt` asset.

---

## Earlier releases

These releases predate the signed-manifest pipeline and carry no manifest or signature.
They are retained for historical accuracy and are not usable by the auto-updater.

| Version | Published | Notes |
|---|---|---|
| `1.1.0` | 2026-09-29 | Linux `tar.gz` 19 476 625 B and Windows `zip` 20 100 359 B; unsigned, no manifest |
| `1.0.3` | 2026-09-22 | patch release |
| `1.0.2` | 2026-09-22 | patch release |
| `1.0.1` | 2026-09-21 | patch release |
| `1.0.0` | 2026-09-21 | first public build, marked pre-release; Windows-only asset `SPLogbook-v1.0.0-Windows-x64.zip` 12 382 201 B |

The full per-release trust audit (signature presence, key identity, hash verification) is
recorded in
[`docs/security/HISTORICAL_RELEASE_AUDIT.md`](docs/security/HISTORICAL_RELEASE_AUDIT.md).

---

## Not yet released

Nothing on the development line has been published. Packaging gates for the Linux formats
and for the Windows installers are delivered as CI-verified artifacts only; publication
requires a separate, explicitly authorized release gate.
