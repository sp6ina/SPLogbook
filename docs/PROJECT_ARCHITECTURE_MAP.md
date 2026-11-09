# SPLogbook — MAPA ARCHITEKTURY PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-ARCH-001
**Data:** 2026-11-09
**Podstawa:** kod z `f73a703` (ingestia pełna: 292 pliki Rust, 155 097 linii) + bramka QA 2026-11-09.
**Zastępuje jako opis bieżący:** `docs/ARCHITECTURE_CURRENT_STATE.md` (opisuje `6f7d3aa` i pliki, których już nie ma).

---

## 1. Wejścia aplikacji i binaria

| Element | Ścieżka | Rola |
|---|---|---|
| Biblioteka | `src/lib.rs` | publiczne API crate `splogbook` (moduły domenowe) |
| Aplikacja GUI | `src/main.rs` (`SPLogbook`) | start eframe/egui, inicjalizacja stanu, CLI `--version` / `--build-info` / `--diagnostic-version` |
| Aktualizator | `src/bin/splogbook-updater.rs` | natywny helper Windows: IPC, plan update, rollback, HealthProof |
| Signer | `src/bin/signer.rs` | offline podpisywanie manifestu Ed25519 |
| Build identity | `build.rs` | `BuildIdentity` + generowanie katalogów i18n; wersja `1.3.0-alpha.1` |

## 2. Mapa modułów (stan `f73a703`)

| Moduł | Pliki | Linie | Zawartość | Status | Ryzyko / następna akcja |
|---|---:|---:|---|---|---|
| `src/cat` | 56 | 41 148 | 8 backendów CAT, 5 kodeków natywnych, HIL harness, TCI | `CURRENT` | F-14 (P0, hardware-gated), F-15 (HIL self-cert), F-17 (PTT F7) — poza tą bramką |
| `src/gui` | 107 | 31 602 | `SpLogApp`, 28 modułów stanu, ~78 widoków, motywy | `CURRENT` | F-16 (panic `page_range`), F-19 (SOTA/POTA) — poza tą bramką |
| `src/core` | 49 | 27 780 | domena QSO, SQLite, ADIF/Cabrillo, XDG, i18n | `CURRENT` | F-20/F-21 (eksport/upload) — poza tą bramką |
| `src/cloud` | 27 | 18 769 | updater (9 431), LoTW/eQSL/QRZ/CloudLog, scheduler | `CURRENT` | F-06 (integralność danych referencyjnych), F-22 (system-managed bez locka) |
| `src/sync` | 12 | 5 661 | protokół P2P/LAN v2, handshake, szyfrowanie, catch-up | `CURRENT` | F-23 naprawione w bramce QA (testy deterministyczne) |
| `src/api` | 2 | 3 527 | Axum REST + SSE/WS | `CURRENT` | DOC-DSH-14 (opis API w README) |
| `src/plugins` | 4 | 1 603 | sandbox Rhai, marketplace | `CURRENT` | — |
| `src/digital` | 5 | 1 553 | WSJT-X / JS8Call / FLdigi / N1MM | `CURRENT` | — |
| `src/media` | 4 | 718 | nagrywanie audio, dźwięki, voice keyer | `CURRENT` | 1 FFI `mciSendStringW` |
| `src/cluster` | 2 | 523 | telnet DX cluster | `CURRENT` | — |
| `src/dsp` | 2 | 276 | FFT, waterfall | `CURRENT` | — |
| `src/network` | 2 | 124 | Wake-on-LAN | `CURRENT` | — |
| `tests` | 15 + `support/` | 20 584+ | strażnicy, CAT natywny, instalatory, Linux | `CURRENT` | `support/native_artifact.rs` — nowy kontrakt hermetyczny QA |

## 3. Przepływ danych i trwałość

