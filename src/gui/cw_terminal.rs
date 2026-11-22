// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Terminal telegraficzny CW: wybór klucza, makra F1..F8, bufor nadawania i podgląd postępu.

use crate::cat::cw_keyer::{CwKeyerHandle, CwKeyerKind, CwKeyerSettings};
use crate::cat::winkeyer::CwTerminalBuffer;
use crate::core::i18n::{Language, key, tr};
use crate::gui::theme::ThemeExt;
use eframe::egui;

/// Działanie wybrane w terminalu; wykonuje je aplikacja.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwTerminalAction {
    Connect,
    Disconnect,
    Send(String),
    /// Makro F1..F8 (indeks od zera).
    Macro(usize),
    Abort,
    SetSpeed(u8),
    /// Zmieniono ustawienia klucza (do zapisania w konfiguracji).
    SettingsChanged,
}

pub struct CwTerminalDialog {
    pub is_open: bool,
    pub buffer: CwTerminalBuffer,
    pub input_text: String,
    pub status_text: String,
    /// Działający klucz; `None`, gdy klucz nie jest podłączony.
    pub keyer: Option<CwKeyerHandle>,
    /// Klucz potwierdził gotowość.
    pub ready: bool,
}

impl Default for CwTerminalDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            buffer: CwTerminalBuffer::new(24),
            input_text: String::new(),
            status_text: String::new(),
            keyer: None,
            ready: false,
        }
    }
}

pub fn keyer_kind_label(kind: CwKeyerKind, lang: Language) -> &'static str {
    match kind {
        CwKeyerKind::WinKeyer => "WinKeyer (K1EL)",
        CwKeyerKind::SerialDtr => tr(key::CW_TERM_KIND_DTR, lang),
        CwKeyerKind::SerialRts => tr(key::CW_TERM_KIND_RTS, lang),
        CwKeyerKind::Hamlib => tr(key::CW_TERM_KIND_HAMLIB, lang),
    }
}

/// Etykieta przycisku makra: numer klawisza i początek treści.
fn macro_button_label(index: usize, text: &str) -> String {
    let preview: String = text.chars().take(14).collect();
    let ellipsis = if text.chars().count() > 14 { "…" } else { "" };
    format!("F{}: {preview}{ellipsis}", index + 1)
}

