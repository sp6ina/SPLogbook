// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy bezpieczeństwa ekstrakcji i preflightu archiwów Linux Portable tar.gz (M3-F)

use splogbook::cloud::updater::installation::{
    LINUX_PORTABLE_MARKER_FILENAME, LinuxPortableMarker,
};
use splogbook::cloud::updater::{
    ArchiveLimits, UpdaterError, extract_linux_portable_tar_gz_two_phase,
    preflight_linux_portable_tar_gz, validate_archive_relative_path,
};
use std::fs;
use std::path::Path;

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

fn helper_create_valid_marker_json() -> String {
    let marker = LinuxPortableMarker::new(
        "1.3.0-alpha.1",
        Path::new("/opt/SPLogbook"),
        "a8541075e14de1537b7940fa3fd48b44d854151f",
        "6.12",
        "1.85.0",
    );
    serde_json::to_string_pretty(&marker).expect("Serialization failed")
}

#[test]
fn test_valid_linux_portable_archive_passes_preflight_and_two_phase_extraction() {
    let temp_dir = std::env::temp_dir().join(format!("test_valid_tar_{}", rand::random::<u64>()));
    let staging = temp_dir.join("staging");
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
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
            b"ELF BINARY",
            tar::EntryType::Regular,
            0o755,
        ),
        (
            "SPLogbook/splogbook-updater",
            b"ELF UPDATER",
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

    let archive_path = temp_dir.join("SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();

    // 1. Faza 1: Preflight read-only
    let preflight_res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(
        preflight_res.is_ok(),
        "Preflight must succeed for valid archive: {preflight_res:?}"
    );
    let inspection = preflight_res.unwrap();
    assert_eq!(inspection.root_directory, "SPLogbook");
    assert!(inspection.has_marker);
    assert_eq!(inspection.entry_count, 5);

    // Przed Fazą 2 katalog staging nie istnieje
    assert!(!staging.exists());

    // 2. Faza 2: Kontrolowana dwufazowa ekstrakcja
    let extract_res = extract_linux_portable_tar_gz_two_phase(&archive_path, &staging, &limits);
    assert!(
        extract_res.is_ok(),
        "Extraction must succeed: {extract_res:?}"
    );

    assert!(staging.join("SPLogbook").is_dir());
    assert!(staging.join(&marker_path).is_file());
    assert!(staging.join("SPLogbook/SPLogbook").is_file());
    assert!(staging.join("SPLogbook/splogbook-updater").is_file());
    assert!(staging.join("SPLogbook/README.md").is_file());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_missing_marker_rejected_in_preflight_without_disk_write() {
    let temp_dir = std::env::temp_dir().join(format!("test_no_marker_{}", rand::random::<u64>()));
    let staging = temp_dir.join("staging");
    fs::create_dir_all(&temp_dir).unwrap();

    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            "SPLogbook/SPLogbook",
            b"ELF BINARY",
            tar::EntryType::Regular,
            0o755,
        ),
    ]);
    let archive_path = temp_dir.join("no_marker.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = extract_linux_portable_tar_gz_two_phase(&archive_path, &staging, &limits);
    assert!(matches!(res, Err(UpdaterError::InstallationDetection(_))));
    // FAIL-CLOSED: Żaden katalog ani plik nie został utworzony
    assert!(!staging.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_multiple_roots_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_multi_root_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
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
            "OtherRoot/payload.bin",
            b"evil",
            tar::EntryType::Regular,
            0o644,
        ),
    ]);
    let archive_path = temp_dir.join("multi_root.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::InvalidStagingLayout(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_file_directly_in_root_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_root_file_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            &marker_path,
            marker_json.as_bytes(),
            tar::EntryType::Regular,
            0o644,
        ),
        ("root_file.sh", b"#!/bin/sh", tar::EntryType::Regular, 0o755),
    ]);
    let archive_path = temp_dir.join("root_file.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::InvalidStagingLayout(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_symlink_entry_rejected_in_preflight_without_disk_write() {
    let temp_dir = std::env::temp_dir().join(format!("test_symlink_{}", rand::random::<u64>()));
    let staging = temp_dir.join("staging");
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
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
            "SPLogbook/sym_link",
            b"target",
            tar::EntryType::Symlink,
            0o755,
        ),
    ]);
    let archive_path = temp_dir.join("symlink.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = extract_linux_portable_tar_gz_two_phase(&archive_path, &staging, &limits);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));
    assert!(!staging.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_hardlink_entry_rejected_in_preflight() {
    let temp_dir = std::env::temp_dir().join(format!("test_hardlink_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
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
            "SPLogbook/hard_link",
            b"target",
            tar::EntryType::Link,
            0o755,
        ),
    ]);
    let archive_path = temp_dir.join("hardlink.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_setuid_setgid_sticky_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_setuid_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    // Mode with setuid bit (0o4755)
    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            &marker_path,
            marker_json.as_bytes(),
            tar::EntryType::Regular,
            0o644,
        ),
        (
            "SPLogbook/dangerous_suid",
            b"payload",
            tar::EntryType::Regular,
            0o4755,
        ),
    ]);
    let archive_path = temp_dir.join("suid.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_special_device_fifo_socket_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_fifo_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            &marker_path,
            marker_json.as_bytes(),
            tar::EntryType::Regular,
            0o644,
        ),
        ("SPLogbook/fifo_node", &[], tar::EntryType::Fifo, 0o644),
    ]);
    let archive_path = temp_dir.join("fifo.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_parent_dir_dot_dot_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_dot_dot_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
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
            "SPLogbook/../escape.txt",
            b"escaped",
            tar::EntryType::Regular,
            0o644,
        ),
    ]);
    let archive_path = temp_dir.join("dot_dot.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_absolute_path_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_abs_path_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            &marker_path,
            marker_json.as_bytes(),
            tar::EntryType::Regular,
            0o644,
        ),
        ("/etc/shadow", b"root:!", tar::EntryType::Regular, 0o644),
    ]);
    let archive_path = temp_dir.join("abs_path.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_duplicate_paths_rejected() {
    let temp_dir = std::env::temp_dir().join(format!("test_dup_paths_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
    let marker_path = format!("SPLogbook/{LINUX_PORTABLE_MARKER_FILENAME}");

    let tar_bytes = helper_create_tar_gz(&[
        ("SPLogbook/", &[], tar::EntryType::Directory, 0o755),
        (
            &marker_path,
            marker_json.as_bytes(),
            tar::EntryType::Regular,
            0o644,
        ),
        ("SPLogbook/file.txt", b"v1", tar::EntryType::Regular, 0o644),
        ("SPLogbook/file.txt", b"v2", tar::EntryType::Regular, 0o644),
    ]);
    let archive_path = temp_dir.join("dup_paths.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    let limits = ArchiveLimits::default();
    let res = preflight_linux_portable_tar_gz(&archive_path, &limits);
    assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_limits_enforced_in_preflight() {
    let temp_dir = std::env::temp_dir().join(format!("test_limits_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_json = helper_create_valid_marker_json();
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
            "SPLogbook/big.bin",
            &[0u8; 1000],
            tar::EntryType::Regular,
            0o644,
        ),
    ]);
    let archive_path = temp_dir.join("limits.tar.gz");
    fs::write(&archive_path, tar_bytes).unwrap();

    // 1. Limit wpisów: max 1 wpis
    let entry_limits = ArchiveLimits {
        max_entries: 1,
        max_total_size: 1_000_000,
        max_single_size: 1_000_000,
    };
    let res = preflight_linux_portable_tar_gz(&archive_path, &entry_limits);
    assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

    // 2. Limit pojedynczego pliku: max 500 B
    let single_limits = ArchiveLimits {
        max_entries: 10,
        max_total_size: 1_000_000,
        max_single_size: 500,
    };
    let res = preflight_linux_portable_tar_gz(&archive_path, &single_limits);
    assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

    // 3. Limit całkowity: max 800 B
    let total_limits = ArchiveLimits {
        max_entries: 10,
        max_total_size: 800,
        max_single_size: 1_000_000,
    };
    let res = preflight_linux_portable_tar_gz(&archive_path, &total_limits);
    assert!(matches!(res, Err(UpdaterError::ArchiveLimitExceeded(_))));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_archive_path_len_exceeded_255_rejected() {
    let long_component = "a".repeat(250);
    let long_path = format!("SPLogbook/{long_component}");
    let raw_path = Path::new(&long_path);

    let res = validate_archive_relative_path(raw_path);
    assert!(matches!(res, Err(UpdaterError::UnsafeArchivePath(_))));
}
