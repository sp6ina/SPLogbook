// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Silnik estymacji propagacji fal krótkich (HF Propagation Engine)
//! Wzorowany na uproszczonych algorytmach VOACAP / ITU-R P.533
//! Szacuje: MUF (Maximum Usable Frequency), LUF (Lowest Usable Frequency),
//! niezawodność łączności (Reliability %), przewidywany poziom sygnału (S-meter)
//! oraz rodzaj warstwy jonosferycznej (F2, Es, D-Absorbed, Groundwave).

use crate::core::geo::{calculate_bearing_deg, calculate_distance_km, locator_to_coordinates, Coordinates};
use serde::{Deserialize, Serialize};

/// Status otwarcia pasma
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BandOpeningStatus {
    Open,
    Marginal,
    Closed,
}

impl BandOpeningStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "Otwarte",
            Self::Marginal => "Trudne",
            Self::Closed => "Zamknięte",
        }
    }
}

/// Wynik prognozy propagacyjnej dla określonej trasy i pasma
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropagationForecast {
    /// Niezawodność obwodu radiowego w % (0-100%)
    pub reliability_pct: u8,
    /// Szacowany poziom sygnału na S-metrze (np. "S9", "S7", "S3", "< S1")
    pub signal_s_units: String,
    /// Warstwa odbijająca lub dominujący mechanizm (np. "F2 (1-hop)", "F2 (multi-hop)", "D-Absorbed", "Przyziemna")
    pub layer: String,
    /// Szacowana maksymalna częstotliwość użytkowa w MHz (MUF)
    pub muf_mhz: f64,
    /// Szacowana najniższa częstotliwość użytkowa w MHz (LUF)
    pub luf_mhz: f64,
    /// Optymalna częstotliwość robocza w MHz (OWF/FOT = 0.85 * MUF)
    pub fot_mhz: f64,
    /// Długość trasy wielkiego koła w kilometrach
    pub distance_km: f64,
    /// Azymut Short Path w stopniach
    pub bearing_deg: f64,
    /// Status otwarcia pasma
    pub status: BandOpeningStatus,
}

/// Zwraca przybliżoną częstotliwość środkową pasma w MHz
pub fn band_to_center_mhz(band: &str) -> Option<f64> {
    match band.trim().to_uppercase().as_str() {
        "160M" => Some(1.85),
        "80M" => Some(3.65),
        "60M" => Some(5.35),
        "40M" => Some(7.10),
        "30M" => Some(10.125),
        "20M" => Some(14.175),
        "17M" => Some(18.118),
        "15M" => Some(21.225),
        "12M" => Some(24.940),
        "10M" => Some(28.500),
        "6M" => Some(50.150),
        _ => None,
    }
}

/// Standardowe pasma HF używane do generowania matrycy propagacyjnej
pub const HF_BANDS: &[(&str, f64)] = &[
    ("160m", 1.85),
    ("80m", 3.65),
    ("60m", 5.35),
    ("40m", 7.10),
    ("30m", 10.125),
    ("20m", 14.175),
    ("17m", 18.118),
    ("15m", 21.225),
    ("12m", 24.940),
    ("10m", 28.500),
    ("6m", 50.150),
];

/// Główny silnik kalkulacji propagacyjnej VOACAP-lite
pub struct PropagationEngine;

impl PropagationEngine {
    /// Oblicza prognozę propagacyjną między dwoma lokatorami Maidenhead
    pub fn forecast(
        origin_grid: &str,
        dest_grid: &str,
        band: &str,
        sfi: u32,
        k_index: u8,
        utc_hour: f64,
        day_of_year: u32,
    ) -> Result<PropagationForecast, &'static str> {
        let origin = locator_to_coordinates(origin_grid)?;
        let dest = locator_to_coordinates(dest_grid)?;
        let freq_mhz = band_to_center_mhz(band).unwrap_or(14.175);

