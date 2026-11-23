// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Panel Spectrum + Waterfall pasma audio (styl WSJT-X / FLDigi): przechwytywanie dźwięku
//! z wejścia (np. wyjście audio radia), widmo nad wodospadem ze wspólną osią częstotliwości
//! (Hz), siatka dB, Peak Hold, Auto-Level, celownik, znacznik tonu/pasma oraz Click-to-QSY.

use crate::core::i18n::{key, tr};
use crate::dsp::waterfall::{
    AutoLevel, PeakHold, SampleRing, WaterfallEngine, af_to_rf_hz, bin_to_hz, click_qsy_delta_hz,
    default_tone_hz, hz_to_bin, sideband_for_mode,
};
use crate::gui::app::SpLogApp;
use crate::gui::theme::{
    Palette, SPACE_SM, TEXT_CAPTION, ThemeExt, WaterfallPalette, secondary_button, undock_button,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const FFT_SIZES: [usize; 4] = [512, 1024, 2048, 4096];
const HISTORY_DEPTH: usize = 256;
const RING_CAPACITY: usize = 1 << 16; // ~1.3 s przy 48 kHz
/// Zakresy częstotliwości widoku (Hz); ograniczane do częstotliwości Nyquista.
const VIEW_RANGES_HZ: [f32; 4] = [3000.0, 6000.0, 12_000.0, 24_000.0];
/// Opadanie linii Peak Hold (dB na nowy wiersz FFT).
const PEAK_DECAY_DB: f32 = 0.35;
/// Margines pod poziomem szumu przy Auto-Level (dB).
const AUTO_FLOOR_MARGIN_DB: f32 = 6.0;
/// Ton CW używany do przeliczeń, gdy brak osobnej konfiguracji (Hz).
const DEFAULT_CW_PITCH_HZ: f64 = 700.0;
/// Wysokość paska osi częstotliwości (px).
const AXIS_HEIGHT: f32 = 18.0;
/// Udział widma w wysokości wykresu (reszta to wodospad).
const SPECTRUM_FRACTION: f32 = 0.30;

// ---------------------------------------------------------------------------------------
// Geometria wykresu i oś częstotliwości (czyste funkcje)
// ---------------------------------------------------------------------------------------

/// Podział obszaru wykresu: widmo (≈30%), oś częstotliwości, wodospad (≈70%).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotLayout {
    pub spectrum: egui::Rect,
    pub axis: egui::Rect,
    pub waterfall: egui::Rect,
}

/// Dzieli `rect` płynnie wg proporcji; widmo i wodospad mają minimalne wysokości.
#[must_use]
pub fn plot_layout(rect: egui::Rect, axis_height: f32) -> PlotLayout {
    let usable = (rect.height() - axis_height).max(0.0);
    let spectrum_h = (usable * SPECTRUM_FRACTION)
        .max(60.0)
        .min((usable - 40.0).max(0.0));
    let spectrum = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), spectrum_h));
    let axis = egui::Rect::from_min_size(
        spectrum.left_bottom(),
        egui::vec2(rect.width(), axis_height),
    );
    let waterfall = egui::Rect::from_min_max(axis.left_bottom(), rect.right_bottom());
    PlotLayout {
        spectrum,
        axis,
        waterfall,
    }
}

/// Częstotliwość audio (Hz) dla współrzędnej `x` na osi o zakresie `0..view_max_hz`.
#[must_use]
pub fn x_to_hz(rect: egui::Rect, x: f32, view_max_hz: f32) -> f32 {
    if rect.width() <= 0.0 {
        return 0.0;
    }
    ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) * view_max_hz
}

/// Współrzędna `x` dla częstotliwości `hz`.
#[must_use]
pub fn hz_to_x(rect: egui::Rect, hz: f32, view_max_hz: f32) -> f32 {
    if view_max_hz <= 0.0 {
        return rect.left();
    }
    rect.left() + rect.width() * (hz / view_max_hz)
}

/// Krok podziałki osi (Hz): najmniejszy z 100/200/500/1000/2000/5000 dający >= 56 px odstępu.
#[must_use]
pub fn axis_step_hz(width_px: f32, view_max_hz: f32) -> f32 {
    const STEPS: [f32; 6] = [100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0];
    if width_px <= 0.0 || view_max_hz <= 0.0 {
        return STEPS[STEPS.len() - 1];
    }
    STEPS
        .into_iter()
        .find(|step| step / view_max_hz * width_px >= 56.0)
        .unwrap_or(STEPS[STEPS.len() - 1])
}

/// Dostępne zakresy widoku dla częstotliwości Nyquista.
#[must_use]
pub fn view_options(nyquist_hz: f32) -> Vec<f32> {
    let mut options: Vec<f32> = VIEW_RANGES_HZ
        .into_iter()
        .filter(|hz| *hz <= nyquist_hz + 1.0)
        .collect();
    if options.is_empty() {
        options.push(nyquist_hz.max(100.0));
    }
    options
}

/// Normalizacja poziomu (dBFS) do 0..1 względem poziomu tła `floor_db` (0 dBFS = pełna skala).
#[must_use]
pub fn display_norm(db: f32, floor_db: f32, color_scale: f32) -> f32 {
    let floor = if floor_db.is_finite() {
        floor_db.min(-1.0)
    } else {
        -100.0
    };
    if !db.is_finite() {
        return 0.0;
    }
    let scale = if color_scale.is_finite() {
        color_scale
    } else {
        1.0
    };
    (((db - floor) / (-floor).max(1.0)) * scale).clamp(0.0, 1.0)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// Parametry, od których zależy tekstura wodospadu — przebudowa tylko przy ich zmianie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextureKey {
    generation: u64,
    palette: WaterfallPalette,
    floor_bits: u32,
    scale_bits: u32,
    gain_bits: u32,
    bins: usize,
}

