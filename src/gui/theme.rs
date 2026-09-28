// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// Centralny system motywów/kolorystyki aplikacji. Zamiast rozproszonych po
// całym kodzie literałów `egui::Color32::from_rgb(...)`, panele mogą (stopniowo)
// pobierać kolory z aktywnego `ThemePreset` przez `SpLogApp::palette()`, dzięki
// czemu zmiana motywu w jednym miejscu spójnie zmienia wygląd całej aplikacji.

use eframe::egui;
use egui::Color32;

/// Gotowy zestaw kolorów (paleta) powiązany z motywem.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Czy motyw bazuje na ciemnym tle (wpływa na dobór domyślnych `egui::Visuals`).
    pub is_dark: bool,
    /// Główny kolor akcentu (przyciski, zaznaczenia, aktywne zakładki).
    pub accent: Color32,
    /// Drugorzędny akcent (np. wyróżnienia, linki, wykresy).
    pub accent_alt: Color32,
    /// Kolor sukcesu / potwierdzenia (np. QSL potwierdzone, połączenie aktywne).
    pub success: Color32,
    /// Kolor ostrzeżenia.
    pub warning: Color32,
    /// Kolor błędu / zagrożenia (np. DUPE, rozłączono).
    pub danger: Color32,
    /// Kolor tła głównych paneli.
    pub panel_bg: Color32,
    /// Kolor tła okien/paneli pływających.
    pub window_bg: Color32,
    /// Główny kolor tekstu.
    pub text_primary: Color32,
    /// Przyciemniony/pomocniczy tekst (etykiety, hinty).
    pub text_muted: Color32,
}

