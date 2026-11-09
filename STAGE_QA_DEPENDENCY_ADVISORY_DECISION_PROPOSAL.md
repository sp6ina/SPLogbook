# PROPOZYCJA DECYZJI OWNER — BLOKER RUSTSEC-2026-0459

**Dokument ID:** DSH-SPLOGBOOK-DEP-001
**Data:** 2026-11-09
**Status:** `OWNER_DECISION_REQUIRED`
**Powiązane:** `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md` §24A,
`docs/NEXT_WORK_GATE.md`

---

## 1. Fakt

`cargo audit` (świeża baza RustSec, 1 429 advisory) na niezmienionym `Cargo.lock` zwraca:

| Pole | Wartość |
|---|---|
| ID | `RUSTSEC-2026-0459` (CVE-2026-119375, GHSA-72w2-rjgg-q729) |
| Pakiet | `libsqlite3-sys 0.38.2` ← `rusqlite 0.40.2` (`bundled`) |
| Dodana do RustSec | **2026-11-09T09:49:34Z** (commit `c5ed556`) |
| Data advisory | 2026-10-30 |
| Typ / CVSS | memory-exposure / 6.8 medium, wektor lokalny (`AV:L/AC:L/AT:N/PR:N/UI:P`) |
| Warunek | otwarcie bazy z sąsiednim `-wal` o innej stronie, zapisanym przez mniej zaufaną stronę |
| Patch | `libsqlite3-sys >= 0.39.0` (SQLite 3.54.0) → `rusqlite 0.41` |

Podatność **nie została wprowadzona przez bramkę QA** — `Cargo.toml` i `Cargo.lock` są nietknięte.
Jest to nowe ujawnienie w istniejącej zależności.

## 2. Dlaczego to blokuje

1. Kryterium ukończenia QA nr 24: „0 nowych podatności” — jest 1 nowa.
2. Naprawa wymaga zmiany `Cargo.toml`/`Cargo.lock` — zakazanej w bramce QA.
3. `--ignore` bez decyzji OWNER = osłabienie asercji bezpieczeństwa.
4. CI uruchamia `cargo audit --deny warnings` → każdy push dzisiaj będzie czerwony na tym kroku.

## 3. Ograniczenie wykonawcze tej sesji

Aktualizacja zależności **nie jest wykonalna w tym sandboxie**:

```
cargo info rusqlite@0.41.0
→ SSL connect error (schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS)
```

Cargo używa schannel i nie ma per-command backendu OpenSSL (inaczej niż `git`).
W lokalnym cache **nie ma** `rusqlite 0.41` ani `libsqlite3-sys 0.39`.
Aktualizację może wykonać: CI GitHub Actions, inna maszyna z dostępem do crates.io,
albo sesja z działającym TLS dla cargo.

## 4. Warianty decyzji

### A. Bramka aktualizacji zależności w środowisku z siecią (rekomendowane)

- `cargo update -p rusqlite --precise 0.41.x` (lub ręcznie `rusqlite = "0.41"` + build),
- `Cargo.lock` regenerowany wyłącznie przez cargo,
- pełny baseline (fmt/check/clippy/test/doc/security_guard/audit),
- testy SQLite/WAL/SHM i danych użytkownika,
- raport, ff `main`, push.
- **Efekt:** podatność usunięta; wymaga środowiska z siecią.

### B. Formalna akceptacja ryzyka (decyzja OWNER, natychmiastowa)

- OWNER akceptuje ryzyko szczątkowe (atak lokalny, wymaga wcześniejszego zapisu `-wal`
  przez mniej zaufaną stronę; aplikacja nie przetwarza baz z niezaufanych źródeł bez otwarcia),
- jawny `--ignore RUSTSEC-2026-0459` **wyłącznie** w dedykowanym kroku bezpieczeństwa,
  z komentarzem i wpisem w `docs/AUDIT_FINDINGS_REGISTER.md`,
- ponowny `cargo audit` z ignorowanym ID → EXIT 0,
- bramka QA może otrzymać `COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`,
  ff + push.
- **Efekt:** podatność pozostaje; ryzyko zaakceptowane i widoczne w repozytorium.

### C. Zmiana modelu linkowania SQLite (długoterminowa)

- rezygnacja z `bundled` na system SQLite (`libsqlite3-sys` bez bundled) — większa zmiana
  platformowa, osobny projekt; nie rozwiązuje problemu na systemach ze starym SQLite.

## 5. Rekomendacja

**A**, a jeżeli środowisko nie pozwala na aktualizację przed zamknięciem QA — **B** z jawną,
udokumentowaną akceptacją ryzyka (nigdy ciche `--ignore`).

Do decyzji: commity QA pozostają lokalne (`repair/full-system-remediation`, 4 commity przed `main`),
`main == origin/main == f73a703`, `v1.2.0` nietknięty, working tree `CLEAN`.
