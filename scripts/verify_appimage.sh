#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# STAGE-M3-G: fail-closed verification of the canonical SPLogbook Linux
# x86_64 AppImage (and of an extracted AppDir in negative-fixture mode).
#
# Contract:
#   * every critical probe is fail-closed: non-zero exit, missing runtime
#     library, loader error, unexpected output or a structural violation aborts
#     the script;
#   * "APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY" is printed ONLY after all
#     critical checks passed and is the very last statement;
#   * no critical command is masked with "|| true";
#   * extraction (`--appimage-extract`) is NOT FUSE evidence; the FUSE job is
#     separate and reports its own status;
#   * the AppDir is read-only in practice: the script never writes into it and
#     proves the tree is byte-identical before and after execution.
#
# Required environment:
#   EXPECTED_BUILD_COMMIT, EXPECTED_DESKTOP_SHA256, EXPECTED_ICON_SHA256,
#   EXPECTED_APPIMAGETOOL_SHA256, EXPECTED_APPIMAGETOOL_COMMIT,
#   EXPECTED_RUNTIME_SHA256, EXPECTED_RUNTIME_COMMIT
#
# Usage:
#   verify_appimage.sh [--appdir DIR]

set -eu

APPIMAGE_PATH="${APPIMAGE_PATH:-SPLogbook-Linux-x86_64.AppImage}"
EXPECTED_VERSION="${EXPECTED_VERSION:-1.3.0-alpha.1}"
GLIBC_POLICY_MAX="2.43"

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

sha256_of() {
    sha256sum "$1" | awk '{print $1}'
}

marker_value() {
    sed -n 's/.*"'"$1"'": *"\([^"]*\)".*/\1/p' "$2" | head -n 1
}

require_env() {
    eval "value=\${$1:-}"
    [ -n "${value}" ] || fail "required environment variable $1 is missing"
}

require_env EXPECTED_BUILD_COMMIT
require_env EXPECTED_DESKTOP_SHA256
require_env EXPECTED_ICON_SHA256
require_env EXPECTED_APPIMAGETOOL_SHA256
require_env EXPECTED_APPIMAGETOOL_COMMIT
require_env EXPECTED_RUNTIME_SHA256
require_env EXPECTED_RUNTIME_COMMIT

# The ABI policy is hard-coded to 2.43. A lower value may be forced ONLY by the
# negative-fixture job (to prove that an artifact requiring more than the policy
# is rejected); the override can never raise the policy above 2.43.
GLIBC_POLICY_OVERRIDE=""
if [ "${SPLOG_APPIMAGE_NEGATIVE_FIXTURE:-0}" = "1" ]; then
    if [ -n "${SPLOG_APPIMAGE_GLIBC_MAX_OVERRIDE:-}" ]; then
        printf '%s' "${SPLOG_APPIMAGE_GLIBC_MAX_OVERRIDE}" | grep -Eq '^[0-9]+\.[0-9]+$' \
            || fail "GLIBC policy override is not a major.minor value"
        version_le "${SPLOG_APPIMAGE_GLIBC_MAX_OVERRIDE}" "${GLIBC_POLICY_MAX}" \
            || fail "GLIBC policy override may never exceed the GLIBC_${GLIBC_POLICY_MAX} policy"
        GLIBC_POLICY_OVERRIDE="${SPLOG_APPIMAGE_GLIBC_MAX_OVERRIDE}"
        GLIBC_POLICY_MAX="${GLIBC_POLICY_OVERRIDE}"
        printf 'NEGATIVE FIXTURE MODE: GLIBC policy lowered to GLIBC_%s on purpose\n' "${GLIBC_POLICY_MAX}"
    fi
fi

MODE="image"
APPDIR_ARG=""
if [ "${1:-}" = "--appdir" ]; then
    [ -n "${2:-}" ] || fail "--appdir requires a directory argument"
    MODE="appdir"
    APPDIR_ARG="$2"
elif [ -n "${1:-}" ]; then
    fail "unknown argument: $1"
