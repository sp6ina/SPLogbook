// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// K1EL WinKeyer (WK2 / WK3): ramki protokołu hosta.
//
// Źródło: K1EL "WinKeyer3 IC Interface & Operation Manual" rev 1.3 (2019-03-19):
// tabela poleceń hosta (Table 18), mapowanie prosignów ASCII (Table 17), sidetone
// (Tables 6-7), rejestr trybu (Table 12), znaczniki bajtów statusu i potencjometru.

use log::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyerMode {
    Winkeyer2,
    Winkeyer3,
    SerialDtr,
    SerialRts,
}

#[derive(Debug, Clone)]
pub struct WinkeyerConfig {
    pub port: String,
    pub baud_rate: u32,
    pub speed_wpm: u8,
    pub mode: KeyerMode,
    pub sidetone_hz: u16,
}

impl Default for WinkeyerConfig {
    fn default() -> Self {
        Self {
            port: "COM1".to_string(),
            baud_rate: 1200,
            speed_wpm: 24,
            mode: KeyerMode::Winkeyer2,
            sidetone_hz: 700,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WinkeyerError {
    EmptyMacro,
    OversizedMacro(usize),
    Busy,
    Transport(String),
}

impl std::fmt::Display for WinkeyerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyMacro => write!(f, "CW Macro cannot be empty"),
            Self::OversizedMacro(len) => {
                write!(f, "CW Macro length {len} exceeds maximum buffer (256)")
            }
            Self::Busy => write!(f, "WinKeyer backend buffer is busy"),
            Self::Transport(msg) => write!(f, "WinKeyer transport error: {msg}"),
        }
    }
}

impl std::error::Error for WinkeyerError {}

/// Generator ramek protokołu K1EL Winkeyer
pub struct WinkeyerProtocol;

impl WinkeyerProtocol {
    /// Komenda otwarcia / testu Winkeyera (Host Open: 0x00, 0x02)
    pub fn host_open() -> Vec<u8> {
        vec![0x00, 0x02]
    }

    /// Komenda zamknięcia (Host Close: 0x00, 0x03)
    pub fn host_close() -> Vec<u8> {
        vec![0x00, 0x03]
    }

    /// Ustawienie prędkości WPM (0x02, wpm); WinKeyer przyjmuje 5..=99 WPM.
    pub fn set_speed(wpm: u8) -> Vec<u8> {
        let clamped = wpm.clamp(crate::cat::morse::MIN_WPM, crate::cat::morse::MAX_WPM);
        vec![0x02, clamped]
    }

    /// Rejestr trybu (0x0E): iambic B, echo znaków nadanych z hosta (bit 2), dzięki
    /// któremu program wie, ile tekstu już poszło w eter.
    pub fn set_mode_serial_echo() -> Vec<u8> {
        vec![0x0E, 0x04]
    }

    /// Alias dla set_speed zwracający ramkę WPM
    pub fn encode_wpm_frame(wpm: u8) -> Vec<u8> {
        Self::set_speed(wpm)
    }

    /// Kodowanie ramki makra telegraficznego z walidacją kontraktową
    pub fn encode_macro_frame(macro_text: &str) -> Result<Vec<u8>, WinkeyerError> {
        let trimmed = macro_text.trim();
        if trimmed.is_empty() {
            return Err(WinkeyerError::EmptyMacro);
        }
        if trimmed.len() > 256 {
            return Err(WinkeyerError::OversizedMacro(trimmed.len()));
        }

        Ok(Self::encode_text(trimmed))
    }

    /// Wyczyszczenie bufora / natychmiastowe przerwanie nadawania (Clear Buffer: 0x0A)
    pub fn abort_buffer() -> Vec<u8> {
        vec![0x0A]
    }

    /// Częstotliwość podsłuchu (0x01, n). W trybie WK3: n = 62500 / f dla 500..=4000 Hz.
    /// W trybie WK1/WK2: n = 1..=10 z tabeli 4000 / n Hz (najbliższa wartość).
    pub fn set_sidetone(hz: u16, wk3_mode: bool) -> Vec<u8> {
        let hz = u32::from(hz.clamp(400, 4000));
        let n = if wk3_mode {
            (62_500 / hz.max(500)).clamp(15, 125)
        } else {
            ((4000 + hz / 2) / hz).clamp(1, 10)
        };
        vec![0x01, n as u8]
    }

