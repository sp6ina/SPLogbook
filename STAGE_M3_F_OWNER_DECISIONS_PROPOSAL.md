# Rejestr Formalnych Decyzji Właściciela Projektu: Etap M3-F
# Dokument: STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md

---

## 1. Wprowadzenie i Status Bramki
- **Identyfikator dokumentu:** `STAGE-M3-F-OWNER-DECISIONS-PROPOSAL`
- **Etap projektu:** `STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE`
- **Kanoniczny tytuł etapu:** Etap M3-F — wielodystrybucyjna implementacja, integracja i natywna weryfikacja przenośnego pakietu Linux tar.gz dla SPLogbook 1.3.0-alpha.1
- **Data sporządzenia / rejestracji:** 2026-11-08
- **Autor / Rola:** Główny Architekt Techniczny SPLogbook, Starszy Programista Rust 2024, Specjalista Linux x86_64, glibc ABI, Kernel, Ekosystemu Dystrybucji i Bezpiecznych Aktualizatorów
- **Status decyzji:** `M3-F OWNER DECISIONS: COMPLETED_DESIGN_VERIFIED`
- **Status implementacji:** `M3-F IMPLEMENTATION: READY_FOR_RESUME`
- **Kluczowa intencja właściciela:** SPLogbook nie może zostać na stałe zamrożony na konkretnej starej wersji dystrybucji, kernela, glibc, toolchainu ani bibliotek. Obowiązuje model:
  `LATEST-STABLE DEVELOPMENT + CURRENT-STABLE DISTRIBUTION SUPPORT + DYNAMIC DISTRIBUTION-DERIVED KERNEL FLOOR + VERIFIED CROSS-DISTRIBUTION ABI COMPATIBILITY + ONE CANONICAL PORTABLE ARTIFACT`.

---

## 2. Kanoniczny Wykaz i Status Formalny Decyzji M3-F

| ID Decyzji | Obszar | Zakres i Wybór Architektoniczny | Status Formalny |
|---|---|---|---|
| **`OWNER-M3-F-LINUX-BASELINE-01`** | Polityka wsparcia dystrybucji | Rolling latest-stable support policy: najnowsze Ubuntu stable, najnowsze Ubuntu LTS, Debian stable, Fedora Final/GA, Arch Linux official rolling snapshot, openSUSE Leap final-supported, openSUSE Tumbleweed official snapshot. | `APPROVED` |
| **`OWNER-M3-F-KERNEL-POLICY-01`** | Polityka kernela | Dynamic distribution-derived kernel compatibility floor: minimum wyliczane jako najniższa domyślna wersja kernela w obowiązkowej macierzy dystrybucji przy każdym wydaniu SPLogbook. Brak wiecznego zamrożenia. Bieżąca wyliczona wartość dynamiczna: Linux 6.12. | `APPROVED` |
| **`OWNER-M3-F-ARCHITECTURE-01`** | Architektura procesora | Wyłącznie `x86_64` (`x86_64-unknown-linux-gnu`). AArch64, ARM 32-bit, x86 32-bit oraz musl libc są out of scope. | `APPROVED` |
| **`OWNER-M3-F-LIBRARY-POLICY-01`** | Polityka bibliotek | Najnowsze stabilne wersje zależności zatwierdzone w kontrolowanym procesie aktualizacji; zasada Common Compatibility Intersection dla ABI glibc. | `APPROVED` |
| **`OWNER-M3-F-DYNAMIC-LIBRARIES-01`** | Biblioteki dynamiczne / Hamlib | Hermetyzacja `lib/libhamlib.so.4` z `RUNPATH = "$ORIGIN/lib"`, 15 warunków weryfikacji ELF i licencji, zakaz modyfikacji systemu hosta. | `APPROVED_WITH_MANDATORY_VERIFICATION_CONDITIONS` |
| **`OWNER-M3-F-PORTABLE-LAYOUT-01`** | Układ archiwum portable | Pojedynczy katalog główny `SPLogbook/`, uprawnienia POSIX `0755`/`0644`, bezwzględny zakaz symlinków, hardlinków i plików specjalnych. | `APPROVED` |
| **`OWNER-M3-F-UPDATE-MODE-01`** | Model aktualizacji | `InstallationType::LinuxPortableTarGz`, `ManagementMode::SelfManaged`, model: `Journaled Transactional Replacement` z kopią zapasową w `.rollback`, testem zdrowia i crash recovery przez dziennik. Zakaz terminologii "Atomic Swap". | `APPROVED` |
| **`OWNER-M3-F-ARCHIVE-EXTRACTION-01`** | Bezpieczna ekstrakcja | Dwuetapowa ekstrakcja w aplikacji (preflight całego archiwum przed jakimkolwiek zapisem, nowy pusty staging). Zakaz systemowego `tar xf` w procesie produkcyjnym. | `APPROVED` |
| **`OWNER-M3-F-XDG-DATA-01`** | Izolacja XDG i ochrona danych | Standard XDG Base Directory (wymóg ścieżek bezwzględnych, bezpieczny fallback pod `$HOME`), brak zapisu obok binarki, detect-and-inform dla legacy (brak auto-migracji). | `APPROVED` |
| **`OWNER-M3-F-SIGNING-01`** | Podpisywanie i integralność | Manifest-first, Ed25519 + SHA-256 + size, brak klucza prywatnego w środowisku testowym (fixtures only), lokalny artefakt niepublikowany, brak nowego tagu/release. | `APPROVED` |
| **`OWNER-M3-F-DESKTOP-INTEGRATION-01`** | Integracja desktopowa | Opcjonalny szablon `.desktop` wewnątrz archiwum. Brak automatycznej instalacji w systemie, brak instalacji w `/usr`, brak `update-desktop-database`, zakaz exec bazującego na powłoce. | `APPROVED_OPTIONAL_TEMPLATE_ONLY` |

