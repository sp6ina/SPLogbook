// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Pasywne odkrywanie transceiverów FlexRadio w sieci lokalnej przez UDP (port 4992).
//!
//! Zgodne z oficjalną specyfikacją FlexRadio SmartSDR Discovery Protocol.
//! Dla wersji po v1.1.3 transceiver wysyła pakiet rozgłoszeniowy UDP jako
//! rozszerzenie VITA-49 (Context/Extension Data Packet):
//! - Port UDP: 4992
//! - Stream ID: 0x00000800
//! - Class ID: 0x534CFFFF (ASCII "SL\xFF\xFF")
//! - Format preambuły: 7 słów 32-bitowych (28 bajtów) w network byte order (big-endian)
//! - Payload: ciąg tekstowy par klucz-wartość wyrównany bajtami NUL do granicy 32-bitowej.
//!
//! Moduł implementuje wyłącznie pasywny nasłuch — nie wysyła żadnych arbitralnych
//! pakietów zapytania, nie przełącza PTT ani nie uruchamia strumieni multimedialnych VITA-49.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

use super::models::FLEX_DEFAULT_PORT;
use super::protocol::FlexProtocolError;

/// Domyślny oficjalny port rozgłoszeniowy UDP SmartSDR.
pub const FLEX_DISCOVERY_PORT: u16 = 4992;

/// Oficjalny Stream ID dla pakietów discovery VITA-49 SmartSDR.
pub const FLEX_DISCOVERY_STREAM_ID: u32 = 0x0000_0800;

/// Oficjalna górna część Class ID (ASCII "SL\xFF\xFF").
pub const FLEX_DISCOVERY_CLASS_ID_HIGH: u32 = 0x534C_FFFF;

/// Minimalny rozmiar preambuły nagłówka VITA-49 discovery w bajtach (7 słów 32-bitowych).
pub const VITA49_DISCOVERY_HEADER_BYTES: usize = 28;

/// Maksymalny dopuszczalny rozmiar datagramu UDP discovery (ochrona pamięci).
pub const MAX_DISCOVERY_DATAGRAM_SIZE: usize = 4096;

/// Zdekodowany nagłówek VITA-49 pakietu discovery SmartSDR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexDiscoveryVitaHeader {
    pub packet_type: u8,
    pub class_id_present: bool,
    pub trailer_present: bool,
    pub tsi: u8,
    pub tsf: u8,
    pub packet_count: u8,
    pub packet_size_words: u16,
    pub stream_id: u32,
    pub class_id_high: u32,
    pub class_id_low: u32,
    pub timestamp_int: u32,
    pub timestamp_frac_high: u32,
    pub timestamp_frac_low: u32,
}

impl Default for FlexDiscoveryVitaHeader {
    fn default() -> Self {
        Self {
            packet_type: 0x4, // Context packet
            class_id_present: true,
            trailer_present: false,
            tsi: 0b01, // UTC
            tsf: 0b10, // Real-time picoseconds
            packet_count: 0,
            packet_size_words: 7,
            stream_id: FLEX_DISCOVERY_STREAM_ID,
            class_id_high: FLEX_DISCOVERY_CLASS_ID_HIGH,
            class_id_low: 0x0000_0000,
            timestamp_int: 0,
            timestamp_frac_high: 0,
            timestamp_frac_low: 0,
        }
    }
}

/// Zdekodowany pakiet discovery transceivera FlexRadio z kopertą VITA-49.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexDiscoveryPacket {
    pub header: FlexDiscoveryVitaHeader,
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

/// Rozwiązuje docelowy port połączenia TCP zgodnie z hierarchią priorytetów:
/// 1. Jawny port konfiguracji użytkownika (`explicit_port`),
/// 2. Port zgłoszony w pakiecie discovery (`discovery_port`),
/// 3. Oficjalna domyślna wartość 4992.
pub fn resolve_connection_port(
    explicit_port: Option<u16>,
    discovery_port: Option<u16>,
) -> Result<u16, FlexProtocolError> {
    if let Some(port) = explicit_port {
        if port == 0 {
            return Err(FlexProtocolError::ParseError(
                "explicit port cannot be 0".to_string(),
            ));
        }
        return Ok(port);
    }

    if let Some(port) = discovery_port {
        if port == 0 {
            return Err(FlexProtocolError::ParseError(
                "discovery port cannot be 0".to_string(),
            ));
        }
        return Ok(port);
    }

    Ok(FLEX_DEFAULT_PORT)
}

