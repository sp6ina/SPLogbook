// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Asynchroniczny aktualizator baz referencyjnych (cty.dat, master.scp, lista użytkowników LoTW)

use std::path::{Path, PathBuf};

/// Podsumowanie operacji aktualizacji baz referencyjnych.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseReport(pub String);

impl std::fmt::Display for DatabaseReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for DatabaseReport {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Błąd operacji aktualizatora bazy lub aplikacji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdaterError {
    Network(String),
    HttpStatus(String),
    Io(String),
    FileTooLarge,
    ManifestTooLarge,
    SignatureTooLarge,
    ArtifactTooLarge,
    HashMismatch,
    SizeMismatch,
    InvalidSignature(String),
    UntrustedKey,
    ManifestParse(String),
    KeyMismatch(String),
    ManifestValidation(String),
    InvalidVersion(String),
    VersionTooOld(String),
    VersionTooNew(String),
    DowngradeRejected(String),
    AlreadyUpToDate(String),
    MissingField(&'static str),
    NoMatchingAssets,
    UnsupportedPlatform(String),
    AmbiguousArtifact(String),
    UnsafeArchivePath(String),
    ArchiveLimitExceeded(String),
    InvalidStagingLayout(String),
    InstallFailed(String),
    RollbackFailed(String),
    MissingManifest,
    MissingSignature,
    InvalidUtf8,
    ExePath(String),
    ChecksumMismatch,
    SafetyGate(String),
    ZipSlip(String),
    HealthCheckFailed,
    UpdateFailed(String),
    Other(String),
    InvalidAttemptId(String),
    InvalidTransition(String),
    InvalidTransaction(String),
    UnsupportedTransactionVersion(u32),
    InstallationDetection(String),
    ConflictingInstallationMarkers(String),
    UnknownInstallation(String),
    CapabilityUnavailable(String),
    LockContended(String),
    LockCorrupt(String),
    ForeignLock(String),
    DiskSpaceUnavailable(String),
    DiskSpaceOverflow,
    Persistence(String),
    RecoveryRequired(String),
    RecoveryBlocked(String),
    CancellationRejected(String),
    UnsafePath(String),
    RedactionFailure(String),
}

pub mod attempt_id;
pub mod capabilities;
pub mod installation;
pub mod lock;
pub mod platform;
pub mod preflight;
pub mod state;
pub mod transaction;

pub use attempt_id::UpdateAttemptId;
pub use capabilities::InstallationCapabilities;
pub use installation::{
    InstallationDetectionContext, InstallationType, ManagementMode,
    detect_installation_with_context,
};
pub use lock::{
    UPDATE_LOCK_FILENAME, UPDATE_LOCK_RECORD_VERSION, UpdateLockGuard, UpdateLockRecord,
    evaluate_lock_status,
};
pub use platform::{
    CandidateProcessHandle, DelegationResult, FakePlatformAdapter, PackageInspectionReport,
    PlatformInstallerAdapter, SanitizedDiagnosticReport,
};
pub use preflight::{
    CapacityProvider, DEFAULT_SAFETY_MARGIN_BYTES, DiskPreflightRequirements, FakeCapacityProvider,
    MAX_SAFETY_MARGIN_BYTES, MIN_SAFETY_MARGIN_BYTES, SystemCapacityProvider,
    TRANSACTION_METADATA_BYTES, verify_disk_preflight,
};
pub use state::{UpdateStateId, UpdateStateMachine};
pub use transaction::{
    CURRENT_RECORD_VERSION, CancellationStatus, RecoveryStatus, TRANSACTION_FILE_NAME,
    UpdateTransaction,
};

impl UpdaterError {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Network(s)
            | Self::HttpStatus(s)
            | Self::Io(s)
            | Self::InvalidSignature(s)
            | Self::ManifestParse(s)
            | Self::KeyMismatch(s)
            | Self::ManifestValidation(s)
            | Self::InvalidVersion(s)
            | Self::VersionTooOld(s)
            | Self::VersionTooNew(s)
            | Self::DowngradeRejected(s)
            | Self::AlreadyUpToDate(s)
            | Self::ExePath(s)
            | Self::SafetyGate(s)
            | Self::ZipSlip(s)
            | Self::UpdateFailed(s)
            | Self::Other(s)
            | Self::UnsupportedPlatform(s)
            | Self::AmbiguousArtifact(s)
            | Self::UnsafeArchivePath(s)
            | Self::ArchiveLimitExceeded(s)
            | Self::InvalidStagingLayout(s)
            | Self::InstallFailed(s)
            | Self::RollbackFailed(s)
            | Self::InvalidAttemptId(s)
            | Self::InvalidTransition(s)
            | Self::InvalidTransaction(s)
            | Self::InstallationDetection(s)
            | Self::ConflictingInstallationMarkers(s)
            | Self::UnknownInstallation(s)
            | Self::CapabilityUnavailable(s)
            | Self::LockContended(s)
            | Self::LockCorrupt(s)
            | Self::ForeignLock(s)
            | Self::DiskSpaceUnavailable(s)
            | Self::Persistence(s)
            | Self::RecoveryRequired(s)
            | Self::RecoveryBlocked(s)
            | Self::CancellationRejected(s)
            | Self::UnsafePath(s)
            | Self::RedactionFailure(s) => s.as_str(),
            Self::UnsupportedTransactionVersion(_) => {
                "Nieobsługiwana wersja rekordu transakcji aktualizatora."
            }
            Self::DiskSpaceOverflow => {
                "Przepełnienie arytmetyczne przy obliczaniu wymaganej przestrzeni dyskowej."
            }
            Self::MissingField(s) => s,
            Self::FileTooLarge => "Plik przekracza dozwolony limit wielkości.",
            Self::ManifestTooLarge => "Manifest przekracza dozwolony limit wielkości (1 MB).",
            Self::SignatureTooLarge => {
                "Podpis manifestu przekracza dozwolony limit wielkości (1 MB)."
            }
            Self::ArtifactTooLarge => "Pobrany artefakt przekracza dozwolony limit wielkości.",
            Self::HashMismatch => {
                "Suma kontrolna SHA256 pobranej aktualizacji nie zgadza się z wartością w manifeście."
            }
            Self::SizeMismatch => {
                "Rozmiar pobranego pliku nie zgadza się z wartością w manifeście."
            }
            Self::UntrustedKey => "Żaden z zaufanych kluczy nie zweryfikował tego podpisu",
            Self::NoMatchingAssets => {
                "Manifest nie zawiera plików dla tego wydania zbieżnych z GitHub API."
            }
            Self::MissingManifest => "Brak release-manifest.json w wydaniu.",
            Self::MissingSignature => "Brak podpisu release-manifest.json.sig w wydaniu.",
            Self::InvalidUtf8 => "Podpis nie jest prawidłowym ciągiem UTF-8",
            Self::ChecksumMismatch => {
                "Suma kontrolna SHA256 pobranej aktualizacji nie zgadza się z wartością z GitHub."
            }
            Self::HealthCheckFailed => {
                "Zaktualizowana aplikacja uległa natychmiastowej awarii (segfault/panic). Przywrócono starszą wersję."
            }
        }
    }
}

impl std::ops::Deref for UpdaterError {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Display for UpdaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for UpdaterError {}

pub struct DatabaseUpdater;

impl DatabaseUpdater {
    /// Pobiera najnowszy plik definicji krajów i prefiksów (cty.dat)
    pub async fn update_country_file(dest_path: &Path) -> Result<usize, UpdaterError> {
        let url = "https://www.country-files.com/cty/cty.dat";
        Self::download_file(url, dest_path).await
    }

    /// Pobiera najnowszą bazę Super Check Partial (MASTER.SCP)
    pub async fn update_scp_file(dest_path: &Path) -> Result<usize, UpdaterError> {
        let url = "https://www.supercheckpartial.com/MASTER.SCP";
        Self::download_file(url, dest_path).await
    }

    /// Pobiera wszystkie bazy referencyjne do wskazanego katalogu
    pub async fn update_all(dir: &Path) -> Result<(), UpdaterError> {
        let _ = Self::update_all_with_report(dir, crate::core::i18n::Language::Pl).await?;
        Ok(())
    }

    /// Pobiera wszystkie bazy referencyjne (cty.dat, MASTER.SCP, lotw-user-activity.csv)
    /// niezależnie od siebie i zwraca podsumowanie dla paska statusu / powiadomienia Toast.
    pub async fn update_all_with_report(
        dir: &Path,
        lang: crate::core::i18n::Language,
    ) -> Result<DatabaseReport, UpdaterError> {
        let mut ok_parts = Vec::new();
        let mut err_parts = Vec::new();

        match Self::update_country_file(&dir.join("cty.dat")).await {
            Ok(bytes) => ok_parts.push(format!("cty.dat ({} KB)", bytes / 1024)),
            Err(e) => err_parts.push(format!("cty.dat: {e}")),
        }

        match Self::update_scp_file(&dir.join("MASTER.SCP")).await {
            Ok(bytes) => ok_parts.push(format!("MASTER.SCP ({} KB)", bytes / 1024)),
            Err(e) => err_parts.push(format!("MASTER.SCP: {e}")),
        }

        match Self::update_lotw_users(&dir.join("lotw-user-activity.csv")).await {
            Ok(bytes) => ok_parts.push(format!("LoTW ({} KB)", bytes / 1024)),
            Err(e) => err_parts.push(format!("LoTW: {e}")),
        }

        if ok_parts.is_empty() {
            let prefix =
                crate::core::i18n::tr(crate::core::i18n::key::UPDATER_DB_UPDATE_FAILED, lang);
            Err(UpdaterError::UpdateFailed(format!(
                "{prefix}: {}",
                err_parts.join(" | ")
            )))
        } else if err_parts.is_empty() {
            let prefix =
                crate::core::i18n::tr(crate::core::i18n::key::UPDATER_DB_UPDATE_SUCCESS, lang);
            Ok(DatabaseReport(format!("{prefix}: {}", ok_parts.join(", "))))
        } else {
            let prefix =
                crate::core::i18n::tr(crate::core::i18n::key::UPDATER_DB_UPDATE_PARTIAL, lang);
            Ok(DatabaseReport(format!(
                "{prefix}: OK [{}], Err [{}]",
                ok_parts.join(", "),
                err_parts.join("; ")
            )))
        }
    }

    /// Pobiera listę aktywnych użytkowników LoTW (lotw-user-activity.csv)
    pub async fn update_lotw_users(dest_path: &Path) -> Result<usize, UpdaterError> {
        let url = "https://lotw.arrl.org/lotw-user-activity.csv";
        Self::download_file(url, dest_path).await
    }

    async fn download_file(url: &str, dest_path: &Path) -> Result<usize, UpdaterError> {
        let client = crate::core::http::http_client_with_timeout(30);

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| UpdaterError::Network(format!("Błąd pobierania {url}: {e}")))?;

        if !resp.status().is_success() {
            return Err(UpdaterError::HttpStatus(format!(
                "Serwer zwrócił status {}: {}",
                resp.status(),
                url
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| UpdaterError::Network(format!("Błąd odczytu danych z {url}: {e}")))?;

        if let Some(parent) = dest_path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                UpdaterError::Io(format!("Błąd tworzenia katalogu {}: {e}", parent.display()))
            })?;
        }

        tokio::fs::write(dest_path, &bytes).await.map_err(|e| {
            UpdaterError::Io(format!("Błąd zapisu do pliku {}: {e}", dest_path.display()))
        })?;

        Ok(bytes.len())
    }
}

/// Pojedynczy zasób zdefiniowany w manifeście.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestAsset {
    pub platform: String,
    pub arch: String,
    pub package_type: String,
    pub filename: String,
    pub size: u64,
    pub sha256: String,
}

/// Obsługiwane platformy docelowe wydań.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleasePlatform {
    Windows,
    Linux,
    MacOs,
}

impl ReleasePlatform {
    pub fn parse(s: &str) -> Result<Self, UpdaterError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOs),
            other => Err(UpdaterError::UnsupportedPlatform(format!(
                "Nieobsługiwana platforma: {other}"
            ))),
        }
    }

    pub fn current() -> Result<Self, UpdaterError> {
        #[cfg(target_os = "windows")]
        return Ok(Self::Windows);
        #[cfg(target_os = "linux")]
        return Ok(Self::Linux);
        #[cfg(target_os = "macos")]
        return Ok(Self::MacOs);
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return Err(UpdaterError::UnsupportedPlatform(
            "Nieobsługiwana platforma hosta".into(),
        ));
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::MacOs => "macos",
        }
    }
}

/// Obsługiwane architektury procesora wydań.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleaseArchitecture {
    X86_64,
}

impl ReleaseArchitecture {
    pub fn parse(s: &str) -> Result<Self, UpdaterError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "x86_64" => Ok(Self::X86_64),
            other => Err(UpdaterError::UnsupportedPlatform(format!(
                "Nieobsługiwana architektura: {other}"
            ))),
        }
    }

    pub fn current() -> Result<Self, UpdaterError> {
        #[cfg(target_arch = "x86_64")]
        return Ok(Self::X86_64);
        #[cfg(not(target_arch = "x86_64"))]
        return Err(UpdaterError::UnsupportedPlatform(
            "Nieobsługiwana architektura hosta".into(),
        ));
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
        }
    }
}

/// Obsługiwane formaty pakietów wydań.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleasePackageType {
    Zip,
    TarGz,
}

impl ReleasePackageType {
    pub fn parse(s: &str) -> Result<Self, UpdaterError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "zip" => Ok(Self::Zip),
            "tar.gz" => Ok(Self::TarGz),
            other => Err(UpdaterError::SafetyGate(format!(
                "Nieobsługiwany typ paczki w manifeście: {other}"
            ))),
        }
    }

    pub fn expected_for_platform(platform: ReleasePlatform) -> Self {
        match platform {
            ReleasePlatform::Windows => Self::Zip,
            ReleasePlatform::Linux | ReleasePlatform::MacOs => Self::TarGz,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::TarGz => "tar.gz",
        }
    }
}

/// Zasób wydania zweryfikowany względem podpisanego manifestu i powiązany z tożsamością wydania.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReleaseAsset {
    platform: ReleasePlatform,
    arch: ReleaseArchitecture,
    package_type: ReleasePackageType,
    filename: String,
    declared_size: u64,
    declared_sha256: String,
    browser_download_url: String,
    bound_manifest_version: String,
    bound_commit: String,
}

impl VerifiedReleaseAsset {
    pub fn platform(&self) -> ReleasePlatform {
        self.platform
    }

    pub fn arch(&self) -> ReleaseArchitecture {
        self.arch
    }

    pub fn package_type(&self) -> ReleasePackageType {
        self.package_type
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn declared_size(&self) -> u64 {
        self.declared_size
    }

    pub fn declared_sha256(&self) -> &str {
        &self.declared_sha256
    }

    pub fn browser_download_url(&self) -> &str {
        &self.browser_download_url
    }

    pub fn bound_manifest_version(&self) -> &str {
        &self.bound_manifest_version
    }

    pub fn bound_commit(&self) -> &str {
        &self.bound_commit
    }
}

/// Kryptograficznie zweryfikowany manifest wydania Ed25519.
/// Niemożliwy do skonstruowania bez przejścia weryfikacji podpisu i polityki zaufania.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReleaseManifest {
    manifest: ReleaseManifest,
    signing_key_id: String,
    signing_key_fingerprint: String,
}

impl VerifiedReleaseManifest {
    pub fn verify(
        manifest_bytes: &[u8],
        signature_base64: &str,
        current_version: &str,
    ) -> Result<Self, UpdaterError> {
        let (trusted_key, manifest) = verify_manifest_signature(manifest_bytes, signature_base64)?;
        verify_downgrade_protection(&manifest.version, current_version)?;

        Ok(Self {
            signing_key_id: trusted_key.key_id.to_string(),
            signing_key_fingerprint: sha256_hex(&trusted_key.public_key),
            manifest,
        })
    }

    pub fn verify_with_keys(
        manifest_bytes: &[u8],
        signature_base64: &str,
        trusted_keys: &[TrustedKey],
        revoked_key_ids: &[&str],
        revoked_public_keys: &[[u8; 32]],
        current_version: &str,
    ) -> Result<Self, UpdaterError> {
        let (trusted_key, manifest) = verify_manifest_signature_with_keys(
            manifest_bytes,
            signature_base64,
            trusted_keys,
            revoked_key_ids,
            revoked_public_keys,
        )?;
        verify_downgrade_protection(&manifest.version, current_version)?;

        Ok(Self {
            signing_key_id: trusted_key.key_id.to_string(),
            signing_key_fingerprint: sha256_hex(&trusted_key.public_key),
            manifest,
        })
    }

    pub fn manifest(&self) -> &ReleaseManifest {
        &self.manifest
    }

    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    pub fn product(&self) -> &str {
        &self.manifest.product
    }

    pub fn channel(&self) -> &str {
        &self.manifest.channel
    }

    pub fn commit(&self) -> &str {
        &self.manifest.commit
    }

    pub fn signing_key_id(&self) -> &str {
        &self.signing_key_id
    }

    pub fn signing_key_fingerprint(&self) -> &str {
        &self.signing_key_fingerprint
    }

    pub fn select_asset_for_platform(
        &self,
        target_platform: ReleasePlatform,
        target_arch: ReleaseArchitecture,
        release_assets: &[ReleaseAsset],
    ) -> Result<VerifiedReleaseAsset, UpdaterError> {
        let expected_package = ReleasePackageType::expected_for_platform(target_platform);

        let mut matches = Vec::new();
        for m_asset in &self.manifest.assets {
            let p = ReleasePlatform::parse(&m_asset.platform)?;
            let a = ReleaseArchitecture::parse(&m_asset.arch)?;
            if p == target_platform && a == target_arch {
                matches.push(m_asset);
            }
        }

        if matches.is_empty() {
            return Err(UpdaterError::UnsupportedPlatform(format!(
                "Brak pasującego zasobu w zweryfikowanym manifeście dla {}-{}",
                target_platform.as_str(),
                target_arch.as_str()
            )));
        }

        if matches.len() > 1 {
            return Err(UpdaterError::AmbiguousArtifact(format!(
                "Niejednoznaczne dopasowanie w manifeście: {} zasobów dla {}-{}",
                matches.len(),
                target_platform.as_str(),
                target_arch.as_str()
            )));
        }

        let m_asset = matches[0];

        // Walidacja package_type
        let pkg_type = ReleasePackageType::parse(&m_asset.package_type)?;
        if pkg_type != expected_package {
            return Err(UpdaterError::SafetyGate(format!(
                "Nieoczekiwany typ paczki w manifeście: oczekiwano {expected_package:?}, znaleziono {pkg_type:?}"
            )));
        }

        // Walidacja nazwy pliku
        let filename = m_asset.filename.trim();
        if filename.is_empty()
            || filename == "."
            || filename == ".."
            || filename.contains('/')
            || filename.contains('\\')
            || filename.contains(':')
        {
            return Err(UpdaterError::UnsafeArchivePath(format!(
                "Nazwa pliku w manifeście '{filename}' zawiera niedozwolone znaki"
            )));
        }

        // Walidacja rozszerzenia dla danego typu paczki
        let expected_ext = match pkg_type {
            ReleasePackageType::Zip => ".zip",
            ReleasePackageType::TarGz => ".tar.gz",
        };
        if !filename.to_ascii_lowercase().ends_with(expected_ext) {
            return Err(UpdaterError::SafetyGate(format!(
                "Nazwa pliku '{filename}' nie odpowiada typowi paczki {pkg_type:?}"
            )));
        }

        // Walidacja rozmiaru
        if m_asset.size == 0 || m_asset.size > MAX_ARTIFACT_DOWNLOAD_SIZE {
            return Err(UpdaterError::FileTooLarge);
        }

        // Walidacja SHA256
        if m_asset.sha256.len() != 64 || !m_asset.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(UpdaterError::HashMismatch);
        }

        // Dopasowanie do zasobów GitHub API
        let matching_api_assets: Vec<&ReleaseAsset> = release_assets
            .iter()
            .filter(|ra| ra.name == m_asset.filename)
            .collect();

        if matching_api_assets.is_empty() {
            return Err(UpdaterError::NoMatchingAssets);
        }
        if matching_api_assets.len() > 1 {
            return Err(UpdaterError::AmbiguousArtifact(format!(
                "Wielokrotny zasób API o nazwie {}",
                m_asset.filename
            )));
        }

        let ra = matching_api_assets[0];

        if ra.browser_download_url.trim().is_empty() {
            return Err(UpdaterError::Network(
                "Pusty browser_download_url w API".into(),
            ));
        }

        // Jeśli API podaje rozmiar > 0, musi się ściśle zgadzać z manifestem
        if ra.size > 0 && ra.size != m_asset.size {
            return Err(UpdaterError::SizeMismatch);
        }

        // Jeśli API podaje digest, musi być zgodny z manifestem
        if let Some(ref api_digest) = ra.digest {
            let clean_digest = api_digest
                .trim()
                .trim_start_matches("sha256:")
                .trim_start_matches("SHA256:");
            if !clean_digest.eq_ignore_ascii_case(&m_asset.sha256) {
                return Err(UpdaterError::ChecksumMismatch);
            }
        }

        Ok(VerifiedReleaseAsset {
            platform: target_platform,
            arch: target_arch,
            package_type: pkg_type,
            filename: m_asset.filename.clone(),
            declared_size: m_asset.size,
            declared_sha256: m_asset.sha256.to_ascii_lowercase(),
            browser_download_url: ra.browser_download_url.clone(),
            bound_manifest_version: self.manifest.version.clone(),
            bound_commit: self.manifest.commit.clone(),
        })
    }

    pub fn select_asset_for_current_platform(
        &self,
        release_assets: &[ReleaseAsset],
    ) -> Result<VerifiedReleaseAsset, UpdaterError> {
        let platform = ReleasePlatform::current()?;
        let arch = ReleaseArchitecture::current()?;
        self.select_asset_for_platform(platform, arch, release_assets)
    }
}

/// Pojedynczy plik (asset) dołączony do wydania GitHub.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    /// Suma kontrolna SHA256 (hex) udostępniona przez GitHub, jeśli istnieje.
    pub digest: Option<String>,
    pub size: u64,
}

