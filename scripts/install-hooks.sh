#!/usr/bin/env bash
# 启用本仓库的 git 守门 hooks（pre-push: cheap gates + 可选 Rust receipt + 未 add .rs）
# 任何 PC（包括服务器 codex CLI）clone 后必须执行一次。
set -e
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/pre-push 2>/dev/null || true

echo "✅ 已启用 .githooks/ 作为 hook 目录"
echo "   pre-push 守门已激活：cheap gates + 可选 Rust receipt + 未 add .rs"
git config --get core.hooksPath
