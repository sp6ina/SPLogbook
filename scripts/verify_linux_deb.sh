#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Fail-closed verifier of the SPLogbook Linux amd64 binary DEB package (M3-H).
#
# Contract:
#   * every critical probe aborts on a non-zero exit or an unexpected value;
#   * no critical command is masked with a blanket success fallback;
#   * "DEB VERIFICATION COMPLETED SUCCESSFULLY" is printed ONLY after the
#     complete static, ABI, installation, upgrade, downgrade, reinstall,
#     removal, purge and user-data-preservation checks passed;
#   * SPLOG_DEB_INSPECTION_ONLY=1 performs the static part only and prints
#     "DEB INSPECTION COMPLETED SUCCESSFULLY" instead (used by the negative
#     fixture harness, never as proof of installation).
#
# Required environment for the full run:
#   EXPECTED_BUILD_COMMIT  (40-hex commit under test)
#   EXPECTED_ICON_SHA256   (optional, source icon digest)
#   EXPECTED_DESKTOP_SHA256(optional, source desktop digest)

set -eu

GLIBC_POLICY_MAX="2.43"
EXPECTED_VERSION="${EXPECTED_APP_VERSION:-1.3.0-alpha.1}"
EXPECTED_DEBIAN_VERSION="${EXPECTED_DEBIAN_VERSION:-1.3.0~alpha.1-1}"
EXPECTED_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
EXPECTED_ICON_SHA256="${EXPECTED_ICON_SHA256:-}"
EXPECTED_DESKTOP_SHA256="${EXPECTED_DESKTOP_SHA256:-}"
INSPECTION_ONLY="${SPLOG_DEB_INSPECTION_ONLY:-0}"
SKIP_LINTIAN="${SPLOG_DEB_SKIP_LINTIAN:-0}"

DEB="${1:-}"
if [ -z "${DEB}" ]; then
    printf 'FAIL: usage: %s <path-to-splogbook.deb>\n' "$0" >&2
    exit 1
fi
[ -f "${DEB}" ] || {
    printf 'FAIL: DEB does not exist: %s\n' "${DEB}" >&2
    exit 1
}
DEB_DIR="$(cd "$(dirname "${DEB}")" && pwd)"
DEB="${DEB_DIR}/$(basename "${DEB}")"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-deb-verify.XXXXXX")"
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

field() {
    dpkg-deb -f "${DEB}" "$1"
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

assert_no_forbidden_token() {
    file="$1"
    if grep -Eq '(^|[^A-Za-z0-9_])(sudo|pkexec|doas)([^A-Za-z0-9_]|$)' "${file}"; then
        fail "privilege escalation command in ${file}"
    fi
    if grep -Eq 'LD_LIBRARY_PATH|https?://|curl|wget|apt-get|systemctl|crontab' "${file}"; then
        fail "forbidden network/service token in ${file}"
    fi
    if grep -Eq '(^|[^A-Za-z0-9_])(HOME|XDG_[A-Z_]+)([^A-Za-z0-9_]|$)' "${file}"; then
        fail "HOME/XDG access is forbidden in ${file}"
    fi
}

echo "=================================================================="
echo "SPLogbook Linux amd64 DEB Verifier (M3-H)"
echo "Package: ${DEB}"
echo "=================================================================="

for tool in dpkg-deb dpkg dpkg-query file readelf ldd sha256sum awk sed \
    grep sort python3 desktop-file-validate gzip; do
    require_tool "${tool}"
done
log "dpkg: $(dpkg --version | head -n 1)"
log "dpkg-deb: $(dpkg-deb --version | head -n 1)"

# ------------------------------------------------------------------
# 1. Checksum sidecar and package type
# ------------------------------------------------------------------
[ -f "${DEB}.sha256" ] || fail "checksum sidecar is missing: ${DEB}.sha256"
EXPECTED_SHA="$(awk '{print $1}' "${DEB}.sha256")"
ACTUAL_SHA="$(sha256sum "${DEB}" | awk '{print $1}')"
[ "${EXPECTED_SHA}" = "${ACTUAL_SHA}" ] \
    || fail "SHA-256 sidecar does not match the DEB (${EXPECTED_SHA} != ${ACTUAL_SHA})"
pass "SHA-256 integrity verified (${ACTUAL_SHA})"

FILE_TYPE="$(file -b "${DEB}")"
printf '%s\n' "${FILE_TYPE}"
case "${FILE_TYPE}" in
    *"Debian binary package"*) pass "file(1) identifies a Debian binary package" ;;
    *) fail "not a Debian binary package: ${FILE_TYPE}" ;;
