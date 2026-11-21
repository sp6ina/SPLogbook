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
    /// QSO markers aggregated per locator, rebuilt only when the log revision changes.
    pub markers: Vec<QsoMarker>,
    pub markers_key: Option<(String, u64)>,
}

/// One locator on the map: its centre, whether any QSO there is LoTW-confirmed and the
/// most recent callsign for the tooltip.
#[derive(Debug, Clone, PartialEq)]
pub struct QsoMarker {
    pub latitude: f64,
    pub longitude: f64,
    pub confirmed: bool,
    pub callsign: String,
}

impl WorldMapState {
    /// Rebuilds the marker cache when the journal or its revision changed.
    /// `rows` are in newest-first order, so the first QSO of a locator names the tooltip.
    pub fn refresh_markers(
        &mut self,
        journal_id: &str,
        revision: u64,
        rows: &[crate::core::qso::QsoRecord],
    ) {
        if self
            .markers_key
            .as_ref()
            .is_some_and(|(j, r)| j == journal_id && *r == revision)
        {
            return;
        }
        let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        self.markers.clear();
        for qso in rows {
            let Some(grid) = qso.gridsquare.as_deref() else {
                continue;
            };
            if grid.len() < 4 {
                continue;
            }
            let confirmed = qso.lotw_qsl_rcvd == "Y" || qso.lotw_qsl_rcvd == "V";
            let key = grid.trim().to_ascii_uppercase();
            if let Some(&slot) = index.get(&key) {
                if let Some(marker) = self.markers.get_mut(slot) {
                    marker.confirmed |= confirmed;
                }
                continue;
            }
            let Ok(c) = crate::core::geo::locator_to_coordinates(grid) else {
                continue;
            };
            index.insert(key, self.markers.len());
            self.markers.push(QsoMarker {
                latitude: c.latitude,
                longitude: c.longitude,
                confirmed,
                callsign: qso.callsign.clone(),
            });
        }
        self.markers_key = Some((journal_id.to_string(), revision));
    }
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
            markers: Vec::new(),
            markers_key: None,
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
