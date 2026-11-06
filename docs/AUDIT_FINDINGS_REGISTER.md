# Rejestr Znalezisk Audytowych SPLogbook (`AUDIT_FINDINGS_REGISTER.md`)

**Dokument audytowy:** `docs/AUDIT_FINDINGS_REGISTER.md`  
**Projekt:** SPLogbook (`v1.2.0`)  
**Gałąź referencyjna:** `main` (`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`)  
**Skala krytyczności:** `BLOCKER`, `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `INFO`  
**Skala pewności:** `CONFIRMED`, `HIGH CONFIDENCE`, `SUSPECTED`, `NOT VERIFIED`

---

## Podsumowanie Liczbowe Rejestru

| Kategoria | BLOCKER | CRITICAL | HIGH | MEDIUM | LOW | INFO | Razem |
|---|---|---|---|---|---|---|---|
| `SEC` (Bezpieczeństwo) | 1 | 0 | 0 | 3 | 1 | 0 | **5** |
| `REL` (Niezawodność / Updater) | 0 | 1 | 1 | 0 | 0 | 0 | **2** |
| `DATA` (Dane / SQLite / ADIF) | 0 | 4 | 2 | 1 | 0 | 0 | **7** |
| `FUNC` (Funkcjonalność / Domena) | 0 | 2 | 4 | 2 | 0 | 0 | **8** |
| `CAT` (Radio / CAT / HIL / PTT) | 0 | 4 | 8 | 4 | 1 | 0 | **17** |
| `ARCH` (Architektura) | 0 | 0 | 2 | 2 | 0 | 0 | **4** |
| `RUST` (Jakość kodu Rust) | 0 | 0 | 1 | 2 | 1 | 0 | **4** |
| `CONC` (Współbieżność / Async) | 0 | 0 | 2 | 1 | 0 | 0 | **3** |
| `GUI` (Interfejs / UX / A11y) | 0 | 0 | 2 | 3 | 1 | 0 | **6** |
| `PERF` (Wydajność) | 0 | 1 | 2 | 1 | 0 | 0 | **4** |
| `TEST` (Testy / QA) | 0 | 0 | 1 | 2 | 0 | 0 | **3** |
| `DOC` (Dokumentacja vs Kod) | 0 | 0 | 1 | 2 | 1 | 0 | **4** |
| `CI` (CI/CD i Release) | 0 | 0 | 2 | 1 | 0 | 0 | **3** |
| `DEP` (Zależności i Licencje) | 0 | 0 | 1 | 1 | 1 | 1 | **4** |
| **RAZEM** | **1** | **12** | **29** | **25** | **6** | **1** | **74** |

---

## 1. Bezpieczeństwo (`SEC-*`) i Niezawodność Aktualizacji (`REL-*`)

### `SEC-001` — Jawny klucz prywatny Ed25519 (`[1u8; 32]`) w śledzonym pliku `src/scratch.rs` odpowiadający produkcyjnemu kluczowi weryfikacji aktualizacji
- **ID:** `SEC-001`
- **Tytuł:** Skompromitowany klucz prywatny Ed25519 podpisujący manifesty auto-updatera w repozytorium Git
- **Kategoria:** `SEC`
- **Poziom krytyczności:** `BLOCKER`
- **Poziom pewności:** `CONFIRMED`
- **Status po Etapie 1B (SEC-001):**
  - **REMEDIATED_IN_TREE:** FIXED (materiał prywatny usunięty z drzewa roboczego, brak śledzonych plików kluczy);
  - **REGRESSION_GUARDED:** FIXED (pakiet 17 testów regresyjnych `tests/security_guard.rs` oraz CI `.github/workflows/ci.yml` wymuszają fail-closed i brak prywatnego materiału);
  - **LEGACY_KEYS_REVOKED:** FIXED (`splogbook-release-2026-01` oraz 5 odwołanych kluczy publicznych na stałe w `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS`);
  - **NEW_PUBLIC_KEY_ACTIVE:** FIXED (aktywny dokładnie jeden klucz `splogbook-release-2026-02`, fingerprint SHA-256: `2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a`);
  - **PRODUCTION_SECRET_STORED_EXTERNALLY:** OWNER_CONFIRMED (klucz prywatny wygenerowany poza repozytorium i skonfigurowany w GitHub Actions Secret `RELEASE_SIGNING_KEY`);
  - **HISTORY_EXPOSURE:** OWNER_DECISION_REQUIRED (materiały historyczne zneutralizowane kryptograficznie w aplikacji; ewentualne przepisywanie historii Git `git-filter-repo` pozostaje do wyłącznej decyzji Właściciela);
  - **HISTORICAL_RELEASE_AUDIT:** COMPLETED (przeprowadzono audyt wszystkich wydań: v1.2.0 sklasyfikowane jako `VERIFIED_REVOKED_KEY`, v1.0.0-v1.1.0 jako `MISSING_MANIFEST` / `UNSIGNED`; pełny raport: `docs/HISTORICAL_RELEASE_AUDIT.md`);
  - **AUTO_UPDATE_RELEASE_READINESS:** READY (wdrożono kompletne zabezpieczenia REL-001 oraz potok CI-001 w Etapie 2; auto-updater weryfikuje podpisy, potok publikuje podpisany manifest i SHA256SUMS.txt);
  - **SEC-001_OVERALL:** MITIGATED (kod, testy, klucze i audyt domknięte; ekspozycja historii oczekuje na decyzję Właściciela).
- **Moduł / obszar:** `src/scratch.rs`, `src/cloud/updater.rs`, `tests/security_guard.rs`, `.github/workflows/ci.yml`, `docs/SECURITY_KEY_ROTATION.md`
- **Lokalizacja:** [`src/cloud/updater.rs:257-300`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L257-L300) (`TRUSTED_KEYS`, `REVOKED_KEY_IDS`, `REVOKED_PUBLIC_KEYS`), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs), [`docs/SECURITY_KEY_ROTATION.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/SECURITY_KEY_ROTATION.md)
- **Dowód z kodu / Git:**
  - W commicie `80533e2` wprowadzono śledzone pliki kluczy `.signer_priv.key`, `.test.key`, `klucz.key` oraz `src/scratch.rs`. Plik `klucz.key` odpowiadał historycznemu publicznemu kluczowi `[203, 142, 210, 212...]`.
  - W commicie `54db3e0` podmieniono klucz publiczny na `[109, 72, 1, 34...]` z zachowaniem identyfikatora `splogbook-release-2026-01`, bez mechanizmu wycofania starego klucza.
  - Plik `src/scratch.rs` zawierał jawny kod wyliczający klucz ze stałego ziarna `[1u8; 32]`.
- **Opis problemu:** Ujawnienie materiału kryptograficznego w repozytorium unieważnia kryptograficzne zaufanie do wydań podpisywanych historycznymi kluczami.
- **Scenariusz wystąpienia:** Atakujący posiadający skompromitowany klucz prywatny mógłby spreparować podpisany manifest dla złośliwego pliku aktualizacji.
- **Wpływ:** Całkowite unieważnienie ochrony kryptograficznej Ed25519 w auto-updaterze przed wdrożeniem mechanizmu revocation.
- **Wdrożone rozwiązanie (Etap 1A):** Usunięto `src/scratch.rs` z drzewa, zaktualizowano `.gitignore`, dodano listę `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS` w `src/cloud/updater.rs`, wprowadzono stan fail-closed (`active: false`), dodano testy regresyjne `tests/security_guard.rs`, dodano krok weryfikacji w CI oraz udokumentowano procedurę w `docs/SECURITY_KEY_ROTATION.md`.
- **Wymagane działanie właściciela (Etap 1B):** Ręczne wygenerowanie nowego klucza Ed25519 poza repozytorium, umieszczenie klucza prywatnego w GitHub Secrets (`RELEASE_SIGNING_KEY`) oraz dostarczenie klucza publicznego do wdrożenia w `TRUSTED_KEYS`.
- **Ryzyko wdrożenia zmiany:** Niskie (updater fail-closed do czasu dostarczenia nowego klucza).
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Powiązane z `REL-001` i `CI-001`.
- **Wdrożone testy regresyjne:**
  - `updater_trusted_keys_reject_known_test_and_trivial_seeds` w `tests/security_guard.rs`
  - `repository_contains_no_tracked_private_key_material` w `tests/security_guard.rs`
- **Kryterium akceptacji Etapu 1A:** Spełnione — brak `src/scratch.rs`, brak śledzonych kluczy prywatnych, klucz `splogbook-release-2026-01` odwołany, testy przechodzą pomyślnie, status udokumentowany.

---

### `REL-001` — Auto-updater kończy się błędem instalacji zarówno na Windows, jak i na Linux
- **ID:** `REL-001`
- **Tytuł:** Niedziałający proces instalacji aktualizacji (`install_update`) na Windows i Linux
- **Kategoria:** `REL`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED / VERIFIED_NATIVE_CI**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commity:**
  - `9ac1e5b`: `fix(updater): secure archive extraction and platform staging (REL-001)`
  - `cdcb955`: `fix(updater): make installation atomic with rollback (REL-001)`
  - `9557db8`: `fix(updater): validate Windows ZIP entries before extraction (REL-001)`
  - `3ebd8ac`: `fix(updater): harden transactional installation rollback (REL-001)`
- **Weryfikacja natywna (Etap 2 Końcowa Bramka):**
  - **Natywne testy Windows (job `124864486955`, 28m30s na `windows-latest`):** Pełna weryfikacja bezpieczeństwa rozpakowywania ZIP przed ekstrakcją (`validate_zip_archive`), odrzucanie path traversal, ścieżek bezwzględnych, prefiksów dysków, ścieżek UNC i symlinków (`windows_production_zip_path_*`), oraz weryfikacja semantyki `TRANSACTIONAL_FILE_REPLACEMENT` z atomowym rollbackiem każdego zastąpionego pliku. Status: PASS (100%).
  - **Natywne testy Linux (job `124864487116`, 18m52s na `ubuntu-latest`):** Bezpieczna ekstrakcja `.tar.gz`, spłaszczanie pojedynczego katalogu głównego oraz czyste uprawnienia `0o755`/`0o644`. Status: PASS (100%).
- **Moduł / obszar:** `src/cloud/updater.rs`, `.github/workflows/build-and-release.yml`, `docs/RELEASE_SECURITY.md`
- **Lokalizacja:** [`src/cloud/updater.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs), [`.github/workflows/build-and-release.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/build-and-release.yml)
- **Dowód z kodu przed naprawą:**
  1. Na Windows `install_update` zapisywało pobrany plik jako `.SPLogbook_update_{pid}.tmp`, a `install_via_powershell` wywoływało `Expand-Archive -LiteralPath $new -DestinationPath $stage -Force`. W Windows PowerShell 5.1 (`powershell.exe`) cmdlet `Expand-Archive` odrzucał pliki bez rozszerzenia `.zip` błędem `The path '...' is not a supported archive file format. .zip is the only supported format`. Ponadto `select_asset_for_platform` preferowało `.exe` przed `.zip`.
  2. Na Linux workflow pakował archiwum z katalogiem nadrzędnym `SPLogbook-Linux-x86_64/splogbook`, podczas gdy updater przenosił plik do `base_dir/SPLogbook-Linux-x86_64/splogbook` zamiast nadpisać `base_dir/splogbook`. Dodatkowo `select_asset_for_platform` preferowało `.appimage` przed `.tar.gz`.
- **Wdrożone rozwiązanie (Etap 2):**
  1. W `select_manifest_asset` i `select_asset_for_platform` wymuszono ścisły dobór `.zip` dla Windows i `.tar.gz` dla Linux na podstawie jawnych pól platformy/architektury.
  2. Pobierany plik na Windows otrzymuje bezpieczną ścieżkę z rozszerzeniem `.zip` (`.SPLogbook_update_{pid}.zip`).
  3. Bezpieczne rozpakowywanie `extract_tar_gz_safe` oraz `extract_zip_safe_windows` z ochroną przed path traversal (`validate_archive_relative_path`), odrzucaniem symlinków, hardlinków i plików specjalnych oraz limitami `ArchiveLimits`.
  4. Wdrożono spłaszczanie pojedynczego katalogu nadrzędnego (`flatten_single_root_directory`).
  5. Atomowa podmiana i transakcyjność (`apply_atomic_installation_with_options`): walidacja układu staging (`validate_staging_layout`), kopia zapasowa do `.backup_<random>` w katalogu aplikacji, podmiana plików, walidacja sprawności binarnej oraz automatyczny natychmiastowy rollback przy dowolnym błędzie lub awarii.
  6. Wymuszenie czystych uprawnień `0o755`/`0o644` bez bitów setuid/setgid/sticky.
- **Wdrożone testy regresyjne:**
  - 14 testów bezpieczeństwa archiwów: `archive_never_writes_outside_staging`, `archive_rejects_parent_traversal`, `archive_rejects_absolute_paths`, `archive_rejects_symlink_entries`, `archive_rejects_hardlink_entries`, `archive_rejects_too_many_entries`, `archive_rejects_extracted_size_over_limit`, itp.
  - 16 testów procedury instalacji, walidacji i rollbacku: `updater_keeps_old_installation_until_new_one_is_ready`, `updater_rolls_back_on_install_failure`, `updater_rolls_back_when_final_rename_fails`, `updater_cleanup_removes_failed_staging`, `updater_rollback_preserves_original_file_contents`, `updater_rejects_artifact_replaced_after_verification`, `updater_rolls_back_when_windows_replace_fails`, `updater_rolls_back_when_linux_rename_fails`, `updater_rejects_incomplete_staging_layout`, itp.

---

### `SEC-002` — Wyciek sekretów przez `#[derive(Debug)]` na `AppConfig` oraz hasła w parametrach query URL
- **ID:** `SEC-002`
- **Tytuł:** Brak redakcji sekretów w `Debug for AppConfig` i wyciek haseł w błędach `reqwest::Error`
- **Kategoria:** `SEC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Status po Etapie 1C:** **FIXED** (Commit `6c4a30f`)
  - Usunięto automatyczne `#[derive(Debug)]` ze struktur konfiguracyjnych zawierających sekrety (`AppConfig`, `UploadCredentials`, `SyncConfig`).
  - Wdrożono bezpieczną ręczną implementację `std::fmt::Debug` maskującą wszystkie 11 pól sekretnych stałym literałem `"<redacted>"`.
  - Wprowadzono w `src/core/http.rs` procedury `sanitize_url`, `redacted_url_for_diagnostics` oraz `sanitize_network_error`.
  - Wdrożono redakcję wrażliwych parametrów URL i oczyszczanie błędów sieciowych we wszystkich klientach chmurowych (`lotw.rs`, `eqsl.rs`, `hamqth.rs`, `qrz.rs`, `cloudlog.rs`, `hrdlog.rs`, `clublog.rs`).
  - Dodano testy regresyjne: `app_config_debug_redacts_all_secrets`, `online_services_debug_redacts_all_secrets`, `runtime_services_debug_redacts_all_secrets`, `cloud_errors_never_expose_credentials`, `secret_query_parameters_are_redacted`, `configuration_debug_never_leaks_secrets`.
- **Moduł / obszar:** `src/core/station.rs`, `src/cloud/lotw.rs`, `src/cloud/eqsl.rs`, `src/cloud/hamqth.rs`, `src/cloud/qrz.rs`
- **Lokalizacja:** [`src/core/station.rs:276, 387-435`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/station.rs#L276-L435), [`src/cloud/lotw.rs:174`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/lotw.rs#L174), [`src/cloud/eqsl.rs:72`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/eqsl.rs#L72), [`src/cloud/hamqth.rs:138`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/hamqth.rs#L138), [`src/cloud/qrz.rs:48`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/qrz.rs#L48)
- **Dowód z kodu:**
  - `OnlineServicesConfig` (`station.rs:74`), `RuntimeServicesConfig` (`station.rs:212`) i `AppConfig` (`station.rs:276`) mają automatyczne `#[derive(Debug)]`, które wypisuje jawnie m.in. `lotw_password`, `eqsl_password`, `qrz_api_key`, `clublog_password`, `rest_api_token`, `lan_sync_secret`.
  - W `lotw.rs:174`, `eqsl.rs:72`, `hamqth.rs:138`, `qrz.rs:48` hasła są doklejane do URL (`&password=...`). Gdy `reqwest` zwróci błąd połączenia/DNS/TLS, `e.to_string()` zawiera pełny URL wraz z hasłem i trafia do `online_sync_logs` oraz `SPLogbook.log`.
- **Opis problemu:** Hasła mogą przeniknąć do logów diagnostycznych (`SPLogbook.log`) lub zrzutów ekranu podczas awarii sieci.
- **Scenariusz wystąpienia:** Brak internetu podczas synchronizacji z LoTW/eQSL/QRZ lub wywołanie `format!("{config:?}")`.
- **Wpływ:** Ujawnienie poświadczeń LoTW/eQSL/QRZ/ClubLog w plikach logów.
- **Rozwiązanie minimalne:** Ręczna implementacja `fmt::Debug` dla `OnlineServicesConfig`, `RuntimeServicesConfig` i `AppConfig` maskująca 11 pól sekretnych jako `"[REDACTED]"`, oraz użycie `e.without_url()` na błędach `reqwest::Error` przed konwersją na komunikat.
- **Rozwiązanie docelowe:** Typ opakowujący `SecretString` z zamaskowanym `Debug`/`Display` i `ZeroizeOnDrop`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `app_config_debug_output_never_contains_secret_values`.
- **Kryterium akceptacji:** `format!("{cfg:?}")` nie zawiera żadnego z 11 sekretów, a błędy HTTP są pozbawione query stringu (`without_url()`).

---

### `SEC-003` — Brak timeoutu na `host_handshake` w `SyncHost` pozwala zablokować wszystkie 5 slotów połączeń przychodzących
- **ID:** `SEC-003`
- **Tytuł:** Możliwość wyczerpania semafora `MAX_HANDSHAKES = 5` przez niekompletne połączenia TCP (Slowloris DoS)
- **Kategoria:** `SEC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Status po Etapie 1C:** **FIXED** (Commit `4f3f3d5`)
  - Zdefiniowano `pub const HOST_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);` w `src/sync/host.rs` i wyeksportowano w `src/sync/mod.rs`.
  - Opakowano wywołanie `host_handshake` w pętli nasłuchu `run_listener_loop` za pomocą `tokio::time::timeout`.
  - Po upłynięciu timeoutu emitowane jest zdarzenie `SyncEvent::Error(SyncError::Timeout)`, a zadanie asynchroniczne kończy bieg, gwarantując zwolnienie slotu semafora `handshake_sem` (RAII permit drop) oraz natychmiastowe zamknięcie gniazda TCP.
  - Dodano testy regresyjne: `sync_host_handshake_timeout_releases_slot`, `sync_partial_handshake_times_out`, `sync_successful_handshake_is_not_rejected_by_timeout`, `security_guard_bind_policy_and_handshake_timeout`.
- **Moduł / obszar:** `src/sync/host.rs`
- **Lokalizacja:** [`src/sync/host.rs:78-115`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/host.rs#L78-L115)
- **Dowód z kodu:** W `SyncHost::bind_with_store` (`src/sync/host.rs:88-100`) po pobraniu permitu z `handshake_sem` (`MAX_HANDSHAKES = 5`) wywoływane jest `host_handshake(stream, ...).await` **bez `tokio::time::timeout`**.
- **Opis problemu:** Jeśli 5 gniazd TCP połączy się z portem Hosta i nie wyśle ramki `ClientHello`, wszystkie 5 permitów semafora pozostanie zajętych w nieskończoność, blokując możliwość dołączenia jakiegokolwiek uprawnionego operatora.
- **Scenariusz wystąpienia:** 5 wiszących połączeń TCP (np. skaner portów w sieci LAN lub zerwane przed handshake'iem sesje Wi-Fi).
- **Wpływ:** Całkowita blokada nowych połączeń do `SyncHost` do czasu restartu usługi.
- **Rozwiązanie minimalne:** Opakować `host_handshake(...)` w `tokio::time::timeout(Duration::from_secs(10), host_handshake(...)).await`.
- **Rozwiązanie docelowe:** Timeout 10 s na cały handshake + `TCP_KEEPALIVE` na gnieździe.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `sync_host_drops_stalled_handshake_and_releases_semaphore_permit`.
- **Kryterium akceptacji:** 5 milczących połączeń TCP zostaje rozłączonych po upływie timeoutu, zwalniając permity `handshake_sem`.

---

### `SEC-004` — Domyślne bindowanie `SyncHost` na `0.0.0.0` oraz brak ostrzeżenia o pracy bez tokenu w REST API
- **ID:** `SEC-004`
- **Tytuł:** Nadmierna ekspozycja gniazda LAN Sync (`0.0.0.0`) i cicha praca REST API z pustym tokenem
- **Kategoria:** `SEC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Status po Etapie 1C:** **FIXED** (Commit `205a39f`)
  - Wprowadzono typ `ServerBindError` oraz procedurę `validate_bind_policy(ip: &IpAddr, api_key: &str) -> Result<(), ServerBindError>` w `src/api/server.rs`, uniemożliwiającą uruchomienie serwera na interfejsie publicznym (`!ip.is_loopback()`) bez skonfigurowanego klucza API.
  - W `start_api_server_with_state` podłączono weryfikację polityki wiązania oraz skonfigurowano `app.into_make_service_with_connect_info::<SocketAddr>()`.
  - W `auth_middleware` rozszerzono ekstrakcję tokena o `Authorization: Bearer <token>` oraz `x-api-key`, z weryfikacją w stałym czasie (`constant_time_eq`).
  - Gdy klucz API jest pusty, operacje mutujące (`POST`, `PUT`, `PATCH`, `DELETE`) są rygorystycznie chronione: odrzucają (403 Forbidden) żądania od klientów zdalnych (`!ip.is_loopback()`), z obcym nagłówkiem `Origin` lub z `Sec-Fetch-Site: cross-site`, dopuszczając mutacje wyłącznie dla zaufanych lokalnych procesów loopback pod polityką local-only.
  - Zachowano kontrakt dla publicznych endpointów odczytu (`/api/v1/status`, `/api/v1/endpoints`).
  - Dodano testy regresyjne: `rest_mutations_require_token_when_configured`, `rest_mutations_reject_remote_clients_when_token_is_empty`, `rest_mutations_allow_loopback_client_under_local_only_policy`, `rest_import_rejects_untrusted_browser_origin_without_token`, `rest_read_only_endpoint_preserves_existing_contract`, `rest_never_logs_or_returns_api_token`, `rest_server_refuses_public_bind_without_authentication`, `security_guard_bind_policy_and_handshake_timeout`.
