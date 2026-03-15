#!/system/bin/sh
if command -v gradle >/dev/null 2>&1; then
  exec gradle "$@"
fi
echo "gradle command not found. Install Gradle in Termux or open the project in Android Studio." >&2
exit 1
