// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Wozniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;
use egui_extras::{Column, TableBuilder};

pub fn render_logbook_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_log.visible {
        return;
    }
    let lang = app.current_language;
    if app.panel_log.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("logbook_viewport"),
            format!(
                "📋 {} ({} QSO) - SPLogbook",
                tr("tab.logbook", lang),
                app.recent_qsos.len()
            ),
            [900.0, 560.0],
            [500.0, 280.0],
            app.panel_log.saved_pos,
            app.panel_log.saved_size,
            |app, ui| {
                egui::Panel::top("logbook_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("↙ {}", tr("window.dock", lang)))
                            .clicked()
                        {
                            dock_back = true;
                        }
                        ui.separator();
                        if ui.button("🔄").clicked() {
                            app.reload_qsos();
                        }
                        if ui
                            .button(format!("⚙ {}", tr("columns.title", lang)))
                            .clicked()
                        {
                            app.show_column_settings = true;
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut app.log_search_query)
                                .hint_text(tr("qso.search", lang))
                                .desired_width(200.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}: {} QSO",
                                    tr("log.total_qsos", lang),
                                    app.recent_qsos.len()
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(148, 163, 184)),
                            );
                        });
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_logbook_body(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );
        if let Some((pos, size)) = captured_geo {
            if app.panel_log.saved_pos != Some(pos) || app.panel_log.saved_size != Some(size) {
                app.panel_log.saved_pos = Some(pos);
                app.panel_log.saved_size = Some(size);
                app.save_station_config();
            }
        }
        if dock_back {
            app.panel_log.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_log.visible = false;
            app.panel_log.floating = false;
            app.save_station_config();
        }
        return;
    }
    let mut open = app.panel_log.visible;
    let screen = ctx.content_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let default_pos = [screen.min.x + 488.0, screen.min.y + 8.0];
    let default_size = [mid_w, (screen.height() * 0.56).max(340.0)];
    let mut win = egui::Window::new(
        egui::RichText::new(format!(
            "   📋 {} ({} QSO)",
            tr("tab.logbook", lang),
            app.recent_qsos.len()
        ))
        .size(12.0)
        .strong(),
    )
    .open(&mut open)
    .min_size([400.0, 240.0])
    .resizable(true)
    .collapsible(true)
    .constrain_to(screen);
    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_log.saved_pos {
        let sz = app.panel_log.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }
    let win_res = win.show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("🔄").clicked() {
                app.reload_qsos();
            }
            if ui
                .button(format!("⚙ {}", tr("columns.title", lang)))
                .clicked()
            {
                app.show_column_settings = true;
            }
            ui.add(
                egui::TextEdit::singleline(&mut app.log_search_query)
                    .hint_text(tr("qso.search", lang))
                    .desired_width(180.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {} QSO",
                        tr("log.total_qsos", lang),
                        app.recent_qsos.len()
                    ))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(148, 163, 184)),
                );
            });
        });
        ui.separator();
        render_logbook_body(app, ui);
    });
    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "logbook_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.panel_log.floating,
            true,
        );
        if app.panel_log.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_log.saved_pos != Some(new_pos)
                || app.panel_log.saved_size != Some(new_size)
            {
                app.panel_log.saved_pos = Some(new_pos);
                app.panel_log.saved_size = Some(new_size);
            }
            if res.response.drag_stopped() {
                app.save_station_config();
            }
        }
    }
    if !open {
        app.panel_log.visible = false;
        app.save_station_config();
    }
}

pub fn col_display_name(
    col_id: &str,
    default_label: &str,
    lang: crate::core::i18n::Language,
) -> String {
    match col_id {
        "nr" => "Nr".to_string(),
        "date" => tr("qso.date", lang).to_string(),
        "time" => tr("qso.time", lang).to_string(),
        "callsign" => tr("qso.callsign", lang).to_string(),
        "band" => tr("qso.band", lang).to_string(),
        "mode" => tr("qso.mode", lang).to_string(),
        "rst_s" => tr("qso.rst_sent", lang).to_string(),
        "rst_r" => tr("qso.rst_rcvd", lang).to_string(),
        "country" => tr("geo.country", lang).to_string(),
        "name" => tr("qso.name", lang).to_string(),
        "qsl" => "QSL".to_string(),
        "freq" => tr("qso.freq", lang).to_string(),
        "cqz" => tr("geo.cq_zone", lang).to_string(),
        "iota" => tr("qso.iota", lang).to_string(),
        "comment" => tr("qso.comment", lang).to_string(),
        _ => default_label.to_string(),
    }
}

