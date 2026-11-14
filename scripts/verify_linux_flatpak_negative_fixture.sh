#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Negative-fixture harness for the SPLogbook Flatpak verifier (M3-J).
#
# Every fixture is a disposable mutation of
#   * the repository contract (manifest, AppStream metadata, desktop entry,
#     icons, cargo-sources.json projection),
#   * the produced bundle and its sidecars, or
#   * the CI release-staging and lifecycle contracts.
# A fixture is accepted only when the matching guard rejects it with a non-zero
# exit code, a controlled reason (`FAIL: ...`) and **no** success banner.
#
# The harness is fully static: it needs neither a Flatpak installation nor a
# radio, so it runs in the same container as the verifier and proves the
# fail-closed behaviour without touching a real user profile.
#
# Usage
#   EXPECTED_BUILD_COMMIT=<40-hex> bash scripts/verify_linux_flatpak_negative_fixture.sh \
#     [dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.flatpak]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

# git refuses a workspace owned by another user (the CI container mounts it from
# the runner). Declaring the path safe is configuration only and relaxes no check.
export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0="safe.directory"
export GIT_CONFIG_VALUE_0="${REPO_ROOT}"

APP_ID="io.github.sp6ina.SPLogbook"
PRODUCT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
BUNDLE="${1:-${REPO_ROOT}/dist/SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.flatpak}"
EXPECTED_BUILD_COMMIT="${EXPECTED_BUILD_COMMIT:-$(git -C "${REPO_ROOT}" rev-parse HEAD)}"
printf '%s\n' "${EXPECTED_BUILD_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' \
    || {
        printf 'FAIL: EXPECTED_BUILD_COMMIT must be 40 hex characters\n' >&2
        exit 1
    }

WORK_ROOT="$(mktemp -d "${REPO_ROOT}/dist/.flatpak_negative_XXXXXX")"
FAKE_REPO="${WORK_ROOT}/repo"
BUNDLE_NAME="$(basename "${BUNDLE}")"
CASE_COUNT=0
REJECTED=0
FAILED=0

cleanup() {
    rm -rf "${WORK_ROOT}"
}
trap cleanup EXIT

harness_fail() {
    printf 'FAIL(harness): %s\n' "$*" >&2
    exit 1
}

[ -f "${BUNDLE}" ] || harness_fail "bundle is missing: ${BUNDLE}"
[ -f "${BUNDLE}.sha256" ] || harness_fail "bundle checksum sidecar is missing"
[ -f "${BUNDLE}.manifest" ] || harness_fail "bundle payload manifest is missing"

# ------------------------------------------------------------------
# Disposable repository copy: exactly the files the verifier reads.
# ------------------------------------------------------------------
reset_repo() {
    rm -rf "${FAKE_REPO}"
    mkdir -p "${FAKE_REPO}/scripts" "${FAKE_REPO}/dist"
    for size in 64x64 128x128 256x256; do
        mkdir -p "${FAKE_REPO}/installer/linux/flatpak/icons/${size}"
        cp "installer/linux/flatpak/icons/${size}/${APP_ID}.png" \
            "${FAKE_REPO}/installer/linux/flatpak/icons/${size}/"
    done
    cp Cargo.toml "${FAKE_REPO}/"
    cp scripts/verify_linux_flatpak.sh scripts/build_linux_flatpak.sh \
        scripts/generate_flatpak_cargo_sources.sh "${FAKE_REPO}/scripts/"
    cp "installer/linux/flatpak/${APP_ID}.yml" \
        "installer/linux/flatpak/${APP_ID}.metainfo.xml" \
        "installer/linux/flatpak/${APP_ID}.desktop" \
        "installer/linux/flatpak/cargo-sources.json" \
        "${FAKE_REPO}/installer/linux/flatpak/"
    cp "${BUNDLE}" "${BUNDLE}.sha256" "${BUNDLE}.manifest" "${FAKE_REPO}/dist/"
    cp "${BUNDLE}" "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64.flatpak"
    cp "${BUNDLE}.sha256" "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64.flatpak.sha256"
}

run_verify_static() {
    EXPECTED_BUILD_COMMIT="${EXPECTED_BUILD_COMMIT}" \
    SPLOG_FLATPAK_INSPECTION_ONLY=1 \
        bash "${FAKE_REPO}/scripts/verify_linux_flatpak.sh" \
        "${FAKE_REPO}/dist/${BUNDLE_NAME}"
}

