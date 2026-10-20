// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Klient XML-RPC dla demona FLRig (http://www.w1hkj.com/flrig-help/)

use quick_xml::Reader;
use quick_xml::events::Event;
use std::fmt::Write as _;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::cat::backend::{BoxFuture, CatBackendKind, RigBackend};
use crate::cat::hamlib::HamlibClient;
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

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
            XmlRpcValue::Bool(b) => Some(i64::from(*b)),
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
        Self {
            method: method.into(),
            params: Vec::new(),
        }
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

    /// Serializuje żądanie do dokumentu XML-RPC, bez nagłówków HTTP.
    pub fn to_xml(&self) -> String {
        let mut out = String::from("<?xml version=\"1.0\"?>\n<methodCall>");
        let _ = write!(out, "<methodName>{}</methodName>", escape_xml(&self.method));
        if !self.params.is_empty() {
            out.push_str("<params>");
            for param in &self.params {
                out.push_str("<param>");
                out.push_str(&value_xml(param));
                out.push_str("</param>");
            }
            out.push_str("</params>");
        }
        out.push_str("</methodCall>");
        out
    }
}

/// Escapuje znaki specjalne XML w treści tekstowej.
pub use crate::core::xml::escape_xml;

fn value_xml(v: &XmlRpcValue) -> String {
    match v {
        XmlRpcValue::Int(i) => format!("<value><int>{i}</int></value>"),
        XmlRpcValue::Double(d) => {
            if d.fract() == 0.0 {
                format!("<value><double>{d:.0}</double></value>")
            } else {
                format!("<value><double>{d}</double></value>")
            }
        }
        XmlRpcValue::Bool(b) => format!("<value><boolean>{}</boolean></value>", i32::from(*b)),
        XmlRpcValue::Str(s) => format!("<value><string>{}</string></value>", escape_xml(s)),
    }
}

/// Błąd wynikający z parsowania odpowiedzi XML-RPC.
#[derive(Debug, PartialEq)]
pub struct XmlRpcFault {
    pub code: i64,
    pub message: String,
}

/// Błędy parsowania odpowiedzi XML-RPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlRpcParseError {
    Xml(String),
    Fault { code: i64, message: String },
    NoValue,
}

impl std::fmt::Display for XmlRpcParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(e) => write!(f, "XML parse error: {e}"),
            Self::Fault { code, message } => write!(f, "Fault {code}: {message}"),
            Self::NoValue => write!(f, "Empty XML-RPC response (missing <value>)"),
        }
    }
}

impl std::error::Error for XmlRpcParseError {}

