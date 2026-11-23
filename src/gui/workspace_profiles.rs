// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno zarządzania profilami układu operatorskiego (workspace).
//! Umożliwia zapisywanie i przywracanie widoczności / pozycji / rozmiarów
//! paneli głównego okna oraz wybór wbudowanych presetów.

use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::station::{
    ClusterFilter, WorkspaceFile, WorkspaceProfile, workspace_profile_presets,
};
use crate::gui::app::SpLogApp;
use crate::gui::state::Tile;
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
                        let (name, description) = preset_text(preset, lang);
                        ui.horizontal(|ui| {
                            ui.label(format!("• {name}")).on_hover_text(description);
                            if ui.button(tr(key::WS_PROFILES_APPLY, lang)).clicked() {
                                apply_preset = Some(i);
                            }
                        });
                    }
                    if let Some(i) = apply_preset
                        && let Some(preset) = workspace_profile_presets().into_iter().nth(i)
                    {
                        apply_profile(app, &preset);
                        let (name, _) = preset_text(&preset, lang);
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Info,
                            format!("{}: {name}", tr(key::WORKSPACE_PRESET_APPLIED, lang)),
                        );
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
                        .hint_text(tr(key::WS_PROFILES_NAME_HINT, lang))
                        .char_limit(MAX_PROFILE_NAME_CHARS)
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
                    app.gui.chrome.push_status_toast(
                        crate::gui::state::ToastSeverity::Success,
                        Msg::new(key::WS_PROFILES_SAVED_TOAST)
                            .arg("name", &profile.name)
                            .text(lang),
                    );
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
                                    if ui
                                        .button(tr(key::WORKSPACE_PAGE_RENAME_APPLY, lang))
                                        .clicked()
                                        && !rename_text.trim().is_empty()
                                    {
                                        commit_rename = Some(idx);
                                    }
                                    if ui
                                        .button("✖")
                                        .on_hover_text(tr(key::BTN_CANCEL, lang))
                                        .clicked()
                                    {
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
                            app.gui.chrome.push_status_toast(
                                crate::gui::state::ToastSeverity::Info,
                                Msg::new(key::WS_PROFILES_APPLIED_TOAST)
                                    .arg("name", &profile.name)
                                    .text(lang),
                            );
                        }
                        if let Some(idx) = commit_rename {
                            app.config.workspace_profiles[idx].name =
                                rename_text.trim().to_string();
                            app.config.workspace_profiles[idx].ensure_id();
                            app.save_station_config();
                            app.gui.chrome.push_status_toast(
                                crate::gui::state::ToastSeverity::Info,
                                tr(key::WS_PROFILES_RENAMED_TOAST, lang).to_string(),
                            );
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

/// Largest `.spws` file accepted on import.
pub const MAX_WORKSPACE_FILE_BYTES: u64 = 1024 * 1024;
/// Longest profile name, in characters.
pub const MAX_PROFILE_NAME_CHARS: usize = 64;

/// Why a `.spws` file was not imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceImportError {
    TooLarge,
    Invalid(String),
    UnsupportedVersion(u32),
}

impl std::fmt::Display for WorkspaceImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "file larger than {MAX_WORKSPACE_FILE_BYTES} bytes"),
            Self::Invalid(detail) => write!(f, "{detail}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported version {version}"),
        }
    }
}

impl std::error::Error for WorkspaceImportError {}

/// Reads a `.spws` file. The name is cleaned and the id derived from it again; module
/// entries for unknown tiles are dropped, and so is a dock layout that does not parse or
/// names tiles this version does not know (the layout is then rebuilt from the panels).
pub fn parse_workspace_file(text: &str) -> Result<WorkspaceProfile, WorkspaceImportError> {
    if text.len() as u64 > MAX_WORKSPACE_FILE_BYTES {
        return Err(WorkspaceImportError::TooLarge);
    }
    let file: WorkspaceFile =
        serde_json::from_str(text).map_err(|e| WorkspaceImportError::Invalid(e.to_string()))?;
    if file.version == 0 || file.version > WorkspaceFile::CURRENT_VERSION {
        return Err(WorkspaceImportError::UnsupportedVersion(file.version));
    }
    let mut ws = file.workspace;
    ws.name = ws
        .name
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_PROFILE_NAME_CHARS)
        .collect();
    if ws.name.is_empty() {
        return Err(WorkspaceImportError::Invalid(
            "empty profile name".to_string(),
        ));
    }
    ws.id.clear();
    ws.ensure_id();
    ws.module_panels
        .retain(|id, _| Tile::from_id(id).is_some_and(Tile::is_module));
    let layout_ok = ws.dock_layout.as_ref().is_none_or(|json| {
        serde_json::from_value::<DockState<String>>(json.clone()).is_ok_and(|state| {
            state
                .iter_all_tabs()
                .all(|(_, tab)| Tile::from_id(tab).is_some())
        })
    });
    if !layout_ok {
        ws.dock_layout = None;
    }
    Ok(ws)
}

