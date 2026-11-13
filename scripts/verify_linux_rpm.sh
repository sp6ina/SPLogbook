#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Fail-closed verifier of the SPLogbook Linux x86_64 binary RPM package (M3-I).
#
# Contract:
#   * every critical probe aborts on a non-zero exit or an unexpected value;
#   * no critical command is masked with a blanket success fallback;
#   * "RPM VERIFICATION COMPLETED SUCCESSFULLY" is printed ONLY after the
#     complete static, ABI, installation, upgrade, downgrade, reinstall,
#     removal and user-data-preservation checks passed;
#   * SPLOG_RPM_INSPECTION_ONLY=1 performs the static part only and prints
#     "RPM INSPECTION COMPLETED SUCCESSFULLY" instead (used by the negative
#     fixture harness, never as proof of installation).
#
# Required environment for the full run:
#   EXPECTED_BUILD_COMMIT  (40-hex commit under test)
#   EXPECTED_ICON_SHA256   (optional, source icon digest)
#   EXPECTED_DESKTOP_SHA256(optional, source desktop digest)

set -eu

GLIBC_POLICY_MAX="2.43"
EXPECTED_VERSION="${EXPECTED_APP_VERSION:-1.3.0-alpha.1}"
EXPECTED_RPM_VERSION="${EXPECTED_RPM_VERSION:-1.3.0}"
EXPECTED_RPM_RELEASE="${EXPECTED_RPM_RELEASE:-0.1.alpha.1}"
EXPECTED_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
EXPECTED_EPOCH="${EXPECTED_SOURCE_DATE_EPOCH:-}"
EXPECTED_ICON_SHA256="${EXPECTED_ICON_SHA256:-}"
EXPECTED_DESKTOP_SHA256="${EXPECTED_DESKTOP_SHA256:-}"
INSPECTION_ONLY="${SPLOG_RPM_INSPECTION_ONLY:-0}"
SKIP_RPMLINT="${SPLOG_RPM_SKIP_RPMLINT:-0}"

RPM="${1:-}"
if [ -z "${RPM}" ]; then
    printf 'FAIL: usage: %s <path-to-splogbook.rpm>\n' "$0" >&2
    exit 1
fi
[ -f "${RPM}" ] || {
    printf 'FAIL: RPM does not exist: %s\n' "${RPM}" >&2
    exit 1
}
RPM_DIR="$(cd "$(dirname "${RPM}")" && pwd)"
RPM="${RPM_DIR}/$(basename "${RPM}")"
MANIFEST="${RPM}.manifest"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-rpm-verify.XXXXXX")"
cleanup() {
    rm -rf "${WORK}"
}
trap cleanup EXIT HUP INT TERM

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}
pass() {
    printf 'PASS: %s\n' "$*"
}
log() {
    printf '%s\n' "$*"
}
require_tool() {
    command -v "$1" >/dev/null 2>&1 || fail "required tool is missing: $1"
}

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

max_glibc() {
    readelf --version-info "$1" \
        | grep -o 'GLIBC_[0-9][0-9.]*' \
        | sed 's/^GLIBC_//' \
        | sort -V \
        | tail -n 1
}

assert_elf() {
    file "$1" | grep -Eq 'ELF 64-bit.*x86-64' || fail "not a 64-bit x86-64 ELF: $1"
    readelf -h "$1" | grep -q 'Class:[[:space:]]*ELF64' || fail "ELF class is not ELF64: $1"
    readelf -h "$1" | grep -q 'Machine:.*X86-64' || fail "ELF machine is not x86-64: $1"
}

assert_runpath_exact() {
    file="$1"
    required="$2"
    runpath="$(readelf -d "${file}" | sed -n 's/.*RUNPATH.*\[\(.*\)\].*/\1/p')"
    [ -n "${runpath}" ] || fail "missing RUNPATH in ${file}"
    [ "${runpath}" = "${required}" ] \
        || fail "RUNPATH '${runpath}' in ${file}, expected exactly '${required}'"
    if readelf -d "${file}" | grep -q '(RPATH)'; then
        fail "legacy RPATH is forbidden in ${file}"
    fi
}

assert_no_forbidden_token() {
    file="$1"
    if grep -Eq '(^|[^A-Za-z0-9_])(sudo|pkexec|doas)([^A-Za-z0-9_]|$)' "${file}"; then
        fail "privilege escalation command in ${file}"
    fi
    if grep -Eq 'LD_LIBRARY_PATH|https?://|curl|wget|apt-get|dnf|zypper|systemctl|crontab' "${file}"; then
        fail "forbidden network/service token in ${file}"
    fi
    if grep -Eq '(^|[^A-Za-z0-9_])(HOME|XDG_[A-Z_]+)([^A-Za-z0-9_]|$)' "${file}"; then
        fail "HOME/XDG access is forbidden in ${file}"
    fi
}

rpm_qf() {
    rpm -qp --qf "$1" "${RPM}"
}

assert_empty_query() {
    label="$1"
    shift
    set +e
    output="$("$@" "${RPM}" 2>&1)"
    code=$?
    set -e
    [ "${code}" -eq 0 ] || fail "rpm ${label} query failed with exit ${code}: ${output}"
    [ -z "${output}" ] || fail "unexpected rpm ${label} entries: ${output}"
}

assert_mode() {
    path="$1"
    expected="$2"
    [ -e "${ROOT}${path}" ] || fail "cannot check mode of missing path: ${path}"
    actual="$(stat -c '%a' "${ROOT}${path}")"
    [ "${actual}" = "${expected}" ] \
        || fail "mode ${actual} for ${path}, expected ${expected}"
}

echo "=================================================================="
echo "SPLogbook Linux x86_64 RPM Verifier (M3-I)"
echo "Package: ${RPM}"
echo "=================================================================="

for tool in rpm rpm2cpio cpio file readelf ldd sha256sum awk sed grep sort \
    python3 desktop-file-validate find stat mktemp diff cmp; do
    require_tool "${tool}"
