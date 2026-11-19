// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! DSP dla panelu widma / waterfallu (SDR).
//!
//! Moduł jest niezależny od sprzętu audio — zawiera okno Hanna, transformację
//! FFT z normalizacją do dBFS, silnik akumulujący kolejne wiersze widma oraz
//! współdzielony bufor próbek wypełniany przez wątek przechwytujący dźwięk.

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Okno Hanna (Hanna) o zadanej długości. Wartości z zakresu [0, 1].
pub fn hann_window(size: usize) -> Vec<f32> {
    if size == 0 {
        return Vec::new();
    }
    if size == 1 {
        return vec![1.0];
    }
    let denom = (size - 1) as f32;
    (0..size)
        .map(|i| {
            let frac = i as f32 / denom;
            0.5 - 0.5 * (2.0 * std::f32::consts::PI * frac).cos()
        })
        .collect()
}

/// Przelicza moc widmową (kwadrat amplitudy) na decybele w skali FS,
/// ograniczając wynik od dołu wartością `floor_db`.
pub fn power_to_dbfs(power: f32, floor_db: f32) -> f32 {
    if !power.is_finite() || power < 0.0 {
        return floor_db;
    }
    let db = 10.0 * (power + 1e-12).log10();
    db.max(floor_db)
}

/// Oblicza widmo mocy sygnału rzeczywistego (dBFS) z użyciem okna Hanna i FFT.
///
/// Zwraca `fft_size / 2` pasm (od DC do Nyquista). Pełnowymiarowa sinusoida
/// o amplitudzie 1.0 odpowiada ~0 dBFS.
pub fn spectrum_dbfs(samples: &[f32], fft_size: usize, floor_db: f32) -> Vec<f32> {
    if fft_size < 2 {
        return Vec::new();
    }
    let window = hann_window(fft_size);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut buf: Vec<Complex<f32>> = (0..fft_size)
        .map(|i| {
            let sample = samples.get(i).copied().unwrap_or(0.0);
            Complex::new(sample * window[i], 0.0)
        })
        .collect();
    fft.process(&mut buf);

    let norm = 2.0 / fft_size as f32;
    let bins = fft_size / 2;
    (0..bins)
        .map(|i| {
            let complex = buf[i];
            let mag = (complex.re * complex.re + complex.im * complex.im).sqrt() * norm;
            power_to_dbfs(mag * mag, floor_db)
        })
        .collect()
}

/// Współdzielony bufor pierścieniowy próbek audio.
///
/// Wypełniany przez wątek przechwytujący dźwięk (cpal), a opróżniany przez
/// wątek GUI, który karmi nim silnik widma.
pub struct SampleRing {
    inner: Mutex<VecDeque<f32>>,
    capacity: usize,
}

impl SampleRing {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(VecDeque::with_capacity(capacity.max(1))),
            capacity: capacity.max(1),
        })
    }

    /// Dodaje próbki; nadmiarowe najstarsze próbki są odrzucane.
    pub fn push(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut queue = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for &s in samples {
            if queue.len() >= self.capacity {
                queue.pop_front();
            }
            queue.push_back(s);
        }
    }

    /// Przenosi wszystkie zgromadzone próbki do `out` (w kolejności chronologicznej).
    pub fn drain(&self, out: &mut Vec<f32>) {
        let mut queue = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        out.extend(queue.drain(..));
    }

    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Silnik widma: akumuluje próbki, liczy kolejne wiersze FFT i prowadzi
/// historię (scrolling waterfall).
pub struct WaterfallEngine {
    fft_size: usize,
    sample_rate: u32,
    window: Vec<f32>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    history: VecDeque<Vec<f32>>,
    history_depth: usize,
    gain_db: f32,
    floor_db: f32,
    pending: Vec<f32>,
    /// Licznik generacji: rośnie przy każdej nowej linii FFT oraz przy `clear()`, dzięki czemu
    /// GUI przebudowuje teksturę wyłącznie wtedy, gdy dane faktycznie się zmieniły.
    generation: u64,
}

impl WaterfallEngine {
    pub fn new(fft_size: usize, sample_rate: u32, history_depth: usize) -> Self {
        let fft_size = fft_size.max(2).next_power_of_two();
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        Self {
            fft_size,
            sample_rate: sample_rate.max(1),
            window: hann_window(fft_size),
            fft,
            scratch: vec![Complex::new(0.0, 0.0); fft_size],
            history: VecDeque::with_capacity(history_depth.max(1)),
            history_depth: history_depth.max(1),
            gain_db: 0.0,
            floor_db: -100.0,
            pending: Vec::with_capacity(fft_size * 2),
            generation: 0,
        }
    }

