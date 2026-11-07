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
    if ctx.has_msi_registry_entry || base_dir.join(".splogbook-msi-marker").exists() {
        detected_types.push(InstallationType::WindowsMsi);
    }
    if ctx.has_inno_uninstall_marker
        || base_dir.join("unins000.exe").exists()
        || base_dir.join(".splogbook-inno-marker").exists()
    {
        detected_types.push(InstallationType::WindowsInnoSetup);
    }
    if base_dir.join(".splogbook-portable").exists() {
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
}
