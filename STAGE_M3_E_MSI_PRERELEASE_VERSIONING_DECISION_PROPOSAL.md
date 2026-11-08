# Propozycja Decyzji Architektonicznej: Polityka Wersjonowania PrzyszĹ‚ych PakietĂłw MSI Prerelease

**Identyfikator propozycji:** OWNER-M3-E-MSI-VERSIONING-01
**Data sporzÄ…dzenia:** 2026-11-08
**Autor:** GĹ‚Ăłwny Audytor Techniczny SPLogbook & Starszy InĹĽynier Systemowy
**Bramka ĹşrĂłdĹ‚owa:** STAGE-M3-E-CLOSURE-CONSISTENCY-GATE
**Status decyzji:** PENDING_OWNER_DECISION
**Priorytet:** P0 â€” KRYTYCZNY DLA PRZYSZĹYCH WYDAĹ PRERELEASE

---

## 1. Stan obecny

W ramach bramki `STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE` wdroĹĽono instalator MSI oparty o WiX Toolset SDK 7.0.1 z nastÄ™pujÄ…cÄ… konfiguracjÄ… toĹĽsamoĹ›ci:
- **SemVer aplikacji:** `1.3.0-alpha.1` (zgodnie z `Cargo.toml`)
- **MSI ProductVersion:** `1.3.0` (wyliczany przez funkcjÄ™ `semver_to_msi_product_version(&str)` odcinajÄ…cÄ… sufiks prerelease)
- **DisplayVersion w rejestrze ARP:** `1.3.0-alpha.1`
- **ProductCode:** Generowany dynamicznie per-build (`ProductCode="*"`)
- **UpgradeCode:** StaĹ‚y GUID rodziny produktowej `{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}`
- **AllowSameVersionUpgrades:** `no` (wartoĹ›Ä‡ domyĹ›lna reguĹ‚y `<MajorUpgrade>` w WiX 4/7)
- **MajorUpgrade Schedule:** `afterInstallInitialize`

Dla bieĹĽÄ…cego wydania `1.3.0-alpha.1` stan ten jest w 100% poprawny i bezpieczny w relacji do publicznej wersji stabilnej `1.2.0`, poniewaĹĽ `1.2.0 < 1.3.0` (porzÄ…dek Ĺ›ciĹ›le monotoniczny).

---

## 2. Ograniczenia MSI ProductVersion

Standard Windows Installer (MSI) nakĹ‚ada sztywne ograniczenia na format i ewaluacjÄ™ wersji pakietu (`ProductVersion`):
1. **Format:** `Major.Minor.Build[.Revision]`, gdzie:
   - `Major`: liczba caĹ‚kowita `0..255`
   - `Minor`: liczba caĹ‚kowita `0..255`
   - `Build`: liczba caĹ‚kowita `0..65535`
   - `Revision`: liczba caĹ‚kowita `0..65535` (opcjonalna)
2. **Ignorowanie czwartego pola:** Windows Installer podczas wykonywania akcji standardowej `FindRelatedProducts` oraz ewaluacji tabeli `Upgrade` (w tym `<MajorUpgrade>`) **caĹ‚kowicie ignoruje czwarte pole (`Revision`)**. Oznacza to, ĹĽe pakiety rĂłĹĽniÄ…ce siÄ™ wyĹ‚Ä…cznie czwartym czĹ‚onem sÄ… traktowane przez mechanizm Major Upgrade jako pakiety o identycznej wersji!
3. **Brak obsĹ‚ugi ciÄ…gĂłw alfanumerycznych:** Format MSI nie dopuszcza oznaczeĹ„ tekstowych takich jak `-alpha.1`, `-beta.2` czy `-rc.1`. PrĂłba wprowadzenia znakĂłw nieliczbowych skutkuje bĹ‚Ä™dem kompilacji WiX.

---

## 3. Konsekwencje wspĂłlnego ProductVersion 1.3.0

JeĹĽeli funkcja `semver_to_msi_product_version` nadal bÄ™dzie mapowaÄ‡ wszystkie przyszĹ‚e wersje prerelease do formatu `Major.Minor.Patch`:
- `1.3.0-alpha.1` -> `1.3.0`
- `1.3.0-alpha.2` -> `1.3.0`
- `1.3.0-beta.1` -> `1.3.0`
- `1.3.0-rc.1` -> `1.3.0`
- `1.3.0` (stable) -> `1.3.0`

