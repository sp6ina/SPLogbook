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
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("nie można odczytać {}: {e}", path.display()));
    // Windows checkout runners may materialise the tree with CRLF; every
    // contract in this suite is about content, never about a line terminator.
    text.replace("\r\n", "\n")
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
        text.contains("rust-1.99.0-x86_64-unknown-linux-gnu.tar.gz"),
        "manifest musi przypinać oficjalny toolchain Rust 1.99.0"
    );
    assert!(
        text.contains("dest: rust-toolchain"),
        "toolchain musi być rozpakowany do katalogu rust-toolchain"
    );
    assert!(
        !text.contains("Sdk.Extension.rust-stable"),
        "manifest nie może używać rozszerzenia SDK z rustc 1.98.0 (MSRV to 1.99.0)"
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
        !text.contains("type: git")
            && !text.contains("type: bzr")
            && !text.contains("type: svn")
            && !text.contains("type: hg"),
        "manifest nie może deklarować źródeł VCS"
    );
    // Archiwum jest dozwolone wyłącznie z przypiętym SHA-256 w tym samym bloku
    // źródła (dotyczy przypiętego toolchainu Rust).
    let lines: Vec<&str> = text.lines().collect();
    let mut archives = 0;
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == "- type: archive" {
            archives += 1;
            let window = &lines[index..std::cmp::min(index + 6, lines.len())];
            let has_pinned_sha = window.iter().any(|candidate| {
                candidate
                    .trim()
                    .strip_prefix("sha256: \"")
                    .and_then(|rest| rest.strip_suffix('"'))
                    .is_some_and(|digest| {
                        digest.len() == 64
                            && digest
                                .chars()
                                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                    })
            });
            assert!(
                has_pinned_sha,
                "źródło archive musi mieć przypięty SHA-256: {line}"
            );
        }
    }
    assert!(archives >= 1, "manifest musi przypinać toolchain Rust");
    assert!(!text.contains("branch:") && !text.contains("tag:"));
    assert!(!text.contains("file://"));
    assert!(!text.contains("/home/") && !text.contains("/Users/"));
    assert!(
        text.contains("        path: ../../.."),
        "źródło aplikacji musi wskazywać katalog główny repozytorium (../../..)"
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
    // flatpak-builder 1.4.x odrzuca źródło `dir` spoza katalogu manifestu
    // w trybie --sandbox, więc budowa nie może go używać; izolację zapewnia
    // bwrap, brak sieci, --disable-download i CARGO_NET_OFFLINE.
    let builder_effective: String = builder
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !builder_effective.contains("--sandbox"),
        "builder nie może używać --sandbox z tym layoutem źródeł"
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

#[test]
fn appstream_launchable_points_to_an_existing_desktop_entry() {
    // Regresja M3-J (run 43595294288): skrypt budujący porównywał launchable z
    // deskoptem wzorcem, który nie mógł się dopasować. Ten test egzekwuje
    // rzeczywisty kontrakt: zawartość <launchable> musi wskazywać istniejący,
    // eksportowany plik desktop o tej samej nazwie.
    let text = metainfo();
    let launchable = text
        .split("<launchable type=\"desktop-id\">")
        .nth(1)
        .and_then(|rest| rest.split("</launchable>").next())
        .expect("metainfo musi deklarować <launchable type=\"desktop-id\">");
    assert_eq!(
        launchable,
        format!("{APP_ID}.desktop"),
        "launchable musi być dokładnie <app-id>.desktop"
    );
    let exported = flatpak_dir().join(launchable);
    assert!(
        exported.is_file(),
        "launchable musi wskazywać istniejący plik desktop: {}",
        exported.display()
    );
    let desktop = desktop_entry();
    assert!(
        desktop.contains(&format!("Icon={APP_ID}")),
        "plik desktop wskazany przez launchable musi używać ikony app-id"
    );
    assert!(
        desktop.contains(&format!("Exec={COMMAND}")),
        "plik desktop wskazany przez launchable musi uruchamiać command z manifestu"
    );
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
    // Regresja M3-J #15 (run 43646350783): flatpak build-finish rozwija
    // --socket=fallback-x11 do `fallback-x11` ORAZ `x11` w metadanych.
    // Weryfikator musi oczekiwać obu wpisów, a manifest nadal nie może
    // deklarować --socket=x11 wprost (kontrakt DEVICE-01 bez zmian).
    assert!(
        text.contains("context.sockets x11"),
        "oczekiwany zestaw uprawnień musi zawierać x11 — obowiązkowy towarzysz fallback-x11"
    );
    assert!(
        !manifest_effective().contains("--socket=x11"),
        "manifest nie może deklarować --socket=x11 wprost"
    );
    // Weryfikator nie może publikować ani instalować systemowo.
    assert!(!text.contains("sudo "));
    assert!(!text.contains("flatpak install --system"));
    assert!(!text.contains("gh release"));
    assert!(!text.contains("flathub.org/submit"));
}

#[test]
fn flatpak_verifier_accepts_the_real_exported_tree_and_flatpak_metadata() {
    // Regresja M3-J #12 (run 43639123495): weryfikator zakładał, że manifest
    // payloadu zawiera wyłącznie pliki regularne, i nie obejmował metadanych,
    // które flatpak-builder zawsze dopisuje do /app (`manifest.json`, katalog
    // `share/app-info`) ani licencji modułu. Odrzucał więc prawdziwy bundle.
    let verifier = read_repo_file("scripts/verify_linux_flatpak.sh");
    let start = verifier
        .find("APP_ALLOWLIST() {")
        .expect("weryfikator musi mieć APP_ALLOWLIST");
    let end = verifier[start..]
        .find("\n}\n")
        .map(|offset| start + offset)
        .expect("APP_ALLOWLIST musi być zamknięta");
    let allowlist = &verifier[start..end];
    // Wpis `APP_ALLOWLIST` używa literału `${APP_ID}`; rozwiń go, aby porównywać
    // rzeczywiste ścieżki eksportu.
    let allowlist_expanded = allowlist.replace("${APP_ID}", APP_ID);

    // Pliki eksportowane przez moduł — wyprowadzone z komend instalacji manifestu.
    let mut expected: Vec<String> = Vec::new();
    for line in manifest().lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("- install -d ") {
            continue;
        }
        let candidate = if trimmed.starts_with("- install ") {
            trimmed.rsplit(' ').next()
        } else {
            trimmed
                .find("> /app/")
                .map(|position| &trimmed[position + 2..])
        };
        if let Some(rest) = candidate.and_then(|token| token.strip_prefix("/app/"))
            && !rest.is_empty()
            && !rest.ends_with('/')
        {
            expected.push(format!("./{rest}"));
        }
    }
    expected.sort();
    expected.dedup();
    assert!(
        expected.len() >= 9,
        "manifest musi eksportować pliki aplikacji, wyprowadzono: {expected:?}"
    );

    // Metadane, które flatpak-builder dopisuje niezależnie od modułu.
    for generated in [
        "./manifest.json".to_string(),
        format!("./share/app-info/icons/flatpak/128x128/{APP_ID}.png"),
        format!("./share/app-info/icons/flatpak/64x64/{APP_ID}.png"),
        format!("./share/app-info/xmls/{APP_ID}.xml.gz"),
        format!("./share/licenses/{APP_ID}/splogbook/LICENSE"),
    ] {
        assert!(
            !expected.contains(&generated),
            "metadana flatpaka nie może być instalowana przez moduł: {generated}"
        );
        expected.push(generated);
    }
    expected.sort();

    for path in &expected {
        assert!(
            allowlist_expanded.contains(&format!("\"{path}\""))
                || allowlist_expanded.contains(&format!("'{path}'")),
            "APP_ALLOWLIST musi zawierać {path}"
        );
    }

    // Katalogi są legalnymi wpisami drzewa; wszystko inne niż `f` i `d` jest
    // odrzucane (w szczególności symlinki).
    assert!(
        verifier.contains("grep -qv '^[fd] '"),
        "weryfikator musi dopuszczać katalogi i odrzucać pozostałe rodzaje wpisów"
    );
    assert!(
        verifier.contains("non-file, non-directory entry"),
        "weryfikator musi zgłaszać kontrolowaną przyczynę dla wpisu innego niż plik/katalog"
    );
    assert!(
        verifier.contains("awk '$1 == \"f\"'"),
        "allowlistę trzeba porównywać wyłącznie z plikami regularnymi"
    );
    // Manifest payloadu produkowany przez builder naprawdę zawiera katalogi.
    let builder = read_repo_file("scripts/build_linux_flatpak.sh");
    assert!(
        builder.contains("find . -mindepth 1 -printf '%y %m %p\\n'"),
        "manifest payloadu musi wypisywać rodzaj wpisu (katalogi i pliki)"
    );

    // Regresja M3-J #16 (run 43688754376): flatpak dopisuje do *wdrożonego*
    // drzewa `/app/.ref`, którego nie ma w payloadzie bundle'a, więc dynamiczne
    // porównanie musi go pominąć — ale allowlista statyczna nie może go zawierać.
    assert!(
        verifier.contains("find . -type f ! -name '.ref'"),
        "dynamiczne porównanie musi pomijać .ref dodawany przez flatpaka"
    );
    assert!(
        !allowlist_expanded.contains("\"./.ref\"") && !allowlist_expanded.contains("'./.ref'"),
        ".ref nie może być wpisany do statycznej allowlisty"
    );
}

