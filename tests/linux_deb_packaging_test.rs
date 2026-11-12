// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Static contract tests for the SPLogbook Linux amd64 binary DEB (M3-H).

use std::fs;
use std::path::Path;

const BUILDER: &str = "scripts/build_linux_deb.sh";
const VERIFIER: &str = "scripts/verify_linux_deb.sh";
const FIXTURES: &str = "scripts/verify_linux_deb_negative_fixture.sh";
const CONTROL: &str = "installer/linux/deb/templates/control";
const LAUNCHER: &str = "installer/linux/deb/launcher";
const DESKTOP: &str = "installer/linux/deb/splogbook.desktop";
const POSTINST: &str = "installer/linux/deb/templates/postinst";
const POSTRM: &str = "installer/linux/deb/templates/postrm";
const WORKFLOW: &str = ".github/workflows/build-and-release.yml";
const GITIGNORE: &str = ".gitignore";

const EXPECTED_DEPENDS: [&str; 17] = [
    "libc6 (>= 2.43)",
    "libasound2t64",
    "libfontconfig1",
    "libgcc-s1",
    "libgl1",
    "libgtk-3-0t64",
    "libudev1",
    "libusb-1.0-0",
    "libwayland-client0",
    "libwayland-cursor0",
    "libwayland-egl1",
    "libx11-6",
    "libxcursor1",
    "libxi6",
    "libxkbcommon-x11-0",
    "libxkbcommon0",
    "libxrandr2",
];

fn read(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
        .replace("\r\n", "\n")
}

fn code_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
}

#[test]
fn deb_builder_uses_official_dpkg_and_is_deterministic() {
    let builder = read(BUILDER);
    for required in [
        "dpkg-deb",
        "--root-owner-group",
        "-Z\"${DEB_COMPRESSION}\"",
        "SOURCE_DATE_EPOCH",
        "touch -h -d",
        "reproducible build",
        "EXPECTED_BUILD_COMMIT",
        "DEB BUILD COMPLETED SUCCESSFULLY",
    ] {
        assert!(
            builder.contains(required),
            "builder must contain {required}"
        );
    }
    for forbidden in ["http://", "ppa:", "latest-debs", "curl | sh", "curl | bash"] {
        assert!(
            !builder.contains(forbidden),
            "builder must not contain {forbidden}"
        );
    }
    assert!(builder.contains("dpkg --validate-version"));
    assert!(builder.contains("dpkg --compare-versions"));
    assert!(builder.contains("1.3.0~alpha.1-1") || builder.contains("~${PRERELEASE}-"));
}

#[test]
fn deb_version_translation_is_explicit_and_ordered() {
    let builder = read(BUILDER);
    assert!(builder.contains("BASE_VERSION=\"${PRODUCT_VERSION%%-*}\""));
    assert!(builder.contains("DEBIAN_VERSION=\"${BASE_VERSION}~${PRERELEASE}-${DEB_REVISION}\""));
    assert!(builder.contains(
        "dpkg --compare-versions \"${DEBIAN_VERSION}\" lt \"${BASE_VERSION}-${DEB_REVISION}\""
    ));
    assert!(builder.contains("dpkg --compare-versions \"${DEBIAN_VERSION}\" gt \"${BASE_VERSION}~alpha.0-${DEB_REVISION}\""));
    assert!(builder.contains("dpkg --compare-versions \"${DEBIAN_VERSION}\" lt \"${BASE_VERSION}~alpha.2-${DEB_REVISION}\""));
    assert!(builder.contains("an epoch is not allowed"));
}

#[test]
fn control_template_has_the_approved_identity() {
    let control = read(CONTROL);
    for required in [
        "Package: splogbook",
        "Version: @DEBIAN_VERSION@",
        "Section: hamradio",
        "Priority: optional",
        "Architecture: amd64",
        "Maintainer: Mariusz Woźniak (SP6INA) <contact@splogbook.org>",
        "Installed-Size: @INSTALLED_SIZE@",
        "Homepage: https://github.com/sp6ina/SPLogbook",
        "Description: Amateur radio logging and station automation workstation",
    ] {
        assert!(
            control.contains(required),
            "control must contain {required}"
        );
    }
    assert!(!control.to_lowercase().contains("official debian"));
    assert!(!control.contains("Recommends:"));
    assert!(!control.contains("Suggests:"));
    assert!(!control.contains("Provides:"));
    assert!(!control.contains("Conflicts:"));
    assert!(!control.contains("Replaces:"));
}

