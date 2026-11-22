// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::contest_rules::{RULES, detect_duplicate};
use crate::core::contest_stats::{
    MultKind, MultMatrix, RateStats, compute_mult_matrix, compute_rate,
};
use crate::core::exchange::{
    ExchangeField, apply_to_qso, exchange_summary, fields_from_format_string, parse_exchange,
};
use crate::core::i18n::{Msg, key, tr};
use crate::core::ingest::{IngestError, QsoOrigin};
use crate::core::qso::QsoRecord;
use crate::core::station::CustomContest;
use crate::gui::app::SpLogApp;
use crate::gui::theme::{ThemeExt, primary_button};

use eframe::egui;

pub fn render_custom_contest_editor(app: &mut SpLogApp, ctx: &egui::Context) {
    let mut open = app.contest.show_custom_editor;
    if !open {
        return;
    }
    let lang = app.gui.appearance.language;

    egui::Window::new(tr(key::CONTEST_CUSTOM_EDITOR_TITLE, lang))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(tr(key::CONTEST_YOUR_CONTESTS, lang));
                    let mut to_delete = None;
                    for (i, contest) in app.config.custom_contests.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&contest.name);
                            if ui.button(tr(key::BTN_EDIT, lang)).clicked() {
                                app.contest.custom_edit_idx = Some(i);
                                app.contest.custom_draft = contest.clone();
                            }
                            if ui.button(tr(key::BTN_DELETE, lang)).clicked() {
                                to_delete = Some(i);
                            }
                        });
                    }
                    if let Some(i) = to_delete {
                        app.config.custom_contests.remove(i);
                        if app.contest.custom_edit_idx == Some(i) {
                            app.contest.custom_edit_idx = None;
                            app.contest.custom_draft = CustomContest::default();
                        }
                        app.save_station_config();
                    }
                    if ui
                        .button(tr(key::CONTEST_NEW_CUSTOM_CONTEST, lang))
                        .clicked()
                    {
                        app.contest.custom_edit_idx = None;
                        app.contest.custom_draft = CustomContest::default();
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.heading(if app.contest.custom_edit_idx.is_some() {
                        tr(key::CONTEST_EDITING, lang)
                    } else {
                        tr(key::CONTEST_NEW, lang)
                    });

                    ui.horizontal(|ui| {
                        ui.label(tr(key::CONTEST_NAME_LABEL, lang));
                        ui.text_edit_singleline(&mut app.contest.custom_draft.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr(key::CONTEST_EXCHANGE_LABEL, lang));
                        ui.text_edit_singleline(&mut app.contest.custom_draft.exchange_format);
                    });

                    ui.label(tr(key::CONTEST_POINTS_PER_QSO_LABEL, lang));
                    ui.add(egui::Slider::new(
                        &mut app.contest.custom_draft.points_per_qso,
                        1..=20,
                    ));

                    ui.label(tr(key::CONTEST_BANDS_LABEL, lang));
                    let all_bands = [
                        "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m",
                        "70cm",
                    ];
                    ui.horizontal_wrapped(|ui| {
                        for band in all_bands {
                            let mut has_band =
                                app.contest.custom_draft.bands.contains(&band.to_string());
                            if ui.checkbox(&mut has_band, band).changed() {
                                if has_band {
                                    app.contest.custom_draft.bands.push(band.to_string());
                                } else {
                                    app.contest.custom_draft.bands.retain(|x| x != band);
                                }
                            }
                        }
                    });

                    ui.label(tr(key::CONTEST_DESCRIPTION_LABEL, lang));
                    ui.text_edit_multiline(&mut app.contest.custom_draft.description);

                    if ui.button(tr(key::BTN_SAVE, lang)).clicked()
                        && !app.contest.custom_draft.name.is_empty()
                    {
                        if let Some(i) = app.contest.custom_edit_idx {
                            app.config.custom_contests[i] = app.contest.custom_draft.clone();
                        } else {
                            app.config
                                .custom_contests
                                .push(app.contest.custom_draft.clone());
                        }
                        app.contest.custom_edit_idx = None;
                        app.contest.custom_draft = CustomContest::default();
                        app.save_station_config();
                    }
                });
            });
        });

    app.contest.show_custom_editor = open;
}

