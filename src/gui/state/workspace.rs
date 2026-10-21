// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Dock workspace state and panel layout synchronization.

use egui_dock::{DockState, NodeIndex};

use super::views::ViewsState;
use crate::core::station::ViewPanelConfig;

/// Session state for the dockable workspace layout and tiles.
pub struct DockWorkspace {
    pub dock_state: DockState<String>,
    pub last_saved_dock_layout: Option<serde_json::Value>,
    pub reset_layout_requested: bool,
    pub dragging_tile: Option<String>,
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
            active_tab: [0; 3],
        }
    }

    /// Canonical order of tile identifiers used when building or synchronizing the dock tree.
    pub fn all_tile_ids() -> [&'static str; 9] {
        [
            "vfo",
            "qso",
            "log",
            "cluster",
            "bandmap",
            "solar",
            "satellites",
            "world_map",
            "waterfall",
        ]
    }

    pub fn panel_config<'a>(views: &'a ViewsState, id: &str) -> &'a ViewPanelConfig {
        match id {
            "qso" => &views.panel_qso,
            "log" => &views.panel_log,
            "cluster" => &views.panel_cluster,
            "bandmap" => &views.panel_bandmap,
            "solar" => &views.panel_solar,
            "satellites" => &views.panel_satellites,
            "world_map" => &views.panel_world_map,
            "waterfall" => &views.panel_waterfall,
            _ => &views.panel_vfo,
        }
    }

    pub fn build(&mut self, dock_layout: Option<&serde_json::Value>, views: &ViewsState) {
        if let Some(json) = dock_layout {
            if let Ok(state) = serde_json::from_value::<DockState<String>>(json.clone()) {
                self.dock_state = state;
                self.sync_with_views(views);
                self.last_saved_dock_layout = self.serialize_layout();
                return;
            }
        }

        self.rebuild_from_views(views);
        self.last_saved_dock_layout = self.serialize_layout();
    }

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

        let first = if !columns[0].is_empty() {
            0
        } else if !columns[1].is_empty() {
            1
        } else {
            2
        };

        self.dock_state = DockState::new(columns[first].clone());

        let mut rightmost = NodeIndex::root();
        for col_items in columns.iter().skip(first + 1) {
            if col_items.is_empty() {
                continue;
            }
            let [_, new_node] =
                self.dock_state
                    .main_surface_mut()
                    .split_right(rightmost, 0.5, col_items.clone());
            rightmost = new_node;
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
                    self.dock_state.push_to_first_leaf(id.to_string());
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
