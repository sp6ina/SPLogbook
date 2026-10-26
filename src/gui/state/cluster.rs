// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::{ClusterEvent, DxSpot};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotBadgeKind {
    NewDxcc,
    NewBand,
    Standard,
}

impl SpotBadgeKind {
    pub fn suffix(self) -> &'static str {
        match self {
            SpotBadgeKind::NewDxcc => " ⭐",
            SpotBadgeKind::NewBand => " ✨",
            SpotBadgeKind::Standard => "",
        }
    }

    pub fn color(self, palette: &crate::gui::theme::Palette) -> eframe::egui::Color32 {
        match self {
            SpotBadgeKind::NewDxcc => palette.accent_alt,
            SpotBadgeKind::NewBand => palette.success,
            SpotBadgeKind::Standard => palette.accent,
        }
    }
}

pub struct ClusterState {
    pub spots: Vec<DxSpot>,
    pub badge_cache: HashMap<String, SpotBadgeKind>,
    pub connected: bool,
    pub connecting: bool,
    pub status_text: String,
    pub show_filter_sidebar: bool,
    pub search_query: String,
    pub filter_band_selection: String,
    pub filter_mode_selection: String,
    pub filter_status: String,
    pub filter_source: String,
    pub filter_pota_sota_only: bool,
    pub show_add_cluster_dialog: bool,
    pub new_cluster_name: String,
    pub new_cluster_host: String,
    pub new_cluster_port: u16,
    pub event_rx: Option<std::sync::mpsc::Receiver<ClusterEvent>>,
    pub stop_tx: Option<tokio::sync::watch::Sender<bool>>,
    pub cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
}

impl Default for ClusterState {
    fn default() -> Self {
        Self {
            spots: Vec::new(),
            badge_cache: HashMap::new(),
            connected: false,
            connecting: false,
            status_text: String::new(),
            show_filter_sidebar: false,
            search_query: String::new(),
            filter_band_selection: "ALL".to_string(),
            filter_mode_selection: "ALL".to_string(),
            filter_status: "ALL".to_string(),
            filter_source: "ALL".to_string(),
            filter_pota_sota_only: false,
            show_add_cluster_dialog: false,
            new_cluster_name: String::new(),
            new_cluster_host: String::new(),
            new_cluster_port: 7300,
            event_rx: None,
            stop_tx: None,
            cmd_tx: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_state_default() {
        let state = ClusterState::default();
        assert!(state.spots.is_empty());
        assert!(state.badge_cache.is_empty());
        assert!(!state.connected);
        assert!(!state.connecting);
        assert!(state.status_text.is_empty());
        assert!(!state.show_filter_sidebar);
        assert!(state.search_query.is_empty());
        assert_eq!(state.filter_band_selection, "ALL");
        assert_eq!(state.filter_mode_selection, "ALL");
        assert_eq!(state.filter_status, "ALL");
        assert_eq!(state.filter_source, "ALL");
        assert!(!state.filter_pota_sota_only);
        assert!(!state.show_add_cluster_dialog);
        assert_eq!(state.new_cluster_port, 7300);
        assert!(state.event_rx.is_none());
        assert!(state.stop_tx.is_none());
        assert!(state.cmd_tx.is_none());
    }
}