    /// Licznik generacji danych widma (zmienia się przy nowej linii FFT i po `clear()`).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Szerokość pojedynczego pasma w Hz.
    pub fn bin_hz(&self) -> f32 {
        self.sample_rate as f32 / self.fft_size as f32
    }

    pub fn set_gain_db(&mut self, db: f32) {
        self.gain_db = db;
    }

    pub fn set_floor_db(&mut self, db: f32) {
        self.floor_db = db;
    }

    pub fn gain_db(&self) -> f32 {
        self.gain_db
    }

    pub fn floor_db(&self) -> f32 {
        self.floor_db
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.pending.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Dokłada próbki i wytwarza nowe wiersze widma, gdy zgromadzi się pełny blok FFT.
    pub fn feed(&mut self, samples: &[f32]) {
        self.pending.extend_from_slice(samples);
        let mut offset = 0;
        while offset + self.fft_size <= self.pending.len() {
            for (dst, (&sample, &win)) in self.scratch[..self.fft_size].iter_mut().zip(
                self.pending[offset..offset + self.fft_size]
                    .iter()
                    .zip(&self.window),
            ) {
                *dst = Complex::new(sample * win, 0.0);
            }
            self.fft.process(&mut self.scratch);

            let norm = 2.0 / self.fft_size as f32;
            let bins = self.fft_size / 2;
            let row: Vec<f32> = (0..bins)
                .map(|i| {
                    let complex = self.scratch[i];
                    let mag = (complex.re * complex.re + complex.im * complex.im).sqrt() * norm;
                    let db = power_to_dbfs(mag * mag, self.floor_db) + self.gain_db;
                    db.max(self.floor_db)
                })
                .collect();

            if self.history.len() == self.history_depth {
                self.history.pop_front();
            }
            self.history.push_back(row);
            self.generation = self.generation.wrapping_add(1);
            offset += self.fft_size;
        }
        if offset > 0 {
            self.pending.drain(..offset);
        }
    }

    /// Najnowszy wiersz widma (dBFS), jeśli istnieje.
    pub fn latest_row(&self) -> Option<&[f32]> {
        self.history.back().map(std::vec::Vec::as_slice)
    }

    /// Historia wierszy (najstarszy na początku, najnowszy na końcu).
    pub fn history(&self) -> &VecDeque<Vec<f32>> {
        &self.history
    }
}

// ---------------------------------------------------------------------------------------
// Częstotliwość: kosze FFT <-> Hz, audio (AF) <-> radio (RF), Click-to-QSY
// ---------------------------------------------------------------------------------------

/// Częstotliwość środka kosza `bin` (Hz).
#[must_use]
pub fn bin_to_hz(bin: usize, bin_hz: f32) -> f32 {
    bin as f32 * bin_hz
}

/// Najbliższy kosz dla częstotliwości `hz`, ograniczony do `0..bins`.
#[must_use]
pub fn hz_to_bin(hz: f32, bin_hz: f32, bins: usize) -> usize {
    if bins == 0 || !hz.is_finite() || bin_hz <= 0.0 {
        return 0;
    }
    ((hz / bin_hz).round().max(0.0) as usize).min(bins - 1)
}

/// Strona wstęgi / sposób przeliczania AF <-> RF dla trybu pracy radia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sideband {
    /// USB, DIGU, PKTUSB, RTTY: `RF = VFO + AF`.
    Upper,
    /// LSB, DIGL, PKTLSB, RTTYR: `RF = VFO - AF`.
    Lower,
    /// CW: `RF = VFO + (AF - ton)`.
    Cw,
    /// CWR: `RF = VFO - (AF - ton)`.
    CwReverse,
}

/// Wstęga dla trybu (nazwy jak w Hamlib; nieznane tryby traktujemy jak USB).
#[must_use]
pub fn sideband_for_mode(mode: &str) -> Sideband {
    match mode.trim().to_ascii_uppercase().as_str() {
        "LSB" | "DIGL" | "PKTLSB" | "RTTYR" => Sideband::Lower,
        "CW" => Sideband::Cw,
        "CWR" => Sideband::CwReverse,
        _ => Sideband::Upper,
    }
}

