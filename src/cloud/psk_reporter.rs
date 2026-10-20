// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, Msg};
use crate::core::qso::QsoRecord;
use crate::core::xml::escape_xml as escape_xml_attr;

#[derive(Debug)]
pub enum PskReporterError {
    MissingFrequency,
    Http(String),
    Server(u16),
}

impl std::fmt::Display for PskReporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFrequency => {
                write!(f, "Missing QSO frequency — cannot send to PSK Reporter")
            }
            Self::Http(detail) => write!(f, "HTTP error: {detail}"),
            Self::Server(status) => write!(f, "Server returned: {status}"),
        }
    }
}

impl std::error::Error for PskReporterError {}

impl PskReporterError {
    pub fn msg(&self) -> Msg {
        match self {
            Self::MissingFrequency => Msg::new(key::PSK_ERR_MISSING_FREQ),
            Self::Http(detail) => Msg::new(key::PSK_ERR_HTTP).arg("detail", detail),
            Self::Server(status) => Msg::new(key::PSK_ERR_SERVER).arg("status", *status),
        }
    }
}

pub struct PskReporterClient {
    callsign: String,
    gridsquare: String,
}

impl PskReporterClient {
    pub fn new(callsign: &str, gridsquare: &str) -> Self {
        Self {
            callsign: callsign.to_string(),
            gridsquare: gridsquare.to_string(),
        }
    }

    pub fn build_report_xml(&self, dx_call: &str, freq_hz: u64, mode: &str, snr: i32) -> String {
        format!(
            r#"<receptionReport reporter="{}" reporterLocator="{}" callsign="{}" frequency="{}" mode="{}" snr="{}" />"#,
            escape_xml_attr(&self.callsign),
            escape_xml_attr(&self.gridsquare),
            escape_xml_attr(dx_call),
            freq_hz,
            escape_xml_attr(mode),
            snr
        )
    }

    pub async fn submit_spot(
        &self,
        dx_call: &str,
        freq_hz: u64,
        mode: &str,
        snr: i32,
    ) -> Result<(), PskReporterError> {
        let xml = self.build_report_xml(dx_call, freq_hz, mode, snr);
        let resp = crate::core::http::retry_async(
            || {
                let xml = xml.clone();
                async move {
                    crate::core::http::http_client()
                        .post("https://www.pskreporter.info/cgi-bin/pskr/upload.pl")
                        .body(xml)
                        .send()
                        .await
                        .map_err(|e| e.to_string())
                }
            },
            3,
        )
        .await
        .map_err(PskReporterError::Http)?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(PskReporterError::Server(resp.status().as_u16()))
        }
    }

    pub async fn submit_reception_report(&self, qso: &QsoRecord) -> Result<(), PskReporterError> {
        // Wymagamy podania częstotliwości — nie wysyłamy z domyślną wartością 14 MHz
        let freq_mhz = qso.freq.ok_or(PskReporterError::MissingFrequency)?;
        let freq_hz = (freq_mhz * 1_000_000.0) as u64;
        self.submit_spot(&qso.callsign, freq_hz, &qso.mode, 0).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psk_reporter_xml_escaping() {
        let client = PskReporterClient::new("SP6INA\"&<>'", "JO81");
        let xml = client.build_report_xml("W1AW\"<>&'", 14074000, "FT8", -10);

        assert!(xml.contains("reporter=\"SP6INA&quot;&amp;&lt;&gt;&apos;\""));
        assert!(xml.contains("callsign=\"W1AW&quot;&lt;&gt;&amp;&apos;\""));
        assert!(xml.contains("frequency=\"14074000\""));
        assert!(xml.contains("mode=\"FT8\""));
        assert!(xml.contains("snr=\"-10\""));
    }
}
