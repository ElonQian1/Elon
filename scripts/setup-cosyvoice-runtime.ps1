param(
    [string]$InstallRoot = "D:\models\CosyVoice",
    [ValidateSet("huggingface", "modelscope")]
    [string]$DownloadFrom = "modelscope",
    [string]$PythonExe = "",
    [switch]$SkipDependencyInstall,
    [switch]$SkipModelDownload,
    [switch]$ForceRefreshRepo
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param([string]$Title, [scriptblock]$Action)
    Write-Host ""
    Write-Host "== $Title =="
    & $Action
}

function Resolve-Python {
    if ($PythonExe) {
        if (-not (Test-Path -LiteralPath $PythonExe)) {
            throw "PythonExe not found: $PythonExe"
        }
        return $PythonExe
    }
    $venvPython = Join-Path $script:VenvDir "Scripts\python.exe"
    if (-not (Test-Path -LiteralPath $venvPython)) {
        $cmd = Get-Command python -ErrorAction SilentlyContinue
        if (-not $cmd) {
            throw "python not found. Install Python 3.10 or pass -PythonExe."
        }
        python -m venv $script:VenvDir
        if ($LASTEXITCODE -ne 0) {
            throw "python -m venv failed with exit code $LASTEXITCODE"
        }
    }
    if (-not (Test-Path -LiteralPath $venvPython)) {
        throw "python not found. Install Python 3.10 or pass -PythonExe."
    }
    return $venvPython
}

$RepoDir = Join-Path $InstallRoot "CosyVoice"
$ModelDir = Join-Path $RepoDir "pretrained_models\Fun-CosyVoice3-0.5B"
$script:VenvDir = Join-Path $InstallRoot ".venv"
$PipCacheDir = Join-Path $InstallRoot ".pip-cache"
New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
New-Item -ItemType Directory -Force -Path $PipCacheDir | Out-Null
$env:PIP_CACHE_DIR = $PipCacheDir
$Py = Resolve-Python

Invoke-Step "Clone CosyVoice" {
    New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
    if ((Test-Path -LiteralPath $RepoDir) -and $ForceRefreshRepo) {
        $resolved = Resolve-Path -LiteralPath $RepoDir
        if (-not $resolved.Path.StartsWith((Resolve-Path -LiteralPath $InstallRoot).Path)) {
            throw "Refusing to remove unexpected path: $($resolved.Path)"
        }
        Remove-Item -LiteralPath $resolved.Path -Recurse -Force
    }
    if (-not (Test-Path -LiteralPath $RepoDir)) {
        git clone --recursive https://github.com/FunAudioLLM/CosyVoice.git $RepoDir
        if ($LASTEXITCODE -ne 0) {
            throw "git clone CosyVoice failed with exit code $LASTEXITCODE"
        }
    }
    Push-Location $RepoDir
    try {
        git submodule update --init --recursive
    } finally {
        Pop-Location
    }
}

if (-not $SkipDependencyInstall) {
    Invoke-Step "Install CosyVoice dependencies" {
        Push-Location $RepoDir
        try {
            & $Py -m pip install -r requirements.txt -i https://mirrors.aliyun.com/pypi/simple/ --trusted-host=mirrors.aliyun.com
            if ($LASTEXITCODE -ne 0) {
                throw "pip install CosyVoice requirements failed with exit code $LASTEXITCODE"
            }
            & $Py -m pip install fastapi==0.115.6 "uvicorn[standard]==0.34.0"
            if ($LASTEXITCODE -ne 0) {
                throw "pip install worker web dependencies failed with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    }
}

if (-not $SkipModelDownload) {
    Invoke-Step "Download Fun-CosyVoice3 model" {
        Push-Location $RepoDir
        try {
            if ($DownloadFrom -eq "huggingface") {
                & $Py -m pip install huggingface_hub
                $code = "from huggingface_hub import snapshot_download; snapshot_download('FunAudioLLM/Fun-CosyVoice3-0.5B-2512', local_dir=r'pretrained_models/Fun-CosyVoice3-0.5B')"
            } else {
                & $Py -m pip install modelscope
                $code = "from modelscope import snapshot_download; snapshot_download('FunAudioLLM/Fun-CosyVoice3-0.5B-2512', local_dir=r'pretrained_models/Fun-CosyVoice3-0.5B')"
            }
            if ($LASTEXITCODE -ne 0) {
                throw "pip install downloader failed with exit code $LASTEXITCODE"
            }
            & $Py -c $code
            if ($LASTEXITCODE -ne 0) {
                throw "CosyVoice model download failed with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    }
}

Write-Host ""
Write-Host "CosyVoice runtime prepared."
Write-Host "Repo:  $RepoDir"
Write-Host "Model: $ModelDir"
Write-Host "Python:$Py"
Write-Host "Pip cache: $PipCacheDir"
Write-Host ""
Write-Host "Start worker with:"
Write-Host "powershell -ExecutionPolicy Bypass -File scripts\start-local-model-tts-worker.ps1 ``"
Write-Host "  -Provider cosyvoice3 ``"
Write-Host "  -AssetRoot `"D:\tts-assets`" ``"
Write-Host "  -PythonExe `"$Py`" ``"
Write-Host "  -ModelPythonPath `"$RepoDir`" ``"
Write-Host "  -CosyVoiceRepoDir `"$RepoDir`" ``"
Write-Host "  -CosyVoiceModelDir `"$ModelDir`""
