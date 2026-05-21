#!/bin/bash
# deploy.sh — 将服务端代码同步到服务器并重启服务

set -e

SERVER="root@43.139.149.158"
REMOTE_DIR="/root/Elon"
SERVICE_NAME="elon-server"

echo "=== 同步代码到服务器 ==="
ssh $SERVER "mkdir -p $REMOTE_DIR"
rsync -avz --exclude='target/' --exclude='.env' \
  "$(dirname "$0")/../" \
  "$SERVER:$REMOTE_DIR/"

echo "=== 在服务器上编译 Rust ==="
ssh $SERVER "
  source \$HOME/.cargo/env
  cd $REMOTE_DIR/server
  cargo build --release 2>&1
"

echo "=== 重启服务 ==="
ssh $SERVER "
  # 如果用 systemd 管理服务
  if systemctl is-active --quiet $SERVICE_NAME 2>/dev/null; then
    sudo systemctl restart $SERVICE_NAME
    echo '服务已重启'
  else
    # 否则直接后台运行
    pkill -f elon-server 2>/dev/null || true
    sleep 1
    nohup $REMOTE_DIR/server/target/release/elon-server \
      > /root/elon-server.log 2>&1 &
    echo '服务已启动，PID: '$!
  fi
"

echo "=== 部署完成 ==="
echo "服务地址: http://43.139.149.158:8080"
echo "健康检查: curl http://43.139.149.158:8080/health"
