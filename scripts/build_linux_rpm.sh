#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Deterministic builder of the SPLogbook Linux x86_64 binary RPM package (M3-I).
#
# Contract:
#   * input is the verified portable payload
#     SPLogbook-Linux-x86_64-<version>.tar.gz plus its .sha256 sidecar;
#   * the payload build_commit MUST equal EXPECTED_BUILD_COMMIT;
#   * the package root is assembled from an explicit allowlist only;
#   * no user data, no secret, no symlink, no setuid/setgid is ever copied;
#   * rpmbuild uses the reviewed spec, root ownership and a fixed xz payload;
#   * SOURCE_DATE_EPOCH, a fixed build host and clamped mtimes make the output
#     reproducible;
#   * the package is built twice and the two RPMs are compared byte-for-byte;
#   * the RPM is never installed on the build host and never published.
#
# Usage:
#   EXPECTED_BUILD_COMMIT=<40-hex> SOURCE_DATE_EPOCH=<epoch> \
#     bash scripts/build_linux_rpm.sh [output-dir] [portable.tar.gz]
#
# The builder does not modify the repository, Cargo.toml, Cargo.lock or src/.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

OUTPUT_DIR="${1:-${REPO_ROOT}/dist}"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"

GLIBC_POLICY_MAX="2.43"
RESOLVED_MIN_KERNEL="7.0"
RPM_RELEASE_OVERRIDE="${RPM_RELEASE:-}"
RPM_COMPRESSION="w6.xzdio"
SPEC_SOURCE="${REPO_ROOT}/installer/linux/rpm/splogbook.spec"
MANIFEST_TOOL="${REPO_ROOT}/scripts/linux_rpm_manifest.sh"

log() { printf '%s\n' "$*"; }
fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
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

vercmp_is() {
    left="$1"
    right="$2"
    expected_code="$3"
    set +e
    rpmdev-vercmp "${left}" "${right}" >/dev/null 2>&1
    code=$?
    set -e
    [ "${code}" -eq "${expected_code}" ] \
        || fail "rpmdev-vercmp ${left} ${right} exited ${code}, expected ${expected_code}"
}

echo "=================================================================="
echo "SPLogbook Linux x86_64 RPM Builder (M3-I)"
echo "=================================================================="

for tool in rpm rpmbuild rpmspec rpmdev-vercmp rpm2cpio cpio sha256sum tar gzip \
    find awk sed readelf file install touch sort grep date mktemp xargs wc head \
    cmp diff uname; do
    require_tool "${tool}"
done

log "rpm: $(rpm --version)"
log "rpmbuild: $(rpmbuild --version | head -n 1)"
log "rpmspec: $(rpmspec --version | head -n 1)"

[ "$(uname -m)" = "x86_64" ] || fail "the builder must run on an x86_64 host"
[ "$(rpm --eval '%{_arch}')" = "x86_64" ] || fail "the RPM build architecture must be x86_64"

# ------------------------------------------------------------------
# 1. Version identity (read from the repository, never modified)
# ------------------------------------------------------------------
PRODUCT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
[ -n "${PRODUCT_VERSION}" ] || fail "cannot read package version from Cargo.toml"
case "${PRODUCT_VERSION}" in
    [0-9]*.[0-9]*.[0-9]*-[0-9A-Za-z.-]*) ;;
    *) fail "unsupported development version format: ${PRODUCT_VERSION}" ;;
esac

BASE_VERSION="${PRODUCT_VERSION%%-*}"
PRERELEASE="${PRODUCT_VERSION#*-}"
RPM_VERSION="${BASE_VERSION}"
if [ -n "${RPM_RELEASE_OVERRIDE}" ]; then
    RPM_RELEASE="${RPM_RELEASE_OVERRIDE}"
else
    RPM_RELEASE="0.1.${PRERELEASE}"