z punktu widzenia Windows Installer wszystkie te pakiety posiadajÄ… **dokĹ‚adnie tÄ™ samÄ… wersjÄ™ `ProductVersion = 1.3.0`**, mimo ĹĽe z punktu widzenia specyfikacji SemVer 2.0 zachodzi miÄ™dzy nimi Ĺ›cisĹ‚y porzÄ…dek:
`1.3.0-alpha.1 < 1.3.0-alpha.2 < 1.3.0-beta.1 < 1.3.0-rc.1 < 1.3.0`

---

## 4. Konsekwencje AllowSameVersionUpgrades=no

Przy obecnej konfiguracji WiX:
```xml
<MajorUpgrade
    DowngradeErrorMessage="Wykryto nowsza wersje SPLogbook. Instalacja starszej wersji (downgrade) zostala zablokowana."
    Schedule="afterInstallInitialize" />
```
DomyĹ›lne ustawienie `AllowSameVersionUpgrades="no"` generuje wpis w tabeli `Upgrade` z warunkiem:
`VersionMax="1.3.0"` oraz `IncludeMaximum="no"`.

W konsekwencji:
1. Podczas prĂłby instalacji `1.3.0-alpha.2` na stacji roboczej z zainstalowanym `1.3.0-alpha.1`, funkcja `FindRelatedProducts` **nie zakwalifikuje** zainstalowanego `1.3.0` jako wersji do podniesienia (`WIX_UPGRADE_DETECTED` nie zostanie ustawione).
2. PoniewaĹĽ pakiet ma inny `ProductCode`, Windows Installer wykryje konflikt toĹĽsamoĹ›ci tego samego produktu o tej samej wersji i **przerwie instalacjÄ™ bĹ‚Ä™dem 1638** (`ERROR_PRODUCT_VERSION`: *Another version of this product is already installed*).
3. **Zablokowane zostanÄ… wszystkie bezpoĹ›rednie przejĹ›cia:**
   - `alpha.1 -> alpha.2`
   - `alpha.2 -> beta.1`
   - `beta.1 -> rc.1`
   - `rc.1 -> 1.3.0 (stable)`
4. UĹĽytkownik musiaĹ‚by rÄ™cznie odinstalowaÄ‡ starszy prerelease przed instalacjÄ… nowszego.

---

## 5. Ryzyko wĹ‚Ä…czenia AllowSameVersionUpgrades=yes

RozwaĹĽenie prostej zmiany w WiX na `AllowSameVersionUpgrades="yes"` niesie **krytyczne ryzyko architektoniczne**:
1. WiX ustawi wtedy `IncludeMaximum="yes"`, co pozwoli na odinstalowanie istniejÄ…cej wersji `1.3.0` i zainstalowanie nowej.
2. Jednak Windows Installer **nie zna relacji SemVer**. Dla instalatora MSI oba pakiety majÄ… wersjÄ™ `1.3.0`.
3. W efekcie instalator nie jest w stanie odrĂłĹĽniÄ‡ instalacji nowszej `alpha.2` na `alpha.1` od przypadkowej instalacji starszej `alpha.1` na `rc.1` czy wrÄ™cz na finalnej wersji stabilnej `1.3.0`!
4. **Ochrona przed downgrade przestaje dziaĹ‚aÄ‡ w ramach caĹ‚ej linii `1.3.0`**: uĹĽytkownik lub skrypt mĂłgĹ‚by przypadkowo nadpisaÄ‡ wersjÄ™ stabilnÄ… lub RC wczesnÄ… wersjÄ… alfa bez ĹĽadnego ostrzeĹĽenia.

---

## 6. MoĹĽliwe deterministyczne mapowania wersji MSI

Aby zapewniÄ‡ zarĂłwno monotoniczny upgrade, jak i peĹ‚nÄ… ochronÄ™ przed downgrade bez utraty spĂłjnoĹ›ci SemVer, analizuje siÄ™ cztery strategie:

