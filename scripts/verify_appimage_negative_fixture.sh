#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# STAGE-M3-G: prove that scripts/verify_appimage.sh rejects every mutated
# fixture. The job succeeds ONLY when all fixtures were rejected with a
# non-zero exit code and without the success message.
#
# Fixtures:
#   1. missing `.sha256`
#   2. wrong checksum
#   3. truncated image
#   4. missing AppRun
#   5. non-executable AppRun
#   6. missing main binary
#   7. dangerous desktop Exec
#   8. missing icon
#   9. user data inside the image
#  10. symlink escaping the AppDir
#  11. wrong build_commit
#  12. artifact requiring more than GLIBC_2.43

set -eu

VERIFIER="${VERIFIER:-/workspace/verify_appimage.sh}"
APPIMAGE_PATH="${APPIMAGE_PATH:-/workspace/SPLogbook-Linux-x86_64.AppImage}"

[ -f "${VERIFIER}" ] || { printf 'FAIL: verifier is missing: %s\n' "${VERIFIER}" >&2; exit 1; }
[ -f "${APPIMAGE_PATH}" ] || { printf 'FAIL: AppImage is missing: %s\n' "${APPIMAGE_PATH}" >&2; exit 1; }

WORK_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-appimage-negative.XXXXXX")"
cleanup() {
    rm -rf "${WORK_ROOT}"
}
trap cleanup EXIT
mkdir -p "${WORK_ROOT}/logs" "${WORK_ROOT}/image-mutations"

APPIMAGE_ABS="$(CDPATH= cd -- "$(dirname -- "${APPIMAGE_PATH}")" && pwd)/$(basename -- "${APPIMAGE_PATH}")"

TOTAL=0
REJECTED=0

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

expect_reject() {
    name="$1"
    shift
    TOTAL=$((TOTAL + 1))
    log="${WORK_ROOT}/logs/${name}.log"
    set +e
    "$@" >"${log}" 2>&1
    code=$?
    set -e
    if [ "${code}" -eq 0 ]; then
        printf '%s\n' "----- ${name} log -----"
        cat "${log}"
        fail "fixture '${name}' was NOT rejected (exit code 0)"
    fi
    if grep -q 'APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY' "${log}"; then
        printf '%s\n' "----- ${name} log -----"
        cat "${log}"
        fail "fixture '${name}' emitted the success message despite being invalid"
    fi
    printf 'REJECTED %-32s exit=%-3s %s\n' "${name}" "${code}" \
        "$(tail -n 1 "${log}" | cut -c1-120)"
    REJECTED=$((REJECTED + 1))
}

echo "=== PREPARING THE NEGATIVE FIXTURES ==="
(
    cd "${WORK_ROOT}"
    APPIMAGE_EXTRACT_AND_RUN=0 "${APPIMAGE_ABS}" --appimage-extract >/dev/null
) || fail "cannot extract the reference AppImage for fixture synthesis"
REFERENCE_APPDIR="${WORK_ROOT}/squashfs-root"
[ -d "${REFERENCE_APPDIR}" ] || fail "reference extraction produced no AppDir"

FIXTURE="${WORK_ROOT}/fixture-appdir"
cp -a "${REFERENCE_APPDIR}" "${FIXTURE}"
CANONICAL_DESKTOP="${WORK_ROOT}/canonical-splogbook.desktop"
CANONICAL_MARKER="${WORK_ROOT}/canonical-marker.json"
cp -a "${FIXTURE}/splogbook.desktop" "${CANONICAL_DESKTOP}"
cp -a "${FIXTURE}/usr/share/splogbook/.splogbook-appimage-marker" "${CANONICAL_MARKER}"

verify_image_at() {
    APPIMAGE_PATH="$1" sh "${VERIFIER}"
}

verify_appdir_at() {
    APPIMAGE_PATH="${APPIMAGE_ABS}" sh "${VERIFIER}" --appdir "$1"
}

echo "=== IMAGE-LEVEL MUTATIONS ==="

mkdir -p "${WORK_ROOT}/image-mutations/missing-sha256"
cp "${APPIMAGE_ABS}" "${WORK_ROOT}/image-mutations/missing-sha256/SPLogbook-Linux-x86_64.AppImage"
chmod 0755 "${WORK_ROOT}/image-mutations/missing-sha256/SPLogbook-Linux-x86_64.AppImage"
expect_reject "missing-sha256" verify_image_at "${WORK_ROOT}/image-mutations/missing-sha256/SPLogbook-Linux-x86_64.AppImage"

mkdir -p "${WORK_ROOT}/image-mutations/wrong-sha256"
cp "${APPIMAGE_ABS}" "${WORK_ROOT}/image-mutations/wrong-sha256/SPLogbook-Linux-x86_64.AppImage"
chmod 0755 "${WORK_ROOT}/image-mutations/wrong-sha256/SPLogbook-Linux-x86_64.AppImage"
printf '%s  %s\n' \
    "0000000000000000000000000000000000000000000000000000000000000000" \
    "SPLogbook-Linux-x86_64.AppImage" \
    > "${WORK_ROOT}/image-mutations/wrong-sha256/SPLogbook-Linux-x86_64.AppImage.sha256"
expect_reject "wrong-sha256" verify_image_at "${WORK_ROOT}/image-mutations/wrong-sha256/SPLogbook-Linux-x86_64.AppImage"

