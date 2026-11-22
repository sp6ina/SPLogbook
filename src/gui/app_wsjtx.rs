// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Dwukierunkowa współpraca z WSJT-X / JTDX (STAGE-V140-S8): nasłuch na porcie z ustawień,
//! okno dekodów, odpowiedź na CQ i kolorowanie znaków w oknie Band Activity.

use super::SpLogApp;
use crate::core::i18n::{Msg, key};
use crate::digital::wsjtx::{
    self, MAX_HIGHLIGHTS, WsjtxDecode, WsjtxMark, WsjtxMessage, WsjtxOutgoing, WsjtxStatus,
};
use crate::gui::state::ToastSeverity;
use crate::gui::state::digital::{MAX_WSJTX_DECODES, WsjtxDecodeRow};
use std::collections::BTreeSet;
use std::time::Duration;

impl SpLogApp {
    /// Uruchamia nasłuch według bieżących ustawień, zamykając poprzedni. Poprzednie gniazdo
    /// jest zwalniane, zanim nowe zajmie port; błąd otwarcia portu trafia do komunikatów.
    pub fn start_wsjtx_link(&mut self) {
        let previous = self.stop_wsjtx_link();
        let settings = self.config.wsjtx.clone();
        if !settings.enabled {
            return;
        }
        let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
        let forward = self.digital.wsjtx_tx.clone();
        let notices = self.runtime.notice_sender();
        let egui_ctx = std::sync::Arc::clone(&self.runtime.egui_ctx);
        let link = tokio::spawn(async move {
            if let Some(previous) = previous {
                let _ = tokio::time::timeout(Duration::from_secs(2), previous).await;
            }
            let socket = match wsjtx::bind_socket(&settings).await {
                Ok(socket) => socket,
                Err(e) => {
                    log::warn!("WSJT-X: {e}");
                    notices.send(
                        ToastSeverity::Error,
                        Msg::new(key::WSJTX_BIND_FAILED).arg("e", e),
                    );
                    return;
                }
            };
            let (tx, mut rx) = tokio::sync::mpsc::channel(256);
            let receiver = tokio::spawn(wsjtx::run_link(socket, tx, out_rx));
            while let Some(msg) = rx.recv().await {
                if forward.send(msg).is_err() {
                    break;
                }
                if let Ok(guard) = egui_ctx.lock()
                    && let Some(ctx) = guard.as_ref()
                {
                    ctx.request_repaint();
                }
            }
            drop(rx);
            let _ = receiver.await;
        });
        self.digital.wsjtx.out = Some(out_tx);
        self.digital.wsjtx.link = Some(link);
    }

    /// Zatrzymuje nasłuch: usuwa wyróżnienia ustawione w WSJT-X i zamyka kanał poleceń.
    /// Zwraca zadanie, które kończy się po zwolnieniu gniazda.
    pub fn stop_wsjtx_link(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        let state = &mut self.digital.wsjtx;
        if let Some(out) = state.out.take() {
            let highlighted: BTreeSet<&String> = state.marks.keys().map(|(id, _)| id).collect();
            for id in highlighted {
                let _ = out.send(WsjtxOutgoing {
                    client_id: id.clone(),
                    packet: wsjtx::encode_clear_highlights(id),
                });
            }
        }
        state.marks.clear();
        state.clients.clear();
        state.link.take()
    }

    /// Odbiór pakietów WSJT-X / JTDX.
    pub fn poll_wsjtx(&mut self) {
        while let Ok(msg) = self.digital.wsjtx_rx.try_recv() {
            self.digital.wsjtx_packets_count += 1;
            match msg {
                WsjtxMessage::Heartbeat { id } => {
                    self.digital
                        .wsjtx
                        .clients
                        .entry(id.clone())
                        .or_insert_with(|| WsjtxStatus {
                            id,
                            ..WsjtxStatus::default()
                        });
                }
                WsjtxMessage::Status(status) => self.apply_wsjtx_status(*status),
                WsjtxMessage::Decode(decode) => self.add_wsjtx_decode(decode),
                WsjtxMessage::Clear { id } => {
                    self.digital.wsjtx.decodes.retain(|row| row.decode.id != id);
                }
                WsjtxMessage::Close { id } => self.forget_wsjtx_client(&id),
                WsjtxMessage::QsoLogged { qso, .. } => self.handle_wsjtx_qso(*qso),
            }
        }
        if self.digital.wsjtx.marks_stale {
            self.refresh_wsjtx_marks();
        }
    }

