use rodio::{OutputStream, Sink, Source};
use rodio::source::SineWave;
use std::time::Duration;

pub fn play_new_dxcc_alert() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                // Triumphant short chime (2 tones)
                let tone1 = SineWave::new(440.0).take_duration(Duration::from_millis(150)).amplify(0.2);
                let tone2 = SineWave::new(660.0).take_duration(Duration::from_millis(300)).amplify(0.2);
                sink.append(tone1);
                sink.append(tone2);
                sink.sleep_until_end();
            }
        }
    });
}

pub fn play_duplicate_alert() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                // Low buzzer (1 tone)
                let tone = SineWave::new(200.0).take_duration(Duration::from_millis(400)).amplify(0.2);
                sink.append(tone);
                sink.sleep_until_end();
            }
        }
    });
}

pub fn play_new_iota_alert() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                // Medium jingle
                let tone1 = SineWave::new(523.25).take_duration(Duration::from_millis(100)).amplify(0.2);
                let tone2 = SineWave::new(659.25).take_duration(Duration::from_millis(100)).amplify(0.2);
                let tone3 = SineWave::new(783.99).take_duration(Duration::from_millis(200)).amplify(0.2);
                sink.append(tone1);
                sink.append(tone2);
                sink.append(tone3);
                sink.sleep_until_end();
            }
        }
    });
}

pub fn play_qso_saved_alert() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                // Soft click
                let tone = SineWave::new(1000.0).take_duration(Duration::from_millis(50)).amplify(0.1);
                sink.append(tone);
                sink.sleep_until_end();
            }
        }
    });
}
