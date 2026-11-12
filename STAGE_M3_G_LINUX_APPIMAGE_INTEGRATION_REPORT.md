# STAGE-M3-G — RAPORT KOŃCOWY (AppImage, Linux x86_64)

**Dokument ID:** DSH-SPLOGBOOK-M3G-FINAL-001
**Bramka:** `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`
**Status bramki:** **`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`**
**Aktualna data projektu:** **2026-11-12**
**Czas pomiaru:** 2026-11-12 (`+01:00`, Europe/Warsaw)

> Bramka została wznowiona i zamknięta po usunięciu blokera zależności
> (`RUSTSEC-2026-0486`, decyzja `OWNER-DEP-RUSTIX-01`). Wcześniejszy stan i dowody
> zatrzymania: `STAGE_M3_G_LINUX_APPIMAGE_BLOCKED_REPORT.md` (dokument historyczny).

---

## 1. Decyzje OWNER

| ID | Decyzja | Wykonanie |
|---|---|---|
| `OWNER-M3-G-01` | GO | bramka wykonana w jednej sesji |
| `OWNER-M3-G-02` | APPROVE PRE-IMPLEMENTATION REPORT | raport przedimplementacyjny przyjęty i zaktualizowany |
| `OWNER-M3-G-03-R1` | APPROVE LATEST VERIFIED APPIMAGETOOL CONTINUOUS BUILD | `continuous` @ `11d54c9…` |
| `OWNER-M3-G-04-R1` | RESOLVE AND VERIFY THE LATEST OFFICIAL TYPE-2 RUNTIME | `continuous` @ `fedfad51…` |
| `OWNER-M3-G-05` | APPROVE IMPLEMENTATION PHASE | implementacja wykonana |
| `OWNER-M3-G-06` | KEEP PUBLIC RELEASE DISABLED | `Publish Release Assets` = `skipped` |
| `OWNER-M3-G-07` | RELEASE MANIFEST / UPDATER OUT OF SCOPE | brak zmian manifestu i API updatera |
| `OWNER-M3-G-08` | USE LATEST VERIFIED APPIMAGETOOL CONTINUOUS BUILD | brak fallbacku do 1.9.1 |
| `OWNER-M3-G-09` | PREFER LATEST AVAILABLE LIBRARIES AND TOOLS | najnowszy zweryfikowany runtime i tool |
| `OWNER-DEP-RUSTIX-01` | APPROVE OPTION A (osobna bramka zależności) | `STAGE-DEPENDENCY-RUSTIX-RUSTSEC-2026-0486-REMEDIATION-GATE`, commit `35464c7` |

## 2. Wejście i ciągłość Git

| Pole | Wartość |
|---|---|
| Commit wejściowy bramki | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| Commit remediacji zależności | `35464c7080e51fc151282d035487a5983bf1d13f` |
| Commit implementacyjny M3-G | `e5fc7c19f73d910dd5a560fcd2bf191b419cce38` |
| Commity korygujące | `eb414e9`, `0fd724c`, `c5ee6dd`, `3486ebf` |
| `F03_STATUS` | `CLOSED` (nietknięte) |
| `v1.2.0` | obiekt `3ea8d51a…`, peeled `6f7d3aaa…` — nietknięty |
| Stash / submoduły | 2 (nietknięte) / brak |
| Jedna sesja | potwierdzona |

## 3. Toolchain

| Pole | `appimagetool` | runtime type 2 |
|---|---|---|
| Źródło | `AppImage/appimagetool` | `AppImage/type2-runtime` |
| Kanał | `continuous` | `continuous` |
| Source commit | `11d54c9acc7f246df1398050a4b880314ac76862` | `fedfad51acc2860b2916a1eef32ba4a500ff31ef` |
| Release id | `435944148` | `417724209` |
| Publikacja | `2026-11-06T00:48:33Z` | `2026-10-19T20:37:20Z` |
| Asset | `appimagetool-x86_64.AppImage` | `runtime-x86_64` |
| Rozmiar | `15 133 176` B | `944 632` B |
| SHA-256 (oficjalny = lokalny) | `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` | `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46` |

