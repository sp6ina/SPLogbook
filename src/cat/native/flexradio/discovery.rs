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
use super::protocol::{FlexProtocolError, tokenize_smart_sdr_fields};

/// Domyślny oficjalny port rozgłoszeniowy UDP SmartSDR.
pub const FLEX_DISCOVERY_PORT: u16 = 4992;

/// Oficjalny Stream ID dla pakietów discovery VITA-49 SmartSDR.
pub const FLEX_DISCOVERY_STREAM_ID: u32 = 0x0000_0800;

/// Oficjalny typ pakietu VITA-49: Extension Data Packet with Stream ID (0x03).
pub const VITA_PACKET_TYPE_EXT_DATA_WITH_STREAM_ID: u8 = 0x03;

/// Oficjalny wskaźnik czasu TSI: Other (0b11).
pub const VITA_TSI_OTHER: u8 = 0b11;

/// Oficjalny wskaźnik czasu TSF: Sample Count 64-bit (0b01).
pub const VITA_TSF_SAMPLE_COUNT: u8 = 0b01;

/// Oficjalny 24-bitowy IEEE OUI producenta FlexRadio Systems, Inc. (0x001C2D).
pub const FLEX_OUI: u32 = 0x0000_1C2D;

/// Oficjalny 16-bitowy kod klasy informacji SmoothLake (ASCII "SL" = 0x534C).
pub const FLEX_INFO_CLASS_SMOOTHLAKE: u16 = 0x534C;

/// Oficjalny 16-bitowy kod klasy pakietu discovery broadcast (0xFFFF).
pub const FLEX_PACKET_CLASS_DISCOVERY: u16 = 0xFFFF;

/// Oficjalne dolne słowo Class ID (0x534CFFFF: Information Class 0x534C, Packet Class 0xFFFF).
pub const FLEX_DISCOVERY_CLASS_ID_LOW: u32 = 0x534C_FFFF;

/// Oficjalna górna część Class ID (OUI w 24 najmłodszych bitach).
pub const FLEX_DISCOVERY_CLASS_ID_HIGH: u32 = FLEX_OUI;

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
    pub tsi_code: u8,
    pub tsf_code: u8,
    pub packet_count: u8,
    pub packet_size_words: u16,
    pub stream_id: u32,
    pub class_id_high: u32,
    pub class_id_low: u32,
    pub raw_timestamp_integer: u32,
    pub raw_timestamp_fractional: u64,
}