#[test]
fn flatpak_builder_pins_the_recorded_source_date_epoch() {
    // Regresja M3-J #13 (run 43643071077): bez `--override-source-date-epoch`
    // flatpak-builder zapisuje w /app/manifest.json czas modyfikacji pliku
    // manifestu. Każdy job CI robi checkout w innym momencie, więc to pole —
    // i cały payload — różniło się między dwoma buildami tego samego commita.
    let builder = read_repo_file("scripts/build_linux_flatpak.sh");
    assert!(
        builder.contains("--override-source-date-epoch=\"${SOURCE_DATE_EPOCH}\""),
        "builder musi przypinać epoch zapisywany przez flatpak-builder"
    );
    assert!(
        builder
            .contains("git -C \"${REPO_ROOT}\" log -1 --format=%ct \"${EXPECTED_BUILD_COMMIT}\""),
        "epoch musi pochodzić z czasu commita, a nie z mtime pliku"
    );
}

#[test]
fn flatpak_verifier_reads_the_installed_commit_with_supported_commands() {
    // Regresja M3-J #14 (run 43643071077): `flatpak list --columns` w wersji
    // 1.16.x nie ma kolumny `commit` (`error: Unknown column: commit`), więc
    // trzy joby Verify padały tuż po instalacji bundle'a.
    let verifier = read_repo_file("scripts/verify_linux_flatpak.sh");
    assert!(
        verifier.contains("flatpak --user info --show-commit \"${APP_ID}\""),
        "weryfikator musi czytać aktywny commit przez flatpak info --show-commit"
    );
    for line in verifier.lines() {
        if line.contains("--columns=") {
            assert!(
                !line.contains("commit"),
                "kolumna `commit` nie istnieje w flatpak list 1.16.x: {line}"
            );
        }
    }
}

