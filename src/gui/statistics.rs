// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Wozniak (SP6INA)

//! Modul statystyk i wykresow QSO — wykresy pasmo/emisja/godzina/miesiac/QSL

use crate::gui::app::SpLogApp;
use crate::core::i18n::tr;
use eframe::egui;

pub fn render_statistics_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_statistics_window { return; }
    let lang = app.current_language;
    let mut is_open = app.show_statistics_window;

    egui::Window::new(egui::RichText::new(format!("📊 {}", tr("stats.window_title", lang))).size(12.0).strong())
        .open(&mut is_open)
        .default_size([780.0, 580.0])
        .min_size([500.0, 400.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new(tr("stats.center_title", lang))
                    .size(15.0).color(egui::Color32::from_rgb(56, 189, 248)));
            });
            ui.add_space(2.0);

            // Zakładki statystyk
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.stats_tab, 0, format!("📅 {}", tr("stats.tab_monthly", lang)));
                ui.selectable_value(&mut app.stats_tab, 1, format!("📻 {}", tr("stats.tab_bands", lang)));
                ui.selectable_value(&mut app.stats_tab, 2, format!("📡 {}", tr("stats.tab_modes", lang)));
                ui.selectable_value(&mut app.stats_tab, 3, format!("⏰ {}", tr("stats.tab_hourly", lang)));
                ui.selectable_value(&mut app.stats_tab, 4, format!("🌍 {}", tr("stats.tab_countries", lang)));
                ui.selectable_value(&mut app.stats_tab, 5, format!("📬 {}", tr("stats.tab_qsl", lang)));
            });
            ui.separator();

            match app.stats_tab {
                0 => render_tab_monthly(app, ui),
                1 => render_tab_bands(app, ui),
                2 => render_tab_modes(app, ui),
                3 => render_tab_hourly(app, ui),
                4 => render_tab_countries(app, ui),
                _ => render_tab_qsl(app, ui),
            }
        });

    app.show_statistics_window = is_open;
}

fn get_stats_band(app: &SpLogApp) -> Vec<(String, i64)> {
    if let Ok(db) = app.log_db.lock() {
        db.stats_qso_per_band().unwrap_or_default()
    } else { vec![] }
}

fn get_stats_mode(app: &SpLogApp) -> Vec<(String, i64)> {
    if let Ok(db) = app.log_db.lock() {
        db.stats_qso_per_mode().unwrap_or_default()
    } else { vec![] }
}

fn get_stats_monthly(app: &SpLogApp) -> Vec<(String, i64)> {
    if let Ok(db) = app.log_db.lock() {
        let mut v = db.stats_qso_per_month().unwrap_or_default();
        v.reverse(); // chronologicznie
        v
    } else { vec![] }
}

fn get_stats_hourly(app: &SpLogApp) -> Vec<(u32, i64)> {
    if let Ok(db) = app.log_db.lock() {
        db.stats_activity_by_hour().unwrap_or_default()
    } else { vec![] }
}

fn get_stats_countries(app: &SpLogApp) -> Vec<(String, i64)> {
    if let Ok(db) = app.log_db.lock() {
        db.stats_top_countries(15).unwrap_or_default()
    } else { vec![] }
}

fn get_stats_qsl(app: &SpLogApp) -> (i64, i64, i64, i64) {
    if let Ok(db) = app.log_db.lock() {
        db.stats_qsl_summary().unwrap_or((0, 0, 0, 0))
    } else { (0, 0, 0, 0) }
}

/// Rysuje pionowy slupek na pozycji x, szerokosci w, wysokosci h (pixels), z kolorem i etykieta
fn bar_chart(ui: &mut egui::Ui, data: &[(String, i64)], color: egui::Color32, empty_text: &str) {
    if data.is_empty() {
        ui.label(empty_text);
        return;
    }
    let max_val = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let chart_h = 220.0f32;
    let bar_w = ((ui.available_width() - 60.0) / data.len() as f32).clamp(4.0, 60.0);
    let total_w = bar_w * data.len() as f32 + 60.0;

    let (resp, painter) = ui.allocate_painter(egui::vec2(total_w, chart_h + 40.0), egui::Sense::hover());
    let origin = resp.rect.min;

    for (i, (label, val)) in data.iter().enumerate() {
        let bar_h = (*val as f32 / max_val as f32) * chart_h;
        let x = origin.x + 30.0 + i as f32 * bar_w;
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(x + 1.0, origin.y + chart_h - bar_h),
            egui::vec2(bar_w - 2.0, bar_h),
        );
        painter.rect_filled(bar_rect, 2.0, color);

        // Wartość nad słupkiem
        if bar_h > 16.0 {
            painter.text(
                egui::pos2(x + bar_w / 2.0, origin.y + chart_h - bar_h - 2.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{}", val),
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
        }

        // Etykieta pod słupkiem (rotacja przez skrócenie)
        let short_label = if label.len() > 6 { &label[..6] } else { label.as_str() };
        painter.text(
            egui::pos2(x + bar_w / 2.0, origin.y + chart_h + 4.0),
            egui::Align2::CENTER_TOP,
            short_label,
            egui::FontId::proportional(9.0),
            egui::Color32::from_rgb(148, 163, 184),
        );
    }

    // Linia bazowa
    painter.line_segment(
        [egui::pos2(origin.x + 28.0, origin.y + chart_h), egui::pos2(origin.x + total_w, origin.y + chart_h)],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 116, 139)),
    );
}