#[test]
fn control_depends_match_the_verified_runtime_contract() {
    let control = read(CONTROL);
    let depends_line = control
        .lines()
        .find(|line| line.starts_with("Depends: "))
        .expect("Depends line");
    for dependency in EXPECTED_DEPENDS {
        assert!(
            depends_line.contains(dependency),
            "Depends is missing {dependency}"
        );
    }
    assert!(
        !depends_line.contains("hamlib"),
        "bundled Hamlib must not be a dependency"
    );
    assert!(
        !depends_line.contains(" | "),
        "alternatives require explicit proof"
    );
    assert_eq!(
        depends_line.matches(',').count(),
        EXPECTED_DEPENDS.len() - 1,
        "unexpected extra dependency"
    );
}

#[test]
fn builder_assembles_the_approved_opt_layout_only() {
    let builder = read(BUILDER);
    assert!(builder.contains("SPLOGBOOK_ROOT=\"${PKG_ROOT}/opt/SPLogbook\""));
    for path in [
        "\"${SPLOGBOOK_ROOT}/SPLogbook\"",
        "\"${SPLOGBOOK_ROOT}/splogbook-updater\"",
        "\"${SPLOGBOOK_ROOT}/lib/libhamlib.so.4\"",
        "\"${SPLOGBOOK_ROOT}/locales\"",
        "\"${SPLOGBOOK_ROOT}/LICENSE\"",
        "\"${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES\"",
        ".splogbook-deb-marker",
        "usr/bin/splogbook",
        "usr/share/applications/splogbook.desktop",
        "usr/share/icons/hicolor/256x256/apps/splogbook.png",
        "usr/share/doc/splogbook/README.Debian",
        "usr/share/doc/splogbook/copyright",
        "usr/share/doc/splogbook/changelog.Debian.gz",
    ] {
        assert!(builder.contains(path), "builder must install {path}");
    }
    assert!(
        !builder.contains("assets/icon.png\" \"${SPLOGBOOK_ROOT}"),
        "assets/ must not be copied into /opt/SPLogbook"
    );
    assert!(
        !builder.contains("README.txt"),
        "portable README.txt must not be installed into the DEB"
    );
    assert!(builder.contains("payload lib/ contains unexpected libraries"));
    assert!(builder.contains("HAMLIB_LGPL-2.1.txt"));
    assert!(builder.contains("templates/LGPL-2.1.txt"));
}

#[test]
fn launcher_is_fail_closed_and_path_based() {
    let launcher = read(LAUNCHER);
    assert!(launcher.starts_with("#!/bin/sh\n"));
    assert!(launcher.contains("\nset -eu\n"));
    assert!(launcher.contains("exec /opt/SPLogbook/SPLogbook \"$@\""));
    for line in code_lines(&launcher) {
        for forbidden in ["sudo", "pkexec", "doas", "LD_LIBRARY_PATH", "HOME", "XDG_"] {
            assert!(
                !line.contains(forbidden),
                "launcher must not contain {forbidden}: {line}"
            );
        }
    }
}

#[test]
fn desktop_entry_uses_the_path_launcher() {
    let desktop = read(DESKTOP);
    assert!(desktop.contains("Exec=splogbook\n"));
    assert!(desktop.contains("Icon=splogbook\n"));
    assert!(desktop.contains("StartupWMClass=SPLogbook\n"));
    assert!(!desktop.contains("/opt/SPLogbook/SPLogbook"));
    assert!(!desktop.contains("sudo"));
}

#[test]
fn icon_is_the_canonical_256x256_png() {
    let bytes = fs::read("assets/icon.png").expect("assets/icon.png");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert_eq!((width, height), (256, 256));
    let builder = read(BUILDER);
    assert!(builder.contains(
        "install -m 0644 \"${REPO_ROOT}/assets/icon.png\" \\\n    \"${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps/splogbook.png\""
    ));
}

