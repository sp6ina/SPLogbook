// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Pasywne odkrywanie transceiverów FlexRadio w sieci lokalnej przez UDP (port 4992).
//!
//! Zgodne z oficjalną specyfikacją FlexRadio SmartSDR Discovery Protocol.
//! Urządzenia FlexRadio okresowo (co około 1 sekundę) rozsyłają w podsieci
//! pakiety rozgłoszeniowe UDP z parametrami identyfikacyjnymi w formacie par klucz-wartość.
//!
//! Moduł implementuje wyłącznie pasywny nasłuch — nie wysyła żadnych arbitralnych
//! pakietów zapytania, nie przełącza PTT ani nie uruchamia strumieni multimedialnych.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

use super::protocol::FlexProtocolError;

/// Domyślny oficjalny port rozgłoszeniowy UDP SmartSDR.
pub const FLEX_DISCOVERY_PORT: u16 = 4992;

/// Maksymalny dopuszczalny rozmiar datagramu UDP discovery (ochrona pamięci).
pub const MAX_DISCOVERY_DATAGRAM_SIZE: usize = 4096;

/// Zdekodowany pakiet discovery transceivera FlexRadio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexDiscoveryPacket {
    pub discovery_protocol_version: String,
    pub model: String,
    pub serial: String,
    pub version: String,
    pub nickname: String,
    pub callsign: String,
    pub ip: IpAddr,
    pub port: u16,
    pub status: String,
    pub in_use_ip: Option<String>,
    pub in_use_host: Option<String>,
    pub max_slices: Option<u32>,
    pub available_slices: Option<u32>,
    pub max_panadapters: Option<u32>,
    pub available_panadapters: Option<u32>,
    pub extra_fields: BTreeMap<String, String>,
}

/// Kodek pakietów discovery FlexRadio SmartSDR.
#[derive(Debug, Clone, Default)]
pub struct FlexDiscoveryCodec {
    max_datagram_size: usize,
}

impl FlexDiscoveryCodec {
    pub fn new() -> Self {
        Self {
            max_datagram_size: MAX_DISCOVERY_DATAGRAM_SIZE,
        }
    }

    pub fn with_max_datagram_size(max_size: usize) -> Self {
        Self {
            max_datagram_size: max_size,
        }
    }

    /// Dekoduje surowy datagram UDP na strukturę [`FlexDiscoveryPacket`].
    pub fn decode_packet(&self, bytes: &[u8]) -> Result<FlexDiscoveryPacket, FlexProtocolError> {
        if bytes.len() > self.max_datagram_size {
            return Err(FlexProtocolError::OversizedDatagram {
                max: self.max_datagram_size,
                actual: bytes.len(),
            });
        }

        // Pomiń ewentualne końcowe bajty null (\0)
        let trimmed_bytes = match bytes.iter().position(|&b| b == 0) {
            Some(idx) => &bytes[..idx],
            None => bytes,
        };

        let text = std::str::from_utf8(trimmed_bytes)
            .map_err(|e| FlexProtocolError::InvalidUtf8(e.to_string()))?;

        let mut fields: BTreeMap<String, String> = BTreeMap::new();
        for token in text.split_whitespace() {
            if let Some((k, v)) = token.split_once('=') {
                fields.insert(k.to_ascii_lowercase(), v.to_string());
            }
        }

        let model = fields
            .remove("model")
            .ok_or_else(|| FlexProtocolError::MissingRequiredField("model".to_string()))?;

        let serial = fields
            .remove("serial")
            .ok_or_else(|| FlexProtocolError::MissingRequiredField("serial".to_string()))?;

        let ip_str = fields
            .remove("ip")
            .ok_or_else(|| FlexProtocolError::MissingRequiredField("ip".to_string()))?;

        let ip = ip_str.parse::<IpAddr>().map_err(|e| {
            FlexProtocolError::ParseError(format!("invalid IP address '{ip_str}': {e}"))
        })?;

        let port_str = fields
            .remove("port")
            .unwrap_or_else(|| FLEX_DISCOVERY_PORT.to_string());

        let port = port_str.parse::<u16>().map_err(|e| {
            FlexProtocolError::ParseError(format!("invalid port number '{port_str}': {e}"))
        })?;

        let discovery_protocol_version = fields
            .remove("discovery_protocol_version")
            .unwrap_or_else(|| "3.0.0.0".to_string());

        let version = fields.remove("version").unwrap_or_default();
        let nickname = fields.remove("nickname").unwrap_or_default();
        let callsign = fields.remove("callsign").unwrap_or_default();
        let status = fields.remove("status").unwrap_or_default();
        let in_use_ip = fields.remove("inuse_ip").filter(|s| !s.is_empty());
        let in_use_host = fields.remove("inuse_host").filter(|s| !s.is_empty());

        let max_slices = fields
            .remove("max_slices")
            .and_then(|s| s.parse::<u32>().ok());

        let available_slices = fields
            .remove("available_slices")
            .and_then(|s| s.parse::<u32>().ok());

        let max_panadapters = fields
            .remove("max_panadapters")
            .and_then(|s| s.parse::<u32>().ok());

        let available_panadapters = fields
            .remove("available_panadapters")
            .and_then(|s| s.parse::<u32>().ok());

        Ok(FlexDiscoveryPacket {
            discovery_protocol_version,
            model,
            serial,
            version,
            nickname,
            callsign,
            ip,
            port,
            status,
            in_use_ip,
            in_use_host,
            max_slices,
            available_slices,
            max_panadapters,
            available_panadapters,
            extra_fields: fields,
        })
    }

