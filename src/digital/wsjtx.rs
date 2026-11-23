// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Dwukierunkowa współpraca z WSJT-X / JTDX przez protokół UDP (`NetworkMessage.hpp`).
//!
//! Program odbiera stan, dekody i zalogowane QSO, a odsyła polecenia Reply (odpowiedź na
//! CQ jak dwuklik w oknie Band Activity) i HighlightCallsign (kolorowanie znaków).

use crate::core::qso::QsoRecord;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

const WSJTX_MAGIC: u32 = 0xadbc_cbda;
/// Schemat 2 (Qt 5.2) rozumie każda wersja WSJT-X od 1.5 oraz JTDX.
const WSJTX_SCHEMA: u32 = 2;
/// Najdłuższy akceptowany napis w pakiecie — ochrona przed zniekształconą ramką UDP.
const MAX_STRING_LEN: usize = 4096;
/// Domyślny port serwera UDP w WSJT-X (Settings → Reporting → UDP Server).
pub const DEFAULT_WSJTX_PORT: u16 = 2237;

fn read_u8(rdr: &mut Cursor<&[u8]>) -> Option<u8> {
    let mut buf = [0u8; 1];
    rdr.read_exact(&mut buf).ok()?;
    Some(buf[0])
}

fn read_bool(rdr: &mut Cursor<&[u8]>) -> Option<bool> {
    read_u8(rdr).map(|b| b != 0)
}

fn read_u32_be(rdr: &mut Cursor<&[u8]>) -> Option<u32> {
    let mut buf = [0u8; 4];
    rdr.read_exact(&mut buf).ok()?;
    Some(u32::from_be_bytes(buf))
}

fn read_i32_be(rdr: &mut Cursor<&[u8]>) -> Option<i32> {
    let mut buf = [0u8; 4];
    rdr.read_exact(&mut buf).ok()?;
    Some(i32::from_be_bytes(buf))
}

fn read_u64_be(rdr: &mut Cursor<&[u8]>) -> Option<u64> {
    let mut buf = [0u8; 8];
    rdr.read_exact(&mut buf).ok()?;
    Some(u64::from_be_bytes(buf))
}

fn read_i64_be(rdr: &mut Cursor<&[u8]>) -> Option<i64> {
    let mut buf = [0u8; 8];
    rdr.read_exact(&mut buf).ok()?;
    Some(i64::from_be_bytes(buf))
}

fn read_f64_be(rdr: &mut Cursor<&[u8]>) -> Option<f64> {
    read_u64_be(rdr).map(f64::from_bits)
}

/// Napis UTF-8 w formacie Qt: długość `quint32` i bajty; `0xffffffff` oznacza napis pusty (null).
fn read_utf8_string(rdr: &mut Cursor<&[u8]>) -> Option<String> {
    let len = read_u32_be(rdr)?;
    if len == 0xffff_ffff {
        return None;
    }
    let len = len as usize;
    if len > MAX_STRING_LEN {
        return None;
    }
    let mut buf = vec![0u8; len];
    rdr.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// Stan instancji WSJT-X (typ 1). Pola dopisane w nowszych wersjach mają wartości
/// domyślne, gdy krótsza ramka ich nie zawiera.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WsjtxStatus {
    pub id: String,
    pub dial_freq: u64,
    pub mode: String,
    pub dx_call: String,
    pub report: String,
    pub tx_mode: String,
    pub tx_enabled: bool,
    pub transmitting: bool,
    pub decoding: bool,
    pub de_call: String,
    pub de_grid: String,
    pub dx_grid: String,
}

impl WsjtxStatus {
    fn read(id: String, rdr: &mut Cursor<&[u8]>) -> Self {
        let mut status = Self {
            id,
            ..Self::default()
        };
        status.dial_freq = read_u64_be(rdr).unwrap_or(0);
        status.mode = read_utf8_string(rdr).unwrap_or_default();
        status.dx_call = read_utf8_string(rdr).unwrap_or_default();
        status.report = read_utf8_string(rdr).unwrap_or_default();
        status.tx_mode = read_utf8_string(rdr).unwrap_or_default();
        status.tx_enabled = read_bool(rdr).unwrap_or(false);
        status.transmitting = read_bool(rdr).unwrap_or(false);
        status.decoding = read_bool(rdr).unwrap_or(false);
        let _rx_df = read_u32_be(rdr);
        let _tx_df = read_u32_be(rdr);
        status.de_call = read_utf8_string(rdr).unwrap_or_default();
        status.de_grid = read_utf8_string(rdr).unwrap_or_default();
        status.dx_grid = read_utf8_string(rdr).unwrap_or_default();
        status
    }
}

