// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::awards::US_STATES;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Okno matrycy dyplomowej (DXCC, WAZ, WAS, WAC, WPX, VUCC, IOTA, SOTA, POTA, PGA)
pub fn render_awards_matrix_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_awards_matrix_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_awards_matrix_window;
    let mut close_req = false;

    egui::Window::new(egui::RichText::new(format!("🏆 {}", tr("awards.matrix_title", lang))).size(12.0).strong())
        .open(&mut is_open)
        .default_size([720.0, 560.0])
        .min_size([500.0, 360.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("Centrum Osiągnięć Dyplomowych SPLogbook")
                            .size(15.0)
                            .color(egui::Color32::from_rgb(56, 189, 248)),
                    );
                });

                ui.add_space(2.0);

                // Zakładki dyplomów
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.awards_matrix_tab, 0, "🌍 DXCC");
                    ui.selectable_value(&mut app.awards_matrix_tab, 1, "🌐 WAZ (Strefy CQ)");
                    ui.selectable_value(&mut app.awards_matrix_tab, 2, "🇺🇸 WAS (Stany USA)");
                    ui.selectable_value(&mut app.awards_matrix_tab, 3, "🌍 WAC (Kontynenty)");
                    ui.selectable_value(&mut app.awards_matrix_tab, 4, "🏆 Inne (WPX/IOTA/VUCC/PGA)");
                    ui.selectable_value(&mut app.awards_matrix_tab, 5, "🇵🇱 SP DX Award");
                    ui.selectable_value(&mut app.awards_matrix_tab, 6, "🌍 WAE (Europe)");
                });

                ui.separator();

                let awards = app.awards_engine.lock().unwrap();

                match app.awards_matrix_tab {
                    0 => render_tab_dxcc(app, ui, &awards),
                    1 => render_tab_waz(ui, &awards),
                    2 => render_tab_was(ui, &awards),
                    3 => render_tab_wac(ui, &awards),
                    5 => render_tab_sp_dx(ui, &awards),
                    6 => render_tab_wae(ui, &awards),
                    _ => render_tab_other(&mut app.awards_other_subtab, &mut app.awards_other_search, ui, &awards),
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr("btn.close", lang)).clicked() {
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

fn render_legend(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "■ Potwierdzone (QSL/LoTW)");
        ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "■ Zrobione (Worked)");
        ui.colored_label(egui::Color32::from_rgb(100, 116, 139), "■ Potrzebne (Needed)");
    });
}

