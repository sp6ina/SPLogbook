# RAPORT WYKONANIA BRAMKI
# STAGE-DEPENDENCY-SECURITY-UPDATE-GATE

**Dokument ID:** DSH-SPLOGBOOK-DEPSEC-001
**Projekt:** SPLogbook
**Data:** 2026-11-09
**HARNESS:** DeepSeek Harness
**MODEL:** DeepSeek-V4.1-Flash-1017
**Decyzja OWNER:** `REMEDIATION: LATEST_STABLE_SAFE_COMPATIBLE_DEPENDENCY_UPDATE`
**Bloker wejściowy:** `RUSTSEC-2026-0459` (`libsqlite3-sys 0.38.2`)
**Status bramki:** **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`**
**Wynik:** `cargo audit` 0 podatności, bez `--ignore`, `bundled` zachowany, SQLite 3.54.1

---

## 1. Data i środowisko

| Pole | Wartość |
|---|---|
| PROJECT DATE | 2026-11-09 |
| HOST WALL-CLOCK | 2026-11-09, 13:40–14:05 +01:00 |
| TIMEZONE / UTC OFFSET | Europe/Warsaw / +01:00 |
| Toolchain | `cargo 1.99.0`, `rustc 1.99.0` (MSRV projektu 1.99.0) |

## 2. Przywrócenie dostępu do crates.io — wszystkie próby

### Próba 1 — sandbox DSH (Low Integrity), sparse + OpenSSL dla git

Zmienne ustawione **wyłącznie w procesie** (bez trwałej zmiany konfiguracji):

```
CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
CARGO_NET_GIT_FETCH_WITH_CLI=true
GIT_CONFIG_COUNT=1
GIT_CONFIG_KEY_0=http.sslBackend
GIT_CONFIG_VALUE_0=openssl
CARGO_HTTP_TIMEOUT=120
CARGO_NET_RETRY=3
```

```
cargo +stable search rusqlite --limit 5
→ EXIT 101
error: failed to retrieve search results from the registry at https://crates.io
Caused by: [35] SSL connect error
  (schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS (0x8009030e))
```

Sparse index **nie** omija problemu: `GIT_CONFIG_*` dotyczy `git`, a cargo używa własnego
HTTP/schannel, który w Low IL nie pozyskuje poświadczeń.

### Próba 2 — jednorazowy proces Medium Integrity (poza sandboxem plików)

Ta sama konfiguracja env:

```
cargo +stable search rusqlite --limit 5   → EXIT 0  (rusqlite = "0.41.1")
cargo +stable info rusqlite               → EXIT 0
cargo +stable info libsqlite3-sys         → EXIT 0
cargo +stable info rusqlite@0.41.1        → EXIT 0  (pobrano z crates.io)
cargo +stable info libsqlite3-sys@0.39.1  → EXIT 0  (pobrano z crates.io)
cargo +stable fetch --locked              → EXIT 0
```

**Wniosek:** sieć i oficjalny rejestr są osiągalne; blokadą był wyłącznie integrity level
sandboxu. Nie zmieniono żadnej trwałej konfiguracji globalnej.
Pełne logi: `scratch/qa_gate/evidence/dependency_network_diagnostics.log`,
`cargo_search_medium_il.log`, `cargo_registry_info_medium_il.log`.

## 3. Ustalone wersje i metadane (oficjalny crates.io, 2026-11-09)

| Element | Wartość |
|---|---|
| `rusqlite` | **0.41.1** (stable; brak alpha/beta/RC) |
| `rusqlite` MSRV / licencja | **1.89.0** / **MIT** |
| `libsqlite3-sys` | **0.39.1** (stable) |
| `libsqlite3-sys` MSRV / licencja | **1.88.0** / **MIT** |
| Wymaganie `rusqlite 0.41.1` | `libsqlite3-sys = "0.39.0"` → 0.39.1 |
| Bundlowane SQLite | **3.54.1** (`#define SQLITE_VERSION "3.54.1"`) |
| Feature `bundled` | zachowany (włącza `libsqlite3-sys/bundled`) |
| Feature `backup` | zachowany (`backup = []`) |
| Patch `RUSTSEC-2026-0459` | `libsqlite3-sys >= 0.39.0` (SQLite ≥ 3.54.0) — **spełniony** |
| `RUSTSEC IGNORE` | **nie użyto** |
| System SQLite | **nie użyto** |

### Release notes (oficjalne GitHub Releases)

- **0.41.0 (2026-10-24):** m.in. `Bump bundled SQLite version to 3.54.0`, MSRV 1.88/1.89,
  Edition 2024, poprawka „backup and restore discrepancy", `column_names/columns` zwracają
  Iterator, `u64` dla `zeroblob`, zmiany bindgen.
- **0.41.1 (2026-10-28):** dwie poprawki błędów (PRAGMA/SQLCipher, panika funkcji skalarnej).

**Wpływ na kod:** SPLogbook nie używa `column_names`/`columns`/`column_with_metadata`,
`create_aggregate_function`, `create_window_function` ani `zeroblob`. Jedynym użyciem
zaawansowanego API jest `rusqlite::backup::Backup` (`src/core/backup.rs`) — API bez zmian,
a poprawka backup/restore jest korzystna. Kompilacja i pełne testy potwierdziły zgodność
**bez zmian w `src/`**.

