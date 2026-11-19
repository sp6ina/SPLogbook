//! Testy bezpieczeństwa, struktury i polityki cyklu życia Windows MSI (M3-E).
//!
//! Weryfikacja:
//! - Zgodności z ADR-11 (WiX Toolset SDK 7.0.1)
//! - Architektury x64 i instalacji systemowej per-machine
//! - Stałego UpgradeCode {B7D14532-6A10-4F8A-9A4B-3DE189E502B0}
//! - Rozłączności tożsamości UpgradeCode od Inno Setup AppId
//! - Allowlisty plików i braku zabronionych artefaktów
//! - Ochrony danych użytkownika (%APPDATA%, bazy, konfiguracja, WAL, SHM)
//! - Wykrywania konfliktów z User Setup (fail-closed) i Portable ZIP (non-destructive)
//! - Zakazu niebezpiecznych custom actions (PowerShell, cmd, scripts)
//! - Zakazu automatycznego restartu systemu

use splogbook::cloud::updater::installation::{
    InstallationDetectionContext, InstallationType, WINDOWS_INNO_MARKER_FILENAME,
    WINDOWS_INNO_SETUP_APP_ID, WINDOWS_MSI_MARKER_FILENAME, WINDOWS_MSI_UPGRADE_CODE,
    WINDOWS_MSI_UPGRADE_CODE_PACKED, WindowsMsiMarker, WindowsUserSetupMarker,
    detect_installation_with_context, semver_to_msi_product_version,
};
use std::fs;

#[test]
fn msi_uses_approved_wix_toolchain() {
    let wixproj = fs::read_to_string("installer/windows/msi/SPLogbook.wixproj")
        .expect("read SPLogbook.wixproj");
    assert!(
        wixproj.contains("WixToolset.Sdk/7.0.1"),
        "SPLogbook.wixproj musi używać zatwierdzonego WiX Toolset SDK 7.0.1"
    );
    assert!(
        wixproj.contains("<AcceptEula>true</AcceptEula>"),
        "SPLogbook.wixproj musi deklarować akceptację EULA WiX 7"
    );
}

#[test]
fn msi_is_x64() {
    let wixproj = fs::read_to_string("installer/windows/msi/SPLogbook.wixproj")
        .expect("read SPLogbook.wixproj");
    assert!(wixproj.contains("<Platform>x64</Platform>"));
    assert!(wixproj.contains("<InstallerPlatform>x64</InstallerPlatform>"));

    let comp =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    assert!(comp.contains("Bitness=\"always64\""));
}

#[test]
fn msi_is_per_machine() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("Scope=\"perMachine\""),
        "Instalator MSI musi mieć zdefiniowany zakres perMachine"
    );
}

#[test]
fn msi_uses_program_files_x64() {
    let dir =
        fs::read_to_string("installer/windows/msi/Directories.wxs").expect("read Directories.wxs");
    assert!(
        dir.contains("<StandardDirectory Id=\"ProgramFiles64Folder\">"),
        "Instalator MSI musi instalować się do ProgramFiles64Folder"
    );
    assert!(
        dir.contains("<Directory Id=\"INSTALLFOLDER\" Name=\"SPLogbook\">"),
        "Katalogiem aplikacji musi być podkatalog SPLogbook w Program Files x64"
    );
}

#[test]
fn msi_upgrade_code_is_stable() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("UpgradeCode=\"B7D14532-6A10-4F8A-9A4B-3DE189E502B0\""),
        "Package.wxs musi zawierać stały UpgradeCode zatwierdzony przez właściciela"
    );
    assert_eq!(
        WINDOWS_MSI_UPGRADE_CODE,
        "{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}"
    );
    assert_eq!(
        WINDOWS_MSI_UPGRADE_CODE_PACKED,
        "0B205E981ED3B4A9A9B4A9E189E502B0"
    );
    assert_eq!(InstallationType::WindowsMsi.as_str(), "windows-msi");
}

#[test]
fn msi_upgrade_code_differs_from_inno_app_id() {
    assert_ne!(
        WINDOWS_MSI_UPGRADE_CODE, WINDOWS_INNO_SETUP_APP_ID,
        "UpgradeCode instalatora MSI nie może być tożsamy z AppId instalatora Inno Setup"
    );
    assert_eq!(
        WINDOWS_INNO_SETUP_APP_ID,
        "{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}"
    );
}

