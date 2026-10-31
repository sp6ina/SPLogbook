// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient serwisu HRDLog.net (XML Upload)

use crate::core::adif::export_adif;
use crate::core::i18n::{Msg, key};
use crate::core::qso::QsoRecord;

#[derive(Debug)]
pub enum HrdlogError {
    NotConfigured,
    Http(String),
    Server(String),
}

impl std::fmt::Display for HrdlogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => {
                write!(
                    f,
                    "HRDLog.net callsign or authorization code is not configured"
                )
            }
            Self::Http(detail) => write!(f, "HTTP error: {detail}"),
            Self::Server(detail) => write!(f, "HRDLog.net response: {detail}"),
        }
    }
}

impl std::error::Error for HrdlogError {}

impl HrdlogError {
    pub fn msg(&self) -> Msg {
        match self {
            Self::NotConfigured => Msg::new(key::HRDLOG_ERR_NOT_CONFIGURED),
            Self::Http(detail) => Msg::new(key::HRDLOG_ERR_HTTP).arg("detail", detail),
            Self::Server(detail) => Msg::new(key::HRDLOG_ERR_SERVER).arg("detail", detail),
        }
    }
}

pub struct HrdlogClient {
    pub callsign: String,
    pub upload_code: String,
}

impl HrdlogClient {
    pub fn new(callsign: String, upload_code: String) -> Self {
        Self {
            callsign,
            upload_code,
        }
    }

    /// Wysyła łączność do serwisu HRDLog.net
    pub async fn upload_qso(&self, qso: &QsoRecord) -> Result<String, HrdlogError> {
        if self.callsign.is_empty() || self.upload_code.is_empty() {
            return Err(HrdlogError::NotConfigured);
        }

        let adif_text = export_adif(std::slice::from_ref(qso), "SPLogbook", &self.callsign);
        let endpoint = "https://robot.hrdlog.net/NewEntry.aspx";

        let params = [
            ("Callsign", self.callsign.as_str()),
            ("Code", self.upload_code.as_str()),
            ("ADIFData", adif_text.as_str()),
        ];

        let resp = crate::core::http::retry_async(
            || {
                let params = params
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect::<Vec<(&str, &str)>>();
                async move {
                    crate::core::http::http_client()
                        .post(endpoint)
                        .form(&params)
                        .send()
                        .await
                        .map_err(|e| crate::core::http::sanitize_network_error(&e))
                }
            },
            3,
        )
        .await
        .map_err(HrdlogError::Http)?;

        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści odpowiedzi".to_string());

        if text.to_lowercase().contains("<insert>ok</insert>") || text.contains("OK") {
            Ok("Pomyślnie dodano łączność do HRDLog.net".to_string())
        } else {
            Err(HrdlogError::Server(text))
        }
    }
}
