// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient serwisu HamQTH (Real-time QSO Upload)

use crate::core::adif::export_adif;
use crate::core::qso::QsoRecord;

pub struct HamQthClient {
    pub username: String,
    pub password: String,
}

impl HamQthClient {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// Wysyła łączność do serwisu HamQTH w czasie rzeczywistym
    pub async fn upload_qso(&self, qso: &QsoRecord) -> Result<String, String> {
        if self.username.is_empty() || self.password.is_empty() {
            return Err("Brak skonfigurowanego loginu lub hasła do HamQTH.com".to_string());
        }

        let adif_text = export_adif(std::slice::from_ref(qso), "SPLogbook", &self.username);
        let endpoint = "http://www.hamqth.com/qso_realtime.php";

        let params = [
            ("u", self.username.as_str()),
            ("p", self.password.as_str()),
            ("adif", adif_text.as_str()),
        ];

        let client = reqwest::Client::new();
        let resp = client
            .post(endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Błąd wysyłania do HamQTH.com: {}", e))?;

        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści".to_string());

        if body.contains("OK") {
            Ok("Pomyślnie dodano łączność do HamQTH.com".to_string())
        } else {
            Err(format!("Odpowiedź HamQTH.com: {}", body))
        }
    }
}

use crate::cloud::qrz::CallbookData;

/// Darmowy klient XML Callbook do serwisu HamQTH.com (OK1RR)
pub struct HamQthXmlClient {
    client: reqwest::Client,
    pub username: String,
    pub password: String,
    session_id: Option<String>,
}

impl HamQthXmlClient {
    pub fn new(username: String, password: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("SPLogbook/1.0.0 (SP6INA)")
            .build()
            .unwrap_or_default();

        Self {
            client,
            username,
            password,
            session_id: None,
        }
    }

    /// Logowanie do darmowego API XML HamQTH i pobranie session_id
    pub async fn login(&mut self) -> Result<String, String> {
        if self.username.is_empty() || self.password.is_empty() {
            return Err("Brak danych logowania do HamQTH".to_string());
        }

        let resp = self
            .client
            .get("https://www.hamqth.com/xml.php")
            .query(&[("u", &self.username), ("p", &self.password)])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let xml = resp.text().await.map_err(|e| e.to_string())?;

        if let Some(sid) = Self::extract_tag(&xml, "session_id") {
            self.session_id = Some(sid.clone());
            Ok(sid)
        } else if let Some(err) = Self::extract_tag(&xml, "error") {
            Err(format!("Błąd logowania HamQTH: {}", err))
        } else {
            Err("Nieznana odpowiedź autoryzacji HamQTH".to_string())
        }
    }

    /// Pobiera pełne dane korespondenta (w tym zdjęcie profilowe) z bazy HamQTH
    pub async fn lookup_callsign(&mut self, callsign: &str) -> Result<CallbookData, String> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return Err("Pusty znak".to_string());
        }

        if self.session_id.is_none() {
            self.login().await?;
        }

        let sid = self.session_id.as_ref().unwrap();
        let url = format!(
            "https://www.hamqth.com/xml.php?id={}&callsign={}&prg=SPLogbook",
            sid, clean
        );

        let resp = self.client.get(&url).send().await.map_err(|e| e.to_string())?;
        let xml = resp.text().await.map_err(|e| e.to_string())?;

        // Jeśli sesja wygasła, zaloguj się ponownie
        if xml.contains("Session does not exist") || xml.contains("session expired") {
            self.session_id = None;
            self.login().await?;
            let sid2 = self.session_id.as_ref().unwrap();
            let url2 = format!(
                "https://www.hamqth.com/xml.php?id={}&callsign={}&prg=SPLogbook",
                sid2, clean
            );
            let resp2 = self.client.get(&url2).send().await.map_err(|e| e.to_string())?;
            let xml2 = resp2.text().await.map_err(|e| e.to_string())?;
            return Self::parse_hamqth_xml(&xml2, &clean);
        }

        Self::parse_hamqth_xml(&xml, &clean)
    }

    fn parse_hamqth_xml(xml: &str, query_call: &str) -> Result<CallbookData, String> {
        if xml.contains("<error>") {
            return Err("Nie znaleziono znaku w HamQTH".to_string());
        }

        let call = Self::extract_tag(xml, "callsign").unwrap_or_else(|| query_call.to_string());
        let name = Self::extract_tag(xml, "nick").or_else(|| Self::extract_tag(xml, "name"));
        let qth = Self::extract_tag(xml, "qth");
        let grid = Self::extract_tag(xml, "grid");
        let country = Self::extract_tag(xml, "country");
        let dxcc = Self::extract_tag(xml, "adif").and_then(|d| d.parse::<u32>().ok());
        let qsl_manager = Self::extract_tag(xml, "qsl");
        let image_url = Self::extract_tag(xml, "picture");

        Ok(CallbookData {
            callsign: call,
            name,
            qth,
            gridsquare: grid,
            state: Self::extract_tag(xml, "us_state"),
            dxcc,
            country,
            qsl_manager,
            email: Self::extract_tag(xml, "email"),
            image_url,
        })
    }

    fn extract_tag(xml: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);
        let start = xml.find(&open_tag)? + open_tag.len();
        let end = xml[start..].find(&close_tag)?;
        let content = &xml[start..start + end];
        if content.is_empty() {
            None
        } else {
            Some(content.trim().to_string())
        }
    }
}
