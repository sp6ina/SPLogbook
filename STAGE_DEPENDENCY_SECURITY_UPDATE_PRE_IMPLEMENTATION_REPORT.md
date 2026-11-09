# RAPORT PRZEDIMPLEMENTACYJNY
# STAGE-DEPENDENCY-SECURITY-UPDATE-GATE

**Dokument ID:** DSH-SPLOGBOOK-DEPSEC-PRE-001
**Projekt:** SPLogbook
**Data:** 2026-11-09
**Podstawa:** wiążąca decyzja OWNER `REMEDIATION: LATEST_STABLE_SAFE_COMPATIBLE_DEPENDENCY_UPDATE`
**Bloker:** `RUSTSEC-2026-0459` (CVE-2026-119375) w `libsqlite3-sys 0.38.2`
**Status przed zmianami:** `BLOCKED_BY_NEW_DEPENDENCY_ADVISORY`

---

## 1. Wiążące ograniczenia OWNER

- `RUSTSEC IGNORE: PROHIBITED` — nie dodajemy `--ignore`, nie maskujemy advisory.
- `SYSTEM SQLITE: PROHIBITED` — pozostajemy przy `bundled`.
- `FEATURE BUNDLED: REQUIRED`.
- Nie modyfikujemy `Cargo.toml`/`Cargo.lock` przed zakończeniem tego raportu.
- Nie korzystamy z wersji znalezionych wyłącznie w lokalnym cache; nie zgadujemy wersji.
- Celowana aktualizacja lockfile, bez ogólnego `cargo update`.
- `PUSH BEFORE GREEN AUDIT: PROHIBITED`.

## 2. Kontrolowane przywrócenie dostępu do crates.io — wykonane próby

### Próba 1 — sandbox DSH (Low Integrity), sparse + OpenSSL dla git

Zmienne **tylko dla procesu** (bez trwałej zmiany konfiguracji):

```
CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
CARGO_NET_GIT_FETCH_WITH_CLI=true
GIT_CONFIG_COUNT=1
GIT_CONFIG_KEY_0=http.sslBackend
GIT_CONFIG_VALUE_0=openssl
CARGO_HTTP_TIMEOUT=120
CARGO_NET_RETRY=3
```

| Komenda | Exit | Wynik |
|---|---|---|
| `cargo +stable search rusqlite --limit 5` | **101** | `[35] SSL connect error (schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS)` |

Wniosek: `GIT_CONFIG_*` dotyczy wyłącznie `git`; cargo używa własnego HTTP (schannel).
Sparse index nie omija problemu — TLS cargo nie inicjalizuje się w Low IL.

### Próba 2 — jednorazowy proces Medium Integrity (poza sandboxem plików)

Ta sama konfiguracja env, `cargo +stable search rusqlite --limit 5` → **EXIT 0**:

```
rusqlite = "0.41.1"
```

Następnie (`cargo info`, `fetch --locked`) → **EXIT 0**. Wniosek: sieć i crates.io są osiągalne;
wyłącznie integrity level sandboxu blokował schannel. Nie zmieniono żadnej konfiguracji globalnej.

## 3. Ustalone fakty z oficjalnego rejestru (2026-11-09)

| Element | Wartość |
|---|---|
| `rusqlite` latest stable | **0.41.1** (bez alpha/beta/RC) |
| `rusqlite` MSRV | **1.89.0** (nasz toolchain: 1.99.0 — OK) |
| `rusqlite` licencja | **MIT** |
| `libsqlite3-sys` latest stable | **0.39.1** |
| `libsqlite3-sys` MSRV | **1.88.0** — OK |
| `libsqlite3-sys` licencja | **MIT** |
| Wymaganie `rusqlite 0.41.1` | `libsqlite3-sys = "0.39.0"` (→ 0.39.1) |
| Bundlowane SQLite | **3.54.1** (`#define SQLITE_VERSION "3.54.1"`) |
| Patch advisory | `libsqlite3-sys >= 0.39.0` (SQLite ≥ 3.54.0) — **spełniony** |
| Feature `backup` | istnieje (`backup = []`) |
| Feature `bundled` | istnieje (włącza `libsqlite3-sys/bundled`) |

`cargo info libsqlite3-sys@0.39.1` i `cargo info rusqlite@0.41.1` → **EXIT 0**;
paczki pobrane z oficjalnego crates.io do cache cargo (nie z mirroru).