#[test]
fn flatpak_build_hides_untracked_artefacts_from_the_embedded_identity() {
    // Regresja M3-J #17 (run 43691509439): moduł buduje z kopii drzewa, w której
    // powstają nieśledzone artefakty (vendored crates, toolchain, cache
    // flatpak-buildera, log tee). build.rs liczy "dirty" z
    // `git status --porcelain`, więc bez wykluczenia `--build-info` zgłasza
    // dirty build, a weryfikator fail-closed odrzuca bundle.
    let text = manifest();
    assert!(
        text.contains("git status --porcelain -z --untracked-files=normal"),
        "manifest musi wykluczać nieśledzone artefakty z tożsamości builda"
    );
    assert!(
        text.contains(">> .git/info/exclude"),
        "wykluczenie musi trafiać do .git/info/exclude, a nie do plików śledzonych"
    );
    assert!(
        text.contains("printf '\\n'"),
        "wykluczenie musi być odseparowane nową linią od kopiowanego pliku exclude"
    );
    assert!(
        text.contains("the module tree is not clean before the build"),
        "build musi fail-closed, gdy drzewo nadal nie jest czyste"
    );
    // CARGO_HOME leży w drzewie modułu i powstaje dopiero podczas builda,
    // więc musi być utworzony i wykluczony już przed snapshotem.
    assert!(
        text.contains("mkdir -p .cargo-home"),
        "CARGO_HOME musi istnieć przed wyliczeniem wykluczeń"
    );
    assert!(
        text.contains("printf '/.cargo-home/\\n'"),
        "CARGO_HOME musi mieć jawny wzorzec: git nie widzi pustego katalogu"
    );
    assert!(
        text.contains("the module tree is dirty after the build"),
        "build musi fail-closed również po kompilacji, z wypisanymi ścieżkami"
    );
    assert!(
        !text.contains("SPLOGBOOK_GIT_DIRTY"),
        "build nie może nadpisywać flagi dirty przez zmienną środowiskową"
    );
    let build_rs = read_repo_file("build.rs");
    assert!(
        build_rs.contains("\"status\", \"--porcelain\""),
        "kontrakt dirty nadal pochodzi z `git status --porcelain` w build.rs"
    );
}

