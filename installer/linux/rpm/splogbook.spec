# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Mariusz Woźniak (SP6INA)
#
# Binary RPM contract for SPLogbook Linux x86_64 (M3-I).
#
# The package is assembled from the already verified portable payload.  It does
# not compile source code, does not create a source RPM and never installs user
# data.  The builder passes splog_version, splog_release and splog_commit.

%{!?splog_version:%global splog_version 1.3.0}
%{!?splog_release:%global splog_release 0.1.alpha.1}
%{!?splog_commit:%global splog_commit 0000000000000000000000000000000000000000}

# Deterministic package header and payload.  The verified portable payload is
# already stripped, so the distribution post-processing scripts are disabled;
# this preserves the exact binaries that passed the M3-F/M3-G/M3-H checks.
%global use_source_date_epoch_as_buildtime 1
%global clamp_mtime_to_source_date_epoch 1
%global _buildhost build.splogbook.invalid
%global debug_package %{nil}
%global _build_id_links none
%global __os_install_post %{nil}
%global _binary_payload w6.xzdio
%global _binary_filedigest_algorithm 8
%global _source_filedigest_algorithm 8

Name:           splogbook
Version:        %{splog_version}
Release:        %{splog_release}
Summary:        Amateur radio logging and station automation workstation
License:        GPL-3.0-or-later AND LGPL-2.1-or-later
URL:            https://github.com/sp6ina/SPLogbook
Vendor:         SPLogbook
Packager:       Mariusz Woźniak (SP6INA) <contact@splogbook.org>
Group:          Applications/Communications
BuildArch:      x86_64
AutoReqProv:    no

# The private Hamlib is bundled under /opt/SPLogbook/lib and must never appear
# as an RPM dependency.  The remaining capabilities are distribution-neutral
# SONAME/GLIBC capabilities understood by both Fedora and openSUSE.
Requires:       glibc >= 2.43
Requires:       libasound.so.2()(64bit)
Requires:       libX11.so.6()(64bit)
Requires:       libXcursor.so.1()(64bit)
Requires:       libXrandr.so.2()(64bit)
Requires:       libXi.so.6()(64bit)
Requires:       libxkbcommon.so.0()(64bit)
Requires:       libxkbcommon-x11.so.0()(64bit)
Requires:       libGL.so.1()(64bit)
Requires:       libfontconfig.so.1()(64bit)
Requires:       libwayland-client.so.0()(64bit)
Requires:       libwayland-cursor.so.0()(64bit)
Requires:       libwayland-egl.so.1()(64bit)
Requires:       libgtk-3.so.0()(64bit)
Requires:       libusb-1.0.so.0()(64bit)
Requires:       libudev.so.1()(64bit)
Requires:       libgcc_s.so.1()(64bit)
Requires:       libm.so.6()(64bit)
Requires:       libc.so.6()(64bit)

BuildRequires:  coreutils
BuildRequires:  findutils
BuildRequires:  sed
BuildRequires:  xz

%description
SPLogbook is a desktop application for amateur radio operators. It provides
QSO logging, ADIF and Cabrillo import and export, contest and award tracking,
DX cluster connectivity, propagation and satellite tools, CAT control and
audio recording.

This package installs the system-wide application below /opt/SPLogbook, a
command launcher at /usr/bin/splogbook and desktop integration files. It does
not install, modify or remove user data, QSO databases or configuration stored
in the XDG directories.

%prep
: prebuilt payload - no source preparation required

%build
: prebuilt payload - no compilation required

%install
rm -rf "%{buildroot}"
install -d "%{buildroot}"
cp -a "%{_sourcedir}/root/." "%{buildroot}/"
if [ -n "${SOURCE_DATE_EPOCH:-}" ]; then
    find "%{buildroot}" -print0 | xargs -0 touch -h -d "@${SOURCE_DATE_EPOCH}"
fi

%post
# Refresh desktop integration caches only when the official helper is already
# present.  No network, no package manager, no user data, no escalation.
if command -v update-desktop-database >/dev/null 2>&1; then
    if ! update-desktop-database -q /usr/share/applications; then
        echo "splogbook: warning: update-desktop-database failed" >&2
    fi
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    if [ -d /usr/share/icons/hicolor ]; then
        if ! gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor; then
            echo "splogbook: warning: gtk-update-icon-cache failed" >&2
        fi
    fi
fi
exit 0

%postun
# Removal and upgrade must never touch user data, QSO databases, WAL/SHM
# files, configuration, exports or backups.  Only desktop caches are refreshed.
case "${1:-}" in
    0|1)
        if command -v update-desktop-database >/dev/null 2>&1; then
            if ! update-desktop-database -q /usr/share/applications; then
                echo "splogbook: warning: update-desktop-database failed" >&2
            fi
        fi
        if command -v gtk-update-icon-cache >/dev/null 2>&1; then
            if [ -d /usr/share/icons/hicolor ]; then
                if ! gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor; then
                    echo "splogbook: warning: gtk-update-icon-cache failed" >&2
                fi
            fi
        fi
        ;;
esac
exit 0

%files
%defattr(-,root,root,-)
%dir /opt/SPLogbook
%attr(0755,root,root) /opt/SPLogbook/SPLogbook
%attr(0755,root,root) /opt/SPLogbook/splogbook-updater
%dir /opt/SPLogbook/lib
%attr(0755,root,root) /opt/SPLogbook/lib/libhamlib.so.4
%dir /opt/SPLogbook/locales
%dir /opt/SPLogbook/THIRD_PARTY_LICENSES
%attr(0644,root,root) /opt/SPLogbook/THIRD_PARTY_LICENSES/HAMLIB_LGPL-2.1.txt
%license /opt/SPLogbook/LICENSE
%attr(0644,root,root) /opt/SPLogbook/.splogbook-rpm-marker

%attr(0755,root,root) /usr/bin/splogbook
%attr(0644,root,root) /usr/share/applications/splogbook.desktop
%attr(0644,root,root) /usr/share/icons/hicolor/256x256/apps/splogbook.png
%dir /usr/share/doc/splogbook
%doc /usr/share/doc/splogbook/README.RPM
%dir /usr/share/licenses/splogbook
%license /usr/share/licenses/splogbook/LICENSE
%license /usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt

%changelog
* Fri Nov 13 2026 Mariusz Woźniak (SP6INA) <contact@splogbook.org> - 1.3.0-0.1.alpha.1
- Binary RPM build for the SPLogbook 1.3.0-alpha.1 development release.
