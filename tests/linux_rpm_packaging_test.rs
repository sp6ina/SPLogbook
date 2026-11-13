// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Static contract tests for the SPLogbook Linux x86_64 binary RPM (M3-I).

use std::fs;
use std::path::Path;

const BUILDER: &str = "scripts/build_linux_rpm.sh";
const VERIFIER: &str = "scripts/verify_linux_rpm.sh";
const FIXTURES: &str = "scripts/verify_linux_rpm_negative_fixture.sh";
const MANIFEST: &str = "scripts/linux_rpm_manifest.sh";
const FIXTURE_BUILDER: &str = "scripts/build_linux_rpm_fixture.sh";
const SPEC: &str = "installer/linux/rpm/splogbook.spec";
const LAUNCHER: &str = "installer/linux/rpm/launcher";
const DESKTOP: &str = "installer/linux/rpm/splogbook.desktop";
const README: &str = "installer/linux/rpm/README.md";
const WORKFLOW: &str = ".github/workflows/build-and-release.yml";
const GITIGNORE: &str = ".gitignore";

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
fn rpm_builder_uses_official_rpmbuild_and_is_deterministic() {
    let builder = read(BUILDER);
    for required in [
        "rpmbuild -bb",
        "SOURCE_DATE_EPOCH",
        "touch -h -d",
        "patchelf",
        "the RPM build is not reproducible",
        "EXPECTED_BUILD_COMMIT",
        "RPM BUILD COMPLETED SUCCESSFULLY",
        "linux_rpm_manifest.sh",
        "rpmdev-vercmp",
    ] {
        assert!(
            builder.contains(required),
            "builder must contain {required}"
        );
    }
    for forbidden in ["http://", "curl | sh", "curl | bash", "git add", "--nodeps"] {
        assert!(
            !builder.contains(forbidden),
            "builder must not contain {forbidden}"
        );
    }
    assert_eq!(
        builder.matches("build_once").count(),
        3,
        "builder must define and call two independent builds"
    );
}

#[test]
fn rpm_version_translation_is_explicit_and_ordered() {
    let builder = read(BUILDER);
    assert!(builder.contains("RPM_VERSION=\"${BASE_VERSION}\""));
    assert!(builder.contains("RPM_RELEASE=\"0.1.${PRERELEASE}\""));
    assert!(builder.contains("vercmp_is \"${CURRENT_EVR}\" \"1.3.0-1\" 12"));
    assert!(builder.contains("vercmp_is \"${CURRENT_EVR}\" \"1.3.0-0.1.alpha.0\" 11"));
    assert!(builder.contains("vercmp_is \"${CURRENT_EVR}\" \"1.3.0-0.1.alpha.2\" 12"));
    assert!(builder.contains("an epoch is not allowed"));
    assert!(!builder.contains("%{?dist}"));
}

#[test]
fn spec_has_the_approved_identity_and_no_epoch() {
    let spec = read(SPEC);
    for required in [
        "Name:           splogbook",
        "Version:        %{splog_version}",
        "Release:        %{splog_release}",
        "BuildArch:      x86_64",
        "Vendor:         SPLogbook",
        "License:        GPL-3.0-or-later AND LGPL-2.1-or-later",
        "URL:            https://github.com/sp6ina/SPLogbook",
        "Summary:        Amateur radio logging and station automation workstation",
        "Group:          Applications/Communications",
        "AutoReqProv:    no",
        "%global _buildhost build.splogbook.invalid",
    ] {
        assert!(spec.contains(required), "spec must contain {required}");
    }
    assert!(!spec.contains("Epoch:"));
    assert!(!spec.contains("%{?dist}"));
    assert!(!spec.to_lowercase().contains("official fedora"));
}

#[test]
fn requires_use_portable_capabilities_and_exclude_hamlib() {
    let spec = read(SPEC);
    for required in [
        "Requires:       glibc >= 2.43",
        "Requires:       libasound.so.2()(64bit)",
        "Requires:       libX11.so.6()(64bit)",
        "Requires:       libgtk-3.so.0()(64bit)",
        "Requires:       libc.so.6(GLIBC_2.43)(64bit)",
        "Requires:       libudev.so.1()(64bit)",
        "Requires:       libusb-1.0.so.0()(64bit)",
    ] {
        assert!(spec.contains(required), "spec must require {required}");
    }
    assert!(
        !spec.contains("Requires:       hamlib"),
        "bundled Hamlib must not be a dependency"
    );
    assert!(!spec.contains("Recommends:"));
    assert!(!spec.contains("Suggests:"));
    assert!(!spec.contains("Conflicts:"));
    assert!(!spec.contains("Obsoletes:"));
}

