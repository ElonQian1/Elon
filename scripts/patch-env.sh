#!/bin/bash
set -e
ENV_FILE=/root/Elon/server/.env
JAVA_HOME_PATH=$(dirname $(dirname $(readlink -f $(which java))))

grep -q 'ANDROID_HOME' "$ENV_FILE" || echo 'ANDROID_HOME=/root/android-sdk' >> "$ENV_FILE"
grep -q 'JAVA_HOME'    "$ENV_FILE" || echo "JAVA_HOME=$JAVA_HOME_PATH"             >> "$ENV_FILE"

echo "--- .env 环境变量（非敏感部分）---"
grep -E '^(ANDROID_HOME|JAVA_HOME|WORKSPACE_ROOT|PUBLIC_URL|LISTEN_ADDR)' "$ENV_FILE"
