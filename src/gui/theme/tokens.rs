// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Tokeny wizualne UI: skala odstępów, zaokrągleń i typografii oraz helpery
//! tekstu monospace dla znaków wywoławczych, częstotliwości i komórek tabel.
//!
//! Kolory pochodzą wyłącznie z `Palette`; ten moduł definiuje geometrię i rozmiary.
//! Czcionka monospace egui (Hack) rozróżnia `0` (przekreślone/kropkowane) od `O`.

use eframe::egui::{FontFamily, FontId, RichText};

/// Odstępy (px logiczne).
pub const SPACE_XS: f32 = 2.0;
pub const SPACE_SM: f32 = 4.0;
pub const SPACE_MD: f32 = 8.0;
pub const SPACE_LG: f32 = 12.0;
pub const SPACE_XL: f32 = 16.0;

/// Zaokrąglenia narożników (px logiczne).
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;

/// Skala typografii czytelna z odległości stanowiska (bez rozmiarów poniżej 11 px).
pub const TEXT_CAPTION: f32 = 11.0;
pub const TEXT_BODY: f32 = 13.0;
pub const TEXT_SUBHEADING: f32 = 15.0;
pub const TEXT_CALLSIGN: f32 = 18.0;
pub const TEXT_FREQ: f32 = 28.0;

/// Czcionka monospace o zadanym rozmiarze.
#[inline]
pub fn mono_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Czcionka pola znaku wywoławczego (monospace, `TEXT_CALLSIGN`).
#[inline]
pub fn callsign_font() -> FontId {
    mono_font(TEXT_CALLSIGN)
}

/// Znak wywoławczy: monospace, `TEXT_CALLSIGN`.
pub fn mono_callsign(text: impl Into<String>) -> RichText {
    RichText::new(text).font(callsign_font())
}

/// Komórka tabeli (znak, czas, dane liczbowe): monospace, `TEXT_BODY`.
pub fn mono_table_cell(text: impl Into<String>) -> RichText {
    RichText::new(text).font(mono_font(TEXT_BODY))
}

/// Odczyt częstotliwości: monospace, `TEXT_FREQ`.
pub fn mono_freq(text: impl Into<String>) -> RichText {
    RichText::new(text).font(mono_font(TEXT_FREQ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_scale_is_strictly_increasing() {
        let scale = [SPACE_XS, SPACE_SM, SPACE_MD, SPACE_LG, SPACE_XL];
        assert!(scale.windows(2).all(|w| w[0] < w[1]));
        const { assert!(RADIUS_SM < RADIUS_MD) };
    }

    #[test]
    fn type_scale_is_increasing_and_has_no_micro_sizes() {
        let scale = [
            TEXT_CAPTION,
            TEXT_BODY,
            TEXT_SUBHEADING,
            TEXT_CALLSIGN,
            TEXT_FREQ,
        ];
        assert!(scale.windows(2).all(|w| w[0] < w[1]));
        const { assert!(TEXT_CAPTION >= 11.0, "mikro-rozmiary 8-9 px są zabronione") };
    }

    #[test]
    fn mono_fonts_use_monospace_family_and_token_sizes() {
        for (font, size) in [
            (callsign_font(), TEXT_CALLSIGN),
            (mono_font(TEXT_BODY), TEXT_BODY),
            (mono_font(TEXT_FREQ), TEXT_FREQ),
        ] {
            assert_eq!(font.family, FontFamily::Monospace);
            assert!((font.size - size).abs() < f32::EPSILON);
        }
        // Helpery tworzą tekst bez paniki.
        let _ = (
            mono_callsign("SP6INA"),
            mono_table_cell("14.074"),
            mono_freq("14.074"),
        );
    }
}