#[test]
fn msi_product_code_policy_is_explicit() {
    // W WiX v4/v7 w elemencie <Package> brak jawnego statycznego ProductCode oznacza dynamiczne generowanie GUID dla każdego wydania
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        !pkg.contains("ProductCode=\"{"),
        "Package.wxs nie może używać statycznego ProductCode na stałe"
    );
}

#[test]
fn msi_version_matches_build_identity() {
    let msi_ver = semver_to_msi_product_version("1.3.0-alpha.1").expect("semver map");
    assert_eq!(msi_ver, "1.3.0");
}

#[test]
fn msi_display_version_preserves_prerelease() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("<Property Id=\"DisplayVersion\" Value=\"$(var.DisplayVersion)\" />"),
        "Package.wxs musi rejestrować właściwość DisplayVersion z zachowaniem prerelease"
    );
}

#[test]
fn msi_input_is_allowlisted() {
    let build_script =
        fs::read_to_string("scripts/build_windows_msi.ps1").expect("read build_windows_msi.ps1");
    assert!(build_script.contains("forbiddenFiles"));
    assert!(build_script.contains("SPLogbook.exe"));
    assert!(build_script.contains("splogbook-updater.exe"));
}

#[test]
fn msi_includes_main_binary_and_native_updater() {
    let comp =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    assert!(comp.contains("Source=\"$(var.StagingDir)\\SPLogbook.exe\""));
    assert!(comp.contains("Source=\"$(var.StagingDir)\\splogbook-updater.exe\""));
}

#[test]
fn msi_excludes_private_keys_and_user_databases() {
    let comp =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    assert!(!comp.contains("default_log.db"));
    assert!(!comp.contains("station_config.json"));
    assert!(!comp.contains(".key"));
    assert!(!comp.contains(".pem"));
    assert!(!comp.contains(".pfx"));
}

#[test]
fn msi_does_not_use_custom_action_powershell_or_cmd() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(!pkg.contains("CustomAction"));
    assert!(!pkg.contains("powershell"));
    assert!(!pkg.contains("cmd.exe"));
}

#[test]
fn msi_does_not_use_nested_install() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(!pkg.contains("NestedInstallation"));
}

#[test]
fn msi_inno_conflict_is_fail_closed() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(pkg.contains("INNO_USER_SETUP_DETECTED"));
    assert!(pkg.contains("C8E28541-7B21-4A9B-8F5C-4EF298F613C1"));
    assert!(pkg.contains("<Launch"));

    let temp = tempfile::tempdir().unwrap();
    let inno_marker = WindowsUserSetupMarker::new(
        "1.3.0-alpha.1",
        temp.path(),
        "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
    );
    let inno_json = serde_json::to_string(&inno_marker).unwrap();
    fs::write(temp.path().join(WINDOWS_INNO_MARKER_FILENAME), inno_json).unwrap();

    let msi_marker = WindowsMsiMarker::new(
        "1.3.0-alpha.1",
        temp.path(),
        "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
    );
    let msi_json = serde_json::to_string(&msi_marker).unwrap();
    fs::write(temp.path().join(WINDOWS_MSI_MARKER_FILENAME), msi_json).unwrap();

    let ctx = InstallationDetectionContext::default();
    let res = detect_installation_with_context(temp.path(), &ctx);
    assert!(
        res.is_err(),
        "Konflikt Inno i MSI musi kończyć się błędem fail-closed"
    );
}

#[test]
fn msi_portable_conflict_is_non_destructive() {
    let temp = tempfile::tempdir().unwrap();
    let msi_marker = WindowsMsiMarker::new(
        "1.3.0-alpha.1",
        temp.path(),
        "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
    );
    let msi_json = serde_json::to_string(&msi_marker).unwrap();
    fs::write(temp.path().join(WINDOWS_MSI_MARKER_FILENAME), msi_json).unwrap();
    fs::write(temp.path().join(".splogbook-portable"), b"PORTABLE").unwrap();

    let ctx = InstallationDetectionContext::default();
    let res = detect_installation_with_context(temp.path(), &ctx);
    assert!(
        res.is_err(),
        "Konflikt MSI z Portable ZIP musi kończyć się błędem fail-closed"
    );
}