fn sort_header_btn(
    ui: &mut egui::Ui,
    label: &str,
    col_id: u8,
    sort_col: u8,
    sort_asc: bool,
) -> (bool, bool) {
    let arrow = if sort_col == col_id {
        if sort_asc { " ▲" } else { " ▼" }
    } else {
        ""
    };
    let clicked = ui
        .button(egui::RichText::new(format!("{label}{arrow}")).strong())
        .clicked();
    if clicked {
        if sort_col == col_id {
            (true, !sort_asc)
        } else {
            (true, false)
        }
    } else {
        (false, sort_asc)
    }
}

fn sort_col_id_for(col_id: &str) -> Option<u8> {
    match col_id {
        "date" => Some(0),
        "callsign" => Some(1),
        "band" => Some(2),
        "mode" => Some(3),
        "country" => Some(4),
        _ => None,
    }
}

/// Bezalokacyjne sprawdzenie zawierania podciągu bez rozróżniania wielkości liter (ASCII + pełny fallback UTF-8).
#[inline]
fn contains_case_insensitive(haystack: &str, needle_upper: &str) -> bool {
    if needle_upper.is_empty() {
        return true;
    }
    if haystack.len() < needle_upper.len() {
        return false;
    }
    if haystack.is_ascii() && needle_upper.is_ascii() {
        let h = haystack.as_bytes();
        let n = needle_upper.as_bytes();
        return h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n));
    }

    // Bezalokacyjny fallback Unicode: okno przesuwne porównuje znaki przez ich
    // rozwinięcie do wielkich liter (char::to_uppercase nie alokuje na stercie).
    for (start, _) in haystack.char_indices() {
        let mut h_chars = haystack[start..].chars();
        let mut n_chars = needle_upper.chars();
        let mut matched = true;
        loop {
            match n_chars.next() {
                None => break,
                Some(nc) => {
                    if let Some(hc) = h_chars.next() {
                        if !hc.to_uppercase().eq(nc.to_uppercase()) {
                            matched = false;
                            break;
                        }
                    } else {
                        matched = false;
                        break;
                    }
                }
            }
        }
        if matched {
            return true;
        }
    }
    false
}

/// Normalizuje ciąg daty/czasu na stosie (pomija `-`, `:`, `.`, `/`) i dopełnia zerami z prawej strony do `N` znaków.
#[inline]
fn normalize_digits_stack<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [b'0'; N];
    let mut idx = 0;
    for &b in s.as_bytes() {
        if b.is_ascii_digit() {
            if idx < N {
                out[idx] = b;
                idx += 1;
            } else {
                break;
            }
        }
    }
    out
}

