// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::contest_rules::{RULES, calculate_score, detect_duplicate};
use crate::core::contest_stats::{
    MultKind, MultMatrix, RateStats, compute_mult_matrix, compute_rate,
};
use crate::core::exchange::{
    ExchangeField, apply_to_qso, exchange_summary, fields_from_format_string, parse_exchange,
};
use crate::core::i18n::tr;
use crate::core::qso::QsoRecord;
use crate::core::station::CustomContest;
use crate::gui::app::SpLogApp;

use eframe::egui;
use std::collections::HashSet;

pub fn render_custom_contest_editor(app: &mut SpLogApp, ctx: &egui::Context) {
    let mut open = app.show_custom_contest_editor;
    if !open {
        return;
    }
    let lang = app.current_language;

    egui::Window::new(tr("contest.custom_editor_title", lang))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(tr("contest.your_contests", lang));
                    let mut to_delete = None;
                    for (i, c) in app.custom_contests.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&c.name);
                            if ui.button(tr("btn.edit", lang)).clicked() {
                                app.custom_contest_edit_idx = Some(i);
                                app.custom_contest_draft = c.clone();
                            }
                            if ui.button(tr("btn.delete", lang)).clicked() {
                                to_delete = Some(i);
                            }
                        });
                    }
                    if let Some(i) = to_delete {
                        app.custom_contests.remove(i);
                        if app.custom_contest_edit_idx == Some(i) {
                            app.custom_contest_edit_idx = None;
                            app.custom_contest_draft = CustomContest::default();
                        }
                    }
                    if ui.button(tr("contest.new_custom_contest", lang)).clicked() {
                        app.custom_contest_edit_idx = None;
                        app.custom_contest_draft = CustomContest::default();
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.heading(if app.custom_contest_edit_idx.is_some() {
                        tr("contest.editing", lang)
                    } else {
                        tr("contest.new", lang)
                    });

                    ui.horizontal(|ui| {
                        ui.label(tr("contest.name_label", lang));
                        ui.text_edit_singleline(&mut app.custom_contest_draft.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("contest.exchange_label", lang));
                        ui.text_edit_singleline(&mut app.custom_contest_draft.exchange_format);
                    });

                    ui.label(tr("contest.points_per_qso_label", lang));
                    ui.add(egui::Slider::new(
                        &mut app.custom_contest_draft.points_per_qso,
                        1..=20,
                    ));

                    ui.label(tr("contest.bands_label", lang));
                    let all_bands = [
                        "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m",
                        "70cm",
                    ];
                    ui.horizontal_wrapped(|ui| {
                        for b in all_bands {
                            let mut has_band =
                                app.custom_contest_draft.bands.contains(&b.to_string());
                            if ui.checkbox(&mut has_band, b).changed() {
                                if has_band {
                                    app.custom_contest_draft.bands.push(b.to_string());
                                } else {
                                    app.custom_contest_draft.bands.retain(|x| x != b);
                                }
                            }
                        }
                    });

                    ui.label(tr("contest.description_label", lang));
                    ui.text_edit_multiline(&mut app.custom_contest_draft.description);

                    if ui.button(tr("btn.save", lang)).clicked()
                        && !app.custom_contest_draft.name.is_empty()
                    {
                        if let Some(i) = app.custom_contest_edit_idx {
                            app.custom_contests[i] = app.custom_contest_draft.clone();
                        } else {
                            app.custom_contests.push(app.custom_contest_draft.clone());
                        }
                        app.custom_contest_edit_idx = None;
                        app.custom_contest_draft = CustomContest::default();
                    }
                });
            });
        });

    app.show_custom_contest_editor = open;
}

