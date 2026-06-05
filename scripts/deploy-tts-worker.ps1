param(
    [string]$Server = "root@43.139.149.158",
    [string]$RemoteRoot = "/root/Elon",
    [int]$Port = 5010,
    [switch]$SkipMainServerRestart
)

$ErrorActionPreference = "Stop"
$SshOpts = @("-o", "ProxyCommand=none")

$gitRoot = git -C $PSScriptRoot rev-parse --show-toplevel 2>$null
if (-not $gitRoot) {
    $gitRoot = git rev-parse --show-toplevel 2>$null
}
if (-not $gitRoot) {
    Write-Error "Cannot resolve git repository root."
}

$RepoRoot = $gitRoot.Trim()
$LocalWorkerDir = Join-Path $RepoRoot "server\tts_worker"
$RemoteWorkerDir = "$RemoteRoot/server/tts_worker"

if (-not (Test-Path (Join-Path $LocalWorkerDir "edge_tts_worker.py"))) {
    Write-Error "Missing local worker files: $LocalWorkerDir"
}

function Invoke-Remote {
    param([string]$Command)
    ssh @SshOpts $Server $Command
    if ($LASTEXITCODE -ne 0) {
        throw "Remote command failed with exit code $LASTEXITCODE"
    }
}

Write-Host "Creating remote worker directory..."
Invoke-Remote "mkdir -p '$RemoteWorkerDir'"

Write-Host "Uploading worker files..."
scp @SshOpts `
    (Join-Path $LocalWorkerDir "edge_tts_worker.py") `
    (Join-Path $LocalWorkerDir "requirements.txt") `
    "${Server}:$RemoteWorkerDir/"
if ($LASTEXITCODE -ne 0) {
    throw "scp worker files failed with exit code $LASTEXITCODE"
}

$restartMain = if ($SkipMainServerRestart) { "0" } else { "1" }
$remoteScript = @"
set -euo pipefail

REMOTE_ROOT='$RemoteRoot'
WORKER_DIR='$RemoteWorkerDir'
PORT='$Port'
RESTART_MAIN='$restartMain'

cd "`$WORKER_DIR"
python3 -m venv .venv
.venv/bin/python -m pip install --upgrade pip
.venv/bin/python -m pip install -r requirements.txt

cat >/etc/systemd/system/elon-tts-worker.service <<UNIT
[Unit]
Description=Elon TTS Worker
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=$RemoteWorkerDir
Environment=ELON_TTS_WORKER_HOST=127.0.0.1
Environment=ELON_TTS_WORKER_PORT=$Port
ExecStart=$RemoteWorkerDir/.venv/bin/python -m uvicorn edge_tts_worker:app --host 127.0.0.1 --port $Port
Restart=on-failure
RestartSec=5
StandardOutput=append:/root/elon-tts-worker.log
StandardError=append:/root/elon-tts-worker.log

[Install]
WantedBy=multi-user.target
UNIT

python3 - <<PY
from pathlib import Path

env_path = Path("$RemoteRoot/server/.env")
updates = {
    "ELON_TTS_WORKER_URL": "http://127.0.0.1:$Port",
    "ELON_TTS_PROVIDER": "auto",
    "ELON_TTS_TIMEOUT_SECS": "120",
    "ELON_TTS_CACHE_ENABLED": "true",
    "ELON_TTS_LLM_REWRITE_ENABLED": "false",
}

text = env_path.read_text(encoding="utf-8") if env_path.exists() else ""
lines = text.splitlines()
seen = set()
out = []
for line in lines:
    stripped = line.strip()
    if not stripped or stripped.startswith("#") or "=" not in line:
        out.append(line)
        continue
    name = line.split("=", 1)[0].strip()
    if name in updates:
        out.append(f"{name}={updates[name]}")
        seen.add(name)
    else:
        out.append(line)

if updates.keys() - seen:
    if out and out[-1].strip():
        out.append("")
    out.append("# TTS Worker")
    for name, value in updates.items():
        if name not in seen:
            out.append(f"{name}={value}")

env_path.write_text("\\n".join(out) + "\\n", encoding="utf-8")
PY

systemctl daemon-reload
systemctl enable --now elon-tts-worker.service
systemctl restart elon-tts-worker.service
sleep 2
curl -fsS "http://127.0.0.1:$Port/health"

if [ "`$RESTART_MAIN" = "1" ]; then
    systemctl restart elon-server.service
    sleep 2
    curl -fsS "http://127.0.0.1:8080/api/voice/tts/catalog"
fi
"@

$tempScript = New-TemporaryFile
try {
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    $remoteScriptLf = $remoteScript -replace "`r`n", "`n" -replace "`r", "`n"
    [System.IO.File]::WriteAllText($tempScript.FullName, $remoteScriptLf, $utf8NoBom)
    scp @SshOpts $tempScript "${Server}:/tmp/deploy-elon-tts-worker.sh"
    if ($LASTEXITCODE -ne 0) {
        throw "scp deploy script failed with exit code $LASTEXITCODE"
    }
    Invoke-Remote "bash /tmp/deploy-elon-tts-worker.sh"
} finally {
    Remove-Item -LiteralPath $tempScript -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "TTS Worker deployment finished."