fi

WORK_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/splogbook-appimage-verify.XXXXXX")"
cleanup() {
    rm -rf "${WORK_ROOT}"
}
trap cleanup EXIT

echo "=== 0. MODE AND INPUTS ==="
printf 'Mode: %s\n' "${MODE}"
printf 'Expected version: %s\n' "${EXPECTED_VERSION}"
printf 'GLIBC policy maximum: GLIBC_%s\n' "${GLIBC_POLICY_MAX}"

if [ "${MODE}" = "image" ]; then
    echo "=== 1. SHA-256 INTEGRITY CHECK ==="
    [ -f "${APPIMAGE_PATH}" ] || fail "AppImage is missing: ${APPIMAGE_PATH}"
    [ -x "${APPIMAGE_PATH}" ] || fail "AppImage is not executable: ${APPIMAGE_PATH}"
    [ -f "${APPIMAGE_PATH}.sha256" ] || fail "checksum file is missing: ${APPIMAGE_PATH}.sha256"
    EXPECTED_IMAGE_SHA256="$(awk '{print $1}' "${APPIMAGE_PATH}.sha256")"
    ACTUAL_IMAGE_SHA256="$(sha256_of "${APPIMAGE_PATH}")"
    [ "${EXPECTED_IMAGE_SHA256}" = "${ACTUAL_IMAGE_SHA256}" ] \
        || fail "SHA-256 integrity check failed (sidecar does not match the file)"
    (
        cd "$(dirname -- "${APPIMAGE_PATH}")"
        sha256sum -c "$(basename -- "${APPIMAGE_PATH}").sha256"
    ) || fail "SHA-256 integrity check failed"
    APPIMAGE_SHA256="${ACTUAL_IMAGE_SHA256}"
    printf 'AppImage SHA-256: %s\n' "${APPIMAGE_SHA256}"
    pass "AppImage SHA-256 integrity verified"

    echo "=== 2. ELF / APPARMOR / TYPE-2 IDENTITY ==="
    FILE_OUT="$(file -b "${APPIMAGE_PATH}")" || fail "file(1) failed on the AppImage"
    printf '%s\n' "${FILE_OUT}"
    case "${FILE_OUT}" in
        *"ELF 64-bit"*"x86-64"*) pass "AppImage starts with a 64-bit x86-64 ELF runtime" ;;
        *) fail "unexpected AppImage ELF identity: ${FILE_OUT}" ;;
    esac
    APPIMAGE_MAGIC="$(od -An -tx1 -j8 -N3 "${APPIMAGE_PATH}" | tr -d ' \n')"
    [ "${APPIMAGE_MAGIC}" = "414902" ] || fail "AppImage type 2 magic is missing (got ${APPIMAGE_MAGIC})"
    pass "AppImage type 2 magic verified (AI\\x02)"
    APPIMAGE_ABS="$(CDPATH= cd -- "$(dirname -- "${APPIMAGE_PATH}")" && pwd)/$(basename -- "${APPIMAGE_PATH}")"

    echo "=== 3. EXTRACTION (NOT FUSE EVIDENCE) ==="
    (
        cd "${WORK_ROOT}"
        APPIMAGE_EXTRACT_AND_RUN=0 "${APPIMAGE_ABS}" --appimage-extract >/dev/null
    ) || fail "AppImage extraction failed"
    APPDIR="${WORK_ROOT}/squashfs-root"
    [ -d "${APPDIR}" ] || fail "extraction produced no squashfs-root directory"
    pass "AppImage extracted for structural verification (FUSE is NOT evaluated here)"
else
    APPDIR="$(CDPATH= cd -- "${APPDIR_ARG}" && pwd)"
    [ -d "${APPDIR}" ] || fail "AppDir is missing: ${APPDIR_ARG}"
    pass "verifying the supplied AppDir (negative-fixture mode)"
fi

