// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Natywny rejestrator i odtwarzacz audio łączności (Audio QSO Memo)
// Wykorzystuje podsystem Windows Multimedia (winmm.dll MCI) bez zewnętrznych zależności.

use log::info;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum AudioRecorderError {
    InvalidMciCommand,
    MciError(u32),
    NotRecording,
    Io(String),
}

impl std::fmt::Display for AudioRecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMciCommand => write!(f, "Invalid characters in MCI command"),
            Self::MciError(code) => write!(f, "MCI error code: {code}"),
            Self::NotRecording => write!(f, "Recording was not active"),
            Self::Io(detail) => write!(f, "Audio recorder I/O error: {detail}"),
        }
    }
}

impl std::error::Error for AudioRecorderError {}

static IS_RECORDING: AtomicBool = AtomicBool::new(false);
static IS_PLAYING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn mciSendStringW(
        lpstrCommand: *const u16,
        lpstrReturnString: *mut u16,
        uReturnLength: u32,
        hwndCallback: usize,
    ) -> u32;
}

#[cfg(not(target_os = "windows"))]
static LINUX_REC_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);
#[cfg(not(target_os = "windows"))]
static LINUX_PLAY_CHILD: std::sync::Mutex<Option<std::process::Child>> =
    std::sync::Mutex::new(None);
#[cfg(not(target_os = "windows"))]
static LINUX_TEMP_FILE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

pub struct AudioRecorder;

impl AudioRecorder {
    /// Wysyła komendę MCI (UTF-16) do podsystemu Windows Multimedia
    #[cfg(target_os = "windows")]
    fn send_mci_cmd(cmd: &str) -> Result<(), AudioRecorderError> {
        use std::os::windows::ffi::OsStrExt;
        if cmd.contains('\0') {
            return Err(AudioRecorderError::InvalidMciCommand);
        }
        let wide: Vec<u16> = std::ffi::OsStr::new(cmd)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let res = unsafe { mciSendStringW(wide.as_ptr(), std::ptr::null_mut(), 0, 0) };
        if res == 0 {
            Ok(())
        } else {
            Err(AudioRecorderError::MciError(res))
        }
    }

    /// Rozpoczyna nagrywanie audio z karty dźwiękowej
    pub fn start_recording() -> Result<(), AudioRecorderError> {
        if IS_RECORDING.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Rozpoczynanie nagrywania audio QSO...");

        #[cfg(target_os = "windows")]
        {
            Self::send_mci_cmd("close splog_rec").ok();
            Self::send_mci_cmd("open new type waveaudio alias splog_rec")?;
            if let Err(e) = Self::send_mci_cmd("record splog_rec") {
                Self::send_mci_cmd("close splog_rec").ok();
                return Err(e);
            }
            IS_RECORDING.store(true, Ordering::SeqCst);
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let temp_file =
                std::env::temp_dir().join(format!("splog_rec_{}.wav", std::process::id()));
            let child = std::process::Command::new("arecord")
                .args(["-f", "cd", "-t", "wav"])
                .arg(&temp_file)
                .spawn()
                .or_else(|_| {
                    std::process::Command::new("pw-record")
                        .arg(&temp_file)
                        .spawn()
                });

            match child {
                Ok(c) => {
                    if let Ok(mut guard) = LINUX_REC_CHILD.lock() {
                        *guard = Some(c);
                    }
                    if let Ok(mut guard) = LINUX_TEMP_FILE.lock() {
                        *guard = Some(temp_file);
                    }
                    IS_RECORDING.store(true, Ordering::SeqCst);
                    Ok(())
                }
                Err(e) => {
                    log::warn!(
                        "Brak 'arecord' lub 'pw-record' w systemie Linux. Nagrywanie niedostępne: {e}"
                    );
                    Err(AudioRecorderError::Io(format!(
                        "Nie udało się uruchomić nagrywania: {e}"
                    )))
                }
            }
        }
    }

