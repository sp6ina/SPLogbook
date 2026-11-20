// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

/// Stan interaktywnej mapy świata i warstw nawigacyjnych (wiązka anteny, kompas).
#[derive(Debug, Clone, PartialEq)]
pub struct WorldMapState {
    pub show_window: bool,
    pub show_beam_lobe: bool,
    pub show_compass: bool,
    /// Siatka pól Maidenhead.
    pub show_grid: bool,
    pub beamwidth_deg: f32,
    /// Płótno mapy z ostatniej klatki (cały obszar pod paskiem narzędzi).
    pub last_canvas: Option<eframe::egui::Rect>,
    /// Prostokąt rzutu `2:1` z ostatniej klatki (wewnątrz płótna).
    pub last_map_rect: Option<eframe::egui::Rect>,
}

impl Default for WorldMapState {
    fn default() -> Self {
        Self {
            show_window: false,
            show_beam_lobe: true,
            show_compass: true,
            show_grid: false,
            beamwidth_deg: 50.0,
            last_canvas: None,
            last_map_rect: None,
        }
    }
}

impl WorldMapState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = WorldMapState::default();
        assert!(!state.show_window);
        assert!(state.show_beam_lobe);
        assert!(state.show_compass);
        assert!(!state.show_grid);
        assert_eq!(state.beamwidth_deg, 50.0);

        let new_state = WorldMapState::new();
        assert_eq!(new_state, state);
    }
}