/// Translated name and description of a built-in preset; other profiles keep their own.
pub fn preset_text(profile: &WorkspaceProfile, lang: Language) -> (String, String) {
    let keys = match profile.id.as_str() {
        "normal-dx" => Some((key::WS_PRESET_NORMAL_DX_NAME, key::WS_PRESET_NORMAL_DX_DESC)),
        "contest" => Some((key::WS_PRESET_CONTEST_NAME, key::WS_PRESET_CONTEST_DESC)),
        "ft8" => Some((key::WS_PRESET_FT8_NAME, key::WS_PRESET_FT8_DESC)),
        "pota" => Some((key::WS_PRESET_POTA_NAME, key::WS_PRESET_POTA_DESC)),
        "sota" => Some((key::WS_PRESET_SOTA_NAME, key::WS_PRESET_SOTA_DESC)),
        "satellite" => Some((key::WS_PRESET_SATELLITE_NAME, key::WS_PRESET_SATELLITE_DESC)),
        "eme" => Some((key::WS_PRESET_EME_NAME, key::WS_PRESET_EME_DESC)),
        _ => None,
    };
    match keys {
        Some((name, desc)) => (tr(name, lang).to_string(), tr(desc, lang).to_string()),
        None => (profile.name.clone(), profile.description.clone()),
    }
}

/// Eksportuje pojedynczy profil do pliku `*.spws` (JSON z polem wersji).
fn export_profile(app: &mut SpLogApp, profile: &WorkspaceProfile) {
    let lang = app.gui.appearance.language;
    let file_name = format!("{}.spws", profile.id);
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(tr(key::WS_PROFILES_FILE_FILTER, lang), &["spws"])
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
                    app.gui.chrome.push_status_toast(
                        crate::gui::state::ToastSeverity::Success,
                        Msg::new(key::WS_PROFILES_EXPORTED_TOAST)
                            .arg("path", path.display())
                            .text(lang),
                    );
                }
                Err(e) => {
                    app.gui.chrome.push_status_toast(
                        crate::gui::state::ToastSeverity::Error,
                        Msg::new(key::WS_PROFILES_ERR_SAVE).arg("err", e).text(lang),
                    );
                }
            },
            Err(e) => {
                app.gui.chrome.push_status_toast(
                    crate::gui::state::ToastSeverity::Error,
                    Msg::new(key::WS_PROFILES_ERR_SERIALIZE)
                        .arg("err", e)
                        .text(lang),
                );
            }
        }
    }
}

/// Importuje profil z pliku `*.spws`.
fn import_profile(app: &mut SpLogApp) {
    let lang = app.gui.appearance.language;
    let Some(path) = rfd::FileDialog::new()
        .add_filter(tr(key::WS_PROFILES_FILE_FILTER, lang), &["spws"])
        .pick_file()
    else {
        return;
    };
    let read = std::fs::metadata(&path).and_then(|meta| {
        if meta.len() > MAX_WORKSPACE_FILE_BYTES {
            Ok(None)
        } else {
            std::fs::read_to_string(&path).map(Some)
        }
    });
    let text = match read {
        Ok(Some(text)) => text,
        Ok(None) => {
            app.gui.chrome.push_status_toast(
                crate::gui::state::ToastSeverity::Error,
                tr(key::WS_PROFILES_ERR_TOO_LARGE, lang),
            );
            return;
        }
        Err(e) => {
            app.gui.chrome.push_status_toast(
                crate::gui::state::ToastSeverity::Error,
                Msg::new(key::WS_PROFILES_ERR_READ).arg("err", e).text(lang),
            );
            return;
        }
    };
    let ws = match parse_workspace_file(&text) {
        Ok(ws) => ws,
        Err(e) => {
            let message = match &e {
                WorkspaceImportError::TooLarge => {
                    tr(key::WS_PROFILES_ERR_TOO_LARGE, lang).to_string()
                }
                WorkspaceImportError::UnsupportedVersion(version) => {
                    Msg::new(key::WS_PROFILES_ERR_UNSUPPORTED_VERSION)
                        .arg("ver", version)
                        .arg("path", path.display())
                        .text(lang)
                }
                WorkspaceImportError::Invalid(detail) => {
                    Msg::new(key::WS_PROFILES_ERR_INVALID_FILE)
                        .arg("err", detail)
                        .text(lang)
                }
            };
            log::warn!("workspace profile not imported: {e}");
            app.gui
                .chrome
                .push_status_toast(crate::gui::state::ToastSeverity::Error, message);
            return;
        }
    };
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
    app.gui.chrome.push_status_toast(
        crate::gui::state::ToastSeverity::Success,
        Msg::new(key::WS_PROFILES_IMPORTED_TOAST)
            .arg("name", &ws.name)
            .text(lang),
    );
}