/// Czy trzeba przebudować teksturę: brak tekstury albo zmiana generacji/parametrów.
fn texture_needs_rebuild(previous: Option<&TextureKey>, current: &TextureKey) -> bool {
    previous != Some(current)
}

/// Akcja panelu do wykonania przez aplikację (poza stanem panelu).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaterfallAction {
    /// Dostrój VFO na podaną częstotliwość (Hz) istniejącą ścieżką CAT.
    Retune(u64),
    /// Odepnij panel do osobnego okna.
    Undock,
}

/// Dane z aplikacji potrzebne panelowi (CAT, język).
#[derive(Debug, Clone)]
pub struct WaterfallContext {
    pub lang: crate::core::i18n::Language,
    pub vfo_hz: u64,
    pub mode: String,
    pub cat_connected: bool,
    pub docked: bool,
}

pub struct WaterfallPanel {
    running: bool,
    devices_loaded: bool,
    engine: Option<WaterfallEngine>,
    ring: Arc<SampleRing>,
    running_flag: Arc<AtomicBool>,
    capture_thread: Option<std::thread::JoinHandle<()>>,
    error_slot: Arc<Mutex<Option<WaterfallAudioError>>>,
    device_names: Vec<String>,
    selected_device: usize,
    fft_idx: usize,
    gain_db: f32,
    floor_db: f32,
    color_scale: f32,
    palette: WaterfallPalette,
    auto_level: bool,
    auto: AutoLevel,
    peak_hold_on: bool,
    peak: PeakHold,
    view_max_hz: f32,
    /// Ton docelowy ustawiony kliknięciem (gdy CAT niepodłączony); `None` = domyślny dla trybu.
    target_tone_hz: Option<f32>,
    texture: Option<egui::TextureHandle>,
    texture_key: Option<TextureKey>,
    /// Licznik wgrań tekstury (diagnostyka i testy: brak przebudowy „na pusto”).
    texture_uploads: u64,
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
            palette: WaterfallPalette::Classic,
            auto_level: true,
            auto: AutoLevel::default(),
            peak_hold_on: true,
            peak: PeakHold::default(),
            view_max_hz: 3000.0,
            target_tone_hz: None,
            texture: None,
            texture_key: None,
            texture_uploads: 0,
        }
    }

    /// Ustawienia do zapisania w konfiguracji.
    pub fn settings(&self) -> crate::core::station::WaterfallSettings {
        crate::core::station::WaterfallSettings {
            palette: self.palette.id().to_string(),
            fft_index: self.fft_idx,
            gain_db: self.gain_db,
            floor_db: self.floor_db,
            color_scale: self.color_scale,
            auto_level: self.auto_level,
            peak_hold: self.peak_hold_on,
            view_max_hz: self.view_max_hz,
        }
    }

    /// Przywraca zapisane ustawienia; wartości spoza zakresów suwaków są przycinane,
    /// a nieznane wartości wracają do domyślnych.
    pub fn apply_settings(&mut self, settings: &crate::core::station::WaterfallSettings) {
        self.palette = WaterfallPalette::ALL
            .into_iter()
            .find(|p| p.id() == settings.palette)
            .unwrap_or(WaterfallPalette::Classic);
        self.fft_idx = settings.fft_index.min(FFT_SIZES.len() - 1);
        self.gain_db = finite_or(settings.gain_db, 0.0).clamp(-40.0, 40.0);
        self.floor_db = finite_or(settings.floor_db, -100.0).clamp(-160.0, -40.0);
        self.color_scale = finite_or(settings.color_scale, 1.0).clamp(0.1, 3.0);
        self.auto_level = settings.auto_level;
        self.peak_hold_on = settings.peak_hold;
        self.view_max_hz = if VIEW_RANGES_HZ.contains(&settings.view_max_hz) {
            settings.view_max_hz
        } else {
            VIEW_RANGES_HZ[0]
        };
        self.texture_key = None;
    }

    /// Odświeża listę urządzeń wejściowych audio (wywoływane raz przy pierwszym
    /// otwarciu panelu oraz ręcznie przyciskiem odświeżania).
    pub fn refresh_devices(&mut self) {
        self.device_names.clear();
        if let Ok(devices) = cpal::default_host().input_devices() {
            for device in devices {
                self.device_names.push(device.to_string());
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
        self.texture_key = None;
        self.peak.reset();
        self.auto.reset();
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
                            Some(WaterfallAudioError::Playback(e.to_string()));
                    }
                    while thread_running.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                }
                Err(e) => {
                    log::error!("Failed to start audio capture: {e}");
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
            self.texture_key = None;
            self.peak.reset();
        }
    }

    /// Poziom tła używany do wyświetlania: z Auto-Level (szum - margines) albo ręczny `Floor`.
    fn display_floor_db(&self) -> f32 {
        if self.auto_level
            && let Some(noise) = self.auto.noise_db()
        {
            return (noise - AUTO_FLOOR_MARGIN_DB).clamp(-150.0, -20.0);
        }
        self.floor_db
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, cx: &WaterfallContext) -> Option<WaterfallAction> {
        let lang = cx.lang;
        let palette = ui.palette();
        let mut action = None;

        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("waterfall_device")
                .width(200.0)
                .selected_text(
                    self.device_names
                        .get(self.selected_device)
                        .cloned()
                        .unwrap_or_else(|| tr(key::WATERFALL_DEFAULT_DEVICE, lang).to_string()),
                )
                .show_ui(ui, |ui| {
                    for (i, name) in self.device_names.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_device, i, name);
                    }
                    if self.device_names.is_empty() {
                        ui.label(tr(key::WATERFALL_NO_INPUT_DEVICES, lang));
                    }
                });

            if self.running {
                if ui
                    .add(secondary_button(
                        &palette,
                        tr(key::WATERFALL_STOP_BTN, lang),
                    ))
                    .clicked()
                {
                    self.stop();
                }
            } else if ui
                .add(secondary_button(
                    &palette,
                    tr(key::WATERFALL_START_BTN, lang),
                ))
                .clicked()
            {
                self.start();
            }

            if ui
                .add(secondary_button(
                    &palette,
                    tr(key::WATERFALL_REFRESH_BTN, lang),
                ))
                .on_hover_text(tr(key::WATERFALL_REFRESH_DEVICES, lang))
                .clicked()
            {
                self.refresh_devices();
            }

            ui.separator();
            ui.label("FFT:");
            egui::ComboBox::from_id_salt("waterfall_fft")
                .width(60.0)
                .selected_text(format!("{}", FFT_SIZES[self.fft_idx]))
                .show_ui(ui, |ui| {
                    for (i, size) in FFT_SIZES.iter().enumerate() {
                        ui.selectable_value(&mut self.fft_idx, i, size.to_string());
                    }
                });

            let nyquist = self
                .engine
                .as_ref()
                .map_or(24_000.0, |e| e.sample_rate() as f32 / 2.0);
            let options = view_options(nyquist);
            if !options.contains(&self.view_max_hz) {
                self.view_max_hz = options[0];
            }
            ui.label(tr(key::WATERFALL_RANGE_LABEL, lang));
            egui::ComboBox::from_id_salt("waterfall_range")
                .width(70.0)
                .selected_text(format!("{:.0} kHz", self.view_max_hz / 1000.0))
                .show_ui(ui, |ui| {
                    for option in &options {
                        ui.selectable_value(
                            &mut self.view_max_hz,
                            *option,
                            format!("{:.0} kHz", option / 1000.0),
                        );
                    }
                });

            ui.label(tr(key::WATERFALL_PALETTE_LABEL, lang));
            egui::ComboBox::from_id_salt("waterfall_palette")
                .width(100.0)
                .selected_text(palette_name(self.palette, lang))
                .show_ui(ui, |ui| {
                    for option in WaterfallPalette::ALL {
                        ui.selectable_value(&mut self.palette, option, palette_name(option, lang));
                    }
                });

            if cx.docked {
                let free = ui.available_width() - 28.0;
                if free > 0.0 {
                    ui.add_space(free);
                }
                if undock_button(&mut *ui, &palette, tr(key::WINDOW_POPOUT_TOOLTIP, lang)).clicked()
                {
                    action = Some(WaterfallAction::Undock);
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(tr(key::WATERFALL_GAIN_LABEL, lang));
            ui.add(egui::Slider::new(&mut self.gain_db, -40.0..=40.0).suffix(" dB"));
            ui.label(tr(key::WATERFALL_NOISE_FLOOR, lang));
            let manual_floor = !self.auto_level;
            ui.add_enabled(
                manual_floor,
                egui::Slider::new(&mut self.floor_db, -160.0..=-40.0).suffix(" dB"),
            )
            .on_disabled_hover_text(tr(key::WATERFALL_AUTO_LEVEL_TOOLTIP, lang));
            ui.label(tr(key::WATERFALL_SATURATION, lang));
            ui.add(egui::Slider::new(&mut self.color_scale, 0.1..=3.0));
            ui.checkbox(&mut self.auto_level, tr(key::WATERFALL_AUTO_LEVEL, lang))
                .on_hover_text(tr(key::WATERFALL_AUTO_LEVEL_TOOLTIP, lang));
            ui.checkbox(&mut self.peak_hold_on, tr(key::WATERFALL_PEAK_HOLD, lang));
            if ui
                .add_enabled(
                    self.peak_hold_on,
                    secondary_button(&palette, tr(key::WATERFALL_PEAK_RESET, lang)),
                )
                .clicked()
            {
                self.peak.reset();
            }
            if ui
                .add(secondary_button(
                    &palette,
                    tr(key::WATERFALL_CLEAR_BTN, lang),
                ))
                .clicked()
            {
                if let Some(e) = self.engine.as_mut() {
                    e.clear();
                }
                self.peak.reset();
                self.auto.reset();
                self.texture_key = None;
            }
        });
        action
    }

    /// Główne ciało panelu; zwraca akcję do wykonania przez aplikację (dostrojenie / odpięcie).
    pub fn render_body(
        &mut self,
        ui: &mut egui::Ui,
        cx: &WaterfallContext,
    ) -> Option<WaterfallAction> {
        let lang = cx.lang;
        if !self.devices_loaded {
            self.refresh_devices();
        }

        let mut action = self.toolbar(ui, cx);

        if let Some(err) = self
            .error_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            ui.colored_label(ui.palette().danger, format!("⚠ {}", err.msg().text(lang)));
        }

        self.sync_engine();
        self.pump_samples();

        let has_data = self
            .engine
            .as_ref()
            .is_some_and(|e| !e.history().is_empty());
        if has_data {
            if let Some(plot_action) = self.draw_plot(ui, cx) {
                action = Some(plot_action);
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(tr(key::WATERFALL_START_HINT, lang))
                        .color(ui.palette().text_muted),
                );
            });
        }

        if self.running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        action
    }

    /// Przetwarza nowe próbki z bufora; Peak Hold i Auto-Level aktualizowane są tylko
    /// przy nowej linii FFT (zmiana generacji).
    fn pump_samples(&mut self) {
        if !self.running {
            return;
        }
        let mut samples = Vec::new();
        self.ring.drain(&mut samples);
        let Some(engine) = self.engine.as_mut() else {
            return;
        };
        // Silnik przycina do dołu do -120 dBFS; poziom wyświetlania ustawia Auto-Level/Floor.
        engine.set_gain_db(self.gain_db);
        engine.set_floor_db(-120.0);
        let before = engine.generation();
        engine.feed(&samples);
        if engine.generation() != before
            && let Some(row) = engine.latest_row()
        {
            self.auto.update(row);
            if self.peak_hold_on {
                self.peak.update(row, PEAK_DECAY_DB);
            }
        }
    }

    /// Przebudowuje teksturę wodospadu wyłącznie przy zmianie generacji lub parametrów.
    fn refresh_texture(&mut self, ctx: &egui::Context) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let Some(first) = engine.history().front() else {
            return;
        };
        let floor = self.display_floor_db();
        let bins = first.len();
        let key = TextureKey {
            generation: engine.generation(),
            palette: self.palette,
            floor_bits: floor.to_bits(),
            scale_bits: self.color_scale.to_bits(),
            gain_bits: self.gain_db.to_bits(),
            bins,
        };
        if !texture_needs_rebuild(self.texture_key.as_ref(), &key) && self.texture.is_some() {
            return;
        }

        // Najnowszy wiersz u góry; brakujące (starsze) wiersze wypełnione kolorem tła.
        let blank = self.palette.color(0.0);
        let mut pixels = vec![blank; bins * HISTORY_DEPTH];
        for (row_index, row) in engine.history().iter().rev().enumerate() {
            if row_index >= HISTORY_DEPTH || row.len() != bins {
                continue;
            }
            for (col, &db) in row.iter().enumerate() {
                pixels[row_index * bins + col] =
                    self.palette
                        .color(display_norm(db, floor, self.color_scale));
            }
        }
        let image = egui::ColorImage::new([bins, HISTORY_DEPTH], pixels);
        let options = egui::TextureOptions::NEAREST;
        if let Some(tex) = &mut self.texture {
            tex.set(image, options);
        } else {
            self.texture = Some(ctx.load_texture("waterfall_spectrogram", image, options));
        }
        self.texture_key = Some(key);
        self.texture_uploads += 1;
    }

    fn draw_plot(&mut self, ui: &mut egui::Ui, cx: &WaterfallContext) -> Option<WaterfallAction> {
        let palette = ui.palette();
        self.refresh_texture(ui.ctx());

        let (bin_hz, bins, nyquist) = {
            let engine = self.engine.as_ref()?;
            let bins = engine.history().front().map_or(0, Vec::len);
            (engine.bin_hz(), bins, engine.sample_rate() as f32 / 2.0)
        };
        if bins < 2 {
            return None;
        }
        let view_max = self.view_max_hz.min(nyquist).max(100.0);

        let avail = ui.available_size();
        let size = egui::vec2(avail.x.max(200.0), (avail.y - 4.0).max(200.0));
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let layout = plot_layout(rect, AXIS_HEIGHT);
        let painter = ui.painter_at(rect);
        let floor = self.display_floor_db();
        let mode = cx.mode.as_str();
        let tone = f64::from(
            self.target_tone_hz
                .unwrap_or_else(|| default_tone_hz(mode, DEFAULT_CW_PITCH_HZ) as f32),
        );

        // Wodospad: kolumny widoku jako przycięcie UV tekstury (zmiana zakresu bez przebudowy).
        if let Some(tex) = &self.texture {
            let u_max = (view_max / nyquist).clamp(0.01, 1.0);
            let image = egui::Image::new((tex.id(), tex.size_vec2()))
                .uv(egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(u_max, 1.0),
                ))
                .fit_to_exact_size(layout.waterfall.size());
            ui.put(layout.waterfall, image);
        }

        // Widmo
        painter.rect_filled(layout.spectrum, 0.0, palette.extreme_bg);
        let step = axis_step_hz(rect.width(), view_max);
        Self::draw_spectrum_grid(&painter, &palette, layout.spectrum, floor, view_max, step);
        Self::draw_passband(&painter, &palette, &layout, mode, tone, view_max);
        self.draw_spectrum_curve(&painter, &palette, layout.spectrum, floor, view_max, bin_hz);

        // Oś częstotliwości (Hz)
        painter.rect_filled(layout.axis, 0.0, palette.panel_bg);
        let mut hz = 0.0_f32;
        while hz <= view_max + 0.1 {
            let x = hz_to_x(rect, hz, view_max);
            painter.line_segment(
                [
                    egui::pos2(x, layout.axis.top()),
                    egui::pos2(x, layout.axis.top() + 4.0),
                ],
                egui::Stroke::new(1.0_f32, palette.text_muted),
            );
            let align = if hz <= 0.1 {
                egui::Align2::LEFT_CENTER
            } else if hz >= view_max - 0.1 {
                egui::Align2::RIGHT_CENTER
            } else {
                egui::Align2::CENTER_CENTER
            };
            painter.text(
                egui::pos2(x, layout.axis.center().y + 2.0),
                align,
                format!("{hz:.0}"),
                egui::FontId::monospace(TEXT_CAPTION),
                palette.text_muted,
            );
            // Pionowa siatka także na wodospadzie.
            painter.line_segment(
                [
                    egui::pos2(x, layout.waterfall.top()),
                    egui::pos2(x, layout.waterfall.bottom()),
                ],
                egui::Stroke::new(0.5_f32, palette.border_subtle),
            );
            hz += step;
        }
        painter.text(
            egui::pos2(layout.axis.right() - 22.0, layout.axis.center().y + 2.0),
            egui::Align2::RIGHT_CENTER,
            "Hz",
            egui::FontId::monospace(TEXT_CAPTION),
            palette.text_subtle,
        );

        // Celownik, etykieta i Click-to-QSY
        let mut action = None;
        if let Some(pos) = response.hover_pos()
            && rect.contains(pos)
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            let af_hz = x_to_hz(rect, pos.x, view_max);
            self.draw_crosshair(&painter, &palette, &layout, pos, af_hz, bin_hz, bins, cx);
            let hint = if cx.cat_connected && cx.vfo_hz > 0 {
                tr(key::WATERFALL_CLICK_QSY_TOOLTIP, cx.lang)
            } else {
                tr(key::WATERFALL_CLICK_TONE_TOOLTIP, cx.lang)
            };
            response.clone().on_hover_text(hint);
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let clicked_af = f64::from(x_to_hz(rect, pos.x, view_max));
            if cx.cat_connected && cx.vfo_hz > 0 {
                let delta = click_qsy_delta_hz(clicked_af, tone, mode);
                let new_vfo = cx.vfo_hz as f64 + delta;
                if new_vfo >= 100_000.0 {
                    action = Some(WaterfallAction::Retune(new_vfo.round() as u64));
                }
            } else {
                // Bez CAT: tylko przesuwamy znacznik tonu docelowego (lokalne zaznaczenie).
                self.target_tone_hz = Some(clicked_af as f32);
            }
        }
        action
    }

    fn draw_spectrum_grid(
        painter: &egui::Painter,
        palette: &Palette,
        rect: egui::Rect,
        floor_db: f32,
        view_max: f32,
        step_hz: f32,
    ) {
        let grid = egui::Stroke::new(0.5_f32, palette.border_subtle);
        let mut hz = 0.0_f32;
        while hz <= view_max + 0.1 {
            let x = hz_to_x(rect, hz, view_max);
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                grid,
            );
            hz += step_hz;
        }
        // Siatka dB co 20 dB od 0 dBFS w dół do poziomu tła.
        let mut db = 0.0_f32;
        while db > floor_db {
            let y = rect.bottom() - display_norm(db, floor_db, 1.0) * rect.height();
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                grid,
            );
            painter.text(
                egui::pos2(rect.left() + 3.0, y - 1.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{db:.0}"),
                egui::FontId::monospace(TEXT_CAPTION),
                palette.text_subtle,
            );
            db -= 20.0;
        }
    }

    /// Półprzezroczysty znacznik pasma przepustowego i linia tonu docelowego.
    fn draw_passband(
        painter: &egui::Painter,
        palette: &Palette,
        layout: &PlotLayout,
        mode: &str,
        tone: f64,
        view_max: f32,
    ) {
        use crate::dsp::waterfall::Sideband;
        let half_width = match sideband_for_mode(mode) {
            Sideband::Cw | Sideband::CwReverse => 100.0_f32,
            _ => 1200.0_f32,
        };
        let tone = tone as f32;
        let whole = egui::Rect::from_min_max(layout.spectrum.min, layout.waterfall.right_bottom());
        let left = hz_to_x(whole, (tone - half_width).max(0.0), view_max);
        let right = hz_to_x(whole, (tone + half_width).min(view_max), view_max);
        if right > left {
            let band = egui::Rect::from_min_max(
                egui::pos2(left, whole.top()),
                egui::pos2(right, whole.bottom()),
            );
            painter.rect_filled(band, 0.0, palette.info.gamma_multiply(0.12));
        }
        let x = hz_to_x(whole, tone, view_max);
        if (0.0..=view_max).contains(&tone) {
            painter.line_segment(
                [egui::pos2(x, whole.top()), egui::pos2(x, whole.bottom())],
                egui::Stroke::new(1.0_f32, palette.info),
            );
        }
    }

    fn draw_spectrum_curve(
        &self,
        painter: &egui::Painter,
        palette: &Palette,
        rect: egui::Rect,
        floor_db: f32,
        view_max: f32,
        bin_hz: f32,
    ) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let Some(row) = engine.latest_row() else {
            return;
        };
        let last_bin = hz_to_bin(view_max, bin_hz, row.len());
        if last_bin < 2 {
            return;
        }
        let stride = ((last_bin as f32 / rect.width().max(1.0)).ceil() as usize).max(1);
        let to_point = |bin: usize, db: f32| {
            egui::pos2(
                hz_to_x(rect, bin_to_hz(bin, bin_hz), view_max),
                rect.bottom() - display_norm(db, floor_db, 1.0) * rect.height(),
            )
        };

        // Wypełnienie pod krzywą: jeden Mesh (pasy ze wspólnymi wierzchołkami = bez szwów).
        let top = palette.accent.gamma_multiply(0.35);
        let bottom = palette.accent.gamma_multiply(0.04);
        let mut mesh = egui::Mesh::default();
        let mut points = Vec::new();
        for bin in (0..=last_bin).step_by(stride) {
            let point = to_point(bin, row[bin]);
            let base = egui::pos2(point.x, rect.bottom());
            let index = mesh.vertices.len() as u32;
            mesh.colored_vertex(point, top);
            mesh.colored_vertex(base, bottom);
            if index >= 2 {
                mesh.add_triangle(index - 2, index - 1, index);
                mesh.add_triangle(index - 1, index + 1, index);
            }
            points.push(point);
        }
        painter.add(egui::Shape::mesh(mesh));
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5_f32, palette.accent),
        ));

        if self.peak_hold_on && !self.peak.values().is_empty() {
            let peak_points: Vec<egui::Pos2> = (0..=last_bin.min(self.peak.values().len() - 1))
                .step_by(stride)
                .map(|bin| to_point(bin, self.peak.values()[bin]))
                .collect();
            painter.add(egui::Shape::line(
                peak_points,
                egui::Stroke::new(1.0_f32, palette.accent_alt),
            ));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_crosshair(
        &self,
        painter: &egui::Painter,
        palette: &Palette,
        layout: &PlotLayout,
        pos: egui::Pos2,
        af_hz: f32,
        bin_hz: f32,
        bins: usize,
        cx: &WaterfallContext,
    ) {
        let stroke = egui::Stroke::new(1.0_f32, palette.text_primary.gamma_multiply(0.7));
        painter.line_segment(
            [
                egui::pos2(pos.x, layout.spectrum.top()),
                egui::pos2(pos.x, layout.waterfall.bottom()),
            ],
            stroke,
        );
        if layout.spectrum.contains(pos) {
            painter.line_segment(
                [
                    egui::pos2(layout.spectrum.left(), pos.y),
                    egui::pos2(layout.spectrum.right(), pos.y),
                ],
                stroke,
            );
        }

        let level = self
            .engine
            .as_ref()
            .and_then(WaterfallEngine::latest_row)
            .and_then(|row| row.get(hz_to_bin(af_hz, bin_hz, bins)).copied());
        let mut label = format!("{af_hz:.0} Hz");
        if let Some(db) = level.filter(|v| v.is_finite()) {
            let _ = write!(label, "  {db:.0} dB");
        }
        if cx.vfo_hz > 0 {
            let rf = af_to_rf_hz(
                cx.vfo_hz as f64,
                f64::from(af_hz),
                &cx.mode,
                DEFAULT_CW_PITCH_HZ,
            );
            let _ = write!(label, "  RF {:.6} MHz", rf / 1.0e6);
        }

        let galley = painter.layout_no_wrap(
            label,
            egui::FontId::monospace(TEXT_CAPTION),
            palette.text_primary,
        );
        let size = galley.size() + egui::vec2(SPACE_SM * 2.0, SPACE_SM);
        let mut origin = pos + egui::vec2(12.0, 12.0);
        let bounds = egui::Rect::from_min_max(layout.spectrum.min, layout.waterfall.max);
        if origin.x + size.x > bounds.right() {
            origin.x = pos.x - size.x - 12.0;
        }
        if origin.y + size.y > bounds.bottom() {
            origin.y = pos.y - size.y - 12.0;
        }
        let tag = egui::Rect::from_min_size(origin, size);
        painter.rect_filled(tag, 3.0, palette.extreme_bg.gamma_multiply(0.9));
        painter.rect_stroke(
            tag,
            3.0,
            egui::Stroke::new(1.0_f32, palette.border),
            egui::StrokeKind::Inside,
        );
        painter.galley(
            tag.min + egui::vec2(SPACE_SM, SPACE_SM * 0.5),
            galley,
            palette.text_primary,
        );
    }
}