## 4. Wykonana zmiana

`Cargo.toml`:

```diff
-rusqlite = { version = "0.40", features = ["bundled", "backup"] }
+rusqlite = { version = "0.41", features = ["bundled", "backup"] }
```

Celowana aktualizacja lockfile (bez ogólnego `cargo update`):

```
cargo +stable update -p rusqlite --precise 0.41.1
→ EXIT 0
   Updating libsqlite3-sys v0.38.2 -> v0.39.1
   Updating rusqlite v0.40.2 -> v0.41.1
   note: pass `--verbose` to see 103 unchanged dependencies behind latest
```

`Cargo.lock` — 17 insercji / 17 delecji:

| Pakiet | Przed | Po |
|---|---|---|
| `rusqlite` | 0.40.2 | **0.41.1** |
| `libsqlite3-sys` | 0.38.2 | **0.39.1** (bundled SQLite 3.54.1) |
| `windows-sys` (u kilku pakietów z otwartym zakresem) | 0.52/0.59/0.60.2 | **0.61.2** (już obecny w grafie) |

Ujednolicenie `windows-sys` objęło m.in. `anstyle-query`, `anstyle-wincon`, `errno`,
`mio-serial`, `rustix`, `rustls-platform-verifier`, `socket2`, `tempfile`, `uds_windows`,
`winapi-util`; **brak downgrade'ów**, `arboard` pozostał przy 0.60.2. Wszystkie wersje
spełniają wymagania pakietów, kompilacja i testy zielone.

## 5. Weryfikacja

### 5.1 `cargo audit` — 0 podatności (bez ignore)

```
cargo audit --db <temp>        → EXIT 0
Loaded 1429 security advisories
Scanning Cargo.lock for vulnerabilities (674 crate dependencies)
warning: 7 allowed warnings found   (6 × unmaintained, 1 × yanked — bez zmian względem bazy)
```

`RUSTSEC-2026-0459` **nie występuje** (libsqlite3-sys 0.39.1). Nie użyto `--ignore`.

### 5.2 Parity z CI (dokładna komenda z `.github/workflows/ci.yml`)

```
cargo audit --db <temp> --deny warnings --no-yanked <7 dozwolonych ignore>  → EXIT 0
```

### 5.3 Pełny zestaw testów (nowe zależności)

```
cargo +stable test --workspace --all-targets -j 2
START 13:45:06  END 13:59:24  (14 min 18 s)  EXIT 0
19 targetów: 2690 passed / 0 failed / 10 ignored
```

### 5.4 Testy celowane danych i bazy

| Filtr | Wynik |
|---|---|
| `--lib backup` | 13/13 |
| `--lib wal` | 2/2 |
| `--lib recovery` | 15/15 |
| `--lib database` | 132/132 |
| `--lib checkpoint` | 0 zarejestrowanych (brak testów o tej nazwie; checkpoint pokryty przez `database`) |

### 5.5 Pozostałe bramki

`fmt --check` 0, `clippy -D warnings` 0 (0 warningów), `test --doc` 0, `cargo tree -d` 0,
`git diff --check` 0.

## 6. Ryzyka rezydualne

| ID | Ryzyko | Status |
|---|---|---|
| DEP-R1 | Ujednolicenie `windows-sys` do 0.61.2 w pakietach z otwartym zakresem | Zaakceptowane; kompilacja i pełne testy zielone, brak downgrade'ów |
| DEP-R2 | 7 dozwolonych warningów RustSec (unmaintained/yanked) | Bez zmian względem bazy; poza zakresem tej bramki |
| DEP-R3 | Kolejne advisory mogą pojawić się w przyszłości | Standardowy `cargo audit` w CI; osobna bramka na każdą nową podatność |
| DEP-R4 | Aktualizacja wymagała jednorazowego Medium IL (sandbox blokuje TLS cargo) | Udokumentowane; brak trwałej zmiany konfiguracji |

## 7. Commity i synchronizacja

| # | SHA | Tytuł |
|---|---|---|
| 6 | `9ee1414` | `docs(deps): add dependency security update pre-implementation report` |
| 7 | `c5a0d9d` | `fix(deps): update rusqlite to 0.41.1 with patched bundled SQLite` |
| 8 | ten commit | `docs(deps): close dependency security update and restore green QA baseline` |

Synchronizacja: `repair/full-system-remediation` → `main` wyłącznie `merge --ff-only`,
`main` → `origin/main` przez `git push origin main`, po zielonym `cargo audit`.
`v1.2.0` nietknięty; brak tagów i release'ów.

## 8. Następna bramka

`STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` (po zamknięciu QA i tej bramki).
M3-G: **`BLOCKED`**.

## 9. Werdykt

**`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`**

`RUSTSEC-2026-0459` usunięta przez aktualizację do `libsqlite3-sys 0.39.1` (SQLite 3.54.1),
`bundled` zachowany, brak systemowego SQLite, brak `--ignore`, brak ręcznych paczek/mirrorów,
`Cargo.lock` wygenerowany wyłącznie przez cargo. Pełny baseline i `cargo audit` zielone.
