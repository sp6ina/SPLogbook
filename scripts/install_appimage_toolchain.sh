#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# STAGE-CI-APPIMAGE-TOOLCHAIN-PINNING: download and verify the statically pinned official
# AppImage toolchain (appimagetool + type-2 runtime).
#
# Contract (OWNER-M3-G-03-R1 / -04-R1 / -09 / -10):
#   1. read the pins from installer/linux/appimage/toolchain-pins and validate the file
#      (format tag, known keys only, no duplicates, hex commit/SHA-256, positive size,
#      official URL https://github.com/AppImage/<repo>/releases/download/<channel>/<asset>);
#   2. never query the GitHub REST API: no rate limit, no moving "latest" resolution;
#   3. download each asset directly from its official release URL, HTTPS only, no mirror,
#      no cache;
#   4. on an HTTP or transport error, log the HTTP status code and stop with
#      <KIND>_DOWNLOAD_FAILED (a 404 means the pinned continuous asset is gone: PIN_STALE);
#   5. verify the exact byte size (<KIND>_SIZE_MISMATCH) and SHA-256
#      (<KIND>_CHECKSUM_MISMATCH) inside a private work directory, fail closed; a mismatch
#      usually means upstream replaced the continuous asset (PIN_STALE);
#   6. only after both checks move the file into place, then set its mode
#      (0755 for appimagetool, 0644 for the runtime);
#   7. write appimage-toolchain.json in the format read by build_linux_appimage.sh and the
#      verification jobs;
#   8. never use `curl | sh`, never fall back to appimagetool 1.9.1;
#   9. never commit the downloaded tool or runtime.
#
# Usage
#   bash scripts/install_appimage_toolchain.sh               # download, verify, write metadata
#   bash scripts/install_appimage_toolchain.sh --pins-only   # validate the pins file, no network
#
# Exit codes
#   20 pins file missing or invalid | 21 download failed (HTTP/transport)
#   22 size mismatch | 23 SHA-256 mismatch | 24 required host tool missing

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PINS_FILE="${REPO_ROOT}/installer/linux/appimage/toolchain-pins"
PINS_FORMAT="splogbook-appimage-toolchain-pins/1"
OFFICIAL_URL_PREFIX="https://github.com/AppImage/"
APPIMAGETOOL_REPO="AppImage/appimagetool"
APPIMAGETOOL_ASSET="appimagetool-x86_64.AppImage"
RUNTIME_REPO="AppImage/type2-runtime"
RUNTIME_ASSET="runtime-x86_64"
PINNED_CHANNEL="continuous"
USER_AGENT="SPLogbook-AppImage-Toolchain/2.0"

log() { printf '%s\n' "$*"; }
blocked() {
    # blocked <exit code> <reason> <message>
    printf 'BLOCKED_%s: %s\n' "$2" "$3" >&2
    exit "$1"
}

mode="install"
for argument in "$@"; do
    case "${argument}" in
        --pins-only) mode="pins-only" ;;
        *) blocked 20 "USAGE" "unknown argument: ${argument}" ;;
    esac
done

# --------------------------------------------------------------------------
# 1. Read and validate the static pins.
# --------------------------------------------------------------------------
[[ -f "${PINS_FILE}" ]] || blocked 20 "PINS_MISSING" "missing ${PINS_FILE}"

declare -A PIN=()
line_number=0
while IFS= read -r line || [[ -n "${line}" ]]; do
    line_number=$((line_number + 1))
    line="${line%$'\r'}"
    [[ -z "${line}" || "${line}" == \#* ]] && continue
    if [[ ! "${line}" =~ ^([a-z0-9_.]+)=([^[:space:]]+)$ ]]; then
        blocked 20 "PINS_INVALID" "line ${line_number} is not <key>=<value>"
    fi
    key="${BASH_REMATCH[1]}"
    value="${BASH_REMATCH[2]}"
    case "${key}" in
        format) ;;
        appimagetool.repo | appimagetool.channel | appimagetool.commit | appimagetool.asset) ;;
        appimagetool.size | appimagetool.sha256 | appimagetool.url) ;;
        runtime.repo | runtime.channel | runtime.commit | runtime.asset) ;;
        runtime.size | runtime.sha256 | runtime.url) ;;
        *) blocked 20 "PINS_INVALID" "unknown key ${key} on line ${line_number}" ;;
    esac
    if [[ -n "${PIN[${key}]+set}" ]]; then
        blocked 20 "PINS_INVALID" "duplicate key ${key} on line ${line_number}"
    fi
    PIN["${key}"]="${value}"