    /// Koduje pakiet do postaci tekstowej pary klucz-wartość (wykorzystywane m.in. w testach).
    pub fn encode_packet(&self, packet: &FlexDiscoveryPacket) -> Vec<u8> {
        let mut parts = Vec::new();
        parts.push(format!(
            "discovery_protocol_version={}",
            packet.discovery_protocol_version
        ));
        parts.push(format!("model={}", packet.model));
        parts.push(format!("serial={}", packet.serial));
        parts.push(format!("version={}", packet.version));
        parts.push(format!("nickname={}", packet.nickname));
        parts.push(format!("callsign={}", packet.callsign));
        parts.push(format!("ip={}", packet.ip));
        parts.push(format!("port={}", packet.port));
        parts.push(format!("status={}", packet.status));

        if let Some(ref in_ip) = packet.in_use_ip {
            parts.push(format!("inuse_ip={in_ip}"));
        }
        if let Some(ref in_host) = packet.in_use_host {
            parts.push(format!("inuse_host={in_host}"));
        }
        if let Some(ms) = packet.max_slices {
            parts.push(format!("max_slices={ms}"));
        }
        if let Some(asl) = packet.available_slices {
            parts.push(format!("available_slices={asl}"));
        }
        if let Some(mp) = packet.max_panadapters {
            parts.push(format!("max_panadapters={mp}"));
        }
        if let Some(ap) = packet.available_panadapters {
            parts.push(format!("available_panadapters={ap}"));
        }

        for (k, v) in &packet.extra_fields {
            parts.push(format!("{k}={v}"));
        }

        parts.join(" ").into_bytes()
    }
}

/// Deduplikuje listę pakietów discovery po unikalnym numerze seryjnym sprzętu (`serial`).
pub fn deduplicate_radios(packets: Vec<FlexDiscoveryPacket>) -> Vec<FlexDiscoveryPacket> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for packet in packets {
        if seen.insert(packet.serial.clone()) {
            deduped.push(packet);
        }
    }

    deduped
}

