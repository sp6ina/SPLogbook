use rodio::source::SineWave;
use rodio::{DeviceSinkBuilder, Player, Source};
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

/// Sekwencja tonów: (częstotliwość Hz, amplituda, czas trwania ms).
type Tones = Vec<(f32, f32, u64)>;

/// Odtwarza sekwencję tonów.
fn play_chime(tones: &Tones) {
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

/// Pojedynczy, współdzielony wątek odtwarzania dźwięków z ograniczoną kolejką.
/// Zapobiega tworzeniu nowego wątku przy każdym alercie oraz nieograniczonemu
/// gromadzeniu się zaległych sygnałów.
fn chime_worker() -> &'static mpsc::SyncSender<Tones> {
    static WORKER: OnceLock<mpsc::SyncSender<Tones>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<Tones>(16);
        std::thread::Builder::new()
            .name("splogbook-audio".to_string())
            .spawn(move || {
                while let Ok(tones) = rx.recv() {
                    play_chime(&tones);
                }
            })
            .expect("uruchomienie wątku audio");
        tx
    })
}

/// Kolejkuje sekwencję tonów; przy pełnej kolejce najstarszy alert jest pomijany.
fn enqueue(tones: Tones) {
    let _ = chime_worker().try_send(tones);
}

pub fn play_new_dxcc_alert() {
    enqueue(vec![(440.0, 0.2, 150), (660.0, 0.2, 300)]);
}

pub fn play_duplicate_alert() {
    enqueue(vec![(200.0, 0.2, 400)]);
}

pub fn play_new_iota_alert() {
    enqueue(vec![
        (523.25, 0.2, 100),
        (659.25, 0.2, 100),
        (783.99, 0.2, 200),
    ]);
}

pub fn play_qso_saved_alert() {
    enqueue(vec![(1000.0, 0.1, 50)]);
}

/// Krótka, wznosząca sekwencja sygnalizująca otwarcie pasma propagacyjnego.
pub fn play_band_opened_alert() {
    enqueue(vec![
        (392.0, 0.18, 120),
        (523.25, 0.18, 120),
        (659.25, 0.18, 240),
    ]);
}
