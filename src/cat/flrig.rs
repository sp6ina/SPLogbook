// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient XML-RPC dla demona FLRig (http://www.w1hkj.com/flrig-help/)

use quick_xml::events::Event;
use quick_xml::Reader;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Wartość XML-RPC używana w parametrach i odpowiedziach.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlRpcValue {
    Int(i64),
    Double(f64),
    Bool(bool),
    Str(String),
}

impl XmlRpcValue {
    /// Pobiera wartość jako liczbę całkowitą (Hz), tolerując warianty typów
    /// używane przez różne wersje FLRig (int/double/string).
    pub fn as_int(&self) -> Option<i64> {
        match self {
            XmlRpcValue::Int(v) => Some(*v),
            XmlRpcValue::Double(v) => Some(*v as i64),
            XmlRpcValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            XmlRpcValue::Str(s) => s.trim().parse::<i64>().ok(),
        }
    }

    pub fn as_str(&self) -> Option<String> {
        match self {
            XmlRpcValue::Str(s) => Some(s.clone()),
            XmlRpcValue::Int(v) => Some(v.to_string()),
            XmlRpcValue::Double(v) => Some(v.to_string()),
            XmlRpcValue::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            XmlRpcValue::Bool(b) => Some(*b),
            XmlRpcValue::Int(v) => Some(*v != 0),
            XmlRpcValue::Double(v) => Some(*v != 0.0),
            XmlRpcValue::Str(s) => match s.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "on" | "yes" => Some(true),
                "0" | "false" | "off" | "no" => Some(false),
                _ => None,
            },
        }
    }
}

/// Żądanie XML-RPC. Budowane przez metodę `new` i łańcuchowe dodawanie parametrów.
#[derive(Debug, Clone)]
pub struct XmlRpcRequest {
    method: String,
    params: Vec<XmlRpcValue>,
}

impl XmlRpcRequest {
    pub fn new(method: impl Into<String>) -> Self {
        Self { method: method.into(), params: Vec::new() }
    }

    pub fn param_int(mut self, v: i64) -> Self {
        self.params.push(XmlRpcValue::Int(v));
        self
    }

    pub fn param_double(mut self, v: f64) -> Self {
        self.params.push(XmlRpcValue::Double(v));
        self
    }

    pub fn param_bool(mut self, v: bool) -> Self {
        self.params.push(XmlRpcValue::Bool(v));
        self
    }

    pub fn param_str(mut self, v: impl Into<String>) -> Self {
        self.params.push(XmlRpcValue::Str(v.into()));
        self
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn params(&self) -> &[XmlRpcValue] {
        &self.params
    }

    /// Serializuje żądanie do dokumentu XML-RPC (bez nagłówka HTTP —
    /// FLRig używa czystego XML-RPC po TCP).
    pub fn to_xml(&self) -> String {
        let mut out = String::from("<?xml version=\"1.0\"?>\n<methodCall>");
        out.push_str(&format!("<methodName>{}</methodName>", escape_xml(&self.method)));
        if !self.params.is_empty() {
            out.push_str("<params>");
            for p in &self.params {
                out.push_str("<param>");
                out.push_str(&value_xml(p));
                out.push_str("</param>");
            }
            out.push_str("</params>");
        }
        out.push_str("</methodCall>");
        out
    }
}

/// Escapuje znaki specjalne XML w treści tekstowej.
pub fn escape_xml(s: &str) -> String {
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

fn value_xml(v: &XmlRpcValue) -> String {
    match v {
        XmlRpcValue::Int(i) => format!("<value><int>{}</int></value>", i),
        XmlRpcValue::Double(d) => {
            if d.fract() == 0.0 {
                format!("<value><double>{:.0}</double></value>", d)
            } else {
                format!("<value><double>{}</double></value>", d)
            }
        }
        XmlRpcValue::Bool(b) => format!("<value><boolean>{}</boolean></value>", if *b { 1 } else { 0 }),
        XmlRpcValue::Str(s) => format!("<value><string>{}</string></value>", escape_xml(s)),
    }
}

/// Błąd wynikający z parsowania odpowiedzi XML-RPC.
#[derive(Debug, PartialEq)]
pub struct XmlRpcFault {
    pub code: i64,
    pub message: String,
}

/// Parsuje dokument XML-RPC (methodResponse) i zwraca pierwszą wartość
/// lub błąd `<fault>`.
pub fn parse_response(xml: &str) -> Result<XmlRpcValue, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_fault = false;
    let mut fault_code: i64 = 0;
    let mut fault_msg = String::new();
    let mut value: Option<XmlRpcValue> = None;
    let mut value_depth = 0usize;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_string();
                match name.as_str() {
                    "fault" => in_fault = true,
                    "value" => {
                        // value otwierająca konkretną daną (nie nested w fault)
                        value_depth += 1;
                    }
                    "int" | "i4" | "i8" => {
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            if in_fault {
                                fault_code = txt.trim().parse().unwrap_or(0);
                            } else if let Ok(i) = txt.trim().parse::<i64>() {
                                value = Some(XmlRpcValue::Int(i));
                            }
                        }
                    }
                    "double" => {
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            if let Ok(d) = txt.trim().parse::<f64>() {
                                value = Some(XmlRpcValue::Double(d));
                            }
                        }
                    }
                    "boolean" => {
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            value = Some(XmlRpcValue::Bool(txt.trim() == "1" || txt.trim().eq_ignore_ascii_case("true")));
                        }
                    }
                    "string" => {
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            if in_fault {
                                fault_msg = txt.trim().to_string();
                            } else {
                                value = Some(XmlRpcValue::Str(txt.trim().to_string()));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_string();
                if name == "fault" {
                    in_fault = false;
                } else if name == "value" {
                    value_depth = value_depth.saturating_sub(1);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Błąd parsowania XML-RPC: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    if fault_code != 0 || !fault_msg.is_empty() {
        return Err(format!("Fault {}: {}", fault_code, fault_msg));
    }
    let _ = value_depth;
    value.ok_or_else(|| "Pusta odpowiedź XML-RPC (brak <value>)".to_string())
}

/// Odczytuje do końca bieżącego elementu i zwraca jego treść tekstową.
fn read_text(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<String, quick_xml::Error> {
    let mut txt = String::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(t)) => txt.push_str(&quick_xml::escape::unescape(t.as_ref())?),
            Ok(Event::CData(c)) => txt.push_str(c.as_ref()),
            Ok(Event::End(_)) | Ok(Event::Empty(_)) => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e),
            _ => {}
        }
        buf.clear();
    }
    Ok(txt)
}

