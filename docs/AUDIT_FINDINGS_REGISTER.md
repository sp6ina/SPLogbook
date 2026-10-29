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
- **Status po Etapie 1A (SEC-001):**
  - **REMEDIATED_IN_TREE:** materiał został usunięty z aktualnego drzewa (`git rm src/scratch.rs`);
  - **REGRESSION_GUARDED:** testy (`tests/security_guard.rs`) i CI (`.github/workflows/ci.yml`) blokują ponowne dodanie;
  - **HISTORY_EXPOSURE:** OWNER_DECISION_REQUIRED (skompromitowany materiał obecny w historii commitów od `80533e2` i tagu `v1.2.0`; ewentualne czyszczenie przez git-filter-repo wymaga decyzji właściciela);
  - **PRODUCTION_KEY_ROTATION:** BLOCKED_EXTERNAL_SECRET (wymaga wygenerowania nowego klucza poza repozytorium i wprowadzenia do GitHub Secrets przez właściciela);
  - **HISTORICAL_RELEASE_AUDIT:** OPEN (wymaga weryfikacji integralności dotychczasowych manifestów i wydań);
  - **AUTO_UPDATE_RELEASE_READINESS:** BLOCKED (auto-updater pozostaje fail-closed z 0 aktywnymi kluczami do czasu ukończenia rotacji).
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
- **Moduł / obszar:** `src/cloud/updater.rs`, `.github/workflows/build-and-release.yml`
- **Lokalizacja:** [`src/cloud/updater.rs:658-672`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L658-L672), [`src/cloud/updater.rs:737`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L737), [`src/cloud/updater.rs:885-907`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L885-L907), [`src/cloud/updater.rs:977`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L977)
- **Dowód z kodu:**
  1. Na Windows `install_update` (`updater.rs:737`) zapisuje pobrany plik jako `.SPLogbook_update_{pid}.tmp`, a `install_via_powershell` (`updater.rs:977`) wywołuje `Expand-Archive -LiteralPath $new -DestinationPath $stage -Force`. W Windows PowerShell 5.1 (`powershell.exe`) cmdlet `Expand-Archive` odrzuca pliki bez rozszerzenia `.zip` błędem `The path '...' is not a supported archive file format. .zip is the only supported format`. Ponadto `select_asset_for_platform` (`updater.rs:660`) preferuje `.exe` przed `.zip`.
  2. Na Linux workflow `.github/workflows/build-and-release.yml:83, 113` pakuje archiwum z katalogiem nadrzędnym `SPLogbook-Linux-x86_64/splogbook`, podczas gdy `install_via_tar_gz` (`updater.rs:890-903`) wykonuje tylko `path.strip_prefix(&staging_dir)`, przenosząc plik do `base_dir/SPLogbook-Linux-x86_64/splogbook` zamiast nadpisać `base_dir/splogbook`. Dodatkowo `select_asset_for_platform` (`updater.rs:664`) preferuje `.appimage` przed `.tar.gz`, co powoduje błąd `GzDecoder`.
- **Opis problemu:** Po pobraniu i zweryfikowaniu aktualizacji etap instalacji (`install_update`) nie może zakończyć się sukcesem na żadnej z docelowych platform.
- **Scenariusz wystąpienia:** Użytkownik klika „Pobierz i zainstaluj aktualizację” w oknie nowej wersji.
- **Wpływ:** Całkowita niezdolność aplikacji do automatycznej aktualizacji.
- **Rozwiązanie minimalne:**
  - W `select_asset_for_platform`: dla Windows wybierać `.zip`, dla Linux `.tar.gz`.
  - Na Windows zapisywać pobrane archiwum z rozszerzeniem `.zip` (`.SPLogbook_update_{pid}.zip`).
  - W `install_via_tar_gz` (oraz po rozpakowaniu `.zip` na Windows) wykrywać pojedynczy katalog nadrzędny wewnątrz `staging_dir` i spłaszczać ścieżkę przed `fs::rename`.
- **Rozwiązanie docelowe:** Ujednolicona rozpakowywarka w czystym Rust (np. `zip` + `tar`/`flate2`) z walidacją obecności pliku wykonywalnego przed podmianą.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** `SEC-001`, `CI-001`.
- **Wymagany test regresyjny:** `updater_install_flattens_single_root_directory_in_archive` oraz `updater_windows_temp_archive_has_zip_extension`.
- **Kryterium akceptacji:** Test integracyjny aktualizacji rozpakowuje archiwum o strukturze `SPLogbook-*/splogbook(.exe)` bezpośrednio do katalogu docelowego.

