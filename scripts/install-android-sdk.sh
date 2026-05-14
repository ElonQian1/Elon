#!/bin/bash
set -e
LOG=/tmp/android-sdk-install.log
exec > "$LOG" 2>&1

echo "[1/5] 配置 JAVA_HOME..."
JAVA_BIN=$(readlink -f $(which java))
JAVA_HOME_PATH=$(dirname $(dirname $JAVA_BIN))
echo "JAVA_HOME = $JAVA_HOME_PATH"

# 写入 ~/.bashrc（幂等）
grep -q "JAVA_HOME" ~/.bashrc   || echo "export JAVA_HOME=$JAVA_HOME_PATH" >> ~/.bashrc
grep -q "ANDROID_HOME" ~/.bashrc || echo 'export ANDROID_HOME=$HOME/android-sdk' >> ~/.bashrc
grep -q 'cmdline-tools/latest/bin' ~/.bashrc || \
  echo 'export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools' >> ~/.bashrc

echo "[2/5] 创建 SDK 目录..."
mkdir -p ~/android-sdk/cmdline-tools

echo "[3/5] 下载 Android cmdline-tools (~130MB)..."
cd ~/android-sdk/cmdline-tools
if [ ! -f cmdline-tools.zip ]; then
  wget -q --show-progress \
    "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" \
    -O cmdline-tools.zip
else
  echo "已存在，跳过下载"
fi

echo "[4/5] 解压..."
unzip -q -o cmdline-tools.zip
# sdkmanager 要求目录名必须是 'latest'
[ -d latest ] && rm -rf latest
mv cmdline-tools latest

echo "[DONE] cmdline-tools 安装完毕"
ls ~/android-sdk/cmdline-tools/latest/bin/
