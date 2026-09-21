// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Dane korespondenta pobrane z internetowej bazy danych (QRZ.COM / HamQTH)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CallbookData {
    pub callsign: String,
    pub name: Option<String>,
    pub qth: Option<String>,
    pub gridsquare: Option<String>,
    pub state: Option<String>,
    pub dxcc: Option<u32>,
    pub country: Option<String>,
    pub qsl_manager: Option<String>,
    pub email: Option<String>,
    pub image_url: Option<String>,
}

/// Bezpieczny klient HTTPS do QRZ.COM XML Data Service
pub struct QrzClient {
    client: Client,
    username: String,
    password: String,
    session_key: Option<String>,
}

impl QrzClient {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("SPLogbook/1.0.0 (SP6INA)")
            .build()
            .unwrap_or_default();

        Self {
            client,
            username: username.into(),
            password: password.into(),
            session_key: None,
        }
    }

    /// Loguje się do QRZ.COM i pobiera token sesji
    pub async fn login(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let resp = self
            .client
            .get("https://xmldata.qrz.com/xml/current/")
            .query(&[
                ("username", &self.username),
                ("password", &self.password),
                ("agent", &"SPLogbook".to_string()),
            ])
            .send()
            .await?
            .text()
            .await?;

        if let Some(key) = Self::extract_xml_tag(&resp, "Key") {
            self.session_key = Some(key);
            Ok(())
        } else if let Some(err) = Self::extract_xml_tag(&resp, "Error") {
            Err(format!("Błąd logowania QRZ.COM: {}", err).into())
        } else {
            Err("Nieznana odpowiedź serwera QRZ.COM".into())
        }
    }

    /// Pobiera dane stacji po znaku wywoławczym
    pub async fn lookup(&mut self, callsign: &str) -> Result<CallbookData, Box<dyn std::error::Error>> {
        if self.session_key.is_none() {
            self.login().await?;
        }

        let key = self.session_key.as_ref().unwrap();
        let resp = self
            .client
            .get("https://xmldata.qrz.com/xml/current/")
            .query(&[("s", key), ("callsign", &callsign.to_string())])
            .send()
            .await?
            .text()
            .await?;

        // Jeśli sesja wygasła, zaloguj się ponownie i ponów
        if resp.contains("Session Timeout") || resp.contains("Invalid session key") {
            self.login().await?;
            let new_key = self.session_key.as_ref().unwrap();
            let retry_resp = self
                .client
                .get("https://xmldata.qrz.com/xml/current/")
                .query(&[("s", new_key), ("callsign", &callsign.to_string())])
                .send()
                .await?
                .text()
                .await?;
            return Self::parse_qrz_response(&retry_resp, callsign);
        }

        Self::parse_qrz_response(&resp, callsign)
    }

    fn parse_qrz_response(xml: &str, callsign: &str) -> Result<CallbookData, Box<dyn std::error::Error>> {
        if let Some(err) = Self::extract_xml_tag(xml, "Error") {
            return Err(format!("QRZ: {}", err).into());
        }

        let fname = Self::extract_xml_tag(xml, "fname");
        let name = Self::extract_xml_tag(xml, "name");
        let full_name = match (fname, name) {
            (Some(f), Some(n)) => Some(format!("{} {}", f, n)),
            (Some(f), None) => Some(f),
            (None, Some(n)) => Some(n),
            (None, None) => None,
        };

        let qth = Self::extract_xml_tag(xml, "addr2");
        let gridsquare = Self::extract_xml_tag(xml, "grid");
        let state = Self::extract_xml_tag(xml, "state");
        let dxcc = Self::extract_xml_tag(xml, "dxcc").and_then(|d| d.parse().ok());
        let country = Self::extract_xml_tag(xml, "country");
        let qsl_manager = Self::extract_xml_tag(xml, "qslmgr");
        let email = Self::extract_xml_tag(xml, "email");
        let image_url = Self::extract_xml_tag(xml, "image");

        Ok(CallbookData {
            callsign: callsign.to_uppercase(),
            name: full_name,
            qth,
            gridsquare,
            state,
            dxcc,
            country,
            qsl_manager,
            email,
            image_url,
        })
    }

    fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
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

    /// Przesyła rekordy ADIF do QRZ.com Logbook API za pomocą klucza API
    pub async fn upload_to_logbook(api_key: &str, adif_content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("SPLogbook/1.0.0 (SP6INA)")
            .build()?;

        let resp = client
            .post("https://logbook.qrz.com/api")
            .form(&[
                ("KEY", api_key),
                ("ACTION", "INSERT"),
                ("ADIF", adif_content),
            ])
            .send()
            .await?;

        let body = resp.text().await?;
        if body.contains("RESULT=OK") || body.contains("STATUS=OK") || body.contains("COUNT=") {
            Ok(format!("QRZ Logbook sukces: {}", body.trim()))
        } else {
            Err(format!("QRZ Logbook błąd: {}", body.trim()).into())
        }
    }
}