done
[ "$(id -u)" -eq 0 ] || fail "RPM verification requires root"
log "rpm: $(rpm --version)"
if command -v rpmbuild >/dev/null 2>&1; then
    log "rpmbuild: $(rpmbuild --version | head -n 1)"
else
    fail "required tool is missing: rpmbuild"
fi
if command -v rpmlint >/dev/null 2>&1; then
    log "rpmlint: $(rpmlint --version 2>&1 | head -n 1)"
fi

# ------------------------------------------------------------------
# 1. Checksum sidecar, package type and manifest header
# ------------------------------------------------------------------
[ -f "${RPM}.sha256" ] || fail "checksum sidecar is missing: ${RPM}.sha256"
EXPECTED_SHA="$(awk '{print $1}' "${RPM}.sha256")"
ACTUAL_SHA="$(sha256sum "${RPM}" | awk '{print $1}')"
[ "${EXPECTED_SHA}" = "${ACTUAL_SHA}" ] \
    || fail "SHA-256 sidecar does not match the RPM (${EXPECTED_SHA} != ${ACTUAL_SHA})"
pass "SHA-256 integrity verified (${ACTUAL_SHA})"

FILE_TYPE="$(file -b "${RPM}")"
printf '%s\n' "${FILE_TYPE}"
case "${FILE_TYPE}" in
    *RPM*) pass "file(1) identifies an RPM package" ;;
    *) fail "not an RPM package: ${FILE_TYPE}" ;;
esac

[ -f "${MANIFEST}" ] || fail "manifest is missing: ${MANIFEST}"
[ "$(head -n 1 "${MANIFEST}")" = "SPLOGBOOK_RPM_MANIFEST 1.0" ] \
    || fail "manifest header is invalid"
manifest_value() {
    awk -F= -v key="$1" '$1 == key { sub(/^[^=]*=/, ""); print; exit }' "${MANIFEST}"
}
[ "$(manifest_value package)" = "splogbook" ] || fail "manifest package mismatch"
[ "$(manifest_value version)" = "${EXPECTED_RPM_VERSION}" ] || fail "manifest version mismatch"
[ "$(manifest_value release)" = "${EXPECTED_RPM_RELEASE}" ] || fail "manifest release mismatch"
[ "$(manifest_value arch)" = "x86_64" ] || fail "manifest architecture mismatch"
[ "$(manifest_value artifact_sha256)" = "${ACTUAL_SHA}" ] || fail "manifest artifact SHA-256 mismatch"
[ "$(manifest_value artifact_size)" = "$(wc -c < "${RPM}" | tr -d ' ')" ] \
    || fail "manifest artifact size mismatch"
if [ -n "${EXPECTED_COMMIT}" ]; then
    [ "$(manifest_value build_commit)" = "${EXPECTED_COMMIT}" ] \
        || fail "manifest build_commit mismatch"
fi
pass "manifest header and artifact identity verified"

set +e
rpm -K "${RPM}" > "${WORK}/rpm-k.txt" 2>&1
RPM_K_CODE=$?
set -e
cat "${WORK}/rpm-k.txt"
[ "${RPM_K_CODE}" -eq 0 ] || fail "rpm -K failed with exit ${RPM_K_CODE}"
grep -q 'digests OK' "${WORK}/rpm-k.txt" || fail "rpm -K did not report digests OK"
pass "RPM header and payload digests verified"

# ------------------------------------------------------------------
# 2. Package metadata
# ------------------------------------------------------------------
[ "$(rpm_qf '%{NAME}')" = "splogbook" ] || fail "Name must be splogbook"
[ "$(rpm_qf '%{VERSION}')" = "${EXPECTED_RPM_VERSION}" ] \
    || fail "Version mismatch: $(rpm_qf '%{VERSION}')"
[ "$(rpm_qf '%{RELEASE}')" = "${EXPECTED_RPM_RELEASE}" ] \
    || fail "Release mismatch: $(rpm_qf '%{RELEASE}')"
[ "$(rpm_qf '%{ARCH}')" = "x86_64" ] || fail "Architecture must be x86_64"
[ "$(rpm_qf '%{VENDOR}')" = "SPLogbook" ] || fail "Vendor mismatch"
[ "$(rpm_qf '%{LICENSE}')" = "GPL-3.0-or-later AND LGPL-2.1-or-later" ] \
    || fail "License mismatch"
[ "$(rpm_qf '%{URL}')" = "https://github.com/sp6ina/SPLogbook" ] \
    || fail "URL mismatch"
[ "$(rpm_qf '%{SUMMARY}')" = "Amateur radio logging and station automation workstation" ] \
    || fail "Summary mismatch"
[ "$(rpm_qf '%{GROUP}')" = "Applications/Communications" ] || fail "Group mismatch"
[ "$(rpm_qf '%{BUILDHOST}')" = "build.splogbook.invalid" ] || fail "BuildHost mismatch"
EPOCH_VALUE="$(rpm_qf '%{EPOCHNUM}')"
[ -z "${EPOCH_VALUE}" ] || [ "${EPOCH_VALUE}" = "0" ] || fail "an epoch is not allowed"
[ "$(rpm_qf '%{PAYLOADFORMAT}')" = "cpio" ] || fail "payload format must be cpio"
[ "$(rpm_qf '%{PAYLOADCOMPRESSOR}')" = "xz" ] || fail "payload compressor must be xz"
if [ -n "${EXPECTED_EPOCH}" ]; then
    [ "$(rpm_qf '%{BUILDTIME}')" = "${EXPECTED_EPOCH}" ] \
        || fail "BuildTime does not match SOURCE_DATE_EPOCH"
fi
pass "RPM identity metadata is correct"

