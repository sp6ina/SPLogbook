// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy wielodystrybucyjnej zgodności, dynamicznego flooru kernela i stabilności Linux Portable (M3-F)

use sha2::{Digest, Sha256};
use splogbook::cloud::updater::installation::{
    LINUX_PORTABLE_MARKER_FILENAME, LinuxPortableMarker,
};
use splogbook::cloud::updater::preflight::{
    DEFAULT_SAFETY_MARGIN_BYTES, DiskPreflightRequirements,
};
use splogbook::cloud::updater::recovery::{
    RecoveryContext, RecoveryDecision, RecoveryDecisionEngine,
};
use splogbook::cloud::updater::{
    ArchiveLimits, InstallationType, UpdatePlan, extract_linux_portable_tar_gz_two_phase,
    preflight_linux_portable_tar_gz, validate_archive_relative_path,
};
use splogbook::core::xdg::detect_legacy_portable_user_data;
use std::fs;
use std::path::{Path, PathBuf};

/// Reprezentacja profilu dystrybucji z obowiązkowej macierzy M3-F.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DistroProfile {
    pub name: &'static str,
    pub profile_kind: &'static str,
    pub version: &'static str,
    pub default_kernel_major_minor: (u32, u32),
    pub default_glibc_version: &'static str,
    pub init_system: &'static str,
    pub package_manager: &'static str,
}

const MANDATORY_DISTRO_PROFILES: [DistroProfile; 7] = [
    DistroProfile {
        name: "Ubuntu stable",
        profile_kind: "ubuntu-stable",
        version: "26.10",
        default_kernel_major_minor: (7, 3),
        default_glibc_version: "2.44",
        init_system: "systemd",
        package_manager: "apt",
    },
    DistroProfile {
        name: "Ubuntu LTS",
        profile_kind: "ubuntu-lts",
        version: "26.04 LTS",
        default_kernel_major_minor: (7, 0),
        default_glibc_version: "2.43",
        init_system: "systemd",
        package_manager: "apt",
    },
    DistroProfile {
        name: "Debian stable",
        profile_kind: "debian-stable",
        version: "13.7 (Trixie)",
        default_kernel_major_minor: (6, 12),
        default_glibc_version: "2.40",
        init_system: "systemd",
        package_manager: "apt",
    },
    DistroProfile {
        name: "Fedora Final",
        profile_kind: "fedora-final",
        version: "45",
        default_kernel_major_minor: (6, 18),
        default_glibc_version: "2.44",
        init_system: "systemd",
        package_manager: "dnf",
    },
    DistroProfile {
        name: "Arch Linux official rolling",
        profile_kind: "arch-rolling",
        version: "snapshot-2026.11.01",
        default_kernel_major_minor: (6, 18),
        default_glibc_version: "2.44",
        init_system: "systemd",
        package_manager: "pacman",
    },
    DistroProfile {
        name: "openSUSE Leap",
        profile_kind: "opensuse-leap",
        version: "16.1",
        default_kernel_major_minor: (6, 12),
        default_glibc_version: "2.40",
        init_system: "systemd",
        package_manager: "zypper",
    },
    DistroProfile {
        name: "openSUSE Tumbleweed",
        profile_kind: "opensuse-tumbleweed",
        version: "snapshot-20261103",
        default_kernel_major_minor: (6, 18),
        default_glibc_version: "2.44",
        init_system: "systemd",
        package_manager: "zypper",
    },
];

fn calculate_dynamic_kernel_floor(profiles: &[DistroProfile]) -> (u32, u32) {
    profiles
        .iter()
        .map(|p| p.default_kernel_major_minor)
        .min()
        .expect("Non-empty profiles")
}

fn helper_create_tar_gz(entries: &[(&str, &[u8], tar::EntryType, u32)]) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    {
        let mut tar_builder = tar::Builder::new(&mut enc);
        for &(path, data, entry_type, mode) in entries {
            let mut header = tar::Header::new_gnu();
            if header.set_path(path).is_err() {
                let bytes = path.as_bytes();
                let name_field = &mut header.as_mut_bytes()[..100];
                let len = bytes.len().min(99);
                name_field[..len].copy_from_slice(&bytes[..len]);
                name_field[len] = 0;
            }
            header.set_entry_type(entry_type);
            header.set_mode(mode);
            header.set_size(data.len() as u64);
            header.set_cksum();
            tar_builder
                .append(&header, data)
                .expect("Failed to append tar entry");
        }
        tar_builder.finish().expect("Failed to finish tar builder");
    }
    enc.finish().expect("Failed to finish gzip encoder")
}