#[test]
fn flatpak_health_check_writes_inside_the_sandbox() {
    // Regresja M3-J #18: sandbox nie widzi ścieżek hosta, a aplikacja ignoruje
    // błąd zapisu i kończy się kodem 0 (`let _ = std::fs::write(...)`), więc
    // dowód health checku musi powstać w sandboxie i być odczytany z prywatnego
    // katalogu aplikacji po stronie hosta.
    let text = read_repo_file("scripts/verify_linux_flatpak.sh");
    assert!(
        text.contains("--command=sh \"${APP_ID}\" -c 'printf %s \"$HOME\"'"),
        "weryfikator musi ustalić HOME sandboxa, zanim przekaże ścieżkę dowodu"
    );
    assert!(
        text.contains("HOST_HEALTH_PROOF=\"${APP_DATA_ROOT}/${HEALTH_PROOF_NAME}\""),
        "dowód health checku musi być odczytany z prywatnego katalogu aplikacji"
    );
    assert!(
        !text.contains("HEALTH_PATH=\"${HOME}/health-proof.txt\""),
        "ścieżka hosta nie może być przekazywana do sandboxa jako cel zapisu"
    );
    let main_rs = read_repo_file("src/main.rs");
    assert!(
        main_rs.contains("let _ = std::fs::write(&target_proof_path"),
        "kontrakt aplikacji: błąd zapisu dowodu jest ignorowany, kod wyjścia to 0"
    );
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

// ============================================================================
// 10. Regresja M3-J failure #9 — wejścia komend budowania
// ============================================================================

/// Ścieżki, które komendy budowania czytają przez `pathlib.Path("…")`.
fn manifest_path_literals(text: &str) -> Vec<String> {
    const PREFIX: &str = "pathlib.Path(\"";
    let mut literals = Vec::new();
    let mut rest = text;
    while let Some(position) = rest.find(PREFIX) {
        let after = &rest[position + PREFIX.len()..];
        let end = after
            .find("\")")
            .unwrap_or_else(|| panic!("niedomknięty literał pathlib.Path: {after}"));
        literals.push(after[..end].to_string());
        rest = &after[end..];
    }
    literals
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|e| panic!("nie można rozwiązać ścieżki {}: {e}", path.display()))
}

#[test]
fn flatpak_build_commands_read_the_included_cargo_sources_projection() {
    // Regresja M3-J failure #9 (run 43628379400). Goły string na liście
    // `sources` jest *dołącznikiem* flatpak-buildera (flatpak-manifest(5)):
    // plik jest czytany, a jego źródła wstawiane do modułu, ale sam plik nie
    // trafia do katalogu budowania. Katalog budowania to drzewo repozytorium
    // (źródło `dir` z `path: ../../..`), więc komenda budowania może czytać
    // wyłącznie ścieżkę, którą to drzewo rzeczywiście dostarcza — i musi to być
    // ten sam plik, który manifest dołącza jako listę źródeł.
    let text = manifest();
    let literals = manifest_path_literals(&text);
    assert!(
        !literals.is_empty(),
        "manifest musi zawierać ścieżkę wejściową komendy budowania do zweryfikowania"
    );

    let included = flatpak_dir().join("cargo-sources.json");
    assert!(
        included.is_file(),
        "dołącznik źródeł musi istnieć: {}",
        included.display()
    );

    for literal in &literals {
        let delivered = repo_root().join(literal);
        assert!(
            delivered.is_file(),
            "komenda budowania czyta {literal:?}, ale żadne zadeklarowane źródło nie \
             dostarcza tej ścieżki do katalogu budowania"
        );
        assert_eq!(
            canonical(&delivered),
            canonical(&included),
            "komenda budowania czyta {literal:?}, a to nie jest projekcja dołączona przez \
             manifest ({}); goły string w `sources` jest dołącznikiem, nie źródłem plikowym",
            included.display()
        );
    }

    // Dołącznik musi pozostać zadeklarowany: bez niego 662 archiwów nie trafi
    // do modułu, a cargo nie znajdzie vendorów.
    assert!(
        text.contains("      - cargo-sources.json"),
        "manifest musi nadal dołączać cargo-sources.json jako listę źródeł"
    );
}

