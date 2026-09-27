// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Prosty predyktor przelotów satelitów LEO oparty o model keplerowski kołowy.
//!
//! Jest to deterministyczny model przybliżony (bez SGP4/TLE): orbita kołowa,
//! stała inklinacja i RAAN, precesja Ziemi wokół osi. Nadaje się do planowania
//! najbliższych przelotów i podglądu Dopplera, ale czasy AOS/LOS mogą różnić się
//! od tych z pełnego propagatora SGP4. Zachowuje spójność wewnętrzną (AOS < LOS,
//! maksymalna elewacja powyżej horyzontu), dzięki czemu jest w pełni testowalny.

use std::f64::consts::PI;

const MU: f64 = 398600.4418; // km^3/s^2 — parametr grawitacyjny Ziemi
const EARTH_RADIUS_KM: f64 = 6371.0;
const EARTH_ROT_RAD_S: f64 = 7.2921159e-5; // rotacja gwiazdowa Ziemi [rad/s]
const C_KM_S: f64 = 299792.458; // prędkość światła [km/s]
const DEG2RAD: f64 = PI / 180.0;
const RAD2DEG: f64 = 180.0 / PI;

/// Obserwator (stacja naziemna) we współrzędnych geograficznych.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observer {
    pub lat_deg: f64,
    pub lon_deg: f64,
}

/// Definicja satelity (orbita kołowa + częstotliwości).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SatelliteDef {
    pub name: &'static str,
    pub altitude_km: f64,
    pub inclination_deg: f64,
    pub raan_deg: f64,
    pub mean_anomaly_deg: f64,
    pub downlink_mhz: f64,
    pub uplink_mhz: f64,
}

/// Pojedynczy przelot nad horyzontem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SatellitePass {
    pub aos_unix: f64,
    pub los_unix: f64,
    pub max_elevation_deg: f64,
    pub aos_azimuth_deg: f64,
    pub aos_doppler_khz: f64,
}

impl SatellitePass {
    /// Czas trwania przelotu w sekundach.
    pub fn duration_secs(&self) -> f64 {
        (self.los_unix - self.aos_unix).max(0.0)
    }
}

/// Domyślna lista satelitów (zgodna z listą w GUI).
pub fn default_satellites() -> Vec<SatelliteDef> {
    vec![
        SatelliteDef {
            name: "ISS (ZARYA)",
            altitude_km: 418.0,
            inclination_deg: 51.64,
            raan_deg: 120.0,
            mean_anomaly_deg: 0.0,
            downlink_mhz: 145.8000,
            uplink_mhz: 437.8000,
        },
        SatelliteDef {
            name: "AO-91 (RadFxSat)",
            altitude_km: 490.0,
            inclination_deg: 97.7,
            raan_deg: 210.0,
            mean_anomaly_deg: 45.0,
            downlink_mhz: 145.9600,
            uplink_mhz: 435.2500,
        },
        SatelliteDef {
            name: "AO-92 (Fox-1D)",
            altitude_km: 500.0,
            inclination_deg: 97.7,
            raan_deg: 300.0,
            mean_anomaly_deg: 90.0,
            downlink_mhz: 145.8800,
            uplink_mhz: 435.3500,
        },
        SatelliteDef {
            name: "SO-50 (Saudisat 1C)",
            altitude_km: 640.0,
            inclination_deg: 64.56,
            raan_deg: 45.0,
            mean_anomaly_deg: 135.0,
            downlink_mhz: 436.7950,
            uplink_mhz: 145.8500,
        },
        SatelliteDef {
            name: "RS-44 (DOSAAF)",
            altitude_km: 1480.0,
            inclination_deg: 82.5,
            raan_deg: 160.0,
            mean_anomaly_deg: 200.0,
            downlink_mhz: 435.6100,
            uplink_mhz: 145.9650,
        },
        SatelliteDef {
            name: "CAS-4A",
            altitude_km: 520.0,
            inclination_deg: 97.5,
            raan_deg: 260.0,
            mean_anomaly_deg: 300.0,
            downlink_mhz: 145.8550,
            uplink_mhz: 435.2200,
        },
        SatelliteDef {
            name: "JO-97 (JY1Sat)",
            altitude_km: 585.0,
            inclination_deg: 97.7,
            raan_deg: 350.0,
            mean_anomaly_deg: 30.0,
            downlink_mhz: 145.8550,
            uplink_mhz: 435.1000,
        },
    ]
}