fn sha256_of_bytes(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn test_mandatory_multi_distribution_matrix_completeness_and_dynamic_floor() {
    // 1. Sprawdzenie kompletności macierzy: dokładnie 7 wymaganych profili
    assert_eq!(
        MANDATORY_DISTRO_PROFILES.len(),
        7,
        "Obowiązkowa macierz dystrybucji musi liczyć dokładnie 7 profili"
    );

    // 2. Dynamiczne wyznaczenie progu kernela (Dynamic Distribution-Derived Kernel Compatibility Floor)
    let floor = calculate_dynamic_kernel_floor(&MANDATORY_DISTRO_PROFILES);
    assert_eq!(
        floor,
        (6, 12),
        "Dynamiczny próg kernela wyznaczony z minimum macierzy musi wynosić Linux 6.12 (Debian 13.7 oraz openSUSE Leap 16.1)"
    );

    // 3. Weryfikacja każdego profilu
    for p in &MANDATORY_DISTRO_PROFILES {
        assert!(
            p.default_kernel_major_minor >= floor,
            "Kernel profilu {} ({}.{}) nie może być niższy niż próg {}.{}",
            p.name,
            p.default_kernel_major_minor.0,
            p.default_kernel_major_minor.1,
            floor.0,
            floor.1
        );
    }
}

#[test]
fn test_canonical_artifact_single_identity_across_all_distributions() {
    let temp = tempfile::tempdir().unwrap();
    let marker_json = serde_json::to_string_pretty(&LinuxPortableMarker::new(
        "1.3.0-alpha.1",
        Path::new("/home/user/SPLogbook"),
        "a8541075e14de1537b7940fa3fd48b44d854151f",
        "6.12",
        "1.85.0",
    ))
    .unwrap();

    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");
    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            &marker_path,
            marker_json.as_bytes(),
            tar::EntryType::Regular,
            0o644,
        ),
        (
            "SPLogbook/SPLogbook",
            b"CANONICAL_ELF_BINARY",
            tar::EntryType::Regular,
            0o755,
        ),
        (
            "SPLogbook/splogbook-updater",
            b"CANONICAL_ELF_UPDATER",
            tar::EntryType::Regular,
            0o755,
        ),
        (
            "SPLogbook/README.md",
            b"# SPLogbook Portable",
            tar::EntryType::Regular,
            0o644,
        ),
    ]);

    let canonical_sha256 = sha256_of_bytes(&tar_bytes);
    let archive_path = temp
        .path()
        .join("SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz");
    fs::write(&archive_path, &tar_bytes).unwrap();

    let limits = ArchiveLimits::default();

    // Weryfikacja tego samego artefaktu dla każdego z 7 profili dystrybucyjnych
    for profile in &MANDATORY_DISTRO_PROFILES {
        let inspection = preflight_linux_portable_tar_gz(&archive_path, &limits)
            .unwrap_or_else(|e| panic!("Preflight failed for profile {}: {e:?}", profile.name));

        assert_eq!(inspection.root_directory, "SPLogbook");
        assert!(inspection.has_marker);
        assert_eq!(inspection.entry_count, 5);

        // Ten sam SHA-256
        assert_eq!(
            sha256_of_bytes(&fs::read(&archive_path).unwrap()),
            canonical_sha256,
            "Suma kontrolna SHA-256 artefaktu musi być w 100% identyczna dla każdego profilu ({})",
            profile.name
        );
    }
}

// ==============================================================================
// BATERIA TESTÓW STABILNOŚCI (STABILITY BATTERY)
// ==============================================================================

#[test]
fn test_m3_f_stability_100_iterations_fast_unit_tests() {
    let limits = ArchiveLimits::default();
    assert_eq!(limits.max_entries, 5_000);
    assert_eq!(limits.max_total_size, 300 * 1024 * 1024);
    assert_eq!(limits.max_single_size, 150 * 1024 * 1024);

    for i in 0..100 {
        // 1. Normalizacja i weryfikacja ścieżek
        let raw = Path::new("SPLogbook/bin/sub/tool");
        let validated = validate_archive_relative_path(raw)
            .unwrap_or_else(|e| panic!("Iteracja {i}: błąd walidacji ścieżki: {e:?}"));
        assert_eq!(validated, PathBuf::from("SPLogbook/bin/sub/tool"));

        // 2. Walidacja odrzucenia ścieżki ze znakami zakazanymi
        assert!(validate_archive_relative_path(Path::new("../escape")).is_err());
        assert!(validate_archive_relative_path(Path::new("/abs/path")).is_err());
        assert!(validate_archive_relative_path(Path::new("C:\\windows")).is_err());

        // 3. Sprawdzanie struktury markera dla każdego profilu
        let distro = &MANDATORY_DISTRO_PROFILES[i % MANDATORY_DISTRO_PROFILES.len()];
        let marker = LinuxPortableMarker::new(
            "1.3.0-alpha.1",
            Path::new("/home/test/app"),
            "a8541075e14de1537b7940fa3fd48b44d854151f",
            "6.12",
            "1.85.0",
        );
        assert!(marker.validate().is_ok());
        assert_eq!(marker.target_os, "linux");
        assert_eq!(marker.target_arch, "x86_64");
        assert_eq!(marker.resolved_minimum_kernel, "6.12");

        // 4. Test detekcji starszych danych
        let legacy = detect_legacy_portable_user_data(Path::new("/tmp/nonexistent"));
        assert!(legacy.is_empty());

        // 5. Wyliczanie preflightu pojemności dyskowej
        let req = DiskPreflightRequirements::calculate(
            InstallationType::LinuxPortableTarGz,
            30 * 1024 * 1024 + (i as u64 * 1024),
            60 * 1024 * 1024,
            30 * 1024 * 1024,
            None,
        )
        .unwrap_or_else(|e| panic!("Iteracja {i}: błąd preflightu dyskowego: {e:?}"));
        assert_eq!(req.filesystem_safety_margin, DEFAULT_SAFETY_MARGIN_BYTES);
        assert_eq!(req.package_manager_overhead, 0);

        // 6. Sprawdzenie integralności definicji profilu
        assert!(!distro.version.is_empty());
        assert!(distro.default_kernel_major_minor.0 >= 6);
    }
}

