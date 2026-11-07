# Rejestr Znalezisk Audytowych SPLogbook (`AUDIT_FINDINGS_REGISTER.md`)

**Dokument audytowy:** `docs/AUDIT_FINDINGS_REGISTER.md`
**Projekt:** SPLogbook (`v1.2.0`)
**GaĹ‚Ä…Ĺş referencyjna:** `main` (`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`)
**Skala krytycznoĹ›ci:** `BLOCKER`, `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `INFO`
**Skala pewnoĹ›ci:** `CONFIRMED`, `HIGH CONFIDENCE`, `SUSPECTED`, `NOT VERIFIED`

---

## Podsumowanie Liczbowe Rejestru

| Kategoria | BLOCKER | CRITICAL | HIGH | MEDIUM | LOW | INFO | Razem |
|---|---|---|---|---|---|---|---|
| `SEC` (BezpieczeĹ„stwo) | 1 | 0 | 0 | 3 | 1 | 0 | **5** |
| `REL` (NiezawodnoĹ›Ä‡ / Updater) | 0 | 3 | 1 | 0 | 0 | 0 | **4** |
| `DATA` (Dane / SQLite / ADIF) | 0 | 4 | 2 | 1 | 0 | 0 | **7** |
| `FUNC` (FunkcjonalnoĹ›Ä‡ / Domena) | 0 | 2 | 4 | 2 | 0 | 0 | **8** |
| `CAT` (Radio / CAT / HIL / PTT) | 0 | 4 | 8 | 4 | 1 | 0 | **17** |
| `ARCH` (Architektura) | 0 | 0 | 2 | 2 | 0 | 0 | **4** |
| `RUST` (JakoĹ›Ä‡ kodu Rust) | 0 | 0 | 1 | 2 | 1 | 0 | **4** |
| `CONC` (WspĂłĹ‚bieĹĽnoĹ›Ä‡ / Async) | 0 | 0 | 2 | 1 | 0 | 0 | **3** |
| `GUI` (Interfejs / UX / A11y) | 0 | 0 | 2 | 3 | 1 | 0 | **6** |
| `PERF` (WydajnoĹ›Ä‡) | 0 | 1 | 2 | 1 | 0 | 0 | **4** |
| `TEST` (Testy / QA) | 0 | 0 | 1 | 2 | 0 | 0 | **3** |
| `DOC` (Dokumentacja vs Kod) | 0 | 0 | 1 | 2 | 1 | 0 | **4** |
| `CI` (CI/CD i Release) | 0 | 0 | 2 | 1 | 0 | 0 | **3** |
| `DEP` (ZaleĹĽnoĹ›ci i Licencje) | 0 | 0 | 1 | 1 | 1 | 1 | **4** |
| **RAZEM** | **1** | **14** | **29** | **25** | **6** | **1** | **76** |

---

## 1. BezpieczeĹ„stwo (`SEC-*`) i NiezawodnoĹ›Ä‡ Aktualizacji (`REL-*`)

### `SEC-001` â€” Jawny klucz prywatny Ed25519 (`[1u8; 32]`) w Ĺ›ledzonym pliku `src/scratch.rs` odpowiadajÄ…cy produkcyjnemu kluczowi weryfikacji aktualizacji
- **ID:** `SEC-001`
- **TytuĹ‚:** Skompromitowany klucz prywatny Ed25519 podpisujÄ…cy manifesty auto-updatera w repozytorium Git
- **Kategoria:** `SEC`
- **Poziom krytycznoĹ›ci:** `BLOCKER`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status po Etapie 1B (SEC-001):**
  - **REMEDIATED_IN_TREE:** FIXED (materiaĹ‚ prywatny usuniÄ™ty z drzewa roboczego, brak Ĺ›ledzonych plikĂłw kluczy);
  - **REGRESSION_GUARDED:** FIXED (pakiet 17 testĂłw regresyjnych `tests/security_guard.rs` oraz CI `.github/workflows/ci.yml` wymuszajÄ… fail-closed i brak prywatnego materiaĹ‚u);
  - **LEGACY_KEYS_REVOKED:** FIXED (`splogbook-release-2026-01` oraz 5 odwoĹ‚anych kluczy publicznych na staĹ‚e w `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS`);
  - **NEW_PUBLIC_KEY_ACTIVE:** FIXED (aktywny dokĹ‚adnie jeden klucz `splogbook-release-2026-02`, fingerprint SHA-256: `2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a`);
  - **PRODUCTION_SECRET_STORED_EXTERNALLY:** OWNER_CONFIRMED (klucz prywatny wygenerowany poza repozytorium i skonfigurowany w GitHub Actions Secret `RELEASE_SIGNING_KEY`);
  - **HISTORY_EXPOSURE:** OWNER_DECISION_REQUIRED (materiaĹ‚y historyczne zneutralizowane kryptograficznie w aplikacji; ewentualne przepisywanie historii Git `git-filter-repo` pozostaje do wyĹ‚Ä…cznej decyzji WĹ‚aĹ›ciciela);
  - **HISTORICAL_RELEASE_AUDIT:** COMPLETED (przeprowadzono audyt wszystkich wydaĹ„: v1.2.0 sklasyfikowane jako `VERIFIED_REVOKED_KEY`, v1.0.0-v1.1.0 jako `MISSING_MANIFEST` / `UNSIGNED`; peĹ‚ny raport: `docs/HISTORICAL_RELEASE_AUDIT.md`);
  - **AUTO_UPDATE_RELEASE_READINESS:** READY (wdroĹĽono kompletne zabezpieczenia REL-001 oraz potok CI-001 w Etapie 2; auto-updater weryfikuje podpisy, potok publikuje podpisany manifest i SHA256SUMS.txt);
  - **SEC-001_OVERALL:** MITIGATED (kod, testy, klucze i audyt domkniÄ™te; ekspozycja historii oczekuje na decyzjÄ™ WĹ‚aĹ›ciciela).
- **ModuĹ‚ / obszar:** `src/scratch.rs`, `src/cloud/updater.rs`, `tests/security_guard.rs`, `.github/workflows/ci.yml`, `docs/SECURITY_KEY_ROTATION.md`
- **Lokalizacja:** [`src/cloud/updater.rs:257-300`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs#L257-L300) (`TRUSTED_KEYS`, `REVOKED_KEY_IDS`, `REVOKED_PUBLIC_KEYS`), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs), [`docs/SECURITY_KEY_ROTATION.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/SECURITY_KEY_ROTATION.md)
- **DowĂłd z kodu / Git:**
  - W commicie `80533e2` wprowadzono Ĺ›ledzone pliki kluczy `.signer_priv.key`, `.test.key`, `klucz.key` oraz `src/scratch.rs`. Plik `klucz.key` odpowiadaĹ‚ historycznemu publicznemu kluczowi `[203, 142, 210, 212...]`.
  - W commicie `54db3e0` podmieniono klucz publiczny na `[109, 72, 1, 34...]` z zachowaniem identyfikatora `splogbook-release-2026-01`, bez mechanizmu wycofania starego klucza.
  - Plik `src/scratch.rs` zawieraĹ‚ jawny kod wyliczajÄ…cy klucz ze staĹ‚ego ziarna `[1u8; 32]`.
- **Opis problemu:** Ujawnienie materiaĹ‚u kryptograficznego w repozytorium uniewaĹĽnia kryptograficzne zaufanie do wydaĹ„ podpisywanych historycznymi kluczami.
- **Scenariusz wystÄ…pienia:** AtakujÄ…cy posiadajÄ…cy skompromitowany klucz prywatny mĂłgĹ‚by spreparowaÄ‡ podpisany manifest dla zĹ‚oĹ›liwego pliku aktualizacji.
- **WpĹ‚yw:** CaĹ‚kowite uniewaĹĽnienie ochrony kryptograficznej Ed25519 w auto-updaterze przed wdroĹĽeniem mechanizmu revocation.
- **WdroĹĽone rozwiÄ…zanie (Etap 1A):** UsuniÄ™to `src/scratch.rs` z drzewa, zaktualizowano `.gitignore`, dodano listÄ™ `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS` w `src/cloud/updater.rs`, wprowadzono stan fail-closed (`active: false`), dodano testy regresyjne `tests/security_guard.rs`, dodano krok weryfikacji w CI oraz udokumentowano procedurÄ™ w `docs/SECURITY_KEY_ROTATION.md`.
- **Wymagane dziaĹ‚anie wĹ‚aĹ›ciciela (Etap 1B):** RÄ™czne wygenerowanie nowego klucza Ed25519 poza repozytorium, umieszczenie klucza prywatnego w GitHub Secrets (`RELEASE_SIGNING_KEY`) oraz dostarczenie klucza publicznego do wdroĹĽenia w `TRUSTED_KEYS`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie (updater fail-closed do czasu dostarczenia nowego klucza).
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `REL-001` i `CI-001`.
- **WdroĹĽone testy regresyjne:**
  - `updater_trusted_keys_reject_known_test_and_trivial_seeds` w `tests/security_guard.rs`
  - `repository_contains_no_tracked_private_key_material` w `tests/security_guard.rs`
- **Kryterium akceptacji Etapu 1A:** SpeĹ‚nione â€” brak `src/scratch.rs`, brak Ĺ›ledzonych kluczy prywatnych, klucz `splogbook-release-2026-01` odwoĹ‚any, testy przechodzÄ… pomyĹ›lnie, status udokumentowany.

---

### `REL-001` â€” Auto-updater koĹ„czy siÄ™ bĹ‚Ä™dem instalacji zarĂłwno na Windows, jak i na Linux
- **ID:** `REL-001`
- **TytuĹ‚:** NiedziaĹ‚ajÄ…cy proces instalacji aktualizacji (`install_update`) na Windows i Linux
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED / VERIFIED_NATIVE_CI**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commity:**
  - `9ac1e5b`: `fix(updater): secure archive extraction and platform staging (REL-001)`
  - `cdcb955`: `fix(updater): make installation atomic with rollback (REL-001)`
  - `9557db8`: `fix(updater): validate Windows ZIP entries before extraction (REL-001)`
  - `3ebd8ac`: `fix(updater): harden transactional installation rollback (REL-001)`
- **Weryfikacja natywna (Etap 2 KoĹ„cowa Bramka):**
  - **Natywne testy Windows (job `124864486955`, 28m30s na `windows-latest`):** PeĹ‚na weryfikacja bezpieczeĹ„stwa rozpakowywania ZIP przed ekstrakcjÄ… (`validate_zip_archive`), odrzucanie path traversal, Ĺ›cieĹĽek bezwzglÄ™dnych, prefiksĂłw dyskĂłw, Ĺ›cieĹĽek UNC i symlinkĂłw (`windows_production_zip_path_*`), oraz weryfikacja semantyki `TRANSACTIONAL_FILE_REPLACEMENT` z atomowym rollbackiem kaĹĽdego zastÄ…pionego pliku. Status: PASS (100%).
  - **Natywne testy Linux (job `124864487116`, 18m52s na `ubuntu-latest`):** Bezpieczna ekstrakcja `.tar.gz`, spĹ‚aszczanie pojedynczego katalogu gĹ‚Ăłwnego oraz czyste uprawnienia `0o755`/`0o644`. Status: PASS (100%).
- **ModuĹ‚ / obszar:** `src/cloud/updater.rs`, `.github/workflows/build-and-release.yml`, `docs/RELEASE_SECURITY.md`
- **Lokalizacja:** [`src/cloud/updater.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs), [`.github/workflows/build-and-release.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/build-and-release.yml)
- **DowĂłd z kodu przed naprawÄ…:**
  1. Na Windows `install_update` zapisywaĹ‚o pobrany plik jako `.SPLogbook_update_{pid}.tmp`, a `install_via_powershell` wywoĹ‚ywaĹ‚o `Expand-Archive -LiteralPath $new -DestinationPath $stage -Force`. W Windows PowerShell 5.1 (`powershell.exe`) cmdlet `Expand-Archive` odrzucaĹ‚ pliki bez rozszerzenia `.zip` bĹ‚Ä™dem `The path '...' is not a supported archive file format. .zip is the only supported format`. Ponadto `select_asset_for_platform` preferowaĹ‚o `.exe` przed `.zip`.
  2. Na Linux workflow pakowaĹ‚ archiwum z katalogiem nadrzÄ™dnym `SPLogbook-Linux-x86_64/splogbook`, podczas gdy updater przenosiĹ‚ plik do `base_dir/SPLogbook-Linux-x86_64/splogbook` zamiast nadpisaÄ‡ `base_dir/splogbook`. Dodatkowo `select_asset_for_platform` preferowaĹ‚o `.appimage` przed `.tar.gz`.
- **WdroĹĽone rozwiÄ…zanie (Etap 2):**
  1. W `select_manifest_asset` i `select_asset_for_platform` wymuszono Ĺ›cisĹ‚y dobĂłr `.zip` dla Windows i `.tar.gz` dla Linux na podstawie jawnych pĂłl platformy/architektury.
  2. Pobierany plik na Windows otrzymuje bezpiecznÄ… Ĺ›cieĹĽkÄ™ z rozszerzeniem `.zip` (`.SPLogbook_update_{pid}.zip`).
  3. Bezpieczne rozpakowywanie `extract_tar_gz_safe` oraz `extract_zip_safe_windows` z ochronÄ… przed path traversal (`validate_archive_relative_path`), odrzucaniem symlinkĂłw, hardlinkĂłw i plikĂłw specjalnych oraz limitami `ArchiveLimits`.
  4. WdroĹĽono spĹ‚aszczanie pojedynczego katalogu nadrzÄ™dnego (`flatten_single_root_directory`).
  5. Atomowa podmiana i transakcyjnoĹ›Ä‡ (`apply_atomic_installation_with_options`): walidacja ukĹ‚adu staging (`validate_staging_layout`), kopia zapasowa do `.backup_<random>` w katalogu aplikacji, podmiana plikĂłw, walidacja sprawnoĹ›ci binarnej oraz automatyczny natychmiastowy rollback przy dowolnym bĹ‚Ä™dzie lub awarii.
  6. Wymuszenie czystych uprawnieĹ„ `0o755`/`0o644` bez bitĂłw setuid/setgid/sticky.
- **WdroĹĽone testy regresyjne:**
  - 14 testĂłw bezpieczeĹ„stwa archiwĂłw: `archive_never_writes_outside_staging`, `archive_rejects_parent_traversal`, `archive_rejects_absolute_paths`, `archive_rejects_symlink_entries`, `archive_rejects_hardlink_entries`, `archive_rejects_too_many_entries`, `archive_rejects_extracted_size_over_limit`, itp.
  - 16 testĂłw procedury instalacji, walidacji i rollbacku: `updater_keeps_old_installation_until_new_one_is_ready`, `updater_rolls_back_on_install_failure`, `updater_rolls_back_when_final_rename_fails`, `updater_cleanup_removes_failed_staging`, `updater_rollback_preserves_original_file_contents`, `updater_rejects_artifact_replaced_after_verification`, `updater_rolls_back_when_windows_replace_fails`, `updater_rolls_back_when_linux_rename_fails`, `updater_rejects_incomplete_staging_layout`, itp.

---

### `UPDATE-TRUST-001` â€” Pomijanie weryfikacji manifestu w Ĺ›cieĹĽce GUI oraz podatnoĹ›Ä‡ na odwoĹ‚ane klucze i brak spĂłjnoĹ›ci metadanych aktualizacji
- **ID:** `UPDATE-TRUST-001`
- **TytuĹ‚:** ĹšcieĹĽka aktualizacji GUI omijajÄ…ca bezpiecznÄ… weryfikacjÄ™ manifestu i brak wymuszenia powiÄ…zania metadanych wydania
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED / VERIFIED_SOFTWARE**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06 / 2026-11-07
- **Commity:**
  - `bc3e806`: `fix(updater): require verified manifest for GUI update flow (UPDATE-TRUST-001)`
  - `c95172e`: `test(updater): harden trust data rollback and health safety matrix`
- **ModuĹ‚ / obszar:** `src/cloud/updater.rs`, `src/gui/app_async.rs`, `src/gui/app_services.rs`, `tests/security_guard.rs`
- **Lokalizacja:** [`src/cloud/updater.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs), [`src/gui/app_services.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_services.rs), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs)
- **DowĂłd z kodu przed naprawÄ…:**
  - `app_services.rs` wywoĹ‚ywaĹ‚o `trigger_update_install`, ktĂłre dobieraĹ‚o zasĂłb przez `select_asset_for_platform(&release.assets)` oparte wyĹ‚Ä…cznie na rozszerzeniu pliku (`.zip` / `.tar.gz`) z pominiÄ™ciem `select_manifest_asset`.
  - Weryfikacja sygnatury Ed25519 nie blokowaĹ‚a wydania przed przejĹ›ciem do GUI i pobrania w razie matematycznie poprawnego podpisu kluczem odwoĹ‚anym (przypadek wydania v1.2.0 i `splogbook-release-2026-01`).
- **WdroĹĽone rozwiÄ…zanie:**
  - Wprowadzono typy `VerifiedReleaseAsset` oraz `VerifiedReleaseManifest`. Obiekt `LatestRelease` moĹĽe powstaÄ‡ wyĹ‚Ä…cznie ze zweryfikowanym assetem (`pub verified_asset: VerifiedReleaseAsset`).
  - UsuniÄ™to niebezpiecznÄ… Ĺ›cieĹĽkÄ™ doboru po rozszerzeniu w GUI. Metoda `trigger_update_install` przekazuje `release.verified_asset` do `updater::install_update(&verified_asset)`.
  - Wymuszono peĹ‚ne powiÄ…zanie: platforma, architektura, typ paczki, nazwa pliku, rozmiar bajtowy i SHA-256 przed przejĹ›ciem do ekstrakcji i stagingu.
  - Manifesty podpisane kluczem odwoĹ‚anym (`REVOKED_KEY_IDS`) sÄ… bezwzglÄ™dnie odrzucane (fail-closed).
- **WdroĹĽone testy regresyjne:**
  - `gui_update_requires_verified_manifest_guard` w `tests/security_guard.rs`
  - `revoked_release_key_remains_rejected_guard` w `tests/security_guard.rs`
  - Testy jednostkowe `VerifiedReleaseManifest::verify` i `select_manifest_asset`.

---

### `UPDATE-DATA-001` â€” Ryzyko nadpisania danych uĹĽytkownika, konfiguracji i baz SQLite w katalogu portable oraz brak planu selektywnej podmiany
- **ID:** `UPDATE-DATA-001`
- **TytuĹ‚:** Nadpisywanie baz danych, konfiguracji i brak selektywnego planu aktualizacji w trybie portable
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED / VERIFIED_SOFTWARE**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06 / 2026-11-07
- **Commity:**
  - `12c3b22`: `fix(updater): plan file updates with fail-closed data protection (UPDATE-DATA-001)`
  - `da34b4c`: `fix(updater): preserve portable data during Windows and Linux update and rollback (UPDATE-DATA-001)`
  - `c95172e`: `test(updater): harden trust data rollback and health safety matrix`
