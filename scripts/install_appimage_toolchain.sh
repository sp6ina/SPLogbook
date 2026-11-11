#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# STAGE-M3-G: resolve, verify and pin the official AppImage toolchain.
#
# Contract (OWNER-M3-G-03-R1 / -04-R1 / -08 / -09):
#   1. resolve the LATEST official continuous build of AppImage/appimagetool;
#   2. verify provenance (official GitHub repository, official download URL);
#   3. record the source commit, release id, publication time, asset name and size;
#   4. read the published SHA-256 digest;
#   5. download the exact asset and compute the local SHA-256;
#   6. REQUIRE both digests to match (fail closed on any mismatch);
#   7. set the executable bit only AFTER verification;
#   8. resolve and verify the latest official type-2 runtime the same way;
#   9. write deterministic audit metadata for the whole gate;
#  10. never use mirrors, never use `curl | sh`, never fall back to appimagetool 1.9.1;
#  11. never commit the downloaded tool or runtime.
#
# Exit codes of the embedded resolver:
#   10 digest unavailable | 11 ambiguous asset | 12 unsupported digest
#   13 provenance mismatch | 14 asset missing

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

APPIMAGETOOL_REPO="AppImage/appimagetool"
APPIMAGETOOL_TAG="continuous"
APPIMAGETOOL_ASSET="appimagetool-x86_64.AppImage"
RUNTIME_REPO="AppImage/type2-runtime"
RUNTIME_TAG="latest"
RUNTIME_ASSET="runtime-x86_64"
GITHUB_API="https://api.github.com"
USER_AGENT="SPLogbook-M3G/1.0"

OUTPUT_DIR="${APPIMAGE_TOOLCHAIN_DIR:-${REPO_ROOT}/dist/appimage-toolchain}"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"

METADATA_JSON="${OUTPUT_DIR}/appimage-toolchain.json"
WORK_DIR="$(mktemp -d "${OUTPUT_DIR}/.resolve_XXXXXX")"
trap 'rm -rf "${WORK_DIR}"' EXIT

log() { printf '%s\n' "$*"; }
blocked() {
    printf 'BLOCKED_%s: %s\n' "$1" "$2" >&2
    exit 1
}

log "=================================================================="
log "SPLogbook AppImage toolchain resolver (STAGE-M3-G)"
log "appimagetool: ${APPIMAGETOOL_REPO}@${APPIMAGETOOL_TAG}"
log "runtime:      ${RUNTIME_REPO}@${RUNTIME_TAG}"
log "output:       ${OUTPUT_DIR}"
log "=================================================================="

if [[ "${EUID}" -eq 0 ]]; then
    log "[WARN] running as root; the toolchain directory must stay outside the repository tree"
fi

# --------------------------------------------------------------------------
# Official metadata fetch (HTTPS only, official API host only).
# --------------------------------------------------------------------------
fetch_release_json() {
    local url="$1" out="$2"
    curl --fail --silent --show-error --location --retry 3 --retry-delay 2 \
        --proto '=https' --tlsv1.2 --max-time 120 \
        -H "Accept: application/vnd.github+json" \
        -H "User-Agent: ${USER_AGENT}" \
        "${url}" -o "${out}"
}

