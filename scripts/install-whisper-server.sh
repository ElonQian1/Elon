#!/bin/bash
# 清代理，下载 Whisper base 模型，启动服务
unset ALL_PROXY all_proxy http_proxy https_proxy HTTP_PROXY HTTPS_PROXY

echo "=== 安装 httpx[socks] ==="
pip3 install --no-cache-dir --index-url https://pypi.org/simple/ "httpx[socks]" -q

echo "=== 下载 Whisper base 模型（使用 hf-mirror.com 国内镜像） ==="
export HF_ENDPOINT=https://hf-mirror.com
python3 - << 'PY'
from faster_whisper import WhisperModel
print("正在下载/验证 base 模型（74MB，首次需要约1分钟）...")
WhisperModel("base", device="cpu", compute_type="int8")
print("模型就绪 OK")
PY

echo "=== 写入 systemd 服务 ==="
cat > /etc/systemd/system/elon-whisper.service << 'UNIT'
[Unit]
Description=Elon Local Whisper ASR Service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/elon
Environment=ALL_PROXY=
Environment=http_proxy=
Environment=https_proxy=
Environment=HF_ENDPOINT=https://hf-mirror.com
ExecStart=/usr/local/bin/uvicorn whisper_service:app --host 127.0.0.1 --port 5001
Restart=always
RestartSec=5
MemoryMax=800M
StandardOutput=append:/var/log/elon-whisper.log
StandardError=append:/var/log/elon-whisper.log

[Install]
WantedBy=multi-user.target
UNIT

echo "=== 启动服务 ==="
systemctl daemon-reload
systemctl enable elon-whisper
systemctl restart elon-whisper
sleep 4

echo "=== 验证健康 ==="
curl -sf http://127.0.0.1:5001/health && echo " Whisper 服务 OK" || { echo "FAIL，查日志："; tail -20 /var/log/elon-whisper.log; exit 1; }

echo "=== 写入 WHISPER_LOCAL_URL 到 /etc/elon-server.env ==="
ENVFILE=/etc/elon-server.env
touch "$ENVFILE"
if grep -q WHISPER_LOCAL_URL "$ENVFILE"; then
    sed -i "s|WHISPER_LOCAL_URL=.*|WHISPER_LOCAL_URL=http://127.0.0.1:5001|" "$ENVFILE"
else
    echo "WHISPER_LOCAL_URL=http://127.0.0.1:5001" >> "$ENVFILE"
fi
echo "已写入 WHISPER_LOCAL_URL"
grep WHISPER_LOCAL_URL "$ENVFILE"
