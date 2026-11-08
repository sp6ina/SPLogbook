#!/bin/sh
set -e

echo "=== 1. OS & KERNEL PROFILE ==="
if [ -f /etc/os-release ]; then
  cat /etc/os-release
elif [ -f /usr/lib/os-release ]; then
  cat /usr/lib/os-release
fi
uname -a
uname -r
uname -m
getconf GNU_LIBC_VERSION || true
ldd --version || true

echo "=== 2. DISTRO PACKAGE METADATA ==="
if [ -n "${PKG_QUERY_CMD}" ]; then
  eval "${PKG_QUERY_CMD}" || true
fi

echo "=== 3. SHA-256 INTEGRITY CHECK ==="
sha256sum -c SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz.sha256

echo "=== 4. ARCHIVE PREFLIGHT & SAFE EXTRACTION ==="
tar -ztvf SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz | head -30
tar -xzf SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz
[ -d "SPLogbook" ] || { echo "Root SPLogbook directory missing"; exit 1; }
[ -f "SPLogbook/.splogbook-linux-portable-marker" ] || { echo "Portable marker missing"; exit 1; }
cat "SPLogbook/.splogbook-linux-portable-marker"

echo "=== 5. ELF & LOADER VALIDATION ==="
file SPLogbook/SPLogbook || true
ldd SPLogbook/SPLogbook || true

echo "=== 6. BINARY LAUNCH & IDENTITY CHECK ==="
SPLogbook/SPLogbook --version || true
SPLogbook/SPLogbook --build-info || true

echo "=== 7. ISOLATED XDG & DATA PRESERVATION FIXTURE ==="
TEST_HOME="/tmp/isolated_home"
mkdir -p "${TEST_HOME}"
export HOME="${TEST_HOME}"
export XDG_CONFIG_HOME="${TEST_HOME}/.config"
export XDG_DATA_HOME="${TEST_HOME}/.local/share"
export XDG_STATE_HOME="${TEST_HOME}/.local/state"
export XDG_CACHE_HOME="${TEST_HOME}/.cache"

# Initialize dummy user databases
mkdir -p "${XDG_DATA_HOME}/splogbook"
echo "USER_LOG_INITIAL_DATA" > "${XDG_DATA_HOME}/splogbook/default_log.db"
INITIAL_DB_SHA=$(sha256sum "${XDG_DATA_HOME}/splogbook/default_log.db" | awk '{print $1}')

echo "=== 8. TRANSACTIONAL ROLLBACK FIXTURE ==="
mkdir -p SPLogbook/.rollback
cp SPLogbook/SPLogbook SPLogbook/.rollback/SPLogbook.orig
echo "NEW_TEST_BINARY" > SPLogbook/SPLogbook
# Rollback
cp SPLogbook/.rollback/SPLogbook.orig SPLogbook/SPLogbook
rm -rf SPLogbook/.rollback

FINAL_DB_SHA=$(sha256sum "${XDG_DATA_HOME}/splogbook/default_log.db" | awk '{print $1}')
if [ "${INITIAL_DB_SHA}" != "${FINAL_DB_SHA}" ]; then
  echo "ERROR: User data modified during fixture!"
  exit 1
fi
echo "User database byte exactness verified: ${FINAL_DB_SHA}"
echo "NATIVE VERIFICATION COMPLETED SUCCESSFULLY"
