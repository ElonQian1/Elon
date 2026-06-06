param(
    [string]$Server = "root@43.139.149.158",
    [string]$RemoteRoot = "/root/Elon",
    [int]$Port = 5011,
    [string]$Provider = "index_tts2",
    [string]$RemoteAssetRoot = "",
    [string]$ModelPythonPath = "",
    [string]$IndexTts2ModelDir = "",
    [string]$IndexTts2CfgPath = "",
    [string]$CosyVoiceRepoDir = "",
    [string]$CosyVoiceModelDir = "",
    [string]$ModelFallbackUrl = "",
    [switch]$SkipMainServerUpdate,
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
$RemoteVenvDir = "$RemoteRoot/.runtime/tts-worker-model/venv"
if (-not $RemoteAssetRoot) {
    $RemoteAssetRoot = "$RemoteRoot/server/assets/tts"
}

foreach ($file in @("model_tts_worker.py", "requirements-model.txt")) {
    if (-not (Test-Path (Join-Path $LocalWorkerDir $file))) {
        Write-Error "Missing local worker file: $file"
    }
}

function Invoke-Remote {
    param([string]$Command)
    ssh @SshOpts $Server $Command
    if ($LASTEXITCODE -ne 0) {
        throw "Remote command failed with exit code $LASTEXITCODE"
    }
}

function Add-SystemdEnv {
    param([string]$Name, [string]$Value)
    if ($Value) {
        return "Environment=$Name=$Value"
    }
    return $null
}

$envLines = @(
    "Environment=ELON_TTS_WORKER_HOST=127.0.0.1",
    "Environment=ELON_TTS_WORKER_PORT=$Port",
    "Environment=ELON_TTS_PROVIDER=$Provider",
    "Environment=ELON_TTS_MODEL_PROVIDER=$Provider",
    "Environment=ELON_TTS_ASSET_ROOT=$RemoteAssetRoot",
    (Add-SystemdEnv "ELON_TTS_MODEL_PYTHONPATH" $ModelPythonPath),
    (Add-SystemdEnv "ELON_INDEXTTS2_MODEL_DIR" $IndexTts2ModelDir),
    (Add-SystemdEnv "ELON_INDEXTTS2_CFG_PATH" $IndexTts2CfgPath),
    (Add-SystemdEnv "ELON_COSYVOICE_REPO_DIR" $CosyVoiceRepoDir),
    (Add-SystemdEnv "ELON_COSYVOICE_MODEL_DIR" $CosyVoiceModelDir),
    (Add-SystemdEnv "ELON_TTS_MODEL_FALLBACK_URL" $ModelFallbackUrl)
) | Where-Object { $_ }
$systemdEnv = ($envLines -join "`n")

Write-Host "Creating remote worker directory..."
Invoke-Remote "mkdir -p '$RemoteWorkerDir'"

Write-Host "Uploading model worker files..."
scp @SshOpts `
    (Join-Path $LocalWorkerDir "model_tts_worker.py") `
    (Join-Path $LocalWorkerDir "requirements-model.txt") `
    "${Server}:$RemoteWorkerDir/"
if ($LASTEXITCODE -ne 0) {
    throw "scp worker files failed with exit code $LASTEXITCODE"
}

$updateMain = if ($SkipMainServerUpdate) { "0" } else { "1" }
$restartMain = if ($SkipMainServerRestart) { "0" } else { "1" }
$remoteScript = @"
set -euo pipefail

REMOTE_ROOT='$RemoteRoot'
WORKER_DIR='$RemoteWorkerDir'
VENV_DIR='$RemoteVenvDir'
PORT='$Port'
PROVIDER='$Provider'
UPDATE_MAIN='$updateMain'
RESTART_MAIN='$restartMain'

cd "`$WORKER_DIR"
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
export NO_PROXY='127.0.0.1,localhost'
export no_proxy='127.0.0.1,localhost'
export PIP_CONFIG_FILE=/dev/null
mkdir -p "`$(dirname "`$VENV_DIR")"
python3 -m venv "`$VENV_DIR"
"`$VENV_DIR/bin/python" -m pip install --upgrade pip --index-url http://mirrors.tencentyun.com/pypi/simple --trusted-host mirrors.tencentyun.com
"`$VENV_DIR/bin/python" -m pip install --no-cache-dir --index-url http://mirrors.tencentyun.com/pypi/simple --trusted-host mirrors.tencentyun.com -r requirements-model.txt

cat >/etc/systemd/system/elon-model-tts-worker.service <<UNIT
[Unit]
Description=Elon Model TTS Worker
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=$RemoteWorkerDir
$systemdEnv
ExecStart=$RemoteVenvDir/bin/python -m uvicorn model_tts_worker:app --host 127.0.0.1 --port $Port
Restart=on-failure
RestartSec=5
StandardOutput=append:/root/elon-model-tts-worker.log
StandardError=append:/root/elon-model-tts-worker.log

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable --now elon-model-tts-worker.service
systemctl restart elon-model-tts-worker.service
sleep 2
curl -fsS "http://127.0.0.1:$Port/health"

if [ "`$UPDATE_MAIN" = "1" ]; then
    python3 - <<PY
from pathlib import Path

env_path = Path("$RemoteRoot/server/.env")
updates = {
    "ELON_TTS_WORKER_URL": "http://127.0.0.1:$Port",
    "ELON_TTS_PROVIDER": "$Provider",
    "ELON_TTS_TIMEOUT_SECS": "180",
    "ELON_TTS_CACHE_ENABLED": "true",
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
    out.append("# Model TTS Worker")
    for name, value in updates.items():
        if name not in seen:
            out.append(f"{name}={value}")

env_path.write_text("\\n".join(out) + "\\n", encoding="utf-8")
PY

    if [ "`$RESTART_MAIN" = "1" ]; then
        systemctl restart elon-server.service
        sleep 2
        curl -fsS "http://127.0.0.1:8080/api/voice/tts/catalog"
    fi
fi
"@

$tempScript = New-TemporaryFile
try {
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    $remoteScriptLf = $remoteScript -replace "`r`n", "`n" -replace "`r", "`n"
    [System.IO.File]::WriteAllText($tempScript.FullName, $remoteScriptLf, $utf8NoBom)
    scp @SshOpts $tempScript "${Server}:/tmp/deploy-elon-model-tts-worker.sh"
    if ($LASTEXITCODE -ne 0) {
        throw "scp deploy script failed with exit code $LASTEXITCODE"
    }
    Invoke-Remote "bash /tmp/deploy-elon-model-tts-worker.sh"
} finally {
    Remove-Item -LiteralPath $tempScript -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Model TTS Worker deployment finished."
