use rodio::source::SineWave;
use rodio::{DeviceSinkBuilder, Player, Source};
use std::time::Duration;

/// Odtwarza sekwencję tonów (częstotliwość Hz, amplituda, czas trwania ms).
fn play_chime(tones: &[(f32, f32, u64)]) {
    let Ok(sink) = DeviceSinkBuilder::open_default_sink() else {
        log::warn!("Nie można otworzyć domyślnego wyjścia audio");
        return;
    };
    let player = Player::connect_new(sink.mixer());
    for &(freq, amp, ms) in tones {
        player.append(
            SineWave::new(freq)
                .take_duration(Duration::from_millis(ms))
                .amplify(amp),
        );
    }
    player.sleep_until_end();
}

pub fn play_new_dxcc_alert() {
    std::thread::spawn(|| play_chime(&[(440.0, 0.2, 150), (660.0, 0.2, 300)]));
}

pub fn play_duplicate_alert() {
    std::thread::spawn(|| play_chime(&[(200.0, 0.2, 400)]));
}

pub fn play_new_iota_alert() {
    std::thread::spawn(|| {
        play_chime(&[
            (523.25, 0.2, 100),
            (659.25, 0.2, 100),
            (783.99, 0.2, 200),
        ])
    });
}

pub fn play_qso_saved_alert() {
    std::thread::spawn(|| play_chime(&[(1000.0, 0.1, 50)]));
}

/// Krótka, wznosząca sekwencja sygnalizująca otwarcie pasma propagacyjnego.
pub fn play_band_opened_alert() {
    std::thread::spawn(|| {
        play_chime(&[
            (392.0, 0.18, 120),
            (523.25, 0.18, 120),
            (659.25, 0.18, 240),
        ])
    });
}
