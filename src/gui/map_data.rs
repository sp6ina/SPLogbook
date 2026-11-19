// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Dane i geometria mapy świata: kontury lądów, cień nocy, Wielkie Koło.
//!
//! Kontury lądów pochodzą z **Natural Earth 110m Land (Public Domain)**,
//! <https://www.naturalearthdata.com/>. Zasób `assets/maps/ne_110m_land.bin` jest zwartym
//! formatem binarnym (delta-kodowane `i16`, rozdzielczość 0,01°), wbudowanym przez
//! `include_bytes!`, bez dodatkowych zależności.
//!
//! Format (little-endian): magia `NEL1`, `u32` liczba pierścieni, a dla każdego pierścienia
//! `u16` liczba punktów (bit `0x8000` = dziura, np. jezioro), `i16 lon0`, `i16 lat0`,
//! po czym `(punktów - 1)` par delt `(i16, i16)` liczonych modulo 2^16 (`wrapping_add`).
//! Współrzędne w setnych stopnia; pierścień jest domykany niejawnie.

use std::sync::OnceLock;

/// Natural Earth 110m Land, Public Domain.
const LAND_BIN: &[u8] = include_bytes!("../../assets/maps/ne_110m_land.bin");

/// Pierścień konturu: punkty `[lon, lat]` w stopniach.
#[derive(Debug, Clone, PartialEq)]
pub struct Ring {
    pub hole: bool,
    pub points: Vec<[f32; 2]>,
}

/// Błąd dekodowania zasobu mapy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapDataError {
    BadMagic,
    Truncated,
}

/// Dekoduje pierścienie z formatu `NEL1`.
pub fn decode_rings(bytes: &[u8]) -> Result<Vec<Ring>, MapDataError> {
    struct Reader<'a> {
        bytes: &'a [u8],
        pos: usize,
    }
    impl Reader<'_> {
        fn take<const N: usize>(&mut self) -> Result<[u8; N], MapDataError> {
            let end = self.pos + N;
            let slice = self
                .bytes
                .get(self.pos..end)
                .ok_or(MapDataError::Truncated)?;
            self.pos = end;
            let mut out = [0_u8; N];
            out.copy_from_slice(slice);
            Ok(out)
        }
        fn i16(&mut self) -> Result<i16, MapDataError> {
            Ok(i16::from_le_bytes(self.take::<2>()?))
        }
    }

    if bytes.get(..4) != Some(b"NEL1") {
        return Err(MapDataError::BadMagic);
    }
    let mut reader = Reader { bytes, pos: 4 };
    let count = u32::from_le_bytes(reader.take::<4>()?) as usize;
    let mut rings = Vec::with_capacity(count);
    for _ in 0..count {
        let header = u16::from_le_bytes(reader.take::<2>()?);
        let hole = header & 0x8000 != 0;
        let len = (header & 0x7FFF) as usize;
        let mut lon = reader.i16()?;
        let mut lat = reader.i16()?;
        let mut points = Vec::with_capacity(len);
        points.push([f32::from(lon) / 100.0, f32::from(lat) / 100.0]);
        for _ in 1..len {
            lon = lon.wrapping_add(reader.i16()?);
            lat = lat.wrapping_add(reader.i16()?);
            points.push([f32::from(lon) / 100.0, f32::from(lat) / 100.0]);
        }
        rings.push(Ring { hole, points });
    }
    Ok(rings)
}

/// Podwojone pole znakowane wielokąta (dodatnie dla CCW).
fn signed_area2(points: &[[f32; 2]]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % n];
            f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1])
        })
        .sum()
}

fn cross(o: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f64 {
    (f64::from(a[0]) - f64::from(o[0])) * (f64::from(b[1]) - f64::from(o[1]))
        - (f64::from(a[1]) - f64::from(o[1])) * (f64::from(b[0]) - f64::from(o[0]))
}

fn in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let d1 = cross(a, b, p);
    let d2 = cross(b, c, p);
    let d3 = cross(c, a, p);
    d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
}

