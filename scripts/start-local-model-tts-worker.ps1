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
    [string]$UvProjectDir = "",
    [string]$PythonExe = "",
    [switch]$SkipInstall
)

$ErrorActionPreference = "Stop"

$RepoRoot = (git -C $PSScriptRoot rev-parse --show-toplevel 2>$null).Trim()
if (-not $RepoRoot) {
    throw "Cannot resolve git repository root."
}

$WorkerDir = Join-Path $RepoRoot "server\tts_worker"
$VenvDir = Join-Path $RepoRoot ".runtime\tts-worker-model\venv"
$WorkerPythonExe = if ($PythonExe) { $PythonExe } else { Join-Path $VenvDir "Scripts\python.exe" }
$Requirements = Join-Path $WorkerDir "requirements-model.txt"
$WorkerFile = Join-Path $WorkerDir "model_tts_worker.py"

if (-not (Test-Path -LiteralPath $WorkerFile)) {
    throw "Missing model worker: $WorkerFile"
}
if (-not $AssetRoot) {
    $AssetRoot = Join-Path $RepoRoot "server\assets\tts"
}

if ($UvProjectDir) {
    if (-not (Test-Path -LiteralPath $UvProjectDir)) {
        throw "UvProjectDir not found: $UvProjectDir"
    }
} elseif ($PythonExe) {
    if (-not (Test-Path -LiteralPath $PythonExe)) {
        throw "PythonExe not found: $PythonExe"
    }
} else {
    if (-not (Test-Path -LiteralPath $VenvDir)) {
        python -m venv $VenvDir
    }
}

if (-not $SkipInstall -and -not $UvProjectDir) {
    & $WorkerPythonExe -m pip install --upgrade pip
    & $WorkerPythonExe -m pip install --no-cache-dir -r $Requirements
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

$pythonPathParts = @($WorkerDir)
if ($ModelPythonPath) {
    $pythonPathParts += $ModelPythonPath.Split([System.IO.Path]::PathSeparator) |
        Where-Object { $_ -and $_.Trim() }
}
if ($env:PYTHONPATH) {
    $pythonPathParts += $env:PYTHONPATH.Split([System.IO.Path]::PathSeparator) |
        Where-Object { $_ -and $_.Trim() }
}
$env:PYTHONPATH = ($pythonPathParts | Select-Object -Unique) -join [System.IO.Path]::PathSeparator

Write-Host "Starting local model TTS worker..."
Write-Host "  URL:       http://127.0.0.1:$Port"
Write-Host "  Provider:  $Provider"
Write-Host "  AssetRoot: $AssetRoot"
if ($UvProjectDir) { Write-Host "  UV project: $UvProjectDir" }
if ($PythonExe) { Write-Host "  PythonExe:  $PythonExe" }
Write-Host ""
Write-Host "Press Ctrl+C to stop."

if ($UvProjectDir) {
    $uvArgs = @(
        "run",
        "--project", $UvProjectDir,
        "--with", "fastapi==0.115.6",
        "--with", "uvicorn[standard]==0.34.0",
        "python", "-m", "uvicorn", "model_tts_worker:app",
        "--host", "127.0.0.1",
        "--port", "$Port"
    )
    Push-Location $UvProjectDir
    try {
        if (Get-Command uv -ErrorAction SilentlyContinue) {
            & uv @uvArgs
        } else {
            & python -m uv @uvArgs
        }
    } finally {
        Pop-Location
    }
} else {
    Push-Location $WorkerDir
    try {
        & $WorkerPythonExe -m uvicorn model_tts_worker:app --host 127.0.0.1 --port $Port
    } finally {
        Pop-Location
    }
}
