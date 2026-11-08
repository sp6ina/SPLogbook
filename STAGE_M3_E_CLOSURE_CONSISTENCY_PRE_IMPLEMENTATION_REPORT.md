# Raport przedimplementacyjny bramki STAGE-M3-E-CLOSURE-CONSISTENCY-GATE

**Data sporzÄ…dzenia:** 2026-11-08
**Autor:** GĹ‚Ăłwny Audytor Techniczny SPLogbook & Starszy InĹĽynier Systemowy
**Status bramki:** IN_PROGRESS (PRE_IMPLEMENTATION_VERIFIED)
**Cel:** KoĹ„cowa weryfikacja spĂłjnoĹ›ci taga `v1.2.0`, dokĹ‚adnych licznikĂłw testĂłw HIL oraz polityki wersjonowania przyszĹ‚ych pakietĂłw MSI prerelease

---

## 1. Data i Ĺ›rodowisko

- **Data robocza projektu:** 2026-11-08
- **Data Ĺ›rodowiska:** 2026-11-08
- **Czas Ĺ›rodowiska:** 15:26:35
- **Timezone:** Central European Standard Time (Warszawa)
- **UTC Offset:** +01:00

---

## 2. Git

- **Repozytorium:** `C:\Users\sp6in\Documents\SPLogbook\SPLogbook`
- **GaĹ‚Ä…Ĺş bieĹĽÄ…ca:** `repair/full-system-remediation`
- **HEAD:** `0ff72f37f532fc6258dff28755a8b9be8f31365d`
- **main:** `0ff72f37f532fc6258dff28755a8b9be8f31365d`
- **origin/main:** `0ff72f37f532fc6258dff28755a8b9be8f31365d`
- **repair/full-system-remediation:** `0ff72f37f532fc6258dff28755a8b9be8f31365d`
- **Merge-base (main, origin/main):** `0ff72f37f532fc6258dff28755a8b9be8f31365d`
- **Ahead / Behind:** 0 / 0
- **Working Tree:** CLEAN (`git status --porcelain=v1` zwraca pusty wynik)
- **Aktywne operacje Git:** BRAK (`MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD` nie istniejÄ…)
- **Klasyfikacja bazowa:** `EXACT_REPORTED_BASELINE` (ĹšcieĹĽka A)

---

## 3. Tag v1.2.0

PeĹ‚na inspekcja referencji Git potwierdziĹ‚a:

- **Local Tag Reference SHA (`refs/tags/v1.2.0`):** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **Local Tag Object Type:** `tag` (obiekt anotowanego taga)
- **Local Tag Object SHA:** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **Local Peeled Commit SHA (`refs/tags/v1.2.0^{}`):** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
- **Remote Tag Reference SHA (`origin` `refs/tags/v1.2.0`):** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **Remote Peeled Commit SHA (`origin` `refs/tags/v1.2.0^{}`):** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
- **Typ taga:** `ANNOTATED`
- **Tagger:** `sp6ina <sp6ina@wp.pl>`
- **Data taga:** `Mon Oct 26 12:18:38 2026 +0100` (Unix timestamp: `1793013518 +0100`)
- **Komunikat taga:** `SPLogbook v1.2.0`
- **ZgodnoĹ›Ä‡ obiektu taga lokalny/zdalny:** `YES`
- **ZgodnoĹ›Ä‡ peeled commit lokalny/zdalny:** `YES`
- **Czy tag zostaĹ‚ przesuniÄ™ty (Tag moved):** `NO`

---

## 4. Dwa raportowane SHA

PorĂłwnanie i klasyfikacja wartoĹ›ci:
1. `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`:
   - Typ obiektu Git: `commit`
   - ToĹĽsamoĹ›Ä‡: Commit wydania `v1.2.0` ("docs(readme): update version 1.2.0 metadata, test counts, and module tree", data: 2026-10-26 12:18:32 +0100)
   - Rola: Peeled commit wskazywany bezpoĹ›rednio przez obiekt anotowanego taga `v1.2.0`
2. `3ea8d51aab436805107a1cf422e0b35cc301458f`:
   - Typ obiektu Git: `tag`
   - ToĹĽsamoĹ›Ä‡: Obiekt anotowanego taga `v1.2.0` w bazie obiektĂłw Git (`.git/objects/3e/a8d5...`)
   - Rola: BezpoĹ›rednia referencja `refs/tags/v1.2.0`

**Klasyfikacja rozbieĹĽnoĹ›ci:** `A. TAG_OBJECT_VS_PEELED_COMMIT`
Obie wartoĹ›ci sÄ… w 100% poprawne, autentyczne i niezmienne od momentu utworzenia wydania stabilnego `v1.2.0`. WczeĹ›niejsze raporty stosowaĹ‚y zamiennie pojÄ™cie "tag v1.2.0", wskazujÄ…c raz obiekt taga, a raz wskazywany peeled commit. Wymagana jest wyĹ‚Ä…cznie precyzyjna terminologia dokumentacyjna bez modyfikacji repozytorium.