esac

dpkg-deb --info "${DEB}" > "${WORK}/deb-info.txt" || fail "dpkg-deb --info failed"
dpkg-deb --contents "${DEB}" > "${WORK}/deb-contents.txt" || fail "dpkg-deb --contents failed"
head -20 "${WORK}/deb-info.txt"

# ------------------------------------------------------------------
# 2. Control fields
# ------------------------------------------------------------------
[ "$(field Package)" = "splogbook" ] || fail "Package must be splogbook"
[ "$(field Version)" = "${EXPECTED_DEBIAN_VERSION}" ] \
    || fail "Version mismatch: $(field Version) != ${EXPECTED_DEBIAN_VERSION}"
[ "$(field Architecture)" = "amd64" ] || fail "Architecture must be amd64"
[ "$(field Section)" = "hamradio" ] || fail "Section must be hamradio"
[ "$(field Priority)" = "optional" ] || fail "Priority must be optional"
[ "$(field Maintainer)" = "Mariusz Woźniak (SP6INA) <contact@splogbook.org>" ] \
    || fail "Maintainer identity mismatch"
[ "$(field Homepage)" = "https://github.com/sp6ina/SPLogbook" ] \
    || fail "Homepage mismatch"
[ -n "$(field Description)" ] || fail "Description is empty"
case "$(field Description)" in
    *"official Debian"*|*"part of Debian"*) fail "Description must not claim an official Debian package" ;;
esac
pass "control identity fields are correct"

dpkg --validate-version "${EXPECTED_DEBIAN_VERSION}" \
    || fail "dpkg rejects the expected Debian version"
dpkg --compare-versions "${EXPECTED_DEBIAN_VERSION}" lt "1.3.0-1" \
    || fail "prerelease must sort before 1.3.0-1"
dpkg --compare-versions "${EXPECTED_DEBIAN_VERSION}" gt "1.3.0~alpha.0-1" \
    || fail "prerelease must sort after 1.3.0~alpha.0-1"
pass "Debian version ordering verified with dpkg --compare-versions"

# ------------------------------------------------------------------
# 3. Depends contract
# ------------------------------------------------------------------
DEPENDS_RAW="$(field Depends)"
log "Depends: ${DEPENDS_RAW}"
case "${DEPENDS_RAW}" in
    *hamlib*) fail "libhamlib is bundled and must not appear in Depends" ;;
esac
printf '%s' "${DEPENDS_RAW}" | grep -q 'libc6 (>= 2.43)' \
    || fail "Depends must require libc6 (>= 2.43)"

DEPENDS_NORMALIZED="$(printf '%s\n' "${DEPENDS_RAW}" \
    | tr ',' '\n' \
    | sed 's/([^)]*)//g; s/[[:space:]]//g' \
    | sort)"
EXPECTED_DEPENDS="$(cat <<'EOF'
libasound2t64
libc6
libfontconfig1
libgcc-s1
libgl1
libgtk-3-0t64
libudev1
libusb-1.0-0
libwayland-client0
libwayland-cursor0
libwayland-egl1
libx11-6
libxcursor1
libxi6
libxkbcommon-x11-0
libxkbcommon0
libxrandr2
EOF
)"
[ "${DEPENDS_NORMALIZED}" = "${EXPECTED_DEPENDS}" ] \
    || fail "Depends set does not match the verified runtime contract"
pass "Depends match the verified runtime contract"

# ------------------------------------------------------------------
# 4. Extract the package and enforce the approved layout
# ------------------------------------------------------------------
ROOT="${WORK}/root"
CONTROL="${WORK}/control"
mkdir -p "${ROOT}" "${CONTROL}"
dpkg-deb -x "${DEB}" "${ROOT}" || fail "dpkg-deb -x failed"
dpkg-deb -e "${DEB}" "${CONTROL}" || fail "dpkg-deb -e failed"

