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

    /// Wyczyszczenie bufora / natychmiastowe przerwanie nadawania (Clear Buffer: 0x0A)
    pub fn abort_buffer() -> Vec<u8> {
        vec![0x0A]
    }

    /// Ustawienie częstotliwości podsłuchu (0x01, 0x04, sidetone_code)
    pub fn set_sidetone(hz: u16) -> Vec<u8> {
        // Kod sidetone wg K1EL WK spec: 4000 / hz
        let code = 4000u16.checked_div(hz).map(|c| c as u8).unwrap_or(5);
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
        if count >= self.queued_text.len() {
            self.sent_text.push_str(&self.queued_text);
            self.queued_text.clear();
        } else {
            let (sent_part, remaining) = self.queued_text.split_at(count);
            self.sent_text.push_str(sent_part);
            self.queued_text = remaining.to_string();
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
        assert_eq!(WinkeyerProtocol::encode_text("cq de sp6ina k"), b"CQ DE SP6INA K".to_vec());
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
        assert!(buf.queued_text.is_empty());
    }
}