/// Asynchroniczny klient FLRig. Łączy się po TCP i wymienia komunikaty XML-RPC.
pub struct FlrigClient {
    host: String,
    port: u16,
}

impl FlrigClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self { host: host.into(), port }
    }

    /// Wysyła żądanie i czyta odpowiedź do znacznika zamykającego `</methodResponse>`.
    async fn call(&self, req: &XmlRpcRequest) -> Result<XmlRpcValue, String> {
        let mut stream = TcpStream::connect(format!("{}:{}", self.host, self.port))
            .await
            .map_err(|e| format!("Błąd połączenia z FLRig: {}", e))?;

        stream
            .write_all(req.to_xml().as_bytes())
            .await
            .map_err(|e| format!("Błąd wysyłki do FLRig: {}", e))?;

        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = stream
                .read(&mut buf)
                .await
                .map_err(|e| format!("Błąd odczytu z FLRig: {}", e))?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&data);
            if text.contains("</methodResponse>") || text.contains("</methodCall>") {
                break;
            }
        }

        if data.is_empty() {
            return Err("FLRig nie zwrócił danych.".to_string());
        }
        parse_response(&String::from_utf8_lossy(&data))
    }

    /// Częstotliwość aktywnego VFO w Hz.
    pub async fn get_vfo(&self) -> Result<u64, String> {
        let req = XmlRpcRequest::new("rig.get_vfo").param_str("A");
        self.call(&req).await?.as_int().map(|v| v as u64).ok_or_else(|| "Nieprawidłowa częstotliwość od FLRig".to_string())
    }

    pub async fn set_vfo(&self, hz: u64) -> Result<(), String> {
        let req = XmlRpcRequest::new("rig.set_vfo").param_double(hz as f64);
        self.call(&req).await?;
        Ok(())
    }

    pub async fn get_mode(&self) -> Result<String, String> {
        let req = XmlRpcRequest::new("rig.get_mode");
        self.call(&req).await?.as_str().ok_or_else(|| "Nieprawidłowy tryb od FLRig".to_string())
    }

    pub async fn set_mode(&self, mode: &str) -> Result<(), String> {
        let req = XmlRpcRequest::new("rig.set_mode").param_str(mode);
        self.call(&req).await?;
        Ok(())
    }

    pub async fn get_ptt(&self) -> Result<bool, String> {
        let req = XmlRpcRequest::new("rig.get_ptt");
        self.call(&req).await?.as_bool().ok_or_else(|| "Nieprawidłowy stan PTT od FLRig".to_string())
    }

    pub async fn set_ptt(&self, ptt: bool) -> Result<(), String> {
        let req = XmlRpcRequest::new("rig.set_ptt").param_bool(ptt);
        self.call(&req).await?;
        Ok(())
    }

    pub async fn get_smeter(&self) -> Result<f64, String> {
        let req = XmlRpcRequest::new("rig.get_smeter");
        self.call(&req).await.and_then(|v| match v {
            XmlRpcValue::Double(d) => Ok(d),
            XmlRpcValue::Int(i) => Ok(i as f64),
            XmlRpcValue::Str(s) => s.trim().parse::<f64>().map_err(|_| "Nieprawidłowy S-meter".to_string()),
            XmlRpcValue::Bool(_) => Err("Nieprawidłowy S-meter".to_string()),
        })
    }
}

