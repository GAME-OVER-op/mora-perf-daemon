#!/usr/bin/env sh
set -e
if command -v gradle >/dev/null 2>&1; then
  exec gradle "$@"
fi
echo "gradle command not found. Install Gradle or use GitHub Actions setup-gradle." >&2
exit 127
