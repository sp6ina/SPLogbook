// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient serwisu HamQTH (Real-time QSO Upload)

use crate::core::adif::export_adif;
use crate::core::qso::QsoRecord;
use crate::core::xml::extract_tag;

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
                        .map_err(|e| format!("Błąd wysyłania do HamQTH.com: {e}"))
                }
            },
            3,
        )
        .await?;

        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści".to_string());

        if body.contains("OK") {
            Ok("Pomyślnie dodano łączność do HamQTH.com".to_string())
        } else {
            Err(format!("Odpowiedź HamQTH.com: {body}"))
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
        let client = crate::core::http::http_client_with_timeout(10);

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

        if let Some(sid) = extract_tag(&xml, "session_id") {
            self.session_id = Some(sid.clone());
            Ok(sid)
        } else if let Some(err) = extract_tag(&xml, "error") {
            Err(format!("Błąd logowania HamQTH: {err}"))
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

        let sid = self
            .session_id
            .as_ref()
            .ok_or_else(|| "Nie udało się utworzyć sesji HamQTH".to_string())?;

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
            .map_err(|e| e.to_string())?;
        let xml = resp.text().await.map_err(|e| e.to_string())?;

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
                .ok_or_else(|| "Nie udało się utworzyć sesji HamQTH".to_string())?;
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
                .map_err(|e| e.to_string())?;
            let xml2 = resp2.text().await.map_err(|e| e.to_string())?;
            return Self::parse_hamqth_xml(&xml2, &clean);
        }

        Self::parse_hamqth_xml(&xml, &clean)
    }

    fn parse_hamqth_xml(xml: &str, query_call: &str) -> Result<CallbookData, String> {
        if xml.contains("<error>") {
            return Err("Nie znaleziono znaku w HamQTH".to_string());
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
