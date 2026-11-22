// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno dekodów WSJT-X / JTDX: stan połączenia, lista dekodów z oceną stacji,
//! odpowiedź na CQ i ustawienia nasłuchu (STAGE-V140-S8).

use crate::core::i18n::{Language as Lang, Msg, key, tr};
use crate::digital::wsjtx::{WsjtxMark, WsjtxSettings};
use crate::gui::app::SpLogApp;
use crate::gui::state::digital::WsjtxLinkState;
use crate::gui::theme::{Palette, ThemeExt};
use eframe::egui;

/// Akcja wybrana w oknie, wykonywana po jego narysowaniu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsjtxWindowAction {
    Reply(usize),
    ClearDecodes,
    ApplySettings(WsjtxSettings),
}

fn mark_label(mark: WsjtxMark, lang: Lang) -> &'static str {
    match mark {
        WsjtxMark::NewDxcc => tr(key::WSJTX_MARK_NEW_DXCC, lang),
        WsjtxMark::NewBand => tr(key::WSJTX_MARK_NEW_BAND, lang),
        WsjtxMark::WorkedBefore => tr(key::WSJTX_MARK_WORKED, lang),
        WsjtxMark::Other => "",
    }
}

fn mark_color(mark: WsjtxMark, palette: &Palette) -> egui::Color32 {
    match mark {
        WsjtxMark::NewDxcc => palette.danger,
        WsjtxMark::NewBand => palette.warning,
        WsjtxMark::WorkedBefore => palette.text_muted,
        WsjtxMark::Other => palette.text_primary,
    }
}