---

### `SEC-002` — Wyciek sekretów przez `#[derive(Debug)]` na `AppConfig` oraz hasła w parametrach query URL
- **ID:** `SEC-002`
- **Tytuł:** Brak redakcji sekretów w `Debug for AppConfig` i wyciek haseł w błędach `reqwest::Error`
- **Kategoria:** `SEC`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
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

---

### `DATA-004` — Operacje niszczące (usuwanie duplikatów i usuwanie zaznaczonych QSO) działają bez potwierdzenia, bez Undo i w poprzek wszystkich dzienników
- **ID:** `DATA-004`
- **Tytuł:** Nieodwracalne usuwanie QSO bez okna potwierdzenia, bez zapisu na stosie `undo_stack` oraz usuwanie duplikatów z obcych dzienników
- **Kategoria:** `DATA`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/gui/find_duplicates.rs`, `src/core/database/search.rs`, `src/gui/app_qso.rs`, `src/gui/state/log_view.rs`
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
- **Moduł / obszar:** `src/core/database/schema.rs`, `src/core/database/qso_store.rs`
- **Lokalizacja:** [`src/core/database/schema.rs:76-86`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/schema.rs#L76-L86), [`src/core/database/qso_store.rs:208, 252, 254, 278, 310, 312`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qso_store.rs#L208-L315)
- **Dowód z kodu:**
  1. Tabela `upload_queue` jest tworzona w `schema.rs:76-86`, ale w całym repozytorium nie ma ani jednego zapytania `INSERT`, `SELECT` ani `DELETE` odwołującego się do `upload_queue`.
  2. `cargo check` i `cargo test` emitują 6 ostrzeżeń kompilatora w `src/core/database/qso_store.rs` (linie 208, 252, 254, 278, 310, 312) z powodu pustych linii po znaku kontynuacji `\` w literałach SQL.
- **Opis problemu:** Szum kompilacyjny i niepodłączona tabela kolejki wysyłek chmurowych.
- **Scenariusz wystąpienia:** Każda kompilacja projektu (`cargo check`, `cargo test`, `cargo clippy`).
- **Wpływ:** Zaciemnienie wyjścia kompilatora przez 6 ostrzeżeń leksera; brak trwałej kolejki retry dla nieudanych wysyłek QSL do chmury.
- **Rozwiązanie minimalne:** Usunąć puste linie po `\` w literałach SQL w `src/core/database/qso_store.rs`.
- **Rozwiązanie docelowe:** Podłączyć `upload_queue` do automatycznego ponawiania wysyłek Cloudlog/ClubLog/QRZ lub udokumentować tabelę jako zarezerwowaną pod przyszłą kolejkę offline.
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
- **Moduł / obszar:** `src/core/cabrillo.rs`
- **Lokalizacja:** [`src/core/cabrillo.rs:27-88`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/cabrillo.rs#L27-L88)
- **Dowód z kodu:**
  1. **Odwrócona kolejność i numeracja QSO (`cabrillo.rs:58`)**: `for (idx, qso) in input.qsos.iter().rev().enumerate()` — jeśli przekazana lista jest już posortowana chronologicznie (lub odwrotnie), brak jawnego sortowania po `(qso_date, time_on)` powoduje przypisanie numeru seryjnego `001` do niewłaściwej łączności.
  2. **Ignorowanie raportów `rst_sent` / `rst_rcvd` (`cabrillo.rs:65`)**: `let rst = if mode_cab == "CW" { "599" } else { "59" };` — eksporter ignoruje rzeczywiste raporty zapisane w `qso.rst_sent` i `qso.rst_rcvd`!
  3. **Błędne mapowanie emisji (`cabrillo.rs:60-64`)**: `if qso.mode == "CW" { "CW" } else { "PH" }` — wszystkie łączności `RTTY`, `FT8`, `FT4`, `PSK` i `FM` są eksportowane jako `"PH"` (Phone/SSB) zamiast `"RY"` / `"DG"` / `"FM"`!
  4. **Ignorowanie `stx_string` / `srx_string` (`cabrillo.rs:68-73`)**: Wymiana jest zawsze formatowana jako `"{idx + 1:03}"`, całkowicie ignorując `qso.stx_string` i `qso.srx_string` (np. strefę CQ w CQ WW, województwo w SP DX Contest, sekcję/moc w ARRL DX, strefę ITU w IARU HF)!
  5. **Błędny fallback częstotliwości (`cabrillo.rs:42-55`)**: Pasma `60m`, `30m`, `17m`, `12m`, `6m`, `2m`, `70cm` przy braku `qso.freq` wpadają w `_ => "14000"` (20m!).
- **Opis problemu:** Wyeksportowany plik Cabrillo zostanie odrzucony przez roboty sędziowskie większości zawodów (CQ WW, SP DX, ARRL DX, IARU HF, RTTY/FT8).
- **Scenariusz wystąpienia:** Eksport logu Cabrillo po zawodach z okna Contest lub menu głównego.
- **Wpływ:** Dyskwalifikacja lub odrzucenie logu konkursowego przez komisję sędziowską.
- **Rozwiązanie minimalne:**
  1. Sortować kopię `qsos` rosnąco po `(qso.adif_date(), qso.adif_time())`.
  2. Mapować emisje: `CW -> "CW"`, `USB|LSB|SSB|AM -> "PH"`, `FM -> "FM"`, `RTTY -> "RY"`, pozostałe cyfrowe -> `"DG"`.
  3. Używać `qso.rst_sent` i `qso.rst_rcvd` (z fallbackiem `599`/`59`).
  4. Używać `qso.stx_string` (lub `qso.stx`) oraz `qso.srx_string` (lub `qso.srx`) jako pól wymiany.
  5. Uzupełnić mapowanie wszystkich pasm ADIF na częstotliwości Cabrillo (`50`, `144`, `432`, `1296` itd.).
- **Rozwiązanie docelowe:** Formatowanie wiersza `QSO:` sterowane przez profil `ContestId` z `src/core/contest_rules.rs`.
- **Ryzyko wdrożenia zmiany:** Niskie (wymaga aktualizacji testu w `cabrillo.rs:106`, który obecnie sam wymusza błędne założenia).
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Powiązane z `FUNC-002`.
- **Wymagany test regresyjny:** `cabrillo_export_preserves_rst_digital_modes_and_custom_contest_exchanges`.
- **Kryterium akceptacji:** Logi dla CQ WW (strefa CQ), SP DX (województwo/nr seryjny) oraz RTTY/FT8 generują poprawne linie `QSO:` zgodne ze specyfikacją Cabrillo v3.

---

### `FUNC-002` — Błędne reguły punktacji i mnożników w `src/core/contest_rules.rs` (SP DX Contest, CQ WW, CQ WPX)
- **ID:** `FUNC-002`
- **Tytuł:** Błędna punktacja łączności zagranicznych oraz zliczanie polskich stacji jako mnożników DXCC w SP DX Contest
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/core/contest_rules.rs`
- **Lokalizacja:** [`src/core/contest_rules.rs:81-165`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_rules.rs#L81-L165)
- **Dowód z kodu:**
  1. W `score_sp_dx` (`contest_rules.rs:95-115`): gdy polska stacja (`my_sp == true`) pracuje ze stacją zagraniczną (`their_sp == false`), mnożnikiem powinno być podmiot DXCC stacji zagranicznej, a dla pracy stacji zagranicznej z Polską — polskie województwo (1 z 16). Tymczasem kod w linii `105-114` przy pracy ze stacją SP jako mnożnik zwraca surowy prefiks `extract_wpx_prefix(&qso.callsign)` (np. `SP6`, `SO9`) zamiast województwa z `srx_string`, a gdy `srx_string` zawiera województwo przy stacji SP, liczy stacji polskiej punkty i mnożnik!
  2. W `score_cq_ww` (`contest_rules.rs:120-137`) i `score_cq_wpx` (`contest_rules.rs:140-156`) punktacja opiera się na uproszczonym sprawdzeniu prefiksu `SP`/`K` zamiast porównania `my_continent` vs `qso.continent` oraz `my_dxcc` vs `qso.dxcc` (oraz braku podwojenia punktów na pasmach niskich 160m/80m/40m w CQ WPX).
- **Opis problemu:** Wynik punktowy i liczba mnożników wyświetlana podczas zawodów odbiega od oficjalnych regulaminów PZK i CQ.
- **Scenariusz wystąpienia:** Udział w SP DX Contest, CQ WW lub CQ WPX.
- **Wpływ:** Błędna kalkulacja wyniku na żywo i błędne oznaczanie nowego mnożnika (`New Mult`).
- **Rozwiązanie minimalne:** Przekazywać kontekst stacji własnej (`my_dxcc`, `my_continent`, `my_sp_province`) do funkcji `score_*` w `contest_rules.rs` i walidować polskie województwa (`B, C, D, F, G, J, K, L, M, O, P, R, S, U, W, Z`) dla SP DX Contest.
- **Rozwiązanie docelowe:** Pełna tabela punktacji kontynentalnej i pasmowej (160m–10m) w `ContestEngine`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `M`
- **Zależności od innych zmian:** `FUNC-001`.
- **Wymagany test regresyjny:** `sp_dx_contest_scoring_for_sp_and_non_sp_stations_matches_pzk_rules`.
- **Kryterium akceptacji:** Punktacja dla stacji SP (1 pkt EU, 3 pkt DX, mnożnik DXCC per pasmo) oraz stacji non-SP (3 pkt za SP, mnożnik województwo A..Z per pasmo) jest zgodna z regulaminem SP DX Contest.

---

### `FUNC-003` — Endpoint `POST /api/v1/rig` w REST API jest atrapą (mutuje tylko RAM i jest nadpisywany w kolejnej klatce GUI)
- **ID:** `FUNC-003`
- **Tytuł:** `post_rig_control` (`POST /api/v1/rig`) nie wysyła żadnych rozkazów do `CatRuntime` ani `RigHub`
- **Kategoria:** `FUNC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
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
- **Kryterium akceptacji:** Odrzucenie PTT przez `TxArbiter` zwraca `RPRT -1` w `HamlibProxyServer` i zatrzymuje odtwarzanie audio w `VoiceKeyer`.

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
- **ID:** `CAT-013`
- **Tytuł:** `FlexRadioBackend::execute_routed` nie wywołuje `wait_for_response(seq)` dla setterów, a `SetSplit` jest pustym blokiem (`no-op`)
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/native/flexradio/backend.rs`
- **Lokalizacja:** [`src/cat/native/flexradio/backend.rs:914-1027, 1080-1126`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L914-L1126)
- **Dowód z kodu:**
  1. Dla `SetFrequency`, `SetSplitFrequency`, `SetMode`, `SetPassband`, `SetPower`, `SetRit` i `SetXit` metoda `execute_routed` wywołuje tylko `self.send_command_tracked(cmd).await?` bez `self.wait_for_response(seq).await?`.
  2. W `RigCommand::SetSplit { enabled, .. }` (`flexradio/backend.rs:1021-1027`) po sprawdzeniu `self.slices.len() < 2` znajduje się pusty blok `{}` zwracający `Ok(())` bez wysłania żadnej komendy do SmartSDR!
- **Opis problemu:** Ignorowanie błędów `R<seq>|5000xxxx|` zwracanych przez SmartSDR oraz całkowity brak działania `SetSplit` na FlexRadio.
- **Scenariusz wystąpienia:** Włączenie Splitu lub wysłanie komendy zmiany emisji/częstotliwości do FlexRadio.
- **Wpływ:** `SetSplit` nic nie robi na FlexRadio; błędy wykonania komend SmartSDR są po cichu ignorowane.
- **Rozwiązanie minimalne:** Dodać `let resp = self.wait_for_response(seq).await?; if !resp.is_success() { return Err(...); }` we wszystkich gałęziach `execute_routed`, a w `SetSplit` wysyłać `slice set <tx_slice_idx> tx=1`.
- **Rozwiązanie docelowe:** Pełna obsługa tworzenia/przełączania slice B przy `SetSplit(true)`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `CAT-002`.
- **Wymagany test regresyjny:** `flexradio_set_split_sends_slice_tx_command_and_awaits_r_seq_ack`.
- **Kryterium akceptacji:** Każdy setter w `FlexRadioBackend` weryfikuje kod odpowiedzi `R<seq>|0|`, a `SetSplit` przełącza flagę `tx=1` na docelowym slice.

---

### `CAT-014` — Odcięcie `NativeRigCommand` i `NativeRigEvent` od `RigService` i `RigHub` oraz martwy kod legacy (`CatController`, `CatBackendKind::So2r`)
- **ID:** `CAT-014`
- **Tytuł:** Brak mostu dla `NativeRigCommand`/`NativeRigEvent` w `RigService` oraz martwy `CatController` i `CatBackendKind::So2r` w `src/cat/backend.rs`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
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

### `CAT-017` — Blokujące `std::thread::sleep` (do 255 ms) w `RigctldSupervisor` wywoływane z głównego wątku GUI oraz drobne niespójności profili Yaesu/Elecraft
- **ID:** `CAT-017`
- **Tytuł:** Przycinanie wątku GUI przez `std::thread::sleep` w `RigctldSupervisor` oraz niespójność `passband` w `create_custom_yaesu_profile`
- **Kategoria:** `CAT`
- **Poziom krytyczności:** `MEDIUM`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/cat/supervisor.rs`, `src/cat/native/yaesu/models.rs`, `Native CAT.md`
- **Lokalizacja:** [`src/cat/supervisor.rs:316, 375, 385`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/supervisor.rs#L316-L385), [`src/cat/native/yaesu/models.rs:85`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/models.rs#L85), [`Native CAT.md:389`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Native%20CAT.md#L389)
- **Dowód z kodu:**
  1. `RigctldSupervisor::start` i `stop` wykonują synchroniczne `std::thread::sleep` (łącznie do 255 ms) na wątku GUI podczas `start_cat_service` / `stop_cat_service`.
  2. `create_custom_yaesu_profile` (`yaesu/models.rs:85`) ustawia `passband: true`, podczas gdy `YaesuBackend::execute(SetPassband)` zwraca `Unsupported`.
  3. `Native CAT.md:389` podaje dla KX2 `supports_vfo_b_modifier: false`, podczas gdy `elecraft/models.rs:299` ustawia `true`.
- **Opis problemu:** Chwilowe zamrożenie GUI przy restarcie CAT oraz drobna rozbieżność flag możliwości.
- **Scenariusz wystąpienia:** Kliknięcie „Restart CAT” w ustawieniach.
- **Wpływ:** Zauważalne przycięcie interfejsu (ok. 15 straconych klatek przy 60 FPS).
- **Rozwiązanie minimalne:** Ustawić `passband: false` w `create_custom_yaesu_profile`, zaktualizować `Native CAT.md:389` i uruchamiać `RigctldSupervisor` w `tokio::task::spawn_blocking`.
- **Rozwiązanie docelowe:** Asynchroniczny nadzorca procesu oparty na `tokio::process::Command`.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `custom_yaesu_profile_capabilities_match_backend_execute_support`.
- **Kryterium akceptacji:** Brak blokowania wątku GUI przez `std::thread::sleep` przy starcie/zatrzymaniu `rigctld`.

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

---

### `CONC-001` — Blokowanie wątków roboczych Tokio przez synchroniczne operacje SQLite (`db.lock()`) bez `spawn_blocking` w `online_sync.rs`
- **ID:** `CONC-001`
- **Tytuł:** Synchroniczne blokowanie `std::sync::Mutex<Database>` i pętla 1000 pojedynczych transakcji SQLite wewnątrz `tokio::spawn` w `src/gui/online_sync.rs`
- **Kategoria:** `CONC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
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

### `PERF-001` — Kwadratowy algorytm $O(N^2)$ klonowania i serializacji JSON w `src/sync/catchup.rs` (`chunk_inventory`, `chunk_requests`, `chunk_records`)
- **ID:** `PERF-001`
- **Tytuł:** Kwadratowa złożoność $O(N^2)$ w `chunk_inventory`, `chunk_requests` i `chunk_records` serializująca całe wektory JSON w każdej iteracji pętli
- **Kategoria:** `PERF`
- **Poziom krytyczności:** `CRITICAL`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/sync/catchup.rs`
- **Lokalizacja:** [`src/sync/catchup.rs:32-50, 77-94, 123-153`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/catchup.rs#L32-L153)
- **Dowód z kodu i testów:**
  - W `chunk_inventory` (`src/sync/catchup.rs:32-50`) dla każdego elementu `id` z `identities` wykonywane jest:
    `current.push(id.clone()); let msg = Message::Inventory { journal: journal.to_string(), identities: current.clone(), last: false }; if serde_json::to_vec(&msg).map_or(0, |v| v.len()) > MAX_CHUNK_BYTES { ... }`
  - Identyczny wzorzec $O(N^2)$ występuje w `chunk_requests` (`catchup.rs:77-94`) oraz `chunk_records` (`catchup.rs:138-152`).
  - Dla 15 000 elementów funkcja klonuje i serializuje do JSON narastający wektor 15 000 razy (łącznie ~68,5 miliona serializacji obiektów `QsoIdentity`, czyli ~6 GiB wygenerowanego JSON-a dla jednego wywołania!), przez co 3 testy jednostkowe `each_*_chunk_is_at_most_one_mib` wykonywały się przez **ponad 500 sekund**!
- **Opis problemu:** Przy połączeniu stacji w sieci LAN Sync z dziennikiem liczącym kilkanaście tysięcy łączności procedura Catch-Up zamraża wątek na kilkadziesiąt sekund lub kilka minut, alokując gigabajty pamięci tymczasowej.
- **Scenariusz wystąpienia:** Połączenie Peer <-> Host w LAN Sync dla dziennika z $\ge 5000$ QSO oraz każde uruchomienie `cargo test`.
- **Wpływ:** Drastyczne spowolnienie synchronizacji Catch-Up w LAN Sync oraz wydłużenie czasu wykonania `cargo test` z ~15 sekund do ~9 minut!
- **Rozwiązanie minimalne:** Zliczać rozmiar bieżącej porcji przyrostowo w $O(N)$: serializować pojedynczy element `let item_len = serde_json::to_vec(id).map_or(0, |v| v.len());` i dodawać `item_len + 1` (przecinek) do `current_bytes` (z narzutem nagłówka koperty JSON ~64 B), wykonując pełną konstrukcję `Message` wyłącznie przy zamknięciu porcji 1 MiB.
- **Rozwiązanie docelowe:** Liniowy algorytm $O(N)$ z akumulatorem bajtów i opcjonalną weryfikacją końcową porcji.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `each_inventory_chunk_is_at_most_one_mib_completes_under_200ms`.
- **Kryterium akceptacji:** Czas wykonania `each_inventory_chunk_is_at_most_one_mib`, `each_request_chunk_is_at_most_one_mib` i `each_records_chunk_is_at_most_one_mib` spada z >500 s do <0,2 s przy zachowaniu limitu $\le 1\text{ MiB}$ na każdy chunk.

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

---

## 7. Testy (`TEST-*`), Dokumentacja (`DOC-*`), CI/CD (`CI-*`) i Zależności (`DEP-*`)

### `TEST-001` — Czas wykonania zestawu testów jednostkowych wynosi ~532 sekundy (~9 minut) w profilu debug z powodu 4 testów w `src/sync/mod.rs`
- **ID:** `TEST-001`
- **Tytuł:** Ekstremalnie wolne wykonanie `cargo test` (532 s) spowodowane przez $O(N^2)$ w `catchup.rs` i 33 wywołania Argon2id w `client_limit_is_enforced`
- **Kategoria:** `TEST`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `src/sync/mod.rs`, `src/sync/catchup.rs`, `src/sync/crypto.rs`
- **Lokalizacja:** [`src/sync/mod.rs:851-904, 1650-1741`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/mod.rs#L851-L1741), [`src/sync/crypto.rs:34-46`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/crypto.rs#L34-L46)
- **Dowód z narzędzi:** Podczas audytu wykonanie `splogbook` unit tests (983 testy) zajęło **532,05 s**, z czego 979 testów zakończyło się w ~15 sekund, a 4 testy (`sync::tests::client_limit_is_enforced`, `each_inventory_chunk_is_at_most_one_mib`, `each_request_chunk_is_at_most_one_mib`, `each_records_chunk_is_at_most_one_mib`) przekroczyły próg 60 sekund i trwały ponad 8,5 minuty.
- **Opis problemu:**
  1. Trzy testy `each_*_chunk_is_at_most_one_mib` cierpią na kwadratowy algorytm $O(N^2)$ w `src/sync/catchup.rs` (`PERF-001`).
  2. Test `client_limit_is_enforced` tworzy 33 klientów wykonujących w pętli `peer_handshake` -> `derive_key` z produkcyjnymi parametrami Argon2id (`64 MiB`, 3 iteracje) bez optymalizacji kompilatora (`opt-level = 0`).
- **Scenariusz wystąpienia:** Każde uruchomienie `cargo test` lokalnie lub w CI.
- **Wpływ:** Poważne spowolnienie cyklu deweloperskiego i CI/CD.
- **Rozwiązanie minimalne:**
  1. Naprawić algorytm $O(N)$ w `src/sync/catchup.rs` (`PERF-001`).
  2. Dodać w `Cargo.toml` sekcję `[profile.dev.package.argon2] opt-level = 3` oraz `[profile.dev.package.chacha20poly1305] opt-level = 3` (standardowa praktyka w Rust dla kryptografii w testach, przyspieszająca Argon2id ~20-krotnie bez osłabiania parametrów bezpieczeństwa!).
- **Rozwiązanie docelowe:** Optymalizacja `catchup.rs` + `opt-level = 3` dla crate'ów kryptograficznych w profilu `dev`/`test`.
- **Ryzyko wdrożenia zmiany:** Zerowe.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `PERF-001`.
- **Wymagany test regresyjny:** Pomiar czasu `cargo test --lib` (< 20 s).
- **Kryterium akceptacji:** Pełny zestaw 983 testów jednostkowych wykonuje się w czasie poniżej 25 sekund.

---

### `DOC-001` — Rozbieżności pomiędzy `README.md` a rzeczywistym stanem kodu (baza klubów, Terminal CW/WinKeyer, JS8Call, FLDigi, zmyślony e-mail `contact@splogbook.org`)
- **ID:** `DOC-001`
- **Tytuł:** Niespójności deklaracji funkcjonalnych w `README.md` względem rzeczywistej implementacji
- **Kategoria:** `DOC`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `README.md`, `Native CAT.md`
- **Lokalizacja:** [`README.md:176-184, 545, 658, 683`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/README.md#L176-L683), [`Native CAT.md:389`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Native%20CAT.md#L389)
- **Dowód z kodu:**
  1. `README.md:176-184` opisuje automatyczne sprawdzanie wbudowanej bazy międzynarodowych klubów (SP-OTC, SPCWC, CWOPS, FOC, SKCC, HSC), podczas gdy `src/core/clubs.rs` ma zahardkodowane tylko 11 znaków i nie zawiera SKCC (`FUNC-006`).
  2. `README.md:545, 658` opisuje obsługę portu szeregowego dla WinKeyera, podczas gdy `src/cat/winkeyer.rs` nie posiada warstwy I/O portu szeregowego (`FUNC-004`).
  3. `Native CAT.md:389` zawiera nieaktualny wpis tabeli dla KX2 (`supports_vfo_b_modifier: false` vs `true` w kodzie).
- **Opis problemu:** Dokumentacja przypisuje pełną gotowość produkcyjną modułom będącym jeszcze w stadium makiety lub częściowego podłączenia.
- **Scenariusz wystąpienia:** Lektura `README.md` przez użytkownika lub audytora.
- **Wpływ:** Rozbieżność oczekiwań użytkownika ze stanem faktycznym programu.
- **Rozwiązanie minimalne:** Zaktualizować `README.md` oraz `Native CAT.md:389` (albo dokończyć podłączenie `winkeyer.rs`, `js8call.rs` i `clubs.rs` zgodnie z dokumentacją).
- **Rozwiązanie docelowe:** Pełna zgodność dokumentacji i kodu po wdrożeniu poprawek `FUNC-004` i `FUNC-006`.
- **Ryzyko wdrożenia zmiany:** Zerowe.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `FUNC-004`, `FUNC-006`.
- **Wymagany test regresyjny:** Przegląd zgodności sekcji funkcji w `README.md`.
- **Kryterium akceptacji:** Każda funkcja wymieniona w `README.md` posiada kompletny, przetestowany przepływ end-to-end.

---

### `CI-001` — Workflow `.github/workflows/build-and-release.yml` nie generuje ani nie podpisuje `release-manifest.json` wymaganego przez auto-updater
- **ID:** `CI-001`
- **Tytuł:** Brak kroku generowania i podpisywania `release-manifest.json` / `release-manifest.json.sig` w potoku wydawniczym GitHub Actions
- **Kategoria:** `CI`
- **Poziom krytyczności:** `HIGH`
- **Poziom pewności:** `CONFIRMED`
- **Moduł / obszar:** `.github/workflows/build-and-release.yml`, `src/bin/splogbook_signer.rs`, `src/cloud/updater.rs`
- **Lokalizacja:** [`.github/workflows/build-and-release.yml:95-120`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/build-and-release.yml#L95-L120), [`src/cloud/updater.rs:582-655`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L582-L655)
- **Dowód z kodu:** `UpdaterClient::fetch_and_verify_manifest` (`src/cloud/updater.rs:582-655`) wymaga obecności plików `release-manifest.json` oraz `release-manifest.json.sig` w zasobach wydania GitHub Release (w przeciwnym razie zwraca `UpdaterError::MissingManifestAsset`). Tymczasem `.github/workflows/build-and-release.yml:108-116` publikuje wyłącznie `SPLogbook-Windows-x86_64.exe`, `.zip`, `.tar.gz`, `.deb`, `.AppImage` i `SHA256SUMS.txt` — **nigdy nie uruchamia `splogbook_signer` i nie dołącza `release-manifest.json` ani `.sig`**!
- **Opis problemu:** Żadne wydanie wygenerowane przez obecny workflow CI/CD nie przejdzie weryfikacji wbudowanego auto-updatera (`MissingManifestAsset`).
- **Scenariusz wystąpienia:** Publikacja nowego tagu `v*` przez GitHub Actions i sprawdzenie aktualizacji w programie.
- **Wpływ:** Auto-updater zawsze odrzuca wydania zbudowane przez GitHub Actions z błędem braku manifestu.
- **Rozwiązanie minimalne:** Dodać w jobie `release` w `.github/workflows/build-and-release.yml` krok budujący `splogbook_signer`, generujący `release-manifest.json` ze skrótami SHA-256 artefaktów, podpisujący go kluczem z `${{ secrets.RELEASE_SIGNING_KEY }}` i dołączający `release-manifest.json` oraz `release-manifest.json.sig` do `files:` w `softprops/action-gh-release`.
- **Rozwiązanie docelowe:** Zautomatyzowany krok CI z weryfikacją wygenerowanego podpisu przed publikacją wydania.
- **Ryzyko wdrożenia zmiany:** Niskie.
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** `SEC-001`, `REL-001`.
- **Wymagany test regresyjny:** Krok weryfikacji `splogbook_signer` w CI sprawdzający poprawność wygenerowanej pary `release-manifest.json` + `.sig`.
- **Kryterium akceptacji:** Workflow wydawniczy dołącza zweryfikowany `release-manifest.json` i `release-manifest.json.sig` do każdego GitHub Release.

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
- **Moduł / obszar:** `Cargo.lock`, `Cargo.toml`
- **Lokalizacja:** [`Cargo.lock`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Cargo.lock)
- **Dowód z narzędzi (`cargo audit`):**
  1. `ctutils 0.4.2`: `RUSTSEC-2026-0385` (Non-Constant-Time Execution in `CtAssignSlice` — naprawione w `>=0.4.3`).
  2. `rustls 0.23.45`: `RUSTSEC-2026-0346`, `RUSTSEC-2026-0347`, `RUSTSEC-2026-0348`, `RUSTSEC-2026-0349` (4 biuletyny bezpieczeństwa — naprawione w `>=0.23.50`).
  3. `rustls-webpki 0.103.15`: `RUSTSEC-2026-0382` (naprawione w `>=0.103.16`).
  4. `webpki-root-certs 1.0.9`: `RUSTSEC-2026-0383` (nieprawidłowe zakotwiczenia zaufania — naprawione w `>=1.0.10`).
  5. `tokio 1.53.1`: `RUSTSEC-2026-0386` (unsound: `NamedPipeClient::ready` can trigger UB when signaling readiness for both read and write).
  6. Wycofane (yanked) wersje w `Cargo.lock`: `clipboard-win 5.4.1`, `yoke-derive 0.8.3`.
- **Opis problemu:** Przestarzałe wersje pośrednich zależności kryptograficznych i sieciowych w `Cargo.lock`.
- **Scenariusz wystąpienia:** Połączenia HTTPS (`reqwest` z `rustls`) oraz działanie runtime'u `tokio` na Windows.
- **Wpływ:** Podatności weryfikacji certyfikatów TLS i potencjalne UB w `tokio` na Windows.
- **Rozwiązanie minimalne:** Wykonać `cargo update -p rustls -p rustls-webpki -p webpki-root-certs -p ctutils -p tokio -p clipboard-win -p yoke-derive`.
- **Rozwiązanie docelowe:** Regularna aktualizacja `Cargo.lock` weryfikowana przez `cargo audit` w CI.
- **Ryzyko wdrożenia zmiany:** Bardzo niskie (aktualizacje wersji patch/minor w obrębie tych samych gałęzi SemVer).
- **Złożoność wdrożenia:** `S`
- **Zależności od innych zmian:** Brak.
- **Wymagany test regresyjny:** `cargo audit` zwracające `0 vulnerabilities found`.
- **Kryterium akceptacji:** `cargo audit` nie zgłasza żadnych podatności `RUSTSEC-*` ani ostrzeżeń `unsound`.
