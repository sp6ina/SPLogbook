// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{tr, Language};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::theme::ThemePreset;
use eframe::egui;

/// Menu: Ustawienia (język, motyw, skalowanie czcionki)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.settings", lang), |ui| {
        // Język interfejsu
        ui.menu_button(icons::GLOBE.label(tr("settings.language", lang)), |ui| {
            let langs = [Language::Pl, Language::En, Language::De, Language::Fr, Language::Es, Language::It, Language::Ru];
            for l in langs {
                let flag = match l {
                    Language::Pl => "🇵🇱",
                    Language::En => "🇬🇧",
                    Language::De => "🇩🇪",
                    Language::Fr => "🇫🇷",
                    Language::Es => "🇪🇸",
                    Language::It => "🇮🇹",
                    Language::Ru => "🇷🇺",
                };
                if ui.selectable_label(lang == l, format!("{} {}", flag, l.display_name())).clicked() {
                    app.current_language = l;
                    app.save_station_config();
                    ui.close();
                }
            }
        });

        // Motyw kolorystyczny
        ui.menu_button(icons::THEME.label(tr("theme.menu", lang)), |ui| {
            for preset in ThemePreset::ALL {
                let selected = app.theme_preset == preset;
                if ui.selectable_label(selected, preset.label_pl()).on_hover_text(preset.description_pl()).clicked() {
                    app.theme_preset = preset;
                    app.save_station_config();
                    ui.close();
                }
            }
        });

        ui.separator();

        // Skala czcionki / UI
        ui.label(egui::RichText::new(tr("settings.font_scale", lang)).strong());
        let mut scale = app.font_scale;
        if ui.add(egui::Slider::new(&mut scale, 0.8..=1.5).step_by(0.05)).changed() {
            app.font_scale = scale;
            app.save_station_config();
        }
        ui.label(format!("{:.0}%", app.font_scale * 100.0));

        // Rodzina czcionek (zmiana wymaga ponownego uruchomienia)
        ui.label(egui::RichText::new(tr("settings.font_family", lang)).strong());
        let mut selected_family = app.font_family.clone();
        let families = ["", "Segoe UI", "Arial", "Consolas", "DejaVu Sans", "Noto Sans"];
        let current_label = if selected_family.is_empty() { tr("settings.font_default", lang) } else { selected_family.as_str() };
        egui::ComboBox::from_id_salt("settings_font_family")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for f in families {
                    let label = if f.is_empty() { tr("settings.font_default", lang) } else { f };
                    if ui.selectable_value(&mut selected_family, f.to_string(), label).changed() {
                        app.font_family = selected_family.clone();
                        app.save_station_config();
                    }
                }
            });

        // Jednostka odległości używana w całym interfejsie (DX, azymut, mapa)
        ui.label(egui::RichText::new(tr("settings.distance_unit", lang)).strong());
        let mut selected_unit = app.distance_unit.clone();
        let unit_label = |u: &str| -> &'static str {
            match u {
                "mi" => "Mile (mi)",
                "nmi" => "Mile morskie (NM)",
                _ => "Kilometry (km)",
            }
        };
        egui::ComboBox::from_id_salt("settings_distance_unit")
            .selected_text(unit_label(&selected_unit))
            .show_ui(ui, |ui| {
                for u in ["km", "mi", "nmi"] {
                    if ui.selectable_value(&mut selected_unit, u.to_string(), unit_label(u)).changed() {
                        app.distance_unit = selected_unit.clone();
                        app.save_station_config();
                    }
                }
            });
    });
}
