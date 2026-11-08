// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Model klasyfikacji instalacji SPLogbook i trybów zarządzania (M3-A)

use crate::cloud::updater::UpdaterError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Typ instalacji SPLogbook na platformie docelowej.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstallationType {
    /// Windows: aplikacja przenośna w archiwum ZIP.
    WindowsPortableZip,
    /// Windows: pakiet instalatora Windows Installer (MSI) utworzony przez WiX.
    WindowsMsi,
    /// Windows: instalator interaktywny EXE utworzony przez Inno Setup.
    WindowsInnoSetup,
    /// Linux: archiwum przenośne tar.gz w katalogu domowym użytkownika.
    LinuxPortableTarGz,
    /// Linux: samodzielny obraz wykonywalny AppImage.
    LinuxAppImage,
    /// Linux: pakiet dystrybucyjny DEB (Debian/Ubuntu/Mint) zarządzany przez APT.
    LinuxDeb,
    /// Linux: pakiet dystrybucyjny RPM (Fedora/RHEL/openSUSE) zarządzany przez DNF.
    LinuxRpm,
    /// Linux: pakiet kontenerowy Flatpak uruchomiony w sandboxie.
    LinuxFlatpak,
    /// Instalacja nierozpoznana, ścieżka sieciowa lub system plików tylko do odczytu (fail-closed).
    UnknownInstallation,
}

impl InstallationType {
    /// Zwraca canonical identifier tekstowy typu instalacji.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WindowsPortableZip => "windows-portable-zip",
            Self::WindowsMsi => "windows-msi",
            Self::WindowsInnoSetup => "windows-inno-exe",
            Self::LinuxPortableTarGz => "linux-portable-tar-gz",
            Self::LinuxAppImage => "linux-appimage",
            Self::LinuxDeb => "linux-deb",
            Self::LinuxRpm => "linux-rpm",
            Self::LinuxFlatpak => "linux-flatpak",
            Self::UnknownInstallation => "unknown-installation",
        }
    }

    /// Określa tryb zarządzania aktualizacją dla danego formatu.
    pub fn management_mode(&self) -> ManagementMode {
        match self {
            Self::WindowsPortableZip | Self::LinuxPortableTarGz | Self::LinuxAppImage => {
                ManagementMode::SelfManaged
            }
            Self::WindowsMsi
            | Self::WindowsInnoSetup
            | Self::LinuxDeb
            | Self::LinuxRpm
            | Self::LinuxFlatpak => ManagementMode::SystemManaged,
            Self::UnknownInstallation => ManagementMode::UnsupportedOrUnknown,
        }
    }

    /// Czy instalacja jest zarządzana samodzielnie przez SPLogbook.
    pub fn is_self_managed(&self) -> bool {
        self.management_mode() == ManagementMode::SelfManaged
    }

    /// Czy instalacja jest zarządzana przez instalator systemowy lub menedżer pakietów.
    pub fn is_system_managed(&self) -> bool {
        self.management_mode() == ManagementMode::SystemManaged
    }

    /// Bezpieczne parsowanie typu instalacji z tekstu (fail-closed na nieznanych wartościach).
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "windows-portable-zip" | "windows_portable_zip" | "portable-zip" => {
                Self::WindowsPortableZip
            }
            "windows-msi" | "windows_msi" | "msi" => Self::WindowsMsi,
            "windows-inno-exe" | "windows_inno_exe" | "inno-setup" | "inno" => {
                Self::WindowsInnoSetup
            }
            "linux-portable-tar-gz" | "linux_portable_tar_gz" | "portable-tar-gz" | "tar.gz" => {
                Self::LinuxPortableTarGz
            }
            "linux-appimage" | "linux_appimage" | "appimage" => Self::LinuxAppImage,
            "linux-deb" | "linux_deb" | "deb" => Self::LinuxDeb,
            "linux-rpm" | "linux_rpm" | "rpm" => Self::LinuxRpm,
            "linux-flatpak" | "linux_flatpak" | "flatpak" => Self::LinuxFlatpak,
            _ => Self::UnknownInstallation,
        }
    }
}

