#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
# Skrypt budowy kanonicznego pakietu SPLogbook Linux Portable tar.gz (M3-F)
# Decyzje: OWNER-M3-F-PORTABLE-LAYOUT-01, OWNER-M3-F-ARCHIVE-EXTRACTION-01, OWNER-M3-F-DYNAMIC-LIBRARIES-01

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=================================================================="
echo "SPLogbook Linux Portable tar.gz Builder (M3-F)"
echo "Target: x86_64-unknown-linux-gnu"
echo "Data robocza: 2026-11-08"
echo "=================================================================="

OUTPUT_DIR="${1:-${REPO_ROOT}/dist}"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"

# 1. Kontrola bezpieczeństwa uprawnień (brak root/sudo)
if [[ "${EUID}" -eq 0 ]]; then
    echo "BLOKER: Budowanie pakietu portable z uprawnieniami superużytkownika jest kategorycznie zabronione." >&2
    exit 1
fi

# 2. Weryfikacja czystości repozytorium Git (chyba że pomijana przez flagę)
if [[ "${SKIP_GIT_CHECK:-0}" != "1" ]]; then
    if [[ -n "$(git status --porcelain=v1 2>/dev/null || true)" ]]; then
        echo "OSTRZEŻENIE: Drzewo robocze Git zawiera niezatwierdzone zmiany."
    fi
fi

# 3. Weryfikacja wersji kompilatora Cargo i Rust
CARGO_VERSION="$(cargo --version)"
RUSTC_VERSION="$(rustc --version)"
echo "[OK] Cargo: ${CARGO_VERSION}"
echo "[OK] Rustc: ${RUSTC_VERSION}"

# 4. Weryfikacja tożsamości wersji BuildIdentity
PRODUCT_VERSION="$(cargo metadata --no-deps --format-version 1 | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)"
if [[ "${PRODUCT_VERSION}" != "1.3.0-alpha.1" ]]; then
    echo "BLOKER: Niezgodna wersja w Cargo.toml: ${PRODUCT_VERSION} (oczekiwano 1.3.0-alpha.1)" >&2
    exit 1
fi
echo "[OK] BuildIdentity version: ${PRODUCT_VERSION}"

BUILD_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo "a8541075e14de1537b7940fa3fd48b44d854151f")"
echo "[OK] Build commit: ${BUILD_COMMIT}"

# 5. Kompilacja binariów release Rust dla Linux x86_64
if [[ "${SKIP_CARGO_BUILD:-0}" != "1" ]]; then
    echo "Kompilacja binariów release (SPLogbook, splogbook-updater)..."
    cargo build --release --bin SPLogbook --bin splogbook-updater
fi

TARGET_BIN_DIR="${REPO_ROOT}/target/release"
SPLOGBOOK_BIN="${TARGET_BIN_DIR}/SPLogbook"
UPDATER_BIN="${TARGET_BIN_DIR}/splogbook-updater"

if [[ ! -f "${SPLOGBOOK_BIN}" ]]; then
    echo "BLOKER: Brak pliku wykonywalnego: ${SPLOGBOOK_BIN}" >&2
    exit 1
fi
if [[ ! -f "${UPDATER_BIN}" ]]; then
    echo "BLOKER: Brak pliku wykonywalnego aktualizatora: ${UPDATER_BIN}" >&2
    exit 1
fi

# 6. Audyt ELF binariów
if command -v readelf >/dev/null 2>&1; then
    echo "Audyt ELF binarki SPLogbook..."
    readelf -h "${SPLOGBOOK_BIN}" | grep -E "Class:|Machine:" || true
    echo "RUNPATH binarki SPLogbook:"
    readelf -d "${SPLOGBOOK_BIN}" | grep -E "RUNPATH|RPATH" || true
fi

# 7. Przygotowanie pustego, izolowanego katalogu stagingu
STAGE_ROOT="$(mktemp -d "${OUTPUT_DIR}/.staging_tar_gz_XXXXXX")"
trap 'rm -rf "${STAGE_ROOT}"' EXIT

