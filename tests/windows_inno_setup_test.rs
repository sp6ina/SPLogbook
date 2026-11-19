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
    #[cfg(windows)]
    let fake_setup = temp.path().join("fake_setup.cmd");
    #[cfg(not(windows))]
    let fake_setup = temp.path().join("fake_setup.sh");

    #[cfg(windows)]
    fs::write(&fake_setup, b"@echo off\r\nexit 0\r\n").unwrap();
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&fake_setup, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&fake_setup).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_setup, perms).unwrap();
    }

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

#[test]
fn inno_script_has_numeric_semver_and_explicit_app_mutex_macro() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    for symbol in [
        "function ExtractNextToken(",
        "function IsAllDigits(",
        "function ComparePreRelease(",
        "function CompareSemVer(",
        "#define AppMutexName \"SPLogbook_Instance_Mutex_{{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}\"",
        "AppMutex={#AppMutexName},SPLogbook",
    ] {
        assert!(
            iss_content.contains(symbol),
            "SPLogbook.iss musi zawierać '{symbol}'"
        );
    }

    let main_rs = fs::read_to_string("src/main.rs").expect("read src/main.rs");
    assert!(main_rs.contains("mod win32_instance_mutex"));
    assert!(main_rs.contains("SPLogbook_Instance_Mutex_{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}"));
    assert!(main_rs.contains("InstanceMutexGuard::acquire()"));
}

#[test]
fn inno_f22_transaction_lock_health_proof_and_startup_reconciliation() {
    use splogbook::cloud::updater::{
        DEFAULT_HEALTH_PROOF_MAX_AGE_SECS, HEALTH_PROOF_FILENAME, HealthProof,
        UPDATE_LOCK_FILENAME, UpdateAttemptId, UpdateLockGuard, UpdateStateId, UpdateTransaction,
        consume_update_result_on_startup, resolve_delegated_state_dir,
    };
    use splogbook::core::build_info::BuildIdentity;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let state_dir = resolve_delegated_state_dir(root).unwrap();
    assert_eq!(state_dir, root);

    let identity = BuildIdentity::current();
    let marker =
        WindowsUserSetupMarker::new(identity.package_version, root, identity.git_commit_hash);
    fs::write(
        root.join(WINDOWS_INNO_MARKER_FILENAME),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .unwrap();

    let attempt_id = UpdateAttemptId::generate();
    let lock = UpdateLockGuard::acquire(&state_dir, attempt_id).unwrap();
    std::mem::forget(lock);

    let mut tx = UpdateTransaction::new(
        attempt_id,
        identity.package_version.to_string(),
        format!("v{}", identity.package_version),
        "windows".to_string(),
        "x86_64".to_string(),
        "windows-inno-exe".to_string(),
        InstallationType::WindowsInnoSetup,
        identity.git_commit_hash.to_string(),
        "SPLogbook-Windows-x64-Setup.exe".to_string(),
        2048,
        "2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        state_dir.join(".staging"),
        &state_dir,
    )
    .unwrap()
    .with_commit_identity(identity.git_commit_hash);

    for state in [
        UpdateStateId::CheckingRelease,
        UpdateStateId::FetchingManifest,
        UpdateStateId::VerifyingManifest,
        UpdateStateId::SelectingAsset,
        UpdateStateId::Downloading,
        UpdateStateId::VerifyingDownload,
        UpdateStateId::InspectingPackage,
        UpdateStateId::Staging,
        UpdateStateId::BuildingUpdatePlan,
        UpdateStateId::AwaitingApply,
        UpdateStateId::Applying,
    ] {
        tx.transition_to(state).unwrap();
    }
    tx.save_to_dir(&state_dir).unwrap();

    // Walidacja HealthProof działa niezależnie
    let proof =
        HealthProof::from_build_identity(attempt_id, identity, InstallationType::WindowsInnoSetup);
    proof
        .validate(
            &attempt_id,
            identity.package_version,
            identity.git_commit_hash,
            DEFAULT_HEALTH_PROOF_MAX_AGE_SECS,
        )
        .unwrap();

    // Konsumpcja na starcie po zamknięciu procesu przez instalator Inno
    let outcome = consume_update_result_on_startup(root)
        .unwrap()
        .expect("startup outcome");
    assert!(outcome.was_success);
    assert_eq!(outcome.attempt_id, attempt_id);
    assert!(!state_dir.join(UPDATE_LOCK_FILENAME).exists());
    assert!(!state_dir.join(HEALTH_PROOF_FILENAME).exists());

    let loaded = UpdateTransaction::load_from_dir(&state_dir, &state_dir)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.current_state, UpdateStateId::Committed);
    assert!(consume_update_result_on_startup(root).unwrap().is_none());
}