echo "=== 4. APPDIR STRUCTURE ==="
for required in \
    AppRun splogbook.desktop splogbook.png .DirIcon \
    usr/bin/SPLogbook usr/bin/splogbook-updater usr/lib/libhamlib.so.4 \
    usr/share/applications/splogbook.desktop \
    usr/share/icons/hicolor/256x256/apps/splogbook.png \
    usr/share/doc/splogbook/LICENSE usr/share/doc/splogbook/README.txt \
    usr/share/splogbook/.splogbook-appimage-marker; do
    [ -e "${APPDIR}/${required}" ] || fail "AppDir is missing ${required}"
done
[ -f "${APPDIR}/.DirIcon" ] && [ ! -L "${APPDIR}/.DirIcon" ] \
    || fail ".DirIcon must be a regular file"
pass "approved AppDir layout is complete"

echo "=== 5. SYMLINKS / SPECIAL BITS / WORLD-WRITABLE ==="
SYMLINKS="$(find "${APPDIR}" -type l -print)"
[ -z "${SYMLINKS}" ] || fail "AppDir contains symlinks: ${SYMLINKS}"
HARDLINKS="$(find "${APPDIR}" -type f -links +1 -print)"
[ -z "${HARDLINKS}" ] || fail "AppDir contains hardlinks: ${HARDLINKS}"
SPECIAL="$(find "${APPDIR}" ! -type d ! -type f -print)"
[ -z "${SPECIAL}" ] || fail "AppDir contains special files: ${SPECIAL}"
SPECIAL_BITS="$(find "${APPDIR}" -perm /6000 -print)"
[ -z "${SPECIAL_BITS}" ] || fail "AppDir contains setuid/setgid entries: ${SPECIAL_BITS}"
WORLD_WRITABLE="$(find "${APPDIR}" -type f -perm -0002 -print)"
[ -z "${WORLD_WRITABLE}" ] || fail "AppDir contains world-writable files: ${WORLD_WRITABLE}"
pass "no symlinks, hardlinks, special files, setuid/setgid or world-writable entries"

echo "=== 6. APPRUN CONTRACT ==="
[ -x "${APPDIR}/AppRun" ] || fail "AppRun is not executable"
grep -Fq '#!/bin/sh' "${APPDIR}/AppRun" || fail "AppRun does not use /bin/sh"
grep -Fq 'set -eu' "${APPDIR}/AppRun" || fail "AppRun does not use 'set -eu'"
grep -Fq 'exec "${APPDIR}/usr/bin/SPLogbook" "$@"' "${APPDIR}/AppRun" \
    || fail "AppRun does not exec usr/bin/SPLogbook with forwarded arguments"
APPRUN_CODE="$(grep -v '^[[:space:]]*#' "${APPDIR}/AppRun")"
case "${APPRUN_CODE}" in
    *sudo* | *pkexec* | *doas*) fail "AppRun must not contain sudo/pkexec/doas" ;;
esac
case "${APPRUN_CODE}" in
    *LD_LIBRARY_PATH*) fail "AppRun must not set LD_LIBRARY_PATH" ;;
esac
case "${APPRUN_CODE}" in
    *XDG_CONFIG_HOME=* | *XDG_DATA_HOME=* | *XDG_STATE_HOME=* | *XDG_CACHE_HOME=* | *HOME=*)
        fail "AppRun must not rewrite HOME or XDG_*"
        ;;
esac
pass "AppRun contract verified (POSIX sh, exec, argument forwarding, no escalation)"