fn render_link_status(ui: &mut egui::Ui, state: &WsjtxLinkState, port: u16, lang: Lang) {
    let palette = ui.palette();
    if !state.is_listening() {
        ui.colored_label(palette.text_muted, tr(key::WSJTX_NOT_LISTENING, lang));
        return;
    }
    ui.colored_label(
        palette.success,
        Msg::new(key::WSJTX_LISTENING).arg("port", port).text(lang),
    );
    if state.clients.is_empty() {
        ui.colored_label(palette.text_muted, tr(key::WSJTX_NO_CLIENTS, lang));
        return;
    }
    let list = state
        .clients
        .values()
        .map(|status| {
            let mut parts = vec![status.id.as_str()];
            if !status.mode.is_empty() {
                parts.push(&status.mode);
            }
            let name = parts.join(" ");
            if status.transmitting {
                format!("{name} · {}", tr(key::WSJTX_TRANSMITTING, lang))
            } else if status.decoding {
                format!("{name} · {}", tr(key::WSJTX_DECODING, lang))
            } else {
                name
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    ui.label(Msg::new(key::WSJTX_CLIENTS).arg("list", list).text(lang));
}

/// Ustawienia nasłuchu; zwraca `true` po kliknięciu „Zastosuj”.
fn render_settings(ui: &mut egui::Ui, draft: &mut WsjtxSettings, lang: Lang) -> bool {
    ui.checkbox(&mut draft.enabled, tr(key::WSJTX_ENABLED, lang));
    egui::Grid::new("wsjtx_settings_grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(tr(key::WSJTX_PORT, lang));
            ui.add(egui::DragValue::new(&mut draft.port).range(1024..=65535));
            ui.end_row();
            ui.label(tr(key::WSJTX_MULTICAST, lang));
            ui.add(
                egui::TextEdit::singleline(&mut draft.multicast_group)
                    .hint_text(tr(key::WSJTX_MULTICAST_HINT, lang))
                    .desired_width(160.0),
            );
            ui.end_row();
        });
    ui.checkbox(&mut draft.highlight, tr(key::WSJTX_HIGHLIGHT, lang));
    ui.button(tr(key::WSJTX_APPLY, lang)).clicked()
}

fn render_decodes(
    ui: &mut egui::Ui,
    state: &WsjtxLinkState,
    lang: Lang,
    actions: &mut Vec<WsjtxWindowAction>,
) {
    let palette = ui.palette();
    let rows: Vec<(usize, &crate::gui::state::digital::WsjtxDecodeRow)> = state
        .decodes
        .iter()
        .enumerate()
        .filter(|(_, row)| !state.only_cq || row.decode.cq_caller().is_some())
        .collect();
    if rows.is_empty() {
        ui.colored_label(palette.text_muted, tr(key::WSJTX_NO_DECODES, lang));
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("wsjtx_decodes_grid")
                .striped(true)
                .num_columns(7)
                .show(ui, |ui| {
                    for header in ["UTC", "SNR", "DT", "DF"] {
                        ui.strong(header);
                    }
                    ui.strong(tr(key::WSJTX_COL_MESSAGE, lang));
                    ui.strong(tr(key::WSJTX_COL_STATUS, lang));
                    ui.label("");
                    ui.end_row();
                    for (index, row) in rows {
                        let color = mark_color(row.mark, &palette);
                        let d = &row.decode;
                        ui.monospace(d.time_hhmmss());
                        ui.monospace(format!("{:+}", d.snr));
                        ui.monospace(format!("{:.1}", d.delta_time));
                        ui.monospace(d.delta_freq.to_string());
                        ui.label(egui::RichText::new(&d.message).monospace().color(color));
                        ui.colored_label(color, mark_label(row.mark, lang));
                        if d.cq_caller().is_some() {
                            if ui
                                .small_button(tr(key::WSJTX_REPLY, lang))
                                .on_hover_text(tr(key::WSJTX_REPLY_HINT, lang))
                                .clicked()
                            {
                                actions.push(WsjtxWindowAction::Reply(index));
                            }
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });
}

/// Rysuje okno i zwraca wybrane akcje.
pub fn render_wsjtx_contents(
    ui: &mut egui::Ui,
    state: &mut WsjtxLinkState,
    settings: &WsjtxSettings,
    lang: Lang,
) -> Vec<WsjtxWindowAction> {
    let mut actions = Vec::new();
    render_link_status(ui, state, settings.port, lang);
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.only_cq, tr(key::WSJTX_ONLY_CQ, lang));
        if ui.button(tr(key::BTN_CLEAR, lang)).clicked() {
            actions.push(WsjtxWindowAction::ClearDecodes);
        }
    });
    egui::CollapsingHeader::new(tr(key::WSJTX_SETTINGS, lang))
        .id_salt("wsjtx_settings")
        .show(ui, |ui| {
            let draft = state.draft.get_or_insert_with(|| settings.clone());
            if render_settings(ui, draft, lang) {
                actions.push(WsjtxWindowAction::ApplySettings(draft.clone()));
            }
        });
    ui.separator();
    render_decodes(ui, state, lang, &mut actions);
    actions
}

pub fn render_wsjtx_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.digital.wsjtx.show_window {
        return;
    }
    let lang = app.gui.appearance.language;
    let mut open = true;
    let mut actions = Vec::new();
    egui::Window::new(tr(key::WSJTX_WINDOW_TITLE, lang))
        .open(&mut open)
        .default_size([680.0, 440.0])
        .show(ctx, |ui| {
            actions = render_wsjtx_contents(ui, &mut app.digital.wsjtx, &app.config.wsjtx, lang);
        });
    if !open {
        app.digital.wsjtx.show_window = false;
        app.digital.wsjtx.draft = None;
    }
    for action in actions {
        match action {
            WsjtxWindowAction::Reply(index) => app.reply_to_wsjtx_decode(index),
            WsjtxWindowAction::ClearDecodes => app.digital.wsjtx.decodes.clear(),
            WsjtxWindowAction::ApplySettings(settings) => {
                app.digital.wsjtx.draft = None;
                app.apply_wsjtx_settings(settings);
            }
        }
    }
}
