// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Model możliwości (Capabilities) formatów instalacji SPLogbook (M3-A)

use crate::cloud::updater::installation::InstallationType;
use serde::{Deserialize, Serialize};

/// Zestaw możliwości operacyjnych i cech danego formatu instalacji.
///
/// Centralny kontrakt architektoniczny zapobiegający rozpraszaniu logiki
/// decyzyjnej w niezależnych blokach `match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstallationCapabilities {
    /// Czy format wspiera bezpośrednią podmianę plików w trybie SelfManaged.
    pub supports_self_apply: bool,
    /// Czy format posiada wbudowany natywny mechanizm rollbacku instalatora OS (np. MSI transaction).
    pub supports_native_rollback: bool,
    /// Czy format wspiera funkcję naprawy instalacji (Repair).
    pub supports_repair: bool,
    /// Czy format wspiera funkcję deinstalacji (Uninstall).
    pub supports_uninstall: bool,
    /// Czy format wspiera potwierdzenie zdrowia (.health_ok powiązany z attempt_id).
    pub supports_health_confirmation: bool,
    /// Czy format wymaga delegacji do zewnętrznego menedżera pakietów lub instalatora.
    pub requires_delegation: bool,
    /// Czy instalator natywny może wymagać eskalacji uprawnień przez systemowy UAC/polkit.
    pub requires_elevation_by_native_installer: bool,
    /// Czy transakcja w tym formacie może być anulowana przez użytkownika przed stanem Applying.
    pub supports_in_app_cancellation_before_apply: bool,
    /// Czy aplikacja tworzy i egzekwuje własną kopię zapasową w .rollback.
    pub supports_sp_logbook_rollback: bool,
    /// Czy możliwe jest monitorowanie procesu przez menedżer pakietów (APT/DNF/Flatpak).
    pub supports_package_manager_monitoring: bool,
}

