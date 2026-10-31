# Procedura Rotacji Kluczy Podpisywania i Raport Incydentu SEC-001

## 1. Zakres incydentu SEC-001

W ramach audytu bezpieczeństwa repozytorium SPLogbook zidentyfikowano incydent ujawnienia kryptograficznego materiału kluczy prywatnych Ed25519 wykorzystywanych do asymetrycznego podpisywania manifestów wydań aplikacji (`release-manifest.json.sig`) w mechanizmie auto-updatera (W8 / `src/cloud/updater.rs`).

Incydent objął:
1. Śledzenie w drzewie repozytorium pomocniczego pliku źródłowego `src/scratch.rs`, zawierającego trywialny klucz prywatny / ziarno testowe Ed25519 (`[1u8; 32]`).
2. Obecność w historii Git plików kluczy prywatnych o rozmiarze 32 bajtów (`klucz.key`, `.signer_priv.key`, `.test.key`).
3. Bezpośrednie powiązanie ujawnionego w historii pliku `klucz.key` z historycznym produkcyjnym kluczem weryfikacyjnym (`[203, 142, 210, 212...]`), zarejestrowanym pod identyfikatorem `splogbook-release-2026-01`.
4. Wykorzystanie tego samego identyfikatora `key_id: "splogbook-release-2026-01"` po rotacji klucza w commicie `54db3e0`, co uniemożliwiało bezpieczne rozróżnienie i odwołanie skompromitowanego klucza bez formalnego mechanizmu revocation.

---

## 2. Metadane i osi czasu incydentu

- **Identyfikator incydentu:** `SEC-001`
- **Klasyfikacja:** `CRITICAL` / `BLOCKER` (CWE-321: Use of Hard-coded Cryptographic Key, CWE-312: Cleartext Storage of Sensitive Information)
- **Data wykrycia audytowego:** 2026-10-27 (pełny audyt programu SPLogbook)
- **Pierwsze znane wprowadzenie do historii Git:** commit `80533e2f220d92bd5499f7ff3b28b95acdbdecc3`
- **Wycofanie części plików binarnych kluczy z drzewa:** commit `8296569` (usunięto `klucz.key`, `.signer_priv.key`, `.test.key`, lecz pozostawiono je w historii oraz pozostawiono `src/scratch.rs` w bieżącym drzewie)
- **Późniejsza modyfikacja nagłówków w `src/scratch.rs`:** commit `054f9c2` (dodanie SPDX)
- **Częściowa podmiana publicznego klucza:** commit `54db3e0` (podmiana tablicy bajtów z zachowaniem starego identyfikatora `splogbook-release-2026-01`, bez procedury revocation)
- **Status bieżący w Etapie 1A:** `REMEDIATED_IN_TREE` / `REGRESSION_GUARDED` / `BLOCKED_EXTERNAL_SECRET`

---

## 3. Lista plików objętych incydentem

| Lp. | Ścieżka pliku | Rozmiar | Skrót SHA-256 (całość pliku) | Fingerprint publiczny Ed25519 (SHA-256) | Status w repozytorium |
|---|---|---|---|---|---|
| 1 | `src/scratch.rs` | 301 B | `c6fdcca7de7fc4f09b14e88be72877b679ebd5d6e87b502f9ed90394a3af5495` | `34750f98bd59fcfc...` | Usunięty z drzewa (`git rm`), zablokowany w `.gitignore` i testach |
| 2 | `klucz.key` | 32 B | `ebc0ae0020741843f4751d481b6c06b39abf7e683be902b9011b2e01516b2d1a` | `d283d0ff3b5ee834...` | Usunięty w `8296569`, obecny w historii, odwołany w kodzie |
| 3 | `.signer_priv.key` | 32 B | `e44159ffe2077c76936a3fbe99cb5551ccaaaf76d7f300226dc16753be084ba3` | `5b880383b63aa2b7...` | Usunięty w `8296569`, obecny w historii, odwołany w kodzie |
| 4 | `.test.key` | 32 B | `a2e1066019db1401974ff1e905cebc2edea335fc87a04f250eda457cf18f28ec` | `6cf71dec642cd4dd...` | Usunięty w `8296569`, obecny w historii, odwołany w kodzie |

---

## 4. Tagi i zakres wydań wymagających weryfikacji

