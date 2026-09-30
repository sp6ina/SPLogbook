# Prompt: Utworzenie nadrzędnego planu modernizacji SPLogbook

Wciel się w rolę głównego architekta oprogramowania, doświadczonego programisty Rust, maintenera projektu open source, eksperta bezpieczeństwa łańcucha dostaw oraz recenzenta systemów aktualizacji Windows i Linux.

Pracujesz nad projektem SPLogbook.

Masz dostęp do całego lokalnego repozytorium.

## CEL

Utwórz jeden nadrzędny, spójny i etapowy plan modernizacji SPLogbook, łączący:

1. dotychczasowy `AUDIT_REPORT.md`,
2. aktualny `PLAN.md`,
3. `CODE_QUALITY_NORMALIZATION_PLAN.md`,
4. `DESIGN_2_2_FINAL.md`,
5. `DESIGN_3_1_UPDATER_SIGNATURES.md`,
6. projekt bezpiecznej aktualizacji Windows,
7. projekt bezpiecznej aktualizacji Linux,
8. wspólny rdzeń weryfikacji aktualizacji,
9. podpisy Ed25519,
10. pozostałe potwierdzone problemy jakościowe i funkcjonalne.

Plan ma zapewnić jedną kolejność realizacji bez konfliktów pomiędzy:

- audytem jakości,
- poprawkami krytycznymi,
- aktualizatorem Windows,
- aktualizatorem Linux,
- kryptografią aktualizacji,
- refaktoryzacją architektury,
- czyszczeniem komentarzy i nazewnictwa.

## DOKUMENTY WEJŚCIOWE

Przeczytaj dokładnie:

- `AUDIT_REPORT.md`
- `PLAN.md`
- `CODE_QUALITY_NORMALIZATION_PLAN.md`
- `DESIGN_2_2_FINAL.md`
- `DESIGN_3_1_UPDATER_SIGNATURES.md`
- pozostałe pliki `DESIGN_*.md`
- `Cargo.toml`
- `Cargo.lock`
- `build.rs`
- `src/cloud/updater.rs`
- `src/gui/app.rs`
- `.github/workflows/build-and-release.yml`
- wszystkie pliki bezpośrednio wskazywane przez audyty i plany

Nie zakładaj, że każdy starszy raport jest aktualny.

Jeżeli ustalenia dokumentów są sprzeczne z aktualnym kodem, oznacz sprzeczność i przyjmij aktualny kod jako źródło informacji o stanie implementacji.

Jeżeli dokument `DESIGN` zawiera zatwierdzoną decyzję architektoniczną, nie zastępuj jej swobodną rekomendacją z wcześniejszego audytu.

## WYNIK

Utwórz nowy dokument:

`MASTER_IMPLEMENTATION_PLAN.md`

Dokument ten ma być jedynym nadrzędnym planem organizacyjnym.

Dotychczasowych dokumentów nie usuwaj.

Ich role mają być następujące:

- `MASTER_IMPLEMENTATION_PLAN.md`:
  nadrzędna kolejność, zależności, statusy i etapy;

- `AUDIT_REPORT.md`:
  historyczne źródło ustaleń audytu;

- `CODE_QUALITY_NORMALIZATION_PLAN.md`:
  szczegółowy rejestr problemów jakościowych;

- `DESIGN_*.md`:
  źródła zatwierdzonych decyzji architektonicznych;

- raporty `REPORT_*.md`:
  wyniki etapów analitycznych i weryfikacyjnych.

## NAJWAŻNIEJSZE OGRANICZENIE

**NIE ZMIENIAJ KODU.**

**NIE ZMIENIAJ `Cargo.toml`.**

**NIE ZMIENIAJ `Cargo.lock`.**

**NIE ZMIENIAJ workflow GitHub Actions.**

**NIE ZMIENIAJ `PLAN.md`.**

**NIE ZMIENIAJ istniejących dokumentów `DESIGN`.**

**NIE WYKONUJ COMMITA.**

**NIE ROZPOCZYNAJ żadnego etapu implementacyjnego.**

Utwórz wyłącznie:

`MASTER_IMPLEMENTATION_PLAN.md`

Po jego utworzeniu zatrzymaj się.