for required in \
    opt/SPLogbook/SPLogbook \
    opt/SPLogbook/splogbook-updater \
    opt/SPLogbook/lib/libhamlib.so.4 \
    opt/SPLogbook/LICENSE \
    opt/SPLogbook/.splogbook-deb-marker \
    usr/bin/splogbook \
    usr/share/applications/splogbook.desktop \
    usr/share/icons/hicolor/256x256/apps/splogbook.png \
    usr/share/doc/splogbook/README.Debian \
    usr/share/doc/splogbook/copyright \
    usr/share/doc/splogbook/changelog.Debian.gz; do
    [ -f "${ROOT}/${required}" ] || fail "required package file is missing: ${required}"
done

[ -d "${ROOT}/opt/SPLogbook/locales" ] || fail "locales/ directory is missing"
[ -d "${ROOT}/opt/SPLogbook/THIRD_PARTY_LICENSES" ] || fail "THIRD_PARTY_LICENSES/ is missing"

UNEXPECTED_OPT="$(find "${ROOT}/opt/SPLogbook" -mindepth 1 -maxdepth 1 -printf '%f\n' \
    | sort \
    | awk '!/^(SPLogbook|splogbook-updater|lib|locales|LICENSE|THIRD_PARTY_LICENSES|\.splogbook-deb-marker)$/')"
[ -z "${UNEXPECTED_OPT}" ] || fail "unexpected entries in /opt/SPLogbook: ${UNEXPECTED_OPT}"

LIB_ENTRIES="$(find "${ROOT}/opt/SPLogbook/lib" -mindepth 1 -maxdepth 1 -printf '%f\n')"
[ "${LIB_ENTRIES}" = "libhamlib.so.4" ] \
    || fail "lib/ must contain exactly libhamlib.so.4, found: ${LIB_ENTRIES}"

UNEXPECTED_USR="$(cd "${ROOT}" && find usr \( -type f -o -type l \) -print | sort \
    | awk '!/^usr\/bin\/splogbook$|^usr\/share\/applications\/splogbook\.desktop$|^usr\/share\/icons\/hicolor\/256x256\/apps\/splogbook\.png$|^usr\/share\/doc\/splogbook\/(README\.Debian|copyright|changelog\.Debian\.gz)$/')"
[ -z "${UNEXPECTED_USR}" ] || fail "unexpected files under usr/: ${UNEXPECTED_USR}"

[ -z "$(find "${ROOT}" -type l -print -quit)" ] || fail "package contains symbolic links"
[ -z "$(find "${ROOT}" \( ! -type f ! -type d \) -print -quit)" ] \
    || fail "package contains special files"
pass "approved /opt/SPLogbook layout verified"

# ------------------------------------------------------------------
# 5. Ownership and permissions from the ar archive metadata
# ------------------------------------------------------------------
awk '{print $1, $2, $6}' "${WORK}/deb-contents.txt" > "${WORK}/modes.txt"
if awk '$2 != "root/root" { exit 1 }' "${WORK}/modes.txt"; then
    pass "all entries are owned by root/root"
else
    fail "package contains entries not owned by root/root"
fi
if awk '$1 ~ /^l/ { exit 1 }' "${WORK}/modes.txt"; then
    pass "no symbolic links in the package archive"
else
    fail "package archive contains a symbolic link"
fi
if awk '$1 ~ /[sSt]/ { exit 1 }' "${WORK}/modes.txt"; then
    pass "no setuid, setgid or sticky bits"
else
    fail "package archive contains a setuid/setgid/sticky bit"
fi
if awk 'substr($1, 9, 1) == "w" { exit 1 }' "${WORK}/modes.txt"; then
    pass "no world-writable entries"
else
    fail "package archive contains a world-writable entry"
fi

assert_mode() {
    path="$1"
    expected="$2"
    actual="$(awk -v p="${path}" '$3 == p { print $1 }' "${WORK}/modes.txt")"
    [ -n "${actual}" ] || fail "missing archive entry: ${path}"
    [ "${actual}" = "${expected}" ] || fail "mode ${actual} for ${path}, expected ${expected}"
}

