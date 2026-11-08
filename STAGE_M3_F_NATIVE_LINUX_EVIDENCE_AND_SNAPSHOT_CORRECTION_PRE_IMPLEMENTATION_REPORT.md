# RAPORT PRZEDIMPLEMENTACYJNY
# STAGE-M3-F-NATIVE-LINUX-EVIDENCE-AND-SNAPSHOT-CORRECTION-GATE

**Projekt:** SPLogbook
**Wersja publiczna bieżąca (Current Public Stable):** 1.2.0
**Wersja rozwojowa bieżąca (Current Development):** 1.3.0-alpha.1
**Status bramki:** PRE-IMPLEMENTATION AUDIT / PLANNING
**Data robocza projektu:** 2026-11-08
**Rzeczywista data środowiska:** 2026-11-08
**Rzeczywisty czas środowiska:** 18:41:25
**Strefa czasowa (Timezone):** Central European Standard Time (CET)
**UTC Offset:** +01:00
**Rola audytora:** Główny Audytor Techniczny SPLogbook / Starszy Programista Rust 2024 / Specjalista Linux x86_64, ELF, glibc ABI i OCI

---

## 1. Data i Czas Środowiska

- **DATA ROBOCZA PROJEKTU:** 2026-11-08
- **DATA ŚRODOWISKA:** 2026-11-08
- **CZAS ŚRODOWISKA:** 18:41:25
- **TIMEZONE:** Central European Standard Time (CET)
- **UTC OFFSET:** +01:00
- **Zasada daty:** Data środowiska nie wyprzedza 2026-11-08; zegar i daty istniejących commitów nie ulegają modyfikacji.

---

## 2. Status Git i Tożsamość Liniowej Historii

Po wykonaniu obowiązkowej sekwencji `git fetch --prune origin` oraz inspekcji drzewa:

```text
git rev-parse HEAD:                           9838dd261973a77faac9cd95fe59db3fb8048683
git rev-parse main:                           9838dd261973a77faac9cd95fe59db3fb8048683
git rev-parse origin/main:                    9838dd261973a77faac9cd95fe59db3fb8048683
git rev-parse repair/full-system-remediation: 9838dd261973a77faac9cd95fe59db3fb8048683
git rev-list --left-right --count origin/main...main:                        0 0
git rev-list --left-right --count origin/main...repair/full-system-remediation: 0 0
git merge-base main origin/main:               9838dd261973a77faac9cd95fe59db3fb8048683
Brak operacji w toku (MERGE_HEAD, REBASE_HEAD, CHERRY_PICK_HEAD, REVERT_HEAD: null)
Working tree: CLEAN
```

**Klasyfikacja bazowa:** `A. EXACT_REPORTED_BASELINE`
`HEAD == main == origin/main == repair/full-system-remediation == 9838dd2`

---

## 3. Ochrona i Integralność Tagu v1.2.0

Zweryfikowano nienaruszalność publicznego taga v1.2.0:
- **TAG REF:** `refs/tags/v1.2.0`
- **TAG TYPE:** ANNOTATED
- **TAG OBJECT:** `3ea8d51aab436805107a1cf422e0b35cc301458f`
- **PEELED COMMIT:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
- **ORIGIN LS-REMOTE MATCH:** Zgodny w 100% z `refs/tags/v1.2.0` oraz `refs/tags/v1.2.0^{}`
- **TAG MOVED:** NO

Tag `v1.2.0` pozostaje bezwzględnie chroniony i nienaruszony.

---

## 4. Wykryte Błędy i Rozbieżności w Poprzednim Snapshotcie i Raportach

W toku szczegółowego audytu plików `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json`, `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md` oraz `STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_INTEGRATION_REPORT.md` zidentyfikowano następujące błędy metadanych, które wymagają bezwzględnej formalnej korekty:

