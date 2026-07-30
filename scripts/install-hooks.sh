#!/usr/bin/env bash
# 启用本仓库的 Git 守门 hooks（pre-commit 文档模块化 + pre-push 质量门禁）。
# 任何 PC（包括服务器 codex CLI）clone 后必须执行一次。
set -e
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit 2>/dev/null || true
chmod +x .githooks/pre-push 2>/dev/null || true

echo "✅ 已启用 .githooks/ 作为 hook 目录"
echo "   pre-commit：阻止正式文档继续变成巨型文件"
echo "   pre-push：重复文档/源码门禁，并保留可选 Rust receipt"
git config --get core.hooksPath
