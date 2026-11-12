# STAGE-DEPENDENCY-RUSTIX-RUSTSEC-2026-0486-REMEDIATION-GATE — RAPORT

**Dokument ID:** DSH-SPLOGBOOK-DEP-RUSTIX-001
**Bramka:** `STAGE-DEPENDENCY-RUSTIX-RUSTSEC-2026-0486-REMEDIATION-GATE`
**Status bramki:** **`COMPLETED_DEPENDENCY_ADVISORY_REMEDIATED`**
**Decyzja OWNER:** `OWNER-DEP-RUSTIX-01` — `APPROVE OPTION A`
**Aktualna data projektu:** **2026-11-12**
**Czas pomiaru:** 2026-11-12 (`+01:00`, Europe/Warsaw)

---

## 1. Cel i zakres (zatwierdzony przez OWNER)

Usunięcie `RUSTSEC-2026-0486` przez **celowaną** aktualizację `rustix 1.1.5 → 1.1.6`
(`RUSTIX_TARGET_VERSION=1.1.6`), bez `--ignore`, bez szerokiego `cargo update`, bez zmian
`Cargo.toml`, bez zmian kodu/skryptów/workflowów AppImage i bez rozpoczynania nowych prac M3-G
w obrębie tej bramki.

## 2. Punkt wejścia

| Pole | Wartość |
|---|---|
| `HEAD = main = origin/main` | `b741a04e50b25adb2c93e699fd7bbbe898947ba6` |
| Ahead/behind | `0 / 0` |
| Drzewo śledzone | czyste (`?? scratch/` — lokalny, nieśledzony) |
| `F03_STATUS` | `CLOSED` |
| `M3_G_STATUS` | `IN_PROGRESS` / `BLOCKED_NEW_DEPENDENCY_ADVISORY` — **potwierdzone** |
| `M3_G_BLOCKER` | `RUSTSEC-2026-0486` (`rustix 1.1.5`) |
| Jedna sesja | potwierdzona (brak procesów `cargo`/`rustc`/`git`, brak `.git/index.lock`) |

Dokumenty odczytane: `AGENTS.md`, `docs/DEEPSEEK_HARNESS_HANDOFF.md`,
`docs/PROJECT_CURRENT_STATE.md`, `docs/NEXT_WORK_GATE.md`, `MASTER_IMPLEMENTATION_PLAN.md`,
`docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/DOCUMENTATION_INDEX.md`.

## 3. Advisory

| Pole | Wartość |
|---|---|
| ID | `RUSTSEC-2026-0486` |
| Crate | `rustix` |
| Zakres podatny | `>= 1.0.0, < 1.1.6` |
| Patch | `>= 1.1.6` |
| Typ | `informational = unsound` (xattr: zero-length buffer → UB) |
| Linia `0.38.x` | **nieobjęta** advisory |
| Wykrycie | `cargo audit --deny warnings --no-yanked` — `cargo audit` EXIT 1 na `b741a04` |

**Rzeczywista ścieżka `rustix 1.1.5`** (`cargo tree -i rustix@1.1.5 --target all`):
`rustix` ← `async-io` / `async-process` / `async-signal` / `calloop` / `calloop-wayland-source` /
`gethostname` / `polling` / `smithay-client-toolkit` / `tempfile` / `wayland-*` / `x11rb` /
`xattr` / `zbus` ← `keyring` / `secret-service` / `eframe` / `egui-winit` / `splogbook`.
(`rustix 0.38.44` ma odrębną, nieobjętą ścieżkę.)

## 4. Weryfikacja wersji docelowej (przed aktualizacją)

| Pole | Wartość |
|---|---|
| Źródło | `https://crates.io/api/v1/crates/rustix` (oficjalny rejestr) |
| Najnowsza stabilna | **`1.1.6`** (opublikowana `2026-10-27T00:49:03Z`); brak nowszej stabilnej |
| MSRV `1.1.6` | `1.65` (kompatybilne z toolchainem projektu `1.99.0`) |
| Zgodność SemVer | `1.1.5 → 1.1.6` — patch w tej samej linii `1.x` |
| `Cargo.toml` | **brak** pinu `rustix`; manifest nietknięty |

## 5. Wykonana aktualizacja

```bash
cargo +stable update -p rustix@1.1.5 --precise 1.1.6
# Updating rustix v1.1.5 -> v1.1.6
# note: pass `--verbose` to see 107 unchanged dependencies behind latest
```

