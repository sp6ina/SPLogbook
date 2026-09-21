// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Natywny rejestrator i odtwarzacz audio łączności (Audio QSO Memo)
// Wykorzystuje podsystem Windows Multimedia (winmm.dll MCI) bez zewnętrznych zależności.

use log::info;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static IS_RECORDING: AtomicBool = AtomicBool::new(false);
static IS_PLAYING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
extern "system" {
    fn mciSendStringA(
        lpstrCommand: *const std::os::raw::c_char,
        lpstrReturnString: *mut std::os::raw::c_char,
        uReturnLength: u32,
        hwndCallback: usize,
    ) -> u32;
}

#[cfg(not(target_os = "windows"))]
static LINUX_REC_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);
#[cfg(not(target_os = "windows"))]
static LINUX_TEMP_FILE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

pub struct AudioRecorder;

impl AudioRecorder {
    /// Wysyła komendę MCI do podsystemu Windows Multimedia
    #[cfg(target_os = "windows")]
    fn send_mci_cmd(cmd: &str) -> Result<(), String> {
        if let Ok(c_str) = CString::new(cmd) {
            let res = unsafe { mciSendStringA(c_str.as_ptr(), std::ptr::null_mut(), 0, 0) };
            if res == 0 {
                Ok(())
            } else {
                Err(format!("MCI error code: {}", res))
            }
        } else {
            Err("Nieprawidłowy ciąg znaków dla MCI".to_string())
        }
    }

    /// Rozpoczyna nagrywanie audio z karty dźwiękowej
    pub fn start_recording() -> Result<(), String> {
        if IS_RECORDING.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Rozpoczynanie nagrywania audio QSO...");

        #[cfg(target_os = "windows")]
        {
            Self::send_mci_cmd("close splog_rec").ok();
            Self::send_mci_cmd("open new type waveaudio alias splog_rec")?;
            Self::send_mci_cmd("record splog_rec")?;
            IS_RECORDING.store(true, Ordering::SeqCst);
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let temp_file = std::env::temp_dir().join(format!("splog_rec_{}.wav", chrono::Utc::now().timestamp()));
            let child = std::process::Command::new("arecord")
                .arg("-f").arg("cd")
                .arg("-t").arg("wav")
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
                    log::warn!("Nie udało się uruchomić nagrywania arecord/pw-record: {}", e);
                    IS_RECORDING.store(true, Ordering::SeqCst);
                    Ok(())
                }
            }
        }
    }

    /// Zatrzymuje nagrywanie i zapisuje do pliku WAV
    pub fn stop_and_save(output_path: &Path) -> Result<PathBuf, String> {
        if !IS_RECORDING.load(Ordering::SeqCst) {
            return Err("Nagrywanie nie było aktywne".to_string());
        }

        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        #[cfg(target_os = "windows")]
        {
            let abs_path = output_path.to_string_lossy().replace('\\', "/");
            let save_cmd = format!("save splog_rec \"{}\"", abs_path);

            Self::send_mci_cmd(&save_cmd)?;
            Self::send_mci_cmd("close splog_rec")?;
            IS_RECORDING.store(false, Ordering::SeqCst);

            info!("Pomyślnie zapisano nagranie łączności do: {:?}", output_path);
            Ok(output_path.to_path_buf())
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(mut guard) = LINUX_REC_CHILD.lock() {
                if let Some(mut child) = guard.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            if let Ok(mut guard) = LINUX_TEMP_FILE.lock() {
                if let Some(temp) = guard.take() {
                    if temp.exists() {
                        let _ = std::fs::copy(&temp, output_path);
                        let _ = std::fs::remove_file(temp);
                    }
                }
            }
            IS_RECORDING.store(false, Ordering::SeqCst);
            info!("Pomyślnie zapisano nagranie łączności do: {:?}", output_path);
            Ok(output_path.to_path_buf())
        }
    }

    /// Odtwarza plik audio łączności
    pub fn play_audio(file_path: &Path) -> Result<(), String> {
        Self::stop_audio().ok();

        #[cfg(target_os = "windows")]
        {
            let abs_path = file_path.to_string_lossy().replace('\\', "/");
            let open_cmd = format!("open \"{}\" type waveaudio alias splog_play", abs_path);
            Self::send_mci_cmd(&open_cmd)?;
            Self::send_mci_cmd("play splog_play")?;
            IS_PLAYING.store(true, Ordering::SeqCst);
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let p = file_path.to_path_buf();
            std::thread::spawn(move || {
                let _ = std::process::Command::new("aplay")
                    .arg(&p)
                    .status()
                    .or_else(|_| {
                        std::process::Command::new("pw-play")
                            .arg(&p)
                            .status()
                    });
            });
            IS_PLAYING.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    /// Zatrzymuje aktualnie odtwarzany plik
    pub fn stop_audio() -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            Self::send_mci_cmd("stop splog_play").ok();
            Self::send_mci_cmd("close splog_play").ok();
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("pkill").arg("-f").arg("aplay").status();
        }

        IS_PLAYING.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Sprawdza czy nagrywanie jest aktywne
    pub fn is_recording() -> bool {
        IS_RECORDING.load(Ordering::SeqCst)
    }

    /// Sprawdza czy odtwarzanie jest aktywne
    pub fn is_playing() -> bool {
        IS_PLAYING.load(Ordering::SeqCst)
    }
}
