#!/bin/sh
# Copied to `postinstall` (no extension) beside `pkgbuild --scripts` when building the macOS .pkg in CI.
# Runs as root after files are installed. Does not modify user home; logs data paths for troubleshooting.

/usr/bin/logger -t kalam-installer "Kalam user data: ~/.kalam and ~/Library/Application Support/com.Kalam.Kalam (models/sidecars). Remove manually to fully uninstall."

exit 0