/// Zbiera bieżący stan paneli do profilu.
pub(crate) fn capture_profile(app: &SpLogApp, name: &str) -> WorkspaceProfile {
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
        module_panels: app.views.module_panels(),
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
    app.views.set_module_panels(&p.module_panels);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn file_text(profile: &WorkspaceProfile) -> String {
        serde_json::to_string(&WorkspaceFile {
            version: WorkspaceFile::CURRENT_VERSION,
            workspace: profile.clone(),
        })
        .unwrap()
    }

    fn sample() -> WorkspaceProfile {
        workspace_profile_presets().remove(0)
    }

    /// Układ w postaci zapisanej po narysowaniu: nienarysowany ma prostokąty NaN,
    /// których JSON nie przenosi.
    fn drawn_layout(tabs: &[&str]) -> serde_json::Value {
        let state = DockState::new(tabs.iter().map(ToString::to_string).collect());
        let text = serde_json::to_string(&state)
            .unwrap()
            .replace("\"x\":null", "\"x\":0.0")
            .replace("\"y\":null", "\"y\":0.0");
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn imported_profile_is_cleaned_and_keeps_known_modules() {
        let mut profile = sample();
        profile.name = "  Pile-up\n20 m  ".to_string();
        profile.id = "../../evil".to_string();
        profile.module_panels.insert(
            "contest".to_string(),
            crate::core::station::ViewPanelConfig::default(),
        );
        profile.module_panels.insert(
            "future_module".to_string(),
            crate::core::station::ViewPanelConfig::default(),
        );
        profile.module_panels.insert(
            "vfo".to_string(),
            crate::core::station::ViewPanelConfig::default(),
        );
        let ws = parse_workspace_file(&file_text(&profile)).unwrap();
        assert_eq!(ws.name, "Pile-up20 m");
        assert!(!ws.id.contains('/') && !ws.id.contains('.'), "{}", ws.id);
        assert_eq!(
            ws.module_panels
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["contest"]
        );
    }

    #[test]
    fn dock_layout_with_unknown_tiles_is_dropped() {
        let mut profile = sample();
        profile.dock_layout = Some(drawn_layout(&["log", "contest"]));
        let ws = parse_workspace_file(&file_text(&profile)).unwrap();
        assert!(ws.dock_layout.is_some());

        profile.dock_layout = Some(drawn_layout(&["log", "spy_panel"]));
        assert_eq!(
            parse_workspace_file(&file_text(&profile))
                .unwrap()
                .dock_layout,
            None
        );

        profile.dock_layout = Some(serde_json::json!({"not": "a dock"}));
        assert_eq!(
            parse_workspace_file(&file_text(&profile))
                .unwrap()
                .dock_layout,
            None
        );
    }

    #[test]
    fn broken_files_are_refused() {
        assert!(matches!(
            parse_workspace_file("not json"),
            Err(WorkspaceImportError::Invalid(_))
        ));
        let mut text = file_text(&sample());
        text = text.replacen("\"version\":1", "\"version\":9", 1);
        assert_eq!(
            parse_workspace_file(&text),
            Err(WorkspaceImportError::UnsupportedVersion(9))
        );
        let zero = file_text(&sample()).replacen("\"version\":1", "\"version\":0", 1);
        assert_eq!(
            parse_workspace_file(&zero),
            Err(WorkspaceImportError::UnsupportedVersion(0))
        );
        let mut nameless = sample();
        nameless.name = " \t ".to_string();
        assert!(matches!(
            parse_workspace_file(&file_text(&nameless)),
            Err(WorkspaceImportError::Invalid(_))
        ));
        let huge = " ".repeat(MAX_WORKSPACE_FILE_BYTES as usize + 1);
        assert_eq!(
            parse_workspace_file(&huge),
            Err(WorkspaceImportError::TooLarge)
        );
    }

    #[test]
    fn built_in_presets_are_translated() {
        for preset in workspace_profile_presets() {
            for lang in [Language::Pl, Language::En, Language::Ru] {
                let (name, description) = preset_text(&preset, lang);
                assert_ne!(name, "", "{}", preset.id);
                assert_ne!(description, "", "{}", preset.id);
            }
            let (en, _) = preset_text(&preset, Language::En);
            let (pl, _) = preset_text(&preset, Language::Pl);
            if preset.id != "ft8" {
                assert_ne!(en, pl, "{}: angielska nazwa presetu", preset.id);
            }
        }
        let mut own = sample();
        own.id = "my-layout".to_string();
        own.name = "Mój układ".to_string();
        assert_eq!(preset_text(&own, Language::En).0, "Mój układ");
    }
}
