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

## 10. Bramka M3-G — AppImage (2026-11-11)

Bramka: `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE` — **`IN_PROGRESS` /
`BLOCKED_NEW_DEPENDENCY_ADVISORY`**. Implementacja i weryfikacja AppImage zostały wykonane
w jednej sesji; formalne zamknięcie blokuje nowe advisory zależności (`RUSTSEC-2026-0486`).

| ID | Decyzja / ustalenie | Realizacja | Dowód | Status |
|---|---|---|---|---|
| `OWNER-M3-G-01` | `GO` dla bramki AppImage | `docs/NEXT_WORK_GATE.md`, ten wiersz | raport bramki | `IMPLEMENTED` |
| `OWNER-M3-G-03-R1` | `APPROVE LATEST VERIFIED APPIMAGETOOL CONTINUOUS BUILD` | `scripts/install_appimage_toolchain.sh` | `continuous` @ `11d54c9acc7f246df1398050a4b880314ac76862`, `sha256:e8c1169c…` | `IMPLEMENTED` |
| `OWNER-M3-G-04-R1` | `RESOLVE AND VERIFY THE LATEST OFFICIAL TYPE-2 RUNTIME` | `scripts/install_appimage_toolchain.sh`, `--runtime-file` | `continuous` @ `fedfad51acc2860b2916a1eef32ba4a500ff31ef`, `sha256:617aa773…` | `IMPLEMENTED` |
| `OWNER-M3-G-08` | `USE LATEST VERIFIED APPIMAGETOOL CONTINUOUS BUILD` | brak fallbacku 1.9.1 | `m3_g_appimage_integration_guard` | `IMPLEMENTED` |
| `OWNER-M3-G-09` | `PREFER LATEST AVAILABLE LIBRARIES AND TOOLS` | najnowszy zweryfikowany runtime i tool | metadane toolchainu | `IMPLEMENTED` |
| `FIND-M3G-01` | Oficjalny digest i lokalny digest `appimagetool` identyczne; asset jednoznaczny | `appimage-toolchain.json` | `sha256:e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` | `VERIFIED` |
| `FIND-M3G-02` | Runtime type 2 statycznie linkowany (brak `PT_INTERP`, `DT_NEEDED`, `GLIBC_*`), przekazany jawnie `--runtime-file`; wcześniejszy pin `20251108` odnotowany, DESIGN go nie wymaga | `runtime-x86_64`, `sha256:617aa773…` | analiza ELF + metadane | `VERIFIED` |
| `FIND-M3G-03` | AppImage zbudowany z zweryfikowanego portable tar.gz tego samego commita; zatwierdzony AppDir, marker, RUNPATH `$ORIGIN/../lib`, wyłącznie `libhamlib.so.4` | `scripts/build_linux_appimage.sh`, `AppRun` | artefakt CI `SPLogbook-Linux-x86_64-appimage` | `VERIFIED` |
| `FIND-M3G-04` | Testy negatywne: 12 mutacji odrzuconych niezerowym kodem i bez komunikatu sukcesu | `scripts/verify_appimage_negative_fixture.sh` | job `verify-appimage-negative-fixtures` | `VERIFIED` |
| `FIND-M3G-05` | Cztery profile weryfikują **ten sam** AppImage (bez przebudowy per profil) | `.github/workflows/build-and-release.yml` | joby `Verify Linux (…)` | `VERIFIED` |
| `FIND-M3G-06` | Ekstrakcja nie jest dowodem FUSE; job FUSE raportuje rzeczywisty status (`VERIFIED` albo `UNAVAILABLE_IN_CI`), nigdy nie oznacza braku jako PASS | job `verify-appimage-fuse` | log `appimage-evidence-fuse` | `DEFINED` |
| `FIND-M3G-07` | Defekt ratchetu liczników CI (`2700/2690`) ujawniony przez run `43011245883`; poprawiony bez osłabiania asercji bezpieczeństwa | `tests/security_guard.rs`, commit `eb414e9` | run `43015927219` (fmt, clippy, testy zielone) | `FIXED` |
| `FIND-M3G-08` | **Nowe advisory** `RUSTSEC-2026-0486` (`rustix 1.1.5`, `informational = unsound`, patch `>= 1.1.6`) wykryte przez `cargo audit --deny warnings`; zgodnie z §27 zatrzymuje M3-G i nie jest naprawiane przy okazji | `cargo audit`, job `CI` → `Install and run cargo audit` = `failure` | run `43015927219` | `BLOCKED` |
| `FIND-M3G-09` | `publish-release` nietknięty, `Publish Release Assets` = `skipped`, brak tagu i release | workflow, GitHub API | run `43015927112` | `VERIFIED` |

## 11. Bramka remediacji RUSTSEC-2026-0486 (2026-11-12)

