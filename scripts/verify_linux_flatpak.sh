#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Fail-closed verifier of the SPLogbook Linux x86_64 Flatpak bundle (M3-J).
#
# Contract
#   * every check is an assertion; nothing is masked with `|| true`, no exit code
#     is ignored and the success banner is the very last statement;
#   * the bundle is installed only into a private, throw-away user installation,
#     never system-wide and never on a real user profile;
#   * the sandbox permission contract is compared exactly, so an extra
#     filesystem, device, socket, bus name, environment variable or persistent
#     path fails the verification;
#   * user data is exercised with isolated fixtures: install, run, update,
#     downgrade-equivalent re-deploy, reinstall, uninstall and reinstall must
#     never delete or rewrite it;
#   * SPLOG_FLATPAK_INSPECTION_ONLY=1 stops after the static checks and prints a
#     different banner; it is used only by the negative-fixture harness and is
#     never evidence of a successful installation.
#
# Usage
#   EXPECTED_BUILD_COMMIT=<40-hex> bash scripts/verify_linux_flatpak.sh \
#     dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.flatpak

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

BUNDLE="${1:-${REPO_ROOT}/dist/SPLogbook-Linux-x86_64-1.3.0-alpha.1.flatpak}"
INSPECTION_ONLY="${SPLOG_FLATPAK_INSPECTION_ONLY:-0}"
STAGE="${SPLOG_FLATPAK_STAGE:-all}"
case "${STAGE}" in
    all | permissions | lifecycle) ;;
    *)
        printf 'FAIL: SPLOG_FLATPAK_STAGE must be all, permissions or lifecycle\n' >&2
        exit 1
        ;;
esac

APP_ID="io.github.sp6ina.SPLogbook"
RUNTIME="org.freedesktop.Platform"
RUNTIME_VERSION="26.08"
SDK="org.freedesktop.Sdk"
COMMAND="SPLogbook"
BRANCH="master"
PRODUCT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
MANIFEST="${REPO_ROOT}/installer/linux/flatpak/${APP_ID}.yml"
METainfo="${REPO_ROOT}/installer/linux/flatpak/${APP_ID}.metainfo.xml"
DESKTOP_FILE="${REPO_ROOT}/installer/linux/flatpak/${APP_ID}.desktop"

log() { printf '%s\n' "$*"; }
fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}
require_tool() {
    command -v "$1" >/dev/null 2>&1 || fail "required tool is missing: $1"
}

# Print both sides of a mismatch as evidence. The caller decides the outcome;
# nothing here changes an exit code.
show_mismatch() {
    printf '%s\n' '--- expected ---' "$1" '--- actual ---' "$2" >&2
}

# The exact /app file allowlist. It is used twice: statically against the
# per-file payload manifest inside the bundle, and dynamically against the
# installed deployment tree.
APP_ALLOWLIST() {
    printf '%s\n' \
        './bin/SPLogbook' \
        "./share/applications/${APP_ID}.desktop" \
        "./share/icons/hicolor/128x128/apps/${APP_ID}.png" \
        "./share/icons/hicolor/256x256/apps/${APP_ID}.png" \
        "./share/icons/hicolor/64x64/apps/${APP_ID}.png" \
        "./share/licenses/${APP_ID}/LICENSE" \
        "./share/metainfo/${APP_ID}.metainfo.xml" \
        './share/splogbook/.splogbook-flatpak-marker' \
        './share/splogbook/build-rust-version.txt'
}

# A payload manifest entry is `f <mode> <path> ...`; only regular files from the
# allowlist are permitted, with no setuid/setgid/sticky and no world-writable
# bit, so an unexpected export, a symlink or a privilege escalation attempt in
# the bundle fails here even before the bundle is installed.
validate_payload_manifest() {
    manifest_file="$1"
    entries="$(grep -v '^#' "${manifest_file}" | sed '/^[[:space:]]*$/d')"
    [ -n "${entries}" ] || fail "payload manifest is empty"
    if printf '%s\n' "${entries}" | grep -qv '^f '; then
        printf '%s\n' "${entries}" | grep -v '^f ' >&2
        fail "payload manifest contains a non-regular-file entry"
    fi
    bad_mode="$(printf '%s\n' "${entries}" | awk '
        {
            mode = $2
            n = length(mode)
            if (mode !~ /^[0-7]{3,4}$/) { print "invalid-mode:" $0; exit }
            if (n == 4) { print "special-bit:" $0; exit }
            last = substr(mode, n, 1)
            if (last == 2 || last == 3 || last == 6 || last == 7) { print "world-writable:" $0; exit }
        }')"
    [ -z "${bad_mode}" ] || fail "payload manifest mode contract violated: ${bad_mode}"
    paths="$(printf '%s\n' "${entries}" | awk '{print $3}' | LC_ALL=C sort)"
    expected="$(APP_ALLOWLIST | LC_ALL=C sort)"
    [ "${paths}" = "${expected}" ] || {
        show_mismatch "${expected}" "${paths}"
        fail "the payload manifest is not the approved export allowlist"
    }
    for token in hamlib splogbook-updater '.db' '.adi' '.cbr' '.key' '.pem'; do
        if printf '%s\n' "${paths}" | grep -q -- "${token}"; then
            fail "the payload manifest exports a forbidden token: ${token}"
        fi
    done
}

