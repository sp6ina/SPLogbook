# Raport Analizy GitHub Digest API (REPORT_3_1_A)

## 1. Ustalenia API GitHub (POTWIERDZONE W SPECYFIKACJI / API)

Na podstawie weryfikacji na żywym organizmie punktu końcowego API GitHub `https://api.github.com/repos/sp6ina/SPLogbook/releases/latest`, dla obiektu wydań `assets` API natywnie dostarcza pole `digest`.

### Format wartości `digest`
Pole to zawiera wartość w postaci tekstowej z jawnym wskazaniem użytego algorytmu.
Przykład pobrany ze zwróconej odpowiedzi dla wydania v1.1.0:
```json
"digest": "sha256:f5ccb1ccc73deaa5b90847333afa80ed74f95bd39ebec4b8ddd4cd791afcc736"
```

## 2. Istniejąca implementacja w aktualizatorze (POTWIERDZONE W KODZIE)

W pliku `src/cloud/updater.rs` aplikacja SPLogbook już poprawnie desuje pole (w formacie `Option<String>`) podczas pobierania informacji o wygenerowanych artefaktach, w obrębie tablicy JSON i przypisuje je do instancji struktury `ReleaseAsset` (linie ok. 205).
Istnieje również działająca metoda weryfikacyjna wycinająca ten prefiks:
```rust
pub fn verify_sha256(data: &[u8], expected: &str) -> bool {
    let expected = expected
        .trim()
        .trim_start_matches("sha256:")
        .trim_start_matches("SHA256:");
// ...
}
```

## 3. Konkluzje i wnioski Architektoniczne

- GitHub udostępnia skróty `sha256` w natywnym zapytaniu release bez konieczności odpytywania osobnego serwisu i odrębnych plików `checksums.txt`.
- Zgodnie z punktem 7 `DESIGN_3_1_UPDATER_SIGNATURES.md`, wartość `digest` pochodząca wprost z API jest "niezaufanym inputem". Choć wygoda jej pobrania jest znaczna, sama z siebie nie dowodzi autentyczności pakietu i służy jedynie za osłonę transportową (ochrona przed uszkodzeniem ramki, przed uciętym pobieraniem).
- Główny mechanizm ufa sumie kontrolnej wyłącznie wtedy, gdy znajdzie się wewnątrz sprawdzonego kluczem Ed25519 `release-manifest.json`.
- Wykonano wymóg etapu 3.1.A - nie jest wymagana modyfikacja kodu.