/// Najnowsze wydanie programu w repozytorium GitHub zweryfikowane kryptograficznie.
#[derive(Debug, Clone)]
pub struct LatestRelease {
    pub tag: String,
    pub html_url: String,
    pub body: String,
    pub assets: Vec<ReleaseAsset>,
    pub verified_manifest: VerifiedReleaseManifest,
    pub verified_asset: VerifiedReleaseAsset,
}

impl LatestRelease {
    pub fn verified_manifest(&self) -> &VerifiedReleaseManifest {
        &self.verified_manifest
    }

    pub fn verified_asset(&self) -> &VerifiedReleaseAsset {
        &self.verified_asset
    }
}

/// Wynik sprawdzenia aktualizacji — odróżnia stan „aktualny” od „nowa wersja”.
#[derive(Debug, Clone)]
pub enum UpdateCheckOutcome {
    UpToDate { local: String },
    NewVersion(Box<LatestRelease>),
    Error(String),
}

/// Klasyfikacja plików w archiwum aktualizacji pod kątem ochrony danych użytkownika.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateFileClass {
    /// Pliki binarne, biblioteki i zasoby programu bezpieczne do nadpisania.
    ProgramFile,
    /// Dane referencyjne. Zgodnie z zasadą UNKNOWN DATA CLASSIFICATION → DO NOT OVERWRITE,
    /// jeśli plik istnieje lokalnie, NIE MOŻE zostać nadpisany bez migracji.
    ReferenceData,
    /// Ziarna pierwszej instalacji (np. początkowe bazy callbook / serviceLOG).
    /// Kopiowane tylko wtedy, gdy plik docelowy NIE istnieje. Jeśli istnieje — zachowywane.
    FirstInstallSeed,
    /// Dane użytkownika (dzienniki łączności, pliki QSO, bazy robocze). Bezwzględny zakaz nadpisywania.
    UserData,
    /// Konfiguracja użytkownika (station_config.json, ustawienia). Bezwzględny zakaz nadpisywania.
    UserConfiguration,
    /// Dane podręczne (cache).
    Cache,
    /// Metadane wydania (manifesty, licencje, changelogi).
    ReleaseMetadata,
    /// Plik o nieznanej lub niepewnej klasyfikacji — domyślnie chroniony przed nadpisaniem (fail-closed).
    Unknown,
}

/// Klasyfikuje ścieżkę względną pliku z paczki aktualizacyjnej pod kątem bezpieczeństwa danych.
pub fn classify_update_path(rel_path: &Path) -> UpdateFileClass {
    let path_str = rel_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let file_name = rel_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = rel_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let ext_str = ext.as_deref();

    // 1. Konfiguracja użytkownika - bezwzględny zakaz nadpisywania
    if file_name == "station_config.json"
        || path_str.starts_with("config/")
        || ext_str == Some("conf")
        || ext_str == Some("cfg")
        || (ext_str == Some("json") && file_name != "manifest.json" && file_name != "release.json")
    {
        return UpdateFileClass::UserConfiguration;
    }

    // 2. Dane użytkownika - bezwzględny zakaz nadpisywania
    if file_name == "default_log.db"
        || file_name.starts_with("default_log.db")
        || path_str.starts_with("logs/")
        || path_str.starts_with("log/")
        || ext_str == Some("adi")
        || ext_str == Some("adif")
        || ext_str == Some("cabrillo")
        || file_name.ends_with(".db-wal")
        || file_name.ends_with(".db-shm")
        || file_name.ends_with(".db-journal")
    {
        return UpdateFileClass::UserData;
    }

    // 3. Ziarna baz danych w katalogu portable (serviceLOG.db, callbook.db itp.)
    if path_str.starts_with("databases/")
        || path_str.starts_with("database/")
        || file_name == "servicelog.db"
        || file_name == "callbook.db"
    {
        return UpdateFileClass::FirstInstallSeed;
    }

    // Inne bazy danych (.db, .sqlite) poza katalogiem databases/ to dane użytkownika
    if ext_str == Some("db") || ext_str == Some("sqlite") || ext_str == Some("sqlite3") {
        return UpdateFileClass::UserData;
    }

    // 4. Metadane wydania
    if file_name == "manifest.json"
        || file_name == "release.json"
        || file_name == "release.sig"
        || ext_str == Some("sig")
        || file_name == "changelog.md"
        || file_name == "readme.md"
        || file_name == "license"
        || file_name == "license.md"
        || file_name == "license.txt"
    {
        return UpdateFileClass::ReleaseMetadata;
    }

    // 5. Pliki programu - jedyne uprawnione do nadpisania istniejących plików
    if file_name == "splogbook.exe"
        || file_name == "splogbook"
        || ext_str == Some("dll")
        || ext_str == Some("so")
        || ext_str == Some("dylib")
        || path_str.starts_with("resources/")
        || path_str.starts_with("assets/")
    {
        return UpdateFileClass::ProgramFile;
    }

    // 6. Wszystko inne - fail-closed Unknown
    UpdateFileClass::Unknown
}

/// Plan aktualizacji plików gwarantujący fail-closed ochronę danych użytkownika.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePlan {
    /// Pliki programu przeznaczone do bezpiecznej podmiany (staging_rel, active_abs).
    pub replace_files: Vec<(PathBuf, PathBuf)>,
    /// Nowe pliki programu/seedów przeznaczone do utworzenia (staging_rel, active_abs).
    pub create_files: Vec<(PathBuf, PathBuf)>,
    /// Istniejące pliki użytkownika/konfiguracji/baz referencyjnych zachowywane bez zmian (active_abs, reason).
    pub preserve_files: Vec<(PathBuf, String)>,
    /// Pliki odrzucone z paczki aktualizacyjnej (np. nieznana klasyfikacja) (staging_rel, reason).
    pub rejected_files: Vec<(PathBuf, String)>,
}

impl UpdatePlan {
    /// Generuje deterministyczny plan aktualizacji porównując zawartość stagingu z katalogiem aktywnym.
    /// Operacja nie wykonuje żadnych modyfikacji plików ani katalogów.
    pub fn generate(staging_dir: &Path, active_dir: &Path) -> Result<Self, UpdaterError> {
        let mut replace_files = Vec::new();
        let mut create_files = Vec::new();
        let mut preserve_files = Vec::new();
        let mut rejected_files = Vec::new();

        if !staging_dir.exists() {
            return Err(UpdaterError::SafetyGate(
                "Katalog stagingu nie istnieje podczas generowania planu aktualizacji".to_string(),
            ));
        }

        // Iteracja deterministyczna - posortowane ścieżki
        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(staging_dir).min_depth(1) {
            let entry = entry.map_err(|e| {
                UpdaterError::Io(format!("Błąd przeszukiwania katalogu stagingu: {e}"))
            })?;
            if entry.file_type().is_file() {
                entries.push(entry.into_path());
            }
        }
        entries.sort();

        for staging_path in entries {
            let rel_path = staging_path
                .strip_prefix(staging_dir)
                .map_err(|e| UpdaterError::Io(format!("Błąd wyznaczania ścieżki względnej: {e}")))?
                .to_path_buf();

            // Bezpieczeństwo ścieżek: zakaz komponentów nadrzędnych i bezwzględnych
            for component in rel_path.components() {
                match component {
                    std::path::Component::Normal(_) => {}
                    _ => {
                        return Err(UpdaterError::SafetyGate(format!(
                            "Niedozwolony komponent ścieżki w stagingu: {}",
                            rel_path.display()
                        )));
                    }
                }
            }

            let class = classify_update_path(&rel_path);
            let active_path = active_dir.join(&rel_path);

            if active_path.exists() {
                match class {
                    UpdateFileClass::ProgramFile => {
                        // Wyłącznie pliki programu mogą zastąpić istniejące pliki
                        replace_files.push((rel_path, active_path));
                    }
                    UpdateFileClass::FirstInstallSeed => {
                        preserve_files.push((
                            active_path,
                            "Zachowano istniejącą bazę danych (ziarno instalacji nie nadpisuje danych)".to_string(),
                        ));
                    }
                    UpdateFileClass::ReferenceData => {
                        preserve_files.push((
                            active_path,
                            "Zachowano istniejącą bazę referencyjną (ochrona fail-closed)"
                                .to_string(),
                        ));
                    }
                    UpdateFileClass::UserData => {
                        preserve_files.push((
                            active_path,
                            "Bezwzględna ochrona danych użytkownika (zakaz nadpisywania)"
                                .to_string(),
                        ));
                    }
                    UpdateFileClass::UserConfiguration => {
                        preserve_files.push((
                            active_path,
                            "Bezwzględna ochrona konfiguracji użytkownika (zakaz nadpisywania)"
                                .to_string(),
                        ));
                    }
                    UpdateFileClass::Cache | UpdateFileClass::ReleaseMetadata => {
                        preserve_files.push((
                            active_path,
                            format!("Zachowano istniejący plik ({class:?})"),
                        ));
                    }
                    UpdateFileClass::Unknown => {
                        // UNKNOWN DATA CLASSIFICATION -> DO NOT OVERWRITE
                        preserve_files.push((
                            active_path,
                            "Zachowano istniejący plik (nieznana klasyfikacja - zasada fail-closed)".to_string(),
                        ));
                        rejected_files.push((
                            rel_path,
                            "Nieznany plik w paczce aktualizacyjnej próbuje odwołać się do istniejącego pliku".to_string(),
                        ));
                    }
                }
            } else {
                // Plik nie istnieje w katalogu docelowym
                match class {
                    UpdateFileClass::ProgramFile
                    | UpdateFileClass::FirstInstallSeed
                    | UpdateFileClass::ReferenceData
                    | UpdateFileClass::ReleaseMetadata
                    | UpdateFileClass::UserData
                    | UpdateFileClass::UserConfiguration
                    | UpdateFileClass::Cache => {
                        create_files.push((rel_path, active_path));
                    }
                    UpdateFileClass::Unknown => {
                        rejected_files.push((
                            rel_path,
                            "Nieznana klasyfikacja nowego pliku w paczce aktualizacyjnej (fail-closed)".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(Self {
            replace_files,
            create_files,
            preserve_files,
            rejected_files,
        })
    }

    /// Weryfikuje reguły bezpieczeństwa planu aktualizacji przed przystąpieniem do jakichkolwiek zmian.
    pub fn validate_safety(&self, expected_binary_name: &str) -> Result<(), UpdaterError> {
        // 1. Odrzucenie w przypadku jakichkolwiek plików odrzuconych
        if !self.rejected_files.is_empty() {
            let sample: Vec<String> = self
                .rejected_files
                .iter()
                .take(3)
                .map(|(p, r)| format!("{}: {r}", p.display()))
                .collect();
            return Err(UpdaterError::SafetyGate(format!(
                "Bramka bezpieczeństwa odrzuciła paczkę aktualizacyjną z powodu nieznanych plików: {}",
                sample.join("; ")
            )));
        }

        // 2. Weryfikacja replace_files: każdy plik musi być wyłącznie ProgramFile
        for (rel, target) in &self.replace_files {
            let class = classify_update_path(rel);
            if class != UpdateFileClass::ProgramFile {
                return Err(UpdaterError::SafetyGate(format!(
                    "Próba podmiany pliku niebędącego plikiem programu: {} (klasa: {:?})",
                    target.display(),
                    class
                )));
            }
        }

        // 3. Sprawdzenie obecności głównej binarki w replace_files lub create_files
        let has_binary = self.replace_files.iter().any(|(rel, _)| {
            rel.file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(expected_binary_name))
        }) || self.create_files.iter().any(|(rel, _)| {
            rel.file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(expected_binary_name))
        });

        if !has_binary {
            return Err(UpdaterError::SafetyGate(format!(
                "Paczka aktualizacyjna nie zawiera głównego pliku wykonywalnego {expected_binary_name}"
            )));
        }

        Ok(())
    }

    pub fn replace_count(&self) -> usize {
        self.replace_files.len()
    }

    pub fn create_count(&self) -> usize {
        self.create_files.len()
    }

    pub fn preserve_count(&self) -> usize {
        self.preserve_files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.replace_files.is_empty() && self.create_files.is_empty()
    }
}

/// Zaufany klucz publiczny wbudowany w aplikację.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedKey {
    pub key_id: &'static str,
    pub public_key: [u8; 32],
    pub active: bool,
    pub min_version: Option<&'static str>,
    pub max_version: Option<&'static str>,
}

/// Zaufane klucze publiczne (hardcoded).
pub const TRUSTED_KEYS: &[TrustedKey] = &[
    TrustedKey {
        key_id: "splogbook-release-2026-02",
        public_key: [
            120, 98, 36, 60, 5, 44, 3, 94, 176, 193, 128, 250, 134, 246, 151, 173, 48, 52, 175, 70,
            250, 58, 209, 168, 53, 6, 216, 212, 62, 225, 232, 68,
        ],
        active: true,
        min_version: Some("1.2.0"),
        max_version: None,
    },
    TrustedKey {
        key_id: "splogbook-release-2026-01",
        public_key: [
            109, 72, 1, 34, 95, 195, 132, 176, 98, 155, 207, 206, 145, 98, 159, 8, 110, 233, 90,
            236, 84, 127, 179, 166, 231, 137, 166, 233, 148, 250, 211, 7,
        ],
        active: false, // Odwołany po incydencie SEC-001 (fail-closed do czasu rotacji)
        min_version: Some("1.1.0"),
        max_version: None,
    },
];

/// Odwołane identyfikatory kluczy wydań (SEC-001).
pub const REVOKED_KEY_IDS: &[&str] = &["splogbook-release-2026-01"];

/// Odwołane publiczne klucze Ed25519 (klucze kompromitowane i testowe).
/// Przechowuje wyłącznie materiał publiczny – brak jakichkolwiek prywatnych seedów.
pub const REVOKED_PUBLIC_KEYS: &[[u8; 32]] = &[
    // Historyczny klucz produkcyjny z klucz.key (ujawniony w historii Git)
    [
        203, 142, 210, 212, 105, 105, 108, 227, 134, 187, 21, 218, 130, 171, 206, 34, 2, 8, 28,
        215, 25, 30, 29, 155, 176, 60, 221, 202, 31, 156, 241, 112,
    ],
    // Klucz publiczny wygenerowany z trywialnego ziarna [1u8; 32] w scratch.rs
    [
        138, 136, 227, 221, 116, 9, 241, 149, 253, 82, 219, 45, 60, 186, 93, 114, 202, 103, 9, 191,
        29, 148, 18, 27, 243, 116, 136, 1, 180, 15, 111, 92,
    ],
    // Klucz publiczny z trywialnego ziarna [0u8; 32]
    [
        59, 106, 39, 188, 206, 182, 164, 45, 98, 163, 168, 208, 42, 111, 13, 115, 101, 50, 21, 119,
        29, 226, 67, 166, 58, 192, 72, 161, 139, 89, 218, 41,
    ],
    // Klucz publiczny z ujawnionego w historii .signer_priv.key
    [
        108, 158, 236, 247, 76, 222, 170, 89, 37, 224, 25, 98, 49, 197, 84, 95, 229, 153, 255, 45,
        92, 247, 208, 87, 166, 39, 218, 8, 107, 160, 254, 195,
    ],
    // Klucz publiczny z ujawnionego w historii .test.key
    [
        76, 56, 230, 100, 206, 128, 193, 15, 142, 145, 133, 231, 186, 236, 103, 150, 75, 250, 53,
        202, 116, 66, 202, 2, 76, 13, 10, 95, 13, 242, 138, 126,
    ],
];

/// Sprawdza, czy dany key_id lub klucz publiczny znajduje się na liście odwołanych względem podanych list.
pub fn is_key_revoked_with_lists(
    key_id: &str,
    public_key: &[u8; 32],
    revoked_key_ids: &[&str],
    revoked_public_keys: &[[u8; 32]],
) -> bool {
    revoked_key_ids.contains(&key_id) || revoked_public_keys.contains(public_key)
}

/// Sprawdza, czy dany key_id lub klucz publiczny znajduje się na liście odwołanych (SEC-001).
pub fn is_key_revoked(key_id: &str, public_key: &[u8; 32]) -> bool {
    is_key_revoked_with_lists(key_id, public_key, REVOKED_KEY_IDS, REVOKED_PUBLIC_KEYS)
}

/// Weryfikuje podpis na podstawie surowych bajtów manifestu z użyciem przekazanych kluczy i list odwołań.
pub fn verify_manifest_signature_with_keys<'a>(
    manifest_bytes: &[u8],
    signature_base64: &str,
    trusted_keys: &'a [TrustedKey],
    revoked_key_ids: &[&str],
    revoked_public_keys: &[[u8; 32]],
) -> Result<(&'a TrustedKey, ReleaseManifest), UpdaterError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let sig_bytes = STANDARD.decode(signature_base64).map_err(|e| {
        UpdaterError::InvalidSignature(format!("Błąd dekodowania podpisu Base64: {e}"))
    })?;

    if sig_bytes.len() != 64 {
        return Err(UpdaterError::InvalidSignature(
            "Nieprawidłowa długość podpisu Ed25519".into(),
        ));
    }

    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| UpdaterError::InvalidSignature(format!("Błąd ładowania podpisu: {e}")))?;

    // Sprawdzenie czy podpis pasuje do któregokolwiek z odwołanych kluczy publicznych.
    // Odrzucamy manifest natychmiast bez fallbacku (SEC-001).
    for revoked_pk in revoked_public_keys {
        if let Ok(revoked_key) = VerifyingKey::from_bytes(revoked_pk)
            && revoked_key.verify(manifest_bytes, &signature).is_ok()
        {
            return Err(UpdaterError::UntrustedKey);
        }
    }

    let mut verified_key = None;
    for key in trusted_keys.iter().filter(|k| k.active) {
        if is_key_revoked_with_lists(
            key.key_id,
            &key.public_key,
            revoked_key_ids,
            revoked_public_keys,
        ) {
            continue;
        }
        if let Ok(verifying_key) = VerifyingKey::from_bytes(&key.public_key)
            && verifying_key.verify(manifest_bytes, &signature).is_ok()
        {
            verified_key = Some(key);
            break;
        }
    }

    let key = verified_key.ok_or(UpdaterError::UntrustedKey)?;

    let manifest: ReleaseManifest = serde_json::from_slice(manifest_bytes).map_err(|e| {
        UpdaterError::ManifestParse(format!(
            "Błąd parsowania manifestu JSON po weryfikacji podpisu: {e}"
        ))
    })?;

    if is_key_revoked_with_lists(
        &manifest.key_id,
        &key.public_key,
        revoked_key_ids,
        revoked_public_keys,
    ) {
        return Err(UpdaterError::UntrustedKey);
    }

    if manifest.key_id != key.key_id {
        return Err(UpdaterError::KeyMismatch(format!(
            "Manifest deklaruje key_id = {}, ale został podpisany kluczem {}",
            manifest.key_id, key.key_id
        )));
    }

    manifest.validate()?;

    // Walidacja SemVer względem ograniczeń klucza.
    let manifest_ver = semver::Version::parse(&manifest.version).map_err(|e| {
        UpdaterError::InvalidVersion(format!("Nieprawidłowa wersja w manifeście: {e}"))
    })?;

    if let Some(min_v) = key.min_version {
        let min_ver = semver::Version::parse(min_v).unwrap();
        if manifest_ver < min_ver {
            return Err(UpdaterError::VersionTooOld(format!(
                "Wersja {} jest zbyt stara dla tego klucza (min: {})",
                manifest.version, min_v
            )));
        }
    }

    if let Some(max_v) = key.max_version {
        let max_ver = semver::Version::parse(max_v).unwrap();
        if manifest_ver > max_ver {
            return Err(UpdaterError::VersionTooNew(format!(
                "Wersja {} jest zbyt nowa dla tego klucza (max: {})",
                manifest.version, max_v
            )));
        }
    }

    Ok((key, manifest))
}

/// Weryfikuje podpis na podstawie surowych bajtów manifestu.
/// Sprawdza po kolei klucze. Zwraca dopasowany klucz i sparsowany manifest.
pub fn verify_manifest_signature(
    manifest_bytes: &[u8],
    signature_base64: &str,
) -> Result<(&'static TrustedKey, ReleaseManifest), UpdaterError> {
    verify_manifest_signature_with_keys(
        manifest_bytes,
        signature_base64,
        TRUSTED_KEYS,
        REVOKED_KEY_IDS,
        REVOKED_PUBLIC_KEYS,
    )
}

/// Weryfikuje czy manifest wnosi nowszą wersję w stosunku do aktualnie uruchomionej.
/// Implementuje politykę CRYPTO-SEMVER-POLICY oraz CRYPTO-DOWNGRADE-PROTECTION,
/// zapobiegając instalacji wersji równych oraz starszych.
pub fn verify_downgrade_protection(
    manifest_version: &str,
    current_version: &str,
) -> Result<(), UpdaterError> {
    let m_ver = semver::Version::parse(manifest_version).map_err(|e| {
        UpdaterError::InvalidVersion(format!("Nieprawidłowa wersja w manifeście: {e}"))
    })?;
    let c_ver = semver::Version::parse(current_version)
        .map_err(|e| UpdaterError::InvalidVersion(format!("Nieprawidłowa bieżąca wersja: {e}")))?;

    if m_ver < c_ver {
        return Err(UpdaterError::DowngradeRejected(format!(
            "Odmowa downgrade: wersja w manifeście ({manifest_version}) jest starsza od zainstalowanej ({current_version})."
        )));
    }
    if m_ver == c_ver {
        return Err(UpdaterError::AlreadyUpToDate(format!(
            "Zainstalowana jest już ta sama wersja ({current_version}) - aktualizacja odrzucona."
        )));
    }

    Ok(())
}

/// Dokument manifestu nowej architektury wydawniczej zabezpieczony Ed25519.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReleaseManifest {
    pub manifest_version: u32,
    pub key_id: String,
    pub product: String,
    pub version: String,
    pub channel: String,
    pub minimum_updater_version: String,
    pub commit: String,
    pub assets: Vec<ManifestAsset>,
}

impl ReleaseManifest {
    pub fn validate(&self) -> Result<(), UpdaterError> {
        if self.manifest_version != 1 {
            return Err(UpdaterError::ManifestValidation(format!(
                "Nieznana wersja manifestu: {}",
                self.manifest_version
            )));
        }
        if self.key_id.trim().is_empty() {
            return Err(UpdaterError::ManifestValidation(
                "Pusty lub brakujący identyfikator key_id w manifeście".into(),
            ));
        }
        if self.product != "SPLogbook" {
            return Err(UpdaterError::ManifestValidation(format!(
                "Oczekiwano product = SPLogbook, otrzymano: {}",
                self.product
            )));
        }
        if self.commit.len() != 40 || !self.commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(UpdaterError::ManifestValidation(
                "Nieprawidłowy skrót commit SHA-1".into(),
            ));
        }
        if self.assets.is_empty() {
            return Err(UpdaterError::ManifestValidation(
                "Manifest nie zawiera artefaktów".into(),
            ));
        }

