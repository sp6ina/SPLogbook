// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wspólny kontrakt adaptera platformowego i sztuczny adapter testowy (M3-A)

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::capabilities::InstallationCapabilities;
use crate::cloud::updater::installation::{InstallationType, ManagementMode};
use crate::cloud::updater::{UpdatePlan, UpdaterError};

/// Raport z inspekcji zawartości i nagłówków pakietu instalacyjnego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInspectionReport {
    pub total_entries: usize,
    pub total_uncompressed_bytes: u64,
    pub contains_executable: bool,
    pub has_unsafe_paths: bool,
}

/// Wynik operacji delegacji aktualizacji do menedżera systemowego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationResult {
    pub manager_name: String,
    pub command_summary: String,
    pub requires_restart: bool,
}

/// Uchwyt do uruchomionego procesu kandydata po aktualizacji.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateProcessHandle {
    pub pid: u32,
    pub attempt_id: UpdateAttemptId,
}

/// Zredagowane podsumowanie diagnostyczne platformy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedDiagnosticReport {
    pub installation_type: InstallationType,
    pub management_mode: ManagementMode,
    pub capabilities: InstallationCapabilities,
    pub details: Vec<String>,
}

/// Wspólny interfejs adaptera platformowego dla wszystkich instalatorów SPLogbook.
///
/// Gwarancje architektoniczne:
/// - Adapter nie przyjmuje surowych, niezweryfikowanych adresów URL (jedynie zweryfikowane tożsamości assetów).
/// - Adapter w trybie SelfManaged nie może pominąć walidacji UpdatePlan.
/// - Adapter w trybie SystemManaged nie podmienia ręcznie plików pakietowych w katalogach systemowych.
/// - Adapter bezwzględnie nie używa narzędzi eskalacji uprawnień (`sudo`, `doas`, `pkexec`, `runas`).
/// - Wywołanie operacji niewspieranej przez macierz możliwości zwraca `UpdaterError::CapabilityUnavailable`.
pub trait PlatformInstallerAdapter: Send + Sync {
    /// Zwraca typ instalacji powiązany z danym adapterem.
    fn installation_type(&self) -> InstallationType;

    /// Zwraca zestaw możliwości danego adaptera.
    fn capabilities(&self) -> InstallationCapabilities {
        InstallationCapabilities::for_installation_type(self.installation_type())
    }

    /// Zwraca tryb zarządzania (SelfManaged / SystemManaged).
    fn management_mode(&self) -> ManagementMode {
        self.installation_type().management_mode()
    }

    /// Wykrywa rodzaj instalacji pod wskazanym katalogiem bazowym.
    fn detect_installation(&self, base_dir: &Path) -> Result<InstallationType, UpdaterError>;

    /// Dokonuje inspekcji nagłówków pobranego archiwum/pakietu przed ekstrakcją.
    fn inspect_package(&self, package_path: &Path)
    -> Result<PackageInspectionReport, UpdaterError>;

    /// Rozpakowuje pakiet do wskazanego katalogu stagingu.
    fn stage_package(&self, package_path: &Path, staging_dir: &Path) -> Result<(), UpdaterError>;

    /// Waliduje bezpieczeństwo wygenerowanego planu aktualizacji.
    fn validate_update_plan(&self, plan: &UpdatePlan) -> Result<(), UpdaterError>;

    /// Wykonuje fizyczną podmianę plików w trybie SelfManaged pod rygorem UpdatePlan.
    fn apply_self_managed(
        &self,
        base_dir: &Path,
        staging_dir: &Path,
        plan: &UpdatePlan,
        attempt_id: &UpdateAttemptId,
    ) -> Result<(), UpdaterError>;

    /// Deleguje wykonanie aktualizacji do instalatora systemowego w trybie SystemManaged.
    fn delegate_system_managed(
        &self,
        installer_path: Option<&Path>,
        inst_type: InstallationType,
    ) -> Result<DelegationResult, UpdaterError>;

    /// Uruchamia proces potomny kandydata z flagą diagnostyczną.
    fn launch_candidate(
        &self,
        exe_path: &Path,
        attempt_id: &UpdateAttemptId,
    ) -> Result<CandidateProcessHandle, UpdaterError>;