# ------------------------------------------------------------------
# 3. Dependency contract
# ------------------------------------------------------------------
EXPECTED_REQUIRES="$(cat <<'EOF'
glibc >= 2.43
libGL.so.1()(64bit)
libX11.so.6()(64bit)
libXi.so.6()(64bit)
libXcursor.so.1()(64bit)
libXrandr.so.2()(64bit)
libasound.so.2()(64bit)
libc.so.6()(64bit)
libc.so.6(GLIBC_2.43)(64bit)
libfontconfig.so.1()(64bit)
libgcc_s.so.1()(64bit)
libgtk-3.so.0()(64bit)
libm.so.6()(64bit)
libudev.so.1()(64bit)
libusb-1.0.so.0()(64bit)
libwayland-client.so.0()(64bit)
libwayland-cursor.so.0()(64bit)
libwayland-egl.so.1()(64bit)
libxkbcommon-x11.so.0()(64bit)
libxkbcommon.so.0()(64bit)
EOF
)"
ACTUAL_REQUIRES="$(rpm -qpR "${RPM}" | grep -v '^rpmlib(' | LC_ALL=C sort)"
EXPECTED_REQUIRES_SORTED="$(printf '%s\n' "${EXPECTED_REQUIRES}" | LC_ALL=C sort)"
[ "${ACTUAL_REQUIRES}" = "${EXPECTED_REQUIRES_SORTED}" ] \
    || fail "Requires set does not match the verified runtime contract"
case "$(rpm -qpR "${RPM}")" in
    *hamlib*) fail "bundled libhamlib must not be an RPM dependency" ;;
esac
set +e
RPMLIB_REQUIRES="$(rpm -qpR "${RPM}" | grep '^rpmlib(')"
RPMLIB_CODE=$?
set -e
[ "${RPMLIB_CODE}" -eq 0 ] || fail "no rpmlib requirements found"
printf '%s\n' "${RPMLIB_REQUIRES}" > "${WORK}/rpmlib-requires.txt"
while IFS= read -r line; do
    case "${line}" in
        "rpmlib(CompressedFileNames) <= 3.0.4-1"|\
        "rpmlib(FileDigests) <= 4.6.0-1"|\
        "rpmlib(PayloadFilesHavePrefix) <= 4.0-1"|\
        "rpmlib(PayloadIsXz) <= 5.2-1") ;;
        *) fail "unexpected rpmlib requirement: ${line}" ;;
    esac
done < "${WORK}/rpmlib-requires.txt"
pass "Requires match the verified runtime contract"

# ------------------------------------------------------------------
# 4. Extract and enforce the approved layout
# ------------------------------------------------------------------
ROOT="${WORK}/root"
mkdir -p "${ROOT}"
( cd "${ROOT}" && rpm2cpio "${RPM}" | cpio -idm --quiet --no-absolute-filenames ) \
    || fail "cannot extract the RPM payload"
[ -d "${ROOT}/opt/SPLogbook" ] || fail "/opt/SPLogbook is missing"
[ -z "$(find "${ROOT}" -type l -print -quit)" ] || fail "package contains symbolic links"

RPM_FILES="$(rpm -qpl "${RPM}" | LC_ALL=C sort)"
MANIFEST_FILES="$(awk '$1=="F"{print $5} $1=="D"{print $4}' "${MANIFEST}" | LC_ALL=C sort)"
[ "${RPM_FILES}" = "${MANIFEST_FILES}" ] \
    || fail "RPM file list does not match the manifest"
MANIFEST_FILE_COUNT="$(awk '$1=="F"{n++} END{print n+0}' "${MANIFEST}")"
MANIFEST_ENTRY_COUNT="$(awk '$1=="F"||$1=="D"{n++} END{print n+0}' "${MANIFEST}")"
[ "$(manifest_value file_count)" = "${MANIFEST_FILE_COUNT}" ] \
    || fail "manifest file_count is inconsistent"
[ "$(manifest_value entry_count)" = "${MANIFEST_ENTRY_COUNT}" ] \
    || fail "manifest entry_count is inconsistent"

awk '$1=="F"{print $1,$2,$3,$4,$5,$6} $1=="D"{print $1,$2,$3,$4,"-",$5}' "${MANIFEST}" \
    > "${WORK}/manifest-entries.txt"
while IFS=' ' read -r kind mode uid gid extra path; do
    case "${kind}" in
        F)
            target="${ROOT}${path}"
            [ -f "${target}" ] || fail "manifest file is missing: ${path}"
            [ "$(stat -c '%a' "${target}")" = "${mode}" ] \
                || fail "mode mismatch for ${path}"
            [ "$(stat -c '%u' "${target}")" = "${uid}" ] \
                || fail "uid mismatch for ${path}"
            [ "$(stat -c '%g' "${target}")" = "${gid}" ] \
                || fail "gid mismatch for ${path}"
            [ "$(sha256sum "${target}" | awk '{print $1}')" = "${extra}" ] \
                || fail "SHA-256 mismatch for ${path}"
            ;;
        D)
            target="${ROOT}${path}"
            [ -d "${target}" ] || fail "manifest directory is missing: ${path}"
            [ "$(stat -c '%a' "${target}")" = "${mode}" ] \
                || fail "directory mode mismatch for ${path}"
            [ "$(stat -c '%u' "${target}")" = "${uid}" ] \
                || fail "directory uid mismatch for ${path}"
            [ "$(stat -c '%g' "${target}")" = "${gid}" ] \
                || fail "directory gid mismatch for ${path}"
            ;;
        *) fail "unknown manifest entry: ${kind}" ;;
    esac
done < "${WORK}/manifest-entries.txt"

for required in \
    opt/SPLogbook/SPLogbook \
    opt/SPLogbook/splogbook-updater \
    opt/SPLogbook/lib/libhamlib.so.4 \
    opt/SPLogbook/LICENSE \
    opt/SPLogbook/.splogbook-rpm-marker \
    usr/bin/splogbook \
    usr/share/applications/splogbook.desktop \
    usr/share/icons/hicolor/256x256/apps/splogbook.png \
    usr/share/doc/splogbook/README.RPM \
    usr/share/licenses/splogbook/LICENSE \
    usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt; do
    [ -f "${ROOT}/${required}" ] || fail "required package file is missing: ${required}"
done

