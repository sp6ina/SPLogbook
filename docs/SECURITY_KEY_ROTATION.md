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

## 7. Publikacja nowego klucza publicznego w repozytorium

Po wygenerowaniu nowego klucza przez właściciela:
1. Zdefiniuj nowy unikalny identyfikator, np. `splogbook-release-2026-02`.
2. Wprowadź wyłącznie publiczne bajty do `TRUSTED_KEYS` w `src/cloud/updater.rs`:
   ```rust
   pub const TRUSTED_KEYS: &[TrustedKey] = &[
       TrustedKey {
           key_id: "splogbook-release-2026-02",
           public_key: [ /* 32 bajty nowego klucza publicznego dostarczone przez właściciela */ ],
           active: true,
           min_version: Some("1.2.1"),
           max_version: None,
       },
   ];
   ```
3. Uzupełnij pole fingerprintu w niniejszym dokumencie (sekcja 12).

---

## 8. Audyt historycznych wydań i manifestów

1. Pobrać wszystkie dotychczasowe pliki `release-manifest.json` oraz `release-manifest.json.sig` z wydań GitHub.
2. Zweryfikować, czy jakikolwiek manifest został podpisany kluczem innym niż autoryzowany.
3. Wycofać stare manifesty podpisane kluczem `splogbook-release-2026-01`.
4. Wygenerować nowe manifesty dla wersji wspieranych i podpisać je nowym kluczem `splogbook-release-2026-02` po wdrożeniu potoku CI (Etap 2 / `CI-001`).

---

## 9. Ścieżka migracji użytkowników dotychczasowych wersji

- Wersje `1.1.0` - `1.2.0` zawierają zaufanie do skompromitowanego identyfikatora klucza `splogbook-release-2026-01`.
- Nowa wersja (od `1.2.1`) posiada unieważniony klucz `splogbook-release-2026-01` w `REVOKED_KEY_IDS` oraz odwołane publiczne klucze w `REVOKED_PUBLIC_KEYS`.
- Do czasu pobrania nowej wersji użytkownicy powinni zostać poinformowani o konieczności ręcznej aktualizacji instalatorem ze strony oficjalnej lub aktualizator automatyczny musi zostać zablokowany po stronie serwera/wydania (brak publikacji starych podpisów).

---

## 10. Procedura awaryjnego wyłączenia auto-updatera (Emergency Kill-Switch)

Jeżeli zachodzi potrzeba natychmiastowego zablokowania mechanizmu aktualizacji we wszystkich działających instancjach aplikacji:
1. **Blokada kodowa (wdrożona w Etapie 1A):**
   Wszystkie klucze w `TRUSTED_KEYS` mają ustawiony parametr `active: false`. Każde wywołanie `verify_manifest_signature` natychmiast kończy się błędem `UpdaterError::UntrustedKey`, zapobiegając instalacji jakichkolwiek plików.
2. **Blokada serwerowa (GitHub Releases):**
   Usunięcie lub zmiana nazwy plików `release-manifest.json.sig` w wydaniach na GitHub powoduje, że klient zgłasza `UpdaterError::MissingSignature`, przerywając procedurę pobierania.

---

## 11. Opcjonalna procedura oczyszczenia historii Git (decyzja właściciela)

> [!WARNING]
> Przepisanie historii Git za pomocą `git-filter-repo` lub BFG:
> - Zmienia sumy kontrolne SHA commitów w całym projekcie.
> - Wymaga unieważnienia i usunięcia istniejących forków, gałęzi oraz tagów wydań.
> - Wymaga wykonania `git push --force --all` i `git push --force --tags`.
> - **NIE MOŻE BYĆ WYKONANE AUTOMATYCZNIE W ETAPIE 1A.** Decyzja należy wyłącznie do właściciela repozytorium.

Procedura referencyjna (wyłącznie do celów informacyjnych dla właściciela, jeśli podejmie decyzję o oczyszczeniu):
```bash
# WYMAGA ZAINSTALOWANEGO git-filter-repo ORAZ ŚWIEŻEGO SKLONOWANIA REPOZYTORIUM JAKO MIRROR:
git clone --bare https://github.com/sp6ina/SPLogbook.git splogbook-clean.git
cd splogbook-clean.git

# Usunięcie skompromitowanych ścieżek z całej historii:
git filter-repo --invert-paths \
  --path src/scratch.rs \
  --path .signer_priv.key \
  --path .test.key \
  --path klucz.key

# Po weryfikacji - force-push do repozytorium:
# git push origin --force --all
# git push origin --force --tags
```

---

## 12. Checklista czynności właściciela repozytorium

- [x] Usunięcie śledzonego materiału prywatnego z drzewa roboczego (`src/scratch.rs`) — zrealizowane w Etapie 1A.
- [x] Wdrożenie mechanizmu `REVOKED_KEY_IDS` i `REVOKED_PUBLIC_KEYS` w `src/cloud/updater.rs` — zrealizowane w Etapie 1A.
- [x] Wdrożenie stanu fail-closed (`active: false`) do czasu dostarczenia nowego klucza — zrealizowane w Etapie 1A.
- [x] Dodanie testów regresyjnych strażnika bezpieczeństwa (`tests/security_guard.rs`) — zrealizowane w Etapie 1A.
- [x] Dodanie kroku kontroli integralności w GitHub Actions CI — zrealizowane w Etapie 1A.
- [ ] **[WŁAŚCICIEL]** Wygenerowanie nowej pary kluczy Ed25519 poza repozytorium (`splogbook-release-2026-02`).
- [ ] **[WŁAŚCICIEL]** Umieszczenie nowego klucza prywatnego w GitHub Actions Secrets (`RELEASE_SIGNING_KEY`).
- [ ] **[WŁAŚCICIEL]** Dostarczenie 32 bajtów nowego klucza publicznego do wklejenia w `TRUSTED_KEYS`.
- [ ] **[WŁAŚCICIEL]** Podjęcie decyzji w sprawie ewentualnego oczyszczenia historii Git (`git-filter-repo`).

---

## 13. Metryka nowego klucza publicznego

- **Identyfikator docelowy:** `splogbook-release-2026-02`
- **Status procedury:** `BLOCKED_EXTERNAL_SECRET`
- **Fingerprint nowego klucza publicznego (SHA-256):** `[OCZEKUJE NA WYGENEROWANIE PRZEZ WŁAŚCICIELA]`