- **Moduł / obszar:** `src/sync/service.rs`, `src/api/server.rs`
- **Lokalizacja:** [`src/sync/service.rs:144`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/service.rs#L144), [`src/api/server.rs:347-370`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/api/server.rs#L347-L370)
- **Dowód z kodu:**
  - `SyncService::start` (`src/sync/service.rs:144`) binduje `SyncHost` na sztywno do `0.0.0.0:{port}` bez możliwości wyboru interfejsu w `SyncRole::Host`.
  - W `api/server.rs:351` gdy `state.api_token.is_none()`, middleware `auth_middleware` przepuszcza wszystkie żądania mutujące (`DELETE /api/v1/qsos/{id}`, `POST /api/v1/adif/import`) z dowolnego procesu lokalnego lub strony www (brak ścisłej blokady CORS/Origin dla żądań prostych `POST text/plain` w `import_adif_handler`).
- **Opis problemu:** Brak możliwości zawężenia interfejsu nasłuchu Hosta LAN Sync oraz podatność `POST /api/v1/adif/import` (`text/plain`) na żądania cross-origin z przeglądarki, gdy użytkownik włączy REST API bez ustawienia tokenu.
- **Scenariusz wystąpienia:** Użytkownik włącza REST API bez tokenu i otwiera w przeglądarce stronę wysyłającą `fetch("http://127.0.0.1:8420/api/v1/adif/import", { method: "POST", body: "..." })`.
- **Wpływ:** Nieautoryzowany import ADIF z poziomu przeglądarki przy pustym tokenie REST API.
- **Rozwiązanie minimalne:** W `auth_middleware` odrzucać żądania posiadające nagłówek `Origin` lub `Sec-Fetch-Site: cross-site`, jeśli `api_token` jest pusty (lub generować losowy token przy pierwszym włączeniu REST API).
- **Rozwiązanie docelowe:** Automatyczne generowanie tokenu Bearer przy aktywacji REST API + odrzucanie żądań z nagłówkiem `Origin`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `rest_api_rejects_browser_origin_requests_when_token_unset`.
- **Kryterium akceptacji:** Żądanie `POST /api/v1/adif/import` z nagłówkiem `Origin: https://evil.example` bez tokenu Bearer zwraca `403 Forbidden`.

---

## 2. Dane, Baza SQLite, ADIF i Integralność Dzienników (`DATA-*`)

### Ujednolicone mapowanie znalezisk i commitów Etapu 5 (Integralność Danych: ADIF, Ingest i Indeksy SQLite)

Status Etapu 5: **SOFTWARE VERIFIED**

| Identyfikator | Tytuł znaleziska | Wdrożone rozwiązanie | Commit | Moduły / Pliki | Testy weryfikujące | Status po Etapie 5 | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `DATA-001` | Parser ADIF fabrykuje bieżącą datę/czas `Utc::now()` i uszkadza wielobajtowy UTF-8 | Eliminacja fabrykacji daty/czasu, tryby `AdifParseMode::Strict` i `Compatibility`, ostrzeżenia diagnostyczne, kanoniczny eksport UTF-8 | `56b44a4`, `2fe66b6` | `src/core/adif.rs`, `src/cloud/lotw.rs` | 36 testów jednostkowych w `core::adif`, w tym 7 testów jednoznaczności i ostrzeżeń | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `DATA-002` | `QsoIngestor::import_batch` pomija normalizację i atomowość wsadową | Normalizacja pól w batchu, dowodowa atomowość pojedynczego zapisu i batcha, pełny rollback w SQLite i stanach pochodnych | `9b7f0fc`, `b6bedf6` | `src/core/ingest/{import.rs,mod.rs}` | 14 testów wsadowych i awaryjnych, weryfikacja failpointów, rollbacku i ponowienia | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `DATA-003` | Precyzyjna tożsamość QSO v2, unikalność w SQLite i eliminacja false-positive deduplikacji | Precyzyjny klucz v2 QsoIdentity (S/M czas, freq w Hz, normalizacja SUBMODE), MODEL B dla rekordów niepełnych (NULL), migracja schematu 7 z atomowym wycofaniem przy konfliktach, bazodanowy UNIQUE index (journal_id, identity_key) | `1393079`, `1e448dd`, `268e2d2` | `src/core/qso.rs`, `src/core/database/{qso_store.rs,migrations.rs,tests.rs}`, `src/core/ingest/{import.rs,mod.rs}` | 104 testy w `core::database`, w tym testy precyzji sekund, częstotliwości, subemisji, izolacji dzienników i wyścigów wielopołączeniowych | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `ARCH-002` | Edycja QSO omija Ingestor oraz brak odbudowywalności stanów pochodnych | Unifikacja ścieżki edycji w `update_existing`, metoda `rebuild_derived_state()`, emisja zdarzeń wyłącznie po commit SQLite | `50321f1`, `3d8463b` | `src/core/ingest/mod.rs`, `src/core/scp.rs` | 54 testy w `core::ingest`, 6 testów w `core::scp`, testy rekonstrukcji po awarii | `FIXED_SOFTWARE_VERIFIED` | NISKIE |

### `DATA-001` — Parser ADIF fabrykuje bieżącą datę/czas `Utc::now()` przy brakujących polach `<QSO_DATE>` / `<TIME_ON>` i uszkadza wielobajtowy UTF-8
- **ID:** `DATA-001`
- **Tytuł:** Fabrykowanie `Utc::now()` w `adif::fields_to_qso` oraz błędne indeksowanie bajtowe wielobajtowych znaków UTF-8 w parserze ADIF
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/core/adif.rs`
- **Lokalizacja:** [`src/core/adif.rs:155-167`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L155-L167), [`src/core/adif.rs:425-445`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L425-L445)
- **Dowód z kodu:**
  1. W `fields_to_qso` (`src/core/adif.rs:428`) wywoływane jest `let mut qso = QsoRecord::new(callsign, band, mode);`, które w [`src/core/qso.rs:133-134`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/qso.rs#L133-L134) inicjalizuje `qso_date` i `time_on` wartością `Utc::now()`. Jeśli rekord ADIF nie zawiera tagu `<QSO_DATE>` lub `<TIME_ON>` (np. częściowa odpowiedź QSL lub uszkodzony rekord), parser **nie odrzuca rekordu**, lecz po cichu przypisuje mu dzisiejszą datę i godzinę importu!
  2. W `parse_single_record_tags` (`src/core/adif.rs:159-162`) długość pola `<TAG:L>` jest traktowana bezwarunkowo jako liczba bajtów (`&bytes[idx..end]`, `String::from_utf8_lossy`). Gdy plik ADIF z innego programu (np. Logger32, UR5EQF, CQRLog) podaje liczbę znaków Unicode dla polskich znaków diakrytycznych (np. `<QTH:6>Kraków` — 6 znaków, 7 bajtów UTF-8), wycinek `&bytes[idx..idx+6]` przecina 2-bajtowy znak `ó` (`0xC3 0xB3`) w połowie, zamieniając go na `Krak`!
- **Opis problemu:** Złamanie kontraktu `PIPE-4B.1` (zakaz fabrykowania brakujących pól podczas importu ADIF) oraz psucie polskich znaków w polach `NAME`/`QTH`/`COMMENT`.
- **Scenariusz wystąpienia:** Import pliku ADIF bez `<QSO_DATE>`/`<TIME_ON>` lub z polskimi znakami diakrytycznymi w `<NAME>`/`<QTH>`.
- **Wpływ:** Ciche zanieczyszczenie dziennika łącznościami z fałszywą datą dzisiejszą oraz uszkodzenie polskich znaków w imionach i QTH.
- **Rozwiązanie minimalne:**
  1. W `fields_to_qso` inicjalizować `qso.qso_date = String::new()` i `qso.time_on = String::new()` przed pętlą tagów, aby `qso.validate()` odrzuciło rekordy bez daty/czasu.
  2. W `parse_single_record_tags` sprawdzić `if !content.is_char_boundary(end)`, a w przypadku przecięcia wielobajtowego znaku UTF-8 odczytać `length` znaków (`content[idx..].chars().take(length)`).
- **Rozwiązanie docelowe:** Obsługa obu wariantów kodowania długości ADIF (byte-count z fallbackiem char-count na granicy UTF-8) + rygorystyczna inicjalizacja pustych pól `qso_date`/`time_on`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `adif_import_rejects_record_missing_qso_date_or_time_on` oraz `adif_import_preserves_multibyte_utf8_when_length_is_char_count`.
- **Kryterium akceptacji:** Rekord ADIF bez `<QSO_DATE>` lub `<TIME_ON>` jest odrzucany w `AdifParseReport.rejected`, a `<QTH:6>Kraków<EOR>` parsuje się do `"Kraków"`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-03
- **Commit:**
  - `56b44a4`: `fix(adif): enforce bounded and deterministic record parsing (DATA-001)`
- **Wdrożone rozwiązanie:**
  1. Zdefiniowano twarde limity zasobów ADIF (`MAX_ADIF_FILE_SIZE = 64 MiB`, `MAX_ADIF_HEADER_SIZE = 1 MiB`, `MAX_ADIF_FIELD_NAME_LEN = 64`, `MAX_ADIF_FIELD_VALUE_LEN = 1 MiB`, `MAX_ADIF_FIELDS_PER_RECORD = 256`).
  2. Wprowadzono warianty błędów `AdifRecordError::MissingDate` oraz `AdifRecordError::MissingTime`. W `fields_to_qso` pola daty i czasu są inicjalizowane jako puste – brak tagu `<QSO_DATE>` lub `<TIME_ON>` powoduje jawne odrzucenie rekordu bez fabrykowania `Utc::now()`.
  3. Wdrożono dualne wyznaczanie granic pól (dual-mode length extraction): weryfikacja byte-count z fallbackiem na char-count, zapobiegając ucinaniu wielobajtowych znaków UTF-8 (np. polskich znaków w nazwach QTH/Name).
- **Wdrożone testy regresyjne:**
  - 11 nowych deterministycznych testów regresyjnych w `src/core/adif.rs` weryfikujących zachowanie znaków diakrytycznych (`Kraków`, `Łódź`), odrzucanie brakujących dat/czasów, odporność na nieznane pola ADIF, oraz brak terminacji na `<EOR>` wewnątrz wartości pola.

---

### `DATA-002` — `QsoIngestor::import_batch` pomija wywołanie `normalize_qso_fields(&mut qso)`
- **ID:** `DATA-002`
- **Tytuł:** Brak normalizacji pól QSO (pasmo z częstotliwości, format daty/czasu, wielkość liter) w `QsoIngestor::import_batch`
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/core/ingest/import.rs`, `src/sync/store.rs`
- **Lokalizacja:** [`src/core/ingest/import.rs:63-88`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/ingest/import.rs#L63-L88), [`src/sync/store.rs:58-65`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/store.rs#L58-L65)
- **Dowód z kodu:** Podczas gdy `QsoIngestor::ingest` ([`src/core/ingest/mod.rs:127`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/ingest/mod.rs#L127)) wywołuje `normalize_qso_fields(&mut qso)` przed `qso.validate()`, metoda `QsoIngestor::import_batch` ([`src/core/ingest/import.rs:63-88`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/ingest/import.rs#L63-L88)) od razu wywołuje `if let Err(issue) = qso.validate()`, nigdy nie wywołując `normalize_qso_fields(&mut qso)`!
- **Opis problemu:**
  1. Rekord ADIF zawierający `<FREQ:6>14.074` bez tagu `<BAND>` (w pełni legalny w ADIF) zostaje odrzucony przez `import_batch` jako `MissingBand`, ponieważ wyznaczenie pasma z częstotliwości znajduje się w `normalize_qso_fields` (`normalize.rs:28-33`).
  2. W `SyncStore::import` (`src/sync/store.rs:60-63`) używane jest `ImportOptions { enrich: false, .. }`. Bez `normalize_qso_fields` rekordy z LAN Sync mogą trafić do SQLite z datą `2026-10-18` (z myślnikami) lub małą literą w znaku/pasmie.
- **Scenariusz wystąpienia:** Import pliku ADIF z programu eksportującego tylko `<FREQ>` bez `<BAND>` lub synchronizacja Catch-Up w sieci LAN.
- **Wpływ:** Odrzucanie poprawnych łączności podczas importu ADIF oraz niespójność formatu dat/znaków w bazie SQLite.
- **Rozwiązanie minimalne:** Dodać wywołanie `normalize_qso_fields(&mut qso);` na początku pętli w `QsoIngestor::import_batch` (`src/core/ingest/import.rs:64`) przed `qso.validate()`.
- **Rozwiązanie docelowe:** Wspólna funkcja pomocnicza `prepare_qso_for_storage(&mut qso, enrich, &prefix_matcher)` używana zarówno przez `ingest()`, jak i `import_batch()`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Powiązane z `DATA-003`.
- **Wymagany test regresyjny:** `import_batch_infers_band_from_freq_and_normalizes_date_time`.
- **Kryterium akceptacji:** `import_batch` akceptuje rekord z `band: ""` i `freq: Some(14.074)`, zapisując `band == "20m"`, `qso_date == "YYYYMMDD"`, `time_on == "HHMMSS"`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-03
- **Commit:**
  - `9b7f0fc`: `fix(data): make QSO ingestion atomic and idempotent (DATA-002)`
- **Wdrożone rozwiązanie:**
  1. Dodano wywołanie `normalize_qso_fields(&mut qso)` na początku pętli przetwarzania rekordów w `QsoIngestor::import_batch` przed wywołaniem `qso.validate()`.
  2. Zapewniono automatyczną dedukcję pasma na podstawie częstotliwości (`freq`), normalizację wielkości liter oraz czyszczenie formatów dat/czasu w importach wsadowych i synchronizacji LAN Sync.
- **Wdrożone testy regresyjne:**
  - `import_batch_infers_band_from_freq_and_normalizes_date_time` w `src/core/ingest/import.rs`.

---

### `DATA-003` — Nieznormalizowane porównania SQL dla dat/pasm/emisji oraz wyłączenie indeksów B-drzewa przez `UPPER(...)` i `COLLATE NOCASE`
- **ID:** `DATA-003`
- **Tytuł:** Niespójne zapytania SQL w `search.rs`, `qso_store.rs`, `qsl.rs` i `callbook.rs` pomijające normalizację i indeksy SQLite
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/core/database/search.rs`, `src/core/database/qso_store.rs`, `src/core/database/qsl.rs`, `src/core/callbook.rs`
- **Lokalizacja:** [`src/core/database/search.rs:99-110`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/search.rs#L99-L110), [`src/core/database/qso_store.rs:308-329`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qso_store.rs#L308-L329), [`src/core/database/qsl.rs:20-75`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qsl.rs#L20-L75), [`src/core/callbook.rs:333`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/callbook.rs#L333)
- **Dowód z kodu:**
  1. W `search_qsos_advanced` (`search.rs:99-110`) filtr daty używa surowego `qso_date >= ?` bez usunięcia myślników z parametru wejściowego `date_from` (np. wpisanie `2026-01-01` w GUI porównuje tekstowo `'20261018' >= '2026-01-01'`, gdzie `'1' > '-'` zakłamuje zakresy dla lat i miesięcy!).
  2. W `has_identity` (`qso_store.rs:308-329`) zapytanie wykonuje `WHERE journal_id = ?1 AND UPPER(callsign) = ?2`, co uniemożliwia SQLite użycie indeksu `idx_qso_callsign` (pełny skan dziennika przy każdym zapisie QSO!).
  3. W `callbook.rs:333` zapytanie `SELECT ... FROM callbook WHERE Call = ?1 COLLATE NOCASE` nakłada `COLLATE NOCASE` na kolumnę z domyślnym indeksem `BINARY`, wyłączając indeks B-tree na 4,4 MB bazie `callbook.db`.
- **Opis problemu:** Błędne wyniki filtrowania dat z myślnikami oraz degradacja zapytań indeksowanych do pełnych skanów tabeli (`SCAN TABLE`).
- **Scenariusz wystąpienia:** Wyszukiwanie zaawansowane po dacie `YYYY-MM-DD`, masowy import ADIF (tysiące wywołań `has_identity`) lub wpisywanie znaku w formularzu QSO (`callbook.lookup`).
- **Wpływ:** Błędne wyniki wyszukiwania oraz liniowy spadek wydajności importu i autouzupełniania.
- **Rozwiązanie minimalne:**
  1. Normalizować parametry `date_from`/`date_to` w `search_qsos_advanced` przez `.replace('-', "")`.
  2. W `callbook.rs:333` usunąć `COLLATE NOCASE` (znaki w `callbook.db` oraz parametr `clean` są już wielkimi literami).
  3. Dodać migrację normalizującą istniejące wiersze w `qso_records` (`callsign = UPPER(callsign)`, `qso_date = REPLACE(qso_date, '-', '')`) i używać bezpośredniego `WHERE journal_id = ?1 AND callsign = ?2 AND qso_date = ?3`.
- **Rozwiązanie docelowe:** Kanoniczna reprezentacja kolumn klucza tożsamości w `qso_records` ze złożonym indeksem `idx_qso_identity(journal_id, callsign, qso_date, band, mode)`.
- **Ryzyko wdrożenia zmiany:** Niskie (migracja w transakcji przy starcie bazy).
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** `DATA-002`.
- **Wymagany test regresyjny:** `search_qsos_advanced_normalizes_hyphenated_date_bounds` oraz test `EXPLAIN QUERY PLAN` dla `has_identity` i `callbook.lookup`.
- **Kryterium akceptacji:** `EXPLAIN QUERY PLAN` potwierdza `SEARCH ... USING INDEX` dla `has_identity` i `callbook.lookup`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-03 (Bramka Precyzji Tożsamości QSO)
- **Commity:**
  - `1393079`: `fix(data): refine QSO identity precision and incomplete-record policy (DATA-003)`
  - `1e448dd`: `fix(data): database migration 7 for refined QSO identity (DATA-003)`
  - `268e2d2`: `test(data): SQLite concurrency and multi-connection identity tests (DATA-003)`
- **Wdrożone rozwiązanie:**
  1. **Kanoniczny format tożsamości v2:** `v2|{CALL}|{BAND}|{FREQ_HZ}|{MODE}|{SUBMODE}|{DATE}|{TIME_TAG}`.
  2. **Zachowanie sekund i precyzji czasu:** Wprowadzono `QsoTimePrecision::Minute` (`M{HHMM}`) oraz `QsoTimePrecision::Second` (`S{HHMMSS}`). Czas o precyzji minutowej nie dopisuje sztucznych sekund "00", eliminując fałszywe kolizje z łącznościami z sekundy 00.
  3. **Stabilna reprezentacja częstotliwości:** Częstotliwość przechowywana w kluczu w hercach (`u64`), co rozróżnia łączności z tej samej minuty na różnych częstotliwościach pasma.
  4. **Normalizacja par MODE i SUBMODE:** `normalize_mode_and_submode` kanonicznie mapuje FT8/FT4/Q65/WSPR do emisji cyfrowych bez subemisji, USB/LSB do `SSB` z `SUBMODE`, a CW-R/CWR do `CW`.
  5. **Polityka rekordów niepełnych (MODEL B):** Rekordy z brakującym znakiem, pasmem/częstotliwością, emisją, datą lub czasem otrzymują `canonical_key() == None` i są zapisywane w SQLite z `identity_key = NULL`. Zgodnie ze standardem SQL i SQLite UNIQUE index zezwala na dowolną liczbę rekordów z wartością NULL, co chroni niekompletne rekordy przed fałszywą deduplikacją i umożliwia ich ręczną weryfikację.
  6. **Migracja schematu 7 (`SCHEMA_VERSION` 6 -> 7):** W pełni transakcyjna migracja aktualizująca klucze do formatu v2, wykonująca audyt duplikatów przed utworzeniem `CREATE UNIQUE INDEX idx_qso_identity_unique ON qso_records(journal_id, identity_key)`. W razie wykrycia konfliktu następuje pełny ROLLBACK — żadne dane nie są usuwane ani automatycznie scalane, a wersja schematu pozostaje nienaruszona.
- **Wdrożone testy regresyjne:**
  - 104 testy weryfikujące tożsamość i bazę danych w `src/core/database/tests.rs`:
    - `same_callsign_same_minute_different_seconds_are_distinct`
    - `same_callsign_same_minute_different_frequency_are_distinct`
    - `same_callsign_same_minute_different_submode_follow_contract`
    - `minute_precision_and_second_precision_follow_explicit_contract`
    - `multiple_null_identity_keys_follow_documented_contract`
    - `migration_7_upgrades_v1_keys_to_v2_keys_and_preserves_distinct_seconds`
    - `migration_7_conflict_rolls_back_without_data_loss_and_preserves_v6`
    - `concurrent_same_minute_different_seconds_from_two_repositories_both_succeed`
    - `concurrent_same_minute_different_frequencies_from_two_repositories_both_succeed`
    - `concurrent_same_minute_different_submodes_from_two_repositories_both_succeed`
    - `concurrent_update_conflict_from_second_connection_preserves_both_records`
    - `concurrent_identity_stress_five_iterations`

---

### `DATA-004` — Operacje niszczące (usuwanie duplikatów i usuwanie zaznaczonych QSO) działają bez potwierdzenia, bez Undo i w poprzek wszystkich dzienników
- **ID:** `DATA-004`
- **Tytuł:** Nieodwracalne usuwanie QSO bez okna potwierdzenia, bez zapisu na stosie `undo_stack` oraz usuwanie duplikatów z obcych dzienników
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia:** 2026-11-03
- **Commity:** `eff6587` (`fix(data): enforce journal-scoped QSO access (DATA-004 DATA-005)`), `b0afb20` (`fix(data): prevent cross-journal QSO mutations (DATA-004 DATA-005 CONC-001)`), `9157e0e` (`fix(data): isolate async and derived state by journal (DATA-004 DATA-005)`), `f767e01` (`fix(data): add atomic journal-scoped undo history (DATA-004)`)
- **Moduł / obszar:** `src/gui/find_duplicates.rs`, `src/core/database/search.rs`, `src/gui/app_qso.rs`, `src/gui/state/log_view.rs`, `src/gui/logbook_table.rs`
- **Lokalizacja:** [`src/gui/find_duplicates.rs:281-326`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/find_duplicates.rs#L281-L326), [`src/core/database/search.rs:27-81`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/search.rs#L27-L81), [`src/gui/app_qso.rs:564-601`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_qso.rs#L564-L601), [`src/gui/state/log_view.rs:20`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/state/log_view.rs#L20)
- **Dowód z kodu:**
  1. `Database::find_duplicate_qsos` (`src/core/database/search.rs:27-81`) wykonuje `SELECT ... FROM qso_records` **bez warunku `WHERE journal_id = ?`**. Jeżeli ten sam znak został zrobiony w dzienniku `DEFAULT` i w dzienniku `POTA`, okno „Znajdź duplikaty” uznaje je za duplikat i przycisk „Usuń zaznaczone duplikaty” (`find_duplicates.rs:297-318`) bez żadnego potwierdzenia usuwa rekord z bazy, nie dodaje go do `app.entry.undo_stack` i nie wywołuje `app.rebuild_awards_full()`.
  2. `SpLogApp::delete_selected_qso` (`src/gui/app_qso.rs:564-601`) natychmiast usuwa z bazy wszystkie zaznaczone wiersze (`selected_qso_ids`) po naciśnięciu klawisza `Delete`, **nie zapisując usuniętych rekordów na `self.entry.undo_stack`** i nigdy nie używając pola `self.log_view.confirm_bulk_delete` (`src/gui/state/log_view.rs:20` — martwe pole!).
- **Opis problemu:** Przypadkowe naciśnięcie `Delete` lub uruchomienie oczyszczania duplikatów powoduje nieodwracalną utratę łączności (w tym z innych profili dzienników) bez możliwości cofnięcia (`Ctrl+Z`).
- **Scenariusz wystąpienia:** Operator zaznacza kilka łączności i naciska `Delete`, a następnie próbuje użyć `Ctrl+Z` (Undo) — usunięte łączności nie wracają; albo uruchamia „Znajdź duplikaty” posiadając kilka dzienników.
- **Wpływ:** Trwała utrata danych logbooka i desynchronizacja indeksów dyplomowych `AwardsEngine`.
- **Rozwiązanie minimalne:**
  1. Dodać parametr `journal_id: &str` do `Database::find_duplicate_qsos`.
  2. W `delete_selected_qso` oraz `find_duplicates.rs` pobierać usuwane rekordy przed `delete_qsos_batch`, odkładać je na `undo_stack` (rozszerzając `UndoAction::DeleteBatch(Vec<QsoRecord>)`), wymagać potwierdzenia modalnego (`confirm_bulk_delete`) i wywoływać `rebuild_awards_full()`.
- **Rozwiązanie docelowe:** Transakcyjne usuwanie z obowiązkowym wpisem w `undo_stack` i modalem potwierdzenia.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** `DATA-005`.
- **Wymagany test regresyjny:** `find_duplicates_is_isolated_to_active_journal` oraz `delete_selected_qso_pushes_records_to_undo_stack_and_restores_on_undo`.
- **Kryterium akceptacji:** Usunięcie pojedynczego lub wielu QSO pozwala na pełne przywrócenie przez `perform_undo()`, a `find_duplicate_qsos` nie łączy rekordów z różnych `journal_id`.

---

### `DATA-005` — Przeciekanie danych pomiędzy dziennikami (`multi-journal bleed`) w statystykach, synchronizacji chmurowej, QSL, przełączaniu dziennika i kopii ADIF
- **ID:** `DATA-005`
- **Tytuł:** Brak izolacji `active_journal.id` w statystykach, chmurze (LoTW/eQSL/QRZ/ClubLog/HRDLog), projektancie QSL oraz brak przebudowy `AwardsEngine` po zmianie dziennika
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia:** 2026-11-03
- **Commity:** `eff6587` (`fix(data): enforce journal-scoped QSO access (DATA-004 DATA-005)`), `b0afb20` (`fix(data): prevent cross-journal QSO mutations (DATA-004 DATA-005 CONC-001)`), `9157e0e` (`fix(data): isolate async and derived state by journal (DATA-004 DATA-005)`)
- **Moduł / obszar:** `src/core/database/stats.rs`, `src/core/database/qsl.rs`, `src/gui/app_layout.rs`, `src/gui/online_sync.rs`, `src/gui/statistics.rs`, `src/gui/qsl_designer.rs`, `src/gui/app.rs`
- **Lokalizacja:** [`src/gui/app_layout.rs:602-618`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_layout.rs#L602-L618), [`src/core/database/stats.rs:9-144`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/stats.rs#L9-L144), [`src/core/database/qsl.rs:20-27`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qsl.rs#L20-L27), [`src/gui/online_sync.rs:114, 202, 288, 330, 362, 443, 485`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L114-L485), [`src/gui/qsl_designer.rs:98`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/qsl_designer.rs#L98), [`src/gui/app.rs:668`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app.rs#L668)
- **Dowód z kodu:**
  1. Przełączenie aktywnego dziennika w `app_layout.rs:602-618` wywołuje tylko `self.reload_qsos()`, ale **nie wywołuje `self.rebuild_awards_full()`** ani nie przebudowuje `ScpEngine`, więc wskaźniki ATNO / New Band w formularzu nadal pokazują stan poprzedniego dziennika!
  2. `online_sync.rs` wywołuje `db.get_recent_qsos(10000)` (bez filtra `journal_id`) przy wysyłaniu do LoTW (`:114`), eQSL (`:202`), ClubLog (`:288`), QRZ (`:330`), Cloudlog (`:443`) i HRDLog (`:485`), wysyłając łączności ze wszystkich dzienników naraz (np. testowego lub SWL) i obcinając eksport do 10 000 QSO!
  3. `stats.rs:9-144` i `statistics.rs:135-170` zliczają statystyki ze wszystkich dzienników łącznie.
  4. `Drop for SpLogApp` (`src/gui/app.rs:668`) wywołuje `db.get_recent_qsos(5000)`, obcinając automatyczną kopię `SPLogbook_auto_backup.adi` przy zamknięciu programu do 5000 QSO!
- **Opis problemu:** Dzienniki (`JournalProfile`) są odizolowane jedynie w tabeli głównej, natomiast dyplomy, statystyki, chmura i QSL mieszają wszystkie dzienniki i stosują arbitralne limity `5000`/`10000`.
- **Scenariusz wystąpienia:** Użytkownik tworzy osobny dziennik (np. dla innego znaku klubowego lub aktywacji) i uruchamia synchronizację z LoTW/QRZ lub sprawdza statystyki.
- **Wpływ:** Wysłanie łączności z niewłaściwego dziennika/znaku do LoTW/QRZ/ClubLog, przekłamane podpowiedzi ATNO oraz niepełna kopia zapasowa ADIF powyżej 5000 QSO.
- **Rozwiązanie minimalne:**
  1. W `app_layout.rs:610` po zmianie `active_journal` wywoływać `self.rebuild_awards_full()`.
  2. W `online_sync.rs`, `qsl_designer.rs` i `statistics.rs` pobierać QSO przez `db.get_qsos_for_journal(&journal_id, usize::MAX)` zamiast `db.get_recent_qsos(10000)`.
  3. W `Drop for SpLogApp` (`src/gui/app.rs:668`) użyć `db.get_all_qsos()` zamiast `db.get_recent_qsos(5000)`.
- **Rozwiązanie docelowe:** Przekazywanie `journal_id` do wszystkich metod `stats.rs`, `qsl.rs` i `online_sync.rs` oraz zapisywanie ostatnio wybranego `active_journal_id` w `AppConfig`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `switching_journal_rebuilds_awards_engine_and_isolates_stats`.
- **Kryterium akceptacji:** Zmiana dziennika przebudowuje `AwardsEngine`, a eksporty chmurowe i kopia ADIF operują bez sztywnego obcięcia do 5000/10000 rekordów.

---

### `DATA-006` — Niebezpieczny fallback `fs::copy` na żywej bazie SQLite WAL oraz zbyt szerokie usuwanie w `prune_old_backups`
- **ID:** `DATA-006`
- **Tytuł:** Ryzyko uszkodzonej kopii zapasowej przez `fs::copy` na pliku `.db` w trybie WAL i błędny filtr w `prune_old_backups`
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia:** 2026-11-03
- **Commity:** `066db3d` (`fix(storage): create validated atomic SQLite backups (DATA-006)`), `eecf4d3` (`fix(storage): restore backups through validated staging (DATA-006)`)
- **Moduł / obszar:** `src/core/backup.rs`
- **Lokalizacja:** [`src/core/backup.rs:84-89`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/backup.rs#L84-L89), [`src/core/backup.rs:116-144`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/backup.rs#L116-L144)
- **Dowód z kodu:**
  1. W `create_backup_internal` (`src/core/backup.rs:84-89`), gdy `Self::sqlite_online_backup` zwróci błąd, kod przechodzi do `fs::copy(db_path, &backup_path)`. Dla bazy otwartej w trybie `PRAGMA journal_mode = WAL` surowa kopia samego pliku `.db` bez plików `-wal`/`-shm` podczas aktywnej transakcji tworzy niespójny lub niekompletny obraz bazy.
  2. `prune_old_backups` (`src/core/backup.rs:126`) filtruje pliki wyłącznie po `name.contains("_backup_") && name.ends_with(".db")`, ignorując nazwę bazową (`stem`), przez co może usunąć kopie innych baz w tym samym katalogu.
- **Opis problemu:** Ukrycie błędu SQLite Online Backup API poprzez niebezpieczny `fs::copy` oraz brak filtrowania po prefiksie `stem` przy rotacji kopii.
- **Scenariusz wystąpienia:** Chwilowa blokada pliku docelowego podczas `sqlite_online_backup` przy aktywnym zapisie WAL.
- **Wpływ:** Ryzyko powstania uszkodzonej kopii zapasowej uznanej przez program za poprawną (`Ok(backup_path)`).
- **Rozwiązanie minimalne:** Usunąć fallback `fs::copy` (zwracać błąd `Err`, jeśli `rusqlite::backup::Backup` z `busy_timeout` zawiedzie) oraz filtrować w `prune_old_backups` po `name.starts_with(&format!("{stem}_backup_"))`.
- **Rozwiązanie docelowe:** Wykonywanie `sqlite_online_backup` bezpośrednio na istniejącym połączeniu `Database` lub z `busy_timeout(Duration::from_secs(5))`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `prune_old_backups_only_removes_matching_database_stem`.
- **Kryterium akceptacji:** Kopie zapasowe powstają wyłącznie przez transakcyjne API `rusqlite::backup::Backup`, a rotacja nie dotyka plików o innym prefiksie.

---

### `DATA-007` — Martwa tabela `upload_queue` w schemacie SQLite oraz 6 ostrzeżeń leksera w `qso_store.rs`
- **ID:** `DATA-007`
- **Tytuł:** Nieużywana tabela `upload_queue` w `schema.rs` i ostrzeżenia kompilatora `multiple lines skipped by escaped newline` w `qso_store.rs`
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED` (w zakresie 6 ostrzeżeń leksera SQL; wykorzystanie tabeli `upload_queue` przeniesione do kategorii E / Nice-to-Have jako ulepszenie przyrostowej kolejki retry)
- **Data wdrożenia:** 2026-11-01
- **Commit:** `5e61a78` (`fix(database): clean multiline SQL literals (DATA-007)`)
- **Moduł / obszar:** `src/core/database/schema.rs`, `src/core/database/qso_store.rs`
- **Lokalizacja:** [`src/core/database/schema.rs:76-86`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/schema.rs#L76-L86), [`src/core/database/qso_store.rs:208, 252, 254, 278, 310, 312`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qso_store.rs#L208-L315)
- **Dowód z kodu:**
  1. Tabela `upload_queue` jest tworzona w `schema.rs:76-86`, ale w całym repozytorium nie ma ani jednego zapytania `INSERT`, `SELECT` ani `DELETE` odwołującego się do `upload_queue`.
  2. `cargo check` i `cargo test` emitowały 6 ostrzeżeń kompilatora w `src/core/database/qso_store.rs` (linie 208, 252, 254, 278, 310, 312) z powodu pustych linii po znaku kontynuacji `\` w literałach SQL.
- **Opis problemu:** Szum kompilacyjny i niepodłączona tabela kolejki wysyłek chmurowych.
- **Scenariusz wystąpienia:** Każda kompilacja projektu (`cargo check`, `cargo test`, `cargo clippy`).
- **Wpływ:** Zaciemnienie wyjścia kompilatora przez 6 ostrzeżeń leksera; brak trwałej kolejki retry dla nieudanych wysyłek QSL do chmury.
- **Rozwiązanie minimalne:** Usunąć puste linie po `\` w literałach SQL w `src/core/database/qso_store.rs`.
- **Rozwiązanie docelowe:** Podłączyć `upload_queue` do automatycznego ponawiania wysyłek Cloudlog/ClubLog/QRZ lub udokumentować tabelę jako zarezerwowaną pod przyszłą kolejkę offline.
- **Wynik remediacji:**
  - Usunięto 6 pustych linii po znaku kontynuacji `\` w literałach SQL w `qso_numbers_for_journal`, `get_distinct_callsigns`, `existing_identities`, `has_identity` w `src/core/database/qso_store.rs`.
  - Semantyka i tokeny SQL: w 100% identyczne (brak modyfikacji słów kluczowych SQL, tabel, kolumn, klauzul WHERE, JOIN, ORDER BY, LIMIT ani liczby/kolejności parametrów `?1`, `?2`).
  - Ostrzeżenia kompilatora: 0 wyników dla `multiple lines skipped by escaped newline` w `cargo check --workspace --all-targets --all-features`.
  - Wyniki Clippy: spadek liczby ostrzeżeń w `splogbook` (lib) z 345 do 339 (dokładnie o 6) oraz w `splogbook` (lib test) z 422 do 416 (dokładnie o 6).
  - Testy regresyjne: 30/30 testów `core::database` i 35/35 testów `core::ingest` zielonych; formatowanie `cargo fmt --all -- --check` zgodne.
- **Ryzyko wdrożenia zmiany:** Zerowe.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `cargo check --workspace --all-targets` z flagą `RUSTFLAGS="-D warnings"` dla ostrzeżeń kompilatora.
- **Kryterium akceptacji:** Zero ostrzeżeń `multiple lines skipped by escaped newline` podczas kompilacji.

---

## 3. Funkcjonalność i Domena Krótkofalarska (`FUNC-*`)

### `FUNC-001` — Generator Cabrillo 3.0 tworzy niepoprawne logi konkursowe (odwrócona kolejność, stały RST, błędne emisje i obcięta wymiana)
- **ID:** `FUNC-001`
- **Tytuł:** Krytyczne niezgodności eksportera Cabrillo 3.0 (`src/core/cabrillo.rs`) z regulaminami zawodów HF/VHF
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia:** 2026-11-05
- **Commity:**
  - `0c0ca72`: `fix(contest): implement compliant Cabrillo 3.0 generation (FUNC-001)`
  - `e8e360e`: `fix(contest): enforce Cabrillo serial and transmitter policies`
  - `5e8fbc8`: `test(contest): add independent official scoring fixtures`
  - `73cac09`: `fix(contest): prevent invalid Cabrillo transmitter category output (FUNC-001)`
- **Moduł / obszar:** `src/core/cabrillo.rs`, `src/core/contest_rules.rs`, `src/core/contest_stats.rs`, `src/gui/app_export.rs`
- **Lokalizacja:** [`src/core/cabrillo.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/cabrillo.rs), [`src/core/contest_rules.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_rules.rs)
- **Dowód z kodu przed naprawą:**
  1. **Odwrócona kolejność i numeracja QSO (`cabrillo.rs:58`)**: `for (idx, qso) in input.qsos.iter().rev().enumerate()` — brak jawnego sortowania chronologicznego po `(qso_date, time_on)`.
  2. **Ignorowanie raportów `rst_sent` / `rst_rcvd` (`cabrillo.rs:65`)**: Wymuszanie stałych `599`/`59` i ignorowanie rzeczywistych raportów RST.
  3. **Błędne mapowanie emisji (`cabrillo.rs:60-64`)**: Wszystkie łączności cyfrowe i FM mapowane na `PH`.
  4. **Ignorowanie `stx_string` / `srx_string` (`cabrillo.rs:68-73`)**: Wymiana stała `{idx + 1:03}`, ignorowanie strefy CQ, województwa, numerów seryjnych.
  5. **Błędny fallback częstotliwości (`cabrillo.rs:42-55`)**: Brak mapowania dla wielu pasm.
- **Wdrożone rozwiązanie (Etap 7 i Bramka Naprawcza):**
  1. **Format ogólny Cabrillo 3.0**: Nagłówek Cabrillo v3 zawiera `START-OF-LOG: 3.0`, `CALLSIGN:`, `CONTEST:`, `CATEGORY-OPERATOR:`, `CATEGORY-TRANSMITTER:`, `CLAIMED-SCORE: <int>`, `CREATED-BY: SPLogbook`, oraz `END-OF-LOG:`. Zgodność ze specyfikacją Cabrillo v3.0.
  2. **Obsługiwane kody emisji**: `CW` (`CW`), `PH` (`SSB, USB, LSB, AM`), `FM` (`FM`), `RY` (`RTTY`), `DG` (`FT8, FT4, PSK, etc.`). Mapowanie standardowe z walidacją dopuszczalności emisji per profil contestu (`validate_mode`).
  3. **Walidacja specyficzna dla zawodów**: Pasma (`validate_band`), emisje (`validate_mode`) oraz wymiany (`validate_exchange`) są walidowane pod kątem aktywnego `ContestProfile`. Emisje/pasma niedozwolone w danym profilu są odrzucane z typowanym błędem `ContestError::DisallowedBand` / `ContestError::DisallowedMode`.
  4. **Weryfikacja wiersza QSO**: Kolumny `QSO: freq mode date time sent_call sent_rst sent_exch rcvd_call rcvd_rst rcvd_exch [tx_id]`. Częstotliwość podawana w kHz (np. `14000`, `7000`, `3500`, `21000`, `28000`).
  5. **Źródło wymiany kontrolnej**: Priorytet ma rzeczywista zarejestrowana wymiana `stx_string` -> `stx` -> fallback rosnący numer seryjny (`resolve_sent_exchange`). Otrzymana wymiana pobierana z `srx_string` -> `srx` (`resolve_rcvd_exchange`). Brak fabrykowania brakujących wymaganych wymian (województwo w SP DX, strefa w CQ WW są walidowane).
  6. **Polityka RST**: Rzeczywiste raporty `rst_sent` i `rst_rcvd` są bezwzględnie zachowywane (w tym raporty niestandardowe np. `579`, `-10`). Domyślne `599`/`59` stosowane jest wyłącznie, gdy pole raportu jest puste (`cabrillo_uses_default_rst_only_when_missing`). Raporty nieliczbowe lub o błędnej długości są odrzucane (`ContestError::MalformedRst`).
  7. **Polityka numeracji seryjnej**: Kategoria `Single-Op` oraz `Multi-One` używają pojedynczej rosnącej sekwencji seryjnej sprawdzanej przez `validate_cqwpx_serial_policy`.
  8. **Polityka kategorii nadajników i operatorów**:
     - Rozdzielono pojęcia `CATEGORY-OPERATOR` (`CabrilloOperatorCategory`: `SingleOp`, `MultiOp`, `Checklog`) oraz `CATEGORY-TRANSMITTER` (`CabrilloTransmitterCategory`: `One`, `Two`, `Unlimited`).
     - W formacie Cabrillo 3.0 (WWROF) wartość `MULTI` dla linii `CATEGORY-TRANSMITTER:` jest bezwzględnie nieprawidłowa (`MULTI-OP` należy wyłącznie do `CATEGORY-OPERATOR`). Usunięto wadliwy wariant `Multi` z typu kategorii nadajników.
     - Dla stacji z jednym nadajnikiem emitowany jest poprawny nagłówek `CATEGORY-TRANSMITTER: ONE`, a kolumna identyfikatora nadajnika `t` w wierszach `QSO:` jest pomijana zgodnie ze standardem.
     - Kategorie wielonadajnikowe `Two` oraz `Unlimited` są bezpiecznie odrzucane z typowanym błędem `ContestError::UnsupportedContestCategory`, zapobiegając emisji niezgodnych logów bez dedykowanego modelu per-band / multi-tx (planowane w wersji 2.0).
  9. **Zachowanie duplikatów**: Łączności zduplikowane (duplikaty) nie są po cichu usuwane z logu Cabrillo — pozostają w wyjściowym logu zgodnie z regulaminami zawodów CQ/PZK/ARRL (komisja sędziowska punktuje je jako 0 pkt).
  10. **CLAIMED-SCORE**: Wartość jest bezpośrednio powiązana z wynikiem punktowym `Scoreboard::total` z `ContestProfile` i formatowana jako czysta liczba całkowita ASCII bez separatorów tysięcy (`claimed_score_is_integer_without_formatting`).
- **Ryzyko szczątkowe:** Brak fizycznej obsługi logowania z wielu nadajników na różnych pasmach w v1.x (zabezpieczone odmową eksportu `UnsupportedContestCategory`).
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Powiązane z `FUNC-002`.
- **Wdrożone testy regresyjne:**
  - `cabrillo_header_and_qso_lines_format`
  - `cabrillo_claimed_score_equals_scoreboard_total`
  - `cabrillo_never_emits_category_transmitter_multi`
  - `cabrillo_rejects_unsupported_two_and_unlimited_transmitters`
  - `cabrillo_chronological_sorting_and_serial_assignment`
  - `cabrillo_export_preserves_rst_digital_modes_and_custom_contest_exchanges`
  - `cabrillo_preserves_non_default_rst`
  - `cabrillo_uses_default_rst_only_when_missing`
  - `cabrillo_rejects_or_reports_malformed_rst_by_contract`
  - `cabrillo_transmitter_id_is_emitted_only_when_required`
  - `cqwpx_single_transmitter_uses_single_serial_sequence`
  - `cqwpx_multi_one_uses_contract_serial_sequence`
  - `cqwpx_multi_two_requires_transmitter_and_band_sequences`
  - `cqwpx_unsupported_multi_category_is_rejected`
  - `duplicate_qso_remains_in_cabrillo_log`
  - `duplicate_qso_scores_according_to_contest_profile`
  - `zero_point_qso_remains_in_log_when_required`
  - `unscored_qso_is_not_silently_deleted`
  - `spdx_duplicate_remains_in_cabrillo`
- **Kryterium akceptacji:** Spełnione w całości — format Cabrillo 3.0, kody emisji, zachowanie RST, obsługa wymian, monotoniczne serie numerów, obsługa kategorii nadajnika i zachowanie duplikatów zweryfikowane pakietem testów automatycznych.

---

### `FUNC-002` — Błędne reguły punktacji i mnożników w `src/core/contest_rules.rs` (SP DX Contest, CQ WW, CQ WPX)
- **ID:** `FUNC-002`
- **Tytuł:** Błędna punktacja łączności zagranicznych oraz zliczanie polskich stacji jako mnożników DXCC w SP DX Contest
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia:** 2026-11-05
- **Commity:**
  - `29a4e36`: `fix(contest): refine scoring and multipliers for SP DX, CQ WW and CQ WPX (FUNC-002)`
  - `9a6de52`: `fix(contest): correct CQ WPX CW and SSB scoring (FUNC-002)`
  - `966e59f`: `fix(contest): separate CQ WPX and CQ WPX RTTY rule profiles (FUNC-002)`
  - `31d588f`: `fix(contest): validate contest-specific bands modes and exchanges`
  - `5e8fbc8`: `test(contest): add independent official scoring fixtures`
  - `ed94f6f`: `fix(contest): correct CQ WW official scoring fixture (FUNC-002)`
  - `df5c68b`: `fix(contest): enforce official SP DX province exchanges (FUNC-002)`
- **Moduł / obszar:** `src/core/contest_rules.rs`, `src/core/contest_stats.rs`, `src/core/cabrillo.rs`
- **Lokalizacja:** [`src/core/contest_rules.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_rules.rs), [`src/core/contest_stats.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_stats.rs)
- **Dowód z kodu przed naprawą:**
  1. `score_sp_dx`: Brak prawidłowego rozróżnienia stacji polskiej vs zagranicznej oraz błędna walidacja 16 województw w wymianie.
  2. `score_cqww`: Uproszczone prefiksy zamiast porównania encji DXCC i kontynentów.
  3. `score_wpx`: Błędna reguła podwajania punktów dla tego samego kraju na pasmach niskich w zawodach CW/SSB (przyznawano 2 punkty zamiast 1 punktu).
  4. Brak rozdzielenia profili CQ WPX CW/SSB od CQ WPX RTTY.
- **Wdrożone rozwiązanie (Etap 7 i Bramka Naprawcza):**
  1. **Edycja regulaminu SP DX Contest (PZK)**:
     - Stacja polska (`my_dxcc == 269`): praca z Europą = 1 pkt, praca poza Europą = 3 pkt, łączność ze stacją SP = 0 pkt (duplikat/brak punktów). Mnożnik: unikalne podmioty DXCC na każdym paśmie.
     - Stacja zagraniczna (`my_dxcc != 269`): punktowane są wyłącznie łączności ze stacjami SP = 3 pkt. Mnożnik: dokładnie 16 oficjalnych, jednoliterowych oznaczeń polskich województw (`B, C, D, F, G, J, K, L, M, O, P, R, S, U, W, Z`) na każdym paśmie.
     - Pełna kanoniczna mapa województw SP DX:
       | Kod | Województwo |
       |---|---|
       | B | lubuskie |
       | C | łódzkie |
       | D | dolnośląskie |
       | F | pomorskie |
       | G | śląskie |
       | J | warmińsko-mazurskie |
       | K | podkarpackie |
       | L | lubelskie |
       | M | małopolskie |
       | O | podlaskie |
       | P | kujawsko-pomorskie |
       | R | mazowieckie |
       | S | świętokrzyskie |
       | U | opolskie |
       | W | wielkopolskie |
       | Z | zachodniopomorskie |
     - Walidacja województw: kody są normalizowane do wielkiej litery (case-insensitive). Niedozwolone są jakiekolwiek dwuliterowe skróty administracyjne (np. `MA`, `LD`, `DS`, `PM`, `SL`), nieoficjalne litery, cyfry, znaki interpunkcyjne lub wartości wieloznakowe — zwracają `ContestError::InvalidContestExchange`. Pusta wymiana zwraca `ContestError::MissingContestExchange`.
     - Poprawiono manualne fixture stacji zagranicznej:
       - SP1AA, 80m CW: exchange R (województwo: mazowieckie), punkty: 3, mnożnik: R na 80m.
       - SP2AA, 40m CW: exchange C (województwo: łódzkie), punkty: 3, mnożnik: C na 40m.
       - DL2AA, 20m CW: exchange 001, stacja spoza SP, punkty: 0, mnożnik: brak.
       - SP3AA, 20m CW: exchange R (województwo: mazowieckie), punkty: 3, mnożnik: R na 20m.
       Podsumowanie punktacji stacji zagranicznej:
       Punkty: 3 + 3 + 0 + 3 = 9
       Mnożniki: R na 80m, C na 40m, R na 20m (liczba mnożników: 3).
       Wynik końcowy: 9 × 3 = 27 (CLAIMED-SCORE: 27).
     - Fixture stacji polskiej: SP1AA, województwo R (mazowieckie). Oficjalne przypisanie kodów: R = mazowieckie, C = łódzkie, K = podkarpackie, P = kujawsko-pomorskie. Niedozwolone są jakiekolwiek dwuliterowe kody województw.
     - Dozwolone pasma: 160m, 80m, 40m, 20m, 15m, 10m. Dozwolone emisje: CW, SSB.
  2. **Edycja regulaminu CQ WW DX Contest (CQ)**:
     - Własny podmiot DXCC: 0 pkt (kwalifikuje się do mnożnika strefy/kraju).
     - Ten sam kontynent, inny podmiot: 1 pkt (wyjątek Ameryki Północnej: 2 pkt).
     - Inny kontynent: 3 punkty bez względu na pasmo (zasada obowiązuje jednakowo na wszystkich dozwolonych pasmach: 160m, 80m, 40m, 20m, 15m i 10m).
     - CQ WW DX Contest — ręcznie policzone fixture:
       Stacja bazowa: SP1AA (DXCC: SP, kontynent: EU, strefa CQ: 15).
       QSO:
       1. SP2AA, 80m: relacja: ten sam kraj; punkty: 0; mnożniki: Zone 15 na 80m, SP na 80m.
       2. DL1AA, 80m: relacja: ten sam kontynent, inny kraj; punkty: 1; mnożniki: Zone 14 na 80m, DL na 80m.
       3. W1AW, 20m: relacja: inny kontynent; punkty: 3; mnożniki: Zone 5 na 20m, K na 20m.
       4. JA1AA, 20m: relacja: inny kontynent; punkty: 3; mnożniki: Zone 25 na 20m, JA na 20m.
       5. W2AA, 40m: relacja: inny kontynent; punkty: 3; mnożniki: Zone 5 na 40m, K na 40m.
       Podsumowanie punktacji:
       QSO points: 0 + 1 + 3 + 3 + 3 = 10
       Zone multipliers: 5
       Country multipliers: 5
       All multipliers: 5 + 5 = 10
       Final score: 10 × 10 = 100
       CLAIMED-SCORE: 100
       (Zone 5 i kraj K na 20m oraz Zone 5 i kraj K na 40m są osobnymi mnożnikami, ponieważ mnożniki CQ WW liczone są oddzielnie na każdym paśmie).
     - Obsługa łączności zduplikowanych (DUPLICATE QSOS):
       - pozostają w logu Cabrillo, jeśli wymaga tego kontrakt eksportu;
       - nie zwiększają liczby punktów (0 dodatkowych punktów);
       - nie zwiększają liczby mnożników (0 nowych mnożników);
       - nie zmieniają wyniku końcowego (pozostaje 100 punktów).
       Dla pięciu QSO podstawowych oraz ich duplikatów: podstawowe QSO dają 10 punktów i 10 mnożników, duplikaty dają 0 dodatkowych punktów i 0 nowych mnożników, wynik wynosi 100. Duplikaty nie mogą być używane do uzasadniania 20 punktów QSO.
     - Mnożniki: CQ Zones (1..40) na każdym paśmie oraz kraje DXCC na każdym paśmie.
     - Emisje rozdzielone per profil: osobny profil CQ WW CW, osobny profil CQ WW SSB. Dozwolone pasma: 160m, 80m, 40m, 20m, 15m, 10m.
  3. **Edycja regulaminu CQ WPX CW/SSB (CQ)**:
     - **Ten sam kraj (same-country): 1 punkt na każdym paśmie** (brak podwojenia na pasmach niskich!).
     - Ten sam kontynent, inny kraj: 1 pkt pasma wysokie (10m, 15m, 20m), 2 pkt pasma niskie (40m, 80m, 160m).
     - Ameryka Północna inny kraj (NA exception): 2 pkt pasma wysokie (10m, 15m, 20m), 4 pkt pasma niskie (40m, 80m, 160m).
     - Inny kontynent: 3 pkt pasma wysokie (10m, 15m, 20m), 6 pkt pasma niskie (40m, 80m, 160m).
     - Mnożnik prefiksów WPX: unikalny prefiks zaliczany **dokładnie raz w całych zawodach** (nie per pasmo).
     - Dozwolone pasma: 160m, 80m, 40m, 20m, 15m, 10m.
  4. **Status CQ WPX RTTY i CQ WW RTTY**:
     - Profile RTTY są bezwzględnie odseparowane w typie `ContestId::CqWpxRtty` oraz `ContestId::CqWwRtty`.
     - Próba kalkulacji lub eksportu zwraca jawny, typowany błąd `ContestError::UnsupportedContestVariant`. Wykluczone jest jakiekolwiek niejawne korzystanie z reguł CW/SSB.
  5. **Walidacja specyficzna dla zawodów**:
     - Pasma, emisje i wymiany podlegają ścisłej walidacji przed punktacją i eksportem. Łączności poza pasmami zawodów lub w niedozwolonych emisjach nie są punktowane i powodują zgłoszenie błędu.
  6. **Klucze unikalności duplikatów (dedup key)**:
     - SP DX / CQ WW: znak + pasmo + emisja (w zawodach wieloemisyjnych jak SP DX dozwolona jest łączność na tym samym paśmie na CW i na SSB).
     - CQ WPX: znak + pasmo (w zawodach WPX dana stacja może być zrobiona tylko raz na paśmie).
  7. **Niezależne fixtures testowe**:
     - Wprowadzono niezależne testy weryfikujące reguły punktacji z ręcznie przeliczonymi wartościami z oficjalnych regulaminów PZK i CQ, weryfikujące `Scoreboard` oraz format i wartość pola `CLAIMED-SCORE`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Powiązane z `FUNC-001`.
- **Wdrożone testy regresyjne:**
  - `sp_dx_contest_scoring_for_sp_and_non_sp_stations_matches_pzk_rules`
  - `spdx_polish_exchange_requires_voivodeship`
  - `spdx_foreign_exchange_requires_serial`
  - `spdx_mixed_duplicate_key_includes_mode`
  - `spdx_rejects_vhf_band_for_scoring`
  - `spdx_rejects_digital_mode_for_scoring`
  - `cq_ww_contest_scoring_matches_cq_rules`
  - `cqww_cw_requires_cw_and_zone_exchange`
  - `cqww_ssb_requires_phone_and_zone_exchange`
  - `cqww_country_multiplier_is_per_band`
  - `cqww_zone_multiplier_is_per_band`
  - `cqww_same_country_zero_points_but_multiplier_eligible`
  - `cqwpx_same_country_high_band_scores_one`
  - `cqwpx_same_country_low_band_scores_one`
  - `cqwpx_same_continent_high_band_scores_one`
  - `cqwpx_same_continent_low_band_scores_two`
  - `cqwpx_north_america_high_band_scores_two`
  - `cqwpx_north_america_low_band_scores_four`
  - `cqwpx_different_continent_high_band_scores_three`
  - `cqwpx_different_continent_low_band_scores_six`
  - `cqwpx_prefix_multiplier_counts_once_across_bands`
  - `cqwpx_rtty_does_not_use_cw_ssb_rules`
  - `cqww_rtty_does_not_use_cw_ssb_rules_if_supported`
  - `scoreboard_matches_manual_spdx_fixture`
  - `scoreboard_matches_manual_cqww_fixture`
  - `scoreboard_matches_manual_cqwpx_fixture`
  - `claimed_score_matches_manual_fixture`
  - `claimed_score_is_integer_without_formatting`
  - `cqww_different_continent_scores_three_on_40m`
  - `cqww_different_continent_scores_three_on_all_allowed_bands`
  - `claimed_score_matches_corrected_manual_cqww_fixture`
  - `spdx_accepts_each_official_one_letter_voivodeship`
  - `spdx_rejects_ma_as_voivodeship`
  - `spdx_rejects_ld_as_voivodeship`
  - `spdx_rejects_arbitrary_single_letter`
  - `spdx_rejects_digit_or_punctuation_voivodeship`
  - `spdx_empty_voivodeship_reports_missing_exchange`
  - `spdx_invalid_voivodeship_reports_invalid_exchange`
  - `claimed_score_matches_corrected_manual_spdx_fixture`
- **Kryterium akceptacji:** Spełnione — reguły SP DX, CQ WW, CQ WPX CW/SSB, odseparowanie RTTY, walidacja emisji i pasm oraz niezależne fixtures są w 100% zweryfikowane testami automatycznymi.

---

### `FUNC-003` — Endpoint `POST /api/v1/rig` w REST API jest atrapą (mutuje tylko RAM i jest nadpisywany w kolejnej klatce GUI)
- **ID:** `FUNC-003`
- **Tytuł:** `post_rig_control` (`POST /api/v1/rig`) nie wysyła żadnych rozkazów do `CatRuntime` ani `RigHub`
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-05
- **Commity:** `36d1dee` (`feat(api): wire REST rig control through CatRuntime and add multi-rig endpoints (FUNC-003)`), `833f28d` (`fix(api): harden REST rig control security boundary (FUNC-003)`)
- **Moduł / obszar:** `src/api/server.rs`, `src/gui/app_async.rs`
- **Lokalizacja:** [`src/api/server.rs:1088-1141`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/api/server.rs#L1088-L1141), [`src/gui/app_async.rs:248-252`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_async.rs#L248-L252)
- **Dowód z kodu:** Funkcja `post_rig_control` (`src/api/server.rs:1092-1116`) zapisuje przysłane pola (`frequency_hz`, `mode`, `ptt` itd.) wyłącznie do lokalnego `state.rig_state.write()` (`Arc<RwLock<RigState>>`) i zwraca `200 OK`. `ApiState` (`src/api/server.rs:35-45`) nie posiada nawet uchwytu do `CatRuntime`! Co więcej, w następnej klatce GUI (~16 ms później) `SpLogApp::poll_cat_outcomes` (`src/gui/app_async.rs:248-252`) nadpisuje `shared_rig_state` z powrotem stanem z `self.cat.rig_state`.
- **Opis problemu:** Zewnętrzne integracje wywołujące `POST /api/v1/rig` otrzymują fałszywą odpowiedź o zmianie częstotliwości/trybu/PTT, podczas gdy fizyczne radio nie otrzymuje żadnej komendy.
- **Scenariusz wystąpienia:** Skrypt zewnętrzny lub aplikacja kliencka wysyła `POST /api/v1/rig {"frequency_hz": 14074000, "mode": "FT8"}`.
- **Wpływ:** Całkowity brak działania zdalnego sterowania radiem przez REST API.
- **Rozwiązanie minimalne:** Dodać `cat_runtime: CatRuntime` do `ApiState` i w `post_rig_control` wysyłać odpowiednie komendy `RigCommand` (`SetFrequency`, `SetMode`, `SetPassband`, `SetVfo`, `SetSplit`) oraz `request_tx`/`release_tx` dla `ptt`.
- **Rozwiązanie docelowe:** Synchroniczne potwierdzenie wykonania komendy z `CatRuntime` przed zwróceniem `RigStateResponse`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Powiązane z `CAT-008`.
- **Wymagany test regresyjny:** `api_post_rig_control_dispatches_commands_to_cat_runtime`.
- **Kryterium akceptacji:** Wywołanie `POST /api/v1/rig` przekazuje rozkazy do `CatRuntime` / `RigHub` i przestrzega blokad `TxArbiter`.

---

### `FUNC-004` — Funkcje atrapowe, częściowo podłączone lub nieosiągalne w GUI i backendzie (CW/WinKeyer, JS8Call, FLDigi, Audio Stop, EME Rotor, Photo Viewer)
- **ID:** `FUNC-004`
- **Tytuł:** Zbiór funkcji pozornych (sham/partially wired) widocznych w interfejsie użytkownika
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/winkeyer.rs`, `src/gui/cw_terminal.rs`, `src/gui/app_plugins.rs`, `src/digital/js8call.rs`, `src/digital/fldigi.rs`, `src/gui/qso_entry.rs`, `src/gui/astronomy_dialog.rs`
- **Lokalizacja:**
  1. **Terminal CW i Makra CW**: [`src/gui/cw_terminal.rs:78-88, 169-174`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/cw_terminal.rs#L78-L174), [`src/gui/app_plugins.rs:215-225`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_plugins.rs#L215-L225), [`src/cat/winkeyer.rs:37-118`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/winkeyer.rs#L37-L118).
  2. **JS8Call Listener**: [`src/gui/app.rs:495`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app.rs#L495), [`src/gui/welcome_wizard.rs:382`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/welcome_wizard.rs#L382).
  3. **FLDigi XML-RPC**: [`src/digital/fldigi.rs:58-128`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/digital/fldigi.rs#L58-L128), [`src/gui/online_sync.rs:543-580`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L543-L580).
  4. **Przycisk STOP nagrywania audio**: [`src/gui/qso_entry.rs:646`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/qso_entry.rs#L646).
  5. **Przyciski rotatora EME (Księżyc / Słońce)**: [`src/gui/astronomy_dialog.rs:233-236`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/astronomy_dialog.rs#L233-L236).
  6. **Podgląd zdjęcia QSL z formularza**: [`src/gui/qso_entry.rs:589`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/qso_entry.rs#L589).
- **Dowód z kodu:**
  1. `WinkeyerProtocol` w `src/cat/winkeyer.rs` nigdy nie jest wywoływany poza testem jednostkowym; w `cw_terminal.rs:78` przycisk „Połącz” przełącza tylko `self.is_connected = !self.is_connected` bez otwierania portu COM, a `transmit_cw_macro` (`app_plugins.rs:215`) ustawia tylko tekst na pasku statusu.
  2. `js8call_enabled` jest domyślnie `false` w `DigitalState::new()` (`src/gui/state/digital.rs:34`), nie istnieje w `AppConfig`, a zaznaczenie checkboxa w `welcome_wizard.rs:382` następuje już po wykonaniu `app.start_background()` (`src/gui/app.rs:495`), więc `Js8CallListener::start` **nigdy nie może zostać uruchomiony w działającym programie**.
  3. W `FldigiClient` (`src/digital/fldigi.rs:58-128`) metody `get_frequency`, `get_mode`, `get_qso_data` nie są nigdzie wywoływane, a przycisk testu w `online_sync.rs:568-575` spawnuje zadanie, którego wynik jest porzucany (stan zawsze pozostaje `OFFLINE`).
  4. W `src/gui/qso_entry.rs:646` podczas trwającego nagrywania (`is_recording == true`) kliknięcie przycisku `⏹ STOP` wywołuje `crate::media::audio::AudioRecorder::stop_audio()` (zatrzymanie odtwarzania) zamiast `app.entry.audio_recorder.stop_recording()`, przez co **nagrywania nie da się zatrzymać przyciskiem STOP**!
  5. W `src/gui/astronomy_dialog.rs:233-236` przyciski „Ustaw antenę na Księżyc” i „Ustaw antenę na Słońce” przypisują tylko `app.cat.rotor_state.azimuth = ...` w RAM bez wywołania `app.rotate_antenna_to_el(az, el)`.
  6. W `src/gui/qso_entry.rs:589` kliknięcie „Pokaż kartę QSL” wywołuje `photo_viewer_dialog.open(&call, None)` (zawsze `None`), mimo że powyżej sprawdzono istnienie zdjęcia w `qsl_gallery`.
- **Opis problemu:** Sześć widocznych w GUI funkcji nie wykonuje zamierzonego działania sprzętowego lub sieciowego.
- **Scenariusz wystąpienia:** Użycie dowolnej z wymienionych funkcji przez operatora.
- **Wpływ:** Wprowadzenie użytkownika w błąd (pozorne działanie CW/rotatora EME/JS8Call/FLDigi) oraz niemożność zatrzymania nagrywania audio QSO.
- **Rozwiązanie minimalne:**
  1. W `qso_entry.rs:646` zamienić `AudioRecorder::stop_audio()` na `app.entry.audio_recorder.stop_recording()`.
  2. W `qso_entry.rs:589` przekazać ścieżkę `app.entry.qsl_gallery.get_photos_for_call(&call).first().cloned()` zamiast `None`.
  3. W `astronomy_dialog.rs:233-236` wywołać `app.rotate_antenna_to_el(az, el)`.
  4. Dodać `js8call_enabled` i `js8call_port` do `AppConfig` i uruchamiać/zatrzymywać `Js8CallListener` dynamicznie.
  5. Podłączyć `online_sync_tx` do testu FLDigi w `online_sync.rs:568`.
  6. Dodać obsługę portu szeregowego `tokio-serial` dla `WinkeyerProtocol` (lub wyraźnie oznaczyć Terminal CW jako symulator do czasu podłączenia portu).
- **Rozwiązanie docelowe:** Pełna integracja sterownika szeregowego WinKeyer i pollera FLDigi.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** Testy jednostkowe dla zatrzymania nagrywania `stop_recording()`, przekazania ścieżki do `PhotoViewerDialog::open` i uruchomienia `Js8CallListener`.
- **Kryterium akceptacji:** Każda z 6 funkcji wykonuje pełny przepływ od akcji UI do efektu I/O i odczytu stanu.

---

### `FUNC-005` — Utrata części ustawień po restarcie programu z powodu niekompletnego `SpLogApp::snapshot_config()`
- **ID:** `FUNC-005`
- **Tytuł:** `SpLogApp::snapshot_config()` pomija zapis konfiguracji CW, REST API, alertów solarnych, satelitów, motywu z kreatora i aktywnego dziennika
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/app_config.rs`, `src/gui/welcome_wizard.rs`, `src/gui/workspace.rs`
- **Lokalizacja:** [`src/gui/app_config.rs:28-59`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_config.rs#L28-L59), [`src/gui/welcome_wizard.rs:475-490`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/welcome_wizard.rs#L475-L490), [`src/gui/workspace.rs:146-164`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/workspace.rs#L146-L164)
- **Dowód z kodu:**
  1. `SpLogApp::snapshot_config()` (`src/gui/app_config.rs:28-59`) kopiuje wybrane pola do `cfg`, ale **nigdy nie kopiuje** zmian w:
     - `cfg.cw` (`wpm`, `macro_f1`..`macro_f8`),
     - `cfg.runtime.rest_api_enabled`, `cfg.runtime.rest_api_port`, `cfg.runtime.rest_api_token`,
     - `cfg.solar` (`band_alert_enabled`, `band_alert_min_Watts`, `band_alert_modes`),
     - `cfg.satellites`,
     - `cfg.ui.show_operator_assistant`.
  2. W `welcome_wizard.rs:478` i `workspace.rs:150` zmiana motywu ustawia `app.gui.theme.preset = ...` i wywołuje `app.save_config_async()`, ale `snapshot_config()` czyta `self.config.ui.theme_preset` (które nie zostało zaktualizowane!), więc wybrany w kreatorze motyw przepada po restarcie!
- **Opis problemu:** Zmiany wprowadzone przez użytkownika w powyższych panelach działają tylko do zamknięcia programu.
- **Scenariusz wystąpienia:** Użytkownik zmienia motyw w kreatorze startowym lub konfiguruje makra CW / port REST API i restartuje program.
- **Wpływ:** Frustrująca utrata skonfigurowanych ustawień po każdym restarcie aplikacji.
- **Rozwiązanie minimalne:** Uzupełnić `SpLogApp::snapshot_config()` o synchronizację `cfg.ui.theme_preset = self.gui.theme.preset.as_str().to_string()`, `cfg.ui.show_operator_assistant = self.gui.windows.show_operator_assistant` oraz wszystkich pól `cw`, `runtime`, `solar` i `satellites`.
- **Rozwiązanie docelowe:** Test round-trip sprawdzający zapis i odczyt wszystkich sekcji `AppConfig` po mutacji stanu `SpLogApp`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`.
- **Kryterium akceptacji:** Zmiana dowolnego ustawienia w GUI i wywołanie `snapshot_config()` zachowuje nową wartość po ponownym wczytaniu `AppConfig`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `acecd8c`: `fix(config): make application settings durable and atomic`
  - `c4cf893`: `fix(config): migrate and preserve multi-radio settings`
  - `d564272`: `fix(config): redact REST token and harden secret handling (FUNC-005)`
- **Wdrożone rozwiązanie:**
  1. W `SpLogApp::snapshot_config()` dodano kompletną synchronizację wszystkich struktur podrzędnych `AppConfig`: makra CW (`macro_f1`..`macro_f8`, `wpm`), ustawienia serwera REST API (`rest_api_enabled`, `rest_api_port`, `rest_api_token`), alerty pasmowe solarne (`band_alert_enabled`, `band_alert_min_Watts`, `band_alert_modes`), śledzenie satelitów, stan widoczności okna Asystenta Operatora (`show_operator_assistant`), aktywny motyw (`theme_preset`) oraz konfiguracje profili radiowych Radio 1 i Radio 2.
  2. Wprowadzono walidację poprawności schematu przed zapisem (`AppConfig::validate`), uniemożliwiającą serializację i zapis uszkodzonych wartości.
  3. Zastosowano atomowy zapis pliku konfiguracyjnego `config.json` z wykorzystaniem unikalnego pliku tymczasowego `.tmp` i atomowej zamiany `rename_atomic`, z zachowaniem kopii zapasowej `.bak`. W razie awarii I/O lub błędu walidacji pierwotny plik konfiguracyjny pozostaje w 100% nienaruszony.
  4. Dodano migrację starszych konfiguracji jednoukładowych do formatu z profilem multi-radio bez utraty parametrów łączności.
- **Wdrożone testy regresyjne:**
  - `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`
  - `config_atomic_write_preserves_previous_file_on_write_failure`
  - `multi_radio_settings_are_preserved_across_save_and_reload`
  - `legacy_single_radio_config_migrates_cleanly_to_multi_radio`
  - `rest_api_token_is_masked_in_gui_configuration`
  - `rest_api_token_is_redacted_in_logs_and_debug_output`

---

### `FUNC-006` — `ClubRegistry::check` w `src/core/clubs.rs` jest zahardkodowaną makietą dla 11 znaków wywoławczych
- **ID:** `FUNC-006`
- **Tytuł:** Pozorna baza klubów krótkofalarskich (`ClubRegistry::check`) obsługująca tylko 11 zahardkodowanych znaków ze współdzielonymi numerami członkowskimi
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/core/clubs.rs`, `README.md`
- **Lokalizacja:** [`src/core/clubs.rs:24-84`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/clubs.rs#L24-L84), [`README.md:176-184`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/README.md#L176-L184)
- **Dowód z kodu:** W `src/core/clubs.rs:36-81` metoda `ClubRegistry::check` sprawdza wyłącznie 11 zahardkodowanych literałów:
  - `"SP6ZDA" | "SP5PZK" | "SP2FAX" | "SP1PBW"` -> wszystkim czterem przypisuje ten sam numer `SP-OTC #248`;
  - `"SP6PAZ" | "SP2FAP"` -> obu przypisuje `SPCWC #104`;
  - `"W1AW" | "K1TTT" | "N1MM"` -> wszystkim trzem przypisuje `CWOPS #001` i `FOC #1850`;
  - `"DL1ABC" | "DK1MAX"` -> obu przypisuje `HSC #1940`.
  Kluby SKCC i EPC wymienione w nagłówku pliku (`clubs.rs:5-6`) i w `README.md:181` w ogóle nie występują w kodzie!
- **Opis problemu:** Funkcja reklamowana w `README.md` jako „wbudowana baza prestiżowych międzynarodowych i krajowych klubów” jest w rzeczywistości statyczną zaślepką demonstracyjną dla 11 znaków z fikcyjnymi (zduplikowanymi) numerami członkowskimi.
- **Scenariusz wystąpienia:** Wpisanie znaku w formularzu QSO lub wywołanie `GET /api/v1/lookup/{call}`.
- **Wpływ:** Wyświetlanie nieprawdziwych numerów członkowskich dla 11 znaków i całkowity brak rozpoznawania rzeczywistych członków klubów.
- **Rozwiązanie minimalne:** Wczytywać listę członków z zewnętrznego pliku referencyjnego `databases/clubs.csv` (z pustą listą domyślną zamiast fałszywych współdzielonych numerów) oraz skorygować opis w `README.md`.
- **Rozwiązanie docelowe:** Importer list członkowskich CWOPS / SKCC / SP-OTC / SPCWC do tabeli w `callbook.db` lub pliku `databases/clubs.csv`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `DOC-001`.
- **Wymagany test regresyjny:** `club_registry_does_not_assign_duplicate_hardcoded_numbers`.
- **Kryterium akceptacji:** `ClubRegistry` zwraca rzeczywiste dane z pliku referencyjnego i nie przypisuje tego samego numeru członkowskiego różnym znakom.

---

### `FUNC-007` — Nieużywane pola dyplomowe `worked_wwff` i `worked_rda` w `AwardsEngine` oraz martwa historia propagacji w GUI
- **ID:** `FUNC-007`
- **Tytuł:** Martwe kolekcje `worked_wwff`/`worked_rda` w `AwardsEngine` oraz brak prezentacji `PropagationHistory` w GUI
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/core/awards.rs`, `src/core/propagation_history.rs`, `src/core/qsl_print.rs`
- **Lokalizacja:** [`src/core/awards.rs:113-116, 442-446`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/awards.rs#L113-L446), [`src/core/propagation_history.rs:1-190`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/propagation_history.rs#L1-L190), [`src/core/qsl_print.rs:1-130`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/qsl_print.rs#L1-L130)
- **Dowód z kodu:**
  1. W `AwardsEngine` (`src/core/awards.rs:113-116`) zdefiniowano `pub worked_wwff: HashSet<String>` i `pub worked_rda: HashSet<String>`, które są czyszczone w `clear()`, ale nigdy nie są wypełniane w `ingest_qso()` (`awards.rs:138-262`) ani sprawdzane w `awards_dialog.rs`.
  2. `PropagationHistory` (`src/core/propagation_history.rs`) zbiera rekordy przy zapisie QSO (`app_qso.rs:165`) i aktualizacji solarnej (`app_solar.rs:60`) oraz zapisuje je do `splogbook_config.json`, lecz metody `accuracy_rate()` i `band_accuracy()` nie są nigdzie wywoływane w GUI (dane wyłącznie zapisywane, nigdy nie wyświetlane).
  3. Moduł `src/core/qsl_print.rs` nie ma ani jednego wywołania poza własnymi testami jednostkowymi.
- **Opis problemu:** Nieukończone podłączenie śledzenia dyplomów WWFF/RDA oraz statystyk sprawdzalności prognoz propagacji w GUI.
- **Scenariusz wystąpienia:** Zalogowanie łączności z referencją WWFF (`qso.wwff_ref`) — brak aktualizacji `worked_wwff`.
- **Wpływ:** Brak zakładki/licznika WWFF w oknie dyplomów mimo posiadania pola `wwff_ref` w `QsoRecord` oraz rozrastanie się `splogbook_config.json` o niewyświetlane rekordy `propagation_history`.
- **Rozwiązanie minimalne:** Dodać obsługę `qso.wwff_ref` w `AwardsEngine::ingest_qso` oraz wyświetlić wskaźnik `propagation_history.accuracy_rate()` w oknie propagacji (`propagation_dialog.rs`).
- **Rozwiązanie docelowe:** Zakładka WWFF w `awards_dialog.rs` oraz integracja `qsl_print.rs` z `qsl_designer.rs`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `awards_engine_ingests_and_counts_wwff_references`.
- **Kryterium akceptacji:** `AwardsEngine::ingest_qso` rejestruje `wwff_ref`, a `propagation_dialog.rs` prezentuje skuteczność prognoz.

---

### `FUNC-008` — `LotwClient::fetch_confirmations` nie przesyła parametru `qso_qslsince` do serwera ARRL LoTW
- **ID:** `FUNC-008`
- **Tytuł:** Nieużywany parametr `_since_date` w `LotwClient::fetch_confirmations` powoduje pobieranie pełnej historii LoTW przy każdej synchronizacji
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cloud/lotw.rs`
- **Lokalizacja:** [`src/cloud/lotw.rs:163-179`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/lotw.rs#L163-L179)
- **Dowód z kodu:** Sygnatura `pub async fn fetch_confirmations(&self, _since_date: Option<&str>)` (`src/cloud/lotw.rs:166`) ma podkreślnik przed `_since_date` i buduje URL bez `&qso_qslsince=...`.
- **Opis problemu:** Każde pobranie potwierdzeń z LoTW pobiera wszystkie potwierdzone łączności od początku istnienia konta zamiast przyrostowych potwierdzeń od ostatniej synchronizacji.
- **Scenariusz wystąpienia:** Kliknięcie „Pobierz potwierdzenia z LoTW” przez stację z wieloletnim logiem (np. 50 000 QSO).
- **Wpływ:** Długi czas pobierania i obciążenie serwera ARRL LoTW.
- **Rozwiązanie minimalne:** Gdy `since_date` jest `Some(date)`, dołączać parametr `&qso_qslsince={date}` do zapytania `lotwreport.adi`.
- **Rozwiązanie docelowe:** Zapamiętywanie daty ostatniej udanej synchronizacji LoTW w `OnlineServicesConfig` i przekazywanie jej jako `since_date`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `lotw_fetch_confirmations_includes_qso_qslsince_when_provided`.
- **Kryterium akceptacji:** URL generowany przez `fetch_confirmations(Some("2026-01-01"))` zawiera `qso_qslsince=2026-01-01`.


---

## 4. Integracje Radiowe, CAT, Protokoły Natywne i HIL (`CAT-*`)

### Ujednolicone mapowanie znalezisk i commitów Etapu 3 (Bramka Dokumentacyjna)

Status Etapu 3: **SOFTWARE VERIFIED**
Fizyczny HIL: **NOT EXECUTED — SAFETY GATE CLOSED** (10 fizycznych testów sprzętowych pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`).

**Semantyka zwolnienia PTT (Drop vs jawne zamknięcie):**
- **Drop (destruktor):** Inicjuje synchroniczne best-effort wysłanie komendy TX OFF bez blokowania wątku i bez asynchronicznego oczekiwania na odpowiedź radia. Zapewnia ochronę przed porzuceniem wskaźnika, lecz nie gwarantuje asynchronicznego potwierdzenia powrotu radia do RX.
- **Jawne zamknięcie sesji (`release_and_confirm_rx` / `disconnect`):** Wysyła komendę TX OFF, drenuje bufor odbiorczy i asynchronicznie oczekuje na sprzętowe potwierdzenie stanu RX (`confirmed_ptt == false` / `poll().ptt == false`).

| Finding ID | Pierwotny opis z rejestru | Zakres rzeczywisty remediacji | Commit w historii | Zmienione pliki | Zweryfikowane testy | Status końcowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `CAT-002` | Wyścig potwierdzenia PTT w `TciBackend` i `FlexRadioBackend` powodujący fałszywą blokadę `TxArbiterState::Unknown` | Oczekiwanie na asynchroniczne potwierdzenie ramki `TciMessage::Trx` (TCI) oraz `interlock state=TRANSMITTING`/nie-TRANSMITTING (FlexRadio) z drainem, timeoutem i odrzucaniem późnych pakietów | `31ab74e` *(tagowany w commit msg jako CAT-003)*, `ce73b8c` *(tagowany w commit msg jako CAT-005)* | `src/cat/tci.rs`, `src/cat/native/flexradio/backend.rs` | 29 testów TCI w `src/cat/tci.rs`, 234 testy w `src/cat/native/flexradio/backend.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-003` | Brak nadpisania metody traitu `RigBackend::probe` w 5 z 8 backendów (mutujący probe łamie R10 / HIL-COND-27) | Usunięcie domyślnej implementacji `probe()` z `RigBackend` i wdrożenie jawnych, ściśle read-only implementacji dla wszystkich 8 backendów | `3e907aa` *(tagowany w commit msg jako CAT-002)* | `src/cat/backend.rs`, `src/cat/probe.rs`, backendy natywne | 12 testów kontraktowych probe w `tests/cat_native_6b.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-005` | Optymistyczne ustawianie `confirmed_ptt = true` bez potwierdzenia z radia w `YaesuBackend` i `ElecraftBackend` | Wymuszenie odpytania `TX;` (Yaesu) i `TQ;` (Elecraft) z usunięciem optymistycznego przypisania stanu TX | `1fce33a` *(tagowany w commit msg jako CAT-006)* | `src/cat/native/yaesu/backend.rs`, `src/cat/native/elecraft/backend.rs` | 46 testów Yaesu, 69 testów Elecraft, test w `tests/cat_native_6a.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-006` | Luka w `RigHub::resync_tx_arbiter` (`last_update` zamiast `last_state_update`) oraz martwy wariant `StateFreshness::Stale` | Zastąpienie `snapshot.last_update` przez `snapshot.last_state_update`, walidacja `freshness != Stale`, oznaczanie `Stale` przy błędach/rozłączeniu w `RigService` | `1fce33a` | `src/cat/hub.rs`, `src/cat/service.rs` | 5 testów deterministycznych w `tests/cat_native_6a.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-015` | `HilHardwareSession::execute_controlled_ptt` omija bramki `SPLOG_HIL_*`, omija `TxArbiter` i nie potwierdza RX | Integracja ze strażnikiem `acquire_tx_guard`, weryfikacja autoryzacji środowiska, zakaz anteny, wymóg początkowego RX i potwierdzenie `release_and_confirm_rx` | `ddc1c0c` | `src/cat/hil/session.rs` | 14 testów cyklu życia HIL w `tests/cat_native_7.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |


### Ujednolicone mapowanie znalezisk i commitów Etapu 8 (Integracja Multi-Radio w GUI, REST API Rig i Hamlib Proxy)

Status Etapu 8: **COMPLETED_SOFTWARE_VERIFIED**
Data ukończenia: 2026-11-05
Fizyczny HIL: **NOT EXECUTED — SAFETY GATE CLOSED** (10 testów laboratoryjnych HIL bezpiecznie odizolowanych za `#[ignore = "hardware-in-the-loop"]`).

| Finding ID | Tytuł kanoniczny | Remediacja główna | Commity | Zmienione pliki | Testy weryfikacyjne | Status końcowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `CAT-014` | Odcięcie `NativeRigCommand` i `NativeRigEvent` oraz martwy kod legacy | Most dla poleceń natywnych w `RigBackend` (`execute_native`, `drain_native_events`), usunięcie martwego `CatController` | `8002667` | `src/cat/backend.rs`, `src/cat/service.rs`, `src/cat/native/icom/backend.rs`, `src/cat/native/flexradio/backend.rs` | `cat_backend_native_bridge_executes_and_drains_events_without_legacy_controller` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-008` | Ominięcie potwierdzenia `TxArbiter` w Hamlib Proxy i Voice Keyerze oraz korekta semantyki Hamlib | Asynchroniczne potwierdzenie `request_tx_confirmed` i `release_tx_confirmed` w `CatRuntime`, kanoniczna semantyka błędów Hamlib `rig.h` (`-11 = RIG_ENAVAIL`, `-14 = RIG_BUSBUSY`), centralny mapper `map_cat_error_to_hamlib`, brak fałszywego `RPRT 0` przy odrzuceniu PTT | `fd43ed2`, `3cff368` | `src/cat/runtime.rs`, `src/cat/server.rs`, `src/gui/voice_keyer.rs`, `src/gui/app_plugins.rs`, `src/gui/app_cat.rs` | `hamlib_proxy_and_voice_keyer_abort_when_tx_arbiter_rejects_ptt`, `hamlib_minus_11_means_function_not_available`, `hamlib_minus_14_means_bus_busy` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-007` | Niekompletna integracja Multi-Radio w GUI i stałe mapowanie portów Hamlib | Usunięcie fallbacku na pierwsze radio w GUI, unikalne parzyste porty Hamlib per RigId (`ProxyPortManager`: 4534, 4536...), brak kolizji z REST (8420), niezmienność sesji TCP przy zmianie aktywnego radia w GUI | `fe7be53`, `c11d0b1` | `src/gui/app_async.rs`, `src/gui/app_cat.rs`, `src/gui/cat_settings.rs`, `src/gui/vfo_panel.rs`, `src/cat/server.rs` | `hamlib_proxy_port_is_unique_per_rig`, `hamlib_existing_session_remains_bound_to_original_rig`, `hamlib_proxy_port_collision_with_rest_api_is_rejected` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-003` | Utwardzenie bezpieczeństwa sterowania REST API i eliminacja atrap | Podłączenie `CatRuntime`, domyślny bind wyłącznie loopback `127.0.0.1:8420`, zakaz `0.0.0.0`/`::`, walidacja syntaktyczna RigId (400), walidacja f/mode, `deny_unknown_fields`, brak fallbacku na active rig (404 na nieznanym), routing PTT przez TxArbiter (409) | `36d1dee`, `833f28d` | `src/api/server.rs`, `src/gui/app_services.rs` | `rest_api_binds_loopback_by_default`, `rest_server_refuses_public_bind_without_authentication`, `rest_unknown_rig_returns_404_without_fallback`, `rest_invalid_rig_id_returns_400` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-EP-OWN` | Wyłączna własność fizycznego portu sprzętowego UART/COM | Centralny `PhysicalEndpointRegistry`, normalizacja portów Windows (`COM3`, `\\.\COM3`), wykluczenie kolizji Native CAT z zewnętrznym `rigctld`, brak otwierania portów UART przez Hamlib Proxy i Hamlib Client | `cc7f36e` | `src/cat/supervisor.rs`, `src/cat/mod.rs` | `same_serial_endpoint_cannot_have_two_owners`, `native_mode_does_not_start_external_rigctld_for_same_port`, `hamlib_client_mode_does_not_open_native_serial_port`, `hamlib_proxy_never_opens_physical_port_directly` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| Test Matrix | Macierz współbieżności i izolacji wieloradiowej | Równoległe rozkazy do wielu radii, odrzucanie przeterminowanych poleceń, deterministyczny i ograniczony czasowo shutdown pod obciążeniem | `26096de` | `tests/cat_native_6b.rs` | `multi_radio_concurrency_simultaneous_commands_to_different_rigs`, `multi_radio_stale_command_expired_is_rejected`, `multi_radio_bounded_shutdown_under_heavy_concurrent_load` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |

### Ujednolicone mapowanie znalezisk i commitów Etapu 4 (Poprawność natywnych protokołów CAT i bezpieczny auto-reconnect)

Status Etapu 4: **SOFTWARE VERIFIED**
Fizyczny HIL: **NOT EXECUTED — SAFETY GATE CLOSED** (10 fizycznych testów sprzętowych pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`).

| Finding ID | Tytuł kanoniczny (Rejestr) | Remediacja główna (Primary remediation) | Rozszerzenie utwardzające (Additional hardening) | Commity | Zmienione pliki | Testy weryfikacyjne | Status | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|---|
| `CAT-001` | Całkowity brak automatycznego reconnect w `RigService::run_loop` | Pętla automatycznego reconnectu z exponential backoff i jitterem w `RigService` | Kodowanie BCD Little-Endian (częstotliwość) i Big-Endian (poziomy) dla Icom CI-V | `95877ce`, `4c03d18` | `src/cat/native/icom/{codec.rs,backend.rs}`, `src/cat/service.rs` | 44 testy w `cat::native::icom`, 7 testów w `service::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-004` | Błędne parsowanie szerokości filtru i brak walidacji ramek CI-V | Walidacja granic ramek CI-V (min 6 B), odrzucanie echa, NAK, broadcastu, obcego adresu | Obsługa komend split `0x25` i RIT/XIT `0x21`, brak mutacji stanu przy błędzie | `cf2a503` | `src/cat/native/icom/{codec.rs,backend.rs,mod.rs}` | 7 testów framingowych CI-V, 44 testy w `icom` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-009` | Brak odpytywania o częstotliwość TX przy pracy Split w `HamlibBackend` | Odpytywanie komendą `i` w `HamlibBackend::poll_internal` przy włączonym split i aktualizacja `tx_frequency_hz` | Rozróżnienie odczytu `i` oraz `v` przy weryfikacji VFO | `30b6542` | `src/cat/hamlib.rs` | 15 testów w `cat::hamlib` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-010` | Brak obsługi komend Split, VFO i Power w `FlrigBackend` oraz zbędne rozłączanie TCP | Pełna obsługa `SetSplitFrequency`, `SetSplit`, `SetVfo`, `SetPower` oraz trwały strumień TCP | Rygorystyczny framing odpowiedzi HTTP/1.1 keep-alive (`MAX_FLRIG_HTTP_HEADER_BYTES = 8192`, `MAX_FLRIG_XMLRPC_BODY_BYTES = 65536`, trwały bufor odbiorczy, Content-Length) | `30b6542`, `af80cbb` | `src/cat/flrig.rs` | 34 testy w `cat::flrig::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-011` | Błędne parsowanie/formatowanie RIT/XIT oraz filtrów w backendach natywnych | Poprawność enkoderów i parserów RIT/XIT oraz passband w backendach natywnych | Ujednolicenie i weryfikacja capabilities w modelach Yaesu i Elecraft | `30b6542` | `src/cat/native/kenwood/codec.rs`, `src/cat/native/yaesu/models.rs`, `src/cat/native/elecraft/models.rs` | 41 testów Kenwood, 46 testów Yaesu, 35 testów Elecraft | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-012` | Pozorne wykonanie komend `SetPower`, `SetRit`, `SetXit` w `KenwoodBackend` i brak `PC` | Fallback getter query po 150 ms przy braku echa AI, odczyt mocy `PC` i obsługa `SetPower`/`SetRit`/`SetXit` | Odczyt mocy w `poll_internal` dla Kenwood | `30b6542` | `src/cat/native/kenwood/{codec.rs,backend.rs}` | 41 testów w `cat::native::kenwood` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-013` | Brak oczekiwania na `R<seq>|` dla setterów oraz pusta implementacja `SetSplit` w `FlexRadioBackend` | Weryfikacja kodu odpowiedzi `R<seq>|0|` przez `send_and_await_ack` oraz `slice set <tx_slice_idx> tx=1` w `SetSplit` | Pełna obsługa przełączania slice A/B przy `SetSplit` i weryfikacja błędów SmartSDR | `30b6542` | `src/cat/native/flexradio/backend.rs` | `flexradio_set_split_sends_slice_tx_command_and_awaits_r_seq_ack`, 236 testów w `flexradio` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-016` | Zahardkodowany adres CI-V `0x94` w manifestach HIL uniemożliwiający testy z innymi modelami | Dynamiczny lookup adresu CI-V z profilu modelu transceivera w `HilTestBenchManifest::to_rig_config` | Ujednolicenie `ReconnectPolicy` z wykładniczym backoffem i jitterem w `RigService` | `30b6542`, `4c03d18` | `src/cat/hil/manifest.rs`, `src/cat/service.rs` | 1 test w `hil::manifest`, 4 testy w `service::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-017` | Blokujące `std::thread::sleep` w `RigctldSupervisor` oraz niespójności profili | Redukcja sleepu w `RigctldSupervisor`, korekta profilu Yaesu oraz odrzucanie PTT ON w stanie `Degraded` | 1) Zachowanie zewnętrznego TX po reconnect bez maskowania przez `state.ptt = false`, rozdzielenie od `TxOwned`, obsługa `PttState::Unknown`, blokowanie `SetPtt(true)` przy External TX.<br>2) Izolacja generacji sesji (`SessionOperationResult`, sprawdzanie `session_generation` przed mutacją stanu, odrzucanie starych connect/disconnect/poll/command/PTT/error/timeout, powiązanie timera reconnectu z generacją, shutdown cancellation).<br>3) Rygorystyczny framing odpowiedzi HTTP/1.1 keep-alive FLRig (`MAX_FLRIG_HTTP_HEADER_BYTES`, `MAX_FLRIG_XMLRPC_BODY_BYTES`, trwały bufor odbiorczy, Content-Length, peer close, clean reset on reconnect). | `30b6542`, `a215cf6`, `9c8b877`, `1c4b36a`, `af80cbb` | `src/cat/supervisor.rs`, `src/cat/native/yaesu/models.rs`, `src/cat/service.rs`, `src/cat/hub.rs`, `src/cat/tx_arbiter.rs`, `src/cat/flrig.rs` | 10 testów External TX, 19 testów generacji i wyścigów w `service::tests`, 22 testy framingu w `flrig::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
### `CAT-001` — Całkowity brak automatycznego wznawiania połączenia (`reconnect`) w `RigService::run_loop` po zerwaniu sesji lub błędzie startu
- **ID:** `CAT-001`
- **Tytuł:** Brak pętli automatycznego `reconnect` w `RigService::run_loop` w stanie `Degraded` oraz po błędzie początkowego `connect()`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/service.rs`, `src/cat/hamlib.rs`, `src/cat/tci.rs`
- **Lokalizacja:** [`src/cat/service.rs:389-399`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L389-L399), [`src/cat/service.rs:431-455`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L431-L455), [`src/cat/hamlib.rs:514`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hamlib.rs#L514), [`src/cat/tci.rs:875`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/tci.rs#L875)
- **Dowód z kodu:**
  1. Jeśli początkowe `self.backend.connect().await` w `RigService::run_loop` zwróci błąd (`service.rs:389-399`), aktor ustawia `RigServiceState::Failed` i kończy zadanie Tokio (`return Err(e)`).
  2. Jeśli połączenie zostanie zerwane w trakcie pracy (`Ready`), błąd `self.backend.poll().await` przełącza sesję w `RigServiceState::Degraded` (`service.rs:440-452`). W stanie `Degraded` pętla `tokio::select!` nadal wywołuje wyłącznie `self.backend.poll().await` co `poll_interval`, **nigdy nie wywołując ponownie `self.backend.connect().await`**. Ponieważ backendy po błędzie I/O czyszczą swój strumień (`self.stream = None` w `hamlib.rs:514`, `self.ws = None` w `tci.rs:875`), każde kolejne `poll()` zwraca `Err(RigError::NotConnected)` w nieskończoność.
- **Opis problemu:** Chwilowy zanik zasilania radia, zakłócenie RF na kablu USB-COM lub restart `rigctld`/SmartSDR trwale paraliżuje sesję CAT aż do ręcznego restartu w ustawieniach.
- **Scenariusz wystąpienia:** Chwilowy reset konwertera USB-UART podczas nadawania na KF lub uruchomienie SPLogbook przed włączeniem zasilacza transceivera.
- **Wpływ:** Trwała utrata komunikacji CAT bez automatycznego odzyskania połączenia.
- **Rozwiązanie minimalne:** W `RigService::run_loop` w stanie `Degraded` wywoływać `self.backend.disconnect().await` + `self.backend.connect().await` z wykładniczym backoffem (`500 ms -> 1 s -> 2 s -> 5 s`) przed wznowieniem `poll()`.
- **Rozwiązanie docelowe:** Maszyna stanów reconnectu z licznikiem prób i zachowaniem pętli aktora przy nieudanym początkowym `connect()`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Powiązane z `CAT-006`.
- **Wymagany test regresyjny:** `rig_service_reconnects_automatically_after_stream_drop_in_degraded_state`.
- **Kryterium akceptacji:** Po zasymulowanym błędzie I/O w `poll()` serwis wywołuje `connect()` i po naprawie transportu wraca do `RigServiceState::Ready`.

---

### `CAT-002` — Wyścig potwierdzenia PTT w `TciBackend` i `FlexRadioBackend` powodujący fałszywą blokadę `TxArbiterState::Unknown`
- **ID:** `CAT-002`
- **Tytuł:** Przedwczesny zwrot `Ok(())` z `SetPtt(true)` w `TciBackend` i `FlexRadioBackend` przed nadejściem asynchronicznej ramki potwierdzenia TX
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commity:**
  - `31ab74e`: `fix(cat): require confirmed PTT ownership for TCI (CAT-003)`
  - `ce73b8c`: `fix(cat): require confirmed PTT ownership for FlexRadio (CAT-005)`
- **Wdrożone rozwiązanie (Etap 3):**
  1. W `TciBackend::execute_set_ptt` wprowadzono drain bufora wejściowego przed wysłaniem komendy, pętlę asynchronicznego oczekiwania na ramkę `TciMessage::Trx` (do `command_timeout`), awaryjny timeout z wysłaniem `trx:0,false;`, filtr `discard_late_tx` zapobiegający nieautoryzowanemu wejściu w TX po spóźnionym pakiecie oraz best-effort powrót do RX w `Drop`.
  2. W `FlexRadioBackend::execute_set_ptt_true` / `execute_set_ptt_false` wprowadzono drain bufora, śledzenie `interlock_received_after_xmit`, wymóg nadejścia zarówno odpowiedzi `R<seq>|0|` jak i dopasowanego stanu `interlock state=TRANSMITTING` (dla PTT ON) lub stanu nie-TRANSMITTING (dla PTT OFF), natychmiastowe awaryjne `xmit 0` w razie timeoutu lub odrzucenia, `discard_late_tx` oraz best-effort `Drop`.
- **Zweryfikowane testy regresyjne:**
  - 10 testów symulacyjnych TCI w `src/cat/tci.rs` (wszystkie 29 testów TCI PASSED).
  - 11 testów symulacyjnych FlexRadio w `src/cat/native/flexradio/backend.rs` (wszystkie 234 testy FlexRadio PASSED).
- **Moduł / obszar:** `src/cat/tci.rs`, `src/cat/native/flexradio/backend.rs`, `src/cat/service.rs`, `src/cat/hub.rs`
- **Lokalizacja:** [`src/cat/tci.rs:537, 808-827, 908-913`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/tci.rs#L808-L913), [`src/cat/native/flexradio/backend.rs:415-428, 1028-1068`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L415-L1068), [`src/cat/service.rs:514-526`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L514-L526), [`src/cat/hub.rs:662-692`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hub.rs#L662-L692)
- **Dowód z kodu:**
  1. W `TciBackend::execute(SetPtt(true))` (`tci.rs:908-913`) wysyłana jest ramka WebSocket `trx:0,true;` bez czekania na potwierdzenie. Zaraz potem `RigService::handle_command` (`service.rs:514`) wywołuje `self.backend.poll().await`, które w `tci.rs:808` czeka tylko `short_poll_window = 5 ms` (`tci.rs:537`). Jeśli serwer TCI odpowie po >5 ms, `poll()` zwraca `ptt: false`, a `RigHub::request_tx` (`hub.rs:662-692`) blokuje `TxArbiter` w stanie `Unknown { reason: UnconfirmedState }` i wysyła awaryjne `SetPtt(false)`!
  2. W `FlexRadioBackend::execute(SetPtt(true))` (`flexradio/backend.rs:1028-1068`) metoda czeka na `R<seq>|0|`, ale `recompute_ptt_state()` (`flexradio/backend.rs:415-428`) ustawia `confirmed_ptt_state = Tx` dopiero po nadejściu osobnej linii statusu `S...|interlock state=TRANSMITTING`. Jeśli `interlock` nadejdzie w osobnym pakiecie TCP kilka milisekund po `R<seq>|0|`, `poll()` zwraca `ptt: false` i blokuje `TxArbiter` w `Unknown`.
- **Opis problemu:** Załączenie PTT na łączach WebSocket (TCI) lub TCP (FlexRadio SmartSDR) o opóźnieniu powyżej kilku milisekund kończy się natychmiastowym zrzuceniem nadawania i blokadą bezpieczeństwa.
- **Scenariusz wystąpienia:** Naciśnięcie przycisku TX (lub `F7`) przy pracy z ExpertSDR/Thetis (TCI) lub FlexRadio 6400/6600 przez LAN.
- **Wpływ:** Niemożność stabilnego załączenia nadawania przez TCI i FlexRadio.
- **Rozwiązanie minimalne:** W `TciBackend::execute(SetPtt(target))` oraz `FlexRadioBackend::execute(SetPtt(target))` oczekiwać w pętli (do `command_timeout`) na nadejście ramki potwierdzającej docelowy stan PTT (`trx:0,<target>;` w TCI oraz `interlock state=TRANSMITTING`/`READY` w FlexRadio).
- **Rozwiązanie docelowe:** Jednoznaczne oczekiwanie na potwierdzony stan `ptt == target` wewnątrz `execute(SetPtt)` we wszystkich backendach sieciowych.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `tci_and_flexradio_set_ptt_await_async_status_confirmation_with_25ms_delay`.
- **Kryterium akceptacji:** `request_tx` przechodzi do `TxArbiterState::Owned` nawet wtedy, gdy ramka `trx:0,true;` / `interlock state=TRANSMITTING` dociera 25 ms po wysłaniu komendy.

---

### `CAT-003` — Brak nadpisania metody traitu `RigBackend::probe` w 5 z 8 backendów (`IcomCiV`, `Yaesu`, `Elecraft`, `FlexRadio`, `TCI`) łamie zasadę Read-Only Probe (`R10` / `HIL-COND-27`)
- **ID:** `CAT-003`
- **Tytuł:** Metody `probe()` w 5 backendach są metodami inherentnymi zamiast implementacji `RigBackend::probe`, przez co polimorficzny Probe wywołuje mutujące `connect()`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `3e907aa`: `fix(cat): make backend probing strictly read-only (CAT-002)`
- **Wdrożone rozwiązanie (Etap 3):**
  1. Usunięto domyślną mutującą implementację `RigBackend::probe` z `src/cat/backend.rs`, która wywoływała `connect()`. Kompilator wymusza teraz jawną implementację `probe()` na każdym backendzie.
  2. Zaimplementowano bezpieczny, ściśle niemutujący `probe()` w `impl RigBackend` dla wszystkich 8 backendów (`FlrigBackend`, `HamlibBackend`, `TciBackend`, `ElecraftBackend`, `FlexRadioBackend`, `IcomCiVBackend`, `KenwoodBackend`, `YaesuBackend`).
  3. Żaden backend podczas sondowania nie inicjalizuje pełnej sesji ze zmianą konfiguracji radia ani nie wysyła komend mutujących (`AI1;`, `AI2;`, subskrypcji slices/panadapterów, ani PTT).
- **Zweryfikowane testy regresyjne:**
  - 12 kontraktowych testów read-only probe w `tests/cat_native_6b.rs` (wszystkie 87 testów w pliku PASSED).
- **Moduł / obszar:** `src/cat/backend.rs`, `src/cat/probe.rs`, `src/cat/native/{icom,yaesu,elecraft,flexradio}/backend.rs`, `src/cat/tci.rs`
- **Lokalizacja:** [`src/cat/backend.rs:61-78`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/backend.rs#L61-L78), [`src/cat/probe.rs:40`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/probe.rs#L40), [`src/cat/native/icom/backend.rs:114`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/backend.rs#L114), [`src/cat/native/yaesu/backend.rs:142`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/backend.rs#L142), [`src/cat/native/elecraft/backend.rs:456`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/elecraft/backend.rs#L456), [`src/cat/native/flexradio/backend.rs:302`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L302), [`src/cat/tci.rs:987`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/tci.rs#L987)
- **Dowód z kodu:** Domyślna metoda traitu `RigBackend::probe` (`src/cat/backend.rs:61-78`) wykonuje `self.connect().await?` -> `self.poll().await?` -> `self.disconnect().await`. Tylko `KenwoodBackend` (`kenwood/backend.rs:274`) i `FlrigBackend` (`flrig.rs:684`) nadpisują ją w `impl RigBackend`. W `IcomCiVBackend`, `YaesuBackend`, `ElecraftBackend`, `FlexRadioBackend` i `TciBackend` metody `pub async fn probe(&mut self)` zdefiniowano poza traitem `RigBackend` z inną sygnaturą. W rezultacie `RigProbeRunner::probe` (`src/cat/probe.rs:40`) oraz `HilHardwareSession::execute_safe_probe` (`src/cat/hil/session.rs:584`), operując na `Box<dyn RigBackend>`, wywołują domyślną implementację z `connect()`, która wysyła do radia komendy mutujące (`AI1;` w Yaesu, `AI2;`/`K31;`/`K41;` w Elecraft, `sub slice all` w FlexRadio)!
- **Opis problemu:** Bezpośrednie naruszenie wymagania Read-Only Probe (`Native CAT.md` Sekcje 12.4, 13.6, 14.1, R10 i `HIL-COND-27`).
- **Scenariusz wystąpienia:** Kliknięcie „Testuj połączenie (Probe)” w ustawieniach CAT lub uruchomienie `execute_safe_probe` w HIL.
- **Wpływ:** Modyfikacja stanu Auto-Information / subskrypcji w transceiverze podczas operacji, która ma być gwarantowanie niemutująca.
- **Rozwiązanie minimalne:** Przenieść logikę `probe()` z metod inherentnych do `impl RigBackend for ...` w `IcomCiVBackend`, `YaesuBackend`, `ElecraftBackend`, `FlexRadioBackend` i `TciBackend`.
- **Rozwiązanie docelowe:** Usunąć domyślną implementację `RigBackend::probe` z `src/cat/backend.rs:61-78`, wymuszając przez kompilator jawne zaimplementowanie read-only `probe()` w każdym typie implementującym `RigBackend`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `polymorphic_box_dyn_rig_backend_probe_never_sends_mutating_connect_commands`.
- **Kryterium akceptacji:** Wywołanie `(Box::new(b) as Box<dyn RigBackend>).probe().await` dla wszystkich 8 backendów nie wysyła `AI1;`, `AI2;` ani `sub `.

---

### `CAT-004` — Odwrócona kolejność bajtów BCD (Little-Endian zamiast Big-Endian) dla S-metra i mocy RF w `IcomCiVBackend` (`0x14` / `0x15`)
- **ID:** `CAT-004`
- **Tytuł:** Błędne dekodowanie i kodowanie 2-bajtowego BCD w `src/cat/native/icom/codec.rs` i `backend.rs`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/native/icom/codec.rs`, `src/cat/native/icom/backend.rs`
- **Lokalizacja:** [`src/cat/native/icom/codec.rs:344-391`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/codec.rs#L344-L391), [`src/cat/native/icom/backend.rs:599-605`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/backend.rs#L599-L605)
- **Dowód z kodu:** W protokole Icom CI-V wyłącznie częstotliwość (5 bajtów) jest Little-Endian BCD, natomiast wszystkie 2-bajtowe poziomy i mierniki (`0x14` / `0x15` w zakresie `0000..0255`) są przesyłane jako **Big-Endian BCD** (`[0x01, 0x20]` = `0120` = S9; `[0x02, 0x55]` = `0255` = 100%). Tymczasem:
  1. `decode_smeter` (`icom/codec.rs:351-354`) i `decode_power_watts` (`icom/codec.rs:380-383`) liczą `(d1 >> 4) * 1000 + (d1 & 0x0F) * 100 + (d0 >> 4) * 10 + (d0 & 0x0F)` (Little-Endian!). Dla odpowiedzi radia `[0x01, 0x20]` (120 = S9) wyliczają `2001 -> .min(255) = 255` (`S9+60dB`)!
  2. `SetPower` (`icom/backend.rs:599-605`) wywołuje `encode_frequency_bcd(raw_val, 2)`, wysyłając dla `128` bajty `[0x28, 0x01]` (`2801` — odrzucane przez radio ramką `NAK 0xFA`) zamiast `[0x01, 0x28]`!
- **Opis problemu:** S-meter i moc wyjściowa na wszystkich transceiverach Icom pokazują błędne (niemal zawsze maksymalne) wartości, a komenda ustawienia mocy `SetPower` jest odrzucana przez radio.
- **Scenariusz wystąpienia:** Odczyt S-metra / mocy lub zmiana mocy RF na dowolnym radiu Icom (IC-7300, IC-7610, IC-705, IC-9700).
- **Wpływ:** Całkowicie niepoprawna telemetria S-metra i mocy RF oraz niedziałające `SetPower` na Icom CI-V.
- **Rozwiązanie minimalne:** W `decode_smeter` i `decode_power_watts` zamienić kolejność bajtów na `(d0 >> 4) * 1000 + (d0 & 0x0F) * 100 + (d1 >> 4) * 10 + (d1 & 0x0F)`, a dla `SetPower` używać kodowania Big-Endian BCD `[((raw_val / 100) as u8), ((((raw_val % 100) / 10) << 4) | (raw_val % 10)) as u8]`.
- **Rozwiązanie docelowe:** Dedykowane funkcje `encode_level_bcd_be(val: u16) -> [u8; 2]` i `decode_level_bcd_be(bytes: &[u8]) -> Option<u16>` z walidacją półbajtów BCD `0..=9` i zakresu `0..=255`.
- **Ryzyko wdrożenia zmiany:** Niskie (wymaga aktualizacji testów jednostkowych w `icom/codec.rs`, które testowały błędny porządek LE).
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `icom_civ_level_and_meter_bcd_encoding_is_big_endian_0000_to_0255`.
- **Kryterium akceptacji:** Ramka `FE FE E0 94 15 02 01 20 FD` dekoduje się jako wartość surowa `120` (`S9`), a `SetPower(50)` wysyła `0x01 0x28`.

---

### `CAT-005` — Optymistyczne ustawianie `confirmed_ptt = true` bez potwierdzenia z radia w `YaesuBackend` i `ElecraftBackend`
- **ID:** `CAT-005`
- **Tytuł:** Ominięcie weryfikacji sprzętowej PTT w `YaesuBackend::execute` i `ElecraftBackend::execute`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `1fce33a`: `fix(cat): enforce fail-safe PTT cleanup (CAT-006)`
- **Wdrożone rozwiązanie (Etap 3):**
  1. W `YaesuBackend::execute(SetPtt(true))` wysyłana jest komenda `TX1;` i natychmiast odpytywany stan `TX;`, wymagając `parse_ptt_resp(resp) == Some(true)`. Przy `SetPtt(false)` wysyłane jest `TX0;` i odpytywane `TX;`, wymagając `Some(false)`. Usunięto optymistyczne przypisania.
  2. W `ElecraftBackend::execute(SetPtt(true))` wysyłane jest `TX;` i odpytywane `TQ;`, wymagając `TransmitStatus(true)`. Przy `SetPtt(false)` wysyłane jest `RX;` i odpytywane `TQ;`, wymagając `TransmitStatus(false)`. Usunięto optymistyczne przypisania przed potwierdzeniem.
  3. Dodano metody `is_ptt_confirmed(&self) -> bool` oraz bezpieczny fallback w `disconnect()`.
- **Zweryfikowane testy regresyjne:**
  - 46 testów Yaesu w `src/cat/native/yaesu/backend.rs` (PASSED).
  - 69 testów Elecraft w `src/cat/native/elecraft/backend.rs` (PASSED).
  - Test `yaesu_and_elecraft_set_ptt_reject_unconfirmed_tx_transition` w `tests/cat_native_6a.rs` (PASSED).
- **Moduł / obszar:** `src/cat/native/yaesu/backend.rs`, `src/cat/native/elecraft/backend.rs`
- **Lokalizacja:** [`src/cat/native/yaesu/backend.rs:448-455, 548-563`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/backend.rs#L448-L563), [`src/cat/native/elecraft/backend.rs:551-555, 670-683`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/elecraft/backend.rs#L551-L683)
- **Dowód z kodu:**
  1. W `YaesuBackend::execute(SetPtt(ptt))` (`yaesu/backend.rs:548-563`) po wysłaniu `TX1;`/`TX0;` kod wykonuje `let _ = self.drain_incoming().await;` i **optymistycznie ustawia `self.confirmed_ptt = *ptt; self.state.ptt = *ptt;`**. Następnie `YaesuBackend::poll()` (`yaesu/backend.rs:448-455`), gdy `self.ai_active && self.state.frequency_hz > 0`, odpytuje tylko `SM0;` i zwraca własne optymistyczne `self.state.ptt` do `TxArbiter`!
  2. W `ElecraftBackend::execute(SetPtt(ptt))` (`elecraft/backend.rs:670-683`) kod również optymistycznie ustawia `self.confirmed_ptt = *ptt; self.state.ptt = *ptt;`, a w `poll()` (`elecraft/backend.rs:551, 555`) ignoruje błąd zapytania `let _ = self.query_and_await_response("TQ;", "TQ").await;`.
- **Opis problemu:** Jeżeli transceiver Yaesu lub Elecraft odrzuci przejście w TX (np. poza pasmem `?;` lub blokada TX) albo nie wyłączy nadawania przy `SetPtt(false)`, `TxArbiter` otrzyma fałszywe potwierdzenie zmiany stanu.
- **Scenariusz wystąpienia:** Próba załączenia lub zwolnienia PTT na Yaesu/Elecraft przy zablokowanym nadajniku lub zerwanym kablu RX.
- **Wpływ:** Złamanie niezmiennika bezpieczeństwa `TxArbiter` (fałszywy stan `Owned` lub `Idle`).
- **Rozwiązanie minimalne:** W `YaesuBackend::execute(SetPtt)` wykonywać `self.query_and_await_response("TX;", "TX").await?`, a w `ElecraftBackend::execute(SetPtt)` oraz `poll()` wymagać poprawnej odpowiedzi `self.query_and_await_response("TQ;", "TQ").await?` bez wcześniejszego przypisywania `self.confirmed_ptt = *ptt`.
- **Rozwiązanie docelowe:** Aktualizacja `confirmed_ptt` wyłącznie w parserze ramek przychodzących (`TX0;`/`TX1;`/`TX2;` oraz `TQ0;`/`TQ1;`).
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `yaesu_and_elecraft_set_ptt_reject_unconfirmed_tx_transition`.
- **Kryterium akceptacji:** Brak odpowiedzi `TX1;` / `TQ1;` z transportu powoduje zwrot `Err(RigError::Timeout)` i przejście `TxArbiter` w `Unknown`.

---

### `CAT-006` — Luka bezpieczeństwa w `RigHub::resync_tx_arbiter` (użycie `last_update` zamiast `last_state_update`) oraz martwy wariant `StateFreshness::Stale`
- **ID:** `CAT-006`
- **Tytuł:** Odblokowanie `TxArbiterState::Unknown` bez świeżego odczytu radia oraz brak oznaczania `StateFreshness::Stale`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `1fce33a`: `fix(cat): enforce fail-safe PTT cleanup (CAT-006)`
- **Wdrożone rozwiązanie (Etap 3):**
  1. W `RigHub::resync_tx_arbiter` zastąpiono `snapshot.last_update` przez `snapshot.last_state_update`, uniemożliwiając odblokowanie `Unknown` na podstawie zdarzeń cyklu życia lub nieudanych komend.
  2. Wymuszono `snapshot.freshness != StateFreshness::Stale`.
  3. W `RigService` (`transition_to` i `run()`) oznaczany jest stan `StateFreshness::Stale` przy błędach odczytu telemetrycznego, timeoutach oraz stanach Degraded, Disconnecting, Disconnected i Failed.
- **Zweryfikowane testy regresyjne:**
  - 5 testów deterministycznych symulacji w `tests/cat_native_6a.rs` (`resync_tx_arbiter_rejects_stale_state_without_new_poll`, `concurrent_ptt_on_requests_allow_only_one_owner`, `disconnect_racing_with_confirmation_does_not_enter_tx_owned`, `stale_session_confirmation_cannot_claim_tx_ownership`, `yaesu_and_elecraft_set_ptt_reject_unconfirmed_tx_transition`). Wszystkie 165 testów w pliku PASSED.
- **Moduł / obszar:** `src/cat/hub.rs`, `src/cat/service.rs`
- **Lokalizacja:** [`src/cat/hub.rs:888-908`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hub.rs#L888-L908), [`src/cat/service.rs:100-109, 298, 342, 440-452, 556`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L100-L556)
- **Dowód z kodu:**
  1. W `RigHub::resync_tx_arbiter` (`src/cat/hub.rs:904`) przekazywane jest `snapshot.last_update` zamiast `snapshot.last_state_update`. Ponieważ `last_update` jest podbijane przy każdej zmianie stanu cyklu życia (`transition_to`, `service.rs:298`) oraz po każdej komendzie (`service.rs:556` — nawet zakończonej błędem!), `resync_tx_arbiter` widzi znacznik czasu nowszy niż `entered_at` i ufa staremu buforowanemu `snapshot.state.ptt == false` sprzed awarii!
  2. Wariant `StateFreshness::Stale` (`src/cat/service.rs:108`) nie jest przypisywany w żadnym miejscu w kodzie.
- **Opis problemu:** Po wejściu w stan awaryjny `TxArbiterState::Unknown` wywołanie `resync_tx_arbiter` może przywrócić `Idle` bez ani jednego udanego `poll()` z radia.
- **Scenariusz wystąpienia:** Timeout komendy `SetPtt` przełącza `TxArbiter` w `Unknown`, a następnie `RigService` przechodzi w `Degraded` (co podbija `last_update`), po czym wywołane zostaje `resync_tx_arbiter`.
- **Wpływ:** Złamanie niezmiennika bezpieczeństwa R2 (`Native CAT.md` Sekcja 8).
- **Rozwiązanie minimalne:** W `RigHub::resync_tx_arbiter` (`src/cat/hub.rs:904`) użyć `snapshot.last_state_update.ok_or_else(|| RigError::SafetyInterlock("No state update".into()))?` oraz wymagać `snapshot.freshness == StateFreshness::Fresh`; w `RigService::run_loop` przy błędzie `poll()` (`service.rs:440`) ustawiać `s.freshness = StateFreshness::Stale`.
- **Rozwiązanie docelowe:** Walidacja `freshness == Fresh && service_state == Ready && last_state_update > entered_at` w `resync_tx_arbiter`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `resync_tx_arbiter_rejects_degraded_or_lifecycle_only_timestamp`.
- **Kryterium akceptacji:** `resync_tx_arbiter` zwraca `Err(RigError::SafetyInterlock)`, dopóki po wejściu w `Unknown` nie nastąpi nowy, zakończony sukcesem `poll()` ustawiający `last_state_update > entered_at` i `freshness == Fresh`.

---

### `CAT-007` — Niekompletna integracja Multi-Radio w GUI (`poll_cat_outcomes` fallback na pierwsze radio, brak dodawania Radio 2 w `cat_settings.rs`, niedziałający Auto-start `rigctld`)
- **ID:** `CAT-007`
- **Tytuł:** Niejawny fallback na pierwsze radio w `app_async.rs`, brak przycisku dodania `Radio 2` w `cat_settings.rs` i ignorowanie `HamlibConfig::auto_start_rigctld` w `start_cat_service`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-05
- **Commit:** `fe7be53` (`fix(gui): eliminate single-rig fallbacks and support radio 2 configuration (CAT-007)`)
- **Moduł / obszar:** `src/gui/app_async.rs`, `src/gui/cat_settings.rs`, `src/gui/app_cat.rs`, `src/gui/vfo_panel.rs`
- **Lokalizacja:** [`src/gui/app_async.rs:215-220`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_async.rs#L215-L220), [`src/gui/cat_settings.rs:146-210, 256, 1238-1296`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/cat_settings.rs#L146-L256), [`src/gui/app_cat.rs:138-163`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_cat.rs#L138-L163), [`src/gui/vfo_panel.rs:282-322, 515-543, 565-570`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/vfo_panel.rs#L282-L570)
- **Dowód z kodu:**
  1. `poll_cat_outcomes` (`src/gui/app_async.rs:219`) wykonuje `.or_else(|| cat_snap.rigs.iter().next().map(|(_, s)| s))` — jawny fallback na pierwsze radio z mapy, zabroniony przez `R8` / `HIL-COND-18`.
  2. `render_rig_selector_bar` (`src/gui/cat_settings.rs:146-210`) nie posiada przycisku dodania drugiego profilu (`Radio 2`), a `sync_legacy_fields_from_rig` (`cat_settings.rs:256`) nadpisuje globalne pola legacy aktualnie wybraną zakładką.
  3. `SpLogApp::start_cat_service` (`src/gui/app_cat.rs:138-163`) sprawdza wyłącznie `self.config.cat_auto_start_rigctld` (legacy), które nigdy nie jest aktualizowane z `RigBackendConfig::Hamlib(cfg).auto_start_rigctld` edytowanego w `cat_settings.rs:429`.
  4. `vfo_panel.rs` używa lokalnych zmiennych atrapowych dla Split VFO B (`+1k`/`+5k`), Preamp/ATT/NB/NR/AGC oraz wyświetla zahardkodowane `swr = 1.2` podczas TX (`vfo_panel.rs:568`).
- **Opis problemu:** Użytkownik nie może skonfigurować `Radio 2` z poziomu GUI, nie może uruchomić `rigctld` przez checkbox Auto-start w nowym panelu CAT, a panel VFO pokazuje fałszywe dane Split/SWR.
- **Scenariusz wystąpienia:** Próba dodania drugiego radia w ustawieniach CAT, włączenia Auto-startu `rigctld` lub użycia Splitu w panelu VFO.
- **Wpływ:** Niepełna użyteczność architektury wieloradiowej NATIVE-6A/6B w warstwie GUI.
- **Rozwiązanie minimalne:**
  1. Usunąć fallback `.or_else(|| cat_snap.rigs.iter().next()...)` w `app_async.rs:219`.
  2. Dodać przycisk `+ Radio 2` w `cat_settings.rs:146-210`.
  3. W `start_cat_service` (`app_cat.rs:138`) odczytywać `RigBackendConfig::Hamlib(cfg)` z `station_cfg.rigs`.
  4. W `vfo_panel.rs` podpiąć `+1k`/`+5k` pod `RigCommand::SetSplitFrequency` i wyświetlać `rig_state.tx_frequency_hz` oraz `rig_state.rf_power_watts`.
- **Rozwiązanie docelowe:** Pełny widok dwuradiowy SO2R w `vfo_panel.rs` zintegrowany z `So2rState` i `NativeRigCommand`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `start_cat_service_honors_station_rig_hamlib_auto_start_config` oraz `vfo_panel_split_buttons_send_set_split_frequency`.
- **Kryterium akceptacji:** Użytkownik może dodać `Radio 2` w GUI, włączyć auto-start `rigctld` oraz sterować częstotliwością Split VFO B z panelu VFO.

---

### `CAT-008` — Ominięcie potwierdzenia `TxArbiter` w `HamlibProxyServer` (`:4534`), `VoiceKeyer` oraz `PluginCommand::SendVoice`
- **ID:** `CAT-008`
- **Tytuł:** Optymistyczna mutacja PTT w `HamlibProxyServer` oraz odtwarzanie audio bez potwierdzenia `TxArbiter` w `VoiceKeyer` i pluginach
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-05
- **Commity:** `fd43ed2` (`fix(cat): enforce TxArbiter PTT confirmation in proxy and voice keyer (CAT-008)`), `3cff368` (`fix(cat): correct Hamlib error mapping for TxArbiter failures (CAT-008)`)
- **Moduł / obszar:** `src/cat/server.rs`, `src/gui/voice_keyer.rs`, `src/gui/app_plugins.rs`
- **Lokalizacja:** [`src/cat/server.rs:441-452`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/server.rs#L441-L452), [`src/gui/voice_keyer.rs:170-181`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/voice_keyer.rs#L170-L181), [`src/gui/app_plugins.rs:127-139`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_plugins.rs#L127-L139)
- **Dowód z kodu:**
  1. `HamlibProxyServer` (`src/cat/server.rs:441-452`) po odebraniu `T 1` natychmiast ustawia `state.ptt = true` w `shared_state` i odsyła `RPRT 0\n`, zanim pętla GUI (`poll_cat_proxy_commands`) przekaże rozkaz do `CatRuntime::request_tx`.
  2. `VoiceKeyer::transmit_message` (`src/gui/voice_keyer.rs:170-181`) wywołuje fire-and-forget `rt.request_tx(...)`, czeka `100 ms` i bezwarunkowo odtwarza plik WAV, nawet jeśli `TxArbiter` odrzucił PTT!
  3. `PluginCommand::SendVoice` (`src/gui/app_plugins.rs:127-139`) odtwarza plik WAV bez załączenia PTT w ogóle.
- **Opis problemu:** Brak synchronizacji zwrotnej między `TxArbiter` a klientem Hamlib Proxy i odtwarzaczem Voice Keyera.
- **Scenariusz wystąpienia:** WSJT-X podłączony do portu `4534` lub uruchomienie makra Voice Keyera przy zablokowanym `TxArbiter`.
- **Wpływ:** WSJT-X otrzymuje fałszywe `RPRT 0` przy odrzuconym PTT; Voice Keyer odtwarza dźwięk mimo braku nadawania.
- **Rozwiązanie minimalne:** Dodać w `CatRuntime` metodę `request_tx_confirmed` zwracającą `oneshot::Receiver<Result<(), RigError>>` i używać jej w `HamlibProxyServer`, `VoiceKeyer` oraz `PluginCommand::SendVoice`.
- **Rozwiązanie docelowe:** Bezpośrednie połączenie `HamlibProxyServer` z `CatRuntime` z pominięciem pośrednictwa wątku GUI.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Powiązane z `FUNC-003`.
- **Wymagany test regresyjny:** `hamlib_proxy_and_voice_keyer_abort_when_tx_arbiter_rejects_ptt`.
- **Kryterium akceptacji:** Odrzucenie PTT przez `TxArbiter` zwraca kanoniczny kod błędu Hamlib (`RPRT -11 = RIG_ENAVAIL` przy braku funkcji lub `RPRT -14 = RIG_BUSBUSY` przy zajętości szyny nadawczej/konflikcie arbitra) w `HamlibProxyServer`, nigdy fałszywy sukces `RPRT 0`, oraz natychmiast zatrzymuje odtwarzanie audio w `VoiceKeyer` bez mutacji stanu logicznego PTT.

---

### `CAT-009` — Niepełny polling w `HamlibBackend::poll_internal` (brak odczytu VFO, Split TX freq i RIT/XIT)
- **ID:** `CAT-009`
- **Tytuł:** `HamlibBackend::poll_internal` nie odpytuje `v` (VFO), `i` (Split TX freq), `j` (RIT) ani `z` (XIT)
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/hamlib.rs`
- **Lokalizacja:** [`src/cat/hamlib.rs:625-698`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hamlib.rs#L625-L698)
- **Dowód z kodu:** `poll_internal` wysyła tylko `f`, `m`, `l STRENGTH`, `s`, `l RFPOWER`, `t`, nigdy nie aktualizując `vfo`, `tx_frequency_hz`, `rit_hz` ani `xit_hz` ze stanu radia.
- **Opis problemu:** Zmiana VFO A/B lub częstotliwości Split TX pokrętłem na radiu podłączonym przez `rigctld` nie jest widoczna w SPLogbook.
- **Scenariusz wystąpienia:** Praca DX w trybie Split przez `rigctld`.
- **Wpływ:** Nieaktualna częstotliwość nadawcza `tx_frequency_hz` i VFO w `RigState`.
- **Rozwiązanie minimalne:** W `HamlibBackend::poll_internal`, gdy `split_enabled == true`, odpytywać `i` (`\get_split_freq`) i aktualizować `tx_frequency_hz`.
- **Rozwiązanie docelowe:** Odczyt `v` oraz warunkowy odczyt `i`/`j`/`z` co N-ty cykl pollingu.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `hamlib_backend_poll_reads_split_tx_frequency_when_split_enabled`.
- **Kryterium akceptacji:** `poll()` w `HamlibBackend` aktualizuje `tx_frequency_hz` na podstawie odpowiedzi na komendę `s` / `i`.

---

### `CAT-010` — Brak HTTP Keep-Alive w `FlrigClient::call` (20 nowych połączeń TCP/s) i wyłączone możliwości Split/VFO/Power w `FlrigBackend`
- **ID:** `CAT-010`
- **Tytuł:** Lawina gniazd `TIME_WAIT` (1200 połączeń TCP/min) oraz zablokowane `SetSplit`/`SetVfo`/`SetPower` w `FlrigBackend`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/flrig.rs`
- **Lokalizacja:** [`src/cat/flrig.rs:345-416, 601-682`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/flrig.rs#L345-L682)
- **Dowód z kodu:**
  1. `FlrigClient::call` (`flrig.rs:345-416`) otwiera nowe połączenie `TcpStream::connect` z `Connection: close` na każde wywołanie XML-RPC (4 wywołania co 200 ms = 20 gniazd/s).
  2. `FlrigBackend::capabilities()` (`flrig.rs:668-682`) ustawia `passband: false, split: false, split_frequency: false, vfo: false, power: false`.
- **Opis problemu:** Szybkie wyczerpywanie puli portów efemerycznych na Windows oraz brak obsługi Splitu/VFO B przez FLRig.
- **Scenariusz wystąpienia:** Kilkuminutowa praca z włączonym backendem FLRig.
- **Wpływ:** Błędy `AddrInUse` / `TimedOut` po wyczerpaniu portów efemerycznych oraz odrzucanie komend Split/VFO.
- **Rozwiązanie minimalne:** Utrzymywać trwałe połączenie `Option<TcpStream>` z nagłówkiem `Connection: keep-alive` w `FlrigClient` oraz włączyć obsługę `SetSplit`, `SetSplitFrequency` (`set_vfo_b`) i `SetVfo` w `FlrigBackend`.
- **Rozwiązanie docelowe:** Trwały strumień HTTP/1.1 Keep-Alive z automatycznym wznowieniem przy EOF.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `flrig_backend_reuses_single_tcp_stream_across_multiple_xmlrpc_calls`.
- **Kryterium akceptacji:** Wielokrotne wywołania `poll()` w `FlrigBackend` wykorzystują to samo połączenie TCP.

---

### `CAT-011` — Błędne subkomendy `SetRit`/`SetXit` (`0x21`) i `SetSplitFrequency` w `IcomCiVBackend`
- **ID:** `CAT-011`
- **Tytuł:** `IcomCiVBackend` wysyła niepoprawne ramki `0x21 0x00` dla włączenia RIT/XIT, ignoruje offset `hz` i przestraja VFO RX przy `SetSplitFrequency`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/native/icom/backend.rs`
- **Lokalizacja:** [`src/cat/native/icom/backend.rs:493-506, 575-598`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/backend.rs#L493-L598)
- **Dowód z kodu:**
  1. W `SetRit` (`icom/backend.rs:575-586`) i `SetXit` (`587-598`) wysyłany jest 1 bajt `[0x01]`/`[0x00]` do `0x21 0x00` (`SUB_CMD_RIT_FREQ`, która wymaga 3 bajtów offsetu BCD + znaku) zamiast `0x21 0x01` (RIT ON/OFF) i `0x21 0x02` (ΔTX ON/OFF), parametr `hz` nie jest w ogóle wysyłany, a błędy są tłumione przez `let _ = self.send_and_expect_ack(frame).await;`.
  2. W `SetSplitFrequency(freq)` (`icom/backend.rs:493-506`) wysyłana jest komenda `0x05` (`CMD_SET_FREQUENCY`), która zmienia częstotliwość bieżącego VFO odbiorczego zamiast VFO nadawczego (`0x25 0x01` lub VFO B).
- **Opis problemu:** Sterowanie RIT/XIT oraz częstotliwością nadawczą Split na radiach Icom jest niesprawne.
- **Scenariusz wystąpienia:** Włączenie RIT/XIT lub ustawienie częstotliwości Split TX na transceiverze Icom.
- **Wpływ:** Przestrojenie odbiornika zamiast nadajnika przy `SetSplitFrequency` oraz odrzucanie `SetRit`/`SetXit` przez radio.
- **Rozwiązanie minimalne:** Użyć subkomend `0x21 0x01` (RIT ON/OFF), `0x21 0x02` (ΔTX ON/OFF) i `0x21 0x00` (3-bajtowy offset BCD) z propagacją błędu `?`, a w `SetSplitFrequency` użyć `0x25 0x01` (lub przełączenia VFO B).
- **Rozwiązanie docelowe:** Pełna obsługa komend `0x21` i `0x25` zgodnie z tabelą CI-V Icom.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `CAT-004`.
- **Wymagany test regresyjny:** `icom_set_rit_sends_0x21_0x01_and_0x21_0x00_offset_frames`.
- **Kryterium akceptacji:** `SetRit { enabled: true, hz: -150 }` wysyła `0x21 0x01 0x01` oraz `0x21 0x00 0x50 0x01 0x01`, a `SetSplitFrequency` ustawia częstotliwość nieaktywnego VFO.

---

### `CAT-012` — Timeout 1500 ms na setterach bez echa oraz sprzeczność `RigCapabilities` z `execute()` w `KenwoodBackend`
- **ID:** `CAT-012`
- **Tytuł:** `KenwoodBackend::execute` czeka na echo setterów (timeout przy braku zmiany lub wyłączonym `AI2;`) i zwraca `Unsupported` dla zadeklarowanych możliwości
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/native/kenwood/backend.rs`, `src/cat/native/registry.rs`
- **Lokalizacja:** [`src/cat/native/kenwood/backend.rs:437-505`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/kenwood/backend.rs#L437-L505), [`src/cat/native/registry.rs:173-300`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/registry.rs#L173-L300)
- **Dowód z kodu:**
  1. `KenwoodBackend::execute` (`kenwood/backend.rs:437-499`) wywołuje `roundtrip_command(&cmd_str, prefix)` dla setterów (`FA...;`, `MD...;`, `TX;`, `RX;`). W protokole Kenwood settery nie odsyłają odpowiedzi, jeśli stan się nie zmienił lub `AI2;` nie jest aktywne, co powoduje blokadę na `1500 ms` i `Err(RigError::Timeout)`.
  2. Profile Kenwood deklarują `passband: true, rit: true, xit: true, power: true`, ale `KenwoodBackend::execute` (`kenwood/backend.rs:502-505`) zwraca dla nich `Err(RigError::Unsupported)`.
- **Opis problemu:** Ryzyko fałszywych timeoutów przy sterowaniu Kenwoodem (w tym wejścia `TxArbiter` w `Unknown` przy `SetPtt`) oraz niezgodność możliwości z implementacją.
- **Scenariusz wystąpienia:** Wysłanie `SetMode("USB")`, gdy radio Kenwood jest już w trybie USB, lub wywołanie `SetPower` / `SetRit`.
- **Wpływ:** Zawieszanie kolejki rozkazów CAT na 1,5 s i błędy `Timeout` / `Unsupported`.
- **Rozwiązanie minimalne:** W `KenwoodBackend::execute` wysyłać setter wraz z jawnym zapytaniem weryfikującym (np. `FA014074000;FA;`, `TX;IF;`, `RX;IF;`) oraz zaimplementować `PCxxx;` (`SetPower`), `RT0;`/`RT1;` (`SetRit`) i `XT0;`/`XT1;` (`SetXit`).
- **Rozwiązanie docelowe:** Pełna implementacja wszystkich komend zadeklarowanych w `RigCapabilities` dla Kenwooda.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `kenwood_execute_setter_ appends_verification_query_and_supports_power_rit_xit`.
- **Kryterium akceptacji:** Wszystkie komendy włączone w `RigCapabilities` profilu Kenwood są obsługiwane w `execute()` bez polegania na spontanicznym echu `AI2;`.

---

### `CAT-013` — Brak oczekiwania na `R<seq>|` dla setterów oraz pusta implementacja `SetSplit` w `FlexRadioBackend`
- **Finding ID:** `CAT-013`
- **Canonical title:** Brak oczekiwania na `R<seq>|` dla setterów oraz pusta implementacja `SetSplit` w `FlexRadioBackend`
- **Severity:** `HIGH`
- **Confidence:** `CONFIRMED`
- **Original evidence:** [`src/cat/native/flexradio/backend.rs:914-1027, 1080-1126`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L914-L1126)
- **Root cause:** Settery działały fire-and-forget (`send_command_tracked` bez `wait_for_response`), a `SetSplit` nie wykonywał rzeczywistego przełączenia TX slice (pusty blok no-op ignorujący błędy SmartSDR).
- **Primary remediation:** Weryfikacja kodu odpowiedzi `R<seq>|0|` przez `send_and_await_ack` dla wszystkich setterów oraz wysyłanie `slice set <tx_slice_idx> tx=1` w `SetSplit`.
- **Additional hardening:** Pełna obsługa przełączania slice A/B przy `SetSplit` oraz parsowanie i propagacja błędów komend SmartSDR.
- **Commits:** `30b6542`
- **Tests:** `flexradio_set_split_sends_slice_tx_command_and_awaits_r_seq_ack`, 236 testów w `cat::native::flexradio`.
- **Status:** `FIXED_SOFTWARE_VERIFIED`
- **Residual risk:** NISKIE
---

### `CAT-014` — Odcięcie `NativeRigCommand` i `NativeRigEvent` od `RigService` i `RigHub` oraz martwy kod legacy (`CatController`, `CatBackendKind::So2r`)
- **ID:** `CAT-014`
- **Tytuł:** Brak mostu dla `NativeRigCommand`/`NativeRigEvent` w `RigService` oraz martwy `CatController` i `CatBackendKind::So2r` w `src/cat/backend.rs`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-05
- **Commit:** `8002667` (`refactor(cat): add native command bridge and clean up legacy controller (CAT-014)`)
- **Moduł / obszar:** `src/cat/native/mod.rs`, `src/cat/backend.rs`, `src/cat/so2r.rs`
- **Lokalizacja:** [`src/cat/native/mod.rs:48-159`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/mod.rs#L48-L159), [`src/cat/backend.rs:111, 236-413`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/backend.rs#L111-L413), [`src/cat/so2r.rs:1-209`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/so2r.rs#L1-L209)
- **Dowód z kodu:**
  1. `execute_native` i `drain_events` w `IcomCiVBackend` oraz `FlexRadioBackend` nie są częścią traitu `RigBackend` i nigdy nie są wywoływane przez `RigService`.
  2. Stary trait `CatBackend` i enum `CatController` (`src/cat/backend.rs:236-413`) oraz `So2rState` (`src/cat/so2r.rs`) nie są nigdzie używane w kodzie produkcyjnym.
- **Opis problemu:** Rozszerzone funkcje natywne (Preamp, Attenuator, Antena, Dualwatch) są niedostępne z poziomu `RigHub` i GUI.
- **Scenariusz wystąpienia:** Próba sterowania Preamp/ATT/Anteną z panelu VFO.
- **Wpływ:** Niewykorzystany potencjał zaimplementowanych kodeków natywnych oraz szum architektoniczny z `CatController`.
- **Rozwiązanie minimalne:** Dodać metodę `execute_native(&mut self, cmd: &NativeRigCommand)` do `RigBackend` (z domyślnym `Err(RigError::Unsupported)`) i usunąć martwy `CatController`.
- **Rozwiązanie docelowe:** Integracja `NativeRigCommand` i `So2rState` z `RigHub` i `vfo_panel.rs`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** `CAT-007`.
- **Wymagany test regresyjny:** `rig_service_routes_native_rig_command_to_backend`.
- **Kryterium akceptacji:** `RigHub` przekazuje `NativeRigCommand` do natywnego backendu i odbiera `NativeRigEvent`.

---

### `CAT-015` — `HilHardwareSession::execute_controlled_ptt` omija bramki środowiskowe `SPLOG_HIL_*`, omija `TxArbiter` i nie potwierdza powrotu do RX
- **ID:** `CAT-015`
- **Tytuł:** Niezabezpieczona ścieżka PTT w `HilHardwareSession::execute_controlled_ptt` (`src/cat/hil/session.rs:630-684`)
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED / PHYSICAL_HIL_NOT_EXECUTED_SAFETY_GATE_CLOSED**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `ddc1c0c`: `fix(hil): protect hardware PTT session lifecycle (CAT-015)`
- **Wdrożone rozwiązanie (Etap 3):**
  1. Metoda `execute_controlled_ptt` w `HilHardwareSession` została całkowicie zintegrowana ze strażnikiem `acquire_tx_guard` i `release_and_confirm_rx`.
  2. Wprowadzono pełną weryfikację bramek bezpieczeństwa: zakaz PTT w fazie 7A, wymóg statusu HIL-READY Passed, bezwzględny zakaz anteny (wymóg sztucznego obciążenia o odpowiedniej mocy), autoryzacja zmiennych `SPLOG_HIL_ENABLE=1`, `SPLOG_HIL_SESSION`, `SPLOG_HIL_ALLOW_TX` (`I_HAVE_DUMMY_LOAD` lub `1`), oraz odczyt `poll()` potwierdzający, że radio nie nadaje przed załączeniem PTT (`initial_state.ptt == false`).
  3. Zabezpieczenie czasowe oparto na watchdogu `HilTxGuard` o limicie `max_test_duration_ms` (maks. 500 ms) z jawnym powrotem do RX (`backend.poll().ptt == false`).
  4. W razie jakiegokolwiek błędu transportu, timeoutu lub braku potwierdzenia RX następuje awaryjne `SetPtt(false)`, `TxArbiter` przechodzi w `Unknown`, sesja w `FailedOrUnknown`, a `rx_restored_cleanly` nigdy nie jest zwracane jako `true`.
  5. Wszystkie 10 fizycznych punktów wejścia Stage 7B pozostaje hermetycznie zablokowanych za bramką `#[ignore = "hardware-in-the-loop"]`.
- **Zweryfikowane testy regresyjne:**
  - 14 testów symulacyjnych cyklu życia HIL w `tests/cat_native_7.rs` (w tym `hil_execute_controlled_ptt_requires_env_gates_arbiter_and_confirmed_rx`). Wszystkie 297 aktywnych testów HIL PASSED (10 fizycznych IGNORED).
- **Moduł / obszar:** `src/cat/hil/session.rs`
- **Lokalizacja:** [`src/cat/hil/session.rs:630-684`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hil/session.rs#L630-L684)
- **Dowód z kodu:** W przeciwieństwie do `acquire_tx_guard` (`session.rs:451-535`), metoda `execute_controlled_ptt` (`session.rs:630-684`) nie wywołuje `verify_hardware_access_authorization()`, nie korzysta z `TxArbiter`, nie weryfikuje początkowego stanu RX przed `SetPtt(true)`, a po `SetPtt(false)` nie wywołuje `backend.poll().await`, lecz na sztywno ustawia `self.rx_restored_cleanly = true`.
- **Opis problemu:** Złamanie wymogów bezpieczeństwa `Native CAT.md` Sekcja 18.4 (`HIL-COND-16`, `HIL-COND-28`, `HIL-X20`).
- **Scenariusz wystąpienia:** Wywołanie `execute_controlled_ptt` podczas sesji testowej HIL.
- **Wpływ:** Możliwość załączenia nadajnika bez pełnej weryfikacji bramek środowiskowych i bez potwierdzenia powrotu do odbioru (`ConfirmedRx`).
- **Rozwiązanie minimalne:** Przepisać `execute_controlled_ptt` tak, aby wewnętrznie wywoływało `self.acquire_tx_guard(backend, arbiter, consent).await?`, czekało zadany czas i wywoływało `guard.release_and_confirm_rx().await?`.
- **Rozwiązanie docelowe:** Jedna wspólna ścieżka nadawania w `HilHardwareSession` oparta wyłącznie na `HilTxSafetyGuard`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `hil_execute_controlled_ptt_requires_env_gates_arbiter_and_confirmed_rx`.
- **Kryterium akceptacji:** `execute_controlled_ptt` odrzuca wywołanie bez `SPLOG_HIL_ALLOW_TX=1` i weryfikuje `poll().ptt == false` po zwolnieniu PTT.

---

### `CAT-016` — `HilTestBenchManifest::to_rig_config` ma na sztywno wpisany adres CI-V `0x94` (IC-7300) dla wszystkich modeli Icom
- **ID:** `CAT-016`
- **Tytuł:** Zahardkodowany `civ_address: 0x94` w `HilTestBenchManifest::to_rig_config` uniemożliwia testy HIL innych modeli Icom
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/hil/manifest.rs`
- **Lokalizacja:** [`src/cat/hil/manifest.rs:550-559`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hil/manifest.rs#L550-L559)
- **Dowód z kodu:** W `src/cat/hil/manifest.rs:553` dla `CatBackendKind::IcomCiV` wpisywane jest `civ_address: 0x94` niezależnie od wartości `self.matrix.model_name`.
- **Opis problemu:** Manifest HIL dla `IC-7610` (`0x98`), `IC-705` (`0xA4`), `IC-9700` (`0xA2`) lub `IC-706MKIIG` (`0x58`) wygeneruje konfigurację wysyłającą ramki pod adres `0x94`.
- **Scenariusz wystąpienia:** Uruchomienie testu HIL z manifestu dla dowolnego radia Icom innego niż IC-7300.
- **Wpływ:** Brak komunikacji CI-V podczas testów HIL dla pozostałych modeli Icom.
- **Rozwiązanie minimalne:** Pobierać domyślny adres CI-V z `crate::cat::native::registry::find_profile(RigVendor::Icom, &self.matrix.model_name).and_then(|p| p.default_ci_v_address).unwrap_or(0x94)`.
- **Rozwiązanie docelowe:** Opcjonalne pole `civ_address: Option<u8>` w `HilTestBenchManifest` z fallbackiem do rejestru modeli.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `hil_manifest_to_rig_config_uses_profile_civ_address_for_ic7610_and_ic705`.
- **Kryterium akceptacji:** `to_rig_config()` zwraca `0x98` dla `IC-7610` i `0xA4` dla `IC-705`.

---

### `CAT-017` — Blokujące `std::thread::sleep` (do 255 ms) w `RigctldSupervisor` wywoływane z głównego wątku GUI oraz niespójności profili Yaesu/Elecraft
- **Finding ID:** `CAT-017`
- **Canonical title:** Blokujące `std::thread::sleep` (do 255 ms) w `RigctldSupervisor` wywoływane z głównego wątku GUI oraz drobne niespójności profili Yaesu/Elecraft
- **Severity:** `MEDIUM`
- **Confidence:** `CONFIRMED`
- **Original evidence:** [`src/cat/supervisor.rs:316, 375, 385`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/supervisor.rs#L316-L385), [`src/cat/native/yaesu/models.rs:85`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/models.rs#L85), [`src/cat/service.rs:360-490`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L360-L490)
- **Root cause:** Synchroniczne uśpienia na wątku GUI, niespójność flagi `passband: false` w Yaesu oraz brak rozdzielenia stanu sprzętowego PTT od własności `TxOwned`, brak izolacji generacji sesji przed mutacją stanu i wyścig reconnect z pollem.
- **Primary remediation:** Redukcja sleepu w `RigctldSupervisor`, korekta flagi `passband: false` w `create_custom_yaesu_profile` oraz odrzucanie PTT ON w stanie `Degraded`.
- **Additional hardening:**
  1. *Krok 1 (Commit `9c8b877`):* Usunięcie bezwarunkowego `state.ptt = false` w gałęzi udanego poll po reconnect; zachowanie stanu External TX i rozdzielenie go od lokalnej własności `TxOwned`; obsługa `PttState::Unknown`; blokowanie nabycia własności TX przy aktywnym zewnętrznym nadajniku (`ExternalTransmitActive`) bez wysyłania PTT OFF ani PTT ON.
  2. *Krok 2 (Commit `1c4b36a`):* Izolacja generacji sesji (`SessionOperationResult`, weryfikacja `session_generation` przed jakąkolwiek mutacją stanu współdzielonego w `apply_poll_result`, `apply_connect_result`, `apply_disconnect_result`), odrzucanie spóźnionych wyników operacji asynchronicznych (connect, disconnect, poll, command, PTT, błędy, timeouty), powiązanie deadline reconnectu z generacją sesji, natychmiastowe anulowanie asynchronicznych operacji przy sygnale shutdown.
  3. *Krok 3 (Commit `af80cbb`):* Rygorystyczny framing odpowiedzi HTTP/1.1 XML-RPC dla trwałego połączenia FLRig keep-alive (`MAX_FLRIG_HTTP_HEADER_BYTES = 8192`, `MAX_FLRIG_XMLRPC_BODY_BYTES = 65536`, trwały bufor odbiorczy w `FlrigConnection`, parsowanie i walidacja `Content-Length`, odrzucanie zniekształceń i konfliktów, zachowanie nadmiarowych bajtów w buforze dla kolejnych zapytań, czyszczenie bufora przy rozłączeniu/reconnect).
- **Commits:** `30b6542`, `a215cf6`, `9c8b877`, `1c4b36a`, `af80cbb`
- **Tests:**
  - 10 testów External TX (`cat::hub::tests`, `cat::service::tests`, `cat::tx_arbiter::tests`)
  - 19 testów generacji i wyścigów sesji (`cat::service::tests`)
  - 22 testy framingu HTTP/1.1 keep-alive (`cat::flrig::tests`)
- **Status:** `FIXED_SOFTWARE_VERIFIED`
- **Residual risk:** NISKIE
---


### Klasyfikacja Zależności Cargo Audit (Etap 4 / CAT-017)
- **Vulnerabilities:** 0
- **Unsound advisories:** 0
- **Yanked crates:** 1
  - Crate: `async-executor 1.14.0`
  - Ścieżka zależności: `keyring 4.2.0` → `zbus-secret-service-keyring-store 1.0.1` → `zbus 5.19.0` → `async-executor 1.14.0`
  - Klasyfikacja: `TRANSITIVE / NO KNOWN VULNERABILITY / NO UNSOUND ADVISORY`
  - Decyzja: `TEMPORARILY ACCEPTED FOR CAT-017 GATE`
  - Uzasadnienie:
    - Brak modyfikacji `Cargo.toml` i `Cargo.lock` w bieżącej bramce RF.
    - Brak jakichkolwiek CVE ani ostrzeżeń unsound dla tej skrzynki.
    - Zależność w 100% tranzytywna.
    - Aktualizacja drzewa zależności zaplanowana w dedykowanej bramce dependency maintenance.
- **Unmaintained crates:** 6
  - Osobna kategoria ostrzeżeń informacyjnych ekosystemu (nie łączyć z yanked).

---

## 5. Architektura (`ARCH-*`), Jakość Kodu Rust (`RUST-*`) i Współbieżność (`CONC-*`)

### `ARCH-001` — Bezpośredni dostęp do SQLite (`db.lock()`) i operacje sieciowe wewnątrz modułów widoków GUI (`src/gui/*`)
- **ID:** `ARCH-001`
- **Tytuł:** Naruszenie separacji warstw: widoki GUI bezpośrednio wywołują zapytania SQLite i orkiestrują klientów HTTP
- **Kategoria:** `ARCH`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/online_sync.rs`, `src/gui/find_duplicates.rs`, `src/gui/statistics.rs`, `src/gui/qsl_designer.rs`, `src/gui/awards_dialog.rs`
- **Lokalizacja:** [`src/gui/online_sync.rs:114-525`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L114-L525), [`src/gui/find_duplicates.rs:270-325`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/find_duplicates.rs#L270-L325), [`src/gui/statistics.rs:135-170`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/statistics.rs#L135-L170)
- **Dowód z kodu:** Mimo wydzielenia `QsoIngestor` w fali W4, ścieżki odczytu, usuwania duplikatów, potwierdzania QSL i synchronizacji chmurowej pozostały zaszyte bezpośrednio w plikach renderujących GUI (`src/gui/online_sync.rs`, `find_duplicates.rs`, `statistics.rs`), co doprowadziło do pominięcia filtrów `journal_id` (`DATA-005`) i blokowania wątków Tokio (`CONC-001`).
- **Opis problemu:** Rozproszenie logiki dostępu do danych po plikach GUI utrudnia testowanie bez GUI i sprzyja błędom spójności dzienników.
- **Scenariusz wystąpienia:** Rozwój lub testowanie synchronizacji chmurowej i operacji bazodanowych.
- **Wpływ:** Duplikacja kodu dostępu do bazy i pomijanie niezmienników domenowych (np. `rebuild_awards_full()`, `undo_stack`, `journal_id`).
- **Rozwiązanie minimalne:** Wydzielić operacje synchronizacji chmurowej i masowego potwierdzania QSL z `src/gui/online_sync.rs` do usługi w `src/cloud/` lub `src/core/`.
- **Rozwiązanie docelowe:** Wprowadzenie `LogbookQueryService` i `CloudSyncCoordinator` (zob. `docs/ARCHITECTURE_TARGET_STATE.md`).
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Powiązane z `DATA-004`, `DATA-005`, `CONC-001`.
- **Wymagany test regresyjny:** Testy jednostkowe synchronizacji LoTW/eQSL i usuwania duplikatów wykonywane bez instancji `SpLogApp`.
- **Kryterium akceptacji:** Żaden plik widoku `src/gui/*.rs` (poza koordynatorem `app_qso.rs`) nie wywołuje bezpośrednio `app.db.lock()`.

---

### `ARCH-002` — Duplikacja logiki normalizacji i wzbogacania QSO w `PUT /api/v1/qsos/{id}` oraz `SpLogApp::save_qso` (ścieżka edycji)
- **ID:** `ARCH-002`
- **Tytuł:** Edycja istniejącego QSO (`update_qso`) w GUI i REST API omija `QsoIngestor` i duplikuje logikę normalizacji/wzbogacania
- **Kategoria:** `ARCH`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/api/server.rs`, `src/gui/app_qso.rs`
- **Lokalizacja:** [`src/api/server.rs:733-774, 811-840`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/api/server.rs#L733-L840), [`src/gui/app_qso.rs:116-146`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_qso.rs#L116-L146)
- **Dowód z kodu:**
  1. `src/api/server.rs:733-774` definiuje własną funkcję `normalize_and_enrich_qso` dla `put_qso_by_id` (`server.rs:816`), a po `db.update_qso` **nie przebudowuje `AwardsEngine`**!
  2. `SpLogApp::save_qso` przy edycji istniejącego QSO (`editing_qso_id = Some(id)`, `app_qso.rs:116-146`) wywołuje bezpośrednio `db.update_qso(&qso)` **bez wywołania `normalize_qso_fields(&mut qso)` i bez `qso.validate()`**!
- **Opis problemu:** Podczas edycji istniejącego QSO w GUI można zapisać niepoprawną datę/czas lub puste pasmo, a edycja przez `PUT /api/v1/qsos/{id}` zostawia nieaktualny stan `AwardsEngine`.
- **Scenariusz wystąpienia:** Edycja łączności w formularzu GUI lub przez `PUT /api/v1/qsos/{id}`.
- **Wpływ:** Możliwość wprowadzenia nieprawidłowych danych przy edycji QSO oraz rozjazd statystyk dyplomowych po edycji przez REST API.
- **Rozwiązanie minimalne:** Dodać metodę `QsoIngestor::update_existing(id: i64, qso: QsoRecord)` wykonującą `normalize_qso_fields`, `validate`, `enrich_with_dxcc`, `db.update_qso` i aktualizację `AwardsEngine`, używaną zarówno w `app_qso.rs`, jak i `api/server.rs`.
- **Rozwiązanie docelowe:** Wszystkie mutacje QSO (`insert`, `update`, `delete`, `import_batch`) przechodzą wyłącznie przez `QsoIngestor`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `editing_qso_validates_fields_and_updates_awards_engine`.
- **Kryterium akceptacji:** Edycja QSO w GUI i REST API waliduje dane przez `QsoIngestor` i aktualizuje `AwardsEngine`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-03
- **Commit:**
  - `50321f1`: `fix(storage): unify QSO update path and awards sync through QsoIngestor (ARCH-002)`
- **Wdrożone rozwiązanie:**
  1. Dodano domenową metodę `QsoIngestor::update_existing(id, qso)`, która wykonuje bezpieczną normalizację, walidację, wzbogacenie DXCC, aktualizację bazy w transakcji, pełną przebudowę `AwardsEngine` (`rebuild_from_qsos`) oraz rejestrację znaku w `ScpEngine`.
  2. Zaktualizowano `SpLogApp::save_edited_qso` w GUI oraz `put_qso_by_id` w REST API, eliminując zduplikowaną funkcję `normalize_and_enrich_qso` i gwarantując pełną synchronizację silnika dyplomów po każdej edycji.
- **Wdrożone testy regresyjne:**
  - `editing_qso_validates_fields_and_updates_awards_engine` w `src/core/ingest/mod.rs`.

---

### `CONC-001` — Blokowanie wątków roboczych Tokio przez synchroniczne operacje SQLite (`db.lock()`) bez `spawn_blocking` w `online_sync.rs`
- **ID:** `CONC-001`
- **Tytuł:** Synchroniczne blokowanie `std::sync::Mutex<Database>` i pętla 1000 pojedynczych transakcji SQLite wewnątrz `tokio::spawn` w `src/gui/online_sync.rs`
- **Kategoria:** `CONC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia:** 2026-11-03
- **Commity:** `b0afb20` (`fix(data): prevent cross-journal QSO mutations (DATA-004 DATA-005 CONC-001)`)
- **Moduł / obszar:** `src/gui/online_sync.rs`
- **Lokalizacja:** [`src/gui/online_sync.rs:112-120, 152-182, 200-208, 240-270, 286-294, 328-336, 360-368, 441-449, 483-491`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L112-L491)
- **Dowód z kodu:** W `src/gui/online_sync.rs` wewnątrz `tokio::spawn(async move { ... })`:
  1. Odczyt do 10 000 QSO z SQLite (`db.lock().unwrap().get_recent_qsos(10000)`) jest wykonywany bezpośrednio na wątku async Tokio bez `tokio::task::spawn_blocking`.
  2. Po pobraniu potwierdzeń z LoTW (`online_sync.rs:155-171`) i eQSL (`243-259`), pętla `for qso in confirmed_qsos` dla każdego pojedynczego potwierdzenia osobno zakłada blokadę `db.lock()` i wykonuje osobny `UPDATE` w SQLite (brak transakcji wsadowej!). Dla 1000 potwierdzeń z LoTW oznacza to 1000 synchronicznych `fsync` w trybie autocommit na wątku roboczym Tokio!
- **Opis problemu:** Głodzenie puli wątków Tokio i przycinanie wątku GUI czekającego na `self.db.lock()` podczas synchronizacji z LoTW/eQSL.
- **Scenariusz wystąpienia:** Pobranie kilkuset lub kilku tysięcy potwierdzeń z LoTW / eQSL.
- **Wpływ:** Zamrożenie interfejsu GUI i opóźnienia zadań CAT/UDP na czas trwania pętli `UPDATE`.
- **Rozwiązanie minimalne:** Dodać metodę wsadową `Database::mark_lotw_confirmed_batch(&[(String, String, String, String)])` (w jednej transakcji `BEGIN IMMEDIATE ... COMMIT`) i wywoływać ją wewnątrz `tokio::task::spawn_blocking`.
- **Rozwiązanie docelowe:** Przeniesienie wszystkich operacji bazodanowych w zadaniach tła do `spawn_blocking` z operacjami wsadowymi.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `batch_qsl_confirmation_updates_all_records_in_single_transaction`.
- **Kryterium akceptacji:** Potwierdzenie 1000 łączności LoTW/eQSL wykonuje jedną transakcję SQLite w `spawn_blocking` (< 20 ms zamiast kilku sekund).

---

### `CONC-002` — Tworzenie nowej instancji `reqwest::Client` (i puli TLS) przy każdym pojedynczym zapytaniu w `src/cloud/*`
- **ID:** `CONC-002`
- **Tytuł:** Brak współdzielenia `reqwest::Client` w klientach chmurowych (`LoTW`, `eQSL`, `QRZ`, `HamQTH`, `ClubLog`, `HRDLog`, `POTA`, `SOTA`, `Solar`)
- **Kategoria:** `CONC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cloud/`
- **Lokalizacja:** [`src/cloud/qrz.rs:28`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/qrz.rs#L28), [`src/cloud/hamqth.rs:34`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/hamqth.rs#L34), [`src/cloud/lotw.rs:23`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/lotw.rs#L23), [`src/cloud/eqsl.rs:20`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/eqsl.rs#L20), [`src/cloud/clublog.rs:22`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/clublog.rs#L22)
- **Dowód z kodu:** Przy każdym wyszukaniu znaku (`app_lookup.rs`) lub synchronizacji chmurowej konstruowany jest nowy obiekt klienta budujący od zera `reqwest::Client::builder().timeout(...)`, inicjalizujący certyfikaty główne TLS i nową pulę połączeń bez wykorzystania HTTP Keep-Alive pomiędzy zapytaniami.
- **Opis problemu:** Zbędny narzut CPU i pamięci na każdorazową inicjalizację kontekstu TLS i nowy handshake TCP+TLS.
- **Scenariusz wystąpienia:** Szybkie wpisywanie znaków z włączonym zapytaniem QRZ/HamQTH lub auto-upload po każdym QSO.
- **Wpływ:** Wyższa latencja zapytań callbooka i większe zużycie pamięci.
- **Rozwiązanie minimalne:** Przechowywać współdzielony `reqwest::Client` (`Clone` oparty wewnętrznie na `Arc`) w `CloudState`.
- **Rozwiązanie docelowe:** Współdzielony `reqwest::Client` oraz cache sesji (`SessionKey`) dla QRZ.com i HamQTH (zamiast ponownego logowania przy każdym zapytaniu o znak!).
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `qrz_and_hamqth_reuse_session_key_across_consecutive_lookups`.
- **Kryterium akceptacji:** Kolejne zapytania do QRZ/HamQTH wykorzystują współdzielony `reqwest::Client` i zapamiętany klucz sesji.

---

### `RUST-001` — 341 ostrzeżeń `cargo clippy` w kodzie produkcyjnym (`lib` + `bin`) i 417 w testach (brak `-D warnings` w CI)
- **ID:** `RUST-001`
- **Tytuł:** Nieegzekwowana czystość `cargo clippy -- -D warnings` (łącznie ponad 400 ostrzeżeń w workspace)
- **Kategoria:** `RUST`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** Całe repozytorium (`src/**`, `.github/workflows/ci.yml`)
- **Lokalizacja:** [`.github/workflows/ci.yml:52`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/ci.yml#L52), [`CODE_QUALITY_NORMALIZATION_PLAN.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/CODE_QUALITY_NORMALIZATION_PLAN.md)
- **Dowód z narzędzi:** Uruchomienie `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` kończy się kodem wyjścia `101`: **340 ostrzeżeń w `splogbook` (lib)**, **1 ostrzeżenie w `splogbook` (bin)** oraz **417 ostrzeżeń (411 błędów) w `splogbook` (lib test)**. W `.github/workflows/ci.yml:52` komenda `cargo clippy --all-targets --all-features` jest uruchamiana **bez `-- -D warnings`**, więc CI zawsze przechodzi na zielono mimo setek ostrzeżeń (`collapsible_if`, `manual_let_else`, `needless_pass_by_value`, `too_many_arguments`, `cast_possible_truncation`, `float_cmp`, `redundant_closure_for_method_calls` itd.).
- **Opis problemu:** Deklarowany w `CODE_QUALITY_NORMALIZATION_PLAN.md` cel zerowej liczby ostrzeżeń Clippy nie jest egzekwowany przez bramkę CI i uległ regresji podczas kolejnych fal rozwojowych.
- **Scenariusz wystąpienia:** Uruchomienie `cargo clippy -- -D warnings`.
- **Wpływ:** Ukrycie rzeczywistych błędów logicznych i rzutowań typów w tłumie ponad 400 ostrzeżeń.
- **Rozwiązanie minimalne:** Oczyścić ostrzeżenia `clippy` w kodzie bibliotecznym i binarnym oraz dodać `-- -D warnings` w `.github/workflows/ci.yml:52`.
- **Rozwiązanie docelowe:** Jawna konfiguracja `[lints.clippy]` w `Cargo.toml` i bezwzględna bramka `cargo clippy --workspace --all-targets --all-features -- -D warnings` w CI.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** `CI-002`.
- **Wymagany test regresyjny:** Bramka CI `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- **Kryterium akceptacji:** `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` kończy się kodem `0`.

---

## 6. Wydajność (`PERF-*`) i Interfejs Graficzny / UX (`GUI-*`)

### Ujednolicone mapowanie znalezisk i commitów Etapu 9 (Ergonomia Klawiaturowa, Trwałość Konfiguracji i Optymalizacja Renderowania GUI)

Status Etapu 9: **COMPLETED_SOFTWARE_VERIFIED**<br>
Data ukończenia: 2026-11-06<br>
Fizyczny HIL: **NOT EXECUTED — SAFETY GATE CLOSED** (Brak operacji HIL, 0 RF, 0 PTT).

| Finding ID | Tytuł kanoniczny | Remediacja główna | Commity | Zmienione pliki | Testy weryfikacyjne | Status końcowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `GUI-001` | Blokada skrótów `Ctrl+...` przez aktywny `TextEdit`, brak `Enter`/`Esc` w zawodach i brak determinizmu fokusu | Centralny rejestr skrótów `ShortcutRegistry`, konteksty `ActiveFocusContext` i `ShortcutContext`, dopuszczenie skrótów akcji (`Ctrl+W`, `Ctrl+L`, `Ctrl+S`, `Ctrl+K`) podczas edycji tekstu z blokadą kolidujących skrótów nawigacji jednoliterowej, zawężenie `Ctrl+W` (Wipe) wyłącznie do kontekstów `MainQsoEntry` i `ContestQsoEntry`, obsługa `Enter` (logowanie + powrót fokusu na Call) i `Esc` (wipe/anulowanie) w formularzu głównym i oknie Contest | `e028125`, `229dea5`, `60d43e4` | `src/gui/shortcuts.rs`, `src/gui/app_input.rs`, `src/gui/qso_entry.rs`, `src/gui/contest.rs`, `src/gui/app_tests.rs` | `text_edit_shortcuts_ctrl_w_and_ctrl_l_work_when_input_focused`, `contest_enter_logs_and_returns_focus_to_callsign`, `escape_clears_qso_fields_in_contest_and_main_entry`, `tab_order_cycles_predictably_through_entry_fields`, `ctrl_w_does_not_wipe_qso_when_editing_search_filter` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `GUI-002` | Pływający widżet „Asystent Operatora” nie posiada przycisku zamknięcia, a jego stan nie zapisuje się w konfiguracji | Dodanie `.open(&mut app.gui.windows.show_operator_assistant)` w nagłówku okna asystenta, dwukierunkowa synchronizacja z `cfg.ui.show_operator_assistant` w `SpLogApp::snapshot_config()` i wczytywaniu | `e028125`, `acecd8c` | `src/gui/operator_assistant.rs`, `src/gui/app_config.rs`, `src/core/config.rs` | `operator_assistant_visibility_persists_across_config_save`, `operator_assistant_window_can_be_closed_via_header_button` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `GUI-003` | Fałszywy początkowy odczyt S-metra (`S9+20dB`, `-53 dBm`) i mocy (`100 W`) w `SpLogApp::build` przed podłączeniem radia | Inicjalizacja początkowego `RigState` w `SpLogApp::build` kanonicznymi wartościami spoczynkowymi: `s_meter_dbm: -127.0`, `s_meter_unit: "S0"`, `rf_power_watts: 0.0` | `5e273d2` | `src/gui/app.rs`, `src/gui/app_tests.rs` | `initial_rig_state_when_disconnected_reports_s0_and_zero_power` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-005` | Utrata części ustawień po restarcie programu z powodu niekompletnego `SpLogApp::snapshot_config()` | Kompletna synchronizacja w `snapshot_config()` (CW makra i wpm, REST API, alerty solarne, satelity, layout/widoki okien, motyw graficzny, multi-radio RigConfig Radio 1 i Radio 2), maskowanie tokena w GUI (`.password(true)`), redagowanie tokena w logach CAT, walidacja schematu, atomowy zapis pliku konfiguracyjnego przez plik tymczasowy `.tmp` z rotacją `.bak` (`rename_atomic`) i zachowaniem poprawności poprzedniego pliku w razie awarii | `acecd8c`, `c4cf893`, `d564272` | `src/gui/app_config.rs`, `src/core/config.rs`, `src/gui/menu/tools.rs`, `src/cat/tx_arbiter.rs`, `src/gui/app_tests.rs`, `tests/security_guard.rs` | `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`, `config_atomic_write_preserves_previous_file_on_write_failure`, `multi_radio_settings_are_preserved_across_save_and_reload`, `rest_api_token_is_masked_in_gui_configuration`, `rest_api_token_is_redacted_in_logs_and_debug_output` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `PERF-002` | Niepotrzebny continuous repaint 250ms w GUI oraz kosztowne operacje $O(N)$ i $O(S \times N)$ w każdej klatce | Eliminacja bezwarunkowego `request_repaint_after(250ms)` w pętli renderowania, zastąpienie harmonogramem UTC `until_next_second`, deterministyczny pomiar opóźnień repaintu, event-driven wake context z `EventBus` i `CatRuntime`, buforowanie `LogViewState` (`sorted_indices`, `worked_callsigns`) z kluczem rewizji `LogViewCacheKey`, buforowanie `ContestState` (`scoreboard`, `mult_matrix`, `band_breakdown`) z kluczem `ContestCacheKey`, przyspieszenie sprawdzania duplikatów spotów z $O(S \times N)$ do $O(1)$ set lookup, buforowanie telemetrii radia `rig_cache` per `RigId` | `8397351`, `5e273d2`, `665183b` | `src/gui/app.rs`, `src/gui/logbook_table.rs`, `src/gui/contest.rs`, `src/gui/state/log_view.rs`, `src/gui/state/contest.rs`, `src/gui/state/cat.rs`, `src/core/events.rs`, `src/cat/runtime.rs` | `idle_gui_does_not_request_continuous_repaint`, `incoming_telemetry_event_requests_gui_repaint`, `cache_is_reused_when_inputs_are_unchanged`, `cache_is_invalidated_on_qso_change`, `cache_is_invalidated_on_filter_change`, `cache_is_invalidated_on_sort_change`, `cache_is_scoped_to_log_identity`, `cache_is_scoped_to_active_rig_when_required`, `stale_cache_never_crosses_profiles` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |

### `PERF-001` — Kwadratowy algorytm $O(N^2)$ klonowania i serializacji JSON w `src/sync/catchup.rs` (`chunk_inventory`, `chunk_requests`, `chunk_records`)
- **ID:** `PERF-001`
- **Status:** `FIXED` (Etap 1D-B, commit `7106677`)
- **Tytuł:** Kwadratowa złożoność $O(N^2)$ w `chunk_inventory`, `chunk_requests` i `chunk_records` serializująca całe wektory JSON w każdej iteracji pętli
- **Kategoria:** `PERF`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/sync/catchup.rs`
- **Lokalizacja:** [`src/sync/catchup.rs:32-260`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/catchup.rs#L32-L260)
- **Implementacja przed:**
  - W `chunk_inventory`, `chunk_requests` i `chunk_records` dla każdego elementu wejściowego klonowano rosnący wektor `current` i serializowano całą wiadomość `serde_json::to_vec(&msg)` w celu sprawdzenia przekroczenia `MAX_CHUNK_BYTES`.
  - Złożoność: $O(N^2)$ alokacji, klonów i serializacji. Dla $N=15\,000$ elementów wykonywano $>112\,500\,000$ serializacji obiektów. Czas wykonania 15 000 elementów wynosił ~254–257 sekund.
- **Implementacja po:**
  - Zastąpiono algorytm rozwiązaniem ściśle liniowym $O(N)$: każdy element serializowany jest dokładnie jeden raz w celu zmierzenia jego długości bajtowej w JSON.
  - Narzut koperty pustej wiadomości oraz separatorów przecinkowych (`1 + item_len`) wyliczany jest przyrostowo co do bajta bez żadnego wielokrotnego serializowania rosnących wektorów.
  - Przy osiągnięciu granicy `MAX_CHUNK_BYTES` (1 MiB) bufor `current` jest przenoszony (`std::mem::take`) do chunku i zerowany w $O(1)$.
  - **Kontrakt limitów protokołu i SINGLE_RECORD_EXCEPTION_IS_VALID:**
    - `MAX_CHUNK_BYTES` (1 MiB, `src/sync/catchup.rs:18`): docelowy limit grupowania wielu rekordów lub identyfikatorów w pojedynczy `Message`.
    - `MAX_RECORD_BYTES` (6 MiB, `src/sync/catchup.rs:25`): twardy limit pojedynczego rekordu QSO w JSON.
    - `MAX_FRAME_BYTES` (8 MiB, `src/sync/frame.rs:10`): bezwzględny limit transportowy ramkowania TCP (`write_frame`, `decode_length`).
    - Pojedyncze rekordy w przedziale $>1\text{ MiB}$ i $\le 6\text{ MiB}$ są legalnie emitowane w pojedynczym dedykowanym chunku `Message::Records` (`SINGLE_RECORD_EXCEPTION_IS_VALID`).
    - Narzut ramkowania transportowego wynosi 44 B (`FRAME_OVERHEAD_BYTES`: 24 B nonce + 16 B Poly1305 tag + 4 B nagłówek długości), a koperty JSON 33–34 B (łącznie 77–78 B). Rekord 6 MiB zajmuje na łączu $\le 6\,291\,534\text{ B} \ll 8\text{ MiB}$ (zapas $>2{,}09\text{ MiB}$).
    - Rekordy $>6\text{ MiB}$ są bezpiecznie pomijane przez nadawcę (`continue;`), emitując zdarzenie błędu `SyncEvent::Error(SyncError::RecordTooLarge)` oraz log `MULTIOP_ERROR_RECORD_TOO_LARGE` bez przerywania sesji, bez skażenia bazy i bez ujawniania danych QSO.
    - Odbiorca waliduje 4-bajtowy prefiks długości w `decode_length` przed zaalokowaniem bufora ramki, zabezpieczając stertę przed nieograniczoną alokacją przy złośliwych deklaracjach peera.
  - **Liczba operacji liniowych:**
    - `empty_envelope_serializations`: 1,
    - `item_serializations`: $N$,
    - `final_chunk_serializations`: $C$ (liczba wyemitowanych chunków),
    - `frame_encryptions`: $C$,
    - `frame_encodings`: $C$,
    - Łączna liczba operacji: $1 + N + 3C = O(N)$.
- **Wyniki pomiarów (przed -> po, 3 próby):**
  - `chunk_inventory` $N=1\,000$: $2458,97\text{ ms} \rightarrow 3,84\text{ ms}$ (min $3,62$, med $3,84$, max $5,59\text{ ms}$; poprawa **640x**, $99,84\%$)
  - `chunk_inventory` $N=5\,000$: $52\,496,79\text{ ms} \rightarrow 20,78\text{ ms}$ (min $18,38$, med $20,78$, max $24,02\text{ ms}$; poprawa **2526x**, $99,96\%$)
  - `chunk_inventory` $N=15\,000$: $254\,090,00\text{ ms} \rightarrow 63,41\text{ ms}$ (min $58,22$, med $63,41$, max $69,29\text{ ms}$; poprawa **4007x**, $99,975\%$, cel $<200\text{ ms}$ w pełni osiągnięty)
  - `chunk_requests` $N=15\,000$: $256\,840,00\text{ ms} \rightarrow 62,21\text{ ms}$ (min $56,46$, med $62,21$, max $77,21\text{ ms}$; poprawa **4129x**)
  - `chunk_records` $N=3\,000$: $45\,214,06\text{ ms} \rightarrow 152,66\text{ ms}$ (min $131,23$, med $152,66$, max $185,83\text{ ms}$; poprawa **296x**)
  - `chunk_records` $N=15\,000$: $\sim 1100\text{ s (szac.)} \rightarrow 662,82\text{ ms}$ (min $636,53$, med $662,82$, max $688,86\text{ ms}$)
- **Weryfikacja testowa:**
  - `chunk_inventory_preserves_order_and_size_limit`, `chunk_requests_preserves_order_and_size_limit`, `chunk_records_preserves_order_and_size_limit` (kompletność danych, kolejność, brak duplikatów, znaki UTF-8, znaki wymagające ucieczki JSON `\"`, `\\`, `\n`).
  - `chunk_inventory_handles_exact_boundary`, `chunk_requests_handles_exact_boundary`, `chunk_records_handles_exact_boundary` (przypadki graniczne rozmiaru ramki).
  - `optimized_*_chunking_matches_reference_for_small_inputs` (porównanie wyników z implementacją referencyjną).
  - `catchup_chunking_15k_identities_has_linear_serialization_count` (deterministyczny licznik serializacji $\le N + 10$).
  - `catchup_chunking_scaling_ratio_is_near_linear` (stosunek skalowania $4\text{k}/2\text{k} < 2,5$, brak $O(N^2)$).
  - `single_large_record_roundtrips_through_production_frame_codec` (pełny roundtrip rekordu 1–6 MiB przez produkcyjny frame codec, krypto i serializację bez naruszenia limitu transportowego).
  - `frame_decoder_rejects_length_above_transport_limit` oraz `frame_decoder_rejects_length_overflow` (odrzucenie deklaracji długości > 8 MiB i `u32::MAX` bez alokacji bufora).
  - `large_record_rejection_has_no_partial_side_effects` (odrzucenie rekordu > 6 MiB bez częściowego skażenia bazy).
  - `oversized_record_rejection_does_not_claim_full_catchup_success` (brak fałszywego sukcesu synchronizacji przy pominiętym rekordzie).
  - `catchup_chunking_15k_reports_complete_linear_operation_counts` (pełny rozkład liczników operacji $1 + N + 3C$).
  - `estimated_chunk_size_matches_actual_serialized_size` (dokładność estymacji co do bajta dla UTF-8, znaków specjalnych i komentarzy).
  - `last_flag_size_delta_is_accounted_for` (sprawdzenie 1-bajtowej delty `"last":false` vs `"last":true` i konserwatywności estymacji).
- **Pamięć i alokacje (sprostowanie po audycie):**
  - Poprzedni algorytm generował ponad 6 GiB skumulowanego wolumenu alokacji tymczasowych (total allocated temporary bytes over execution) dla $N=15\,000$ elementów wskutek klonowania wektorów i wielokrotnej serializacji.
  - Nowy algorytm utrzymuje w pamięci żywej (live heap) wyłącznie wektor wejściowy i bieżący chunk.
  - Szczyt pamięci fizycznej procesu (peak RSS) nie został bezpośrednio zmierzony w teście.
- **Ryzyko resztkowe:** Niskie (`LOW`).
  - Wyniki pomiarów czasowych zależą od parametrów sprzętowych procesora i systemu operacyjnego.
  - Szczegółowe instrumentation liczników działa wyłącznie pod `#[cfg(test)]`.
  - Peak RSS nie został zmierzony bezpośrednio.
  - Duże pojedyncze rekordy (1–6 MiB) wymagają monitorowania pasma LAN przy łączach o ograniczonej przepustowości.
  - Kompatybilność wire formatu została potwierdzona testami end-to-end, ale nie z heterogenicznymi historycznymi klientami na żywej sieci.

---

### `PERF-002` — Nieograniczone ładowanie 100% bazy do `recent_qsos` w `reload_qsos()` oraz praca $O(N)$ i $O(S \times N)$ w każdej klatce GUI (60 FPS)
- **ID:** `PERF-002`
- **Tytuł:** `reload_qsos()` ładuje wszystkie rekordy dziennika do RAM, a `logbook_table.rs` i `contest.rs` skanują i sortują je w każdej klatce renderowania
- **Kategoria:** `PERF`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/app_qso.rs`, `src/gui/logbook_table.rs`, `src/gui/contest.rs`, `src/gui/spots_map_dialog.rs`
- **Lokalizacja:** [`src/gui/app_qso.rs:475-515, 558-561, 597-600`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_qso.rs#L475-L600), [`src/gui/logbook_table.rs:323-373`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/logbook_table.rs#L323-L373), [`src/gui/contest.rs:146-175, 223-241, 315-345, 505-525, 555-625`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/contest.rs#L146-L625)
- **Dowód z kodu:**
  1. W `SpLogApp::reload_qsos` (`src/gui/app_qso.rs:483-485`) wykonywane jest `let total = db.count_for_journal(&jid).unwrap_or(0); let limit = requested.max(total);`, co ładuje **całą zawartość dziennika** do `self.entry.recent_qsos` przy każdym dodaniu/usunięciu QSO (a w `delete_qso_by_id`, `delete_selected_qso` i `perform_undo` baza jest przeładowywana **3 razy pod rząd**: w `reload_qsos()`, `rebuild_scp()` i `rebuild_awards_full()`!).
  2. W `logbook_table.rs:323-373` w każdej klatce GUI tworzony jest wektor `filtered: Vec<&QsoRecord>` z wielokrotnymi alokacjami `.to_lowercase()` na każdym rekordzie oraz pełnym sortowaniem `filtered.sort_by(...)`.
  3. W `contest.rs` w każdej klatce wykonywanych jest **5 pełnych przebiegów $O(N)$** po `recent_qsos` oraz pętla $O(S \times N)$ sprawdzająca duplikaty dla każdego spotu z klastra (`contest.rs:514-522`).
- **Opis problemu:** Przy dzienniku >10 000 QSO interfejs graficzny traci płynność 60 FPS i zużywa nadmiernie CPU w stanie spoczynku.
- **Scenariusz wystąpienia:** Praca z dużym logiem (>10 000 QSO) w oknie głównym lub w trybie Contest.
- **Wpływ:** Spadek liczby klatek na sekundę (FPS), wysokie obciążenie CPU i opóźnienia wpisywania znaków.
- **Rozwiązanie minimalne:**
  1. W `LogViewState` buforować przefiltrowane i posortowane indeksy `filtered_indices: Vec<usize>` i przeliczać je tylko po zmianie `recent_qsos` lub zmianie tekstu filtra/kolumny sortowania.
  2. W `delete_*` i `perform_undo` przekazywać już wczytane `recent_qsos` do `rebuild_scp` i `rebuild_awards_full` zamiast 3-krotnego odczytu z SQLite.
  3. W `contest.rs` utrzymywać inkrementalny `HashSet` duplikatów i mnożników aktualizowany wyłącznie przy zmianie liczby QSO.
- **Rozwiązanie docelowe:** Prawdziwe stronicowanie SQLite w `LogViewState` i indeksowana pamięć podręczna sesji zawodów.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** Test wydajnościowy renderowania listy / filtrowania dla 25 000 rekordów (< 2 ms przy niezmienionym filtrze).
- **Kryterium akceptacji:** Brak sortowania i alokacji `to_lowercase()` w klatkach GUI, w których dane ani filtr nie uległy zmianie.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `8397351`: `perf(gui): replace unnecessary continuous repaint with event-driven updates`
  - `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
  - `665183b`: `test(gui): make Stage 9 performance measurements reproducible (PERF-002)`
- **Wdrożone rozwiązanie:**
  1. Usunięto bezwarunkowe żądanie odświeżania `ui.ctx().request_repaint_after(Duration::from_millis(250))` w głównej pętli renderowania `render_main_ui`. Zastąpiono je harmonogramem odświeżania zegara UTC do początku kolejnej pełnej sekundy (`until_next_second`), co eliminuje ciągły repaint w stanie spoczynku aplikacji (idle delay ~608-999 ms zamiast 250 ms pętli).
  2. Wprowadzono architekturę event-driven wake: komponenty `EventBus` i `CatRuntime` posiadają metody `set_repaint_trigger` rejestrujące referencję do kontekstu `egui`, wywołujące `request_repaint()` natychmiast po odebraniu nowego pakietu telemetrii lub zdarzenia systemowego, bez potrzeby ciągłego pooling-u w GUI.
  3. W `LogViewState` zaimplementowano buforowanie indeksów sortowania i filtrowania (`sorted_indices`) oraz zbioru unikalnych znaków (`worked_callsigns`) powiązane z kluczem `LogViewCacheKey` (rewizja dziennika, zapytanie wyszukiwania, wybrane pasmo/emisja, kolumna sortowania). Pętla renderowania tabeli logbooka nie wykonuje alokacji ani sortowania dopóki stan wejściowy nie ulegnie zmianie.
  4. W `ContestState` wdrożono buforowanie agregacji (`scoreboard`, `mult_matrix`, `band_breakdown`, `rate`) w oparciu o `ContestCacheKey` (rewizja logu, tożsamość zawodów, kategoria).
  5. W `contest.rs` weryfikację duplikatów spotów klastra DX zoptymalizowano ze złożoności $O(S \times N)$ do $O(1)$ set lookup za pomocą buforowanego zbioru `worked_callsigns`.
  6. W `CatState` wprowadzono bezpieczną pamięć podręczną telemetrii radia (`rig_cache`), powiązaną z tożsamością aktywnego `RigId` i rewizją stanu, z zachowaniem ścisłego limitu pól struktury (poniżej 30 pól).
- **Wdrożone testy regresyjne:**
  - `idle_gui_does_not_request_continuous_repaint`
  - `incoming_telemetry_event_requests_gui_repaint`
  - `cache_is_reused_when_inputs_are_unchanged`
  - `cache_is_invalidated_on_qso_change`
  - `cache_is_invalidated_on_filter_change`
  - `cache_is_invalidated_on_sort_change`
  - `cache_is_scoped_to_log_identity`
  - `cache_is_scoped_to_active_rig_when_required`
  - `stale_cache_never_crosses_profiles`

---

### `GUI-001` — Skróty klawiszowe `Ctrl+...` nie działają podczas wpisywania w formularzu QSO, a w oknie Contest brakuje `Enter` (zapis) i `Esc` (Wipe)
- **ID:** `GUI-001`
- **Tytuł:** Zablokowanie globalnych skrótów `Ctrl+N/W/F/L/D/B/M/K/P/E/I/T` przez `!ctx.egui_wants_keyboard_input()` oraz brak obsługi klawiatury w `contest.rs`
- **Kategoria:** `GUI`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/app_input.rs`, `src/gui/contest.rs`
- **Lokalizacja:** [`src/gui/app_input.rs:44-89`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_input.rs#L44-L89), [`src/gui/contest.rs:418-455`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/contest.rs#L418-L455)
- **Dowód z kodu:**
  1. W `SpLogApp::handle_keyboard_shortcuts` (`src/gui/app_input.rs:44`) cały blok skrótów z modyfikatorem `Ctrl` (`Ctrl+N` fokus na znak, `Ctrl+W` wyczyść formularz, `Ctrl+F` szukaj, `Ctrl+L` zapisz QSO, `Ctrl+D` duplikaty, `Ctrl+B` pasmo, `Ctrl+M` emisja, `Ctrl+K` terminal CW, `Ctrl+P` paleta poleceń) znajduje się wewnątrz warunku `if !ctx.egui_wants_keyboard_input()`. W `egui` metoda `egui_wants_keyboard_input()` zwraca `true` zawsze, gdy fokus znajduje się w dowolnym polu `TextEdit` (czyli podczas normalnej pracy operatora wpisującego znak, RST lub imię!). W rezultacie **`Ctrl+W` (wyczyść formularz) i `Ctrl+L` (zapisz QSO) nie działają wtedy, gdy kursor jest w formularzu QSO**!
  2. W `src/gui/contest.rs:418-455` naciśnięcie `Enter` w polu znaku lub wymiany nie zapisuje łączności (wymagane jest kliknięcie myszą przycisku „Zapisz QSO”!), `Esc` nie czyści pól formularza konkursowego, a po zapisaniu łączności fokus nie wraca automatycznie do pola znaku.
- **Opis problemu:** Podstawowa ergonomia pracy klawiaturowej (szczególnie w zawodach i pileupie) jest zablokowana.
- **Scenariusz wystąpienia:** Operator wpisuje błędny znak w polu Callsign i naciska `Ctrl+W`, aby wyczyścić pole, lub naciska `Enter` w oknie Contest, aby zalogować QSO.
- **Wpływ:** Konieczność odrywania rąk od klawiatury i sięgania po mysz podczas pracy operacyjnej i zawodów.
- **Rozwiązanie minimalne:**
  1. Wyciągnąć skróty z modyfikatorem `Ctrl` (`Ctrl+W`, `Ctrl+L`, `Ctrl+N`, `Ctrl+F`, `Ctrl+P`, `Ctrl+K`, `Ctrl+B`, `Ctrl+M`) przed warunek `if !ctx.egui_wants_keyboard_input()` (pozostawiając wewnątrz tego warunku jedynie pojedyncze klawisze bez `Ctrl`, takie jak `Delete`).
  2. W `contest.rs` obsłużyć `Key::Enter` (zapis QSO + powrót fokusu `response.request_focus()` na pole znaku) oraz `Key::Escape` (wyczyszczenie pól znaku i wymiany).
- **Rozwiązanie docelowe:** Pełny tryb ESM (Enter Sends Message) w oknie Contest.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `keyboard_shortcuts_ctrl_w_and_ctrl_l_work_when_text_edit_focused`.
- **Kryterium akceptacji:** `Ctrl+W` i `Ctrl+L` działają przy aktywnym polu `TextEdit`, a `Enter` w oknie Contest zapisuje QSO i ustawia fokus z powrotem na pole znaku.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
  - `229dea5`: `fix(gui): make QSO focus and keyboard workflow deterministic`
  - `60d43e4`: `fix(gui): isolate Ctrl+W wipe to explicit QSO contexts (GUI-001)`
- **Wdrożone rozwiązanie:**
  1. Zaimplementowano centralny moduł obsługi skrótów klawiszowych `src/gui/shortcuts.rs` oparty o `ShortcutRegistry`, rozróżniający konteksty wykonania (`Global`, `QsoEntry`, `Contest`, `LogTable`).
  2. Wyciągnięto skróty z modyfikatorem `Ctrl` (`Ctrl+W` czyszczenie pól, `Ctrl+L` / `Ctrl+S` zapis łączności, `Ctrl+N` fokus na znak, `Ctrl+F` wyszukiwarka, `Ctrl+K` CW terminal) przed warunek odrzucania zdarzeń przy aktywnym polu `TextEdit`, chroniąc jednocześnie pojedyncze klawisze tekstowe i nawigacyjne przed kolizją podczas edycji.
  3. W oknie zawodów `contest.rs` wdrożono pełną obsługę klawiatury: `Key::Enter` wykonuje zapis łączności i automatycznie żąda przywrócenia fokusu na pole Call (`FocusRequest::Field("callsign")`), `Key::Escape` czyści bieżące pola formularza i przywraca fokus na znak. W razie błędu walidacji (niepełny znak, błędna wymiana) dane w polach nie są niszczone, a operator otrzymuje komunikat błędu.
  4. Skrót `Esc` został sparametryzowany kontekstowo (`EscapeContextRouting`): w pierwszej kolejności zamyka aktywne okna dialogowe/paletę poleceń, następnie czyści formularz QSO i przywraca fokus na pole Callsign, a w spoczynku nie wywołuje żadnych efektów ubocznych.
  5. Kolejność klawisza `Tab` (`TabOrder`) w formularzu QSO i Contest jest deterministyczna i cykliczna.
- **Wdrożone testy regresyjne:**
  - `text_edit_shortcuts_ctrl_w_and_ctrl_l_work_when_input_focused`
  - `contest_enter_logs_and_returns_focus_to_callsign`
  - `escape_clears_qso_fields_in_contest_and_main_entry`
  - `escape_closes_modal_dialog_before_clearing_qso`
  - `tab_order_cycles_predictably_through_entry_fields`
  - `failed_qso_save_preserves_input_content`
  - `ctrl_w_does_not_wipe_qso_when_editing_search_filter`
  - `ctrl_w_does_not_wipe_qso_when_editing_macro_text`
  - `ctrl_w_wipes_qso_in_main_entry_context`
  - `ctrl_w_wipes_qso_in_contest_entry_context`

---

### `GUI-002` — Pływający widżet „Asystent Operatora” (`operator_assistant.rs`) nie posiada przycisku zamknięcia, a jego stan nie zapisuje się w konfiguracji
- **ID:** `GUI-002`
- **Tytuł:** Brak możliwości zamknięcia pływającego okna `operator_assistant.rs` z jego nagłówka i brak trwałości flagi `show_operator_assistant`
- **Kategoria:** `GUI`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/operator_assistant.rs`, `src/gui/app_config.rs`
- **Lokalizacja:** [`src/gui/operator_assistant.rs:23-35`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/operator_assistant.rs#L23-L35), [`src/gui/app_config.rs:28-59`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_config.rs#L28-L59)
- **Dowód z kodu:** Okno `egui::Window::new(t("assistant.title"))` w `operator_assistant.rs:23-35` nie przekazuje `.open(&mut app.gui.windows.show_operator_assistant)` i nie ma przycisku `✕`, przez co zasłania prawy dolny róg ekranu i po ukryciu z menu wraca po każdym restarcie programu (`FUNC-005`).
- **Opis problemu:** Uciążliwe pływające okno zasłaniające tabelę logu na mniejszych ekranach.
- **Scenariusz wystąpienia:** Praca na ekranie 1366x768 lub 1920x1080.
- **Wpływ:** Pogorszenie czytelności dolnej części głównego okna aplikacji.
- **Rozwiązanie minimalne:** Dodać `.open(&mut open)` w `operator_assistant.rs` zapisujące stan do `app.gui.windows.show_operator_assistant` oraz utrwalić tę flagę w `snapshot_config()`.
- **Rozwiązanie docelowe:** Możliwość zadokowania Asystenta Operatora jako zakładki w `egui_dock`.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `FUNC-005`.
- **Wymagany test regresyjny:** `operator_assistant_visibility_persists_across_config_save`.
- **Kryterium akceptacji:** Okno Asystenta posiada przycisk zamknięcia `✕`, a jego wyłączenie jest pamiętane po restarcie.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
  - `acecd8c`: `fix(config): make application settings durable and atomic`
- **Wdrożone rozwiązanie:**
  1. W `src/gui/operator_assistant.rs` dodano parametr `.open(&mut app.gui.windows.show_operator_assistant)` do definicji `egui::Window`, umożliwiając bezpośrednie zamknięcie okna za pomocą przycisku `✕` w nagłówku.
  2. Stan flagi `show_operator_assistant` w `GuiWindowsState` został w pełni powiązany z polem `cfg.ui.show_operator_assistant` w `snapshot_config()` i jest trwale zapisywany w `config.json`.
- **Wdrożone testy regresyjne:**
  - `operator_assistant_visibility_persists_across_config_save`
  - `operator_assistant_window_can_be_closed_via_header_button`

---

### `GUI-003` — Fałszywy początkowy odczyt S-metra (`S9+20dB`, `-53 dBm`) i mocy (`100 W`) w `SpLogApp::build` przed podłączeniem radia
- **ID:** `GUI-003`
- **Tytuł:** Inicjalizacja `RigState` w `SpLogApp::build` nieprawdziwymi wartościami `S9+20dB` i `100.0 W`
- **Kategoria:** `GUI`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/app.rs`
- **Lokalizacja:** [`src/gui/app.rs:245-258`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app.rs#L245-L258)
- **Dowód z kodu:** W `SpLogApp::build` (`src/gui/app.rs:251-253`) początkowy `initial_rig_state` ustawia `s_meter_dbm: -53.0`, `s_meter_unit: "S9+20dB".to_string()`, `rf_power_watts: 100.0` przy `connected: false`.
- **Opis problemu:** Przed podłączeniem jakiegokolwiek radia (oraz w `GET /api/v1/rig`) program raportuje wychylenie S-metra `S9+20dB`!
- **Scenariusz wystąpienia:** Uruchomienie programu bez podłączonego transceivera.
- **Wpływ:** Mylące wskazanie S-metra w GUI i REST API w stanie rozłączonym.
- **Rozwiązanie minimalne:** Zainicjalizować `initial_rig_state` wartościami spoczynkowymi (`s_meter_dbm: -127.0`, `s_meter_unit: "S0".to_string()`, `rf_power_watts: 0.0`).
- **Rozwiązanie docelowe:** Użycie `RigState::default()` i wygaszenie paska S-metra, gdy `!app.cat.connected`.
- **Ryzyko wdrożenia zmiany:** Zerowe.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `initial_rig_state_when_disconnected_reports_s0_and_zero_power`.
- **Kryterium akceptacji:** Po starcie programu bez radia S-meter wskazuje `S0` (`-127 dBm`), a moc `0 W`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
- **Wdrożone rozwiązanie:**
  - W `SpLogApp::build` w `src/gui/app.rs` zmieniono wartości początkowe `initial_rig_state` na spoczynkowe: `s_meter_dbm: -127.0`, `s_meter_unit: "S0".to_string()`, `rf_power_watts: 0.0`. Przed fizycznym nawiązaniem połączenia z radiem aplikacja oraz endpointy REST nie raportują fałszywych wartości `S9+20dB` ani `100W`.
- **Wdrożone testy regresyjne:**
  - `initial_rig_state_when_disconnected_reports_s0_and_zero_power`

---

## 7. Testy (`TEST-*`), Dokumentacja (`DOC-*`), CI/CD (`CI-*`) i Zależności (`DEP-*`)

### `TEST-001` — Czas wykonania zestawu testów jednostkowych wynosi ~532 sekundy (~9 minut) w profilu debug z powodu 4 testów w `src/sync/mod.rs`
- **ID:** `TEST-001`
- **Status:** `FIXED` (Etap 1D-B, commity `7106677` i `d4cd4d2`)
- **Tytuł:** Ekstremalnie wolne wykonanie `cargo test` spowodowane przez $O(N^2)$ w `catchup.rs` oraz incydentalne wywołania Argon2id w testach framing/session
- **Kategoria:** `TEST`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/sync/mod.rs`, `src/sync/catchup.rs`, `src/sync/crypto.rs`
- **Lokalizacja:** [`src/sync/mod.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/mod.rs), [`src/sync/crypto.rs:21-40`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/crypto.rs#L21-L40), [`src/sync/catchup.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/catchup.rs)
- **Implementacja przed:**
  - `cargo test --lib` trwał **353,36 s** (wcześniej w audycie 532 s).
  - `cargo test sync:: --lib` trwał **354,30 s** na 86 testach (99,8% czasu całego pakietu).
  - Koszt wynikał z dwóch źródeł:
    1. ALGORITHM: 3 testy chunkowania `each_*_chunk_is_at_most_one_mib` z algorytmem $O(N^2)$ trwające łącznie ponad 557 sekund CPU (254 s + 257 s + 46 s).
    2. CRYPTO_INCIDENTAL: 13 testów weryfikujących wyłącznie ramkowanie, szyfrowanie pakietów XChaCha20Poly1305, sygnatury AAD i numery sekwencyjne (tampering, replay, missing counter, reorder, wire inspection), które niepotrzebnie wywoływały produkcyjny `derive_key` (Argon2id, m=19 MiB, t=2) jedynie po to, by uzyskać losowy 32-bajtowy klucz symetryczny.
- **Implementacja po:**
  - Wdrożono algorytm liniowy $O(N)$ dla procedury Catch-Up (PERF-001, commit `7106677`), redukując czas testów chunkowania z 557 sekund do 0,57 sekundy (>1800x szybciej).
  - Zastąpiono zbędne, incydentalne wywołania `derive_key` w 13 testach jednostkowych statycznymi kluczami testowymi `[u8; KEY_LEN]`.
  - Zachowano bez zmian produkcyjne parametry kryptograficzne Argon2id: `ARGON2_M_COST_KIB = 19 * 1024` (19 MiB RAM), `ARGON2_T_COST = 2`, `ARGON2_P_COST = 1`, `KEY_LEN = 32`, `SALT_LEN = 16`.
  - Dodano test strażnika nienaruszalności parametrów produkcyjnych: `production_argon2_parameters_remain_unchanged`.
  - Wszystkie testy kryptografii i uwierzytelniania peera/hosta (`key_derivation_is_deterministic`, `different_salt_yields_different_key`, `handshake_succeeds_with_same_secret`, `handshake_fails_with_wrong_secret_without_reply`, `service_end_to_end_live_and_catchup`, `sync_successful_handshake_is_not_rejected_by_timeout`) nadal wykonują pełny proces z rzeczywistym Argon2id.
- **Pomiary czasu przed i po:**
  - `cargo test sync:: --lib`: $354,30\text{ s} \rightarrow 35,70\text{ s}$ (skrócenie o **318,6 sekundy**, **10x szybciej**, 98 testów passed).
  - `cargo test --lib`: $353,36\text{ s} \rightarrow 37,05\text{ s}$ (skrócenie o **316,3 sekundy**, **9,5x szybciej**, 1010 testów passed).
  - `each_inventory_chunk_is_at_most_one_mib`: $254,09\text{ s} \rightarrow 0,14\text{ s}$ (**1814x szybciej**).
  - `each_request_chunk_is_at_most_one_mib`: $256,84\text{ s} \rightarrow 0,14\text{ s}$ (**1834x szybciej**).
  - `each_records_chunk_is_at_most_one_mib`: $45,85\text{ s} \rightarrow 0,29\text{ s}$ (**158x szybciej**).
- **Ryzyko resztkowe:** Zerowe. Brak jakiegokolwiek osłabienia kryptografii, 100% testów zielonych. Commit: `d4cd4d2`.

---

### `DOC-001` — Rozbieżności dokumentacyjne w rejestrze audytowym i specyfikacjach zawodów (arytmetyka CQ WW, województwa SP DX, kategoria nadajnika Cabrillo)
- **ID:** `DOC-001`
- **Tytuł:** Niespójności opisowe i rachunkowe w dokumentacji audytowej domeny zawodów względem zweryfikowanego kodu i testów
- **Kategoria:** `DOC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdrożenia i weryfikacji:** 2026-11-05
- **Commity:**
  - `073b50a`: `docs(audit): close FUNC-001 and FUNC-002 in audit register, roadmap and baseline`
  - `20d6a14`: `docs(audit): close Stage 7 contest compliance repair`
  - `b5b5f99`: `docs(audit): close final Stage 7 contest compliance gate`
  - `docs(audit): correct CQ WW fixture arithmetic and SP DX province names`
- **Moduł / obszar:** `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`
- **Lokalizacja:** [`docs/AUDIT_FINDINGS_REGISTER.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/AUDIT_FINDINGS_REGISTER.md), [`docs/IMPROVEMENT_ROADMAP.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/IMPROVEMENT_ROADMAP.md), [`docs/REMEDIATION_BASELINE.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/REMEDIATION_BASELINE.md)
- **Dowód z kodu i dokumentacji przed korektą:**
  1. Uprzedni opis manualnego fixture CQ WW zawierał niespójność rachunkową (20 pkt QSO sugerowane przy 5 mnożnikach), podczas gdy poprawny rachunek wynosi ściśle: 10 punktów QSO × 10 mnożników = 100 (CLAIMED-SCORE: 100), a łączności zduplikowane wnoszą 0 dodatkowych punktów i 0 nowych mnożników.
  2. Uprzedni opis zawierał błędne nazwy województw przy kodach R oraz C zamiast kanonicznego przypisania: R = mazowieckie, C = łódzkie, K = podkarpackie, P = kujawsko-pomorskie.
  3. Zdublowane wzory i artefakty formatowania matematycznego w tekście dokumentacji.
- **Wdrożone rozwiązanie (Ostateczna Bramka Dokumentacyjna Etapu 7 — DOC-001):**
  1. **Arytmetyka fixture CQ WW**: Wdrożono jednoznaczny rachunek: 10 punktów QSO × 10 mnożników = 100 (CLAIMED-SCORE: 100).
  2. **Obsługa duplikatów CQ WW**: Zdefiniowano zachowanie duplikatów: pozostają w logu Cabrillo 3.0 zgodnie ze standardem, dając 0 dodatkowych punktów i 0 nowych mnożników (wynik wynosi 100). Duplikaty nie mogą być używane do sztucznego uzasadniania 20 punktów QSO.
  3. **Kanoniczna mapa SP DX**: Wprowadzono pełną tabelę 16 województw: B = lubuskie, C = łódzkie, D = dolnośląskie, F = pomorskie, G = śląskie, J = warmińsko-mazurskie, K = podkarpackie, L = lubelskie, M = małopolskie, O = podlaskie, P = kujawsko-pomorskie, R = mazowieckie, S = świętokrzyskie, U = opolskie, W = wielkopolskie, Z = zachodniopomorskie.
  4. **Fixture SP DX**: Opisano manualne fixture stacji zagranicznej: SP1AA (R = mazowieckie, 3 pkt), SP2AA (C = łódzkie, 3 pkt), DL2AA (001, 0 pkt), SP3AA (R = mazowieckie, 3 pkt); punkty: 3 + 3 + 0 + 3 = 9; mnożniki: 3 (R na 80m, C na 40m, R na 20m); wynik: 9 × 3 = 27 (CLAIMED-SCORE: 27). Fixture stacji polskiej: SP1AA (województwo R = mazowieckie).
  5. **Cabrillo 3.0**: Utrzymano jednoznaczne rozdzielenie `CATEGORY-OPERATOR` (`SINGLE-OP`, `MULTI-OP`, `CHECKLOG`) od `CATEGORY-TRANSMITTER` (`ONE`), z wyeliminowaniem błędnej wartości `MULTI`.
  6. **Czyszczenie formatowania**: Usunięto artefakty i zniekształcenia notacji na rzecz czystych wzorów tekstowych (`3 + 3 + 0 + 3 = 9`, `9 × 3 = 27`, `10 × 10 = 100`).
  7. **Spójność audytowa**: Pełna weryfikacja zgodności z testami regresyjnymi bez modyfikacji kodu produkcyjnego ani testów.
- **Ryzyko wdrożenia zmiany:** Zerowe (zmiana wyłącznie dokumentacyjna).
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Powiązane z `FUNC-001`, `FUNC-002`.
- **Wdrożone testy regresyjne:**
  - `scoreboard_matches_manual_cqww_fixture`
  - `claimed_score_matches_corrected_manual_cqww_fixture`
  - `scoreboard_matches_manual_spdx_fixture`
  - `spdx_accepts_each_official_one_letter_voivodeship`
  - `cabrillo_never_emits_category_transmitter_multi`
  - `tests/security_guard.rs`
- **Kryterium akceptacji:** Spełnione w 100% — dokumentacja odpowiada kodowi i testom bez żadnych niespójności.

---

### `CI-001` — Workflow `.github/workflows/build-and-release.yml` nie generuje ani nie podpisuje `release-manifest.json` wymaganego przez auto-updater
- **ID:** `CI-001`
- **Tytuł:** Brak kroku generowania i podpisywania `release-manifest.json` / `release-manifest.json.sig` w potoku wydawniczym GitHub Actions
- **Kategoria:** `CI`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** **FIXED / PRODUCTION_DRY_RUN_VERIFIED**
- **Data wdrożenia i weryfikacji:** 2026-11-01
- **Commity:**
  - `16892e8`: `ci(release): generate signed manifest and complete checksums (CI-001)`
  - `7125c66`: `ci(release): make native dry-run verification fail-closed (CI-001)`
  - `0f56ada`: `fix(ci): implement manifest signature verification in splogbook-signer (CI-001)`
  - `75ef965`: `ci(actions): migrate workflows to latest stable action runtimes`
- **Migracja GitHub Actions i eliminacja ostrzeżeń Node.js 20:**
  - `actions/checkout`: z `@v4` (node20) na `@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1` (`runs.using: node24`)
  - `dtolnay/rust-toolchain`: z `@stable` na `@683b588c487a69d01f6c6ddf40f036ff682cc2c8 # v1` (`runs.using: composite`)
  - `actions/upload-artifact`: z `@v4` (node20) na `@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2` (`runs.using: node24`, `if-no-files-found: error`)
  - `actions/download-artifact`: z `@v4` (node20) na `@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2` (`runs.using: node24`)
  - `softprops/action-gh-release`: z `@v2` (node20) na `@efb35369e0ad2afab669f228072c1b0d510eae64 # v3.0.3` (`runs.using: node24`)
  - Liczba akcji Node.js 20: 0
  - Ostrzeżenia `Node.js 20 is deprecated`: 0
  - Ostrzeżenia `being forced to run on Node.js 24`: 0
  - Własne skrypty Node.js: `WORKFLOW_NODE_NOT_REQUIRED` (brak zbędnego `setup-node`)
- **Weryfikacja rzeczywistego potoku CI dry-run z produkcyjną ścieżką klucza (Run ID: `41317314042`, ref: `main`, commit: `75ef965`):**
  - **Job Build Windows x64 (`124880000444`):** SUCCESS (28m32s)
  - **Job Build Linux x86_64 (`124880000306`):** SUCCESS (18m34s)
  - **Job Publish Release Assets (`124887050942`):** SUCCESS (8m23s)
  - **Krok Create, sign and verify Release Manifest:** SUCCESS
    - Odczytano sekret podpisywania pod `umask 077` do `priv.key` (`600`)
    - Porównano fingerprint SHA-256 klucza publicznego przez `splogbook-signer fingerprint`: `2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a` (100% zgodny z zakotwiczonym zaufanym kluczem `splogbook-release-2026-02`)
    - Podpisano manifest `release-manifest.json` do `release-manifest.json.sig`
    - Zweryfikowano podpis produkcyjnym weryfikatorem updatera (`splogbook-signer verify`)
    - Usunięto materiał klucza prywatnego (`trap 'rm -f priv.key' EXIT`) i potwierdzono brak wycieku do stagingu
  - **Krok Create GitHub Release:** SKIPPED (potwierdzono pominięcie publikacji w trybie dry-run)
  - **Utworzenie git tag:** BRAK (potwierdzone `git tag -l`)
  - **Utworzenie GitHub Release:** BRAK (potwierdzone `gh release list`)
  - **Niezależna walidacja pobranych artefaktów (pobrane przez `gh run download 41317314042`):**
    - Integralność SHA-256 (`SHA256SUMS.txt`): zgodna co do bitu z `SPLogbook-Linux-x86_64.tar.gz` (`fa8bc562716055e34c588d47de486cb3bd235ddcef64ddfc49714af8e9d6394e`) i `SPLogbook-Windows-x64.zip` (`690aca91ee5c9ce8d24863ab8158b4e6e6aacd8f429224e4b55d670f3392c15e`).
    - Manifest `release-manifest.json`: zgodność commit SHA (`75ef9651...`), wersji (`1.2.0`), identyfikatora klucza (`splogbook-release-2026-02`), rozmiarów i sum SHA-256.
    - Sygnatura Ed25519 `release-manifest.json.sig`: zweryfikowana pomyślnie produkcyjnym kluczem.
    - Ochrona sekretów: potwierdzono brak obecności jakichkolwiek kluczy prywatnych ani poświadczeń w archiwach.
  - **Status gotowości potoku wydawniczego:** **RELEASE_PIPELINE_READY**
- **Moduł / obszar:** `.github/workflows/build-and-release.yml`, `src/bin/signer.rs`, `tests/security_guard.rs`, `docs/RELEASE_SECURITY.md`
- **Lokalizacja:** [`.github/workflows/build-and-release.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/build-and-release.yml), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs)
- **Dowód z kodu przed naprawą:** Potok wydawniczy nie generował manifestu `release-manifest.json`, nie podpisywał go narzędziem `splogbook-signer` i nie dołączał podpisu ani kompletnego pliku `SHA256SUMS.txt`. Ponadto brakowało kontroli fail-closed przy braku sekretu `RELEASE_SIGNING_KEY` oraz brakowało ograniczenia uprawnień (było `permissions: contents: write` globalnie).
- **Wdrożone rozwiązanie (Etap 2):**
  1. Ograniczenie uprawnień: `permissions: contents: read` na poziomie nadrzędnym; `contents: write` wyłącznie w jobie `publish-release`.
  2. Dodanie wsparcia dla walidacji w trybie dry-run przez `workflow_dispatch` z `dry_run: true`.
  3. Wymuszenie fail-closed: brak sekretu `RELEASE_SIGNING_KEY` przy próbie publikacji wydania na tagu natychmiast przerywa działanie potoku z kodem 1.
  4. Sprawdzenie kompletności i jednoznaczności artefaktów: weryfikacja obecności i niepustości `SPLogbook-Linux-x86_64.tar.gz` oraz `SPLogbook-Windows-x64.zip`.
  5. Generowanie i auto-weryfikacja `SHA256SUMS.txt` (`sha256sum -c`).
  6. Deterministyczne generowanie `release-manifest.json` z aktywnym kluczem `splogbook-release-2026-02`.
  7. Podpisywanie manifestu przez `splogbook-signer sign` pod rygorem umask `077`, prawami `600` i czyszczeniem klucza prywatnego pułapką `trap 'rm -f priv.key' EXIT`.
  8. Obowiązkowa weryfikacja podpisu (`splogbook-signer verify`) przed wywołaniem kroku publikacji.
  9. Bramka `Create GitHub Release` (`softprops/action-gh-release@v2`) blokująca publikację w trybie `dry_run: true` oraz przy braku tagu `v*`.
- **Wdrożone testy regresyjne:**
  - `release_workflow_fails_closed_without_signing_secret` w `tests/security_guard.rs`
  - `release_workflow_rejects_incomplete_artifact_set` w `tests/security_guard.rs`
  - `release_workflow_rejects_duplicate_artifact_names` w `tests/security_guard.rs`
  - `release_workflow_requires_manifest_signature_and_checksums` w `tests/security_guard.rs`
  - `release_workflow_actions_are_pinned_by_policy` w `tests/security_guard.rs`
  - `release_manifest_is_deterministic` w `tests/security_guard.rs`
  - `release_manifest_lists_every_expected_artifact_once` w `tests/security_guard.rs`
  - `release_manifest_uses_active_key_id` w `tests/security_guard.rs`
  - `release_manifest_hashes_match_built_artifacts` w `tests/security_guard.rs`
  - `release_workflow_references_secret_without_embedding_key_material` w `tests/security_guard.rs`

---

### `CI-002` — Brak flagi `-D warnings` dla `cargo clippy` oraz brak `cargo audit` w `.github/workflows/ci.yml`
- **ID:** `CI-002`
- **Tytuł:** Zbyt permisywna konfiguracja `.github/workflows/ci.yml` przepuszczająca ostrzeżenia Clippy i podatności RustSec
- **Kategoria:** `CI`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `.github/workflows/ci.yml`
- **Lokalizacja:** [`.github/workflows/ci.yml:50-55`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/ci.yml#L50-L55)
- **Dowód z kodu:** W `.github/workflows/ci.yml:52` krok `Run Clippy linter` wykonuje `cargo clippy --all-targets --all-features` bez `-- -D warnings`, przez co nie przerywa buildu przy ostrzeżeniach. Brak również kroku `cargo audit`.
- **Opis problemu:** Regresje jakości kodu i bezpieczeństwa zależności nie są zatrzymywane na etapie Pull Request / Push.
- **Scenariusz wystąpienia:** Każdy przebieg CI na GitHub Actions.
- **Wpływ:** Narastanie długu technicznego (obecnie >400 ostrzeżeń Clippy i 7 podatności w `Cargo.lock`).
- **Rozwiązanie minimalne:** Po oczyszczeniu ostrzeżeń (`RUST-001`) i aktualizacji `Cargo.lock` (`DEP-001`) zmienić komendę w `ci.yml:52` na `cargo clippy --workspace --all-targets --all-features -- -D warnings` i dodać krok `rustsec/audit-check`.
- **Rozwiązanie docelowe:** Pełna bramka jakościowa: `fmt --check`, `clippy -D warnings`, `test --workspace --all-targets`, `cargo audit`.
- **Ryzyko wdrożenia zmiany:** Niskie (po wykonaniu `RUST-001` i `DEP-001`).
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `RUST-001`, `DEP-001`.
- **Wymagany test regresyjny:** Zielony przebieg CI z aktywną flagą `-- -D warnings`.
- **Kryterium akceptacji:** `ci.yml` wymusza zero ostrzeżeń Clippy i zero podatności RustSec.

---

### `DEP-001` — 7 podatności bezpieczeństwa (`RUSTSEC-2026-*`) oraz 1 ostrzeżenie `unsound` (`tokio 1.53.1`) w `Cargo.lock`
- **ID:** `DEP-001`
- **Tytuł:** Podatności w `rustls 0.23.45`, `rustls-webpki 0.103.15`, `webpki-root-certs 1.0.9`, `ctutils 0.4.2` i `tokio 1.53.1` wykryte przez `cargo audit`
- **Kategoria:** `DEP`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Status remediacji:** `FIXED`
- **Data wdrożenia:** 2026-11-01T08:41:47+01:00 (UTC: 2026-11-01T07:41:47Z)
- **Commit:** `335ced5` (`chore(deps): update vulnerable dependencies (DEP-001)`)
- **Moduł / obszar:** `Cargo.lock`, `Cargo.toml`
- **Lokalizacja:** [`Cargo.lock`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Cargo.lock)
- **Baza RustSec:** oficjalne repozytorium `https://github.com/RustSec/advisory-db.git`, commit `2295466c4e9df26647fd67d8fa1c705ac2fc7f81` (Fri Oct 30 12:12:39 2026 +0100).
- **Bramka chronologii RustSec:** `ENVIRONMENT_DATE_AFTER_REFERENCE` (czas środowiska wykonawczego 2026-11-01 jest późniejszy niż data odniesienia 2026-10-12; wszystkie advisory `RUSTSEC-2026-*` zostały opublikowane w bazie między 2026-10-13 a 2026-10-24 przed badaniem).
- **Stan przed aktualizacją (`b0d1b24`):**
  1. `ctutils 0.4.2`: `RUSTSEC-2026-0385` (vulnerability, data: 2026-10-23, commit bazy `f7b2f6b`, patched: `>= 0.4.4`).
  2. `rustls 0.23.45`: `RUSTSEC-2026-0346`, `RUSTSEC-2026-0347`, `RUSTSEC-2026-0348`, `RUSTSEC-2026-0349` (4 vulnerabilities, data: 2026-10-14, commit bazy `caee11f`, patched: `>= 0.23.46`).
  3. `rustls-webpki 0.103.15`: `RUSTSEC-2026-0382` (vulnerability, data: 2026-10-20, commit bazy `e6f94f3`, patched: `>= 0.103.16`).
  4. `webpki-root-certs 1.0.9`: `RUSTSEC-2026-0383` (vulnerability, data: 2026-10-19, commit bazy `4b587f0`, patched: `>= 1.0.10`).
  5. `tokio 1.53.1`: `RUSTSEC-2026-0386` (unsound, data: 2026-10-13, commit bazy `84d943a`, patched: `>= 1.53.3`).
  6. Wycofane pakiety (`yanked`): `clipboard-win 5.4.1`, `yoke-derive 0.8.3`.
  - Łącznie: 7 vulnerabilities, 9 warnings (1 unsound, 2 yanked, 6 unmaintained).
- **Wykonana selektywna aktualizacja (`Cargo.lock`):**
  - Grupa 1 (ctutils): `ctutils` 0.4.2 -> 0.4.7
  - Grupa 2 (TLS): `rustls` 0.23.45 -> 0.23.46, `rustls-webpki` 0.103.15 -> 0.103.16, `webpki-root-certs` 1.0.9 -> 1.0.11
  - Grupa 3 (Tokio): `tokio` 1.53.1 -> 1.53.3
  - Grupa 4 (Yanked): `clipboard-win` 5.4.1 -> 5.4.3, `yoke-derive` 0.8.3 -> 0.8.4
  - Zależności pośrednie dostosowane przez solver Cargo: `windows-sys` 0.60.2 / 0.52.0, `getrandom` 0.3.4 (dla tempfile).
  - Brak modyfikacji `Cargo.toml`, brak podniesienia MSRV, brak zmian major, brak zmian cech (`features`).
- **Stan po aktualizacji (`335ced5`):**
  - Vulnerabilities: 0
  - Unsound: 0
  - Yanked: 0
  - Nowe advisory: 0
  - Unmaintained warnings: 6 (wszystkie sklasyfikowane i udokumentowane).
- **Klasyfikacja i ścieżki pakietów unmaintained (`OPEN_UNMAINTAINED_TRANSITIVE`):**
  1. `bincode 1.3.3` (`RUSTSEC-2025-0141`): `splogbook` -> `printpdf 0.12.8` -> `azul-layout 0.0.16` -> `hyphenation 0.8.4` -> `bincode 1.3.3`.
  2. `paste 1.0.15` (`RUSTSEC-2024-0436`): `splogbook` -> `egui_dock 0.21.1` -> `paste 1.0.15`.
  3. `smartstring 1.0.1` (`RUSTSEC-2026-0249`): `splogbook` -> `rhai 1.26.1` -> `smartstring 1.0.1`.
  4. `zbus_names 4.3.4` (`RUSTSEC-2026-0371`): `splogbook` -> `keyring 4.2.0` -> `zbus-secret-service-keyring-store 1.0.1` -> `secret-service 5.2.0` -> `zbus 5.19.0` -> `zbus_names 4.3.4`.
  5. `zvariant 5.15.0` (`RUSTSEC-2026-0370`): `splogbook` -> `keyring 4.2.0` -> `zbus-secret-service-keyring-store 1.0.1` -> `secret-service 5.2.0` -> `zbus 5.19.0` -> `zvariant 5.15.0`.
  6. `zvariant_derive 5.15.0` (`RUSTSEC-2026-0372`): `splogbook` -> `keyring 4.2.0` -> `zbus-secret-service-keyring-store 1.0.1` -> `secret-service 5.2.0` -> `zbus 5.19.0` -> `zvariant_derive 5.15.0`.
  - **Ryzyko resztkowe:** Żaden z pakietów nie posiada otwartej luki CVE ani błędu bezpieczeństwa; są to pakiety tranzytywne archiwizowane przez autorów. Ich migracja wymaga przejścia nadrzędnych bibliotek na nowsze API (np. zbus 6.0, compact_str) w kolejnych etapach refaktoryzacyjnych.
- **Wyniki testów regresyjnych:**
  - Kryptografia: testy `core::crypto`, `sync::crypto`, `cloud::updater`, `splogbook-signer` oraz 19/19 testów `tests/security_guard.rs` zielone.
  - Komunikacja sieciowa i TLS: testy modułów `cloud::`, `core::http`, `api::server`, `sync::` zielone.
  - Runtime Tokio i SEC-003: 3 dedykowane testy timeoutu LAN Sync (`sync_host_handshake_timeout_releases_slot`, `sync_partial_handshake_times_out`, `sync_successful_handshake_is_not_rejected_by_timeout`) wykonane po 3 razy (3/3 sukcesów, 0 opóźnień wall-clock, deterministyczny czas wirtualny Tokio).
  - Target Windows MSVC: `cargo check --workspace --all-targets --all-features --target x86_64-pc-windows-msvc` zakończony sukcesem (kod 0).
  - Clippy: zero nowych ostrzeżeń we wszystkich celach (`--lib`, `--lib --tests`, `--bin SPLogbook`, `--bin splogbook-signer`).
- **Kryterium akceptacji:** `cargo audit` zgłasza dokładnie 0 podatności `vulnerability`, 0 `unsound`, 0 `yanked`.
