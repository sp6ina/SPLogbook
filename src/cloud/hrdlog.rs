// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient serwisu HRDLog.net (XML Upload)

use crate::core::adif::export_adif;
use crate::core::qso::QsoRecord;

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
    pub async fn upload_qso(&self, qso: &QsoRecord) -> Result<String, String> {
        if self.callsign.is_empty() || self.upload_code.is_empty() {
            return Err(
                "Brak skonfigurowanego znaku lub kodu autoryzacyjnego HRDLog.net".to_string(),
            );
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
                        .map_err(|e| format!("Błąd wysyłania do HRDLog.net: {e}"))
                }
            },
            3,
        )
        .await?;

        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Brak treści odpowiedzi".to_string());

        if text.to_lowercase().contains("<insert>ok</insert>") || text.contains("OK") {
            Ok("Pomyślnie dodano łączność do HRDLog.net".to_string())
        } else {
            Err(format!("Odpowiedź HRDLog.net: {text}"))
        }
    }
}