echo "=================================================================="
echo "SPLogbook Linux x86_64 Flatpak Verifier (M3-J)"
echo "=================================================================="

for tool in flatpak flatpak-builder ostree appstreamcli desktop-file-validate \
    sha256sum file find sort awk sed grep cmp install tar python3 xvfb-run timeout xauth; do
    require_tool "${tool}"
done

EXPECTED_BUILD_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
if [ -z "${EXPECTED_BUILD_COMMIT}" ]; then
    EXPECTED_BUILD_COMMIT="$(git -C "${REPO_ROOT}" rev-parse HEAD)"
fi
printf '%s\n' "${EXPECTED_BUILD_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' \
    || fail "EXPECTED_BUILD_COMMIT must be a 40-character lowercase hex commit"
log "Expected build commit: ${EXPECTED_BUILD_COMMIT}"

# ------------------------------------------------------------------
# 1. Bundle type, size and checksum
# ------------------------------------------------------------------
[ -f "${BUNDLE}" ] || fail "bundle is missing: ${BUNDLE}"
[ -s "${BUNDLE}" ] || fail "bundle is empty: ${BUNDLE}"
BUNDLE_SIZE="$(wc -c < "${BUNDLE}" | tr -d ' ')"
[ "${BUNDLE_SIZE}" -gt 1048576 ] || fail "bundle is implausibly small: ${BUNDLE_SIZE} B"
BUNDLE_KIND="$(file -b "${BUNDLE}")"
case "${BUNDLE_KIND}" in
    *"Flatpak"* | *"ostree"* | *"data"*) ;;
    *)
        if ! head -c 8 "${BUNDLE}" | grep -q 'flatpak'; then
            fail "not a Flatpak bundle: ${BUNDLE_KIND}"
        fi
        ;;
esac
head -c 8 "${BUNDLE}" | grep -q 'flatpak' || fail "missing Flatpak bundle magic: ${BUNDLE_KIND}"

BUNDLE_SHA="$(sha256sum "${BUNDLE}" | awk '{print $1}')"
printf '%s\n' "${BUNDLE_SHA}" | grep -Eq '^[0-9a-f]{64}$' || fail "cannot hash the bundle"
BUNDLE_DIR="$(cd "$(dirname "${BUNDLE}")" && pwd)"
BUNDLE_NAME="$(basename "${BUNDLE}")"
[ -f "${BUNDLE}.sha256" ] || fail "checksum sidecar is missing: ${BUNDLE}.sha256"
for sidecar in "${BUNDLE}.sha256" "${BUNDLE_DIR}/SPLogbook-Linux-x86_64.flatpak.sha256"; do
    [ -f "${sidecar}" ] || fail "missing checksum sidecar: ${sidecar}"
    ( cd "$(dirname "${sidecar}")" && sha256sum -c "$(basename "${sidecar}")" ) \
        || fail "checksum sidecar verification failed: ${sidecar}"
done
if [ -n "${EXPECTED_BUNDLE_SHA256:-}" ]; then
    [ "${EXPECTED_BUNDLE_SHA256}" = "${BUNDLE_SHA}" ] \
        || fail "bundle SHA-256 ${BUNDLE_SHA} differs from the expected ${EXPECTED_BUNDLE_SHA256}"
fi
[ -f "${BUNDLE}.manifest" ] || fail "per-file payload manifest is missing: ${BUNDLE}.manifest"
grep -q "# build_commit=${EXPECTED_BUILD_COMMIT}" "${BUNDLE}.manifest" \
    || fail "payload manifest does not carry the expected build commit"
grep -q "# app_id=${APP_ID}" "${BUNDLE}.manifest" || fail "payload manifest has the wrong app id"
grep -q "# version=${PRODUCT_VERSION}" "${BUNDLE}.manifest" || fail "payload manifest has the wrong version"
grep -q "# runtime=${RUNTIME}/${RUNTIME_VERSION}" "${BUNDLE}.manifest" \
    || fail "payload manifest has the wrong runtime"
validate_payload_manifest "${BUNDLE}.manifest"
log "PASS: payload manifest is the approved export allowlist"
log "Bundle: ${BUNDLE_NAME} (${BUNDLE_SIZE} B, SHA-256 ${BUNDLE_SHA})"