impl std::fmt::Display for InstallationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Tryb zarządzania cyklem życia i aktualizacją programu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManagementMode {
    /// Auto-updater SPLogbook odpowiada za pełną orkiestrację na poziomie systemu plików.
    SelfManaged,
    /// Aktualizacja jest delegowana do menedżera pakietów lub instalatora systemowego.
    SystemManaged,
    /// Typ nierozpoznany lub niewspierany — operacje modyfikacji są zablokowane (fail-closed).
    UnsupportedOrUnknown,
}

impl ManagementMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SelfManaged => "SelfManaged",
            Self::SystemManaged => "SystemManaged",
            Self::UnsupportedOrUnknown => "UnsupportedOrUnknown",
        }
    }
}

impl std::fmt::Display for ManagementMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Stały identyfikator GUID aplikacji dla Windows User Setup (Inno Setup)
pub const WINDOWS_INNO_SETUP_APP_ID: &str = "{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}";

/// Stały identyfikator UpgradeCode rodziny instalatora Windows System Setup (MSI)
pub const WINDOWS_MSI_UPGRADE_CODE: &str = "{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}";

/// Spakowany GUID (Packed GUID) w formacie Darwin/Windows Installer dla WINDOWS_MSI_UPGRADE_CODE
pub const WINDOWS_MSI_UPGRADE_CODE_PACKED: &str = "0B205E981ED3B4A9A9B4A9E189E502B0";

/// Nazwa kanonicznego pliku markera instalacji Inno Setup
pub const WINDOWS_INNO_MARKER_FILENAME: &str = ".splogbook-inno-marker";

/// Nazwa kanonicznego pliku markera instalacji MSI
pub const WINDOWS_MSI_MARKER_FILENAME: &str = ".splogbook-msi-marker";

/// Zawartość strukturalna markera instalacji Windows System Setup (MSI)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsMsiMarker {
    pub marker_version: String,
    pub installation_type: String,
    pub management_mode: String,
    pub package_version: String,
    pub upgrade_code: String,
    pub product_code: String,
    pub installation_scope: String,
    pub installation_root: String,
    pub build_commit: String,
    pub installer_engine: String,
}

impl WindowsMsiMarker {
    pub fn new(package_version: &str, installation_root: &Path, build_commit: &str) -> Self {
        Self {
            marker_version: "1.0".to_string(),
            installation_type: "WindowsMsi".to_string(),
            management_mode: "SystemManaged".to_string(),
            package_version: package_version.to_string(),
            upgrade_code: WINDOWS_MSI_UPGRADE_CODE.to_string(),
            product_code: "*".to_string(),
            installation_scope: "PerMachine".to_string(),
            installation_root: installation_root.to_string_lossy().to_string(),
            build_commit: build_commit.to_string(),
            installer_engine: "WindowsInstallerWiX".to_string(),
        }
    }

    /// Waliduje poprawność i spójność markera MSI.
    pub fn validate(&self) -> Result<(), UpdaterError> {
        if self.marker_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker MSI: pusty marker_version".into(),
            ));
        }
        if self.installation_type != "WindowsMsi" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker MSI: niezgodny installation_type '{}' (oczekiwano 'WindowsMsi')",
                self.installation_type
            )));
        }
        if self.management_mode != "SystemManaged" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker MSI: niezgodny management_mode '{}' (oczekiwano 'SystemManaged')",
                self.management_mode
            )));
        }
        if self.upgrade_code != WINDOWS_MSI_UPGRADE_CODE {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker MSI: niezgodny upgrade_code '{}' (oczekiwano '{}')",
                self.upgrade_code, WINDOWS_MSI_UPGRADE_CODE
            )));
        }
        if self.installation_scope != "PerMachine" && self.installation_scope != "AllUsers" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker MSI: niezgodny installation_scope '{}' (oczekiwano 'PerMachine' lub 'AllUsers')",
                self.installation_scope
            )));
        }
        if self.installer_engine != "WindowsInstallerWiX" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker MSI: niezgodny installer_engine '{}' (oczekiwano 'WindowsInstallerWiX')",
                self.installer_engine
            )));
        }
        if self.package_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker MSI: pusty package_version".into(),
            ));
        }
        if self.build_commit.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker MSI: pusty build_commit".into(),
            ));
        }
        Ok(())
    }

    /// Odczytuje i waliduje marker z podanego katalogu instalacji.
    pub fn read_from_dir(base_dir: &Path) -> Result<Self, UpdaterError> {
        let marker_path = base_dir.join(WINDOWS_MSI_MARKER_FILENAME);
        if !marker_path.exists() {
            return Err(UpdaterError::InstallationDetection(format!(
                "Brak pliku markera MSI w {}",
                marker_path.display()
            )));
        }
        let data = std::fs::read_to_string(&marker_path)
            .map_err(|e| UpdaterError::Io(format!("Błąd odczytu markera MSI: {e}")))?;
        let marker: Self = serde_json::from_str(&data).map_err(|e| {
            UpdaterError::InstallationDetection(format!("Błąd parsowania markera MSI JSON: {e}"))
        })?;
        marker.validate()?;
        Ok(marker)
    }
}