---

## 3. Szczegółowa Rejestracja Decyzji

---

### Decyzja 1: `OWNER-M3-F-LINUX-BASELINE-01`
- **Identyfikator:** `OWNER-M3-F-LINUX-BASELINE-01`
- **Status:** `APPROVED`
- **Nazwa polityki:** Rolling latest-stable Linux distribution support policy.
- **Obowiązkowe profile dystrybucyjne:**
  1. Najnowsza oficjalnie wydana stabilna wersja Ubuntu.
  2. Najnowsza oficjalnie wydana wersja Ubuntu LTS, jeżeli różni się od najnowszego stabilnego Ubuntu.
  3. Aktualny Debian stable.
  4. Najnowsza Fedora Final/GA.
  5. Aktualny, w pełni zaktualizowany Arch Linux rolling snapshot.
  6. Najnowszy finalny i wspierany openSUSE Leap.
  7. Aktualny oficjalnie opublikowany openSUSE Tumbleweed snapshot.
- **Polityka szczegółowa:**
  - **Ubuntu:** wyłącznie oficjalne wydanie stabilne oraz najnowsze LTS (jeśli oddzielne); bez development release, beta, RC; bez mutable `ubuntu-latest` jako jedynego dowodu; wersja i digest obrazu muszą być utrwalone w snapshocie.
  - **Debian:** wyłącznie aktualne stable; zakaz testing, unstable, experimental; zakaz używania backports do wyznaczania minimum.
  - **Fedora:** wyłącznie najnowsze Final/GA; zakaz Rawhide, Branched przed Final, Beta, RC, COPR do wyznaczania minimum.
  - **Arch Linux:** model rolling release (bez fikcyjnej klasycznej wersji stable); oficjalny snapshot, pełny system upgrade, wyłącznie oficjalne stabilne repozytoria, brak AUR, brak testing, brak partial upgrade; profil służy do weryfikacji latest-library, latest-toolchain, rolling-release i forward compatibility (nie wyznacza samodzielnie minimalnego ABI glibc).
  - **openSUSE Leap:** wyłącznie najnowsze finalne i wspierane wydanie Leap; bez Alpha/Beta/RC; bez eksperymentalnych repozytoriów ani doinstalowywanego kernela do wyliczania minimum.
  - **openSUSE Tumbleweed:** aktualny oficjalnie opublikowany snapshot; wyłącznie oficjalne repozytoria (brak niestabilnego Factory bez snapshotu); obowiązkowy profil rolling-stable, latest-library, latest-kernel i forward compatibility.
- **Rozwiązywanie wersji:** Dokładne wersje dystrybucji są rozwiązywane z oficjalnych źródeł przy każdym wydaniu SPLogbook i zapisywane w dokumencie `LINUX_SUPPORT_SNAPSHOT_<version>.json`.

---

