#!/bin/bash
# deploy.sh — 将服务端代码同步到服务器并重启服务
#
# ⚠️ 安全规则：必须基于已 commit 的 HEAD 构建，通过临时 worktree 隔离，
#    避免把主工作区其他 AI 未提交的改动一并 rsync 到服务器。

set -e

SERVER="root@43.139.149.158"
REMOTE_DIR="/root/Elon"
SERVICE_NAME="elon-server"

# 获取仓库根目录和当前 HEAD SHA
REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
SHA_BIG=$(git -C "$REPO_ROOT" rev-parse HEAD)
TMP_WORKTREE="${REPO_ROOT}/../Elon-deploy-temp-${SHA}"

# 注册退出时自动清理临时工作树
cleanup() {
  echo "=== 清理临时工作树 ==="
  git -C "$REPO_ROOT" worktree remove "$TMP_WORKTREE" --force 2>/dev/null || true
}
trap cleanup EXIT

echo "=== 基于 commit ${SHA} 创建临时工作树 ==="
git -C "$REPO_ROOT" worktree add --detach "$TMP_WORKTREE" HEAD

echo "=== 同步代码到服务器（基于 ${SHA}，排除 target/ 和 .env） ==="
ssh $SERVER "mkdir -p $REMOTE_DIR"
rsync -avz --exclude='target/' --exclude='.env' \
  "${TMP_WORKTREE}/" \
  "$SERVER:$REMOTE_DIR/"

echo "=== 在服务器上编译 Rust ==="
ssh $SERVER "
  source \$HOME/.cargo/env
  cd $REMOTE_DIR/server
  ELON_SERVER_GIT_SHA='$SHA_BIG' cargo build --release 2>&1 | tail -10
"

echo "=== 重启服务 ==="
ssh $SERVER "
  if systemctl is-active --quiet $SERVICE_NAME 2>/dev/null; then
    sudo systemctl restart $SERVICE_NAME
    echo '服务已重启'
  else
    pkill -f elon-server 2>/dev/null || true
    sleep 1
    nohup $REMOTE_DIR/server/target/release/elon-server \
      > /root/elon-server.log 2>&1 &
    echo '服务已启动，PID: '$!
  fi
"

echo "=== 验证 ==="
ssh $SERVER 'curl -s http://localhost:8080/health'
ssh $SERVER 'curl -s http://localhost:8080/api/server/version'

echo ""
echo "=== 部署完成，基于 SHA: ${SHA} ==="
echo "服务地址: http://43.139.149.158:8080"
echo "健康检查: curl http://43.139.149.158:8080/health"
echo "后端版本: curl http://43.139.149.158:8080/api/server/version"
