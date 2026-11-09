# SPLogbook — MACIERZ ŚLEDZENIA DECYZJI

**Dokument ID:** DSH-SPLOGBOOK-DTM-001
**Data:** 2026-11-09
**Zakres:** decyzje OWNER i ich pokrycie w kodzie, testach i commitach oraz deklarowana intencja OWNER,
której **nie ma jeszcze w repozytorium**.
**Legenda statusu:** `IMPLEMENTED`, `PARTIAL`, `OPEN`, `SUPERSEDED`, `CONTRADICTED`,
`OWNER_INTENT_NOT_YET_IN_REPO`.

> Zasada: wiersz `SUPERSEDED`/`OWNER_INTENT_NOT_YET_IN_REPO` **nie** upoważnia do samodzielnej zmiany
> repozytorium. Zmiana wymaga bramki wskazanej w `docs/NEXT_WORK_GATE.md` (lub jej następcy).

## 1. Decyzje M1 / M2

| ID | Decyzja | Realizacja w kodzie | Test strażniczy | Commit | Status |
|---|---|---|---|---|---|
| OWNER-M2-08 | Retencja `.rollback` — polityka tymczasowa konserwatywna | `src/cloud/updater/transaction.rs` | `tests/security_guard.rs` | `c95172e` | `OPEN` (nieblokujące) |
| OWNER-M2-09 | Natychmiastowe usuwanie kopii zapasowych | brak implementacji | — | — | `OPEN` |
| OWNER-M2-10 | `serviceLOG.db` = FirstInstallSeed | `installer/windows/inno/SPLogbook.iss`, MSI `Components.wxs` | `portable_database_files_are_protected_guard` | `12c3b22` | `IMPLEMENTED` (spór dokumentacyjny: plan „APPROVED”, DESIGN „OPEN”) |
| OWNER-M2-11 | `callbook.db` = FirstInstallSeed | jak wyżej | jak wyżej | `12c3b22` | `IMPLEMENTED` |
| OWNER-M2-12 | Retencja logu audytowego | brak implementacji | — | — | `OPEN` |
| OWNER-M2-13 | `Applying` nieanulowalne | `src/cloud/updater/cancellation.rs` | testy anulowania | `cf77f4a` | `IMPLEMENTED` |
| OWNER-M2-14 | Pobieranie całego pliku (bez resume) | updater | — | — | `IMPLEMENTED` |
| OWNER-M2-15 | Wymagane natywne testy Windows i Linux | CI matrix | — | `dbbe8f5` | `PARTIAL` (kontener ≠ dowód jądra — C-05) |

## 2. Decyzje M3 (Windows)

| ID | Decyzja | Realizacja | Test | Commit | Status |
|---|---|---|---|---|---|
| OWNER-M3-D-TOOLCHAIN-01 | Inno Setup 7.1.1 | `installer/windows/inno/SPLogbook.iss`, `scripts/build_windows_user_setup.ps1` | `tests/windows_inno_setup_test.rs` (14) | `db4417e` | `IMPLEMENTED` |
| OWNER-M3-E-TOOLCHAIN-01 | WiX SDK 7.0.1 | `installer/windows/msi/SPLogbook.wixproj` | `tests/windows_msi_test.rs` (18) | `7f8b040` | `IMPLEMENTED` |
| OWNER-M3-E-MSI-VERSIONING-01 | Wersjonowanie MSI prerelease | brak | matrix w raporcie | — | `OPEN` — blokuje MSI `1.3.0-alpha.2` (błąd 1638) |

## 3. Decyzje M3-F (Linux)