### Decyzja 2: `OWNER-M3-F-KERNEL-POLICY-01`
- **Identyfikator:** `OWNER-M3-F-KERNEL-POLICY-01`
- **Status:** `APPROVED`
- **Nazwa polityki:** DYNAMIC DISTRIBUTION-DERIVED KERNEL COMPATIBILITY FLOOR.
- **Stałe wieczne minimum:** `NO`.
- **Formuła wyznaczania minimum:**
  ```text
  RESOLVED_MINIMUM_KERNEL = MINIMUM(
      UBUNTU_STABLE_DEFAULT_KERNEL,
      UBUNTU_LTS_DEFAULT_KERNEL_IF_SEPARATE,
      DEBIAN_STABLE_DEFAULT_KERNEL,
      FEDORA_FINAL_DEFAULT_KERNEL,
      ARCH_SNAPSHOT_DEFAULT_KERNEL,
      OPENSUSE_LEAP_DEFAULT_KERNEL,
      OPENSUSE_TUMBLEWEED_DEFAULT_KERNEL
  )
  ```
- **Bieżący wynik dynamiczny:** `Linux 6.12` (wartość dynamiczna snapshotu, wyznaczona przez Debian stable oraz openSUSE Leap; brak wiecznej gwarancji).
- **Przeliczanie minimum:** Wymagane przy każdym wydaniu SPLogbook. Zakaz kopiowania z poprzednich wydań bez ponownego odczytu domyślnych kerneli całej macierzy.
- **Źródło kernela do minimum:** wyłącznie oficjalny domyślny kernel dystrybucji zainstalowany w zatwierdzonym profilu. Zabronione: backports, PPA, COPR, AUR, testing, Rawhide, ręcznie kompilowane kernele.
- **Podstawowy kernel deweloperski (Primary Development Kernel):** Najnowszy oficjalnie wydany stabilny upstream Linux kernel dostępny w czasie builda. Test na latest stable kernel jest obowiązkowy.
- **Zasada funkcji jądra (Progressive Enhancement):** Użycie funkcji nowszych niż `RESOLVED_MINIMUM_KERNEL` jest dozwolone wyłącznie pod warunkami: runtime detection, bezpieczny fallback do minimum, brak awarii, brak utraty danych, brak osłabienia bezpieczeństwa. Zakaz niejawnego podnoszenia minimum.

---

### Decyzja 3: `OWNER-M3-F-ARCHITECTURE-01`
- **Identyfikator:** `OWNER-M3-F-ARCHITECTURE-01`
- **Status:** `APPROVED`
- **Obsługiwana architektura:** Wyłącznie `x86_64` (AMD64 / Intel 64).
- **Rust target:** `x86_64-unknown-linux-gnu`.
- **Zakresy wyłączone:** AArch64 (out of scope w M3-F), x86 32-bit (not supported), ARM 32-bit (not supported), musl libc (out of scope).
- **Wymóg:** Każda przyszła architektura wymagać będzie osobnego artefaktu, wpisu w manifeście, analizy ELF, support snapshotu i osobnej macierzy testowej.

---

### Decyzja 4: `OWNER-M3-F-LIBRARY-POLICY-01`
- **Identyfikator:** `OWNER-M3-F-LIBRARY-POLICY-01`
- **Status:** `APPROVED`
- **Polityka deweloperska:** Stosowanie najnowszych stabilnych wersji zależności zatwierdzonych w kontrolowanym procesie aktualizacji (Dependency Update Gate).
- **Wersje zakazane:** alpha, beta, RC, nightly, nieprzypięty master git, wersje z aktywnymi podatnościami.
- **Bramka aktualizacji:** Analiza changelogu, breaking changes, security advisories, audyt licencji, lockfile, kompilacja, Clippy, testy, audyt ABI/ELF, walidacja na `RESOLVED_MINIMUM_KERNEL` oraz latest stable kernel.
- **Kompatybilność wydawnicza:** Zasada *Common Compatibility Intersection*. Wymagania symboli GLIBC/GLIBCXX finalnej binarki nie mogą przekraczać najmniej nowoczesnego ABI w obowiązkowej macierzy. Wymagania glibc są ustalane empirycznie z binarki za pomocą `readelf --version-info`, `objdump -T`, `ldd`.

---

### Decyzja 5: `OWNER-M3-F-DYNAMIC-LIBRARIES-01`
- **Identyfikator:** `OWNER-M3-F-DYNAMIC-LIBRARIES-01`
- **Status:** `APPROVED_WITH_MANDATORY_VERIFICATION_CONDITIONS`
- **Planowana biblioteka:** `lib/libhamlib.so.4`.
- **Planowany RUNPATH:** `$ORIGIN/lib`.
- **15 warunków weryfikacji:**
  1. Potwierdzenie SONAME.
  2. Potwierdzenie ELF x86_64.
  3. Potwierdzenie wersji Hamlib.
  4. Potwierdzenie licencji (LGPL/GPL).
  5. Potwierdzenie prawa do redystrybucji.
  6. Dołączenie wymaganych plików licencyjnych w `THIRD_PARTY_LICENSES/`.
  7. Potwierdzenie zależności bezpośrednich.
  8. Potwierdzenie zależności przechodnich.
  9. Potwierdzenie wymaganych symboli GLIBC.
  10. Potwierdzenie wymaganych symboli GLIBCXX.
  11. Potwierdzenie braku absolutnego RPATH.
  12. Potwierdzenie braku ścieżek build hosta.
  13. Potwierdzenie poprawnego `$ORIGIN/lib` RUNPATH.
  14. Potwierdzenie braku niezatwierdzonego library shadowing.
  15. Potwierdzenie działania tego samego artefaktu na całej macierzy.

