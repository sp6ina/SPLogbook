// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Panel widma / waterfallu (SDR) — przechwytywanie dźwięku z wejścia
//! (np. wyjście audio radia) i rysowanie spektrogramu FFT w czasie rzeczywistym.

use crate::dsp::waterfall::{SampleRing, WaterfallEngine};
use crate::gui::app::SpLogApp;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const FFT_SIZES: [usize; 4] = [512, 1024, 2048, 4096];
const HISTORY_DEPTH: usize = 256;
const RING_CAPACITY: usize = 1 << 16; // ~1.3 s przy 48 kHz

pub struct WaterfallPanel {
    running: bool,
    devices_loaded: bool,
    engine: Option<WaterfallEngine>,
    ring: Arc<SampleRing>,
    running_flag: Arc<AtomicBool>,
    capture_thread: Option<std::thread::JoinHandle<()>>,
    error_slot: Arc<Mutex<Option<String>>>,
    device_names: Vec<String>,
    selected_device: usize,
    fft_idx: usize,
    gain_db: f32,
    floor_db: f32,
    color_scale: f32,
    texture: Option<egui::TextureHandle>,
}

impl Default for WaterfallPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl WaterfallPanel {
    pub fn new() -> Self {
        Self {
            running: false,
            devices_loaded: false,
            engine: None,
            ring: SampleRing::new(RING_CAPACITY),
            running_flag: Arc::new(AtomicBool::new(false)),
            capture_thread: None,
            error_slot: Arc::new(Mutex::new(None)),
            device_names: Vec::new(),
            selected_device: 0,
            fft_idx: 1, // 1024
            gain_db: 0.0,
            floor_db: -100.0,
            color_scale: 1.0,
            texture: None,
        }
    }

    /// Odświeża listę urządzeń wejściowych audio (wywoływane raz przy pierwszym
    /// otwarciu panelu oraz ręcznie przyciskiem ⟳).
    pub fn refresh_devices(&mut self) {
        self.device_names.clear();
        if let Ok(devices) = cpal::default_host().input_devices() {
            for d in devices {
                self.device_names.push(d.to_string());
            }
        }
        self.devices_loaded = true;
    }

    fn start(&mut self) {
        if self.running {
            return;
        }

        // Określenie częstotliwości próbkowania wybranego urządzenia (do budowy silnika FFT).
        let device_name = self.selected_device_name();
        let sample_rate =
            pick_input_device(device_name.as_deref()).map_or(48_000, |(_, cfg)| cfg.sample_rate());

        let fft_size = FFT_SIZES[self.fft_idx];
        self.engine = Some(WaterfallEngine::new(fft_size, sample_rate, HISTORY_DEPTH));
        self.texture = None;
        *self
            .error_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;

        let running = Arc::new(AtomicBool::new(true));
        let ring = self.ring.clone();
        let error_slot = self.error_slot.clone();
        let thread_running = running.clone();
        let thread_name = self.selected_device_name();

        let handle = std::thread::spawn(move || {
            match start_audio_stream(thread_name.as_deref(), ring) {
                Ok(stream) => {
                    // Strumień pozostaje żywy, dopóki `thread_running` jest ustawione.
                    if let Err(e) = stream.play() {
                        *error_slot
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) =
                            Some(format!("Błąd odtwarzania strumienia: {e}"));
                    }
                    while thread_running.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                }
                Err(e) => {
                    log::error!("Nie udało się uruchomić przechwytywania audio: {e}");
                    *error_slot
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(e);
                }
            }
        });

