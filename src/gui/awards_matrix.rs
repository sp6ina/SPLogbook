// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::awards::US_STATES;
use crate::core::i18n::{Language, Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

/// Okno matrycy dyplomowej (DXCC, WAZ, WAS, WAC, WPX, VUCC, IOTA, SOTA, POTA, PGA)
pub fn render_awards_matrix_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_awards_matrix_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_awards_matrix_window;
    let mut close_req = false;

    egui::Window::new(
        egui::RichText::new(format!("🏆 {}", tr(key::AWARDS_MATRIX_TITLE, lang)))
            .size(12.0)
            .strong(),
    )
    .open(&mut is_open)
    .default_size([720.0, 560.0])
    .min_size([500.0, 360.0])
    .resizable(true)
    .show(ctx, |ui| {
        let palette = ui.palette();
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(tr(key::AWARDS_ACHIEVEMENT_CENTER, lang))
                        .size(15.0)
                        .color(palette.accent),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::gui::user_manual::help_button(app, ui, "awards");
                });
            });

            ui.add_space(2.0);

            // Zakładki dyplomów
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut app.awards_matrix_tab,
                    0,
                    tr(key::AWARDS_TAB_DXCC, lang),
                );
                ui.selectable_value(&mut app.awards_matrix_tab, 1, tr(key::AWARDS_TAB_WAZ, lang));
                ui.selectable_value(&mut app.awards_matrix_tab, 2, tr(key::AWARDS_TAB_WAS, lang));
                ui.selectable_value(&mut app.awards_matrix_tab, 3, tr(key::AWARDS_TAB_WAC, lang));
                ui.selectable_value(
                    &mut app.awards_matrix_tab,
                    4,
                    tr(key::AWARDS_TAB_OTHER, lang),
                );
                ui.selectable_value(
                    &mut app.awards_matrix_tab,
                    5,
                    tr(key::AWARDS_TAB_SP_DX, lang),
                );
                ui.selectable_value(&mut app.awards_matrix_tab, 6, tr(key::AWARDS_TAB_WAE, lang));
            });

            ui.separator();

            let awards = app
                .awards_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            match app.awards_matrix_tab {
                0 => render_tab_dxcc(app, ui, &awards, lang),
                1 => render_tab_waz(ui, &awards, lang),
                2 => render_tab_was(ui, &awards, lang),
                3 => render_tab_wac(ui, &awards, lang),
                5 => render_tab_sp_dx(ui, &awards, lang),
                6 => render_tab_wae(ui, &awards, lang),
                _ => render_tab_other(
                    &mut app.awards_other_subtab,
                    &mut app.awards_other_search,
                    ui,
                    &awards,
                    lang,
                ),
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                    close_req = true;
                }
            });
        });
    });

    if close_req {
        is_open = false;
    }
    app.show_awards_matrix_window = is_open;
}

fn render_legend(ui: &mut egui::Ui, lang: Language) {
    let palette = ui.palette();
    ui.horizontal(|ui| {
        ui.colored_label(palette.success, tr(key::AWARDS_LEGEND_CONFIRMED_QSL, lang));
        ui.colored_label(palette.warning, tr(key::AWARDS_LEGEND_WORKED, lang));
        ui.colored_label(palette.text_muted, tr(key::AWARDS_LEGEND_NEEDED, lang));
    });
}

