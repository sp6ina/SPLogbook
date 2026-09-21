// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient Cloudlog API (Cloudlog REST API JSON)

use crate::core::adif::export_adif;
use crate::core::qso::QsoRecord;
use serde_json::json;

pub struct CloudlogClient {
    pub server_url: String,
    pub api_key: String,
    pub station_id: String,
}

impl CloudlogClient {
    pub fn new(server_url: String, api_key: String, station_id: String) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            api_key,
            station_id,
        }
    }

    /// Wysyła łączność do Cloudlog przez oficjalne REST API
    pub async fn upload_qso(&self, qso: &QsoRecord) -> Result<String, String> {
        if self.server_url.is_empty() || self.api_key.is_empty() {
            return Err("Brak skonfigurowanego adresu serwera Cloudlog lub klucza API".to_string());
        }

        let adif_record = export_adif(std::slice::from_ref(qso), "SPLogbook", "SP6INA");
        let endpoint = format!("{}/index.php/api/qso", self.server_url);

        let payload = json!({
            "key": self.api_key,
            "station_profile_id": self.station_id,
            "type": "adif",
            "string": adif_record
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Błąd połączenia z Cloudlog {}: {}", endpoint, e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści odpowiedzi".to_string());

        if status.is_success() {
            Ok(format!("Pomyślnie przesłano do Cloudlog: {}", body))
        } else {
            Err(format!("Błąd serwera Cloudlog [{}]: {}", status, body))
        }
    }
}