fi
RPM_RELEASE="${RPM_RELEASE//-/.}"
printf '%s\n' "${RPM_VERSION}" | grep -Eq '^[0-9][0-9A-Za-z.+~_]*$' \
    || fail "invalid RPM Version: ${RPM_VERSION}"
printf '%s\n' "${RPM_RELEASE}" | grep -Eq '^[0-9][0-9A-Za-z.+~_]*$' \
    || fail "invalid RPM Release: ${RPM_RELEASE}"
[ "${RPM_VERSION#*:}" = "${RPM_VERSION}" ] || fail "an epoch is not allowed"
[ "${RPM_RELEASE#*:}" = "${RPM_RELEASE}" ] || fail "an epoch is not allowed"
log "Application version: ${PRODUCT_VERSION}"
log "RPM version: ${RPM_VERSION}-${RPM_RELEASE}"

CURRENT_EVR="${RPM_VERSION}-${RPM_RELEASE}"
vercmp_is "${CURRENT_EVR}" "1.3.0-1" 11
vercmp_is "${CURRENT_EVR}" "1.3.0-0.1.alpha.0" 12
vercmp_is "${CURRENT_EVR}" "1.3.0-0.1.alpha.2" 11
vercmp_is "${CURRENT_EVR}" "${CURRENT_EVR}" 0
log "PASS: RPM version ordering verified with rpmdev-vercmp"

# ------------------------------------------------------------------
# 2. Build identity and reproducibility epoch
# ------------------------------------------------------------------
EXPECTED_BUILD_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
if [ -z "${EXPECTED_BUILD_COMMIT}" ]; then
    command -v git >/dev/null 2>&1 || fail "EXPECTED_BUILD_COMMIT is required when git is absent"
    EXPECTED_BUILD_COMMIT="$(git -C "${REPO_ROOT}" rev-parse HEAD)"
fi
printf '%s\n' "${EXPECTED_BUILD_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' \
    || fail "EXPECTED_BUILD_COMMIT must be a 40-character lowercase hex commit"
log "Expected build commit: ${EXPECTED_BUILD_COMMIT}"

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-}"
if [ -z "${SOURCE_DATE_EPOCH}" ]; then
    command -v git >/dev/null 2>&1 || fail "SOURCE_DATE_EPOCH is required when git is absent"
    SOURCE_DATE_EPOCH="$(git -C "${REPO_ROOT}" log -1 --format=%ct "${EXPECTED_BUILD_COMMIT}")"
fi
printf '%s\n' "${SOURCE_DATE_EPOCH}" | grep -Eq '^[0-9]+$' \
    || fail "SOURCE_DATE_EPOCH must be a non-negative integer"
export SOURCE_DATE_EPOCH
log "SOURCE_DATE_EPOCH: ${SOURCE_DATE_EPOCH}"

# ------------------------------------------------------------------
# 3. Verified portable payload
# ------------------------------------------------------------------
PAYLOAD_ARCHIVE="${2:-${OUTPUT_DIR}/SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.tar.gz}"
[ -f "${PAYLOAD_ARCHIVE}" ] || fail "portable payload is missing: ${PAYLOAD_ARCHIVE}"
[ -f "${PAYLOAD_ARCHIVE}.sha256" ] || fail "payload checksum is missing: ${PAYLOAD_ARCHIVE}.sha256"

EXPECTED_PAYLOAD_SHA="$(awk '{print $1}' "${PAYLOAD_ARCHIVE}.sha256")"
ACTUAL_PAYLOAD_SHA="$(sha256sum "${PAYLOAD_ARCHIVE}" | awk '{print $1}')"
[ "${EXPECTED_PAYLOAD_SHA}" = "${ACTUAL_PAYLOAD_SHA}" ] \
    || fail "portable payload SHA-256 mismatch"
log "PASS: portable payload SHA-256 verified (${ACTUAL_PAYLOAD_SHA})"

