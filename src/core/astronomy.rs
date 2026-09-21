// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Precyzyjny kalkulator astronomiczny pozycji Księżyca i Słońca dla łączności EME (Earth-Moon-Earth)


#[derive(Debug, Clone, PartialEq)]
pub struct CelestialPosition {
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub distance_km: f64,
    pub right_ascension_hours: f64,
    pub declination_deg: f64,
    pub is_visible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmeStatus {
    pub moon: CelestialPosition,
    pub sun: CelestialPosition,
    pub doppler_144_hz: f64,
    pub doppler_432_hz: f64,
    pub doppler_1296_hz: f64,
    pub path_loss_144_db: f64,
}

pub struct AstronomyEngine;

impl AstronomyEngine {
    /// Oblicza aktualną pozycję Księżyca i Słońca oraz przesunięcie Dopplera EME dla danej pozycji na Ziemi
    pub fn calculate_eme(lat_deg: f64, lon_deg: f64, now_utc: chrono::DateTime<chrono::Utc>) -> EmeStatus {
        let julian_day = Self::datetime_to_jd(now_utc);
        let d = julian_day - 2451543.5; // Dni od J2000.0

        let sun_pos = Self::calculate_sun(lat_deg, lon_deg, d, julian_day);
        let moon_pos = Self::calculate_moon(lat_deg, lon_deg, d, julian_day);

        // Wyliczenie prędkości radialnej Księżyca i przesunięcia Dopplera EME
        let dt_sec = 60.0;
        let jd_next = julian_day + (dt_sec / 86400.0);
        let d_next = jd_next - 2451543.5;
        let moon_next = Self::calculate_moon(lat_deg, lon_deg, d_next, jd_next);

        let delta_dist_m = (moon_next.distance_km - moon_pos.distance_km) * 1000.0;
        let radial_velocity_m_s = delta_dist_m / dt_sec;

        let c = 299_792_458.0; // Prędkość światła m/s
        // Doppler w obie strony (stacja -> Księżyc -> stacja): 2 * v / c * freq
        let doppler_144 = -2.0 * (radial_velocity_m_s / c) * 144_100_000.0;
        let doppler_432 = -2.0 * (radial_velocity_m_s / c) * 432_100_000.0;
        let doppler_1296 = -2.0 * (radial_velocity_m_s / c) * 1_296_100_000.0;

        // Szacowane tłumienie trasy EME na 144 MHz (~252 dB)
        let path_loss_144 = 252.0 + 20.0 * (moon_pos.distance_km / 384_400.0).log10();

        EmeStatus {
            moon: moon_pos,
            sun: sun_pos,
            doppler_144_hz: doppler_144,
            doppler_432_hz: doppler_432,
            doppler_1296_hz: doppler_1296,
            path_loss_144_db: path_loss_144,
        }
    }

