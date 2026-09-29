# MASTER_IMPLEMENTATION_PLAN.md — SPLogbook

**Utworzono:** 2026-09-29  
**Wersja:** 1.0  
**Rola:** Jedyny nadrzędny plan organizacyjny projektu SPLogbook.

---

# 1. Cel dokumentu

Ten dokument jest jedynym nadrzędnym planem modernizacji SPLogbook. Łączy ustalenia z:
- `AUDIT_REPORT.md` (historyczne źródło ustaleń audytu),
- `PLAN.md` (dotychczasowy plan etapowy z wykonanymi etapami 1.x–2.x),
- `CODE_QUALITY_NORMALIZATION_PLAN.md` (szczegółowy rejestr problemów jakościowych, prefiksy CQ-),
- `DESIGN_2_2_FINAL.md` (zatwierdzona architektura docking GUI — DONE),
- `DESIGN_3_1_UPDATER_SIGNATURES.md` (zatwierdzona architektura Ed25519),
- projekt bezpiecznej aktualizacji Windows i Linux,
- wspólny rdzeń weryfikacji aktualizacji.

Żaden z powyższych dokumentów nie jest usuwany. Ich role:
- `MASTER_IMPLEMENTATION_PLAN.md` → kolejność, zależności, statusy, etapy.
- `AUDIT_REPORT.md` → historyczne obserwacje wymagające ponownej weryfikacji.
- `CODE_QUALITY_NORMALIZATION_PLAN.md` → szczegółowy rejestr problemów CQ.
- `DESIGN_*.md` → zatwierdzone decyzje architektoniczne.
- `REPORT_*.md` → wyniki etapów analitycznych.

---

# 2. Hierarchia źródeł decyzji

1. **Aktualny kod repozytorium** — źródło informacji o obecnym stanie implementacji.
2. **Zatwierdzone dokumenty DESIGN** — źródło obowiązujących decyzji architektonicznych.
3. **MASTER_IMPLEMENTATION_PLAN.md** — kolejność, zależności, statusy.
4. **Raporty weryfikacyjne REPORT_*.md** — wyniki konkretnych analiz.
5. **CODE_QUALITY_NORMALIZATION_PLAN.md** — rejestr problemów jakościowych.
6. **AUDIT_REPORT.md** — obserwacje historyczne, wymagające ponownej weryfikacji.

Reguły konfliktów:
- DESIGN ma pierwszeństwo przed rekomendacją audytu.
- Aktualny kod ma pierwszeństwo przy ustalaniu stanu faktycznego.
- Niejasność wymaga raportu weryfikacyjnego.
- Nie wolno podejmować arbitralnej decyzji podczas implementacji.

---

# 3. Stan projektu i ukończone etapy

| Etap | Opis | Status | Źródło |
|---|---|---|---|
| PLAN 1.1 | `row_to_qso` `.ok()` → `?` | **DONE** | PLAN.md |
| PLAN 1.2 | Walidacja zakresu czasu HH:MM | **DONE** | PLAN.md |
| PLAN 1.3 | Indeksy `country` i `freq` | **DONE** | PLAN.md |
| PLAN 1.4 | Walidacja formatów IOTA/SOTA/POTA | **DONE** | PLAN.md |
| PLAN 2.1 | Aktualizacja ADIF_VERSION | **DONE** | PLAN.md |
| PLAN 2.2 | Naprawa MODE/SUBMODE (DESIGN_2_2_FINAL) | **DONE** | PLAN.md, DESIGN_2_2_FINAL.md |
| PLAN 2.3 | Fallback pasma "20m" → błąd | **DONE** | PLAN.md |

Etapy DONE nie podlegają ponownemu otwieraniu bez potwierdzonej regresji.

**Informacje o środowisku:**
- Edition: 2024, MSRV: 1.85, Wersja: 1.1.0
- Artefakty CI: Windows = `SPLogbook-Windows-x64.zip`, Linux = `SPLogbook-Linux-x86_64.tar.gz`
- Brak: AppImage, deb, rpm, Flatpak, MSI, standalone .exe release
- Brak: `ed25519-dalek` w `Cargo.toml`
- CI nie uruchamia `cargo fmt` ani `cargo clippy`

---

# 4. Statusy, priorytety i prefiksy

## Prefiksy zadań

