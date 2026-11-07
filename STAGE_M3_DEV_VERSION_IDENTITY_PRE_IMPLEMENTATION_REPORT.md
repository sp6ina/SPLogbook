# RAPORT PRZEDIMPLEMENTACYJNY STAGE-M3-DEV-VERSION-IDENTITY-GATE

**Bramka:** STAGE-M3-DEV-VERSION-IDENTITY-GATE  
**Kanoniczny tytuł:** Wprowadzenie jednoznacznej wersji rozwojowej SPLogbook 1.3.0-alpha.1 oraz spójnej tożsamości kompilacji dla dalszych testów instalacji i aktualizacji  
**Priorytet:** P0 — KRYTYCZNY  
**Rola:** Główny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Semantic Versioning, Cargo, Tożsamości Kompilacji, Bezpiecznych Aktualizatorów i Zarządzania Cyklem Wydawniczym  

---

## 1. Data, Czas i Środowisko Robocze

- **Data robocza projektu:** 2026-11-07
- **Rzeczywista data i czas odczytany ze środowiska:** `2026-11-07 21:19:05 +01:00`
- **Strefa czasowa / UTC offset:** CET (`+01:00`)
- **System operacyjny:** Windows 11 x86_64
- **Rust Toolchain:** `rustc 1.85.0+ / stable-x86_64-pc-windows-msvc` (Rust Edition 2024, MSRV 1.99.0)
- **Integralność zegara:** Brak przesunięć zegara systemowego; brak modyfikacji dat commitów historycznych.

---

## 2. Stan Repozytorium Git i Weryfikacja Bazowa

- **Zgłoszony HEAD z poprzedniego etapu (M3-C):** `7f66b5222c4acbb82322ef25f59bca0cb6dd2e45`
- **Rzeczywisty lokalny HEAD:** `7f66b5222c4acbb82322ef25f59bca0cb6dd2e45`
- **Stan gałęzi roboczych po `git fetch --prune origin`:**
  - `main`: `7f66b5222c4acbb82322ef25f59bca0cb6dd2e45`
  - `repair/full-system-remediation`: `7f66b5222c4acbb82322ef25f59bca0cb6dd2e45`
  - `origin/main`: `7f66b5222c4acbb82322ef25f59bca0cb6dd2e45`
  - `ahead / behind`: `0 / 0`
  - `merge-base`: `7f66b5222c4acbb82322ef25f59bca0cb6dd2e45`
- **Stan operacji Git:** Brak aktywnych `MERGE_HEAD`, `REBASE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`.
- **Working Tree:** Czysty (`git status --porcelain=v1` = pusty).
- **Klasyfikacja Git:** `EXACT_BASELINE` (HEAD == main == origin/main == repair/full-system-remediation).

---

## 3. Ochrona i Weryfikacja Publicznego Wydania v1.2.0

- **Lokalny obiekt taga `v1.2.0`:** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **Zdalny obiekt taga `refs/tags/v1.2.0` na origin:** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **Peeled commit taga `v1.2.0`:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` (identyczny lokalnie i zdalnie)
- **Status ochrony v1.2.0:** Tag i release `v1.2.0` pozostają nienaruszone. Bramka nie modyfikuje, nie usuwa, nie przesuwa taga `v1.2.0` ani nie publikuje żadnego wydania na platformie GitHub.

---

## 4. Stan Plików Cargo Przed Zmianami

- **`Cargo.toml` SHA-256:** `742102A2ADAD3F372912D4E1AEDEB7F20BA046199635C9B4D847E945996CA6CE` (rozmiar: 2462 bajty, EOL: LF)
- **`Cargo.lock` SHA-256:** `6792AEF554899E588EF0DBFAAD0F0E3741AAD3F9434E832D220DAB27E82D557A` (rozmiar: 179792 bajty, EOL: CRLF)
- **`cargo pkgid`:** `path+file:///C:/Users/sp6in/Documents/SPLogbook/SPLogbook#splogbook@1.2.0`
- **`cargo metadata` version:** `1.2.0`
- **Istniejąca zależność `semver`:** `semver = "1.0.28"` (potwierdzona obecność w `Cargo.toml:68`). Brak potrzeby dodawania nowych zewnętrznych zależności.
- **Wyniki pełnego baseline:**
  - `cargo +stable fmt --all -- --check`: ZALICZONY (0 błędów formatowania)
  - `cargo +stable check --workspace --all-targets --all-features -j 2`: ZALICZONY
  - `cargo +stable clippy --workspace --all-targets --all-features -j 2 -- -D warnings`: ZALICZONY (0 ostrzeżeń)
  - `cargo +stable test --workspace --all-targets -j 2`: ZALICZONY (1855 testów jednostkowych, 91 testów security_guard, 28 testów windows_helper, testy CAT/i18n/theme — 0 niepowodzeń)
  - `cargo +stable test --doc -j 2`: ZALICZONY
  - `cargo +stable check --bin SPLogbook` / `cargo +stable check --bin splogbook-updater`: ZALICZONY
  - `cargo +stable test --bin splogbook-updater`: ZALICZONY (5 testów)
  - `cargo audit`: ZALICZONY (0 podatności, 7 dozwolonych ostrzeżeń unmaintained/yanked identycznych z baseline)
  - `git diff --check`: Czysty

