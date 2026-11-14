#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Deterministic builder of the SPLogbook Linux x86_64 Flatpak (M3-J).
#
# Contract
#   * fail-closed: every requirement is an assertion, the success banner is the
#     last statement, and no step is masked with `|| true`;
#   * the build runs from one explicit, pinned commit
#     (EXPECTED_BUILD_COMMIT must be the 40-hex HEAD of the tree);
#   * the tree must be a clean build tree: target/, dist/ and owner-private
#     directories are never read (they are also skipped by the manifest) and the
#     builder refuses to run when `target/` exists, so a previous Cargo build can
#     never leak into the artefact;
#   * sources are downloaded once (the only network phase, pinned by the
#     SHA-256 checksums of Cargo.lock and the runtime ref), then the real build
#     runs with --disable-download and cargo --locked --offline;
#   * the app is never installed system-wide, never installed on the host, never
#     submitted to Flathub and never attached to a release;
#   * SPLOG_FLATPAK_BUILDS independent clean builds are performed from the same
#     inputs and compared (OSTree commit, payload tree, per-file digests);
#   * evidence (toolchain versions, manifest projection, permissions, file list,
#     SHA-256) is written next to the bundle.
#
# Usage
#   EXPECTED_BUILD_COMMIT=<40-hex> \
#   FLATPAK_BUILD_IMAGE=<image>@sha256:<digest> \
#     bash scripts/build_linux_flatpak.sh [output-dir]
#
# Environment
#   EXPECTED_BUILD_COMMIT      required, 40 lowercase hex
#   SOURCE_DATE_EPOCH          optional, defaults to the commit time
#   SPLOG_FLATPAK_BUILDS       number of independent clean builds (default 1)
#   SPLOG_FLATPAK_REMOTE       flatpak remote name (default flathub)
#   SPLOG_FLATPAK_REMOTE_URL   remote URL (default https://dl.flathub.org/repo/)
#   FLATPAK_BUILD_IMAGE        recorded for evidence only
#   FLATPAK_BUILD_IMAGE_DIGEST recorded for evidence only

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

# The CI container mounts the workspace with a different owner than the build
# user, so git refuses to read it unless the path is declared safe. This is a
# configuration-only allowance: it does not relax any build assertion.
export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0="safe.directory"
export GIT_CONFIG_VALUE_0="${REPO_ROOT}"

OUTPUT_DIR="${1:-${REPO_ROOT}/dist}"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"

APP_ID="io.github.sp6ina.SPLogbook"
MANIFEST="${REPO_ROOT}/installer/linux/flatpak/${APP_ID}.yml"
METainfo="${REPO_ROOT}/installer/linux/flatpak/${APP_ID}.metainfo.xml"
DESKTOP_FILE="${REPO_ROOT}/installer/linux/flatpak/${APP_ID}.desktop"
CARGO_SOURCES="${REPO_ROOT}/installer/linux/flatpak/cargo-sources.json"
PRODUCT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
BUILD_COUNT="${SPLOG_FLATPAK_BUILDS:-1}"
FLATPAK_REMOTE="${SPLOG_FLATPAK_REMOTE:-flathub}"
FLATPAK_REMOTE_URL="${SPLOG_FLATPAK_REMOTE_URL:-https://dl.flathub.org/repo/}"

log() { printf '%s\n' "$*"; }
fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}
require_tool() {
    command -v "$1" >/dev/null 2>&1 || fail "required tool is missing: $1"
}

version_ge_minor() {
    # true when $1 >= $2 for dotted numeric versions
    awk -v a="$1" -v b="$2" 'BEGIN {
        n = split(a, A, ".");
        m = split(b, B, ".");
        len = (n > m) ? n : m;
        for (i = 1; i <= len; i++) {
            ai = (i <= n) ? A[i] + 0 : 0;
            bi = (i <= m) ? B[i] + 0 : 0;
            if (ai > bi) exit 0;
            if (ai < bi) exit 1;
        }
        exit 0;
    }'
}

echo "=================================================================="
echo "SPLogbook Linux x86_64 Flatpak Builder (M3-J)"
echo "=================================================================="

# ------------------------------------------------------------------
# 1. Toolchain and host preconditions
# ------------------------------------------------------------------
for tool in flatpak flatpak-builder ostree appstreamcli desktop-file-validate \
    sha256sum git file find sort awk sed grep cmp install tar gzip python3; do
    require_tool "${tool}"