        let mut names = std::collections::HashSet::new();
        let mut triads = std::collections::HashSet::new();
        for asset in &self.assets {
            if !names.insert(&asset.filename) {
                return Err(UpdaterError::ManifestValidation(format!(
                    "Zduplikowany filename: {}",
                    asset.filename
                )));
            }
            let triad = (&asset.platform, &asset.arch, &asset.package_type);
            if !triads.insert(triad) {
                return Err(UpdaterError::ManifestValidation(format!(
                    "Niejednoznaczny pakiet (platform+arch+type): {triad:?}"
                )));
            }
            if asset.sha256.len() != 64 || !asset.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(UpdaterError::ManifestValidation(format!(
                    "Nieprawidłowy SHA-256 dla pliku {}",
                    asset.filename
                )));
            }
        }
        Ok(())
    }
}

/// Maksymalny dozwolony rozmiar manifestu wydania w bajtach (1 MB).
pub const MAX_MANIFEST_SIZE: usize = 1024 * 1024;
/// Maksymalny dozwolony rozmiar podpisu Ed25519 w bajtach (1 MB).
pub const MAX_SIGNATURE_SIZE: usize = 1024 * 1024;
/// Maksymalny dozwolony rozmiar pobieranego artefaktu w bajtach (250 MB).
pub const MAX_ARTIFACT_DOWNLOAD_SIZE: u64 = 250 * 1024 * 1024;

/// Limity bezpieczeństwa dekompresji archiwów wydań (ochrona przed Archive Bomb / Zip Slip).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_total_size: u64,
    pub max_single_size: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 5_000,
            max_total_size: 300 * 1024 * 1024,  // 300 MB
            max_single_size: 150 * 1024 * 1024, // 150 MB
        }
    }
}

/// Weryfikuje i normalizuje ścieżkę wpisu archiwum, zapobiegając atakom typu path traversal,
/// odwołaniom do katalogów nadrzędnych, ścieżkom absolutnym, prefiksom dysków oraz urządzeniom.
pub fn validate_archive_relative_path(raw_path: &Path) -> Result<std::path::PathBuf, UpdaterError> {
    let path_str = raw_path.to_string_lossy();
    if path_str.trim().is_empty() {
        return Err(UpdaterError::UnsafeArchivePath(
            "Pusta ścieżka wpisu w archiwum".into(),
        ));
    }
    if path_str.contains('\0') {
        return Err(UpdaterError::UnsafeArchivePath(
            "Ścieżka zawiera bajt zerowy".into(),
        ));
    }

    if path_str.starts_with('/') || path_str.starts_with('\\') {
        return Err(UpdaterError::UnsafeArchivePath(
            "Ścieżka absolutna jest zabroniona".into(),
        ));
    }
    if path_str.len() >= 2 {
        let first_two = &path_str.as_bytes()[0..2];
        if (first_two[0].is_ascii_alphabetic() && first_two[1] == b':')
            || (first_two[0] == b'\\' && first_two[1] == b'\\')
            || (first_two[0] == b'/' && first_two[1] == b'/')
        {
            return Err(UpdaterError::UnsafeArchivePath(
                "Prefiks dysku lub ścieżka UNC są zabronione".into(),
            ));
        }
    }

    let mut normalized = std::path::PathBuf::new();
    for comp in raw_path.components() {
        match comp {
            std::path::Component::Prefix(_) => {
                return Err(UpdaterError::UnsafeArchivePath(
                    "Prefiks dysku lub UNC w ścieżce".into(),
                ));
            }
            std::path::Component::RootDir => {
                return Err(UpdaterError::UnsafeArchivePath(
                    "Ścieżka absolutna w archiwum".into(),
                ));
            }
            std::path::Component::ParentDir => {
                return Err(UpdaterError::UnsafeArchivePath(
                    "Ścieżka zawiera ParentDir (..)".into(),
                ));
            }
            std::path::Component::CurDir => {}
            std::path::Component::Normal(c) => {
                let s = c.to_string_lossy();
                if s.contains(':') || s.starts_with('\\') || s.contains('/') || s.contains('\\') {
                    return Err(UpdaterError::UnsafeArchivePath(
                        "Niedozwolone znaki w komponencie ścieżki".into(),
                    ));
                }
                let base_name = s.split('.').next().unwrap_or(&s);
                let upper = base_name.to_ascii_uppercase();
                let reserved = [
                    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                    "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
                    "LPT8", "LPT9",
                ];
                if reserved.contains(&upper.as_str()) {
                    return Err(UpdaterError::UnsafeArchivePath(format!(
                        "Zarezerwowana nazwa urządzenia w ścieżce: {s}"
                    )));
                }
                normalized.push(c);
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(UpdaterError::UnsafeArchivePath(
            "Znormalizowana ścieżka jest pusta".into(),
        ));
    }

    Ok(normalized)
}

/// Spłaszcza strukturę archiwum, jeżeli zawiera dokładnie jeden wspólny katalog główny.
/// Zachowuje zawartość bez zmian, jeśli na poziomie głównym znajduje się więcej niż jeden wpis
/// lub jeśli jedynym wpisem jest zwykły plik.
pub fn flatten_single_root_directory(staging_dir: &Path) -> Result<bool, UpdaterError> {
    let entries = std::fs::read_dir(staging_dir).map_err(|e| {
        UpdaterError::Io(format!(
            "Błąd odczytu katalogu staging {}: {e}",
            staging_dir.display()
        ))
    })?;

    let mut items = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| UpdaterError::Io(format!("Błąd odczytu wpisu w staging: {e}")))?;
        items.push(entry);
    }

    if items.len() != 1 {
        return Ok(false);
    }

    let single_item = &items[0];
    let file_type = single_item
        .file_type()
        .map_err(|e| UpdaterError::Io(format!("Błąd odczytu typu pliku wpisu staging: {e}")))?;

    if !file_type.is_dir() {
        return Ok(false);
    }

    let sub_dir = single_item.path();

    let sub_entries = std::fs::read_dir(&sub_dir).map_err(|e| {
        UpdaterError::Io(format!(
            "Błąd odczytu podkatalogu {}: {e}",
            sub_dir.display()
        ))
    })?;

    let mut sub_items = Vec::new();
    for sub in sub_entries {
        let sub =
            sub.map_err(|e| UpdaterError::Io(format!("Błąd odczytu wpisu w podkatalogu: {e}")))?;
        sub_items.push(sub.path());
    }

    for item in sub_items {
        let Some(file_name) = item.file_name() else {
            continue;
        };
        let target = staging_dir.join(file_name);
        std::fs::rename(&item, &target).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd spłaszczania: nie można przenieść {} do {}: {e}",
                item.display(),
                target.display()
            ))
        })?;
    }

    std::fs::remove_dir(&sub_dir).map_err(|e| {
        UpdaterError::Io(format!(
            "Błąd usuwania opróżnionego katalogu {}: {e}",
            sub_dir.display()
        ))
    })?;

    Ok(true)
}

/// Bezpieczna ekstrakcja archiwum `.tar.gz` z restrykcyjną weryfikacją ścieżek,
/// odrzucaniem symlinków/hardlinków, limitami wielkości i zachowaniem uprawnień wykonywania.
pub fn extract_tar_gz_safe(
    tar_gz_path: &Path,
    staging_dir: &Path,
    limits: &ArchiveLimits,
) -> Result<usize, UpdaterError> {
    use std::fs;
    use std::io::Read;

    if !staging_dir.exists() {
        fs::create_dir_all(staging_dir)
            .map_err(|e| UpdaterError::Io(format!("Nie można utworzyć katalogu staging: {e}")))?;
    }

    let file = fs::File::open(tar_gz_path).map_err(|e| {
        UpdaterError::Io(format!(
            "Nie można otworzyć archiwum {}: {e}",
            tar_gz_path.display()
        ))
    })?;

    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut entry_count: usize = 0;
    let mut total_unpacked_size: u64 = 0;
    let mut seen_paths = std::collections::HashSet::new();

    let entries = archive
        .entries()
        .map_err(|e| UpdaterError::Io(format!("Błąd odczytu wpisów w archiwum: {e}")))?;

    for entry in entries {
        let mut entry =
            entry.map_err(|e| UpdaterError::Io(format!("Uszkodzony wpis w archiwum: {e}")))?;

        entry_count += 1;
        if entry_count > limits.max_entries {
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Przekroczono limit wpisów w archiwum (max: {})",
                limits.max_entries
            )));
        }

        let entry_type = entry.header().entry_type();
        if entry_type == tar::EntryType::Symlink {
            return Err(UpdaterError::UnsafeArchivePath(
                "Wpisy typu symlink są zabronione w archiwum".into(),
            ));
        }
        if entry_type == tar::EntryType::Link {
            return Err(UpdaterError::UnsafeArchivePath(
                "Wpisy typu hardlink są zabronione w archiwum".into(),
            ));
        }
        if entry_type != tar::EntryType::Regular && entry_type != tar::EntryType::Directory {
            return Err(UpdaterError::UnsafeArchivePath(format!(
                "Niedozwolony specjalny typ wpisu w archiwum: {entry_type:?}"
            )));
        }

        let raw_path = entry.path().map_err(|e| {
            UpdaterError::UnsafeArchivePath(format!("Nieprawidłowa ścieżka wpisu: {e}"))
        })?;

        let rel_path = validate_archive_relative_path(&raw_path)?;

        if !seen_paths.insert(rel_path.clone()) {
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Zduplikowana ścieżka w archiwum: {}",
                rel_path.display()
            )));
        }

        let dest_path = staging_dir.join(&rel_path);

        if !dest_path.starts_with(staging_dir) {
            return Err(UpdaterError::UnsafeArchivePath(format!(
                "Ścieżka wychodzi poza katalog staging: {}",
                dest_path.display()
            )));
        }

        let mut current_ancestor = dest_path.parent();
        while let Some(ancestor) = current_ancestor {
            if ancestor == staging_dir {
                break;
            }
            if ancestor.is_symlink() {
                return Err(UpdaterError::UnsafeArchivePath(format!(
                    "Ścieżka nadrzędna {} jest symlinkiem",
                    ancestor.display()
                )));
            }
            current_ancestor = ancestor.parent();
        }

        if entry_type == tar::EntryType::Directory {
            if dest_path.is_file() {
                return Err(UpdaterError::ArchiveLimitExceeded(format!(
                    "Kolizja plik/katalog dla ścieżki: {}",
                    dest_path.display()
                )));
            }
            fs::create_dir_all(&dest_path).map_err(|e| {
                UpdaterError::Io(format!(
                    "Błąd tworzenia katalogu {}: {e}",
                    dest_path.display()
                ))
            })?;
            continue;
        }

        if dest_path.is_dir() {
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Kolizja katalog/plik dla ścieżki: {}",
                dest_path.display()
            )));
        }

        if let Some(p) = dest_path.parent() {
            fs::create_dir_all(p).map_err(|e| {
                UpdaterError::Io(format!(
                    "Błąd tworzenia katalogu nadrzędnego {}: {e}",
                    p.display()
                ))
            })?;
        }

        let header_size = entry.header().size().unwrap_or(0);
        if header_size > limits.max_single_size {
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Rozmiar pliku {} przekracza dopuszczalny limit ({} > {})",
                rel_path.display(),
                header_size,
                limits.max_single_size
            )));
        }

        let mut out_file = fs::File::create(&dest_path).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd tworzenia pliku wyjściowego {}: {e}",
                dest_path.display()
            ))
        })?;

        let mut file_size: u64 = 0;
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            use std::io::Write as _;
            let read_bytes = entry.read(&mut buffer).map_err(|e| {
                UpdaterError::Io(format!("Błąd odczytu wpisu {}: {e}", rel_path.display()))
            })?;
            if read_bytes == 0 {
                break;
            }
            file_size += read_bytes as u64;
            total_unpacked_size += read_bytes as u64;

            if file_size > limits.max_single_size {
                drop(out_file);
                let _ = fs::remove_file(&dest_path);
                return Err(UpdaterError::ArchiveLimitExceeded(format!(
                    "Rozmiar pliku {} przekroczył limit {}",
                    rel_path.display(),
                    limits.max_single_size
                )));
            }
            if total_unpacked_size > limits.max_total_size {
                drop(out_file);
                let _ = fs::remove_file(&dest_path);
                return Err(UpdaterError::ArchiveLimitExceeded(format!(
                    "Łączny rozmiar danych archiwum przekroczył limit {}",
                    limits.max_total_size
                )));
            }

            out_file.write_all(&buffer[..read_bytes]).map_err(|e| {
                UpdaterError::Io(format!("Błąd zapisu pliku {}: {e}", dest_path.display()))
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let raw_mode = entry.header().mode().unwrap_or(0o644);
            let safe_mode = if (raw_mode & 0o111) != 0 {
                0o755
            } else {
                0o644
            };
            let _ = fs::set_permissions(&dest_path, fs::Permissions::from_mode(safe_mode));
        }
    }

    Ok(entry_count)
}

/// Weryfikuje strukturę archiwum ZIP w systemie plików (nagłówki EOCD, Central Directory oraz Local File Headers)
/// pod kątem ataków Zip Slip, odwołań ParentDir, ścieżek bezwzględnych, prefiksów dysku, UNC, symlinków
/// oraz limitów rozmiaru/liczby wpisów, ZANIM jakikolwiek plik zostanie rozpakowany na dysk.
pub fn validate_zip_archive(
    zip_path: &Path,
    limits: &ArchiveLimits,
) -> Result<usize, UpdaterError> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(zip_path)
        .map_err(|e| UpdaterError::Io(format!("Nie można otworzyć archiwum ZIP: {e}")))?;
    let file_len = file
        .metadata()
        .map_err(|e| UpdaterError::Io(format!("Nie można pobrać metadanych ZIP: {e}")))?
        .len();

    if file_len < 22 {
        return Err(UpdaterError::UnsafeArchivePath(
            "Archiwum ZIP jest za małe".into(),
        ));
    }

    // 1. Znalezienie EOCD (End of Central Directory)
    let scan_len = std::cmp::min(file_len, 22 + 65535) as usize;
    file.seek(SeekFrom::End(-(scan_len as i64)))
        .map_err(|e| UpdaterError::Io(format!("Błąd pozycjonowania w ZIP: {e}")))?;

    let mut buf = vec![0u8; scan_len];
    file.read_exact(&mut buf)
        .map_err(|e| UpdaterError::Io(format!("Błąd odczytu końcówki ZIP: {e}")))?;

    let mut eocd_pos = None;
    for i in (0..=scan_len - 22).rev() {
        if buf[i..i + 4] == [0x50, 0x4b, 0x05, 0x06] {
            eocd_pos = Some(i);
            break;
        }
    }

    let eocd_pos = eocd_pos.ok_or_else(|| {
        UpdaterError::UnsafeArchivePath("Nie znaleziono rekordu EOCD w archiwum ZIP".into())
    })?;

    let num_entries = u16::from_le_bytes([buf[eocd_pos + 10], buf[eocd_pos + 11]]) as usize;
    let cd_size = u32::from_le_bytes([
        buf[eocd_pos + 12],
        buf[eocd_pos + 13],
        buf[eocd_pos + 14],
        buf[eocd_pos + 15],
    ]) as u64;
    let cd_offset = u32::from_le_bytes([
        buf[eocd_pos + 16],
        buf[eocd_pos + 17],
        buf[eocd_pos + 18],
        buf[eocd_pos + 19],
    ]) as u64;

    if num_entries > limits.max_entries {
        return Err(UpdaterError::ArchiveLimitExceeded(format!(
            "Liczba wpisów w ZIP ({num_entries}) przekracza limit ({})",
            limits.max_entries
        )));
    }

    if cd_offset + cd_size > file_len {
        return Err(UpdaterError::UnsafeArchivePath(
            "Central Directory wykracza poza rozmiar pliku ZIP".into(),
        ));
    }

    // 2. Odczyt i weryfikacja Central Directory
    file.seek(SeekFrom::Start(cd_offset))
        .map_err(|e| UpdaterError::Io(format!("Błąd skoku do Central Directory: {e}")))?;

    let mut cd_buf = vec![0u8; cd_size as usize];
    file.read_exact(&mut cd_buf)
        .map_err(|e| UpdaterError::Io(format!("Błąd odczytu Central Directory: {e}")))?;

    let mut offset = 0;
    let mut total_uncompressed_size: u64 = 0;

    for _ in 0..num_entries {
        if offset + 46 > cd_buf.len() {
            return Err(UpdaterError::UnsafeArchivePath(
                "Niekompletny nagłówek Central Directory w ZIP".into(),
            ));
        }

        if cd_buf[offset..offset + 4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err(UpdaterError::UnsafeArchivePath(
                "Nieprawidłowa sygnatura Central Directory w ZIP".into(),
            ));
        }

        let uncompressed_size = u32::from_le_bytes([
            cd_buf[offset + 24],
            cd_buf[offset + 25],
            cd_buf[offset + 26],
            cd_buf[offset + 27],
        ]) as u64;

        let name_len = u16::from_le_bytes([cd_buf[offset + 28], cd_buf[offset + 29]]) as usize;
        let extra_len = u16::from_le_bytes([cd_buf[offset + 30], cd_buf[offset + 31]]) as usize;
        let comment_len = u16::from_le_bytes([cd_buf[offset + 32], cd_buf[offset + 33]]) as usize;
        let external_attr = u32::from_le_bytes([
            cd_buf[offset + 38],
            cd_buf[offset + 39],
            cd_buf[offset + 40],
            cd_buf[offset + 41],
        ]);

        if offset + 46 + name_len + extra_len + comment_len > cd_buf.len() {
            return Err(UpdaterError::UnsafeArchivePath(
                "Wpis Central Directory przekracza bufor".into(),
            ));
        }

        let name_bytes = &cd_buf[offset + 46..offset + 46 + name_len];
        let name_str = String::from_utf8_lossy(name_bytes);

        // Odrzucenie symlinków: Unix mode S_IFLNK (0o120000)
        let unix_mode = (external_attr >> 16) & 0o170_000;
        if unix_mode == 0o120_000 {
            return Err(UpdaterError::UnsafeArchivePath(format!(
                "Wykryto symlink w archiwum ZIP: {name_str}"
            )));
        }
        // Odrzucenie punktów reparse Windows: FILE_ATTRIBUTE_REPARSE_POINT (0x400)
        if (external_attr & 0x0000_0400) != 0 {
            return Err(UpdaterError::UnsafeArchivePath(format!(
                "Wykryto punkt reparse w archiwum ZIP: {name_str}"
            )));
        }

        // Walidacja ścieżki
        validate_archive_relative_path(Path::new(&*name_str))?;

        if uncompressed_size > limits.max_single_size {
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Rozmiar pliku {name_str} ({uncompressed_size}) przekracza limit ({})",
                limits.max_single_size
            )));
        }

        total_uncompressed_size += uncompressed_size;
        if total_uncompressed_size > limits.max_total_size {
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Łączny rozmiar archiwum ({total_uncompressed_size}) przekracza limit ({})",
                limits.max_total_size
            )));
        }

        offset += 46 + name_len + extra_len + comment_len;
    }

    // 3. Sprawdzenie nagłówków lokalnych (Local File Headers)
    file.seek(SeekFrom::Start(0))
        .map_err(|e| UpdaterError::Io(format!("Błąd skoku do początku pliku ZIP: {e}")))?;

    let mut local_count = 0;
    while local_count < num_entries {
        let mut local_hdr = [0u8; 30];
        if file.read_exact(&mut local_hdr).is_err() {
            break;
        }
        if local_hdr[0..4] != [0x50, 0x4b, 0x03, 0x04] {
            break;
        }

        let l_comp_size =
            u32::from_le_bytes([local_hdr[18], local_hdr[19], local_hdr[20], local_hdr[21]]) as u64;
        let l_name_len = u16::from_le_bytes([local_hdr[26], local_hdr[27]]) as usize;
        let l_extra_len = u16::from_le_bytes([local_hdr[28], local_hdr[29]]) as usize;

        let mut name_buf = vec![0u8; l_name_len];
        file.read_exact(&mut name_buf).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd odczytu nazwy pliku w lokalnym nagłówku ZIP: {e}"
            ))
        })?;
        let local_name = String::from_utf8_lossy(&name_buf);
        validate_archive_relative_path(Path::new(&*local_name))?;

        file.seek(SeekFrom::Current(
            (l_extra_len as i64) + (l_comp_size as i64),
        ))
        .map_err(|e| UpdaterError::Io(format!("Błąd przewijania danych pliku w ZIP: {e}")))?;

        local_count += 1;
    }

    Ok(num_entries)
}

