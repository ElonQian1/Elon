#!/bin/bash
# 验证模板：复制到临时目录 → 编译 → 检查 APK
set -e
LOG=/tmp/template-build-test.log
exec > "$LOG" 2>&1

export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
export ANDROID_HOME=/home/ubuntu/android-sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools

TEST_DIR=/tmp/android-template-test
rm -rf "$TEST_DIR"
cp -r /home/ubuntu/templates/android "$TEST_DIR"
cd "$TEST_DIR"

echo "[1/2] 开始编译（首次需要下载 Gradle ~150MB）..."
chmod +x gradlew
./gradlew assembleDebug --no-daemon --stacktrace 2>&1

echo "[2/2] 检查 APK 产物..."
find . -name "*.apk" -type f
echo "BUILD_SUCCESS"
