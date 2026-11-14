# SPLogbook — BIEŻĄCY STAN PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-STATE-001
**Data pomiaru:** **2026-11-14** (praca bramki rozpoczęta 2026-11-13; data hosta przyjęta zgodnie z `AGENTS.md` §1)
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
| Toolchain w środowisku | `cargo 1.99.0`, `rustc 1.99.0` — bez zmian |
| Release publiczny | **NIE OPUBLIKOWANO** (brak tagu `v1.3.0*`, brak GitHub Release) |
| `rusqlite` / `libsqlite3-sys` | **0.41.1** / **0.39.1** (bundled SQLite **3.54.1**, feature `bundled` + `backup`) |
| `cargo audit` | **0 podatności** (2026-11-14; **1509** advisory, **663** zależności, `--deny warnings --no-yanked`, **bez nowych `--ignore`**) |
| `quinn-proto` / `quinn-udp` | **0.11.19** / **0.5.16** (naprawa `RUSTSEC-2026-0523/0524/0538`) |
| **Minimum Linux** | **kernel 7.0, glibc 2.43** (`OWNER-M3-F-KERNEL-POLICY-OPTION-A`, `OWNER-M3-F-GLIBC-POLICY-01`) |
| **Wspierane dystrybucje** | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| **Kontrakt DEB** | Ubuntu 26.04 LTS (glibc 2.43); Debian 13.7 (glibc 2.41) i Leap 16.1 (2.40) niewspierane |
| **Układ DEB / RPM** | `/opt/SPLogbook` + launcher `/usr/bin/splogbook` (`OWNER-M3-H-03`) |
| Gałąź robocza | `main` |

## 2. Git po bramkach M3-I-SEC i M3-I

| Pole | Wartość |
|---|---|
| HEAD wejściowy bramki bezpieczeństwa | `8a7c745d5707c5aecd118b92d398dc3dbf52e831` |
| Commit raportu przedimplementacyjnego | `3c6f4597f9b57e9df60b3168a9eb523593393550` |
| Commit remediacji zależności | `326e469e44deece341288817dbe692204456fb4b` |
| Commit raportu bezpieczeństwa | commit `docs(security): record M3-I RustSec remediation verification` |
| Commit domknięcia bramki | najnowszy commit `main`, który zawiera ten dokument (`git log -1 --format=%H`) |
| main | = finalny HEAD |
| origin/main | = finalny HEAD (push ff-only wykonany w tej samej sesji; `git rev-parse origin/main`) |
| ahead / behind | `0 / 0` (zmierzone po pushu) |
| Zmienione pliki śledzone | **brak** (0 staged, 0 unstaged; `git diff --check` = EXIT 0) |
| Pliki nieśledzone i nieignorowane | **obecne** — `?? scratch/` |
| Czystość ścisła | **FALSE** — `STRICT_WORKTREE_CLEAN=false`, `TRACKED_WORKTREE_CLEAN=true`, `UNTRACKED_SCRATCH_PRESENT=true` |
| Aktywna operacja Git | brak |

`scratch/` **nie jest** ignorowany jako katalog; ignorowane są tylko wzorce plików
(`.gitignore:14` `*.log`, `.gitignore:52` `*.py`). Plik niepasujący do wzorca jest realnie
nieśledzony i daje `?? scratch/`. `scratch/` nie został commitowany.

## 3. Baseline (pomiar 2026-11-14, po naprawie RustSec)

```
cargo test --workspace --all-targets -j 1 -- --skip powershell_real_execution_full_cycle_success
```

| Licznik | Wartość |
|---|---|
| Zarejestrowane | **2 753**, binarek testowych **22** |
| Wykonane | **2 742** |
| Zaliczone | **2 742** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** |
| HIL wykonane | **0** |
| PTT / RF | **0 / 0** |
| Przefiltrowane | **1** (`windows_helper_test::powershell_real_execution_full_cycle_success` — ograniczenie hosta; w CI przechodzi) |
| Doc-testy | 0 |
| `security_guard` | **125 / 125** |
| `linux_rpm_packaging_test` | **17 / 17** |
| `linux_deb_packaging_test` | **17 / 17** |
| `cargo fmt --check` / `clippy -D warnings` / `cargo doc` | EXIT 0 / EXIT 0 / EXIT 0 (21 pre-existing warningów rustdoc) |

`cargo audit` lokalnie i w CI: **EXIT 0**.
`cargo tree -d`: **47** duplikatów (było 53; bez nowych).

## 4. Stan bramek

