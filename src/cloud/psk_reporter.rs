use crate::core::qso::QsoRecord;

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

    pub async fn submit_spot(&self, dx_call: &str, freq_hz: u64, mode: &str, snr: i32) -> Result<(), String> {
        let xml = format!(
            r#"<receptionReport reporter="{}" reporterLocator="{}" callsign="{}" frequency="{}" mode="{}" snr="{}" />"#,
            self.callsign, self.gridsquare, dx_call, freq_hz, mode, snr
        );
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
        .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("Server returned: {}", resp.status()))
        }
    }

    pub async fn submit_reception_report(&self, qso: &QsoRecord) -> Result<(), String> {
        // Wymagamy podania czestotliwosci — nie wysylamy z domyslna wartoscia 14 MHz
        let freq_mhz = qso.freq.ok_or_else(|| {
            "Brak czestotliwosci QSO — nie mozna wyslac do PSK Reporter".to_string()
        })?;
        let freq_hz = (freq_mhz * 1_000_000.0) as u64;
        self.submit_spot(&qso.callsign, freq_hz, &qso.mode, 0).await
    }
}