| Prefiks | Tor | Opis |
|---|---|---|
| `VERIFY-*` | Weryfikacja | Zadania analityczne i raporty |
| `FIX-*` | Poprawki | Małe potwierdzone poprawki funkcjonalne |
| `UPDATE-*` | Aktualizator | Windows, Linux, wspólny instalator |
| `CRYPTO-*` | Kryptografia | Manifest, Ed25519, klucze, podpisywanie |
| `CQ-*` | Jakość kodu | Komentarze, nazewnictwo, duplikacje |
| `ARCH-*` | Architektura | Duże refaktoryzacje wymagające DESIGN.md |
| `RELEASE-*` | Wydania | Próbne i publiczne wydania |

## Statusy techniczne

- POTWIERDZONY W KODZIE
- CZĘŚCIOWO POTWIERDZONY
- WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ
- WYMAGA TESTU URUCHOMIENIOWEGO
- NIEPOTWIERDZONY
- FAŁSZYWY ALARM
- ROZWIĄZANY

## Statusy wykonawcze

- GOTOWY DO IMPLEMENTACJI
- ZABLOKOWANY DO CZASU RAPORTU
- ZABLOKOWANY DO CZASU DESIGN.md
- ZABLOKOWANY PRZEZ INNE ZADANIE
- ROZWIĄZYWANY PRZEZ PLAN NADRZĘDNY
- TYLKO ZADANIE ANALITYCZNE
- NIE IMPLEMENTOWAĆ
- DONE

Zadanie implementacyjne wymaga statusu wykonawczego = `GOTOWY DO IMPLEMENTACJI`.

---

# 5. Zasady własności problemów

Każdy problem ma dokładnie jeden tor nadrzędny. Nie może istnieć drugi konkurencyjny task naprawiający ten sam problem.

**Kluczowa reguła koordynacji updatera:** Wszystkie zmiany dotyczące `src/cloud/updater.rs`, wyboru artefaktów, pobierania, instalacji Windows/Linux, SHA-256, manifestu, Ed25519, SemVer, GitHub Actions i procesu wydawniczego należą do toru `UPDATE + CRYPTO + RELEASE`. Zadania `CQ` nie mogą równolegle modyfikować tych elementów.

Problem `PROB-P0-03` (updater Linux) z audytu jakości → **ROZWIĄZYWANY PRZEZ PLAN NADRZĘDNY** (tor UPDATE).

---

# 6. Mapa zależności

```
VERIFY-ARTIFACTS ─────────────────────────────────────┐
VERIFY-P0 ────────────────────────────────────────────┤
VERIFY-GITHUB-DIGEST ─────────────────────────────────┤
                                                      ▼
                                              ETAP M1: Safety Gates
                                              UPDATE-WIN-SAFETY-GATE
                                              UPDATE-LINUX-SAFETY-GATE
                                                      │
                                                      ▼
                                              ETAP M2: DESIGN docs
                                              DESIGN_3_0_A/B/C
                                                      │
                                                      ▼
                                              ETAP M3: Instalatory
                                              UPDATE-WIN-INSTALLER
                                              UPDATE-LINUX-INSTALLER
                                                      │
                                                      ▼
                                              ETAP M4: Schemat manifestu
                                              CRYPTO-MANIFEST-SCHEMA
                                                      │
                                                      ▼
                                              ETAP M5: Kryptografia
                                              CRYPTO-ED25519-VERIFY
                                                      │
                                                      ▼
                                              ETAP M6: CI signing
                                              CRYPTO-CI-SIGNING
                                                      │
                                                      ▼
                                              ETAP M7: Wydania próbne
                                                      │
                                                      ▼
                                              ETAP M8: Wydanie publiczne

    (Równolegle z M1-M8, jeśli brak konfliktu plików:)
    CQ-0 → CQ-1.x → CQ-2.x → CQ-3.x → CQ-4.x
    ARCH-* (po M8 i stabilizacji)
```

---

# 7. Etap M0: Raporty i weryfikacja

Brak modyfikacji kodu. Wyłącznie analiza.