Bramka: `STAGE-DEPENDENCY-RUSTIX-RUSTSEC-2026-0486-REMEDIATION-GATE` —
**`COMPLETED_DEPENDENCY_ADVISORY_REMEDIATED`**. Wykonana jako **jedna sesja** po decyzji
`OWNER-DEP-RUSTIX-01` (`APPROVE OPTION A`).

| ID | Decyzja / ustalenie | Realizacja | Dowód | Status |
|---|---|---|---|---|
| `OWNER-DEP-RUSTIX-01` | `APPROVE OPTION A` — celowana aktualizacja `rustix 1.1.5 → 1.1.6`, bez `--ignore`, bez szerokiego `cargo update`, bez zmian `Cargo.toml` i kodu AppImage | `Cargo.lock` | raport bramki | `IMPLEMENTED` |
| `OWNER_GO_M3_I_SECURITY_REMEDIATION` | `GO` — osobna bramka `STAGE-M3-I-SEC`: celowana aktualizacja `quinn-proto 0.11.19` i `quinn-udp 0.5.16`, bez `--ignore`/suppression, bez zmian `Cargo.toml` i `src/**`, bez M3-J, bez tagu/release | `Cargo.lock`, `STAGE_M3_I_RUSTSEC_REMEDIATION_*` | `cargo audit` EXIT 0; `CI` `43485657304`; `Build and Release` `43485657283` | `IMPLEMENTED` |
| `FIND-DEP-RUSTIX-01` | Rzeczywista ścieżka `rustix 1.1.5`: `async-io`/`calloop`/`gethostname`/`polling`/`tempfile`/`wayland-*`/`x11rb`/`xattr`/`zbus` ← `splogbook` | `cargo tree -i rustix@1.1.5 --target all` | logi sesji | `VERIFIED` |
| `FIND-DEP-RUSTIX-02` | `1.1.6` jest najnowszą stabilną wersją (crates.io, 2026-10-27), MSRV `1.65`, patch w linii `1.x` | crates.io API | `LATEST_STABLE=1.1.6` | `VERIFIED` |
| `FIND-DEP-RUSTIX-03` | Lockfile: 674 → 674 paczek; usunięto wyłącznie `rustix 1.1.5`, dodano wyłącznie `rustix 1.1.6` (nowy checksum `e5e76742…`); żadna inna wersja paczki się nie zmieniła | `Cargo.lock` | analiza TOML przed/po | `VERIFIED` |
| `FIND-DEP-RUSTIX-04` | `rustix 0.38.44` **nietknięty** (blok identyczny) | `Cargo.lock` | analiza TOML | `VERIFIED` |
| `FIND-DEP-RUSTIX-05` | Efekt resolvera: dla 10 paczek wyłącznie windowsowych cargo wybrał inne (już obecne) wersje `windows-sys`; bez zmian wersji tych paczek i bez wpływu na Linux | `Cargo.lock` | analiza TOML | `DOCUMENTED` |
| `FIND-DEP-RUSTIX-06` | `cargo audit --deny warnings --no-yanked <7 dozwolonych ignore>` = **EXIT 0** (1458 advisory, 674 zależności); `RUSTSEC-2026-0486` nie występuje; nie dodano żadnego `--ignore` | `cargo audit` | log audytu 2026-11-12 | `VERIFIED` |
| `FIND-DEP-RUSTIX-07` | Pełny baseline po aktualizacji zielony: fmt, check, clippy `-D warnings`, testy **2717/2707/2707/0/10**, doc, tree | 20 targetów | log baseline 2026-11-12 | `VERIFIED` |
| `FIND-DEP-RUSTIX-08` | M3-G odblokowane: `M3_G_STATUS` = `IN_PROGRESS`, `M3_G_BLOCKER` = `NONE`; zamknięcie M3-G nadal wymaga pełnych dowodów CI (4/4 profile, job negatywny, FUSE) | `docs/NEXT_WORK_GATE.md`, `docs/PROJECT_CURRENT_STATE.md` | ten raport | `DEFINED` |

## 12. Zamknięcie bramki M3-G — AppImage (2026-11-12)

Bramka: `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE` —
**`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`**. Wznowiona i zamknięta po usunięciu blokera
zależności (`OWNER-DEP-RUSTIX-01`), w jednej sesji.