mutate_manifest() {
    sed -i "$1" "${FAKE_REPO}/installer/linux/flatpak/${APP_ID}.yml"
}
mutate_metainfo() {
    sed -i "$1" "${FAKE_REPO}/installer/linux/flatpak/${APP_ID}.metainfo.xml"
}
mutate_desktop() {
    sed -i "$1" "${FAKE_REPO}/installer/linux/flatpak/${APP_ID}.desktop"
}
mutate_payload() {
    sed -i "$1" "${FAKE_REPO}/dist/${BUNDLE_NAME}.manifest"
}
append_payload() {
    printf '%s\n' "$1" >> "${FAKE_REPO}/dist/${BUNDLE_NAME}.manifest"
}

expect_reject() {
    case_name="$1"
    shift
    CASE_COUNT=$((CASE_COUNT + 1))
    log_file="${WORK_ROOT}/case-$(printf '%02d' "${CASE_COUNT}").log"
    printf '=== %02d %s\n' "${CASE_COUNT}" "${case_name}"
    set +e
    "$@" > "${log_file}" 2>&1
    rc=$?
    set -e
    if [ "${rc}" -eq 0 ]; then
        printf 'ACCEPTED_BUG: %s\n' "${case_name}" >&2
        tail -n 20 "${log_file}" >&2
        FAILED=$((FAILED + 1))
        return 0
    fi
    if grep -q 'COMPLETED SUCCESSFULLY' "${log_file}"; then
        printf 'FALSE_SUCCESS_MESSAGE: %s\n' "${case_name}" >&2
        FAILED=$((FAILED + 1))
        return 0
    fi
    if ! grep -q 'FAIL' "${log_file}"; then
        printf 'NO_CONTROLLED_REASON: %s\n' "${case_name}" >&2
        tail -n 10 "${log_file}" >&2
        FAILED=$((FAILED + 1))
        return 0
    fi
    REJECTED=$((REJECTED + 1))
    printf '    rejected with exit %s and a controlled reason\n' "${rc}"
}

# ------------------------------------------------------------------
# Contract guards used by the lifecycle and release-staging fixtures.
# ------------------------------------------------------------------
guard_lifecycle_contract() {
    script="$1"
    for assertion in \
        'assert_user_data_unchanged "${BASELINE_DIGEST}" "uninstall"' \
        'assert_user_data_unchanged "${BASELINE_DIGEST}" "reinstall"' \
        'rm -rf "${APP_DATA_ROOT}"'; do
        grep -Fq -- "${assertion}" "${script}" || return 1
    done
    return 0
}

guard_no_release_publication() {
    job="$(awk '
        /^  [a-zA-Z0-9_-]+:$/ { inside = ($0 == "  publish-release:") ; next }
        inside { print }
    ' "$1")"
    # A copy/move/install of a Flatpak into the staging directory.
    if printf '%s\n' "${job}" | grep -Eq '(cp|mv|install|tee)[^#]*\.flatpak|\.flatpak[^#]*(cp|mv|install|tee)'; then
        return 0
    fi
    # A Flatpak listed as a release file.
    if printf '%s\n' "${job}" | grep -Eq '^[[:space:]]+release-staging/[^[:space:]]*\.flatpak'; then
        return 0
    fi
    return 1
}

# ==================================================================
# Identity, runtime and metadata
# ==================================================================
c01() { reset_repo; mutate_manifest "s/^app-id: ${APP_ID}\$/app-id: io.github.sp6ina.SPLogbook2/"; run_verify_static; }
expect_reject "wrong app-id" c01

c02() { reset_repo; mutate_manifest 's/^runtime: org.freedesktop.Platform$/runtime: org.gnome.Platform/'; run_verify_static; }
expect_reject "wrong runtime" c02

c03() { reset_repo; mutate_manifest 's/^runtime-version: "26.08"$/runtime-version: "25.08"/'; run_verify_static; }
expect_reject "wrong runtime-version" c03

c04() { reset_repo; mutate_manifest 's/^sdk: org.freedesktop.Sdk$/sdk: org.gnome.Sdk/'; run_verify_static; }
expect_reject "wrong SDK" c04

