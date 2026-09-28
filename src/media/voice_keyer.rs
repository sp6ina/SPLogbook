// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Voice keyer (SSB) — odtwarzanie zapowiedzi z plików WAV przez kartę
//! dźwiękową oraz pomocnicza logika wyboru slotów F1..F8.

use crate::core::station::VoiceKeyerMessage;
use rodio::{Decoder, DeviceSinkBuilder, Player, Source};
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Globalna flaga zatrzymania odtwarzania w pętli (np. CQ loop).
static STOP_PLAYBACK: AtomicBool = AtomicBool::new(false);

/// Żąda zatrzymania odtwarzania zapętlonego komunikatu.
pub fn request_stop() {
    STOP_PLAYBACK.store(true, Ordering::SeqCst);
}

/// Odtwarza plik WAV do domyślnego urządzenia wyjściowego (blokujące).
pub fn play_wav_file(path: &str) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Nie można otworzyć pliku: {e}"))?;
    let source = Decoder::new(BufReader::new(file))
        .map_err(|e| format!("Nieobsługiwany format audio: {e}"))?;
    let sink = DeviceSinkBuilder::open_default_sink()
        .map_err(|e| format!("Brak urządzenia audio: {e}"))?;
    let player = Player::connect_new(sink.mixer());
    player.append(source);
    player.sleep_until_end();
    Ok(())
}

/// Odtwarza komunikat voice keyer'a. Gdy plik WAV nie jest ustawiony,
/// generuje krótki sygnał testowy (sine 800 Hz).
pub fn play_message(msg: &VoiceKeyerMessage) -> Result<(), String> {
    STOP_PLAYBACK.store(false, Ordering::SeqCst);
    let sink = DeviceSinkBuilder::open_default_sink()
        .map_err(|e| format!("Brak urządzenia audio: {e}"))?;
    let player = Player::connect_new(sink.mixer());

    let wav_path = msg
        .wav_path
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .map(std::string::ToString::to_string);

    if let Some(path) = wav_path {
        let file = File::open(&path).map_err(|e| format!("Nie można otworzyć pliku: {e}"))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Nieobsługiwany format audio: {e}"))?;
        if msg.repeat {
            player.append(source.repeat_infinite());
            wait_for_stop(&player);
        } else {
            player.append(source);
            player.sleep_until_end();
        }
    } else {
        let tone = rodio::source::SineWave::new(800.0)
            .take_duration(Duration::from_millis(700))
            .amplify(0.25);
        if msg.repeat {
            player.append(tone.repeat_infinite());
            wait_for_stop(&player);
        } else {
            player.append(tone);
            player.sleep_until_end();
        }
    }
    Ok(())
}

/// Oczekuje na żądanie zatrzymania (dla trybu pętli).
fn wait_for_stop(player: &Player) {
    while !STOP_PLAYBACK.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(50));
    }
    player.stop();
}

/// Zwraca indeks następnego włączonego slotu (z zawinięciem, pomija wyłączone).
pub fn next_enabled_index(messages: &[VoiceKeyerMessage], current: Option<usize>) -> Option<usize> {
    let n = messages.len();
    if n == 0 {
        return None;
    }
    let start = current.map_or(0, |c| (c + 1) % n);
    for i in 0..n {
        let idx = (start + i) % n;
        if messages[idx].enabled {
            return Some(idx);
        }
    }
    None
}

/// Zwraca indeks poprzedniego włączonego slotu (z zawinięciem, pomija wyłączone).
pub fn prev_enabled_index(messages: &[VoiceKeyerMessage], current: Option<usize>) -> Option<usize> {
    let n = messages.len();
    if n == 0 {
        return None;
    }
    let start = current.map_or(n - 1, |c| (c + n - 1) % n);
    for i in 0..n {
        let idx = (start + n - i) % n;
        if messages[idx].enabled {
            return Some(idx);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(enabled: bool) -> VoiceKeyerMessage {
        VoiceKeyerMessage {
            enabled,
            ..Default::default()
        }
    }

    #[test]
    fn next_skips_disabled_and_wraps() {
        let msgs = vec![msg(true), msg(false), msg(true), msg(false)];
        assert_eq!(next_enabled_index(&msgs, None), Some(0));
        assert_eq!(next_enabled_index(&msgs, Some(0)), Some(2));
        assert_eq!(next_enabled_index(&msgs, Some(2)), Some(0));
    }

    #[test]
    fn prev_skips_disabled_and_wraps() {
        let msgs = vec![msg(true), msg(false), msg(true), msg(false)];
        assert_eq!(prev_enabled_index(&msgs, None), Some(2));
        assert_eq!(prev_enabled_index(&msgs, Some(0)), Some(2));
        assert_eq!(prev_enabled_index(&msgs, Some(2)), Some(0));
    }

    #[test]
    fn empty_returns_none() {
        let msgs: Vec<VoiceKeyerMessage> = vec![];
        assert_eq!(next_enabled_index(&msgs, None), None);
        assert_eq!(prev_enabled_index(&msgs, None), None);
    }

    #[test]
    fn all_disabled_returns_none() {
        let msgs = vec![msg(false), msg(false)];
        assert_eq!(next_enabled_index(&msgs, None), None);
        assert_eq!(prev_enabled_index(&msgs, None), None);
    }
}
