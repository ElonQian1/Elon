#!/usr/bin/env pwsh
# NOTE: Keep this file encoded as UTF-8 with BOM. Windows PowerShell 5.1 must
# read Chinese launcher filenames correctly.
<#
.SYNOPSIS
    构建并上传一龙 PC 节点客户端可执行文件到服务器。

.DESCRIPTION
    1. 交叉编译 Linux musl 版本（x86_64-unknown-linux-musl）
    2. 编译 Windows 版本
    3. 通过 SCP 上传到服务器 /opt/elon/data/downloads/（与服务端 data_dir 一致）
    4. 验证下载地址可访问

.EXAMPLE
    .\scripts\publish-node-agent.ps1
#>

param(
    [switch]$SkipBroadcast,
    [string]$AdminToken = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Server = "root@43.139.149.158"
$BaseUrl = "http://43.139.149.158:8080"
# data_dir = /opt/elon/data，downloads 子目录与 router.rs 中 state.data_dir.join("downloads") 一致
$RemoteDir = "/opt/elon/data/downloads"
$Bin = "elon-pc-node"
$WindowsClientPackageName = "elon-node-agent-windows.zip"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$ServerDir = Join-Path $RepoRoot "server"
$ServerManifest = Join-Path $ServerDir "Cargo.toml"

Write-Host "=== 一龙 PC 节点客户端构建 + 发布 ===" -ForegroundColor Cyan

function Compress-ArchiveWithRetry {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$DestinationPath,
        [int]$MaxAttempts = 5
    )

    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        try {
            Remove-Item -LiteralPath $DestinationPath -Force -ErrorAction SilentlyContinue
            Compress-Archive -Path $Path -DestinationPath $DestinationPath -Force -ErrorAction Stop
            return
        } catch {
            if ($attempt -ge $MaxAttempts) { throw }
            $delayMs = 500 * $attempt
            Write-Host "  压缩被文件占用中断，${delayMs}ms 后重试 ($attempt/$MaxAttempts)..." -ForegroundColor DarkYellow
            Start-Sleep -Milliseconds $delayMs
        }
    }
}

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Content
    )

    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

function Invoke-GitFetchMain {
    git -C $RepoRoot -c http.proxy= -c https.proxy= fetch origin main
    if ($LASTEXITCODE -ne 0) { throw "git fetch origin main 失败，无法确认 PC 节点发布基线。" }
}

function Assert-NodeAgentPublishHeadCurrent {
    param([Parameter(Mandatory = $true)][string]$Phase)

    Invoke-GitFetchMain
    $headSha = (git -C $RepoRoot rev-parse HEAD).Trim()
    $originMainSha = (git -C $RepoRoot rev-parse origin/main).Trim()
    if ($headSha -ne $originMainSha) {
        throw "$Phase：PC 节点发布停止。当前 HEAD=$($headSha.Substring(0, 7))，origin/main=$($originMainSha.Substring(0, 7))。请先把最新 main 发布，避免用旧 Win 客户端覆盖新主线。"
    }

    return $headSha
}

function Resolve-NodeAgentAdminToken {
    param([string]$ExplicitToken)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitToken)) { return $ExplicitToken }
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_ADMIN_TOKEN)) { return $env:ELON_ADMIN_TOKEN }
    if (-not [string]::IsNullOrWhiteSpace($env:ADMIN_TOKEN)) { return $env:ADMIN_TOKEN }
    return ""
}

