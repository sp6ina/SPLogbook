// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno zarządzania profilami układu operatorskiego (workspace).
//! Umożliwia zapisywanie i przywracanie widoczności / pozycji / rozmiarów
//! paneli głównego okna oraz wybór wbudowanych presetów.

use crate::core::station::{workspace_profile_presets, WorkspaceProfile};
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Rysuje okno profili układu, jeśli jest otwarte.
pub fn render_workspace_profiles_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_workspace_profiles_window {
        return;
    }

    let mut open = app.show_workspace_profiles_window;
    egui::Window::new("🖥 Profile układu operatorskiego")
        .open(&mut open)
        .default_size([560.0, 440.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading(egui::RichText::new("🖥 Profile układu (workspace)").size(15.0).color(egui::Color32::from_rgb(56, 189, 248)));
            ui.separator();
            ui.label(egui::RichText::new("Zapisuj i przywracaj układ paneli (widoczność, kolumna, kolejność, pozycja okien pływających). Przełączaj się jednym kliknięciem między konfiguracjami do logowania, zawodów, pracy cyfrowej itp.").small().color(egui::Color32::GRAY));
            ui.add_space(6.0);

            // Sekcja: wbudowane presety
            ui.label(egui::RichText::new("⭐ Wbudowane presety").strong().color(egui::Color32::from_rgb(250, 204, 21)));
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                let mut apply_preset: Option<usize> = None;
                for (i, preset) in workspace_profile_presets().iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("• {}", preset.name));
                        if ui.button("Zastosuj").clicked() {
                            apply_preset = Some(i);
                        }
                    });
                }
                if let Some(i) = apply_preset {
                    let preset = workspace_profile_presets().into_iter().nth(i).unwrap();
                    apply_profile(app, &preset);
                    app.status_toast = Some((format!("Zastosowano preset: {}", preset.name), std::time::Instant::now()));
                }
            });
            ui.separator();

            // Sekcja: własne profile
            ui.label(egui::RichText::new("💾 Własne profile").strong().color(egui::Color32::from_rgb(134, 239, 172)));

            // Pole nazwy nowego profilu (przechowywane w pamięci egui)
            let name_id = egui::Id::new("workspace_profile_name_input");
            let mut new_name = ctx.data_mut(|d| d.get_temp::<String>(name_id)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label("Nazwa:");
                ui.add(egui::TextEdit::singleline(&mut new_name).hint_text("np. DX-pileup 20m").desired_width(220.0));
                let can_save = !new_name.trim().is_empty();
                if ui.add_enabled(can_save, egui::Button::new("💾 Zapisz bieżący układ")).clicked() {
                    let profile = capture_profile(app, new_name.trim());
                    // Nadpisanie istniejącego profilu o tej samej nazwie
                    if let Some(existing) = app.workspace_profiles.iter_mut().find(|p| p.name == profile.name) {
                        *existing = profile.clone();
                    } else {
                        app.workspace_profiles.push(profile.clone());
                    }
                    app.save_station_config();
                    app.status_toast = Some((format!("Zapisano profil: {}", profile.name), std::time::Instant::now()));
                    new_name.clear();
                }
            });
            ctx.data_mut(|d| d.insert_temp(name_id, new_name.clone()));

            ui.add_space(6.0);
            if app.workspace_profiles.is_empty() {
                ui.label(egui::RichText::new("Brak zapisanych profili. Ustaw układ paneli i kliknij „Zapisz bieżący układ”.").italics().color(egui::Color32::from_rgb(148, 163, 184)));
            } else {
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    let mut apply_idx: Option<usize> = None;
                    let mut delete_idx: Option<usize> = None;
                    for (idx, profile) in app.workspace_profiles.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", profile.name));
                            if ui.button("Zastosuj").clicked() {
                                apply_idx = Some(idx);
                            }
                            if ui.button("🗑").on_hover_text("Usuń profil").clicked() {
                                delete_idx = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = apply_idx {
                        let p = app.workspace_profiles[idx].clone();
                        apply_profile(app, &p);
                        app.status_toast = Some((format!("Zastosowano profil: {}", p.name), std::time::Instant::now()));
                    }
                    if let Some(idx) = delete_idx {
                        app.workspace_profiles.remove(idx);
                        app.save_station_config();
                    }
                });
            }
        });
    app.show_workspace_profiles_window = open;
}

/// Zbiera bieżący stan paneli do profilu.
fn capture_profile(app: &SpLogApp, name: &str) -> WorkspaceProfile {
    WorkspaceProfile {
        name: name.to_string(),
        panel_vfo: app.panel_vfo.clone(),
        panel_qso: app.panel_qso.clone(),
        panel_log: app.panel_log.clone(),
        panel_cluster: app.panel_cluster.clone(),
        panel_solar: app.panel_solar.clone(),
        panel_bandmap: app.panel_bandmap.clone(),
        panel_satellites: app.panel_satellites.clone(),
        panel_world_map: app.panel_world_map.clone(),
        theme_preset: None,
    }
}

/// Stosuje profil do bieżącego układu.
fn apply_profile(app: &mut SpLogApp, p: &WorkspaceProfile) {
    app.panel_vfo = p.panel_vfo.clone();
    app.panel_qso = p.panel_qso.clone();
    app.panel_log = p.panel_log.clone();
    app.panel_cluster = p.panel_cluster.clone();
    app.panel_solar = p.panel_solar.clone();
    app.panel_bandmap = p.panel_bandmap.clone();
    app.panel_satellites = p.panel_satellites.clone();
    app.panel_world_map = p.panel_world_map.clone();

    // Synchronizacja okien pływających (popout) dla paneli z obsługą ↗.
    app.show_bandmap_window = p.panel_bandmap.visible && p.panel_bandmap.floating;
    app.show_satellites_window = p.panel_satellites.visible && p.panel_satellites.floating;
    app.show_world_map_window = p.panel_world_map.visible && p.panel_world_map.floating;

    // Wymuś ponowny rozkład kolumn dokujących.
    app.reset_layout_requested = true;
    app.save_station_config();
}