[ -d "${ROOT}/opt/SPLogbook/locales" ] || fail "locales/ directory is missing"
[ -d "${ROOT}/opt/SPLogbook/THIRD_PARTY_LICENSES" ] || fail "THIRD_PARTY_LICENSES/ is missing"

UNEXPECTED_OPT="$(find "${ROOT}/opt/SPLogbook" -mindepth 1 -maxdepth 1 -printf '%f\n' \
    | sort \
    | awk '!/^(SPLogbook|splogbook-updater|lib|locales|LICENSE|THIRD_PARTY_LICENSES|\.splogbook-rpm-marker)$/')"
[ -z "${UNEXPECTED_OPT}" ] || fail "unexpected entries in /opt/SPLogbook: ${UNEXPECTED_OPT}"

LIB_ENTRIES="$(find "${ROOT}/opt/SPLogbook/lib" -mindepth 1 -maxdepth 1 -printf '%f\n')"
[ "${LIB_ENTRIES}" = "libhamlib.so.4" ] \
    || fail "lib/ must contain exactly libhamlib.so.4, found: ${LIB_ENTRIES}"

EXPECTED_USR_FILES="$(cat <<'EOF'
usr/bin/splogbook
usr/share/applications/splogbook.desktop
usr/share/doc/splogbook/README.RPM
usr/share/icons/hicolor/256x256/apps/splogbook.png
usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt
usr/share/licenses/splogbook/LICENSE
EOF
)"
ACTUAL_USR_FILES="$(cd "${ROOT}" && find usr -type f -print | LC_ALL=C sort)"
[ "${ACTUAL_USR_FILES}" = "${EXPECTED_USR_FILES}" ] \
    || fail "unexpected files under usr/: ${ACTUAL_USR_FILES}"

[ -z "$(find "${ROOT}" -type l -print -quit)" ] || fail "package contains symbolic links"
[ -z "$(find "${ROOT}" \( ! -type f ! -type d \) -print -quit)" ] \
    || fail "package contains special files"
[ -z "$(find "${ROOT}" -type f -perm /6000 -print -quit)" ] \
    || fail "package contains setuid/setgid files"
[ -z "$(find "${ROOT}" -type f -perm -0002 -print -quit)" ] \
    || fail "package contains world-writable files"
OWNERS="$(find "${ROOT}" -mindepth 1 -printf '%u:%g\n' | sort -u)"
[ "${OWNERS}" = "0:0" ] || fail "package contains entries not owned by 0:0: ${OWNERS}"
assert_mode "/opt/SPLogbook" 755
assert_mode "/opt/SPLogbook/lib" 755
assert_mode "/opt/SPLogbook/locales" 755
assert_mode "/opt/SPLogbook/THIRD_PARTY_LICENSES" 755
assert_mode "/opt/SPLogbook/SPLogbook" 755
assert_mode "/opt/SPLogbook/splogbook-updater" 755
assert_mode "/opt/SPLogbook/lib/libhamlib.so.4" 644
assert_mode "/opt/SPLogbook/LICENSE" 644
assert_mode "/opt/SPLogbook/.splogbook-rpm-marker" 644
assert_mode "/usr/bin/splogbook" 755
assert_mode "/usr/share/applications/splogbook.desktop" 644
assert_mode "/usr/share/icons/hicolor/256x256/apps/splogbook.png" 644
assert_mode "/usr/share/doc/splogbook" 755
assert_mode "/usr/share/doc/splogbook/README.RPM" 644
assert_mode "/usr/share/licenses/splogbook" 755
assert_mode "/usr/share/licenses/splogbook/LICENSE" 644
assert_mode "/usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt" 644
pass "approved /opt/SPLogbook layout, ownership and modes verified"

# ------------------------------------------------------------------
# 5. Forbidden user data, secrets and bundled system libraries
# ------------------------------------------------------------------
FORBIDDEN_FILES="$(find "${ROOT}/opt" "${ROOT}/usr" -type f \
    \( -name '*.db' -o -name '*.db-wal' -o -name '*.db-shm' \
       -o -name 'default_log.db' -o -name 'station_config.json' \
       -o -name '*.adi' -o -name '*.adif' -o -name '*.cbr' \
       -o -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '*.pfx' \
       -o -name 'id_rsa*' -o -name '*.token' -o -name '*.secret' \
       -o -name '.git' -o -name '.gitignore' \
       -o -name '.splogbook-linux-portable-marker' \) -print)"
[ -z "${FORBIDDEN_FILES}" ] || fail "forbidden user data or secret in the package: ${FORBIDDEN_FILES}"

set +e
grep -rIl 'BEGIN [A-Z ]*PRIVATE KEY' "${ROOT}/opt" "${ROOT}/usr" > "${WORK}/private-key-hits.txt" 2>/dev/null
PRIVATE_KEY_CODE=$?
set -e
[ "${PRIVATE_KEY_CODE}" -le 1 ] || fail "private key scan failed"
[ ! -s "${WORK}/private-key-hits.txt" ] || fail "private key material found in the package"

BUNDLED_SYSTEM_LIBS="$(find "${ROOT}" -type f \
    \( -name 'ld-linux*' -o -name 'libc.so*' -o -name 'libm.so*' \
       -o -name 'libgcc_s.so*' -o -name 'libstdc++.so*' -o -name 'libpthread.so*' \
       -o -name 'libdl.so*' -o -name 'librt.so*' \) -print)"
[ -z "${BUNDLED_SYSTEM_LIBS}" ] || fail "system libraries must not be bundled: ${BUNDLED_SYSTEM_LIBS}"
pass "no user data, secret or system library is bundled"