/// Nazwa palety w interfejsie (nazwy własne bez tłumaczenia; wysoki kontrast — z katalogu).
fn palette_name(palette: WaterfallPalette, lang: crate::core::i18n::Language) -> String {
    match palette {
        WaterfallPalette::Classic => tr(key::WATERFALL_PALETTE_CLASSIC, lang).to_string(),
        WaterfallPalette::Inferno => "Inferno".to_string(),
        WaterfallPalette::Viridis => "Viridis".to_string(),
        WaterfallPalette::IcomBlue => "Icom Blue".to_string(),
        WaterfallPalette::HighContrast => tr(key::THEME_PRESET_CONTRAST_LABEL, lang).to_string(),
    }
}

impl Drop for WaterfallPanel {
    fn drop(&mut self) {
        // Zatrzymuje wątek przechwytujący i strumień audio przy zamykaniu aplikacji.
        self.running_flag.store(false, Ordering::Relaxed);
    }
}

fn waterfall_context(app: &SpLogApp, docked: bool) -> WaterfallContext {
    WaterfallContext {
        lang: app.gui.appearance.language,
        vfo_hz: app.cat.rig_state.frequency_hz,
        mode: app.cat.rig_state.mode.clone(),
        cat_connected: app.cat.connected,
        docked,
    }
}