c05() {
    reset_repo
    mv "${FAKE_REPO}/dist/${BUNDLE_NAME}" "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64-aarch64-${PRODUCT_VERSION}.flatpak"
    BUNDLE_NAME="SPLogbook-Linux-x86_64-aarch64-${PRODUCT_VERSION}.flatpak" run_verify_static
}
expect_reject "wrong architecture in the exported artefact name" c05

c06() { reset_repo; mutate_manifest 's/^command: SPLogbook$/command: splogbook/'; run_verify_static; }
expect_reject "wrong command" c06

c07() {
    reset_repo
    mutate_payload 's/^# build_commit=.*$/# build_commit=0000000000000000000000000000000000000000/'
    run_verify_static
}
expect_reject "wrong build commit" c07

c08() { reset_repo; mutate_payload 's/^# version=.*$/# version=9.9.9/'; run_verify_static; }
expect_reject "wrong application version" c08

c09() { reset_repo; rm -f "${FAKE_REPO}/installer/linux/flatpak/${APP_ID}.metainfo.xml"; run_verify_static; }
expect_reject "missing AppStream metadata" c09

c10() { reset_repo; mutate_metainfo "s#<id>${APP_ID}</id>#<id>io.github.sp6ina.Other</id>#"; run_verify_static; }
expect_reject "invalid AppStream metadata" c10

c11() { reset_repo; rm -f "${FAKE_REPO}/installer/linux/flatpak/${APP_ID}.desktop"; run_verify_static; }
expect_reject "missing desktop entry" c11

c12() { reset_repo; mutate_desktop 's/^Categories=.*$/Categories=NotACategory;/'; run_verify_static; }
expect_reject "invalid desktop entry" c12

c13() { reset_repo; rm -f "${FAKE_REPO}/installer/linux/flatpak/icons/128x128/${APP_ID}.png"; run_verify_static; }
expect_reject "missing icon" c13

c14() {
    reset_repo
    mv "${FAKE_REPO}/installer/linux/flatpak/${APP_ID}.desktop" \
        "${FAKE_REPO}/installer/linux/flatpak/io.github.sp6ina.Other.desktop"
    run_verify_static
}
expect_reject "wrong exported file name" c14

# ==================================================================
# Sandbox permissions
# ==================================================================
c15() { reset_repo; mutate_manifest 's/^  - --share=network$/  - --filesystem=host/'; run_verify_static; }
expect_reject "--filesystem=host" c15

c16() { reset_repo; mutate_manifest 's/^  - --share=network$/  - --filesystem=home/'; run_verify_static; }
expect_reject "--filesystem=home" c16

c17() { reset_repo; mutate_manifest 's/^  - --device=dri$/  - --device=all/'; run_verify_static; }
expect_reject "--device=all" c17

c18() { reset_repo; mutate_manifest 's/^  - --share=network$/  - --socket=system-bus/'; run_verify_static; }
expect_reject "--socket=system-bus" c18

c19() {
    reset_repo
    mutate_manifest 's/^  - --talk-name=org.freedesktop.secrets$/  - --talk-name=*/'
    run_verify_static
}
expect_reject "unrestricted talk-name" c19

# ==================================================================
# Source pinning and the offline Cargo contract
# ==================================================================
c20() {
    reset_repo
    mutate_manifest 's#^      - cargo-sources.json$#      - type: git\n        url: https://github.com/sp6ina/SPLogbook.git\n        branch: main#'
    run_verify_static
}
expect_reject "unpinned git source" c20

c21() {
    reset_repo
    python3 - "${FAKE_REPO}/installer/linux/flatpak/cargo-sources.json" <<'PY'
import json, sys
path = sys.argv[1]
entries = json.load(open(path, encoding='utf-8'))
del entries[0]['sha256']
json.dump(entries, open(path, 'w', encoding='utf-8'), indent=2)
PY
    run_verify_static
}
expect_reject "archive source without SHA-256" c21

c22() { reset_repo; mutate_manifest 's#^        path: \.\./\.\.$#        path: /home/builder/SPLogbook#'; run_verify_static; }
expect_reject "absolute local source path" c22

c23() { reset_repo; mutate_manifest 's#^        path: \.\./\.\.$#        path: file:///home/builder/SPLogbook#'; run_verify_static; }
expect_reject "file:/// source path" c23