## 1. HIERARCHIA ŹRÓDEŁ DECYZJI

W dokumencie zdefiniuj następującą hierarchię:

1. Aktualny kod repozytorium:
   źródło informacji o obecnym stanie implementacji.

2. Zatwierdzone dokumenty `DESIGN`:
   źródło obowiązujących decyzji architektonicznych.

3. `MASTER_IMPLEMENTATION_PLAN.md`:
   źródło kolejności, zależności, statusów i organizacji pracy.

4. Raporty weryfikacyjne `REPORT_*.md`:
   źródło wyników konkretnych analiz.

5. `CODE_QUALITY_NORMALIZATION_PLAN.md`:
   szczegółowy rejestr problemów jakościowych.

6. `AUDIT_REPORT.md`:
   źródło historycznych obserwacji, które muszą zostać ponownie zweryfikowane przed implementacją.

Jeżeli dwa źródła są sprzeczne:

- `DESIGN` ma pierwszeństwo przed rekomendacją audytu,
- aktualny kod ma pierwszeństwo przy ustalaniu stanu faktycznego,
- niejasność wymaga raportu weryfikacyjnego,
- nie wolno podejmować arbitralnej decyzji podczas implementacji.

## 2. JEDEN SYSTEM IDENTYFIKATORÓW

Nadaj wszystkim zadaniom unikatowe prefiksy.

Użyj następujących rodzin:

- `VERIFY-*`:
  zadania analityczne i raporty;

- `FIX-*`:
  małe potwierdzone poprawki funkcjonalne;

- `UPDATE-*`:
  aktualizator Windows, Linux i wspólny instalator;

- `CRYPTO-*`:
  manifest, Ed25519, klucze i podpisywanie;

- `CQ-*`:
  jakość kodu, komentarze, nazewnictwo i małe duplikacje;

- `ARCH-*`:
  duże refaktoryzacje wymagające `DESIGN.md`;

- `RELEASE-*`:
  próbne i publiczne wydania.

Nie używaj identyfikatorów mogących kolidować z dotychczasowym numerem 3.1.

Każde zadanie musi posiadać jeden unikatowy identyfikator.

## 3. STATUSY PROBLEMÓW I ZADAŃ

Dla każdego problemu zapisz:

- Status techniczny
- Status wykonawczy
- Priorytet
- Źródło ustalenia
- Dokument nadrzędny
- Zadanie weryfikacyjne
- Zadanie implementacyjne

Dozwolone statusy techniczne:

- POTWIERDZONY W KODZIE
- CZĘŚCIOWO POTWIERDZONY
- WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ
- WYMAGA TESTU URUCHOMIENIOWEGO
- NIEPOTWIERDZONY
- FAŁSZYWY ALARM
- ROZWIĄZANY

Dozwolone statusy wykonawcze:

- GOTOWY DO IMPLEMENTACJI
- ZABLOKOWANY DO CZASU RAPORTU
- ZABLOKOWANY DO CZASU DESIGN.md
- ZABLOKOWANY PRZEZ INNE ZADANIE
- ROZWIĄZYWANY PRZEZ PLAN NADRZĘDNY
- TYLKO ZADANIE ANALITYCZNE
- NIE IMPLEMENTOWAĆ
- DONE

Zadanie można implementować wyłącznie, gdy jego status wykonawczy to:

`GOTOWY DO IMPLEMENTACJI`

## 4. ZASADA KOORDYNACJI UPDATERA

Wszystkie zmiany związane z:

- `src/cloud/updater.rs`,
- wyborem artefaktów,
- pobieraniem aktualizacji,
- instalacją Windows,
- instalacją Linux,
- GitHub digest,
- SHA-256,
- manifestem wydania,
- Ed25519,
- SemVer i downgrade,
- GitHub Actions,
- procesem wydawniczym

mają należeć do jednego nadrzędnego toru:

`UPDATE + CRYPTO + RELEASE`

Zadania `CQ` nie mogą równolegle przebudowywać tych elementów.

Jeżeli problem wykryty przez audyt jakości jest objęty torem updatera, oznacz go:

`ROZWIĄZYWANY PRZEZ PLAN NADRZĘDNY`

Nie twórz wtedy drugiego zadania implementacyjnego zmieniającego ten sam kod.

