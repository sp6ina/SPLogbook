// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::collections::BTreeMap;

use crate::core::station::{AppConfig, ViewPanelConfig};

/// Kafelek pulpitu. Panele (VFO … wodospad) odpinają się do własnego okna systemowego;
/// moduły (kontest, WSJT-X, makra CW, statystyki, dyplomy, ewidencja sprzętu) odpięte
/// są oknem wewnątrz okna głównego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tile {
    Vfo,
    Qso,
    Log,
    Cluster,
    Bandmap,
    Solar,
    Satellites,
    WorldMap,
    Waterfall,
    Contest,
    Wsjtx,
    CwMacros,
    Statistics,
    Awards,
    Ledger,
}

impl Tile {
    pub const ALL: [Self; 15] = [
        Self::Vfo,
        Self::Qso,
        Self::Log,
        Self::Cluster,
        Self::Bandmap,
        Self::Solar,
        Self::Satellites,
        Self::WorldMap,
        Self::Waterfall,
        Self::Contest,
        Self::Wsjtx,
        Self::CwMacros,
        Self::Statistics,
        Self::Awards,
        Self::Ledger,
    ];

    /// Identyfikator karty w układzie dokowania i klucz w konfiguracji.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Vfo => "vfo",
            Self::Qso => "qso",
            Self::Log => "log",
            Self::Cluster => "cluster",
            Self::Bandmap => "bandmap",
            Self::Solar => "solar",
            Self::Satellites => "satellites",
            Self::WorldMap => "world_map",
            Self::Waterfall => "waterfall",
            Self::Contest => "contest",
            Self::Wsjtx => "wsjtx",
            Self::CwMacros => "cw_macros",
            Self::Statistics => "statistics",
            Self::Awards => "awards",
            Self::Ledger => "ledger",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tile| tile.id() == id)
    }

    pub const fn is_module(self) -> bool {
        matches!(
            self,
            Self::Contest
                | Self::Wsjtx
                | Self::CwMacros
                | Self::Statistics
                | Self::Awards
                | Self::Ledger
        )
    }
}

/// Stan widoków: 9 paneli i okna modułów.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewsState {
    pub panel_vfo: ViewPanelConfig,
    pub panel_qso: ViewPanelConfig,
    pub panel_log: ViewPanelConfig,
    pub panel_cluster: ViewPanelConfig,
    pub panel_solar: ViewPanelConfig,
    pub panel_bandmap: ViewPanelConfig,
    pub panel_satellites: ViewPanelConfig,
    pub panel_world_map: ViewPanelConfig,
    pub panel_waterfall: ViewPanelConfig,
    /// Moduły z ustawieniem innym niż domyślne ([`ViewPanelConfig::MODULE_DEFAULT`]).
    modules: BTreeMap<Tile, ViewPanelConfig>,
}

