# SPLogbook — BIEŻĄCY STAN PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-STATE-001
**Data pomiaru:** 2026-11-13
**Zasada:** ten plik jest jednostronicowym źródłem prawdy o stanie. Jest regenerowany przy
zamknięciu każdej bramki i **zastępuje wszystkie historyczne liczniki** w innych dokumentach.

---

## 1. Wersje i tożsamość

| Element | Wartość |
|---|---|
| Publiczna wersja stabilna | **1.2.0** |
| Tag `v1.2.0` | obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — `VERIFIED`, niemodyfikowany |
| Wersja rozwojowa | **1.3.0-alpha.1** (`Cargo.toml:3`) |
| Edition / MSRV | `2024` / `rust 1.99.0` |
| Toolchain w środowisku | `cargo 1.99.0`, `rustc 1.99.0` |
| Release publiczny | **NIE OPUBLIKOWANO** (brak tagu `v1.3.0*`, brak GitHub Release) |
| `rusqlite` / `libsqlite3-sys` | **0.41.1** / **0.39.1** (bundled SQLite **3.54.1**, feature `bundled` + `backup`) |
| `cargo audit` | **0 podatności** (2026-11-12; 1471 advisory, 674 zależności, `--deny warnings --no-yanked`, bez nowych `--ignore`) |
| **Minimum Linux** | **kernel 7.0, glibc 2.43** (`OWNER-M3-F-KERNEL-POLICY-OPTION-A`, `OWNER-M3-F-GLIBC-POLICY-01`) |
| **Wspierane dystrybucje** | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| **Kontrakt DEB** | Ubuntu 26.04 LTS (glibc 2.43); Debian 13.7 (glibc 2.41) i Leap 16.1 (2.40) niewspierane |
| **Układ DEB** | `/opt/SPLogbook` + launcher `/usr/bin/splogbook` (`OWNER-M3-H-03`) |
| Gałąź robocza | `main` |

## 2. Git po bramce M3-H

| Pole | Wartość |
|---|---|
| HEAD wejściowy (przed M3-H) | `cf127bbedae0b6b57efcce521a9f546f083b5406` |
| Commit implementacyjny M3-H | `236b7ea4be70b028fd0f701143144ac6f0952262` — `feat(linux): add verified DEB packaging pipeline` |
| Commity korygujące | `f9bab2e23951ba59bd7ca0ce7fed55f7e407ef37` (Hamlib payload + CRLF testy), `af04b1300a6e34f8b5c572b68a5c9d6b5a6c93f5` (libhamlib 0644), `3171aa8d03444901c55be7aa1f134e297b061604` (doc path-exclude w kontenerze) |
| Commit domknięcia tej bramki | najnowszy commit `main`, który zawiera ten dokument (`git log -1 --format=%H`) |
| main | = HEAD |
| origin/main | = HEAD (push ff-only wykonany w tej samej sesji; `git rev-parse origin/main`) |
| ahead / behind | `0 / 0` (zmierzone po pushu) |
| Zmienione pliki śledzone | **brak** (0 staged, 0 unstaged; `git diff --check` = EXIT 0) |
| Pliki nieśledzone i nieignorowane | **obecne** — `?? scratch/` (m.in. `scratch/tree_dupes.txt`, logi i dowody M3-H) |
| Pliki ignorowane | **obecne** (`target/`, `dist/`, `SPLogbook-backup/`, logi i skrypty w `scratch/` — wzorce `.gitignore:14` `*.log`, `.gitignore:52` `*.py`) |
| Czystość ścisła | **FALSE** — `STRICT_WORKTREE_CLEAN=false`, `TRACKED_WORKTREE_CLEAN=true`, `UNTRACKED_SCRATCH_PRESENT=true` |
| Aktywna operacja Git | brak |

`scratch/` **nie jest** ignorowany jako katalog; ignorowane są tylko wzorce plików. Plik
niepasujący do wzorca (`scratch/tree_dupes.txt`) jest realnie nieśledzony i daje `?? scratch/`
także z `-c core.fsmonitor=false -c core.untrackedCache=false`. To **nie** jest fantom F-25.

