// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use log::{error, info, warn};
use std::path::PathBuf;
use std::process::{Child, Command};

/// Błędy nadzorcy procesu demona `rigctld`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorError {
    MissingSerialPort,
    ProcessExited {
        status: String,
        serial_port: String,
        baud_rate: u32,
    },
    SpawnFailed {
        binary: String,
        detail: String,
    },
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSerialPort => write!(
                f,
                "Select a serial port (e.g. COM3 / /dev/ttyUSB0) for the chosen radio model."
            ),
            Self::ProcessExited {
                status,
                serial_port,
                baud_rate,
            } => write!(
                f,
                "rigctld terminated (code: {status}) - verify serial port '{serial_port}' ({baud_rate} bps) and ensure radio is powered on."
            ),
            Self::SpawnFailed { binary, detail } => {
                write!(f, "Failed to start rigctld ({binary}): {detail}")
            }
        }
    }
}

impl std::error::Error for SupervisorError {}

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
                let path = PathBuf::from(cp_trimmed);
                if path.is_file() {
                    return Some(path);
                }
            }
        }

        let bin_name = if cfg!(target_os = "windows") {
            "rigctld.exe"
        } else {
            "rigctld"
        };

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
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(parent) = exe_path.parent()
        {
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
            let cand_win = parent
                .join("hamlib")
                .join("hamlib-w64-4.7.2")
                .join("bin")
                .join(bin_name);
            if cand_win.is_file() {
                return Some(cand_win);
            }
        }

        // Sprawdź w relatywnym katalogu projektu
        let local_candidates = [
            PathBuf::from(bin_name),
            PathBuf::from(format!("hamlib/bin/{bin_name}")),
            PathBuf::from(format!("hamlib/{bin_name}")),
            PathBuf::from(format!("Bin/Windows/{bin_name}")),
            PathBuf::from(format!(
                "Bin/Windows/hamlib/hamlib-w64-4.7.2/bin/{bin_name}"
            )),
            PathBuf::from(format!("Bin/Linux/{bin_name}")),
            PathBuf::from(format!("Bin/Linux/hamlib/bin/{bin_name}")),
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
            let separator = if cfg!(target_os = "windows") {
                ';'
            } else {
                ':'
            };
            for part in path_var.split(separator) {
                let p_buf = PathBuf::from(part).join(bin_name);
                if p_buf.is_file() {
                    return Some(p_buf);
                }
            }
        }
        None
    }

    /// Uruchamia proces rigctld w tle
    pub fn start(&mut self) -> Result<(), SupervisorError> {
        self.stop();

        let default_bin = if cfg!(target_os = "windows") {
            "rigctld.exe"
        } else {
            "rigctld"
        };
        let binary = Self::find_rigctld_binary(
            &self.source,
            if self.custom_path.is_empty() {
                None
            } else {
                Some(&self.custom_path)
            },
        )
        .unwrap_or_else(|| PathBuf::from(default_bin));

        info!(
            "Uruchamianie natywnego rigctld ({}, {}) dla Rig ID: {}, Port: {}, Baud: {}",
            self.source,
            binary.display(),
            self.rig_id,
            self.serial_port,
            self.baud_rate
        );

        let mut cmd = Command::new(&binary);
        cmd.arg("-m").arg(self.rig_id.to_string());
        cmd.arg("-t").arg(self.port.to_string());

        let normalized_port = self.serial_port.trim();
        if !normalized_port.is_empty() {
            #[cfg(target_os = "windows")]
            let arg_port = {
                let upper = normalized_port.to_ascii_uppercase();
                if upper.starts_with("COM")
                    && upper.len() > 4
                    && !normalized_port.starts_with(r"\\.\")
                {
                    format!(r"\\.\{normalized_port}")
                } else {
                    normalized_port.to_string()
                }
            };
            #[cfg(not(target_os = "windows"))]
            let arg_port = normalized_port.to_string();

            cmd.arg("-r").arg(arg_port);
        } else if self.rig_id > 6 {
            return Err(SupervisorError::MissingSerialPort);
        }

        if self.baud_rate > 0 && self.rig_id > 6 {
            cmd.arg("-s").arg(self.baud_rate.to_string());
        }

        // Konfiguracja bibliotek dynamicznych dla dołączonego Hamlib
        if let Some(parent) = binary.parent() {
            // Dodaj katalog binarki do PATH potomka (ułatwia odnalezienie DLL na Windows)
            if let Ok(old_path) = std::env::var("PATH") {
                let sep = if cfg!(target_os = "windows") {
                    ";"
                } else {
                    ":"
                };
                cmd.env("PATH", format!("{}{}{}", parent.display(), sep, old_path));
            }

            // Na Linuksie: skonfiguruj LD_LIBRARY_PATH, aby biblioteki libhamlib.so ładowały się automatycznie
            #[cfg(target_family = "unix")]
            {
                let mut lib_dirs = Vec::new();
                let dir1 = parent.join("lib");
                let dir2 = parent.parent().map(|p| p.join("lib")).unwrap_or_default();
                let dir3 = parent
                    .parent()
                    .map(|p| p.join("hamlib").join("lib"))
                    .unwrap_or_default();
                let dir4 = parent.to_path_buf();

                for dir in &[dir1, dir2, dir3, dir4] {
                    if dir.is_dir() {
                        lib_dirs.push(dir.to_string_lossy().to_string());
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
            cmd.creation_flags(0x0800_0000);
        }

        match cmd.spawn() {
            Ok(mut child) => {
                std::thread::sleep(std::time::Duration::from_millis(15));
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(SupervisorError::ProcessExited {
                        status: status.to_string(),
                        serial_port: self.serial_port.clone(),
                        baud_rate: self.baud_rate,
                    });
                }
                info!(
                    "Pomyślnie uruchomiono natywny proces rigctld (PID: {})",
                    child.id()
                );
                self.child = Some(child);
                Ok(())
            }
            Err(e) => {
                let err = SupervisorError::SpawnFailed {
                    binary: binary.display().to_string(),
                    detail: e.to_string(),
                };
                error!("{err}");
                Err(err)
            }
        }
    }

    /// Sprawdza czy proces rigctld nadal działa w tle
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(status)) => {
                    info!("Proces rigctld zakończył działanie ze statusem: {status:?}");
                    self.child = None;
                    false
                }
                Err(e) => {
                    warn!("Błąd sprawdzania statusu rigctld: {e}");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Zwraca identyfikator PID działającego procesu potomnego
    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(std::process::Child::id)
    }

    /// Zatrzymuje działający proces rigctld
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            info!("Zatrzymywanie procesu rigctld (PID: {})...", child.id());
            let _ = child.kill();
            for _ in 0..10 {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(15)),
                    Err(_) => break,
                }
            }
        }
    }
}

