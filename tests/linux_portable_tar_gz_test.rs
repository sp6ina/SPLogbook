// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy definicji, struktury i markera pakietu Linux Portable tar.gz (M3-F)

use splogbook::cloud::updater::installation::{
    InstallationDetectionContext, InstallationType, LINUX_PORTABLE_MARKER_FILENAME,
    LinuxPortableMarker, ManagementMode, detect_installation_with_context,
};
use std::fs;
use std::path::Path;

#[test]
fn linux_portable_marker_creation_and_validation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("SPLogbook");
    fs::create_dir_all(&app_dir).expect("create_dir_all");

    let marker = LinuxPortableMarker::new(
        "1.3.0-alpha.1",
        &app_dir,
        "a8541075e14de1537b7940fa3fd48b44d854151f",
        "7.0",
        "1.99.0",
    );

    assert_eq!(marker.marker_version, "1.0");
    assert_eq!(marker.installation_type, "LinuxPortableTarGz");
    assert_eq!(marker.management_mode, "SelfManaged");
    assert_eq!(marker.package_version, "1.3.0-alpha.1");
    assert_eq!(marker.target_os, "linux");
    assert_eq!(marker.target_arch, "x86_64");
    assert_eq!(marker.libc_family, "glibc");
    assert_eq!(marker.archive_format, "tar.gz");
    assert_eq!(marker.resolved_minimum_kernel, "7.0");
    assert_eq!(marker.build_rust_version, "1.99.0");
    assert_eq!(marker.created_by, "splogbook-packaging-m3-f");

    assert!(marker.validate().is_ok());

    // Serializacja i deserializacja
    let json = serde_json::to_string(&marker).expect("to_json");
    let marker_file = app_dir.join(LINUX_PORTABLE_MARKER_FILENAME);
    fs::write(&marker_file, &json).expect("write marker");

    let read_marker = LinuxPortableMarker::read_from_dir(&app_dir).expect("read_from_dir");
    assert_eq!(read_marker, marker);

    // Detekcja instalacji
    let ctx = InstallationDetectionContext::default();
    let detected = detect_installation_with_context(&app_dir, &ctx).expect("detect");
    assert_eq!(detected, InstallationType::LinuxPortableTarGz);
    assert_eq!(detected.management_mode(), ManagementMode::SelfManaged);
    assert!(detected.is_self_managed());
    assert!(!detected.is_system_managed());
}

#[test]
fn linux_portable_marker_corrupt_fields_fail_closed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("SPLogbook");
    fs::create_dir_all(&app_dir).expect("create_dir_all");

    // Zły installation_type
    let mut bad_marker = LinuxPortableMarker::new(
        "1.3.0-alpha.1",
        &app_dir,
        "a8541075e14de1537b7940fa3fd48b44d854151f",
        "7.0",
        "1.99.0",
    );
    bad_marker.installation_type = "WindowsPortableZip".to_string();
    assert!(bad_marker.validate().is_err());

    // Zły target_os
    bad_marker.installation_type = "LinuxPortableTarGz".to_string();
    bad_marker.target_os = "windows".to_string();
    assert!(bad_marker.validate().is_err());

    // Zła architektura (np. aarch64 out of scope)
    bad_marker.target_os = "linux".to_string();
    bad_marker.target_arch = "aarch64".to_string();
    assert!(bad_marker.validate().is_err());

    // Zła rodzina libc (np. musl out of scope)
    bad_marker.target_arch = "x86_64".to_string();
    bad_marker.libc_family = "musl".to_string();
    assert!(bad_marker.validate().is_err());

    // Zły format archiwum
    bad_marker.libc_family = "glibc".to_string();
    bad_marker.archive_format = "zip".to_string();
    assert!(bad_marker.validate().is_err());

    // Pusty resolved_minimum_kernel
    bad_marker.archive_format = "tar.gz".to_string();
    bad_marker.resolved_minimum_kernel = "  ".to_string();
    assert!(bad_marker.validate().is_err());

    // Uszkodzony plik JSON markera powoduje błąd w detect_installation_with_context (fail-closed)
    let marker_file = app_dir.join(LINUX_PORTABLE_MARKER_FILENAME);
    fs::write(&marker_file, b"{ corrupt json").expect("write corrupt");
    let ctx = InstallationDetectionContext::default();
    let res = detect_installation_with_context(&app_dir, &ctx);
    assert!(
        res.is_err(),
        "Uszkodzony marker Linux Portable musi zwracać fail-closed błąd"
    );
}

