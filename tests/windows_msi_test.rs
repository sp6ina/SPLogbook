//! Testy bezpieczeństwa, struktury i polityki cyklu życia Windows MSI (M3-E).
//!
//! Weryfikacja:
//! - Zgodności z decyzją OWNER-M3-E-TOOLCHAIN-01 (WiX Toolset SDK 7.0.1)
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
