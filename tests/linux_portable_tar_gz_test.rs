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
        "6.12",
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
    assert_eq!(marker.resolved_minimum_kernel, "6.12");
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
        "6.12",
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
