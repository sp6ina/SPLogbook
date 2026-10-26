// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno dialogowe voice keyer'a (SSB) — zarządzanie komunikatami F1..F8,
//! nagrywanie, podgląd i transmisja z kluczowaniem PTT przez CAT.

use crate::core::i18n::{key, tr};
use crate::core::station::VoiceKeyerMessage;
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

/// Rysuje okno voice keyer'a, jeśli jest otwarte.
pub fn render_voice_keyer_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.voice_keyer.show_window {
        return;
    }

    let mut open = app.voice_keyer.show_window;
    let lang = app.gui.appearance.language;
    egui::Window::new(tr(key::VOICE_WINDOW_TITLE, lang))
        .open(&mut open)
        .default_size([520.0, 460.0])
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(tr(key::VOICE_HEADING, lang))
                        .size(15.0)
                        .color(palette.accent),
                );
            });
            ui.separator();
            ui.label(
                egui::RichText::new(tr(key::VOICE_DESCRIPTION, lang))
                    .small()
                    .color(palette.text_muted),
            );
            ui.add_space(6.0);

            if ui.button(tr(key::VOICE_ADD_SLOT, lang)).clicked() {
                app.config.voice_keyer_messages.push(VoiceKeyerMessage {
                    label: format!("F{}", app.config.voice_keyer_messages.len() + 1),
                    ..Default::default()
                });
                app.save_station_config();
            }

            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    let mut tx_idx: Option<usize> = None;
                    let mut preview_idx: Option<usize> = None;
                    let mut rec_idx: Option<usize> = None;

                    for (idx, msg) in app.config.voice_keyer_messages.iter_mut().enumerate() {
                        let active = app.voice_keyer.active == Some(idx);
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let dot = if active { "🔴" } else { "⚪" };
                                if ui
                                    .selectable_label(active, format!("{dot} {}", msg.label))
                                    .clicked()
                                {
                                    app.voice_keyer.active = Some(idx);
                                }
                                ui.add(
                                    egui::TextEdit::singleline(&mut msg.label).desired_width(90.0),
                                );
                                ui.checkbox(&mut msg.enabled, tr(key::VOICE_ENABLED, lang));
                                ui.checkbox(&mut msg.repeat, tr(key::VOICE_REPEAT, lang));
                                ui.checkbox(&mut msg.ptt, "PTT");
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr(key::VOICE_TEXT_LABEL, lang));
                                ui.add(
                                    egui::TextEdit::singleline(&mut msg.text)
                                        .desired_width(f32::INFINITY),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("WAV:");
                                let path = msg.wav_path.get_or_insert_with(String::new);
                                ui.add(
                                    egui::TextEdit::singleline(path).desired_width(f32::INFINITY),
                                );
                            });
                            ui.horizontal(|ui| {
                                if ui.button(tr(key::VOICE_PREVIEW, lang)).clicked() {
                                    preview_idx = Some(idx);
                                }
                                if ui.button("📻 TX").clicked() {
                                    tx_idx = Some(idx);
                                }
                                let recording_here = app.voice_keyer.recording == Some(idx);
                                if recording_here {
                                    if ui.button(tr(key::VOICE_SAVE_RECORDING, lang)).clicked() {
                                        rec_idx = Some(idx);
                                    }
                                } else if ui.button(tr(key::VOICE_RECORD, lang)).clicked() {
                                    rec_idx = Some(idx);
                                }
                                if ui
                                    .button("🗑")
                                    .on_hover_text(tr(key::VOICE_DELETE_SLOT, lang))
                                    .clicked()
                                {
                                    remove_idx = Some(idx);
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }

                    if ui.button(tr(key::VOICE_STOP_LOOP, lang)).clicked() {
                        crate::media::voice_keyer::request_stop();
                    }
                    ui.add_space(4.0);
                    if ui.button(tr(key::VOICE_SAVE_SETTINGS, lang)).clicked() {
                        app.save_station_config();
                    }

                    // Przetwarzanie akcji po pętli (unikamy jednoczesnego pożyczania)
                    if let Some(idx) = preview_idx
                        && let Some(m) = app.config.voice_keyer_messages.get(idx).cloned()
                    {
                        std::thread::spawn(move || {
                            let _ = crate::media::voice_keyer::play_message(&m);
                        });
                    }
                    if let Some(idx) = tx_idx
                        && let Some(m) = app.config.voice_keyer_messages.get(idx).cloned()
                    {
                        app.voice_keyer.active = Some(idx);
                        transmit_message(&m, app);
                    }
                    if let Some(idx) = rec_idx {
                        toggle_recording(app, idx);
                    }
                    if let Some(idx) = remove_idx
                        && idx < app.config.voice_keyer_messages.len()
                    {
                        app.config.voice_keyer_messages.remove(idx);
                        app.voice_keyer.active = None;
                        app.save_station_config();
                    }
                });
        });
    app.voice_keyer.show_window = open;
}

/// Kluczuje PTT (jeśli włączone i CAT połączony), odtwarza komunikat, zwalnia PTT.
fn transmit_message(msg: &VoiceKeyerMessage, app: &SpLogApp) {
    let connected = app.cat_connected;
    let was_already_transmitting = app.rig_state.ptt || app.ptt_active;
    let key_ptt = msg.ptt && connected && !was_already_transmitting;

    let snap = app.cat_runtime.snapshot();
    let tx_rig = snap
        .active_tx
        .as_ref()
        .map(|t| t.rig_id.clone())
        .or_else(|| snap.active_rx.as_ref().map(|r| r.rig_id.clone()));
    let rt = app.cat_runtime.clone();

    let msg_clone = msg.clone();
    tokio::spawn(async move {
        if key_ptt && let Some(ref rig_id) = tx_rig {
            rt.request_tx(rig_id.clone(), std::time::Duration::from_millis(1500));
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let m2 = msg_clone.clone();
        let _ =
            tokio::task::spawn_blocking(move || crate::media::voice_keyer::play_message(&m2)).await;
        if key_ptt && let Some(ref rig_id) = tx_rig {
            rt.release_tx(rig_id.clone(), std::time::Duration::from_millis(1500));
        }
    });
}

/// Rozpoczyna lub kończy nagrywanie slotu do pliku recordings/voice_keyer/fN.wav.
fn toggle_recording(app: &mut SpLogApp, idx: usize) {
    if app.voice_keyer.recording == Some(idx) {
        let rec_dir = std::path::Path::new("recordings").join("voice_keyer");
        if let Some(parent) = rec_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let path = rec_dir.join(format!("f{}.wav", idx + 1));
        if let Ok(saved) = crate::media::audio_recorder::AudioRecorder::stop_and_save(&path) {
            if let Some(msg) = app.config.voice_keyer_messages.get_mut(idx) {
                msg.wav_path = Some(saved.to_string_lossy().to_string());
            }
            app.save_station_config();
        }
        app.voice_keyer.recording = None;
    } else {
        match crate::media::audio_recorder::AudioRecorder::start_recording() {
            Ok(()) => app.voice_keyer.recording = Some(idx),
            Err(e) => log::warn!("Failed to start recording: {e}"),
        }
    }
}
