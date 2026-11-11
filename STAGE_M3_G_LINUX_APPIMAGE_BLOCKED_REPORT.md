# STAGE-M3-G — RAPORT BLOKUJĄCY (AppImage, Linux x86_64)

**Dokument ID:** DSH-SPLOGBOOK-M3G-BLOCKED-001
**Bramka:** `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`
**Status bramki:** **`BLOCKED_NEW_DEPENDENCY_ADVISORY`** (bramka `IN_PROGRESS`, **nie zamknięta**)
**Aktualna data projektu:** **2026-11-11**
**Czas pomiaru:** 2026-11-11, sesja M3-G (`+01:00`, Europe/Warsaw)

> Zgodnie z §27 promptu bramki nowe advisory zależności **zatrzymuje M3-G** i **nie jest
> naprawiane przy okazji**. Ten raport dokumentuje wykonaną pracę, dowody, dokładny blocker,
> problemy infrastrukturalne CI oraz bezpieczne opcje. Bramka **nie** otrzymuje statusu
> `COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`.

---

## 1. Podsumowanie

| Element | Wynik |
|---|---|
| Implementacja AppImage (skrypty, AppRun, marker, workflow, testy, strażnik) | **WYKONANA** |
| Baseline lokalny (fmt, check, clippy, testy, doc, tree, YAML, JSON, `git diff --check`) | **ZIELONY** |
| Toolchain `appimagetool` continuous + type-2 runtime | **ROZWIĄZANY, ZWERYFIKOWANY, PRZYPIĘTY** |
| Build AppImage w CI | **SUKCES** (`Build Linux x86_64` = `success`) |
| Weryfikacja AppImage w profilach | **2/4 `success`** (Fedora 45, Arch Linux); Ubuntu/openSUSE — problemy infrastrukturalne/`findutils` |
| FUSE | **`VERIFIED`** — rzeczywisty mount (run `43022317306`) |
| Testy negatywne (12 mutacji) | **nieukończone w CI** — przerwane przez awarię DNS w kontenerze |
| `cargo audit --deny warnings` | **FAILURE** — `RUSTSEC-2026-0486` (`rustix 1.1.5`) |
| Formalne zamknięcie bramki | **ZABLOKOWANE** |

## 2. Blocker (jedyne blokujące ustalenie merytoryczne)

| Pole | Wartość |
|---|---|
| Kod | `BLOCKED_NEW_DEPENDENCY_ADVISORY` |
| Advisory | `RUSTSEC-2026-0486` |
| Crate / wersja | `rustix 1.1.5` |
| Typ | `informational = unsound` (xattr: zero-length buffer → UB) |
| Zakres | `>= 1.0.0, < 1.1.6` |
| Patch | `>= 1.1.6` |
| `rustix 0.38.44` | **nieobjęty** advisory |
| Ścieżka w grafie | `rustix 1.1.5` ← `async-io` / `calloop` / `gethostname` ← `zbus` / `eframe` / `x11rb` ← `splogbook` |
| Wykrycie | `cargo audit --deny warnings --no-yanked` (lokalnie) oraz job `CI` → `Install and run cargo audit` = `failure` |
| Runy potwierdzające | `43015927219`, `43022317421`, `43031361969` |
| Zmiana w tej bramce | **brak** — `Cargo.toml`/`Cargo.lock` nietknięte (`git diff` pusty) |

**Dlaczego to blokuje:** `cargo audit` jest częścią joba `CI`; przy `--deny warnings` nowe
advisory kończy job błędem, więc baseline CI nie jest zielony. §27 promptu nakazuje zatrzymanie
bramki, a naprawa (aktualizacja `rustix` do `>= 1.1.6`) jest zmianą zależności — dozwoloną
wyłącznie w dedykowanej bramce bezpieczeństwa po decyzji OWNER.

## 3. Co zostało wykonane i zweryfikowane

### 3.1 Wejście i Git