pub fn render_contest_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_contest_window {
        return;
    }
    let lang = app.current_language;

    let mut open = app.show_contest_window;
    let mut close_req = false;
    let mut export_clicked = false;
    let mut reset_clicked = false;
    let mut save_clicked = false;
    let mut spot_fill_target: Option<(String, f64, String, bool)> = None;

    // Is it a custom contest?
    let mut is_custom = false;
    let mut custom_idx = None;
    if let Some(idx) = app
        .custom_contests
        .iter()
        .position(|c| c.name == app.contest_name)
    {
        is_custom = true;
        custom_idx = Some(idx);
    }

    // Uporządkowane pola wymiany dla aktualnie wybranego kontestu.
    let exchange_fields: Option<Vec<ExchangeField>> = if let Some(idx) = custom_idx {
        Some(fields_from_format_string(
            &app.custom_contests[idx].exchange_format,
        ))
    } else {
        RULES
            .iter()
            .position(|r| r.name == app.contest_name)
            .map(|rule_idx| RULES[rule_idx].exchange_fields.to_vec())
    };
    let exchange_hint = exchange_fields
        .as_ref()
        .map(|f| {
            f.iter()
                .map(|e| e.label().to_string())
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .unwrap_or_default();

    let my_dxcc: u32 = 269;
    let my_cqzone = app.my_station.cq_zone as u8;

    let mut unknown_contest_rule = false;
    let (pts, mults, total) = if let Some(idx) = custom_idx {
        let c = &app.custom_contests[idx];
        let mut p = 0;
        let mut m = HashSet::new();
        for q in &app.recent_qsos {
            p += c.points_per_qso;
            if let Some(d) = q.dxcc {
                m.insert(d.to_string());
            } else if let Some(z) = q.cqz {
                m.insert(z.to_string());
            } else if !q.rst_rcvd.is_empty() {
                m.insert(q.rst_rcvd.clone());
            }
        }
        let mult_count = m.len() as u32;
        (p, mult_count, p * mult_count)
    } else if let Some(rule_idx) = RULES.iter().position(|r| r.name == app.contest_name) {
        let active_rule = &RULES[rule_idx];
        calculate_score(active_rule, &app.recent_qsos, my_dxcc, my_cqzone)
    } else {
        // Nieznana/nieistniejąca nazwa kontestu (np. reguła usunięta w nowszej wersji) -
        // NIE zgadujemy wyniku wg pierwszej reguły z listy, tylko jawnie sygnalizujemy błąd.
        unknown_contest_rule = true;
        (0, 0, 0)
    };

    app.contest_points = pts;
    app.contest_mults = mults;
    app.contest_qsos = app.recent_qsos.len() as u32;

    // Rodzaj mnożnika i pasma dla macierzy mnożników.
    let (mult_kind, rule_bands): (MultKind, Vec<String>) = if let Some(idx) = custom_idx {
        (MultKind::Dxcc, app.custom_contests[idx].bands.clone())
    } else if let Some(rule_idx) = RULES.iter().position(|r| r.name == app.contest_name) {
        (
            RULES[rule_idx].mult_kind,
            RULES[rule_idx]
                .bands
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
        )
    } else {
        (MultKind::None, vec![])
    };

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let rate = compute_rate(&app.recent_qsos, now_secs);

    let mult_matrix = if mult_kind != MultKind::None && !rule_bands.is_empty() {
        let band_refs: Vec<&str> = rule_bands.iter().map(std::string::String::as_str).collect();
        compute_mult_matrix(mult_kind, &band_refs, &app.recent_qsos)
    } else {
        MultMatrix::default()
    };

    // Prognoza wyniku: obecny wynik + tempo z ostatniej godziny × średnia punktów × mnożniki.
    let avg_points_per_qso = if app.contest_qsos > 0 {
        pts as f64 / app.contest_qsos as f64
    } else {
        0.0
    };
    let projected_1h =
        total as f64 + rate.rate_60m() as f64 * avg_points_per_qso * mults.max(1) as f64;

    egui::Window::new(tr("contest.window_title", lang))
        .id(egui::Id::new("splogbook_contest_window"))
        .open(&mut open)
        .default_size([800.0, 600.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("contest.select", lang));
                    egui::ComboBox::from_id_salt("contest_type")
                        .selected_text(&app.contest_name)
                        .show_ui(ui, |ui| {
                            for r in RULES {
                                ui.selectable_value(&mut app.contest_name, r.name.to_string(), r.name);
                            }
                            if !app.custom_contests.is_empty() {
                                ui.separator();
                                for c in &app.custom_contests {
                                    ui.selectable_value(&mut app.contest_name, c.name.clone(), c.name.clone());
                                }
                            }
                        });

                    if ui.button(tr("contest.add_custom", lang)).clicked() {
                        app.show_custom_contest_editor = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        crate::gui::user_manual::help_button(app, ui, "contest");
                    });
                });

                if unknown_contest_rule {
                    ui.colored_label(
                        egui::Color32::from_rgb(239, 68, 68),
                        "⚠ Nieznane reguły kontestu dla wybranej nazwy — wynik NIE jest liczony wg domyślnych reguł. Wybierz kontest z listy lub utwórz regułę niestandardową.",
                    );
                }

                ui.separator();

                ui.columns(3, |cols| {
                    cols[0].group(|ui| {
                        ui.label(egui::RichText::new(tr("contest.current_results", lang)).strong().color(egui::Color32::from_rgb(251, 191, 36)));
                        ui.label(format!("{} {}", tr("contest.qso_count", lang), app.contest_qsos));
                        ui.label(format!("{} {}", tr("contest.qso_points", lang), app.contest_points));
                        ui.label(format!("{} {}", tr("contest.multipliers", lang), app.contest_mults));
                        ui.separator();
                        ui.label(egui::RichText::new(format!("{}: {}", tr("contest.final_score", lang), total)).size(16.0).strong().color(egui::Color32::from_rgb(34, 197, 94)));
                    });

                    cols[1].group(|ui| {
                        ui.label(egui::RichText::new(tr("contest.band_stats", lang)).strong().color(egui::Color32::from_rgb(147, 197, 253)));
                        egui::ScrollArea::vertical().id_salt("band_stats").max_height(80.0).show(ui, |ui| {
                            let mut bands = std::collections::HashMap::new();
                            for q in &app.recent_qsos {
                                *bands.entry(q.band.clone()).or_insert(0) += 1;
                            }
                            let mut band_vec: Vec<_> = bands.into_iter().collect();
                            band_vec.sort_by_key(|a| std::cmp::Reverse(a.1));
                            for (b, count) in band_vec {
                                ui.label(format!("{} {}: {} QSOs", tr("contest.band_header", lang), b, count));
                            }
                        });
                    });

                    cols[2].group(|ui| {
                        ui.label(egui::RichText::new(tr("contest.rate", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                        ui.label(format!("1m:  {} QSO/h", rate.rate_1m()));
                        ui.label(format!("5m:  {} QSO/h", rate.rate_5m()));
                        ui.label(format!("10m: {} QSO/h", rate.rate_10m()));
                        ui.label(format!("60m: {} QSO/h", rate.rate_60m()));
                        ui.label(
                            egui::RichText::new(format!("{}: {:.0}", tr("contest.projected_1h", lang), projected_1h))
                                .color(egui::Color32::from_rgb(34, 197, 94)),
                        );
                        ui.label(format!("{} {:03}", tr("contest.stx_nr", lang), app.contest_stx));
                    });
                });

                ui.add_space(8.0);
                ui.separator();

                // Wykres tempa (ostatnie 60 minut) + macierz mnożników.
                egui::CollapsingHeader::new(tr("contest.rate_chart", lang))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_rate_chart(ui, &rate);
                    });

                if mult_kind != MultKind::None && !rule_bands.is_empty() {
                    egui::CollapsingHeader::new(format!("{} ({})", tr("contest.mult_matrix", lang), mult_kind.as_str()))
                        .default_open(false)
                        .show(ui, |ui| {
                            render_mult_matrix(ui, &mult_matrix, lang);
                        });
                }

                // QSO ENTRY FORM
                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr("contest.entry_header", lang)).strong());
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", tr("qso.callsign", lang)));

                        let dummy_qso = QsoRecord {
                            callsign: app.entry_callsign.clone(),
                            band: app.entry_band.clone(),
                            mode: app.entry_mode.clone(),
                            ..Default::default()
                        };

                        let is_dupe = if is_custom {
                            app.recent_qsos.iter().any(|q| q.callsign == dummy_qso.callsign && q.band == dummy_qso.band)
                        } else if let Some(rule_idx) = RULES.iter().position(|r| r.name == app.contest_name) {
                            detect_duplicate(&RULES[rule_idx], &dummy_qso, &app.recent_qsos)
                        } else {
                            // Nieznane reguły kontestu: prosta kontrola duplikatu wg znaku+pasma
                            // zamiast cichego zastosowania reguł innego (pierwszego) kontestu.
                            app.recent_qsos.iter().any(|q| q.callsign == dummy_qso.callsign && q.band == dummy_qso.band)
                        };

                        if is_dupe && !app.entry_callsign.is_empty() {
                            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::RED);
                        }

                        ui.add(egui::TextEdit::singleline(&mut app.entry_callsign).desired_width(120.0));

                        if is_dupe && !app.entry_callsign.is_empty() {
                            ui.label(egui::RichText::new("DUPE!").color(egui::Color32::RED).strong());
                        }

                        ui.label(tr("contest.rcvd_report_exchange", lang));
                        ui.add(egui::TextEdit::singleline(&mut app.entry_exchange).desired_width(200.0));

                        if ui.button(tr("contest.save_qso", lang)).clicked() {
                            save_clicked = true;
                        }
                    });

                    if !exchange_hint.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Format wymiany: {exchange_hint}"))
                                .italics()
                                .weak(),
                        );
                    }
                    // Podgląd / walidacja parsowania na żywo.
                    if !app.entry_exchange.trim().is_empty() {
                        if let Some(fields) = &exchange_fields {
                            match parse_exchange(fields, &app.entry_exchange) {
                                Ok(parsed) => {
                                    ui.label(
                                        egui::RichText::new(format!("✓ {}", exchange_summary(&parsed)))
                                            .color(egui::Color32::from_rgb(34, 197, 94)),
                                    );
                                }
                                Err(e) => {
                                    ui.label(
                                        egui::RichText::new(format!("⚠ {}", e.message))
                                            .color(egui::Color32::from_rgb(239, 68, 68)),
                                    );
                                }
                            }
                        }
                    }
                });

                // DX Cluster / Bandmap wbudowany w okno kontestu: spot -> wpis jednym klikiem.
                egui::CollapsingHeader::new(tr("contest.spot_list", lang))
                    .default_open(false)
                    .show(ui, |ui| {
                        let mut tune: Option<(String, f64, String, bool)> = None;
                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            egui::Grid::new("contest_spot_grid")
                                .striped(true)
                                .num_columns(5)
                                .spacing([10.0, 3.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(tr("cluster.dx", lang)).strong());
                                    ui.label(egui::RichText::new(tr("cluster.freq", lang)).strong());
                                    ui.label(egui::RichText::new(tr("qso.band", lang)).strong());
                                    ui.label(egui::RichText::new(tr("cluster.comment", lang)).strong());
                                    ui.label("");
                                    ui.end_row();

                                    for spot in &app.cluster_spots {
                                        let worked = app
                                            .recent_qsos
                                            .iter()
                                            .any(|q| q.callsign.eq_ignore_ascii_case(&spot.dx_call));
                                        let call_color = if worked {
                                            egui::Color32::from_rgb(34, 197, 94)
                                        } else {
                                            egui::Color32::from_rgb(56, 189, 248)
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{}{}", spot.dx_call, if worked { " ✓" } else { "" }))
                                                .strong()
                                                .color(call_color),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.1} kHz", spot.frequency_khz)).monospace(),
                                        );
                                        ui.label(egui::RichText::new(&spot.band).color(egui::Color32::from_rgb(251, 191, 36)));
                                        ui.label(egui::RichText::new(&spot.comment).size(11.0).weak());
                                        if ui
                                            .button(egui::RichText::new(tr("contest.spot_fill_entry", lang)).color(egui::Color32::from_rgb(34, 197, 94)))
                                            .clicked()
                                        {
                                            tune = Some((spot.dx_call.clone(), spot.frequency_khz, spot.band.clone(), spot.is_ft8));
                                        }
                                        ui.end_row();
                                    }
                                });
                        });

                        if let Some((call, freq_khz, band, is_ft8)) = tune {
                            spot_fill_target = Some((call, freq_khz, band, is_ft8));
                        }
                    });

                ui.add_space(8.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button(tr("contest.export_cabrillo", lang)).clicked() {
                        export_clicked = true;
                    }
                    if ui.button(tr("contest.reset_numbering", lang)).clicked() {
                        reset_clicked = true;
                    }
                    if ui.button(tr("btn.close", lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });

    if let Some((call, freq_khz, band, is_ft8)) = spot_fill_target {
        app.tune_to_spot(&call, freq_khz, &band);
        if is_ft8 {
            app.entry_mode = "FT8".to_string();
        }
        // Uzupełnij wymianę z wcześniejszego QSO z tą stacją (jeśli istnieje).
        if let Some(prev) = app
            .recent_qsos
            .iter()
            .rev()
            .find(|q| q.callsign.eq_ignore_ascii_case(&call))
        {
            if let Some(srx) = &prev.srx_string {
                app.entry_exchange = srx.clone();
            } else if !prev.rst_rcvd.is_empty() {
                app.entry_exchange = prev.rst_rcvd.clone();
            }
            app.status_toast = Some((
                format!(
                    "{}: {}",
                    tr("contest.worked_before", app.current_language),
                    call
                ),
                std::time::Instant::now(),
            ));
        }
    }

    if save_clicked && !app.entry_callsign.is_empty() {
        let mut new_qso = QsoRecord::new(&app.entry_callsign, &app.entry_band, &app.entry_mode);
        new_qso.stx = Some(app.contest_stx);
        new_qso.journal_id = Some("CONTEST".to_string());

        // Parsuj wymianę, jeśli kontest ma zdefiniowane pola; w przeciwnym razie
        // zachowaj dawny tryb "sam RST" w polu `entry_rst_rcvd`.
        let mut exchange_ok = true;
        if let Some(fields) = &exchange_fields {
            let exchange_text = if app.entry_exchange.trim().is_empty() {
                app.entry_rst_rcvd.clone()
            } else {
                app.entry_exchange.clone()
            };
            match parse_exchange(fields, &exchange_text) {
                Ok(parsed) => apply_to_qso(&parsed, &mut new_qso),
                Err(e) => {
                    exchange_ok = false;
                    app.status_toast = Some((
                        format!("Błąd wymiany: {}", e.message),
                        std::time::Instant::now(),
                    ));
                }
            }
        } else {
            new_qso.rst_rcvd.clone_from(&app.entry_rst_rcvd);
        }

        if exchange_ok {
            if let Err(reason) = new_qso.validate() {
                app.status_toast = Some((
                    format!("Błąd walidacji QSO kontestowego: {reason}"),
                    std::time::Instant::now(),
                ));
            } else {
                let insert_result = match app.log_db.lock() {
                    Ok(db) => db.insert_qso(&new_qso).map_err(|error| error.to_string()),
                    Err(error) => Err(format!("Nie można otworzyć dziennika: {error}")),
                };
                match insert_result {
                    Ok(_) => {
                        app.contest_stx += 1;
                        app.entry_callsign.clear();
                        app.entry_rst_rcvd.clear();
                        app.entry_exchange.clear();
                        app.reload_qsos();
                    }
                    Err(error) => {
                        app.status_toast = Some((
                            format!("Błąd zapisu QSO kontestowego: {error}"),
                            std::time::Instant::now(),
                        ));
                    }
                }
            }
        }
    }

    if close_req {
        open = false;
    }
    app.show_contest_window = open;

    if export_clicked {
        app.export_cabrillo();
    }
    if reset_clicked {
        app.contest_stx = 1;
        app.contest_qsos = 0;
        app.contest_points = 0;
        app.contest_mults = 0;
    }
}