    fn apply_wsjtx_status(&mut self, status: WsjtxStatus) {
        if !status.dx_call.is_empty() {
            self.digital.wsjtx_last_call = Some(status.dx_call.clone());
        }
        if status.dial_freq > 0 {
            self.cat.rig_state.frequency_hz = status.dial_freq;
            if let Some(band_def) = crate::core::bandplan::get_band_by_freq(status.dial_freq) {
                self.entry.band = band_def.name.to_string();
            }
        }
        if !status.mode.is_empty() {
            self.entry.mode.clone_from(&status.mode);
        }
        let clients = &self.digital.wsjtx.clients;
        // Po zmianie pasma albo emisji dotychczasowe oceny znaków są nieaktualne.
        if clients.get(&status.id).is_some_and(|previous| {
            band_of(previous.dial_freq) != band_of(status.dial_freq) || previous.mode != status.mode
        }) {
            self.digital.wsjtx.marks_stale = true;
        }
        self.digital.wsjtx.clients.insert(status.id.clone(), status);
    }

    fn add_wsjtx_decode(&mut self, decode: WsjtxDecode) {
        let (band, mode) = self
            .digital
            .wsjtx
            .clients
            .get(&decode.id)
            .map(|status| (band_of(status.dial_freq), status.mode.clone()))
            .unwrap_or_default();
        let call = decode.sender().map(str::to_uppercase);
        let mark = match (&call, &band) {
            (Some(call), Some(band)) => self.wsjtx_mark(call, band, &mode),
            _ => WsjtxMark::Other,
        };
        if decode.cq_caller().is_some()
            && let Some(call) = &call
        {
            self.highlight_wsjtx_call(&decode.id, call, mark);
        }
        let decodes = &mut self.digital.wsjtx.decodes;
        decodes.push_front(WsjtxDecodeRow {
            decode,
            call,
            band,
            mode,
            mark,
        });
        decodes.truncate(MAX_WSJTX_DECODES);
    }

    fn forget_wsjtx_client(&mut self, id: &str) {
        let state = &mut self.digital.wsjtx;
        state.clients.remove(id);
        state.decodes.retain(|row| row.decode.id != id);
        state.marks.retain(|(client, _), _| client != id);
    }

    /// Ocena stacji względem dziennika: nowe DXCC, nowe pasmo albo już zrobiona.
    pub fn wsjtx_mark(&self, call: &str, band: &str, mode: &str) -> WsjtxMark {
        let Some(info) = self.services.prefix_matcher.lookup(call) else {
            return WsjtxMark::Other;
        };
        let awards = self
            .services
            .awards_engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let status = awards.check_status_full(
            call,
            band,
            mode,
            Some(info.dxcc),
            None,
            Some(info.cqz),
            None,
            Some(info.continent.as_str()),
            None,
        );
        if status.is_new_dxcc {
            WsjtxMark::NewDxcc
        } else if status.is_new_band {
            WsjtxMark::NewBand
        } else if status.is_worked_b4 {
            WsjtxMark::WorkedBefore
        } else {
            WsjtxMark::Other
        }
    }

