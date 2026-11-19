# SPLogbook — Security process (maintainer guide)

How security work is performed in this repository. For the public policy and reporting
instructions see [`../SECURITY.md`](../SECURITY.md); for the release supply-chain
specification see [`security/RELEASE_SECURITY.md`](security/RELEASE_SECURITY.md).

---

## 1. Non-negotiable rules

1. **A new advisory is a stop condition.** A new `cargo audit` finding halts the current
   work, is recorded by advisory ID, and is fixed only in a dedicated dependency-security
   change. It is never absorbed into an unrelated change and never silenced.
2. **`cargo audit --ignore` requires an explicit ADR (`ADR-65`).** Adding an entry without one
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

### 5.1 Flatpak sandbox (`M3-J`)

The Flatpak package `io.github.sp6ina.SPLogbook` is verified by
`scripts/verify_linux_flatpak.sh`, whose contract mirrors the F-03 rules:

- the bundle type, magic, size, both SHA-256 sidecars, the per-file payload manifest and the
  exact `/app` export allowlist are **assertions**; the payload manifest may contain no
  symlink, no setuid/setgid/sticky bit and no world-writable file;
- `flatpak info --show-permissions` is compared with the exact approved minimal set
  (`shared=network;ipc;`, `sockets=wayland;fallback-x11;pulseaudio;`, `devices=dri;`,
  `org.freedesktop.secrets=talk`), so an extra filesystem, device, socket, bus name,
  environment variable or persistent path fails the job;
- `--filesystem=host`, `--filesystem=home`, `--filesystem=/dev`, `--device=all`,
  `--socket=system-bus`, wildcard bus names and `--allow=devel|multiarch` are rejected in the
  manifest before anything is downloaded;
- the build performs exactly one network phase (`--download-only`); the real build runs with
  `--disable-download`, a vendored Cargo source replacement, `CARGO_NET_OFFLINE=1` and
  `--locked --offline`;
- `FLATPAK VERIFICATION COMPLETED SUCCESSFULLY` is the last statement, and
  `SPLOG_FLATPAK_INSPECTION_ONLY=1` prints a different banner so a static-only run can never
  be mistaken for an installation proof.

`ADR-57` fixes the device policy: the shipped manifest never declares
`--device=all`. Native serial CAT is unavailable in the default sandbox and fails closed;
the documented user-side opt-in (`flatpak override --user --device=all
io.github.sp6ina.SPLogbook`) widens access to every device visible to Flatpak and is
labelled as such in `installer/linux/flatpak/README.md`.

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
  attempt ID and plan digest, and is never auto-deleted while `ADR-03`
  remains open.

## 7. CI security controls

| Control | Enforcement |
|---|---|
| Minimal permissions | top-level `permissions: contents: read`; `contents: write` only in the release job |
| Action pinning | every action pinned to a 40-character commit SHA |
| Formatter and linter gates | `cargo fmt --check`, `cargo clippy -D warnings`; no `continue-on-error` |
| Dependency audit | pinned `cargo-audit 0.22.2`, explicit advisory baseline, `--deny warnings` |
| Secret scan | tracked key files and PEM private keys rejected before the build |
| Release gating | `publish-release` runs only for a pushed `v*` tag (`startsWith(github.ref, 'refs/tags/v') && github.event_name == 'push'`, `ADR-71`) and needs every verification job; every `workflow_dispatch` skips it and never reads the signing secret |
| Artifact set | incomplete or duplicate artifact names abort the release job (4 release artifacts: Linux `tar.gz`, Windows `zip`, `inno-exe`, `msi`) |
| Signature checks | manifest signature, key fingerprint, and `SHA256SUMS.txt` self-verification |

`security_guard` enforces each of these controls as executable assertions. If a control must
change, the guard changes with it — and only to become stricter or to follow a recorded
decision.

### 7.1 HIL readiness evidence and authorization (F-15)

Hardware-in-the-loop readiness is a trust boundary, not a configuration flag. The production
default is **denied**, and the following rules are enforced by code and by `p1_f15_*` guards:

- no safety field of the HIL plan defaults to `true`; a missing value is a parse error;
- the test plan is not an evidence provider; the operator declaration is not objective evidence;
- the independent evidence document and the authorization decision are separate types that are
  deliberately not deserialisable, so an untrusted document cannot forge an authorization;