1. **Ubuntu 26.10 (Stable Interim):**
   - *Błąd w poprzednim stanie:* Brak jawnej, kanonicznej nazwy kodowej `Stonking Stingray` w snapshotcie; ryzyko mylenia z poprzednimi wydaniami (Oracular, Plucky).
   - *Wymagana korekta:* Przypisanie oficjalnej nazwy kodowej `Stonking Stingray`, kanał `stable interim`, status oficjalnie wydany (`officially released`).
2. **Ubuntu 26.04 LTS:**
   - *Błąd w poprzednim stanie:* Brak rozwiązania aktualnego point release oraz brak nazwy kodowej `Resolute Raccoon`.
   - *Wymagana korekta:* Oficjalna nazwa `Ubuntu 26.04 LTS Resolute Raccoon`, point release `26.04.1`, kanał `LTS`.
3. **Debian Stable:**
   - *Błąd w poprzednim stanie:* Wpisanie nieaktualnego punktowego wydania `Debian 13.2 (Trixie)`.
   - *Wymagana korekta:* Oficjalny najnowszy stabilny release `Debian 13.7 Trixie` (wydany 2026-09-12), architektura `amd64 / x86_64`.
4. **Fedora Final/GA:**
   - *Błąd w poprzednim stanie:* Zarejestrowanie przestarzałej wersji `Fedora 44 Final/GA`.
   - *Wymagana korekta:* `Fedora Linux 45 Final/GA` (oficjalnie wydana 2026-11-03 jako najnowsze stabilne wydanie Fedora).
5. **openSUSE Leap:**
   - *Błąd w poprzednim stanie:* Wpisanie starego `Leap 16.0`.
   - *Wymagana korekta:* `openSUSE Leap 16.1` (final-supported, oficjalnie wydany 2026-11-03).
6. **Arch Linux:**
   - *Błąd w poprzednim stanie:* Status czysto modelowy bez zarejestrowanego rzeczywistego sprawdzenia pakietów i digestu oficjalnego obrazu OCI.
   - *Wymagana korekta:* Przypięcie oficjalnego snapshotu `archlinux:base-20261101.0.620205` z digestem `sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9`, brak AUR, brak repozytoriów testowych.
7. **openSUSE Tumbleweed:**
   - *Błąd w poprzednim stanie:* Brak odczytu rzeczywistego digestu opublikowanego snapshotu.
   - *Wymagana korekta:* Przypięcie snapshotu `opensuse/tumbleweed:latest` (snapshot 20261103) z digestem `sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1`.
8. **Rozróżnienie testów modelowych od dowodów natywnych:**
   - Poprzednie testy jednostkowe weryfikowały jedynie model Rust struktur danych. Żaden profil bez rzeczywistego wykonania natywnego kodu Linux i zerowego kodu wyjścia nie może posiadać statusu `VERIFIED_PASS`. Wszelkie profile oczekujące na natywny log muszą mieć status `NOT_EXECUTED` lub `PENDING_NATIVE_VERIFICATION`.
9. **Kompilacja warunkowa w kodzie produkcyjnym dla platformy Linux:**
   - Wykryto usterki kompilacji w kodzie `src/cloud/updater.rs` oraz `src/cloud/updater/windows_helper.rs` ujawniające się wyłącznie przy kompilacji pod Linux (`x86_64-unknown-linux-gnu`): brak importu `PathBuf` w `windows_helper.rs:295`, brak definicji `tx` i `lock_guard` w sekcji `#[cfg(not(target_os = "windows"))]` w `updater.rs:4332`, oraz odwołanie do nieistniejącego wariantu `UpdateStateId::Completed` zamiast `Committed`/`Cleanup`. Wymagają one precyzyjnej naprawy funkcjonalnej popartej testem.

---

## 5. Dostępne Środowiska Natywne Linux i Plan Wykonania

Host roboczy użytkownika to system **Windows 10/11 x86_64** (brak lokalnego Dockera, WSL ani Podmana).
Jednakże projekt posiada w pełni uwierzytelnione środowisko **GitHub Actions** (`gh` CLI v2.101.0, konto `sp6ina`, uprawnienia `workflow` i `repo`).

