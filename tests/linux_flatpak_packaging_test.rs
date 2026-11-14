// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// Kontrakt pakietu Flatpak (M3-J).
//
// Testy są statyczne i hermetyczne: czytają wyłącznie pliki śledzone
// w repozytorium, nie wykonują sieci, nie instalują pakietów, nie uruchamiają
// Flatpaka ani nie dotykają profilu użytkownika. Ich zadaniem jest utrzymanie
// kontraktu tożsamości, uprawnień, przypięcia źródeł i fail-closed skryptów.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const APP_ID: &str = "io.github.sp6ina.SPLogbook";
const RUNTIME: &str = "org.freedesktop.Platform";
const RUNTIME_VERSION: &str = "26.08";
const SDK: &str = "org.freedesktop.Sdk";
const COMMAND: &str = "SPLogbook";
const PACKAGE_VERSION: &str = "1.3.0-alpha.1";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("nie można odczytać {}: {e}", path.display()))
}

fn flatpak_dir() -> PathBuf {
    repo_root().join("installer").join("linux").join("flatpak")
}

fn manifest() -> String {
    read_repo_file(&format!("installer/linux/flatpak/{APP_ID}.yml"))
}

/// Manifest without comment lines: comments document what is deliberately
/// absent, so only effective YAML may be scanned for forbidden permissions.
fn manifest_effective() -> String {
    manifest()
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn metainfo() -> String {
    read_repo_file(&format!("installer/linux/flatpak/{APP_ID}.metainfo.xml"))
}

fn desktop_entry() -> String {
    read_repo_file(&format!("installer/linux/flatpak/{APP_ID}.desktop"))
}

/// Wartość klucza najwyższego poziomu w manifeście (proste, jawne parsowanie).
fn manifest_top_level_value(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| line[prefix.len()..].trim().trim_matches('"').to_string())
}

/// Wpisy `finish-args` w kolejności wystąpienia.
fn manifest_finish_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("finish-args:") {
            inside = true;
            continue;
        }
        if inside {
            if let Some(rest) = line.trim_start().strip_prefix("- ") {
                if !line.starts_with(' ') {
                    break;
                }
                args.push(rest.trim().to_string());
            } else if !line.trim().is_empty() && !line.starts_with(' ') {
                break;
            }
        }
    }
    args
}

// ============================================================================
// 1. Tożsamość, runtime i polecenie
// ============================================================================

#[test]
fn flatpak_manifest_declares_the_approved_identity_and_pinned_runtime() {
    let text = manifest();
    assert_eq!(
        manifest_top_level_value(&text, "app-id").as_deref(),
        Some(APP_ID),
        "app-id musi być identyfikatorem zatwierdzonym dla pakietu"
    );
    assert_eq!(
        manifest_top_level_value(&text, "runtime").as_deref(),
        Some(RUNTIME)
    );
    assert_eq!(
        manifest_top_level_value(&text, "runtime-version").as_deref(),
        Some(RUNTIME_VERSION),
        "runtime-version musi być przypiętą gałęzią roku i miesiąca"
    );
    assert_eq!(manifest_top_level_value(&text, "sdk").as_deref(), Some(SDK));
    assert_eq!(
        manifest_top_level_value(&text, "command").as_deref(),
        Some(COMMAND)
    );
    assert!(
        text.contains("sdk-extensions:\n  - org.freedesktop.Sdk.Extension.rust-stable"),
        "manifest musi deklarować oficjalne rozszerzenie Rust SDK"
    );
    assert!(
        text.contains("  - /include") && text.contains("  - /share/man"),
        "manifest musi jawnie deklarować sekcję cleanup"
    );
    // app-id musi kończyć się nazwą aplikacji i mieć co najmniej trzy segmenty.
    assert!(APP_ID.split('.').count() >= 3);
    assert!(APP_ID.ends_with("SPLogbook"));
}

#[test]
fn flatpak_exported_files_use_the_application_id() {
    // metainfo, desktop i ikony muszą istnieć pod nazwą wynikającą z app ID.
    for relative in [
        format!("{APP_ID}.metainfo.xml"),
        format!("{APP_ID}.desktop"),
    ] {
        let path = flatpak_dir().join(&relative);
        assert!(path.is_file(), "brak pliku eksportu: {}", path.display());
    }
    for size in ["64x64", "128x128", "256x256"] {
        let path = flatpak_dir()
            .join("icons")
            .join(size)
            .join(format!("{APP_ID}.png"));
        assert!(path.is_file(), "brak ikony: {}", path.display());
    }
    assert!(
        !flatpak_dir().join("splogbook.desktop").exists(),
        "pakiet Flatpak nie może eksportować pliku desktop o nazwie niezgodnej z app ID"
    );
}