impl Default for FlexDiscoveryVitaHeader {
    fn default() -> Self {
        Self {
            packet_type: VITA_PACKET_TYPE_EXT_DATA_WITH_STREAM_ID, // 0x03
            class_id_present: true,
            trailer_present: false,
            tsi_code: VITA_TSI_OTHER,        // 0b11 (Other)
            tsf_code: VITA_TSF_SAMPLE_COUNT, // 0b01 (Sample Count)
            packet_count: 0,
            packet_size_words: 7,
            stream_id: FLEX_DISCOVERY_STREAM_ID,
            class_id_high: FLEX_DISCOVERY_CLASS_ID_HIGH,
            class_id_low: FLEX_DISCOVERY_CLASS_ID_LOW,
            raw_timestamp_integer: 0,
            raw_timestamp_fractional: 0,
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
        let tsi_code = ((word0 >> 22) & 0x03) as u8;
        let tsf_code = ((word0 >> 20) & 0x03) as u8;
        let packet_count = ((word0 >> 16) & 0x0F) as u8;
        let packet_size_words = (word0 & 0xFFFF) as u16;

        // Dokładny typ pakietu VITA-49 discovery: wyłącznie Extension Data with Stream ID (0x03)
        if packet_type != VITA_PACKET_TYPE_EXT_DATA_WITH_STREAM_ID {
            return Err(FlexProtocolError::UnsupportedVitaPacketType {
                expected: VITA_PACKET_TYPE_EXT_DATA_WITH_STREAM_ID,
                actual: packet_type,
            });
        }

        if !class_id_present {
            return Err(FlexProtocolError::MissingClassId);
        }

        if tsi_code != VITA_TSI_OTHER {
            return Err(FlexProtocolError::UnsupportedVitaTsi {
                expected: VITA_TSI_OTHER,
                actual: tsi_code,
            });
        }

        if tsf_code != VITA_TSF_SAMPLE_COUNT {
            return Err(FlexProtocolError::UnsupportedVitaTsf {
                expected: VITA_TSF_SAMPLE_COUNT,
                actual: tsf_code,
            });
        }

        if trailer_present {
            return Err(FlexProtocolError::TrailerNotPermitted);
        }

        let expected_total_bytes = (packet_size_words as usize) * 4;
        if (packet_size_words as usize) < 7 || bytes.len() != expected_total_bytes {
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

        // Słowa 2 i 3: Class ID
        // Word 2 (class_id_high): 24-bit IEEE OUI w najmłodszych 24 bitach (0x001C2D)
        // Bity 31..24 to pole zarezerwowane/rozszerzenia (nie zakładamy zera, zachowujemy diagnostycznie)
        let class_id_high = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let oui = class_id_high & 0x00FF_FFFF;
        if oui != FLEX_OUI {
            return Err(FlexProtocolError::WrongOui {
                expected: FLEX_OUI,
                actual: oui,
            });
        }

        // Word 3 (class_id_low): Information Class Code (0x534C) i Packet Class Code (0xFFFF)
        let class_id_low = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let info_class = ((class_id_low >> 16) & 0xFFFF) as u16;
        if info_class != FLEX_INFO_CLASS_SMOOTHLAKE {
            return Err(FlexProtocolError::WrongInformationClass {
                expected: FLEX_INFO_CLASS_SMOOTHLAKE,
                actual: info_class,
            });
        }
        let packet_class = (class_id_low & 0xFFFF) as u16;
        if packet_class != FLEX_PACKET_CLASS_DISCOVERY {
            return Err(FlexProtocolError::WrongPacketClass {
                expected: FLEX_PACKET_CLASS_DISCOVERY,
                actual: packet_class,
            });
        }

        // Słowa 4, 5, 6: Timestamp fields (przechowywane jako surowe bez konwersji na UTC/GPS/pikosekundy)
        let raw_timestamp_integer =
            u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let timestamp_frac_high = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let timestamp_frac_low = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let raw_timestamp_fractional =
            ((timestamp_frac_high as u64) << 32) | (timestamp_frac_low as u64);

        let header = FlexDiscoveryVitaHeader {
            packet_type,
            class_id_present,
            trailer_present,
            tsi_code,
            tsf_code,
            packet_count,
            packet_size_words,
            stream_id,
            class_id_high,
            class_id_low,
            raw_timestamp_integer,
            raw_timestamp_fractional,
        };

        // Wyznaczenie granicy payloadu (wyłącznie końcowe bajty NUL od 0 do 4 jako terminacja i wyrównanie 32-bitowe)
        let payload_slice = &bytes[VITA49_DISCOVERY_HEADER_BYTES..expected_total_bytes];
        let trailing_nuls = payload_slice.iter().rev().take_while(|&&b| b == 0).count();
        if trailing_nuls > 4 {
            return Err(FlexProtocolError::ExcessiveNulPadding {
                actual: trailing_nuls,
                max: 4,
            });
        }

        let text_bytes = &payload_slice[..payload_slice.len() - trailing_nuls];
        if text_bytes.contains(&0) {
            return Err(FlexProtocolError::EmbeddedNulInPayload);
        }

        let text = std::str::from_utf8(text_bytes)
            .map_err(|e| FlexProtocolError::InvalidUtf8(e.to_string()))?;

        let (_, mut fields) = tokenize_smart_sdr_fields(text);

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
        let payload_len_bytes = payload.len();
        // Oficjalny algorytm producenta: (payload_len_bytes + 4) / 4 słów z terminacją i paddingiem
        let payload_words = (payload_len_bytes + 4) / 4;
        let total_payload_len = payload_words * 4;
        let pad_len = total_payload_len - payload_len_bytes;
        let total_words = ((VITA49_DISCOVERY_HEADER_BYTES + total_payload_len) / 4) as u16;

        let mut buf = Vec::with_capacity(total_words as usize * 4);
        let word0: u32 = ((packet.header.packet_type as u32) << 28)
            | ((packet.header.class_id_present as u32) << 27)
            | ((packet.header.trailer_present as u32) << 26)
            | ((packet.header.tsi_code as u32) << 22)
            | ((packet.header.tsf_code as u32) << 20)
            | ((packet.header.packet_count as u32) << 16)
            | (total_words as u32);

        buf.extend_from_slice(&word0.to_be_bytes());
        buf.extend_from_slice(&packet.header.stream_id.to_be_bytes());
        buf.extend_from_slice(&packet.header.class_id_high.to_be_bytes());
        buf.extend_from_slice(&packet.header.class_id_low.to_be_bytes());
        buf.extend_from_slice(&packet.header.raw_timestamp_integer.to_be_bytes());
        buf.extend_from_slice(&packet.header.raw_timestamp_fractional.to_be_bytes());
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
        let payload_len_bytes = payload.len();
        let payload_words = (payload_len_bytes + 4) / 4;
        let total_payload_len = payload_words * 4;
        let pad_len = total_payload_len - payload_len_bytes;
        let total_words = ((VITA49_DISCOVERY_HEADER_BYTES + total_payload_len) / 4) as u16;
        let word0: u32 = ((VITA_PACKET_TYPE_EXT_DATA_WITH_STREAM_ID as u32) << 28)
            | (1 << 27) // Class ID present (bit 26 trailer present = 0)
            | ((VITA_TSI_OTHER as u32) << 22)
            | ((VITA_TSF_SAMPLE_COUNT as u32) << 20)
            | (total_words as u32);

        let mut buf = Vec::with_capacity(total_words as usize * 4);
        buf.extend_from_slice(&word0.to_be_bytes());
        buf.extend_from_slice(&FLEX_DISCOVERY_STREAM_ID.to_be_bytes());
        buf.extend_from_slice(&FLEX_OUI.to_be_bytes()); // Słowo 2: 24-bit OUI 0x001C2D
        buf.extend_from_slice(&FLEX_DISCOVERY_CLASS_ID_LOW.to_be_bytes()); // Słowo 3: 0x534CFFFF
        buf.extend_from_slice(&0_u32.to_be_bytes()); // raw timestamp int: 0 per official spec
        buf.extend_from_slice(&0_u32.to_be_bytes()); // raw timestamp frac high: 0 per official spec
        buf.extend_from_slice(&0_u32.to_be_bytes()); // raw timestamp frac low: 0 per official spec
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
        assert_eq!(
            packet.header.packet_type,
            VITA_PACKET_TYPE_EXT_DATA_WITH_STREAM_ID
        );
        assert_eq!(packet.header.stream_id, FLEX_DISCOVERY_STREAM_ID);
        assert_eq!(packet.header.class_id_high & 0x00FF_FFFF, FLEX_OUI);
        assert_eq!(packet.header.class_id_low, FLEX_DISCOVERY_CLASS_ID_LOW);
    }

    #[test]
    fn flex_discovery_accepts_only_official_packet_type() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.120";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec
            .decode_packet(&datagram)
            .expect("should accept official type");
        assert_eq!(packet.header.packet_type, 0x03);
    }

    #[test]
    fn flex_discovery_rejects_context_packet() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Ustaw typ na 0x4 (Context packet)
        datagram[0] = (datagram[0] & 0x0F) | (0x4 << 4);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject context packet");
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaPacketType {
                expected: 0x03,
                actual: 0x04
            }
        ));
    }

    #[test]
    fn flex_discovery_rejects_extension_context_packet() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Ustaw typ na 0x5 (Extension Context packet)
        datagram[0] = (datagram[0] & 0x0F) | (0x5 << 4);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject extension context packet");
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaPacketType {
                expected: 0x03,
                actual: 0x05
            }
        ));
    }

    #[test]
    fn flex_discovery_rejects_extension_data_without_stream_id() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Ustaw typ na 0x2 (Extension data w/o stream ID)
        datagram[0] = (datagram[0] & 0x0F) | (0x2 << 4);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject extension data without stream ID");
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaPacketType {
                expected: 0x03,
                actual: 0x02
            }
        ));
    }

    #[test]
    fn flex_discovery_rejects_plain_text_datagram() {
        let codec = FlexDiscoveryCodec::new();
        let plain_text = b"discovery_protocol_version=3.0.0.2 model=FLEX-6600 serial=1320 ip=192.168.1.1 port=4992";
        let err = codec
            .decode_packet(plain_text)
            .expect_err("should reject plain text");
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaPacketType { .. }
                | FlexProtocolError::InvalidVitaHeader(_)
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
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaPacketType {
                expected: 0x03,
                actual: 0x00
            }
        ));
    }

    #[test]
    fn flex_discovery_class_id_layout_matches_spec() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let datagram = build_valid_vita49_packet(payload);

        let packet = codec.decode_packet(&datagram).expect("decoding failed");
        assert_eq!(packet.header.class_id_high & 0x00FF_FFFF, FLEX_OUI);
        assert_eq!((packet.header.class_id_low >> 16) as u16, 0x534C);
        assert_eq!((packet.header.class_id_low & 0xFFFF) as u16, 0xFFFF);
    }

    #[test]
    fn flex_discovery_rejects_wrong_oui() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień OUI z 0x001C2D na 0x112233
        datagram[9] = 0x11;
        datagram[10] = 0x22;
        datagram[11] = 0x33;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject wrong OUI");
        assert!(matches!(err, FlexProtocolError::WrongOui { .. }));
    }

    #[test]
    fn flex_discovery_rejects_wrong_information_class() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień Information Class z 0x534C ("SL") na 0x1234
        datagram[12] = 0x12;
        datagram[13] = 0x34;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject wrong info class");
        assert!(matches!(
            err,
            FlexProtocolError::WrongInformationClass {
                expected: 0x534C,
                actual: 0x1234
            }
        ));
    }

    #[test]
    fn flex_discovery_rejects_wrong_packet_class() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień Packet Class z 0xFFFF na 0x0001
        datagram[14] = 0x00;
        datagram[15] = 0x01;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject wrong packet class");
        assert!(matches!(
            err,
            FlexProtocolError::WrongPacketClass {
                expected: 0xFFFF,
                actual: 0x0001
            }
        ));
    }

    #[test]
    fn flex_discovery_does_not_assume_undocumented_zero_word() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let mut datagram = build_valid_vita49_packet(payload);
        // Ustaw bajt 8 (bity 31..24 słowa class_id_high, pole pad/vendor) na wartość niezerową 0xAA
        datagram[8] = 0xAA;
        let packet = codec
            .decode_packet(&datagram)
            .expect("should accept non-zero pad word if OUI matches");
        assert_eq!(packet.header.class_id_high >> 24, 0xAA);
        assert_eq!(packet.header.class_id_high & 0x00FF_FFFF, FLEX_OUI);
    }

    #[test]
    fn flex_discovery_header_layout_matches_spec() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decode failed");

        assert_eq!(packet.header.packet_type, 0x03);
        assert!(packet.header.class_id_present);
        assert!(!packet.header.trailer_present);
        assert_eq!(packet.header.tsi_code, 0b11);
        assert_eq!(packet.header.tsf_code, 0b01);
    }

    #[test]
    fn flex_discovery_rejects_unsupported_tsi() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień TSI z 0b11 (Other) na 0b01 (UTC)
        datagram[1] = (datagram[1] & !(0x03 << 6)) | (0b01 << 6);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject unsupported TSI");
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaTsi {
                expected: 0b11,
                actual: 0b01
            }
        ));
    }

    #[test]
    fn flex_discovery_rejects_unsupported_tsf() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Zmień TSF z 0b01 (Sample count) na 0b10 (Real-time)
        datagram[1] = (datagram[1] & !(0x03 << 4)) | (0b10 << 4);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject unsupported TSF");
        assert!(matches!(
            err,
            FlexProtocolError::UnsupportedVitaTsf {
                expected: 0b01,
                actual: 0b10
            }
        ));
    }

    #[test]
    fn flex_discovery_handles_or_rejects_trailer_explicitly() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Ustaw bit trailer present (bit 26)
        datagram[0] |= 1 << 2;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should explicitly reject trailer in discovery packet");
        assert!(matches!(err, FlexProtocolError::TrailerNotPermitted));
    }

    #[test]
    fn flex_discovery_trailer_policy_is_explicit() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        datagram[0] |= 1 << 2; // trailer bit set
        let err = codec
            .decode_packet(&datagram)
            .expect_err("trailer rejected");
        assert_eq!(err, FlexProtocolError::TrailerNotPermitted);
    }

    #[test]
    fn flex_discovery_packet_size_matches_datagram_exactly() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        // Dodaj 4 nadmiarowe bajty poza packet_size
        datagram.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject extra bytes beyond packet_size");
        assert!(matches!(err, FlexProtocolError::InvalidPacketSize { .. }));
    }

    #[test]
    fn flex_discovery_padding_is_bounded() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1234 ip=10.0.0.1";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec
            .decode_packet(&datagram)
            .expect("valid padding accepted");
        assert_eq!(packet.model, "FLEX-6400");
    }

    #[test]
    fn flex_discovery_zero_timestamps_are_valid() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let datagram = build_valid_vita49_packet(payload);
        assert_eq!(&datagram[16..20], &[0, 0, 0, 0]);
        assert_eq!(&datagram[20..28], &[0, 0, 0, 0, 0, 0, 0, 0]);

        let packet = codec
            .decode_packet(&datagram)
            .expect("valid zero timestamps accepted");
        assert_eq!(packet.header.raw_timestamp_integer, 0);
        assert_eq!(packet.header.raw_timestamp_fractional, 0);
    }

    #[test]
    fn flex_discovery_timestamps_remain_raw() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let mut datagram = build_valid_vita49_packet(payload);
        let raw_int: u32 = 0x1234_5678;
        let raw_frac: u64 = 0x9ABC_DEF0_1357_9BDF;
        datagram[16..20].copy_from_slice(&raw_int.to_be_bytes());
        datagram[20..28].copy_from_slice(&raw_frac.to_be_bytes());

        let packet = codec.decode_packet(&datagram).expect("decoded");
        assert_eq!(packet.header.raw_timestamp_integer, raw_int);
        assert_eq!(packet.header.raw_timestamp_fractional, raw_frac);
    }

    #[test]
    fn flex_discovery_does_not_label_tsi_other_as_utc() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decoded");

        assert_eq!(packet.header.tsi_code, VITA_TSI_OTHER);
        assert_eq!(packet.header.tsi_code, 0b11);
        assert_ne!(packet.header.tsi_code, 0b01);
    }

    #[test]
    fn flex_discovery_does_not_convert_fractional_timestamp() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let mut datagram = build_valid_vita49_packet(payload);
        let sample_frac: u64 = 0xFEDC_BA98_7654_3210;
        datagram[20..28].copy_from_slice(&sample_frac.to_be_bytes());

        let packet = codec.decode_packet(&datagram).expect("decoded");
        assert_eq!(packet.header.raw_timestamp_fractional, sample_frac);
    }

    #[test]
    fn flex_discovery_tsi_tsf_codes_match_official_header() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6600 serial=1320 ip=192.168.1.50";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec.decode_packet(&datagram).expect("decoded");

        assert_eq!(packet.header.tsi_code, 0b11);
        assert_eq!(packet.header.tsf_code, 0b01);
    }

    #[test]
    fn flex_discovery_padding_matches_official_copy_algorithm() {
        for n in 0..100 {
            let payload_words = (n + 4) / 4;
            let total_payload_bytes = payload_words * 4;
            let pad_len = total_payload_bytes - n;
            let expected_pad = 4 - (n % 4);
            assert_eq!(pad_len, expected_pad);
            assert!((1..=4).contains(&pad_len));
        }
    }

    #[test]
    fn flex_discovery_payload_modulo_zero() {
        let codec = FlexDiscoveryCodec::new();
        let text = b"model=FLEX-6600 serial=1320 ip=192.168.1.50 tag=";
        assert_eq!(text.len() % 4, 0);
        let datagram = build_valid_vita49_packet(text);
        let trailing_nuls = datagram[28..].iter().rev().take_while(|&&b| b == 0).count();
        assert_eq!(trailing_nuls, 4);

        let packet = codec.decode_packet(&datagram).expect("decoded modulo 0");
        assert_eq!(packet.model, "FLEX-6600");
        assert_eq!(packet.serial, "1320");
    }

    #[test]
    fn flex_discovery_payload_modulo_one() {
        let codec = FlexDiscoveryCodec::new();
        let text = b"model=FLEX-6600 serial=1320 ip=192.168.1.50 tag=a";
        assert_eq!(text.len() % 4, 1);
        let datagram = build_valid_vita49_packet(text);
        let trailing_nuls = datagram[28..].iter().rev().take_while(|&&b| b == 0).count();
        assert_eq!(trailing_nuls, 3);

        let packet = codec.decode_packet(&datagram).expect("decoded modulo 1");
        assert_eq!(packet.model, "FLEX-6600");
    }

    #[test]
    fn flex_discovery_payload_modulo_two() {
        let codec = FlexDiscoveryCodec::new();
        let text = b"model=FLEX-6600 serial=1320 ip=192.168.1.50 tag=ab";
        assert_eq!(text.len() % 4, 2);
        let datagram = build_valid_vita49_packet(text);
        let trailing_nuls = datagram[28..].iter().rev().take_while(|&&b| b == 0).count();
        assert_eq!(trailing_nuls, 2);

        let packet = codec.decode_packet(&datagram).expect("decoded modulo 2");
        assert_eq!(packet.model, "FLEX-6600");
    }

    #[test]
    fn flex_discovery_payload_modulo_three() {
        let codec = FlexDiscoveryCodec::new();
        let text = b"model=FLEX-6600 serial=1320 ip=192.168.1.50 tag=abc";
        assert_eq!(text.len() % 4, 3);
        let datagram = build_valid_vita49_packet(text);
        let trailing_nuls = datagram[28..].iter().rev().take_while(|&&b| b == 0).count();
        assert_eq!(trailing_nuls, 1);

        let packet = codec.decode_packet(&datagram).expect("decoded modulo 3");
        assert_eq!(packet.model, "FLEX-6600");
    }

    #[test]
    fn flex_discovery_accepts_official_aligned_payload() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=5678 ip=10.0.0.5 port=4992";
        let datagram = build_valid_vita49_packet(payload);
        let packet = codec
            .decode_packet(&datagram)
            .expect("official aligned packet decoded");
        assert_eq!(packet.model, "FLEX-6400");
        assert_eq!(packet.serial, "5678");
        assert_eq!(packet.port, 4992);
        assert_eq!(packet.ip.to_string(), "10.0.0.5");
    }

    #[test]
    fn flex_discovery_packet_size_controls_payload_boundary() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1111 ip=10.0.0.1";
        let datagram = build_valid_vita49_packet(payload);
        let total_words = (datagram.len() / 4) as u16;

        let packet = codec.decode_packet(&datagram).expect("decoded");
        assert_eq!(packet.header.packet_size_words, total_words);

        let mut malformed = datagram.clone();
        malformed.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let err = codec
            .decode_packet(&malformed)
            .expect_err("extra bytes rejected");
        assert!(matches!(err, FlexProtocolError::InvalidPacketSize { .. }));
    }

    #[test]
    fn flex_discovery_rejects_excessive_nul_padding() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1234 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        let total_words = (datagram.len() / 4 + 2) as u16;
        datagram[2] = (total_words >> 8) as u8;
        datagram[3] = (total_words & 0xFF) as u8;
        datagram.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);

        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject excessive NUL padding");
        assert!(matches!(err, FlexProtocolError::ExcessiveNulPadding { .. }));
    }

    #[test]
    fn flex_discovery_rejects_embedded_nul() {
        let codec = FlexDiscoveryCodec::new();
        let mut payload = b"model=FLEX-6400 serial=1234 ip=10.0.0.1".to_vec();
        // Wstaw znak NUL w środku tekstu
        payload[10] = 0;
        let datagram = build_valid_vita49_packet(&payload);
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject embedded NUL");
        assert_eq!(err, FlexProtocolError::EmbeddedNulInPayload);
    }

    #[test]
    fn flex_discovery_payload_boundary_excludes_trailer() {
        let codec = FlexDiscoveryCodec::new();
        let payload = b"model=FLEX-6400 serial=1234 ip=10.0.0.1";
        let mut datagram = build_valid_vita49_packet(payload);
        datagram[0] |= 1 << 2; // set trailer bit
        // Powinno zostać odrzucone zgodnie z polityką braku trailera
        let err = codec
            .decode_packet(&datagram)
            .expect_err("trailer rejected");
        assert_eq!(err, FlexProtocolError::TrailerNotPermitted);
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
        // Zmień OUI na błędne
        datagram[10] = 0x99;
        let err = codec
            .decode_packet(&datagram)
            .expect_err("should reject wrong class ID / OUI");
        assert!(matches!(err, FlexProtocolError::WrongOui { .. }));
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
        assert_eq!(&datagram[8..12], &[0x00, 0x00, 0x1C, 0x2D]); // class ID high (OUI)
        assert_eq!(&datagram[12..16], &[0x53, 0x4C, 0xFF, 0xFF]); // class ID low

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
