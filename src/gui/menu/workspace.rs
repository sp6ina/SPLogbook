// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Menu „Workspace" — szybkie przełączanie profili układu operatorskiego.

use crate::core::i18n::{key, tr};
use crate::core::station::workspace_profile_presets;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::workspace_profiles::apply_profile;
use eframe::egui;

/// Menu: Workspace (presety układu + zapisz/zarządzaj).
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    use crate::gui::theme::ThemeExt;
    let palette = ui.palette();
    let lang = app.gui.appearance.language;

    ui.menu_button(icons::DESKTOP.label(tr(key::MENU_WORKSPACE, lang)), |ui| {
        ui.label(
            egui::RichText::new(tr(key::WORKSPACE_BUILTIN_PRESETS, lang))
                .strong()
                .color(palette.warning),
        );
        ui.separator();

        for preset in workspace_profile_presets() {
            let name = preset.name.clone();
            let tooltip = preset.description.clone();
            let mut resp = ui.add(egui::Button::new(format!("• {name}")));
            if !tooltip.is_empty() {
                resp = resp.on_hover_text(tooltip);
            }
            if resp.clicked() {
                apply_profile(app, &preset);
                app.gui.chrome.status_toast = Some((
                    format!("{}: {name}", tr(key::WORKSPACE_PRESET_APPLIED, lang)),
                    std::time::Instant::now(),
                ));
                ui.close();
            }
        }

        ui.separator();
        if ui.button(tr(key::WORKSPACE_SAVE_CURRENT, lang)).clicked() {
            app.show_workspace_profiles_window = true;
            ui.close();
        }
        if ui
            .button(tr(key::WORKSPACE_MANAGE_PROFILES, lang))
            .clicked()
        {
            app.show_workspace_profiles_window = true;
            ui.close();
        }
    });
}