pub fn render_logbook_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    let query = app.log_search_query.trim().to_uppercase();

    // Filtrowanie (wyszukiwarka tekstowa + filtr drill-down ze statystyk) — bez alokacji na stercie dla ASCII
    let drill = app.log_drill_filter.clone();
    let mut sorted_indices: Vec<usize> = (0..app.recent_qsos.len())
        .filter(|&i| {
            let q = &app.recent_qsos[i];
            if let Some(ref d) = drill {
                if !d.matches(q) {
                    return false;
                }
            }
            if query.is_empty() {
                return true;
            }
            contains_case_insensitive(&q.callsign, &query)
                || contains_case_insensitive(&q.band, &query)
                || contains_case_insensitive(&q.mode, &query)
                || contains_case_insensitive(q.country.as_deref().unwrap_or(""), &query)
                || contains_case_insensitive(q.name.as_deref().unwrap_or(""), &query)
                || contains_case_insensitive(q.qth.as_deref().unwrap_or(""), &query)
                || contains_case_insensitive(q.gridsquare.as_deref().unwrap_or(""), &query)
                || contains_case_insensitive(q.comment.as_deref().unwrap_or(""), &query)
        })
        .collect();

    // Sortowanie (bezalokacyjne klucze na stosie dla daty i czasu)
    let sc = app.log_sort_column;
    let sa = app.log_sort_asc;
    sorted_indices.sort_by(|&a, &b| {
        let qa = &app.recent_qsos[a];
        let qb = &app.recent_qsos[b];
        let ord = match sc {
            1 => qa.callsign.cmp(&qb.callsign),
            2 => qa.band.cmp(&qb.band),
            3 => qa.mode.cmp(&qb.mode),
            4 => qa
                .country
                .as_deref()
                .unwrap_or("")
                .cmp(qb.country.as_deref().unwrap_or("")),
            _ => {
                let da = normalize_digits_stack::<8>(&qa.qso_date);
                let db = normalize_digits_stack::<8>(&qb.qso_date);
                da.cmp(&db)
                    .then_with(|| {
                        let ta = normalize_digits_stack::<6>(&qa.time_on);
                        let tb = normalize_digits_stack::<6>(&qb.time_on);
                        ta.cmp(&tb)
                    })
                    .then_with(|| qa.id.cmp(&qb.id))
            }
        };
        if sa { ord } else { ord.reverse() }
    });

    let total = sorted_indices.len();

    // Aktywny filtr drill-down ze statystyk — widoczny pasek z przyciskiem czyszczenia
    if let Some(ref d) = app.log_drill_filter {
        let label = d.label();
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "🔎 {}: {}",
                    tr("stats.drilldown_filter", lang),
                    label
                ))
                .size(12.0)
                .color(egui::Color32::from_rgb(56, 189, 248)),
            );
            if ui
                .small_button("✕")
                .on_hover_text(tr("stats.drilldown_clear", lang))
                .clicked()
            {
                clear = true;
            }
        });
        ui.add_space(2.0);
        if clear {
            app.clear_drill_down();
        }
    }

    // Pasek paginacji
    ui.horizontal(|ui| {
        ui.label(tr("log.rows_per_page", lang));
        for &size in &[25usize, 50, 100, 0usize] {
            let lbl = if size == 0 {
                tr("log.all_rows", lang).to_string()
            } else {
                size.to_string()
            };
            if ui
                .selectable_label(app.log_page_size == size, &lbl)
                .clicked()
            {
                app.log_page_size = size;
                app.log_page = 0;
            }
        }
        ui.separator();
        let (start, end) = page_range(app.log_page, app.log_page_size, total);
        if total > 0 {
            ui.label(format!("{}-{} / {}", start + 1, end, total));
        } else {
            ui.label(tr("log.no_results", lang));
        }
        if app.log_page_size > 0 {
            let pages = (total + app.log_page_size - 1) / app.log_page_size.max(1);
            if ui
                .add_enabled(app.log_page > 0, egui::Button::new("◀"))
                .clicked()
            {
                app.log_page = app.log_page.saturating_sub(1);
            }
            ui.label(format!("{}/{}", app.log_page + 1, pages.max(1)));
            if ui
                .add_enabled(end < total, egui::Button::new("▶"))
                .clicked()
            {
                app.log_page += 1;
            }
        }
        ui.separator();
        if ui
            .add_enabled(!app.undo_stack.is_empty(), egui::Button::new("↩ Undo"))
            .on_hover_text(tr("log.undo_tooltip", lang))
            .clicked()
        {
            app.perform_undo();
        }
        if ui
            .add_enabled(!app.redo_stack.is_empty(), egui::Button::new("↪ Redo"))
            .clicked()
        {
            app.perform_redo();
        }
    });
    ui.separator();

    // Pasek operacji masowych (widoczny, gdy cokolwiek zaznaczono)
    if !app.selected_qso_ids.is_empty() {
        ui.horizontal(|ui| {
            let n = app.selected_qso_ids.len();
            ui.label(
                egui::RichText::new(format!("✔ Zaznaczono {n} QSO"))
                    .color(egui::Color32::from_rgb(34, 197, 94)),
            );
            if ui.button(format!("🗑 {}", tr("qso.delete", lang))).clicked() {
                app.delete_selected_qso();
            }
            if ui
                .button("✕")
                .on_hover_text("Wyczyść zaznaczenie")
                .clicked()
            {
                app.selected_qso_ids.clear();
            }
        });
        ui.separator();
    }

    // Wyznacz zakres strony
    let (page_start, page_end) = page_range(app.log_page, app.log_page_size, total);
    let page_indices = &sorted_indices[page_start..page_end];

    let mut qso_to_edit: Option<crate::core::qso::QsoRecord> = None;
    let mut qso_id_to_delete: Option<(i64, crate::core::qso::QsoRecord)> = None;
    let mut new_sc = app.log_sort_column;
    let mut new_sa = app.log_sort_asc;

    // Snapshot widocznych kolumn (id, wyświetlana etykieta, szerokość)
    let visible_cols: Vec<(String, String, f32)> = app
        .logbook_columns
        .iter()
        .filter(|c| c.visible)
        .map(|c| {
            (
                c.id.clone(),
                col_display_name(&c.id, &c.label, lang),
                c.width,
            )
        })
        .collect();

    // Identyfikatory QSO na bieżącej stronie (do zaznaczania zbiorowego)
    let page_ids: Vec<i64> = page_indices
        .iter()
        .filter_map(|&i| app.recent_qsos[i].id)
        .collect();
    let all_selected =
        !page_ids.is_empty() && page_ids.iter().all(|id| app.selected_qso_ids.contains(id));
    let mut select_all = all_selected;

    let row_height = 22.0;

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(72.0))
        .column(Column::auto().at_least(24.0));
    for (_id, _label, width) in &visible_cols {
        table = table.column(
            Column::initial(*width)
                .at_least(28.0)
                .clip(true)
                .resizable(true),
        );
    }

    table
        .header(22.0, |mut header| {
            header.col(|ui| {
                ui.label(egui::RichText::new(tr("ledger.col_actions", lang)).strong());
            });
            header.col(|ui| {
                ui.checkbox(&mut select_all, "");
            });
            for (id, label, _w) in &visible_cols {
                let label = label.clone();
                let id = id.clone();
                header.col(|ui| {
                    if let Some(sc_id) = sort_col_id_for(&id) {
                        let (c, s) = sort_header_btn(ui, &label, sc_id, new_sc, new_sa);
                        if c {
                            new_sc = sc_id;
                            new_sa = s;
                        }
                    } else {
                        ui.label(egui::RichText::new(&label).strong());
                    }
                });
            }
        })
        .body(|body| {
            body.rows(row_height, page_indices.len(), |mut row| {
                let idx = page_indices[row.index()];
                let qso = app.recent_qsos[idx].clone();
                let qso_id = qso.id;

                row.col(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("✏")
                            .on_hover_text(tr("log.edit_tooltip", lang))
                            .clicked()
                        {
                            qso_to_edit = Some(qso.clone());
                        }
                        if ui
                            .small_button("🗑")
                            .on_hover_text(tr("log.delete_tooltip", lang))
                            .clicked()
                        {
                            if let Some(id) = qso_id {
                                qso_id_to_delete = Some((id, qso.clone()));
                            }
                        }
                        if let Some(ref audio_path) = qso.audio_file {
                            if ui.small_button("🔊").clicked() {
                                let _ = crate::media::audio_recorder::AudioRecorder::play_audio(
                                    std::path::Path::new(audio_path),
                                );
                            }
                        }
                    });
                });

                row.col(|ui| {
                    if let Some(id) = qso_id {
                        let mut checked = app.selected_qso_ids.contains(&id);
                        if ui.checkbox(&mut checked, "").changed() {
                            if checked {
                                if !app.selected_qso_ids.contains(&id) {
                                    app.selected_qso_ids.push(id);
                                }
                            } else {
                                app.selected_qso_ids.retain(|x| *x != id);
                            }
                        }
                    }
                });

                for (col_id, _label, _w) in &visible_cols {
                    let col_id = col_id.clone();
                    row.col(|ui| match col_id.as_str() {
                        "nr" => {
                            ui.label(
                                qso.id
                                    .and_then(|id| app.qso_numbers.get(&id).copied())
                                    .map_or_else(|| "—".to_string(), |number| number.to_string()),
                            );
                        }
                        "date" => {
                            ui.label(&qso.qso_date);
                        }
                        "time" => {
                            ui.label(&qso.time_on);
                        }
                        "callsign" => {
                            let call_color = if qso.lotw_qsl_rcvd == "Y" {
                                egui::Color32::from_rgb(34, 197, 94)
                            } else {
                                egui::Color32::from_rgb(56, 189, 248)
                            };
                            ui.label(
                                egui::RichText::new(&qso.callsign)
                                    .strong()
                                    .color(call_color),
                            );
                        }
                        "band" => {
                            ui.label(&qso.band);
                        }
                        "mode" => {
                            ui.label(
                                egui::RichText::new(&qso.mode)
                                    .color(egui::Color32::from_rgb(251, 191, 36)),
                            );
                        }
                        "rst_s" => {
                            ui.label(&qso.rst_sent);
                        }
                        "rst_r" => {
                            ui.label(&qso.rst_rcvd);
                        }
                        "country" => {
                            ui.label(qso.country.as_deref().unwrap_or("-"));
                        }
                        "name" => {
                            ui.label(qso.name.as_deref().unwrap_or(""));
                        }
                        "qsl" => {
                            ui.horizontal(|ui| {
                                if qso.lotw_qsl_rcvd == "Y" {
                                    ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "L");
                                }
                                if qso.eqsl_qsl_rcvd == "Y" {
                                    ui.colored_label(egui::Color32::from_rgb(56, 189, 248), "E");
                                }
                                if qso.qsl_rcvd == "Y" {
                                    ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "Q");
                                }
                                if qso.lotw_qsl_rcvd != "Y"
                                    && qso.eqsl_qsl_rcvd != "Y"
                                    && qso.qsl_rcvd != "Y"
                                {
                                    ui.label("-");
                                }
                            });
                        }
                        "freq" => {
                            let freq_str = qso
                                .freq
                                .as_ref()
                                .map(std::string::ToString::to_string)
                                .unwrap_or_default();
                            ui.label(freq_str);
                        }
                        "cqz" => {
                            let cqz_str = qso
                                .cqz
                                .as_ref()
                                .map(std::string::ToString::to_string)
                                .unwrap_or_default();
                            ui.label(cqz_str);
                        }
                        "iota" => {
                            ui.label(qso.iota.as_deref().unwrap_or(""));
                        }
                        "comment" => {
                            ui.label(qso.comment.as_deref().unwrap_or(""));
                        }
                        _ => {
                            ui.label("");
                        }
                    });
                }
            });
        });

    // Zastosuj nowe sortowanie
    app.log_sort_column = new_sc;
    app.log_sort_asc = new_sa;

    // Zastosuj przełączenie zaznaczenia całej strony
    if select_all != all_selected {
        if select_all {
            for id in &page_ids {
                if !app.selected_qso_ids.contains(id) {
                    app.selected_qso_ids.push(*id);
                }
            }
        } else {
            app.selected_qso_ids.retain(|id| !page_ids.contains(id));
        }
    }

    if let Some(qso) = qso_to_edit {
        app.editing_qso = Some(qso);
    }
    if let Some((id, backup)) = qso_id_to_delete {
        if app.undo_stack.len() >= 20 {
            app.undo_stack.pop_back();
        }
        app.undo_stack.push_front(backup);
        app.redo_stack.clear();
        app.delete_qso_by_id(id);
    }
}

