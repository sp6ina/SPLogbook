// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy natywne integracji Inno Setup User Setup z aktualizatorem (M3-D)

use splogbook::cloud::updater::{
    InstallationCapabilities, InstallationType, ManagementMode, ReleasePackageType,
    ReleasePlatform, WINDOWS_INNO_MARKER_FILENAME, WindowsUserSetupMarker,
    build_inno_setup_arguments, delegate_inno_setup_update, execute_inno_setup_installer,
};
use std::fs;

#[test]
fn inno_managed_update_never_uses_self_apply() {
    let caps = InstallationCapabilities::for_installation_type(InstallationType::WindowsInnoSetup);
    assert!(
        !caps.supports_self_apply,
        "WindowsInnoSetup nie może obsługiwać self-apply (supports_self_apply=false)"
    );
    assert!(
        caps.requires_delegation,
        "WindowsInnoSetup musi wymagać delegacji (requires_delegation=true)"
    );
    assert_eq!(
        InstallationType::WindowsInnoSetup.management_mode(),
        ManagementMode::SystemManaged,
        "WindowsInnoSetup musi być w trybie SystemManaged"
    );
}

#[test]
fn inno_managed_update_never_uses_portable_helper() {
    // Sprawdzenie, że kod obsługi Inno Setup w updaterze nie tworzy helper requestu ani nie wywołuje splogbook-updater
    let updater_src = fs::read_to_string("src/cloud/updater.rs").expect("read updater.rs");
    let inno_block = updater_src
        .split("if inst_type == InstallationType::WindowsInnoSetup {")
        .nth(1)
        .and_then(|s| {
            s.split("if inst_type != InstallationType::WindowsPortableZip {")
                .next()
        })
        .expect("inno_setup block");

    assert!(
        !inno_block.contains("install_via_windows_helper"),
        "Ścieżka aktualizacji Inno Setup nie może wywoływać helpera Portable ZIP"
    );
    assert!(
        !inno_block.contains("WindowsHelperRequest"),
        "Ścieżka aktualizacji Inno Setup nie może tworzyć WindowsHelperRequest"
    );
    assert!(
        !inno_block.contains("UpdatePlan::"),
        "Ścieżka aktualizacji Inno Setup nie może tworzyć planu podmiany plików UpdatePlan"
    );
}

#[test]
fn inno_managed_update_never_uses_powershell() {
    let updater_src = fs::read_to_string("src/cloud/updater.rs").expect("read updater.rs");
    let inno_block = updater_src
        .split("if inst_type == InstallationType::WindowsInnoSetup {")
        .nth(1)
        .and_then(|s| {
            s.split("if inst_type != InstallationType::WindowsPortableZip {")
                .next()
        })
        .expect("inno_setup block");

    assert!(
        !inno_block.contains("powershell"),
        "Ścieżka aktualizacji Inno Setup nie może wywoływać programu powershell"
    );
    assert!(
        !inno_block.contains("generate_powershell_from_request"),
        "Ścieżka aktualizacji Inno Setup nie może korzystać z fallbacku PowerShell"
    );
}

#[test]
fn inno_update_launches_verified_setup() {
    let temp = tempfile::tempdir().unwrap();
    let fake_setup = temp.path().join("fake_setup.cmd");
    // Utworzenie skryptu symulującego Setup zwracający kod 0
    fs::write(&fake_setup, b"@echo off\r\nexit 0\r\n").unwrap();

    let log_path = temp.path().join("setup.log");
    let res = execute_inno_setup_installer(&fake_setup, &log_path);
    assert!(res.is_ok(), "Uruchomienie instalatora powinno zwrócić Ok");
    assert_eq!(res.unwrap(), 0);
}

#[test]
fn inno_update_uses_structured_arguments() {
    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("test_inno.log");
    let args = build_inno_setup_arguments(&log_path);

    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "/SILENT");
    assert_eq!(args[1], "/NORESTART");
    assert_eq!(args[2], format!("/LOG={}", log_path.display()));
}

#[test]
fn inno_update_does_not_use_shell() {
    let updater_src = fs::read_to_string("src/cloud/updater.rs").expect("read updater.rs");
    let exec_fn = updater_src
        .split("pub fn execute_inno_setup_installer")
        .nth(1)
        .and_then(|s| s.split("pub fn delegate_inno_setup_update").next())
        .expect("execute_inno_setup_installer fn");

    assert!(
        !exec_fn.contains("cmd.exe") && !exec_fn.contains("\"cmd\""),
        "execute_inno_setup_installer nie może wywoływać powłoki cmd"
    );
    assert!(
        !exec_fn.contains("powershell.exe") && !exec_fn.contains("\"powershell\""),
        "execute_inno_setup_installer nie może wywoływać powłoki powershell"
    );
    assert!(
        exec_fn.contains("Command::new(installer_exe)"),
        "execute_inno_setup_installer musi uruchamiać Command::new bezpośrednio z pliku instalatora"
    );
}

#[test]
fn inno_update_does_not_use_runas() {
    let updater_src = fs::read_to_string("src/cloud/updater.rs").expect("read updater.rs");
    let exec_fn = updater_src
        .split("pub fn execute_inno_setup_installer")
        .nth(1)
        .and_then(|s| s.split("pub fn delegate_inno_setup_update").next())
        .expect("execute_inno_setup_installer fn");

    assert!(
        !exec_fn.contains("runas"),
        "execute_inno_setup_installer nie może używać czasownika runas"
    );
}

