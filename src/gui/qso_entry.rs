// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_qso_entry_panel(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new(format!("📝 {}", tr("tab.new_qso", lang))).color(egui::Color32::from_rgb(56, 189, 248)).size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text("Ukryj ten kafelek").clicked() {
                        app.panel_qso.visible = false;
                        app.save_station_config();
                    }
                    if ui.button("↗").on_hover_text("Odepnij do osobnego okna pływającego").clicked() {
                        app.panel_qso.floating = true;
                        app.save_station_config();
                    }
                    if app.cat_connected {
                        if ui.button(egui::RichText::new("● CAT ONLINE").color(egui::Color32::from_rgb(34, 197, 94)).size(11.0).strong()).clicked() {
                            app.show_cat_settings_window = true;
                        }
                    } else {
                        if ui.button(egui::RichText::new("○ CAT OFFLINE").color(egui::Color32::from_rgb(148, 163, 184)).size(11.0)).clicked() {
                            app.show_cat_settings_window = true;
                        }
                    }
                });
            });
            ui.separator();

            render_qso_entry_body(app, ui);
        });
    });
}

pub fn render_qso_entry_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_qso.visible {
        return;
    }

    let lang = app.current_language;

    if app.panel_qso.floating {
        let mut still_open = true;
        let mut dock_back = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("qso_entry_viewport"),
            egui::ViewportBuilder::default()
                .with_title(format!("📝 {} - SPLogbook", tr("tab.new_qso", lang)))
                .with_inner_size([480.0, 460.0])
                .with_min_inner_size([360.0, 360.0]),
            |ctx, _class| {
                egui::TopBottomPanel::top("qso_vp_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("↙ Przypnij do pulpitu").on_hover_text("Przenieś okno z powrotem na główny pulpit SPLogbook").clicked() {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_qso_entry_body(app, ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if dock_back {
            app.panel_qso.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_qso.visible = false;
            app.panel_qso.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.panel_qso.visible;
    let screen = ctx.available_rect();
    let default_pos = [screen.min.x + 8.0, screen.min.y + 265.0];
    let default_size = [470.0, (screen.height() - 275.0).max(420.0)];

    let mut win = egui::Window::new(egui::RichText::new(format!("   📝 {}", tr("tab.new_qso", lang))).size(12.0).strong())
        .open(&mut open)
        .min_size([340.0, 380.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_qso.saved_pos {
        let sz = app.panel_qso.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_qso_entry_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "qso_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_qso.floating, true);
        if app.panel_qso.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_qso.saved_pos != Some(new_pos) || app.panel_qso.saved_size != Some(new_size) {
                app.panel_qso.saved_pos = Some(new_pos);
                app.panel_qso.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !open {
        app.panel_qso.visible = false;
        app.save_station_config();
    }
}


pub fn render_qso_entry_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

            let is_dupe = !app.entry_callsign.is_empty()
                && app.past_qsos_for_active_call.iter().any(|q| q.band == app.entry_band && q.mode == app.entry_mode);

            // Znak korespondenta & natychmiastowe wykrywanie duplikatów
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr("qso.callsign", lang)).strong().size(13.0));
                if is_dupe {
                    ui.colored_label(egui::Color32::from_rgb(239, 68, 68), egui::RichText::new("⚠ DUPE!").strong());
                }
            });

            ui.horizontal(|ui| {
                let call_response = ui.add(
                    egui::TextEdit::singleline(&mut app.entry_callsign)
                        .font(egui::TextStyle::Heading)
                        .hint_text("W1AW, SP6INA, DL1ABC")
                        .desired_width(ui.available_width() - 85.0)
                );

                if app.focus_callsign_requested {
                    call_response.request_focus();
                    app.focus_callsign_requested = false;
                }

                if call_response.changed() {
                    app.entry_callsign = app.entry_callsign.to_uppercase();
                    app.on_callsign_changed();
                }

                if ui.button(egui::RichText::new("🔍 QRZ").strong()).on_hover_text("Pobierz dane z bazy QRZ.com / Callbook").clicked() {
                    app.lookup_active_callsign_online();
                }
            });

            // Podpowiedzi Super Check Partial (SCP)
            if !app.scp_suggestions.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("SCP:").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    for call in app.scp_suggestions.clone() {
                        if ui.button(egui::RichText::new(&call).color(egui::Color32::from_rgb(56, 189, 248)).size(11.0).monospace()).clicked() {
                            app.entry_callsign = call;
                            app.on_callsign_changed();
                        }
                    }
                });
            }

            // Badges informacji o kraju DXCC, prefiksie WPX i dyplomach
            ui.horizontal_wrapped(|ui| {
                if let Some(ref info) = app.active_prefix_info {
                    ui.colored_label(egui::Color32::from_rgb(147, 197, 253), format!("📍 {} ({})", info.country, info.wpx_prefix));
                    ui.colored_label(egui::Color32::from_rgb(216, 180, 254), format!("Strefy: CQ {} / ITU {}", info.cqz, info.ituz));
                    
                    if let Some(ref dist) = app.active_polish_district {
                        ui.colored_label(egui::Color32::from_rgb(134, 239, 172), format!("🇵🇱 Okręg PZK SP{}", dist.district));
                    }
                    if app.active_distance_km > 0.0 {
                        ui.colored_label(egui::Color32::from_rgb(253, 224, 71), format!("🧭 {:.0} km | {:.0}°", app.active_distance_km, app.active_bearing_deg));
                        if ui.small_button(format!("🔄 Obróć ({:.0}°)", app.active_bearing_deg))
                            .on_hover_text("Ustawia rotor antenowy na azymut korespondenta (Hamlib rotctld)")
                            .clicked()
                        {
                            app.rotate_antenna_to(app.active_bearing_deg as f32);
                        }
                    }
                }
            });

            // Kluby krótkofalarskie (SP-OTC, PGA, SKCC, CWOPS, FOC)
            if !app.active_clubs.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for club in &app.active_clubs {
                        let col = egui::Color32::from_rgb(club.badge_color.0, club.badge_color.1, club.badge_color.2);
                        let text = match club.number {
                            Some(num) => format!("🎖 {} {}", club.code, num),
                            None => format!("🎖 {}", club.code),
                        };
                        ui.colored_label(col, egui::RichText::new(text).strong())
                            .on_hover_text(club.name);
                    }
                });
            }

            // Prognoza propagacji HF w czasie rzeczywistym
            if let Some(ref prop) = app.active_propagation {
                ui.horizontal_wrapped(|ui| {
                    let prop_color = match prop.status {
                        crate::core::propagation::BandOpeningStatus::Open => egui::Color32::from_rgb(34, 197, 94),
                        crate::core::propagation::BandOpeningStatus::Marginal => egui::Color32::from_rgb(250, 204, 21),
                        crate::core::propagation::BandOpeningStatus::Closed => egui::Color32::from_rgb(239, 68, 68),
                    };
                    let text = format!("📡 Propagacja: {}% REL · {} ({}) · MUF {:.1} MHz",
                        prop.reliability_pct, prop.signal_s_units, prop.layer, prop.muf_mhz);
                    ui.colored_label(prop_color, egui::RichText::new(text).strong())
                        .on_hover_text(format!(
                            "Prognoza HF VOACAP-lite:\nNiezawodność obwodu: {}%\nSiła sygnału: {}\nWarstwa: {}\nMUF: {:.1} MHz\nLUF: {:.1} MHz\nFOT/OWF: {:.1} MHz\nStatus: {}",
                            prop.reliability_pct, prop.signal_s_units, prop.layer, prop.muf_mhz, prop.luf_mhz, prop.fot_mhz, prop.status.as_str()
                        ));
                });
            }

            // Status dyplomowy (ATNO / New Band / Worked B4 / WAZ / WAS / WAC / IOTA / PGA)
            if let Some(ref award_st) = app.active_award_status {
                ui.horizontal_wrapped(|ui| {
                    if award_st.is_worked_b4 {
                        ui.label(egui::RichText::new("⚠ WORKED BEFORE (B4)").color(egui::Color32::from_rgb(251, 146, 60)).strong());
                    } else {
                        if award_st.is_new_dxcc {
                            ui.label(egui::RichText::new("⭐ NEW DXCC (ATNO)!").color(egui::Color32::from_rgb(250, 204, 21)).strong());
                        }
                        if award_st.is_new_band {
                            ui.label(egui::RichText::new("✨ NEW BAND!").color(egui::Color32::from_rgb(52, 211, 153)).strong());
                        }
                        if award_st.is_new_mode {
                            ui.label(egui::RichText::new("🔹 NEW MODE!").color(egui::Color32::from_rgb(56, 189, 248)).strong());
                        }
                        if award_st.is_new_waz {
                            ui.label(egui::RichText::new("🌐 NEW WAZ (ZONE)!").color(egui::Color32::from_rgb(168, 85, 247)).strong());
                        }
                        if award_st.is_new_was {
                            ui.label(egui::RichText::new("🇺🇸 NEW STATE (WAS)!").color(egui::Color32::from_rgb(244, 63, 94)).strong());
                        }
                        if award_st.is_new_wac {
                            ui.label(egui::RichText::new("🌍 NEW CONTINENT (WAC)!").color(egui::Color32::from_rgb(234, 179, 8)).strong());
                        }
                        if award_st.is_new_iota {
                            ui.label(egui::RichText::new("🏝 NEW IOTA!").color(egui::Color32::from_rgb(20, 184, 166)).strong());
                        }
                        if award_st.is_new_pga {
                            ui.label(egui::RichText::new("🇵🇱 NOWA GMINA PGA!").color(egui::Color32::from_rgb(34, 197, 94)).strong());
                        }
                    }
                });
            }

            ui.add_space(4.0);

            // 2. Pasmo i Emisja
            ui.horizontal(|ui| {
                ui.label(tr("qso.band", lang));
                egui::ComboBox::from_id_salt("qso_band")
                    .selected_text(&app.entry_band)
                    .show_ui(ui, |ui| {
                        for b in &["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "4m", "2m", "70cm", "23cm"] {
                            ui.selectable_value(&mut app.entry_band, b.to_string(), *b);
                        }
                    });

                ui.add_space(10.0);

                ui.label(tr("qso.mode", lang));
                egui::ComboBox::from_id_salt("qso_mode")
                    .selected_text(&app.entry_mode)
                    .show_ui(ui, |ui| {
                        for m in &["CW", "SSB", "FT8", "FT4", "RTTY", "PSK31", "AM", "FM"] {
                            ui.selectable_value(&mut app.entry_mode, m.to_string(), *m);
                        }
                    });
            });

            // 3. Raporty RST
            ui.horizontal(|ui| {
                ui.label(tr("qso.rst_sent", lang));
                ui.add(egui::TextEdit::singleline(&mut app.entry_rst_sent).desired_width(60.0));

                ui.add_space(10.0);

                ui.label(tr("qso.rst_rcvd", lang));
                ui.add(egui::TextEdit::singleline(&mut app.entry_rst_rcvd).desired_width(60.0));
            });

            // 4. Imię operatora & Lokator QTH
            ui.horizontal(|ui| {
                ui.label(tr("qso.name", lang));
                ui.add(egui::TextEdit::singleline(&mut app.entry_name).desired_width(120.0));

                ui.add_space(10.0);

                ui.label(tr("qso.locator", lang));
                let loc_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_grid).desired_width(80.0));
                if loc_resp.changed() {
                    app.entry_grid = app.entry_grid.to_uppercase();
                    app.recalculate_distance_from_grid();
                }
            });

            // 5. Miejscowość (QTH) & Gmina PGA
            ui.horizontal(|ui| {
                ui.label(tr("qso.qth", lang));
                let qth_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_qth).desired_width(120.0));
                if qth_resp.changed() && app.entry_pga.is_empty() {
                    if let Some(gmina) = crate::core::pga::suggest_pga_for_qth(&app.entry_qth) {
                        app.entry_pga = gmina.code.to_string();
                    }
                }

                ui.add_space(8.0);

                ui.label(tr("qso.pga", lang));
                let pga_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_pga).desired_width(65.0));
                if pga_resp.changed() {
                    app.entry_pga = app.entry_pga.to_uppercase();
                }
            });

            // Podgląd nazwy gminy PGA jeśli kod jest wpisany
            if !app.entry_pga.is_empty() {
                if let Some(gmina) = crate::core::pga::find_by_code(&app.entry_pga) {
                    ui.label(egui::RichText::new(format!("🇵🇱 Gmina: {} (powiat {})", gmina.name, gmina.powiat)).small().color(egui::Color32::from_rgb(134, 239, 172)));
                }
            }

            // Referencje dyplomowe i lokalizacyjne
            ui.horizontal(|ui| {
                ui.label(format!("🏝 {}:", tr("qso.iota", lang)));
                let iota_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_iota).desired_width(65.0).hint_text("EU-001"));
                if iota_resp.changed() {
                    app.entry_iota = app.entry_iota.to_uppercase();
                }
                if ui.button("🔍").on_hover_text("Przeglądaj bazę wysp IOTA").clicked() {
                    app.iota_dialog.open();
                }

                ui.add_space(4.0);

                ui.label(format!("🗺 {}:", tr("qso.state", lang)));
                let st_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_state).desired_width(45.0).hint_text("CA"));
                if st_resp.changed() {
                    app.entry_state = app.entry_state.to_uppercase();
                }
                if ui.button("🔍").on_hover_text("Przeglądaj stany US/VE i okręgi").clicked() {
                    app.states_dialog.open();
                }

                ui.add_space(4.0);

                ui.label(format!("🏔 {}:", tr("qso.sota", lang)));
                let sota_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_sota).desired_width(75.0).hint_text("SP/BZ-001"));
                if sota_resp.changed() {
                    app.entry_sota = app.entry_sota.to_uppercase();
                }
            });

            // QSL Manager i dane stacji
            ui.horizontal(|ui| {
                ui.label(format!("📋 {}:", tr("qso.qsl_via", lang)));
                let mgr_resp = ui.add(egui::TextEdit::singleline(&mut app.entry_qsl_manager).desired_width(80.0).hint_text("Manager"));
                if mgr_resp.changed() {
                    app.entry_qsl_manager = app.entry_qsl_manager.to_uppercase();
                }
                if ui.button("🔍").on_hover_text("Szukaj managera w bazie 29k").clicked() {
                    let call = app.entry_callsign.clone();
                    let sdb = app.service_db.clone();
                    app.qsl_manager_dialog.open_for_call(&call, &sdb);
                }

                if let Some(ref mgr_rec) = app.active_qsl_manager_record {
                    ui.colored_label(egui::Color32::from_rgb(34, 197, 94), format!("✓ Mgr: {}", mgr_rec.manager));
                }

                if ui.button("📷 Foto").on_hover_text("Pokaż zdjęcie stacji").clicked() {
                    let call = app.entry_callsign.clone();
                    app.photo_viewer_dialog.open(&call, None);
                }
            });

            // Szybkie sterowanie rotatorem antenowym (SP / LP)
            if app.active_bearing_deg > 0.0 {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{}:", tr("geo.azimuth", lang))).small().color(egui::Color32::GRAY));
                    if ui.button(egui::RichText::new(format!("🧭 SP ({:.0}°)", app.active_bearing_deg)).small().strong()).clicked() {
                        app.turn_rotor_short_path();
                    }
                    if ui.button(egui::RichText::new(format!("🔄 LP ({:.0}°)", (app.active_bearing_deg + 180.0) % 360.0)).small()).clicked() {
                        app.turn_rotor_long_path();
                    }
                });
            }

            // 6. Komentarz i nagrywanie audio łączności
            ui.horizontal(|ui| {
                ui.label(tr("qso.comment", lang));
                ui.add(egui::TextEdit::singleline(&mut app.entry_comment).desired_width(ui.available_width() - 85.0));

                let is_rec = crate::media::audio_recorder::AudioRecorder::is_recording();
                if is_rec {
                    if ui.button(egui::RichText::new("⏹ STOP").color(egui::Color32::WHITE).strong()).on_hover_text("Zatrzymaj nagrywanie audio").clicked() {
                        let _ = crate::media::audio_recorder::AudioRecorder::stop_audio();
                    }
                } else {
                    if ui.button(egui::RichText::new("● REC").color(egui::Color32::from_rgb(239, 68, 68)).strong()).on_hover_text("Rozpocznij nagrywanie audio łączności (Audio Memo)").clicked() {
                        let _ = crate::media::audio_recorder::AudioRecorder::start_recording();
                    }
                }
            });

            ui.add_space(6.0);

            // 7. Przyciski Zapisz (Enter) / Wyczyść (Esc)
            ui.horizontal(|ui| {
                let save_btn = ui.add_sized(
                    [ui.available_width() * 0.65, 30.0],
                    egui::Button::new(egui::RichText::new(format!("💾 {}", tr("qso.save", lang))).strong().color(egui::Color32::from_rgb(15, 23, 42)))
                        .fill(egui::Color32::from_rgb(56, 189, 248))
                );
                if save_btn.clicked() || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && !app.entry_callsign.is_empty()) {
                    app.save_qso();
                }

                let clear_btn = ui.add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new(tr("qso.clear", lang))
                );
                if clear_btn.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    app.clear_qso_form();
                }
            });

            // 8. Sekcja Poprzednich Łączności (Worked Before / B4) z wpisywaną stacją
            if !app.past_qsos_for_active_call.is_empty() {
                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new(format!("📜 {} ({})", tr("qso.worked_before", lang), app.past_qsos_for_active_call.len()))
                    .strong().size(12.0).color(egui::Color32::from_rgb(251, 191, 36)));

                egui::ScrollArea::vertical()
                    .max_height(80.0)
                    .show(ui, |ui| {
                        egui::Grid::new("b4_grid").striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                            for prev in &app.past_qsos_for_active_call {
                                let is_dupe = prev.band == app.entry_band && prev.mode == app.entry_mode;
                                let color = if is_dupe {
                                    egui::Color32::from_rgb(239, 68, 68)
                                } else {
                                    egui::Color32::from_rgb(148, 163, 184)
                                };

                                ui.label(egui::RichText::new(&prev.qso_date).color(color).size(11.0));
                                ui.label(egui::RichText::new(&prev.band).strong().color(color).size(11.0));
                                ui.label(egui::RichText::new(&prev.mode).color(color).size(11.0));
                                ui.label(egui::RichText::new(format!("{}/{}", prev.rst_sent, prev.rst_rcvd)).size(11.0));
                                if is_dupe {
                                    ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "DUPE!");
                                } else {
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                        });
                    });
            }
}