/// Parsuje dokument XML-RPC (methodResponse) i zwraca pierwszą wartość
/// lub błąd `<fault>`. Obsługuje zarówno surowe XML, jak i odpowiedzi z nagłówkami HTTP.
pub fn parse_response(xml: &str) -> Result<XmlRpcValue, XmlRpcParseError> {
    let trimmed_input = xml.trim_start();
    let body = if trimmed_input.starts_with("HTTP/") {
        if let Some(idx) = trimmed_input.find("\r\n\r\n") {
            &trimmed_input[idx + 4..]
        } else if let Some(idx) = trimmed_input.find("\n\n") {
            &trimmed_input[idx + 2..]
        } else {
            trimmed_input
        }
    } else {
        trimmed_input
    };

    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);

    let mut in_fault = false;
    let mut saw_fault = false;
    let mut fault_code: i64 = 0;
    let mut fault_msg = String::new();
    let mut value: Option<XmlRpcValue> = None;
    let mut in_value = false;
    let mut current_type = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_string();
                match name.as_str() {
                    "fault" => {
                        in_fault = true;
                        saw_fault = true;
                    }
                    "value" => {
                        in_value = true;
                        current_type.clear();
                    }
                    "int" | "i4" | "i8" => {
                        current_type = name;
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            if in_fault {
                                fault_code = txt.trim().parse().unwrap_or(0);
                            } else if let Ok(i) = txt.trim().parse::<i64>() {
                                value = Some(XmlRpcValue::Int(i));
                            }
                        }
                    }
                    "double" => {
                        current_type = name;
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            if let Ok(d) = txt.trim().parse::<f64>() {
                                value = Some(XmlRpcValue::Double(d));
                            }
                        }
                    }
                    "boolean" => {
                        current_type = name;
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            value = Some(XmlRpcValue::Bool(
                                txt.trim() == "1" || txt.trim().eq_ignore_ascii_case("true"),
                            ));
                        }
                    }
                    "string" => {
                        current_type = name;
                        if let Ok(txt) = read_text(&mut reader, &mut buf) {
                            if in_fault {
                                fault_msg = txt.trim().to_string();
                            } else {
                                value = Some(XmlRpcValue::Str(txt.trim().to_string()));
                            }
                        }
                    }
                    _ => {
                        if in_value {
                            current_type = name;
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if in_value && current_type.is_empty() {
                    if let Ok(unescaped) = quick_xml::escape::unescape(t.as_ref()) {
                        let txt = unescaped.trim();
                        if in_fault {
                            if fault_msg.is_empty() && !txt.is_empty() {
                                fault_msg = txt.to_string();
                            }
                        } else if value.is_none() {
                            value = Some(XmlRpcValue::Str(txt.to_string()));
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_string();
                if name == "fault" {
                    in_fault = false;
                } else if name == "value" {
                    in_value = false;
                    current_type.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlRpcParseError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if saw_fault || fault_code != 0 || !fault_msg.is_empty() {
        return Err(XmlRpcParseError::Fault {
            code: fault_code,
            message: fault_msg,
        });
    }
    value.ok_or(XmlRpcParseError::NoValue)
}

/// Odczytuje do końca bieżącego elementu i zwraca jego treść tekstową.
fn read_text(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<String, quick_xml::Error> {
    let mut txt = String::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(t)) => txt.push_str(&quick_xml::escape::unescape(t.as_ref())?),
            Ok(Event::CData(c)) => txt.push_str(c.as_ref()),
            Ok(Event::End(_) | Event::Empty(_) | Event::Eof) => break,
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
    timeout: Duration,
}

impl FlrigClient {
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1500);

    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Self::DEFAULT_TIMEOUT,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(host: impl Into<String>, port: u16, timeout: Duration) -> Self {
        Self {
            host: host.into(),
            port,
            timeout,
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Wysyła żądanie HTTP/1.1 XML-RPC i czyta odpowiedź do znacznika zamykającego `</methodResponse>`.
    pub async fn call(&self, req: &XmlRpcRequest) -> Result<XmlRpcValue, RigError> {
        const MAX_RESPONSE_BYTES: usize = 64 * 1024;

        let endpoint = self.endpoint();
        let mut stream = tokio::time::timeout(self.timeout, TcpStream::connect(&endpoint))
            .await
            .map_err(|_| RigError::Timeout {
                endpoint: endpoint.clone(),
                operation: "connect",
            })?
            .map_err(|e| RigError::Connect {
                endpoint: endpoint.clone(),
                detail: e.to_string(),
            })?;

        let xml = req.to_xml();
        let http_req = format!(
            "POST /RPC2 HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: SPLogbook\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.host,
            self.port,
            xml.len(),
            xml
        );

        tokio::time::timeout(self.timeout, stream.write_all(http_req.as_bytes()))
            .await
            .map_err(|_| RigError::Timeout {
                endpoint: endpoint.clone(),
                operation: "write",
            })?
            .map_err(|e| RigError::Connect {
                endpoint: endpoint.clone(),
                detail: e.to_string(),
            })?;

        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let bytes_read = tokio::time::timeout(self.timeout, stream.read(&mut buf))
                .await
                .map_err(|_| RigError::Timeout {
                    endpoint: endpoint.clone(),
                    operation: "read",
                })?
                .map_err(|e| RigError::Connect {
                    endpoint: endpoint.clone(),
                    detail: e.to_string(),
                })?;
            if bytes_read == 0 {
                break;
            }
            data.extend_from_slice(&buf[..bytes_read]);
            if data.len() > MAX_RESPONSE_BYTES {
                return Err(RigError::Protocol {
                    detail: "response exceeded maximum length of 64 KB".to_string(),
                });
            }
            let text = String::from_utf8_lossy(&data);
            if text.contains("</methodResponse>") || text.contains("</methodCall>") {
                break;
            }
        }

        if data.is_empty() {
            return Err(RigError::Protocol {
                detail: "empty response from FLRig".to_string(),
            });
        }
        parse_response(&String::from_utf8_lossy(&data))
            .map_err(|detail| RigError::Protocol {
                detail: detail.to_string(),
            })
    }

    pub async fn get_vfo_typed(&self) -> Result<u64, RigError> {
        let req = XmlRpcRequest::new("rig.get_vfo");
        self.call(&req)
            .await?
            .as_int()
            .map(|v| v as u64)
            .ok_or_else(|| RigError::Protocol {
                detail: "invalid frequency value in FLRig response".to_string(),
            })
    }

    pub async fn set_vfo_typed(&self, hz: u64) -> Result<(), RigError> {
        let req = XmlRpcRequest::new("rig.set_vfo").param_double(hz as f64);
        self.call(&req).await?;
        Ok(())
    }

    pub async fn get_mode_typed(&self) -> Result<String, RigError> {
        let req = XmlRpcRequest::new("rig.get_mode");
        self.call(&req)
            .await?
            .as_str()
            .ok_or_else(|| RigError::Protocol {
                detail: "invalid mode value in FLRig response".to_string(),
            })
    }

    pub async fn set_mode_typed(&self, mode: &str) -> Result<(), RigError> {
        let req = XmlRpcRequest::new("rig.set_mode").param_str(mode);
        self.call(&req).await?;
        Ok(())
    }

    pub async fn get_ptt_typed(&self) -> Result<bool, RigError> {
        let req = XmlRpcRequest::new("rig.get_ptt");
        self.call(&req)
            .await?
            .as_bool()
            .ok_or_else(|| RigError::Protocol {
                detail: "invalid PTT value in FLRig response".to_string(),
            })
    }

    pub async fn set_ptt_typed(&self, ptt: bool) -> Result<(), RigError> {
        let req = XmlRpcRequest::new("rig.set_ptt").param_bool(ptt);
        self.call(&req).await?;
        Ok(())
    }

    pub async fn get_smeter_typed(&self) -> Result<f32, RigError> {
        let req = XmlRpcRequest::new("rig.get_smeter");
        let val = self.call(&req).await?;
        match val {
            XmlRpcValue::Double(d) => Ok(d as f32),
            XmlRpcValue::Int(i) => Ok(i as f32),
            XmlRpcValue::Str(s) => s.trim().parse::<f32>().map_err(|_| RigError::Protocol {
                detail: "invalid S-meter numeric string in FLRig response".to_string(),
            }),
            XmlRpcValue::Bool(_) => Err(RigError::Protocol {
                detail: "unexpected boolean value for S-meter in FLRig response".to_string(),
            }),
        }
    }

    /// Częstotliwość aktywnego VFO w Hz.
    pub async fn get_vfo(&self) -> Result<u64, RigError> {
        self.get_vfo_typed().await
    }

    /// Alias dla `get_vfo`.
    pub async fn get_frequency(&self) -> Result<u64, RigError> {
        self.get_vfo().await
    }

    pub async fn set_vfo(&self, hz: u64) -> Result<(), RigError> {
        self.set_vfo_typed(hz).await
    }

    pub async fn get_mode(&self) -> Result<String, RigError> {
        self.get_mode_typed().await
    }

    pub async fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        self.set_mode_typed(mode).await
    }

    pub async fn get_ptt(&self) -> Result<bool, RigError> {
        self.get_ptt_typed().await
    }

    pub async fn set_ptt(&self, ptt: bool) -> Result<(), RigError> {
        self.set_ptt_typed(ptt).await
    }

    pub async fn get_smeter(&self) -> Result<f64, RigError> {
        self.get_smeter_typed().await.map(|v| v as f64)
    }
}

/// Implementacja wspólnego interfejsu [`crate::cat::backend::CatBackend`]
/// dla klienta FLRig (XML-RPC).
impl crate::cat::backend::CatBackend for FlrigClient {
    fn kind(&self) -> crate::cat::backend::CatBackendKind {
        crate::cat::backend::CatBackendKind::Flrig
    }

    async fn connect(&mut self) -> Result<(), RigError> {
        // Weryfikacja łączności przez odczyt aktywnego VFO.
        self.get_vfo().await.map(|_| ())
    }

    async fn poll_state(&mut self) -> Result<crate::cat::hamlib::RigState, RigError> {
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

    async fn set_frequency(&self, freq_hz: u64) -> Result<(), RigError> {
        FlrigClient::set_vfo(self, freq_hz).await
    }

    async fn set_mode(&self, mode: &str, _passband_hz: u32) -> Result<(), RigError> {
        FlrigClient::set_mode(self, mode).await
    }

    async fn set_ptt(&self, ptt: bool) -> Result<(), RigError> {
        FlrigClient::set_ptt(self, ptt).await
    }
}

/// Backend protokołu FLRig oparty o XML-RPC (HTTP).
pub struct FlrigBackend {
    client: FlrigClient,
    connected: bool,
}

impl FlrigBackend {
    /// Tworzy nowy backend FLRig dla wskazanego hosta i portu.
    ///
    /// Konstruktor nie wykonuje operacji wejścia/wyjścia. Połączenie weryfikowane
    /// jest asynchronicznie przez [`RigBackend::connect`].
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            client: FlrigClient::new(host, port),
            connected: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(host: impl Into<String>, port: u16, timeout: Duration) -> Self {
        Self {
            client: FlrigClient::with_timeout(host, port, timeout),
            connected: false,
        }
    }

    /// Sprawdza, czy backend jest logicznie połączony.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    async fn connect_internal(&mut self) -> Result<(), RigError> {
        self.connected = false;
        // Weryfikacja łączności przez odczyt aktywnego VFO (lekkie zapytanie probe).
        self.client.get_vfo_typed().await?;
        self.connected = true;
        Ok(())
    }

    fn disconnect_internal(&mut self) {
        self.connected = false;
    }

    async fn poll_internal(&mut self) -> Result<RigState, RigError> {
        if !self.connected {
            return Err(RigError::NotConnected);
        }

        let freq = self.client.get_vfo_typed().await?;
        let mode = self.client.get_mode_typed().await?;
        let smeter = self.client.get_smeter_typed().await?;
        let ptt = self.client.get_ptt_typed().await?;

        Ok(RigState {
            frequency_hz: freq,
            mode,
            s_meter_dbm: smeter,
            s_meter_unit: HamlibClient::raw_str_to_s_unit(smeter),
            ptt,
            connected: true,
            ..Default::default()
        })
    }

    async fn execute_cmd(&mut self, command: &RigCommand) -> Result<(), RigError> {
        if !self.connected {
            return Err(RigError::NotConnected);
        }

        match command {
            RigCommand::SetFrequency(hz) => self.client.set_vfo_typed(*hz).await,
            RigCommand::SetMode { mode, .. } => self.client.set_mode_typed(mode).await,
            RigCommand::SetPtt(ptt) => self.client.set_ptt_typed(*ptt).await,
            RigCommand::SetSplitFrequency(_) => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetSplitFrequency",
            }),
            RigCommand::SetPassband(_) => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetPassband",
            }),
            RigCommand::SetSplit { .. } => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetSplit",
            }),
            RigCommand::SetVfo(_) => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetVfo",
            }),
            RigCommand::SetRit(_) => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetRit",
            }),
            RigCommand::SetXit(_) => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetXit",
            }),
            RigCommand::SetPower(_) => Err(RigError::Unsupported {
                backend: CatBackendKind::Flrig,
                command: "SetPower",
            }),
        }
    }
}

