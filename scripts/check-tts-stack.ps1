param(
    [string]$AssetRoot = "",
    [string]$ModelRoot = "D:\models",
    [string]$IndexTts2ProjectDir = "",
    [string]$CosyVoicePythonExe = "",
    [string[]]$SearchRoots = @(
        "D:\models",
        "D:\rust",
        "$env:USERPROFILE\Downloads",
        "$env:USERPROFILE\Documents",
        "$env:USERPROFILE\source",
        "D:\BaiduNetdiskDownload",
        "D:\opt"
    )
)

$ErrorActionPreference = "Continue"

if ($ModelRoot -and (Test-Path -LiteralPath $ModelRoot)) {
    $SearchRoots = @($ModelRoot) + $SearchRoots
}
$SearchRoots = $SearchRoots | Where-Object { $_ -and $_.Trim() } | Select-Object -Unique

if (-not $IndexTts2ProjectDir -and $ModelRoot) {
    $IndexTts2ProjectDir = Join-Path $ModelRoot "IndexTTS2\index-tts"
}

if (-not $CosyVoicePythonExe -and $ModelRoot) {
    $CosyVoicePythonExe = Join-Path $ModelRoot "CosyVoice\.venv\Scripts\python.exe"
}

function Write-ItemStatus {
    param(
        [string]$Name,
        [bool]$Ok,
        [string]$Detail = ""
    )
    $flag = if ($Ok) { "OK" } else { "MISS" }
    if ($Detail) {
        Write-Output "$flag`t$Name`t$Detail"
    } else {
        Write-Output "$flag`t$Name"
    }
}

Write-Output "== Environment =="
Write-ItemStatus "ELON_TTS_WORKER_URL" ([bool]$env:ELON_TTS_WORKER_URL) $env:ELON_TTS_WORKER_URL
Write-ItemStatus "ELON_TTS_PROVIDER" ([bool]$env:ELON_TTS_PROVIDER) $env:ELON_TTS_PROVIDER
Write-ItemStatus "ELON_TTS_ASSET_ROOT" ([bool]$env:ELON_TTS_ASSET_ROOT) $env:ELON_TTS_ASSET_ROOT
Write-ItemStatus "ELON_TTS_MODEL_PYTHONPATH" ([bool]$env:ELON_TTS_MODEL_PYTHONPATH) $env:ELON_TTS_MODEL_PYTHONPATH
Write-ItemStatus "ELON_INDEXTTS2_MODEL_DIR" ([bool]$env:ELON_INDEXTTS2_MODEL_DIR) $env:ELON_INDEXTTS2_MODEL_DIR
Write-ItemStatus "ELON_INDEXTTS2_CFG_PATH" ([bool]$env:ELON_INDEXTTS2_CFG_PATH) $env:ELON_INDEXTTS2_CFG_PATH
Write-ItemStatus "ELON_COSYVOICE_REPO_DIR" ([bool]$env:ELON_COSYVOICE_REPO_DIR) $env:ELON_COSYVOICE_REPO_DIR
Write-ItemStatus "ELON_COSYVOICE_MODEL_DIR" ([bool]$env:ELON_COSYVOICE_MODEL_DIR) $env:ELON_COSYVOICE_MODEL_DIR

Write-Output ""
Write-Output "== Python =="
$python = Get-Command python -ErrorAction SilentlyContinue
Write-ItemStatus "python" ($null -ne $python) $(if ($python) { $python.Source } else { "" })
if ($python) {
    & python --version
}

$pip = Get-Command pip -ErrorAction SilentlyContinue
Write-ItemStatus "pip" ($null -ne $pip) $(if ($pip) { $pip.Source } else { "" })

$conda = Get-Command conda -ErrorAction SilentlyContinue
Write-ItemStatus "conda" ($null -ne $conda) $(if ($conda) { $conda.Source } else { "" })

$uv = Get-Command uv -ErrorAction SilentlyContinue
Write-ItemStatus "uv" ($null -ne $uv) $(if ($uv) { $uv.Source } else { "" })
if (-not $uv -and $python) {
    $uvModule = & python -m uv --version 2>$null
    Write-ItemStatus "python -m uv" ([bool]$uvModule) $(if ($uvModule) { $uvModule } else { "" })
}

$git = Get-Command git -ErrorAction SilentlyContinue
Write-ItemStatus "git" ($null -ne $git) $(if ($git) { $git.Source } else { "" })
if ($git) {
    $gitLfs = & git lfs version 2>$null
    Write-ItemStatus "git-lfs" ([bool]$gitLfs) $(if ($gitLfs) { $gitLfs } else { "" })
}

$nvidiaSmi = Get-Command nvidia-smi -ErrorAction SilentlyContinue
Write-ItemStatus "nvidia-smi" ($null -ne $nvidiaSmi) $(if ($nvidiaSmi) { $nvidiaSmi.Source } else { "" })
if ($nvidiaSmi) {
    & nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader 2>$null |
        ForEach-Object { Write-ItemStatus "gpu" $true $_ }
}