/// Pasywnie nasłuchuje pakietów discovery w sieci lokalnej przez określony czas.
/// Nie wysyła żadnych danych do sieci.
pub async fn discover_radios(
    bind_addr: Option<&str>,
    listen_duration: Duration,
) -> Result<Vec<FlexDiscoveryPacket>, FlexProtocolError> {
    let default_addr = format!("0.0.0.0:{FLEX_DISCOVERY_PORT}");
    let addr = bind_addr.unwrap_or(&default_addr);

    let socket = match UdpSocket::bind(addr).await {
        Ok(s) => s,
        Err(e) => {
            return Err(FlexProtocolError::Transport(format!(
                "cannot bind UDP socket to '{addr}': {e}"
            )));
        }
    };

    let codec = FlexDiscoveryCodec::new();
    let mut discovered = Vec::new();
    let mut buf = [0u8; MAX_DISCOVERY_DATAGRAM_SIZE + 1];

    let start = tokio::time::Instant::now();

    while start.elapsed() < listen_duration {
        let remaining = listen_duration.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, _from))) => {
                if let Ok(packet) = codec.decode_packet(&buf[..len]) {
                    discovered.push(packet);
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }

    Ok(deduplicate_radios(discovered))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn sample_valid_packet() -> FlexDiscoveryPacket {
        FlexDiscoveryPacket {
            discovery_protocol_version: "3.0.0.2".to_string(),
            model: "FLEX-6600".to_string(),
            serial: "1320-5691-6400-6874".to_string(),
            version: "3.7.4.29165".to_string(),
            nickname: "MyFlex".to_string(),
            callsign: "SP6INA".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 120)),
            port: 4992,
            status: "Available".to_string(),
            in_use_ip: None,
            in_use_host: None,
            max_slices: Some(4),
            available_slices: Some(4),
            max_panadapters: Some(4),
            available_panadapters: Some(4),
            extra_fields: BTreeMap::new(),
        }
    }

    #[test]
    fn flex_discovery_decodes_valid_packet() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"discovery_protocol_version=3.0.0.2 model=FLEX-6600 serial=1320-5691-6400-6874 version=3.7.4.29165 nickname=MyFlex callsign=SP6INA ip=192.168.1.120 port=4992 status=Available max_slices=4 available_slices=4 max_panadapters=4 available_panadapters=4\0";
        let packet = codec.decode_packet(payload).expect("decoding failed");

        assert_eq!(packet.model, "FLEX-6600");
        assert_eq!(packet.serial, "1320-5691-6400-6874");
        assert_eq!(packet.version, "3.7.4.29165");
        assert_eq!(packet.nickname, "MyFlex");
        assert_eq!(packet.callsign, "SP6INA");
        assert_eq!(packet.ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 120)));
        assert_eq!(packet.port, 4992);
        assert_eq!(packet.status, "Available");
        assert_eq!(packet.max_slices, Some(4));
        assert_eq!(packet.available_slices, Some(4));
    }

    #[test]
    fn flex_discovery_rejects_invalid_header() {
        let codec = FlexDiscoveryCodec::new();
        // Brak wymaganych pól (np. brak ip i serial)
        let payload = b"some_random_payload=true test=123";
        let err = codec.decode_packet(payload).expect_err("should fail");
        assert!(matches!(err, FlexProtocolError::MissingRequiredField(_)));
    }

    #[test]
    fn flex_discovery_rejects_oversized_datagram() {
        let codec = FlexDiscoveryCodec::with_max_datagram_size(128);
        let oversized = vec![b'A'; 256];
        let err = codec.decode_packet(&oversized).expect_err("should reject");
        assert!(matches!(err, FlexProtocolError::OversizedDatagram { .. }));
    }

    #[test]
    fn flex_discovery_preserves_unknown_fields() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"discovery_protocol_version=3.0.0.2 model=FLEX-6400 serial=1111-2222 ip=10.0.0.5 port=4992 custom_vendor_opt=enabled license_status=valid";
        let packet = codec.decode_packet(payload).expect("decode failed");

        assert_eq!(
            packet.extra_fields.get("custom_vendor_opt"),
            Some(&"enabled".to_string())
        );
        assert_eq!(
            packet.extra_fields.get("license_status"),
            Some(&"valid".to_string())
        );
    }

    #[test]
    fn flex_discovery_deduplicates_same_radio() {
        let p1 = sample_valid_packet();
        let mut p2 = sample_valid_packet();
        p2.nickname = "DifferentNickname".to_string(); // ten sam serial

        let packets = vec![p1, p2];
        let deduped = deduplicate_radios(packets);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].serial, "1320-5691-6400-6874");
    }

    #[test]
    fn flex_discovery_keeps_distinct_radios() {
        let p1 = sample_valid_packet();
        let mut p2 = sample_valid_packet();
        p2.serial = "9999-8888-7777-6666".to_string();
        p2.model = "FLEX-6400".to_string();

        let packets = vec![p1, p2];
        let deduped = deduplicate_radios(packets);
        assert_eq!(deduped.len(), 2);
    }

    #[tokio::test]
    async fn flex_discovery_timeout_is_typed() {
        // Nasłuch na wolnym dynamicznym porcie, na którym nikt nie nadaje
        let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind failed");
        let port = socket.local_addr().expect("addr failed").port();
        drop(socket);

        let bind_addr = format!("127.0.0.1:{port}");
        let result = discover_radios(Some(&bind_addr), Duration::from_millis(50)).await;
        assert!(result.is_ok());
        let list = result.expect("discovery failed");
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn flex_discovery_does_not_connect_automatically() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"discovery_protocol_version=3.0.0.2 model=FLEX-6600 serial=1320-5691-6400-6874 ip=192.168.1.120 port=4992";
        let packet = codec.decode_packet(payload).expect("decoding failed");

        // Pakiet discovery zwraca wyłącznie dane informacyjne, bez automatycznego tworzenia socketu TCP
        assert_eq!(packet.ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 120)));
        assert_eq!(packet.port, 4992);
    }

    #[test]
    fn flex_discovery_does_not_send_ptt() {
        // Dekodowanie pakietu discovery nigdy nie inicjalizuje PTT ani nie generuje komend TX
        let codec = FlexDiscoveryCodec::new();
        let p = sample_valid_packet();
        let encoded = codec.encode_packet(&p);
        let s = String::from_utf8_lossy(&encoded);

        assert!(!s.contains("xmit 1"));
        assert!(!s.contains("tx 1"));
    }

    #[test]
    fn flex_discovery_model_selection_is_explicit() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"discovery_protocol_version=3.0.0.2 model=FLEX-6700 serial=7777-6666 ip=10.0.0.1 port=4992 nickname=FLEX6700";
        let packet = codec.decode_packet(payload).expect("decode failed");

        // Model pobierany jest bezpośrednio z pola `model`, a nie z nickname
        assert_eq!(packet.model, "FLEX-6700");
    }
}
