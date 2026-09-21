// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::gui::app::SpLogApp;
use crate::core::i18n::tr;
use crate::core::contest_rules::{RULES, calculate_score, detect_duplicate};
use crate::core::qso::QsoRecord;
use crate::core::station::CustomContest;

use eframe::egui;
use std::collections::HashSet;

pub fn render_custom_contest_editor(app: &mut SpLogApp, ctx: &egui::Context) {
    let mut open = app.show_custom_contest_editor;
    if !open { return; }
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
                    ui.heading(if app.custom_contest_edit_idx.is_some() { tr("contest.editing", lang) } else { tr("contest.new", lang) });
                    
                    ui.horizontal(|ui| {
                        ui.label(tr("contest.name_label", lang));
                        ui.text_edit_singleline(&mut app.custom_contest_draft.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("contest.exchange_label", lang));
                        ui.text_edit_singleline(&mut app.custom_contest_draft.exchange_format);
                    });
                    
                    ui.label(tr("contest.points_per_qso_label", lang));
                    ui.add(egui::Slider::new(&mut app.custom_contest_draft.points_per_qso, 1..=20));

                    ui.label(tr("contest.bands_label", lang));
                    let all_bands = ["160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"];
                    ui.horizontal_wrapped(|ui| {
                        for b in all_bands {
                            let mut has_band = app.custom_contest_draft.bands.contains(&b.to_string());
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

                    if ui.button(tr("btn.save", lang)).clicked() {
                        if !app.custom_contest_draft.name.is_empty() {
                            if let Some(i) = app.custom_contest_edit_idx {
                                app.custom_contests[i] = app.custom_contest_draft.clone();
                            } else {
                                app.custom_contests.push(app.custom_contest_draft.clone());
                            }
                            app.custom_contest_edit_idx = None;
                            app.custom_contest_draft = CustomContest::default();
                        }
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

    // Is it a custom contest?
    let mut is_custom = false;
    let mut custom_idx = None;
    if let Some(idx) = app.custom_contests.iter().position(|c| c.name == app.contest_name) {
        is_custom = true;
        custom_idx = Some(idx);
    }
    
    let my_dxcc: u32 = 269;
    let my_cqzone = app.my_station.cq_zone as u8;

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
    } else {
        let rule_idx = RULES.iter().position(|r| r.name == app.contest_name).unwrap_or(0);
        let active_rule = &RULES[rule_idx];
        calculate_score(active_rule, &app.recent_qsos, my_dxcc, my_cqzone)
    };
    
    app.contest_points = pts;
    app.contest_mults = mults;
    app.contest_qsos = app.recent_qsos.len() as u32;

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
                                    ui.selectable_value(&mut app.contest_name, c.name.to_string(), c.name.clone());
                                }
                            }
                        });
                        
                    if ui.button(tr("contest.add_custom", lang)).clicked() {
                        app.show_custom_contest_editor = true;
                    }
                });

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
                            band_vec.sort_by(|a, b| b.1.cmp(&a.1));
                            for (b, count) in band_vec {
                                ui.label(format!("{} {}: {} QSOs", tr("contest.band_header", lang), b, count));
                            }
                        });
                    });

                    cols[2].group(|ui| {
                        ui.label(egui::RichText::new(tr("contest.rate", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                        let now_mins = {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() / 60;
                            now
                        };
                        let recent_count = app.recent_qsos.iter().filter(|q| {
                            if q.time_on.len() >= 5 {
                                if let (Ok(h), Ok(m)) = (
                                    q.time_on[..2].parse::<u64>(),
                                    q.time_on[3..5].parse::<u64>()
                                ) {
                                    let qso_mins_today = h * 60 + m;
                                    let now_mins_today = now_mins % (24 * 60);
                                    let diff = now_mins_today.saturating_sub(qso_mins_today);
                                    return diff <= 60;
                                }
                            }
                            false
                        }).count() as u32;
                        ui.label(format!("{} {} QSO/h", tr("contest.current_rate", lang), recent_count));
                        ui.label(format!("{} {:03}", tr("contest.stx_nr", lang), app.contest_stx));
                    });
                });

                ui.add_space(8.0);
                ui.separator();

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
                        } else {
                            let rule_idx = RULES.iter().position(|r| r.name == app.contest_name).unwrap_or(0);
                            detect_duplicate(&RULES[rule_idx], &dummy_qso, &app.recent_qsos)
                        };
                        
                        if is_dupe && !app.entry_callsign.is_empty() {
                            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::RED);
                        }
                        
                        ui.add(egui::TextEdit::singleline(&mut app.entry_callsign).desired_width(120.0));
                        
                        if is_dupe && !app.entry_callsign.is_empty() {
                            ui.label(egui::RichText::new("DUPE!").color(egui::Color32::RED).strong());
                        }
                        
                        ui.label(tr("contest.rcvd_report_exchange", lang));
                        ui.add(egui::TextEdit::singleline(&mut app.entry_rst_rcvd).desired_width(100.0));
                        
                        if ui.button(tr("contest.save_qso", lang)).clicked() {
                            save_clicked = true;
                        }
                    });
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

    if save_clicked && !app.entry_callsign.is_empty() {
        let mut new_qso = QsoRecord::new(&app.entry_callsign, &app.entry_band, &app.entry_mode);
        new_qso.rst_rcvd = app.entry_rst_rcvd.clone();
        new_qso.stx = Some(app.contest_stx);
        new_qso.journal_id = Some("CONTEST".to_string());
        app.recent_qsos.push(new_qso.clone());
        if let Ok(db) = app.log_db.lock() {
            let _ = db.insert_qso(&new_qso);
        }
        app.contest_stx += 1;
        app.entry_callsign.clear();
        app.entry_rst_rcvd.clear();
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
                            if app.multi_op_server.is_some() {
                                if ui.button(egui::RichText::new("🛑 ZATRZYMAJ SERWER LAN").color(egui::Color32::from_rgb(239, 68, 68)).strong()).clicked() {
                                    app.multi_op_server = None;
                                    app.multi_op_status = "Serwer zatrzymany".to_string();
                                    app.multi_op_log.push("Zatrzymano serwer Multi-Op LAN.".to_string());
                                }
                            } else {
                                if ui.button(egui::RichText::new("🚀 URUCHOM SERWER LAN").color(egui::Color32::from_rgb(34, 197, 94)).strong()).clicked() {
                                    let server = std::sync::Arc::new(crate::cluster::lan_sync::MultiOpServer::new(app.lan_sync_port));
                                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                                    app.multi_op_incoming_rx = Some(rx);
                                    let srv_clone = server.clone();
                                    tokio::spawn(async move {
                                        let _ = srv_clone.start(tx).await;
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
                        if ui.button("⚡ Test Połączenia z Hostem").clicked() {
                            let ip = app.lan_sync_server_ip.clone();
                            let port = app.lan_sync_port;
                            match std::net::TcpStream::connect_timeout(
                                &format!("{}:{}", ip, port).parse().unwrap_or_else(|_| "127.0.0.1:7373".parse().unwrap()),
                                std::time::Duration::from_millis(800),
                            ) {
                                Ok(_) => {
                                    app.multi_op_status = format!("Połączenie z Hostem {}:{} pomyślne!", ip, port);
                                    app.multi_op_log.push(format!("Połączono z {}:{}", ip, port));
                                }
                                Err(e) => {
                                    app.multi_op_status = format!("Błąd połączenia z {}:{}: {}", ip, port, e);
                                    app.multi_op_log.push(format!("Błąd połączenia: {}", e));
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