## 5. WSPÓLNY RDZEŃ AKTUALIZACJI

Zaprojektuj w planie jeden wspólny rdzeń bezpieczeństwa dla Windows i Linux.

Rdzeń odpowiada za:

- pobranie metadanych wydania,
- pobranie surowych bajtów manifestu,
- pobranie podpisu,
- dekodowanie podpisu Base64,
- weryfikację Ed25519,
- wybór zaufanego `key_id`,
- parsowanie zweryfikowanego manifestu,
- walidację `manifest_version`,
- walidację `product`,
- walidację `channel`,
- walidację pełnego commit SHA,
- sprawdzenie `minimum_updater_version`,
- analizę SemVer,
- ochronę przed downgrade,
- wybór systemu,
- wybór architektury,
- wybór `package_type`,
- kontrolę `asset.size`,
- kontrolę `MAX_UPDATE_SIZE`,
- strumieniowe pobieranie,
- obliczenie SHA-256,
- usunięcie niepełnego pliku,
- przekazanie zweryfikowanego artefaktu instalatorowi platformowemu.

Rdzeń nie wykonuje samodzielnie platformowej wymiany plików.

## 6. AKTUALIZATOR WINDOWS

Utwórz osobny tor:

`UPDATE-WIN`

Najpierw wymagaj dokumentu:

`DESIGN_3_0_A_WINDOWS_UPDATE.md`

Dokument ma poprzedzać implementację.

Plan Windows musi obejmować:

- ustalenie faktycznego formatu wydania,
- ZIP, EXE albo MSI,
- zakaz nadpisywania EXE archiwum ZIP,
- ochronę przed Zip Slip,
- staging,
- kontrolę struktury paczki,
- zamknięcie starego procesu,
- aktualizację bez utraty danych użytkownika,
- rollback,
- uruchomienie nowej wersji,
- health confirmation,
- sprzątanie,
- zachowanie przy braku uprawnień,
- testy na rzeczywistym Windows.

Nie implementuj tych elementów teraz.

## 7. AKTUALIZATOR LINUX

Utwórz osobny tor:

`UPDATE-LINUX`

Najpierw wymagaj dokumentu:

`DESIGN_3_0_B_LINUX_UPDATE.md`

Przed zaprojektowaniem implementacji ustal z repozytorium:

- jakie artefakty Linux są faktycznie budowane,
- jak są pakowane,
- jaka jest struktura `tar.gz`,
- czy istnieje AppImage,
- czy istnieją deb, rpm albo Flatpak,
- gdzie znajduje się binarka,
- gdzie znajdują się konfiguracja i baza użytkownika,
- czy aplikacja rozpoznaje sposób instalacji.

Nie przedstawiaj AppImage, deb, rpm ani Flatpak jako obecnie wspieranych, jeśli repozytorium tego nie potwierdza.

## 8. AWARYJNA BLOKADA LINUX

Przed pełnym instalatorem Linux uwzględnij małe zadanie:

`UPDATE-LINUX-SAFETY-GATE`

Zakres:

- zablokowanie nadpisania pliku wykonywalnego archiwum `tar.gz`,
- zablokowanie nadpisania pliku wykonywalnego pakietem deb lub rpm,
- brak rozpakowywania,
- brak sudo,
- brak modyfikacji katalogów systemowych,
- usunięcie pliku tymczasowego,
- komunikat o ręcznej aktualizacji,
- jawny błąd dla brakującego `tag_name`.

To zadanie jest wyłącznie bramką bezpieczeństwa.

Nie zastępuje docelowego instalatora Linux.

Jeżeli pełna poprawka Linux zostanie wykonana wcześniej, zadanie bezpieczeństwa może zostać oznaczone:

`ROZWIĄZANE PRZEZ UPDATE-LINUX`

## 9. POLITYKI FORMATÓW LINUX

W planie rozróżnij formaty.

### Portable tar.gz

Wymaga:

- ekstrakcji do staging,
- ochrony przed ścieżkami absolutnymi i `..`,
- ochrony przed symlinkami i hardlinkami wychodzącymi poza staging,
- odrzucenia urządzeń, FIFO i socketów,
- weryfikacji struktury,
- zakazu nadpisywania danych użytkownika,
- przygotowania rollbacku,
- kontrolowanej zamiany katalogów.