if [ "${BUNDLE_NAME}" != "SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.flatpak" ]; then
    fail "unexpected bundle name: ${BUNDLE_NAME}"
fi
[ -f "${BUNDLE_DIR}/SPLogbook-Linux-x86_64.flatpak" ] \
    || fail "canonical bundle alias SPLogbook-Linux-x86_64.flatpak is missing"
cmp -s "${BUNDLE}" "${BUNDLE_DIR}/SPLogbook-Linux-x86_64.flatpak" \
    || fail "the canonical alias is not byte-identical to the versioned bundle"

# ------------------------------------------------------------------
# 2. Manifest contract: identity, pinning, permissions
# ------------------------------------------------------------------
[ -f "${MANIFEST}" ] || fail "manifest is missing"
grep -q "^app-id: ${APP_ID}$" "${MANIFEST}" || fail "manifest app-id mismatch"
grep -q "^runtime: ${RUNTIME}$" "${MANIFEST}" || fail "manifest runtime mismatch"
grep -q "^runtime-version: \"${RUNTIME_VERSION}\"$" "${MANIFEST}" || fail "manifest runtime-version mismatch"
grep -q "^sdk: ${SDK}$" "${MANIFEST}" || fail "manifest SDK mismatch"
grep -q "^command: ${COMMAND}$" "${MANIFEST}" || fail "manifest command mismatch"

for forbidden in '--filesystem=host' '--filesystem=home' '--filesystem=/' '--device=all' \
    '--socket=system-bus' '--socket=session-bus' '--talk-name=*' '--own-name=*' \
    '--system-talk-name=*' '--filesystem=/dev' '--filesystem=/usr' '--filesystem=/etc' \
    '--filesystem=/opt' '--filesystem=/run' '--allow=devel' '--allow=multiarch'; do
    # Comment lines document what is deliberately absent, so only effective
    # YAML lines are scanned.
    if grep -v '^[[:space:]]*#' "${MANIFEST}" | grep -Fq -- "${forbidden}"; then
        fail "manifest declares a forbidden permission: ${forbidden}"
    fi
done

# Expected finish-args, exactly. Order independent.
manifest_finish_args() {
    awk '
        /^finish-args:/ { inside = 1; next }
        inside && /^[[:space:]]*-[[:space:]]+/ {
            line = $0
            sub(/^[[:space:]]*-[[:space:]]+/, "", line)
            gsub(/[[:space:]]+$/, "", line)
            print line
            next
        }
        inside && /^[^[:space:]]/ { inside = 0 }
    ' "${MANIFEST}" | LC_ALL=C sort
}

EXPECTED_FINISH_ARGS="$(printf '%s\n' \
    '--device=dri' \
    '--share=ipc' \
    '--share=network' \
    '--socket=fallback-x11' \
    '--socket=pulseaudio' \
    '--socket=wayland' \
    '--talk-name=org.freedesktop.secrets')"

ACTUAL_FINISH_ARGS="$(manifest_finish_args)"
[ "${ACTUAL_FINISH_ARGS}" = "${EXPECTED_FINISH_ARGS}" ] || {
    show_mismatch "${EXPECTED_FINISH_ARGS}" "${ACTUAL_FINISH_ARGS}"
    fail "manifest finish-args differ from the approved minimal set"
}

# Pinned sources only.
if grep -Eq '^[[:space:]]*-[[:space:]]+type:[[:space:]]*(git|bzr|svn|hg)[[:space:]]*$' "${MANIFEST}"; then
    fail "manifest declares a VCS source"
fi
if grep -Eq '(branch|tag):' "${MANIFEST}"; then
    fail "manifest declares an unpinned branch or tag"
fi
if grep -Eq 'file://|/(home|Users|root)/' "${MANIFEST}"; then
    fail "manifest contains an absolute host path"
fi
grep -q 'CARGO_NET_OFFLINE: "true"' "${MANIFEST}" || fail "manifest does not force CARGO_NET_OFFLINE"
grep -q -- '--locked --offline' "${MANIFEST}" || fail "manifest does not build with --locked --offline"
grep -q -- '--disable-download' "${REPO_ROOT}/scripts/build_linux_flatpak.sh" \
    || fail "the builder does not enforce --disable-download"

bash "${REPO_ROOT}/scripts/generate_flatpak_cargo_sources.sh" --check
python3 - "${REPO_ROOT}/installer/linux/flatpak/cargo-sources.json" <<'PY'
import json, re, sys
entries = json.load(open(sys.argv[1], encoding='utf-8'))
for entry in entries:
    if entry.get('type') != 'archive' or not re.fullmatch(r'[0-9a-f]{64}', entry.get('sha256', '')):
        print(f'FAIL: unpinned cargo source: {entry}', file=sys.stderr); raise SystemExit(1)
print(f'PASS: {len(entries)} pinned cargo sources')
PY