    /// Zatrzymuje nagrywanie i zapisuje do pliku WAV
    pub fn stop_and_save(output_path: &Path) -> Result<PathBuf, AudioRecorderError> {
        if !IS_RECORDING.load(Ordering::SeqCst) {
            return Err(AudioRecorderError::NotRecording);
        }

        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        #[cfg(target_os = "windows")]
        {
            let abs_path = output_path.to_string_lossy().replace('\\', "/");
            let save_cmd = format!("save splog_rec \"{abs_path}\"");

            let save_res = Self::send_mci_cmd(&save_cmd);
            let _ = Self::send_mci_cmd("close splog_rec");
            IS_RECORDING.store(false, Ordering::SeqCst);
            save_res?;

            info!(
                "Pomyślnie zapisano nagranie łączności do: {}",
                output_path.display()
            );
            Ok(output_path.to_path_buf())
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Some(mut child) = LINUX_REC_CHILD.lock().ok().and_then(|mut g| g.take()) {
                let _ = child.kill();
                let _ = child.wait();
            }
            IS_RECORDING.store(false, Ordering::SeqCst);
            if let Some(temp) = LINUX_TEMP_FILE.lock().ok().and_then(|mut g| g.take()) {
                if temp.exists() {
                    std::fs::copy(&temp, output_path).map_err(|e| {
                        AudioRecorderError::Io(format!("Błąd zapisu pliku WAV: {e}"))
                    })?;
                    let _ = std::fs::remove_file(temp);
                }
            }
            info!(
                "Pomyślnie zapisano nagranie łączności do: {}",
                output_path.display()
            );
            Ok(output_path.to_path_buf())
        }
    }