### AppImage

Jeżeli zostanie rzeczywiście wprowadzony:

- zweryfikowany AppImage jest pojedynczym artefaktem,
- wymiana wymaga katalogu zapisywalnego przez użytkownika,
- brak niejawnego sudo,
- staging na tym samym systemie plików,
- atomowa zamiana,
- rollback,
- health confirmation.

### deb i rpm

Jeżeli zostaną wprowadzone:

- wewnętrzny updater nie nadpisuje samodzielnie `/usr`,
- nie przechwytuje hasła administratora,
- nie wykonuje niejawnego sudo,
- używa właściwego menedżera pakietów albo kieruje użytkownika do ręcznej aktualizacji,
- nie miesza kanału pakietowego z portable `tar.gz` ani AppImage.

### Flatpak

Jeżeli zostanie wprowadzony:

- aktualizacja odbywa się mechanizmem Flatpak,
- wewnętrzny updater nie podmienia własnych plików,
- nie miesza kanału Flatpak z innym formatem.

## 10. WSPÓLNY INTERFEJS INSTALATORÓW

Utwórz tor:

`UPDATE-PLATFORM`

Wymagaj dokumentu:

`DESIGN_3_0_C_PLATFORM_INSTALLERS.md`

Dokument ma zdefiniować logiczne operacje:

- `detect_installation_type`,
- `validate_installation_target`,
- `stage_verified_artifact`,
- `prepare_rollback`,
- `apply_update`,
- `launch_updated_application`,
- `confirm_health`,
- `rollback`,
- `cleanup`.

Nie generuj kodu ani traita Rust.

W planie określ odpowiedzialności:

- wspólnego rdzenia,
- instalatora Windows,
- instalatora Linux,
- helpera aktualizacyjnego,
- GUI.

## 11. PODPISY ED25519

Zachowaj zatwierdzone decyzje z `DESIGN_3_1_UPDATER_SIGNATURES.md`.

Tor `CRYPTO` powinien obejmować:

- `VERIFY-GITHUB-DIGEST`,
- `CRYPTO-MANIFEST-SCHEMA`,
- `CRYPTO-KEY-TABLE`,
- `CRYPTO-ED25519-VERIFY`,
- `CRYPTO-MANIFEST-VALIDATE`,
- `CRYPTO-SEMVER-POLICY`,
- `CRYPTO-DOWNGRADE-PROTECTION`,
- `CRYPTO-SIGNER-CLI`,
- `CRYPTO-KEY-MANAGEMENT`,
- `CRYPTO-KEY-ROTATION`,
- `CRYPTO-CI-SIGNING`.

Nie zmieniaj podstawowych decyzji:

- podpisywany jest manifest,
- podpis dotyczy dokładnych bajtów manifestu,
- podpis jest Base64 i po dekodowaniu ma 64 bajty,
- klucze publiczne są `[u8; 32]`,
- `key_id` jest sprawdzane po udanej weryfikacji,
- nieznana wersja manifestu jest odrzucana,
- błąd podpisu blokuje instalację,
- brak trybu „ostrzeżenie i kontynuuj”,
- wersja starsza lub równa jest odrzucana,
- pierwsze przejście starego klienta do pierwszej wersji z kluczem nie jest zabezpieczone przez stary klient.

## 12. PACKAGE_TYPE W MANIFEŚCIE

Dodaj do nadrzędnego planu zależność:

Schemat manifestu musi zostać sfinalizowany dopiero po ustaleniu faktycznych formatów Windows i Linux.

Każdy asset powinien docelowo zawierać:

- `platform`,
- `arch`,
- `package_type`,
- `filename`,
- `size`,
- `sha256`.

Nieznany `package_type` jest odrzucany.

Wybór artefaktu nie może opierać się wyłącznie na rozszerzeniu.

Dla danej instalacji musi istnieć dokładnie jedno dopasowanie:

`platform + arch + package_type`

Zero dopasowań oznacza brak aktualizacji dla tego kanału.

Wiele dopasowań oznacza niejednoznaczny manifest i jego odrzucenie.

## 13. ATOMOWOŚĆ, ROLLBACK I HEALTH CONFIRMATION