# ------------------------------------------------------------------
# 6. Scriptlets and config policy
# ------------------------------------------------------------------
set +e
SCRIPTS="$(rpm -qp --scripts "${RPM}")"
SCRIPTS_CODE=$?
set -e
[ "${SCRIPTS_CODE}" -eq 0 ] || fail "rpm --scripts failed"
printf '%s\n' "${SCRIPTS}" | head -80
set +e
SCRIPT_HEADERS="$(printf '%s\n' "${SCRIPTS}" | grep -E 'scriptlet \(using ')"
SCRIPT_HEADERS_CODE=$?
set -e
[ "${SCRIPT_HEADERS_CODE}" -eq 0 ] || fail "no RPM scriptlets found"
HEADER_COUNT="$(printf '%s\n' "${SCRIPT_HEADERS}" | awk 'NF { n++ } END { print n + 0 }')"
[ "${HEADER_COUNT}" -eq 2 ] || fail "exactly two RPM scriptlets are allowed (post/postun)"
printf '%s\n' "${SCRIPT_HEADERS}" | grep -qx 'postinstall scriptlet (using /bin/sh):' \
    || fail "postinstall scriptlet is missing"
printf '%s\n' "${SCRIPT_HEADERS}" | grep -qx 'postuninstall scriptlet (using /bin/sh):' \
    || fail "postuninstall scriptlet is missing"
if printf '%s\n' "${SCRIPT_HEADERS}" | grep -Eq '^(preinstall|preuninstall|pretrans|posttrans|trigger|verify)'; then
    fail "forbidden RPM scriptlet present"
fi
printf '%s\n' "${SCRIPTS}" > "${WORK}/scripts"
assert_no_forbidden_token "${WORK}/scripts"
assert_no_forbidden_token "${ROOT}/usr/bin/splogbook"
assert_empty_query conflicts rpm -qp --conflicts
assert_empty_query obsoletes rpm -qp --obsoletes
assert_empty_query recommends rpm -qp --recommends
assert_empty_query suggests rpm -qp --suggests
assert_empty_query supplements rpm -qp --supplements
assert_empty_query enhances rpm -qp --enhances
set +e
CONFLICTS_OUT="$(rpm -qp --triggers "${RPM}" 2>&1)"
TRIGGERS_CODE=$?
set -e
[ "${TRIGGERS_CODE}" -eq 0 ] || fail "rpm --triggers failed"
[ -z "${CONFLICTS_OUT}" ] || fail "unexpected RPM triggers: ${CONFLICTS_OUT}"
set +e
CONFIG_FILES="$(rpm -qc "${RPM}" 2>&1)"
CONFIG_CODE=$?
set -e
[ "${CONFIG_CODE}" -eq 0 ] || fail "rpm -qc failed: ${CONFIG_FILES}"
[ -z "${CONFIG_FILES}" ] || fail "RPM config files are not approved: ${CONFIG_FILES}"
pass "scriptlets are minimal and no config files are packaged"

# ------------------------------------------------------------------
# 7. ELF, RUNPATH, GLIBC, ldd
# ------------------------------------------------------------------
for elf in \
    "${ROOT}/opt/SPLogbook/SPLogbook" \
    "${ROOT}/opt/SPLogbook/splogbook-updater" \
    "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4"; do
    assert_elf "${elf}"
done
file "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4" | grep -q 'shared object' \
    || fail "bundled Hamlib is not a shared object"
readelf -d "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4" | grep -q 'SONAME.*libhamlib.so.4' \
    || fail "bundled Hamlib SONAME mismatch"
assert_runpath_exact "${ROOT}/opt/SPLogbook/SPLogbook" '$ORIGIN/lib'
assert_runpath_exact "${ROOT}/opt/SPLogbook/splogbook-updater" '$ORIGIN/lib'
assert_runpath_exact "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4" '$ORIGIN'

for elf in \
    "${ROOT}/opt/SPLogbook/SPLogbook" \
    "${ROOT}/opt/SPLogbook/splogbook-updater" \
    "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4"; do
    glibc="$(max_glibc "${elf}")"
    [ -n "${glibc}" ] || fail "no GLIBC versioned symbols in ${elf}"
    version_le "${glibc}" "${GLIBC_POLICY_MAX}" \
        || fail "${elf} requires GLIBC_${glibc} > ${GLIBC_POLICY_MAX}"
    log "GLIBC max for $(basename "${elf}"): ${glibc}"
done
pass "GLIBC policy satisfied (maximum required symbol <= ${GLIBC_POLICY_MAX})"

for elf in \
    "${ROOT}/opt/SPLogbook/SPLogbook" \
    "${ROOT}/opt/SPLogbook/splogbook-updater" \
    "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4"; do
    set +e
    LDD_OUT="$(ldd "${elf}" 2>&1)"
    LDD_CODE=$?
    set -e
    printf '%s\n' "${LDD_OUT}"
    [ "${LDD_CODE}" -eq 0 ] || fail "ldd exited with ${LDD_CODE} for ${elf}"
    case "${LDD_OUT}" in
        *"not found"*) fail "ldd reports an unresolved library for ${elf}" ;;
    esac
done
pass "ldd resolves every shared library in the extracted package"

# ------------------------------------------------------------------
# 8. Launcher, desktop entry, icon and marker
# ------------------------------------------------------------------
LAUNCHER="${ROOT}/usr/bin/splogbook"
grep -qx '#!/bin/sh' "${LAUNCHER}" || fail "launcher must start with #!/bin/sh"
grep -qx 'set -eu' "${LAUNCHER}" || fail "launcher must use set -eu"
grep -qx 'exec /opt/SPLogbook/SPLogbook "$@"' "${LAUNCHER}" \
    || fail "launcher must exec the binary with all arguments"
assert_no_forbidden_token "${LAUNCHER}"
pass "launcher is the approved fail-closed POSIX sh wrapper"

DESKTOP="${ROOT}/usr/share/applications/splogbook.desktop"
desktop-file-validate "${DESKTOP}" || fail "desktop-file-validate failed"
grep -qx 'Exec=splogbook' "${DESKTOP}" || fail "desktop Exec must be exactly splogbook"
grep -qx 'Icon=splogbook' "${DESKTOP}" || fail "desktop Icon must be exactly splogbook"
grep -qx 'StartupWMClass=SPLogbook' "${DESKTOP}" || fail "desktop StartupWMClass mismatch"
if grep -q '/opt/SPLogbook/SPLogbook' "${DESKTOP}"; then
    fail "desktop file must not use the absolute /opt path"
