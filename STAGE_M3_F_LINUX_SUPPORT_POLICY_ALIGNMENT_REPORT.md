# RAPORT KOŃCOWY BRAMKI
# STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE

**Projekt:** SPLogbook
**Repozytorium:** `sp6ina/SPLogbook`
**Gałąź docelowa:** `main`
**Tryb:** AUTONOMOUS REPOSITORY AGENT — analiza, weryfikacja, wyrównanie polityki, testy, dokumentacja

---

## 1. Data, czas i strefa czasowa

- **Data projektu:** 2026-11-09
- **Czas rozpoczęcia:** 2026-11-09T14:22:13+01:00 (13:22:13Z)
- **Czas raportu:** 2026-11-09T15:35+01:00 (14:35Z)
- **Strefa:** Europe/Warsaw (CET, +01:00)
- **Środowisko:** Windows 10.0.26300, x86_64-pc-windows-msvc, `rustc/cargo 1.99.0`, Git 2.56.0

## 2. Identyfikator i status bramki

- **ID:** `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`
- **Status:** **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`**

## 3. Stan początkowy repozytorium

- HEAD = main = origin/main = `351124c6c748459864e75f1d51292dbdfa004aaf`
- Gałąź checkoutowana: `repair/full-system-remediation` (ten sam commit co `main`)
- `origin/repair/full-system-remediation` = `f73a703` — **przodek** `main` (brak rozbieżności)
- Working tree: **CLEAN**, brak nieśledzonych plików istotnych dla Git
- Tag `v1.2.0`: obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
- Brak submodułów

## 4. Wynik synchronizacji z origin/main

`git fetch --all --tags --prune` → EXIT 0. `origin/main` wskazywał ten sam commit co oczekiwany SHA
z promptu — **synchronizacja nie była potrzebna**, ahead/behind `0/0`. Nie wykonano merge, rebase
ani force-push.

## 5. SHA HEAD, main i origin/main

| Moment | HEAD / main / origin/main |
|---|---|
| Wejście | `351124c6c748459864e75f1d51292dbdfa004aaf` |
| Bramka — commit 1 | `c96c1ae` — `docs(linux): record M3-F support policy baseline` |
| Bramka — commit 2 | `ae11b16` — `ci(linux): align supported distribution matrix` |
| Bramka — commit 3 | `1253b9e` — `test(linux): enforce documented support policy` |
| Bramka — commit 4 | commit domknięcia (`docs(project): close M3-F Linux support policy gate`) |

Po pushu: `HEAD == main == origin/main` (weryfikacja w `git log` i `git rev-parse`; dokładny SHA
w odpowiedzi końcowej sesji).

## 6. Ahead/behind i czystość drzewa

- Przed push: ahead = liczba commitów bramki (4), behind = 0
- Po push: **0 / 0**, working tree **CLEAN**

## 7. Przeczytane dokumenty kanoniczne

`AGENTS.md`; `docs/PROJECT_CURRENT_STATE.md`; `docs/NEXT_WORK_GATE.md`; `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{md,json}`;
`STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md`; `STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md`;
`OWNER-M3-F-LINUX-BASELINE-01-PROPOSAL.md`; `docs/DECISION_TRACEABILITY_MATRIX.md`;
`docs/DOCUMENTATION_INDEX.md`; `docs/DEEPSEEK_HARNESS_HANDOFF.md`; `MASTER_IMPLEMENTATION_PLAN.md`;
`DESIGN_3_0_B_LINUX_UPDATE.md`; `.github/workflows/{ci,build-and-release}.yml`;
`scripts/build_linux_portable_tar_gz.sh`; `scripts/verify_linux_container.sh`;
`tests/{security_guard,linux_multi_distribution_compatibility_test,linux_portable_tar_gz_test,linux_archive_security_test}.rs`;
`README.md`; `installer/linux/portable/README.md`; `docs/IMPROVEMENT_ROADMAP.md`; `docs/REMEDIATION_BASELINE.md`.

## 8. Hierarchia źródeł zastosowana w zadaniu

1. aktualny kod i konfiguracja repozytorium (skrypty, CI, testy),
2. oficjalne źródła dystrybucji (packages.ubuntu.com, packages.debian.org, packages.fedoraproject.org,
   archlinux.org, news.opensuse.org, releases.ubuntu.com),
3. `AGENTS.md`,
4. `docs/PROJECT_CURRENT_STATE.md`,
5. `docs/NEXT_WORK_GATE.md`,
6. `MASTER_IMPLEMENTATION_PLAN.md`,
7. raporty wykonawcze M3-F (z naniesioną erratą),
8. raporty historyczne i analizy (oznaczone jako historyczne).

Kod i oficjalne źródła miały pierwszeństwo; dokumenty historyczne nie nadpisywały pomiarów.

## 9. Potwierdzenie tożsamości bramki

`docs/NEXT_WORK_GATE.md` wskazywał dokładnie tę bramkę jako jedyną następną, `NOT_STARTED`,
z zamkniętą prekondycją QA + zależności. Nazwa zgodna, bramka niewykonana wcześniej.

**Rozstrzygnięty konflikt intencji:** `AGENTS.md` §5, `NEXT_WORK_GATE.md` i macierz decyzji
zapisywały sztywne `7.2` (`OWNER_APPROVED_FIXED_MINIMUM`), a prompt wprowadził nowszą decyzję
**`OWNER-M3-F-KERNEL-POLICY-OPTION-A`**. Zastosowano decyzję nowszą (OPTION A) i wyrównano
dokumenty — nie odwrotnie.

## 10. Stan poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` | `COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED` |
| `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` | `COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED` |
| `STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE` | `COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED` (implementacja; dowód legacy, nie fail-closed) |
| **Ta bramka** | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** |

