use crate::core::qso::QsoRecord;

fn escape_xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
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
    ) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_report_xml_escapes_special_chars() {
        let client = PskReporterClient::new("SP6INA\"&<>'", "JO81");
        let xml = client.build_report_xml("DL1ABC<test>", 14_074_000, "FT8", -10);
        assert!(xml.contains(r#"reporter="SP6INA&quot;&amp;&lt;&gt;&apos;""#));
        assert!(xml.contains(r#"callsign="DL1ABC&lt;test&gt;""#));
    }
}