# --------------------------------------------------------------------------
# Validate metadata and emit KEY=VALUE records.
# --------------------------------------------------------------------------
resolve_release() {
    local repo="$1" tag="$2" asset="$3" json="$4" kind="$5" py_out rc
    set +e
    py_out="$(python3 - "${repo}" "${tag}" "${asset}" "${json}" "${kind}" <<'PY'
import json
import sys
from urllib.parse import urlparse

repo, tag, asset_name, path, kind = sys.argv[1:6]


def die(code, message):
    print(f"ERROR[{kind}]: {message}", file=sys.stderr)
    raise SystemExit(code)


with open(path, encoding="utf-8") as handle:
    release = json.load(handle)

if release.get("draft") or release.get("prerelease"):
    die(13, "release is draft or prerelease")
release_tag = release.get("tag_name")
if tag != "latest" and release_tag != tag:
    die(13, f"unexpected release tag {release_tag!r}")
assets = [a for a in release.get("assets", []) if a.get("name") == asset_name]
if len(assets) != 1:
    die(11 if len(assets) > 1 else 14, f"expected exactly one asset {asset_name!r}, found {len(assets)}")
asset = assets[0]
size = asset.get("size")
digest = asset.get("digest")
if not digest:
    die(10, f"asset {asset_name!r} has no published digest")
algorithm, _, hexdigest = digest.partition(":")
if algorithm != "sha256" or len(hexdigest) != 64 or any(c not in "0123456789abcdef" for c in hexdigest):
    die(12, f"unsupported or malformed digest {digest!r}")
url = asset.get("browser_download_url", "")
parsed = urlparse(url)
expected_prefix = f"/{repo}/releases/download/{release_tag}/"
if parsed.scheme != "https" or parsed.netloc != "github.com" or not parsed.path.startswith(expected_prefix):
    die(13, f"untrusted asset download URL {url!r}")
if not isinstance(size, int) or size <= 0:
    die(13, "asset size is missing or invalid")
commit = release.get("target_commitish")
if not isinstance(commit, str) or len(commit) < 7:
    die(13, "release source commit is missing")

print(f"RELEASE_ID={release.get('id')}")
print(f"RELEASE_TAG={release_tag}")
print(f"RELEASE_COMMIT={commit}")
print(f"RELEASE_PUBLISHED={release.get('published_at')}")
print(f"ASSET_NAME={asset_name}")
print(f"ASSET_SIZE={size}")
print(f"ASSET_SHA256={hexdigest}")
print(f"ASSET_URL={url}")
PY
)"
    rc=$?
    set -e
    case "${rc}" in
        0) ;;
        10) blocked "${kind}_DIGEST_UNAVAILABLE" "official release metadata publishes no SHA-256 digest" ;;
        11)
            if [[ "${kind}" == "TOOL" ]]; then
                blocked "AMBIGUOUS_TOOL_ASSET" "more than one asset matches ${asset}"
            else
                blocked "AMBIGUOUS_RUNTIME_ASSET" "more than one asset matches ${asset}"
            fi
            ;;
        12) blocked "UNSUPPORTED_${kind}_DIGEST" "published digest is not a supported SHA-256 value" ;;
        13) blocked "${kind}_PROVENANCE_MISMATCH" "official provenance validation failed" ;;
        14) blocked "${kind}_ASSET_MISSING" "official asset ${asset} is missing" ;;
        *) blocked "${kind}_RESOLUTION_FAILED" "metadata resolver failed with exit code ${rc}" ;;
    esac
    printf '%s\n' "${py_out}"
}

assign_release_fields() {
    local payload="$1"
    while IFS='=' read -r key value; do
        value="${value%$'\r'}"
        case "${key}" in
            RELEASE_ID) RESOLVED_RELEASE_ID="${value}" ;;
            RELEASE_TAG) RESOLVED_TAG="${value}" ;;
            RELEASE_COMMIT) RESOLVED_COMMIT="${value}" ;;
            RELEASE_PUBLISHED) RESOLVED_PUBLISHED="${value}" ;;
            ASSET_NAME) RESOLVED_ASSET_NAME="${value}" ;;
            ASSET_SIZE) RESOLVED_ASSET_SIZE="${value}" ;;
            ASSET_SHA256) RESOLVED_SHA256="${value}" ;;
            ASSET_URL) RESOLVED_URL="${value}" ;;
            '') ;;
            *) blocked "MALFORMED_METADATA" "unexpected resolver record: ${key}" ;;
        esac
    done <<< "${payload}"
    for required in RESOLVED_RELEASE_ID RESOLVED_TAG RESOLVED_COMMIT RESOLVED_PUBLISHED \
        RESOLVED_ASSET_NAME RESOLVED_ASSET_SIZE RESOLVED_SHA256 RESOLVED_URL; do
        if [[ -z "${!required:-}" ]]; then
            blocked "MALFORMED_METADATA" "resolver did not return ${required}"
        fi
    done
}

