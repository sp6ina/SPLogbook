// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient HamQTH.com (Realtime ADIF Upload & XML Callbook)

use crate::cloud::qrz::CallbookData;
use crate::core::adif::export_adif;
use crate::core::i18n::{key, Msg};
use crate::core::qso::QsoRecord;

#[derive(Debug)]
pub enum HamQthError {
    NotConfigured,
    Http(String),
    Server(String),
    LoginFailed(String),
    SessionCreateFailed,
    EmptyCallsign,
    NotFound,
    UnknownAuthResponse,
}

impl std::fmt::Display for HamQthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "HamQTH.com username or password is not configured"),
            Self::Http(detail) => write!(f, "HTTP error: {detail}"),
            Self::Server(detail) => write!(f, "HamQTH.com response: {detail}"),
            Self::LoginFailed(detail) => write!(f, "HamQTH login failed: {detail}"),
            Self::SessionCreateFailed => write!(f, "Failed to create HamQTH session"),
            Self::EmptyCallsign => write!(f, "Empty callsign"),
            Self::NotFound => write!(f, "Callsign not found in HamQTH"),
            Self::UnknownAuthResponse => write!(f, "Unknown HamQTH authorization response"),
        }
    }
}

impl std::error::Error for HamQthError {}

impl HamQthError {
    pub fn msg(&self) -> Msg {
        match self {
            Self::NotConfigured => Msg::new(key::HAMQTH_ERR_NOT_CONFIGURED),
            Self::Http(detail) => Msg::new(key::HAMQTH_ERR_HTTP).arg("detail", detail),
            Self::Server(detail) | Self::LoginFailed(detail) => {
                Msg::new(key::HAMQTH_ERR_SERVER).arg("detail", detail)
            }
            _ => Msg::new(key::HAMQTH_ERR_SERVER).arg("detail", self.to_string()),
        }
    }
}

pub struct HamQthClient {
    pub username: String,
    pub password: String,
}

impl HamQthClient {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// Wysyła łączność do serwisu HamQTH w czasie rzeczywistym
    pub async fn upload_qso(&self, qso: &QsoRecord) -> Result<String, HamQthError> {
        if self.username.is_empty() || self.password.is_empty() {
            return Err(HamQthError::NotConfigured);
        }

        let adif_text = export_adif(std::slice::from_ref(qso), "SPLogbook", &self.username);
        let endpoint = "https://www.hamqth.com/qso_realtime.php";

        let params = [
            ("u", self.username.as_str()),
            ("p", self.password.as_str()),
            ("adif", adif_text.as_str()),
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
                        .map_err(|e| e.to_string())
                }
            },
            3,
        )
        .await
        .map_err(HamQthError::Http)?;

        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści".to_string());

        if body.contains("OK") {
            Ok("Pomyślnie dodano łączność do HamQTH.com".to_string())
        } else {
            Err(HamQthError::Server(body))
        }
    }
}

/// Darmowy klient XML Callbook do serwisu HamQTH.com (OK1RR)
pub struct HamQthXmlClient {
    client: reqwest::Client,
    pub username: String,
    pub password: String,
    session_id: Option<String>,
}

impl HamQthXmlClient {
    pub fn new(username: String, password: String) -> Self {
        let client = crate::core::http::http_client_with_timeout(10);

        Self {
            client,
            username,
            password,
            session_id: None,
        }
    }

    /// Logowanie do darmowego API XML HamQTH i pobranie session_id
    pub async fn login(&mut self) -> Result<String, HamQthError> {
        if self.username.is_empty() || self.password.is_empty() {
            return Err(HamQthError::NotConfigured);
        }

        let resp = self
            .client
            .get("https://www.hamqth.com/xml.php")
            .query(&[("u", &self.username), ("p", &self.password)])
            .send()
            .await
            .map_err(|e| HamQthError::Http(e.to_string()))?;
        let xml = resp
            .text()
            .await
            .map_err(|e| HamQthError::Http(e.to_string()))?;

        if let Some(sid) = extract_tag(&xml, "session_id") {
            self.session_id = Some(sid.clone());
            Ok(sid)
        } else if let Some(err) = extract_tag(&xml, "error") {
            Err(HamQthError::LoginFailed(err))
        } else {
            Err(HamQthError::UnknownAuthResponse)
        }
    }

    /// Pobiera pełne dane korespondenta (w tym zdjęcie profilowe) z bazy HamQTH
    pub async fn lookup_callsign(&mut self, callsign: &str) -> Result<CallbookData, HamQthError> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return Err(HamQthError::EmptyCallsign);
        }

        if self.session_id.is_none() {
            self.login().await?;
        }

        let sid = self
            .session_id
            .as_ref()
            .ok_or(HamQthError::SessionCreateFailed)?;

        let resp = self
            .client
            .get("https://www.hamqth.com/xml.php")
            .query(&[
                ("id", sid.as_str()),
                ("callsign", clean.as_str()),
                ("prg", "SPLogbook"),
            ])
            .send()
            .await
            .map_err(|e| HamQthError::Http(e.to_string()))?;
        let xml = resp
            .text()
            .await
            .map_err(|e| HamQthError::Http(e.to_string()))?;

        // Jeśli sesja wygasła, zaloguj się ponownie
        let is_session_error = extract_tag(&xml, "error")
            .is_some_and(|err| err.to_ascii_lowercase().contains("session"))
            || xml.to_ascii_lowercase().contains("session does not exist")
            || xml.to_ascii_lowercase().contains("session expired");
        if is_session_error {
            self.session_id = None;
            self.login().await?;
            let sid2 = self
                .session_id
                .as_ref()
                .ok_or(HamQthError::SessionCreateFailed)?;
            let resp2 = self
                .client
                .get("https://www.hamqth.com/xml.php")
                .query(&[
                    ("id", sid2.as_str()),
                    ("callsign", clean.as_str()),
                    ("prg", "SPLogbook"),
                ])
                .send()
                .await
                .map_err(|e| HamQthError::Http(e.to_string()))?;
            let xml2 = resp2
                .text()
                .await
                .map_err(|e| HamQthError::Http(e.to_string()))?;
            return Self::parse_hamqth_xml(&xml2, &clean);
        }

        Self::parse_hamqth_xml(&xml, &clean)
    }

    fn parse_hamqth_xml(xml: &str, query_call: &str) -> Result<CallbookData, HamQthError> {
        if xml.contains("<error>") {
            return Err(HamQthError::NotFound);
        }

        let call = extract_tag(xml, "callsign").unwrap_or_else(|| query_call.to_string());
        let name = extract_tag(xml, "nick").or_else(|| extract_tag(xml, "name"));
        let qth = extract_tag(xml, "qth");
        let grid = extract_tag(xml, "grid");
        let country = extract_tag(xml, "country");
        let dxcc = extract_tag(xml, "adif").and_then(|d| d.parse::<u32>().ok());
        let qsl_manager = extract_tag(xml, "qsl");
        let image_url = extract_tag(xml, "picture");

        Ok(CallbookData {
            callsign: call,
            name,
            qth,
            gridsquare: grid,
            state: extract_tag(xml, "us_state"),
            dxcc,
            country,
            qsl_manager,
            email: extract_tag(xml, "email"),
            image_url,
        })
    }
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}
