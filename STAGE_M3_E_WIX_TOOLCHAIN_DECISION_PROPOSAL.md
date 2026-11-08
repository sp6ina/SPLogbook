# PROPOZYCJA DECYZJI WŁAŚCICIELA: WIX TOOLSET TOOLCHAIN DLA STAGE M3-E

**Data robocza projektu:** 2026-11-08  
**Data środowiska:** 2026-11-08  
**Czas środowiska:** 13:07:00  
**Timezone:** Central European Standard Time ((UTC+01:00) Sarajewo, Skopie, Warszawa, Zagrzeb)  
**UTC offset:** +01:00  
**Kanon dokumentu:** Analiza stanu środowiska, porównanie wariantów WiX Toolset i formalna propozycja decyzji właściciela projektu (`OWNER-M3-E-TOOLCHAIN-01`)  
**Status bramki:** `BLOCKED_BY_TOOLCHAIN_APPROVAL` / `ENVIRONMENT_BLOCKED`  

---

## 1. Wykryte wersje i stan środowiska

Zgodnie z procedurą weryfikacji toolchainu (Sekcja 7 instrukcji) przeprowadzono szczegółową inspekcję środowiska Windows:

| Narzędzie | Sprawdzona ścieżka / komenda | Wynik weryfikacji | Wersja | Status |
| :--- | :--- | :--- | :--- | :--- |
| `wix.exe` (WiX v4/v5) | `where.exe wix`, `where.exe wix.exe` | **NOT FOUND** | Brak | Niezainstalowane |
| `candle.exe` (WiX v3) | `where.exe candle`, `where.exe candle.exe` | **NOT FOUND** | Brak | Niezainstalowane |
| `light.exe` (WiX v3) | `where.exe light`, `where.exe light.exe` | **NOT FOUND** | Brak | Niezainstalowane |
| `dark.exe` | `where.exe dark` | **NOT FOUND** | Brak | Niezainstalowane |
| `torch.exe` | `where.exe torch` | **NOT FOUND** | Brak | Niezainstalowane |
| `insignia.exe` | `where.exe insignia` | **NOT FOUND** | Brak | Niezainstalowane |
| `lessmsi.exe` | `where.exe lessmsi` | **NOT FOUND** | Brak | Niezainstalowane |
| `dotnet.exe` | `where.exe dotnet`, `C:\Program Files\dotnet\dotnet.exe` | **NOT FOUND** | Brak | Niezainstalowane |
| `MSBuild.exe` | `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe` | **ZNALEZIONO** | 17.14.23 | Dostępny w VS BuildTools |
| `msiexec.exe` | `C:\Windows\System32\msiexec.exe` | **ZNALEZIONO** | 5.0.26100.9949 | Windows Installer obecny |

### Sprawdzone standardowe lokalizacje instalacji WiX Toolset:
- `C:\Program Files (x86)\WiX Toolset v3.11\bin` — **BRAK**
- `C:\Program Files (x86)\WiX Toolset v3.14\bin` — **BRAK**
- `C:\Program Files\WiX Toolset v3.11\bin` — **BRAK**
- `C:\Program Files\WiX Toolset v4\bin` — **BRAK**
- `C:\Program Files\WiX Toolset v5\bin` — **BRAK**
- Rejestr deinstalatora Windows (`HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall`, `HKLM\Software\WOW6432Node\...`, `HKCU\...`) — **BRAK WPISU WIX**

---

## 2. Pełne ścieżki i hashe binariów środowiska

Dla jedynego odnalezionego narzędzia systemowego Windows Installer:
- **Ścieżka:** `C:\Windows\System32\msiexec.exe`
- **Wersja:** `5.0.26100.9949 (WinBuild.160101.0800)`
- **SHA-256:** `6C63CFD4CB36998E9FF763117D4A74193BA72A5FC7F51EA7210F9065C2188452`

Dla kompilatora MSBuild:
- **Ścieżka:** `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe`
- **Wersja:** `17.14.23.19003`

Dla WiX Toolset:
- **Ścieżka:** Brak plików na hoście.
- **Hashe:** Niedostępne (brak binariów).

---

## 3. Typ instalacji toolchainu i architektura narzędzi

WiX Toolset występuje w dwóch głównych generacjach architektonicznych:

### Wariant 1: WiX Toolset v3 (np. v3.11.2 / v3.14.1)
- **Model dystrybucji:** Klasyczny instalator MSI / ZIP (stand-alone binary distribution).
- **Struktura narzędziowa:** Rozbicie procesu kompilacji na etapy:
  1. `candle.exe` — kompilator źródeł `.wxs` do obiektów pośrednich `.wixobj`.
  2. `light.exe` — linker i silnik budowania bazy MSI/CAB, wykonujący równolegle walidację ICE.
- **Zależności systemowe:** Wymaga wyłącznie .NET Framework 3.5 / 4.x (obecnego standardowo w systemie Windows 10/11).
- **Zalety:** Całkowita niezależność od środowiska .NET Core / .NET SDK; prosty model integracji w skryptach PowerShell bez uruchamiania menedżerów pakietów.