download_and_verify() {
    local url="$1" destination="$2" expected_size="$3" expected_sha="$4" kind="$5"
    local actual_size actual_sha
    curl --fail --silent --show-error --location --retry 3 --retry-delay 2 \
        --proto '=https' --tlsv1.2 --max-time 600 \
        -H "User-Agent: ${USER_AGENT}" \
        "${url}" -o "${destination}.partial"
    actual_size="$(wc -c < "${destination}.partial" | tr -d ' ')"
    if [[ "${actual_size}" != "${expected_size}" ]]; then
        rm -f "${destination}.partial"
        blocked "${kind}_SIZE_MISMATCH" "downloaded size ${actual_size} != published size ${expected_size}"
    fi
    actual_sha="$(sha256sum "${destination}.partial" | awk '{print $1}')"
    if [[ "${actual_sha}" != "${expected_sha}" ]]; then
        rm -f "${destination}.partial"
        if [[ "${kind}" == "TOOL" ]]; then
            blocked "TOOL_CHECKSUM_MISMATCH" "local SHA-256 ${actual_sha} != published ${expected_sha}"
        else
            blocked "RUNTIME_CHECKSUM_MISMATCH" "local SHA-256 ${actual_sha} != published ${expected_sha}"
        fi
    fi
    # The executable bit is applied only after a successful digest comparison.
    mv "${destination}.partial" "${destination}"
    printf '%s' "${actual_sha}"
}

assert_release_unchanged() {
    local json="$1" expected_id="$2" expected_commit="$3" expected_sha="$4" expected_size="$5" kind="$6"
    local payload
    payload="$(resolve_release "$7" "${8}" "${9}" "${json}" "${kind}")"
    assign_release_fields "${payload}"
    if [[ "${RESOLVED_RELEASE_ID}" != "${expected_id}" ]] \
        || [[ "${RESOLVED_COMMIT}" != "${expected_commit}" ]] \
        || [[ "${RESOLVED_SHA256}" != "${expected_sha}" ]] \
        || [[ "${RESOLVED_ASSET_SIZE}" != "${expected_size}" ]]; then
        blocked "CONTINUOUS_RELEASE_RACE" "the continuous release changed between resolution and download (${kind})"
    fi
}

# --------------------------------------------------------------------------
# 1. appimagetool: resolve once, verify, pin for the whole gate.
# --------------------------------------------------------------------------
TOOL_JSON="${WORK_DIR}/appimagetool-release.json"
TOOL_JSON_RECHECK="${WORK_DIR}/appimagetool-release-recheck.json"

log "--- resolving ${APPIMAGETOOL_REPO}@${APPIMAGETOOL_TAG} ---"
fetch_release_json "${GITHUB_API}/repos/${APPIMAGETOOL_REPO}/releases/tags/${APPIMAGETOOL_TAG}" "${TOOL_JSON}"
TOOL_PAYLOAD="$(resolve_release "${APPIMAGETOOL_REPO}" "${APPIMAGETOOL_TAG}" "${APPIMAGETOOL_ASSET}" "${TOOL_JSON}" "TOOL")"
assign_release_fields "${TOOL_PAYLOAD}"
TOOL_RELEASE_ID="${RESOLVED_RELEASE_ID}"
TOOL_COMMIT="${RESOLVED_COMMIT}"
TOOL_PUBLISHED="${RESOLVED_PUBLISHED}"
TOOL_SIZE="${RESOLVED_ASSET_SIZE}"
TOOL_SHA256="${RESOLVED_SHA256}"
TOOL_URL="${RESOLVED_URL}"

log "APPIMAGETOOL_CHANNEL=${APPIMAGETOOL_TAG}"
log "APPIMAGETOOL_SOURCE_COMMIT=${TOOL_COMMIT}"
log "APPIMAGETOOL_RELEASE_ID=${TOOL_RELEASE_ID}"
log "APPIMAGETOOL_PUBLISHED_AT=${TOOL_PUBLISHED}"
log "APPIMAGETOOL_ASSET=${APPIMAGETOOL_ASSET}"
log "APPIMAGETOOL_SIZE=${TOOL_SIZE}"
log "APPIMAGETOOL_PUBLISHED_SHA256=${TOOL_SHA256}"

if [[ -n "${APPIMAGETOOL_EXPECTED_SHA256:-}" && "${APPIMAGETOOL_EXPECTED_SHA256}" != "${TOOL_SHA256}" ]]; then
    blocked "TOOL_PIN_MISMATCH" "resolved SHA-256 ${TOOL_SHA256} != expected pin ${APPIMAGETOOL_EXPECTED_SHA256}"
fi

