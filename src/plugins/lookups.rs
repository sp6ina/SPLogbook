// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Odpytywanie zewnętrznych baz POTA i SOTA (tryb tylko do odczytu).
//! Używane przez akcje pluginów `pota_lookup` / `sota_lookup`; wyniki trafiają
//! do haków `on_pota_info` / `on_sota_info` w silniku Rhai.

use serde::Deserialize;

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
pub async fn lookup_pota(reference: &str) -> Result<PotaInfo, String> {
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
        .map_err(|e| format!("Błąd zapytania POTA: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Serwer POTA zwrócił status {}", resp.status()));
    }

    let data: PotaResponse = resp.json().await.map_err(|e| format!("POTA JSON: {e}"))?;
    Ok(PotaInfo {
        reference: if data.reference.is_empty() { clean } else { data.reference },
        name: data.name,
        active: data.active,
    })
}

/// Pobiera dane szczytu SOTA z `api2.sota.org.uk/api/summits/{reference}`.
pub async fn lookup_sota(reference: &str) -> Result<SotaInfo, String> {
    // Referencje SOTA zawierają ukośnik (np. SP/BZ-001) — zostaw go w URL.
    let clean = reference.trim();
    let url = format!("https://api2.sota.org.uk/api/summits/{clean}");
    let client = crate::core::http::http_client_with_timeout(15);
    let resp = client
        .get(&url)
        .header("User-Agent", crate::core::http::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Błąd zapytania SOTA: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Serwer SOTA zwrócił status {}", resp.status()));
    }

    let data: SotaResponse = resp.json().await.map_err(|e| format!("SOTA JSON: {e}"))?;
    Ok(SotaInfo {
        reference: clean.to_string(),
        name: data.name,
        points: data.points,
    })
}