fn render_tab_dxcc(app: &SpLogApp, ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine) {
    let worked_dxcc = awards.worked_dxcc_all.len();
    let conf_dxcc = awards.confirmed_dxcc.len();
    let total_qsos = app.recent_qsos.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(egui::RichText::new("🌍 Wszystkie podmioty DXCC").strong().size(12.0).color(egui::Color32::from_rgb(250, 204, 21)));
            ui.label(format!("Zrobione: {} | Potwierdzone: {}", worked_dxcc, conf_dxcc));
        });

        ui.group(|ui| {
            ui.label(egui::RichText::new("📊 Pasmo-Kraje").strong().size(12.0).color(egui::Color32::from_rgb(56, 189, 248)));
            ui.label(format!("Sloty: {} band-slots", awards.worked_dxcc_band.len()));
        });

        ui.group(|ui| {
            ui.label(egui::RichText::new("📋 Łącznie w logu").strong().size(12.0).color(egui::Color32::from_rgb(216, 180, 254)));
            ui.label(format!("{} łączności", total_qsos));
        });
    });

    ui.add_space(4.0);
    render_legend(ui);
    ui.add_space(4.0);

    let bands = &["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "4m", "2m", "70cm"];
    let modes = &["CW", "SSB", "DIGI", "MIXED"];

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("awards_dxcc_grid")
            .striped(true)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Pasmo").strong());
                for m in modes {
                    ui.label(egui::RichText::new(*m).strong());
                }
                ui.label(egui::RichText::new("Łączności").strong());
                ui.end_row();

                for b in bands {
                    ui.label(egui::RichText::new(*b).strong().color(egui::Color32::from_rgb(56, 189, 248)));

                    for m in modes {
                        let worked = app.recent_qsos.iter().any(|q| {
                            q.band == *b && (m == &"MIXED" || q.mode == *m || (*m == "DIGI" && (q.mode == "FT8" || q.mode == "FT4" || q.mode == "RTTY")))
                        });

                        let confirmed = app.recent_qsos.iter().any(|q| {
                            q.band == *b
                                && (m == &"MIXED" || q.mode == *m || (*m == "DIGI" && (q.mode == "FT8" || q.mode == "FT4" || q.mode == "RTTY")))
                                && (q.lotw_qsl_rcvd == "Y" || q.qsl_rcvd == "Y" || q.eqsl_qsl_rcvd == "Y")
                        });

                        if confirmed {
                            ui.label(egui::RichText::new("✓ C").color(egui::Color32::from_rgb(34, 197, 94)).strong());
                        } else if worked {
                            ui.label(egui::RichText::new("● W").color(egui::Color32::from_rgb(250, 204, 21)));
                        } else {
                            ui.label(egui::RichText::new("-").color(egui::Color32::from_rgb(100, 116, 139)));
                        }
                    }

                    let band_qsos = app.recent_qsos.iter().filter(|q| q.band == *b).count();
                    ui.label(egui::RichText::new(format!("{} QSO", band_qsos)).size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    ui.end_row();
                }
            });
    });
}

fn render_tab_waz(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine) {
    let worked_count = awards.worked_waz.len();
    let conf_count = awards.confirmed_waz.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(egui::RichText::new("🌐 WAZ (Worked All Zones - 40 Stref CQ)").strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
            ui.label(format!("Zrobione: {} / 40 | Potwierdzone: {} / 40 | Potrzebne: {}", worked_count, conf_count, 40 - worked_count));
        });
    });

    ui.add_space(4.0);
    render_legend(ui);
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
                        (egui::Color32::from_rgb(34, 197, 94), "✓")
                    } else if is_wrk {
                        (egui::Color32::from_rgb(250, 204, 21), "●")
                    } else {
                        (egui::Color32::from_rgb(100, 116, 139), "-")
                    };

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Z{:02}", zone)).strong().size(11.0));
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

