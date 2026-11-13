SPLogbook — Linux RPM package

Package:        splogbook
Version:        1.3.0
Release:        0.1.alpha.1
Architecture:   x86_64
Installed root: /opt/SPLogbook
Launcher:       /usr/bin/splogbook

This is a binary RPM built from the verified SPLogbook portable payload.  It
does not contain a source RPM, build recipes or a package repository.

The application keeps all user data outside the package:

  configuration  $XDG_CONFIG_HOME/splogbook
  data / QSO DB  $XDG_DATA_HOME/splogbook
  state / logs   $XDG_STATE_HOME/splogbook
  cache          $XDG_CACHE_HOME/splogbook

Install, upgrade, downgrade, reinstall and erase never modify or delete QSO
databases, WAL/SHM files, station configuration, ADIF/Cabrillo exports or
backups.

Run:

  splogbook

The private Hamlib shared library is bundled at
/opt/SPLogbook/lib/libhamlib.so.4.  The application binaries use the relative
RUNPATH $ORIGIN/lib and the library uses $ORIGIN; no system-wide Hamlib package
is required.

Licenses:

  GPL-3.0-or-later              /opt/SPLogbook/LICENSE
  LGPL-2.1-or-later (Hamlib)    /usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt

This package is a CI artifact in M3-I and is not published as a public RPM
repository or GitHub Release asset.
