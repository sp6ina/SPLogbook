# RAPORT PRZEDIMPLEMENTACYJNY
# STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE

**Projekt:** SPLogbook
**Repozytorium:** `sp6ina/SPLogbook`
**Data projektu:** 2026-11-09
**Data i czas środowiska:** 2026-11-09T19:26:23+01:00 → 19:45+01:00 (Europe/Warsaw, CET +01:00)
**Tryb:** AUTONOMOUS REPOSITORY AGENT — jedna sesja; faza analityczna **bez edycji** przed tym raportem
**Zakres:** F-03 — fail-closed natywna weryfikacja Linux, biblioteki runtime w 4 obrazach OCI,
jawna kontrola `GLIBC_2.43`, testy regresyjne, dowody CI.
**Poza zakresem:** AppImage, M3-G, release, tag, zmiany `src/`, `Cargo.toml`, `Cargo.lock`, schemat SQLite.

---

## 1. Data projektu

**2026-11-09** — zgodna z `AGENTS.md` §1 i datą hosta. Zegar i daty commitów niezmienione.

## 2. Czas i strefa środowiska

Start sesji `2026-11-09T19:26:23+01:00`; pomiary do `19:45+01:00`. Strefa `Europe/Warsaw`
(CET, `+01:00`, bez DST w listopadzie). Windows NT 10.0.26300 x86_64, PowerShell 5.1.26100,
`cargo/rustc 1.99.0`, Git 2.56.0.

## 3. Wynik stabilizacji workspace

Jedna sesja: `Get-Process` (cargo/rustc/git/bash/sh/pwsh/powershell/node/python/DeepSeek) →
wyłącznie własny `powershell`. Brak `.git/index.lock`, brak `target/.cargo-lock`, 0 procesów
Cargo. `target/` mtime `18:54:49+01:00`. Dwa odczyty metadanych 6 plików w odstępie 8 s →
identyczny rozmiar i mtime. **STABILNY.**

## 4–7. HEAD / main / origin-main / ahead-behind

| Ref | SHA |
|---|---|
| HEAD | `e85ee4e0c80d2f3ca96b9e7068f401586662b18b` |
| main | `e85ee4e0c80d2f3ca96b9e7068f401586662b18b` |
| origin/main | `e85ee4e0c80d2f3ca96b9e7068f401586662b18b` |

`main...origin/main` = **0 / 0**; `merge-base --is-ancestor e85ee4e origin/main` → EXIT 0;
fetch (OpenSSL per-command) → EXIT 0.

## 8. Aktywna gałąź

`main` (`## main...origin/main`).

## 9. Tag `v1.2.0`

Obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f` (typ `tag`), peeled
`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — **VERIFIED, nietknięty**; brak `v1.3.0*`.

## 10. Stash

2 istniejące (`clubs_csv_untracked`, `uncommitted pre-stage-10-work`) — **nietknięte**.

## 11. Submoduły

Brak (`.gitmodules` nie istnieje, 0 wpisów trybu `160000`).

## 12. Precyzyjny stan drzewa

`TRACKED_WORKTREE_CLEAN=true`; `UNTRACKED_FILES_PRESENT=true` (`?? scratch/`, 8 plików
nieignorowanych); `IGNORED_FILES_PRESENT=true`; `STRICT_WORKTREE_CLEAN=false`;
`git diff --check` EXIT 0.

## 13. Klasyfikacja `scratch/`

Katalog lokalny, nieśledzony, niecommitowany, **nieusuwany**, niepodstawa oceny kodu; zawiera
lokalne logi i dowody (m.in. `scratch/evidence_r42638736785/*`).

## 14. Przeczytane dokumenty

`AGENTS.md`; `docs/PROJECT_CURRENT_STATE.md`; `docs/NEXT_WORK_GATE.md`;
`MASTER_IMPLEMENTATION_PLAN.md` (§10 M3-F); `docs/DECISION_TRACEABILITY_MATRIX.md`;
`docs/DOCUMENTATION_INDEX.md`; `docs/DEEPSEEK_HARNESS_HANDOFF.md`;
`DESIGN_3_0_B_LINUX_UPDATE.md`; `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`;
`STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_REPORT.md` i `..._PRE_IMPLEMENTATION_REPORT.md`;
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`;
`STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md`;
`STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_INTEGRATION_REPORT.md`;
`STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`;
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`; `scripts/verify_linux_container.sh`;
`scripts/build_linux_portable_tar_gz.sh`; `.github/workflows/{build-and-release,ci}.yml`;
`tests/security_guard.rs`; `tests/linux_multi_distribution_compatibility_test.rs`;
`tests/linux_portable_tar_gz_test.rs`; `tests/linux_archive_security_test.rs`;
`src/main.rs` (kontrakt CLI); `src/core/build_info.rs`; `build.rs`; logi legacy
`scratch/evidence_r42638736785/*`.