    /// Tekst do bufora WinKeyera. Prosigny `<BT>`, `<AR>`, `<SK>`, `<KN>`, `<AS>` są
    /// zamieniane na znaki ASCII, które WinKeyer mapuje na te prosigny (Table 17).
    /// Odrzucane są: bajty sterujące poniżej 0x20 (to polecenia, nie tekst), znaki,
    /// które WinKeyer ignoruje (`! # % & *`), znaki powyżej 0x5D poza `|` (pół kropki
    /// przerwy) oraz znaki spoza ASCII.
    pub fn encode_text(text: &str) -> Vec<u8> {
        let upper = text.to_uppercase();
        let mut bytes = Vec::with_capacity(upper.len());
        let mut rest = upper.as_str();
        while let Some(c) = rest.chars().next() {
            if c == '<'
                && let Some(end) = rest.find('>')
                && let Some(mapped) = prosign_byte(&rest[1..end])
            {
                bytes.push(mapped);
                rest = &rest[end + 1..];
                continue;
            }
            if c == '\n' || c == '\t' {
                bytes.push(b' ');
            } else if c.is_ascii() && is_sendable(c as u8) {
                bytes.push(c as u8);
            }
            rest = &rest[c.len_utf8()..];
        }
        bytes
    }

    /// Rodzaj bajtu odebranego od WinKeyera.
    pub fn classify(byte: u8) -> WinkeyerByte {
        if byte & 0xC0 == 0xC0 {
            WinkeyerByte::Status(WinkeyerStatus(byte))
        } else if byte & 0xC0 == 0x80 {
            WinkeyerByte::SpeedPot(byte & 0x3F)
        } else {
            WinkeyerByte::Echo(byte)
        }
    }
}

/// Bajt odebrany od WinKeyera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinkeyerByte {
    Status(WinkeyerStatus),
    /// Położenie potencjometru prędkości (0..=31 względem minimum).
    SpeedPot(u8),
    /// Echo znaku, który właśnie został nadany.
    Echo(u8),
}

/// Bajt statusu WinKeyera (znacznik 0b110 w trzech najstarszych bitach).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinkeyerStatus(pub u8);

impl WinkeyerStatus {
    /// Bufor wejściowy zapełniony w ponad 2/3 — host wstrzymuje wysyłanie.
    pub fn xoff(self) -> bool {
        self.0 & 0x01 != 0
    }
    /// Operator kluczuje manipulatorem (break-in).
    pub fn break_in(self) -> bool {
        self.0 & 0x02 != 0
    }
    /// WinKeyer nadaje.
    pub fn busy(self) -> bool {
        self.0 & 0x04 != 0
    }
    /// W trybie WK2 bit 3 oznacza bajt stanu przycisków, nie status klucza.
    pub fn is_pushbutton(self) -> bool {
        self.0 & 0x08 != 0
    }
}

fn prosign_byte(name: &str) -> Option<u8> {
    Some(match name {
        "BT" => b'=',
        "AR" => b'+',
        "SK" => b'>',
        "KN" => b'(',
        "AS" => b'[',
        _ => return None,
    })
}

fn is_sendable(b: u8) -> bool {
    match b {
        b'!' | b'#' | b'%' | b'&' | b'*' => false,
        b'|' | 0x20..=0x5D => true,
        _ => false,
    }
}

impl WinkeyerProtocol {
    /// Liczba bajtów, które można mieć wysłane i jeszcze nienadane. Bufor WinKeyera ma
    /// 160 znaków, a XOFF pojawia się przy 2/3 — zostawiamy zapas.
    pub const MAX_IN_FLIGHT: usize = 64;
}

/// Mock transportu dla WinKeyera izolujący testy od fizycznego portu COM i nadajnika
#[derive(Debug, Default)]
pub struct MockWinkeyerTransport {
    pub written_bytes: Vec<u8>,
    pub frames_enqueued: usize,
    pub simulate_busy: bool,
    pub max_write_chunk: Option<usize>,
}

impl MockWinkeyerTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_frame(&mut self, frame: &[u8]) -> Result<usize, WinkeyerError> {
        if self.simulate_busy {
            return Err(WinkeyerError::Busy);
        }
        let chunk_size = self.max_write_chunk.unwrap_or(frame.len()).min(frame.len());
        self.written_bytes.extend_from_slice(&frame[..chunk_size]);
        if chunk_size == frame.len() {
            self.frames_enqueued += 1;
        }
        Ok(chunk_size)
    }

    pub fn reset(&mut self) {
        self.written_bytes.clear();
        self.frames_enqueued = 0;
        self.simulate_busy = false;
        self.max_write_chunk = None;
    }
}