Runtime jest przekazany jawnie `--runtime-file`, statycznie linkowany (brak `PT_INTERP`,
`DT_NEEDED`, `GLIBC_*`). Wcześniejszy pin `20251108` odnotowany; DESIGN go nie wymaga.

## 4. Artefakt AppImage (run `43166825054`, commit `35464c7`)

| Pole | Wartość |
|---|---|
| Nazwa | `SPLogbook-Linux-x86_64.AppImage` |
| Rozmiar | `15 583 736` B |
| SHA-256 | `8eb88eaad025a1b120362d0db95dd1c5a1d7165f6b354b0ecc4c6983fda3b729` |
| build_commit | `35464c7080e51fc151282d035487a5983bf1d13f` |
| Maksymalny wymagany GLIBC | `2.43` |
| Struktura AppDir | `AppRun`, `splogbook.desktop`, `splogbook.png`, `.DirIcon`, `usr/bin/{SPLogbook,splogbook-updater}`, `usr/lib/libhamlib.so.4`, `usr/share/applications/…`, `usr/share/icons/hicolor/256x256/apps/…`, `usr/share/doc/splogbook/…`, `usr/share/splogbook/.splogbook-appimage-marker` |
| Symlinki / setuid / world-writable | brak / brak / brak |
| Bundlowane biblioteki | wyłącznie `libhamlib.so.4` |
| RUNPATH | `$ORIGIN/../lib` |
| Desktop | `Exec=SPLogbook`, `Icon=splogbook`, `StartupWMClass=SPLogbook`; identyczne bajty w obu kopiach; `desktop-file-validate` PASS |
| Ikony | `assets/icon.png` 256×256; identyczne bajty w 3 kopiach; `.DirIcon` regularny plik |
| Marker | `installation_type=LinuxAppImage`, `resolved_minimum_kernel=7.0`, `maximum_glibc=2.43`, `artifact_format=AppImage`, `architecture=x86_64`, `appimagetool_channel=continuous`, commit+SHA toolchainu i runtime |
| Update information / podpis | brak / brak |

## 5. Weryfikacja w czterech profilach (ten sam artefakt)

| Profil | Obraz OCI (pinned) | glibc | `--version` | `--build-info` | GLIBC max | `not found` | Wynik |
|---|---|---|---|---|---|---|---|
| Ubuntu 26.04.1 LTS | `ubuntu:26.04@sha256:6a06111c…` | `2.43` | `SPLogbook 1.3.0-alpha.1` | `Commit: 35464c7…`, `Target: linux-x86_64` | `2.43` ≤ `2.43` | **0** | **PASS** |
| Fedora 45 | `fedora:45@sha256:a7d72ba2…` | `2.44` | `SPLogbook 1.3.0-alpha.1` | `Commit: 35464c7…`, `Target: linux-x86_64` | `2.43` ≤ `2.43` | **0** | **PASS** |
| Arch Linux | `archlinux:base-20261101.0.620205@sha256:b1c2fe6e…` | `2.44+r50` | `SPLogbook 1.3.0-alpha.1` | `Commit: 35464c7…`, `Target: linux-x86_64` | `2.43` ≤ `2.43` | **0** | **PASS** |
| openSUSE Tumbleweed | `opensuse/tumbleweed:latest@sha256:6e3a892d…` | `2.44` | `SPLogbook 1.3.0-alpha.1` | `Commit: 35464c7…`, `Target: linux-x86_64` | `2.43` ≤ `2.43` | **0** | **PASS** |

Każdy profil zakończył `APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY` oraz (portable tar.gz)
`NATIVE VERIFICATION COMPLETED SUCCESSFULLY`. Dowód XDG: dane fixture w tymczasowym
`XDG_DATA_HOME` niezmienione. Dowód immutability: hash drzewa AppDir i hash pliku AppImage
identyczne przed/po uruchomieniu.