/// Dekod nadany przez WSJT-X / JTDX (typ 2).
#[derive(Debug, Clone, PartialEq)]
pub struct WsjtxDecode {
    pub id: String,
    /// `false` dla dekodów powtórzonych po żądaniu Replay.
    pub is_new: bool,
    /// Czas UTC dekodu w milisekundach od północy.
    pub time_ms: u32,
    pub snr: i32,
    pub delta_time: f64,
    pub delta_freq: u32,
    /// Symbol emisji z WSJT-X (np. `~` FT8, `+` FT4) — odsyłany bez zmian w Reply.
    pub mode: String,
    pub message: String,
    pub low_confidence: bool,
    pub off_air: bool,
}

/// Znak bez nawiasów skrótu (`<SP6INA>` → `SP6INA`), jeśli wygląda na znak.
fn clean_call(token: &str) -> Option<&str> {
    let call = token.trim_matches(|c| c == '<' || c == '>');
    let has_digit = call.bytes().any(|b| b.is_ascii_digit());
    let has_letter = call.bytes().any(|b| b.is_ascii_alphabetic());
    (call.len() >= 3
        && has_digit
        && has_letter
        && call.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'/'))
    .then_some(call)
}

impl WsjtxDecode {
    fn read(id: String, rdr: &mut Cursor<&[u8]>) -> Option<Self> {
        Some(Self {
            id,
            is_new: read_bool(rdr)?,
            time_ms: read_u32_be(rdr)?,
            snr: read_i32_be(rdr)?,
            delta_time: read_f64_be(rdr)?,
            delta_freq: read_u32_be(rdr)?,
            mode: read_utf8_string(rdr).unwrap_or_default(),
            message: read_utf8_string(rdr)?,
            low_confidence: read_bool(rdr).unwrap_or(false),
            off_air: read_bool(rdr).unwrap_or(false),
        })
    }

    /// Znak stacji wołającej CQ/QRZ (`CQ K1ABC FN42`, `CQ DX K1ABC FN42`, `CQ POTA …`).
    pub fn cq_caller(&self) -> Option<&str> {
        let mut tokens = self.message.split_whitespace();
        let first = tokens.next()?;
        if !(first.eq_ignore_ascii_case("CQ") || first.eq_ignore_ascii_case("QRZ")) {
            return None;
        }
        let second = tokens.next()?;
        // Dopisek CQ: kierunek (DX, EU, NA), program (POTA, SOTA, TEST) albo częstotliwość.
        let is_modifier = second.len() <= 4
            && (second.bytes().all(|b| b.is_ascii_alphabetic())
                || second.bytes().all(|b| b.is_ascii_digit()));
        if is_modifier {
            tokens.next().and_then(clean_call)
        } else {
            clean_call(second)
        }
    }

    /// Znak nadawcy komunikatu: wołający CQ albo druga pozycja (`K1ABC SP6INA -10`).
    pub fn sender(&self) -> Option<&str> {
        if let Some(caller) = self.cq_caller() {
            return Some(caller);
        }
        self.message.split_whitespace().nth(1).and_then(clean_call)
    }

    /// Lokator w komunikacie (ostatni token w formacie Maidenhead 4-znakowym).
    pub fn grid(&self) -> Option<&str> {
        let last = self.message.split_whitespace().last()?;
        let b = last.as_bytes();
        (b.len() == 4
            && b[0].is_ascii_uppercase()
            && b[1].is_ascii_uppercase()
            && b[0] <= b'R'
            && b[1] <= b'R'
            && b[2].is_ascii_digit()
            && b[3].is_ascii_digit()
            && last != "RR73")
            .then_some(last)
    }

    /// Czas dekodu jako `HHMMSS` UTC.
    pub fn time_hhmmss(&self) -> String {
        let secs = self.time_ms / 1000;
        format!(
            "{:02}{:02}{:02}",
            (secs / 3600) % 24,
            (secs / 60) % 60,
            secs % 60
        )
    }
}

/// Pakiet zdekodowany z WSJT-X / JTDX.
#[derive(Debug, Clone)]
pub enum WsjtxMessage {
    Heartbeat {
        id: String,
    },
    Status(Box<WsjtxStatus>),
    Decode(WsjtxDecode),
    /// WSJT-X wyczyścił okno Band Activity.
    Clear {
        id: String,
    },
    QsoLogged {
        id: String,
        qso: Box<QsoRecord>,
    },
    /// Instancja WSJT-X kończy pracę.
    Close {
        id: String,
    },
}

impl WsjtxMessage {
    /// Identyfikator instancji (nazwa konfiguracji WSJT-X, np. `WSJT-X` albo `JTDX`).
    pub fn client_id(&self) -> &str {
        match self {
            Self::Heartbeat { id }
            | Self::Clear { id }
            | Self::Close { id }
            | Self::QsoLogged { id, .. } => id,
            Self::Status(status) => &status.id,
            Self::Decode(decode) => &decode.id,
        }
    }
}