WORK_ROOT="$(mktemp -d "${OUTPUT_DIR}/.rpm_build_XXXXXX")"
cleanup() {
    rm -rf "${WORK_ROOT}"
}
trap cleanup EXIT HUP INT TERM

mkdir -p "${WORK_ROOT}/payload"
tar -xzf "${PAYLOAD_ARCHIVE}" -C "${WORK_ROOT}/payload"
PAYLOAD="${WORK_ROOT}/payload/SPLogbook"
[ -d "${PAYLOAD}" ] || fail "payload does not contain a single SPLogbook/ root"
[ -z "$(find "${WORK_ROOT}/payload" -mindepth 1 -maxdepth 1 ! -name SPLogbook -print -quit)" ] \
    || fail "payload contains entries outside the SPLogbook/ root"
[ -z "$(find "${PAYLOAD}" -type l -print -quit)" ] \
    || fail "payload contains symbolic links"

for required in SPLogbook splogbook-updater lib/libhamlib.so.4 LICENSE \
    .splogbook-linux-portable-marker locales THIRD_PARTY_LICENSES; do
    [ -e "${PAYLOAD}/${required}" ] || fail "payload is missing ${required}"
done

PAYLOAD_MARKER="${PAYLOAD}/.splogbook-linux-portable-marker"
PAYLOAD_COMMIT="$(sed -n 's/.*"build_commit": *"\([0-9a-f]\{40\}\)".*/\1/p' "${PAYLOAD_MARKER}")"
[ -n "${PAYLOAD_COMMIT}" ] || fail "payload marker has no valid build_commit"
[ "${PAYLOAD_COMMIT}" = "${EXPECTED_BUILD_COMMIT}" ] \
    || fail "payload build_commit (${PAYLOAD_COMMIT}) differs from EXPECTED_BUILD_COMMIT (${EXPECTED_BUILD_COMMIT})"
grep -q "\"package_version\": \"${PRODUCT_VERSION}\"" "${PAYLOAD_MARKER}" \
    || fail "payload marker package_version mismatch"
grep -q '"installation_type": "LinuxPortableTarGz"' "${PAYLOAD_MARKER}" \
    || fail "payload marker installation_type mismatch"
log "PASS: payload marker identity verified"

[ -f "${PAYLOAD}/lib/libhamlib.so.4" ] || fail "payload is missing lib/libhamlib.so.4"
UNEXPECTED_LIBS="$(find "${PAYLOAD}/lib" -maxdepth 1 -type f ! -name 'libhamlib.so*' -printf '%f\n')"
[ -z "${UNEXPECTED_LIBS}" ] \
    || fail "payload lib/ contains unexpected libraries: ${UNEXPECTED_LIBS}"

# ------------------------------------------------------------------
# 4. ELF / RUNPATH / GLIBC preflight of the payload
# ------------------------------------------------------------------
for elf in "${PAYLOAD}/SPLogbook" "${PAYLOAD}/splogbook-updater" "${PAYLOAD}/lib/libhamlib.so.4"; do
    assert_elf "${elf}"
done
assert_runpath_exact "${PAYLOAD}/SPLogbook" '$ORIGIN/lib'
assert_runpath_exact "${PAYLOAD}/splogbook-updater" '$ORIGIN/lib'
assert_runpath_exact "${PAYLOAD}/lib/libhamlib.so.4" '$ORIGIN'
for elf in "${PAYLOAD}/SPLogbook" "${PAYLOAD}/splogbook-updater" "${PAYLOAD}/lib/libhamlib.so.4"; do
    glibc="$(max_glibc "${elf}")"
    [ -n "${glibc}" ] || fail "no GLIBC versioned symbols in ${elf}"
    version_le "${glibc}" "${GLIBC_POLICY_MAX}" \
        || fail "${elf} requires GLIBC_${glibc} > ${GLIBC_POLICY_MAX}"
    log "PASS: $(basename "${elf}") requires at most GLIBC_${glibc}"
done

