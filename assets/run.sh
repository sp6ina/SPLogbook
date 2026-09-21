#!/usr/bin/env bash
# SPLogbook Linux Portable Launcher
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LD_LIBRARY_PATH="${SCRIPT_DIR}:${LD_LIBRARY_PATH}"
cd "${SCRIPT_DIR}"
exec "${SCRIPT_DIR}/splogbook" "$@"