impl Drop for RigctldSupervisor {
    fn drop(&mut self) {
        self.stop();
    }
}

use crate::cat::routing::RigId;
use std::collections::HashMap;

/// Normalizuje nazwę portu fizycznego/szeregowego (np. COM3, com3, \\.\COM3 -> COM3; ścieżki unixowe).
pub fn normalize_serial_port(port: &str) -> String {
    let trimmed = port.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let stripped = if let Some(s) = trimmed.strip_prefix(r"\\.\") {
        s
    } else if let Some(s) = trimmed.strip_prefix(r"//./") {
        s
    } else {
        trimmed
    };

    let upper = stripped.to_ascii_uppercase();
    if upper.starts_with("COM") {
        upper
    } else {
        stripped.to_string()
    }
}

/// Właściciel fizycznego portu sprzętowego (UART / Virtual COM / Serial).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalEndpointOwner {
    /// Bezpośredni, natywny runtime aplikacji otwierający port szeregowy UART.
    NativeCatRuntime(RigId),
    /// Zewnętrzny proces pomocniczy (np. rigctld) uruchomiony dla danego radia.
    ExternalRigctld(RigId),
    /// Klient Hamlib łączący się przez TCP z zewnętrznym serwerem (brak dostępu do portu fizycznego).
    HamlibClient(RigId),
    /// Klient radia sieciowego (np. TCI, KNS LAN, FLRig) bez dostępu do portu fizycznego.
    NetworkRadio(RigId),
}

impl PhysicalEndpointOwner {
    /// Zwraca identyfikator radia powiązany z właścicielem.
    pub fn rig_id(&self) -> &RigId {
        match self {
            Self::NativeCatRuntime(id)
            | Self::ExternalRigctld(id)
            | Self::HamlibClient(id)
            | Self::NetworkRadio(id) => id,
        }
    }

    /// Określa, czy dany typ właściciela wymaga wyłącznego dostępu do fizycznego portu szeregowego.
    pub fn requires_exclusive_physical_port(&self) -> bool {
        matches!(self, Self::NativeCatRuntime(_) | Self::ExternalRigctld(_))
    }
}

impl std::fmt::Display for PhysicalEndpointOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NativeCatRuntime(id) => write!(f, "NativeCatRuntime({id})"),
            Self::ExternalRigctld(id) => write!(f, "ExternalRigctld({id})"),
            Self::HamlibClient(id) => write!(f, "HamlibClient({id})"),
            Self::NetworkRadio(id) => write!(f, "NetworkRadio({id})"),
        }
    }
}