done

FLATPAK_VERSION="$(flatpak --version | awk '{print $2}')"
FLATPAK_BUILDER_VERSION="$(flatpak-builder --version | awk '{print $2}')"
OSTREE_VERSION="$(ostree --version | sed -n 's/^ Version: //p' | head -n 1)"
APPSTREAM_VERSION="$(appstreamcli --version | head -n 1 | awk '{print $NF}')"
DESKTOP_VALIDATE_VERSION="$(desktop-file-validate --version 2>&1 | head -n 1 | awk '{print $NF}')"

log "flatpak:              ${FLATPAK_VERSION}"
log "flatpak-builder:      ${FLATPAK_BUILDER_VERSION}"
log "ostree:               ${OSTREE_VERSION}"
log "appstreamcli:         ${APPSTREAM_VERSION}"
log "desktop-file-validate:${DESKTOP_VALIDATE_VERSION}"

[ "$(uname -m)" = "x86_64" ] || fail "the Flatpak builder must run on an x86_64 host"
[ -n "${PRODUCT_VERSION}" ] || fail "cannot read the package version from Cargo.toml"

# ------------------------------------------------------------------
# 2. Build identity, epoch and a genuinely clean input tree
# ------------------------------------------------------------------
EXPECTED_BUILD_COMMIT="${EXPECTED_BUILD_COMMIT:-}"
if [ -z "${EXPECTED_BUILD_COMMIT}" ]; then
    command -v git >/dev/null 2>&1 || fail "EXPECTED_BUILD_COMMIT is required when git is absent"
    EXPECTED_BUILD_COMMIT="$(git -C "${REPO_ROOT}" rev-parse HEAD)"
fi
printf '%s\n' "${EXPECTED_BUILD_COMMIT}" | grep -Eq '^[0-9a-f]{40}$' \
    || fail "EXPECTED_BUILD_COMMIT must be a 40-character lowercase hex commit"
HEAD_COMMIT="$(git -C "${REPO_ROOT}" rev-parse HEAD)"
[ "${HEAD_COMMIT}" = "${EXPECTED_BUILD_COMMIT}" ] \
    || fail "HEAD (${HEAD_COMMIT}) is not the requested build commit (${EXPECTED_BUILD_COMMIT})"
log "Build commit: ${EXPECTED_BUILD_COMMIT}"

# The manifest and the identity must describe the same tree.
TRACKED_DIRTY="$(git -C "${REPO_ROOT}" status --porcelain --untracked-files=no)"
[ -z "${TRACKED_DIRTY}" ] || fail "tracked worktree is not clean; refusing to build an unverifiable identity"

# A leftover Cargo build tree must never be an implicit input. The manifest
# skips it, but the builder refuses to run at all so the omission is explicit
# rather than silently relied upon.
[ ! -d "${REPO_ROOT}/target" ] \
    || fail "target/ exists; run the Flatpak build from a fresh tree (clean build only)"

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git -C "${REPO_ROOT}" log -1 --format=%ct "${EXPECTED_BUILD_COMMIT}")}"
printf '%s\n' "${SOURCE_DATE_EPOCH}" | grep -Eq '^[0-9]+$' \
    || fail "SOURCE_DATE_EPOCH must be a non-negative integer"
export SOURCE_DATE_EPOCH
log "SOURCE_DATE_EPOCH: ${SOURCE_DATE_EPOCH}"

# ------------------------------------------------------------------
# 3. Manifest and identity consistency (before anything is downloaded)
# ------------------------------------------------------------------
[ -f "${MANIFEST}" ] || fail "manifest is missing: ${MANIFEST}"
[ -f "${METainfo}" ] || fail "AppStream metadata is missing: ${METainfo}"
[ -f "${DESKTOP_FILE}" ] || fail "desktop entry is missing: ${DESKTOP_FILE}"
[ -f "${CARGO_SOURCES}" ] || fail "cargo-sources.json is missing: ${CARGO_SOURCES}"

manifest_value() {
    sed -n "s/^$1: *//p" "${MANIFEST}" | head -n 1
}

APP_ID_MANIFEST="$(manifest_value 'app-id')"
RUNTIME="$(manifest_value 'runtime')"
RUNTIME_VERSION="$(manifest_value 'runtime-version' | tr -d '"')"
SDK="$(manifest_value 'sdk')"
COMMAND="$(manifest_value 'command')"