mkdir -p "${WORK_ROOT}/image-mutations/truncated"
head -c 2097152 "${APPIMAGE_ABS}" > "${WORK_ROOT}/image-mutations/truncated/SPLogbook-Linux-x86_64.AppImage"
chmod 0755 "${WORK_ROOT}/image-mutations/truncated/SPLogbook-Linux-x86_64.AppImage"
TRUNCATED_SHA="$(sha256sum "${WORK_ROOT}/image-mutations/truncated/SPLogbook-Linux-x86_64.AppImage" | awk '{print $1}')"
printf '%s  %s\n' "${TRUNCATED_SHA}" "SPLogbook-Linux-x86_64.AppImage" \
    > "${WORK_ROOT}/image-mutations/truncated/SPLogbook-Linux-x86_64.AppImage.sha256"
expect_reject "truncated-image" verify_image_at "${WORK_ROOT}/image-mutations/truncated/SPLogbook-Linux-x86_64.AppImage"

expect_reject "glibc-above-policy" env \
    SPLOG_APPIMAGE_NEGATIVE_FIXTURE=1 \
    SPLOG_APPIMAGE_GLIBC_MAX_OVERRIDE=2.17 \
    APPIMAGE_PATH="${APPIMAGE_ABS}" \
    sh "${VERIFIER}"

echo "=== APPDIR-LEVEL MUTATIONS ==="

echo "--- 4. missing AppRun ---"
mv "${FIXTURE}/AppRun" "${FIXTURE}/AppRun.saved"
expect_reject "missing-apprun" verify_appdir_at "${FIXTURE}"
mv "${FIXTURE}/AppRun.saved" "${FIXTURE}/AppRun"

echo "--- 5. non-executable AppRun ---"
chmod 0644 "${FIXTURE}/AppRun"
expect_reject "apprun-not-executable" verify_appdir_at "${FIXTURE}"
chmod 0755 "${FIXTURE}/AppRun"

echo "--- 6. missing main binary ---"
mv "${FIXTURE}/usr/bin/SPLogbook" "${WORK_ROOT}/SPLogbook.saved"
expect_reject "missing-main-binary" verify_appdir_at "${FIXTURE}"
mv "${WORK_ROOT}/SPLogbook.saved" "${FIXTURE}/usr/bin/SPLogbook"

echo "--- 7. dangerous desktop Exec ---"
sed 's/^Exec=SPLogbook$/Exec=sudo sh -c SPLogbook/' "${CANONICAL_DESKTOP}" > "${WORK_ROOT}/dangerous.desktop"
cp "${WORK_ROOT}/dangerous.desktop" "${FIXTURE}/splogbook.desktop"
cp "${WORK_ROOT}/dangerous.desktop" "${FIXTURE}/usr/share/applications/splogbook.desktop"
DANGEROUS_DESKTOP_SHA="$(sha256sum "${WORK_ROOT}/dangerous.desktop" | awk '{print $1}')"
expect_reject "dangerous-desktop-exec" env \
    EXPECTED_DESKTOP_SHA256="${DANGEROUS_DESKTOP_SHA}" \
    APPIMAGE_PATH="${APPIMAGE_ABS}" \
    sh "${VERIFIER}" --appdir "${FIXTURE}"
cp "${CANONICAL_DESKTOP}" "${FIXTURE}/splogbook.desktop"
cp "${CANONICAL_DESKTOP}" "${FIXTURE}/usr/share/applications/splogbook.desktop"

echo "--- 8. missing icon ---"
mv "${FIXTURE}/usr/share/icons/hicolor/256x256/apps/splogbook.png" "${WORK_ROOT}/splogbook.png.saved"
expect_reject "missing-icon" verify_appdir_at "${FIXTURE}"
mv "${WORK_ROOT}/splogbook.png.saved" "${FIXTURE}/usr/share/icons/hicolor/256x256/apps/splogbook.png"

echo "--- 9. user data inside the image ---"
printf 'USER_QSO_DATA\n' > "${FIXTURE}/usr/share/splogbook/default_log.db"
expect_reject "user-data-in-image" verify_appdir_at "${FIXTURE}"
rm -f "${FIXTURE}/usr/share/splogbook/default_log.db"

echo "--- 10. symlink escaping the AppDir ---"
ln -s /etc/passwd "${FIXTURE}/usr/lib/escape"
expect_reject "symlink-escape" verify_appdir_at "${FIXTURE}"
rm -f "${FIXTURE}/usr/lib/escape"

echo "--- 11. wrong build_commit ---"
sed 's/"build_commit": "[0-9a-f]\{40\}"/"build_commit": "1111111111111111111111111111111111111111"/' \
    "${CANONICAL_MARKER}" > "${FIXTURE}/usr/share/splogbook/.splogbook-appimage-marker"
expect_reject "wrong-build-commit" verify_appdir_at "${FIXTURE}"
cp "${CANONICAL_MARKER}" "${FIXTURE}/usr/share/splogbook/.splogbook-appimage-marker"

echo "=== NEGATIVE FIXTURE SUMMARY ==="
printf 'Rejected %s/%s fixtures\n' "${REJECTED}" "${TOTAL}"
if [ "${REJECTED}" -ne "${TOTAL}" ]; then
    fail "not every negative fixture was rejected"
fi
printf '%s\n' "NEGATIVE FIXTURES REJECTED AS EXPECTED: ${REJECTED}/${TOTAL}"
