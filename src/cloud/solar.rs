// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Wskaźniki aktywności słonecznej i geomagnetycznej (NOAA SWPC / HamQTH)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpaceWeather {
    pub sfi: u32,             // Solar Flux Index (np. 145)
    pub ssn: u32,             // Sunspot Number (Liczba Wolfa, np. 98)
    pub a_index: u32,         // Indeks A (24-godzinna aktywność geomagnetyczna, np. 6)
    pub k_index: u32,         // Indeks K (planitarny wskaźnik burz geomagnetycznych 0..9, np. 1)
    pub x_ray: String,        // Poziom promieniowania rentgenowskiego (np. "B3.2", "M1.5")
    pub geomagnetic_field: String, // "Quiet", "Unsettled", "Active", "Minor Storm"
    pub aurora_latitude: u32, // Szerokość geograficzna owalu zorzy polarnej (np. 67°N)
    pub updated_utc: String,
}

impl Default for SpaceWeather {
    fn default() -> Self {
        Self {
            sfi: 140,
            ssn: 90,
            a_index: 5,
            k_index: 1,
            x_ray: "B1.0".to_string(),
            geomagnetic_field: "Quiet".to_string(),
            aurora_latitude: 67,
            updated_utc: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        }
    }
}

/// Ocena warunków propagacyjnych na danym paśmie
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BandCondition {
    Poor,
    Fair,
    Good,
}

impl SpaceWeather {
    /// Szacuje warunki propagacyjne dla pasm wyższych (20m - 10m) w ciągu dnia
    pub fn hf_day_condition(&self) -> BandCondition {
        if self.k_index >= 4 {
            BandCondition::Poor // Burza geomagnetyczna tłumi wyższe pasma
        } else if self.sfi >= 120 {
            BandCondition::Good
        } else if self.sfi >= 85 {
            BandCondition::Fair
        } else {
            BandCondition::Poor
        }
    }

    /// Szacuje warunki propagacyjne dla pasm niższych (160m - 40m) w nocy
    pub fn lf_night_condition(&self) -> BandCondition {
        if self.k_index >= 4 || self.a_index >= 20 {
            BandCondition::Poor
        } else if self.k_index <= 2 && self.a_index <= 10 {
            BandCondition::Good
        } else {
            BandCondition::Fair
        }
    }
}

/// Klient pobierania danych solarnych z NOAA Space Weather Prediction Center
pub struct SpaceWeatherClient {
    client: Client,
}

impl Default for SpaceWeatherClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceWeatherClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("SPLogbook/1.0.0 (SP6INA)")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Pobiera najnowszy raport solarny z HamQTH XML
    pub async fn fetch_hamqth_solar(&self) -> Result<SpaceWeather, Box<dyn std::error::Error>> {
        let url = "https://www.hamqth.com/xml.php?solar=1";
        let xml = self.client.get(url).send().await?.text().await?;

        let sfi = Self::extract_tag(&xml, "solarflux").and_then(|v| v.parse().ok()).unwrap_or(135);
        let ssn = Self::extract_tag(&xml, "sunspots").and_then(|v| v.parse().ok()).unwrap_or(80);
        let a_index = Self::extract_tag(&xml, "aindex").and_then(|v| v.parse().ok()).unwrap_or(6);
        let k_index = Self::extract_tag(&xml, "kindex").and_then(|v| v.parse().ok()).unwrap_or(1);
        let x_ray = Self::extract_tag(&xml, "xray").unwrap_or_else(|| "B1.0".to_string());
        let geomagnetic_field = Self::extract_tag(&xml, "geomagfield").unwrap_or_else(|| "Quiet".to_string());

        Ok(SpaceWeather {
            sfi,
            ssn,
            a_index,
            k_index,
            x_ray,
            geomagnetic_field,
            aurora_latitude: 65,
            updated_utc: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        })
    }

    fn extract_tag(xml: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        let start = xml.find(&open_tag)? + open_tag.len();
        let end = xml[start..].find(&close_tag)? + start;
        let val = xml[start..end].trim().to_string();
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
    }
}
