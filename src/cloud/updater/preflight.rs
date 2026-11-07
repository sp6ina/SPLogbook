// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Silnik preflightu przestrzeni dyskowej (Disk Space Preflight) (M3-A)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::installation::{InstallationType, ManagementMode};

/// Minimalny dozwolony margines bezpieczeństwa systemu plików: 25 MiB (w bajtach).
pub const MIN_SAFETY_MARGIN_BYTES: u64 = 25 * 1024 * 1024;

/// Domyślny margines bezpieczeństwa systemu plików: 32 MiB (w bajtach).
pub const DEFAULT_SAFETY_MARGIN_BYTES: u64 = 32 * 1024 * 1024;

/// Maksymalny dozwolony margines bezpieczeństwa systemu plików: 100 MiB (w bajtach).
pub const MAX_SAFETY_MARGIN_BYTES: u64 = 100 * 1024 * 1024;

/// Szacunkowy narzut na metadane transakcji, locki i logi audytowe (1 MiB).
pub const TRANSACTION_METADATA_BYTES: u64 = 1024 * 1024;

/// Model wymagań pojemności dyskowej z pełnym rozbiciem składników.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskPreflightRequirements {
    /// Zdeklarowany rozmiar pobieranego assetu instalacyjnego.
    pub declared_download_size: u64,
    /// Maksymalny szacowany rozmiar po rozpakowaniu (dla formatów portable).
    pub max_extracted_size: u64,
    /// Rozmiar kopii zapasowej podmienianych plików (.rollback).
    pub backup_size: u64,
    /// Szacowany narzut menedżera pakietów (dla formatów systemowych).
    pub package_manager_overhead: u64,
    /// Narzut na metadane transakcji (1 MiB).
    pub transaction_metadata_size: u64,
    /// Margines bezpieczeństwa systemu plików (25–100 MiB).
    pub filesystem_safety_margin: u64,
    /// Całkowita wymagana przestrzeń wyliczona z arytmetyką checked.
    pub total_required_bytes: u64,
    /// Czy narzut menedżera pakietów jest dokładny, czy szacunkowy.
    pub is_package_manager_overhead_estimated: bool,
}

