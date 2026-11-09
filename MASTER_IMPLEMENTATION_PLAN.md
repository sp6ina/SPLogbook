# MASTER_IMPLEMENTATION_PLAN.md â€” SPLogbook

**Utworzono:** 2026-09-29
**Wersja:** 2.0 (2026-11-07)
**Rola:** Jedyny nadrzÄ™dny plan organizacyjny projektu SPLogbook.

---

# 1. Cel dokumentu

Ten dokument jest jedynym nadrzÄ™dnym planem modernizacji SPLogbook. ĹÄ…czy ustalenia z:
- `AUDIT_REPORT.md` (historyczne ĹşrĂłdĹ‚o ustaleĹ„ audytu),
- `PLAN.md` (dotychczasowy plan etapowy z wykonanymi etapami 1.xâ€“2.x),
- `CODE_QUALITY_NORMALIZATION_PLAN.md` (szczegĂłĹ‚owy rejestr problemĂłw jakoĹ›ciowych, prefiksy CQ-),
- `DESIGN_2_2_FINAL.md` (zatwierdzona architektura docking GUI â€” DONE),
- `DESIGN_3_1_UPDATER_SIGNATURES.md` (zatwierdzona architektura Ed25519),
- projekt bezpiecznej aktualizacji Windows i Linux,
- wspĂłlny rdzeĹ„ weryfikacji aktualizacji.

Ĺ»aden z powyĹĽszych dokumentĂłw nie jest usuwany. Ich role:
- `MASTER_IMPLEMENTATION_PLAN.md` â†’ kolejnoĹ›Ä‡, zaleĹĽnoĹ›ci, statusy, etapy.
- `AUDIT_REPORT.md` â†’ historyczne obserwacje wymagajÄ…ce ponownej weryfikacji.
- `CODE_QUALITY_NORMALIZATION_PLAN.md` â†’ szczegĂłĹ‚owy rejestr problemĂłw CQ.
- `DESIGN_*.md` â†’ zatwierdzone decyzje architektoniczne.
- `REPORT_*.md` â†’ wyniki etapĂłw analitycznych.

---

# 2. Hierarchia ĹşrĂłdeĹ‚ decyzji

1. **Aktualny kod repozytorium** â€” ĹşrĂłdĹ‚o informacji o obecnym stanie implementacji.
2. **Zatwierdzone dokumenty DESIGN** â€” ĹşrĂłdĹ‚o obowiÄ…zujÄ…cych decyzji architektonicznych.
3. **MASTER_IMPLEMENTATION_PLAN.md** â€” kolejnoĹ›Ä‡, zaleĹĽnoĹ›ci, statusy.
4. **Raporty weryfikacyjne REPORT_*.md** â€” wyniki konkretnych analiz.
5. **CODE_QUALITY_NORMALIZATION_PLAN.md** â€” rejestr problemĂłw jakoĹ›ciowych.
6. **AUDIT_REPORT.md** â€” obserwacje historyczne, wymagajÄ…ce ponownej weryfikacji.

ReguĹ‚y konfliktĂłw:
- DESIGN ma pierwszeĹ„stwo przed rekomendacjÄ… audytu.
- Aktualny kod ma pierwszeĹ„stwo przy ustalaniu stanu faktycznego.
- NiejasnoĹ›Ä‡ wymaga raportu weryfikacyjnego.
- Nie wolno podejmowaÄ‡ arbitralnej decyzji podczas implementacji.

---

# 3. Stan projektu i ukoĹ„czone etapy

| Etap | Opis | Status | ĹąrĂłdĹ‚o |
|---|---|---|---|
| PLAN 1.1 | `row_to_qso` `.ok()` â†’ `?` | **DONE** | PLAN.md |
| PLAN 1.2 | Walidacja zakresu czasu HH:MM | **DONE** | PLAN.md |
| PLAN 1.3 | Indeksy `country` i `freq` | **DONE** | PLAN.md |
| PLAN 1.4 | Walidacja formatĂłw IOTA/SOTA/POTA | **DONE** | PLAN.md |
| PLAN 2.1 | Aktualizacja ADIF_VERSION | **DONE** | PLAN.md |
| PLAN 2.2 | Naprawa MODE/SUBMODE (DESIGN_2_2_FINAL) | **DONE** | PLAN.md, DESIGN_2_2_FINAL.md |
| PLAN 2.3 | Fallback pasma "20m" â†’ bĹ‚Ä…d | **DONE** | PLAN.md |

Etapy DONE nie podlegajÄ… ponownemu otwieraniu bez potwierdzonej regresji.

**Informacje o Ĺ›rodowisku:**
- Edition: 2024, MSRV: 1.85, Wersja: 1.1.0
- Artefakty CI: Windows = `SPLogbook-Windows-x64.zip`, Linux = `SPLogbook-Linux-x86_64.tar.gz`
- Brak: AppImage, deb, rpm, Flatpak, MSI, standalone .exe release
- Brak: `ed25519-dalek` w `Cargo.toml`
- CI nie uruchamia `cargo fmt` ani `cargo clippy`

---

# 4. Statusy, priorytety i prefiksy

## Prefiksy zadaĹ„

| Prefiks | Tor | Opis |
|---|---|---|
| `VERIFY-*` | Weryfikacja | Zadania analityczne i raporty |
| `FIX-*` | Poprawki | MaĹ‚e potwierdzone poprawki funkcjonalne |
| `UPDATE-*` | Aktualizator | Windows, Linux, wspĂłlny instalator |
| `CRYPTO-*` | Kryptografia | Manifest, Ed25519, klucze, podpisywanie |
| `CQ-*` | JakoĹ›Ä‡ kodu | Komentarze, nazewnictwo, duplikacje |
| `ARCH-*` | Architektura | DuĹĽe refaktoryzacje wymagajÄ…ce DESIGN.md |
| `RELEASE-*` | Wydania | PrĂłbne i publiczne wydania |

## Statusy techniczne

- POTWIERDZONY W KODZIE
- CZÄĹšCIOWO POTWIERDZONY
- ZAKOĹCZONY (DONE)
- WYMAGA TESTU URUCHOMIENIOWEGO
- NIEPOTWIERDZONY
- FAĹSZYWY ALARM
- ROZWIÄ„ZANY

## Statusy wykonawcze

- GOTOWY DO IMPLEMENTACJI
- ZABLOKOWANY DO CZASU RAPORTU
- ZABLOKOWANY DO CZASU DESIGN.md
- ZABLOKOWANY PRZEZ INNE ZADANIE
- ROZWIÄ„ZYWANY PRZEZ PLAN NADRZÄDNY
- TYLKO ZADANIE ANALITYCZNE
- NIE IMPLEMENTOWAÄ†
- DONE

Zadanie implementacyjne wymaga statusu wykonawczego = `GOTOWY DO IMPLEMENTACJI`.

---

# 5. Zasady wĹ‚asnoĹ›ci problemĂłw

