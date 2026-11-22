// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Nadawanie CW z GUI (STAGE-V140-S6): podłączenie klucza, makra, terminal i postęp.

use super::SpLogApp;
use crate::cat::cw_keyer::{self, CwCommand, CwEvent, CwKeyerKind};
use crate::core::i18n::{Msg, key};
use crate::gui::cw_terminal::CwTerminalAction;

impl SpLogApp {
    /// Podłącza klucz wybrany w ustawieniach. Błąd otwarcia portu jest zgłaszany.
    pub fn connect_cw_keyer(&mut self) {
        self.disconnect_cw_keyer();
        let settings = self.config.cw.keyer.clone();
        let opened = match settings.kind {
            CwKeyerKind::WinKeyer => cw_keyer::open_winkeyer(&settings),
            CwKeyerKind::SerialDtr | CwKeyerKind::SerialRts => {
                cw_keyer::open_serial_line(&settings)
            }
            CwKeyerKind::Hamlib => Ok(cw_keyer::spawn_hamlib(
                self.config.cat_host.clone(),
                self.config.cat_port,
                settings.wpm,
            )),
        };
        let terminal = &mut self.gui.dialogs.cw_terminal_dialog;
        match opened {
            Ok(handle) => {
                terminal.keyer = Some(handle);
                terminal.ready = false;
                terminal.status_text =
                    Msg::new(key::CW_TERM_CONNECTING).text(self.gui.appearance.language);
            }
            Err(e) => self.fail(Msg::new(key::CW_KEYER_ERROR).arg("err", e)),
        }
    }

    /// Odłącza klucz; zadanie klucza zwalnia linie i zamyka WinKeyera.
    pub fn disconnect_cw_keyer(&mut self) {
        let terminal = &mut self.gui.dialogs.cw_terminal_dialog;
        if terminal.keyer.take().is_some() {
            terminal.ready = false;
            terminal.buffer.abort();
            terminal.status_text =
                crate::core::i18n::tr(key::CW_TERM_DISCONNECTED, self.gui.appearance.language)
                    .to_string();
        }
    }

    /// Wysyła tekst do klucza i dopisuje go do bufora terminala. Bez podłączonego klucza
    /// nic nie jest „udawane”: operator dostaje ostrzeżenie.
    pub fn send_cw(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let terminal = &mut self.gui.dialogs.cw_terminal_dialog;
        let Some(keyer) = terminal.keyer.as_ref() else {
            self.warn(Msg::new(key::CW_KEYER_NOT_CONNECTED));
            return;
        };
        if keyer.send(CwCommand::Send(text.to_string())) {
            terminal.buffer.append_to_queue(text);
        } else {
            self.disconnect_cw_keyer();
            self.warn(Msg::new(key::CW_KEYER_NOT_CONNECTED));
        }
    }

    /// Natychmiastowe przerwanie nadawania.
    pub fn abort_cw(&mut self) {
        let terminal = &mut self.gui.dialogs.cw_terminal_dialog;
        if let Some(keyer) = terminal.keyer.as_ref() {
            keyer.send(CwCommand::Abort);
        }
        terminal.buffer.abort();
    }

    /// Makro z podstawionymi zmiennymi (`%MYCALL%`, `%HISCALL%`, `%RST%`, `%SERIAL%`,
    /// `%MYQTH%`, `%MYLOC%`).
    pub fn resolve_cw_macro(&self, text: &str) -> String {
        text.replace("%MYCALL%", &self.config.station.callsign)
            .replace("%MYQTH%", &self.config.station.city)
            .replace("%MYLOC%", &self.config.station.gridsquare)
            .replace("%HISCALL%", &self.entry.callsign)
            .replace("%RST%", &self.entry.rst_sent)
            .replace("%SERIAL%", &format!("{:03}", self.contest.stx))
    }

    /// Nadaje makro (okno makr, terminal, pluginy).
    pub fn transmit_cw_macro(&mut self, text: &str) {
        let resolved = self.resolve_cw_macro(text);
        let with_gap = if resolved.ends_with(' ') {
            resolved
        } else {
            format!("{resolved} ")
        };
        self.send_cw(&with_gap);
    }

    /// Wykonuje działania wybrane w terminalu CW.
    pub fn apply_cw_terminal_actions(&mut self, actions: Vec<CwTerminalAction>) {
        for action in actions {
            match action {
                CwTerminalAction::Connect => self.connect_cw_keyer(),
                CwTerminalAction::Disconnect => self.disconnect_cw_keyer(),
                CwTerminalAction::Send(text) => self.send_cw(&text),
                CwTerminalAction::Macro(index) => {
                    if let Some(text) = self.cw.macros.get(index).cloned() {
                        self.transmit_cw_macro(&text);
                    }
                }
                CwTerminalAction::Abort => self.abort_cw(),
                CwTerminalAction::SetSpeed(wpm) => {
                    if let Some(keyer) = self.gui.dialogs.cw_terminal_dialog.keyer.as_ref() {
                        keyer.send(CwCommand::SetSpeed(wpm));
                    }
                }
                CwTerminalAction::SettingsChanged => self.save_station_config(),
            }
        }
    }

    /// Zdarzenia klucza: postęp nadawania, gotowość, błędy, zamknięcie.
    pub fn poll_cw_keyer(&mut self) {
        let lang = self.gui.appearance.language;
        let events = match self.gui.dialogs.cw_terminal_dialog.keyer.as_ref() {
            Some(keyer) => keyer.drain_events(),
            None => return,
        };
        for event in events {
            let terminal = &mut self.gui.dialogs.cw_terminal_dialog;
            match event {
                CwEvent::Ready(detail) => {
                    terminal.ready = true;
                    terminal.status_text = Msg::new(key::CW_TERM_READY)
                        .arg("detail", keyer_description(&self.config.cw.keyer, &detail))
                        .text(lang);
                }
                CwEvent::Sent(count) => terminal.buffer.advance_sent(count),
                CwEvent::Idle => {
                    if !terminal.buffer.queued_text.is_empty() {
                        terminal.buffer.abort();
                    }
                }
                CwEvent::Error(e) => {
                    terminal.buffer.abort();
                    self.fail(Msg::new(key::CW_KEYER_ERROR).arg("err", e));
                }
                CwEvent::Closed => {
                    terminal.keyer = None;
                    terminal.ready = false;
                    terminal.status_text =
                        crate::core::i18n::tr(key::CW_TERM_DISCONNECTED, lang).to_string();
                }
            }
        }
        if !self
            .gui
            .dialogs
            .cw_terminal_dialog
            .buffer
            .queued_text
            .is_empty()
        {
            self.runtime.request_repaint();
        }
    }
}

/// Opis klucza w pasku stanu terminala.
fn keyer_description(settings: &cw_keyer::CwKeyerSettings, detail: &str) -> String {
    if settings.kind == CwKeyerKind::Hamlib || settings.port.is_empty() {
        detail.to_string()
    } else if detail.is_empty() {
        settings.port.clone()
    } else {
        format!("{detail} ({})", settings.port)
    }
}
