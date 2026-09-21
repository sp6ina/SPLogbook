// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::CLUSTER_PRESETS;
use crate::core::i18n::{tr, Language};
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Główny pasek menu (Menu Bar) z rozwijanymi kategoriami
pub fn render_menu_bar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    egui::menu::bar(ui, |ui| {
        // Menu: Plik
        ui.menu_button(tr("menu.file", lang), |ui| {
            if ui.button(format!("📁 {}", tr("menu.journal_mgmt", lang))).clicked() {
                if let Ok(db) = app.log_db.lock() {
                    app.journal_dialog.reload(&db);
                }
                app.journal_dialog.is_open = true;
                ui.close_menu();
            }
            if ui.button(format!("🔍 {}", tr("menu.advanced_filter", lang))).clicked() {
                app.advanced_filter_dialog.is_open = true;
                ui.close_menu();
            }
            if ui.button(format!("🏷 {}", tr("menu.qsl_print", lang))).clicked() {
                app.qsl_designer_dialog.is_open = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button(format!("⭐ {}", tr("wizard.setup_station", lang))).clicked() {
                app.show_welcome_wizard = true;
                ui.close_menu();
            }
            if ui.button(format!("🌐 {}", tr("menu.online_sync", lang))).clicked() {
                app.show_online_sync_window = true;
                ui.close_menu();
            }
            if ui.button(tr("menu.import_adif", lang)).clicked() {
                app.trigger_import_adif();
                ui.close_menu();
            }
            if ui.button(tr("menu.export_adif", lang)).clicked() {
                app.trigger_export_adif();
                ui.close_menu();
            }
            if ui.button(format!("🏔 {}", tr("menu.export_sota", lang))).clicked() {
                let call = app.my_station.callsign.clone();
                app.sota_dialog.open(&call);
                ui.close_menu();
            }
            if ui.button(format!("📄 {}", tr("menu.export_pdf", lang))).clicked() {
                app.export_pdf_log();
                ui.close_menu();
            }
            if ui.button(format!("🗺 {}", tr("menu.export_gpx", lang))).clicked() {
                app.export_gpx_log();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(tr("menu.backup", lang)).clicked() {
                app.run_manual_backup();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(tr("menu.exit", lang)).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        // Menu: Edycja
        ui.menu_button(tr("menu.edit", lang), |ui| {
            if ui.button(tr("qso.clear", lang)).clicked() {
                app.clear_qso_form();
                ui.close_menu();
            }
            if ui.button(tr("qso.delete", lang)).clicked() {
                app.delete_selected_qso();
                ui.close_menu();
            }
        });

        // Menu: Widok (Zarządzanie kafelkami modułów i układem pulpitu)
        ui.menu_button(tr("menu.view", lang), |ui| {
            ui.label(egui::RichText::new(tr("view.panels_title", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
            ui.separator();

            let mut changed = false;

            if ui.checkbox(&mut app.panel_vfo.visible, format!("📻 {}", tr("view.panel_vfo", lang))).changed() { changed = true; }
            if ui.checkbox(&mut app.panel_qso.visible, format!("📝 {}", tr("view.panel_qso", lang))).changed() { changed = true; }
            if ui.checkbox(&mut app.panel_log.visible, format!("📖 {}", tr("view.panel_log", lang))).changed() { changed = true; }
            if ui.checkbox(&mut app.panel_cluster.visible, format!("📡 {}", tr("view.panel_cluster", lang))).changed() { changed = true; }
            if ui.checkbox(&mut app.panel_bandmap.visible, format!("📶 {}", tr("view.panel_bandmap", lang))).changed() {
                app.show_bandmap_window = app.panel_bandmap.visible;
                changed = true;
            }
            if ui.checkbox(&mut app.panel_solar.visible, format!("☀ {}", tr("view.panel_solar", lang))).changed() { changed = true; }
            if ui.checkbox(&mut app.panel_satellites.visible, format!("🛰 {}", tr("view.panel_satellites", lang))).changed() {
                app.show_satellites_window = app.panel_satellites.visible;
                changed = true;
            }
            if ui.checkbox(&mut app.panel_world_map.visible, format!("🗺 {}", tr("view.panel_world_map", lang))).changed() {
                app.show_world_map_window = app.panel_world_map.visible;
                changed = true;
            }

            if changed {
                app.save_station_config();
            }

            ui.separator();
            if ui.button(egui::RichText::new(format!("🔄 {}", tr("view.reset_layout", lang))).strong()).clicked() {
                app.reset_panel_layout();
                app.save_station_config();
                ui.close_menu();
            }
            ui.separator();
            ui.checkbox(&mut app.show_awards_matrix_window, format!("🏆 {}", tr("view.awards_matrix", lang)));
            if ui.checkbox(&mut app.compact_hud_mode, format!("🗗 {}", tr("view.compact_hud", lang))).changed() {
                app.save_station_config();
            }
            let theme_label = if app.dark_theme { tr("theme.dark", lang) } else { tr("theme.light", lang) };
            if ui.checkbox(&mut app.dark_theme, theme_label).changed() {
                app.save_station_config();
            }
        });

        // Menu: DX Cluster (Zarządzanie połączeniem, serwerami i filtrami)
        ui.menu_button(format!("📡 {}", tr("menu.dx_cluster", lang)), |ui| {
            if app.cluster_connected {
                if ui.button(format!("🔴 {}", tr("cluster.disconnect_btn", lang))).clicked() {
                    app.disconnect_dx_cluster();
                    ui.close_menu();
                }
            } else {
                let txt = if app.cluster_connecting { format!("● {}", tr("cluster.connecting_btn", lang)) } else { format!("● {}", tr("cluster.connect_btn", lang)) };
                if ui.button(txt).clicked() {
                    app.connect_dx_cluster();
                    ui.close_menu();
                }
            }

            if ui.button(format!("📢 {}", tr("cluster.announce_spot", lang))).clicked() {
                let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
                app.send_spot_dialog.open_with(&app.entry_callsign, freq_khz);
                ui.close_menu();
            }

            ui.separator();
            ui.label(egui::RichText::new(tr("cluster.quick_server", lang)).strong());
            for (name, host, port) in CLUSTER_PRESETS {
                let is_current = app.cluster_host == *host && app.cluster_port == *port;
                if ui.selectable_label(is_current, *name).clicked() {
                    app.cluster_host = host.to_string();
                    app.cluster_port = *port;
                    app.save_station_config();
                    if app.cluster_connected {
                        app.connect_dx_cluster();
                    }
                    ui.close_menu();
                }
            }

            ui.separator();
            ui.label(egui::RichText::new(tr("cluster.filters_options", lang)).strong());
            if ui.checkbox(&mut app.cluster_auto_connect, tr("cluster.auto_connect_startup", lang)).changed() {
                app.save_station_config();
            }
            if ui.checkbox(&mut app.cluster_filter_current_band, tr("cluster.filter_vfo_band", lang)).changed() {
                app.save_station_config();
            }
            if ui.checkbox(&mut app.cluster_hide_ft8, tr("cluster.hide_ft8", lang)).changed() {
                app.save_station_config();
            }
            if ui.checkbox(&mut app.cluster_hide_skimmers, tr("cluster.hide_skimmers", lang)).changed() {
                app.save_station_config();
            }

            ui.separator();
            if ui.button(format!("🗑 {}", tr("cluster.clear_spots", lang))).clicked() {
                app.cluster_spots.clear();
                ui.close_menu();
            }
        });

        // Menu: Bazy Referencyjne
        ui.menu_button(format!("📚 {}", tr("menu.ref_databases", lang)), |ui| {
            if ui.button(format!("🏝 {}", tr("ref.iota_db", lang))).clicked() {
                app.iota_dialog.open();
                ui.close_menu();
            }
            if ui.button(format!("🗺 {}", tr("ref.states_db", lang))).clicked() {
                app.states_dialog.open();
                ui.close_menu();
            }
            if ui.button(format!("📋 {}", tr("ref.qsl_managers", lang))).clicked() {
                app.qsl_manager_dialog.open();
                ui.close_menu();
            }
            if ui.button(format!("🌐 {}", tr("ref.prefix_manager", lang))).clicked() {
                app.prefix_manager_dialog.open();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(format!("🌙 {}", tr("ref.moon_sun", lang))).clicked() {
                app.astronomy_dialog.open();
                ui.close_menu();
            }
            if ui.button(format!("⚡ {}", tr("ref.wol", lang))).clicked() {
                app.wol_dialog.open();
                ui.close_menu();
            }
            if ui.button(format!("🔄 {}", tr("ref.update_online_dbs", lang))).clicked() {
                app.trigger_database_update();
                ui.close_menu();
            }
        });

        // Menu: Moduły i Narzędzia
        ui.menu_button(tr("menu.tools", lang), |ui| {
            if ui.button(format!("📻 {}", tr("tools.cat_connection", lang))).clicked() {
                app.show_cat_settings_window = true;
                ui.close_menu();
            }
            if ui.button(format!("📟 {}", tr("tools.cw_terminal", lang))).clicked() {
                app.cw_terminal_dialog.is_open = true;
                ui.close_menu();
            }
            if ui.button(format!("🌐 {}", tr("tools.online_sync", lang))).clicked() {
                app.show_online_sync_window = true;
                ui.close_menu();
            }
            if ui.button(format!("🗺 {}", tr("tools.world_map", lang))).clicked() {
                app.panel_world_map.visible = true;
                app.panel_world_map.floating = true;
                app.show_world_map_window = true;
                ui.close_menu();
            }
            if ui.button(format!("🏆 {}", tr("tools.awards_matrix", lang))).clicked() {
                app.show_awards_matrix_window = true;
                ui.close_menu();
            }
            if ui.button(format!("📊 {}", tr("tools.statistics", lang))).clicked() {
                app.show_statistics_window = true;
                ui.close_menu();
            }
            if ui.button(format!("📡 {}", tr("tools.wspr_monitor", lang))).clicked() {
                app.show_wspr_window = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button(format!("🛰 {}", tr("tools.satellites", lang))).clicked() {
                app.panel_satellites.visible = true;
                app.panel_satellites.floating = true;
                app.show_satellites_window = true;
                ui.close_menu();
            }
            if ui.button(format!("🏆 {}", tr("tools.contest_module", lang))).clicked() {
                app.show_contest_window = true;
                ui.close_menu();
            }
            if ui.button("🌐 Praca Zespołowa Multi-Op (LAN)").clicked() {
                app.show_multi_op_window = true;
                ui.close_menu();
            }
            if ui.button(format!("⚡ {}", tr("tools.cw_macros", lang))).clicked() {
                app.show_cw_window = true;
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button(format!("🌐 {}", tr("tools.rest_api_server", lang)), |ui| {
                ui.label("Włącz serwer na porcie 8080 (wymaga restartu aplikacji dla zmiany portu)");
                if ui.checkbox(&mut app.rest_api_enabled, tr("tools.rest_api_enable", lang)).changed() {
                    app.save_station_config();
                    if app.rest_api_enabled {
                        let db = app.log_db.clone();
                        let cs = app.my_station.callsign.clone();
                        let port = app.rest_api_port;
                        let spots = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                        app.cluster_spots_api = Some(spots.clone());
                        tokio::spawn(async move {
                            crate::api::server::start_api_server(db, cs, port, spots).await;
                        });
                    }
                }
                ui.label(format!("API dostępne pod: http://127.0.0.1:{}/api/v1/", app.rest_api_port));
                if ui.button(format!("🔗 {}", tr("tools.rest_api_open", lang))).clicked() {
                    let _ = open::that(format!("http://127.0.0.1:{}/api/v1/status", app.rest_api_port));
                    ui.close_menu();
                }
            });
            ui.separator();
            if ui.button(tr("station.equipment", lang)).clicked() {
                app.show_ledger_window = true;
                ui.close_menu();
            }
            if ui.button(format!("🏷 {}", tr("tools.qsl_designer", lang))).clicked() {
                app.qsl_designer_dialog.is_open = true;
                ui.close_menu();
            }
        });

        // Menu: Język / Language (wybór języka interfejsu)
        ui.menu_button(format!("🌐 {}", lang.display_name()), |ui| {
            if ui.selectable_label(lang == Language::Pl, "🇵🇱 Polski").clicked() {
                app.current_language = Language::Pl;
                app.save_station_config();
                ui.close_menu();
            }
            if ui.selectable_label(lang == Language::En, "🇬🇧 English").clicked() {
                app.current_language = Language::En;
                app.save_station_config();
                ui.close_menu();
            }
            if ui.selectable_label(lang == Language::De, "🇩🇪 Deutsch").clicked() {
                app.current_language = Language::De;
                app.save_station_config();
                ui.close_menu();
            }
            if ui.selectable_label(lang == Language::Fr, "🇫🇷 Français").clicked() {
                app.current_language = Language::Fr;
                app.save_station_config();
                ui.close_menu();
            }
            if ui.selectable_label(lang == Language::Es, "🇪🇸 Español").clicked() {
                app.current_language = Language::Es;
                app.save_station_config();
                ui.close_menu();
            }
            if ui.selectable_label(lang == Language::Ru, "🇷🇺 Русский").clicked() {
                app.current_language = Language::Ru;
                app.save_station_config();
                ui.close_menu();
            }
        });

        // Menu: Pomoc
        ui.menu_button(tr("menu.help", lang), |ui| {
            if ui.button(tr("tab.about", lang)).clicked() {
                app.show_about_window = true;
                ui.close_menu();
            }
        });

        // Prawa strona paska menu: wyłącznie znak OP i zegar UTC (nie koliduje z lewym menu!)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let utc_now = chrono::Utc::now().format("%H:%M:%S UTC").to_string();
            ui.label(egui::RichText::new(format!("⏱ {}", utc_now)).color(egui::Color32::from_rgb(148, 163, 184)).monospace().strong());
            ui.separator();
            ui.label(egui::RichText::new(format!("OP: {} ({})", app.my_station.callsign, app.my_station.gridsquare)).color(egui::Color32::from_rgb(56, 189, 248)).strong());
        });
    });
}

/// Drugi rząd paska narzędziowego z automatycznym zawijaniem (horizontal_wrapped) i pełną personalizacją (PPM / ikona zębatki)
pub fn render_main_toolbar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    let mut show_customize_popup = false;

    let toolbar_resp = ui.horizontal_wrapped(|ui| {
        // 1. Aktywny dziennik
        if app.quick_access.show_journal
            && ui.button(egui::RichText::new(format!("📁 {}: {} ({})", tr("toolbar.journal", lang), app.active_journal.name, app.active_journal.station_callsign)).strong().color(egui::Color32::from_rgb(100, 220, 100))).on_hover_text("Kliknij aby zarządzać profilami dzienników (PPM: personalizacja paska)").clicked() {
                if let Ok(db) = app.log_db.lock() {
                    app.journal_dialog.reload(&db);
                }
                app.journal_dialog.is_open = true;
            }

        // 2. Sygnalizator WSJT-X
        if app.quick_access.show_wsjtx {
            ui.label(egui::RichText::new(format!("● WSJT-X ({})", app.wsjtx_packets_count)).small().color(egui::Color32::from_rgb(52, 211, 153)));
        }

        // 3. Sygnalizator aktywnego filtra
        if app.advanced_filter_dialog.is_filtered_active
            && ui.button(egui::RichText::new("🔍 FILTR").strong().color(egui::Color32::YELLOW)).clicked() {
                app.advanced_filter_dialog.is_open = true;
            }

        // 4. Status DX Cluster
        if app.quick_access.show_cluster_status {
            let (cl_color, cl_text) = if app.cluster_connected {
                (egui::Color32::from_rgb(34, 197, 94), format!("● Cluster ({})", app.cluster_host))
            } else if app.cluster_connecting {
                (egui::Color32::from_rgb(250, 204, 21), format!("● Cluster: {}", tr("toolbar.cluster_connecting", lang)))
            } else {
                (egui::Color32::from_rgb(148, 163, 184), format!("○ Cluster: {}", tr("toolbar.cluster_disconnected", lang)))
            };

            if ui.button(egui::RichText::new(cl_text).small().color(cl_color)).on_hover_text("Kliknij, aby połączyć / rozłączyć DX Cluster (PPM: personalizacja)").clicked() {
                if app.cluster_connected {
                    app.disconnect_dx_cluster();
                } else {
                    app.connect_dx_cluster();
                }
            }
        }

        ui.separator();

        // 5. Szybkie przyciski modułów
        if app.quick_access.show_vfo
            && ui.button(format!("📻 {}", tr("toolbar.vfo", lang))).on_hover_text("Włącz/wyłącz VFO Transceivera").clicked() {
                app.panel_vfo.visible = !app.panel_vfo.visible;
                app.save_station_config();
            }

        if app.quick_access.show_qso
            && ui.button(format!("📝 {}", tr("toolbar.qso", lang))).on_hover_text("Włącz/wyłącz formularz wprowadzania QSO").clicked() {
                app.panel_qso.visible = !app.panel_qso.visible;
                app.save_station_config();
            }

        if app.quick_access.show_log
            && ui.button(format!("📖 {}", tr("toolbar.log", lang))).on_hover_text("Włącz/wyłącz tabelę dziennika łączności").clicked() {
                app.panel_log.visible = !app.panel_log.visible;
                app.save_station_config();
            }

        if app.quick_access.show_cat
            && ui.button(format!("📻 {}", tr("toolbar.cat", lang))).on_hover_text("Ustawienia radia Hamlib CAT").clicked() {
                app.show_cat_settings_window = !app.show_cat_settings_window;
            }

        if app.quick_access.show_cluster
            && ui.button(format!("📡 {}", tr("toolbar.cluster", lang))).on_hover_text("Włącz/wyłącz panel DX Cluster").clicked() {
                app.panel_cluster.visible = !app.panel_cluster.visible;
                app.save_station_config();
            }

        if app.quick_access.show_bandmap
            && ui.button(format!("📶 {}", tr("toolbar.bandmap", lang))).on_hover_text("Włącz/wyłącz Panoramę Pasma").clicked() {
                app.panel_bandmap.visible = !app.panel_bandmap.visible;
                app.show_bandmap_window = app.panel_bandmap.visible;
                app.save_station_config();
            }

        if app.quick_access.show_solar
            && ui.button(format!("☀ {}", tr("toolbar.solar", lang))).on_hover_text("Włącz/wyłącz panel warunków kosmicznych Solar").clicked() {
                app.panel_solar.visible = !app.panel_solar.visible;
                app.save_station_config();
            }

        if app.quick_access.show_lotw
            && ui.button(format!("🌐 {}", tr("toolbar.lotw", lang))).on_hover_text("Synchronizacja LoTW / eQSL / Club Log").clicked() {
                app.show_online_sync_window = !app.show_online_sync_window;
            }

        if app.quick_access.show_map
            && ui.button(format!("🗺 {}", tr("toolbar.map", lang))).on_hover_text("Mapa świata & Grayline").clicked() {
                app.panel_world_map.visible = !app.panel_world_map.visible;
                app.show_world_map_window = app.panel_world_map.visible;
                app.save_station_config();
            }

        if app.quick_access.show_awards
            && ui.button(format!("🏆 {}", tr("toolbar.awards", lang))).on_hover_text("Matryca osiągnięć dyplomowych").clicked() {
                app.show_awards_matrix_window = !app.show_awards_matrix_window;
            }

        if app.quick_access.show_cw
            && ui.button(format!("📟 {}", tr("toolbar.cw", lang))).on_hover_text("Otwórz Terminal CW").clicked() {
                app.cw_terminal_dialog.is_open = !app.cw_terminal_dialog.is_open;
            }

        if app.quick_access.show_satellites
            && ui.button(format!("🛰 {}", tr("toolbar.satellites", lang))).on_hover_text("Śledzenie satelitów").clicked() {
                app.panel_satellites.visible = !app.panel_satellites.visible;
                app.show_satellites_window = app.panel_satellites.visible;
                app.save_station_config();
            }

        if app.quick_access.show_contest
            && ui.button(format!("🏁 {}", tr("toolbar.contest", lang))).on_hover_text("Moduł zawodów Contest & Cabrillo").clicked() {
                app.show_contest_window = !app.show_contest_window;
            }

        if app.quick_access.show_equipment
            && ui.button(format!("📋 {}", tr("toolbar.equipment", lang))).on_hover_text("Ewidencja sprzętu radiowego").clicked() {
                app.show_ledger_window = !app.show_ledger_window;
            }

        if app.quick_access.show_eme
            && ui.button("🌙 EME").on_hover_text("Położenie Księżyca i Słońca").clicked() {
                app.astronomy_dialog.open();
            }

        if app.quick_access.show_wol
            && ui.button("⚡ WOL").on_hover_text("Zdalne wybudzenie Wake-on-LAN").clicked() {
                app.wol_dialog.open();
            }

        if app.quick_access.show_theme {
            let theme_btn_text = if app.dark_theme { format!("🌙 {}", tr("toolbar.theme_dark", lang)) } else { format!("☀ {}", tr("toolbar.theme_light", lang)) };
            if ui.button(theme_btn_text).clicked() {
                app.dark_theme = !app.dark_theme;
                app.save_station_config();
            }
        }

        // Przycisk personalizacji paska (oprócz PPM)
        if ui.button(egui::RichText::new("⚙").size(12.0)).on_hover_text("Dostosuj pasek szybkiego dostępu (lub kliknij PPM w dowolnym miejscu paska)").clicked() {
            show_customize_popup = true;
        }
    });

    // Menu kontekstowe pod prawym przyciskiem myszy na pasku narzędziowym
    toolbar_resp.response.context_menu(|ui| {
        render_quick_access_menu(app, ui);
    });

    if show_customize_popup {
        app.show_quick_access_customizer = true;
    }

    if app.show_quick_access_customizer {
        let mut open = true;
        egui::Window::new(format!("⚙ {}", tr("toolbar.customize_title", lang)))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(360.0)
            .show(ui.ctx(), |ui| {
                render_quick_access_menu(app, ui);
            });
        if !open {
            app.show_quick_access_customizer = false;
        }
    }
}

fn render_quick_access_menu(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.heading(tr("toolbar.visible_elements", lang));
    ui.label(egui::RichText::new("Zaznacz elementy, które mają być widoczne w pasku szybkiego dostępu:").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
    ui.separator();

    let mut changed = false;
    egui::Grid::new("qa_customizer_grid").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
        changed |= ui.checkbox(&mut app.quick_access.show_journal, "📁 Dziennik").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_wsjtx, "● Status WSJT-X").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_cluster_status, "📡 Status Klastra").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_vfo, "📻 VFO").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_qso, "📝 QSO").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_log, "📖 Log").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_cat, "📻 CAT").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_cluster, "📡 Cluster").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_bandmap, "📶 Band Map").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_solar, "☀ Solar").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_lotw, "🌐 LoTW").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_map, "🗺 Mapa").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_awards, "🏆 Dyplomy").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_cw, "📟 CW").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_satellites, "🛰 Satelity").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_contest, "🏁 Zawody").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_equipment, "📋 Sprzęt").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_eme, "🌙 EME").changed();
        ui.end_row();

        changed |= ui.checkbox(&mut app.quick_access.show_wol, "⚡ WOL").changed();
        changed |= ui.checkbox(&mut app.quick_access.show_theme, "🌙/☀ Motyw").changed();
        ui.end_row();
    });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr("toolbar.restore_defaults", lang)).clicked() {
            app.quick_access = crate::core::station::QuickAccessConfig::default();
            changed = true;
        }
        if ui.button("Minimalny pasek").clicked() {
            app.quick_access.show_journal = true;
            app.quick_access.show_wsjtx = true;
            app.quick_access.show_cluster_status = true;
            app.quick_access.show_vfo = true;
            app.quick_access.show_qso = true;
            app.quick_access.show_log = true;
            app.quick_access.show_cat = false;
            app.quick_access.show_cluster = false;
            app.quick_access.show_bandmap = false;
            app.quick_access.show_solar = false;
            app.quick_access.show_lotw = false;
            app.quick_access.show_map = false;
            app.quick_access.show_awards = false;
            app.quick_access.show_cw = false;
            app.quick_access.show_satellites = false;
            app.quick_access.show_contest = false;
            app.quick_access.show_equipment = false;
            app.quick_access.show_eme = false;
            app.quick_access.show_wol = false;
            app.quick_access.show_theme = true;
            changed = true;
        }
    });

    if changed {
        app.save_station_config();
    }
}

