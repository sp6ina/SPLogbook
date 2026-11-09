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
| Gałąź robocza | `repair/full-system-remediation` → fast-forward do `main` |

## 2. Git po zamknięciu bramki QA

| Pole | Wartość |
|---|---|
| HEAD (`repair/full-system-remediation`) | commit dokumentacyjny bramki QA (4. commit gałęzi; `git log -1`) |
| main | `f73a703c40f4462b48bed241aafc47fe15486bd0` (**bez zmian — brak ff**) |
| origin/main | `f73a703c40f4462b48bed241aafc47fe15486bd0` (**bez push**) |
| ahead / behind (`origin/main...main`) | `0 / 0` (main nietknięty) |
| ahead / behind (`origin/main...repair/...`) | `4 / 0` (commity lokalne, niepushowane) |
| Working tree | `CLEAN` |
| Aktywna operacja Git | brak |
| Push / ff na main | **WSTRZYMANE** — bloker RUSTSEC-2026-0459 |

Commity bramki QA:

1. `418720b` — `test(qa): make native installer E2E explicit and hermetic`
2. `72263fc` — `test(qa): remove wall-clock assumptions from concurrency tests`
3. `f35330e` — `ci(qa): enforce deterministic hermetic baseline`
4. commit dokumentacyjny — `docs(agent): add DeepSeek Harness project handoff` (HEAD bramki)

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
| `cargo audit` (świeża baza) | **FAILED — 1 podatność: RUSTSEC-2026-0459 (`libsqlite3-sys 0.38.2`), ujawniona 2026-11-09T09:49Z** |
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
| **QA baseline determinism + hermeticity** | **`BLOCKED_BY_NEW_DEPENDENCY_ADVISORY`** — testy zielone (2 690/2 690), blokuje `RUSTSEC-2026-0459` (2026-11-09) |
| M3-F Linux support policy alignment | `NOT_STARTED` — **następna bramka** |
| M3-G AppImage | **`BLOCKED`** |
| M3-H/J (DEB/RPM/Flatpak) | `NOT_STARTED` |
| NATIVE-7B (fizyczny HIL) | `BLOCKED` (10 testów `#[ignore]`, 0 wykonanych) |

## 5. Otwarte decyzje OWNER

- **BLOKER: `RUSTSEC-2026-0459`** — nowa advisory (dodana do RustSec 2026-11-09T09:49Z) w niezmienionej
  zależności `libsqlite3-sys 0.38.2` (`rusqlite 0.40.2`, `bundled`). Wymaga decyzji:
  (A) bramka aktualizacji `rusqlite 0.41` / `libsqlite3-sys 0.39` — rekomendowane,
  (B) formalna akceptacja ryzyka + jawny `--ignore` w dedykowanej bramce bezpieczeństwa,
  (C) rezygnacja z `bundled` na system SQLite. **Nie wolno ignorować w tej bramce.**
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

1. **`STAGE-DEPENDENCY-SECURITY-UPDATE-GATE`** — zamknięcie `RUSTSEC-2026-0459` (decyzja OWNER;
   rekomendowane `rusqlite 0.41` / `libsqlite3-sys 0.39`, `Cargo.lock`, pełny baseline).
   **Blokuje wszystko dalej, w tym push.**
2. `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` — po odblokowaniu QA.

**M3-G pozostaje zablokowane.**
