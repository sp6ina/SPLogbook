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
            let t = i as f32 / denom;
            0.5 - 0.5 * (2.0 * std::f32::consts::PI * t).cos()
        })
        .collect()
}

/// Przelicza moc widmową (kwadrat amplitudy) na decybele w skali FS,
/// ograniczając wynik od dołu wartością `floor_db`.
pub fn power_to_dbfs(power: f32, floor_db: f32) -> f32 {
    let db = 10.0 * (power + 1e-12).log10();
    db.max(floor_db)
}

/// Oblicza widmo mocy sygnału rzeczywistego (dBFS) z użyciem okna Hanna i FFT.
///
/// Zwraca `fft_size / 2` pasm (od DC do Nyquista). Pełnowymiarowa sinusoida
/// o amplitudzie 1.0 odpowiada ~0 dBFS.
pub fn spectrum_dbfs(samples: &[f32], fft_size: usize, floor_db: f32) -> Vec<f32> {
    let window = hann_window(fft_size);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut buf: Vec<Complex<f32>> = (0..fft_size)
        .map(|i| {
            let s = samples.get(i).copied().unwrap_or(0.0);
            Complex::new(s * window[i], 0.0)
        })
        .collect();
    fft.process(&mut buf);

    let norm = 2.0 / fft_size as f32;
    let bins = fft_size / 2;
    (0..bins)
        .map(|i| {
            let c = buf[i];
            let mag = (c.re * c.re + c.im * c.im).sqrt() * norm;
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
        let mut q = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for &s in samples {
            if q.len() >= self.capacity {
                q.pop_front();
            }
            q.push_back(s);
        }
    }

    /// Przenosi wszystkie zgromadzone próbki do `out` (w kolejności chronologicznej).
    pub fn drain(&self, out: &mut Vec<f32>) {
        let mut q = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        out.extend(q.drain(..));
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
}

impl WaterfallEngine {
    pub fn new(fft_size: usize, sample_rate: u32, history_depth: usize) -> Self {
        debug_assert!(
            fft_size >= 2 && fft_size.is_power_of_two(),
            "FFT size must be a power of two >= 2"
        );
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        Self {
            fft_size,
            sample_rate,
            window: hann_window(fft_size),
            fft,
            scratch: vec![Complex::new(0.0, 0.0); fft_size],
            history: VecDeque::with_capacity(history_depth.max(1)),
            history_depth: history_depth.max(1),
            gain_db: 0.0,
            floor_db: -100.0,
            pending: Vec::with_capacity(fft_size * 2),
        }
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
    }

    /// Dokłada próbki i wytwarza nowe wiersze widma, gdy zgromadzi się pełny blok FFT.
    pub fn feed(&mut self, samples: &[f32]) {
        self.pending.extend_from_slice(samples);
        while self.pending.len() >= self.fft_size {
            let block: Vec<f32> = self.pending.drain(..self.fft_size).collect();
            let row = self.compute_row(&block);
            if self.history.len() == self.history_depth {
                self.history.pop_front();
            }
            self.history.push_back(row);
        }
    }

    fn compute_row(&mut self, samples: &[f32]) -> Vec<f32> {
        for (dst, (&sample, &win)) in self.scratch[..self.fft_size]
            .iter_mut()
            .zip(samples.iter().zip(&self.window))
        {
            *dst = Complex::new(sample * win, 0.0);
        }
        self.fft.process(&mut self.scratch);

        let norm = 2.0 / self.fft_size as f32;
        let bins = self.fft_size / 2;
        (0..bins)
            .map(|i| {
                let c = self.scratch[i];
                let mag = (c.re * c.re + c.im * c.im).sqrt() * norm;
                let db = power_to_dbfs(mag * mag, self.floor_db) + self.gain_db;
                db.max(self.floor_db)
            })
            .collect()
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
}