/// Okno synchronizacji pracy wielostanowiskowej w sieci lokalnej (Multi-Op LAN)
pub fn render_multi_op_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_multi_op_window {
        return;
    }
    let mut is_open = app.show_multi_op_window;
    let mut close_req = false;
    let lang = app.current_language;

    egui::Window::new("🌐 Praca Zespołowa Multi-Op (Synchronizacja LAN)")
        .open(&mut is_open)
        .default_size([520.0, 420.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading(egui::RichText::new("Wielostanowiskowa Synchronizacja QSO w Sieci Lokalnej").size(14.0).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.label("Umożliwia automatyczną wymianę nowo dodanych QSO pomiędzy stanowiskami w klubie lub w zawodach.");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.multi_op_is_server, true, "🖥 Serwer (Host zawodów)");
                    ui.selectable_value(&mut app.multi_op_is_server, false, "💻 Klient (Drugie stanowisko)");
                });

                ui.add_space(4.0);

                if app.multi_op_is_server {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Konfiguracja Hosta:").strong());
                        ui.horizontal(|ui| {
                            ui.label("Port nasłuchu TCP:");
                            let mut port_str = app.lan_sync_port.to_string();
                            if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(70.0)).changed() {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    app.lan_sync_port = p;
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Hasło współdzielone:");
                            let mut secret_display = app.lan_sync_secret.clone();
                            if ui.add(egui::TextEdit::singleline(&mut secret_display).desired_width(160.0).password(false)).changed() {
                                app.lan_sync_secret = secret_display;
                            }
                            if ui.button("🎲").on_hover_text("Wygeneruj losowe hasło").clicked() {
                                app.lan_sync_secret = crate::api::server::generate_api_key()[..12].to_string();
                                app.save_station_config();
                            }
                        });
                        ui.label(egui::RichText::new("⚠ Podaj to samo hasło na stacji klienckiej. Hasło jest wymagane — serwer nie wystartuje bez niego.").size(11.0).italics());

                        ui.horizontal(|ui| {
                            if app.multi_op_server.is_some() {
                                if ui.button(egui::RichText::new("🛑 ZATRZYMAJ SERWER LAN").color(egui::Color32::from_rgb(239, 68, 68)).strong()).clicked() {
                                    if let Some(srv) = app.multi_op_server.take() {
                                        srv.stop();
                                    }
                                    app.multi_op_status = "Serwer zatrzymany".to_string();
                                    app.multi_op_log.push("Zatrzymano serwer Multi-Op LAN.".to_string());
                                }
                            } else if ui.button(egui::RichText::new("🚀 URUCHOM SERWER LAN").color(egui::Color32::from_rgb(34, 197, 94)).strong()).clicked() {
                                if app.lan_sync_secret.trim().is_empty() {
                                    app.multi_op_status = "Błąd: serwer wymaga hasła współdzielonego.".to_string();
                                    app.multi_op_log.push("Odmowa uruchomienia serwera: brak hasła współdzielonego.".to_string());
                                } else {
                                    let secret = Some(app.lan_sync_secret.clone());
                                    let server = std::sync::Arc::new(crate::cluster::lan_sync::MultiOpServer::new_with_secret(app.lan_sync_port, secret));
                                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                                    app.multi_op_incoming_rx = Some(rx);
                                    let srv_clone = server.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = srv_clone.start(tx).await {
                                            log::error!("Multi-Op: nie udało się uruchomić serwera: {e}");
                                        }
                                    });
                                    app.multi_op_server = Some(server);
                                    app.multi_op_status = format!("Serwer nasłuchuje na porcie {}", app.lan_sync_port);
                                    app.multi_op_log.push(format!("Uruchomiono serwer Multi-Op LAN na porcie {}", app.lan_sync_port));
                                }
                            }
                        });
                    });
                } else {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Połączenie ze stacją główną:").strong());
                        ui.horizontal(|ui| {
                            ui.label("Adres IP Hosta:");
                            ui.add(egui::TextEdit::singleline(&mut app.lan_sync_server_ip).desired_width(120.0));
                            ui.label("Port:");
                            let mut port_str = app.lan_sync_port.to_string();
                            if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0)).changed() {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    app.lan_sync_port = p;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Hasło współdzielone (od hosta):");
                            let mut secret_display = app.lan_sync_secret.clone();
                            if ui.add(egui::TextEdit::singleline(&mut secret_display).desired_width(160.0)).changed() {
                                app.lan_sync_secret = secret_display;
                            }
                        });
                        if ui.button("⚡ Test Połączenia z Hostem").clicked() {
                            let ip = app.lan_sync_server_ip.clone();
                            let port = app.lan_sync_port;
                            match std::net::TcpStream::connect_timeout(
                                &format!("{ip}:{port}").parse().unwrap_or_else(|_| "127.0.0.1:7373".parse().unwrap()),
                                std::time::Duration::from_millis(800),
                            ) {
                                Ok(_) => {
                                    app.multi_op_status = format!("Połączenie z Hostem {ip}:{port} pomyślne!");
                                    app.multi_op_log.push(format!("Połączono z {ip}:{port}"));
                                }
                                Err(e) => {
                                    app.multi_op_status = format!("Błąd połączenia z {ip}:{port}: {e}");
                                    app.multi_op_log.push(format!("Błąd połączenia: {e}"));
                                }
                            }
                        }
                    });
                }

                ui.add_space(4.0);
                ui.label(egui::RichText::new(format!("Status: {}", app.multi_op_status)).strong().color(egui::Color32::from_rgb(56, 189, 248)));

                ui.separator();
                ui.label(egui::RichText::new("Dziennik zdarzeń sieci LAN:").strong());
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    if app.multi_op_log.is_empty() {
                        ui.label("Brak zdarzeń w sieci Multi-Op.");
                    } else {
                        for entry in app.multi_op_log.iter().rev() {
                            ui.label(egui::RichText::new(entry).size(11.0).monospace());
                        }
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(tr("btn.save", lang)).strong()).clicked() {
                        app.save_station_config();
                        close_req = true;
                    }
                    if ui.button(tr("btn.close", lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });
    if close_req {
        is_open = false;
    }
    app.show_multi_op_window = is_open;
}

