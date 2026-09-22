// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::path::PathBuf;
use std::process::{Child, Command};
use log::{info, warn, error};

#[derive(Debug)]
pub struct RigctldSupervisor {
    child: Option<Child>,
    pub port: u16,
    pub rig_id: u32,
    pub serial_port: String,
    pub baud_rate: u32,
    pub source: String,
    pub custom_path: String,
}

impl RigctldSupervisor {
    pub fn new(
        port: u16,
        rig_id: u32,
        serial_port: &str,
        baud_rate: u32,
        source: &str,
        custom_path: &str,
    ) -> Self {
        Self {
            child: None,
            port,
            rig_id,
            serial_port: serial_port.to_string(),
            baud_rate,
            source: source.to_string(),
            custom_path: custom_path.to_string(),
        }
    }

    /// Wyszukuje plik wykonywalny rigctld w zależności od wybranego źródła
    pub fn find_rigctld_binary(source: &str, custom_path: Option<&str>) -> Option<PathBuf> {
        // 1. Niestandardowa ścieżka wpisana przez użytkownika
        if let Some(cp) = custom_path {
            let cp_trimmed = cp.trim();
            if !cp_trimmed.is_empty() {
                let p = PathBuf::from(cp_trimmed);
                if p.is_file() {
                    return Some(p);
                }
            }
        }

        let bin_name = if cfg!(target_os = "windows") { "rigctld.exe" } else { "rigctld" };

        if source == "system" {
            // Źródło systemowe: najpierw PATH, potem standardowe ścieżki instalacyjne
            if let Some(p) = Self::find_in_path(bin_name) {
                return Some(p);
            }

            let sys_candidates: &[&str] = if cfg!(target_os = "windows") {
                &[
                    "C:/Program Files/hamlib/bin/rigctld.exe",
                    "C:/Program Files (x86)/hamlib/bin/rigctld.exe",
                    "C:/hamlib/bin/rigctld.exe",
                ]
            } else {
                &[
                    "/usr/bin/rigctld",
                    "/usr/local/bin/rigctld",
                    "/opt/hamlib/bin/rigctld",
                    "/opt/local/bin/rigctld",
                ]
            };

            for cand in sys_candidates {
                let pb = PathBuf::from(cand);
                if pb.is_file() {
                    return Some(pb);
                }
            }

            warn!("Nie znaleziono systemowego rigctld. Sprawdzanie wersji wbudowanej...");
            return Self::find_bundled_binary(bin_name);
        }

        // Źródło wbudowane (bundled)
        if let Some(p) = Self::find_bundled_binary(bin_name) {
            return Some(p);
        }

        // Bezpieczny fallback do systemowego PATH
        warn!("Nie znaleziono wbudowanego rigctld. Próba użycia systemowego...");
        Self::find_in_path(bin_name)
    }