Wydziel wspólne zadania:

- `UPDATE-TRANSACTION-MODEL`,
- `UPDATE-ROLLBACK`,
- `UPDATE-HEALTH-CHECK`.

Model logiczny ma obejmować stany:

- Downloaded
- Verified
- Staged
- SwapPending
- Swapped
- LaunchPending
- Healthy
- RollbackRequired
- RolledBack
- Failed

Zaznacz:

- samo uruchomienie procesu nie potwierdza powodzenia aktualizacji,
- nowa wersja musi potwierdzić osiągnięcie bezpiecznego punktu inicjalizacji,
- brak potwierdzenia prowadzi do rollbacku,
- czas oczekiwania nie może być wybrany arbitralnie,
- staging i atomowy rename powinny znajdować się na tym samym systemie plików,
- danych użytkownika nie wolno objąć zamianą instalacji.

## 14. AUDYT JAKOŚCI

Przenieś wszystkie zadania jakościowe do osobnego toru:

`CQ`

Tor `CQ` może obejmować:

- SOTA,
- N1MM,
- kluby,
- callbook,
- WPX,
- AwardsEngine,
- Multi-Op,
- Voice Keyer,
- błędy async,
- SQLite,
- komentarze,
- nazewnictwo,
- martwy kod,
- małe duplikacje.

Nie duplikuj problemów updatera.

Jeżeli problem jakościowy dotyczy updatera, wskaż odpowiednie zadanie `UPDATE` lub `CRYPTO`.

## 15. DUŻE REFAKTORYZACJE

Przenieś duże zmiany do toru:

`ARCH`

Obejmuje to:

- dekompozycję `SpLogApp`,
- warstwę serwisową,
- wspólny potok QSO,
- normalizację 53-kolumnowego SQL,
- architekturę CAT,
- synchronizację P2P i LAN,
- pełną lokalizację,
- centralizację motywów.

Każdy `ARCH-*` musi mieć:

- osobny `DESIGN.md`,
- stan obecny,
- warianty,
- decyzję,
- plan migracji,
- plan testów,
- małe podzadania.

Nie wolno wykonywać `ARCH-*` podczas napraw krytycznych updatera.

## 16. RAPORTY WERYFIKACYJNE

Przed implementacją zaplanuj:

- `REPORT_CODE_QUALITY_P0_VERIFICATION.md`,
- `REPORT_3_1_A_GITHUB_DIGEST.md`,
- `DEAD_CODE_VERIFICATION_REPORT.md`,
- `UPDATE_ARTIFACT_INVENTORY_REPORT.md`.

`UPDATE_ARTIFACT_INVENTORY_REPORT.md` ma ustalić:

- wszystkie artefakty Windows,
- wszystkie artefakty Linux,
- nazwy plików,
- formaty,
- strukturę archiwów,
- architektury,
- sposób instalacji,
- lokalizację danych użytkownika,
- możliwość zapisu do katalogu instalacji,
- wymagane uprawnienia.

Raport nie zmienia kodu.

## 17. ETAPY NADRZĘDNEGO PLANU

Zbuduj `MASTER_IMPLEMENTATION_PLAN.md` z następujących etapów.

### ETAP M0: Inwentaryzacja i potwierdzenie stanu

Obejmuje wyłącznie raporty:

- P0,
- GitHub digest,
- artefakty Windows/Linux,
- martwy kod.

Brak modyfikacji kodu.

### ETAP M1: Awaryjne zabezpieczenia

Obejmuje:

- blokadę niszczącej aktualizacji Linux,
- blokadę niszczącej aktualizacji Windows, jeśli problem zostanie potwierdzony,
- wyłącznie minimalne poprawki bezpieczeństwa.

Nie implementuje jeszcze pełnego instalatora ani Ed25519.

### ETAP M2: Projekty instalatorów

Obejmuje utworzenie i zatwierdzenie:

- `DESIGN_3_0_A_WINDOWS_UPDATE.md`,
- `DESIGN_3_0_B_LINUX_UPDATE.md`,
- `DESIGN_3_0_C_PLATFORM_INSTALLERS.md`.

Brak implementacji przed zatwierdzeniem.

