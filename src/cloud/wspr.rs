// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WsprSpot {
    pub callsign: String,
    pub frequency: f64,
    pub snr: i32,
    pub gridsquare: String,
}

pub async fn fetch_wspr_spots(my_callsign: &str) -> Result<Vec<WsprSpot>, String> {
    let sanitized: String = my_callsign
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '/')
        .collect();
    if sanitized.is_empty() {
        return Err("Nieprawidłowy lub pusty znak wywoławczy".to_string());
    }
    let encoded_call = sanitized.replace('/', "%2F");
    let url = format!(
        "https://db1.wspr.live/?query=SELECT+callsign,frequency,snr,drift,gridsquare+FROM+wspr.rx+WHERE+rx_sign%3D%27{encoded_call}%27+ORDER+BY+time+DESC+LIMIT+50+FORMAT+JSONEachRow"
    );
    let resp = crate::core::http::http_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP Error: {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let mut spots = Vec::new();
    for line in body.lines() {
        if let Ok(spot) = serde_json::from_str::<WsprSpot>(line) {
            spots.push(spot);
        }
    }
    Ok(spots)
}