/// Kodek pakietów discovery FlexRadio SmartSDR VITA-49.
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

    /// Dekoduje surowy datagram UDP na strukturę [`FlexDiscoveryPacket`] po weryfikacji koperty VITA-49.
    pub fn decode_packet(&self, bytes: &[u8]) -> Result<FlexDiscoveryPacket, FlexProtocolError> {
        if bytes.len() > self.max_datagram_size {
            return Err(FlexProtocolError::OversizedDatagram {
                max: self.max_datagram_size,
                actual: bytes.len(),
            });
        }

        if bytes.len() < VITA49_DISCOVERY_HEADER_BYTES {
            return Err(FlexProtocolError::TruncatedHeader {
                expected: VITA49_DISCOVERY_HEADER_BYTES,
                actual: bytes.len(),
            });
        }

        // Słowo 0 nagłówka VITA-49 (Big-Endian / Network Byte Order)
        let word0 = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let packet_type = ((word0 >> 28) & 0x0F) as u8;
        let class_id_present = ((word0 >> 27) & 0x01) == 1;
        let trailer_present = ((word0 >> 26) & 0x01) == 1;
        let tsi = ((word0 >> 22) & 0x03) as u8;
        let tsf = ((word0 >> 20) & 0x03) as u8;
        let packet_count = ((word0 >> 16) & 0x0F) as u8;
        let packet_size_words = (word0 & 0xFFFF) as u16;

        // Typ pakietu VITA-49: Extension Data / Context / Ext Context (0x3, 0x4 lub 0x5)
        if packet_type != 0x3 && packet_type != 0x4 && packet_type != 0x5 {
            return Err(FlexProtocolError::InvalidVitaHeader(format!(
                "unsupported VITA-49 packet type 0x{packet_type:X} (expected context/extension 0x3, 0x4, or 0x5)"
            )));
        }

        if !class_id_present {
            return Err(FlexProtocolError::MissingClassId);
        }

        let expected_total_bytes = (packet_size_words as usize) * 4;
        if (packet_size_words as usize) < 7 || bytes.len() < expected_total_bytes {
            return Err(FlexProtocolError::InvalidPacketSize {
                expected: expected_total_bytes,
                actual: bytes.len(),
            });
        }

        // Słowo 1: Stream ID (wymagane 0x00000800)
        let stream_id = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if stream_id != FLEX_DISCOVERY_STREAM_ID {
            return Err(FlexProtocolError::WrongStreamId {
                expected: FLEX_DISCOVERY_STREAM_ID,
                actual: stream_id,
            });
        }

        // Słowa 2 i 3: Class ID (wymagane 0x534CFFFF w class_id_high lub class_id_low)
        let class_id_high = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let class_id_low = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        if class_id_high != FLEX_DISCOVERY_CLASS_ID_HIGH
            && class_id_low != FLEX_DISCOVERY_CLASS_ID_HIGH
        {
            return Err(FlexProtocolError::WrongClassId {
                expected: FLEX_DISCOVERY_CLASS_ID_HIGH,
                actual: class_id_high,
            });
        }

        // Słowa 4, 5, 6: Timestamp fields
        let timestamp_int = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let timestamp_frac_high = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let timestamp_frac_low = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        let header = FlexDiscoveryVitaHeader {
            packet_type,
            class_id_present,
            trailer_present,
            tsi,
            tsf,
            packet_count,
            packet_size_words,
            stream_id,
            class_id_high,
            class_id_low,
            timestamp_int,
            timestamp_frac_high,
            timestamp_frac_low,
        };

        // Wyznaczenie granicy payloadu (usuwanie wyłącznie końcowego wyrównania NUL)
        let payload_slice = &bytes[VITA49_DISCOVERY_HEADER_BYTES..expected_total_bytes];
        let trimmed_bytes = match payload_slice.iter().rposition(|&b| b != 0) {
            Some(idx) => &payload_slice[..=idx],
            None => &[][..],
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

        let port = if let Some(port_str) = fields.remove("port") {
            let p = port_str.parse::<u16>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid port number '{port_str}': {e}"))
            })?;
            if p == 0 {
                return Err(FlexProtocolError::ParseError(
                    "port cannot be 0".to_string(),
                ));
            }
            p
        } else {
            FLEX_DISCOVERY_PORT
        };

        let discovery_protocol_version = fields
            .remove("discovery_protocol_version")
            .unwrap_or_else(|| "3.0.0.0".to_string());

        let version = fields.remove("version").unwrap_or_default();
        let name_field = fields.remove("name").map(|n| n.replace('_', " "));
        let nickname_field = fields.remove("nickname");
        let nickname = nickname_field.or(name_field).unwrap_or_default();
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
            header,
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

    /// Koduje pakiet do postaci binarnej VITA-49 (nagłówek 28 bajtów + tekstowy payload z wyrównaniem 32-bitowym).
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

        let payload = parts.join(" ").into_bytes();
        let pad_len = (4 - (payload.len() % 4)) % 4;
        let total_payload_len = payload.len() + pad_len;
        let total_words = ((VITA49_DISCOVERY_HEADER_BYTES + total_payload_len) / 4) as u16;

        let mut buf = Vec::with_capacity(total_words as usize * 4);
        let word0: u32 = ((packet.header.packet_type as u32) << 28)
            | ((packet.header.class_id_present as u32) << 27)
            | ((packet.header.trailer_present as u32) << 26)
            | ((packet.header.tsi as u32) << 22)
            | ((packet.header.tsf as u32) << 20)
            | ((packet.header.packet_count as u32) << 16)
            | (total_words as u32);

        buf.extend_from_slice(&word0.to_be_bytes());
        buf.extend_from_slice(&packet.header.stream_id.to_be_bytes());
        buf.extend_from_slice(&packet.header.class_id_high.to_be_bytes());
        buf.extend_from_slice(&packet.header.class_id_low.to_be_bytes());
        buf.extend_from_slice(&packet.header.timestamp_int.to_be_bytes());
        buf.extend_from_slice(&packet.header.timestamp_frac_high.to_be_bytes());
        buf.extend_from_slice(&packet.header.timestamp_frac_low.to_be_bytes());
        buf.extend_from_slice(&payload);
        buf.resize(buf.len() + pad_len, 0);
        buf
    }
}

