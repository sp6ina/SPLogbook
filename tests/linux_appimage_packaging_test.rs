// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! STAGE-M3-G — durable contracts for the Linux AppImage packaging pipeline.
//!
//! These tests are deliberately independent of `dist/` and of any downloaded
//! binary. They assert the *contracts* that the shell pipeline implements, so
//! that a future change cannot silently drop the pinned toolchain, the approved
//! AppDir layout, the GLIBC policy, the fail-closed verifier or the negative
//! fixtures. The concrete appimagetool commit/SHA-256 of a gate is evidence in
//! the gate report, never a permanent requirement here.

use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
        .replace("\r\n", "\n")
}

fn executable_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
}

#[test]
fn apprun_is_posix_and_forwards_arguments() {
    let apprun = read("installer/linux/appimage/AppRun");

    assert!(
        apprun.starts_with("#!/bin/sh"),
        "AppRun must use the POSIX /bin/sh interpreter"
    );
    assert!(
        apprun.contains("set -eu"),
        "AppRun must use strict `set -eu`"
    );
    assert!(
        apprun.contains("exec \"${APPDIR}/usr/bin/SPLogbook\" \"$@\""),
        "AppRun must exec usr/bin/SPLogbook and forward every argument"
    );
    assert!(
        apprun.contains("CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd"),
        "AppRun must resolve its own AppDir independently of the current CWD"
    );
    assert!(
        !apprun.contains("[["),
        "AppRun must not use bash-only `[[` tests"
    );
    assert!(
        !apprun.contains("BASH_SOURCE"),
        "AppRun must not depend on bash-only variables"
    );

    let code: Vec<&str> = executable_lines(&apprun).collect();
    for line in &code {
        for forbidden in ["sudo", "pkexec", "doas", "LD_LIBRARY_PATH"] {
            assert!(
                !line.contains(forbidden),
                "AppRun must not reference {forbidden}: {line}"
            );
        }
        for assigned in ["HOME=", "XDG_CONFIG_HOME=", "XDG_DATA_HOME="] {
            assert!(
                !line.contains(assigned),
                "AppRun must not rewrite {assigned}: {line}"
            );
        }
    }
}

#[test]
fn appdir_uses_the_approved_layout() {
    let script = read("scripts/build_linux_appimage.sh");

    for required in [
        "AppRun",
        "splogbook.desktop",
        "splogbook.png",
        ".DirIcon",
        "usr/bin/SPLogbook",
        "usr/bin/splogbook-updater",
        "usr/lib/libhamlib.so.4",
        "usr/share/applications/splogbook.desktop",
        "usr/share/icons/hicolor/256x256/apps/splogbook.png",
        "usr/share/doc/splogbook/LICENSE",
        "usr/share/doc/splogbook/README.txt",
        "usr/share/doc/splogbook/THIRD_PARTY_LICENSES/",
        "usr/share/splogbook/.splogbook-appimage-marker",
    ] {
        assert!(
            script.contains(required),
            "the AppDir builder must create the approved path {required}"
        );
    }

    assert!(
        script.contains(".DirIcon must be a regular file"),
        "the builder must require .DirIcon to be a regular file"
    );
    assert!(
        script.contains("the AppDir must not contain symlinks"),
        "the builder must forbid symlinks"
    );
    assert!(
        script.contains("setuid/setgid"),
        "the builder must forbid setuid/setgid entries"
    );
    assert!(
        script.contains("desktop-file-validate"),
        "the builder must run desktop-file-validate"
    );
    assert!(
        !script.contains("metainfo.xml") && !script.contains(".appdata.xml"),
        "AppStream metadata must not be generated for M3-G"
    );
    assert!(
        script.contains("--no-appstream"),
        "the builder must explicitly disable the AppStream check"
    );
}