Nie otwierano ponownie bramki zależności ani bramki QA.

## 11. Tabela wspieranych dystrybucji

| Dystrybucja | Model | Wersja / snapshot | Kernel (linia) | glibc | Status |
|---|---|---|---|---|---|
| Ubuntu 26.04.1 LTS (Resolute Raccoon) | LTS | 26.04.1 | **7.0** (`7.0.0-42-generic`) | **2.43** | `SUPPORTED` |
| Fedora Linux 45 | Classic GA | 45 | **7.2** (`7.2.8-300.fc45`) | **2.44** | `SUPPORTED` |
| Arch Linux | Rolling (official core) | `7.2.14.arch1-1` | **7.2** | **2.44** | `SUPPORTED` |
| openSUSE Tumbleweed | Rolling snapshot | `20261021` | **7.2** (`7.2.11-1-default`) | **2.44** | `SUPPORTED` |
| Debian 13.7 (Trixie) | Classic stable | 13.7 | 6.12 (`6.12.101-1`) | 2.41 | `NOT_SUPPORTED` |
| openSUSE Leap 16.1 | Classic | 16.1 | 6.12 | 2.40 | `NOT_SUPPORTED` |
| Ubuntu 26.10 (Stonking Stingray) | Interim | 26.10 | 7.3 | 2.44 | `OUT_OF_SCOPE` |

## 12. Oficjalne źródło dla każdej wersji dystrybucji

- Ubuntu: `https://releases.ubuntu.com/26.04/`, `https://packages.ubuntu.com/resolute/linux-image-generic`,
  `https://packages.ubuntu.com/resolute/libc6`
- Debian: `https://www.debian.org/releases/trixie/`, `https://packages.debian.org/trixie/linux-image-amd64`,
  `https://packages.debian.org/trixie/libc6`
- Fedora: `https://packages.fedoraproject.org/pkgs/kernel/kernel/fedora-45.html`,
  `https://packages.fedoraproject.org/pkgs/glibc/glibc/fedora-45.html`
