// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;
use std::time::Duration;

/// Klient API serwisu Club Log (https://clublog.org)
pub struct ClubLogClient {
    client: Client,
}

impl Default for ClubLogClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ClubLogClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("SPLogbook/1.0.0 (SP6INA)")
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Przesyła plik łączności ADIF do serwisu Club Log
    pub async fn upload_adif(
        &self,
        callsign: &str,
        email: &str,
        password: &str,
        api_key: &str,
        adif_content: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let form = reqwest::multipart::Form::new()
            .text("email", email.to_string())
            .text("password", password.to_string())
            .text("callsign", callsign.to_uppercase())
            .text("api", if api_key.is_empty() { "splogbook_api_key".to_string() } else { api_key.to_string() })
            .part(
                "file",
                reqwest::multipart::Part::bytes(adif_content.as_bytes().to_vec())
                    .file_name("clublog_upload.adi")
                    .mime_str("application/octet-stream")?,
            );

        let resp = self
            .client
            .post("https://clublog.org/putfile.php")
            .multipart(form)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if status.is_success() && (body.contains("accepted") || body.contains("Uploaded") || body.contains("queued") || body.contains("OK")) {
            Ok(format!("Club Log: Łączności przyjęte pomyślnie. ({})", body.trim()))
        } else {
            Err(format!("Club Log błąd: {} - {}", status, body.trim()).into())
        }
    }
}
