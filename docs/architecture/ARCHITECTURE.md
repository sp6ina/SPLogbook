# SPLogbook — MAPA ARCHITEKTURY PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-ARCH-001
**Data:** 2026-11-09; liczby plików i statusy zweryfikowane ponownie 2026-11-14.
**Podstawa:** kod z `f73a703`; liczby zweryfikowane 2026-11-14: `src/**` = **276** plików `.rs`, całe repozytorium = **296** plików `.rs`, `tests/` = 18 plików testowych + `support/`.
**Zastępuje jako opis bieżący:** wcześniejszy opis architektury z `6f7d3aa`, który opisywał pliki już nieistniejące; ten dokument jest jedynym bieżącym opisem architektury.

---

## 1. Wejścia aplikacji i binaria

| Element | Ścieżka | Rola |
|---|---|---|
| Biblioteka | `src/lib.rs` | publiczne API crate `splogbook` (moduły domenowe) |
| Aplikacja GUI | `src/main.rs` (`SPLogbook`) | start eframe/egui, inicjalizacja stanu, CLI `--version` / `--build-info` / `--diagnostic-version` |
| Aktualizator | `src/bin/splogbook-updater.rs` | natywny helper Windows: IPC, plan update, rollback, HealthProof |
| Signer | `src/bin/signer.rs` | offline podpisywanie manifestu Ed25519 |
| Build identity | `build.rs` | `BuildIdentity` + generowanie katalogów i18n; wersja `1.3.0` |

## 2. Mapa modułów (stan `f73a703`)

| Moduł | Pliki | Linie | Zawartość | Status | Ryzyko / następna akcja |
|---|---:|---:|---|---|---|
| `src/cat` | 56 | 41 148 | 8 backendów CAT, 5 kodeków natywnych, HIL harness, TCI | `CURRENT` | F-14 (P0), F-15 (HIL self-cert), F-17 (PTT) — rozwiązane programowo; HIL sprzętowe oczekuje na fizyczne radio |
| `src/gui` | 107 | 31 602 | `SpLogApp`, 28 modułów stanu, ~78 widoków, motywy | `CURRENT` | F-16 (`gui::pagination`, `slice::get`, checked arithmetic), F-17 (bramka intencji TX), F-19 (SOTA/POTA) — rozwiązane |
| `src/core` | 50 | ~28 000 | domena QSO, SQLite, ADIF/Cabrillo, XDG, i18n, model stron referencji dyplomowych | `CURRENT` | F-19 (`core::award_refs`), F-20/F-21 (eksport/upload) — rozwiązane |
| `src/cloud` | 27 | 18 769 | updater (9 431), LoTW/eQSL/QRZ/CloudLog, scheduler | `CURRENT` | F-06 (integralność danych referencyjnych), F-22 (system-managed bez locka) |
| `src/sync` | 12 | 5 661 | protokół P2P/LAN v2, handshake, szyfrowanie, catch-up | `CURRENT` | F-23 rozwiązane (testy deterministyczne) |
| `src/api` | 2 | 3 527 | Axum REST + SSE/WS | `CURRENT` | opis API w `README.md` |
| `src/plugins` | 4 | 1 603 | sandbox Rhai, marketplace | `CURRENT` | — |
| `src/digital` | 5 | 1 553 | WSJT-X / JS8Call / FLdigi / N1MM | `CURRENT` | — |
| `src/media` | 4 | 718 | nagrywanie audio, dźwięki, voice keyer | `CURRENT` | 1 FFI `mciSendStringW` |
| `src/cluster` | 2 | 523 | telnet DX cluster | `CURRENT` | — |
| `src/dsp` | 2 | 276 | FFT, waterfall | `CURRENT` | — |
| `src/network` | 2 | 124 | Wake-on-LAN | `CURRENT` | — |
| `tests` | 18 + `support/` | 20 584+ | strażnicy, CAT natywny, instalatory, Linux | `CURRENT` | `support/native_artifact.rs` — kontrakt hermetyczny E2E |

## 3. Przepływ danych i trwałość

- **SQLite** jest jedynym źródłem prawdy (`src/core/database/`), WAL, backup online, migracje schematu.
- **Dane referencyjne** (`cty.dat`, `MASTER.SCP`, `lotw-user-activity.csv`) są pobierane przez updater (F-06: brak podpisu/hash).
- **Ścieżki danych:** Windows `%APPDATA%\SPLogbook`; Linux XDG (`$XDG_DATA_HOME/splogbook`) z fallbackiem `$HOME` (F-07 = CLOSED w `R1D`: katalog `databases/` w `exe_dir`/CWD nie przysłania kanonicznej ścieżki użytkownika).
- **Eksport:** ADIF, Cabrillo (F-21: limit 10 000 QSO), CSV, SOTA.
- **Referencje dyplomowe (F-19):** jedno centralne mapowanie `core::award_refs` tłumaczy model `AwardReferences { sota, pota }` (każda strona osobno) na pola ADIF: `MY_SOTA_REF`/`SOTA_REF` oraz kontrakt POTA `MY_SIG`+`MY_SIG_INFO` / `SIG`+`SIG_INFO`, z polami `MY_POTA_REF`/`POTA_REF` wyłącznie dla listy wielu parków. Silnik ADIF nie mapuje tych pól samodzielnie; kontrakt opisuje `docs/ADIF_AWARD_REFERENCES.md`.
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
| Linux AppImage | `SelfManaged` | podmiana obrazu + health check; brak własnego kanału aktualizacji (manifest i podpis poza zakresem M3-G) |
| Linux DEB | `SystemManaged` | delegacja do menedżera pakietów (APT); brak `sudo`/`pkexec`; układ `/opt/SPLogbook` + `/usr/bin/splogbook` |
| Linux RPM | `SystemManaged` | delegacja do menedżera pakietów (DNF/zypper); brak `sudo`/`pkexec`; ten sam układ `/opt/SPLogbook` |