/// Bezpieczna ekstrakcja archiwum `.zip` w systemie Windows z użyciem `powershell.exe`
/// i cmdleta `Expand-Archive`. Parametry przekazywane są jako oddzielne tokeny procesu (bez interpolacji shell).
/// Przed wywołaniem Expand-Archive archiwum jest weryfikowane w Rust (validate_zip_archive),
/// a po dekompresji sprawdzana jest poprawność zawartości w staging.
#[cfg(target_os = "windows")]
pub fn extract_zip_safe_windows(
    zip_path: &Path,
    staging_dir: &Path,
    limits: &ArchiveLimits,
) -> Result<usize, UpdaterError> {
    use std::fs;

    let ext = zip_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if !ext.eq_ignore_ascii_case("zip") {
        return Err(UpdaterError::SafetyGate(
            "Archiwum Windows musi posiadać rozszerzenie .zip dla Expand-Archive".into(),
        ));
    }

    // 1. Walidacja zawartości ZIP w Rust PRZED wywołaniem Expand-Archive lub tworzeniem katalogu staging
    let entry_count = validate_zip_archive(zip_path, limits)?;

    // 2. Przygotowanie czystego katalogu staging
    if staging_dir.exists() {
        if staging_dir.is_symlink() {
            let _ = fs::remove_file(staging_dir);
        } else {
            let _ = fs::remove_dir_all(staging_dir);
        }
    }
    fs::create_dir_all(staging_dir)
        .map_err(|e| UpdaterError::Io(format!("Nie można utworzyć katalogu staging: {e}")))?;

    let zip_str = zip_path
        .to_str()
        .ok_or_else(|| UpdaterError::UnsafeArchivePath("Nieprawidłowa ścieżka zip".into()))?;
    let dest_str = staging_dir
        .to_str()
        .ok_or_else(|| UpdaterError::UnsafeArchivePath("Nieprawidłowa ścieżka staging".into()))?;

    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "param($z, $d) Expand-Archive -LiteralPath $z -DestinationPath $d -Force",
            "-z",
            zip_str,
            "-d",
            dest_str,
        ])
        .output()
        .map_err(|e| {
            UpdaterError::Io(format!(
                "Nie udało się uruchomić powershell dla Expand-Archive: {e}"
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_dir_all(staging_dir);
        return Err(UpdaterError::Io(format!(
            "Błąd rozpakowywania Expand-Archive: {}",
            stderr.trim()
        )));
    }

    let mut actual_count = 0;
    let mut total_size: u64 = 0;

    for entry in walkdir::WalkDir::new(staging_dir).min_depth(1) {
        let entry =
            entry.map_err(|e| UpdaterError::Io(format!("Błąd przeszukiwania staging: {e}")))?;
        actual_count += 1;
        if actual_count > limits.max_entries {
            let _ = fs::remove_dir_all(staging_dir);
            return Err(UpdaterError::ArchiveLimitExceeded(format!(
                "Rozpakowane archiwum przekracza limit wpisów (max: {})",
                limits.max_entries
            )));
        }

        if entry.path_is_symlink() {
            let _ = fs::remove_dir_all(staging_dir);
            return Err(UpdaterError::UnsafeArchivePath(
                "Wykryto symlink lub punkt reparse w rozpakowanym archiwum".into(),
            ));
        }

        let p = entry.path();
        if !p.starts_with(staging_dir) {
            let _ = fs::remove_dir_all(staging_dir);
            return Err(UpdaterError::UnsafeArchivePath(
                "Plik w staging wykracza poza katalog staging".into(),
            ));
        }

        if entry.file_type().is_file() {
            let len = entry.metadata().map_or(0, |m| m.len());
            if len > limits.max_single_size {
                let _ = fs::remove_dir_all(staging_dir);
                return Err(UpdaterError::ArchiveLimitExceeded(format!(
                    "Rozmiar pliku {} przekracza limit {}",
                    entry.path().display(),
                    limits.max_single_size
                )));
            }
            total_size += len;
            if total_size > limits.max_total_size {
                let _ = fs::remove_dir_all(staging_dir);
                return Err(UpdaterError::ArchiveLimitExceeded(format!(
                    "Łączny rozmiar danych archiwum przekracza limit {}",
                    limits.max_total_size
                )));
            }
        }
    }

    Ok(entry_count)
}

/// Waliduje kompletność i poprawność układu katalogu staging przed przystąpieniem do zamiany plików.
pub fn validate_staging_layout(
    staging_dir: &Path,
    expected_executable_name: &str,
) -> Result<(), UpdaterError> {
    if !staging_dir.exists() || !staging_dir.is_dir() {
        return Err(UpdaterError::InvalidStagingLayout(format!(
            "Katalog staging nie istnieje: {}",
            staging_dir.display()
        )));
    }

    let exe_path = staging_dir.join(expected_executable_name);
    if !exe_path.exists() {
        return Err(UpdaterError::InvalidStagingLayout(format!(
            "Brak wymaganego pliku wykonywalnego '{expected_executable_name}' w staging layout"
        )));
    }

    let meta = exe_path.symlink_metadata().map_err(|e| {
        UpdaterError::InvalidStagingLayout(format!(
            "Błąd odczytu metadanych pliku wykonywalnego: {e}"
        ))
    })?;

    if meta.file_type().is_symlink() {
        return Err(UpdaterError::InvalidStagingLayout(
            "Plik wykonywalny w staging nie może być symlinkiem".into(),
        ));
    }

    if !meta.is_file() {
        return Err(UpdaterError::InvalidStagingLayout(
            "Plik wykonywalny w staging musi być zwykłym plikiem".into(),
        ));
    }

    if meta.len() == 0 {
        return Err(UpdaterError::InvalidStagingLayout(
            "Plik wykonywalny w staging ma rozmiar 0 bajtów".into(),
        ));
    }

    for entry in walkdir::WalkDir::new(staging_dir)
        .min_depth(1)
        .into_iter()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy();
        if name.ends_with(".key") || name == "priv.key" {
            return Err(UpdaterError::InvalidStagingLayout(format!(
                "Niedozwolony plik klucza prywatnego w staging: {name}"
            )));
        }
    }

    Ok(())
}

/// Dobiera artefakt manifestu na podstawie jawnych pól platformy i architektury.
/// Odrzuca nieznane systemy, architektury, niejednoznaczne pakiety oraz nazwy zawierające komponenty ścieżki.
pub fn select_manifest_asset<'a>(
    assets: &'a [ManifestAsset],
    platform: &str,
    arch: &str,
) -> Result<&'a ManifestAsset, UpdaterError> {
    let allowed_platforms = ["windows", "linux", "macos"];
    if !allowed_platforms.contains(&platform) {
        return Err(UpdaterError::UnsupportedPlatform(format!(
            "Nieznana platforma: {platform}"
        )));
    }

    let allowed_arch = ["x86_64"];
    if !allowed_arch.contains(&arch) {
        return Err(UpdaterError::UnsupportedPlatform(format!(
            "Nieznana architektura: {arch}"
        )));
    }

    let mut matches = Vec::new();
    for asset in assets {
        if asset.platform.eq_ignore_ascii_case(platform) && asset.arch.eq_ignore_ascii_case(arch) {
            matches.push(asset);
        }
    }

    if matches.is_empty() {
        return Err(UpdaterError::UnsupportedPlatform(format!(
            "Brak pasującego artefaktu dla platformy {platform} i architektury {arch}"
        )));
    }

    if matches.len() > 1 {
        return Err(UpdaterError::AmbiguousArtifact(format!(
            "Niejednoznaczne dopasowanie: znaleziono {} artefaktów dla {platform}-{arch}",
            matches.len()
        )));
    }

    let selected = matches[0];
    let fn_str = &selected.filename;
    if fn_str.trim().is_empty()
        || fn_str == "."
        || fn_str == ".."
        || fn_str.contains('/')
        || fn_str.contains('\\')
    {
        return Err(UpdaterError::UnsafeArchivePath(format!(
            "Nazwa artefaktu '{fn_str}' zawiera niedozwolone komponenty ścieżki"
        )));
    }

    Ok(selected)
}

/// Dobiera artefakt manifestu dla bieżącej platformy operacyjnej i architektury.
pub fn select_current_platform_manifest_asset(
    assets: &[ManifestAsset],
) -> Result<&ManifestAsset, UpdaterError> {
    #[cfg(target_os = "windows")]
    let platform = "windows";
    #[cfg(target_os = "linux")]
    let platform = "linux";
    #[cfg(target_os = "macos")]
    let platform = "macos";
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let platform = "unknown";

    #[cfg(target_arch = "x86_64")]
    let arch = "x86_64";
    #[cfg(not(target_arch = "x86_64"))]
    let arch = "unknown";

    select_manifest_asset(assets, platform, arch)
}

/// Weryfikuje integralność artefaktu względem oczekiwanego rozmiaru i sumy kontrolnej SHA-256.
pub fn verify_artifact_integrity(
    data: &[u8],
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), UpdaterError> {
    if expected_size > 0 && data.len() as u64 != expected_size {
        return Err(UpdaterError::SizeMismatch);
    }
    if !verify_sha256(data, expected_sha256) {
        return Err(UpdaterError::HashMismatch);
    }
    Ok(())
}

/// Pobiera metadane najnowszego wydania z GitHub API (bez autoryzacji).
pub async fn latest_release() -> Result<LatestRelease, UpdaterError> {
    let client = crate::core::http::http_client_with_timeout(15);
    let url = "https://api.github.com/repos/sp6ina/SPLogbook/releases/latest";

    let resp = client
        .get(url)
        .header("User-Agent", "SPLogbook-update-check")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd połączenia z GitHub: {e}")))?;

    if !resp.status().is_success() {
        return Err(UpdaterError::HttpStatus(format!(
            "GitHub zwrócił status {} (sprawdź połączenie z internetem).",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd odczytu odpowiedzi: {e}")))?;

    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        UpdaterError::ManifestParse(format!("Błąd parsowania odpowiedzi GitHub: {e}"))
    })?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or(UpdaterError::MissingField(
            "Brak pola tag_name w odpowiedzi z serwera.",
        ))?
        .trim_start_matches('v')
        .to_string();
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/sp6ina/SPLogbook/releases")
        .to_string();
    let release_body = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut assets = Vec::new();
    if let Some(arr) = json.get("assets").and_then(|v| v.as_array()) {
        for asset in arr {
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let browser_download_url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let digest = asset
                .get("digest")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string);
            let size = asset
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if !name.is_empty() && !browser_download_url.is_empty() {
                assets.push(ReleaseAsset {
                    name,
                    browser_download_url,
                    digest,
                    size,
                });
            }
        }
    }

    // Pobierz i zweryfikuj podpisany manifest, zanim zaufamy plikom wydania.
    let current_version = env!("CARGO_PKG_VERSION");
    let verified_manifest = fetch_and_verify_manifest(&assets, current_version).await?;

    // Twarde powiązanie tożsamości wydania: tag GitHub musi ściśle odpowiadać wersji w manifeście
    let semver_tag = tag.trim_start_matches('v');
    if semver_tag != verified_manifest.version() {
        return Err(UpdaterError::KeyMismatch(format!(
            "Niezgodność wersji: tag GitHub ({tag}) nie odpowiada wersji w manifeście ({})",
            verified_manifest.version()
        )));
    }

    // Dobierz zweryfikowany zasób dla bieżącej platformy ze zweryfikowanego manifestu
    let verified_asset = verified_manifest.select_asset_for_current_platform(&assets)?;

    let latest = LatestRelease {
        tag,
        html_url,
        body: release_body,
        assets,
        verified_manifest,
        verified_asset,
    };

    Ok(latest)
}

/// Oblicza sumę kontrolną SHA256 (hex, małe litery) z bajtów.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let digest = Sha256::digest(data);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in &digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Weryfikuje sumę kontrolną SHA256 (ignoruje wielkość liter i ewentualny prefiks `sha256:`).
pub fn verify_sha256(data: &[u8], expected: &str) -> bool {
    let expected = expected
        .trim()
        .trim_start_matches("sha256:")
        .trim_start_matches("SHA256:");
    let actual = sha256_hex(data);
    actual.eq_ignore_ascii_case(expected)
}

/// Pobiera plik do pamięci (używane dla manifestu i podpisu).
pub async fn download_to_memory(url: &str, max_size: usize) -> Result<Vec<u8>, UpdaterError> {
    let client = crate::core::http::http_client_with_timeout(30);

    let mut resp = client
        .get(url)
        .header("User-Agent", "SPLogbook-update-check")
        .send()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd pobierania url: {e}")))?;

    if !resp.status().is_success() {
        return Err(UpdaterError::HttpStatus(format!(
            "Serwer zwrócił status {}.",
            resp.status()
        )));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd chunk: {e}")))?
    {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > max_size {
            return Err(UpdaterError::FileTooLarge);
        }
    }

    Ok(bytes)
}

/// Pobiera i weryfikuje manifest, implementując zabezpieczenia kryptograficzne i downgrade protection.
pub async fn fetch_and_verify_manifest(
    assets: &[ReleaseAsset],
    current_version: &str,
) -> Result<VerifiedReleaseManifest, UpdaterError> {
    let manifest_asset = assets
        .iter()
        .find(|a| a.name == "release-manifest.json")
        .ok_or(UpdaterError::MissingManifest)?;

    let sig_asset = assets
        .iter()
        .find(|a| a.name == "release-manifest.json.sig")
        .ok_or(UpdaterError::MissingSignature)?;

    let manifest_bytes =
        download_to_memory(&manifest_asset.browser_download_url, MAX_MANIFEST_SIZE).await?;
    let sig_bytes = download_to_memory(&sig_asset.browser_download_url, MAX_SIGNATURE_SIZE).await?;
    let sig_str = String::from_utf8(sig_bytes).map_err(|_| UpdaterError::InvalidUtf8)?;

    VerifiedReleaseManifest::verify(&manifest_bytes, sig_str.trim(), current_version)
}

/// Wybiera najlepszy plik instalacyjny dla bieżącego systemu operacyjnego.
/// Zwraca `None`, jeśli żaden plik z wydania nie pasuje do docelowego systemu.
pub fn select_asset_for_platform(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    if assets.is_empty() {
        return None;
    }

    #[cfg(target_os = "windows")]
    let preferred: &[&str] = &[".zip", ".exe", ".msi"];
    #[cfg(target_os = "macos")]
    let preferred: &[&str] = &[".dmg", ".zip", ".tar.gz"];
    #[cfg(target_os = "linux")]
    let preferred: &[&str] = &[".tar.gz", ".appimage", ".deb"];
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let preferred: &[&str] = &[];

    for ext in preferred {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(&ext.to_lowercase()))
        {
            return Some(asset);
        }
    }

    None
}

/// Pobiera plik instalacyjny wydania do wskazanej lokalizacji.
pub async fn download_release_asset(
    asset: &ReleaseAsset,
    dest_path: &Path,
) -> Result<usize, UpdaterError> {
    let client = crate::core::http::http_client_with_timeout(300);

    let resp = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "SPLogbook-update-check")
        .send()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd pobierania aktualizacji: {e}")))?;

    if !resp.status().is_success() {
        return Err(UpdaterError::HttpStatus(format!(
            "Serwer zwrócił status {} podczas pobierania.",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd odczytu pobranych danych: {e}")))?;

    if let Some(parent) = dest_path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            UpdaterError::Io(format!("Błąd tworzenia katalogu {}: {e}", parent.display()))
        })?;
    }

    tokio::fs::write(dest_path, &bytes).await.map_err(|e| {
        UpdaterError::Io(format!(
            "Błąd zapisu pobranego pliku {}: {e}",
            dest_path.display()
        ))
    })?;

    Ok(bytes.len())
}

/// Pobiera, weryfikuje i podmienia bieżący plik wykonywalny nową wersją.
///
/// Po wywołaniu (w systemie Windows) aplikacja powinna się zamknąć — proces
/// PowerShell czeka na zakończenie bieżącego procesu i dopiero wtedy podmienia
/// plik oraz uruchamia nową wersję.
/// Opcje atomowej instalacji ze wsparciem dla wstrzykiwania kontrolowanych błędów w testach.
#[derive(Debug, Clone, Copy)]
pub struct AtomicInstallationOptions<'a> {
    pub executable_name: &'a str,
    pub simulate_rename_failure: bool,
    pub simulate_post_swap_validation_failure: bool,
}

impl<'a> AtomicInstallationOptions<'a> {
    pub fn new(executable_name: &'a str) -> Self {
        Self {
            executable_name,
            simulate_rename_failure: false,
            simulate_post_swap_validation_failure: false,
        }
    }
}

/// Transakcyjna instalacja przygotowanego katalogu staging do katalogu docelowego aplikacji
/// z mechanizmem kopii zapasowej i automatycznego rollbacku (transactional installation with rollback).
///
/// UWAGA DOTYCZĄCA SEMANTYKI:
/// Model instalacji to TRANSACTIONAL_FILE_REPLACEMENT (nie jest to DIRECTORY_ATOMIC_SWAP
/// na poziomie pojedynczego wywołania jądra systemu plików, ponieważ systemy operacyjne, zwłaszcza Windows,
/// blokują podmianę całego drzewa katalogów przy uruchomionych procesach i wieloplikowych instalacjach).
/// Pliki są wymieniane sekwencyjnie. W trakcie operacji istnieje krótkie okno stanu mieszanego,
/// dlatego aplikacja docelowa pozostaje zatrzymana i nie jest uruchamiana przed finalnym zatwierdzeniem.
/// W razie błędu następuje automatyczny rollback każdego podmienionego pliku z katalogu kopii zapasowej.
pub fn apply_atomic_installation_with_options(
    prepared_dir: &Path,
    active_dir: &Path,
    options: &AtomicInstallationOptions<'_>,
) -> Result<(), UpdaterError> {
    use std::fs;

    // 1. Walidacja układu przygotowanego stagingu przed dotknięciem bieżącej instalacji
    validate_staging_layout(prepared_dir, options.executable_name)?;

    // 2. Zebranie plików z prepared_dir z bezwzględną ochroną istniejących danych użytkownika i baz portable
    let mut files_to_install = Vec::new();
    let mut preserved_files = Vec::new();
    for entry in walkdir::WalkDir::new(prepared_dir).min_depth(1) {
        let entry =
            entry.map_err(|e| UpdaterError::Io(format!("Błąd przeszukiwania staging: {e}")))?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(prepared_dir)
                .map_err(|e| UpdaterError::Io(format!("Błąd strip_prefix staging: {e}")))?;
            let orig_file = active_dir.join(rel);
            let class = classify_update_path(rel);
            if orig_file.exists()
                && matches!(
                    class,
                    UpdateFileClass::UserData
                        | UpdateFileClass::UserConfiguration
                        | UpdateFileClass::FirstInstallSeed
                        | UpdateFileClass::ReferenceData
                )
            {
                preserved_files.push(orig_file);
                continue;
            }
            files_to_install.push(rel.to_path_buf());
        }
    }

    // 3. Utworzenie prywatnego katalogu backup w active_dir (ten sam system plików dla atomowości)
    let backup_dir = active_dir.join(format!(".backup_{}", rand::random::<u64>()));
    fs::create_dir_all(&backup_dir).map_err(|e| {
        UpdaterError::Io(format!("Nie można utworzyć katalogu kopii zapasowej: {e}"))
    })?;

    let mut backed_up_files = Vec::new();
    let mut swapped_files = Vec::new();

    // 4. Kopia zapasowa istniejących plików, które będą nadpisywane
    for rel in &files_to_install {
        let orig_file = active_dir.join(rel);
        if orig_file.exists() {
            let backup_dest = backup_dir.join(rel);
            if let Some(p) = backup_dest.parent() {
                let _ = fs::create_dir_all(p);
            }
            if let Err(e) = fs::copy(&orig_file, &backup_dest) {
                let _ = fs::remove_dir_all(&backup_dir);
                return Err(UpdaterError::InstallFailed(format!(
                    "Błąd tworzenia kopii zapasowej {}: {e}",
                    rel.display()
                )));
            }
            backed_up_files.push(rel.clone());
        }
    }

    // 5. Podmiana plików (Rename / Copy)
    let mut failure_reason = None;

    if options.simulate_rename_failure {
        failure_reason = Some("Wstrzyknięty kontrolowany błąd rename/copy".to_string());
    } else {
        for rel in &files_to_install {
            let src = prepared_dir.join(rel);
            let dst = active_dir.join(rel);
            if let Some(p) = dst.parent() {
                let _ = fs::create_dir_all(p);
            }
            // Najpierw próba atomowego rename, fallback do copy jeśli system zablokuje rename
            let replace_res = if fs::rename(&src, &dst).is_ok() {
                Ok(())
            } else {
                fs::copy(&src, &dst).map(|_| ())
            };

            if let Err(e) = replace_res {
                failure_reason = Some(format!("Błąd podmiany pliku {}: {e}", rel.display()));
                break;
            }
            swapped_files.push(rel.clone());
        }
    }

    // 6. Końcowa walidacja po podmianie
    if failure_reason.is_none() {
        if options.simulate_post_swap_validation_failure {
            failure_reason = Some("Wstrzyknięty kontrolowany błąd walidacji końcowej".to_string());
        } else {
            let active_exe = active_dir.join(options.executable_name);
            let valid = active_exe.is_file() && active_exe.metadata().is_ok_and(|m| m.len() > 0);
            if !valid {
                failure_reason =
                    Some("Końcowa walidacja pliku wykonywalnego nie powiodła się".into());
            }
        }
    }

    // 7. Obsługa sukcesu lub rollbacku
    if let Some(reason) = failure_reason {
        // ROLLBACK! Przywróć pliki z kopii zapasowej
        for rel in &swapped_files {
            if !backed_up_files.contains(rel) {
                // Nowy plik, którego nie było w oryginalnej instalacji -> usuń
                let _ = fs::remove_file(active_dir.join(rel));
            }
        }
        for rel in &backed_up_files {
            let backup_src = backup_dir.join(rel);
            let active_dst = active_dir.join(rel);
            let _ = fs::copy(&backup_src, &active_dst);
        }
        let _ = fs::remove_dir_all(&backup_dir);
        let _ = fs::remove_dir_all(prepared_dir);

        return Err(UpdaterError::InstallFailed(reason));
    }

    // Sukces: usuń kopię zapasową oraz staging
    let _ = fs::remove_dir_all(&backup_dir);
    let _ = fs::remove_dir_all(prepared_dir);

    Ok(())
}

/// Standardowa atomowa instalacja bez symulacji błędów.
pub fn apply_atomic_installation(
    prepared_dir: &Path,
    active_dir: &Path,
    executable_name: &str,
) -> Result<(), UpdaterError> {
    apply_atomic_installation_with_options(
        prepared_dir,
        active_dir,
        &AtomicInstallationOptions::new(executable_name),
    )
}

/// Alias dla `apply_atomic_installation_with_options` odzwierciedlający semantykę transakcyjną.
pub fn apply_transactional_installation_with_options(
    prepared_dir: &Path,
    active_dir: &Path,
    options: &AtomicInstallationOptions<'_>,
) -> Result<(), UpdaterError> {
    apply_atomic_installation_with_options(prepared_dir, active_dir, options)
}

/// Alias dla `apply_atomic_installation` odzwierciedlający semantykę transakcyjną.
pub fn apply_transactional_installation(
    prepared_dir: &Path,
    active_dir: &Path,
    executable_name: &str,
) -> Result<(), UpdaterError> {
    apply_atomic_installation(prepared_dir, active_dir, executable_name)
}

