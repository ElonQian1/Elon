param(
    [int]$Port = 5011,
    [string]$Provider = "index_tts2",
    [string]$AssetRoot = "",
    [string]$ModelPythonPath = "",
    [string]$IndexTts2ModelDir = "",
    [string]$IndexTts2CfgPath = "",
    [string]$CosyVoiceRepoDir = "",
    [string]$CosyVoiceModelDir = "",
    [string]$ModelFallbackUrl = "",
    [switch]$SkipInstall
)

$ErrorActionPreference = "Stop"

$RepoRoot = (git -C $PSScriptRoot rev-parse --show-toplevel 2>$null).Trim()
if (-not $RepoRoot) {
    throw "Cannot resolve git repository root."
}

$WorkerDir = Join-Path $RepoRoot "server\tts_worker"
$VenvDir = Join-Path $RepoRoot ".runtime\tts-worker-model\venv"
$PythonExe = Join-Path $VenvDir "Scripts\python.exe"
$Requirements = Join-Path $WorkerDir "requirements-model.txt"
$WorkerFile = Join-Path $WorkerDir "model_tts_worker.py"

if (-not (Test-Path -LiteralPath $WorkerFile)) {
    throw "Missing model worker: $WorkerFile"
}
if (-not $AssetRoot) {
    $AssetRoot = Join-Path $RepoRoot "server\assets\tts"
}

if (-not (Test-Path -LiteralPath $VenvDir)) {
    python -m venv $VenvDir
}

if (-not $SkipInstall) {
    & $PythonExe -m pip install --upgrade pip
    & $PythonExe -m pip install --no-cache-dir -r $Requirements
}

$env:ELON_TTS_WORKER_HOST = "127.0.0.1"
$env:ELON_TTS_WORKER_PORT = "$Port"
$env:ELON_TTS_PROVIDER = $Provider
$env:ELON_TTS_MODEL_PROVIDER = $Provider
$env:ELON_TTS_ASSET_ROOT = $AssetRoot

if ($ModelPythonPath) { $env:ELON_TTS_MODEL_PYTHONPATH = $ModelPythonPath }
if ($IndexTts2ModelDir) { $env:ELON_INDEXTTS2_MODEL_DIR = $IndexTts2ModelDir }
if ($IndexTts2CfgPath) { $env:ELON_INDEXTTS2_CFG_PATH = $IndexTts2CfgPath }
if ($CosyVoiceRepoDir) { $env:ELON_COSYVOICE_REPO_DIR = $CosyVoiceRepoDir }
if ($CosyVoiceModelDir) { $env:ELON_COSYVOICE_MODEL_DIR = $CosyVoiceModelDir }
if ($ModelFallbackUrl) { $env:ELON_TTS_MODEL_FALLBACK_URL = $ModelFallbackUrl }

Write-Host "Starting local model TTS worker..."
Write-Host "  URL:       http://127.0.0.1:$Port"
Write-Host "  Provider:  $Provider"
Write-Host "  AssetRoot: $AssetRoot"
Write-Host ""
Write-Host "Press Ctrl+C to stop."

Push-Location $WorkerDir
try {
    & $PythonExe -m uvicorn model_tts_worker:app --host 127.0.0.1 --port $Port
} finally {
    Pop-Location
}