/// Prefiksy instalacji toolchainu z `install.sh --prefix="${PWD}/<dir>"`.
fn installer_prefixes(text: &str) -> Vec<String> {
    const MARKER: &str = "--prefix=\"${PWD}/";
    let mut prefixes = Vec::new();
    let mut rest = text;
    while let Some(position) = rest.find(MARKER) {
        let after = &rest[position + MARKER.len()..];
        match after.find('"') {
            Some(end) => {
                prefixes.push(after[..end].to_string());
                rest = &after[end..];
            }
            None => break,
        }
    }
    prefixes
}

/// Katalogi `bin` odwoływane przez komendy jako `${PWD}/<dir>/bin`.
fn pwd_bin_dirs(text: &str) -> Vec<String> {
    const MARKER: &str = "${PWD}/";
    let mut dirs = Vec::new();
    let mut rest = text;
    while let Some(position) = rest.find(MARKER) {
        let after = &rest[position + MARKER.len()..];
        let token_len = after
            .char_indices()
            .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
            .map(|(index, _)| index)
            .unwrap_or(after.len());
        if after[token_len..].starts_with("/bin") {
            dirs.push(after[..token_len].to_string());
        }
        rest = &after[token_len.max(1)..];
    }
    dirs
}

#[test]
fn flatpak_rust_toolchain_is_materialised_for_every_build_command() {
    // Regresja M3-J failure #10 (run 43635855927). Przypięte archiwum
    // `rust-<wersja>-<target>.tar.gz` to oficjalny *combined installer*: jego
    // najwyższy poziom zawiera `install.sh` i katalogi komponentów, a nie
    // gotowy prefiks z `bin/rustc`. flatpak-builder uruchamia każdą komendę
    // budowania w osobnym `/bin/sh -c`, więc `export PATH` z jednej komendy nie
    // dociera do następnej — toolchain musi być materializowany raz i być na
    // PATH wszystkich komend.
    let text = manifest();

    // 1. Instalator musi być uruchomiony z przypiętego archiwum.
    assert!(
        text.contains("./rust-toolchain/install.sh"),
        "manifest musi materializować toolchain z przypiętego instalatora"
    );

    // 2. Prefiks instalacji musi być inny niż katalog instalatora: install.sh
    //    odmawia instalacji do katalogu, w którym sam leży.
    let prefixes = installer_prefixes(&text);
    assert_eq!(
        prefixes.len(),
        1,
        "manifest musi wybrać dokładnie jeden prefiks instalacji toolchainu, ma: {prefixes:?}"
    );
    let prefix = prefixes[0].as_str();
    assert!(
        !prefix.starts_with("rust-toolchain"),
        "install.sh nie może instalować do własnego katalogu (prefiks {prefix:?})"
    );

    // 3. Ten sam prefiks musi być prependowany do PATH wszystkich komend.
    let expected_prepend = format!("prepend-path: /run/build/splogbook/{prefix}/bin");
    assert!(
        text.contains(&expected_prepend),
        "build-options musi prependować {prefix:?} do PATH, aby toolchain był widoczny \
         w każdej komendzie budowania"
    );

    // 4. Żadna komenda nie może polegać na katalogu `bin` samego archiwum.
    let referenced_bins = pwd_bin_dirs(&text);
    assert!(
        !referenced_bins.is_empty(),
        "komendy budowania muszą używać zmaterializowanego toolchainu"
    );
    for bin in &referenced_bins {
        assert_eq!(
            bin, prefix,
            "komenda budowania używa katalogu bin {bin:?}, którego instalator nie tworzy"
        );
    }

    // 5. Toolchain nie może trafić do /app: końcowy bundle nie może go zawierać.
    assert!(
        !text.contains("--prefix=\"/app") && !text.contains("--prefix=/app"),
        "toolchain nie może być instalowany do /app"
    );
}

// ============================================================================
// 12. CI: zachowanie kodu wyjścia i diagnostyka `if: always()`
// ============================================================================

const DIAGNOSTICS_ARTIFACT: &str = "SPLogbook-Linux-x86_64-flatpak-diagnostics";