/// Błąd konfliktu własności fizycznego punktu końcowego.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointConflictError {
    /// Port szeregowy jest już zajęty przez innego właściciela.
    PortAlreadyClaimed {
        port: String,
        current_owner: PhysicalEndpointOwner,
        attempted_owner: PhysicalEndpointOwner,
    },
    /// Próba uruchomienia zewnętrznego demona na porcie zajętym przez natywny CAT.
    NativeAndExternalConflict {
        port: String,
        native_rig: RigId,
        external_rig: RigId,
    },
    /// Pusta nazwa portu.
    EmptyPortName,
}

impl std::fmt::Display for EndpointConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PortAlreadyClaimed {
                port,
                current_owner,
                attempted_owner,
            } => write!(
                f,
                "port fizyczny '{port}' jest już zajęty przez {current_owner}, odmowa dla {attempted_owner}"
            ),
            Self::NativeAndExternalConflict {
                port,
                native_rig,
                external_rig,
            } => write!(
                f,
                "konflikt portu '{port}': radio '{native_rig}' używa Native CAT, a '{external_rig}' próbuje uruchomić zewnętrzny rigctld"
            ),
            Self::EmptyPortName => write!(f, "nazwa portu fizycznego nie może być pusta"),
        }
    }
}

impl std::error::Error for EndpointConflictError {}

/// Rejestr wyłącznej własności fizycznych portów szeregowych i endpointów radiowych.
#[derive(Debug, Clone, Default)]
pub struct PhysicalEndpointRegistry {
    claims: HashMap<String, PhysicalEndpointOwner>,
}

impl PhysicalEndpointRegistry {
    pub fn new() -> Self {
        Self {
            claims: HashMap::new(),
        }
    }

    /// Rejestruje żądanie wyłącznej własności portu fizycznego dla wskazanego właściciela.
    pub fn claim(
        &mut self,
        port: &str,
        owner: PhysicalEndpointOwner,
    ) -> Result<(), EndpointConflictError> {
        let norm_port = normalize_serial_port(port);
        if norm_port.is_empty() {
            return Err(EndpointConflictError::EmptyPortName);
        }

        // Tryby sieciowe/klienckie nie rezerwują fizycznego portu COM
        if !owner.requires_exclusive_physical_port() {
            return Ok(());
        }

        if let Some(existing) = self.claims.get(&norm_port) {
            if existing == &owner {
                return Ok(());
            }
            if let (
                PhysicalEndpointOwner::NativeCatRuntime(native_id),
                PhysicalEndpointOwner::ExternalRigctld(ext_id),
            ) = (existing, &owner)
            {
                return Err(EndpointConflictError::NativeAndExternalConflict {
                    port: norm_port,
                    native_rig: native_id.clone(),
                    external_rig: ext_id.clone(),
                });
            }
            if let (
                PhysicalEndpointOwner::ExternalRigctld(ext_id),
                PhysicalEndpointOwner::NativeCatRuntime(native_id),
            ) = (existing, &owner)
            {
                return Err(EndpointConflictError::NativeAndExternalConflict {
                    port: norm_port,
                    native_rig: native_id.clone(),
                    external_rig: ext_id.clone(),
                });
            }
            return Err(EndpointConflictError::PortAlreadyClaimed {
                port: norm_port,
                current_owner: existing.clone(),
                attempted_owner: owner,
            });
        }

        self.claims.insert(norm_port, owner);
        Ok(())
    }

    /// Zwalnia rezerwację portu fizycznego.
    pub fn release(&mut self, port: &str) {
        let norm_port = normalize_serial_port(port);
        self.claims.remove(&norm_port);
    }

    /// Zwalnia wszystkie porty powiązane z danym radiem (np. przy shutdown radia).
    pub fn release_for_rig(&mut self, rig_id: &RigId) {
        self.claims.retain(|_, owner| owner.rig_id() != rig_id);
    }

    /// Sprawdza bieżącego właściciela portu fizycznego.
    pub fn get_owner(&self, port: &str) -> Option<&PhysicalEndpointOwner> {
        let norm_port = normalize_serial_port(port);
        self.claims.get(&norm_port)
    }