# ------------------------------------------------------------------
# 5. Package root (explicit allowlist, approved /opt/SPLogbook layout)
# ------------------------------------------------------------------
PKG_ROOT="${WORK_ROOT}/package-root"
SPLOGBOOK_ROOT="${PKG_ROOT}/opt/SPLogbook"
mkdir -p "${SPLOGBOOK_ROOT}/lib" "${SPLOGBOOK_ROOT}/locales" \
    "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES"
mkdir -p "${PKG_ROOT}/usr/bin" \
    "${PKG_ROOT}/usr/share/applications" \
    "${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps" \
    "${PKG_ROOT}/usr/share/doc/splogbook" \
    "${PKG_ROOT}/usr/share/licenses/splogbook"

install -m 0755 "${PAYLOAD}/SPLogbook" "${SPLOGBOOK_ROOT}/SPLogbook"
install -m 0755 "${PAYLOAD}/splogbook-updater" "${SPLOGBOOK_ROOT}/splogbook-updater"
install -m 0644 "${PAYLOAD}/lib/libhamlib.so.4" "${SPLOGBOOK_ROOT}/lib/libhamlib.so.4"
install -m 0644 "${PAYLOAD}/LICENSE" "${SPLOGBOOK_ROOT}/LICENSE"

cp -R "${PAYLOAD}/locales/." "${SPLOGBOOK_ROOT}/locales/"
cp -R "${PAYLOAD}/THIRD_PARTY_LICENSES/." "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES/"
find "${SPLOGBOOK_ROOT}/locales" "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES" \
    -type d -exec chmod 0755 {} +
find "${SPLOGBOOK_ROOT}/locales" "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES" \
    -type f -exec chmod 0644 {} +

install -m 0644 "${REPO_ROOT}/installer/linux/rpm/templates/LGPL-2.1.txt" \
    "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES/HAMLIB_LGPL-2.1.txt"

install -m 0755 "${REPO_ROOT}/installer/linux/rpm/launcher" "${PKG_ROOT}/usr/bin/splogbook"
install -m 0644 "${REPO_ROOT}/installer/linux/rpm/splogbook.desktop" \
    "${PKG_ROOT}/usr/share/applications/splogbook.desktop"
install -m 0644 "${REPO_ROOT}/assets/icon.png" \
    "${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps/splogbook.png"
install -m 0644 "${REPO_ROOT}/installer/linux/rpm/templates/README.RPM" \
    "${PKG_ROOT}/usr/share/doc/splogbook/README.RPM"
install -m 0644 "${REPO_ROOT}/LICENSE" \
    "${PKG_ROOT}/usr/share/licenses/splogbook/LICENSE"
install -m 0644 "${REPO_ROOT}/installer/linux/rpm/templates/LGPL-2.1.txt" \
    "${PKG_ROOT}/usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt"

cat > "${SPLOGBOOK_ROOT}/.splogbook-rpm-marker" <<EOF
{
  "marker_version": "1.0",
  "installation_type": "LinuxRpm",
  "management_mode": "SystemManaged",
  "package_version": "${PRODUCT_VERSION}",
  "rpm_version": "${RPM_VERSION}",
  "rpm_release": "${RPM_RELEASE}",
  "build_commit": "${EXPECTED_BUILD_COMMIT}",
  "resolved_minimum_kernel": "${RESOLVED_MIN_KERNEL}",
  "maximum_glibc": "${GLIBC_POLICY_MAX}",
  "artifact_format": "rpm",
  "architecture": "x86_64",
  "package_name": "splogbook",
  "target": "linux-x86_64",
  "created_by": "scripts/build_linux_rpm.sh"
}
EOF
chmod 0644 "${SPLOGBOOK_ROOT}/.splogbook-rpm-marker"