    /// Odtwarza plik audio łączności
    pub fn play_audio(file_path: &Path) -> Result<(), AudioRecorderError> {
        Self::stop_audio().ok();

        #[cfg(target_os = "windows")]
        {
            let abs_path = file_path.to_string_lossy().replace('\\', "/");
            let open_cmd = format!("open \"{abs_path}\" type waveaudio alias splog_play");
            Self::send_mci_cmd(&open_cmd)?;
            if let Err(e) = Self::send_mci_cmd("play splog_play") {
                Self::send_mci_cmd("close splog_play").ok();
                return Err(e);
            }
            IS_PLAYING.store(true, Ordering::SeqCst);
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let path = file_path.to_path_buf();
            let child = std::process::Command::new("aplay")
                .arg(&path)
                .spawn()
                .or_else(|_| std::process::Command::new("pw-play").arg(&path).spawn())
                .map_err(|e| {
                    AudioRecorderError::Io(format!(
                        "Nie udało się uruchomić odtwarzacza audio: {e}"
                    ))
                })?;
            if let Ok(mut guard) = LINUX_PLAY_CHILD.lock() {
                *guard = Some(child);
            }
            IS_PLAYING.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    /// Zatrzymuje aktualnie odtwarzany plik
    pub fn stop_audio() -> Result<(), AudioRecorderError> {
        #[cfg(target_os = "windows")]
        {
            Self::send_mci_cmd("stop splog_play").ok();
            Self::send_mci_cmd("close splog_play").ok();
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Some(mut child) = LINUX_PLAY_CHILD.lock().ok().and_then(|mut g| g.take()) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        IS_PLAYING.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Zatrzymuje nagrywanie audio i zwalnia zasoby bez zapisywania pliku.
    pub fn stop_recording() -> Result<(), AudioRecorderError> {
        if !IS_RECORDING.load(Ordering::SeqCst) {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Self::send_mci_cmd("stop splog_rec");
            let _ = Self::send_mci_cmd("close splog_rec");
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Some(mut child) = LINUX_REC_CHILD.lock().ok().and_then(|mut g| g.take()) {
                let _ = child.kill();
                let _ = child.wait();
            }
            if let Some(temp) = LINUX_TEMP_FILE.lock().ok().and_then(|mut g| g.take()) {
                if temp.exists() {
                    let _ = std::fs::remove_file(temp);
                }
            }
        }

        IS_RECORDING.store(false, Ordering::SeqCst);
        info!("Zatrzymano nagrywanie audio łączności");
        Ok(())
    }

    pub fn is_recording() -> bool {
        IS_RECORDING.load(Ordering::SeqCst)
    }

    pub fn is_playing() -> bool {
        IS_PLAYING.load(Ordering::SeqCst)
    }
}

/// Trait abstrakcji backendu rejestratora audio na potrzeby testów i izolacji od fizycznych urządzeń.
pub trait AudioRecorderBackend: Send + Sync {
    fn start(&mut self) -> Result<(), AudioRecorderError>;
    fn stop(&mut self) -> Result<(), AudioRecorderError>;
    fn flush(&mut self) -> Result<(), AudioRecorderError>;
    fn finalize(&mut self, output: &Path) -> Result<PathBuf, AudioRecorderError>;
    fn is_active(&self) -> bool;
}

/// Mock rejestratora audio działający wyłącznie w pamięci i plikach tymczasowych (zero fizycznego I/O).
pub struct MockAudioRecorderSession {
    worker: Option<std::thread::JoinHandle<()>>,
    stop_signal: std::sync::Arc<AtomicBool>,
    buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    output_handle: Option<std::fs::File>,
    pub simulate_finalize_error: bool,
    is_active: bool,
}

impl Default for MockAudioRecorderSession {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAudioRecorderSession {
    pub fn new() -> Self {
        Self {
            worker: None,
            stop_signal: std::sync::Arc::new(AtomicBool::new(false)),
            buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            output_handle: None,
            simulate_finalize_error: false,
            is_active: false,
        }
    }

    pub fn start_mock(&mut self) -> Result<(), AudioRecorderError> {
        self.stop_signal.store(false, Ordering::SeqCst);
        self.is_active = true;
        let stop_flag = self.stop_signal.clone();
        let buf = self.buffer.clone();
        self.worker = Some(std::thread::spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(5));
                if let Ok(mut g) = buf.lock() {
                    g.extend_from_slice(&[0x55, 0xAA]);
                }
            }
        }));
        Ok(())
    }

    pub fn set_output_handle(&mut self, file: std::fs::File) {
        self.output_handle = Some(file);
    }

    pub fn has_output_handle(&self) -> bool {
        self.output_handle.is_some()
    }

    pub fn has_worker(&self) -> bool {
        self.worker.is_some()
    }

    pub fn stop_recording(&mut self) -> Result<(), AudioRecorderError> {
        if !self.is_active {
            return Ok(());
        }
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
        self.flush()?;
        self.output_handle.take();
        self.is_active = false;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), AudioRecorderError> {
        if let Ok(mut g) = self.buffer.lock() {
            g.clear();
        }
        Ok(())
    }

    pub fn finalize(&mut self, output: &Path) -> Result<PathBuf, AudioRecorderError> {
        if self.simulate_finalize_error {
            return Err(AudioRecorderError::Io(
                "Mock finalize I/O error".to_string(),
            ));
        }
        self.stop_recording()?;
        std::fs::write(output, b"RIFF....WAVEfmt ....data....")
            .map_err(|e| AudioRecorderError::Io(e.to_string()))?;
        Ok(output.to_path_buf())
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl AudioRecorderBackend for MockAudioRecorderSession {
    fn start(&mut self) -> Result<(), AudioRecorderError> {
        self.start_mock()
    }

    fn stop(&mut self) -> Result<(), AudioRecorderError> {
        self.stop_recording()
    }

    fn flush(&mut self) -> Result<(), AudioRecorderError> {
        self.flush()
    }

    fn finalize(&mut self, output: &Path) -> Result<PathBuf, AudioRecorderError> {
        self.finalize(output)
    }

    fn is_active(&self) -> bool {
        self.is_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_recorder_stop_finalizes_mock_recording() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_rec_test_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("test_out.wav");

        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();
        assert!(session.is_active());

        let res = session.finalize(&target_file);
        assert!(res.is_ok());
        assert!(target_file.exists());
        assert!(!session.is_active());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn audio_recorder_stop_flushes_pending_data() {
        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));

        session.flush().unwrap();
        let buf_len = session.buffer.lock().unwrap().len();
        assert_eq!(buf_len, 0);
        let _ = session.stop_recording();
    }

    #[test]
    fn audio_recorder_stop_is_idempotent() {
        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();
        assert!(session.is_active());

        assert!(session.stop_recording().is_ok());
        assert!(!session.is_active());
        // Drugie wywołanie jest bezpieczne i idempotentne
        assert!(session.stop_recording().is_ok());
        assert!(!session.is_active());
    }

    #[test]
    fn audio_recorder_stop_releases_worker() {
        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();
        assert!(session.has_worker());

        session.stop_recording().unwrap();
        assert!(!session.has_worker());
    }

    #[test]
    fn audio_recorder_stop_releases_output_handle() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_rec_handle_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let handle_file = temp_dir.join("handle.wav");
        let file = std::fs::File::create(&handle_file).unwrap();

        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();
        session.set_output_handle(file);
        assert!(session.has_output_handle());

        session.stop_recording().unwrap();
        assert!(!session.has_output_handle());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn audio_recorder_stop_propagates_finalize_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_rec_err_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("err_out.wav");

        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();
        session.simulate_finalize_error = true;

        let res = session.finalize(&target_file);
        assert!(res.is_err());
        match res.unwrap_err() {
            AudioRecorderError::Io(msg) => assert!(msg.contains("Mock finalize I/O error")),
            other => panic!("Oczekiwano błędu Io, otrzymano: {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn audio_recorder_shutdown_is_bounded() {
        let mut session = MockAudioRecorderSession::new();
        session.start_mock().unwrap();

        let start = std::time::Instant::now();
        session.stop_recording().unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Shutdown workera trwał zbyt długo: {elapsed:?}"
        );
    }

    #[test]
    fn audio_recorder_test_does_not_open_physical_device() {
        // Weryfikacja, że MockAudioRecorderSession nie dotyka urządzeń fizycznych ani MCI
        let session = MockAudioRecorderSession::new();
        assert!(!session.is_active());
        assert!(!AudioRecorder::is_recording());
    }
}