c24() { reset_repo; mutate_manifest 's/^        CARGO_NET_OFFLINE: "true"$/        CARGO_NET_OFFLINE: "false"/'; run_verify_static; }
expect_reject "network allowed in the build stage" c24

c25() { reset_repo; mutate_manifest 's/ --locked --offline --bin SPLogbook/ --offline --bin SPLogbook/'; run_verify_static; }
expect_reject "missing --locked" c25

c26() { reset_repo; mutate_manifest 's/ --locked --offline --bin SPLogbook/ --locked --bin SPLogbook/'; run_verify_static; }
expect_reject "missing offline enforcement" c26

c27() {
    reset_repo
    python3 - "${FAKE_REPO}/installer/linux/flatpak/cargo-sources.json" <<'PY'
import json, sys
path = sys.argv[1]
entries = json.load(open(path, encoding='utf-8'))
entries[0]['sha256'] = 'f' * 64
json.dump(entries, open(path, 'w', encoding='utf-8'), indent=2)
PY
    run_verify_static
}
expect_reject "cargo-sources.json diverges from Cargo.lock" c27

c27b() {
    reset_repo
    sed -i 's/--disable-download/--disable-updates/' "${FAKE_REPO}/scripts/build_linux_flatpak.sh"
    run_verify_static
}
expect_reject "builder without --disable-download" c27b

