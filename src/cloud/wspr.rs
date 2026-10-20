// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, Msg};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WsprSpot {
    pub callsign: String,
    pub frequency: f64,
    pub snr: i32,
    pub gridsquare: String,
}

#[derive(Debug, Clone)]
pub enum WsprError {
    InvalidCallsign,
    Http(String),
    Server(u16),
}

impl std::fmt::Display for WsprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCallsign => write!(f, "Invalid or empty callsign"),
            Self::Http(detail) => write!(f, "HTTP error: {detail}"),
            Self::Server(status) => write!(f, "Server returned: {status}"),
        }
    }
}

impl std::error::Error for WsprError {}

impl WsprError {
    pub fn msg(&self) -> Msg {
        match self {
            Self::InvalidCallsign => Msg::new(key::WSPR_ERR_INVALID_CALLSIGN),
            Self::Http(detail) => Msg::new(key::WSPR_ERR_HTTP).arg("detail", detail),
            Self::Server(status) => Msg::new(key::WSPR_ERR_SERVER).arg("status", *status),
        }
    }
}

pub async fn fetch_wspr_spots(my_callsign: &str) -> Result<Vec<WsprSpot>, WsprError> {
    let sanitized: String = my_callsign
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '/')
        .collect();
    if sanitized.is_empty() {
        return Err(WsprError::InvalidCallsign);
    }
    let encoded_call = sanitized.replace('/', "%2F");
    let url = format!(
        "https://db1.wspr.live/?query=SELECT+callsign,frequency,snr,drift,gridsquare+FROM+wspr.rx+WHERE+rx_sign%3D%27{encoded_call}%27+ORDER+BY+time+DESC+LIMIT+50+FORMAT+JSONEachRow"
    );
    let resp = crate::core::http::http_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| WsprError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(WsprError::Server(resp.status().as_u16()));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| WsprError::Http(e.to_string()))?;
    let mut spots = Vec::new();
    for line in body.lines() {
        if let Ok(spot) = serde_json::from_str::<WsprSpot>(line) {
            spots.push(spot);
        }
    }
    Ok(spots)
}