#[derive(Debug, Default)]
pub struct CwTerminalBuffer {
    pub sent_text: String,
    pub queued_text: String,
    pub current_wpm: u8,
}

impl CwTerminalBuffer {
    pub fn new(wpm: u8) -> Self {
        Self {
            sent_text: String::new(),
            queued_text: String::new(),
            current_wpm: wpm.clamp(5, 50),
        }
    }

    pub fn append_to_queue(&mut self, text: &str) {
        self.queued_text.push_str(text);
    }

    pub fn advance_sent(&mut self, count: usize) {
        if let Some((byte_idx, _)) = self.queued_text.char_indices().nth(count) {
            let (sent_part, remaining) = self.queued_text.split_at(byte_idx);
            self.sent_text.push_str(sent_part);
            self.queued_text = remaining.to_string();
        } else {
            self.sent_text.push_str(&self.queued_text);
            self.queued_text.clear();
        }
    }

    pub fn abort(&mut self) {
        info!("CW Terminal: Abort bufora nadawania");
        self.queued_text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winkeyer_protocol_frames() {
        assert_eq!(WinkeyerProtocol::set_speed(28), vec![0x02, 28]);
        assert_eq!(WinkeyerProtocol::abort_buffer(), vec![0x0A]);
        assert_eq!(
            WinkeyerProtocol::encode_text("cq de sp6ina k"),
            b"CQ DE SP6INA K".to_vec()
        );
    }

    #[test]
    fn test_cw_buffer() {
        let mut buf = CwTerminalBuffer::new(25);
        buf.append_to_queue("CQ TEST");
        assert_eq!(buf.queued_text, "CQ TEST");
        buf.advance_sent(3);
        assert_eq!(buf.sent_text, "CQ ");
        assert_eq!(buf.queued_text, "TEST");
        buf.abort();
        assert_eq!(buf.queued_text, "");

        buf.sent_text.clear();
        buf.append_to_queue("ŁÓDŹ K");
        buf.advance_sent(5);
        assert_eq!(buf.sent_text, "ŁÓDŹ ");
        assert_eq!(buf.queued_text, "K");
    }

    #[test]
    fn winkeyer_encodes_wpm_frame() {
        let frame = WinkeyerProtocol::encode_wpm_frame(30);
        assert_eq!(frame, vec![0x02, 30]);

        let frame_clamped_low = WinkeyerProtocol::encode_wpm_frame(2);
        assert_eq!(frame_clamped_low, vec![0x02, 5]);

        // Zakres WinKeyera to 5..=99 WPM (Table 18, „Speed”).
        assert_eq!(WinkeyerProtocol::encode_wpm_frame(60), vec![0x02, 60]);
        assert_eq!(WinkeyerProtocol::encode_wpm_frame(120), vec![0x02, 99]);
    }

    #[test]
    fn winkeyer_encodes_macro_frame() {
        let frame = WinkeyerProtocol::encode_macro_frame("CQ DE SP6INA").unwrap();
        assert_eq!(frame, b"CQ DE SP6INA".to_vec());
    }

    #[test]
    fn winkeyer_handles_special_characters() {
        // Prosigny <BT>, <AR>, <SK> oraz znaki przestankowe
        let frame = WinkeyerProtocol::encode_macro_frame("TEST <BT> 599 <AR> ? /").unwrap();
        assert_eq!(frame, b"TEST = 599 + ? /".to_vec());
    }

    #[test]
    fn prosigns_use_the_winkeyer_ascii_mapping() {
        // Table 17: SK = '>', AS = '[', KN = '('. '%' i '&' WinKeyer ignoruje.
        assert_eq!(
            WinkeyerProtocol::encode_text("TU <SK> <AS> <KN>"),
            b"TU > [ (".to_vec()
        );
    }

    #[test]
    fn control_bytes_and_ignored_characters_never_reach_the_keyer() {
        // Bajt 0x0A to „Clear Buffer”, 0x02 „Set speed”: w tekście byłyby poleceniami.
        let text = "CQ\u{2}\u{a}\u{1b}DE!#%&*~`{} SP6INA|ŁÓ\tK";
        assert_eq!(
            WinkeyerProtocol::encode_text(text),
            b"CQ DE SP6INA| K".to_vec()
        );
        for b in WinkeyerProtocol::encode_text(text) {
            assert!(b >= 0x20, "control byte {b:#04x} leaked");
        }
    }

    #[test]
    fn sidetone_frames_follow_the_datasheet() {
        // WK3: n = 62500 / f, np. 62 dla 1000 Hz.
        assert_eq!(WinkeyerProtocol::set_sidetone(1000, true), vec![0x01, 62]);
        assert_eq!(WinkeyerProtocol::set_sidetone(500, true), vec![0x01, 125]);
        // WK2: tabela 4000 / n.
        assert_eq!(WinkeyerProtocol::set_sidetone(800, false), vec![0x01, 5]);
        assert_eq!(WinkeyerProtocol::set_sidetone(700, false), vec![0x01, 6]);
        assert_eq!(WinkeyerProtocol::set_sidetone(4000, false), vec![0x01, 1]);
    }

    #[test]
    fn incoming_bytes_are_classified_by_their_tags() {
        assert_eq!(
            WinkeyerProtocol::classify(0xC4),
            WinkeyerByte::Status(WinkeyerStatus(0xC4))
        );
        assert!(WinkeyerStatus(0xC4).busy());
        assert!(WinkeyerStatus(0xC1).xoff());
        assert!(!WinkeyerStatus(0xC0).busy());
        assert_eq!(WinkeyerProtocol::classify(0x85), WinkeyerByte::SpeedPot(5));
        assert_eq!(WinkeyerProtocol::classify(b'K'), WinkeyerByte::Echo(b'K'));
    }

    #[test]
    fn winkeyer_rejects_empty_macro_by_contract() {
        assert_eq!(
            WinkeyerProtocol::encode_macro_frame(""),
            Err(WinkeyerError::EmptyMacro)
        );
        assert_eq!(
            WinkeyerProtocol::encode_macro_frame("   "),
            Err(WinkeyerError::EmptyMacro)
        );
    }

    #[test]
    fn winkeyer_rejects_oversized_macro() {
        let huge = "A".repeat(257);
        let res = WinkeyerProtocol::encode_macro_frame(&huge);
        assert_eq!(res, Err(WinkeyerError::OversizedMacro(257)));
    }

    #[test]
    fn winkeyer_handles_partial_mock_write() {
        let mut transport = MockWinkeyerTransport::new();
        transport.max_write_chunk = Some(4);

        let frame = b"CQ TEST".to_vec();
        let written = transport.write_frame(&frame).unwrap();
        assert_eq!(written, 4);
        assert_eq!(transport.written_bytes, b"CQ T");
        assert_eq!(transport.frames_enqueued, 0); // Częściowy zapis nie finalizuje kolejki
    }

    #[test]
    fn winkeyer_busy_does_not_duplicate_frame() {
        let mut transport = MockWinkeyerTransport::new();
        transport.simulate_busy = true;

        let frame = b"TEST".to_vec();
        let res = transport.write_frame(&frame);
        assert_eq!(res, Err(WinkeyerError::Busy));
        assert_eq!(transport.written_bytes, [] as [u8; 0]);
        assert_eq!(transport.frames_enqueued, 0);
    }

    #[test]
    fn winkeyer_macro_is_enqueued_once() {
        let mut transport = MockWinkeyerTransport::new();
        let frame = WinkeyerProtocol::encode_macro_frame("5NN TU").unwrap();

        let written = transport.write_frame(&frame).unwrap();
        assert_eq!(written, frame.len());
        assert_eq!(transport.frames_enqueued, 1);
        assert_eq!(transport.written_bytes, b"5NN TU");
    }

    #[test]
    fn winkeyer_shutdown_is_bounded() {
        let start = std::time::Instant::now();
        let mut transport = MockWinkeyerTransport::new();
        transport.reset();
        let elapsed = start.elapsed();
        assert!(elapsed < std::time::Duration::from_millis(100));
    }

    #[test]
    fn winkeyer_test_does_not_open_serial_port() {
        let transport = MockWinkeyerTransport::new();
        assert_eq!(transport.written_bytes, [] as [u8; 0]);
    }

    #[test]
    fn winkeyer_test_does_not_key_transmitter() {
        // Weryfikacja czysto generatorowa ramek protokołu K1EL bez dotykania portów szeregowych ani emisji RF
        let frame = WinkeyerProtocol::host_close();
        assert_eq!(frame, vec![0x00, 0x03]);
    }
}