        self.running_flag = running;
        self.capture_thread = Some(handle);
        self.running = true;
    }

    fn stop(&mut self) {
        if !self.running {
            return;
        }
        self.running_flag.store(false, Ordering::Relaxed);
        self.capture_thread = None; // odłączenie wątku; zakończy się po bieżącej iteracji
        self.running = false;
    }

    fn selected_device_name(&self) -> Option<String> {
        self.device_names.get(self.selected_device).cloned()
    }

    /// Zapewnia, że silnik FFT ma właściwe FFT size (po zmianie z listy).
    fn sync_engine(&mut self) {
        let fft_size = FFT_SIZES[self.fft_idx];
        let needs_rebuild = match &self.engine {
            Some(e) => e.fft_size() != fft_size,
            None => false,
        };
        if needs_rebuild {
            let sample_rate = self.engine.as_ref().map_or(
                48_000,
                super::super::dsp::waterfall::WaterfallEngine::sample_rate,
            );
            self.engine = Some(WaterfallEngine::new(fft_size, sample_rate, HISTORY_DEPTH));
            self.texture = None;
        }
    }

    pub fn render_body(&mut self, ui: &mut egui::Ui) {
        if !self.devices_loaded {
            self.refresh_devices();
        }

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("waterfall_device")
                .width(220.0)
                .selected_text(
                    self.device_names
                        .get(self.selected_device)
                        .cloned()
                        .unwrap_or_else(|| "Domyślne urządzenie".to_string()),
                )
                .show_ui(ui, |ui| {
                    for (i, name) in self.device_names.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_device, i, name);
                    }
                    if self.device_names.is_empty() {
                        ui.label("Brak urządzeń wejściowych.");
                    }
                });

            if self.running {
                if ui.button(icons_stop()).on_hover_text("Zatrzymaj").clicked() {
                    self.stop();
                }
            } else if ui.button(icons_play()).on_hover_text("Start").clicked() {
                self.start();
            }

            if ui
                .button("⟳")
                .on_hover_text("Odśwież listę urządzeń")
                .clicked()
            {
                self.refresh_devices();
            }

            ui.separator();
            ui.label("FFT:");
            egui::ComboBox::from_id_salt("waterfall_fft")
                .selected_text(format!("{}", FFT_SIZES[self.fft_idx]))
                .show_ui(ui, |ui| {
                    for (i, n) in FFT_SIZES.iter().enumerate() {
                        ui.selectable_value(&mut self.fft_idx, i, n.to_string());
                    }
                });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Wzmocnienie:");
            if ui
                .add(egui::Slider::new(&mut self.gain_db, -40.0..=40.0).suffix(" dB"))
                .changed()
            {
                self.texture = None;
            }
            ui.label("Podłoga szumów:");
            if ui
                .add(egui::Slider::new(&mut self.floor_db, -160.0..=-40.0).suffix(" dB"))
                .changed()
            {
                self.texture = None;
            }
            ui.label("Nasycenie:");
            if ui
                .add(egui::Slider::new(&mut self.color_scale, 0.1..=3.0))
                .changed()
            {
                self.texture = None;
            }
            if ui.button("Wyczyść").clicked() {
                if let Some(e) = self.engine.as_mut() {
                    e.clear();
                }
                self.texture = None;
            }
        });

        if let Some(err) = self
            .error_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            ui.colored_label(egui::Color32::from_rgb(239, 68, 68), format!("⚠ {err}"));
        }

        ui.add_space(4.0);
        self.sync_engine();

        // Przetworzenie nowych próbek z bufora pierścieniowego.
        if self.running {
            let mut samples = Vec::new();
            self.ring.drain(&mut samples);
            if let Some(engine) = self.engine.as_mut() {
                engine.set_gain_db(self.gain_db);
                engine.set_floor_db(self.floor_db);
                engine.feed(&samples);
            }
        }

        // Rysowanie spektrogramu.
        if self
            .engine
            .as_ref()
            .is_some_and(|e| !e.history().is_empty())
        {
            self.draw_spectrogram(ui);
            ui.add_space(4.0);
            self.draw_spectrum(ui);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Naciśnij ▶ Start, aby rozpocząć przechwytywanie widma.")
                        .color(egui::Color32::from_gray(140)),
                );
            });
        }
    }

    fn draw_spectrogram(&mut self, ui: &mut egui::Ui) {
        if self.running || self.texture.is_none() {
            // Obliczenie pikseli w osobnym zakresie, aby nie trzymać pożyczki `self.engine`
            // podczas mutowania `self.texture`.
            let (pixels, bins, rows) = {
                let Some(engine) = self.engine.as_ref() else {
                    return;
                };
                let history = engine.history();
                let rows = history.len();
                let bins = history[0].len();
                let mut pixels = Vec::with_capacity(rows * bins);
                let floor_db = if self.floor_db.is_finite() {
                    self.floor_db.min(-1.0)
                } else {
                    -100.0
                };
                let denom = (-floor_db).max(1.0);
                let color_scale = if self.color_scale.is_finite() {
                    self.color_scale
                } else {
                    1.0
                };

                for row in history {
                    for &db in row {
                        let norm = if db.is_finite() {
                            (((db - floor_db) / denom) * color_scale).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        pixels.push(waterfall_color(norm));
                    }
                }
                (pixels, bins, rows)
            };

            let image = egui::ColorImage::new([bins, rows], pixels);

            let options = egui::TextureOptions::NEAREST;
            if let Some(tex) = &mut self.texture {
                tex.set(image, options);
            } else {
                self.texture = Some(
                    ui.ctx()
                        .load_texture("waterfall_spectrogram", image, options),
                );
            }
        }

        let height = 240.0_f32;
        if let Some(tex) = &self.texture {
            let size = egui::vec2(ui.available_width().max(64.0), height);
            ui.add(egui::Image::new(tex).fit_to_exact_size(size).max_size(size));
        }
    }

    fn draw_spectrum(&self, ui: &mut egui::Ui) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let Some(row) = engine.latest_row() else {
            return;
        };
        let desired = egui::vec2(ui.available_width().max(64.0), 120.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, egui::Color32::from_gray(8));

        let n = row.len();
        if n < 2 {
            return;
        }
        let floor_db = if self.floor_db.is_finite() {
            self.floor_db.min(-1.0)
        } else {
            -100.0
        };
        let denom = (-floor_db).max(1.0);
        let points: Vec<egui::Pos2> = row
            .iter()
            .enumerate()
            .map(|(i, &db)| {
                let x = rect.left() + rect.width() * (i as f32 / (n - 1) as f32);
                let norm = if db.is_finite() {
                    ((db - floor_db) / denom).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let y = rect.bottom() - norm * rect.height();
                egui::pos2(x, y)
            })
            .collect();

        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(56, 189, 248)),
        ));
    }
}