/// Triangulacja prostego wielokąta metodą ear-clipping. Zwraca trójki indeksów punktów
/// (zorientowane CCW). Wklęsłe wybrzeża są obsługiwane, bo ucho jest odcinane tylko, gdy
/// żaden inny wierzchołek nie leży w jego trójkącie.
#[must_use]
pub fn triangulate(points: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let mut remaining: Vec<usize> = (0..n).collect();
    if signed_area2(points) < 0.0 {
        remaining.reverse();
    }
    let mut triangles = Vec::with_capacity(n - 2);

    while remaining.len() > 3 {
        let m = remaining.len();
        let mut clipped = false;
        for i in 0..m {
            let prev = remaining[(i + m - 1) % m];
            let cur = remaining[i];
            let next = remaining[(i + 1) % m];
            let (a, b, c) = (points[prev], points[cur], points[next]);
            let turn = cross(a, b, c);
            if turn <= 0.0 {
                continue; // wierzchołek wklęsły lub współliniowy
            }
            let blocked = remaining.iter().any(|&other| {
                other != prev
                    && other != cur
                    && other != next
                    && in_triangle(points[other], a, b, c)
            });
            if blocked {
                continue;
            }
            triangles.push([prev, cur, next]);
            remaining.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // Zdegenerowany wielokąt (np. współliniowe punkty): usuń wierzchołek bez trójkąta,
            // żeby pętla zawsze się kończyła.
            let m = remaining.len();
            let pos = (0..m)
                .min_by(|&x, &y| {
                    let area = |i: usize| {
                        cross(
                            points[remaining[(i + m - 1) % m]],
                            points[remaining[i]],
                            points[remaining[(i + 1) % m]],
                        )
                        .abs()
                    };
                    area(x).total_cmp(&area(y))
                })
                .unwrap_or(0);
            remaining.remove(pos);
        }
    }
    if remaining.len() == 3
        && cross(
            points[remaining[0]],
            points[remaining[1]],
            points[remaining[2]],
        ) > 0.0
    {
        triangles.push([remaining[0], remaining[1], remaining[2]]);
    }
    triangles
}

/// Zdekodowana i zatriangulowana geometria lądów (wspólna lista wierzchołków `[lon, lat]`).
#[derive(Debug)]
pub struct LandGeometry {
    pub vertices: Vec<[f32; 2]>,
    /// Indeksy trójkątów lądu (płaska lista po trzy).
    pub land: Vec<u32>,
    /// Indeksy trójkątów dziur (np. Morze Kaspijskie) — rysowane kolorem oceanu.
    pub holes: Vec<u32>,
    /// Zakresy `(start, długość, dziura)` pierścieni w `vertices` (obrys linii brzegowej).
    pub rings: Vec<(usize, usize, bool)>,
}

/// Buduje geometrię z pierścieni.
#[must_use]
pub fn build_geometry(rings: &[Ring]) -> LandGeometry {
    let mut geometry = LandGeometry {
        vertices: Vec::new(),
        land: Vec::new(),
        holes: Vec::new(),
        rings: Vec::new(),
    };
    for ring in rings {
        let base = geometry.vertices.len();
        geometry.vertices.extend_from_slice(&ring.points);
        geometry.rings.push((base, ring.points.len(), ring.hole));
        let target = if ring.hole {
            &mut geometry.holes
        } else {
            &mut geometry.land
        };
        for [a, b, c] in triangulate(&ring.points) {
            target.extend([(base + a) as u32, (base + b) as u32, (base + c) as u32]);
        }
    }
    geometry
}

/// Geometria lądów Natural Earth 110m (dekodowana i triangulowana jednorazowo).
pub fn land() -> &'static LandGeometry {
    static LAND: OnceLock<LandGeometry> = OnceLock::new();
    LAND.get_or_init(|| {
        let rings = decode_rings(LAND_BIN).unwrap_or_else(|error| {
            log::error!("corrupted land map asset: {error:?}");
            Vec::new()
        });
        build_geometry(&rings)
    })
}