fn page_range(page: usize, page_size: usize, total: usize) -> (usize, usize) {
    if page_size == 0 {
        return (0, total);
    }
    let start = page * page_size;
    let end = (start + page_size).min(total);
    (start, end)
}

/// Okno dialogowe edycji rekordu QSO
pub fn render_edit_qso_dialog(app: &mut SpLogApp, ctx: &egui::Context) {
    if app.editing_qso.is_none() {
        return;
    }
    let lang = app.current_language;
    let mut is_open = true;
    let mut should_save = false;
    let mut should_cancel = false;

    if let Some(ref mut qso) = app.editing_qso {
        egui::Window::new(format!(
            "✏ {} - {}",
            tr("qso.edit_title", lang),
            qso.callsign
        ))
        .open(&mut is_open)
        .default_size([460.0, 520.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(tr("qso.callsign", lang)).strong());
                        ui.add(egui::TextEdit::singleline(&mut qso.callsign).desired_width(120.0));
                        ui.label(tr("qso.band", lang));
                        ui.add(egui::TextEdit::singleline(&mut qso.band).desired_width(60.0));
                        ui.label(tr("qso.mode", lang));
                        ui.add(egui::TextEdit::singleline(&mut qso.mode).desired_width(60.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("qso.date", lang));
                        ui.add(egui::TextEdit::singleline(&mut qso.qso_date).desired_width(90.0));
                        ui.label(tr("qso.time", lang));
                        ui.add(egui::TextEdit::singleline(&mut qso.time_on).desired_width(70.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("qso.rst_sent", lang));
                        ui.add(egui::TextEdit::singleline(&mut qso.rst_sent).desired_width(60.0));
                        ui.label(tr("qso.rst_rcvd", lang));
                        ui.add(egui::TextEdit::singleline(&mut qso.rst_rcvd).desired_width(60.0));
                    });
                });
                ui.add_space(6.0);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("qso.name", lang));
                        let mut v = qso.name.clone().unwrap_or_default();
                        if ui
                            .add(egui::TextEdit::singleline(&mut v).desired_width(130.0))
                            .changed()
                        {
                            qso.name = if v.is_empty() { None } else { Some(v) };
                        }
                        ui.label(tr("qso.locator", lang));
                        let mut v = qso.gridsquare.clone().unwrap_or_default();
                        if ui
                            .add(egui::TextEdit::singleline(&mut v).desired_width(80.0))
                            .changed()
                        {
                            qso.gridsquare = if v.is_empty() {
                                None
                            } else {
                                Some(v.to_uppercase())
                            };
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("qso.qth", lang));
                        let mut v = qso.qth.clone().unwrap_or_default();
                        if ui
                            .add(egui::TextEdit::singleline(&mut v).desired_width(140.0))
                            .changed()
                        {
                            qso.qth = if v.is_empty() { None } else { Some(v) };
                        }
                        ui.label("PGA:");
                        let mut v = qso.pga_ref.clone().unwrap_or_default();
                        if ui
                            .add(egui::TextEdit::singleline(&mut v).desired_width(70.0))
                            .changed()
                        {
                            qso.pga_ref = if v.is_empty() {
                                None
                            } else {
                                Some(v.to_uppercase())
                            };
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("qso.comment", lang));
                        let mut v = qso.comment.clone().unwrap_or_default();
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut v)
                                    .desired_width(ui.available_width()),
                            )
                            .changed()
                        {
                            qso.comment = if v.is_empty() { None } else { Some(v) };
                        }
                    });
                });
                ui.add_space(6.0);
                ui.group(|ui| {
                    ui.label(egui::RichText::new(tr("qso.qsl_status_heading", lang)).strong());
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} {}:",
                            tr("stats.paper_qsl_label", lang),
                            tr("qso.paper_sent", lang)
                        ));
                        ui.add(egui::TextEdit::singleline(&mut qso.qsl_sent).desired_width(30.0));
                        ui.label(format!("{}:", tr("qso.received", lang)));
                        ui.add(egui::TextEdit::singleline(&mut qso.qsl_rcvd).desired_width(30.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("LoTW {}:", tr("qso.lotw_sent", lang)));
                        ui.add(
                            egui::TextEdit::singleline(&mut qso.lotw_qsl_sent).desired_width(30.0),
                        );
                        ui.label(format!("{}:", tr("qso.received", lang)));
                        ui.add(
                            egui::TextEdit::singleline(&mut qso.lotw_qsl_rcvd).desired_width(30.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("eQSL {}:", tr("qso.eqsl_sent", lang)));
                        ui.add(
                            egui::TextEdit::singleline(&mut qso.eqsl_qsl_sent).desired_width(30.0),
                        );
                        ui.label(format!("{}:", tr("qso.received", lang)));
                        ui.add(
                            egui::TextEdit::singleline(&mut qso.eqsl_qsl_rcvd).desired_width(30.0),
                        );
                    });
                });
                ui.horizontal(|ui| {
                    let save_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("💾 {}", tr("btn.save", lang)))
                                .strong()
                                .color(egui::Color32::from_rgb(15, 23, 42)),
                        )
                        .fill(egui::Color32::from_rgb(56, 189, 248)),
                    );
                    if save_btn.clicked() {
                        should_save = true;
                    }
                    if ui.button(tr("btn.cancel", lang)).clicked() {
                        should_cancel = true;
                    }
                });
            });
        });
    }

    if !is_open || should_cancel {
        app.editing_qso = None;
    } else if should_save {
        app.save_edited_qso();
    }
}