/// Pozycja satelity w układzie inercjalnym (ECI) w km.
fn eci_position(def: &SatelliteDef, t: f64) -> [f64; 3] {
    let a = EARTH_RADIUS_KM + def.altitude_km;
    let n = (MU / (a * a * a)).sqrt();
    let m = def.mean_anomaly_deg * DEG2RAD + n * t;
    let u = m; // orbita kołowa: anomalia prawdziwa = anomalia średnia
    let i = def.inclination_deg * DEG2RAD;
    let raan = def.raan_deg * DEG2RAD;

    let x = a * u.cos();
    let y = a * u.sin();

    // Obrót o inklinację wokół osi X.
    let xp = x;
    let yp = y * i.cos();
    let zp = y * i.sin();

    // Obrót o RAAN wokół osi Z.
    let x_eci = xp * raan.cos() - yp * raan.sin();
    let y_eci = xp * raan.sin() + yp * raan.cos();
    [x_eci, y_eci, zp]
}

/// Pozycja satelity w układie związanym z Ziemią (ECEF) w km.
fn ecef_position(def: &SatelliteDef, t: f64) -> [f64; 3] {
    let eci = eci_position(def, t);
    let theta = EARTH_ROT_RAD_S * t;
    let (c, s) = (theta.cos(), theta.sin());
    [
        eci[0] * c + eci[1] * s,
        -eci[0] * s + eci[1] * c,
        eci[2],
    ]
}

/// Pozycja obserwatora w ECEF (sferyczna Ziemia).
fn observer_ecef(obs: &Observer) -> [f64; 3] {
    let lat = obs.lat_deg * DEG2RAD;
    let lon = obs.lon_deg * DEG2RAD;
    let r = EARTH_RADIUS_KM;
    [
        r * lat.cos() * lon.cos(),
        r * lat.cos() * lon.sin(),
        r * lat.sin(),
    ]
}

