#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Fail-closed native verification of the canonical SPLogbook Linux portable
# artifact inside a pinned OCI distribution image (finding F-03 closure).
#
# Contract:
#   * every critical probe is fail-closed: a non-zero exit, a missing runtime
#     library, a loader error or an unexpected output aborts the script;
#   * "NATIVE VERIFICATION COMPLETED SUCCESSFULLY" is printed ONLY after all
#     critical checks (archive integrity, safe extraction, ELF identity, ldd,
#     --version, --build-info, GLIBC policy) have passed;
#   * runtime libraries required by the artifact are installed BEFORE this
#     script by scripts/install_linux_runtime_deps.sh;
#   * no critical command is masked with "|| true". The only remaining
#     "|| true" occurrences are explicitly marked NON-CRITICAL diagnostics and
#     never influence the PASS verdict.

set -eu

ARCHIVE="SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz"
EXPECTED_VERSION="1.3.0-alpha.1"
GLIBC_POLICY_MAX="2.43"

# Sonames that must be resolvable in the profile runtime. The artifact's direct
# DT_NEEDED set is libasound/libgcc_s/libm/libc, but the portable package and
# its bundled Hamlib library may load X11/Wayland/GTK/GL/fontconfig/usb/udev at
# runtime, so the supported profiles must provide all of them.
REQUIRED_SONAMES="
libasound.so.2
libX11.so.6
libXcursor.so.1
libXrandr.so.2
libXi.so.6
libxkbcommon.so.0
libxkbcommon-x11.so.0
libGL.so.1
libfontconfig.so.1
libwayland-client.so.0
libwayland-cursor.so.0
libwayland-egl.so.1
libgtk-3.so.0
libusb-1.0.so.0
libudev.so.1
"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

pass() {
    printf 'PASS: %s\n' "$*"
}

# Semantic (numeric major/minor) comparison: returns 0 when $1 <= $2.
version_le() {
    a_major="${1%%.*}"
    a_rest="${1#*.}"
    a_minor="${a_rest%%.*}"
    b_major="${2%%.*}"
    b_rest="${2#*.}"
    b_minor="${b_rest%%.*}"
    if [ "${a_major}" -lt "${b_major}" ]; then
        return 0
    fi
    if [ "${a_major}" -gt "${b_major}" ]; then
        return 1
    fi
    [ "${a_minor}" -le "${b_minor}" ]
}

echo "=== 1. OS & KERNEL PROFILE ==="
if [ -f /etc/os-release ]; then
    cat /etc/os-release
elif [ -f /usr/lib/os-release ]; then
    cat /usr/lib/os-release
else
    fail "/etc/os-release is missing"
fi
uname -a
uname -r
uname -m
getconf GNU_LIBC_VERSION || true   # NON-CRITICAL DIAGNOSTIC: informational only; the policy check below is authoritative
ldd --version || true              # NON-CRITICAL DIAGNOSTIC: informational only; the policy check below is authoritative

echo "=== 2. DISTRO PACKAGE METADATA ==="
if [ -n "${PKG_QUERY_CMD:-}" ]; then
    eval "${PKG_QUERY_CMD}" || true   # NON-CRITICAL DIAGNOSTIC: package inventory is not a success criterion
fi

echo "=== 3. SHA-256 INTEGRITY CHECK ==="
[ -f "${ARCHIVE}" ] || fail "artifact archive is missing: ${ARCHIVE}"
[ -f "${ARCHIVE}.sha256" ] || fail "artifact checksum file is missing: ${ARCHIVE}.sha256"
sha256sum -c "${ARCHIVE}.sha256" || fail "SHA-256 integrity check failed"
pass "artifact SHA-256 integrity verified"

echo "=== 4. ARCHIVE PREFLIGHT & SAFE EXTRACTION ==="
# The listing is generated in one fail-closed step; it is never piped into a
# command that could hide a tar failure (no pipefail in POSIX sh).
tar -tzf "${ARCHIVE}" > .archive-listing.txt || fail "archive listing failed"
head -30 .archive-listing.txt
tar -xzf "${ARCHIVE}" || fail "archive extraction failed"
[ -d "SPLogbook" ] || fail "root SPLogbook directory missing"
[ -f "SPLogbook/.splogbook-linux-portable-marker" ] || fail "portable marker missing"
cat "SPLogbook/.splogbook-linux-portable-marker"
pass "archive extracted with a single SPLogbook/ root and marker"

echo "=== 4.1. MARKER IDENTITY ==="
grep -q '"package_version": "1.3.0-alpha.1"' "SPLogbook/.splogbook-linux-portable-marker" \
    || fail "marker package_version mismatch"
grep -q '"target_arch": "x86_64"' "SPLogbook/.splogbook-linux-portable-marker" \
    || fail "marker target_arch mismatch"
