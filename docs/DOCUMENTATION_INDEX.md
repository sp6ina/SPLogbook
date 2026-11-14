# SPLogbook — Documentation index

Every Markdown document tracked in this repository is listed here. `security_guard`
enforces this: a new document must be added to this index in the same commit that adds it.

Legend: **CURRENT** — current and authoritative; **REFERENCE** — stable technical reference;
**EVIDENCE** — immutable historical record that must not be edited to look current.

---

## 1. Project entry points

| Path | Role | Status |
|---|---|---|
| `README.md` | product overview, features, requirements, installation, licensing | CURRENT |
| `CHANGELOG.md` | public release history and unreleased development line | CURRENT |
| `CONTRIBUTING.md` | contribution workflow, required checks, test and code rules | CURRENT |
| `SECURITY.md` | security policy, reporting, guarantees, dependency policy | CURRENT |

The full license text is in `LICENSE` (GPL-3.0); `LICENSE` is not Markdown and is therefore
not listed in this index.

## 2. Maintainer documentation

| Path | Role | Status |
|---|---|---|
| `docs/DEVELOPMENT.md` | toolchain pins, build, test baseline, packaging commands, git workflow, documentation policy | CURRENT |
| `docs/VERIFICATION_BASELINE.md` | single source of truth for verified test counters, audit state and measurement date | CURRENT |
| `docs/ROADMAP.md` | public roadmap: delivered milestones, closed risks, not-started work | CURRENT |
| `docs/KNOWN_ISSUES.md` | open defects, open decisions, not-implemented areas, environment limits | CURRENT |
| `docs/SECURITY_PROCESS.md` | maintainer security process: audit policy, key handling, fail-closed rules, CI controls | CURRENT |
| `docs/DOCUMENTATION_INDEX.md` | this index | CURRENT |

## 3. Architecture

| Path | Role | Status |
|---|---|---|
| `docs/architecture/ARCHITECTURE.md` | module map, entry points, data flows, security boundaries, test map | CURRENT |
| `docs/architecture/DECISIONS.md` | binding owner decisions, architectural invariants, decision → code → guard traceability | REFERENCE |
| `docs/architecture/THEMES.md` | semantic palette and theme subsystem, theme guard contract | CURRENT |
| `DESIGN_3_0_A_WINDOWS_UPDATE.md` | approved Windows installer and updater design | REFERENCE |
| `DESIGN_3_0_B_LINUX_UPDATE.md` | approved Linux installer and updater design (see the errata banner at the top for superseded values) | REFERENCE |
| `DESIGN_3_0_C_PLATFORM_INSTALLERS.md` | approved cross-platform installer contract and update state machine | REFERENCE |

## 4. Security

| Path | Role | Status |
|---|---|---|
| `docs/security/RELEASE_SECURITY.md` | release supply chain: manifest format, Ed25519 trust, archive hardening, journaled replacement, CI release flow | REFERENCE |
| `docs/security/SECURITY_KEY_ROTATION.md` | SEC-001 incident record, key rotation runbook, emergency kill-switch, user migration | REFERENCE |
| `docs/security/HISTORICAL_RELEASE_AUDIT.md` | per-release trust audit of every published GitHub release | EVIDENCE |

## 5. Platform support and data

| Path | Role | Status |
|---|---|---|
| `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md` | human-readable Linux support snapshot: kernel and glibc minimums, supported and evaluated distributions | CURRENT |
| `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json` | machine-readable snapshot (`splogbook-linux-support-snapshot-v2`), validated by `security_guard` | CURRENT |
| `docs/DATABASE_BACKUP_RECOVERY.md` | SQLite backup, validation, restore and operator recovery runbook | CURRENT |
| `docs/HARDWARE_VALIDATION_MATRIX.md` | CAT protocol knowledge, per-backend hardware requirements, HIL preconditions | CURRENT |

## 6. Radio control (Native CAT)

| Path | Role | Status |
|---|---|---|
| `Native CAT.md` | native CAT architecture: backends, PTT invariants, HIL harness, RF safety | CURRENT |

## 7. Packaging and installers

| Path | Role | Status |
|---|---|---|
| `installer/linux/portable/README.md` | portable `tar.gz` layout, extraction rules, XDG data placement | CURRENT |
| `installer/linux/appimage/README.md` | AppImage toolchain pins, AppDir layout, verification and FUSE proof | CURRENT |
| `installer/linux/deb/README.md` | DEB package contract, `Depends`, layout, lifecycle, negative fixtures | CURRENT |
| `installer/linux/rpm/README.md` | RPM package contract, explicit dependencies, RUNPATH, lifecycle, negative fixtures | CURRENT |
| `installer/windows/inno/README.md` | Inno Setup user setup: approved toolchain, identity, input allowlist, uninstall safety | CURRENT |
| `installer/windows/msi/README.md` | WiX MSI: toolchain, identity, components, delegation, conflict detection | CURRENT |

## 8. Localization

| Path | Role | Status |
|---|---|---|
| `locales/README.md` | translation catalog structure, keys, and the i18n guard contract | CURRENT |

## 9. Documents intentionally not in this repository

Working material — prompts, model handoffs, session notes, gate evidence logs, owner-local
plans and pre-implementation reports — is kept outside the repository. It is never tracked,
never published, and is excluded locally through `.git/info/exclude`, which is itself not
tracked.

If a document referenced from an older report is not listed in this index, it does not exist
in this repository. Durable technical content that used to live only in such reports has been
consolidated into the CURRENT and REFERENCE documents above.

## 10. Maintenance rules

1. A new document is added to this index in the same commit.
2. `CURRENT` claims must be verifiable against the code; a document never overrides code.
3. Counters appear only in `docs/VERIFICATION_BASELINE.md` and always carry a measurement date.
4. `EVIDENCE` documents are not rewritten to look current.
5. No tracked document links to a path that is not tracked.
