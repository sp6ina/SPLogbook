// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::rig_models::search_rig_models_by_mfg;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_cat_settings_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_cat_settings_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_cat_settings_window;
    let mut close_req = false;

    egui::Window::new(egui::RichText::new(format!("📻 {}", tr("cat.settings_title", lang))).size(14.0).strong())
        .open(&mut is_open)
        .default_size([650.0, 560.0])
        .min_size([480.0, 420.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if app.cat_connected {
                        ui.colored_label(egui::Color32::from_rgb(34, 197, 94), tr("cat.connected", lang));
                        if ui.button(tr("cat.disconnect_radio", lang)).clicked() {
                            app.stop_cat_service();
                        }
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(239, 68, 68), tr("cat.disconnected", lang));
                        if ui.button(tr("cat.connect_radio", lang)).clicked() {
                            app.start_cat_service();
                        }
                    }
                });

                ui.separator();

                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr("cat_settings.step1_title", lang)).strong().color(egui::Color32::from_rgb(250, 204, 21)));
                    ui.add_space(2.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(tr("cat_settings.filter_mfg", lang)).size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                        let top_mfgs = ["Wszystkie", "Icom", "Yaesu", "Kenwood", "Elecraft", "Xiegu", "FlexRadio", "SunSDR", "Hamlib"];
                        for m in top_mfgs {
                            let is_sel = if m == "SunSDR" {
                                app.cat_backend == "tci"
                            } else {
                                app.cat_mfg_selected == m && app.cat_backend != "tci"
                            };

                            let display_mfg = if m == "Wszystkie" { tr("cat_settings.all_mfgs", lang) } else { m };
                            if ui.selectable_label(is_sel, display_mfg).clicked() {
                                if m == "SunSDR" {
                                    app.cat_backend = "tci".to_string();
                                    app.cat_conn_type = "tci".to_string();
                                    app.cat_rig_model = "Expert Electronics SunSDR / TCI".to_string();
                                } else {
                                    app.cat_backend = "hamlib".to_string();
                                    app.cat_mfg_selected = m.to_string();
                                    if app.cat_conn_type == "tci" {
                                        app.cat_conn_type = "serial".to_string();
                                    }
                                }
                            }
                        }
                    });

                    if app.cat_backend != "tci" {
                        ui.horizontal(|ui| {
                            ui.label(tr("cat_settings.search_model", lang));
                            ui.add(egui::TextEdit::singleline(&mut app.cat_model_search).hint_text("7300, FT-891, TS-590, G90, K4"));
                        });

                        let mfg_query = if app.cat_mfg_selected == "Wszystkie" { "" } else { &app.cat_mfg_selected };
                        let filtered_rigs = search_rig_models_by_mfg(mfg_query, &app.cat_model_search);

                        egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                            for rig in filtered_rigs {
                                let is_selected = app.cat_rig_id == rig.rig_id;
                                let label_text = format!("[ID {:4}] {:15} - {:20} ({})", rig.rig_id, rig.mfg, rig.model, rig.status);
                                if ui.selectable_label(is_selected, label_text).clicked() {
                                    app.cat_rig_id = rig.rig_id;
                                    app.cat_rig_model = format!("{} {}", rig.mfg, rig.model);
                                    app.cat_baud_rate = rig.default_baud;
                                    app.cat_test_result = Some(format!("{} ({} bps)", app.cat_rig_model, rig.default_baud));
                                }
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", tr("cat.rig_model", lang))).strong());
                        ui.colored_label(egui::Color32::from_rgb(56, 189, 248), format!("{} (ID: {})", app.cat_rig_model, app.cat_rig_id));
                    });
                });

                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr("cat_settings.step2_title", lang)).strong().color(egui::Color32::from_rgb(250, 204, 21)));
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label(tr("cat_settings.conn_type", lang));
                        if ui.selectable_value(&mut app.cat_conn_type, "serial".to_string(), tr("cat_settings.conn_serial", lang)).clicked() {
                            app.cat_backend = "hamlib".to_string();
                        }
                        if ui.selectable_value(&mut app.cat_conn_type, "tcp".to_string(), tr("cat_settings.conn_tcp", lang)).clicked() {
                            app.cat_backend = "hamlib".to_string();
                        }
                        if ui.selectable_value(&mut app.cat_conn_type, "tci".to_string(), tr("cat_settings.conn_tci", lang)).clicked() {
                            app.cat_backend = "tci".to_string();
                        }
                    });

                    ui.separator();

                    if app.cat_conn_type == "tci" {
                        ui.horizontal(|ui| {
                            ui.label("Host TCI:");
                            ui.add(egui::TextEdit::singleline(&mut app.tci_host).desired_width(120.0));
                            ui.add_space(10.0);
                            ui.label("Port TCI:");
                            let mut port_str = app.tci_port.to_string();
                            if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0)).changed() {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    app.tci_port = p;
                                }
                            }
                            if ui.button("⚡ Test TCI").clicked() {
                                let host = app.tci_host.clone();
                                let port = app.tci_port;
                                match std::net::TcpStream::connect_timeout(
                                    &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:40001".parse().unwrap()),
                                    std::time::Duration::from_millis(800),
                                ) {
                                    Ok(_) => app.tci_test_result = Some(format!("TCI {}:{} OK", host, port)),
                                    Err(e) => app.tci_test_result = Some(e.to_string()),
                                }
                            }
                        });
                        if let Some(ref res) = app.tci_test_result {
                            ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
                        }
                    } else if app.cat_conn_type == "tcp" {
                        ui.horizontal(|ui| {
                            ui.label("Host TCP:");
                            ui.add(egui::TextEdit::singleline(&mut app.cat_host).desired_width(120.0));
                            ui.add_space(10.0);
                            ui.label("Port TCP:");
                            let mut port_str = app.cat_port.to_string();
                            if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0)).changed() {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    app.cat_port = p;
                                }
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Port COM / TTY:");
                            ui.add(egui::TextEdit::singleline(&mut app.cat_serial_port).desired_width(110.0).hint_text("COM3, /dev/ttyUSB0"));
                            ui.add_space(10.0);
                            ui.label("Baud rate:");
                            egui::ComboBox::from_id_salt("cat_baud_combo")
                                .selected_text(format!("{} bps", app.cat_baud_rate))
                                .show_ui(ui, |ui| {
                                    for &b in &[4800, 9600, 19200, 38400, 57600, 115200] {
                                        ui.selectable_value(&mut app.cat_baud_rate, b, format!("{} bps", b));
                                    }
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut app.cat_auto_start_rigctld, "Auto-start rigctld").on_hover_text("Uruchamia proces rigctld w tle dla tego portu szeregowego");
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", tr("cat.poll_rate", lang)));
                        ui.add(egui::DragValue::new(&mut app.cat_poll_rate_ms).range(50..=2000).suffix(" ms"));

                        if app.cat_conn_type != "tci" {
                            if ui.button("⚡ Test TCP").clicked() {
                                let host = app.cat_host.clone();
                                let port = app.cat_port;
                                match std::net::TcpStream::connect_timeout(
                                    &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:4532".parse().unwrap()),
                                    std::time::Duration::from_millis(800),
                                ) {
                                    Ok(_) => app.cat_test_result = Some(format!("TCP {}:{} OK", host, port)),
                                    Err(e) => app.cat_test_result = Some(e.to_string()),
                                }
                            }
                        }
                    });

                    if let Some(ref res) = app.cat_test_result {
                        ui.add_space(2.0);
                        let color = if app.cat_connected {
                            egui::Color32::from_rgb(34, 197, 94)
                        } else {
                            egui::Color32::from_rgb(251, 146, 60)
                        };
                        ui.colored_label(color, res);
                    }
                });

                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr("cat_settings.sharing_title", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut app.cat_sharing_enabled, tr("cat_settings.sharing_enable", lang)).changed() {
                            app.toggle_cat_proxy_server();
                            app.save_station_config();
                        }
                        ui.add_space(10.0);
                        ui.label(tr("cat_settings.server_port", lang));
                        ui.add(egui::DragValue::new(&mut app.cat_sharing_port).range(1024..=65535));
                    });

                    if app.cat_sharing_active {
                        ui.label(egui::RichText::new(format!("● 127.0.0.1:{}", app.cat_sharing_port)).color(egui::Color32::from_rgb(34, 197, 94)).size(11.0));
                    }
                });

                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr("cat_settings.rotor_section", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.horizontal(|ui| {
                        ui.label("Host:");
                        ui.add(egui::TextEdit::singleline(&mut app.rotor_host).desired_width(110.0));
                        ui.add_space(10.0);
                        ui.label("Port TCP:");
                        let mut port_str = app.rotor_port.to_string();
                        if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0)).changed() {
                            if let Ok(p) = port_str.trim().parse::<u16>() {
                                app.rotor_port = p;
                            }
                        }
                        ui.add_space(10.0);
                        if ui.button("⚡ Test Rotora").clicked() {
                            let host = app.rotor_host.clone();
                            let port = app.rotor_port;
                            match std::net::TcpStream::connect_timeout(
                                &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:4533".parse().unwrap()),
                                std::time::Duration::from_millis(800),
                            ) {
                                Ok(_) => app.rotor_test_result = Some(format!("rotctld {}:{} OK", host, port)),
                                Err(e) => app.rotor_test_result = Some(e.to_string()),
                            }
                        }
                    });

                    if let Some(ref res) = app.rotor_test_result {
                        ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
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
    app.show_cat_settings_window = is_open;
}