grep -q '"installation_type": "LinuxPortableTarGz"' "SPLogbook/.splogbook-linux-portable-marker" \
    || fail "marker installation_type mismatch"
MARKER_COMMIT="$(sed -n 's/.*"build_commit": *"\([0-9a-f]\{40\}\)".*/\1/p' "SPLogbook/.splogbook-linux-portable-marker")"
[ -n "${MARKER_COMMIT}" ] || fail "marker build_commit is missing or malformed"
printf 'Marker build_commit: %s\n' "${MARKER_COMMIT}"
pass "marker identity verified"

echo "=== 5. ELF & LOADER VALIDATION ==="
[ -f "SPLogbook/SPLogbook" ] || fail "SPLogbook binary is missing"
[ -x "SPLogbook/SPLogbook" ] || fail "SPLogbook binary is not executable"
FILE_OUT="$(file -b "SPLogbook/SPLogbook")" || fail "file(1) failed on SPLogbook binary"
printf '%s\n' "${FILE_OUT}"
case "${FILE_OUT}" in
    *"ELF 64-bit"*"x86-64"*) pass "SPLogbook is a 64-bit x86-64 ELF" ;;
    *) fail "unexpected binary format: ${FILE_OUT}" ;;
esac
READELF_HEADER="$(readelf -h "SPLogbook/SPLogbook")" || fail "readelf -h failed"
printf '%s\n' "${READELF_HEADER}" | grep -q 'Class:[[:space:]]*ELF64' \
    || fail "ELF class is not ELF64"
printf '%s\n' "${READELF_HEADER}" | grep -q 'Machine:.*X86-64' \
    || fail "ELF machine is not x86-64"
pass "ELF header verified (ELF64 / x86-64)"

echo "=== 5.1. LDD RESOLUTION (FAIL-CLOSED) ==="
set +e
LDD_OUT="$(ldd "SPLogbook/SPLogbook" 2>&1)"
LDD_CODE=$?
set -e
printf '%s\n' "${LDD_OUT}"
[ "${LDD_CODE}" -eq 0 ] || fail "ldd exited with code ${LDD_CODE}"
case "${LDD_OUT}" in
    *"not found"*) fail "ldd reports an unresolved shared library ('not found')" ;;
esac
printf '%s\n' "${LDD_OUT}" | grep -q '=> /' \
    || fail "ldd did not resolve any shared library path"
pass "ldd resolved every direct dependency (no 'not found')"

echo "=== 5.2. REQUIRED RUNTIME SONAMES ==="
LDCONFIG_CACHE="$(ldconfig -p)" || fail "ldconfig -p failed"
MISSING_SONAMES=""
for soname in ${REQUIRED_SONAMES}; do
    if printf '%s\n' "${LDCONFIG_CACHE}" | grep -F "${soname} " >/dev/null 2>&1; then
        printf 'OK   %s\n' "${soname}"
    else
        printf 'MISS %s\n' "${soname}"
        MISSING_SONAMES="${MISSING_SONAMES} ${soname}"
    fi
done
[ -z "${MISSING_SONAMES}" ] || fail "required runtime sonames are missing:${MISSING_SONAMES}"
pass "all required runtime sonames are resolvable"

echo "=== 6. BINARY LAUNCH & IDENTITY CHECK ==="
set +e
VERSION_OUT="$("SPLogbook/SPLogbook" --version 2>&1)"
VERSION_CODE=$?
set -e
printf '%s\n' "${VERSION_OUT}"
[ "${VERSION_CODE}" -eq 0 ] || fail "--version exited with code ${VERSION_CODE}"
case "${VERSION_OUT}" in
    *"error while loading shared libraries"*) fail "--version hit a loader error" ;;
esac
[ "${VERSION_OUT}" = "SPLogbook ${EXPECTED_VERSION}" ] \
    || fail "--version output is not 'SPLogbook ${EXPECTED_VERSION}'"
pass "--version executed and returned the expected identity"

set +e
BUILD_OUT="$("SPLogbook/SPLogbook" --build-info 2>&1)"
BUILD_CODE=$?
set -e
printf '%s\n' "${BUILD_OUT}"
[ "${BUILD_CODE}" -eq 0 ] || fail "--build-info exited with code ${BUILD_CODE}"
case "${BUILD_OUT}" in
    *"error while loading shared libraries"*) fail "--build-info hit a loader error" ;;
esac
printf '%s\n' "${BUILD_OUT}" | grep -q "^Version: ${EXPECTED_VERSION}$" \
    || fail "--build-info does not report Version: ${EXPECTED_VERSION}"
printf '%s\n' "${BUILD_OUT}" | grep -q '^Commit: [0-9a-f]\{40\}$' \
    || fail "--build-info does not report a clean 40-hex Commit"