| ID | Decyzja | Realizacja | Test | Commit | Status |
|---|---|---|---|---|---|
| OWNER-M3-F-LINUX-BASELINE-01 | Macierz 7 dystrybucji (Debian, Leap, …) | zastąpiona macierzą 4 wspieranych dystrybucji | `linux_multi_distribution_compatibility_test` | `e5ac2d1`, `28d95cd` | `SUPERSEDED` (2026-11-09) |
| OWNER-M3-F-KERNEL-POLICY-01 | Dynamiczny floor kernela = 6.12 | `scripts/build_linux_portable_tar_gz.sh:179` + snapshot | test dynamicznego floor | `e5ac2d1` | `SUPERSEDED` → OPTION A (dynamiczny **7.0**) |
| OWNER-M3-F-KERNEL-POLICY-OPTION-A | Minimum = najstarsza domyślna linia kernela wspieranych dystrybucji = **7.0**; bez sztywnego 7.2; kanały niestabilne nie wyznaczają minimum | snapshot v2 + `RESOLVED_MIN_KERNEL="7.0"` + macierz CI | `security_guard::m3_f_owner_decisions_and_support_policy_guard`, `linux_multi_distribution_compatibility_test` | bramka 2026-11-09 | `IMPLEMENTED` |
| OWNER-M3-F-GLIBC-POLICY-01 | Minimum glibc **2.43**, maksymalny symbol `GLIBC_2.43`, bez bundled glibc | snapshot v2 + strażnik | jw. | bramka 2026-11-09 | `IMPLEMENTED` |
| OWNER-M3-F-ARCHITECTURE-01 | Wyłącznie x86_64 | marker + manifest | testy manifestu | `e5ac2d1` | `IMPLEMENTED` |
| OWNER-M3-F-LIBRARY-POLICY-01 | Najnowsze stabilne zależności + przecięcie ABI | `Cargo.toml` | `cargo audit` | — | `IMPLEMENTED` |
| OWNER-M3-F-DYNAMIC-LIBRARIES-01 | Bundled `libhamlib.so.4`, RUNPATH `$ORIGIN` | skrypt budowania | audyt ELF w CI | `a983e62` | `IMPLEMENTED` |
| OWNER-M3-F-PORTABLE-LAYOUT-01 | Jeden root `SPLogbook/`, bez linków, 0755/0644 | ekstraktor | 13 testów archiwum | `82da7e4` | `IMPLEMENTED` |
| OWNER-M3-F-UPDATE-MODE-01 | `JOURNALED_TRANSACTIONAL_REPLACEMENT`, `.rollback` | `transaction.rs` | security guards | `ce19b37` | `IMPLEMENTED` |
| OWNER-M3-F-ARCHIVE-EXTRACTION-01 | Dwufazowa ekstrakcja w aplikacji, bez systemowego `tar` | `preflight` + `extract_*_two_phase` | testy archiwum | `82da7e4` | `IMPLEMENTED` (skrypt CI używa `tar -xzf` do fixture — oznaczone) |
| OWNER-M3-F-XDG-DATA-01 | XDG, ścieżki absolutne, bez auto-migracji | `src/core/xdg.rs` | 5 testów XDG | `b974b50` | `IMPLEMENTED` (F-07: precedence portable) |
| OWNER-M3-F-SIGNING-01 | Manifest-first, Ed25519 + SHA-256 + size | `updater.rs` | security guards | — | `IMPLEMENTED` |
| OWNER-M3-F-DESKTOP-INTEGRATION-01 | Opcjonalny szablon `.desktop` | `installer/linux/portable/templates/splogbook.desktop` | 1 test | `03f0dc9` | `IMPLEMENTED` |
| OWNER-M3-F-TEST-MATRIX-01 | Pętle stabilności 100/50/20 | testy | 3 testy pętli | `fffe429` | `IMPLEMENTED` |
| OWNER-M3-F-GIT-WORKFLOW-01 | Historia liniowa, ff-only, niezmienny tag | proces | weryfikacja w raportach | — | `IMPLEMENTED` (potwierdzone) |
| OWNER-M3-F-SCRIPT-STANDARDS-01 | `set -euo pipefail`, deterministyczny tar | skrypt | guard | `13609fd` | `IMPLEMENTED` |
| OWNER-M3-F-ABI-BASELINE-01 | Budowa pod glibc 2.35, forward compatible | — | — | — | `SUPERSEDED` przez minimum 2.43 (`OWNER-M3-F-GLIBC-POLICY-01`) |
| `OWNER-M3-F-F03-01` | **GO** dla `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`: usunięcie fałszywego PASS, biblioteki runtime w 4 obrazach OCI, fail-closed `ldd`/`--version`/`--build-info`, jawna kontrola `GLIBC_2.43`, test negatywny | `scripts/verify_linux_container.sh`, `scripts/install_linux_runtime_deps.sh`, `.github/workflows/build-and-release.yml` | `m3_f_f03_fail_closed_verification_guard`; run `42685105375` | `4699eac`, `abdb767` | `IMPLEMENTED` |

## 4. Polityka Linux zapisana w repozytorium (2026-11-09)