#[test]
fn linux_portable_desktop_template_safety() {
    let desktop_path = Path::new("installer/linux/portable/templates/splogbook.desktop");
    assert!(
        desktop_path.exists(),
        "Szablon splogbook.desktop musi istnieć"
    );

    let content = fs::read_to_string(desktop_path).expect("read desktop file");
    assert!(content.contains("[Desktop Entry]"));
    assert!(content.contains("Type=Application"));
    assert!(content.contains("Name=SPLogbook"));
    assert!(content.contains("Exec=SPLogbook"));
    assert!(content.contains("Icon=splogbook"));
    assert!(content.contains("StartupWMClass=SPLogbook"));

    // Zakaz odwołań do powłoki, skryptów sh i roota
    assert!(!content.contains("sh -c"));
    assert!(!content.contains("bash -c"));
    assert!(!content.contains("sudo"));
    assert!(!content.contains("pkexec"));
}

#[test]
fn linux_portable_build_script_safety() {
    let script_path = Path::new("scripts/build_linux_portable_tar_gz.sh");
    assert!(
        script_path.exists(),
        "Skrypt build_linux_portable_tar_gz.sh musi istnieć"
    );

    let script = fs::read_to_string(script_path).expect("read build script");
    assert!(
        script.contains("set -euo pipefail"),
        "Skrypt musi działać fail-closed"
    );
    assert!(
        script.contains("EUID"),
        "Skrypt musi sprawdzać EUID i blokować roota"
    );
    assert!(
        script.contains("SPLogbook-Linux-x86_64-"),
        "Skrypt musi tworzyć kanoniczną nazwę archiwum"
    );
    assert!(
        script.contains("SPLogbook"),
        "Skrypt musi tworzyć pojedynczy korzeń SPLogbook/"
    );
    assert!(
        script.contains(".splogbook-linux-portable-marker"),
        "Skrypt musi generować marker"
    );

    // Zakazy
    assert!(!script.contains("sudo "));
    assert!(!script.contains("pkexec "));
    assert!(!script.contains("tar -P"));
    assert!(!script.contains("--absolute-names"));
    assert!(!script.contains("--dereference"));
    assert!(!script.contains("apt "));
    assert!(!script.contains("dnf "));
    assert!(!script.contains("pacman "));
    assert!(!script.contains("zypper "));
}

