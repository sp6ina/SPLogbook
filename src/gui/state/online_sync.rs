// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Online synchronization UI state (LoTW, eQSL, Club Log, etc.).

/// State for the online synchronization window and credentials.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineSyncUiState {
    pub show_online_sync_window: bool,
    pub online_sync_logs: Vec<String>,
    pub clublog_callsign: String,
    pub clublog_email: String,
    pub clublog_password: String,
    pub clublog_api_key: String,
    pub eqsl_username: String,
    pub eqsl_password: String,
}

impl OnlineSyncUiState {
    pub fn from_config(config: &crate::core::station::AppConfig) -> Self {
        Self {
            show_online_sync_window: false,
            online_sync_logs: Vec::new(),
            clublog_callsign: config.clublog_callsign.clone(),
            clublog_email: config.clublog_email.clone(),
            clublog_password: config.clublog_password.clone(),
            clublog_api_key: config.clublog_api_key.clone(),
            eqsl_username: config.eqsl_username.clone(),
            eqsl_password: config.eqsl_password.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = OnlineSyncUiState::default();
        assert!(!state.show_online_sync_window);
        assert!(state.online_sync_logs.is_empty());
        assert!(state.clublog_callsign.is_empty());
        assert!(state.clublog_email.is_empty());
        assert!(state.clublog_password.is_empty());
        assert!(state.clublog_api_key.is_empty());
        assert!(state.eqsl_username.is_empty());
        assert!(state.eqsl_password.is_empty());
    }
}