/// Odległość (slant range) obserwator-satelita w km.
pub fn range_km(def: &SatelliteDef, obs: &Observer, t: f64) -> f64 {
    let s = ecef_position(def, t);
    let o = observer_ecef(obs);
    let dx = s[0] - o[0];
    let dy = s[1] - o[1];
    let dz = s[2] - o[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Kąt elewacji satelity nad horyzontem w stopniach (-90..90).
pub fn elevation_deg(def: &SatelliteDef, obs: &Observer, t: f64) -> f64 {
    let s = ecef_position(def, t);
    let o = observer_ecef(obs);
    let r = EARTH_RADIUS_KM;
    let up_hat = [o[0] / r, o[1] / r, o[2] / r];
    let dx = s[0] - o[0];
    let dy = s[1] - o[1];
    let dz = s[2] - o[2];
    let range = (dx * dx + dy * dy + dz * dz).sqrt();
    let up = (dx * up_hat[0] + dy * up_hat[1] + dz * up_hat[2]) / range;
    up.asin() * RAD2DEG
}

/// Azymut satelity w stopniach (0..360, od północy zgodnie z ruchem wskazówek).
pub fn azimuth_deg(def: &SatelliteDef, obs: &Observer, t: f64) -> f64 {
    let s = ecef_position(def, t);
    let o = observer_ecef(obs);
    let lat = obs.lat_deg * DEG2RAD;
    let lon = obs.lon_deg * DEG2RAD;
    let dx = s[0] - o[0];
    let dy = s[1] - o[1];
    let dz = s[2] - o[2];

    let east = [-lon.sin(), lon.cos(), 0.0];
    let north = [-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()];

    let e = dx * east[0] + dy * east[1] + dz * east[2];
    let n = dx * north[0] + dy * north[1] + dz * north[2];

    let az = e.atan2(n) * RAD2DEG;
    (az + 360.0) % 360.0
}

/// Przesunięcie Dopplera w kHz dla danej częstotliwości nośnej (MHz).
/// Wartość dodatnia = zbliżanie (częstotliwość odbierana wyższa).
pub fn doppler_khz(def: &SatelliteDef, obs: &Observer, t: f64, freq_mhz: f64) -> f64 {
    let dt = 0.5;
    let r1 = range_km(def, obs, t);
    let r2 = range_km(def, obs, t + dt);
    let range_rate = (r2 - r1) / dt; // km/s, dodatnie = oddalanie
    -(range_rate / C_KM_S) * freq_mhz * 1000.0
}

/// Przewiduje przeloty nad horyzontem `horizon_deg` w oknie `hours` godzin od `t0`.
pub fn predict_passes(
    def: &SatelliteDef,
    obs: &Observer,
    t0: f64,
    horizon_deg: f64,
    hours: f64,
) -> Vec<SatellitePass> {
    let dt = 10.0; // krok czasowy [s]
    let steps = ((hours * 3600.0) / dt).max(1.0) as usize;
    let mut passes = Vec::new();

    let mut prev_elev = elevation_deg(def, obs, t0);
    let mut in_pass = prev_elev > horizon_deg;
    let mut aos = if in_pass { t0 } else { 0.0 };
    let mut max_elev = if in_pass { prev_elev } else { f64::NEG_INFINITY };
    let mut aos_az = if in_pass { azimuth_deg(def, obs, t0) } else { 0.0 };

    for k in 1..=steps {
        let t = t0 + (k as f64) * dt;
        let elev = elevation_deg(def, obs, t);

        if !in_pass && prev_elev <= horizon_deg && elev > horizon_deg {
            in_pass = true;
            aos = t;
            max_elev = elev;
            aos_az = azimuth_deg(def, obs, t);
        } else if in_pass {
            if elev > max_elev {
                max_elev = elev;
            }
            if prev_elev >= horizon_deg && elev < horizon_deg {
                let los = t;
                passes.push(SatellitePass {
                    aos_unix: aos,
                    los_unix: los,
                    max_elevation_deg: max_elev,
                    aos_azimuth_deg: aos_az,
                    aos_doppler_khz: doppler_khz(def, obs, aos, def.downlink_mhz),
                });
                in_pass = false;
                max_elev = f64::NEG_INFINITY;
            }
        }
        prev_elev = elev;
    }
    passes
}

/// Zwraca najbliższy przelot: trwający teraz lub nadchodzący (los > t_now).
pub fn next_pass<'a>(passes: &'a [SatellitePass], t_now: f64) -> Option<&'a SatellitePass> {
    passes.iter().find(|p| p.los_unix > t_now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iss() -> SatelliteDef {
        default_satellites()
            .into_iter()
            .find(|s| s.name == "ISS (ZARYA)")
            .unwrap()
    }

    fn warsaw() -> Observer {
        Observer { lat_deg: 52.2297, lon_deg: 21.0122 }
    }

    #[test]
    fn orbital_period_returns_to_same_position() {
        let def = iss();
        let a = EARTH_RADIUS_KM + def.altitude_km;
        let n = (MU / (a * a * a)).sqrt();
        let period = 2.0 * PI / n;

        let p0 = eci_position(&def, 100.0);
        let p1 = eci_position(&def, 100.0 + period);
        for i in 0..3 {
            assert!((p0[i] - p1[i]).abs() < 1e-3, "axis {i} mismatch");
        }
    }

    #[test]
    fn elevation_is_within_range() {
        let def = iss();
        let obs = warsaw();
        for k in 0..200 {
            let e = elevation_deg(&def, &obs, k as f64 * 60.0);
            assert!((-90.0..=90.0).contains(&e), "elevation {e} out of range");
        }
    }

    #[test]
    fn azimuth_is_within_range() {
        let def = iss();
        let obs = warsaw();
        for k in 0..200 {
            let az = azimuth_deg(&def, &obs, k as f64 * 60.0);
            assert!((0.0..360.0).contains(&az), "azimuth {az} out of range");
        }
    }

    #[test]
    fn doppler_is_positive_on_approach_negative_on_recede() {
        let def = iss();
        let obs = warsaw();
        // Wybierz przelot i sprawdź znak Dopplera na AOS/LOS.
        let passes = predict_passes(&def, &obs, 0.0, 5.0, 24.0);
        assert!(!passes.is_empty());
        for p in &passes {
            // Na AOS (zbliżanie) Doppler > 0, przy LOS (oddalanie) < 0.
            assert!(p.aos_doppler_khz > 0.0, "AOS doppler should be positive");
            let los_doppler = doppler_khz(&def, &obs, p.los_unix, def.downlink_mhz);
            assert!(los_doppler < 0.0, "LOS doppler should be negative");
        }
    }

    #[test]
    fn passes_are_consistent() {
        let def = iss();
        let obs = warsaw();
        let t0 = 1_700_000_000.0;
        let passes = predict_passes(&def, &obs, t0, 10.0, 24.0);
        assert!(!passes.is_empty(), "expected at least one pass in 24h");
        for p in &passes {
            assert!(p.aos_unix < p.los_unix, "AOS must precede LOS");
            assert!(p.max_elevation_deg >= 10.0, "max elevation above horizon");
            assert!(p.aos_unix >= t0 - 1.0, "AOS not before window");
            assert!(p.los_unix <= t0 + 24.0 * 3600.0 + 1.0, "LOS within window");
        }
    }

    #[test]
    fn next_pass_returns_in_progress_or_upcoming() {
        let def = iss();
        let obs = warsaw();
        let t0 = 1_700_000_000.0;
        let passes = predict_passes(&def, &obs, t0, 10.0, 24.0);
        let np = next_pass(&passes, t0).expect("next pass exists");
        assert!(np.los_unix > t0);
        // Następny po zakończeniu pierwszego też istnieje.
        let np2 = next_pass(&passes, np.los_unix + 1.0);
        assert!(np2.is_some());
    }
}