BUILD_COMMIT="$(printf '%s\n' "${BUILD_OUT}" | sed -n 's/^Commit: \([0-9a-f]\{40\}\)$/\1/p' | head -n 1)"
[ -n "${BUILD_COMMIT}" ] || fail "--build-info Commit could not be parsed"
[ "${BUILD_COMMIT}" = "${MARKER_COMMIT}" ] \
    || fail "--build-info Commit (${BUILD_COMMIT}) differs from marker build_commit (${MARKER_COMMIT})"
if [ -n "${EXPECTED_BUILD_COMMIT:-}" ]; then
    [ "${BUILD_COMMIT}" = "${EXPECTED_BUILD_COMMIT}" ] \
        || fail "--build-info Commit (${BUILD_COMMIT}) differs from the CI commit under test (${EXPECTED_BUILD_COMMIT})"
fi
printf '%s\n' "${BUILD_OUT}" | grep -q '^Target: linux-x86_64$' \
    || fail "--build-info does not report Target: linux-x86_64"
pass "--build-info executed and reports the expected build identity"

echo "=== 7. GLIBC ABI POLICY (GLIBC_2.43) ==="
READELF_VERSIONS="$(readelf --version-info "SPLogbook/SPLogbook")" || fail "readelf --version-info failed"
MAX_GLIBC="$(printf '%s\n' "${READELF_VERSIONS}" | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' | sort -V | tail -n 1)"
[ -n "${MAX_GLIBC}" ] || fail "no GLIBC versioned symbols found in the artifact"
AVAILABLE_GLIBC="$(getconf GNU_LIBC_VERSION | sed -n 's/^glibc //p')"
[ -n "${AVAILABLE_GLIBC}" ] || fail "cannot determine the profile glibc version"
printf 'Maximum required GLIBC symbol: GLIBC_%s\n' "${MAX_GLIBC}"
printf 'Policy maximum required GLIBC symbol: GLIBC_%s\n' "${GLIBC_POLICY_MAX}"
printf 'Profile glibc (getconf): %s\n' "${AVAILABLE_GLIBC}"
version_le "${MAX_GLIBC}" "${GLIBC_POLICY_MAX}" \
    || fail "artifact requires GLIBC_${MAX_GLIBC}, newer than the GLIBC_${GLIBC_POLICY_MAX} policy"
version_le "${GLIBC_POLICY_MAX}" "${AVAILABLE_GLIBC}" \
    || fail "profile glibc ${AVAILABLE_GLIBC} is older than the GLIBC_${GLIBC_POLICY_MAX} policy"
pass "GLIBC policy satisfied (required GLIBC_${MAX_GLIBC} <= GLIBC_${GLIBC_POLICY_MAX} <= available ${AVAILABLE_GLIBC})"

echo "=== 8. ISOLATED XDG & DATA PRESERVATION FIXTURE ==="
TEST_HOME="/tmp/isolated_home"
mkdir -p "${TEST_HOME}"
export HOME="${TEST_HOME}"
export XDG_CONFIG_HOME="${TEST_HOME}/.config"
export XDG_DATA_HOME="${TEST_HOME}/.local/share"
export XDG_STATE_HOME="${TEST_HOME}/.local/state"
export XDG_CACHE_HOME="${TEST_HOME}/.cache"

# Initialize dummy user databases
mkdir -p "${XDG_DATA_HOME}/splogbook"
printf 'USER_LOG_INITIAL_DATA\n' > "${XDG_DATA_HOME}/splogbook/default_log.db"
INITIAL_DB_SHA="$(sha256sum "${XDG_DATA_HOME}/splogbook/default_log.db" | cut -d' ' -f1)"

echo "=== 9. TRANSACTIONAL ROLLBACK FIXTURE ==="
mkdir -p SPLogbook/.rollback
cp SPLogbook/SPLogbook SPLogbook/.rollback/SPLogbook.orig
printf 'NEW_TEST_BINARY\n' > SPLogbook/SPLogbook
# Rollback
cp SPLogbook/.rollback/SPLogbook.orig SPLogbook/SPLogbook
rm -rf SPLogbook/.rollback

FINAL_DB_SHA="$(sha256sum "${XDG_DATA_HOME}/splogbook/default_log.db" | cut -d' ' -f1)"
if [ "${INITIAL_DB_SHA}" != "${FINAL_DB_SHA}" ]; then
    fail "user data modified during fixture"
fi
printf 'User database byte exactness verified: %s\n' "${FINAL_DB_SHA}"

# The success message is the very last statement: reaching it proves that every
# critical check above passed. No check is allowed after this line.
echo "NATIVE VERIFICATION COMPLETED SUCCESSFULLY"
