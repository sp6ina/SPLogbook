// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient Cloudlog API (Cloudlog REST API JSON)

use crate::core::adif::export_adif;
use crate::core::i18n::{Msg, key};
use crate::core::qso::QsoRecord;
use serde_json::json;

#[derive(Debug)]
pub enum CloudlogError {
    NotConfigured,
    Http(String),
    Server { status: u16, body: String },
}

impl std::fmt::Display for CloudlogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Cloudlog server URL or API key is not configured"),
            Self::Http(detail) => write!(f, "HTTP error: {detail}"),
            Self::Server { status, body } => write!(f, "Cloudlog server error [{status}]: {body}"),
        }
    }
}

impl std::error::Error for CloudlogError {}

impl CloudlogError {
    pub fn msg(&self) -> Msg {
        match self {
            Self::NotConfigured => Msg::new(key::CLOUDLOG_ERR_NOT_CONFIGURED),
            Self::Http(detail) => Msg::new(key::CLOUDLOG_ERR_HTTP).arg("detail", detail),
            Self::Server { status, body } => Msg::new(key::CLOUDLOG_ERR_SERVER)
                .arg("status", *status)
                .arg("body", body),
        }
    }
}

pub struct CloudlogClient {
    pub server_url: String,
    pub api_key: String,
    pub station_id: String,
}

impl CloudlogClient {
    pub fn new(server_url: &str, api_key: String, station_id: String) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            api_key,
            station_id,
        }
    }

    /// Wysyła łączność do Cloudlog przez oficjalne REST API
    pub async fn upload_qso(&self, qso: &QsoRecord) -> Result<String, CloudlogError> {
        if self.server_url.is_empty() || self.api_key.is_empty() {
            return Err(CloudlogError::NotConfigured);
        }

        let station_call = if self.station_id.trim().is_empty() {
            "SPLogbook"
        } else {
            self.station_id.trim()
        };
        let adif_record = export_adif(std::slice::from_ref(qso), "SPLogbook", station_call);
        let endpoint = format!("{}/index.php/api/qso", self.server_url);

        let payload = json!({
            "key": self.api_key,
            "station_profile_id": self.station_id,
            "type": "adif",
            "string": adif_record
        });

        let resp = crate::core::http::retry_async(
            || {
                let endpoint = endpoint.clone();
                let payload = payload.clone();
                async move {
                    crate::core::http::http_client()
                        .post(&endpoint)
                        .header("Content-Type", "application/json")
                        .json(&payload)
                        .send()
                        .await
                        .map_err(|e| crate::core::http::sanitize_network_error(&e))
                }
            },
            3,
        )
        .await
        .map_err(CloudlogError::Http)?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści odpowiedzi".to_string());

        if status.is_success() {
            Ok(format!("Pomyślnie przesłano do Cloudlog: {body}"))
        } else {
            Err(CloudlogError::Server {
                status: status.as_u16(),
                body,
            })
        }
    }
}