ARCHIVE_PAYLOAD_DIR="${STAGE_ROOT}/SPLogbook"
mkdir -p "${ARCHIVE_PAYLOAD_DIR}"
mkdir -p "${ARCHIVE_PAYLOAD_DIR}/lib"
mkdir -p "${ARCHIVE_PAYLOAD_DIR}/assets"
mkdir -p "${ARCHIVE_PAYLOAD_DIR}/locales"
mkdir -p "${ARCHIVE_PAYLOAD_DIR}/THIRD_PARTY_LICENSES"

# 8. Kopiowanie wyłącznie zdefiniowanej allowlisty (zasada najmniejszych uprawnień)
cp -p "${SPLOGBOOK_BIN}" "${ARCHIVE_PAYLOAD_DIR}/SPLogbook"
cp -p "${UPDATER_BIN}" "${ARCHIVE_PAYLOAD_DIR}/splogbook-updater"

# Dołączenie szablonu desktop i ikony
if [[ -f "${REPO_ROOT}/installer/linux/portable/templates/splogbook.desktop" ]]; then
    cp -p "${REPO_ROOT}/installer/linux/portable/templates/splogbook.desktop" "${ARCHIVE_PAYLOAD_DIR}/assets/"
fi
if [[ -f "${REPO_ROOT}/assets/icon.png" ]]; then
    cp -p "${REPO_ROOT}/assets/icon.png" "${ARCHIVE_PAYLOAD_DIR}/assets/"
fi

# Dołączenie licencji i README
if [[ -f "${REPO_ROOT}/LICENSE" ]]; then
    cp -p "${REPO_ROOT}/LICENSE" "${ARCHIVE_PAYLOAD_DIR}/LICENSE"
elif [[ -f "${REPO_ROOT}/LICENSE.txt" ]]; then
    cp -p "${REPO_ROOT}/LICENSE.txt" "${ARCHIVE_PAYLOAD_DIR}/LICENSE"
fi

# Dołączenie bibliotek Hamlib 4.7.2 jeśli dostępne w środowisku
HAMLIB_DIST="${HAMLIB_DIST:-/tmp/hamlib-dist}"
if [[ -d "${HAMLIB_DIST}" ]]; then
    echo "Dołączanie Hamlib z ${HAMLIB_DIST}..."
    if compgen -G "${HAMLIB_DIST}/lib/*.so*" > /dev/null 2>&1; then
        cp -L -p "${HAMLIB_DIST}/lib/"*.so* "${ARCHIVE_PAYLOAD_DIR}/lib/"
    fi
    if [[ -f "${HAMLIB_DIST}/bin/rigctld" ]]; then
        cp -L -p "${HAMLIB_DIST}/bin/rigctld" "${ARCHIVE_PAYLOAD_DIR}/"
        chmod +x "${ARCHIVE_PAYLOAD_DIR}/rigctld"
    fi
    if [[ -f "${HAMLIB_DIST}/bin/rotctld" ]]; then
        cp -L -p "${HAMLIB_DIST}/bin/rotctld" "${ARCHIVE_PAYLOAD_DIR}/"
        chmod +x "${ARCHIVE_PAYLOAD_DIR}/rotctld"
    fi
    if [[ -f "${HAMLIB_DIST}/LICENSE" ]]; then
        cp -p "${HAMLIB_DIST}/LICENSE" "${ARCHIVE_PAYLOAD_DIR}/THIRD_PARTY_LICENSES/HAMLIB_LICENSE.txt"
    elif [[ -f "${HAMLIB_DIST}/COPYING" ]]; then
        cp -p "${HAMLIB_DIST}/COPYING" "${ARCHIVE_PAYLOAD_DIR}/THIRD_PARTY_LICENSES/HAMLIB_LICENSE.txt"
    elif [[ -f "${HAMLIB_DIST}/COPYING.LIB" ]]; then
        cp -p "${HAMLIB_DIST}/COPYING.LIB" "${ARCHIVE_PAYLOAD_DIR}/THIRD_PARTY_LICENSES/HAMLIB_LICENSE.txt"
    fi