KaĹĽdy problem ma dokĹ‚adnie jeden tor nadrzÄ™dny. Nie moĹĽe istnieÄ‡ drugi konkurencyjny task naprawiajÄ…cy ten sam problem.

**Kluczowa reguĹ‚a koordynacji updatera:** Wszystkie zmiany dotyczÄ…ce `src/cloud/updater.rs`, wyboru artefaktĂłw, pobierania, instalacji Windows/Linux, SHA-256, manifestu, Ed25519, SemVer, GitHub Actions i procesu wydawniczego naleĹĽÄ… do toru `UPDATE + CRYPTO + RELEASE`. Zadania `CQ` nie mogÄ… rĂłwnolegle modyfikowaÄ‡ tych elementĂłw.

Problem `PROB-P0-03` (updater Linux) z audytu jakoĹ›ci â†’ **ROZWIÄ„ZYWANY PRZEZ PLAN NADRZÄDNY** (tor UPDATE).

---

# 6. Mapa zaleĹĽnoĹ›ci

```
VERIFY-ARTIFACTS â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
VERIFY-P0 â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
VERIFY-GITHUB-DIGEST â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
                                                      â–Ľ
                                              ETAP M1: Safety Gates
                                              UPDATE-WIN-SAFETY-GATE
                                              UPDATE-LINUX-SAFETY-GATE
                                                      â”‚
                                                      â–Ľ
                                              ETAP M2: DESIGN docs
                                              DESIGN_3_0_A/B/C
                                                      â”‚
                                                      â–Ľ
                                              ETAP M3: Instalatory
                                              UPDATE-WIN-INSTALLER
                                              UPDATE-LINUX-INSTALLER
                                                      â”‚
                                                      â–Ľ
                                              ETAP M4: Schemat manifestu
                                              CRYPTO-MANIFEST-SCHEMA
                                                      â”‚
                                                      â–Ľ
                                              ETAP M5: Kryptografia
                                              CRYPTO-ED25519-VERIFY
                                                      â”‚
                                                      â–Ľ
                                              ETAP M6: CI signing
                                              CRYPTO-CI-SIGNING
                                                      â”‚
                                                      â–Ľ
                                              ETAP M7: Wydania prĂłbne
                                                      â”‚
                                                      â–Ľ
                                              ETAP M8: Wydanie publiczne

    (RĂłwnolegle z M1-M8, jeĹ›li brak konfliktu plikĂłw:)
    CQ-0 â†’ CQ-1.x â†’ CQ-2.x â†’ CQ-3.x â†’ CQ-4.x
    ARCH-* (po M8 i stabilizacji)
```

---

# 7. Etap M0: Raporty i weryfikacja

Brak modyfikacji kodu. WyĹ‚Ä…cznie analiza.

#### VERIFY-ARTIFACTS â€” Inwentaryzacja artefaktĂłw Windows i Linux
- **Tor:** VERIFY
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P0 Krytyczny
- **ĹąrĂłdĹ‚o:** MASTER_IMPLEMENTATION_PLAN.md
- **Zakres:** UstaliÄ‡ faktyczne formaty, nazwy, struktury archiwĂłw, lokalizacje binarek, dane uĹĽytkownika, uprawnienia zapisu. ZbadaÄ‡ `build-and-release.yml` i istniejÄ…ce release'y GitHub.
- **Poza zakresem:** Zmiana kodu, zmiana workflow.
- **Pliki (read-only):** `.github/workflows/build-and-release.yml`, `src/cloud/updater.rs`
- **Plik wynikowy:** `UPDATE_ARTIFACT_INVENTORY_REPORT.md`
- **ZaleĹĽnoĹ›ci:** brak
- **Konflikty:** brak
- **Kryterium ukoĹ„czenia:** Raport zawiera peĹ‚nÄ… listÄ™ artefaktĂłw obu platform z dokĹ‚adnymi nazwami i strukturÄ….

#### VERIFY-P0 â€” Weryfikacja problemĂłw krytycznych P0
- **Tor:** VERIFY
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P0 Krytyczny
- **ĹąrĂłdĹ‚o:** CODE_QUALITY_NORMALIZATION_PLAN.md (CQ-0)
- **Zakres:** Ponowna weryfikacja: SOTA Utc::now(), N1MM jednostki/pasma, updater Linux, updater Windows (nowo odkryty!), liczniki zawodĂłw. Sprawdzenie osiÄ…galnoĹ›ci, przepĹ‚ywu wykonania, wpĹ‚ywu na dane.
- **Poza zakresem:** Zmiana kodu.
- **Pliki (read-only):** `src/core/sota_export.rs`, `src/digital/n1mm.rs`, `src/cloud/updater.rs`, `src/gui/app.rs`
- **Plik wynikowy:** `REPORT_CODE_QUALITY_P0_VERIFICATION.md`
- **ZaleĹĽnoĹ›ci:** brak
- **Kryterium ukoĹ„czenia:** Raport z ostatecznym priorytetem kaĹĽdego P0.

#### VERIFY-GITHUB-DIGEST â€” Analiza GitHub Digest API
- **Tor:** VERIFY
- **Status techniczny:** ZAKOĹCZONY (DONE)
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** DESIGN_3_1_UPDATER_SIGNATURES.md
- **Zakres:** Ustalenie, czy GitHub API dostarcza digest dla wĹ‚asnych release assets i w jakim formacie.
- **Poza zakresem:** Implementacja.
- **Plik wynikowy:** `REPORT_3_1_A_GITHUB_DIGEST.md`
- **ZaleĹĽnoĹ›ci:** brak

#### VERIFY-DEAD-CODE â€” Raport uĹĽycia symboli
- **Tor:** VERIFY
- **Status techniczny:** WYMAGA TESTU URUCHOMIENIOWEGO
- **Status wykonawczy:** TYLKO ZADANIE ANALITYCZNE
- **Priorytet:** P2 Ĺšredni
- **ĹąrĂłdĹ‚o:** CODE_QUALITY_NORMALIZATION_PLAN.md (CQ-3.1.A)
- **Plik wynikowy:** `DEAD_CODE_VERIFICATION_REPORT.md`
- **ZaleĹĽnoĹ›ci:** brak

---

# 8. Etap M1: Awaryjne zabezpieczenia

Minimalne poprawki zapobiegajÄ…ce niszczÄ…cej aktualizacji.

