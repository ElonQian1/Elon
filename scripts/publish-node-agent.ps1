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

# ── 1. 交叉编译 Linux musl 版本 ───────────────────────────────────────────────
Write-Host "[1/4] 交叉编译 Linux x86_64-musl..." -ForegroundColor Yellow
Push-Location (Join-Path $PSScriptRoot "..\server")
try {
    $env:RUSTFLAGS = "-C target-cpu=x86-64"
    cargo build --release --bin $Bin --target x86_64-unknown-linux-musl
    if ($LASTEXITCODE -ne 0) { throw "Linux 编译失败" }
} finally {
    Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue
    Pop-Location
}

$LinuxBin = Join-Path $PSScriptRoot "..\server\target\x86_64-unknown-linux-musl\release\$Bin"
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

$WinBin = Join-Path $PSScriptRoot "..\server\target\release\$Bin.exe"
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

$size    = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${Bin}"
$sizeWin = ssh -o ProxyCommand=none $Server "stat -c '%s' ${RemoteDir}/${Bin}.exe"
Write-Host "  Linux  $Bin size = $size bytes" -ForegroundColor Green
Write-Host "  Windows $Bin.exe size = $sizeWin bytes" -ForegroundColor Green

Write-Host ""
Write-Host "✅ elon-node-agent 发布完成" -ForegroundColor Green
Write-Host "   下载地址（Linux）:   $BaseUrl/downloads/$Bin"
Write-Host "   下载地址（Windows）: $BaseUrl/downloads/$Bin.exe"