impl ViewsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_config(config: &AppConfig) -> Self {
        let mut views = Self {
            panel_vfo: config.panel_vfo.clone(),
            panel_qso: config.panel_qso.clone(),
            panel_log: config.panel_log.clone(),
            panel_cluster: config.panel_cluster.clone(),
            panel_solar: config.panel_solar.clone(),
            panel_bandmap: config.panel_bandmap.clone(),
            panel_satellites: config.panel_satellites.clone(),
            panel_world_map: config.panel_world_map.clone(),
            panel_waterfall: config.panel_waterfall.clone(),
            modules: BTreeMap::new(),
        };
        views.set_module_panels(&config.module_panels);
        views
    }

    pub fn panel(&self, tile: Tile) -> &ViewPanelConfig {
        match tile {
            Tile::Vfo => &self.panel_vfo,
            Tile::Qso => &self.panel_qso,
            Tile::Log => &self.panel_log,
            Tile::Cluster => &self.panel_cluster,
            Tile::Bandmap => &self.panel_bandmap,
            Tile::Solar => &self.panel_solar,
            Tile::Satellites => &self.panel_satellites,
            Tile::WorldMap => &self.panel_world_map,
            Tile::Waterfall => &self.panel_waterfall,
            module => self
                .modules
                .get(&module)
                .unwrap_or(&ViewPanelConfig::MODULE_DEFAULT),
        }
    }

    pub fn panel_mut(&mut self, tile: Tile) -> &mut ViewPanelConfig {
        match tile {
            Tile::Vfo => &mut self.panel_vfo,
            Tile::Qso => &mut self.panel_qso,
            Tile::Log => &mut self.panel_log,
            Tile::Cluster => &mut self.panel_cluster,
            Tile::Bandmap => &mut self.panel_bandmap,
            Tile::Solar => &mut self.panel_solar,
            Tile::Satellites => &mut self.panel_satellites,
            Tile::WorldMap => &mut self.panel_world_map,
            Tile::Waterfall => &mut self.panel_waterfall,
            module => self
                .modules
                .entry(module)
                .or_insert(ViewPanelConfig::MODULE_DEFAULT),
        }
    }

    /// Ustawienia modułów do zapisu w konfiguracji lub profilu, według identyfikatora.
    pub fn module_panels(&self) -> BTreeMap<String, ViewPanelConfig> {
        self.modules
            .iter()
            .filter(|(_, cfg)| **cfg != ViewPanelConfig::MODULE_DEFAULT)
            .map(|(tile, cfg)| (tile.id().to_string(), cfg.clone()))
            .collect()
    }

    /// Wczytuje ustawienia modułów; nieznane identyfikatory i panele są pomijane.
    pub fn set_module_panels(&mut self, panels: &BTreeMap<String, ViewPanelConfig>) {
        self.modules = panels
            .iter()
            .filter_map(|(id, cfg)| {
                Tile::from_id(id)
                    .filter(|tile| tile.is_module())
                    .map(|tile| (tile, cfg.clone()))
            })
            .collect();
    }

    /// Liczba kafelków zajmujących pulpit lub własne okno systemowe; moduły liczą się
    /// tylko zadokowane.
    pub fn visible_count(&self) -> usize {
        Tile::ALL
            .into_iter()
            .filter(|&tile| {
                let cfg = self.panel(tile);
                cfg.visible && (!tile.is_module() || !cfg.floating)
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = ViewsState::default();
        let default_panel = ViewPanelConfig::default();
        assert_eq!(state.panel_vfo, default_panel);
        assert_eq!(state.panel_qso, default_panel);
        assert_eq!(state.panel_log, default_panel);
        assert_eq!(state.panel_cluster, default_panel);
        assert_eq!(state.panel_solar, default_panel);
        assert_eq!(state.panel_bandmap, default_panel);
        assert_eq!(state.panel_satellites, default_panel);
        assert_eq!(state.panel_world_map, default_panel);
        assert_eq!(state.panel_waterfall, default_panel);
        for tile in Tile::ALL.into_iter().filter(|t| t.is_module()) {
            assert_eq!(state.panel(tile), &ViewPanelConfig::MODULE_DEFAULT);
        }

        let new_state = ViewsState::new();
        assert_eq!(new_state, state);
        assert_eq!(state.visible_count(), 9); // default panels have visible: true
    }

    #[test]
    fn tile_ids_are_unique_and_round_trip() {
        for tile in Tile::ALL {
            assert_eq!(Tile::from_id(tile.id()), Some(tile));
        }
        let mut ids: Vec<&str> = Tile::ALL.iter().map(|t| t.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Tile::ALL.len());
        assert_eq!(Tile::from_id("unknown"), None);
        assert_eq!(Tile::ALL.iter().filter(|t| t.is_module()).count(), 6);
    }

    #[test]
    fn module_settings_round_trip_and_ignore_unknown_ids() {
        let mut views = ViewsState::new();
        views.panel_mut(Tile::Contest).visible = true;
        views.panel_mut(Tile::Contest).floating = false;
        // Ustawienie równe domyślnemu nie trafia do zapisu.
        let _ = views.panel_mut(Tile::Ledger);
        let saved = views.module_panels();
        assert_eq!(saved.keys().collect::<Vec<_>>(), ["contest"]);

        let mut with_noise = saved.clone();
        with_noise.insert("vfo".to_string(), ViewPanelConfig::default());
        with_noise.insert("old_module".to_string(), ViewPanelConfig::default());
        let mut restored = ViewsState::new();
        restored.set_module_panels(&with_noise);
        assert_eq!(restored.module_panels(), saved);
        assert!(restored.panel(Tile::Contest).visible);
        assert_eq!(restored.panel_vfo, ViewPanelConfig::default());
        // Zadokowany moduł zajmuje pulpit, pływający — nie.
        assert_eq!(restored.visible_count(), 10);
        restored.panel_mut(Tile::Contest).floating = true;
        assert_eq!(restored.visible_count(), 9);
    }
}