#### UPDATE-LINUX-SAFETY-GATE â€” Blokada niszczÄ…cej aktualizacji Linux
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P0 Krytyczny
- **ĹąrĂłdĹ‚o:** CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P0-03), AUDIT_REPORT.md
- **Zakres:** ZablokowaÄ‡ nadpisanie binarki plikiem .tar.gz/.deb/.rpm. Komunikat o rÄ™cznej aktualizacji. Jawny bĹ‚Ä…d dla brakujÄ…cego tag_name. UsuniÄ™cie pliku tymczasowego. Ĺ»adnego sudo, rozpakowywania, instalowania deb.
- **Poza zakresem:** PeĹ‚ny instalator Linux. Rozpakowywanie tar.gz. Ed25519.
- **Pliki:** `src/cloud/updater.rs`
- **ZaleĹĽnoĹ›ci:** VERIFY-ARTIFACTS
- **Konflikty:** Nie moĹĽe byÄ‡ rĂłwnolegĹ‚y z ĹĽadnym innym zadaniem UPDATE-*.
- **Zachowanie przed:** `rename(tar.gz, splogbook)` â†’ uszkodzenie instalacji.
- **Zachowanie po:** Przerwanie aktualizacji z komunikatem; binarka nienaruszona.
- **Testy:** Test jednostkowy: artefakt .tar.gz â†’ Err. Test: brak tag_name â†’ Err.
- **Test Linux:** TAK (wymagany)
- **WpĹ‚yw na API:** brak
- **WpĹ‚yw na format danych:** brak
- **Sugerowany tytuĹ‚ commita:** `fix(updater): block destructive Linux self-replace with archive files`

#### UPDATE-WIN-SAFETY-GATE â€” Blokada niszczÄ…cej aktualizacji Windows
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P0 Krytyczny
- **ĹąrĂłdĹ‚o:** MASTER_IMPLEMENTATION_PLAN.md (nowo odkryty problem â€” artefakt Windows to ZIP!)
- **Zakres:** ZablokowaÄ‡ nadpisanie SPLogbook.exe plikiem .zip. Komunikat bĹ‚Ä™du. Obecny skrypt PowerShell przenosi surowy ZIP w miejsce EXE â€” to ten sam bĹ‚Ä…d co na Linux.
- **Poza zakresem:** PeĹ‚ny instalator Windows. Rozpakowanie ZIP.
- **Pliki:** `src/cloud/updater.rs`
- **ZaleĹĽnoĹ›ci:** VERIFY-ARTIFACTS
- **Konflikty:** Nie moĹĽe byÄ‡ rĂłwnolegĹ‚y z UPDATE-LINUX-SAFETY-GATE (ten sam plik).
- **Zachowanie przed:** `Move-Item zip â†’ exe` â†’ uszkodzenie.
- **Zachowanie po:** Przerwanie z komunikatem; EXE nienaruszone.
- **Testy:** Test: artefakt .zip â†’ Err.
- **Test Windows:** TAK (wymagany)
- **Sugerowany tytuĹ‚ commita:** `fix(updater): block destructive Windows self-replace with ZIP archives`

---

# 9. Etap M2: Projekty instalatorĂłw

Utworzenie, uzgodnienie i zweryfikowanie trzech kompletnych dokumentĂłw projektowych M2 (DESIGN-GATE). Brak zmian w kodzie.

#### UPDATE-DESIGN-WIN â€” Projekt instalatora Windows
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W DOKUMENCIE PROJEKTOWYM
- **Status wykonawczy:** APPROVED (Formalnie zatwierdzony w STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE po decyzjach OWNER-M2-01, 02, 03)
- **Priorytet:** P0 Krytyczny
- **Plik wynikowy:** DESIGN_3_0_A_WINDOWS_UPDATE.md (v2.1.0)
- **Zakres:** Format wydania (ZIP), ochrona przed Zip Slip, staging (.staging), wyĹ‚Ä…czenie procesu (PID loop), tablice literaĹ‚Ăłw PowerShell bez wildcardĂłw, rollback (.rollback), health confirmation (--check-health-startup, .health_ok), zachowanie baz i konfiguracji.

#### UPDATE-DESIGN-LINUX â€” Projekt instalatora Linux
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W DOKUMENCIE PROJEKTOWYM
- **Status wykonawczy:** APPROVED (Formalnie zatwierdzony w STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE po decyzjach OWNER-M2-04, 05, 06, 07)
- **Priorytet:** P0 Krytyczny
- **Plik wynikowy:** DESIGN_3_0_B_LINUX_UPDATE.md (v2.1.0)
- **Zakres:** Format tar.gz (oficjalne wydanie portable), ekstrakcja do .staging, spĹ‚aszczanie pojedynczego katalogu nadrzÄ™dnego, ochrona przed traversal i nodami specjalnymi, uprawnienia 0o755, RUNPATH, biblioteki Hamlib, atomowe rename, rollback, health confirmation, odmowa dla /usr/bin bez sudo/pkexec.

#### UPDATE-DESIGN-PLATFORM â€” WspĂłlny interfejs instalatorĂłw
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W DOKUMENCIE PROJEKTOWYM
- **Status wykonawczy:** APPROVED (Formalnie zatwierdzony w STAGE-M2-OWNER-DECISIONS-APPROVAL-GATE po zatwierdzeniu 9 kluczowych decyzji OWNER-M2-01..07, 13, 15)
- **Priorytet:** P0 Krytyczny
- **Plik wynikowy:** DESIGN_3_0_C_PLATFORM_INSTALLERS.md (v2.1.0)
- **Zakres:** Czterowarstwowa architektura, deterministyczna maszyna stanĂłw, cecha PlatformInstallerAdapter, model transakcji UpdateAttemptId, klasyfikacja UpdateFileClass, zasada UNKNOWN DATA CLASSIFICATION â†’ DO NOT OVERWRITE, blokada wspĂłĹ‚bieĹĽnoĹ›ci .update.lock, test matrix M3.

---

# 10. Etap M3: Implementacja Windows i Linux

