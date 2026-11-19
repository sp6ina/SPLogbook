// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::{RADIUS_MD, SPACE_MD, TEXT_BODY, TEXT_CAPTION, ThemeExt, callsign_font};
use eframe::egui;

pub fn render_qso_entry_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.views.panel_qso.visible {
        return;
    }

    let lang = app.gui.appearance.language;

    if app.views.panel_qso.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("qso_entry_viewport"),
            format!("📝 {} - SPLogbook", tr(key::TAB_NEW_QSO, lang)),
            [480.0, 460.0],
            [360.0, 360.0],
            app.views.panel_qso.saved_pos,
            app.views.panel_qso.saved_size,
            |app, ui| {
                egui::Panel::top("qso_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let lang = app.gui.appearance.language;
                        if ui
                            .button(format!("↙ {}", tr(key::WINDOW_DOCK, lang)))
                            .on_hover_text(tr(key::WINDOW_DOCK_TOOLTIP, lang))
                            .clicked()
                        {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_qso_entry_body(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo
            && (app.views.panel_qso.saved_pos != Some(pos)
                || app.views.panel_qso.saved_size != Some(size))
        {
            app.views.panel_qso.saved_pos = Some(pos);
            app.views.panel_qso.saved_size = Some(size);
            app.save_station_config();
        }
        if dock_back {
            app.views.panel_qso.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.views.panel_qso.visible = false;
            app.views.panel_qso.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.views.panel_qso.visible;
    let screen = ctx.content_rect();
    let default_pos = [screen.min.x + 8.0, screen.min.y + 265.0];
    let default_size = [470.0, (screen.height() - 275.0).max(420.0)];

    let mut win = egui::Window::new(
        egui::RichText::new(format!("   📝 {}", tr(key::TAB_NEW_QSO, lang)))
            .size(12.0)
            .strong(),
    )
    .open(&mut open)
    .min_size([340.0, 380.0])
    .resizable(true)
    .collapsible(true)
    .constrain_to(screen);

    if app.gui.workspace.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.views.panel_qso.saved_pos {
        let sz = app.views.panel_qso.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_qso_entry_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "qso_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.views.panel_qso.floating,
            true,
            app.gui.appearance.language,
        );
        if app.views.panel_qso.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let rect = res.response.rect;
            let new_pos = [rect.min.x, rect.min.y];
            let new_size = [rect.width(), rect.height()];
            if app.views.panel_qso.saved_pos != Some(new_pos)
                || app.views.panel_qso.saved_size != Some(new_size)
            {
                app.views.panel_qso.saved_pos = Some(new_pos);
                app.views.panel_qso.saved_size = Some(new_size);
            }
            if res.response.drag_stopped() {
                app.save_station_config();
            }
        }
    }

    if !open {
        app.views.panel_qso.visible = false;
        app.save_station_config();
    }
}

pub fn render_qso_entry_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;
    let palette = ui.palette();
    // Odpowiedzi jednowierszowych pól tego panelu. Enter=Zapisz / Esc=Wyczyść działają
    // wyłącznie, gdy fokus klawiatury był faktycznie w tym panelu: jednowierszowe
    // `TextEdit` oddaje fokus przy Enter/Esc, więc wzorzec to `lost_focus() && key_pressed`.
    let mut field_responses: Vec<egui::Response> = Vec::new();
    // Stan popupów (np. rozwinięty `ComboBox`) z początku klatki: Esc zamykający popup
    // nie może przy okazji wyczyścić formularza.
    let popup_open_at_start = egui::Popup::is_any_open(ui.ctx());

    let is_dupe = !app.entry.callsign.is_empty()
        && app
            .entry
            .past_qsos_for_active_call
            .iter()
            .any(|q| q.band == app.entry.band && q.mode == app.entry.mode);

    // Znak korespondenta & natychmiastowe wykrywanie duplikatów
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(key::QSO_CALLSIGN, lang))
                .strong()
                .size(TEXT_BODY),
        );
        if is_dupe {
            ui.colored_label(palette.dupe, egui::RichText::new("⚠ DUPE!").strong());
        }
    });
    ui.horizontal(|ui| {
        let call_response = ui.add(
            egui::TextEdit::singleline(&mut app.entry.callsign)
                .id(egui::Id::new("qso_entry_callsign"))
                .font(callsign_font())
                .hint_text("W1AW, SP6INA, DL1ABC")
                .desired_width(ui.available_width()),
        );

        if app.entry.focus_callsign_requested {
            call_response.request_focus();
            app.entry.focus_callsign_requested = false;
        }

        if call_response.changed() {
            app.entry.callsign = app.entry.callsign.to_uppercase();
            app.on_callsign_changed();
        }

        field_responses.push(call_response.clone());
    });
    ui.add_space(4.0);

    // 2. Pasmo i Emisja
    ui.horizontal(|ui| {
        ui.label(tr(key::QSO_BAND, lang));
        egui::ComboBox::from_id_salt("qso_band")
            .selected_text(&app.entry.band)
            .show_ui(ui, |ui| {
                for band in &[
                    "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m",
                    "4m", "2m", "70cm", "23cm",
                ] {
                    ui.selectable_value(&mut app.entry.band, band.to_string(), *band);
                }
            });

        ui.add_space(10.0);

        ui.label(tr(key::QSO_MODE, lang));
        egui::ComboBox::from_id_salt("qso_mode")
            .selected_text(&app.entry.mode)
            .show_ui(ui, |ui| {
                for mode in &["CW", "SSB", "FT8", "FT4", "RTTY", "PSK31", "AM", "FM"] {
                    ui.selectable_value(&mut app.entry.mode, mode.to_string(), *mode);
                }
            });
    });
    // 3. Raporty RST
    ui.horizontal(|ui| {
        ui.label(tr(key::QSO_RST_SENT, lang));
        let rst_sent_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.rst_sent)
                .id(egui::Id::new("qso_entry_rst_sent"))
                .desired_width(60.0),
        );

        ui.add_space(10.0);

        ui.label(tr(key::QSO_RST_RCVD, lang));
        let rst_rcvd_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.rst_rcvd)
                .id(egui::Id::new("qso_entry_rst_rcvd"))
                .desired_width(60.0),
        );
        field_responses.push(rst_sent_resp.clone());
        field_responses.push(rst_rcvd_resp.clone());
    });
    // 4. Imię operatora & Lokator QTH
    ui.horizontal(|ui| {
        ui.label(tr(key::QSO_NAME, lang));
        let name_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.name)
                .id(egui::Id::new("qso_entry_name"))
                .desired_width(120.0),
        );
        field_responses.push(name_resp.clone());

        ui.add_space(10.0);

        ui.label(tr(key::QSO_LOCATOR, lang));
        let loc_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.grid)
                .id(egui::Id::new("qso_entry_grid"))
                .desired_width(80.0),
        );
        if loc_resp.changed() {
            app.entry.grid = app.entry.grid.to_uppercase();
            app.recalculate_distance_from_grid();
        }
        field_responses.push(loc_resp.clone());
    });
    // 5. Miejscowość (QTH) & Gmina PGA
    ui.horizontal(|ui| {
        ui.label(tr(key::QSO_QTH, lang));
        let qth_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.qth)
                .id(egui::Id::new("qso_entry_qth"))
                .desired_width(120.0),
        );
        if qth_resp.changed()
            && app.entry.pga.is_empty()
            && let Some(gmina) = crate::core::pga::suggest_pga_for_qth(&app.entry.qth)
        {
            app.entry.pga = gmina.code.to_string();
        }

        ui.add_space(8.0);

        ui.label(tr(key::QSO_PGA, lang));
        let pga_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.pga)
                .id(egui::Id::new("qso_entry_pga"))
                .desired_width(65.0),
        );
        if pga_resp.changed() {
            app.entry.pga = app.entry.pga.to_uppercase();
        }
        field_responses.push(qth_resp.clone());
        field_responses.push(pga_resp.clone());
    });
    // Referencje dyplomowe i lokalizacyjne
    ui.horizontal(|ui| {
        ui.label(format!("🏝 {}:", tr(key::QSO_IOTA, lang)));
        let iota_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.iota)
                .id(egui::Id::new("qso_entry_iota"))
                .desired_width(65.0)
                .hint_text("EU-001"),
        );
        if iota_resp.changed() {
            app.entry.iota = app.entry.iota.to_uppercase();
        }
        if side_button(ui, "🔍")
            .on_hover_text(tr(key::QSO_BROWSE_IOTA_TOOLTIP, lang))
            .clicked()
        {
            app.gui.dialogs.iota_dialog.open();
        }

        ui.add_space(4.0);

        ui.label(format!("🗺 {}:", tr(key::QSO_STATE, lang)));
        let st_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.state)
                .id(egui::Id::new("qso_entry_state"))
                .desired_width(45.0)
                .hint_text("CA"),
        );
        if st_resp.changed() {
            app.entry.state = app.entry.state.to_uppercase();
        }
        if side_button(ui, "🔍")
            .on_hover_text(tr(key::QSO_BROWSE_STATES_TOOLTIP, lang))
            .clicked()
        {
            app.gui.dialogs.states_dialog.open();
        }

        ui.add_space(4.0);

        ui.label(format!("🏔 {}:", tr(key::QSO_SOTA, lang)));
        let sota_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.sota)
                .id(egui::Id::new("qso_entry_sota"))
                .desired_width(75.0)
                .hint_text("SP/BZ-001"),
        );
        if sota_resp.changed() {
            app.entry.sota = app.entry.sota.to_uppercase();
        }
        field_responses.push(iota_resp.clone());
        field_responses.push(st_resp.clone());
        field_responses.push(sota_resp.clone());
    });

    // F-19: własne referencje stacji logującej (kontekst aktywacji).
    // Osobne pola, osobna strona QSO — nigdy nie są kopiowane do pól korespondenta.
    ui.horizontal(|ui| {
        ui.label(format!("🏔 {}:", tr(key::QSO_MY_SOTA, lang)));
        let my_sota_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.my_refs.sota)
                .id(egui::Id::new("qso_entry_my_sota"))
                .desired_width(75.0)
                .hint_text("SP/BZ-001"),
        );
        if my_sota_resp.changed() {
            app.entry.my_refs.sota = app.entry.my_refs.sota.to_uppercase();
        }
        ui.label(format!("🌲 {}:", tr(key::QSO_MY_POTA, lang)));
        let my_pota_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.my_refs.pota)
                .id(egui::Id::new("qso_entry_my_pota"))
                .desired_width(80.0)
                .hint_text("SP-0123"),
        );
        if my_pota_resp.changed() {
            app.entry.my_refs.pota = app.entry.my_refs.pota.to_uppercase();
        }
        field_responses.push(my_sota_resp.clone());
        field_responses.push(my_pota_resp.clone());
    });
    // QSL Manager i dane stacji
    ui.horizontal(|ui| {
        ui.label(format!("📋 {}:", tr(key::QSO_QSL_VIA, lang)));
        let mgr_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.qsl_manager)
                .id(egui::Id::new("qso_entry_qsl_via"))
                .desired_width(80.0)
                .hint_text("Manager"),
        );
        if mgr_resp.changed() {
            app.entry.qsl_manager = app.entry.qsl_manager.to_uppercase();
        }
        if side_button(ui, "🔍")
            .on_hover_text(tr(key::QSO_SEARCH_MANAGER_TOOLTIP, lang))
            .clicked()
        {
            let call = app.entry.callsign.clone();
            app.gui.dialogs.qsl_manager_dialog.open_for_call(&call);
        }

        if let Some(ref mgr_rec) = app.entry.active_qsl_manager_record {
            ui.colored_label(palette.success, format!("✓ Mgr: {}", mgr_rec.manager));
        }

        if side_button(ui, tr(key::QSO_BTN_PHOTO, lang))
            .on_hover_text(tr(key::QSO_SHOW_PHOTO_TOOLTIP, lang))
            .clicked()
        {
            let call = app.entry.callsign.clone();
            let photo_url = app.gui.dialogs.photo_viewer_dialog.photo_url.clone();
            app.gui.dialogs.photo_viewer_dialog.open(&call, photo_url);
        }
        field_responses.push(mgr_resp.clone());
    });
    // 6. Komentarz i nagrywanie audio łączności
    ui.horizontal(|ui| {
        ui.label(tr(key::QSO_COMMENT, lang));
        let comment_resp = ui.add(
            egui::TextEdit::singleline(&mut app.entry.comment)
                .id(egui::Id::new("qso_entry_comment"))
                .desired_width(ui.available_width() - 85.0),
        );

        let is_rec = crate::media::audio_recorder::AudioRecorder::is_recording();
        if is_rec {
            if side_button(
                ui,
                egui::RichText::new("⏹ STOP")
                    .color(palette.text_primary)
                    .strong(),
            )
            .on_hover_text(tr(key::QSO_STOP_RECORDING_TOOLTIP, lang))
            .clicked()
            {
                let _ = crate::media::audio_recorder::AudioRecorder::stop_recording();
            }
        } else if side_button(
            ui,
            egui::RichText::new("● REC").color(palette.danger).strong(),
        )
        .on_hover_text(tr(key::QSO_START_RECORDING_TOOLTIP, lang))
        .clicked()
        {
            let _ = crate::media::audio_recorder::AudioRecorder::start_recording();
        }
        field_responses.push(comment_resp.clone());
    });

    ui.add_space(SPACE_MD);

    // 7. Przyciski Zapisz / Wyczyść (Enter=Zapisz, Esc=Wyczyść z pól formularza)
    ui.horizontal(|ui| {
        let save_btn = ui.add_sized(
            [ui.available_width() * 0.65, 30.0],
            egui::Button::new(
                egui::RichText::new(format!("💾 {}", tr(key::QSO_SAVE, lang)))
                    .strong()
                    .color(palette.selection_text),
            )
            .fill(palette.accent)
            .corner_radius(RADIUS_MD),
        );
        if save_btn.clicked() {
            app.save_qso();
        }

        let clear_btn = ui.add_sized(
            [ui.available_width(), 30.0],
            egui::Button::new(tr(key::QSO_CLEAR, lang)),
        );
        if clear_btn.clicked() {
            app.clear_qso_form();
        }
    });

    handle_form_keys(app, ui, &field_responses, popup_open_at_start);

    // ── Strefa dynamiczna ──────────────────────────────────────────────────────────
    // Wszystko poniżej może zmieniać wysokość (podpowiedzi, odznaki, propagacja), ale
    // leży pod polami wpisu, więc nie przesuwa ich pod kursorem ani nie wstrzykuje
    // dodatkowych stopów `Tab` między Znak a RST.
    ui.add_space(SPACE_MD);
    ui.horizontal(|ui| {
        if side_button(ui, egui::RichText::new("🔍 QRZ").strong())
            .on_hover_text(tr(key::QSO_FETCH_QRZ_TOOLTIP, lang))
            .clicked()
        {
            app.lookup_active_callsign_online();
        }
    });

    // Podpowiedzi Super Check Partial (SCP)
    if !app.entry.scp_suggestions.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("SCP:")
                    .size(TEXT_CAPTION)
                    .color(palette.text_muted),
            );
            for call in app.entry.scp_suggestions.clone() {
                if side_button(
                    ui,
                    egui::RichText::new(&call)
                        .color(palette.accent)
                        .size(TEXT_CAPTION)
                        .monospace(),
                )
                .clicked()
                {
                    app.entry.callsign = call;
                    app.on_callsign_changed();
                }
            }
        });
    }

    // Lokalna korekta rozmyta (fuzzy) — propozycje poprawy błędnie wpisanego znaku
    if !app.entry.callsign_corrections.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(tr(key::QSO_DID_YOU_MEAN, lang))
                    .size(11.0)
                    .color(palette.warning),
            );
            for (call, _dist) in app.entry.callsign_corrections.clone() {
                if side_button(
                    ui,
                    egui::RichText::new(&call)
                        .color(palette.warning)
                        .size(11.0)
                        .monospace(),
                )
                .clicked()
                {
                    app.entry.callsign = call;
                    app.on_callsign_changed();
                }
            }
        });
    }
    // Badges informacji o kraju DXCC, prefiksie WPX i dyplomach
    ui.horizontal_wrapped(|ui| {
        if let Some(ref info) = app.entry.active_prefix_info {
            ui.colored_label(
                palette.accent,
                format!("📍 {} ({})", info.country, info.wpx_prefix),
            );
            ui.colored_label(
                palette.accent_alt,
                Msg::new(key::QSO_ZONES_CQ_ITU)
                    .arg("cqz", info.cqz)
                    .arg("ituz", info.ituz)
                    .text(lang),
            );

            if let Some(ref dist) = app.entry.active_polish_district {
                ui.colored_label(
                    palette.success,
                    Msg::new(key::QSO_PZK_DISTRICT_LABEL)
                        .arg("district", dist.district)
                        .text(lang),
                );
            }
            if app.entry.active_distance_km > 0.0 {
                ui.colored_label(
                    palette.text_primary,
                    format!(
                        "🧭 {} | {:.0}°",
                        app.active_distance_display(),
                        app.entry.active_bearing_deg
                    ),
                );
                if side_small_button(
                    ui,
                    Msg::new(key::QSO_ROTATE_BEARING)
                        .arg("bearing", format!("{:.0}", app.entry.active_bearing_deg))
                        .text(lang),
                )
                .on_hover_text(tr(key::QSO_ROTOR_TOOLTIP, lang))
                .clicked()
                {
                    app.rotate_antenna_to(app.entry.active_bearing_deg as f32);
                }
            }
        }
    });
    // Kluby krótkofalarskie (SP-OTC, PGA, SKCC, CWOPS, FOC)
    if !app.entry.active_clubs.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for club in &app.entry.active_clubs {
                let col = match club.code.as_ref() {
                    "SP-OTC" => palette.danger,
                    "SPCWC" => palette.accent,
                    "CWOPS" => palette.warning,
                    "FOC" => palette.accent_alt,
                    _ => palette.info,
                };
                let text = match club.number.as_deref() {
                    Some(num) => format!("🎖 {} {}", club.code, num),
                    None => format!("🎖 {}", club.code),
                };
                ui.colored_label(col, egui::RichText::new(text).strong())
                    .on_hover_text(&*club.name);
            }
        });
    }

    // Prognoza propagacji HF w czasie rzeczywistym
    if let Some(ref prop) = app.entry.active_propagation {
        ui.horizontal_wrapped(|ui| {
            let prop_color = match prop.status {
                crate::core::propagation::BandOpeningStatus::Open => palette.success,
                crate::core::propagation::BandOpeningStatus::Marginal => palette.warning,
                crate::core::propagation::BandOpeningStatus::Closed => palette.danger,
            };
            let text = Msg::new(key::QSO_PROPAGATION_SUMMARY)
                .arg("rel", prop.reliability_pct)
                .arg("signal", &prop.signal_s_units)
                .arg("layer", &prop.layer)
                .arg("muf", format!("{:.1}", prop.muf_mhz))
                .text(lang);
            ui.colored_label(prop_color, egui::RichText::new(text).strong())
                .on_hover_text(format!(
                    "HF VOACAP-lite:\nREL: {}%\nS-Meter: {}\nLayer: {}\nMUF: {:.1} MHz\nLUF: {:.1} MHz\nFOT/OWF: {:.1} MHz\nStatus: {}",
                    prop.reliability_pct,
                    prop.signal_s_units,
                    prop.layer,
                    prop.muf_mhz,
                    prop.luf_mhz,
                    prop.fot_mhz,
                    prop.status.as_str()
                ));
        });
    }
    // Status dyplomowy (ATNO / New Band / Worked B4 / WAZ / WAS / WAC / IOTA / PGA)
    if let Some(ref award_st) = app.entry.active_award_status {
        ui.horizontal_wrapped(|ui| {
            if award_st.is_worked_b4 {
                ui.label(
                    egui::RichText::new("⚠ WORKED BEFORE (B4)")
                        .color(palette.warning)
                        .strong(),
                );
            } else {
                if award_st.is_new_dxcc {
                    ui.label(
                        egui::RichText::new("⭐ NEW DXCC (ATNO)!")
                            .color(palette.multiplier)
                            .strong(),
                    );
                }
                if award_st.is_new_band {
                    ui.label(
                        egui::RichText::new("✨ NEW BAND!")
                            .color(palette.success)
                            .strong(),
                    );
                }
                if award_st.is_new_mode {
                    ui.label(
                        egui::RichText::new("🔹 NEW MODE!")
                            .color(palette.accent)
                            .strong(),
                    );
                }
                if award_st.is_new_waz {
                    ui.label(
                        egui::RichText::new("🌐 NEW WAZ (ZONE)!")
                            .color(palette.accent_alt)
                            .strong(),
                    );
                }
                if award_st.is_new_was {
                    ui.label(
                        egui::RichText::new("🇺🇸 NEW STATE (WAS)!")
                            .color(palette.series(4))
                            .strong(),
                    );
                }
                if award_st.is_new_wac {
                    ui.label(
                        egui::RichText::new("🌍 NEW CONTINENT (WAC)!")
                            .color(palette.warning)
                            .strong(),
                    );
                }
                if award_st.is_new_iota {
                    ui.label(
                        egui::RichText::new("🏝 NEW IOTA!")
                            .color(palette.series(5))
                            .strong(),
                    );
                }
                if award_st.is_new_pga {
                    ui.label(
                        egui::RichText::new(tr(key::QSO_NEW_PGA_GMINA, lang))
                            .color(palette.success)
                            .strong(),
                    );
                }
            }
        });
    }
    // Podgląd nazwy gminy PGA jeśli kod jest wpisany
    if !app.entry.pga.is_empty()
        && let Some(gmina) = crate::core::pga::find_by_code(&app.entry.pga)
    {
        ui.label(
            egui::RichText::new(format!(
                "🇵🇱 Gmina: {} (powiat {})",
                gmina.name, gmina.powiat
            ))
            .small()
            .color(palette.success),
        );
    }

    // Szybkie sterowanie rotatorem antenowym (SP / LP)
    if app.entry.active_bearing_deg >= 0.0 && app.entry.active_distance_km > 0.0 {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{}:", tr(key::GEO_AZIMUTH, lang)))
                    .small()
                    .color(palette.text_muted),
            );
            if side_button(
                ui,
                egui::RichText::new(format!("🧭 SP ({:.0}°)", app.entry.active_bearing_deg))
                    .small()
                    .strong(),
            )
            .clicked()
            {
                app.turn_rotor_short_path();
            }
            if side_button(
                ui,
                egui::RichText::new(format!(
                    "🔄 LP ({:.0}°)",
                    (app.entry.active_bearing_deg + 180.0) % 360.0
                ))
                .small(),
            )
            .clicked()
            {
                app.turn_rotor_long_path();
            }
        });
    }

    // 8. Sekcja Poprzednich Łączności (Worked Before / B4) z wpisywaną stacją
    if !app.entry.past_qsos_for_active_call.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "📜 {} ({})",
                tr(key::QSO_WORKED_BEFORE, lang),
                app.entry.past_qsos_for_active_call.len()
            ))
            .strong()
            .size(12.0)
            .color(palette.warning),
        );

        egui::ScrollArea::vertical()
            .max_height(80.0)
            .show(ui, |ui| {
                egui::Grid::new("b4_grid")
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for prev in &app.entry.past_qsos_for_active_call {
                            let is_dupe =
                                prev.band == app.entry.band && prev.mode == app.entry.mode;
                            let color = if is_dupe {
                                palette.dupe
                            } else {
                                palette.text_muted
                            };

                            ui.label(egui::RichText::new(&prev.qso_date).color(color).size(11.0));
                            ui.label(
                                egui::RichText::new(&prev.band)
                                    .strong()
                                    .color(color)
                                    .size(11.0),
                            );
                            ui.label(egui::RichText::new(&prev.mode).color(color).size(11.0));
                            ui.label(
                                egui::RichText::new(format!("{}/{}", prev.rst_sent, prev.rst_rcvd))
                                    .size(11.0),
                            );
                            if is_dupe {
                                ui.colored_label(palette.dupe, "DUPE!");
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}

/// Przycisk poboczny formularza: klikalny myszą, ale poza ścieżką `Tab`.
/// W egui 0.36.2 `Sense::click()` = `CLICK | FOCUSABLE`; samo `Sense::CLICK` nie
/// rejestruje widgetu jako fokusowalnego, więc `Tab` omija przycisk.
fn side_button(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    ui.add(egui::Button::new(text).sense(egui::Sense::CLICK))
}

/// Mały przycisk poboczny (jak [`side_button`], ale w rozmiarze `small`).
fn side_small_button(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    ui.add(egui::Button::new(text).small().sense(egui::Sense::CLICK))
}

/// `Enter` w jednowierszowym polu formularza zapisuje QSO; `Esc` czyści formularz.
///
/// Jednowierszowe `TextEdit` oddaje fokus przy `Enter` oraz przy `Esc` (egui czyści
/// fokus na początku klatki), dlatego sprawdzamy `lost_focus()`, nie `has_focus()`.
fn handle_form_keys(
    app: &mut SpLogApp,
    ui: &egui::Ui,
    fields: &[egui::Response],
    popup_open_at_start: bool,
) {
    let (enter, esc) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        )
    });

    if enter && let Some(field) = fields.iter().find(|r| r.lost_focus()) {
        app.save_qso();
        // Udany zapis sam prosi o fokus na znaku; przy błędzie walidacji formularz
        // zostaje nienaruszony, a fokus wraca do pola, w którym wciśnięto Enter.
        if !app.entry.focus_callsign_requested {
            field.request_focus();
        }
        return;
    }

    if esc
        && !popup_open_at_start
        && fields.iter().any(|r| r.has_focus() || r.lost_focus())
        && app.main_qso_escape_allowed(ui.ctx())
    {
        app.entry.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_buttons_are_clickable_but_outside_tab_order() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let normal = ui.button("normal");
            assert!(
                normal.sense.is_focusable(),
                "zwykły przycisk jest na ścieżce Tab"
            );

            for response in [side_button(ui, "side"), side_small_button(ui, "small")] {
                assert!(
                    response.sense.senses_click(),
                    "przycisk nadal reaguje na klik"
                );
                assert!(
                    !response.sense.is_focusable(),
                    "przycisk poboczny nie może być fokusowalny (Tab)"
                );
            }
        });
        output.textures_delta.clear();
    }
}