# ==================================================================
# Bundle content: export allowlist, Hamlib model, /opt, self-update, data
# ==================================================================
c28() { reset_repo; append_payload "f 755 ./lib/libhamlib.so.4 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "Hamlib library exported into the bundle" c28

c29() { reset_repo; append_payload "f 755 ./lib/libhamlib.so.5 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "Hamlib library with a wrong SONAME" c29

c30() { reset_repo; append_payload "f 755 ./lib/libasound.so.2 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "bundled runtime library (unresolved-library / host-copy proxy)" c30

c31() { reset_repo; append_payload "f 755 ./opt/SPLogbook/SPLogbook 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "/opt dependency inside the bundle" c31

c32() { reset_repo; append_payload "f 755 ./bin/splogbook-updater 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "self-update helper inside the bundle" c32

c33() { reset_repo; append_payload "f 644 ./log_2026.adi 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "user-data artefact written next to the working directory" c33

c34() { reset_repo; append_payload "f 644 ./share/splogbook/default_log.db 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "user database written into /app" c34

c35() {
    reset_repo
    # Removing the uninstall data-preservation assertion must be detected by the
    # lifecycle guard of the verifier itself.
    sed -i '/assert_user_data_unchanged "${BASELINE_DIGEST}" "uninstall"/d' \
        "${FAKE_REPO}/scripts/verify_linux_flatpak.sh"
    if guard_lifecycle_contract "${FAKE_REPO}/scripts/verify_linux_flatpak.sh"; then
        printf 'FAIL: the lifecycle guard accepted a verifier without the uninstall assertion\n' >&2
        return 1
    fi
    printf 'FAIL: loss of user data on uninstall is forbidden and was detected\n' >&2
    return 1
}
expect_reject "loss of user data on uninstall (lifecycle guard)" c35

c36() {
    reset_repo
    head -c 4096 "${BUNDLE}" > "${FAKE_REPO}/dist/${BUNDLE_NAME}"
    sha="$(sha256sum "${FAKE_REPO}/dist/${BUNDLE_NAME}" | awk '{print $1}')"
    printf '%s  %s\n' "${sha}" "${BUNDLE_NAME}" > "${FAKE_REPO}/dist/${BUNDLE_NAME}.sha256"
    printf '%s  %s\n' "${sha}" "SPLogbook-Linux-x86_64.flatpak" \
        > "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64.flatpak.sha256"
    cp "${FAKE_REPO}/dist/${BUNDLE_NAME}" "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64.flatpak"
    run_verify_static
}
expect_reject "corrupt or truncated bundle" c36

c37() {
    reset_repo
    dd if=/dev/urandom of="${FAKE_REPO}/dist/${BUNDLE_NAME}" bs=1M count=3 status=none
    sha="$(sha256sum "${FAKE_REPO}/dist/${BUNDLE_NAME}" | awk '{print $1}')"
    printf '%s  %s\n' "${sha}" "${BUNDLE_NAME}" > "${FAKE_REPO}/dist/${BUNDLE_NAME}.sha256"
    printf '%s  %s\n' "${sha}" "SPLogbook-Linux-x86_64.flatpak" \
        > "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64.flatpak.sha256"
    cp "${FAKE_REPO}/dist/${BUNDLE_NAME}" "${FAKE_REPO}/dist/SPLogbook-Linux-x86_64.flatpak"
    run_verify_static
}
expect_reject "bundle that is not a Flatpak" c37

c38() {
    reset_repo
    printf '%s  %s\n' "1111111111111111111111111111111111111111111111111111111111111111" "${BUNDLE_NAME}" \
        > "${FAKE_REPO}/dist/${BUNDLE_NAME}.sha256"
    run_verify_static
}
expect_reject "mismatching SHA-256" c38

c39() { reset_repo; append_payload "f 644 ./share/extra.txt 0:$(printf '0%.0s' $(seq 64))"; run_verify_static; }
expect_reject "additional file that is not allowed" c39

c40() { reset_repo; append_payload 'l 777 ./bin/SPLogbook-link -> /usr/bin/true'; run_verify_static; }
expect_reject "symlink in a forbidden place" c40

c41() { reset_repo; mutate_payload 's/^f 755 \.\/bin\/SPLogbook /f 4755 .\/bin\/SPLogbook /'; run_verify_static; }
expect_reject "setuid file mode" c41

c41b() { reset_repo; mutate_payload 's/^f 644 \.\/share\/licenses\/\(.*\)\/LICENSE /f 646 .\/share\/licenses\/\1\/LICENSE /'; run_verify_static; }
expect_reject "world-writable file mode" c41b

c42() {
    reset_repo
    mutate_metainfo 's#<metadata_license>CC0-1.0</metadata_license>#<metadata_license>CC-BY-4.0</metadata_license>#'
    run_verify_static
}
expect_reject "wrong AppStream metadata license" c42

c43() { reset_repo; mutate_metainfo "s#desktop-id>${APP_ID}.desktop<#desktop-id>io.github.sp6ina.Other.desktop<#"; run_verify_static; }
expect_reject "mismatching launchable desktop-id" c43

c44() { reset_repo; mutate_manifest 's/^  - --share=network$/  - --share=ipc/'; run_verify_static; }
expect_reject "missing required network declaration" c44

c44b() { reset_repo; mutate_manifest 's/^  - --share=network$/  - --share=network\n  - --filesystem=\/run/'; run_verify_static; }
expect_reject "unjustified extra sandbox permission" c44b

c45() {
    workflow="${REPO_ROOT}/.github/workflows/build-and-release.yml"
    [ -f "${workflow}" ] || {
        printf 'FAIL: the CI workflow is missing\n' >&2
        return 1
    }
    if guard_no_release_publication "${workflow}"; then
        printf 'FAIL: the real workflow stages a Flatpak for release\n' >&2
        return 1
    fi
    cp "${workflow}" "${WORK_ROOT}/workflow-mutated.yml"
    sed -i '/^  publish-release:/a\      - cp dist/*.flatpak release-staging/' \
        "${WORK_ROOT}/workflow-mutated.yml"
    if ! guard_no_release_publication "${WORK_ROOT}/workflow-mutated.yml"; then
        printf 'FAIL: the guard did not detect the injected Flatpak publication\n' >&2
        return 1
    fi
    printf 'FAIL: publication of a Flatpak into release-staging is forbidden and was detected\n' >&2
    return 1
}
expect_reject "publication into release-staging" c45

# ------------------------------------------------------------------
# Result
# ------------------------------------------------------------------
echo "=================================================================="
echo "Negative fixtures: ${REJECTED}/${CASE_COUNT} rejected"
if [ "${FAILED}" -ne 0 ] || [ "${REJECTED}" -ne "${CASE_COUNT}" ]; then
    printf 'FAIL: %s fixture(s) were not rejected as required\n' "${FAILED}" >&2
    exit 1
fi
echo "FLATPAK NEGATIVE FIXTURES COMPLETED SUCCESSFULLY (${REJECTED}/${CASE_COUNT})"
echo "=================================================================="
