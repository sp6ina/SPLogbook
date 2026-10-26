// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno zarządzania profilami układu operatorskiego (workspace).
//! Umożliwia zapisywanie i przywracanie widoczności / pozycji / rozmiarów
//! paneli głównego okna oraz wybór wbudowanych presetów.

use crate::core::i18n::{Msg, key, tr};
use crate::core::station::{
    ClusterFilter, WorkspaceFile, WorkspaceProfile, workspace_profile_presets,
};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemePreset;
use eframe::egui;
use egui_dock::DockState;

/// Rysuje okno profili układu, jeśli jest otwarte.
pub fn render_workspace_profiles_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.gui.windows.show_workspace_profiles_window {
        return;
    }
    let lang = app.gui.appearance.language;

    let mut open = app.gui.windows.show_workspace_profiles_window;
    egui::Window::new(tr(key::WS_PROFILES_WINDOW_TITLE, lang))
        .open(&mut open)
        .default_size([560.0, 440.0])
        .resizable(true)
        .show(ctx, |ui| {
            use crate::gui::theme::ThemeExt;
            let palette = ui.palette();
            ui.heading(
                egui::RichText::new(tr(key::WS_PROFILES_HEADING, lang))
                    .size(15.0)
                    .color(palette.accent),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(tr(key::WS_PROFILES_DESCRIPTION, lang))
                    .small()
                    .color(palette.text_muted),
            );
            ui.add_space(6.0);

            // Sekcja: wbudowane presety
            ui.label(
                egui::RichText::new(tr(key::WS_PROFILES_BUILTIN_PRESETS, lang))
                    .strong()
                    .color(palette.warning),
            );
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    let mut apply_preset: Option<usize> = None;
                    for (i, preset) in workspace_profile_presets().iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", preset.name));
                            if ui.button(tr(key::WS_PROFILES_APPLY, lang)).clicked() {
                                apply_preset = Some(i);
                            }
                        });
                    }
                    if let Some(i) = apply_preset {
                        if let Some(preset) = workspace_profile_presets().into_iter().nth(i) {
                            apply_profile(app, &preset);
                            app.gui.chrome.status_toast = Some((
                                format!("Zastosowano preset: {}", preset.name),
                                std::time::Instant::now(),
                            ));
                        }
                    }
                });
            ui.separator();

            // Sekcja: własne profile
            ui.label(
                egui::RichText::new(tr(key::WS_PROFILES_CUSTOM_PROFILES, lang))
                    .strong()
                    .color(palette.success),
            );

            // Pole nazwy nowego profilu (przechowywane w pamięci egui)
            let name_id = egui::Id::new("workspace_profile_name_input");
            let mut new_name = ctx
                .data_mut(|d| d.get_temp::<String>(name_id))
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(tr(key::WS_PROFILES_NAME_LABEL, lang));
                ui.add(
                    egui::TextEdit::singleline(&mut new_name)
                        .hint_text("np. DX-pileup 20m")
                        .desired_width(220.0),
                );
                let can_save = !new_name.trim().is_empty();
                if ui
                    .add_enabled(
                        can_save,
                        egui::Button::new(tr(key::WS_PROFILES_SAVE_CURRENT_LAYOUT, lang)),
                    )
                    .clicked()
                {
                    let profile = capture_profile(app, new_name.trim());
                    // Nadpisanie istniejącego profilu o tej samej nazwie
                    if let Some(existing) = app
                        .config
                        .workspace_profiles
                        .iter_mut()
                        .find(|p| p.name == profile.name)
                    {
                        *existing = profile.clone();
                    } else {
                        app.config.workspace_profiles.push(profile.clone());
                    }
                    app.save_station_config();
                    app.gui.chrome.status_toast = Some((
                        format!("Zapisano profil: {}", profile.name),
                        std::time::Instant::now(),
                    ));
                    new_name.clear();
                }
            });
            ctx.data_mut(|d| d.insert_temp(name_id, new_name.clone()));

            ui.add_space(6.0);
            if app.config.workspace_profiles.is_empty() {
                ui.label(
                    egui::RichText::new(tr(key::WS_PROFILES_NO_PROFILES, lang))
                        .italics()
                        .color(palette.text_muted),
                );
            } else {
                let rename_idx_id = egui::Id::new("workspace_rename_idx");
                let rename_text_id = egui::Id::new("workspace_rename_text");
                let mut renaming: Option<usize> =
                    ctx.data_mut(|d| d.get_temp(rename_idx_id)).flatten();
                let mut rename_text = ctx
                    .data_mut(|d| d.get_temp::<String>(rename_text_id))
                    .unwrap_or_default();

                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        let mut apply_idx: Option<usize> = None;
                        let mut delete_idx: Option<usize> = None;
                        let mut export_idx: Option<usize> = None;
                        let mut commit_rename: Option<usize> = None;
                        for (idx, profile) in app.config.workspace_profiles.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if renaming == Some(idx) {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut rename_text)
                                            .desired_width(200.0),
                                    );
                                    if ui.button("OK").clicked() && !rename_text.trim().is_empty() {
                                        commit_rename = Some(idx);
                                    }
                                    if ui.button("✖").clicked() {
                                        renaming = None;
                                    }
                                } else {
                                    ui.label(format!("• {}", profile.name));
                                    if ui.button(tr(key::WS_PROFILES_APPLY, lang)).clicked() {
                                        apply_idx = Some(idx);
                                    }
                                    if ui
                                        .button("✏")
                                        .on_hover_text(tr(key::WS_PROFILES_RENAME_HINT, lang))
                                        .clicked()
                                    {
                                        renaming = Some(idx);
                                        rename_text.clone_from(&profile.name);
                                    }
                                    if ui
                                        .button("⤓")
                                        .on_hover_text(tr(key::WS_PROFILES_EXPORT_HINT, lang))
                                        .clicked()
                                    {
                                        export_idx = Some(idx);
                                    }
                                    if ui
                                        .button("🗑")
                                        .on_hover_text(tr(key::WS_PROFILES_DELETE_HINT, lang))
                                        .clicked()
                                    {
                                        delete_idx = Some(idx);
                                    }
                                }
                            });
                        }
                        if let Some(idx) = apply_idx {
                            let profile = app.config.workspace_profiles[idx].clone();
                            apply_profile(app, &profile);
                            app.gui.chrome.status_toast = Some((
                                format!("Zastosowano profil: {}", profile.name),
                                std::time::Instant::now(),
                            ));
                        }
                        if let Some(idx) = commit_rename {
                            app.config.workspace_profiles[idx].name =
                                rename_text.trim().to_string();
                            app.config.workspace_profiles[idx].ensure_id();
                            app.save_station_config();
                            app.gui.chrome.status_toast = Some((
                                tr(key::WS_PROFILES_RENAMED_TOAST, lang).to_string(),
                                std::time::Instant::now(),
                            ));
                            renaming = None;
                        }
                        if let Some(idx) = delete_idx {
                            app.config.workspace_profiles.remove(idx);
                            app.save_station_config();
                        }
                        if let Some(idx) = export_idx {
                            export_profile(app, &app.config.workspace_profiles[idx].clone());
                        }
                    });

                ctx.data_mut(|d| d.insert_temp(rename_idx_id, renaming));
                ctx.data_mut(|d| d.insert_temp(rename_text_id, rename_text));
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(tr(key::WS_PROFILES_IMPORT_BTN, lang)).clicked() {
                    import_profile(app);
                }
                ui.label(
                    egui::RichText::new(tr(key::WS_PROFILES_SHARE_NOTE, lang))
                        .small()
                        .color(palette.text_muted),
                );
            });
        });
    app.gui.windows.show_workspace_profiles_window = open;
}

