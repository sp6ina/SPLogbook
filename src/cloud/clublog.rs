// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;

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
        let client = crate::core::http::http_client_with_timeout(30);

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
        let client = self.client.clone();
        let email = email.to_string();
        let password = password.to_string();
        let callsign = callsign.to_uppercase();
        let api = if api_key.is_empty() {
            "splogbook_api_key".to_string()
        } else {
            api_key.to_string()
        };
        let adif_bytes = adif_content.as_bytes().to_vec();

        let resp = crate::core::http::retry_async(
            || {
                let client = client.clone();
                let email = email.clone();
                let password = password.clone();
                let callsign = callsign.clone();
                let api = api.clone();
                let adif_bytes = adif_bytes.clone();
                async move {
                    let form = reqwest::multipart::Form::new()
                        .text("email", email)
                        .text("password", password)
                        .text("callsign", callsign)
                        .text("api", api)
                        .part(
                            "file",
                            reqwest::multipart::Part::bytes(adif_bytes)
                                .file_name("clublog_upload.adi")
                                .mime_str("application/octet-stream")?,
                        );
                    client
                        .post("https://clublog.org/putfile.php")
                        .multipart(form)
                        .send()
                        .await
                        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
                }
            },
            3,
        )
        .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if status.is_success()
            && (body.contains("accepted")
                || body.contains("Uploaded")
                || body.contains("queued")
                || body.contains("OK"))
        {
            Ok(format!(
                "Club Log: Łączności przyjęte pomyślnie. ({})",
                body.trim()
            ))
        } else {
            Err(format!("Club Log błąd: {} - {}", status, body.trim()).into())
        }
    }
}