pub fn render_contest_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.contest.show_window {
        return;
    }
    let lang = app.gui.appearance.language;

    let mut open = app.contest.show_window;
    let mut close_req = false;
    let mut export_clicked = false;
    let mut reset_clicked = false;
    let mut save_clicked = false;
    let mut session_action = SessionAction::None;
    let mut macro_clicked: Option<usize> = None;
    let mut spot_fill_target: Option<crate::cluster::telnet::DxSpot> = None;

    // Is it a custom contest?
    let mut is_custom = false;
    let mut custom_idx = None;
    if let Some(idx) = app
        .config
        .custom_contests
        .iter()
        .position(|c| c.name == app.contest.name)
    {
        is_custom = true;
        custom_idx = Some(idx);
    }

    // Uporządkowane pola wymiany dla aktualnie wybranego kontestu.
    let exchange_fields: Option<Vec<ExchangeField>> = if let Some(idx) = custom_idx {
        Some(fields_from_format_string(
            &app.config.custom_contests[idx].exchange_format,
        ))
    } else {
        RULES
            .iter()
            .position(|r| r.name == app.contest.name)
            .map(|rule_idx| RULES[rule_idx].exchange_fields.to_vec())
    };
    let exchange_hint = exchange_fields
        .as_ref()
        .map(|f| {
            f.iter()
                .map(|e| e.label().to_string())
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .unwrap_or_default();

    let my_dxcc: u32 = app
        .services
        .prefix_matcher
        .lookup(&app.config.station.callsign)
        .and_then(|m| if m.dxcc > 0 { Some(m.dxcc) } else { None })
        .unwrap_or(269);
    let my_cqzone = app.config.station.cq_zone as u8;

    // Rodzaj mnożnika i pasma dla macierzy mnożników.
    let (mult_kind, rule_bands): (MultKind, Vec<String>) = if let Some(idx) = custom_idx {
        (
            MultKind::Dxcc,
            app.config.custom_contests[idx].bands.clone(),
        )
    } else if let Some(rule_idx) = RULES.iter().position(|r| r.name == app.contest.name) {
        (
            RULES[rule_idx].mult_kind,
            RULES[rule_idx]
                .bands
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
        )
    } else {
        (MultKind::None, vec![])
    };

    let current_key = crate::gui::state::contest::ContestCacheKey {
        profile_id: app.config.active_profile_id.clone(),
        log_identity: app.active_journal.id.clone(),
        log_revision: app.log_view.log_revision,
        journal_qso_count: app.log_view.qso_numbers.len(),
        contest_name: app.contest.name.clone(),
        custom_contests_len: app.config.custom_contests.len(),
        my_dxcc,
        my_cqzone,
        session: app
            .contest
            .session
            .as_ref()
            .map(|s| (s.contest_id.clone(), s.started_at, s.ended_at)),
    };

    let cache_hit = app.contest.cache.key.as_ref() == Some(&current_key);
    if !cache_hit {
        // The log buffer already holds the whole journal; only an advanced-filter view needs
        // the untruncated journal from the database.
        let full_journal_qsos = if app.log_view.buffer_is_filtered {
            app.services
                .log_db
                .lock()
                .ok()
                .and_then(|db| db.get_all_qsos_for_journal(&app.active_journal.id).ok())
        } else {
            None
        };
        let journal_qsos: &[QsoRecord] = full_journal_qsos
            .as_deref()
            .unwrap_or(&app.log_view.recent_qsos);

        // Wynik obejmuje wyłącznie QSO sesji zawodów, nie cały dziennik.
        let mut session_qsos = app
            .contest
            .session
            .as_ref()
            .map(|session| session.qsos(journal_qsos))
            .unwrap_or_default();
        // Tempo i kontrola duplikatów zakładają kolejność od najnowszego QSO.
        session_qsos
            .sort_by_key(|q| std::cmp::Reverse(crate::core::database::schema::chrono_key(q)));

        let scoreboard = app.contest_score(&session_qsos, my_dxcc, my_cqzone);

        let mult_matrix = if mult_kind != MultKind::None && !rule_bands.is_empty() {
            let band_refs: Vec<&str> = rule_bands.iter().map(std::string::String::as_str).collect();
            compute_mult_matrix(mult_kind, &band_refs, &session_qsos)
        } else {
            MultMatrix::default()
        };

        let mut bands = std::collections::HashMap::new();
        for qso in &session_qsos {
            *bands.entry(qso.band.clone()).or_insert(0) += 1;
        }
        let mut band_vec: Vec<_> = bands.into_iter().collect();
        band_vec.sort_by_key(|a| std::cmp::Reverse(a.1));

        app.contest.cache.scoreboard = scoreboard;
        app.contest.cache.mult_matrix = mult_matrix;
        app.contest.cache.band_breakdown = band_vec;
        app.contest.cache.session_qsos = session_qsos;
        app.contest.cache.key = Some(current_key);
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if !cache_hit || app.contest.cache.rate_cached_at_sec != now_secs {
        let rows = &app.contest.cache.session_qsos;
        let window = crate::core::contest_stats::rate_window_len(rows, now_secs);
        app.contest.cache.rate = compute_rate(rows.get(..window).unwrap_or(rows), now_secs);
        app.contest.cache.rate_cached_at_sec = now_secs;
    }

    let scoreboard = app.contest.cache.scoreboard;
    let rate = app.contest.cache.rate.clone();
    let pts = scoreboard.points;
    let mults = scoreboard.mults;
    let total = scoreboard.total;
    let unknown_contest_rule = !is_custom && !RULES.iter().any(|r| r.name == app.contest.name);

    // Prognoza wyniku: obecny wynik + tempo z ostatniej godziny × średnia punktów × mnożniki.
    let avg_points_per_qso = if scoreboard.qsos > 0 {
        pts as f64 / scoreboard.qsos as f64
    } else {
        0.0
    };
    let projected_1h =
        total as f64 + rate.rate_60m() as f64 * avg_points_per_qso * mults.max(1) as f64;

    egui::Window::new(tr(key::CONTEST_WINDOW_TITLE, lang))
        .id(egui::Id::new("splogbook_contest_window"))
        .open(&mut open)
        .default_size([800.0, 600.0])
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(key::CONTEST_SELECT, lang));
                    egui::ComboBox::from_id_salt("contest_type")
                        .selected_text(&app.contest.name)
                        .show_ui(ui, |ui| {
                            for rule in RULES {
                                ui.selectable_value(
                                    &mut app.contest.name,
                                    rule.name.to_string(),
                                    rule.name,
                                );
                            }
                            if !app.config.custom_contests.is_empty() {
                                ui.separator();
                                for contest in &app.config.custom_contests {
                                    ui.selectable_value(
                                        &mut app.contest.name,
                                        contest.name.clone(),
                                        contest.name.clone(),
                                    );
                                }
                            }
                        });

                    if ui.button(tr(key::CONTEST_ADD_CUSTOM, lang)).clicked() {
                        app.contest.show_custom_editor = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        crate::gui::user_manual::help_button(app, ui, "contest");
                    });
                });

                if unknown_contest_rule {
                    ui.colored_label(palette.danger, tr(key::CONTEST_UNKNOWN_RULES_WARNING, lang));
                }

                session_action = render_session_bar(app, ui, lang);

                ui.separator();

                ui.columns(3, |cols| {
                    cols[0].group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::CONTEST_CURRENT_RESULTS, lang))
                                .strong()
                                .color(palette.warning),
                        );
                        ui.label(format!(
                            "{} {}",
                            tr(key::CONTEST_QSO_COUNT, lang),
                            scoreboard.qsos
                        ));
                        ui.label(format!(
                            "{} {}",
                            tr(key::CONTEST_QSO_POINTS, lang),
                            scoreboard.points
                        ));
                        ui.label(format!(
                            "{} {}",
                            tr(key::CONTEST_MULTIPLIERS, lang),
                            scoreboard.mults
                        ));
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                tr(key::CONTEST_FINAL_SCORE, lang),
                                scoreboard.total
                            ))
                            .size(16.0)
                            .strong()
                            .color(palette.success),
                        );
                    });

                    cols[1].group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::CONTEST_BAND_STATS, lang))
                                .strong()
                                .color(palette.accent),
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("band_stats")
                            .max_height(80.0)
                            .show(ui, |ui| {
                                for (band, count) in &app.contest.cache.band_breakdown {
                                    ui.label(format!(
                                        "{} {}: {} QSOs",
                                        tr(key::CONTEST_BAND_HEADER, lang),
                                        band,
                                        count
                                    ));
                                }
                            });
                    });

                    cols[2].group(|ui| {
                        ui.label(
                            egui::RichText::new(tr(key::CONTEST_RATE, lang))
                                .strong()
                                .color(palette.accent),
                        );
                        ui.label(format!("1m:  {} QSO/h", rate.rate_1m()));
                        ui.label(format!("5m:  {} QSO/h", rate.rate_5m()));
                        ui.label(format!("10m: {} QSO/h", rate.rate_10m()));
                        ui.label(format!("60m: {} QSO/h", rate.rate_60m()));
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {:.0}",
                                tr(key::CONTEST_PROJECTED_1H, lang),
                                projected_1h
                            ))
                            .color(palette.success),
                        );
                        ui.label(format!(
                            "{} {:03}",
                            tr(key::CONTEST_STX_NR, lang),
                            app.contest.stx
                        ));
                    });
                });

                ui.add_space(8.0);
                ui.separator();

                // Wykres tempa (ostatnie 60 minut) + macierz mnożników.
                egui::CollapsingHeader::new(tr(key::CONTEST_RATE_CHART, lang))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_rate_chart(ui, &rate);
                    });

                if mult_kind != MultKind::None && !rule_bands.is_empty() {
                    egui::CollapsingHeader::new(format!(
                        "{} ({})",
                        tr(key::CONTEST_MULT_MATRIX, lang),
                        mult_kind.as_str()
                    ))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_mult_matrix(ui, &app.contest.cache.mult_matrix, lang);
                    });
                }

                // QSO ENTRY FORM
                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr(key::CONTEST_ENTRY_HEADER, lang)).strong());
                    // Siatka „etykieta | pole”: długie etykiety (np. DE „Empfangener Rapport /
                    // Austausch:”) nie wylewają się w jednym wierszu poziomym.
                    egui::Grid::new("contest_entry_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(format!("{}:", tr(key::QSO_CALLSIGN, lang)));

                            let dummy_qso = QsoRecord {
                                callsign: app.entry.callsign.clone(),
                                band: app.entry.band.clone(),
                                mode: app.entry.mode.clone(),
                                ..Default::default()
                            };

                            // Duplikat liczy się tylko w obrębie sesji zawodów: stacja zrobiona
                            // rok temu na tym paśmie nie jest duplikatem w nowych zawodach.
                            let session_qsos = &app.contest.cache.session_qsos;
                            let is_dupe = if is_custom {
                                session_qsos.iter().any(|q| {
                                    q.callsign.eq_ignore_ascii_case(&dummy_qso.callsign)
                                        && q.band.eq_ignore_ascii_case(&dummy_qso.band)
                                })
                            } else if let Some(rule_idx) =
                                RULES.iter().position(|r| r.name == app.contest.name)
                            {
                                detect_duplicate(&RULES[rule_idx], &dummy_qso, session_qsos)
                            } else {
                                // Nieznane reguły kontestu: prosta kontrola duplikatu wg znaku+pasma
                                // zamiast cichego zastosowania reguł innego (pierwszego) kontestu.
                                session_qsos.iter().any(|q| {
                                    q.callsign.eq_ignore_ascii_case(&dummy_qso.callsign)
                                        && q.band.eq_ignore_ascii_case(&dummy_qso.band)
                                })
                            };

                            if is_dupe && !app.entry.callsign.is_empty() {
                                ui.visuals_mut().widgets.inactive.bg_stroke =
                                    egui::Stroke::new(2.0_f32, palette.dupe);
                            }

                            let call_resp = ui
                                .horizontal(|ui| {
                                    let call_resp = ui.add(
                                        egui::TextEdit::singleline(&mut app.entry.callsign)
                                            .id(egui::Id::new("contest_entry_callsign"))
                                            .desired_width(140.0),
                                    );
                                    if app.entry.focus_callsign_requested {
                                        call_resp.request_focus();
                                        app.entry.focus_callsign_requested = false;
                                    }
                                    if is_dupe && !app.entry.callsign.is_empty() {
                                        ui.label(
                                            egui::RichText::new("DUPE!")
                                                .color(palette.dupe)
                                                .strong(),
                                        );
                                    }
                                    call_resp
                                })
                                .inner;
                            ui.end_row();

                            ui.label(tr(key::CONTEST_RCVD_REPORT_EXCHANGE, lang));
                            let exch_resp = ui.add(
                                egui::TextEdit::singleline(&mut app.entry.exchange)
                                    .id(egui::Id::new("contest_entry_exchange"))
                                    .desired_width(ui.available_width().clamp(120.0, 240.0)),
                            );
                            if std::mem::take(&mut app.contest.focus_exchange_requested) {
                                exch_resp.request_focus();
                            }
                            ui.end_row();

                            // `Enter` zwykle trafia tu przez `AppAction::ContestSaveQso`; jednowierszowe
                            // `TextEdit` oddaje fokus przy Enter, więc zapasowo sprawdzamy `lost_focus()`.
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let enter = std::mem::take(&mut app.contest.save_requested)
                                || ((call_resp.lost_focus() || exch_resp.lost_focus())
                                    && enter_pressed);
                            if enter && app.contest_enter() {
                                save_clicked = true;
                            }

                            ui.label("");
                            if ui
                                .add(primary_button(&palette, tr(key::CONTEST_SAVE_QSO, lang)))
                                .clicked()
                            {
                                save_clicked = true;
                            }
                            ui.end_row();
                        });

                    if !exchange_hint.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                Msg::new(key::CONTEST_EXCHANGE_FORMAT)
                                    .arg("format", &exchange_hint)
                                    .text(lang),
                            )
                            .italics()
                            .weak(),
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        for (index, text) in app.cw.macros.iter().enumerate() {
                            if ui
                                .small_button(format!("F{}", index + 1))
                                .on_hover_text(text)
                                .clicked()
                            {
                                macro_clicked = Some(index);
                            }
                        }
                    });
                    // Podgląd / walidacja parsowania na żywo.
                    if !app.entry.exchange.trim().is_empty()
                        && let Some(fields) = &exchange_fields
                    {
                        match parse_exchange(fields, &app.entry.exchange) {
                            Ok(parsed) => {
                                ui.label(
                                    egui::RichText::new(format!("✓ {}", exchange_summary(&parsed)))
                                        .color(palette.success),
                                );
                            }
                            Err(e) => {
                                ui.label(
                                    egui::RichText::new(format!("⚠ {}", e.message))
                                        .color(palette.danger),
                                );
                            }
                        }
                    }
                });

                // DX Cluster / Bandmap wbudowany w okno kontestu: spot -> wpis jednym klikiem.
                egui::CollapsingHeader::new(tr(key::CONTEST_SPOT_LIST, lang))
                    .default_open(false)
                    .show(ui, |ui| {
                        let mut tune: Option<crate::cluster::telnet::DxSpot> = None;
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .show(ui, |ui| {
                                egui::Grid::new("contest_spot_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .spacing([10.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(tr(key::CLUSTER_DX, lang)).strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(tr(key::CLUSTER_FREQ, lang))
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(tr(key::QSO_BAND, lang)).strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(tr(key::CLUSTER_COMMENT, lang))
                                                .strong(),
                                        );
                                        ui.label("");
                                        ui.end_row();

                                        let worked_calls =
                                            app.log_view.get_or_compute_worked_callsigns();
                                        for spot in &app.cluster.spots {
                                            let worked = worked_calls
                                                .contains(&spot.dx_call.to_ascii_uppercase());
                                            let call_color = if worked {
                                                palette.qsl_rcvd
                                            } else {
                                                palette.accent
                                            };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}{}",
                                                    spot.dx_call,
                                                    if worked { " ✓" } else { "" }
                                                ))
                                                .strong()
                                                .color(call_color),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1} kHz",
                                                    spot.frequency_khz
                                                ))
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&spot.band)
                                                    .color(palette.warning),
                                            );
                                            ui.label(
                                                egui::RichText::new(&spot.comment)
                                                    .size(11.0)
                                                    .weak(),
                                            );
                                            if ui
                                                .button(
                                                    egui::RichText::new(tr(
                                                        key::CONTEST_SPOT_FILL_ENTRY,
                                                        lang,
                                                    ))
                                                    .color(palette.success),
                                                )
                                                .clicked()
                                            {
                                                tune = Some(spot.clone());
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });

                        if tune.is_some() {
                            spot_fill_target = tune;
                        }
                    });

                ui.add_space(8.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button(tr(key::CONTEST_EXPORT_CABRILLO, lang)).clicked() {
                        export_clicked = true;
                    }
                    if ui.button(tr(key::CONTEST_RESET_NUMBERING, lang)).clicked() {
                        reset_clicked = true;
                    }
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });

    if let Some(spot) = spot_fill_target {
        app.tune_to_cluster_spot(&spot);
        let call = spot.dx_call;
        // Uzupełnij wymianę z wcześniejszego QSO z tą stacją (jeśli istnieje).
        if let Some(prev) = app
            .log_view
            .recent_qsos
            .iter()
            .rev()
            .find(|q| q.callsign.eq_ignore_ascii_case(&call))
        {
            if let Some(srx) = &prev.srx_string {
                app.entry.exchange = srx.clone();
            } else if !prev.rst_rcvd.is_empty() {
                app.entry.exchange = prev.rst_rcvd.clone();
            }
            app.gui.chrome.push_status_toast(
                crate::gui::state::ToastSeverity::Warning,
                format!(
                    "{}: {}",
                    tr(key::CONTEST_WORKED_BEFORE, app.gui.appearance.language),
                    call
                ),
            );
        }
    }

    if save_clicked {
        if app.entry.callsign.trim().is_empty() {
            app.gui.chrome.push_status_toast(
                crate::gui::state::ToastSeverity::Error,
                Msg::new(key::CONTEST_VALIDATION_ERROR)
                    .arg("err", Msg::new(key::QSO_ISSUE_EMPTY_CALLSIGN).text(lang))
                    .text(lang),
            );
        } else {
            if app.active_contest_session().is_none() {
                app.start_contest_session();
            }
            let mut new_qso = QsoRecord::new(&app.entry.callsign, &app.entry.band, &app.entry.mode);
            let uses_serial = app.tag_contest_qso(&mut new_qso);
            new_qso.journal_id = Some(app.active_journal.id.clone());
            if !app.config.station.gridsquare.trim().is_empty() {
                new_qso.my_gridsquare = Some(app.config.station.gridsquare.clone());
            }
            if app.cat.rig_state.frequency_hz > 0 {
                new_qso.freq = Some(app.cat.rig_state.frequency_hz as f64 / 1_000_000.0);
            }
            // Parsuj wymianę, jeśli kontest ma zdefiniowane pola; w przeciwnym razie
            // zachowaj dawny tryb "sam RST" w polu `entry_rst_rcvd`.
            let mut exchange_ok = true;
            if let Some(fields) = &exchange_fields {
                let exchange_text = if app.entry.exchange.trim().is_empty() {
                    app.entry.rst_rcvd.clone()
                } else {
                    app.entry.exchange.clone()
                };
                match parse_exchange(fields, &exchange_text) {
                    Ok(parsed) => apply_to_qso(&parsed, &mut new_qso),
                    Err(e) => {
                        exchange_ok = false;
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Error,
                            Msg::new(key::CONTEST_EXCHANGE_ERROR)
                                .arg("err", &e.message)
                                .text(lang),
                        );
                    }
                }
            } else {
                new_qso.rst_rcvd.clone_from(&app.entry.rst_rcvd);
            }

            if exchange_ok {
                match app.services.ingestor.ingest(new_qso, QsoOrigin::Contest) {
                    Ok(outcome) => {
                        if !outcome.warnings.is_empty() {
                            let warning_texts: Vec<String> = outcome
                                .warnings
                                .iter()
                                .map(|w| w.msg().text(app.gui.appearance.language))
                                .collect();
                            app.report_warning(
                                Msg::new(key::CONTEST_WARNINGS)
                                    .arg("warnings", warning_texts.join("; "))
                                    .text(lang),
                            );
                        }
                        app.invalidate_cluster_badges();
                        app.apply_post_effects(&outcome);
                        if uses_serial {
                            app.contest.stx += 1;
                        }
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Success,
                            Msg::new(key::CONTEST_QSO_SAVED)
                                .arg("call", &outcome.qso.callsign)
                                .arg("rst", &outcome.qso.rst_rcvd)
                                .arg("exch", outcome.qso.srx_string.as_deref().unwrap_or(""))
                                .text(lang),
                        );
                        app.entry.callsign.clear();
                        app.entry.rst_rcvd.clear();
                        app.entry.exchange.clear();
                        app.entry.focus_callsign_requested = true;
                        app.apply_stored_qso(&outcome.qso);
                    }
                    Err(IngestError::Invalid(reason)) => {
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Error,
                            Msg::new(key::CONTEST_VALIDATION_ERROR)
                                .arg("err", reason.msg().text(lang))
                                .text(lang),
                        );
                    }
                    Err(IngestError::Duplicate) => {
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Warning,
                            Msg::new(key::MSG_QSO_DUPLICATE).text(lang),
                        );
                    }
                    Err(IngestError::EmptyCallsign) => {
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Error,
                            Msg::new(key::CONTEST_VALIDATION_ERROR)
                                .arg("err", Msg::new(key::QSO_ISSUE_EMPTY_CALLSIGN).text(lang))
                                .text(lang),
                        );
                    }
                    Err(IngestError::Storage(error)) => {
                        app.gui.chrome.push_status_toast(
                            crate::gui::state::ToastSeverity::Error,
                            Msg::new(key::CONTEST_SAVE_ERROR)
                                .arg("err", error)
                                .text(lang),
                        );
                    }
                }
            }
        }
    }

    if close_req {
        open = false;
    }
    app.contest.show_window = open;

    if export_clicked {
        app.export_cabrillo();
    }
    if reset_clicked {
        app.contest.stx = 1;
    }
    if let Some(index) = macro_clicked {
        app.contest_macro(index);
    }
    match session_action {
        SessionAction::None => {}
        SessionAction::Start => app.start_contest_session(),
        SessionAction::End => app.end_contest_session(),
        SessionAction::Resume => app.resume_contest_session(),
        SessionAction::Changed => app.save_station_config(),
    }
}

