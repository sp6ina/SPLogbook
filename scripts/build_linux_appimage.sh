#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# STAGE-M3-G: build the canonical SPLogbook Linux x86_64 AppImage from the
# already verified portable tar.gz of the SAME commit.
#
# Contract:
#   * input is the verified `SPLogbook-Linux-x86_64-<version>.tar.gz` plus its
#     `.sha256` sidecar; its SHA-256 is re-checked before anything is unpacked;
#   * the archive is unpacked with an explicit safe validator (no absolute
#     paths, no `..`, no symlinks/hardlinks/devices/FIFOs, size limits);
#   * build_commit is confirmed against the portable marker and, when set,
#     against EXPECTED_BUILD_COMMIT (the CI commit under test);
#   * the AppDir follows the approved M3-G layout (no symlinks, no AppStream,
#     only libhamlib.so.4 bundled);
#   * appimagetool is the pinned, verified continuous build and receives the
#     pinned type-2 runtime explicitly through --runtime-file;
#   * no update information, no signing, no publication, no tag;
#   * a deterministic `.sha256` sidecar is produced next to the AppImage.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

PRODUCT_VERSION_EXPECTED="1.3.0-alpha.1"
APPIMAGE_FILENAME="SPLogbook-Linux-x86_64.AppImage"
GLIBC_POLICY_MAX="2.43"

usage() {
    cat <<'EOF'
Usage: scripts/build_linux_appimage.sh <portable.tar.gz> [output_dir]

Environment:
  APPIMAGE_TOOLCHAIN_DIR   directory with the pinned appimagetool + runtime
                           (default: <repo>/dist/appimage-toolchain)
  EXPECTED_BUILD_COMMIT    required commit under test (CI); when set, the
                           portable marker must match it
EOF
}

PORTABLE_TAR="${1:-}"
OUTPUT_DIR="${2:-${REPO_ROOT}/dist}"
if [[ -z "${PORTABLE_TAR}" ]]; then
    usage >&2
    exit 2
fi
if [[ ! -f "${PORTABLE_TAR}" ]]; then
    printf 'BLOCKED_PORTABLE_ARTIFACT_MISSING: %s\n' "${PORTABLE_TAR}" >&2
    exit 1
fi
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"
PORTABLE_TAR="$(cd "$(dirname "${PORTABLE_TAR}")" && pwd)/$(basename "${PORTABLE_TAR}")"
TOOLCHAIN_DIR="${APPIMAGE_TOOLCHAIN_DIR:-${REPO_ROOT}/dist/appimage-toolchain}"

log() { printf '%s\n' "$*"; }
fail() {
    printf 'BLOCKED_%s: %s\n' "$1" "$2" >&2
    exit 1
}

log "=================================================================="
log "SPLogbook Linux AppImage Builder (STAGE-M3-G)"
log "Portable input: ${PORTABLE_TAR}"
log "Output:         ${OUTPUT_DIR}"
log "Toolchain:      ${TOOLCHAIN_DIR}"
log "=================================================================="

for required in \
    "${TOOLCHAIN_DIR}/appimagetool-x86_64.AppImage" \
    "${TOOLCHAIN_DIR}/runtime-x86_64" \
    "${TOOLCHAIN_DIR}/appimage-toolchain.json"; do
    [[ -f "${required}" ]] || fail "TOOLCHAIN_MISSING" "missing ${required} (run scripts/install_appimage_toolchain.sh)"
done

# --------------------------------------------------------------------------
# 0. Verify the toolchain pins again (a cache hit must never be trusted).
# --------------------------------------------------------------------------
TOOLCHAIN_FIELDS="$(python3 - "${TOOLCHAIN_DIR}/appimage-toolchain.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
tool = data["appimagetool"]
runtime = data["runtime"]
if tool.get("digest_match") is not True or runtime.get("digest_match") is not True:
    raise SystemExit("toolchain metadata reports a digest mismatch")
print(f"TOOL_SHA256={tool['published_sha256']}")
print(f"TOOL_COMMIT={tool['source_commit']}")
print(f"TOOL_CHANNEL={tool['channel']}")
print(f"RUNTIME_SHA256={runtime['published_sha256']}")
print(f"RUNTIME_COMMIT={runtime['source_commit']}")
print(f"RUNTIME_RELEASE={runtime['release']}")
PY
)"
while IFS='=' read -r key value; do
    value="${value%$'\r'}"
    case "${key}" in
        TOOL_SHA256) EXPECTED_TOOL_SHA256="${value}" ;;
        TOOL_COMMIT) EXPECTED_TOOL_COMMIT="${value}" ;;
        TOOL_CHANNEL) TOOL_CHANNEL="${value}" ;;
        RUNTIME_SHA256) EXPECTED_RUNTIME_SHA256="${value}" ;;
        RUNTIME_COMMIT) RUNTIME_COMMIT="${value}" ;;
        RUNTIME_RELEASE) RUNTIME_RELEASE="${value}" ;;
        '') ;;
        *) fail "TOOLCHAIN_METADATA_INVALID" "unexpected metadata key ${key}" ;;
    esac