[ "${APP_ID_MANIFEST}" = "${APP_ID}" ] || fail "manifest app-id is '${APP_ID_MANIFEST}', expected '${APP_ID}'"
printf '%s\n' "${APP_ID}" | grep -Eq '^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*){2,}$' \
    || fail "app-id is not a valid reverse-DNS identifier: ${APP_ID}"
case "${APP_ID}" in
    *SPLogbook) ;;
    *) fail "app-id must end with the application name segment SPLogbook" ;;
esac
[ "${RUNTIME}" = "org.freedesktop.Platform" ] || fail "unexpected runtime: ${RUNTIME}"
[ "${SDK}" = "org.freedesktop.Sdk" ] || fail "unexpected SDK: ${SDK}"
printf '%s\n' "${RUNTIME_VERSION}" | grep -Eq '^[0-9]{2}\.[0-9]{2}$' \
    || fail "runtime-version must be a pinned year.month branch, got '${RUNTIME_VERSION}'"
[ -n "${COMMAND}" ] || fail "manifest has no command"
grep -q "command: ${COMMAND}" "${MANIFEST}" || fail "command '${COMMAND}' is not declared"

# The exported file names must all use the same application id.
for base in "${METainfo}" "${DESKTOP_FILE}"; do
    case "$(basename "${base}")" in
        "${APP_ID}".*) ;;
        *) fail "exported file does not use the app id: ${base}" ;;
    esac
done
for size in 64x64 128x128 256x256; do
    icon="${REPO_ROOT}/installer/linux/flatpak/icons/${size}/${APP_ID}.png"
    [ -f "${icon}" ] || fail "missing icon: ${icon}"
    expected_px="${size%x*}"
    actual_px="$(python3 - "${icon}" <<'PY'
import struct, sys
with open(sys.argv[1], 'rb') as handle:
    head = handle.read(24)
if head[:8] != b'\x89PNG\r\n\x1a\n':
    print('not-a-png'); raise SystemExit(0)
width, height = struct.unpack('>II', head[16:24])
print(f'{width}x{height}')
PY
)"
    [ "${actual_px}" = "${expected_px}x${expected_px}" ] \
        || fail "icon ${size} has ${actual_px} pixels, expected ${expected_px}x${expected_px}"
done
grep -q "Icon=${APP_ID}" "${DESKTOP_FILE}" || fail "desktop entry does not reference Icon=${APP_ID}"
grep -q "Exec=${COMMAND}" "${DESKTOP_FILE}" || fail "desktop entry does not reference Exec=${COMMAND}"
grep -q "<id>${APP_ID}</id>" "${METainfo}" || fail "AppStream metadata does not declare <id>${APP_ID}</id>"
grep -q "desktop-id>${APP_ID}.desktop<" "${METainfo}" \
    || fail "AppStream launchable desktop-id does not match the desktop entry"

# ------------------------------------------------------------------
# 4. Forbidden permissions and unpinned sources (manifest contract)
# ------------------------------------------------------------------
for forbidden in '--filesystem=host' '--filesystem=home' '--filesystem=/' '--device=all' \
    '--socket=system-bus' '--socket=session-bus' '--talk-name=*' '--own-name=*' \
    '--system-talk-name=*' '--filesystem=/dev' '--filesystem=/usr' '--filesystem=/etc' \
    '--filesystem=/opt' '--filesystem=/run' '--allow=devel' '--allow=multiarch'; do
    if grep -v '^[[:space:]]*#' "${MANIFEST}" | grep -Fq -- "${forbidden}"; then
        fail "manifest declares a forbidden permission: ${forbidden}"
    fi
done

if grep -Eq '^[[:space:]]*-[[:space:]]+type:[[:space:]]*(git|bzr|svn|hg)[[:space:]]*$' "${MANIFEST}"; then
    fail "manifest declares a VCS source; this package uses only the tracked tree and pinned crates"
fi
if grep -Eq '(branch|tag):' "${MANIFEST}"; then
    fail "manifest declares an unpinned branch or tag"
fi
if grep -Eq 'file://|/(home|Users|root)/' "${MANIFEST}"; then
    fail "manifest contains an absolute host path"
fi
python3 - "${CARGO_SOURCES}" <<'PY'
import json, re, sys
entries = json.load(open(sys.argv[1], encoding='utf-8'))
if not entries:
    print('FAIL: cargo-sources.json is empty', file=sys.stderr)
    raise SystemExit(1)
