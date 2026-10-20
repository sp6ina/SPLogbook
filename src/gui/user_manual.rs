// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wbudowany podręcznik użytkownika (Instrukcja obsługi).

pub use crate::core::i18n::ManualSection;
use crate::core::i18n::{key, manual, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::{ColorExt, ThemeExt};
use eframe::egui;

/// Kontekstowa ikona pomocy „?” — otwiera podręcznik na wskazanej sekcji.
pub fn help_button(app: &mut SpLogApp, ui: &mut egui::Ui, section: &'static str) {
    let tip = tr(key::MANUAL_HELP_BUTTON_TOOLTIP, app.current_language);
    if ui.small_button("?").on_hover_text(tip).clicked() {
        app.show_user_manual = true;
        app.manual_section = Some(section.to_string());
    }
}

fn section_by_id(sections: &[ManualSection], id: &str) -> usize {
    sections.iter().position(|s| s.id == id).unwrap_or(0)
}

/// Okno „Instrukcja obsługi” — panel boczny z sekcjami + przewijana, czytelnie sformatowana treść.
pub fn render_user_manual_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_user_manual {
        return;
    }
    let lang = app.current_language;
    let sections = manual(lang);
    let mut is_open = app.show_user_manual;

    let selected = if let Some(section) = app.manual_section.take() {
        section_by_id(sections, &section)
    } else {
        app.manual_selected.min(sections.len().saturating_sub(1))
    };

    let mut new_selected = selected;
    let mut close_requested = false;

    let screen = ctx.content_rect();
    let max_w = (screen.width() - 32.0).clamp(640.0, 940.0);
    let max_h = (screen.height() - 48.0).clamp(420.0, 680.0);
    let default_w = 860.0_f32.min(max_w);
    let default_h = 560.0_f32.min(max_h);

    let win_title = tr(key::MANUAL_WINDOW_TITLE, lang);

    let search_id = egui::Id::new("user_manual_search_filter");
    let mut search_query: String = ctx.data_mut(|d| d.get_temp(search_id).unwrap_or_default());

    egui::Window::new(win_title)
        .id(egui::Id::new("splogbook_user_manual_window"))
        .open(&mut is_open)
        .default_size([default_w, default_h])
        .min_size([620.0, 380.0])
        .max_size([max_w, max_h])
        .constrain_to(screen)
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.set_max_width(max_w - 20.0);

            // Górny pasek nawigacji i wyszukiwarki
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍").size(13.0).color(palette.accent));
                let hint = tr(key::MANUAL_SEARCH_HINT, lang);
                ui.add(
                    egui::TextEdit::singleline(&mut search_query)
                        .hint_text(hint)
                        .desired_width(260.0),
                );
                if !search_query.is_empty()
                    && ui
                        .small_button("✕")
                        .on_hover_text(tr(key::BTN_CLEAR, lang))
                        .clicked()
                {
                    search_query.clear();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let next_label = tr(key::MANUAL_BTN_NEXT, lang);
                    if ui
                        .add_enabled(
                            new_selected + 1 < sections.len(),
                            egui::Button::new(next_label),
                        )
                        .clicked()
                    {
                        new_selected = (new_selected + 1).min(sections.len() - 1);
                    }
                    ui.label(
                        egui::RichText::new(format!("{} / {}", new_selected + 1, sections.len()))
                            .size(11.5)
                            .color(palette.text_muted),
                    );
                    let prev_label = tr(key::MANUAL_BTN_PREV, lang);
                    if ui
                        .add_enabled(new_selected > 0, egui::Button::new(prev_label))
                        .clicked()
                    {
                        new_selected = new_selected.saturating_sub(1);
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let body_height = (ui.available_height() - 44.0).clamp(260.0, 540.0);
            let q_lower = search_query.trim().to_lowercase();

            ui.horizontal(|ui| {
                // Lewy panel boczny: spis treści w układzie pionowym
                egui::Frame::NONE
                    .fill(palette.surface_bg.alpha(180))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(6))
                    .stroke(egui::Stroke::new(1.0, palette.border_subtle))
                    .show(ui, |ui| {
                        ui.set_width(235.0);
                        ui.set_height(body_height);
                        egui::ScrollArea::vertical()
                            .id_salt("manual_toc_scroll")
                            .max_height(body_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(220.0);
                                    for (i, sec) in sections.iter().enumerate() {
                                        if !q_lower.is_empty()
                                            && !sec.title.to_lowercase().contains(&q_lower)
                                            && !sec.body.to_lowercase().contains(&q_lower)
                                        {
                                            continue;
                                        }
                                        let is_sel = new_selected == i;
                                        let text = egui::RichText::new(sec.title)
                                            .size(12.5)
                                            .color(if is_sel {
                                                palette.accent
                                            } else {
                                                palette.text_primary
                                            })
                                            .strong();
                                        let resp = ui.add_sized(
                                            [218.0, 26.0],
                                            egui::Button::selectable(is_sel, text),
                                        );
                                        if resp.clicked() {
                                            new_selected = i;
                                        }
                                    }
                                });
                            });
                    });

                ui.add_space(6.0);

                // Prawy panel: treść rozdziału z zawijaniem wierszy w układzie pionowym
                let content_width = (ui.available_width() - 4.0).max(340.0);
                egui::Frame::NONE
                    .fill(palette.subtle_bg.alpha(140))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(12))
                    .stroke(egui::Stroke::new(1.0, palette.border_subtle))
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.set_height(body_height);
                        egui::ScrollArea::vertical()
                            .id_salt("manual_content_scroll")
                            .max_height(body_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let inner_w = (content_width - 28.0).max(300.0);
                                    ui.set_max_width(inner_w);

                                    let sec = &sections[new_selected.min(sections.len() - 1)];
                                    ui.label(
                                        egui::RichText::new(sec.title)
                                            .size(17.5)
                                            .strong()
                                            .color(palette.accent),
                                    );
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(8.0);

                                    for paragraph in sec.body.split("\n\n") {
                                        for line in paragraph.lines() {
                                            let trimmed = line.trim();
                                            if trimmed.is_empty() {
                                                continue;
                                            }
                                            if let Some(rest) = trimmed.strip_prefix("- ") {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.set_max_width(inner_w);
                                                    ui.label(
                                                        egui::RichText::new("•")
                                                            .strong()
                                                            .color(palette.accent),
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(rest).size(13.0),
                                                        )
                                                        .wrap(),
                                                    );
                                                });
                                            } else if let Some(rest) = trimmed.strip_prefix("* ") {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.set_max_width(inner_w);
                                                    ui.add_space(12.0);
                                                    ui.label(
                                                        egui::RichText::new("◦")
                                                            .color(palette.success),
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(rest).size(12.5),
                                                        )
                                                        .wrap(),
                                                    );
                                                });
                                            } else {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(trimmed).size(13.0),
                                                    )
                                                    .wrap(),
                                                );
                                            }
                                            ui.add_space(3.0);
                                        }
                                        ui.add_space(8.0);
                                    }
                                });
                            });
                    });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                let open_dir_label = tr(key::MANUAL_BTN_OPEN_DATA_DIR, lang);
                let open_dir_tip = tr(key::MANUAL_OPEN_DATA_DIR_TOOLTIP, lang);
                if ui
                    .button(open_dir_label)
                    .on_hover_text(open_dir_tip)
                    .clicked()
                {
                    if let Some(parent) = app.config_file_path.parent() {
                        let _ = open::that(parent);
                    }
                }
                let issue_label = tr(key::MANUAL_BTN_REPORT_ISSUE, lang);
                if ui.button(issue_label).clicked() {
                    let _ = open::that("https://github.com/sp6ina/SPLogbook/issues/new");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_label = tr(key::MANUAL_BTN_CLOSE_ESC, lang);
                    if ui.button(close_label).clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    ctx.data_mut(|d| d.insert_temp(search_id, search_query));
    app.manual_selected = new_selected;
    app.show_user_manual = is_open && !close_requested;
}
