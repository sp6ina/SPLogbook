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
        let mut locked = app.layout_locked();
        if ui
            .checkbox(&mut locked, tr(key::WORKSPACE_LOCK, lang))
            .on_hover_text(tr(key::WORKSPACE_LOCK_HINT, lang))
            .changed()
        {
            app.set_layout_locked(locked);
        }
        ui.separator();
        ui.label(
            egui::RichText::new(tr(key::WORKSPACE_BUILTIN_PRESETS, lang))
                .strong()
                .color(palette.warning),
        );
        ui.separator();

        for preset in workspace_profile_presets() {
            let (name, tooltip) = crate::gui::workspace_profiles::preset_text(&preset, lang);
            let mut resp = ui.add(egui::Button::new(format!("• {name}")));
            if !tooltip.is_empty() {
                resp = resp.on_hover_text(tooltip);
            }
            if resp.clicked() {
                apply_profile(app, &preset);
                app.gui.chrome.push_status_toast(
                    crate::gui::state::ToastSeverity::Info,
                    format!("{}: {name}", tr(key::WORKSPACE_PRESET_APPLIED, lang)),
                );
                ui.close();
            }
        }

        ui.separator();
        ui.label(
            egui::RichText::new(tr(key::WORKSPACE_PAGES, lang))
                .strong()
                .color(palette.accent),
        );
        let active = app.active_workspace_page();
        let mut switch_to = None;
        for (index, page) in app.workspace_pages().iter().enumerate() {
            if ui.selectable_label(index == active, &page.name).clicked() {
                switch_to = Some(index);
            }
        }
        if let Some(index) = switch_to {
            app.switch_workspace_page(index);
            ui.close();
        }
        if ui
            .add_enabled(
                app.workspace_pages().len() < crate::gui::workspace_pages::MAX_PAGES,
                egui::Button::new(tr(key::WORKSPACE_PAGE_ADD, lang)),
            )
            .clicked()
        {
            app.add_workspace_page();
            ui.close();
        }
        let has_pages = !app.workspace_pages().is_empty();
        if ui
            .add_enabled(
                has_pages,
                egui::Button::new(tr(key::WORKSPACE_PAGE_RENAME, lang)),
            )
            .clicked()
        {
            app.start_page_rename(active);
            ui.close();
        }
        if ui
            .add_enabled(
                has_pages,
                egui::Button::new(tr(key::WORKSPACE_PAGE_REMOVE, lang)),
            )
            .clicked()
        {
            app.remove_workspace_page(active);
            ui.close();
        }

        ui.separator();
        if ui.button(tr(key::WORKSPACE_SAVE_CURRENT, lang)).clicked() {
            app.gui.windows.show_workspace_profiles_window = true;
            ui.close();
        }
        if ui
            .button(tr(key::WORKSPACE_MANAGE_PROFILES, lang))
            .clicked()
        {
            app.gui.windows.show_workspace_profiles_window = true;
            ui.close();
        }
    });
}