    /// Oczekuje na dowód zdrowia (.health_ok związany z attempt_id).
    fn wait_for_health(
        &self,
        base_dir: &Path,
        attempt_id: &UpdateAttemptId,
        timeout: Duration,
    ) -> Result<bool, UpdaterError>;

    /// Przywraca stan sprzed aktualizacji z katalogu .rollback.
    fn rollback(
        &self,
        base_dir: &Path,
        rollback_dir: &Path,
        plan: &UpdatePlan,
    ) -> Result<(), UpdaterError>;

    /// Czyści tymczasowe artefakty transakcji i staging.
    fn cleanup(&self, base_dir: &Path) -> Result<(), UpdaterError>;

    /// Zwraca zredagowany raport diagnostyczny.
    fn diagnostics(&self) -> SanitizedDiagnosticReport;
}

/// Sztuczny adapter testowy (Fake Platform Adapter) symulujący operacje instalacyjne
/// w kontrolowanym środowisku pamięciowym i katalogach tymczasowych.
#[derive(Debug, Clone)]
pub struct FakePlatformAdapter {
    pub inst_type: InstallationType,
    pub simulate_apply_failure: bool,
    pub simulate_health_timeout: bool,
    pub simulate_rollback_failure: bool,
    pub recorded_apply_calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub recorded_rollback_calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub recorded_delegations: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl FakePlatformAdapter {
    pub fn new(inst_type: InstallationType) -> Self {
        Self {
            inst_type,
            simulate_apply_failure: false,
            simulate_health_timeout: false,
            simulate_rollback_failure: false,
            recorded_apply_calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            recorded_rollback_calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            recorded_delegations: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl PlatformInstallerAdapter for FakePlatformAdapter {
    fn installation_type(&self) -> InstallationType {
        self.inst_type
    }

    fn detect_installation(&self, _base_dir: &Path) -> Result<InstallationType, UpdaterError> {
        Ok(self.inst_type)
    }

    fn inspect_package(
        &self,
        _package_path: &Path,
    ) -> Result<PackageInspectionReport, UpdaterError> {
        Ok(PackageInspectionReport {
            total_entries: 10,
            total_uncompressed_bytes: 50_000_000,
            contains_executable: true,
            has_unsafe_paths: false,
        })
    }

    fn stage_package(&self, _package_path: &Path, _staging_dir: &Path) -> Result<(), UpdaterError> {
        Ok(())
    }

    fn validate_update_plan(&self, plan: &UpdatePlan) -> Result<(), UpdaterError> {
        plan.validate_safety("SPLogbook.exe")
    }

    fn apply_self_managed(
        &self,
        _base_dir: &Path,
        _staging_dir: &Path,
        _plan: &UpdatePlan,
        _attempt_id: &UpdateAttemptId,
    ) -> Result<(), UpdaterError> {
        if !self.capabilities().supports_self_apply {
            return Err(UpdaterError::CapabilityUnavailable(format!(
                "Format {} nie wspiera operacji apply_self_managed",
                self.inst_type
            )));
        }
        self.recorded_apply_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.simulate_apply_failure {
            return Err(UpdaterError::InstallFailed("Symulowany błąd apply".into()));
        }
        Ok(())
    }

    fn delegate_system_managed(
        &self,
        _installer_path: Option<&Path>,
        inst_type: InstallationType,
    ) -> Result<DelegationResult, UpdaterError> {
        if !self.capabilities().requires_delegation {
            return Err(UpdaterError::CapabilityUnavailable(format!(
                "Format {} nie wspiera operacji delegate_system_managed",
                self.inst_type
            )));
        }
        self.recorded_delegations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(DelegationResult {
            manager_name: inst_type.as_str().to_string(),
            command_summary: format!("delegate to {inst_type}"),
            requires_restart: false,
        })
    }

    fn launch_candidate(
        &self,
        _exe_path: &Path,
        attempt_id: &UpdateAttemptId,
    ) -> Result<CandidateProcessHandle, UpdaterError> {
        Ok(CandidateProcessHandle {
            pid: 12345,
            attempt_id: *attempt_id,
        })
    }

    fn wait_for_health(
        &self,
        _base_dir: &Path,
        _attempt_id: &UpdateAttemptId,
        _timeout: Duration,
    ) -> Result<bool, UpdaterError> {
        if self.simulate_health_timeout {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn rollback(
        &self,
        _base_dir: &Path,
        _rollback_dir: &Path,
        _plan: &UpdatePlan,
    ) -> Result<(), UpdaterError> {
        if !self.capabilities().supports_sp_logbook_rollback {
            return Err(UpdaterError::CapabilityUnavailable(format!(
                "Format {} nie wspiera operacji rollback przez SPLogbook",
                self.inst_type
            )));
        }
        self.recorded_rollback_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.simulate_rollback_failure {
            return Err(UpdaterError::RollbackFailed(
                "Symulowany błąd rollbacku".into(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, _base_dir: &Path) -> Result<(), UpdaterError> {
        Ok(())
    }

    fn diagnostics(&self) -> SanitizedDiagnosticReport {
        SanitizedDiagnosticReport {
            installation_type: self.inst_type,
            management_mode: self.management_mode(),
            capabilities: self.capabilities(),
            details: vec!["fake_adapter_active".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_adapter_self_managed_flow() {
        let adapter = FakePlatformAdapter::new(InstallationType::WindowsPortableZip);
        assert!(adapter.capabilities().supports_self_apply);
        assert_eq!(adapter.management_mode(), ManagementMode::SelfManaged);

        let temp = tempfile::tempdir().unwrap();
        let plan = UpdatePlan {
            replace_files: vec![(
                std::path::PathBuf::from("SPLogbook.exe"),
                temp.path().join("SPLogbook.exe"),
            )],
            create_files: Vec::new(),
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };

        let attempt = UpdateAttemptId::generate();
        adapter
            .apply_self_managed(temp.path(), temp.path(), &plan, &attempt)
            .expect("Pomyślne apply_self_managed");
        assert_eq!(
            adapter
                .recorded_apply_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        let health_ok = adapter
            .wait_for_health(temp.path(), &attempt, Duration::from_secs(1))
            .expect("Pomyślne health check");
        assert!(health_ok);
    }

    #[test]
    fn fake_adapter_system_managed_flow() {
        let adapter = FakePlatformAdapter::new(InstallationType::WindowsMsi);
        assert!(!adapter.capabilities().supports_self_apply);
        assert!(adapter.capabilities().requires_delegation);
        assert_eq!(adapter.management_mode(), ManagementMode::SystemManaged);

        let res = adapter
            .delegate_system_managed(None, InstallationType::WindowsMsi)
            .expect("Pomyślna delegacja do MSI");
        assert_eq!(res.manager_name, "windows-msi");
        assert_eq!(
            adapter
                .recorded_delegations
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn fake_adapter_rejects_unsupported_capability() {
        // MSI nie wspiera self_apply
        let adapter = FakePlatformAdapter::new(InstallationType::WindowsMsi);
        let temp = tempfile::tempdir().unwrap();
        let plan = UpdatePlan {
            replace_files: Vec::new(),
            create_files: Vec::new(),
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };
        let attempt = UpdateAttemptId::generate();
        let res = adapter.apply_self_managed(temp.path(), temp.path(), &plan, &attempt);
        assert!(matches!(res, Err(UpdaterError::CapabilityUnavailable(_))));

        // Portable ZIP nie wspiera delegacji
        let zip_adapter = FakePlatformAdapter::new(InstallationType::WindowsPortableZip);
        let del_res =
            zip_adapter.delegate_system_managed(None, InstallationType::WindowsPortableZip);
        assert!(matches!(
            del_res,
            Err(UpdaterError::CapabilityUnavailable(_))
        ));
    }

    #[test]
    fn fake_adapter_system_managed_cannot_self_apply() {
        let deb_adapter = FakePlatformAdapter::new(InstallationType::LinuxDeb);
        let temp = tempfile::tempdir().unwrap();
        let plan = UpdatePlan {
            replace_files: Vec::new(),
            create_files: Vec::new(),
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };
        let attempt = UpdateAttemptId::generate();
        let res = deb_adapter.apply_self_managed(temp.path(), temp.path(), &plan, &attempt);
        assert!(matches!(res, Err(UpdaterError::CapabilityUnavailable(_))));
    }

    #[test]
    fn fake_adapter_diagnostics_contain_no_secrets() {
        let adapter = FakePlatformAdapter::new(InstallationType::LinuxAppImage);
        let diag = adapter.diagnostics();
        let serialized = serde_json::to_string(&diag).unwrap();

        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("key"));
        assert!(!serialized.contains("callsign"));
    }
}
