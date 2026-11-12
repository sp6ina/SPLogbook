# SPLogbook — INDEKS DOKUMENTACJI

**Dokument ID:** DSH-SPLOGBOOK-DOCIDX-001
**Data:** 2026-11-09
**Zakres:** wszystkie śledzone dokumenty Markdown w repozytorium (stan `abdb767` + bramka F-03).
**Zasada:** ten plik jest **wyłącznie indeksem** i nie tworzy konkurencyjnego źródła prawdy.

Legenda statusu: `CURRENT` — aktualny; `PARTIAL` — częściowo nieaktualny (wskazano obszar);
`HISTORICAL` — zapis historyczny, nie źródło bieżącego stanu; `SUPERSEDED` — zastąpiony decyzją OWNER;
`EVIDENCE` — raport dowodowy zamkniętej bramki.

## 1. Dokumenty nadrzędne

| Ścieżka | Rola | Status | Uwagi / korekty |
|---|---|---|---|
| `AGENTS.md` | obowiązkowe zasady operacyjne agentów | `CURRENT` | nowy w bramce QA 2026-11-09 |
| `README.md` | opis produktu, funkcje, status | `PARTIAL` | sekcja Linux (ok. linii 531) nadal opisuje zastąpioną politykę — do korekty w bramce polityki Linux (DOC-DSH-04) |
| `MASTER_IMPLEMENTATION_PLAN.md` | kolejność i zależności etapów | `PARTIAL` | blok „Informacje o środowisku” (MSRV 1.85, wersja 1.1.0, „brak MSI”) jest nieaktualny (DOC-DSH-03) |
| `docs/PROJECT_CURRENT_STATE.md` | jednostronicowy stan prawdy | `CURRENT` | generowany przy zamknięciu każdej bramki |
| `docs/NEXT_WORK_GATE.md` | dokładnie jedna następna bramka | `CURRENT` | wskazuje `M3-H` (DEB, `NOT_STARTED`); M3-G `CLOSED`; F-03 `CLOSED` |
| `docs/PROJECT_ARCHITECTURE_MAP.md` | mapa modułów i przepływów | `CURRENT` | zastępuje `docs/ARCHITECTURE_CURRENT_STATE.md` jako opis bieżący |
| `docs/DEEPSEEK_HARNESS_HANDOFF.md` | raport przejęcia Antigravity → DSH | `CURRENT` | operacyjne podsumowanie raportu ingestii |
| `docs/DEEPSEEK_HARNESS_SESSION_BOOTSTRAP.md` | procedura startu sesji | `CURRENT` | |
| `docs/DEEPSEEK_HARNESS_REQUIRED_CAPABILITIES.md` | wymagane zdolności harnessu | `CURRENT` | |
| `docs/DOCUMENTATION_INDEX.md` | ten indeks | `CURRENT` | |
| `docs/DECISION_TRACEABILITY_MATRIX.md` | decyzja → kod → test → commit | `CURRENT` | zawiera wiersze `OWNER_INTENT_NOT_YET_IN_REPO` |
| `DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md` | pełny raport ingestii i audytu przejęcia | `EVIDENCE` | dodany do repo w bramce QA; nie zmieniać ustaleń bez dowodu |

## 2. Dokumenty projektowe (DESIGN)

| Ścieżka | Rola | Status |
|---|---|---|
| `DESIGN_3_0_A_WINDOWS_UPDATE.md` | architektura aktualizacji Windows (ZIP, staging, rollback, health) | `CURRENT` |
| `DESIGN_3_0_B_LINUX_UPDATE.md` | architektura aktualizacji Linux | `PARTIAL` — limity/glibc/layout i polityka dystrybucji do korekty (DOC-DSH-10), `SUPERSEDED` w części „glibc 2.35” i macierzy 7 dystrybucji |
| `DESIGN_3_0_C_PLATFORM_INSTALLERS.md` | wspólny model instalatorów platformowych | `CURRENT` (adnotacja o braku rozszerzenia manifestu — DOC-DSH-11) |

## 3. Rejestry i plany naprawcze