- **M3 STATUS:** **IN_PROGRESS**
- **CURRENT PUBLIC STABLE:** 1.2.0 (reprezentowane przez niezmieniony tag 1.2.0)
- **CURRENT DEVELOPMENT:** 1.3.0-alpha.1 (aktywny kod źródłowy, centralny BuildIdentity)
- **PUBLIC RELEASE STATUS:** NOT PUBLISHED (brak tagów, brak publikacji, brak zewnętrznych kluczy)
- **M3-A STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (Implementacja wspolnego rdzenia platformowego, modelu transakcji, 20-stanowego automatu i mechanizmow recovery aktualizatora SPLogbook zamknieta w STAGE-M3-A-PLATFORM-CORE-IMPLEMENTATION-GATE)
- **M3-B STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (Implementacja natywnego pomocnika Windows splogbook-updater.exe, protokolu IPC, weryfikacji HealthProof, rollbacku i utwardzonego fallbacku PowerShell zamknieta w STAGE-M3-B-WINDOWS-HELPERS-IMPLEMENTATION-GATE oraz formalnie potwierdzona w STAGE-M3-B-CLOSURE-VERIFICATION-GATE)
- **M3-C STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (Produkcyjna integracja Windows Portable ZIP ze zweryfikowanym manifestem, UpdatePlan, natywnym helperem Windows i utwardzonym fallbackiem PowerShell zamknieta w STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE)
- **M3-DEV-VERSION-IDENTITY STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (Eliminacja niejednoznaczności wersji, wprowadzenie wersji rozwojowej 1.3.0-alpha.1, centralnego BuildIdentity, wczesnych flag CLI --version/--build-info/--diagnostic-version, weryfikacji tożsamości HealthProof oraz ewaluacji kanałów i SemVer 2.0 zamknieta w STAGE-M3-DEV-VERSION-IDENTITY-GATE)
- **M3-D PREREQUISITES STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (I18N i terminologia: COMPLETED_SOFTWARE_VERIFIED; Środowisko Inno Setup: INNO_SETUP_7_1_1_APPROVED_FOR_M3_D na mocy decyzji OWNER-M3-D-TOOLCHAIN-01 — raport STAGE_M3_D_PREREQUISITES_REMEDIATION_REPORT.md)
- **M3-D STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (Integracja Windows User Setup / Inno Setup 7.1.1, obsługa modelu SystemManaged, ochrona danych użytkownika, brak uprawnień administratora, zamknięta w STAGE-M3-D-WINDOWS-USER-SETUP-INTEGRATION-GATE)
- **M3-E STATUS:** **COMPLETED_SOFTWARE_VERIFIED** (Integracja Windows System Setup / MSI / WiX Toolset 7.0.1, model SystemManaged, per-machine, UpgradeCode {B7D14532-6A10-4F8A-9A4B-3DE189E502B0}, ochrona danych użytkownika, msiexec delegation, zamknięta w STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE; spójność taga v1.2.0, kanonicznych liczników testów oraz izolacja polityki wersjonowania przyszłych pakietów MSI prerelease potwierdzona w STAGE-M3-E-CLOSURE-CONSISTENCY-GATE)
- **M3-F STATUS:** **COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED** (Wielodystrybucyjna integracja Linux Portable tar.gz, polityka wsparcia Linux 7.0 / glibc 2.43, 4 wspierane dystrybucje, dwufazowa bezpieczna ekstrakcja, aktualizacje Self-Managed JOURNALED_TRANSACTIONAL_REPLACEMENT, zgodność z XDG Base Directory i ochrona baz SQLite/WAL/SHM, weryfikacja pojedynczego kanonicznego artefaktu o identycznym SHA-256 5df6ac20dd9717fb568567d53d7566758fa62ec507b1a9cba446c39ea346b7ac na macierzy 7 obrazów OCI (legacy, nie fail-closed); macierz wspierana od 2026-11-09: 4 profile w GitHub Actions Run ID 42518873662; zamknięta w STAGE-M3-F-NATIVE-LINUX-EVIDENCE-AND-SNAPSHOT-CORRECTION-GATE)
- **Dalsze etapy M3 (instalatory Windows i pakiety Linux):**
  - M3-D: Integracja Windows User Setup / Inno Setup (COMPLETED_SOFTWARE_VERIFIED)
  - M3-E: Integracja Windows System Setup / MSI (COMPLETED_SOFTWARE_VERIFIED)
  - M3-F: Integracja Linux Portable tar.gz (COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED; polityka wsparcia Linux 7.0 / glibc 2.43 zamknieta w STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE jako COMPLETED_POLICY_ALIGNMENT_VERIFIED)
  - M3-G: Integracja Linux AppImage
  - M3-H: Integracja Linux DEB
  - M3-I: Integracja Linux RPM
  - M3-J: Integracja Linux Flatpak

#### UPDATE-CORE-PLATFORM (M3-A) — Wspolny rdzen transakcyjny aktualizatora
- **Tor:** UPDATE
- **Status wykonawczy:** **COMPLETED_SOFTWARE_VERIFIED**
- **Zaleznosci:** UPDATE-DESIGN-PLATFORM (APPROVED), decyzje OWNER-M2-01..15 (APPROVED)
- **Zakres:** UpdateAttemptId, UpdateStateMachine (20 stanow), UpdateTransaction, InstallationType (9 formatow), InstallationCapabilities, PlatformInstallerAdapter, DiskPreflightRequirements, blokada concurrency .update.lock, centralna polityka anulowania, RecoveryDecisionEngine, kontrakt dowodu zdrowia HealthProof, security guards.

#### UPDATE-WIN-HELPERS (M3-B) — Pomocnicy Windows i PowerShell
- **Tor:** UPDATE
- **Status wykonawczy:** **COMPLETED_SOFTWARE_VERIFIED** (Natywny helper splogbook-updater.exe oraz utwardzony fallback PowerShell)
- **Zaleznosci:** UPDATE-DESIGN-WIN (APPROVED), UPDATE-DESIGN-PLATFORM (APPROVED), UPDATE-CORE-PLATFORM (COMPLETED_SOFTWARE_VERIFIED), decyzje OWNER-M2-01..03 (APPROVED)
- **Zakres:** Natywny helper Windows splogbook-updater.exe, wersjonowany kontrakt IPC WindowsHelperRequest/Result, integralnosc plan_digest, monitor procesu macierzystego, dziennik operacji HelperOperationJournal z semantyką JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED, strukturalne uruchamianie kandydata w CREATE_NO_WINDOW, weryfikacja HealthProof, transakcyjny rollback, utwardzony generator PowerShell bez wildcardow i iex, pelna parzystosc bezpieczenstwa, testy Windows (28 testow natywnych/PowerShell), strazniki bezpieczenstwa (83 testy). Formalnie potwierdzony w STAGE-M3-B-CLOSURE-VERIFICATION-GATE.

#### UPDATE-WIN-PORTABLE-ZIP (M3-C) — Integracja Windows Portable ZIP
- **Tor:** UPDATE
- **Status wykonawczy:** **COMPLETED_SOFTWARE_VERIFIED**
- **Zaleznosci:** UPDATE-DESIGN-WIN (APPROVED), UPDATE-DESIGN-PLATFORM (APPROVED), UPDATE-CORE-PLATFORM (COMPLETED_SOFTWARE_VERIFIED), UPDATE-WIN-HELPERS (COMPLETED_SOFTWARE_VERIFIED), decyzje OWNER-M2-01..03 (APPROVED), decyzje OWNER-M2-08, 09 (OPEN, NON-BLOCKING — TEMPORARY CONSERVATIVE RETENTION POLICY), decyzja OWNER-M2-10 (APPROVED)
- **Zakres:** Produkcyjna integracja adaptera Windows Portable ZIP ze zweryfikowanym manifestem Ed25519, bezpieczną inspekcją i ekstrakcją ZIP (`inspect_windows_zip_archive`), wyznaczaniem `UpdatePlan`, obliczaniem `plan_digest`, walidacją tożsamości PE AMD64 natywnego pomocnika `splogbook-updater.exe`, utwardzonym fallbackiem PowerShell, procedurą startową `consume_update_result_on_startup` z pomiarem zachowanego `.rollback`, bezwzględną ochroną baz SQLite (`serviceLOG.db`), WAL, SHM, konfiguracji i danych użytkownika. Zamknięty w STAGE-M3-C-WINDOWS-PORTABLE-ZIP-INTEGRATION-GATE (2556 testów passed, 0 failed, 91 security guards, 28 Windows helper tests).