    /// Wysyła wyróżnienie znaku, jeśli różni się od już ustawionego.
    fn highlight_wsjtx_call(&mut self, id: &str, call: &str, mark: WsjtxMark) {
        if !self.config.wsjtx.highlight {
            return;
        }
        let state = &mut self.digital.wsjtx;
        let Some(out) = state.out.as_ref() else {
            return;
        };
        let entry = (id.to_string(), call.to_string());
        if state.marks.get(&entry) == Some(&mark) {
            return;
        }
        let send = |packet: Vec<u8>| {
            let _ = out.send(WsjtxOutgoing {
                client_id: id.to_string(),
                packet,
            });
        };
        let Some((background, foreground)) = mark.colors() else {
            if state.marks.remove(&entry).is_some() {
                send(wsjtx::encode_highlight(id, call, None, None, false));
            }
            return;
        };
        let client_marks = state.marks.keys().filter(|(c, _)| c == id).count();
        if client_marks >= MAX_HIGHLIGHTS {
            send(wsjtx::encode_clear_highlights(id));
            state.marks.retain(|(client, _), _| client != id);
        }
        send(wsjtx::encode_highlight(
            id, call, background, foreground, false,
        ));
        state.marks.insert(entry, mark);
    }

    /// Przelicza oceny dekodów po zmianie dziennika albo pasma i odnawia wyróżnienia.
    fn refresh_wsjtx_marks(&mut self) {
        self.digital.wsjtx.marks_stale = false;
        if let Some(out) = self.digital.wsjtx.out.as_ref() {
            let highlighted: BTreeSet<&String> =
                self.digital.wsjtx.marks.keys().map(|(id, _)| id).collect();
            for id in highlighted {
                let _ = out.send(WsjtxOutgoing {
                    client_id: id.clone(),
                    packet: wsjtx::encode_clear_highlights(id),
                });
            }
        }
        self.digital.wsjtx.marks.clear();

        let inputs: Vec<(Option<String>, Option<String>, String)> = self
            .digital
            .wsjtx
            .decodes
            .iter()
            .map(|row| (row.call.clone(), row.band.clone(), row.mode.clone()))
            .collect();
        let marks: Vec<WsjtxMark> = inputs
            .iter()
            .map(|(call, band, mode)| match (call, band) {
                (Some(call), Some(band)) => self.wsjtx_mark(call, band, mode),
                _ => WsjtxMark::Other,
            })
            .collect();
        let mut to_highlight = Vec::new();
        for (row, mark) in self.digital.wsjtx.decodes.iter_mut().zip(marks) {
            row.mark = mark;
            if row.decode.cq_caller().is_some()
                && let Some(call) = &row.call
            {
                to_highlight.push((row.decode.id.clone(), call.clone(), mark));
            }
        }
        let mut seen = BTreeSet::new();
        for (id, call, mark) in to_highlight {
            if seen.insert((id.clone(), call.clone())) {
                self.highlight_wsjtx_call(&id, &call, mark);
            }
        }
    }

    /// Prosi WSJT-X o odpowiedź na CQ z wiersza `index` okna dekodów.
    pub fn reply_to_wsjtx_decode(&mut self, index: usize) {
        let Some(row) = self.digital.wsjtx.decodes.get(index) else {
            return;
        };
        let Some(call) = row.decode.cq_caller().map(str::to_string) else {
            return;
        };
        let lang = self.gui.appearance.language;
        let Some(out) = self.digital.wsjtx.out.as_ref() else {
            self.gui.chrome.push_status_toast(
                ToastSeverity::Warning,
                Msg::new(key::WSJTX_NOT_LISTENING).text(lang),
            );
            return;
        };
        let packet = wsjtx::encode_reply(&row.decode, 0);
        if out
            .send(WsjtxOutgoing {
                client_id: row.decode.id.clone(),
                packet,
            })
            .is_ok()
        {
            self.gui.chrome.push_status_toast(
                ToastSeverity::Info,
                Msg::new(key::WSJTX_REPLY_SENT).arg("call", call).text(lang),
            );
        }
    }

    /// Zapisuje ustawienia z okna i uruchamia nasłuch od nowa.
    pub fn apply_wsjtx_settings(&mut self, settings: wsjtx::WsjtxSettings) {
        self.config.wsjtx = settings;
        self.save_station_config();
        self.start_wsjtx_link();
    }
}

fn band_of(freq_hz: u64) -> Option<String> {
    crate::core::bandplan::get_band_by_freq(freq_hz).map(|band| band.name.to_string())
}
