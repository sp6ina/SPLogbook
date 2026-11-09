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

## 2. Git po bramce spójności po zamknięciu M3-F

| Pole | Wartość |
|---|---|
| HEAD wejściowy (przed tą bramką) | `72c3fe6451e9cfbc9ab25d3bbf23ba4e955ba605` |
| Commit domknięcia tej bramki | najnowszy commit `main`, który zawiera ten dokument (`git log -1 --format=%H`) |
| main | = HEAD (fast-forward z `repair/full-system-remediation`) |
| origin/main | = HEAD (push ff-only wykonany w tej samej sesji; `git rev-parse origin/main`) |
| ahead / behind | `0 / 0` (zmierzone po pushu) |
| Zmienione pliki śledzone | **brak** (0 staged, 0 unstaged; `git diff --check` = EXIT 0) |
| Pliki nieśledzone i nieignorowane | **1**: `scratch/tree_dupes.txt` (lokalny artefakt analizy, nie commitowany) |
| Pliki ignorowane | **obecne** (`target/`, `dist/`, `SPLogbook-backup/`, logi i skrypty w `scratch/` — wzorce `.gitignore:14` `*.log`, `.gitignore:52` `*.py`) |
| Czystość ścisła | **FALSE** — `STRICT_WORKTREE_CLEAN=false`, `TRACKED_WORKTREE_CLEAN=true`, `UNTRACKED_SCRATCH_PRESENT=true` |
| Aktywna operacja Git | brak |

`scratch/` **nie jest** ignorowany jako katalog; ignorowane są tylko wzorce plików. Plik
niepasujący do wzorca (`scratch/tree_dupes.txt`) jest realnie nieśledzony i daje `?? scratch/`
także z `-c core.fsmonitor=false -c core.untrackedCache=false`. To **nie** jest fantom F-25.

Commity bramki polityki Linux (poprzedzającej tę bramkę):

1. `c96c1ae` — `docs(linux): record M3-F support policy baseline`
2. `ae11b16` — `ci(linux): align supported distribution matrix`
3. `1253b9e` — `test(linux): enforce documented support policy`
4. `72c3fe6` — `docs(project): close M3-F Linux support policy gate`

## 3. Baseline (ostatni pełny pomiar: 2026-11-09)

```
cargo +stable test --workspace --all-targets -j 1
```

| Licznik | Wartość |
|---|---|
| Zarejestrowane | **2 701** (`cargo test -- --list`) |
| Wykonane | **2 691** |
| Zaliczone | **2 691** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** |
| HIL wykonane | **0** |
| PTT / RF | **0 / 0** |
| Doc-testy | 0 (brak zarejestrowanych) |
| `security_guard` | **121 / 121** |
| `inno_user_data_safety_test` | **8 / 8** (E2E jawnie niewykonany bez opt-in) |
| `msi_user_data_safety_test` | **6 / 6** |

Pomiar wykonany w **tej jednej sesji domykającej** (2026-11-09, `cargo +stable test --workspace
--all-targets -j 1` → EXIT 0; 1 422 s łącznie z kompilacją; 19 targetów, w tym 14 plików
integracyjnych). Wszystkie liczniki powyżej pochodzą wyłącznie z tego pomiaru — w tym
`security_guard` **121 / 121** z nowym strażnikiem `m3_f_post_closure_consistency_guard`.
Baseline nie zależy od `dist/`; natywny E2E instalatora pozostaje jawnym opt-in.
`-j 1` zamiast `-j 2` z powodu limitu pliku stronicowania tego hosta (`os error 1455`) —
różnica wydajnościowa, nie wynikowa.

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
| M3-F Linux portable tar.gz | `COMPLETED_SOFTWARE_VERIFIED` (implementacja; dowód natywny: legacy, nie fail-closed) |
| QA baseline determinism + hermeticity | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** |
| Dependency security update | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** |
| M3-F Linux support policy alignment | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — 7.0 / 2.43, 4 dystrybucje, Debian/Leap niewspierane |
| **M3-F post-closure consistency verification** | **`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED`** — domknięta **jedną sesją** (jeden commit dokumentacyjny + push ff-only po własnym zielonym baseline); klasyfikacja drzewa, sprostowanie 7 profili, formalna bramka F-03 |
| M3-G AppImage | **`BLOCKED` / `NOT_STARTED`** (wymaga zamknięcia F-03 i osobnego GO OWNER) |
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
  (**F-03**); formalna bramka `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` = `OPEN`.
  Dowód bieżący: przebieg 4-profilowy `42638736785` (commit `72c3fe6`) — wszystkie 4 joby `success`,
  ale logi **wszystkich czterech** profili pokazują `libasound.so.2 => not found` i nieudane uruchomienie
  binarki przy jednoczesnym `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` (wykazany fałszywy PASS).
  Zweryfikowane pierwszoosobowo w sesji domykającej: GitHub API `…/runs/42638736785/jobs`
  (`head_sha = 72c3fe6`, 4 joby `Verify Linux` = `success`, `Publish Release Assets` = `skipped`)
  oraz pobrane artefakty `native-evidence-{ubuntu-lts,fedora-final,arch-rolling,opensuse-tumbleweed}`
  (każdy z nich: `=== 5. ELF & LOADER VALIDATION ===` → `libasound.so.2 => not found`, potem
  `error while loading shared libraries`, a na końcu `NATIVE VERIFICATION COMPLETED SUCCESSFULLY`).

## 6. Co NIE jest zrobione

- AppImage / DEB / RPM / Flatpak — brak implementacji.
- MSI i Setup nie są budowane ani publikowane przez CI (F-05).
- Natywna weryfikacja Linux nie jest fail-closed (F-03); przebieg 4-profilowy **istnieje**
  (`42638736785`), ale jego logi nie są dowodem natywnym (brak bibliotek runtime w obrazach).
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
- `git status` klasyfikuje stan poprawnie; `scratch/` daje `?? scratch/` dopóki zawiera plik
  niepasujący do wzorca `*.log`/`*.py` — to nie fantom cache (F-25 skorygowane 2026-11-09).
- Brak lokalnego środowiska Linux/Docker — weryfikacja natywna dystrybucji wymaga CI.
- Limit pliku stronicowania hosta (`os error 1455`) uniemożliwia stabilny `-j 2`; baseline
  wykonuje się `-j 1`.

## 8. Następna bramka

NEXT_GATE: **`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`**
NEXT_GATE_STATUS: `NOT_STARTED`
NEXT_GATE_OWNER_GO: `REQUIRED`
M3_G_STATUS: `BLOCKED_BY_F03`

Jedyna następna bramka **wykonywalna** to **`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`**
(`OPEN`, `NOT_STARTED`, wymaga jawnego GO OWNER) — domyka F-03 (fail-closed weryfikacja Linux)
i dopiero po jej zamknięciu odblokowuje M3-G. Zakres i kryteria akceptacji: `docs/NEXT_WORK_GATE.md`.

**`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`** pozostaje **`BLOCKED` / `NOT_STARTED`**: wymaga
zamkniętej bramki F-03 **oraz** osobnego promptu OWNER. Wejście po odblokowaniu: minimum
**Linux 7.0 / glibc 2.43**, cztery wspierane dystrybucje, jeden kanoniczny artefakt.
