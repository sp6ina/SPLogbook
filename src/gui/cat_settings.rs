// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::rig_models::search_rig_models_by_mfg;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Testuje połączenie TCP z podanym hostem/portem, wspierając zarówno adresy IP
/// jak i nazwy hosta (DNS/`localhost`). W przypadku niepoprawnego adresu zwraca
/// jawny błąd zamiast po cichu łączyć się z zupełnie innym, domyślnym adresem.
fn test_tcp_connection(host: &str, port: u16, timeout_ms: u64) -> Result<(), String> {
    use std::net::ToSocketAddrs;
    let host = host.trim();
    if host.is_empty() {
        return Err("Host nie może być pusty".to_string());
    }
    let mut addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("Nieprawidłowy adres {host}:{port} — {e}"))?;
    let addr = addrs
        .next()
        .ok_or_else(|| format!("Nie udało się rozwiązać adresu {host}:{port}"))?;
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(timeout_ms))
        .map(|_| ())
        .map_err(|e| format!("Błąd połączenia z {host}:{port} — {e}"))
}

pub fn render_cat_settings_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_cat_settings_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_cat_settings_window;
    let mut close_req = false;

    egui::Window::new(
        egui::RichText::new(format!("📻 {}", tr("cat.settings_title", lang)))
            .size(14.0)
            .strong(),
    )
    .open(&mut is_open)
    .default_size([650.0, 560.0])
    .min_size([480.0, 420.0])
    .resizable(true)
    .show(ctx, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if app.cat_connected {
                    ui.colored_label(
                        egui::Color32::from_rgb(34, 197, 94),
                        tr("cat.connected", lang),
                    );
                    if ui.button(tr("cat.disconnect_radio", lang)).clicked() {
                        app.stop_cat_service();
                    }
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(239, 68, 68),
                        tr("cat.disconnected", lang),
                    );
                    if ui.button(tr("cat.connect_radio", lang)).clicked() {
                        app.start_cat_service();
                    }
                }
            });

            ui.separator();

            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(tr("cat_settings.step1_title", lang))
                        .strong()
                        .color(egui::Color32::from_rgb(250, 204, 21)),
                );
                ui.add_space(2.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(tr("cat_settings.filter_mfg", lang))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );
                    let top_mfgs = [
                        "Wszystkie",
                        "Icom",
                        "Yaesu",
                        "Kenwood",
                        "Elecraft",
                        "Xiegu",
                        "FlexRadio",
                        "SunSDR",
                        "Hamlib",
                    ];
                    for m in top_mfgs {
                        let is_sel = if m == "SunSDR" {
                            app.cat_backend == "tci"
                        } else {
                            app.cat_mfg_selected == m && app.cat_backend != "tci"
                        };

                        let display_mfg = if m == "Wszystkie" {
                            tr("cat_settings.all_mfgs", lang)
                        } else {
                            m
                        };
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
                        ui.add(
                            egui::TextEdit::singleline(&mut app.cat_model_search)
                                .hint_text("7300, FT-891, TS-590, G90, K4"),
                        );
                    });

                    let mfg_query = if app.cat_mfg_selected == "Wszystkie" {
                        ""
                    } else {
                        &app.cat_mfg_selected
                    };
                    let filtered_rigs = search_rig_models_by_mfg(mfg_query, &app.cat_model_search);

                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            for rig in filtered_rigs {
                                let is_selected = app.cat_rig_id == rig.rig_id;
                                let label_text = format!(
                                    "[ID {:4}] {:15} - {:20} ({})",
                                    rig.rig_id, rig.mfg, rig.model, rig.status
                                );
                                if ui.selectable_label(is_selected, label_text).clicked() {
                                    app.cat_rig_id = rig.rig_id;
                                    app.cat_rig_model = format!("{} {}", rig.mfg, rig.model);
                                    app.cat_baud_rate = rig.default_baud;
                                    app.cat_test_result = Some(format!(
                                        "{} ({} bps)",
                                        app.cat_rig_model, rig.default_baud
                                    ));
                                }
                            }
                        });
                }

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}:", tr("cat.rig_model", lang))).strong(),
                    );
                    ui.colored_label(
                        egui::Color32::from_rgb(56, 189, 248),
                        format!("{} (ID: {})", app.cat_rig_model, app.cat_rig_id),
                    );
                });
            });

            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(tr("cat_settings.step2_title", lang))
                        .strong()
                        .color(egui::Color32::from_rgb(250, 204, 21)),
                );
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(tr("cat_settings.conn_type", lang));
                    if ui
                        .selectable_value(
                            &mut app.cat_conn_type,
                            "serial".to_string(),
                            tr("cat_settings.conn_serial", lang),
                        )
                        .clicked()
                    {
                        app.cat_backend = "hamlib".to_string();
                        app.cat_auto_start_rigctld = true;
                    }
                    if ui
                        .selectable_value(
                            &mut app.cat_conn_type,
                            "tcp".to_string(),
                            tr("cat_settings.conn_tcp", lang),
                        )
                        .clicked()
                    {
                        if app.cat_backend != "flrig" {
                            app.cat_backend = "hamlib".to_string();
                        }
                    }
                    if ui
                        .selectable_value(
                            &mut app.cat_conn_type,
                            "tci".to_string(),
                            tr("cat_settings.conn_tci", lang),
                        )
                        .clicked()
                    {
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
                        if ui
                            .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                            .changed()
                        {
                            if let Ok(p) = port_str.trim().parse::<u16>() {
                                app.tci_port = p;
                            }
                        }
                        if ui.button(tr("cat_settings.test_tci", lang)).clicked() {
                            let host = app.tci_host.clone();
                            let port = app.tci_port;
                            match test_tcp_connection(&host, port, 800) {
                                Ok(()) => {
                                    app.tci_test_result = Some(format!("TCI {host}:{port} OK"));
                                }
                                Err(e) => app.tci_test_result = Some(e),
                            }
                        }
                    });
                    if let Some(ref res) = app.tci_test_result {
                        ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
                    }
                } else {
                    if app.cat_conn_type == "tcp" {
                        ui.horizontal(|ui| {
                            ui.label("Backend TCP:");
                            if ui
                                .selectable_label(
                                    app.cat_backend != "flrig",
                                    "Hamlib (rigctld :4532)",
                                )
                                .clicked()
                            {
                                app.cat_backend = "hamlib".to_string();
                                if app.cat_port == 12345 {
                                    app.cat_port = 4532;
                                }
                            }
                            if ui
                                .selectable_label(
                                    app.cat_backend == "flrig",
                                    "FLRig (XML-RPC :12345)",
                                )
                                .clicked()
                            {
                                app.cat_backend = "flrig".to_string();
                                app.cat_auto_start_rigctld = false;
                                if app.cat_port == 4532 {
                                    app.cat_port = 12345;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Host TCP:");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.cat_host).desired_width(120.0),
                            );
                            ui.add_space(10.0);
                            ui.label("Port TCP:");
                            let mut port_str = app.cat_port.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                                .changed()
                            {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    app.cat_port = p;
                                }
                            }
                        });
                    } else {
                        // Tryb Serial (RS-232 / USB) — automatycznie korzysta z lokalnego mostka rigctld
                        app.cat_auto_start_rigctld = true;
                        if app.cat_host.trim().is_empty() {
                            app.cat_host = "127.0.0.1".to_string();
                        }
                        if app.cat_port == 0 {
                            app.cat_port = 4532;
                        }
                    }

                    if app.cat_conn_type == "serial" || app.cat_auto_start_rigctld {
                        ui.horizontal(|ui| {
                            ui.label(tr("cat_settings.serial_port_label", lang));
                            ui.add(
                                egui::TextEdit::singleline(&mut app.cat_serial_port)
                                    .desired_width(95.0)
                                    .hint_text("COM3, /dev/ttyUSB0"),
                            );
                            let port_presets: &[&str] = if cfg!(target_os = "windows") {
                                &[
                                    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
                                    "COM9", "COM10", "COM11", "COM12",
                                ]
                            } else {
                                &[
                                    "/dev/ttyUSB0",
                                    "/dev/ttyUSB1",
                                    "/dev/ttyACM0",
                                    "/dev/ttyACM1",
                                    "/dev/ttyS0",
                                ]
                            };
                            egui::ComboBox::from_id_salt("cat_serial_port_preset_combo")
                                .width(85.0)
                                .selected_text(if app.cat_serial_port.is_empty() {
                                    crate::core::i18n::tr_or(lang, "Wybierz...", "Select...")
                                } else {
                                    app.cat_serial_port.as_str()
                                })
                                .show_ui(ui, |ui| {
                                    for &p in port_presets {
                                        ui.selectable_value(
                                            &mut app.cat_serial_port,
                                            p.to_string(),
                                            p,
                                        );
                                    }
                                });

                            ui.add_space(6.0);
                            ui.label(tr("cat_settings.baud_rate_label", lang));
                            egui::ComboBox::from_id_salt("cat_baud_combo")
                                .selected_text(format!("{} bps", app.cat_baud_rate))
                                .show_ui(ui, |ui| {
                                    for &b in &[4800, 9600, 19200, 38400, 57600, 115_200] {
                                        ui.selectable_value(
                                            &mut app.cat_baud_rate,
                                            b,
                                            format!("{b} bps"),
                                        );
                                    }
                                });
                        });
                    }

                    if app.cat_conn_type == "tcp" && app.cat_backend != "flrig" {
                        ui.horizontal(|ui| {
                            ui.checkbox(
                                &mut app.cat_auto_start_rigctld,
                                tr("cat_settings.auto_start_rigctld", lang),
                            )
                            .on_hover_text(tr("cat_settings.auto_start_rigctld_tooltip", lang));
                        });
                    }

                    if app.cat_auto_start_rigctld {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}:",
                                    tr("cat_settings.hamlib_source", lang)
                                ))
                                .strong(),
                            );
                            ui.selectable_value(
                                &mut app.cat_hamlib_source,
                                "bundled".to_string(),
                                tr("cat_settings.hamlib_source_bundled", lang),
                            );
                            ui.selectable_value(
                                &mut app.cat_hamlib_source,
                                "system".to_string(),
                                tr("cat_settings.hamlib_source_system", lang),
                            );
                        });

                        let detected =
                            crate::cat::supervisor::RigctldSupervisor::find_rigctld_binary(
                                &app.cat_hamlib_source,
                                if app.cat_custom_rigctld_path.is_empty() {
                                    None
                                } else {
                                    Some(&app.cat_custom_rigctld_path)
                                },
                            );

                        ui.horizontal(|ui| {
                            if let Some(ref p) = detected {
                                ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "✔");
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}: {}",
                                        tr("cat_settings.detected_binary", lang),
                                        p.display()
                                    ))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                                );
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "⚠");
                                ui.label(
                                    egui::RichText::new(tr("cat_settings.binary_not_found", lang))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(239, 68, 68)),
                                );
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(tr("cat_settings.custom_path_label", lang))
                                    .size(11.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.cat_custom_rigctld_path)
                                    .desired_width(220.0)
                                    .hint_text(tr("cat_settings.custom_path_hint", lang)),
                            );
                        });

                        if let Some(ref mut sup) = app.rigctld_supervisor {
                            if sup.is_running() {
                                if let Some(pid) = sup.pid() {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(34, 197, 94),
                                            "●",
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} (PID: {})",
                                                tr("cat_settings.running_pid", lang),
                                                pid
                                            ))
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(34, 197, 94)),
                                        );
                                    });
                                }
                            }
                        }
                    }
                }

                ui.horizontal(|ui| {
                    ui.label(format!("{}:", tr("cat.poll_rate", lang)));
                    ui.add(
                        egui::DragValue::new(&mut app.cat_poll_rate_ms)
                            .range(50..=2000)
                            .suffix(" ms"),
                    );

                    if app.cat_conn_type != "tci"
                        && ui.button(tr("cat_settings.test_tcp", lang)).clicked()
                    {
                        let host = app.cat_host.clone();
                        let port = app.cat_port;
                        match test_tcp_connection(&host, port, 800) {
                            Ok(()) => app.cat_test_result = Some(format!("TCP {host}:{port} OK")),
                            Err(e) => app.cat_test_result = Some(e),
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
                ui.label(
                    egui::RichText::new(tr("cat_settings.sharing_title", lang))
                        .strong()
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut app.cat_sharing_enabled,
                            tr("cat_settings.sharing_enable", lang),
                        )
                        .changed()
                    {
                        app.toggle_cat_proxy_server();
                        app.save_station_config();
                    }
                    ui.add_space(10.0);
                    ui.label(tr("cat_settings.server_port", lang));
                    ui.add(egui::DragValue::new(&mut app.cat_sharing_port).range(1024..=65535));
                });

                if app.cat_sharing_active {
                    ui.label(
                        egui::RichText::new(format!("● 127.0.0.1:{}", app.cat_sharing_port))
                            .color(egui::Color32::from_rgb(34, 197, 94))
                            .size(11.0),
                    );
                }
            });

            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(tr("cat_settings.rotor_section", lang))
                        .strong()
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.add(egui::TextEdit::singleline(&mut app.rotor_host).desired_width(110.0));
                    ui.add_space(10.0);
                    ui.label("Port TCP:");
                    let mut port_str = app.rotor_port.to_string();
                    if ui
                        .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                        .changed()
                    {
                        if let Ok(p) = port_str.trim().parse::<u16>() {
                            app.rotor_port = p;
                        }
                    }
                    ui.add_space(10.0);
                    if ui.button(tr("cat_settings.test_rotor", lang)).clicked() {
                        let host = app.rotor_host.clone();
                        let port = app.rotor_port;
                        match test_tcp_connection(&host, port, 800) {
                            Ok(()) => {
                                app.rotor_test_result = Some(format!("rotctld {host}:{port} OK"));
                            }
                            Err(e) => app.rotor_test_result = Some(e.clone()),
                        }
                    }
                });

                if let Some(ref res) = app.rotor_test_result {
                    ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new(tr("btn.save", lang)).strong())
                    .clicked()
                {
                    if app.cat_connected || app.cat_auto_start_rigctld {
                        app.start_cat_service();
                    }
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
