// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use eframe::egui::Color32;

/// Zestaw kolorów przypisanych do pasm amatorskich.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandColors {
    pub b160m: Color32,
    pub b80m: Color32,
    pub b60m: Color32,
    pub b40m: Color32,
    pub b30m: Color32,
    pub b20m: Color32,
    pub b17m: Color32,
    pub b15m: Color32,
    pub b12m: Color32,
    pub b10m: Color32,
    pub b6m: Color32,
    pub b4m: Color32,
    pub b2m: Color32,
    pub b70cm: Color32,
    pub b23cm: Color32,
    pub other: Color32,
}

/// Kolory segmentów pasma (np. w Bandmapie).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentColors {
    pub cw: Color32,
    pub cw_text: Color32,
    pub phone: Color32,
    pub phone_text: Color32,
    pub data: Color32,
    pub data_text: Color32,
    pub fm: Color32,
    pub fm_text: Color32,
    pub beacon: Color32,
    pub beacon_text: Color32,
}

/// Kolory widoku mapy świata (World Map).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapColors {
    pub ocean: Color32,
    pub land: Color32,
    pub graticule: Color32,
    pub axis: Color32,
    pub ocean_text: Color32,
    pub night_overlay: Color32,
    pub terminator: Color32,
    pub sun: Color32,
    pub sun_halo: Color32,
    pub moon: Color32,
    pub beam_fill: Color32,
    pub beam_stroke: Color32,
    pub compass_ring: Color32,
    pub compass_text: Color32,
    pub qso_marker: Color32,
    pub qso_marker_confirmed: Color32,
}

/// Gotowy zestaw kolorów (paleta) powiązany z motywem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Czy motyw bazuje na ciemnym tle (wpływa na dobór domyślnych `egui::Visuals`).
    pub is_dark: bool,

    // Główne tokeny interfejsu
    pub accent: Color32,
    pub accent_alt: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub info: Color32,

    pub panel_bg: Color32,
    pub window_bg: Color32,
    pub surface_bg: Color32,
    pub subtle_bg: Color32,
    pub extreme_bg: Color32,

    pub border: Color32,
    pub border_subtle: Color32,

    pub text_primary: Color32,
    pub text_muted: Color32,
    pub text_subtle: Color32,

    pub selection_bg: Color32,
    pub selection_text: Color32,

    // Tokeny operacyjne i radiowe
    pub tx: Color32,
    pub rx: Color32,
    pub split: Color32,
    pub dupe: Color32,
    pub multiplier: Color32,
    pub new_call: Color32,
    pub vfo_a: Color32,
    pub vfo_b: Color32,

    // Statusy QSL i Sync
    pub qsl_rcvd: Color32,
    pub qsl_sent: Color32,
    pub qsl_queued: Color32,
    pub sync_synced: Color32,
    pub sync_syncing: Color32,
    pub sync_error: Color32,

    // Podgrupy
    pub bands: BandColors,
    pub segments: SegmentColors,
    pub map: MapColors,
}

impl Palette {
    /// Zwraca kolor przypisany do pasma radiowego.
    pub fn band_color(&self, band: &str) -> Color32 {
        let b = band.trim().to_ascii_lowercase();
        match b.as_str() {
            "160m" => self.bands.b160m,
            "80m" => self.bands.b80m,
            "60m" => self.bands.b60m,
            "40m" => self.bands.b40m,
            "30m" => self.bands.b30m,
            "20m" => self.bands.b20m,
            "17m" => self.bands.b17m,
            "15m" => self.bands.b15m,
            "12m" => self.bands.b12m,
            "10m" => self.bands.b10m,
            "6m" => self.bands.b6m,
            "4m" => self.bands.b4m,
            "2m" => self.bands.b2m,
            "70cm" => self.bands.b70cm,
            "23cm" => self.bands.b23cm,
            _ => self.bands.other,
        }
    }