pub fn render_column_settings(app: &mut SpLogApp, ctx: &egui::Context) {
    let mut is_open = app.show_column_settings;
    if !is_open {
        return;
    }
    let lang = app.current_language;

    let mut save = false;
    let mut reset = false;
    let mut save_preset = false;
    let mut load_preset: Option<usize> = None;
    let mut delete_preset: Option<usize> = None;

    egui::Window::new(format!("⚙ {}", tr("columns.title", lang)))
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(tr("columns.subtitle", lang));
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    let mut move_up = None;
                    let mut move_down = None;

                    for (i, col) in app.logbook_columns.iter_mut().enumerate() {
                        let display_lbl = col_display_name(&col.id, &col.label, lang);
                        ui.horizontal(|ui| {
                            if ui.button("⬆").clicked() {
                                move_up = Some(i);
                            }
                            if ui.button("⬇").clicked() {
                                move_down = Some(i);
                            }
                            ui.checkbox(&mut col.visible, display_lbl);
                        });
                    }

                    if let Some(i) = move_up {
                        if i > 0 {
                            app.logbook_columns.swap(i, i - 1);
                        }
                    }
                    if let Some(i) = move_down {
                        if i + 1 < app.logbook_columns.len() {
                            app.logbook_columns.swap(i, i + 1);
                        }
                    }
                });

            ui.separator();
            ui.label(egui::RichText::new("Presety kolumn").strong());
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.column_preset_name);
                if ui.button("💾 Zapisz").clicked() {
                    save_preset = true;
                }
            });
            for (i, preset) in app.logbook_column_presets.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.button("↩").on_hover_text("Wczytaj preset").clicked() {
                        load_preset = Some(i);
                    }
                    ui.label(&preset.name);
                    if ui.small_button("🗑").on_hover_text("Usuń preset").clicked() {
                        delete_preset = Some(i);
                    }
                });
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr("btn.save", lang)).clicked() {
                    save = true;
                }
                if ui.button(tr("btn.close", lang)).clicked() {
                    app.show_column_settings = false;
                }
                if ui.button(tr("toolbar.restore_defaults", lang)).clicked() {
                    reset = true;
                }
            });
        });

    if reset {
        app.logbook_columns = crate::core::station::default_logbook_columns();
        app.save_station_config();
    } else if save || !is_open {
        app.show_column_settings = false;
        app.save_station_config();
    }

    if save_preset {
        let name = app.column_preset_name.trim().to_string();
        if !name.is_empty() {
            let preset = crate::core::station::ColumnPreset {
                name: name.clone(),
                columns: app.logbook_columns.clone(),
            };
            app.logbook_column_presets.retain(|p| p.name != name);
            app.logbook_column_presets.push(preset);
            app.column_preset_name.clear();
            app.save_station_config();
        }
    }

    if let Some(i) = load_preset {
        if let Some(preset) = app.logbook_column_presets.get(i) {
            app.logbook_columns = preset.columns.clone();
            app.save_station_config();
        }
    }

    if let Some(i) = delete_preset {
        if i < app.logbook_column_presets.len() {
            app.logbook_column_presets.remove(i);
            app.save_station_config();
        }
    }
}