done <<< "${TOOLCHAIN_FIELDS}"

ACTUAL_TOOL_SHA256="$(sha256sum "${TOOLCHAIN_DIR}/appimagetool-x86_64.AppImage" | awk '{print $1}')"
[[ "${ACTUAL_TOOL_SHA256}" == "${EXPECTED_TOOL_SHA256}" ]] \
    || fail "TOOL_CHECKSUM_MISMATCH" "appimagetool on disk does not match the pinned SHA-256"
ACTUAL_RUNTIME_SHA256="$(sha256sum "${TOOLCHAIN_DIR}/runtime-x86_64" | awk '{print $1}')"
[[ "${ACTUAL_RUNTIME_SHA256}" == "${EXPECTED_RUNTIME_SHA256}" ]] \
    || fail "RUNTIME_CHECKSUM_MISMATCH" "type-2 runtime on disk does not match the pinned SHA-256"
[[ "${TOOL_CHANNEL}" == "continuous" ]] \
    || fail "TOOL_CHANNEL_INVALID" "appimagetool channel must be continuous"
log "[OK] appimagetool ${EXPECTED_TOOL_COMMIT} (${EXPECTED_TOOL_SHA256})"
log "[OK] type-2 runtime ${RUNTIME_RELEASE}@${RUNTIME_COMMIT} (${EXPECTED_RUNTIME_SHA256})"

# --------------------------------------------------------------------------
# 1. Verify the portable tar.gz SHA-256 from its sidecar.
# --------------------------------------------------------------------------
[[ -f "${PORTABLE_TAR}.sha256" ]] \
    || fail "PORTABLE_CHECKSUM_MISSING" "missing ${PORTABLE_TAR}.sha256"
(
    cd "$(dirname "${PORTABLE_TAR}")"
    sha256sum -c "$(basename "${PORTABLE_TAR}").sha256"
) || fail "PORTABLE_CHECKSUM_MISMATCH" "portable tar.gz SHA-256 verification failed"
PORTABLE_SHA256="$(sha256sum "${PORTABLE_TAR}" | awk '{print $1}')"
log "[OK] portable SHA-256: ${PORTABLE_SHA256}"

# --------------------------------------------------------------------------
# 2. Safe extraction into an isolated staging directory.
# --------------------------------------------------------------------------
STAGE_ROOT="$(mktemp -d "${OUTPUT_DIR}/.staging_appimage_XXXXXX")"
trap 'rm -rf "${STAGE_ROOT}"' EXIT

python3 - "${PORTABLE_TAR}" "${STAGE_ROOT}" <<'PY'
import sys
import tarfile

archive, destination = sys.argv[1:3]
MAX_ENTRIES = 10000
MAX_TOTAL_BYTES = 250 * 1024 * 1024
MAX_ENTRY_BYTES = 150 * 1024 * 1024

with tarfile.open(archive, "r:gz") as handle:
    members = handle.getmembers()
    if len(members) > MAX_ENTRIES:
        raise SystemExit(f"BLOCKED_ARCHIVE_TOO_MANY_ENTRIES: {len(members)}")
    total = 0
    for member in members:
        name = member.name
        if name.startswith("/") or "\\" in name:
            raise SystemExit(f"BLOCKED_ARCHIVE_UNSAFE_PATH: {name}")
        parts = [part for part in name.split("/") if part not in ("", ".")]
        if ".." in parts:
            raise SystemExit(f"BLOCKED_ARCHIVE_UNSAFE_PATH: {name}")
        if member.issym() or member.islnk():
            raise SystemExit(f"BLOCKED_ARCHIVE_LINK_FORBIDDEN: {name}")
        if member.isdev() or member.isfifo():
            raise SystemExit(f"BLOCKED_ARCHIVE_SPECIAL_FILE: {name}")
        if not (member.isfile() or member.isdir()):
            raise SystemExit(f"BLOCKED_ARCHIVE_UNSUPPORTED_ENTRY: {name}")
        if parts[:1] != ["SPLogbook"]:
            raise SystemExit(f"BLOCKED_ARCHIVE_UNEXPECTED_ROOT: {name}")
        if member.size > MAX_ENTRY_BYTES:
            raise SystemExit(f"BLOCKED_ARCHIVE_ENTRY_TOO_LARGE: {name}")
        total += member.size
        if total > MAX_TOTAL_BYTES:
            raise SystemExit("BLOCKED_ARCHIVE_TOO_LARGE")
    try:
        handle.extractall(destination, filter="data")
    except TypeError:
        handle.extractall(destination)
