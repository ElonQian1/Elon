#!/bin/bash
# scripts/setup-whisper.sh
# 在服务器上安装本地 Whisper ASR 服务（faster-whisper + FastAPI + systemd）
# 运行方式：bash scripts/setup-whisper.sh
# 也可以通过 publish-server.ps1 调用，或手动 SSH 到服务器执行

set -euo pipefail

DEPLOY_DIR="/opt/elon"
SERVICE_FILE="/etc/systemd/system/elon-whisper.service"
SERVICE_PY="${DEPLOY_DIR}/whisper_service.py"

echo "=== 1. 安装 Python 依赖 ==="
pip3 install --quiet faster-whisper fastapi "uvicorn[standard]" python-multipart

echo "=== 2. 部署 whisper_service.py ==="
mkdir -p "${DEPLOY_DIR}"
# 脚本目录在 scripts/，服务文件在同目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "${SCRIPT_DIR}/whisper_service.py" "${SERVICE_PY}"
echo "已写入 ${SERVICE_PY}"

echo "=== 3. 预下载 Whisper base 模型（首次约 74MB） ==="
python3 -c "
from faster_whisper import WhisperModel
print('正在下载/验证 base 模型...')
WhisperModel('base', device='cpu', compute_type='int8')
print('模型就绪')
"

echo "=== 4. 写入 systemd 服务 ==="
cat > "${SERVICE_FILE}" << 'UNIT'
[Unit]
Description=Elon Local Whisper ASR Service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/elon
ExecStart=/usr/bin/uvicorn whisper_service:app --host 127.0.0.1 --port 5001
Restart=always
RestartSec=5
# 内存保护：base 模型约 400MB，限制 800MB 防止内存泄漏
MemoryMax=800M
StandardOutput=append:/var/log/elon-whisper.log
StandardError=append:/var/log/elon-whisper.log

[Install]
WantedBy=multi-user.target
UNIT

echo "=== 5. 启动并设为开机自启 ==="
systemctl daemon-reload
systemctl enable elon-whisper
systemctl restart elon-whisper
sleep 2

echo "=== 6. 验证 ==="
if curl -sf http://127.0.0.1:5001/health > /dev/null; then
    echo "✅ Whisper 服务运行正常"
    echo "   端点: http://127.0.0.1:5001/transcribe"
    echo ""
    echo "请在 /opt/elon/.env（或 elon-server 的环境变量）中添加："
    echo "  WHISPER_LOCAL_URL=http://127.0.0.1:5001"
else
    echo "❌ Whisper 服务启动失败，查看日志："
    echo "   journalctl -u elon-whisper -n 50"
    exit 1
fi
