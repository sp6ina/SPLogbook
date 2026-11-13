#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Deterministic plain-text manifest for a SPLogbook Linux RPM (M3-I).
#
# Usage:
#   linux_rpm_manifest.sh <rpm-file> <output> <package> <version> <release> <arch> <commit>
#
# The manifest describes the exact content of the RPM as installed: artifact
# SHA-256 and size, package identity, every packaged directory and every
# regular file with its mode, numeric owner/group and SHA-256 digest.  It is
# deliberately free of timestamps, host names and locale-dependent output.

set -eu

if [ "$#" -ne 7 ]; then
    printf 'usage: %s <rpm> <output> <package> <version> <release> <arch> <commit>\n' "$0" >&2
    exit 1
fi

RPM="$1"
OUTPUT="$2"
PACKAGE="$3"
VERSION="$4"
RELEASE="$5"
ARCH="$6"
COMMIT="$7"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

[ -f "${RPM}" ] || fail "RPM does not exist: ${RPM}"
[ "$(id -u)" -eq 0 ] || fail "manifest generation requires root to preserve RPM ownership"
printf '%s\n' "${PACKAGE}" | grep -Eq '^[a-z0-9][a-z0-9+._-]*$' \
    || fail "invalid package name: ${PACKAGE}"
printf '%s\n' "${VERSION}" | grep -Eq '^[0-9][0-9A-Za-z.+~_]*$' \
    || fail "invalid version: ${VERSION}"
printf '%s\n' "${RELEASE}" | grep -Eq '^[0-9][0-9A-Za-z.+~_]*$' \
    || fail "invalid release: ${RELEASE}"
[ "${ARCH}" = "x86_64" ] || fail "architecture must be x86_64"
printf '%s\n' "${COMMIT}" | grep -Eq '^[0-9a-f]{40}$' \
    || fail "build commit must be 40 lowercase hex characters"

for tool in rpm rpm2cpio cpio sort stat sha256sum wc awk sed mktemp; do
    command -v "${tool}" >/dev/null 2>&1 || fail "required tool is missing: ${tool}"
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-rpm-manifest.XXXXXX")"
cleanup() {
    rm -rf "${WORK}"
}
trap cleanup EXIT HUP INT TERM

ROOT="${WORK}/root"
mkdir -p "${ROOT}"
( cd "${ROOT}" && rpm2cpio "${RPM}" | cpio -idm --quiet --no-absolute-filenames ) \
    || fail "cannot extract the RPM for manifest generation"

ARTIFACT_SHA="$(sha256sum "${RPM}" | awk '{print $1}')"
ARTIFACT_SIZE="$(wc -c < "${RPM}" | tr -d ' ')"
[ -n "${ARTIFACT_SHA}" ] || fail "cannot compute artifact SHA-256"
[ -n "${ARTIFACT_SIZE}" ] || fail "cannot compute artifact size"

ENTRIES="${WORK}/entries.txt"
FILE_COUNT=0
ENTRY_COUNT=0

rpm -qpl "${RPM}" | LC_ALL=C sort > "${WORK}/paths.txt"
while IFS= read -r rel; do
    case "${rel}" in
        /*) ;;
        *) fail "RPM path is not absolute: ${rel}" ;;
    esac
    case "${rel}" in
        *" "*|*"	"*) fail "RPM path contains whitespace: ${rel}" ;;
    esac
    path="${ROOT}${rel}"
    kind="$(stat -c '%F' "${path}")"
    mode="$(stat -c '%a' "${path}")"
    uid="$(stat -c '%u' "${path}")"
    gid="$(stat -c '%g' "${path}")"
    case "${kind}" in
        directory)
            printf 'D %s %s %s %s\n' "${mode}" "${uid}" "${gid}" "${rel}" >> "${ENTRIES}"
            ENTRY_COUNT=$((ENTRY_COUNT + 1))
            ;;
        "regular file"|"regular empty file")
            digest="$(sha256sum "${path}" | awk '{print $1}')"
            printf 'F %s %s %s %s %s\n' "${mode}" "${uid}" "${gid}" "${digest}" "${rel}" >> "${ENTRIES}"
            FILE_COUNT=$((FILE_COUNT + 1))
            ENTRY_COUNT=$((ENTRY_COUNT + 1))
            ;;
        *)
            fail "unsupported package entry type (${kind}): ${rel}"
            ;;
    esac
done < "${WORK}/paths.txt"

{
    printf 'SPLOGBOOK_RPM_MANIFEST 1.0\n'
    printf 'package=%s\n' "${PACKAGE}"
    printf 'version=%s\n' "${VERSION}"
    printf 'release=%s\n' "${RELEASE}"
    printf 'arch=%s\n' "${ARCH}"
    printf 'build_commit=%s\n' "${COMMIT}"
    printf 'artifact_sha256=%s\n' "${ARTIFACT_SHA}"
    printf 'artifact_size=%s\n' "${ARTIFACT_SIZE}"
    printf 'file_count=%s\n' "${FILE_COUNT}"
    printf 'entry_count=%s\n' "${ENTRY_COUNT}"
    printf 'ENTRIES\n'
    cat "${ENTRIES}"
} > "${OUTPUT}"

chmod 0644 "${OUTPUT}"
printf 'manifest: %s (%s files, %s entries)\n' "${OUTPUT}" "${FILE_COUNT}" "${ENTRY_COUNT}"