/// Dekoduje strukturę QDateTime ze strumienia Qt QDataStream.
/// Zwraca `(YYYYMMDD, HHMMSS)` w UTC zgodnie ze standardem ADIF i bazą SQLite.
fn read_qdatetime(rdr: &mut Cursor<&[u8]>) -> Option<(String, String)> {
    let julian_day = read_i64_be(rdr)?;
    let ms_since_midnight = read_u32_be(rdr)?;
    let timespec = read_u8(rdr)?;
    let utc_offset_secs: i64 = match timespec {
        2 => i64::from(read_i32_be(rdr)?),
        3 => {
            // Qt::TimeZone — w strumieniu znajduje się QByteArray z identyfikatorem IANA
            let _ = read_utf8_string(rdr);
            0
        }
        _ => 0,
    };
    if julian_day <= 0 || ms_since_midnight >= 86_400_000 {
        return None;
    }

    // Julian Day Number (JDN) 2440588 to 1970-01-01 (Unix Epoch)
    let days_from_epoch = julian_day.checked_sub(2_440_588)?;
    let secs = days_from_epoch
        .checked_mul(86400)?
        .checked_add(i64::from(ms_since_midnight / 1000))?
        .checked_sub(utc_offset_secs)?;
    let dt = jiff::Timestamp::from_second(secs).ok()?;
    let date_str = dt.strftime("%Y%m%d").to_string();
    let time_str = dt.strftime("%H%M%S").to_string();
    Some((date_str, time_str))
}

fn freq_to_band(freq_hz: u64) -> String {
    crate::core::bandplan::get_band_by_freq(freq_hz)
        .map_or_else(|| "OTHER".to_string(), |b| b.name.to_string())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}

/// QSO Logged (typ 5). Raporty, których WSJT-X nie podał, zostają puste — program
/// niczego nie dopisuje za operatora.
fn read_qso_logged(rdr: &mut Cursor<&[u8]>) -> Option<QsoRecord> {
    let (date_off, time_off) = read_qdatetime(rdr)?;
    let dx_call = read_utf8_string(rdr)?;
    let dx_grid = read_utf8_string(rdr).unwrap_or_default();
    let dial_freq = read_u64_be(rdr).unwrap_or(0);
    let mode = non_empty(read_utf8_string(rdr))?;
    let rst_sent = read_utf8_string(rdr).unwrap_or_default();
    let rst_rcvd = read_utf8_string(rdr).unwrap_or_default();
    let _tx_power = read_utf8_string(rdr);
    let comments = read_utf8_string(rdr);
    let name = read_utf8_string(rdr);
    // Pola 11..17 specyfikacji (opcjonalne w krótszych ramkach starszych wersji)
    let dt_on = read_qdatetime(rdr);
    let _operator_call = read_utf8_string(rdr);
    let _my_call = read_utf8_string(rdr);
    let my_grid = read_utf8_string(rdr);
    let exch_sent = read_utf8_string(rdr);
    let exch_rcvd = read_utf8_string(rdr);
    let prop_mode = read_utf8_string(rdr);

    let (qso_date, time_on) = dt_on.unwrap_or_else(|| (date_off.clone(), time_off.clone()));

    let mut qso = QsoRecord::new(dx_call, freq_to_band(dial_freq), mode);
    qso.qso_date = qso_date;
    qso.time_on = time_on;
    qso.time_off = Some(time_off);
    qso.freq = (dial_freq > 0).then(|| dial_freq as f64 / 1_000_000.0);
    qso.rst_sent = rst_sent;
    qso.rst_rcvd = rst_rcvd;
    qso.gridsquare = non_empty(Some(dx_grid));
    qso.name = non_empty(name);
    qso.comment = non_empty(comments);
    qso.my_gridsquare = non_empty(my_grid);
    qso.stx_string = non_empty(exch_sent);
    qso.srx_string = non_empty(exch_rcvd);
    qso.prop_mode = non_empty(prop_mode);
    Some(qso)
}

/// Dekoduje binarny pakiet Qt QDataStream z WSJT-X. Nieznane typy i pakiety
/// wejściowe innych programów są pomijane.
pub fn parse_packet(data: &[u8]) -> Option<WsjtxMessage> {
    let mut rdr = Cursor::new(data);
    if read_u32_be(&mut rdr)? != WSJTX_MAGIC {
        return None;
    }
    let _schema = read_u32_be(&mut rdr)?;
    let packet_type = read_u32_be(&mut rdr)?;
    let id = read_utf8_string(&mut rdr)?;

    match packet_type {
        0 => Some(WsjtxMessage::Heartbeat { id }),
        1 => Some(WsjtxMessage::Status(Box::new(WsjtxStatus::read(
            id, &mut rdr,
        )))),
        2 => WsjtxDecode::read(id, &mut rdr).map(WsjtxMessage::Decode),
        3 => Some(WsjtxMessage::Clear { id }),
        5 => read_qso_logged(&mut rdr).map(|qso| WsjtxMessage::QsoLogged {
            id,
            qso: Box::new(qso),
        }),
        6 => Some(WsjtxMessage::Close { id }),
        12 => {
            // Logged ADIF (WSJT-X 2.0+)
            let adif_text = read_utf8_string(&mut rdr)?;
            crate::core::adif::AdifEngine::parse_reader(Cursor::new(adif_text))
                .into_iter()
                .next()
                .map(|qso| WsjtxMessage::QsoLogged {
                    id,
                    qso: Box::new(qso),
                })
        }
        _ => None,
    }
}