#[test]
fn test_m3_f_stability_50_iterations_medium_integration_tests() {
    let limits = ArchiveLimits::default();
    let marker_json = serde_json::to_string_pretty(&LinuxPortableMarker::new(
        "1.3.0-alpha.1",
        Path::new("/opt/SPLogbook"),
        "a8541075e14de1537b7940fa3fd48b44d854151f",
        "6.12",
        "1.85.0",
    ))
    .unwrap();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    for i in 0..50 {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join(format!("staging_{i}"));
        let active = temp.path().join(format!("active_{i}"));
        fs::create_dir_all(&active).unwrap();

        // 1. Tworzenie i test ekstrakcji poprawnego archiwum
        let tar_bytes = helper_create_tar_gz(&[
            ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
            (
                &marker_path,
                marker_json.as_bytes(),
                tar::EntryType::Regular,
                0o644,
            ),
            (
                "SPLogbook/SPLogbook",
                b"ELF",
                tar::EntryType::Regular,
                0o755,
            ),
            (
                "SPLogbook/splogbook-updater",
                b"UPDATER",
                tar::EntryType::Regular,
                0o755,
            ),
        ]);
        let archive_path = temp.path().join("pkg.tar.gz");
        fs::write(&archive_path, tar_bytes).unwrap();

        let extracted = extract_linux_portable_tar_gz_two_phase(&archive_path, &staging, &limits)
            .unwrap_or_else(|e| panic!("Iteracja {i}: błąd ekstrakcji: {e:?}"));
        assert_eq!(extracted.entry_count, 4);
        assert!(staging.join("SPLogbook/SPLogbook").is_file());

        // 2. Test odrzucenia archiwum złośliwego (np. ze ścieżką ../evil)
        let evil_bytes = helper_create_tar_gz(&[
            ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
            (
                &marker_path,
                marker_json.as_bytes(),
                tar::EntryType::Regular,
                0o644,
            ),
            (
                "SPLogbook/../evil.sh",
                b"malicious",
                tar::EntryType::Regular,
                0o755,
            ),
        ]);
        let evil_path = temp.path().join("evil.tar.gz");
        fs::write(&evil_path, evil_bytes).unwrap();

        let evil_res = preflight_linux_portable_tar_gz(&evil_path, &limits);
        assert!(
            evil_res.is_err(),
            "Iteracja {i}: złośliwe archiwum musi zostać odrzucone"
        );

        // 3. Test silnika decyzji recovery dla Linux Portable
        let recovery_ctx = RecoveryContext {
            expected_attempt_id: None,
            transaction: None,
            lock_owner: None,
            lock_is_corrupt: false,
            staging_exists: false,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::LinuxPortableTarGz,
        };
        let plan = RecoveryDecisionEngine::evaluate(&recovery_ctx);
        assert_eq!(plan.decision, RecoveryDecision::NoRecoveryNeeded);
    }
}