#### VERIFY-ARTIFACTS — Inwentaryzacja artefaktów Windows i Linux
- **Tor:** VERIFY
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P0 Krytyczny
- **Źródło:** MASTER_IMPLEMENTATION_PLAN.md
- **Zakres:** Ustalić faktyczne formaty, nazwy, struktury archiwów, lokalizacje binarek, dane użytkownika, uprawnienia zapisu. Zbadać `build-and-release.yml` i istniejące release'y GitHub.
- **Poza zakresem:** Zmiana kodu, zmiana workflow.
- **Pliki (read-only):** `.github/workflows/build-and-release.yml`, `src/cloud/updater.rs`
- **Plik wynikowy:** `UPDATE_ARTIFACT_INVENTORY_REPORT.md`
- **Zależności:** brak
- **Konflikty:** brak
- **Kryterium ukończenia:** Raport zawiera pełną listę artefaktów obu platform z dokładnymi nazwami i strukturą.

#### VERIFY-P0 — Weryfikacja problemów krytycznych P0
- **Tor:** VERIFY
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P0 Krytyczny
- **Źródło:** CODE_QUALITY_NORMALIZATION_PLAN.md (CQ-0)
- **Zakres:** Ponowna weryfikacja: SOTA Utc::now(), N1MM jednostki/pasma, updater Linux, updater Windows (nowo odkryty!), liczniki zawodów. Sprawdzenie osiągalności, przepływu wykonania, wpływu na dane.
- **Poza zakresem:** Zmiana kodu.
- **Pliki (read-only):** `src/core/sota_export.rs`, `src/digital/n1mm.rs`, `src/cloud/updater.rs`, `src/gui/app.rs`
- **Plik wynikowy:** `REPORT_CODE_QUALITY_P0_VERIFICATION.md`
- **Zależności:** brak
- **Kryterium ukończenia:** Raport z ostatecznym priorytetem każdego P0.

#### VERIFY-GITHUB-DIGEST — Analiza GitHub Digest API
- **Tor:** VERIFY
- **Status techniczny:** WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** DESIGN_3_1_UPDATER_SIGNATURES.md
- **Zakres:** Ustalenie, czy GitHub API dostarcza digest dla własnych release assets i w jakim formacie.
- **Poza zakresem:** Implementacja.
- **Plik wynikowy:** `REPORT_3_1_A_GITHUB_DIGEST.md`
- **Zależności:** brak

#### VERIFY-DEAD-CODE — Raport użycia symboli
- **Tor:** VERIFY
- **Status techniczny:** WYMAGA TESTU URUCHOMIENIOWEGO
- **Status wykonawczy:** TYLKO ZADANIE ANALITYCZNE
- **Priorytet:** P2 Średni
- **Źródło:** CODE_QUALITY_NORMALIZATION_PLAN.md (CQ-3.1.A)
- **Plik wynikowy:** `DEAD_CODE_VERIFICATION_REPORT.md`
- **Zależności:** brak

---

# 8. Etap M1: Awaryjne zabezpieczenia

Minimalne poprawki zapobiegające niszczącej aktualizacji.

#### UPDATE-LINUX-SAFETY-GATE — Blokada niszczącej aktualizacji Linux
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU (VERIFY-ARTIFACTS)
- **Priorytet:** P0 Krytyczny
- **Źródło:** CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P0-03), AUDIT_REPORT.md
- **Zakres:** Zablokować nadpisanie binarki plikiem .tar.gz/.deb/.rpm. Komunikat o ręcznej aktualizacji. Jawny błąd dla brakującego tag_name. Usunięcie pliku tymczasowego. Żadnego sudo, rozpakowywania, instalowania deb.
- **Poza zakresem:** Pełny instalator Linux. Rozpakowywanie tar.gz. Ed25519.
- **Pliki:** `src/cloud/updater.rs`
- **Zależności:** VERIFY-ARTIFACTS
- **Konflikty:** Nie może być równoległy z żadnym innym zadaniem UPDATE-*.
- **Zachowanie przed:** `rename(tar.gz, splogbook)` → uszkodzenie instalacji.
- **Zachowanie po:** Przerwanie aktualizacji z komunikatem; binarka nienaruszona.
- **Testy:** Test jednostkowy: artefakt .tar.gz → Err. Test: brak tag_name → Err.
- **Test Linux:** TAK (wymagany)
- **Wpływ na API:** brak
- **Wpływ na format danych:** brak
- **Sugerowany tytuł commita:** `fix(updater): block destructive Linux self-replace with archive files`

