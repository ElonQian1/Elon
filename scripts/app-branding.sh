#!/usr/bin/env bash
# Source after REPO_ROOT is resolved; never eval JSON.
ELON_APP_APK_NAMES="$(python3 - "$REPO_ROOT/server/src/assets/app_branding.json" <<'PY'
import json, re, sys
with open(sys.argv[1], encoding='utf-8-sig') as config:
    branding = json.load(config)
names = [branding[key] for key in ('apkFileName', 'legacyApkFileName')]
if not all(isinstance(name, str) and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*\.apk', name) for name in names):
    sys.exit('Invalid main-app APK filename')
print('|'.join(names))
PY
)" || return 1
APK_FILE_NAME="${ELON_APP_APK_NAMES%%|*}"
APK_STORAGE_NAME="${ELON_APP_APK_NAMES#*|}"
unset ELON_APP_APK_NAMES