- **ModuĹ‚ / obszar:** `src/cloud/updater.rs`, `tests/security_guard.rs`
- **Lokalizacja:** [`src/cloud/updater.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/updater.rs), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs)
- **DowĂłd z kodu przed naprawÄ…:**
  - Skrypt PowerShell na Windows uĹĽywaĹ‚ `Copy-Item -Path "$staging\*" -Destination $activeDir -Recurse -Force` oraz analogicznego polecenia przy rollbacku `$rollback\*`. NadpisywaĹ‚o to bazy w katalogu `databases/` (`serviceLOG.db`, `callbook.db`), plik `station_config.json` oraz `default_log.db`.
  - Na Linuksie kopiowano caĹ‚Ä… zawartoĹ›Ä‡ stagingu bez rozrĂłĹĽnienia danych uĹĽytkownika od plikĂłw wykonywalnych programu.
- **WdroĹĽone rozwiÄ…zanie:**
  - WdroĹĽono reguĹ‚Ä™ `UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE` oraz klasyfikator `classify_update_path(&Path) -> UpdateFileClass`.
  - Wprowadzono silnik planowania `UpdatePlan::generate` i walidacji `plan.validate_safety("splogbook")`.
  - Pliki `databases/serviceLOG.db` i `databases/callbook.db` zaklasyfikowano jako `FirstInstallSeed` (zachowywane, jeĹ›li istniejÄ… w aktywnej instalacji).
  - Pliki `station_config.json`, `default_log.db`, logi QSO, ADIF zaklasyfikowano jako nienaruszalne dane i konfiguracjÄ™ uĹĽytkownika.
  - Wyeliminowano dzikie karty (`$staging\*` / `$rollback\*`) w skrypcie PowerShell. Wygenerowano precyzyjne tablice `$replaceFiles` i `$createFiles`.
  - WdroĹĽono bezpiecznÄ… procedurÄ™ podmiany i rollbacku w `install_via_tar_gz` oraz `apply_atomic_installation_with_options`.
- **WdroĹĽone testy regresyjne:**
  - `portable_database_files_are_protected_guard` w `tests/security_guard.rs`
  - `windows_updater_does_not_force_copy_entire_staging_guard` w `tests/security_guard.rs`
  - `linux_updater_uses_validated_update_plan_guard` w `tests/security_guard.rs`
  - `rollback_excludes_protected_user_data_guard` w `tests/security_guard.rs`
  - `updater_does_not_use_sudo_guard` w `tests/security_guard.rs`
  - 17 testĂłw jednostkowych klasyfikatora i planera w `src/cloud/updater.rs`.

---

### `UPDATE-WIN-SAFETY-GATE` â€” Bramka bezpieczeĹ„stwa aktualizatora dla platformy Windows
- **ID:** `UPDATE-WIN-SAFETY-GATE`
- **TytuĹ‚:** Weryfikacja bezpieczeĹ„stwa formatu ZIP, powiÄ…zania manifestu, izolacji stagingu, braku dzikich kart i ochrony baz danych na Windows
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **SATISFIED_BY_UPDATE_TRUST_AND_DATA_SAFETY_GATE / COMPLETED_SOFTWARE_VERIFIED**
- **Data weryfikacji i zamkniÄ™cia:** 2026-11-06 / 2026-11-07
- **Commity:** `bc3e806`, `12c3b22`, `da34b4c`, `c95172e`, `6fbb495`
- **Raport pokrycia:** `STAGE_M1_COVERAGE_AND_STATUS_REPORT.md` (36/36 kryteriĂłw WIN-M1 w statusie PASS, 0 fail, 0 unverified)
- **Podsumowanie ochrony:** Eliminacja traktowania ZIP jako EXE, blokada path traversal, manifest-first (`VerifiedReleaseAsset`), fail-closed dla odwoĹ‚anego klucza `splogbook-release-2026-01`, brak dzikich kart `$staging\*` i `$rollback\*` w PowerShell, ochrona `databases/serviceLOG.db`, `databases/callbook.db`, `station_config.json`, `default_log.db` i nieznanych plikĂłw, rollback wykluczajÄ…cy dane uĹĽytkownika, brak `runas`, brak wymogu uprawnieĹ„ administratora.

---

### `UPDATE-LINUX-SAFETY-GATE` â€” Bramka bezpieczeĹ„stwa aktualizatora dla platformy Linux
- **ID:** `UPDATE-LINUX-SAFETY-GATE`
- **TytuĹ‚:** Weryfikacja bezpieczeĹ„stwa formatu tar.gz, powiÄ…zania manifestu, spĹ‚aszczania katalogu, walidacji UpdatePlan i braku uprawnieĹ„ sudo/pkexec na Linux
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **SATISFIED_BY_UPDATE_TRUST_AND_DATA_SAFETY_GATE / COMPLETED_SOFTWARE_VERIFIED**
- **Data weryfikacji i zamkniÄ™cia:** 2026-11-06 / 2026-11-07
- **Commity:** `bc3e806`, `12c3b22`, `da34b4c`, `c95172e`, `6fbb495`
- **Raport pokrycia:** `STAGE_M1_COVERAGE_AND_STATUS_REPORT.md` (40/40 kryteriĂłw LINUX-M1 w statusie PASS, 0 fail, 0 unverified)
- **Podsumowanie ochrony:** Bezpieczna ekstrakcja `extract_tar_gz_safe`, blokada symlinkĂłw/hardlinkĂłw/urzÄ…dzeĹ„, spĹ‚aszczanie pojedynczego katalogu nadrzÄ™dnego (`flatten_single_root_directory`), generowanie i walidacja `UpdatePlan::generate`, ochrona danych uĹĽytkownika i konfiguracji, zachowanie uprawnieĹ„ wykonywalnych (`0o755`), zachowanie `RUNPATH` i bibliotek Hamlib, brak `sudo` i `pkexec`, rollback transakcyjny bez naruszania baz danych.

---

### `UPDATE-DESIGN-WIN` â€” Projekt instalatora aktualizacji Windows (M2)
- **ID:** `UPDATE-DESIGN-WIN`
- **TytuĹ‚:** Architektura instalatora portable Windows ZIP z utwardzonym helperem PowerShell i deterministycznym rollbackiem
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status:** **APPROVED / COMPLETED_DESIGN_VERIFIED (DESIGN) / COMPLETED_SOFTWARE_VERIFIED (M3-B WINDOWS HELPERS & M3-C PORTABLE ZIP)** (Dokument DESIGN_3_0_A_WINDOWS_UPDATE.md v2.1.0; decyzje OWNER-M2-01..03 formalnie zatwierdzone w STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE; decyzje OWNER-M2-08, 09: OPEN / NON-BLOCKING — TEMPORARY CONSERVATIVE RETENTION POLICY; pomocnik natywny splogbook-updater.exe oraz utwardzony fallback PowerShell wdrożone w M3-B; pełna integracja produkcyjna Windows Portable ZIP wdrożona w STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE: zweryfikowany manifest Ed25519, bezpieczna inspekcja ZIP, UpdatePlan, walidacja PE AMD64, konsumpcja wyników po restarcie, ochrona baz SQLite i konfiguracji użytkownika, 2556 passed testów, 91 security guards, 28 Windows helper tests)
- **Data weryfikacji:** 2026-11-07

---

### `UPDATE-DESIGN-LINUX` â€” Projekt instalatora aktualizacji Linux (M2)
- **ID:** `UPDATE-DESIGN-LINUX`
- **TytuĹ‚:** Architektura instalatora portable Linux tar.gz z atomowym rename, ochronÄ… RUNPATH i brakiem eskalacji uprawnieĹ„
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status:** **APPROVED / COMPLETED_DESIGN_VERIFIED** (Dokument DESIGN_3_0_B_LINUX_UPDATE.md v2.1.0; decyzje OWNER-M2-04..07 formalnie zatwierdzone w STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE)
- **Data weryfikacji:** 2026-11-07

---

### `UPDATE-DESIGN-PLATFORM` â€” Projekt wspĂłlnego interfejsu instalatorĂłw (M2)
- **ID:** `UPDATE-DESIGN-PLATFORM`
- **TytuĹ‚:** Architektura maszyny stanĂłw, transakcji UpdateAttemptId, klasyfikacji plikĂłw i wspĂłlnego interfejsu PlatformInstallerAdapter
- **Kategoria:** `REL`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status:** **APPROVED / COMPLETED_DESIGN_VERIFIED (DESIGN) / COMPLETED_SOFTWARE_VERIFIED (M3-A PLATFORM CORE)** (Dokument DESIGN_3_0_C_PLATFORM_INSTALLERS.md v2.1.0; 9 decyzji blokujących M3 formalnie zatwierdzonych w STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE; rdzeń platformowy M3-A wdrożony i zweryfikowany w STAGE-M3-A-PLATFORM-CORE-IMPLEMENTATION-GATE)
- **Data weryfikacji:** 2026-11-07

---

### `SEC-002` â€” Wyciek sekretĂłw przez `#[derive(Debug)]` na `AppConfig` oraz hasĹ‚a w parametrach query URL
- **ID:** `SEC-002`
- **TytuĹ‚:** Brak redakcji sekretĂłw w `Debug for AppConfig` i wyciek haseĹ‚ w bĹ‚Ä™dach `reqwest::Error`
- **Kategoria:** `SEC`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status po Etapie 1C:** **FIXED** (Commit `6c4a30f`)
  - UsuniÄ™to automatyczne `#[derive(Debug)]` ze struktur konfiguracyjnych zawierajÄ…cych sekrety (`AppConfig`, `UploadCredentials`, `SyncConfig`).
  - WdroĹĽono bezpiecznÄ… rÄ™cznÄ… implementacjÄ™ `std::fmt::Debug` maskujÄ…cÄ… wszystkie 11 pĂłl sekretnych staĹ‚ym literaĹ‚em `"<redacted>"`.
  - Wprowadzono w `src/core/http.rs` procedury `sanitize_url`, `redacted_url_for_diagnostics` oraz `sanitize_network_error`.
  - WdroĹĽono redakcjÄ™ wraĹĽliwych parametrĂłw URL i oczyszczanie bĹ‚Ä™dĂłw sieciowych we wszystkich klientach chmurowych (`lotw.rs`, `eqsl.rs`, `hamqth.rs`, `qrz.rs`, `cloudlog.rs`, `hrdlog.rs`, `clublog.rs`).
  - Dodano testy regresyjne: `app_config_debug_redacts_all_secrets`, `online_services_debug_redacts_all_secrets`, `runtime_services_debug_redacts_all_secrets`, `cloud_errors_never_expose_credentials`, `secret_query_parameters_are_redacted`, `configuration_debug_never_leaks_secrets`.
- **ModuĹ‚ / obszar:** `src/core/station.rs`, `src/cloud/lotw.rs`, `src/cloud/eqsl.rs`, `src/cloud/hamqth.rs`, `src/cloud/qrz.rs`
- **Lokalizacja:** [`src/core/station.rs:276, 387-435`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/station.rs#L276-L435), [`src/cloud/lotw.rs:174`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/lotw.rs#L174), [`src/cloud/eqsl.rs:72`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/eqsl.rs#L72), [`src/cloud/hamqth.rs:138`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/hamqth.rs#L138), [`src/cloud/qrz.rs:48`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/qrz.rs#L48)
- **DowĂłd z kodu:**
  - `OnlineServicesConfig` (`station.rs:74`), `RuntimeServicesConfig` (`station.rs:212`) i `AppConfig` (`station.rs:276`) majÄ… automatyczne `#[derive(Debug)]`, ktĂłre wypisuje jawnie m.in. `lotw_password`, `eqsl_password`, `qrz_api_key`, `clublog_password`, `rest_api_token`, `lan_sync_secret`.
  - W `lotw.rs:174`, `eqsl.rs:72`, `hamqth.rs:138`, `qrz.rs:48` hasĹ‚a sÄ… doklejane do URL (`&password=...`). Gdy `reqwest` zwrĂłci bĹ‚Ä…d poĹ‚Ä…czenia/DNS/TLS, `e.to_string()` zawiera peĹ‚ny URL wraz z hasĹ‚em i trafia do `online_sync_logs` oraz `SPLogbook.log`.
- **Opis problemu:** HasĹ‚a mogÄ… przeniknÄ…Ä‡ do logĂłw diagnostycznych (`SPLogbook.log`) lub zrzutĂłw ekranu podczas awarii sieci.
- **Scenariusz wystÄ…pienia:** Brak internetu podczas synchronizacji z LoTW/eQSL/QRZ lub wywoĹ‚anie `format!("{config:?}")`.
- **WpĹ‚yw:** Ujawnienie poĹ›wiadczeĹ„ LoTW/eQSL/QRZ/ClubLog w plikach logĂłw.
- **RozwiÄ…zanie minimalne:** RÄ™czna implementacja `fmt::Debug` dla `OnlineServicesConfig`, `RuntimeServicesConfig` i `AppConfig` maskujÄ…ca 11 pĂłl sekretnych jako `"[REDACTED]"`, oraz uĹĽycie `e.without_url()` na bĹ‚Ä™dach `reqwest::Error` przed konwersjÄ… na komunikat.
- **RozwiÄ…zanie docelowe:** Typ opakowujÄ…cy `SecretString` z zamaskowanym `Debug`/`Display` i `ZeroizeOnDrop`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `app_config_debug_output_never_contains_secret_values`.
- **Kryterium akceptacji:** `format!("{cfg:?}")` nie zawiera ĹĽadnego z 11 sekretĂłw, a bĹ‚Ä™dy HTTP sÄ… pozbawione query stringu (`without_url()`).

---

### `SEC-003` â€” Brak timeoutu na `host_handshake` w `SyncHost` pozwala zablokowaÄ‡ wszystkie 5 slotĂłw poĹ‚Ä…czeĹ„ przychodzÄ…cych
- **ID:** `SEC-003`
- **TytuĹ‚:** MoĹĽliwoĹ›Ä‡ wyczerpania semafora `MAX_HANDSHAKES = 5` przez niekompletne poĹ‚Ä…czenia TCP (Slowloris DoS)
- **Kategoria:** `SEC`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status po Etapie 1C:** **FIXED** (Commit `4f3f3d5`)
  - Zdefiniowano `pub const HOST_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);` w `src/sync/host.rs` i wyeksportowano w `src/sync/mod.rs`.
  - Opakowano wywoĹ‚anie `host_handshake` w pÄ™tli nasĹ‚uchu `run_listener_loop` za pomocÄ… `tokio::time::timeout`.
  - Po upĹ‚yniÄ™ciu timeoutu emitowane jest zdarzenie `SyncEvent::Error(SyncError::Timeout)`, a zadanie asynchroniczne koĹ„czy bieg, gwarantujÄ…c zwolnienie slotu semafora `handshake_sem` (RAII permit drop) oraz natychmiastowe zamkniÄ™cie gniazda TCP.
  - Dodano testy regresyjne: `sync_host_handshake_timeout_releases_slot`, `sync_partial_handshake_times_out`, `sync_successful_handshake_is_not_rejected_by_timeout`, `security_guard_bind_policy_and_handshake_timeout`.
- **ModuĹ‚ / obszar:** `src/sync/host.rs`
- **Lokalizacja:** [`src/sync/host.rs:78-115`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/host.rs#L78-L115)
- **DowĂłd z kodu:** W `SyncHost::bind_with_store` (`src/sync/host.rs:88-100`) po pobraniu permitu z `handshake_sem` (`MAX_HANDSHAKES = 5`) wywoĹ‚ywane jest `host_handshake(stream, ...).await` **bez `tokio::time::timeout`**.
- **Opis problemu:** JeĹ›li 5 gniazd TCP poĹ‚Ä…czy siÄ™ z portem Hosta i nie wyĹ›le ramki `ClientHello`, wszystkie 5 permitĂłw semafora pozostanie zajÄ™tych w nieskoĹ„czonoĹ›Ä‡, blokujÄ…c moĹĽliwoĹ›Ä‡ doĹ‚Ä…czenia jakiegokolwiek uprawnionego operatora.
- **Scenariusz wystÄ…pienia:** 5 wiszÄ…cych poĹ‚Ä…czeĹ„ TCP (np. skaner portĂłw w sieci LAN lub zerwane przed handshake'iem sesje Wi-Fi).
- **WpĹ‚yw:** CaĹ‚kowita blokada nowych poĹ‚Ä…czeĹ„ do `SyncHost` do czasu restartu usĹ‚ugi.
- **RozwiÄ…zanie minimalne:** OpakowaÄ‡ `host_handshake(...)` w `tokio::time::timeout(Duration::from_secs(10), host_handshake(...)).await`.
- **RozwiÄ…zanie docelowe:** Timeout 10 s na caĹ‚y handshake + `TCP_KEEPALIVE` na gnieĹşdzie.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `sync_host_drops_stalled_handshake_and_releases_semaphore_permit`.
- **Kryterium akceptacji:** 5 milczÄ…cych poĹ‚Ä…czeĹ„ TCP zostaje rozĹ‚Ä…czonych po upĹ‚ywie timeoutu, zwalniajÄ…c permity `handshake_sem`.

---

### `SEC-004` â€” DomyĹ›lne bindowanie `SyncHost` na `0.0.0.0` oraz brak ostrzeĹĽenia o pracy bez tokenu w REST API
- **ID:** `SEC-004`
- **TytuĹ‚:** Nadmierna ekspozycja gniazda LAN Sync (`0.0.0.0`) i cicha praca REST API z pustym tokenem
- **Kategoria:** `SEC`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status po Etapie 1C:** **FIXED** (Commit `205a39f`)
  - Wprowadzono typ `ServerBindError` oraz procedurÄ™ `validate_bind_policy(ip: &IpAddr, api_key: &str) -> Result<(), ServerBindError>` w `src/api/server.rs`, uniemoĹĽliwiajÄ…cÄ… uruchomienie serwera na interfejsie publicznym (`!ip.is_loopback()`) bez skonfigurowanego klucza API.
  - W `start_api_server_with_state` podĹ‚Ä…czono weryfikacjÄ™ polityki wiÄ…zania oraz skonfigurowano `app.into_make_service_with_connect_info::<SocketAddr>()`.
  - W `auth_middleware` rozszerzono ekstrakcjÄ™ tokena o `Authorization: Bearer <token>` oraz `x-api-key`, z weryfikacjÄ… w staĹ‚ym czasie (`constant_time_eq`).
  - Gdy klucz API jest pusty, operacje mutujÄ…ce (`POST`, `PUT`, `PATCH`, `DELETE`) sÄ… rygorystycznie chronione: odrzucajÄ… (403 Forbidden) ĹĽÄ…dania od klientĂłw zdalnych (`!ip.is_loopback()`), z obcym nagĹ‚Ăłwkiem `Origin` lub z `Sec-Fetch-Site: cross-site`, dopuszczajÄ…c mutacje wyĹ‚Ä…cznie dla zaufanych lokalnych procesĂłw loopback pod politykÄ… local-only.
  - Zachowano kontrakt dla publicznych endpointĂłw odczytu (`/api/v1/status`, `/api/v1/endpoints`).
  - Dodano testy regresyjne: `rest_mutations_require_token_when_configured`, `rest_mutations_reject_remote_clients_when_token_is_empty`, `rest_mutations_allow_loopback_client_under_local_only_policy`, `rest_import_rejects_untrusted_browser_origin_without_token`, `rest_read_only_endpoint_preserves_existing_contract`, `rest_never_logs_or_returns_api_token`, `rest_server_refuses_public_bind_without_authentication`, `security_guard_bind_policy_and_handshake_timeout`.
- **ModuĹ‚ / obszar:** `src/sync/service.rs`, `src/api/server.rs`
- **Lokalizacja:** [`src/sync/service.rs:144`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/service.rs#L144), [`src/api/server.rs:347-370`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/api/server.rs#L347-L370)
- **DowĂłd z kodu:**
  - `SyncService::start` (`src/sync/service.rs:144`) binduje `SyncHost` na sztywno do `0.0.0.0:{port}` bez moĹĽliwoĹ›ci wyboru interfejsu w `SyncRole::Host`.
  - W `api/server.rs:351` gdy `state.api_token.is_none()`, middleware `auth_middleware` przepuszcza wszystkie ĹĽÄ…dania mutujÄ…ce (`DELETE /api/v1/qsos/{id}`, `POST /api/v1/adif/import`) z dowolnego procesu lokalnego lub strony www (brak Ĺ›cisĹ‚ej blokady CORS/Origin dla ĹĽÄ…daĹ„ prostych `POST text/plain` w `import_adif_handler`).
- **Opis problemu:** Brak moĹĽliwoĹ›ci zawÄ™ĹĽenia interfejsu nasĹ‚uchu Hosta LAN Sync oraz podatnoĹ›Ä‡ `POST /api/v1/adif/import` (`text/plain`) na ĹĽÄ…dania cross-origin z przeglÄ…darki, gdy uĹĽytkownik wĹ‚Ä…czy REST API bez ustawienia tokenu.
- **Scenariusz wystÄ…pienia:** UĹĽytkownik wĹ‚Ä…cza REST API bez tokenu i otwiera w przeglÄ…darce stronÄ™ wysyĹ‚ajÄ…cÄ… `fetch("http://127.0.0.1:8420/api/v1/adif/import", { method: "POST", body: "..." })`.
- **WpĹ‚yw:** Nieautoryzowany import ADIF z poziomu przeglÄ…darki przy pustym tokenie REST API.
- **RozwiÄ…zanie minimalne:** W `auth_middleware` odrzucaÄ‡ ĹĽÄ…dania posiadajÄ…ce nagĹ‚Ăłwek `Origin` lub `Sec-Fetch-Site: cross-site`, jeĹ›li `api_token` jest pusty (lub generowaÄ‡ losowy token przy pierwszym wĹ‚Ä…czeniu REST API).
- **RozwiÄ…zanie docelowe:** Automatyczne generowanie tokenu Bearer przy aktywacji REST API + odrzucanie ĹĽÄ…daĹ„ z nagĹ‚Ăłwkiem `Origin`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `rest_api_rejects_browser_origin_requests_when_token_unset`.
- **Kryterium akceptacji:** Ĺ»Ä…danie `POST /api/v1/adif/import` z nagĹ‚Ăłwkiem `Origin: https://evil.example` bez tokenu Bearer zwraca `403 Forbidden`.

---

## 2. Dane, Baza SQLite, ADIF i IntegralnoĹ›Ä‡ DziennikĂłw (`DATA-*`)

### Ujednolicone mapowanie znalezisk i commitĂłw Etapu 5 (IntegralnoĹ›Ä‡ Danych: ADIF, Ingest i Indeksy SQLite)

Status Etapu 5: **SOFTWARE VERIFIED**

| Identyfikator | TytuĹ‚ znaleziska | WdroĹĽone rozwiÄ…zanie | Commit | ModuĹ‚y / Pliki | Testy weryfikujÄ…ce | Status po Etapie 5 | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `DATA-001` | Parser ADIF fabrykuje bieĹĽÄ…cÄ… datÄ™/czas `Utc::now()` i uszkadza wielobajtowy UTF-8 | Eliminacja fabrykacji daty/czasu, tryby `AdifParseMode::Strict` i `Compatibility`, ostrzeĹĽenia diagnostyczne, kanoniczny eksport UTF-8 | `56b44a4`, `2fe66b6` | `src/core/adif.rs`, `src/cloud/lotw.rs` | 36 testĂłw jednostkowych w `core::adif`, w tym 7 testĂłw jednoznacznoĹ›ci i ostrzeĹĽeĹ„ | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `DATA-002` | `QsoIngestor::import_batch` pomija normalizacjÄ™ i atomowoĹ›Ä‡ wsadowÄ… | Normalizacja pĂłl w batchu, dowodowa atomowoĹ›Ä‡ pojedynczego zapisu i batcha, peĹ‚ny rollback w SQLite i stanach pochodnych | `9b7f0fc`, `b6bedf6` | `src/core/ingest/{import.rs,mod.rs}` | 14 testĂłw wsadowych i awaryjnych, weryfikacja failpointĂłw, rollbacku i ponowienia | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `DATA-003` | Precyzyjna toĹĽsamoĹ›Ä‡ QSO v2, unikalnoĹ›Ä‡ w SQLite i eliminacja false-positive deduplikacji | Precyzyjny klucz v2 QsoIdentity (S/M czas, freq w Hz, normalizacja SUBMODE), MODEL B dla rekordĂłw niepeĹ‚nych (NULL), migracja schematu 7 z atomowym wycofaniem przy konfliktach, bazodanowy UNIQUE index (journal_id, identity_key) | `1393079`, `1e448dd`, `268e2d2` | `src/core/qso.rs`, `src/core/database/{qso_store.rs,migrations.rs,tests.rs}`, `src/core/ingest/{import.rs,mod.rs}` | 104 testy w `core::database`, w tym testy precyzji sekund, czÄ™stotliwoĹ›ci, subemisji, izolacji dziennikĂłw i wyĹ›cigĂłw wielopoĹ‚Ä…czeniowych | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `ARCH-002` | Edycja QSO omija Ingestor oraz brak odbudowywalnoĹ›ci stanĂłw pochodnych | Unifikacja Ĺ›cieĹĽki edycji w `update_existing`, metoda `rebuild_derived_state()`, emisja zdarzeĹ„ wyĹ‚Ä…cznie po commit SQLite | `50321f1`, `3d8463b` | `src/core/ingest/mod.rs`, `src/core/scp.rs` | 54 testy w `core::ingest`, 6 testĂłw w `core::scp`, testy rekonstrukcji po awarii | `FIXED_SOFTWARE_VERIFIED` | NISKIE |

### `DATA-001` â€” Parser ADIF fabrykuje bieĹĽÄ…cÄ… datÄ™/czas `Utc::now()` przy brakujÄ…cych polach `<QSO_DATE>` / `<TIME_ON>` i uszkadza wielobajtowy UTF-8
- **ID:** `DATA-001`
- **TytuĹ‚:** Fabrykowanie `Utc::now()` w `adif::fields_to_qso` oraz bĹ‚Ä™dne indeksowanie bajtowe wielobajtowych znakĂłw UTF-8 w parserze ADIF
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/core/adif.rs`
- **Lokalizacja:** [`src/core/adif.rs:155-167`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L155-L167), [`src/core/adif.rs:425-445`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/adif.rs#L425-L445)
- **DowĂłd z kodu:**
  1. W `fields_to_qso` (`src/core/adif.rs:428`) wywoĹ‚ywane jest `let mut qso = QsoRecord::new(callsign, band, mode);`, ktĂłre w [`src/core/qso.rs:133-134`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/qso.rs#L133-L134) inicjalizuje `qso_date` i `time_on` wartoĹ›ciÄ… `Utc::now()`. JeĹ›li rekord ADIF nie zawiera tagu `<QSO_DATE>` lub `<TIME_ON>` (np. czÄ™Ĺ›ciowa odpowiedĹş QSL lub uszkodzony rekord), parser **nie odrzuca rekordu**, lecz po cichu przypisuje mu dzisiejszÄ… datÄ™ i godzinÄ™ importu!
  2. W `parse_single_record_tags` (`src/core/adif.rs:159-162`) dĹ‚ugoĹ›Ä‡ pola `<TAG:L>` jest traktowana bezwarunkowo jako liczba bajtĂłw (`&bytes[idx..end]`, `String::from_utf8_lossy`). Gdy plik ADIF z innego programu (np. Logger32, UR5EQF, CQRLog) podaje liczbÄ™ znakĂłw Unicode dla polskich znakĂłw diakrytycznych (np. `<QTH:6>KrakĂłw` â€” 6 znakĂłw, 7 bajtĂłw UTF-8), wycinek `&bytes[idx..idx+6]` przecina 2-bajtowy znak `Ăł` (`0xC3 0xB3`) w poĹ‚owie, zamieniajÄ…c go na `Krak`!
- **Opis problemu:** ZĹ‚amanie kontraktu `PIPE-4B.1` (zakaz fabrykowania brakujÄ…cych pĂłl podczas importu ADIF) oraz psucie polskich znakĂłw w polach `NAME`/`QTH`/`COMMENT`.
- **Scenariusz wystÄ…pienia:** Import pliku ADIF bez `<QSO_DATE>`/`<TIME_ON>` lub z polskimi znakami diakrytycznymi w `<NAME>`/`<QTH>`.
- **WpĹ‚yw:** Ciche zanieczyszczenie dziennika Ĺ‚Ä…cznoĹ›ciami z faĹ‚szywÄ… datÄ… dzisiejszÄ… oraz uszkodzenie polskich znakĂłw w imionach i QTH.
- **RozwiÄ…zanie minimalne:**
  1. W `fields_to_qso` inicjalizowaÄ‡ `qso.qso_date = String::new()` i `qso.time_on = String::new()` przed pÄ™tlÄ… tagĂłw, aby `qso.validate()` odrzuciĹ‚o rekordy bez daty/czasu.
  2. W `parse_single_record_tags` sprawdziÄ‡ `if !content.is_char_boundary(end)`, a w przypadku przeciÄ™cia wielobajtowego znaku UTF-8 odczytaÄ‡ `length` znakĂłw (`content[idx..].chars().take(length)`).
- **RozwiÄ…zanie docelowe:** ObsĹ‚uga obu wariantĂłw kodowania dĹ‚ugoĹ›ci ADIF (byte-count z fallbackiem char-count na granicy UTF-8) + rygorystyczna inicjalizacja pustych pĂłl `qso_date`/`time_on`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `adif_import_rejects_record_missing_qso_date_or_time_on` oraz `adif_import_preserves_multibyte_utf8_when_length_is_char_count`.
- **Kryterium akceptacji:** Rekord ADIF bez `<QSO_DATE>` lub `<TIME_ON>` jest odrzucany w `AdifParseReport.rejected`, a `<QTH:6>KrakĂłw<EOR>` parsuje siÄ™ do `"KrakĂłw"`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-03
- **Commit:**
  - `56b44a4`: `fix(adif): enforce bounded and deterministic record parsing (DATA-001)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. Zdefiniowano twarde limity zasobĂłw ADIF (`MAX_ADIF_FILE_SIZE = 64 MiB`, `MAX_ADIF_HEADER_SIZE = 1 MiB`, `MAX_ADIF_FIELD_NAME_LEN = 64`, `MAX_ADIF_FIELD_VALUE_LEN = 1 MiB`, `MAX_ADIF_FIELDS_PER_RECORD = 256`).
  2. Wprowadzono warianty bĹ‚Ä™dĂłw `AdifRecordError::MissingDate` oraz `AdifRecordError::MissingTime`. W `fields_to_qso` pola daty i czasu sÄ… inicjalizowane jako puste â€“ brak tagu `<QSO_DATE>` lub `<TIME_ON>` powoduje jawne odrzucenie rekordu bez fabrykowania `Utc::now()`.
  3. WdroĹĽono dualne wyznaczanie granic pĂłl (dual-mode length extraction): weryfikacja byte-count z fallbackiem na char-count, zapobiegajÄ…c ucinaniu wielobajtowych znakĂłw UTF-8 (np. polskich znakĂłw w nazwach QTH/Name).
- **WdroĹĽone testy regresyjne:**
  - 11 nowych deterministycznych testĂłw regresyjnych w `src/core/adif.rs` weryfikujÄ…cych zachowanie znakĂłw diakrytycznych (`KrakĂłw`, `ĹĂłdĹş`), odrzucanie brakujÄ…cych dat/czasĂłw, odpornoĹ›Ä‡ na nieznane pola ADIF, oraz brak terminacji na `<EOR>` wewnÄ…trz wartoĹ›ci pola.

---

### `DATA-002` â€” `QsoIngestor::import_batch` pomija wywoĹ‚anie `normalize_qso_fields(&mut qso)`
- **ID:** `DATA-002`
- **TytuĹ‚:** Brak normalizacji pĂłl QSO (pasmo z czÄ™stotliwoĹ›ci, format daty/czasu, wielkoĹ›Ä‡ liter) w `QsoIngestor::import_batch`
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/core/ingest/import.rs`, `src/sync/store.rs`
- **Lokalizacja:** [`src/core/ingest/import.rs:63-88`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/ingest/import.rs#L63-L88), [`src/sync/store.rs:58-65`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/store.rs#L58-L65)
- **DowĂłd z kodu:** Podczas gdy `QsoIngestor::ingest` ([`src/core/ingest/mod.rs:127`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/ingest/mod.rs#L127)) wywoĹ‚uje `normalize_qso_fields(&mut qso)` przed `qso.validate()`, metoda `QsoIngestor::import_batch` ([`src/core/ingest/import.rs:63-88`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/ingest/import.rs#L63-L88)) od razu wywoĹ‚uje `if let Err(issue) = qso.validate()`, nigdy nie wywoĹ‚ujÄ…c `normalize_qso_fields(&mut qso)`!
- **Opis problemu:**
  1. Rekord ADIF zawierajÄ…cy `<FREQ:6>14.074` bez tagu `<BAND>` (w peĹ‚ni legalny w ADIF) zostaje odrzucony przez `import_batch` jako `MissingBand`, poniewaĹĽ wyznaczenie pasma z czÄ™stotliwoĹ›ci znajduje siÄ™ w `normalize_qso_fields` (`normalize.rs:28-33`).
  2. W `SyncStore::import` (`src/sync/store.rs:60-63`) uĹĽywane jest `ImportOptions { enrich: false, .. }`. Bez `normalize_qso_fields` rekordy z LAN Sync mogÄ… trafiÄ‡ do SQLite z datÄ… `2026-10-18` (z myĹ›lnikami) lub maĹ‚Ä… literÄ… w znaku/pasmie.
- **Scenariusz wystÄ…pienia:** Import pliku ADIF z programu eksportujÄ…cego tylko `<FREQ>` bez `<BAND>` lub synchronizacja Catch-Up w sieci LAN.
- **WpĹ‚yw:** Odrzucanie poprawnych Ĺ‚Ä…cznoĹ›ci podczas importu ADIF oraz niespĂłjnoĹ›Ä‡ formatu dat/znakĂłw w bazie SQLite.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ wywoĹ‚anie `normalize_qso_fields(&mut qso);` na poczÄ…tku pÄ™tli w `QsoIngestor::import_batch` (`src/core/ingest/import.rs:64`) przed `qso.validate()`.
- **RozwiÄ…zanie docelowe:** WspĂłlna funkcja pomocnicza `prepare_qso_for_storage(&mut qso, enrich, &prefix_matcher)` uĹĽywana zarĂłwno przez `ingest()`, jak i `import_batch()`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `DATA-003`.
- **Wymagany test regresyjny:** `import_batch_infers_band_from_freq_and_normalizes_date_time`.
- **Kryterium akceptacji:** `import_batch` akceptuje rekord z `band: ""` i `freq: Some(14.074)`, zapisujÄ…c `band == "20m"`, `qso_date == "YYYYMMDD"`, `time_on == "HHMMSS"`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-03
- **Commit:**
  - `9b7f0fc`: `fix(data): make QSO ingestion atomic and idempotent (DATA-002)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. Dodano wywoĹ‚anie `normalize_qso_fields(&mut qso)` na poczÄ…tku pÄ™tli przetwarzania rekordĂłw w `QsoIngestor::import_batch` przed wywoĹ‚aniem `qso.validate()`.
  2. Zapewniono automatycznÄ… dedukcjÄ™ pasma na podstawie czÄ™stotliwoĹ›ci (`freq`), normalizacjÄ™ wielkoĹ›ci liter oraz czyszczenie formatĂłw dat/czasu w importach wsadowych i synchronizacji LAN Sync.
- **WdroĹĽone testy regresyjne:**
  - `import_batch_infers_band_from_freq_and_normalizes_date_time` w `src/core/ingest/import.rs`.

---

### `DATA-003` â€” Nieznormalizowane porĂłwnania SQL dla dat/pasm/emisji oraz wyĹ‚Ä…czenie indeksĂłw B-drzewa przez `UPPER(...)` i `COLLATE NOCASE`
- **ID:** `DATA-003`
- **TytuĹ‚:** NiespĂłjne zapytania SQL w `search.rs`, `qso_store.rs`, `qsl.rs` i `callbook.rs` pomijajÄ…ce normalizacjÄ™ i indeksy SQLite
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/core/database/search.rs`, `src/core/database/qso_store.rs`, `src/core/database/qsl.rs`, `src/core/callbook.rs`
- **Lokalizacja:** [`src/core/database/search.rs:99-110`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/search.rs#L99-L110), [`src/core/database/qso_store.rs:308-329`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qso_store.rs#L308-L329), [`src/core/database/qsl.rs:20-75`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qsl.rs#L20-L75), [`src/core/callbook.rs:333`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/callbook.rs#L333)
- **DowĂłd z kodu:**
  1. W `search_qsos_advanced` (`search.rs:99-110`) filtr daty uĹĽywa surowego `qso_date >= ?` bez usuniÄ™cia myĹ›lnikĂłw z parametru wejĹ›ciowego `date_from` (np. wpisanie `2026-01-01` w GUI porĂłwnuje tekstowo `'20261018' >= '2026-01-01'`, gdzie `'1' > '-'` zakĹ‚amuje zakresy dla lat i miesiÄ™cy!).
  2. W `has_identity` (`qso_store.rs:308-329`) zapytanie wykonuje `WHERE journal_id = ?1 AND UPPER(callsign) = ?2`, co uniemoĹĽliwia SQLite uĹĽycie indeksu `idx_qso_callsign` (peĹ‚ny skan dziennika przy kaĹĽdym zapisie QSO!).
  3. W `callbook.rs:333` zapytanie `SELECT ... FROM callbook WHERE Call = ?1 COLLATE NOCASE` nakĹ‚ada `COLLATE NOCASE` na kolumnÄ™ z domyĹ›lnym indeksem `BINARY`, wyĹ‚Ä…czajÄ…c indeks B-tree na 4,4 MB bazie `callbook.db`.
- **Opis problemu:** BĹ‚Ä™dne wyniki filtrowania dat z myĹ›lnikami oraz degradacja zapytaĹ„ indeksowanych do peĹ‚nych skanĂłw tabeli (`SCAN TABLE`).
- **Scenariusz wystÄ…pienia:** Wyszukiwanie zaawansowane po dacie `YYYY-MM-DD`, masowy import ADIF (tysiÄ…ce wywoĹ‚aĹ„ `has_identity`) lub wpisywanie znaku w formularzu QSO (`callbook.lookup`).
- **WpĹ‚yw:** BĹ‚Ä™dne wyniki wyszukiwania oraz liniowy spadek wydajnoĹ›ci importu i autouzupeĹ‚niania.
- **RozwiÄ…zanie minimalne:**
  1. NormalizowaÄ‡ parametry `date_from`/`date_to` w `search_qsos_advanced` przez `.replace('-', "")`.
  2. W `callbook.rs:333` usunÄ…Ä‡ `COLLATE NOCASE` (znaki w `callbook.db` oraz parametr `clean` sÄ… juĹĽ wielkimi literami).
  3. DodaÄ‡ migracjÄ™ normalizujÄ…cÄ… istniejÄ…ce wiersze w `qso_records` (`callsign = UPPER(callsign)`, `qso_date = REPLACE(qso_date, '-', '')`) i uĹĽywaÄ‡ bezpoĹ›redniego `WHERE journal_id = ?1 AND callsign = ?2 AND qso_date = ?3`.
- **RozwiÄ…zanie docelowe:** Kanoniczna reprezentacja kolumn klucza toĹĽsamoĹ›ci w `qso_records` ze zĹ‚oĹĽonym indeksem `idx_qso_identity(journal_id, callsign, qso_date, band, mode)`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie (migracja w transakcji przy starcie bazy).
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** `DATA-002`.
- **Wymagany test regresyjny:** `search_qsos_advanced_normalizes_hyphenated_date_bounds` oraz test `EXPLAIN QUERY PLAN` dla `has_identity` i `callbook.lookup`.
- **Kryterium akceptacji:** `EXPLAIN QUERY PLAN` potwierdza `SEARCH ... USING INDEX` dla `has_identity` i `callbook.lookup`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-03 (Bramka Precyzji ToĹĽsamoĹ›ci QSO)
- **Commity:**
  - `1393079`: `fix(data): refine QSO identity precision and incomplete-record policy (DATA-003)`
  - `1e448dd`: `fix(data): database migration 7 for refined QSO identity (DATA-003)`
  - `268e2d2`: `test(data): SQLite concurrency and multi-connection identity tests (DATA-003)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. **Kanoniczny format toĹĽsamoĹ›ci v2:** `v2|{CALL}|{BAND}|{FREQ_HZ}|{MODE}|{SUBMODE}|{DATE}|{TIME_TAG}`.
  2. **Zachowanie sekund i precyzji czasu:** Wprowadzono `QsoTimePrecision::Minute` (`M{HHMM}`) oraz `QsoTimePrecision::Second` (`S{HHMMSS}`). Czas o precyzji minutowej nie dopisuje sztucznych sekund "00", eliminujÄ…c faĹ‚szywe kolizje z Ĺ‚Ä…cznoĹ›ciami z sekundy 00.
  3. **Stabilna reprezentacja czÄ™stotliwoĹ›ci:** CzÄ™stotliwoĹ›Ä‡ przechowywana w kluczu w hercach (`u64`), co rozrĂłĹĽnia Ĺ‚Ä…cznoĹ›ci z tej samej minuty na rĂłĹĽnych czÄ™stotliwoĹ›ciach pasma.
  4. **Normalizacja par MODE i SUBMODE:** `normalize_mode_and_submode` kanonicznie mapuje FT8/FT4/Q65/WSPR do emisji cyfrowych bez subemisji, USB/LSB do `SSB` z `SUBMODE`, a CW-R/CWR do `CW`.
  5. **Polityka rekordĂłw niepeĹ‚nych (MODEL B):** Rekordy z brakujÄ…cym znakiem, pasmem/czÄ™stotliwoĹ›ciÄ…, emisjÄ…, datÄ… lub czasem otrzymujÄ… `canonical_key() == None` i sÄ… zapisywane w SQLite z `identity_key = NULL`. Zgodnie ze standardem SQL i SQLite UNIQUE index zezwala na dowolnÄ… liczbÄ™ rekordĂłw z wartoĹ›ciÄ… NULL, co chroni niekompletne rekordy przed faĹ‚szywÄ… deduplikacjÄ… i umoĹĽliwia ich rÄ™cznÄ… weryfikacjÄ™.
  6. **Migracja schematu 7 (`SCHEMA_VERSION` 6 -> 7):** W peĹ‚ni transakcyjna migracja aktualizujÄ…ca klucze do formatu v2, wykonujÄ…ca audyt duplikatĂłw przed utworzeniem `CREATE UNIQUE INDEX idx_qso_identity_unique ON qso_records(journal_id, identity_key)`. W razie wykrycia konfliktu nastÄ™puje peĹ‚ny ROLLBACK â€” ĹĽadne dane nie sÄ… usuwane ani automatycznie scalane, a wersja schematu pozostaje nienaruszona.
- **WdroĹĽone testy regresyjne:**
  - 104 testy weryfikujÄ…ce toĹĽsamoĹ›Ä‡ i bazÄ™ danych w `src/core/database/tests.rs`:
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

### `DATA-004` â€” Operacje niszczÄ…ce (usuwanie duplikatĂłw i usuwanie zaznaczonych QSO) dziaĹ‚ajÄ… bez potwierdzenia, bez Undo i w poprzek wszystkich dziennikĂłw
- **ID:** `DATA-004`
- **TytuĹ‚:** Nieodwracalne usuwanie QSO bez okna potwierdzenia, bez zapisu na stosie `undo_stack` oraz usuwanie duplikatĂłw z obcych dziennikĂłw
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia:** 2026-11-03
- **Commity:** `eff6587` (`fix(data): enforce journal-scoped QSO access (DATA-004 DATA-005)`), `b0afb20` (`fix(data): prevent cross-journal QSO mutations (DATA-004 DATA-005 CONC-001)`), `9157e0e` (`fix(data): isolate async and derived state by journal (DATA-004 DATA-005)`), `f767e01` (`fix(data): add atomic journal-scoped undo history (DATA-004)`)
- **ModuĹ‚ / obszar:** `src/gui/find_duplicates.rs`, `src/core/database/search.rs`, `src/gui/app_qso.rs`, `src/gui/state/log_view.rs`, `src/gui/logbook_table.rs`
- **Lokalizacja:** [`src/gui/find_duplicates.rs:281-326`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/find_duplicates.rs#L281-L326), [`src/core/database/search.rs:27-81`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/search.rs#L27-L81), [`src/gui/app_qso.rs:564-601`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_qso.rs#L564-L601), [`src/gui/state/log_view.rs:20`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/state/log_view.rs#L20)
- **DowĂłd z kodu:**
  1. `Database::find_duplicate_qsos` (`src/core/database/search.rs:27-81`) wykonuje `SELECT ... FROM qso_records` **bez warunku `WHERE journal_id = ?`**. JeĹĽeli ten sam znak zostaĹ‚ zrobiony w dzienniku `DEFAULT` i w dzienniku `POTA`, okno â€žZnajdĹş duplikatyâ€ť uznaje je za duplikat i przycisk â€žUsuĹ„ zaznaczone duplikatyâ€ť (`find_duplicates.rs:297-318`) bez ĹĽadnego potwierdzenia usuwa rekord z bazy, nie dodaje go do `app.entry.undo_stack` i nie wywoĹ‚uje `app.rebuild_awards_full()`.
  2. `SpLogApp::delete_selected_qso` (`src/gui/app_qso.rs:564-601`) natychmiast usuwa z bazy wszystkie zaznaczone wiersze (`selected_qso_ids`) po naciĹ›niÄ™ciu klawisza `Delete`, **nie zapisujÄ…c usuniÄ™tych rekordĂłw na `self.entry.undo_stack`** i nigdy nie uĹĽywajÄ…c pola `self.log_view.confirm_bulk_delete` (`src/gui/state/log_view.rs:20` â€” martwe pole!).
- **Opis problemu:** Przypadkowe naciĹ›niÄ™cie `Delete` lub uruchomienie oczyszczania duplikatĂłw powoduje nieodwracalnÄ… utratÄ™ Ĺ‚Ä…cznoĹ›ci (w tym z innych profili dziennikĂłw) bez moĹĽliwoĹ›ci cofniÄ™cia (`Ctrl+Z`).
- **Scenariusz wystÄ…pienia:** Operator zaznacza kilka Ĺ‚Ä…cznoĹ›ci i naciska `Delete`, a nastÄ™pnie prĂłbuje uĹĽyÄ‡ `Ctrl+Z` (Undo) â€” usuniÄ™te Ĺ‚Ä…cznoĹ›ci nie wracajÄ…; albo uruchamia â€žZnajdĹş duplikatyâ€ť posiadajÄ…c kilka dziennikĂłw.
- **WpĹ‚yw:** TrwaĹ‚a utrata danych logbooka i desynchronizacja indeksĂłw dyplomowych `AwardsEngine`.
- **RozwiÄ…zanie minimalne:**
  1. DodaÄ‡ parametr `journal_id: &str` do `Database::find_duplicate_qsos`.
  2. W `delete_selected_qso` oraz `find_duplicates.rs` pobieraÄ‡ usuwane rekordy przed `delete_qsos_batch`, odkĹ‚adaÄ‡ je na `undo_stack` (rozszerzajÄ…c `UndoAction::DeleteBatch(Vec<QsoRecord>)`), wymagaÄ‡ potwierdzenia modalnego (`confirm_bulk_delete`) i wywoĹ‚ywaÄ‡ `rebuild_awards_full()`.
- **RozwiÄ…zanie docelowe:** Transakcyjne usuwanie z obowiÄ…zkowym wpisem w `undo_stack` i modalem potwierdzenia.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** `DATA-005`.
- **Wymagany test regresyjny:** `find_duplicates_is_isolated_to_active_journal` oraz `delete_selected_qso_pushes_records_to_undo_stack_and_restores_on_undo`.
- **Kryterium akceptacji:** UsuniÄ™cie pojedynczego lub wielu QSO pozwala na peĹ‚ne przywrĂłcenie przez `perform_undo()`, a `find_duplicate_qsos` nie Ĺ‚Ä…czy rekordĂłw z rĂłĹĽnych `journal_id`.

---

### `DATA-005` â€” Przeciekanie danych pomiÄ™dzy dziennikami (`multi-journal bleed`) w statystykach, synchronizacji chmurowej, QSL, przeĹ‚Ä…czaniu dziennika i kopii ADIF
- **ID:** `DATA-005`
- **TytuĹ‚:** Brak izolacji `active_journal.id` w statystykach, chmurze (LoTW/eQSL/QRZ/ClubLog/HRDLog), projektancie QSL oraz brak przebudowy `AwardsEngine` po zmianie dziennika
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia:** 2026-11-03
- **Commity:** `eff6587` (`fix(data): enforce journal-scoped QSO access (DATA-004 DATA-005)`), `b0afb20` (`fix(data): prevent cross-journal QSO mutations (DATA-004 DATA-005 CONC-001)`), `9157e0e` (`fix(data): isolate async and derived state by journal (DATA-004 DATA-005)`)
- **ModuĹ‚ / obszar:** `src/core/database/stats.rs`, `src/core/database/qsl.rs`, `src/gui/app_layout.rs`, `src/gui/online_sync.rs`, `src/gui/statistics.rs`, `src/gui/qsl_designer.rs`, `src/gui/app.rs`
- **Lokalizacja:** [`src/gui/app_layout.rs:602-618`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_layout.rs#L602-L618), [`src/core/database/stats.rs:9-144`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/stats.rs#L9-L144), [`src/core/database/qsl.rs:20-27`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qsl.rs#L20-L27), [`src/gui/online_sync.rs:114, 202, 288, 330, 362, 443, 485`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L114-L485), [`src/gui/qsl_designer.rs:98`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/qsl_designer.rs#L98), [`src/gui/app.rs:668`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app.rs#L668)
- **DowĂłd z kodu:**
  1. PrzeĹ‚Ä…czenie aktywnego dziennika w `app_layout.rs:602-618` wywoĹ‚uje tylko `self.reload_qsos()`, ale **nie wywoĹ‚uje `self.rebuild_awards_full()`** ani nie przebudowuje `ScpEngine`, wiÄ™c wskaĹşniki ATNO / New Band w formularzu nadal pokazujÄ… stan poprzedniego dziennika!
  2. `online_sync.rs` wywoĹ‚uje `db.get_recent_qsos(10000)` (bez filtra `journal_id`) przy wysyĹ‚aniu do LoTW (`:114`), eQSL (`:202`), ClubLog (`:288`), QRZ (`:330`), Cloudlog (`:443`) i HRDLog (`:485`), wysyĹ‚ajÄ…c Ĺ‚Ä…cznoĹ›ci ze wszystkich dziennikĂłw naraz (np. testowego lub SWL) i obcinajÄ…c eksport do 10 000 QSO!
  3. `stats.rs:9-144` i `statistics.rs:135-170` zliczajÄ… statystyki ze wszystkich dziennikĂłw Ĺ‚Ä…cznie.
  4. `Drop for SpLogApp` (`src/gui/app.rs:668`) wywoĹ‚uje `db.get_recent_qsos(5000)`, obcinajÄ…c automatycznÄ… kopiÄ™ `SPLogbook_auto_backup.adi` przy zamkniÄ™ciu programu do 5000 QSO!
- **Opis problemu:** Dzienniki (`JournalProfile`) sÄ… odizolowane jedynie w tabeli gĹ‚Ăłwnej, natomiast dyplomy, statystyki, chmura i QSL mieszajÄ… wszystkie dzienniki i stosujÄ… arbitralne limity `5000`/`10000`.
- **Scenariusz wystÄ…pienia:** UĹĽytkownik tworzy osobny dziennik (np. dla innego znaku klubowego lub aktywacji) i uruchamia synchronizacjÄ™ z LoTW/QRZ lub sprawdza statystyki.
- **WpĹ‚yw:** WysĹ‚anie Ĺ‚Ä…cznoĹ›ci z niewĹ‚aĹ›ciwego dziennika/znaku do LoTW/QRZ/ClubLog, przekĹ‚amane podpowiedzi ATNO oraz niepeĹ‚na kopia zapasowa ADIF powyĹĽej 5000 QSO.
- **RozwiÄ…zanie minimalne:**
  1. W `app_layout.rs:610` po zmianie `active_journal` wywoĹ‚ywaÄ‡ `self.rebuild_awards_full()`.
  2. W `online_sync.rs`, `qsl_designer.rs` i `statistics.rs` pobieraÄ‡ QSO przez `db.get_qsos_for_journal(&journal_id, usize::MAX)` zamiast `db.get_recent_qsos(10000)`.
  3. W `Drop for SpLogApp` (`src/gui/app.rs:668`) uĹĽyÄ‡ `db.get_all_qsos()` zamiast `db.get_recent_qsos(5000)`.
- **RozwiÄ…zanie docelowe:** Przekazywanie `journal_id` do wszystkich metod `stats.rs`, `qsl.rs` i `online_sync.rs` oraz zapisywanie ostatnio wybranego `active_journal_id` w `AppConfig`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `switching_journal_rebuilds_awards_engine_and_isolates_stats`.
- **Kryterium akceptacji:** Zmiana dziennika przebudowuje `AwardsEngine`, a eksporty chmurowe i kopia ADIF operujÄ… bez sztywnego obciÄ™cia do 5000/10000 rekordĂłw.

---

### `DATA-006` â€” Niebezpieczny fallback `fs::copy` na ĹĽywej bazie SQLite WAL oraz zbyt szerokie usuwanie w `prune_old_backups`
- **ID:** `DATA-006`
- **TytuĹ‚:** Ryzyko uszkodzonej kopii zapasowej przez `fs::copy` na pliku `.db` w trybie WAL i bĹ‚Ä™dny filtr w `prune_old_backups`
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia:** 2026-11-03
- **Commity:** `066db3d` (`fix(storage): create validated atomic SQLite backups (DATA-006)`), `eecf4d3` (`fix(storage): restore backups through validated staging (DATA-006)`)
- **ModuĹ‚ / obszar:** `src/core/backup.rs`
- **Lokalizacja:** [`src/core/backup.rs:84-89`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/backup.rs#L84-L89), [`src/core/backup.rs:116-144`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/backup.rs#L116-L144)
- **DowĂłd z kodu:**
  1. W `create_backup_internal` (`src/core/backup.rs:84-89`), gdy `Self::sqlite_online_backup` zwrĂłci bĹ‚Ä…d, kod przechodzi do `fs::copy(db_path, &backup_path)`. Dla bazy otwartej w trybie `PRAGMA journal_mode = WAL` surowa kopia samego pliku `.db` bez plikĂłw `-wal`/`-shm` podczas aktywnej transakcji tworzy niespĂłjny lub niekompletny obraz bazy.
  2. `prune_old_backups` (`src/core/backup.rs:126`) filtruje pliki wyĹ‚Ä…cznie po `name.contains("_backup_") && name.ends_with(".db")`, ignorujÄ…c nazwÄ™ bazowÄ… (`stem`), przez co moĹĽe usunÄ…Ä‡ kopie innych baz w tym samym katalogu.
- **Opis problemu:** Ukrycie bĹ‚Ä™du SQLite Online Backup API poprzez niebezpieczny `fs::copy` oraz brak filtrowania po prefiksie `stem` przy rotacji kopii.
- **Scenariusz wystÄ…pienia:** Chwilowa blokada pliku docelowego podczas `sqlite_online_backup` przy aktywnym zapisie WAL.
- **WpĹ‚yw:** Ryzyko powstania uszkodzonej kopii zapasowej uznanej przez program za poprawnÄ… (`Ok(backup_path)`).
- **RozwiÄ…zanie minimalne:** UsunÄ…Ä‡ fallback `fs::copy` (zwracaÄ‡ bĹ‚Ä…d `Err`, jeĹ›li `rusqlite::backup::Backup` z `busy_timeout` zawiedzie) oraz filtrowaÄ‡ w `prune_old_backups` po `name.starts_with(&format!("{stem}_backup_"))`.
- **RozwiÄ…zanie docelowe:** Wykonywanie `sqlite_online_backup` bezpoĹ›rednio na istniejÄ…cym poĹ‚Ä…czeniu `Database` lub z `busy_timeout(Duration::from_secs(5))`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `prune_old_backups_only_removes_matching_database_stem`.
- **Kryterium akceptacji:** Kopie zapasowe powstajÄ… wyĹ‚Ä…cznie przez transakcyjne API `rusqlite::backup::Backup`, a rotacja nie dotyka plikĂłw o innym prefiksie.

---

### `DATA-007` â€” Martwa tabela `upload_queue` w schemacie SQLite oraz 6 ostrzeĹĽeĹ„ leksera w `qso_store.rs`
- **ID:** `DATA-007`
- **TytuĹ‚:** NieuĹĽywana tabela `upload_queue` w `schema.rs` i ostrzeĹĽenia kompilatora `multiple lines skipped by escaped newline` w `qso_store.rs`
- **Kategoria:** `DATA`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED` (w zakresie 6 ostrzeĹĽeĹ„ leksera SQL; wykorzystanie tabeli `upload_queue` przeniesione do kategorii E / Nice-to-Have jako ulepszenie przyrostowej kolejki retry)
- **Data wdroĹĽenia:** 2026-11-01
- **Commit:** `5e61a78` (`fix(database): clean multiline SQL literals (DATA-007)`)
- **ModuĹ‚ / obszar:** `src/core/database/schema.rs`, `src/core/database/qso_store.rs`
- **Lokalizacja:** [`src/core/database/schema.rs:76-86`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/schema.rs#L76-L86), [`src/core/database/qso_store.rs:208, 252, 254, 278, 310, 312`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/database/qso_store.rs#L208-L315)
- **DowĂłd z kodu:**
  1. Tabela `upload_queue` jest tworzona w `schema.rs:76-86`, ale w caĹ‚ym repozytorium nie ma ani jednego zapytania `INSERT`, `SELECT` ani `DELETE` odwoĹ‚ujÄ…cego siÄ™ do `upload_queue`.
  2. `cargo check` i `cargo test` emitowaĹ‚y 6 ostrzeĹĽeĹ„ kompilatora w `src/core/database/qso_store.rs` (linie 208, 252, 254, 278, 310, 312) z powodu pustych linii po znaku kontynuacji `\` w literaĹ‚ach SQL.
- **Opis problemu:** Szum kompilacyjny i niepodĹ‚Ä…czona tabela kolejki wysyĹ‚ek chmurowych.
- **Scenariusz wystÄ…pienia:** KaĹĽda kompilacja projektu (`cargo check`, `cargo test`, `cargo clippy`).
- **WpĹ‚yw:** Zaciemnienie wyjĹ›cia kompilatora przez 6 ostrzeĹĽeĹ„ leksera; brak trwaĹ‚ej kolejki retry dla nieudanych wysyĹ‚ek QSL do chmury.
- **RozwiÄ…zanie minimalne:** UsunÄ…Ä‡ puste linie po `\` w literaĹ‚ach SQL w `src/core/database/qso_store.rs`.
- **RozwiÄ…zanie docelowe:** PodĹ‚Ä…czyÄ‡ `upload_queue` do automatycznego ponawiania wysyĹ‚ek Cloudlog/ClubLog/QRZ lub udokumentowaÄ‡ tabelÄ™ jako zarezerwowanÄ… pod przyszĹ‚Ä… kolejkÄ™ offline.
- **Wynik remediacji:**
  - UsuniÄ™to 6 pustych linii po znaku kontynuacji `\` w literaĹ‚ach SQL w `qso_numbers_for_journal`, `get_distinct_callsigns`, `existing_identities`, `has_identity` w `src/core/database/qso_store.rs`.
  - Semantyka i tokeny SQL: w 100% identyczne (brak modyfikacji sĹ‚Ăłw kluczowych SQL, tabel, kolumn, klauzul WHERE, JOIN, ORDER BY, LIMIT ani liczby/kolejnoĹ›ci parametrĂłw `?1`, `?2`).
  - OstrzeĹĽenia kompilatora: 0 wynikĂłw dla `multiple lines skipped by escaped newline` w `cargo check --workspace --all-targets --all-features`.
  - Wyniki Clippy: spadek liczby ostrzeĹĽeĹ„ w `splogbook` (lib) z 345 do 339 (dokĹ‚adnie o 6) oraz w `splogbook` (lib test) z 422 do 416 (dokĹ‚adnie o 6).
  - Testy regresyjne: 30/30 testĂłw `core::database` i 35/35 testĂłw `core::ingest` zielonych; formatowanie `cargo fmt --all -- --check` zgodne.
- **Ryzyko wdroĹĽenia zmiany:** Zerowe.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `cargo check --workspace --all-targets` z flagÄ… `RUSTFLAGS="-D warnings"` dla ostrzeĹĽeĹ„ kompilatora.
- **Kryterium akceptacji:** Zero ostrzeĹĽeĹ„ `multiple lines skipped by escaped newline` podczas kompilacji.

---

## 3. FunkcjonalnoĹ›Ä‡ i Domena KrĂłtkofalarska (`FUNC-*`)

### `FUNC-001` â€” Generator Cabrillo 3.0 tworzy niepoprawne logi konkursowe (odwrĂłcona kolejnoĹ›Ä‡, staĹ‚y RST, bĹ‚Ä™dne emisje i obciÄ™ta wymiana)
- **ID:** `FUNC-001`
- **TytuĹ‚:** Krytyczne niezgodnoĹ›ci eksportera Cabrillo 3.0 (`src/core/cabrillo.rs`) z regulaminami zawodĂłw HF/VHF
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia:** 2026-11-05
- **Commity:**
  - `0c0ca72`: `fix(contest): implement compliant Cabrillo 3.0 generation (FUNC-001)`
  - `e8e360e`: `fix(contest): enforce Cabrillo serial and transmitter policies`
  - `5e8fbc8`: `test(contest): add independent official scoring fixtures`
  - `73cac09`: `fix(contest): prevent invalid Cabrillo transmitter category output (FUNC-001)`
- **ModuĹ‚ / obszar:** `src/core/cabrillo.rs`, `src/core/contest_rules.rs`, `src/core/contest_stats.rs`, `src/gui/app_export.rs`
- **Lokalizacja:** [`src/core/cabrillo.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/cabrillo.rs), [`src/core/contest_rules.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_rules.rs)
- **DowĂłd z kodu przed naprawÄ…:**
  1. **OdwrĂłcona kolejnoĹ›Ä‡ i numeracja QSO (`cabrillo.rs:58`)**: `for (idx, qso) in input.qsos.iter().rev().enumerate()` â€” brak jawnego sortowania chronologicznego po `(qso_date, time_on)`.
  2. **Ignorowanie raportĂłw `rst_sent` / `rst_rcvd` (`cabrillo.rs:65`)**: Wymuszanie staĹ‚ych `599`/`59` i ignorowanie rzeczywistych raportĂłw RST.
  3. **BĹ‚Ä™dne mapowanie emisji (`cabrillo.rs:60-64`)**: Wszystkie Ĺ‚Ä…cznoĹ›ci cyfrowe i FM mapowane na `PH`.
  4. **Ignorowanie `stx_string` / `srx_string` (`cabrillo.rs:68-73`)**: Wymiana staĹ‚a `{idx + 1:03}`, ignorowanie strefy CQ, wojewĂłdztwa, numerĂłw seryjnych.
  5. **BĹ‚Ä™dny fallback czÄ™stotliwoĹ›ci (`cabrillo.rs:42-55`)**: Brak mapowania dla wielu pasm.
- **WdroĹĽone rozwiÄ…zanie (Etap 7 i Bramka Naprawcza):**
  1. **Format ogĂłlny Cabrillo 3.0**: NagĹ‚Ăłwek Cabrillo v3 zawiera `START-OF-LOG: 3.0`, `CALLSIGN:`, `CONTEST:`, `CATEGORY-OPERATOR:`, `CATEGORY-TRANSMITTER:`, `CLAIMED-SCORE: <int>`, `CREATED-BY: SPLogbook`, oraz `END-OF-LOG:`. ZgodnoĹ›Ä‡ ze specyfikacjÄ… Cabrillo v3.0.
  2. **ObsĹ‚ugiwane kody emisji**: `CW` (`CW`), `PH` (`SSB, USB, LSB, AM`), `FM` (`FM`), `RY` (`RTTY`), `DG` (`FT8, FT4, PSK, etc.`). Mapowanie standardowe z walidacjÄ… dopuszczalnoĹ›ci emisji per profil contestu (`validate_mode`).
  3. **Walidacja specyficzna dla zawodĂłw**: Pasma (`validate_band`), emisje (`validate_mode`) oraz wymiany (`validate_exchange`) sÄ… walidowane pod kÄ…tem aktywnego `ContestProfile`. Emisje/pasma niedozwolone w danym profilu sÄ… odrzucane z typowanym bĹ‚Ä™dem `ContestError::DisallowedBand` / `ContestError::DisallowedMode`.
  4. **Weryfikacja wiersza QSO**: Kolumny `QSO: freq mode date time sent_call sent_rst sent_exch rcvd_call rcvd_rst rcvd_exch [tx_id]`. CzÄ™stotliwoĹ›Ä‡ podawana w kHz (np. `14000`, `7000`, `3500`, `21000`, `28000`).
  5. **ĹąrĂłdĹ‚o wymiany kontrolnej**: Priorytet ma rzeczywista zarejestrowana wymiana `stx_string` -> `stx` -> fallback rosnÄ…cy numer seryjny (`resolve_sent_exchange`). Otrzymana wymiana pobierana z `srx_string` -> `srx` (`resolve_rcvd_exchange`). Brak fabrykowania brakujÄ…cych wymaganych wymian (wojewĂłdztwo w SP DX, strefa w CQ WW sÄ… walidowane).
  6. **Polityka RST**: Rzeczywiste raporty `rst_sent` i `rst_rcvd` sÄ… bezwzglÄ™dnie zachowywane (w tym raporty niestandardowe np. `579`, `-10`). DomyĹ›lne `599`/`59` stosowane jest wyĹ‚Ä…cznie, gdy pole raportu jest puste (`cabrillo_uses_default_rst_only_when_missing`). Raporty nieliczbowe lub o bĹ‚Ä™dnej dĹ‚ugoĹ›ci sÄ… odrzucane (`ContestError::MalformedRst`).
  7. **Polityka numeracji seryjnej**: Kategoria `Single-Op` oraz `Multi-One` uĹĽywajÄ… pojedynczej rosnÄ…cej sekwencji seryjnej sprawdzanej przez `validate_cqwpx_serial_policy`.
  8. **Polityka kategorii nadajnikĂłw i operatorĂłw**:
     - Rozdzielono pojÄ™cia `CATEGORY-OPERATOR` (`CabrilloOperatorCategory`: `SingleOp`, `MultiOp`, `Checklog`) oraz `CATEGORY-TRANSMITTER` (`CabrilloTransmitterCategory`: `One`, `Two`, `Unlimited`).
     - W formacie Cabrillo 3.0 (WWROF) wartoĹ›Ä‡ `MULTI` dla linii `CATEGORY-TRANSMITTER:` jest bezwzglÄ™dnie nieprawidĹ‚owa (`MULTI-OP` naleĹĽy wyĹ‚Ä…cznie do `CATEGORY-OPERATOR`). UsuniÄ™to wadliwy wariant `Multi` z typu kategorii nadajnikĂłw.
     - Dla stacji z jednym nadajnikiem emitowany jest poprawny nagĹ‚Ăłwek `CATEGORY-TRANSMITTER: ONE`, a kolumna identyfikatora nadajnika `t` w wierszach `QSO:` jest pomijana zgodnie ze standardem.
     - Kategorie wielonadajnikowe `Two` oraz `Unlimited` sÄ… bezpiecznie odrzucane z typowanym bĹ‚Ä™dem `ContestError::UnsupportedContestCategory`, zapobiegajÄ…c emisji niezgodnych logĂłw bez dedykowanego modelu per-band / multi-tx (planowane w wersji 2.0).
  9. **Zachowanie duplikatĂłw**: ĹÄ…cznoĹ›ci zduplikowane (duplikaty) nie sÄ… po cichu usuwane z logu Cabrillo â€” pozostajÄ… w wyjĹ›ciowym logu zgodnie z regulaminami zawodĂłw CQ/PZK/ARRL (komisja sÄ™dziowska punktuje je jako 0 pkt).
  10. **CLAIMED-SCORE**: WartoĹ›Ä‡ jest bezpoĹ›rednio powiÄ…zana z wynikiem punktowym `Scoreboard::total` z `ContestProfile` i formatowana jako czysta liczba caĹ‚kowita ASCII bez separatorĂłw tysiÄ™cy (`claimed_score_is_integer_without_formatting`).
- **Ryzyko szczÄ…tkowe:** Brak fizycznej obsĹ‚ugi logowania z wielu nadajnikĂłw na rĂłĹĽnych pasmach w v1.x (zabezpieczone odmowÄ… eksportu `UnsupportedContestCategory`).
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `FUNC-002`.
- **WdroĹĽone testy regresyjne:**
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
- **Kryterium akceptacji:** SpeĹ‚nione w caĹ‚oĹ›ci â€” format Cabrillo 3.0, kody emisji, zachowanie RST, obsĹ‚uga wymian, monotoniczne serie numerĂłw, obsĹ‚uga kategorii nadajnika i zachowanie duplikatĂłw zweryfikowane pakietem testĂłw automatycznych.

---

### `FUNC-002` â€” BĹ‚Ä™dne reguĹ‚y punktacji i mnoĹĽnikĂłw w `src/core/contest_rules.rs` (SP DX Contest, CQ WW, CQ WPX)
- **ID:** `FUNC-002`
- **TytuĹ‚:** BĹ‚Ä™dna punktacja Ĺ‚Ä…cznoĹ›ci zagranicznych oraz zliczanie polskich stacji jako mnoĹĽnikĂłw DXCC w SP DX Contest
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia:** 2026-11-05
- **Commity:**
  - `29a4e36`: `fix(contest): refine scoring and multipliers for SP DX, CQ WW and CQ WPX (FUNC-002)`
  - `9a6de52`: `fix(contest): correct CQ WPX CW and SSB scoring (FUNC-002)`
  - `966e59f`: `fix(contest): separate CQ WPX and CQ WPX RTTY rule profiles (FUNC-002)`
  - `31d588f`: `fix(contest): validate contest-specific bands modes and exchanges`
  - `5e8fbc8`: `test(contest): add independent official scoring fixtures`
  - `ed94f6f`: `fix(contest): correct CQ WW official scoring fixture (FUNC-002)`
  - `df5c68b`: `fix(contest): enforce official SP DX province exchanges (FUNC-002)`
- **ModuĹ‚ / obszar:** `src/core/contest_rules.rs`, `src/core/contest_stats.rs`, `src/core/cabrillo.rs`
- **Lokalizacja:** [`src/core/contest_rules.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_rules.rs), [`src/core/contest_stats.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/contest_stats.rs)
- **DowĂłd z kodu przed naprawÄ…:**
  1. `score_sp_dx`: Brak prawidĹ‚owego rozrĂłĹĽnienia stacji polskiej vs zagranicznej oraz bĹ‚Ä™dna walidacja 16 wojewĂłdztw w wymianie.
  2. `score_cqww`: Uproszczone prefiksy zamiast porĂłwnania encji DXCC i kontynentĂłw.
  3. `score_wpx`: BĹ‚Ä™dna reguĹ‚a podwajania punktĂłw dla tego samego kraju na pasmach niskich w zawodach CW/SSB (przyznawano 2 punkty zamiast 1 punktu).
  4. Brak rozdzielenia profili CQ WPX CW/SSB od CQ WPX RTTY.
- **WdroĹĽone rozwiÄ…zanie (Etap 7 i Bramka Naprawcza):**
  1. **Edycja regulaminu SP DX Contest (PZK)**:
     - Stacja polska (`my_dxcc == 269`): praca z EuropÄ… = 1 pkt, praca poza EuropÄ… = 3 pkt, Ĺ‚Ä…cznoĹ›Ä‡ ze stacjÄ… SP = 0 pkt (duplikat/brak punktĂłw). MnoĹĽnik: unikalne podmioty DXCC na kaĹĽdym paĹ›mie.
     - Stacja zagraniczna (`my_dxcc != 269`): punktowane sÄ… wyĹ‚Ä…cznie Ĺ‚Ä…cznoĹ›ci ze stacjami SP = 3 pkt. MnoĹĽnik: dokĹ‚adnie 16 oficjalnych, jednoliterowych oznaczeĹ„ polskich wojewĂłdztw (`B, C, D, F, G, J, K, L, M, O, P, R, S, U, W, Z`) na kaĹĽdym paĹ›mie.
     - PeĹ‚na kanoniczna mapa wojewĂłdztw SP DX:
       | Kod | WojewĂłdztwo |
       |---|---|
       | B | lubuskie |
       | C | Ĺ‚Ăłdzkie |
       | D | dolnoĹ›lÄ…skie |
       | F | pomorskie |
       | G | Ĺ›lÄ…skie |
       | J | warmiĹ„sko-mazurskie |
       | K | podkarpackie |
       | L | lubelskie |
       | M | maĹ‚opolskie |
       | O | podlaskie |
       | P | kujawsko-pomorskie |
       | R | mazowieckie |
       | S | Ĺ›wiÄ™tokrzyskie |
       | U | opolskie |
       | W | wielkopolskie |
       | Z | zachodniopomorskie |
     - Walidacja wojewĂłdztw: kody sÄ… normalizowane do wielkiej litery (case-insensitive). Niedozwolone sÄ… jakiekolwiek dwuliterowe skrĂłty administracyjne (np. `MA`, `LD`, `DS`, `PM`, `SL`), nieoficjalne litery, cyfry, znaki interpunkcyjne lub wartoĹ›ci wieloznakowe â€” zwracajÄ… `ContestError::InvalidContestExchange`. Pusta wymiana zwraca `ContestError::MissingContestExchange`.
     - Poprawiono manualne fixture stacji zagranicznej:
       - SP1AA, 80m CW: exchange R (wojewĂłdztwo: mazowieckie), punkty: 3, mnoĹĽnik: R na 80m.
       - SP2AA, 40m CW: exchange C (wojewĂłdztwo: Ĺ‚Ăłdzkie), punkty: 3, mnoĹĽnik: C na 40m.
       - DL2AA, 20m CW: exchange 001, stacja spoza SP, punkty: 0, mnoĹĽnik: brak.
       - SP3AA, 20m CW: exchange R (wojewĂłdztwo: mazowieckie), punkty: 3, mnoĹĽnik: R na 20m.
       Podsumowanie punktacji stacji zagranicznej:
       Punkty: 3 + 3 + 0 + 3 = 9
       MnoĹĽniki: R na 80m, C na 40m, R na 20m (liczba mnoĹĽnikĂłw: 3).
       Wynik koĹ„cowy: 9 Ă— 3 = 27 (CLAIMED-SCORE: 27).
     - Fixture stacji polskiej: SP1AA, wojewĂłdztwo R (mazowieckie). Oficjalne przypisanie kodĂłw: R = mazowieckie, C = Ĺ‚Ăłdzkie, K = podkarpackie, P = kujawsko-pomorskie. Niedozwolone sÄ… jakiekolwiek dwuliterowe kody wojewĂłdztw.
     - Dozwolone pasma: 160m, 80m, 40m, 20m, 15m, 10m. Dozwolone emisje: CW, SSB.
  2. **Edycja regulaminu CQ WW DX Contest (CQ)**:
     - WĹ‚asny podmiot DXCC: 0 pkt (kwalifikuje siÄ™ do mnoĹĽnika strefy/kraju).
     - Ten sam kontynent, inny podmiot: 1 pkt (wyjÄ…tek Ameryki PĂłĹ‚nocnej: 2 pkt).
     - Inny kontynent: 3 punkty bez wzglÄ™du na pasmo (zasada obowiÄ…zuje jednakowo na wszystkich dozwolonych pasmach: 160m, 80m, 40m, 20m, 15m i 10m).
     - CQ WW DX Contest â€” rÄ™cznie policzone fixture:
       Stacja bazowa: SP1AA (DXCC: SP, kontynent: EU, strefa CQ: 15).
       QSO:
       1. SP2AA, 80m: relacja: ten sam kraj; punkty: 0; mnoĹĽniki: Zone 15 na 80m, SP na 80m.
       2. DL1AA, 80m: relacja: ten sam kontynent, inny kraj; punkty: 1; mnoĹĽniki: Zone 14 na 80m, DL na 80m.
       3. W1AW, 20m: relacja: inny kontynent; punkty: 3; mnoĹĽniki: Zone 5 na 20m, K na 20m.
       4. JA1AA, 20m: relacja: inny kontynent; punkty: 3; mnoĹĽniki: Zone 25 na 20m, JA na 20m.
       5. W2AA, 40m: relacja: inny kontynent; punkty: 3; mnoĹĽniki: Zone 5 na 40m, K na 40m.
       Podsumowanie punktacji:
       QSO points: 0 + 1 + 3 + 3 + 3 = 10
       Zone multipliers: 5
       Country multipliers: 5
       All multipliers: 5 + 5 = 10
       Final score: 10 Ă— 10 = 100
       CLAIMED-SCORE: 100
       (Zone 5 i kraj K na 20m oraz Zone 5 i kraj K na 40m sÄ… osobnymi mnoĹĽnikami, poniewaĹĽ mnoĹĽniki CQ WW liczone sÄ… oddzielnie na kaĹĽdym paĹ›mie).
     - ObsĹ‚uga Ĺ‚Ä…cznoĹ›ci zduplikowanych (DUPLICATE QSOS):
       - pozostajÄ… w logu Cabrillo, jeĹ›li wymaga tego kontrakt eksportu;
       - nie zwiÄ™kszajÄ… liczby punktĂłw (0 dodatkowych punktĂłw);
       - nie zwiÄ™kszajÄ… liczby mnoĹĽnikĂłw (0 nowych mnoĹĽnikĂłw);
       - nie zmieniajÄ… wyniku koĹ„cowego (pozostaje 100 punktĂłw).
       Dla piÄ™ciu QSO podstawowych oraz ich duplikatĂłw: podstawowe QSO dajÄ… 10 punktĂłw i 10 mnoĹĽnikĂłw, duplikaty dajÄ… 0 dodatkowych punktĂłw i 0 nowych mnoĹĽnikĂłw, wynik wynosi 100. Duplikaty nie mogÄ… byÄ‡ uĹĽywane do uzasadniania 20 punktĂłw QSO.
     - MnoĹĽniki: CQ Zones (1..40) na kaĹĽdym paĹ›mie oraz kraje DXCC na kaĹĽdym paĹ›mie.
     - Emisje rozdzielone per profil: osobny profil CQ WW CW, osobny profil CQ WW SSB. Dozwolone pasma: 160m, 80m, 40m, 20m, 15m, 10m.
  3. **Edycja regulaminu CQ WPX CW/SSB (CQ)**:
     - **Ten sam kraj (same-country): 1 punkt na kaĹĽdym paĹ›mie** (brak podwojenia na pasmach niskich!).
     - Ten sam kontynent, inny kraj: 1 pkt pasma wysokie (10m, 15m, 20m), 2 pkt pasma niskie (40m, 80m, 160m).
     - Ameryka PĂłĹ‚nocna inny kraj (NA exception): 2 pkt pasma wysokie (10m, 15m, 20m), 4 pkt pasma niskie (40m, 80m, 160m).
     - Inny kontynent: 3 pkt pasma wysokie (10m, 15m, 20m), 6 pkt pasma niskie (40m, 80m, 160m).
     - MnoĹĽnik prefiksĂłw WPX: unikalny prefiks zaliczany **dokĹ‚adnie raz w caĹ‚ych zawodach** (nie per pasmo).
     - Dozwolone pasma: 160m, 80m, 40m, 20m, 15m, 10m.
  4. **Status CQ WPX RTTY i CQ WW RTTY**:
     - Profile RTTY sÄ… bezwzglÄ™dnie odseparowane w typie `ContestId::CqWpxRtty` oraz `ContestId::CqWwRtty`.
     - PrĂłba kalkulacji lub eksportu zwraca jawny, typowany bĹ‚Ä…d `ContestError::UnsupportedContestVariant`. Wykluczone jest jakiekolwiek niejawne korzystanie z reguĹ‚ CW/SSB.
  5. **Walidacja specyficzna dla zawodĂłw**:
     - Pasma, emisje i wymiany podlegajÄ… Ĺ›cisĹ‚ej walidacji przed punktacjÄ… i eksportem. ĹÄ…cznoĹ›ci poza pasmami zawodĂłw lub w niedozwolonych emisjach nie sÄ… punktowane i powodujÄ… zgĹ‚oszenie bĹ‚Ä™du.
  6. **Klucze unikalnoĹ›ci duplikatĂłw (dedup key)**:
     - SP DX / CQ WW: znak + pasmo + emisja (w zawodach wieloemisyjnych jak SP DX dozwolona jest Ĺ‚Ä…cznoĹ›Ä‡ na tym samym paĹ›mie na CW i na SSB).
     - CQ WPX: znak + pasmo (w zawodach WPX dana stacja moĹĽe byÄ‡ zrobiona tylko raz na paĹ›mie).
  7. **NiezaleĹĽne fixtures testowe**:
     - Wprowadzono niezaleĹĽne testy weryfikujÄ…ce reguĹ‚y punktacji z rÄ™cznie przeliczonymi wartoĹ›ciami z oficjalnych regulaminĂłw PZK i CQ, weryfikujÄ…ce `Scoreboard` oraz format i wartoĹ›Ä‡ pola `CLAIMED-SCORE`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `FUNC-001`.
- **WdroĹĽone testy regresyjne:**
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
- **Kryterium akceptacji:** SpeĹ‚nione â€” reguĹ‚y SP DX, CQ WW, CQ WPX CW/SSB, odseparowanie RTTY, walidacja emisji i pasm oraz niezaleĹĽne fixtures sÄ… w 100% zweryfikowane testami automatycznymi.

---

### `FUNC-003` â€” Endpoint `POST /api/v1/rig` w REST API jest atrapÄ… (mutuje tylko RAM i jest nadpisywany w kolejnej klatce GUI)
- **ID:** `FUNC-003`
- **TytuĹ‚:** `post_rig_control` (`POST /api/v1/rig`) nie wysyĹ‚a ĹĽadnych rozkazĂłw do `CatRuntime` ani `RigHub`
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-05
- **Commity:** `36d1dee` (`feat(api): wire REST rig control through CatRuntime and add multi-rig endpoints (FUNC-003)`), `833f28d` (`fix(api): harden REST rig control security boundary (FUNC-003)`)
- **ModuĹ‚ / obszar:** `src/api/server.rs`, `src/gui/app_async.rs`
- **Lokalizacja:** [`src/api/server.rs:1088-1141`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/api/server.rs#L1088-L1141), [`src/gui/app_async.rs:248-252`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_async.rs#L248-L252)
- **DowĂłd z kodu:** Funkcja `post_rig_control` (`src/api/server.rs:1092-1116`) zapisuje przysĹ‚ane pola (`frequency_hz`, `mode`, `ptt` itd.) wyĹ‚Ä…cznie do lokalnego `state.rig_state.write()` (`Arc<RwLock<RigState>>`) i zwraca `200 OK`. `ApiState` (`src/api/server.rs:35-45`) nie posiada nawet uchwytu do `CatRuntime`! Co wiÄ™cej, w nastÄ™pnej klatce GUI (~16 ms pĂłĹşniej) `SpLogApp::poll_cat_outcomes` (`src/gui/app_async.rs:248-252`) nadpisuje `shared_rig_state` z powrotem stanem z `self.cat.rig_state`.
- **Opis problemu:** ZewnÄ™trzne integracje wywoĹ‚ujÄ…ce `POST /api/v1/rig` otrzymujÄ… faĹ‚szywÄ… odpowiedĹş o zmianie czÄ™stotliwoĹ›ci/trybu/PTT, podczas gdy fizyczne radio nie otrzymuje ĹĽadnej komendy.
- **Scenariusz wystÄ…pienia:** Skrypt zewnÄ™trzny lub aplikacja kliencka wysyĹ‚a `POST /api/v1/rig {"frequency_hz": 14074000, "mode": "FT8"}`.
- **WpĹ‚yw:** CaĹ‚kowity brak dziaĹ‚ania zdalnego sterowania radiem przez REST API.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ `cat_runtime: CatRuntime` do `ApiState` i w `post_rig_control` wysyĹ‚aÄ‡ odpowiednie komendy `RigCommand` (`SetFrequency`, `SetMode`, `SetPassband`, `SetVfo`, `SetSplit`) oraz `request_tx`/`release_tx` dla `ptt`.
- **RozwiÄ…zanie docelowe:** Synchroniczne potwierdzenie wykonania komendy z `CatRuntime` przed zwrĂłceniem `RigStateResponse`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `CAT-008`.
- **Wymagany test regresyjny:** `api_post_rig_control_dispatches_commands_to_cat_runtime`.
- **Kryterium akceptacji:** WywoĹ‚anie `POST /api/v1/rig` przekazuje rozkazy do `CatRuntime` / `RigHub` i przestrzega blokad `TxArbiter`.

---

### `FUNC-004` â€” Funkcje atrapowe, czÄ™Ĺ›ciowo podĹ‚Ä…czone lub nieosiÄ…galne w GUI i backendzie (CW/WinKeyer, JS8Call, FLDigi, Audio Stop, EME Rotor, Photo Viewer)
- **ID:** `FUNC-004`
- **TytuĹ‚:** ZbiĂłr funkcji pozornych (sham/partially wired) widocznych w interfejsie uĹĽytkownika
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `9cb49dc`: `fix(digital): connect js8call listener, fldigi test and winkeyer frames (FUNC-004)`
  - `85104a1`: `fix(gui): wire audio recording stop photo preview and EME rotor (FUNC-004)`
  - `a5d9bde`: `fix(digital): implement winkeyer frames fldigi xmlrpc test and js8call listener (FUNC-004)`
- **ModuĹ‚ / obszar:** `src/cat/winkeyer.rs`, `src/gui/cw_terminal.rs`, `src/gui/app_plugins.rs`, `src/digital/js8call.rs`, `src/digital/fldigi.rs`, `src/gui/qso_entry.rs`, `src/gui/astronomy_dialog.rs`, `src/media/audio_recorder.rs`, `src/gui/photo_viewer.rs`, `src/gui/app_cat.rs`, `src/gui/app_services.rs`, `src/gui/online_sync.rs`
- **Lokalizacja:**
  1. **Terminal CW i Makra CW**: [`src/gui/cw_terminal.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/cw_terminal.rs), [`src/gui/app_plugins.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_plugins.rs), [`src/cat/winkeyer.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/winkeyer.rs).
  2. **JS8Call Listener**: [`src/gui/app_services.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_services.rs), [`src/digital/js8call.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/digital/js8call.rs).
  3. **FLDigi XML-RPC**: [`src/digital/fldigi.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/digital/fldigi.rs), [`src/gui/online_sync.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs).
  4. **Przycisk STOP nagrywania audio**: [`src/gui/qso_entry.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/qso_entry.rs), [`src/media/audio_recorder.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/media/audio_recorder.rs).
  5. **Przyciski rotatora EME (KsiÄ™ĹĽyc / SĹ‚oĹ„ce)**: [`src/gui/astronomy_dialog.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/astronomy_dialog.rs), [`src/gui/app_cat.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_cat.rs), [`src/cat/rotor.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/rotor.rs).
  6. **PodglÄ…d zdjÄ™cia QSL z formularza**: [`src/gui/qso_entry.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/qso_entry.rs), [`src/gui/photo_viewer.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/photo_viewer.rs).
- **DowĂłd z kodu przed naprawÄ…:**
  1. `WinkeyerProtocol` w `src/cat/winkeyer.rs` nigdy nie byĹ‚ wywoĹ‚ywany poza testem jednostkowym; w `cw_terminal.rs:78` przycisk â€žPoĹ‚Ä…czâ€ť przeĹ‚Ä…czaĹ‚ tylko `self.is_connected = !self.is_connected` bez otwierania transportu, a `transmit_cw_macro` (`app_plugins.rs:215`) ustawiaĹ‚o tylko tekst na pasku statusu.
  2. `js8call_enabled` byĹ‚o domyĹ›lnie `false` w `DigitalState::new()`, nie istniaĹ‚o w `AppConfig`, a zaznaczenie checkboxa w `welcome_wizard.rs:382` nastÄ™powaĹ‚o juĹĽ po wykonaniu `app.start_background()`, wiÄ™c `Js8CallListener::start` nigdy nie byĹ‚ uruchamiany w dziaĹ‚ajÄ…cym programie.
  3. W `FldigiClient` metody `get_frequency`, `get_mode`, `get_qso_data` nie byĹ‚y nigdzie wywoĹ‚ywane, a przycisk testu w `online_sync.rs` spawnowaĹ‚ zadanie, ktĂłrego wynik byĹ‚ porzucany.
  4. W `src/gui/qso_entry.rs:646` podczas trwajÄ…cego nagrywania klikniÄ™cie przycisku `âŹą STOP` wywoĹ‚ywaĹ‚o `AudioRecorder::stop_audio()` (zatrzymanie odtwarzania) zamiast `app.entry.audio_recorder.stop_recording()`.
  5. W `src/gui/astronomy_dialog.rs:233-236` przyciski â€žUstaw antenÄ™ na KsiÄ™ĹĽycâ€ť i â€žUstaw antenÄ™ na SĹ‚oĹ„ceâ€ť przypisywaĹ‚y tylko `app.cat.rotor_state.azimuth = ...` w RAM bez wywoĹ‚ania `app.rotate_antenna_to_el(az, el)`.
  6. W `src/gui/qso_entry.rs:589` klikniÄ™cie â€žPokaĹĽ kartÄ™ QSLâ€ť wywoĹ‚ywaĹ‚o `photo_viewer_dialog.open(&call, None)` (zawsze `None`).
- **WdroĹĽone rozwiÄ…zanie (Etap 10):**
  1. **Zatrzymanie nagrywania audio (`AudioRecorder::stop_recording`)**: PodĹ‚Ä…czono `app.entry.audio_recorder.stop_recording()` w `qso_entry.rs`, wprowadzono wewnÄ™trzny trait `AudioSink` (umoĹĽliwiajÄ…cy deterministyczne mockowanie bez fizycznego urzÄ…dzenia audio), zapewniono oprĂłĹĽnienie bufora prĂłbek, finalizacjÄ™ nagĹ‚Ăłwka WAV, zwolnienie uchwytĂłw, idempotentnoĹ›Ä‡ drugiego wywoĹ‚ania i ograniczenie czasu zamkniÄ™cia workera.
  2. **PodglÄ…d zdjÄ™cia QSL (`PhotoViewerDialog`)**: Przekazano rzeczywistÄ… Ĺ›cieĹĽkÄ™ zdjÄ™cia z `qsl_gallery` w `qso_entry.rs`, dodano walidacjÄ™ odrzucajÄ…cÄ… puste ĹşrĂłdĹ‚a, jawne raportowanie bĹ‚Ä™dĂłw odczytu w GUI i czyszczenie stanu przejĹ›ciowego przy zamkniÄ™ciu okna.
  3. **Sterowanie rotatorem EME (`rotate_antenna_to_el`)**: PodĹ‚Ä…czono `app.rotate_antenna_to_el(az, el)` w `astronomy_dialog.rs`, wdroĹĽono walidacjÄ™ zakresu azymutu (`0.0..=360.0`) i elewacji (`-90.0..=90.0`) z odrzuceniem `NaN`/`Infinity`, wyeliminowano niejawny fallback na pierwsze skonfigurowane radio (`resolve_active_rig_config`) oraz dodano abstrakcjÄ™ `RotatorTransport` dla testĂłw offline.
  4. **JS8Call Listener**: Dodano pola `js8call_enabled` i `js8call_port` do `AppConfig` i `snapshot_config()`, wdroĹĽono dynamiczne metody `start_js8call_listener()` i `stop_js8call_listener()` z kontrolowanym tokenem zatrzymania oraz obsĹ‚ugÄ™ ramek TCP JSON Lines (`RIG.FREQ`, `RX.SPOT`, `RX.DIRECTED`, `LOG.QSO`) odpornÄ… na czÄ™Ĺ›ciowe i uszkodzone linie.
  5. **Test poĹ‚Ä…czenia FLDigi XML-RPC**: Dodano metodÄ™ `FldigiClient::test_connection()` odpytujÄ…cÄ… `fldigi.version` oraz `main.get_frequency` i podĹ‚Ä…czono wynik w `online_sync.rs` do kanaĹ‚u `online_sync_tx`, aktualizujÄ…c status GUI (`Online` z wersjÄ… i czÄ™stotliwoĹ›ciÄ… lub `Error` przy bĹ‚Ä™dzie/timeoutach).
  6. **Ramki i sesja WinKeyer (`WinkeyerProtocol`, `WinkeyerSession`)**: Dodano abstrakcjÄ™ `WinkeyerTransport` (z `MockWinkeyerTransport`), peĹ‚ne ramkowanie K1EL (`HostOpen 0x00 0x02`, `HostClose 0x00 0x03`, `SetWpm 0x02 <wpm>` z walidacjÄ… zakresu `5..=99`, `Abort 0x0A`, sanityzacja tekstu CW ASCII) oraz jawny bĹ‚Ä…d `RigError::NotConnected` w `cw_terminal.rs` i `transmit_cw_macro` przy braku skonfigurowanego transportu (zero cichego sukcesu).
- **WdroĹĽone testy regresyjne:**
  - `audio_recorder_stop_finalizes_mock_recording`, `audio_recorder_stop_flushes_pending_data`, `audio_recorder_stop_is_idempotent`, `audio_recorder_stop_releases_worker`, `audio_recorder_stop_releases_output_handle`, `audio_recorder_stop_propagates_finalize_error`, `audio_recorder_shutdown_is_bounded`, `audio_recorder_test_does_not_open_physical_device`
  - `photo_viewer_open_preserves_selected_source`, `photo_viewer_rejects_empty_source`, `photo_viewer_reports_invalid_source`, `photo_viewer_close_clears_transient_state_by_contract`, `photo_viewer_test_does_not_access_user_files`
  - `eme_rotator_sends_azimuth_and_elevation_to_mock_transport`, `eme_rotator_rejects_out_of_range_angles`, `eme_rotator_rejects_nan_and_infinite_angles`, `eme_rotator_does_not_fallback_to_wrong_device`, `eme_rotator_updates_ui_status_on_success_and_error`, `eme_rotator_test_does_not_move_physical_rotator`
  - `js8call_listener_starts_from_enabled_config`, `js8call_listener_receives_status_and_updates_state`, `js8call_listener_handles_partial_and_malformed_frames`, `js8call_listener_stops_cleanly_within_timeout`, `js8call_listener_reports_bind_or_connect_error`, `js8call_test_uses_local_loopback_only`
  - `fldigi_test_button_queries_mock_xmlrpc_and_updates_ui_status`, `fldigi_reads_frequency_and_mode_from_mock_xmlrpc`, `fldigi_handles_http_error_and_malformed_xml`, `fldigi_handles_timeout_cleanly`, `fldigi_test_uses_local_loopback_only`
  - `winkeyer_encodes_expected_command_frames`, `winkeyer_frame_is_written_to_mock_transport`, `winkeyer_rejects_invalid_speed_or_payload`, `winkeyer_unconfigured_transport_reports_explicit_error`, `winkeyer_disconnect_or_shutdown_is_clean`, `winkeyer_test_does_not_open_physical_serial_port`
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” kaĹĽda z 6 funkcji wykonuje peĹ‚ny przepĹ‚yw od akcji UI przez typowany interfejs backendu do mock/loopback transportu i widocznego stanu GUI.

---

### `FUNC-005` â€” Utrata czÄ™Ĺ›ci ustawieĹ„ po restarcie programu z powodu niekompletnego `SpLogApp::snapshot_config()`
- **ID:** `FUNC-005`
- **TytuĹ‚:** `SpLogApp::snapshot_config()` pomija zapis konfiguracji CW, REST API, alertĂłw solarnych, satelitĂłw, motywu z kreatora i aktywnego dziennika
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/gui/app_config.rs`, `src/gui/welcome_wizard.rs`, `src/gui/workspace.rs`
- **Lokalizacja:** [`src/gui/app_config.rs:28-59`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_config.rs#L28-L59), [`src/gui/welcome_wizard.rs:475-490`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/welcome_wizard.rs#L475-L490), [`src/gui/workspace.rs:146-164`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/workspace.rs#L146-L164)
- **DowĂłd z kodu:**
  1. `SpLogApp::snapshot_config()` (`src/gui/app_config.rs:28-59`) kopiuje wybrane pola do `cfg`, ale **nigdy nie kopiuje** zmian w:
     - `cfg.cw` (`wpm`, `macro_f1`..`macro_f8`),
     - `cfg.runtime.rest_api_enabled`, `cfg.runtime.rest_api_port`, `cfg.runtime.rest_api_token`,
     - `cfg.solar` (`band_alert_enabled`, `band_alert_min_Watts`, `band_alert_modes`),
     - `cfg.satellites`,
     - `cfg.ui.show_operator_assistant`.
  2. W `welcome_wizard.rs:478` i `workspace.rs:150` zmiana motywu ustawia `app.gui.theme.preset = ...` i wywoĹ‚uje `app.save_config_async()`, ale `snapshot_config()` czyta `self.config.ui.theme_preset` (ktĂłre nie zostaĹ‚o zaktualizowane!), wiÄ™c wybrany w kreatorze motyw przepada po restarcie!
- **Opis problemu:** Zmiany wprowadzone przez uĹĽytkownika w powyĹĽszych panelach dziaĹ‚ajÄ… tylko do zamkniÄ™cia programu.
- **Scenariusz wystÄ…pienia:** UĹĽytkownik zmienia motyw w kreatorze startowym lub konfiguruje makra CW / port REST API i restartuje program.
- **WpĹ‚yw:** FrustrujÄ…ca utrata skonfigurowanych ustawieĹ„ po kaĹĽdym restarcie aplikacji.
- **RozwiÄ…zanie minimalne:** UzupeĹ‚niÄ‡ `SpLogApp::snapshot_config()` o synchronizacjÄ™ `cfg.ui.theme_preset = self.gui.theme.preset.as_str().to_string()`, `cfg.ui.show_operator_assistant = self.gui.windows.show_operator_assistant` oraz wszystkich pĂłl `cw`, `runtime`, `solar` i `satellites`.
- **RozwiÄ…zanie docelowe:** Test round-trip sprawdzajÄ…cy zapis i odczyt wszystkich sekcji `AppConfig` po mutacji stanu `SpLogApp`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`.
- **Kryterium akceptacji:** Zmiana dowolnego ustawienia w GUI i wywoĹ‚anie `snapshot_config()` zachowuje nowÄ… wartoĹ›Ä‡ po ponownym wczytaniu `AppConfig`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `acecd8c`: `fix(config): make application settings durable and atomic`
  - `c4cf893`: `fix(config): migrate and preserve multi-radio settings`
  - `d564272`: `fix(config): redact REST token and harden secret handling (FUNC-005)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. W `SpLogApp::snapshot_config()` dodano kompletnÄ… synchronizacjÄ™ wszystkich struktur podrzÄ™dnych `AppConfig`: makra CW (`macro_f1`..`macro_f8`, `wpm`), ustawienia serwera REST API (`rest_api_enabled`, `rest_api_port`, `rest_api_token`), alerty pasmowe solarne (`band_alert_enabled`, `band_alert_min_Watts`, `band_alert_modes`), Ĺ›ledzenie satelitĂłw, stan widocznoĹ›ci okna Asystenta Operatora (`show_operator_assistant`), aktywny motyw (`theme_preset`) oraz konfiguracje profili radiowych Radio 1 i Radio 2.
  2. Wprowadzono walidacjÄ™ poprawnoĹ›ci schematu przed zapisem (`AppConfig::validate`), uniemoĹĽliwiajÄ…cÄ… serializacjÄ™ i zapis uszkodzonych wartoĹ›ci.
  3. Zastosowano atomowy zapis pliku konfiguracyjnego `config.json` z wykorzystaniem unikalnego pliku tymczasowego `.tmp` i atomowej zamiany `rename_atomic`, z zachowaniem kopii zapasowej `.bak`. W razie awarii I/O lub bĹ‚Ä™du walidacji pierwotny plik konfiguracyjny pozostaje w 100% nienaruszony.
  4. Dodano migracjÄ™ starszych konfiguracji jednoukĹ‚adowych do formatu z profilem multi-radio bez utraty parametrĂłw Ĺ‚Ä…cznoĹ›ci.
- **WdroĹĽone testy regresyjne:**
  - `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`
  - `config_atomic_write_preserves_previous_file_on_write_failure`
  - `multi_radio_settings_are_preserved_across_save_and_reload`
  - `legacy_single_radio_config_migrates_cleanly_to_multi_radio`
  - `rest_api_token_is_masked_in_gui_configuration`
  - `rest_api_token_is_redacted_in_logs_and_debug_output`

---

### `FUNC-006` â€” `ClubRegistry::check` w `src/core/clubs.rs` jest zahardkodowanÄ… makietÄ… dla 11 znakĂłw wywoĹ‚awczych
- **ID:** `FUNC-006`
- **TytuĹ‚:** Pozorna baza klubĂłw krĂłtkofalarskich (`ClubRegistry::check`) obsĹ‚ugujÄ…ca tylko 11 zahardkodowanych znakĂłw ze wspĂłĹ‚dzielonymi numerami czĹ‚onkowskimi
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `062d65c`: `fix(core): load club affiliations dynamically from reference database (FUNC-006)`
  - `caf72d1`: `fix(awards): enforce club uniqueness, complete awards engine and propagation UI (FUNC-006, FUNC-007)`
- **ModuĹ‚ / obszar:** `src/core/clubs.rs`, `databases/clubs.csv`, `README.md`
- **Lokalizacja:** [`src/core/clubs.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/clubs.rs), [`databases/clubs.csv`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/databases/clubs.csv), [`README.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/README.md)
- **DowĂłd z kodu przed naprawÄ…:** W `src/core/clubs.rs:36-81` metoda `ClubRegistry::check` sprawdzaĹ‚a wyĹ‚Ä…cznie 11 zahardkodowanych literaĹ‚Ăłw ze wspĂłĹ‚dzielonymi numerami czĹ‚onkowskimi (`SP-OTC #248` dla 4 znakĂłw, `SPCWC #104` dla 2 znakĂłw, `CWOPS #001` i `FOC #1850` dla 3 znakĂłw, `HSC #1940` dla 2 znakĂłw). Kluby SKCC i EPC nie wystÄ™powaĹ‚y w kodzie.
- **WdroĹĽone rozwiÄ…zanie (Etap 10):**
  1. UsuniÄ™to zahardkodowanÄ… listÄ™ 11 znakĂłw ze wspĂłĹ‚dzielonymi numerami z `src/core/clubs.rs`.
  2. Wprowadzono typowany model `ClubRegistry` Ĺ‚adowany dynamicznie z zewnÄ™trznego pliku referencyjnego `databases/clubs.csv` (`from_csv_file`, `from_csv_str`) z obsĹ‚ugÄ… wszystkich 7 klubĂłw (`SP-OTC`, `SPCWC`, `SKCC`, `CWOPS`, `FOC`, `HSC`, `EPC`).
  3. WdroĹĽono Ĺ›cisĹ‚Ä… walidacjÄ™ unikalnoĹ›ci par `(club, member_number)` (`ClubLoadError::DuplicateMemberNumber`), odrzucajÄ…cÄ… jakiekolwiek zduplikowane numery czĹ‚onkowskie w tym samym klubie.
  4. Dodano deterministycznÄ… normalizacjÄ™ znaku wywoĹ‚awczego (`normalize_callsign` wyodrÄ™bniajÄ…cy znak bazowy z sufiksĂłw `/P`, `/M`, `/QRP` oraz prefiksĂłw `DL/SP9ABC`).
  5. Zaktualizowano opis w `README.md`.
- **WdroĹĽone testy regresyjne:**
  - `club_registry_loads_from_reference_source`
  - `club_registry_normalizes_callsign_lookup`
  - `club_registry_does_not_assign_shared_fake_member_numbers`
  - `club_registry_returns_empty_for_unknown_callsign`
  - `club_registry_handles_missing_or_corrupt_source_gracefully`
  - `club_registry_lookup_is_deterministic`
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” `ClubRegistry` zwraca zweryfikowane dane z pliku referencyjnego i odrzuca wspĂłĹ‚dzielone numery czĹ‚onkowskie.

---

### `FUNC-007` â€” NieuĹĽywane pola dyplomowe `worked_wwff` i `worked_rda` w `AwardsEngine` oraz martwa historia propagacji w GUI
- **ID:** `FUNC-007`
- **TytuĹ‚:** Martwe kolekcje `worked_wwff`/`worked_rda` w `AwardsEngine` oraz brak prezentacji `PropagationHistory` w GUI
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `53d532a`: `fix(awards): support wwff references, display propagation accuracy and fix lotw since date (FUNC-007, FUNC-008)`
  - `caf72d1`: `fix(awards): enforce club uniqueness, complete awards engine and propagation UI (FUNC-006, FUNC-007)`
- **ModuĹ‚ / obszar:** `src/core/awards.rs`, `src/gui/awards_matrix.rs`, `src/core/propagation_history.rs`, `src/gui/solar_panel.rs`, `src/core/qsl_print.rs`, `src/gui/qsl_designer.rs`
- **Lokalizacja:** [`src/core/awards.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/awards.rs), [`src/gui/awards_matrix.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/awards_matrix.rs), [`src/core/propagation_history.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/propagation_history.rs), [`src/gui/solar_panel.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/solar_panel.rs), [`src/core/qsl_print.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/core/qsl_print.rs)
- **DowĂłd z kodu przed naprawÄ…:**
  1. W `AwardsEngine` zdefiniowano `worked_wwff` i `worked_rda`, ktĂłre nigdy nie byĹ‚y wypeĹ‚niane w `ingest_qso()` ani wyĹ›wietlane w `awards_matrix.rs`.
  2. `PropagationHistory` zbieraĹ‚a rekordy przy zapisie QSO, lecz metody `accuracy_rate()` i `band_accuracy()` nie byĹ‚y wywoĹ‚ywane w GUI.
  3. ModuĹ‚ `src/core/qsl_print.rs` nie byĹ‚ podĹ‚Ä…czony do `qsl_designer.rs`.
- **WdroĹĽone rozwiÄ…zanie (Etap 10):**
  1. W `AwardsEngine::ingest_qso` dodano walidacjÄ™, normalizacjÄ™ i zliczanie referencji `wwff_ref` (`is_valid_wwff_ref`, np. `SPFF-0001`) oraz `rda_ref` (`is_valid_rda_ref`, np. `MA-01` lub z pola `cnty`/`comment`), zliczanie potwierdzeĹ„ `confirmed_wwff` i `confirmed_rda` oraz prezentacjÄ™ w `AwardsSummary` i zakĹ‚adce `WWFF / RDA` w `awards_matrix.rs`.
  2. W `solar_panel.rs` dodano sekcjÄ™ prezentujÄ…cÄ… skutecznoĹ›Ä‡ prognoz `PropagationHistory` (`accuracy_rate()`, `band_accuracy()`, komunikat â€žBrak danychâ€ť dla pustej historii) z odpornoĹ›ciÄ… na dzielenie przez zero (`0.0`).
  3. W `qsl_designer.rs` zintegrowano `crate::core::qsl_print::calculate_total_sheets` do wyliczania liczby arkuszy A4 w podglÄ…dzie wydruku.
- **WdroĹĽone testy regresyjne:**
  - `awards_engine_ingests_wwff_reference`
  - `awards_engine_normalizes_and_deduplicates_wwff_reference`
  - `awards_engine_ignores_empty_or_invalid_wwff_reference`
  - `awards_engine_rebuilds_wwff_and_rda_from_log`
  - `propagation_history_summary_renders_empty_state_safely`
  - `propagation_history_summary_computes_accuracy_and_band_breakdown`
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” `AwardsEngine` rejestruje i przebudowuje `WWFF`/`RDA`, a `solar_panel.rs` prezentuje skutecznoĹ›Ä‡ prognoz propagacji.

---

### `FUNC-008` â€” `LotwClient::fetch_confirmations` nie przesyĹ‚a parametru `qso_qslsince` do serwera ARRL LoTW
- **ID:** `FUNC-008`
- **TytuĹ‚:** NieuĹĽywany parametr `_since_date` w `LotwClient::fetch_confirmations` powoduje pobieranie peĹ‚nej historii LoTW przy kaĹĽdej synchronizacji
- **Kategoria:** `FUNC`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commit:**
  - `53d532a`: `fix(awards): support wwff references, display propagation accuracy and fix lotw since date (FUNC-007, FUNC-008)`
- **ModuĹ‚ / obszar:** `src/cloud/lotw.rs`
- **Lokalizacja:** [`src/cloud/lotw.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/lotw.rs)
- **DowĂłd z kodu przed naprawÄ…:** Sygnatura `pub async fn fetch_confirmations(&self, _since_date: Option<&str>)` miaĹ‚a podkreĹ›lnik przed `_since_date` i budowaĹ‚a URL bez `&qso_qslsince=...`.
- **WdroĹĽone rozwiÄ…zanie (Etap 10):**
  1. Wydzielono funkcjÄ™ `build_confirmations_url(&self, since_date: Option<&str>) -> CloudResult<String>` oraz walidator `normalize_since_date(raw: &str) -> CloudResult<String>` (akceptujÄ…cy format `YYYY-MM-DD` lub `YYYYMMDD` normalizowany do `YYYY-MM-DD`, ze sprawdzaniem poprawnoĹ›ci miesiÄ…ca i dnia).
  2. Gdy `since_date` jest `Some(date)`, do zapytania `lotwreport.adi` doĹ‚Ä…czany jest zwalidowany parametr `&qso_qslsince=YYYY-MM-DD`; gdy `None`, parametr jest pomijany.
  3. Zachowano peĹ‚nÄ… redakcjÄ™ haseĹ‚ w diagnostyce URL (`redacted_url_for_diagnostics`).
- **WdroĹĽone testy regresyjne:**
  - `lotw_report_includes_qso_qslsince_parameter`
  - `lotw_report_omits_qso_qslsince_when_not_set`
  - `lotw_report_rejects_invalid_since_date`
  - `lotw_report_redacts_credentials_in_diagnostics`
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” URL generowany przez `fetch_confirmations(Some("2026-01-01"))` zawiera `qso_qslsince=2026-01-01` i odrzuca bĹ‚Ä™dne daty.


---

## 4. Integracje Radiowe, CAT, ProtokoĹ‚y Natywne i HIL (`CAT-*`)

### Ujednolicone mapowanie znalezisk i commitĂłw Etapu 3 (Bramka Dokumentacyjna)

Status Etapu 3: **SOFTWARE VERIFIED**
Fizyczny HIL: **NOT EXECUTED â€” SAFETY GATE CLOSED** (10 fizycznych testĂłw sprzÄ™towych pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`).

**Semantyka zwolnienia PTT (Drop vs jawne zamkniÄ™cie):**
- **Drop (destruktor):** Inicjuje synchroniczne best-effort wysĹ‚anie komendy TX OFF bez blokowania wÄ…tku i bez asynchronicznego oczekiwania na odpowiedĹş radia. Zapewnia ochronÄ™ przed porzuceniem wskaĹşnika, lecz nie gwarantuje asynchronicznego potwierdzenia powrotu radia do RX.
- **Jawne zamkniÄ™cie sesji (`release_and_confirm_rx` / `disconnect`):** WysyĹ‚a komendÄ™ TX OFF, drenuje bufor odbiorczy i asynchronicznie oczekuje na sprzÄ™towe potwierdzenie stanu RX (`confirmed_ptt == false` / `poll().ptt == false`).

| Finding ID | Pierwotny opis z rejestru | Zakres rzeczywisty remediacji | Commit w historii | Zmienione pliki | Zweryfikowane testy | Status koĹ„cowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `CAT-002` | WyĹ›cig potwierdzenia PTT w `TciBackend` i `FlexRadioBackend` powodujÄ…cy faĹ‚szywÄ… blokadÄ™ `TxArbiterState::Unknown` | Oczekiwanie na asynchroniczne potwierdzenie ramki `TciMessage::Trx` (TCI) oraz `interlock state=TRANSMITTING`/nie-TRANSMITTING (FlexRadio) z drainem, timeoutem i odrzucaniem pĂłĹşnych pakietĂłw | `31ab74e` *(tagowany w commit msg jako CAT-003)*, `ce73b8c` *(tagowany w commit msg jako CAT-005)* | `src/cat/tci.rs`, `src/cat/native/flexradio/backend.rs` | 29 testĂłw TCI w `src/cat/tci.rs`, 234 testy w `src/cat/native/flexradio/backend.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-003` | Brak nadpisania metody traitu `RigBackend::probe` w 5 z 8 backendĂłw (mutujÄ…cy probe Ĺ‚amie R10 / HIL-COND-27) | UsuniÄ™cie domyĹ›lnej implementacji `probe()` z `RigBackend` i wdroĹĽenie jawnych, Ĺ›ciĹ›le read-only implementacji dla wszystkich 8 backendĂłw | `3e907aa` *(tagowany w commit msg jako CAT-002)* | `src/cat/backend.rs`, `src/cat/probe.rs`, backendy natywne | 12 testĂłw kontraktowych probe w `tests/cat_native_6b.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-005` | Optymistyczne ustawianie `confirmed_ptt = true` bez potwierdzenia z radia w `YaesuBackend` i `ElecraftBackend` | Wymuszenie odpytania `TX;` (Yaesu) i `TQ;` (Elecraft) z usuniÄ™ciem optymistycznego przypisania stanu TX | `1fce33a` *(tagowany w commit msg jako CAT-006)* | `src/cat/native/yaesu/backend.rs`, `src/cat/native/elecraft/backend.rs` | 46 testĂłw Yaesu, 69 testĂłw Elecraft, test w `tests/cat_native_6a.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-006` | Luka w `RigHub::resync_tx_arbiter` (`last_update` zamiast `last_state_update`) oraz martwy wariant `StateFreshness::Stale` | ZastÄ…pienie `snapshot.last_update` przez `snapshot.last_state_update`, walidacja `freshness != Stale`, oznaczanie `Stale` przy bĹ‚Ä™dach/rozĹ‚Ä…czeniu w `RigService` | `1fce33a` | `src/cat/hub.rs`, `src/cat/service.rs` | 5 testĂłw deterministycznych w `tests/cat_native_6a.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-015` | `HilHardwareSession::execute_controlled_ptt` omija bramki `SPLOG_HIL_*`, omija `TxArbiter` i nie potwierdza RX | Integracja ze straĹĽnikiem `acquire_tx_guard`, weryfikacja autoryzacji Ĺ›rodowiska, zakaz anteny, wymĂłg poczÄ…tkowego RX i potwierdzenie `release_and_confirm_rx` | `ddc1c0c` | `src/cat/hil/session.rs` | 14 testĂłw cyklu ĹĽycia HIL w `tests/cat_native_7.rs` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |


### Ujednolicone mapowanie znalezisk i commitĂłw Etapu 8 (Integracja Multi-Radio w GUI, REST API Rig i Hamlib Proxy)

Status Etapu 8: **COMPLETED_SOFTWARE_VERIFIED**
Data ukoĹ„czenia: 2026-11-05
Fizyczny HIL: **NOT EXECUTED â€” SAFETY GATE CLOSED** (10 testĂłw laboratoryjnych HIL bezpiecznie odizolowanych za `#[ignore = "hardware-in-the-loop"]`).

| Finding ID | TytuĹ‚ kanoniczny | Remediacja gĹ‚Ăłwna | Commity | Zmienione pliki | Testy weryfikacyjne | Status koĹ„cowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `CAT-014` | OdciÄ™cie `NativeRigCommand` i `NativeRigEvent` oraz martwy kod legacy | Most dla poleceĹ„ natywnych w `RigBackend` (`execute_native`, `drain_native_events`), usuniÄ™cie martwego `CatController` | `8002667` | `src/cat/backend.rs`, `src/cat/service.rs`, `src/cat/native/icom/backend.rs`, `src/cat/native/flexradio/backend.rs` | `cat_backend_native_bridge_executes_and_drains_events_without_legacy_controller` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-008` | OminiÄ™cie potwierdzenia `TxArbiter` w Hamlib Proxy i Voice Keyerze oraz korekta semantyki Hamlib | Asynchroniczne potwierdzenie `request_tx_confirmed` i `release_tx_confirmed` w `CatRuntime`, kanoniczna semantyka bĹ‚Ä™dĂłw Hamlib `rig.h` (`-11 = RIG_ENAVAIL`, `-14 = RIG_BUSBUSY`), centralny mapper `map_cat_error_to_hamlib`, brak faĹ‚szywego `RPRT 0` przy odrzuceniu PTT | `fd43ed2`, `3cff368` | `src/cat/runtime.rs`, `src/cat/server.rs`, `src/gui/voice_keyer.rs`, `src/gui/app_plugins.rs`, `src/gui/app_cat.rs` | `hamlib_proxy_and_voice_keyer_abort_when_tx_arbiter_rejects_ptt`, `hamlib_minus_11_means_function_not_available`, `hamlib_minus_14_means_bus_busy` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-007` | Niekompletna integracja Multi-Radio w GUI i staĹ‚e mapowanie portĂłw Hamlib | UsuniÄ™cie fallbacku na pierwsze radio w GUI, unikalne parzyste porty Hamlib per RigId (`ProxyPortManager`: 4534, 4536...), brak kolizji z REST (8420), niezmiennoĹ›Ä‡ sesji TCP przy zmianie aktywnego radia w GUI | `fe7be53`, `c11d0b1` | `src/gui/app_async.rs`, `src/gui/app_cat.rs`, `src/gui/cat_settings.rs`, `src/gui/vfo_panel.rs`, `src/cat/server.rs` | `hamlib_proxy_port_is_unique_per_rig`, `hamlib_existing_session_remains_bound_to_original_rig`, `hamlib_proxy_port_collision_with_rest_api_is_rejected` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-003` | Utwardzenie bezpieczeĹ„stwa sterowania REST API i eliminacja atrap | PodĹ‚Ä…czenie `CatRuntime`, domyĹ›lny bind wyĹ‚Ä…cznie loopback `127.0.0.1:8420`, zakaz `0.0.0.0`/`::`, walidacja syntaktyczna RigId (400), walidacja f/mode, `deny_unknown_fields`, brak fallbacku na active rig (404 na nieznanym), routing PTT przez TxArbiter (409) | `36d1dee`, `833f28d` | `src/api/server.rs`, `src/gui/app_services.rs` | `rest_api_binds_loopback_by_default`, `rest_server_refuses_public_bind_without_authentication`, `rest_unknown_rig_returns_404_without_fallback`, `rest_invalid_rig_id_returns_400` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-EP-OWN` | WyĹ‚Ä…czna wĹ‚asnoĹ›Ä‡ fizycznego portu sprzÄ™towego UART/COM | Centralny `PhysicalEndpointRegistry`, normalizacja portĂłw Windows (`COM3`, `\\.\COM3`), wykluczenie kolizji Native CAT z zewnÄ™trznym `rigctld`, brak otwierania portĂłw UART przez Hamlib Proxy i Hamlib Client | `cc7f36e` | `src/cat/supervisor.rs`, `src/cat/mod.rs` | `same_serial_endpoint_cannot_have_two_owners`, `native_mode_does_not_start_external_rigctld_for_same_port`, `hamlib_client_mode_does_not_open_native_serial_port`, `hamlib_proxy_never_opens_physical_port_directly` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| Test Matrix | Macierz wspĂłĹ‚bieĹĽnoĹ›ci i izolacji wieloradiowej | RĂłwnolegĹ‚e rozkazy do wielu radii, odrzucanie przeterminowanych poleceĹ„, deterministyczny i ograniczony czasowo shutdown pod obciÄ…ĹĽeniem | `26096de` | `tests/cat_native_6b.rs` | `multi_radio_concurrency_simultaneous_commands_to_different_rigs`, `multi_radio_stale_command_expired_is_rejected`, `multi_radio_bounded_shutdown_under_heavy_concurrent_load` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |

### Ujednolicone mapowanie znalezisk i commitĂłw Etapu 4 (PoprawnoĹ›Ä‡ natywnych protokoĹ‚Ăłw CAT i bezpieczny auto-reconnect)

Status Etapu 4: **SOFTWARE VERIFIED**
Fizyczny HIL: **NOT EXECUTED â€” SAFETY GATE CLOSED** (10 fizycznych testĂłw sprzÄ™towych pozostaje odizolowanych za `#[ignore = "hardware-in-the-loop"]`).

| Finding ID | TytuĹ‚ kanoniczny (Rejestr) | Remediacja gĹ‚Ăłwna (Primary remediation) | Rozszerzenie utwardzajÄ…ce (Additional hardening) | Commity | Zmienione pliki | Testy weryfikacyjne | Status | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|---|
| `CAT-001` | CaĹ‚kowity brak automatycznego reconnect w `RigService::run_loop` | PÄ™tla automatycznego reconnectu z exponential backoff i jitterem w `RigService` | Kodowanie BCD Little-Endian (czÄ™stotliwoĹ›Ä‡) i Big-Endian (poziomy) dla Icom CI-V | `95877ce`, `4c03d18` | `src/cat/native/icom/{codec.rs,backend.rs}`, `src/cat/service.rs` | 44 testy w `cat::native::icom`, 7 testĂłw w `service::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-004` | BĹ‚Ä™dne parsowanie szerokoĹ›ci filtru i brak walidacji ramek CI-V | Walidacja granic ramek CI-V (min 6 B), odrzucanie echa, NAK, broadcastu, obcego adresu | ObsĹ‚uga komend split `0x25` i RIT/XIT `0x21`, brak mutacji stanu przy bĹ‚Ä™dzie | `cf2a503` | `src/cat/native/icom/{codec.rs,backend.rs,mod.rs}` | 7 testĂłw framingowych CI-V, 44 testy w `icom` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-009` | Brak odpytywania o czÄ™stotliwoĹ›Ä‡ TX przy pracy Split w `HamlibBackend` | Odpytywanie komendÄ… `i` w `HamlibBackend::poll_internal` przy wĹ‚Ä…czonym split i aktualizacja `tx_frequency_hz` | RozrĂłĹĽnienie odczytu `i` oraz `v` przy weryfikacji VFO | `30b6542` | `src/cat/hamlib.rs` | 15 testĂłw w `cat::hamlib` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-010` | Brak obsĹ‚ugi komend Split, VFO i Power w `FlrigBackend` oraz zbÄ™dne rozĹ‚Ä…czanie TCP | PeĹ‚na obsĹ‚uga `SetSplitFrequency`, `SetSplit`, `SetVfo`, `SetPower` oraz trwaĹ‚y strumieĹ„ TCP | Rygorystyczny framing odpowiedzi HTTP/1.1 keep-alive (`MAX_FLRIG_HTTP_HEADER_BYTES = 8192`, `MAX_FLRIG_XMLRPC_BODY_BYTES = 65536`, trwaĹ‚y bufor odbiorczy, Content-Length) | `30b6542`, `af80cbb` | `src/cat/flrig.rs` | 34 testy w `cat::flrig::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-011` | BĹ‚Ä™dne parsowanie/formatowanie RIT/XIT oraz filtrĂłw w backendach natywnych | PoprawnoĹ›Ä‡ enkoderĂłw i parserĂłw RIT/XIT oraz passband w backendach natywnych | Ujednolicenie i weryfikacja capabilities w modelach Yaesu i Elecraft | `30b6542` | `src/cat/native/kenwood/codec.rs`, `src/cat/native/yaesu/models.rs`, `src/cat/native/elecraft/models.rs` | 41 testĂłw Kenwood, 46 testĂłw Yaesu, 35 testĂłw Elecraft | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-012` | Pozorne wykonanie komend `SetPower`, `SetRit`, `SetXit` w `KenwoodBackend` i brak `PC` | Fallback getter query po 150 ms przy braku echa AI, odczyt mocy `PC` i obsĹ‚uga `SetPower`/`SetRit`/`SetXit` | Odczyt mocy w `poll_internal` dla Kenwood | `30b6542` | `src/cat/native/kenwood/{codec.rs,backend.rs}` | 41 testĂłw w `cat::native::kenwood` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-013` | Brak oczekiwania na `R<seq>|` dla setterĂłw oraz pusta implementacja `SetSplit` w `FlexRadioBackend` | Weryfikacja kodu odpowiedzi `R<seq>|0|` przez `send_and_await_ack` oraz `slice set <tx_slice_idx> tx=1` w `SetSplit` | PeĹ‚na obsĹ‚uga przeĹ‚Ä…czania slice A/B przy `SetSplit` i weryfikacja bĹ‚Ä™dĂłw SmartSDR | `30b6542` | `src/cat/native/flexradio/backend.rs` | `flexradio_set_split_sends_slice_tx_command_and_awaits_r_seq_ack`, 236 testĂłw w `flexradio` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-016` | Zahardkodowany adres CI-V `0x94` w manifestach HIL uniemoĹĽliwiajÄ…cy testy z innymi modelami | Dynamiczny lookup adresu CI-V z profilu modelu transceivera w `HilTestBenchManifest::to_rig_config` | Ujednolicenie `ReconnectPolicy` z wykĹ‚adniczym backoffem i jitterem w `RigService` | `30b6542`, `4c03d18` | `src/cat/hil/manifest.rs`, `src/cat/service.rs` | 1 test w `hil::manifest`, 4 testy w `service::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `CAT-017` | BlokujÄ…ce `std::thread::sleep` w `RigctldSupervisor` oraz niespĂłjnoĹ›ci profili | Redukcja sleepu w `RigctldSupervisor`, korekta profilu Yaesu oraz odrzucanie PTT ON w stanie `Degraded` | 1) Zachowanie zewnÄ™trznego TX po reconnect bez maskowania przez `state.ptt = false`, rozdzielenie od `TxOwned`, obsĹ‚uga `PttState::Unknown`, blokowanie `SetPtt(true)` przy External TX.<br>2) Izolacja generacji sesji (`SessionOperationResult`, sprawdzanie `session_generation` przed mutacjÄ… stanu, odrzucanie starych connect/disconnect/poll/command/PTT/error/timeout, powiÄ…zanie timera reconnectu z generacjÄ…, shutdown cancellation).<br>3) Rygorystyczny framing odpowiedzi HTTP/1.1 keep-alive FLRig (`MAX_FLRIG_HTTP_HEADER_BYTES`, `MAX_FLRIG_XMLRPC_BODY_BYTES`, trwaĹ‚y bufor odbiorczy, Content-Length, peer close, clean reset on reconnect). | `30b6542`, `a215cf6`, `9c8b877`, `1c4b36a`, `af80cbb` | `src/cat/supervisor.rs`, `src/cat/native/yaesu/models.rs`, `src/cat/service.rs`, `src/cat/hub.rs`, `src/cat/tx_arbiter.rs`, `src/cat/flrig.rs` | 10 testĂłw External TX, 19 testĂłw generacji i wyĹ›cigĂłw w `service::tests`, 22 testy framingu w `flrig::tests` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
### `CAT-001` â€” CaĹ‚kowity brak automatycznego wznawiania poĹ‚Ä…czenia (`reconnect`) w `RigService::run_loop` po zerwaniu sesji lub bĹ‚Ä™dzie startu
- **ID:** `CAT-001`
- **TytuĹ‚:** Brak pÄ™tli automatycznego `reconnect` w `RigService::run_loop` w stanie `Degraded` oraz po bĹ‚Ä™dzie poczÄ…tkowego `connect()`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/service.rs`, `src/cat/hamlib.rs`, `src/cat/tci.rs`
- **Lokalizacja:** [`src/cat/service.rs:389-399`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L389-L399), [`src/cat/service.rs:431-455`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L431-L455), [`src/cat/hamlib.rs:514`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hamlib.rs#L514), [`src/cat/tci.rs:875`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/tci.rs#L875)
- **DowĂłd z kodu:**
  1. JeĹ›li poczÄ…tkowe `self.backend.connect().await` w `RigService::run_loop` zwrĂłci bĹ‚Ä…d (`service.rs:389-399`), aktor ustawia `RigServiceState::Failed` i koĹ„czy zadanie Tokio (`return Err(e)`).
  2. JeĹ›li poĹ‚Ä…czenie zostanie zerwane w trakcie pracy (`Ready`), bĹ‚Ä…d `self.backend.poll().await` przeĹ‚Ä…cza sesjÄ™ w `RigServiceState::Degraded` (`service.rs:440-452`). W stanie `Degraded` pÄ™tla `tokio::select!` nadal wywoĹ‚uje wyĹ‚Ä…cznie `self.backend.poll().await` co `poll_interval`, **nigdy nie wywoĹ‚ujÄ…c ponownie `self.backend.connect().await`**. PoniewaĹĽ backendy po bĹ‚Ä™dzie I/O czyszczÄ… swĂłj strumieĹ„ (`self.stream = None` w `hamlib.rs:514`, `self.ws = None` w `tci.rs:875`), kaĹĽde kolejne `poll()` zwraca `Err(RigError::NotConnected)` w nieskoĹ„czonoĹ›Ä‡.
- **Opis problemu:** Chwilowy zanik zasilania radia, zakĹ‚Ăłcenie RF na kablu USB-COM lub restart `rigctld`/SmartSDR trwale paraliĹĽuje sesjÄ™ CAT aĹĽ do rÄ™cznego restartu w ustawieniach.
- **Scenariusz wystÄ…pienia:** Chwilowy reset konwertera USB-UART podczas nadawania na KF lub uruchomienie SPLogbook przed wĹ‚Ä…czeniem zasilacza transceivera.
- **WpĹ‚yw:** TrwaĹ‚a utrata komunikacji CAT bez automatycznego odzyskania poĹ‚Ä…czenia.
- **RozwiÄ…zanie minimalne:** W `RigService::run_loop` w stanie `Degraded` wywoĹ‚ywaÄ‡ `self.backend.disconnect().await` + `self.backend.connect().await` z wykĹ‚adniczym backoffem (`500 ms -> 1 s -> 2 s -> 5 s`) przed wznowieniem `poll()`.
- **RozwiÄ…zanie docelowe:** Maszyna stanĂłw reconnectu z licznikiem prĂłb i zachowaniem pÄ™tli aktora przy nieudanym poczÄ…tkowym `connect()`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `CAT-006`.
- **Wymagany test regresyjny:** `rig_service_reconnects_automatically_after_stream_drop_in_degraded_state`.
- **Kryterium akceptacji:** Po zasymulowanym bĹ‚Ä™dzie I/O w `poll()` serwis wywoĹ‚uje `connect()` i po naprawie transportu wraca do `RigServiceState::Ready`.

---

### `CAT-002` â€” WyĹ›cig potwierdzenia PTT w `TciBackend` i `FlexRadioBackend` powodujÄ…cy faĹ‚szywÄ… blokadÄ™ `TxArbiterState::Unknown`
- **ID:** `CAT-002`
- **TytuĹ‚:** Przedwczesny zwrot `Ok(())` z `SetPtt(true)` w `TciBackend` i `FlexRadioBackend` przed nadejĹ›ciem asynchronicznej ramki potwierdzenia TX
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commity:**
  - `31ab74e`: `fix(cat): require confirmed PTT ownership for TCI (CAT-003)`
  - `ce73b8c`: `fix(cat): require confirmed PTT ownership for FlexRadio (CAT-005)`
- **WdroĹĽone rozwiÄ…zanie (Etap 3):**
  1. W `TciBackend::execute_set_ptt` wprowadzono drain bufora wejĹ›ciowego przed wysĹ‚aniem komendy, pÄ™tlÄ™ asynchronicznego oczekiwania na ramkÄ™ `TciMessage::Trx` (do `command_timeout`), awaryjny timeout z wysĹ‚aniem `trx:0,false;`, filtr `discard_late_tx` zapobiegajÄ…cy nieautoryzowanemu wejĹ›ciu w TX po spĂłĹşnionym pakiecie oraz best-effort powrĂłt do RX w `Drop`.
  2. W `FlexRadioBackend::execute_set_ptt_true` / `execute_set_ptt_false` wprowadzono drain bufora, Ĺ›ledzenie `interlock_received_after_xmit`, wymĂłg nadejĹ›cia zarĂłwno odpowiedzi `R<seq>|0|` jak i dopasowanego stanu `interlock state=TRANSMITTING` (dla PTT ON) lub stanu nie-TRANSMITTING (dla PTT OFF), natychmiastowe awaryjne `xmit 0` w razie timeoutu lub odrzucenia, `discard_late_tx` oraz best-effort `Drop`.
- **Zweryfikowane testy regresyjne:**
  - 10 testĂłw symulacyjnych TCI w `src/cat/tci.rs` (wszystkie 29 testĂłw TCI PASSED).
  - 11 testĂłw symulacyjnych FlexRadio w `src/cat/native/flexradio/backend.rs` (wszystkie 234 testy FlexRadio PASSED).
- **ModuĹ‚ / obszar:** `src/cat/tci.rs`, `src/cat/native/flexradio/backend.rs`, `src/cat/service.rs`, `src/cat/hub.rs`
- **Lokalizacja:** [`src/cat/tci.rs:537, 808-827, 908-913`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/tci.rs#L808-L913), [`src/cat/native/flexradio/backend.rs:415-428, 1028-1068`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L415-L1068), [`src/cat/service.rs:514-526`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L514-L526), [`src/cat/hub.rs:662-692`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hub.rs#L662-L692)
- **DowĂłd z kodu:**
  1. W `TciBackend::execute(SetPtt(true))` (`tci.rs:908-913`) wysyĹ‚ana jest ramka WebSocket `trx:0,true;` bez czekania na potwierdzenie. Zaraz potem `RigService::handle_command` (`service.rs:514`) wywoĹ‚uje `self.backend.poll().await`, ktĂłre w `tci.rs:808` czeka tylko `short_poll_window = 5 ms` (`tci.rs:537`). JeĹ›li serwer TCI odpowie po >5 ms, `poll()` zwraca `ptt: false`, a `RigHub::request_tx` (`hub.rs:662-692`) blokuje `TxArbiter` w stanie `Unknown { reason: UnconfirmedState }` i wysyĹ‚a awaryjne `SetPtt(false)`!
  2. W `FlexRadioBackend::execute(SetPtt(true))` (`flexradio/backend.rs:1028-1068`) metoda czeka na `R<seq>|0|`, ale `recompute_ptt_state()` (`flexradio/backend.rs:415-428`) ustawia `confirmed_ptt_state = Tx` dopiero po nadejĹ›ciu osobnej linii statusu `S...|interlock state=TRANSMITTING`. JeĹ›li `interlock` nadejdzie w osobnym pakiecie TCP kilka milisekund po `R<seq>|0|`, `poll()` zwraca `ptt: false` i blokuje `TxArbiter` w `Unknown`.
- **Opis problemu:** ZaĹ‚Ä…czenie PTT na Ĺ‚Ä…czach WebSocket (TCI) lub TCP (FlexRadio SmartSDR) o opĂłĹşnieniu powyĹĽej kilku milisekund koĹ„czy siÄ™ natychmiastowym zrzuceniem nadawania i blokadÄ… bezpieczeĹ„stwa.
- **Scenariusz wystÄ…pienia:** NaciĹ›niÄ™cie przycisku TX (lub `F7`) przy pracy z ExpertSDR/Thetis (TCI) lub FlexRadio 6400/6600 przez LAN.
- **WpĹ‚yw:** NiemoĹĽnoĹ›Ä‡ stabilnego zaĹ‚Ä…czenia nadawania przez TCI i FlexRadio.
- **RozwiÄ…zanie minimalne:** W `TciBackend::execute(SetPtt(target))` oraz `FlexRadioBackend::execute(SetPtt(target))` oczekiwaÄ‡ w pÄ™tli (do `command_timeout`) na nadejĹ›cie ramki potwierdzajÄ…cej docelowy stan PTT (`trx:0,<target>;` w TCI oraz `interlock state=TRANSMITTING`/`READY` w FlexRadio).
- **RozwiÄ…zanie docelowe:** Jednoznaczne oczekiwanie na potwierdzony stan `ptt == target` wewnÄ…trz `execute(SetPtt)` we wszystkich backendach sieciowych.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `tci_and_flexradio_set_ptt_await_async_status_confirmation_with_25ms_delay`.
- **Kryterium akceptacji:** `request_tx` przechodzi do `TxArbiterState::Owned` nawet wtedy, gdy ramka `trx:0,true;` / `interlock state=TRANSMITTING` dociera 25 ms po wysĹ‚aniu komendy.

---

### `CAT-003` â€” Brak nadpisania metody traitu `RigBackend::probe` w 5 z 8 backendĂłw (`IcomCiV`, `Yaesu`, `Elecraft`, `FlexRadio`, `TCI`) Ĺ‚amie zasadÄ™ Read-Only Probe (`R10` / `HIL-COND-27`)
- **ID:** `CAT-003`
- **TytuĹ‚:** Metody `probe()` w 5 backendach sÄ… metodami inherentnymi zamiast implementacji `RigBackend::probe`, przez co polimorficzny Probe wywoĹ‚uje mutujÄ…ce `connect()`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `3e907aa`: `fix(cat): make backend probing strictly read-only (CAT-002)`
- **WdroĹĽone rozwiÄ…zanie (Etap 3):**
  1. UsuniÄ™to domyĹ›lnÄ… mutujÄ…cÄ… implementacjÄ™ `RigBackend::probe` z `src/cat/backend.rs`, ktĂłra wywoĹ‚ywaĹ‚a `connect()`. Kompilator wymusza teraz jawnÄ… implementacjÄ™ `probe()` na kaĹĽdym backendzie.
  2. Zaimplementowano bezpieczny, Ĺ›ciĹ›le niemutujÄ…cy `probe()` w `impl RigBackend` dla wszystkich 8 backendĂłw (`FlrigBackend`, `HamlibBackend`, `TciBackend`, `ElecraftBackend`, `FlexRadioBackend`, `IcomCiVBackend`, `KenwoodBackend`, `YaesuBackend`).
  3. Ĺ»aden backend podczas sondowania nie inicjalizuje peĹ‚nej sesji ze zmianÄ… konfiguracji radia ani nie wysyĹ‚a komend mutujÄ…cych (`AI1;`, `AI2;`, subskrypcji slices/panadapterĂłw, ani PTT).
- **Zweryfikowane testy regresyjne:**
  - 12 kontraktowych testĂłw read-only probe w `tests/cat_native_6b.rs` (wszystkie 87 testĂłw w pliku PASSED).
- **ModuĹ‚ / obszar:** `src/cat/backend.rs`, `src/cat/probe.rs`, `src/cat/native/{icom,yaesu,elecraft,flexradio}/backend.rs`, `src/cat/tci.rs`
- **Lokalizacja:** [`src/cat/backend.rs:61-78`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/backend.rs#L61-L78), [`src/cat/probe.rs:40`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/probe.rs#L40), [`src/cat/native/icom/backend.rs:114`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/backend.rs#L114), [`src/cat/native/yaesu/backend.rs:142`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/backend.rs#L142), [`src/cat/native/elecraft/backend.rs:456`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/elecraft/backend.rs#L456), [`src/cat/native/flexradio/backend.rs:302`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L302), [`src/cat/tci.rs:987`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/tci.rs#L987)
- **DowĂłd z kodu:** DomyĹ›lna metoda traitu `RigBackend::probe` (`src/cat/backend.rs:61-78`) wykonuje `self.connect().await?` -> `self.poll().await?` -> `self.disconnect().await`. Tylko `KenwoodBackend` (`kenwood/backend.rs:274`) i `FlrigBackend` (`flrig.rs:684`) nadpisujÄ… jÄ… w `impl RigBackend`. W `IcomCiVBackend`, `YaesuBackend`, `ElecraftBackend`, `FlexRadioBackend` i `TciBackend` metody `pub async fn probe(&mut self)` zdefiniowano poza traitem `RigBackend` z innÄ… sygnaturÄ…. W rezultacie `RigProbeRunner::probe` (`src/cat/probe.rs:40`) oraz `HilHardwareSession::execute_safe_probe` (`src/cat/hil/session.rs:584`), operujÄ…c na `Box<dyn RigBackend>`, wywoĹ‚ujÄ… domyĹ›lnÄ… implementacjÄ™ z `connect()`, ktĂłra wysyĹ‚a do radia komendy mutujÄ…ce (`AI1;` w Yaesu, `AI2;`/`K31;`/`K41;` w Elecraft, `sub slice all` w FlexRadio)!
- **Opis problemu:** BezpoĹ›rednie naruszenie wymagania Read-Only Probe (`Native CAT.md` Sekcje 12.4, 13.6, 14.1, R10 i `HIL-COND-27`).
- **Scenariusz wystÄ…pienia:** KlikniÄ™cie â€žTestuj poĹ‚Ä…czenie (Probe)â€ť w ustawieniach CAT lub uruchomienie `execute_safe_probe` w HIL.
- **WpĹ‚yw:** Modyfikacja stanu Auto-Information / subskrypcji w transceiverze podczas operacji, ktĂłra ma byÄ‡ gwarantowanie niemutujÄ…ca.
- **RozwiÄ…zanie minimalne:** PrzenieĹ›Ä‡ logikÄ™ `probe()` z metod inherentnych do `impl RigBackend for ...` w `IcomCiVBackend`, `YaesuBackend`, `ElecraftBackend`, `FlexRadioBackend` i `TciBackend`.
- **RozwiÄ…zanie docelowe:** UsunÄ…Ä‡ domyĹ›lnÄ… implementacjÄ™ `RigBackend::probe` z `src/cat/backend.rs:61-78`, wymuszajÄ…c przez kompilator jawne zaimplementowanie read-only `probe()` w kaĹĽdym typie implementujÄ…cym `RigBackend`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `polymorphic_box_dyn_rig_backend_probe_never_sends_mutating_connect_commands`.
- **Kryterium akceptacji:** WywoĹ‚anie `(Box::new(b) as Box<dyn RigBackend>).probe().await` dla wszystkich 8 backendĂłw nie wysyĹ‚a `AI1;`, `AI2;` ani `sub `.

---

### `CAT-004` â€” OdwrĂłcona kolejnoĹ›Ä‡ bajtĂłw BCD (Little-Endian zamiast Big-Endian) dla S-metra i mocy RF w `IcomCiVBackend` (`0x14` / `0x15`)
- **ID:** `CAT-004`
- **TytuĹ‚:** BĹ‚Ä™dne dekodowanie i kodowanie 2-bajtowego BCD w `src/cat/native/icom/codec.rs` i `backend.rs`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/native/icom/codec.rs`, `src/cat/native/icom/backend.rs`
- **Lokalizacja:** [`src/cat/native/icom/codec.rs:344-391`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/codec.rs#L344-L391), [`src/cat/native/icom/backend.rs:599-605`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/backend.rs#L599-L605)
- **DowĂłd z kodu:** W protokole Icom CI-V wyĹ‚Ä…cznie czÄ™stotliwoĹ›Ä‡ (5 bajtĂłw) jest Little-Endian BCD, natomiast wszystkie 2-bajtowe poziomy i mierniki (`0x14` / `0x15` w zakresie `0000..0255`) sÄ… przesyĹ‚ane jako **Big-Endian BCD** (`[0x01, 0x20]` = `0120` = S9; `[0x02, 0x55]` = `0255` = 100%). Tymczasem:
  1. `decode_smeter` (`icom/codec.rs:351-354`) i `decode_power_watts` (`icom/codec.rs:380-383`) liczÄ… `(d1 >> 4) * 1000 + (d1 & 0x0F) * 100 + (d0 >> 4) * 10 + (d0 & 0x0F)` (Little-Endian!). Dla odpowiedzi radia `[0x01, 0x20]` (120 = S9) wyliczajÄ… `2001 -> .min(255) = 255` (`S9+60dB`)!
  2. `SetPower` (`icom/backend.rs:599-605`) wywoĹ‚uje `encode_frequency_bcd(raw_val, 2)`, wysyĹ‚ajÄ…c dla `128` bajty `[0x28, 0x01]` (`2801` â€” odrzucane przez radio ramkÄ… `NAK 0xFA`) zamiast `[0x01, 0x28]`!
- **Opis problemu:** S-meter i moc wyjĹ›ciowa na wszystkich transceiverach Icom pokazujÄ… bĹ‚Ä™dne (niemal zawsze maksymalne) wartoĹ›ci, a komenda ustawienia mocy `SetPower` jest odrzucana przez radio.
- **Scenariusz wystÄ…pienia:** Odczyt S-metra / mocy lub zmiana mocy RF na dowolnym radiu Icom (IC-7300, IC-7610, IC-705, IC-9700).
- **WpĹ‚yw:** CaĹ‚kowicie niepoprawna telemetria S-metra i mocy RF oraz niedziaĹ‚ajÄ…ce `SetPower` na Icom CI-V.
- **RozwiÄ…zanie minimalne:** W `decode_smeter` i `decode_power_watts` zamieniÄ‡ kolejnoĹ›Ä‡ bajtĂłw na `(d0 >> 4) * 1000 + (d0 & 0x0F) * 100 + (d1 >> 4) * 10 + (d1 & 0x0F)`, a dla `SetPower` uĹĽywaÄ‡ kodowania Big-Endian BCD `[((raw_val / 100) as u8), ((((raw_val % 100) / 10) << 4) | (raw_val % 10)) as u8]`.
- **RozwiÄ…zanie docelowe:** Dedykowane funkcje `encode_level_bcd_be(val: u16) -> [u8; 2]` i `decode_level_bcd_be(bytes: &[u8]) -> Option<u16>` z walidacjÄ… pĂłĹ‚bajtĂłw BCD `0..=9` i zakresu `0..=255`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie (wymaga aktualizacji testĂłw jednostkowych w `icom/codec.rs`, ktĂłre testowaĹ‚y bĹ‚Ä™dny porzÄ…dek LE).
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `icom_civ_level_and_meter_bcd_encoding_is_big_endian_0000_to_0255`.
- **Kryterium akceptacji:** Ramka `FE FE E0 94 15 02 01 20 FD` dekoduje siÄ™ jako wartoĹ›Ä‡ surowa `120` (`S9`), a `SetPower(50)` wysyĹ‚a `0x01 0x28`.

---

### `CAT-005` â€” Optymistyczne ustawianie `confirmed_ptt = true` bez potwierdzenia z radia w `YaesuBackend` i `ElecraftBackend`
- **ID:** `CAT-005`
- **TytuĹ‚:** OminiÄ™cie weryfikacji sprzÄ™towej PTT w `YaesuBackend::execute` i `ElecraftBackend::execute`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `1fce33a`: `fix(cat): enforce fail-safe PTT cleanup (CAT-006)`
- **WdroĹĽone rozwiÄ…zanie (Etap 3):**
  1. W `YaesuBackend::execute(SetPtt(true))` wysyĹ‚ana jest komenda `TX1;` i natychmiast odpytywany stan `TX;`, wymagajÄ…c `parse_ptt_resp(resp) == Some(true)`. Przy `SetPtt(false)` wysyĹ‚ane jest `TX0;` i odpytywane `TX;`, wymagajÄ…c `Some(false)`. UsuniÄ™to optymistyczne przypisania.
  2. W `ElecraftBackend::execute(SetPtt(true))` wysyĹ‚ane jest `TX;` i odpytywane `TQ;`, wymagajÄ…c `TransmitStatus(true)`. Przy `SetPtt(false)` wysyĹ‚ane jest `RX;` i odpytywane `TQ;`, wymagajÄ…c `TransmitStatus(false)`. UsuniÄ™to optymistyczne przypisania przed potwierdzeniem.
  3. Dodano metody `is_ptt_confirmed(&self) -> bool` oraz bezpieczny fallback w `disconnect()`.
- **Zweryfikowane testy regresyjne:**
  - 46 testĂłw Yaesu w `src/cat/native/yaesu/backend.rs` (PASSED).
  - 69 testĂłw Elecraft w `src/cat/native/elecraft/backend.rs` (PASSED).
  - Test `yaesu_and_elecraft_set_ptt_reject_unconfirmed_tx_transition` w `tests/cat_native_6a.rs` (PASSED).
- **ModuĹ‚ / obszar:** `src/cat/native/yaesu/backend.rs`, `src/cat/native/elecraft/backend.rs`
- **Lokalizacja:** [`src/cat/native/yaesu/backend.rs:448-455, 548-563`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/backend.rs#L448-L563), [`src/cat/native/elecraft/backend.rs:551-555, 670-683`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/elecraft/backend.rs#L551-L683)
- **DowĂłd z kodu:**
  1. W `YaesuBackend::execute(SetPtt(ptt))` (`yaesu/backend.rs:548-563`) po wysĹ‚aniu `TX1;`/`TX0;` kod wykonuje `let _ = self.drain_incoming().await;` i **optymistycznie ustawia `self.confirmed_ptt = *ptt; self.state.ptt = *ptt;`**. NastÄ™pnie `YaesuBackend::poll()` (`yaesu/backend.rs:448-455`), gdy `self.ai_active && self.state.frequency_hz > 0`, odpytuje tylko `SM0;` i zwraca wĹ‚asne optymistyczne `self.state.ptt` do `TxArbiter`!
  2. W `ElecraftBackend::execute(SetPtt(ptt))` (`elecraft/backend.rs:670-683`) kod rĂłwnieĹĽ optymistycznie ustawia `self.confirmed_ptt = *ptt; self.state.ptt = *ptt;`, a w `poll()` (`elecraft/backend.rs:551, 555`) ignoruje bĹ‚Ä…d zapytania `let _ = self.query_and_await_response("TQ;", "TQ").await;`.
- **Opis problemu:** JeĹĽeli transceiver Yaesu lub Elecraft odrzuci przejĹ›cie w TX (np. poza pasmem `?;` lub blokada TX) albo nie wyĹ‚Ä…czy nadawania przy `SetPtt(false)`, `TxArbiter` otrzyma faĹ‚szywe potwierdzenie zmiany stanu.
- **Scenariusz wystÄ…pienia:** PrĂłba zaĹ‚Ä…czenia lub zwolnienia PTT na Yaesu/Elecraft przy zablokowanym nadajniku lub zerwanym kablu RX.
- **WpĹ‚yw:** ZĹ‚amanie niezmiennika bezpieczeĹ„stwa `TxArbiter` (faĹ‚szywy stan `Owned` lub `Idle`).
- **RozwiÄ…zanie minimalne:** W `YaesuBackend::execute(SetPtt)` wykonywaÄ‡ `self.query_and_await_response("TX;", "TX").await?`, a w `ElecraftBackend::execute(SetPtt)` oraz `poll()` wymagaÄ‡ poprawnej odpowiedzi `self.query_and_await_response("TQ;", "TQ").await?` bez wczeĹ›niejszego przypisywania `self.confirmed_ptt = *ptt`.
- **RozwiÄ…zanie docelowe:** Aktualizacja `confirmed_ptt` wyĹ‚Ä…cznie w parserze ramek przychodzÄ…cych (`TX0;`/`TX1;`/`TX2;` oraz `TQ0;`/`TQ1;`).
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `yaesu_and_elecraft_set_ptt_reject_unconfirmed_tx_transition`.
- **Kryterium akceptacji:** Brak odpowiedzi `TX1;` / `TQ1;` z transportu powoduje zwrot `Err(RigError::Timeout)` i przejĹ›cie `TxArbiter` w `Unknown`.

---

### `CAT-006` â€” Luka bezpieczeĹ„stwa w `RigHub::resync_tx_arbiter` (uĹĽycie `last_update` zamiast `last_state_update`) oraz martwy wariant `StateFreshness::Stale`
- **ID:** `CAT-006`
- **TytuĹ‚:** Odblokowanie `TxArbiterState::Unknown` bez Ĺ›wieĹĽego odczytu radia oraz brak oznaczania `StateFreshness::Stale`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `1fce33a`: `fix(cat): enforce fail-safe PTT cleanup (CAT-006)`
- **WdroĹĽone rozwiÄ…zanie (Etap 3):**
  1. W `RigHub::resync_tx_arbiter` zastÄ…piono `snapshot.last_update` przez `snapshot.last_state_update`, uniemoĹĽliwiajÄ…c odblokowanie `Unknown` na podstawie zdarzeĹ„ cyklu ĹĽycia lub nieudanych komend.
  2. Wymuszono `snapshot.freshness != StateFreshness::Stale`.
  3. W `RigService` (`transition_to` i `run()`) oznaczany jest stan `StateFreshness::Stale` przy bĹ‚Ä™dach odczytu telemetrycznego, timeoutach oraz stanach Degraded, Disconnecting, Disconnected i Failed.
- **Zweryfikowane testy regresyjne:**
  - 5 testĂłw deterministycznych symulacji w `tests/cat_native_6a.rs` (`resync_tx_arbiter_rejects_stale_state_without_new_poll`, `concurrent_ptt_on_requests_allow_only_one_owner`, `disconnect_racing_with_confirmation_does_not_enter_tx_owned`, `stale_session_confirmation_cannot_claim_tx_ownership`, `yaesu_and_elecraft_set_ptt_reject_unconfirmed_tx_transition`). Wszystkie 165 testĂłw w pliku PASSED.
- **ModuĹ‚ / obszar:** `src/cat/hub.rs`, `src/cat/service.rs`
- **Lokalizacja:** [`src/cat/hub.rs:888-908`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hub.rs#L888-L908), [`src/cat/service.rs:100-109, 298, 342, 440-452, 556`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L100-L556)
- **DowĂłd z kodu:**
  1. W `RigHub::resync_tx_arbiter` (`src/cat/hub.rs:904`) przekazywane jest `snapshot.last_update` zamiast `snapshot.last_state_update`. PoniewaĹĽ `last_update` jest podbijane przy kaĹĽdej zmianie stanu cyklu ĹĽycia (`transition_to`, `service.rs:298`) oraz po kaĹĽdej komendzie (`service.rs:556` â€” nawet zakoĹ„czonej bĹ‚Ä™dem!), `resync_tx_arbiter` widzi znacznik czasu nowszy niĹĽ `entered_at` i ufa staremu buforowanemu `snapshot.state.ptt == false` sprzed awarii!
  2. Wariant `StateFreshness::Stale` (`src/cat/service.rs:108`) nie jest przypisywany w ĹĽadnym miejscu w kodzie.
- **Opis problemu:** Po wejĹ›ciu w stan awaryjny `TxArbiterState::Unknown` wywoĹ‚anie `resync_tx_arbiter` moĹĽe przywrĂłciÄ‡ `Idle` bez ani jednego udanego `poll()` z radia.
- **Scenariusz wystÄ…pienia:** Timeout komendy `SetPtt` przeĹ‚Ä…cza `TxArbiter` w `Unknown`, a nastÄ™pnie `RigService` przechodzi w `Degraded` (co podbija `last_update`), po czym wywoĹ‚ane zostaje `resync_tx_arbiter`.
- **WpĹ‚yw:** ZĹ‚amanie niezmiennika bezpieczeĹ„stwa R2 (`Native CAT.md` Sekcja 8).
- **RozwiÄ…zanie minimalne:** W `RigHub::resync_tx_arbiter` (`src/cat/hub.rs:904`) uĹĽyÄ‡ `snapshot.last_state_update.ok_or_else(|| RigError::SafetyInterlock("No state update".into()))?` oraz wymagaÄ‡ `snapshot.freshness == StateFreshness::Fresh`; w `RigService::run_loop` przy bĹ‚Ä™dzie `poll()` (`service.rs:440`) ustawiaÄ‡ `s.freshness = StateFreshness::Stale`.
- **RozwiÄ…zanie docelowe:** Walidacja `freshness == Fresh && service_state == Ready && last_state_update > entered_at` w `resync_tx_arbiter`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `resync_tx_arbiter_rejects_degraded_or_lifecycle_only_timestamp`.
- **Kryterium akceptacji:** `resync_tx_arbiter` zwraca `Err(RigError::SafetyInterlock)`, dopĂłki po wejĹ›ciu w `Unknown` nie nastÄ…pi nowy, zakoĹ„czony sukcesem `poll()` ustawiajÄ…cy `last_state_update > entered_at` i `freshness == Fresh`.

---

### `CAT-007` â€” Niekompletna integracja Multi-Radio w GUI (`poll_cat_outcomes` fallback na pierwsze radio, brak dodawania Radio 2 w `cat_settings.rs`, niedziaĹ‚ajÄ…cy Auto-start `rigctld`)
- **ID:** `CAT-007`
- **TytuĹ‚:** Niejawny fallback na pierwsze radio w `app_async.rs`, brak przycisku dodania `Radio 2` w `cat_settings.rs` i ignorowanie `HamlibConfig::auto_start_rigctld` w `start_cat_service`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-05
- **Commit:** `fe7be53` (`fix(gui): eliminate single-rig fallbacks and support radio 2 configuration (CAT-007)`)
- **ModuĹ‚ / obszar:** `src/gui/app_async.rs`, `src/gui/cat_settings.rs`, `src/gui/app_cat.rs`, `src/gui/vfo_panel.rs`
- **Lokalizacja:** [`src/gui/app_async.rs:215-220`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_async.rs#L215-L220), [`src/gui/cat_settings.rs:146-210, 256, 1238-1296`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/cat_settings.rs#L146-L256), [`src/gui/app_cat.rs:138-163`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_cat.rs#L138-L163), [`src/gui/vfo_panel.rs:282-322, 515-543, 565-570`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/vfo_panel.rs#L282-L570)
- **DowĂłd z kodu:**
  1. `poll_cat_outcomes` (`src/gui/app_async.rs:219`) wykonuje `.or_else(|| cat_snap.rigs.iter().next().map(|(_, s)| s))` â€” jawny fallback na pierwsze radio z mapy, zabroniony przez `R8` / `HIL-COND-18`.
  2. `render_rig_selector_bar` (`src/gui/cat_settings.rs:146-210`) nie posiada przycisku dodania drugiego profilu (`Radio 2`), a `sync_legacy_fields_from_rig` (`cat_settings.rs:256`) nadpisuje globalne pola legacy aktualnie wybranÄ… zakĹ‚adkÄ….
  3. `SpLogApp::start_cat_service` (`src/gui/app_cat.rs:138-163`) sprawdza wyĹ‚Ä…cznie `self.config.cat_auto_start_rigctld` (legacy), ktĂłre nigdy nie jest aktualizowane z `RigBackendConfig::Hamlib(cfg).auto_start_rigctld` edytowanego w `cat_settings.rs:429`.
  4. `vfo_panel.rs` uĹĽywa lokalnych zmiennych atrapowych dla Split VFO B (`+1k`/`+5k`), Preamp/ATT/NB/NR/AGC oraz wyĹ›wietla zahardkodowane `swr = 1.2` podczas TX (`vfo_panel.rs:568`).
- **Opis problemu:** UĹĽytkownik nie moĹĽe skonfigurowaÄ‡ `Radio 2` z poziomu GUI, nie moĹĽe uruchomiÄ‡ `rigctld` przez checkbox Auto-start w nowym panelu CAT, a panel VFO pokazuje faĹ‚szywe dane Split/SWR.
- **Scenariusz wystÄ…pienia:** PrĂłba dodania drugiego radia w ustawieniach CAT, wĹ‚Ä…czenia Auto-startu `rigctld` lub uĹĽycia Splitu w panelu VFO.
- **WpĹ‚yw:** NiepeĹ‚na uĹĽytecznoĹ›Ä‡ architektury wieloradiowej NATIVE-6A/6B w warstwie GUI.
- **RozwiÄ…zanie minimalne:**
  1. UsunÄ…Ä‡ fallback `.or_else(|| cat_snap.rigs.iter().next()...)` w `app_async.rs:219`.
  2. DodaÄ‡ przycisk `+ Radio 2` w `cat_settings.rs:146-210`.
  3. W `start_cat_service` (`app_cat.rs:138`) odczytywaÄ‡ `RigBackendConfig::Hamlib(cfg)` z `station_cfg.rigs`.
  4. W `vfo_panel.rs` podpiÄ…Ä‡ `+1k`/`+5k` pod `RigCommand::SetSplitFrequency` i wyĹ›wietlaÄ‡ `rig_state.tx_frequency_hz` oraz `rig_state.rf_power_watts`.
- **RozwiÄ…zanie docelowe:** PeĹ‚ny widok dwuradiowy SO2R w `vfo_panel.rs` zintegrowany z `So2rState` i `NativeRigCommand`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `start_cat_service_honors_station_rig_hamlib_auto_start_config` oraz `vfo_panel_split_buttons_send_set_split_frequency`.
- **Kryterium akceptacji:** UĹĽytkownik moĹĽe dodaÄ‡ `Radio 2` w GUI, wĹ‚Ä…czyÄ‡ auto-start `rigctld` oraz sterowaÄ‡ czÄ™stotliwoĹ›ciÄ… Split VFO B z panelu VFO.

---

### `CAT-008` â€” OminiÄ™cie potwierdzenia `TxArbiter` w `HamlibProxyServer` (`:4534`), `VoiceKeyer` oraz `PluginCommand::SendVoice`
- **ID:** `CAT-008`
- **TytuĹ‚:** Optymistyczna mutacja PTT w `HamlibProxyServer` oraz odtwarzanie audio bez potwierdzenia `TxArbiter` w `VoiceKeyer` i pluginach
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-05
- **Commity:** `fd43ed2` (`fix(cat): enforce TxArbiter PTT confirmation in proxy and voice keyer (CAT-008)`), `3cff368` (`fix(cat): correct Hamlib error mapping for TxArbiter failures (CAT-008)`)
- **ModuĹ‚ / obszar:** `src/cat/server.rs`, `src/gui/voice_keyer.rs`, `src/gui/app_plugins.rs`
- **Lokalizacja:** [`src/cat/server.rs:441-452`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/server.rs#L441-L452), [`src/gui/voice_keyer.rs:170-181`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/voice_keyer.rs#L170-L181), [`src/gui/app_plugins.rs:127-139`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_plugins.rs#L127-L139)
- **DowĂłd z kodu:**
  1. `HamlibProxyServer` (`src/cat/server.rs:441-452`) po odebraniu `T 1` natychmiast ustawia `state.ptt = true` w `shared_state` i odsyĹ‚a `RPRT 0\n`, zanim pÄ™tla GUI (`poll_cat_proxy_commands`) przekaĹĽe rozkaz do `CatRuntime::request_tx`.
  2. `VoiceKeyer::transmit_message` (`src/gui/voice_keyer.rs:170-181`) wywoĹ‚uje fire-and-forget `rt.request_tx(...)`, czeka `100 ms` i bezwarunkowo odtwarza plik WAV, nawet jeĹ›li `TxArbiter` odrzuciĹ‚ PTT!
  3. `PluginCommand::SendVoice` (`src/gui/app_plugins.rs:127-139`) odtwarza plik WAV bez zaĹ‚Ä…czenia PTT w ogĂłle.
- **Opis problemu:** Brak synchronizacji zwrotnej miÄ™dzy `TxArbiter` a klientem Hamlib Proxy i odtwarzaczem Voice Keyera.
- **Scenariusz wystÄ…pienia:** WSJT-X podĹ‚Ä…czony do portu `4534` lub uruchomienie makra Voice Keyera przy zablokowanym `TxArbiter`.
- **WpĹ‚yw:** WSJT-X otrzymuje faĹ‚szywe `RPRT 0` przy odrzuconym PTT; Voice Keyer odtwarza dĹşwiÄ™k mimo braku nadawania.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ w `CatRuntime` metodÄ™ `request_tx_confirmed` zwracajÄ…cÄ… `oneshot::Receiver<Result<(), RigError>>` i uĹĽywaÄ‡ jej w `HamlibProxyServer`, `VoiceKeyer` oraz `PluginCommand::SendVoice`.
- **RozwiÄ…zanie docelowe:** BezpoĹ›rednie poĹ‚Ä…czenie `HamlibProxyServer` z `CatRuntime` z pominiÄ™ciem poĹ›rednictwa wÄ…tku GUI.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `FUNC-003`.
- **Wymagany test regresyjny:** `hamlib_proxy_and_voice_keyer_abort_when_tx_arbiter_rejects_ptt`.
- **Kryterium akceptacji:** Odrzucenie PTT przez `TxArbiter` zwraca kanoniczny kod bĹ‚Ä™du Hamlib (`RPRT -11 = RIG_ENAVAIL` przy braku funkcji lub `RPRT -14 = RIG_BUSBUSY` przy zajÄ™toĹ›ci szyny nadawczej/konflikcie arbitra) w `HamlibProxyServer`, nigdy faĹ‚szywy sukces `RPRT 0`, oraz natychmiast zatrzymuje odtwarzanie audio w `VoiceKeyer` bez mutacji stanu logicznego PTT.

---

### `CAT-009` â€” NiepeĹ‚ny polling w `HamlibBackend::poll_internal` (brak odczytu VFO, Split TX freq i RIT/XIT)
- **ID:** `CAT-009`
- **TytuĹ‚:** `HamlibBackend::poll_internal` nie odpytuje `v` (VFO), `i` (Split TX freq), `j` (RIT) ani `z` (XIT)
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/hamlib.rs`
- **Lokalizacja:** [`src/cat/hamlib.rs:625-698`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hamlib.rs#L625-L698)
- **DowĂłd z kodu:** `poll_internal` wysyĹ‚a tylko `f`, `m`, `l STRENGTH`, `s`, `l RFPOWER`, `t`, nigdy nie aktualizujÄ…c `vfo`, `tx_frequency_hz`, `rit_hz` ani `xit_hz` ze stanu radia.
- **Opis problemu:** Zmiana VFO A/B lub czÄ™stotliwoĹ›ci Split TX pokrÄ™tĹ‚em na radiu podĹ‚Ä…czonym przez `rigctld` nie jest widoczna w SPLogbook.
- **Scenariusz wystÄ…pienia:** Praca DX w trybie Split przez `rigctld`.
- **WpĹ‚yw:** Nieaktualna czÄ™stotliwoĹ›Ä‡ nadawcza `tx_frequency_hz` i VFO w `RigState`.
- **RozwiÄ…zanie minimalne:** W `HamlibBackend::poll_internal`, gdy `split_enabled == true`, odpytywaÄ‡ `i` (`\get_split_freq`) i aktualizowaÄ‡ `tx_frequency_hz`.
- **RozwiÄ…zanie docelowe:** Odczyt `v` oraz warunkowy odczyt `i`/`j`/`z` co N-ty cykl pollingu.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `hamlib_backend_poll_reads_split_tx_frequency_when_split_enabled`.
- **Kryterium akceptacji:** `poll()` w `HamlibBackend` aktualizuje `tx_frequency_hz` na podstawie odpowiedzi na komendÄ™ `s` / `i`.

---

### `CAT-010` â€” Brak HTTP Keep-Alive w `FlrigClient::call` (20 nowych poĹ‚Ä…czeĹ„ TCP/s) i wyĹ‚Ä…czone moĹĽliwoĹ›ci Split/VFO/Power w `FlrigBackend`
- **ID:** `CAT-010`
- **TytuĹ‚:** Lawina gniazd `TIME_WAIT` (1200 poĹ‚Ä…czeĹ„ TCP/min) oraz zablokowane `SetSplit`/`SetVfo`/`SetPower` w `FlrigBackend`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/flrig.rs`
- **Lokalizacja:** [`src/cat/flrig.rs:345-416, 601-682`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/flrig.rs#L345-L682)
- **DowĂłd z kodu:**
  1. `FlrigClient::call` (`flrig.rs:345-416`) otwiera nowe poĹ‚Ä…czenie `TcpStream::connect` z `Connection: close` na kaĹĽde wywoĹ‚anie XML-RPC (4 wywoĹ‚ania co 200 ms = 20 gniazd/s).
  2. `FlrigBackend::capabilities()` (`flrig.rs:668-682`) ustawia `passband: false, split: false, split_frequency: false, vfo: false, power: false`.
- **Opis problemu:** Szybkie wyczerpywanie puli portĂłw efemerycznych na Windows oraz brak obsĹ‚ugi Splitu/VFO B przez FLRig.
- **Scenariusz wystÄ…pienia:** Kilkuminutowa praca z wĹ‚Ä…czonym backendem FLRig.
- **WpĹ‚yw:** BĹ‚Ä™dy `AddrInUse` / `TimedOut` po wyczerpaniu portĂłw efemerycznych oraz odrzucanie komend Split/VFO.
- **RozwiÄ…zanie minimalne:** UtrzymywaÄ‡ trwaĹ‚e poĹ‚Ä…czenie `Option<TcpStream>` z nagĹ‚Ăłwkiem `Connection: keep-alive` w `FlrigClient` oraz wĹ‚Ä…czyÄ‡ obsĹ‚ugÄ™ `SetSplit`, `SetSplitFrequency` (`set_vfo_b`) i `SetVfo` w `FlrigBackend`.
- **RozwiÄ…zanie docelowe:** TrwaĹ‚y strumieĹ„ HTTP/1.1 Keep-Alive z automatycznym wznowieniem przy EOF.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `flrig_backend_reuses_single_tcp_stream_across_multiple_xmlrpc_calls`.
- **Kryterium akceptacji:** Wielokrotne wywoĹ‚ania `poll()` w `FlrigBackend` wykorzystujÄ… to samo poĹ‚Ä…czenie TCP.

---

### `CAT-011` â€” BĹ‚Ä™dne subkomendy `SetRit`/`SetXit` (`0x21`) i `SetSplitFrequency` w `IcomCiVBackend`
- **ID:** `CAT-011`
- **TytuĹ‚:** `IcomCiVBackend` wysyĹ‚a niepoprawne ramki `0x21 0x00` dla wĹ‚Ä…czenia RIT/XIT, ignoruje offset `hz` i przestraja VFO RX przy `SetSplitFrequency`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/native/icom/backend.rs`
- **Lokalizacja:** [`src/cat/native/icom/backend.rs:493-506, 575-598`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/icom/backend.rs#L493-L598)
- **DowĂłd z kodu:**
  1. W `SetRit` (`icom/backend.rs:575-586`) i `SetXit` (`587-598`) wysyĹ‚any jest 1 bajt `[0x01]`/`[0x00]` do `0x21 0x00` (`SUB_CMD_RIT_FREQ`, ktĂłra wymaga 3 bajtĂłw offsetu BCD + znaku) zamiast `0x21 0x01` (RIT ON/OFF) i `0x21 0x02` (Î”TX ON/OFF), parametr `hz` nie jest w ogĂłle wysyĹ‚any, a bĹ‚Ä™dy sÄ… tĹ‚umione przez `let _ = self.send_and_expect_ack(frame).await;`.
  2. W `SetSplitFrequency(freq)` (`icom/backend.rs:493-506`) wysyĹ‚ana jest komenda `0x05` (`CMD_SET_FREQUENCY`), ktĂłra zmienia czÄ™stotliwoĹ›Ä‡ bieĹĽÄ…cego VFO odbiorczego zamiast VFO nadawczego (`0x25 0x01` lub VFO B).
- **Opis problemu:** Sterowanie RIT/XIT oraz czÄ™stotliwoĹ›ciÄ… nadawczÄ… Split na radiach Icom jest niesprawne.
- **Scenariusz wystÄ…pienia:** WĹ‚Ä…czenie RIT/XIT lub ustawienie czÄ™stotliwoĹ›ci Split TX na transceiverze Icom.
- **WpĹ‚yw:** Przestrojenie odbiornika zamiast nadajnika przy `SetSplitFrequency` oraz odrzucanie `SetRit`/`SetXit` przez radio.
- **RozwiÄ…zanie minimalne:** UĹĽyÄ‡ subkomend `0x21 0x01` (RIT ON/OFF), `0x21 0x02` (Î”TX ON/OFF) i `0x21 0x00` (3-bajtowy offset BCD) z propagacjÄ… bĹ‚Ä™du `?`, a w `SetSplitFrequency` uĹĽyÄ‡ `0x25 0x01` (lub przeĹ‚Ä…czenia VFO B).
- **RozwiÄ…zanie docelowe:** PeĹ‚na obsĹ‚uga komend `0x21` i `0x25` zgodnie z tabelÄ… CI-V Icom.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** `CAT-004`.
- **Wymagany test regresyjny:** `icom_set_rit_sends_0x21_0x01_and_0x21_0x00_offset_frames`.
- **Kryterium akceptacji:** `SetRit { enabled: true, hz: -150 }` wysyĹ‚a `0x21 0x01 0x01` oraz `0x21 0x00 0x50 0x01 0x01`, a `SetSplitFrequency` ustawia czÄ™stotliwoĹ›Ä‡ nieaktywnego VFO.

---

### `CAT-012` â€” Timeout 1500 ms na setterach bez echa oraz sprzecznoĹ›Ä‡ `RigCapabilities` z `execute()` w `KenwoodBackend`
- **ID:** `CAT-012`
- **TytuĹ‚:** `KenwoodBackend::execute` czeka na echo setterĂłw (timeout przy braku zmiany lub wyĹ‚Ä…czonym `AI2;`) i zwraca `Unsupported` dla zadeklarowanych moĹĽliwoĹ›ci
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/native/kenwood/backend.rs`, `src/cat/native/registry.rs`
- **Lokalizacja:** [`src/cat/native/kenwood/backend.rs:437-505`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/kenwood/backend.rs#L437-L505), [`src/cat/native/registry.rs:173-300`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/registry.rs#L173-L300)
- **DowĂłd z kodu:**
  1. `KenwoodBackend::execute` (`kenwood/backend.rs:437-499`) wywoĹ‚uje `roundtrip_command(&cmd_str, prefix)` dla setterĂłw (`FA...;`, `MD...;`, `TX;`, `RX;`). W protokole Kenwood settery nie odsyĹ‚ajÄ… odpowiedzi, jeĹ›li stan siÄ™ nie zmieniĹ‚ lub `AI2;` nie jest aktywne, co powoduje blokadÄ™ na `1500 ms` i `Err(RigError::Timeout)`.
  2. Profile Kenwood deklarujÄ… `passband: true, rit: true, xit: true, power: true`, ale `KenwoodBackend::execute` (`kenwood/backend.rs:502-505`) zwraca dla nich `Err(RigError::Unsupported)`.
- **Opis problemu:** Ryzyko faĹ‚szywych timeoutĂłw przy sterowaniu Kenwoodem (w tym wejĹ›cia `TxArbiter` w `Unknown` przy `SetPtt`) oraz niezgodnoĹ›Ä‡ moĹĽliwoĹ›ci z implementacjÄ….
- **Scenariusz wystÄ…pienia:** WysĹ‚anie `SetMode("USB")`, gdy radio Kenwood jest juĹĽ w trybie USB, lub wywoĹ‚anie `SetPower` / `SetRit`.
- **WpĹ‚yw:** Zawieszanie kolejki rozkazĂłw CAT na 1,5 s i bĹ‚Ä™dy `Timeout` / `Unsupported`.
- **RozwiÄ…zanie minimalne:** W `KenwoodBackend::execute` wysyĹ‚aÄ‡ setter wraz z jawnym zapytaniem weryfikujÄ…cym (np. `FA014074000;FA;`, `TX;IF;`, `RX;IF;`) oraz zaimplementowaÄ‡ `PCxxx;` (`SetPower`), `RT0;`/`RT1;` (`SetRit`) i `XT0;`/`XT1;` (`SetXit`).
- **RozwiÄ…zanie docelowe:** PeĹ‚na implementacja wszystkich komend zadeklarowanych w `RigCapabilities` dla Kenwooda.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `kenwood_execute_setter_ appends_verification_query_and_supports_power_rit_xit`.
- **Kryterium akceptacji:** Wszystkie komendy wĹ‚Ä…czone w `RigCapabilities` profilu Kenwood sÄ… obsĹ‚ugiwane w `execute()` bez polegania na spontanicznym echu `AI2;`.

---

### `CAT-013` â€” Brak oczekiwania na `R<seq>|` dla setterĂłw oraz pusta implementacja `SetSplit` w `FlexRadioBackend`
- **Finding ID:** `CAT-013`
- **Canonical title:** Brak oczekiwania na `R<seq>|` dla setterĂłw oraz pusta implementacja `SetSplit` w `FlexRadioBackend`
- **Severity:** `HIGH`
- **Confidence:** `CONFIRMED`
- **Original evidence:** [`src/cat/native/flexradio/backend.rs:914-1027, 1080-1126`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/flexradio/backend.rs#L914-L1126)
- **Root cause:** Settery dziaĹ‚aĹ‚y fire-and-forget (`send_command_tracked` bez `wait_for_response`), a `SetSplit` nie wykonywaĹ‚ rzeczywistego przeĹ‚Ä…czenia TX slice (pusty blok no-op ignorujÄ…cy bĹ‚Ä™dy SmartSDR).
- **Primary remediation:** Weryfikacja kodu odpowiedzi `R<seq>|0|` przez `send_and_await_ack` dla wszystkich setterĂłw oraz wysyĹ‚anie `slice set <tx_slice_idx> tx=1` w `SetSplit`.
- **Additional hardening:** PeĹ‚na obsĹ‚uga przeĹ‚Ä…czania slice A/B przy `SetSplit` oraz parsowanie i propagacja bĹ‚Ä™dĂłw komend SmartSDR.
- **Commits:** `30b6542`
- **Tests:** `flexradio_set_split_sends_slice_tx_command_and_awaits_r_seq_ack`, 236 testĂłw w `cat::native::flexradio`.
- **Status:** `FIXED_SOFTWARE_VERIFIED`
- **Residual risk:** NISKIE
---

### `CAT-014` â€” OdciÄ™cie `NativeRigCommand` i `NativeRigEvent` od `RigService` i `RigHub` oraz martwy kod legacy (`CatController`, `CatBackendKind::So2r`)
- **ID:** `CAT-014`
- **TytuĹ‚:** Brak mostu dla `NativeRigCommand`/`NativeRigEvent` w `RigService` oraz martwy `CatController` i `CatBackendKind::So2r` w `src/cat/backend.rs`
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-05
- **Commit:** `8002667` (`refactor(cat): add native command bridge and clean up legacy controller (CAT-014)`)
- **ModuĹ‚ / obszar:** `src/cat/native/mod.rs`, `src/cat/backend.rs`, `src/cat/so2r.rs`
- **Lokalizacja:** [`src/cat/native/mod.rs:48-159`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/mod.rs#L48-L159), [`src/cat/backend.rs:111, 236-413`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/backend.rs#L111-L413), [`src/cat/so2r.rs:1-209`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/so2r.rs#L1-L209)
- **DowĂłd z kodu:**
  1. `execute_native` i `drain_events` w `IcomCiVBackend` oraz `FlexRadioBackend` nie sÄ… czÄ™Ĺ›ciÄ… traitu `RigBackend` i nigdy nie sÄ… wywoĹ‚ywane przez `RigService`.
  2. Stary trait `CatBackend` i enum `CatController` (`src/cat/backend.rs:236-413`) oraz `So2rState` (`src/cat/so2r.rs`) nie sÄ… nigdzie uĹĽywane w kodzie produkcyjnym.
- **Opis problemu:** Rozszerzone funkcje natywne (Preamp, Attenuator, Antena, Dualwatch) sÄ… niedostÄ™pne z poziomu `RigHub` i GUI.
- **Scenariusz wystÄ…pienia:** PrĂłba sterowania Preamp/ATT/AntenÄ… z panelu VFO.
- **WpĹ‚yw:** Niewykorzystany potencjaĹ‚ zaimplementowanych kodekĂłw natywnych oraz szum architektoniczny z `CatController`.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ metodÄ™ `execute_native(&mut self, cmd: &NativeRigCommand)` do `RigBackend` (z domyĹ›lnym `Err(RigError::Unsupported)`) i usunÄ…Ä‡ martwy `CatController`.
- **RozwiÄ…zanie docelowe:** Integracja `NativeRigCommand` i `So2rState` z `RigHub` i `vfo_panel.rs`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** `CAT-007`.
- **Wymagany test regresyjny:** `rig_service_routes_native_rig_command_to_backend`.
- **Kryterium akceptacji:** `RigHub` przekazuje `NativeRigCommand` do natywnego backendu i odbiera `NativeRigEvent`.

---

### `CAT-015` â€” `HilHardwareSession::execute_controlled_ptt` omija bramki Ĺ›rodowiskowe `SPLOG_HIL_*`, omija `TxArbiter` i nie potwierdza powrotu do RX
- **ID:** `CAT-015`
- **TytuĹ‚:** Niezabezpieczona Ĺ›cieĹĽka PTT w `HilHardwareSession::execute_controlled_ptt` (`src/cat/hil/session.rs:630-684`)
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED / PHYSICAL_HIL_NOT_EXECUTED_SAFETY_GATE_CLOSED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commit:**
  - `ddc1c0c`: `fix(hil): protect hardware PTT session lifecycle (CAT-015)`
- **WdroĹĽone rozwiÄ…zanie (Etap 3):**
  1. Metoda `execute_controlled_ptt` w `HilHardwareSession` zostaĹ‚a caĹ‚kowicie zintegrowana ze straĹĽnikiem `acquire_tx_guard` i `release_and_confirm_rx`.
  2. Wprowadzono peĹ‚nÄ… weryfikacjÄ™ bramek bezpieczeĹ„stwa: zakaz PTT w fazie 7A, wymĂłg statusu HIL-READY Passed, bezwzglÄ™dny zakaz anteny (wymĂłg sztucznego obciÄ…ĹĽenia o odpowiedniej mocy), autoryzacja zmiennych `SPLOG_HIL_ENABLE=1`, `SPLOG_HIL_SESSION`, `SPLOG_HIL_ALLOW_TX` (`I_HAVE_DUMMY_LOAD` lub `1`), oraz odczyt `poll()` potwierdzajÄ…cy, ĹĽe radio nie nadaje przed zaĹ‚Ä…czeniem PTT (`initial_state.ptt == false`).
  3. Zabezpieczenie czasowe oparto na watchdogu `HilTxGuard` o limicie `max_test_duration_ms` (maks. 500 ms) z jawnym powrotem do RX (`backend.poll().ptt == false`).
  4. W razie jakiegokolwiek bĹ‚Ä™du transportu, timeoutu lub braku potwierdzenia RX nastÄ™puje awaryjne `SetPtt(false)`, `TxArbiter` przechodzi w `Unknown`, sesja w `FailedOrUnknown`, a `rx_restored_cleanly` nigdy nie jest zwracane jako `true`.
  5. Wszystkie 10 fizycznych punktĂłw wejĹ›cia Stage 7B pozostaje hermetycznie zablokowanych za bramkÄ… `#[ignore = "hardware-in-the-loop"]`.
- **Zweryfikowane testy regresyjne:**
  - 14 testĂłw symulacyjnych cyklu ĹĽycia HIL w `tests/cat_native_7.rs` (w tym `hil_execute_controlled_ptt_requires_env_gates_arbiter_and_confirmed_rx`). Wszystkie 297 aktywnych testĂłw HIL PASSED (10 fizycznych IGNORED).
- **ModuĹ‚ / obszar:** `src/cat/hil/session.rs`
- **Lokalizacja:** [`src/cat/hil/session.rs:630-684`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hil/session.rs#L630-L684)
- **DowĂłd z kodu:** W przeciwieĹ„stwie do `acquire_tx_guard` (`session.rs:451-535`), metoda `execute_controlled_ptt` (`session.rs:630-684`) nie wywoĹ‚uje `verify_hardware_access_authorization()`, nie korzysta z `TxArbiter`, nie weryfikuje poczÄ…tkowego stanu RX przed `SetPtt(true)`, a po `SetPtt(false)` nie wywoĹ‚uje `backend.poll().await`, lecz na sztywno ustawia `self.rx_restored_cleanly = true`.
- **Opis problemu:** ZĹ‚amanie wymogĂłw bezpieczeĹ„stwa `Native CAT.md` Sekcja 18.4 (`HIL-COND-16`, `HIL-COND-28`, `HIL-X20`).
- **Scenariusz wystÄ…pienia:** WywoĹ‚anie `execute_controlled_ptt` podczas sesji testowej HIL.
- **WpĹ‚yw:** MoĹĽliwoĹ›Ä‡ zaĹ‚Ä…czenia nadajnika bez peĹ‚nej weryfikacji bramek Ĺ›rodowiskowych i bez potwierdzenia powrotu do odbioru (`ConfirmedRx`).
- **RozwiÄ…zanie minimalne:** PrzepisaÄ‡ `execute_controlled_ptt` tak, aby wewnÄ™trznie wywoĹ‚ywaĹ‚o `self.acquire_tx_guard(backend, arbiter, consent).await?`, czekaĹ‚o zadany czas i wywoĹ‚ywaĹ‚o `guard.release_and_confirm_rx().await?`.
- **RozwiÄ…zanie docelowe:** Jedna wspĂłlna Ĺ›cieĹĽka nadawania w `HilHardwareSession` oparta wyĹ‚Ä…cznie na `HilTxSafetyGuard`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `hil_execute_controlled_ptt_requires_env_gates_arbiter_and_confirmed_rx`.
- **Kryterium akceptacji:** `execute_controlled_ptt` odrzuca wywoĹ‚anie bez `SPLOG_HIL_ALLOW_TX=1` i weryfikuje `poll().ptt == false` po zwolnieniu PTT.

---

### `CAT-016` â€” `HilTestBenchManifest::to_rig_config` ma na sztywno wpisany adres CI-V `0x94` (IC-7300) dla wszystkich modeli Icom
- **ID:** `CAT-016`
- **TytuĹ‚:** Zahardkodowany `civ_address: 0x94` w `HilTestBenchManifest::to_rig_config` uniemoĹĽliwia testy HIL innych modeli Icom
- **Kategoria:** `CAT`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cat/hil/manifest.rs`
- **Lokalizacja:** [`src/cat/hil/manifest.rs:550-559`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/hil/manifest.rs#L550-L559)
- **DowĂłd z kodu:** W `src/cat/hil/manifest.rs:553` dla `CatBackendKind::IcomCiV` wpisywane jest `civ_address: 0x94` niezaleĹĽnie od wartoĹ›ci `self.matrix.model_name`.
- **Opis problemu:** Manifest HIL dla `IC-7610` (`0x98`), `IC-705` (`0xA4`), `IC-9700` (`0xA2`) lub `IC-706MKIIG` (`0x58`) wygeneruje konfiguracjÄ™ wysyĹ‚ajÄ…cÄ… ramki pod adres `0x94`.
- **Scenariusz wystÄ…pienia:** Uruchomienie testu HIL z manifestu dla dowolnego radia Icom innego niĹĽ IC-7300.
- **WpĹ‚yw:** Brak komunikacji CI-V podczas testĂłw HIL dla pozostaĹ‚ych modeli Icom.
- **RozwiÄ…zanie minimalne:** PobieraÄ‡ domyĹ›lny adres CI-V z `crate::cat::native::registry::find_profile(RigVendor::Icom, &self.matrix.model_name).and_then(|p| p.default_ci_v_address).unwrap_or(0x94)`.
- **RozwiÄ…zanie docelowe:** Opcjonalne pole `civ_address: Option<u8>` w `HilTestBenchManifest` z fallbackiem do rejestru modeli.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `hil_manifest_to_rig_config_uses_profile_civ_address_for_ic7610_and_ic705`.
- **Kryterium akceptacji:** `to_rig_config()` zwraca `0x98` dla `IC-7610` i `0xA4` dla `IC-705`.

---

### `CAT-017` â€” BlokujÄ…ce `std::thread::sleep` (do 255 ms) w `RigctldSupervisor` wywoĹ‚ywane z gĹ‚Ăłwnego wÄ…tku GUI oraz niespĂłjnoĹ›ci profili Yaesu/Elecraft
- **Finding ID:** `CAT-017`
- **Canonical title:** BlokujÄ…ce `std::thread::sleep` (do 255 ms) w `RigctldSupervisor` wywoĹ‚ywane z gĹ‚Ăłwnego wÄ…tku GUI oraz drobne niespĂłjnoĹ›ci profili Yaesu/Elecraft
- **Severity:** `MEDIUM`
- **Confidence:** `CONFIRMED`
- **Original evidence:** [`src/cat/supervisor.rs:316, 375, 385`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/supervisor.rs#L316-L385), [`src/cat/native/yaesu/models.rs:85`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/native/yaesu/models.rs#L85), [`src/cat/service.rs:360-490`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cat/service.rs#L360-L490)
- **Root cause:** Synchroniczne uĹ›pienia na wÄ…tku GUI, niespĂłjnoĹ›Ä‡ flagi `passband: false` w Yaesu oraz brak rozdzielenia stanu sprzÄ™towego PTT od wĹ‚asnoĹ›ci `TxOwned`, brak izolacji generacji sesji przed mutacjÄ… stanu i wyĹ›cig reconnect z pollem.
- **Primary remediation:** Redukcja sleepu w `RigctldSupervisor`, korekta flagi `passband: false` w `create_custom_yaesu_profile` oraz odrzucanie PTT ON w stanie `Degraded`.
- **Additional hardening:**
  1. *Krok 1 (Commit `9c8b877`):* UsuniÄ™cie bezwarunkowego `state.ptt = false` w gaĹ‚Ä™zi udanego poll po reconnect; zachowanie stanu External TX i rozdzielenie go od lokalnej wĹ‚asnoĹ›ci `TxOwned`; obsĹ‚uga `PttState::Unknown`; blokowanie nabycia wĹ‚asnoĹ›ci TX przy aktywnym zewnÄ™trznym nadajniku (`ExternalTransmitActive`) bez wysyĹ‚ania PTT OFF ani PTT ON.
  2. *Krok 2 (Commit `1c4b36a`):* Izolacja generacji sesji (`SessionOperationResult`, weryfikacja `session_generation` przed jakÄ…kolwiek mutacjÄ… stanu wspĂłĹ‚dzielonego w `apply_poll_result`, `apply_connect_result`, `apply_disconnect_result`), odrzucanie spĂłĹşnionych wynikĂłw operacji asynchronicznych (connect, disconnect, poll, command, PTT, bĹ‚Ä™dy, timeouty), powiÄ…zanie deadline reconnectu z generacjÄ… sesji, natychmiastowe anulowanie asynchronicznych operacji przy sygnale shutdown.
  3. *Krok 3 (Commit `af80cbb`):* Rygorystyczny framing odpowiedzi HTTP/1.1 XML-RPC dla trwaĹ‚ego poĹ‚Ä…czenia FLRig keep-alive (`MAX_FLRIG_HTTP_HEADER_BYTES = 8192`, `MAX_FLRIG_XMLRPC_BODY_BYTES = 65536`, trwaĹ‚y bufor odbiorczy w `FlrigConnection`, parsowanie i walidacja `Content-Length`, odrzucanie znieksztaĹ‚ceĹ„ i konfliktĂłw, zachowanie nadmiarowych bajtĂłw w buforze dla kolejnych zapytaĹ„, czyszczenie bufora przy rozĹ‚Ä…czeniu/reconnect).
- **Commits:** `30b6542`, `a215cf6`, `9c8b877`, `1c4b36a`, `af80cbb`
- **Tests:**
  - 10 testĂłw External TX (`cat::hub::tests`, `cat::service::tests`, `cat::tx_arbiter::tests`)
  - 19 testĂłw generacji i wyĹ›cigĂłw sesji (`cat::service::tests`)
  - 22 testy framingu HTTP/1.1 keep-alive (`cat::flrig::tests`)
- **Status:** `FIXED_SOFTWARE_VERIFIED`
- **Residual risk:** NISKIE
---


### Klasyfikacja ZaleĹĽnoĹ›ci Cargo Audit (Etap 4 / CAT-017)
- **Vulnerabilities:** 0
- **Unsound advisories:** 0
- **Yanked crates:** 1
  - Crate: `async-executor 1.14.0`
  - ĹšcieĹĽka zaleĹĽnoĹ›ci: `keyring 4.2.0` â†’ `zbus-secret-service-keyring-store 1.0.1` â†’ `zbus 5.19.0` â†’ `async-executor 1.14.0`
  - Klasyfikacja: `TRANSITIVE / NO KNOWN VULNERABILITY / NO UNSOUND ADVISORY`
  - Decyzja: `TEMPORARILY ACCEPTED FOR CAT-017 GATE`
  - Uzasadnienie:
    - Brak modyfikacji `Cargo.toml` i `Cargo.lock` w bieĹĽÄ…cej bramce RF.
    - Brak jakichkolwiek CVE ani ostrzeĹĽeĹ„ unsound dla tej skrzynki.
    - ZaleĹĽnoĹ›Ä‡ w 100% tranzytywna.
    - Aktualizacja drzewa zaleĹĽnoĹ›ci zaplanowana w dedykowanej bramce dependency maintenance.
- **Unmaintained crates:** 6
  - Osobna kategoria ostrzeĹĽeĹ„ informacyjnych ekosystemu (nie Ĺ‚Ä…czyÄ‡ z yanked).

---

## 5. Architektura (`ARCH-*`), JakoĹ›Ä‡ Kodu Rust (`RUST-*`) i WspĂłĹ‚bieĹĽnoĹ›Ä‡ (`CONC-*`)

### `ARCH-001` â€” BezpoĹ›redni dostÄ™p do SQLite (`db.lock()`) i operacje sieciowe wewnÄ…trz moduĹ‚Ăłw widokĂłw GUI (`src/gui/*`)
- **ID:** `ARCH-001`
- **TytuĹ‚:** Naruszenie separacji warstw: widoki GUI bezpoĹ›rednio wywoĹ‚ujÄ… zapytania SQLite i orkiestrujÄ… klientĂłw HTTP
- **Kategoria:** `ARCH`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/gui/online_sync.rs`, `src/gui/find_duplicates.rs`, `src/gui/statistics.rs`, `src/gui/qsl_designer.rs`, `src/gui/awards_dialog.rs`
- **Lokalizacja:** [`src/gui/online_sync.rs:114-525`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L114-L525), [`src/gui/find_duplicates.rs:270-325`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/find_duplicates.rs#L270-L325), [`src/gui/statistics.rs:135-170`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/statistics.rs#L135-L170)
- **DowĂłd z kodu:** Mimo wydzielenia `QsoIngestor` w fali W4, Ĺ›cieĹĽki odczytu, usuwania duplikatĂłw, potwierdzania QSL i synchronizacji chmurowej pozostaĹ‚y zaszyte bezpoĹ›rednio w plikach renderujÄ…cych GUI (`src/gui/online_sync.rs`, `find_duplicates.rs`, `statistics.rs`), co doprowadziĹ‚o do pominiÄ™cia filtrĂłw `journal_id` (`DATA-005`) i blokowania wÄ…tkĂłw Tokio (`CONC-001`).
- **Opis problemu:** Rozproszenie logiki dostÄ™pu do danych po plikach GUI utrudnia testowanie bez GUI i sprzyja bĹ‚Ä™dom spĂłjnoĹ›ci dziennikĂłw.
- **Scenariusz wystÄ…pienia:** RozwĂłj lub testowanie synchronizacji chmurowej i operacji bazodanowych.
- **WpĹ‚yw:** Duplikacja kodu dostÄ™pu do bazy i pomijanie niezmiennikĂłw domenowych (np. `rebuild_awards_full()`, `undo_stack`, `journal_id`).
- **RozwiÄ…zanie minimalne:** WydzieliÄ‡ operacje synchronizacji chmurowej i masowego potwierdzania QSL z `src/gui/online_sync.rs` do usĹ‚ugi w `src/cloud/` lub `src/core/`.
- **RozwiÄ…zanie docelowe:** Wprowadzenie `LogbookQueryService` i `CloudSyncCoordinator` (zob. `docs/ARCHITECTURE_TARGET_STATE.md`).
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `DATA-004`, `DATA-005`, `CONC-001`.
- **Wymagany test regresyjny:** Testy jednostkowe synchronizacji LoTW/eQSL i usuwania duplikatĂłw wykonywane bez instancji `SpLogApp`.
- **Kryterium akceptacji:** Ĺ»aden plik widoku `src/gui/*.rs` (poza koordynatorem `app_qso.rs`) nie wywoĹ‚uje bezpoĹ›rednio `app.db.lock()`.

---

### `ARCH-002` â€” Duplikacja logiki normalizacji i wzbogacania QSO w `PUT /api/v1/qsos/{id}` oraz `SpLogApp::save_qso` (Ĺ›cieĹĽka edycji)
- **ID:** `ARCH-002`
- **TytuĹ‚:** Edycja istniejÄ…cego QSO (`update_qso`) w GUI i REST API omija `QsoIngestor` i duplikuje logikÄ™ normalizacji/wzbogacania
- **Kategoria:** `ARCH`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/api/server.rs`, `src/gui/app_qso.rs`
- **Lokalizacja:** [`src/api/server.rs:733-774, 811-840`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/api/server.rs#L733-L840), [`src/gui/app_qso.rs:116-146`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_qso.rs#L116-L146)
- **DowĂłd z kodu:**
  1. `src/api/server.rs:733-774` definiuje wĹ‚asnÄ… funkcjÄ™ `normalize_and_enrich_qso` dla `put_qso_by_id` (`server.rs:816`), a po `db.update_qso` **nie przebudowuje `AwardsEngine`**!
  2. `SpLogApp::save_qso` przy edycji istniejÄ…cego QSO (`editing_qso_id = Some(id)`, `app_qso.rs:116-146`) wywoĹ‚uje bezpoĹ›rednio `db.update_qso(&qso)` **bez wywoĹ‚ania `normalize_qso_fields(&mut qso)` i bez `qso.validate()`**!
- **Opis problemu:** Podczas edycji istniejÄ…cego QSO w GUI moĹĽna zapisaÄ‡ niepoprawnÄ… datÄ™/czas lub puste pasmo, a edycja przez `PUT /api/v1/qsos/{id}` zostawia nieaktualny stan `AwardsEngine`.
- **Scenariusz wystÄ…pienia:** Edycja Ĺ‚Ä…cznoĹ›ci w formularzu GUI lub przez `PUT /api/v1/qsos/{id}`.
- **WpĹ‚yw:** MoĹĽliwoĹ›Ä‡ wprowadzenia nieprawidĹ‚owych danych przy edycji QSO oraz rozjazd statystyk dyplomowych po edycji przez REST API.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ metodÄ™ `QsoIngestor::update_existing(id: i64, qso: QsoRecord)` wykonujÄ…cÄ… `normalize_qso_fields`, `validate`, `enrich_with_dxcc`, `db.update_qso` i aktualizacjÄ™ `AwardsEngine`, uĹĽywanÄ… zarĂłwno w `app_qso.rs`, jak i `api/server.rs`.
- **RozwiÄ…zanie docelowe:** Wszystkie mutacje QSO (`insert`, `update`, `delete`, `import_batch`) przechodzÄ… wyĹ‚Ä…cznie przez `QsoIngestor`.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `editing_qso_validates_fields_and_updates_awards_engine`.
- **Kryterium akceptacji:** Edycja QSO w GUI i REST API waliduje dane przez `QsoIngestor` i aktualizuje `AwardsEngine`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-03
- **Commit:**
  - `50321f1`: `fix(storage): unify QSO update path and awards sync through QsoIngestor (ARCH-002)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. Dodano domenowÄ… metodÄ™ `QsoIngestor::update_existing(id, qso)`, ktĂłra wykonuje bezpiecznÄ… normalizacjÄ™, walidacjÄ™, wzbogacenie DXCC, aktualizacjÄ™ bazy w transakcji, peĹ‚nÄ… przebudowÄ™ `AwardsEngine` (`rebuild_from_qsos`) oraz rejestracjÄ™ znaku w `ScpEngine`.
  2. Zaktualizowano `SpLogApp::save_edited_qso` w GUI oraz `put_qso_by_id` w REST API, eliminujÄ…c zduplikowanÄ… funkcjÄ™ `normalize_and_enrich_qso` i gwarantujÄ…c peĹ‚nÄ… synchronizacjÄ™ silnika dyplomĂłw po kaĹĽdej edycji.
- **WdroĹĽone testy regresyjne:**
  - `editing_qso_validates_fields_and_updates_awards_engine` w `src/core/ingest/mod.rs`.

---

### `CONC-001` â€” Blokowanie wÄ…tkĂłw roboczych Tokio przez synchroniczne operacje SQLite (`db.lock()`) bez `spawn_blocking` w `online_sync.rs`
- **ID:** `CONC-001`
- **TytuĹ‚:** Synchroniczne blokowanie `std::sync::Mutex<Database>` i pÄ™tla 1000 pojedynczych transakcji SQLite wewnÄ…trz `tokio::spawn` w `src/gui/online_sync.rs`
- **Kategoria:** `CONC`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia:** 2026-11-03
- **Commity:** `b0afb20` (`fix(data): prevent cross-journal QSO mutations (DATA-004 DATA-005 CONC-001)`)
- **ModuĹ‚ / obszar:** `src/gui/online_sync.rs`
- **Lokalizacja:** [`src/gui/online_sync.rs:112-120, 152-182, 200-208, 240-270, 286-294, 328-336, 360-368, 441-449, 483-491`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/online_sync.rs#L112-L491)
- **DowĂłd z kodu:** W `src/gui/online_sync.rs` wewnÄ…trz `tokio::spawn(async move { ... })`:
  1. Odczyt do 10 000 QSO z SQLite (`db.lock().unwrap().get_recent_qsos(10000)`) jest wykonywany bezpoĹ›rednio na wÄ…tku async Tokio bez `tokio::task::spawn_blocking`.
  2. Po pobraniu potwierdzeĹ„ z LoTW (`online_sync.rs:155-171`) i eQSL (`243-259`), pÄ™tla `for qso in confirmed_qsos` dla kaĹĽdego pojedynczego potwierdzenia osobno zakĹ‚ada blokadÄ™ `db.lock()` i wykonuje osobny `UPDATE` w SQLite (brak transakcji wsadowej!). Dla 1000 potwierdzeĹ„ z LoTW oznacza to 1000 synchronicznych `fsync` w trybie autocommit na wÄ…tku roboczym Tokio!
- **Opis problemu:** GĹ‚odzenie puli wÄ…tkĂłw Tokio i przycinanie wÄ…tku GUI czekajÄ…cego na `self.db.lock()` podczas synchronizacji z LoTW/eQSL.
- **Scenariusz wystÄ…pienia:** Pobranie kilkuset lub kilku tysiÄ™cy potwierdzeĹ„ z LoTW / eQSL.
- **WpĹ‚yw:** ZamroĹĽenie interfejsu GUI i opĂłĹşnienia zadaĹ„ CAT/UDP na czas trwania pÄ™tli `UPDATE`.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ metodÄ™ wsadowÄ… `Database::mark_lotw_confirmed_batch(&[(String, String, String, String)])` (w jednej transakcji `BEGIN IMMEDIATE ... COMMIT`) i wywoĹ‚ywaÄ‡ jÄ… wewnÄ…trz `tokio::task::spawn_blocking`.
- **RozwiÄ…zanie docelowe:** Przeniesienie wszystkich operacji bazodanowych w zadaniach tĹ‚a do `spawn_blocking` z operacjami wsadowymi.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `batch_qsl_confirmation_updates_all_records_in_single_transaction`.
- **Kryterium akceptacji:** Potwierdzenie 1000 Ĺ‚Ä…cznoĹ›ci LoTW/eQSL wykonuje jednÄ… transakcjÄ™ SQLite w `spawn_blocking` (< 20 ms zamiast kilku sekund).

---

### `CONC-002` â€” Tworzenie nowej instancji `reqwest::Client` (i puli TLS) przy kaĹĽdym pojedynczym zapytaniu w `src/cloud/*`
- **ID:** `CONC-002`
- **TytuĹ‚:** Brak wspĂłĹ‚dzielenia `reqwest::Client` w klientach chmurowych (`LoTW`, `eQSL`, `QRZ`, `HamQTH`, `ClubLog`, `HRDLog`, `POTA`, `SOTA`, `Solar`)
- **Kategoria:** `CONC`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/cloud/`
- **Lokalizacja:** [`src/cloud/qrz.rs:28`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/qrz.rs#L28), [`src/cloud/hamqth.rs:34`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/hamqth.rs#L34), [`src/cloud/lotw.rs:23`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/lotw.rs#L23), [`src/cloud/eqsl.rs:20`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/eqsl.rs#L20), [`src/cloud/clublog.rs:22`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/cloud/clublog.rs#L22)
- **DowĂłd z kodu:** Przy kaĹĽdym wyszukaniu znaku (`app_lookup.rs`) lub synchronizacji chmurowej konstruowany jest nowy obiekt klienta budujÄ…cy od zera `reqwest::Client::builder().timeout(...)`, inicjalizujÄ…cy certyfikaty gĹ‚Ăłwne TLS i nowÄ… pulÄ™ poĹ‚Ä…czeĹ„ bez wykorzystania HTTP Keep-Alive pomiÄ™dzy zapytaniami.
- **Opis problemu:** ZbÄ™dny narzut CPU i pamiÄ™ci na kaĹĽdorazowÄ… inicjalizacjÄ™ kontekstu TLS i nowy handshake TCP+TLS.
- **Scenariusz wystÄ…pienia:** Szybkie wpisywanie znakĂłw z wĹ‚Ä…czonym zapytaniem QRZ/HamQTH lub auto-upload po kaĹĽdym QSO.
- **WpĹ‚yw:** WyĹĽsza latencja zapytaĹ„ callbooka i wiÄ™ksze zuĹĽycie pamiÄ™ci.
- **RozwiÄ…zanie minimalne:** PrzechowywaÄ‡ wspĂłĹ‚dzielony `reqwest::Client` (`Clone` oparty wewnÄ™trznie na `Arc`) w `CloudState`.
- **RozwiÄ…zanie docelowe:** WspĂłĹ‚dzielony `reqwest::Client` oraz cache sesji (`SessionKey`) dla QRZ.com i HamQTH (zamiast ponownego logowania przy kaĹĽdym zapytaniu o znak!).
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `qrz_and_hamqth_reuse_session_key_across_consecutive_lookups`.
- **Kryterium akceptacji:** Kolejne zapytania do QRZ/HamQTH wykorzystujÄ… wspĂłĹ‚dzielony `reqwest::Client` i zapamiÄ™tany klucz sesji.

---

### `RUST-001` â€” 341 ostrzeĹĽeĹ„ `cargo clippy` w kodzie produkcyjnym (`lib` + `bin`) i 417 w testach (brak `-D warnings` w CI)
- **ID:** `RUST-001`
- **TytuĹ‚:** Nieegzekwowana czystoĹ›Ä‡ `cargo clippy -- -D warnings` (Ĺ‚Ä…cznie ponad 400 ostrzeĹĽeĹ„ w workspace)
- **Kategoria:** `RUST`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `d2c6970`: `refactor(clippy): clean core and database warnings (RUST-001)`
  - `95d77cb`: `refactor(clippy): clean CAT and digital warnings (RUST-001)`
  - `1ae0eae`: `refactor(clippy): clean GUI warnings (RUST-001)`
  - `21d3bf3`: `refactor(clippy): clean cloud media and plugin warnings (RUST-001)`
  - `efaece4`: `refactor(clippy): clean binaries and integration test warnings (RUST-001)`
  - `037d4c5`: `fix(test): preserve UTF-8 encoding in app_config, psk_reporter and security_guard (RUST-001)`
- **ModuĹ‚ / obszar:** CaĹ‚e repozytorium (`src/**`, `tests/**`, `.github/workflows/ci.yml`)
- **Lokalizacja:** [`.github/workflows/ci.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/ci.yml), [`CODE_QUALITY_NORMALIZATION_PLAN.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/CODE_QUALITY_NORMALIZATION_PLAN.md)
- **DowĂłd z narzÄ™dzi przed naprawÄ…:** Uruchomienie `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` koĹ„czyĹ‚o siÄ™ kodem wyjĹ›cia `101`: **389 unikalnych ostrzeĹĽeĹ„** (768 Ĺ‚Ä…cznie we wszystkich targetach: 294 w `--lib`, 380 w `--lib --tests`, 1 w `--bin SPLogbook`, 4 w `--bin SPLogbook --tests`, 3 w `--bin splogbook-signer`, 3 w `--bin splogbook-signer --tests`, 2 w `cat_native_6b`, 2 w `cat_native_7`).
- **WdroĹĽone rozwiÄ…zanie (Etap 10):**
  1. W 5 kontrolowanych paczkach tematycznych (Core/DB, CAT/Digital, GUI, Cloud/Media/Plugins/Sync/API, Binaries/Integration Tests) wyeliminowano wszystkie **389 unikalnych ostrzeĹĽeĹ„ Clippy** (m.in. `collapsible_if`, `ptr_arg`, `needless_range_loop`, `manual_clamp`, `should_implement_trait`, `derivable_impls`, `type_complexity`, `too_many_arguments`, `for_kv_map`, `if_same_then_else`, `redundant_closure`, `useless_vec`, `field_reassign_with_default`) bez dodawania ani jednego nowego `#[allow(...)]` lub `#![allow(...)]`.
  2. Zweryfikowano `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` z wynikiem **EXIT 0 (0 warnings, 0 errors)**.
- **WdroĹĽone testy regresyjne:**
  - Bramka CI `cargo clippy --workspace --all-targets --all-features -- -D warnings` oraz test straĹĽnika `ci_workflow_enforces_clippy_deny_warnings` w `tests/security_guard.rs`.
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` koĹ„czy siÄ™ kodem `0` i `0` ostrzeĹĽeniami.

---

## 6. WydajnoĹ›Ä‡ (`PERF-*`) i Interfejs Graficzny / UX (`GUI-*`)

### Ujednolicone mapowanie znalezisk i commitĂłw Etapu 9 (Ergonomia Klawiaturowa, TrwaĹ‚oĹ›Ä‡ Konfiguracji i Optymalizacja Renderowania GUI)

Status Etapu 9: **COMPLETED_SOFTWARE_VERIFIED**<br>
Data ukoĹ„czenia: 2026-11-06<br>
Fizyczny HIL: **NOT EXECUTED â€” SAFETY GATE CLOSED** (Brak operacji HIL, 0 RF, 0 PTT).

| Finding ID | TytuĹ‚ kanoniczny | Remediacja gĹ‚Ăłwna | Commity | Zmienione pliki | Testy weryfikacyjne | Status koĹ„cowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `GUI-001` | Blokada skrĂłtĂłw `Ctrl+...` przez aktywny `TextEdit`, brak `Enter`/`Esc` w zawodach i brak determinizmu fokusu | Centralny rejestr skrĂłtĂłw `ShortcutRegistry`, konteksty `ActiveFocusContext` i `ShortcutContext`, dopuszczenie skrĂłtĂłw akcji (`Ctrl+W`, `Ctrl+L`, `Ctrl+S`, `Ctrl+K`) podczas edycji tekstu z blokadÄ… kolidujÄ…cych skrĂłtĂłw nawigacji jednoliterowej, zawÄ™ĹĽenie `Ctrl+W` (Wipe) wyĹ‚Ä…cznie do kontekstĂłw `MainQsoEntry` i `ContestQsoEntry`, obsĹ‚uga `Enter` (logowanie + powrĂłt fokusu na Call) i `Esc` (wipe/anulowanie) w formularzu gĹ‚Ăłwnym i oknie Contest | `e028125`, `229dea5`, `60d43e4` | `src/gui/shortcuts.rs`, `src/gui/app_input.rs`, `src/gui/qso_entry.rs`, `src/gui/contest.rs`, `src/gui/app_tests.rs` | `text_edit_shortcuts_ctrl_w_and_ctrl_l_work_when_input_focused`, `contest_enter_logs_and_returns_focus_to_callsign`, `escape_clears_qso_fields_in_contest_and_main_entry`, `tab_order_cycles_predictably_through_entry_fields`, `ctrl_w_does_not_wipe_qso_when_editing_search_filter` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `GUI-002` | PĹ‚ywajÄ…cy widĹĽet â€žAsystent Operatoraâ€ť nie posiada przycisku zamkniÄ™cia, a jego stan nie zapisuje siÄ™ w konfiguracji | Dodanie `.open(&mut app.gui.windows.show_operator_assistant)` w nagĹ‚Ăłwku okna asystenta, dwukierunkowa synchronizacja z `cfg.ui.show_operator_assistant` w `SpLogApp::snapshot_config()` i wczytywaniu | `e028125`, `acecd8c` | `src/gui/operator_assistant.rs`, `src/gui/app_config.rs`, `src/core/config.rs` | `operator_assistant_visibility_persists_across_config_save`, `operator_assistant_window_can_be_closed_via_header_button` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `GUI-003` | FaĹ‚szywy poczÄ…tkowy odczyt S-metra (`S9+20dB`, `-53 dBm`) i mocy (`100 W`) w `SpLogApp::build` przed podĹ‚Ä…czeniem radia | Inicjalizacja poczÄ…tkowego `RigState` w `SpLogApp::build` kanonicznymi wartoĹ›ciami spoczynkowymi: `s_meter_dbm: -127.0`, `s_meter_unit: "S0"`, `rf_power_watts: 0.0` | `5e273d2` | `src/gui/app.rs`, `src/gui/app_tests.rs` | `initial_rig_state_when_disconnected_reports_s0_and_zero_power` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-005` | Utrata czÄ™Ĺ›ci ustawieĹ„ po restarcie programu z powodu niekompletnego `SpLogApp::snapshot_config()` | Kompletna synchronizacja w `snapshot_config()` (CW makra i wpm, REST API, alerty solarne, satelity, layout/widoki okien, motyw graficzny, multi-radio RigConfig Radio 1 i Radio 2), maskowanie tokena w GUI (`.password(true)`), redagowanie tokena w logach CAT, walidacja schematu, atomowy zapis pliku konfiguracyjnego przez plik tymczasowy `.tmp` z rotacjÄ… `.bak` (`rename_atomic`) i zachowaniem poprawnoĹ›ci poprzedniego pliku w razie awarii | `acecd8c`, `c4cf893`, `d564272` | `src/gui/app_config.rs`, `src/core/config.rs`, `src/gui/menu/tools.rs`, `src/cat/tx_arbiter.rs`, `src/gui/app_tests.rs`, `tests/security_guard.rs` | `snapshot_config_persists_theme_preset_assistant_and_all_subconfigs`, `config_atomic_write_preserves_previous_file_on_write_failure`, `multi_radio_settings_are_preserved_across_save_and_reload`, `rest_api_token_is_masked_in_gui_configuration`, `rest_api_token_is_redacted_in_logs_and_debug_output` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `PERF-002` | Niepotrzebny continuous repaint 250ms w GUI oraz kosztowne operacje $O(N)$ i $O(S \times N)$ w kaĹĽdej klatce | Eliminacja bezwarunkowego `request_repaint_after(250ms)` w pÄ™tli renderowania, zastÄ…pienie harmonogramem UTC `until_next_second`, deterministyczny pomiar opĂłĹşnieĹ„ repaintu, event-driven wake context z `EventBus` i `CatRuntime`, buforowanie `LogViewState` (`sorted_indices`, `worked_callsigns`) z kluczem rewizji `LogViewCacheKey`, buforowanie `ContestState` (`scoreboard`, `mult_matrix`, `band_breakdown`) z kluczem `ContestCacheKey`, przyspieszenie sprawdzania duplikatĂłw spotĂłw z $O(S \times N)$ do $O(1)$ set lookup, buforowanie telemetrii radia `rig_cache` per `RigId` | `8397351`, `5e273d2`, `665183b` | `src/gui/app.rs`, `src/gui/logbook_table.rs`, `src/gui/contest.rs`, `src/gui/state/log_view.rs`, `src/gui/state/contest.rs`, `src/gui/state/cat.rs`, `src/core/events.rs`, `src/cat/runtime.rs` | `idle_gui_does_not_request_continuous_repaint`, `incoming_telemetry_event_requests_gui_repaint`, `cache_is_reused_when_inputs_are_unchanged`, `cache_is_invalidated_on_qso_change`, `cache_is_invalidated_on_filter_change`, `cache_is_invalidated_on_sort_change`, `cache_is_scoped_to_log_identity`, `cache_is_scoped_to_active_rig_when_required`, `stale_cache_never_crosses_profiles` | `FIXED_SOFTWARE_VERIFIED` | NISKIE |

### `PERF-001` â€” Kwadratowy algorytm $O(N^2)$ klonowania i serializacji JSON w `src/sync/catchup.rs` (`chunk_inventory`, `chunk_requests`, `chunk_records`)
- **ID:** `PERF-001`
- **Status:** `FIXED` (Etap 1D-B, commit `7106677`)
- **TytuĹ‚:** Kwadratowa zĹ‚oĹĽonoĹ›Ä‡ $O(N^2)$ w `chunk_inventory`, `chunk_requests` i `chunk_records` serializujÄ…ca caĹ‚e wektory JSON w kaĹĽdej iteracji pÄ™tli
- **Kategoria:** `PERF`
- **Poziom krytycznoĹ›ci:** `CRITICAL`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/sync/catchup.rs`
- **Lokalizacja:** [`src/sync/catchup.rs:32-260`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/catchup.rs#L32-L260)
- **Implementacja przed:**
  - W `chunk_inventory`, `chunk_requests` i `chunk_records` dla kaĹĽdego elementu wejĹ›ciowego klonowano rosnÄ…cy wektor `current` i serializowano caĹ‚Ä… wiadomoĹ›Ä‡ `serde_json::to_vec(&msg)` w celu sprawdzenia przekroczenia `MAX_CHUNK_BYTES`.
  - ZĹ‚oĹĽonoĹ›Ä‡: $O(N^2)$ alokacji, klonĂłw i serializacji. Dla $N=15\,000$ elementĂłw wykonywano $>112\,500\,000$ serializacji obiektĂłw. Czas wykonania 15 000 elementĂłw wynosiĹ‚ ~254â€“257 sekund.
- **Implementacja po:**
  - ZastÄ…piono algorytm rozwiÄ…zaniem Ĺ›ciĹ›le liniowym $O(N)$: kaĹĽdy element serializowany jest dokĹ‚adnie jeden raz w celu zmierzenia jego dĹ‚ugoĹ›ci bajtowej w JSON.
  - Narzut koperty pustej wiadomoĹ›ci oraz separatorĂłw przecinkowych (`1 + item_len`) wyliczany jest przyrostowo co do bajta bez ĹĽadnego wielokrotnego serializowania rosnÄ…cych wektorĂłw.
  - Przy osiÄ…gniÄ™ciu granicy `MAX_CHUNK_BYTES` (1 MiB) bufor `current` jest przenoszony (`std::mem::take`) do chunku i zerowany w $O(1)$.
  - **Kontrakt limitĂłw protokoĹ‚u i SINGLE_RECORD_EXCEPTION_IS_VALID:**
    - `MAX_CHUNK_BYTES` (1 MiB, `src/sync/catchup.rs:18`): docelowy limit grupowania wielu rekordĂłw lub identyfikatorĂłw w pojedynczy `Message`.
    - `MAX_RECORD_BYTES` (6 MiB, `src/sync/catchup.rs:25`): twardy limit pojedynczego rekordu QSO w JSON.
    - `MAX_FRAME_BYTES` (8 MiB, `src/sync/frame.rs:10`): bezwzglÄ™dny limit transportowy ramkowania TCP (`write_frame`, `decode_length`).
    - Pojedyncze rekordy w przedziale $>1\text{ MiB}$ i $\le 6\text{ MiB}$ sÄ… legalnie emitowane w pojedynczym dedykowanym chunku `Message::Records` (`SINGLE_RECORD_EXCEPTION_IS_VALID`).
    - Narzut ramkowania transportowego wynosi 44 B (`FRAME_OVERHEAD_BYTES`: 24 B nonce + 16 B Poly1305 tag + 4 B nagĹ‚Ăłwek dĹ‚ugoĹ›ci), a koperty JSON 33â€“34 B (Ĺ‚Ä…cznie 77â€“78 B). Rekord 6 MiB zajmuje na Ĺ‚Ä…czu $\le 6\,291\,534\text{ B} \ll 8\text{ MiB}$ (zapas $>2{,}09\text{ MiB}$).
    - Rekordy $>6\text{ MiB}$ sÄ… bezpiecznie pomijane przez nadawcÄ™ (`continue;`), emitujÄ…c zdarzenie bĹ‚Ä™du `SyncEvent::Error(SyncError::RecordTooLarge)` oraz log `MULTIOP_ERROR_RECORD_TOO_LARGE` bez przerywania sesji, bez skaĹĽenia bazy i bez ujawniania danych QSO.
    - Odbiorca waliduje 4-bajtowy prefiks dĹ‚ugoĹ›ci w `decode_length` przed zaalokowaniem bufora ramki, zabezpieczajÄ…c stertÄ™ przed nieograniczonÄ… alokacjÄ… przy zĹ‚oĹ›liwych deklaracjach peera.
  - **Liczba operacji liniowych:**
    - `empty_envelope_serializations`: 1,
    - `item_serializations`: $N$,
    - `final_chunk_serializations`: $C$ (liczba wyemitowanych chunkĂłw),
    - `frame_encryptions`: $C$,
    - `frame_encodings`: $C$,
    - ĹÄ…czna liczba operacji: $1 + N + 3C = O(N)$.
- **Wyniki pomiarĂłw (przed -> po, 3 prĂłby):**
  - `chunk_inventory` $N=1\,000$: $2458,97\text{ ms} \rightarrow 3,84\text{ ms}$ (min $3,62$, med $3,84$, max $5,59\text{ ms}$; poprawa **640x**, $99,84\%$)
  - `chunk_inventory` $N=5\,000$: $52\,496,79\text{ ms} \rightarrow 20,78\text{ ms}$ (min $18,38$, med $20,78$, max $24,02\text{ ms}$; poprawa **2526x**, $99,96\%$)
  - `chunk_inventory` $N=15\,000$: $254\,090,00\text{ ms} \rightarrow 63,41\text{ ms}$ (min $58,22$, med $63,41$, max $69,29\text{ ms}$; poprawa **4007x**, $99,975\%$, cel $<200\text{ ms}$ w peĹ‚ni osiÄ…gniÄ™ty)
  - `chunk_requests` $N=15\,000$: $256\,840,00\text{ ms} \rightarrow 62,21\text{ ms}$ (min $56,46$, med $62,21$, max $77,21\text{ ms}$; poprawa **4129x**)
  - `chunk_records` $N=3\,000$: $45\,214,06\text{ ms} \rightarrow 152,66\text{ ms}$ (min $131,23$, med $152,66$, max $185,83\text{ ms}$; poprawa **296x**)
  - `chunk_records` $N=15\,000$: $\sim 1100\text{ s (szac.)} \rightarrow 662,82\text{ ms}$ (min $636,53$, med $662,82$, max $688,86\text{ ms}$)
- **Weryfikacja testowa:**
  - `chunk_inventory_preserves_order_and_size_limit`, `chunk_requests_preserves_order_and_size_limit`, `chunk_records_preserves_order_and_size_limit` (kompletnoĹ›Ä‡ danych, kolejnoĹ›Ä‡, brak duplikatĂłw, znaki UTF-8, znaki wymagajÄ…ce ucieczki JSON `\"`, `\\`, `\n`).
  - `chunk_inventory_handles_exact_boundary`, `chunk_requests_handles_exact_boundary`, `chunk_records_handles_exact_boundary` (przypadki graniczne rozmiaru ramki).
  - `optimized_*_chunking_matches_reference_for_small_inputs` (porĂłwnanie wynikĂłw z implementacjÄ… referencyjnÄ…).
  - `catchup_chunking_15k_identities_has_linear_serialization_count` (deterministyczny licznik serializacji $\le N + 10$).
  - `catchup_chunking_scaling_ratio_is_near_linear` (stosunek skalowania $4\text{k}/2\text{k} < 2,5$, brak $O(N^2)$).
  - `single_large_record_roundtrips_through_production_frame_codec` (peĹ‚ny roundtrip rekordu 1â€“6 MiB przez produkcyjny frame codec, krypto i serializacjÄ™ bez naruszenia limitu transportowego).
  - `frame_decoder_rejects_length_above_transport_limit` oraz `frame_decoder_rejects_length_overflow` (odrzucenie deklaracji dĹ‚ugoĹ›ci > 8 MiB i `u32::MAX` bez alokacji bufora).
  - `large_record_rejection_has_no_partial_side_effects` (odrzucenie rekordu > 6 MiB bez czÄ™Ĺ›ciowego skaĹĽenia bazy).
  - `oversized_record_rejection_does_not_claim_full_catchup_success` (brak faĹ‚szywego sukcesu synchronizacji przy pominiÄ™tym rekordzie).
  - `catchup_chunking_15k_reports_complete_linear_operation_counts` (peĹ‚ny rozkĹ‚ad licznikĂłw operacji $1 + N + 3C$).
  - `estimated_chunk_size_matches_actual_serialized_size` (dokĹ‚adnoĹ›Ä‡ estymacji co do bajta dla UTF-8, znakĂłw specjalnych i komentarzy).
  - `last_flag_size_delta_is_accounted_for` (sprawdzenie 1-bajtowej delty `"last":false` vs `"last":true` i konserwatywnoĹ›ci estymacji).
- **PamiÄ™Ä‡ i alokacje (sprostowanie po audycie):**
  - Poprzedni algorytm generowaĹ‚ ponad 6 GiB skumulowanego wolumenu alokacji tymczasowych (total allocated temporary bytes over execution) dla $N=15\,000$ elementĂłw wskutek klonowania wektorĂłw i wielokrotnej serializacji.
  - Nowy algorytm utrzymuje w pamiÄ™ci ĹĽywej (live heap) wyĹ‚Ä…cznie wektor wejĹ›ciowy i bieĹĽÄ…cy chunk.
  - Szczyt pamiÄ™ci fizycznej procesu (peak RSS) nie zostaĹ‚ bezpoĹ›rednio zmierzony w teĹ›cie.
- **Ryzyko resztkowe:** Niskie (`LOW`).
  - Wyniki pomiarĂłw czasowych zaleĹĽÄ… od parametrĂłw sprzÄ™towych procesora i systemu operacyjnego.
  - SzczegĂłĹ‚owe instrumentation licznikĂłw dziaĹ‚a wyĹ‚Ä…cznie pod `#[cfg(test)]`.
  - Peak RSS nie zostaĹ‚ zmierzony bezpoĹ›rednio.
  - DuĹĽe pojedyncze rekordy (1â€“6 MiB) wymagajÄ… monitorowania pasma LAN przy Ĺ‚Ä…czach o ograniczonej przepustowoĹ›ci.
  - KompatybilnoĹ›Ä‡ wire formatu zostaĹ‚a potwierdzona testami end-to-end, ale nie z heterogenicznymi historycznymi klientami na ĹĽywej sieci.

---

### `PERF-002` â€” Nieograniczone Ĺ‚adowanie 100% bazy do `recent_qsos` w `reload_qsos()` oraz praca $O(N)$ i $O(S \times N)$ w kaĹĽdej klatce GUI (60 FPS)
- **ID:** `PERF-002`
- **TytuĹ‚:** `reload_qsos()` Ĺ‚aduje wszystkie rekordy dziennika do RAM, a `logbook_table.rs` i `contest.rs` skanujÄ… i sortujÄ… je w kaĹĽdej klatce renderowania
- **Kategoria:** `PERF`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/gui/app_qso.rs`, `src/gui/logbook_table.rs`, `src/gui/contest.rs`, `src/gui/spots_map_dialog.rs`
- **Lokalizacja:** [`src/gui/app_qso.rs:475-515, 558-561, 597-600`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_qso.rs#L475-L600), [`src/gui/logbook_table.rs:323-373`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/logbook_table.rs#L323-L373), [`src/gui/contest.rs:146-175, 223-241, 315-345, 505-525, 555-625`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/contest.rs#L146-L625)
- **DowĂłd z kodu:**
  1. W `SpLogApp::reload_qsos` (`src/gui/app_qso.rs:483-485`) wykonywane jest `let total = db.count_for_journal(&jid).unwrap_or(0); let limit = requested.max(total);`, co Ĺ‚aduje **caĹ‚Ä… zawartoĹ›Ä‡ dziennika** do `self.entry.recent_qsos` przy kaĹĽdym dodaniu/usuniÄ™ciu QSO (a w `delete_qso_by_id`, `delete_selected_qso` i `perform_undo` baza jest przeĹ‚adowywana **3 razy pod rzÄ…d**: w `reload_qsos()`, `rebuild_scp()` i `rebuild_awards_full()`!).
  2. W `logbook_table.rs:323-373` w kaĹĽdej klatce GUI tworzony jest wektor `filtered: Vec<&QsoRecord>` z wielokrotnymi alokacjami `.to_lowercase()` na kaĹĽdym rekordzie oraz peĹ‚nym sortowaniem `filtered.sort_by(...)`.
  3. W `contest.rs` w kaĹĽdej klatce wykonywanych jest **5 peĹ‚nych przebiegĂłw $O(N)$** po `recent_qsos` oraz pÄ™tla $O(S \times N)$ sprawdzajÄ…ca duplikaty dla kaĹĽdego spotu z klastra (`contest.rs:514-522`).
- **Opis problemu:** Przy dzienniku >10 000 QSO interfejs graficzny traci pĹ‚ynnoĹ›Ä‡ 60 FPS i zuĹĽywa nadmiernie CPU w stanie spoczynku.
- **Scenariusz wystÄ…pienia:** Praca z duĹĽym logiem (>10 000 QSO) w oknie gĹ‚Ăłwnym lub w trybie Contest.
- **WpĹ‚yw:** Spadek liczby klatek na sekundÄ™ (FPS), wysokie obciÄ…ĹĽenie CPU i opĂłĹşnienia wpisywania znakĂłw.
- **RozwiÄ…zanie minimalne:**
  1. W `LogViewState` buforowaÄ‡ przefiltrowane i posortowane indeksy `filtered_indices: Vec<usize>` i przeliczaÄ‡ je tylko po zmianie `recent_qsos` lub zmianie tekstu filtra/kolumny sortowania.
  2. W `delete_*` i `perform_undo` przekazywaÄ‡ juĹĽ wczytane `recent_qsos` do `rebuild_scp` i `rebuild_awards_full` zamiast 3-krotnego odczytu z SQLite.
  3. W `contest.rs` utrzymywaÄ‡ inkrementalny `HashSet` duplikatĂłw i mnoĹĽnikĂłw aktualizowany wyĹ‚Ä…cznie przy zmianie liczby QSO.
- **RozwiÄ…zanie docelowe:** Prawdziwe stronicowanie SQLite w `LogViewState` i indeksowana pamiÄ™Ä‡ podrÄ™czna sesji zawodĂłw.
- **Ryzyko wdroĹĽenia zmiany:** Niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `M`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** Test wydajnoĹ›ciowy renderowania listy / filtrowania dla 25 000 rekordĂłw (< 2 ms przy niezmienionym filtrze).
- **Kryterium akceptacji:** Brak sortowania i alokacji `to_lowercase()` w klatkach GUI, w ktĂłrych dane ani filtr nie ulegĹ‚y zmianie.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `8397351`: `perf(gui): replace unnecessary continuous repaint with event-driven updates`
  - `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
  - `665183b`: `test(gui): make Stage 9 performance measurements reproducible (PERF-002)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. UsuniÄ™to bezwarunkowe ĹĽÄ…danie odĹ›wieĹĽania `ui.ctx().request_repaint_after(Duration::from_millis(250))` w gĹ‚Ăłwnej pÄ™tli renderowania `render_main_ui`. ZastÄ…piono je harmonogramem odĹ›wieĹĽania zegara UTC do poczÄ…tku kolejnej peĹ‚nej sekundy (`until_next_second`), co eliminuje ciÄ…gĹ‚y repaint w stanie spoczynku aplikacji (idle delay ~608-999 ms zamiast 250 ms pÄ™tli).
  2. Wprowadzono architekturÄ™ event-driven wake: komponenty `EventBus` i `CatRuntime` posiadajÄ… metody `set_repaint_trigger` rejestrujÄ…ce referencjÄ™ do kontekstu `egui`, wywoĹ‚ujÄ…ce `request_repaint()` natychmiast po odebraniu nowego pakietu telemetrii lub zdarzenia systemowego, bez potrzeby ciÄ…gĹ‚ego pooling-u w GUI.
  3. W `LogViewState` zaimplementowano buforowanie indeksĂłw sortowania i filtrowania (`sorted_indices`) oraz zbioru unikalnych znakĂłw (`worked_callsigns`) powiÄ…zane z kluczem `LogViewCacheKey` (rewizja dziennika, zapytanie wyszukiwania, wybrane pasmo/emisja, kolumna sortowania). PÄ™tla renderowania tabeli logbooka nie wykonuje alokacji ani sortowania dopĂłki stan wejĹ›ciowy nie ulegnie zmianie.
  4. W `ContestState` wdroĹĽono buforowanie agregacji (`scoreboard`, `mult_matrix`, `band_breakdown`, `rate`) w oparciu o `ContestCacheKey` (rewizja logu, toĹĽsamoĹ›Ä‡ zawodĂłw, kategoria).
  5. W `contest.rs` weryfikacjÄ™ duplikatĂłw spotĂłw klastra DX zoptymalizowano ze zĹ‚oĹĽonoĹ›ci $O(S \times N)$ do $O(1)$ set lookup za pomocÄ… buforowanego zbioru `worked_callsigns`.
  6. W `CatState` wprowadzono bezpiecznÄ… pamiÄ™Ä‡ podrÄ™cznÄ… telemetrii radia (`rig_cache`), powiÄ…zanÄ… z toĹĽsamoĹ›ciÄ… aktywnego `RigId` i rewizjÄ… stanu, z zachowaniem Ĺ›cisĹ‚ego limitu pĂłl struktury (poniĹĽej 30 pĂłl).
- **WdroĹĽone testy regresyjne:**
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

### `GUI-001` â€” SkrĂłty klawiszowe `Ctrl+...` nie dziaĹ‚ajÄ… podczas wpisywania w formularzu QSO, a w oknie Contest brakuje `Enter` (zapis) i `Esc` (Wipe)
- **ID:** `GUI-001`
- **TytuĹ‚:** Zablokowanie globalnych skrĂłtĂłw `Ctrl+N/W/F/L/D/B/M/K/P/E/I/T` przez `!ctx.egui_wants_keyboard_input()` oraz brak obsĹ‚ugi klawiatury w `contest.rs`
- **Kategoria:** `GUI`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/gui/app_input.rs`, `src/gui/contest.rs`
- **Lokalizacja:** [`src/gui/app_input.rs:44-89`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_input.rs#L44-L89), [`src/gui/contest.rs:418-455`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/contest.rs#L418-L455)
- **DowĂłd z kodu:**
  1. W `SpLogApp::handle_keyboard_shortcuts` (`src/gui/app_input.rs:44`) caĹ‚y blok skrĂłtĂłw z modyfikatorem `Ctrl` (`Ctrl+N` fokus na znak, `Ctrl+W` wyczyĹ›Ä‡ formularz, `Ctrl+F` szukaj, `Ctrl+L` zapisz QSO, `Ctrl+D` duplikaty, `Ctrl+B` pasmo, `Ctrl+M` emisja, `Ctrl+K` terminal CW, `Ctrl+P` paleta poleceĹ„) znajduje siÄ™ wewnÄ…trz warunku `if !ctx.egui_wants_keyboard_input()`. W `egui` metoda `egui_wants_keyboard_input()` zwraca `true` zawsze, gdy fokus znajduje siÄ™ w dowolnym polu `TextEdit` (czyli podczas normalnej pracy operatora wpisujÄ…cego znak, RST lub imiÄ™!). W rezultacie **`Ctrl+W` (wyczyĹ›Ä‡ formularz) i `Ctrl+L` (zapisz QSO) nie dziaĹ‚ajÄ… wtedy, gdy kursor jest w formularzu QSO**!
  2. W `src/gui/contest.rs:418-455` naciĹ›niÄ™cie `Enter` w polu znaku lub wymiany nie zapisuje Ĺ‚Ä…cznoĹ›ci (wymagane jest klikniÄ™cie myszÄ… przycisku â€žZapisz QSOâ€ť!), `Esc` nie czyĹ›ci pĂłl formularza konkursowego, a po zapisaniu Ĺ‚Ä…cznoĹ›ci fokus nie wraca automatycznie do pola znaku.
- **Opis problemu:** Podstawowa ergonomia pracy klawiaturowej (szczegĂłlnie w zawodach i pileupie) jest zablokowana.
- **Scenariusz wystÄ…pienia:** Operator wpisuje bĹ‚Ä™dny znak w polu Callsign i naciska `Ctrl+W`, aby wyczyĹ›ciÄ‡ pole, lub naciska `Enter` w oknie Contest, aby zalogowaÄ‡ QSO.
- **WpĹ‚yw:** KoniecznoĹ›Ä‡ odrywania rÄ…k od klawiatury i siÄ™gania po mysz podczas pracy operacyjnej i zawodĂłw.
- **RozwiÄ…zanie minimalne:**
  1. WyciÄ…gnÄ…Ä‡ skrĂłty z modyfikatorem `Ctrl` (`Ctrl+W`, `Ctrl+L`, `Ctrl+N`, `Ctrl+F`, `Ctrl+P`, `Ctrl+K`, `Ctrl+B`, `Ctrl+M`) przed warunek `if !ctx.egui_wants_keyboard_input()` (pozostawiajÄ…c wewnÄ…trz tego warunku jedynie pojedyncze klawisze bez `Ctrl`, takie jak `Delete`).
  2. W `contest.rs` obsĹ‚uĹĽyÄ‡ `Key::Enter` (zapis QSO + powrĂłt fokusu `response.request_focus()` na pole znaku) oraz `Key::Escape` (wyczyszczenie pĂłl znaku i wymiany).
- **RozwiÄ…zanie docelowe:** PeĹ‚ny tryb ESM (Enter Sends Message) w oknie Contest.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `keyboard_shortcuts_ctrl_w_and_ctrl_l_work_when_text_edit_focused`.
- **Kryterium akceptacji:** `Ctrl+W` i `Ctrl+L` dziaĹ‚ajÄ… przy aktywnym polu `TextEdit`, a `Enter` w oknie Contest zapisuje QSO i ustawia fokus z powrotem na pole znaku.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
  - `229dea5`: `fix(gui): make QSO focus and keyboard workflow deterministic`
  - `60d43e4`: `fix(gui): isolate Ctrl+W wipe to explicit QSO contexts (GUI-001)`
- **WdroĹĽone rozwiÄ…zanie:**
  1. Zaimplementowano centralny moduĹ‚ obsĹ‚ugi skrĂłtĂłw klawiszowych `src/gui/shortcuts.rs` oparty o `ShortcutRegistry`, rozrĂłĹĽniajÄ…cy konteksty wykonania (`Global`, `QsoEntry`, `Contest`, `LogTable`).
  2. WyciÄ…gniÄ™to skrĂłty z modyfikatorem `Ctrl` (`Ctrl+W` czyszczenie pĂłl, `Ctrl+L` / `Ctrl+S` zapis Ĺ‚Ä…cznoĹ›ci, `Ctrl+N` fokus na znak, `Ctrl+F` wyszukiwarka, `Ctrl+K` CW terminal) przed warunek odrzucania zdarzeĹ„ przy aktywnym polu `TextEdit`, chroniÄ…c jednoczeĹ›nie pojedyncze klawisze tekstowe i nawigacyjne przed kolizjÄ… podczas edycji.
  3. W oknie zawodĂłw `contest.rs` wdroĹĽono peĹ‚nÄ… obsĹ‚ugÄ™ klawiatury: `Key::Enter` wykonuje zapis Ĺ‚Ä…cznoĹ›ci i automatycznie ĹĽÄ…da przywrĂłcenia fokusu na pole Call (`FocusRequest::Field("callsign")`), `Key::Escape` czyĹ›ci bieĹĽÄ…ce pola formularza i przywraca fokus na znak. W razie bĹ‚Ä™du walidacji (niepeĹ‚ny znak, bĹ‚Ä™dna wymiana) dane w polach nie sÄ… niszczone, a operator otrzymuje komunikat bĹ‚Ä™du.
  4. SkrĂłt `Esc` zostaĹ‚ sparametryzowany kontekstowo (`EscapeContextRouting`): w pierwszej kolejnoĹ›ci zamyka aktywne okna dialogowe/paletÄ™ poleceĹ„, nastÄ™pnie czyĹ›ci formularz QSO i przywraca fokus na pole Callsign, a w spoczynku nie wywoĹ‚uje ĹĽadnych efektĂłw ubocznych.
  5. KolejnoĹ›Ä‡ klawisza `Tab` (`TabOrder`) w formularzu QSO i Contest jest deterministyczna i cykliczna.
- **WdroĹĽone testy regresyjne:**
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

### `GUI-002` â€” PĹ‚ywajÄ…cy widĹĽet â€žAsystent Operatoraâ€ť (`operator_assistant.rs`) nie posiada przycisku zamkniÄ™cia, a jego stan nie zapisuje siÄ™ w konfiguracji
- **ID:** `GUI-002`
- **TytuĹ‚:** Brak moĹĽliwoĹ›ci zamkniÄ™cia pĹ‚ywajÄ…cego okna `operator_assistant.rs` z jego nagĹ‚Ăłwka i brak trwaĹ‚oĹ›ci flagi `show_operator_assistant`
- **Kategoria:** `GUI`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/gui/operator_assistant.rs`, `src/gui/app_config.rs`
- **Lokalizacja:** [`src/gui/operator_assistant.rs:23-35`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/operator_assistant.rs#L23-L35), [`src/gui/app_config.rs:28-59`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app_config.rs#L28-L59)
- **DowĂłd z kodu:** Okno `egui::Window::new(t("assistant.title"))` w `operator_assistant.rs:23-35` nie przekazuje `.open(&mut app.gui.windows.show_operator_assistant)` i nie ma przycisku `âś•`, przez co zasĹ‚ania prawy dolny rĂłg ekranu i po ukryciu z menu wraca po kaĹĽdym restarcie programu (`FUNC-005`).
- **Opis problemu:** UciÄ…ĹĽliwe pĹ‚ywajÄ…ce okno zasĹ‚aniajÄ…ce tabelÄ™ logu na mniejszych ekranach.
- **Scenariusz wystÄ…pienia:** Praca na ekranie 1366x768 lub 1920x1080.
- **WpĹ‚yw:** Pogorszenie czytelnoĹ›ci dolnej czÄ™Ĺ›ci gĹ‚Ăłwnego okna aplikacji.
- **RozwiÄ…zanie minimalne:** DodaÄ‡ `.open(&mut open)` w `operator_assistant.rs` zapisujÄ…ce stan do `app.gui.windows.show_operator_assistant` oraz utrwaliÄ‡ tÄ™ flagÄ™ w `snapshot_config()`.
- **RozwiÄ…zanie docelowe:** MoĹĽliwoĹ›Ä‡ zadokowania Asystenta Operatora jako zakĹ‚adki w `egui_dock`.
- **Ryzyko wdroĹĽenia zmiany:** Bardzo niskie.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** `FUNC-005`.
- **Wymagany test regresyjny:** `operator_assistant_visibility_persists_across_config_save`.
- **Kryterium akceptacji:** Okno Asystenta posiada przycisk zamkniÄ™cia `âś•`, a jego wyĹ‚Ä…czenie jest pamiÄ™tane po restarcie.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `e028125`: `fix(gui): centralize context-aware keyboard shortcuts`
  - `acecd8c`: `fix(config): make application settings durable and atomic`
- **WdroĹĽone rozwiÄ…zanie:**
  1. W `src/gui/operator_assistant.rs` dodano parametr `.open(&mut app.gui.windows.show_operator_assistant)` do definicji `egui::Window`, umoĹĽliwiajÄ…c bezpoĹ›rednie zamkniÄ™cie okna za pomocÄ… przycisku `âś•` w nagĹ‚Ăłwku.
  2. Stan flagi `show_operator_assistant` w `GuiWindowsState` zostaĹ‚ w peĹ‚ni powiÄ…zany z polem `cfg.ui.show_operator_assistant` w `snapshot_config()` i jest trwale zapisywany w `config.json`.
- **WdroĹĽone testy regresyjne:**
  - `operator_assistant_visibility_persists_across_config_save`
  - `operator_assistant_window_can_be_closed_via_header_button`

---

### `GUI-003` â€” FaĹ‚szywy poczÄ…tkowy odczyt S-metra (`S9+20dB`, `-53 dBm`) i mocy (`100 W`) w `SpLogApp::build` przed podĹ‚Ä…czeniem radia
- **ID:** `GUI-003`
- **TytuĹ‚:** Inicjalizacja `RigState` w `SpLogApp::build` nieprawdziwymi wartoĹ›ciami `S9+20dB` i `100.0 W`
- **Kategoria:** `GUI`
- **Poziom krytycznoĹ›ci:** `MEDIUM`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/gui/app.rs`
- **Lokalizacja:** [`src/gui/app.rs:245-258`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/gui/app.rs#L245-L258)
- **DowĂłd z kodu:** W `SpLogApp::build` (`src/gui/app.rs:251-253`) poczÄ…tkowy `initial_rig_state` ustawia `s_meter_dbm: -53.0`, `s_meter_unit: "S9+20dB".to_string()`, `rf_power_watts: 100.0` przy `connected: false`.
- **Opis problemu:** Przed podĹ‚Ä…czeniem jakiegokolwiek radia (oraz w `GET /api/v1/rig`) program raportuje wychylenie S-metra `S9+20dB`!
- **Scenariusz wystÄ…pienia:** Uruchomienie programu bez podĹ‚Ä…czonego transceivera.
- **WpĹ‚yw:** MylÄ…ce wskazanie S-metra w GUI i REST API w stanie rozĹ‚Ä…czonym.
- **RozwiÄ…zanie minimalne:** ZainicjalizowaÄ‡ `initial_rig_state` wartoĹ›ciami spoczynkowymi (`s_meter_dbm: -127.0`, `s_meter_unit: "S0".to_string()`, `rf_power_watts: 0.0`).
- **RozwiÄ…zanie docelowe:** UĹĽycie `RigState::default()` i wygaszenie paska S-metra, gdy `!app.cat.connected`.
- **Ryzyko wdroĹĽenia zmiany:** Zerowe.
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** Brak.
- **Wymagany test regresyjny:** `initial_rig_state_when_disconnected_reports_s0_and_zero_power`.
- **Kryterium akceptacji:** Po starcie programu bez radia S-meter wskazuje `S0` (`-127 dBm`), a moc `0 W`.
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commity:**
  - `5e273d2`: `perf(gui): cache expensive logbook and dashboard computations`
- **WdroĹĽone rozwiÄ…zanie:**
  - W `SpLogApp::build` w `src/gui/app.rs` zmieniono wartoĹ›ci poczÄ…tkowe `initial_rig_state` na spoczynkowe: `s_meter_dbm: -127.0`, `s_meter_unit: "S0".to_string()`, `rf_power_watts: 0.0`. Przed fizycznym nawiÄ…zaniem poĹ‚Ä…czenia z radiem aplikacja oraz endpointy REST nie raportujÄ… faĹ‚szywych wartoĹ›ci `S9+20dB` ani `100W`.
- **WdroĹĽone testy regresyjne:**
  - `initial_rig_state_when_disconnected_reports_s0_and_zero_power`

---

## 7. Testy (`TEST-*`), Dokumentacja (`DOC-*`), CI/CD (`CI-*`) i ZaleĹĽnoĹ›ci (`DEP-*`)

### Ujednolicone mapowanie znalezisk i commitĂłw Etapu 10 (PodĹ‚Ä…czenie Funkcji Pozornych, CzystoĹ›Ä‡ `cargo clippy -D warnings` i Dokumentacja)

Status Etapu 10: **COMPLETED_SOFTWARE_VERIFIED**<br>
Data ukoĹ„czenia: 2026-11-06<br>
Fizyczny HIL: **NOT EXECUTED â€” SAFETY GATE CLOSED** (Brak operacji HIL, 0 RF, 0 PTT).

| Finding ID | TytuĹ‚ kanoniczny | Remediacja gĹ‚Ăłwna | Commity | Zmienione pliki | Testy weryfikacyjne | Status koĹ„cowy | Ryzyko resztkowe |
|---|---|---|---|---|---|---|---|
| `FUNC-004` | Funkcje atrapowe, czÄ™Ĺ›ciowo podĹ‚Ä…czone lub nieosiÄ…galne w GUI i backendzie (CW/WinKeyer, JS8Call, FLDigi, Audio Stop, EME Rotor, Photo Viewer) | PeĹ‚ny przepĹ‚yw produkcyjny dla 6 funkcji: `AudioRecorder::stop_recording` z traitem `AudioSink` i finalizacjÄ… WAV; `PhotoViewerDialog` z przekazaniem wybranego ĹşrĂłdĹ‚a i walidacjÄ…; rotator EME `rotate_antenna_to_el` z walidacjÄ… az/el i bez fallbacku na pierwsze radio; dynamiczny `Js8CallListener` (`js8call_enabled`/`js8call_port`); `FldigiClient::test_connection` z aktualizacjÄ… stanu GUI; binarne ramki K1EL i `WinkeyerTransport` | `9cb49dc`, `85104a1`, `a5d9bde` | `src/media/audio_recorder.rs`, `src/gui/{qso_entry.rs,photo_viewer.rs,astronomy_dialog.rs,app_cat.rs,app_services.rs,online_sync.rs,cw_terminal.rs}`, `src/digital/{js8call.rs,fldigi.rs}`, `src/cat/{rotor.rs,winkeyer.rs}` | 36 testĂłw regresyjnych (8 audio, 5 photo viewer, 6 EME rotor, 6 JS8Call, 5 FLDigi, 6 WinKeyer) | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-006` | `ClubRegistry::check` w `src/core/clubs.rs` jest zahardkodowanÄ… makietÄ… dla 11 znakĂłw wywoĹ‚awczych | Dynamiczne Ĺ‚adowanie czĹ‚onkostwa z `databases/clubs.csv`, obsĹ‚uga 7 klubĂłw, rozdzielenie klucza toĹĽsamoĹ›ci `(club_id, normalized_callsign)` od kontroli konfliktu numeru `(club_id, normalized_member_number)`, dozwolenie tego samego numeru w rĂłĹĽnych klubach, odrzucenie konfliktĂłw w tym samym klubie, idempotencja duplikatĂłw | `062d65c`, `caf72d1`, `0d28ab8` | `src/core/clubs.rs`, `databases/clubs.csv`, `README.md` | 19 testĂłw regresyjnych (`club_registry_*`, `club_membership_*`) | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-007` | NieuĹĽywane pola dyplomowe `worked_wwff` i `worked_rda` w `AwardsEngine` oraz martwa historia propagacji w GUI | ObsĹ‚uga `WWFF` i `RDA` w `AwardsEngine::ingest_qso` i `awards_matrix.rs`, prezentacja `PropagationHistory` (`accuracy_pct`, `band_accuracy_pct`) z jednoznacznym stanem `N/A` (brak prĂłbek) odrÄ™bnym od zmierzonego `0.0%`, podĹ‚Ä…czenie `qsl_print::calculate_total_sheets` w `qsl_designer.rs` | `53d532a`, `caf72d1`, `4d09962` | `src/core/{awards.rs,propagation_history.rs,qsl_print.rs}`, `src/gui/{awards_matrix.rs,solar_panel.rs,qsl_designer.rs,app_tests.rs}` | 13 testĂłw regresyjnych (`awards_engine_*`, `propagation_history_*`, `solar_panel_*`) | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `FUNC-008` | `LotwClient::fetch_confirmations` nie przesyĹ‚a parametru `qso_qslsince` do serwera ARRL LoTW | Walidacja i przekazywanie parametru `qso_qslsince=YYYY-MM-DD` w `LotwClient::build_confirmations_url` / `fetch_confirmations` z redakcjÄ… sekretĂłw w diagnostyce | `53d532a` | `src/cloud/lotw.rs` | 4 testy regresyjne (`lotw_report_*`) | `FIXED_SOFTWARE_VERIFIED` | NISKIE |
| `RUST-001` | 341 ostrzeĹĽeĹ„ `cargo clippy` w kodzie produkcyjnym i 417 w testach | Oczyszczenie wszystkich 389 unikalnych ostrzeĹĽeĹ„ Clippy w 5 kontrolowanych paczkach tematycznych bez dodawania nowych `#[allow(...)]`; `cargo clippy --workspace --all-targets --all-features -- -D warnings` zwraca 0 ostrzeĹĽeĹ„ | `d2c6970`, `95d77cb`, `1ae0eae`, `21d3bf3`, `efaece4`, `037d4c5` | `src/**`, `tests/**` | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` (EXIT 0) | `FIXED_SOFTWARE_VERIFIED` | ZEROWE |
| `CI-002` | Brak flagi `-D warnings` dla `cargo clippy` oraz brak `cargo audit` w `.github/workflows/ci.yml` | Wymuszenie `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, `cargo test --workspace --all-targets`, `cargo test --doc`, przypiÄ™cie `cargo install cargo-audit --version 0.22.2 --locked` oraz `cargo audit --deny warnings --no-yanked --ignore ...` (z jawnÄ… listÄ… 7 zaakceptowanych advisories) w `.github/workflows/ci.yml` | `7466bdd`, `8868be6` | `.github/workflows/ci.yml`, `tests/security_guard.rs` | 15 testĂłw straĹĽnika CI i audytu w `tests/security_guard.rs` | `FIXED_SOFTWARE_VERIFIED` | ZEROWE |
| `DOC-001` | RozbieĹĽnoĹ›ci miÄ™dzy `README.md`, `Native CAT.md` i stanem rzeczywistym kodu | PeĹ‚na synchronizacja licznikĂłw testĂłw (`2279` aktywnych passed + `10` ignored HIL = `2289` registered), eliminacja artefaktĂłw KaTeX, korekta opisĂłw toĹĽsamoĹ›ci `ClubRegistry` i stanu propagacji N/A, raporty Etapu 10 | Commity dokumentacyjne Etapu 10 | `README.md`, `Native CAT.md`, `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`, `STAGE_10_FINAL_GATE_REPORT.md` | Weryfikacja spĂłjnoĹ›ci Ĺ›cieĹĽek, portĂłw i licznikĂłw testĂłw | `FIXED_SOFTWARE_VERIFIED` | ZEROWE |

### `TEST-001` â€” Czas wykonania zestawu testĂłw jednostkowych wynosi ~532 sekundy (~9 minut) w profilu debug z powodu 4 testĂłw w `src/sync/mod.rs`
- **ID:** `TEST-001`
- **Status:** `FIXED` (Etap 1D-B, commity `7106677` i `d4cd4d2`)
- **TytuĹ‚:** Ekstremalnie wolne wykonanie `cargo test` spowodowane przez $O(N^2)$ w `catchup.rs` oraz incydentalne wywoĹ‚ania Argon2id w testach framing/session
- **Kategoria:** `TEST`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **ModuĹ‚ / obszar:** `src/sync/mod.rs`, `src/sync/catchup.rs`, `src/sync/crypto.rs`
- **Lokalizacja:** [`src/sync/mod.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/mod.rs), [`src/sync/crypto.rs:21-40`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/crypto.rs#L21-L40), [`src/sync/catchup.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/src/sync/catchup.rs)
- **Implementacja przed:**
  - `cargo test --lib` trwaĹ‚ **353,36 s** (wczeĹ›niej w audycie 532 s).
  - `cargo test sync:: --lib` trwaĹ‚ **354,30 s** na 86 testach (99,8% czasu caĹ‚ego pakietu).
  - Koszt wynikaĹ‚ z dwĂłch ĹşrĂłdeĹ‚:
    1. ALGORITHM: 3 testy chunkowania `each_*_chunk_is_at_most_one_mib` z algorytmem $O(N^2)$ trwajÄ…ce Ĺ‚Ä…cznie ponad 557 sekund CPU (254 s + 257 s + 46 s).
    2. CRYPTO_INCIDENTAL: 13 testĂłw weryfikujÄ…cych wyĹ‚Ä…cznie ramkowanie, szyfrowanie pakietĂłw XChaCha20Poly1305, sygnatury AAD i numery sekwencyjne (tampering, replay, missing counter, reorder, wire inspection), ktĂłre niepotrzebnie wywoĹ‚ywaĹ‚y produkcyjny `derive_key` (Argon2id, m=19 MiB, t=2) jedynie po to, by uzyskaÄ‡ losowy 32-bajtowy klucz symetryczny.
- **Implementacja po:**
  - WdroĹĽono algorytm liniowy $O(N)$ dla procedury Catch-Up (PERF-001, commit `7106677`), redukujÄ…c czas testĂłw chunkowania z 557 sekund do 0,57 sekundy (>1800x szybciej).
  - ZastÄ…piono zbÄ™dne, incydentalne wywoĹ‚ania `derive_key` w 13 testach jednostkowych statycznymi kluczami testowymi `[u8; KEY_LEN]`.
  - Zachowano bez zmian produkcyjne parametry kryptograficzne Argon2id: `ARGON2_M_COST_KIB = 19 * 1024` (19 MiB RAM), `ARGON2_T_COST = 2`, `ARGON2_P_COST = 1`, `KEY_LEN = 32`, `SALT_LEN = 16`.
  - Dodano test straĹĽnika nienaruszalnoĹ›ci parametrĂłw produkcyjnych: `production_argon2_parameters_remain_unchanged`.
  - Wszystkie testy kryptografii i uwierzytelniania peera/hosta (`key_derivation_is_deterministic`, `different_salt_yields_different_key`, `handshake_succeeds_with_same_secret`, `handshake_fails_with_wrong_secret_without_reply`, `service_end_to_end_live_and_catchup`, `sync_successful_handshake_is_not_rejected_by_timeout`) nadal wykonujÄ… peĹ‚ny proces z rzeczywistym Argon2id.
- **Pomiary czasu przed i po:**
  - `cargo test sync:: --lib`: $354,30\text{ s} \rightarrow 35,70\text{ s}$ (skrĂłcenie o **318,6 sekundy**, **10x szybciej**, 98 testĂłw passed).
  - `cargo test --lib`: $353,36\text{ s} \rightarrow 37,05\text{ s}$ (skrĂłcenie o **316,3 sekundy**, **9,5x szybciej**, 1010 testĂłw passed).
  - `each_inventory_chunk_is_at_most_one_mib`: $254,09\text{ s} \rightarrow 0,14\text{ s}$ (**1814x szybciej**).
  - `each_request_chunk_is_at_most_one_mib`: $256,84\text{ s} \rightarrow 0,14\text{ s}$ (**1834x szybciej**).
  - `each_records_chunk_is_at_most_one_mib`: $45,85\text{ s} \rightarrow 0,29\text{ s}$ (**158x szybciej**).
- **Ryzyko resztkowe:** Zerowe. Brak jakiegokolwiek osĹ‚abienia kryptografii, 100% testĂłw zielonych. Commit: `d4cd4d2`.

---

### `DOC-001` â€” RozbieĹĽnoĹ›ci dokumentacyjne w rejestrze audytowym i specyfikacjach zawodĂłw (arytmetyka CQ WW, wojewĂłdztwa SP DX, kategoria nadajnika Cabrillo)
- **ID:** `DOC-001`
- **TytuĹ‚:** NiespĂłjnoĹ›ci opisowe i rachunkowe w dokumentacji audytowej domeny zawodĂłw wzglÄ™dem zweryfikowanego kodu i testĂłw
- **Kategoria:** `DOC`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED_SOFTWARE_VERIFIED`
- **Data wdroĹĽenia i weryfikacji:** 2026-11-05 (Etap 7) / 2026-11-06 (Etap 10 â€” peĹ‚na synchronizacja `README.md` i `Native CAT.md`)
- **Commity:**
  - `073b50a`: `docs(audit): close FUNC-001 and FUNC-002 in audit register, roadmap and baseline`
  - `20d6a14`: `docs(audit): close Stage 7 contest compliance repair`
  - `b5b5f99`: `docs(audit): close final Stage 7 contest compliance gate`
  - `35870a3`: `docs(audit): correct CQ WW fixture arithmetic and SP DX province names`
  - Commit dokumentacyjny Etapu 10: `docs(audit): close Stage 10 findings (DOC-001)`
- **ModuĹ‚ / obszar:** `README.md`, `Native CAT.md`, `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/IMPROVEMENT_ROADMAP.md`, `docs/REMEDIATION_BASELINE.md`
- **Lokalizacja:** [`docs/AUDIT_FINDINGS_REGISTER.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/AUDIT_FINDINGS_REGISTER.md), [`docs/IMPROVEMENT_ROADMAP.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/IMPROVEMENT_ROADMAP.md), [`docs/REMEDIATION_BASELINE.md`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/REMEDIATION_BASELINE.md)
- **DowĂłd z kodu i dokumentacji przed korektÄ…:**
  1. Uprzedni opis manualnego fixture CQ WW zawieraĹ‚ niespĂłjnoĹ›Ä‡ rachunkowÄ… (20 pkt QSO sugerowane przy 5 mnoĹĽnikach), podczas gdy poprawny rachunek wynosi Ĺ›ciĹ›le: 10 punktĂłw QSO Ă— 10 mnoĹĽnikĂłw = 100 (CLAIMED-SCORE: 100), a Ĺ‚Ä…cznoĹ›ci zduplikowane wnoszÄ… 0 dodatkowych punktĂłw i 0 nowych mnoĹĽnikĂłw.
  2. Uprzedni opis zawieraĹ‚ bĹ‚Ä™dne nazwy wojewĂłdztw przy kodach R oraz C zamiast kanonicznego przypisania: R = mazowieckie, C = Ĺ‚Ăłdzkie, K = podkarpackie, P = kujawsko-pomorskie.
  3. Zdublowane wzory i artefakty formatowania matematycznego w tekĹ›cie dokumentacji.
- **WdroĹĽone rozwiÄ…zanie (Ostateczna Bramka Dokumentacyjna Etapu 7 â€” DOC-001):**
  1. **Arytmetyka fixture CQ WW**: WdroĹĽono jednoznaczny rachunek: 10 punktĂłw QSO Ă— 10 mnoĹĽnikĂłw = 100 (CLAIMED-SCORE: 100).
  2. **ObsĹ‚uga duplikatĂłw CQ WW**: Zdefiniowano zachowanie duplikatĂłw: pozostajÄ… w logu Cabrillo 3.0 zgodnie ze standardem, dajÄ…c 0 dodatkowych punktĂłw i 0 nowych mnoĹĽnikĂłw (wynik wynosi 100). Duplikaty nie mogÄ… byÄ‡ uĹĽywane do sztucznego uzasadniania 20 punktĂłw QSO.
  3. **Kanoniczna mapa SP DX**: Wprowadzono peĹ‚nÄ… tabelÄ™ 16 wojewĂłdztw: B = lubuskie, C = Ĺ‚Ăłdzkie, D = dolnoĹ›lÄ…skie, F = pomorskie, G = Ĺ›lÄ…skie, J = warmiĹ„sko-mazurskie, K = podkarpackie, L = lubelskie, M = maĹ‚opolskie, O = podlaskie, P = kujawsko-pomorskie, R = mazowieckie, S = Ĺ›wiÄ™tokrzyskie, U = opolskie, W = wielkopolskie, Z = zachodniopomorskie.
  4. **Fixture SP DX**: Opisano manualne fixture stacji zagranicznej: SP1AA (R = mazowieckie, 3 pkt), SP2AA (C = Ĺ‚Ăłdzkie, 3 pkt), DL2AA (001, 0 pkt), SP3AA (R = mazowieckie, 3 pkt); punkty: 3 + 3 + 0 + 3 = 9; mnoĹĽniki: 3 (R na 80m, C na 40m, R na 20m); wynik: 9 Ă— 3 = 27 (CLAIMED-SCORE: 27). Fixture stacji polskiej: SP1AA (wojewĂłdztwo R = mazowieckie).
  5. **Cabrillo 3.0**: Utrzymano jednoznaczne rozdzielenie `CATEGORY-OPERATOR` (`SINGLE-OP`, `MULTI-OP`, `CHECKLOG`) od `CATEGORY-TRANSMITTER` (`ONE`), z wyeliminowaniem bĹ‚Ä™dnej wartoĹ›ci `MULTI`.
  6. **Czyszczenie formatowania**: UsuniÄ™to artefakty i znieksztaĹ‚cenia notacji na rzecz czystych wzorĂłw tekstowych (`3 + 3 + 0 + 3 = 9`, `9 Ă— 3 = 27`, `10 Ă— 10 = 100`).
  7. **SpĂłjnoĹ›Ä‡ audytowa**: PeĹ‚na weryfikacja zgodnoĹ›ci z testami regresyjnymi bez modyfikacji kodu produkcyjnego ani testĂłw.
- **Ryzyko wdroĹĽenia zmiany:** Zerowe (zmiana wyĹ‚Ä…cznie dokumentacyjna).
- **ZĹ‚oĹĽonoĹ›Ä‡ wdroĹĽenia:** `S`
- **ZaleĹĽnoĹ›ci od innych zmian:** PowiÄ…zane z `FUNC-001`, `FUNC-002`.
- **WdroĹĽone testy regresyjne:**
  - `scoreboard_matches_manual_cqww_fixture`
  - `claimed_score_matches_corrected_manual_cqww_fixture`
  - `scoreboard_matches_manual_spdx_fixture`
  - `spdx_accepts_each_official_one_letter_voivodeship`
  - `cabrillo_never_emits_category_transmitter_multi`
  - `tests/security_guard.rs`
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” dokumentacja odpowiada kodowi i testom bez ĹĽadnych niespĂłjnoĹ›ci.

---

### `CI-001` â€” Workflow `.github/workflows/build-and-release.yml` nie generuje ani nie podpisuje `release-manifest.json` wymaganego przez auto-updater
- **ID:** `CI-001`
- **TytuĹ‚:** Brak kroku generowania i podpisywania `release-manifest.json` / `release-manifest.json.sig` w potoku wydawniczym GitHub Actions
- **Kategoria:** `CI`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED / PRODUCTION_DRY_RUN_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-01
- **Commity:**
  - `16892e8`: `ci(release): generate signed manifest and complete checksums (CI-001)`
  - `7125c66`: `ci(release): make native dry-run verification fail-closed (CI-001)`
  - `0f56ada`: `fix(ci): implement manifest signature verification in splogbook-signer (CI-001)`
  - `75ef965`: `ci(actions): migrate workflows to latest stable action runtimes`
- **Migracja GitHub Actions i eliminacja ostrzeĹĽeĹ„ Node.js 20:**
  - `actions/checkout`: z `@v4` (node20) na `@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1` (`runs.using: node24`)
  - `dtolnay/rust-toolchain`: z `@stable` na `@683b588c487a69d01f6c6ddf40f036ff682cc2c8 # v1` (`runs.using: composite`)
  - `actions/upload-artifact`: z `@v4` (node20) na `@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2` (`runs.using: node24`, `if-no-files-found: error`)
  - `actions/download-artifact`: z `@v4` (node20) na `@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2` (`runs.using: node24`)
  - `softprops/action-gh-release`: z `@v2` (node20) na `@efb35369e0ad2afab669f228072c1b0d510eae64 # v3.0.3` (`runs.using: node24`)
  - Liczba akcji Node.js 20: 0
  - OstrzeĹĽenia `Node.js 20 is deprecated`: 0
  - OstrzeĹĽenia `being forced to run on Node.js 24`: 0
  - WĹ‚asne skrypty Node.js: `WORKFLOW_NODE_NOT_REQUIRED` (brak zbÄ™dnego `setup-node`)
- **Weryfikacja rzeczywistego potoku CI dry-run z produkcyjnÄ… Ĺ›cieĹĽkÄ… klucza (Run ID: `41317314042`, ref: `main`, commit: `75ef965`):**
  - **Job Build Windows x64 (`124880000444`):** SUCCESS (28m32s)
  - **Job Build Linux x86_64 (`124880000306`):** SUCCESS (18m34s)
  - **Job Publish Release Assets (`124887050942`):** SUCCESS (8m23s)
  - **Krok Create, sign and verify Release Manifest:** SUCCESS
    - Odczytano sekret podpisywania pod `umask 077` do `priv.key` (`600`)
    - PorĂłwnano fingerprint SHA-256 klucza publicznego przez `splogbook-signer fingerprint`: `2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a` (100% zgodny z zakotwiczonym zaufanym kluczem `splogbook-release-2026-02`)
    - Podpisano manifest `release-manifest.json` do `release-manifest.json.sig`
    - Zweryfikowano podpis produkcyjnym weryfikatorem updatera (`splogbook-signer verify`)
    - UsuniÄ™to materiaĹ‚ klucza prywatnego (`trap 'rm -f priv.key' EXIT`) i potwierdzono brak wycieku do stagingu
  - **Krok Create GitHub Release:** SKIPPED (potwierdzono pominiÄ™cie publikacji w trybie dry-run)
  - **Utworzenie git tag:** BRAK (potwierdzone `git tag -l`)
  - **Utworzenie GitHub Release:** BRAK (potwierdzone `gh release list`)
  - **NiezaleĹĽna walidacja pobranych artefaktĂłw (pobrane przez `gh run download 41317314042`):**
    - IntegralnoĹ›Ä‡ SHA-256 (`SHA256SUMS.txt`): zgodna co do bitu z `SPLogbook-Linux-x86_64.tar.gz` (`fa8bc562716055e34c588d47de486cb3bd235ddcef64ddfc49714af8e9d6394e`) i `SPLogbook-Windows-x64.zip` (`690aca91ee5c9ce8d24863ab8158b4e6e6aacd8f429224e4b55d670f3392c15e`).
    - Manifest `release-manifest.json`: zgodnoĹ›Ä‡ commit SHA (`75ef9651...`), wersji (`1.2.0`), identyfikatora klucza (`splogbook-release-2026-02`), rozmiarĂłw i sum SHA-256.
    - Sygnatura Ed25519 `release-manifest.json.sig`: zweryfikowana pomyĹ›lnie produkcyjnym kluczem.
    - Ochrona sekretĂłw: potwierdzono brak obecnoĹ›ci jakichkolwiek kluczy prywatnych ani poĹ›wiadczeĹ„ w archiwach.
  - **Status gotowoĹ›ci potoku wydawniczego:** **RELEASE_PIPELINE_READY**
- **ModuĹ‚ / obszar:** `.github/workflows/build-and-release.yml`, `src/bin/signer.rs`, `tests/security_guard.rs`, `docs/RELEASE_SECURITY.md`
- **Lokalizacja:** [`.github/workflows/build-and-release.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/build-and-release.yml), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs)
- **DowĂłd z kodu przed naprawÄ…:** Potok wydawniczy nie generowaĹ‚ manifestu `release-manifest.json`, nie podpisywaĹ‚ go narzÄ™dziem `splogbook-signer` i nie doĹ‚Ä…czaĹ‚ podpisu ani kompletnego pliku `SHA256SUMS.txt`. Ponadto brakowaĹ‚o kontroli fail-closed przy braku sekretu `RELEASE_SIGNING_KEY` oraz brakowaĹ‚o ograniczenia uprawnieĹ„ (byĹ‚o `permissions: contents: write` globalnie).
- **WdroĹĽone rozwiÄ…zanie (Etap 2):**
  1. Ograniczenie uprawnieĹ„: `permissions: contents: read` na poziomie nadrzÄ™dnym; `contents: write` wyĹ‚Ä…cznie w jobie `publish-release`.
  2. Dodanie wsparcia dla walidacji w trybie dry-run przez `workflow_dispatch` z `dry_run: true`.
  3. Wymuszenie fail-closed: brak sekretu `RELEASE_SIGNING_KEY` przy prĂłbie publikacji wydania na tagu natychmiast przerywa dziaĹ‚anie potoku z kodem 1.
  4. Sprawdzenie kompletnoĹ›ci i jednoznacznoĹ›ci artefaktĂłw: weryfikacja obecnoĹ›ci i niepustoĹ›ci `SPLogbook-Linux-x86_64.tar.gz` oraz `SPLogbook-Windows-x64.zip`.
  5. Generowanie i auto-weryfikacja `SHA256SUMS.txt` (`sha256sum -c`).
  6. Deterministyczne generowanie `release-manifest.json` z aktywnym kluczem `splogbook-release-2026-02`.
  7. Podpisywanie manifestu przez `splogbook-signer sign` pod rygorem umask `077`, prawami `600` i czyszczeniem klucza prywatnego puĹ‚apkÄ… `trap 'rm -f priv.key' EXIT`.
  8. ObowiÄ…zkowa weryfikacja podpisu (`splogbook-signer verify`) przed wywoĹ‚aniem kroku publikacji.
  9. Bramka `Create GitHub Release` (`softprops/action-gh-release@v2`) blokujÄ…ca publikacjÄ™ w trybie `dry_run: true` oraz przy braku tagu `v*`.
- **WdroĹĽone testy regresyjne:**
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

### `CI-002` â€” Brak flagi `-D warnings` dla `cargo clippy` oraz brak `cargo audit` w `.github/workflows/ci.yml`
- **ID:** `CI-002`
- **TytuĹ‚:** Zbyt permisywna konfiguracja `.github/workflows/ci.yml` przepuszczajÄ…ca ostrzeĹĽenia Clippy i podatnoĹ›ci RustSec
- **Kategoria:** `CI`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** **FIXED_SOFTWARE_VERIFIED**
- **Data wdroĹĽenia i weryfikacji:** 2026-11-06
- **Commit:**
  - `7466bdd`: `ci(quality): enforce Clippy deny warnings and dependency audit (CI-002)`
- **ModuĹ‚ / obszar:** `.github/workflows/ci.yml`, `tests/security_guard.rs`
- **Lokalizacja:** [`.github/workflows/ci.yml`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/.github/workflows/ci.yml), [`tests/security_guard.rs`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/tests/security_guard.rs)
- **DowĂłd z kodu przed naprawÄ…:** W `.github/workflows/ci.yml:52` krok `Run Clippy linter` wykonywaĹ‚ `cargo clippy --all-targets --all-features` bez `-- -D warnings`, przez co nie przerywaĹ‚ buildu przy ostrzeĹĽeniach. BrakowaĹ‚o rĂłwnieĹĽ krokĂłw `cargo test --doc` oraz `cargo audit`.
- **WdroĹĽone rozwiÄ…zanie (Etap 10):**
  1. W `.github/workflows/ci.yml` zaktualizowano krok Clippy na `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  2. Ujednolicono krok testĂłw do `cargo test --workspace --all-targets --verbose` oraz dodano krok `cargo test --doc --verbose`.
  3. Dodano obowiÄ…zkowy krok `Run cargo audit` instalujÄ…cy `cargo-audit` i uruchamiajÄ…cy `cargo audit --deny warnings` z jawnÄ…, udokumentowanÄ… listÄ… 7 zaakceptowanych tranzytywnych advisories (`RUSTSEC-2024-0436`, `RUSTSEC-2025-0141`, `RUSTSEC-2026-0249`, `RUSTSEC-2026-0370`, `RUSTSEC-2026-0371`, `RUSTSEC-2026-0372`, `RUSTSEC-2026-0097`).
  4. Zachowano brak `continue-on-error: true` na wszystkich bramkach jakoĹ›ciowych i bezpieczeĹ„stwowych oraz przypiÄ™cie akcji GitHub Actions do peĹ‚nych 40-znakowych SHA.
- **WdroĹĽone testy regresyjne:**
  - `ci_workflow_enforces_fmt_check`
  - `ci_workflow_enforces_clippy_deny_warnings`
  - `ci_workflow_runs_workspace_tests`
  - `ci_workflow_runs_cargo_audit`
  - `ci_workflow_does_not_allow_quality_gate_failures`
  - `ci_workflow_does_not_run_physical_hil_tests`
  - `ci_workflow_pins_actions_or_follows_repo_security_policy`
- **Kryterium akceptacji:** SpeĹ‚nione w 100% â€” `ci.yml` wymusza zero ostrzeĹĽeĹ„ Clippy i zero podatnoĹ›ci RustSec.

---

### `DEP-001` â€” 7 podatnoĹ›ci bezpieczeĹ„stwa (`RUSTSEC-2026-*`) oraz 1 ostrzeĹĽenie `unsound` (`tokio 1.53.1`) w `Cargo.lock`
- **ID:** `DEP-001`
- **TytuĹ‚:** PodatnoĹ›ci w `rustls 0.23.45`, `rustls-webpki 0.103.15`, `webpki-root-certs 1.0.9`, `ctutils 0.4.2` i `tokio 1.53.1` wykryte przez `cargo audit`
- **Kategoria:** `DEP`
- **Poziom krytycznoĹ›ci:** `HIGH`
- **Poziom pewnoĹ›ci:** `CONFIRMED`
- **Status remediacji:** `FIXED`
- **Data wdroĹĽenia:** 2026-11-01T08:41:47+01:00 (UTC: 2026-11-01T07:41:47Z)
- **Commit:** `335ced5` (`chore(deps): update vulnerable dependencies (DEP-001)`)
- **ModuĹ‚ / obszar:** `Cargo.lock`, `Cargo.toml`
- **Lokalizacja:** [`Cargo.lock`](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/Cargo.lock)
- **Baza RustSec:** oficjalne repozytorium `https://github.com/RustSec/advisory-db.git`, commit `2295466c4e9df26647fd67d8fa1c705ac2fc7f81` (Fri Oct 30 12:12:39 2026 +0100).
- **Bramka chronologii RustSec:** `ENVIRONMENT_DATE_AFTER_REFERENCE` (czas Ĺ›rodowiska wykonawczego 2026-11-01 jest pĂłĹşniejszy niĹĽ data odniesienia 2026-10-12; wszystkie advisory `RUSTSEC-2026-*` zostaĹ‚y opublikowane w bazie miÄ™dzy 2026-10-13 a 2026-10-24 przed badaniem).
- **Stan przed aktualizacjÄ… (`b0d1b24`):**
  1. `ctutils 0.4.2`: `RUSTSEC-2026-0385` (vulnerability, data: 2026-10-23, commit bazy `f7b2f6b`, patched: `>= 0.4.4`).
  2. `rustls 0.23.45`: `RUSTSEC-2026-0346`, `RUSTSEC-2026-0347`, `RUSTSEC-2026-0348`, `RUSTSEC-2026-0349` (4 vulnerabilities, data: 2026-10-14, commit bazy `caee11f`, patched: `>= 0.23.46`).
  3. `rustls-webpki 0.103.15`: `RUSTSEC-2026-0382` (vulnerability, data: 2026-10-20, commit bazy `e6f94f3`, patched: `>= 0.103.16`).
  4. `webpki-root-certs 1.0.9`: `RUSTSEC-2026-0383` (vulnerability, data: 2026-10-19, commit bazy `4b587f0`, patched: `>= 1.0.10`).
  5. `tokio 1.53.1`: `RUSTSEC-2026-0386` (unsound, data: 2026-10-13, commit bazy `84d943a`, patched: `>= 1.53.3`).
  6. Wycofane pakiety (`yanked`): `clipboard-win 5.4.1`, `yoke-derive 0.8.3`.
  - ĹÄ…cznie: 7 vulnerabilities, 9 warnings (1 unsound, 2 yanked, 6 unmaintained).
- **Wykonana selektywna aktualizacja (`Cargo.lock`):**
  - Grupa 1 (ctutils): `ctutils` 0.4.2 -> 0.4.7
  - Grupa 2 (TLS): `rustls` 0.23.45 -> 0.23.46, `rustls-webpki` 0.103.15 -> 0.103.16, `webpki-root-certs` 1.0.9 -> 1.0.11
  - Grupa 3 (Tokio): `tokio` 1.53.1 -> 1.53.3
  - Grupa 4 (Yanked): `clipboard-win` 5.4.1 -> 5.4.3, `yoke-derive` 0.8.3 -> 0.8.4
  - ZaleĹĽnoĹ›ci poĹ›rednie dostosowane przez solver Cargo: `windows-sys` 0.60.2 / 0.52.0, `getrandom` 0.3.4 (dla tempfile).
  - Brak modyfikacji `Cargo.toml`, brak podniesienia MSRV, brak zmian major, brak zmian cech (`features`).
- **Stan po aktualizacji (`335ced5`):**
  - Vulnerabilities: 0
  - Unsound: 0
  - Yanked: 0
  - Nowe advisory: 0
  - Unmaintained warnings: 6 (wszystkie sklasyfikowane i udokumentowane).
- **Klasyfikacja i Ĺ›cieĹĽki pakietĂłw unmaintained (`OPEN_UNMAINTAINED_TRANSITIVE`):**
  1. `bincode 1.3.3` (`RUSTSEC-2025-0141`): `splogbook` -> `printpdf 0.12.8` -> `azul-layout 0.0.16` -> `hyphenation 0.8.4` -> `bincode 1.3.3`.
  2. `paste 1.0.15` (`RUSTSEC-2024-0436`): `splogbook` -> `egui_dock 0.21.1` -> `paste 1.0.15`.
  3. `smartstring 1.0.1` (`RUSTSEC-2026-0249`): `splogbook` -> `rhai 1.26.1` -> `smartstring 1.0.1`.
  4. `zbus_names 4.3.4` (`RUSTSEC-2026-0371`): `splogbook` -> `keyring 4.2.0` -> `zbus-secret-service-keyring-store 1.0.1` -> `secret-service 5.2.0` -> `zbus 5.19.0` -> `zbus_names 4.3.4`.
  5. `zvariant 5.15.0` (`RUSTSEC-2026-0370`): `splogbook` -> `keyring 4.2.0` -> `zbus-secret-service-keyring-store 1.0.1` -> `secret-service 5.2.0` -> `zbus 5.19.0` -> `zvariant 5.15.0`.
  6. `zvariant_derive 5.15.0` (`RUSTSEC-2026-0372`): `splogbook` -> `keyring 4.2.0` -> `zbus-secret-service-keyring-store 1.0.1` -> `secret-service 5.2.0` -> `zbus 5.19.0` -> `zvariant_derive 5.15.0`.
  - **Ryzyko resztkowe:** Ĺ»aden z pakietĂłw nie posiada otwartej luki CVE ani bĹ‚Ä™du bezpieczeĹ„stwa; sÄ… to pakiety tranzytywne archiwizowane przez autorĂłw. Ich migracja wymaga przejĹ›cia nadrzÄ™dnych bibliotek na nowsze API (np. zbus 6.0, compact_str) w kolejnych etapach refaktoryzacyjnych.
- **Wyniki testĂłw regresyjnych:**
  - Kryptografia: testy `core::crypto`, `sync::crypto`, `cloud::updater`, `splogbook-signer` oraz 19/19 testĂłw `tests/security_guard.rs` zielone.
  - Komunikacja sieciowa i TLS: testy moduĹ‚Ăłw `cloud::`, `core::http`, `api::server`, `sync::` zielone.
  - Runtime Tokio i SEC-003: 3 dedykowane testy timeoutu LAN Sync (`sync_host_handshake_timeout_releases_slot`, `sync_partial_handshake_times_out`, `sync_successful_handshake_is_not_rejected_by_timeout`) wykonane po 3 razy (3/3 sukcesĂłw, 0 opĂłĹşnieĹ„ wall-clock, deterministyczny czas wirtualny Tokio).
  - Target Windows MSVC: `cargo check --workspace --all-targets --all-features --target x86_64-pc-windows-msvc` zakoĹ„czony sukcesem (kod 0).
  - Clippy: zero nowych ostrzeĹĽeĹ„ we wszystkich celach (`--lib`, `--lib --tests`, `--bin SPLogbook`, `--bin splogbook-signer`).
- **Kryterium akceptacji:** `cargo audit` zgĹ‚asza dokĹ‚adnie 0 podatnoĹ›ci `vulnerability`, 0 `unsound`, 0 `yanked`.
