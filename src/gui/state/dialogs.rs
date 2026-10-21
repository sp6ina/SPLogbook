// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Aggregated dialog and auxiliary tool state.

use crate::gui::advanced_filter::AdvancedFilterDialog;
use crate::gui::astronomy_dialog::AstronomyDialog;
use crate::gui::csv_export_dialog::CsvExportDialog;
use crate::gui::cw_terminal::CwTerminalDialog;
use crate::gui::iota_browser::IotaBrowserDialog;
use crate::gui::journal_manager::JournalManagerDialog;
use crate::gui::photo_viewer::PhotoViewerDialog;
use crate::gui::prefix_manager::PrefixManagerDialog;
use crate::gui::qsl_designer::QslDesignerDialog;
use crate::gui::qsl_manager::QslManagerDialog;
use crate::gui::send_spot::SendSpotDialog;
use crate::gui::sota_dialog::SotaDialog;
use crate::gui::states_browser::StatesBrowserDialog;
use crate::gui::waterfall_panel::WaterfallPanel;
use crate::gui::wol_dialog::WolDialog;

/// Collection of 15 modal/modeless dialogs and panels.
pub struct DialogState {
    pub journal_dialog: JournalManagerDialog,
    pub advanced_filter_dialog: AdvancedFilterDialog,
    pub cw_terminal_dialog: CwTerminalDialog,
    pub qsl_designer_dialog: QslDesignerDialog,
    pub send_spot_dialog: SendSpotDialog,
    pub iota_dialog: IotaBrowserDialog,
    pub states_dialog: StatesBrowserDialog,
    pub qsl_manager_dialog: QslManagerDialog,
    pub photo_viewer_dialog: PhotoViewerDialog,
    pub prefix_manager_dialog: PrefixManagerDialog,
    pub astronomy_dialog: AstronomyDialog,
    pub wol_dialog: WolDialog,
    pub sota_dialog: SotaDialog,
    pub csv_export_dialog: CsvExportDialog,
    pub waterfall_panel: WaterfallPanel,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            journal_dialog: JournalManagerDialog::default(),
            advanced_filter_dialog: AdvancedFilterDialog::default(),
            cw_terminal_dialog: CwTerminalDialog::default(),
            qsl_designer_dialog: QslDesignerDialog::default(),
            send_spot_dialog: SendSpotDialog::new(),
            iota_dialog: IotaBrowserDialog::new(),
            states_dialog: StatesBrowserDialog::new(),
            qsl_manager_dialog: QslManagerDialog::new(),
            photo_viewer_dialog: PhotoViewerDialog::new(),
            prefix_manager_dialog: PrefixManagerDialog::new(),
            astronomy_dialog: AstronomyDialog::new(),
            wol_dialog: WolDialog::new(),
            sota_dialog: SotaDialog::new(),
            csv_export_dialog: CsvExportDialog::new(),
            waterfall_panel: WaterfallPanel::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = DialogState::default();
        assert!(!state.journal_dialog.is_open);
        assert!(!state.advanced_filter_dialog.is_open);
        assert!(!state.cw_terminal_dialog.is_open);
        assert!(!state.qsl_designer_dialog.is_open);
        assert!(!state.send_spot_dialog.is_open);
        assert!(!state.iota_dialog.is_open);
        assert!(!state.states_dialog.is_open);
        assert!(!state.qsl_manager_dialog.is_open);
        assert!(!state.photo_viewer_dialog.is_open);
        assert!(!state.prefix_manager_dialog.is_open);
        assert!(!state.astronomy_dialog.is_open);
        assert!(!state.wol_dialog.is_open);
        assert!(!state.sota_dialog.is_open);
        assert!(!state.csv_export_dialog.is_open);
    }
}
