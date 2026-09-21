// SPLogbook: Natywny zarządca procesu rigctld Hamlib w tle (Windows)
// Uruchamia rigctld.exe jako proces potomny z flagą CREATE_NO_WINDOW (bez konsoli)
// i zarządza jego cyklem życia.

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
}

impl RigctldSupervisor {
    pub fn new(port: u16, rig_id: u32, serial_port: &str, baud_rate: u32) -> Self {
        Self {
            child: None,
            port,
            rig_id,
            serial_port: serial_port.to_string(),
            baud_rate,
        }
    }

    /// Wyszukuje plik wykonywalny rigctld.exe w znanych lokalizacjach programu
    pub fn find_rigctld_binary() -> Option<PathBuf> {
        // 1. Sprawdź folder z bieżącym plikiem wykonywalnym aplikacji
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let candidate = parent.join("rigctld.exe");
                if candidate.exists() {
                    return Some(candidate);
                }
                let cand2 = parent.join("hamlib").join("hamlib-w64-4.7.2").join("bin").join("rigctld.exe");
                if cand2.exists() {
                    return Some(cand2);
                }
            }
        }

        // 2. Sprawdź relatywnie w folderze roboczym projektu
        let candidates = [
            PathBuf::from("Bin/Windows/rigctld.exe"),
            PathBuf::from("Bin/Windows/hamlib/hamlib-w64-4.7.2/bin/rigctld.exe"),
            PathBuf::from("rigctld.exe"),
            PathBuf::from("C:/Program Files/hamlib/bin/rigctld.exe"),
        ];

        for cand in &candidates {
            if cand.exists() {
                return Some(cand.clone());
            }
        }

        // 3. Sprawdź w PATH
        None
    }

    /// Uruchamia proces rigctld.exe w tle bez pokazywania okna konsoli
    pub fn start(&mut self) -> Result<(), String> {
        self.stop();

        let binary = Self::find_rigctld_binary()
            .unwrap_or_else(|| PathBuf::from("rigctld.exe"));

        info!("Uruchamianie natywnego rigctld: {:?} dla Rig ID: {}, Port: {}, Baud: {}", 
            binary, self.rig_id, self.serial_port, self.baud_rate);

        let mut cmd = Command::new(&binary);
        cmd.arg("-m").arg(self.rig_id.to_string());
        cmd.arg("-t").arg(self.port.to_string());

        if !self.serial_port.is_empty() {
            cmd.arg("-r").arg(&self.serial_port);
        }

        if self.baud_rate > 0 {
            cmd.arg("-s").arg(self.baud_rate.to_string());
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