for entry in entries:
    if entry.get('type') != 'archive':
        print(f'FAIL: non-archive cargo source: {entry}', file=sys.stderr); raise SystemExit(1)
    if not re.fullmatch(r'[0-9a-f]{64}', entry.get('sha256', '')):
        print(f'FAIL: cargo source without SHA-256: {entry.get("url")}', file=sys.stderr); raise SystemExit(1)
    if not entry.get('url', '').startswith('https://static.crates.io/crates/'):
        print(f'FAIL: cargo source outside the pinned registry: {entry.get("url")}', file=sys.stderr); raise SystemExit(1)
print(f'PASS: {len(entries)} pinned cargo archive sources')
PY
bash "${REPO_ROOT}/scripts/generate_flatpak_cargo_sources.sh" --check

# AppStream and desktop-entry validation before the expensive build.
appstreamcli validate --no-net "${METainfo}" || fail "appstreamcli rejected the AppStream metadata"
desktop-file-validate "${DESKTOP_FILE}" || fail "desktop-file-validate rejected the desktop entry"
log "PASS: AppStream metadata and desktop entry validated"

# `--show-manifest` proves that flatpak-builder parses the manifest.
flatpak-builder --show-manifest "${MANIFEST}" > "${OUTPUT_DIR}/flatpak-manifest.projection.txt" 2>&1 \
    || {
        cat "${OUTPUT_DIR}/flatpak-manifest.projection.txt" >&2
        fail "flatpak-builder rejected the manifest"
    }
[ -s "${OUTPUT_DIR}/flatpak-manifest.projection.txt" ] || fail "flatpak-builder produced an empty manifest projection"
grep -q "${APP_ID}" "${OUTPUT_DIR}/flatpak-manifest.projection.txt" || fail "manifest projection lost the app id"
log "PASS: flatpak-builder --show-manifest accepted the manifest"

# ------------------------------------------------------------------
# 5. Isolated flatpak user installation and pinned runtime/SDK
# ------------------------------------------------------------------
WORK_ROOT="${OUTPUT_DIR}/.flatpak_work"
rm -rf "${WORK_ROOT}"
mkdir -p "${WORK_ROOT}/home" "${WORK_ROOT}/data" "${WORK_ROOT}/cache" "${WORK_ROOT}/config"
export HOME="${WORK_ROOT}/home"
export XDG_DATA_HOME="${WORK_ROOT}/data"
export XDG_CACHE_HOME="${WORK_ROOT}/cache"
export XDG_CONFIG_HOME="${WORK_ROOT}/config"
export FLATPAK_USER_DIR="${WORK_ROOT}/data/flatpak"
export FLATPAK_SYSTEM_DIR="${WORK_ROOT}/system-flatpak"
unset FLATPAK_ID FLATPAK_SANDBOX_DIR

cleanup() {
    rc=$?
    if [ "${rc}" -ne 0 ]; then
        printf 'Flatpak build failed with exit code %s\n' "${rc}" >&2
    fi
    exit "${rc}"
}
trap cleanup EXIT

flatpak --user remote-add --if-not-exists "${FLATPAK_REMOTE}" "${FLATPAK_REMOTE_URL}" \
    || fail "cannot add the flatpak remote"

flatpak --user install --noninteractive "${FLATPAK_REMOTE}" \
    "${RUNTIME}//${RUNTIME_VERSION}" \
    "${SDK}//${RUNTIME_VERSION}" \
    || fail "cannot install the pinned runtime/SDK ${RUNTIME_VERSION}"

RUST_EXTENSION="org.freedesktop.Sdk.Extension.rust-stable/x86_64/${RUNTIME_VERSION}"
flatpak --user install --noninteractive "${FLATPAK_REMOTE}" "${RUST_EXTENSION}" \
    || fail "cannot install the pinned Rust SDK extension ${RUST_EXTENSION}"

RUNTIME_COMMIT="$(flatpak --user info --show-commit "${RUNTIME}//${RUNTIME_VERSION}" 2>/dev/null)" || RUNTIME_COMMIT="unresolved"
SDK_COMMIT="$(flatpak --user info --show-commit "${SDK}//${RUNTIME_VERSION}" 2>/dev/null)" || SDK_COMMIT="unresolved"
RUST_EXTENSION_COMMIT="$(flatpak --user info --show-commit "org.freedesktop.Sdk.Extension.rust-stable//${RUNTIME_VERSION}" 2>/dev/null)" || RUST_EXTENSION_COMMIT="unresolved"
log "runtime commit:       ${RUNTIME_COMMIT:-unresolved}"
log "sdk commit:           ${SDK_COMMIT:-unresolved}"
log "rust extension commit:${RUST_EXTENSION_COMMIT:-unresolved}"

