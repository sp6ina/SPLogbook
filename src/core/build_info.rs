// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny moduł tożsamości kompilacji oraz metadanych Semantic Versioning aplikacji SPLogbook.
//!
//! Gwarantuje jednolitość tożsamości wersji w całym ekosystemie:
//! - aplikacji głównej GUI (`SPLogbook.exe`),
//! - pomocnika instalatora Windows (`splogbook-updater.exe`),
//! - narzędzia podpisywania manifestów (`splogbook-signer.exe`),
//! - transakcji aktualizacji (`UpdateTransaction`),
//! - dowodów uruchomienia kandydata (`HealthProof`),
//! - kontraktów pomocnika (`WindowsHelperRequest`, `WindowsHelperResult`).

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Kanał dystrybucji i aktualizacji oprogramowania SPLogbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// Wersja robocza deweloperska.
    Development,
    /// Wersja wczesna niestabilna (alpha).
    Alpha,
    /// Wersja testowa przedwydaniowa (beta).
    Beta,
    /// Kandydat do wydania (release candidate).
    Rc,
    /// Stabilne wydanie publiczne.
    Stable,
}

impl UpdateChannel {
    /// Zwraca kanoniczną nazwę tekstową kanału.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Rc => "rc",
            Self::Stable => "stable",
        }
    }

    /// Określa kanał na podstawie identyfikatora prerelease w SemVer.
    pub fn from_prerelease(pre: &str) -> Self {
        let pre_lower = pre.to_ascii_lowercase();
        if pre_lower.is_empty() {
            Self::Stable
        } else if pre_lower.starts_with("alpha") {
            Self::Alpha
        } else if pre_lower.starts_with("beta") {
            Self::Beta
        } else if pre_lower.starts_with("rc") {
            Self::Rc
        } else {
            Self::Development
        }
    }
}

/// Status dystrybucyjny kompilacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionStatus {
    /// Wersja lokalna / rozwojowa (nieopublikowana).
    Development,
    /// Opublikowany kandydat prerelease.
    PrereleaseCandidate,
    /// Oficjalne publiczne wydanie stabilne.
    PublishedStable,
}

impl DistributionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::PrereleaseCandidate => "prerelease_candidate",
            Self::PublishedStable => "published_stable",
        }
    }
}

/// Profil kompilacji Cargo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildProfile {
    Debug,
    Release,
    Test,
    Unknown,
}

impl BuildProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
            Self::Test => "test",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "debug" | "dev" => Self::Debug,
            "release" => Self::Release,
            "test" => Self::Test,
            _ => Self::Unknown,
        }
    }
}

/// Preferencja kanału aktualizacji konfigurowana przez użytkownika.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannelPreference {
    /// Pobieraj wyłącznie oficjalne stabilne wydania publiczne (np. 1.2.0, 1.3.0).
    StableOnly,
    /// Dopuszczaj wersje beta, rc i stabilne.
    Beta,
    /// Dopuszczaj wersje alpha, beta, rc i stabilne.
    Alpha,
    /// Dopuszczaj wszelkie kompilacje rozwojowe.
    Development,
}

impl UpdateChannelPreference {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StableOnly => "stable_only",
            Self::Beta => "beta",
            Self::Alpha => "alpha",
            Self::Development => "development",
        }
    }

    /// Sprawdza, czy preferencja użytkownika zezwala na dany kanał aktualizacji.
    pub fn allows_channel(self, channel: UpdateChannel) -> bool {
        match self {
            Self::StableOnly => channel == UpdateChannel::Stable,
            Self::Beta => matches!(
                channel,
                UpdateChannel::Beta | UpdateChannel::Rc | UpdateChannel::Stable
            ),
            Self::Alpha => matches!(
                channel,
                UpdateChannel::Alpha
                    | UpdateChannel::Beta
                    | UpdateChannel::Rc
                    | UpdateChannel::Stable
            ),
            Self::Development => true,
        }
    }
}