## 3. Baseline (pomiar 2026-11-12, po implementacji M3-H)

```
cargo +stable test --workspace --all-targets -j 1
```

| Licznik | Wartość |
|---|---|
| Zarejestrowane | **2 753** (`cargo test -- --list`) |
| Wykonane | **2 742** |
| Zaliczone | **2 742** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** |
| HIL wykonane | **0** |
| PTT / RF | **0 / 0** |
| Doc-testy | 0 (brak zarejestrowanych) |
| `security_guard` | **125 / 125** (w tym `m3_h_deb_integration_guard`, `m3_i_rpm_integration_guard`) |
| `linux_deb_packaging_test` | **17 / 17** |
| `linux_rpm_packaging_test` | **17 / 17** |

Pomiar wykonany w tej jednej sesji: `cargo test --workspace --all-targets -j 1 -- --skip
powershell_real_execution_full_cycle_success` → **2 742 / 2 742**, 0 failed, 10 ignored,
1 filtered. Test `windows_helper_test::powershell_real_execution_full_cycle_success` jest
**pre-existing** i w tym środowisku blokuje się na prompcie bezpieczeństwa Windows (Low IL);
ten sam test przeszedł w CI. Limit `-j 1` wynika z limitu pliku stronicowania hosta
(`os error 1455`).

## 4. Stan bramek

| Bramka | Status |
|---|---|
| M1 | `COMPLETED_SOFTWARE_VERIFIED` |
| M2 (design + decyzje OWNER) | `COMPLETED_DESIGN_VERIFIED` (część decyzji `OPEN`) |
| M3-A | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-B | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-C | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-DEV version identity | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-D Windows User Setup | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-E Windows MSI | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-F Linux portable tar.gz | **`COMPLETED_SOFTWARE_VERIFIED`** — dowód natywny fail-closed: run `42685105375`, commit `abdb767`, 4/4 profile `success` |
| QA baseline determinism + hermeticity | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** |
| Dependency security update | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** |
| M3-F Linux support policy alignment | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — 7.0 / 2.43, 4 dystrybucje |
| M3-F post-closure consistency verification | **`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED`** |
| M3-F F-03 fail-closed CI verification | **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`** — run `42685105375` (4/4 profile, zero `not found`, GLIBC ≤ 2.43) + job negatywny (exit 1) |
| M3-G AppImage | **`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`** — 4/4 profile `success`, 12/12 mutacji odrzuconych, FUSE `VERIFIED`; run `43166825054` + `CI` `43166825036` |
| **M3-H Linux DEB** | **`COMPLETED_DEB_INTEGRATION_VERIFIED`** — run `43245181565` (Build DEB, Verify DEB, 27/27 negative = `success`, Windows/Linux/AppImage `success`, `Publish Release Assets` `skipped`); `CI` `43245181469` = `success` |
| M3-I Linux RPM | **`BLOCKED`** — implementacja i weryfikacja RPM `success` (run `43453322341`, 34/34 mutacji, Fedora 45 + openSUSE Tumbleweed), ale `CI` `43453322339` = `failure` na `RUSTSEC-2026-0523/0524/0538` |
| M3-J (Flatpak) | `NOT_STARTED` |
| NATIVE-7B (fizyczny HIL) | `BLOCKED` (10 testów `#[ignore]`, 0 wykonanych) |

M3_G_STATUS: `CLOSED` — bramka AppImage pozostaje zamknięta i nietknięta.
M3_H_STATUS: `CLOSED` — bramka DEB pozostaje zamknięta.
M3_I_STATUS: `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES` — nie zamknięta.
M3_J_STATUS: `NOT_STARTED`.
F03_STATUS: `CLOSED` — F-03 pozostaje zamknięte.

## 5. Dowód DEB (skrót)

