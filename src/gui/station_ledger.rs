// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Language, key, tr};
use crate::core::station::{EquipmentCategory, EquipmentItem};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

fn category_label(cat: &EquipmentCategory, lang: Language) -> &str {
    match cat {
        EquipmentCategory::Transceiver => tr(key::LEDGER_CAT_TRANSCEIVER, lang),
        EquipmentCategory::Antenna => tr(key::LEDGER_CAT_ANTENNA, lang),
        EquipmentCategory::Amplifier => tr(key::LEDGER_CAT_AMPLIFIER, lang),
        EquipmentCategory::Tuner => tr(key::LEDGER_CAT_TUNER, lang),
        EquipmentCategory::PowerSupply => tr(key::LEDGER_CAT_POWERSUPPLY, lang),
        EquipmentCategory::Keyer => tr(key::LEDGER_CAT_KEYER, lang),
        EquipmentCategory::Other => tr(key::LEDGER_CAT_OTHER, lang),
    }
}

pub fn render_station_ledger_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.ledger.show_window {
        return;
    }

    let lang = app.gui.appearance.language;
    let mut open = app.ledger.show_window;
    let mut close_req = false;
    let mut item_to_delete: Option<String> = None;
    let mut item_to_edit: Option<EquipmentItem> = None;

    egui::Window::new(format!("📋 {}", tr(key::LEDGER_TITLE, lang)))
        .open(&mut open)
        .default_size([760.0, 520.0])
        .min_size([650.0, 400.0])
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::LEDGER_PROFILE_HEADING, lang))
                            .strong()
                            .color(palette.accent),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(key::LEDGER_SAVE_PROFILE, lang)).clicked() {
                            app.save_station_config();
                            app.gui.chrome.status_message =
                                Some(tr(key::LEDGER_PROFILE_SAVED, lang).to_string());
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(format!("{}:", tr(key::QSO_CALLSIGN, lang)));
                    let call_resp = ui.add(
                        egui::TextEdit::singleline(&mut app.config.station.callsign)
                            .desired_width(80.0),
                    );
                    if call_resp.changed() {
                        app.config.station.callsign = app.config.station.callsign.to_uppercase();
                    }

                    ui.label(format!("{}:", tr(key::STATION_OPERATOR, lang)));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.config.station.operator)
                            .desired_width(120.0),
                    );

                    ui.label(format!("{}:", tr(key::GEO_LOCATOR, lang)));
                    let loc_resp = ui.add(
                        egui::TextEdit::singleline(&mut app.config.station.gridsquare)
                            .desired_width(65.0),
                    );
                    if loc_resp.changed() {
                        app.config.station.gridsquare =
                            app.config.station.gridsquare.to_uppercase();
                    }

                    ui.label("QTH:");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.config.station.city)
                            .desired_width(100.0),
                    );
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::LEDGER_REGISTERED_EQUIPMENT, lang))
                            .strong()
                            .color(palette.warning),
                    );
                    ui.label(format!(
                        "({} {})",
                        tr(key::LEDGER_TOTAL_ITEMS, lang),
                        app.config.equipment.len()
                    ));
                });

                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        egui::Grid::new("equipment_crud_grid")
                            .striped(true)
                            .spacing([8.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_CATEGORY, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_MANUFACTURER, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_MODEL, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_SERIAL, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_PURCHASE_DATE, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_NOTES, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::LEDGER_COL_ACTIONS, lang)).strong(),
                                );
                                ui.end_row();

                                for item in &app.config.equipment {
                                    let cat_name = category_label(&item.category, lang);
                                    ui.label(cat_name);
                                    ui.label(&item.manufacturer);
                                    ui.label(
                                        egui::RichText::new(&item.model)
                                            .strong()
                                            .color(palette.accent),
                                    );
                                    ui.label(item.serial_number.as_deref().unwrap_or("-"));
                                    ui.label(item.purchase_date.as_deref().unwrap_or("-"));
                                    ui.label(item.notes.as_deref().unwrap_or(""));

                                    ui.horizontal(|ui| {
                                        if ui
                                            .button(format!("✏ {}", tr(key::BTN_EDIT, lang)))
                                            .clicked()
                                        {
                                            item_to_edit = Some(item.clone());
                                        }
                                        if ui
                                            .button(egui::RichText::new("🗑").color(palette.danger))
                                            .clicked()
                                        {
                                            item_to_delete = Some(item.id.clone());
                                        }
                                    });
                                    ui.end_row();
                                }
                            });
                    });

                ui.separator();

                // Formularz dodawania lub edycji sprzętu
                if let Some(ref edit_id) = app.ledger.editing_id.clone() {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::LEDGER_EDIT_ITEM, lang))
                                .strong()
                                .color(palette.warning),
                        );

                        egui::Grid::new("edit_form_grid")
                            .num_columns(2)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(format!("{}:", tr(key::LEDGER_COL_CATEGORY, lang)));
                                egui::ComboBox::from_id_salt("edit_eq_cat")
                                    .selected_text(category_label(&app.ledger.edit_cat, lang))
                                    .show_ui(ui, |ui| {
                                        for cat in EquipmentCategory::all() {
                                            ui.selectable_value(
                                                &mut app.ledger.edit_cat,
                                                *cat,
                                                category_label(cat, lang),
                                            );
                                        }
                                    });
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MANUFACTURER, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.edit_mfr)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MODEL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.edit_model)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_SERIAL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.edit_sn)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_PURCHASE_DATE, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.edit_date)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_NOTES, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.edit_notes)
                                        .desired_width(320.0),
                                );
                                ui.end_row();
                            });

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new(format!("💾 {}", tr(key::BTN_SAVE, lang)))
                                        .strong()
                                        .color(palette.text_primary),
                                )
                                .clicked()
                            {
                                app.save_edited_equipment(edit_id);
                            }
                            if ui.button(tr(key::BTN_CANCEL, lang)).clicked() {
                                app.ledger.editing_id = None;
                            }
                        });
                    });
                } else {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::LEDGER_ADD_NEW_TITLE, lang))
                                .strong()
                                .color(palette.success),
                        );

                        egui::Grid::new("add_form_grid")
                            .num_columns(4)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(format!("{}:", tr(key::LEDGER_COL_CATEGORY, lang)));
                                egui::ComboBox::from_id_salt("new_eq_cat")
                                    .selected_text(category_label(&app.ledger.new_cat, lang))
                                    .show_ui(ui, |ui| {
                                        for cat in EquipmentCategory::all() {
                                            ui.selectable_value(
                                                &mut app.ledger.new_cat,
                                                *cat,
                                                category_label(cat, lang),
                                            );
                                        }
                                    });

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MANUFACTURER, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.new_mfr)
                                        .hint_text(tr(key::LEDGER_HINT_MAKE, lang)),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MODEL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.new_model)
                                        .hint_text(tr(key::LEDGER_HINT_MODEL, lang)),
                                );

                                ui.label(format!("{}:", tr(key::LEDGER_COL_SERIAL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.new_sn)
                                        .hint_text(tr(key::LEDGER_HINT_OPTIONAL, lang)),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_NOTES, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.ledger.new_notes)
                                        .hint_text(tr(key::LEDGER_HINT_NOTES, lang)),
                                );
                                ui.end_row();
                            });

                        ui.add_space(4.0);
                        if ui
                            .button(
                                egui::RichText::new(tr(key::LEDGER_ADD_BUTTON, lang))
                                    .strong()
                                    .color(palette.success),
                            )
                            .clicked()
                            && !app.ledger.new_model.trim().is_empty()
                        {
                            app.add_new_equipment();
                        }
                    });
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });

    if close_req {
        open = false;
    }
    app.ledger.show_window = open;

    if let Some(id) = item_to_delete {
        app.delete_equipment_item(&id);
    }

    if let Some(item) = item_to_edit {
        app.ledger.editing_id = Some(item.id.clone());
        app.ledger.edit_cat = item.category;
        app.ledger.edit_mfr = item.manufacturer;
        app.ledger.edit_model = item.model;
        app.ledger.edit_sn = item.serial_number.unwrap_or_default();
        app.ledger.edit_date = item.purchase_date.unwrap_or_default();
        app.ledger.edit_notes = item.notes.unwrap_or_default();
    }
}