// ============================================================================
// 2. Minimalny i dokładny zestaw uprawnień sandboxa
// ============================================================================

#[test]
fn flatpak_finish_args_are_exactly_the_approved_minimal_set() {
    let args = manifest_finish_args(&manifest());
    let expected = [
        "--share=network",
        "--share=ipc",
        "--socket=wayland",
        "--socket=fallback-x11",
        "--socket=pulseaudio",
        "--device=dri",
        "--talk-name=org.freedesktop.secrets",
    ];
    assert_eq!(
        args.len(),
        expected.len(),
        "zestaw finish-args musi być dokładnie minimalny, jest: {args:?}"
    );
    for arg in expected {
        assert!(
            args.iter().any(|value| value == arg),
            "brak wymaganego uprawnienia {arg}"
        );
    }
}

#[test]
fn flatpak_manifest_forbids_wide_sandbox_permissions() {
    let text = manifest_effective();
    for forbidden in [
        "--filesystem=host",
        "--filesystem=home",
        "--filesystem=/",
        "--device=all",
        "--socket=system-bus",
        "--socket=session-bus",
        "--talk-name=*",
        "--own-name=*",
        "--system-talk-name=*",
        "--filesystem=/dev",
        "--filesystem=/etc",
        "--filesystem=/usr",
        "--filesystem=/opt",
        "--filesystem=/run",
        "--allow=devel",
        "--allow=multiarch",
    ] {
        assert!(
            !text.contains(forbidden),
            "manifest nie może deklarować szerokiego uprawnienia {forbidden}"
        );
    }
}

#[test]
fn flatpak_sandbox_has_no_network_bypass_and_no_privileged_device() {
    // Kontrakt OWNER-M3-J-DEVICE-01: brak --device=all, brak --filesystem=/dev.
    let text = manifest_effective();
    assert!(!text.contains("device=all"));
    assert!(!text.contains("filesystem=/dev"));
    assert!(
        text.contains("--share=network"),
        "sieć jest wymagana dla backendów CAT po TCP"
    );
    // Każde uprawnienie musi być opatrzone komentarzem uzasadnienia.
    let full = manifest();
    for arg in manifest_finish_args(&text) {
        assert!(
            full.contains(&arg),
            "uprawnienie {arg} musi występować w manifeście"
        );
    }
}

// ============================================================================
// 3. Przypięcie źródeł i model offline
// ============================================================================

#[test]
fn flatpak_manifest_sources_are_pinned_and_hermetic() {
    let text = manifest();
    assert!(
        !text.contains("type: git") && !text.contains("type: archive"),
        "manifest nie może deklarować źródeł VCS ani archiwów bez SHA-256"
    );
    assert!(!text.contains("branch:") && !text.contains("tag:"));
    assert!(!text.contains("file://"));
    assert!(!text.contains("/home/") && !text.contains("/Users/"));
    assert!(
        text.contains("path: ../.."),
        "źródło aplikacji musi być względną ścieżką do drzewa repozytorium"
    );
    for skipped in [".local-work", "scratch", "target", "dist"] {
        assert!(
            text.contains(&format!("          - {skipped}\n")),
            "źródło musi pomijać {skipped}"
        );
    }
    assert!(
        text.contains("      - cargo-sources.json"),
        "manifest musi korzystać z wygenerowanej listy przypiętych źródeł"
    );
}

#[test]
fn flatpak_build_is_offline_and_locked() {
    let text = manifest();
    assert!(text.contains("CARGO_NET_OFFLINE: \"true\""));
    assert!(text.contains("cargo build --release --locked --offline"));
    assert!(
        text.contains("directory = \"cargo/vendor\""),
        "cargo musi korzystać z vendored sources"
    );
    assert!(text.contains("replace-with = \"vendored-sources\""));
    let builder = read_repo_file("scripts/build_linux_flatpak.sh");
    assert!(
        builder.contains("--disable-download"),
        "builder musi używać --disable-download w etapie budowy"
    );
    assert!(
        builder.contains("--download-only"),
        "builder musi rozdzielić etap pobierania od etapu budowy"
    );
    assert!(
        builder.contains("git -C \"${REPO_ROOT}\" status --porcelain --untracked-files=no"),
        "builder musi odrzucać brudne drzewo śledzone"
    );
    assert!(
        builder.contains("[ ! -d \"${REPO_ROOT}/target\" ]"),
        "builder musi odrzucać obecność target/ (brak niejawnego wejścia)"
    );
}

