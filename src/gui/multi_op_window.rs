// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno synchronizacji pracy wielostanowiskowej Multi-Op LAN (`src/gui/multi_op_window.rs`).

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::sync::{SyncConfig, SyncEvent, SyncRole, SyncService};
use eframe::egui;

/// Stan interfejsu użytkownika synchronizacji Multi-Op.
pub struct SyncUiState {
    pub show: bool,
    pub is_host: bool,
    pub status: String,
    pub log: Vec<String>,
    pub service: Option<SyncService>,
    pub events_tx: std::sync::mpsc::Sender<SyncEvent>,
    pub events_rx: std::sync::mpsc::Receiver<SyncEvent>,
}

impl Default for SyncUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncUiState {
    pub fn new() -> Self {
        let (events_tx, events_rx) = std::sync::mpsc::channel();
        Self {
            show: false,
            is_host: true,
            status: "Wyłączony".to_string(),
            log: Vec::new(),
            service: None,
            events_tx,
            events_rx,
        }
    }

    /// Dopisuje wpis do dziennika zdarzeń GUI, ograniczając historię do 500 wpisów.
    pub fn push_log(&mut self, entry: impl Into<String>) {
        self.log.push(entry.into());
        if self.log.len() > 500 {
            let excess = self.log.len() - 500;
            self.log.drain(0..excess);
        }
    }

    /// Zatrzymuje działający serwis synchronizacji.
    pub fn stop(&mut self) {
        if let Some(mut srv) = self.service.take() {
            srv.stop();
        }
    }

    /// Sprawdza, czy serwis synchronizacji jest aktywny.
    pub fn is_running(&self) -> bool {
        self.service.is_some()
    }
}