impl CwTerminalDialog {
    pub fn is_connected(&self) -> bool {
        self.keyer.is_some()
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        settings: &mut CwKeyerSettings,
        macros: &[String],
        lang: Language,
    ) -> Vec<CwTerminalAction> {
        let mut actions = Vec::new();
        if !self.is_open {
            return actions;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            actions.push(CwTerminalAction::Abort);
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new(tr(key::CW_TERM_WINDOW_TITLE, lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                let connected = self.is_connected();

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!connected, |ui| {
                        egui::ComboBox::from_id_salt("cw_keyer_kind")
                            .selected_text(keyer_kind_label(settings.kind, lang))
                            .show_ui(ui, |ui| {
                                for kind in [
                                    CwKeyerKind::WinKeyer,
                                    CwKeyerKind::SerialDtr,
                                    CwKeyerKind::SerialRts,
                                    CwKeyerKind::Hamlib,
                                ] {
                                    if ui
                                        .selectable_value(
                                            &mut settings.kind,
                                            kind,
                                            keyer_kind_label(kind, lang),
                                        )
                                        .changed()
                                    {
                                        actions.push(CwTerminalAction::SettingsChanged);
                                    }
                                }
                            });
                        if settings.kind != CwKeyerKind::Hamlib {
                            ui.label(tr(key::CW_TERM_KEY_PORT, lang));
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut settings.port)
                                        .desired_width(110.0)
                                        .hint_text("COM3, /dev/ttyUSB0"),
                                )
                                .lost_focus()
                            {
                                actions.push(CwTerminalAction::SettingsChanged);
                            }
                        }
                    });

                    let label = if connected {
                        tr(key::CW_TERM_DISCONNECT, lang)
                    } else {
                        tr(key::CW_TERM_CONNECT, lang)
                    };
                    if ui.button(label).clicked() {
                        actions.push(if connected {
                            CwTerminalAction::Disconnect
                        } else {
                            CwTerminalAction::Connect
                        });
                    }
                    let (dot, color) = match (connected, self.ready) {
                        (true, true) => ("●", palette.success),
                        (true, false) => ("◐", palette.warning),
                        _ => ("○", palette.text_muted),
                    };
                    ui.colored_label(color, dot);
                });

                ui.horizontal(|ui| {
                    ui.label(tr(key::CW_TERM_SPEED, lang));
                    let mut wpm = settings.wpm;
                    let response = ui.add(egui::Slider::new(&mut wpm, 5..=60).text("WPM"));
                    if response.changed() {
                        settings.wpm = wpm;
                        actions.push(CwTerminalAction::SetSpeed(wpm));
                    }
                    if response.drag_stopped() || (response.changed() && !response.dragged()) {
                        actions.push(CwTerminalAction::SettingsChanged);
                    }
                });

                egui::CollapsingHeader::new(tr(key::CW_TERM_OPTIONS, lang))
                    .id_salt("cw_keyer_options")
                    .show(ui, |ui| {
                        ui.add_enabled_ui(!connected, |ui| {
                            let mut changed = false;
                            match settings.kind {
                                CwKeyerKind::WinKeyer => {
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::CW_TERM_SIDETONE, lang));
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut settings.sidetone_hz)
                                                    .range(400..=4000)
                                                    .suffix(" Hz"),
                                            )
                                            .changed();
                                    });
                                }
                                CwKeyerKind::SerialDtr | CwKeyerKind::SerialRts => {
                                    changed |= ui
                                        .checkbox(
                                            &mut settings.line_ptt,
                                            tr(key::CW_TERM_LINE_PTT, lang),
                                        )
                                        .changed();
                                    ui.horizontal(|ui| {
                                        ui.label(tr(key::CW_TERM_PTT_LEAD, lang));
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut settings.ptt_lead_ms)
                                                    .range(0..=500)
                                                    .suffix(" ms"),
                                            )
                                            .changed();
                                        ui.label(tr(key::CW_TERM_PTT_TAIL, lang));
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut settings.ptt_tail_ms)
                                                    .range(0..=2000)
                                                    .suffix(" ms"),
                                            )
                                            .changed();
                                    });
                                }
                                CwKeyerKind::Hamlib => {
                                    ui.label(
                                        egui::RichText::new(tr(key::CW_TERM_HAMLIB_HINT, lang))
                                            .small()
                                            .color(palette.text_muted),
                                    );
                                }
                            }
                            if changed {
                                actions.push(CwTerminalAction::SettingsChanged);
                            }
                        });
                    });

                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    for (index, text) in macros.iter().enumerate() {
                        if ui
                            .button(macro_button_label(index, text))
                            .on_hover_text(text)
                            .clicked()
                        {
                            actions.push(CwTerminalAction::Macro(index));
                        }
                    }
                    if ui
                        .button(
                            egui::RichText::new(format!("⛔ {}", tr(key::CW_TERM_ABORT, lang)))
                                .color(palette.danger),
                        )
                        .clicked()
                    {
                        actions.push(CwTerminalAction::Abort);
                    }
                });

                ui.separator();

                ui.label(egui::RichText::new(tr(key::CW_TERM_BUFFER_PREVIEW, lang)).strong());
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if !self.buffer.sent_text.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.buffer.sent_text).color(palette.success),
                            );
                        }
                        if !self.buffer.queued_text.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.buffer.queued_text)
                                    .color(palette.warning)
                                    .strong(),
                            );
                        }
                        if self.buffer.sent_text.is_empty() && self.buffer.queued_text.is_empty() {
                            ui.label(
                                egui::RichText::new(tr(key::CW_TERM_BUFFER_EMPTY, lang))
                                    .color(palette.text_muted)
                                    .italics(),
                            );
                        }
                    });
                });

                ui.add_space(6.0);
                ui.label(tr(key::CW_TERM_INPUT_PROMPT, lang));
                let response = ui.text_edit_singleline(&mut self.input_text);
                if response.lost_focus()
                    && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.input_text.trim().is_empty()
                {
                    let text = format!("{} ", self.input_text.trim().to_uppercase());
                    actions.push(CwTerminalAction::Send(text));
                    self.input_text.clear();
                    response.request_focus();
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status_text).small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(key::CW_TERM_CLEAR_HISTORY, lang)).clicked() {
                            self.buffer.sent_text.clear();
                        }
                        if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        if close_requested {
            open = false;
        }
        self.is_open = open;
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_labels_show_the_key_and_a_short_preview() {
        assert_eq!(macro_button_label(0, "CQ TEST"), "F1: CQ TEST");
        assert_eq!(
            macro_button_label(7, "QTH WROCLAW LOC JO81LC"),
            "F8: QTH WROCLAW LO…"
        );
    }
}