/// Centralny model tożsamości kompilacji aplikacji SPLogbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    /// Nazwa pakietu z Cargo.toml (`splogbook`).
    pub package_name: &'static str,
    /// Wersja pakietu pochodząca z `env!("CARGO_PKG_VERSION")`.
    pub package_version: &'static str,
    /// Sparsowana wersja SemVer.
    pub semver: semver::Version,
    /// Pełny 40-znakowy hash commita Git lub "unknown".
    pub git_commit_hash: &'static str,
    /// Skrócony 7-znakowy hash commita Git lub "unknown".
    pub git_commit_short: &'static str,
    /// Flaga oznaczająca czy w chwili kompilacji repozytorium miało niescommitowane zmiany.
    pub git_dirty: bool,
    /// Kanał aktualizacji wyznaczony z SemVer.
    pub channel: UpdateChannel,
    /// Status dystrybucyjny kompilacji.
    pub distribution_status: DistributionStatus,
    /// Profil kompilatora Cargo (debug/release/test).
    pub profile: BuildProfile,
    /// Docelowy system operacyjny (np. "windows", "linux").
    pub target_os: &'static str,
    /// Docelowa architektura procesora (np. "x86_64").
    pub target_arch: &'static str,
}

impl BuildIdentity {
    /// Zwraca globalną statyczną instancję tożsamości kompilacji.
    pub fn current() -> &'static Self {
        static INSTANCE: OnceLock<BuildIdentity> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let pkg_version = env!("CARGO_PKG_VERSION");
            let semver_parsed = semver::Version::parse(pkg_version)
                .expect("CARGO_PKG_VERSION musi być poprawną wersją SemVer");

            let channel = UpdateChannel::from_prerelease(semver_parsed.pre.as_str());

            // Status dystrybucyjny: w repozytorium roboczym jest to wersja rozwojowa (Development)
            let distribution_status = if semver_parsed.pre.is_empty() {
                DistributionStatus::PublishedStable
            } else {
                DistributionStatus::Development
            };

            let profile_str = env!("SPLOGBOOK_BUILD_PROFILE");
            let profile = BuildProfile::from_str_name(profile_str);

            let dirty_str = env!("SPLOGBOOK_GIT_DIRTY");
            let git_dirty = dirty_str.trim().eq_ignore_ascii_case("true");