assert_mode "./opt/SPLogbook/SPLogbook" "-rwxr-xr-x"
assert_mode "./opt/SPLogbook/splogbook-updater" "-rwxr-xr-x"
assert_mode "./opt/SPLogbook/lib/libhamlib.so.4" "-rw-r--r--"
assert_mode "./opt/SPLogbook/LICENSE" "-rw-r--r--"
assert_mode "./opt/SPLogbook/.splogbook-deb-marker" "-rw-r--r--"
assert_mode "./usr/bin/splogbook" "-rwxr-xr-x"
assert_mode "./usr/share/applications/splogbook.desktop" "-rw-r--r--"
assert_mode "./usr/share/icons/hicolor/256x256/apps/splogbook.png" "-rw-r--r--"
assert_mode "./usr/share/doc/splogbook/README.Debian" "-rw-r--r--"
assert_mode "./usr/share/doc/splogbook/copyright" "-rw-r--r--"
assert_mode "./usr/share/doc/splogbook/changelog.Debian.gz" "-rw-r--r--"
pass "file modes are correct"

# The changelog must be a valid, deterministically compressed gzip stream.
gzip -t "${ROOT}/usr/share/doc/splogbook/changelog.Debian.gz" \
    || fail "changelog.Debian.gz is not a valid gzip stream"
pass "binary package changelog is present and valid"

# ------------------------------------------------------------------
# 6. Forbidden user data, secrets and bundled system libraries
# ------------------------------------------------------------------
FORBIDDEN_FILES="$(find "${ROOT}/opt" "${ROOT}/usr" -type f \
    \( -name '*.db' -o -name '*.db-wal' -o -name '*.db-shm' \
       -o -name 'default_log.db' -o -name 'station_config.json' \
       -o -name '*.adi' -o -name '*.adif' -o -name '*.cbr' \
       -o -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '*.pfx' \
       -o -name 'id_rsa*' -o -name '*.token' -o -name '*.secret' \
       -o -name '.git' -o -name '.gitignore' \) -print)"
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
# 7. Maintainer scripts
# ------------------------------------------------------------------
for forbidden_control in conffiles preinst prerm; do
    [ ! -e "${CONTROL}/${forbidden_control}" ] \
        || fail "unapproved control file present: ${forbidden_control}"
done
[ -f "${CONTROL}/postinst" ] || fail "postinst is missing"
[ -f "${CONTROL}/postrm" ] || fail "postrm is missing"
for script in "${CONTROL}/postinst" "${CONTROL}/postrm"; do
    [ -x "${script}" ] || fail "maintainer script is not executable: ${script}"
    grep -qx '#!/bin/sh' "${script}" || fail "maintainer script must use /bin/sh: ${script}"
    grep -q 'set -e' "${script}" || fail "maintainer script must use set -e: ${script}"
    sh -n "${script}" || fail "maintainer script syntax error: ${script}"
    assert_no_forbidden_token "${script}"
done
pass "maintainer scripts are minimal, non-interactive and safe"

# ------------------------------------------------------------------
# 8. ELF, RUNPATH, GLIBC and ldd
# ------------------------------------------------------------------
for elf in \
    "${ROOT}/opt/SPLogbook/SPLogbook" \
    "${ROOT}/opt/SPLogbook/splogbook-updater" \
    "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4"; do
    assert_elf "${elf}"
done
assert_runpath "${ROOT}/opt/SPLogbook/SPLogbook" '$ORIGIN/lib'
assert_runpath "${ROOT}/opt/SPLogbook/splogbook-updater" '$ORIGIN/lib'
assert_runpath "${ROOT}/opt/SPLogbook/lib/libhamlib.so.4" '$ORIGIN'

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

# ldd is executed after installation, when the declared Depends are satisfied;
# a missing dependency must therefore fail the lifecycle, not the static pass.

# ------------------------------------------------------------------
# 9. Launcher, desktop entry and icon
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

# ------------------------------------------------------------------
# 10. Marker identity
# ------------------------------------------------------------------
MARKER="${ROOT}/opt/SPLogbook/.splogbook-deb-marker"
python3 - "${MARKER}" "${EXPECTED_VERSION}" "${EXPECTED_DEBIAN_VERSION}" "${EXPECTED_COMMIT}" <<'PY'
import json
import re
import sys

