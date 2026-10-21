// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::lotw::detect_tqsl_path;
use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTab {
    Lotw,
    Eqsl,
    Clublog,
    QrzHamqth,
    CloudlogHrdlog,
    Fldigi,
    PskReporter,
    N1mm,
    Databases,
    Queue,
    Logs,
}

/// Kompaktowe okno dialogowe synchronizacji z serwisami online i ARRL LoTW
pub fn render_online_sync_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.online.show_online_sync_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.online.show_online_sync_window;
    let mut close_req = false;

    let screen_height = ctx.content_rect().height();
    let max_h = (screen_height * 0.82).min(520.0);

    let active_tab_id = egui::Id::new("online_sync_active_tab");
    let mut active_tab = ctx
        .data_mut(|d| d.get_temp::<SyncTab>(active_tab_id))
        .unwrap_or(SyncTab::Lotw);

    egui::Window::new(format!("🌐 {}", tr(key::SYNC_TITLE, lang)))
        .open(&mut is_open)
        .default_size([640.0, 420.0])
        .max_height(max_h)
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.vertical(|ui| {
                // Nagłówek i status
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new(tr(key::SYNC_WINDOW_TITLE, lang)).size(15.0).color(palette.accent));
                });
                ui.separator();

                // Pasek zakładek (Tabs)
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut active_tab, SyncTab::Lotw, "🏆 LoTW");
                    ui.selectable_value(&mut active_tab, SyncTab::Eqsl, "📨 eQSL.cc");
                    ui.selectable_value(&mut active_tab, SyncTab::Clublog, "📡 Club Log");
                    ui.selectable_value(&mut active_tab, SyncTab::QrzHamqth, "📖 QRZ & HamQTH");
                    ui.selectable_value(&mut active_tab, SyncTab::CloudlogHrdlog, "☁ Cloudlog / HRDLog");
                    ui.selectable_value(&mut active_tab, SyncTab::Fldigi, "💻 FLDigi");
                    ui.selectable_value(&mut active_tab, SyncTab::PskReporter, "📡 PSK Reporter");
                    ui.selectable_value(&mut active_tab, SyncTab::N1mm, "📻 N1MM Broadcast");
                    ui.selectable_value(&mut active_tab, SyncTab::Databases, tr(key::SYNC_TAB_DATABASES, lang));
                    ui.selectable_value(&mut active_tab, SyncTab::Queue, tr(key::SYNC_TAB_QUEUE, lang));
                    ui.selectable_value(&mut active_tab, SyncTab::Logs, Msg::new(key::SYNC_TAB_LOGS).arg("count", app.online.online_sync_logs.len()).text(lang));
                });
                ui.separator();

                // Zawartość aktywnej zakładki w przewijanym obszarze o kontrolowanej wysokości
                egui::ScrollArea::vertical()
                    .max_height(max_h - 135.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match active_tab {
                            SyncTab::Lotw => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_LOTW, lang)).strong().size(13.0).color(palette.warning));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_TQSL_PATH, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_tqsl_path).desired_width(260.0));
                                        if ui.button(tr(key::SYNC_DETECT_BTN, lang)).clicked() {
                                            if let Some(detected) = detect_tqsl_path() {
                                                app.lotw_tqsl_path = detected.to_string_lossy().to_string();
                                                app.online.online_sync_logs.push(format!("Wykryto TQSL w: {}", app.lotw_tqsl_path));
                                            } else {
                                                app.online.online_sync_logs.push(tr(key::SYNC_TQSL_NOT_FOUND, lang).to_string());
                                            }
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_STATION_LOCATION, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_station_name).desired_width(180.0));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_LOTW_LOGIN, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label(tr(key::SYNC_PASSWORD, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_password).password(true).desired_width(120.0));
                                    });

                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button(egui::RichText::new(format!("📤 {}", tr(key::SYNC_LOTW_TQSL, lang))).strong()).clicked() {
                                            let qsos = if let Ok(db) = app.log_db.lock() {
                                                db.get_recent_qsos(1000).unwrap_or_default()
                                            } else {
                                                vec![]
                                            };
                                            let qso_count = qsos.len();
                                            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                            let tqsl_path = app.lotw_tqsl_path.clone();
                                            let station_loc = app.lotw_station_name.clone();
                                            let tx = app.sync_log_tx.clone();
                                            app.online.online_sync_logs.push(Msg::new(key::SYNC_LOTW_SIGNING_LOG).arg("count", qso_count).text(lang));
                                            app.status_message = Some(tr(key::SYNC_LOTW_IN_PROGRESS, lang).to_string());

                                            tokio::spawn(async move {
                                                match crate::cloud::lotw::export_and_sign_tqsl_async(&tqsl_path, &station_loc, &adif).await {
                                                    Ok(msg) => {
                                                        let _ = tx.send((Msg::new(key::SYNC_LOTW_SUCCESS).arg("count", qso_count).arg("msg", &msg).text(lang), false));
                                                    }
                                                    Err(e) => {
                                                        let _ = tx.send((Msg::new(key::SYNC_LOTW_ERROR).arg("err", &e).text(lang), false));
                                                    }
                                                }
                                            });
                                        }

                                        if ui.button(egui::RichText::new(format!("📥 {}", tr(key::SYNC_LOTW_DOWNLOAD, lang)))).clicked() {
                                            if app.lotw_username.is_empty() || app.lotw_password.is_empty() {
                                                app.online.online_sync_logs.push(tr(key::SYNC_LOTW_MISSING_CREDS, lang).to_string());
                                            } else {
                                                let tx = app.sync_log_tx.clone();
                                                let user = app.lotw_username.clone();
                                                let pass = app.lotw_password.clone();
                                                let log_db = app.log_db.clone();
                                                tokio::spawn(async move {
                                                    let _ = tx.send((tr(key::SYNC_LOTW_DOWNLOADING, lang).to_string(), false));
                                                    let report_res = crate::cloud::lotw::download_lotw_report(&user, &pass, None)
                                                        .await
                                                        .map_err(|e| e.to_string());
                                                    match report_res {
                                                        Ok(adif) => {
                                                            let (count, updated) = tokio::task::spawn_blocking(move || {
                                                                let confs = crate::cloud::lotw::parse_lotw_confirmations(&adif);
                                                                let count = confs.len();
                                                                let mut updated = 0;
                                                                if let Ok(db) = log_db.lock() {
                                                                    for conf in confs {
                                                                        if let Ok(n) = db.mark_lotw_confirmed(&conf.callsign, &conf.band, &conf.mode, &conf.qso_date, &conf.qsl_rdate) {
                                                                            updated += n;
                                                                        }
                                                                    }
                                                                }
                                                                (count, updated)
                                                            })
                                                            .await
                                                            .unwrap_or((0, 0));
                                                            let _ = tx.send((Msg::new(key::SYNC_LOTW_DOWNLOAD_SUCCESS).arg("count", count).arg("updated", updated).text(lang), true));
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send((Msg::new(key::SYNC_LOTW_ERROR).arg("err", &e).text(lang), false));
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    });
                                });
                            }
                            SyncTab::Eqsl => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_EQSL, lang)).strong().size(13.0).color(palette.accent));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_EQSL_USER, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.online.eqsl_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label(tr(key::SYNC_EQSL_PASSWORD, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.online.eqsl_password).password(true).desired_width(120.0));
                                    });

                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button(format!("📤 {}", tr(key::SYNC_EQSL_UPLOAD, lang))).clicked() {
                                            if app.online.eqsl_username.is_empty() || app.online.eqsl_password.is_empty() {
                                                app.online.online_sync_logs.push(tr(key::SYNC_EQSL_MISSING_CREDS, lang).to_string());
                                            } else {
                                                let tx = app.sync_log_tx.clone();
                                                let user = app.online.eqsl_username.clone();
                                                let pass = app.online.eqsl_password.clone();
                                                let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(1000).unwrap_or_default() } else { vec![] };
                                                let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                                tokio::spawn(async move {
                                                    let client = crate::cloud::eqsl::EqslClient::new(user, pass);
                                                    let _ = tx.send((Msg::new(key::SYNC_EQSL_SENDING).arg("count", qsos.len()).text(lang), false));
                                                    match client.upload_adif(&adif).await {
                                                        Ok(msg) => { let _ = tx.send((format!("[eQSL SUKCES] {msg}"), false)); }
                                                        Err(e) => { let _ = tx.send((Msg::new(key::SYNC_EQSL_ERROR).arg("err", &e).text(lang), false)); }
                                                    }
                                                });
                                            }
                                        }
                                        if ui.button(format!("📥 {}", tr(key::SYNC_EQSL_DOWNLOAD, lang))).clicked() {
                                            if app.online.eqsl_username.is_empty() || app.online.eqsl_password.is_empty() {
                                                app.online.online_sync_logs.push(tr(key::SYNC_EQSL_MISSING_CREDS, lang).to_string());
                                            } else {
                                                let tx = app.sync_log_tx.clone();
                                                let user = app.online.eqsl_username.clone();
                                                let pass = app.online.eqsl_password.clone();
                                                let log_db = app.log_db.clone();
                                                tokio::spawn(async move {
                                                    let client = crate::cloud::eqsl::EqslClient::new(user, pass);
                                                    let _ = tx.send(("[eQSL] Pobieranie skrzynki odbiorczej (Inbox ADIF)...".to_string(), false));
                                                    match client.download_inbox_adif().await {
                                                        Ok(adif) => {
                                                            let (count, updated) = tokio::task::spawn_blocking(move || {
                                                                let qsos = crate::core::adif::parse_adif(&adif);
                                                                let mut updated = 0;
                                                                if let Ok(db) = log_db.lock() {
                                                                    for qso in &qsos {
                                                                        let rdate = qso.eqsl_qslrdate.as_deref().unwrap_or(&qso.qso_date);
                                                                        if let Ok(n) = db.mark_eqsl_confirmed(&qso.callsign, &qso.band, &qso.mode, &qso.qso_date, rdate) {
                                                                            updated += n;
                                                                        }
                                                                    }
                                                                }
                                                                (qsos.len(), updated)
                                                            })
                                                            .await
                                                            .unwrap_or((0, 0));
                                                            let _ = tx.send((Msg::new(key::SYNC_EQSL_DOWNLOAD_SUCCESS).arg("count", count).arg("updated", updated).text(lang), true));
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send((Msg::new(key::SYNC_EQSL_ERROR).arg("err", &e).text(lang), false));
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    });
                                });
                            }
                            SyncTab::Clublog => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_CLUBLOG, lang)).strong().size(13.0).color(palette.success));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_CALLSIGN, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.online.clublog_callsign).desired_width(100.0));
                                        ui.add_space(10.0);
                                        ui.label("Email:");
                                        ui.add(egui::TextEdit::singleline(&mut app.online.clublog_email).desired_width(160.0));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_PASSWORD, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.online.clublog_password).password(true).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label(tr(key::SYNC_API_KEY_OPTIONAL, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.online.clublog_api_key).desired_width(140.0));
                                    });

                                    ui.add_space(4.0);
                                    ui.checkbox(&mut app.live_auto_upload_clublog, tr(key::SYNC_LIVE_AUTO_UPLOAD_CLUBLOG, lang));

                                    ui.add_space(6.0);
                                    if ui.button(format!("📤 {}", tr(key::SYNC_CLUBLOG_UPLOAD, lang))).clicked() {
                                        if app.online.clublog_callsign.is_empty() || app.online.clublog_password.is_empty() {
                                            app.online.online_sync_logs.push(tr(key::SYNC_CLUBLOG_MISSING_CREDS, lang).to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let call = app.online.clublog_callsign.clone();
                                            let email = app.online.clublog_email.clone();
                                            let pass = app.online.clublog_password.clone();
                                            let api = app.online.clublog_api_key.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(5000).unwrap_or_default() } else { vec![] };
                                            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                            tokio::spawn(async move {
                                                let client = crate::cloud::clublog::ClubLogClient::new();
                                                let _ = tx.send((Msg::new(key::SYNC_CLUBLOG_SENDING).arg("count", qsos.len()).text(lang), false));
                                                match client.upload_adif(&call, &email, &pass, &api, &adif).await {
                                                    Ok(msg) => { let _ = tx.send((format!("[Club Log SUKCES] {msg}"), false)); }
                                                    Err(e) => { let _ = tx.send((Msg::new(key::SYNC_CLUBLOG_ERROR).arg("err", &e).text(lang), false)); }
                                                }
                                            });
                                        }
                                    }
                                });
                            }
                            SyncTab::QrzHamqth => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_QRZ, lang)).strong().size(13.0).color(palette.series(6)));
                                    ui.separator();

                                    ui.checkbox(&mut app.qrz_auto_lookup, tr(key::SYNC_AUTO_LOOKUP, lang));
                                    ui.checkbox(&mut app.live_auto_upload_qrz, tr(key::SYNC_LIVE_AUTO_UPLOAD_QRZ, lang));

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_QRZ_LOGIN, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.qrz_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label(tr(key::SYNC_QRZ_PASSWORD, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.qrz_password).password(true).desired_width(120.0));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_QRZ_API_KEY, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.qrz_api_key).desired_width(220.0));
                                    });

                                    ui.add_space(4.0);
                                    if ui.button(format!("📤 {}", tr(key::SYNC_QRZ_UPLOAD, lang))).clicked() {
                                        if app.qrz_api_key.is_empty() {
                                            app.online.online_sync_logs.push(tr(key::SYNC_QRZ_MISSING_API_KEY, lang).to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let api = app.qrz_api_key.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(1000).unwrap_or_default() } else { vec![] };
                                            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                            tokio::spawn(async move {
                                                let _ = tx.send((Msg::new(key::SYNC_QRZ_SENDING).arg("count", qsos.len()).text(lang), false));
                                                match crate::cloud::qrz::QrzClient::upload_to_logbook(&api, &adif).await {
                                                    Ok(msg) => { let _ = tx.send((format!("[QRZ.com SUKCES] {msg}"), false)); }
                                                    Err(e) => { let _ = tx.send((Msg::new(key::SYNC_QRZ_ERROR).arg("err", &e).text(lang), false)); }
                                                }
                                            });
                                        }
                                    }
                                });

                                ui.add_space(6.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_HAMQTH, lang)).strong().size(13.0).color(palette.series(1)));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_HAMQTH_LOGIN, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.hamqth_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label(tr(key::SYNC_HAMQTH_PASSWORD, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.hamqth_password).password(true).desired_width(120.0));
                                    });
                                    if ui.button(tr(key::SYNC_HAMQTH_UPLOAD_BTN, lang)).clicked() {
                                        if app.hamqth_username.is_empty() || app.hamqth_password.is_empty() {
                                            app.online.online_sync_logs.push(tr(key::SYNC_HAMQTH_MISSING_CREDS, lang).to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let user = app.hamqth_username.clone();
                                            let pass = app.hamqth_password.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(100).unwrap_or_default() } else { vec![] };
                                            app.status_message = Some(tr(key::SYNC_HAMQTH_IN_PROGRESS, lang).to_string());
                                            app.online.online_sync_logs.push(Msg::new(key::SYNC_HAMQTH_SENDING).arg("count", qsos.len()).text(lang));
                                            tokio::spawn(async move {
                                                let client = crate::cloud::hamqth::HamQthClient::new(user, pass);
                                                let mut ok_count = 0usize;
                                                let mut last_err = None;
                                                for qso in &qsos {
                                                    match client.upload_qso(qso).await {
                                                        Ok(_) => ok_count += 1,
                                                        Err(e) => last_err = Some(e),
                                                    }
                                                }
                                                if let Some(e) = last_err {
                                                    let _ = tx.send((Msg::new(key::SYNC_HAMQTH_PARTIAL_ERROR).arg("ok_count", ok_count).arg("total_count", qsos.len()).arg("err", e.to_string()).text(lang), false));
                                                } else {
                                                    let _ = tx.send((Msg::new(key::SYNC_HAMQTH_SUCCESS).arg("count", ok_count).text(lang), false));
                                                }
                                            });
                                        }
                                    }
                                });

                                ui.add_space(6.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_CALLBOOK_AGG, lang)).strong().size(13.0).color(palette.series(7)));
                                    ui.separator();
                                    ui.checkbox(&mut app.callbook_cache_enabled, tr(key::SYNC_CACHE_ENABLE, lang));
                                    if app.callbook_cache_enabled {
                                        ui.horizontal(|ui| {
                                            ui.label(tr(key::SYNC_CACHE_TTL_DAYS, lang));
                                            ui.add(egui::DragValue::new(&mut app.callbook_cache_ttl_days).range(1..=365).speed(0.1));
                                        });
                                    }
                                    ui.add_space(4.0);
                                    ui.label(tr(key::SYNC_SOURCE_PRIORITY, lang));
                                    let mut move_up: Option<usize> = None;
                                    let mut move_down: Option<usize> = None;
                                    let len = app.callbook_priority.len();
                                    for (i, src) in app.callbook_priority.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{}. {}", i + 1, src.label()));
                                            if i > 0 && ui.small_button("▲").clicked() {
                                                move_up = Some(i);
                                            }
                                            if i + 1 < len && ui.small_button("▼").clicked() {
                                                move_down = Some(i);
                                            }
                                        });
                                    }
                                    if let Some(i) = move_up {
                                        app.callbook_priority.swap(i, i - 1);
                                    }
                                    if let Some(i) = move_down {
                                        app.callbook_priority.swap(i, i + 1);
                                    }
                                    if ui.button(tr(key::SYNC_RESET_PRIORITY_BTN, lang)).clicked() {
                                        app.callbook_priority = crate::core::callbook::default_callbook_priority();
                                    }
                                });
                            }
                            SyncTab::CloudlogHrdlog => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_CLOUDLOG, lang)).strong().size(13.0).color(palette.accent));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_SERVER_URL, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.cloudlog_url).desired_width(220.0).hint_text("https://cloudlog.twojadomena.pl"));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_CLOUDLOG_API_KEY, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.cloudlog_api_key).password(true).desired_width(220.0));
                                    });
                                    if ui.button(tr(key::SYNC_CLOUDLOG_UPLOAD_BTN, lang)).clicked() {
                                        if app.cloudlog_url.trim().is_empty() || app.cloudlog_api_key.trim().is_empty() {
                                            app.online.online_sync_logs.push(tr(key::SYNC_CLOUDLOG_MISSING_CREDS, lang).to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let url = app.cloudlog_url.clone();
                                            let api_key = app.cloudlog_api_key.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(100).unwrap_or_default() } else { vec![] };
                                            app.status_message = Some(tr(key::SYNC_CLOUDLOG_IN_PROGRESS, lang).to_string());
                                            app.online.online_sync_logs.push(Msg::new(key::SYNC_CLOUDLOG_SENDING).arg("count", qsos.len()).text(lang));
                                            tokio::spawn(async move {
                                                let client = crate::cloud::cloudlog::CloudlogClient::new(&url, api_key, "1".to_string());
                                                let mut ok_count = 0usize;
                                                let mut last_err = None;
                                                for qso in &qsos {
                                                    match client.upload_qso(qso).await {
                                                        Ok(_) => ok_count += 1,
                                                        Err(e) => last_err = Some(e),
                                                    }
                                                }
                                                if let Some(e) = last_err {
                                                    let _ = tx.send((Msg::new(key::SYNC_CLOUDLOG_PARTIAL_ERROR).arg("ok_count", ok_count).arg("total_count", qsos.len()).arg("err", e.to_string()).text(lang), false));
                                                } else {
                                                    let _ = tx.send((Msg::new(key::SYNC_CLOUDLOG_SUCCESS).arg("count", ok_count).text(lang), false));
                                                }
                                            });
                                        }
                                    }
                                });

                                ui.add_space(6.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_HRDLOG, lang)).strong().size(13.0).color(palette.warning));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_CALLSIGN, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.hrdlog_username).desired_width(100.0));
                                        ui.add_space(10.0);
                                        ui.label("Upload Code:");
                                        ui.add(egui::TextEdit::singleline(&mut app.hrdlog_upload_code).password(true).desired_width(120.0));
                                    });
                                    if ui.button(tr(key::SYNC_HRDLOG_UPLOAD_BTN, lang)).clicked() {
                                        if app.hrdlog_username.trim().is_empty() || app.hrdlog_upload_code.trim().is_empty() {
                                            app.online.online_sync_logs.push(tr(key::SYNC_HRDLOG_MISSING_CREDS, lang).to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let call = app.hrdlog_username.clone();
                                            let code = app.hrdlog_upload_code.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(100).unwrap_or_default() } else { vec![] };
                                            app.status_message = Some(tr(key::SYNC_HRDLOG_IN_PROGRESS, lang).to_string());
                                            app.online.online_sync_logs.push(Msg::new(key::SYNC_HRDLOG_SENDING).arg("count", qsos.len()).text(lang));
                                            tokio::spawn(async move {
                                                let client = crate::cloud::hrdlog::HrdlogClient::new(call, code);
                                                let mut ok_count = 0usize;
                                                let mut last_err = None;
                                                for qso in &qsos {
                                                    match client.upload_qso(qso).await {
                                                        Ok(_) => ok_count += 1,
                                                        Err(e) => last_err = Some(e),
                                                    }
                                                }
                                                if let Some(e) = last_err {
                                                    let _ = tx.send((Msg::new(key::SYNC_HRDLOG_PARTIAL_ERROR).arg("ok_count", ok_count).arg("total_count", qsos.len()).arg("err", e.to_string()).text(lang), false));
                                                } else {
                                                    let _ = tx.send((Msg::new(key::SYNC_HRDLOG_SUCCESS).arg("count", ok_count).text(lang), false));
                                                }
                                            });
                                        }
                                    }
                                });
                            }
                            SyncTab::N1mm => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_N1MM, lang)).strong().size(13.0).color(palette.accent));
                                    ui.separator();
                                    if ui.checkbox(&mut app.n1mm_broadcast_enabled, tr(key::SYNC_N1MM_ENABLE, lang)).changed() {
                                        app.save_station_config();
                                    }
                                    ui.label(egui::RichText::new(tr(key::SYNC_N1MM_DESCRIPTION, lang)).small().color(palette.text_muted));
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::SYNC_LISTEN_ADDRESS, lang));
                                        ui.add(egui::TextEdit::singleline(&mut app.n1mm_broadcast_host).desired_width(140.0));
                                        ui.label("Port:");
                                        ui.add(egui::DragValue::new(&mut app.n1mm_broadcast_port).range(1..=65535));
                                    });
                        ui.label(egui::RichText::new(Msg::new(key::SYNC_N1MM_DEFAULT_HINT).arg("port", crate::digital::n1mm::N1MM_DEFAULT_PORT).text(lang)).small().color(palette.text_muted));
                                    ui.add_space(6.0);
                                    if ui.button(tr(key::SYNC_SAVE_SETTINGS_BTN, lang)).clicked() {
                                        app.save_station_config();
                                        app.online.online_sync_logs.push("Zapisano ustawienia emisji N1MM UDP broadcast.".to_string());
                                    }
                                });
                            }
                            SyncTab::Databases => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_REFERENCE_DATA, lang)).strong().size(13.0).color(palette.success));
                                    ui.separator();
                                    ui.label(tr(key::SYNC_REF_DATA_DESC, lang));
                                    ui.add_space(4.0);
                                    if ui.button(egui::RichText::new(tr(key::SYNC_REF_DATA_UPDATE_BTN, lang)).strong()).clicked() {
                                        app.trigger_database_update();
                                        app.online.online_sync_logs.push(tr(key::SYNC_REF_DATA_STARTED, lang).to_string());
                                    }
                                });
                            }
                            SyncTab::Fldigi => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_FLDIGI, lang)).strong().size(13.0).color(palette.accent));
                                    ui.separator();
                                    ui.checkbox(&mut app.fldigi_enabled, tr(key::SYNC_FLDIGI_ENABLE, lang));
                                    ui.horizontal(|ui| {
                                        ui.label("Host FLDigi XML-RPC:");
                                        ui.add(egui::TextEdit::singleline(&mut app.fldigi_host).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label("Port XML-RPC:");
                                        let mut port_str = app.fldigi_port.to_string();
                                        if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0)).changed() {
                                            if let Ok(p) = port_str.trim().parse::<u16>() {
                                                app.fldigi_port = p;
                                            }
                                        }
                                    });
                                    ui.add_space(6.0);
                                    if ui.button(tr(key::SYNC_FLDIGI_TEST_BTN, lang)).clicked() {
                                        let host = app.fldigi_host.clone();
                                        let port = app.fldigi_port;
                                        tokio::spawn(async move {
                                            let client = crate::digital::fldigi::FldigiClient::new(&host, port);
                                            match client.get_version().await {
                                                Ok(v) => {
                                                    log::info!("FLDigi connected: {v}");
                                                }
                                                Err(e) => {
                                                    log::warn!("FLDigi error: {e}");
                                                }
                                            }
                                        });
                    app.fldigi_test_result = Some(Msg::new(key::SYNC_FLDIGI_QUERY_SENT).arg("host", &app.fldigi_host).arg("port", app.fldigi_port).text(lang));
                                    }
                                    if let Some(ref res) = app.fldigi_test_result {
                                        ui.colored_label(palette.accent, res);
                                    }
                                });
                            }
                            SyncTab::PskReporter => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_TAB_PSK_REPORTER, lang)).strong().size(13.0).color(palette.warning));
                                    ui.separator();
                                    ui.checkbox(&mut app.psk_reporter_enabled, tr(key::SYNC_PSK_REPORTER_ENABLE, lang));
                                    ui.label(egui::RichText::new(tr(key::SYNC_PSK_REPORTER_DESC, lang)).small().color(palette.text_muted));
                                    ui.add_space(6.0);
                                    ui.label(Msg::new(key::SYNC_REPORTING_STATION_CALL).arg("call", &app.my_station.callsign).text(lang));
                                    ui.label(format!("Lokator stacji: {}", app.my_station.gridsquare));
                                    ui.add_space(6.0);
                                    if ui.button(tr(key::SYNC_OPEN_PSK_MAP_BTN, lang)).clicked() {
                                        let url = format!("https://pskreporter.info/pskmap.html?callsign={}", app.my_station.callsign);
                                        let _ = open::that(url);
                                    }
                                });
                            }
                            SyncTab::Queue => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_HEADING, lang)).strong().size(13.0).color(palette.accent));
                                    ui.separator();
                                    ui.label(egui::RichText::new(
                                        tr(key::SYNC_QUEUE_DESCRIPTION, lang)
                                    ).small().color(palette.text_muted));
                                    ui.add_space(6.0);

                                    let status = app.upload_scheduler.lock().map(|s| s.status_all()).unwrap_or_default();
                                    egui::Grid::new("upload_queue_grid")
                                        .num_columns(6)
                                        .striped(true)
                                        .spacing([12.0, 4.0])
                                        .show(ui, |ui| {
                                             ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_COL_SERVICE, lang)).strong());
                                             ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_STATUS_PENDING, lang)).strong());
                                             ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_STATUS_SENDING, lang)).strong());
                                             ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_STATUS_SENT, lang)).strong());
                                             ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_STATUS_ERRORS, lang)).strong());
                                             ui.label(egui::RichText::new(tr(key::SYNC_QUEUE_COL_LAST_ERROR, lang)).strong());
                                             ui.end_row();

                                             for svc in crate::cloud::scheduler::UploadService::all() {
                                                 let st = status.get(&svc).cloned().unwrap_or_default();
                                                 ui.label(svc.label());
                                                 ui.label(st.pending.to_string());
                                                 ui.label(st.in_flight.to_string());
                                                 ui.label(st.done.to_string());
                                                 ui.colored_label(
                                                     if st.failed > 0 { palette.danger } else { palette.text_muted },
                                                     st.failed.to_string(),
                                                 );
                                                 ui.label(st.last_error.clone().unwrap_or_else(|| "—".to_string()));
                                                 ui.end_row();
                                             }
                                         });

                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button(tr(key::SYNC_RETRY_FAILED_BTN, lang)).clicked() {
                                            let now = crate::cloud::scheduler::now_unix();
                                            let retry_count = app.upload_scheduler.lock().map_or(0, |mut s| {
                                                let retried = s.retry_failed(now);
                                                s.save_to_disk();
                                                retried
                                            });
                                            if retry_count > 0 {
                        app.online.online_sync_logs.push(Msg::new(key::SYNC_RETRIED_TOAST).arg("count", retry_count).text(lang));
                                            }
                                        }
                                        if ui.button(tr(key::SYNC_CLEAR_SENT_BTN, lang)).clicked() {
                                            let purged_count = app.upload_scheduler.lock().map_or(0, |mut s| {
                                                let purged = s.purge_done();
                                                s.save_to_disk();
                                                purged
                                            });
                                            if purged_count > 0 {
                        app.online.online_sync_logs.push(Msg::new(key::SYNC_PURGED_TOAST).arg("count", purged_count).text(lang));
                                            }
                                        }
                                    });
                                });
                            }
                            SyncTab::Logs => {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(tr(key::SYNC_LOG_TITLE, lang)).strong());
                                    if ui.button(tr(key::SYNC_CLEAR_LOGS_BTN, lang)).clicked() {
                                        app.online.online_sync_logs.clear();
                                    }
                                });
                                ui.separator();
                                ui.group(|ui| {
                                    egui::ScrollArea::vertical()
                                        .max_height(200.0)
                                        .stick_to_bottom(true)
                                        .show(ui, |ui| {
                                            if app.online.online_sync_logs.is_empty() {
                                                ui.label(egui::RichText::new(tr(key::SYNC_NO_OPERATION_IDLE, lang)).italics().color(palette.text_muted));
                                            } else {
                                                for log_entry in &app.online.online_sync_logs {
                                                    ui.label(egui::RichText::new(log_entry).size(11.0).monospace());
                                                }
                                            }
                                        });
                                });
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(tr(key::BTN_SAVE, lang)).strong()).clicked() {
                        app.save_station_config();
                        close_req = true;
                    }
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });

    ctx.data_mut(|d| d.insert_temp(active_tab_id, active_tab));

    if close_req {
        is_open = false;
    }
    app.online.show_online_sync_window = is_open;
}
