# Audyt Historycznych Wydań SPLogbook (SEC-001)

## 1. Cel audytu i metodologia

W ramach procedury remediation incydentu `SEC-001` przeprowadzono audyt kryptograficzny wszystkich publicznych wydań programu SPLogbook opublikowanych w repozytorium GitHub (`https://github.com/sp6ina/SPLogbook/releases`).

Audyt został przeprowadzony w oparciu o metadane pobrane bezpośrednio z GitHub Releases API bez pobierania i bez uruchamiania nieznanych binariów wykonywalnych. Weryfikowano obecność manifestów wydań (`release-manifest.json`), plików podpisów asymetrycznych Ed25519 (`release-manifest.json.sig`), zgodność deklarowanych skrótów SHA-256 z rzeczywistymi digestami artefaktów oraz status kryptograficzny klucza, którym podpisano manifest.

### Dozwolone klasyfikacje wyniku końcowego:
- `VERIFIED_CURRENT_KEY` — manifest zweryfikowany aktywnym, aktualnym kluczem produkcyjnym (`splogbook-release-2026-02`).
- `VERIFIED_REVOKED_KEY` — manifest poprawny kryptograficznie, lecz podpisany kluczem historycznym, który został odwołany z powodu kompromitacji materiału klucza. Wydanie nie jest zaufane dla auto-updatera.
- `UNSIGNED` / `MISSING_MANIFEST` — wydanie opublikowane przed wdrożeniem infrastruktury kryptograficznej podpisywania wydań lub z pominięciem generowania manifestu w potoku CI.
- `INVALID_SIGNATURE` — podpis manifestu jest uszkodzony lub nie pasuje do żadnego znanego klucza.
- `HASH_MISMATCH` — skrót SHA-256 w manifeście różni się od rzeczywistego skrótu pliku artefaktu.
- `NOT_AVAILABLE` / `NOT_VERIFIABLE` — brak możliwości pobrania metadanych.

---

## 2. Zestawienie audytowanych wydań

| Tag | Data wydania (GitHub) | Artefakty binarne | Manifest i podpis | Zadeklarowany `key_id` | Status hashy SHA-256 | Wynik końcowy | Dalsza czynność Właściciela |
|---|---|---|---|---|---|---|---|
| **v1.2.0** | 2026-10-26 11:33 UTC | `SPLogbook-Linux-x86_64.tar.gz`<br>`SPLogbook-Windows-x64.zip` | `release-manifest.json`<br>`release-manifest.json.sig` | `splogbook-release-2026-01` | **MATCH** (100% zgodne z digest API GitHub) | **VERIFIED_REVOKED_KEY** | Wycofać lub zastąpić nowym manifestem podpisanym kluczem `splogbook-release-2026-02` w Etapie 2. |
| **v1.1.0** | 2026-09-29 06:42 UTC | `SPLogbook-Linux-x86_64.tar.gz`<br>`SPLogbook-Windows-x64.zip` | Brak | Brak | Brak manifestu | **MISSING_MANIFEST** / **UNSIGNED** | Pozostawić jako wydanie archiwalne legacy; auto-updater odrzuci instalację (fail-closed). |
| **v1.0.3** | 2026-09-22 19:47 UTC | `SPLogbook-Linux-x86_64.tar.gz`<br>`SPLogbook-Windows-x64.zip` | Brak | Brak | Brak manifestu | **MISSING_MANIFEST** / **UNSIGNED** | Pozostawić jako wydanie archiwalne legacy. |
| **v1.0.2** | 2026-09-22 19:20 UTC | `SPLogbook-Linux-x86_64.tar.gz`<br>`SPLogbook-Windows-x64.zip` | Brak | Brak | Brak manifestu | **MISSING_MANIFEST** / **UNSIGNED** | Pozostawić jako wydanie archiwalne legacy. |
| **v1.0.1** | 2026-09-21 19:11 UTC | `SPLogbook-Linux-x86_64.tar.gz`<br>`SPLogbook-Windows-x64.zip` | Brak | Brak | Brak manifestu | **MISSING_MANIFEST** / **UNSIGNED** | Pozostawić jako wydanie archiwalne legacy. |
| **v1.0.0** | 2026-09-21 17:26 UTC | `SPLogbook-v1.0.0-Windows-x64.zip` | Brak | Brak | Brak manifestu | **MISSING_MANIFEST** / **UNSIGNED** | Pozostawić jako wydanie archiwalne legacy. |

---

## 3. Szczegółowa analiza wydania v1.2.0

Wydanie `v1.2.0` jest jedynym historycznym wydaniem zawierającym manifest kryptograficzny.
- **Commit Git:** `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`
- **Podpis manifestu:** `2x0V5Uq6eMKatp09fJVUQI3OPTxi7+A47hUnA/WKumENoVrDt6UMQaBdsQy2qFikT6RwJ5xzAlVeUILH/1IeBQ==`
- **Weryfikacja kryptograficzna podpisu:** Podpis jest w 100% poprawny kryptograficznie względem klucza publicznego wprowadzonego w commicie `54db3e0` (`[109, 72, 1, 34, 95, 195, ...]`).
- **Status klucza:** Klucz ten współdzielił identyfikator `splogbook-release-2026-01` ze skompromitowanym kluczem ujawnionym w historii Git. Zgodnie z zasadą ostrożności kryptograficznej klucz ten został wpisany na listę `REVOKED_KEY_IDS` i jest uznany za odwołany.
- **Status integralności artefaktów:**
  - `SPLogbook-Linux-x86_64.tar.gz`:
    - SHA-256 w manifeście: `3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93`
    - SHA-256 z API GitHub (digest): `sha256:3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93` (**ZGODNY**)
  - `SPLogbook-Windows-x64.zip`:
    - SHA-256 w manifeście: `bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0`
    - SHA-256 z API GitHub (digest): `sha256:bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0` (**ZGODNY**)
- **Ocena bezpieczeństwa:** Wydanie nie wykazuje cech manipulacji przez intruza zewnętrznego (hashe artefaktów idealnie odpowiadają zadeklarowanym w manifeście wygenerowanym przez potok CI), jednak ze względu na odwołanie klucza `splogbook-release-2026-01`, auto-updater aplikacji w wersjach bezpiecznych odrzuca ten manifest jako `UntrustedKey`.

---

## 4. Wnioski i zalecenia dla Właściciela

1. **Wydania v1.0.0 – v1.1.0:** Pozostają wydaniami historycznymi (bez mechanizmu manifestu). Użytkownicy tych wersji nie mogą przeprowadzić bezpiecznej automatycznej aktualizacji i wymagają pobrania instalatora ręcznie z zaufanego źródła.
2. **Wydanie v1.2.0:** Pozostaje jako `VERIFIED_REVOKED_KEY`. W ramach Etapu 2 (CI-001 / REL-001) zaleca się ponowne podpisanie artefaktów kluczem `splogbook-release-2026-02` lub wydanie zaktualizowanej wersji v1.2.1.
3. Żadne nieznane ani uszkodzone binarne artefakty nie zostały uruchomione w trakcie weryfikacji audytowej.
