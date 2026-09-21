// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::lotw::{detect_tqsl_path, export_and_sign_tqsl};
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
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
    Databases,
    Logs,
}

/// Kompaktowe okno dialogowe synchronizacji z serwisami online i ARRL LoTW
pub fn render_online_sync_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_online_sync_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_online_sync_window;
    let mut close_req = false;

    let screen_height = ctx.screen_rect().height();
    let max_h = (screen_height * 0.82).min(520.0);

    let active_tab_id = egui::Id::new("online_sync_active_tab");
    let mut active_tab = ctx.data_mut(|d| d.get_temp::<SyncTab>(active_tab_id)).unwrap_or(SyncTab::Lotw);

    egui::Window::new(format!("🌐 {}", tr("sync.title", lang)))
        .open(&mut is_open)
        .default_size([640.0, 420.0])
        .max_height(max_h)
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Nagłówek i status
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Centrum Synchronizacji Logów Online & LoTW").size(15.0).color(egui::Color32::from_rgb(56, 189, 248)));
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
                    ui.selectable_value(&mut active_tab, SyncTab::Databases, "🔄 Bazy");
                    ui.selectable_value(&mut active_tab, SyncTab::Logs, format!("📋 Logi ({})", app.online_sync_logs.len()));
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
                                    ui.label(egui::RichText::new("🏆 ARRL Logbook of the World (LoTW)").strong().size(13.0).color(egui::Color32::from_rgb(250, 204, 21)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Ścieżka do tqsl.exe:");
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_tqsl_path).desired_width(260.0));
                                        if ui.button("🔍 Wykryj").clicked() {
                                            if let Some(detected) = detect_tqsl_path() {
                                                app.lotw_tqsl_path = detected.to_string_lossy().to_string();
                                                app.online_sync_logs.push(format!("Wykryto TQSL w: {}", app.lotw_tqsl_path));
                                            } else {
                                                app.online_sync_logs.push("Nie wykryto TQSL w domyślnych ścieżkach Program Files.".to_string());
                                            }
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Nazwa lokalizacji (TQSL Station Location):");
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_station_name).desired_width(180.0));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Login LoTW:");
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label("Hasło:");
                                        ui.add(egui::TextEdit::singleline(&mut app.lotw_password).password(true).desired_width(120.0));
                                    });

                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button(egui::RichText::new(format!("📤 {}", tr("sync.lotw_tqsl", lang))).strong()).clicked() {
                                            let qsos = if let Ok(db) = app.log_db.lock() {
                                                db.get_recent_qsos(1000).unwrap_or_default()
                                            } else {
                                                vec![]
                                            };
                                            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                            let tqsl_path = app.lotw_tqsl_path.clone();
                                            let station_loc = app.lotw_station_name.clone();

                                            match export_and_sign_tqsl(&tqsl_path, &station_loc, &adif) {
                                                Ok(msg) => {
                                                    app.online_sync_logs.push(format!("[LoTW SUKCES] Wyeksportowano i podpisano {} łączności: {}", qsos.len(), msg));
                                                    app.status_message = Some("Łączności przesłane do LoTW pomyślnie.".to_string());
                                                }
                                                Err(e) => {
                                                    app.online_sync_logs.push(format!("[LoTW BŁĄD] {}", e));
                                                    app.status_message = Some(format!("Błąd LoTW: {}", e));
                                                }
                                            }
                                        }

                                        if ui.button(egui::RichText::new(format!("📥 {}", tr("sync.lotw_download", lang)))).clicked() {
                                            if app.lotw_username.is_empty() || app.lotw_password.is_empty() {
                                                app.online_sync_logs.push("[LoTW] Błąd: Brak loginu lub hasła do LoTW.".to_string());
                                            } else {
                                                let tx = app.sync_log_tx.clone();
                                                let user = app.lotw_username.clone();
                                                let pass = app.lotw_password.clone();
                                                let log_db = app.log_db.clone();
                                                tokio::spawn(async move {
                                                    let _ = tx.send(("[LoTW] Pobieranie raportu potwierdzeń z serwera ARRL LoTW...".to_string(), false));
                                                    match crate::cloud::lotw::download_lotw_report(&user, &pass, None).await {
                                                        Ok(adif) => {
                                                            let confs = crate::cloud::lotw::parse_lotw_confirmations(&adif);
                                                            let count = confs.len();
                                                            let mut updated = 0;
                                                            if let Ok(db) = log_db.lock() {
                                                                for c in confs {
                                                                    if let Ok(n) = db.mark_lotw_confirmed(&c.callsign, &c.band, &c.mode, &c.qso_date, &c.qsl_rdate) {
                                                                        updated += n;
                                                                    }
                                                                }
                                                            }
                                                            let _ = tx.send((format!("[LoTW SUKCES] Odebrano {} potwierdzeń, zaktualizowano w bazie: {}", count, updated), true));
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send((format!("[LoTW BŁĄD] {}", e), false));
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
                                    ui.label(egui::RichText::new("📨 Serwis eQSL.cc").strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Użytkownik eQSL:");
                                        ui.add(egui::TextEdit::singleline(&mut app.eqsl_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label("Hasło eQSL:");
                                        ui.add(egui::TextEdit::singleline(&mut app.eqsl_password).password(true).desired_width(120.0));
                                    });

                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button(format!("📤 {}", tr("sync.eqsl_upload", lang))).clicked() {
                                            if app.eqsl_username.is_empty() || app.eqsl_password.is_empty() {
                                                app.online_sync_logs.push("[eQSL] Błąd: Brak loginu lub hasła do eQSL.cc.".to_string());
                                            } else {
                                                let tx = app.sync_log_tx.clone();
                                                let user = app.eqsl_username.clone();
                                                let pass = app.eqsl_password.clone();
                                                let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(1000).unwrap_or_default() } else { vec![] };
                                                let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                                tokio::spawn(async move {
                                                    let client = crate::cloud::eqsl::EqslClient::new(user, pass);
                                                    let _ = tx.send((format!("[eQSL] Wysyłanie pakietu {} łączności do eQSL.cc...", qsos.len()), false));
                                                    match client.upload_adif(&adif).await {
                                                        Ok(msg) => { let _ = tx.send((format!("[eQSL SUKCES] {}", msg), false)); }
                                                        Err(e) => { let _ = tx.send((format!("[eQSL BŁĄD] {}", e), false)); }
                                                    }
                                                });
                                            }
                                        }
                                        if ui.button(format!("📥 {}", tr("sync.eqsl_download", lang))).clicked() {
                                            if app.eqsl_username.is_empty() || app.eqsl_password.is_empty() {
                                                app.online_sync_logs.push("[eQSL] Błąd: Brak loginu lub hasła do eQSL.cc.".to_string());
                                            } else {
                                                let tx = app.sync_log_tx.clone();
                                                let user = app.eqsl_username.clone();
                                                let pass = app.eqsl_password.clone();
                                                let log_db = app.log_db.clone();
                                                tokio::spawn(async move {
                                                    let client = crate::cloud::eqsl::EqslClient::new(user, pass);
                                                    let _ = tx.send(("[eQSL] Pobieranie skrzynki odbiorczej (Inbox ADIF)...".to_string(), false));
                                                    match client.download_inbox_adif().await {
                                                        Ok(adif) => {
                                                            let qsos = crate::core::adif::parse_adif(&adif);
                                                            let mut updated = 0;
                                                            if let Ok(db) = log_db.lock() {
                                                                for q in &qsos {
                                                                    let rdate = q.eqsl_qslrdate.as_deref().unwrap_or(&q.qso_date);
                                                                    if let Ok(n) = db.mark_eqsl_confirmed(&q.callsign, &q.band, &q.mode, &q.qso_date, rdate) {
                                                                        updated += n;
                                                                    }
                                                                }
                                                            }
                                                            let _ = tx.send((format!("[eQSL SUKCES] Pobrano {} potwierdzeń, zaktualizowano w bazie: {}", qsos.len(), updated), true));
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send((format!("[eQSL BŁĄD] {}", e), false));
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
                                    ui.label(egui::RichText::new("🌐 Club Log (DX Cluster & Super Log)").strong().size(13.0).color(egui::Color32::from_rgb(134, 239, 172)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Znak stacji:");
                                        ui.add(egui::TextEdit::singleline(&mut app.clublog_callsign).desired_width(100.0));
                                        ui.add_space(10.0);
                                        ui.label("Email:");
                                        ui.add(egui::TextEdit::singleline(&mut app.clublog_email).desired_width(160.0));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Hasło:");
                                        ui.add(egui::TextEdit::singleline(&mut app.clublog_password).password(true).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label("API Key (opcjonalny):");
                                        ui.add(egui::TextEdit::singleline(&mut app.clublog_api_key).desired_width(140.0));
                                    });

                                    ui.add_space(4.0);
                                    ui.checkbox(&mut app.live_auto_upload_clublog, "Automatycznie wysyłaj każde nowo dodane QSO w czasie rzeczywistym");

                                    ui.add_space(6.0);
                                    if ui.button(format!("📤 {}", tr("sync.clublog_upload", lang))).clicked() {
                                        if app.clublog_callsign.is_empty() || app.clublog_password.is_empty() {
                                            app.online_sync_logs.push("[Club Log] Błąd: Brak znaku lub hasła do Club Log.".to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let call = app.clublog_callsign.clone();
                                            let email = app.clublog_email.clone();
                                            let pass = app.clublog_password.clone();
                                            let api = app.clublog_api_key.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(5000).unwrap_or_default() } else { vec![] };
                                            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                            tokio::spawn(async move {
                                                let client = crate::cloud::clublog::ClubLogClient::new();
                                                let _ = tx.send((format!("[Club Log] Wysyłanie dziennika ADIF ({} łączności)...", qsos.len()), false));
                                                match client.upload_adif(&call, &email, &pass, &api, &adif).await {
                                                    Ok(msg) => { let _ = tx.send((format!("[Club Log SUKCES] {}", msg), false)); }
                                                    Err(e) => { let _ = tx.send((format!("[Club Log BŁĄD] {}", e), false)); }
                                                }
                                            });
                                        }
                                    }
                                });
                            }
                            SyncTab::QrzHamqth => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("📖 QRZ.COM Callbook & Logbook").strong().size(13.0).color(egui::Color32::from_rgb(251, 146, 60)));
                                    ui.separator();

                                    ui.checkbox(&mut app.qrz_auto_lookup, tr("sync.auto_lookup", lang));
                                    ui.checkbox(&mut app.live_auto_upload_qrz, "Automatycznie przesyłaj łączności do logbooka QRZ.com");

                                    ui.horizontal(|ui| {
                                        ui.label("Login QRZ.com:");
                                        ui.add(egui::TextEdit::singleline(&mut app.qrz_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label("Hasło QRZ.com:");
                                        ui.add(egui::TextEdit::singleline(&mut app.qrz_password).password(true).desired_width(120.0));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Klucz API Logbooka QRZ:");
                                        ui.add(egui::TextEdit::singleline(&mut app.qrz_api_key).desired_width(220.0));
                                    });

                                    ui.add_space(4.0);
                                    if ui.button(format!("📤 {}", tr("sync.qrz_upload", lang))).clicked() {
                                        if app.qrz_api_key.is_empty() {
                                            app.online_sync_logs.push("[QRZ.com] Błąd: Brak klucza API logbooka QRZ.".to_string());
                                        } else {
                                            let tx = app.sync_log_tx.clone();
                                            let api = app.qrz_api_key.clone();
                                            let qsos = if let Ok(db) = app.log_db.lock() { db.get_recent_qsos(1000).unwrap_or_default() } else { vec![] };
                                            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &app.my_station.callsign);
                                            tokio::spawn(async move {
                                                let _ = tx.send((format!("[QRZ.com] Wysyłanie dziennika ({} łączności)...", qsos.len()), false));
                                                match crate::cloud::qrz::QrzClient::upload_to_logbook(&api, &adif).await {
                                                    Ok(msg) => { let _ = tx.send((format!("[QRZ.com SUKCES] {}", msg), false)); }
                                                    Err(e) => { let _ = tx.send((format!("[QRZ.com BŁĄD] {}", e), false)); }
                                                }
                                            });
                                        }
                                    }
                                });

                                ui.add_space(6.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("🌍 HamQTH (OK1RR Logbook)").strong().size(13.0).color(egui::Color32::from_rgb(147, 197, 253)));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label("Login HamQTH:");
                                        ui.add(egui::TextEdit::singleline(&mut app.hamqth_username).desired_width(120.0));
                                        ui.add_space(10.0);
                                        ui.label("Hasło HamQTH:");
                                        ui.add(egui::TextEdit::singleline(&mut app.hamqth_password).password(true).desired_width(120.0));
                                    });
                                    if ui.button("📤 Wyślij łączności do HamQTH").clicked() {
                                        app.online_sync_logs.push("Wysyłanie danych do HamQTH.com...".to_string());
                                        app.status_message = Some("Wysyłka do HamQTH w toku...".to_string());
                                    }
                                });
                            }
                            SyncTab::CloudlogHrdlog => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("☁ Cloudlog (REST API)").strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label("Adres URL serwera:");
                                        ui.add(egui::TextEdit::singleline(&mut app.cloudlog_url).desired_width(220.0).hint_text("https://cloudlog.twojadomena.pl"));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Klucz API Cloudlog:");
                                        ui.add(egui::TextEdit::singleline(&mut app.cloudlog_api_key).password(true).desired_width(220.0));
                                    });
                                    if ui.button("📤 Wyślij łączności do Cloudlog").clicked() {
                                        app.online_sync_logs.push("Wysyłanie łączności do Cloudlog REST API...".to_string());
                                        app.status_message = Some("Wysyłka do Cloudlog w toku...".to_string());
                                    }
                                });

                                ui.add_space(6.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("📻 HRDLog.net").strong().size(13.0).color(egui::Color32::from_rgb(253, 224, 71)));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label("Znak stacji:");
                                        ui.add(egui::TextEdit::singleline(&mut app.hrdlog_username).desired_width(100.0));
                                        ui.add_space(10.0);
                                        ui.label("Upload Code:");
                                        ui.add(egui::TextEdit::singleline(&mut app.hrdlog_upload_code).password(true).desired_width(120.0));
                                    });
                                    if ui.button("📤 Wyślij łączności do HRDLog.net").clicked() {
                                        app.online_sync_logs.push("Wysyłanie danych do HRDLog.net...".to_string());
                                        app.status_message = Some("Wysyłka do HRDLog.net w toku...".to_string());
                                    }
                                });
                            }
                            SyncTab::Databases => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("🔄 Aktualizacja Baz Danych z Sieci (cty.dat, SCP, LoTW Users)").strong().size(13.0).color(egui::Color32::from_rgb(134, 239, 172)));
                                    ui.separator();
                                    ui.label("Pobiera najnowsze pliki z country-files.com, supercheckpartial.com i arrl.org:");
                                    ui.add_space(4.0);
                                    if ui.button(egui::RichText::new("⬇ Aktualizuj bazy referencyjne online").strong()).clicked() {
                                        app.trigger_database_update();
                                        app.online_sync_logs.push("Uruchomiono pobieranie cty.dat, MASTER.SCP oraz bazy aktywności LoTW...".to_string());
                                    }
                                });
                            }
                            SyncTab::Fldigi => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("💻 Integracja z FLDigi (Cyfra XML-RPC)").strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                                    ui.separator();
                                    ui.checkbox(&mut app.fldigi_enabled, "Włącz integrację z FLDigi (odpytywanie częstotliwości i logu)");
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
                                    if ui.button("⚡ Test Połączenia XML-RPC z FLDigi").clicked() {
                                        let host = app.fldigi_host.clone();
                                        let port = app.fldigi_port;
                                        tokio::spawn(async move {
                                            let client = crate::digital::fldigi::FldigiClient::new(&host, port);
                                            match client.get_version().await {
                                                Ok(v) => {
                                                    log::info!("FLDigi połączono: {}", v);
                                                }
                                                Err(e) => {
                                                    log::warn!("FLDigi błąd: {}", e);
                                                }
                                            }
                                        });
                                        app.fldigi_test_result = Some(format!("Wysłano zapytanie do FLDigi na {}:{}", app.fldigi_host, app.fldigi_port));
                                    }
                                    if let Some(ref res) = app.fldigi_test_result {
                                        ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
                                    }
                                });
                            }
                            SyncTab::PskReporter => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("📡 PSK Reporter (pskreporter.info)").strong().size(13.0).color(egui::Color32::from_rgb(250, 204, 21)));
                                    ui.separator();
                                    ui.checkbox(&mut app.psk_reporter_enabled, "Automatycznie raportuj odbierane stacje do PSK Reporter");
                                    ui.label(egui::RichText::new("Gdy opcja jest włączona, każde nowo zapisane QSO jest automatycznie raportowane w formacie XML do serwera pskreporter.info z Twoim znakiem i lokatorem.").small().color(egui::Color32::GRAY));
                                    ui.add_space(6.0);
                                    ui.label(format!("Znak stacji raportującej: {}", app.my_station.callsign));
                                    ui.label(format!("Lokator stacji: {}", app.my_station.gridsquare));
                                    ui.add_space(6.0);
                                    if ui.button("🔗 Otwórz Mapę PSK Reporter w Przeglądarce").clicked() {
                                        let url = format!("https://pskreporter.info/pskmap.html?callsign={}", app.my_station.callsign);
                                        let _ = open::that(url);
                                    }
                                });
                            }
                            SyncTab::Logs => {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("📋 Dziennik zdarzeń synchronizacji:").strong());
                                    if ui.button("Wyczyść logi").clicked() {
                                        app.online_sync_logs.clear();
                                    }
                                });
                                ui.separator();
                                ui.group(|ui| {
                                    egui::ScrollArea::vertical()
                                        .max_height(200.0)
                                        .stick_to_bottom(true)
                                        .show(ui, |ui| {
                                            if app.online_sync_logs.is_empty() {
                                                ui.label(egui::RichText::new("Brak operacji. Wybierz synchronizację powyżej.").italics().color(egui::Color32::from_rgb(148, 163, 184)));
                                            } else {
                                                for log_entry in &app.online_sync_logs {
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

    ctx.data_mut(|d| d.insert_temp(active_tab_id, active_tab));

    if close_req {
        is_open = false;
    }
    app.show_online_sync_window = is_open;
}