echo "=== 6.1. APPRUN BEHAVIOUR (CWD INDEPENDENCE, ARGS, EXIT CODE) ==="
FAKE_ROOT="${WORK_ROOT}/apprun-behaviour"
mkdir -p "${FAKE_ROOT}/good/usr/bin" "${FAKE_ROOT}/bad"
cp "${APPDIR}/AppRun" "${FAKE_ROOT}/good/AppRun"
chmod 0755 "${FAKE_ROOT}/good/AppRun"
cat > "${FAKE_ROOT}/good/usr/bin/SPLogbook" <<'FAKE'
#!/bin/sh
printf '%s\n' "$@" > "${SPLOG_FAKE_ARGS_FILE}"
exit "${SPLOG_FAKE_EXIT_CODE}"
FAKE
chmod 0755 "${FAKE_ROOT}/good/usr/bin/SPLogbook"
SPLOG_FAKE_ARGS_FILE="${WORK_ROOT}/fake-args.txt"
SPLOG_FAKE_EXIT_CODE=37
export SPLOG_FAKE_ARGS_FILE SPLOG_FAKE_EXIT_CODE
set +e
(
    cd /
    "${FAKE_ROOT}/good/AppRun" --version --build-info "argument with spaces"
)
FAKE_CODE=$?
set -e
[ "${FAKE_CODE}" -eq 37 ] || fail "AppRun did not preserve the application exit code (got ${FAKE_CODE})"
printf '%s\n' "--version" "--build-info" "argument with spaces" > "${WORK_ROOT}/expected-args.txt"
[ -f "${SPLOG_FAKE_ARGS_FILE}" ] || fail "AppRun did not forward arguments to the application"
EXPECTED_ARGS_SHA="$(sha256_of "${WORK_ROOT}/expected-args.txt")"
ACTUAL_ARGS_SHA="$(sha256_of "${SPLOG_FAKE_ARGS_FILE}")"
if [ "${EXPECTED_ARGS_SHA}" != "${ACTUAL_ARGS_SHA}" ]; then
    printf 'Expected arguments:\n'
    cat "${WORK_ROOT}/expected-args.txt"
    printf 'Observed arguments:\n'
    cat "${SPLOG_FAKE_ARGS_FILE}"
    fail "AppRun did not forward all arguments verbatim"
fi
cp "${APPDIR}/AppRun" "${FAKE_ROOT}/bad/AppRun"
chmod 0755 "${FAKE_ROOT}/bad/AppRun"
set +e
"${FAKE_ROOT}/bad/AppRun" >/dev/null 2>&1
BAD_CODE=$?
set -e
[ "${BAD_CODE}" -ne 0 ] || fail "AppRun must reject a missing main binary"
unset SPLOG_FAKE_ARGS_FILE SPLOG_FAKE_EXIT_CODE
pass "AppRun runs from any CWD, forwards arguments verbatim, preserves exit codes and rejects a missing binary"

echo "=== 7. BINARIES AND BUNDLED LIBRARY ==="
for binary in usr/bin/SPLogbook usr/bin/splogbook-updater; do
    [ -x "${APPDIR}/${binary}" ] || fail "${binary} is not executable"
    BIN_FILE="$(file -b "${APPDIR}/${binary}")" || fail "file(1) failed on ${binary}"
    case "${BIN_FILE}" in
        *"ELF 64-bit"*"x86-64"*) ;;
        *) fail "${binary} is not a 64-bit x86-64 ELF: ${BIN_FILE}" ;;
    esac
    readelf -h "${APPDIR}/${binary}" | grep -q 'Class:[[:space:]]*ELF64' || fail "${binary} is not ELF64"
    readelf -h "${APPDIR}/${binary}" | grep -q 'Machine:.*X86-64' || fail "${binary} is not x86-64"
done
[ -f "${APPDIR}/usr/lib/libhamlib.so.4" ] || fail "libhamlib.so.4 is missing"
LIB_FILE="$(file -b "${APPDIR}/usr/lib/libhamlib.so.4")" || fail "file(1) failed on libhamlib.so.4"
case "${LIB_FILE}" in
    *"ELF 64-bit"*) ;;
    *) fail "libhamlib.so.4 is not a 64-bit ELF: ${LIB_FILE}" ;;