#[test]
fn marker_contract_is_deterministic_and_complete() {
    let builder = read(BUILDER);
    for required in [
        "\"marker_version\": \"1.0\"",
        "\"installation_type\": \"LinuxDeb\"",
        "\"management_mode\": \"SystemManaged\"",
        "\"package_version\": \"${PRODUCT_VERSION}\"",
        "\"debian_version\": \"${DEBIAN_VERSION}\"",
        "\"build_commit\": \"${EXPECTED_BUILD_COMMIT}\"",
        "\"resolved_minimum_kernel\": \"${RESOLVED_MIN_KERNEL}\"",
        "\"maximum_glibc\": \"${GLIBC_POLICY_MAX}\"",
        "\"artifact_format\": \"deb\"",
        "\"architecture\": \"amd64\"",
        "\"package_name\": \"splogbook\"",
    ] {
        assert!(builder.contains(required), "marker must contain {required}");
    }
    assert!(
        !builder.contains("date +"),
        "marker must not embed wall-clock time"
    );
    assert!(
        !builder.contains("$USER"),
        "marker must not embed the user name"
    );
}

#[test]
fn maintainer_scripts_are_minimal_and_safe() {
    for (name, source) in [("postinst", read(POSTINST)), ("postrm", read(POSTRM))] {
        assert!(source.starts_with("#!/bin/sh\n"), "{name} must use /bin/sh");
        assert!(source.contains("\nset -e\n"), "{name} must use set -e");
        assert!(source.contains("command -v update-desktop-database"));
        assert!(source.contains("command -v gtk-update-icon-cache"));
        assert!(source.contains("update-desktop-database -q"));
        assert!(!source.contains("|| true"), "{name} must not mask errors");
        assert!(!source.contains("debconf"), "{name} must not use debconf");
        assert!(!source.contains("systemctl"), "{name} must not use systemd");
        for forbidden in ["sudo", "pkexec", "doas", "curl", "wget", "HOME", "XDG_"] {
            for line in code_lines(&source) {
                assert!(
                    !line.contains(forbidden),
                    "{name} must not contain {forbidden}: {line}"
                );
            }
        }
    }
    let builder = read(BUILDER);
    assert!(!builder.contains("templates/preinst"));
    assert!(!builder.contains("templates/prerm"));
    assert!(builder.contains("conffiles are not approved"));
}

#[test]
fn verifier_is_fail_closed_and_prints_success_only_at_the_end() {
    let verifier = read(VERIFIER);
    for required in [
        "set -eu",
        "GLIBC_POLICY_MAX=\"2.43\"",
        "dpkg-deb",
        "lintian",
        "desktop-file-validate",
        "USER_QSO_DATABASE",
        "user data unchanged",
        "dpkg --purge",
        "dpkg --remove",
        "dpkg --audit",
        "DEB VERIFICATION COMPLETED SUCCESSFULLY",
    ] {
        assert!(
            verifier.contains(required),
            "verifier must contain {required}"
        );
    }
    let success_lines = verifier
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| line.contains("DEB VERIFICATION COMPLETED SUCCESSFULLY"))
        .count();
    assert_eq!(success_lines, 1, "success message must exist exactly once");
    assert_eq!(
        verifier.lines().rfind(|line| !line.trim().is_empty()),
        Some("echo \"DEB VERIFICATION COMPLETED SUCCESSFULLY\""),
        "success message must be the final statement"
    );
    for forbidden in ["|| true", "|| :"] {
        assert!(
            !verifier.contains(forbidden),
            "verifier must not mask errors"
        );
    }
}

#[test]
fn verifier_checks_every_required_security_property() {
    let verifier = read(VERIFIER);
    for required in [
        "checksum sidecar is missing",
        "SHA-256 sidecar does not match",
        "Debian binary package",
        "Package must be splogbook",
        "Architecture must be amd64",
        "Depends set does not match",
        "Installed-Size",
        "symbolic links",
        "setuid, setgid or sticky",
        "world-writable",
        "forbidden user data or secret",
        "system libraries must not be bundled",
        "RUNPATH",
        "GLIBC policy satisfied",
        "not found",
        "Exec must be exactly splogbook",
        "256x256 PNG",
        "marker build_commit",
        "0 errors",
    ] {
        assert!(
            verifier.contains(required),
            "verifier must check {required}"
        );
    }
    assert!(
        verifier
            .contains("LINTIAN_SUPPRESSED_TAGS=\"dir-or-file-in-opt,custom-library-search-path\"")
    );
    assert!(verifier.contains("OWNER-M3-H-03"));
    for forbidden in [
        "*.db",
        "*.db-wal",
        "*.db-shm",
        "default_log.db",
        "station_config.json",
        "*.adi",
        "*.adif",
        "*.cbr",
    ] {
        assert!(
            verifier.contains(forbidden),
            "verifier must forbid {forbidden}"
        );
    }
}

