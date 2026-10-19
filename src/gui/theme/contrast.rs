// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use eframe::egui::Color32;

/// Oblicza względną luminancję (relative luminance) składowej sRGB zgodnie z WCAG 2.1.
pub fn relative_luminance(color: Color32) -> f32 {
    let srgb_to_linear = |c_u8: u8| -> f32 {
        let c = c_u8 as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let r = srgb_to_linear(color.r());
    let g = srgb_to_linear(color.g());
    let b = srgb_to_linear(color.b());

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Oblicza współczynnik kontrastu (contrast ratio) między dwoma kolorami zgodnie z WCAG 2.1.
/// Wynik mieści się w przedziale [1.0, 21.0].
pub fn contrast_ratio(c1: Color32, c2: Color32) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (l_bright, l_dark) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (l_bright + 0.05) / (l_dark + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_white_contrast() {
        let black = Color32::from_rgb(0, 0, 0);
        let white = Color32::from_rgb(255, 255, 255);

        assert!((relative_luminance(black) - 0.0).abs() < 1e-4);
        assert!((relative_luminance(white) - 1.0).abs() < 1e-4);

        let ratio = contrast_ratio(black, white);
        assert!((ratio - 21.0).abs() < 0.1);

        let self_ratio = contrast_ratio(white, white);
        assert!((self_ratio - 1.0).abs() < 1e-4);
    }
}
