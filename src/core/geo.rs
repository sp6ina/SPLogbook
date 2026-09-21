// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::f64::consts::PI;

const EARTH_RADIUS_KM: f64 = 6371.0;
const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;

/// Współrzędne geograficzne w stopniach dziesiętnych
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,  // -90.0 .. +90.0
    pub longitude: f64, // -180.0 .. +180.0
}

impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self { latitude, longitude }
    }
}

/// Konwertuje współrzędne geograficzne na lokator Maidenhead (QTH Locator)
pub fn coordinates_to_locator(coords: Coordinates, precision: usize) -> Result<String, &'static str> {
    if !(-90.0..=90.0).contains(&coords.latitude) || !(-180.0..=180.0).contains(&coords.longitude) {
        return Err("Współrzędne poza dozwolonym zakresem (-90..90, -180..180)");
    }

    let lat = coords.latitude + 90.0;
    let lon = coords.longitude + 180.0;

    let f1 = (lon / 20.0).floor() as u8;
    let f2 = (lat / 10.0).floor() as u8;
    let rem_lon1 = lon - (f1 as f64 * 20.0);
    let rem_lat1 = lat - (f2 as f64 * 10.0);

    let s1 = (rem_lon1 / 2.0).floor() as u8;
    let s2 = rem_lat1.floor() as u8;
    let rem_lon2 = rem_lon1 - (s1 as f64 * 2.0);
    let rem_lat2 = rem_lat1 - (s2 as f64);

    let mut loc = String::with_capacity(precision);
    loc.push((b'A' + f1) as char);
    loc.push((b'A' + f2) as char);
    loc.push((b'0' + s1) as char);
    loc.push((b'0' + s2) as char);

    if precision >= 6 {
        let subs1 = (rem_lon2 / (2.0 / 24.0)).floor() as u8;
        let subs2 = (rem_lat2 / (1.0 / 24.0)).floor() as u8;
        loc.push((b'A' + subs1.min(23)) as char);
        loc.push((b'A' + subs2.min(23)) as char);
    }

    if precision >= 8 {
        let rem_lon3 = rem_lon2 - (loc.as_bytes()[4] - b'A') as f64 * (2.0 / 24.0);
        let rem_lat3 = rem_lat2 - (loc.as_bytes()[5] - b'A') as f64 * (1.0 / 24.0);
        let ext1 = (rem_lon3 / (2.0 / 240.0)).floor() as u8;
        let ext2 = (rem_lat3 / (1.0 / 240.0)).floor() as u8;
        loc.push((b'0' + ext1.min(9)) as char);
        loc.push((b'0' + ext2.min(9)) as char);
    }

    Ok(loc)
}

/// Konwertuje lokator Maidenhead (4, 6 lub 8 znaków) na współrzędne geograficzne środka kwadratu
pub fn locator_to_coordinates(locator: &str) -> Result<Coordinates, &'static str> {
    let loc = locator.trim().to_uppercase();
    let bytes = loc.as_bytes();
    if bytes.len() < 4 {
        return Err("Lokator musi mieć co najmniej 4 znaki");
    }

    if !bytes[0].is_ascii_uppercase() || !bytes[1].is_ascii_uppercase()
        || !bytes[2].is_ascii_digit() || !bytes[3].is_ascii_digit() {
        return Err("Nieprawidłowy format lokatora (oczekiwano np. JO81 lub JO81WA)");
    }

    let f1 = (bytes[0] - b'A') as f64;
    let f2 = (bytes[1] - b'A') as f64;
    let s1 = (bytes[2] - b'0') as f64;
    let s2 = (bytes[3] - b'0') as f64;

    let mut lon = f1 * 20.0 + s1 * 2.0 - 180.0;
    let mut lat = f2 * 10.0 + s2 * 1.0 - 90.0;

    if bytes.len() >= 6 && bytes[4].is_ascii_alphabetic() && bytes[5].is_ascii_alphabetic() {
        let sub1 = (bytes[4].to_ascii_uppercase() - b'A') as f64;
        let sub2 = (bytes[5].to_ascii_uppercase() - b'A') as f64;
        lon += sub1 * (2.0 / 24.0) + (1.0 / 24.0); // środek podkwadratu
        lat += sub2 * (1.0 / 24.0) + (0.5 / 24.0);
    } else {
        // środek kwadratu 4-znakowego
        lon += 1.0;
        lat += 0.5;
    }

    Ok(Coordinates::new(lat, lon))
}