        Ok(Self::calculate(origin, dest, freq_mhz, sfi, k_index, utc_hour, day_of_year))
    }

    /// Oblicza prognozę na podstawie bezpośrednich współrzędnych i częstotliwości
    pub fn calculate(
        origin: Coordinates,
        dest: Coordinates,
        freq_mhz: f64,
        sfi: u32,
        k_index: u8,
        utc_hour: f64,
        day_of_year: u32,
    ) -> PropagationForecast {
        let distance_km = calculate_distance_km(origin, dest);
        let bearing_deg = calculate_bearing_deg(origin, dest);

        // 1. Wyznaczenie punktu środkowego trasy (ionospheric reflection midpoint)
        let mid_lat = (origin.latitude + dest.latitude) / 2.0;
        let mut mid_lon = (origin.longitude + dest.longitude) / 2.0;
        if (origin.longitude - dest.longitude).abs() > 180.0 {
            mid_lon = (mid_lon + 180.0) % 360.0 - 180.0;
        }

        // 2. Deklinacja słońca i wysokość słońca w punkcie środkowym trasy
        let solar_declination_deg = -23.44 * ((2.0 * std::f64::consts::PI / 365.0) * (day_of_year as f64 + 10.0)).cos();
        let solar_dec_rad = solar_declination_deg.to_radians();
        let mid_lat_rad = mid_lat.to_radians();

        // Kąt godzinowy słońca w południku środkowym
        let solar_time_hours = (utc_hour + mid_lon / 15.0).rem_euclid(24.0);
        let hour_angle_rad = ((solar_time_hours - 12.0) * 15.0).to_radians();

        // Sinus wysokości słońca (sin_elevation)
        let sin_elevation = mid_lat_rad.sin() * solar_dec_rad.sin() + mid_lat_rad.cos() * solar_dec_rad.cos() * hour_angle_rad.cos();
        let is_daylight = sin_elevation > 0.0;
        let day_factor = sin_elevation.max(0.0);

        // 3. Krytyczna częstotliwość warstwy F2 (foF2)
        let sfi_f64 = sfi.clamp(60, 300) as f64;
        let base_fof2 = if is_daylight {
            // W dzień foF2 rośnie z SFI oraz kątem padania promieni słonecznych
            0.65 * sfi_f64.sqrt() * day_factor.powf(0.25) + 3.2
        } else {
            // W nocy foF2 opada do wartości bazowej zależnej od aktywności słonecznej
            2.2 + 0.28 * sfi_f64.sqrt()
        };

        // Redukcja foF2 podczas burzy geomagnetycznej (wysoki indeks K)
        let storm_penalty = if k_index >= 4 {
            1.0 - (k_index - 3) as f64 * 0.08
        } else {
            1.0
        };
        let fof2 = (base_fof2 * storm_penalty).max(2.0);

        // 4. Współczynnik skośnego padania M(d) zależny od odległości hopa
        // Dla dystansów do 3500 km (1 hop F2) M wynosi od 1.1 do ~3.2
        let hop_factor = (distance_km / 3500.0).clamp(0.05, 1.0);
        let m_factor = 1.1 + 2.1 * hop_factor;
        let muf_mhz = (fof2 * m_factor).min(65.0);
        let fot_mhz = muf_mhz * 0.85;

        // 5. Najniższa częstotliwość użytkowa (LUF) - pochłanianie w warstwie D
        // W ciągu dnia niskie częstotliwości (160m, 80m, 40m) są silnie tłumione
        let luf_mhz = if is_daylight {
            (6.5 * day_factor.powf(0.5) * (sfi_f64 / 100.0).powf(0.3)).max(2.0)
        } else {
            1.6
        };

        // 6. Mechanizm propagacji
        let (layer, path_loss_db) = if distance_km < 80.0 {
            ("Przyziemna (Groundwave)".to_string(), 75.0)
        } else if distance_km < 3500.0 {
            ("F2 (1-hop)".to_string(), 95.0 + distance_km * 0.008)
        } else if distance_km < 7000.0 {
            ("F2 (2-hop)".to_string(), 115.0 + distance_km * 0.007)
        } else {
            ("F2 (multi-hop)".to_string(), 130.0 + distance_km * 0.006)
        };

        // 7. Obliczenie Niezawodności Obwodu (Reliability %)
        let mut rel = 0.0;
        if freq_mhz >= luf_mhz && freq_mhz <= muf_mhz {
            // W optymalnym oknie pracy FOT/OWF
            let diff_from_fot = (freq_mhz - fot_mhz).abs() / fot_mhz;
            let base_rel = (1.0 - diff_from_fot * 0.6).clamp(0.4, 0.98);
            rel = base_rel * 100.0;
        } else if freq_mhz > muf_mhz {
            // Powyżej MUF sygnał ucieka w kosmos (chyba że Sporadic-E w lecie na 10m/6m)
            let over_ratio = freq_mhz / muf_mhz;
            if over_ratio < 1.15 {
                rel = (1.15 - over_ratio) / 0.15 * 35.0;
            } else {
                rel = 0.0;
            }
        } else if freq_mhz < luf_mhz {
            // Poniżej LUF - tłumienie w warstwie D
            let under_ratio = freq_mhz / luf_mhz;
            rel = (under_ratio * 40.0).max(5.0);
        }

        // Degradacja geomagnetyczna K-index
        if k_index >= 4 {
            let k_pen = (k_index - 3) as f64 * 12.0;
            rel = (rel - k_pen).max(0.0);
        }
        let reliability_pct = rel.round().clamp(0.0, 100.0) as u8;

        // 8. Szacowanie sygnału na S-metrze
        // Na podstawie tłumienia trasy, D-layer absorption oraz SFI
        let absorption_db = if is_daylight && freq_mhz < 15.0 {
            (15.0 - freq_mhz) * 3.5 * day_factor
        } else {
            0.0
        };
        let total_loss = path_loss_db + absorption_db;

        let signal_s_units = if reliability_pct < 10 {
            "< S1".to_string()
        } else if total_loss < 95.0 {
            "S9+".to_string()
        } else if total_loss < 105.0 {
            "S9".to_string()
        } else if total_loss < 115.0 {
            "S7".to_string()
        } else if total_loss < 125.0 {
            "S5".to_string()
        } else if total_loss < 135.0 {
            "S3".to_string()
        } else {
            "S1".to_string()
        };

        // 9. Status otwarcia pasma
        let status = if reliability_pct >= 60 {
            BandOpeningStatus::Open
        } else if reliability_pct >= 25 {
            BandOpeningStatus::Marginal
        } else {
            BandOpeningStatus::Closed
        };

        PropagationForecast {
            reliability_pct,
            signal_s_units,
            layer,
            muf_mhz: (muf_mhz * 10.0).round() / 10.0,
            luf_mhz: (luf_mhz * 10.0).round() / 10.0,
            fot_mhz: (fot_mhz * 10.0).round() / 10.0,
            distance_km: distance_km.round(),
            bearing_deg: bearing_deg.round(),
            status,
        }
    }

    /// Generuje prognozę dla wszystkich popularnych pasm HF
    pub fn forecast_all_bands(
        origin_grid: &str,
        dest_grid: &str,
        sfi: u32,
        k_index: u8,
        utc_hour: f64,
        day_of_year: u32,
    ) -> Result<Vec<(&'static str, PropagationForecast)>, &'static str> {
        let origin = locator_to_coordinates(origin_grid)?;
        let dest = locator_to_coordinates(dest_grid)?;

        let results = HF_BANDS
            .iter()
            .map(|&(b_name, freq)| {
                (
                    b_name,
                    Self::calculate(origin, dest, freq, sfi, k_index, utc_hour, day_of_year),
                )
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagation_calculation_eu_to_us() {
        // Wrocław (JO81) do Nowy Jork (FN30)
        let sp = Coordinates::new(51.1079, 17.0385);
        let us = Coordinates::new(40.7128, -74.0060);

        let forecast_20m = PropagationEngine::calculate(sp, us, 14.175, 150, 2, 14.0, 100);
        assert!(forecast_20m.distance_km > 6500.0);
        assert!(forecast_20m.muf_mhz > 14.0, "W południe przy SFI 150 MUF powinien przekraczać 14 MHz");
        assert!(forecast_20m.reliability_pct > 30, "Pasmo 20m powinno być otwarte lub częściowo otwarte");
    }

    #[test]
    fn test_day_night_d_layer_absorption() {
        // 80m w południe powinno mieć wysoki LUF i silne tłumienie w dzień
        let sp = Coordinates::new(51.1079, 17.0385);
        let dl = Coordinates::new(52.5200, 13.4050);

        let noon = PropagationEngine::calculate(sp, dl, 3.65, 120, 1, 12.0, 170);
        let midnight = PropagationEngine::calculate(sp, dl, 3.65, 120, 1, 0.0, 170);

        assert!(noon.luf_mhz > midnight.luf_mhz, "LUF w południe powinien być wyższy niż o północy");
        assert!(midnight.reliability_pct >= noon.reliability_pct, "80m w nocy powinno mieć lepszą niezawodność niż w dzień");
    }

    #[test]
    fn test_band_opening_status_string() {
        assert_eq!(BandOpeningStatus::Open.as_str(), "Otwarte");
        assert_eq!(BandOpeningStatus::Marginal.as_str(), "Trudne");
        assert_eq!(BandOpeningStatus::Closed.as_str(), "Zamknięte");
    }
}
