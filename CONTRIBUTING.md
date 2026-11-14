# Contributing to SPLogbook

Thank you for contributing. This document describes the workflow and the rules that keep
the project buildable, testable and safe for its users. Detailed commands live in
[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md); security rules live in
[`SECURITY.md`](SECURITY.md).

---

## 1. Before you start

1. Read [`README.md`](README.md) for product scope and
   [`docs/PROJECT_ARCHITECTURE_MAP.md`](docs/architecture/ARCHITECTURE.md) for the module map.
2. Check [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) for open defects, especially
   **F-14 (P0)** which must be resolved before any hardware validation.
3. Fetch and start from the current `origin/main`:

   ```bash
   git fetch origin --prune
   git switch main
   git merge --ff-only origin/main
   ```

## 2. Branch and commit workflow

1. Fork the repository and create a feature branch
   (`git switch -c feature/short-description`).
2. Keep changes focused; one logical change per commit.
3. Run the full baseline before committing (see §3).
4. Commit with a descriptive message:

   ```
   feat(logbook): add <thing>
   fix(updater): reject <condition>
   docs(packaging): document <format>
   test(guard): enforce <invariant>
   chore(repo): <maintenance>
   ```

5. Push to your fork and open a pull request against `main`.

Repository history rules (enforced by review and by CI):

- **Fast-forward only.** No rebase, no merge commits, no force-push, no history rewrite,
  no `git filter-repo`, no BFG.
- **Tags are immutable.** Never create, move or delete a tag. The public stable tag
  `v1.2.0` is protected: tag object `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled
  commit `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`.
- **No releases from a pull request.** Publication happens only from a `v*` tag through
  the `publish-release` job.

## 3. Required checks

Run exactly what CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo audit --deny warnings --no-yanked
```

`cargo fmt` and `cargo clippy -D warnings` are required, not advisory. The canonical,
current test counters and their measurement date are recorded in
[`docs/VERIFICATION_BASELINE.md`](docs/VERIFICATION_BASELINE.md) — never hard-code
counters anywhere else, and never update a counter without a real measurement.

## 4. Test rules

- **Never change a test just to get a green result.** Never weaken a security assertion,
  never disable a guard, never convert a failure into a skip.
- Never ignore an exit code, never append `|| true`, never use an empty `return` as a
  hidden skip.
- The standard baseline must not depend on `dist/` or on any untracked artifact.
- Every new invariant in `src/**` that protects user data, trust or physical safety needs a
  guard in `tests/security_guard.rs`.
- If you must move an invariant's source of truth, move it to **code, a manifest, a
  configuration file, a dedicated fixture, or proper technical documentation** — never to a
  gate report or a working note.

### Hardware-in-the-loop

Never run hardware-in-the-loop tests, PTT or RF emission. The 10 HIL tests in
`tests/cat_native_7.rs` are `#[ignore = "hardware-in-the-loop"]` and require physical
hardware, an attested 50 Ω dummy load and explicit operator consent.

### Installer end-to-end tests

Native Windows installer E2E tests are explicit opt-in via `SPLOG_RUN_INNO_E2E=1` /
`SPLOG_RUN_MSI_E2E=1` plus an absolute artifact path. Without opt-in they are an
explicitly unexecuted fixture, never a native PASS. Do not open production databases or
user configuration, and never use the real `HOME`/`APPDATA` in tests.

## 5. Code rules

- Single Cargo package; `core` and `cat` must not depend on `gui`.
- No `unsafe` in the updater; no `unwrap()`, `expect()` or `panic!` in production updater
  paths.
- No shell-outs for installer execution; no `cmd /c`, no `powershell.exe -Command` for
  installers, no wildcard file copies.
- No privilege escalation anywhere: no `sudo`, `doas`, `pkexec`, `runas`.
- GUI code must not construct raw `Color32` outside `src/gui/theme/**` and the isolated
  `qsl_colors` module (enforced by `tests/theme_guard.rs`).
- All strings that reach the user go through the localization catalogs; see
  `locales/README.md` and `tests/i18n_guard.rs`.

## 6. Dependencies

- Dependency updates happen **only** in a dedicated dependency-security change with a
  green `cargo audit` before and after.
- Never add `cargo audit --ignore` entries without a formal owner decision.
- Never hand-edit `Cargo.lock`; use `cargo update -p <package> --precise <version>`.
- `rusqlite` must keep the `bundled` feature.
- Do not change the MSRV or the pinned CI toolchain as a side effect of another change.

## 7. Documentation

- Documentation is written in Markdown, in UTF-8, and in English or Polish as the
  surrounding document already uses.
- **Every tracked Markdown file must be listed in
  [`docs/DOCUMENTATION_INDEX.md`](docs/DOCUMENTATION_INDEX.md) in the same commit that
  adds it.** `security_guard` enforces this.
- A document must not link to a path that is not tracked, and must not describe a removed
  file as existing.
- Working notes, prompts, model handoffs, gate evidence logs and owner-local plans do not
  belong in this repository. Keep them outside the tree (for example in a local directory
  excluded through `.git/info/exclude`, which is never tracked).
- Do not claim hardware atomicity for the updater; the correct terms are
  `JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED`, `CRASH_RECOVERABLE_BY_JOURNAL` and
  `NOT_POWER_LOSS_ATOMIC`.
- Do not falsify authorship or project history, do not remove legally required notices, and
  do not hide security incidents that carry durable public value.

## 8. Licensing

SPLogbook is licensed under **GPL-3.0**. By contributing you agree that your contribution
is distributed under the same license. Third-party components keep their own licenses — see
`installer/linux/deb/templates/` and `installer/linux/rpm/templates/` for the bundled
Hamlib LGPL-2.1 text.

## 9. Reporting bugs and security issues

- Functional bugs and feature requests: open a GitHub issue with the version, platform and
  reproduction steps.
- Security issues: follow [`SECURITY.md`](SECURITY.md) and report privately. Do not open a
  public issue for a vulnerability.