/// Domyślny ton odniesienia (Hz): ton CW dla CW, 1500 Hz dla pozostałych trybów.
#[must_use]
pub fn default_tone_hz(mode: &str, cw_pitch_hz: f64) -> f64 {
    match sideband_for_mode(mode) {
        Sideband::Cw | Sideband::CwReverse => cw_pitch_hz,
        _ => 1500.0,
    }
}

/// Częstotliwość radiowa (RF, Hz) sygnału o częstotliwości audio `af_hz` przy VFO `vfo_hz`.
#[must_use]
pub fn af_to_rf_hz(vfo_hz: f64, af_hz: f64, mode: &str, cw_pitch_hz: f64) -> f64 {
    match sideband_for_mode(mode) {
        Sideband::Upper => vfo_hz + af_hz,
        Sideband::Lower => vfo_hz - af_hz,
        Sideband::Cw => vfo_hz + (af_hz - cw_pitch_hz),
        Sideband::CwReverse => vfo_hz - (af_hz - cw_pitch_hz),
    }
}

/// Przesunięcie VFO (Hz) potrzebne, aby sygnał z `clicked_af_hz` trafił na ton `target_tone_hz`:
/// `+delta` dla USB/DIGU/CW, `-delta` dla LSB/DIGL (delta = kliknięta AF - ton docelowy).
#[must_use]
pub fn click_qsy_delta_hz(clicked_af_hz: f64, target_tone_hz: f64, mode: &str) -> f64 {
    let delta = clicked_af_hz - target_tone_hz;
    match sideband_for_mode(mode) {
        Sideband::Upper | Sideband::Cw => delta,
        Sideband::Lower | Sideband::CwReverse => -delta,
    }
}

// ---------------------------------------------------------------------------------------
// Peak Hold i Auto-Level
// ---------------------------------------------------------------------------------------

/// Linia Peak Hold: śledzi maksima kosz po koszu i powoli opada.
#[derive(Debug, Default, Clone)]
pub struct PeakHold {
    values: Vec<f32>,
}

impl PeakHold {
    /// Aktualizuje maksima nowym wierszem widma; każdy kosz opada o `decay_db` na wiersz.
    pub fn update(&mut self, row: &[f32], decay_db: f32) {
        if self.values.len() != row.len() {
            self.values = row.to_vec();
            return;
        }
        let decay = decay_db.max(0.0);
        for (peak, &now) in self.values.iter_mut().zip(row) {
            let held = *peak - decay;
            *peak = if now.is_finite() && now > held {
                now
            } else {
                held
            };
        }
    }

    pub fn reset(&mut self) {
        self.values.clear();
    }

    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }
}

/// Percentyl `p` (0..=1) wartości skończonych; `None`, gdy brak danych.
#[must_use]
pub fn percentile_db(values: &[f32], p: f32) -> Option<f32> {
    let mut finite: Vec<f32> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return None;
    }
    finite.sort_by(f32::total_cmp);
    let idx = ((finite.len() - 1) as f32 * p.clamp(0.0, 1.0)).round() as usize;
    finite.get(idx).copied()
}

/// Auto-Level: poziom tła szumu z dolnego percentyla wiersza FFT, wygładzony wykładniczo (EMA).
/// Pojedynczy silny sygnał nie zawyża progu, bo percentyl ignoruje górę rozkładu.
#[derive(Debug, Clone)]
pub struct AutoLevel {
    ema_noise_db: Option<f32>,
    alpha: f32,
    percentile: f32,
}

impl Default for AutoLevel {
    fn default() -> Self {
        Self {
            ema_noise_db: None,
            alpha: 0.05,
            percentile: 0.20,
        }
    }
}

impl AutoLevel {
    /// Uwzględnia nowy wiersz i zwraca wygładzony poziom szumu (dBFS).
    pub fn update(&mut self, row: &[f32]) -> Option<f32> {
        let noise = percentile_db(row, self.percentile)?;
        let next = match self.ema_noise_db {
            Some(prev) => prev + (noise - prev) * self.alpha,
            None => noise,
        };
        self.ema_noise_db = Some(next);
        self.ema_noise_db
    }

    #[must_use]
    pub fn noise_db(&self) -> Option<f32> {
        self.ema_noise_db
    }