- Arch: `https://archlinux.org/packages/core/x86_64/linux/`, `https://archlinux.org/packages/core/x86_64/glibc/`
- openSUSE: `https://news.opensuse.org/2026/10/30/planet-roundup/`,
  `https://news.opensuse.org/2026/11/01/tw-monthly-update-october/`

## 13. Domyślny kernel każdej dystrybucji

Ubuntu 26.04 LTS: `7.0.0-42-generic` (`linux-image-generic 7.0.0-42.42`); Debian 13.7: `6.12.101-1`;
Fedora 45: `7.2.8-300.fc45`; Arch: `7.2.14-arch1-1`; Tumbleweed: `7.2.11-1-default`;
Ubuntu 26.10: linia `7.3`.

## 14. Kernel stabilnego repozytorium każdej dystrybucji

- Ubuntu 26.04 LTS: `7.0.0-42.42` (repo `resolute`, także `security`)
- Debian 13.7: `6.12.101-1`
- Fedora 45: `7.2.8-300.fc45` (stabilne aktualizacje)
- Arch: `7.2.14.arch1-1` (core, build 2026-10-30)
- Tumbleweed: `7.2.11-1` (snapshot 20261021)

## 15. Rozróżnienie stable / testing / backports / rolling

Wyznaczają minimum wyłącznie **stabilne repozytoria** wspieranych dystrybucji. Nie wyznaczają:
`fedora-45-updates-testing` (`7.2.9`), Rawhide, backports, repozytoria eksperymentalne, RC ani
niestabilne snapshoty. Arch i Tumbleweed są rolling, ale liczy się ich **oficjalne stabilne**
repozytorium/snapshot, nie wersja upstream.

## 16. Wyznaczone minimum kernela

**Linux 7.0** = `MIN(7.0, 7.2, 7.2, 7.2)` — wyznaczone przez **Ubuntu 26.04 LTS**.

## 17. Wyznaczone minimum glibc

**2.43** (`GLIBC_2.43`).

## 18. Uzasadnienie techniczne minimum

- Artefakt deklaruje maksymalny wymagany symbol `GLIBC_2.43` (`libm.so.6`; `libc.so.6` — 2.39),
  więc runtime wymaga glibc ≥ 2.43. Debian 13.7 (2.41) i Leap 16.1 (2.40) **nie spełniają** ABI
  i dlatego są niewspierane — to nie jest arbitralne zawężenie, lecz konsekwencja zmierzonego ABI.
- Brak w `src/` użycia `libc::`/syscalli i API jądra nowszego niż 7.0 (jedyne `/proc/<pid>` jest
  w helperze Windows). Kernel nie tworzy ostrzejszego ograniczenia niż polityka dystrybucyjna.
- Najostrzejszym ograniczeniem jest **glibc 2.43**, a nie kernel; minimum kernela wynika z najstarszej
  wspieranej dystrybucji, która spełnia ABI.
- Nie użyto najwyższego kernela rolling-release (7.2) jako minimum — zgodnie z OPTION A.

## 19. Rozdzielenie runtime, build i CI

| Warstwa | Wartość |
|---|---|
| Minimum runtime | Linux 7.0, glibc 2.43, x86_64 |
| Środowisko budowania | Ubuntu 26.04 LTS, Rust 1.99.x, `ubuntu-latest` (**nieprzypięty** — ryzyko) |
| Macierz CI | 4 przypięte obrazy OCI: Ubuntu 26.04, Fedora 45, Arch, Tumbleweed |
| Najnowsze środowisko testowe | Fedora 45 / Arch / Tumbleweed (7.2) |
| Wersja w CI | **nie** jest automatycznie minimum runtime |

## 20. Znalezione sprzeczności dokumentacji