| Bramka | Status |
|---|---|
| M1 | `COMPLETED_SOFTWARE_VERIFIED` |
| M2 (design + decyzje OWNER) | `COMPLETED_DESIGN_VERIFIED` (część decyzji `OPEN`) |
| M3-A…M3-E | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-F Linux portable tar.gz | **`COMPLETED_SOFTWARE_VERIFIED`** — dowód natywny fail-closed: run `42685105375`, commit `abdb767`, 4/4 profile `success` |
| QA baseline determinism + hermeticity | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** |
| Dependency security update | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** |
| M3-F Linux support policy alignment | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** |
| M3-F post-closure consistency | **`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED`** |
| M3-F F-03 fail-closed CI | **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`** — `CLOSED` |
| M3-G AppImage | **`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`** — `CLOSED` |
| M3-H Linux DEB | **`COMPLETED_DEB_INTEGRATION_VERIFIED`** — `CLOSED` |
| **M3-I-SEC RustSec remediation** | **`COMPLETED_RUSTSEC_REMEDIATION_VERIFIED`** — `cargo audit` EXIT 0, 3 advisory usunięte |
| **M3-I Linux RPM** | **`COMPLETED_RPM_INTEGRATION_VERIFIED`** — zamknięte 2026-11-14 po odblokowaniu; `CI` `43485657304` = `success`, `Build and Release` `43485657283` = `success` |
| M3-J (Flatpak) | `NOT_STARTED` |
| NATIVE-7B (fizyczny HIL) | `BLOCKED` (10 testów `#[ignore]`, 0 wykonanych) |

M3_G_STATUS: `CLOSED`.
M3_H_STATUS: `CLOSED`.
M3_I_STATUS: `COMPLETED_RPM_INTEGRATION_VERIFIED`.
M3_I_SEC_STATUS: `COMPLETED_RUSTSEC_REMEDIATION_VERIFIED`.
M3_J_STATUS: `NOT_STARTED`.
F03_STATUS: `CLOSED`.

## 5. Dowód DEB (skrót, stan po przebudowie)

| Pole | Wartość |
|---|---|
| Artefakt | `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` (+ alias `SPLogbook-Linux-amd64.deb`) |
| Rozmiar | **11 898 688 B** |
| SHA-256 | **`69f4153f20316f3bd953cd6f6cd181a99e8bd1e64cff02ec5f8c49d0a15e02ac`** |
| `Package` / `Version` / `Architecture` | `splogbook` / `1.3.0~alpha.1-1` / `amd64` |
| `Installed-Size` | 45 895 KiB |
| `build_commit` | `326e469e44deece341288817dbe692204456fb4b` |
| Maksymalny GLIBC | `2.43` (SPLogbook, libhamlib); `2.39` (updater) |
| Budowa | `dpkg 1.23.7`, obraz `ubuntu:26.04@sha256:6a06111c…`, dwie niezależne budowy o identycznym SHA-256 |
| Lintian | **0 errors**, 2 warnings; suppression wyłącznie `dir-or-file-in-opt,custom-library-search-path` (`OWNER-M3-H-03`) |
| Testy negatywne | **27/27** odrzuconych |
| Cykl życia | fresh/upgrade/downgrade/re-upgrade/reinstall/remove/purge — PASS; dane użytkownika SHA-256 identyczne |
| Publikacja | `Publish Release Assets` = `skipped` |

## 5a. Dowód RPM (skrót, stan po przebudowie i odblokowaniu)

| Pole | Wartość |
|---|---|
| Artefakt | `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` (+ alias `SPLogbook-Linux-x86_64.rpm`) |
| Rozmiar | **11 881 961 B** |
| SHA-256 | **`a25dcb7a7595e15cc4d47fc290f9833f2aaa3cdbfca6f4b0e5e73e56872873bd`** |
| `Name` / `Version` / `Release` / `Arch` | `splogbook` / `1.3.0` / `0.1.alpha.1` / `x86_64` |
| `Installed Size` | **47 054 820 B** |
| `build_commit` | `326e469e44deece341288817dbe692204456fb4b` |
| Maksymalny GLIBC | `2.43` (SPLogbook, Hamlib); `2.39` (updater) |
| Pliki / wpisy | 12 regularnych / 18 |
| Budowa | `rpm`/`rpmbuild`/`rpmspec` **6.1.0**, `rpmlint 2.9.0`, obraz `fedora:45@sha256:a7d72ba…`, dwie budowy o identycznym SHA-256 |
| rpmlint | **0 errors**, 3 warnings (`non-coherent-filename`, `no-manual-page`, `hidden-file-or-dir`) |
| Testy negatywne | **34/34** odrzuconych |
| Cykl życia | Fedora 45 + openSUSE Tumbleweed: pełna sekwencja PASS; dane użytkownika SHA-256 identyczne |
| Publikacja | `Publish Release Assets` = `skipped`; brak tagu i release |