## 4. Release notes (oficjalne GitHub Releases)

**0.41.0 (2026-10-24):** m.in. `Bump bundled SQLite version to 3.54.0`, `Lower MSRV to 1.88.0`,
`Edition 2024`, `Fix discrepancy between backup and restore`, `Make Statement::column_names,
columns, columns_with_metadata return Iterator`, `Bump bindgen`, `u64 for zeroblob`,
`Don't enable SQLITE_ENABLE_MEMORY_MANAGEMENT in bundled build`.
**0.41.1 (2026-10-28):** dwie poprawki błędów (PRAGMA/SQLCipher, panika funkcji skalarnej).

**Ocena wpływu na SPLogbook:** kod nie używa `column_names`/`columns`/`column_with_metadata`,
`create_aggregate_function`, `create_window_function` ani `zeroblob`; używa wyłącznie
`rusqlite::backup::Backup` (feature `backup`) — API kompatybilne, a poprawka „backup/restore
discrepancy" jest korzystna. Kompilacja i pełny zestaw testów zweryfikują zgodność.

## 5. Stan obecny i oczekiwana zmiana lockfile

```
Cargo.toml: rusqlite = { version = "0.40", features = ["bundled", "backup"] }
Cargo.lock: rusqlite 0.40.2, libsqlite3-sys 0.38.2
```

W lockfile są już: `chrono 0.4.45` (≥ 0.4.42), `time 0.3.55` (≥ 0.3.47), `jiff 0.2.37`,
`hashlink 0.12.2`, `smallvec 1.16.2`. Oczekiwane zmiany w zamknięciu `rusqlite`:
`rusqlite → 0.41.1`, `libsqlite3-sys → 0.39.1`, dodanie `rusqlite-macros 0.4.4`,
aktualizacja `uuid` (1.26.1 → ≥1.28 wymagane przez rusqlite) i ewentualnie `sqlite-wasm-rs`.
**Bez** ogólnego `cargo update`; wyłącznie `cargo update -p rusqlite --precise 0.41.1`
(z zamknięciem zależności).

## 6. Plan implementacji

1. `Cargo.toml`: `rusqlite = { version = "0.41", features = ["bundled", "backup"] }`
   (bez `default-features = false` — zachowanie obecnego zestawu funkcji).
2. `cargo update -p rusqlite --precise 0.41.1` (celowo, nie globalnie).
3. `cargo build --workspace --all-targets` — weryfikacja kompilacji, w tym bundled SQLite 3.54.1.
4. Testy: pełny `cargo test --workspace --all-targets -j 2` + celowane:
   - backup/restore/recovery (`core::backup`, `tests`),
   - WAL/SHM i checkpoint (`core::database::*`),
   - migracje i integralność danych,
   - `security_guard`, `i18n_guard`, `theme_guard`, `windows_*`, `linux_*`.
5. `cargo audit --db <temp>` na świeżej bazie → **0 podatności** (bez `--ignore`).
6. `cargo fmt --check`, `cargo check --all-targets --all-features`, `cargo clippy -D warnings`,
   `cargo test --doc`, `cargo tree -d`, `git diff --check`.
7. Raport końcowy, commity (osobno: zależność, ewentualne adaptacje, raport), ff `main`, push
   **dopiero po zielonym audit**.

## 7. Ryzyka i warunki zatrzymania

| Ryzyko | Reakcja |
|---|---|
| Breaking change API `rusqlite` | minimalna adaptacja kodu; pełne testy |
| Konflikt wersji `uuid`/`sqlite-wasm-rs` | rozwiązać celowo; nie downgrade'ować bezpieczeństwa |
| Nowa advisory w nowym zamknięciu | **STOP** — nie pushować, raport |
| Brak sieci przy pobieraniu | jedna próba w CI; jeśli brak — `BLOCKED_BY_CRATES_IO_NETWORK_ACCESS` |
| `bundled` nie kompiluje się (cc/MSVC) | zdiagnozować; nie przechodzić na system SQLite |
| Testy DB/WAL/backup czerwone | **STOP**, nie pushować |

**Zakazane:** `--ignore`, system SQLite, ręczne `.crate`, nieoficjalne mirroru, zmiana sum kontrolnych,
push przed zielonym audit, automatyczny merge, tag/release, HIL/PTT/RF.

---

*Raport sporządzono przed modyfikacją `Cargo.toml`/`Cargo.lock`, zgodnie z decyzją OWNER.*