/// Wykonuje akcję zwróconą przez panel istniejącymi ścieżkami aplikacji (CAT / układ okien).
fn apply_waterfall_action(app: &mut SpLogApp, action: Option<WaterfallAction>) {
    match action {
        Some(WaterfallAction::Retune(freq_hz)) => app.set_vfo_frequency(freq_hz),
        Some(WaterfallAction::Undock) => app.popout_tile("waterfall"),
        None => {}
    }
}

/// Renderuje zawartość dokowanego kafelka waterfall (wywoływane z `app_layout`).
pub fn render_waterfall_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let cx = waterfall_context(app, !app.views.panel_waterfall.floating);
    let action = app.gui.dialogs.waterfall_panel.render_body(ui, &cx);
    apply_waterfall_action(app, action);
}

/// Renderuje kafelek waterfall jako odpięte, pływające okno widoku
/// (wzorzec zgodny z `world_map`). Wywoływane z `app.rs`, gdy
/// `panel_waterfall.floating` jest włączone.
pub fn render_waterfall_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.views.panel_waterfall.visible || !app.views.panel_waterfall.floating {
        return;
    }

    let mut dock_back = false;
    let mut still_open = true;

    let ((), captured_geo) = app.show_floating_viewport(
        ctx,
        egui::ViewportId::from_hash_of("waterfall_viewport"),
        format!(
            "{} - SPLogbook",
            tr(key::VIEW_PANEL_WATERFALL, app.gui.appearance.language)
        ),
        [720.0, 480.0],
        [480.0, 320.0],
        app.views.panel_waterfall.saved_pos,
        app.views.panel_waterfall.saved_size,
        |app, ui| {
            let lang = app.gui.appearance.language;
            egui::Panel::top("waterfall_vp_bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let palette = ui.palette();
                    if crate::gui::theme::dock_button(
                        ui,
                        &palette,
                        tr(key::WATERFALL_DOCK_BTN, lang),
                        tr(key::WATERFALL_DOCK_TOOLTIP, lang),
                    )
                    .clicked()
                    {
                        dock_back = true;
                    }
                });
            });
            egui::CentralPanel::default().show(ui, |ui| {
                let cx = waterfall_context(app, false);
                let action = app.gui.dialogs.waterfall_panel.render_body(ui, &cx);
                apply_waterfall_action(app, action);
            });
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                still_open = false;
            }
        },
    );

    if let Some((pos, size)) = captured_geo
        && (app.views.panel_waterfall.saved_pos != Some(pos)
            || app.views.panel_waterfall.saved_size != Some(size))
    {
        app.views.panel_waterfall.saved_pos = Some(pos);
        app.views.panel_waterfall.saved_size = Some(size);
        if !ctx.input(|i| i.pointer.any_down()) {
            app.save_station_config();
        }
    }

    if dock_back {
        app.dock_tile("waterfall");
        app.save_station_config();
    }

    if !still_open {
        app.views.panel_waterfall.visible = false;
        app.views.panel_waterfall.floating = false;
        app.save_station_config();
    }
}