---

## 5. Inwentaryzacja i Klasyfikacja Wystąpień Wersji 1.2.0

Zgodnie z wymaganiami Bramki (Sekcja 7 i 10), przeprowadzono inwentaryzację każdego wystąpienia ciągu `1.2.0` w całym repozytorium. Każde wystąpienie zostało jednoznacznie zaklasyfikowane:

| Ścieżka i linia | Kontekst | Klasyfikacja | Działanie w ramach bramki |
|---|---|---|---|
| `Cargo.toml:3` | `version = "1.2.0"` | **aktualna wersja** | Zmiana na `1.3.0-alpha.1` |
| `Cargo.lock:1180, 4686, 4894, 5049` | `version = "1.2.0"` dla pakietu `splogbook` | **aktualna wersja** | Automatyczna aktualizacja przez Cargo wyłącznie dla lokalnego pakietu `splogbook` |
| `src/bin/splogbook-updater.rs:116` | `println!("splogbook-updater 1.2.0 (Windows Helper M3-B)")` | **stale reference** | Powiązanie z centralnym `BuildIdentity` / `env!("CARGO_PKG_VERSION")` |
| `src/cloud/updater.rs:1131` | `min_version: Some("1.2.0")` | **fixture** | Pozostawienie bez zmian (weryfikacja kompatybilności wstecznej manifestu) |
| `src/cloud/updater.rs:4112` | `assert!(verify_downgrade_protection("1.2.0", "1.1.0").is_ok())` | **źródłowa wersja testu** | Pozostawienie bez zmian (test ochrony przed downgrade wersji starszej od 1.2.0) |
| `src/cloud/updater.rs:4211` | `"version": "1.2.0"` w fiksturze manifestu | **fixture** | Pozostawienie bez zmian (test parsowania manifestu) |
| `src/cloud/updater.rs:5474, 5484, 5596, 5604, 5619, 5631` | `b"OLD CANONICAL VERSION 1.2.0"` | **fixture** | Pozostawienie bez zmian (test atomowego zastępowania binarki starej wersji) |
| `src/cloud/updater.rs:6240, 6424, 6504` | `"1.2.0"` jako wersja zainstalowana | **źródłowa wersja testu** | Pozostawienie jako źródłowa wersja testowa aktualizacji do 1.3.0-alpha.1 |
| `src/cloud/updater.rs:6354, 6377` | `"version": "1.2.0"` manifestu z odwołanym kluczem | **historyczne wydanie / fixture** | Pozostawienie bez zmian (krytyczny test odrzucenia klucza `splogbook-release-2026-01`) |
| `src/cloud/updater/health.rs:287` | `"1.2.0"` jako błędna/starsza wersja w teście | **źródłowa wersja testu** | Pozostawienie bez zmian (weryfikacja odrzucenia niepoprawnej wersji w `HealthProof`) |
| `tests/security_guard.rs:252` | `min_version: Some("1.2.0")` | **fixture** | Pozostawienie bez zmian |
| `tests/security_guard.rs:1289, 1328, 1875` | `"version": "1.2.0"` | **historyczne wydanie / fixture** | Pozostawienie bez zmian |
| `tests/windows_helper_test.rs:281, 483, 516` | `b"SPLogbook v1.2.0 stare"` | **fixture** | Pozostawienie bez zmian |
| `README.md:13` | Badge wersji | **dokumentowana stabilna wersja** | Aktualizacja: wskazanie publicznej stabilnej 1.2.0 oraz rozwojowej 1.3.0-alpha.1 |
| `README.md:483` | Przykładowy payload API | **dokumentowana stabilna wersja** | Zachowanie jako przykład kontraktu API |
| `DESIGN_3_0_*.md` | Odwołania do v1.2.0 | **dokumentowana stabilna wersja / historyczne wydanie** | Pozostawienie w dokumentacji projektowej architektury |
| `STAGE_*.md`, `UPDATE_*.md` | Raporty historyczne z zakończonych etapów | **historyczne wydanie** | Pozostawienie bez modyfikacji wstecznej (integralność audytowa) |
| `docs/HISTORICAL_RELEASE_AUDIT.md` | Audyt wydania v1.2.0 | **historyczne wydanie** | Pozostawienie bez zmian |
| `docs/SECURITY_KEY_ROTATION.md` | Audyt rotacji kluczy v1.2.0 | **historyczne wydanie** | Pozostawienie bez zmian |
| `docs/IMPROVEMENT_ROADMAP.md` | Nagłówek i rejestr stanu | **dokumentowana stabilna wersja** | Aktualizacja statusu wersji |
| `docs/REMEDIATION_BASELINE.md` | Stan odniesienia | **dokumentowana stabilna wersja** | Aktualizacja statusu wersji |
| `docs/AUDIT_FINDINGS_REGISTER.md` | Rejestr ustaleń | **dokumentowana stabilna wersja** | Aktualizacja |