## 15. Potwierdzenie następnej bramki

`docs/NEXT_WORK_GATE.md` → `NEXT_GATE: STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`,
`NOT_STARTED`, `OWNER_GO: REQUIRED`. **Zgodne.**

## 16. Potwierdzenie blokady M3-G

`M3_G_STATUS: BLOCKED_BY_F03` w `NEXT_WORK_GATE.md`, `PROJECT_CURRENT_STATE.md`,
`AGENTS.md` §5 pkt 25 i handoff. **Potwierdzone.**

## 17. Aktualna treść `scripts/verify_linux_container.sh` (przed zmianą)

80 linii, `#!/bin/sh`, `set -e`; marker „LEGACY NON-FAIL-CLOSED FIXTURE (finding F-03)” (`:4-15`);
OS/kernel (`:17-27`); `PKG_QUERY_CMD` (`:29-32`); `sha256sum -c` (`:35`); `tar -ztvf | head -30`,
`tar -xzf`, katalog/marker (`:38-42`); `file ... || true` (`:45`), `ldd ... || true` (`:46`);
`--version || true` (`:49`), `--build-info || true` (`:50`); XDG fixture (`:52-64`); rollback
fixture (`:66-79`); `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` (`:80`).

## 18–19. Wszystkie `|| true` w weryfikatorze i klasyfikacja

| Linia | Komenda | Klasa |
|---|---|---|
| `:26` | `getconf GNU_LIBC_VERSION` | `NON_CRITICAL_DIAGNOSTIC` |
| `:27` | `ldd --version` | `NON_CRITICAL_DIAGNOSTIC` |
| `:31` | `eval "$PKG_QUERY_CMD"` | `NON_CRITICAL_DIAGNOSTIC` |
| `:45` | `file SPLogbook/SPLogbook` | **`CRITICAL`** (w nowym skrypcie ELF jest asercją) |
| `:46` | `ldd SPLogbook/SPLogbook` | **`FALSE_PASS_RISK`** |
| `:49` | `SPLogbook --version` | **`FALSE_PASS_RISK`** |
| `:50` | `SPLogbook --build-info` | **`FALSE_PASS_RISK`** |

Poza weryfikatorem (poza zakresem F-03): `build_linux_portable_tar_gz.sh:31,75,77,145-155,225-228,262`;
`build-and-release.yml:95,96,119,127,135`; `ci.yml:28,34` — niezmieniane.

## 20. Krytyczne kontrole

SHA-256, rozpakowanie, root/marker, ELF x86_64 + bit wykonywalny, `ldd` bez `not found`,
`--version` (exit 0 + format), `--build-info` (exit 0 + commit/version/target), GLIBC max ≤ 2.43,
obecność 15 sonamów, komunikat sukcesu jako ostatnia instrukcja.

## 21. Diagnostyczne kontrole niekrytyczne

`getconf GNU_LIBC_VERSION`, `ldd --version`, `PKG_QUERY_CMD`, listing `tar -tzf`, `uname -a/-r/-m`
— pozostają nieblokujące z komentarzem `NON-CRITICAL`.

## 22. Analiza workflow

`build-and-release.yml`: `build-linux` → artefakt + Hamlib; `verify-linux-matrix` (4 profile,
`fail-fast: false`, `docker run ... /bin/sh /workspace/run_test.sh`); `publish-release`
(`if: startsWith(github.ref, 'refs/tags/v') || workflow_dispatch`, `Create GitHub Release`
tylko dla tagu). `ci.yml`: job `quality`. **Anomalia:** runy na `e85ee4e`
(`42673055954` CI — 0 jobów; `42673056245` — brak joba Windows) powstały w czasie awarii
GitHub Actions `major_outage` 18:12–18:43Z (potwierdzone status page), nie z powodu repozytorium.

## 23. Cztery obrazy OCI i digesty

- `ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c`
- `fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c`
- `archlinux:base-20261101.0.620205@sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9`
- `opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1`

## 24. Sposób instalacji bibliotek w każdym obrazie