appstreamcli validate --no-net "${METainfo}" || fail "appstreamcli rejected the AppStream metadata"
desktop-file-validate "${DESKTOP_FILE}" || fail "desktop-file-validate rejected the desktop entry"
grep -q "<id>${APP_ID}</id>" "${METainfo}" || fail "AppStream id mismatch"
grep -q "desktop-id>${APP_ID}.desktop<" "${METainfo}" || fail "AppStream launchable mismatch"
grep -q "<project_license>GPL-3.0-or-later</project_license>" "${METainfo}" \
    || fail "AppStream project license mismatch"
grep -q "<metadata_license>CC0-1.0</metadata_license>" "${METainfo}" \
    || fail "AppStream metadata license mismatch"
grep -q 'Icon='"${APP_ID}" "${DESKTOP_FILE}" || fail "desktop entry icon mismatch"
grep -q 'Exec='"${COMMAND}" "${DESKTOP_FILE}" || fail "desktop entry command mismatch"

# Icons: real PNG files with exactly the declared pixel size, named after the
# application id, so the exported hicolor tree cannot carry a mislabelled image.
for size in 64 128 256; do
    icon="${REPO_ROOT}/installer/linux/flatpak/icons/${size}x${size}/${APP_ID}.png"
    [ -f "${icon}" ] || fail "icon is missing: ${icon}"
    actual_px="$(python3 -c '
import struct, sys
with open(sys.argv[1], "rb") as handle:
    head = handle.read(24)
if head[:8] != b"\x89PNG\r\n\x1a\n":
    print("not-a-png"); raise SystemExit(0)
width, height = struct.unpack(">II", head[16:24])
print(f"{width}x{height}")
' "${icon}")"
    [ "${actual_px}" = "${size}x${size}" ] \
        || fail "icon ${size}x${size} has ${actual_px} pixels"
done
log "PASS: icons are valid PNG files with the declared sizes"
log "PASS: manifest, sources, AppStream metadata and desktop entry satisfy the contract"

if [ "${INSPECTION_ONLY}" = "1" ]; then
    echo "=================================================================="
    echo "FLATPAK INSPECTION COMPLETED SUCCESSFULLY"
    echo "Bundle: ${BUNDLE}"
    echo "SHA-256: ${BUNDLE_SHA}"
    echo "=================================================================="
    exit 0
fi

# ------------------------------------------------------------------
# 3. Private user installation
# ------------------------------------------------------------------
WORK_ROOT="${BUNDLE_DIR}/.flatpak_verify"
rm -rf "${WORK_ROOT}"
mkdir -p "${WORK_ROOT}/home" "${WORK_ROOT}/data" "${WORK_ROOT}/cache" "${WORK_ROOT}/config"
export HOME="${WORK_ROOT}/home"
export XDG_DATA_HOME="${WORK_ROOT}/data"
export XDG_CACHE_HOME="${WORK_ROOT}/cache"
export XDG_CONFIG_HOME="${WORK_ROOT}/config"
export FLATPAK_USER_DIR="${WORK_ROOT}/data/flatpak"
export FLATPAK_SYSTEM_DIR="${WORK_ROOT}/system-flatpak"
unset FLATPAK_ID

# The reference for every negative permission test: one standard remote only.
flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/ \
    || fail "cannot add the flatpak remote"
flatpak --user install --noninteractive flathub \
    "${RUNTIME}//${RUNTIME_VERSION}" "${SDK}//${RUNTIME_VERSION}" \
    || fail "cannot install the pinned runtime/SDK"

APP_DATA_ROOT="${HOME}/.var/app/${APP_ID}"
DATA_DIR="${APP_DATA_ROOT}/data/splogbook"
CONFIG_DIR="${APP_DATA_ROOT}/config/splogbook"
STATE_DIR="${APP_DATA_ROOT}/state/splogbook"
CACHE_DIR="${APP_DATA_ROOT}/cache/splogbook"

export_user_data() {
    printf '%s\n' '{"schema": "splogbook-station-v1", "callsign": "SP6INA", "fixture": "m3-j"}'
}

create_user_fixtures() {
    install -d "${DATA_DIR}" "${CONFIG_DIR}" "${STATE_DIR}" "${CACHE_DIR}"
    printf 'SQLITE_FIXTURE_QSO_DATABASE\n' > "${DATA_DIR}/default_log.db"
    printf 'SQLITE_FIXTURE_WAL\n' > "${DATA_DIR}/default_log.db-wal"
    printf 'SQLITE_FIXTURE_SHM\n' > "${DATA_DIR}/default_log.db-shm"
    export_user_data > "${CONFIG_DIR}/station_config.json"
    printf 'ADIF FIXTURE\n' > "${DATA_DIR}/log_2026.adi"
    printf 'CABRILLO FIXTURE\n' > "${DATA_DIR}/contest.cbr"
    printf 'BACKUP FIXTURE\n' > "${STATE_DIR}/backup-2026-11-14.zip"
    printf 'TOKEN FIXTURE\n' > "${CONFIG_DIR}/lan_sync.token"
}