/// Rysuje mini-wykres słupkowy tempa QSO na minutę (ostatnie 60 minut).
fn render_rate_chart(ui: &mut egui::Ui, rate: &RateStats) {
    let chart_h = 60.0f32;
    let chart_w = ui.available_width();
    let (resp, painter) =
        ui.allocate_painter(egui::vec2(chart_w, chart_h + 14.0), egui::Sense::hover());
    let origin = resp.rect.min;
    let max_val = rate.per_minute.iter().copied().max().unwrap_or(1).max(1) as f32;
    let n = rate.per_minute.len();
    if n == 0 {
        return;
    }
    let bar_w = chart_w / n as f32;
    for (i, v) in rate.per_minute.iter().enumerate() {
        let bar_h = (*v as f32 / max_val) * chart_h;
        let x = origin.x + i as f32 * bar_w;
        let rect = egui::Rect::from_min_size(
            egui::pos2(x + 0.5, origin.y + chart_h - bar_h),
            egui::vec2((bar_w - 1.0).max(0.5), bar_h),
        );
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(56, 189, 248));
    }
    // Linia bazowa i etykiety osi.
    painter.line_segment(
        [
            egui::pos2(origin.x, origin.y + chart_h),
            egui::pos2(origin.x + chart_w, origin.y + chart_h),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 116, 139)),
    );
    painter.text(
        egui::pos2(origin.x, origin.y + chart_h + 4.0),
        egui::Align2::LEFT_TOP,
        "-60 min",
        egui::FontId::proportional(9.0),
        egui::Color32::from_rgb(148, 163, 184),
    );
    painter.text(
        egui::pos2(origin.x + chart_w, origin.y + chart_h + 4.0),
        egui::Align2::RIGHT_TOP,
        "now",
        egui::FontId::proportional(9.0),
        egui::Color32::from_rgb(148, 163, 184),
    );
}

