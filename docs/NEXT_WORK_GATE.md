# NASTĘPNA BRAMKA PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-NEXT-001
**Data:** 2026-11-09
**Zasada:** ten plik wskazuje **dokładnie jedną** następną bramkę. Żadna inna bramka nie może być
w tym miejscu wymieniona jako „następna”.

---

## Status poprzedniej bramki

`STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` → **`BLOCKED_BY_NEW_DEPENDENCY_ADVISORY`**

Część testowa jest zielona (2 700 zarejestrowanych / 2 690 wykonanych / **0 nieudanych** /
10 ignorowanych HIL; 3 pełne przebiegi, w tym pod obciążeniem; 20 iteracji bez flaky), ale
`cargo audit` (świeża baza) wykrył **1 nową podatność**:

- **RUSTSEC-2026-0459** / CVE-2026-119375 — `libsqlite3-sys 0.38.2` (przez `rusqlite 0.40.2`, `bundled`),
- advisory dodana do RustSec **2026-11-09T09:49:34Z**, 13 minut przed startem sesji,
- patch: `libsqlite3-sys >= 0.39.0` (SQLite 3.54.0) → `rusqlite 0.41`,
- naprawa wymaga zmiany `Cargo.toml`/`Cargo.lock` — **zakazanej w bramce QA**,
- ignorowanie = osłabienie asercji bezpieczeństwa — **zakazane**.

Commity QA pozostają **lokalne** na `repair/full-system-remediation`; `main` = `origin/main` =
`f73a703` (bez ff, bez push). Szczegóły: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md` §24A.

## Następna bramka (jedyna)

# `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE`

**Typ:** bezpieczeństwo zależności + aktualizacja lockfile + pełny re-test.
**Priorytet:** P0 (blokuje push, zamknięcie QA i wszystko dalej).
**Status:** `NOT_STARTED`.
**Prekondycja:** decyzja OWNER wybierająca wariant A, B albo C (poniżej).

### Decyzja OWNER (blokująca)

| Wariant | Opis | Skutek |
|---|---|---|
| **A (rekomendowany)** | Podnieść `rusqlite` do `0.41` / `libsqlite3-sys` do `0.39` (SQLite 3.54.x); MSRV 1.89 ≤ 1.99; zaktualizować `Cargo.lock`; pełny baseline; zamknąć advisory | Usuwa podatność; zmiana zależności w dedykowanej bramce |
| B | Formalna akceptacja ryzyka (atak lokalny, wymaga wcześniejszego zapisu `-wal`) z jawnym `--ignore` w dedykowanej bramce bezpieczeństwa | Podatność pozostaje; wymaga udokumentowanej decyzji OWNER i wpisu do rejestru |
| C | Rezygnacja z `bundled` na system SQLite | Większa zmiana platformowa; osobny projekt |

### Zakres (wariant A)

1. `Cargo.toml`: `rusqlite = { version = "0.41", features = ["bundled","backup"] }`.
2. `Cargo.lock`: regeneracja wyłącznie przez `cargo update -p rusqlite` / build.
3. Pełny baseline: fmt / check / clippy `-D warnings` / test / doc / security_guard / audit.
4. Testy migracji SQLite i WAL/SHM (dane użytkownika) — bez zmian zachowania.
5. Raport bramki + aktualizacja `docs/PROJECT_CURRENT_STATE.md` i rejestru znalezisk.
6. Po zielonym `cargo audit`: ff `main` z gałęzi QA **i** tej bramki, push, potwierdzenie `0/0`.

### Czego nie wolno w tej bramce

- zmieniać logiki produktu poza niezbędną adaptacją API `rusqlite`,
- dodawać `--ignore` bez decyzji OWNER,
- publikować release ani tworzyć tagu,
- wykonywać HIL/PTT/RF,
- otwierać produkcyjnych baz.

## Następna bramka merytoryczna (po odblokowaniu)

# `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`

Bez zmian zakresu: rejestracja decyzji OWNER (kernel 7.2 fixed, glibc 2.43, bez Debian 13.7 i Leap,
Ubuntu 26.04 LTS warunkowe), przepisanie snapshotu do schematu v2, aktualizacja testów, skryptu
budowania i CI, usunięcie `|| true` z weryfikacji natywnej, raport końcowy. Szczegóły w raporcie
przejęcia §19.2. **Wymaga najpierw decyzji OWNER o jądrach 6.18 vs minimum 7.2.**

## M3-G

**`BLOCKED`** — nie rozpoczynać.