fn render_tab_was(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine) {
    let worked_count = awards.worked_was.len();
    let conf_count = awards.confirmed_was.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(egui::RichText::new("🇺🇸 WAS (Worked All States - 50 Stanów USA)").strong().size(13.0).color(egui::Color32::from_rgb(244, 63, 94)));
            ui.label(format!("Zrobione: {} / 50 | Potwierdzone: {} / 50 | Potrzebne: {}", worked_count, conf_count, 50 - worked_count));
        });
    });

    ui.add_space(4.0);
    render_legend(ui);
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
                        (egui::Color32::from_rgb(34, 197, 94), "✓")
                    } else if is_wrk {
                        (egui::Color32::from_rgb(250, 204, 21), "●")
                    } else {
                        (egui::Color32::from_rgb(100, 116, 139), "-")
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

fn render_tab_wac(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine) {
    let worked_count = awards.worked_wac.len();
    let conf_count = awards.confirmed_wac.len();

    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label(egui::RichText::new("🌍 WAC (Worked All Continents - 7 Kontynentów)").strong().size(13.0).color(egui::Color32::from_rgb(234, 179, 8)));
            ui.label(format!("Zrobione: {} / 7 | Potwierdzone: {} / 7", worked_count, conf_count));
        });
    });

    ui.add_space(4.0);
    render_legend(ui);
    ui.add_space(8.0);

    let names = [
        ("AF", "Afryka (Africa)"),
        ("AN", "Antarktyda (Antarctica)"),
        ("AS", "Azja (Asia)"),
        ("EU", "Europa (Europe)"),
        ("NA", "Ameryka Północna (North America)"),
        ("OC", "Oceania"),
        ("SA", "Ameryka Południowa (South America)"),
    ];

    ui.vertical(|ui| {
        for (code, name) in names {
            let is_conf = awards.confirmed_wac.contains(code);
            let is_wrk = awards.worked_wac.contains(code);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(code).strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.label(name);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_conf {
                            ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "✓ POTWIERDZONY (Confirmed)");
                        } else if is_wrk {
                            ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "● ZROBIONY (Worked)");
                        } else {
                            ui.colored_label(egui::Color32::from_rgb(100, 116, 139), "- POTRZEBNY (Needed)");
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
) {
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
            0 => ("CQ WPX (Prefiksy)", egui::Color32::from_rgb(56, 189, 248), &awards.details_wpx),
            1 => ("RSGB IOTA (Wyspy Świata)", egui::Color32::from_rgb(20, 184, 166), &awards.details_iota),
            2 => ("ARRL VUCC (Kwadraty Maidenhead VHF/UHF)", egui::Color32::from_rgb(168, 85, 247), &awards.details_vucc),
            3 => ("SOTA (Szczyty Górskie)", egui::Color32::from_rgb(234, 179, 8), &awards.details_sota),
            4 => ("POTA (Parki Krajobrazowe)", egui::Color32::from_rgb(34, 197, 94), &awards.details_pota),
            _ => ("PGA (Polska Gmina Award)", egui::Color32::from_rgb(239, 68, 68), &awards.details_pga),
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
            ui.label(format!("Zrobione: {} | Potwierdzone: {} | Łącznie łączności: {}", total_entities, total_confirmed, total_qsos));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(search_query).hint_text("🔍 Szukaj kodu, znaku, pasma...").desired_width(180.0));
            });
        });

        ui.add_space(4.0);

        // Pobieramy posortowaną listę kluczy
        let query = search_query.trim().to_uppercase();
        let mut keys: Vec<&String> = details_map.keys().collect();
        keys.sort();

        // Filtrowanie kluczy
        let filtered_keys: Vec<&String> = keys.into_iter().filter(|k| {
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
        }).collect();

        if filtered_keys.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                if total_entities == 0 {
                    ui.label(egui::RichText::new("Brak zarejestrowanych łączności dla tego dyplomu w bieżącym dzienniku.").size(12.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    ui.label("Wprowadź QSO z odpowiednim polem (lokator, IOTA, PGA, SOTA) aby śledzić postępy!");
                } else {
                    ui.label(egui::RichText::new("Brak wyników odpowiadających wpisanej frazie wyszukiwania.").size(12.0).color(egui::Color32::from_rgb(251, 146, 60)));
                }
            });
        } else {
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                egui::Grid::new("awards_other_detail_grid")
                    .striped(true)
                    .spacing([12.0, 6.0])
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        // Nagłówek tabeli
                        ui.label(egui::RichText::new("Kod").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.label(egui::RichText::new("Łączności").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.label(egui::RichText::new("Ostatni znak").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.label(egui::RichText::new("Pasmo").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.label(egui::RichText::new("Emisja").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.label(egui::RichText::new("Data QSO").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.label(egui::RichText::new("Status").strong().color(egui::Color32::from_rgb(203, 213, 225)));
                        ui.end_row();

                        for key in filtered_keys {
                            if let Some(qsos) = details_map.get(key) {
                                let last_qso = qsos.last().unwrap();
                                let is_conf = qsos.iter().any(|q| q.is_confirmed);

                                ui.label(egui::RichText::new(key.as_str()).monospace().strong().color(color));
                                ui.label(format!("{} QSO", qsos.len()));
                                ui.label(egui::RichText::new(&last_qso.callsign).monospace().strong());
                                ui.label(&last_qso.band);
                                ui.label(&last_qso.mode);
                                ui.label(&last_qso.qso_date);

                                if is_conf {
                                    ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "✓ POTWIERDZONE");
                                } else {
                                    ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "● ZROBIONE");
                                }
                                ui.end_row();
                            }
                        }
                    });
            });
        }
    });
}