#### UPDATE-WIN-USER-SETUP (M3-D) — Integracja Windows User Setup (Inno Setup 7.1.1)
- **Tor:** UPDATE
- **Status wykonawczy:** **COMPLETED_SOFTWARE_VERIFIED**
- **Zaleznosci:** OWNER-M3-D-TOOLCHAIN-01 (APPROVED), UPDATE-DESIGN-WIN (APPROVED), UPDATE-DESIGN-PLATFORM (APPROVED), UPDATE-CORE-PLATFORM (COMPLETED_SOFTWARE_VERIFIED)
- **Zakres:** Integracja instalatora Windows User Setup per-user opartego o Inno Setup 7.1.1 (`C:\Program Files\Inno Setup 7\ISCC.exe`), AppId `{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}`, instalacja do `{localappdata}\Programs\SPLogbook`, `PrivilegesRequired=lowest` (CurrentUser, no admin, no UAC), `SetupArchitecture=x64`, `ArchitecturesAllowed=x64compatible`, `ArchitecturesInstallIn64BitMode=x64compatible`. Wdrożono bezpieczny model SystemManaged, delegację aktualizacji z flagą `/NORESTART` bez powłoki shell, markery instalacyjne `.splogbook-inno-marker`, bezwzględną ochronę baz danych SQLite, plików WAL, SHM, konfiguracji i nieznanych plików użytkownika (polityki `uninsneveruninstall`, `onlyifdoesntexist`, `dirifempty`, zakaz wildcardów i DelTree({app})). Zamknięty w STAGE-M3-D-WINDOWS-USER-SETUP-INTEGRATION-GATE (108 security guards passed, 14 inno update tests passed, 7 user data safety tests passed).

#### UPDATE-WIN-SYSTEM-SETUP (M3-E) — Integracja Windows System Setup (WiX Toolset 7.0.1 / MSI)
- **Tor:** UPDATE
- **Status wykonawczy:** **COMPLETED_SOFTWARE_VERIFIED**
- **Zaleznosci:** OWNER-M3-E-TOOLCHAIN-01 (APPROVED), UPDATE-DESIGN-WIN (APPROVED), UPDATE-DESIGN-PLATFORM (APPROVED), UPDATE-CORE-PLATFORM (COMPLETED_SOFTWARE_VERIFIED)
- **Zakres:** Integracja instalatora Windows System Setup per-machine opartego o WiX Toolset SDK 7.0.1 (`installer/windows/msi/SPLogbook.wixproj`), stały UpgradeCode `{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}` (packed `0B205E981ED3B4A9A9B4A9E189E502B0`), instalacja do `ProgramFiles64Folder\SPLogbook`, `Scope="perMachine"` (kontrolowane podniesienie uprawnień przez Windows Installer UAC), architektura x64 (`Bitness="always64"`). Wdrożono bezpieczny model SystemManaged, delegację aktualizacji, naprawy i deinstalacji przez `msiexec.exe` z logowaniem i flagą `/norestart` bez powłoki shell, markery instalacyjne `.splogbook-msi-marker`, bezwzględną ochronę baz danych SQLite, plików WAL, SHM, konfiguracji i nieznanych plików użytkownika (polityki `NeverOverwrite="yes"`, brak wildcardów i RemoveFolderEx). Wzajemne wykrywanie konfliktów z Inno Setup i Portable ZIP. Zamknięty w STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE (118 security guards passed, 18 windows msi tests passed, 6 user data safety tests passed). W STAGE-M3-E-CLOSURE-CONSISTENCY-GATE potwierdzono tożsamość taga v1.2.0 (obiekt taga `3ea8d51a...`, peeled commit `6f7d3aaa...`), kanoniczne liczniki testów (2663 registered, 2653 passed, 0 failed, 10 ignored HIL) oraz wyodrębniono propozycję decyzji wersjonowania przyszłych pakietów MSI prerelease (OWNER-M3-E-MSI-VERSIONING-01).

#### UPDATE-LINUX-PORTABLE-TAR-GZ (M3-F) — Integracja Linux Portable tar.gz
- **Tor:** UPDATE
- **Status wykonawczy:** **COMPLETED_NATIVE_LINUX_EVIDENCE_VERIFIED**
- **Zaleznosci:** OWNER-M3-F-LINUX-BASELINE-01..OWNER-M3-F-DESKTOP-INTEGRATION-01 (APPROVED), UPDATE-DESIGN-LINUX (APPROVED), UPDATE-DESIGN-PLATFORM (APPROVED), UPDATE-CORE-PLATFORM (COMPLETED_SOFTWARE_VERIFIED)
- **Zakres:** Integracja kanonicznego pakietu Linux x86_64 portable `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz` o pojedynczym katalogu głównym `SPLogbook/`, identycznym SHA-256 dla wszystkich dystrybucji, markera metadanych `.splogbook-linux-portable-marker`, dwufazowej bezpiecznej ekstrakcji (`extract_linux_portable_tar_gz_two_phase` z fazą 1 read-only preflight, ochroną przed symlinkami/hardlinkami/urządzeniami/setuid/setgid/bombami), aktualizacji Self-Managed w semantyce JOURNALED_TRANSACTIONAL_REPLACEMENT (z kopią zapasową w `.rollback`, odizolowanym stagingiem, walidacją HealthProof), pełnej zgodności z XDG Base Directory (`src/core/xdg.rs`) z egzekwowaniem ścieżek bezwzględnych, ochroną baz SQLite (`serviceLOG.db`), WAL, SHM, konfiguracji i eksportów użytkownika oraz bezpiecznym skryptem budowania `scripts/build_linux_portable_tar_gz.sh`. Zamknięty w STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE oraz zweryfikowany natywnie w STAGE-M3-F-NATIVE-LINUX-EVIDENCE-AND-SNAPSHOT-CORRECTION-GATE (potwierdzono identyczny SHA-256 `5df6ac20dd9717fb568567d53d7566758fa62ec507b1a9cba446c39ea346b7ac` na 7 natywnych obrazach OCI w CI run 42518873662; 2698 testów zarejestrowanych, 2688 wykonanych i zaliczonych, 0 błędów, 10 ignorowanych HIL).

#### UPDATE-LINUX-PACKAGES (Dalsze etapy M3: M3-G..M3-J) — Pakiety Linux (AppImage, DEB, RPM, Flatpak)
- **Tor:** UPDATE
- **Status wykonawczy:** PENDING (Dalsza kolejność po instalatorach Windows)
- **Zaleznosci:** UPDATE-DESIGN-LINUX (APPROVED), UPDATE-DESIGN-PLATFORM (APPROVED), UPDATE-CORE-PLATFORM (COMPLETED_SOFTWARE_VERIFIED), decyzje OWNER-M2-04..07, 11..12 (APPROVED)
---