fn render_tab_dxcc(
    app: &SpLogApp,
    ui: &mut egui::Ui,
    awards: &crate::core::awards::AwardsEngine,
    lang: Language,
) {
    let palette = ui.palette();
    let worked_dxcc = awards.worked_dxcc_all.len();
    let conf_dxcc = awards.confirmed_dxcc.len();
    let total_qsos = app.recent_qsos.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(tr(key::AWARDS_DXCC_ALL_ENTITIES, lang))
                    .strong()
                    .size(12.0)
                    .color(palette.warning),
            );
            ui.label(
                Msg::new(key::AWARDS_DXCC_SUMMARY)
                    .arg("worked", worked_dxcc)
                    .arg("confirmed", conf_dxcc)
                    .text(lang),
            );
        });

        ui.group(|ui| {
            ui.label(
                egui::RichText::new(tr(key::AWARDS_BAND_COUNTRIES, lang))
                    .strong()
                    .size(12.0)
                    .color(palette.accent),
            );
            ui.label(format!(
                "{}: {}",
                tr(key::AWARDS_SLOTS_LABEL, lang),
                Msg::new(key::AWARDS_BAND_SLOTS_COUNT)
                    .arg("count", awards.worked_dxcc_band.len())
                    .text(lang)
            ));
        });

        ui.group(|ui| {
            ui.label(
                egui::RichText::new(tr(key::AWARDS_TOTAL_IN_LOG, lang))
                    .strong()
                    .size(12.0)
                    .color(palette.accent),
            );
            ui.label(
                Msg::new(key::AWARDS_TOTAL_QSOS_COUNT)
                    .arg("count", total_qsos)
                    .text(lang),
            );
        });
    });

    ui.add_space(4.0);
    render_legend(ui, lang);
    ui.add_space(4.0);

    let bands = &[
        "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "4m", "2m",
        "70cm",
    ];
    let modes = &["CW", "SSB", "DIGI", "MIXED"];

    // Pojedynczy przebieg po QSO zamiast skanowania listy dla każdej komórki siatki.
    let mut worked = vec![[false; 4]; bands.len()];
    let mut confirmed = vec![[false; 4]; bands.len()];
    let mut band_counts = vec![0usize; bands.len()];

    for q in &app.recent_qsos {
        let Some(bi) = bands.iter().position(|b| *b == q.band) else {
            continue;
        };
        band_counts[bi] += 1;

        let mut mode_flags = [false; 4];
        mode_flags[3] = true; // MIXED — każde QSO pasuje do pasma
        if q.mode == "CW" {
            mode_flags[0] = true;
        }
        if q.mode == "SSB" {
            mode_flags[1] = true;
        }
        if q.mode == "DIGI" || q.mode == "FT8" || q.mode == "FT4" || q.mode == "RTTY" {
            mode_flags[2] = true;
        }

        let is_confirmed = q.lotw_qsl_rcvd == "Y" || q.qsl_rcvd == "Y" || q.eqsl_qsl_rcvd == "Y";

        for (mi, &flag) in mode_flags.iter().enumerate() {
            if flag {
                worked[bi][mi] = true;
                if is_confirmed {
                    confirmed[bi][mi] = true;
                }
            }
        }
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("awards_dxcc_grid")
            .striped(true)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(tr(key::AWARDS_COL_BAND, lang)).strong());
                for mode in modes {
                    ui.label(egui::RichText::new(*mode).strong());
                }
                ui.label(egui::RichText::new(tr(key::AWARDS_COL_QSOS, lang)).strong());
                ui.end_row();

                for (bi, band) in bands.iter().enumerate() {
                    ui.label(egui::RichText::new(*band).strong().color(palette.accent));

                    for mi in 0..modes.len() {
                        let worked = worked[bi][mi];
                        let confirmed = confirmed[bi][mi];

                        if confirmed {
                            ui.label(egui::RichText::new("✓ C").color(palette.success).strong());
                        } else if worked {
                            ui.label(egui::RichText::new("● W").color(palette.warning));
                        } else {
                            ui.label(egui::RichText::new("-").color(palette.text_muted));
                        }
                    }

                    let band_qsos = band_counts[bi];
                    ui.label(
                        egui::RichText::new(format!("{band_qsos} QSO"))
                            .size(11.0)
                            .color(palette.text_muted),
                    );
                    ui.end_row();
                }
            });
    });
}

fn render_tab_waz(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine, lang: Language) {
    let palette = ui.palette();
    let worked_count = awards.worked_waz.len();
    let conf_count = awards.confirmed_waz.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(tr(key::AWARDS_WAZ_TITLE, lang))
                    .strong()
                    .size(13.0)
                    .color(palette.accent),
            );
            ui.label(
                Msg::new(key::AWARDS_WAZ_SUMMARY)
                    .arg("worked", worked_count)
                    .arg("confirmed", conf_count)
                    .arg("needed", 40 - worked_count)
                    .text(lang),
            );
        });
    });

    ui.add_space(4.0);
    render_legend(ui, lang);
    ui.add_space(6.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("awards_waz_grid")
            .striped(true)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                for zone in 1..=40 {
                    let is_conf = awards.confirmed_waz.contains(&zone);
                    let is_wrk = awards.worked_waz.contains(&zone);

                    let (color, mark) = if is_conf {
                        (palette.success, "✓")
                    } else if is_wrk {
                        (palette.warning, "●")
                    } else {
                        (palette.text_muted, "-")
                    };

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Z{zone:02}"))
                                    .strong()
                                    .size(11.0),
                            );
                            ui.colored_label(color, mark);
                        });
                    });

                    if zone % 8 == 0 {
                        ui.end_row();
                    }
                }
            });
    });
}