### Wariant 1 (Rekomendowany): Monotoniczne mapowanie pola Build z bazÄ… prerelease
W formacie `Major.Minor.Build` pole `Build` przyjmuje wartoĹ›ci `0..65535`. MoĹĽna zdefiniowaÄ‡ deterministyczne kodowanie dla kanaĹ‚Ăłw prerelease:
- Wersja stabilna: `Build = 0` (np. `1.3.0.0`)
- Aby zachowaÄ‡ porzÄ…dek `prerelease < stable`, pakiety prerelease dla linii `1.3.0` otrzymujÄ… bazÄ™ `Minor = 2` (poprzedni minor) z wysokim zakresem `Build`:
  - `1.2.0` (stable) -> `1.2.0`
  - `1.3.0-alpha.1` -> `1.2.9101`
  - `1.3.0-alpha.2` -> `1.2.9102`
  - `1.3.0-beta.1` -> `1.2.9201`
  - `1.3.0-rc.1` -> `1.2.9301`
  - `1.3.0` (stable) -> `1.3.0` (poniewaĹĽ `1.3.0 > 1.2.9301`!)

**Zalety:** ĹšcisĹ‚a zgodnoĹ›Ä‡ z reguĹ‚ami Windows Installer, peĹ‚na obsĹ‚uga `<MajorUpgrade AllowSameVersionUpgrades="no">`, brak ryzyka downgrade, automatyczne podniesienie do wersji stabilnej. `DisplayVersion` w panelu Dodaj/UsuĹ„ programy nadal wyĹ›wietla czytelne `1.3.0-alpha.2`.

### Wariant 2: Monotoniczny numer kompilacji CI/CD w polu Build
Wersja MSI przyjmuje format `1.3.<BUILD_NUMBER>`:
- `1.3.0-alpha.1` (build 101) -> `1.3.101`
- `1.3.0-alpha.2` (build 108) -> `1.3.108`
- `1.3.0-beta.1` (build 142) -> `1.3.142`
- `1.3.0` (stable, build 200) -> `1.3.200`

**Zalety:** Bardzo proste, naturalnie rosnÄ…ce.
**Wady:** Wymaga Ĺ›cisĹ‚ego przekazywania monotonicznego licznika z pipeline'u CI do skryptu budujÄ…cego MSI.

### Wariant 3: Pakiety MSI wyĹ‚Ä…cznie dla wydaĹ„ stabilnych (Prerelease Portable / Inno only)
Pakiety MSI (systemowe per-machine) sÄ… publikowane wyĹ‚Ä…cznie dla wydaĹ„ stabilnych (`1.2.0`, `1.3.0`, `1.4.0`). Wydania prerelease (`alpha`, `beta`, `rc`) sÄ… dystrybuowane wyĹ‚Ä…cznie jako pakiety per-user (Inno Setup) oraz Portable ZIP.

**Zalety:** Zero narzutu na sztuczne mapowanie wersji MSI, peĹ‚na separacja Ĺ›rodowisk testowych od instalacji systemowych.

---

## 7. WpĹ‚yw na downgrade

- Przy obecnym stanie: downgrade wewnÄ…trz prerelease nie jest blokowany dedykowanym komunikatem downgrade, lecz caĹ‚a instalacja (upgrade i downgrade) jest blokowana bĹ‚Ä™dem 1638.
- Przy Wariancie 1: prĂłba zainstalowania `1.3.0-alpha.1` na maszynie z `1.3.0-alpha.2` zostanie natychmiast przechwycona przez `FindRelatedProducts` jako `WIX_DOWNGRADE_DETECTED` (`1.2.9102 > 1.2.9101`) i zatrzymana oficjalnym komunikatem DowngradeErrorMessage.

---

## 8. WpĹ‚yw na stable 1.3.0

Gdy nadejdzie moment wydania stabilnego `1.3.0`:
- Przy obecnym stanie: instalacja `1.3.0` MSI na maszynie z `1.3.0-alpha.1` MSI zostanie **zablokowana bĹ‚Ä™dem 1638**, zmuszajÄ…c uĹĽytkownika do odinstalowania wersji alfa.
- Przy Wariancie 1: instalacja `1.3.0` MSI bez problemu usunie wersjÄ™ prerelease (`1.3.0 > 1.2.9xxx`), zachowujÄ…c bazy danych i konfiguracjÄ™ dziÄ™ki zaimplementowanym w M3-E reguĹ‚om `NeverOverwrite="yes"`.

---

## 9. WpĹ‚yw na ProductCode