#[test]
fn linux_portable_package_type_and_asset_matching() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use ed25519_dalek::Signer;
    use splogbook::cloud::updater::{
        ReleaseArchitecture, ReleaseAsset, ReleasePackageType, ReleasePlatform, TrustedKey,
        VerifiedReleaseManifest,
    };

    assert_eq!(
        ReleasePackageType::parse("tar.gz").unwrap(),
        ReleasePackageType::TarGz
    );
    assert_eq!(
        ReleasePackageType::parse("linux-portable-tar-gz").unwrap(),
        ReleasePackageType::TarGz
    );
    assert_eq!(
        ReleasePackageType::expected_for_installation(
            ReleasePlatform::Linux,
            InstallationType::LinuxPortableTarGz
        ),
        ReleasePackageType::TarGz
    );

    let manifest_json = r#"{
        "manifest_version": 1,
        "key_id": "test-m3f-key",
        "product": "SPLogbook",
        "version": "1.3.0-alpha.1",
        "channel": "alpha",
        "minimum_updater_version": "1.1.0",
        "commit": "a8541075e14de1537b7940fa3fd48b44d854151f",
        "assets": [
            {
                "platform": "linux",
                "arch": "x86_64",
                "package_type": "linux-portable-tar-gz",
                "filename": "SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz",
                "size": 52428800,
                "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            }
        ]
    }"#;

    let secret: [u8; 32] = rand::random();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
    let verifying_key = signing_key.verifying_key();
    let sig = signing_key.sign(manifest_json.as_bytes());
    let sig_b64 = BASE64.encode(sig.to_bytes());

    let tkey = TrustedKey {
        key_id: "test-m3f-key",
        public_key: verifying_key.to_bytes(),
        active: true,
        min_version: Some("1.2.0"),
        max_version: None,
    };

    let verified = VerifiedReleaseManifest::verify_with_keys_and_channel(
        manifest_json.as_bytes(),
        &sig_b64,
        &[tkey],
        &[],
        &[],
        "1.2.0",
        splogbook::core::build_info::UpdateChannelPreference::Alpha,
    )
    .expect("Manifest verification");

    let api_assets = vec![ReleaseAsset {
        name: "SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz".into(),
        browser_download_url: "https://example.com/asset.tar.gz".into(),
        digest: Some(
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        ),
        size: 52428800,
    }];

    let selected = verified
        .select_asset_for_installation_type(
            ReleasePlatform::Linux,
            ReleaseArchitecture::X86_64,
            InstallationType::LinuxPortableTarGz,
            &api_assets,
        )
        .expect("Select asset for Linux Portable TarGz");

    assert_eq!(
        selected.filename(),
        "SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz"
    );
    assert_eq!(
        selected.declared_sha256(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn linux_portable_disk_space_preflight() {
    use splogbook::cloud::updater::preflight::{
        DEFAULT_SAFETY_MARGIN_BYTES, DiskPreflightRequirements, TRANSACTION_METADATA_BYTES,
    };

    let req = DiskPreflightRequirements::calculate(
        InstallationType::LinuxPortableTarGz,
        50 * 1024 * 1024,  // 50 MiB download
        100 * 1024 * 1024, // 100 MiB extracted
        50 * 1024 * 1024,  // 50 MiB backup
        None,
    )
    .expect("Preflight calculate");

    assert_eq!(req.declared_download_size, 50 * 1024 * 1024);
    assert_eq!(req.max_extracted_size, 100 * 1024 * 1024);
    assert_eq!(req.backup_size, 50 * 1024 * 1024);
    assert_eq!(req.package_manager_overhead, 0);
    assert_eq!(req.transaction_metadata_size, TRANSACTION_METADATA_BYTES);
    assert_eq!(req.filesystem_safety_margin, DEFAULT_SAFETY_MARGIN_BYTES);
    assert_eq!(
        req.total_required_bytes,
        50 * 1024 * 1024
            + 100 * 1024 * 1024
            + 50 * 1024 * 1024
            + TRANSACTION_METADATA_BYTES
            + DEFAULT_SAFETY_MARGIN_BYTES
    );
}

#[test]
fn linux_portable_journaled_transactional_replacement_and_rollback() {
    use splogbook::cloud::updater::UpdatePlan;

    let temp = tempfile::tempdir().expect("tempdir");
    let active_dir = temp.path().join("active");
    let staging_dir = temp.path().join("staging");
    let rollback_dir = temp.path().join("rollback");

    fs::create_dir_all(&active_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();
    fs::create_dir_all(&rollback_dir).unwrap();

    // 1. Aktywne pliki aplikacji i dane użytkownika
    fs::write(active_dir.join("SPLogbook"), b"v1 binary").unwrap();
    fs::write(active_dir.join("splogbook-updater"), b"v1 updater").unwrap();
    fs::write(active_dir.join("default_log.db"), b"USER QSO DATABASE").unwrap();
    fs::write(active_dir.join("station_config.json"), b"USER CONFIG").unwrap();

    // 2. Pliki w stagingu (nowa wersja + próba nadpisania bazy/konfiguracji)
    fs::write(staging_dir.join("SPLogbook"), b"v2 binary").unwrap();
    fs::write(staging_dir.join("splogbook-updater"), b"v2 updater").unwrap();
    fs::write(staging_dir.join("default_log.db"), b"OVERWRITE ATTEMPT").unwrap();
    fs::write(
        staging_dir.join("station_config.json"),
        b"OVERWRITE ATTEMPT",
    )
    .unwrap();
    fs::write(staging_dir.join("README.md"), b"New doc").unwrap();

    // 3. Generowanie planu aktualizacji
    let plan = UpdatePlan::generate(&staging_dir, &active_dir).expect("UpdatePlan generate");
    plan.validate_safety("SPLogbook")
        .expect("UpdatePlan validate_safety");

    // Zabezpieczenie danych użytkownika: bazy i konfigi MUSZĄ być w preserve_files i NIE w replace_files
    let active_db = active_dir.join("default_log.db");
    let active_cfg = active_dir.join("station_config.json");
    let preserved: Vec<_> = plan
        .preserve_files
        .iter()
        .map(|(p, _)| p.as_path())
        .collect();
    assert!(preserved.contains(&active_db.as_path()));
    assert!(preserved.contains(&active_cfg.as_path()));

    let replaced: Vec<_> = plan
        .replace_files
        .iter()
        .map(|(p, _)| p.as_path())
        .collect();
    assert!(replaced.contains(&Path::new("SPLogbook")));
    assert!(replaced.contains(&Path::new("splogbook-updater")));
    assert!(!replaced.contains(&Path::new("default_log.db")));
    assert!(!replaced.contains(&Path::new("station_config.json")));

    // 4. Kopia zapasowa (.rollback)
    for (rel, orig) in &plan.replace_files {
        let dest = rollback_dir.join(rel);
        if let Some(p) = dest.parent() {
            let _ = fs::create_dir_all(p);
        }
        fs::copy(orig, &dest).unwrap();
    }
    assert_eq!(
        fs::read(rollback_dir.join("SPLogbook")).unwrap(),
        b"v1 binary"
    );

    // 5. Podmiana plików (JOURNALED_TRANSACTIONAL_REPLACEMENT)
    for (rel, orig) in &plan.replace_files {
        let src = staging_dir.join(rel);
        fs::copy(&src, orig).unwrap();
    }
    for (rel, orig) in &plan.create_files {
        let src = staging_dir.join(rel);
        fs::copy(&src, orig).unwrap();
    }
    assert_eq!(
        fs::read(active_dir.join("SPLogbook")).unwrap(),
        b"v2 binary"
    );
    assert_eq!(fs::read(active_dir.join("README.md")).unwrap(), b"New doc");

    // Bazy użytkownika nietknięte
    assert_eq!(
        fs::read(active_dir.join("default_log.db")).unwrap(),
        b"USER QSO DATABASE"
    );

    // 6. Symulowany Rollback (np. w razie awarii kandydata)
    for (_rel, orig) in &plan.create_files {
        let _ = fs::remove_file(orig);
    }
    for (rel, orig) in &plan.replace_files {
        let bkp = rollback_dir.join(rel);
        fs::copy(&bkp, orig).unwrap();
    }

    // Weryfikacja po rollbacku: przywrócone v1, usunięty README, nienaruszone dane
    assert_eq!(
        fs::read(active_dir.join("SPLogbook")).unwrap(),
        b"v1 binary"
    );
    assert!(!active_dir.join("README.md").exists());
    assert_eq!(
        fs::read(active_dir.join("default_log.db")).unwrap(),
        b"USER QSO DATABASE"
    );
    assert_eq!(
        fs::read(active_dir.join("station_config.json")).unwrap(),
        b"USER CONFIG"
    );
}