| Element | Wartość | Stan repozytorium | Status |
|---|---|---|---|
| Minimalny kernel | **Linux 7.0**, model dynamiczny (`OWNER-M3-F-KERNEL-POLICY-OPTION-A`) | 7.0 w snapshot/skrypcie/CI | `ZAPISANE` |
| Minimalny glibc | **2.43** | 2.43 w snapshot/AGENTS | `ZAPISANE` |
| Maksymalny wymagany symbol glibc | `GLIBC_2.43` | `GLIBC_2.43` w snapshot | zgodne |
| Debian 13.7 (konfiguracja standardowa) | **NIE WSPIERANY** (glibc 2.41) | `evaluated_not_supported`, brak w CI | `ZGODNE` |
| openSUSE Leap 16.1 | **NIE WSPIERANY** (glibc 2.40) | `evaluated_not_supported`, brak w CI | `ZGODNE` |
| Ubuntu 26.04 LTS | **WSPIERANY** (kernel 7.0, glibc 2.43) — wyznacza minimum | wspierany w snapshot/CI | `ZGODNE` |
| Profile bezpośrednio wspierane | Ubuntu 26.04 LTS, Fedora 45, Arch, Tumbleweed | 4 profile | `ZGODNE` |
| Ubuntu interim 26.10 | **POZA ZAKRESEM** (polityka obejmuje tylko LTS) | usunięty z CI/snapshotu | `ZGODNE` |
| Rebuild do glibc 2.41 / bundled glibc | **NIE** | brak bundled glibc | zgodne |

**Konsekwencja:** M3-F jest domknięte politycznie (`COMPLETED_POLICY_ALIGNMENT_VERIFIED`) oraz
natywnie (`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`, run `42685105375`). M3-G ma status
`READY_FOR_OWNER_GO` / `NOT_STARTED`: implementacja wymaga osobnego GO OWNER.

## 5. Decyzje bramki QA 2026-11-09

| ID | Decyzja | Realizacja | Test | Status |
|---|---|---|---|---|
| QA-F24-01 | Natywny E2E instalatora wyłącznie jawny opt-in | `tests/support/native_artifact.rs`, `tests/inno_user_data_safety_test.rs` | `inno_native_e2e_contract_is_fail_closed`, strażnik `qa_native_installer_tests_are_hermetic_and_fail_closed` | `IMPLEMENTED` |
| QA-F24-02 | Jawna, absolutna ścieżka artefaktu + SHA-256; brak globów/`dist/` | jw. | jw. | `IMPLEMENTED` |
| QA-F24-03 | Przy opt-in wszystkie błędy fail-closed | jw. | jw. | `IMPLEMENTED` |
| QA-F23-01 | Usunięcie zależności od wall-clock w 3 testach | `src/sync/mod.rs` (testy), `src/cloud/updater/lock.rs` (testy) | te same testy + 20 iteracji | `IMPLEMENTED` |
| QA-F23-02 | Niezmiennik „max 1 jednoczesny posiadacz” blokady | `src/cloud/updater/lock.rs` (test) | `concurrent_lock_stress_has_single_winner` | `IMPLEMENTED` |
| QA-CI-01 | Standardowy job nie wymaga instalatora, uruchamia strażnik hermetyczności | `.github/workflows/ci.yml` | strażnik + pełny suite | `IMPLEMENTED` |

## 6. Decyzja o aktualizacji bezpieczeństwa zależności (2026-11-09)

| ID | Decyzja | Realizacja | Dowód | Status |
|---|---|---|---|---|
| `OWNER-DEP-SECURITY-01` | `REMEDIATION: LATEST_STABLE_SAFE_COMPATIBLE_DEPENDENCY_UPDATE`; bez `--ignore`; bez systemowego SQLite; `bundled` wymagany | `Cargo.toml` (`rusqlite 0.41`), `Cargo.lock` (`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`, SQLite 3.54.1) | pełny suite 2 690/2 690; `cargo audit` 0 podatności | `IMPLEMENTED` |
| `OWNER-DEP-SECURITY-02` | Aktualizacja tylko celowa (`cargo update -p rusqlite --precise`), bez ogólnego `cargo update` | 103 zależności bez zmian | `cargo update` output + `Cargo.lock` diff | `IMPLEMENTED` |
| `OWNER-DEP-SECURITY-03` | Push wyłącznie po zielonym `cargo audit` | ff-only + push po audycie | `git rev-list --left-right --count origin/main...main` = `0 0` | `IMPLEMENTED` |

## 7. Ustalenia bramki spójności po zamknięciu M3-F (2026-11-09)

Bramka: `STAGE-M3-F-POST-CLOSURE-CONSISTENCY-VERIFICATION-GATE` —
`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED` (domknięta **jedną sesją**: jeden commit
dokumentacyjny + push ff-only po własnym zielonym baseline). Zakres: dokumentacja + strażnik;
**bez zmian** `src/`, `Cargo.toml`, `Cargo.lock`, workflowów i skryptów.

> Identyfikatory ustaleń mają prefiks `FIND-`, aby **nie kolidowały** z decyzjami OWNER
> `OWNER-M3-F-PCV-01…07` z sekcji 8.