/// Strefy zmierzchu: kąty zenitalne Słońca (84° = dzień tuż przed terminatorem, 90° = terminator,
/// 96° = zmierzch cywilny, 108° = zmierzch astronomiczny) oraz ich nieprzezroczystość (0..1)
/// względem pełnej nocy.
pub const NIGHT_ZENITHS_DEG: [f64; 4] = [84.0, 90.0, 96.0, 108.0];
pub const NIGHT_ALPHAS: [f32; 5] = [0.0, 0.45, 0.8, 1.0, 1.0];

/// Szerokość geograficzna, na której Słońce ma kąt zenitalny `zenith_deg` dla długości `lon_deg`.
/// Gdy południk jest w całości po jednej stronie krzywej, zwraca odpowiedni biegun.
#[must_use]
pub fn zenith_curve_lat(lon_deg: f64, sub_lat_deg: f64, sub_lon_deg: f64, zenith_deg: f64) -> f64 {
    let day_pole = if sub_lat_deg >= 0.0 { 90.0 } else { -90.0 };
    let night_pole = -day_pole;
    let delta = sub_lat_deg.to_radians();
    let hour_angle = (lon_deg - sub_lon_deg).to_radians();
    let a = delta.sin();
    let b = delta.cos() * hour_angle.cos();
    let r = a.hypot(b);
    let cos_zenith = zenith_deg.to_radians().cos();
    if r < 1e-9 {
        return if cos_zenith >= 0.0 {
            day_pole
        } else {
            night_pole
        };
    }
    let x = cos_zenith / r;
    if x > 1.0 {
        return day_pole;
    }
    if x < -1.0 {
        return night_pole;
    }
    let theta = b.atan2(a);
    let normalize = |angle: f64| {
        let two_pi = 2.0 * std::f64::consts::PI;
        let mut v = angle % two_pi;
        if v > std::f64::consts::PI {
            v -= two_pi;
        } else if v < -std::f64::consts::PI {
            v += two_pi;
        }
        v
    };
    let roots = [
        normalize(x.asin() - theta),
        normalize(std::f64::consts::PI - x.asin() - theta),
    ];
    let half_pi = std::f64::consts::FRAC_PI_2 + 1e-9;
    let valid: Vec<f64> = roots
        .into_iter()
        .filter(|v| v.abs() <= half_pi)
        .map(f64::to_degrees)
        .collect();
    let pick = if sub_lat_deg >= 0.0 {
        valid.iter().copied().fold(f64::INFINITY, f64::min)
    } else {
        valid.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    };
    if pick.is_finite() {
        pick.clamp(-90.0, 90.0)
    } else {
        night_pole
    }
}

/// Szerokości kolumny cienia nocy dla długości `lon_deg`: cztery krzywe zenitalne (od dnia do
/// głębokiej nocy, monotoniczne w stronę bieguna nocy) oraz biegun nocy.
#[must_use]
pub fn night_column(lon_deg: f64, sub_lat_deg: f64, sub_lon_deg: f64) -> [f64; 5] {
    let mut column = [0.0_f64; 5];
    for (slot, zenith) in column.iter_mut().zip(NIGHT_ZENITHS_DEG) {
        *slot = zenith_curve_lat(lon_deg, sub_lat_deg, sub_lon_deg, zenith);
    }
    column[4] = if sub_lat_deg >= 0.0 { -90.0 } else { 90.0 };
    // Monotoniczność: kolejne krzywe nie mogą cofać się w stronę dnia.
    for i in 1..5 {
        column[i] = if sub_lat_deg >= 0.0 {
            column[i].min(column[i - 1])
        } else {
            column[i].max(column[i - 1])
        };
    }
    column
}