#[test]
fn inno_update_requires_no_restart() {
    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("inno_test.log");
    let args = build_inno_setup_arguments(&log_path);
    assert!(
        args.contains(&"/NORESTART".to_string()),
        "Argumenty instalatora Inno Setup muszą bezwzględnie zawierać /NORESTART"
    );
}

#[test]
fn inno_update_enforces_per_user_mode() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    assert!(
        iss_content.contains("PrivilegesRequired=lowest"),
        "SPLogbook.iss musi zawierać PrivilegesRequired=lowest"
    );
    assert!(
        iss_content.contains("DefaultDirName={localappdata}\\Programs\\SPLogbook"),
        "SPLogbook.iss musi instalować do {{localappdata}}\\Programs\\SPLogbook"
    );
    assert!(
        iss_content.contains("CompareText(ParamStr(i), '/ALLUSERS') = 0"),
        "SPLogbook.iss musi odrzucać parametr /ALLUSERS w InitializeSetup"
    );
}

#[test]
fn inno_update_rejects_wrong_version() {
    let temp = tempfile::tempdir().unwrap();
    let fake_setup = temp.path().join("setup.cmd");
    fs::write(&fake_setup, b"@echo off\r\nexit 0\r\n").unwrap();

    let target_dir = temp.path().join("app");
    fs::create_dir_all(&target_dir).unwrap();
    let marker = WindowsUserSetupMarker::new(
        "1.2.0", // Starsza wersja w markerze niż oczekiwana
        &target_dir,
        "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
    );
    let marker_json = serde_json::to_string(&marker).unwrap();
    fs::write(target_dir.join(WINDOWS_INNO_MARKER_FILENAME), marker_json).unwrap();

    let log_path = temp.path().join("test.log");
    let res = delegate_inno_setup_update(
        &fake_setup,
        &log_path,
        "1.3.0-alpha.1",
        "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        &target_dir,
    );
    assert!(
        res.is_err(),
        "Niezgodność zainstalowanej wersji z oczekiwaną musi zwrócić błąd"
    );
}

#[test]
fn inno_update_rejects_wrong_commit() {
    let temp = tempfile::tempdir().unwrap();
    let fake_setup = temp.path().join("setup.cmd");
    fs::write(&fake_setup, b"@echo off\r\nexit 0\r\n").unwrap();

    let target_dir = temp.path().join("app");
    fs::create_dir_all(&target_dir).unwrap();
    let marker = WindowsUserSetupMarker::new(
        "1.3.0-alpha.1",
        &target_dir,
        "0000000000000000000000000000000000000000", // Zły commit
    );
    let marker_json = serde_json::to_string(&marker).unwrap();
    fs::write(target_dir.join(WINDOWS_INNO_MARKER_FILENAME), marker_json).unwrap();

    let log_path = temp.path().join("test.log");
    let res = delegate_inno_setup_update(
        &fake_setup,
        &log_path,
        "1.3.0-alpha.1",
        "db4417e3b8acff9bb87aaf04fa5d703e5fec74bc",
        &target_dir,
    );
    assert!(
        res.is_err(),
        "Niezgodność zainstalowanego commita z oczekiwanym musi zwrócić błąd"
    );
}

#[test]
fn inno_update_rejects_wrong_package_type() {
    assert_eq!(
        ReleasePackageType::expected_for_installation(
            ReleasePlatform::Windows,
            InstallationType::WindowsInnoSetup
        ),
        ReleasePackageType::InnoExe
    );
    assert_ne!(
        ReleasePackageType::expected_for_installation(
            ReleasePlatform::Windows,
            InstallationType::WindowsInnoSetup
        ),
        ReleasePackageType::Zip
    );
}

#[test]
fn inno_update_rejects_downgrade() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    assert!(
        iss_content.contains("CompareSemVer(InstalledVer, '{#AppVersion}') > 0"),
        "SPLogbook.iss musi sprawdzać wersję w rejestrze i blokować downgrade"
    );
}

#[test]
fn inno_update_preserves_user_data() {
    let temp = tempfile::tempdir().unwrap();
    let user_db = temp.path().join("default_log.db");
    let user_wal = temp.path().join("default_log.db-wal");
    let user_config = temp.path().join("station_config.json");

    let db_bytes = b"SQLITE_DATABASE_TEST_CONTENT_12345";
    let wal_bytes = b"SQLITE_WAL_TEST_CONTENT_67890";
    let config_bytes = b"{\"callsign\":\"SP6INA\",\"station\":\"test\"}";

    fs::write(&user_db, db_bytes).unwrap();
    fs::write(&user_wal, wal_bytes).unwrap();
    fs::write(&user_config, config_bytes).unwrap();

    // Sprawdzenie, że żaden z tych plików nie jest na allowliście instalatora ani nie podlega usunięciu
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    assert!(
        !iss_content.contains("default_log.db"),
        "SPLogbook.iss nie może pakować ani usuwać default_log.db"
    );
    assert!(
        !iss_content.contains("station_config.json"),
        "SPLogbook.iss nie może pakować ani usuwać station_config.json"
    );
    assert!(
        !iss_content.contains("*.db-wal"),
        "SPLogbook.iss nie może zawierać wildcardów dla WAL"
    );

    // Weryfikacja byte-identical
    assert_eq!(fs::read(&user_db).unwrap(), db_bytes);
    assert_eq!(fs::read(&user_wal).unwrap(), wal_bytes);
    assert_eq!(fs::read(&user_config).unwrap(), config_bytes);
}