esac
EXTRA_LIBS="$(find "${APPDIR}/usr/lib" -maxdepth 1 -type f ! -name 'libhamlib.so.4' -print)"
[ -z "${EXTRA_LIBS}" ] || fail "only libhamlib.so.4 may be bundled, found: ${EXTRA_LIBS}"
BUNDLED_LOADER="$(find "${APPDIR}" -name 'ld-linux*' -o -name 'libc.so*' -o -name 'libm.so*' -o -name 'libgcc_s.so*' -o -name 'libstdc++.so*')"
[ -z "${BUNDLED_LOADER}" ] || fail "the loader/glibc/libm/libgcc_s/libstdc++ must not be bundled: ${BUNDLED_LOADER}"
pass "binaries are ELF64 x86-64 and libhamlib.so.4 is the only bundled library"

echo "=== 8. APPIMAGE MARKER ==="
MARKER="${APPDIR}/usr/share/splogbook/.splogbook-appimage-marker"
HEAD_BYTE="$(head -c 1 "${MARKER}")"
[ "${HEAD_BYTE}" = "{" ] || fail "marker is not a JSON object"
TAIL_BYTE="$(tail -c 2 "${MARKER}" | tr -d '\n')"
[ "${TAIL_BYTE}" = "}" ] || fail "marker JSON is not terminated"
MARKER_PACKAGE_VERSION="$(marker_value package_version "${MARKER}")"
MARKER_BUILD_COMMIT="$(marker_value build_commit "${MARKER}")"
MARKER_INSTALLATION_TYPE="$(marker_value installation_type "${MARKER}")"
MARKER_MIN_KERNEL="$(marker_value resolved_minimum_kernel "${MARKER}")"
MARKER_MAX_GLIBC="$(marker_value maximum_glibc "${MARKER}")"
MARKER_ARTIFACT_FORMAT="$(marker_value artifact_format "${MARKER}")"
MARKER_ARCHITECTURE="$(marker_value architecture "${MARKER}")"
MARKER_TOOL_CHANNEL="$(marker_value appimagetool_channel "${MARKER}")"
MARKER_TOOL_COMMIT="$(marker_value appimagetool_source_commit "${MARKER}")"
MARKER_TOOL_SHA256="$(marker_value appimagetool_sha256 "${MARKER}")"
MARKER_RUNTIME_COMMIT="$(marker_value runtime_release_or_commit "${MARKER}")"
MARKER_RUNTIME_SHA256="$(marker_value runtime_sha256 "${MARKER}")"

[ "${MARKER_PACKAGE_VERSION}" = "${EXPECTED_VERSION}" ] || fail "marker package_version mismatch"
printf '%s\n' "${MARKER_BUILD_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' || fail "marker build_commit is malformed"
[ "${MARKER_INSTALLATION_TYPE}" = "LinuxAppImage" ] || fail "marker installation_type must be LinuxAppImage"
[ "${MARKER_MIN_KERNEL}" = "7.0" ] || fail "marker resolved_minimum_kernel must be 7.0"
[ "${MARKER_MAX_GLIBC}" = "2.43" ] || fail "marker maximum_glibc must be 2.43"
[ "${MARKER_ARTIFACT_FORMAT}" = "AppImage" ] || fail "marker artifact_format must be AppImage"
[ "${MARKER_ARCHITECTURE}" = "x86_64" ] || fail "marker architecture must be x86_64"
[ "${MARKER_TOOL_CHANNEL}" = "continuous" ] || fail "marker appimagetool_channel must be continuous"
printf '%s\n' "${MARKER_TOOL_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' || fail "marker appimagetool_source_commit is malformed"
printf '%s\n' "${MARKER_TOOL_SHA256}" | grep -Eq '^[0-9a-f]{64}$' || fail "marker appimagetool_sha256 is malformed"
printf '%s\n' "${MARKER_RUNTIME_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' || fail "marker runtime_release_or_commit is malformed"
printf '%s\n' "${MARKER_RUNTIME_SHA256}" | grep -Eq '^[0-9a-f]{64}$' || fail "marker runtime_sha256 is malformed"
[ "${MARKER_TOOL_SHA256}" = "${EXPECTED_APPIMAGETOOL_SHA256}" ] \
    || fail "marker appimagetool_sha256 differs from the pinned toolchain metadata"
