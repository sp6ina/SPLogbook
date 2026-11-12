# NASTĘPNA BRAMKA PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-NEXT-001
**Data:** 2026-11-12
**Zasada:** ten plik wskazuje **dokładnie jedną** następną bramkę. Żadna inna bramka nie może być
w tym miejscu wymieniona jako „następna”.

---

## Status poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** — baseline deterministyczny i hermetyczny, E2E tylko opt-in |
| `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite 3.54.1), `bundled` zachowany, bez `--ignore` |
| `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — minimum **Linux 7.0 / glibc 2.43**, 4 wspierane dystrybucje |
| `STAGE-M3-F-POST-CLOSURE-CONSISTENCY-VERIFICATION-GATE` | **`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED`** — klasyfikacja drzewa, potwierdzony fałszywy PASS na 4 profilach, formalne zdefiniowanie F-03 |
| `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` | **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`** — fail-closed weryfikator, biblioteki runtime w 4 obrazach OCI, jawna kontrola `GLIBC_2.43`, test negatywny; run `42685105375` na commicie `abdb767`, 4/4 profile `success` |
| `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE` | **`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`** — AppImage: 4/4 profile `success`, 12/12 mutacji odrzuconych, FUSE `VERIFIED`; run `43166825054` + `CI` `43166825036` = `success` |
| `STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE` | **`COMPLETED_DEB_INTEGRATION_VERIFIED`** — binarny DEB: deterministyczna budowa `dpkg-deb`, `/opt/SPLogbook` + launcher, lintian 0 errors, 27/27 mutacji odrzuconych, pełny cykl życia bez utraty danych; run `43245181565` + `CI` `43245181469` = `success`; `Publish Release Assets` = `skipped` |

Raporty: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`,
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`,
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`,
`STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_REPORT.md`,
`STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_REPORT.md`,
`STAGE_M3_G_LINUX_APPIMAGE_INTEGRATION_REPORT.md`,
`STAGE_M3_H_LINUX_DEB_INTEGRATION_REPORT.md`.

## Polityka Linux (obowiązująca, zapisana w repozytorium)

| Element | Wartość |
|---|---|
| Decyzja o kernelu | `OWNER-M3-F-KERNEL-POLICY-OPTION-A` (dynamiczny floor) |
| Minimum kernela | **Linux 7.0** (Ubuntu 26.04.1 LTS) |
| Minimum glibc | **2.43** (`GLIBC_2.43`) |
| Wspierane dystrybucje | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| Niewspierane (potwierdzone) | Debian 13.7 (glibc 2.41), openSUSE Leap 16.1 (glibc 2.40), Ubuntu interim 26.10 |
| Kontrakt DEB | Ubuntu 26.04 LTS (glibc 2.43); układ `/opt/SPLogbook` (`OWNER-M3-H-03`) |
| Snapshot | `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}` |

Zasada rewizji: minimum jest przeliczane przy każdym wydaniu i przy zmianie domyślnego kernela
wspieranej dystrybucji; nie wolno go podnosić bez dowodu technicznego. Kanały testing/Rawhide/
backports/RC nie wyznaczają minimum.

## F-03 — ZAMKNIĘTE (dowód fail-closed)

`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` = `COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`.

- **Run:** `42685105375` (Build and Release SPLogbook), `head_sha = abdb767d41bd07da1304c1c8f5d8993726a6575a`,
  `conclusion = success`.
- **4/4 profile `success`:** Ubuntu 26.04.1 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed.
- **Dowód pozytywny per profil:** instalacja bibliotek runtime zakończona sukcesem, `ldd` bez
  `not found` (zero wystąpień w logu), `--version` = `SPLogbook 1.3.0-alpha.1` (exit 0),
  `--build-info` z `Commit: abdb767…` (exit 0), maksymalny wymagany symbol `GLIBC_2.43`
  ≤ polityka, komunikat `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` na końcu.
- **Dowód negatywny:** job `Verify Linux (negative fixture, fail-closed proof)` — goły obraz
  `ubuntu:26.04` bez bibliotek runtime → `FAIL: ldd reports an unresolved shared library`,
  `libasound.so.2 => not found`, **exit code 1**, brak komunikatu sukcesu.
- **Publikacja:** `Publish Release Assets` = `skipped`; brak nowego tagu i release.
- F-03 pozostaje `CLOSED` i nie może zostać otwarte ponownie bez odtwarzalnej regresji.

## M3-H — ZAMKNIĘTE (DEB)

`STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE` = `COMPLETED_DEB_INTEGRATION_VERIFIED` (2026-11-12).

- **Run:** `43245181565` (Build and Release SPLogbook), `head_sha = 3171aa8d03444901c55be7aa1f134e297b061604`,
  `conclusion = success`; `CI` `43245181469` = `success`.
- **Build DEB** = `success`; **Verify DEB** = `success`; **Verify DEB (negative fixtures)** =
  `success` (27/27); `Build Windows x64`, `Build Linux x86_64`, 4× `Verify Linux`,
  `Verify Linux (negative)`, 2× `Verify AppImage` = `success`; `Publish Release Assets` = `skipped`.
- **Artefakt:** `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` (+ alias `SPLogbook-Linux-amd64.deb`),
  **11 883 596 B**, SHA-256 `85a4ca9877305d97f7c31929eb6034bd36e88dc8d7fd208f9fab9cf7d8890061`,
  `Version: 1.3.0~alpha.1-1`, `Architecture: amd64`, `Installed-Size: 45 879`,
  `build_commit 3171aa8…`, maksymalny `GLIBC_2.43`, 12 plików regularnych,
  `dpkg/dpkg-deb 1.23.7`, lintian **0 errors** / 2 warnings.
- **Cykl życia:** fresh install, upgrade z `1.3.0~alpha.0-1`, downgrade, re-upgrade, reinstall,
  remove, purge — PASS; dane użytkownika (XDG, bazy QSO, WAL/SHM, konfiguracja, ADIF/Cabrillo,
  backupy) niezmienione (SHA-256 identyczne).
- **Publikacja:** `Publish Release Assets` = `skipped`; brak nowego tagu i release.

## Następna bramka (jedyna wykonywalna)

NEXT_GATE: **`M3-I`** — Integracja Linux RPM (`MASTER_IMPLEMENTATION_PLAN.md` §10)
NEXT_GATE_STATUS: `NOT_STARTED`
NEXT_GATE_OWNER_GO: `REQUIRED`
F03_STATUS: `CLOSED`
M3_G_STATUS: `CLOSED`
M3_H_STATUS: `CLOSED`

Kolejna bramka `M3-I` (RPM) **nie została rozpoczęta** i wymaga osobnego GO OWNER.
`M3-J` (Flatpak) pozostaje `NOT_STARTED` po zamknięciu M3-H.