impl InstallationCapabilities {
    /// Tworzy zestaw możliwości dla danego typu instalacji zgodnie ze specyfikacją DESIGN 3.0 C.
    pub fn for_installation_type(inst_type: InstallationType) -> Self {
        match inst_type {
            InstallationType::WindowsPortableZip
            | InstallationType::LinuxPortableTarGz
            | InstallationType::LinuxAppImage => Self {
                supports_self_apply: true,
                supports_native_rollback: false,
                supports_repair: false,
                supports_uninstall: false,
                supports_health_confirmation: true,
                requires_delegation: false,
                requires_elevation_by_native_installer: false,
                supports_in_app_cancellation_before_apply: true,
                supports_sp_logbook_rollback: true,
                supports_package_manager_monitoring: false,
            },
            InstallationType::WindowsMsi => Self {
                supports_self_apply: false,
                supports_native_rollback: true,
                supports_repair: true,
                supports_uninstall: true,
                supports_health_confirmation: false,
                requires_delegation: true,
                requires_elevation_by_native_installer: true,
                supports_in_app_cancellation_before_apply: true,
                supports_sp_logbook_rollback: false,
                supports_package_manager_monitoring: false,
            },
            InstallationType::WindowsInnoSetup => Self {
                supports_self_apply: false,
                supports_native_rollback: false,
                supports_repair: false,
                supports_uninstall: true,
                supports_health_confirmation: false,
                requires_delegation: true,
                requires_elevation_by_native_installer: true,
                supports_in_app_cancellation_before_apply: true,
                supports_sp_logbook_rollback: false,
                supports_package_manager_monitoring: false,
            },
            InstallationType::LinuxDeb | InstallationType::LinuxRpm => Self {
                supports_self_apply: false,
                supports_native_rollback: true,
                supports_repair: false,
                supports_uninstall: true,
                supports_health_confirmation: false,
                requires_delegation: true,
                requires_elevation_by_native_installer: true,
                supports_in_app_cancellation_before_apply: true,
                supports_sp_logbook_rollback: false,
                supports_package_manager_monitoring: true,
            },
            InstallationType::LinuxFlatpak => Self {
                supports_self_apply: false,
                supports_native_rollback: true,
                supports_repair: false,
                supports_uninstall: true,
                supports_health_confirmation: false,
                requires_delegation: true,
                requires_elevation_by_native_installer: false,
                supports_in_app_cancellation_before_apply: true,
                supports_sp_logbook_rollback: false,
                supports_package_manager_monitoring: true,
            },
            InstallationType::UnknownInstallation => Self {
                // Fail-closed: brak jakichkolwiek uprawnień ani możliwości wykonawczych
                supports_self_apply: false,
                supports_native_rollback: false,
                supports_repair: false,
                supports_uninstall: false,
                supports_health_confirmation: false,
                requires_delegation: false,
                requires_elevation_by_native_installer: false,
                supports_in_app_cancellation_before_apply: false,
                supports_sp_logbook_rollback: false,
                supports_package_manager_monitoring: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_matrix_capabilities_verification() {
        // Portable ZIP
        let zip =
            InstallationCapabilities::for_installation_type(InstallationType::WindowsPortableZip);
        assert!(zip.supports_self_apply);
        assert!(zip.supports_sp_logbook_rollback);
        assert!(!zip.supports_native_rollback);
        assert!(zip.supports_health_confirmation);
        assert!(!zip.requires_delegation);

        // Portable tar.gz
        let targz =
            InstallationCapabilities::for_installation_type(InstallationType::LinuxPortableTarGz);
        assert!(targz.supports_self_apply);
        assert!(targz.supports_sp_logbook_rollback);
        assert!(!targz.supports_native_rollback);
        assert!(targz.supports_health_confirmation);
        assert!(!targz.requires_delegation);

        // AppImage
        let appimage =
            InstallationCapabilities::for_installation_type(InstallationType::LinuxAppImage);
        assert!(appimage.supports_self_apply);
        assert!(appimage.supports_sp_logbook_rollback);
        assert!(appimage.supports_health_confirmation);
        assert!(!appimage.requires_delegation);

        // MSI
        let msi = InstallationCapabilities::for_installation_type(InstallationType::WindowsMsi);
        assert!(!msi.supports_self_apply);
        assert!(msi.supports_native_rollback);
        assert!(msi.supports_repair);
        assert!(msi.supports_uninstall);
        assert!(msi.requires_delegation);
        assert!(msi.requires_elevation_by_native_installer);

        // Inno Setup
        let inno =
            InstallationCapabilities::for_installation_type(InstallationType::WindowsInnoSetup);
        assert!(!inno.supports_self_apply);
        assert!(!inno.supports_native_rollback);
        assert!(!inno.supports_repair);
        assert!(inno.supports_uninstall);
        assert!(inno.requires_delegation);
        assert!(inno.requires_elevation_by_native_installer);

        // DEB
        let deb = InstallationCapabilities::for_installation_type(InstallationType::LinuxDeb);
        assert!(!deb.supports_self_apply);
        assert!(deb.requires_delegation);
        assert!(deb.supports_package_manager_monitoring);
        assert!(deb.supports_native_rollback);

        // RPM
        let rpm = InstallationCapabilities::for_installation_type(InstallationType::LinuxRpm);
        assert!(!rpm.supports_self_apply);
        assert!(rpm.requires_delegation);
        assert!(rpm.supports_package_manager_monitoring);
        assert!(rpm.supports_native_rollback);

        // Flatpak
        let flatpak =
            InstallationCapabilities::for_installation_type(InstallationType::LinuxFlatpak);
        assert!(!flatpak.supports_self_apply);
        assert!(flatpak.requires_delegation);
        assert!(flatpak.supports_package_manager_monitoring);
        assert!(!flatpak.requires_elevation_by_native_installer);

        // Unknown (Fail-Closed)
        let unk =
            InstallationCapabilities::for_installation_type(InstallationType::UnknownInstallation);
        assert!(!unk.supports_self_apply);
        assert!(!unk.supports_native_rollback);
        assert!(!unk.supports_repair);
        assert!(!unk.supports_uninstall);
        assert!(!unk.supports_health_confirmation);
        assert!(!unk.requires_delegation);
        assert!(!unk.requires_elevation_by_native_installer);
        assert!(!unk.supports_in_app_cancellation_before_apply);
        assert!(!unk.supports_sp_logbook_rollback);
        assert!(!unk.supports_package_manager_monitoring);
    }
}