/// Działanie z paska sesji zawodów (wykonywane po narysowaniu okna).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionAction {
    None,
    Start,
    End,
    Resume,
    Changed,
}

/// Pasek sesji: start/koniec, tryb Run/S&P, ESM, wymiana nadawana i kategorie Cabrillo.
fn render_session_bar(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    lang: crate::core::i18n::Language,
) -> SessionAction {
    use crate::core::contest_session::{OperatingMode, allowed};
    let palette = ui.palette();
    let mut action = SessionAction::None;
    let selected = app.contest.name.clone();
    let Some(session) = app
        .contest
        .session
        .as_mut()
        .filter(|s| s.contest_name == selected)
    else {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(tr(key::CONTEST_SESSION_NONE, lang))
                    .italics()
                    .color(palette.text_muted),
            );
            if ui.button(tr(key::CONTEST_SESSION_START, lang)).clicked() {
                action = SessionAction::Start;
            }
        });
        return action;
    };

    let started = jiff::Timestamp::from_second(session.started_at)
        .map(|t| t.strftime("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        if session.is_active() {
            ui.label(
                egui::RichText::new(
                    Msg::new(key::CONTEST_SESSION_ACTIVE)
                        .arg("id", &session.contest_id)
                        .arg("since", &started)
                        .text(lang),
                )
                .color(palette.success)
                .strong(),
            );
            if ui.button(tr(key::CONTEST_SESSION_END, lang)).clicked() {
                action = SessionAction::End;
            }
        } else {
            ui.label(
                egui::RichText::new(
                    Msg::new(key::CONTEST_SESSION_ENDED)
                        .arg("id", &session.contest_id)
                        .arg("since", &started)
                        .text(lang),
                )
                .color(palette.warning),
            );
            if ui.button(tr(key::CONTEST_SESSION_RESUME, lang)).clicked() {
                action = SessionAction::Resume;
            }
            if ui.button(tr(key::CONTEST_SESSION_NEW, lang)).clicked() {
                action = SessionAction::Start;
            }
        }
    });

    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        changed |= ui
            .selectable_value(&mut session.mode, OperatingMode::Run, "RUN")
            .on_hover_text(tr(key::CONTEST_MODE_RUN_HINT, lang))
            .changed();
        changed |= ui
            .selectable_value(&mut session.mode, OperatingMode::SearchAndPounce, "S&P")
            .on_hover_text(tr(key::CONTEST_MODE_SP_HINT, lang))
            .changed();
        changed |= ui
            .checkbox(&mut session.esm, tr(key::CONTEST_ESM, lang))
            .on_hover_text(tr(key::CONTEST_ESM_HINT, lang))
            .changed();
        ui.label(tr(key::CONTEST_SENT_EXCHANGE, lang));
        changed |= ui
            .add(egui::TextEdit::singleline(&mut session.sent_exchange).desired_width(70.0))
            .on_hover_text(tr(key::CONTEST_SENT_EXCHANGE_HINT, lang))
            .lost_focus();
    });

    egui::CollapsingHeader::new(tr(key::CONTEST_CABRILLO_CATEGORIES, lang))
        .id_salt("contest_cabrillo_categories")
        .show(ui, |ui| {
            egui::Grid::new("contest_categories_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    let c = &mut session.categories;
                    let rows: [(&str, &mut String, &[&str]); 7] = [
                        ("CATEGORY-OPERATOR", &mut c.operator, allowed::OPERATOR),
                        ("CATEGORY-POWER", &mut c.power, allowed::POWER),
                        ("CATEGORY-BAND", &mut c.band, allowed::BAND),
                        ("CATEGORY-MODE", &mut c.mode, allowed::MODE),
                        ("CATEGORY-STATION", &mut c.station, allowed::STATION),
                        ("CATEGORY-ASSISTED", &mut c.assisted, allowed::ASSISTED),
                        ("CATEGORY-OVERLAY", &mut c.overlay, allowed::OVERLAY),
                    ];
                    for (tag, value, options) in rows {
                        ui.label(tag);
                        egui::ComboBox::from_id_salt(tag)
                            .selected_text(value.as_str())
                            .show_ui(ui, |ui| {
                                for option in options {
                                    changed |= ui
                                        .selectable_value(value, (*option).to_string(), *option)
                                        .changed();
                                }
                            });
                        ui.end_row();
                    }
                    for (tag, value) in [
                        ("CONTEST", &mut session.contest_id),
                        ("LOCATION", &mut session.categories.location),
                        ("CLUB", &mut session.categories.club),
                        ("OPERATORS", &mut session.categories.operators),
                        ("SOAPBOX", &mut session.categories.soapbox),
                    ] {
                        ui.label(tag);
                        changed |= ui.text_edit_singleline(value).lost_focus();
                        ui.end_row();
                    }
                });
            ui.label(
                egui::RichText::new(tr(key::CONTEST_CATEGORIES_HINT, lang))
                    .small()
                    .color(palette.text_muted),
            );
        });

    if changed && action == SessionAction::None {
        action = SessionAction::Changed;
    }
    action
}