---

### Decyzja 6: `OWNER-M3-F-PORTABLE-LAYOUT-01`
- **Identyfikator:** `OWNER-M3-F-PORTABLE-LAYOUT-01`
- **Status:** `APPROVED`
- **Pojedynczy katalog główny archiwum:** `SPLogbook/`.
- **Kanoniczny układ:**
  ```text
  SPLogbook/
  ├── SPLogbook
  ├── splogbook-updater
  ├── lib/
  ├── assets/
  ├── locales/
  ├── README.txt
  ├── LICENSE
  ├── THIRD_PARTY_LICENSES/
  └── .splogbook-linux-portable-marker
  ```
- **Uprawnienia POSIX:** Binaria i katalogi `0755`, zasoby i marker `0644`.
- **Bezwzględne zakazy w archiwum:** Dowiązania symboliczne (`symlinks`), dowiązania twarde (`hard links`), urządzenia blokowe i znakowe, potoki FIFO, gniazda (sockets), bity `setuid`/`setgid`, ścieżki bezwzględne, sekwencje traversal (`..`), wiele katalogów głównych.

---

### Decyzja 7: `OWNER-M3-F-UPDATE-MODE-01`
- **Identyfikator:** `OWNER-M3-F-UPDATE-MODE-01`
- **Status:** `APPROVED`
- **Klasyfikacja:** `InstallationType::LinuxPortableTarGz`, `ManagementMode::SelfManaged`.
- **Model zastępowania:** `JOURNALED_TRANSACTIONAL_REPLACEMENT`.
- **Katalog kopii zapasowej:** `.rollback`.
- **24 fazy aktualizacji:** Weryfikacja manifestu, Ed25519, nazwa pliku, rozmiar, SHA-256, platforma, architektura, typ pakietu, wersja, commit, kompletny preflight, utworzenie nowego pustego stagingu, bezpieczna ekstrakcja, weryfikacja markera, BuildIdentity kandydata, utworzenie dziennika transakcji, backup plików programu, podmiana transakcyjna, health check, potwierdzenie HealthProof (wersja i commit), commit transakcji, rollback w razie błędu, crash recovery na podstawie dziennika.
- **Atomowość:** Operacja wieloplikowa NIE posiada sprzętowej atomowości przy zaniku zasilania (*Not Power-Loss Atomic*). Ochrona przed przerwaniem opiera się na dzienniku (*Crash Recoverable by Journal*). Zakaz używania terminu "Atomic Swap".

---

### Decyzja 8: `OWNER-M3-F-ARCHIVE-EXTRACTION-01`
- **Identyfikator:** `OWNER-M3-F-ARCHIVE-EXTRACTION-01`
- **Status:** `APPROVED`
- **Model ekstrakcji:** Dwufazowa dekompresja sterowana w kodzie aplikacji (*Application-Controlled Two-Phase Extraction*):
  - **Faza 1 (Preflight):** Pełny odczyt wszystkich nagłówków i składowych archiwum bez zapisywania ani jednego bajtu na dysk. Sprawdzenie pojedynczego katalogu głównego, normalizacja ścieżek, containment, weryfikacja typów, duplikatów, kolizji, limitów i uprawnień.
  - **Faza 2 (Ekstrakcja):** Zapis do nowego, całkowicie pustego katalogu stagingu wyłącznie po przejściu Fazy 1 przez 100% składowych.
- **Zakaz systemowego tar:** Kategoryczny zakaz używania `tar xf`, `tar -P`, `--absolute-names`, `--dereference` ani skryptów powłoki jako produkcyjnego mechanizmu rozpakowywania.

---

