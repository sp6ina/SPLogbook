// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno synchronizacji pracy wielostanowiskowej Multi-Op LAN (`src/gui/multi_op_window.rs`).

use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
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
            status: "Off".to_string(),
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
    let lang = app.gui.appearance.language;

    egui::Window::new(tr(key::MULTIOP_TITLE, lang))
        .open(&mut is_open)
        .default_size([520.0, 420.0])
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.vertical(|ui| {
                ui.heading(
                    egui::RichText::new(tr(key::MULTIOP_TITLE, lang))
                        .size(14.0)
                        .color(palette.accent),
                );
                ui.label(tr(key::MULTIOP_DESCRIPTION, lang));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut app.sync_ui.is_host,
                        true,
                        tr(key::MULTIOP_ROLE_HOST, lang),
                    );
                    ui.selectable_value(
                        &mut app.sync_ui.is_host,
                        false,
                        tr(key::MULTIOP_ROLE_CLIENT, lang),
                    );
                });

                ui.add_space(4.0);

                if app.sync_ui.is_host {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(tr(key::MULTIOP_HOST_CONFIG, lang)).strong());
                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_PORT, lang));
                            let mut port_str = app.config.lan_sync_port.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut port_str).desired_width(70.0))
                                .changed()
                                && let Ok(p) = port_str.trim().parse::<u16>()
                            {
                                app.config.lan_sync_port = p;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_SECRET, lang));
                            let mut secret_display = app.config.lan_sync_secret.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut secret_display)
                                        .desired_width(160.0)
                                        .password(false),
                                )
                                .changed()
                            {
                                app.config.lan_sync_secret = secret_display;
                            }
                            if ui
                                .button("🎲")
                                .on_hover_text(tr(key::MULTIOP_GENERATE_SECRET, lang))
                                .clicked()
                            {
                                app.config.lan_sync_secret =
                                    crate::api::server::generate_api_key()[..12].to_string();
                                app.save_station_config();
                            }
                        });
                        ui.label(
                            egui::RichText::new(tr(key::MULTIOP_PASSWORD_REQUIRED_WARNING, lang))
                                .size(11.0)
                                .italics(),
                        );

                        ui.horizontal(|ui| {
                            if app.sync_ui.is_running() {
                                if ui
                                    .button(
                                        egui::RichText::new(tr(key::MULTIOP_STOP_SERVER, lang))
                                            .color(palette.danger)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    app.sync_ui.stop();
                                    app.sync_ui.status =
                                        tr(key::MULTIOP_SERVER_STOPPED_STATUS, lang).to_string();
                                    app.sync_ui
                                        .push_log(tr(key::MULTIOP_LOG_SERVER_STOPPED, lang));
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new(tr(key::MULTIOP_START_SERVER, lang))
                                        .color(palette.success)
                                        .strong(),
                                )
                                .clicked()
                            {
                                let journal = app.active_journal.id.clone();
                                let config = SyncConfig {
                                    role: SyncRole::Host {
                                        port: app.config.lan_sync_port,
                                    },
                                    secret: app.config.lan_sync_secret.clone(),
                                    station: app.config.station.callsign.clone(),
                                    journal,
                                };

                                match SyncService::start(
                                    config,
                                    std::sync::Arc::new(app.services.ingestor.clone()),
                                    app.sync_ui.events_tx.clone(),
                                ) {
                                    Ok(srv) => {
                                        app.sync_ui.service = Some(srv);
                                        app.sync_ui.status =
                                            Msg::new(key::MULTIOP_SERVER_LISTENING)
                                                .arg("port", app.config.lan_sync_port)
                                                .text(lang);
                                        app.sync_ui.push_log(
                                            Msg::new(key::MULTIOP_LOG_SERVER_STARTED)
                                                .arg("port", app.config.lan_sync_port)
                                                .text(lang),
                                        );
                                    }
                                    Err(err) => {
                                        let err_str = err.to_string();
                                        app.sync_ui.status = Msg::new(key::MULTIOP_ERROR_PREFIX)
                                            .arg("err", &err_str)
                                            .text(lang);
                                        app.sync_ui.push_log(
                                            Msg::new(key::MULTIOP_SERVER_START_ERROR_LOG)
                                                .arg("err", &err_str)
                                                .text(lang),
                                        );
                                    }
                                }
                            }
                        });
                    });
                } else {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::MULTIOP_CONNECT_TO_HOST, lang)).strong(),
                        );
                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_HOST_IP, lang));
                            ui.add(
                                egui::TextEdit::singleline(&mut app.config.lan_sync_server_ip)
                                    .desired_width(120.0),
                            );
                            ui.label(tr(key::MULTIOP_PORT, lang));
                            let mut port_str = app.config.lan_sync_port.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                                .changed()
                                && let Ok(p) = port_str.trim().parse::<u16>()
                            {
                                app.config.lan_sync_port = p;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(tr(key::MULTIOP_SECRET, lang));
                            let mut secret_display = app.config.lan_sync_secret.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut secret_display)
                                        .desired_width(160.0),
                                )
                                .changed()
                            {
                                app.config.lan_sync_secret = secret_display;
                            }
                        });

                        ui.horizontal(|ui| {
                            if app.sync_ui.is_running() {
                                if ui
                                    .button(
                                        egui::RichText::new(tr(key::MULTIOP_DISCONNECT, lang))
                                            .color(palette.danger)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    app.sync_ui.stop();
                                    app.sync_ui.status =
                                        tr(key::MULTIOP_DISCONNECTED_STATUS, lang).to_string();
                                    app.sync_ui
                                        .push_log(tr(key::MULTIOP_DISCONNECTED_STATUS, lang));
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new(tr(key::MULTIOP_CONNECT, lang))
                                        .color(palette.success)
                                        .strong(),
                                )
                                .clicked()
                            {
                                let journal = app.active_journal.id.clone();
                                let config = SyncConfig {
                                    role: SyncRole::Peer {
                                        host: app.config.lan_sync_server_ip.clone(),
                                        port: app.config.lan_sync_port,
                                    },
                                    secret: app.config.lan_sync_secret.clone(),
                                    station: app.config.station.callsign.clone(),
                                    journal,
                                };

                                match SyncService::start(
                                    config,
                                    std::sync::Arc::new(app.services.ingestor.clone()),
                                    app.sync_ui.events_tx.clone(),
                                ) {
                                    Ok(srv) => {
                                        app.sync_ui.service = Some(srv);
                                        app.sync_ui.status =
                                            Msg::new(key::MULTIOP_CONNECTING_STATUS)
                                                .arg("host", &app.config.lan_sync_server_ip)
                                                .arg("port", app.config.lan_sync_port)
                                                .text(lang);
                                        app.sync_ui.push_log(
                                            Msg::new(key::MULTIOP_CLIENT_STARTED_LOG)
                                                .arg("host", &app.config.lan_sync_server_ip)
                                                .arg("port", app.config.lan_sync_port)
                                                .text(lang),
                                        );
                                    }
                                    Err(err) => {
                                        let err_str = err.to_string();
                                        app.sync_ui.status = Msg::new(key::MULTIOP_ERROR_PREFIX)
                                            .arg("err", &err_str)
                                            .text(lang);
                                        app.sync_ui.push_log(
                                            Msg::new(key::MULTIOP_CLIENT_START_ERROR_LOG)
                                                .arg("err", &err_str)
                                                .text(lang),
                                        );
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
                        .color(palette.accent),
                );

                ui.separator();
                ui.label(egui::RichText::new(tr(key::MULTIOP_LOG_TITLE, lang)).strong());
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        if app.sync_ui.log.is_empty() {
                            ui.label(tr(key::MULTIOP_NO_EVENTS, lang));
                        } else {
                            for entry in app.sync_ui.log.iter().rev() {
                                ui.label(egui::RichText::new(entry).size(11.0).monospace());
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new(tr(key::BTN_SAVE, lang)).strong())
                        .clicked()
                    {
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