/// Rysuje okno synchronizacji pracy wielostanowiskowej Multi-Op LAN.
pub fn render_multi_op_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.sync_ui.show {
        return;
    }

    let mut is_open = app.sync_ui.show;
    let mut close_req = false;
    let lang = app.current_language;

    egui::Window::new(tr(key::MULTIOP_TITLE, lang))
        .open(&mut is_open)
        .default_size([520.0, 420.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading(
                    egui::RichText::new(tr(key::MULTIOP_TITLE, lang))
                        .size(14.0)
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );
                ui.label("Umożliwia automatyczną wymianę nowo dodanych QSO pomiędzy stanowiskami w klubie lub w zawodach.");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.sync_ui.is_host, true, tr(key::MULTIOP_ROLE_HOST, lang));
                    ui.selectable_value(&mut app.sync_ui.is_host, false, tr(key::MULTIOP_ROLE_CLIENT, lang));
                });

                ui.add_space(4.0);

                if app.sync_ui.is_host {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Konfiguracja Hosta:").strong());
                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_PORT, lang));
                            let mut port_str = app.lan_sync_port.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut port_str).desired_width(70.0))
                                .changed()
                                && let Ok(p) = port_str.trim().parse::<u16>()
                            {
                                app.lan_sync_port = p;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_SECRET, lang));
                            let mut secret_display = app.lan_sync_secret.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut secret_display)
                                        .desired_width(160.0)
                                        .password(false),
                                )
                                .changed()
                            {
                                app.lan_sync_secret = secret_display;
                            }
                            if ui.button("🎲").on_hover_text(tr(key::MULTIOP_GENERATE_SECRET, lang)).clicked() {
                                app.lan_sync_secret =
                                    crate::api::server::generate_api_key()[..12].to_string();
                                app.save_station_config();
                            }
                        });
                        ui.label(
                            egui::RichText::new("⚠ Podaj to samo hasło na stacji klienckiej. Hasło jest wymagane — serwer nie wystartuje bez niego.")
                                .size(11.0)
                                .italics(),
                        );

                        ui.horizontal(|ui| {
                            if app.sync_ui.is_running() {
                                if ui
                                    .button(
                                        egui::RichText::new(tr(key::MULTIOP_STOP_SERVER, lang))
                                            .color(egui::Color32::from_rgb(239, 68, 68))
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    app.sync_ui.stop();
                                    app.sync_ui.status = "Serwer zatrzymany".to_string();
                                    app.sync_ui.push_log("Zatrzymano serwer Multi-Op LAN.");
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new(tr(key::MULTIOP_START_SERVER, lang))
                                        .color(egui::Color32::from_rgb(34, 197, 94))
                                        .strong(),
                                )
                                .clicked()
                            {
                                let journal = app.active_journal.id.clone();
                                let config = SyncConfig {
                                    role: SyncRole::Host {
                                        port: app.lan_sync_port,
                                    },
                                    secret: app.lan_sync_secret.clone(),
                                    station: app.my_station.callsign.clone(),
                                    journal,
                                };

                                match SyncService::start(
                                    config,
                                    std::sync::Arc::new(app.qso_ingestor.clone()),
                                    app.sync_ui.events_tx.clone(),
                                ) {
                                    Ok(srv) => {
                                        app.sync_ui.service = Some(srv);
                                        app.sync_ui.status = format!("Serwer nasłuchuje na porcie {}", app.lan_sync_port);
                                        app.sync_ui.push_log(format!("Uruchomiono serwer Multi-Op LAN na porcie {}", app.lan_sync_port));
                                    }
                                    Err(err) => {
                                        let err_str = err.to_string();
                                        app.sync_ui.status = format!("Błąd: {err_str}");
                                        app.sync_ui.push_log(format!("Odmowa uruchomienia serwera: {err_str}"));
                                    }
                                }
                            }
                        });
                    });
                } else {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Połączenie ze stacją główną:").strong());
                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_HOST_IP, lang));
                            ui.add(
                                egui::TextEdit::singleline(&mut app.lan_sync_server_ip)
                                    .desired_width(120.0),
                            );
                            ui.label(tr(key::MULTIOP_PORT, lang));
                            let mut port_str = app.lan_sync_port.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                                .changed()
                                && let Ok(p) = port_str.trim().parse::<u16>()
                            {
                                app.lan_sync_port = p;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_SECRET, lang));
                            let mut secret_display = app.lan_sync_secret.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut secret_display)
                                        .desired_width(160.0),
                                )
                                .changed()
                            {
                                app.lan_sync_secret = secret_display;
                            }
                        });

                        ui.horizontal(|ui| {
                            if app.sync_ui.is_running() {
                                if ui
                                    .button(
                                        egui::RichText::new(tr(key::MULTIOP_DISCONNECT, lang))
                                            .color(egui::Color32::from_rgb(239, 68, 68))
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    app.sync_ui.stop();
                                    app.sync_ui.status = "Rozłączono ze stacją główną".to_string();
                                    app.sync_ui.push_log("Rozłączono ze stacją główną.");
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new(tr(key::MULTIOP_CONNECT, lang))
                                        .color(egui::Color32::from_rgb(34, 197, 94))
                                        .strong(),
                                )
                                .clicked()
                            {
                                let journal = app.active_journal.id.clone();
                                let config = SyncConfig {
                                    role: SyncRole::Peer {
                                        host: app.lan_sync_server_ip.clone(),
                                        port: app.lan_sync_port,
                                    },
                                    secret: app.lan_sync_secret.clone(),
                                    station: app.my_station.callsign.clone(),
                                    journal,
                                };

                                match SyncService::start(
                                    config,
                                    std::sync::Arc::new(app.qso_ingestor.clone()),
                                    app.sync_ui.events_tx.clone(),
                                ) {
                                    Ok(srv) => {
                                        app.sync_ui.service = Some(srv);
                                        app.sync_ui.status = format!("Łączenie z {}:{}...", app.lan_sync_server_ip, app.lan_sync_port);
                                        app.sync_ui.push_log(format!("Uruchomiono klienta Multi-Op: {}:{}", app.lan_sync_server_ip, app.lan_sync_port));
                                    }
                                    Err(err) => {
                                        let err_str = err.to_string();
                                        app.sync_ui.status = format!("Błąd: {err_str}");
                                        app.sync_ui.push_log(format!("Błąd uruchomienia klienta: {err_str}"));
                                    }
                                }
                            }
                        });
                    });
                }

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("Status: {}", app.sync_ui.status))
                        .strong()
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );

                ui.separator();
                ui.label(egui::RichText::new(tr(key::MULTIOP_LOG_TITLE, lang)).strong());
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    if app.sync_ui.log.is_empty() {
                        ui.label("Brak zdarzeń w sieci Multi-Op.");
                    } else {
                        for entry in app.sync_ui.log.iter().rev() {
                            ui.label(egui::RichText::new(entry).size(11.0).monospace());
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

    if close_req {
        is_open = false;
    }
    app.sync_ui.show = is_open;
    app.show_multi_op_window = is_open;
}