/// Gotowe, predefiniowane motywy kolorystyczne aplikacji.
/// Wybór motywu jest zapisywany w konfiguracji stacji (`station_config.json`)
/// i ma pierwszeństwo przed prostym przełącznikiem jasny/ciemny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    /// Domyślny ciemny motyw operatorski — wysoki kontrast, niebiesko-cyjanowy
    /// akcent, zaprojektowany do wielogodzinnej pracy nocnej/kontestowej bez
    /// zmęczenia oczu (podobny do dotychczasowego wyglądu SPLogbook).
    #[default]
    OperatorDark,
    /// Jasny motyw dzienny — czytelny w jasnych pomieszczeniach i przy słońcu,
    /// z niebieskim akcentem o dobrej czytelności na jasnym tle.
    Daylight,
    /// Motyw wysokiego kontrastu, bezpieczny dla osób z daltonizmem
    /// (unika czerwono-zielonych par jako jedynego nośnika informacji;
    /// status/ostrzeżenia korzystają dodatkowo z ikon/tekstu, nie tylko koloru).
    HighContrast,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 3] = [
        ThemePreset::OperatorDark,
        ThemePreset::Daylight,
        ThemePreset::HighContrast,
    ];

    /// Stabilny identyfikator używany do zapisu w konfiguracji (nie zmieniać
    /// wartości bez migracji, aby nie zgubić wyboru użytkownika przy aktualizacji).
    pub fn id(&self) -> &'static str {
        match self {
            ThemePreset::OperatorDark => "operator_dark",
            ThemePreset::Daylight => "daylight",
            ThemePreset::HighContrast => "high_contrast",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "daylight" => ThemePreset::Daylight,
            "high_contrast" => ThemePreset::HighContrast,
            _ => ThemePreset::OperatorDark,
        }
    }

    pub fn label_pl(&self) -> &'static str {
        match self {
            ThemePreset::OperatorDark => "🌙 Operator Dark (domyślny)",
            ThemePreset::Daylight => "☀ Daylight (jasny, dzienny)",
            ThemePreset::HighContrast => "◐ High-Contrast (dostępność, daltonizm)",
        }
    }

    pub fn description_pl(&self) -> &'static str {
        match self {
            ThemePreset::OperatorDark => {
                "Ciemny motyw operatorski do pracy wieczornej/kontestowej — mniejsze zmęczenie oczu."
            }
            ThemePreset::Daylight => {
                "Jasny motyw czytelny w słonecznym pomieszczeniu lub na dworze (polowe operacje)."
            }
            ThemePreset::HighContrast => {
                "Wysoki kontrast i paleta bezpieczna dla daltonistów (bez polegania wyłącznie na czerwieni/zieleni)."
            }
        }
    }

    pub fn is_dark(&self) -> bool {
        match self {
            ThemePreset::OperatorDark | ThemePreset::HighContrast => true,
            ThemePreset::Daylight => false,
        }
    }

    pub fn palette(&self) -> Palette {
        match self {
            ThemePreset::OperatorDark => Palette {
                is_dark: true,
                accent: Color32::from_rgb(56, 189, 248),
                accent_alt: Color32::from_rgb(168, 85, 247),
                success: Color32::from_rgb(34, 197, 94),
                warning: Color32::from_rgb(234, 179, 8),
                danger: Color32::from_rgb(239, 68, 68),
                panel_bg: Color32::from_rgb(15, 23, 42),
                window_bg: Color32::from_rgb(30, 41, 59),
                text_primary: Color32::from_rgb(226, 232, 240),
                text_muted: Color32::from_rgb(148, 163, 184),
            },
            ThemePreset::Daylight => Palette {
                is_dark: false,
                accent: Color32::from_rgb(37, 99, 235),
                accent_alt: Color32::from_rgb(124, 58, 237),
                success: Color32::from_rgb(21, 128, 61),
                warning: Color32::from_rgb(180, 130, 5),
                danger: Color32::from_rgb(185, 28, 28),
                panel_bg: Color32::from_rgb(248, 250, 252),
                window_bg: Color32::from_rgb(255, 255, 255),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_muted: Color32::from_rgb(71, 85, 105),
            },
            ThemePreset::HighContrast => Palette {
                is_dark: true,
                // Niebiesko-bursztynowa para jest rozróżnialna we wszystkich
                // głównych typach ślepoty barw (protanopia/deuteranopia/tritanopia).
                accent: Color32::from_rgb(255, 191, 0),
                accent_alt: Color32::from_rgb(0, 200, 255),
                success: Color32::from_rgb(0, 200, 255),
                warning: Color32::from_rgb(255, 191, 0),
                danger: Color32::from_rgb(255, 255, 255),
                panel_bg: Color32::from_rgb(0, 0, 0),
                window_bg: Color32::from_rgb(18, 18, 18),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_muted: Color32::from_rgb(200, 200, 200),
            },
        }
    }

    /// Buduje i stosuje kompletny `egui::Visuals` na podstawie tego motywu.
    /// Wywoływane raz na klatkę z pętli renderowania aplikacji.
    pub fn apply(&self, ctx: &egui::Context) {
        let p = self.palette();
        let mut visuals = if p.is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.hyperlink_color = p.accent;
        visuals.selection.bg_fill = p.accent;
        visuals.selection.stroke.color = if p.is_dark {
            Color32::BLACK
        } else {
            Color32::WHITE
        };
        visuals.panel_fill = p.panel_bg;
        visuals.window_fill = p.window_bg;
        visuals.extreme_bg_color = if p.is_dark {
            Color32::from_rgb(8, 12, 20)
        } else {
            Color32::from_rgb(233, 236, 241)
        };
        visuals.override_text_color = None; // pozostaw domyślną logikę kontrastu per-widget
        visuals.widgets.hovered.bg_fill = blend(visuals.widgets.hovered.bg_fill, p.accent, 0.12);
        visuals.widgets.active.bg_fill = blend(visuals.widgets.active.bg_fill, p.accent, 0.22);

        ctx.set_visuals(visuals);
    }
}

fn blend(base: Color32, tint: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * t).round() as u8 };
    Color32::from_rgb(
        lerp(base.r(), tint.r()),
        lerp(base.g(), tint.g()),
        lerp(base.b(), tint.b()),
    )
}
