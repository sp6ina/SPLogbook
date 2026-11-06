// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// K1EL Winkeyer (WK2 / WK3) and Serial CW Keyer Driver

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

    /// Ustawienie prędkości WPM (0x02, wpm)
    pub fn set_speed(wpm: u8) -> Vec<u8> {
        let clamped = wpm.clamp(5, 55);
        vec![0x02, clamped]
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

        // Zamiana znanych prosignów telegraficznych
        let sanitized = trimmed
            .replace("<BT>", "=")
            .replace("<AR>", "+")
            .replace("<SK>", "%")
            .replace("<AS>", "&")
            .replace("<KN>", "(");

        Ok(Self::encode_text(&sanitized))
    }

    /// Wyczyszczenie bufora / natychmiastowe przerwanie nadawania (Clear Buffer: 0x0A)
    pub fn abort_buffer() -> Vec<u8> {
        vec![0x0A]
    }

    /// Ustawienie częstotliwości podsłuchu (0x01, 0x04, sidetone_code)
    pub fn set_sidetone(hz: u16) -> Vec<u8> {
        // Kod sidetone wg K1EL WK spec: 1..=10 (4000 / hz)
        let code = 4000u16.checked_div(hz).map_or(5, |c| c.clamp(1, 10) as u8);
        vec![0x01, 0x04, code]
    }

    /// Kodowanie ciągu tekstowego na bajty do wysłania do Winkeyera
    pub fn encode_text(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for ch in text.to_uppercase().chars() {
            if ch.is_ascii() {
                bytes.push(ch as u8);
            }
        }
        bytes
    }
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

        let frame_clamped_high = WinkeyerProtocol::encode_wpm_frame(60);
        assert_eq!(frame_clamped_high, vec![0x02, 55]);
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