#### UPDATE-WIN-SAFETY-GATE — Blokada niszczącej aktualizacji Windows
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU (VERIFY-ARTIFACTS)
- **Priorytet:** P0 Krytyczny
- **Źródło:** MASTER_IMPLEMENTATION_PLAN.md (nowo odkryty problem — artefakt Windows to ZIP!)
- **Zakres:** Zablokować nadpisanie SPLogbook.exe plikiem .zip. Komunikat błędu. Obecny skrypt PowerShell przenosi surowy ZIP w miejsce EXE — to ten sam błąd co na Linux.
- **Poza zakresem:** Pełny instalator Windows. Rozpakowanie ZIP.
- **Pliki:** `src/cloud/updater.rs`
- **Zależności:** VERIFY-ARTIFACTS
- **Konflikty:** Nie może być równoległy z UPDATE-LINUX-SAFETY-GATE (ten sam plik).
- **Zachowanie przed:** `Move-Item zip → exe` → uszkodzenie.
- **Zachowanie po:** Przerwanie z komunikatem; EXE nienaruszone.
- **Testy:** Test: artefakt .zip → Err.
- **Test Windows:** TAK (wymagany)
- **Sugerowany tytuł commita:** `fix(updater): block destructive Windows self-replace with ZIP archives`

---

# 9. Etap M2: Projekty instalatorów

Utworzenie i zatwierdzenie dokumentów projektowych. Brak implementacji.

#### UPDATE-DESIGN-WIN — Projekt instalatora Windows
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M1)
- **Priorytet:** P0 Krytyczny
- **Plik wynikowy:** `DESIGN_3_0_A_WINDOWS_UPDATE.md`
- **Zakres:** Format wydania (ZIP), ochrona przed Zip Slip, staging, zamknięcie starego procesu, rollback, health confirmation, zachowanie danych.

#### UPDATE-DESIGN-LINUX — Projekt instalatora Linux
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M1)
- **Plik wynikowy:** `DESIGN_3_0_B_LINUX_UPDATE.md`
- **Zakres:** Format tar.gz (jedyny faktycznie budowany), ekstrakcja do staging, ochrona przed ścieżkami absolutnymi i symlinkami, rollback, health confirmation.

#### UPDATE-DESIGN-PLATFORM — Wspólny interfejs instalatorów
- **Tor:** UPDATE
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M2-WIN, M2-LINUX)
- **Plik wynikowy:** `DESIGN_3_0_C_PLATFORM_INSTALLERS.md`
- **Zakres:** Operacje logiczne: detect_installation_type, validate, stage, rollback, apply, launch, health, cleanup.

---

# 10. Etap M3: Implementacja Windows i Linux

#### UPDATE-WIN-INSTALLER — Instalator Windows
- **Tor:** UPDATE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Zależności:** UPDATE-DESIGN-WIN, UPDATE-DESIGN-PLATFORM
- **Pliki:** `src/cloud/updater.rs`

#### UPDATE-LINUX-INSTALLER — Instalator Linux (tar.gz)
- **Tor:** UPDATE
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Zależności:** UPDATE-DESIGN-LINUX, UPDATE-DESIGN-PLATFORM
- **Pliki:** `src/cloud/updater.rs`

---

# 11. Etap M4: Schemat manifestu

#### CRYPTO-MANIFEST-SCHEMA — Schemat release-manifest.json
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M3 — package_type zależy od faktycznie wspieranych formatów)
- **Źródło:** DESIGN_3_1_UPDATER_SIGNATURES.md
- **Zakres:** manifest_version, product, channel, version (SemVer), minimum_updater_version, commit (40 hex), assets z platform+arch+package_type+filename+size+sha256.

---

# 12. Etap M5: Ed25519 i polityka wersji

#### CRYPTO-KEY-TABLE — Tabela zaufanych kluczy publicznych
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M4)
- **Źródło:** DESIGN_3_1_UPDATER_SIGNATURES.md

#### CRYPTO-ED25519-VERIFY — Weryfikator podpisów Ed25519
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M4)
- **Zakres:** Pobranie manifestu i .sig, dekodowanie Base64, weryfikacja Ed25519, sprawdzenie key_id, odrzucenie nieznanych manifest_version.
- **Pliki:** `src/cloud/updater.rs`, `Cargo.toml` (dodanie ed25519-dalek)

#### CRYPTO-MANIFEST-VALIDATE — Walidacja zawartości manifestu
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (CRYPTO-ED25519-VERIFY)

#### CRYPTO-SEMVER-POLICY — Polityka wersji i ochrona przed downgrade
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (CRYPTO-MANIFEST-VALIDATE)

#### CRYPTO-DOWNGRADE-PROTECTION — Odrzucanie starszych wersji
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (CRYPTO-SEMVER-POLICY)

