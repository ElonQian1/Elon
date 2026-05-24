#!/bin/bash
# Legacy compatibility wrapper.
#
# The old deploy.sh rsynced source code to the server and ran cargo build there.
# That flow is intentionally retired: the server is low-powered and should only
# receive already-built artifacts. Keep this wrapper so older instructions fail
# into the supported local cross-compile pipeline instead of reviving remote
# compilation.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1:-server}" in
  server)
    if [ "$#" -gt 0 ]; then
      shift
    fi
    exec "$SCRIPT_DIR/publish-server.sh" "$@"
    ;;
  apk)
    echo "APK releases use scripts/publish-apk.ps1 from a Windows/Android build environment." >&2
    echo "Do not use deploy.sh for APK publishing." >&2
    exit 2
    ;;
  *)
    exec "$SCRIPT_DIR/publish-server.sh" "$@"
    ;;
esac