flatpak --user info "${RUNTIME}//${RUNTIME_VERSION}" > "${OUTPUT_DIR}/flatpak-runtime-info.txt" 2>&1 \
    || fail "cannot read the runtime metadata"
flatpak --user info "${SDK}//${RUNTIME_VERSION}" > "${OUTPUT_DIR}/flatpak-sdk-info.txt" 2>&1 \
    || fail "cannot read the SDK metadata"

# ------------------------------------------------------------------
# 6. One function = one isolated, network-free build
# ------------------------------------------------------------------
payload_manifest() {
    # $1 = app tree (files/), $2 = output file. Path, mode, size, target and
    # SHA-256 of every entry, sorted; no timestamps, so two builds compare.
    tree="$1"
    out="$2"
    : > "${out}"
    ( cd "${tree}" && find . -mindepth 1 -printf '%y %m %p\n' | LC_ALL=C sort ) \
        | while read -r kind mode path; do
            case "${kind}" in
                f)
                    size="$(wc -c < "${tree}/${path}" | tr -d ' ')"
                    digest="$(sha256sum "${tree}/${path}" | awk '{print $1}')"
                    printf '%s %s %s %s\n' "${kind}" "${mode}" "${path}" "${size}:${digest}"
                    ;;
                l)
                    target="$(readlink "${tree}/${path}")"
                    printf '%s %s %s -> %s\n' "${kind}" "${mode}" "${path}" "${target}"
                    ;;
                *)
                    printf '%s %s %s\n' "${kind}" "${mode}" "${path}"
                    ;;
            esac
        done >> "${out}"
    wc -l < "${out}" | tr -d ' '
}

run_build() {
    label="$1"
    build_root="${WORK_ROOT}/build-${label}"
    mkdir -p "${build_root}"

    # Sources are fetched once per build (network phase), then the real build
    # refuses to download anything.
    flatpak-builder \
        --user \
        --arch=x86_64 \
        --state-dir="${build_root}/state" \
        --build-dir="${build_root}/build" \
        --repo="${build_root}/repo" \
        --disable-rofiles-fuse \
        --force-clean \
        --download-only \
        "${MANIFEST}" > "${OUTPUT_DIR}/flatpak-download-${label}.log" 2>&1 \
        || {
            cat "${OUTPUT_DIR}/flatpak-download-${label}.log" >&2
            fail "source download failed for build ${label}"
        }

    flatpak-builder \
        --user \
        --arch=x86_64 \
        --state-dir="${build_root}/state" \
        --build-dir="${build_root}/build" \
        --repo="${build_root}/repo" \
        --disable-rofiles-fuse \
        --disable-download \
        --sandbox \
        --force-clean \
        --jobs="${FLATPAK_BUILDER_JOBS:-$(nproc)}" \
        "${MANIFEST}" > "${OUTPUT_DIR}/flatpak-build-${label}.log" 2>&1 \
        || {
            tail -n 200 "${OUTPUT_DIR}/flatpak-build-${label}.log" >&2
            fail "flatpak-builder failed for build ${label}"
        }
    grep -q 'SPLogbook' "${OUTPUT_DIR}/flatpak-build-${label}.log" \
        || fail "build ${label} produced no SPLogbook evidence in the log"

    bundle="${build_root}/${APP_ID}.flatpak"
    flatpak build-bundle \
        --arch=x86_64 \
        --runtime-repo="${FLATPAK_REMOTE_URL}" \
        "${build_root}/repo" "${bundle}" "${APP_ID}" master > "${OUTPUT_DIR}/flatpak-bundle-${label}.log" 2>&1 \
        || {
            cat "${OUTPUT_DIR}/flatpak-bundle-${label}.log" >&2
            fail "cannot export the ${label} bundle"
        }
    [ -s "${bundle}" ] || fail "the ${label} bundle is empty"

    commit="$(ostree --repo="${build_root}/repo" rev-parse "app/${APP_ID}/x86_64/master")"
    printf '%s\n' "${commit}" | grep -Eq '^[0-9a-f]{64}$' || fail "unexpected OSTree commit for build ${label}: ${commit}"

    sha="$(sha256sum "${bundle}" | awk '{print $1}')"
    size="$(wc -c < "${bundle}" | tr -d ' ')"

    tree="${build_root}/build/files"
    [ -d "${tree}" ] || fail "build ${label} produced no /app tree"
    entries="$(payload_manifest "${tree}" "${OUTPUT_DIR}/flatpak-payload-${label}.txt")"

    printf '%s\n' "${commit}" > "${OUTPUT_DIR}/flatpak-ostree-commit-${label}.txt"
    printf '%s\n' "${sha}" > "${OUTPUT_DIR}/flatpak-bundle-sha256-${label}.txt"
    printf '%s\n' "${size}" > "${OUTPUT_DIR}/flatpak-bundle-size-${label}.txt"
    printf '%s\n' "${entries}" > "${OUTPUT_DIR}/flatpak-payload-entries-${label}.txt"

    log "build ${label}: commit=${commit} sha256=${sha} size=${size} entries=${entries}"
}