Środowisko GitHub Actions na biegaczach `ubuntu-latest` posiada:
- Natywny system Linux x86_64 (Ubuntu 24.04/26.04 runner),
- Demon Docker CE z pełnym dostępem do uruchamiania kontenerów OCI,
- Narzędzia kompilacji Rust (`cargo`, `rustc` stable), C/C++ (`gcc`, `clang`, `patchelf`),
- Możliwość uruchomienia kontenerów dla wszystkich 7 dystrybucji z przypiętymi digestami,
- Możliwość transferu dokładnie jednego kanonicznego artefikatu tar.gz pomiędzy jobem budującym a jobami walidacyjnymi.

---

## 6. Rozwiązanie Oficjalnych Profili i Przypięcie Obrazów OCI

Zgodnie z weryfikacją rejestrów Docker Hub z dnia 2026-11-08:

| ID Dystrybucji | Oficjalna Nazwa | Wersja / Codenam | Kanał | Oficjalny Obraz OCI | Przypięty Digest SHA-256 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `ubuntu-stable` | Ubuntu Linux | 26.10 Stonking Stingray | stable interim | `ubuntu:26.10` | `sha256:262df65c0ec6bf8d9dcc87454f4330a05fae4b97825f45854af3b136666584c2` |
| `ubuntu-lts` | Ubuntu Linux LTS | 26.04.1 LTS Resolute Raccoon | LTS | `ubuntu:26.04` | `sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c` |
| `debian-stable` | Debian GNU/Linux | 13.7 Trixie | stable | `debian:13.7` | `sha256:9c6e381421d0f48d1b8133e573f33c185ee16cbf505eda9c4a8a4791b25ea8a6` |
| `fedora-final` | Fedora Linux | 45 Final/GA | Final/GA | `fedora:45` | `sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c` |
| `arch-rolling` | Arch Linux | snapshot-2026.11.01 | official rolling | `archlinux:base-20261101.0.620205` | `sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9` |
| `opensuse-leap` | openSUSE Leap | 16.1 | final-supported | `opensuse/leap:16.1` | `sha256:756634f02c733fc52d5c9aa09aca6374c91c53c675dd445743736e13a8428858` |
| `opensuse-tumbleweed` | openSUSE Tumbleweed | snapshot-20261103 | official snapshot | `opensuse/tumbleweed:latest` | `sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1` |

---

## 7. Plan Jednego Kanonicznego Artefaktu i Audytu ELF