impl Drop for WaterfallPanel {
    fn drop(&mut self) {
        // Zatrzymuje wątek przechwytujący i strumień audio przy zamykaniu aplikacji.
        self.stop();
    }
}

/// Renderuje zawartość dokowanego kafelka waterfall (wywoływane z `app_layout`).
pub fn render_waterfall_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    app.waterfall_panel.render_body(ui);
}

/// Renderuje kafelek waterfall jako odpięte, pływające okno widoku
/// (wzorzec zgodny z `world_map`). Wywoływane z `app.rs`, gdy
/// `panel_waterfall.floating` jest włączone.
pub fn render_waterfall_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_waterfall.visible || !app.panel_waterfall.floating {
        return;
    }

    let mut dock_back = false;
    let mut still_open = true;

    let ((), captured_geo) = app.show_floating_viewport(
        ctx,
        egui::ViewportId::from_hash_of("waterfall_viewport"),
        "Widmo / Waterfall (SDR) - SPLogbook".to_string(),
        [720.0, 480.0],
        [480.0, 320.0],
        app.panel_waterfall.saved_pos,
        app.panel_waterfall.saved_size,
        |app, ui| {
            egui::Panel::top("waterfall_vp_bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button("↙ Zadokuj")
                        .on_hover_text("Zadokuj kafelek")
                        .clicked()
                    {
                        dock_back = true;
                    }
                });
            });
            egui::CentralPanel::default().show(ui, |ui| {
                app.waterfall_panel.render_body(ui);
            });
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                still_open = false;
            }
        },
    );

    if let Some((pos, size)) = captured_geo {
        if app.panel_waterfall.saved_pos != Some(pos)
            || app.panel_waterfall.saved_size != Some(size)
        {
            app.panel_waterfall.saved_pos = Some(pos);
            app.panel_waterfall.saved_size = Some(size);
            if !ctx.input(|i| i.pointer.any_down()) {
                app.save_station_config();
            }
        }
    }

    if dock_back {
        app.panel_waterfall.floating = false;
        app.save_station_config();
    }

    if !still_open {
        app.panel_waterfall.visible = false;
        app.panel_waterfall.floating = false;
        app.save_station_config();
    }
}