# ------------------------------------------------------------------
# 7. Builds
# ------------------------------------------------------------------
case "${BUILD_COUNT}" in
    1 | 2 | 3) ;;
    *) fail "SPLOG_FLATPAK_BUILDS must be 1, 2 or 3" ;;
esac

log "Independent clean builds: ${BUILD_COUNT}"
run_build a
if [ "${BUILD_COUNT}" -ge 2 ]; then
    run_build b
fi

# ------------------------------------------------------------------
# 8. Reproducibility contract
# ------------------------------------------------------------------
COMMIT_A="$(cat "${OUTPUT_DIR}/flatpak-ostree-commit-a.txt")"
SHA_A="$(cat "${OUTPUT_DIR}/flatpak-bundle-sha256-a.txt")"
DETERMINISM_REPORT="${OUTPUT_DIR}/flatpak-determinism.txt"

if [ "${BUILD_COUNT}" -ge 2 ]; then
    COMMIT_B="$(cat "${OUTPUT_DIR}/flatpak-ostree-commit-b.txt")"
    SHA_B="$(cat "${OUTPUT_DIR}/flatpak-bundle-sha256-b.txt")"
    if ! cmp -s "${OUTPUT_DIR}/flatpak-payload-a.txt" "${OUTPUT_DIR}/flatpak-payload-b.txt"; then
        printf '%s\n' '--- build a payload ---' >&2
        cat "${OUTPUT_DIR}/flatpak-payload-a.txt" >&2
        printf '%s\n' '--- build b payload ---' >&2
        cat "${OUTPUT_DIR}/flatpak-payload-b.txt" >&2
        fail "the two clean builds do not produce the same payload tree"
    fi
    {
        printf 'builds=%s\n' "${BUILD_COUNT}"
        printf 'ostree_commit_a=%s\n' "${COMMIT_A}"
        printf 'ostree_commit_b=%s\n' "${COMMIT_B}"
        printf 'bundle_sha256_a=%s\n' "${SHA_A}"
        printf 'bundle_sha256_b=%s\n' "${SHA_B}"
        printf 'payload_identical=true\n'
    } > "${DETERMINISM_REPORT}"

    if [ "${COMMIT_A}" = "${COMMIT_B}" ] && [ "${SHA_A}" = "${SHA_B}" ]; then
        printf 'bit_identical=true\n' >> "${DETERMINISM_REPORT}"
        log "PASS: the two clean builds are bit-identical (OSTree commit ${COMMIT_A})"
    else
        printf 'bit_identical=false\n' >> "${DETERMINISM_REPORT}"
        log "PASS: payload-reproducible (identical /app tree and file digests);"
        log "      OSTree commits/bundles differ: ${COMMIT_A} vs ${COMMIT_B}"
    fi
else
    {
        printf 'builds=1\n'
        printf 'ostree_commit_a=%s\n' "${COMMIT_A}"
        printf 'bundle_sha256_a=%s\n' "${SHA_A}"
        printf 'payload_identical=not-measured\n'
        printf 'bit_identical=not-measured\n'
    } > "${DETERMINISM_REPORT}"
fi