### ETAP M3: Implementacja instalatorów platformowych

Obejmuje:

- instalator Windows,
- instalator Linux dla faktycznie wspieranego formatu,
- wspólny interfejs,
- staging,
- rollback,
- health confirmation.

### ETAP M4: Schemat bezpiecznego wydania

Obejmuje:

- `package_type`,
- `manifest_version`,
- `key_id`,
- `product`,
- `channel`,
- `version`,
- `minimum_updater_version`,
- pełny commit SHA,
- `size`,
- `sha256`,
- `platform`,
- `arch`.

### ETAP M5: Kryptografia

Obejmuje:

- weryfikator Ed25519,
- parser manifestu,
- walidację kluczy,
- SemVer,
- downgrade,
- rotację kluczy,
- testy kryptograficzne.

### ETAP M6: Proces wydawniczy

Obejmuje:

- narzędzie podpisujące,
- GitHub Actions,
- Protected Environment,
- ręczne zatwierdzanie,
- minimalne uprawnienia,
- generowanie manifestu,
- podpis,
- publikację artefaktów.

### ETAP M7: Wydanie próbne

Osobno dla:

- Windows,
- Linux.

Nie używa produkcyjnego `/releases/latest`.

Obejmuje:

- test poprawnego wydania,
- podmieniony manifest,
- podmieniony podpis,
- podmieniony artefakt,
- zły rozmiar,
- przerwany transfer,
- złą architekturę,
- rollback,
- health confirmation.

### ETAP M8: Pierwsze wydanie publiczne

Dopiero po pomyślnych testach Windows i Linux.

### ETAP M9: Potwierdzone błędy funkcjonalne CQ

Realizacja pojedynczo według priorytetu.

Zadania P0/P1 niezwiązane z updaterem mogą zostać wykonane wcześniej, jeśli:

- nie zmieniają tych samych plików,
- nie zmieniają `Cargo.toml`,
- nie zmieniają workflow,
- nie kolidują z aktywnym zadaniem updatera.

### ETAP M10: Małe prace jakościowe

Obejmuje:

- komentarze,
- nazewnictwo,
- małe duplikacje,
- SPDX,
- diakrytykę,
- niewielkie optymalizacje.

### ETAP M11: Duże refaktoryzacje ARCH

Dopiero po zakończeniu krytycznych zmian updatera i stabilizacji testów.

## 18. KOLEJNOŚĆ I RÓWNOLEGŁOŚĆ

Dla każdego zadania określ:

- czy może być wykonywane równolegle,
- z jakimi zadaniami nie może być wykonywane,
- pliki będące punktem konfliktu,
- dokument nadrzędny,
- wymagany wcześniejszy etap.

Nie zezwalaj na równoległe zmiany w tym samym czasie w:

- `src/cloud/updater.rs`,
- `Cargo.toml`,
- `Cargo.lock`,
- `build-and-release.yml`,
- `src/gui/app.rs`,
- `src/core/database.rs`.

Jeżeli dwa zadania modyfikują ten sam krytyczny plik, wykonuj je sekwencyjnie.

## 19. WŁAŚCICIEL PROBLEMU

Każdy problem musi mieć dokładnie jeden tor nadrzędny:

- VERIFY,
- FIX,
- UPDATE,
- CRYPTO,
- CQ,
- ARCH,
- RELEASE.

Nie może istnieć drugi konkurencyjny task naprawiający ten sam problem.

Jeżeli problem pojawia się w kilku dokumentach:

- wybierz jeden tor nadrzędny,
- pozostałe miejsca oznacz jako referencje,
- nie twórz zduplikowanych commitów.

## 20. FORMAT KAŻDEGO ZADANIA

Każde zadanie w `MASTER_IMPLEMENTATION_PLAN.md` musi mieć:

- identyfikator,
- nazwę,
- tor,
- status techniczny,
- status wykonawczy,
- priorytet,
- źródło ustalenia,
- dokument nadrzędny,
- zakres,
- elementy wykluczone,
- przewidywane pliki,
- zależności,
- konflikty,
- ryzyko,
- zachowanie przed zmianą,
- zachowanie po zmianie,
- wymagane testy,
- test Windows,
- test Linux,
- test ręczny,
- wpływ na publiczne API,
- wpływ na format danych,
- kryterium ukończenia,
- sugerowany tytuł commita.