Analiza `Cargo.lock` (parsowanie TOML przed/po):

| Metryka | Wartość |
|---|---|
| Paczki przed / po | `674 / 674` |
| Usunięte paczki | wyłącznie `rustix 1.1.5` |
| Dodane paczki | wyłącznie `rustix 1.1.6` |
| Zmienione bloki zależności | 24 (14 × `rustix 1.1.5 → 1.1.6`, 10 × efekt uboczny rozwiązywania `windows-sys`) |
| Checksum `rustix` | `891efaba…` → `e5e76742…` |
| `rustix 0.38.44` | **identyczny** (nietknięty) |
| Inne wersje paczek | **bez zmian** |
| `Cargo.toml` | **bez zmian** (`git diff -- Cargo.toml` pusty) |
| `--ignore` | **nie dodano**; lista 7 udokumentowanych ignore bez zmian |

Efekt uboczny `windows-sys`: cargo, re-rozwiązując graf po zmianie wymagań `rustix`, wybrał
inne (już obecne w locku) wersje `windows-sys` dla 10 paczek wyłącznie windowsowych
(`anstyle-query`, `anstyle-wincon`, `errno`, `mio-serial`, `quinn-udp`,
`rustls-platform-verifier`, `socket2`, `tempfile`, `uds_windows`, `winapi-util`).
Są to **niezmienione wersje paczek** i wyłącznie ścieżki `cfg(windows)`; zmiana została
wygenerowana w całości przez Cargo (dozwolone przez `OWNER-DEP-RUSTIX-01` pkt 8) i jest
odnotowana jawnie.

## 6. Pełny baseline i pełny audyt (2026-11-12)

| Komenda | Wynik |
|---|---|
| `cargo +stable fmt --all -- --check` | EXIT 0 |
| `cargo +stable check --workspace --all-targets -j 1` | EXIT 0 |
| `cargo +stable clippy --workspace --all-targets -j 1 -- -D warnings` | EXIT 0, 0 ostrzeżeń |
| `cargo +stable test --workspace --all-targets -j 1` | EXIT 0 — 20 targetów, **2717** zarejestrowanych, **2707** wykonanych, **2707** zaliczonych, **0** nieudanych, **10** ignorowanych |
| `cargo +stable doc --workspace --no-deps` | EXIT 0 |
| `cargo +stable tree -d` | EXIT 0 |
| `git diff --check` | EXIT 0 |
| `cargo audit --deny warnings --no-yanked <7 dozwolonych ignore>` | **EXIT 0** — 1458 advisory w bazie, 674 zależności, **0 znalezisk** |

`cargo audit` po aktualizacji nie zgłasza `RUSTSEC-2026-0486`; liczniki testów pozostają
kanoniczne (`2717 / 2707 / 10`), więc ratchet CI nie wymaga zmiany.

## 7. Commit i push

| Pole | Wartość |
|---|---|
| Commit | commit tej bramki (`git log -1 --format=%H`) |
| Tytuł | `fix(deps): remediate RUSTSEC-2026-0486 by updating rustix to 1.1.6` |
| Zmienione pliki | `Cargo.lock`, raport tej bramki, dokumentacja stanu |
| Push | `git push origin main` (fast-forward, bez force) |
| Po pushu | `HEAD == main == origin/main`, `ahead/behind = 0/0` |

## 8. Wpływ na M3-G

Blocker `BLOCKED_NEW_DEPENDENCY_ADVISORY` **został usunięty**. `M3_G_STATUS` wraca do
`IN_PROGRESS` (bez blokera). M3-G zostanie wznowione w **odrębnej** fazie: ponowny run CI,
potwierdzenie 4/4 profili, testów negatywnych i FUSE, a następnie domknięcie bramki.
W tej bramce **nie wykonano** żadnych zmian kodu, skryptów ani workflowów AppImage.

## 9. Ryzyka rezydualne

1. `windows-sys` dla 10 paczek windowsowych wybrano w innych wersjach (efekt resolvera) —
   brak wpływu na Linux i na wersje paczek; odnotowane.
2. Kolejne advisory mogą pojawić się w przyszłości → standardowy `cargo audit` w CI i osobna
   bramka na każdą nową podatność (bez `--ignore`).
3. M3-G nadal wymaga pełnych dowodów CI po wznowieniu; ten raport **nie** zamyka M3-G.