#[test]
fn marker_is_complete_and_deterministic() {
    let script = read("scripts/build_linux_appimage.sh");
    let marker_start = script
        .find("cat > \"${MARKER}\"")
        .expect("the builder must write the AppImage marker");
    let marker_end = script
        .find("chmod 0644 \"${MARKER}\"")
        .expect("the builder must set explicit marker permissions");
    let marker_block = &script[marker_start..marker_end];

    for field in [
        "package_version",
        "build_commit",
        "installation_type",
        "resolved_minimum_kernel",
        "maximum_glibc",
        "artifact_format",
        "architecture",
        "appimagetool_channel",
        "appimagetool_source_commit",
        "appimagetool_sha256",
        "runtime_release_or_commit",
        "runtime_sha256",
    ] {
        assert!(
            marker_block.contains(&format!("\"{field}\":")),
            "the marker must contain the field {field}"
        );
    }

    for (field, value) in [
        ("installation_type", "LinuxAppImage"),
        ("artifact_format", "AppImage"),
        ("architecture", "x86_64"),
        ("resolved_minimum_kernel", "7.0"),
    ] {
        let expected = format!("\"{field}\": \"{value}\"");
        assert!(
            marker_block.contains(&expected),
            "the marker must pin {expected}"
        );
    }
    assert!(
        marker_block.contains("\"maximum_glibc\": \"${GLIBC_POLICY_MAX}\""),
        "the marker must derive maximum_glibc from the hard-coded policy"
    );
    assert!(
        marker_block.contains("\"appimagetool_channel\": \"${TOOL_CHANNEL}\""),
        "the marker must derive appimagetool_channel from the pinned toolchain"
    );
    assert!(
        script.contains("GLIBC_POLICY_MAX=\"2.43\""),
        "the builder must hard-code the GLIBC_2.43 policy"
    );
    assert!(
        script.contains("[[ \"${TOOL_CHANNEL}\" == \"continuous\" ]]"),
        "the builder must require the continuous channel"
    );

    for forbidden in [
        "$(date",
        "$(whoami)",
        "$HOSTNAME",
        "$USER",
        "/home/",
        "/Users/",
    ] {
        assert!(
            !marker_block.contains(forbidden),
            "the marker must stay deterministic and host independent: {forbidden}"
        );
    }
    assert!(
        script.contains("\"created_by\": \"scripts/build_linux_appimage.sh\""),
        "the marker must record a stable producer identity"
    );
}

#[test]
fn desktop_uses_the_canonical_template() {
    let template = read("installer/linux/portable/templates/splogbook.desktop");
    let legacy = read("assets/splogbook.desktop");
    let script = read("scripts/build_linux_appimage.sh");

    assert!(
        template.contains("Exec=SPLogbook\n"),
        "the canonical desktop file must use Exec=SPLogbook"
    );
    assert!(
        template.contains("Icon=splogbook\n"),
        "the canonical desktop file must use Icon=splogbook"
    );
    assert!(
        template.contains("StartupWMClass=SPLogbook\n"),
        "the canonical desktop file must use StartupWMClass=SPLogbook"
    );
    assert!(
        legacy.contains("Exec=splogbook"),
        "the legacy assets/ desktop file is expected to stay inconsistent and must not be used"
    );
    assert!(
        !script.contains("assets/splogbook.desktop"),
        "the builder must not use the legacy assets/splogbook.desktop"
    );
    assert!(
        script.contains("installer/linux/portable/templates/splogbook.desktop"),
        "the builder must use the canonical portable desktop template"
    );
    assert!(
        script.contains("desktop file differs from the canonical template"),
        "the builder must enforce byte identity of both desktop copies"
    );
}