| Ścieżka | Rola | Status |
|---|---|---|
| `docs/AUDIT_FINDINGS_REGISTER.md` | rejestr znalezisk audytowych SEC/DATA/FUNC/CAT/ARCH/PERF/GUI/TEST/DOC/CI/DEP | `PARTIAL` — nie zawiera jeszcze F-23/F-24/F-25 z raportu ingestii; liczniki historyczne |
| `docs/IMPROVEMENT_ROADMAP.md` | roadmapa 10 etapów i kategorii zmian | `PARTIAL` — Etap 1-10 opisany historycznie; polityka Linux do korekty |
| `docs/REMEDIATION_BASELINE.md` | baza pomiarowa programu naprawczego | `PARTIAL` — liczniki z 2026-10-28/11-03 (1 519 / 1 874), nieaktualne wobec 2 698 |
| `docs/TEST_GAP_ANALYSIS.md` | analiza luk testowych | `HISTORICAL` |
| `docs/FULL_PROGRAM_AUDIT.md` | audyt całego programu | `HISTORICAL` |
| `docs/HISTORICAL_RELEASE_AUDIT.md` | audyt wydań historycznych | `HISTORICAL` |
| `docs/ARCHITECTURE_CURRENT_STATE.md` | opis architektury z `6f7d3aa` | `HISTORICAL` — nazywa pliki, które już nie istnieją (DOC-DSH-06) |
| `docs/ARCHITECTURE_TARGET_STATE.md` | architektura docelowa | `HISTORICAL` |
| `docs/ARCHITECTURE_THEMES.md` | motywy GUI | `HISTORICAL` |
| `docs/RELEASE_SECURITY.md` | bezpieczeństwo wydań | `PARTIAL` — §5 opisuje `.backup_<id>` zamiast `JOURNALED_TRANSACTIONAL_REPLACEMENT` (DOC-DSH-08) |
| `docs/DATABASE_BACKUP_RECOVERY.md` | backup/odzyskiwanie SQLite | `PARTIAL` — nazwa `splogbook.db` vs `default_log.db` (DOC-DSH-09) |
| `docs/SECURITY_KEY_ROTATION.md` | rotacja kluczy | `CURRENT` |
| `Native CAT.md` | dokumentacja CAT/HIL | `PARTIAL` — liczniki testów historyczne (DOC-DSH-01) |

## 4. Raporty bramek (EVIDENCE)

| Ścieżka | Bramka | Status |
|---|---|---|
| `STAGE_9_EXECUTION_REPORT.md`, `STAGE_9_FINAL_GATE_REPORT.md` | etap 9 | `EVIDENCE` |
| `STAGE_10_EXECUTION_REPORT.md`, `STAGE_10_FINAL_GATE_REPORT.md` | etap 10 | `EVIDENCE` |
| `STAGE_M1_COVERAGE_AND_STATUS_REPORT.md` | M1 | `EVIDENCE` |
| `STAGE_M2_INSTALLER_DESIGN_GATE_REPORT.md`, `STAGE_M2_OWNER_DECISIONS_APPROVAL_REPORT.md` | M2 | `EVIDENCE` |
| `STAGE_M3_A_*` | M3-A | `EVIDENCE` |
| `STAGE_M3_B_*` (+ duplikaty w `docs/`) | M3-B | `EVIDENCE` — duplikaty do usunięcia (DOC-DSH-15) |
| `STAGE_M3_C_*` | M3-C | `EVIDENCE` |
| `STAGE_M3_D_*` | M3-D | `EVIDENCE` — §6.3 deklaruje E2E z `/NOICONS` (parametr niedziałający przy `DisableProgramGroupPage=yes`); nie wpływa na ważność implementacji |
| `STAGE_M3_E_*` | M3-E | `EVIDENCE` |
| `STAGE_M3_DEV_VERSION_*` | M3-DEV | `EVIDENCE` |
| `STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_*` | M3-F | `EVIDENCE` — polityka dystrybucji `SUPERSEDED` (errata 2026-11-09) |
| `STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_*` | M3-F native | `EVIDENCE` — dowód userspace/CI, nie dowód jądra (errata 2026-11-09) |
| `STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_PRE_IMPLEMENTATION_REPORT.md` | M3-F policy | `EVIDENCE` — raport wstępny |
| `STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md` | M3-F policy | `EVIDENCE` — `COMPLETED_POLICY_ALIGNMENT_VERIFIED`; errata 2026-11-09 (klasyfikacja `scratch/`, chronologia pushu, klasyfikacja dowodu natywnego) |
| `STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_PRE_IMPLEMENTATION_REPORT.md` | M3-F post-closure consistency | `EVIDENCE` — raport wstępny |
| `STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_REPORT.md` | M3-F post-closure consistency | `EVIDENCE` — `COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED` (domknięta **jedną sesją**: commit + push ff-only); definiuje bramkę F-03; liczniki wyłącznie z własnego pomiaru tej sesji |
| `STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_PRE_IMPLEMENTATION_REPORT.md` | M3-F F-03 | `EVIDENCE` — raport wstępny (analiza, plan, kryteria) |
| `STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_REPORT.md` | M3-F F-03 | `EVIDENCE` — `COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`; run `42685105375` (4/4 profile, zero `not found`, GLIBC ≤ 2.43) + test negatywny |
| `STAGE_M3_G_LINUX_APPIMAGE_PRE_IMPLEMENTATION_REPORT.md` | M3-G | `EVIDENCE` — raport przedimplementacyjny z sekcją „M3-G APPIMAGETOOL CONTINUOUS RESOLUTION UPDATE” |
| `STAGE_M3_G_LINUX_APPIMAGE_BLOCKED_REPORT.md` | M3-G | `EVIDENCE` — `BLOCKED_NEW_DEPENDENCY_ADVISORY` (`RUSTSEC-2026-0486`, `rustix 1.1.5`); implementacja i weryfikacja AppImage wykonane |
| `STAGE_M3_G_LINUX_APPIMAGE_INTEGRATION_REPORT.md` | M3-G | `EVIDENCE` — `COMPLETED_APPIMAGE_INTEGRATION_VERIFIED` (4/4 profile, 12/12 negatywnych, FUSE `VERIFIED`) |
| `STAGE_DEPENDENCY_RUSTIX_RUSTSEC_2026_0486_REMEDIATION_REPORT.md` | dependency | `EVIDENCE` — `COMPLETED_DEPENDENCY_ADVISORY_REMEDIATED` (`rustix 1.1.6`, `cargo audit` EXIT 0) |
| `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_PRE_IMPLEMENTATION_REPORT.md` | QA baseline | `EVIDENCE` |
| `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md` | QA baseline | `EVIDENCE` — `COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED` |
| `STAGE_QA_DEPENDENCY_ADVISORY_DECISION_PROPOSAL.md` | propozycja decyzji OWNER dla RUSTSEC-2026-0459 | `EVIDENCE` — rozstrzygnięta decyzją `OWNER-DEP-SECURITY-01` |
| `STAGE_DEPENDENCY_SECURITY_UPDATE_PRE_IMPLEMENTATION_REPORT.md` | bramka aktualizacji zależności — raport wstępny | `EVIDENCE` |
| `STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md` | bramka aktualizacji zależności — raport końcowy | `EVIDENCE` — `COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED` |
| `UPDATE_ARTIFACT_INVENTORY_REPORT.md`, `UPDATE_PRE_IMPLEMENTATION_REPORT.md`, `UPDATE_TRUST_AND_DATA_SAFETY_GATE_REPORT.md` | tor UPDATE | `EVIDENCE` |