[ "${MARKER_TOOL_COMMIT}" = "${EXPECTED_APPIMAGETOOL_COMMIT}" ] \
    || fail "marker appimagetool_source_commit differs from the pinned toolchain metadata"
[ "${MARKER_RUNTIME_SHA256}" = "${EXPECTED_RUNTIME_SHA256}" ] \
    || fail "marker runtime_sha256 differs from the pinned runtime metadata"
[ "${MARKER_RUNTIME_COMMIT}" = "${EXPECTED_RUNTIME_COMMIT}" ] \
    || fail "marker runtime_release_or_commit differs from the pinned runtime metadata"
[ "${MARKER_BUILD_COMMIT}" = "${EXPECTED_BUILD_COMMIT}" ] \
    || fail "marker build_commit (${MARKER_BUILD_COMMIT}) differs from the CI commit (${EXPECTED_BUILD_COMMIT})"
pass "AppImage marker verified (build identity and pinned toolchain metadata)"

echo "=== 9. DESKTOP FILE ==="
for desktop in "${APPDIR}/splogbook.desktop" "${APPDIR}/usr/share/applications/splogbook.desktop"; do
    [ -f "${desktop}" ] || fail "desktop file is missing: ${desktop}"
    [ "$(sha256_of "${desktop}")" = "${EXPECTED_DESKTOP_SHA256}" ] \
        || fail "desktop file bytes differ from the canonical template: ${desktop}"
    grep -Fqx 'Exec=SPLogbook' "${desktop}" || fail "desktop Exec must be exactly Exec=SPLogbook"
    grep -Fqx 'Icon=splogbook' "${desktop}" || fail "desktop Icon must be Icon=splogbook"
    grep -Fqx 'StartupWMClass=SPLogbook' "${desktop}" || fail "desktop StartupWMClass must be SPLogbook"
    if grep -Eq '^(Exec|TryExec)=.*(sudo|pkexec|doas|sh -c|/bin/sh)' "${desktop}"; then
        fail "desktop Exec must not spawn a shell or an escalation tool"
    fi
done
command -v desktop-file-validate >/dev/null 2>&1 \
    || fail "desktop-file-validate is not available in this profile"
desktop-file-validate "${APPDIR}/splogbook.desktop" || fail "desktop-file-validate rejected the root desktop file"
desktop-file-validate "${APPDIR}/usr/share/applications/splogbook.desktop" \
    || fail "desktop-file-validate rejected the applications desktop file"
pass "desktop files are byte-identical to the canonical template and validate"

echo "=== 10. ICONS ==="
PNG_MAGIC="$(od -An -tx1 -N8 "${APPDIR}/splogbook.png" | tr -d ' \n')"
[ "${PNG_MAGIC}" = "89504e470d0a1a0a" ] || fail "splogbook.png is not a PNG file"
for icon in "${APPDIR}/splogbook.png" "${APPDIR}/.DirIcon" "${APPDIR}/usr/share/icons/hicolor/256x256/apps/splogbook.png"; do
    [ "$(sha256_of "${icon}")" = "${EXPECTED_ICON_SHA256}" ] \
        || fail "icon bytes differ from assets/icon.png: ${icon}"
done
ICON_DIMENSIONS="$(od -An -tx1 -j16 -N8 "${APPDIR}/splogbook.png" | tr -d ' \n')"
[ "${ICON_DIMENSIONS}" = "0000010000000100" ] || fail "splogbook.png must be exactly 256x256"
pass "all icon copies are byte-identical 256x256 PNG files"

echo "=== 11. USER DATA AND SECRETS ==="
FORBIDDEN="$(find "${APPDIR}" -type f \( \
    -name '*.db' -o -name '*.db-wal' -o -name '*.db-shm' -o -name 'default_log.db' \
    -o -name 'station_config.json' -o -name '*.adi' -o -name '*.adif' -o -name '*.cbr' \
    -o -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '*.pfx' \) -print)"