# ------------------------------------------------------------------
# 6. Forbidden content scan and layout invariants
# ------------------------------------------------------------------
[ -z "$(find "${PKG_ROOT}" -type l -print -quit)" ] || fail "package root contains symbolic links"
[ -z "$(find "${PKG_ROOT}" -type f -perm /6000 -print -quit)" ] \
    || fail "package root contains setuid/setgid files"
[ -z "$(find "${PKG_ROOT}" -type f -perm -0002 -print -quit)" ] \
    || fail "package root contains world-writable files"

FORBIDDEN_FILES="$(find "${PKG_ROOT}/opt" "${PKG_ROOT}/usr" -type f \
    \( -name '*.db' -o -name '*.db-wal' -o -name '*.db-shm' \
       -o -name 'default_log.db' -o -name 'station_config.json' \
       -o -name '*.adi' -o -name '*.adif' -o -name '*.cbr' \
       -o -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '*.pfx' \
       -o -name 'id_rsa*' -o -name '*.token' -o -name '*.secret' \
       -o -name '.git' -o -name '.gitignore' \
       -o -name '.splogbook-linux-portable-marker' \) -print)"
[ -z "${FORBIDDEN_FILES}" ] || fail "forbidden user data or secret in the package: ${FORBIDDEN_FILES}"

LIB_ENTRIES="$(find "${SPLOGBOOK_ROOT}/lib" -mindepth 1 -maxdepth 1 -printf '%f\n')"
[ "${LIB_ENTRIES}" = "libhamlib.so.4" ] \
    || fail "lib/ must contain exactly libhamlib.so.4, found: ${LIB_ENTRIES}"
for forbidden in rigctld rotctld README.txt assets; do
    [ ! -e "${SPLOGBOOK_ROOT}/${forbidden}" ] || fail "forbidden payload entry copied into the package: ${forbidden}"
done

for script in "${PKG_ROOT}/usr/bin/splogbook"; do
    if grep -Eq '(^|[^A-Za-z0-9_])(sudo|pkexec|doas)([^A-Za-z0-9_]|$)' "${script}"; then
        fail "privilege escalation command in ${script}"
    fi
    if grep -Eq 'LD_LIBRARY_PATH|https?://|curl|wget|apt-get|systemctl|crontab' "${script}"; then
        fail "forbidden network/service token in ${script}"
    fi
    if grep -Eq '(^|[^A-Za-z0-9_])(HOME|XDG_[A-Z_]+)([^A-Za-z0-9_]|$)' "${script}"; then
        fail "launcher must not access HOME or XDG"
    fi
done

# ------------------------------------------------------------------
# 7. Deterministic metadata and reproducible build
# ------------------------------------------------------------------
find "${PKG_ROOT}" -print0 | xargs -0 touch -h -d "@${SOURCE_DATE_EPOCH}"
install -m 0644 "${SPEC_SOURCE}" "${WORK_ROOT}/splogbook.spec"

SPEC_QUERY="$(rpmspec -q --qf '%{NAME} %{VERSION} %{RELEASE} %{ARCH}\n' \
    --define "_splog_version ${RPM_VERSION}" \
    --define "_splog_release ${RPM_RELEASE}" \
    --define "_splog_commit ${EXPECTED_BUILD_COMMIT}" \
    "${WORK_ROOT}/splogbook.spec")"
[ "${SPEC_QUERY}" = "splogbook ${RPM_VERSION} ${RPM_RELEASE} x86_64" ] \
    || fail "spec query returned '${SPEC_QUERY}'"