#[test]
fn icon_is_a_256_png_and_all_copies_are_byte_identical() {
    let icon = fs::read("assets/icon.png").expect("assets/icon.png");
    assert_eq!(
        &icon[..8],
        &b"\x89PNG\r\n\x1a\n"[..],
        "assets/icon.png must be a PNG file"
    );
    let width = u32::from_be_bytes([icon[16], icon[17], icon[18], icon[19]]);
    let height = u32::from_be_bytes([icon[20], icon[21], icon[22], icon[23]]);
    assert_eq!((width, height), (256, 256), "the icon must be 256x256");

    let script = read("scripts/build_linux_appimage.sh");
    for copy in ["splogbook.png", ".DirIcon", "apps/splogbook.png"] {
        assert!(
            script.contains(copy),
            "the builder must install the icon copy {copy}"
        );
    }
    assert!(
        script.contains("icon differs from assets/icon.png"),
        "the builder must enforce byte identity of every icon copy"
    );
    let verifier = read("scripts/verify_appimage.sh");
    assert!(
        verifier.contains("must be exactly 256x256"),
        "the verifier must enforce the 256x256 icon dimensions"
    );
}

#[test]
fn only_libhamlib_is_bundled() {
    let build = read("scripts/build_linux_appimage.sh");
    let verify = read("scripts/verify_appimage.sh");

    assert!(
        build.contains("install -m 0755 \"${PAYLOAD}/lib/libhamlib.so.4\""),
        "the builder must bundle exactly libhamlib.so.4 from the verified payload"
    );
    assert!(
        build.contains("only libhamlib.so.4 may be bundled"),
        "the builder must reject any other bundled library"
    );
    assert!(
        verify.contains("only libhamlib.so.4 may be bundled"),
        "the verifier must reject any other bundled library"
    );
    assert!(
        verify.contains("ld-linux") && verify.contains("libc.so"),
        "the verifier must reject a bundled loader or glibc"
    );
    assert!(
        verify.contains("libgcc_s.so") && verify.contains("libstdc++.so"),
        "the verifier must reject bundled libgcc_s/libstdc++"
    );
}

#[test]
fn glibc_policy_is_2_43_and_compared_semantically() {
    let verify = read("scripts/verify_appimage.sh");

    assert!(
        verify.contains("GLIBC_POLICY_MAX=\"2.43\""),
        "the verifier must hard-code the GLIBC_2.43 policy"
    );
    assert!(
        verify.contains("sort -V") && verify.contains("version_le"),
        "the verifier must compare GLIBC versions semantically"
    );
    assert!(
        verify.contains("newer than the GLIBC_"),
        "the verifier must fail when an artifact requires more than the policy"
    );
    assert!(
        verify.contains("SPLOG_APPIMAGE_NEGATIVE_FIXTURE")
            && verify.contains("SPLOG_APPIMAGE_GLIBC_MAX_OVERRIDE"),
        "a lowered policy must be available only for the negative fixture"
    );
    assert!(
        verify.contains("may never exceed the GLIBC_${GLIBC_POLICY_MAX} policy"),
        "the negative-fixture override must never raise the policy"
    );
}

#[test]
fn verifier_is_fail_closed_and_success_is_last() {
    let verify = read("scripts/verify_appimage.sh");

    assert!(verify.contains("set -eu"), "the verifier must use set -eu");
    assert!(
        !executable_lines(&verify).any(|line| line.contains("|| true")),
        "the verifier must not mask any critical command with `|| true`"
    );
    for required in [
        "sha256sum -c",
        "--appimage-extract",
        "desktop-file-validate",
        "find \"${APPDIR}\" -type l",
        "-perm /6000",
        "-perm -0002",
        "not found",
        "error while loading shared libraries",
        "Library runpath: [$ORIGIN/../lib]",
        "default_log.db",
        "user data was modified",
        "the AppDir content changed",
    ] {
        assert!(
            verify.contains(required),
            "the verifier must contain the fail-closed probe: {required}"
        );
    }

    let success = "APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY";
    assert_eq!(
        verify
            .lines()
            .filter(|line| !line.trim_start().starts_with('#') && line.contains(success))
            .count(),
        1,
        "the success message must appear exactly once in executable code"
    );
    let last = verify
        .lines()
        .rfind(|line| !line.trim().is_empty())
        .expect("the verifier must not be empty")
        .trim();
    assert_eq!(
        last, "echo \"APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY\"",
        "the success message must be the last statement"
    );
}