fn render_tab_was(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine, lang: Language) {
    let palette = ui.palette();
    let worked_count = awards.worked_was.len();
    let conf_count = awards.confirmed_was.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(tr(key::AWARDS_WAS_TITLE, lang))
                    .strong()
                    .size(13.0)
                    .color(palette.danger),
            );
            ui.label(
                Msg::new(key::AWARDS_WAS_SUMMARY)
                    .arg("worked", worked_count)
                    .arg("confirmed", conf_count)
                    .arg("needed", 50 - worked_count)
                    .text(lang),
            );
        });
    });

    ui.add_space(4.0);
    render_legend(ui, lang);
    ui.add_space(6.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("awards_was_grid")
            .striped(true)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                for (idx, st) in US_STATES.iter().enumerate() {
                    let is_conf = awards.confirmed_was.contains(*st);
                    let is_wrk = awards.worked_was.contains(*st);

                    let (color, mark) = if is_conf {
                        (palette.success, "✓")
                    } else if is_wrk {
                        (palette.warning, "●")
                    } else {
                        (palette.text_muted, "-")
                    };

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(*st).strong().size(11.0));
                            ui.colored_label(color, mark);
                        });
                    });

                    if (idx + 1) % 10 == 0 {
                        ui.end_row();
                    }
                }
            });
    });
}

fn render_tab_wac(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine, lang: Language) {
    let palette = ui.palette();
    let worked_count = awards.worked_wac.len();
    let conf_count = awards.confirmed_wac.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(tr(key::AWARDS_WAC_TITLE, lang))
                    .strong()
                    .size(13.0)
                    .color(palette.warning),
            );
            ui.label(
                Msg::new(key::AWARDS_WAC_SUMMARY)
                    .arg("worked", worked_count)
                    .arg("confirmed", conf_count)
                    .text(lang),
            );
        });
    });

    ui.add_space(4.0);
    render_legend(ui, lang);
    ui.add_space(8.0);

    let names = [
        ("AF", tr(key::AWARDS_CONTINENT_AF, lang)),
        ("AN", tr(key::AWARDS_CONTINENT_AN, lang)),
        ("AS", tr(key::AWARDS_CONTINENT_AS, lang)),
        ("EU", tr(key::AWARDS_CONTINENT_EU, lang)),
        ("NA", tr(key::AWARDS_CONTINENT_NA, lang)),
        ("OC", tr(key::AWARDS_CONTINENT_OC, lang)),
        ("SA", tr(key::AWARDS_CONTINENT_SA, lang)),
    ];

    ui.vertical(|ui| {
        for (code, name) in names {
            let is_conf = awards.confirmed_wac.contains(code);
            let is_wrk = awards.worked_wac.contains(code);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(code)
                            .strong()
                            .size(13.0)
                            .color(palette.accent),
                    );
                    ui.label(name);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_conf {
                            ui.colored_label(
                                palette.success,
                                tr(key::AWARDS_BADGE_CONFIRMED, lang),
                            );
                        } else if is_wrk {
                            ui.colored_label(palette.warning, tr(key::AWARDS_BADGE_WORKED, lang));
                        } else {
                            ui.colored_label(
                                palette.text_muted,
                                tr(key::AWARDS_BADGE_NEEDED, lang),
                            );
                        }
                    });
                });
            });
        }
    });
}