build_once() {
    topdir="$1"
    logfile="$2"
    mkdir -p "${topdir}/BUILD" "${topdir}/BUILDROOT" "${topdir}/RPMS" \
        "${topdir}/SOURCES" "${topdir}/SPECS" "${topdir}/SRPMS"
    mkdir -p "${topdir}/SOURCES/root"
    cp -a "${PKG_ROOT}/." "${topdir}/SOURCES/root/"
    rpmbuild -bb \
        --define "_topdir ${topdir}" \
        --define "_sourcedir ${topdir}/SOURCES" \
        --define "_builddir ${topdir}/BUILD" \
        --define "_buildrootdir ${topdir}/BUILDROOT" \
        --define "_rpmdir ${topdir}/RPMS" \
        --define "_specdir ${topdir}/SPECS" \
        --define "_srcrpmdir ${topdir}/SRPMS" \
        --define "_splog_version ${RPM_VERSION}" \
        --define "_splog_release ${RPM_RELEASE}" \
        --define "_splog_commit ${EXPECTED_BUILD_COMMIT}" \
        "${WORK_ROOT}/splogbook.spec" > "${logfile}" 2>&1 \
        || {
            cat "${logfile}" >&2
            fail "rpmbuild failed"
        }
    find "${topdir}/RPMS" -type f -name '*.rpm' -print -quit
}

RPM_A="$(build_once "${WORK_ROOT}/rpmbuild-a" "${WORK_ROOT}/build-a.log")"
RPM_B="$(build_once "${WORK_ROOT}/rpmbuild-b" "${WORK_ROOT}/build-b.log")"
[ -f "${RPM_A}" ] || fail "first rpmbuild produced no RPM"
[ -f "${RPM_B}" ] || fail "second rpmbuild produced no RPM"

SHA_A="$(sha256sum "${RPM_A}" | awk '{print $1}')"
SHA_B="$(sha256sum "${RPM_B}" | awk '{print $1}')"
[ "${SHA_A}" = "${SHA_B}" ] \
    || fail "the RPM build is not reproducible (${SHA_A} != ${SHA_B})"
log "PASS: reproducible build (two independent builds share SHA-256 ${SHA_A})"

extract_rpm() {
    rpm_path="$1"
    destination="$2"
    mkdir -p "${destination}"
    ( cd "${destination}" && rpm2cpio "${rpm_path}" | cpio -idm --quiet )
}

RPM_A_ROOT="${WORK_ROOT}/extract-a"
RPM_B_ROOT="${WORK_ROOT}/extract-b"
extract_rpm "${RPM_A}" "${RPM_A_ROOT}"
extract_rpm "${RPM_B}" "${RPM_B_ROOT}"