done < "${PINS_FILE}"

[[ "${PIN[format]:-}" == "${PINS_FORMAT}" ]] \
    || blocked 20 "PINS_INVALID" "format must be ${PINS_FORMAT}"

validate_pins() {
    # validate_pins <artifact> <expected repo> <expected asset>
    local artifact="$1" expected_repo="$2" expected_asset="$3" field
    for field in repo channel commit asset size sha256 url; do
        [[ -n "${PIN[${artifact}.${field}]:-}" ]] \
            || blocked 20 "PINS_INVALID" "missing ${artifact}.${field}"
    done
    local repo="${PIN[${artifact}.repo]}" channel="${PIN[${artifact}.channel]}"
    local asset="${PIN[${artifact}.asset]}" url="${PIN[${artifact}.url]}"
    [[ "${repo}" == "${expected_repo}" ]] \
        || blocked 20 "PINS_INVALID" "${artifact}.repo must be ${expected_repo}"
    [[ "${channel}" == "${PINNED_CHANNEL}" ]] \
        || blocked 20 "PINS_INVALID" "${artifact}.channel must be ${PINNED_CHANNEL}"
    [[ "${asset}" == "${expected_asset}" ]] \
        || blocked 20 "PINS_INVALID" "${artifact}.asset must be ${expected_asset}"
    [[ "${PIN[${artifact}.commit]}" =~ ^[0-9a-f]{40}$ ]] \
        || blocked 20 "PINS_INVALID" "${artifact}.commit must be a 40-character lowercase hex commit"
    [[ "${PIN[${artifact}.size]}" =~ ^[1-9][0-9]{0,11}$ ]] \
        || blocked 20 "PINS_INVALID" "${artifact}.size must be a positive byte count"
    [[ "${PIN[${artifact}.sha256]}" =~ ^[0-9a-f]{64}$ ]] \
        || blocked 20 "PINS_INVALID" "${artifact}.sha256 must be 64 lowercase hex characters"
    [[ "${url}" == "${OFFICIAL_URL_PREFIX}"* ]] \
        || blocked 20 "PINS_UNTRUSTED_URL" "${artifact}.url must start with ${OFFICIAL_URL_PREFIX}"
    [[ "${url}" == "https://github.com/${repo}/releases/download/${channel}/${asset}" ]] \
        || blocked 20 "PINS_UNTRUSTED_URL" "${artifact}.url is not the official release URL of ${asset}"
}

validate_pins appimagetool "${APPIMAGETOOL_REPO}" "${APPIMAGETOOL_ASSET}"
validate_pins runtime "${RUNTIME_REPO}" "${RUNTIME_ASSET}"

TOOL_COMMIT="${PIN[appimagetool.commit]}"
TOOL_SIZE="${PIN[appimagetool.size]}"
TOOL_SHA256="${PIN[appimagetool.sha256]}"
TOOL_URL="${PIN[appimagetool.url]}"
RUNTIME_COMMIT="${PIN[runtime.commit]}"
RUNTIME_SIZE="${PIN[runtime.size]}"
RUNTIME_SHA256="${PIN[runtime.sha256]}"
RUNTIME_URL="${PIN[runtime.url]}"

log "=================================================================="
log "SPLogbook AppImage toolchain (static pins, no REST API)"
log "pins:         ${PINS_FILE#"${REPO_ROOT}/"}"
log "appimagetool: ${APPIMAGETOOL_REPO}@${PINNED_CHANNEL} ${TOOL_COMMIT}"
log "runtime:      ${RUNTIME_REPO}@${PINNED_CHANNEL} ${RUNTIME_COMMIT}"
log "=================================================================="

if [[ "${mode}" == "pins-only" ]]; then
    log "PINS_VALID=true"
    exit 0
fi

for required in curl sha256sum python3 mktemp; do
    command -v "${required}" >/dev/null 2>&1 \
        || blocked 24 "HOST_TOOL_MISSING" "${required} is required"
done