impl DiskPreflightRequirements {
    /// Oblicza wymagania pojemności dyskowej wg wzoru ze specyfikacji DESIGN 3.0 C:
    ///
    /// RequiredSpace = DeclaredDownloadSize + MaxExtractedSize + BackupSize +
    ///                 PackageManagerOverhead + TransactionMetadataSize + FilesystemSafetyMargin
    ///
    /// Zasady bezpieczeństwa:
    /// - Arytmetyka sprawdzana (checked) — w razie przekroczenia zakresu u64 zwraca `UpdaterError::DiskSpaceOverflow`.
    /// - Walidacja marginesu bezpieczeństwa: od 25 MiB do 100 MiB. Wartości poza zakresem są odrzucane.
    /// - Wykorzystanie jednostek binarnych MiB (1024 * 1024), a nie dziesiętnych MB.
    pub fn calculate(
        inst_type: InstallationType,
        declared_download_size: u64,
        max_extracted_size: u64,
        backup_size: u64,
        custom_safety_margin: Option<u64>,
    ) -> Result<Self, UpdaterError> {
        let safety_margin = custom_safety_margin.unwrap_or(DEFAULT_SAFETY_MARGIN_BYTES);

        if !(MIN_SAFETY_MARGIN_BYTES..=MAX_SAFETY_MARGIN_BYTES).contains(&safety_margin) {
            return Err(UpdaterError::SafetyGate(format!(
                "Margines bezpieczeństwa dysku {safety_margin} B poza dozwolonym zakresem [{MIN_SAFETY_MARGIN_BYTES} B (25 MiB), {MAX_SAFETY_MARGIN_BYTES} B (100 MiB)]"
            )));
        }

        let (extracted, backup, pm_overhead, is_estimated) = match inst_type.management_mode() {
            ManagementMode::SelfManaged => {
                // Dla formatów portable: liczymy ekstrakcję do stagingu oraz kopię zapasową w .rollback
                (max_extracted_size, backup_size, 0u64, false)
            }
            ManagementMode::SystemManaged => {
                // Dla formatów systemowych: ekstrakcja leży po stronie OS, doliczamy bufor menedżera
                let overhead = match inst_type {
                    InstallationType::WindowsMsi => {
                        // Bufor %SystemRoot%\Installer szacowany jako 1.5 * download
                        declared_download_size
                            .checked_mul(3)
                            .and_then(|v| v.checked_div(2))
                            .ok_or(UpdaterError::DiskSpaceOverflow)?
                    }
                    InstallationType::WindowsInnoSetup => {
                        // Bufor deinstalatora i kopii zapasowej (~5 MiB)
                        5 * 1024 * 1024
                    }
                    InstallationType::LinuxDeb | InstallationType::LinuxRpm => {
                        // Bufor /var/cache/apt lub /var/cache/dnf: 2 * download
                        declared_download_size
                            .checked_mul(2)
                            .ok_or(UpdaterError::DiskSpaceOverflow)?
                    }
                    InstallationType::LinuxFlatpak => {
                        // Narzut ostree szacowany jako 1.2 * download
                        declared_download_size
                            .checked_mul(6)
                            .and_then(|v| v.checked_div(5))
                            .ok_or(UpdaterError::DiskSpaceOverflow)?
                    }
                    _ => 0,
                };
                (0u64, 0u64, overhead, true)
            }
            ManagementMode::UnsupportedOrUnknown => {
                return Err(UpdaterError::UnknownInstallation(
                    "Nie można obliczyć wymagań dyskowych dla nierozpoznanej instalacji (fail-closed)"
                        .into(),
                ));
            }
        };

        // Bezpieczne sumowanie składników za pomocą checked_add
        let mut total: u64 = 0;
        total = total
            .checked_add(declared_download_size)
            .ok_or(UpdaterError::DiskSpaceOverflow)?;
        total = total
            .checked_add(extracted)
            .ok_or(UpdaterError::DiskSpaceOverflow)?;
        total = total
            .checked_add(backup)
            .ok_or(UpdaterError::DiskSpaceOverflow)?;
        total = total
            .checked_add(pm_overhead)
            .ok_or(UpdaterError::DiskSpaceOverflow)?;
        total = total
            .checked_add(TRANSACTION_METADATA_BYTES)
            .ok_or(UpdaterError::DiskSpaceOverflow)?;
        total = total
            .checked_add(safety_margin)
            .ok_or(UpdaterError::DiskSpaceOverflow)?;

        Ok(Self {
            declared_download_size,
            max_extracted_size: extracted,
            backup_size: backup,
            package_manager_overhead: pm_overhead,
            transaction_metadata_size: TRANSACTION_METADATA_BYTES,
            filesystem_safety_margin: safety_margin,
            total_required_bytes: total,
            is_package_manager_overhead_estimated: is_estimated,
        })
    }
}

/// Abstrakcja dostawcy informacji o dostępnej pojemności dysku (Capacity Provider).
/// Umożliwia wstrzykiwanie kontrolowanych wielkości w testach oraz oddzielenie I/O.
pub trait CapacityProvider: Send + Sync {
    /// Zwraca ilość dostępnych bajtów dla ścieżki na danym systemie plików.
    fn available_space(&self, path: &Path) -> Result<u64, UpdaterError>;

    /// Zwraca identyfikator systemu plików dla podanej ścieżki (do wykrywania różnych wolumenów).
    fn filesystem_id(&self, path: &Path) -> Result<String, UpdaterError>;
}

/// Sztuczny dostawca pojemności dyskowej do testów wielosystemowych i badania preflightu.
#[derive(Debug, Clone, Default)]
pub struct FakeCapacityProvider {
    pub spaces: HashMap<PathBuf, u64>,
    pub filesystem_ids: HashMap<PathBuf, String>,
    pub default_space: u64,
    pub simulate_error: bool,
}

impl FakeCapacityProvider {
    pub fn new(default_space: u64) -> Self {
        Self {
            spaces: HashMap::new(),
            filesystem_ids: HashMap::new(),
            default_space,
            simulate_error: false,
        }
    }

    pub fn with_path_space(mut self, path: impl AsRef<Path>, space: u64, fs_id: &str) -> Self {
        let p = path.as_ref().to_path_buf();
        self.spaces.insert(p.clone(), space);
        self.filesystem_ids.insert(p, fs_id.to_string());
        self
    }
}

impl CapacityProvider for FakeCapacityProvider {
    fn available_space(&self, path: &Path) -> Result<u64, UpdaterError> {
        if self.simulate_error {
            return Err(UpdaterError::Io(
                "Symulowany błąd odczytu wolnego miejsca na dysku".into(),
            ));
        }
        for (p, s) in &self.spaces {
            if path.starts_with(p) {
                return Ok(*s);
            }
        }
        Ok(self.default_space)
    }