/// Błędy podsystemu audio wodospadu (Waterfall).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaterfallAudioError {
    ListDevices(String),
    NoDevicesFound,
    DeviceNotFound,
    DefaultConfig(String),
    UnsupportedSampleFormat(String),
    BuildStream(String),
    Playback(String),
}

impl WaterfallAudioError {
    pub fn msg(&self) -> crate::core::i18n::Msg {
        use crate::core::i18n::Msg;
        match self {
            Self::ListDevices(e) => Msg::new(key::WATERFALL_ERR_LIST_DEVICES).arg("err", e),
            Self::NoDevicesFound => Msg::new(key::WATERFALL_ERR_NO_DEVICES),
            Self::DeviceNotFound => Msg::new(key::WATERFALL_ERR_DEVICE_NOT_FOUND),
            Self::DefaultConfig(e) => Msg::new(key::WATERFALL_ERR_DEFAULT_CONFIG).arg("err", e),
            Self::UnsupportedSampleFormat(f) => {
                Msg::new(key::WATERFALL_ERR_SAMPLE_FORMAT).arg("format", f)
            }
            Self::BuildStream(e) => Msg::new(key::WATERFALL_ERR_BUILD_STREAM).arg("err", e),
            Self::Playback(e) => Msg::new(key::WATERFALL_ERR_PLAYBACK).arg("err", e),
        }
    }
}