1. Repo kodowało 6.12 i 7 profili; OWNER wymaga minimum dynamicznego z 4 wspieranych dystrybucji.
2. Snapshot deklarował `GLIBC_2.43` **i** `VERIFIED_PASS` na Debian (2.41) / Leap (2.40) — niemożliwe.
3. `verify_linux_container.sh` maskował błędy `|| true` (F-03) — PASS nie był fail-closed.
4. Nieaktualne dane: Arch 6.18.2 (jest 7.2.14), Fedora 6.18.1 (7.2.8), Tumbleweed 6.18.2 (7.2.11),
   Ubuntu 7.0.0-18 (7.0.0-42), Debian 6.12.43-1 (6.12.101-1), glibc Debiana w teście 2.40 (2.41).
5. `README.md` deklarował Ubuntu 20.04+/Debian 11+/Fedora 36+ — sprzeczne z ABI i polityką.
6. `AGENTS.md`/`NEXT_WORK_GATE` deklarowały sztywne 7.2 — zastąpione przez OPTION A.

Wszystkie powyższe zostały usunięte lub jawnie oznaczone jako historyczne/niewspierane.

## 21. Lista zmienionych plików

**Nowe (2):** `STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_PRE_IMPLEMENTATION_REPORT.md`,
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`.

**Zmienione (27):** `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json`, `.md`,
`scripts/build_linux_portable_tar_gz.sh`, `scripts/verify_linux_container.sh`,
`.github/workflows/build-and-release.yml`, `.github/workflows/ci.yml`,
`tests/security_guard.rs`, `tests/linux_multi_distribution_compatibility_test.rs`,
`tests/linux_portable_tar_gz_test.rs`, `tests/linux_archive_security_test.rs`, `AGENTS.md`,
`README.md`, `MASTER_IMPLEMENTATION_PLAN.md`, `OWNER-M3-F-LINUX-BASELINE-01-PROPOSAL.md`,
`DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md`, `DESIGN_3_0_B_LINUX_UPDATE.md`,
`STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_INTEGRATION_REPORT.md`,
`STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md`,
`STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md`, `docs/NEXT_WORK_GATE.md`,
`docs/PROJECT_CURRENT_STATE.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`,
`docs/DOCUMENTATION_INDEX.md`, `docs/DEEPSEEK_HARNESS_HANDOFF.md`,
`docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`,
`installer/linux/portable/README.md`.

Bez zmian: `src/`, `Cargo.toml`, `Cargo.lock`, schemat SQLite, protokoły CAT, emisja RF.

## 22. Opis każdej zmiany

- **Snapshot v2:** minimum 7.0/2.43, 4 profile, `evaluated_not_supported` (Debian/Leap/26.10), data
  snapshotu, źródła, zasada rewizji; dowody natywne oznaczone jako legacy/non-fail-closed.
- **Builder:** `RESOLVED_MIN_KERNEL="7.0"` + komentarz o OPTION A.
- **Verifier:** jawny nagłówek „LEGACY NON-FAIL-CLOSED” (F-03), bez zmiany zachowania.
- **CI:** macierz 4 profili; usunięte Ubuntu 26.10, Debian, Leap; odświeżone liczniki baseline.
- **Testy:** strażnik polityki + dynamiczny floor; wartości 7.0; 4 profile.
- **Dokumenty:** polityka zapisana, macierze i statusy wyrównane, errata w raportach historycznych.

## 23. Zachowanie przed i po

| Obszar | Przed | Po |
|---|---|---|
| Minimum kernela | 6.12 (dynamiczny) | **7.0** (OPTION A) |
| Minimum glibc | 2.43 (niezapisane w repo) | **2.43 zapisane i egzekwowane** |
| Profile wspierane | 7 (z Debian/Leap/26.10) | **4** |
| Debian 13.7 / Leap | deklarowane PASS | **jawnie NOT_SUPPORTED** |
| Marker artefaktu | `6.12` | `7.0` |
| Macierz CI | 7 obrazów | 4 obrazy |
| Dowód natywny | „VERIFIED_PASS” bez fail-closed | jawnie legacy, nie-PASS |

## 24. Wyniki testów celowanych

| Test | Wynik |
|---|---|
| `security_guard::m3_f_owner_decisions_and_support_policy_guard` | **1 passed, 0 failed** |
| `security_guard::qa_native_installer_tests_are_hermetic_and_fail_closed` | **1 passed, 0 failed** |
| `linux_multi_distribution_compatibility_test` | **5 passed, 0 failed** |
| `linux_portable_tar_gz_test` | **7 passed, 0 failed** |
| `linux_archive_security_test` | **13 passed, 0 failed** |

## 25. Wyniki pełnego baseline

```
cargo +stable test --workspace --all-targets -j 1   # -j 2 niemożliwe: limit pliku stronicowania (os error 1455)
```

| Licznik | Wartość |
|---|---|
| Zarejestrowane | **2 700** |
| Wykonane | **2 700** |
| Zaliczone | **2 690** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** |
| HIL wykonane | **0** |
| `security_guard` | **120 / 120** |

`cargo +stable check --workspace --all-targets` → EXIT 0 (14m 14s).

## 26. Wynik cargo audit

**Nie uruchamiano w tej bramce.** Bramka zależności jest zamknięta
(`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`, 0 podatności, bez `--ignore`), a w tej bramce nie
zmieniano `Cargo.toml`/`Cargo.lock` ani nie pojawił się nowy advisory. Zgodnie z `AGENTS.md` nie
otwierano jej ponownie.

## 27. Wynik clippy -D warnings

`cargo +stable clippy --workspace --all-targets -j 1 -- -D warnings` → **EXIT 0, 0 ostrzeżeń**.

## 28. Wynik git diff --check

**EXIT 0** (przed commitami i po wszystkich zmianach).

## 29. Lista commitów bramki

1. `c96c1ae` — `docs(linux): record M3-F support policy baseline`
2. `ae11b16` — `ci(linux): align supported distribution matrix`
3. `1253b9e` — `test(linux): enforce documented support policy`
4. commit domknięcia — `docs(project): close M3-F Linux support policy gate`

## 30. Potwierdzenie push

Push `ff-only` z `repair/full-system-remediation` do `origin/main` po zielonym baseline; po push
ponowny `fetch` i weryfikacja `HEAD == main == origin/main`, ahead/behind `0/0`, working tree `CLEAN`.

## 31. Stan tagu v1.2.0

Nietknięty: obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled
`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`. Nie utworzono, nie przeniesiono ani nie usunięto tagu.

## 32. Potwierdzenie braku nowego release

Brak nowego tagu `v1.3.0*`, brak GitHub Release, brak publikacji assetów. Wersja publiczna nadal `1.2.0`.

## 33. Potwierdzenie braku rozpoczęcia M3-G

M3-G (`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`) pozostaje **`NOT_STARTED`**; nie wykonano
żadnych prac AppImage.

## 34. Ryzyka rezydualne

1. **F-03 OPEN:** `verify_linux_container.sh` nadal nie jest fail-closed (brak bibliotek runtime
   w bazowych obrazach). Wymaga osobnego CI-hardeningu przed M3-G.
2. **Brak natywnego re-runu** nowej 4-profilowej macierzy; dowody z runu `42518873662` są legacy.
3. **`ubuntu-latest` nieprzypięty** — baseline glibc budowy może dryfować i podnieść wymagania ABI.
4. `cargo tree -d`: 24 crates z duplikatami wersji (52 wpisy) — stan zastany, nie pogorszony.
5. Odstępstwo wykonawcze: `-j 1` zamiast `-j 2` z powodu limitu pliku stronicowania
   (os error 1455) — różnica wydajnościowa, nie wynikowa.
6. 21 pre-existing ostrzeżeń rustdoc (`cargo doc` bez `-D warnings`).

## 35. Następny dozwolony krok

`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE` — **dopiero po osobnym GO OWNER** i po domknięciu
CI-hardeningu F-03. Nie rozpoczynać automatycznie.

## 36. Ostateczny status bramki

**`COMPLETED_POLICY_ALIGNMENT_VERIFIED`**

---

## 37. ERRATA (2026-11-09, `STAGE-M3-F-POST-CLOSURE-CONSISTENCY-VERIFICATION-GATE`)

Errata **nie zmienia** statusu bramki (`COMPLETED_POLICY_ALIGNMENT_VERIFIED`) ani żadnej wartości
polityki (7.0 / 2.43 / 4 dystrybucje). Prostuje trzy opisy, które po pomiarze okazały się
nieprecyzyjne lub nieaktualne. Źródło: `STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_REPORT.md`.

### 37.1. Klasyfikacja drzewa roboczego (§3, §6)

Zapis „Working tree: **CLEAN**” był **nieprecyzyjny**. Poprawna klasyfikacja wymaga czterech
wymiarów (definicja w `AGENTS.md` §2 pkt 4):

| Wymiar | Wartość przy pomiarze bramki |
|---|---|
| Zmiany śledzone | brak (`TRACKED_WORKTREE_CLEAN=true`) |
| Pliki nieśledzone i nieignorowane | obecne (`UNTRACKED_SCRATCH_PRESENT=true`) |
| Pliki ignorowane | obecne (`target/`, `dist/`, logi w `scratch/`) |
| Czystość ścisła | **FALSE** (`STRICT_WORKTREE_CLEAN=false`) |

`scratch/` nie jest ignorowany jako katalog — ignorowane są wyłącznie wzorce plików
(`.gitignore:14` `*.log`, `.gitignore:52` `*.py`). To **nie** jest fantom cache (F-25 skorygowane).

### 37.2. Chronologia pushu i znacznik czasu (§1)

- **Czas raportu:** `2026-11-09T15:35+01:00` — bez zmian.
- Znacznik `15:51:32+01:00` z promptu to **koniec sesji po pushu**, a nie czas raportu.
- Push commita `72c3fe6` uruchomił runy `42638736785` (Build and Release) i `42638736856` (CI);
  joby runu `42638736785` utworzono `2026-11-09T14:51:16Z` = `15:51:16+01:00` (zweryfikowane
  pierwszoosobowo GitHub API `created_at`, sesja domykająca 2026-11-09).

### 37.3. Klasyfikacja dowodu natywnego (§34 pkt 2)

- Zdanie „brak natywnego re-runu nowej 4-profilowej macierzy” było prawdziwe tylko w chwili
  pisania raportu. **Przebieg 4-profilowy istnieje:** run `42638736785`, commit `72c3fe6`,
  joby `Verify Linux (Ubuntu 26.04.1 LTS)`, `(Fedora 45)`, `(Arch)` i `(openSUSE Tumbleweed)` —
  wszystkie `success`.
- **Jakość dowodu pozostaje legacy / nie fail-closed (F-03):** artefakty
  `native-evidence-ubuntu-lts`, `native-evidence-fedora-final`, `native-evidence-arch-rolling`
  i `native-evidence-opensuse-tumbleweed` zawierają na **wszystkich czterech** profilach
  `libasound.so.2 => not found` oraz `error while loading shared libraries`, po czym skrypt
  wypisuje `NATIVE VERIFICATION COMPLETED SUCCESSFULLY`, a job kończy się `success`.
  Binarka nie została uruchomiona ani razu — to **wykazany fałszywy PASS**, nie hipoteza.
- Run `42518873662` (commit `2e40a64`) dotyczył **7 profili**, nie 4 — sprostowane także
  w `MASTER_IMPLEMENTATION_PLAN.md`.
- `uname -r` w logu (`7.0.0-1019-azure`) to jądro hosta OCI, nie jądro dystrybucji.

**Status F-03 po erracie:** `OPEN`; formalna bramka
`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` (`OPEN`, `NOT_STARTED`) poprzedza M3-G.