            Self {
                package_name: env!("CARGO_PKG_NAME"),
                package_version: pkg_version,
                semver: semver_parsed,
                git_commit_hash: env!("SPLOGBOOK_GIT_COMMIT_HASH"),
                git_commit_short: env!("SPLOGBOOK_GIT_COMMIT_SHORT"),
                git_dirty,
                channel,
                distribution_status,
                profile,
                target_os: std::env::consts::OS,
                target_arch: std::env::consts::ARCH,
            }
        })
    }

    /// Zwraca ciąg wersji do prezentacji użytkownikowi (np. "1.3.0-alpha.1").
    pub fn display_version(&self) -> &str {
        self.package_version
    }

    /// Czy bieżąca wersja jest wersją przedwydaniową (prerelease).
    pub fn is_prerelease(&self) -> bool {
        !self.semver.pre.is_empty()
    }

    /// Czy bieżąca kompilacja reprezentuje oficjalne stabilne wydanie.
    ///
    /// Uwaga: Profil `Release` kompilatora NIE oznacza stabilnego wydania.
    /// Wersja alpha zbudowana w profilu Release nadal pozostaje wersją niestabilną / alpha.
    pub fn is_stable(&self) -> bool {
        self.channel == UpdateChannel::Stable
            && self.semver.pre.is_empty()
            && self.distribution_status == DistributionStatus::PublishedStable
    }

    /// Zwraca zwięzłe podsumowanie tożsamości docelowej platformy (np. "windows-x86_64").
    pub fn target_platform(&self) -> String {
        format!("{}-{}", self.target_os, self.target_arch)
    }

    /// Zwraca pełny ciąg diagnostyczny tożsamości kompilacji dla opcji `--build-info`.
    pub fn format_diagnostic(&self) -> String {
        format!(
            "SPLogbook {}\nVersion: {}\nCommit: {}{}\nChannel: {}\nDistribution Status: {}\nProfile: {}\nTarget: {}-{}",
            self.package_version,
            self.package_version,
            self.git_commit_hash,
            if self.git_dirty { " (dirty)" } else { "" },
            self.channel.as_str(),
            self.distribution_status.as_str(),
            self.profile.as_str(),
            self.target_os,
            self.target_arch,
        )
    }

    /// Zwraca zwięzłą tożsamość dla narzędzia `splogbook-updater --diagnostic-version`.
    pub fn format_updater_diagnostic(&self) -> String {
        format!(
            "{} (commit: {}, channel: {}, profile: {})",
            self.package_version,
            self.git_commit_short,
            self.channel.as_str(),
            self.profile.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_identity_reports_alpha_development_version() {
        let id = BuildIdentity::current();
        assert_eq!(id.package_name, "splogbook");
        assert_eq!(id.package_version, "1.3.0-alpha.1");
        assert_eq!(id.semver.major, 1);
        assert_eq!(id.semver.minor, 3);
        assert_eq!(id.semver.patch, 0);
        assert_eq!(id.semver.pre.as_str(), "alpha.1");
        assert!(id.is_prerelease());
        assert!(!id.is_stable());
        assert_eq!(id.channel, UpdateChannel::Alpha);
        assert_eq!(id.distribution_status, DistributionStatus::Development);
    }

    #[test]
    fn build_profile_release_does_not_imply_stable() {
        let mut id = BuildIdentity::current().clone();
        id.profile = BuildProfile::Release;
        assert_eq!(id.channel, UpdateChannel::Alpha);
        // Profil release dla wersji alpha pozostaje niestabilny
        assert!(!id.is_stable());
        assert!(id.is_prerelease());
    }

    #[test]
    fn channel_from_prerelease_mapping() {
        assert_eq!(UpdateChannel::from_prerelease(""), UpdateChannel::Stable);
        assert_eq!(
            UpdateChannel::from_prerelease("alpha.1"),
            UpdateChannel::Alpha
        );
        assert_eq!(
            UpdateChannel::from_prerelease("beta.2"),
            UpdateChannel::Beta
        );
        assert_eq!(UpdateChannel::from_prerelease("rc.1"), UpdateChannel::Rc);
        assert_eq!(
            UpdateChannel::from_prerelease("dev.20261107"),
            UpdateChannel::Development
        );
    }

    #[test]
    fn channel_preference_filtering() {
        assert!(UpdateChannelPreference::StableOnly.allows_channel(UpdateChannel::Stable));
        assert!(!UpdateChannelPreference::StableOnly.allows_channel(UpdateChannel::Alpha));
        assert!(!UpdateChannelPreference::StableOnly.allows_channel(UpdateChannel::Beta));
        assert!(!UpdateChannelPreference::StableOnly.allows_channel(UpdateChannel::Rc));

        assert!(UpdateChannelPreference::Alpha.allows_channel(UpdateChannel::Alpha));
        assert!(UpdateChannelPreference::Alpha.allows_channel(UpdateChannel::Beta));
        assert!(UpdateChannelPreference::Alpha.allows_channel(UpdateChannel::Stable));

        assert!(!UpdateChannelPreference::Beta.allows_channel(UpdateChannel::Alpha));
        assert!(UpdateChannelPreference::Beta.allows_channel(UpdateChannel::Beta));
        assert!(UpdateChannelPreference::Beta.allows_channel(UpdateChannel::Stable));
    }

    #[test]
    fn diagnostic_strings_contain_version_and_channel() {
        let id = BuildIdentity::current();
        let diag = id.format_diagnostic();
        assert!(diag.contains("SPLogbook 1.3.0-alpha.1"));
        assert!(diag.contains("Version: 1.3.0-alpha.1"));
        assert!(diag.contains("Channel: alpha"));
        assert!(diag.contains("Distribution Status: development"));

        let updater_diag = id.format_updater_diagnostic();
        assert!(updater_diag.starts_with("1.3.0-alpha.1"));
        assert!(updater_diag.contains("channel: alpha"));
    }
}