| Pole | Wartość |
|---|---|
| Commit wejściowy | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `HEAD = main = origin/main` (wejście) | `81db60c…` |
| Ahead/behind (wejście) | `0 / 0` |
| `v1.2.0` | obiekt `3ea8d51a…`, peeled `6f7d3aaa…` — nietknięty |
| Stash | 2, nietknięte |
| Submoduły | brak |
| Jedna sesja | potwierdzona |

### 3.2 Toolchain (resolve-once, pin)

| Pole | `appimagetool` | runtime type 2 |
|---|---|---|
| Źródło | `AppImage/appimagetool` | `AppImage/type2-runtime` |
| Kanał/release | `continuous` | `continuous` |
| Source commit | `11d54c9acc7f246df1398050a4b880314ac76862` | `fedfad51acc2860b2916a1eef32ba4a500ff31ef` |
| Release id | `435944148` | `417724209` |
| Publikacja | `2026-11-06T00:48:33Z` | `2026-10-19T20:37:20Z` |
| Asset | `appimagetool-x86_64.AppImage` | `runtime-x86_64` |
| Rozmiar | `15133176` B | `944632` B |
| Oficjalny SHA-256 | `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` | `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46` |
| Lokalny SHA-256 | identyczny | identyczny |
| Digest match | **TRUE** | **TRUE** |

Wcześniej zatwierdzony runtime `20251108`
(`sha256:2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d`, ten sam rozmiar)
pozostaje odnotowany; DESIGN nie przypina tego release, więc użyto najnowszego zweryfikowanego
runtime. Runtime jest statycznie linkowany (brak `PT_INTERP`, `DT_NEEDED`, `GLIBC_*`).

### 3.3 Artefakt AppImage (run `43031361998`, commit `c5ee6dd`)

| Pole | Wartość |
|---|---|
| Nazwa | `SPLogbook-Linux-x86_64.AppImage` |
| Rozmiar | `15 591 928` B |
| SHA-256 | `01a93a68fae287019fdd3c5891f2eb9612c71d3cf830f0b1771062d0b493e815` |
| build_commit | `c5ee6dda6ba33636a454406e88fca54d27de56c3` |
| Maksymalny wymagany GLIBC | `2.43` |
| AppDir | `AppRun`, desktop, ikona, `.DirIcon`, `usr/bin/{SPLogbook,splogbook-updater}`, `usr/lib/libhamlib.so.4`, `usr/share/...`, marker — bez symlinków, bez AppStream |
| Bundlowane biblioteki | wyłącznie `libhamlib.so.4` |
| RUNPATH | `$ORIGIN/../lib` |
| Marker | `installation_type=LinuxAppImage`, `resolved_minimum_kernel=7.0`, `maximum_glibc=2.43`, `artifact_format=AppImage`, `architecture=x86_64`, `appimagetool_channel=continuous` |

### 3.4 Testy i baseline lokalny (2026-11-11)

| Komenda | Wynik |
|---|---|
| `cargo +stable fmt --all -- --check` | EXIT 0 |
| `cargo +stable check --workspace --all-targets -j 1` | EXIT 0 |
| `cargo +stable clippy --workspace --all-targets -j 1 -- -D warnings` | EXIT 0, 0 ostrzeżeń |
| `cargo +stable test --workspace --all-targets -j 1` | EXIT 0 — 20 targetów, **2717** zarejestrowanych, **2707** wykonanych, **2707** zaliczonych, **0** nieudanych, **10** ignorowanych |
| `cargo +stable doc --workspace --no-deps` | EXIT 0 |
| `cargo +stable tree -d` | EXIT 0 (duplikaty pre-existing) |
| `cargo audit --deny warnings …` | **EXIT 1** — `RUSTSEC-2026-0486` |
| Walidacja YAML (`build-and-release.yml`, `ci.yml`) | PASS |
| Walidacja JSON | PASS |
| `git diff --check` | EXIT 0 |