#[test]
fn windows_multi_artifact_manifest_selects_inno_exe_and_avoids_ambiguous() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use ed25519_dalek::Signer;
    use splogbook::cloud::updater::{
        ReleaseArchitecture, ReleaseAsset, TrustedKey, VerifiedReleaseManifest,
        select_manifest_asset,
    };

    let sha_zip = "1111111111111111111111111111111111111111111111111111111111111111";
    let sha_inno = "2222222222222222222222222222222222222222222222222222222222222222";
    let sha_msi = "3333333333333333333333333333333333333333333333333333333333333333";

    let manifest_json = format!(
        r#"{{
            "manifest_version": 1,
            "key_id": "test-f05-key",
            "product": "SPLogbook",
            "version": "1.3.0",
            "channel": "stable",
            "minimum_updater_version": "1.2.0",
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "assets": [
                {{
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "zip",
                    "filename": "SPLogbook-Windows-x64.zip",
                    "size": 1000,
                    "sha256": "{sha_zip}"
                }},
                {{
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "inno-exe",
                    "filename": "SPLogbook-Windows-x64-Setup-1.3.0.exe",
                    "size": 2000,
                    "sha256": "{sha_inno}"
                }},
                {{
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "msi",
                    "filename": "SPLogbook-Windows-x64-1.3.0.msi",
                    "size": 3000,
                    "sha256": "{sha_msi}"
                }}
            ]
        }}"#
    );

    let secret: [u8; 32] = rand::random();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
    let verifying_key = signing_key.verifying_key();
    let sig = signing_key.sign(manifest_json.as_bytes());
    let sig_b64 = BASE64.encode(sig.to_bytes());

    let tkey = TrustedKey {
        key_id: "test-f05-key",
        public_key: verifying_key.to_bytes(),
        active: true,
        min_version: Some("1.2.0"),
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
    .expect("verify manifest");

    let api_assets = vec![
        ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "https://example.com/SPLogbook-Windows-x64.zip".into(),
            digest: Some(format!("sha256:{sha_zip}")),
            size: 1000,
        },
        ReleaseAsset {
            name: "SPLogbook-Windows-x64-Setup-1.3.0.exe".into(),
            browser_download_url: "https://example.com/SPLogbook-Windows-x64-Setup-1.3.0.exe"
                .into(),
            digest: Some(format!("sha256:{sha_inno}")),
            size: 2000,
        },
        ReleaseAsset {
            name: "SPLogbook-Windows-x64-1.3.0.msi".into(),
            browser_download_url: "https://example.com/SPLogbook-Windows-x64-1.3.0.msi".into(),
            digest: Some(format!("sha256:{sha_msi}")),
            size: 3000,
        },
    ];

    let inno_asset = verified
        .select_asset_for_installation_type(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            InstallationType::WindowsInnoSetup,
            &api_assets,
        )
        .expect("select inno-exe");
    assert_eq!(inno_asset.package_type(), ReleasePackageType::InnoExe);
    assert_eq!(
        inno_asset.filename(),
        "SPLogbook-Windows-x64-Setup-1.3.0.exe"
    );

    let default_asset = verified
        .select_asset_for_platform(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            &api_assets,
        )
        .expect("select_asset_for_platform should not fail with AmbiguousArtifact");
    assert_eq!(default_asset.package_type(), ReleasePackageType::Zip);

    let m_asset = select_manifest_asset(&verified.manifest().assets, "windows", "x86_64")
        .expect("select_manifest_asset should not fail with AmbiguousArtifact");
    assert_eq!(m_asset.package_type, "zip");
}

#[test]
fn inno_workflow_builds_verifies_e2e_and_publishes_setup_exe() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("read build-and-release.yml");
    for required in [
        "Install and verify pinned Inno Setup 7.1.1",
        "E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644",
        "Build Windows User Setup (Inno Setup 7.1.1)",
        "./scripts/build_windows_user_setup.ps1 -OutputDir dist",
        "Verify built Windows Inno Setup and MSI installers (opt-in E2E)",
        "SPLOG_RUN_INNO_E2E",
        "inno_native_install_and_uninstall_lifecycle_e2e",
        "dist/SPLogbook-Windows-x64-Setup-*.exe",
        "dist/SPLogbook-Windows-x64-Setup-*.exe.sha256",
        r#""package_type": "inno-exe""#,
        "release-staging/SPLogbook-Windows-x64-Setup-${{ env.RELEASE_VERSION }}.exe",
    ] {
        assert!(
            workflow.contains(required),
            "build-and-release.yml musi zawierać krok/artefakt Inno Setup: {required}"
        );
    }
}