Commit wprowadzający materiał (`80533e2`) oraz commity pochodne weszły w zakres następujących tagów wydań i punktów kontrolnych:
- `v1.2.0` (wydanie oficjalne)
- `refactor-baseline`
- `refactor-w0-done` do `refactor-w8-done`
- `refactor-w6-closure-done`
- `refactor-w7-sync-stability-done`

Wydanie `v1.2.0` oraz wszelkie artefakty podpisane historycznym kluczem `splogbook-release-2026-01` wymagają weryfikacji integralności i ponownego podpisania po zakończeniu procedury rotacji.

---

## 5. Status kryptograficzny dotychczasowych kluczy

- **Klucz `splogbook-release-2026-01`:** `COMPROMISED` / `REVOKED`.
- **Klucze testowe (`klucz.key`, `scratch.rs`, `.signer_priv.key`, `.test.key`):** `REVOKED`.
- **Klucz wprowadzony w `54db3e0`:** `SUSPENDED` / `FAIL-CLOSED` (niepotwierdzone pochodzenie poza repozytorium, współdzielenie identyfikatora z kluczem kompromitowanym).
- **Zasada nadrzędna:** **NIGDY WIĘCEJ NIE PODPISYWAĆ WYDAŃ ANI MANIFESTÓW ŻADNYM Z POWYŻSZYCH KLUCZY.**

---

## 6. Procedura wygenerowania nowego produkcyjnego klucza (czynność ręczna właściciela)

> [!CAUTION]
> Nowy produkcyjny klucz prywatny Ed25519 MUSI zostać wygenerowany wyłącznie przez właściciela repozytorium na bezpiecznej, zaufanej stacji roboczej, POZA drzewem katalogowym repozytorium SPLogbook.
> Nigdy nie umieszczaj pliku klucza prywatnego w repozytorium ani w żadnym podkatalogu projektu.

### Instrukcja wykonania przez właściciela:

1. Przejdź do bezpiecznego katalogu tymczasowego poza repozytorium (np. na pamięci RAM lub w zaszyfrowanym wolumenie):
   ```powershell
   # PRZYKŁAD DLA POWERSHELL NA BEZPIECZNEJ STACJI WŁAŚCICIELA:
   # Ścieżka docelowa MUSI leżeć poza repozytorium, np. $HOME/secure_keys/
   mkdir -p "$HOME/splogbook_secrets"
   Set-Location "$HOME/splogbook_secrets"
   ```

2. Uruchom narzędzie `splogbook-signer keygen` wskazując ścieżkę do bezpiecznego magazynu:
   ```bash
   # Wskazanie ścieżki wyjściowej poza repozytorium:
   splogbook-signer keygen /sciezka/bezpieczna/poza/repozytorium/splogbook_release_2026_02_priv.key
   ```

3. Narzędzie wyświetli na standardowym wyjściu tablicę bajtów klucza publicznego (32 bajty).
4. Skopiuj wygenerowany klucz prywatny bezpośrednio do magazynu sekretów:
   - Nazwa sekretu GitHub Actions: `RELEASE_SIGNING_KEY`
   - Format: ciąg znaków Base64 z 32 surowych bajtów klucza prywatnego lub surowy plik zabezpieczony w GitHub Actions Secrets (Environment: `release`).
5. Usuń lokalny plik klucza ze stacji roboczej po zarchiwizowaniu w bezpiecznym sejfie (np. KeePassXC / HSM / YubiKey).

---

## 7. Publikacja i aktywacja nowego klucza publicznego (Etap 1B)

Właściciel repozytorium wygenerował nową parę kluczy Ed25519 poza repozytorium, zabezpieczył klucz prywatny w GitHub Actions Secrets (`RELEASE_SIGNING_KEY`) oraz przekazał agentowi wyłącznie 32 publiczne bajty i ich fingerprint SHA-256.

W `src/cloud/updater.rs` aktywowano nowy klucz publiczny:
```rust
pub const TRUSTED_KEYS: &[TrustedKey] = &[
    TrustedKey {
        key_id: "splogbook-release-2026-02",
        public_key: [
            120, 98, 36, 60, 5, 44, 3, 94, 176, 193, 128, 250, 134, 246, 151, 173, 48, 52, 175,
            70, 250, 58, 209, 168, 53, 6, 216, 212, 62, 225, 232, 68,
        ],
        active: true,
        min_version: Some("1.2.0"),
        max_version: None,
    },
    TrustedKey {
        key_id: "splogbook-release-2026-01",
        public_key: [
            109, 72, 1, 34, 95, 195, 132, 176, 98, 155, 207, 206, 145, 98, 159, 8, 110, 233, 90,
            236, 84, 127, 179, 166, 231, 137, 166, 233, 148, 250, 211, 7,
        ],
        active: false, // Odwołany po incydencie SEC-001
        min_version: Some("1.1.0"),
        max_version: None,
    },
];
```