OUTPUT_DIR="${APPIMAGE_TOOLCHAIN_DIR:-${REPO_ROOT}/dist/appimage-toolchain}"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"
METADATA_JSON="${OUTPUT_DIR}/appimage-toolchain.json"
TOOL_PATH="${OUTPUT_DIR}/${APPIMAGETOOL_ASSET}"
RUNTIME_PATH="${OUTPUT_DIR}/${RUNTIME_ASSET}"

# A leftover from an earlier run is never trusted: only files verified now are kept.
rm -f "${TOOL_PATH}" "${RUNTIME_PATH}" "${METADATA_JSON}"

WORK_DIR="$(mktemp -d "${OUTPUT_DIR}/.download_XXXXXX")"
trap 'rm -rf "${WORK_DIR}"' EXIT

if [[ "${EUID}" -eq 0 ]]; then
    log "[WARN] running as root; the toolchain directory must stay outside the repository tree"
fi

# --------------------------------------------------------------------------
# 2. Direct download into the private work directory, then size + SHA-256 checks.
#    Prints the verified local SHA-256 on stdout; all diagnostics go to stderr.
# --------------------------------------------------------------------------
download_and_verify() {
    # download_and_verify <KIND> <url> <expected size> <expected sha256> <partial path>
    local kind="$1" url="$2" expected_size="$3" expected_sha="$4" partial="$5"
    local status rc actual_size actual_sha
    set +e
    status="$(curl --silent --show-error --location --retry 3 --retry-delay 2 \
        --proto '=https' --proto-redir '=https' --tlsv1.2 --max-time 600 \
        -H "User-Agent: ${USER_AGENT}" \
        --output "${partial}" --write-out '%{http_code}' "${url}")"
    rc=$?
    set -e
    printf '%s_HTTP_STATUS=%s\n' "${kind}" "${status:-none}" >&2
    if [[ "${rc}" -ne 0 || "${status}" != "200" ]]; then
        rm -f "${partial}"
        if [[ "${status}" == "404" || "${status}" == "410" ]]; then
            blocked 21 "${kind}_DOWNLOAD_FAILED" \
                "HTTP ${status} for ${url}; the pinned continuous asset is gone upstream (PIN_STALE): a new pin needs an owner decision"
        fi
        blocked 21 "${kind}_DOWNLOAD_FAILED" "HTTP ${status:-none}, curl exit ${rc} for ${url}"
    fi
    actual_size="$(wc -c < "${partial}" | tr -d ' ')"
    if [[ "${actual_size}" != "${expected_size}" ]]; then
        rm -f "${partial}"
        blocked 22 "${kind}_SIZE_MISMATCH" \
            "downloaded ${actual_size} bytes, pinned ${expected_size}; upstream may have replaced the continuous asset (PIN_STALE)"
    fi
    actual_sha="$(sha256sum "${partial}" | awk '{print $1}')"
    if [[ "${actual_sha}" != "${expected_sha}" ]]; then
        rm -f "${partial}"
        blocked 23 "${kind}_CHECKSUM_MISMATCH" \
            "local SHA-256 ${actual_sha} != pinned ${expected_sha}; upstream may have replaced the continuous asset (PIN_STALE)"
    fi
    printf '%s' "${actual_sha}"
}

# --------------------------------------------------------------------------
# 3. appimagetool: verify, then move into place, then make it executable.
# --------------------------------------------------------------------------
log "--- downloading pinned ${APPIMAGETOOL_ASSET} ---"
log "APPIMAGETOOL_URL=${TOOL_URL}"
TOOL_PARTIAL="${WORK_DIR}/${APPIMAGETOOL_ASSET}.partial"
TOOL_LOCAL_SHA256="$(download_and_verify TOOL "${TOOL_URL}" "${TOOL_SIZE}" "${TOOL_SHA256}" "${TOOL_PARTIAL}")"
[[ "${TOOL_LOCAL_SHA256}" == "${TOOL_SHA256}" ]] \
    || blocked 23 "TOOL_CHECKSUM_MISMATCH" "digest comparison failed after download"
mv -f "${TOOL_PARTIAL}" "${TOOL_PATH}"
chmod 0755 "${TOOL_PATH}"
log "APPIMAGETOOL_SIZE=${TOOL_SIZE}"
log "APPIMAGETOOL_LOCAL_SHA256=${TOOL_LOCAL_SHA256}"
log "APPIMAGETOOL_DIGEST_MATCH=true"