fn render_tab_other(
    subtab: &mut usize,
    search_query: &mut String,
    ui: &mut egui::Ui,
    awards: &crate::core::awards::AwardsEngine,
    lang: Language,
) {
    let palette = ui.palette();
    ui.vertical(|ui| {
        // Podmenu programów dyplomowych
        ui.horizontal_wrapped(|ui| {
            let subtabs = [
                (0, format!("🏷 WPX ({})", awards.worked_wpx.len())),
                (1, format!("🏝 IOTA ({})", awards.worked_iota.len())),
                (2, format!("📡 VUCC ({})", awards.worked_vucc.len())),
                (3, format!("⛰ SOTA ({})", awards.worked_sota.len())),
                (4, format!("🌲 POTA ({})", awards.worked_pota.len())),
                (5, format!("🇵🇱 PGA ({})", awards.worked_pga.len())),
            ];
            for (idx, title) in subtabs {
                if ui.selectable_label(*subtab == idx, title).clicked() {
                    *subtab = idx;
                }
            }
        });

        ui.separator();

        // Wybór aktualnego źródła danych
        let (title, color, details_map) = match *subtab {
            0 => (
                tr(key::AWARDS_OTHER_WPX, lang),
                palette.accent,
                &awards.details_wpx,
            ),
            1 => (
                tr(key::AWARDS_OTHER_IOTA, lang),
                palette.success,
                &awards.details_iota,
            ),
            2 => (
                tr(key::AWARDS_OTHER_VUCC, lang),
                palette.series(2),
                &awards.details_vucc,
            ),
            3 => (
                tr(key::AWARDS_OTHER_SOTA, lang),
                palette.warning,
                &awards.details_sota,
            ),
            4 => (
                tr(key::AWARDS_OTHER_POTA, lang),
                palette.series(1),
                &awards.details_pota,
            ),
            _ => (
                tr(key::AWARDS_OTHER_PGA, lang),
                palette.danger,
                &awards.details_pga,
            ),
        };

        // Obliczenia statystyczne
        let total_entities = details_map.len();
        let mut total_confirmed = 0;
        let mut total_qsos = 0;
        for qsos in details_map.values() {
            total_qsos += qsos.len();
            if qsos.iter().any(|q| q.is_confirmed) {
                total_confirmed += 1;
            }
        }

        // Pasek podsumowania i wyszukiwarka
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(title).strong().size(13.0).color(color));
            ui.separator();
            ui.label(
                Msg::new(key::AWARDS_OTHER_SUMMARY)
                    .arg("worked", total_entities)
                    .arg("confirmed", total_confirmed)
                    .arg("total", total_qsos)
                    .text(lang),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(search_query)
                        .hint_text(tr(key::AWARDS_SEARCH_HINT, lang))
                        .desired_width(180.0),
                );
            });
        });

        ui.add_space(4.0);

        // Pobieramy posortowaną listę kluczy
        let query = search_query.trim().to_uppercase();
        let mut keys: Vec<&String> = details_map.keys().collect();
        keys.sort();

        // Filtrowanie kluczy
        let filtered_keys: Vec<&String> = keys
            .into_iter()
            .filter(|k| {
                if query.is_empty() {
                    return true;
                }
                if k.to_uppercase().contains(&query) {
                    return true;
                }
                if let Some(list) = details_map.get(*k) {
                    list.iter().any(|q| {
                        q.callsign.to_uppercase().contains(&query)
                            || q.band.to_uppercase().contains(&query)
                            || q.mode.to_uppercase().contains(&query)
                            || q.qso_date.contains(&query)
                    })
                } else {
                    false
                }
            })
            .collect();

        if filtered_keys.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                if total_entities == 0 {
                    ui.label(
                        egui::RichText::new(tr(key::AWARDS_EMPTY_AWARD_QSOS, lang))
                            .size(12.0)
                            .color(palette.text_muted),
                    );
                    ui.label(tr(key::AWARDS_EMPTY_AWARD_HINT, lang));
                } else {
                    ui.label(
                        egui::RichText::new(tr(key::AWARDS_NO_SEARCH_RESULTS, lang))
                            .size(12.0)
                            .color(palette.warning),
                    );
                }
            });
        } else {
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    egui::Grid::new("awards_other_detail_grid")
                        .striped(true)
                        .spacing([12.0, 6.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            // Nagłówek tabeli
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_CODE, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_QSOS, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_LAST_CALLSIGN, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_BAND, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_MODE, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_QSO_DATE, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::AWARDS_COL_STATUS, lang))
                                    .strong()
                                    .color(palette.text_muted),
                            );
                            ui.end_row();

                            for key in filtered_keys {
                                if let Some(qsos) = details_map.get(key) {
                                    let Some(last_qso) = qsos.last() else {
                                        continue;
                                    };
                                    let is_conf = qsos.iter().any(|q| q.is_confirmed);

                                    ui.label(
                                        egui::RichText::new(key.as_str())
                                            .monospace()
                                            .strong()
                                            .color(color),
                                    );
                                    ui.label(format!("{} QSO", qsos.len()));
                                    ui.label(
                                        egui::RichText::new(&last_qso.callsign)
                                            .monospace()
                                            .strong(),
                                    );
                                    ui.label(&last_qso.band);
                                    ui.label(&last_qso.mode);
                                    ui.label(&last_qso.qso_date);

                                    if is_conf {
                                        ui.colored_label(
                                            palette.success,
                                            tr(key::AWARDS_STATUS_CONFIRMED, lang),
                                        );
                                    } else {
                                        ui.colored_label(
                                            palette.warning,
                                            tr(key::AWARDS_STATUS_WORKED, lang),
                                        );
                                    }
                                    ui.end_row();
                                }
                            }
                        });
                });
        }
    });
}

