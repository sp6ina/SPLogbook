#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Instaluje biblioteki runtime wymagane przez kanoniczny artefakt SPLogbook
# wewnątrz przypiętego obrazu OCI profilu dystrybucji (F-03).
#
# Kontrakt:
#   - działa wyłącznie wewnątrz kontenera OCI (root),
#   - używa wyłącznie oficjalnego menedżera pakietów dystrybucji,
#   - jest nieinteraktywny i jawny (stałe listy pakietów, bez -dev),
#   - jest fail-closed: błąd instalacji lub brak sonamu => exit 1,
#   - nie używa LD_LIBRARY_PATH ani zewnętrznych repozytoriów.
#
# Uruchamiany PRZED scripts/verify_linux_container.sh.

set -eu

log() { printf '%s\n' "$*"; }
fail() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

[ -r /etc/os-release ] || fail "/etc/os-release is missing"
# shellcheck disable=SC1091
. /etc/os-release
DISTRO_ID="${ID:-unknown}"
DISTRO_LIKE="${ID_LIKE:-}"

log "=== RUNTIME DEPENDENCY INSTALLATION: ${DISTRO_ID} (like: ${DISTRO_LIKE}) ==="

# Sonames wymagane przez artefakt SPLogbook oraz jego biblioteki towarzyszące.
# Lista jest kontraktem: brak któregokolwiek sonamu po instalacji = FAIL.
REQUIRED_SONAMES="
libasound.so.2
libX11.so.6
libXcursor.so.1
libXrandr.so.2
libXi.so.6
libxkbcommon.so.0
libxkbcommon-x11.so.0
libGL.so.1
libfontconfig.so.1
libwayland-client.so.0
libwayland-cursor.so.0
libwayland-egl.so.1
libgtk-3.so.0
libusb-1.0.so.0
libudev.so.1
"

install_ubuntu() {
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends \
        file \
        binutils \
        libasound2t64 \
        libx11-6 \
        libxcursor1 \
        libxrandr2 \
        libxi6 \
        libxkbcommon0 \
        libxkbcommon-x11-0 \
        libgl1 \
        libfontconfig1 \
        libwayland-client0 \
        libwayland-cursor0 \
        libwayland-egl1 \
        libgtk-3-0t64 \
        libusb-1.0-0 \
        libudev1
}

install_fedora() {
    dnf install -y --setopt=install_weak_deps=False \
        file \
        binutils \
        alsa-lib \
        libX11 \
        libXcursor \
        libXrandr \
        libXi \
        libxkbcommon \
        libxkbcommon-x11 \
        mesa-libGL \
        fontconfig \
        libwayland-client \
        libwayland-cursor \
        libwayland-egl \
        gtk3 \
        libusb1 \
        systemd-libs
}

install_arch() {
    pacman -Sy --noconfirm --needed \
        file \
        binutils \
        alsa-lib \
        libx11 \
        libxcursor \
        libxrandr \
        libxi \
        libxkbcommon \
        libxkbcommon-x11 \
        mesa \
        fontconfig \
        wayland \
        gtk3 \
        libusb \
        systemd-libs
}

install_opensuse() {
    zypper --non-interactive --gpg-auto-import-keys refresh
    zypper --non-interactive install --no-recommends --auto-agree-with-licenses \
        file \
        binutils \
        libasound2 \
        libX11-6 \
        libXcursor1 \
        libXrandr2 \
        libXi6 \
        libxkbcommon0 \
        libxkbcommon-x11-0 \
        Mesa-libGL1 \
        libfontconfig1 \
        libwayland-client0 \
        libwayland-cursor0 \
        libwayland-egl1 \
        libgtk-3-0 \
        libusb-1_0-0 \
        libudev1
}

case "${DISTRO_ID}" in
    ubuntu | debian) install_ubuntu ;;
    fedora | rhel | centos | rocky | almalinux) install_fedora ;;
    arch) install_arch ;;
    opensuse* | suse | sles) install_opensuse ;;
    *)
        case " ${DISTRO_LIKE} " in
            *" ubuntu "* | *" debian "*) install_ubuntu ;;
            *" fedora "* | *" rhel "*) install_fedora ;;
            *" arch "*) install_arch ;;
            *" suse "* | *" opensuse "*) install_opensuse ;;
            *) fail "unsupported distribution: ID=${DISTRO_ID} ID_LIKE=${DISTRO_LIKE}" ;;
        esac
        ;;
esac

log "=== RUNTIME DEPENDENCY VERIFICATION (sonames) ==="
command -v ldconfig >/dev/null 2>&1 || fail "ldconfig is not available"
LDCONFIG_CACHE="$(ldconfig -p)" || fail "ldconfig -p failed"

MISSING=""
for soname in ${REQUIRED_SONAMES}; do
    if printf '%s\n' "${LDCONFIG_CACHE}" | grep -F "${soname} " >/dev/null 2>&1; then
        log "OK   ${soname}"
    else
        log "MISS ${soname}"
        MISSING="${MISSING} ${soname}"
    fi
done
[ -z "${MISSING}" ] || fail "missing runtime sonames:${MISSING}"

log "=== RUNTIME DEPENDENCIES INSTALLED SUCCESSFULLY ==="