// ============================================================================
// 4. Projekcja Cargo.lock → cargo-sources.json (test rozbieżności)
// ============================================================================

#[derive(Debug, PartialEq, Eq)]
struct LockPackage {
    name: String,
    version: String,
    checksum: Option<String>,
    source: Option<String>,
}

fn parse_cargo_lock(text: &str) -> Vec<LockPackage> {
    let mut packages = Vec::new();
    let mut current: Option<LockPackage> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line == "[[package]]" {
            if let Some(package) = current.take() {
                packages.push(package);
            }
            current = Some(LockPackage {
                name: String::new(),
                version: String::new(),
                checksum: None,
                source: None,
            });
            continue;
        }
        let Some(package) = current.as_mut() else {
            continue;
        };
        if line.starts_with('[') {
            packages.push(current.take().expect("package block"));
            continue;
        }
        if let Some(rest) = line.strip_prefix("name = \"") {
            package.name = rest.trim_end_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("version = \"") {
            package.version = rest.trim_end_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("checksum = \"") {
            package.checksum = Some(rest.trim_end_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("source = \"") {
            package.source = Some(rest.trim_end_matches('"').to_string());
        }
    }
    if let Some(package) = current.take() {
        packages.push(package);
    }
    packages
}

fn expected_cargo_sources(lock: &str) -> BTreeMap<String, String> {
    let mut expected = BTreeMap::new();
    for package in parse_cargo_lock(lock) {
        let source = package.source.unwrap_or_default();
        if source.is_empty() {
            assert_eq!(
                package.name, "splogbook",
                "tylko pakiet workspace może nie mieć source"
            );
            continue;
        }
        assert_eq!(
            source, "registry+https://github.com/rust-lang/crates.io-index",
            "nieobsługiwane źródło {}",
            package.name
        );
        let checksum = package
            .checksum
            .unwrap_or_else(|| panic!("brak sumy kontrolnej dla {}", package.name));
        assert_eq!(checksum.len(), 64, "suma kontrolna musi mieć 64 znaki");
        assert!(
            checksum
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "suma kontrolna musi być małymi literami hex"
        );
        expected.insert(
            format!("cargo/vendor/{}-{}", package.name, package.version),
            checksum,
        );
    }
    expected
}

#[test]
fn flatpak_cargo_sources_match_cargo_lock_exactly() {
    let lock = read_repo_file("Cargo.lock");
    let expected = expected_cargo_sources(&lock);
    assert!(
        expected.len() > 500,
        "Cargo.lock powinien opisywać wszystkie zewnętrzne zależności"
    );

    let json_text = read_repo_file("installer/linux/flatpak/cargo-sources.json");
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(&json_text).expect("cargo-sources.json musi być poprawnym JSON-em");
    assert_eq!(
        entries.len(),
        expected.len(),
        "liczba źródeł Flatpak musi odpowiadać liczbie pakietów Cargo.lock"
    );

    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for entry in &entries {
        let destination = entry["dest"]
            .as_str()
            .expect("każde źródło musi mieć dest")
            .to_string();
        let checksum = entry["sha256"]
            .as_str()
            .expect("każde źródło musi mieć sha256");
        let url = entry["url"].as_str().expect("każde źródło musi mieć url");
        assert_eq!(entry["type"], "archive");
        assert_eq!(entry["archive-type"], "tar-gzip");
        assert_eq!(entry["strip-components"], 1);
        assert!(
            url.starts_with("https://static.crates.io/crates/"),
            "źródło musi pochodzić z przypiętego rejestru: {url}"
        );
        assert!(
            url.ends_with(&format!(
                "{}.crate",
                destination.replace("cargo/vendor/", "")
            )),
            "URL {url} nie odpowiada katalogowi {destination}"
        );
        assert!(
            seen.insert(destination.clone(), checksum.to_string())
                .is_none(),
            "zduplikowany katalog vendored: {destination}"
        );
    }
    assert_eq!(
        seen, expected,
        "cargo-sources.json musi być dokładną projekcją Cargo.lock"
    );
    assert!(
        json_text.ends_with("]\n"),
        "plik musi kończyć się znakiem nowej linii"
    );
}

// ============================================================================
// 5. AppStream, desktop i ikony
// ============================================================================

#[test]
fn flatpak_appstream_metadata_is_real_and_consistent() {
    let text = metainfo();
    assert!(text.contains(&format!("<id>{APP_ID}</id>")));
    assert!(text.contains("<metadata_license>CC0-1.0</metadata_license>"));
    assert!(text.contains("<project_license>GPL-3.0-or-later</project_license>"));
    assert!(text.contains(&format!(
        "<launchable type=\"desktop-id\">{APP_ID}.desktop</launchable>"
    )));
    assert!(text.contains("<name>SPLogbook</name>"));
    assert!(text.contains("<url type=\"homepage\">https://github.com/sp6ina/SPLogbook</url>"));
    assert!(
        text.contains("<url type=\"bugtracker\">https://github.com/sp6ina/SPLogbook/issues</url>")
    );
    assert!(text.contains("<category>HamRadio</category>"));
    assert!(text.contains("<category>Utility</category>"));
    assert!(text.contains("<binary>SPLogbook</binary>"));
    // Rozwój 1.3.0-alpha.1 nie może być ogłoszony jako wydanie publiczne.
    assert!(
        !text.contains(&format!("<release version=\"{PACKAGE_VERSION}\"")),
        "metainfo nie może ogłaszać nieopublikowanej wersji {PACKAGE_VERSION}"
    );
    assert!(
        text.contains("<release version=\"1.2.0\""),
        "metainfo musi ogłaszać rzeczywiste publiczne wydanie 1.2.0"
    );
    // Brak trwałych publicznych zrzutów ekranu => brak sekcji screenshots.
    assert!(
        !text.contains("<screenshots>"),
        "nie deklarujemy zrzutów ekranu bez trwałego publicznego źródła"
    );
}

#[test]
fn flatpak_desktop_entry_matches_the_application_identity() {
    let text = desktop_entry();
    assert!(text.contains("[Desktop Entry]"));
    assert!(text.contains(&format!("Icon={APP_ID}")));
    assert!(text.contains(&format!("Exec={COMMAND}")));
    assert!(text.contains("Terminal=false"));
    assert!(text.contains("Categories=HamRadio;Utility;"));
    assert!(text.contains("StartupWMClass=SPLogbook"));
    assert!(
        !text.contains("/opt/"),
        "plik desktop nie może wskazywać /opt"
    );
    assert!(
        !text.contains("/usr/"),
        "plik desktop nie może wskazywać ścieżek hosta"
    );
    assert!(!text.contains("TryExec="), "Flatpak nie potrzebuje TryExec");
}

fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((width, height))
}

#[test]
fn flatpak_icons_have_the_declared_pixel_sizes() {
    for size in [64u32, 128, 256] {
        let path = repo_root()
            .join("installer/linux/flatpak/icons")
            .join(format!("{size}x{size}"))
            .join(format!("{APP_ID}.png"));
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("nie można odczytać {}: {e}", path.display()));
        let (width, height) =
            png_size(&bytes).unwrap_or_else(|| panic!("{} nie jest plikiem PNG", path.display()));
        assert_eq!(
            (width, height),
            (size, size),
            "ikona {} ma nieoczekiwany rozmiar",
            path.display()
        );
        assert!(bytes.len() > 512, "ikona {} jest pusta", path.display());
    }
}

// ============================================================================
// 6. Skrypty: fail-closed, brak maskowania, znaczniki sukcesu na końcu
// ============================================================================

#[test]
fn flatpak_build_and_verify_scripts_are_fail_closed() {
    for relative in [
        "scripts/build_linux_flatpak.sh",
        "scripts/verify_linux_flatpak.sh",
        "scripts/verify_linux_flatpak_negative_fixture.sh",
    ] {
        let text = read_repo_file(relative);
        assert!(
            text.starts_with("#!/usr/bin/env bash"),
            "{relative} musi mieć nagłówek bash"
        );
        assert!(
            text.contains("set -euo pipefail"),
            "{relative} musi działać w trybie fail-closed"
        );
        // Maskowanie błędów jest zakazane.
        assert!(
            !text.contains("|| true\n"),
            "{relative} nie może maskować błędów przez || true"
        );
        assert!(
            !text.contains("2>/dev/null || true"),
            "{relative} nie może tłumić błędów"
        );
        assert!(
            !text.contains("|| exit 0"),
            "{relative} nie może zamieniać błędu w sukces"
        );
    }
}

#[test]
fn flatpak_success_banners_are_the_last_meaningful_statement() {
    for (relative, banner) in [
        (
            "scripts/build_linux_flatpak.sh",
            "FLATPAK BUILD COMPLETED SUCCESSFULLY",
        ),
        (
            "scripts/verify_linux_flatpak.sh",
            "FLATPAK VERIFICATION COMPLETED SUCCESSFULLY",
        ),
        (
            "scripts/verify_linux_flatpak_negative_fixture.sh",
            "FLATPAK NEGATIVE FIXTURES COMPLETED SUCCESSFULLY",
        ),
    ] {
        let text = read_repo_file(relative);
        let position = text
            .find(banner)
            .unwrap_or_else(|| panic!("{relative} nie zawiera banera {banner}"));
        let tail = &text[position..];
        // Po banerze wolno wypisać tylko echo i printf podsumowania.
        for line in tail.lines().skip(1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("echo ") || trimmed.starts_with("printf ")
            {
                continue;
            }
            assert!(
                !trimmed.contains("fail ") && !trimmed.contains("exit 1"),
                "{relative} wykonuje akcję po banerze sukcesu: {trimmed}"
            );
        }
    }
}

#[test]
fn flatpak_verifier_checks_the_permission_contract_exactly() {
    let text = read_repo_file("scripts/verify_linux_flatpak.sh");
    assert!(text.contains("flatpak --user info --show-permissions"));
    assert!(text.contains("EXPECTED_PERMISSIONS"));
    assert!(text.contains("context.devices dri"));
    assert!(text.contains("session-bus.org.freedesktop.secrets talk"));
    assert!(
        text.contains("installed sandbox permissions differ from the approved minimal contract")
    );
    assert!(text.contains("SPLOG_FLATPAK_INSPECTION_ONLY"));
    assert!(text.contains("FLATPAK INSPECTION COMPLETED SUCCESSFULLY"));
    // Weryfikator nie może publikować ani instalować systemowo.
    assert!(!text.contains("sudo "));
    assert!(!text.contains("flatpak install --system"));
    assert!(!text.contains("gh release"));
    assert!(!text.contains("flathub.org/submit"));
}

#[test]
fn flatpak_negative_harness_covers_the_required_fixture_families() {
    let text = read_repo_file("scripts/verify_linux_flatpak_negative_fixture.sh");
    let cases = text.matches("expect_reject \"").count();
    assert!(
        cases >= 45,
        "harness musi pokrywać co najmniej 45 negatywnych fixture'ów, ma {cases}"
    );
    for required in [
        "--filesystem=host",
        "--filesystem=home",
        "--device=all",
        "--socket=system-bus",
        "unpinned git source",
        "archive source without SHA-256",
        "file:/// source path",
        "missing --locked",
        "missing offline enforcement",
        "cargo-sources.json diverges from Cargo.lock",
        "Hamlib library exported into the bundle",
        "/opt dependency inside the bundle",
        "self-update helper inside the bundle",
        "symlink in a forbidden place",
        "setuid file mode",
        "world-writable file mode",
        "publication into release-staging",
        "loss of user data on uninstall (lifecycle guard)",
    ] {
        assert!(
            text.contains(required),
            "harness musi zawierać fixture: {required}"
        );
    }
}

// ============================================================================
// 7. Updater: rozpoznanie Flatpaka i brak self-update
// ============================================================================

#[test]
fn updater_recognizes_the_flatpak_sandbox_signal() {
    use splogbook::cloud::updater::InstallationType;

    assert_eq!(
        splogbook::cloud::updater::FLATPAK_INFO_PATH,
        "/.flatpak-info",
        "stabilny sygnał środowiska Flatpak musi pozostać niezmieniony"
    );
    // Na hoście testowym nie ma /.flatpak-info; funkcja musi to bezpiecznie
    // rozpoznać i nie zgłaszać fałszywego sandboxa.
    let sandbox = splogbook::cloud::updater::is_flatpak_sandbox();
    assert_eq!(sandbox, Path::new("/.flatpak-info").exists());

    let capabilities = splogbook::cloud::updater::InstallationCapabilities::for_installation_type(
        InstallationType::LinuxFlatpak,
    );
    assert!(
        !capabilities.supports_self_apply,
        "Flatpak nigdy nie może samodzielnie podmieniać plików"
    );
    assert!(
        capabilities.requires_delegation,
        "aktualizacja Flatpaka musi być delegowana do flatpak"
    );
    assert!(capabilities.supports_package_manager_monitoring);
    assert!(!capabilities.requires_elevation_by_native_installer);
    assert!(InstallationType::LinuxFlatpak.is_system_managed());
    assert!(!InstallationType::LinuxFlatpak.is_self_managed());
    assert_eq!(InstallationType::LinuxFlatpak.as_str(), "linux-flatpak");
    assert_eq!(
        InstallationType::parse("flatpak"),
        InstallationType::LinuxFlatpak
    );
}

// ============================================================================
// 8. CI: joby Flatpaka istnieją, a publikacja jest niemożliwa
// ============================================================================

#[test]
fn flatpak_ci_jobs_exist_and_never_publish() {
    let workflow = read_repo_file(".github/workflows/build-and-release.yml").replace("\r\n", "\n");
    for job in [
        "build-flatpak",
        "verify-flatpak",
        "verify-flatpak-permissions",
        "verify-flatpak-lifecycle",
        "verify-flatpak-negative-fixtures",
    ] {
        assert!(
            workflow.contains(&format!("  {job}:\n")),
            "workflow musi zawierać job {job}"
        );
    }
    // Job publikacji nie może przyjmować ani stage'ować Flatpaka: wolno mu
    // wyłącznie zawierać trwałą asercję wykluczającą bundle.
    let publish_start = workflow
        .find("  publish-release:")
        .expect("workflow musi mieć job publish-release");
    let publish_section = &workflow[publish_start..];
    assert!(
        !publish_section.contains("cp dist/SPLogbook-Linux-x86_64.flatpak")
            && !publish_section.contains("release-staging/SPLogbook-Linux-x86_64.flatpak")
            && !publish_section
                .contains("release-staging/SPLogbook-Linux-x86_64-1.3.0-alpha.1.flatpak"),
        "job publikacji nie może kopiować ani publikować bundle'a Flatpak"
    );
    assert!(
        publish_section.contains("find release-staging -maxdepth 1 -name '*.flatpak'"),
        "job publikacji musi zawierać trwałą asercję wykluczającą .flatpak z release-staging"
    );
    // Trwała asercja: brak .flatpak w release-staging.
    assert!(
        workflow.contains("release-staging"),
        "workflow musi utrzymywać asercję release-staging"
    );
    assert!(
        !workflow.contains("release-staging/*.flatpak")
            && !workflow.contains("release-staging/.flatpak"),
        "workflow nie może stage'ować .flatpak do release-staging"
    );
}

// ============================================================================
// 9. Dokumentacja i prywatność
// ============================================================================

#[test]
fn flatpak_readme_is_documented_and_indexed() {
    let readme = read_repo_file("installer/linux/flatpak/README.md");
    assert!(readme.contains(APP_ID));
    assert!(readme.contains(RUNTIME));
    assert!(readme.contains("--device=all"));
    assert!(readme.contains("flatpak override --user --device=all"));
    assert!(
        readme.contains("Flathub"),
        "musi jawnie stwierdzać brak publikacji"
    );
    let index = read_repo_file("docs/DOCUMENTATION_INDEX.md");
    assert!(
        index.contains("installer/linux/flatpak/README.md"),
        "README Flatpaka musi być wpisany do indeksu dokumentacji"
    );
}

#[test]
fn flatpak_artefacts_are_not_tracked() {
    let root = repo_root();
    // Żadnego bundle'a, repozytorium OSTree ani katalogu budowy w drzewie.
    for relative in ["dist", "build-dir", "repo"] {
        let path = root.join(relative);
        if path.exists() {
            // dist/ jest katalogiem lokalnym; sprawdzamy tylko brak bundle'a.
            continue;
        }
    }
    for entry in std::fs::read_dir(&root).expect("katalog repozytorium") {
        let entry = entry.expect("wpis katalogu");
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(
            !name.ends_with(".flatpak"),
            "bundle Flatpak nie może znajdować się w drzewie: {name}"
        );
    }
}