[ -z "${FORBIDDEN}" ] || fail "user data or secret material found in the AppDir: ${FORBIDDEN}"
if grep -rIl -E 'BEGIN [A-Z ]*PRIVATE KEY|ghp_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}' "${APPDIR}" >/dev/null 2>&1; then
    fail "secret material found inside the AppDir"
fi
pass "no user data, databases, tokens or private keys inside the AppDir"

echo "=== 12. RUNPATH ==="
for binary in usr/bin/SPLogbook usr/bin/splogbook-updater; do
    readelf -d "${APPDIR}/${binary}" | grep -Fq 'Library runpath: [$ORIGIN/../lib]' \
        || fail "${binary} does not declare RUNPATH=\$ORIGIN/../lib"
done
pass "RUNPATH is exactly \$ORIGIN/../lib for both usr/bin binaries"

echo "=== 13. LDD RESOLUTION (FAIL-CLOSED) ==="
for target in usr/bin/SPLogbook usr/bin/splogbook-updater usr/lib/libhamlib.so.4; do
    set +e
    LDD_OUT="$(ldd "${APPDIR}/${target}" 2>&1)"
    LDD_CODE=$?
    set -e
    printf '%s\n' "${LDD_OUT}"
    [ "${LDD_CODE}" -eq 0 ] || fail "ldd exited with code ${LDD_CODE} for ${target}"
    case "${LDD_OUT}" in
        *"not found"*) fail "ldd reports an unresolved shared library for ${target}" ;;
    esac
    printf '%s\n' "${LDD_OUT}" | grep -q '=> /' || fail "ldd resolved no shared library for ${target}"
done
pass "ldd resolved every direct dependency (no 'not found')"

echo "=== 14. GLIBC ABI POLICY (GLIBC_2.43) ==="
MAX_GLIBC="0.0"
for target in usr/bin/SPLogbook usr/bin/splogbook-updater usr/lib/libhamlib.so.4; do
    TARGET_MAX="$(readelf --version-info "${APPDIR}/${target}" \
        | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' | sort -V | tail -n 1)"
    [ -n "${TARGET_MAX}" ] || fail "no GLIBC versioned symbols found in ${target}"
    printf 'GLIBC requirement %s: GLIBC_%s\n' "${target}" "${TARGET_MAX}"
    if ! version_le "${TARGET_MAX}" "${MAX_GLIBC}"; then
        MAX_GLIBC="${TARGET_MAX}"
    fi
done
if [ "${MODE}" = "image" ]; then
    RUNTIME_GLIBC="$(readelf --version-info "${APPIMAGE_PATH}" 2>/dev/null \
        | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' | sort -V | tail -n 1)"
    if [ -n "${RUNTIME_GLIBC}" ]; then
        printf 'GLIBC requirement AppImage runtime: GLIBC_%s\n' "${RUNTIME_GLIBC}"
        if ! version_le "${RUNTIME_GLIBC}" "${MAX_GLIBC}"; then
            MAX_GLIBC="${RUNTIME_GLIBC}"
        fi
    else
        printf 'AppImage runtime declares no GLIBC symbols (statically linked type-2 runtime)\n'
    fi
fi
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

echo "=== 15. BINARY LAUNCH AND BUILD IDENTITY ==="
set +e
VERSION_OUT="$(cd / && "${APPDIR}/AppRun" --version 2>&1)"
VERSION_CODE=$?
set -e
printf '%s\n' "${VERSION_OUT}"
[ "${VERSION_CODE}" -eq 0 ] || fail "--version exited with code ${VERSION_CODE}"
case "${VERSION_OUT}" in
    *"error while loading shared libraries"*) fail "--version hit a loader error" ;;
esac
[ "${VERSION_OUT}" = "SPLogbook ${EXPECTED_VERSION}" ] \
    || fail "--version output is not 'SPLogbook ${EXPECTED_VERSION}'"

set +e
BUILD_OUT="$(cd / && "${APPDIR}/AppRun" --build-info 2>&1)"
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
[ "${BUILD_COMMIT}" = "${MARKER_BUILD_COMMIT}" ] \
    || fail "--build-info Commit (${BUILD_COMMIT}) differs from marker build_commit (${MARKER_BUILD_COMMIT})"