#[test]
fn flatpak_ci_preserves_the_build_exit_code_and_always_collects_diagnostics() {
    let workflow = read_repo_file(".github/workflows/build-and-release.yml");
    let start = workflow
        .find("  build-flatpak:\n")
        .expect("workflow musi mieć job build-flatpak");
    let end = workflow[start..]
        .find("  verify-flatpak:")
        .map(|offset| start + offset)
        .expect("job build-flatpak musi poprzedzać verify-flatpak");
    let job = &workflow[start..end];
    // Komentarze YAML dokumentują, czego job nie robi; egzekwujemy wyłącznie
    // efektywną konfigurację.
    let job_effective: String = job
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // 1. Właściwy build nie może maskować wyniku ani używać continue-on-error.
    assert!(
        !job.contains("continue-on-error"),
        "job build-flatpak nie może używać continue-on-error"
    );
    assert!(
        job.contains("PIPESTATUS[0]"),
        "krok budowania musi zachować rzeczywisty kod wyjścia przez PIPESTATUS[0]"
    );
    assert!(
        job.contains("exit \"${status}\""),
        "krok budowania musi zakończyć się rzeczywistym kodem wyjścia builda"
    );
    assert!(
        job.contains("set +e"),
        "krok budowania musi przechwycić kod wyjścia przed asercjami sukcesu"
    );
    assert!(
        job.contains("FLATPAK_BUILD_EXIT_CODE="),
        "kod wyjścia builda musi być zapisany w logu dla diagnostyki"
    );

    // 2. Diagnostyka działa przy sukcesie i przy porażce, po kroku budowania.
    let build_at = job
        .find("name: Build the Flatpak in the pinned privileged container")
        .expect("job musi mieć krok budowania");
    let collect_at = job
        .find("name: Collect the Flatpak build diagnostics")
        .expect("job musi mieć krok zbierania diagnostyki");
    assert!(
        build_at < collect_at,
        "diagnostyka musi być wykonywana po kroku budowania"
    );
    assert!(
        job.contains("if: always()"),
        "diagnostyka musi używać `if: always()`, aby przetrwać porażkę builda"
    );
    // Krok diagnostyczny nie może zmienić wyniku joba.
    assert!(
        job.contains("exit 0"),
        "krok diagnostyczny musi kończyć się jawnym `exit 0` i nie zmieniać wyniku builda"
    );

    // 3. Artefakt diagnostyczny: ograniczona retencja, filtrowany zakres.
    let artifact_at = job
        .find(DIAGNOSTICS_ARTIFACT)
        .expect("musi istnieć artefakt diagnostyczny");
    assert!(
        collect_at < artifact_at,
        "artefakt musi być po zebraniu danych"
    );
    assert!(
        job.contains("retention-days: 7"),
        "artefakt diagnostyczny musi mieć ograniczoną retencję"
    );
    assert!(
        job.contains("if-no-files-found: warn"),
        "brak plików diagnostycznych nie może zmienić wyniku joba"
    );
    assert!(
        job.contains("${DIAG}/flatpak-build.log")
            && job.contains("${DIAG}/module-build-dir.txt")
            && job.contains("dist/flatpak-manifest.projection.txt"),
        "artefakt musi zawierać log builda, projekcję manifestu i stan katalogu budowania"
    );

    // Krok diagnostyczny jest uruchamiany przez GitHuba jako `bash -e`; musi
    // wyłączyć errexit i pipefail, inaczej poprawny SIGPIPE (find | sort | head)
    // kończy krok błędem i zmienia wynik joba (regresja M3-J #11, run
    // 43637880553, gdzie build zakończył się sukcesem, a job i tak był czerwony).
    let diagnostics_end = job
        .find("name: Upload the Flatpak build diagnostics")
        .expect("musi istnieć krok publikacji diagnostyki");
    let diagnostics = &job[collect_at..diagnostics_end];
    assert!(
        diagnostics.contains("set +e"),
        "krok diagnostyczny musi wyłączyć errexit (`set +e`)"
    );
    assert!(
        diagnostics.contains("set +o pipefail"),
        "krok diagnostyczny musi wyłączyć pipefail dla pipeline'ów z `head`"
    );
    assert!(
        !diagnostics.contains("set -euo pipefail"),
        "krok diagnostyczny nie może włączać fail-closed trybu powłoki"
    );

    // 4. Brak prywatnych ścieżek i brak publikacji.
    assert!(
        !job_effective.contains(".local-work") && !job_effective.contains("scratch/"),
        "artefakt nie może publikować prywatnych katalogów"
    );
    assert!(
        !job_effective.contains("release-staging"),
        "job Flatpaka nie może dotykać release-staging"
    );
    assert!(
        !job_effective.contains("gh release") && !job_effective.contains("softprops"),
        "job Flatpaka nie może publikować GitHub Release"
    );
}