path, expected_version, expected_debian, expected_commit = sys.argv[1:5]
with open(path, encoding="utf-8") as handle:
    data = json.load(handle)

expected = {
    "marker_version": "1.0",
    "installation_type": "LinuxDeb",
    "management_mode": "SystemManaged",
    "package_version": expected_version,
    "debian_version": expected_debian,
    "resolved_minimum_kernel": "7.0",
    "maximum_glibc": "2.43",
    "artifact_format": "deb",
    "architecture": "amd64",
    "package_name": "splogbook",
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
print(f"Marker debian_version: {data['debian_version']}")
PY
pass "DEB marker identity verified"

# ------------------------------------------------------------------
# 11. Installed-Size
# ------------------------------------------------------------------
INSTALLED_BYTES="$(find "${ROOT}" -type f -printf '%s\n' | awk '{ total += $1 } END { print total + 0 }')"
COMPUTED_SIZE="$(( (INSTALLED_BYTES + 1023) / 1024 ))"
DECLARED_SIZE="$(field Installed-Size)"
[ "${DECLARED_SIZE}" = "${COMPUTED_SIZE}" ] \
    || fail "Installed-Size ${DECLARED_SIZE} != computed ${COMPUTED_SIZE}"
pass "Installed-Size is consistent with the payload"

# ------------------------------------------------------------------
# 12. lintian
# ------------------------------------------------------------------
if [ "${SKIP_LINTIAN}" = "1" ]; then
    log "lintian: skipped by SPLOG_DEB_SKIP_LINTIAN=1 (inspection-only fixtures)"
else
    require_tool lintian
    # OWNER-M3-H-03 explicitly approves the /opt/SPLogbook layout and the
    # private bundled libhamlib.so.4. FHS reserves /opt for add-on software and
    # the private library requires RUNPATH=$ORIGIN/lib, so lintian reports
    # dir-or-file-in-opt and custom-library-search-path as errors. These two
    # documented suppressions record the approved design; no other tag is hidden.
    LINTIAN_SUPPRESSED_TAGS="dir-or-file-in-opt,custom-library-search-path"
    log "lintian suppressed tags (OWNER-M3-H-03 approved /opt layout): ${LINTIAN_SUPPRESSED_TAGS}"
    set +e
    lintian --no-tag-display-limit --suppress-tags "${LINTIAN_SUPPRESSED_TAGS}" "${DEB}" \
        > "${WORK}/lintian.txt" 2>&1
    LINTIAN_CODE=$?
    set -e
    cat "${WORK}/lintian.txt"
    LINTIAN_ERRORS="$(awk '/^E:/ { n++ } END { print n + 0 }' "${WORK}/lintian.txt")"
    LINTIAN_WARNINGS="$(awk '/^W:/ { n++ } END { print n + 0 }' "${WORK}/lintian.txt")"
    if [ "${LINTIAN_ERRORS}" -gt 0 ]; then
        fail "lintian reported ${LINTIAN_ERRORS} error(s)"
    fi
    if [ "${LINTIAN_CODE}" -ne 0 ]; then
        fail "lintian exited with unexpected code ${LINTIAN_CODE}"
    fi
    pass "lintian reported 0 errors (${LINTIAN_WARNINGS} warnings; suppressed: ${LINTIAN_SUPPRESSED_TAGS})"
fi

echo "------------------------------------------------------------------"
echo "STATIC VERIFICATION COMPLETED"
echo "------------------------------------------------------------------"

if [ "${INSPECTION_ONLY}" = "1" ]; then
    printf '%s\n' "DEB INSPECTION COMPLETED SUCCESSFULLY"
    exit 0
fi

# ==================================================================
# 13. Native installation, upgrade, downgrade, remove and purge
# ==================================================================
[ "$(id -u)" -eq 0 ] || fail "the installation lifecycle requires root"
export DEBIAN_FRONTEND=noninteractive

# Minimal container images frequently ship a dpkg path-exclude for
# /usr/share/doc. The native lifecycle must prove that the package installs
# its documentation, so disable that image-only policy inside the disposable
# test system before installing anything. A normal host is unaffected.
if [ -f /etc/dpkg/dpkg.cfg.d/excludes ]; then
    mv /etc/dpkg/dpkg.cfg.d/excludes "${WORK}/dpkg-doc-excludes.disabled"
    log "disabled the container image dpkg path-exclude for /usr/share/doc"
fi

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

# A controlled lower-version fixture is derived from the same payload.
PREV_ROOT="${WORK}/previous-root"
PREV_DEB="${WORK}/previous.deb"
dpkg-deb -R "${DEB}" "${PREV_ROOT}" || fail "cannot extract the previous-version fixture"
sed -i 's/^Version: .*/Version: 1.3.0~alpha.0-1/' "${PREV_ROOT}/DEBIAN/control"
sed -i 's/"debian_version": "[^"]*"/"debian_version": "1.3.0~alpha.0-1"/' \
    "${PREV_ROOT}/opt/SPLogbook/.splogbook-deb-marker"
dpkg-deb --root-owner-group -Zxz --build "${PREV_ROOT}" "${PREV_DEB}" >/dev/null \
    || fail "cannot build the previous-version fixture"

install_previous_fixture() {
    if ! dpkg -i "${PREV_DEB}" > "${WORK}/dpkg-prev.log" 2>&1; then
        apt-get -y -f install > "${WORK}/apt-prev.log" 2>&1 \
            || fail "apt-get -f install failed for the previous fixture"
        dpkg -i "${PREV_DEB}" > "${WORK}/dpkg-prev.log" 2>&1 \
            || fail "dpkg -i failed for the previous fixture"
    fi
    INSTALLED_VERSION="$(dpkg-query -W -f='${Version}' splogbook)"
    [ "${INSTALLED_VERSION}" = "1.3.0~alpha.0-1" ] \
        || fail "previous fixture version is ${INSTALLED_VERSION}"
}

install_current_package() {
    if ! dpkg -i "${DEB}" > "${WORK}/dpkg-current.log" 2>&1; then
        apt-get -y -f install > "${WORK}/apt-current.log" 2>&1 \
            || fail "apt-get -f install failed for the current package"
        dpkg -i "${DEB}" > "${WORK}/dpkg-current.log" 2>&1 \
            || fail "dpkg -i failed for the current package"
    fi
    INSTALLED_VERSION="$(dpkg-query -W -f='${Version}' splogbook)"
    [ "${INSTALLED_VERSION}" = "${EXPECTED_DEBIAN_VERSION}" ] \
        || fail "installed version is ${INSTALLED_VERSION}"
    [ "$(dpkg-query -W -f='${Status}' splogbook)" = "install ok installed" ] \
        || fail "package is not in the installed state"
    [ -z "$(dpkg --audit)" ] || fail "dpkg --audit reports an inconsistent package state"
}

echo "=== 13.1 CONTROLLED PREVIOUS FIXTURE INSTALL ==="
install_previous_fixture
[ -x /usr/bin/splogbook ] || fail "launcher is missing after the previous install"
assert_user_home_unchanged "previous fixture install"

echo "=== 13.2 UPGRADE TO THE CURRENT VERSION ==="
install_current_package
assert_user_home_unchanged "upgrade"

echo "=== 13.3 DOWNGRADE AND RE-UPGRADE ==="
dpkg -i "${PREV_DEB}" > "${WORK}/dpkg-downgrade.log" 2>&1 \
    || fail "downgrade install failed"
[ "$(dpkg-query -W -f='${Version}' splogbook)" = "1.3.0~alpha.0-1" ] \
    || fail "downgrade did not install the previous version"
dpkg -i "${DEB}" > "${WORK}/dpkg-reupgrade.log" 2>&1 \
    || fail "re-upgrade install failed"
[ "$(dpkg-query -W -f='${Version}' splogbook)" = "${EXPECTED_DEBIAN_VERSION}" ] \
    || fail "re-upgrade did not install the current version"
assert_user_home_unchanged "downgrade and re-upgrade"

echo "=== 13.4 REINSTALL THE CURRENT VERSION ==="
dpkg -i "${DEB}" > "${WORK}/dpkg-reinstall.log" 2>&1 || fail "reinstall failed"
[ "$(dpkg-query -W -f='${Version}' splogbook)" = "${EXPECTED_DEBIAN_VERSION}" ] \
    || fail "reinstall changed the version"
assert_user_home_unchanged "reinstall"

echo "=== 13.5 INSTALLED FILES AND PACKAGE QUERY ==="
dpkg-query -W -f='${Package} ${Version} ${Architecture} ${Status}\n' splogbook
for installed in \
    /opt/SPLogbook/SPLogbook \
    /opt/SPLogbook/splogbook-updater \
    /opt/SPLogbook/lib/libhamlib.so.4 \
    /opt/SPLogbook/.splogbook-deb-marker \
    /usr/bin/splogbook \
    /usr/share/applications/splogbook.desktop \
    /usr/share/icons/hicolor/256x256/apps/splogbook.png \
    /usr/share/doc/splogbook/README.Debian \
    /usr/share/doc/splogbook/copyright \
    /usr/share/doc/splogbook/changelog.Debian.gz; do
    [ -f "${installed}" ] || fail "installed file is missing: ${installed}"
done
[ -x /usr/bin/splogbook ] || fail "installed launcher is not executable"
desktop-file-validate /usr/share/applications/splogbook.desktop \
    || fail "installed desktop file is invalid"
if [ -n "${EXPECTED_ICON_SHA256}" ]; then
    [ "$(sha256sum /usr/share/icons/hicolor/256x256/apps/splogbook.png | awk '{print $1}')" = "${EXPECTED_ICON_SHA256}" ] \
        || fail "installed icon differs from the source icon"
fi
pass "all approved files are installed with the expected identity"

echo "=== 13.6 LDD RESOLUTION AFTER INSTALL ==="
for elf in \
    /opt/SPLogbook/SPLogbook \
    /opt/SPLogbook/splogbook-updater \
    /opt/SPLogbook/lib/libhamlib.so.4; do
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
pass "ldd resolves every shared library after dependency resolution"

echo "=== 13.7 LAUNCHER, VERSION, BUILD INFO AND HEALTH CHECK ==="
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
MARKER_COMMIT="$(sed -n 's/.*"build_commit": *"\([0-9a-f]\{40\}\)".*/\1/p' /opt/SPLogbook/.splogbook-deb-marker)"
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

echo "=== 13.8 REMOVE ==="
dpkg --remove splogbook > "${WORK}/dpkg-remove.log" 2>&1 || fail "dpkg --remove failed"
[ ! -e /opt/SPLogbook ] || fail "/opt/SPLogbook remains after remove"
[ ! -e /usr/bin/splogbook ] || fail "/usr/bin/splogbook remains after remove"
[ ! -e /usr/share/applications/splogbook.desktop ] || fail "desktop file remains after remove"
assert_user_home_unchanged "remove"

echo "=== 13.9 PURGE ==="
dpkg --purge splogbook > "${WORK}/dpkg-purge.log" 2>&1 || fail "dpkg --purge failed"
[ ! -e /opt/SPLogbook ] || fail "/opt/SPLogbook remains after purge"
[ ! -e /usr/bin/splogbook ] || fail "/usr/bin/splogbook remains after purge"
if dpkg-query -W splogbook >/dev/null 2>&1; then
    fail "package is still registered after purge"
fi
assert_user_home_unchanged "purge"

echo "=== 13.10 FRESH INSTALL OF THE CURRENT VERSION ==="
install_current_package
[ -f /opt/SPLogbook/SPLogbook ] || fail "binary missing after fresh install"
[ -f /usr/share/doc/splogbook/copyright ] || fail "copyright missing after fresh install"
[ -f /usr/share/doc/splogbook/changelog.Debian.gz ] || fail "changelog missing after fresh install"
assert_user_home_unchanged "fresh install"

echo "=== 13.11 FINAL REMOVE AND PURGE ==="
dpkg --remove splogbook > "${WORK}/dpkg-remove-final.log" 2>&1 || fail "final remove failed"
assert_user_home_unchanged "final remove"
dpkg --purge splogbook > "${WORK}/dpkg-purge-final.log" 2>&1 || fail "final purge failed"
assert_user_home_unchanged "final purge"
[ -z "$(dpkg --audit)" ] || fail "dpkg --audit is not clean after the lifecycle"

echo "------------------------------------------------------------------"
echo "DEB VERIFICATION COMPLETED SUCCESSFULLY"