---

# 13. Etap M6: CI i podpisywanie

#### CRYPTO-SIGNER-CLI — Narzędzie podpisujące offline
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M5)

#### CRYPTO-KEY-MANAGEMENT — Zarządzanie kluczami
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (M5)

#### CRYPTO-KEY-ROTATION — Rotacja kluczy
- **Tor:** CRYPTO
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (CRYPTO-KEY-MANAGEMENT)

#### CRYPTO-CI-SIGNING — Integracja z GitHub Actions
- **Tor:** CRYPTO/RELEASE
- **Status wykonawczy:** ZABLOKOWANY PRZEZ INNE ZADANIE (CRYPTO-SIGNER-CLI)
- **Pliki:** `.github/workflows/build-and-release.yml`

---

# 14. Etap M7: Wydania próbne

#### RELEASE-TRIAL-WIN — Wydanie próbne Windows
- **Tor:** RELEASE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Zakres:** Test poprawnego wydania, podmieniony manifest/podpis/artefakt, zły rozmiar, przerwany transfer, rollback, health confirmation.
- **Test Windows:** TAK

#### RELEASE-TRIAL-LINUX — Wydanie próbne Linux
- **Tor:** RELEASE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Test Linux:** TAK

---

# 15. Etap M8: Pierwsze wydanie publiczne

#### RELEASE-PUBLIC-V1 — Pierwsze wydanie z podpisami
- **Tor:** RELEASE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Zależności:** RELEASE-TRIAL-WIN OK, RELEASE-TRIAL-LINUX OK

---

# 16. Etap M9: Poprawki funkcjonalne z audytu

Zadania P0/P1 niezwiązane z updaterem mogą być wykonywane równolegle z torem UPDATE, pod warunkiem braku konfliktu plików.

#### FIX-SOTA-FALLBACK (CQ-1.1)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Priorytet:** P0 Krytyczny
- **Źródło:** CQ: PROB-P0-01
- **Pliki:** `src/core/sota_export.rs`
- **Konflikty:** brak z UPDATE
- **Sugerowany tytuł commita:** `fix(sota): return error instead of Utc::now() fallback on bad QSO date`

#### FIX-N1MM-BAND (CQ-1.2.A)
- **Tor:** FIX
- **Status techniczny:** WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU (VERIFY-P0)
- **Priorytet tymczasowy:** P1 Wysoki (potencjalnie P0)
- **Źródło:** CQ: PROB-N1MM-A
- **Pliki:** `src/digital/n1mm.rs`

#### FIX-N1MM-FREQ (CQ-1.2.B)
- **Tor:** FIX
- **Status techniczny:** WYMAGA SPECYFIKACJI ZEWNĘTRZNEJ
- **Status wykonawczy:** ZABLOKOWANY DO CZASU RAPORTU (VERIFY-P0)
- **Priorytet tymczasowy:** P1 Wysoki (potencjalnie P0)
- **Źródło:** CQ: PROB-N1MM-B
- **Pliki:** `src/digital/n1mm.rs`

#### FIX-CONTEST-COUNTERS (CQ-1.4)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-01
- **Pliki:** `src/gui/app.rs`
- **Konflikty:** Plik `app.rs` — nie może być równoległy z ARCH-SPLOGAPP

#### FIX-CLUBS-HEURISTIC (CQ-2.1.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-02
- **Pliki:** `src/core/clubs.rs`
- **Konflikty:** brak
- **Sugerowany tytuł commita:** `fix(clubs): remove incorrect ends_with("CW") heuristic`

#### FIX-CALLBOOK-DEMO (CQ-2.1.B)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** ZAKOŃCZONY (DONE)
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-02
- **Pliki:** `src/core/callbook.rs`
- **Sugerowany tytuł commita:** `fix(callbook): remove hardcoded demo data`

#### FIX-WPX-PREFIX (CQ-2.2.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-03A
- **Pliki:** `src/core/prefix.rs`, `src/core/awards.rs`

#### FIX-SP-DISTRICT (CQ-2.2.B)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-03B
- **Pliki:** `src/core/awards.rs`

#### FIX-AWARDS-DOUBLE-REG (CQ-2.2.C)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-03C
- **Pliki:** `src/core/awards.rs`

#### FIX-MULTIOP-FALLBACK (CQ-2.3.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-04
- **Pliki:** `src/gui/contest.rs`