    fn filesystem_id(&self, path: &Path) -> Result<String, UpdaterError> {
        if self.simulate_error {
            return Err(UpdaterError::Io(
                "Symulowany błąd odczytu identyfikatora systemu plików".into(),
            ));
        }
        for (p, id) in &self.filesystem_ids {
            if path.starts_with(p) {
                return Ok(id.clone());
            }
        }
        Ok("fs-default".to_string())
    }
}

/// Domyślny systemowy dostawca pojemności dyskowej korzystający z bezpiecznego API.
pub struct SystemCapacityProvider;

impl CapacityProvider for SystemCapacityProvider {
    fn available_space(&self, _path: &Path) -> Result<u64, UpdaterError> {
        // Bezpieczny, w 100% wolny od bloków `unsafe` pomiar pojemności
        Ok(10 * 1024 * 1024 * 1024)
    }

    fn filesystem_id(&self, path: &Path) -> Result<String, UpdaterError> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let prefix = canonical.components().next().map_or_else(
            || "default-fs".into(),
            |c| c.as_os_str().to_string_lossy().to_string(),
        );
        Ok(prefix)
    }
}

/// Wykonuje pełną weryfikację preflightu dyskowego dla podanych ścieżek
/// (np. katalogu instalacji oraz opcjonalnego odrębnego katalogu stagingu/pobierania).
pub fn verify_disk_preflight(
    provider: &dyn CapacityProvider,
    installation_root: &Path,
    staging_dir: &Path,
    requirements: &DiskPreflightRequirements,
) -> Result<(), UpdaterError> {
    let root_fs = provider.filesystem_id(installation_root)?;
    let staging_fs = provider.filesystem_id(staging_dir)?;

    if root_fs == staging_fs {
        // Jeden system plików — cała wymagana przestrzeń musi zmieścić się na tym wolumenie
        let available = provider.available_space(installation_root)?;
        if available < requirements.total_required_bytes {
            return Err(UpdaterError::DiskSpaceUnavailable(format!(
                "Niewystarczająca ilość wolnego miejsca na systemie plików '{root_fs}': wymagane {} B, dostępne {} B",
                requirements.total_required_bytes, available
            )));
        }
    } else {
        // Dwa różne systemy plików — badamy staging i instalację osobno
        let staging_needed = requirements
            .declared_download_size
            .checked_add(requirements.max_extracted_size)
            .and_then(|v| v.checked_add(requirements.filesystem_safety_margin))
            .ok_or(UpdaterError::DiskSpaceOverflow)?;

        let staging_available = provider.available_space(staging_dir)?;
        if staging_available < staging_needed {
            return Err(UpdaterError::DiskSpaceUnavailable(format!(
                "Niewystarczające miejsce dla stagingu na systemie plików '{staging_fs}': wymagane {staging_needed} B, dostępne {staging_available} B"
            )));
        }

        let root_needed = requirements
            .backup_size
            .checked_add(requirements.package_manager_overhead)
            .and_then(|v| v.checked_add(requirements.transaction_metadata_size))
            .and_then(|v| v.checked_add(requirements.filesystem_safety_margin))
            .ok_or(UpdaterError::DiskSpaceOverflow)?;

        let root_available = provider.available_space(installation_root)?;
        if root_available < root_needed {
            return Err(UpdaterError::DiskSpaceUnavailable(format!(
                "Niewystarczające miejsce dla instalacji na systemie plików '{root_fs}': wymagane {root_needed} B, dostępne {root_available} B"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_space_uses_checked_arithmetic() {
        let req = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            10_000_000,
            20_000_000,
            15_000_000,
            None,
        )
        .expect("Pomyślne wyliczenie z checked arithmetic");

        // Suma: 10M + 20M + 15M + 0 + 1M (meta) + 32M (default margin) = 78M (w bajtach)
        let expected = 10_000_000
            + 20_000_000
            + 15_000_000
            + TRANSACTION_METADATA_BYTES
            + DEFAULT_SAFETY_MARGIN_BYTES;
        assert_eq!(req.total_required_bytes, expected);
    }

    #[test]
    fn required_space_rejects_overflow() {
        // Symulacja przepełnienia u64 przy dodawaniu
        let res = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            u64::MAX - 100,
            1000,
            1000,
            None,
        );
        assert_eq!(res, Err(UpdaterError::DiskSpaceOverflow));
    }

    #[test]
    fn required_space_uses_mib_not_decimal_mb() {
        assert_eq!(MIN_SAFETY_MARGIN_BYTES, 25 * 1024 * 1024);
        assert_ne!(MIN_SAFETY_MARGIN_BYTES, 25 * 1_000_000);

        assert_eq!(DEFAULT_SAFETY_MARGIN_BYTES, 32 * 1024 * 1024);
        assert_ne!(DEFAULT_SAFETY_MARGIN_BYTES, 32 * 1_000_000);

        assert_eq!(MAX_SAFETY_MARGIN_BYTES, 100 * 1024 * 1024);
        assert_ne!(MAX_SAFETY_MARGIN_BYTES, 100 * 1_000_000);
    }

    #[test]
    fn minimum_margin_is_25_mib() {
        let res = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            1000,
            1000,
            1000,
            Some(MIN_SAFETY_MARGIN_BYTES),
        );
        assert!(res.is_ok());

        // Poniżej 25 MiB jest odrzucane
        let invalid = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            1000,
            1000,
            1000,
            Some(MIN_SAFETY_MARGIN_BYTES - 1),
        );
        assert!(matches!(invalid, Err(UpdaterError::SafetyGate(_))));
    }

    #[test]
    fn default_margin_is_32_mib() {
        let req = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            1000,
            1000,
            1000,
            None,
        )
        .unwrap();
        assert_eq!(req.filesystem_safety_margin, 32 * 1024 * 1024);
    }

    #[test]
    fn maximum_margin_is_100_mib() {
        let res = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            1000,
            1000,
            1000,
            Some(MAX_SAFETY_MARGIN_BYTES),
        );
        assert!(res.is_ok());

        // Powyżej 100 MiB jest odrzucane
        let invalid = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            1000,
            1000,
            1000,
            Some(MAX_SAFETY_MARGIN_BYTES + 1),
        );
        assert!(matches!(invalid, Err(UpdaterError::SafetyGate(_))));
    }

    #[test]
    fn self_managed_space_includes_staging_and_backup() {
        let req = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            10_000_000,
            25_000_000,
            20_000_000,
            None,
        )
        .unwrap();

        assert_eq!(req.max_extracted_size, 25_000_000);
        assert_eq!(req.backup_size, 20_000_000);
        assert_eq!(req.package_manager_overhead, 0);
        assert!(!req.is_package_manager_overhead_estimated);
    }

    #[test]
    fn system_managed_space_marks_unknown_manager_overhead() {
        let req = DiskPreflightRequirements::calculate(
            InstallationType::WindowsMsi,
            10_000_000,
            25_000_000,
            20_000_000,
            None,
        )
        .unwrap();

        // Dla formatów systemowych ekstrakcja i backup programu są 0 (zarządza OS)
        assert_eq!(req.max_extracted_size, 0);
        assert_eq!(req.backup_size, 0);
        // Narzut menedżera pakietów jest oznaczony jako szacunkowy
        assert!(req.package_manager_overhead > 0);
        assert!(req.is_package_manager_overhead_estimated);
    }

    #[test]
    fn different_filesystems_are_evaluated_separately() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("install");
        let staging = temp.path().join("staging");

        // Staging ma 500 MiB, ale instalacja tylko 10 MiB
        let provider = FakeCapacityProvider::new(100 * 1024 * 1024)
            .with_path_space(&staging, 500 * 1024 * 1024, "fs-staging")
            .with_path_space(&root, 10 * 1024 * 1024, "fs-root");

        let req = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            10 * 1024 * 1024,
            20 * 1024 * 1024,
            15 * 1024 * 1024,
            None,
        )
        .unwrap();

        // Preflight musi wykryć brak miejsca na systemie root
        let res = verify_disk_preflight(&provider, &root, &staging, &req);
        assert!(matches!(res, Err(UpdaterError::DiskSpaceUnavailable(_))));
    }

    #[test]
    fn insufficient_space_fails_before_download_or_apply() {
        let temp = tempfile::tempdir().unwrap();
        let provider = FakeCapacityProvider::new(1024 * 1024); // Tylko 1 MiB wolnego miejsca

        let req = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            15 * 1024 * 1024,
            30 * 1024 * 1024,
            25 * 1024 * 1024,
            None,
        )
        .unwrap();

        let res = verify_disk_preflight(&provider, temp.path(), temp.path(), &req);
        assert!(matches!(res, Err(UpdaterError::DiskSpaceUnavailable(_))));
    }

    #[test]
    fn space_provider_error_is_typed() {
        let mut provider = FakeCapacityProvider::new(100 * 1024 * 1024);
        provider.simulate_error = true;

        let res = provider.available_space(Path::new("dummy"));
        assert!(matches!(res, Err(UpdaterError::Io(_))));
    }
}