Strażnicy: `security_guard` 123/123 (w tym `m3_g_appimage_integration_guard`,
`m3_f_f03_fail_closed_verification_guard`). `F-03` pozostaje `CLOSED` i fail-closed.
Uwaga środowiskowa: masowy `tempdir` `PermissionDenied` w `%TEMP%` hosta został obejściem
`TMP`/`TEMP` w katalogu workspace; to defekt środowiska, nie kodu.

### 3.5 Commity wypchnięte w tej sesji (ff-only)

| Commit | Tytuł | CI |
|---|---|---|
| `e5fc7c19f73d910dd5a560fcd2bf191b419cce38` | `feat(linux): add verified AppImage packaging pipeline` | `failure` — ratchet liczników (`43011245904`) |
| `eb414e9a74013139e90e81861ae84f6c392fe104` | `fix(qa): align canonical test counters with the M3-G baseline` | `failure` — tylko `cargo audit` (`43015927219`) |
| `0fd724c2c7cb793db0d629c21851bf0eb3bea09b` | `fix(linux): stabilise AppImage artifact metadata, FUSE packages and CRLF test` | `failure` — tylko `cargo audit` (`43022317421`); **FUSE `VERIFIED`** |
| `c5ee6dd` | `fix(linux): restore AppImage exec bit after download and harden Arch runtime install` | `failure` — `cargo audit` + zdarzenia infrastrukturalne (`43031361998`) |
| `3486ebf1b344764a0c8c10066585415a3082ea5d` | `fix(linux): install findutils in every supported runtime profile` | run `43037615693`/`43037615578` **zawieszony** na `Install system dependencies` (infrastruktura) |

Wszystkie pushe: fast-forward, bez force, bez amend, bez rebase.

### 3.6 Dowody CI (run `43031361998`, commit `c5ee6dd`)

| Job | Wynik |
|---|---|
| Build Linux x86_64 (portable + AppImage + smoke) | `success` |
| Build Windows x64 | `success` |
| Verify Linux (Fedora 45) — portable + AppImage | **`success`** (pełny PASS) |
| Verify Linux (Arch Linux) — portable + AppImage | **`success`** (pełny PASS) |
| Verify Linux (Ubuntu 26.04.1 LTS) | `failure` — `apt-get update` nie rozwiązał DNS w kontenerze |
| Verify Linux (openSUSE Tumbleweed) | `failure` — brak `findutils` (naprawione w `3486ebf`, brak ponownej weryfikacji) |
| Verify Linux (negative fixture, F-03) | `failure` — brak logu (awaria DNS/apt w kontenerze) |
| Verify AppImage (negative fixtures) | `failure` — `apt-get update` nie rozwiązał DNS |
| Verify AppImage (real FUSE) | `failure` — awaria apt/DNS w tym runie; **w runie `43022317306` = `success`** |
| Publish Release Assets | `skipped` |

Dowód FUSE (run `43022317306`, commit `0fd724c`): `--version` = `SPLogbook 1.3.0-alpha.1`,
`--build-info` = `Commit: 0fd724c…`, `Target: linux-x86_64`, wykonane przez **rzeczywisty mount
FUSE** (`REAL FUSE MOUNT EXECUTION VERIFIED`).

### 3.7 Dowód pełnego PASS AppImage (Fedora 45 i Arch Linux)

Z logów `appimage-evidence-{fedora-final,arch-rolling}.log`:

```
PASS: AppImage SHA-256 integrity verified
PASS: AppImage type 2 magic verified (AI\x02)
PASS: approved AppDir layout is complete
PASS: AppRun runs from any CWD, forwards arguments verbatim, preserves exit codes and rejects a missing binary
PASS: binaries are ELF64 x86-64 and libhamlib.so.4 is the only bundled library
PASS: AppImage marker verified (build identity and pinned toolchain metadata)
PASS: desktop files are byte-identical to the canonical template and validate
PASS: all icon copies are byte-identical 256x256 PNG files
PASS: no user data, databases, tokens or private keys inside the AppDir
PASS: RUNPATH is exactly $ORIGIN/../lib for both usr/bin binaries
PASS: ldd resolved every direct dependency (no 'not found')
PASS: GLIBC policy satisfied (required GLIBC_2.43 <= GLIBC_2.43 <= available 2.44)
PASS: --version and --build-info executed and report the expected build identity
PASS: XDG isolation, health check and AppDir immutability verified
APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY
```

