//! Application update state.

/// State for update checks, changelog display, and binary installation.
#[derive(Default)]
pub struct UpdateState {
    pub show_update_window: bool,
    pub show_changelog_window: bool,
    pub update_check_status: Option<String>,
    pub update_check_rx:
        Option<std::sync::mpsc::Receiver<crate::cloud::updater::UpdateCheckOutcome>>,
    pub update_available: Option<crate::cloud::updater::LatestRelease>,
    pub update_install_status: Option<String>,
    pub update_install_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub pending_restart: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = UpdateState::default();
        assert!(!state.show_update_window);
        assert!(!state.show_changelog_window);
        assert!(state.update_check_status.is_none());
        assert!(state.update_check_rx.is_none());
        assert!(state.update_available.is_none());
        assert!(state.update_install_status.is_none());
        assert!(state.update_install_rx.is_none());
        assert!(!state.pending_restart);
    }
}