#### FIX-VOICEKEYER-CAT (CQ-2.3.B)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-05
- **Pliki:** `src/gui/voice_keyer.rs`

#### FIX-ASYNC-SEND-SYNC (CQ-2.4.A)
- **Tor:** FIX
- **Status techniczny:** POTWIERDZONY W KODZIE
- **Status wykonawczy:** GOTOWY DO IMPLEMENTACJI
- **Priorytet:** P1 Wysoki
- **Źródło:** CQ: PROB-P1-07
- **Pliki:** `src/cloud/lotw.rs`, `src/cloud/qrz.rs`, `src/cloud/solar.rs`

---

# 17. Etap M10: Normalizacja jakości

Wszystkie zadania CQ-2.4.B do CQ-4.4 z `CODE_QUALITY_NORMALIZATION_PLAN.md`.

Pełna lista zadań (z prefiksem CQ-):
- CQ-2.4.B: Audyt rows.flatten() (polityki)
- CQ-2.4.C1–C4: Obsługa błędów SQLite per moduł
- CQ-3.1.B1–B4: Usuwanie martwego kodu (po raporcie CQ-3.1.A / VERIFY-DEAD-CODE)
- CQ-3.3.A–E: Wrappery, command_palette, StateMutation, eQSL, contest rules
- CQ-3.4.A–G: Helpery XML/Text/TCP/Rhai/GUI
- CQ-3.5.A–F: Optymalizacje per-frame, awards_matrix, callbook, pga, fields_to_qso, nazwy metod
- CQ-4.1.A–E: Komentarze ADIF/core/CAT/GUI/banery
- CQ-4.2.A: Polskie diakrytyki
- CQ-4.2.B: Nagłówki SPDX
- CQ-4.3.A–D: Nazewnictwo core/cat/gui/digital
- CQ-4.4: CI/CD fmt+clippy

Każde zadanie: osobny commit, osobna sesja. Szczegóły w `CODE_QUALITY_NORMALIZATION_PLAN.md`.

---

# 18. Etap M11: Duże refaktoryzacje

Każde wymaga osobnego DESIGN.md. Nie wolno wykonywać podczas napraw krytycznych updatera.

#### ARCH-SPLOGAPP — Dekompozycja SpLogApp
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Pliki:** `src/gui/app.rs` (konflikt z FIX-CONTEST-COUNTERS)

#### ARCH-QSO-PIPELINE — Wspólny potok zapisu QSO
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Źródło:** CQ: PROB-P1-06

#### ARCH-SQL-NORMALIZE — Normalizacja 53-kolumnowego SQL
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md
- **Pliki:** `src/core/database.rs` (strefa wyłączności)

#### ARCH-CAT — Architektura CAT z TCI
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

#### ARCH-SYNC — Synchronizacja P2P vs LAN
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

#### ARCH-I18N — Pełna lokalizacja
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

#### ARCH-THEMES — Centralizacja motywów
- **Status wykonawczy:** ZABLOKOWANY DO CZASU DESIGN.md

---

# 19. Macierz śledzenia problemów

