# RAPORT PRZEDIMPLEMENTACYJNY
# STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE

**Projekt:** SPLogbook
**Data projektu:** 2026-11-09
**Data i czas środowiska:** 2026-11-09T14:28:29+01:00 (2026-11-09T13:28:29Z, Europe/Warsaw)
**Tryb:** AUTONOMOUS REPOSITORY AGENT — faza analityczna (bez zmian w kodzie do momentu zakończenia tego raportu)

---

## A. Data projektu

2026-11-09. Zgodna z `AGENTS.md` §1 i z datą hosta. Nie cofnięto zegara ani dat commitów.

## B. Data i czas środowiska

- Czas lokalny: 2026-11-09T14:28:29+01:00
- UTC: 2026-11-09T13:28:29Z
- Strefa: Europe/Warsaw (CET, +01:00)
- Host: Windows 10.0.26300, `x86_64-pc-windows-msvc`
- Toolchain: `rustc 1.99.0` (b940084d7), `cargo 1.99.0`, Git 2.56.0.windows.4

## C. HEAD / main / origin/main

| Ref | SHA |
|---|---|
| HEAD | `351124c6c748459864e75f1d51292dbdfa004aaf` |
| main | `351124c6c748459864e75f1d51292dbdfa004aaf` |
| origin/main | `351124c6c748459864e75f1d51292dbdfa004aaf` |

`git fetch --all --tags --prune` wykonany; EXIT=0. Oczekiwany SHA z promptu jest aktualny.
Gałąź roboczo-checkoutowana: `repair/full-system-remediation` wskazuje ten sam commit co `main`.
`origin/repair/full-system-remediation` = `f73a703`, który jest **przodkiem** `main`
(merge-base = `f73a703`), więc nie zawiera żadnych nowych commitów — brak rozbieżności.

## D. Ahead / behind

`git rev-list --left-right --count main...origin/main` = **0 / 0**.

## E. Stan drzewa