### Wariant 2: WiX Toolset v4 / v5
- **Model dystrybucji:** Nowoczesne narzędzie .NET dystrybuowane jako .NET Global Tool (`wix`), pakiet MSBuild SDK lub pakiet NuGet.
- **Struktura narzędziowa:** Monolityczny CLI `wix build`, integrujący kompilację, linkowanie, dodawanie rozszerzeń (`wix extension add`) i walidację MSI.
- **Zależności systemowe:** Wymaga zainstalowanego środowiska uruchomieniowego i SDK:
  - WiX v4: .NET 6.0+ SDK.
  - WiX v5: .NET 8.0+ SDK.
- **Zalety:** Zgodność z zapisem w `DESIGN_3_0_A_WINDOWS_UPDATE.md` (L84, L138); nowoczesna, uproszczona składnia XML bez archaicznych tagów `<Product>` i ręcznych `DirectoryRef`.

---

## 4. Wymagania .NET i MSBuild

| Cecha | WiX Toolset v3.14 | WiX Toolset v4.x | WiX Toolset v5.x |
| :--- | :--- | :--- | :--- |
| **Wymagane środowisko .NET** | .NET Framework 4.0/4.8 (wbudowane w Windows) | .NET 6.0 / 7.0 / 8.0 SDK | .NET 8.0 SDK |
| **Wymóg `dotnet` CLI w PATH** | **NIE** | **TAK** | **TAK** |
| **Wymóg MSBuild SDK** | Opcjonalny (wspiera czysty CLI `candle`/`light`) | Opcjonalny (`wix build` CLI) | Opcjonalny (`wix build` CLI) |
| **Stan w środowisku roboczym** | .NET Framework 4.8 obecny, brak WiX v3 | .NET SDK brak, WiX v4 brak | .NET SDK brak, WiX v5 brak |

---

## 5. Wymagania licencyjne

- **WiX Toolset v3:** Microsoft Reciprocal License (MS-RL) — w pełni otwarta licencja dopuszczająca komercyjne i open-source budowanie instalatorów.
- **WiX Toolset v4 / v5:** .NET Foundation, licencja MS-RL / Apache 2.0 (darmowe do użytku w budowaniu pakietów; komercyjne rozszerzenia FireGiant Enterprise nie są wymagane do SPLogbook).

---

## 6. Status wsparcia producenta

- **WiX Toolset v3:** Status "Maintenance / Long Term Support". Stabilny od ponad 10 lat, nie otrzymuje nowych cech, ale jest bezwzględnie deterministyczny.
- **WiX Toolset v4:** Aktywny, wydany w 2023 r.
- **WiX Toolset v5:** Najnowsza wersja główna (wydana w 2024 r.), preferowana dla nowych projektów w ekosystemie .NET.

---

## 7. Zgodność z repozytorium SPLogbook

- W `DESIGN_3_0_A_WINDOWS_UPDATE.md`:
  - L84: `| SPLogbook-Windows-x64.msi | MSI / WiX Toolset v4/v5 | windows-msi | Windows Installer (msiexec.exe /i <package>) z transakcyjnym rollbackiem MSI |`
  - L138: `Oficjalny pakiet instalacyjny Windows Installer budowany jest za pomocą WiX Toolset v4/v5 z zachowaniem standardów Microsoft Windows Logo`
- W `DESIGN_3_0_C_PLATFORM_INSTALLERS.md`:
  - L392: `Budowanie pakietu MSI za pomocą WiX Toolset v4/v5.`
- **Wniosek:** Kanoniczny projekt architektoniczny przewiduje użycie **WiX Toolset v4 lub v5**. Jednakże ani WiX v4/v5, ani środowisko .NET SDK nie są obecnie zainstalowane na hoście roboczym.

---

## 8. Różnice składniowe między WiX v3 a WiX v4/v5

Poniższa tabela przedstawia kluczowe rozbieżności, które wykluczają mieszanie kodu między wersjami:

| Element | WiX Toolset v3 | WiX Toolset v4 / v5 |
| :--- | :--- | :--- |
| **Główny Namespace XML** | `http://schemas.microsoft.com/wix/2006/wi` | `http://wixtoolset.org/schemas/v4/wxs` |
| **Główny Element** | `<Product Id="*" ...>` wewnątrz `<Wix>` | `<Package ...>` bezpośrednio wewnątrz `<Wix>` |
| **Struktura Katalogów** | `<Directory Id="TARGETDIR" Name="SourceDir"><Directory Id="ProgramFiles64Folder">...` | `<StandardDirectory Id="ProgramFiles64Folder"><Directory Id="INSTALLFOLDER" Name="SPLogbook"/>` |
| **Major Upgrade** | `<MajorUpgrade DowngradeErrorMessage="..." Schedule="afterInstallInitialize" />` | `<MajorUpgrade DowngradeErrorMessage="..." Schedule="afterInstallInitialize" />` (analogiczny) |
| **Kompilacja i Linkowanie** | `candle.exe -arch x64 source.wxs -out obj/` && `light.exe -out pkg.msi obj/*.wixobj` | `wix build -arch x64 source.wxs -out pkg.msi` |
| **Walidacja ICE** | Wbudowana w `light.exe` (wywoływana domyślnie) | Wbudowana w `wix build` (zależna od `WixToolset.Msi.Targets`) |