| ID | Ustalenie | Dowód | Status |
|---|---|---|---|
| `FIND-M3G-10` | Toolchain: `appimagetool` continuous `11d54c9…` (`sha256:e8c1169c…`) + runtime `fedfad51…` (`sha256:617aa773…`); digesty oficjalne = lokalne | `appimage-toolchain.json` w artefakcie CI | `VERIFIED` |
| `FIND-M3G-11` | AppImage `15 583 736` B, `sha256:8eb88eaa…`, `build_commit 35464c7…`, wymagany `GLIBC_2.43`, tylko `libhamlib.so.4`, RUNPATH `$ORIGIN/../lib`, marker `LinuxAppImage` | artefakt `SPLogbook-Linux-x86_64-appimage` | `VERIFIED` |
| `FIND-M3G-12` | 4/4 profile `success` (Ubuntu 26.04.1, Fedora 45, Arch, Tumbleweed): `--version`, `--build-info` (`Commit: 35464c7…`), `ldd` bez `not found`, GLIBC ≤ 2.43, XDG, immutability, `APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY` | run `43166825054`, logi `appimage-evidence-*` | `VERIFIED` |
| `FIND-M3G-13` | Testy negatywne: **12/12** mutacji odrzuconych (exit 1, bez komunikatu sukcesu) | `negative-fixtures.log` | `VERIFIED` |
| `FIND-M3G-14` | FUSE: `FUSE_STATUS=VERIFIED`, rzeczywisty mount, `Commit: 35464c7…`; ekstrakcja nie jest dowodem FUSE | job `verify-appimage-fuse` | `VERIFIED` |
| `FIND-M3G-15` | Publikacja: `Publish Release Assets` = `skipped`; brak nowego tagu i release; `v1.2.0` nietknięty | run `43166825054`, GitHub API | `VERIFIED` |
| `FIND-M3G-16` | Baseline i audyt: `CI` run `43166825036` = `success` (fmt, clippy, testy 2717/2707, `cargo audit` EXIT 0) | run `43166825036` | `VERIFIED` |
| `FIND-M3G-17` | Następna bramka `M3-H` (DEB) — `NOT_STARTED`; nie rozpoczęto prac DEB/RPM/Flatpak | `docs/NEXT_WORK_GATE.md` | `DEFINED` |

## 13. Zamknięcie bramki M3-H — Linux DEB amd64 (2026-11-12)

Bramka: `STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE` —
**`COMPLETED_DEB_INTEGRATION_VERIFIED`**. Jedna sesja, commit implementacyjny
`236b7ea`, korekty `f9bab2e` / `af04b13` / `3171aa8`.

| ID | Decyzja / ustalenie | Realizacja | Dowód | Status |
|---|---|---|---|---|
| `OWNER-M3-H-01` | APPROVE BINARY DEB (bez source package, bez debhelper) | `scripts/build_linux_deb.sh` | brak `debian/`, brak `.dsc` | `IMPLEMENTED` |
| `OWNER-M3-H-02` | APPROVE DPKG-DEB z oficjalnego repo przypiętego obrazu | `build-deb` job, obraz `ubuntu:26.04@sha256:6a06111c…` | `deb-toolchain.json` | `VERIFIED` |
| `OWNER-M3-H-03` | APPROVE `/opt/SPLogbook` + launcher + desktop + ikona + marker | builder + verifier | `deb-contents.txt`, log Verify DEB | `VERIFIED` |
| `OWNER-M3-H-04` | APPROVE FAIL-CLOSED LAUNCHER | `installer/linux/deb/launcher` | `m3_h_deb_integration_guard`, log weryfikatora | `VERIFIED` |
| `OWNER-M3-H-05` | APPROVE GENERATED-THEN-REVIEWED DEPENDS | `templates/control` + weryfikator | `Depends` 17 pakietów, `dpkg -S`/`ldd` w CI | `VERIFIED` |
| `OWNER-M3-H-06` | APPROVE MINIMAL MAINTAINER SCRIPTS | `templates/postinst`, `templates/postrm` | brak sieci/sudo/HOME; `sh -n` + skan tokenów | `VERIFIED` |
| `OWNER-M3-H-07` | APPROVE SYSTEM-WIDE INSTALL WITH USER-DATA PRESERVATION | verifier lifecycle | `PASS: user data unchanged after: final purge` | `VERIFIED` |
| `OWNER-M3-H-08` | NO PACKAGE SIGNING IN M3-H (tylko SHA-256, bez APT/release) | builder sidecary | `.deb.sha256`, brak `Release`/`InRelease` | `VERIFIED` |
| `OWNER-M3-H-09` | CI ARTIFACT ONLY (bez release, bez tagu) | workflow `SPLogbook-Linux-amd64-deb` | `Publish Release Assets` = `skipped` | `VERIFIED` |
| `OWNER-M3-H-10` | APPROVE PACKAGE IDENTITY (`splogbook`, `amd64`, `1.3.0~alpha.1-1`) | control + weryfikator | `deb-fields.txt`, `dpkg --compare-versions` | `VERIFIED` |
| `FIND-M3H-01` | Deterministyczna budowa: dwie niezależne budowy `dpkg-deb --root-owner-group -Zxz` mają identyczny SHA-256 | builder | `deb-build.log` (`reproducible build`) | `VERIFIED` |
| `FIND-M3H-02` | Poprawny porządek wersji: `1.3.0~alpha.1-1` < `1.3.0-1`, > `1.3.0~alpha.0-1` | builder/verifier | `dpkg --compare-versions`, downgrade test | `VERIFIED` |
| `FIND-M3H-03` | Bundlowany wyłącznie `libhamlib.so.4` (0644); glibc/loader/systemowe biblioteki niebundlowane | builder/verifier | `deb-contents.txt` | `VERIFIED` |
| `FIND-M3H-04` | `GLIBC_2.43` ≤ polityka dla SPLogbook i libhamlib; updater 2.39 | verifier | log Verify DEB | `VERIFIED` |
| `FIND-M3H-05` | Lintian: **0 errors**, 2 warnings; suppression wyłącznie `dir-or-file-in-opt,custom-library-search-path` (OWNER-M3-H-03) | verifier | log lintian w `deb-verify.log` | `VERIFIED` |
| `FIND-M3H-06` | Pełny cykl życia PASS: fresh install, upgrade, downgrade, re-upgrade, reinstall, remove, purge; dane XDG SHA-256 identyczne | verifier | `deb-verify.log` | `VERIFIED` |
| `FIND-M3H-07` | Testy negatywne: **27/27** mutacji odrzuconych, exit 1, bez komunikatu sukcesu | negative harness | `deb-negative-fixtures.log` | `VERIFIED` |
| `FIND-M3H-08` | Portable tar.gz, AppImage i F-03 bez regresji; 4 profile Linux i 2 joby AppImage = `success` | workflow | run `43245181565` | `VERIFIED` |
| `FIND-M3H-09` | Baseline: 2735 zarejestrowanych / 2725 wykonanych / 2725 zaliczonych / 0 failed / 10 ignored; `security_guard` 124/124; `cargo audit` EXIT 0 | baseline sesji | logi `scratch/m3h_full_test*.log` | `VERIFIED` |
| `FIND-M3H-10` | Następna bramka `M3-I` (RPM) — `NOT_STARTED`; M3-J nadal `NOT_STARTED` | `docs/NEXT_WORK_GATE.md` | ten dokument | `DEFINED` |