    /// Zwraca kolor serii danych dla wykresów i statystyk o zadanym indeksie.
    pub fn series(&self, idx: usize) -> Color32 {
        let series_colors = if self.is_dark {
            [
                self.accent,
                self.accent_alt,
                self.success,
                self.warning,
                Color32::from_rgb(244, 114, 182), // pink
                Color32::from_rgb(45, 212, 191),  // teal
                Color32::from_rgb(251, 146, 60),  // orange
                Color32::from_rgb(167, 139, 250), // indigo
            ]
        } else {
            [
                self.accent,
                self.accent_alt,
                self.success,
                self.warning,
                Color32::from_rgb(190, 24, 93),  // dark pink
                Color32::from_rgb(15, 118, 110), // dark teal
                Color32::from_rgb(194, 65, 12),  // dark orange
                Color32::from_rgb(109, 40, 217), // dark indigo
            ]
        };
        series_colors[idx % series_colors.len()]
    }
}

/// Gotowe, predefiniowane motywy kolorystyczne aplikacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    /// Domyślny ciemny motyw operatorski — wysoki kontrast, niebiesko-cyjanowy
    /// akcent, zaprojektowany do wielogodzinnej pracy nocnej/kontestowej bez
    /// zmęczenia oczu.
    #[default]
    OperatorDark,
    /// Jasny motyw dzienny — czytelny w jasnych pomieszczeniach i przy słońcu.
    Daylight,
    /// Motyw wysokiego kontrastu, bezpieczny dla osób z daltonizmem.
    HighContrast,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 3] = [
        ThemePreset::OperatorDark,
        ThemePreset::Daylight,
        ThemePreset::HighContrast,
    ];

    /// Stabilny identyfikator używany do zapisu w konfiguracji stacji.
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

    pub fn label(&self, lang: crate::core::i18n::Language) -> &'static str {
        match self {
            ThemePreset::OperatorDark => {
                crate::core::i18n::tr(crate::core::i18n::key::THEME_PRESET_DARK_LABEL, lang)
            }
            ThemePreset::Daylight => {
                crate::core::i18n::tr(crate::core::i18n::key::THEME_PRESET_LIGHT_LABEL, lang)
            }
            ThemePreset::HighContrast => {
                crate::core::i18n::tr(crate::core::i18n::key::THEME_PRESET_CONTRAST_LABEL, lang)
            }
        }
    }

    pub fn description(&self, lang: crate::core::i18n::Language) -> &'static str {
        match self {
            ThemePreset::OperatorDark => {
                crate::core::i18n::tr(crate::core::i18n::key::THEME_PRESET_DARK_DESC, lang)
            }
            ThemePreset::Daylight => {
                crate::core::i18n::tr(crate::core::i18n::key::THEME_PRESET_LIGHT_DESC, lang)
            }
            ThemePreset::HighContrast => {
                crate::core::i18n::tr(crate::core::i18n::key::THEME_PRESET_CONTRAST_DESC, lang)
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
                info: Color32::from_rgb(56, 189, 248),
                panel_bg: Color32::from_rgb(15, 23, 42),
                window_bg: Color32::from_rgb(30, 41, 59),
                surface_bg: Color32::from_rgb(20, 29, 47),
                subtle_bg: Color32::from_rgb(24, 34, 53),
                extreme_bg: Color32::from_rgb(8, 12, 20),
                border: Color32::from_rgb(51, 65, 85),
                border_subtle: Color32::from_rgb(30, 41, 59),
                text_primary: Color32::from_rgb(226, 232, 240),
                text_muted: Color32::from_rgb(148, 163, 184),
                text_subtle: Color32::from_rgb(100, 116, 139),
                selection_bg: Color32::from_rgb(56, 189, 248),
                selection_text: Color32::BLACK,
                tx: Color32::from_rgb(239, 68, 68),
                rx: Color32::from_rgb(34, 197, 94),
                split: Color32::from_rgb(234, 179, 8),
                dupe: Color32::from_rgb(239, 68, 68),
                multiplier: Color32::from_rgb(234, 179, 8),
                new_call: Color32::from_rgb(56, 189, 248),
                vfo_a: Color32::from_rgb(56, 189, 248),
                vfo_b: Color32::from_rgb(168, 85, 247),
                qsl_rcvd: Color32::from_rgb(34, 197, 94),
                qsl_sent: Color32::from_rgb(56, 189, 248),
                qsl_queued: Color32::from_rgb(234, 179, 8),
                sync_synced: Color32::from_rgb(34, 197, 94),
                sync_syncing: Color32::from_rgb(234, 179, 8),
                sync_error: Color32::from_rgb(239, 68, 68),
                bands: BandColors {
                    b160m: Color32::from_rgb(192, 132, 252),
                    b80m: Color32::from_rgb(168, 85, 247),
                    b60m: Color32::from_rgb(129, 140, 248),
                    b40m: Color32::from_rgb(96, 165, 250),
                    b30m: Color32::from_rgb(56, 189, 248),
                    b20m: Color32::from_rgb(45, 212, 191),
                    b17m: Color32::from_rgb(52, 211, 153),
                    b15m: Color32::from_rgb(74, 222, 128),
                    b12m: Color32::from_rgb(163, 230, 53),
                    b10m: Color32::from_rgb(250, 204, 21),
                    b6m: Color32::from_rgb(251, 146, 60),
                    b4m: Color32::from_rgb(248, 113, 113),
                    b2m: Color32::from_rgb(244, 114, 182),
                    b70cm: Color32::from_rgb(232, 121, 249),
                    b23cm: Color32::from_rgb(217, 70, 239),
                    other: Color32::from_rgb(148, 163, 184),
                },
                segments: SegmentColors {
                    cw: Color32::from_rgb(14, 116, 144),
                    cw_text: Color32::WHITE,
                    phone: Color32::from_rgb(15, 118, 110),
                    phone_text: Color32::WHITE,
                    data: Color32::from_rgb(109, 40, 217),
                    data_text: Color32::WHITE,
                    fm: Color32::from_rgb(180, 83, 9),
                    fm_text: Color32::WHITE,
                    beacon: Color32::from_rgb(161, 98, 7),
                    beacon_text: Color32::WHITE,
                },
                map: MapColors {
                    ocean: Color32::from_rgb(15, 23, 42),
                    land: Color32::from_rgb(33, 44, 66),
                    graticule: Color32::from_rgba_unmultiplied(255, 255, 255, 18),
                    axis: Color32::from_rgba_unmultiplied(56, 189, 248, 45),
                    ocean_text: Color32::from_rgba_unmultiplied(148, 163, 184, 25),
                    night_overlay: Color32::from_rgba_unmultiplied(3, 7, 18, 120),
                    terminator: Color32::from_rgba_unmultiplied(251, 191, 36, 170),
                    sun: Color32::from_rgb(250, 204, 21),
                    sun_halo: Color32::from_rgba_unmultiplied(245, 158, 11, 60),
                    moon: Color32::from_rgb(226, 232, 240),
                    beam_fill: Color32::from_rgba_unmultiplied(56, 189, 248, 55),
                    beam_stroke: Color32::from_rgba_unmultiplied(56, 189, 248, 140),
                    compass_ring: Color32::from_rgba_unmultiplied(56, 189, 248, 100),
                    compass_text: Color32::from_rgb(56, 189, 248),
                    qso_marker: Color32::from_rgb(244, 63, 94),
                    qso_marker_confirmed: Color32::from_rgb(34, 197, 94),
                },
            },
            ThemePreset::Daylight => Palette {
                is_dark: false,
                accent: Color32::from_rgb(37, 99, 235),
                accent_alt: Color32::from_rgb(124, 58, 237),
                success: Color32::from_rgb(21, 128, 61),
                warning: Color32::from_rgb(180, 130, 5),
                danger: Color32::from_rgb(185, 28, 28),
                info: Color32::from_rgb(2, 132, 199),
                panel_bg: Color32::from_rgb(248, 250, 252),
                window_bg: Color32::from_rgb(255, 255, 255),
                surface_bg: Color32::from_rgb(241, 245, 249),
                subtle_bg: Color32::from_rgb(226, 232, 240),
                extreme_bg: Color32::from_rgb(233, 236, 241),
                border: Color32::from_rgb(203, 213, 225),
                border_subtle: Color32::from_rgb(226, 232, 240),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_muted: Color32::from_rgb(71, 85, 105),
                text_subtle: Color32::from_rgb(100, 116, 139),
                selection_bg: Color32::from_rgb(37, 99, 235),
                selection_text: Color32::WHITE,
                tx: Color32::from_rgb(185, 28, 28),
                rx: Color32::from_rgb(21, 128, 61),
                split: Color32::from_rgb(180, 130, 5),
                dupe: Color32::from_rgb(185, 28, 28),
                multiplier: Color32::from_rgb(180, 130, 5),
                new_call: Color32::from_rgb(37, 99, 235),
                vfo_a: Color32::from_rgb(37, 99, 235),
                vfo_b: Color32::from_rgb(124, 58, 237),
                qsl_rcvd: Color32::from_rgb(21, 128, 61),
                qsl_sent: Color32::from_rgb(37, 99, 235),
                qsl_queued: Color32::from_rgb(180, 130, 5),
                sync_synced: Color32::from_rgb(21, 128, 61),
                sync_syncing: Color32::from_rgb(180, 130, 5),
                sync_error: Color32::from_rgb(185, 28, 28),
                bands: BandColors {
                    b160m: Color32::from_rgb(147, 51, 234),
                    b80m: Color32::from_rgb(126, 34, 206),
                    b60m: Color32::from_rgb(79, 70, 229),
                    b40m: Color32::from_rgb(29, 78, 216),
                    b30m: Color32::from_rgb(2, 132, 199),
                    b20m: Color32::from_rgb(15, 118, 110),
                    b17m: Color32::from_rgb(4, 120, 87),
                    b15m: Color32::from_rgb(21, 128, 61),
                    b12m: Color32::from_rgb(101, 163, 13),
                    b10m: Color32::from_rgb(161, 98, 7),
                    b6m: Color32::from_rgb(194, 65, 12),
                    b4m: Color32::from_rgb(185, 28, 28),
                    b2m: Color32::from_rgb(190, 24, 93),
                    b70cm: Color32::from_rgb(162, 28, 175),
                    b23cm: Color32::from_rgb(134, 25, 143),
                    other: Color32::from_rgb(71, 85, 105),
                },
                segments: SegmentColors {
                    cw: Color32::from_rgb(14, 116, 144),
                    cw_text: Color32::WHITE,
                    phone: Color32::from_rgb(15, 118, 110),
                    phone_text: Color32::WHITE,
                    data: Color32::from_rgb(109, 40, 217),
                    data_text: Color32::WHITE,
                    fm: Color32::from_rgb(180, 83, 9),
                    fm_text: Color32::WHITE,
                    beacon: Color32::from_rgb(161, 98, 7),
                    beacon_text: Color32::WHITE,
                },
                map: MapColors {
                    ocean: Color32::from_rgb(240, 249, 255),
                    land: Color32::from_rgb(203, 213, 225),
                    graticule: Color32::from_rgba_unmultiplied(100, 116, 139, 35),
                    axis: Color32::from_rgba_unmultiplied(37, 99, 235, 60),
                    ocean_text: Color32::from_rgba_unmultiplied(71, 85, 105, 45),
                    night_overlay: Color32::from_rgba_unmultiplied(15, 23, 42, 70),
                    terminator: Color32::from_rgba_unmultiplied(180, 130, 5, 200),
                    sun: Color32::from_rgb(217, 119, 6),
                    sun_halo: Color32::from_rgba_unmultiplied(245, 158, 11, 70),
                    moon: Color32::from_rgb(71, 85, 105),
                    beam_fill: Color32::from_rgba_unmultiplied(37, 99, 235, 45),
                    beam_stroke: Color32::from_rgba_unmultiplied(37, 99, 235, 160),
                    compass_ring: Color32::from_rgba_unmultiplied(37, 99, 235, 120),
                    compass_text: Color32::from_rgb(37, 99, 235),
                    qso_marker: Color32::from_rgb(225, 29, 72),
                    qso_marker_confirmed: Color32::from_rgb(21, 128, 61),
                },
            },
            ThemePreset::HighContrast => Palette {
                is_dark: true,
                accent: Color32::from_rgb(255, 191, 0),
                accent_alt: Color32::from_rgb(0, 200, 255),
                success: Color32::from_rgb(0, 220, 255),
                warning: Color32::from_rgb(255, 191, 0),
                danger: Color32::from_rgb(255, 255, 255),
                info: Color32::from_rgb(0, 200, 255),
                panel_bg: Color32::from_rgb(0, 0, 0),
                window_bg: Color32::from_rgb(18, 18, 18),
                surface_bg: Color32::from_rgb(10, 10, 10),
                subtle_bg: Color32::from_rgb(24, 24, 24),
                extreme_bg: Color32::from_rgb(0, 0, 0),
                border: Color32::from_rgb(255, 255, 255),
                border_subtle: Color32::from_rgb(160, 160, 160),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_muted: Color32::from_rgb(210, 210, 210),
                text_subtle: Color32::from_rgb(180, 180, 180),
                selection_bg: Color32::from_rgb(255, 191, 0),
                selection_text: Color32::BLACK,
                tx: Color32::from_rgb(255, 191, 0),
                rx: Color32::from_rgb(0, 200, 255),
                split: Color32::from_rgb(255, 191, 0),
                dupe: Color32::from_rgb(255, 255, 255),
                multiplier: Color32::from_rgb(255, 191, 0),
                new_call: Color32::from_rgb(0, 200, 255),
                vfo_a: Color32::from_rgb(255, 191, 0),
                vfo_b: Color32::from_rgb(0, 200, 255),
                qsl_rcvd: Color32::from_rgb(0, 200, 255),
                qsl_sent: Color32::from_rgb(255, 191, 0),
                qsl_queued: Color32::from_rgb(255, 255, 255),
                sync_synced: Color32::from_rgb(0, 200, 255),
                sync_syncing: Color32::from_rgb(255, 191, 0),
                sync_error: Color32::from_rgb(255, 255, 255),
                bands: BandColors {
                    b160m: Color32::from_rgb(0, 200, 255),
                    b80m: Color32::from_rgb(0, 220, 255),
                    b60m: Color32::from_rgb(100, 220, 255),
                    b40m: Color32::from_rgb(150, 230, 255),
                    b30m: Color32::from_rgb(200, 240, 255),
                    b20m: Color32::from_rgb(255, 255, 255),
                    b17m: Color32::from_rgb(255, 240, 180),
                    b15m: Color32::from_rgb(255, 230, 140),
                    b12m: Color32::from_rgb(255, 215, 80),
                    b10m: Color32::from_rgb(255, 191, 0),
                    b6m: Color32::from_rgb(255, 170, 0),
                    b4m: Color32::from_rgb(255, 150, 0),
                    b2m: Color32::from_rgb(255, 130, 0),
                    b70cm: Color32::from_rgb(255, 200, 200),
                    b23cm: Color32::from_rgb(255, 220, 220),
                    other: Color32::from_rgb(220, 220, 220),
                },
                segments: SegmentColors {
                    cw: Color32::from_rgb(0, 180, 230),
                    cw_text: Color32::BLACK,
                    phone: Color32::from_rgb(255, 191, 0),
                    phone_text: Color32::BLACK,
                    data: Color32::from_rgb(0, 220, 255),
                    data_text: Color32::BLACK,
                    fm: Color32::from_rgb(255, 220, 80),
                    fm_text: Color32::BLACK,
                    beacon: Color32::from_rgb(255, 255, 255),
                    beacon_text: Color32::BLACK,
                },
                map: MapColors {
                    ocean: Color32::from_rgb(0, 0, 0),
                    land: Color32::from_rgb(40, 40, 40),
                    graticule: Color32::from_rgba_unmultiplied(200, 200, 200, 50),
                    axis: Color32::from_rgba_unmultiplied(255, 191, 0, 120),
                    ocean_text: Color32::from_rgba_unmultiplied(200, 200, 200, 80),
                    night_overlay: Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                    terminator: Color32::from_rgb(255, 191, 0),
                    sun: Color32::from_rgb(255, 191, 0),
                    sun_halo: Color32::from_rgba_unmultiplied(255, 191, 0, 100),
                    moon: Color32::from_rgb(0, 200, 255),
                    beam_fill: Color32::from_rgba_unmultiplied(255, 191, 0, 70),
                    beam_stroke: Color32::from_rgb(255, 191, 0),
                    compass_ring: Color32::from_rgb(255, 191, 0),
                    compass_text: Color32::from_rgb(255, 191, 0),
                    qso_marker: Color32::from_rgb(255, 191, 0),
                    qso_marker_confirmed: Color32::from_rgb(0, 200, 255),
                },
            },
        }
    }
}
