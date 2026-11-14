# SPLogbook — Security process (maintainer guide)

How security work is performed in this repository. For the public policy and reporting
instructions see [`../SECURITY.md`](../SECURITY.md); for the release supply-chain
specification see [`security/RELEASE_SECURITY.md`](security/RELEASE_SECURITY.md).

---

## 1. Non-negotiable rules

1. **A new advisory is a stop condition.** A new `cargo audit` finding halts the current
   work, is recorded by advisory ID, and is fixed only in a dedicated dependency-security
   change. It is never absorbed into an unrelated change and never silenced.
2. **`cargo audit --ignore` requires a formal owner decision.** Adding an entry without one
   is a defect.
3. **Never weaken a control to make a build pass.** No test is edited to become green, no
   guard is disabled, no verification step is masked with `|| true`.
4. **Never hide a secret.** A discovered secret is reported by type and path only; the value
   is never copied into a document, commit message, issue or chat.
5. **History is not rewritten** in this repository: no `git filter-repo`, no BFG, no rebase of
   published history, no force-push. Exposure decisions belong to the repository owner.
6. **CI is fail-closed.** A release job aborts when the signing secret is missing, when any
   verification job fails, or when the artifact set is incomplete or has duplicate names.

## 2. Dependency audit

The audit command is inline in `.github/workflows/ci.yml` and must keep this shape:

```bash
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --deny warnings --no-yanked \
  --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2025-0141 --ignore RUSTSEC-2026-0249 \
  --ignore RUSTSEC-2026-0370 --ignore RUSTSEC-2026-0371 --ignore RUSTSEC-2026-0372 \
  --ignore RUSTSEC-2026-0097
```

Those seven identifiers are the approved baseline: `unmaintained` and `yanked` entries in
transitive dependencies that do not affect shipped behaviour. Any change to that list is a
decision, not a maintenance detail. `security_guard` enforces that the list stays explicit,
fully qualified, wildcard-free, and that `--deny warnings` and vulnerability failures stay
enabled.

### Procedure for a new advisory

1. Record the advisory ID, the affected crate and version range, and the dependency path
   (`cargo tree -i <crate>`).
2. Stop the current work and report the blocker. Do not update dependencies in an unrelated
   change and do not add an `--ignore` entry.
3. Open a dedicated dependency-security change: update only the affected package
   (`cargo update -p <pkg> --precise <ver>`), keep `Cargo.toml` unchanged where possible, and
   require `cargo audit` exit 0 before and after.
4. Re-run the full baseline and the packaging guards, because a lockfile change can alter
   binaries and artifact digests.
5. Record the outcome in [`VERIFICATION_BASELINE.md`](VERIFICATION_BASELINE.md) and, if the
   change is user-visible, in [`../CHANGELOG.md`](../CHANGELOG.md).

Note: `quinn` is an optional `reqwest` dependency behind the `http3` feature, which is not
enabled. It is not compiled into the shipped binaries, but it is still scanned because
`cargo audit` evaluates the lockfile. Do not remove the dependency to silence the scanner.

## 3. Release signing and key management

- The release manifest is signed with Ed25519. The private key exists **only** as the
  `RELEASE_SIGNING_KEY` GitHub Actions secret. The repository holds only the 32 public bytes
  and their SHA-256 fingerprint.
- The signer writes the private key to a temporary file with `umask 077`, verifies its own
  signature, and removes the file through an `EXIT` trap. It never prints key material, and
  error messages never include it.
- Active key: `splogbook-release-2026-02`. Revoked: `splogbook-release-2026-01` and the test
  keys. Revocation is permanent: `REVOKED_KEY_IDS` and `REVOKED_PUBLIC_KEYS` are appended to,
  never pruned, and a manifest signed by a revoked key is rejected with `UntrustedKey` before
  any network or filesystem side effect.
- Rotation procedure and the incident record: [`security/SECURITY_KEY_ROTATION.md`](security/SECURITY_KEY_ROTATION.md).
- Emergency revocation: set the key record to `active: false`, which makes every manifest
  signed with it fail closed; removing `release-manifest.json.sig` makes updates fail with
  `MissingSignature`. Both are deliberate, user-visible stop switches.

## 4. Secret handling and scanning

- CI runs a secret scan before anything else: it rejects tracked key files
  (`scratch.rs`, `.signer_priv.key`, `klucz.key`, `.test.key`, `*.p12`, `*.pfx`) and any
  `*.pem` file containing `PRIVATE KEY`.