- an evidence document is bound by SHA-256 to the plan and to the declaration, scoped to the
  test, rig, session, attempt and validity window, and must cover every required measured fact;
- expired, not-yet-valid, out-of-scope, replayed, provider-error, provider-timeout and cancelled
  evidence are controlled denials; a dry run never keys PTT;
- there is no production evidence provider, and no CLI flag, GUI control, configuration key,
  environment variable or feature flag can select a test provider;
- audit records contain the outcome, the denial reason, the missing element, the evidence id,
  the scope, a truncated plan digest and the attempt id — never secrets or credentials, and an
  audit failure can never turn a denial into an authorization.

The software remediation does **not** prove any physical behaviour: `HARDWARE_READINESS_NOT_VERIFIED`,
`HIL_EXECUTED=false`, `REAL_PTT_EXECUTED=false`, `RF_TRANSMISSION_EXECUTED=false`.

### 7.2 Explicit PTT interaction gate (F-17)

Transmit intent from the user interface is a trust boundary, not a convenience shortcut. The
following rules are enforced by code and by `p1_f17_*` guards:

- PTT is never bound to a single key and never exposed as an ambiguous toggle: `Ctrl+Shift+F7`
  requests TX ON and `Ctrl+F7` requests TX OFF as two disjoint intents, and a bare `F7` does
  nothing;
- both bindings require exact modifiers, so an extra `Shift`, `Alt` or `Super` is an explicit
  mismatch rather than a silent trigger;
- text focus, IME composition, missing window focus and any open modal window each block both
  intents, and a gate that cannot be read fails closed;
- key repeat never retriggers, a release event never creates an intent, and the matching press is
  consumed exactly once per frame;
- every user-interaction surface (shortcut, on-screen buttons, command palette) goes through one
  central dispatcher; the GUI contains no toggle entry point, no `RigCommand::SetPtt` and no
  direct `SetPtt(true)`, and the set of direct runtime transmit call sites in `src/gui` is closed
  and pinned by a guard;
- PTT ON is forwarded only for the F-15 outcome `HardwareExecutionAuthorized`; the production GUI
  gate `evaluate_gui_ptt_authorization()` always returns `NotEvaluated`, so
  `PTT_ON_COUNT_WITHOUT_AUTHORIZATION = 0`;
- PTT OFF is never authorization-gated, because a blocked OFF could leave a transmitter keyed;
- the GUI TX indicator is telemetry-derived, and a denial, an arbiter error or a port error never
  presents TX and is never reported as success;
- losing window focus while transmitting performs exactly one controlled OFF attempt through the
  existing `TxArbiter` path, with no retries and no wall-clock timer;
- test-only PTT doubles live exclusively under `#[cfg(test)]` and `tests/`, and no test input
  provider can be selected from the GUI, the CLI, the configuration or the environment.

The software remediation does **not** prove any physical behaviour:
`HARDWARE_PTT_NOT_VERIFIED`, `HARDWARE_CONNECTED=false`, `REAL_TRANSCEIVER_CONNECTED=false`,
`REAL_PTT_EXECUTED=false`, `RF_TRANSMISSION_EXECUTED=false`, `HIL_EXECUTED=false`.

### 7.3 Panic-free logbook pagination (P1-F16)

Logbook pagination is a crash boundary: the table renders a filtered, sorted index list whose
length changes on every filter, journal switch, deletion, import and sync. The following rules
are enforced by code and by the `p1_f16_*` guards:

- one pure, GUI-independent model (`src/gui/pagination.rs`, `PaginationInput` →
  `PaginationWindow`) computes the page count, the normalized page and the `start..end` window;
  no renderer, reload path or filter path computes its own page count or range;
- the page is normalized against the **filtered** result before slicing and written back; a
  stale page becomes the last existing page, a still-valid page is preserved, an empty result is
  page 0 with `page_count = 0` and range `0..0`, and `page_size = 0` is the defined all-rows
  mode with `effective_page_size >= 1` (never a division by zero);
- page arithmetic uses `checked_mul` / `checked_add`; a defensive overflow branch produces a
  safe, signalled window instead of a panic, a wrapped index or a silent truncation, and
  `saturating_*` is not used to hide an invalid state;
