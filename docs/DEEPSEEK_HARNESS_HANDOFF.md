# DEEPSEEK HARNESS — RAPORT PRZEJĘCIA PROJEKTU (OPERACYJNY)

**Dokument ID:** DSH-SPLOGBOOK-HANDOFF-001
**Data:** 2026-11-09
**Źródło migracji:** Antigravity → DeepSeek Harness
**Pełny raport dowodowy:** `DEEPSEEK_HARNESS_PROJECT_INGESTION_AND_HANDOFF_REPORT.md` (DSH-SPLOGBOOK-INGEST-001)
**Dokumenty operacyjne:** `AGENTS.md`, `docs/PROJECT_CURRENT_STATE.md`, `docs/NEXT_WORK_GATE.md`

---

## 1. Nagłówek środowiska (tej sesji)

```
PROJECT DATE:      2026-11-09
ENVIRONMENT DATE:  2026-11-09
ENVIRONMENT TIME:  11:02:17 +0100 (pierwszy odczyt)
TIMEZONE:          Europe/Warsaw (Central European Standard Time, CET)
UTC OFFSET:        +01:00
HARNESS:           DeepSeek Harness
MODEL:             DeepSeek-V4.1-Flash-1017 (deklaracja zadania; harness: deepseek-flash)
OS:                Windows NT 10.0.26300.0 (x86_64)
SHELL:             PowerShell 5.1.26100.9818
WORKSPACE:         C:\Users\sp6in\Documents\SPLogbook\SPLogbook
FILE POLICY:       workspace-write (approval prompts enabled)
```

Zasada daty: data projektu **nie została cofnięta**; host wskazywał tę samą datę (`DATE CONSISTENCY: VERIFIED`).

## 2. Bramka Git (wynik)

| Kontrola | Wartość |
|---|---|
| `HEAD` / `main` / `origin/main` (baza) | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `repair/full-system-remediation` (baza) | `f73a703c40f4462b48bed241aafc47fe15486bd0` |
| `origin/main...main` | `0 0` |
| Aktywna operacja Git | brak |
| Working tree (przed bramką QA) | czysty + 1 nieśledzony raport przejęcia |
| Tag `v1.2.0` | obiekt `3ea8d51a…`, peeled `6f7d3aaa…` — **VERIFIED** |

### 2.1 Obejście TLS w sandboxie (obowiązkowe dla przyszłych sesji)

`git fetch` z domyślnym backendem **schannel** kończy się w sandboxie DSH błędem:

```
schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS (0x8009030e)
```

Rozwiązanie (zweryfikowane, bez zmiany konfiguracji globalnej):

```
git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false fetch --prune origin
```

`core.fsmonitor=false` i `core.untrackedCache=false` tłumią dodatkowo fantomowy `?? scratch/`
i błędy `fsmonitor_ipc__send_query`.

## 3. Zakres ingestii (faza 1, zakończona)

| Miara | Wartość |
|---|---|
| Pliki śledzone | 393 |
| Pliki Rust | 292 (155 097 linii) |
| Pliki testów integracyjnych | 15 |
| Dokumenty Markdown | 60 (15 160 linii) |
| Commity przejrzane | 300 |
| Znaleziska | 25 (F-01…F-25) |
| Commity utworzone w fazie analitycznej | 0 |

Wszystkie pliki źródłowe, testy i dokumenty zostały przeczytane w całości w fazie ingestii;
dowody per-moduł zapisano w raporcie DSH-SPLOGBOOK-INGEST-001 (§4, §12, §16).

## 4. Co zweryfikowano pierwszoosobowo w tej sesji

- Git gate + tag `v1.2.0` (lokalnie i zdalnie) — bez rozbieżności.
- Toolchain: `cargo/rustc 1.99.0`, `edition 2024`, wersja `1.3.0-alpha.1`.
- Nieudany test `inno_native_install_and_uninstall_lifecycle_e2e` — **odtworzony 3×** (RUN 1-3), zawsze `exit 101`.
- Przyczyna: **sandbox DSH (Low Integrity Level)** blokuje zapis do `HKCU` i Menu Start
  (`0x80070005`), mimo `FullControl` w ACL; warstwa plikowa instalatora działa poprawnie
  (wszystkie pliki allowlisty zainstalowane, rollback czysty). **To nie regresja M3-D.**
- Pełny log Inno Setup zachowany w raporcie QA (istotne fragmenty) i lokalnie w
  `scratch/qa_gate/native_e2e_diag4/logs/setup_diag.log` (plik ignorowany przez Git).
- Trzy testy czasowe (F-23) — poprawione wyłącznie w modułach `#[cfg(test)]`.

## 5. Baseline (obowiązkowy, `-j 2`)