    /// Wyszukuje plik rigctld w katalogu programu (wersja dołączona)
    fn find_bundled_binary(bin_name: &str) -> Option<PathBuf> {
        // Sprawdź obok pliku wykonywalnego aplikacji
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let candidate = parent.join(bin_name);
                if candidate.is_file() {
                    return Some(candidate);
                }
                let cand_hamlib = parent.join("hamlib").join(bin_name);
                if cand_hamlib.is_file() {
                    return Some(cand_hamlib);
                }
                let cand_hamlib_bin = parent.join("hamlib").join("bin").join(bin_name);
                if cand_hamlib_bin.is_file() {
                    return Some(cand_hamlib_bin);
                }
                let cand_win = parent.join("hamlib").join("hamlib-w64-4.7.2").join("bin").join(bin_name);
                if cand_win.is_file() {
                    return Some(cand_win);
                }
            }
        }

        // Sprawdź w relatywnym katalogu projektu
        let local_candidates = [
            PathBuf::from(bin_name),
            PathBuf::from(format!("hamlib/bin/{}", bin_name)),
            PathBuf::from(format!("hamlib/{}", bin_name)),
            PathBuf::from(format!("Bin/Windows/{}", bin_name)),
            PathBuf::from(format!("Bin/Windows/hamlib/hamlib-w64-4.7.2/bin/{}", bin_name)),
            PathBuf::from(format!("Bin/Linux/{}", bin_name)),
            PathBuf::from(format!("Bin/Linux/hamlib/bin/{}", bin_name)),
        ];

        for cand in &local_candidates {
            if cand.is_file() {
                return Some(cand.clone());
            }
        }

        None
    }

    /// Wyszukuje plik w zmiennej środowiskowej PATH
    fn find_in_path(bin_name: &str) -> Option<PathBuf> {
        if let Ok(path_var) = std::env::var("PATH") {
            let separator = if cfg!(target_os = "windows") { ';' } else { ':' };
            for p in path_var.split(separator) {
                let p_buf = PathBuf::from(p).join(bin_name);
                if p_buf.is_file() {
                    return Some(p_buf);
                }
            }
        }
        None
    }

    /// Uruchamia proces rigctld w tle
    pub fn start(&mut self) -> Result<(), String> {
        self.stop();

        let default_bin = if cfg!(target_os = "windows") { "rigctld.exe" } else { "rigctld" };
        let binary = Self::find_rigctld_binary(
            &self.source,
            if self.custom_path.is_empty() { None } else { Some(&self.custom_path) },
        )
        .unwrap_or_else(|| PathBuf::from(default_bin));

        info!("Uruchamianie natywnego rigctld ({}, {:?}) dla Rig ID: {}, Port: {}, Baud: {}", 
            self.source, binary, self.rig_id, self.serial_port, self.baud_rate);

        let mut cmd = Command::new(&binary);
        cmd.arg("-m").arg(self.rig_id.to_string());
        cmd.arg("-t").arg(self.port.to_string());

        if !self.serial_port.is_empty() {
            cmd.arg("-r").arg(&self.serial_port);
        }

        if self.baud_rate > 0 {
            cmd.arg("-s").arg(self.baud_rate.to_string());
        }

        // Konfiguracja bibliotek dynamicznych dla dołączonego Hamlib
        if let Some(parent) = binary.parent() {
            // Dodaj katalog binarki do PATH potomka (ułatwia odnalezienie DLL na Windows)
            if let Ok(old_path) = std::env::var("PATH") {
                let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
                cmd.env("PATH", format!("{}{}{}", parent.display(), sep, old_path));
            }

            // Na Linuksie: skonfiguruj LD_LIBRARY_PATH, aby biblioteki libhamlib.so ładowały się automatycznie
            #[cfg(target_family = "unix")]
            {
                let mut lib_dirs = Vec::new();
                let dir1 = parent.join("lib");
                let dir2 = parent.parent().map(|p| p.join("lib")).unwrap_or_default();
                let dir3 = parent.parent().map(|p| p.join("hamlib").join("lib")).unwrap_or_default();
                let dir4 = parent.to_path_buf();

                for d in &[dir1, dir2, dir3, dir4] {
                    if d.is_dir() {
                        lib_dirs.push(d.to_string_lossy().to_string());
                    }
                }

                if !lib_dirs.is_empty() {
                    let old_ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
                    let new_ld = if old_ld.is_empty() {
                        lib_dirs.join(":")
                    } else {
                        format!("{}:{}", lib_dirs.join(":"), old_ld)
                    };
                    cmd.env("LD_LIBRARY_PATH", new_ld);
                }
            }
        }

        // Na Windows: ukryj okno konsoli (CREATE_NO_WINDOW = 0x08000000)
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        match cmd.spawn() {
            Ok(child) => {
                info!("Pomyślnie uruchomiono natywny proces rigctld (PID: {})", child.id());
                self.child = Some(child);
                // Daj procesowi chwilę na otwarcie gniazda TCP
                std::thread::sleep(std::time::Duration::from_millis(300));
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Nie udało się uruchomić rigctld ({:?}): {}", binary, e);
                error!("{}", err_msg);
                Err(err_msg)
            }
        }
    }

    /// Sprawdza czy proces rigctld nadal działa w tle
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(status)) => {
                    info!("Proces rigctld zakończył działanie ze statusem: {:?}", status);
                    self.child = None;
                    false
                }
                Err(e) => {
                    warn!("Błąd sprawdzania statusu rigctld: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Zwraca identyfikator PID działającego procesu potomnego
    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }

    /// Zatrzymuje działający proces rigctld
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            info!("Zatrzymywanie procesu rigctld (PID: {})...", child.id());
            let _ = child.kill();
            for _ in 0..10 {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                    Err(_) => break,
                }
            }
            let _ = child.wait();
        }
    }
}

impl Drop for RigctldSupervisor {
    fn drop(&mut self) {
        self.stop();
    }
}