# --------------------------------------------------------------------------
# 4. type-2 runtime: verify, then move into place (never executable).
# --------------------------------------------------------------------------
log "--- downloading pinned ${RUNTIME_ASSET} ---"
log "RUNTIME_URL=${RUNTIME_URL}"
RUNTIME_PARTIAL="${WORK_DIR}/${RUNTIME_ASSET}.partial"
RUNTIME_LOCAL_SHA256="$(download_and_verify RUNTIME "${RUNTIME_URL}" "${RUNTIME_SIZE}" "${RUNTIME_SHA256}" "${RUNTIME_PARTIAL}")"
[[ "${RUNTIME_LOCAL_SHA256}" == "${RUNTIME_SHA256}" ]] \
    || blocked 23 "RUNTIME_CHECKSUM_MISMATCH" "digest comparison failed after download"
mv -f "${RUNTIME_PARTIAL}" "${RUNTIME_PATH}"
chmod 0644 "${RUNTIME_PATH}"
log "RUNTIME_SIZE=${RUNTIME_SIZE}"
log "RUNTIME_LOCAL_SHA256=${RUNTIME_LOCAL_SHA256}"
log "RUNTIME_DIGEST_MATCH=true"

# --------------------------------------------------------------------------
# 5. Toolchain metadata (keys read by build_linux_appimage.sh and the verify jobs).
# --------------------------------------------------------------------------
python3 - "${METADATA_JSON}" \
    "${APPIMAGETOOL_REPO}" "${PINNED_CHANNEL}" "${TOOL_COMMIT}" "${APPIMAGETOOL_ASSET}" \
    "${TOOL_SIZE}" "${TOOL_SHA256}" "${TOOL_LOCAL_SHA256}" "${TOOL_URL}" \
    "${RUNTIME_REPO}" "${PINNED_CHANNEL}" "${RUNTIME_COMMIT}" "${RUNTIME_ASSET}" \
    "${RUNTIME_SIZE}" "${RUNTIME_SHA256}" "${RUNTIME_LOCAL_SHA256}" "${RUNTIME_URL}" <<'PY'
import json
import sys

(
    path,
    tool_repo,
    tool_channel,
    tool_commit,
    tool_asset,
    tool_size,
    tool_sha,
    tool_local_sha,
    tool_url,
    runtime_repo,
    runtime_channel,
    runtime_commit,
    runtime_asset,
    runtime_size,
    runtime_sha,
    runtime_local_sha,
    runtime_url,
) = sys.argv[1:18]

payload = {
    "schema_version": 2,
    "gate": "STAGE-CI-APPIMAGE-TOOLCHAIN-PINNING",
    "pins": "installer/linux/appimage/toolchain-pins",
    "appimagetool": {
        "source": f"https://github.com/{tool_repo}",
        "channel": tool_channel,
        "source_commit": tool_commit,
        "asset": tool_asset,
        "size": int(tool_size),
        "url": tool_url,
        "published_sha256": tool_sha,
        "local_sha256": tool_local_sha,
        "digest_algorithm": "sha256",
        "digest_match": tool_sha == tool_local_sha,
        "downloaded_filename": tool_asset,
    },
    "runtime": {
        "source": f"https://github.com/{runtime_repo}",
        "release": runtime_channel,
        "source_commit": runtime_commit,
        "asset": runtime_asset,
        "size": int(runtime_size),
        "url": runtime_url,
        "published_sha256": runtime_sha,
        "local_sha256": runtime_local_sha,
        "digest_algorithm": "sha256",
        "digest_match": runtime_sha == runtime_local_sha,
        "downloaded_filename": runtime_asset,
    },
}
with open(path, "w", encoding="utf-8", newline="\n") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

log "=================================================================="
log "SUCCESS: pinned AppImage toolchain downloaded and verified"
log "Metadata: ${METADATA_JSON}"
log "APPIMAGETOOL_PINNED_COMMIT=${TOOL_COMMIT}"
log "APPIMAGETOOL_PINNED_SHA256=${TOOL_SHA256}"
log "RUNTIME_PINNED_COMMIT=${RUNTIME_COMMIT}"
log "RUNTIME_PINNED_SHA256=${RUNTIME_SHA256}"
log "=================================================================="