    pub fn reset(&mut self) {
        self.ema_noise_db = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_window_endpoints_near_zero_and_peak_in_middle() {
        let w = hann_window(256);
        assert_eq!(w.len(), 256);
        assert!(w[0].abs() < 1e-6, "lewy brzeg powinien być ~0");
        assert!(w[255].abs() < 1e-6, "prawy brzeg powinien być ~0");
        assert!(w[128] > 0.999, "środek powinien być ~1, jest {}", w[128]);
        for v in &w {
            assert!((0.0..=1.0).contains(v));
        }
    }

    #[test]
    fn hann_window_handles_small_sizes() {
        assert_eq!(hann_window(0), Vec::<f32>::new());
        assert_eq!(hann_window(1), vec![1.0]);
    }

    #[test]
    fn power_to_dbfs_full_scale_is_zero_and_floor_clamps() {
        assert!((power_to_dbfs(1.0, -100.0) - 0.0).abs() < 1e-3);
        assert!((power_to_dbfs(0.0001, -100.0) - (-40.0)).abs() < 0.01);
        assert_eq!(power_to_dbfs(0.0, -90.0), -90.0);
    }

    #[test]
    fn spectrum_dbfs_finds_sine_peak() {
        let fft_size = 1024;
        let f = 64.0; // bin 64
        let signal: Vec<f32> = (0..fft_size)
            .map(|i| (2.0 * std::f32::consts::PI * f * i as f32 / fft_size as f32).sin())
            .collect();
        let spectrum = spectrum_dbfs(&signal, fft_size, -120.0);
        assert_eq!(spectrum.len(), fft_size / 2);
        let peak = spectrum[64];
        let away = spectrum[128];
        assert!(peak > -10.0, "szczyt powinien być silny, jest {peak}");
        assert!(
            peak > away + 30.0,
            "szczyt powinien dominować nad sąsiednim pasmem"
        );
    }

    #[test]
    fn engine_builds_history_and_reports_latest_row() {
        let mut engine = WaterfallEngine::new(256, 48000, 10);
        assert_eq!(engine.bin_hz(), 48000.0 / 256.0);

        // 3 bloki -> 3 wiersze
        let samples: Vec<f32> = (0..(256 * 3)).map(|i| (i as f32 * 0.01).sin()).collect();
        engine.feed(&samples);
        assert_eq!(engine.history().len(), 3);
        let latest = engine.latest_row().unwrap();
        assert_eq!(latest.len(), 128);

        // Limit głębokości historii: po 20 blokach powinno zostać 10.
        engine.clear();
        let many: Vec<f32> = vec![0.0; 256 * 20];
        engine.feed(&many);
        assert_eq!(engine.history().len(), 10);
    }

    #[test]
    fn bin_hz_conversions_round_trip() {
        let engine = WaterfallEngine::new(1024, 48_000, 8);
        let bin_hz = engine.bin_hz();
        assert!((bin_hz - 46.875).abs() < 1e-4);
        assert!((bin_to_hz(32, bin_hz) - 1500.0).abs() < 1e-3);
        assert_eq!(hz_to_bin(1500.0, bin_hz, 512), 32);
        assert_eq!(hz_to_bin(-5.0, bin_hz, 512), 0);
        assert_eq!(
            hz_to_bin(1.0e9, bin_hz, 512),
            511,
            "ograniczone do ostatniego kosza"
        );
        assert_eq!(hz_to_bin(f32::NAN, bin_hz, 512), 0);
        assert_eq!(hz_to_bin(100.0, bin_hz, 0), 0);
    }

    #[test]
    fn af_to_rf_follows_the_sideband() {
        let vfo = 14_074_000.0;
        assert_eq!(af_to_rf_hz(vfo, 1500.0, "USB", 700.0), 14_075_500.0);
        assert_eq!(af_to_rf_hz(vfo, 1500.0, "DIGU", 700.0), 14_075_500.0);
        assert_eq!(af_to_rf_hz(vfo, 1500.0, "RTTY", 700.0), 14_075_500.0);
        assert_eq!(af_to_rf_hz(vfo, 1500.0, "LSB", 700.0), 14_072_500.0);
        assert_eq!(af_to_rf_hz(vfo, 1500.0, "DIGL", 700.0), 14_072_500.0);
        assert_eq!(af_to_rf_hz(vfo, 900.0, "CW", 700.0), 14_074_200.0);
        assert_eq!(af_to_rf_hz(vfo, 900.0, "CWR", 700.0), 14_073_800.0);
        assert_eq!(
            af_to_rf_hz(vfo, 1500.0, "cw", 700.0),
            af_to_rf_hz(vfo, 1500.0, "CW", 700.0)
        );
    }

    #[test]
    fn click_qsy_delta_puts_the_clicked_signal_on_the_target_tone() {
        let vfo = 7_050_000.0;
        for (mode, tone) in [
            ("USB", 1500.0),
            ("LSB", 1500.0),
            ("CW", 700.0),
            ("CWR", 700.0),
        ] {
            let clicked = tone + 230.0;
            let signal_rf = af_to_rf_hz(vfo, clicked, mode, 700.0);
            let new_vfo = vfo + click_qsy_delta_hz(clicked, tone, mode);
            // Po dostrojeniu sygnał ma wypaść dokładnie na tonie docelowym.
            let after = af_to_rf_hz(new_vfo, tone, mode, 700.0);
            assert!(
                (after - signal_rf).abs() < 1e-6,
                "{mode}: {after} vs {signal_rf}"
            );
        }
        assert_eq!(click_qsy_delta_hz(1730.0, 1500.0, "USB"), 230.0);
        assert_eq!(click_qsy_delta_hz(1730.0, 1500.0, "LSB"), -230.0);
        assert_eq!(click_qsy_delta_hz(930.0, 700.0, "CW"), 230.0);
        assert_eq!(default_tone_hz("CW", 650.0), 650.0);
        assert_eq!(default_tone_hz("USB", 650.0), 1500.0);
    }

    #[test]
    fn peak_hold_tracks_maxima_decays_and_resets() {
        let mut peak = PeakHold::default();
        peak.update(&[-60.0, -40.0, -80.0], 1.0);
        assert_eq!(peak.values(), &[-60.0, -40.0, -80.0]);
        peak.update(&[-70.0, -20.0, -90.0], 1.0);
        assert_eq!(peak.values()[1], -20.0, "nowe maksimum przejmuje linię");
        assert_eq!(peak.values()[0], -61.0, "pozostałe opadają o decay");
        assert_eq!(peak.values()[2], -81.0);
        for _ in 0..5 {
            peak.update(&[-120.0, -120.0, -120.0], 1.0);
        }
        assert_eq!(peak.values()[1], -25.0, "stopniowe opadanie");
        peak.update(&[f32::NAN, -120.0, -120.0], 1.0);
        assert!(peak.values()[0].is_finite(), "NaN nie psuje linii");
        peak.reset();
        assert_eq!(peak.values().len(), 0);
    }

    #[test]
    fn auto_level_ignores_a_single_strong_peak() {
        // Szum około -80 dBFS z szumem deterministycznym + jeden silny ton -5 dBFS.
        let mut row: Vec<f32> = (0..512).map(|i| -82.0 + (i % 7) as f32).collect();
        row[100] = -5.0;
        let mut auto = AutoLevel::default();
        let noise = auto.update(&row).expect("poziom");
        assert!(
            (-83.0..=-74.0).contains(&noise),
            "szum {noise} nie może być zawyżony pikiem"
        );
        // EMA: skok poziomu przesuwa próg stopniowo, nie natychmiast.
        let louder: Vec<f32> = row.iter().map(|v| v + 30.0).collect();
        let next = auto.update(&louder).unwrap();
        assert!(next > noise && next < noise + 30.0, "{next}");
        assert!(auto.update(&[]).is_none() || auto.noise_db().is_some());
        let mut empty = AutoLevel::default();
        assert_eq!(empty.update(&[f32::NAN, f32::INFINITY]), None);
        assert_eq!(percentile_db(&[3.0, 1.0, 2.0], 0.0), Some(1.0));
        assert_eq!(percentile_db(&[3.0, 1.0, 2.0], 1.0), Some(3.0));
    }

    #[test]
    fn generation_changes_only_when_new_data_arrives() {
        let mut engine = WaterfallEngine::new(256, 8_000, 8);
        let g0 = engine.generation();
        engine.feed(&vec![0.0; 100]); // za mało na pełny blok FFT
        assert_eq!(
            engine.generation(),
            g0,
            "brak nowej linii = brak zmiany generacji"
        );
        engine.feed(&vec![0.1; 200]); // dopełnia blok (300 >= 256)
        let g1 = engine.generation();
        assert_ne!(g1, g0, "nowa linia FFT zmienia generację");
        engine.feed(&[]);
        assert_eq!(engine.generation(), g1);
        engine.clear();
        assert_ne!(engine.generation(), g1, "clear też unieważnia teksturę");
    }
}