# 11. Etap M4: Schemat manifestu

#### CRYPTO-MANIFEST-SCHEMA â€” Schemat release-manifest.json
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **ĹąrĂłdĹ‚o:** DESIGN_3_1_UPDATER_SIGNATURES.md
- **Zakres:** manifest_version, product, channel, version (SemVer), minimum_updater_version, commit (40 hex), assets z platform+arch+package_type+filename+size+sha256.

---

# 12. Etap M5: Ed25519 i polityka wersji

#### CRYPTO-KEY-TABLE â€” Tabela zaufanych kluczy publicznych
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **ĹąrĂłdĹ‚o:** DESIGN_3_1_UPDATER_SIGNATURES.md

#### CRYPTO-ED25519-VERIFY â€” Weryfikator podpisĂłw Ed25519
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Zakres:** Pobranie manifestu i .sig, dekodowanie Base64, weryfikacja Ed25519, sprawdzenie key_id, odrzucenie nieznanych manifest_version.
- **Pliki:** `src/cloud/updater.rs`, `Cargo.toml` (dodanie ed25519-dalek)

#### CRYPTO-MANIFEST-VALIDATE â€” Walidacja zawartoĹ›ci manifestu
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)

#### CRYPTO-SEMVER-POLICY â€” Polityka wersji i ochrona przed downgrade
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)

#### CRYPTO-DOWNGRADE-PROTECTION â€” Odrzucanie starszych wersji
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)

---

# 13. Etap M6: CI i podpisywanie

#### CRYPTO-SIGNER-CLI â€” NarzÄ™dzie podpisujÄ…ce offline
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)

#### CRYPTO-KEY-MANAGEMENT â€” ZarzÄ…dzanie kluczami
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)

#### CRYPTO-KEY-ROTATION â€” Rotacja kluczy
- **Tor:** CRYPTO
- **Status wykonawczy:** ZAKOĹCZONY (DONE)

#### CRYPTO-CI-SIGNING â€” Integracja z GitHub Actions
- **Tor:** CRYPTO/RELEASE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Pliki:** `.github/workflows/build-and-release.yml`

---

# 14. Etap M7: Wydania prĂłbne

#### RELEASE-TRIAL-WIN â€” Wydanie prĂłbne Windows
- **Tor:** RELEASE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Zakres:** Test poprawnego wydania, podmieniony manifest/podpis/artefakt, zĹ‚y rozmiar, przerwany transfer, rollback, health confirmation.
- **Test Windows:** TAK

#### RELEASE-TRIAL-LINUX â€” Wydanie prĂłbne Linux
- **Tor:** RELEASE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Test Linux:** TAK

---

# 15. Etap M8: Pierwsze wydanie publiczne

#### RELEASE-PUBLIC-V1 â€” Pierwsze wydanie z podpisami
- **Tor:** RELEASE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **ZaleĹĽnoĹ›ci:** RELEASE-TRIAL-WIN OK, RELEASE-TRIAL-LINUX OK

---

# 16. Etap M9: Poprawki funkcjonalne z audytu

Zadania P0/P1 niezwiÄ…zane z updaterem mogÄ… byÄ‡ wykonywane rĂłwnolegle z torem UPDATE, pod warunkiem braku konfliktu plikĂłw.

#### FIX-SOTA-FALLBACK (CQ-1.1)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P0 Krytyczny
- **ĹąrĂłdĹ‚o:** CQ: PROB-P0-01
- **Pliki:** `src/core/sota_export.rs`
- **Konflikty:** brak z UPDATE
- **Sugerowany tytuĹ‚ commita:** `fix(sota): return error instead of Utc::now() fallback on bad QSO date`

#### FIX-N1MM-BAND (CQ-1.2.A)
- **Tor:** FIX
- **Status techniczny:** ZAKOĹCZONY (DONE)
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet tymczasowy:** P1 Wysoki (potencjalnie P0)
- **ĹąrĂłdĹ‚o:** CQ: PROB-N1MM-A
- **Pliki:** `src/digital/n1mm.rs`

#### FIX-N1MM-FREQ (CQ-1.2.B)
- **Tor:** FIX
- **Status techniczny:** ZAKOĹCZONY (DONE)
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet tymczasowy:** P1 Wysoki (potencjalnie P0)
- **ĹąrĂłdĹ‚o:** CQ: PROB-N1MM-B
- **Pliki:** `src/digital/n1mm.rs`

#### FIX-CONTEST-COUNTERS (CQ-1.4)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-01
- **Pliki:** `src/gui/app.rs`
- **Konflikty:** Plik `app.rs` â€” nie moĹĽe byÄ‡ rĂłwnolegĹ‚y z ARCH-SPLOGAPP

#### FIX-CLUBS-HEURISTIC (CQ-2.1.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-02
- **Pliki:** `src/core/clubs.rs`
- **Konflikty:** brak
- **Sugerowany tytuĹ‚ commita:** `fix(clubs): remove incorrect ends_with("CW") heuristic`

#### FIX-CALLBOOK-DEMO (CQ-2.1.B)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-02
- **Pliki:** `src/core/callbook.rs`
- **Sugerowany tytuĹ‚ commita:** `fix(callbook): remove hardcoded demo data`

#### FIX-WPX-PREFIX (CQ-2.2.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-03A
- **Pliki:** `src/core/prefix.rs`, `src/core/awards.rs`

#### FIX-SP-DISTRICT (CQ-2.2.B)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-03B
- **Pliki:** `src/core/awards.rs`

#### FIX-AWARDS-DOUBLE-REG (CQ-2.2.C)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-03C
- **Pliki:** `src/core/awards.rs`

#### FIX-MULTIOP-FALLBACK (CQ-2.3.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-04
- **Pliki:** `src/gui/contest.rs`

#### FIX-VOICEKEYER-CAT (CQ-2.3.B)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-05
- **Pliki:** `src/gui/voice_keyer.rs`

#### FIX-ASYNC-SEND-SYNC (CQ-2.4.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOĹCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-07
- **Pliki:** `src/cloud/lotw.rs`, `src/cloud/qrz.rs`, `src/cloud/solar.rs`

---

# 17. Etap M10: Normalizacja jakoĹ›ci

Wszystkie zadania CQ-2.4.B do CQ-4.4 z `CODE_QUALITY_NORMALIZATION_PLAN.md`.