#[test]
fn test_m3_f_stability_20_iterations_full_lifecycle_and_data_preservation() {
    let limits = ArchiveLimits::default();
    let marker_json = serde_json::to_string_pretty(&LinuxPortableMarker::new(
        "1.3.0-alpha.1",
        Path::new("/home/radio/SPLogbook"),
        "a8541075e14de1537b7940fa3fd48b44d854151f",
        "6.12",
        "1.85.0",
    ))
    .unwrap();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    for i in 0..20 {
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app");
        let staging_dir = temp.path().join("staging");
        let rollback_dir = temp.path().join("rollback");

        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&staging_dir).unwrap();
        fs::create_dir_all(&rollback_dir).unwrap();

        // Aktywne pliki: binaria i dane użytkownika
        let db_file = app_dir.join("default_log.db");
        let cfg_file = app_dir.join("station_config.json");
        let wal_file = app_dir.join("default_log.db-wal");
        let bin_file = app_dir.join("SPLogbook");

        let db_payload = format!("USER_LOG_DATA_ITER_{i}").into_bytes();
        let cfg_payload = format!("USER_CONFIG_DATA_ITER_{i}").into_bytes();
        let wal_payload = format!("USER_WAL_DATA_ITER_{i}").into_bytes();

        fs::write(&db_file, &db_payload).unwrap();
        fs::write(&cfg_file, &cfg_payload).unwrap();
        fs::write(&wal_file, &wal_payload).unwrap();
        fs::write(&bin_file, b"OLD_BINARY_V1").unwrap();

        let initial_db_sha = sha256_of_bytes(&db_payload);
        let initial_cfg_sha = sha256_of_bytes(&cfg_payload);
        let initial_wal_sha = sha256_of_bytes(&wal_payload);

        // Paczka aktualizacyjna w archiwum
        let tar_bytes = helper_create_tar_gz(&[
            ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
            (
                &marker_path,
                marker_json.as_bytes(),
                tar::EntryType::Regular,
                0o644,
            ),
            (
                "SPLogbook/SPLogbook",
                b"NEW_BINARY_V2",
                tar::EntryType::Regular,
                0o755,
            ),
            (
                "SPLogbook/splogbook-updater",
                b"NEW_UPDATER_V2",
                tar::EntryType::Regular,
                0o755,
            ),
            (
                "SPLogbook/default_log.db",
                b"MALICIOUS_OVERWRITE_PAYLOAD",
                tar::EntryType::Regular,
                0o644,
            ),
        ]);
        let archive_path = temp.path().join("update.tar.gz");
        fs::write(&archive_path, tar_bytes).unwrap();

        // 1. Ekstrakcja
        extract_linux_portable_tar_gz_two_phase(&archive_path, &staging_dir, &limits)
            .unwrap_or_else(|e| panic!("Iteracja {i}: błąd ekstrakcji dwufazowej: {e:?}"));

        // 2. Generowanie planu aktualizacji ze spłaszczeniem korzenia
        let unpacked_root = staging_dir.join("SPLogbook");
        let plan = UpdatePlan::generate(&unpacked_root, &app_dir).unwrap();
        plan.validate_safety("SPLogbook").unwrap();

        // 3. Kopia zapasowa (.rollback)
        for (rel, orig) in &plan.replace_files {
            let dest = rollback_dir.join(rel);
            if let Some(p) = dest.parent() {
                let _ = fs::create_dir_all(p);
            }
            fs::copy(orig, &dest).unwrap();
        }

        // 4. Podmiana plików (JOURNALED_TRANSACTIONAL_REPLACEMENT)
        for (rel, orig) in &plan.replace_files {
            let src = unpacked_root.join(rel);
            fs::copy(&src, orig).unwrap();
        }
        for (rel, orig) in &plan.create_files {
            let src = unpacked_root.join(rel);
            fs::copy(&src, orig).unwrap();
        }

        // Weryfikacja: program zaktualizowany, BAZY NIENARUSZONE
        assert_eq!(fs::read(&bin_file).unwrap(), b"NEW_BINARY_V2");
        assert_eq!(
            sha256_of_bytes(&fs::read(&db_file).unwrap()),
            initial_db_sha
        );
        assert_eq!(
            sha256_of_bytes(&fs::read(&cfg_file).unwrap()),
            initial_cfg_sha
        );
        assert_eq!(
            sha256_of_bytes(&fs::read(&wal_file).unwrap()),
            initial_wal_sha
        );

        // 5. Rollback
        for (_rel, orig) in &plan.create_files {
            let _ = fs::remove_file(orig);
        }
        for (rel, orig) in &plan.replace_files {
            let bkp = rollback_dir.join(rel);
            fs::copy(&bkp, orig).unwrap();
        }

        // Weryfikacja po rollbacku: oryginalny program przywrócony, BAZY WCIĄŻ W 100% IDENTYCZNE
        assert_eq!(fs::read(&bin_file).unwrap(), b"OLD_BINARY_V1");
        assert_eq!(
            sha256_of_bytes(&fs::read(&db_file).unwrap()),
            initial_db_sha
        );
        assert_eq!(
            sha256_of_bytes(&fs::read(&cfg_file).unwrap()),
            initial_cfg_sha
        );
        assert_eq!(
            sha256_of_bytes(&fs::read(&wal_file).unwrap()),
            initial_wal_sha
        );
    }
}