W repozytorium istnieje dokładnie **jeden aktywny klucz** (`splogbook-release-2026-02`). Stary identyfikator `splogbook-release-2026-01` oraz wszystkie kompromitowane klucze pozostają na stałe odwołane w `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS`.

---

## 8. Audyt historycznych wydań i manifestów

Szczegółowy raport audytu kryptograficznego wydań historycznych znajduje się w osobnym dokumencie:
👉 [docs/HISTORICAL_RELEASE_AUDIT.md](file:///c:/Users/sp6in/Documents/SPLogbook/SPLogbook/docs/HISTORICAL_RELEASE_AUDIT.md)

**Podsumowanie ustaleń audytowych:**
1. Wydania `v1.0.0` do `v1.1.0` zostały sklasyfikowane jako `MISSING_MANIFEST` / `UNSIGNED`. Nie zawierają plików manifestu ani sygnatur.
2. Wydanie `v1.2.0` zawiera manifest `release-manifest.json` oraz podpis `release-manifest.json.sig`. Hashe artefaktów są w 100% zgodne z digestami API GitHub, jednak manifest jest podpisany odwołanym kluczem `splogbook-release-2026-01`. Klasyfikacja: **`VERIFIED_REVOKED_KEY`**.
3. Nowy auto-updater w aplikacji odrzuca manifest `v1.2.0` jako niezaufany (`UntrustedKey`), co chroni przed atakiem podmiany artefaktu (fail-closed).
4. Żadne binarne artefakty nie zostały uruchomione podczas audytu.

---

## 9. Ścieżka migracji użytkowników dotychczasowych wersji (Plan migracji)

1. **Brak zaufania tranzytywnego:** Wersje aplikacji ufające wyłącznie staremu kluczowi (`splogbook-release-2026-01`) nie mogą w bezpieczny i zaufany sposób przyjąć nowego klucza przez kanał podpisany wyłącznie nowym kluczem (`splogbook-release-2026-02`).
2. **Zachowanie starszych instancji:** Wersja pośrednia (Etap 1A) posiada updater w trybie fail-closed (`active: false`). Nowa wersja odrzuca stary klucz jako odwołany (`is_key_revoked`).
3. **Dostarczenie nowej wersji zaufanym kanałem:** Nowa wersja zawierająca zaufany klucz `splogbook-release-2026-02` musi zostać dostarczona użytkownikom za pośrednictwem bezpiecznego, oficjalnego kanału dystrybucyjnego (GitHub Release / oficjalna strona).
4. **Wymóg instalacji ręcznej:** Użytkownicy instalacji archiwalnych (v1.0.0 – v1.2.0) muszą pobrać nową wersję i przeprowadzić jednorazową instalację ręczną.
5. **Weryfikacja instalatora:** Instalatory wydań od wersji v1.2.1 wzwyż muszą być publikowane z:
   - jawnym skrótem SHA-256 w pliku `SHA256SUMS.txt`,
   - podpisanym manifestem `release-manifest.json` oraz sygnaturą `release-manifest.json.sig`,
   - publicznym fingerprintem klucza opublikowanym w niezależnym kanale informacyjnym (np. strona projektu / repozytorium dokumentacji).
6. **Brak fallbacku:** Stary `key_id` (`splogbook-release-2026-01`) pozostaje nieodwracalnie odwołany. Kod nie posiada żadnego trybu fallbacku ani możliwości obejścia weryfikacji.
7. **Zakaz automatycznego zaufania:** Zabrania się tworzenia mechanizmów automatycznego wprowadzania nowego klucza na podstawie manifestów podpisanych starym kluczem (ponieważ stary klucz prywatny był ujawniony w historii Git).
8. **Minimalna wersja obsługująca nowy klucz:** `1.2.0` (dla manifestu o ograniczeniu `min_version: "1.2.0"`).

---

## 10. Procedura awaryjnego wyłączenia auto-updatera (Emergency Kill-Switch)

Jeżeli zachodzi potrzeba natychmiastowego zablokowania mechanizmu aktualizacji we wszystkich działających instancjach aplikacji:
1. **Blokada kodowa:**
   Ustawienie flagi `active: false` w `TRUSTED_KEYS`. Każde wywołanie `verify_manifest_signature` natychmiast kończy się błędem `UpdaterError::UntrustedKey`, zapobiegając instalacji jakichkolwiek plików.
2. **Blokada serwerowa (GitHub Releases):**
   Usunięcie lub zmiana nazwy plików `release-manifest.json.sig` w wydaniach na GitHub powoduje, że klient zgłasza `UpdaterError::MissingSignature`, przerywając procedurę pobierania.

---

## 11. Status historii Git i decyzja właściciela

```text
HISTORY_EXPOSURE: OWNER_DECISION_REQUIRED
```

- **Stan obecny:** Wszystkie pliki zawierające materiał kluczy prywatnych (`src/scratch.rs`, `klucz.key`, `.signer_priv.key`, `.test.key`) zostały usunięte z bieżącego drzewa roboczego i zabezpieczone przed ponownym dodaniem w regułach `.gitignore` oraz testach `tests/security_guard.rs`.
- **Ekspozycja w historii:** Ujawniony w commitach historycznych materiał kryptograficzny pozostaje zapisany w starych obiektach Git.
- **Kryptograficzna neutralizacja:** Poprzez odwołanie identyfikatora `splogbook-release-2026-01` w `REVOKED_KEY_IDS` oraz dodanie skrótów i kluczy publicznych do `REVOKED_PUBLIC_KEYS`, ujawnione historyczne klucze utraciły możliwość autoryzowania aktualizacji w aplikacji SPLogbook.
- **Decyzja Właściciela:** Zgodnie z wytycznymi bezpieczeństwa agent **nie wykonuje** procedury przepisywania historii (`git filter-repo`, BFG, rebase, force-push). Decyzja o ewentualnym przepisaniu historii i unieważnieniu istniejących forków/tagów leży wyłącznie w gestii Właściciela repozytorium.
- **Status dokumentacyjny:** Historia Git **nie jest** oznaczona jako oczyszczona. Pozostaje w statusie `OWNER_DECISION_REQUIRED`.

---

## 12. Checklista czynności właściciela repozytorium

- [x] Usunięcie śledzonego materiału prywatnego z drzewa roboczego (`src/scratch.rs`) — zrealizowane w Etapie 1A.
- [x] Wdrożenie mechanizmu `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS` w `src/cloud/updater.rs` — zrealizowane w Etapie 1A.
- [x] Wdrożenie stanu fail-closed (`active: false`) do czasu dostarczenia nowego klucza — zrealizowane w Etapie 1A.
- [x] Dodanie testów regresyjnych strażnika bezpieczeństwa (`tests/security_guard.rs`) — zrealizowane w Etapie 1A.
- [x] Dodanie kroku kontroli integralności w GitHub Actions CI — zrealizowane w Etapie 1A.
- [x] **[WŁAŚCICIEL]** Wygenerowanie nowej pary kluczy Ed25519 poza repozytorium (`splogbook-release-2026-02`).
- [x] **[WŁAŚCICIEL]** Umieszczenie nowego klucza prywatnego w GitHub Actions Secrets (`RELEASE_SIGNING_KEY`).
- [x] **[WŁAŚCICIEL]** Przekazanie wyłącznie danych publicznych (32 bajty i fingerprint SHA-256).
- [x] Wdrożenie i aktywacja nowego klucza publicznego `splogbook-release-2026-02` w `src/cloud/updater.rs` — zrealizowane w Etapie 1B.
- [x] Pełny audyt kryptograficzny wydań historycznych (`docs/HISTORICAL_RELEASE_AUDIT.md`) — zrealizowane w Etapie 1B.
- [ ] **[WŁAŚCICIEL]** Podjęcie ostatecznej decyzji w sprawie ewentualnego oczyszczenia historii Git (`git-filter-repo`).

---

## 13. Metryka aktywnego klucza publicznego

- **Identyfikator:** `splogbook-release-2026-02`
- **Algorytm:** Ed25519 (32 bajty klucza publicznego)
- **Status:** `ACTIVE` / `PRODUCTION_VERIFIED`
- **Fingerprint publiczny (SHA-256):** `2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a`
- **Zabezpieczenie materiału prywatnego:** Poza repozytorium w GitHub Actions Secret `RELEASE_SIGNING_KEY` (potwierdzone przez Właściciela).
