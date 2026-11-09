# SPLogbook — BIEŻĄCY STAN PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-STATE-001
**Data pomiaru:** 2026-11-09
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
| `cargo audit` | **0 podatności** (świeża baza, bez `--ignore`), 7 dozwolonych warningów |
| Gałąź robocza | `repair/full-system-remediation` → fast-forward do `main` |

## 2. Git po zamknięciu bramki QA

| Pole | Wartość |
|---|---|
| HEAD | commit dokumentacyjny domknięcia bramki zależności (ostatni commit `main`; `git log -1`) |
| main | = HEAD (fast-forward z `repair/full-system-remediation`) |
| origin/main | = HEAD (push ff-only po zielonym `cargo audit`) |
| ahead / behind | `0 / 0` |
| Working tree | `CLEAN` |
| Aktywna operacja Git | brak |

Commity bramki QA:

1. `418720b` — `test(qa): make native installer E2E explicit and hermetic`
2. `72263fc` — `test(qa): remove wall-clock assumptions from concurrency tests`
3. `f35330e` — `ci(qa): enforce deterministic hermetic baseline`
4. `8e00b5d` — `docs(agent): add DeepSeek Harness project handoff`
5. `551e74f` — `docs(qa): register dependency advisory decision proposal`

Commity bramki zależności:

6. raport wstępny `STAGE_DEPENDENCY_SECURITY_UPDATE_PRE_IMPLEMENTATION_REPORT.md`
7. `fix(deps): update rusqlite to 0.41.1 with patched bundled SQLite`
8. raport końcowy + aktualizacja stanu i następnej bramki

## 3. Baseline (ostatni pełny pomiar: 2026-11-09)

```
cargo +stable test --workspace --all-targets -j 2
```

| Licznik | Wartość |
|---|---|
| Zarejestrowane | **2 700** (`cargo test -- --list`) |
| Wykonane | **2 690** |
| Zaliczone | **2 690** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** |
| HIL wykonane | **0** |
| PTT / RF | **0 / 0** |
| Doc-testy | 0 (brak zarejestrowanych) |
| `cargo audit` (świeża baza) | **PASSED — 0 podatności**, 7 dozwolonych warningów; bez `--ignore` |
| `security_guard` | **120 / 120** |
| `inno_user_data_safety_test` | **8 / 8** (E2E jawnie niewykonany bez opt-in) |
| `msi_user_data_safety_test` | **6 / 6** |

Względem bazy `f73a703` (2 698 / 2 688 / 10) licznik wzrósł o **2** nowe testy:
`inno_native_e2e_contract_is_fail_closed` oraz
`qa_native_installer_tests_are_hermetic_and_fail_closed`.

## 4. Stan bramek

| Bramka | Status |
|---|---|
| M1 | `COMPLETED_SOFTWARE_VERIFIED` |
| M2 (design + decyzje OWNER) | `COMPLETED_DESIGN_VERIFIED` (część decyzji `OPEN`) |
| M3-A | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-B | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-C | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-DEV version identity | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-D Windows User Setup | `COMPLETED_SOFTWARE_VERIFIED` (bez otwarcia; brak dowodu regresji) |
| M3-E Windows MSI | `COMPLETED_SOFTWARE_VERIFIED` |
| M3-F Linux portable tar.gz | `COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED` **wobec polityki obowiązującej przy zamknięciu**; wobec intencji OWNER z 2026-11-09 — `REOPENED_BY_POLICY` (do bramki polityki) |
| **QA baseline determinism + hermeticity** | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** (2026-11-09; chwilowy bloker zależności usunięty) |
| **Dependency security update** | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite 3.54.1), 0 podatności |
| M3-F Linux support policy alignment | `NOT_STARTED` — **następna bramka** |
| M3-G AppImage | **`BLOCKED`** |
| M3-H/J (DEB/RPM/Flatpak) | `NOT_STARTED` |
| NATIVE-7B (fizyczny HIL) | `BLOCKED` (10 testów `#[ignore]`, 0 wykonanych) |

## 5. Otwarte decyzje OWNER

- ~~`RUSTSEC-2026-0459`~~ — **ROZWIĄZANE** 2026-11-09 przez `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE`
  (`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`, SQLite 3.54.1; `bundled` zachowany; bez `--ignore`).
- `OWNER-M2-08`, `OWNER-M2-09`, `OWNER-M2-11`, `OWNER-M2-12`, `OWNER-M2-14` — otwarte, nieblokujące.
- `OWNER-M2-10` — spór dokumentacyjny (plan: APPROVED, DESIGN: OPEN), kod zaimplementowany.
- `OWNER-M3-E-MSI-VERSIONING-01` — `OPEN`; blokuje MSI dla `1.3.0-alpha.2` (Windows Installer 1638).
- **Polityka Linux 2026-11-09** (kernel 7.2 fixed, glibc 2.43, bez Debian 13.7 i Leap,
  Ubuntu 26.04 LTS warunkowe) — `OWNER_INTENT_NOT_YET_IN_REPO`; wymaga
  `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`.
- Pytanie blokujące tę bramkę: snapshot rejestruje jądra Fedora 45 / Arch / Tumbleweed jako
  **6.18**, czyli poniżej wymaganego 7.2 — OWNER musi rozstrzygnąć, czy dane są nieaktualne,
  czy minimum 7.2 wymaga rewizji, czy lista profili zawęża się do Ubuntu 26.10.

## 6. Co NIE jest zrobione

- AppImage / DEB / RPM / Flatpak — brak implementacji.
- MSI i Setup nie są budowane ani publikowane przez CI (F-05).
- Fizyczny HIL (NATIVE-7B) — niewykonany, sprzęt niedostępny.
- F-14 (P0, panika po PTT) — nadal otwarte, obowiązkowe przed HIL.
- F-19/F-20/F-21/F-16/F-15/F-17 — otwarte; wymagają bramki danych/RF.
- Polityka Linux — niezapisana w repozytorium.

## 7. Ograniczenia środowiska (znane, udokumentowane)

- Sandbox DSH: **Low Mandatory Integrity Level** → zapis do `HKCU` i Menu Start = `0x80070005`.
  Natywny E2E instalatora jest w tym środowisku jawnie niewykonany (opt-in), a nie „zielony”.
- Windows pokazuje prompt „Otwieranie pliku — ostrzeżenie o zabezpieczeniach” dla niepodpisanych
  binariów uruchamianych przez `ShellExecute` (fallback PowerShell). W sesji interaktywnej
  wymaga kliknięcia „Uruchom”; na CI prompt nie występuje.
- `Start-Process` w sandboxie DSH może zawisnąć na tym prompcie; testy używają `CreateProcess`.
- `git status` bez `-c core.untrackedCache=false` potrafi zgłosić fantomowy `scratch/`.

## 8. Następna bramka

**`STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`** — `NOT STARTED`.
Wymaga wcześniejszej decyzji OWNER o jądrach 6.18 vs minimum 7.2 (szczegóły w `docs/NEXT_WORK_GATE.md`).

**M3-G pozostaje zablokowane.**