[ "${BUILD_COMMIT}" = "${EXPECTED_BUILD_COMMIT}" ] \
    || fail "--build-info Commit (${BUILD_COMMIT}) differs from the CI commit (${EXPECTED_BUILD_COMMIT})"
printf '%s\n' "${BUILD_OUT}" | grep -q '^Target: linux-x86_64$' \
    || fail "--build-info does not report Target: linux-x86_64"
pass "--version and --build-info executed and report the expected build identity"

echo "=== 16. XDG ISOLATION, HEALTH CHECK AND APPDIR IMMUTABILITY ==="
TEST_HOME="${WORK_ROOT}/home"
mkdir -p "${TEST_HOME}"
HOME="${TEST_HOME}"
XDG_CONFIG_HOME="${TEST_HOME}/.config"
XDG_DATA_HOME="${TEST_HOME}/.local/share"
XDG_STATE_HOME="${TEST_HOME}/.local/state"
XDG_CACHE_HOME="${TEST_HOME}/.cache"
export HOME XDG_CONFIG_HOME XDG_DATA_HOME XDG_STATE_HOME XDG_CACHE_HOME

TREE_BEFORE="$(cd "${APPDIR}" && find . -type f -print | LC_ALL=C sort | while IFS= read -r entry; do sha256sum "${entry}"; done | sha256sum | awk '{print $1}')"
LIST_BEFORE="$(cd "${APPDIR}" && find . -type f -print | LC_ALL=C sort)"
if [ "${MODE}" = "image" ]; then
    APPIMAGE_BEFORE="$(sha256_of "${APPIMAGE_PATH}")"
fi

mkdir -p "${XDG_DATA_HOME}/splogbook"
printf 'USER_LOG_INITIAL_DATA\n' > "${XDG_DATA_HOME}/splogbook/default_log.db"
INITIAL_DB_SHA="$(sha256_of "${XDG_DATA_HOME}/splogbook/default_log.db")"

HEALTH_PROOF="${WORK_ROOT}/health-proof.json"
set +e
HEALTH_OUT="$(cd / && "${APPDIR}/AppRun" --check-health-startup --health-proof-path "${HEALTH_PROOF}" 2>&1)"
HEALTH_CODE=$?
set -e
printf '%s\n' "${HEALTH_OUT}"
[ "${HEALTH_CODE}" -eq 0 ] || fail "health check exited with code ${HEALTH_CODE}"
[ -s "${HEALTH_PROOF}" ] || fail "health check produced no proof file at the requested XDG-independent path"

FINAL_DB_SHA="$(sha256_of "${XDG_DATA_HOME}/splogbook/default_log.db")"
[ "${INITIAL_DB_SHA}" = "${FINAL_DB_SHA}" ] || fail "user data was modified during the health check"
TREE_AFTER="$(cd "${APPDIR}" && find . -type f -print | LC_ALL=C sort | while IFS= read -r entry; do sha256sum "${entry}"; done | sha256sum | awk '{print $1}')"
LIST_AFTER="$(cd "${APPDIR}" && find . -type f -print | LC_ALL=C sort)"
[ "${TREE_BEFORE}" = "${TREE_AFTER}" ] || fail "the AppDir content changed while the application was running"
[ "${LIST_BEFORE}" = "${LIST_AFTER}" ] || fail "the AppDir listing changed while the application was running"
if [ "${MODE}" = "image" ]; then
    APPIMAGE_AFTER="$(sha256_of "${APPIMAGE_PATH}")"
    [ "${APPIMAGE_BEFORE}" = "${APPIMAGE_AFTER}" ] || fail "the AppImage file changed while it was executing"
fi
pass "XDG isolation, health check and AppDir immutability verified (tree ${TREE_AFTER})"

# The success message is the very last statement: reaching it proves that every
# critical check above passed. No check is allowed after this line.
echo "APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY"