/// WSJT-X wysyła jedną łączność dwa razy: jako QSO Logged (typ 5) i Logged ADIF (typ 12).
/// Drugi pakiet z tym samym znakiem, pasmem, emisją i czasem w ciągu kilku sekund jest pomijany.
#[derive(Debug, Default)]
pub struct LoggedDedup {
    last: Option<(String, Instant)>,
}

impl LoggedDedup {
    const WINDOW: Duration = Duration::from_secs(5);

    fn signature(qso: &QsoRecord) -> String {
        let digits = |s: &str, n: usize| -> String {
            s.chars().filter(char::is_ascii_digit).take(n).collect()
        };
        format!(
            "{}|{}|{}|{}|{}",
            qso.callsign.trim().to_uppercase(),
            qso.band.trim().to_lowercase(),
            qso.mode.trim().to_uppercase(),
            digits(&qso.qso_date, 8),
            digits(&qso.time_on, 4)
        )
    }

    /// `true`, gdy pakiet powtarza właśnie odebrane QSO.
    pub fn is_repeat(&mut self, msg: &WsjtxMessage, now: Instant) -> bool {
        let WsjtxMessage::QsoLogged { qso, .. } = msg else {
            return false;
        };
        let signature = Self::signature(qso);
        if let Some((previous, at)) = &self.last
            && *previous == signature
            && now.saturating_duration_since(*at) < Self::WINDOW
        {
            log::debug!("WSJT-X: skipped repeated QSO packet ({signature})");
            return true;
        }
        self.last = Some((signature, now));
        false
    }
}

/// Kolor RGB dla polecenia HighlightCallsign.
pub type Rgb = (u8, u8, u8);

/// Zapis pól w formacie Qt QDataStream (big-endian), jak w `NetworkMessage.hpp` WSJT-X.
struct QtWriter(Vec<u8>);

impl QtWriter {
    /// Nagłówek: magic, schemat, typ, identyfikator klienta.
    fn header(packet_type: u32, id: &str) -> Self {
        let mut w = Self(Vec::with_capacity(128));
        w.u32(WSJTX_MAGIC);
        w.u32(WSJTX_SCHEMA);
        w.u32(packet_type);
        w.utf8(id);
        w
    }

    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn bool(&mut self, v: bool) {
        self.0.push(u8::from(v));
    }

    fn utf8(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }

    /// QColor w strumieniu Qt 5: spec (1 = RGB, 0 = niepoprawny), alfa, R, G, B, wypełnienie;
    /// składowe 16-bitowe (wartość 8-bitowa × 257). Kolor niepoprawny oznacza brak koloru.
    fn color(&mut self, color: Option<Rgb>) {
        let (spec, alpha, (r, g, b)) = match color {
            Some(rgb) => (1, 0xffff, rgb),
            None => (0, 0xffff, (0, 0, 0)),
        };
        self.u8(spec);
        self.u16(alpha);
        self.u16(u16::from(r) * 257);
        self.u16(u16::from(g) * 257);
        self.u16(u16::from(b) * 257);
        self.u16(0);
    }
}

/// Reply (typ 4): prośba o odpowiedź na dekod — jak dwuklik w oknie Band Activity.
/// WSJT-X honoruje ją tylko dla dekodu CQ/QRZ albo skierowanego do operatora.
pub fn encode_reply(decode: &WsjtxDecode, modifiers: u8) -> Vec<u8> {
    let mut w = QtWriter::header(4, &decode.id);
    w.u32(decode.time_ms);
    w.i32(decode.snr);
    w.f64(decode.delta_time);
    w.u32(decode.delta_freq);
    w.utf8(&decode.mode);
    w.utf8(&decode.message);
    w.bool(decode.low_confidence);
    w.u8(modifiers);
    w.0
}

/// HighlightCallsign (typ 13): kolory znaku w oknie Band Activity. Brak obu kolorów
/// usuwa wyróżnienie znaku; znak `CLEARALL!` usuwa wszystkie wyróżnienia.
pub fn encode_highlight(
    id: &str,
    callsign: &str,
    background: Option<Rgb>,
    foreground: Option<Rgb>,
    highlight_last: bool,
) -> Vec<u8> {
    let mut w = QtWriter::header(13, id);
    w.utf8(callsign);
    w.color(background);
    w.color(foreground);
    w.bool(highlight_last);
    w.0
}

/// Usuwa wszystkie wyróżnienia ustawione przez ten program.
pub fn encode_clear_highlights(id: &str) -> Vec<u8> {
    encode_highlight(id, "CLEARALL!", None, None, false)
}

/// Najwięcej znaków wyróżnionych naraz w jednej instancji — WSJT-X przechowuje każde
/// wyróżnienie, więc po przekroczeniu limitu lista jest czyszczona i budowana od nowa.
pub const MAX_HIGHLIGHTS: usize = 100;