// Re-eksport okna Multi-Op przeniesionego do multi_op_window.rs (SYNC-6)
pub use super::multi_op_window::render_multi_op_window;

/// Rysuje mini-wykres słupkowy tempa QSO na minutę (ostatnie 60 minut).
fn render_rate_chart(ui: &mut egui::Ui, rate: &RateStats) {
    let palette = ui.palette();
    let chart_h = 60.0f32;
    let chart_w = ui.available_width();
    let (resp, painter) =
        ui.allocate_painter(egui::vec2(chart_w, chart_h + 14.0), egui::Sense::hover());
    let origin = resp.rect.min;
    let max_val = rate.per_minute.iter().copied().max().unwrap_or(1).max(1) as f32;
    let count = rate.per_minute.len();
    if count == 0 {
        return;
    }
    let bar_w = chart_w / count as f32;
    for (i, value) in rate.per_minute.iter().enumerate() {
        let bar_h = (*value as f32 / max_val) * chart_h;
        let x_pos = origin.x + i as f32 * bar_w;
        let rect = egui::Rect::from_min_size(
            egui::pos2(x_pos + 0.5, origin.y + chart_h - bar_h),
            egui::vec2((bar_w - 1.0).max(0.5), bar_h),
        );
        painter.rect_filled(rect, 0.0, palette.accent);
    }
    // Linia bazowa i etykiety osi.
    painter.line_segment(
        [
            egui::pos2(origin.x, origin.y + chart_h),
            egui::pos2(origin.x + chart_w, origin.y + chart_h),
        ],
        egui::Stroke::new(1.0_f32, palette.border),
    );
    painter.text(
        egui::pos2(origin.x, origin.y + chart_h + 4.0),
        egui::Align2::LEFT_TOP,
        "-60 min",
        egui::FontId::proportional(9.0),
        palette.text_muted,
    );
    painter.text(
        egui::pos2(origin.x + chart_w, origin.y + chart_h + 4.0),
        egui::Align2::RIGHT_TOP,
        "now",
        egui::FontId::proportional(9.0),
        palette.text_muted,
    );
}

