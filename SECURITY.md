# Security Policy — SPLogbook

This document is the public security policy for the SPLogbook project. Maintainer-level
engineering rules lives in [`docs/SECURITY_PROCESS.md`](docs/SECURITY_PROCESS.md); the
release supply-chain specification is
[`docs/security/RELEASE_SECURITY.md`](docs/security/RELEASE_SECURITY.md).

---

## 1. Supported versions

| Version | State | Supported |
|---|---|---|
| `1.2.0` | public stable, immutable annotated tag `v1.2.0` | yes |
| `1.3.0-alpha.1` | development, **not published** (no tag, no release) | development builds only |
| `≤ 1.1.0` | historical | no |

The development line is not a supported release. Security fixes are delivered through a
new gate and a new version; they are never pushed as silent edits to a published tag.

## 2. Reporting a vulnerability

Report privately — **do not open a public issue** and do not publish proof-of-concept
details before a fix is available.

- E-mail: **contact@splogbook.org**
- Include: affected version, platform, reproduction steps, impact assessment, and
  whether user data or radio hardware could be affected.
- Expect an acknowledgement within a reasonable time. Please allow a fix and a release
  window before disclosure; credit is given on request.

Never include private keys, credentials or user database contents in a report. If you
believe a secret was committed to the repository, report the **type of secret and the
path only** and request rotation — do not redistribute the value.

## 3. In scope

- The application (`SPLogbook`), the update helper (`splogbook-updater`) and the release
  signer (`splogbook-signer`).
- The update trust chain: release manifest, Ed25519 signature, asset selection, archive
  inspection, staging, journaled replacement, health proof and rollback.
- The installers and packages: Windows portable ZIP, Inno Setup user setup, WiX MSI,
  Linux portable `tar.gz`, AppImage, DEB and RPM.
- The build and release pipeline in `.github/workflows/`.

Out of scope: vulnerabilities in third-party distribution package managers, in radio
firmware, or requiring the attacker to already control the user's account.

## 4. Security guarantees the project maintains

These are enforced in code and by `tests/security_guard.rs`; weakening them is a defect.

1. **Manifest-first trust.** An update asset can only be selected from a release whose
   manifest signature verified against a trusted, active Ed25519 key. Selecting an asset
   by filename extension alone is forbidden.
2. **Revoked keys stay revoked.** `splogbook-release-2026-01` and the revoked test keys
   are permanently rejected (`UntrustedKey`), before any network or filesystem side
   effect. A release signed with a revoked key is rejected even if the signature is
   mathematically valid.
3. **Integrity binding.** Asset identity is the exact tuple (platform, architecture,
   package type) **plus** filename, declared size and SHA-256. A mismatch fails closed.
4. **Downgrade protection.** Reinstalling the same or an older version, and prereleases on
   the `StableOnly` channel, are rejected.
5. **No privilege escalation.** The updater and the installers never invoke `sudo`,
   `doas`, `pkexec`, `runas` or UAC elevation, never write to `/usr`, and never replace
   files directly inside `Program Files`.
6. **User data is never overwritten.** SQLite databases, WAL/SHM files, configuration and
   exports are classified as user data or user configuration and are excluded from every
   update plan. An unknown file is never overwritten.
7. **Archive hardening.** Absolute paths, `..` traversal, drive/UNC prefixes, NTFS
   alternate data streams, device names, symlinks, hardlinks, FIFOs and device nodes are
   rejected; entry count, per-file size and total extracted size are bounded.
8. **Fail-closed verification.** Every verifier, guard and installer check aborts the
   operation instead of continuing; a success banner is printed only after all assertions
   passed. `|| true` masking of a verification step is forbidden.
9. **Journaled replacement, not power-loss atomic.** File replacement is
   `JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED` and `CRASH_RECOVERABLE_BY_JOURNAL`. It is
   explicitly **`NOT_POWER_LOSS_ATOMIC`**; documentation must not claim hardware atomicity.
10. **Fail-closed CI.** The release job aborts when the signing secret is missing, when any
    verification job fails, or when an artifact set is incomplete or has duplicate names.

## 5. Dependency and supply-chain policy

- `cargo audit --deny warnings --no-yanked` runs in CI and must exit 0.
- `cargo audit --ignore` is **prohibited** without a formal owner decision; a new advisory
  stops the work and requires a dedicated dependency-security gate.
- Dependencies are updated only through cargo, only in a dedicated gate, and never
  "on the side" of an unrelated change.
- `rusqlite` must keep the `bundled` feature; system SQLite is forbidden.
- GitHub Actions are pinned to full 40-character commit SHAs; the workflows request
  minimal permissions (`contents: read` for CI).
- The repository contains no private keys. The release signing key exists only as the
  `RELEASE_SIGNING_KEY` GitHub Actions secret; the repository holds only the 32 public
  bytes and their SHA-256 fingerprint.
- `.key`, `.pem`, `.p12` and `.pfx` files, and the historical `scratch.rs` key-material
  path, are ignored and are rejected by a CI secret scan.

For the current advisory state and the resolved advisory history, see
[`docs/VERIFICATION_BASELINE.md`](docs/VERIFICATION_BASELINE.md) and
[`docs/SECURITY_PROCESS.md`](docs/SECURITY_PROCESS.md).

## 6. Verifying a download

No release newer than `v1.2.0` has been published; `1.3.0-alpha.1` artifacts are CI-only
and are not attached to any release. The `v1.2.0` release predates the current key
rotation, so its manifest was signed with the now-revoked `splogbook-release-2026-01` key
and the built-in updater correctly refuses it. The full audit of every published release
is recorded in
[`docs/security/HISTORICAL_RELEASE_AUDIT.md`](docs/security/HISTORICAL_RELEASE_AUDIT.md).

## 7. Physical and radio safety

SPLogbook controls radio transceivers. Automated work in this repository must never
perform hardware-in-the-loop tests, PTT keying or RF emission. Those 10 tests are marked
`#[ignore = "hardware-in-the-loop"]` and additionally require an attested 50 Ω dummy load
and explicit operator consent. See [`Native CAT.md`](Native%20CAT.md).