impl std::fmt::Display for WaterfallAudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListDevices(e) => write!(f, "Failed to list input devices: {e}"),
            Self::NoDevicesFound => write!(f, "No input devices found."),
            Self::DeviceNotFound => write!(f, "Input device not found."),
            Self::DefaultConfig(e) => write!(f, "No default input configuration: {e}"),
            Self::UnsupportedSampleFormat(fmt) => write!(f, "Unsupported sample format: {fmt}"),
            Self::BuildStream(e) => write!(f, "Failed to build input stream: {e}"),
            Self::Playback(e) => write!(f, "Stream playback error: {e}"),
        }
    }
}

impl std::error::Error for WaterfallAudioError {}

/// Wyszukuje urządzenie wejściowe po nazwie (lub domyślne) i zwraca je z konfiguracją.
fn pick_input_device(
    name: Option<&str>,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig), WaterfallAudioError> {
    let host = cpal::default_host();
    let devices: Vec<cpal::Device> = host
        .input_devices()
        .map_err(|e| WaterfallAudioError::ListDevices(e.to_string()))?
        .collect();
    if devices.is_empty() {
        return Err(WaterfallAudioError::NoDevicesFound);
    }

    let by_name = name
        .filter(|n| !n.is_empty())
        .and_then(|n| devices.iter().find(|d| d.to_string() == n).cloned());

    let device = by_name
        .or_else(|| host.default_input_device())
        .or_else(|| devices.into_iter().next())
        .ok_or(WaterfallAudioError::DeviceNotFound)?;

    let config = device
        .default_input_config()
        .map_err(|e| WaterfallAudioError::DefaultConfig(e.to_string()))?;
    Ok((device, config))
}