/// Rysuje macierz mnożników: wiersze = pasma, kolumny = mnożniki.
/// Zielony = zaliczony na paśmie, żółty = zaliczony gdzie indziej, szary = potrzebny.
fn render_mult_matrix(ui: &mut egui::Ui, matrix: &MultMatrix, lang: crate::core::i18n::Language) {
    if matrix.bands.is_empty() {
        ui.label(tr("log.no_results", lang));
        return;
    }

    // Legenda.
    ui.horizontal(|ui| {
        ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "●");
        ui.label(tr("contest.mult_worked_here", lang));
        ui.colored_label(egui::Color32::from_rgb(251, 191, 36), "◐");
        ui.label(tr("contest.mult_worked_other", lang));
        ui.colored_label(egui::Color32::from_rgb(71, 85, 105), "·");
        ui.label(tr("contest.mult_needed", lang));
        ui.label(
            egui::RichText::new(format!(
                "({}: {})",
                tr("contest.mult_needed_count", lang),
                matrix.needed_count()
            ))
            .weak(),
        );
    });

    egui::ScrollArea::horizontal().show(ui, |ui| {
        egui::Grid::new("contest_mult_matrix")
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                // Nagłówek z etykietami mnożników.
                ui.label("");
                for col in &matrix.columns {
                    ui.label(
                        egui::RichText::new(col)
                            .size(8.0)
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );
                }
                ui.end_row();

                for (r, band) in matrix.bands.iter().enumerate() {
                    ui.label(egui::RichText::new(band).strong().size(10.0));
                    for (c, col) in matrix.columns.iter().enumerate() {
                        let v = matrix.worked[r][c];
                        let (color, glyph) = match v {
                            2 => (egui::Color32::from_rgb(34, 197, 94), "●"),
                            1 => (egui::Color32::from_rgb(251, 191, 36), "◐"),
                            _ => (egui::Color32::from_rgb(71, 85, 105), "·"),
                        };
                        ui.colored_label(color, glyph)
                            .on_hover_text(format!("{band} × {col}"));
                    }
                    ui.end_row();
                }
            });
    });
}
