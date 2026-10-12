// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_find_duplicates_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_find_duplicates_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_find_duplicates_window;
    let mut close_requested = false;
    let mut scan_requested = false;
    let mut delete_requested = false;

    egui::Window::new(
        egui::RichText::new(tr(key::DUPLICATES_TITLE, lang))
            .size(14.0)
            .strong(),
    )
    .open(&mut is_open)
    .default_size([720.0, 520.0])
    .min_size([500.0, 350.0])
    .resizable(true)
    .show(ctx, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr(key::DUPLICATES_CRITERIA, lang)).strong());
                if ui
                    .checkbox(
                        &mut app.duplicates_match_date,
                        tr(key::DUPLICATES_SAME_DAY, lang),
                    )
                    .changed()
                {
                    scan_requested = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            egui::RichText::new(tr(key::DUPLICATES_SCAN_BTN, lang))
                                .color(egui::Color32::from_rgb(56, 189, 248))
                                .strong(),
                        )
                        .clicked()
                    {
                        scan_requested = true;
                    }
                });
            });

            ui.separator();

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(tr(key::DUPLICATES_QUICK_SELECT, lang))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
                if ui.button(tr(key::DUPLICATES_KEEP_OLDEST, lang)).clicked() {
                    app.duplicates_selected_ids.clear();
                    for group in &app.duplicates_groups {
                        for qso in group.iter().skip(1) {
                            if let Some(id) = qso.id {
                                app.duplicates_selected_ids.insert(id);
                            }
                        }
                    }
                }

                if ui.button(tr(key::DUPLICATES_KEEP_NEWEST, lang)).clicked() {
                    app.duplicates_selected_ids.clear();
                    for group in &app.duplicates_groups {
                        let count = group.len();
                        for qso in group.iter().take(count.saturating_sub(1)) {
                            if let Some(id) = qso.id {
                                app.duplicates_selected_ids.insert(id);
                            }
                        }
                    }
                }

                if ui
                    .button(tr(key::DUPLICATES_KEEP_CONFIRMED, lang))
                    .clicked()
                {
                    app.duplicates_selected_ids.clear();
                    for group in &app.duplicates_groups {
                        let has_confirmed = group.iter().any(|q| {
                            q.lotw_qsl_rcvd == "Y"
                                || q.lotw_qsl_rcvd == "V"
                                || q.eqsl_qsl_rcvd == "Y"
                                || q.qsl_rcvd == "Y"
                        });
                        if has_confirmed {
                            for qso in group {
                                let is_conf = qso.lotw_qsl_rcvd == "Y"
                                    || qso.lotw_qsl_rcvd == "V"
                                    || qso.eqsl_qsl_rcvd == "Y"
                                    || qso.qsl_rcvd == "Y";
                                if !is_conf {
                                    if let Some(id) = qso.id {
                                        app.duplicates_selected_ids.insert(id);
                                    }
                                }
                            }
                        }
                    }
                }

                if ui.button(tr(key::DUPLICATES_CLEAR_SEL, lang)).clicked() {
                    app.duplicates_selected_ids.clear();
                }
            });

            if let Some(ref st) = app.duplicates_status {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(st)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(250, 204, 21)),
                );
            }

            ui.separator();

            if app.duplicates_groups.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(egui::RichText::new("✨").size(24.0));
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(340.0)
                    .show(ui, |ui| {
                        for (grp_idx, group) in app.duplicates_groups.iter().enumerate() {
                            if group.is_empty() {
                                continue;
                            }
                            let first = &group[0];
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "#{}: {} | {} | {}",
                                            grp_idx + 1,
                                            first.callsign,
                                            first.band,
                                            first.mode
                                        ))
                                        .strong()
                                        .color(egui::Color32::from_rgb(56, 189, 248)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("({} QSO)", group.len()))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                    );
                                });

                                egui::Grid::new(format!("dup_grid_{grp_idx}"))
                                    .striped(true)
                                    .spacing([10.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("✓").strong().size(11.0));
                                        ui.label(egui::RichText::new("ID").strong().size(11.0));
                                        ui.label(
                                            egui::RichText::new(tr(key::QSO_DATE, lang))
                                                .strong()
                                                .size(11.0),
                                        );
                                        ui.label(
                                            egui::RichText::new(tr(key::QSO_TIME, lang))
                                                .strong()
                                                .size(11.0),
                                        );
                                        ui.label(
                                            egui::RichText::new("RST S/R").strong().size(11.0),
                                        );
                                        ui.label(egui::RichText::new("QSL").strong().size(11.0));
                                        ui.label(
                                            egui::RichText::new(tr(key::QSO_COMMENT, lang))
                                                .strong()
                                                .size(11.0),
                                        );
                                        ui.end_row();

                                        for qso in group {
                                            if let Some(id) = qso.id {
                                                let mut checked =
                                                    app.duplicates_selected_ids.contains(&id);
                                                if ui.checkbox(&mut checked, "").changed() {
                                                    if checked {
                                                        app.duplicates_selected_ids.insert(id);
                                                    } else {
                                                        app.duplicates_selected_ids.remove(&id);
                                                    }
                                                }
                                                ui.label(format!("#{id}"));
                                                ui.label(&qso.qso_date);
                                                ui.label(&qso.time_on);
                                                ui.label(format!(
                                                    "{}/{}",
                                                    qso.rst_sent, qso.rst_rcvd
                                                ));

                                                let qsl_info = format!(
                                                    "LoTW:{} eQSL:{} QSL:{}",
                                                    qso.lotw_qsl_rcvd,
                                                    qso.eqsl_qsl_rcvd,
                                                    qso.qsl_rcvd
                                                );
                                                let qsl_color = if qso.lotw_qsl_rcvd == "Y"
                                                    || qso.eqsl_qsl_rcvd == "Y"
                                                    || qso.qsl_rcvd == "Y"
                                                {
                                                    egui::Color32::from_rgb(34, 197, 94)
                                                } else {
                                                    egui::Color32::from_rgb(148, 163, 184)
                                                };
                                                ui.label(
                                                    egui::RichText::new(qsl_info)
                                                        .color(qsl_color)
                                                        .size(11.0),
                                                );
                                                ui.label(
                                                    egui::RichText::new(
                                                        qso.comment.as_deref().unwrap_or(""),
                                                    )
                                                    .size(11.0),
                                                );
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                            ui.add_space(2.0);
                        }
                    });
            }

            ui.separator();

            ui.horizontal(|ui| {
                let sel_count = app.duplicates_selected_ids.len();
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}",
                        tr(key::DUPLICATES_SELECTED_COUNT, lang),
                        sel_count
                    ))
                    .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_requested = true;
                    }

                    if sel_count > 0 {
                        let btn_text =
                            format!("{} ({})", tr(key::DUPLICATES_DELETE_BTN, lang), sel_count);
                        if ui
                            .button(
                                egui::RichText::new(btn_text)
                                    .color(egui::Color32::from_rgb(239, 68, 68))
                                    .strong(),
                            )
                            .clicked()
                        {
                            delete_requested = true;
                        }
                    }
                });
            });
        });
    });

    if close_requested {
        is_open = false;
    }
    app.show_find_duplicates_window = is_open;

    if scan_requested {
        let db = app
            .log_db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match db.find_duplicate_qsos(app.duplicates_match_date) {
            Ok(groups) => {
                let total_dups: usize = groups.iter().map(std::vec::Vec::len).sum();
                app.duplicates_status = Some(format!(
                    "Found {} duplicate groups (total {} records)",
                    groups.len(),
                    total_dups
                ));
                app.duplicates_groups = groups;
                app.duplicates_selected_ids.clear();
            }
            Err(e) => {
                app.duplicates_status = Some(e.to_string());
            }
        }
    }

    if delete_requested {
        let ids: Vec<i64> = app.duplicates_selected_ids.iter().copied().collect();
        let db = app
            .log_db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match db.delete_multiple_qsos(&ids) {
            Ok(deleted) => {
                app.duplicates_status = Some(format!("Deleted {deleted} duplicates"));
                app.duplicates_selected_ids.clear();
                if let Ok(groups) = db.find_duplicate_qsos(app.duplicates_match_date) {
                    app.duplicates_groups = groups;
                }
                drop(db);
                app.reload_qsos();
            }
            Err(e) => {
                app.duplicates_status = Some(e.to_string());
            }
        }
    }
}