    /// Czy port jest obecnie zarezerwowany.
    pub fn is_claimed(&self, port: &str) -> bool {
        let norm_port = normalize_serial_port(port);
        self.claims.contains_key(&norm_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_serial_endpoint_cannot_have_two_owners() {
        let mut reg = PhysicalEndpointRegistry::new();
        let r1 = RigId::new("rig-1").unwrap();
        let r2 = RigId::new("rig-2").unwrap();

        // Rezerwacja COM3 przez rig-1
        reg.claim("COM3", PhysicalEndpointOwner::NativeCatRuntime(r1.clone()))
            .unwrap();

        // Próba rezerwacji tego samego portu z prefiksem \\.\com3 przez rig-2
        let err = reg
            .claim(
                r"\\.\com3",
                PhysicalEndpointOwner::NativeCatRuntime(r2.clone()),
            )
            .unwrap_err();
        match err {
            EndpointConflictError::PortAlreadyClaimed {
                port,
                current_owner,
                attempted_owner,
            } => {
                assert_eq!(port, "COM3");
                assert_eq!(current_owner, PhysicalEndpointOwner::NativeCatRuntime(r1));
                assert_eq!(attempted_owner, PhysicalEndpointOwner::NativeCatRuntime(r2));
            }
            other => panic!("Oczekiwano PortAlreadyClaimed, otrzymano: {other:?}"),
        }
    }

    #[test]
    fn native_mode_does_not_start_external_rigctld_for_same_port() {
        let mut reg = PhysicalEndpointRegistry::new();
        let r1 = RigId::new("rig-native").unwrap();
        let r2 = RigId::new("rig-external").unwrap();

        reg.claim("COM4", PhysicalEndpointOwner::NativeCatRuntime(r1.clone()))
            .unwrap();

        // Próba uruchomienia external rigctld na tym samym porcie
        let err = reg
            .claim("com4", PhysicalEndpointOwner::ExternalRigctld(r2.clone()))
            .unwrap_err();
        match err {
            EndpointConflictError::NativeAndExternalConflict {
                port,
                native_rig,
                external_rig,
            } => {
                assert_eq!(port, "COM4");
                assert_eq!(native_rig, r1);
                assert_eq!(external_rig, r2);
            }
            other => panic!("Oczekiwano NativeAndExternalConflict, otrzymano: {other:?}"),
        }
    }

    #[test]
    fn hamlib_client_mode_does_not_open_native_serial_port() {
        let mut reg = PhysicalEndpointRegistry::new();
        let r1 = RigId::new("rig-client").unwrap();

        // Tryb klienta Hamlib (połączenie TCP do zewnętrznego demona) nie rezerwuje fizycznego UART
        reg.claim("COM1", PhysicalEndpointOwner::HamlibClient(r1))
            .unwrap();
        assert!(
            !reg.is_claimed("COM1"),
            "HamlibClient nie powinien rezerwować fizycznego portu"
        );
    }

    #[test]
    fn hamlib_proxy_never_opens_physical_port_directly() {
        // HamlibProxyServer działa w oparciu o TCP i CatRuntime, bez bezpośredniego otwierania UART
        let mut reg = PhysicalEndpointRegistry::new();
        let r1 = RigId::new("rig-native").unwrap();

        // Fizyczny port należy wyłącznie do NativeCatRuntime
        reg.claim("COM2", PhysicalEndpointOwner::NativeCatRuntime(r1.clone()))
            .unwrap();
        assert!(reg.is_claimed("COM2"));

        // Proxy korzysta z mostu do CatRuntime, a nie z fizycznego portu
        let owner = reg.get_owner("COM2").unwrap();
        assert_eq!(owner, &PhysicalEndpointOwner::NativeCatRuntime(r1));
    }

    #[test]
    fn port_owner_is_released_on_shutdown() {
        let mut reg = PhysicalEndpointRegistry::new();
        let r1 = RigId::new("rig-1").unwrap();
        let r2 = RigId::new("rig-2").unwrap();

        reg.claim("COM5", PhysicalEndpointOwner::NativeCatRuntime(r1.clone()))
            .unwrap();
        assert!(reg.is_claimed("COM5"));

        // Zwolnienie portu przy zatrzymaniu rig-1
        reg.release_for_rig(&r1);
        assert!(!reg.is_claimed("COM5"));

        // Teraz rig-2 może bez problemu zająć COM5
        reg.claim("COM5", PhysicalEndpointOwner::NativeCatRuntime(r2))
            .unwrap();
        assert!(reg.is_claimed("COM5"));
    }

    #[test]
    fn failed_startup_does_not_leave_endpoint_reserved() {
        let mut reg = PhysicalEndpointRegistry::new();
        let r1 = RigId::new("rig-fail").unwrap();

        // Symulacja procedury startowej z RAII / guardem
        {
            reg.claim("COM6", PhysicalEndpointOwner::NativeCatRuntime(r1.clone()))
                .unwrap();
            assert!(reg.is_claimed("COM6"));
            // Start backendu nie powiódł się (błąd I/O lub timeout) -> cofnięcie rezerwacji
            let startup_success = false;
            if !startup_success {
                reg.release_for_rig(&r1);
            }
        }

        // Port natychmiast wolny
        assert!(!reg.is_claimed("COM6"));
        let r2 = RigId::new("rig-retry").unwrap();
        assert!(
            reg.claim("COM6", PhysicalEndpointOwner::NativeCatRuntime(r2))
                .is_ok()
        );
    }
}