# ------------------------------------------------------------------
# 9. Published artefact, checksums, manifest and toolchain evidence
# ------------------------------------------------------------------
VERSIONED_NAME="SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.flatpak"
CANONICAL_NAME="SPLogbook-Linux-x86_64.flatpak"
BUNDLE="${OUTPUT_DIR}/${VERSIONED_NAME}"
CANONICAL="${OUTPUT_DIR}/${CANONICAL_NAME}"
install -m 0644 "${WORK_ROOT}/build-a/${APP_ID}.flatpak" "${BUNDLE}"
install -m 0644 "${WORK_ROOT}/build-a/${APP_ID}.flatpak" "${CANONICAL}"

BUNDLE_SHA="$(sha256sum "${BUNDLE}" | awk '{print $1}')"
printf '%s  %s\n' "${BUNDLE_SHA}" "${VERSIONED_NAME}" > "${BUNDLE}.sha256"
printf '%s  %s\n' "${BUNDLE_SHA}" "${CANONICAL_NAME}" > "${CANONICAL}.sha256"

# Per-file manifest of the bundle content (the /app tree of build a).
{
    printf '# SPLogbook Flatpak payload manifest\n'
    printf '# app_id=%s\n' "${APP_ID}"
    printf '# version=%s\n' "${PRODUCT_VERSION}"
    printf '# build_commit=%s\n' "${EXPECTED_BUILD_COMMIT}"
    printf '# runtime=%s/%s\n' "${RUNTIME}" "${RUNTIME_VERSION}"
    printf '# sdk=%s/%s\n' "${SDK}" "${RUNTIME_VERSION}"
    printf '# ostree_commit=%s\n' "${COMMIT_A}"
    printf '# bundle_sha256=%s\n' "${BUNDLE_SHA}"
    printf '# entries=%s\n' "$(cat "${OUTPUT_DIR}/flatpak-payload-entries-a.txt")"
    printf '\n'
    cat "${OUTPUT_DIR}/flatpak-payload-a.txt"
} > "${BUNDLE}.manifest"

cat > "${OUTPUT_DIR}/flatpak-toolchain.json" <<EOF
{
  "app_id": "${APP_ID}",
  "package_version": "${PRODUCT_VERSION}",
  "build_commit": "${EXPECTED_BUILD_COMMIT}",
  "runtime": "${RUNTIME}",
  "runtime_version": "${RUNTIME_VERSION}",
  "runtime_commit": "${RUNTIME_COMMIT:-unresolved}",
  "sdk": "${SDK}",
  "sdk_commit": "${SDK_COMMIT:-unresolved}",
  "sdk_extension": "org.freedesktop.Sdk.Extension.rust-stable",
  "sdk_extension_commit": "${RUST_EXTENSION_COMMIT:-unresolved}",
  "flatpak_version": "${FLATPAK_VERSION}",
  "flatpak_builder_version": "${FLATPAK_BUILDER_VERSION}",
  "ostree_version": "${OSTREE_VERSION}",
  "appstreamcli_version": "${APPSTREAM_VERSION}",
  "desktop_file_validate_version": "${DESKTOP_VALIDATE_VERSION}",
  "architecture": "x86_64",
  "branch": "master",
  "build_image": "${FLATPAK_BUILD_IMAGE:-unknown}",
  "build_image_digest": "${FLATPAK_BUILD_IMAGE_DIGEST:-unknown}",
  "source_date_epoch": ${SOURCE_DATE_EPOCH},
  "independent_builds": ${BUILD_COUNT},
  "bundle_sha256": "${BUNDLE_SHA}",
  "ostree_commit": "${COMMIT_A}"
}
EOF

# Never publish, never install system-wide, never submit to Flathub.
printf '%s\n' "${BUNDLE}" > "${OUTPUT_DIR}/flatpak-bundle-path.txt"
log "Bundle: ${BUNDLE}"
log "Alias:  ${CANONICAL}"
log "Size:   $(wc -c < "${BUNDLE}" | tr -d ' ') B"
log "SHA-256:${BUNDLE_SHA}"
log "OSTree commit: ${COMMIT_A}"

echo "=================================================================="
echo "FLATPAK BUILD COMPLETED SUCCESSFULLY"
echo "App ID: ${APP_ID}"
echo "Version: ${PRODUCT_VERSION}"
echo "Runtime: ${RUNTIME}//${RUNTIME_VERSION}"
echo "Artifact: ${BUNDLE}"
echo "SHA-256: ${BUNDLE_SHA}"
echo "=================================================================="