/// Implementacja wspólnego interfejsu [`crate::cat::backend::CatBackend`]
/// dla klienta FLRig (XML-RPC).
impl crate::cat::backend::CatBackend for FlrigClient {
    fn kind(&self) -> crate::cat::backend::CatBackendKind {
        crate::cat::backend::CatBackendKind::Flrig
    }

    async fn connect(&mut self) -> Result<(), String> {
        // Weryfikacja łączności przez odczyt aktywnego VFO.
        self.get_vfo()
            .await
            .map(|_| ())
            .map_err(|e| format!("Brak połączenia z FLRig: {}", e))
    }

    async fn poll_state(&mut self) -> Result<crate::cat::hamlib::RigState, String> {
        let freq = self.get_vfo().await?;
        let mode = self.get_mode().await?;
        let smeter = self.get_smeter().await.unwrap_or(-100.0) as f32;
        let ptt = self.get_ptt().await.unwrap_or(false);
        Ok(crate::cat::hamlib::RigState {
            frequency_hz: freq,
            mode,
            s_meter_dbm: smeter,
            s_meter_unit: crate::cat::hamlib::HamlibClient::raw_str_to_s_unit(smeter),
            ptt,
            connected: true,
            ..Default::default()
        })
    }

    async fn set_frequency(&self, freq_hz: u64) -> Result<(), String> {
        FlrigClient::set_vfo(self, freq_hz).await
    }

    async fn set_mode(&self, mode: &str, _passband_hz: u32) -> Result<(), String> {
        FlrigClient::set_mode(self, mode).await
    }

    async fn set_ptt(&self, ptt: bool) -> Result<(), String> {
        FlrigClient::set_ptt(self, ptt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_xml_handles_special_chars() {
        assert_eq!(escape_xml("a<b>&c\"d'e"), "a&lt;b&gt;&amp;c&quot;d&apos;e");
    }

    #[test]
    fn request_serializes_with_params() {
        let req = XmlRpcRequest::new("rig.set_vfo").param_double(14074000.0);
        let xml = req.to_xml();
        assert!(xml.contains("<methodName>rig.set_vfo</methodName>"));
        assert!(xml.contains("<double>14074000</double>"));
        assert!(xml.starts_with("<?xml"));
        assert!(xml.ends_with("</methodCall>"));
    }

    #[test]
    fn request_serializes_empty_params() {
        let xml = XmlRpcRequest::new("rig.get_mode").to_xml();
        assert!(!xml.contains("<params>"));
    }

    #[test]
    fn string_params_are_escaped() {
        let xml = XmlRpcRequest::new("rig.set_mode").param_str("USB<test>").to_xml();
        assert!(xml.contains("USB&lt;test&gt;"));
    }

    #[test]
    fn parse_int_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><int>14074000</int></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Int(14074000));
    }

    #[test]
    fn parse_double_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><double>14.074</double></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Double(14.074));
    }

    #[test]
    fn parse_string_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><string>USB</string></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Str("USB".to_string()));
    }

    #[test]
    fn parse_bool_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><boolean>1</boolean></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Bool(true));
    }

    #[test]
    fn parse_fault_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><fault><value><struct><member><name>faultCode</name><value><int>4</int></value></member><member><name>faultString</name><value><string>Too many parameters</string></value></member></struct></value></fault></methodResponse>";
        assert!(parse_response(xml).unwrap_err().contains("Too many parameters"));
    }

    #[test]
    fn as_int_tolerates_numeric_strings() {
        assert_eq!(XmlRpcValue::Str("14074000".to_string()).as_int(), Some(14074000));
        assert_eq!(XmlRpcValue::Double(7074000.0).as_int(), Some(7074000));
        assert_eq!(XmlRpcValue::Bool(true).as_int(), Some(1));
    }

    #[test]
    fn as_bool_tolerates_text_variants() {
        assert_eq!(XmlRpcValue::Str("ON".to_string()).as_bool(), Some(true));
        assert_eq!(XmlRpcValue::Int(0).as_bool(), Some(false));
    }
}
