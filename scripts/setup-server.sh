#!/bin/bash
# setup-server.sh — 首次在服务器上初始化环境

set -e

echo "=== 安装依赖 ==="
sudo apt-get update -qq
sudo apt-get install -y git build-essential pkg-config libssl-dev openjdk-17-jdk unzip wget curl

echo "=== 检查 Rust ==="
if ! command -v cargo &> /dev/null; then
  echo "安装 Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

echo "Rust 版本: $(rustc --version)"
echo "Java 版本: $(java -version 2>&1 | head -1)"

echo "=== 初始化项目目录 ==="
mkdir -p /root/Elon
mkdir -p /root/workspaces /root/templates

echo "=== 配置 git ==="
git config --global user.email "server@elon.app"
git config --global user.name "Elon Server"

echo "=== 下一步 ==="
echo "如需在服务器上打包 APK，请继续执行："
echo "  bash scripts/install-android-sdk.sh"
echo "  bash scripts/install-android-sdk-step2.sh"
echo "  bash scripts/create-android-template.sh"

echo "=== 服务器初始化完成 ==="