/// Oblicza odległość ortodromiczną (Great Circle) w kilometrach między dwoma punktami
pub fn calculate_distance_km(p1: Coordinates, p2: Coordinates) -> f64 {
    let lat1 = p1.latitude * DEG_TO_RAD;
    let lon1 = p1.longitude * DEG_TO_RAD;
    let lat2 = p2.latitude * DEG_TO_RAD;
    let lon2 = p2.longitude * DEG_TO_RAD;

    let dlat = lat2 - lat1;
    let dlon = lon2 - lon1;

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.clamp(0.0, 1.0).sqrt().asin();

    EARTH_RADIUS_KM * c
}

/// Oblicza azymut Short Path (krótsza droga) w stopniach (0..360) z p1 do p2
pub fn calculate_bearing_deg(p1: Coordinates, p2: Coordinates) -> f64 {
    let lat1 = p1.latitude * DEG_TO_RAD;
    let lon1 = p1.longitude * DEG_TO_RAD;
    let lat2 = p2.latitude * DEG_TO_RAD;
    let lon2 = p2.longitude * DEG_TO_RAD;

    let dlon = lon2 - lon1;
    let y = dlon.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();

    let bearing = y.atan2(x) * RAD_TO_DEG;
    (bearing + 360.0) % 360.0
}

/// Zwraca azymut Long Path (dłuższa droga)
pub fn calculate_long_path_deg(short_path_deg: f64) -> f64 {
    (short_path_deg + 180.0) % 360.0
}

/// Wynik kalkulacji pozycji słońca
#[derive(Debug, Clone, Copy)]
pub struct SolarPosition {
    pub declination: f64,      // Deklinacja słońca w stopniach
    pub gha: f64,              // Kąt godzinny Greenwich (Greenwich Hour Angle) w stopniach
    pub elevation: f64,        // Elewacja słońca nad horyzontem w danym punkcie
    pub is_grayline: bool,     // Czy punkt znajduje się w strefie Gray Line (-12° do 0° pod horyzontem)
}

/// Wylicza pozycję słońca dla danej lokalizacji i czasu UTC (rok, dzień roku, godzina dziesiętna UTC)
pub fn calculate_solar_position(coords: Coordinates, day_of_year: u32, utc_hours: f64) -> SolarPosition {
    // Przybliżenie równania czasu i deklinacji słońca (Spencer / Meeus)
    let b = 2.0 * PI * (day_of_year as f64 - 81.0) / 365.0;
    let declination_rad = (23.45 * DEG_TO_RAD) * b.sin();
    let declination = declination_rad * RAD_TO_DEG;

    // Kąt godzinny Greenwich (GHA)
    let gha = (utc_hours - 12.0) * 15.0;

    // Kąt godzinny lokalny (LHA)
    let lha_rad = (gha - coords.longitude) * DEG_TO_RAD;

    let lat_rad = coords.latitude * DEG_TO_RAD;
    let sin_elev = lat_rad.sin() * declination_rad.sin() + lat_rad.cos() * declination_rad.cos() * lha_rad.cos();
    let elevation = sin_elev.clamp(-1.0, 1.0).asin() * RAD_TO_DEG;

    // Grayline to pasmo zmierzchu radiowego (zazwyczaj od zachodu słońca do zmierzchu morskiego: -12° do 0°)
    let is_grayline = (-12.0..=0.0).contains(&elevation);

    SolarPosition {
        declination,
        gha,
        elevation,
        is_grayline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maidenhead_conversion() {
        // Wrocław / Dolny Śląsk (SP6INA) - lokator JO81wa
        let coords = locator_to_coordinates("JO81WA").unwrap();
        assert!((coords.latitude - 51.0).abs() < 0.5);
        assert!((coords.longitude - 17.9).abs() < 0.5);

        let loc = coordinates_to_locator(coords, 6).unwrap();
        assert_eq!(loc, "JO81WA");
    }

    #[test]
    fn test_distance_and_azimuth() {
        // JO81 (Wrocław) do KO02 (Warszawa)
        let p1 = locator_to_coordinates("JO81").unwrap();
        let p2 = locator_to_coordinates("KO02").unwrap();

        let dist = calculate_distance_km(p1, p2);
        assert!(dist > 250.0 && dist < 350.0);

        let bearing = calculate_bearing_deg(p1, p2);
        assert!(bearing > 45.0 && bearing < 75.0); // Kierunek północny-wschód
    }
}
