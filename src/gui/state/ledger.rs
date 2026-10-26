// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::station::EquipmentCategory;

/// Stan księgi sprzętowej stacji oraz formularzy dodawania/edycji sprzętu.
#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentLedgerState {
    pub show_window: bool,
    pub new_model: String,
    pub editing_id: Option<String>,
    pub edit_cat: EquipmentCategory,
    pub edit_mfr: String,
    pub edit_model: String,
    pub edit_sn: String,
    pub edit_date: String,
    pub edit_notes: String,
    pub new_cat: EquipmentCategory,
    pub new_mfr: String,
    pub new_sn: String,
    pub new_notes: String,
}

impl Default for EquipmentLedgerState {
    fn default() -> Self {
        Self {
            show_window: false,
            new_model: String::new(),
            editing_id: None,
            edit_cat: EquipmentCategory::Transceiver,
            edit_mfr: String::new(),
            edit_model: String::new(),
            edit_sn: String::new(),
            edit_date: String::new(),
            edit_notes: String::new(),
            new_cat: EquipmentCategory::Transceiver,
            new_mfr: String::new(),
            new_sn: String::new(),
            new_notes: String::new(),
        }
    }
}

impl EquipmentLedgerState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = EquipmentLedgerState::default();
        assert!(!state.show_window);
        assert_eq!(state.new_model, "");
        assert_eq!(state.editing_id, None);
        assert_eq!(state.edit_cat, EquipmentCategory::Transceiver);
        assert_eq!(state.edit_mfr, "");
        assert_eq!(state.edit_model, "");
        assert_eq!(state.edit_sn, "");
        assert_eq!(state.edit_date, "");
        assert_eq!(state.edit_notes, "");
        assert_eq!(state.new_cat, EquipmentCategory::Transceiver);
        assert_eq!(state.new_mfr, "");
        assert_eq!(state.new_sn, "");
        assert_eq!(state.new_notes, "");

        let new_state = EquipmentLedgerState::new();
        assert_eq!(new_state, state);
    }
}