fn render_tab_sp_dx(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine, lang: Language) {
    let palette = ui.palette();
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(tr(key::AWARDS_SP_DX_TITLE, lang))
                    .size(14.0)
                    .color(palette.danger),
            );
        });

        let worked = awards.worked_sp_districts.len();
        let confirmed = awards.confirmed_sp_districts.len();
        let total = 9; // SP1-SP9 districts

        ui.horizontal(|ui| {
            ui.colored_label(
                palette.warning,
                Msg::new(key::AWARDS_SP_DX_DISTRICT_WORKED)
                    .arg("worked", worked)
                    .arg("total", total)
                    .text(lang),
            );
            ui.separator();
            ui.colored_label(
                palette.success,
                Msg::new(key::AWARDS_SP_DX_DISTRICT_CONFIRMED)
                    .arg("confirmed", confirmed)
                    .arg("total", total)
                    .text(lang),
            );
        });

        ui.add_space(4.0);

        // Siatka okręgów SP1-SP9
        let districts = [
            "SP1", "SP2", "SP3", "SP4", "SP5", "SP6", "SP7", "SP8", "SP9",
        ];
        let district_names = [
            tr(key::AWARDS_DISTRICT_SP1, lang),
            tr(key::AWARDS_DISTRICT_SP2, lang),
            tr(key::AWARDS_DISTRICT_SP3, lang),
            tr(key::AWARDS_DISTRICT_SP4, lang),
            tr(key::AWARDS_DISTRICT_SP5, lang),
            tr(key::AWARDS_DISTRICT_SP6, lang),
            tr(key::AWARDS_DISTRICT_SP7, lang),
            tr(key::AWARDS_DISTRICT_SP8, lang),
            tr(key::AWARDS_DISTRICT_SP9, lang),
        ];

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("sp_districts_grid")
                .num_columns(3)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (i, district) in districts.iter().enumerate() {
                        let confirmed = awards.confirmed_sp_districts.contains(*district);
                        let worked = awards.worked_sp_districts.contains(*district);

                        let color = if confirmed {
                            palette.success
                        } else if worked {
                            palette.warning
                        } else {
                            palette.text_muted
                        };

                        ui.colored_label(color, format!("■ {district}"));
                        ui.label(district_names[i]);
                        if confirmed {
                            ui.label(tr(key::AWARDS_SP_DX_CONFIRMED_ICON, lang));
                        } else if worked {
                            ui.label(tr(key::AWARDS_SP_DX_WORKED_ICON, lang));
                        } else {
                            ui.label(tr(key::AWARDS_SP_DX_NEEDED_ICON, lang));
                        }
                        ui.end_row();
                    }
                });
        });
    });
}

fn render_tab_wae(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine, lang: Language) {
    let palette = ui.palette();
    ui.vertical(|ui| {
        ui.heading(
            egui::RichText::new(tr(key::AWARDS_WAE_TITLE, lang))
                .size(14.0)
                .color(palette.accent),
        );

        let worked = awards.worked_wae.len();
        let confirmed = awards.confirmed_wae.len();
        let total = crate::core::awards::WAE_EUROPEAN_ENTITIES.len();

        ui.horizontal(|ui| {
            ui.colored_label(
                palette.warning,
                Msg::new(key::AWARDS_WAE_STATUS_WORKED)
                    .arg("worked", worked)
                    .arg("total", total)
                    .text(lang),
            );
            ui.separator();
            ui.colored_label(
                palette.success,
                Msg::new(key::AWARDS_WAE_STATUS_CONFIRMED)
                    .arg("confirmed", confirmed)
                    .arg("total", total)
                    .text(lang),
            );
        });

        let progress = worked as f32 / total.max(1) as f32;
        ui.add(
            egui::ProgressBar::new(progress)
                .text(format!("{:.0}%", progress * 100.0))
                .fill(palette.accent),
        );

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tr(key::AWARDS_WAE_DESCRIPTION, lang))
                .weak()
                .size(11.0),
        );

        ui.separator();
        ui.horizontal(|ui| {
            ui.colored_label(palette.success, tr(key::AWARDS_WAE_LEGEND_CONFIRMED, lang));
            ui.colored_label(palette.warning, tr(key::AWARDS_WAE_LEGEND_WORKED, lang));
            ui.colored_label(palette.text_muted, tr(key::AWARDS_WAE_LEGEND_NEEDED, lang));
        });
        ui.label(
            Msg::new(key::AWARDS_WAE_REMAINING_NEEDED)
                .arg("remaining", total.saturating_sub(worked))
                .text(lang),
        );
    });
}