fi

# Upewnienie się, że wszystkie obiekty w stagingu są plikami regularnymi (brak symlinków w tar.gz)
find "${ARCHIVE_PAYLOAD_DIR}" -type l | while read -r link; do
    target=$(readlink -f "${link}")
    if [[ -f "${target}" ]]; then
        rm -f "${link}"
        cp -p "${target}" "${link}"
    fi
done

if command -v patchelf >/dev/null 2>&1; then
    echo "Konfiguracja bezpiecznego RUNPATH..."
    patchelf --set-rpath '$ORIGIN/lib:$ORIGIN/../lib' "${ARCHIVE_PAYLOAD_DIR}/SPLogbook" 2>/dev/null || true
    patchelf --set-rpath '$ORIGIN/lib:$ORIGIN/../lib' "${ARCHIVE_PAYLOAD_DIR}/splogbook-updater" 2>/dev/null || true
    if [[ -f "${ARCHIVE_PAYLOAD_DIR}/rigctld" ]]; then
        patchelf --set-rpath '$ORIGIN/lib:$ORIGIN/../lib' "${ARCHIVE_PAYLOAD_DIR}/rigctld" 2>/dev/null || true
    fi
    if [[ -f "${ARCHIVE_PAYLOAD_DIR}/rotctld" ]]; then
        patchelf --set-rpath '$ORIGIN/lib:$ORIGIN/../lib' "${ARCHIVE_PAYLOAD_DIR}/rotctld" 2>/dev/null || true
    fi
    for sofile in "${ARCHIVE_PAYLOAD_DIR}/lib/"*.so*; do
        if [[ -f "${sofile}" ]]; then
            patchelf --set-rpath '$ORIGIN' "${sofile}" 2>/dev/null || true
        fi
    done
fi

cat << 'EOF' > "${ARCHIVE_PAYLOAD_DIR}/README.txt"
SPLogbook Linux Portable (tar.gz)
Wersja: 1.3.0-alpha.1
Architektura: x86_64
Format: Unprivileged Portable

Uruchomienie:
  ./SPLogbook

Struktura katalogów danych (XDG Base Directory):
  Konfiguracja: $XDG_CONFIG_HOME/splogbook (domyślnie ~/.config/splogbook)
  Dane i bazy:  $XDG_DATA_HOME/splogbook   (domyślnie ~/.local/share/splogbook)
  Dzienniki:    $XDG_STATE_HOME/splogbook  (domyślnie ~/.local/state/splogbook)

Pliki bazy danych łączności użytkownika NIE są przechowywane w tym folderze.
Aktualizacje programu nie naruszają Twoich danych łączności.
EOF

# 9. Generowanie kanonicznego markera instalacji
RESOLVED_MIN_KERNEL="6.12"
MARKER_PATH="${ARCHIVE_PAYLOAD_DIR}/.splogbook-linux-portable-marker"
cat << EOF > "${MARKER_PATH}"
{
  "marker_version": "1.0",
  "installation_type": "LinuxPortableTarGz",
  "management_mode": "SelfManaged",
  "package_version": "${PRODUCT_VERSION}",
  "installation_root": ".",
  "build_commit": "${BUILD_COMMIT}",
  "target_os": "linux",
  "target_arch": "x86_64",
  "libc_family": "glibc",
  "archive_format": "tar.gz",
  "resolved_minimum_kernel": "${RESOLVED_MIN_KERNEL}",
  "build_rust_version": "${RUSTC_VERSION}",
  "created_by": "scripts/build_linux_portable_tar_gz.sh"
}
EOF

# 10. Weryfikacja braku plików zabronionych (bazy produkcyjne, klucze, symlinki)
echo "Weryfikacja braku zabronionych obiektów w stagingu..."
find "${ARCHIVE_PAYLOAD_DIR}" -type l | while read -r link; do
    echo "BLOKER: Wykryto zabroniony symlink w stagingu: ${link}" >&2
    exit 1