```
cargo +stable metadata --no-deps --format-version 1
cargo +stable fmt --all -- --check
cargo +stable check --workspace --all-targets --all-features -j 2
cargo +stable clippy --workspace --all-targets --all-features -j 2 -- -D warnings
cargo +stable test --workspace --all-targets -j 2
cargo +stable test --doc -j 2
cargo +stable test --test security_guard -j 2
cargo +stable test --test i18n_guard -j 2
cargo +stable test --test theme_guard -j 2
cargo audit --db <zapisywalny katalog tymczasowy>
cargo tree -d
git diff --check
git -c core.fsmonitor=false -c core.untrackedCache=false status --porcelain=v1
```

**Zasady baseline:**
- standardowy baseline **nie wykonuje** lokalnego instalatora (brak opt-in),
- `dist/*.exe` i `dist/*.msi` nie zmieniają wyniku,
- `#[ignore]` HIL pozostaje dokładnie 10,
- HIL/PTT/RF = 0.

## 6. Kluczowe znaleziska (skrót)

| ID | Severity | Treść | Status |
|---|---|---|---|
| F-01 | P1 | Repozytorium koduje zastąpioną politykę Linux (6.12, Debian, Leap) | do bramki polityki Linux |
| F-02/F-03/F-04 | P1 | „Natywna” weryfikacja w kontenerze nie dowodzi jądra; `\|\| true` maskuje błędy; sprzeczne twierdzenia ABI | do bramki polityki Linux |
| F-05 | P2 | MSI/Setup nie są budowane/publikowane przez CI | przyszła bramka release |
| F-07 | P2 | XDG vs portable DB — precedence | przyszła bramka |
| F-14 | P0 (sprzętowo zablokowane) | Panika po załączeniu PTT przy `max_test_duration_ms` 1-9 | obowiązkowe przed HIL |
| F-15…F-21 | P1/P2 | HIL self-cert, GUI panic, PTT F7, SOTA/POTA, LoTW scope, Cabrillo cap | bramka danych/RF |
| F-23 | P2 | 3 testy zależne od wall-clock | **NAPRAWIONE** w tej bramce |
| F-24 | P1 | Niehermetyczny E2E instalatora | **NAPRAWIONE** w tej bramce |
| F-25 | P3 | Phantom `?? scratch/` przez fsmonitor/untracked cache | obejście: `-c` per-command |

## 7. Czego NIE powtarzać

1. Nie uruchamiaj lokalnego instalatora bez opt-in i jawnej ścieżki artefaktu.
2. Nie oceniaj instalatora po nieudanym teście — najpierw log i środowisko (Low IL!).
3. Nie używaj `Start-Process` w sandboxie DSH do uruchamiania instalatora (ShellExecute
   zawiesza proces); użyj `CreateProcess` (`UseShellExecute=$false`) jak w testach.
4. Nie traktuj `uname -r` w kontenerze jako dowodu jądra dystrybucji.
5. Nie „naprawiaj” snapshotu Linux przy okazji — to osobna bramka.
6. Nie twórz tagów ani release bez bramki; nie ruszaj `v1.2.0`.
7. Nie otwieraj produkcyjnej bazy/konfiguracji; nie używaj produkcyjnego `HOME`/`APPDATA`.
8. Nie zmieniaj `src/` produkcyjnie bez dowodu regresji i osobnej bramki.
9. Nie używaj rebase/zwykłego merge/force-push.
10. Nie uznawaj testu modelowego za natywny E2E.

## 8. Procedura restartu sesji

1. `AGENTS.md` → `docs/PROJECT_CURRENT_STATE.md` → `docs/NEXT_WORK_GATE.md`.
2. Fetch z OpenSSL per-command; sprawdź `HEAD == main == origin/main` i tag `v1.2.0`.
3. Uruchom baseline z §5 i porównaj liczniki z `docs/PROJECT_CURRENT_STATE.md`.
4. Pracuj tylko nad bramką z `docs/NEXT_WORK_GATE.md`.

## 9. Stan bramki QA i następny krok

- Bramka QA: **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** — 2 690/2 690, 0 failed,
  10 HIL, 3 pełne przebiegi (w tym pod obciążeniem) + 20 iteracji bez flaky.
- Bramka zależności: **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1`,
  `libsqlite3-sys 0.39.1` (SQLite **3.54.1**), `bundled` zachowany, bez `--ignore`;
  `cargo audit` **0 podatności**.
- Następna bramka: **`STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`** — `NOT STARTED`
  (wymaga decyzji OWNER o jądrach 6.18 vs 7.2).
- M3-G (AppImage): **`BLOCKED`**.

Raporty: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`,
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`.

## 10. Linki

- `AGENTS.md` — zasady operacyjne
- `docs/PROJECT_CURRENT_STATE.md` — stan prawdy
- `docs/PROJECT_ARCHITECTURE_MAP.md` — mapa modułów
- `docs/DECISION_TRACEABILITY_MATRIX.md` — decyzje → kod → test
- `docs/DOCUMENTATION_INDEX.md` — indeks dokumentów
- `docs/NEXT_WORK_GATE.md` — jedna następna bramka
- `docs/DEEPSEEK_HARNESS_SESSION_BOOTSTRAP.md` — start sesji
- `docs/DEEPSEEK_HARNESS_REQUIRED_CAPABILITIES.md` — wymagane zdolności