PY

PAYLOAD="${STAGE_ROOT}/SPLogbook"
[[ -d "${PAYLOAD}" ]] || fail "PAYLOAD_INVALID" "single SPLogbook/ root directory is missing"
for required in SPLogbook splogbook-updater .splogbook-linux-portable-marker lib/libhamlib.so.4; do
    [[ -e "${PAYLOAD}/${required}" ]] || fail "PAYLOAD_INCOMPLETE" "payload is missing ${required}"
done

PORTABLE_MARKER="${PAYLOAD}/.splogbook-linux-portable-marker"
PAYLOAD_VERSION="$(sed -n 's/.*"package_version": *"\([^"]*\)".*/\1/p' "${PORTABLE_MARKER}" | head -n 1)"
PAYLOAD_COMMIT="$(sed -n 's/.*"build_commit": *"\([0-9a-f]\{40\}\)".*/\1/p' "${PORTABLE_MARKER}" | head -n 1)"
[[ -n "${PAYLOAD_VERSION}" ]] || fail "PAYLOAD_MARKER_INVALID" "portable marker package_version is missing"
[[ -n "${PAYLOAD_COMMIT}" ]] || fail "PAYLOAD_MARKER_INVALID" "portable marker build_commit is missing"
[[ "${PAYLOAD_VERSION}" == "${PRODUCT_VERSION_EXPECTED}" ]] \
    || fail "VERSION_MISMATCH" "payload version ${PAYLOAD_VERSION} != ${PRODUCT_VERSION_EXPECTED}"
if [[ -n "${EXPECTED_BUILD_COMMIT:-}" && "${PAYLOAD_COMMIT}" != "${EXPECTED_BUILD_COMMIT}" ]]; then
    fail "BUILD_COMMIT_MISMATCH" "payload build_commit ${PAYLOAD_COMMIT} != CI commit ${EXPECTED_BUILD_COMMIT}"
fi
log "[OK] payload version ${PAYLOAD_VERSION}, build commit ${PAYLOAD_COMMIT}"

# --------------------------------------------------------------------------
# 3. Assemble the approved AppDir layout.
# --------------------------------------------------------------------------
APPDIR="${STAGE_ROOT}/AppDir"
mkdir -p \
    "${APPDIR}/usr/bin" \
    "${APPDIR}/usr/lib" \
    "${APPDIR}/usr/share/applications" \
    "${APPDIR}/usr/share/icons/hicolor/256x256/apps" \
    "${APPDIR}/usr/share/doc/splogbook/THIRD_PARTY_LICENSES" \
    "${APPDIR}/usr/share/splogbook"

CANONICAL_DESKTOP="${REPO_ROOT}/installer/linux/portable/templates/splogbook.desktop"
CANONICAL_ICON="${REPO_ROOT}/assets/icon.png"
CANONICAL_APPRUN="${REPO_ROOT}/installer/linux/appimage/AppRun"
for required in "${CANONICAL_DESKTOP}" "${CANONICAL_ICON}" "${CANONICAL_APPRUN}"; do
    [[ -f "${required}" ]] || fail "CANONICAL_SOURCE_MISSING" "missing ${required}"
done

install -m 0755 "${CANONICAL_APPRUN}" "${APPDIR}/AppRun"
install -m 0755 "${PAYLOAD}/SPLogbook" "${APPDIR}/usr/bin/SPLogbook"
install -m 0755 "${PAYLOAD}/splogbook-updater" "${APPDIR}/usr/bin/splogbook-updater"
install -m 0755 "${PAYLOAD}/lib/libhamlib.so.4" "${APPDIR}/usr/lib/libhamlib.so.4"

install -m 0644 "${CANONICAL_DESKTOP}" "${APPDIR}/splogbook.desktop"
install -m 0644 "${CANONICAL_DESKTOP}" "${APPDIR}/usr/share/applications/splogbook.desktop"
install -m 0644 "${CANONICAL_ICON}" "${APPDIR}/splogbook.png"
install -m 0644 "${CANONICAL_ICON}" "${APPDIR}/.DirIcon"
install -m 0644 "${CANONICAL_ICON}" "${APPDIR}/usr/share/icons/hicolor/256x256/apps/splogbook.png"

if [[ -f "${REPO_ROOT}/LICENSE" ]]; then
    install -m 0644 "${REPO_ROOT}/LICENSE" "${APPDIR}/usr/share/doc/splogbook/LICENSE"
fi
if [[ -d "${PAYLOAD}/THIRD_PARTY_LICENSES" ]]; then
    cp -R "${PAYLOAD}/THIRD_PARTY_LICENSES/." "${APPDIR}/usr/share/doc/splogbook/THIRD_PARTY_LICENSES/"
fi
if [[ -f "${PAYLOAD}/README.txt" ]]; then
    install -m 0644 "${PAYLOAD}/README.txt" "${APPDIR}/usr/share/doc/splogbook/README.txt"
fi

# RUNPATH for binaries in usr/bin is exactly $ORIGIN/../lib.
if command -v patchelf >/dev/null 2>&1; then
    patchelf --set-rpath '$ORIGIN/../lib' "${APPDIR}/usr/bin/SPLogbook"
    patchelf --set-rpath '$ORIGIN/../lib' "${APPDIR}/usr/bin/splogbook-updater"
    patchelf --set-rpath '$ORIGIN' "${APPDIR}/usr/lib/libhamlib.so.4" 2>/dev/null || true
else
    fail "PATCHELF_MISSING" "patchelf is required to enforce RUNPATH=\$ORIGIN/../lib"
fi

# --------------------------------------------------------------------------
# 4. Deterministic AppImage marker.
# --------------------------------------------------------------------------
RUSTC_VERSION="$(rustc --version)"
MARKER="${APPDIR}/usr/share/splogbook/.splogbook-appimage-marker"
cat > "${MARKER}" <<EOF
{
  "marker_version": "1.0",
  "installation_type": "LinuxAppImage",
  "management_mode": "SelfManaged",
  "package_version": "${PAYLOAD_VERSION}",
  "installation_root": ".",
  "build_commit": "${PAYLOAD_COMMIT}",
  "target_os": "linux",
  "target_arch": "x86_64",
  "architecture": "x86_64",
  "libc_family": "glibc",
  "artifact_format": "AppImage",
  "resolved_minimum_kernel": "7.0",
  "maximum_glibc": "${GLIBC_POLICY_MAX}",
  "appimagetool_channel": "${TOOL_CHANNEL}",
  "appimagetool_source_commit": "${EXPECTED_TOOL_COMMIT}",
  "appimagetool_sha256": "${EXPECTED_TOOL_SHA256}",
  "runtime_release_or_commit": "${RUNTIME_COMMIT}",
  "runtime_sha256": "${EXPECTED_RUNTIME_SHA256}",
  "build_rust_version": "${RUSTC_VERSION}",
  "created_by": "scripts/build_linux_appimage.sh"
}
EOF
chmod 0644 "${MARKER}"

# --------------------------------------------------------------------------
# 5. Structure and safety validation of the AppDir (fail closed).
# --------------------------------------------------------------------------
for required in \
    AppRun splogbook.desktop splogbook.png .DirIcon \
    usr/bin/SPLogbook usr/bin/splogbook-updater usr/lib/libhamlib.so.4 \
    usr/share/applications/splogbook.desktop \
    usr/share/icons/hicolor/256x256/apps/splogbook.png \
    usr/share/doc/splogbook/LICENSE usr/share/doc/splogbook/README.txt \
    usr/share/splogbook/.splogbook-appimage-marker; do
    [[ -e "${APPDIR}/${required}" ]] || fail "APPDIR_INCOMPLETE" "missing ${required}"
done
[[ -f "${APPDIR}/.DirIcon" && ! -L "${APPDIR}/.DirIcon" ]] \
    || fail "APPDIR_DIRICON_INVALID" ".DirIcon must be a regular file"

if [[ -n "$(find "${APPDIR}" -type l -print -quit)" ]]; then
    fail "APPDIR_SYMLINK_FORBIDDEN" "the AppDir must not contain symlinks"
fi
if [[ -n "$(find "${APPDIR}" -perm /6000 -print -quit)" ]]; then
    fail "APPDIR_SETUID_FORBIDDEN" "the AppDir must not contain setuid/setgid entries"
fi
if [[ -n "$(find "${APPDIR}" -type f -perm -0002 -print -quit)" ]]; then
    fail "APPDIR_WORLD_WRITABLE" "the AppDir must not contain world-writable files"
fi
EXTRA_LIBS="$(find "${APPDIR}/usr/lib" -maxdepth 1 -type f ! -name 'libhamlib.so.4' -print)"
[[ -z "${EXTRA_LIBS}" ]] || fail "APPDIR_EXTRA_LIBRARY" "only libhamlib.so.4 may be bundled: ${EXTRA_LIBS}"
cmp -s "${APPDIR}/splogbook.desktop" "${CANONICAL_DESKTOP}" \
    || fail "APPDIR_DESKTOP_MISMATCH" "root desktop file differs from the canonical template"
cmp -s "${APPDIR}/usr/share/applications/splogbook.desktop" "${CANONICAL_DESKTOP}" \
    || fail "APPDIR_DESKTOP_MISMATCH" "applications desktop file differs from the canonical template"
cmp -s "${APPDIR}/splogbook.png" "${CANONICAL_ICON}" \
    || fail "APPDIR_ICON_MISMATCH" "root icon differs from assets/icon.png"
cmp -s "${APPDIR}/.DirIcon" "${CANONICAL_ICON}" \
    || fail "APPDIR_ICON_MISMATCH" ".DirIcon differs from assets/icon.png"
cmp -s "${APPDIR}/usr/share/icons/hicolor/256x256/apps/splogbook.png" "${CANONICAL_ICON}" \
    || fail "APPDIR_ICON_MISMATCH" "hicolor icon differs from assets/icon.png"
for key in package_version build_commit installation_type resolved_minimum_kernel \
    maximum_glibc artifact_format architecture appimagetool_channel \
    appimagetool_source_commit appimagetool_sha256 runtime_release_or_commit runtime_sha256; do
    grep -q "\"${key}\":" "${MARKER}" || fail "APPDIR_MARKER_INCOMPLETE" "marker is missing ${key}"
done
if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "${APPDIR}/splogbook.desktop" \
        || fail "APPDIR_DESKTOP_INVALID" "desktop-file-validate rejected the canonical desktop file"
fi
log "[OK] AppDir structure, permissions, marker and desktop contract verified"

# --------------------------------------------------------------------------
# 6. Build the AppImage with the pinned continuous appimagetool.
# --------------------------------------------------------------------------
APPIMAGE_PATH="${OUTPUT_DIR}/${APPIMAGE_FILENAME}"
rm -f "${APPIMAGE_PATH}" "${APPIMAGE_PATH}.sha256"

log "--- running pinned appimagetool ${EXPECTED_TOOL_COMMIT} ---"
# APPIMAGE_EXTRACT_AND_RUN=1 avoids FUSE for the build tool itself; the verified
# file is still the only executable used. --no-appstream is required because
# AppStream metadata is not approved by DESIGN for M3-G.
ARCH=x86_64 \
APPIMAGE_EXTRACT_AND_RUN=1 \
    "${TOOLCHAIN_DIR}/appimagetool-x86_64.AppImage" \
    --no-appstream \
    --runtime-file "${TOOLCHAIN_DIR}/runtime-x86_64" \
    "${APPDIR}" \
    "${APPIMAGE_PATH}"

[[ -s "${APPIMAGE_PATH}" ]] || fail "APPIMAGE_BUILD_FAILED" "appimagetool produced no output"
chmod 0755 "${APPIMAGE_PATH}"

APPIMAGE_MAGIC="$(od -An -tx1 -j8 -N3 "${APPIMAGE_PATH}" | tr -d ' \n')"
[[ "${APPIMAGE_MAGIC}" == "414902" ]] \
    || fail "APPIMAGE_TYPE_INVALID" "output is not an AppImage type 2 file (magic ${APPIMAGE_MAGIC})"

APPIMAGE_SIZE="$(wc -c < "${APPIMAGE_PATH}" | tr -d ' ')"
APPIMAGE_SHA256="$(sha256sum "${APPIMAGE_PATH}" | awk '{print $1}')"
printf '%s  %s\n' "${APPIMAGE_SHA256}" "${APPIMAGE_FILENAME}" > "${APPIMAGE_PATH}.sha256"

log "=================================================================="
log "SUCCESS: AppImage built with the pinned, verified toolchain"
log "AppImage:      ${APPIMAGE_PATH}"
log "Size:          ${APPIMAGE_SIZE} B"
log "SHA-256:       ${APPIMAGE_SHA256}"
log "build_commit:  ${PAYLOAD_COMMIT}"
log "appimagetool:  ${EXPECTED_TOOL_COMMIT} (${TOOL_CHANNEL})"
log "runtime:       ${RUNTIME_RELEASE}@${RUNTIME_COMMIT}"
log "Published:     NO (CI artifact only; no update information, no signing)"
log "=================================================================="
