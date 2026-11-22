// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Dock workspace state and panel layout synchronization.

use egui_dock::{DockState, NodeIndex};

use super::views::{Tile, ViewsState};
use crate::core::station::ViewPanelConfig;

/// Session state for the dockable workspace layout and tiles.
pub struct DockWorkspace {
    pub dock_state: DockState<String>,
    pub last_saved_dock_layout: Option<serde_json::Value>,
    pub reset_layout_requested: bool,
    pub dragging_tile: Option<String>,
    /// Otwarte okno zmiany nazwy strony pulpitu.
    pub page_rename: Option<crate::gui::workspace_pages::PageRename>,
    pub active_tab: [usize; 3],
}

impl Default for DockWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl DockWorkspace {
    pub fn new() -> Self {
        Self {
            dock_state: DockState::new(vec![]),
            last_saved_dock_layout: None,
            reset_layout_requested: false,
            dragging_tile: None,
            page_rename: None,
            active_tab: [0; 3],
        }
    }

    /// Canonical order of tile identifiers used when building or synchronizing the dock tree.
    pub fn all_tile_ids() -> [&'static str; 15] {
        Tile::ALL.map(Tile::id)
    }

    pub fn panel_config<'a>(views: &'a ViewsState, id: &str) -> &'a ViewPanelConfig {
        views.panel(Tile::from_id(id).unwrap_or(Tile::Vfo))
    }

    /// Dokuje kartę: moduł dołącza do liścia dziennika (środkowa kolumna), panel do
    /// pierwszego liścia.
    fn add_tab(&mut self, tile: Tile) {
        let anchor = if tile.is_module() {
            self.dock_state.find_tab(&Tile::Log.id().to_string())
        } else {
            None
        };
        if let Some(leaf) = anchor.and_then(|path| self.dock_state.leaf_mut(path.node_path()).ok())
        {
            leaf.append_tab(tile.id().to_string());
        } else {
            self.dock_state.push_to_first_leaf(tile.id().to_string());
        }
    }

    pub fn build(&mut self, dock_layout: Option<&serde_json::Value>, views: &ViewsState) {
        if let Some(json) = dock_layout
            && let Ok(state) = serde_json::from_value::<DockState<String>>(json.clone())
        {
            self.dock_state = state;
            self.sync_with_views(views);
            self.last_saved_dock_layout = self.serialize_layout();
            return;
        }

        self.rebuild_from_views(views);
        self.last_saved_dock_layout = self.serialize_layout();
    }

    /// Udziały szerokości kolumn 0/1/2 (lewa: VFO + QSO, środkowa: Logbook, prawa: Klaster/narzędzia).
    pub const COLUMN_WEIGHTS: [f32; 3] = [0.29, 0.47, 0.24];

    /// Udział wysokości górnego kafelka w kolumnie, w której kafelki są ułożone pionowo
    /// (lewa: VFO nad wyższym formularzem QSO; pozostałe: główny widok nad zakładkami).
    pub const STACK_TOP_FRACTION: [f32; 3] = [0.28, 0.62, 0.62];

    /// Ułamki kolejnych podziałów poziomych dla zajętych kolumn (`present` rosnąco).
    ///
    /// Każdy podział dzieli „resztę” po prawej stronie, więc ułamek k-tego podziału
    /// to waga k-tej kolumny podzielona przez sumę wag kolumn od k-tej w prawo.
    /// Dzięki temu przy 3 kolumnach powstaje ok. 29% / 47% / 24%, a przy 2 kolumnach
    /// proporcje są normalizowane analogicznie (środkowa/Logbook zawsze najszersza).
    pub fn split_fractions(present: &[usize]) -> Vec<f32> {
        (0..present.len().saturating_sub(1))
            .map(|k| {
                let remaining: f32 = present[k..]
                    .iter()
                    .map(|c| Self::COLUMN_WEIGHTS[(*c).min(2)])
                    .sum();
                (Self::COLUMN_WEIGHTS[present[k].min(2)] / remaining).clamp(0.1, 0.9)
            })
            .collect()
    }

    /// Buduje czysty układ domyślny: każda zajęta kolumna dostaje własny liść z pierwszym
    /// kafelkiem, a pozostałe kafelki kolumny trafiają do liścia pod nim (jako zakładki),
    /// więc kluczowe widoki (VFO + formularz QSO, Logbook) są widoczne jednocześnie.
    pub fn rebuild_from_views(&mut self, views: &ViewsState) {
        let mut columns: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for id in Self::all_tile_ids() {
            let cfg = Self::panel_config(views, id);
            if cfg.visible && !cfg.floating {
                columns[cfg.column.clamp(0, 2)].push(id.to_string());
            }
        }
        for col in &mut columns {
            col.sort_by_key(|id| Self::panel_config(views, id).order);
        }

        let present: Vec<usize> = (0..3).filter(|c| !columns[*c].is_empty()).collect();
        let Some(&first) = present.first() else {
            self.dock_state = DockState::new(vec![]);
            return;
        };

        self.dock_state = DockState::new(vec![columns[first][0].clone()]);

        let fractions = Self::split_fractions(&present);
        let mut col_nodes: Vec<(usize, NodeIndex)> = vec![(first, NodeIndex::root())];
        let mut rightmost = NodeIndex::root();
        for (pos, &col) in present.iter().enumerate().skip(1) {
            let [old, new] = self.dock_state.main_surface_mut().split_right(
                rightmost,
                fractions[pos - 1],
                vec![columns[col][0].clone()],
            );
            // Poprzednia skrajna kolumna przeniosła się do `old`.
            if let Some(last) = col_nodes.last_mut() {
                last.1 = old;
            }
            col_nodes.push((col, new));
            rightmost = new;
        }

        for (col, node) in col_nodes {
            if columns[col].len() > 1 {
                self.dock_state.main_surface_mut().split_below(
                    node,
                    Self::STACK_TOP_FRACTION[col.min(2)],
                    columns[col][1..].to_vec(),
                );
            }
        }
    }

    pub fn sync_with_views(&mut self, views: &ViewsState) {
        for id in Self::all_tile_ids() {
            let cfg = Self::panel_config(views, id);
            let should_be_docked = cfg.visible && !cfg.floating;
            let slot = self.dock_state.find_tab(&id.to_string());
            match (should_be_docked, slot) {
                (false, Some(s)) => {
                    self.dock_state.remove_tab(s);
                }
                (true, None) => {
                    if let Some(tile) = Tile::from_id(id) {
                        self.add_tab(tile);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn serialize_layout(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.dock_state).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::station::AppConfig;
    use egui_dock::{Node, TabPath};

    fn default_views() -> ViewsState {
        ViewsState::from_config(&AppConfig::default())
    }

    fn path_of(ws: &DockWorkspace, id: &str) -> TabPath {
        ws.dock_state
            .find_tab(&id.to_string())
            .unwrap_or_else(|| panic!("kafelek {id} musi być w układzie"))
    }

    /// Czy kafelek jest aktywną (widoczną) kartą swojego liścia.
    fn is_active_tab(ws: &DockWorkspace, id: &str) -> bool {
        let path = path_of(ws, id);
        ws.dock_state
            .leaf(path.node_path())
            .is_ok_and(|leaf| leaf.active == path.tab)
    }

    /// Udział szerokości liścia kafelka w całym obszarze (po drzewie podziałów poziomych).
    fn width_share(ws: &DockWorkspace, id: &str) -> f32 {
        let tree = &ws.dock_state.main_surface();
        let mut n = path_of(ws, id).node.0;
        let mut share = 1.0_f32;
        while n > 0 {
            let parent = (n - 1) / 2;
            if let Node::Horizontal(split) = &tree[NodeIndex(parent)] {
                share *= if n == 2 * parent + 1 {
                    split.fraction
                } else {
                    1.0 - split.fraction
                };
            }
            n = parent;
        }
        share
    }

    #[test]
    fn default_layout_shows_vfo_and_qso_simultaneously() {
        let mut ws = DockWorkspace::new();
        ws.rebuild_from_views(&default_views());

        let vfo = path_of(&ws, "vfo");
        let qso = path_of(&ws, "qso");
        assert_ne!(
            vfo.node, qso.node,
            "VFO i QSO muszą być w osobnych liściach (stos pionowy), nie w zakładkach"
        );
        assert!(is_active_tab(&ws, "vfo"), "VFO widoczne od razu");
        assert!(is_active_tab(&ws, "qso"), "QSO widoczne od razu");
        // VFO nad QSO: w układzie binarnym górny liść ma mniejszy indeks.
        assert!(vfo.node.0 < qso.node.0);
    }

    #[test]
    fn default_layout_logbook_has_own_leaf_and_widest_column() {
        let mut ws = DockWorkspace::new();
        ws.rebuild_from_views(&default_views());

        let log = path_of(&ws, "log");
        assert!(is_active_tab(&ws, "log"), "Logbook aktywny przy starcie");
        let leaf = ws.dock_state.leaf(log.node_path()).unwrap();
        assert_eq!(leaf.tabs.len(), 1, "Logbook nie dzieli liścia z zakładkami");

        let log_w = width_share(&ws, "log");
        assert!(
            (0.45..=0.48).contains(&log_w),
            "środkowa kolumna ~45-48%, jest {log_w}"
        );
        for other in ["vfo", "qso", "cluster"] {
            assert!(
                width_share(&ws, other) < log_w,
                "{other} nie może być szerszy niż Logbook"
            );
        }
        let left = width_share(&ws, "vfo");
        let right = width_share(&ws, "cluster");
        assert!((0.28..=0.30).contains(&left), "lewa ~28-30%, jest {left}");
        assert!(
            (0.22..=0.25).contains(&right),
            "prawa ~22-25%, jest {right}"
        );
    }

    #[test]
    fn default_layout_keeps_bandmap_off_the_qso_form() {
        let mut ws = DockWorkspace::new();
        ws.rebuild_from_views(&default_views());
        assert_ne!(
            path_of(&ws, "bandmap").node,
            path_of(&ws, "qso").node,
            "Bandmapa nie może przykrywać formularza QSO"
        );
        assert!(is_active_tab(&ws, "cluster"));
    }

    #[test]
    fn split_fractions_normalize_for_two_and_three_columns() {
        let three = DockWorkspace::split_fractions(&[0, 1, 2]);
        assert_eq!(three.len(), 2);
        assert!((three[0] - 0.29).abs() < 1e-3);
        let two = DockWorkspace::split_fractions(&[0, 1]);
        assert_eq!(two.len(), 1);
        assert!(two[0] < 0.5, "lewa węższa niż środkowa (Logbook)");
        let mid_right = DockWorkspace::split_fractions(&[1, 2]);
        assert!(mid_right[0] > 0.5, "Logbook szerszy niż prawa kolumna");
        assert_eq!(DockWorkspace::split_fractions(&[1]).len(), 0);
    }

    #[test]
    fn default_logbook_callsign_column_fits_at_1280px() {
        let cfg = AppConfig::default();
        let mut x = 72.0 + 24.0; // kolumny akcji i zaznaczania
        let mut found = false;
        for col in cfg.logbook_columns.iter().filter(|c| c.visible) {
            x += col.width;
            if col.id == "callsign" {
                found = true;
                break;
            }
        }
        assert!(found, "kolumna Znak musi być domyślnie widoczna");
        let log_width = 1280.0 * DockWorkspace::COLUMN_WEIGHTS[1];
        assert!(
            x < log_width,
            "kolumna Znak ({x}px od lewej) musi mieścić się w kolumnie Logbooka ({log_width}px)"
        );
    }

    #[test]
    fn default_matches_initial_values() {
        let ws = DockWorkspace::default();
        assert!(ws.last_saved_dock_layout.is_none());
        assert!(!ws.reset_layout_requested);
        assert!(ws.dragging_tile.is_none());
        assert_eq!(ws.active_tab, [0, 0, 0]);
    }

    #[test]
    fn dock_layout_serde_roundtrip() {
        let views = ViewsState::default();
        let mut ws = DockWorkspace::new();
        ws.rebuild_from_views(&views);
        let serialized = ws.serialize_layout().expect("should serialize");

        let mut restored = DockWorkspace::new();
        restored.build(Some(&serialized), &views);
        let reserialized = restored.serialize_layout().expect("should serialize");
        assert_eq!(serialized, reserialized);
    }

    #[test]
    fn sync_dock_state_adds_visible_panels() {
        let mut views = ViewsState::default();
        views.panel_satellites.visible = false;
        views.panel_world_map.visible = false;

        let mut ws = DockWorkspace::new();
        ws.rebuild_from_views(&views);
        assert!(ws.dock_state.find_tab(&"satellites".to_string()).is_none());

        views.panel_satellites.visible = true;
        ws.sync_with_views(&views);
        assert!(ws.dock_state.find_tab(&"satellites".to_string()).is_some());

        views.panel_satellites.floating = true;
        ws.sync_with_views(&views);
        assert!(ws.dock_state.find_tab(&"satellites".to_string()).is_none());
    }
}
