# SPLogbook — BIEŻĄCY STAN PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-STATE-001
**Data pomiaru:** 2026-11-11
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
| `cargo audit` | **0 podatności** (ostatni pomiar bramki zależności, bez `--ignore`) |
| **Minimum Linux** | **kernel 7.0, glibc 2.43** (`OWNER-M3-F-KERNEL-POLICY-OPTION-A`, `OWNER-M3-F-GLIBC-POLICY-01`) |
| **Wspierane dystrybucje** | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| Gałąź robocza | `main` |

## 2. Git po bramce F-03

| Pole | Wartość |
|---|---|
| HEAD wejściowy (przed tą bramką) | `e85ee4e0c80d2f3ca96b9e7068f401586662b18b` |
| Commit implementacyjny F-03 | `abdb767d41bd07da1304c1c8f5d8993726a6575a` — `ci(linux): make native evidence free of diagnostic ambiguity` (poprzedzony `4699eacdf0076554e51107593641fb64f4dc6dfb`) |
| Commit domknięcia tej bramki | najnowszy commit `main`, który zawiera ten dokument (`git log -1 --format=%H`) |
| main | = HEAD |
| origin/main | = HEAD (push ff-only wykonany w tej samej sesji; `git rev-parse origin/main`) |
| ahead / behind | `0 / 0` (zmierzone po pushu) |
| Zmienione pliki śledzone | **brak** (0 staged, 0 unstaged; `git diff --check` = EXIT 0) |
| Pliki nieśledzone i nieignorowane | **obecne** — `?? scratch/` (m.in. `scratch/tree_dupes.txt`, `scratch/audit/…`, `scratch/ci/…`, logi dowodowe F-03) |
| Pliki ignorowane | **obecne** (`target/`, `dist/`, `SPLogbook-backup/`, logi i skrypty w `scratch/` — wzorce `.gitignore:14` `*.log`, `.gitignore:52` `*.py`) |
| Czystość ścisła | **FALSE** — `STRICT_WORKTREE_CLEAN=false`, `TRACKED_WORKTREE_CLEAN=true`, `UNTRACKED_SCRATCH_PRESENT=true` |
| Aktywna operacja Git | brak |

`scratch/` **nie jest** ignorowany jako katalog; ignorowane są tylko wzorce plików. Plik
niepasujący do wzorca (`scratch/tree_dupes.txt`) jest realnie nieśledzony i daje `?? scratch/`
także z `-c core.fsmonitor=false -c core.untrackedCache=false`. To **nie** jest fantom F-25.

## 3. Baseline (ostatni pełny pomiar: 2026-11-09, po implementacji F-03)

```
cargo +stable test --workspace --all-targets -j 1
```

| Licznik | Wartość |
|---|---|
| Zarejestrowane | **2 702** (`cargo test -- --list`) |
| Wykonane | **2 692** |
| Zaliczone | **2 692** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** |
| HIL wykonane | **0** |
| PTT / RF | **0 / 0** |
| Doc-testy | 0 (brak zarejestrowanych) |
| `security_guard` | **122 / 122** (w tym nowy `m3_f_f03_fail_closed_verification_guard`) |
| `inno_user_data_safety_test` | **8 / 8** (E2E jawnie niewykonany bez opt-in) |
| `msi_user_data_safety_test` | **6 / 6** |

Pomiar wykonany w **tej jednej sesji** (2026-11-09, `cargo +stable test --workspace --all-targets
-j 1` → EXIT 0; 19 targetów). Wzrost 2701→2702 / 2691→2692 wynika wyłącznie z dodania strażnika
F-03. Baseline nie zależy od `dist/`; natywny E2E instalatora pozostaje jawnym opt-in.
`-j 1` zamiast `-j 2` z powodu limitu pliku stronicowania tego hosta (`os error 1455`) —
różnica wydajnościowa, nie wynikowa. Po zmianach dokumentacyjnych powtórka z pominięciem
jednego pre-existing testu zablokowanego przez prompt sandboxa
(`windows_helper_test::powershell_real_execution_full_cycle_success`) → 2 691 / 2 691, 0 failed,
10 ignored, 1 filtered; ten test przeszedł w pełnym przebiegu na tym samym kodzie (2 692).

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
| **M3-F F-03 fail-closed CI verification** | **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`** — run `42685105375` (4/4 profile, zero `not found`, GLIBC ≤ 2.43) + job negatywny (exit 1) |
| M3-G AppImage | **`IN_PROGRESS`** — GO OWNER udzielone 2026-11-11; implementacja w toku, brak dowodów CI |
| M3-H/J (DEB/RPM/Flatpak) | `NOT_STARTED` |
| NATIVE-7B (fizyczny HIL) | `BLOCKED` (10 testów `#[ignore]`, 0 wykonanych) |

## 5. Otwarte decyzje OWNER