## 6. Otwarte decyzje OWNER

- ~~`RUSTSEC-2026-0523`, `RUSTSEC-2026-0524`, `RUSTSEC-2026-0538`~~ — **ROZWIĄZANE** 2026-11-13/14
  bramką `STAGE-M3-I-SEC` (`quinn-proto 0.11.19`, `quinn-udp 0.5.16`, `cargo audit` EXIT 0,
  bez nowych `--ignore`).
- ~~`RUSTSEC-2026-0459`~~ — **ROZWIĄZANE** 2026-11-09 (`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`).
- ~~Polityka Linux~~ — **ZAPISANA** 2026-11-09.
- ~~F-03~~ — **ZAMKNIĘTE** 2026-11-09.
- ~~`RUSTSEC-2026-0486`~~ — **ROZWIĄZANE** 2026-11-12 (`rustix 1.1.6`).
- ~~M3-H (DEB)~~ — **ZAMKNIĘTE** 2026-11-12.
- `OWNER-M2-08`, `OWNER-M2-09`, `OWNER-M2-11`, `OWNER-M2-12`, `OWNER-M2-14` — otwarte, nieblokujące.
- `OWNER-M2-10` — spór dokumentacyjny (plan: APPROVED, DESIGN: OPEN), kod zaimplementowany.
- `OWNER-M3-E-MSI-VERSIONING-01` — `OPEN`; blokuje MSI dla `1.3.0-alpha.2`.

## 7. Co NIE jest zrobione

- Portable tar.gz, AppImage, DEB, RPM — **ZREALIZOWANE**, nadal PASS.
- Flatpak (M3-J) — `NOT_STARTED`, nie rozpoczęty; wymaga `OWNER_GO_REQUIRED`.
- MSI i Setup nie są budowane ani publikowane przez CI (F-05).
- Fizyczny HIL (NATIVE-7B) — niewykonany, sprzęt niedostępny.
- F-14 (P0, panika po PTT) — nadal otwarte, obowiązkowe przed HIL.
- F-19/F-20/F-21/F-16/F-15/F-17 — otwarte; wymagają bramki danych/RF.

## 8. Ograniczenia środowiska (znane, udokumentowane)

- Sandbox DSH: **Low Mandatory Integrity Level** → zapis do `HKCU` i Menu Start = `0x80070005`.
  Natywny E2E instalatora Windows jest w tym środowisku jawnie niewykonany (opt-in), a nie „zielony”.
- Masowy `tempfile::tempdir()` poza workspace daje `PermissionDenied`; baseline uruchamiany
  z `TMP`/`TEMP` = `target/m3isec-tmp` (obejście środowiska, nie kodu).
- `windows_helper_test::powershell_real_execution_full_cycle_success` może zawisnąć na prompcie
  bezpieczeństwa Windows; w takim przebiegu test jest jawnie filtrowany (`1 filtered`), a w CI przechodzi.
- Brak lokalnego środowiska Linux/Docker — natywna weryfikacja dystrybucji wymaga CI.
- Limit pliku stronicowania hosta (`os error 1455`) uniemożliwia stabilny `-j 2`; baseline `-j 1`.
- Kontenery OCI współdzielą jądro hosta — dowód jest dowodem userspace/CI, nie dowodem jądra dystrybucji.

## 9. Następna bramka

NEXT_GATE: **`M3-J`** — Integracja Linux Flatpak (`MASTER_IMPLEMENTATION_PLAN.md` §10)
NEXT_GATE_STATUS: `NOT_STARTED`
NEXT_GATE_OWNER_GO: **`OWNER_GO_REQUIRED`** — brak autoryzacji; nie rozpoczęto analizy implementacyjnej.
F03_STATUS: `CLOSED`
M3_G_STATUS: `CLOSED`
M3_H_STATUS: `CLOSED`
M3_I_STATUS: `COMPLETED_RPM_INTEGRATION_VERIFIED`
M3_I_SEC_STATUS: `COMPLETED_RUSTSEC_REMEDIATION_VERIFIED`
M3_J_STATUS: `NOT_STARTED`

**`STAGE-M3-I-LINUX-RPM-INTEGRATION-GATE`** = **`COMPLETED_RPM_INTEGRATION_VERIFIED`** (2026-11-14):
RPM przebudowany po naprawie RustSec, zweryfikowany deterministycznie (34/34 negatywnych,
Fedora 45 + openSUSE Tumbleweed, dane użytkownika bez zmian), `cargo audit` EXIT 0;
finalne `CI` `43485657304` i `Build and Release` `43485657283` = `success`;
`Publish Release Assets` = `skipped`. **M3-J pozostaje `NOT_STARTED`.**