**Zasada:** W repozytorium **nie zostanie wykonany żaden automatyczny global search-and-replace**. Modyfikowane są wyłącznie pozycje oznaczone jako `aktualna wersja`, `stale reference` oraz dedykowane rejestry dokumentacji.

---

## 6. Architektura Centralnego BuildIdentity

W projekcie brakuje obecnie jednego zunifikowanego modelu tożsamości kompilacji, przez co poszczególne moduły sięgały bezpośrednio po `env!("CARGO_PKG_VERSION")`, a helper w `src/bin/splogbook-updater.rs` posiadał zakodowany ciąg tekstowy.

W ramach bramki zostanie zaimplementowany moduł `src/core/build_info.rs` udostępniający:

1. **Strukturę `BuildIdentity`:**
   - `package_name: &'static str` (`splogbook`)
   - `package_version: &'static str` (`env!("CARGO_PKG_VERSION")`)
   - `semver: semver::Version`
   - `git_commit_hash: &'static str` (pełny SHA-1 commita z `build.rs` lub `unknown`)
   - `git_commit_short: &'static str` (skrócony SHA-1 commita lub `unknown`)
   - `git_dirty: bool` (czy drzewo robocze posiadało niescommitowane zmiany w chwili kompilacji)
   - `channel: UpdateChannel` (`Alpha`)
   - `distribution_status: DistributionStatus` (`Development`)
   - `profile: BuildProfile` (`Debug`, `Release`, `Test`, `Unknown`)
   - `target_os: &'static str` (wartość `target_os` przekazana z rustc)
   - `target_arch: &'static str` (wartość `target_arch` przekazana z rustc)
   - `target_triple: &'static str`