/// Znaczenie stacji dla operatora, sygnalizowane kolorem w oknie Band Activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsjtxMark {
    NewDxcc,
    NewBand,
    WorkedBefore,
    Other,
}

impl WsjtxMark {
    /// Nazwa oznaczenia przekazywana pluginom (`on_wsjtx_decode`).
    pub const fn id(self) -> &'static str {
        match self {
            WsjtxMark::NewDxcc => "new_dxcc",
            WsjtxMark::NewBand => "new_band",
            WsjtxMark::WorkedBefore => "worked",
            WsjtxMark::Other => "other",
        }
    }

    /// Kolory (tło, tekst) polecenia HighlightCallsign; `None` oznacza brak wyróżnienia.
    pub fn colors(self) -> Option<(Option<Rgb>, Option<Rgb>)> {
        match self {
            Self::NewDxcc => Some((Some((220, 40, 40)), Some((255, 255, 255)))),
            Self::NewBand => Some((Some((255, 165, 0)), Some((0, 0, 0)))),
            Self::WorkedBefore => Some((None, Some((128, 128, 128)))),
            Self::Other => None,
        }
    }
}

/// Ustawienia połączenia z WSJT-X / JTDX (zapisywane w konfiguracji).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WsjtxSettings {
    pub enabled: bool,
    /// Port UDP, na który WSJT-X wysyła pakiety (Settings → Reporting → UDP Server).
    pub port: u16,
    /// Adres grupy multicast (np. `224.0.0.1`), gdy kilka programów słucha jednocześnie;
    /// pusty oznacza zwykły unicast.
    pub multicast_group: String,
    /// Kolorowanie znaków w oknie Band Activity programu WSJT-X.
    pub highlight: bool,
}

impl Default for WsjtxSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: DEFAULT_WSJTX_PORT,
            multicast_group: String::new(),
            highlight: true,
        }
    }
}

/// Błąd uruchomienia nasłuchu WSJT-X.
#[derive(Debug)]
pub enum WsjtxLinkError {
    /// Adres grupy multicast jest niepoprawny albo spoza zakresu 224.0.0.0/4.
    InvalidGroup(String),
    Bind {
        port: u16,
        source: std::io::Error,
    },
    Join {
        group: Ipv4Addr,
        source: std::io::Error,
    },
}

impl std::fmt::Display for WsjtxLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGroup(group) => write!(f, "invalid multicast group: {group}"),
            Self::Bind { port, source } => write!(f, "UDP port {port}: {source}"),
            Self::Join { group, source } => write!(f, "multicast group {group}: {source}"),
        }
    }
}

impl std::error::Error for WsjtxLinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidGroup(_) => None,
            Self::Bind { source, .. } | Self::Join { source, .. } => Some(source),
        }
    }
}

/// Grupa multicast z ustawień: `None` dla pustego pola, błąd dla adresu spoza 224.0.0.0/4.
fn multicast_group(settings: &WsjtxSettings) -> Result<Option<Ipv4Addr>, WsjtxLinkError> {
    let group = settings.multicast_group.trim();
    if group.is_empty() {
        return Ok(None);
    }
    match group.parse::<Ipv4Addr>() {
        Ok(addr) if addr.is_multicast() => Ok(Some(addr)),
        _ => Err(WsjtxLinkError::InvalidGroup(group.to_string())),
    }
}

/// Gniazdo multicast współdzielone z innymi programami (np. GridTracker, JTAlert):
/// `SO_REUSEADDR` pozwala kilku odbiorcom na jednym komputerze zająć ten sam port
/// i odbierać tę samą grupę.
fn shared_socket(addr: SocketAddr) -> std::io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket.into())
}

/// Gniazdo UDP do odbioru pakietów i odsyłania poleceń do nadawców. Port unicast jest
/// zajmowany na wyłączność — pakiety skierowane do jednego odbiorcy nie mogą trafić
/// do innego programu.
pub async fn bind_socket(settings: &WsjtxSettings) -> Result<UdpSocket, WsjtxLinkError> {
    let group = multicast_group(settings)?;
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, settings.port));
    let bind_error = |source| WsjtxLinkError::Bind {
        port: settings.port,
        source,
    };
    let Some(group) = group else {
        return UdpSocket::bind(addr).await.map_err(bind_error);
    };
    let socket = shared_socket(addr).map_err(bind_error)?;
    socket
        .join_multicast_v4(group, Ipv4Addr::UNSPECIFIED)
        .map_err(|source| WsjtxLinkError::Join { group, source })?;
    Ok(socket)
}

/// Pakiet do wysłania do konkretnej instancji WSJT-X (po jej identyfikatorze).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WsjtxOutgoing {
    pub client_id: String,
    pub packet: Vec<u8>,
}

