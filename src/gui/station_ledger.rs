// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Language, key, tr};
use crate::core::station::{EquipmentCategory, EquipmentItem};
use crate::gui::app::SpLogApp;
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
    if !app.show_ledger_window {
        return;
    }

    let lang = app.current_language;
    let mut open = app.show_ledger_window;
    let mut close_req = false;
    let mut item_to_delete: Option<String> = None;
    let mut item_to_edit: Option<EquipmentItem> = None;

    egui::Window::new(format!("📋 {}", tr(key::LEDGER_TITLE, lang)))
        .open(&mut open)
        .default_size([760.0, 520.0])
        .min_size([650.0, 400.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::LEDGER_PROFILE_HEADING, lang))
                            .strong()
                            .color(egui::Color32::from_rgb(56, 189, 248)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(key::LEDGER_SAVE_PROFILE, lang)).clicked() {
                            app.save_station_config();
                            app.status_message =
                                Some(tr(key::LEDGER_PROFILE_SAVED, lang).to_string());
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(format!("{}:", tr(key::QSO_CALLSIGN, lang)));
                    let call_resp = ui.add(
                        egui::TextEdit::singleline(&mut app.my_station.callsign)
                            .desired_width(80.0),
                    );
                    if call_resp.changed() {
                        app.my_station.callsign = app.my_station.callsign.to_uppercase();
                    }

                    ui.label(format!("{}:", tr(key::STATION_OPERATOR, lang)));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.my_station.operator)
                            .desired_width(120.0),
                    );

                    ui.label(format!("{}:", tr(key::GEO_LOCATOR, lang)));
                    let loc_resp = ui.add(
                        egui::TextEdit::singleline(&mut app.my_station.gridsquare)
                            .desired_width(65.0),
                    );
                    if loc_resp.changed() {
                        app.my_station.gridsquare = app.my_station.gridsquare.to_uppercase();
                    }

                    ui.label("QTH:");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.my_station.city).desired_width(100.0),
                    );
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::LEDGER_REGISTERED_EQUIPMENT, lang))
                            .strong()
                            .color(egui::Color32::from_rgb(251, 191, 36)),
                    );
                    ui.label(format!(
                        "({} {})",
                        tr(key::LEDGER_TOTAL_ITEMS, lang),
                        app.equipment_items.len()
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

                                for item in &app.equipment_items {
                                    let cat_name = category_label(&item.category, lang);
                                    ui.label(cat_name);
                                    ui.label(&item.manufacturer);
                                    ui.label(
                                        egui::RichText::new(&item.model)
                                            .strong()
                                            .color(egui::Color32::from_rgb(56, 189, 248)),
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
                                            .button(
                                                egui::RichText::new("🗑")
                                                    .color(egui::Color32::from_rgb(239, 68, 68)),
                                            )
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
                if let Some(ref edit_id) = app.editing_equipment_id.clone() {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::LEDGER_EDIT_ITEM, lang))
                                .strong()
                                .color(egui::Color32::from_rgb(250, 204, 21)),
                        );

                        egui::Grid::new("edit_form_grid")
                            .num_columns(2)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(format!("{}:", tr(key::LEDGER_COL_CATEGORY, lang)));
                                egui::ComboBox::from_id_salt("edit_eq_cat")
                                    .selected_text(category_label(&app.edit_eq_cat, lang))
                                    .show_ui(ui, |ui| {
                                        for cat in EquipmentCategory::all() {
                                            ui.selectable_value(
                                                &mut app.edit_eq_cat,
                                                *cat,
                                                category_label(cat, lang),
                                            );
                                        }
                                    });
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MANUFACTURER, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.edit_eq_mfr)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MODEL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.edit_eq_model)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_SERIAL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.edit_eq_sn)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_PURCHASE_DATE, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.edit_eq_date)
                                        .desired_width(220.0),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_NOTES, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.edit_eq_notes)
                                        .desired_width(320.0),
                                );
                                ui.end_row();
                            });

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new(format!("💾 {}", tr(key::BTN_SAVE, lang)))
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .clicked()
                            {
                                app.save_edited_equipment(edit_id);
                            }
                            if ui.button(tr(key::BTN_CANCEL, lang)).clicked() {
                                app.editing_equipment_id = None;
                            }
                        });
                    });
                } else {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::LEDGER_ADD_NEW_TITLE, lang))
                                .strong()
                                .color(egui::Color32::from_rgb(34, 197, 94)),
                        );

                        egui::Grid::new("add_form_grid")
                            .num_columns(4)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(format!("{}:", tr(key::LEDGER_COL_CATEGORY, lang)));
                                egui::ComboBox::from_id_salt("new_eq_cat")
                                    .selected_text(category_label(&app.new_eq_cat, lang))
                                    .show_ui(ui, |ui| {
                                        for cat in EquipmentCategory::all() {
                                            ui.selectable_value(
                                                &mut app.new_eq_cat,
                                                *cat,
                                                category_label(cat, lang),
                                            );
                                        }
                                    });

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MANUFACTURER, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.new_eq_mfr)
                                        .hint_text("np. Icom, Yaesu, Kenwood"),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_MODEL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.new_eq_model)
                                        .hint_text("np. IC-7610, FT-891"),
                                );

                                ui.label(format!("{}:", tr(key::LEDGER_COL_SERIAL, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.new_eq_sn)
                                        .hint_text("opcjonalnie"),
                                );
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::LEDGER_COL_NOTES, lang)));
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.new_eq_notes)
                                        .hint_text("np. moc 100W, pasma KF/6m"),
                                );
                                ui.end_row();
                            });

                        ui.add_space(4.0);
                        if ui
                            .button(
                                egui::RichText::new(tr(key::LEDGER_ADD_BUTTON, lang))
                                    .strong()
                                    .color(egui::Color32::from_rgb(34, 197, 94)),
                            )
                            .clicked()
                            && !app.new_eq_model.trim().is_empty()
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
    app.show_ledger_window = open;

    if let Some(id) = item_to_delete {
        app.delete_equipment_item(&id);
    }

    if let Some(item) = item_to_edit {
        app.editing_equipment_id = Some(item.id.clone());
        app.edit_eq_cat = item.category;
        app.edit_eq_mfr = item.manufacturer;
        app.edit_eq_model = item.model;
        app.edit_eq_sn = item.serial_number.unwrap_or_default();
        app.edit_eq_date = item.purchase_date.unwrap_or_default();
        app.edit_eq_notes = item.notes.unwrap_or_default();
    }
}