---

## 5. Test inventory

Kanoniczna inwentaryzacja targetĂłw testowych wykonana poleceniem `cargo +stable test --workspace --all-targets -j 2 -- --list` wykazaĹ‚a dokĹ‚adnie 15 targetĂłw w przestrzeni roboczej:

1. `src/lib.rs` (unit tests): 1875 zarejestrowanych
2. `src/main.rs` (SPLogbook bin): 0 zarejestrowanych
3. `src/bin/signer.rs` (splogbook-signer bin): 0 zarejestrowanych
4. `src/bin/splogbook-updater.rs` (splogbook-updater bin): 5 zarejestrowanych
5. `tests/cat_native_6a.rs` (integration): 165 zarejestrowanych
6. `tests/cat_native_6b.rs` (integration): 99 zarejestrowanych
7. `tests/cat_native_7.rs` (integration): 307 zarejestrowanych (w tym 10 ignored HIL)
8. `tests/i18n_guard.rs` (integration): 15 zarejestrowanych
9. `tests/inno_user_data_safety_test.rs` (integration): 7 zarejestrowanych
10. `tests/msi_user_data_safety_test.rs` (integration): 6 zarejestrowanych
11. `tests/security_guard.rs` (integration): 118 zarejestrowanych
12. `tests/theme_guard.rs` (integration): 1 zarejestrowany
13. `tests/windows_helper_test.rs` (integration): 33 zarejestrowanych
14. `tests/windows_inno_setup_test.rs` (integration): 14 zarejestrowanych
15. `tests/windows_msi_test.rs` (integration): 18 zarejestrowanych

- **TOTAL REGISTERED TESTS:** 2663 (w tym 10 ignored)
- **TOTAL EXECUTED TESTS:** 2653
- **TOTAL PASSED TESTS:** 2653
- **TOTAL FAILED TESTS:** 0
- **TOTAL IGNORED TESTS:** 10
- **DOC TESTS REGISTERED / EXECUTED / PASSED:** 0 / 0 / 0

---

## 6. Ignored / HIL

Wszystkie 10 testĂłw `ignored` w repozytorium to hermetycznie odizolowane testy sprzÄ™towe HIL zlokalizowane w pliku `tests/cat_native_7.rs`:
1. `hil_hardware_read_01` (linia 2486) â€” `#[ignore = "hardware-in-the-loop"]`
2. `hil_hardware_freq_01` (linia 2498) â€” `#[ignore = "hardware-in-the-loop"]`
3. `hil_hardware_mode_01` (linia 2510) â€” `#[ignore = "hardware-in-the-loop"]`
4. `hil_hardware_ptt_01` (linia 2522) â€” `#[ignore = "hardware-in-the-loop"]`
5. `hil_hardware_ptt_timeout_01` (linia 2536) â€” `#[ignore = "hardware-in-the-loop"]`
6. `hil_hardware_resync_01` (linia 2550) â€” `#[ignore = "hardware-in-the-loop"]`
7. `hil_hardware_multi_radio_01` (linia 2562) â€” `#[ignore = "hardware-in-the-loop"]`
8. `hil_hardware_shutdown_01` (linia 2574) â€” `#[ignore = "hardware-in-the-loop"]`
9. `hil_hardware_shutdown_tx_01` (linia 2586) â€” `#[ignore = "hardware-in-the-loop"]`
10. `hil_hardware_live_test_bench` (linia 2601) â€” `#[ignore = "hardware-in-the-loop"]`

- **HIL REGISTERED:** 10
- **HIL EXECUTED:** 0
- **NON-HIL IGNORED:** 0
- **PHYSICAL HIL:** NOT EXECUTED
- **PHYSICAL PTT:** NOT EXECUTED
- **RF EMISSION:** 0

**WyjaĹ›nienie rozbieĹĽnoĹ›ci 0 vs 10 ignored:**
W raporcie `STAGE_M3_E_WINDOWS_MSI_WIX_INTEGRATION_REPORT.md` sekcja 9 wyszczegĂłlniĹ‚a wyĹ‚Ä…cznie targety bezpoĹ›rednio weryfikujÄ…ce pakiet MSI (`security_guard`: 118, `windows_msi_test`: 18, `msi_user_data_safety_test`: 6), w ktĂłrych liczba ignored wynosi 0. W promptach uĹĽyto skrĂłtowego okreĹ›lenia `2600+ passed; 0 ignored`, ktĂłre byĹ‚o bĹ‚Ä™dem agregacji zakresu (Klasyfikacja D: DIFFERENT_COMMAND_SCOPE / A: REPORT_AGGREGATION_ERROR). W repozytorium ĹĽaden test HIL nie zostaĹ‚ usuniÄ™ty, od-ignorowany ani uruchomiony.

---

## 7. MSI version policy