```
Problem: Updater Linux nadpisuje binarkę archiwum tar.gz
  Źródła: AUDIT_REPORT.md, CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P0-03)
  Weryfikacja: VERIFY-ARTIFACTS
  Awaryjna poprawka: UPDATE-LINUX-SAFETY-GATE
  Docelowy DESIGN: DESIGN_3_0_B_LINUX_UPDATE.md, DESIGN_3_0_C_PLATFORM_INSTALLERS.md
  Implementacja: UPDATE-LINUX-INSTALLER
  Test: RELEASE-TRIAL-LINUX
  Wydanie: RELEASE-PUBLIC-V1

Problem: Updater Windows nadpisuje EXE plikiem ZIP
  Źródła: MASTER_IMPLEMENTATION_PLAN.md (nowo odkryty), src/cloud/updater.rs
  Weryfikacja: VERIFY-ARTIFACTS
  Awaryjna poprawka: UPDATE-WIN-SAFETY-GATE
  Docelowy DESIGN: DESIGN_3_0_A_WINDOWS_UPDATE.md
  Implementacja: UPDATE-WIN-INSTALLER
  Test: RELEASE-TRIAL-WIN
  Wydanie: RELEASE-PUBLIC-V1

Problem: Brak podpisów Ed25519
  Źródła: AUDIT_REPORT.md, DESIGN_3_1_UPDATER_SIGNATURES.md
  Weryfikacja: VERIFY-GITHUB-DIGEST
  DESIGN: DESIGN_3_1_UPDATER_SIGNATURES.md (zatwierdzony)
  Implementacja: CRYPTO-ED25519-VERIFY, CRYPTO-MANIFEST-SCHEMA, CRYPTO-CI-SIGNING
  Test: RELEASE-TRIAL-WIN, RELEASE-TRIAL-LINUX
  Wydanie: RELEASE-PUBLIC-V1

Problem: SOTA Utc::now() fallback
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P0-01)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-SOTA-FALLBACK (CQ-1.1)
  Zamknięcie: test błędnej daty SOTA

Problem: N1MM band suffix 70cm → 70c
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-N1MM-A)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-N1MM-BAND (CQ-1.2.A)
  Zamknięcie: test pasma wg specyfikacji N1MM

Problem: N1MM niespójne jednostki częstotliwości
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-N1MM-B)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-N1MM-FREQ (CQ-1.2.B)
  Zamknięcie: test jednostek ContactInfo/RadioInfo

Problem: Contest counters bezwarunkowa inkrementacja
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-01)
  Weryfikacja: VERIFY-P0
  Implementacja: FIX-CONTEST-COUNTERS (CQ-1.4)
  Zamknięcie: test statystyk poza trybem zawodów

Problem: Heurystyka ends_with("CW") w klubach
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-02)
  Weryfikacja: brak (potwierdzony)
  Implementacja: FIX-CLUBS-HEURISTIC (CQ-2.1.A), FIX-CALLBOOK-DEMO (CQ-2.1.B)
  Zamknięcie: test negatywny dla SP9XCW

Problem: Duplikacja WPX prefix
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-03A)
  Implementacja: FIX-WPX-PREFIX (CQ-2.2.A)

Problem: Duplikacja SP district
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-03B)
  Implementacja: FIX-SP-DISTRICT (CQ-2.2.B)

Problem: Podwójna rejestracja dyplomów
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-03C)
  Implementacja: FIX-AWARDS-DOUBLE-REG (CQ-2.2.C)

Problem: Fallback TCP 127.0.0.1:7373
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-04)
  Implementacja: FIX-MULTIOP-FALLBACK (CQ-2.3.A)

Problem: Voice Keyer PTT bypass
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-05)
  Implementacja: FIX-VOICEKEYER-CAT (CQ-2.3.B)

Problem: 5x duplikacja potoku QSO
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-06)
  Implementacja: ARCH-QSO-PIPELINE (wymaga DESIGN)

Problem: Brak Send+Sync w async error
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P1-07)
  Implementacja: FIX-ASYNC-SEND-SYNC (CQ-2.4.A)

Problem: Martwy kod GUI
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P2-01)
  Weryfikacja: VERIFY-DEAD-CODE
  Implementacja: CQ-3.1.B1–B4

Problem: rows.flatten() maskuje błędy
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P2-02)
  Implementacja: CQ-2.4.B → CQ-2.4.C1–C4

Problem: Komentarze promptowe
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P3-01)
  Implementacja: CQ-4.1.A–E

Problem: Skrypty robocze i brak CI lint
  Źródła: CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P3-04)
  Implementacja: CQ-4.4
```

---

# 20. Migracja ze starych planów

## Z PLAN.md:
| Stary ID | Nowy ID | Status |
|---|---|---|
| PLAN 1.1 | — | DONE, nie otwierać |
| PLAN 1.2 | — | DONE, nie otwierać |
| PLAN 1.3 | — | DONE, nie otwierać |
| PLAN 1.4 | — | DONE, nie otwierać |
| PLAN 2.1 | — | DONE, nie otwierać |
| PLAN 2.2 | — | DONE, nie otwierać |
| PLAN 2.3 | — | DONE, nie otwierać |
| PLAN 3.1 | CRYPTO-* (M4–M6) | Aktywny, nowy tor |
| PLAN 3.2 | FIX w ramach CQ | Aktywny |
| PLAN 4.1–4.3 | ARCH-SQL-NORMALIZE | Aktywny, wymaga DESIGN |
| PLAN 5.1–5.4 | CQ-3.5.A–F | Aktywny |
| PLAN 6.1–6.4 | ARCH-SPLOGAPP | Aktywny, wymaga DESIGN |
| PLAN 7.1–7.2 | CQ (przyszłe) | Aktywny |
| PLAN 8.1–8.3 | Poza zakresem | Nowe funkcje |