- **SQLite** jest jedynym źródłem prawdy (`src/core/database/`), WAL, backup online, migracje schematu.
- **Dane referencyjne** (`cty.dat`, `MASTER.SCP`, `lotw-user-activity.csv`) są pobierane przez updater (F-06: brak podpisu/hash).
- **Ścieżki danych:** Windows `%APPDATA%\SPLogbook`; Linux XDG (`$XDG_DATA_HOME/splogbook`) z fallbackiem `$HOME` (F-07: precedence portable vs XDG).
- **Eksport:** ADIF, Cabrillo (F-21: limit 10 000 QSO), CSV, SOTA.
- **Synchronizacja:** frame TCP (limit 8 MiB), chunking (1 MiB), pojedynczy rekord ≤ 6 MiB, szyfrowanie ChaCha20-Poly1305.

## 4. Aktualizator (mapa bezpieczeństwa)

```
manifest (Ed25519, raw bytes) → select_manifest_asset (SHA-256 + size)
→ UpdatePlan (UNKNOWN ⇒ DO NOT OVERWRITE) → staging/preflight
→ JOURNALED_TRANSACTIONAL_REPLACEMENT + .rollback + update_transaction.json
→ HealthProof → rollback deterministyczny / recovery z journala
```

Ścieżki instalacji:

| Typ instalacji | Model | Ścieżka aktualizacji |
|---|---|---|
| Windows Portable ZIP | `SelfManaged` | helper `splogbook-updater.exe` + fallback PowerShell (`CREATE_NO_WINDOW`) |
| Windows Inno Setup (User) | `SystemManaged` | delegacja do zweryfikowanego `Setup-*.exe` (`/SILENT /NORESTART /LOG=`), marker `.splogbook-inno-marker` |
| Windows MSI (System) | `SystemManaged` | delegacja do `msiexec` z logiem i `/norestart`, marker `.splogbook-msi-marker` |
| Linux Portable tar.gz | `SelfManaged` | dwufazowa ekstrakcja + transakcja journaled + `.rollback` |

## 5. Granice bezpieczeństwa (obowiązujące niezmienniki)

1. Brak manifestu lub nieznany/odwołany klucz ⇒ brak aktualizacji (fail-closed, przed I/O sieciowym).
2. SHA-256 i rozmiar związane z `VerifiedReleaseAsset`; podpis liczony po surowych bajtach manifestu.
3. Ochrona przed downgrade i kanałami wydania.
4. Brak `sudo`/`doas`/`pkexec`/`runas` w kodzie aplikacji (`OWNER-M2-07`).
5. Brak shell injection w fallbacku PowerShell (bez wildcardów i `iex`).
6. Instalatory nigdy nie usuwają danych użytkownika (seed `NeverOverwrite`, uninstall `dirifempty`, brak wildcardów).
7. HIL: 10 testów `#[ignore]`; PTT wymaga bramek środowiskowych i zgody; RF = 0.

## 6. Mapa testów (cele i liczność)

| Cel | Znaczenie |
|---|---|
| `lib` (unit, `#[cfg(test)]`) | domena, updater, sync, GUI-state; tu żyją 3 testy czasowe z F-23 |
| `tests/cat_native_6a/6b/7` | natywne protokoły CAT i HIL (7 zawiera 10 `#[ignore]`) |
| `tests/security_guard.rs` | strażnicy decyzji OWNER, w tym nowy strażnik hermetyczności QA |
| `tests/inno_user_data_safety_test.rs` | Inno: statyczne bezpieczeństwo + **opt-in** E2E |
| `tests/msi_user_data_safety_test.rs` | MSI: statyczne bezpieczeństwo + **opt-in** kontrola artefaktu |
| `tests/windows_inno_setup_test.rs`, `tests/windows_msi_test.rs` | integracja aktualizatora z instalatorami |
| `tests/linux_*` | archiwum, bezpieczeństwo ekstrakcji, XDG, macierz modelowa |
| `tests/i18n_guard.rs`, `tests/theme_guard.rs` | kontrakty statyczne |

## 7. Miejsca „cienkie” lub martwe (stan faktyczny)

- `src/cat/so2r.rs` — konfiguracja SO2R, brak pełnej ścieżki sprzętowej.
- WinKeyer — transport szeregowy zależny od sprzętu (HIL).
- AppImage / DEB / RPM / Flatpak — **brak implementacji** (M3-G…M3-J `NOT_STARTED`).
- MSI/Setup nie są budowane ani publikowane przez CI (F-05).
