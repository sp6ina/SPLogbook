#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Deterministic builder of the SPLogbook Linux amd64 binary DEB package (M3-H).
#
# Contract:
#   * input is the verified portable payload
#     SPLogbook-Linux-x86_64-<version>.tar.gz plus its .sha256 sidecar;
#   * the payload build_commit MUST equal EXPECTED_BUILD_COMMIT;
#   * the package root is assembled from an explicit allowlist only;
#   * no user data, no secret, no symlink, no setuid/setgid is ever copied;
#   * dpkg-deb --root-owner-group with explicit xz compression is used;
#   * SOURCE_DATE_EPOCH and fixed file mtimes make the output reproducible;
#   * the package is built twice and both SHA-256 digests MUST match;
#   * the DEB is never installed on the build host and never published.
#
# Usage:
#   EXPECTED_BUILD_COMMIT=<40-hex> SOURCE_DATE_EPOCH=<epoch> \
#     bash scripts/build_linux_deb.sh [output-dir] [portable.tar.gz]
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
DEB_REVISION="${DEB_REVISION:-1}"
DEB_COMPRESSION="${DEB_COMPRESSION:-xz}"

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

assert_runpath() {
    file="$1"
    required="$2"
    runpath="$(readelf -d "${file}" | sed -n 's/.*RUNPATH.*\[\(.*\)\].*/\1/p')"
    [ -n "${runpath}" ] || fail "missing RUNPATH in ${file}"
    case "${runpath}" in
        *"${required}"*) ;;
        *) fail "RUNPATH '${runpath}' in ${file} does not contain '${required}'" ;;
    esac
}

echo "=================================================================="
echo "SPLogbook Linux amd64 DEB Builder (M3-H)"
echo "=================================================================="

for tool in dpkg-deb dpkg dpkg-architecture sha256sum tar gzip find awk sed \
    readelf file install touch sort grep date; do
    require_tool "${tool}"
done

log "dpkg: $(dpkg --version | head -n 1)"
log "dpkg-deb: $(dpkg-deb --version | head -n 1)"
log "dpkg-architecture: $(dpkg-architecture -qDEB_HOST_ARCH)"

[ "$(dpkg-architecture -qDEB_HOST_ARCH)" = "amd64" ] \
    || fail "the builder must run on an amd64 host"

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
DEBIAN_VERSION="${BASE_VERSION}~${PRERELEASE}-${DEB_REVISION}"
log "Application version: ${PRODUCT_VERSION}"
log "Debian version: ${DEBIAN_VERSION}"

dpkg --validate-version "${DEBIAN_VERSION}" \
    || fail "dpkg rejects the generated Debian version: ${DEBIAN_VERSION}"
[ "${DEBIAN_VERSION#*:}" = "${DEBIAN_VERSION}" ] || fail "an epoch is not allowed"
dpkg --compare-versions "${DEBIAN_VERSION}" lt "${BASE_VERSION}-${DEB_REVISION}" \
    || fail "prerelease must sort before the stable version"
dpkg --compare-versions "${DEBIAN_VERSION}" gt "${BASE_VERSION}~alpha.0-${DEB_REVISION}" \
    || fail "prerelease must sort after alpha.0"
dpkg --compare-versions "${DEBIAN_VERSION}" lt "${BASE_VERSION}~alpha.2-${DEB_REVISION}" \
    || fail "prerelease must sort before alpha.2"
dpkg --compare-versions "${DEBIAN_VERSION}" eq "${DEBIAN_VERSION}" \
    || fail "version self-comparison failed"
log "PASS: Debian version ordering verified with dpkg --compare-versions"

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

WORK_ROOT="$(mktemp -d "${OUTPUT_DIR}/.deb_build_XXXXXX")"
trap 'rm -rf "${WORK_ROOT}"' EXIT

tar -xzf "${PAYLOAD_ARCHIVE}" -C "${WORK_ROOT}"
PAYLOAD="${WORK_ROOT}/SPLogbook"
[ -d "${PAYLOAD}" ] || fail "payload does not contain a single SPLogbook/ root"
[ -z "$(find "${WORK_ROOT}" -mindepth 1 -maxdepth 1 ! -name SPLogbook -print -quit)" ] \
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

