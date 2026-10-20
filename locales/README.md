# Przewodnik Tłumacza SPLogbook (locales/)

Niniejszy katalog zawiera kompletne pliki lokalizacyjne interfejsu użytkownika oraz podręcznika użytkownika aplikacji SPLogbook.

## 1. Obsługiwane języki

Wspieranych jest dokładnie siedem języków:
- `pl.json` — Język polski (język źródłowy/referencyjny)
- `en.json` — Angielski (English)
- `de.json` — Niemiecki (Deutsch)
- `fr.json` — Francuski (Français)
- `es.json` — Hiszpański (Español)
- `it.json` — Włoski (Italiano)
- `ru.json` — Rosyjski (Русский)

Te same 7 języków posiada pełne wersje w podkatalogu `locales/manual/`.

## 2. Format plików katalogu głównego (`locales/*.json`)

- Płaski format JSON: mapa klucz-wartość (`string -> string`).
- Kodowanie: **UTF-8**.
- Sortowanie kluczy: leksykograficzne.
- Dozwolony wzorzec klucza: `^[a-z0-9_]+(\.[a-z0-9_]+)+$`.
- Klucze muszą być unikalne po przekształceniu na stałą w Rust (`replace('.', '_').to_uppercase()`).

## 3. Podręcznik użytkownika (`locales/manual/*.json`)

Katalog `locales/manual/` zawiera zlokalizowane rozdziały wbudowanego podręcznika:
- Każdy plik jest tablicą obiektów o strukturze:
  ```json
  [
    {
      "id": "start",
      "title": "1. Pierwsze kroki",
      "body": "Treść rozdziału..."
    }
  ]
  ```
- Wszystkie 7 plików musi posiadać dokładnie identyczny zestaw identyfikatorów `id` w identycznej kolejności.
- Pola `id`, `title` oraz `body` nie mogą być puste.
- Listy punktowane formatowane są za pomocą `- ` oraz `* `.

## 4. Nazwane parametry i nawiasy klamrowe

- Dozwolone są nazwane placeholdery w formacie `{nazwa}`, gdzie nazwa parametru spełnia wzorzec: `^[a-z][a-z0-9_]*$` (np. `{callsign}`, `{band}`, `{mode}`).
- **Wymóg identycznego zbioru parametrów**: Wartości tego samego klucza we wszystkich 7 językach muszą zawierać dokładnie ten sam zestaw nazwanych parametrów.
- **Zakaz pustych wartości**: Każdy klucz w każdym języku musi mieć niepustą wartość tekstową.
- **Zakaz niezbalansowanych lub dowolnych nawiasów**: Nawiasy klamrowe `{` i `}` mogą występować wyłącznie jako poprawne placeholdery parametrów `{nazwa}`.
- **Brak mechanizmu liczby mnogiej**: Komunikaty należy formułować neutralnie gramatycznie, np. `Znaleziono: {count}`.

## 5. Skróty radiowe i terminologia techniczna

- Powszechnie przyjęte skróty krótkofalarskie i radiowe zachowują swoją standardową postać techniczną we wszystkich językach (np. `QSO`, `DXCC`, `CW`, `SSB`, `FT8`, `RST`, `SWR`, `CAT`, `VFO`, `PTT`, `RIT`, `XIT`, `IOTA`, `SOTA`, `POTA`).
- Tłumaczenia nie mogą być kopiami języka angielskiego poza ściśle ograniczoną i uzasadnioną listą `IDENTICAL_ALLOWED` (maks. 40 pozycji, wyłącznie nazwy własne i skróty techniczne).

## 6. Typowany katalog czasu kompilacji (`build.rs`)

Katalog tłumaczeń nie jest parsowany w czasie działania aplikacji (`runtime`).
Generator `build.rs` podczas kompilacji:
1. Weryfikuje kompletność, format kluczy i spójność parametrów we wszystkich 7 plikach.
2. Weryfikuje strukturę, identyfikatory i kolejność sekcji w `locales/manual/`.
3. Generuje typowane stałe `key::*` typu `Key`, tablice tekstów oraz typed accessor podręcznika `manual(lang: Language) -> &'static [ManualSection]`, gwarantujące dostęp $O(1)$ bez alokacji.

## 7. Typowane komunikaty i błędy domenowe

- Do formatowania komunikatów z parametrami służy bezpieczna struktura `Msg`:
  ```rust
  Msg::new(key::MSG_QSO_SAVED)
      .arg("callsign", "SP6INA")
      .arg("band", "20m")
      .arg("mode", "CW")
      .text(lang)
  ```
- Wszystkie moduły domenowe (core, cloud, cat, sync, digital, api) zwracają silnie typowane enumy błędów implementujące `.msg() -> Msg`.
- **Kontrakt REST API**: Błędy zwracane przez interfejs REST API są lokalizowane w języku polskim (`Language::Pl`) jako stabilna referencja dla integracji lokalnych.

## 8. Weryfikacja strażnicza (Automated Guards)

Zgodność systemu lokalizacji jest bezwarunkowo egzekwowana przez zestaw 15 testów strażniczych w `tests/i18n_guard.rs`:
```bash
cargo test --test i18n_guard -j 2
```
Testy weryfikują:
- Równość zbiorów kluczy między językami (`catalog_languages_have_identical_keys`)
- Identyczność zestawów parametrów (`all_locales_have_identical_parameter_sets`)
- Całkowity brak martwych kluczy w kodzie (`catalog_has_no_dead_keys`)
- Brak nieprzetłumaczonych kopii angielskich poza `IDENTICAL_ALLOWED` (`translations_are_not_english_copies`)
- Ograniczenie i pełne wykorzystanie `IDENTICAL_ALLOWED` (`identical_allowed_is_bounded`, `identical_allowed_entries_are_used`)
- Całkowity brak twardo zakodowanych tekstów w `src/gui/` (`gui_has_no_hardcoded_text`)
- Całkowity brak błędów typu `Result<..., String>` w domenach (`no_string_errors_in_domain`)
- Spójność, kolejność i niepustość podręcznika (`manual_languages_have_identical_section_ids`, `manual_languages_have_identical_order`, `manual_has_no_empty_titles_or_bodies`, `manual_returns_every_language`, `user_manual_renderer_has_no_embedded_translatable_sections`)
- Brak jakichkolwiek zapadek/białych list (`guards_have_no_pending_whitelists`)
- Podstawianie wszystkich parametrów przez komunikaty błędów (`domain_errors_substitute_all_parameters`)