- **SemVer Display Version:** `1.3.0-alpha.1`
- **MSI Product Version:** `1.3.0`
- **Product Code Policy:** Dynamiczny GUID na kompilacjÄ™ (`*`)
- **Package Code Policy:** Unikalny per build
- **Upgrade Code:** `{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}`
- **Allow Same Version Upgrades:** `no` (domyĹ›lne w WiX `<MajorUpgrade>`)
- **Major Upgrade Schedule:** `afterInstallInitialize`

**Ocena przejĹ›Ä‡ przyszĹ‚ych wydaĹ„:**
- BieĹĽÄ…cy `1.3.0-alpha.1` instaluje siÄ™ poprawnie i dokonuje czystego major upgrade ze stabilnego `1.2.0` (`1.2.0 < 1.3.0`).
- PrzejĹ›cia `1.3.0-alpha.1 -> 1.3.0-alpha.2 -> 1.3.0-beta.1 -> 1.3.0-rc.1 -> 1.3.0` przy obecnym mapowaniu `semver_to_msi_product_version` dajÄ… ten sam MSI ProductVersion `1.3.0`.
- PoniewaĹĽ `AllowSameVersionUpgrades="no"`, Windows Installer blokuje takÄ… instalacjÄ™ bĹ‚Ä™dem 1638 (same-version conflict).
- WĹ‚Ä…czenie `AllowSameVersionUpgrades="yes"` zniszczyĹ‚oby ochronÄ™ przed downgrade miÄ™dzy prerelease i dopuĹ›ciĹ‚oby nadpisanie stabilnego `1.3.0` przez `alpha.1`.
- **Klasyfikacja polityki:** `B. CURRENT_ALPHA_ONLY_SUPPORTED` oraz `D. POLICY_INCOMPLETE`. Wymagana jest formalna propozycja decyzji wĹ‚aĹ›ciciela architektury przed wydaniem `1.3.0-alpha.2`.

---

## 8. Expected documentation corrections

1. Dodanie erraty do `STAGE_M3_E_WINDOWS_MSI_WIX_INTEGRATION_REPORT.md` w zakresie:
   - toĹĽsamoĹ›ci taga `v1.2.0` (rozrĂłĹĽnienie tag object `3ea8d51a...` i peeled commit `6f7d3aaa...`);
   - kanonicznych, dokĹ‚adnych licznikĂłw testĂłw (2663 registered, 2653 executed, 2653 passed, 0 failed, 10 ignored HIL);
   - wyjaĹ›nienia relacji ignored w targetach MSI vs caĹ‚ym workspace;
   - analizy ograniczeĹ„ mapowania MSI ProductVersion dla przyszĹ‚ych wydaĹ„ prerelease.
2. Utworzenie propozycji decyzji architektonicznej: `STAGE_M3_E_MSI_PRERELEASE_VERSIONING_DECISION_PROPOSAL.md`.
3. Utworzenie raportu koĹ„cowego bramki: `STAGE_M3_E_CLOSURE_CONSISTENCY_REPORT.md`.
4. Aktualizacja `MASTER_IMPLEMENTATION_PLAN.md` odnotowujÄ…ca przeprowadzenie weryfikacji spĂłjnoĹ›ci zamykajÄ…cej etap M3-E.

---

## 9. Files allowed to change

Dozwolone sÄ… wyĹ‚Ä…cznie pliki dokumentacyjne:
- `STAGE_M3_E_CLOSURE_CONSISTENCY_PRE_IMPLEMENTATION_REPORT.md` (niniejszy plik)
- `STAGE_M3_E_MSI_PRERELEASE_VERSIONING_DECISION_PROPOSAL.md`
- `STAGE_M3_E_CLOSURE_CONSISTENCY_REPORT.md`
- `STAGE_M3_E_WINDOWS_MSI_WIX_INTEGRATION_REPORT.md` (errata)
- `MASTER_IMPLEMENTATION_PLAN.md`

BezwzglÄ™dny zakaz zmian w:
- `src/**`
- `tests/**` (poza ewentualnym read-only modelem testowym polityki, jeĹ›li zatwierdzony)
- `installer/**`
- `scripts/**`
- `Cargo.toml`, `Cargo.lock`
- Git tags (brak operacji `git tag`, `git push --tags`)

---

## 10. Stop conditions

Zatrzymaj natychmiast procedurÄ™, jeĹĽeli:
1. Lokalny i zdalny tag `v1.2.0` rĂłĹĽniÄ… siÄ™ (zweryfikowano: identyczne).
2. Peeled commit lokalny i zdalny rĂłĹĽniÄ… siÄ™ (zweryfikowano: identyczne).
3. Wykryto przesuniÄ™cie taga (zweryfikowano: brak).
4. Historia Git rozbiegĹ‚a siÄ™ z `origin/main` (zweryfikowano: 0/0).
5. PeĹ‚ny przebieg testĂłw ma `failed > 0` (zweryfikowano: 0 failed).
6. Uruchomiono testy HIL lub PTT (zweryfikowano: 0 HIL executed, 0 RF).
7. Konieczna byĹ‚aby zmiana kodu produkcyjnego WiX/MSI/Rust.