| ID | Ustalenie | Dowód | Status |
|---|---|---|---|
| `FIND-PCV-01` | `scratch/` nie jest ignorowany jako katalog — ignorowane są tylko wzorce plików (`*.log`, `*.py`); `scratch/tree_dupes.txt` jest realnie nieśledzony | `git check-ignore` EXIT 1 (katalog i plik); `git ls-files -o --exclude-standard` | `CORRECTED` |
| `FIND-PCV-02` | Stan drzewa wymaga czterech wymiarów: tracked / untracked / ignored / strict | `AGENTS.md` §2 pkt 4 | `CORRECTED` |
| `FIND-PCV-03` | Run `42518873662` dotyczył **7 profili** i commita `2e40a64`, nie 4 profili | GitHub API `…/runs/42518873662/jobs` → 10 jobów, w tym **7** `Verify Linux` (Ubuntu 26.04.1, Fedora 45, Arch, Tumbleweed, Leap 16.1, Debian 13.7, Ubuntu 26.10) | `CORRECTED` |
| `FIND-PCV-04` | Przebieg 4-profilowy **istnieje**: run `42638736785` (commit `72c3fe6`), 4/4 joby `success` | GitHub API `…/runs/42638736785/jobs` → `head_sha = 72c3fe6…`, `created_at = 2026-11-09T14:51:16Z`, 4 joby `Verify Linux` = `success`, `Publish Release Assets` = `skipped` | `VERIFIED` |
| `FIND-PCV-05` | Logi natywne runu `42638736785` **nie są dowodem PASS**: na **wszystkich czterech** profilach `libasound.so.2 => not found`, binarka nie startuje, a skrypt kończy `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` przy `success` joba | pobrane artefakty `native-evidence-{ubuntu-lts,fedora-final,arch-rolling,opensuse-tumbleweed}` — 4/4 z `libasound.so.2 => not found` i `error while loading shared libraries` | `OPEN` (F-03) |
| `FIND-PCV-06` | F-03 ma formalną bramkę `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` (`OPEN`, `NOT_STARTED`), poprzedzającą M3-G | `docs/NEXT_WORK_GATE.md` §„Następna bramka (jedyna wykonywalna)” | `DEFINED` |
| `FIND-PCV-07` | Run `42518873662` nie jest dowodem jądra dystrybucji (`uname -r` = jądro hosta OCI) | log Ubuntu: `7.0.0-1019-azure` | `CONFIRMED` |
| `FIND-PCV-08` | Chronologia pushu M3-F jest odtwarzalna: push `72c3fe6` → joby runu utworzone `2026-11-09T14:51:16Z` = `15:51:16+01:00`; znacznik `15:51:32+01:00` to koniec sesji, nie „czas raportu” | GitHub API `created_at`; `STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md` §1 i §37.2 | `CORRECTED` |

**Konsekwencja:** M3-G pozostaje `BLOCKED` do zamknięcia F-03. Ta bramka nie zmienia kolejności
etapów M3 i nie otwiera M3-G.

## 8. Decyzje OWNER bramki spójności po zamknięciu M3-F (2026-11-09)

| ID | Decyzja OWNER | Realizacja w repozytorium | Dowód | Status |
|---|---|---|---|---|
| `OWNER-M3-F-PCV-01` | `APPROVE SINGLE-SESSION CLOSURE` — domknięcie bramki przez **dokładnie jedną** sesję | zasada jednej sesji zapisana w `AGENTS.md` §8; ta bramka wykonana jako jedna sesja | `AGENTS.md` §8; raport końcowy tej bramki | `IMPLEMENTED` |
| `OWNER-M3-F-PCV-02` | `STOP THE SECOND PARALLEL SESSION` — druga sesja zatrzymana **przed** testami, edycją, commitem i pushem | brak procesów `cargo`/`rustc`/`git`/`bash`/`sh`; dwa odczyty metadanych bez zmian; brak blokady `target/` | raport końcowy §5–§6 (dwa pomiary + odczyt stabilności) | `IMPLEMENTED` |
| `OWNER-M3-F-PCV-03` | `PRESERVE THE CURRENT WORKTREE` — bez resetu, czyszczenia i stashowania | brak `git reset --hard`, `git clean`, `git restore`, `git stash`; 2 istniejące stashy nietknięte | `git stash list` bez zmian; diff bramki | `IMPLEMENTED` |
| `OWNER-M3-F-PCV-04` | `DO NOT START F-03 IMPLEMENTATION YET` | brak zmian `scripts/verify_linux_container.sh`, workflowów i obrazów OCI; `\|\| true` (linie 46, 49, 50) **nietknięte** | `git diff --name-status` tej bramki | `IMPLEMENTED` |
| `OWNER-M3-F-PCV-05` | `KEEP M3-G BLOCKED` | M3-G `BLOCKED` / `NOT_STARTED` w `NEXT_WORK_GATE.md`, `PROJECT_CURRENT_STATE.md` §4/§8, `AGENTS.md` §5 pkt 25, handoff | sekcje 4 i 7 tej macierzy | `IMPLEMENTED` |
| `OWNER-M3-F-PCV-06` | `AUTHORIZE ONE FINAL DOCUMENTATION/TEST COMMIT AFTER INDEPENDENT GREEN VERIFICATION` | **jeden** commit dokumentacyjny po własnym zielonym baseline (fmt, check, clippy `-D warnings`, pełny test, doc) | raport końcowy §8 i §10 | `IMPLEMENTED` |
| `OWNER-M3-F-PCV-07` | `AUTHORIZE FF-ONLY PUSH TO MAIN AFTER ALL CLOSURE CRITERIA PASS` | `git push origin main` (bez force) po spełnieniu kryteriów; po pushu `HEAD == main == origin/main`, ahead/behind `0/0` | raport końcowy §10 | `IMPLEMENTED` |