#[test]
fn toolchain_resolution_is_official_verified_and_pinned() {
    let script = read("scripts/install_appimage_toolchain.sh");

    assert!(
        script.contains("APPIMAGETOOL_REPO=\"AppImage/appimagetool\""),
        "appimagetool must come from the official AppImage/appimagetool repository"
    );
    assert!(
        script.contains("APPIMAGETOOL_TAG=\"continuous\""),
        "the continuous channel must be resolved explicitly"
    );
    assert!(
        script.contains("APPIMAGETOOL_ASSET=\"appimagetool-x86_64.AppImage\""),
        "the x86_64 asset must be selected explicitly"
    );
    assert!(
        script.contains("releases/tags/${APPIMAGETOOL_TAG}"),
        "the release metadata must be read from the official tag endpoint"
    );
    assert!(
        script.contains("RUNTIME_REPO=\"AppImage/type2-runtime\""),
        "the type-2 runtime must come from the official repository"
    );

    for code in [
        "TOOL_CHECKSUM_MISMATCH",
        "DIGEST_UNAVAILABLE",
        "AMBIGUOUS_TOOL_ASSET",
        "UNSUPPORTED_",
        "CONTINUOUS_RELEASE_RACE",
    ] {
        assert!(
            script.contains(code),
            "the resolver must fail closed with {code}"
        );
    }

    assert!(
        !script.contains("AppImageKit")
            || !executable_lines(&script).any(|line| line.contains("AppImageKit")),
        "the resolver must never use the legacy AppImageKit repository"
    );
    assert!(
        !executable_lines(&script).any(|line| line.contains("1.9.1")),
        "the resolver must never fall back to appimagetool 1.9.1"
    );
    assert!(
        !script.contains("http://"),
        "the resolver must only use HTTPS"
    );
    for untrusted in ["mirror.", "cdn.", "objects.githubusercontent.com"] {
        assert!(
            !script.contains(untrusted),
            "the resolver must not use the untrusted host marker {untrusted}"
        );
    }

    let download_at = script
        .find("TOOL_LOCAL_SHA256=\"$(download_and_verify")
        .expect("the tool must be downloaded and verified");
    let chmod_at = script
        .find("chmod 0755 \"${TOOL_PATH}\"")
        .expect("the executable bit must be applied explicitly");
    assert!(
        download_at < chmod_at,
        "the executable bit must be applied only after SHA-256 verification"
    );

    let recheck_at = script
        .find("assert_release_unchanged \"${TOOL_JSON_RECHECK}\"")
        .expect("the resolver must re-check the release for a continuous race");
    assert!(
        recheck_at > download_at,
        "the release race check must happen after the download"
    );
}

#[test]
fn appimage_builder_uses_the_pinned_toolchain_without_update_information() {
    let build = read("scripts/build_linux_appimage.sh");

    for required in [
        "--runtime-file",
        "--no-appstream",
        "APPIMAGE_EXTRACT_AND_RUN=1",
        "ARCH=x86_64",
        "SPLogbook-Linux-x86_64.AppImage",
        "${APPIMAGE_PATH}.sha256",
        "APPIMAGE_TYPE_INVALID",
    ] {
        assert!(
            build.contains(required),
            "the builder must contain {required}"
        );
    }
    assert!(
        !build.contains("--updateinformation") && !build.contains("zsync"),
        "the builder must not embed update information"
    );
    assert!(
        !build.contains("--sign") && !build.contains("APPIMAGETOOL_SIGN"),
        "the builder must not sign the AppImage"
    );
    assert!(
        build.contains("portable tar.gz SHA-256 verification failed"),
        "the builder must re-verify the portable tar.gz before packaging"
    );
    assert!(
        build.contains("payload build_commit"),
        "the builder must confirm the portable build_commit"
    );
}