    fn datetime_to_jd(dt: chrono::DateTime<chrono::Utc>) -> f64 {
        use chrono::Datelike;
        use chrono::Timelike;

        let year = dt.year();
        let month = dt.month() as i32;
        let day = dt.day() as i32;
        let hour = dt.hour() as f64 + dt.minute() as f64 / 60.0 + dt.second() as f64 / 3600.0;

        let (y, m) = if month <= 2 {
            (year - 1, month + 12)
        } else {
            (year, month)
        };

        let a = (y as f64 / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();

        (365.25 * (y as f64 + 4716.0)).floor()
            + (30.6001 * (m as f64 + 1.0)).floor()
            + day as f64
            + hour / 24.0
            + b
            - 1524.5
    }

    fn calculate_sun(lat_deg: f64, lon_deg: f64, d: f64, jd: f64) -> CelestialPosition {
        let w = 282.9404 + 4.70935e-5 * d; // longitude of perihelion
        let a = 1.000000; // semi-major axis
        let e = 0.016709 - 1.151e-9 * d; // eccentricity
        let m = Self::rev(356.0470 + 0.9856002585 * d); // mean anomaly

        let e_rad = m.to_radians() + e * m.to_radians().sin() * (1.0 + e * m.to_radians().cos());
        let x = a * (e_rad.cos() - e);
        let y = a * (1.0 - e * e).sqrt() * e_rad.sin();

        let r = (x * x + y * y).sqrt();
        let v = y.atan2(x).to_degrees();
        let lon = Self::rev(v + w);

        // Ekliptyka do równika
        let obl_ecl = (23.4393 - 3.563e-7 * d).to_radians();
        let x_ecl = r * lon.to_radians().cos();
        let y_ecl = r * lon.to_radians().sin();

        let x_eq = x_ecl;
        let y_eq = y_ecl * obl_ecl.cos();
        let z_eq = y_ecl * obl_ecl.sin();

        let ra_rad = y_eq.atan2(x_eq);
        let dec_rad = z_eq.atan2((x_eq * x_eq + y_eq * y_eq).sqrt());

        let sidereal_time = Self::greenwich_mean_sidereal_time(jd) + lon_deg;
        let ha_rad = (sidereal_time - ra_rad.to_degrees()).to_radians();

        let lat_rad = lat_deg.to_radians();
        let sin_alt = lat_rad.sin() * dec_rad.sin() + lat_rad.cos() * dec_rad.cos() * ha_rad.cos();
        let alt_rad = sin_alt.clamp(-1.0, 1.0).asin();

        let cos_az = (dec_rad.sin() - lat_rad.sin() * alt_rad.sin()) / (lat_rad.cos() * alt_rad.cos());
        let sin_ha = ha_rad.sin();
        let mut az_deg = cos_az.clamp(-1.0, 1.0).acos().to_degrees();
        if sin_ha > 0.0 {
            az_deg = 360.0 - az_deg;
        }

        CelestialPosition {
            azimuth_deg: az_deg,
            elevation_deg: alt_rad.to_degrees(),
            distance_km: r * 149_597_870.7,
            right_ascension_hours: Self::rev(ra_rad.to_degrees()) / 15.0,
            declination_deg: dec_rad.to_degrees(),
            is_visible: alt_rad > 0.0,
        }
    }

    fn calculate_moon(lat_deg: f64, lon_deg: f64, d: f64, jd: f64) -> CelestialPosition {
        let n = Self::rev(125.1228 - 0.0529538083 * d); // Long of asc. node
        let i = 5.1454_f64.to_radians(); // Inclination
        let w = Self::rev(318.0634 + 0.1643573223 * d); // Arg of perigee
        let a = 60.2666; // Earth radii
        let e = 0.054900; // Eccentricity
        let m = Self::rev(115.3654 + 13.0649929509 * d); // Mean anomaly

        let e_rad = m.to_radians() + e * m.to_radians().sin() * (1.0 + e * m.to_radians().cos());
        let x = a * (e_rad.cos() - e);
        let y = a * (1.0 - e * e).sqrt() * e_rad.sin();

        let r = (x * x + y * y).sqrt();
        let v = y.atan2(x).to_degrees();

        let x_ecl = r * (n.to_radians().cos() * (v + w).to_radians().cos() - n.to_radians().sin() * (v + w).to_radians().sin() * i.cos());
        let y_ecl = r * (n.to_radians().sin() * (v + w).to_radians().cos() + n.to_radians().cos() * (v + w).to_radians().sin() * i.cos());
        let z_ecl = r * (v + w).to_radians().sin() * i.sin();

        let obl_ecl = (23.4393 - 3.563e-7 * d).to_radians();
        let x_eq = x_ecl;
        let y_eq = y_ecl * obl_ecl.cos() - z_ecl * obl_ecl.sin();
        let z_eq = y_ecl * obl_ecl.sin() + z_ecl * obl_ecl.cos();

        let ra_rad = y_eq.atan2(x_eq);
        let dec_rad = z_eq.atan2((x_eq * x_eq + y_eq * y_eq).sqrt());

        let sidereal_time = Self::greenwich_mean_sidereal_time(jd) + lon_deg;
        let ha_rad = (sidereal_time - ra_rad.to_degrees()).to_radians();

        let lat_rad = lat_deg.to_radians();
        let sin_alt = lat_rad.sin() * dec_rad.sin() + lat_rad.cos() * dec_rad.cos() * ha_rad.cos();
        let alt_rad = sin_alt.clamp(-1.0, 1.0).asin();

        let cos_az = (dec_rad.sin() - lat_rad.sin() * alt_rad.sin()) / (lat_rad.cos() * alt_rad.cos());
        let sin_ha = ha_rad.sin();
        let mut az_deg = cos_az.clamp(-1.0, 1.0).acos().to_degrees();
        if sin_ha > 0.0 {
            az_deg = 360.0 - az_deg;
        }

        let dist_km = r * 6378.137;

        CelestialPosition {
            azimuth_deg: az_deg,
            elevation_deg: alt_rad.to_degrees(),
            distance_km: dist_km,
            right_ascension_hours: Self::rev(ra_rad.to_degrees()) / 15.0,
            declination_deg: dec_rad.to_degrees(),
            is_visible: alt_rad > 0.0,
        }
    }

    fn greenwich_mean_sidereal_time(jd: f64) -> f64 {
        let d = jd - 2451545.0;
        let gmst = 280.46061837 + 360.98564736629 * d;
        Self::rev(gmst)
    }

    fn rev(angle: f64) -> f64 {
        let mut a = angle % 360.0;
        if a < 0.0 {
            a += 360.0;
        }
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lunar_coordinates() {
        // Obliczenia dla Wrocławia (JO81WA)
        let wroclaw_lat = 51.1079;
        let wroclaw_lon = 17.0385;
        let now = chrono::Utc::now();
        let eme = AstronomyEngine::calculate_eme(wroclaw_lat, wroclaw_lon, now);

        assert!(eme.moon.distance_km >= 350_000.0 && eme.moon.distance_km <= 410_000.0);
        assert!(eme.sun.distance_km >= 140_000_000.0);
        assert!(eme.path_loss_144_db >= 250.0 && eme.path_loss_144_db <= 255.0);
    }
}