/// Odbiera pakiety WSJT-X i wysyła polecenia na adres, z którego przyszedł ostatni pakiet
/// danej instancji. Kończy się, gdy konsument zamknie kanał odbiorczy albo nadawczy.
pub async fn run_link(
    socket: UdpSocket,
    sender: mpsc::Sender<WsjtxMessage>,
    mut outgoing: mpsc::UnboundedReceiver<WsjtxOutgoing>,
) {
    let mut buf = vec![0u8; 8192];
    let mut peers: HashMap<String, SocketAddr> = HashMap::new();
    let mut dedup = LoggedDedup::default();
    loop {
        tokio::select! {
            received = socket.recv_from(&mut buf) => {
                let (len, from) = match received {
                    Ok(v) => v,
                    Err(e) => {
                        // Windows zgłasza ICMP "port unreachable" po wysłaniu na zamknięty
                        // port jako błąd odbioru — to nie kończy nasłuchu.
                        log::debug!("WSJT-X: UDP receive error: {e}");
                        continue;
                    }
                };
                let Some(msg) = parse_packet(&buf[..len]) else {
                    continue;
                };
                peers.insert(msg.client_id().to_string(), from);
                if dedup.is_repeat(&msg, Instant::now()) {
                    continue;
                }
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
            packet = outgoing.recv() => {
                let Some(packet) = packet else { break };
                match peers.get(&packet.client_id) {
                    Some(addr) => {
                        if let Err(e) = socket.send_to(&packet.packet, addr).await {
                            log::warn!("WSJT-X: cannot send to {addr}: {e}");
                        }
                    }
                    None => log::debug!("WSJT-X: no address for client {}", packet.client_id),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(packet_type: u32, id: &str) -> QtWriter {
        QtWriter::header(packet_type, id)
    }

    fn logged_packet(rst: bool) -> Vec<u8> {
        let mut w = header(5, "WSJT-X");
        // QDateTime Off: QDate (8 bajtów) + QTime (4 bajty) + timespec (1 bajt)
        w.0.extend_from_slice(&2_460_000u64.to_be_bytes()); // 2023-02-24
        w.u32(43_260_000); // 12:01:00.000
        w.u8(1); // UTC
        w.utf8("K1ABC");
        w.utf8("FN31pr");
        w.0.extend_from_slice(&14_074_000u64.to_be_bytes());
        w.utf8("FT8");
        if !rst {
            return w.0;
        }
        w.utf8("-05");
        w.utf8("-12");
        w.utf8("50");
        w.utf8("73 TU");
        w.utf8("John");
        // QDateTime On (pole 11): 12:00:00 UTC
        w.0.extend_from_slice(&2_460_000u64.to_be_bytes());
        w.u32(43_200_000);
        w.u8(1);
        for field in ["SP6INA", "SP6INA", "JO81WA", "599 15", "599 05", "F2"] {
            w.utf8(field);
        }
        w.0
    }

    fn decode(message: &str) -> WsjtxDecode {
        WsjtxDecode {
            id: "WSJT-X".to_string(),
            is_new: true,
            time_ms: 45_015_000,
            snr: -12,
            delta_time: 0.2,
            delta_freq: 1234,
            mode: "~".to_string(),
            message: message.to_string(),
            low_confidence: false,
            off_air: false,
        }
    }

    fn decode_packet(d: &WsjtxDecode) -> Vec<u8> {
        let mut w = header(2, &d.id);
        w.bool(d.is_new);
        w.u32(d.time_ms);
        w.i32(d.snr);
        w.f64(d.delta_time);
        w.u32(d.delta_freq);
        w.utf8(&d.mode);
        w.utf8(&d.message);
        w.bool(d.low_confidence);
        w.bool(d.off_air);
        w.0
    }

    #[test]
    fn qso_logged_packet_keeps_every_field() {
        let Some(WsjtxMessage::QsoLogged { id, qso }) = parse_packet(&logged_packet(true)) else {
            panic!("expected QsoLogged");
        };
        assert_eq!(id, "WSJT-X");
        assert_eq!(qso.callsign, "K1ABC");
        assert_eq!(qso.gridsquare.as_deref(), Some("FN31pr"));
        assert_eq!(qso.band, "20m");
        assert_eq!(qso.mode, "FT8");
        assert_eq!(qso.freq, Some(14.074));
        assert_eq!(qso.rst_sent, "-05");
        assert_eq!(qso.rst_rcvd, "-12");
        assert_eq!(qso.qso_date, "20230224");
        assert_eq!(qso.time_on, "120000");
        assert_eq!(qso.time_off.as_deref(), Some("120100"));
        assert_eq!(qso.name.as_deref(), Some("John"));
        assert_eq!(qso.comment.as_deref(), Some("73 TU"));
        assert_eq!(qso.my_gridsquare.as_deref(), Some("JO81WA"));
        assert_eq!(qso.stx_string.as_deref(), Some("599 15"));
        assert_eq!(qso.srx_string.as_deref(), Some("599 05"));
        assert_eq!(qso.prop_mode.as_deref(), Some("F2"));
    }

    #[test]
    fn missing_reports_stay_empty_instead_of_being_invented() {
        let Some(WsjtxMessage::QsoLogged { qso, .. }) = parse_packet(&logged_packet(false)) else {
            panic!("expected QsoLogged");
        };
        assert_eq!(qso.rst_sent, "");
        assert_eq!(qso.rst_rcvd, "");
        assert_eq!(qso.time_on, "120100");
    }

    #[test]
    fn qso_logged_without_mode_is_rejected() {
        let mut w = header(5, "WSJT-X");
        w.0.extend_from_slice(&2_460_000u64.to_be_bytes());
        w.u32(43_260_000);
        w.u8(1);
        w.utf8("K1ABC");
        w.utf8("");
        w.0.extend_from_slice(&14_074_000u64.to_be_bytes());
        w.utf8("");
        assert!(parse_packet(&w.0).is_none());
    }

    #[test]
    fn decode_packet_round_trips() {
        let original = decode("CQ DX K1ABC FN42");
        let Some(WsjtxMessage::Decode(parsed)) = parse_packet(&decode_packet(&original)) else {
            panic!("expected Decode");
        };
        assert_eq!(parsed, original);
        assert_eq!(parsed.time_hhmmss(), "123015");
    }

    #[test]
    fn status_packet_reads_the_extended_fields() {
        let mut w = header(1, "JTDX");
        w.0.extend_from_slice(&7_074_000u64.to_be_bytes());
        w.utf8("FT8");
        w.utf8("K1ABC");
        w.utf8("-07");
        w.utf8("FT8");
        w.bool(true);
        w.bool(true);
        w.bool(false);
        w.u32(1500);
        w.u32(1200);
        w.utf8("SP6INA");
        w.utf8("JO81");
        w.utf8("FN42");
        let Some(WsjtxMessage::Status(status)) = parse_packet(&w.0) else {
            panic!("expected Status");
        };
        assert_eq!(status.id, "JTDX");
        assert_eq!(status.dial_freq, 7_074_000);
        assert_eq!(status.dx_call, "K1ABC");
        assert!(status.tx_enabled && status.transmitting && !status.decoding);
        assert_eq!(status.de_call, "SP6INA");
        assert_eq!(status.de_grid, "JO81");
        assert_eq!(status.dx_grid, "FN42");
    }

    #[test]
    fn short_status_packet_from_older_versions_still_parses() {
        let mut w = header(1, "WSJT-X");
        w.0.extend_from_slice(&14_074_000u64.to_be_bytes());
        w.utf8("FT8");
        let Some(WsjtxMessage::Status(status)) = parse_packet(&w.0) else {
            panic!("expected Status");
        };
        assert_eq!(status.mode, "FT8");
        assert!(status.dx_call.is_empty() && !status.transmitting);
    }

    #[test]
    fn clear_close_and_unknown_packets() {
        assert!(matches!(
            parse_packet(&header(3, "A").0),
            Some(WsjtxMessage::Clear { id }) if id == "A"
        ));
        assert!(matches!(
            parse_packet(&header(6, "A").0),
            Some(WsjtxMessage::Close { id }) if id == "A"
        ));
        assert!(parse_packet(&header(9, "A").0).is_none());
        let mut bad = header(0, "A").0;
        bad[0] = 0;
        assert!(parse_packet(&bad).is_none());
    }

    #[test]
    fn oversized_string_is_rejected() {
        let mut w = header(2, "A");
        w.bool(true);
        w.u32(0);
        w.i32(0);
        w.f64(0.0);
        w.u32(0);
        w.utf8("~");
        w.u32(1_000_000);
        assert!(parse_packet(&w.0).is_none());
    }

    #[test]
    fn cq_caller_handles_modifiers_and_hashed_calls() {
        assert_eq!(decode("CQ K1ABC FN42").cq_caller(), Some("K1ABC"));
        assert_eq!(decode("CQ DX K1ABC FN42").cq_caller(), Some("K1ABC"));
        assert_eq!(decode("CQ POTA W1AW/P FN31").cq_caller(), Some("W1AW/P"));
        assert_eq!(decode("CQ 290 VK2XYZ QF56").cq_caller(), Some("VK2XYZ"));
        assert_eq!(decode("QRZ <PJ4/K1ABC>").cq_caller(), Some("PJ4/K1ABC"));
        assert_eq!(decode("SP6INA K1ABC -10").cq_caller(), None);
        assert_eq!(decode("SP6INA K1ABC -10").sender(), Some("K1ABC"));
        assert_eq!(decode("CQ").cq_caller(), None);
    }

    #[test]
    fn grid_is_only_a_real_maidenhead_square() {
        assert_eq!(decode("CQ K1ABC FN42").grid(), Some("FN42"));
        assert_eq!(decode("K1ABC SP6INA RR73").grid(), None);
        assert_eq!(decode("K1ABC SP6INA JO81").grid(), Some("JO81"));
        assert_eq!(decode("K1ABC SP6INA ZZ99").grid(), None);
        assert_eq!(decode("K1ABC SP6INA -10").grid(), None);
    }

    #[test]
    fn reply_packet_matches_the_wsjtx_layout() {
        let d = decode("CQ K1ABC FN42");
        let packet = encode_reply(&d, 0x02);
        let mut expected = Vec::new();
        expected.extend_from_slice(&WSJTX_MAGIC.to_be_bytes());
        expected.extend_from_slice(&2u32.to_be_bytes());
        expected.extend_from_slice(&4u32.to_be_bytes());
        expected.extend_from_slice(&6u32.to_be_bytes());
        expected.extend_from_slice(b"WSJT-X");
        expected.extend_from_slice(&45_015_000u32.to_be_bytes());
        expected.extend_from_slice(&(-12i32).to_be_bytes());
        expected.extend_from_slice(&0.2f64.to_be_bytes());
        expected.extend_from_slice(&1234u32.to_be_bytes());
        expected.extend_from_slice(&1u32.to_be_bytes());
        expected.extend_from_slice(b"~");
        expected.extend_from_slice(&13u32.to_be_bytes());
        expected.extend_from_slice(b"CQ K1ABC FN42");
        expected.push(0);
        expected.push(0x02);
        assert_eq!(packet, expected);
    }

    #[test]
    fn highlight_packet_encodes_qt5_colors() {
        let packet = encode_highlight("A", "K1ABC", Some((255, 0, 128)), None, true);
        let tail = &packet[packet.len() - 23..];
        assert_eq!(
            tail,
            [
                1, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x80, 0x80, 0, 0, // tło RGB
                0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, // brak koloru tekstu
                1, // highlight last
            ]
        );
        let clear = encode_clear_highlights("A");
        assert!(clear.windows(9).any(|w| w == b"CLEARALL!"));
    }

    #[test]
    fn logged_dedup_skips_the_adif_twin_only_within_the_window() {
        let Some(msg) = parse_packet(&logged_packet(true)) else {
            panic!("expected QsoLogged");
        };
        let mut dedup = LoggedDedup::default();
        let start = Instant::now();
        assert!(!dedup.is_repeat(&msg, start));
        assert!(dedup.is_repeat(&msg, start + Duration::from_secs(1)));
        assert!(!dedup.is_repeat(&msg, start + Duration::from_secs(10)));
        let heartbeat = WsjtxMessage::Heartbeat { id: "A".into() };
        assert!(!dedup.is_repeat(&heartbeat, start));
    }

    #[test]
    fn multicast_group_is_validated() {
        let mut settings = WsjtxSettings::default();
        assert!(matches!(multicast_group(&settings), Ok(None)));
        settings.multicast_group = " 224.0.0.1 ".into();
        assert_eq!(
            multicast_group(&settings).ok().flatten(),
            Some(Ipv4Addr::new(224, 0, 0, 1))
        );
        for bad in ["192.168.1.10", "nonsense", "300.0.0.1"] {
            settings.multicast_group = bad.into();
            assert!(matches!(
                multicast_group(&settings),
                Err(WsjtxLinkError::InvalidGroup(_))
            ));
        }
    }

    #[tokio::test]
    async fn multicast_receivers_share_the_port_but_unicast_keeps_it() {
        let first = shared_socket(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))).unwrap();
        let port = first.local_addr().unwrap().port();
        let second = shared_socket(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)));
        assert!(
            second.is_ok(),
            "a second multicast receiver can bind the same port"
        );

        let unicast = WsjtxSettings {
            port: UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
                .await
                .unwrap()
                .local_addr()
                .unwrap()
                .port(),
            ..WsjtxSettings::default()
        };
        let owner = bind_socket(&unicast).await.unwrap();
        assert!(matches!(
            bind_socket(&unicast).await,
            Err(WsjtxLinkError::Bind { .. })
        ));
        drop(owner);
    }

    /// Wirtualny WSJT-X na pętli zwrotnej: dekod trafia do programu, a Reply wraca
    /// na adres nadawcy.
    #[tokio::test]
    async fn link_forwards_decodes_and_replies_to_the_sending_instance() {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let link_addr = socket.local_addr().unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let link = tokio::spawn(run_link(socket, tx, out_rx));

        let peer = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let cq = decode("CQ K1ABC FN42");
        peer.send_to(&decode_packet(&cq), link_addr).await.unwrap();
        let received = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap();
        let Some(WsjtxMessage::Decode(received)) = received else {
            panic!("expected Decode");
        };
        assert_eq!(received, cq);

        out_tx
            .send(WsjtxOutgoing {
                client_id: "WSJT-X".into(),
                packet: encode_reply(&received, 0),
            })
            .unwrap();
        let mut buf = [0u8; 512];
        let (len, from) = tokio::time::timeout(Duration::from_secs(5), peer.recv_from(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(from, link_addr);
        assert_eq!(&buf[..len], encode_reply(&cq, 0).as_slice());

        drop(out_tx);
        tokio::time::timeout(Duration::from_secs(5), link)
            .await
            .unwrap()
            .unwrap();
    }
}
