// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::rig_models::search_rig_models;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Okno dialogowe konfiguracji połączenia CAT / Hamlib
pub fn render_cat_settings_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_cat_settings_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_cat_settings_window;
    let mut close_req = false;

    egui::Window::new(format!("📻 {}", tr("cat.settings_title", lang)))
        .open(&mut is_open)
        .default_size([540.0, 520.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Konfiguracja interfejsu CAT (Hamlib / TCI)").size(15.0).color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if app.cat_connected {
                            ui.label(egui::RichText::new("● POŁĄCZONO").color(egui::Color32::from_rgb(34, 197, 94)).strong());
                        } else {
                            ui.label(egui::RichText::new("○ ROZŁĄCZONO").color(egui::Color32::from_rgb(239, 68, 68)).strong());
                        }
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Protokół sterowania radiem:").strong());
                    ui.selectable_value(&mut app.cat_backend, "hamlib".to_string(), "📻 Hamlib rigctld");
                    ui.selectable_value(&mut app.cat_backend, "tci".to_string(), "⚡ TCI (ExpertSDR / Thetis)");
                });
                ui.separator();

                if app.cat_backend == "tci" {
                    // Konfiguracja TCI dla radii SDR
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Konfiguracja protokołu TCI (ExpertSDR / SunSDR / Thetis):").strong());
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
                        });

                        if ui.button("⚡ Test Połączenia TCI").clicked() {
                            let host = app.tci_host.clone();
                            let port = app.tci_port;
                            match std::net::TcpStream::connect_timeout(
                                &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:40001".parse().unwrap()),
                                std::time::Duration::from_millis(800),
                            ) {
                                Ok(_) => {
                                    app.tci_test_result = Some(format!("Połączenie TCI z {}:{} pomyślne!", host, port));
                                }
                                Err(e) => {
                                    app.tci_test_result = Some(format!("Błąd TCI z {}:{}: {}", host, port, e));
                                }
                            }
                        }

                        if let Some(ref res) = app.tci_test_result {
                            ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
                        }
                    });
                } else {
                    // 1. Wyszukiwarka i wybór modelu z pełnej bazy 313 modeli Hamlib
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Wybór radiostacji (313 modeli Hamlib):").strong());
                        ui.horizontal(|ui| {
                            ui.label("🔍 Szukaj modelu / producenta:");
                            ui.add(egui::TextEdit::singleline(&mut app.cat_model_search).hint_text("np. 7300, FT-891, TS-590, G90, KX3, FTDX"));
                        });

                        let filtered_rigs = search_rig_models(&app.cat_model_search);
                        ui.label(egui::RichText::new(format!("Pasujące modele: {} pozycji", filtered_rigs.len())).small().color(egui::Color32::GRAY));

                        egui::ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                            for rig in filtered_rigs {
                                let is_selected = app.cat_rig_id == rig.rig_id;
                                let label_text = format!("[ID {:4}] {:15} - {:20} ({})", rig.rig_id, rig.mfg, rig.model, rig.status);
                                if ui.selectable_label(is_selected, label_text).clicked() {
                                    app.cat_rig_id = rig.rig_id;
                                    app.cat_rig_model = format!("{} {}", rig.mfg, rig.model);
                                    app.cat_baud_rate = rig.default_baud;
                                    app.cat_test_result = Some(format!("Wybrano: {} (Domyślny baud: {} bps)", app.cat_rig_model, rig.default_baud));
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Wybrany transceiver:").strong());
                            ui.colored_label(egui::Color32::from_rgb(56, 189, 248), format!("{} (ID: {})", app.cat_rig_model, app.cat_rig_id));
                        });
                    });

                    ui.add_space(4.0);

                    // 2. Opcja natywnego uruchamiania rigctld w tle (zero konsoli)
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Tryb uruchomienia Hamlib:").strong());
                        ui.checkbox(&mut app.cat_auto_start_rigctld, "⚡ Uruchamiaj rigctld natywnie w tle (zalecane - bez otwierania terminala)");
                        if app.cat_auto_start_rigctld {
                            ui.label(egui::RichText::new("Program SPLogbook automatycznie uruchomi i zamknie oficjalnego demona rigctld.exe.").small().italics());
                        }

                        ui.horizontal(|ui| {
                            ui.label("Port COM radia:");
                            ui.add(egui::TextEdit::singleline(&mut app.cat_serial_port).desired_width(70.0));
                            ui.add_space(10.0);
                            ui.label("Prędkość (Baud):");
                            egui::ComboBox::from_id_salt("cat_baud_combo")
                                .selected_text(format!("{} bps", app.cat_baud_rate))
                                .show_ui(ui, |ui| {
                                    for b in &[1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200] {
                                        ui.selectable_value(&mut app.cat_baud_rate, *b, format!("{} bps", b));
                                    }
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Host TCP:");
                            ui.add(egui::TextEdit::singleline(&mut app.cat_host).desired_width(100.0));
                            ui.add_space(10.0);
                            ui.label("Port TCP:");
                            let mut port_str = app.cat_port.to_string();
                            if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(55.0)).changed() {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    app.cat_port = p;
                                }
                            }
                            ui.add_space(10.0);
                            ui.label("Odpytywanie:");
                            ui.add(egui::Slider::new(&mut app.cat_poll_rate_ms, 50..=2000).suffix(" ms"));
                        });
                    });

                    ui.add_space(4.0);

                    // 3. Połącz / Odłącz i Zarządca procesu
                    ui.horizontal(|ui| {
                        if app.cat_connected {
                            if ui.button(egui::RichText::new("❌ ODŁĄCZ RADIO").color(egui::Color32::from_rgb(239, 68, 68)).strong()).clicked() {
                                app.stop_cat_service();
                            }
                        } else {
                            if ui.button(egui::RichText::new("🔌 POŁĄCZ Z RADIEM").color(egui::Color32::from_rgb(34, 197, 94)).strong()).clicked() {
                                app.start_cat_service();
                            }
                        }

                        if ui.button("⚡ Test Połączenia TCP").clicked() {
                            let host = app.cat_host.clone();
                            let port = app.cat_port;
                            match std::net::TcpStream::connect_timeout(
                                &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:4532".parse().unwrap()),
                                std::time::Duration::from_millis(800),
                            ) {
                                Ok(_) => {
                                    app.cat_test_result = Some(format!("Połączenie TCP z {}:{} pomyślne!", host, port));
                                }
                                Err(e) => {
                                    app.cat_test_result = Some(format!("Błąd TCP z {}:{}: {}", host, port, e));
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
                }

                ui.add_space(6.0);
                ui.separator();

                // Konfiguracja rotora antenowego (Hamlib rotctld)
                ui.group(|ui| {
                    ui.label(egui::RichText::new("🧭 Sterowanie Rotorem Antenowym (Hamlib rotctld):").strong().color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.horizontal(|ui| {
                        ui.label("Host Rotora:");
                        ui.add(egui::TextEdit::singleline(&mut app.rotor_host).desired_width(110.0));
                        ui.add_space(10.0);
                        ui.label("Port TCP Rotora:");
                        let mut port_str = app.rotor_port.to_string();
                        if ui.add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0)).changed() {
                            if let Ok(p) = port_str.trim().parse::<u16>() {
                                app.rotor_port = p;
                            }
                        }
                        ui.add_space(10.0);
                        if ui.button("⚡ Test Połączenia z Rotorem").clicked() {
                            let host = app.rotor_host.clone();
                            let port = app.rotor_port;
                            match std::net::TcpStream::connect_timeout(
                                &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:4533".parse().unwrap()),
                                std::time::Duration::from_millis(800),
                            ) {
                                Ok(_) => {
                                    app.rotor_test_result = Some(format!("Połączenie z rotctld na {}:{} pomyślne!", host, port));
                                }
                                Err(e) => {
                                    app.rotor_test_result = Some(format!("Błąd połączenia z rotctld {}:{}: {}", host, port, e));
                                }
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
