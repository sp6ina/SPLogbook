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
| `cargo audit` | **0 podatności** (ostatni pomiar bramki zależności, bez `--ignore`) |
| **Minimum Linux** | **kernel 7.0, glibc 2.43** (`OWNER-M3-F-KERNEL-POLICY-OPTION-A`, `OWNER-M3-F-GLIBC-POLICY-01`) |
| **Wspierane dystrybucje** | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| Gałąź robocza | `repair/full-system-remediation` → fast-forward do `main` |

## 2. Git po zamknięciu bramki polityki Linux

| Pole | Wartość |
|---|---|
| HEAD | commit dokumentacyjny domknięcia tej bramki (ostatni commit `main`; `git log -1`) |
| main | = HEAD (fast-forward z `repair/full-system-remediation`) |
| origin/main | = HEAD (push ff-only po zielonym baseline) |
| ahead / behind | `0 / 0` |
| Working tree | `CLEAN` |
| Aktywna operacja Git | brak |

Commity bramki polityki Linux:

1. `docs(linux): record M3-F support policy baseline` — raport wstępny + snapshot v2
2. `ci(linux): align supported distribution matrix` — macierz CI + skrypt budowania + fixture
3. `test(linux): enforce documented support policy` — strażnicy i testy
4. `docs(project): close M3-F Linux support policy gate` — dokumenty stanu + raport końcowy

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
| `security_guard` | **120 / 120** |
| `inno_user_data_safety_test` | **8 / 8** (E2E jawnie niewykonany bez opt-in) |
| `msi_user_data_safety_test` | **6 / 6** |

Baseline nie zależy od `dist/`; natywny E2E instalatora pozostaje jawnym opt-in.

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
| M3-F Linux portable tar.gz | `COMPLETED_SOFTWARE_VERIFIED` (implementacja i dowody z runu `42518873662`; jakość dowodu: legacy, nie fail-closed) |
| QA baseline determinism + hermeticity | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** |
| Dependency security update | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** |
| **M3-F Linux support policy alignment** | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — 7.0 / 2.43, 4 dystrybucje, Debian/Leap niewspierane |
| M3-G AppImage | **`NOT_STARTED`** (nie rozpoczęto; wymaga osobnego GO) |
| M3-H/J (DEB/RPM/Flatpak) | `NOT_STARTED` |
| NATIVE-7B (fizyczny HIL) | `BLOCKED` (10 testów `#[ignore]`, 0 wykonanych) |

## 5. Otwarte decyzje OWNER

- ~~`RUSTSEC-2026-0459`~~ — **ROZWIĄZANE** 2026-11-09 (`rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`).
- ~~Polityka Linux (kernel, dystrybucje, glibc)~~ — **ZAPISANA** 2026-11-09:
  `OWNER-M3-F-KERNEL-POLICY-OPTION-A` (dynamiczne minimum 7.0) oraz
  `OWNER-M3-F-GLIBC-POLICY-01` (2.43). Decyzja zastępuje sztywne 7.2.
- `OWNER-M2-08`, `OWNER-M2-09`, `OWNER-M2-11`, `OWNER-M2-12`, `OWNER-M2-14` — otwarte, nieblokujące.
- `OWNER-M2-10` — spór dokumentacyjny (plan: APPROVED, DESIGN: OPEN), kod zaimplementowany.
- `OWNER-M3-E-MSI-VERSIONING-01` — `OPEN`; blokuje MSI dla `1.3.0-alpha.2` (Windows Installer 1638).
- Otwarte ryzyko techniczne (nie decyzja): fixture natywnej weryfikacji Linux nie jest fail-closed
  (F-03); wymaga osobnego zadania CI-hardening przed publikacją M3-G.

## 6. Co NIE jest zrobione

- AppImage / DEB / RPM / Flatpak — brak implementacji.
- MSI i Setup nie są budowane ani publikowane przez CI (F-05).
- Natywna weryfikacja Linux nie jest fail-closed (F-03); brak nowego przebiegu na 4-profilowej macierzy.
- Fizyczny HIL (NATIVE-7B) — niewykonany, sprzęt niedostępny.
- F-14 (P0, panika po PTT) — nadal otwarte, obowiązkowe przed HIL.
- F-19/F-20/F-21/F-16/F-15/F-17 — otwarte; wymagają bramki danych/RF.

## 7. Ograniczenia środowiska (znane, udokumentowane)

- Sandbox DSH: **Low Mandatory Integrity Level** → zapis do `HKCU` i Menu Start = `0x80070005`.
  Natywny E2E instalatora jest w tym środowisku jawnie niewykonany (opt-in), a nie „zielony”.
- Windows pokazuje prompt „Otwieranie pliku — ostrzeżenie o zabezpieczeniach” dla niepodpisanych
  binariów uruchamianych przez `ShellExecute` (fallback PowerShell). W sesji interaktywnej
  wymaga kliknięcia „Uruchom”; na CI prompt nie występuje.
- `Start-Process` w sandboxie DSH może zawisnąć na tym prompcie; testy używają `CreateProcess`.
- `git status` bez `-c core.untrackedCache=false` potrafi zgłosić fantomowy `scratch/`.
- Brak lokalnego środowiska Linux/Docker — weryfikacja natywna dystrybucji wymaga CI.

## 8. Następna bramka

**`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`** — `NOT_STARTED`; wymaga osobnego promptu OWNER.
Wejście: minimum **Linux 7.0 / glibc 2.43**, cztery wspierane dystrybucje, jeden kanoniczny artefakt.