## 6. Testy negatywne AppImage — 12/12 odrzuconych

| Mutacja | Exit | Powód odrzucenia |
|---|---|---|
| brak `.sha256` | 1 | `checksum file is missing` |
| błędna suma | 1 | `SHA-256 integrity check failed (sidecar does not match the file)` |
| obcięty obraz | 1 | `AppImage extraction failed` |
| brak AppRun | 1 | `AppDir is missing AppRun` |
| złe prawa AppRun | 1 | `AppRun is not executable` |
| brak głównej binarki | 1 | `AppDir is missing usr/bin/SPLogbook` |
| niebezpieczny `Exec` | 1 | `desktop Exec must be exactly Exec=SPLogbook` |
| brak ikony | 1 | `AppDir is missing usr/share/icons/hicolor/256x256/apps/splogbook.png` |
| dane użytkownika w obrazie | 1 | `user data or secret material found in the AppDir` |
| symlink uciekający z AppDir | 1 | `AppDir contains symlinks` |
| zły `build_commit` | 1 | `marker build_commit (1111…) differs from the CI commit (35464c7…)` |
| wymaganie GLIBC > 2.43 | 1 | `artifact requires GLIBC_2.43, newer than the GLIBC_2.17 policy` |

`Rejected 12/12 fixtures` — job `success` wyłącznie dlatego, że wszystkie mutacje zostały
odrzucone; w logu nie występuje komunikat sukcesu weryfikatora.

## 7. FUSE

`FUSE_STATUS=VERIFIED`, `FUSE_RUNTIME_VERIFIED=true` — rzeczywisty mount FUSE:
`--version` = `SPLogbook 1.3.0-alpha.1`, `--build-info` = `Commit: 35464c7…`,
`--check-health-startup` z proofem, hash AppImage niezmieniony. Ekstrakcja **nie** jest
przedstawiana jako dowód FUSE.

## 8. Zachowanie F-03 i portable tar.gz

- Portable tar.gz: budowany i weryfikowany w tym samym runie — 4/4 profile `PASS`,
  `NATIVE VERIFICATION COMPLETED SUCCESSFULLY`.
- F-03: job negatywny `success` wyłącznie po odrzuceniu (brak biblioteki → exit 1, bez
  komunikatu sukcesu). `scripts/verify_linux_container.sh` nietknięty.
- `install_linux_runtime_deps.sh`: dodane `desktop-file-utils` i `findutils`, `pacman -Syu`
  (pełna aktualizacja Arch), usunięty BOM; lista sonamów i fail-closed bez zmian.

## 9. Baseline, liczniki i audyt (2026-11-12)

| Komenda | Wynik |
|---|---|
| `cargo +stable fmt --all -- --check` | EXIT 0 |
| `cargo +stable check --workspace --all-targets -j 1` | EXIT 0 |
| `cargo +stable clippy --workspace --all-targets -j 1 -- -D warnings` | EXIT 0, 0 ostrzeżeń |
| `cargo +stable test --workspace --all-targets -j 1` | EXIT 0 — 20 targetów, **2717** zarejestrowanych, **2707** wykonanych, **2707** zaliczonych, **0** nieudanych, **10** ignorowanych |

Uwaga o środowisku hosta: powtórka po zmianach dokumentacyjnych została wykonana z pominięciem
jednego **pre-existing** testu zablokowanego przez prompt bezpieczeństwa Windows
(`windows_helper_test::powershell_real_execution_full_cycle_success`) → **2706/2706**,
0 failed, 10 ignored, 1 filtered. Ten sam test przeszedł w pełnym przebiegu na tym samym kodzie
(2707/2707); pominięcie jest udokumentowanym ograniczeniem hosta, nie zmianą testu.
| `cargo +stable doc --workspace --no-deps` | EXIT 0 |
| `cargo +stable tree -d` | EXIT 0 |
| `git diff --check` | EXIT 0 |
| `cargo audit --deny warnings --no-yanked <7 dozwolonych ignore>` | **EXIT 0** (1458 advisory, 0 znalezisk) |
| Walidacja YAML / JSON | PASS / PASS |