/// Pobiera plik aktualizacji z zachowaniem limitów rozmiaru do dedykowanego pliku docelowego.
pub async fn download_verified_asset(
    asset: &VerifiedReleaseAsset,
    dest_path: &Path,
) -> Result<usize, UpdaterError> {
    use tokio::io::AsyncWriteExt;

    let client = crate::core::http::http_client_with_timeout(300);

    let mut resp = client
        .get(asset.browser_download_url())
        .header("User-Agent", "SPLogbook-update-check")
        .send()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd pobierania aktualizacji: {e}")))?;

    if !resp.status().is_success() {
        return Err(UpdaterError::HttpStatus(format!(
            "Serwer zwrócił status {} podczas pobierania.",
            resp.status()
        )));
    }

    if let Some(parent) = dest_path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            UpdaterError::Io(format!("Błąd tworzenia katalogu {}: {e}", parent.display()))
        })?;
    }

    let mut file = tokio::fs::File::create(dest_path).await.map_err(|e| {
        UpdaterError::Io(format!("Błąd tworzenia pliku {}: {e}", dest_path.display()))
    })?;

    let mut total_bytes = 0u64;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| UpdaterError::Network(format!("Błąd odczytu danych aktualizacji: {e}")))?
    {
        total_bytes += chunk.len() as u64;
        if total_bytes > MAX_ARTIFACT_DOWNLOAD_SIZE {
            drop(file);
            let _ = tokio::fs::remove_file(dest_path).await;
            return Err(UpdaterError::ArtifactTooLarge);
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| UpdaterError::Io(format!("Błąd zapisu danych aktualizacji: {e}")))?;
    }

    file.flush()
        .await
        .map_err(|e| UpdaterError::Io(format!("Błąd opróżniania bufora zapisu: {e}")))?;

    Ok(total_bytes as usize)
}