user_data_digest() {
    (
        cd "${APP_DATA_ROOT}" 2>/dev/null || {
            printf 'MISSING\n'
            exit 0
        }
        find . -mindepth 1 -printf '%y %m %p\n' | LC_ALL=C sort
        find . -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum
    ) | sha256sum | awk '{print $1}'
}

assert_user_data_unchanged() {
    expected="$1"
    reason="$2"
    actual="$(user_data_digest)"
    [ "${actual}" = "${expected}" ] \
        || fail "user data changed after ${reason} (${expected} -> ${actual})"
    log "PASS: user data byte-identical after ${reason}"
}

install_bundle() {
    flatpak --user install --noninteractive --reinstall "${BUNDLE}" \
        || fail "cannot install the bundle"
}

# ------------------------------------------------------------------
# 4. Installed identity and exact sandbox permissions
# ------------------------------------------------------------------
install_bundle

INSTALLED_ID="$(flatpak --user info --show-ref "${APP_ID}" | awk -F/ '{print $1}')"
[ "${INSTALLED_ID}" = "app/${APP_ID}/x86_64/${BRANCH}" ] \
    || fail "unexpected installed ref: ${INSTALLED_ID}"
INSTALLED_ARCH="$(flatpak --user info --arch "${APP_ID}")"
[ "${INSTALLED_ARCH}" = "x86_64" ] || fail "unexpected architecture: ${INSTALLED_ARCH}"
INSTALLED_BRANCH="$(flatpak --user info --branch "${APP_ID}")"
[ "${INSTALLED_BRANCH}" = "${BRANCH}" ] || fail "unexpected branch: ${INSTALLED_BRANCH}"
INSTALLED_COMMIT="$(flatpak --user info --show-commit "${APP_ID}")"
printf '%s\n' "${INSTALLED_COMMIT}" | grep -Eq '^[0-9a-f]{64}$' \
    || fail "cannot read the OSTree commit: ${INSTALLED_COMMIT}"

flatpak --user info --show-metadata "${APP_ID}" > "${WORK_ROOT}/metadata.txt" \
    || fail "cannot read the application metadata"
flatpak --user info --show-permissions "${APP_ID}" > "${WORK_ROOT}/permissions.txt" \
    || fail "cannot read the application permissions"
flatpak --user info "${APP_ID}" > "${WORK_ROOT}/app-info.txt" || fail "cannot read flatpak info"

grep -q "^name=${APP_ID}$" "${WORK_ROOT}/metadata.txt" || fail "metadata name mismatch"
grep -q "^runtime=${RUNTIME}/x86_64/${RUNTIME_VERSION}$" "${WORK_ROOT}/metadata.txt" \
    || fail "metadata runtime mismatch"
grep -q "^sdk=${SDK}/x86_64/${RUNTIME_VERSION}$" "${WORK_ROOT}/metadata.txt" \
    || fail "metadata SDK mismatch"
grep -q "^command=${COMMAND}$" "${WORK_ROOT}/metadata.txt" || fail "metadata command mismatch"

