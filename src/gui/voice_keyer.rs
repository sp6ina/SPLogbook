// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno dialogowe voice keyer'a (SSB) — zarządzanie komunikatami F1..F8,
//! nagrywanie, podgląd i transmisja z kluczowaniem PTT przez CAT.

use crate::core::station::VoiceKeyerMessage;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Rysuje okno voice keyer'a, jeśli jest otwarte.
pub fn render_voice_keyer_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_voice_keyer_window {
        return;
    }

    let mut open = app.show_voice_keyer_window;
    egui::Window::new("🎙 Voice Keyer (SSB)")
        .open(&mut open)
        .default_size([520.0, 460.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🎙 Voice Keyer — komunikaty głosowe").size(15.0).color(egui::Color32::from_rgb(56, 189, 248)));
            });
            ui.separator();
            ui.label(egui::RichText::new("Skonfiguruj zapowiedzi (pliki WAV) i odtwarzaj je na żywo z kluczowaniem PTT przez CAT. Plik WAV możesz nagrać mikrofonem lub wskazać istniejący.").small().color(egui::Color32::GRAY));
            ui.add_space(6.0);

            if ui.button("➕ Dodaj slot").clicked() {
                app.voice_keyer_messages.push(VoiceKeyerMessage {
                    label: format!("F{}", app.voice_keyer_messages.len() + 1),
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

                    for (idx, msg) in app.voice_keyer_messages.iter_mut().enumerate() {
                        let active = app.voice_keyer_active == Some(idx);
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let dot = if active { "🔴" } else { "⚪" };
                                if ui.selectable_label(active, format!("{dot} {}", msg.label)).clicked() {
                                    app.voice_keyer_active = Some(idx);
                                }
                                ui.add(egui::TextEdit::singleline(&mut msg.label).desired_width(90.0));
                                ui.checkbox(&mut msg.enabled, "Wł.");
                                ui.checkbox(&mut msg.repeat, "Pętla");
                                ui.checkbox(&mut msg.ptt, "PTT");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Tekst:");
                                ui.add(egui::TextEdit::singleline(&mut msg.text).desired_width(f32::INFINITY));
                            });
                            ui.horizontal(|ui| {
                                ui.label("WAV:");
                                let path = msg.wav_path.get_or_insert_with(String::new);
                                ui.add(egui::TextEdit::singleline(path).desired_width(f32::INFINITY));
                            });
                            ui.horizontal(|ui| {
                                if ui.button("▶ Podgląd").clicked() {
                                    preview_idx = Some(idx);
                                }
                                if ui.button("📻 TX").clicked() {
                                    tx_idx = Some(idx);
                                }
                                let recording_here = app.voice_keyer_recording == Some(idx);
                                if recording_here {
                                    if ui.button("⏹ Zapisz nagranie").clicked() {
                                        rec_idx = Some(idx);
                                    }
                                } else if ui.button("🎙 Nagraj").clicked() {
                                    rec_idx = Some(idx);
                                }
                                if ui.button("🗑").on_hover_text("Usuń slot").clicked() {
                                    remove_idx = Some(idx);
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }

                    if ui.button("⏹ STOP (zatrzymaj pętlę CQ)").clicked() {
                        crate::media::voice_keyer::request_stop();
                    }
                    ui.add_space(4.0);
                    if ui.button("💾 Zapisz ustawienia").clicked() {
                        app.save_station_config();
                    }

                    // Przetwarzanie akcji po pętli (unikamy jednoczesnego pożyczania)
                    if let Some(idx) = preview_idx {
                        if let Some(m) = app.voice_keyer_messages.get(idx).cloned() {
                            std::thread::spawn(move || {
                                let _ = crate::media::voice_keyer::play_message(&m);
                            });
                        }
                    }
                    if let Some(idx) = tx_idx {
                        if let Some(m) = app.voice_keyer_messages.get(idx).cloned() {
                            app.voice_keyer_active = Some(idx);
                            transmit_message(&m, app);
                        }
                    }
                    if let Some(idx) = rec_idx {
                        toggle_recording(app, idx);
                    }
                    if let Some(idx) = remove_idx {
                        if idx < app.voice_keyer_messages.len() {
                            app.voice_keyer_messages.remove(idx);
                            app.voice_keyer_active = None;
                            app.save_station_config();
                        }
                    }
                });
        });
    app.show_voice_keyer_window = open;
}

/// Kluczuje PTT (jeśli włączone i CAT połączony), odtwarza komunikat, zwalnia PTT.
fn transmit_message(msg: &VoiceKeyerMessage, app: &SpLogApp) {
    let host = app.cat_host.clone();
    let port = app.cat_port;
    let connected = app.cat_connected;
    let was_already_transmitting = app.rig_state.ptt || app.ptt_active;
    let key_ptt = msg.ptt && connected && !was_already_transmitting;

    let msg_clone = msg.clone();
    tokio::spawn(async move {
        if key_ptt {
            let _ = crate::cat::hamlib::HamlibClient::set_ptt(&host, port, true).await;
        }
        let m2 = msg_clone.clone();
        let _ =
            tokio::task::spawn_blocking(move || crate::media::voice_keyer::play_message(&m2)).await;
        if key_ptt {
            let _ = crate::cat::hamlib::HamlibClient::set_ptt(&host, port, false).await;
        }
    });
}

/// Rozpoczyna lub kończy nagrywanie slotu do pliku recordings/voice_keyer/fN.wav.
fn toggle_recording(app: &mut SpLogApp, idx: usize) {
    if app.voice_keyer_recording == Some(idx) {
        let rec_dir = std::path::Path::new("recordings").join("voice_keyer");
        if let Some(parent) = rec_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let path = rec_dir.join(format!("f{}.wav", idx + 1));
        if let Ok(saved) = crate::media::audio_recorder::AudioRecorder::stop_and_save(&path) {
            if let Some(msg) = app.voice_keyer_messages.get_mut(idx) {
                msg.wav_path = Some(saved.to_string_lossy().to_string());
            }
            app.save_station_config();
        }
        app.voice_keyer_recording = None;
    } else {
        match crate::media::audio_recorder::AudioRecorder::start_recording() {
            Ok(()) => app.voice_keyer_recording = Some(idx),
            Err(e) => log::warn!("Nie udało się rozpocząć nagrywania: {e}"),
        }
    }
}