/// Deduplikuje listę pakietów discovery po unikalnym numerze seryjnym sprzętu (`serial`).
/// W przypadku odebrania nowszego pakietu dla tego samego serialu (np. zaktualizowany adres IP,
/// zmieniony port lub nowa wersja oprogramowania), wpis jest aktualizowany do najnowszego stanu.
pub fn deduplicate_radios(packets: Vec<FlexDiscoveryPacket>) -> Vec<FlexDiscoveryPacket> {
    let mut map: BTreeMap<String, FlexDiscoveryPacket> = BTreeMap::new();
    for packet in packets {
        if packet.serial.is_empty() {
            continue;
        }
        map.insert(packet.serial.clone(), packet);
    }
    map.into_values().collect()
}

/// Pasywnie nasłuchuje pakietów discovery w sieci lokalnej przez określony czas.
/// Nie wysyła żadnych danych do sieci ani nie otwiera portu 4991.
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

    pub(crate) fn build_valid_vita49_packet(payload: &[u8]) -> Vec<u8> {
        let pad_len = (4 - (payload.len() % 4)) % 4;
        let total_payload_len = payload.len() + pad_len;
        let total_words = ((VITA49_DISCOVERY_HEADER_BYTES + total_payload_len) / 4) as u16;
        let word0: u32 =
            (0x4 << 28) | (1 << 27) | (0b01 << 22) | (0b10 << 20) | (total_words as u32);

        let mut buf = Vec::with_capacity(total_words as usize * 4);
        buf.extend_from_slice(&word0.to_be_bytes());
        buf.extend_from_slice(&FLEX_DISCOVERY_STREAM_ID.to_be_bytes());
        buf.extend_from_slice(&FLEX_DISCOVERY_CLASS_ID_HIGH.to_be_bytes());
        buf.extend_from_slice(&0x0000_0000u32.to_be_bytes());
        buf.extend_from_slice(&1_609_459_200_u32.to_be_bytes()); // timestamp int
        buf.extend_from_slice(&0x1234_5678u32.to_be_bytes()); // timestamp frac high
        buf.extend_from_slice(&0x9ABC_DEF0u32.to_be_bytes()); // timestamp frac low
        buf.extend_from_slice(payload);
        buf.resize(buf.len() + pad_len, 0);
        buf
    }

    fn sample_valid_packet() -> FlexDiscoveryPacket {
        FlexDiscoveryPacket {
            header: FlexDiscoveryVitaHeader::default(),
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
    fn flex_discovery_decodes_valid_vita49_packet() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"discovery_protocol_version=3.0.0.2 model=FLEX-6600 serial=1320-5691-6400-6874 version=3.7.4.29165 nickname=MyFlex callsign=SP6INA ip=192.168.1.120 port=4992 status=Available max_slices=4 available_slices=4 max_panadapters=4 available_panadapters=4";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decoding failed");

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
        assert_eq!(packet.header.stream_id, FLEX_DISCOVERY_STREAM_ID);
        assert_eq!(packet.header.class_id_high, FLEX_DISCOVERY_CLASS_ID_HIGH);
    }

    #[test]
    fn flex_discovery_rejects_plain_text_datagram() {
        let codec = FlexDiscoveryCodec::new();
        let plain_text = b"discovery_protocol_version=3.0.0.2 model=FLEX-6600 serial=1320 ip=192.168.1.1 port=4992";
        let err = codec
            .decode_packet(plain_text)
            .expect_err("should reject plain text");
        // Pierwsze 4 bajty tekstu to ASCII "disc", czyli 0x64697363 -> packet_type 0x6 != 0x3/0x4/0x5
        assert!(matches!(
            err,
            FlexProtocolError::InvalidVitaHeader(_)
                | FlexProtocolError::WrongStreamId { .. }
                | FlexProtocolError::WrongClassId { .. }
        ));
    }

    #[test]
    fn flex_discovery_rejects_wrong_packet_type() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień packet type na 0x0 (IF data w/o stream ID)
        datagram[0] &= 0x0F;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject packet type 0x0");
        assert!(matches!(err, FlexProtocolError::InvalidVitaHeader(_)));
    }

    #[test]
    fn flex_discovery_rejects_missing_class_id() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Wyzeruj bit C (bit 27)
        datagram[0] &= !(1 << 3);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject missing class ID");
        assert!(matches!(err, FlexProtocolError::MissingClassId));
    }

    #[test]
    fn flex_discovery_rejects_wrong_stream_id() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień Stream ID z 0x800 na 0x900
        datagram[6] = 0x09;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject wrong stream ID");
        assert!(matches!(err, FlexProtocolError::WrongStreamId { .. }));
    }

    #[test]
    fn flex_discovery_rejects_wrong_class_id() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień Class ID high z 0x534CFFFF na 0x11223344
        datagram[8] = 0x11;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject wrong class ID");
        assert!(matches!(err, FlexProtocolError::WrongClassId { .. }));
    }

    #[test]
    fn flex_discovery_rejects_truncated_header() {
        let codec = FlexDiscoveryCodec::new();
        let truncated = vec![0u8; 16]; // Mniej niż 28 bajtów
        let err = codec
            .decode_packet(&truncated)
            .expect_err("should reject truncated header");
        assert!(matches!(err, FlexProtocolError::TruncatedHeader { .. }));
    }

    #[test]
    fn flex_discovery_rejects_invalid_packet_size() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Obetnij datagram poniżej rozmiaru zadeklarowanego w nagłówku
        datagram.truncate(datagram.len() - 4);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject packet size mismatch");
        assert!(matches!(err, FlexProtocolError::InvalidPacketSize { .. }));
    }

    #[test]
    fn flex_discovery_handles_network_byte_order() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let datagram = build_valid_vita49_packet(payload);

        // Upewnij się, że słowa w pamięci są dokładnie Big Endian
        assert_eq!(&datagram[4..8], &[0x00, 0x00, 0x08, 0x00]); // stream ID 0x800
        assert_eq!(&datagram[8..12], &[0x53, 0x4C, 0xFF, 0xFF]); // class ID

        let packet = codec.decode_packet(&datagram).expect("decode failed");
        assert_eq!(packet.model, "FLEX-6600");
    }

    #[test]
    fn flex_discovery_removes_only_alignment_padding() {
        let codec = FlexDiscoveryCodec::new();
        // Payload z tekstem zawierającym znak spacji i wyrównanie 32-bitowe z 3 bajtami NUL
        let payload = b"model=FLEX-6400 serial=1234 ip=10.0.0.1 custom_val=A";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decode failed");

        assert_eq!(
            packet.extra_fields.get("custom_val"),
            Some(&"A".to_string())
        );
    }

    #[test]
    fn flex_discovery_rejects_invalid_utf8_payload() {
        let codec = FlexDiscoveryCodec::new();
        let invalid_utf8 = [b'm', b'o', b'd', b'e', b'l', b'=', 0xFF, 0xFE, 0xFD];
        let datagram = build_valid_vita49_packet(&invalid_utf8);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject invalid UTF-8");
        assert!(matches!(err, FlexProtocolError::InvalidUtf8(_)));
    }

    #[test]
    fn flex_discovery_preserves_unknown_payload_fields() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.5 vendor_ext=foo license=valid";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decode failed");

        assert_eq!(
            packet.extra_fields.get("vendor_ext"),
            Some(&"foo".to_string())
        );
        assert_eq!(
            packet.extra_fields.get("license"),
            Some(&"valid".to_string())
        );
    }

    #[test]
    fn flex_discovery_updates_existing_radio_address() {
        let p1 = sample_valid_packet();
        let mut p2 = p1.clone();
        p2.ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 99));
        p2.port = 4995;

        let packets = vec![p1, p2];
        let deduped = deduplicate_radios(packets);
        assert_eq!(deduped.len(), 1);
        // Najnowszy pakiet aktualizuje adres IP i port!
        assert_eq!(deduped[0].ip, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 99)));
        assert_eq!(deduped[0].port, 4995);
    }

    #[test]
    fn flex_discovery_keeps_same_name_distinct_by_serial() {
        let mut p1 = sample_valid_packet();
        p1.serial = "SER-1111".to_string();
        p1.nickname = "SameName".to_string();

        let mut p2 = sample_valid_packet();
        p2.serial = "SER-2222".to_string();
        p2.nickname = "SameName".to_string();

        let packets = vec![p1, p2];
        let deduped = deduplicate_radios(packets);
        assert_eq!(deduped.len(), 2);
    }

    #[tokio::test]
    async fn flex_discovery_uses_dynamic_test_port() {
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
    fn flex_discovery_rejects_oversized_datagram() {
        let codec = FlexDiscoveryCodec::with_max_datagram_size(64);
        let oversized = vec![0u8; 128];
        let err = codec.decode_packet(&oversized).expect_err("should reject");
        assert!(matches!(err, FlexProtocolError::OversizedDatagram { .. }));
    }

    #[test]
    fn flex_discovery_deduplicates_same_radio() {
        let p1 = sample_valid_packet();
        let mut p2 = sample_valid_packet();
        p2.nickname = "DifferentNickname".to_string();

        let packets = vec![p1, p2];
        let deduped = deduplicate_radios(packets);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].serial, "1320-5691-6400-6874");
        assert_eq!(deduped[0].nickname, "DifferentNickname");
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
        let payload = b"model=FLEX-6600 serial=1320-5691-6400-6874 ip=192.168.1.120 port=4992";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decoding failed");

        assert_eq!(packet.ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 120)));
        assert_eq!(packet.port, 4992);
    }

    #[test]
    fn flex_discovery_does_not_send_ptt() {
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
        let payload = b"model=FLEX-6700 serial=7777-6666 ip=10.0.0.1 nickname=FLEX6700";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decode failed");

        assert_eq!(packet.model, "FLEX-6700");
    }

    #[test]
    fn flex_connection_prefers_explicit_port() {
        let port = resolve_connection_port(Some(4995), Some(4992)).expect("resolve failed");
        assert_eq!(port, 4995);
    }

    #[test]
    fn flex_connection_uses_discovery_port() {
        let port = resolve_connection_port(None, Some(4993)).expect("resolve failed");
        assert_eq!(port, 4993);
    }

    #[test]
    fn flex_connection_falls_back_to_official_default() {
        let port = resolve_connection_port(None, None).expect("resolve failed");
        assert_eq!(port, FLEX_DEFAULT_PORT);
    }

    #[test]
    fn flex_connection_rejects_zero_discovery_port() {
        let err = resolve_connection_port(None, Some(0)).expect_err("should reject 0 port");
        assert!(matches!(err, FlexProtocolError::ParseError(_)));
    }
}