Strażnicy: `security_guard` 123/123 (w tym `m3_g_appimage_integration_guard`,
`m3_f_f03_fail_closed_verification_guard`).

## 10. Dowody CI

| Run | Workflow | head_sha | Wynik |
|---|---|---|---|
| `43166825036` | `CI` | `35464c7…` | **`success`** (fmt, clippy, testy, `cargo audit`) |
| `43166825054` | `Build and Release SPLogbook` | `35464c7…` | **`success`** |

Joby runu `43166825054`: `Build Linux x86_64` = `success`, `Build Windows x64` = `success`,
`Verify Linux (Ubuntu/Fedora/Arch/openSUSE)` = **4 × `success`**,
`Verify Linux (negative fixture)` = `success`, `Verify AppImage (negative fixtures)` = `success`,
`Verify AppImage (real FUSE)` = `success`, `Publish Release Assets` = `skipped`.

## 11. Kryteria zamknięcia (51/51)

| # | Kryterium | Wynik |
|---|---|---|
| 1–4 | origin/main odtworzony; dokumenty odczytane; F-03 CLOSED; M3-G właściwą bramką | ✅ |
| 5–12 | najnowszy Continuous rozwiązany; commit/release/rozmiar zapisane; oficjalny i lokalny SHA-256 zgodne; jeden toolchain | ✅ |
| 13 | runtime przypięty i zweryfikowany | ✅ |
| 14 | portable tar.gz nadal działa | ✅ |
| 15–17 | AppImage zbudowany, właściwa nazwa, poprawny SHA-256 | ✅ |
| 18–21 | struktura AppDir, AppRun (CWD/argumenty/exit code/brak zapisu) | ✅ |
| 22–24 | desktop, ikony, marker | ✅ |
| 25 | build_commit poprawny | ✅ |
| 26–27 | tylko `libhamlib.so.4`; glibc i loader niebundlowane | ✅ |
| 28–29 | brak nierozwiązanych bibliotek; GLIBC ≤ 2.43 | ✅ |
| 30–31 | dane użytkownika poza obrazem; XDG działa | ✅ |
| 32–34 | testy pozytywne; negatywne 12/12; F-03 fail-closed | ✅ |
| 35 | cztery profile `success` | ✅ (4/4) |
| 36 | dowód FUSE zgodny z DESIGN | ✅ (`VERIFIED`) |
| 37–41 | baseline zielony; clippy 0; security_guard; strażnik M3-G; `git diff --check` | ✅ |
| 42–45 | brak release; brak tagu; `v1.2.0` nietknięty; AppImage poza release | ✅ |
| 46–50 | commit dokumentacyjny; `HEAD == main == origin/main`; `0/0`; tracked clean; `scratch/` niecommitowany | ✅ |
| 51 | następna bramka nierozpoczęta | ✅ |

## 12. Ryzyka rezydualne

1. Kontenery OCI współdzielą jądro hosta — dowód jest dowodem userspace/CI, nie dowodem jądra
   dystrybucji.
2. `continuous` appimagetool i runtime są kanałami ruchomymi; bramka przypina je na czas runu.
3. FUSE jest dowodem dodatkowym (DESIGN go nie wymaga), ale został rzeczywiście zweryfikowany.
4. HIL/PTT/RF pozostają poza zakresem (nieuruchamiane).

## 13. Następna bramka

`M3-H` — Integracja Linux DEB (`MASTER_IMPLEMENTATION_PLAN.md` §10). `NEXT_GATE_STATUS=NOT_STARTED`,
`NEXT_GATE_OWNER_GO=REQUIRED`. **Nie rozpoczęto** żadnych prac DEB/RPM/Flatpak.

**Ostateczny status M3-G:** **`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`**.
