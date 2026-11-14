#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Generate the pinned Cargo source list for the SPLogbook Flatpak (M3-J).
#
# Contract
#   * the only input is the tracked Cargo.lock; no network, no cargo, no
#     registry index and no host cache is consulted;
#   * every [[package]] block that carries a crates.io registry source becomes
#     exactly one `archive` source for static.crates.io, pinned by the SHA-256
#     checksum Cargo itself recorded in Cargo.lock;
#   * the local workspace package (no `source` field) is never emitted: it is
#     built from the tree, not from a crate archive;
#   * the output is a plain JSON array consumed by flatpak-builder
#     (`sources: [ ..., "cargo-sources.json" ]`), one crate per line with sorted
#     keys, so the file is reviewable and diffable;
#   * the generator is deterministic: the same Cargo.lock always produces a
#     byte-identical file. tests/linux_flatpak_packaging_test.rs re-implements
#     this projection in Rust and fails when the committed file diverges.
#
# Usage
#   bash scripts/generate_flatpak_cargo_sources.sh            # rewrite
#   bash scripts/generate_flatpak_cargo_sources.sh --check    # verify only

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LOCK_PATH="${REPO_ROOT}/Cargo.lock"
OUTPUT_PATH="${REPO_ROOT}/installer/linux/flatpak/cargo-sources.json"

REGISTRY_SOURCE="registry+https://github.com/rust-lang/crates.io-index"
REGISTRY_BASE="https://static.crates.io/crates"
VENDOR_PREFIX="cargo/vendor"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

command -v awk >/dev/null 2>&1 || fail "awk is required"
command -v sort >/dev/null 2>&1 || fail "sort is required"

mode="write"
for argument in "$@"; do
    case "${argument}" in
        --check) mode="check" ;;
        -h | --help)
            sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) fail "unknown argument: ${argument}" ;;
    esac
done

[ -f "${LOCK_PATH}" ] || fail "Cargo.lock not found at ${LOCK_PATH}"

project_lock() {
    awk -v registry="${REGISTRY_SOURCE}" -v base="${REGISTRY_BASE}" \
        -v vendor="${VENDOR_PREFIX}" '
        function emit() {
            if (name == "") return
            if (source == "") {
                # Local workspace member: built from the tree, never vendored.
                if (name != "splogbook") {
                    printf "FAIL: local package without source: %s %s\n", name, version > "/dev/stderr"
                    bad = 1
                }
                reset()
                return
            }
            if (source != registry) {
                printf "FAIL: unsupported source for %s %s: %s\n", name, version, source > "/dev/stderr"
                bad = 1
                reset()
                return
            }
            if (name !~ /^[A-Za-z0-9_][A-Za-z0-9_.+-]*$/) {
                printf "FAIL: unsafe crate name: %s\n", name > "/dev/stderr"
                bad = 1
                reset()
                return
            }
            if (version !~ /^[0-9A-Za-z][0-9A-Za-z.+-]*$/) {
                printf "FAIL: unsafe crate version for %s: %s\n", name, version > "/dev/stderr"
                bad = 1
                reset()
                return
            }
            if (length(checksum) != 64 || checksum !~ /^[0-9a-f]+$/) {
                printf "FAIL: missing or malformed checksum for %s %s\n", name, version > "/dev/stderr"
                bad = 1
                reset()
                return
            }
            printf "  {\"archive-type\": \"tar-gzip\", \"dest\": \"%s/%s-%s\", \"sha256\": \"%s\", \"strip-components\": 1, \"type\": \"archive\", \"url\": \"%s/%s/%s-%s.crate\"}\n", vendor, name, version, checksum, base, name, name, version
            count++
            reset()
        }
        function reset() {
            name = ""; version = ""; checksum = ""; source = ""
        }
        BEGIN { block = 0; bad = 0; count = 0 }
        /^\[\[package\]\]$/ { emit(); block = 1; next }
        block && /^\[[^[]/ { emit(); block = 0; next }
        block {
            if ($0 ~ /^name = "/) { value = $0; sub(/^name = "/, "", value); sub(/"$/, "", value); name = value }
            else if ($0 ~ /^version = "/) { value = $0; sub(/^version = "/, "", value); sub(/"$/, "", value); version = value }
            else if ($0 ~ /^checksum = "/) { value = $0; sub(/^checksum = "/, "", value); sub(/"$/, "", value); checksum = value }
            else if ($0 ~ /^source = "/) { value = $0; sub(/^source = "/, "", value); sub(/"$/, "", value); source = value }
        }
        END {
            emit()
            if (bad) exit 1
            if (count == 0) { print "FAIL: no registry package found in Cargo.lock" > "/dev/stderr"; exit 1 }
            printf "%d\n", count > "/dev/stderr"
        }
    ' "${LOCK_PATH}"
}

render() {
    printf '[\n'
    project_lock | LC_ALL=C sort | sed '$!s/$/,/'
    printf ']\n'
}

count="$(project_lock 2>&1 >/dev/null | tail -n 1)"
printf '%s\n' "${count}" | grep -Eq '^[0-9]+$' \
    || fail "cannot project Cargo.lock (${count})"
[ "${count}" -gt 0 ] || fail "projection produced no crate"

if [ "${mode}" = "check" ]; then
    [ -f "${OUTPUT_PATH}" ] || fail "${OUTPUT_PATH} is missing; run without --check"
    rendered="$(mktemp)"
    trap 'rm -f "${rendered}"' EXIT
    render > "${rendered}"
    if ! cmp -s "${OUTPUT_PATH}" "${rendered}"; then
        printf 'FAIL: %s is stale; first difference:\n' "${OUTPUT_PATH#"${REPO_ROOT}/"}" >&2
        diff -u "${OUTPUT_PATH}" "${rendered}" >&2 || printf '(diff reported differences)\n' >&2
        exit 1
    fi
    printf 'PASS: installer/linux/flatpak/cargo-sources.json matches Cargo.lock (%s crates)\n' "${count}"
    exit 0
fi

rendered="$(mktemp)"
trap 'rm -f "${rendered}"' EXIT
render > "${rendered}"
install -m 0644 "${rendered}" "${OUTPUT_PATH}"
printf 'Wrote installer/linux/flatpak/cargo-sources.json: %s crates, %s bytes\n' \
    "${count}" "$(wc -c < "${OUTPUT_PATH}" | tr -d ' ')"
