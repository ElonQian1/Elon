#!/usr/bin/env pwsh
<#
.SYNOPSIS
    构建并上传 elon-node-agent 可执行文件到服务器。

.DESCRIPTION
    1. 交叉编译 Linux musl 版本（x86_64-unknown-linux-musl）
    2. 编译 Windows 版本
    3. 通过 SCP 上传到服务器 /opt/elon/data/downloads/（与服务端 data_dir 一致）
    4. 验证下载地址可访问

.EXAMPLE
    .\scripts\publish-node-agent.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Server = "root@43.139.149.158"
# data_dir = /opt/elon/data，downloads 子目录与 router.rs 中 state.data_dir.join("downloads") 一致
$RemoteDir = "/opt/elon/data/downloads"
$Bin = "elon-node-agent"

Write-Host "=== elon-node-agent 构建 + 发布 ===" -ForegroundColor Cyan

# 解析真实 target 目录（可能被全局 .cargo/config.toml 的 target-dir 重定向到共享目录）
$ServerDir = Join-Path $PSScriptRoot "..\server"
Push-Location $ServerDir
try {
    $meta = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    $TargetDir = $meta.target_directory
} finally { Pop-Location }
if (-not $TargetDir) { throw "无法解析 cargo target 目录" }
$PackageVersion = ($meta.packages | Where-Object { $_.name -eq "elon-server" } | Select-Object -First 1).version
if (-not $PackageVersion) { throw "无法解析 elon-node-agent 版本号" }
$GitSha = (git -C (Join-Path $PSScriptRoot "..") rev-parse HEAD).Trim()
Write-Host "  target 目录: $TargetDir" -ForegroundColor DarkGray

# ── 1. 交叉编译 Linux musl 版本 ───────────────────────────────────────────────
Write-Host "[1/4] 交叉编译 Linux x86_64-musl..." -ForegroundColor Yellow
Push-Location (Join-Path $PSScriptRoot "..\server")
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
    cargo zigbuild --release --bin $Bin --target x86_64-unknown-linux-musl
    if ($LASTEXITCODE -ne 0) { throw "Linux 编译失败" }
} finally {
    Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
    Pop-Location
}

$LinuxBin = Join-Path $TargetDir "x86_64-unknown-linux-musl\release\$Bin"
if (-not (Test-Path $LinuxBin)) { throw "Linux 二进制不存在：$LinuxBin" }

# ── 2. 编译 Windows 版本 ─────────────────────────────────────────────────────
Write-Host "[2/4] 编译 Windows 版本..." -ForegroundColor Yellow
Push-Location (Join-Path $PSScriptRoot "..\server")
try {
    cargo build --release --bin $Bin
    if ($LASTEXITCODE -ne 0) { throw "Windows 编译失败" }
} finally {
    Pop-Location
}

$WinBin = Join-Path $TargetDir "release\$Bin.exe"
if (-not (Test-Path $WinBin)) { throw "Windows 二进制不存在：$WinBin" }

# ── 3. 上传到服务器 ───────────────────────────────────────────────────────────
Write-Host "[3/4] 上传到服务器..." -ForegroundColor Yellow
ssh -o ProxyCommand=none $Server "mkdir -p $RemoteDir"
scp -o ProxyCommand=none $LinuxBin "${Server}:${RemoteDir}/${Bin}"
ssh -o ProxyCommand=none $Server "chmod +x ${RemoteDir}/${Bin}"
scp -o ProxyCommand=none $WinBin "${Server}:${RemoteDir}/${Bin}.exe"
if ($LASTEXITCODE -ne 0) { throw "上传失败" }

# ── 4. 验证下载地址 ──────────────────────────────────────────────────────────
Write-Host "[4/4] 验证下载地址..." -ForegroundColor Yellow
$BaseUrl = "http://43.139.149.158:8080"
$LinuxDownloadUrl = "$BaseUrl/api/node-agent/download/linux"
$WindowsDownloadUrl = "$BaseUrl/api/node-agent/download/windows"

$size    = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${Bin}"
$sizeWin = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${Bin}.exe"
$VersionInfo = [ordered]@{
    version = $PackageVersion
    gitSha = $GitSha
    updated_at = (Get-Date).ToString("o")
    downloadUrl = $WindowsDownloadUrl
    linuxDownloadUrl = $LinuxDownloadUrl
    fileSize = [int64]$sizeWin
    linuxFileSize = [int64]$size
}
$VersionFile = New-TemporaryFile
try {
    $VersionInfo | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $VersionFile -Encoding UTF8
    scp -o ProxyCommand=none $VersionFile "${Server}:${RemoteDir}/node-agent-version.json"
    if ($LASTEXITCODE -ne 0) { throw "版本信息上传失败" }
} finally {
    Remove-Item -LiteralPath $VersionFile -Force -ErrorAction SilentlyContinue
}
Write-Host "  Linux  $Bin size = $size bytes" -ForegroundColor Green
Write-Host "  Windows $Bin.exe size = $sizeWin bytes" -ForegroundColor Green
Write-Host "  Version info gitSha = $GitSha" -ForegroundColor Green

Write-Host ""
Write-Host "✅ elon-node-agent 发布完成" -ForegroundColor Green
Write-Host "   下载地址（Linux）:   $LinuxDownloadUrl"
Write-Host "   下载地址（Windows）: $WindowsDownloadUrl"
