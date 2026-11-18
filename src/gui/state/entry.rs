// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::awards::{PolishDistrictInfo, QsoAwardStatus};
use crate::core::clubs::ClubAffiliation;
use crate::core::prefix::PrefixInfo;
use crate::core::propagation::PropagationForecast;
use crate::core::qso::QsoRecord;
use crate::core::reference::tables::QslManagerRecord;
use crate::core::station::AppConfig;
use std::time::Instant;

/// Własne referencje stacji logującej — kontekst aktywacji (F-19).
///
/// Osobny typ, a nie dwa luźne pola, aby strony QSO były jawne także w stanie
/// formularza: `MyAwardRefs` opisuje **wyłącznie** stację SPLogbook i nigdy nie
/// jest kopiowany do pól korespondenta.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MyAwardRefs {
    /// Własna referencja SOTA (ADIF `MY_SOTA_REF`).
    pub sota: String,
    /// Własna referencja POTA (ADIF `MY_SIG_INFO`).
    pub pota: String,
}

#[derive(Debug, Clone)]
pub struct QsoEntryState {
    pub callsign: String,
    pub callsign_corrections: Vec<(String, usize)>,
    pub band: String,
    pub mode: String,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub exchange: String,
    pub name: String,
    pub qth: String,
    pub grid: String,
    pub pga: String,
    pub comment: String,
    pub iota: String,
    pub state: String,
    /// Referencja SOTA **korespondenta** (ADIF `SOTA_REF`).
    pub sota: String,
    /// Referencja POTA **korespondenta** (ADIF `SIG_INFO` w kontrakcie POTA).
    pub pota: String,
    /// Własne referencje stacji logującej — kontekst bieżącej aktywacji (F-19).
    pub my_refs: MyAwardRefs,
    pub qsl_manager: String,
    pub active_qsl_manager_record: Option<QslManagerRecord>,
    pub scp_suggestions: Vec<String>,
    pub active_prefix_info: Option<PrefixInfo>,
    pub active_polish_district: Option<PolishDistrictInfo>,
    pub active_award_status: Option<QsoAwardStatus>,
    pub active_distance_km: f64,
    pub active_bearing_deg: f64,
    pub active_propagation: Option<PropagationForecast>,
    pub active_clubs: Vec<ClubAffiliation>,
    pub past_qsos_for_active_call: Vec<QsoRecord>,
    pub focus_callsign_requested: bool,
    pub lookup_debounce_until: Option<Instant>,
}

impl Default for QsoEntryState {
    fn default() -> Self {
        Self {
            callsign: String::new(),
            callsign_corrections: Vec::new(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            rst_sent: "599".to_string(),
            rst_rcvd: "599".to_string(),
            exchange: String::new(),
            name: String::new(),
            qth: String::new(),
            grid: String::new(),
            pga: String::new(),
            comment: String::new(),
            iota: String::new(),
            state: String::new(),
            sota: String::new(),
            pota: String::new(),
            my_refs: MyAwardRefs::default(),
            qsl_manager: String::new(),
            active_qsl_manager_record: None,
            scp_suggestions: Vec::new(),
            active_prefix_info: None,
            active_polish_district: None,
            active_award_status: None,
            active_distance_km: 0.0,
            active_bearing_deg: 0.0,
            active_propagation: None,
            active_clubs: Vec::new(),
            past_qsos_for_active_call: Vec::new(),
            focus_callsign_requested: false,
            lookup_debounce_until: None,
        }
    }
}

impl QsoEntryState {
    /// Tworzy stan formularza z kontekstem stacji (F-19).
    ///
    /// Referencja własnej stacji (profil stacji) trafia wyłącznie do pól
    /// **lokalnych** (`my_sota`, `my_pota`), które opisują stronę SPLogbook.
    /// Pola korespondenta (`sota`, `pota`) pozostają puste — wypełnia je spot
    /// cudzego aktywatora albo ręczne wpisanie. Wcześniej referencja własna była
    /// wpisywana do pola korespondenta i eksportowana jako `SOTA_REF`/`POTA_REF`.
    pub fn new(config: &AppConfig) -> Self {
        Self {
            my_refs: MyAwardRefs {
                sota: config.station.sota_ref.clone().unwrap_or_default(),
                pota: config.station.pota_ref.clone().unwrap_or_default(),
            },
            ..Default::default()
        }
    }

