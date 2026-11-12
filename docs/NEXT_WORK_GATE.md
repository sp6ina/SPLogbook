# NASTĘPNA BRAMKA PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-NEXT-001
**Data:** 2026-11-09
**Zasada:** ten plik wskazuje **dokładnie jedną** następną bramkę. Żadna inna bramka nie może być
w tym miejscu wymieniona jako „następna”.

---

## Status poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** — baseline deterministyczny i hermetyczny, E2E tylko opt-in |
| `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite 3.54.1), `bundled` zachowany, bez `--ignore` |
| `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — minimum **Linux 7.0 / glibc 2.43**, 4 wspierane dystrybucje |
| `STAGE-M3-F-POST-CLOSURE-CONSISTENCY-VERIFICATION-GATE` | **`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED`** — klasyfikacja drzewa, sprostowanie 7 profili, potwierdzony fałszywy PASS na 4 profilach, formalne zdefiniowanie F-03 |
| `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` | **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`** — fail-closed weryfikator, biblioteki runtime w 4 obrazach OCI, jawna kontrola `GLIBC_2.43`, test negatywny; run `42685105375` na commicie `abdb767`, 4/4 profile `success` |

Raporty: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`,
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`,
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`,
`STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_REPORT.md`,
`STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_REPORT.md`.

## Polityka Linux (obowiązująca, zapisana w repozytorium)

| Element | Wartość |
|---|---|
| Decyzja o kernelu | `OWNER-M3-F-KERNEL-POLICY-OPTION-A` (dynamiczny floor) |
| Minimum kernela | **Linux 7.0** (Ubuntu 26.04.1 LTS) |
| Minimum glibc | **2.43** (`GLIBC_2.43`) |
| Wspierane dystrybucje | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| Niewspierane (potwierdzone) | Debian 13.7 (glibc 2.41), openSUSE Leap 16.1 (glibc 2.40), Ubuntu interim 26.10 |
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
- **Implementacja:** `scripts/verify_linux_container.sh` (fail-closed), `scripts/install_linux_runtime_deps.sh`
  (instalacja per dystrybucja), `.github/workflows/build-and-release.yml` (instalacja przed
  weryfikacją, job negatywny), strażnik `m3_f_f03_fail_closed_verification_guard`.

## Następna bramka (jedyna wykonywalna)

NEXT_GATE: **`STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`**
NEXT_GATE_STATUS: `IN_PROGRESS`
NEXT_GATE_OWNER_GO: `GRANTED` (`OWNER-M3-G-01`, `OWNER-M3-G-05`)
F03_STATUS: `CLOSED`
M3_G_STATUS: `IN_PROGRESS`
M3_G_BLOCKER: `NONE` — `RUSTSEC-2026-0486` usunięte decyzją `OWNER-DEP-RUSTIX-01` (`rustix 1.1.6`)

# `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`

**Typ:** implementacja pakietu AppImage + weryfikacja natywna.
**Priorytet:** P1.
**Status:** `IN_PROGRESS` — bramka wznowiona po usunięciu blokera zależności
(`RUSTSEC-2026-0486`, decyzja `OWNER-DEP-RUSTIX-01`, bramka
`STAGE-DEPENDENCY-RUSTIX-RUSTSEC-2026-0486-REMEDIATION-GATE`, `rustix 1.1.6`).
Zamknięcie wymaga pełnych, zielonych dowodów CI: 4/4 profile, job negatywny i FUSE.
**Prekondycja:** zamknięta bramka QA, bramka zależności, bramka wyrównania polityki Linux
**oraz** bramka F-03 — wszystkie spełnione.

Kontrakt wejściowy dla M3-G: minimum **Linux 7.0 / glibc 2.43**, cztery wspierane dystrybucje
z sekcji powyżej, jeden kanoniczny artefakt, layout `SPLogbook/`, aktualizacje
`JOURNALED_TRANSACTIONAL_REPLACEMENT`, XDG Base Directory, fail-closed weryfikator natywny
z runu `42685105375`.

**Czego nie wolno:** publikować release, tworzyć tagu, zmieniać polityki 7.0/2.43, zmieniać
`publish-release`, dodawać AppImage do publicznego wydania, zmieniać `Cargo.toml`/`Cargo.lock`
bez osobnej decyzji OWNER.

## M3-H/J

**`NOT_STARTED`** — DEB/RPM/Flatpak po M3-G.