#[test]
fn negative_fixtures_cover_all_required_mutations() {
    let fixtures = read(FIXTURES);
    let count = fixtures.matches("expect_reject \"").count();
    assert_eq!(count, 27, "expected 27 negative fixtures, found {count}");
    for required in [
        "missing .sha256",
        "wrong SHA-256",
        "truncated DEB",
        "missing control.tar",
        "wrong Architecture",
        "incomplete Depends",
        "missing main binary",
        "missing launcher",
        "launcher mode 0644",
        "launcher uses sudo",
        "launcher LD_LIBRARY_PATH",
        "dangerous desktop Exec",
        "missing icon",
        "missing marker",
        "wrong build_commit",
        "GLIBC above 2.43",
        "user data in package",
        "setuid binary",
        "absolute symlink",
        "traversal symlink",
        "postinst uses HOME",
        "postinst uses network",
    ] {
        assert!(
            fixtures.contains(required),
            "negative fixtures must cover {required}"
        );
    }
    assert!(fixtures.contains("NEGATIVE FIXTURES REJECTED AS EXPECTED"));
    assert!(fixtures.contains("fixture emitted a success message"));
    assert!(fixtures.contains("SPLOG_DEB_INSPECTION_ONLY=1"));
}

#[test]
fn workflow_builds_verifies_and_uploads_the_deb_as_a_ci_artifact() {
    let workflow = read(WORKFLOW);
    assert!(workflow.contains("build-deb:"));
    assert!(workflow.contains("verify-deb:"));
    assert!(workflow.contains("verify-deb-negative-fixtures:"));
    assert!(workflow.contains("name: SPLogbook-Linux-amd64-deb"));
    assert!(workflow.contains(
        "ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c"
    ));
    assert!(workflow.contains("scripts/build_linux_deb.sh"));
    assert!(workflow.contains("scripts/verify_linux_deb.sh"));
    assert!(workflow.contains("scripts/verify_linux_deb_negative_fixture.sh"));
    assert!(!workflow.contains("git add"));
    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("publish-release job");
    assert!(
        !publish.contains("SPLogbook-Linux-amd64"),
        "release must not publish the DEB"
    );
    assert!(publish.contains("startsWith(github.ref, 'refs/tags/v')"));
}

#[test]
fn gitignore_excludes_generated_deb_artifacts() {
    let gitignore = read(GITIGNORE);
    assert!(gitignore.contains("*.deb"));
    assert!(gitignore.contains("*.deb.sha256"));
}

#[test]
fn no_debian_source_package_is_created() {
    for forbidden in [
        "debian/rules",
        "debian/changelog",
        "debian/control",
        "debian/copyright",
        ".dsc",
        "orig.tar",
        "debhelper",
    ] {
        assert!(
            !Path::new(forbidden).exists(),
            "M3-H must not create a source package artifact: {forbidden}"
        );
    }
    assert!(!Path::new("debian").exists());
    let builder = read(BUILDER);
    assert!(!builder.contains("dpkg-buildpackage"));
    assert!(!builder.contains("debuild"));
    assert!(!builder.contains("dh_"));
}

#[test]
fn readme_documents_the_deb_contract() {
    let readme = read("installer/linux/deb/README.md");
    let lower = readme.to_lowercase();
    for required in [
        "binary",
        "ubuntu 26.04 lts",
        "glibc_2.43",
        "1.3.0~alpha.1-1",
        "/opt/splogbook",
        "libhamlib.so.4",
        "linuxdeb",
        "systemmanaged",
        "user data",
        "negative fixtures",
        "ci artifact only",
    ] {
        assert!(lower.contains(required), "README must document {required}");
    }
}
