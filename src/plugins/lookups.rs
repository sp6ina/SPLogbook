// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Odpytywanie zewnętrznych baz POTA i SOTA (tryb tylko do odczytu).
//! Używane przez akcje pluginów `pota_lookup` / `sota_lookup`; wyniki trafiają
//! do haków `on_pota_info` / `on_sota_info` w silniku Rhai.

use serde::Deserialize;

/// Błędy zapytań do zewnętrznych serwisów referencyjnych (POTA / SOTA).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    Http(String),
    HttpStatus {
        service: &'static str,
        status: u16,
    },
    JsonParse {
        service: &'static str,
        detail: String,
    },
}

impl std::fmt::Display for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP request error: {e}"),
            Self::HttpStatus { service, status } => {
                write!(f, "{service} server returned status {status}")
            }
            Self::JsonParse { service, detail } => {
                write!(f, "Failed to parse {service} response: {detail}")
            }
        }
    }
}

impl std::error::Error for LookupError {}

/// Informacja o parku POTA.
#[derive(Debug, Clone, Default)]
pub struct PotaInfo {
    pub reference: String,
    pub name: String,
    pub active: bool,
}

/// Informacja o szczycie SOTA.
#[derive(Debug, Clone, Default)]
pub struct SotaInfo {
    pub reference: String,
    pub name: String,
    pub points: i64,
}

#[derive(Debug, Deserialize)]
struct PotaResponse {
    #[serde(default)]
    reference: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    active: bool,
}

#[derive(Debug, Deserialize)]
struct SotaResponse {
    #[serde(default)]
    name: String,
    #[serde(default)]
    points: i64,
}

/// Pobiera dane parku POTA z `api.pota.app/park/{reference}`.
pub async fn lookup_pota(reference: &str) -> Result<PotaInfo, LookupError> {
    let clean: String = reference
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let url = format!("https://api.pota.app/park/{clean}");
    let client = crate::core::http::http_client_with_timeout(15);
    let resp = client
        .get(&url)
        .header("User-Agent", crate::core::http::USER_AGENT)
        .send()
        .await
        .map_err(|e| LookupError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LookupError::HttpStatus {
            service: "POTA",
            status: resp.status().as_u16(),
        });
    }

    let data: PotaResponse = resp.json().await.map_err(|e| LookupError::JsonParse {
        service: "POTA",
        detail: e.to_string(),
    })?;
    Ok(PotaInfo {
        reference: if data.reference.is_empty() {
            clean
        } else {
            data.reference
        },
        name: data.name,
        active: data.active,
    })
}

/// Pobiera dane szczytu SOTA z `api2.sota.org.uk/api/summits/{reference}`.
pub async fn lookup_sota(reference: &str) -> Result<SotaInfo, LookupError> {
    // Referencje SOTA zawierają ukośnik (np. SP/BZ-001). Filtrujemy do dozwolonych
    // znaków (alfanumeryczne, '/' i '-'), by uniknąć wstrzyknięcia znaków do URL.
    let clean: String = reference
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-')
        .collect();
    let url = format!("https://api2.sota.org.uk/api/summits/{clean}");
    let client = crate::core::http::http_client_with_timeout(15);
    let resp = client
        .get(&url)
        .header("User-Agent", crate::core::http::USER_AGENT)
        .send()
        .await
        .map_err(|e| LookupError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LookupError::HttpStatus {
            service: "SOTA",
            status: resp.status().as_u16(),
        });
    }

    let data: SotaResponse = resp.json().await.map_err(|e| LookupError::JsonParse {
        service: "SOTA",
        detail: e.to_string(),
    })?;
    Ok(SotaInfo {
        reference: clean,
        name: data.name,
        points: data.points,
    })
}
