// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Integracja z programem emisji cyfrowych FLDIGI za pośrednictwem protokołu XML-RPC

#[derive(Debug, Clone)]
pub struct FldigiClient {
    pub endpoint: String,
}

#[derive(Debug, Clone, Default)]
pub struct FldigiQsoState {
    pub call: String,
    pub name: String,
    pub qth: String,
    pub locator: String,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub freq_hz: f64,
    pub mode: String,
}

impl FldigiClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            endpoint: format!("http://{}:{}/RPC2", host, port),
        }
    }

    /// Sprawdza czy FLDIGI jest uruchomiony i zwraca wersję programu
    pub async fn get_version(&self) -> Result<String, String> {
        self.call_xmlrpc("fldigi.version", "").await
    }

    /// Pobiera bieżącą częstotliwość odbiornika/nadajnika z FLDIGI (w Hz)
    pub async fn get_frequency(&self) -> Result<f64, String> {
        let resp = self.call_xmlrpc("main.get_frequency", "").await?;
        resp.trim().parse::<f64>().map_err(|e| e.to_string())
    }

    /// Ustawia częstotliwość w FLDIGI (w Hz)
    pub async fn set_frequency(&self, freq_hz: f64) -> Result<(), String> {
        let param = format!("<param><value><double>{}</double></value></param>", freq_hz);
        self.call_xmlrpc("main.set_frequency", &param).await?;
        Ok(())
    }

    /// Pobiera aktywną modulację (PSK31, RTTY, Olivia itd.)
    pub async fn get_mode(&self) -> Result<String, String> {
        self.call_xmlrpc("modem.get_name", "").await
    }

    /// Pobiera aktualnie wprowadzane dane łączności z okna logu FLDIGI
    pub async fn get_qso_data(&self) -> Result<FldigiQsoState, String> {
        let call = self.call_xmlrpc("log.get_call", "").await.unwrap_or_default();
        let name = self.call_xmlrpc("log.get_name", "").await.unwrap_or_default();
        let qth = self.call_xmlrpc("log.get_qth", "").await.unwrap_or_default();
        let locator = self.call_xmlrpc("log.get_locator", "").await.unwrap_or_default();
        let rst_sent = self.call_xmlrpc("log.get_rst_out", "").await.unwrap_or_else(|_| "599".to_string());
        let rst_rcvd = self.call_xmlrpc("log.get_rst_in", "").await.unwrap_or_else(|_| "599".to_string());
        let freq_hz = self.get_frequency().await.unwrap_or(0.0);
        let mode = self.get_mode().await.unwrap_or_default();

        Ok(FldigiQsoState {
            call,
            name,
            qth,
            locator,
            rst_sent,
            rst_rcvd,
            freq_hz,
            mode,
        })
    }

    /// Przełącza FLDIGI w tryb nadawania TX
    pub async fn tx(&self) -> Result<(), String> {
        self.call_xmlrpc("main.tx", "").await?;
        Ok(())
    }

    /// Przełącza FLDIGI w tryb odbioru RX
    pub async fn rx(&self) -> Result<(), String> {
        self.call_xmlrpc("main.rx", "").await?;
        Ok(())
    }

    /// Wysyła surowe wywołanie metody XML-RPC do FLDIGI
    async fn call_xmlrpc(&self, method_name: &str, params_xml: &str) -> Result<String, String> {
        let body = format!(
            r#"<?xml version="1.0"?><methodCall><methodName>{}</methodName><params>{}</params></methodCall>"#,
            method_name, params_xml
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client
            .post(&self.endpoint)
            .header("Content-Type", "text/xml")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Błąd połączenia z FLDIGI XML-RPC: {}", e))?;

        let xml = resp
            .text()
            .await
            .map_err(|e| format!("Błąd odczytu odpowiedzi FLDIGI: {}", e))?;

        Self::parse_xmlrpc_value(&xml)
    }

    /// Ekstrahuje zawartość z tagów <value><string>...</string></value> lub <double>
    pub fn parse_xmlrpc_value(xml: &str) -> Result<String, String> {
        if xml.contains("<fault>") {
            return Err("FLDIGI zgłosiło błąd RPC Fault".to_string());
        }

        // Szukaj <string>...</string>
        if let Some(start) = xml.find("<string>") {
            if let Some(end) = xml[start + 8..].find("</string>") {
                return Ok(xml[start + 8..start + 8 + end].to_string());
            }
        }

        // Szukaj <double>...</double>
        if let Some(start) = xml.find("<double>") {
            if let Some(end) = xml[start + 8..].find("</double>") {
                return Ok(xml[start + 8..start + 8 + end].to_string());
            }
        }

        // Szukaj <i4>...</i4> lub <int>...</int>
        if let Some(start) = xml.find("<i4>") {
            if let Some(end) = xml[start + 4..].find("</i4>") {
                return Ok(xml[start + 4..start + 4 + end].to_string());
            }
        }

        // Szukaj <value>...</value>
        if let Some(start) = xml.find("<value>") {
            if let Some(end) = xml[start + 7..].find("</value>") {
                let inner = &xml[start + 7..start + 7 + end];
                if !inner.starts_with('<') {
                    return Ok(inner.to_string());
                }
            }
        }

        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmlrpc_parser() {
        let xml_str = r#"<?xml version="1.0"?><methodResponse><params><param><value><string>BPSK31</string></value></param></params></methodResponse>"#;
        let val = FldigiClient::parse_xmlrpc_value(xml_str).unwrap();
        assert_eq!(val, "BPSK31");

        let xml_double = r#"<?xml version="1.0"?><methodResponse><params><param><value><double>14070000</double></value></param></params></methodResponse>"#;
        let freq = FldigiClient::parse_xmlrpc_value(xml_double).unwrap();
        assert_eq!(freq, "14070000");
    }
}