# Only libhamlib.so.4 may be bundled.
[ "$(find "${PAYLOAD}/lib" -maxdepth 1 -type f | wc -l)" -eq 1 ] \
    || fail "payload lib/ must contain exactly one file"
[ -f "${PAYLOAD}/lib/libhamlib.so.4" ] || fail "payload is missing lib/libhamlib.so.4"

# ------------------------------------------------------------------
# 4. ELF / RUNPATH / GLIBC preflight of the payload
# ------------------------------------------------------------------
for elf in "${PAYLOAD}/SPLogbook" "${PAYLOAD}/splogbook-updater" "${PAYLOAD}/lib/libhamlib.so.4"; do
    assert_elf "${elf}"
done
assert_runpath "${PAYLOAD}/SPLogbook" '$ORIGIN/lib'
assert_runpath "${PAYLOAD}/splogbook-updater" '$ORIGIN/lib'
assert_runpath "${PAYLOAD}/lib/libhamlib.so.4" '$ORIGIN'
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
    "${PKG_ROOT}/DEBIAN"

install -m 0755 "${PAYLOAD}/SPLogbook" "${SPLOGBOOK_ROOT}/SPLogbook"
install -m 0755 "${PAYLOAD}/splogbook-updater" "${SPLOGBOOK_ROOT}/splogbook-updater"
install -m 0755 "${PAYLOAD}/lib/libhamlib.so.4" "${SPLOGBOOK_ROOT}/lib/libhamlib.so.4"
install -m 0644 "${PAYLOAD}/LICENSE" "${SPLOGBOOK_ROOT}/LICENSE"

cp -R "${PAYLOAD}/locales/." "${SPLOGBOOK_ROOT}/locales/"
cp -R "${PAYLOAD}/THIRD_PARTY_LICENSES/." "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES/"
find "${SPLOGBOOK_ROOT}/locales" "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES" \
    -type d -exec chmod 0755 {} +
find "${SPLOGBOOK_ROOT}/locales" "${SPLOGBOOK_ROOT}/THIRD_PARTY_LICENSES" \
    -type f -exec chmod 0644 {} +

install -m 0755 "${REPO_ROOT}/installer/linux/deb/launcher" "${PKG_ROOT}/usr/bin/splogbook"
install -m 0644 "${REPO_ROOT}/installer/linux/deb/splogbook.desktop" \
    "${PKG_ROOT}/usr/share/applications/splogbook.desktop"
install -m 0644 "${REPO_ROOT}/assets/icon.png" \
    "${PKG_ROOT}/usr/share/icons/hicolor/256x256/apps/splogbook.png"
install -m 0644 "${REPO_ROOT}/installer/linux/deb/templates/README.Debian" \
    "${PKG_ROOT}/usr/share/doc/splogbook/README.Debian"
install -m 0644 "${REPO_ROOT}/installer/linux/deb/templates/copyright" \
    "${PKG_ROOT}/usr/share/doc/splogbook/copyright"

# A binary Debian package must ship a changelog (Policy 12.7). This is a
# package documentation file, not a Debian source package changelog.
CHANGELOG_DATE="$(date -R -u -d "@${SOURCE_DATE_EPOCH}")"
cat > "${WORK_ROOT}/changelog.Debian" <<EOF
splogbook (${DEBIAN_VERSION}) unstable; urgency=low

  * Binary package build for the SPLogbook ${PRODUCT_VERSION} development release.

 -- Mariusz Woźniak (SP6INA) <contact@splogbook.org>  ${CHANGELOG_DATE}
EOF
gzip -9 -n -c "${WORK_ROOT}/changelog.Debian" \
    > "${PKG_ROOT}/usr/share/doc/splogbook/changelog.Debian.gz"
chmod 0644 "${PKG_ROOT}/usr/share/doc/splogbook/changelog.Debian.gz"

cat > "${SPLOGBOOK_ROOT}/.splogbook-deb-marker" <<EOF
{
  "marker_version": "1.0",
  "installation_type": "LinuxDeb",
  "management_mode": "SystemManaged",
  "package_version": "${PRODUCT_VERSION}",
  "debian_version": "${DEBIAN_VERSION}",
  "build_commit": "${EXPECTED_BUILD_COMMIT}",
  "resolved_minimum_kernel": "${RESOLVED_MIN_KERNEL}",
  "maximum_glibc": "${GLIBC_POLICY_MAX}",
  "artifact_format": "deb",
  "architecture": "amd64",
  "package_name": "splogbook",
  "created_by": "scripts/build_linux_deb.sh"
}
EOF
chmod 0644 "${SPLOGBOOK_ROOT}/.splogbook-deb-marker"