## 5. Propozycje decyzji OWNER (nierozstrzygnięte lub zastąpione)

| Ścieżka | Status |
|---|---|
| `OWNER-M3-F-LINUX-BASELINE-01-PROPOSAL.md` | `SUPERSEDED` przez politykę OWNER z 2026-11-09: `OWNER-M3-F-KERNEL-POLICY-OPTION-A` (dynamiczne minimum **7.0**) + `OWNER-M3-F-GLIBC-POLICY-01` (**2.43**), bez Debian/Leap |
| `STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md` | `SUPERSEDED` w części macierzy dystrybucji i modelu kernela |
| `STAGE_M3_E_MSI_PRERELEASE_VERSIONING_DECISION_PROPOSAL.md` | `OPEN` — blokuje MSI dla `1.3.0-alpha.2` |
| `STAGE_M3_E_WIX_TOOLCHAIN_DECISION_PROPOSAL.md` | `APPROVED` (`OWNER-M3-E-TOOLCHAIN-01`) |

## 6. Dokumenty lokalne, których NIE MA w repozytorium

Wymieniane w starszych dokumentach jako źródła, ale **nieobecne** (nie szukaj ich, nie cytuj jako istniejących):
`AUDIT_REPORT.md`, `PLAN.md`, `CODE_QUALITY_NORMALIZATION_PLAN.md`, `DESIGN_2_2_FINAL.md`,
`DESIGN_3_1_UPDATER_SIGNATURES.md`, `REPORT_3_1_A_GITHUB_DIGEST.md`,
`REPORT_CODE_QUALITY_P0_VERIFICATION.md`, `DEAD_CODE_VERIFICATION_REPORT.md`.
Jeżeli decyzja z nich jest nadal wiążąca — musi być powtórzona inline w dokumencie bieżącym.

## 7. Zasady utrzymania indeksu

- Nowy dokument musi zostać dopisany tutaj w tym samym commicie, w którym powstaje.
- Dokument `HISTORICAL` nie może być cytowany jako dowód bieżącego stanu.
- Raport bramki nie nadpisuje kodu; kod nie nadpisuje decyzji OWNER.
- Liczniki testów podaje się **wyłącznie** z pomiarem `cargo test -- --list` i datą pomiaru.
