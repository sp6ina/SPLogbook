#!/usr/bin/env bash
# SPLogbook Linux Portable Launcher
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LD_LIBRARY_PATH="${SCRIPT_DIR}:${SCRIPT_DIR}/hamlib/lib:${SCRIPT_DIR}/lib:${LD_LIBRARY_PATH}"
export PATH="${SCRIPT_DIR}:${SCRIPT_DIR}/hamlib/bin:${PATH}"
cd "${SCRIPT_DIR}"
exec "${SCRIPT_DIR}/splogbook" "$@"