## 5. Granice bezpieczeństwa (obowiązujące niezmienniki)

1. Brak manifestu lub nieznany/odwołany klucz ⇒ brak aktualizacji (fail-closed, przed I/O sieciowym).
2. SHA-256 i rozmiar związane z `VerifiedReleaseAsset`; podpis liczony po surowych bajtach manifestu.
3. Ochrona przed downgrade i kanałami wydania.
4. Brak `sudo`/`doas`/`pkexec`/`runas` w kodzie aplikacji (`ADR-02`).
5. Brak shell injection w fallbacku PowerShell (bez wildcardów i `iex`).
6. Instalatory nigdy nie usuwają danych użytkownika (brak preinstalowanych baz `FirstInstallSeed` po `R1C`, uninstall `dirifempty`, brak wildcardów).
7. HIL: 10 testów `#[ignore]`; PTT wymaga niezależnego dowodu, bramek środowiskowych i zgody;
   manifest testu nie jest dowodem gotowości; RF = 0.
8. GUI (F-17): intencja TX wyłącznie przez jawną bramkę — żaden pojedynczy klawisz nie uruchamia
   PTT, oba kierunki (`Ctrl+Shift+F7` = TX ON, `Ctrl+F7` = TX OFF) wymagają dokładnych
   modyfikatorów i są blokowane przy fokusie tekstowym, kompozycji IME, braku fokusu okna i przy
   otwartym oknie modalnym; PTT ON wymaga wyniku F-15, PTT OFF nigdy nie jest przez F-15
   blokowane; stan TX w GUI pochodzi wyłącznie z telemetrii.
9. GUI (F-16, `PANIC_FREE_SOFTWARE_VERIFIED`): paginacja logbooka wyłącznie przez czysty model
   `gui::pagination` (`PaginationInput` → `PaginationWindow`). Liczniki stron, normalizacja
   indeksu i zakres `start..end` są liczone w jednym miejscu, przed slicingiem; dostęp do
   zakresu wyłącznie przez `slice::get`; brak `get_unchecked`, brak `catch_unwind`; pusty wynik
   i `page_size = 0` mają jawny stan; zaznaczenie QSO opiera się na stabilnym `id`, nie na
   indeksie strony. Szczegóły: [`docs/DEVELOPMENT.md`](../DEVELOPMENT.md) §P1-F16.

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
- AppImage / DEB / RPM / Flatpak — **ZREALIZOWANE** i zweryfikowane w CI (M3-G, M3-H, M3-I, M3-J).
- MSI/Setup nie są budowane ani publikowane przez CI (F-05).
## 8. Upload selection and batching (`ADR-113`–`ADR-121`)

```
GUI button (src/gui/online_sync.rs)
   └─ start_upload(target, transport, credentials)      src/gui/online_sync_upload.rs
        ├─ JournalId::new(active_journal.id)            (tożsamość z journals.id)
        ├─ LogDatabase::select_upload_candidates_for_journal(journal, target)
        │      WHERE journal_id = ?1  AND sent NOT IN ('Y','I')     -- bez LIMIT
        ├─ UploadPlan::build(...)                       src/cloud/upload_plan.rs
        │      sort kanoniczny (data, czas, id) · plan_id = SHA-256 · batch_size = polityka
        └─ run_plan(plan, cancel, transport)            pętla paczek, brak retry
               ├─ LogDatabase::get_qsos_by_ids_for_journal   -- ładunek tylko z tego dziennika
               ├─ export_adif(...)                            -- jeden pakiet = jedna paczka
               ├─ transport: TQSL / eQSL POST / ADIF / per-QSO
               └─ po dowodzie akceptacji:
                     LogDatabase::commit_batch_acceptance  -- transakcja + kontrola liczby wierszy
```

Niezmienniki: każdy plan ma jawny `journal_id`; każdy rekord należy do dokładnie jednej paczki;
suma paczek równa się liczbie rekordów planu; kolejność jest stabilna; status jest zapisywany
wyłącznie po dowodzie; częściowy sukces, timeout i anulowanie nigdy nie są sukcesem.

## F-20R — persistent upload checkpoints and download credential confinement (2026-11-17)

The online-upload subsystem gained a durable state machine:

```
src/cloud/upload_checkpoint.rs        pure model: states, transitions, digests, recovery actions
src/core/database/upload_checkpoint.rs SQLite persistence: plan/batch/member tables, atomic commit, recovery scan
src/gui/online_sync_upload.rs          orchestration: prepare -> start -> transport -> record -> commit
src/gui/app.rs                         startup recovery (local only, no transport)
```

Schema version 8 adds `upload_plans`, `upload_batches`, `upload_batch_members`; schema version 9
adds `qso_records.lotw_qslsdate` and `qso_records.eqsl_qslsdate`. Both migrations are additive,
transactional (`BEGIN IMMEDIATE`) and reversible in test scope.

The download path uses `core::http::SecretUrl` plus `http_client_no_redirect` for the two
documented query-credential endpoints. No new module depends on the network at construction time.