Nowy `scripts/install_linux_runtime_deps.sh` (POSIX sh, `set -eu`), detekcja `ID` z
`/etc/os-release`, wyłącznie oficjalny menedżer: `apt-get` (Ubuntu), `dnf` (Fedora),
`pacman -Sy` (Arch), `zypper -n install --no-recommends` (Tumbleweed), plus `file` i `binutils`.
Nazwy pakietów **potwierdzone w aktywnych obrazach** tymczasowym runem probe `42675025132`:
Ubuntu wymaga `libusb-1.0-0` (nie `-t64`); Fedora używa `libwayland-client/-cursor/-egl`
(nie `wayland-libs-*`); Arch `wayland`/`mesa`/`systemd-libs`; Tumbleweed `Mesa-libGL1`,
`libusb-1_0-0`, `libudev1`. Po instalacji wszystkie sonamy weryfikowane przez `ldconfig -p`;
brak = FAIL.

## 25. Wymagane biblioteki runtime

`libasound.so.2`, `libX11.so.6`, `libXcursor.so.1`, `libXrandr.so.2`, `libXi.so.6`,
`libxkbcommon.so.0`, `libxkbcommon-x11.so.0`, `libGL.so.1`, `libfontconfig.so.1`,
`libwayland-client.so.0`, `libwayland-cursor.so.0`, `libwayland-egl.so.1`, `libgtk-3.so.0`,
`libusb-1.0.so.0`, `libudev.so.1`. Bezpośrednie `DT_NEEDED` artefaktu: `libasound.so.2`,
`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`; `libhamlib.so.4` z `$ORIGIN/lib` wymaga
`libusb-1.0.so.0`.

## 26. Test `ldd`

`ldd` bez maskowania; przechwycenie outputu i exit code; jawny `case *"not found"*` → FAIL;
wymóg co najmniej jednego `=> /...`; dodatkowo `ldconfig -p` dla pełnej listy sonamów.

## 27. Test `--version`

Realne uruchomienie, exit 0, output dokładnie `SPLogbook 1.3.0-alpha.1`, brak błędu loadera.

## 28. Test `--build-info`

Exit 0, `Version: 1.3.0-alpha.1`, `Commit: <40 hex>` = marker `build_commit` =
`EXPECTED_BUILD_COMMIT` (`${{ github.sha }}`), `Target: linux-x86_64`, brak błędu loadera.

## 29. Test `GLIBC_2.43`

`readelf --version-info`, ekstrakcja `GLIBC_x.y`, maksimum przez `sort -V`, **semantyczne
porównanie** major/minor do `2.43`; `getconf GNU_LIBC_VERSION` ≥ 2.43; przekroczenie → FAIL,
bez podnoszenia polityki.

## 30. Plan testów regresyjnych

Nowy `security_guard::m3_f_f03_fail_closed_verification_guard`: brak `|| true` na
`ldd`/`--version`/`--build-info`; każde pozostałe `|| true` oznaczone `NON-CRITICAL`; sukces
jako ostatnia instrukcja; `not found` → FAIL; GLIBC 2.43; 4 profile; instalacja przed
weryfikatorem; `publish-release` tag-gated; test negatywny w CI (goły `ubuntu:26.04` z realnym
artefaktem → exit 1, brak sukcesu, `not found`). Aktualizacja istniejącego
`m3_f_post_closure_consistency_guard`.

## 31. Plan commitów

1. `ci(linux): make native verification fail closed` — skrypt, installer, workflow, testy.
2. `ci(linux): make native evidence free of diagnostic ambiguity` — jednoznaczność logów.
3. `docs(ci): close F-03 Linux verification gate` — raporty, stan, plan, następna bramka,
   macierz, indeks, handoff, snapshot, aktualizacja asercji statusu.

## 32. Plan CI

Push (bez force) → run Build and Release na dokładnym SHA → pobranie logów/artefaktów
4 profili + joba negatywnego → analiza treści → potwierdzenie `Publish Release Assets = skipped`,
brak tagu/release. Po commicie dokumentacyjnym ponowny pełny baseline lokalny i run CI na
finalnym HEAD.

## 33. Kryteria sukcesu

Wszystkie warunki §24 promptu: fail-closed 4/4, logi jako dowód pozytywny, test negatywny,
GLIBC ≤ 2.43, brak zmian `src/`/`Cargo.*`, brak release/tagu, `HEAD == main == origin/main`,
`TRACKED_WORKTREE_CLEAN=true`, `scratch/` nietknięty.

## 34. Warunki zatrzymania

Wszystkie z §27 promptu, w szczególności: brak dostępu do obrazów/menedżerów pakietów,
GLIBC > 2.43, nieodrzucenie fixture negatywnego, jakikolwiek profil czerwony, brak logów,
konieczność force/reset/czyszczenia, wykrycie sekretu, zmiana `origin/main` w trakcie pracy.

---

**Status:** raport przedstawiony przed pierwszą edycją. Implementacja i dowody:
`STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_REPORT.md`.
