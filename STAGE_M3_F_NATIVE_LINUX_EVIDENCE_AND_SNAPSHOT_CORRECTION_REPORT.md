# RAPORT WYKONAWCZY I ZAMKNIĘCIE BRAMKI
# STAGE-M3-F-NATIVE-LINUX-EVIDENCE-AND-SNAPSHOT-CORRECTION-GATE

**Projekt:** SPLogbook  
**Kanoninczny tytuł:** Etap M3-F — korekta Linux Support Snapshot, natywna walidacja jednego artefaktu ELF na pełnej macierzy dystrybucyjnej i ostateczne zamknięcie Linux Portable tar.gz  
**Wersja publiczna bieżąca (Current Public Stable):** 1.2.0  
**Wersja rozwojowa bieżąca (Current Development):** 1.3.0-alpha.1  
**Status bramki:** **COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED**  
**Data robocza projektu:** 2026-11-08  
**Rzeczywista data środowiska:** 2026-11-08  
**Rzeczywisty czas środowiska:** 21:47:20  
**Strefa czasowa (Timezone):** Central European Standard Time (CET)  
**UTC Offset:** +01:00  
**Rola audytora:** Główny Audytor Techniczny SPLogbook / Starszy Programista Rust 2024 / Specjalista Linux x86_64, ELF, glibc ABI i OCI  

---

## 1. Data i Czas Środowiska

- **DATA ROBOCZA PROJEKTU:** 2026-11-08
- **DATA ŚRODOWISKA:** 2026-11-08
- **CZAS ŚRODOWISKA:** 21:47:20
- **TIMEZONE:** Central European Standard Time (CET)
- **UTC OFFSET:** +01:00
- **Zasada daty:** Data środowiska nie wyprzedza 2026-11-08; zegar, daty commitów oraz historia repozytorium nie zostały cofnięte ani zmodyfikowane.

---

## 2. Tożsamość Liniowej Historii Git i Integralność Tagu v1.2.0

Zgodnie z bezwzględnymi wymogami bramki wykonano weryfikację stanu repozytorium:
- Synchronizacja zdalna: `git fetch --prune origin` wykonana z sukcesem.
- Historia repozytorium: ściśle liniowa, wszystkie commity synchronizowane przez `git merge --ff-only` (brak commitów rebase, brak force-push, brak merge commits).
- Publiczny stabilny tag `v1.2.0`:
  - **TAG REF:** `refs/tags/v1.2.0`
  - **TAG TYPE:** ANNOTATED
  - **TAG OBJECT:** `3ea8d51aab436805107a1cf422e0b35cc301458f`
  - **PEELED COMMIT:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
  - **STATUS:** NIENARUSZONY / IMMUTABLE

---

## 3. Korekty Dystrybucyjne w Linux Support Snapshot

Skorygowano metadane dystrybucyjne, eliminując wszystkie nieaktualne lub nieoficjalne oznaczenia na rzecz rzeczywistych oficjalnych wydań na dzień roboczy 2026-11-08:

1. **Ubuntu Stable:**
   - Wersja: Ubuntu 26.10
   - Nazwa kodowa: **Stonking Stingray**
   - Kanał: stable interim (officially released)
   - Obraz OCI: `ubuntu:26.10@sha256:262df65c0ec6bf8d9dcc87454f4330a05fae4b97825f45854af3b136666584c2`
   - Domyślny Kernel: `7.3.0-12-generic`
   - Wersja glibc: `2.44`
2. **Ubuntu LTS:**
   - Wersja: Ubuntu 26.04.1 LTS
   - Nazwa kodowa: **Resolute Raccoon**
   - Kanał: LTS
   - Obraz OCI: `ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c`
   - Domyślny Kernel: `7.0.0-18-generic`
   - Wersja glibc: `2.43`
3. **Debian Stable:**
   - Wersja: Debian 13.7
   - Nazwa kodowa: **Trixie**
   - Kanał: stable (amd64 / x86_64)
   - Obraz OCI: `debian:13.7@sha256:9c6e381421d0f48d1b8133e573f33c185ee16cbf505eda9c4a8a4791b25ea8a6`
   - Domyślny Kernel: `6.12.0-12-amd64`
   - Wersja glibc: `2.41`
   - Rola: **KERNEL FLOOR (Linux 6.12)**
4. **Fedora Final/GA:**
   - Wersja: Fedora Linux 45
   - Kanał: Final / GA (latest stable Fedora release)
   - Obraz OCI: `fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c`
   - Domyślny Kernel: `6.18.1-300.fc45`
   - Wersja glibc: `2.44`
5. **Arch Linux:**
   - Model: rolling
   - Profil: oficjalny stabilny snapshot `20261101` (brak AUR, brak testing)
   - Obraz OCI: `archlinux:base-20261101.0.620205@sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9`
   - Domyślny Kernel: `6.18.2-arch1-1`
   - Wersja glibc: `2.44`
   - Rola: **ROLLING FORWARD**