- `git status --porcelain=v1` = pusty → **CLEAN**
- `git ls-files -o --exclude-standard` = pusty (brak nieśledzonych plików widocznych dla Git)
- Brak submodułów.
- Tag `v1.2.0`: obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled
  `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — **nienaruszony**, zgodny z `AGENTS.md` §2.10.

## F. Dokumenty kanoniczne

Odczytane w całości lub w zakresie właściwym dla bramki:

1. `AGENTS.md` (84 linie, brak innych `AGENTS.md` w repo),
2. `docs/PROJECT_CURRENT_STATE.md`,
3. `docs/NEXT_WORK_GATE.md`,
4. `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md` i `.json`,
5. `STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md`,
6. `STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md` (fragmenty decyzji),
7. `OWNER-M3-F-LINUX-BASELINE-01-PROPOSAL.md` (fragmenty),
8. `docs/DECISION_TRACEABILITY_MATRIX.md` §3–§4,
9. `docs/DOCUMENTATION_INDEX.md` §5,
10. `docs/DEEPSEEK_HARNESS_HANDOFF.md` (fragmenty F-01/F-03/F-04, §5, §19),
11. `MASTER_IMPLEMENTATION_PLAN.md` (M3-F §301, §341–345),
12. `DESIGN_3_0_B_LINUX_UPDATE.md` (polityka ABI, §9, §412–413),
13. `.github/workflows/ci.yml`, `.github/workflows/build-and-release.yml`,
14. `scripts/build_linux_portable_tar_gz.sh`, `scripts/verify_linux_container.sh`,
15. `tests/security_guard.rs` (strażnik M3-F oraz QA), `tests/linux_multi_distribution_compatibility_test.rs`,
    `tests/linux_portable_tar_gz_test.rs`, `tests/linux_archive_security_test.rs`,
16. `README.md`, `installer/linux/portable/README.md`, `docs/IMPROVEMENT_ROADMAP.md`,
    `docs/REMEDIATION_BASELINE.md`.

Dokumenty usunięte/przeniesione: `DOCUMENTATION_INDEX.md` §6 wskazuje, że `PLAN.md`,
`DESIGN_2_2_FINAL.md` itp. **nie istnieją** — nie szukano ich jako źródeł.

## G. Potwierdzona następna bramka

`docs/NEXT_WORK_GATE.md:22` wskazuje dokładnie `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`
jako jedyną następną bramkę, ze statusem `NOT_STARTED` i spełnioną prekondycją (QA + zależności).
Nazwa **zgodna** z promptem, bramka **nie została wykonana**, nie została zastąpiona ani połączona.
Brak nowej decyzji OWNER blokującej tę bramkę — wręcz przeciwnie, prompt dostarcza decyzję
`OWNER-M3-F-KERNEL-POLICY: APPROVE OPTION A` odpowiadającą na otwarte pytanie z `NEXT_WORK_GATE.md` §„Pytanie OWNER”.

**Konflikt do rozstrzygnięcia w tej bramce:** `AGENTS.md` §5 pkt 22–24, `NEXT_WORK_GATE.md` §Zakres pkt 1–2,
`PROJECT_CURRENT_STATE.md` §5 i `DECISION_TRACEABILITY_MATRIX.md` §4 zapisują **stałe** minimum
`7.2` (`OWNER_APPROVED_FIXED_MINIMUM`), a prompt wprowadza nowszą decyzję **OPTION A** (minimum
dynamiczne, bez sztywnego 7.2). Rozstrzygnięcie: prompt jest nowszą, jawną decyzją OWNER i ma
pierwszeństwo; dokumenty repo zostaną wyrównane w tej bramce, a nie odwrotnie.

## H. Stan M3-F portable tar.gz

- `STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE`: zamknięta (`COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED`).
- Artefakt kanoniczny: `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`, 19 479 546 B,
  SHA-256 `5df6ac20…b7ac`, CI run `42518873662`.
- Nie ponawiam implementacji tar.gz.
- **Potwierdzona wada dowodowa:** `scripts/verify_linux_container.sh:32-37` używa
  `ldd … || true` i `SPLogbook --version || true`, więc błąd ładowania ELF (np. brak
  `GLIBC_2.43`) **nie przerywa** joba; wynik `VERIFIED_PASS` dla 7 profili nie jest fail-closed
  (F-03). Dodatkowo snapshot deklaruje `maximum_required_glibc_symbol = GLIBC_2.43` i jednocześnie
  `VERIFIED_PASS` na Debian 13.7 (glibc 2.41) oraz Leap 16.1 (glibc 2.40) — twierdzenia wzajemnie
  wykluczające się (F-04).

## I. Stan M3-G

`BLOCKED` / `NOT_STARTED` — zgodnie z `NEXT_WORK_GATE.md:79-81` i `PROJECT_CURRENT_STATE.md:90`.
Nie rozpoczęto żadnych prac M3-G.

## J. Deklaracje polityki znalezione w repozytorium

| Miejsce | Deklaracja |
|---|---|
| `scripts/build_linux_portable_tar_gz.sh:179` | `RESOLVED_MIN_KERNEL="6.12"` |
| `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json:19` | `resolved_minimum_kernel: "6.12"`, 7 profili |
| `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md:41-44` | minimum 6.12, floor: Debian + Leap |
| `tests/security_guard.rs:4966` | `resolved_minimum_kernel == "6.12"`, 7 profili |
| `tests/linux_multi_distribution_compatibility_test.rs:35-99,154` | 7 profili, floor `(6,12)`, Debian glibc błędnie `2.40` |
| `tests/linux_portable_tar_gz_test.rs:34`, `linux_archive_security_test.rs:46` | marker `"6.12"` |
| `.github/workflows/build-and-release.yml:218-247` | macierz 7 obrazów (w tym Ubuntu 26.10, Debian, Leap) |
| `README.md:531` | „Ubuntu 20.04+, Debian 11+, Fedora 36+…” — sprzeczne z każdym minimum |
| `installer/linux/portable/README.md:9-13` | macierz 7 dystrybucji |
| `AGENTS.md:53-58` | intencja OWNER: kernel 7.2 fixed, glibc 2.43, Debian/Leap nie, „jeszcze nie w repo” |
| `docs/NEXT_WORK_GATE.md:51-61` | zakres: fixed 7.2, usunięcie Debian/Leap, `|| true` |
| `MASTER_IMPLEMENTATION_PLAN.md:301` | M3-F „dynamiczny floor 6.12”, 7 obrazów |

## K. Zweryfikowane dystrybucje i kernele (stan na 2026-11-09)

Oficjalne źródła pierwszej strony (data pobrania: 2026-11-09):

| Dystrybucja | Stabilne wydanie | Domyślny kernel (linia) | Pakiet kernela | glibc | Źródło |
|---|---|---|---|---|---|
| Ubuntu 26.04 LTS | 26.04.1 LTS „Resolute Raccoon” (2026-08-26) | **7.0** | `linux-image-generic 7.0.0-42.42` (`7.0.0-42-generic`) | **2.43** (`2.43-2ubuntu2.4`) | releases.ubuntu.com/26.04; packages.ubuntu.com/resolute |
| Debian 13 | 13.7 „Trixie” (2026-09-12) | **6.12** | `linux-image-amd64 6.12.101-1` | **2.41** (`2.41-12+deb13u4`) | debian.org/releases/trixie; packages.debian.org/trixie |
| Fedora 45 | 45 GA | **7.2** | `kernel 7.2.8-300.fc45` (stable; 7.2.9 w updates-testing) | **2.44** (`2.44-1.fc45`) | packages.fedoraproject.org |
| Arch Linux | rolling, official core | **7.2** | `linux 7.2.14.arch1-1` (build 2026-10-30, upd. 2026-11-01) | **2.44+r50** | archlinux.org/packages/core/x86_64/linux |
| openSUSE Tumbleweed | rolling snapshot (weekly 2026/43) | **7.2** | `kernel-default 7.2.11-1` (weekly 2026/43: 7.2.11) | **2.44** | news.opensuse.org (monthly Oct 2026, weekly 2026/43) |

Rozdzielenie kategorii: powyżej podano **kernel stabilnego repozytorium**. Kanały
`updates-testing` (Fedora 7.2.9), `Rawhide`, backports i RC **nie** wyznaczają minimum.
Kernel obrazu instalacyjnego Ubuntu 26.04.1 może być starszy w linii 7.0 niż 7.0.0-42 — linia
kernela jest ta sama; dla polityki liczy się linia, nie numer EasyAntiCheat patcha.

## L. Potwierdzone sprzeczności

1. **Kernel:** repo koduje 6.12 i 7 profili; OWNER (OPTION A) wymaga minimum dynamicznego
   z aktualnych, oficjalnie wspieranych dystrybucji — bez sztywnego 7.2.
2. **glibc vs Debian/Leap:** artefakt deklaruje maksymalny wymagany symbol `GLIBC_2.43`;
   Debian 13.7 ma glibc 2.41, Leap 16.1 — 2.40. Debian/Leap **nie mogą** spełnić wymagania ABI,
   a mimo to snapshot i CI deklarują `VERIFIED_PASS`. Sprzeczność wewnętrzna (F-04).
3. **Dowód natywny:** `verify_linux_container.sh` maskuje błędy `|| true` (F-03) — status
   `VERIFIED_PASS` nie jest fail-closed.
4. **Dane nieaktualne/błędne w snapshotach:** Arch 6.18.2 (faktycznie 7.2.14), Fedora 45 6.18.1
   (faktycznie 7.2.8), Ubuntu 26.04 7.0.0-18 (faktycznie 7.0.0-42), Debian 6.12.43-1 (faktycznie
   6.12.101-1), Debian glibc w teście 2.40 (faktycznie 2.41), Tumbleweed 6.18.2 (faktycznie 7.2.11).
5. **README:** deklaruje Ubuntu 20.04+/Debian 11+/Fedora 36+ — sprzeczne z polityką i z ABI.
6. **AGENTS.md vs prompt:** AGENTS.md §5 mówi „fixed 7.2”, prompt — „OPTION A dynamiczne”.
   Rozstrzygnięcie: prompt (nowsza decyzja OWNER) wygrywa; AGENTS.md §5 do wyrównania.

## M. Proponowane minimum kernela

**Linux 7.0** — najstarsza domyślna linia kernela wśród **oficjalnie wspieranych** dystrybucji
po wykluczeniu Debian 13.7 i Leap 16.1 przez wymóg ABI `glibc >= 2.43`:

```
MIN( Ubuntu 26.04 LTS 7.0, Fedora 45 7.2, Arch 7.2, Tumbleweed 7.2 ) = 7.0
```

Uzasadnienie: brak w `src/` użycia API jądra nowszego niż 7.0 (brak `libc::`/syscalli; jedyne
odwołanie do `/proc/<pid>` jest w helperze Windows i nie dotyczy Linuksa). Minimum wynika z
polityki dystrybucyjnej, nie z wymagań API jądra. **Nie** jest to najwyższa wersja rolling-release
(7.2) — zgodnie z OPTION A pkt 1–2.

Wariant alternatywny (gdyby OWNER chciał wesprzeć Debian 13.7): minimum spadłoby do 6.12, ale
wymagałoby przebudowy artefaktu pod glibc ≤ 2.41 — **poza zakresem** tej bramki.

## N. Proponowane minimum glibc

**2.43** (`GLIBC_2.43`), zgodnie z: (a) zmierzonym maksymalnym symbolem artefaktu
(`STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md:119`), (b) wiążącą intencją
OWNER `OWNER-M3-F-GLIBC-POLICY-01` zapisaną w `AGENTS.md:54`, `NEXT_WORK_GATE.md:53`,
`DECISION_TRACEABILITY_MATRIX.md:59`. Najstarsza glibc wśród wspieranych dystrybucji to również
2.43 (Ubuntu 26.04 LTS) — oba ograniczenia są zgodne.

## O. Plan minimalnych zmian

1. Przepisać snapshot do schematu v2: minimum 7.0/glibc 2.43, 4 profile wspierane, sekcja
   `evaluated_not_supported` (Debian 13.7, Leap 16.1, Ubuntu 26.10), data snapshotu, zasada rewizji.
2. `scripts/build_linux_portable_tar_gz.sh`: `RESOLVED_MIN_KERNEL="7.0"`.
3. `scripts/verify_linux_container.sh`: usunąć `|| true` z weryfikacji ELF/uruchomienia (fail-closed).
4. `.github/workflows/build-and-release.yml`: macierz 4 wspieranych obrazów; usunąć Ubuntu 26.10,
   Debian, Leap.
5. Wyrównać strażników i testy polityki (`security_guard`, `linux_multi_distribution_*`,
   `linux_portable_tar_gz_test`, `linux_archive_security_test`).
6. Zaktualizować dokumentację: `AGENTS.md` §5, `NEXT_WORK_GATE.md`, `PROJECT_CURRENT_STATE.md`,
   `DECISION_TRACEABILITY_MATRIX.md`, `DOCUMENTATION_INDEX.md`, `README.md`,
   `MASTER_IMPLEMENTATION_PLAN.md`, `IMPROVEMENT_ROADMAP.md`, `installer/linux/portable/README.md`,
   raporty M3-F (errata). Raport końcowy bramki.
7. **Bez zmian w kodzie produkcyjnym `src/`, bez zmian `Cargo.toml`/`Cargo.lock`, bez zmian
   schematu SQLite, bez tagów i release.**

## P. Pliki planowane do modyfikacji

- `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json`, `.md`
- `scripts/build_linux_portable_tar_gz.sh`, `scripts/verify_linux_container.sh`
- `.github/workflows/build-and-release.yml`, `.github/workflows/ci.yml` (komentarz liczników)
- `tests/security_guard.rs`, `tests/linux_multi_distribution_compatibility_test.rs`,
  `tests/linux_portable_tar_gz_test.rs`, `tests/linux_archive_security_test.rs`
- `AGENTS.md`, `README.md`, `MASTER_IMPLEMENTATION_PLAN.md`
- `docs/NEXT_WORK_GATE.md`, `docs/PROJECT_CURRENT_STATE.md`, `docs/DECISION_TRACEABILITY_MATRIX.md`,
  `docs/DOCUMENTATION_INDEX.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`,
  `docs/DEEPSEEK_HARNESS_HANDOFF.md`
- `installer/linux/portable/README.md`
- `STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_INTEGRATION_REPORT.md`,
  `STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md` (errata)
- nowe: ten raport wstępny + `STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`

## Q. Plan testów

Celowane:

```
cargo +stable test --test security_guard m3_f_owner_decisions_and_support_policy_guard
cargo +stable test --test linux_multi_distribution_compatibility_test
cargo +stable test --test linux_portable_tar_gz_test
cargo +stable test --test linux_archive_security_test
```

Pełny baseline (jak `docs/DEEPSEEK_HARNESS_HANDOFF.md` §5):

```
cargo +stable fmt --all -- --check
cargo +stable check --workspace --all-targets
cargo +stable clippy --workspace --all-targets -- -D warnings
cargo +stable test --workspace --all-targets -j 2
cargo +stable doc --workspace --no-deps
cargo +stable tree -d
git diff --check
```

`cargo audit` tylko jeśli dostępny i bez `--ignore` (bramka zależności zamknięta; nie otwieram
jej ponownie bez nowego advisory).

## R. Ryzyka

1. Natywna weryfikacja nowej 4-profilowej macierzy nie zostanie wykonana w tej sesji (host
   Windows, brak Docker/Linux). Bramka wyrównuje **politykę i dowody dokumentacyjne**; rerun CI
   po pushu jest osobnym, jawnym krokiem. Nie deklaruję natywnego PASS dla nowej macierzy.
2. Usunięcie `|| true` może ujawnić rzeczywistą niezgodność ABI na którymś profilu — to zamierzone
   (fail-closed), ale wynik CI jest nieznany do czasu uruchomienia.
3. `ubuntu-latest` w jobie budującym nie jest przypięty — baseline glibc budowy może dryfować
   (ryzyko rezydualne; poza minimalnym zakresem tej bramki, raportowane).
4. Liczniki testów zmienią się po dodaniu/zmianie testów — zaktualizuję je jawnie, bez przepisywania
   starych wyników.
5. Pliki `docs/DECISION_TRACEABILITY_MATRIX.md` i `docs/DOCUMENTATION_INDEX.md` zawierają
   wcześniejsze uszkodzenia kodowania (mojibake); edycje będą punktowe, bez masowego przepisywania.

## S. Warunki zatrzymania

Zatrzymam się i sporządzę raport blokujący, jeżeli: origin/main się zmieni lub drzewo stanie się
brudne; okaże się, że któraś z wersji stabilnych jest niejednoznaczna; testy bazowe będą czerwone
przed zmianami; albo okaże się, że wymagana jest zmiana ABI artefaktu (przebudowa pod glibc ≤ 2.41)
— to wykracza poza tę bramkę i wymaga decyzji OWNER.