`ProductCode` musi pozostaÄ‡ dynamiczny (`ProductCode="*"`) dla kaĹĽdego nowego wydania. Jest to podstawowy warunek prawidĹ‚owego dziaĹ‚ania mechanizmu Major Upgrade w Windows Installer.

---

## 10. WpĹ‚yw na UpgradeCode

`UpgradeCode` pozostaje Ĺ›ciĹ›le niezmienny: `{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}`. Identyfikuje on caĹ‚Ä… rodzinÄ™ produktowÄ… SPLogbook Windows System Setup.

---

## 11. WpĹ‚yw na updater (splogbook-updater)

ModuĹ‚ `src/cloud/updater/installation.rs` zawiera funkcjÄ™:
```rust
pub fn semver_to_msi_product_version(semver_str: &str) -> Result<String, UpdaterError>
```
oraz test jednostkowy `msi_product_version_mapping`.
JeĹĽeli wĹ‚aĹ›ciciel zatwierdzi Wariant 1 lub 2, funkcja ta zostanie zaktualizowana w dedykowanej bramce implementacyjnej, aby updater delegujÄ…c instalacjÄ™ MSI przekazywaĹ‚ i weryfikowaĹ‚ wĹ‚aĹ›ciwy ProductVersion.

---

## 12. Rekomendacja techniczna

1. **Dla bieĹĽÄ…cej bramki M3-E Consistency Gate:**
   - Nie dokonywaÄ‡ zmian funkcjonalnych w kodzie ani projekcie WiX.
   - UznaÄ‡ bieĹĽÄ…cy pakiet `1.3.0-alpha.1` MSI za poprawny i bezpieczny (jest pierwszym wydaniem w linii 1.3.x i prawidĹ‚owo zastÄ™puje stabilne 1.2.0).
   - OznaczyÄ‡ status przyszĹ‚ych pakietĂłw MSI prerelease jako: `BLOCKED_BY_OWNER_VERSIONING_DECISION`.
2. **Dla przyszĹ‚ego etapu przed wydaniem 1.3.0-alpha.2:**
   - WdroĹĽyÄ‡ Wariant 1 (baza minor z kodowaniem kanaĹ‚u w polu Build) lub Wariant 3 (ograniczenie MSI do wydaĹ„ stabilnych).

---

## 13. Wymagane testy

Po zatwierdzeniu wybranego wariantu przez WĹ‚aĹ›ciciela Architektury:
1. Test jednostkowy parsowania i transformacji wersji SemVer -> MSI ProductVersion w `installation.rs`.
2. Test macierzy przejĹ›Ä‡ upgrade:
   - `1.2.0 -> 1.3.0-alpha.1` (PASS)
   - `1.3.0-alpha.1 -> 1.3.0-alpha.2` (PASS)
   - `1.3.0-alpha.2 -> 1.3.0-beta.1` (PASS)
   - `1.3.0-beta.1 -> 1.3.0-rc.1` (PASS)
   - `1.3.0-rc.1 -> 1.3.0` (PASS)
3. Test blokady downgrade:
   - `1.3.0-alpha.2 -> 1.3.0-alpha.1` (BLOCKED)
   - `1.3.0 -> 1.3.0-rc.1` (BLOCKED)
4. Weryfikacja statyczna tabeli `Upgrade` w artefaktach WiX.

---

## 14. Wymagana decyzja wĹ‚aĹ›ciciela (Owner Decision Form)

ProszÄ™ o wybĂłr jednego z wariantĂłw przed rozpoczÄ™ciem prac nad kolejnym wydaniem prerelease MSI (`1.3.0-alpha.2`):

- [ ] **OPCJA A:** WdroĹĽenie Wariantu 1 (deterministyczne kodowanie kanaĹ‚Ăłw prerelease w polu Build z bazÄ… poprzedniego minora).
- [ ] **OPCJA B:** WdroĹĽenie Wariantu 2 (wstrzykiwanie monotonicznego licznika kompilacji CI/CD w pole Build).
- [ ] **OPCJA C:** WdroĹĽenie Wariantu 3 (wycofanie pakietĂłw MSI z kanaĹ‚Ăłw prerelease; dystrybucja MSI wyĹ‚Ä…cznie dla wydaĹ„ stabilnych GA).
- [ ] **OPCJA D:** Inne wytyczne architektoniczne WĹ‚aĹ›ciciela.