/// Rysuje macierz mnożników: wiersze = pasma, kolumny = mnożniki.
/// Zielony = zaliczony na paśmie, żółty = zaliczony gdzie indziej, szary = potrzebny.
fn render_mult_matrix(ui: &mut egui::Ui, matrix: &MultMatrix, lang: crate::core::i18n::Language) {
    if matrix.bands.is_empty() {
        ui.label(tr(key::LOG_NO_RESULTS, lang));
        return;
    }

    let palette = ui.palette();

    // Legenda.
    ui.horizontal(|ui| {
        ui.colored_label(palette.success, "●");
        ui.label(tr(key::CONTEST_MULT_WORKED_HERE, lang));
        ui.colored_label(palette.multiplier, "◐");
        ui.label(tr(key::CONTEST_MULT_WORKED_OTHER, lang));
        ui.colored_label(palette.text_subtle, "·");
        ui.label(tr(key::CONTEST_MULT_NEEDED, lang));
        ui.label(
            egui::RichText::new(format!(
                "({}: {})",
                tr(key::CONTEST_MULT_NEEDED_COUNT, lang),
                matrix.needed_count()
            ))
            .weak(),
        );
    });

    egui::ScrollArea::horizontal().show(ui, |ui| {
        egui::Grid::new("contest_mult_matrix")
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                // Nagłówek z etykietami mnożników.
                ui.label("");
                for col in &matrix.columns {
                    ui.label(egui::RichText::new(col).size(8.0).color(palette.text_muted));
                }
                ui.end_row();

                for (r, band) in matrix.bands.iter().enumerate() {
                    ui.label(egui::RichText::new(band).strong().size(10.0));
                    for (c, col) in matrix.columns.iter().enumerate() {
                        let value = matrix.worked[r][c];
                        let (color, glyph) = match value {
                            2 => (palette.success, "●"),
                            1 => (palette.multiplier, "◐"),
                            _ => (palette.text_subtle, "·"),
                        };
                        ui.colored_label(color, glyph)
                            .on_hover_text(format!("{band} × {col}"));
                    }
                    ui.end_row();
                }
            });
    });
}