fn icons_play() -> &'static str {
    "▶ Start"
}

fn icons_stop() -> &'static str {
    "⏹ Stop"
}

/// Mapuje wartość znormalizowaną (0..1) na kolor waterfallu:
/// czarny → granat → cyjan → zielony → żółty → czerwony.
fn waterfall_color(t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f32, (u8, u8, u8)); 6] = [
        (0.00, (0, 0, 0)),
        (0.25, (0, 0, 128)),
        (0.50, (0, 200, 255)),
        (0.68, (0, 230, 0)),
        (0.85, (255, 230, 0)),
        (1.00, (255, 0, 0)),
    ];
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            let f = f.clamp(0.0, 1.0);
            let r = lerp_u8(c0.0, c1.0, f);
            let g = lerp_u8(c0.1, c1.1, f);
            let b = lerp_u8(c0.2, c1.2, f);
            return egui::Color32::from_rgb(r, g, b);
        }
    }
    egui::Color32::from_rgb(255, 0, 0)
}

fn lerp_u8(a: u8, b: u8, f: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * f).round() as u8
}

/// Wyszukuje urządzenie wejściowe po nazwie (lub domyślne) i zwraca je z konfiguracją.
fn pick_input_device(
    name: Option<&str>,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig), String> {
    let host = cpal::default_host();
    let devices: Vec<cpal::Device> = host
        .input_devices()
        .map_err(|e| format!("Błąd listowania urządzeń wejściowych: {e}"))?
        .collect();
    if devices.is_empty() {
        return Err("Brak urządzeń wejściowych audio.".to_string());
    }

    let by_name = name
        .filter(|n| !n.is_empty())
        .and_then(|n| devices.iter().find(|d| d.to_string() == n).cloned());

    let device = by_name
        .or_else(|| host.default_input_device())
        .or_else(|| devices.into_iter().next())
        .ok_or_else(|| "Nie znaleziono urządzenia wejściowego.".to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Brak domyślnej konfiguracji wejścia: {e}"))?;
    Ok((device, config))
}

/// Buduje strumień wejściowy cpal dla wybranego urządzenia i formatu próbek.
fn start_audio_stream(
    device_name: Option<&str>,
    ring: Arc<SampleRing>,
) -> Result<cpal::Stream, String> {
    let (device, supported) = pick_input_device(device_name)?;
    let sample_format = supported.sample_format();
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.into();

    match sample_format {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, config, channels, ring),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, config, channels, ring),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, config, channels, ring),
        cpal::SampleFormat::I8 => build_stream::<i8>(&device, config, channels, ring),
        cpal::SampleFormat::U8 => build_stream::<u8>(&device, config, channels, ring),
        cpal::SampleFormat::I32 => build_stream::<i32>(&device, config, channels, ring),
        other => Err(format!("Nieobsługiwany format próbek: {other:?}")),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn stream_err_fn(err: cpal::Error) {
    log::error!("Błąd strumienia audio (waterfall): {err}");
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    ring: Arc<SampleRing>,
) -> Result<cpal::Stream, String>
where
    T: cpal::Sample<Float = f32> + cpal::SizedSample,
{
    let mut mono: Vec<f32> = Vec::with_capacity(4096);
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                mono.clear();
                for chunk in data.chunks(channels.max(1)) {
                    let sum: f32 = chunk.iter().map(|s| s.to_float_sample()).sum();
                    mono.push(sum / chunk.len() as f32);
                }
                ring.push(&mono);
            },
            stream_err_fn,
            None,
        )
        .map_err(|e| format!("Nie udało się zbudować strumienia wejściowego: {e}"))
}
