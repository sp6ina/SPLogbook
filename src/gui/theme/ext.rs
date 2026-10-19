// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use eframe::egui;
use egui::Color32;

use super::palette::{Palette, ThemePreset};
use super::palette_id;

/// Rozszerzenie dla `egui::Context` i `egui::Ui` dające bezpanicowy dostęp do aktywnej palety.
pub trait ThemeExt {
    fn palette(&self) -> Palette;
}

impl ThemeExt for egui::Context {
    fn palette(&self) -> Palette {
        self.data(|d| d.get_temp::<Palette>(palette_id()))
            .unwrap_or_else(|| ThemePreset::OperatorDark.palette())
    }
}

impl ThemeExt for egui::Ui {
    fn palette(&self) -> Palette {
        self.ctx().palette()
    }
}

/// Rozszerzenie dla `Color32` pozwalające na tworzenie wariantu z zadanym kanałem alpha.
pub trait ColorExt {
    fn alpha(self, a: u8) -> Color32;
}

impl ColorExt for Color32 {
    fn alpha(self, a: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(self.r(), self.g(), self.b(), a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_ext_alpha() {
        let base = Color32::from_rgb(10, 20, 30);
        let with_alpha = base.alpha(128);
        assert_eq!(with_alpha, Color32::from_rgba_unmultiplied(10, 20, 30, 128));
    }
}