#[test]
fn spec_installs_the_approved_layout_and_modes() {
    let spec = read(SPEC);
    for required in [
        "%dir /opt/SPLogbook",
        "%attr(0755,root,root) /opt/SPLogbook/SPLogbook",
        "%attr(0755,root,root) /opt/SPLogbook/splogbook-updater",
        "%attr(0644,root,root) /opt/SPLogbook/lib/libhamlib.so.4",
        "%attr(0644,root,root) /opt/SPLogbook/.splogbook-rpm-marker",
        "%attr(0755,root,root) /usr/bin/splogbook",
        "%attr(0644,root,root) /usr/share/applications/splogbook.desktop",
        "%attr(0644,root,root) /usr/share/icons/hicolor/256x256/apps/splogbook.png",
        "%doc /usr/share/doc/splogbook/README.RPM",
        "%license /usr/share/licenses/splogbook/LICENSE",
        "%license /usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt",
    ] {
        assert!(spec.contains(required), "spec must install {required}");
    }
    assert!(
        !spec.contains("/usr/share/applications\n"),
        "the package must not own system-wide /usr/share/applications"
    );
    assert!(spec.contains("%global __os_install_post %{nil}"));
    assert!(spec.contains("%global _binary_payload w6.xzdio"));
}

#[test]
fn spec_has_only_post_and_postun_scriptlets() {
    let spec = read(SPEC);
    assert!(spec.contains("%post\n"));
    assert!(spec.contains("%postun\n"));
    for forbidden in [
        "%pre\n",
        "%preun\n",
        "%pretrans\n",
        "%posttrans\n",
        "%trigger",
        "%verifyscript",
        "%config",
        "%ghost",
    ] {
        assert!(
            !spec.contains(forbidden),
            "spec must not contain {forbidden}"
        );
    }
    for forbidden in [
        "sudo",
        "pkexec",
        "doas",
        "curl",
        "wget",
        "HOME",
        "XDG_",
        "systemctl",
    ] {
        for line in code_lines(&spec) {
            assert!(
                !line.contains(forbidden),
                "spec scriptlet must not contain {forbidden}: {line}"
            );
        }
    }
}

#[test]
fn launcher_and_desktop_are_fail_closed() {
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
    assert!(
        read(SPEC)
            .contains("%attr(0644,root,root) /usr/share/icons/hicolor/256x256/apps/splogbook.png")
    );
}

#[test]
fn marker_contract_is_deterministic_and_complete() {
    let builder = read(BUILDER);
    for required in [
        "\"marker_version\": \"1.0\"",
        "\"installation_type\": \"LinuxRpm\"",
        "\"management_mode\": \"SystemManaged\"",
        "\"package_version\": \"${PRODUCT_VERSION}\"",
        "\"rpm_version\": \"${RPM_VERSION}\"",
        "\"rpm_release\": \"${RPM_RELEASE}\"",
        "\"build_commit\": \"${EXPECTED_BUILD_COMMIT}\"",
        "\"resolved_minimum_kernel\": \"${RESOLVED_MIN_KERNEL}\"",
        "\"maximum_glibc\": \"${GLIBC_POLICY_MAX}\"",
        "\"artifact_format\": \"rpm\"",
        "\"architecture\": \"x86_64\"",
        "\"package_name\": \"splogbook\"",
        "\"target\": \"linux-x86_64\"",
    ] {
        assert!(builder.contains(required), "marker must contain {required}");
    }
    assert!(
        !builder.contains("date +"),
        "marker must not embed wall-clock time"
    );
}

#[test]
fn verifier_is_fail_closed_and_success_is_last() {
    let verifier = read(VERIFIER);
    for required in [
        "set -eu",
        "GLIBC_POLICY_MAX=\"2.43\"",
        "rpm -qpR",
        "rpmlint",
        "desktop-file-validate",
        "USER_QSO_DATABASE",
        "user data unchanged",
        "rpm -e",
        "rpm -U --oldpackage",
        "rpm -U --replacepkgs",
        "RPM VERIFICATION COMPLETED SUCCESSFULLY",
    ] {
        assert!(
            verifier.contains(required),
            "verifier must contain {required}"
        );
    }
    let success_lines = verifier
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| line.contains("RPM VERIFICATION COMPLETED SUCCESSFULLY"))
        .count();
    assert_eq!(success_lines, 1, "success message must exist exactly once");
    assert_eq!(
        verifier.lines().rfind(|line| !line.trim().is_empty()),
        Some("echo \"RPM VERIFICATION COMPLETED SUCCESSFULLY\""),
        "success message must be the final statement"
    );
    for forbidden in ["|| true", "|| :", "--nodeps"] {
        assert!(
            !verifier.contains(forbidden),
            "verifier must not mask errors with {forbidden}"
        );
    }
}