( cd "${RPM_A_ROOT}" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "${WORK_ROOT}/payload-a.txt"
( cd "${RPM_B_ROOT}" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "${WORK_ROOT}/payload-b.txt"
cmp -s "${WORK_ROOT}/payload-a.txt" "${WORK_ROOT}/payload-b.txt" \
    || fail "the RPM payload differs between the two builds"

queue_compare() {
    label="$1"
    shift
    "$@" "${RPM_A}" > "${WORK_ROOT}/${label}-a.txt"
    "$@" "${RPM_B}" > "${WORK_ROOT}/${label}-b.txt"
    cmp -s "${WORK_ROOT}/${label}-a.txt" "${WORK_ROOT}/${label}-b.txt" \
        || {
            diff -u "${WORK_ROOT}/${label}-a.txt" "${WORK_ROOT}/${label}-b.txt" >&2
            fail "RPM ${label} differs between the two builds"
        }
}

queue_compare fields rpm -qp --qf '%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}|%{VENDOR}|%{LICENSE}|%{URL}|%{SUMMARY}|%{GROUP}|%{BUILDHOST}|%{BUILDTIME}|%{SIZE}\n'
queue_compare files rpm -qpl
queue_compare requires rpm -qpR
queue_compare scripts rpm -qp --scripts
queue_compare dump rpm -qp --dump

# ------------------------------------------------------------------
# 8. Output, checksums, manifest and toolchain evidence
# ------------------------------------------------------------------
RPM_NAME="SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.rpm"
RPM_PATH="${OUTPUT_DIR}/${RPM_NAME}"
CANONICAL_NAME="SPLogbook-Linux-x86_64.rpm"
CANONICAL_PATH="${OUTPUT_DIR}/${CANONICAL_NAME}"
install -m 0644 "${RPM_A}" "${RPM_PATH}"
install -m 0644 "${RPM_A}" "${CANONICAL_PATH}"

RPM_SHA="$(sha256sum "${RPM_PATH}" | awk '{print $1}')"
printf '%s  %s\n' "${RPM_SHA}" "${RPM_NAME}" > "${RPM_PATH}.sha256"
printf '%s  %s\n' "${RPM_SHA}" "${CANONICAL_NAME}" > "${CANONICAL_PATH}.sha256"

sh "${MANIFEST_TOOL}" "${RPM_PATH}" "${RPM_PATH}.manifest" \
    "splogbook" "${RPM_VERSION}" "${RPM_RELEASE}" "x86_64" "${EXPECTED_BUILD_COMMIT}"

rpm -qpi "${RPM_PATH}" > "${OUTPUT_DIR}/rpm-info.txt"
rpm -qpl "${RPM_PATH}" > "${OUTPUT_DIR}/rpm-contents.txt"
rpm -qpR "${RPM_PATH}" > "${OUTPUT_DIR}/rpm-requires.txt"
rpm -qp --scripts "${RPM_PATH}" > "${OUTPUT_DIR}/rpm-scripts.txt"
rpm -qp --qf '%{NAME}\n%{VERSION}\n%{RELEASE}\n%{ARCH}\n%{VENDOR}\n%{LICENSE}\n%{URL}\n%{SUMMARY}\n%{GROUP}\n%{SIZE}\n%{BUILDHOST}\n%{BUILDTIME}\n%{PAYLOADFORMAT}\n%{PAYLOADCOMPRESSOR}\n' \
    "${RPM_PATH}" > "${OUTPUT_DIR}/rpm-fields.txt"

DISTRIBUTION="unknown"
if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    DISTRIBUTION="${PRETTY_NAME:-${ID:-unknown}}"
fi

cat > "${OUTPUT_DIR}/rpm-toolchain.json" <<EOF
{
  "package_version": "${PRODUCT_VERSION}",
  "rpm_version": "${RPM_VERSION}",
  "rpm_release": "${RPM_RELEASE}",
  "build_commit": "${EXPECTED_BUILD_COMMIT}",
  "build_image": "${BUILD_IMAGE:-unknown}",
  "build_image_digest": "${BUILD_IMAGE_DIGEST:-unknown}",
  "distribution": "${DISTRIBUTION}",
  "rpm_tool_version": "$(rpm --version)",
  "rpmbuild_version": "$(rpmbuild --version | head -n 1)",
  "rpmspec_version": "$(rpmspec --version | head -n 1)",
  "architecture": "x86_64",
  "compression": "${RPM_COMPRESSION}",
  "source_date_epoch": ${SOURCE_DATE_EPOCH},
  "reproducible_sha256": "${RPM_SHA}"
}
EOF

PAYLOAD_FILE_COUNT="$(find "${PKG_ROOT}/opt" "${PKG_ROOT}/usr" -type f | wc -l | tr -d ' ')"
PAYLOAD_ENTRY_COUNT="$(find "${PKG_ROOT}/opt" "${PKG_ROOT}/usr" -mindepth 1 | wc -l | tr -d ' ')"
log "Package files: ${PAYLOAD_FILE_COUNT}"
log "Package entries: ${PAYLOAD_ENTRY_COUNT}"
log "RPM: ${RPM_PATH}"
log "Size: $(wc -c < "${RPM_PATH}") B"
log "SHA-256: ${RPM_SHA}"

echo "=================================================================="
echo "RPM BUILD COMPLETED SUCCESSFULLY"
echo "Version: ${RPM_VERSION}-${RPM_RELEASE}"
echo "Artifact: ${RPM_PATH}"
echo "SHA-256: ${RPM_SHA}"
echo "=================================================================="