TOOL_PATH="${OUTPUT_DIR}/${APPIMAGETOOL_ASSET}"
log "--- downloading and verifying ${APPIMAGETOOL_ASSET} ---"
TOOL_LOCAL_SHA256="$(download_and_verify "${TOOL_URL}" "${TOOL_PATH}" "${TOOL_SIZE}" "${TOOL_SHA256}" "TOOL")"
log "APPIMAGETOOL_LOCAL_SHA256=${TOOL_LOCAL_SHA256}"
if [[ "${TOOL_LOCAL_SHA256}" != "${TOOL_SHA256}" ]]; then
    blocked "TOOL_CHECKSUM_MISMATCH" "digest comparison failed after download"
fi
log "APPIMAGETOOL_DIGEST_MATCH=true"
chmod 0755 "${TOOL_PATH}"

log "--- re-checking ${APPIMAGETOOL_REPO}@${APPIMAGETOOL_TAG} for a continuous release race ---"
fetch_release_json "${GITHUB_API}/repos/${APPIMAGETOOL_REPO}/releases/tags/${APPIMAGETOOL_TAG}" "${TOOL_JSON_RECHECK}"
assert_release_unchanged "${TOOL_JSON_RECHECK}" "${TOOL_RELEASE_ID}" "${TOOL_COMMIT}" "${TOOL_SHA256}" "${TOOL_SIZE}" "TOOL" \
    "${APPIMAGETOOL_REPO}" "${APPIMAGETOOL_TAG}" "${APPIMAGETOOL_ASSET}"

# --------------------------------------------------------------------------
# 2. type-2 runtime: resolve latest official, verify, pin.
# --------------------------------------------------------------------------
RUNTIME_JSON="${WORK_DIR}/runtime-release.json"
RUNTIME_JSON_RECHECK="${WORK_DIR}/runtime-release-recheck.json"

log "--- resolving ${RUNTIME_REPO}@${RUNTIME_TAG} ---"
fetch_release_json "${GITHUB_API}/repos/${RUNTIME_REPO}/releases/${RUNTIME_TAG}" "${RUNTIME_JSON}"
RUNTIME_PAYLOAD="$(resolve_release "${RUNTIME_REPO}" "${RUNTIME_TAG}" "${RUNTIME_ASSET}" "${RUNTIME_JSON}" "RUNTIME")"
assign_release_fields "${RUNTIME_PAYLOAD}"
RUNTIME_RELEASE_ID="${RESOLVED_RELEASE_ID}"
RUNTIME_RELEASE_TAG="${RESOLVED_TAG}"
RUNTIME_COMMIT="${RESOLVED_COMMIT}"
RUNTIME_PUBLISHED="${RESOLVED_PUBLISHED}"
RUNTIME_SIZE="${RESOLVED_ASSET_SIZE}"
RUNTIME_SHA256="${RESOLVED_SHA256}"
RUNTIME_URL="${RESOLVED_URL}"

log "RUNTIME_SOURCE=${RUNTIME_REPO}"
log "RUNTIME_RELEASE_OR_COMMIT=${RUNTIME_RELEASE_TAG}@${RUNTIME_COMMIT}"
log "RUNTIME_RELEASE_ID=${RUNTIME_RELEASE_ID}"
log "RUNTIME_PUBLISHED_AT=${RUNTIME_PUBLISHED}"
log "RUNTIME_ASSET=${RUNTIME_ASSET}"
log "RUNTIME_SIZE=${RUNTIME_SIZE}"
log "RUNTIME_PUBLISHED_SHA256=${RUNTIME_SHA256}"

if [[ -n "${APPIMAGE_RUNTIME_EXPECTED_SHA256:-}" && "${APPIMAGE_RUNTIME_EXPECTED_SHA256}" != "${RUNTIME_SHA256}" ]]; then
    blocked "RUNTIME_PIN_MISMATCH" "resolved SHA-256 ${RUNTIME_SHA256} != expected pin ${APPIMAGE_RUNTIME_EXPECTED_SHA256}"
fi

RUNTIME_PATH="${OUTPUT_DIR}/${RUNTIME_ASSET}"
log "--- downloading and verifying ${RUNTIME_ASSET} ---"
RUNTIME_LOCAL_SHA256="$(download_and_verify "${RUNTIME_URL}" "${RUNTIME_PATH}" "${RUNTIME_SIZE}" "${RUNTIME_SHA256}" "RUNTIME")"
log "RUNTIME_LOCAL_SHA256=${RUNTIME_LOCAL_SHA256}"
if [[ "${RUNTIME_LOCAL_SHA256}" != "${RUNTIME_SHA256}" ]]; then
    blocked "RUNTIME_CHECKSUM_MISMATCH" "digest comparison failed after download"