PeĹ‚na lista zadaĹ„ (z prefiksem CQ-):
- CQ-2.4.B: Audyt rows.flatten() (polityki)
- CQ-2.4.C1â€“C4: ObsĹ‚uga bĹ‚Ä™dĂłw SQLite per moduĹ‚
- CQ-3.1.B1â€“B4: Usuwanie martwego kodu (po raporcie CQ-3.1.A / VERIFY-DEAD-CODE)
- CQ-3.3.Aâ€“E: Wrappery, command_palette, StateMutation, eQSL, contest rules
- CQ-3.4.Aâ€“G: Helpery XML/Text/TCP/Rhai/GUI
- CQ-3.5.Aâ€“F: Optymalizacje per-frame, awards_matrix, callbook, pga, fields_to_qso, nazwy metod
- CQ-4.1.Aâ€“E: Komentarze ADIF/core/CAT/GUI/banery
- CQ-4.2.A: Polskie diakrytyki
- CQ-4.2.B: NagĹ‚Ăłwki SPDX
- CQ-4.3.Aâ€“D: Nazewnictwo core/cat/gui/digital
- CQ-4.4: CI/CD fmt+clippy

KaĹĽde zadanie: osobny commit, osobna sesja. SzczegĂłĹ‚y w `CODE_QUALITY_NORMALIZATION_PLAN.md`.

---

# 18. Etap M11: DuĹĽe refaktoryzacje

KaĹĽde wymaga osobnego DESIGN.md. Nie wolno wykonywaÄ‡ podczas napraw krytycznych updatera.

#### ARCH-SPLOGAPP â€” Dekompozycja SpLogApp
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Pliki:** `src/gui/app.rs` (konflikt z FIX-CONTEST-COUNTERS)

#### ARCH-QSO-PIPELINE â€” WspĂłlny potok zapisu QSO
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **ĹąrĂłdĹ‚o:** CQ: PROB-P1-06

#### ARCH-SQL-NORMALIZE â€” Normalizacja 53-kolumnowego SQL
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Pliki:** `src/core/database.rs` (strefa wyĹ‚Ä…cznoĹ›ci)

#### ARCH-CAT â€” Architektura CAT z TCI
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

#### ARCH-SYNC â€” Synchronizacja P2P vs LAN
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

#### ARCH-I18N â€” PeĹ‚na lokalizacja
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

#### ARCH-THEMES â€” Centralizacja motywĂłw
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

---

# 19. Macierz Ĺ›ledzenia problemĂłw

```
Problem: Updater Linux nadpisuje binarkÄ™ archiwum tar.gz
  ĹąrĂłdĹ‚a: AUDIT_REPORT.md, CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P0-03)
  Weryfikacja: VERIFY-ARTIFACTS
  Awaryjna poprawka: UPDATE-LINUX-SAFETY-GATE
  Docelowy DESIGN: DESIGN_3_0_B_LINUX_UPDATE.md, DESIGN_3_0_C_PLATFORM_INSTALLERS.md
  Implementacja: UPDATE-LINUX-INSTALLER
  Test: RELEASE-TRIAL-LINUX
  Wydanie: RELEASE-PUBLIC-V1

Problem: Updater Windows nadpisuje EXE plikiem ZIP
  ĹąrĂłdĹ‚a: MASTER_IMPLEMENTATION_PLAN.md (nowo odkryty), src/cloud/updater.rs
  Weryfikacja: VERIFY-ARTIFACTS
  Awaryjna poprawka: UPDATE-WIN-SAFETY-GATE
  Docelowy DESIGN: DESIGN_3_0_A_WINDOWS_UPDATE.md
  Implementacja: UPDATE-WIN-INSTALLER
  Test: RELEASE-TRIAL-WIN
  Wydanie: RELEASE-PUBLIC-V1

Problem: Brak podpisĂłw Ed25519
  ĹąrĂłdĹ‚a: AUDIT_REPORT.md, DESIGN_3_1_UPDATER_SIGNATURES.md
  Weryfikacja: VERIFY-GITHUB-DIGEST
  DESIGN: DESIGN_3_1_UPDATER_SIGNATURES.md (zatwierdzony)
  Implementacja: CRYPTO-ED25519-VERIFY, CRYPTO-MANIFEST-SCHEMA, CRYPTO-CI-SIGNING
  Test: RELEASE-TRIAL-WIN, RELEASE-TRIAL-LINUX
  Wydanie: RELEASE-PUBLIC-V1

Problem: SOTA Utc::now() fallback
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P0-01)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-SOTA-FALLBACK (CQ-1.1)
  ZamkniÄ™cie: test bĹ‚Ä™dnej daty SOTA

Problem: N1MM band suffix 70cm â†’ 70c
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-N1MM-A)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-N1MM-BAND (CQ-1.2.A)
  ZamkniÄ™cie: test pasma wg specyfikacji N1MM

Problem: N1MM niespĂłjne jednostki czÄ™stotliwoĹ›ci
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-N1MM-B)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-N1MM-FREQ (CQ-1.2.B)
  ZamkniÄ™cie: test jednostek ContactInfo/RadioInfo

Problem: Contest counters bezwarunkowa inkrementacja
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-01)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-CONTEST-COUNTERS (CQ-1.4)
  ZamkniÄ™cie: test statystyk poza trybem zawodĂłw

Problem: Heurystyka ends_with("CW") w klubach
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-02)
  Weryfikacja: brak (potwierdzony)
  Implementacja: FIX-CLUBS-HEURISTIC (CQ-2.1.A), FIX-CALLBOOK-DEMO (CQ-2.1.B)
  ZamkniÄ™cie: test negatywny dla SP9XCW

Problem: Duplikacja WPX prefix
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-03A)
  Implementacja: FIX-WPX-PREFIX (CQ-2.2.A)

Problem: Duplikacja SP district
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-03B)
  Implementacja: FIX-SP-DISTRICT (CQ-2.2.B)

Problem: PodwĂłjna rejestracja dyplomĂłw
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-03C)
  Implementacja: FIX-AWARDS-DOUBLE-REG (CQ-2.2.C)

Problem: Fallback TCP 127.0.0.1:7373
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-04)
  Implementacja: FIX-MULTIOP-FALLBACK (CQ-2.3.A)

Problem: Voice Keyer PTT bypass
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-05)
  Implementacja: FIX-VOICEKEYER-CAT (CQ-2.3.B)

Problem: 5x duplikacja potoku QSO
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-06)
  Implementacja: ARCH-QSO-PIPELINE (wymaga DESIGN)

Problem: Brak Send+Sync w async error
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-07)
  Implementacja: FIX-ASYNC-SEND-SYNC (CQ-2.4.A)

Problem: Martwy kod GUI
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P2-01)
  Weryfikacja: VERIFY-DEAD-CODE
  Implementacja: CQ-3.1.B1â€“B4

Problem: rows.flatten() maskuje bĹ‚Ä™dy
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P2-02)
  Implementacja: CQ-2.4.B â†’ CQ-2.4.C1â€“C4

Problem: Komentarze promptowe
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P3-01)
  Implementacja: CQ-4.1.Aâ€“E

Problem: Skrypty robocze i brak CI lint
  ĹąrĂłdĹ‚a: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P3-04)
  Implementacja: CQ-4.4
```

---

# 20. Migracja ze starych planĂłw