Każde zadanie ma odpowiadać jednej sesji i jednemu commitowi.

Jeżeli nie jest to możliwe, zadanie wymaga dalszego podziału albo osobnego `DESIGN.md`.

## 21. MACIERZ ŚLEDZENIA

Dodaj kompletną sekcję:

`Problem → źródło → weryfikacja → DESIGN → implementacja → test → wydanie`

Przykład:

```text
Problem:
Updater Linux nadpisuje binarkę archiwum tar.gz

Źródła:
AUDIT_REPORT.md
CODE_QUALITY_NORMALIZATION_PLAN.md
src/cloud/updater.rs

Weryfikacja:
VERIFY-UPDATE-ARTIFACTS

Plan nadrzędny:
UPDATE-LINUX

Awaryjna poprawka:
UPDATE-LINUX-SAFETY-GATE

Docelowy DESIGN:
DESIGN_3_0_B_LINUX_UPDATE.md
DESIGN_3_0_C_PLATFORM_INSTALLERS.md

Implementacja:
UPDATE-LINUX-TARGZ-INSTALLER

Test:
UPDATE-LINUX-ROLLBACK-TEST

Wydanie:
RELEASE-TRIAL-LINUX
```

Macierz ma obejmować wszystkie problemy z audytu i planu jakości.

## 22. KRYTERIA BRAMEK ETAPOWYCH

Każdy etap musi mieć bramkę wejścia i wyjścia.

Przykład:

### ETAP M5: Kryptografia

Wejście:

- zatwierdzony schemat manifestu,
- zatwierdzony `package_type`,
- działające instalatory platformowe,
- znana lista artefaktów,
- ustalone MSRV.

Wyjście:

- poprawny podpis jest akceptowany,
- błędny podpis jest odrzucany,
- zmieniony manifest jest odrzucany,
- nieznany klucz jest odrzucany,
- zła wersja klucza jest odrzucana,
- wszystkie testy przechodzą.

Nie pozwalaj przejść do kolejnego etapu bez spełnienia bramki wyjścia.

## 23. WALIDACJA PO KAŻDYM COMMICIE

Po każdym zadaniu implementacyjnym wymagaj:

- `cargo fmt --check`,
- `cargo check`,
- `cargo clippy --all-targets --all-features`,
- `cargo test`,
- nowych testów regresyjnych,
- listy zmodyfikowanych plików,
- podsumowania zachowania przed i po,
- wpływu na API,
- wpływu na format danych,
- testu platformowego, jeśli zadanie jest platformowe.

Dla zmian updatera wymagaj dodatkowo:

- testu niepełnego pobrania,
- testu złego rozmiaru,
- testu złego SHA-256,
- testu braku praw zapisu,
- testu rollbacku,
- testu zachowania danych użytkownika.

## 24. OCHRONA PRZED ROZSZERZANIEM ZAKRESU

Do każdego zadania dodaj sekcję:

`Poza zakresem`

Model nie może wykonywać poprawek „przy okazji”.

Jeżeli podczas pracy zostanie znaleziony nowy problem:

- zapisz go w sekcji obserwacji,
- nie implementuj,
- nie rozszerzaj aktywnego commita,
- zaproponuj osobne zadanie.

## 25. MIGRACJA ZE STARYCH PLANÓW

Dodaj sekcję pokazującą, jak stare zadania zostały przeniesione do nowego planu.

Dla każdej pozycji z:

- `PLAN.md`,
- `AUDIT_REPORT.md`,
- `CODE_QUALITY_NORMALIZATION_PLAN.md`,
- `DESIGN_3_1_UPDATER_SIGNATURES.md`

określ:

- nowy identyfikator,
- nowy etap,
- status,
- czy zadanie zostało zastąpione,
- czy zadanie pozostaje aktywne,
- czy zadanie jest rozwiązane,
- czy zadanie zostało uznane za fałszywy alarm.

Nie usuwaj informacji o już zakończonych etapach 2.2 i 2.3.

Oznacz je jako `DONE` i nie otwieraj ponownie bez potwierdzonej regresji.

## 26. STRUKTURA MASTER_IMPLEMENTATION_PLAN.md