function Invoke-NoProxyJson {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [string]$Method = "Get",
        [hashtable]$Headers = @{},
        [string]$Body = "",
        [int]$TimeoutSec = 15
    )

    $irmCommand = Get-Command Invoke-RestMethod -ErrorAction Stop
    $params = @{
        Uri = $Uri
        Method = $Method
        TimeoutSec = $TimeoutSec
    }
    if ($Headers.Count -gt 0) { $params["Headers"] = $Headers }
    if (-not [string]::IsNullOrWhiteSpace($Body)) {
        $params["Body"] = $Body
        $params["ContentType"] = "application/json"
    }
    if ($irmCommand.Parameters.ContainsKey("NoProxy")) {
        $params["NoProxy"] = $true
        return Invoke-RestMethod @params
    }

    $curl = Get-Command "curl.exe" -ErrorAction SilentlyContinue
    if ($curl) {
        $curlArgs = @(
            "--noproxy", "*",
            "--silent", "--show-error", "--fail",
            "--max-time", [string]$TimeoutSec,
            "-X", $Method
        )
        foreach ($key in $Headers.Keys) {
            $curlArgs += @("-H", "${key}: $($Headers[$key])")
        }
        if (-not [string]::IsNullOrWhiteSpace($Body)) {
            $curlArgs += @("-H", "Content-Type: application/json", "--data", $Body)
        }
        $curlArgs += $Uri
        $raw = & $curl.Source @curlArgs
        if ($LASTEXITCODE -ne 0) { throw "curl.exe 请求失败：$Uri" }
        if ([string]::IsNullOrWhiteSpace($raw)) { return $null }
        return $raw | ConvertFrom-Json
    }

    return Invoke-RestMethod @params
}

function Invoke-RemoteBash {
    param([Parameter(Mandatory = $true)][string]$Script)

    $output = $Script | ssh -o ProxyCommand=none $Server "bash -s" 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "服务器远程命令失败($LASTEXITCODE)：$(($output -join "`n").Trim())"
    }
    return ($output -join "`n").Trim()
}