## Z PLAN.md:
| Stary ID | Nowy ID | Status |
|---|---|---|
| PLAN 1.1 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 1.2 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 1.3 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 1.4 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 2.1 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 2.2 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 2.3 | â€” | DONE, nie otwieraÄ‡ |
| PLAN 3.1 | CRYPTO-* (M4â€“M6) | Aktywny, nowy tor |
| PLAN 3.2 | FIX w ramach CQ | Aktywny |
| PLAN 4.1â€“4.3 | ARCH-SQL-NORMALIZE | Aktywny, wymaga DESIGN |
| PLAN 5.1â€“5.4 | CQ-3.5.Aâ€“F | Aktywny |
| PLAN 6.1â€“6.4 | ARCH-SPLOGAPP | Aktywny, wymaga DESIGN |
| PLAN 7.1â€“7.2 | CQ (przyszĹ‚e) | Aktywny |
| PLAN 8.1â€“8.3 | Poza zakresem | Nowe funkcje |

## Z CODE_QUALITY_NORMALIZATION_PLAN.md:
| CQ ID | Nowy ID | Status |
|---|---|---|
| PROB-P0-03 / CQ-1.3 | UPDATE-LINUX-SAFETY-GATE | ROZWIÄ„ZYWANY PRZEZ PLAN NADRZÄDNY |
| Wszystkie inne CQ-* | Zachowane bez zmian | Aktywne w torze CQ/FIX |

## Z DESIGN_3_1_UPDATER_SIGNATURES.md:
| Element | Nowy ID | Status |
|---|---|---|
| Ed25519 weryfikator | CRYPTO-ED25519-VERIFY | Aktywny |
| Manifest schema | CRYPTO-MANIFEST-SCHEMA | Aktywny |
| Key table | CRYPTO-KEY-TABLE | Aktywny |
| Signer CLI | CRYPTO-SIGNER-CLI | Aktywny |
| CI signing | CRYPTO-CI-SIGNING | Aktywny |

---

# 21. Konflikty plikĂłw i zasady rĂłwnolegĹ‚oĹ›ci

Nie zezwalaÄ‡ na rĂłwnolegĹ‚e zmiany w:
- `src/cloud/updater.rs` â€” wyĹ‚Ä…cznoĹ›Ä‡ toru UPDATE/CRYPTO
- `Cargo.toml` â€” wyĹ‚Ä…cznoĹ›Ä‡ per commit
- `Cargo.lock` â€” wyĹ‚Ä…cznoĹ›Ä‡ per commit
- `.github/workflows/build-and-release.yml` â€” wyĹ‚Ä…cznoĹ›Ä‡ toru RELEASE
- `src/gui/app.rs` â€” wyĹ‚Ä…cznoĹ›Ä‡ (FIX-CONTEST-COUNTERS xor ARCH-SPLOGAPP)
- `src/core/database.rs` â€” wyĹ‚Ä…cznoĹ›Ä‡ (CQ-2.4.C1 xor ARCH-SQL-NORMALIZE)

RĂłwnolegĹ‚oĹ›Ä‡ dozwolona: FIX-CLUBS-HEURISTIC (clubs.rs) âĄ FIX-WPX-PREFIX (prefix.rs) âĄ FIX-VOICEKEYER-CAT (voice_keyer.rs) â€” rĂłĹĽne pliki, brak konfliktĂłw.

---

# 22. Kryteria akceptacji

Dla kaĹĽdego zadania implementacyjnego:
- Zachowanie przed zmianÄ… (udokumentowane).
- Zachowanie po zmianie (udokumentowane).
- Test regresyjny (nowy lub zaktualizowany).
- Lista zmodyfikowanych plikĂłw.
- WpĹ‚yw na publiczne API.
- WpĹ‚yw na format danych.
- `cargo fmt --check` âś…
- `cargo check` âś…
- `cargo clippy --all-targets --all-features` âś…
- `cargo test` âś…
- Test Windows (jeĹ›li platformowy).
- Test Linux (jeĹ›li platformowy).
- Test rÄ™czny GUI (jeĹ›li automatyczny nie wystarcza).

Dla zmian updatera dodatkowo:
- Test niepeĹ‚nego pobrania.
- Test zĹ‚ego rozmiaru.
- Test zĹ‚ego SHA-256.
- Test braku praw zapisu.
- Test rollbacku.
- Test zachowania danych uĹĽytkownika.

Dla zmian redakcyjnych: kompilacja i testy wystarczajÄ….

---

# 23. Otwarte decyzje wĹ‚aĹ›ciciela projektu

1. **Czy updater Windows ma obsĹ‚ugiwaÄ‡ rozpakowanie ZIP w ramach aktualizacji?** (Aktualny artefakt to ZIP z wieloma plikami: EXE, DLL, bazy danych). Czy docelowo format Windows zmieni siÄ™ na standalone EXE lub MSI?
2. **Czy plik `serviceLOG.db` powinien byÄ‡ w archiwum wydania?** (Obecnie jest pakowany. Powinien byÄ‡ instalowany tylko przy pierwszym uruchomieniu, nie nadpisywany przy aktualizacji.)
3. **Czy ADIF_VERSION powinien byÄ‡ 3.1.7 czy 3.1.8?** (PLAN.md etap 2.1 ustawiĹ‚ 3.1.8, ale CODE_QUALITY_NORMALIZATION_PLAN.md wymaga 3.1.7. Wymaga decyzji wĹ‚aĹ›ciciela.)
4. **Jakie jest docelowe minimum obsĹ‚ugiwanych formatĂłw Linux?** (Tylko portable tar.gz? AppImage w przyszĹ‚oĹ›ci?)

---

# 24. NajbliĹĽsze nastÄ™pne zadanie

#### VERIFY-ARTIFACTS â€” Inwentaryzacja artefaktĂłw Windows i Linux

- **Identyfikator:** VERIFY-ARTIFACTS
- **Cel:** UstaliÄ‡ dokĹ‚adnÄ… listÄ™ artefaktĂłw budowanych i publikowanych dla Windows i Linux, ich nazwy, formaty, wewnÄ™trznÄ… strukturÄ™ archiwĂłw, lokalizacjÄ™ binarek i danych uĹĽytkownika, uprawnienia zapisu do katalogu instalacji.
- **WejĹ›cia:** `.github/workflows/build-and-release.yml`, `src/cloud/updater.rs`, istniejÄ…ce release'y GitHub (read-only).
- **Wynik:** `UPDATE_ARTIFACT_INVENTORY_REPORT.md`
- **Pliki (read-only):** `.github/workflows/build-and-release.yml`, `src/cloud/updater.rs`, `Cargo.toml`
- **Plik wynikowy:** `UPDATE_ARTIFACT_INVENTORY_REPORT.md`
- **Zakaz:** Nie implementowaÄ‡. Nie zmieniaÄ‡ kodu. Nie zmieniaÄ‡ workflow.