- ~~`RUSTSEC-2026-0459`~~ — **ROZWIĄZANE** 2026-11-09 (`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`).
- ~~Polityka Linux (kernel, dystrybucje, glibc)~~ — **ZAPISANA** 2026-11-09.
- ~~F-03 (fail-closed weryfikacja Linux)~~ — **ZAMKNIĘTE** 2026-11-09 decyzją
  `OWNER-M3-F-F03-01` (GO) i bramką `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`.
- `OWNER-M2-08`, `OWNER-M2-09`, `OWNER-M2-11`, `OWNER-M2-12`, `OWNER-M2-14` — otwarte, nieblokujące.
- `OWNER-M2-10` — spór dokumentacyjny (plan: APPROVED, DESIGN: OPEN), kod zaimplementowany.
- `OWNER-M3-E-MSI-VERSIONING-01` — `OPEN`; blokuje MSI dla `1.3.0-alpha.2` (Windows Installer 1638).

## 6. Co NIE jest zrobione

- AppImage / DEB / RPM / Flatpak — AppImage w **`IN_PROGRESS`** (M3-G, GO OWNER 2026-11-11,
  brak dowodów CI); DEB / RPM / Flatpak bez implementacji (M3-H/J `NOT_STARTED`).
- MSI i Setup nie są budowane ani publikowane przez CI (F-05).
- Fizyczny HIL (NATIVE-7B) — niewykonany, sprzęt niedostępny.
- F-14 (P0, panika po PTT) — nadal otwarte, obowiązkowe przed HIL.
- F-19/F-20/F-21/F-16/F-15/F-17 — otwarte; wymagają bramki danych/RF.

## 7. Dowód natywny F-03 (skrót)

Run `42685105375` (`head_sha = abdb767d41bd07da1304c1c8f5d8993726a6575a`, `conclusion = success`):

| Profil | Biblioteki runtime | `not found` | `--version` | `--build-info` | GLIBC max | Wynik |
|---|---|---|---|---|---|---|
| Ubuntu 26.04.1 LTS | zainstalowane | **0** | exit 0, `1.3.0-alpha.1` | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.43) | PASS |
| Fedora 45 | zainstalowane | **0** | exit 0, `1.3.0-alpha.1` | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.44) | PASS |
| Arch Linux | zainstalowane | **0** | exit 0, `1.3.0-alpha.1` | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.44) | PASS |
| openSUSE Tumbleweed | zainstalowane | **0** | exit 0, `1.3.0-alpha.1` | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.44) | PASS |

Test negatywny: goły obraz `ubuntu:26.04` bez bibliotek runtime → `libasound.so.2 => not found`,
`FAIL: ldd reports an unresolved shared library`, **exit code 1**, brak komunikatu sukcesu.
`Publish Release Assets` = `skipped`; brak nowego tagu i release.

## 8. Ograniczenia środowiska (znane, udokumentowane)

- Sandbox DSH: **Low Mandatory Integrity Level** → zapis do `HKCU` i Menu Start = `0x80070005`.
  Natywny E2E instalatora jest w tym środowisku jawnie niewykonany (opt-in), a nie „zielony”.
- Windows pokazuje prompt „Otwieranie pliku — ostrzeżenie o zabezpieczeniach” dla niepodpisanych
  binariów uruchamianych przez `ShellExecute` (fallback PowerShell).
- `Start-Process` w sandboxie DSH może zawisnąć na tym prompcie; testy używają `CreateProcess`.
- `git status` klasyfikuje stan poprawnie; `scratch/` daje `?? scratch/` dopóki zawiera plik
  niepasujący do wzorca `*.log`/`*.py` — to nie fantom cache.
- Brak lokalnego środowiska Linux/Docker — weryfikacja natywna dystrybucji wymaga CI.
- Limit pliku stronicowania hosta (`os error 1455`) uniemożliwia stabilny `-j 2`; baseline
  wykonuje się `-j 1`.
- 2026-11-09 18:12–18:43Z: awaria GitHub Actions (`major_outage`, potwierdzona status page).
  Runy na commicie wejściowym `e85ee4e` (`42673055954` CI — 0 jobów; `42673056245` — brak joba
  Windows) są **skutkiem awarii infrastruktury**, nie defektem repozytorium. Runy bramki F-03
  (`42685105375`, `42685105448`) wykonały się w pełni po ustąpieniu awarii.

## 9. Następna bramka

NEXT_GATE: **`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`**
NEXT_GATE_STATUS: `IN_PROGRESS`
NEXT_GATE_OWNER_GO: `GRANTED` (`OWNER-M3-G-01`, `OWNER-M3-G-05`)
F03_STATUS: `CLOSED`
M3_G_STATUS: `IN_PROGRESS`

**`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`** jest **`IN_PROGRESS`** od 2026-11-11
(GO OWNER udzielone). Wejście: minimum **Linux 7.0 / glibc 2.43**, cztery wspierane dystrybucje,
jeden kanoniczny artefakt, fail-closed weryfikator natywny z runu `42685105375`. Bramka nie jest
zamknięta do czasu pełnych, zielonych dowodów CI.
