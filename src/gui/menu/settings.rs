// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Language, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::theme::ThemePreset;
use eframe::egui;

/// Menu: Ustawienia (język, motyw, skalowanie czcionki)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_SETTINGS, lang), |ui| {
        // Język interfejsu
        ui.menu_button(icons::GLOBE.label(tr(key::SETTINGS_LANGUAGE, lang)), |ui| {
            let langs = [
                Language::Pl,
                Language::En,
                Language::De,
                Language::Fr,
                Language::Es,
                Language::It,
                Language::Ru,
            ];
            for language in langs {
                let flag = match language {
                    Language::Pl => "🇵🇱",
                    Language::En => "🇬🇧",
                    Language::De => "🇩🇪",
                    Language::Fr => "🇫🇷",
                    Language::Es => "🇪🇸",
                    Language::It => "🇮🇹",
                    Language::Ru => "🇷🇺",
                };
                if ui
                    .selectable_label(
                        lang == language,
                        format!("{} {}", flag, language.display_name()),
                    )
                    .clicked()
                {
                    app.gui.appearance.language = language;
                    app.save_station_config();
                    ui.close();
                }
            }
        });

        // Motyw kolorystyczny
        ui.menu_button(icons::THEME.label(tr(key::THEME_MENU, lang)), |ui| {
            for preset in ThemePreset::ALL {
                let selected = app.gui.appearance.theme_preset == preset;
                if ui
                    .selectable_label(selected, preset.label(lang))
                    .on_hover_text(preset.description(lang))
                    .clicked()
                {
                    app.gui.appearance.theme_preset = preset;
                    app.save_station_config();
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button(icons::THEME.label(tr(key::THEME_CUSTOM_MENU, lang)))
                .clicked()
            {
                use crate::gui::theme::ThemeExt;
                let accent = ui.palette().accent;
                app.open_theme_customizer([accent.r(), accent.g(), accent.b()]);
                ui.close();
            }
        });

        backup_settings_menu(app, ui);

        if ui
            .button(icons::SETTINGS.label(tr(key::STATUSBAR_CUSTOMIZE_MENU, lang)))
            .clicked()
        {
            app.gui.windows.show_status_bar_settings = true;
            ui.close();
        }

        if ui
            .button(icons::WIZARD.label(tr(key::WIZARD_SETUP_STATION, lang)))
            .clicked()
        {
            app.gui.chrome.show_welcome_wizard = true;
            app.gui.chrome.wizard_tab = 0;
            ui.close();
        }

        if ui
            .button(icons::BANDMAP.label(tr(key::BANDPLAN_MENU, lang)))
            .clicked()
        {
            app.open_bandplan_editor();
            ui.close();
        }

        ui.separator();

        // Skala czcionki / UI
        ui.label(egui::RichText::new(tr(key::SETTINGS_FONT_SCALE, lang)).strong());
        let mut scale = app.config.font_scale;
        if ui
            .add(egui::Slider::new(&mut scale, 0.8..=1.5).step_by(0.05))
            .changed()
        {
            app.config.font_scale = scale;
            app.save_station_config();
        }
        ui.label(format!("{:.0}%", app.config.font_scale * 100.0));

        // Rodzina czcionek (zmiana wymaga ponownego uruchomienia)
        ui.label(egui::RichText::new(tr(key::SETTINGS_FONT_FAMILY, lang)).strong());
        let mut selected_family = app.config.font_family.clone();
        let families = [
            "",
            "Segoe UI",
            "Arial",
            "Consolas",
            "DejaVu Sans",
            "Noto Sans",
        ];
        let current_label = if selected_family.is_empty() {
            tr(key::SETTINGS_FONT_DEFAULT, lang)
        } else {
            selected_family.as_str()
        };
        egui::ComboBox::from_id_salt("settings_font_family")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for family in families {
                    let label = if family.is_empty() {
                        tr(key::SETTINGS_FONT_DEFAULT, lang)
                    } else {
                        family
                    };
                    if ui
                        .selectable_value(&mut selected_family, family.to_string(), label)
                        .changed()
                    {
                        app.config.font_family.clone_from(&selected_family);
                        app.save_station_config();
                    }
                }
            });

        // Jednostka odległości używana w całym interfejsie (DX, azymut, mapa)
        ui.label(egui::RichText::new(tr(key::SETTINGS_DISTANCE_UNIT, lang)).strong());
        let mut selected_unit = app.config.distance_unit.clone();
        let unit_label = |u: &str| -> &str {
            match u {
                "mi" => tr(key::SETTINGS_UNIT_MI, lang),
                "nmi" => tr(key::SETTINGS_UNIT_NMI, lang),
                _ => tr(key::SETTINGS_UNIT_KM, lang),
            }
        };
        egui::ComboBox::from_id_salt("settings_distance_unit")
            .selected_text(unit_label(&selected_unit))
            .show_ui(ui, |ui| {
                for unit in ["km", "mi", "nmi"] {
                    if ui
                        .selectable_value(&mut selected_unit, unit.to_string(), unit_label(unit))
                        .changed()
                    {
                        app.config.distance_unit.clone_from(&selected_unit);
                        app.save_station_config();
                    }
                }
            });
    });
}

/// Podmenu kopii zapasowych: odstęp kopii okresowych, retencja, kopia przy zamknięciu, ADIF.
fn backup_settings_menu(app: &mut SpLogApp, ui: &mut egui::Ui) {
    use crate::core::station::BackupSettings;
    let lang = app.gui.appearance.language;
    ui.menu_button(tr(key::SETTINGS_BACKUP_MENU, lang), |ui| {
        let before = app.config.backup.clone();
        let backup = &mut app.config.backup;

        ui.label(egui::RichText::new(tr(key::SETTINGS_BACKUP_INTERVAL, lang)).strong());
        ui.add(
            egui::DragValue::new(&mut backup.interval_minutes)
                .range(0..=BackupSettings::MAX_INTERVAL_MINUTES)
                .suffix(" min"),
        )
        .on_hover_text(tr(key::SETTINGS_BACKUP_INTERVAL_HINT, lang));

        ui.label(egui::RichText::new(tr(key::SETTINGS_BACKUP_KEEP, lang)).strong());
        ui.add(egui::DragValue::new(&mut backup.keep).range(1..=BackupSettings::MAX_KEEP));

        ui.checkbox(&mut backup.on_exit, tr(key::SETTINGS_BACKUP_ON_EXIT, lang));
        ui.checkbox(
            &mut backup.with_adif,
            tr(key::SETTINGS_BACKUP_WITH_ADIF, lang),
        );

        if app.config.backup != before {
            app.save_station_config();
        }
    });
}