/// Konwertuje wersję SemVer (np. "1.3.0-alpha.1") na zgodną ze specyfikacją Windows Installer wersję ProductVersion ("1.3.0").
pub fn semver_to_msi_product_version(semver_str: &str) -> Result<String, UpdaterError> {
    let clean = semver_str.trim();
    if clean.is_empty() {
        return Err(UpdaterError::SafetyGate(
            "Pusty ciąg wersji SemVer do mapowania MSI".to_string(),
        ));
    }
    let version_part = clean.split(['-', '+']).next().unwrap_or(clean);
    let parts: Vec<&str> = version_part.split('.').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return Err(UpdaterError::SafetyGate(format!(
            "Niepoprawny format wersji dla MSI ProductVersion: {clean}"
        )));
    }
    for p in &parts {
        if p.parse::<u16>().is_err() {
            return Err(UpdaterError::SafetyGate(format!(
                "Niepoprawny człon liczbowy wersji MSI: '{p}' w {clean}"
            )));
        }
    }
    if parts.len() == 2 {
        Ok(format!("{}.{}.0", parts[0], parts[1]))
    } else if parts.len() == 3 {
        Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
    } else {
        Ok(format!(
            "{}.{}.{}.{}",
            parts[0], parts[1], parts[2], parts[3]
        ))
    }
}

/// Zawartość strukturalna markera instalacji Windows User Setup
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsUserSetupMarker {
    pub marker_version: String,
    pub installation_type: String,
    pub management_mode: String,
    pub package_version: String,
    pub app_id: String,
    pub installation_scope: String,
    pub installation_root: String,
    pub build_commit: String,
    pub installer_engine: String,
}

impl WindowsUserSetupMarker {
    pub fn new(package_version: &str, installation_root: &Path, build_commit: &str) -> Self {
        Self {
            marker_version: "1.0".to_string(),
            installation_type: "WindowsInnoSetup".to_string(),
            management_mode: "SystemManaged".to_string(),
            package_version: package_version.to_string(),
            app_id: WINDOWS_INNO_SETUP_APP_ID.to_string(),
            installation_scope: "CurrentUser".to_string(),
            installation_root: installation_root.to_string_lossy().to_string(),
            build_commit: build_commit.to_string(),
            installer_engine: "InnoSetup".to_string(),
        }
    }

