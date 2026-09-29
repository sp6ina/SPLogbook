# Raport Inwentaryzacji Artefaktów (VERIFY-ARTIFACTS)

## 1. Analiza workflow budowania (POTWIERDZONE W WORKFLOW)

Plik: `.github/workflows/build-and-release.yml`

### Linux (build-linux)
- **System CI:** ubuntu-latest
- **Kompilacja:** `cargo build --release`
- **Dodatkowe komponenty:** Hamlib 4.7.2 (budowany ze źródeł)
- **Struktura katalogu tymczasowego (`SPLogbook-Linux-x86_64`):**
  - `splogbook` (binarka, ze zmienionym RPATH)
  - `splogbook.desktop`
  - `run.sh`
  - `icon.png`
  - `databases/serviceLOG.db` (i opcjonalnie `callbook.db`)
  - `README.md`, `LICENSE`
  - `hamlib/bin/` (narzędzia Hamlib)
  - `hamlib/lib/` (biblioteki współdzielone Hamlib)
  - `rigctld`, `rotctld` (kopie w głównym katalogu)
  - pliki `.so*` (kopie w głównym katalogu)
- **Format docelowy:** `SPLogbook-Linux-x86_64.tar.gz` (archiwum tarball)

### Windows (build-windows)
- **System CI:** windows-latest
- **Kompilacja:** `cargo build --release`
- **Dodatkowe komponenty:** Hamlib 4.7.2 (pobierane prekompilowane binarki)
- **Struktura katalogu tymczasowego (`SPLogbook-Windows-x64`):**
  - `SPLogbook.exe`
  - `icon.ico`
  - `databases/serviceLOG.db` (i opcjonalnie `callbook.db`)
  - `README.md`, `LICENSE`
  - pliki DLL i binarki Hamlib w katalogu głównym (skopiowane z `bin/*`)
  - `hamlib/` (cała zawartość `hamlib-w64-4.7.2`)
- **Format docelowy:** `SPLogbook-Windows-x64.zip` (archiwum ZIP)

## 2. Analiza procesu instalacji w kodzie (POTWIERDZONE W KODZIE)

Plik: `src/cloud/updater.rs`

- **Mechanizm pobierania:** Funkcja `install_update` pobiera zasób wskazany w `ReleaseAsset` do pliku tymczasowego `.SPLogbook_update_<pid>.tmp`.
- **Wybór pliku instalacyjnego (`select_asset_for_platform`):**
  - Windows: `.exe`, `.zip`, `.msi` (wybierze `.zip`, bo tylko taki jest publikowany i pasuje w kolejności jako 2)
  - Linux: `.appimage`, `.deb`, `.tar.gz` (wybierze `.tar.gz`, bo tylko taki jest publikowany i pasuje w kolejności jako 3)
- **Błąd w instalatorze Windows:** Funkcja `install_via_powershell` wykonuje `Move-Item -Force -LiteralPath $new -Destination $exe`. Podmienia plik wykonywalny `SPLogbook.exe` bezpośrednio archiwum `.zip` (bez jego wyodrębnienia), co fizycznie uszkadza aplikację.
- **Błąd w instalatorze Linux:** Funkcja `self_replace` wykonuje `std::fs::rename(new, current)`. Podmienia binarkę `splogbook` archiwum `.tar.gz`, również prowadząc do błędu formatu przy następnym uruchomieniu.
- **Brak uwzględnienia danych użytkownika:** Ze względu na to że archiwa nigdy nie są rozpakowywane, kod w ogóle nie obsługuje zapisu starych/nowych baz z katalogu `databases/`.

## 3. Uprawnienia i lokalizacje danych (POTWIERDZONE W KODZIE / HIPOTEZA)

- Kod `updater.rs` uzyskuje ścieżkę do pliku poprzez `std::env::current_exe()` i zakłada, że w jego folderze nadrzędnym można zapisać plik tymczasowy pobierania oraz nadpisać binarkę.
- Hipoteza: Przy instalacjach systemowych (w `C:\Program Files` w Windows, lub `/opt`/`/usr/bin` w Linuksie), operacje te wyrzucą błąd uprawnień, ponieważ updater nie żąda wyższych uprawnień. Jednak z racji pociągania ZIP/tar.gz ten proces i tak by aplikację uszkodził.

## 4. Konkluzje i rekomendacje

Potwierdzono w 100% błędy opisane w `MASTER_IMPLEMENTATION_PLAN.md`.
Należy przystąpić do stworzenia bramek bezpieczeństwa (SAFETY GATES) zadeklarowanych w planie jako etap M1.