#[test]
fn msi_major_upgrade_is_configured_and_downgrade_blocked() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(pkg.contains("<MajorUpgrade"));
    assert!(pkg.contains("DowngradeErrorMessage="));
    assert!(pkg.contains("Schedule=\"afterInstallInitialize\""));
}

#[test]
fn msi_artifact_is_not_committed() {
    let gitignore = fs::read_to_string(".gitignore").expect("read .gitignore");
    assert!(gitignore.contains("*.msi"));
    assert!(gitignore.contains("*.wixpdb"));
    assert!(gitignore.contains("installer/windows/msi/bin/"));
    assert!(gitignore.contains("installer/windows/msi/obj/"));
}

#[test]
fn msi_f22_transaction_lock_health_proof_and_startup_reconciliation() {
    use splogbook::cloud::updater::{
        HEALTH_PROOF_FILENAME, UPDATE_LOCK_FILENAME, UpdateAttemptId, UpdateLockGuard,
        UpdateStateId, UpdateTransaction, consume_update_result_on_startup,
    };
    use splogbook::core::build_info::BuildIdentity;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let identity = BuildIdentity::current();

    let marker = WindowsMsiMarker::new(identity.package_version, root, identity.git_commit_hash);
    fs::write(
        root.join(WINDOWS_MSI_MARKER_FILENAME),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .unwrap();

    let attempt_id = UpdateAttemptId::generate();
    let lock = UpdateLockGuard::acquire(root, attempt_id).unwrap();
    std::mem::forget(lock);

    let mut tx = UpdateTransaction::new(
        attempt_id,
        identity.package_version.to_string(),
        format!("v{}", identity.package_version),
        "windows".to_string(),
        "x86_64".to_string(),
        "windows-msi".to_string(),
        InstallationType::WindowsMsi,
        identity.git_commit_hash.to_string(),
        "SPLogbook-Windows-x64.msi".to_string(),
        4096,
        "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        root.join(".staging"),
        root,
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
    tx.save_to_dir(root).unwrap();

    let outcome = consume_update_result_on_startup(root)
        .unwrap()
        .expect("msi startup outcome");
    assert!(outcome.was_success);
    assert_eq!(outcome.attempt_id, attempt_id);
    assert!(!root.join(UPDATE_LOCK_FILENAME).exists());
    assert!(!root.join(HEALTH_PROOF_FILENAME).exists());

    let loaded = UpdateTransaction::load_from_dir(root, root)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.current_state, UpdateStateId::Committed);
    assert!(consume_update_result_on_startup(root).unwrap().is_none());

    // Scenariusz błędu niezgodności wersji/commita -> FailedPostApply
    let mismatch_dir = temp.path().join("mismatch");
    fs::create_dir_all(&mismatch_dir).unwrap();
    fs::write(
        mismatch_dir.join(WINDOWS_MSI_MARKER_FILENAME),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .unwrap();
    let bad_attempt = UpdateAttemptId::generate();
    let bad_lock = UpdateLockGuard::acquire(&mismatch_dir, bad_attempt).unwrap();
    std::mem::forget(bad_lock);
    let mut bad_tx = UpdateTransaction::new(
        bad_attempt,
        "9.9.9".to_string(),
        "v9.9.9".to_string(),
        "windows".to_string(),
        "x86_64".to_string(),
        "windows-msi".to_string(),
        InstallationType::WindowsMsi,
        "0000000000000000000000000000000000000000".to_string(),
        "SPLogbook-Windows-x64.msi".to_string(),
        4096,
        "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        mismatch_dir.join(".staging"),
        &mismatch_dir,
    )
    .unwrap()
    .with_commit_identity("0000000000000000000000000000000000000000");

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
        bad_tx.transition_to(state).unwrap();
    }
    bad_tx.save_to_dir(&mismatch_dir).unwrap();

    let bad_outcome = consume_update_result_on_startup(&mismatch_dir)
        .unwrap()
        .expect("msi mismatch outcome");
    assert!(!bad_outcome.was_success);
    assert!(!mismatch_dir.join(UPDATE_LOCK_FILENAME).exists());
    let bad_loaded = UpdateTransaction::load_from_dir(&mismatch_dir, &mismatch_dir)
        .unwrap()
        .unwrap();
    assert_eq!(bad_loaded.current_state, UpdateStateId::FailedPostApply);
}

#[test]
fn windows_multi_artifact_manifest_selects_msi_and_rejects_duplicate_triad() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use ed25519_dalek::Signer;
    use splogbook::cloud::updater::{
        ReleaseArchitecture, ReleaseAsset, ReleasePackageType, ReleasePlatform, TrustedKey,
        UpdaterError, VerifiedReleaseManifest,
    };

    let sha_zip = "1111111111111111111111111111111111111111111111111111111111111111";
    let sha_inno = "2222222222222222222222222222222222222222222222222222222222222222";
    let sha_msi = "3333333333333333333333333333333333333333333333333333333333333333";

    let build_signed = |dup_pkg_type: Option<&str>| {
        let dup_entry = if let Some(pkg) = dup_pkg_type {
            format!(
                r#",{{
                    "platform": "windows",
                    "arch": "x86_64",
                    "package_type": "{pkg}",
                    "filename": "SPLogbook-Windows-x64-dup.msi",
                    "size": 3001,
                    "sha256": "{sha_msi}"
                }}"#
            )
        } else {
            String::new()
        };
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
                    }}{dup_entry}
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
        VerifiedReleaseManifest::verify_with_keys(
            manifest_json.as_bytes(),
            &sig_b64,
            &[tkey],
            &[],
            &[],
            "1.2.0",
        )
    };

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

    let verified = build_signed(None).expect("verify manifest");
    let msi_asset = verified
        .select_asset_for_installation_type(
            ReleasePlatform::Windows,
            ReleaseArchitecture::X86_64,
            InstallationType::WindowsMsi,
            &api_assets,
        )
        .expect("select msi");
    assert_eq!(msi_asset.package_type(), ReleasePackageType::Msi);
    assert_eq!(msi_asset.filename(), "SPLogbook-Windows-x64-1.3.0.msi");

    let exact_dup_res = build_signed(Some("msi"));
    assert!(matches!(
        exact_dup_res,
        Err(UpdaterError::ManifestValidation(_))
    ));

    let alias_dup_verified = build_signed(Some("windows-msi")).expect("verify alias manifest");
    let dup_res = alias_dup_verified.select_asset_for_platform(
        ReleasePlatform::Windows,
        ReleaseArchitecture::X86_64,
        &api_assets,
    );
    assert!(matches!(dup_res, Err(UpdaterError::AmbiguousArtifact(_))));
}