#[test]
fn workflow_builds_once_verifies_in_four_profiles_and_keeps_publication_disabled() {
    let workflow = read(".github/workflows/build-and-release.yml");

    assert_eq!(
        workflow.matches("distro_id: \"").count(),
        4,
        "the matrix must still contain exactly four supported profiles"
    );
    for artifact in [
        "SPLogbook-Linux-x86_64-appimage",
        "verify-appimage-negative-fixtures",
        "verify-appimage-fuse",
        "scripts/verify_appimage.sh",
        "scripts/verify_appimage_negative_fixture.sh",
        "scripts/build_linux_appimage.sh",
        "NEGATIVE FIXTURES REJECTED AS EXPECTED: 12/12",
    ] {
        assert!(
            workflow.contains(artifact),
            "the workflow must contain {artifact}"
        );
    }
    assert!(
        workflow.contains("APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY"),
        "every profile must require the AppImage success marker"
    );
    assert!(
        workflow.contains("FUSE_RUNTIME_VERIFIED=true")
            && workflow.contains("FUSE_STATUS=UNAVAILABLE_IN_CI"),
        "the FUSE job must distinguish verified FUSE from an unavailable runner"
    );

    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("publish-release must exist");
    assert!(
        !publish.contains("AppImage"),
        "publish-release must never publish the AppImage in M3-G"
    );
    assert!(
        publish.contains(
            "needs: [build-linux, build-windows, verify-linux-matrix, verify-linux-negative]"
        ),
        "publish-release must remain unchanged"
    );
}

#[test]
fn negative_fixtures_cover_every_required_mutation() {
    let script = read("scripts/verify_appimage_negative_fixture.sh");

    for fixture in [
        "missing-sha256",
        "wrong-sha256",
        "truncated-image",
        "missing-apprun",
        "apprun-not-executable",
        "missing-main-binary",
        "dangerous-desktop-exec",
        "missing-icon",
        "user-data-in-image",
        "symlink-escape",
        "wrong-build-commit",
        "glibc-above-policy",
    ] {
        assert!(
            script.contains(fixture),
            "the negative fixture suite must cover {fixture}"
        );
    }
    assert!(
        script.contains("APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY"),
        "the suite must reject any fixture that emits the success message"
    );
    assert!(
        script.contains("NEGATIVE FIXTURES REJECTED AS EXPECTED"),
        "the suite must report the explicit rejection summary"
    );
}

#[test]
fn xdg_isolation_and_portable_pipeline_are_preserved() {
    let verify = read("scripts/verify_appimage.sh");
    let portable_build = read("scripts/build_linux_portable_tar_gz.sh");

    for xdg in [
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
    ] {
        assert!(verify.contains(xdg), "the verifier must isolate {xdg}");
    }
    assert!(
        verify.contains("--check-health-startup"),
        "the verifier must execute the health check"
    );
    assert!(
        portable_build.contains("SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.tar.gz"),
        "the existing portable tar.gz pipeline must remain intact"
    );
    assert!(
        !portable_build.contains("AppImage"),
        "the portable builder must stay independent of the AppImage builder"
    );
}

#[test]
fn documentation_records_the_m3_g_gate() {
    let next_gate = read("docs/NEXT_WORK_GATE.md");
    let state = read("docs/PROJECT_CURRENT_STATE.md");

    assert!(
        next_gate.contains("STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE"),
        "the next-work-gate document must reference M3-G"
    );
    assert!(
        next_gate.contains("F03_STATUS: `CLOSED`"),
        "F-03 must remain CLOSED"
    );
    assert!(
        state.contains("M3-G"),
        "the project state must document the M3-G gate"
    );
}
