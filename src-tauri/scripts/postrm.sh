#!/bin/sh
# Debian maintainer script — runs after the .deb package is removed.
# Only purge user data on `apt purge` (not on plain `remove`), and only for the installing user when invoked via sudo.
# See: https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html

# $1 is the action: remove|purge|upgrade|deconfigure|failed-upgrade|abort-install|abort-upgrade|disappear
ACTION="$1"

case "$ACTION" in
  purge)
    ;;
  *)
    exit 0
    ;;
esac

# Resolve the non-root user when apt runs this script as root.
if [ -n "${SUDO_USER:-}" ]; then
  TARGET_USER="$SUDO_USER"
elif [ -n "${LOGNAME:-}" ] && [ "$(id -u)" -eq 0 ] && [ "$LOGNAME" != "root" ]; then
  TARGET_USER="$LOGNAME"
else
  # Cannot infer a home directory safely; skip automatic cleanup.
  exit 0
fi

TARGET_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
if [ -z "$TARGET_HOME" ] || [ ! -d "$TARGET_HOME" ]; then
  exit 0
fi

# Match Kalam's ~/.kalam and ProjectDirs("com","Kalam","Kalam") on Linux (~/.local/share/kalam).
rm -rf "$TARGET_HOME/.kalam"
rm -rf "$TARGET_HOME/.local/share/kalam"

exit 0