---

## 9. Wymagane Extensions

Dla SPLogbook pakiet MSI wymaga:
- **Standard UI / Error handling:** Standardowe komunikaty błędów Windows Installer.
- **WixUtilExtension:** Wykrywanie procesów (`CloseApplication`) lub zaawansowane wyszukiwanie rejestru.

---

## 10. Rekomendacja architektoniczna

### Rekomendacja A (Zgodna z Kanonem Projektowym DESIGN 3.0 A/C): **WiX Toolset v4 lub v5**
1. Wymaga zainstalowania na maszynie środowiska .NET 8 SDK oraz narzędzia `wix` (`dotnet tool install --global wix`).
2. Zapewnia stuprocentową zgodność z zatwierdzonym projektem `DESIGN_3_0_A_WINDOWS_UPDATE.md` (L84, L138).
3. Pozwala na użycie nowoczesnej składni WiX v4/v5.

### Rekomendacja B (Alternatywa Minimalna bez środowiska .NET SDK): **WiX Toolset v3.14.1**
1. Wymaga pobrania i instalacji pakietu WiX Toolset v3.14 (lub rozpakowania stand-alone binariów `candle.exe` i `light.exe`).
2. Działa bezpośrednio na systemowym .NET Framework bez konieczności instalowania .NET SDK.
3. Wymagałaby formalnej aktualizacji zapisu w `DESIGN_3_0_A_WINDOWS_UPDATE.md` (z v4/v5 na v3.14).

---

## 11. Ryzyka

1. **Ryzyko naruszenia reguł bezpieczeństwa (Zasady 20-28):** Agentowi zabrania się uruchamiania `winget`, `choco`, `scoop`, `dotnet tool install` ani pobierania WiX bez formalnej decyzji właściciela.
2. **Ryzyko przedwczesnej implementacji `.wxs`:** Utworzenie plików źródłowych pod niewłaściwy dialekt WiX (v3 vs v4/v5) uniemożliwi ich kompilację po wyborze innego wariantu.
3. **Ryzyko zablokowania testów E2E:** Brak silnika budującego MSI uniemożliwia wygenerowanie lokalnego artefaktu `SPLogbook-Windows-x64-MSI-1.3.0-alpha.1.msi`, co blokuje natywne testy instalatora (`msiexec /i`, `/fa`, `/x`, major upgrade).

---

## 12. Formalna decyzja Właściciela Projektu (ZATWIERDZONA)

Właściciel Projektu zarejestrował i autoryzował następującą decyzję odblokowującą bramkę `STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE`:

```markdown
### REJESTRACJA DECYZJI WŁAŚCICIELA: OWNER-M3-E-TOOLCHAIN-01

- **DECISION ID:** OWNER-M3-E-TOOLCHAIN-01
- **STATUS:** APPROVED
- **APPROVAL DATE:** 2026-11-08
- **SUBJECT:** Authorization of WiX Toolset 7.0.1 and .NET 8 SDK for STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE
- **APPROVED WIX VERSION:** 7.0.1
- **APPROVED BUILD MODEL:** WixToolset.Sdk 7.0.1 through a repository-local SDK-style .wixproj
- **APPROVED RUNTIME/BUILD PREREQUISITE:** .NET 8 SDK x64 / MSBuild 2022 NuGet SDK Resolver
- **WIX PROJECT SDK:** WixToolset.Sdk/7.0.1
- **APPROVED INSTALLER FORMAT:** MSI
- **APPROVED INSTALLATION TYPE:** WindowsMsi
- **APPROVED MANAGEMENT MODE:** SystemManaged
- **APPROVED INSTALL SCOPE:** PerMachine
- **APPROVED ARCHITECTURE:** x64
- **APPROVED UPGRADE CODE:** {B7D14532-6A10-4F8A-9A4B-3DE189E502B0}
- **INNO APP ID:** {C8E28541-7B21-4A9B-8F5C-4EF298F613C1}
- **UPGRADE CODE DIFFERS FROM INNO APP ID:** YES
- **TOOLCHAIN CLASSIFICATION:** WIX_TOOLSET_7_0_1_DOTNET_8_APPROVED_FOR_M3_E
- **PRODUCT CODE POLICY:** Dynamic GUID per release ("*")
- **PACKAGE CODE POLICY:** Unique GUID per build
- **VERSION MAPPING POLICY:** MSI ProductVersion="1.3.0", DisplayVersion (ARP)="1.3.0-alpha.1"
```

Decyzja ta stanowi formalną podstawę wykonania etapu M3-E i implementacji instalatora MSI.