# ------------------------------------------------------------------
# 6. Control, maintainer scripts and installed size
# ------------------------------------------------------------------
INSTALLED_BYTES="$(find "${PKG_ROOT}" -type f ! -path "${PKG_ROOT}/DEBIAN/*" -printf '%s\n' \
    | awk '{ total += $1 } END { print total + 0 }')"
INSTALLED_SIZE="$(( (INSTALLED_BYTES + 1023) / 1024 ))"
[ "${INSTALLED_SIZE}" -gt 0 ] || fail "computed Installed-Size is zero"
log "Installed-Size: ${INSTALLED_SIZE} KiB (${INSTALLED_BYTES} bytes of payload)"

sed \
    -e "s|@DEBIAN_VERSION@|${DEBIAN_VERSION}|g" \
    -e "s|@INSTALLED_SIZE@|${INSTALLED_SIZE}|g" \
    "${REPO_ROOT}/installer/linux/deb/templates/control" > "${PKG_ROOT}/DEBIAN/control"
chmod 0644 "${PKG_ROOT}/DEBIAN/control"

install -m 0755 "${REPO_ROOT}/installer/linux/deb/templates/postinst" "${PKG_ROOT}/DEBIAN/postinst"
install -m 0755 "${REPO_ROOT}/installer/linux/deb/templates/postrm" "${PKG_ROOT}/DEBIAN/postrm"

for script in "${PKG_ROOT}/DEBIAN/postinst" "${PKG_ROOT}/DEBIAN/postrm"; do
    sh -n "${script}" || fail "maintainer script has a syntax error: ${script}"
done

# The approved binary package has no conffiles, no preinst and no prerm.
[ ! -e "${PKG_ROOT}/DEBIAN/conffiles" ] || fail "conffiles are not approved"
[ ! -e "${PKG_ROOT}/DEBIAN/preinst" ] || fail "preinst is not approved"
[ ! -e "${PKG_ROOT}/DEBIAN/prerm" ] || fail "prerm is not approved"

# ------------------------------------------------------------------
# 7. Forbidden content scan
# ------------------------------------------------------------------
FORBIDDEN_FILES="$(find "${PKG_ROOT}/opt" "${PKG_ROOT}/usr" -type f \
    \( -name '*.db' -o -name '*.db-wal' -o -name '*.db-shm' \
       -o -name 'default_log.db' -o -name 'station_config.json' \
       -o -name '*.adi' -o -name '*.adif' -o -name '*.cbr' \
       -o -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '*.pfx' \
       -o -name 'id_rsa*' -o -name '*.token' -o -name '*.secret' \
       -o -name '.git' -o -name '.gitignore' \) -print)"
[ -z "${FORBIDDEN_FILES}" ] || fail "forbidden user data or secret in the package: ${FORBIDDEN_FILES}"

for script in "${PKG_ROOT}/DEBIAN/postinst" "${PKG_ROOT}/DEBIAN/postrm" \
    "${PKG_ROOT}/usr/bin/splogbook"; do
    if grep -Eq '(^|[^A-Za-z0-9_])(sudo|pkexec|doas)([^A-Za-z0-9_]|$)' "${script}"; then
        fail "privilege escalation command in ${script}"
    fi
    if grep -Eq 'LD_LIBRARY_PATH|https?://|curl|wget|apt-get|systemctl|crontab' "${script}"; then
        fail "forbidden network/service token in ${script}"
    fi
    if grep -Eq '(^|[^A-Za-z0-9_])(HOME|XDG_[A-Z_]+)([^A-Za-z0-9_]|$)' "${script}"; then
        fail "maintainer scripts and launcher must not access HOME or XDG"
    fi
done

# ------------------------------------------------------------------
# 8. Deterministic metadata and reproducible build
# ------------------------------------------------------------------
find "${PKG_ROOT}" -print0 | xargs -0 touch -h -d "@${SOURCE_DATE_EPOCH}"

build_once() {
    target="$1"
    dpkg-deb --root-owner-group -Z"${DEB_COMPRESSION}" --build "${PKG_ROOT}" "${target}" >/dev/null
}

FIRST_DEB="${WORK_ROOT}/deb-a.deb"
SECOND_DEB="${WORK_ROOT}/deb-b.deb"
build_once "${FIRST_DEB}"
build_once "${SECOND_DEB}"
FIRST_SHA="$(sha256sum "${FIRST_DEB}" | awk '{print $1}')"
SECOND_SHA="$(sha256sum "${SECOND_DEB}" | awk '{print $1}')"
[ "${FIRST_SHA}" = "${SECOND_SHA}" ] \
    || fail "the DEB build is not reproducible (${FIRST_SHA} != ${SECOND_SHA})"
log "PASS: reproducible build (two independent builds share SHA-256 ${FIRST_SHA})"

# ------------------------------------------------------------------
# 9. Output, checksums and toolchain evidence
# ------------------------------------------------------------------
DEB_NAME="SPLogbook-Linux-amd64-${PRODUCT_VERSION}.deb"
DEB_PATH="${OUTPUT_DIR}/${DEB_NAME}"
CANONICAL_NAME="SPLogbook-Linux-amd64.deb"
CANONICAL_PATH="${OUTPUT_DIR}/${CANONICAL_NAME}"
install -m 0644 "${FIRST_DEB}" "${DEB_PATH}"
install -m 0644 "${FIRST_DEB}" "${CANONICAL_PATH}"

DEB_SHA="$(sha256sum "${DEB_PATH}" | awk '{print $1}')"
printf '%s  %s\n' "${DEB_SHA}" "${DEB_NAME}" > "${DEB_PATH}.sha256"
printf '%s  %s\n' "${DEB_SHA}" "${CANONICAL_NAME}" > "${CANONICAL_PATH}.sha256"

DISTRIBUTION="unknown"
if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    DISTRIBUTION="${PRETTY_NAME:-${ID:-unknown}}"
fi
DPKG_VERSION="$(dpkg --version | head -n 1)"
DPKG_DEB_VERSION="$(dpkg-deb --version | head -n 1)"
DPKG_ARCH="$(dpkg-architecture -qDEB_HOST_ARCH)"

cat > "${OUTPUT_DIR}/deb-toolchain.json" <<EOF
{
  "package_version": "${PRODUCT_VERSION}",
  "debian_version": "${DEBIAN_VERSION}",
  "build_commit": "${EXPECTED_BUILD_COMMIT}",
  "build_image": "${BUILD_IMAGE:-unknown}",
  "build_image_digest": "${BUILD_IMAGE_DIGEST:-unknown}",
  "distribution": "${DISTRIBUTION}",
  "dpkg_version": "${DPKG_VERSION}",
  "dpkg_deb_version": "${DPKG_DEB_VERSION}",
  "dpkg_architecture": "${DPKG_ARCH}",
  "package_source": "official distribution repository (apt)",
  "compression": "${DEB_COMPRESSION}",
  "source_date_epoch": ${SOURCE_DATE_EPOCH},
  "reproducible_sha256": "${DEB_SHA}"
}
EOF

# ------------------------------------------------------------------
# 10. Package inspection (never installation)
# ------------------------------------------------------------------
dpkg-deb --info "${DEB_PATH}" > "${OUTPUT_DIR}/deb-info.txt"
dpkg-deb --contents "${DEB_PATH}" > "${OUTPUT_DIR}/deb-contents.txt"
dpkg-deb --field "${DEB_PATH}" Package Version Architecture Depends Installed-Size \
    > "${OUTPUT_DIR}/deb-fields.txt"

PAYLOAD_FILE_COUNT="$(find "${PKG_ROOT}/opt" "${PKG_ROOT}/usr" -type f | wc -l)"
log "Package files: ${PAYLOAD_FILE_COUNT}"
log "DEB: ${DEB_PATH}"
log "Size: $(wc -c < "${DEB_PATH}") B"
log "SHA-256: ${DEB_SHA}"

echo "=================================================================="
echo "DEB BUILD COMPLETED SUCCESSFULLY"
echo "Version: ${DEBIAN_VERSION}"
echo "Artifact: ${DEB_PATH}"
echo "SHA-256: ${DEB_SHA}"
echo "=================================================================="