    /// Waliduje poprawność i spójność markera.
    pub fn validate(&self) -> Result<(), UpdaterError> {
        if self.marker_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker User Setup: pusty marker_version".into(),
            ));
        }
        if self.installation_type != "WindowsInnoSetup" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker User Setup: niezgodny installation_type '{}' (oczekiwano 'WindowsInnoSetup')",
                self.installation_type
            )));
        }
        if self.management_mode != "SystemManaged" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker User Setup: niezgodny management_mode '{}' (oczekiwano 'SystemManaged')",
                self.management_mode
            )));
        }
        if self.app_id != WINDOWS_INNO_SETUP_APP_ID {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker User Setup: niezgodny app_id '{}' (oczekiwano '{}')",
                self.app_id, WINDOWS_INNO_SETUP_APP_ID
            )));
        }
        if self.installation_scope != "CurrentUser" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker User Setup: niezgodny installation_scope '{}' (oczekiwano 'CurrentUser')",
                self.installation_scope
            )));
        }
        if self.installer_engine != "InnoSetup" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker User Setup: niezgodny installer_engine '{}' (oczekiwano 'InnoSetup')",
                self.installer_engine
            )));
        }
        if self.package_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker User Setup: pusty package_version".into(),
            ));
        }
        if self.build_commit.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker User Setup: pusty build_commit".into(),
            ));
        }
        Ok(())
    }

    /// Odczytuje i waliduje marker z podanego katalogu instalacji.
    pub fn read_from_dir(base_dir: &Path) -> Result<Self, UpdaterError> {
        let marker_path = base_dir.join(WINDOWS_INNO_MARKER_FILENAME);
        if !marker_path.exists() {
            return Err(UpdaterError::InstallationDetection(format!(
                "Brak pliku markera User Setup w {}",
                marker_path.display()
            )));
        }
        let data = std::fs::read_to_string(&marker_path)
            .map_err(|e| UpdaterError::Io(format!("Błąd odczytu markera User Setup: {e}")))?;
        let marker: Self = serde_json::from_str(&data).map_err(|e| {
            UpdaterError::InstallationDetection(format!(
                "Błąd parsowania markera User Setup JSON: {e}"
            ))
        })?;
        marker.validate()?;
        Ok(marker)
    }
}

/// Nazwa kanonicznego pliku markera instalacji Linux Portable tar.gz
pub const LINUX_PORTABLE_MARKER_FILENAME: &str = ".splogbook-linux-portable-marker";

/// Zawartość strukturalna markera instalacji Linux Portable tar.gz (M3-F)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxPortableMarker {
    pub marker_version: String,
    pub installation_type: String,
    pub management_mode: String,
    pub package_version: String,
    pub installation_root: String,
    pub build_commit: String,
    pub target_os: String,
    pub target_arch: String,
    pub libc_family: String,
    pub archive_format: String,
    pub resolved_minimum_kernel: String,
    pub build_rust_version: String,
    pub created_by: String,
}

impl LinuxPortableMarker {
    pub fn new(
        package_version: &str,
        installation_root: &Path,
        build_commit: &str,
        resolved_minimum_kernel: &str,
        build_rust_version: &str,
    ) -> Self {
        Self {
            marker_version: "1.0".to_string(),
            installation_type: "LinuxPortableTarGz".to_string(),
            management_mode: "SelfManaged".to_string(),
            package_version: package_version.to_string(),
            installation_root: installation_root.to_string_lossy().to_string(),
            build_commit: build_commit.to_string(),
            target_os: "linux".to_string(),
            target_arch: "x86_64".to_string(),
            libc_family: "glibc".to_string(),
            archive_format: "tar.gz".to_string(),
            resolved_minimum_kernel: resolved_minimum_kernel.to_string(),
            build_rust_version: build_rust_version.to_string(),
            created_by: "splogbook-packaging-m3-f".to_string(),
        }
    }