Write-Output ""
Write-Output "== Python packages =="
$packages = @(
    "indextts",
    "index-tts",
    "cosyvoice",
    "gpt-sovits",
    "GPT-SoVITS",
    "edge-tts",
    "fastapi",
    "uvicorn",
    "kokoro",
    "sherpa-onnx",
    "torch",
    "torchaudio",
    "modelscope",
    "funasr"
)
if ($python) {
    foreach ($pkg in $packages) {
        $show = & python -m pip show $pkg 2>$null
        Write-ItemStatus $pkg ([bool]$show) $(if ($show) { ($show | Select-Object -First 1) } else { "" })
    }
}

Write-Output ""
Write-Output "== Model runtimes =="
Write-ItemStatus "IndexTTS2 project" ((Test-Path -LiteralPath $IndexTts2ProjectDir)) $IndexTts2ProjectDir
if (Test-Path -LiteralPath $IndexTts2ProjectDir) {
    Push-Location $IndexTts2ProjectDir
    try {
        $indexStatus = if (Get-Command uv -ErrorAction SilentlyContinue) {
            & uv run python -c "import torch, indextts.infer_v2; print('torch=' + torch.__version__ + ';cuda=' + str(torch.cuda.is_available()))" 2>&1
        } elseif ($python) {
            & python -m uv run python -c "import torch, indextts.infer_v2; print('torch=' + torch.__version__ + ';cuda=' + str(torch.cuda.is_available()))" 2>&1
        } else {
            @()
        }
        Write-ItemStatus "IndexTTS2 import" ($LASTEXITCODE -eq 0 -and [bool]$indexStatus) $(($indexStatus | Select-Object -First 1) -join "")
    } finally {
        Pop-Location
    }
}

Write-ItemStatus "CosyVoice python" ((Test-Path -LiteralPath $CosyVoicePythonExe)) $CosyVoicePythonExe
if (Test-Path -LiteralPath $CosyVoicePythonExe) {
    $cosyStatus = & $CosyVoicePythonExe -c "import importlib.util; print(importlib.util.find_spec('cosyvoice.cli.cosyvoice') is not None)" 2>&1
    Write-ItemStatus "CosyVoice import" ($LASTEXITCODE -eq 0 -and (($cosyStatus | Select-Object -First 1) -eq "True")) $(($cosyStatus | Select-Object -First 1) -join "")
}

Write-Output ""
Write-Output "== Source directories =="
$pattern = "index.?tts|cosyvoice|gpt.?sovits|so-vits|sovits|kokoro|sherpa"
foreach ($root in $SearchRoots) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $matches = Get-ChildItem -LiteralPath $root -Directory -Recurse -Depth 4 -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match $pattern } |
        Select-Object -First 20
    if ($matches) {
        foreach ($m in $matches) {
            Write-ItemStatus "source" $true $m.FullName
        }
    }
}

Write-Output ""
Write-Output "== Docker images =="
$docker = Get-Command docker -ErrorAction SilentlyContinue
Write-ItemStatus "docker" ($null -ne $docker) $(if ($docker) { $docker.Source } else { "" })
if ($docker) {
    docker images --format "{{.Repository}}:{{.Tag}}" 2>$null |
        Select-String -Pattern "index.?tts|cosyvoice|gpt.?sovits|kokoro|sherpa" |
        ForEach-Object { Write-ItemStatus "docker-image" $true $_.Line }
}

Write-Output ""
Write-Output "== TTS assets =="
$repoRoot = git -C $PSScriptRoot rev-parse --show-toplevel 2>$null
if ($AssetRoot) {
    $assetRoot = $AssetRoot
} elseif ($env:ELON_TTS_ASSET_ROOT) {
    $assetRoot = $env:ELON_TTS_ASSET_ROOT
} elseif ($repoRoot) {
    $assetRoot = Join-Path $repoRoot.Trim() "server\assets\tts"
} else {
    $assetRoot = ""
}
Write-ItemStatus "asset-root" ([bool]$assetRoot -and (Test-Path -LiteralPath $assetRoot)) $assetRoot

$voiceAssets = @(
    "voices/female_warm_neutral.wav",
    "voices/female_bright_neutral.wav",
    "voices/female_mature_neutral.wav",
    "voices/female_cool_neutral.wav",
    "voices/female_sweet_neutral.wav"
)
$emotionAssets = @(
    "emotions/female_neutral.wav",
    "emotions/female_gentle_comfort.wav",
    "emotions/female_crying_broken.wav",
    "emotions/female_happy_soft.wav",
    "emotions/female_happy_excited.wav",
    "emotions/female_angry_repressed.wav",
    "emotions/female_cool_detached.wav",
    "emotions/female_shy_nervous.wav",
    "emotions/female_sad_low.wav",
    "emotions/female_surprised.wav",
    "emotions/female_serious_encourage.wav",
    "emotions/female_whisper.wav"
)
if ($assetRoot) {
    foreach ($asset in $voiceAssets) {
        $path = Join-Path $assetRoot $asset
        Write-ItemStatus "voice-asset" (Test-Path -LiteralPath $path) $asset
    }
    foreach ($asset in $emotionAssets) {
        $path = Join-Path $assetRoot $asset
        Write-ItemStatus "emotion-asset" (Test-Path -LiteralPath $path) $asset
    }
}
