#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Wo?niak (SP6INA)
#
# Build a disposable RPM fixture from an already extracted package root.
#
# Usage:
#   build_linux_rpm_fixture.sh <root> <output.rpm> <name> <version> <release> <arch> [extra-spec]
#
# This helper is used only by the RPM verifier (controlled previous-version
# fixture) and by the negative fixture harness.  It is not the production
# builder: the production path is scripts/build_linux_rpm.sh, which starts from
# the verified portable payload and the reviewed spec.

set -eu

if [ "$#" -lt 6 ] || [ "$#" -gt 7 ]; then
    printf 'usage: %s <root> <output.rpm> <name> <version> <release> <arch> [extra-spec]\n' "$0" >&2
    exit 1
fi

ROOT="$1"
OUTPUT="$2"
NAME="$3"
VERSION="$4"
RELEASE="$5"
ARCH="$6"
EXTRA_SPEC="${7:-}"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

[ -d "${ROOT}" ] || fail "fixture root does not exist: ${ROOT}"
[ -n "${NAME}" ] || fail "fixture name is empty"
[ -n "${VERSION}" ] || fail "fixture version is empty"
[ -n "${RELEASE}" ] || fail "fixture release is empty"
[ -n "${ARCH}" ] || fail "fixture arch is empty"
if [ -n "${EXTRA_SPEC}" ]; then
    [ -f "${EXTRA_SPEC}" ] || fail "extra spec does not exist: ${EXTRA_SPEC}"
fi
for tool in rpmbuild find sort stat sed awk cp; do
    command -v "${tool}" >/dev/null 2>&1 || fail "required tool is missing: ${tool}"
done

ROOT="$(cd "${ROOT}" && pwd)"
OUTPUT_DIR="$(dirname "${OUTPUT}")"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"
OUTPUT="${OUTPUT_DIR}/$(basename "${OUTPUT}")"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-rpm-fixture.XXXXXX")"
cleanup() {
    rm -rf "${WORK}"
}
trap cleanup EXIT HUP INT TERM

TOP="${WORK}/rpmbuild"
mkdir -p "${TOP}/BUILD" "${TOP}/BUILDROOT" "${TOP}/RPMS" \
    "${TOP}/SOURCES" "${TOP}/SPECS" "${TOP}/SRPMS"
cp -a "${ROOT}/." "${TOP}/SOURCES/root"

SPEC="${TOP}/SPECS/${NAME}.spec"
{
    printf 'Name: %s\n' "${NAME}"
    printf 'Version: %s\n' "${VERSION}"
    printf 'Release: %s\n' "${RELEASE}"
    printf 'Summary: SPLogbook RPM fixture\n'
    printf 'License: GPL-3.0-or-later\n'
    printf 'BuildArch: %s\n' "${ARCH}"
    printf 'AutoReqProv: no\n'
    printf '%%global use_source_date_epoch_as_buildtime 1\n'
    printf '%%global clamp_mtime_to_source_date_epoch 1\n'
    printf '%%global _buildhost build.splogbook.invalid\n'
    printf '%%global debug_package %%{nil}\n'
    printf '%%global _build_id_links none\n'
    printf '%%global __os_install_post %%{nil}\n'
    printf '%%global _binary_payload w6.xzdio\n'
    printf '%%description\nDisposable fixture for M3-I testing.\n'
    printf '%%prep\n: fixture\n'
    printf '%%build\n: fixture\n'
    printf '%%install\n'
    printf 'rm -rf "%%{buildroot}"\n'
    printf 'install -d "%%{buildroot}"\n'
    printf 'cp -a "%%{_sourcedir}/root/." "%%{buildroot}/"\n'
    if [ -n "${EXTRA_SPEC}" ]; then
        cat "${EXTRA_SPEC}"
        printf '\n'
    fi
    printf '%%files\n%%defattr(-,root,root,-)\n'
    find "${ROOT}" -mindepth 1 -print | LC_ALL=C sort | while IFS= read -r path; do
        rel="${path#"${ROOT}"}"
        case "${rel}" in
            *" "*|*"	"*) fail "fixture path contains whitespace: ${rel}" ;;
        esac
        case "${rel}" in
            /opt|/usr|/usr/bin|/usr/share|/usr/share/applications|\
            /usr/share/doc|/usr/share/licenses|\
            /usr/share/icons|/usr/share/icons/hicolor|\
            /usr/share/icons/hicolor/256x256|\
            /usr/share/icons/hicolor/256x256/apps)
                continue
                ;;
        esac
        kind="$(stat -c '%F' "${path}")"
        mode="$(stat -c '%a' "${path}")"
        uid="$(stat -c '%u' "${path}")"
        gid="$(stat -c '%g' "${path}")"
        case "${kind}" in
            directory)
                printf '%%dir %%attr(%s,%s,%s) %s\n' "${mode}" "${uid}" "${gid}" "${rel}"
                ;;
            "regular file"|"regular empty file")
                printf '%%attr(%s,%s,%s) %s\n' "${mode}" "${uid}" "${gid}" "${rel}"
                ;;
            "symbolic link")
                printf '%%attr(0777,%s,%s) %s\n' "${uid}" "${gid}" "${rel}"
                ;;
            *)
                fail "unsupported fixture entry type (${kind}): ${rel}"
                ;;
        esac
    done
    printf '%%changelog\n* Fri Nov 13 2026 Mariusz Wo?niak (SP6INA) <contact@splogbook.org> - %s-%s\n- Disposable RPM fixture.\n' "${VERSION}" "${RELEASE}"
} > "${SPEC}"

# Fixed build identity; the fixture is never a production artifact.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-1783900800}"

rpmbuild -bb \
    --define "_topdir ${TOP}" \
    --define "_sourcedir ${TOP}/SOURCES" \
    --define "_builddir ${TOP}/BUILD" \
    --define "_buildrootdir ${TOP}/BUILDROOT" \
    --define "_rpmdir ${TOP}/RPMS" \
    --define "_specdir ${TOP}/SPECS" \
    --define "_srcrpmdir ${TOP}/SRPMS" \
    "${SPEC}" > "${WORK}/rpmbuild.log" 2>&1 \
    || {
        cat "${WORK}/rpmbuild.log" >&2
        fail "rpmbuild failed for fixture ${NAME}-${VERSION}-${RELEASE}.${ARCH}"
    }

BUILT="$(find "${TOP}/RPMS" -type f -name '*.rpm' -print | head -n 1)"
[ -n "${BUILT}" ] || fail "fixture rpmbuild produced no RPM"
cp "${BUILT}" "${OUTPUT}"
chmod 0644 "${OUTPUT}"
printf 'fixture: %s\n' "${OUTPUT}"