## Z CODE_QUALITY_NORMALIZATION_PLAN.md:
| CQ ID | Nowy ID | Status |
|---|---|---|
| PROB-P0-03 / CQ-1.3 | UPDATE-LINUX-SAFETY-GATE | ROZWIĄZYWANY PRZEZ PLAN NADRZĘDNY |
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

# 21. Konflikty plików i zasady równoległości

Nie zezwalać na równoległe zmiany w:
- `src/cloud/updater.rs` — wyłączność toru UPDATE/CRYPTO
- `Cargo.toml` — wyłączność per commit
- `Cargo.lock` — wyłączność per commit
- `.github/workflows/build-and-release.yml` — wyłączność toru RELEASE
- `src/gui/app.rs` — wyłączność (FIX-CONTEST-COUNTERS xor ARCH-SPLOGAPP)
- `src/core/database.rs` — wyłączność (CQ-2.4.C1 xor ARCH-SQL-NORMALIZE)

Równoległość dozwolona: FIX-CLUBS-HEURISTIC (clubs.rs) ∥ FIX-WPX-PREFIX (prefix.rs) ∥ FIX-VOICEKEYER-CAT (voice_keyer.rs) — różne pliki, brak konfliktów.

---

# 22. Kryteria akceptacji

Dla każdego zadania implementacyjnego:
- Zachowanie przed zmianą (udokumentowane).
- Zachowanie po zmianie (udokumentowane).
- Test regresyjny (nowy lub zaktualizowany).
- Lista zmodyfikowanych plików.
- Wpływ na publiczne API.
- Wpływ na format danych.
- `cargo fmt --check` ✅
- `cargo check` ✅
- `cargo clippy --all-targets --all-features` ✅
- `cargo test` ✅
- Test Windows (jeśli platformowy).
- Test Linux (jeśli platformowy).
- Test ręczny GUI (jeśli automatyczny nie wystarcza).

Dla zmian updatera dodatkowo:
- Test niepełnego pobrania.
- Test złego rozmiaru.
- Test złego SHA-256.
- Test braku praw zapisu.
- Test rollbacku.
- Test zachowania danych użytkownika.

Dla zmian redakcyjnych: kompilacja i testy wystarczają.

---

# 23. Otwarte decyzje właściciela projektu

1. **Czy updater Windows ma obsługiwać rozpakowanie ZIP w ramach aktualizacji?** (Aktualny artefakt to ZIP z wieloma plikami: EXE, DLL, bazy danych). Czy docelowo format Windows zmieni się na standalone EXE lub MSI?
2. **Czy plik `serviceLOG.db` powinien być w archiwum wydania?** (Obecnie jest pakowany. Powinien być instalowany tylko przy pierwszym uruchomieniu, nie nadpisywany przy aktualizacji.)
3. **Czy ADIF_VERSION powinien być 3.1.7 czy 3.1.8?** (PLAN.md etap 2.1 ustawił 3.1.8, ale CODE_QUALITY_NORMALIZATION_PLAN.md wymaga 3.1.7. Wymaga decyzji właściciela.)
4. **Jakie jest docelowe minimum obsługiwanych formatów Linux?** (Tylko portable tar.gz? AppImage w przyszłości?)

---

# 24. Najbliższe następne zadanie

#### VERIFY-ARTIFACTS — Inwentaryzacja artefaktów Windows i Linux

- **Identyfikator:** VERIFY-ARTIFACTS
- **Cel:** Ustalić dokładną listę artefaktów budowanych i publikowanych dla Windows i Linux, ich nazwy, formaty, wewnętrzną strukturę archiwów, lokalizację binarek i danych użytkownika, uprawnienia zapisu do katalogu instalacji.
- **Wejścia:** `.github/workflows/build-and-release.yml`, `src/cloud/updater.rs`, istniejące release'y GitHub (read-only).
- **Wynik:** `UPDATE_ARTIFACT_INVENTORY_REPORT.md`
- **Pliki (read-only):** `.github/workflows/build-and-release.yml`, `src/cloud/updater.rs`, `Cargo.toml`
- **Plik wynikowy:** `UPDATE_ARTIFACT_INVENTORY_REPORT.md`
- **Zakaz:** Nie implementować. Nie zmieniać kodu. Nie zmieniać workflow.