fi
if [ -n "${EXPECTED_DESKTOP_SHA256}" ]; then
    ACTUAL_DESKTOP_SHA="$(sha256sum "${DESKTOP}" | awk '{print $1}')"
    [ "${ACTUAL_DESKTOP_SHA}" = "${EXPECTED_DESKTOP_SHA256}" ] \
        || fail "installed desktop file is not byte-identical to the source template"
fi
pass "desktop entry is valid and uses the PATH launcher"

ICON="${ROOT}/usr/share/icons/hicolor/256x256/apps/splogbook.png"
file "${ICON}" | grep -q 'PNG image data, 256 x 256' || fail "icon is not a 256x256 PNG"
if [ -n "${EXPECTED_ICON_SHA256}" ]; then
    ACTUAL_ICON_SHA="$(sha256sum "${ICON}" | awk '{print $1}')"
    [ "${ACTUAL_ICON_SHA}" = "${EXPECTED_ICON_SHA256}" ] \
        || fail "installed icon is not byte-identical to assets/icon.png"
fi
pass "icon is the canonical 256x256 PNG"

MARKER="${ROOT}/opt/SPLogbook/.splogbook-rpm-marker"
python3 - "${MARKER}" "${EXPECTED_VERSION}" "${EXPECTED_RPM_VERSION}" \
    "${EXPECTED_RPM_RELEASE}" "${EXPECTED_COMMIT}" <<'PY'
import json
import re
import sys

path, expected_version, expected_rpm, expected_release, expected_commit = sys.argv[1:6]
with open(path, encoding="utf-8") as handle:
    data = json.load(handle)

expected = {
    "marker_version": "1.0",
    "installation_type": "LinuxRpm",
    "management_mode": "SystemManaged",
    "package_version": expected_version,
    "rpm_version": expected_rpm,
    "rpm_release": expected_release,
    "resolved_minimum_kernel": "7.0",
    "maximum_glibc": "2.43",
    "artifact_format": "rpm",
    "architecture": "x86_64",
    "package_name": "splogbook",
    "target": "linux-x86_64",
    "created_by": "scripts/build_linux_rpm.sh",
}
for key, value in expected.items():
    if data.get(key) != value:
        raise SystemExit(f"FAIL: marker field {key}={data.get(key)!r}, expected {value!r}")

commit = data.get("build_commit", "")
if not re.fullmatch(r"[0-9a-f]{40}", commit):
    raise SystemExit("FAIL: marker build_commit is not 40 lowercase hex characters")
if expected_commit and commit != expected_commit:
    raise SystemExit(
        f"FAIL: marker build_commit ({commit}) differs from the CI commit ({expected_commit})"
    )
print(f"Marker build_commit: {commit}")
print(f"Marker RPM version: {data['rpm_version']}-{data['rpm_release']}")
PY
pass "RPM marker identity verified"

# ------------------------------------------------------------------
# 9. Installed size
# ------------------------------------------------------------------
INSTALLED_BYTES="$(find "${ROOT}" -type f -printf '%s\n' | awk '{ total += $1 } END { print total + 0 }')"
DECLARED_SIZE="$(rpm_qf '%{SIZE}')"
[ "${DECLARED_SIZE}" = "${INSTALLED_BYTES}" ] \
    || fail "RPM SIZE ${DECLARED_SIZE} != computed ${INSTALLED_BYTES}"
pass "RPM SIZE is consistent with the payload"

# ------------------------------------------------------------------
# 10. rpmlint
# ------------------------------------------------------------------
if [ "${SKIP_RPMLINT}" = "1" ]; then
    log "rpmlint: skipped by SPLOG_RPM_SKIP_RPMLINT=1 (compatibility profile)"
else
    require_tool rpmlint
    RPMLINT_SUPPRESSED_TAGS="dir-or-file-in-opt,custom-library-search-path"
    log "rpmlint suppressed tags (OWNER-M3-H-03 approved /opt layout): ${RPMLINT_SUPPRESSED_TAGS}"
    set +e
    rpmlint --no-tag-display-limit --suppress-tags "${RPMLINT_SUPPRESSED_TAGS}" "${RPM}" \
        > "${WORK}/rpmlint.txt" 2>&1
    RPMLINT_CODE=$?
    set -e
    cat "${WORK}/rpmlint.txt"
    RPMLINT_ERRORS="$(awk '/^E:/ { n++ } END { print n + 0 }' "${WORK}/rpmlint.txt")"
    RPMLINT_WARNINGS="$(awk '/^W:/ { n++ } END { print n + 0 }' "${WORK}/rpmlint.txt")"
    [ "${RPMLINT_ERRORS}" -eq 0 ] || fail "rpmlint reported ${RPMLINT_ERRORS} error(s)"
    if [ "${RPMLINT_CODE}" -ne 0 ]; then
        fail "rpmlint exited with unexpected code ${RPMLINT_CODE}"
    fi
    pass "rpmlint reported 0 errors (${RPMLINT_WARNINGS} warnings; suppressed: ${RPMLINT_SUPPRESSED_TAGS})"
fi

echo "------------------------------------------------------------------"
echo "STATIC VERIFICATION COMPLETED"
echo "------------------------------------------------------------------"

if [ "${INSPECTION_ONLY}" = "1" ]; then
    printf '%s\n' "RPM INSPECTION COMPLETED SUCCESSFULLY"
    exit 0
fi

# ==================================================================
# 11. Native installation, upgrade, downgrade, reinstall and erase
# ==================================================================
TEST_HOME="${WORK}/user-home"
mkdir -p \
    "${TEST_HOME}/.config/splogbook" \
    "${TEST_HOME}/.local/share/splogbook" \
    "${TEST_HOME}/.local/state/splogbook" \
    "${TEST_HOME}/.cache/splogbook" \
    "${TEST_HOME}/backups" \
    "${TEST_HOME}/exports"