## 9. Bramka F-03 — decyzja OWNER i ustalenia (2026-11-09)

Bramka: `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` —
**`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`**. Wykonana jako **jedna sesja**; dowód:
run `42685105375` na commicie `abdb767`, 4/4 profile `success`, job negatywny `success`,
`Publish Release Assets` = `skipped`.

| ID | Ustalenie | Dowód | Status |
|---|---|---|---|
| `OWNER-M3-F-F03-01` | `GO` — implementacja fail-closed i zamknięcie F-03 (decyzja OWNER z promptu bramki) | `docs/NEXT_WORK_GATE.md`, ten wiersz, commit `4699eac`/`abdb767` | `IMPLEMENTED` |
| `FIND-F03-01` | Fałszywy PASS potwierdzony: run `42638736785` (commit `72c3fe6`) — 4/4 joby `success`, ale we wszystkich logach `libasound.so.2 => not found` i `error while loading shared libraries` przy jednoczesnym `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` | artefakty `native-evidence-*` runu `42638736785`; `scratch/evidence_r42638736785/*` | `CONFIRMED` |
| `FIND-F03-02` | Krytyczne maskowanie `|| true` usunięte wyłącznie z `ldd`, `--version`, `--build-info`; pozostałe `|| true` są jawnie oznaczone `NON-CRITICAL` (getconf, `ldd --version`, `PKG_QUERY_CMD`) | `scripts/verify_linux_container.sh`; `m3_f_f03_fail_closed_verification_guard` | `FIXED` |
| `FIND-F03-03` | Biblioteki runtime instalowane w każdym z 4 obrazów przez oficjalny menedżer pakietów przed weryfikacją; 15 sonamów weryfikowanych przez `ldconfig -p`; nazwy potwierdzone w obrazach (m.in. Ubuntu `libusb-1.0-0`, nie `-t64`) | `scripts/install_linux_runtime_deps.sh`; logi instalacji w runie `42685105375` | `FIXED` |
| `FIND-F03-04` | ABI: maksymalny wymagany symbol `GLIBC_2.43` (binarka), polityka `GLIBC_2.43`; dostępne glibc 2.43 (Ubuntu) / 2.44 (Fedora, Arch, Tumbleweed); porównanie semantyczne | logi `Maximum required GLIBC symbol` + `PASS: GLIBC policy satisfied` w 4 profilach | `VERIFIED` |
| `FIND-F03-05` | Test negatywny: goły `ubuntu:26.04` bez bibliotek runtime → `libasound.so.2 => not found`, `FAIL`, **exit 1**, brak komunikatu sukcesu | job `Verify Linux (negative fixture, fail-closed proof)`, log `native-evidence-negative-fixture` | `VERIFIED` |
| `FIND-F03-06` | Publikacja zablokowana: `Publish Release Assets` = `skipped` przy pushu na `main`; brak nowego tagu i release; `v1.2.0` nietknięty | GitHub API jobs/releases/tags; `git rev-parse` | `VERIFIED` |
| `FIND-F03-07` | Anomalia runów na commicie wejściowym `e85ee4e` (CI `42673055954` — 0 jobów; Build and Release `42673056245` — brak joba Windows) to skutek awarii GitHub Actions `major_outage` 18:12–18:43Z, nie defekt repozytorium | `githubstatus.com` summary; API jobs; runy bramki F-03 wykonane w pełni | `EXPLAINED` |