/// Łuk Wielkiego Koła (najkrótsza ścieżka) między dwoma punktami `(lat, lon)`, podzielony na
/// segmenty na antypołudniku `±180°` (z punktami granicznymi interpolowanymi na krawędzi mapy).
#[must_use]
pub fn great_circle_segments(
    from: (f64, f64),
    to: (f64, f64),
    steps: usize,
) -> Vec<Vec<(f64, f64)>> {
    let to_vec = |(lat, lon): (f64, f64)| {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
    };
    let v1 = to_vec(from);
    let v2 = to_vec(to);
    let dot = (v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]).clamp(-1.0, 1.0);
    let omega = dot.acos();
    let sin_omega = omega.sin();
    let steps = steps.max(2);

    let points: Vec<(f64, f64)> = (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            if sin_omega.abs() < 1e-9 {
                return (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
            }
            let w1 = ((1.0 - t) * omega).sin() / sin_omega;
            let w2 = (t * omega).sin() / sin_omega;
            let x = w1 * v1[0] + w2 * v2[0];
            let y = w1 * v1[1] + w2 * v2[1];
            let z = w1 * v1[2] + w2 * v2[2];
            (z.atan2(x.hypot(y)).to_degrees(), y.atan2(x).to_degrees())
        })
        .collect();

    let mut segments: Vec<Vec<(f64, f64)>> = vec![vec![points[0]]];
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if (b.1 - a.1).abs() > 180.0 {
            // Przejście przez antypołudnik: dosuń `b` do ciągłości i znajdź punkt graniczny.
            let b_shifted = if b.1 < a.1 { b.1 + 360.0 } else { b.1 - 360.0 };
            let edge = if b.1 < a.1 { 180.0 } else { -180.0 };
            let t = (edge - a.1) / (b_shifted - a.1);
            let lat_edge = a.0 + (b.0 - a.0) * t;
            if let Some(last) = segments.last_mut() {
                last.push((lat_edge, edge));
            }
            segments.push(vec![(lat_edge, -edge), b]);
        } else if let Some(last) = segments.last_mut() {
            last.push(b);
        }
    }
    segments.retain(|segment| segment.len() >= 2);
    segments
}