printf 'USER_QSO_DATABASE\n' > "${TEST_HOME}/.local/share/splogbook/default_log.db"
printf 'USER_WAL\n' > "${TEST_HOME}/.local/share/splogbook/default_log.db-wal"
printf 'USER_SHM\n' > "${TEST_HOME}/.local/share/splogbook/default_log.db-shm"
printf '{"station":"SP6INA"}\n' > "${TEST_HOME}/.config/splogbook/station_config.json"
printf 'ADIF EXPORT\n' > "${TEST_HOME}/exports/log.adi"
printf 'CABRILLO EXPORT\n' > "${TEST_HOME}/exports/contest.cbr"
printf 'BACKUP DATA\n' > "${TEST_HOME}/backups/backup.db"
printf 'TOKEN FIXTURE\n' > "${TEST_HOME}/.local/state/splogbook/user.token"

snapshot_user_home() {
    ( cd "${TEST_HOME}" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "$1"
}
snapshot_user_home "${WORK}/user-home.before"

assert_user_home_unchanged() {
    snapshot_user_home "${WORK}/user-home.after"
    if ! cmp -s "${WORK}/user-home.before" "${WORK}/user-home.after"; then
        fail "user data changed during: $1"
    fi
    pass "user data unchanged after: $1"
}

PREV_ROOT="${WORK}/previous-root"
PREV_RPM="${WORK}/previous.rpm"
cp -a "${ROOT}/." "${PREV_ROOT}"
sed -i 's/"package_version": "[^"]*"/"package_version": "1.3.0-alpha.0"/' \
    "${PREV_ROOT}/opt/SPLogbook/.splogbook-rpm-marker"
sed -i 's/"rpm_version": "[^"]*"/"rpm_version": "1.3.0"/' \
    "${PREV_ROOT}/opt/SPLogbook/.splogbook-rpm-marker"
sed -i 's/"rpm_release": "[^"]*"/"rpm_release": "0.1.alpha.0"/' \
    "${PREV_ROOT}/opt/SPLogbook/.splogbook-rpm-marker"
sh scripts/build_linux_rpm_fixture.sh "${PREV_ROOT}" "${PREV_RPM}" \
    splogbook 1.3.0 0.1.alpha.0 x86_64 \
    || fail "cannot build the previous-version fixture"

installed_version() {
    rpm -q --qf '%{VERSION}-%{RELEASE}' splogbook
}

assert_installed_files() {
    for installed in \
        /opt/SPLogbook/SPLogbook \
        /opt/SPLogbook/splogbook-updater \
        /opt/SPLogbook/lib/libhamlib.so.4 \
        /opt/SPLogbook/.splogbook-rpm-marker \
        /usr/bin/splogbook \
        /usr/share/applications/splogbook.desktop \
        /usr/share/icons/hicolor/256x256/apps/splogbook.png \
        /usr/share/doc/splogbook/README.RPM \
        /usr/share/licenses/splogbook/LICENSE \
        /usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt; do
        [ -f "${installed}" ] || fail "installed file is missing: ${installed}"
    done
    [ -x /usr/bin/splogbook ] || fail "installed launcher is not executable"
    desktop-file-validate /usr/share/applications/splogbook.desktop \
        || fail "installed desktop file is invalid"
    if [ -n "${EXPECTED_ICON_SHA256}" ]; then
        [ "$(sha256sum /usr/share/icons/hicolor/256x256/apps/splogbook.png | awk '{print $1}')" = "${EXPECTED_ICON_SHA256}" ] \
            || fail "installed icon differs from the source icon"
    fi
    set +e
    RPM_VERIFY_OUT="$(rpm -V splogbook 2>&1)"
    RPM_VERIFY_CODE=$?
    set -e
    [ "${RPM_VERIFY_CODE}" -eq 0 ] || fail "rpm -V failed with exit ${RPM_VERIFY_CODE}"
    [ -z "${RPM_VERIFY_OUT}" ] || fail "rpm -V reports modified package files: ${RPM_VERIFY_OUT}"
}

install_current() {
    if ! rpm -i "${RPM}" > "${WORK}/rpm-install.log" 2>&1; then
        cat "${WORK}/rpm-install.log" >&2
        fail "rpm -i failed for the current package"
    fi
    [ "$(installed_version)" = "${EXPECTED_RPM_VERSION}-${EXPECTED_RPM_RELEASE}" ] \
        || fail "installed version is $(installed_version)"
    [ "$(rpm -q --qf '%{ARCH}' splogbook)" = "x86_64" ] || fail "installed arch mismatch"
    assert_installed_files
}

upgrade_current() {
    if ! rpm -U "${RPM}" > "${WORK}/rpm-upgrade.log" 2>&1; then
        cat "${WORK}/rpm-upgrade.log" >&2
        fail "rpm -U failed for the current package"
    fi
    [ "$(installed_version)" = "${EXPECTED_RPM_VERSION}-${EXPECTED_RPM_RELEASE}" ] \
        || fail "upgrade did not install the current version"
}

erase_package() {
    if ! rpm -e splogbook > "${WORK}/rpm-erase.log" 2>&1; then
        cat "${WORK}/rpm-erase.log" >&2
        fail "rpm -e failed"
    fi
    [ ! -e /opt/SPLogbook ] || fail "/opt/SPLogbook remains after erase"
    [ ! -e /usr/bin/splogbook ] || fail "/usr/bin/splogbook remains after erase"
    [ ! -e /usr/share/applications/splogbook.desktop ] || fail "desktop file remains after erase"
    [ ! -e /usr/share/doc/splogbook ] || fail "documentation remains after erase"
    [ ! -e /usr/share/licenses/splogbook ] || fail "licenses remain after erase"
    set +e
    rpm -q splogbook >/dev/null 2>&1
    RPM_QUERY_CODE=$?
    set -e
    [ "${RPM_QUERY_CODE}" -ne 0 ] || fail "package is still registered after erase"
}

check_runtime_identity() {
    OPT_BEFORE="$(cd /opt/SPLogbook && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')"

    VERSION_OUT="$(cd / && env PATH=/usr/bin:/bin HOME="${TEST_HOME}" \
        XDG_CONFIG_HOME="${TEST_HOME}/.config" XDG_DATA_HOME="${TEST_HOME}/.local/share" \
        XDG_STATE_HOME="${TEST_HOME}/.local/state" XDG_CACHE_HOME="${TEST_HOME}/.cache" \
        splogbook --version)"
    [ "${VERSION_OUT}" = "SPLogbook ${EXPECTED_VERSION}" ] \
        || fail "launcher --version returned '${VERSION_OUT}'"

    BUILD_OUT="$(cd /tmp && env PATH=/usr/bin:/bin HOME="${TEST_HOME}" \
        XDG_CONFIG_HOME="${TEST_HOME}/.config" XDG_DATA_HOME="${TEST_HOME}/.local/share" \
        XDG_STATE_HOME="${TEST_HOME}/.local/state" XDG_CACHE_HOME="${TEST_HOME}/.cache" \
        splogbook --build-info)"
    printf '%s\n' "${BUILD_OUT}"
    printf '%s\n' "${BUILD_OUT}" | grep -q "^Version: ${EXPECTED_VERSION}$" \
        || fail "--build-info version mismatch"
    printf '%s\n' "${BUILD_OUT}" | grep -q '^Commit: [0-9a-f]\{40\}$' \
        || fail "--build-info commit is missing"
    BUILD_COMMIT="$(printf '%s\n' "${BUILD_OUT}" | sed -n 's/^Commit: \([0-9a-f]\{40\}\)$/\1/p')"
    MARKER_COMMIT="$(sed -n 's/.*"build_commit": *"\([0-9a-f]\{40\}\)".*/\1/p' /opt/SPLogbook/.splogbook-rpm-marker)"
    [ "${BUILD_COMMIT}" = "${MARKER_COMMIT}" ] || fail "--build-info commit differs from the marker"
    if [ -n "${EXPECTED_COMMIT}" ]; then
        [ "${BUILD_COMMIT}" = "${EXPECTED_COMMIT}" ] || fail "--build-info commit differs from the CI commit"
    fi
    printf '%s\n' "${BUILD_OUT}" | grep -q '^Target: linux-x86_64$' \
        || fail "--build-info target mismatch"

    HEALTH_PROOF="${WORK}/health.proof"
    env HOME="${TEST_HOME}" \
        XDG_CONFIG_HOME="${TEST_HOME}/.config" XDG_DATA_HOME="${TEST_HOME}/.local/share" \
        XDG_STATE_HOME="${TEST_HOME}/.local/state" XDG_CACHE_HOME="${TEST_HOME}/.cache" \
        /usr/bin/splogbook --check-health-startup --health-proof-path "${HEALTH_PROOF}"
    [ -s "${HEALTH_PROOF}" ] || fail "health check did not create a proof file"
    [ "$(cat "${HEALTH_PROOF}")" = "OK" ] || fail "health check proof is not OK"

    OPT_AFTER="$(cd /opt/SPLogbook && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')"
    [ "${OPT_BEFORE}" = "${OPT_AFTER}" ] || fail "launching the application modified /opt/SPLogbook"
    assert_user_home_unchanged "launcher execution and health check"
    pass "launcher, --version, --build-info and health check work from arbitrary CWD"
}

echo "=== 11.1 FRESH INSTALL OF THE CURRENT VERSION ==="
install_current
assert_user_home_unchanged "fresh install"
check_runtime_identity

echo "=== 11.2 REMOVE AND FULL ERASE AFTER FRESH INSTALL ==="
erase_package
assert_user_home_unchanged "fresh install erase"

echo "=== 11.3 CONTROLLED PREVIOUS FIXTURE INSTALL ==="
if ! rpm -i "${PREV_RPM}" > "${WORK}/rpm-prev-install.log" 2>&1; then
    cat "${WORK}/rpm-prev-install.log" >&2
    fail "rpm -i failed for the previous fixture"
fi
[ "$(installed_version)" = "1.3.0-0.1.alpha.0" ] \
    || fail "previous fixture version is $(installed_version)"
[ -x /usr/bin/splogbook ] || fail "launcher is missing after the previous install"
assert_user_home_unchanged "previous fixture install"

echo "=== 11.4 UPGRADE TO THE CURRENT VERSION ==="
upgrade_current
assert_user_home_unchanged "upgrade"

echo "=== 11.5 DOWNGRADE AND RE-UPGRADE ==="
if ! rpm -U --oldpackage "${PREV_RPM}" > "${WORK}/rpm-downgrade.log" 2>&1; then
    cat "${WORK}/rpm-downgrade.log" >&2
    fail "downgrade install failed"
fi
[ "$(installed_version)" = "1.3.0-0.1.alpha.0" ] \
    || fail "downgrade did not install the previous version"
upgrade_current
assert_user_home_unchanged "downgrade and re-upgrade"

echo "=== 11.6 REINSTALL THE CURRENT VERSION ==="
if ! rpm -U --replacepkgs "${RPM}" > "${WORK}/rpm-reinstall.log" 2>&1; then
    cat "${WORK}/rpm-reinstall.log" >&2
    fail "reinstall failed"
fi
[ "$(installed_version)" = "${EXPECTED_RPM_VERSION}-${EXPECTED_RPM_RELEASE}" ] \
    || fail "reinstall changed the version"
assert_installed_files
assert_user_home_unchanged "reinstall"

echo "=== 11.7 REMOVE AND FULL ERASE ==="
erase_package
assert_user_home_unchanged "remove and full erase"

echo "=== 11.8 FINAL FRESH INSTALL AND ERASE ==="
install_current
assert_user_home_unchanged "final fresh install"
erase_package
assert_user_home_unchanged "final remove and full erase"

echo "------------------------------------------------------------------"
echo "RPM VERIFICATION COMPLETED SUCCESSFULLY"
