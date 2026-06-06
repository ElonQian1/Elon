param(
    [string]$InstallRoot = "D:\models\IndexTTS2",
    [ValidateSet("huggingface", "modelscope")]
    [string]$DownloadFrom = "huggingface",
    [string]$PypiMirror = "https://mirrors.tuna.tsinghua.edu.cn/pypi/web/simple",
    [switch]$SkipDependencyInstall,
    [switch]$SkipModelDownload,
    [switch]$PullLfsExamples,
    [switch]$UseFp16,
    [switch]$ForceRefreshRepo
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Title,
        [scriptblock]$Action
    )
    Write-Host ""
    Write-Host "== $Title =="
    & $Action
}

function Require-Command {
    param([string]$Name, [string]$Hint)
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $cmd) {
        throw "$Name not found. $Hint"
    }
    return $cmd.Source
}

function Invoke-Uv {
    param([string[]]$Arguments)
    if (Get-Command uv -ErrorAction SilentlyContinue) {
        & uv @Arguments
    } else {
        & python -m uv @Arguments
    }
    if ($LASTEXITCODE -ne 0) {
        throw "uv failed with exit code $LASTEXITCODE"
    }
}

$RepoDir = Join-Path $InstallRoot "index-tts"
$CheckpointDir = Join-Path $RepoDir "checkpoints"

Invoke-Step "Prerequisites" {
    Require-Command git "Install Git first." | Out-Null
    if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
        Write-Host "uv command not found in PATH; installing uv into the current Python user environment..."
        & python -m pip install -U uv
        if ($LASTEXITCODE -ne 0) {
            throw "pip install uv failed with exit code $LASTEXITCODE"
        }
    }
    git lfs version | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "git-lfs is required by IndexTTS2. Install Git LFS, then rerun this script."
    }
}

Invoke-Step "Clone IndexTTS2" {
    New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
    if ((Test-Path -LiteralPath $RepoDir) -and $ForceRefreshRepo) {
        $resolved = Resolve-Path -LiteralPath $RepoDir
        if (-not $resolved.Path.StartsWith((Resolve-Path -LiteralPath $InstallRoot).Path)) {
            throw "Refusing to remove unexpected path: $($resolved.Path)"
        }
        Remove-Item -LiteralPath $resolved.Path -Recurse -Force
    }
    if (-not (Test-Path -LiteralPath $RepoDir)) {
        $oldSkipSmudge = $env:GIT_LFS_SKIP_SMUDGE
        $env:GIT_LFS_SKIP_SMUDGE = "1"
        git clone https://github.com/index-tts/index-tts.git $RepoDir
        if ($null -eq $oldSkipSmudge) {
            Remove-Item Env:\GIT_LFS_SKIP_SMUDGE -ErrorAction SilentlyContinue
        } else {
            $env:GIT_LFS_SKIP_SMUDGE = $oldSkipSmudge
        }
        if ($LASTEXITCODE -ne 0) {
            throw "git clone IndexTTS2 failed with exit code $LASTEXITCODE"
        }
    }
    Push-Location $RepoDir
    try {
        git lfs install
        if ($PullLfsExamples) {
            git lfs pull
        } else {
            Write-Host "Skipping GitHub LFS example assets. Use -PullLfsExamples only if the official repo LFS quota is available."
        }
    } finally {
        Pop-Location
    }
}

if (-not $SkipDependencyInstall) {
    Invoke-Step "Install IndexTTS2 dependencies" {
        Push-Location $RepoDir
        try {
            $syncArgs = @("sync", "--extra", "webui", "--default-index", $PypiMirror)
            Invoke-Uv $syncArgs
        } finally {
            Pop-Location
        }
    }
}

if (-not $SkipModelDownload) {
    Invoke-Step "Download IndexTTS2 checkpoints" {
        Push-Location $RepoDir
        try {
            if ($DownloadFrom -eq "modelscope") {
                $code = "from modelscope import snapshot_download; snapshot_download('IndexTeam/IndexTTS-2', local_dir='checkpoints')"
                Invoke-Uv @("run", "--with", "modelscope", "python", "-c", $code)
            } else {
                $code = "from huggingface_hub import snapshot_download; snapshot_download(repo_id='IndexTeam/IndexTTS-2', local_dir='checkpoints')"
                Invoke-Uv @("run", "--with", "huggingface-hub[hf_xet]", "python", "-c", $code)
            }
        } finally {
            Pop-Location
        }
    }
}

$cfgPath = Join-Path $CheckpointDir "config.yaml"
Write-Host ""
Write-Host "IndexTTS2 runtime prepared."
Write-Host "Repo:       $RepoDir"
Write-Host "Checkpoints:$CheckpointDir"
Write-Host "Config:     $cfgPath"
Write-Host ""
Write-Host "Start worker with:"
$fp16Flag = if ($UseFp16) { " `$`n  `$env:ELON_INDEXTTS2_USE_FP16='1';" } else { "" }
Write-Host "$fp16Flag powershell -ExecutionPolicy Bypass -File scripts\start-local-model-tts-worker.ps1 ``"
Write-Host "  -Provider index_tts2 ``"
Write-Host "  -AssetRoot `"D:\tts-assets`" ``"
Write-Host "  -UvProjectDir `"$RepoDir`" ``"
Write-Host "  -ModelPythonPath `"$RepoDir`" ``"
Write-Host "  -IndexTts2ModelDir `"$CheckpointDir`" ``"
Write-Host "  -IndexTts2CfgPath `"$cfgPath`""