/// Eksportuje pojedynczy profil do pliku `*.spws` (JSON z polem wersji).
fn export_profile(app: &mut SpLogApp, profile: &WorkspaceProfile) {
    let lang = app.gui.appearance.language;
    let file_name = format!("{}.spws", profile.id);
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("SPLogbook Workspace", &["spws"])
        .set_file_name(&file_name)
        .save_file()
    {
        let file = WorkspaceFile {
            version: WorkspaceFile::CURRENT_VERSION,
            workspace: profile.clone(),
        };
        match serde_json::to_string_pretty(&file) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => {
                    app.gui.chrome.status_toast = Some((
                        format!("Wyeksportowano: {}", path.display()),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    app.gui.chrome.status_toast = Some((
                        Msg::new(key::WS_PROFILES_ERR_SAVE).arg("err", e).text(lang),
                        std::time::Instant::now(),
                    ));
                }
            },
            Err(e) => {
                app.gui.chrome.status_toast = Some((
                    Msg::new(key::WS_PROFILES_ERR_SERIALIZE)
                        .arg("err", e)
                        .text(lang),
                    std::time::Instant::now(),
                ));
            }
        }
    }
}

/// Importuje profil z pliku `*.spws`.
fn import_profile(app: &mut SpLogApp) {
    let lang = app.gui.appearance.language;
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("SPLogbook Workspace", &["spws"])
        .pick_file()
    {
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<WorkspaceFile>(&text) {
                Ok(file) => {
                    if file.version > WorkspaceFile::CURRENT_VERSION {
                        app.gui.chrome.status_toast = Some((
                            Msg::new(key::WS_PROFILES_ERR_UNSUPPORTED_VERSION)
                                .arg("ver", file.version)
                                .arg("path", path.display())
                                .text(lang),
                            std::time::Instant::now(),
                        ));
                        return;
                    }
                    let mut ws = file.workspace;
                    ws.ensure_id();
                    // Nadpisz profil o tym samym id lub dodaj nowy.
                    if let Some(existing) = app
                        .config
                        .workspace_profiles
                        .iter_mut()
                        .find(|p| p.id == ws.id)
                    {
                        *existing = ws.clone();
                    } else {
                        app.config.workspace_profiles.push(ws.clone());
                    }
                    app.save_station_config();
                    app.gui.chrome.status_toast = Some((
                        format!("Zaimportowano profil: {}", ws.name),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    app.gui.chrome.status_toast = Some((
                        Msg::new(key::WS_PROFILES_ERR_INVALID_FILE)
                            .arg("err", e)
                            .text(lang),
                        std::time::Instant::now(),
                    ));
                }
            },
            Err(e) => {
                app.gui.chrome.status_toast = Some((
                    Msg::new(key::WS_PROFILES_ERR_READ).arg("err", e).text(lang),
                    std::time::Instant::now(),
                ));
            }
        }
    }
}

/// Zbiera bieżący stan paneli do profilu.
fn capture_profile(app: &SpLogApp, name: &str) -> WorkspaceProfile {
    let mut profile = WorkspaceProfile {
        id: String::new(),
        name: name.to_string(),
        description: String::new(),
        panel_vfo: app.views.panel_vfo.clone(),
        panel_qso: app.views.panel_qso.clone(),
        panel_log: app.views.panel_log.clone(),
        panel_cluster: app.views.panel_cluster.clone(),
        panel_solar: app.views.panel_solar.clone(),
        panel_bandmap: app.views.panel_bandmap.clone(),
        panel_satellites: app.views.panel_satellites.clone(),
        panel_world_map: app.views.panel_world_map.clone(),
        panel_waterfall: app.views.panel_waterfall.clone(),
        theme_preset: Some(app.gui.appearance.theme_preset.id().to_string()),
        enabled_plugins: Vec::new(),
        cat_profile: None,
        cw_profile: None,
        cluster_filter: Some(ClusterFilter {
            band: app.cluster.filter_band_selection.clone(),
            mode: app.cluster.filter_mode_selection.clone(),
            source: app.cluster.filter_source.clone(),
            pota_sota_only: app.cluster.filter_pota_sota_only,
        }),
        dock_layout: app.serialize_dock_layout(),
    };
    profile.ensure_id();
    profile
}

/// Stosuje profil do bieżącego układu.
pub(crate) fn apply_profile(app: &mut SpLogApp, p: &WorkspaceProfile) {
    // Motyw kolorystyczny przypisany do profilu (jeśli określony).
    if let Some(theme_id) = &p.theme_preset {
        app.gui.appearance.theme_preset = ThemePreset::from_id(theme_id);
    }

    // Filtr spotów DX Cluster przypisany do profilu (jeśli zapisany).
    if let Some(f) = &p.cluster_filter {
        app.cluster.filter_band_selection.clone_from(&f.band);
        app.cluster.filter_mode_selection.clone_from(&f.mode);
        app.cluster.filter_source.clone_from(&f.source);
        app.cluster.filter_pota_sota_only = f.pota_sota_only;
    }

    app.views.panel_vfo = p.panel_vfo.clone();
    app.views.panel_qso = p.panel_qso.clone();
    app.views.panel_log = p.panel_log.clone();
    app.views.panel_cluster = p.panel_cluster.clone();
    app.views.panel_solar = p.panel_solar.clone();
    app.views.panel_bandmap = p.panel_bandmap.clone();
    app.views.panel_satellites = p.panel_satellites.clone();
    app.views.panel_world_map = p.panel_world_map.clone();
    app.views.panel_waterfall = p.panel_waterfall.clone();

    // Synchronizacja okien pływających (popout) dla paneli z obsługą ↗.
    app.gui.windows.show_bandmap_window = p.panel_bandmap.visible && p.panel_bandmap.floating;
    app.satellites.show_window = p.panel_satellites.visible && p.panel_satellites.floating;
    app.world_map.show_window = p.panel_world_map.visible && p.panel_world_map.floating;

    // Odtwórz układ dokowania. Preferuj zapisany układ egui_dock (drzewo
    // podziałów, karty, powierzchnie okien); w przeciwnym razie odbuduj
    // z pól column/order, aby zmiany kolumn/kolejności miały skutek.
    if let Some(json) = &p.dock_layout {
        if let Ok(state) = serde_json::from_value::<DockState<String>>(json.clone()) {
            app.gui.workspace.dock_state = state;
        } else {
            app.rebuild_dock_state_from_panels();
        }
    } else {
        app.rebuild_dock_state_from_panels();
    }
    // Uzgodnij widoczność/odpięcie paneli z odtworzonym układem.
    app.sync_dock_state();
    app.gui.workspace.last_saved_dock_layout = app.serialize_dock_layout();

    // Wymuś ponowny rozkład okien pływających.
    app.gui.workspace.reset_layout_requested = true;
    app.save_station_config();

    // Powiadom wtyczki Rhai o zmianie profilu układu.
    app.services.plugin_engine.run_on_workspace_changed(&p.name);
}