    /// Waliduje poprawność i spójność markera Linux Portable.
    pub fn validate(&self) -> Result<(), UpdaterError> {
        if self.marker_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker Linux Portable: pusty marker_version".into(),
            ));
        }
        if self.installation_type != "LinuxPortableTarGz" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker Linux Portable: niezgodny installation_type '{}' (oczekiwano 'LinuxPortableTarGz')",
                self.installation_type
            )));
        }
        if self.management_mode != "SelfManaged" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker Linux Portable: niezgodny management_mode '{}' (oczekiwano 'SelfManaged')",
                self.management_mode
            )));
        }
        if self.package_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker Linux Portable: pusty package_version".into(),
            ));
        }
        if self.installation_root.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker Linux Portable: pusty installation_root".into(),
            ));
        }
        if self.build_commit.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker Linux Portable: pusty build_commit".into(),
            ));
        }
        if self.target_os != "linux" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker Linux Portable: niezgodny target_os '{}' (oczekiwano 'linux')",
                self.target_os
            )));
        }
        if self.target_arch != "x86_64" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker Linux Portable: niezgodny target_arch '{}' (oczekiwano 'x86_64')",
                self.target_arch
            )));
        }
        if self.libc_family != "glibc" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker Linux Portable: niezgodny libc_family '{}' (oczekiwano 'glibc')",
                self.libc_family
            )));
        }
        if self.archive_format != "tar.gz" {
            return Err(UpdaterError::InstallationDetection(format!(
                "Marker Linux Portable: niezgodny archive_format '{}' (oczekiwano 'tar.gz')",
                self.archive_format
            )));
        }
        if self.resolved_minimum_kernel.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker Linux Portable: pusty resolved_minimum_kernel".into(),
            ));
        }
        if self.build_rust_version.trim().is_empty() {
            return Err(UpdaterError::InstallationDetection(
                "Marker Linux Portable: pusty build_rust_version".into(),
            ));
        }
        Ok(())
    }

    /// Odczytuje i waliduje marker z podanego katalogu instalacji.
    pub fn read_from_dir(base_dir: &Path) -> Result<Self, UpdaterError> {
        let marker_path = base_dir.join(LINUX_PORTABLE_MARKER_FILENAME);
        if !marker_path.exists() {
            return Err(UpdaterError::InstallationDetection(format!(
                "Brak pliku markera Linux Portable w {}",
                marker_path.display()
            )));
        }
        let data = std::fs::read_to_string(&marker_path)
            .map_err(|e| UpdaterError::Io(format!("Błąd odczytu markera Linux Portable: {e}")))?;
        let marker: Self = serde_json::from_str(&data).map_err(|e| {
            UpdaterError::InstallationDetection(format!(
                "Błąd parsowania markera Linux Portable JSON: {e}"
            ))
        })?;
        marker.validate()?;
        Ok(marker)
    }
}

/// Kontekst detekcji instalacji — umożliwia wstrzykiwanie środowiska i markerów testowych
/// bez konieczności odwoływania się do globalnego stanu OS.
#[derive(Debug, Clone, Default)]
pub struct InstallationDetectionContext {
    pub explicit_override: Option<InstallationType>,
    pub markers: Vec<String>,
    pub is_flatpak_sandbox: bool,
    pub is_appimage_env: bool,
    pub has_dpkg_manifest: bool,
    pub has_rpm_manifest: bool,
    pub has_msi_registry_entry: bool,
    pub has_inno_uninstall_marker: bool,
}