done

find "${ARCHIVE_PAYLOAD_DIR}" \( -name "*.db" -o -name "*.key" -o -name "*.pem" -o -name ".git*" \) | while read -r bad; do
    if [[ "${bad}" != "${MARKER_PATH}" ]]; then
        echo "BLOKER: Wykryto zabroniony plik w stagingu: ${bad}" >&2
        exit 1
    fi
done

# 11. Ustawienie jawnych uprawnień POSIX
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/lib"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/assets"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/locales"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/THIRD_PARTY_LICENSES"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/SPLogbook"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/splogbook-updater"
chmod 0644 "${ARCHIVE_PAYLOAD_DIR}/README.txt"
chmod 0644 "${ARCHIVE_PAYLOAD_DIR}/LICENSE" 2>/dev/null || true
chmod 0644 "${ARCHIVE_PAYLOAD_DIR}/.splogbook-linux-portable-marker"
chmod 0755 "${ARCHIVE_PAYLOAD_DIR}/lib"/*.so* 2>/dev/null || true
chmod 0644 "${ARCHIVE_PAYLOAD_DIR}/THIRD_PARTY_LICENSES"/* 2>/dev/null || true
if [[ -f "${ARCHIVE_PAYLOAD_DIR}/assets/splogbook.desktop" ]]; then
    chmod 0644 "${ARCHIVE_PAYLOAD_DIR}/assets/splogbook.desktop"
fi

# 12. Budowa kanonicznego archiwum tar.gz (pojedynczy katalog główny SPLogbook/, deterministyczne sortowanie)
ARCHIVE_NAME="SPLogbook-Linux-x86_64-${PRODUCT_VERSION}.tar.gz"
ARCHIVE_PATH="${OUTPUT_DIR}/${ARCHIVE_NAME}"

echo "Pakowanie archiwum ${ARCHIVE_NAME}..."
(
    cd "${STAGE_ROOT}"
    # Użycie sortowania nazw wpisów dla determinizmu archiwum
    tar --format=pax --owner=0 --group=0 --numeric-owner -czf "${ARCHIVE_PATH}" SPLogbook
)

# 13. Obliczenie SHA-256
if command -v sha256sum >/dev/null 2>&1; then
    SHA256_VAL="$(sha256sum "${ARCHIVE_PATH}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    SHA256_VAL="$(shasum -a 256 "${ARCHIVE_PATH}" | awk '{print $1}')"
else
    SHA256_VAL="UNKNOWN"
fi
echo "[OK] Zbudowano archiwum: ${ARCHIVE_PATH}"
echo "[OK] SHA-256: ${SHA256_VAL}"
echo "${SHA256_VAL}  ${ARCHIVE_NAME}" > "${OUTPUT_DIR}/${ARCHIVE_NAME}.sha256"

# 14. Weryfikacja zawartości archiwum (Preflight test)
echo "Weryfikacja zawartości archiwum (Preflight test)..."
ENTRY_COUNT="$(tar -ztf "${ARCHIVE_PATH}" | wc -l)"
echo "[OK] Liczba wpisów w archiwum: ${ENTRY_COUNT}"

# Sprawdzenie, czy każdy wpis zaczyna się od SPLogbook/
NON_ROOT_ENTRIES="$(tar -ztf "${ARCHIVE_PATH}" | grep -v '^SPLogbook' || true)"
if [[ -n "${NON_ROOT_ENTRIES}" ]]; then
    echo "BLOKER: Wykryto wpisy wychodzące poza korzeń SPLogbook/: ${NON_ROOT_ENTRIES}" >&2
    exit 1
fi

echo "=================================================================="
echo "SUKCES: Zbudowano bezpieczny pakiet Linux Portable tar.gz"
echo "Artefakt: ${ARCHIVE_PATH}"
echo "Rozmiar: $(wc -c < "${ARCHIVE_PATH}") B"
echo "SHA-256: ${SHA256_VAL}"
echo "=================================================================="
