# Architektura Motywów i Palety Semantycznej w SPLogbook (Wave W6 / ARCH-THEMES)

## 1. Wstęp i Cel Architektury

W ramach fali refaktoryzacyjnej W6 (ARCH-THEMES) cały podsystem wizualny aplikacji SPLogbook został zmigrowany z bezpośrednich literałów kolorystycznych (`Color32::from_rgb`, `Color32::from_rgba`, stałe nazwane np. `Color32::WHITE`) na scentralizowaną architekturę palety semantycznej.

Główne cele architektury:
- **Spójność wizualna**: Pełne wsparcie dla przełączania motywów w czasie rzeczywistym (np. Dark, Light, Cyberpunk, Solarized, High-Contrast).
- **Enkapsulacja kolorów**: Zero bezpośrednich konstruktorów `Color32` w plikach widoków `src/gui/**`.
- **Precyzja domenowa**: Dedykowane pod-palety dla mapy (`MapColors`), pasm radiowych (`BandColors`) oraz segmentów emisji (`SegmentColors`).
- **Automatyczna weryfikacja w CI**: Test regresyjny `tests/theme_guard.rs`, który blokuje pojawienie się jakichkolwiek bezpośrednich literałów barwnych poza dozwolonym modułem motywu oraz wyizolowanym modułem poligraficznym `qsl_colors`.

---

## 2. Hierarchia Modułów Motywu (`src/gui/theme/*`)

System motywów zlokalizowany jest w `src/gui/theme/`:

```
src/gui/theme/
├── mod.rs          # Główny interfejs modułu, re-eksporty, aplikacja do egui::Context
├── palette.rs      # Definicje struktur Palette, BandColors, SegmentColors, MapColors, waterfall_color()
├── presets.rs      # Fabryki gotowych palet (dark, light, cyberpunk, solarized itp.)
└── ext.rs          # Traity ThemeExt (dla Context i Ui) oraz ColorExt (.alpha())
```

### 2.1. `ThemeExt`
Umożliwia ergonomiczny dostęp do aktywnej palety z poziomu dowolnego kontekstu `egui`:
```rust
use crate::gui::theme::ThemeExt;

// Pobranie referencji do palety z Ui:
let palette = ui.palette();

// Lub z Context:
let palette = ctx.palette();
```

### 2.2. `ColorExt`
Pozwala na bezpieczne i eleganckie tworzenie przezroczystości bazując na istniejących tokenach:
```rust
use crate::gui::theme::ColorExt;

let semi_transparent_border = palette.border.alpha(128);
let shadow = palette.map.night_overlay.alpha(90);
```

---

## 3. Paleta Semantyczna vs Kolory Domenowe

### 3.1. Tokeny UI (Semantyczne)
Wszystkie elementy interfejsu (nagłówki, etykiety, ramki, tła, przyciski, komunikaty błędów, ostrzeżenia, sukcesy) korzystają z tokenów abstrakcyjnych:
- `accent`, `accent_alt` — wiodące akcenty interaktywne.
- `success`, `warning`, `danger`, `info` — statusy operacyjne i komunikaty.
- `panel_bg`, `window_bg`, `surface_bg`, `subtle_bg`, `extreme_bg` — warstwy tła.
- `border`, `border_subtle` — obrysy i separatory.
- `text_primary`, `text_muted`, `text_subtle` — hierarchia typograficzna.
- `selection_bg`, `selection_text` — zaznaczenia.
- `tx`, `rx`, `split`, `dupe`, `multiplier`, `new_call`, `vfo_a`, `vfo_b` — operacje radiowe.

### 3.2. Pod-palety domenowe
- **`BandColors`** (`palette.bands` / `palette.band_color(band)`): Dedykowane kolory dla pasm (160m..23cm).
- **`MapColors`** (`palette.map`): Kolory widoku mapy świata (oceany, lądy, siatka współrzędnych, terminator zmierzchu, słońce, księżyc, wiązka rotatora antenowego, róża kompasu).
- **`SegmentColors`** (`palette.segments`): Zakresy emisji w bandmapie (CW, SSB, Data, FM, Beacony).

### 3.3. Wyjątek: Poligraficzny Edytor QSL (`qsl_designer.rs`)
W odróżnieniu od interfejsu aplikacji, edytor druku kart i naklejek QSL projektuje **fizyczny wydruk** o stałych parametrach barwnych (CMYK/RGB).
Dlatego:
- Chrom okna i narzędzia (suwaki, marginesy, przyciski, komunikaty) korzystają wyłącznie z `ui.palette()`.
- Kolory podglądu fizycznej naklejki są wyizolowane w lokalnym module `pub mod qsl_colors`:
```rust
pub mod qsl_colors {
    use eframe::egui::Color32;

    pub const CALLSIGN_HIGHLIGHT: Color32 = Color32::from_rgb(250, 204, 21);
    pub const CARD_BORDER: Color32 = Color32::from_rgb(100, 116, 139);
    pub const CARD_BG: Color32 = Color32::WHITE;
    pub const CARD_TEXT: Color32 = Color32::BLACK;
}
```

---

## 4. Jak Dodać Nowy Motyw lub Token

### 4.1. Dodanie nowego motywu
1. Otwórz `src/gui/theme/presets.rs`.
2. Zdefiniuj funkcję fabrykującą paletę, np. `pub fn high_contrast() -> Palette`.
3. Zarejestruj identyfikator motywu w konfiguracji stacji oraz w menu wyboru motywów (`src/gui/theme/mod.rs`).

### 4.2. Dodanie nowego tokenu semantycznego
1. Dodaj pole do struktury `Palette` w `src/gui/theme/palette.rs`.
2. Uzupełnij definicję pola we wszystkich presetach w `src/gui/theme/presets.rs`.
3. Użyj nowego tokenu w kodzie widoków za pośrednictwem `ui.palette().nowy_token`.

---

## 5. Działanie Theme Guard w CI (`tests/theme_guard.rs`)

Plik `tests/theme_guard.rs` implementuje statyczny audyt kodu w ramach zestawu testów jednostkowych i integracyjnych:
1. Przeszukuje rekuzywnie katalog `src/gui/` pod kątem plików `.rs`.
2. Wyklucza dozwolony katalog `src/gui/theme/**`.
3. Weryfikuje zakaz używania konstruktorów:
   - `Color32::from_rgb`
   - `Color32::from_rgba`
   - `Color32::from_gray`
   - `Color32::from_black_alpha`
   - `Color32::from_white_alpha`
4. Weryfikuje zakaz nazwanych stałych (`Color32::WHITE`, `Color32::BLACK`, `Color32::RED` itp.).
5. Weryfikuje zakaz lokalnych stałych `const LIGHT_*` i `const DARK_*`.
6. Dopuszcza strukturalny moduł `mod qsl_colors` wewnątrz `qsl_designer.rs`.
7. Bezwarunkowo skanuje wszystkie pliki `src/gui/` bez listy pominięć — każda próba wprowadzenia surowego koloru poza modułem motywu zakończy się błędem kompilacji i asercją testu `test_theme_guard_no_forbidden_colors`.