/// Wykrywa typ instalacji na podstawie wiarygodnych dowodów i markerów.
///
/// Zasada bezpieczeństwa: Sama obecność ścieżki (np. `/usr/bin`) lub rozszerzenia binarki (`.exe`)
/// NIE stanowi wystarczającego dowodu konkretnego formatu pakietu.
/// W razie braku jednoznacznych markerów lub w razie konfliktów markerów procedura zwraca fail-closed.
pub fn detect_installation_with_context(
    base_dir: &Path,
    ctx: &InstallationDetectionContext,
) -> Result<InstallationType, UpdaterError> {
    if let Some(explicit) = ctx.explicit_override {
        return Ok(explicit);
    }

    // Sprawdzenie konfliktów markerów
    let mut detected_types = Vec::new();

    if ctx.is_flatpak_sandbox || base_dir.join(".flatpak-info").exists() {
        detected_types.push(InstallationType::LinuxFlatpak);
    }
    if ctx.is_appimage_env || std::env::var_os("APPIMAGE").is_some() {
        detected_types.push(InstallationType::LinuxAppImage);
    }
    if ctx.has_dpkg_manifest || base_dir.join(".splogbook-deb-marker").exists() {
        detected_types.push(InstallationType::LinuxDeb);
    }
    if ctx.has_rpm_manifest || base_dir.join(".splogbook-rpm-marker").exists() {
        detected_types.push(InstallationType::LinuxRpm);
    }
    let msi_marker_file = base_dir.join(WINDOWS_MSI_MARKER_FILENAME);
    if msi_marker_file.exists() {
        // Weryfikacja strukturalna markera MSI - uszkodzony marker = błąd fail-closed
        WindowsMsiMarker::read_from_dir(base_dir)?;
        detected_types.push(InstallationType::WindowsMsi);
    } else if ctx.has_msi_registry_entry {
        detected_types.push(InstallationType::WindowsMsi);
    }
    let inno_marker_file = base_dir.join(WINDOWS_INNO_MARKER_FILENAME);
    if inno_marker_file.exists() {
        // Weryfikacja strukturalna markera - uszkodzony marker = błąd fail-closed
        WindowsUserSetupMarker::read_from_dir(base_dir)?;
        detected_types.push(InstallationType::WindowsInnoSetup);
    } else if ctx.has_inno_uninstall_marker || base_dir.join("unins000.exe").exists() {
        detected_types.push(InstallationType::WindowsInnoSetup);
    }
    let linux_portable_marker_file = base_dir.join(LINUX_PORTABLE_MARKER_FILENAME);
    if linux_portable_marker_file.exists() {
        // Weryfikacja strukturalna markera Linux Portable - uszkodzony marker = błąd fail-closed
        LinuxPortableMarker::read_from_dir(base_dir)?;
        detected_types.push(InstallationType::LinuxPortableTarGz);
    } else if base_dir.join(".splogbook-portable").exists() {
        #[cfg(target_os = "windows")]
        detected_types.push(InstallationType::WindowsPortableZip);
        #[cfg(not(target_os = "windows"))]
        detected_types.push(InstallationType::LinuxPortableTarGz);
    }

    if detected_types.len() > 1 {
        return Err(UpdaterError::ConflictingInstallationMarkers(format!(
            "Wykryto sprzeczne markery instalacji ({:?}) w katalogu {}",
            detected_types,
            base_dir.display()
        )));
    }

    if let Some(single) = detected_types.into_iter().next() {
        return Ok(single);
    }

    // Domyślny fallback: jeśli brak jakichkolwiek markerów, instalacja jest nierozpoznana (fail-closed)
    Ok(InstallationType::UnknownInstallation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_zip_is_self_managed() {
        assert_eq!(
            InstallationType::WindowsPortableZip.management_mode(),
            ManagementMode::SelfManaged
        );
        assert!(InstallationType::WindowsPortableZip.is_self_managed());
        assert!(!InstallationType::WindowsPortableZip.is_system_managed());
    }

    #[test]
    fn portable_tar_gz_is_self_managed() {
        assert_eq!(
            InstallationType::LinuxPortableTarGz.management_mode(),
            ManagementMode::SelfManaged
        );
        assert!(InstallationType::LinuxPortableTarGz.is_self_managed());
        assert!(!InstallationType::LinuxPortableTarGz.is_system_managed());
    }

    #[test]
    fn appimage_is_self_managed() {
        assert_eq!(
            InstallationType::LinuxAppImage.management_mode(),
            ManagementMode::SelfManaged
        );
        assert!(InstallationType::LinuxAppImage.is_self_managed());
        assert!(!InstallationType::LinuxAppImage.is_system_managed());
    }

    #[test]
    fn msi_is_system_managed() {
        assert_eq!(
            InstallationType::WindowsMsi.management_mode(),
            ManagementMode::SystemManaged
        );
        assert!(InstallationType::WindowsMsi.is_system_managed());
        assert!(!InstallationType::WindowsMsi.is_self_managed());
    }

    #[test]
    fn inno_setup_is_system_managed() {
        assert_eq!(
            InstallationType::WindowsInnoSetup.management_mode(),
            ManagementMode::SystemManaged
        );
        assert!(InstallationType::WindowsInnoSetup.is_system_managed());
        assert!(!InstallationType::WindowsInnoSetup.is_self_managed());
    }

    #[test]
    fn deb_is_system_managed() {
        assert_eq!(
            InstallationType::LinuxDeb.management_mode(),
            ManagementMode::SystemManaged
        );
        assert!(InstallationType::LinuxDeb.is_system_managed());
        assert!(!InstallationType::LinuxDeb.is_self_managed());
    }

    #[test]
    fn rpm_is_system_managed() {
        assert_eq!(
            InstallationType::LinuxRpm.management_mode(),
            ManagementMode::SystemManaged
        );
        assert!(InstallationType::LinuxRpm.is_system_managed());
        assert!(!InstallationType::LinuxRpm.is_self_managed());
    }

    #[test]
    fn flatpak_is_system_managed() {
        assert_eq!(
            InstallationType::LinuxFlatpak.management_mode(),
            ManagementMode::SystemManaged
        );
        assert!(InstallationType::LinuxFlatpak.is_system_managed());
        assert!(!InstallationType::LinuxFlatpak.is_self_managed());
    }

    #[test]
    fn unknown_installation_is_fail_closed() {
        assert_eq!(
            InstallationType::UnknownInstallation.management_mode(),
            ManagementMode::UnsupportedOrUnknown
        );
        assert!(!InstallationType::UnknownInstallation.is_self_managed());
        assert!(!InstallationType::UnknownInstallation.is_system_managed());

        let temp = tempfile::tempdir().unwrap();
        let ctx = InstallationDetectionContext::default();
        let detected = detect_installation_with_context(temp.path(), &ctx).unwrap();
        assert_eq!(detected, InstallationType::UnknownInstallation);
    }

    #[test]
    fn file_extension_alone_does_not_prove_installation_type() {
        let temp = tempfile::tempdir().unwrap();
        // Tworzymy binarkę .exe w katalogu bez markerów
        std::fs::write(temp.path().join("SPLogbook.exe"), b"DUMMY_EXE").unwrap();
        let ctx = InstallationDetectionContext::default();
        let detected = detect_installation_with_context(temp.path(), &ctx).unwrap();
        // Rozszerzenie .exe bez markerów instalacji zwraca UnknownInstallation (fail-closed)
        assert_eq!(detected, InstallationType::UnknownInstallation);
    }

    #[test]
    fn system_path_alone_does_not_prove_package_type() {
        let temp = tempfile::tempdir().unwrap();
        let fake_usr_bin = temp.path().join("usr").join("bin");
        std::fs::create_dir_all(&fake_usr_bin).unwrap();
        std::fs::write(fake_usr_bin.join("splogbook"), b"DUMMY_BIN").unwrap();

        let ctx = InstallationDetectionContext::default();
        let detected = detect_installation_with_context(&fake_usr_bin, &ctx).unwrap();
        // Sama ścieżka /usr/bin bez markerów DEB/RPM/Flatpak nie dowodzi typu pakietu
        assert_eq!(detected, InstallationType::UnknownInstallation);
    }

    #[test]
    fn conflicting_installation_markers_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".splogbook-deb-marker"), b"DEB").unwrap();
        std::fs::write(temp.path().join(".splogbook-rpm-marker"), b"RPM").unwrap();

        let ctx = InstallationDetectionContext::default();
        let res = detect_installation_with_context(temp.path(), &ctx);
        assert!(matches!(
            res,
            Err(UpdaterError::ConflictingInstallationMarkers(_))
        ));
    }

    #[test]
    fn inno_user_setup_marker_serialization_and_validation() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsUserSetupMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        );
        assert!(marker.validate().is_ok());

        let json = serde_json::to_string_pretty(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_INNO_MARKER_FILENAME), json).unwrap();

        let loaded = WindowsUserSetupMarker::read_from_dir(temp.path()).unwrap();
        assert_eq!(loaded, marker);
        assert_eq!(loaded.app_id, WINDOWS_INNO_SETUP_APP_ID);
        assert_eq!(loaded.installation_scope, "CurrentUser");
        assert_eq!(loaded.installer_engine, "InnoSetup");
        assert_eq!(loaded.management_mode, "SystemManaged");
        assert_eq!(loaded.installation_type, "WindowsInnoSetup");
    }

    #[test]
    fn inno_user_setup_corrupt_marker_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join(WINDOWS_INNO_MARKER_FILENAME),
            b"{ corrupt json ...",
        )
        .unwrap();

        let ctx = InstallationDetectionContext::default();
        let res = detect_installation_with_context(temp.path(), &ctx);
        assert!(matches!(res, Err(UpdaterError::InstallationDetection(_))));
    }

    #[test]
    fn inno_user_setup_detected_with_valid_marker() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsUserSetupMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        );
        let json = serde_json::to_string(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_INNO_MARKER_FILENAME), json).unwrap();

        let ctx = InstallationDetectionContext::default();
        let detected = detect_installation_with_context(temp.path(), &ctx).unwrap();
        assert_eq!(detected, InstallationType::WindowsInnoSetup);
        assert!(detected.is_system_managed());
        assert!(!detected.is_self_managed());
    }

    #[test]
    fn inno_user_setup_conflict_with_portable_zip_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsUserSetupMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        );
        let json = serde_json::to_string(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_INNO_MARKER_FILENAME), json).unwrap();
        std::fs::write(temp.path().join(".splogbook-portable"), b"PORTABLE").unwrap();

        let ctx = InstallationDetectionContext::default();
        let res = detect_installation_with_context(temp.path(), &ctx);
        assert!(matches!(
            res,
            Err(UpdaterError::ConflictingInstallationMarkers(_))
        ));
    }

    #[test]
    fn inno_user_setup_conflict_with_msi_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsUserSetupMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        );
        let json = serde_json::to_string(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_INNO_MARKER_FILENAME), json).unwrap();

        let msi_marker = WindowsMsiMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        );
        let msi_json = serde_json::to_string(&msi_marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_MSI_MARKER_FILENAME), msi_json).unwrap();

        let ctx = InstallationDetectionContext::default();
        let res = detect_installation_with_context(temp.path(), &ctx);
        assert!(matches!(
            res,
            Err(UpdaterError::ConflictingInstallationMarkers(_))
        ));
    }

    #[test]
    fn msi_upgrade_code_is_stable_and_distinct_from_inno() {
        assert_eq!(
            WINDOWS_MSI_UPGRADE_CODE,
            "{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}"
        );
        assert_ne!(WINDOWS_MSI_UPGRADE_CODE, WINDOWS_INNO_SETUP_APP_ID);
        assert_eq!(
            WINDOWS_MSI_UPGRADE_CODE_PACKED,
            "0B205E981ED3B4A9A9B4A9E189E502B0"
        );
    }

    #[test]
    fn msi_marker_serialization_and_validation() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsMsiMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
        );
        assert_eq!(marker.installation_type, "WindowsMsi");
        assert_eq!(marker.management_mode, "SystemManaged");
        assert_eq!(marker.upgrade_code, WINDOWS_MSI_UPGRADE_CODE);
        assert_eq!(marker.product_code, "*");
        assert_eq!(marker.installation_scope, "PerMachine");
        assert_eq!(marker.installer_engine, "WindowsInstallerWiX");

        let json = serde_json::to_string_pretty(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_MSI_MARKER_FILENAME), json).unwrap();

        let loaded = WindowsMsiMarker::read_from_dir(temp.path()).unwrap();
        assert_eq!(loaded, marker);
    }

    #[test]
    fn msi_product_version_mapping() {
        assert_eq!(
            semver_to_msi_product_version("1.3.0-alpha.1").unwrap(),
            "1.3.0"
        );
        assert_eq!(semver_to_msi_product_version("1.2.0").unwrap(), "1.2.0");
        assert_eq!(
            semver_to_msi_product_version("2.0.0-rc.3+build.42").unwrap(),
            "2.0.0"
        );
        assert_eq!(semver_to_msi_product_version("10.5").unwrap(), "10.5.0");
        assert!(semver_to_msi_product_version("").is_err());
        assert!(semver_to_msi_product_version("invalid").is_err());
    }

    #[test]
    fn windows_msi_detected_with_valid_marker() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsMsiMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
        );
        let json = serde_json::to_string(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_MSI_MARKER_FILENAME), json).unwrap();

        let ctx = InstallationDetectionContext::default();
        let detected = detect_installation_with_context(temp.path(), &ctx).unwrap();
        assert_eq!(detected, InstallationType::WindowsMsi);
        assert!(detected.is_system_managed());
        assert!(!detected.is_self_managed());
    }

    #[test]
    fn windows_msi_conflict_with_portable_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let marker = WindowsMsiMarker::new(
            "1.3.0-alpha.1",
            temp.path(),
            "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
        );
        let json = serde_json::to_string(&marker).unwrap();
        std::fs::write(temp.path().join(WINDOWS_MSI_MARKER_FILENAME), json).unwrap();
        std::fs::write(temp.path().join(".splogbook-portable"), b"PORTABLE").unwrap();

        let ctx = InstallationDetectionContext::default();
        let res = detect_installation_with_context(temp.path(), &ctx);
        assert!(matches!(
            res,
            Err(UpdaterError::ConflictingInstallationMarkers(_))
        ));
    }
}