    /// Czyści formularz po zapisaniu QSO.
    ///
    /// Własne referencje (`my_sota`, `my_pota`) **nie** są czyszczone: opisują
    /// bieżącą aktywację i obowiązują dla kolejnych łączności, dopóki operator
    /// ich nie zmieni. Referencje korespondenta są per-QSO i są czyszczone.
    pub fn clear(&mut self) {
        self.callsign.clear();
        self.callsign_corrections.clear();
        self.exchange.clear();
        self.name.clear();
        self.qth.clear();
        self.grid.clear();
        self.pga.clear();
        self.comment.clear();
        self.iota.clear();
        self.state.clear();
        self.sota.clear();
        self.pota.clear();
        self.qsl_manager.clear();
        self.active_qsl_manager_record = None;
        self.scp_suggestions.clear();
        self.active_prefix_info = None;
        self.active_polish_district = None;
        self.active_award_status = None;
        self.active_distance_km = 0.0;
        self.active_bearing_deg = 0.0;
        self.active_propagation = None;
        self.active_clubs.clear();
        self.past_qsos_for_active_call.clear();
        self.focus_callsign_requested = true;
        self.lookup_debounce_until = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = QsoEntryState::default();
        assert_eq!(state.callsign, "");
        assert_eq!(state.band, "20m");
        assert_eq!(state.mode, "CW");
        assert_eq!(state.rst_sent, "599");
        assert_eq!(state.rst_rcvd, "599");
        assert_eq!(state.exchange, "");
        assert_eq!(state.name, "");
        assert_eq!(state.qth, "");
        assert_eq!(state.grid, "");
        assert_eq!(state.pga, "");
        assert_eq!(state.comment, "");
        assert_eq!(state.iota, "");
        assert_eq!(state.state, "");
        assert_eq!(state.sota, "");
        assert_eq!(state.pota, "");
        assert_eq!(state.my_refs, MyAwardRefs::default());
        assert_eq!(state.qsl_manager, "");
        assert!(state.active_qsl_manager_record.is_none());
        assert_eq!(state.scp_suggestions.len(), 0);
        assert_eq!(state.callsign_corrections.len(), 0);
        assert!(state.active_prefix_info.is_none());
        assert!(state.active_polish_district.is_none());
        assert!(state.active_award_status.is_none());
        assert_eq!(state.active_distance_km, 0.0);
        assert_eq!(state.active_bearing_deg, 0.0);
        assert!(state.active_propagation.is_none());
        assert_eq!(state.active_clubs.len(), 0);
        assert_eq!(state.past_qsos_for_active_call.len(), 0);
        assert!(!state.focus_callsign_requested);
        assert!(state.lookup_debounce_until.is_none());
    }

    #[test]
    fn new_inherits_own_references_into_the_local_side_only() {
        let mut cfg = AppConfig::default();
        cfg.station.sota_ref = Some("AA/BB-001".to_string());
        cfg.station.pota_ref = Some("XX-0001".to_string());
        let state = QsoEntryState::new(&cfg);
        // F-19: the station profile describes the LOCAL station, so its references
        // may only appear on the local side of the entry state.
        assert_eq!(state.my_refs.sota, "AA/BB-001");
        assert_eq!(state.my_refs.pota, "XX-0001");
        assert_eq!(state.sota, "");
        assert_eq!(state.pota, "");
    }

    #[test]
    fn clear_keeps_the_activation_context_on_the_local_side() {
        let mut state = QsoEntryState {
            callsign: "TSTREMOTE".to_string(),
            sota: "CC/DD-999".to_string(),
            pota: "YY-9999".to_string(),
            my_refs: MyAwardRefs {
                sota: "AA/BB-001".to_string(),
                pota: "XX-0001".to_string(),
            },
            ..Default::default()
        };
        state.clear();
        assert_eq!(state.sota, "", "the correspondent reference is per-QSO");
        assert_eq!(state.pota, "");
        assert_eq!(
            state.my_refs.sota, "AA/BB-001",
            "the activation context persists"
        );
        assert_eq!(state.my_refs.pota, "XX-0001");
    }

    #[test]
    fn clear_resets_fields() {
        let mut state = QsoEntryState {
            callsign: "SP6INA".to_string(),
            name: "Mariusz".to_string(),
            grid: "JO81LA".to_string(),
            active_distance_km: 123.4,
            focus_callsign_requested: false,
            ..Default::default()
        };
        state.clear();
        assert_eq!(state.callsign, "");
        assert_eq!(state.name, "");
        assert_eq!(state.grid, "");
        assert_eq!(state.active_distance_km, 0.0);
        assert!(state.focus_callsign_requested);
    }
}