- `.gitignore` rejects `*.key`, `*.pem`, `*.p12`, `*.pfx`, `.signer_priv.key`, `.test.key`,
  `klucz.key`, `scratch.rs` and `src/scratch.rs`.
- Credentials for QRZ, eQSL, ClubLog, LAN sync and the local REST token live in the operating
  system credential store (or the encrypted `vault.enc` fallback), never in
  `station_config.json`, never in logs, and never in a diagnostic export.
- If a secret is found in the working tree: remove it from the tree, rotate it, and report.
  If a secret is found in **history**: do not rewrite history; report the type and path,
  request immediate rotation, and let the owner decide about exposure.

## 5. Fail-closed native Linux verification

`scripts/verify_linux_container.sh` is the native Linux verifier. Its contract:

- ELF type, `ldd` output, `--version`, `--build-info` and the maximum required `GLIBC_2.43`
  symbol are **assertions**, not informational output.
- `error while loading shared libraries`, `not found`, and a non-zero `ldd` exit code are
  fatal.
- The version comparison is semantic (`sort -V` with a `version_le` helper), not textual.
- Runtime libraries are installed from the official package manager of each pinned OCI image
  by `scripts/install_linux_runtime_deps.sh` **before** the verifier runs, and the 15 required
  sonames are checked through `ldconfig -p`.
- `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` is printed only as the last statement, after
  every check passed.
- A negative CI job runs the verifier on a bare image without runtime libraries and requires
  a non-zero exit code **and** the absence of the success message.

Masking any of those steps with `|| true` is forbidden, and `m3_f_f03_fail_closed_verification_guard`
enforces the contract. This control was established by the now-closed finding `F-03` and must
not be reopened without a reproducible regression.

OCI containers share the host kernel. Native verification is userspace and CI evidence; it is
never proof of a distribution kernel, and `uname -r` inside a container is not native evidence.

## 6. Data-safety invariants

- `classify_update_path()` assigns every path to `ProgramFile`, `RuntimeLibrary`,
  `FirstInstallSeed`, `UserConfiguration`, `UserData`, `ReferenceData`, `ReleaseMetadata` or
  `Unknown`. **`Unknown` is never overwritten.**
- `FirstInstallSeed` files (`databases/serviceLOG.db`, `databases/callbook.db`) are created
  only when absent, in installers and in the updater alike.
- User data (`default_log.db`, `*.db-wal`, `*.db-shm`, `*.db-journal`, `*.adi`, `*.adif`,
  `*.cbr`, backups, logs) and user configuration (`station_config.json`, `vault.enc`) are never
  packaged, never replaced, and never deleted by an installer or an uninstaller.
- Installers remove only files they registered; directory cleanup uses `dirifempty`
  semantics, never wildcards and never recursive deletion.
- File replacement is journaled: `.rollback` holds only replaced program files, is bound to the
  attempt ID and plan digest, and is never auto-deleted while `OWNER-M2-08`/`OWNER-M2-09`
  remain open.

## 7. CI security controls

| Control | Enforcement |
|---|---|
| Minimal permissions | top-level `permissions: contents: read`; `contents: write` only in the release job |
| Action pinning | every action pinned to a 40-character commit SHA |
| Formatter and linter gates | `cargo fmt --check`, `cargo clippy -D warnings`; no `continue-on-error` |
| Dependency audit | pinned `cargo-audit 0.22.2`, explicit advisory baseline, `--deny warnings` |
| Secret scan | tracked key files and PEM private keys rejected before the build |
| Release gating | `publish-release` requires a `v*` tag, a non-dry-run dispatch, and every verification job |
| Artifact set | incomplete or duplicate artifact names abort the release job |
| Signature checks | manifest signature, key fingerprint, and `SHA256SUMS.txt` self-verification |

`security_guard` enforces each of these controls as executable assertions. If a control must
change, the guard changes with it — and only to become stricter or to follow a recorded
decision.

## 8. Reviewing a security-sensitive change

1. Confirm the change does not alter trust, data classification or physical-safety behaviour
   without a matching decision in [`architecture/DECISIONS.md`](architecture/DECISIONS.md).
2. Confirm no guard was weakened: compare assertion counts and messages, not only the pass
   result.
3. Confirm `cargo audit` is green and the advisory baseline list is unchanged.
4. Confirm the change does not introduce a new command execution path, network client, or
   privilege boundary.
5. Confirm user data is still excluded from every update plan and every installer allowlist.