Dokument końcowy ma zawierać:

# 1. Cel dokumentu

# 2. Hierarchia źródeł decyzji

# 3. Stan projektu i ukończone etapy

# 4. Statusy, priorytety i prefiksy

# 5. Zasady własności problemów

# 6. Mapa zależności

# 7. Etap M0: Raporty i weryfikacja

# 8. Etap M1: Awaryjne zabezpieczenia

# 9. Etap M2: Projekty instalatorów

# 10. Etap M3: Implementacja Windows i Linux

# 11. Etap M4: Schemat manifestu

# 12. Etap M5: Ed25519 i polityka wersji

# 13. Etap M6: CI i podpisywanie

# 14. Etap M7: Wydania próbne

# 15. Etap M8: Pierwsze wydanie publiczne

# 16. Etap M9: Poprawki funkcjonalne z audytu

# 17. Etap M10: Normalizacja jakości

# 18. Etap M11: Duże refaktoryzacje

# 19. Macierz śledzenia problemów

# 20. Migracja ze starych planów

# 21. Konflikty plików i zasady równoległości

# 22. Kryteria akceptacji

# 23. Otwarte decyzje właściciela projektu

# 24. Najbliższe następne zadanie

## 27. NAJBLIŻSZE NASTĘPNE ZADANIE

Na końcu dokumentu wybierz dokładnie jedno następne zadanie.

Nie może to być implementacja całego updatera.

Preferowana kolejność wyboru:

1. inwentaryzacja artefaktów Windows i Linux,
2. raport GitHub digest,
3. raport krytycznych problemów P0,
4. awaryjna bramka bezpieczeństwa,
5. dokumenty `DESIGN` instalatorów.

Podaj:

- identyfikator,
- cel,
- wejścia,
- wynik,
- pliki tylko do odczytu,
- plik wynikowy,
- zakaz implementacji.

## OGRANICZENIA KOŃCOWE

Nie zmieniaj kodu.

Nie zmieniaj istniejących dokumentów.

Nie usuwaj istniejących planów.

Nie aktualizuj statusów w `PLAN.md`.

Nie twórz raportów weryfikacyjnych.

Nie twórz dokumentów instalatorów.

Nie zmieniaj `Cargo.toml` ani `Cargo.lock`.

Nie zmieniaj workflow.

Nie wykonuj commitów.

Utwórz wyłącznie:

`MASTER_IMPLEMENTATION_PLAN.md`

## KONTROLA KOŃCOWA

Przed zakończeniem sprawdź:

1. Czy istnieje tylko jeden nadrzędny plan kolejności.
2. Czy dokumenty `DESIGN` zachowują rolę źródeł decyzji.
3. Czy updater ma jednego właściciela zmian.
4. Czy Windows i Linux mają osobne tory instalacji.
5. Czy wspólny rdzeń bezpieczeństwa jest współdzielony.
6. Czy awaryjna blokada Linux nie implementuje pełnego instalatora.
7. Czy `package_type` zależy od faktycznie wspieranych formatów.
8. Czy kryptografia nie jest wdrażana przed ustaleniem manifestu.
9. Czy każde zadanie ma jeden cel i jeden commit.
10. Czy zadania `CQ` nie duplikują zmian `UPDATE` lub `CRYPTO`.
11. Czy etapy 2.2 i 2.3 pozostają `DONE`.
12. Czy wszystkie problemy mają właściciela.
13. Czy istnieje macierz śledzenia.
14. Czy istnieją bramki wejścia i wyjścia.
15. Czy wybrano dokładnie jedno następne zadanie.
16. Czy nie zmodyfikowano żadnego istniejącego pliku.

Po utworzeniu `MASTER_IMPLEMENTATION_PLAN.md`:

1. Podsumuj strukturę etapów.
2. Wymień problemy, które miały konkurencyjnych właścicieli.
3. Wymień zadania przeniesione do toru `UPDATE/CRYPTO`.
4. Wymień zadania pozostawione w torze `CQ`.
5. Wymień zadania wymagające `DESIGN.md`.
6. Podaj jedno wybrane następne zadanie.
7. Nie rozpoczynaj tego zadania.
8. Zatrzymaj się.