#[test]
fn verifier_checks_every_required_security_property() {
    let verifier = read(VERIFIER);
    for required in [
        "checksum sidecar is missing",
        "SHA-256 sidecar does not match",
        "not an RPM package",
        "Name must be splogbook",
        "Architecture must be x86_64",
        "Requires set does not match",
        "RPM SIZE",
        "symbolic links",
        "setuid/setgid",
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
        "no config files are packaged",
    ] {
        assert!(
            verifier.contains(required),
            "verifier must check {required}"
        );
    }
    assert!(verifier.contains("OWNER-M3-H-03"));
    assert!(verifier.contains("dir-or-file-in-opt,custom-library-search-path"));
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
fn manifest_is_generated_from_the_rpm_and_hashed() {
    let manifest = read(MANIFEST);
    for required in [
        "SPLOGBOOK_RPM_MANIFEST 1.0",
        "artifact_sha256",
        "artifact_size",
        "file_count",
        "entry_count",
        "rpm2cpio",
        "sha256sum",
    ] {
        assert!(
            manifest.contains(required),
            "manifest tool must contain {required}"
        );
    }
    assert!(read(BUILDER).contains("\"${RPM_PATH}.manifest\""));
    assert!(read(FIXTURE_BUILDER).contains("rpmbuild -bb"));
}

#[test]
fn negative_fixtures_cover_all_required_mutations() {
    let fixtures = read(FIXTURES);
    let count = fixtures.matches("expect_reject \"").count();
    assert_eq!(count, 34, "expected 34 negative fixtures, found {count}");
    for required in [
        "wrong package name",
        "wrong version",
        "wrong release",
        "wrong architecture",
        "wrong build_commit",
        "wrong target",
        "missing main binary",
        "missing updater",
        "missing launcher",
        "extra forbidden file",
        "launcher mode 0644",
        "wrong owner/group",
        "symlink instead of file",
        "missing private Hamlib",
        "wrong Hamlib SONAME",
        "Hamlib is not an ELF",
        "forbidden RUNPATH",
        "forbidden RPATH",
        "unresolved library",
        "launcher uses sudo",
        "launcher LD_LIBRARY_PATH",
        "dangerous desktop Exec",
        "missing icon",
        "missing marker",
        "GLIBC above 2.43",
        "forbidden %pre scriptlet",
        "scriptlet uses network",
        "scriptlet removes user data",
        "manifest mismatch",
        "wrong SHA-256",
        "truncated/non-RPM",
    ] {
        assert!(
            fixtures.contains(required),
            "negative fixtures must cover {required}"
        );
    }
    assert!(fixtures.contains("NEGATIVE FIXTURES REJECTED AS EXPECTED"));
    assert!(fixtures.contains("fixture emitted a success message"));
    assert!(fixtures.contains("SPLOG_RPM_INSPECTION_ONLY=1"));
}

#[test]
fn workflow_builds_verifies_and_never_publishes_rpm() {
    let workflow = read(WORKFLOW);
    assert!(workflow.contains("build-rpm:"));
    assert!(workflow.contains("verify-rpm:"));
    assert!(workflow.contains("verify-rpm-negative-fixtures:"));
    assert!(workflow.contains("name: SPLogbook-Linux-x86_64-rpm"));
    assert!(workflow.contains(
        "fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c"
    ));
    assert!(workflow.contains(
        "opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1"
    ));
    assert!(workflow.contains("scripts/build_linux_rpm.sh"));
    assert!(workflow.contains("scripts/verify_linux_rpm.sh"));
    assert!(workflow.contains("scripts/verify_linux_rpm_negative_fixture.sh"));
    assert!(!workflow.contains("git add"));
    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("publish-release job");
    assert!(
        !publish.contains("SPLogbook-Linux-x86_64-rpm"),
        "release must not publish the RPM artifact"
    );
    assert!(publish.contains("startsWith(github.ref, 'refs/tags/v')"));
    assert!(publish.contains("RPM must never be published"));
}

#[test]
fn gitignore_excludes_generated_rpm_artifacts() {
    let gitignore = read(GITIGNORE);
    assert!(gitignore.contains("*.rpm"));
    assert!(gitignore.contains("*.rpm.sha256"));
    assert!(gitignore.contains("*.rpm.manifest"));
}

#[test]
fn no_source_rpm_or_distribution_repository_is_created() {
    for forbidden in [
        "SOURCES",
        "SPECS",
        "SRPMS",
        "copr",
        "dnf.conf",
        "zypper.repo",
    ] {
        assert!(
            !Path::new(forbidden).exists(),
            "M3-I must not create a source package or repository artifact: {forbidden}"
        );
    }
    assert!(!read(BUILDER).contains("rpmbuild -bs"));
    assert!(!read(BUILDER).contains("copr-cli"));
    assert!(!read(BUILDER).contains("createrepo"));
}

#[test]
fn readme_documents_the_rpm_contract() {
    let readme = read(README);
    let lower = readme.to_lowercase();
    for required in [
        "fedora linux 45",
        "opensuse tumbleweed",
        "glibc_2.43",
        "1.3.0-0.1.alpha.1",
        "/opt/splogbook",
        "libhamlib.so.4",
        "linuxrpm",
        "systemmanaged",
        "user data",
        "negative fixtures",
        "ci artifact only",
        "rpmlint",
    ] {
        assert!(lower.contains(required), "README must document {required}");
    }
}