- every page access goes through `slice::get`; `get_unchecked`, `catch_unwind`, direct indexing
  of `sorted_indices`/`recent_qsos` with a page range and `unwrap`/`expect` on a checked access
  are forbidden;
- selection is keyed by the stable QSO `id` (`INTEGER PRIMARY KEY AUTOINCREMENT`), never by a
  page or filtered index; deleting a record removes exactly that ID and a filtered-out
  selection is never replaced by a different QSO;
- the test suite uses synthetic `TST…` records and an in-memory database only; it never opens
  the production database, the user configuration or a real journal, and it uses no wall clock,
  sleep or retry.

The software remediation does **not** prove any real user database or corrupted database:
`REAL_USER_DATA_USED=false`, `SOFTWARE_ONLY_VERIFICATION=true`. Hardware readiness and hardware
PTT remain `NOT_VERIFIED`, `HIL_EXECUTED=false`, the ten HIL tests stay
`#[ignore = "hardware-in-the-loop"]` and `NATIVE-7B = NOT_STARTED`. F-14, CAT-018, CAT-019,
F-15 and F-17 remain CLOSED.

## 8. Reviewing a security-sensitive change
## 9. Journal-scoped upload selection and upload credentials (P1-F20)

Upload scope is a security boundary, not a convenience: LoTW submissions cannot be deleted, so a
cross-journal submission is an irreversible disclosure. The following rules are enforced in code
and pinned by guards:

1. **Journal scope is mandatory.** Selection, payload read and status write all carry
   `journal_id`; a missing or malformed journal id is a controlled refusal before any transport.
2. **No global selection, no truncation.** Upload selection never uses a global selector, `LIMIT`,
   `OFFSET` or pagination. The complete eligible set is materialised; batching is lossless.
3. **Status only on evidence.** `<service>_qsl_sent = 'Y'` is written only after the service
   confirmed the whole batch, in one transaction, asserting that the updated row count equals the
   batch size (rollback otherwise).
4. **No automatic retry of an upload.** An ambiguous outcome (timeout, unknown body, partial) is
   never laundered into success; retry is explicit and re-derives the eligible set.
5. **Credentials stay in the request body.** eQSL receives `EQSL_USER`/`EQSL_PSWD` as FORM fields
   over HTTPS; the pre-existing LoTW *download* endpoint keeps ARRL's documented query-parameter
   contract (out of F-20's upload scope, recorded as a residual risk).
6. **Redaction at the source.** The eQSL response body is redacted of the password before it
   returns from the client; the upload credentials snapshot has a `Debug` impl that prints
   `<redacted>` for every secret.
7. **No certificate password on the command line.** SPLogbook never passes `-p` to TQSL.
8. **Test doubles are test-scope only.** The loopback server and the endpoint override live in
   `#[cfg(test)]`; there is no environment variable, feature flag, config key or CLI switch that
   can select a test transport, and no automatic fallback.
9. **No real endpoints in tests.** `tests/**` must not reference `lotw.arrl.org` or
   `www.eqsl.cc`.

## 14. Download credentials in a query string (F-20R)

The F-20R security review verified every download path against the current official documentation.
**Both documented download interfaces require credentials in the query string** and document no
alternative (no POST, no Basic auth, no session, no cookie, no API key). Adding one would be an
undocumented protocol invention and is forbidden.

Therefore the rule is not "keep secrets out of the query" but:

1. the credential must never be logged, persisted, put in telemetry or emitted in an error;
2. the URL must never be formatted from a `String` that carries the query — a `SecretUrl` wrapper is
   mandatory, and its `Debug`/`Display` render only `scheme://host/path`;
3. redirects must be disabled on download clients so the query cannot be replayed to another host;
4. transport errors must pass through `sanitize_network_error` before leaving the client;
5. a helper that returns a credential-bearing `String` may exist only under `#[cfg(test)]`.

Any future change to POST, to a session, or to a new endpoint requires a **new official document**
from the service, cited with URL, version, date and access time, and a new architectural decision (ADR). The audit
trail for the current decision is in `docs/LOTW_EQSL_CONTRACT.md`.

`CREDENTIALS_NOT_IN_QUERY=true` must never be claimed while the official contract requires the
query.