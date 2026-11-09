# NASTĘPNA BRAMKA PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-NEXT-001
**Data:** 2026-11-09
**Zasada:** ten plik wskazuje **dokładnie jedną** następną bramkę. Żadna inna bramka nie może być
w tym miejscu wymieniona jako „następna”.

---

## Status poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** — 2 700 zarejestrowanych / 2 690 wykonanych / 0 failed / 10 HIL; 3 pełne przebiegi (w tym pod obciążeniem) + 20 iteracji bez flaky; `dist/` nie wpływa na baseline; E2E tylko opt-in |
| `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite **3.54.1**), `bundled` zachowany, bez `--ignore`; `cargo audit` **0 podatności**; pełny suite 2 690/2 690 |

Raporty: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`,
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`.

## Następna bramka (jedyna)

# `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`

**Typ:** dokumentacja + polityka + wyrównanie testów/skryptów/CI.
**Priorytet:** P1 (blokuje domknięcie M3-F wobec intencji OWNER i odblokowanie M3-G).
**Status:** `NOT_STARTED`.
**Prekondycja:** zamknięta bramka QA i bramka zależności (spełnione 2026-11-09).

### Dlaczego ta bramka, a nie M3-G

M3-G (AppImage) konsumuje kontrakt wsparcia Linux: minimum kernela, minimum glibc, listę
dystrybucji, layout artefaktu i tryb aktualizacji. Ten kontrakt jest obecnie niezgodny z wiążącą
intencją OWNER, a trzy artefakty **egzekwują** starą wartość:

- `scripts/build_linux_portable_tar_gz.sh` (`RESOLVED_MIN_KERNEL`),
- `tests/linux_multi_distribution_compatibility_test.rs`,
- `tests/security_guard.rs` (`m3_f_owner_decisions_and_support_policy_guard`).

Otwarcie M3-G przed wyrównaniem pomnoży korektę.

### Pytanie OWNER (blokujące pierwszy krok)

Snapshot rejestruje jądra Fedora 45 / Arch / Tumbleweed jako **6.18** (poniżej wymaganego 7.2),
Ubuntu 26.04 LTS jako **7.0**, a Ubuntu 26.10 jako 7.3. OWNER musi rozstrzygnąć:
(a) dane snapshotu są nieaktualne i należy je ponownie ustalić z oficjalnych źródeł,
(b) minimum 7.2 wymaga rewizji,
(c) lista profili zawęża się do Ubuntu 26.10.

### Zakres (skrót — pełna lista w raporcie przejęcia §19.2)

1. Zarejestrować decyzje OWNER z 2026-11-09 ze stabilnymi ID:
   `OWNER-M3-F-LINUX-BASELINE-02`, `OWNER-M3-F-KERNEL-POLICY-02` (fixed **7.2**),
   `OWNER-M3-F-GLIBC-POLICY-01` (minimum **2.43**, bez bundled glibc, bez rebuild do 2.41).
2. Przepisać `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}` do schematu v2
   (`OWNER_APPROVED_FIXED_MINIMUM`, `resolved_minimum_kernel: "7.2"`, `minimum_glibc: "2.43"`,
   `native_kernel_evidence: "NOT_VERIFIED_IN_CONTAINER"`).
3. Usunąć Debian 13.7 i openSUSE Leap z macierzy obowiązkowej; zaktualizować strażnika.
4. Zaktualizować skrypt budowania (`RESOLVED_MIN_KERNEL="7.2"`) i walidację markera.
5. Zaktualizować macierz CI (bez Debian/Leap; Ubuntu 26.04 LTS jako warunkowe).
6. Usunąć `|| true` z `scripts/verify_linux_container.sh` (F-03); oznaczyć fixture jako syntetyczny.
7. Zaktualizować `README.md`, `MASTER_IMPLEMENTATION_PLAN.md`, rejestry i raporty M3-F.
8. Raport końcowy z pełnym baseline i nowymi licznikami.

### Poza zakresem

- implementacja M3-G/DEB/RPM/Flatpak,
- wersjonowanie MSI prerelease,
- fizyczny HIL/PTT/RF,
- zmiany runtime produktu (poza stałą skryptu budowania),
- tagi/release/publikacja.

### Kryteria ukończenia (skrót)

`fmt`/`check`/`clippy -D warnings`/pełny test/`cargo audit` zielone; **brak** twierdzeń
o Debian 13.7 / openSUSE Leap i o dynamicznym floor 6.12 (dowód `grep`); snapshot zawiera
`OWNER_APPROVED_FIXED_MINIMUM` i `7.2`; raport końcowy opublikowany; M3-F otrzymuje status
`COMPLETED_WITH_ABI_ONLY_NATIVE_EVIDENCE` (lub wording OWNER).

## M3-G

**`BLOCKED`** — nie rozpoczynać do zamknięcia bramki polityki Linux.