1. **Jeden Kanoniczny Artefakt:**
   - Nazwa: `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
   - Struktura: Dokładnie jeden katalog nadrzędny `SPLogbook/`
   - Zawartość: binaria `SPLogbook`, `splogbook-updater`, marker `.splogbook-linux-portable-marker`, licencje, zasoby desktop/ikony, biblioteki dynamiczne Hamlib w `SPLogbook/lib/`.
   - Suma kontrolna: Kanoniczny SHA-256 wyliczony natychmiast po spakowaniu w jobie budującym i rygorystycznie weryfikowany w każdym jobie macierzy (`sha256sum -c`).
2. **Audyt ELF dla binariów:**
   - `file`: Potwierdzenie `ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV)`.
   - `readelf -l`: Sprawdzenie interpretera dynamicznego (`/lib64/ld-linux-x86-64.so.2`).
   - `readelf -d`: Weryfikacja `RUNPATH` (zakaz sztywnych ścieżek bezwzględnych hosta budującego, poprawny `$ORIGIN/lib` lub `$ORIGIN/../lib`).
   - `readelf --version-info`: Wyznaczenie najwyższego wymaganego symbolu GLIBC i GLIBCXX.
3. **Audyt Hamlib (`libhamlib.so.4`):**
   - Sprawdzenie SONAME `libhamlib.so.4`.
   - Sprawdzenie zależności bezpośrednich i przechodnich przez `ldd` oraz `readelf -d`.
   - Weryfikacja obecności plików licencyjnych (GPLv3 / LGPLv2.1).
   - Weryfikacja RUNPATH biblioteki.

---

## 8. Dynamiczne Przeliczenie Flooru Kernela

Po zebraniu domyślnych kerneli z oficjalnych profili dystrybucyjnych (Debian 13.7 i Leap 16.1 bazujące na serii 6.12 LTS, pozostałe dystrybucje na nowszych seriach 6.14, 6.17, 6.18):
$$\text{Resolved Minimum Kernel} = \min_{d} (\text{DefaultKernel}(d)) = \mathbf{6.12}$$
Floor ten zostanie precyzyjnie zarejestrowany w metadanych snapshotu oraz markera.

---

## 9. Dozwolone Pliki do Zmiany

Zgodnie z punktem 28 zasad:
- `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json` (korekta metadanych dystrybucji i schematu)
- `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md` (korekta dokumentacyjna tabel i opisów)
- `tests/linux_multi_distribution_compatibility_test.rs` (korekta stałych macierzy i asercji)
- `tests/security_guard.rs` (korekta asercji snapshotu)
- `src/cloud/updater.rs` oraz `src/cloud/updater/windows_helper.rs` (naprawa kompilacji warunkowej pod Linux `x86_64-unknown-linux-gnu`)
- `scripts/build_linux_portable_tar_gz.sh` (ewentualne metadane builda)
- `.github/workflows/ci.yml` oraz `.github/workflows/build-and-release.yml` (przechwytywanie natywnych dowodów)
- `STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_INTEGRATION_REPORT.md` (dodanie sekcji erraty)
- Dokumenty rejestru i mapy drogowej (`docs/IMPROVEMENT_ROADMAP.md`, `docs/AUDIT_FINDINGS_REGISTER.md`, `docs/REMEDIATION_BASELINE.md`, `MASTER_IMPLEMENTATION_PLAN.md`).

---

## 10. Plan Commitów

1. **COMMIT 1:** `docs(linux): correct M3-F distribution support snapshot`
   - Korekta metadanych: Ubuntu 26.10 Stonking Stingray, Ubuntu 26.04 LTS Resolute Raccoon, Debian 13.7 Trixie, Fedora Linux 45 Final/GA, openSUSE Leap 16.1, rolling snapshots placeholders.
2. **COMMIT 2 (Fix):** `fix(linux): resolve portable updater compilation on Linux targets`
   - Usunięcie brakujących symboli w `src/cloud/updater.rs` i `windows_helper.rs` dla celów nie-Windows, poprawienie przejścia stanu na `Committed`.
3. **COMMIT 3 (CI):** `ci(linux): capture native multi-distribution M3-F evidence`
   - Konfiguracja biegaczy CI z przypiętymi digestami obrazów, transferem pojedynczego artefaktu i asercjami SHA-256.
4. **COMMIT 4 (Tests):** `test(linux): enforce native evidence and ABI verification`
   - Aktualizacja testów modelu wielodystrybucyjnego do nowych wersji dystrybucji.
5. **COMMIT 5 (Docs):** `docs(linux): finalize corrected M3-F native verification`
   - Oficjalna errata, raport końcowy, aktualizacja planu głównego i rejestru audytu.

---

## 11. Warunki Zatrzymania (Stop Conditions)

Praca zostanie natychmiast zatrzymana, jeżeli:
1. Wystąpi konflikt w historii Git lub rozbieżność `origin/main` / `main`.
2. Zostanie naruszony tag `v1.2.0`.
3. Zostanie wykryty brak zgodności ABI lub brakujące biblioteki współdzielone na którymkolwiek z 7 profili.
4. Różne dystrybucje będą wymagać różnych sum SHA-256 (złamanie zasady jednego artefikatu).
5. Dojdzie do wycieku ścieżek budowania hosta w binariach ELF.
6. Naruszone zostaną bazy danych użytkownika (`.db`, `-wal`, `-shm`) lub pliki konfiguracyjne.
7. Kod produkcyjny wymagałby uprawnień `root`/`sudo`.