| Pole | Wartość |
|---|---|
| Artefakt | `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` (+ alias `SPLogbook-Linux-amd64.deb`) |
| Rozmiar | **11 883 596 B** |
| SHA-256 | **`85a4ca9877305d97f7c31929eb6034bd36e88dc8d7fd208f9fab9cf7d8890061`** |
| `Package` / `Version` / `Architecture` | `splogbook` / `1.3.0~alpha.1-1` / `amd64` |
| `Installed-Size` | 45 879 KiB |
| `build_commit` | `3171aa8d03444901c55be7aa1f134e297b061604` |
| Maksymalny GLIBC | `2.43` (SPLogbook, libhamlib); `2.39` (updater) |
| Pliki | 12 plików regularnych / 28 wpisów |
| Budowa | `dpkg 1.23.7`, `dpkg-deb 1.23.7`, obraz `ubuntu:26.04@sha256:6a06111c…`, dwie niezależne budowy o identycznym SHA-256 |
| Lintian | **0 errors**, 2 warnings (`initial-upload-closes-no-bugs`, `no-manual-page`); suppression wyłącznie `dir-or-file-in-opt,custom-library-search-path` (OWNER-M3-H-03) |
| Testy negatywne | **27/27** odrzuconych, exit 1, bez komunikatu sukcesu |
| Cykl życia | fresh install, upgrade z `1.3.0~alpha.0-1`, downgrade, re-upgrade, reinstall, remove, purge — wszystkie PASS; dane użytkownika SHA-256 identyczne |
| Publikacja | `Publish Release Assets` = `skipped`; brak nowego tagu i release |

## 5a. Dowód RPM (skrót)

| Pole | Wartość |
|---|---|
| Artefakt | `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` (+ alias `SPLogbook-Linux-x86_64.rpm`) |
| Rozmiar | **11 879 109 B** |
| SHA-256 | **`8915f7bfc0a829d5e3a10d5c961dc68f285b0ade901cb8475ab13705c6c4a57a`** |
| `Name` / `Version` / `Release` / `Arch` | `splogbook` / `1.3.0` / `0.1.alpha.1` / `x86_64` |
| `Installed Size` | **47 054 820 B** |
| `build_commit` | `3afd3ee0d097281e4cb14e6bcb2ec901ce9d702f` |
| Maksymalny GLIBC | `2.43` (SPLogbook, Hamlib); `2.39` (updater) |
| Pliki / wpisy | 12 regularnych / 18 |
| Budowa | `rpm 6.1.0`, `rpmbuild 6.1.0`, obraz `fedora:45@sha256:a7d72ba…`, dwie budowy o identycznym SHA-256 |
| rpmlint | **0 errors**, 3 warnings (`non-coherent-filename`, `no-manual-page`, `hidden-file-or-dir`) |
| Testy negatywne | **34/34** odrzuconych |
| Cykl życia | Fedora 45 + openSUSE Tumbleweed: fresh/remove/upgrade/downgrade/re-upgrade/reinstall/erase — PASS; dane użytkownika SHA-256 identyczne |
| Publikacja | `Publish Release Assets` = `skipped`; brak tagu i release |

## 6. Otwarte decyzje OWNER

- ~~`RUSTSEC-2026-0459`~~ — **ROZWIĄZANE** 2026-11-09 (`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`).
- ~~Polityka Linux (kernel, dystrybucje, glibc)~~ — **ZAPISANA** 2026-11-09.
- ~~F-03 (fail-closed weryfikacja Linux)~~ — **ZAMKNIĘTE** 2026-11-09 decyzją `OWNER-M3-F-F03-01` (GO).
- ~~`RUSTSEC-2026-0486`~~ — **ROZWIĄZANE** 2026-11-12 decyzją `OWNER-DEP-RUSTIX-01` (`rustix 1.1.6`).
- ~~M3-H (DEB)~~ — **ZAMKNIĘTE** 2026-11-12 decyzjami `OWNER-M3-H-01…10`.
- **`RUSTSEC-2026-0523`, `RUSTSEC-2026-0524`, `RUSTSEC-2026-0538`** — **NOWE, OTWARTE** (2026-11-13); blokują M3-I. Wymagają osobnej bramki bezpieczeństwa i aktualizacji `quinn-proto`/`quinn-udp` przez cargo; zakaz `--ignore`.
- `OWNER-M2-08`, `OWNER-M2-09`, `OWNER-M2-11`, `OWNER-M2-12`, `OWNER-M2-14` — otwarte, nieblokujące.
- `OWNER-M2-10` — spór dokumentacyjny (plan: APPROVED, DESIGN: OPEN), kod zaimplementowany.
- `OWNER-M3-E-MSI-VERSIONING-01` — `OPEN`; blokuje MSI dla `1.3.0-alpha.2` (Windows Installer 1638).

