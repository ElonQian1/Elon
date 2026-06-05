param(
    [string[]]$SearchRoots = @(
        "D:\rust",
        "$env:USERPROFILE\Downloads",
        "$env:USERPROFILE\Documents",
        "$env:USERPROFILE\source",
        "D:\BaiduNetdiskDownload",
        "D:\opt"
    )
)

$ErrorActionPreference = "Continue"

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
Write-Output "== Source directories =="
$pattern = "index.?tts|cosyvoice|gpt.?sovits|so-vits|sovits|kokoro|sherpa"
foreach ($root in $SearchRoots) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $matches = Get-ChildItem -LiteralPath $root -Directory -Recurse -Depth 4 -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match $pattern } |
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
