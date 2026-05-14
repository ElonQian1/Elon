#!/bin/bash
# setup-server.sh — 首次在服务器上初始化环境

set -e

echo "=== 安装依赖 ==="
sudo apt-get update -qq
sudo apt-get install -y git build-essential pkg-config libssl-dev

echo "=== 检查 Rust ==="
if ! command -v cargo &> /dev/null; then
  echo "安装 Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

echo "Rust 版本: $(rustc --version)"

echo "=== 初始化项目目录 ==="
mkdir -p /home/ubuntu/Elon

echo "=== 配置 git ==="
git config --global user.email "server@elon.app"
git config --global user.name "Elon Server"

echo "=== 服务器初始化完成 ==="
