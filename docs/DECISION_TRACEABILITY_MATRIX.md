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
| OWNER-M3-F-LINUX-BASELINE-01 | Macierz 7 dystrybucji (Debian, Leap, …) | snapshot + skrypt + CI matrix | `linux_multi_distribution_compatibility_test` | `e5ac2d1`, `28d95cd` | `SUPERSEDED` (2026-11-09) |
| OWNER-M3-F-KERNEL-POLICY-01 | Dynamiczny floor kernela = 6.12 | `scripts/build_linux_portable_tar_gz.sh:179` + snapshot | test dynamicznego floor | `e5ac2d1` | `SUPERSEDED` → fixed **7.2** |
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
| OWNER-M3-F-ABI-BASELINE-01 | Budowa pod glibc 2.35, forward compatible | — | — | — | `CONTRADICTED` przez minimum 2.43 (F-01) |

## 4. Intencja OWNER z 2026-11-09 (NIE MA JEJ W REPOZYTORIUM)

| Element | Wartość OWNER | Stan repozytorium | Status |
|---|---|---|---|
| Minimalny kernel | **Linux 7.2**, `OWNER_APPROVED_FIXED_MINIMUM` | 6.12 dynamiczny | `OWNER_INTENT_NOT_YET_IN_REPO` |
| Minimalny glibc | **2.43** | 2.35 baseline w DESIGN | `OWNER_INTENT_NOT_YET_IN_REPO` |
| Maksymalny wymagany symbol glibc | `GLIBC_2.43` | `GLIBC_2.43` w snapshot | zgodne |
| Debian 13.7 (konfiguracja standardowa) | **NIE WSPIERANY** | wspierany w snapshot/CI | `CONTRADICTED` |
| openSUSE Leap | **NIE WSPIERANY** | wspierany w snapshot/CI | `CONTRADICTED` |
| Ubuntu 26.04 LTS | **WARUNKOWY** (kernel ≥ 7.2, glibc ≥ 2.43, oficjalne źródło jądra, test natywny, bez nieoficjalnych repo) | wspierany bezwarunkowo | `OWNER_INTENT_NOT_YET_IN_REPO` |
| Profile bezpośrednio wspierane | Ubuntu 26.10, Fedora 45 GA, Arch, Tumbleweed | 7 profili z Debian/Leap | `OWNER_INTENT_NOT_YET_IN_REPO` |
| Rebuild do glibc 2.41 / bundled glibc | **NIE** | brak bundled glibc | zgodne |

**Konsekwencja:** M3-F nie może być uznane za zamknięte wobec intencji OWNER; wymagana bramka
`STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` po zamknięciu QA. M3-G pozostaje `BLOCKED`.

## 5. Decyzje bramki QA 2026-11-09

| ID | Decyzja | Realizacja | Test | Status |
|---|---|---|---|---|
| QA-F24-01 | Natywny E2E instalatora wyłącznie jawny opt-in | `tests/support/native_artifact.rs`, `tests/inno_user_data_safety_test.rs` | `inno_native_e2e_contract_is_fail_closed`, strażnik `qa_native_installer_tests_are_hermetic_and_fail_closed` | `IMPLEMENTED` |
| QA-F24-02 | Jawna, absolutna ścieżka artefaktu + SHA-256; brak globów/`dist/` | jw. | jw. | `IMPLEMENTED` |
| QA-F24-03 | Przy opt-in wszystkie błędy fail-closed | jw. | jw. | `IMPLEMENTED` |
| QA-F23-01 | Usunięcie zależności od wall-clock w 3 testach | `src/sync/mod.rs` (testy), `src/cloud/updater/lock.rs` (testy) | te same testy + 20 iteracji | `IMPLEMENTED` |
| QA-F23-02 | Niezmiennik „max 1 jednoczesny posiadacz” blokady | `src/cloud/updater/lock.rs` (test) | `concurrent_lock_stress_has_single_winner` | `IMPLEMENTED` |
| QA-CI-01 | Standardowy job nie wymaga instalatora, uruchamia strażnik hermetyczności | `.github/workflows/ci.yml` | strażnik + pełny suite | `IMPLEMENTED` |