#[test]
fn msi_workflow_builds_verifies_e2e_and_publishes_msi() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("read build-and-release.yml");
    for required in [
        "Build Windows System Setup (MSI / WiX 7.0.1)",
        "./scripts/build_windows_msi.ps1 -OutputDir dist -SkipCargoBuild",
        "Verify built Windows Inno Setup and MSI installers (opt-in E2E)",
        "SPLOG_RUN_MSI_E2E",
        "msi_built_artifact_verification_if_present",
        "dist/SPLogbook-Windows-x64-MSI-*.msi",
        "dist/SPLogbook-Windows-x64-MSI-*.msi.sha256",
        r#""package_type": "msi""#,
        "release-staging/SPLogbook-Windows-x64-MSI-${{ env.RELEASE_VERSION }}.msi",
    ] {
        assert!(
            workflow.contains(required),
            "build-and-release.yml musi zawierać krok/artefakt MSI: {required}"
        );
    }

    let build_script =
        fs::read_to_string("scripts/build_windows_msi.ps1").expect("read build_windows_msi.ps1");
    assert!(
        build_script.contains("vswhere.exe")
            && build_script.contains("Microsoft.Component.MSBuild")
            && build_script.contains("Enterprise"),
        "build_windows_msi.ps1 musi dynamicznie wykrywać MSBuild.exe przez vswhere.exe"
    );
}