fn render_tab_monthly(app: &SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.label(egui::RichText::new(tr("stats.monthly_dist", lang)).strong());
    ui.add_space(4.0);
    let data = get_stats_monthly(app);
    egui::ScrollArea::horizontal().show(ui, |ui| {
        bar_chart(ui, &data, egui::Color32::from_rgb(56, 189, 248), tr("log.no_results", lang));
    });
    ui.add_space(8.0);
    // Tabela podsumowania
    ui.separator();
    let total: i64 = data.iter().map(|(_, v)| v).sum();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}: {} QSO ({} m.)", tr("stats.total_qso_count", lang), total, data.len())).strong());
        if let Some((best_month, best_val)) = data.iter().max_by_key(|(_, v)| v) {
            ui.label(format!("  |  MAX: {} ({} QSO)", best_month, best_val));
        }
    });
}

fn render_tab_bands(app: &SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.label(egui::RichText::new(tr("stats.bands_dist", lang)).strong());
    ui.add_space(4.0);
    let data = get_stats_band(app);
    bar_chart(ui, &data, egui::Color32::from_rgb(251, 191, 36), tr("log.no_results", lang));
    ui.separator();
    egui::Grid::new("band_table").striped(true).spacing([20.0, 4.0]).show(ui, |ui| {
        ui.label(egui::RichText::new(tr("qso.band", lang)).strong());
        ui.label(egui::RichText::new("QSO").strong());
        ui.label(egui::RichText::new("%").strong());
        ui.end_row();
        let total: i64 = data.iter().map(|(_, v)| v).sum::<i64>().max(1);
        for (band, cnt) in &data {
            ui.colored_label(egui::Color32::from_rgb(251, 191, 36), band);
            ui.label(format!("{}", cnt));
            ui.label(format!("{:.1}%", *cnt as f64 / total as f64 * 100.0));
            ui.end_row();
        }
    });
}

fn render_tab_modes(app: &SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.label(egui::RichText::new(tr("stats.modes_dist", lang)).strong());
    ui.add_space(4.0);
    let data = get_stats_mode(app);
    bar_chart(ui, &data, egui::Color32::from_rgb(34, 197, 94), tr("log.no_results", lang));
    ui.separator();
    egui::Grid::new("mode_table").striped(true).spacing([20.0, 4.0]).show(ui, |ui| {
        ui.label(egui::RichText::new(tr("qso.mode", lang)).strong());
        ui.label(egui::RichText::new("QSO").strong());
        ui.label(egui::RichText::new("%").strong());
        ui.end_row();
        let total: i64 = data.iter().map(|(_, v)| v).sum::<i64>().max(1);
        for (mode, cnt) in &data {
            ui.colored_label(egui::Color32::from_rgb(34, 197, 94), mode);
            ui.label(format!("{}", cnt));
            ui.label(format!("{:.1}%", *cnt as f64 / total as f64 * 100.0));
            ui.end_row();
        }
    });
}

fn render_tab_hourly(app: &SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.label(egui::RichText::new(tr("stats.activity_histogram", lang)).strong());
    ui.add_space(4.0);
    let raw = get_stats_hourly(app);
    // Uzupełnij brakujące godziny
    let mut hours = vec![(String::new(), 0i64); 24];
    for (h, cnt) in &raw {
        if (*h as usize) < 24 {
            hours[*h as usize] = (format!("{:02}:00", h), *cnt);
        }
    }
    for (h, item) in hours.iter_mut().enumerate() {
        if item.0.is_empty() { item.0 = format!("{:02}:00", h); }
    }
    egui::ScrollArea::horizontal().show(ui, |ui| {
        bar_chart(ui, &hours, egui::Color32::from_rgb(147, 197, 253), tr("log.no_results", lang));
    });
    ui.separator();
    let peak = hours.iter().enumerate().max_by_key(|(_, (_, v))| v);
    if let Some((h, (_, cnt))) = peak {
        if *cnt > 0 {
            ui.label(format!("{}: {:02}:00–{:02}:00 UTC ({} QSO)", tr("stats.peak_activity", lang), h, (h+1)%24, cnt));
        }
    }
}