### Decyzja 9: `OWNER-M3-F-XDG-DATA-01`
- **Identyfikator:** `OWNER-M3-F-XDG-DATA-01`
- **Status:** `APPROVED`
- **Standard ścieżek:**
  - `CONFIG`: `$XDG_CONFIG_HOME/splogbook` (fallback: `$HOME/.config/splogbook`)
  - `DATA`: `$XDG_DATA_HOME/splogbook` (fallback: `$HOME/.local/share/splogbook`)
  - `STATE`: `$XDG_STATE_HOME/splogbook` (fallback: `$HOME/.local/state/splogbook`)
  - `CACHE`: `$XDG_CACHE_HOME/splogbook` (fallback: `$HOME/.cache/splogbook`)
- **Reguła bezpieczeństwa ścieżek XDG:** Wartości zmiennych środowiskowych muszą być ścieżkami bezwzględnymi. Wartości względne są odrzucane z przejściem do bezpiecznego fallbacku pod `$HOME`.
- **Separacja danych:** Zakaz zapisywania danych użytkownika obok binarki. Bezwzględna ochrona baz SQLite, WAL, SHM, dzienników, ADIF, Cabrillo, konfiguracji i plików nieznanych.
- **Dane legacy:** Wykrywanie i informowanie użytkownika w UI; zakaz automatycznej migracji bez formalnej odrębnej decyzji.

---

### Decyzja 10: `OWNER-M3-F-SIGNING-01`
- **Identyfikator:** `OWNER-M3-F-SIGNING-01`
- **Status:** `APPROVED`
- **Dobór assetu:** Manifest-first przez `VerifiedReleaseManifest` i `VerifiedReleaseAsset` (sprawdzenie platformy, architektury, typu pakietu, wersji, commitu, rozmiaru i SHA-256).
- **Podpis manifestu:** Kryptograficzny klucz Ed25519.
- **Ochrona klucza produkcyjnego:** Kategoryczny zakaz obecności klucza prywatnego wydania w środowisku testowym. Weryfikacja opiera się wyłącznie na fixture'ach testowych (`splogbook-signer`).
- **Status artefaktów:** Lokalne archiwa testowe pozostają nieopublikowane; brak publikacji wydań GitHub Release i brak tworzenia nowych tagów Git w ramach M3-F.

---

### Decyzja 11: `OWNER-M3-F-DESKTOP-INTEGRATION-01`
- **Identyfikator:** `OWNER-M3-F-DESKTOP-INTEGRATION-01`
- **Status:** `APPROVED_OPTIONAL_TEMPLATE_ONLY`
- **Szablon Desktop:** Opcjonalny szablon `splogbook.desktop` dostarczany wewnątrz archiwum (w `assets/`).
- **Ograniczenia instalacji:** Brak automatycznej instalacji skrótu na systemie, brak instalacji w `/usr/share/applications`, brak modyfikacji menu systemowego, brak instalacji ikon w katalogach systemowych, brak wywoływania `update-desktop-database`, zakaz odwołań do powłoki w `Exec`.

---

## 4. Podsumowanie Weryfikacji Decyzji
Wszystkie 11 decyzji zostało zarejestrowanych ze statusem wiążącym. Stan etapu:
- `M3-F OWNER DECISIONS: COMPLETED_DESIGN_VERIFIED`
- `M3-F IMPLEMENTATION: READY_FOR_RESUME`

---

## ERRATA — 2026-11-09 (STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE)

Ten raport jest dokumentem historycznym. Obowiązująca polityka została wyrównana decyzją
`OWNER-M3-F-KERNEL-POLICY-OPTION-A` (model dynamiczny) oraz `OWNER-M3-F-GLIBC-POLICY-01`:

- **Minimum runtime:** Linux **7.0**, glibc **2.43** (`GLIBC_2.43`).
- **Wspierane dystrybucje:** Ubuntu 26.04 LTS (7.0), Fedora 45 (7.2), Arch Linux (7.2),
  openSUSE Tumbleweed (7.2).
- **Niewspierane:** Debian 13.7 (glibc 2.41), openSUSE Leap 16.1 (glibc 2.40), Ubuntu interim 26.10.
- **Sprostowanie wartości:** Arch `7.2.14-arch1-1` (nie 6.18.2), Fedora `7.2.8-300.fc45`
  (nie 6.18.1), Tumbleweed `7.2.11-1-default` (nie 6.18.2), Ubuntu 26.04 `7.0.0-42`
  (nie 7.0.0-18), Debian `6.12.101-1` (nie 6.12.43-1).
- **Jakość dowodu:** `scripts/verify_linux_container.sh` nie jest fail-closed (`|| true`, F-03);
  deklaracje `VERIFIED_PASS` z runu `42518873662` nie stanowią dowodu natywnego.
- **Aktualny snapshot:** `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}` (schema v2).