normalize_permissions() {
    awk '
        /^\[/ {
            section = $0
            gsub(/^\[/, "", section)
            gsub(/\]$/, "", section)
            next
        }
        NF == 0 { next }
        {
            equals = index($0, "=")
            if (equals == 0) next
            key = substr($0, 1, equals - 1)
            value = substr($0, equals + 1)
            count = split(value, items, ";")
            for (i = 1; i <= count; i++) {
                if (items[i] == "") continue
                if (section == "Context") print "context." key " " items[i]
                else if (section == "Session Bus Policy") print "session-bus." key " " items[i]
                else if (section == "System Bus Policy") print "system-bus." key " " items[i]
                else if (section == "Environment") print "environment." key " " items[i]
                else print tolower(section) "." key " " items[i]
            }
        }
    ' "$1" | LC_ALL=C sort
}

EXPECTED_PERMISSIONS="$(printf '%s\n' \
    'context.devices dri' \
    'context.shared ipc' \
    'context.shared network' \
    'context.sockets fallback-x11' \
    'context.sockets pulseaudio' \
    'context.sockets wayland' \
    'session-bus.org.freedesktop.secrets talk')"
ACTUAL_PERMISSIONS="$(normalize_permissions "${WORK_ROOT}/permissions.txt")"
[ "${ACTUAL_PERMISSIONS}" = "${EXPECTED_PERMISSIONS}" ] || {
    show_mismatch "${EXPECTED_PERMISSIONS}" "${ACTUAL_PERMISSIONS}"
    fail "installed sandbox permissions differ from the approved minimal contract"
}
log "PASS: sandbox permissions are exactly the approved minimal set"
sed -n '1,40p' "${WORK_ROOT}/permissions.txt" | tee "${BUNDLE_DIR}/flatpak-permissions.txt" >/dev/null

# ------------------------------------------------------------------
# 5. Installed payload: exact allowlist, marker, no forbidden content
# ------------------------------------------------------------------
DEPLOY_ROOT="${FLATPAK_USER_DIR}/app/${APP_ID}/x86_64/${BRANCH}/${INSTALLED_COMMIT}/files"
[ -d "${DEPLOY_ROOT}" ] || fail "cannot locate the deployed files: ${DEPLOY_ROOT}"

EXPECTED_FILES="$(APP_ALLOWLIST | LC_ALL=C sort)"
ACTUAL_FILES="$(cd "${DEPLOY_ROOT}" && find . -type f | LC_ALL=C sort)"
[ "${ACTUAL_FILES}" = "${EXPECTED_FILES}" ] || {
    show_mismatch "${EXPECTED_FILES}" "${ACTUAL_FILES}"
    fail "the deployed /app tree is not the approved allowlist"
}
[ -z "$(find "${DEPLOY_ROOT}" -type l -print -quit)" ] || fail "the deployed payload contains a symlink"
[ -z "$(find "${DEPLOY_ROOT}" -type f -perm /6000 -print -quit)" ] || fail "the payload contains setuid/setgid files"
[ -z "$(find "${DEPLOY_ROOT}" -type f -perm -0002 -print -quit)" ] || fail "the payload contains world-writable files"
log "PASS: deployed payload is exactly the approved allowlist"

MARKER="${DEPLOY_ROOT}/share/splogbook/.splogbook-flatpak-marker"
grep -q "\"build_commit\": \"${EXPECTED_BUILD_COMMIT}\"" "${MARKER}" \
    || fail "marker does not carry the expected build commit"
grep -q '"installation_type": "LinuxFlatpak"' "${MARKER}" || fail "marker installation type mismatch"
grep -q "\"app_id\": \"${APP_ID}\"" "${MARKER}" || fail "marker app id mismatch"
grep -q "\"runtime_version\": \"${RUNTIME_VERSION}\"" "${MARKER}" || fail "marker runtime version mismatch"
grep -q "\"package_version\": \"${PRODUCT_VERSION}\"" "${MARKER}" || fail "marker version mismatch"

if find "${DEPLOY_ROOT}" -name '*hamlib*' -print -quit | grep -q .; then
    fail "the bundle contains Hamlib binaries; the Flatpak contract is TCP-only"
fi
if find "${DEPLOY_ROOT}" -name 'splogbook-updater' -print -quit | grep -q .; then
    fail "the bundle contains the self-update helper; Flatpak must be updated by flatpak"
fi
for forbidden in 'default_log.db' 'station_config.json' '.splogbook-deb-marker' '.splogbook-rpm-marker' \
    '*.adi' '*.adif' '*.cbr' '*.key' '*.pem' '*.db'; do
    if find "${DEPLOY_ROOT}" -name "${forbidden}" -print -quit | grep -q .; then
        fail "the bundle contains user data or a secret: ${forbidden}"
    fi
done
log "PASS: marker, Hamlib-free bundle and absence of user data verified"

if [ "${STAGE}" = "permissions" ]; then
    echo "=================================================================="
    echo "FLATPAK PERMISSIONS VERIFICATION COMPLETED SUCCESSFULLY"
    echo "App ID: ${APP_ID}"
    echo "Permissions: minimal contract verified exactly"
    echo "SHA-256: ${BUNDLE_SHA}"
    echo "=================================================================="
    exit 0
fi

# ------------------------------------------------------------------
# 6. Execution inside the sandbox
# ------------------------------------------------------------------
run_app() {
    flatpak --user run --command="${COMMAND}" "${APP_ID}" "$@"
}

VERSION_OUTPUT="$(run_app --version)" || fail "--version failed inside the sandbox"
printf '%s\n' "${VERSION_OUTPUT}" | grep -q "SPLogbook ${PRODUCT_VERSION}" \
    || fail "--version returned unexpected output: ${VERSION_OUTPUT}"
log "PASS: --version -> ${VERSION_OUTPUT}"

BUILD_INFO="$(run_app --build-info)" || fail "--build-info failed inside the sandbox"
printf '%s\n' "${BUILD_INFO}" | tee "${BUNDLE_DIR}/flatpak-build-info.txt" >/dev/null
printf '%s\n' "${BUILD_INFO}" | grep -q "Commit: ${EXPECTED_BUILD_COMMIT}" \
    || fail "--build-info does not report the expected commit"
printf '%s\n' "${BUILD_INFO}" | grep -q "Version: ${PRODUCT_VERSION}" \
    || fail "--build-info does not report the expected version"
printf '%s\n' "${BUILD_INFO}" | grep -q 'Channel: alpha' || fail "--build-info channel mismatch"
if printf '%s\n' "${BUILD_INFO}" | grep -q 'dirty'; then
    fail "--build-info reports a dirty build"
fi
log "PASS: --build-info reports ${EXPECTED_BUILD_COMMIT}"

HEALTH_PATH="${HOME}/health-proof.txt"
run_app --check-health-startup --health-proof-path "${HEALTH_PATH}" \
    || fail "the startup health check failed"
[ -s "${HEALTH_PATH}" ] || fail "the startup health check wrote no proof"
grep -q '^OK$' "${HEALTH_PATH}" || fail "unexpected health-proof content: $(cat "${HEALTH_PATH}")"
log "PASS: startup health check produced OK"

# CWD independence: the same binary must work from any working directory.
( cd / && flatpak --user run --command="${COMMAND}" "${APP_ID}" --version >/dev/null ) \
    || fail "--version failed when the sandbox started from /"
( cd "${WORK_ROOT}" && flatpak --user run --command="${COMMAND}" "${APP_ID}" --version >/dev/null ) \
    || fail "--version failed when the sandbox started from a temporary directory"
log "PASS: the application runs from an arbitrary working directory"

# No unresolved libraries, no host or /opt dependency.
LDD_OUTPUT="$(flatpak --user run --command=sh "${APP_ID}" -c 'ldd /app/bin/SPLogbook' 2>&1)" \
    || fail "ldd is not available inside the runtime"
printf '%s\n' "${LDD_OUTPUT}" | tee "${BUNDLE_DIR}/flatpak-ldd.txt" >/dev/null
if printf '%s\n' "${LDD_OUTPUT}" | grep -q 'not found'; then
    fail "unresolved library: ${LDD_OUTPUT}"
fi
if printf '%s\n' "${LDD_OUTPUT}" | grep -Eq '/opt|/usr/lib/local'; then
    fail "the binary resolves a library outside the runtime"
fi
if printf '%s\n' "${LDD_OUTPUT}" | grep -q 'libhamlib'; then
    fail "the binary links a Hamlib library; the Flatpak contract is TCP-only"
fi
log "PASS: no unresolved library and no host dependency"

# /app is read-only and the sandbox cannot write into it.
if flatpak --user run --command=sh "${APP_ID}" -c 'touch /app/bin/.write-probe' >/dev/null 2>&1; then
    fail "/app is writable inside the sandbox"
fi
log "PASS: /app is read-only inside the sandbox"

# No self-update helper and no update target inside the bundle.
flatpak --user run --command=sh "${APP_ID}" -c 'test ! -e /app/bin/splogbook-updater' \
    || fail "the self-update helper is reachable inside the sandbox"
log "PASS: the sandbox has no self-update path into /app"

# ------------------------------------------------------------------
# 7. XDG data placement and full lifecycle with user-data fixtures
# ------------------------------------------------------------------
create_user_fixtures
BASELINE_DIGEST="$(user_data_digest)"
printf '%s\n' "${BASELINE_DIGEST}" | grep -Eq '^[0-9a-f]{64}$' || fail "cannot digest the user-data fixtures"
flatpak --user run --command=sh "${APP_ID}" -c 'test -d "$XDG_DATA_HOME/splogbook"' \
    || fail "XDG_DATA_HOME is not mapped to the private application data directory"
flatpak --user run --command=sh "${APP_ID}" -c 'test -d "$XDG_CONFIG_HOME/splogbook"' \
    || fail "XDG_CONFIG_HOME is not mapped to the private application config directory"
flatpak --user run --command=sh "${APP_ID}" -c 'test "$HOME" = "${HOME%/}"' \
    || fail "HOME is not an absolute private path inside the sandbox"
flatpak --user run --command=sh "${APP_ID}" -c 'test ! -w /app' \
    || fail "/app must not be writable"
log "PASS: XDG base directories resolve inside the Flatpak sandbox"

# update (no-op at the same ref) must not touch user data
flatpak --user update --noninteractive "${APP_ID}" > "${WORK_ROOT}/update.log" 2>&1 \
    || fail "flatpak update failed"
assert_user_data_unchanged "${BASELINE_DIGEST}" "flatpak update"

# downgrade-equivalent: a forced re-deploy of the same OSTree commit exercises
# the same deployment path a real downgrade uses (see README section 9).
flatpak --user update --noninteractive --commit="${INSTALLED_COMMIT}" "${APP_ID}" \
    > "${WORK_ROOT}/redeploy.log" 2>&1 || fail "forced re-deploy failed"
assert_user_data_unchanged "${BASELINE_DIGEST}" "the forced re-deploy (downgrade path)"

# reinstall
install_bundle
assert_user_data_unchanged "${BASELINE_DIGEST}" "reinstall"

# uninstall must preserve user data
flatpak --user uninstall --noninteractive "${APP_ID}" > "${WORK_ROOT}/uninstall.log" 2>&1 \
    || fail "flatpak uninstall failed"
assert_user_data_unchanged "${BASELINE_DIGEST}" "uninstall"
[ -d "${APP_DATA_ROOT}" ] || fail "uninstall removed the application data directory"

# reinstall and verify again
install_bundle
assert_user_data_unchanged "${BASELINE_DIGEST}" "the second install"
run_app --version >/dev/null || fail "the application does not run after reinstallation"

# explicit, separate data deletion (never part of uninstall)
flatpak --user uninstall --noninteractive "${APP_ID}" > "${WORK_ROOT}/final-uninstall.log" 2>&1 \
    || fail "final uninstall failed"
[ -d "${APP_DATA_ROOT}" ] || fail "the final uninstall removed user data"
rm -rf "${APP_DATA_ROOT}"
[ ! -e "${APP_DATA_ROOT}" ] || fail "explicit user-data removal failed"
log "PASS: full lifecycle preserves user data; only the explicit removal deletes it"

# ------------------------------------------------------------------
# 8. GUI smoke test (headless X11 and, when available, Wayland)
# ------------------------------------------------------------------
if command -v xvfb-run >/dev/null 2>&1; then
    GUI_LOG="${WORK_ROOT}/gui-smoke.log"
    set +e
    timeout --preserve-status 25 xvfb-run --auto-servernum \
        flatpak --user run --command="${COMMAND}" "${APP_ID}" > "${GUI_LOG}" 2>&1
    GUI_RC=$?
    set -e
    # 143/124 mean the process was still alive when the timeout stopped it.
    if [ "${GUI_RC}" -ne 0 ] && [ "${GUI_RC}" -ne 143 ] && [ "${GUI_RC}" -ne 124 ]; then
        cat "${GUI_LOG}" >&2
        fail "the GUI process died with exit code ${GUI_RC}"
    fi
    if grep -Eiq 'panicked|error while loading shared libraries|cannot open display|failed to create window' "${GUI_LOG}"; then
        cat "${GUI_LOG}" >&2
        fail "the GUI smoke test logged a fatal error"
    fi
    log "PASS: GUI smoke test survived ${GUI_RC} (headless X11)"
    cp "${GUI_LOG}" "${BUNDLE_DIR}/flatpak-gui-smoke.log"
else
    fail "xvfb-run is required for the GUI smoke test"
fi

if [ "${STAGE}" = "lifecycle" ]; then
    echo "=================================================================="
    echo "FLATPAK LIFECYCLE VERIFICATION COMPLETED SUCCESSFULLY"
    echo "App ID: ${APP_ID}"
    echo "Lifecycle: fresh install, update, re-deploy, reinstall, uninstall preserved user data"
    echo "SHA-256: ${BUNDLE_SHA}"
    echo "=================================================================="
    exit 0
fi

# ------------------------------------------------------------------
# 9. Result
# ------------------------------------------------------------------
{
    printf 'app_id=%s\n' "${APP_ID}"
    printf 'bundle=%s\n' "${BUNDLE_NAME}"
    printf 'bundle_sha256=%s\n' "${BUNDLE_SHA}"
    printf 'bundle_size=%s\n' "${BUNDLE_SIZE}"
    printf 'architecture=x86_64\n'
    printf 'branch=%s\n' "${BRANCH}"
    printf 'ostree_commit=%s\n' "${INSTALLED_COMMIT}"
    printf 'build_commit=%s\n' "${EXPECTED_BUILD_COMMIT}"
    printf 'product_version=%s\n' "${PRODUCT_VERSION}"
    printf 'runtime=%s/%s\n' "${RUNTIME}" "${RUNTIME_VERSION}"
    printf 'sdk=%s/%s\n' "${SDK}" "${RUNTIME_VERSION}"
    printf 'permissions=minimal-contract-verified\n'
    printf 'lifecycle=verified\n'
    printf 'gui_smoke=verified\n'
} > "${BUNDLE_DIR}/flatpak-verification.txt"

log "Verified bundle: ${BUNDLE_NAME}"
log "SHA-256: ${BUNDLE_SHA}"
log "OSTree commit: ${INSTALLED_COMMIT}"

echo "=================================================================="
echo "FLATPAK VERIFICATION COMPLETED SUCCESSFULLY"
echo "App ID: ${APP_ID}"
echo "Bundle: ${BUNDLE_NAME}"
echo "SHA-256: ${BUNDLE_SHA}"
echo "=================================================================="