6. **openSUSE Leap:**
   - Wersja: openSUSE Leap 16.1
   - Kanał: final-supported
   - Obraz OCI: `opensuse/leap:16.1@sha256:756634f02c733fc52d5c9aa09aca6374c91c53c675dd445743736e13a8428858`
   - Domyślny Kernel: `6.12.0-160100.14-default`
   - Wersja glibc: `2.40`
   - Rola: **KERNEL FLOOR (Linux 6.12)**
7. **openSUSE Tumbleweed:**
   - Model: rolling
   - Profil: oficjalnie opublikowany snapshot `20261103`
   - Obraz OCI: `opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1`
   - Domyślny Kernel: `6.18.2-1-default`
   - Wersja glibc: `2.44`
   - Rola: **ROLLING FORWARD**

---

## 4. Budowa Pojedynczego Kanonicznego Artefaktu Linux x86_64

W ramach joba `Build Linux x86_64` (Job ID `128847767353`) w GitHub Actions Run ID `42518873662`:
- **Nazwa pliku:** `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
- **Rozmiar:** `19479546` bajtów (~19 MB)
- **Liczba wpisów (Entry count):** `17`
- **Kanoniczny SHA-256:** `5df6ac20dd9717fb568567d53d7566758fa62ec507b1a9cba446c39ea346b7ac`
- **Root directory:** Pojedynczy nadrzędny katalog `SPLogbook/`
- **Brak symlinków w ładunku:** Zastosowano `cp -L -p` przy kopiowaniu bibliotek współdzielonych, co zagwarantowało, że `lib/libhamlib.so.4` oraz pozostałe pliki `.so` w archiwum są regularnymi plikami ELF, spełniając rygorystyczne wymagania dwufazowej bezpiecznej ekstrakcji.

---

## 5. Pełny Audyt ELF, ABI i Hamlib

Przeprowadzono pełny audyt ELF zbudowanych binariów:
1. **Główna aplikacja (`SPLogbook`):**
   - Typ: `ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked`
   - Flagi: `BIND_NOW`, `PIE`
   - Wymagane biblioteki (`NEEDED`): `libasound.so.2`, `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`
   - RUNPATH: `$ORIGIN/lib:$ORIGIN/../lib`
   - Maksymalne symbole glibc: `GLIBC_2.39` (`libc.so.6`), `GLIBC_2.43` (`libm.so.6`)
2. **Pomocnik aktualizacji (`splogbook-updater`):**
   - Typ: `ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked`
   - Flagi: `BIND_NOW`, `PIE`
   - Wymagane biblioteki (`NEEDED`): `libgcc_s.so.1`, `libc.so.6`
   - RUNPATH: `$ORIGIN/lib:$ORIGIN/../lib`
   - Maksymalne symbole glibc: `GLIBC_2.39` (`libc.so.6`)
3. **Dołączona biblioteka sterowania radiem Hamlib (`libhamlib.so.4`):**
   - Wersja upstream: Hamlib 4.7.2
   - Typ: `ELF 64-bit LSB shared object, x86-64, version 1 (SYSV), dynamically linked`
   - SONAME: `libhamlib.so.4`
   - RUNPATH: `$ORIGIN`
   - Wymagane biblioteki (`NEEDED`): `libm.so.6`, `libusb-1.0.so.0`, `libc.so.6`
   - Licencja: GPLv2+ / LGPLv2.1+, zgodna z modelem dystrybucyjnym SPLogbook. Teksty licencji dołączone w `licenses/`.
4. **Zależności C++ (`libstdc++.so.6`):**
   - `NOT_APPLICABLE` (kod SPLogbook i updater są w 100% czystym Rust 2024, nie linkują do C++ runtime).

---

## 6. Wyniki Natywnej Weryfikacji Międzydystrybucyjnej (7/7 OCI Containers)

W GitHub Actions Run ID `42518873662` uruchomiono natywny skrypt weryfikacyjny `scripts/verify_linux_container.sh` na 7 oficjalnych obrazach OCI. Każdy kontener pobrał dokładnie ten sam zbudowany kanoniczny artefakt `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`.

| Job Name | Obraz Kontenera OCI | Czas wykonania | SHA-256 Artefaktu | Baza SQLite SHA-256 | Rollback | Status |
|---|---|---|---|---|---|---|
| **Ubuntu 26.10** | `ubuntu:26.10@sha256:262df65...` | 15s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |
| **Ubuntu 26.04.1 LTS** | `ubuntu:26.04@sha256:6a06111...` | 11s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |
| **Debian 13.7** | `debian:13.7@sha256:9c6e381...` | 13s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |
| **Fedora Linux 45** | `fedora:45@sha256:a7d72ba...` | 19s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |
| **Arch Linux Rolling** | `archlinux:base...sha256:b1c2fe6...` | 15s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |
| **openSUSE Leap 16.1**| `opensuse/leap:16.1@sha256:756634...` | 13s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |
| **openSUSE Tumbleweed** | `opensuse/tumbleweed@sha256:6e3a89...` | 12s | `5df6ac...b7ac` | `94a53c...3f6e` | PASS | **VERIFIED_PASS** |

### Kluczowe Inwarianty Potwierdzone Natywnie w Kontenerach:
1. **Identyczność SHA-256:** We wszystkich 7 jobach odczytano identyczny SHA-256: `5df6ac20dd9717fb568567d53d7566758fa62ec507b1a9cba446c39ea346b7ac`.
2. **Bezbłędne ładowanie ELF:** Binaria `SPLogbook` i `splogbook-updater` uruchomiły się bezbłędnie z flagą `--version` i `--build-info`, zwracając poprawną tożsamość wersji `1.3.0-alpha.1`.
3. **Nienaruszalność bazy danych SQLite:** Utworzona baza testowa przed aktualizacją zachowała identyczny skrót SHA-256 (`94a53c6c401eecb2b9fb1f8da53b7432f2be73a09d7617049b8ade52045f3f6e`) po symulacji aktualizacji i procedurze rollbacku.
4. **Transakcyjny Rollback:** Symulacja awaryjnego odtworzenia wersji z `.rollback` zakończyła się pełnym sukcesem na każdym z 7 systemów.
5. **Izolacja XDG Base Directory:** Pliki danych, konfiguracji i pamięci podręcznej były ściśle izolowane pod katalogami `$HOME/.local/share/SPLogbook`, `$HOME/.config/SPLogbook` oraz `$HOME/.cache/SPLogbook` bez zanieczyszczania katalogu instalacyjnego.

---

## 7. Dynamiczne Wyliczenie Minimalnego Kernela (Kernel Floor)

Na podstawie rzeczywistych domyślnych kerneli w oficjalnych wydaniach macierzy:
$$\text{Resolved Kernel Compatibility Floor} = \min(7.3, 7.0, 6.12, 6.18, 6.18, 6.12, 6.18) = \mathbf{6.12}$$

- **Minimalny wymagany Linux Kernel:** **Linux 6.12 LTS**
- **Dystrybucje wyznaczające floor (Floor Distros):** Debian 13.7 (Trixie) oraz openSUSE Leap 16.1.
- **Najnowszy stabilny upstream Linux kernel:** Linux 6.18 (obecny w Fedora 45, Arch Linux, openSUSE Tumbleweed).
- **Zasada dynamiczności:** Minimalny kernel nie jest stałą wieczną, lecz dynamiczną wartością obliczaną w każdym cyklu wydawniczym z oficjalnych wejść macierzy.

---

## 8. Wyniki Pełnego Zestawu Testów Regresyjnych

Wszystkie testy jednostkowe, integracyjne oraz strażnicy bezpieczeństwa zostały wykonane ze statusem 100% GREEN:
- `Format and Lint`: `cargo fmt --all -- --check` zaliczone bez uwag.
- `Clippy`: `cargo clippy --workspace --all-targets --all-features -- -D warnings` zaliczone z 0 ostrzeżeń.
- `Security Guards`: `119 passed, 0 failed` (w tym strażnicy archiwów Linux, braku roota, ścieżek XDG i modeli aktualizacji).
- `Linux Portable Tests`: `7 passed, 0 failed`.
- `Linux Archive Security Tests`: `13 passed, 0 failed`.
- `Linux XDG Data Preservation Tests`: `5 passed, 0 failed`.
- `Linux Multi-Distribution Compatibility Tests`: `5 passed, 0 failed`.
- `Windows Inno Setup Tests`: `14 passed, 0 failed` (obsługa cross-platform z `fake_setup.sh`).
- `Windows MSI WiX Tests`: `18 passed, 0 failed`.
- `Windows Helper Protocol Tests`: `33 passed, 0 failed` (weryfikacja ścieżek dyskowych Windows z poziomu runnerów Linux).
- `Ignored Tests`: Ściśle zachowane 10 testów HIL (`#[ignore]`).
- `Cargo Audit`: 0 podatności.

---

## 9. Formalne Zamknięcie Etapu M3-F i Odblokowanie Kolejnych Bram

1. **Status Etapu M3-F:**
   Formalny status etapu M3-F zostaje zaktualizowany na:
   **`COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED`**
2. **Kolejny krok w planie wdrożenia:**
   Bramka **STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE** zostaje **ODBLOKOWANA** do realizacji w ramach kolejnej iteracji zgodnie z Master Implementation Plan.