2. **Wsparcie w `build.rs`:**
   - Sprawdzenie obecności katalogu `.git` oraz uruchomienie `git rev-parse HEAD` i `git status --porcelain`.
   - W przypadku braku Gita lub błędu wykonania: deterministyczny fallback na `"unknown"` i `false` (bez zgłaszania paniki i bez dodawania nowych zewnętrznych crate'ów).
   - Rejestracja `cargo:rerun-if-changed=.git/HEAD` i `cargo:rerun-if-changed=.git/index` dla deterministycznej rekompilacji metadanych tożsamości.
   - Brak dodawania timestampu kompilacji do `BuildIdentity` (ochrona pełnej powtarzalności kompilacji / reproducible builds).

3. **Rozróżnienie Kanału, Statusu i Profilu:**
   - **Distribution Channel:** `Alpha` (wersja prerelease).
   - **Distribution Status:** `Development` (kod w trakcie rozwoju, nieopublikowany).
   - **Profile:** `Release` / `Debug` (profil kompilatora; `Release` nie implikuje stabilnego wydania publicznego).

---

## 7. Polityka SemVer i UpdateVersionDecision

Rozszerzenie walidacji wersji w `src/cloud/updater.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateVersionDecision {
    UpdateAvailable,
    SameVersion,
    DowngradeRejected,
    PrereleaseChannelNotAllowed,
    InvalidInstalledVersion,
    InvalidTargetVersion,
    UnsupportedTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateChannelPreference {
    StableOnly,
    Beta,
    Alpha,
    Development,
}
```

Reguły decyzyjne ewaluowane przez `evaluate_version_transition(installed: &str, target: &str, preference: UpdateChannelPreference)`:
- `1.2.0` → `1.3.0-alpha.1` + `UpdateChannelPreference::Alpha` => `UpdateAvailable`
- `1.2.0` → `1.3.0-alpha.1` + `UpdateChannelPreference::StableOnly` => `PrereleaseChannelNotAllowed`
- `1.3.0-alpha.1` → `1.3.0-alpha.1` => `SameVersion`
- `1.3.0-alpha.1` → `1.2.0` => `DowngradeRejected`
- `1.3.0-alpha.1` → `1.3.0-alpha.2` => `UpdateAvailable`
- `1.3.0-alpha.2` → `1.3.0-beta.1` => `UpdateAvailable`
- `1.3.0-beta.1` → `1.3.0-rc.1` => `UpdateAvailable`
- `1.3.0-rc.1` → `1.3.0` => `UpdateAvailable`

---

## 8. Powiązanie Tożsamości z Updaterem i CLI

1. **Wczesne flagi CLI w `SPLogbook.exe`:**
   - `--version` / `-V`: wypisuje `SPLogbook 1.3.0-alpha.1` i kończy działanie kodem 0 przed inicjalizacją Tokio, bazy danych, audio, CAT i GUI.
   - `--build-info`: wypisuje sformatowaną tożsamość kompilacji (wersja, commit, kanał, profil, platforma, dirty) i kończy działanie kodem 0.
2. **Flaga w `splogbook-updater.exe`:**
   - `--diagnostic-version`: wypisuje jednoznaczną tożsamość opartą o `BuildIdentity` (`splogbook-updater 1.3.0-alpha.1 (commit: ... channel: alpha)`) zamiast zdezaktualizowanego ciągu znaków.
3. **Powiązanie z transakcją i dowodem zdrowia:**
   - `HealthProof` generowany w trybie `--check-health-startup` kandydata czerpie wersję i commit z `BuildIdentity::current()`.
   - Helper weryfikuje, czy `HealthProof` zawiera oczekiwaną wersję docelową (`1.3.0-alpha.1`) oraz właściwy commit Git. W przypadku niespójności proof jest odrzucany, co wymusza automatyczny rollback i przywrócenie poprzedniej stabilnej wersji instalacji.

---

## 9. Analiza Ryzyk i Środki Zaradcze

1. **Ryzyko naruszenia publicznego taga v1.2.0:**
   - *Środek zaradczy:* Weryfikacja hashów lokalnych i zdalnych tagów; brak jakichkolwiek poleceń `git tag`, `git push --tags` czy modyfikacji wydań.
2. **Ryzyko przypadkowej aktualizacji zależności w `Cargo.lock`:**
   - *Środek zaradczy:* Aktualizacja wersji wyłącznie poleceniem edycji `Cargo.toml` oraz celowanym `cargo check -p splogbook`, a następnie inspekcja `git diff Cargo.lock` pod kątem braku zmian w pakietach zewnętrznych.
3. **Ryzyko regresji istniejących testów sprawdzających wersję:**
   - *Środek zaradczy:* Utrzymanie wstecznej kompatybilności fikstur testowych operujących na `1.2.0` (np. symulacja instalacji źródłowej `1.2.0` aktualizowanej do `1.3.0-alpha.1`).
4. **Ryzyko niepoprawnego zachowania w środowiskach bez Gita:**
   - *Środek zaradczy:* `build.rs` posiada bezpieczny fallback na wartości `"unknown"`, nie powodując błędów kompilacji przy braku katalogu `.git`.

---

## 10. Plan Commitów

Zgodnie z wymaganiami Sekcji 20:
1. `chore(version): begin SPLogbook 1.3.0 alpha development` — zmiana `Cargo.toml` na `1.3.0-alpha.1`, odświeżenie `Cargo.lock` wyłącznie dla pakietu lokalnego.
2. `feat(build): add unified SPLogbook build identity` — wdrożenie centralnego modułu `BuildIdentity`, obsługa w `build.rs`, flagi CLI `--version` i `--build-info` w `SPLogbook`, odświeżenie `splogbook-updater --diagnostic-version`, integracja z GUI i oknem „O programie”.
3. `feat(version): bind updater health and results to build identity` — powiązanie `HealthProof`, `UpdateTransaction`, `WindowsHelperRequest` i `WindowsHelperResult` z tożsamością kompilacji.
4. `feat(version): enforce semantic update and prerelease policy` — wdrożenie `UpdateVersionDecision`, walidacji kanałów i reguł SemVer przejścia wersji.
5. `test(version): verify alpha build and update identity end to end` — testy jednostkowe, integracyjne, guardy bezpieczeństwa i testy wielokrotne stabilności (100x / 50x / 20x).
6. `docs(version): close development version identity gate` — sporządzenie raportu `STAGE_M3_DEV_VERSION_IDENTITY_REPORT.md` i aktualizacja rejestrów dokumentacji.