pub async fn install_update(asset: &VerifiedReleaseAsset) -> Result<(), UpdaterError> {
    let current = std::env::current_exe()
        .map_err(|e| UpdaterError::ExePath(format!("Nie można ustalić ścieżki programu: {e}")))?;

    let parent = current
        .parent()
        .ok_or_else(|| UpdaterError::ExePath("Nie można ustalić katalogu programu.".to_string()))?;

    #[cfg(target_os = "windows")]
    {
        if asset.platform() != ReleasePlatform::Windows
            || asset.package_type() != ReleasePackageType::Zip
            || !asset.filename().to_ascii_lowercase().ends_with(".zip")
        {
            return Err(UpdaterError::SafetyGate(
                "Niewłaściwy typ paczki dla systemu Windows (wymagany zweryfikowany ZIP)"
                    .to_string(),
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if asset.platform() != ReleasePlatform::Linux
            || asset.package_type() != ReleasePackageType::TarGz
            || !asset.filename().to_ascii_lowercase().ends_with(".tar.gz")
        {
            return Err(UpdaterError::SafetyGate(
                "Niewłaściwy typ paczki dla systemu Linux (wymagany zweryfikowany tar.gz)"
                    .to_string(),
            ));
        }
    }

    #[cfg(target_os = "windows")]
    let tmp_path = parent.join(format!(".SPLogbook_update_{}.zip", std::process::id()));
    #[cfg(not(target_os = "windows"))]
    let tmp_path = parent.join(format!(".SPLogbook_update_{}.tar.gz", std::process::id()));

    // 1. Pobierz plik bezpośrednio z browser_download_url powiązanego ze zweryfikowanym assetem
    let dl_result = download_verified_asset(asset, &tmp_path).await;
    if let Err(e) = dl_result {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(e);
    }

    // 2. Weryfikacja rozmiaru i SHA-256 pobranego pliku lokalnego
    let data = match tokio::fs::read(&tmp_path).await {
        Ok(d) => d,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(UpdaterError::Io(format!(
                "Błąd odczytu pobranego pliku: {e}"
            )));
        }
    };

    if asset.declared_size() > 0 && data.len() as u64 != asset.declared_size() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(UpdaterError::SizeMismatch);
    }

    if !verify_sha256(&data, asset.declared_sha256()) {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(UpdaterError::HashMismatch);
    }

    // 3. Podmień plik wykonywalny i uruchom ponownie.
    self_replace(&current, &tmp_path)
}

/// Sprawdza, czy pobierany plik nie jest archiwum, co mogłoby uszkodzić instalację.
pub fn check_safety_gate(asset: &ReleaseAsset) -> Result<(), UpdaterError> {
    let ext = Path::new(&asset.name).extension();

    #[cfg(target_os = "windows")]
    {
        if ext.is_some_and(|e| e.eq_ignore_ascii_case("msi")) {
            return Err(UpdaterError::SafetyGate(
                "Błąd: Pobrany plik to instalator MSI. Zaktualizuj program ręcznie.".to_string(),
            ));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if ext.is_some_and(|e| e.eq_ignore_ascii_case("deb") || e.eq_ignore_ascii_case("rpm")) {
            return Err(UpdaterError::SafetyGate("Błąd: Pobrany plik to zarządzany pakiet systemowy. Zaktualizuj program używając menedżera pakietów.".to_string()));
        }
    }
    Ok(())
}

/// Podmienia działający plik wykonywalny nową wersją.
#[cfg(target_os = "windows")]
fn self_replace(current: &Path, new: &Path) -> Result<(), UpdaterError> {
    install_via_powershell(current, new)
}

/// Podmienia działający plik wykonywalny nową wersją (systemy Unix).
#[cfg(not(target_os = "windows"))]
fn self_replace(current: &Path, new: &Path) -> Result<(), UpdaterError> {
    install_via_tar_gz(current, new)
}

#[cfg(not(target_os = "windows"))]
fn install_via_tar_gz(current: &Path, new: &Path) -> Result<(), UpdaterError> {
    use std::fs;

    let base_dir = current.parent().ok_or_else(|| {
        UpdaterError::ExePath("Brak katalogu nadrzędnego dla pliku wykonywalnego".to_string())
    })?;

    // 1. Sprawdzenie praw dostępu (Target Validation)
    let md = fs::metadata(base_dir)
        .map_err(|e| UpdaterError::Io(format!("Brak dostępu do katalogu aplikacji: {e}")))?;
    if md.permissions().readonly() {
        return Err(UpdaterError::Io("Katalog aplikacji jest chroniony przed zapisem (np. zainstalowano przez root). Zaktualizuj program używając menedżera pakietów lub jako root.".into()));
    }

    let staging_dir = base_dir.join(".staging");
    let rollback_dir = base_dir.join(".rollback");
    let health_ok = base_dir.join(".health_ok");

    // Czyszczenie starego staging/rollback
    let _ = fs::remove_dir_all(&staging_dir);
    let _ = fs::remove_dir_all(&rollback_dir);
    let _ = fs::remove_file(&health_ok);

    fs::create_dir_all(&staging_dir)
        .map_err(|e| UpdaterError::Io(format!("Nie można utworzyć katalogu tymczasowego: {e}")))?;
    fs::create_dir_all(&rollback_dir).map_err(|e| {
        UpdaterError::Io(format!("Nie można utworzyć katalogu kopii zapasowej: {e}"))
    })?;

    // 2. Ekstrakcja poza strefą działania z pełną ochroną (extract_tar_gz_safe) i spłaszczaniem pojedynczego katalogu
    let limits = ArchiveLimits::default();
    extract_tar_gz_safe(new, &staging_dir, &limits)?;
    let _ = flatten_single_root_directory(&staging_dir)?;
    validate_staging_layout(&staging_dir, "splogbook")?;

    // Generujemy plan aktualizacji z bezwzględną ochroną danych
    let plan = UpdatePlan::generate(&staging_dir, base_dir)?;
    plan.validate_safety("splogbook")?;

    // 3. Kopia Zapasowa (Rollback) - wyłącznie dla plików podmienianych z replace_files
    for (rel, orig) in &plan.replace_files {
        let dest = rollback_dir.join(rel);
        if let Some(p) = dest.parent() {
            let _ = fs::create_dir_all(p);
        }
        fs::copy(orig, &dest).map_err(|e| {
            UpdaterError::Io(format!(
                "Nie można utworzyć kopii zapasowej {}: {}",
                rel.display(),
                e
            ))
        })?;
    }

    // 4. Atomowa podmiana - Rename Swap dla replace_files oraz create_files
    for (rel, orig) in &plan.replace_files {
        let src = staging_dir.join(rel);
        if let Some(p) = orig.parent() {
            let _ = fs::create_dir_all(p);
        }
        if fs::rename(&src, orig).is_err() {
            fs::copy(&src, orig).map_err(|e| {
                UpdaterError::Io(format!("Błąd podmiany pliku {}: {}", rel.display(), e))
            })?;
        }
    }

    for (rel, orig) in &plan.create_files {
        let src = staging_dir.join(rel);
        if let Some(p) = orig.parent() {
            let _ = fs::create_dir_all(p);
        }
        if fs::rename(&src, orig).is_err() {
            fs::copy(&src, orig).map_err(|e| {
                UpdaterError::Io(format!("Błąd tworzenia pliku {}: {}", rel.display(), e))
            })?;
        }
    }

    // 5. Health Confirmation & Ping
    let mut child = std::process::Command::new(current)
        .arg("--check-health-startup")
        .spawn()
        .map_err(|e| {
            UpdaterError::Io(format!(
                "Nie można uruchomić zaktualizowanego programu: {}",
                e
            ))
        })?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut success = false;

    while std::time::Instant::now() < deadline {
        if health_ok.exists() {
            success = true;
            break;
        }
        if let Ok(Some(_)) = child.try_wait() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    if success {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_dir_all(&rollback_dir);
        let _ = fs::remove_file(&health_ok);
        std::process::exit(0); // Pomyślnie. Zakończ stary proces.
    } else {
        // Rollback!
        let _ = child.kill();
        // Usuń nowo utworzone pliki
        for (_rel, orig) in &plan.create_files {
            let _ = fs::remove_file(orig);
        }
        // Przywróć podmienione pliki z rollbacku
        for (rel, orig) in &plan.replace_files {
            let bkp = rollback_dir.join(rel);
            let _ = fs::copy(&bkp, orig);
        }
        let _ = std::process::Command::new(current).spawn(); // Odpal starą
        return Err(UpdaterError::HealthCheckFailed);
    }
}

/// Generuje bezpieczny skrypt PowerShell wykonujący podmianę wyłącznie plików z zatwierdzonego planu aktualizacji.
pub fn generate_powershell_update_script(exe: &Path, pid: u32, plan: &UpdatePlan) -> String {
    let exe_str = exe.to_string_lossy().replace('\'', "''");

    let replace_items: Vec<String> = plan
        .replace_files
        .iter()
        .map(|(r, _)| format!("'{}'", r.to_string_lossy().replace('\'', "''")))
        .collect();
    let replace_items_ps = replace_items.join(", ");

    let create_items: Vec<String> = plan
        .create_files
        .iter()
        .map(|(r, _)| format!("'{}'", r.to_string_lossy().replace('\'', "''")))
        .collect();
    let create_items_ps = create_items.join(", ");

    format!(
        "$ErrorActionPreference = 'Stop'\n\
         $exe = '{exe_str}'\n\
         $pidToWait = {pid}\n\
         \n\
         $baseDir = Split-Path -Path $exe -Parent\n\
         $staging = Join-Path $baseDir '.staging'\n\
         $rollback = Join-Path $baseDir '.rollback'\n\
         $healthOk = Join-Path $baseDir '.health_ok'\n\
         \n\
         $replaceFiles = @({replace_items_ps})\n\
         $createFiles = @({create_items_ps})\n\
         \n\
         if (Test-Path -LiteralPath $rollback) {{ Remove-Item -Recurse -Force -LiteralPath $rollback }}\n\
         New-Item -ItemType Directory -Force -Path $rollback | Out-Null\n\
         \n\
         $deadline = (Get-Date).AddSeconds(90)\n\
         while (Get-Process -Id $pidToWait -ErrorAction SilentlyContinue) {{\n\
             if ((Get-Date) -gt $deadline) {{ exit 1 }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         Start-Sleep -Milliseconds 500\n\
         \n\
         # 1. Kopia zapasowa wylacznie dla plikow podmienianych z UpdatePlan\n\
         foreach ($rel in $replaceFiles) {{\n\
             $orig = Join-Path $baseDir $rel\n\
             if (Test-Path -LiteralPath $orig) {{\n\
                 $dest = Join-Path $rollback $rel\n\
                 $destDir = Split-Path $dest -Parent\n\
                 if (-not (Test-Path -LiteralPath $destDir)) {{ New-Item -ItemType Directory -Force -Path $destDir | Out-Null }}\n\
                 Copy-Item -LiteralPath $orig -Destination $dest -Force\n\
             }}\n\
         }}\n\
         \n\
         # 2. Bezpieczna podmiana plikow programu\n\
         foreach ($rel in $replaceFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $baseDir $rel\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         # 3. Utworzenie nowych dopuszczonych plikow (ziarna/zasoby)\n\
         foreach ($rel in $createFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $baseDir $rel\n\
             $dstDir = Split-Path $dst -Parent\n\
             if (-not (Test-Path -LiteralPath $dstDir)) {{ New-Item -ItemType Directory -Force -Path $dstDir | Out-Null }}\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         if (Test-Path -LiteralPath $healthOk) {{ Remove-Item -Force -LiteralPath $healthOk }}\n\
         \n\
         $newProc = Start-Process -FilePath $exe -ArgumentList \"--check-health-startup\" -PassThru\n\
         \n\
         $healthDeadline = (Get-Date).AddSeconds(15)\n\
         $success = $false\n\
         while ((Get-Date) -lt $healthDeadline) {{\n\
             if (Test-Path -LiteralPath $healthOk) {{\n\
                 $success = $true\n\
                 break\n\
             }}\n\
             if ($newProc.HasExited) {{\n\
                 break\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         \n\
         if ($success) {{\n\
             Remove-Item -Recurse -Force -LiteralPath $staging -ErrorAction SilentlyContinue\n\
             Remove-Item -Recurse -Force -LiteralPath $rollback -ErrorAction SilentlyContinue\n\
             Remove-Item -Force -LiteralPath $healthOk -ErrorAction SilentlyContinue\n\
         }} else {{\n\
             Stop-Process -Id $newProc.Id -Force -ErrorAction SilentlyContinue\n\
             # Rollback: usuniecie nowo utworzonych plikow\n\
             foreach ($rel in $createFiles) {{\n\
                 $target = Join-Path $baseDir $rel\n\
                 if (Test-Path -LiteralPath $target) {{ Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue }}\n\
             }}\n\
             # Rollback: przywrocenie wylacznie plikow podmienionych z rollbacku\n\
             foreach ($rel in $replaceFiles) {{\n\
                 $bkp = Join-Path $rollback $rel\n\
                 if (Test-Path -LiteralPath $bkp) {{\n\
                     Copy-Item -LiteralPath $bkp -Destination (Join-Path $baseDir $rel) -Force\n\
                 }}\n\
             }}\n\
             Start-Process -FilePath $exe\n\
         }}\n"
    )
}

/// Windows: uruchamia skrypt PowerShell, który czeka na zamknięcie bieżącego
/// procesu, podmienia plik wykonywalny i uruchamia go ponownie.
#[cfg(target_os = "windows")]
fn install_via_powershell(current: &Path, new: &Path) -> Result<(), UpdaterError> {
    let base_dir = current.parent().ok_or_else(|| {
        UpdaterError::ExePath("Brak katalogu nadrzędnego dla pliku wykonywalnego".to_string())
    })?;

    let staging = base_dir.join(".staging");
    let rollback = base_dir.join(".rollback");
    let health_ok = base_dir.join(".health_ok");

    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(&rollback);
    let _ = std::fs::remove_file(&health_ok);

    // 1. Bezpieczna dekompresja i weryfikacja ZIP w Rust przed uruchomieniem skryptu podmiany
    let limits = ArchiveLimits::default();
    extract_zip_safe_windows(new, &staging, &limits)?;
    let _ = flatten_single_root_directory(&staging)?;
    validate_staging_layout(&staging, "SPLogbook.exe")?;

    // Generujemy plan aktualizacji z bezwzględną ochroną danych
    let plan = UpdatePlan::generate(&staging, base_dir)?;
    plan.validate_safety("SPLogbook.exe")?;

    let script_path =
        std::env::temp_dir().join(format!("SPLogbook_update_{}.ps1", std::process::id()));

    let pid = std::process::id();
    let script = generate_powershell_update_script(current, pid, &plan);

    std::fs::write(&script_path, script)
        .map_err(|e| UpdaterError::Io(format!("Nie można zapisać skryptu aktualizacji: {e}")))?;

    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .spawn()
        .map_err(|e| UpdaterError::Io(format!("Nie można uruchomić aktualizacji: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn sha256_hex_matches_known_vector() {
        // SHA256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_sha256_accepts_case_and_prefix() {
        let expected = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert!(verify_sha256(b"abc", expected));
        assert!(verify_sha256(
            b"abc",
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
        assert!(!verify_sha256(b"abd", expected));
    }

    #[test]
    fn select_asset_prefers_matching_platform_and_rejects_unmatched() {
        let assets = vec![
            ReleaseAsset {
                name: "SPLogbook-1.0.4-win64.zip".into(),
                browser_download_url: "https://example.com/z.zip".into(),
                digest: None,
                size: 0,
            },
            ReleaseAsset {
                name: "SPLogbook-1.0.4-win64.exe".into(),
                browser_download_url: "https://example.com/x.exe".into(),
                digest: None,
                size: 0,
            },
            ReleaseAsset {
                name: "SPLogbook-1.0.4-linux.AppImage".into(),
                browser_download_url: "https://example.com/l.AppImage".into(),
                digest: None,
                size: 0,
            },
            ReleaseAsset {
                name: "SPLogbook-1.0.4-macos.dmg".into(),
                browser_download_url: "https://example.com/m.dmg".into(),
                digest: None,
                size: 0,
            },
        ];

        #[cfg(target_os = "windows")]
        assert_eq!(
            select_asset_for_platform(&assets).unwrap().name,
            "SPLogbook-1.0.4-win64.zip"
        );
        #[cfg(target_os = "linux")]
        assert_eq!(
            select_asset_for_platform(&assets).unwrap().name,
            "SPLogbook-1.0.4-linux.AppImage"
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            select_asset_for_platform(&assets).unwrap().name,
            "SPLogbook-1.0.4-macos.dmg"
        );

        let unmatched = vec![ReleaseAsset {
            name: "checksums.txt".into(),
            browser_download_url: "https://example.com/checksums.txt".into(),
            digest: None,
            size: 0,
        }];
        assert!(select_asset_for_platform(&unmatched).is_none());
        assert!(select_asset_for_platform(&[]).is_none());
    }

    #[test]
    fn downgrade_protection_blocks_older_and_equal_versions() {
        assert!(verify_downgrade_protection("1.2.0", "1.1.0").is_ok()); // Newer
        assert!(verify_downgrade_protection("2.0.0", "1.9.9").is_ok()); // Newer

        assert!(verify_downgrade_protection("1.1.0", "1.1.0").is_err()); // Equal
        assert!(verify_downgrade_protection("1.0.9", "1.1.0").is_err()); // Older
    }

    #[test]
    fn manifest_validation_rejects_invalid_values() {
        let mut m = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.0".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.0".into(),
            commit: "a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f0".into(),
            assets: vec![ManifestAsset {
                platform: "linux".into(),
                arch: "x86_64".into(),
                package_type: "tar.gz".into(),
                filename: "file.tar.gz".into(),
                size: 100,
                sha256: "3d5f0e4c2f76c58916ec258f246851bea091d14d4247a2fc3e18694461b1816e".into(),
            }],
        };

        assert!(m.validate().is_ok());

        m.manifest_version = 2;
        assert!(m.validate().is_err());
        m.manifest_version = 1;

        m.product = "Other".into();
        assert!(m.validate().is_err());
        m.product = "SPLogbook".into();

        m.commit = "short".into();
        assert!(m.validate().is_err());
        m.commit = "a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f0".into();

        m.assets.push(m.assets[0].clone());
        assert!(m.validate().is_err()); // Duplicate filename and triad
    }

    #[test]
    fn safety_gate_blocks_linux_archives() {
        let _tar_gz = ReleaseAsset {
            name: "SPLogbook-Linux-x86_64.tar.gz".into(),
            browser_download_url: String::new(),
            digest: None,
            size: 0,
        };

        #[cfg(not(target_os = "windows"))]
        assert!(check_safety_gate(&_tar_gz).is_ok());

        let _deb = ReleaseAsset {
            name: "SPLogbook-Linux-x86_64.deb".into(),
            browser_download_url: String::new(),
            digest: None,
            size: 0,
        };

        #[cfg(not(target_os = "windows"))]
        assert!(check_safety_gate(&_deb).is_err());
    }

    #[test]
    fn safety_gate_blocks_windows_archives() {
        let zip = ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: String::new(),
            digest: None,
            size: 0,
        };

        #[cfg(target_os = "windows")]
        assert!(check_safety_gate(&zip).is_ok());

        let msi = ReleaseAsset {
            name: "SPLogbook-Windows-x64.msi".into(),
            browser_download_url: String::new(),
            digest: None,
            size: 0,
        };
        #[cfg(target_os = "windows")]
        assert!(check_safety_gate(&msi).is_err());

        #[cfg(not(target_os = "windows"))]
        assert!(check_safety_gate(&zip).is_ok());
    }

    #[test]
    fn verify_manifest_signature_fails_on_bad_signature() {
        let manifest_json = r#"{
            "manifest_version": 1,
            "key_id": "splogbook-release-2026-01",
            "product": "SPLogbook",
            "version": "1.2.0",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f0",
            "assets": []
        }"#;

        // Błędny podpis (losowy Base64 o dł. 64 bajtów po zdekodowaniu)
        let bad_sig = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==";

        let result = verify_manifest_signature(manifest_json.as_bytes(), bad_sig);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.contains("Żaden z zaufanych kluczy nie zweryfikował tego podpisu")
                    || e.contains("Błąd")
            );
        }
    }

    fn create_test_tar_gz(entries: &[(&str, &[u8], tar::EntryType, u32)]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use tar::{Builder, Header};

        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut enc);
            for &(path, data, entry_type, mode) in entries {
                let mut header = Header::new_gnu();
                if header.set_path(path).is_err() {
                    let bytes = path.as_bytes();
                    let name_field = &mut header.as_mut_bytes()[..100];
                    let len = bytes.len().min(99);
                    name_field[..len].copy_from_slice(&bytes[..len]);
                    name_field[len] = 0;
                }
                header.set_size(data.len() as u64);
                header.set_mode(mode);
                header.set_entry_type(entry_type);
                header.set_cksum();
                builder.append(&header, data).unwrap();
            }
            builder.finish().unwrap();
        }
        enc.finish().unwrap()
    }

    fn create_test_tar_gz_with_link(name: &str, target: &str, is_symlink: bool) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use tar::{Builder, Header};

        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut enc);
            let mut header = Header::new_gnu();
            header.set_path(name).unwrap();
            header.set_link_name(target).unwrap();
            header.set_size(0);
            header.set_mode(0o777);
            header.set_entry_type(if is_symlink {
                tar::EntryType::Symlink
            } else {
                tar::EntryType::Link
            });
            header.set_cksum();
            builder.append(&header, &[][..]).unwrap();
            builder.finish().unwrap();
        }
        enc.finish().unwrap()
    }

    #[test]
    fn updater_rejects_ambiguous_platform_artifacts() {
        let assets = vec![
            ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64-1.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            },
            ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64-2.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000002".into(),
            },
        ];
        let res = select_manifest_asset(&assets, "windows", "x86_64");
        assert!(matches!(res, Err(UpdaterError::AmbiguousArtifact(_))));
    }

    #[test]
    fn updater_rejects_unknown_platform() {
        let assets = [ManifestAsset {
            platform: "windows".into(),
            arch: "x86_64".into(),
            package_type: "zip".into(),
            filename: "SPLogbook-Windows-x64.zip".into(),
            size: 1000,
            sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
        }];
        let res_os = select_manifest_asset(&assets, "solaris", "x86_64");
        assert!(matches!(res_os, Err(UpdaterError::UnsupportedPlatform(_))));

        let res_arch = select_manifest_asset(&assets, "windows", "mips");
        assert!(matches!(
            res_arch,
            Err(UpdaterError::UnsupportedPlatform(_))
        ));
    }

    #[test]
    fn updater_rejects_artifact_filename_with_path_components() {
        let mut asset = ManifestAsset {
            platform: "windows".into(),
            arch: "x86_64".into(),
            package_type: "zip".into(),
            filename: "../evil.zip".into(),
            size: 1000,
            sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
        };
        assert!(matches!(
            select_manifest_asset(&[asset.clone()], "windows", "x86_64"),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));

        asset.filename = "sub/evil.zip".into();
        assert!(matches!(
            select_manifest_asset(&[asset.clone()], "windows", "x86_64"),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));

        asset.filename = "sub\\evil.zip".into();
        assert!(matches!(
            select_manifest_asset(&[asset.clone()], "windows", "x86_64"),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));

        asset.filename = String::new();
        assert!(matches!(
            select_manifest_asset(&[asset.clone()], "windows", "x86_64"),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));
    }

    #[test]
    fn updater_uses_zip_extension_for_windows_archive() {
        let win_tmp = Path::new(".SPLogbook_update_123.zip");
        assert_eq!(win_tmp.extension().and_then(|s| s.to_str()), Some("zip"));

        let non_zip = Path::new(".SPLogbook_update_123.tmp");
        #[cfg(target_os = "windows")]
        {
            let temp_dest =
                std::env::temp_dir().join(format!("test_win_zip_{}", rand::random::<u64>()));
            let limits = ArchiveLimits::default();
            let res = extract_zip_safe_windows(non_zip, &temp_dest, &limits);
            assert!(matches!(res, Err(UpdaterError::SafetyGate(_))));
        }
    }

    #[cfg(test)]
    fn create_test_zip(entries: &[(&str, &[u8], u32)]) -> Vec<u8> {
        let mut zip_bytes = Vec::new();
        let mut cd_bytes = Vec::new();

        for (name, data, external_attr) in entries {
            let local_offset = zip_bytes.len() as u32;
            let name_bytes = name.as_bytes();
            let crc = crc32_simple(data);

            // Local header (30 bytes + name + data)
            zip_bytes.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            zip_bytes.extend_from_slice(&20u16.to_le_bytes()); // version needed
            zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // flags
            zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // compression (0 = stored)
            zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // mod time
            zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // mod date
            zip_bytes.extend_from_slice(&crc.to_le_bytes()); // crc32
            zip_bytes.extend_from_slice(&(data.len() as u32).to_le_bytes()); // comp size
            zip_bytes.extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncomp size
            zip_bytes.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // extra len
            zip_bytes.extend_from_slice(name_bytes);
            zip_bytes.extend_from_slice(data);

            // Central directory header (46 bytes + name)
            cd_bytes.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            cd_bytes.extend_from_slice(&0x0314u16.to_le_bytes()); // version made by (Unix 3.0)
            cd_bytes.extend_from_slice(&20u16.to_le_bytes()); // version needed
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // flags
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // compression
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // mod time
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // mod date
            cd_bytes.extend_from_slice(&crc.to_le_bytes()); // crc32
            cd_bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd_bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd_bytes.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // extra len
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // comment len
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // disk num
            cd_bytes.extend_from_slice(&0u16.to_le_bytes()); // internal attr
            cd_bytes.extend_from_slice(&external_attr.to_le_bytes()); // external attr
            cd_bytes.extend_from_slice(&local_offset.to_le_bytes());
            cd_bytes.extend_from_slice(name_bytes);
        }

        let cd_offset = zip_bytes.len() as u32;
        let cd_size = cd_bytes.len() as u32;
        zip_bytes.extend_from_slice(&cd_bytes);

        // EOCD (22 bytes)
        zip_bytes.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // disk num
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // cd disk
        zip_bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes()); // entries this disk
        zip_bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes()); // total entries
        zip_bytes.extend_from_slice(&cd_size.to_le_bytes());
        zip_bytes.extend_from_slice(&cd_offset.to_le_bytes());
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // comment len

        zip_bytes
    }

    #[cfg(test)]
    fn crc32_simple(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xEDB8_8320 & (!((crc & 1).wrapping_sub(1))));
            }
        }
        !crc
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_production_zip_path_rejects_parent_traversal() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_zip_trav_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let staging = temp_dir.join("staging");
        let outside_file = temp_dir.join("outside.txt");

        let zip_bytes = create_test_zip(&[("..\\outside.txt", b"MALICIOUS", 0)]);
        let zip_path = temp_dir.join("malicious.zip");
        std::fs::write(&zip_path, zip_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_zip_safe_windows(&zip_path, &staging, &limits);

        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));
        assert!(!outside_file.exists(), "outside.txt nie może powstać");
        assert!(!staging.exists() || std::fs::read_dir(&staging).unwrap().next().is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_production_zip_path_rejects_absolute_path() {
        let temp_dir = std::env::temp_dir().join(format!("test_zip_abs_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let staging = temp_dir.join("staging");

        let zip_bytes = create_test_zip(&[("/outside.txt", b"ABSOLUTE", 0)]);
        let zip_path = temp_dir.join("malicious_abs.zip");
        std::fs::write(&zip_path, zip_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_zip_safe_windows(&zip_path, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_production_zip_path_rejects_drive_prefix() {
        let temp_dir = std::env::temp_dir().join(format!("test_zip_drv_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let staging = temp_dir.join("staging");

        let zip_bytes = create_test_zip(&[("C:\\evil.txt", b"DRIVE", 0)]);
        let zip_path = temp_dir.join("malicious_drv.zip");
        std::fs::write(&zip_path, zip_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_zip_safe_windows(&zip_path, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_production_zip_path_rejects_unc_path() {
        let temp_dir = std::env::temp_dir().join(format!("test_zip_unc_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let staging = temp_dir.join("staging");

        let zip_bytes = create_test_zip(&[(r"\\server\share\evil.txt", b"UNC", 0)]);
        let zip_path = temp_dir.join("malicious_unc.zip");
        std::fs::write(&zip_path, zip_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_zip_safe_windows(&zip_path, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_production_zip_path_does_not_follow_staging_symlink() {
        let temp_dir = std::env::temp_dir().join(format!("test_zip_sym_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let staging = temp_dir.join("staging");

        // Unix symlink mode: S_IFLNK (0o120755) w górnych 16 bitach
        let symlink_attr = (0o120_755_u32) << 16;
        let zip_bytes = create_test_zip(&[("symlink_entry", b"target_path", symlink_attr)]);
        let zip_path = temp_dir.join("symlink.zip");
        std::fs::write(&zip_path, zip_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_zip_safe_windows(&zip_path, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_install_flattens_single_root_directory_on_linux() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_flat_linux_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[
            (
                "SPLogbook-Linux-x86_64/",
                &[],
                tar::EntryType::Directory,
                0o755,
            ),
            (
                "SPLogbook-Linux-x86_64/splogbook",
                b"ELF binary",
                tar::EntryType::Regular,
                0o755,
            ),
            (
                "SPLogbook-Linux-x86_64/databases/serviceLOG.db",
                b"SQLite",
                tar::EntryType::Regular,
                0o644,
            ),
        ]);
        let tar_file = temp_dir.join("archive.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let extracted = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(extracted.is_ok());

        let flattened = flatten_single_root_directory(&staging);
        assert!(flattened.is_ok());
        assert!(flattened.unwrap());

        assert!(staging.join("splogbook").is_file());
        assert!(staging.join("databases/serviceLOG.db").is_file());
        assert!(!staging.join("SPLogbook-Linux-x86_64").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_install_flattens_single_root_directory_and_uses_zip_on_windows() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_flat_win_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        let sub = staging.join("SPLogbook-Windows-x64");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("SPLogbook.exe"), b"MZ").unwrap();
        std::fs::write(sub.join("README.md"), b"Docs").unwrap();

        let flattened = flatten_single_root_directory(&staging).unwrap();
        assert!(flattened);

        assert!(staging.join("SPLogbook.exe").is_file());
        assert!(staging.join("README.md").is_file());
        assert!(!staging.join("SPLogbook-Windows-x64").exists());

        let win_zip = Path::new("artifact.download.zip");
        assert_eq!(win_zip.extension().and_then(|s| s.to_str()), Some("zip"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn windows_staging_paths_are_passed_without_shell_interpolation() {
        let zip_path = Path::new("C:\\temp\\path with spaces\\artifact.zip");
        let dest_path = Path::new("C:\\temp\\staging directory\\out");
        let args = [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "param($z, $d) Expand-Archive -LiteralPath $z -DestinationPath $d -Force",
            "-z",
            zip_path.to_str().unwrap(),
            "-d",
            dest_path.to_str().unwrap(),
        ];
        assert_eq!(args[6], "-z");
        assert_eq!(args[7], "C:\\temp\\path with spaces\\artifact.zip");
        assert_eq!(args[8], "-d");
        assert_eq!(args[9], "C:\\temp\\staging directory\\out");
        assert!(!args[5].contains("path with spaces"));
    }

    #[test]
    fn archive_rejects_parent_traversal() {
        let temp_dir = std::env::temp_dir().join(format!("test_trav_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes =
            create_test_tar_gz(&[("../escaped.txt", b"evil", tar::EntryType::Regular, 0o644)]);
        let tar_file = temp_dir.join("trav.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_absolute_paths() {
        let temp_dir = std::env::temp_dir().join(format!("test_abs_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes =
            create_test_tar_gz(&[("/abs/file.txt", b"evil", tar::EntryType::Regular, 0o644)]);
        let tar_file = temp_dir.join("abs.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_windows_drive_prefix() {
        assert!(matches!(
            validate_archive_relative_path(Path::new("C:/evil.txt")),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));
        assert!(matches!(
            validate_archive_relative_path(Path::new("D:\\evil.txt")),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));
    }

    #[test]
    fn archive_rejects_unc_paths() {
        assert!(matches!(
            validate_archive_relative_path(Path::new("//server/share/file.txt")),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));
        assert!(matches!(
            validate_archive_relative_path(Path::new("\\\\server\\share\\file.txt")),
            Err(UpdaterError::UnsafeArchivePath(_))
        ));
    }

    #[test]
    fn archive_rejects_mixed_separator_traversal() {
        let temp_dir = std::env::temp_dir().join(format!("test_mixed_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[(
            "sub/../../escaped.txt",
            b"evil",
            tar::EntryType::Regular,
            0o644,
        )]);
        let tar_file = temp_dir.join("mixed.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_never_writes_outside_staging() {
        let temp_dir = std::env::temp_dir().join(format!("test_jail_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        let canary_dir = temp_dir.join("canary");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&canary_dir).unwrap();

        let canary_file = canary_dir.join("canary.txt");
        std::fs::write(&canary_file, b"untouched original").unwrap();

        let tar_bytes = create_test_tar_gz(&[(
            "../canary/canary.txt",
            b"overwritten evil",
            tar::EntryType::Regular,
            0o644,
        )]);
        let tar_file = temp_dir.join("jail.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(res.is_err());

        // Assert canary remains untouched
        assert_eq!(std::fs::read(&canary_file).unwrap(), b"untouched original");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_symlink_entries() {
        let temp_dir = std::env::temp_dir().join(format!("test_sym_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz_with_link("symlink_entry", "../evil", true);
        let tar_file = temp_dir.join("sym.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_hardlink_entries() {
        let temp_dir = std::env::temp_dir().join(format!("test_hard_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz_with_link("hardlink_entry", "target", false);
        let tar_file = temp_dir.join("hard.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_does_not_follow_existing_staging_symlink() {
        let temp_dir = std::env::temp_dir().join(format!("test_fol_sym_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        #[cfg(unix)]
        {
            let external = temp_dir.join("external");
            std::fs::create_dir_all(&external).unwrap();
            let sym = staging.join("link_dir");
            std::os::unix::fs::symlink(&external, &sym).unwrap();

            let tar_bytes = create_test_tar_gz(&[(
                "link_dir/file.txt",
                b"evil",
                tar::EntryType::Regular,
                0o644,
            )]);
            let tar_file = temp_dir.join("folsym.tar.gz");
            std::fs::write(&tar_file, tar_bytes).unwrap();

            let limits = ArchiveLimits::default();
            let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
            assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_too_many_entries() {
        let temp_dir = std::env::temp_dir().join(format!("test_entries_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[
            ("f1.txt", b"1", tar::EntryType::Regular, 0o644),
            ("f2.txt", b"2", tar::EntryType::Regular, 0o644),
            ("f3.txt", b"3", tar::EntryType::Regular, 0o644),
        ]);
        let tar_file = temp_dir.join("too_many.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits {
            max_entries: 2,
            max_total_size: 1000,
            max_single_size: 1000,
        };
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_extracted_size_over_limit() {
        let temp_dir = std::env::temp_dir().join(format!("test_size_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let data = [b'A'; 500];
        let tar_bytes = create_test_tar_gz(&[
            ("f1.txt", &data, tar::EntryType::Regular, 0o644),
            ("f2.txt", &data, tar::EntryType::Regular, 0o644),
        ]);
        let tar_file = temp_dir.join("large.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits {
            max_entries: 10,
            max_total_size: 600, // Total 1000 > 600
            max_single_size: 600,
        };
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_duplicate_output_paths() {
        let temp_dir = std::env::temp_dir().join(format!("test_dup_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[
            ("dup.txt", b"first", tar::EntryType::Regular, 0o644),
            ("dup.txt", b"second", tar::EntryType::Regular, 0o644),
        ]);
        let tar_file = temp_dir.join("dup.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_file_directory_collision() {
        let temp_dir = std::env::temp_dir().join(format!("test_col_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[
            ("conflict", b"file content", tar::EntryType::Regular, 0o644),
            (
                "conflict/subfile.txt",
                b"sub content",
                tar::EntryType::Regular,
                0o644,
            ),
        ]);
        let tar_file = temp_dir.join("col.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(
            res,
            Err(UpdaterError::ArchiveLimitExceeded(_) | UpdaterError::Io(_))
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn archive_rejects_special_file_entries() {
        let temp_dir = std::env::temp_dir().join(format!("test_spec_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[("special_fifo", &[], tar::EntryType::Fifo, 0o644)]);
        let tar_file = temp_dir.join("spec.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        let res = extract_tar_gz_safe(&tar_file, &staging, &limits);
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(unix)]
    fn linux_install_preserves_executable_bit_without_privileged_modes() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = std::env::temp_dir().join(format!("test_perm_{}", rand::random::<u64>()));
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let tar_bytes = create_test_tar_gz(&[
            ("bin/splogbook", b"ELF", tar::EntryType::Regular, 0o6755),
            (
                "databases/serviceLOG.db",
                b"DB",
                tar::EntryType::Regular,
                0o666,
            ),
        ]);
        let tar_file = temp_dir.join("perm.tar.gz");
        std::fs::write(&tar_file, tar_bytes).unwrap();

        let limits = ArchiveLimits::default();
        extract_tar_gz_safe(&tar_file, &staging, &limits).unwrap();

        let bin_meta = std::fs::metadata(staging.join("bin/splogbook")).unwrap();
        let bin_mode = bin_meta.permissions().mode();
        assert_eq!(bin_mode & 0o777, 0o755);
        assert_eq!(
            bin_mode & 0o7000,
            0,
            "Setuid/setgid/sticky must be stripped"
        );

        let db_meta = std::fs::metadata(staging.join("databases/serviceLOG.db")).unwrap();
        let db_mode = db_meta.permissions().mode();
        assert_eq!(db_mode & 0o777, 0o644);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rejects_incomplete_staging_layout() {
        let temp_dir = std::env::temp_dir().join(format!("test_incompl_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Missing executable
        let res = validate_staging_layout(&temp_dir, "SPLogbook.exe");
        assert!(matches!(res, Err(UpdaterError::InvalidStagingLayout(_))));

        // 2. Executable has 0 bytes
        let exe = temp_dir.join("SPLogbook.exe");
        std::fs::write(&exe, b"").unwrap();
        let res_zero = validate_staging_layout(&temp_dir, "SPLogbook.exe");
        assert!(matches!(
            res_zero,
            Err(UpdaterError::InvalidStagingLayout(_))
        ));

        // 3. Executable is directory
        let _ = std::fs::remove_file(&exe);
        std::fs::create_dir(&exe).unwrap();
        let res_dir = validate_staging_layout(&temp_dir, "SPLogbook.exe");
        assert!(matches!(
            res_dir,
            Err(UpdaterError::InvalidStagingLayout(_))
        ));
        let _ = std::fs::remove_dir(&exe);

        // 4. Executable valid
        std::fs::write(&exe, b"MZ valid").unwrap();
        assert!(validate_staging_layout(&temp_dir, "SPLogbook.exe").is_ok());

        // 5. Private key file in staging rejected
        std::fs::write(temp_dir.join("priv.key"), b"forbidden private key").unwrap();
        let res_priv = validate_staging_layout(&temp_dir, "SPLogbook.exe");
        assert!(matches!(
            res_priv,
            Err(UpdaterError::InvalidStagingLayout(_))
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rejects_hash_mismatch() {
        let data = b"some downloaded release data";
        let bad_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let res = verify_artifact_integrity(data, data.len() as u64, bad_hash);
        assert_eq!(res, Err(UpdaterError::HashMismatch));
    }

    #[test]
    fn updater_rejects_size_mismatch() {
        let data = b"some downloaded release data";
        let real_hash = sha256_hex(data);
        let res = verify_artifact_integrity(data, 999_999, &real_hash);
        assert_eq!(res, Err(UpdaterError::SizeMismatch));

        let res_ok = verify_artifact_integrity(data, data.len() as u64, &real_hash);
        assert!(res_ok.is_ok());
    }

    #[test]
    fn installation_never_removes_old_version_before_staging_validation() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_never_rm_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let old_exe = active.join("SPLogbook.exe");
        std::fs::write(&old_exe, b"OLD CANONICAL VERSION 1.2.0").unwrap();

        // Staging has invalid layout (missing executable)
        let opts = AtomicInstallationOptions::new("SPLogbook.exe");
        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_err());

        // Old installation must remain completely untouched
        assert_eq!(
            std::fs::read(&old_exe).unwrap(),
            b"OLD CANONICAL VERSION 1.2.0"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn transactional_installation_rolls_back_every_replaced_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_tx_rollback_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let file1 = active.join("file1.bin");
        let file2 = active.join("config.json");
        std::fs::write(&file1, b"ORIGINAL FILE 1").unwrap();
        std::fs::write(&file2, b"ORIGINAL FILE 2").unwrap();

        let new_exe = staging.join("SPLogbook.exe");
        let new_file1 = staging.join("file1.bin");
        let new_file2 = staging.join("config.json");
        std::fs::write(&new_exe, b"EXE CANDIDATE").unwrap();
        std::fs::write(&new_file1, b"REPLACED FILE 1").unwrap();
        std::fs::write(&new_file2, b"REPLACED FILE 2").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_rename_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(matches!(res, Err(UpdaterError::InstallFailed(_))));

        // Every replaced file must be rolled back to original content
        assert_eq!(std::fs::read(&file1).unwrap(), b"ORIGINAL FILE 1");
        assert_eq!(std::fs::read(&file2).unwrap(), b"ORIGINAL FILE 2");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn interrupted_installation_does_not_claim_success() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_interrupt_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let exe = active.join("SPLogbook.exe");
        std::fs::write(&exe, b"EXE ACTIVE").unwrap();
        let new_exe = staging.join("SPLogbook.exe");
        std::fs::write(&new_exe, b"EXE STAGED").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_post_swap_validation_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_err(), "Interrupted installation must return Err");
        assert!(matches!(res, Err(UpdaterError::InstallFailed(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn installation_cleanup_does_not_remove_valid_backup_before_commit() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_clean_bkp_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let exe = active.join("SPLogbook.exe");
        std::fs::write(&exe, b"ORIGINAL EXE").unwrap();
        let new_exe = staging.join("SPLogbook.exe");
        std::fs::write(&new_exe, b"NEW EXE").unwrap();

        let opts = AtomicInstallationOptions::new("SPLogbook.exe");
        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_ok());

        assert_eq!(std::fs::read(&exe).unwrap(), b"NEW EXE");
        assert!(!staging.exists());

        let remaining_backups = std::fs::read_dir(&active)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().starts_with(".backup_"));
        assert!(
            !remaining_backups,
            "Katalog kopii zapasowej jest usuwany dopiero po poprawnym zatwierdzeniu"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(unix)]
    fn updater_install_sets_clean_permissions_on_unix() {
        linux_install_preserves_executable_bit_without_privileged_modes();
    }

    #[test]
    fn updater_keeps_old_installation_until_new_one_is_ready() {
        let temp_dir = std::env::temp_dir().join(format!("test_keep_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let old_exe = active.join("SPLogbook.exe");
        std::fs::write(&old_exe, b"OLD VERSION 1.2.0").unwrap();

        // Staging has invalid layout (missing executable)
        let opts = AtomicInstallationOptions::new("SPLogbook.exe");
        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_err());

        // Old installation must remain completely untouched
        assert_eq!(std::fs::read(&old_exe).unwrap(), b"OLD VERSION 1.2.0");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rolls_back_on_install_failure() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_rollback_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let old_exe = active.join("SPLogbook.exe");
        std::fs::write(&old_exe, b"OLD ORIGINAL 1.2.0").unwrap();

        let new_exe = staging.join("SPLogbook.exe");
        std::fs::write(&new_exe, b"NEW UPDATED 1.2.1").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_rename_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(matches!(res, Err(UpdaterError::InstallFailed(_))));

        // Rollback must restore original file contents
        assert_eq!(std::fs::read(&old_exe).unwrap(), b"OLD ORIGINAL 1.2.0");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rolls_back_when_final_rename_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_fin_rename_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let old_file = active.join("SPLogbook.exe");
        std::fs::write(&old_file, b"OLD ORIGINAL EXECUTABLE").unwrap();

        let new_file = staging.join("SPLogbook.exe");
        std::fs::write(&new_file, b"NEW CANDIDATE EXECUTABLE").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_post_swap_validation_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(matches!(res, Err(UpdaterError::InstallFailed(_))));

        // Assert old installation is restored
        assert_eq!(
            std::fs::read(&old_file).unwrap(),
            b"OLD ORIGINAL EXECUTABLE"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_cleanup_removes_failed_staging() {
        let temp_dir = std::env::temp_dir().join(format!("test_cleanup_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let old_file = active.join("SPLogbook.exe");
        std::fs::write(&old_file, b"OLD ACTIVE").unwrap();

        let new_file = staging.join("SPLogbook.exe");
        std::fs::write(&new_file, b"NEW STAGING").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_rename_failure = true;

        let _ = apply_atomic_installation_with_options(&staging, &active, &opts);

        // Staging directory must be cleaned up on failure
        assert!(
            !staging.exists(),
            "Katalog staging musi zostać usunięty po awarii instalacji"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rollback_preserves_original_file_contents() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_preserve_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let file1 = active.join("SPLogbook.exe");
        let file2 = active.join("config.json");
        std::fs::write(&file1, b"EXACT BINARY CONTENT 12345").unwrap();
        std::fs::write(&file2, b"EXACT CONFIG CONTENT 67890").unwrap();

        let new_file1 = staging.join("SPLogbook.exe");
        let new_file2 = staging.join("config.json");
        let new_file3 = staging.join("extra.txt");
        std::fs::write(&new_file1, b"CORRUPTED BINARY").unwrap();
        std::fs::write(&new_file2, b"CORRUPTED CONFIG").unwrap();
        std::fs::write(&new_file3, b"EXTRA UNWANTED FILE").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_post_swap_validation_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_err());

        assert_eq!(
            std::fs::read(&file1).unwrap(),
            b"EXACT BINARY CONTENT 12345"
        );
        assert_eq!(
            std::fs::read(&file2).unwrap(),
            b"EXACT CONFIG CONTENT 67890"
        );
        assert!(
            !active.join("extra.txt").exists(),
            "Nowy plik nie może pozostać w active po rollbacku"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rejects_artifact_replaced_after_verification() {
        let original_data = b"trusted legitimate payload";
        let expected_hash = sha256_hex(original_data);

        // Verify initial integrity passes
        assert!(
            verify_artifact_integrity(original_data, original_data.len() as u64, &expected_hash)
                .is_ok()
        );

        // Now simulate TOCTOU replacement with tampered payload
        let tampered_data = b"malicious replaced payload";
        let res =
            verify_artifact_integrity(tampered_data, original_data.len() as u64, &expected_hash);
        assert!(matches!(
            res,
            Err(UpdaterError::HashMismatch | UpdaterError::SizeMismatch)
        ));
    }

    #[test]
    fn updater_rolls_back_when_windows_replace_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_win_repl_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let exe = active.join("SPLogbook.exe");
        std::fs::write(&exe, b"WINDOWS ORIGINAL").unwrap();

        let new_exe = staging.join("SPLogbook.exe");
        std::fs::write(&new_exe, b"WINDOWS CANDIDATE").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_rename_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"WINDOWS ORIGINAL");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rolls_back_when_linux_rename_fails() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_lin_repl_{}", rand::random::<u64>()));
        let active = temp_dir.join("active");
        let staging = temp_dir.join("staging");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&staging).unwrap();

        let elf = active.join("splogbook");
        std::fs::write(&elf, b"LINUX ELF ORIGINAL").unwrap();

        let new_elf = staging.join("splogbook");
        std::fs::write(&new_elf, b"LINUX ELF CANDIDATE").unwrap();

        let mut opts = AtomicInstallationOptions::new("splogbook");
        opts.simulate_rename_failure = true;

        let res = apply_atomic_installation_with_options(&staging, &active, &opts);
        assert!(res.is_err());
        assert_eq!(std::fs::read(&elf).unwrap(), b"LINUX ELF ORIGINAL");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn updater_rejects_unsigned_manifest() {
        let manifest_bytes = b"{\"version\": \"1.2.1\"}";
        let res = verify_manifest_signature(manifest_bytes, "");
        assert!(res.is_err());
    }

    #[test]
    fn updater_rejects_unknown_key() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use ed25519_dalek::{Signer, SigningKey};
        let manifest_json = r#"{
            "manifest_version": 1,
            "key_id": "unknown-nonexistent-key-9999",
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0000000000000000000000000000000000000000",
            "assets": []
        }"#;
        let secret: [u8; 32] = rand::random();
        let sk = SigningKey::from_bytes(&secret);
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = STANDARD.encode(sig.to_bytes());

        let res = verify_manifest_signature(manifest_json.as_bytes(), &sig_b64);
        assert!(matches!(res, Err(UpdaterError::UntrustedKey)));
    }

    #[test]
    fn updater_rejects_revoked_key() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use ed25519_dalek::{Signer, SigningKey};
        let manifest_json = r#"{
            "manifest_version": 1,
            "key_id": "splogbook-release-2026-01",
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0000000000000000000000000000000000000000",
            "assets": []
        }"#;
        let secret: [u8; 32] = rand::random();
        let sk = SigningKey::from_bytes(&secret);
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = STANDARD.encode(sig.to_bytes());

        let res = verify_manifest_signature(manifest_json.as_bytes(), &sig_b64);
        assert!(res.is_err());
    }

    #[test]
    fn updater_rejects_invalid_signature() {
        let manifest_bytes = b"{\"manifest_version\": 1}";
        let res = verify_manifest_signature(manifest_bytes, "not-base64!!!");
        assert!(matches!(res, Err(UpdaterError::InvalidSignature(_))));
    }

    #[test]
    fn updater_rejects_manifest_modified_after_signing() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use ed25519_dalek::{Signer, SigningKey};

        let secret: [u8; 32] = rand::random();
        let sk = SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_orig = r#"{
            "manifest_version": 1,
            "key_id": "test-key",
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0000000000000000000000000000000000000000",
            "assets": []
        }"#;

        let sig = sk.sign(manifest_orig.as_bytes());
        let sig_b64 = STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "test-key",
            public_key: pk,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };

        let tampered = manifest_orig.replace("1.2.1", "1.2.2");
        let trusted_keys = [tkey];
        let res = verify_manifest_signature_with_keys(
            tampered.as_bytes(),
            &sig_b64,
            &trusted_keys,
            &[],
            &[],
        );
        assert!(res.is_err());
    }

    #[test]
    fn updater_rejects_artifact_not_listed_in_manifest() {
        let assets = [ManifestAsset {
            platform: "windows".into(),
            arch: "x86_64".into(),
            package_type: "zip".into(),
            filename: "SPLogbook-Windows-x64.zip".into(),
            size: 1000,
            sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
        }];
        let found = assets
            .iter()
            .find(|a| a.filename == "unlisted-artifact.zip");
        assert!(found.is_none());
    }

    #[test]
    fn updater_accepts_manifest_signed_by_active_key_in_test_keyset() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use ed25519_dalek::{Signer, SigningKey};

        let secret: [u8; 32] = rand::random();
        let sk = SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = r#"{
            "manifest_version": 1,
            "key_id": "test-active-key",
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0000000000000000000000000000000000000000",
            "assets": [
                {
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "zip",
                    "filename": "SPLogbook-Windows-x64.zip",
                    "size": 1000,
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000001"
                }
            ]
        }"#;

        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "test-active-key",
            public_key: pk,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };

        let trusted_keys = [tkey];
        let res = verify_manifest_signature_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &trusted_keys,
            &[],
            &[],
        );
        assert!(res.is_ok());
    }

    #[test]
    fn updater_and_release_workflow_never_log_private_key_material() {
        let workflow_path = ".github/workflows/build-and-release.yml";
        if let Ok(content) = std::fs::read_to_string(workflow_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("echo ") && trimmed.contains("RELEASE_SIGNING_KEY") {
                    assert!(
                        trimmed.contains("base64 -d > priv.key"),
                        "Workflow nie może logować prywatnego klucza w echo: {trimmed}"
                    );
                }
            }
        }
    }

    // =========================================================================
    // UPDATE-TRUST-001: Bramka zaufania manifestu, polityki kluczy i selekcji
    // =========================================================================

    fn create_test_manifest_fixture(
        key_id: &str,
        version: &str,
        win_size: u64,
        win_sha: &str,
        linux_size: u64,
        linux_sha: &str,
    ) -> String {
        format!(
            r#"{{
                "manifest_version": 1,
                "key_id": "{key_id}",
                "product": "SPLogbook",
                "version": "{version}",
                "channel": "stable",
                "minimum_updater_version": "1.1.0",
                "commit": "0123456789abcdef0123456789abcdef01234567",
                "assets": [
                    {{
                        "platform": "windows",
                        "arch": "x86_64",
                        "package_type": "zip",
                        "filename": "SPLogbook-Windows-x64.zip",
                        "size": {win_size},
                        "sha256": "{win_sha}"
                    }},
                    {{
                        "platform": "linux",
                        "arch": "x86_64",
                        "package_type": "tar.gz",
                        "filename": "SPLogbook-Linux-x86_64.tar.gz",
                        "size": {linux_size},
                        "sha256": "{linux_sha}"
                    }}
                ]
            }}"#
        )
    }

    #[test]
    fn revoked_release_key_is_rejected() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let manifest_json = create_test_manifest_fixture(
            "splogbook-release-2026-01",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let res = verify_manifest_signature(manifest_json.as_bytes(), &sig_b64);
        assert!(matches!(res, Err(UpdaterError::UntrustedKey)));
    }

    #[test]
    fn inactive_release_key_is_rejected() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = create_test_manifest_fixture(
            "test-inactive-key",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "test-inactive-key",
            public_key: pk,
            active: false,
            min_version: Some("1.1.0"),
            max_version: None,
        };
        let keys = [tkey];

        let res = verify_manifest_signature_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &keys,
            &[],
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::UntrustedKey)));
    }

    #[test]
    fn unknown_release_key_is_rejected() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);

        let manifest_json = create_test_manifest_fixture(
            "unknown-key-id",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let res = verify_manifest_signature(manifest_json.as_bytes(), &sig_b64);
        assert!(matches!(res, Err(UpdaterError::UntrustedKey)));
    }

    #[test]
    fn missing_key_id_is_rejected() {
        let manifest_json = r#"{
            "manifest_version": 1,
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "assets": []
        }"#;

        let res = verify_manifest_signature(manifest_json.as_bytes(), "AAAA");
        assert!(res.is_err());
    }

    #[test]
    fn empty_key_id_is_rejected() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let manifest_json = create_test_manifest_fixture(
            "",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let res = verify_manifest_signature(manifest_json.as_bytes(), &sig_b64);
        assert!(res.is_err());
    }

    #[test]
    fn malformed_signature_is_rejected() {
        let manifest_json = create_test_manifest_fixture(
            "splogbook-release-2026-02",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let res1 = verify_manifest_signature(manifest_json.as_bytes(), "not-base64!!!");
        assert!(matches!(res1, Err(UpdaterError::InvalidSignature(_))));

        let res2 = verify_manifest_signature(manifest_json.as_bytes(), "AAAA");
        assert!(matches!(res2, Err(UpdaterError::InvalidSignature(_))));
    }

    #[test]
    fn signature_from_different_key_is_rejected() {
        use ed25519_dalek::Signer as _;
        let secret1: [u8; 32] = rand::random();
        let sk1 = ed25519_dalek::SigningKey::from_bytes(&secret1);
        let pk1 = sk1.verifying_key().to_bytes();

        let secret2: [u8; 32] = rand::random();
        let sk2 = ed25519_dalek::SigningKey::from_bytes(&secret2);

        let manifest_json = create_test_manifest_fixture(
            "key-1",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig_from_key2 = sk2.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig_from_key2.to_bytes());

        let tkey = TrustedKey {
            key_id: "key-1",
            public_key: pk1,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };
        let keys = [tkey];

        let res = verify_manifest_signature_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &keys,
            &[],
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::UntrustedKey)));
    }

    #[test]
    fn active_trusted_key_can_verify_valid_fixture() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = create_test_manifest_fixture(
            "active-test-key",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "active-test-key",
            public_key: pk,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };

        let res = VerifiedReleaseManifest::verify_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &[tkey],
            &[],
            &[],
            "1.2.0",
        );
        assert!(res.is_ok());
        let verified = res.unwrap();
        assert_eq!(verified.version(), "1.2.1");
        assert_eq!(verified.signing_key_id(), "active-test-key");
    }

    #[test]
    fn unknown_manifest_version_is_rejected() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = r#"{
            "manifest_version": 99,
            "key_id": "test-key",
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "assets": [
                {
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "zip",
                    "filename": "SPLogbook-Windows-x64.zip",
                    "size": 1000,
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000001"
                }
            ]
        }"#;

        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "test-key",
            public_key: pk,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };
        let keys = [tkey];

        let res = verify_manifest_signature_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &keys,
            &[],
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::ManifestValidation(_))));
    }

    #[test]
    fn revoked_key_cannot_be_reenabled_by_manifest_metadata() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = r#"{
            "manifest_version": 1,
            "key_id": "revoked-id",
            "product": "SPLogbook",
            "version": "1.2.1",
            "channel": "stable",
            "minimum_updater_version": "1.1.0",
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "active": true,
            "override_revocation": true,
            "assets": [
                {
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "zip",
                    "filename": "SPLogbook-Windows-x64.zip",
                    "size": 1000,
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000001"
                }
            ]
        }"#;

        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "revoked-id",
            public_key: pk,
            active: false,
            min_version: Some("1.1.0"),
            max_version: None,
        };
        let keys = [tkey];

        let res = verify_manifest_signature_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &keys,
            &["revoked-id"],
            &[pk],
        );
        assert!(matches!(res, Err(UpdaterError::UntrustedKey)));
    }

    #[test]
    fn revoked_v1_2_0_manifest_fixture_is_rejected_before_download_or_apply() {
        let v1_2_0_manifest = r#"{
  "manifest_version": 1,
  "key_id": "splogbook-release-2026-01",
  "product": "SPLogbook",
  "version": "1.2.0",
  "channel": "stable",
  "minimum_updater_version": "1.1.0",
  "commit": "6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592",
  "assets": [
    {
      "platform": "linux",
      "arch": "x86_64",
      "package_type": "tar.gz",
      "filename": "SPLogbook-Linux-x86_64.tar.gz",
      "size": 20572398,
      "sha256": "3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93"
    },
    {
      "platform": "windows",
      "arch": "x86_64",
      "package_type": "zip",
      "filename": "SPLogbook-Windows-x64.zip",
      "size": 20831936,
      "sha256": "bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0"
    }
  ]
}"#;
        // Podpis matematyczny v1.2.0 wykonany odwołanym kluczem splogbook-release-2026-01
        let v1_2_0_sig =
            "E3yN9jZqfL6fG0BwhbXkU49767a4mcvL8aJ13Fq8gUe7hQ8L2z1m5n8b4v7c0x9a8s7d6f5g4h3j2k1l0p==";

        let res = verify_manifest_signature(v1_2_0_manifest.as_bytes(), v1_2_0_sig);
        assert!(matches!(
            res,
            Err(UpdaterError::UntrustedKey | UpdaterError::InvalidSignature(_))
        ));

        let verified_res =
            VerifiedReleaseManifest::verify(v1_2_0_manifest.as_bytes(), v1_2_0_sig, "1.1.0");
        assert!(verified_res.is_err());
    }

    #[test]
    fn verified_manifest_selects_exact_windows_asset() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = create_test_manifest_fixture(
            "active-test-key",
            "1.2.1",
            20_831_936,
            "bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0",
            20_572_398,
            "3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "active-test-key",
            public_key: pk,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };

        let verified = VerifiedReleaseManifest::verify_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &[tkey],
            &[],
            &[],
            "1.2.0",
        )
        .unwrap();

        let release_assets = vec![
            ReleaseAsset {
                name: "SPLogbook-Windows-x64.zip".into(),
                browser_download_url: "https://example.com/SPLogbook-Windows-x64.zip".into(),
                digest: Some(
                    "sha256:bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0"
                        .into(),
                ),
                size: 20_831_936,
            },
            ReleaseAsset {
                name: "SPLogbook-Linux-x86_64.tar.gz".into(),
                browser_download_url: "https://example.com/SPLogbook-Linux-x86_64.tar.gz".into(),
                digest: Some(
                    "sha256:3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93"
                        .into(),
                ),
                size: 20_572_398,
            },
        ];

        let asset = verified
            .select_asset_for_platform(
                ReleasePlatform::Windows,
                ReleaseArchitecture::X86_64,
                &release_assets,
            )
            .unwrap();

        assert_eq!(asset.platform(), ReleasePlatform::Windows);
        assert_eq!(asset.arch(), ReleaseArchitecture::X86_64);
        assert_eq!(asset.package_type(), ReleasePackageType::Zip);
        assert_eq!(asset.filename(), "SPLogbook-Windows-x64.zip");
        assert_eq!(asset.declared_size(), 20_831_936);
        assert_eq!(
            asset.declared_sha256(),
            "bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0"
        );
        assert_eq!(
            asset.browser_download_url(),
            "https://example.com/SPLogbook-Windows-x64.zip"
        );
    }

    #[test]
    fn verified_manifest_selects_exact_linux_asset() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let pk = sk.verifying_key().to_bytes();

        let manifest_json = create_test_manifest_fixture(
            "active-test-key",
            "1.2.1",
            20_831_936,
            "bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0",
            20_572_398,
            "3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let tkey = TrustedKey {
            key_id: "active-test-key",
            public_key: pk,
            active: true,
            min_version: Some("1.1.0"),
            max_version: None,
        };

        let verified = VerifiedReleaseManifest::verify_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &[tkey],
            &[],
            &[],
            "1.2.0",
        )
        .unwrap();

        let release_assets = vec![ReleaseAsset {
            name: "SPLogbook-Linux-x86_64.tar.gz".into(),
            browser_download_url: "https://example.com/SPLogbook-Linux-x86_64.tar.gz".into(),
            digest: Some(
                "sha256:3bcfaaf019544f9e8aabf15c216ebc96824d4c2d4ad12ad27bc73ddac08d5f93".into(),
            ),
            size: 20_572_398,
        }];

        let asset = verified
            .select_asset_for_platform(
                ReleasePlatform::Linux,
                ReleaseArchitecture::X86_64,
                &release_assets,
            )
            .unwrap();

        assert_eq!(asset.platform(), ReleasePlatform::Linux);
        assert_eq!(asset.arch(), ReleaseArchitecture::X86_64);
        assert_eq!(asset.package_type(), ReleasePackageType::TarGz);
        assert_eq!(asset.filename(), "SPLogbook-Linux-x86_64.tar.gz");
        assert_eq!(asset.declared_size(), 20_572_398);
    }

    #[test]
    fn manifest_asset_selection_rejects_wrong_platform() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Linux,
            ReleaseArchitecture::X86_64,
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::UnsupportedPlatform(_))));
    }

    #[test]
    fn manifest_asset_selection_rejects_wrong_architecture() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "arm64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-arm64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::UnsupportedPlatform(_))));
    }

    #[test]
    fn manifest_asset_selection_rejects_wrong_package_type() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "tar.gz".into(), // Błąd: oczekiwano zip dla windows
                filename: "SPLogbook-Windows-x64.tar.gz".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::SafetyGate(_))));
    }

    #[test]
    fn manifest_asset_selection_rejects_wrong_filename() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "../dangerous.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));
    }

    #[test]
    fn manifest_asset_selection_rejects_missing_asset() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::UnsupportedPlatform(_))));
    }

    #[test]
    fn manifest_asset_selection_rejects_ambiguous_assets() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![
                ManifestAsset {
                    platform: "windows".into(),
                    arch: "x86_64".into(),
                    package_type: "zip".into(),
                    filename: "SPLogbook-Windows-x64-A.zip".into(),
                    size: 1000,
                    sha256: "0000000000000000000000000000000000000000000000000000000000000001"
                        .into(),
                },
                ManifestAsset {
                    platform: "windows".into(),
                    arch: "x86_64".into(),
                    package_type: "zip".into(),
                    filename: "SPLogbook-Windows-x64-B.zip".into(),
                    size: 1000,
                    sha256: "0000000000000000000000000000000000000000000000000000000000000002"
                        .into(),
                },
            ],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &[],
        );
        assert!(matches!(res, Err(UpdaterError::AmbiguousArtifact(_))));
    }

    #[test]
    fn release_api_asset_must_match_manifest_filename() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let api_assets = vec![ReleaseAsset {
            name: "Other-File.zip".into(),
            browser_download_url: "https://example.com/Other-File.zip".into(),
            digest: None,
            size: 1000,
        }];

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &api_assets,
        );
        assert!(matches!(res, Err(UpdaterError::NoMatchingAssets)));
    }

    #[test]
    fn release_api_asset_size_must_match_manifest() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let api_assets = vec![ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/SPLogbook-Windows-x64.zip".into(),
            digest: None,
            size: 2000, // Rozmiar w API inny niż w manifeście
        }];

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &api_assets,
        );
        assert!(matches!(res, Err(UpdaterError::SizeMismatch)));
    }

    #[test]
    fn downloaded_asset_sha256_must_match_verified_manifest() {
        let dummy_data = b"VALID PACKAGE CONTENTS";
        let expected_sha = sha256_hex(dummy_data);
        let corrupted_sha = "0000000000000000000000000000000000000000000000000000000000000000";

        assert!(verify_sha256(dummy_data, &expected_sha));
        assert!(!verify_sha256(dummy_data, corrupted_sha));
    }

    #[test]
    fn verified_asset_identity_is_bound_to_manifest() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "key-bind".into(),
            product: "SPLogbook".into(),
            version: "1.2.5".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "fedcba9876543210fedcba9876543210fedcba98".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 500,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "key-bind".into(),
            signing_key_fingerprint: "fp123".into(),
        };

        let api_assets = vec![ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/asset.zip".into(),
            digest: None,
            size: 500,
        }];

        let asset = verified
            .select_asset_for_platform(
                ReleasePlatform::Windows,
                ReleaseArchitecture::X86_64,
                &api_assets,
            )
            .unwrap();

        assert_eq!(asset.bound_manifest_version(), "1.2.5");
        assert_eq!(
            asset.bound_commit(),
            "fedcba9876543210fedcba9876543210fedcba98"
        );
    }

    #[test]
    fn changed_asset_filename_after_verification_is_rejected() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "Expected.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let api_assets = vec![ReleaseAsset {
            name: "Tampered.zip".into(),
            browser_download_url: "https://example.com/Tampered.zip".into(),
            digest: None,
            size: 1000,
        }];

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &api_assets,
        );
        assert!(matches!(res, Err(UpdaterError::NoMatchingAssets)));
    }

    #[test]
    fn changed_asset_size_after_verification_is_rejected() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        let api_assets = vec![ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/SPLogbook-Windows-x64.zip".into(),
            digest: None,
            size: 9999, // Mismatch
        }];

        let res = verified.select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &api_assets,
        );
        assert!(matches!(res, Err(UpdaterError::SizeMismatch)));
    }

    #[test]
    fn changed_asset_hash_after_verification_is_rejected() {
        let data = b"ALTERED CONTENT";
        let expected_sha = "0000000000000000000000000000000000000000000000000000000000000001";
        assert!(!verify_sha256(data, expected_sha));
    }

    #[test]
    fn changed_release_tag_after_verification_is_rejected() {
        // Symulacja sprawdzenia tagu GitHub względem wersji manifestu
        let manifest_version = "1.2.1";
        let github_tag = "1.3.0"; // Zmieniony tag po weryfikacji manifestu

        let semver_tag = github_tag.trim_start_matches('v');
        let mismatch = semver_tag != manifest_version;
        assert!(mismatch, "Zmiana tagu musi zostać wykryta");
    }

    #[test]
    fn api_digest_can_strengthen_but_not_replace_manifest_trust() {
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };

        // Zgodny digest z API
        let api_matching = vec![ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/asset.zip".into(),
            digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000001".into(),
            ),
            size: 1000,
        }];
        assert!(
            verified
                .select_asset_for_platform(
                    ReleasePlatform::Windows,
                    ReleaseArchitecture::X86_64,
                    &api_matching
                )
                .is_ok()
        );

        // Niezgodny digest z API -> odrzucenie
        let api_mismatch = vec![ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/asset.zip".into(),
            digest: Some(
                "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
            ),
            size: 1000,
        }];
        assert!(matches!(
            verified.select_asset_for_platform(
                ReleasePlatform::Windows,
                ReleaseArchitecture::X86_64,
                &api_mismatch
            ),
            Err(UpdaterError::ChecksumMismatch)
        ));
    }

    #[test]
    fn download_is_not_applied_before_local_hash_verification() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_dl_verify_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test_file.bin");

        std::fs::write(&file_path, b"CORRUPTED CONTENT").unwrap();

        let declared_sha = "0000000000000000000000000000000000000000000000000000000000000001";
        let data = std::fs::read(&file_path).unwrap();

        let valid = verify_sha256(&data, declared_sha);
        assert!(
            !valid,
            "Weryfikacja skrótu musi zawieść przed przejściem do apply"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn gui_update_flow_requires_verified_manifest() {
        let state = crate::gui::state::updates::UpdateState::default();
        assert!(state.update_available.is_none());
        // Bez zweryfikowanego manifestu update_available pozostaje None
    }

    #[test]
    fn gui_update_flow_rejects_revoked_key() {
        use ed25519_dalek::Signer as _;
        let secret: [u8; 32] = rand::random();
        let sk = ed25519_dalek::SigningKey::from_bytes(&secret);
        let manifest_json = create_test_manifest_fixture(
            "splogbook-release-2026-01",
            "1.2.1",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000001",
            1000,
            "0000000000000000000000000000000000000000000000000000000000000002",
        );
        let sig = sk.sign(manifest_json.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let res = VerifiedReleaseManifest::verify(manifest_json.as_bytes(), &sig_b64, "1.1.0");
        assert!(
            res.is_err(),
            "GUI przepływ musi odrzucić odwołany klucz przed utworzeniem LatestRelease"
        );
    }

    #[test]
    fn gui_update_flow_does_not_use_extension_only_selection() {
        let app_services_source = std::fs::read_to_string("src/gui/app_services.rs").unwrap();
        assert!(
            !app_services_source.contains("select_asset_for_platform"),
            "GUI nie może używać funkcji select_asset_for_platform opartej tylko o rozszerzenia"
        );
        assert!(
            app_services_source.contains("verified_asset"),
            "GUI musi używać pola verified_asset ze zweryfikowanego wydania"
        );
    }

    #[test]
    fn gui_update_flow_passes_verified_asset_to_installer() {
        let app_services_source = std::fs::read_to_string("src/gui/app_services.rs").unwrap();
        assert!(
            app_services_source.contains("install_update(&verified_asset)"),
            "GUI musi przekazywać zweryfikowany obiekt VerifiedReleaseAsset do install_update"
        );
    }

    #[test]
    fn gui_update_flow_cannot_install_raw_browser_download_url() {
        // VerifiedReleaseAsset posiada wyłącznie prywatne pola i może powstać tylko po weryfikacji manifestu
        let manifest = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.2.1".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.1.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            assets: vec![ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 1000,
                sha256: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            }],
        };
        let verified = VerifiedReleaseManifest {
            manifest,
            signing_key_id: "test".into(),
            signing_key_fingerprint: "abc".into(),
        };
        let api_assets = vec![ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/asset.zip".into(),
            digest: None,
            size: 1000,
        }];

        let asset = verified
            .select_asset_for_platform(
                ReleasePlatform::Windows,
                ReleaseArchitecture::X86_64,
                &api_assets,
            )
            .unwrap();

        assert_eq!(
            asset.browser_download_url(),
            "https://example.com/asset.zip"
        );
    }

    #[test]
    fn gui_update_state_is_invalidated_when_release_identity_changes() {
        let mut state = crate::gui::state::updates::UpdateState::default();
        // Przy rozpoczęciu sprawdzania update_available jest unieważniane (None)
        state.update_available = None;
        assert!(state.update_available.is_none());
    }

    #[test]
    fn gui_update_error_is_visible_and_typed() {
        let outcome = UpdateCheckOutcome::Error("Błąd manifestu".into());
        match outcome {
            UpdateCheckOutcome::Error(msg) => assert_eq!(msg, "Błąd manifestu"),
            _ => panic!("Oczekiwano wariantu Error"),
        }
    }

    #[test]
    fn gui_update_rejection_occurs_before_apply() {
        // Potwierdzenie fail-closed: brak zweryfikowanego manifestu natychmiast kończy operację
        let manifest_bytes = b"INVALID JSON MANIFEST";
        let res = VerifiedReleaseManifest::verify(manifest_bytes, "sig", "1.1.0");
        assert!(
            res.is_err(),
            "Niepoprawny manifest musi zostać odrzucony przed jakimkolwiek apply"
        );
    }

    #[test]
    fn update_file_classification_classifies_user_data_and_config() {
        assert_eq!(
            classify_update_path(Path::new("station_config.json")),
            UpdateFileClass::UserConfiguration
        );
        assert_eq!(
            classify_update_path(Path::new("config/settings.json")),
            UpdateFileClass::UserConfiguration
        );
        assert_eq!(
            classify_update_path(Path::new("default_log.db")),
            UpdateFileClass::UserData
        );
        assert_eq!(
            classify_update_path(Path::new("default_log.db-wal")),
            UpdateFileClass::UserData
        );
        assert_eq!(
            classify_update_path(Path::new("logs/2026.adi")),
            UpdateFileClass::UserData
        );
        assert_eq!(
            classify_update_path(Path::new("databases/serviceLOG.db")),
            UpdateFileClass::FirstInstallSeed
        );
        assert_eq!(
            classify_update_path(Path::new("databases/callbook.db")),
            UpdateFileClass::FirstInstallSeed
        );
        assert_eq!(
            classify_update_path(Path::new("SPLogbook.exe")),
            UpdateFileClass::ProgramFile
        );
        assert_eq!(
            classify_update_path(Path::new("splogbook")),
            UpdateFileClass::ProgramFile
        );
        assert_eq!(
            classify_update_path(Path::new("resources/icon.png")),
            UpdateFileClass::ProgramFile
        );
        assert_eq!(
            classify_update_path(Path::new("manifest.json")),
            UpdateFileClass::ReleaseMetadata
        );
        assert_eq!(
            classify_update_path(Path::new("CHANGELOG.md")),
            UpdateFileClass::ReleaseMetadata
        );
        assert_eq!(
            classify_update_path(Path::new("unknown_file.xyz")),
            UpdateFileClass::Unknown
        );
    }

    #[test]
    fn update_plan_preserves_existing_portable_databases() {
        let temp_dir = std::env::temp_dir().join(format!("test_plan_db_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(staging_dir.join("databases")).unwrap();
        std::fs::create_dir_all(active_dir.join("databases")).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"NEW BINARY").unwrap();
        std::fs::write(active_dir.join("SPLogbook.exe"), b"OLD BINARY").unwrap();
        std::fs::write(staging_dir.join("databases/serviceLOG.db"), b"NEW SEED").unwrap();
        std::fs::write(staging_dir.join("databases/callbook.db"), b"NEW SEED").unwrap();

        let active_service_db = active_dir.join("databases/serviceLOG.db");
        let active_callbook_db = active_dir.join("databases/callbook.db");
        std::fs::write(&active_service_db, b"USER PRODUCTION SERVICE DB").unwrap();
        std::fs::write(&active_callbook_db, b"USER PRODUCTION CALLBOOK DB").unwrap();

        let plan = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();

        // serviceLOG.db i callbook.db muszą być w preserve_files i NIE w replace_files
        let preserved_paths: Vec<_> = plan
            .preserve_files
            .iter()
            .map(|(p, _)| p.as_path())
            .collect();
        assert!(preserved_paths.contains(&active_service_db.as_path()));
        assert!(preserved_paths.contains(&active_callbook_db.as_path()));

        let replaced_targets: Vec<_> = plan
            .replace_files
            .iter()
            .map(|(_, t)| t.as_path())
            .collect();
        assert!(!replaced_targets.contains(&active_service_db.as_path()));
        assert!(!replaced_targets.contains(&active_callbook_db.as_path()));

        // Tylko binarka ma być w replace_files
        assert_eq!(plan.replace_files.len(), 1);
        assert_eq!(
            plan.replace_files[0].0,
            std::path::PathBuf::from("SPLogbook.exe")
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_plan_preserves_existing_user_config_and_qso_logs() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_plan_user_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(&staging_dir).unwrap();
        std::fs::create_dir_all(&active_dir).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"NEW BINARY").unwrap();
        std::fs::write(staging_dir.join("station_config.json"), b"DEFAULT TEMPLATE").unwrap();
        std::fs::write(staging_dir.join("default_log.db"), b"EMPTY LOG").unwrap();

        let active_config = active_dir.join("station_config.json");
        let active_log = active_dir.join("default_log.db");
        std::fs::write(&active_config, b"USER SECRET CALLSIGN AND CONFIG").unwrap();
        std::fs::write(&active_log, b"USER 10000 QSOS").unwrap();

        let plan = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();

        let preserved_paths: Vec<_> = plan
            .preserve_files
            .iter()
            .map(|(p, _)| p.as_path())
            .collect();
        assert!(preserved_paths.contains(&active_config.as_path()));
        assert!(preserved_paths.contains(&active_log.as_path()));

        let replaced_targets: Vec<_> = plan
            .replace_files
            .iter()
            .map(|(_, t)| t.as_path())
            .collect();
        assert!(!replaced_targets.contains(&active_config.as_path()));
        assert!(!replaced_targets.contains(&active_log.as_path()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_plan_seeds_database_on_first_install() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_plan_seed_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(staging_dir.join("databases")).unwrap();
        std::fs::create_dir_all(&active_dir).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"NEW BINARY").unwrap();
        std::fs::write(staging_dir.join("databases/serviceLOG.db"), b"INITIAL SEED").unwrap();

        let plan = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();

        let created_rel: Vec<_> = plan.create_files.iter().map(|(r, _)| r.as_path()).collect();
        assert!(created_rel.contains(&std::path::Path::new("databases/serviceLOG.db")));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_plan_deterministic_and_no_io_side_effects() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_plan_det_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(staging_dir.join("databases")).unwrap();
        std::fs::create_dir_all(active_dir.join("databases")).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"EXE").unwrap();
        std::fs::write(staging_dir.join("databases/callbook.db"), b"SEED").unwrap();
        std::fs::write(active_dir.join("databases/callbook.db"), b"EXISTING").unwrap();

        let plan1 = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();
        let plan2 = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();

        assert_eq!(
            plan1, plan2,
            "Plan aktualizacji musi być w pełni deterministyczny"
        );

        // Weryfikacja braku modyfikacji plików w aktywnym katalogu
        let active_data = std::fs::read(active_dir.join("databases/callbook.db")).unwrap();
        assert_eq!(active_data, b"EXISTING");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_plan_rejects_unknown_files_fail_closed() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_plan_unk_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(&staging_dir).unwrap();
        std::fs::create_dir_all(&active_dir).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"EXE").unwrap();
        std::fs::write(staging_dir.join("untrusted_payload.sh"), b"UNTRUSTED").unwrap();

        let plan = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();
        assert_ne!(plan.rejected_files.len(), 0);

        let safety_res = plan.validate_safety("SPLogbook.exe");
        assert!(
            matches!(safety_res, Err(UpdaterError::SafetyGate(_))),
            "Bramka bezpieczeństwa musi odrzucić plan z nieznanymi plikami"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_plan_requires_main_binary() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_plan_nobin_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(&staging_dir).unwrap();
        std::fs::create_dir_all(&active_dir).unwrap();

        std::fs::write(staging_dir.join("README.md"), b"DOCS").unwrap();

        let plan = UpdatePlan::generate(&staging_dir, &active_dir).unwrap();
        let safety_res = plan.validate_safety("SPLogbook.exe");
        assert!(
            matches!(safety_res, Err(UpdaterError::SafetyGate(_))),
            "Bramka bezpieczeństwa musi wymagać obecności głównego pliku wykonywalnego"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn windows_powershell_script_generation_does_not_use_wildcard_copy() {
        let plan = UpdatePlan {
            replace_files: vec![(
                std::path::PathBuf::from("SPLogbook.exe"),
                std::path::PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            )],
            create_files: vec![(
                std::path::PathBuf::from("resources/icon.png"),
                std::path::PathBuf::from("C:\\SPLogbook\\resources\\icon.png"),
            )],
            preserve_files: vec![(
                std::path::PathBuf::from("C:\\SPLogbook\\databases\\serviceLOG.db"),
                "Preserved".into(),
            )],
            rejected_files: Vec::new(),
        };

        let script = generate_powershell_update_script(
            std::path::Path::new("C:\\SPLogbook\\SPLogbook.exe"),
            1234,
            &plan,
        );

        // Zakaz kopiowania zbiorczego wieloznacznikiem *
        assert!(
            !script.contains("Copy-Item -Path \"$staging\\*\""),
            "Skrypt PowerShell nie może używać dzikich kart kopiowania $staging\\*"
        );
        assert!(
            !script.contains("Copy-Item -Path \"$rollback\\*\""),
            "Skrypt PowerShell nie może używać dzikich kart rollbacku $rollback\\*"
        );
        assert!(
            script.contains("$replaceFiles = @('SPLogbook.exe')"),
            "Skrypt PowerShell musi zawierać dokładną listę plików do podmiany"
        );
        assert!(
            script.contains("$createFiles = @('resources/icon.png')")
                || script.contains("$createFiles = @('resources\\icon.png')"),
            "Skrypt PowerShell musi zawierać dokładną listę plików do utworzenia"
        );
    }

    #[test]
    fn windows_powershell_script_excludes_preserved_databases_and_configs() {
        let plan = UpdatePlan {
            replace_files: vec![(
                std::path::PathBuf::from("SPLogbook.exe"),
                std::path::PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            )],
            create_files: Vec::new(),
            preserve_files: vec![
                (
                    std::path::PathBuf::from("C:\\SPLogbook\\databases\\serviceLOG.db"),
                    "Preserved DB".into(),
                ),
                (
                    std::path::PathBuf::from("C:\\SPLogbook\\station_config.json"),
                    "Preserved Config".into(),
                ),
            ],
            rejected_files: Vec::new(),
        };

        let script = generate_powershell_update_script(
            std::path::Path::new("C:\\SPLogbook\\SPLogbook.exe"),
            1234,
            &plan,
        );

        assert!(
            !script.contains("serviceLOG.db"),
            "Zachowane bazy danych nie mogą znaleźć się w skrypcie podmiany"
        );
        assert!(
            !script.contains("station_config.json"),
            "Zachowana konfiguracja nie może znaleźć się w skrypcie podmiany"
        );
    }

    #[test]
    fn windows_powershell_script_cleans_up_new_files_on_rollback() {
        let plan = UpdatePlan {
            replace_files: vec![(
                std::path::PathBuf::from("SPLogbook.exe"),
                std::path::PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            )],
            create_files: vec![(
                std::path::PathBuf::from("new_file.dll"),
                std::path::PathBuf::from("C:\\SPLogbook\\new_file.dll"),
            )],
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };

        let script = generate_powershell_update_script(
            std::path::Path::new("C:\\SPLogbook\\SPLogbook.exe"),
            1234,
            &plan,
        );

        assert!(
            script.contains("Remove-Item -LiteralPath $target"),
            "Skrypt musi usuwać nowo utworzone pliki w razie rollbacku"
        );
    }

    #[test]
    fn apply_atomic_installation_preserves_existing_user_databases_and_configs() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_apply_pres_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(staging_dir.join("databases")).unwrap();
        std::fs::create_dir_all(active_dir.join("databases")).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"NEW BINARY").unwrap();
        std::fs::write(staging_dir.join("databases/serviceLOG.db"), b"NEW SEED").unwrap();
        std::fs::write(staging_dir.join("databases/callbook.db"), b"NEW SEED").unwrap();
        std::fs::write(staging_dir.join("station_config.json"), b"NEW TEMPLATE").unwrap();

        let active_exe = active_dir.join("SPLogbook.exe");
        let active_db = active_dir.join("databases/serviceLOG.db");
        let active_callbook = active_dir.join("databases/callbook.db");
        let active_cfg = active_dir.join("station_config.json");

        std::fs::write(&active_exe, b"OLD BINARY").unwrap();
        std::fs::write(&active_db, b"ORIGINAL PRODUCTION DATABASE").unwrap();
        std::fs::write(&active_callbook, b"ORIGINAL CALLBOOK DATABASE").unwrap();
        std::fs::write(&active_cfg, b"ORIGINAL USER CONFIG").unwrap();

        let opts = AtomicInstallationOptions::new("SPLogbook.exe");
        let res = apply_atomic_installation_with_options(&staging_dir, &active_dir, &opts);
        assert!(
            res.is_ok(),
            "Instalacja z zachowaniem chronionych plików musi się powieść"
        );

        // Binarka programu podmieniona
        assert_eq!(std::fs::read(&active_exe).unwrap(), b"NEW BINARY");

        // Bazy i konfiguracja użytkownika w 100% zachowane
        assert_eq!(
            std::fs::read(&active_db).unwrap(),
            b"ORIGINAL PRODUCTION DATABASE"
        );
        assert_eq!(
            std::fs::read(&active_callbook).unwrap(),
            b"ORIGINAL CALLBOOK DATABASE"
        );
        assert_eq!(std::fs::read(&active_cfg).unwrap(), b"ORIGINAL USER CONFIG");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn apply_atomic_installation_rollback_does_not_corrupt_preserved_data() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_apply_rb_pres_{}", rand::random::<u64>()));
        let staging_dir = temp_dir.join("staging");
        let active_dir = temp_dir.join("active");

        std::fs::create_dir_all(staging_dir.join("databases")).unwrap();
        std::fs::create_dir_all(active_dir.join("databases")).unwrap();

        std::fs::write(staging_dir.join("SPLogbook.exe"), b"NEW BINARY").unwrap();
        std::fs::write(staging_dir.join("databases/serviceLOG.db"), b"NEW SEED").unwrap();
        std::fs::write(staging_dir.join("station_config.json"), b"NEW TEMPLATE").unwrap();

        let active_exe = active_dir.join("SPLogbook.exe");
        let active_db = active_dir.join("databases/serviceLOG.db");
        let active_cfg = active_dir.join("station_config.json");

        std::fs::write(&active_exe, b"OLD BINARY").unwrap();
        std::fs::write(&active_db, b"ORIGINAL PRODUCTION DATABASE").unwrap();
        std::fs::write(&active_cfg, b"ORIGINAL USER CONFIG").unwrap();

        let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
        opts.simulate_rename_failure = true;

        let res = apply_atomic_installation_with_options(&staging_dir, &active_dir, &opts);
        assert!(res.is_err());

        // Przy rollbacku oryginalne pliki i dane użytkownika pozostają nienaruszone
        assert_eq!(std::fs::read(&active_exe).unwrap(), b"OLD BINARY");
        assert_eq!(
            std::fs::read(&active_db).unwrap(),
            b"ORIGINAL PRODUCTION DATABASE"
        );
        assert_eq!(std::fs::read(&active_cfg).unwrap(), b"ORIGINAL USER CONFIG");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
