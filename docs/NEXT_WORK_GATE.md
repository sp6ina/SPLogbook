# NASTĘPNA BRAMKA PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-NEXT-001
**Data:** 2026-11-09
**Zasada:** ten plik wskazuje **dokładnie jedną** następną bramkę. Żadna inna bramka nie może być
w tym miejscu wymieniona jako „następna”.

---

## Status poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** — 2 700 zarejestrowanych / 2 690 wykonanych / 0 failed / 10 HIL; 3 pełne przebiegi + 20 iteracji bez flaky; `dist/` nie wpływa na baseline; E2E tylko opt-in |
| `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite 3.54.1), `bundled` zachowany, bez `--ignore`; `cargo audit` 0 podatności |
| `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — minimum **Linux 7.0 / glibc 2.43**, 4 wspierane dystrybucje, Debian 13.7 i Leap 16.1 jawnie niewspierane, macierz CI wyrównana, strażnicy zieloni |

Raporty: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`,
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`,
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`.

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

## Następna bramka (jedyna)

# `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`

**Typ:** implementacja pakietu AppImage + weryfikacja natywna.
**Priorytet:** P1.
**Status:** `NOT_STARTED` — **nie rozpoczęto**; wymaga osobnego promptu/GO od OWNER.
**Prekondycja:** zamknięta bramka QA, bramka zależności i bramka wyrównania polityki Linux
(spełnione 2026-11-09).

Kontrakt wejściowy dla M3-G: minimum **Linux 7.0 / glibc 2.43**, cztery wspierane dystrybucje
z sekcji powyżej, jeden kanoniczny artefakt, layout `SPLogbook/`, aktualizacje
`JOURNALED_TRANSACTIONAL_REPLACEMENT`, XDG Base Directory.

**Uwaga:** przed publikacją M3-G należy domknąć osobne zadanie CI-hardening: fixture
`scripts/verify_linux_container.sh` nie jest fail-closed (F-03) i nie stanowi dowodu natywnego.

## M3-H/J

**`NOT_STARTED`** — DEB/RPM/Flatpak po M3-G.