/// Buduje strumień wejściowy cpal dla wybranego urządzenia i formatu próbek.
fn start_audio_stream(
    device_name: Option<&str>,
    ring: Arc<SampleRing>,
) -> Result<cpal::Stream, WaterfallAudioError> {
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
        other => Err(WaterfallAudioError::UnsupportedSampleFormat(format!(
            "{other:?}"
        ))),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn stream_err_fn(err: cpal::Error) {
    log::error!("Audio stream error: {err}");
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    ring: Arc<SampleRing>,
) -> Result<cpal::Stream, WaterfallAudioError>
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
        .map_err(|e| WaterfallAudioError::BuildStream(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(w, h))
    }

    #[test]
    fn layout_splits_about_30_70_and_scales_with_size() {
        for height in [320.0_f32, 500.0, 900.0] {
            let layout = plot_layout(rect(600.0, height), AXIS_HEIGHT);
            let usable = height - AXIS_HEIGHT;
            let ratio = layout.spectrum.height() / usable;
            assert!(
                (0.25..=0.40).contains(&ratio),
                "wysokość {height}: udział widma {ratio}"
            );
            assert!(layout.waterfall.height() > layout.spectrum.height());
            assert_eq!(layout.spectrum.bottom(), layout.axis.top());
            assert_eq!(layout.axis.bottom(), layout.waterfall.top());
            assert_eq!(layout.waterfall.bottom(), height + 20.0);
        }
        // Bardzo niski panel: wodospad nadal ma miejsce.
        let tiny = plot_layout(rect(300.0, 160.0), AXIS_HEIGHT);
        assert!(tiny.waterfall.height() >= 40.0 - 1e-3);
    }

    #[test]
    fn frequency_axis_round_trips_and_picks_a_readable_step() {
        let area = rect(600.0, 300.0);
        for hz in [0.0_f32, 500.0, 1500.0, 3000.0] {
            let x = hz_to_x(area, hz, 3000.0);
            assert!((x_to_hz(area, x, 3000.0) - hz).abs() < 1e-2);
        }
        assert_eq!(x_to_hz(area, -50.0, 3000.0), 0.0, "obcięcie lewej krawędzi");
        assert_eq!(x_to_hz(area, 9999.0, 3000.0), 3000.0);
        // 600 px na 3 kHz: 500 Hz = 100 px >= 56, a 200 Hz = 40 px < 56.
        assert_eq!(axis_step_hz(600.0, 3000.0), 500.0);
        assert_eq!(axis_step_hz(300.0, 3000.0), 1000.0);
        assert!(axis_step_hz(0.0, 3000.0) > 0.0);
        assert_eq!(
            view_options(24_000.0),
            vec![3000.0, 6000.0, 12_000.0, 24_000.0]
        );
        assert_eq!(view_options(4_000.0), vec![3000.0]);
        assert_eq!(view_options(1_000.0), vec![1000.0]);
    }

    #[test]
    fn display_norm_is_safe_for_non_finite_input() {
        assert_eq!(display_norm(f32::NAN, -100.0, 1.0), 0.0);
        assert_eq!(display_norm(-100.0, -100.0, 1.0), 0.0);
        assert_eq!(display_norm(0.0, -100.0, 1.0), 1.0);
        assert_eq!(display_norm(-50.0, f32::NAN, f32::NAN), 0.5);
        assert!((0.0..=1.0).contains(&display_norm(500.0, -100.0, 3.0)));
    }

    #[test]
    fn texture_is_rebuilt_only_when_key_changes() {
        let key = TextureKey {
            generation: 5,
            palette: WaterfallPalette::Classic,
            floor_bits: (-80.0_f32).to_bits(),
            scale_bits: 1.0_f32.to_bits(),
            gain_bits: 0.0_f32.to_bits(),
            bins: 512,
        };
        assert!(texture_needs_rebuild(None, &key));
        assert!(!texture_needs_rebuild(Some(&key), &key));
        for changed in [
            TextureKey {
                generation: 6,
                ..key
            },
            TextureKey {
                palette: WaterfallPalette::Viridis,
                ..key
            },
            TextureKey {
                floor_bits: (-70.0_f32).to_bits(),
                ..key
            },
            TextureKey {
                scale_bits: 2.0_f32.to_bits(),
                ..key
            },
            TextureKey {
                gain_bits: 3.0_f32.to_bits(),
                ..key
            },
            TextureKey { bins: 1024, ..key },
        ] {
            assert!(texture_needs_rebuild(Some(&key), &changed));
        }
    }

    fn sine_samples(count: usize) -> Vec<f32> {
        (0..count)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 1500.0 / 8000.0).sin() * 0.5)
            .collect()
    }

    fn screen(width: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 700.0))
    }

    fn render_frame(
        ctx: &egui::Context,
        panel: &mut WaterfallPanel,
        cx: &WaterfallContext,
        events: Vec<egui::Event>,
    ) -> Option<WaterfallAction> {
        let raw_input = egui::RawInput {
            screen_rect: Some(screen(900.0)),
            events,
            ..Default::default()
        };
        let mut action = None;
        let mut output = ctx.run_ui(raw_input, |ui| {
            action = panel.render_body(ui, cx);
        });
        output.textures_delta.clear();
        action
    }

    fn test_context() -> WaterfallContext {
        WaterfallContext {
            lang: crate::core::i18n::Language::Pl,
            vfo_hz: 14_074_000,
            mode: "USB".to_string(),
            cat_connected: false,
            docked: true,
        }
    }

    fn panel_with_data() -> WaterfallPanel {
        let mut panel = WaterfallPanel::new();
        panel.devices_loaded = true; // bez dotykania sprzętu audio w testach
        panel.fft_idx = 0; // 512, zgodnie z silnikiem poniżej (inaczej sync_engine go odbuduje)
        let mut engine = WaterfallEngine::new(512, 8_000, HISTORY_DEPTH);
        engine.feed(&sine_samples(512 * 6));
        panel.auto.update(engine.latest_row().unwrap());
        panel.engine = Some(engine);
        panel
    }

    #[test]
    fn panel_renders_in_all_themes_with_and_without_data() {
        use crate::gui::theme::ThemePreset;
        let cx = test_context();
        for preset in ThemePreset::ALL {
            let ctx = egui::Context::default();
            crate::gui::theme::ensure_applied(&ctx, preset, 1.0);

            let mut empty = WaterfallPanel::new();
            empty.devices_loaded = true;
            render_frame(&ctx, &mut empty, &cx, Vec::new());
            render_frame(&ctx, &mut empty, &cx, Vec::new());

            let mut full = panel_with_data();
            render_frame(&ctx, &mut full, &cx, Vec::new());
            render_frame(&ctx, &mut full, &cx, Vec::new());
            assert!(full.texture.is_some(), "{preset:?}: tekstura wodospadu");
        }
    }

    #[test]
    fn texture_is_not_rebuilt_without_new_fft_rows() {
        let ctx = egui::Context::default();
        let cx = test_context();
        let mut panel = panel_with_data();
        render_frame(&ctx, &mut panel, &cx, Vec::new());
        let uploads = panel.texture_uploads;
        assert_eq!(uploads, 1, "pierwsza klatka wgrywa teksturę");
        for _ in 0..5 {
            render_frame(&ctx, &mut panel, &cx, Vec::new());
        }
        assert_eq!(
            panel.texture_uploads, uploads,
            "bez nowej ramki FFT: bez przebudowy"
        );

        // Zmiana palety przebudowuje teksturę; nowa ramka FFT także.
        panel.palette = WaterfallPalette::Viridis;
        render_frame(&ctx, &mut panel, &cx, Vec::new());
        assert_eq!(panel.texture_uploads, uploads + 1);
        if let Some(engine) = panel.engine.as_mut() {
            engine.feed(&sine_samples(512));
        }
        render_frame(&ctx, &mut panel, &cx, Vec::new());
        assert_eq!(panel.texture_uploads, uploads + 2);
    }

    #[test]
    fn click_without_cat_moves_the_target_tone_marker_without_error() {
        let ctx = egui::Context::default();
        let cx = test_context();
        let mut panel = panel_with_data();
        render_frame(&ctx, &mut panel, &cx, Vec::new());
        assert_eq!(panel.target_tone_hz, None);

        let pos = egui::pos2(450.0, 400.0);
        let click = |pressed: bool| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        for events in [
            vec![egui::Event::PointerMoved(pos)],
            vec![click(true)],
            vec![click(false)],
        ] {
            let action = render_frame(&ctx, &mut panel, &cx, events);
            assert_eq!(action, None, "CAT niepodłączony: bez komendy dostrojenia");
        }
        assert!(
            panel.target_tone_hz.is_some(),
            "znacznik tonu przesunięty kliknięciem"
        );
    }
}
