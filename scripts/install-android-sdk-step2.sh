#!/bin/bash
set -euo pipefail
# 第二步：安装 SDK 组件并创建项目模板
set -e
LOG=/tmp/android-sdk-step2.log
exec > "$LOG" 2>&1

export JAVA_HOME=$(dirname "$(dirname "$(readlink -f "$(which java)")")")
export ANDROID_HOME=$HOME/android-sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools

SDKMANAGER=$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager

echo "[1/3] 接受所有 License..."
yes | "$SDKMANAGER" --licenses 2>/dev/null || true

echo "[2/3] 安装 platforms;android-34 和 build-tools;34.0.0..."
"$SDKMANAGER" "platforms;android-34" "build-tools;34.0.0"

echo "[3/3] 验证安装..."
"$SDKMANAGER" --list_installed

echo "[DONE] Android SDK 组件安装完毕"