## M3-I — RPM (2026-11-13, `BLOCKED`)

| ID | Decyzja / ustalenie | Realizacja | Dowód | Status |
|---|---|---|---|---|
| `FIND-M3I-01` | RPM `splogbook-1.3.0-0.1.alpha.1.x86_64`, artefakt `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` + alias | spec + builder | `rpm-fields.txt` | `VERIFIED` |
| `FIND-M3I-02` | Układ `/opt/SPLogbook` + launcher + desktop + ikona + marker; 12 plików / 18 wpisów | spec + verifier | `rpm-contents.txt`, manifest | `VERIFIED` |
| `FIND-M3I-03` | Prywatny `libhamlib.so.4`, RUNPATH `$ORIGIN/lib` / `$ORIGIN`, `GLIBC_2.43` | builder + verifier | log `Verify RPM` | `VERIFIED` |
| `FIND-M3I-04` | Dwie niezależne budowy mają identyczny SHA-256 `8915f7bf…4a57a` | builder | `rpm-build.log` | `VERIFIED` |
| `FIND-M3I-05` | rpmlint 2.9.0: 0 errors, 3 warnings; suppression tylko `dir-or-file-in-opt,binary-or-shlib-defines-rpath` | verifier | log `Verify RPM` | `VERIFIED` z 3 warningami |
| `FIND-M3I-06` | 34/34 mutacji odrzuconych, exit 1, bez komunikatu sukcesu | negative harness | `rpm-negative-fixtures.log` | `VERIFIED` |
| `FIND-M3I-07` | Cykl życia Fedora 45 + openSUSE Tumbleweed bez utraty danych | verifier | run `43453322341` | `VERIFIED` |
| `FIND-M3I-08` | Finalny `Build and Release` = `success`; `Publish Release Assets` = `skipped` | workflow | run `43453322341` | `VERIFIED` |
| `FIND-M3I-09` | **ROZWIĄZANE:** `CI` `43453322339` = `failure` (`RUSTSEC-2026-0523/0524/0538`) → remediacja `quinn-proto 0.11.19`, `quinn-udp 0.5.16`; `cargo audit` EXIT 0 bez nowych `--ignore` | `cargo audit` | log `CI` `43485657304` | **`RESOLVED`** |
| `FIND-M3I-10` | M3-J nadal `NOT_STARTED`; brak tagu/release | `docs/NEXT_WORK_GATE.md` | git/gh | `DEFINED` |