## 7. Co NIE jest zrobione

- Portable tar.gz — **ZREALIZOWANE** (M3-F), nadal PASS.
- AppImage — **ZREALIZOWANE** (M3-G `COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`), nadal PASS.
- DEB — **ZREALIZOWANE** (M3-H `COMPLETED_DEB_INTEGRATION_VERIFIED`, 2026-11-12).
- RPM (M3-I) — **ZABLOKOWANE**: pakiet zbudowany i zweryfikowany, ale `cargo audit` wykrył 3 nowe RustSec; brak autoryzacji na zmianę zależności.
- Flatpak (M3-J) — `NOT_STARTED`, nie rozpoczęty.
- MSI i Setup nie są budowane ani publikowane przez CI (F-05).
- Fizyczny HIL (NATIVE-7B) — niewykonany, sprzęt niedostępny.
- F-14 (P0, panika po PTT) — nadal otwarte, obowiązkowe przed HIL.
- F-19/F-20/F-21/F-16/F-15/F-17 — otwarte; wymagają bramki danych/RF.

## 8. Ograniczenia środowiska (znane, udokumentowane)

- Sandbox DSH: **Low Mandatory Integrity Level** → zapis do `HKCU` i Menu Start = `0x80070005`.
  Natywny E2E instalatora Windows jest w tym środowisku jawnie niewykonany (opt-in), a nie „zielony”.
- Masowy `tempfile::tempdir()` w `%TEMP%` hosta daje `PermissionDenied`; baseline uruchamiany
  z `TMP`/`TEMP` w katalogu workspace (obejście środowiska, nie kodu).
- `windows_helper_test::powershell_real_execution_full_cycle_success` może zawisnąć na prompcie
  bezpieczeństwa Windows; w takim przebiegu test jest jawnie filtrowany (`1 filtered`), a ten sam
  test przechodzi w CI oraz przeszedł w pełnym przebiegu tej bramki.
- Brak lokalnego środowiska Linux/Docker — natywna weryfikacja dystrybucji wymaga CI.
- Limit pliku stronicowania hosta (`os error 1455`) uniemożliwia stabilny `-j 2`; baseline
  wykonuje się `-j 1`.
- Kontenery OCI współdzielą jądro hosta — dowód jest dowodem userspace/CI, nie dowodem jądra
  dystrybucji.

## 9. Następna bramka

NEXT_GATE: **`M3-I`** — Integracja Linux RPM (`MASTER_IMPLEMENTATION_PLAN.md` §10)
NEXT_GATE_STATUS: `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES`
NEXT_GATE_OWNER_GO: `REQUIRED_FOR_SECURITY_REMEDIATION`
F03_STATUS: `CLOSED`
M3_G_STATUS: `CLOSED`
M3_H_STATUS: `CLOSED`
M3_I_STATUS: `BLOCKED`
M3_J_STATUS: `NOT_STARTED`

**`STAGE-M3-I-LINUX-RPM-INTEGRATION-GATE`** = **`BLOCKED`** (2026-11-13):
RPM jest zbudowany deterministycznie i zweryfikowany (run `43453322341`,
34/34 mutacji, Fedora 45 + openSUSE Tumbleweed), ale finalny `CI`
`43453322339` = `failure` na `RUSTSEC-2026-0523/0524/0538`. Bramka nie jest
zamknięta; M3-J pozostaje `NOT_STARTED`.
