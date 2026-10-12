# Przewodnik Tłumacza SPLogbook (locales/)

Niniejszy katalog zawiera pliki lokalizacyjne interfejsu użytkownika aplikacji SPLogbook.

## 1. Obsługiwane pliki językowe

Wspieranych jest dokładnie siedem języków:
- `pl.json` — Język polski (język źródłowy/referencyjny)
- `en.json` — Angielski (English)
- `de.json` — Niemiecki (Deutsch)
- `fr.json` — Francuski (Français)
- `es.json` — Hiszpański (Español)
- `it.json` — Włoski (Italiano)
- `ru.json` — Rosyjski (Русский)

## 2. Format plików

- Wszystkie pliki mają **płaski format JSON** (obiekt mapujący klucz na wartość: `string -> string`).
- Kodowanie plików: **UTF-8**.
- Sortowanie kluczy: leksykograficzne (alfabetyczne).

## 3. Format i wzorzec klucza

- Format klucza to dwu- lub wieloczłonowa ścieżka domenowa: `obszar.nazwa` (np. `qso.callsign`, `btn.clear`, `msg.qso_saved`).
- Dozwolony wzorzec klucza: `^[a-z0-9_]+(\.[a-z0-9_]+)+$`.
- Klucze muszą być unikalne po przekształceniu na stałą w Rust (`replace('.', '_').to_uppercase()`).

## 4. Nazwane parametry i nawiasy klamrowe

- Dozwolone są nazwane placeholdery w formacie `{nazwa}`, gdzie nazwa parametru spełnia wzorzec: `^[a-z][a-z0-9_]*$` (np. `{callsign}`, `{band}`, `{mode}`).
- **Wymóg identycznego zbioru parametrów**: Wartości tego samego klucza we wszystkich 7 językach muszą zawierać dokładnie ten sam zestaw nazwanych parametrów.
- **Zakaz pustych wartości**: Każdy klucz w każdym języku musi mieć niepustą wartość tekstową.
- **Zakaz niezbalansowanych lub dowolnych nawiasów**: Nawiasy klamrowe `{` i `}` mogą występować wyłącznie jako poprawne placeholdery parametrów `{nazwa}`.
- **Brak mechanizmu liczby mnogiej**: Program nie obsługuje pluralizacji zależnej od języka. Komunikaty należy formułować neutralnie gramatycznie, np. `Znaleziono: {count}`.

## 5. Skróty radiowe i terminologia

- Powszechnie przyjęte skróty krótkofalarskie i radiowe zachowują swoją standardową postać techniczną we wszystkich językach (np. `QSO`, `DXCC`, `CW`, `SSB`, `FT8`, `RST`, `SWR`, `CAT`, `VFO`, `PTT`, `RIT`, `XIT`, `IOTA`, `SOTA`, `POTA`).

## 6. Typowany katalog czasu kompilacji

Katalog tłumaczeń nie jest parsowany w czasie działania aplikacji (`runtime`).
Generator `build.rs` podczas kompilacji weryfikuje kompletność i spójność wszystkich siedmiu plików JSON oraz generuje typowane stałe `key::*` i tablice tekstów zapewniające dostęp $O(1)$ bez alokacji.

## 7. Dodawanie nowego klucza

Aby dodać nowe tłumaczenie:
1. Dodaj nowy klucz `obszar.nazwa` do **wszystkich siedmiu plików JSON** (`pl.json`, `en.json`, `de.json`, `fr.json`, `es.json`, `it.json`, `ru.json`).
2. Podaj naturalne, poprawne tłumaczenie dla każdego z języków (nie kopiuj angielskich tekstów do innych języków, jeżeli istnieje naturalny odpowiednik).
3. Jeżeli komunikat przyjmuje parametry, użyj identycznych nazw `{parametr}` we wszystkich 7 plikach.
4. Skompiluj projekt — `build.rs` automatycznie utworzy stałą `crate::core::i18n::key::OBSZAR_NAZWA`.

## 8. Weryfikacja poprawności

Przed zatwierdzeniem zmian wykonaj:
```bash
cargo +stable check --all-targets --all-features
cargo +stable test --all-targets --all-features
```

## 9. Plany na kolejne fazy

W bieżącej fazie (ARCH-I18N faza A) wdrożono typowany katalog, generator kompilacji oraz mechanizm sparametryzowanych komunikatów `Msg`.
Faza B (ARCH-I18N faza B) obejmie audyt jakościowy istniejących tłumaczeń, usunięcie historycznych martwych kluczy oraz lokalizację podręcznika użytkownika.