fi
log "RUNTIME_DIGEST_MATCH=true"
chmod 0644 "${RUNTIME_PATH}"

log "--- re-checking ${RUNTIME_REPO}@${RUNTIME_TAG} for a runtime release race ---"
fetch_release_json "${GITHUB_API}/repos/${RUNTIME_REPO}/releases/${RUNTIME_TAG}" "${RUNTIME_JSON_RECHECK}"
assert_release_unchanged "${RUNTIME_JSON_RECHECK}" "${RUNTIME_RELEASE_ID}" "${RUNTIME_COMMIT}" "${RUNTIME_SHA256}" "${RUNTIME_SIZE}" "RUNTIME" \
    "${RUNTIME_REPO}" "${RUNTIME_TAG}" "${RUNTIME_ASSET}"

# --------------------------------------------------------------------------
# 3. Deterministic audit metadata for the whole gate.
# --------------------------------------------------------------------------
python3 - "${METADATA_JSON}" \
    "${APPIMAGETOOL_REPO}" "${APPIMAGETOOL_TAG}" "${TOOL_RELEASE_ID}" "${TOOL_COMMIT}" \
    "${TOOL_PUBLISHED}" "${APPIMAGETOOL_ASSET}" "${TOOL_SIZE}" "${TOOL_SHA256}" "${TOOL_LOCAL_SHA256}" \
    "${RUNTIME_REPO}" "${RUNTIME_RELEASE_TAG}" "${RUNTIME_COMMIT}" "${RUNTIME_RELEASE_ID}" \
    "${RUNTIME_PUBLISHED}" "${RUNTIME_ASSET}" "${RUNTIME_SIZE}" "${RUNTIME_SHA256}" "${RUNTIME_LOCAL_SHA256}" \
    "${APPIMAGETOOL_ASSET}" "${RUNTIME_ASSET}" <<'PY'
import json
import sys

(
    path,
    tool_repo,
    tool_channel,
    tool_release_id,
    tool_commit,
    tool_published,
    tool_asset,
    tool_size,
    tool_sha,
    tool_local_sha,
    runtime_repo,
    runtime_tag,
    runtime_commit,
    runtime_release_id,
    runtime_published,
    runtime_asset,
    runtime_size,
    runtime_sha,
    runtime_local_sha,
    tool_file,
    runtime_file,
) = sys.argv[1:22]

payload = {
    "schema_version": 1,
    "gate": "STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE",
    "appimagetool": {
        "source": f"https://github.com/{tool_repo}",
        "channel": tool_channel,
        "release_id": int(tool_release_id),
        "source_commit": tool_commit,
        "published_at": tool_published,
        "asset": tool_asset,
        "size": int(tool_size),
        "published_sha256": tool_sha,
        "local_sha256": tool_local_sha,
        "digest_algorithm": "sha256",
        "digest_match": tool_sha == tool_local_sha,
        "downloaded_filename": tool_file,
    },
    "runtime": {
        "source": f"https://github.com/{runtime_repo}",
        "release": runtime_tag,
        "release_id": int(runtime_release_id),
        "source_commit": runtime_commit,
        "published_at": runtime_published,
        "asset": runtime_asset,
        "size": int(runtime_size),
        "published_sha256": runtime_sha,
        "local_sha256": runtime_local_sha,
        "digest_algorithm": "sha256",
        "digest_match": runtime_sha == runtime_local_sha,
        "downloaded_filename": runtime_file,
    },
}
with open(path, "w", encoding="utf-8", newline="\n") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

log "=================================================================="
log "SUCCESS: official AppImage toolchain resolved, verified and pinned"
log "Metadata: ${METADATA_JSON}"
log "APPIMAGETOOL_PINNED_COMMIT=${TOOL_COMMIT}"
log "APPIMAGETOOL_PINNED_SHA256=${TOOL_SHA256}"
log "RUNTIME_PINNED_COMMIT=${RUNTIME_COMMIT}"
log "RUNTIME_PINNED_SHA256=${RUNTIME_SHA256}"
log "=================================================================="