## 4. Czego NIE wykonano (kryteria zamknięcia)

- **#37 baseline zielony** — `cargo audit` czerwony (blocker merytoryczny).
- **#35 cztery profile** — 2/4 `success`; pozostałe przerwane przez infrastrukturę CI
  (DNS/apt) i brak `findutils` w openSUSE (poprawka `3486ebf` oczekuje na ponowny run).
- **#33 testy negatywne** — nieukończone w CI z powodu awarii DNS w kontenerze.
- **#36 FUSE** — dowód dodatkowy: `VERIFIED` w runie `43022317306`; w późniejszych runach
  job przerwany przez awarię apt.
- **#46–#49** — brak commita dokumentacyjnego zamykającego bramkę; `M3_G_STATUS` pozostaje
  `IN_PROGRESS` / `BLOCKED_NEW_DEPENDENCY_ADVISORY`.
- **#121** — ostateczny status M3-G: **`BLOCKED_NEW_DEPENDENCY_ADVISORY`**, nie
  `COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`.

## 5. Bezpieczne opcje (wymagają decyzji OWNER)

| Opcja | Opis | Wpływ |
|---|---|---|
| **A (rekomendowana)** | Dedykowana bramka bezpieczeństwa zależności: `cargo update -p rustix@1.1.5 --precise 1.1.6` (lub nowszy patch), pełny baseline + `cargo audit`, osobny commit; następnie wznowienie M3-G bez zmian w artefakcie AppImage | zmiana `Cargo.lock` poza M3-G; najczystsze, trwałe |
| **B** | Formalne `--ignore RUSTSEC-2026-0486` z uzasadnieniem (funkcje xattr nieużywane przez SPLogbook) | osłabia `cargo audit`; wymaga jawnej decyzji i wpisu do macierzy |
| **C** | Odczekanie na aktualizację łańcucha (`zbus`/`async-io`/`calloop`) do `rustix >= 1.1.6` | bez zmian w repo; przesuwa zamknięcie w czasie |
| **D** | Zamknięcie M3-G „warunkowo” z pominięciem `cargo audit` | **niedozwolone** — `AGENTS.md` §7 i §27 wymagają zielonego audytu |

## 6. Jedno pytanie do OWNER

Czy OWNER zatwierdza **Opcję A** — osobną bramkę aktualizacji zależności
(`rustix 1.1.5 → >= 1.1.6`, wyłącznie `Cargo.lock` przez `cargo update -p`), po której M3-G
zostanie wznowione (ponowny run CI dla 4/4 profili, testów negatywnych i FUSE) i domknięte
bez zmian w artefakcie AppImage?

## 7. Stan Git po tej sesji

Stan bezpośrednio **przed** commitem tego raportu:

```
HEAD        = 3486ebf1b344764a0c8c10066585415a3082ea5d
main        = 3486ebf1b344764a0c8c10066585415a3082ea5d
origin/main = 3486ebf1b344764a0c8c10066585415a3082ea5d
ahead/behind = 0/0
TRACKED_WORKTREE_CLEAN = true
UNTRACKED_FILES_PRESENT = true (scratch/)
IGNORED_FILES_PRESENT = true
STRICT_WORKTREE_CLEAN = false
v1.2.0 = 3ea8d51a… / peeled 6f7d3aaa… (nietknięty)
```

Commit tego raportu jest wypychany metodą fast-forward (`git push origin main`, bez force);
po pushu `HEAD == main == origin/main`, `ahead/behind = 0/0`. Dokładny SHA commita raportu
i końcowa rekonstrukcja Git są podane w odpowiedzi końcowej sesji.