/// Etykieta pola Maidenhead (`AA`..`RR`) dla pola o indeksach `(kolumna lon 0..18, wiersz lat 0..18)`.
#[must_use]
pub fn maidenhead_field(lon_index: usize, lat_index: usize) -> String {
    let letter = |i: usize| char::from(b'A' + (i.min(17)) as u8);
    format!("{}{}", letter(lon_index), letter(lat_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon_area(points: &[[f32; 2]]) -> f64 {
        signed_area2(points).abs() / 2.0
    }

    fn triangles_area(points: &[[f32; 2]], triangles: &[[usize; 3]]) -> f64 {
        triangles
            .iter()
            .map(|t| cross(points[t[0]], points[t[1]], points[t[2]]).abs() / 2.0)
            .sum()
    }

    #[test]
    fn natural_earth_asset_decodes_with_valid_ranges() {
        let rings = decode_rings(LAND_BIN).expect("zasób musi się zdekodować");
        assert!(rings.len() >= 100, "rzędu 127 lądów: {}", rings.len());
        assert!(LAND_BIN.len() < 64 * 1024, "zasób ma być lekki");
        let mut points = 0;
        for ring in &rings {
            assert!(ring.points.len() >= 3);
            for [lon, lat] in &ring.points {
                assert!(lon.is_finite() && lat.is_finite());
                assert!((-180.0..=180.0).contains(lon), "lon {lon}");
                assert!((-90.0..=90.0).contains(lat), "lat {lat}");
            }
            points += ring.points.len();
        }
        assert!(points > 4000);
        assert!(rings.iter().any(|r| r.hole), "Morze Kaspijskie to dziura");
        assert_eq!(decode_rings(b"XXXX"), Err(MapDataError::BadMagic));
        assert_eq!(decode_rings(&LAND_BIN[..20]), Err(MapDataError::Truncated));
    }

    #[test]
    fn triangulation_covers_every_ring_without_artifacts() {
        let rings = decode_rings(LAND_BIN).unwrap();
        for (i, ring) in rings.iter().enumerate() {
            let triangles = triangulate(&ring.points);
            assert!(!triangles.is_empty(), "pierścień {i} bez trójkątów");
            let polygon = polygon_area(&ring.points);
            let covered = triangles_area(&ring.points, &triangles);
            assert!(covered.is_finite());
            // Suma pól trójkątów = pole wielokąta ⇒ brak nakładania i dziur (wklęsłe wybrzeża).
            assert!(
                (covered - polygon).abs() <= polygon * 0.002 + 1e-3,
                "pierścień {i}: trójkąty {covered} vs wielokąt {polygon}"
            );
            for t in &triangles {
                let area = cross(ring.points[t[0]], ring.points[t[1]], ring.points[t[2]]);
                assert!(area > 0.0, "pierścień {i}: pusty/odwrócony trójkąt");
            }
        }
    }

    #[test]
    fn concave_polygon_is_triangulated_exactly() {
        // Litera „L” — wklęsła.
        let l_shape = [
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 1.0],
            [1.0, 1.0],
            [1.0, 4.0],
            [0.0, 4.0],
        ];
        let triangles = triangulate(&l_shape);
        assert_eq!(triangles.len(), 4);
        assert!((triangles_area(&l_shape, &triangles) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn land_geometry_is_cached_and_consistent() {
        let a = land();
        let b = land();
        assert!(std::ptr::eq(a, b), "OnceLock: jedna instancja");
        assert_eq!(a.land.len() % 3, 0);
        assert!(a.land.iter().all(|&i| (i as usize) < a.vertices.len()));
        assert!(a.holes.len().is_multiple_of(3) && !a.holes.is_empty());
    }

    #[test]
    fn night_columns_are_finite_and_monotonic_all_year() {
        for sub_lat in [-23.4, -10.0, 0.0, 0.0005, 10.0, 23.4] {
            for lon_step in 0..=90 {
                let lon = -180.0 + f64::from(lon_step) * 4.0;
                let column = night_column(lon, sub_lat, 17.0);
                for v in column {
                    assert!(v.is_finite() && (-90.0..=90.0).contains(&v), "{column:?}");
                }
                for pair in column.windows(2) {
                    if sub_lat >= 0.0 {
                        assert!(pair[1] <= pair[0] + 1e-9, "{column:?}");
                    } else {
                        assert!(pair[1] >= pair[0] - 1e-9, "{column:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn terminator_matches_the_classic_formula() {
        let (dec, sub_lon): (f64, f64) = (20.0, 0.0);
        for lon in [-120.0, -45.0, 30.0, 100.0] {
            let h = (lon - sub_lon).to_radians();
            let classic = (-h.cos() / dec.to_radians().tan()).atan().to_degrees();
            let ours = zenith_curve_lat(lon, dec, sub_lon, 90.0);
            assert!(
                (classic - ours).abs() < 1e-6,
                "lon {lon}: {classic} vs {ours}"
            );
        }
    }

    #[test]
    fn great_circle_splits_at_the_antimeridian() {
        // Tokio -> Honolulu przechodzi przez 180°.
        let segments = great_circle_segments((35.7, 139.7), (21.3, -157.9), 48);
        assert!(segments.len() >= 2, "łuk dzieli się na antypołudniku");
        let first_end = segments[0].last().unwrap();
        let second_start = segments[1].first().unwrap();
        assert!((first_end.1.abs() - 180.0).abs() < 1e-6);
        assert!((second_start.1.abs() - 180.0).abs() < 1e-6);
        assert!((first_end.0 - second_start.0).abs() < 1e-6);
        for segment in &segments {
            for pair in segment.windows(2) {
                assert!((pair[1].1 - pair[0].1).abs() < 180.0, "skok w segmencie");
            }
        }
        // Bez przejścia: jeden segment.
        assert_eq!(
            great_circle_segments((51.1, 17.0), (40.7, -74.0), 36).len(),
            1
        );
    }

    #[test]
    fn maidenhead_fields_are_labelled() {
        assert_eq!(maidenhead_field(0, 0), "AA");
        assert_eq!(maidenhead_field(14, 13), "ON");
        assert_eq!(maidenhead_field(17, 17), "RR");
        // JO81 leży w polu JO: lon 15..., lat 50...
        assert_eq!(maidenhead_field(9, 14), "JO");
    }
}