impl RigBackend for FlrigBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Flrig
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities {
            frequency: true,
            mode: true,
            passband: false,
            split: false,
            split_frequency: false,
            ptt: true,
            vfo: false,
            rit: false,
            xit: false,
            power: false,
            s_meter: true,
        }
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        Box::pin(self.connect_internal())
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        Box::pin(self.poll_internal())
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        Box::pin(self.execute_cmd(command))
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        self.disconnect_internal();
        Box::pin(std::future::ready(()))
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
        let req = XmlRpcRequest::new("rig.set_vfo").param_double(14_074_000.0);
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
        let xml = XmlRpcRequest::new("rig.set_mode")
            .param_str("USB<test>")
            .to_xml();
        assert!(xml.contains("USB&lt;test&gt;"));
    }

    #[test]
    fn parse_int_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><int>14074000</int></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Int(14_074_000));
    }

    #[test]
    fn parse_double_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><double>14.074</double></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Double(14.074));
    }

    #[test]
    fn parse_string_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><string>USB</string></value></param></params></methodResponse>";
        assert_eq!(
            parse_response(xml).unwrap(),
            XmlRpcValue::Str("USB".to_string())
        );
    }

    #[test]
    fn parse_bool_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><params><param><value><boolean>1</boolean></value></param></params></methodResponse>";
        assert_eq!(parse_response(xml).unwrap(), XmlRpcValue::Bool(true));
    }

    #[test]
    fn parse_fault_response() {
        let xml = "<?xml version=\"1.0\"?><methodResponse><fault><value><struct><member><name>faultCode</name><value><int>4</int></value></member><member><name>faultString</name><value><string>Too many parameters</string></value></member></struct></value></fault></methodResponse>";
        assert!(
            parse_response(xml)
                .unwrap_err()
                .to_string()
                .contains("Too many parameters")
        );
    }

    #[test]
    fn as_int_tolerates_numeric_strings() {
        assert_eq!(
            XmlRpcValue::Str("14074000".to_string()).as_int(),
            Some(14_074_000)
        );
        assert_eq!(XmlRpcValue::Double(7_074_000.0).as_int(), Some(7_074_000));
        assert_eq!(XmlRpcValue::Bool(true).as_int(), Some(1));
    }

    #[test]
    fn as_bool_tolerates_text_variants() {
        assert_eq!(XmlRpcValue::Str("ON".to_string()).as_bool(), Some(true));
        assert_eq!(XmlRpcValue::Int(0).as_bool(), Some(false));
    }

    // ==========================================
    // Atrapa HTTP/XML-RPC dla testów FlrigBackend
    // ==========================================

    async fn run_mock_flrig_server<F, Fut>(handler: F) -> (u16, tokio::task::JoinHandle<()>)
    where
        F: FnOnce(tokio::net::TcpListener) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock listener");
        let port = listener.local_addr().expect("local addr").port();
        let handle = tokio::spawn(handler(listener));
        (port, handle)
    }

    fn xmlrpc_response(xml_body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            xml_body.len(),
            xml_body
        )
    }

    fn ok_int_response(val: i64) -> String {
        xmlrpc_response(&format!(
            "<?xml version=\"1.0\"?><methodResponse><params><param><value><int>{val}</int></value></param></params></methodResponse>"
        ))
    }

    fn ok_double_response(val: f64) -> String {
        xmlrpc_response(&format!(
            "<?xml version=\"1.0\"?><methodResponse><params><param><value><double>{val}</double></value></param></params></methodResponse>"
        ))
    }

    fn ok_str_response(val: &str) -> String {
        xmlrpc_response(&format!(
            "<?xml version=\"1.0\"?><methodResponse><params><param><value><string>{val}</string></value></param></params></methodResponse>"
        ))
    }

    fn ok_bool_response(val: bool) -> String {
        xmlrpc_response(&format!(
            "<?xml version=\"1.0\"?><methodResponse><params><param><value><boolean>{}</boolean></value></param></params></methodResponse>",
            if val { 1 } else { 0 }
        ))
    }

    fn fault_response(code: i32, msg: &str) -> String {
        xmlrpc_response(&format!(
            "<?xml version=\"1.0\"?><methodResponse><fault><value><struct><member><name>faultCode</name><value><int>{code}</int></value></member><member><name>faultString</name><value><string>{msg}</string></value></member></struct></value></fault></methodResponse>"
        ))
    }

    async fn read_xmlrpc_request(stream: &mut TcpStream) -> (String, String) {
        let mut buf = Vec::new();
        let mut temp = [0u8; 1024];
        let mut content_len: Option<usize> = None;
        let mut body_start: Option<usize> = None;

        loop {
            let n = tokio::time::timeout(Duration::from_millis(1500), stream.read(&mut temp))
                .await
                .expect("timeout reading request from stream")
                .expect("read from stream");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&temp[..n]);

            if body_start.is_none() {
                if let Some(idx) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    body_start = Some(idx + 4);
                    let headers_str = String::from_utf8_lossy(&buf[..idx]);
                    for line in headers_str.lines() {
                        let trimmed = line.trim();
                        if let Some(val) = trimmed.strip_prefix("Content-Length:") {
                            content_len = val.trim().parse::<usize>().ok();
                        } else if let Some(val) = trimmed.strip_prefix("content-length:") {
                            content_len = val.trim().parse::<usize>().ok();
                        }
                    }
                }
            }

            if let (Some(start), Some(len)) = (body_start, content_len) {
                if buf.len() >= start + len {
                    break;
                }
            }
        }

        let body = if let Some(start) = body_start {
            String::from_utf8_lossy(&buf[start..]).to_string()
        } else {
            String::from_utf8_lossy(&buf).to_string()
        };

        let method = if let Some(start) = body.find("<methodName>") {
            if let Some(end) = body[start..].find("</methodName>") {
                body[start + 12..start + end].trim().to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        (method, body)
    }

    // ==========================================
    // Testy FlrigBackend (CAT-3)
    // ==========================================

    #[tokio::test]
    async fn flrig_poll_reads_vfo_mode_smeter_ptt() {
        let (port, server_task) = run_mock_flrig_server(|listener| async move {
            // 1. connect() probe: rig.get_vfo
            let (mut stream, _) = listener.accept().await.expect("accept 1");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_vfo");
            stream
                .write_all(ok_int_response(14_074_000).as_bytes())
                .await
                .unwrap();

            // 2. poll(): rig.get_vfo
            let (mut stream, _) = listener.accept().await.expect("accept 2");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_vfo");
            stream
                .write_all(ok_int_response(14_074_000).as_bytes())
                .await
                .unwrap();

            // 3. poll(): rig.get_mode
            let (mut stream, _) = listener.accept().await.expect("accept 3");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_mode");
            stream
                .write_all(ok_str_response("USB").as_bytes())
                .await
                .unwrap();

            // 4. poll(): rig.get_smeter
            let (mut stream, _) = listener.accept().await.expect("accept 4");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_smeter");
            stream
                .write_all(ok_double_response(10.0).as_bytes())
                .await
                .unwrap();

            // 5. poll(): rig.get_ptt
            let (mut stream, _) = listener.accept().await.expect("accept 5");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_ptt");
            stream
                .write_all(ok_bool_response(true).as_bytes())
                .await
                .unwrap();
        })
        .await;

        let mut backend = FlrigBackend::new("127.0.0.1", port);
        assert!(!backend.is_connected());
        backend.connect().await.unwrap();
        assert!(backend.is_connected());

        let state = backend.poll().await.unwrap();
        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "USB");
        assert_eq!(state.s_meter_dbm, 10.0);
        assert_eq!(state.s_meter_unit, "S9+10dB");
        assert!(state.ptt);
        assert!(state.connected);
        assert_eq!(state.passband_hz, 500);
        assert!(!state.split_enabled);

        backend.disconnect().await;
        assert!(!backend.is_connected());
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn flrig_execute_supported_commands() {
        let (port, server_task) = run_mock_flrig_server(|listener| async move {
            // 1. connect() probe
            let (mut stream, _) = listener.accept().await.expect("accept connect");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_vfo");
            stream
                .write_all(ok_int_response(14_074_000).as_bytes())
                .await
                .unwrap();

            // 2. SetFrequency
            let (mut stream, _) = listener.accept().await.expect("accept set_vfo");
            let (method, body) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.set_vfo");
            assert!(body.contains("7074000"));
            stream
                .write_all(ok_double_response(7_074_000.0).as_bytes())
                .await
                .unwrap();

            // 3. SetMode
            let (mut stream, _) = listener.accept().await.expect("accept set_mode");
            let (method, body) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.set_mode");
            assert!(body.contains("CW"));
            stream
                .write_all(ok_str_response("CW").as_bytes())
                .await
                .unwrap();

            // 4. SetPtt(true)
            let (mut stream, _) = listener.accept().await.expect("accept set_ptt true");
            let (method, body) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.set_ptt");
            assert!(body.contains("<boolean>1</boolean>"));
            stream
                .write_all(ok_bool_response(true).as_bytes())
                .await
                .unwrap();

            // 5. SetPtt(false)
            let (mut stream, _) = listener.accept().await.expect("accept set_ptt false");
            let (method, body) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.set_ptt");
            assert!(body.contains("<boolean>0</boolean>"));
            stream
                .write_all(ok_bool_response(false).as_bytes())
                .await
                .unwrap();
        })
        .await;

        let mut backend = FlrigBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();

        backend
            .execute(&RigCommand::SetFrequency(7_074_000))
            .await
            .unwrap();
        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();
        backend.execute(&RigCommand::SetPtt(false)).await.unwrap();

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn flrig_unsupported_commands() {
        let (port, server_task) = run_mock_flrig_server(|listener| async move {
            let (mut stream, _) = listener.accept().await.expect("accept connect");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_vfo");
            stream
                .write_all(ok_int_response(14_074_000).as_bytes())
                .await
                .unwrap();
        })
        .await;

        let mut backend = FlrigBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();

        let unsupported = [
            (
                RigCommand::SetSplitFrequency(14_075_000),
                "SetSplitFrequency",
            ),
            (RigCommand::SetPassband(2400), "SetPassband"),
            (
                RigCommand::SetSplit {
                    enabled: true,
                    tx_vfo: "VFOB".to_string(),
                },
                "SetSplit",
            ),
            (RigCommand::SetVfo("VFOB".to_string()), "SetVfo"),
            (RigCommand::SetRit(100), "SetRit"),
            (RigCommand::SetXit(-50), "SetXit"),
            (RigCommand::SetPower(50.0), "SetPower"),
        ];

        for (cmd, name) in unsupported {
            let err = backend.execute(&cmd).await.unwrap_err();
            assert_eq!(
                err,
                RigError::Unsupported {
                    backend: CatBackendKind::Flrig,
                    command: name,
                }
            );
        }

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn flrig_fault_response_is_protocol_error() {
        let (port, server_task) = run_mock_flrig_server(|listener| async move {
            let (mut stream, _) = listener.accept().await.expect("accept connect");
            let (method, _) = read_xmlrpc_request(&mut stream).await;
            assert_eq!(method, "rig.get_vfo");
            stream
                .write_all(fault_response(4, "Too many parameters").as_bytes())
                .await
                .unwrap();
        })
        .await;

        let mut backend = FlrigBackend::new("127.0.0.1", port);
        let err = backend.connect().await.unwrap_err();
        assert!(!backend.is_connected());
        match err {
            RigError::Protocol { detail } => {
                assert!(detail.contains("Too many parameters"));
            }
            other => panic!("expected RigError::Protocol, got: {other:?}"),
        }

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn flrig_connect_failure_is_typed_error() {
        let mut backend = FlrigBackend::with_timeout("127.0.0.1", 1, Duration::from_millis(50));
        let err = backend.connect().await.unwrap_err();
        assert!(!backend.is_connected());
        assert!(matches!(
            err,
            RigError::Connect { .. } | RigError::Timeout { .. }
        ));
    }

    #[tokio::test]
    async fn flrig_poll_without_connect_is_not_connected() {
        let mut backend = FlrigBackend::new("127.0.0.1", 1234);
        let err = backend.poll().await.unwrap_err();
        assert_eq!(err, RigError::NotConnected);
    }

    #[tokio::test]
    async fn flrig_execute_without_connect_is_not_connected() {
        let mut backend = FlrigBackend::new("127.0.0.1", 1234);
        let err = backend
            .execute(&RigCommand::SetFrequency(14_074_000))
            .await
            .unwrap_err();
        assert_eq!(err, RigError::NotConnected);
    }

    #[tokio::test]
    async fn flrig_disconnect_is_idempotent() {
        let mut backend = FlrigBackend::new("127.0.0.1", 1234);
        backend.disconnect().await;
        backend.disconnect().await;
        backend.disconnect().await;
        assert!(!backend.is_connected());
    }

    #[test]
    fn flrig_kind_and_capabilities() {
        let backend = FlrigBackend::new("127.0.0.1", 1234);
        assert_eq!(backend.kind(), CatBackendKind::Flrig);

        let caps = backend.capabilities();
        let expected = RigCapabilities {
            frequency: true,
            mode: true,
            passband: false,
            split: false,
            split_frequency: false,
            ptt: true,
            vfo: false,
            rit: false,
            xit: false,
            power: false,
            s_meter: true,
        };
        assert_eq!(caps, expected);

        assert!(caps.supports(&RigCommand::SetFrequency(14_000_000)));
        assert!(caps.supports(&RigCommand::SetMode {
            mode: "USB".to_string(),
            passband_hz: 2400,
        }));
        assert!(caps.supports(&RigCommand::SetPtt(true)));

        assert!(!caps.supports(&RigCommand::SetSplitFrequency(14_000_000)));
        assert!(!caps.supports(&RigCommand::SetPassband(2400)));
        assert!(!caps.supports(&RigCommand::SetSplit {
            enabled: true,
            tx_vfo: "VFOB".to_string(),
        }));
        assert!(!caps.supports(&RigCommand::SetVfo("VFOB".to_string())));
        assert!(!caps.supports(&RigCommand::SetRit(100)));
        assert!(!caps.supports(&RigCommand::SetXit(100)));
        assert!(!caps.supports(&RigCommand::SetPower(100.0)));
    }

    #[tokio::test]
    async fn flrig_call_timeout_is_typed_timeout() {
        let (port, server_task) = run_mock_flrig_server(|listener| async move {
            let (_stream, _) = listener.accept().await.expect("accept connect");
            // Do not reply, hold connection until client times out
            tokio::time::sleep(Duration::from_millis(150)).await;
        })
        .await;

        let mut backend = FlrigBackend::with_timeout("127.0.0.1", port, Duration::from_millis(50));
        let err = backend.connect().await.unwrap_err();
        assert!(!backend.is_connected());
        assert!(matches!(err, RigError::Timeout { .. }));

        server_task.await.unwrap();
    }
}