function Test-RemoteNodeAgentAdminToken {
    $script = @'
set -euo pipefail
load_env() {
  if [ -f "$1" ]; then
    set -a
    . "$1" >/dev/null 2>&1 || true
    set +a
  fi
}
load_env /etc/elon-server.env
load_env /root/Elon/server/.env
test -n "${ADMIN_TOKEN:-}"
'@
    $output = $script | ssh -o ProxyCommand=none $Server "bash -s" 2>&1
    if ($LASTEXITCODE -eq 0) { return $true }
    if ($LASTEXITCODE -eq 1) { return $false }
    throw "无法检查服务器 ADMIN_TOKEN：$(($output -join "`n").Trim())"
}

function Invoke-RemoteNodeAgentUpdateBroadcast {
    $script = @'
set -euo pipefail
load_env() {
  if [ -f "$1" ]; then
    set -a
    . "$1" >/dev/null 2>&1 || true
    set +a
  fi
}
load_env /etc/elon-server.env
load_env /root/Elon/server/.env
if [ -z "${ADMIN_TOKEN:-}" ]; then
  echo "ADMIN_TOKEN missing on server" >&2
  exit 2
fi
curl --noproxy '*' -fsS -X POST 'http://127.0.0.1:8080/api/admin/nodes/push-update' \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -H 'Content-Type: application/json' \
  --data '{}'
'@
    $raw = Invoke-RemoteBash -Script $script
    if ([string]::IsNullOrWhiteSpace($raw)) { return $null }
    return $raw | ConvertFrom-Json
}

function Resolve-NodeAgentTargetDir {
    $candidates = @()
    if ($env:ELON_NODE_AGENT_TARGET_DIR) {
        $candidates += $env:ELON_NODE_AGENT_TARGET_DIR
    }
    if ($env:LOCALAPPDATA) {
        $candidates += (Join-Path $env:LOCALAPPDATA "Elon\build-target\elon-node-agent")
    }
    if ($env:PUBLIC) {
        $candidates += (Join-Path $env:PUBLIC "Elon\build-target\elon-node-agent")
    }

    foreach ($candidate in $candidates) {
        if ([string]::IsNullOrWhiteSpace($candidate)) { continue }
        $fullPath = [System.IO.Path]::GetFullPath($candidate)
        if ($fullPath -match "\s") { continue }
        New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
        return $fullPath
    }

    throw "无法解析无空格的 PC 节点 target 目录；请设置 ELON_NODE_AGENT_TARGET_DIR 为无空格路径。"
}

$env:CARGO_TARGET_DIR = Resolve-NodeAgentTargetDir

foreach ($requiredPath in @(
    $ServerManifest,
    (Join-Path $RepoRoot "default-project-docs\files"),
    (Join-Path $RepoRoot "scripts\setup-node-env.ps1"),
    (Join-Path $ServerDir "src\node_agent_admin.html")
)) {
    if (-not (Test-Path -LiteralPath $requiredPath)) {
        throw "发布目录不完整，缺少必要文件：$requiredPath。请在完整仓库 worktree 中运行。"
    }
}

$BroadcastAdminToken = Resolve-NodeAgentAdminToken -ExplicitToken $AdminToken
$UseRemoteAdminToken = $false
if (-not $SkipBroadcast -and [string]::IsNullOrWhiteSpace($BroadcastAdminToken)) {
    if (Test-RemoteNodeAgentAdminToken) {
        $UseRemoteAdminToken = $true
        Write-Host "  本机未设置 ADMIN_TOKEN，将通过服务器本机环境广播更新（token 不回传）。" -ForegroundColor DarkGray
    } else {
        throw "缺少 ADMIN_TOKEN 或 ELON_ADMIN_TOKEN，且服务器环境未找到 ADMIN_TOKEN。若只想上传文件请显式传 -SkipBroadcast。"
    }
}

# 解析真实 target 目录（可能被全局 .cargo/config.toml 的 target-dir 重定向到共享目录）
$meta = cargo metadata --manifest-path $ServerManifest --no-deps --format-version 1 | ConvertFrom-Json
$TargetDir = $meta.target_directory
if (-not $TargetDir) { throw "无法解析 cargo target 目录" }
$PackageVersion = ($meta.packages | Where-Object { $_.name -eq "elon-server" } | Select-Object -First 1).version
if (-not $PackageVersion) { throw "无法解析一龙 PC 节点版本号" }
$GitSha = Assert-NodeAgentPublishHeadCurrent -Phase "发布开始"
Write-Host "  target 目录: $TargetDir" -ForegroundColor DarkGray
Write-Host "  发布基线: origin/main@$($GitSha.Substring(0, 7))" -ForegroundColor DarkGray

# ── 1. 交叉编译 Linux musl 版本 ───────────────────────────────────────────────
Write-Host "[1/5] 交叉编译 Linux x86_64-musl..." -ForegroundColor Yellow
try {
    # musl 交叉编译需要 C 工具链（ring 依赖 gcc）；用 cargo-zigbuild 提供 zig cc，
    # 与 publish-server.ps1 同方案，避免缺少 x86_64-linux-musl-gcc 时编译失败。
    $hasZigbuild = $null -ne (Get-Command "cargo-zigbuild" -ErrorAction SilentlyContinue)
    if (-not $hasZigbuild) { $hasZigbuild = $null -ne (cargo zigbuild --version 2>$null) }
    if (-not $hasZigbuild) {
        Write-Host "  安装 cargo-zigbuild..." -ForegroundColor Yellow
        cargo install cargo-zigbuild
        if ($LASTEXITCODE -ne 0) { throw "cargo-zigbuild 安装失败（需先安装 zig 并加入 PATH）" }
    }
    # 强制通用 CPU，避免全局 target-cpu=native 产出服务器/他人机器无法运行的指令
    $unitSeparator = [char]0x1f
    $env:CARGO_ENCODED_RUSTFLAGS = "-C${unitSeparator}target-cpu=x86-64"
    cargo zigbuild --manifest-path $ServerManifest --release --bin $Bin --target x86_64-unknown-linux-musl
    if ($LASTEXITCODE -ne 0) { throw "Linux 编译失败" }
} finally {
    Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
}

$LinuxBin = Join-Path $TargetDir "x86_64-unknown-linux-musl\release\$Bin"
if (-not (Test-Path $LinuxBin)) { throw "Linux 二进制不存在：$LinuxBin" }

# ── 2. 编译 Windows 版本 ─────────────────────────────────────────────────────
Write-Host "[2/5] 编译 Windows 版本..." -ForegroundColor Yellow
try {
    # 强制通用 CPU，避免全局 target-cpu=native 产出用户机器无法运行的指令。
    $unitSeparator = [char]0x1f
    $env:CARGO_ENCODED_RUSTFLAGS = "-C${unitSeparator}target-cpu=x86-64"
    Write-Host "  Windows release rustflags: -C target-cpu=x86-64" -ForegroundColor DarkGray
    cargo build --manifest-path $ServerManifest --release --bin $Bin
    if ($LASTEXITCODE -ne 0) { throw "Windows 编译失败" }
} finally {
    Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
}

$WinBin = Join-Path $TargetDir "release\$Bin.exe"
if (-not (Test-Path $WinBin)) { throw "Windows 二进制不存在：$WinBin" }

# ── 2.5 打包 Windows 客户端 ──────────────────────────────────────────────────
Write-Host "[2.5/5] 打包 Windows 客户端..." -ForegroundColor Yellow
$LinuxDownloadUrl = "$BaseUrl/api/node-agent/download/linux"
$WindowsDownloadUrl = "$BaseUrl/api/node-agent/download/windows"
$WindowsClientDownloadUrl = "$BaseUrl/api/node-agent/download/windows-client"
$LauncherDir = Join-Path $PSScriptRoot "node-agent-launcher"
$PackageRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-node-agent-windows-" + [Guid]::NewGuid().ToString("N"))
$PackageInternal = Join-Path $PackageRoot "_internal"
$WindowsClientPackage = Join-Path $TargetDir "release\$WindowsClientPackageName"
New-Item -ItemType Directory -Force -Path $PackageRoot, $PackageInternal | Out-Null
try {
    Copy-Item -LiteralPath $WinBin -Destination (Join-Path $PackageRoot "一龙开发平台.exe") -Force
    Copy-Item -LiteralPath $WinBin -Destination (Join-Path $PackageRoot "卸载一龙开发平台.exe") -Force
    Copy-Item -LiteralPath (Join-Path $LauncherDir "node-agent.env.example") -Destination (Join-Path $PackageInternal "node-agent.env.example") -Force
    Copy-Item -LiteralPath (Join-Path $LauncherDir "README.txt") -Destination (Join-Path $PackageInternal "README.txt") -Force
    $PackageVersionInfo = [ordered]@{
        version = $PackageVersion
        gitSha = $GitSha
        updated_at = (Get-Date).ToString("o")
        downloadUrl = $WindowsDownloadUrl
        linuxDownloadUrl = $LinuxDownloadUrl
        windowsClientDownloadUrl = $WindowsClientDownloadUrl
    }
    Write-Utf8NoBom `
        -Path (Join-Path $PackageInternal "node-agent-version.json") `
        -Content ($PackageVersionInfo | ConvertTo-Json -Depth 4)
    Compress-ArchiveWithRetry -Path (Join-Path $PackageRoot "*") -DestinationPath $WindowsClientPackage
} finally {
    Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
}
if (-not (Test-Path $WindowsClientPackage)) { throw "Windows 客户端压缩包不存在：$WindowsClientPackage" }

# ── 3. 上传到服务器 ───────────────────────────────────────────────────────────
Write-Host "[3/5] 上传到服务器..." -ForegroundColor Yellow
Assert-NodeAgentPublishHeadCurrent -Phase "上传前" | Out-Null
ssh -o ProxyCommand=none $Server "mkdir -p $RemoteDir"
scp -o ProxyCommand=none $LinuxBin "${Server}:${RemoteDir}/${Bin}"
ssh -o ProxyCommand=none $Server "chmod +x ${RemoteDir}/${Bin}"
scp -o ProxyCommand=none $WinBin "${Server}:${RemoteDir}/${Bin}.exe"
scp -o ProxyCommand=none $WindowsClientPackage "${Server}:${RemoteDir}/${WindowsClientPackageName}"
if ($LASTEXITCODE -ne 0) { throw "上传失败" }

# ── 4. 验证下载地址 ──────────────────────────────────────────────────────────
Write-Host "[4/5] 验证下载地址..." -ForegroundColor Yellow

$size    = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${Bin}"
$sizeWin = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${Bin}.exe"
$sizeWinClient = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${WindowsClientPackageName}"
$VersionInfo = [ordered]@{
    version = $PackageVersion
    gitSha = $GitSha
    updated_at = (Get-Date).ToString("o")
    downloadUrl = $WindowsDownloadUrl
    linuxDownloadUrl = $LinuxDownloadUrl
    windowsClientDownloadUrl = $WindowsClientDownloadUrl
    fileSize = [int64]$sizeWin
    linuxFileSize = [int64]$size
    windowsClientFileSize = [int64]$sizeWinClient
}
$VersionFile = New-TemporaryFile
try {
    Write-Utf8NoBom -Path $VersionFile -Content ($VersionInfo | ConvertTo-Json -Depth 4)
    scp -o ProxyCommand=none $VersionFile "${Server}:${RemoteDir}/node-agent-version.json"
    if ($LASTEXITCODE -ne 0) { throw "版本信息上传失败" }
} finally {
    Remove-Item -LiteralPath $VersionFile -Force -ErrorAction SilentlyContinue
}
Write-Host "  Linux  $Bin size = $size bytes" -ForegroundColor Green
Write-Host "  Windows $Bin.exe size = $sizeWin bytes" -ForegroundColor Green
Write-Host "  Windows client package size = $sizeWinClient bytes" -ForegroundColor Green
Write-Host "  Version info gitSha = $GitSha" -ForegroundColor Green

# ── 5. 推送在线 Windows 节点更新 ──────────────────────────────────────────────
Write-Host "[5/5] 推送在线 Windows 节点更新..." -ForegroundColor Yellow
if ($SkipBroadcast) {
    Write-Host "  已按 -SkipBroadcast 跳过在线节点推送；离线/重启客户端仍会通过版本接口自动更新。" -ForegroundColor Yellow
} elseif (-not [string]::IsNullOrWhiteSpace($BroadcastAdminToken)) {
    $broadcast = Invoke-NoProxyJson `
        -Uri "$BaseUrl/api/admin/nodes/push-update" `
        -Method "Post" `
        -Headers @{ Authorization = "Bearer $BroadcastAdminToken" } `
        -Body "{}" `
        -TimeoutSec 20
    $broadcastTo = "unknown"
    if ($null -ne $broadcast -and ($broadcast.PSObject.Properties.Name -contains "broadcast_to")) {
        $broadcastTo = [string]$broadcast.broadcast_to
    }
    Write-Host "  已通知在线节点更新：$broadcastTo 个" -ForegroundColor Green
} elseif ($UseRemoteAdminToken) {
    $broadcast = Invoke-RemoteNodeAgentUpdateBroadcast
    $broadcastTo = "unknown"
    if ($null -ne $broadcast -and ($broadcast.PSObject.Properties.Name -contains "broadcast_to")) {
        $broadcastTo = [string]$broadcast.broadcast_to
    }
    Write-Host "  已通过服务器本机通知在线节点更新：$broadcastTo 个" -ForegroundColor Green
} else {
    throw "无法广播在线节点更新：没有可用的本机或服务器 ADMIN_TOKEN。"
}

Write-Host ""
Write-Host "✅ 一龙 PC 节点客户端发布完成" -ForegroundColor Green
Write-Host "   下载地址（Linux）:   $LinuxDownloadUrl"
Write-Host "   下载地址（Windows）: $WindowsDownloadUrl"
Write-Host "   客户端包（Windows）: $WindowsClientDownloadUrl"