fn render_tab_countries(app: &SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.label(egui::RichText::new(tr("stats.top_countries_title", lang)).strong());
    ui.add_space(4.0);
    let data = get_stats_countries(app);
    let max_val = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("countries_grid").striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label(egui::RichText::new("#").strong());
            ui.label(egui::RichText::new(tr("geo.country", lang)).strong());
            ui.label(egui::RichText::new("QSO").strong());
            ui.label(egui::RichText::new("%").strong());
            ui.end_row();

            for (rank, (country, cnt)) in data.iter().enumerate() {
                ui.label(format!("{}.", rank + 1));
                ui.label(egui::RichText::new(country).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.label(format!("{}", cnt));

                // Mini progress bar jako wykres
                let progress = *cnt as f32 / max_val as f32;
                ui.add(
                    egui::ProgressBar::new(progress)
                        .desired_width(160.0)
                        .fill(egui::Color32::from_rgb(56, 189, 248))
                );
                ui.end_row();
            }
        });
    });
}

fn render_tab_qsl(app: &SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    let (total, lotw, eqsl, paper) = get_stats_qsl(app);
    ui.label(egui::RichText::new(tr("stats.qsl_summary_title", lang)).strong());
    ui.add_space(8.0);

    if total == 0 {
        ui.label(tr("log.no_results", lang));
        return;
    }

    egui::Grid::new("qsl_grid").spacing([20.0, 8.0]).show(ui, |ui| {
        ui.label(egui::RichText::new(format!("{}:", tr("stats.total_qso_count", lang))).strong());
        ui.label(egui::RichText::new(format!("{}", total)).size(18.0).color(egui::Color32::WHITE));
        ui.end_row();

        ui.label(egui::RichText::new(format!("{}:", tr("stats.lotw_confirmed_label", lang))).strong().color(egui::Color32::from_rgb(34, 197, 94)));
        ui.label(egui::RichText::new(format!("{} ({:.1}%)", lotw, lotw as f64 / total as f64 * 100.0)).color(egui::Color32::from_rgb(34, 197, 94)));
        ui.end_row();

        ui.label(egui::RichText::new(format!("{}:", tr("stats.eqsl_confirmed_label", lang))).strong().color(egui::Color32::from_rgb(56, 189, 248)));
        ui.label(egui::RichText::new(format!("{} ({:.1}%)", eqsl, eqsl as f64 / total as f64 * 100.0)).color(egui::Color32::from_rgb(56, 189, 248)));
        ui.end_row();

        ui.label(egui::RichText::new(format!("{}:", tr("stats.paper_qsl_label", lang))).strong().color(egui::Color32::from_rgb(250, 204, 21)));
        ui.label(egui::RichText::new(format!("{} ({:.1}%)", paper, paper as f64 / total as f64 * 100.0)).color(egui::Color32::from_rgb(250, 204, 21)));
        ui.end_row();

        let unconfirmed = total - lotw.max(eqsl).max(paper);
        ui.label(egui::RichText::new(format!("{}:", tr("stats.unconfirmed_label", lang))).strong().color(egui::Color32::from_rgb(100, 116, 139)));
        ui.label(egui::RichText::new(format!("{} ({:.1}%)", unconfirmed.max(0), unconfirmed.max(0) as f64 / total as f64 * 100.0)).color(egui::Color32::from_rgb(100, 116, 139)));
        ui.end_row();
    });

    ui.add_space(12.0);
    ui.separator();

    // Wizualizacja słupkowa
    ui.label(egui::RichText::new(tr("stats.visualization_label", lang)).strong());
    ui.add_space(4.0);

    let bar_data = vec![
        ("LoTW".to_string(), lotw),
        ("eQSL".to_string(), eqsl),
        ("Papier".to_string(), paper),
        ("Brak".to_string(), (total - lotw.max(eqsl).max(paper)).max(0)),
    ];
    bar_chart(ui, &bar_data, egui::Color32::from_rgb(56, 189, 248), tr("log.no_results", lang));
}