fn render_tab_sp_dx(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("🇵🇱 SP DX Award — Worked Polish Districts")
                .size(14.0).color(egui::Color32::from_rgb(220, 38, 38)));
        });
        
        let worked = awards.worked_sp_districts.len();
        let confirmed = awards.confirmed_sp_districts.len();
        let total = 9; // SP1-SP9 districts
        
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(250, 204, 21),
                format!("Pracowane okręgi: {}/{}", worked, total)
            );
            ui.separator();
            ui.colored_label(
                egui::Color32::from_rgb(34, 197, 94),
                format!("Potwierdzone: {}/{}", confirmed, total)
            );
        });
        
        ui.add_space(4.0);
        
        // Siatka okręgów SP1-SP9
        let districts = ["SP1", "SP2", "SP3", "SP4", "SP5", "SP6", "SP7", "SP8", "SP9"];
        let district_names = [
            "Zachodniopomorskie/Lubuskie", "Kujawsko-Pomorskie/Pomorskie",
            "Wielkopolskie", "Łódzkie", "Mazowieckie",
            "Dolnośląskie/Opolskie", "Świętokrzyskie/Małopolskie",
            "Podkarpackie/Lubelskie", "Podlaskie/Warmińsko-Mazurskie"
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
                            egui::Color32::from_rgb(34, 197, 94)
                        } else if worked {
                            egui::Color32::from_rgb(250, 204, 21)
                        } else {
                            egui::Color32::from_rgb(100, 116, 139)
                        };
                        
                        ui.colored_label(color, format!("■ {}", district));
                        ui.label(district_names[i]);
                        if confirmed {
                            ui.label("✅ Potwierdzone");
                        } else if worked {
                            ui.label("☑ Pracowane");
                        } else {
                            ui.label("○ Potrzebne");
                        }
                        ui.end_row();
                    }
                });
        });
    });
}

fn render_tab_wae(ui: &mut egui::Ui, awards: &crate::core::awards::AwardsEngine) {
    ui.vertical(|ui| {
        ui.heading(egui::RichText::new("🌍 WAE — Worked All Europe")
            .size(14.0).color(egui::Color32::from_rgb(56, 189, 248)));
        
        let worked = awards.worked_wae.len();
        let confirmed = awards.confirmed_wae.len();
        let total = crate::core::awards::WAE_EUROPEAN_ENTITIES.len();
        
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(250, 204, 21),
                format!("Pracowane: {}/{}", worked, total)
            );
            ui.separator();
            ui.colored_label(
                egui::Color32::from_rgb(34, 197, 94),
                format!("Potwierdzone: {}/{}", confirmed, total)
            );
        });
        
        let progress = worked as f32 / total.max(1) as f32;
        ui.add(egui::ProgressBar::new(progress)
            .text(format!("{:.0}%", progress * 100.0))
            .fill(egui::Color32::from_rgb(56, 189, 248))
        );
        
        ui.add_space(4.0);
        ui.label(egui::RichText::new(
            "WAE Award wymaga przeprowadzenia łączności ze stacjami ze wszystkich europejskich enklaw DXCC."
        ).weak().size(11.0));
        
        ui.separator();
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "■ Potwierdzone");
            ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "■ Pracowane");
            ui.colored_label(egui::Color32::from_rgb(100, 116, 139), "■ Potrzebne");
        });
        ui.label(format!("Potrzeba jeszcze {} europejskich enklaw DXCC", total.saturating_sub(worked)));
    });
}

