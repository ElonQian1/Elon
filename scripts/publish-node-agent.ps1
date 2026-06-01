#!/usr/bin/env pwsh
<#
.SYNOPSIS
    构建并上传 elon-node-agent 可执行文件到服务器。

.DESCRIPTION
    1. 在服务器上编译 Linux 版本（SSH 远程 cargo build，无需本机 musl 工具链）
    2. 在本机编译 Windows 版本
    3. 上传到服务器 /opt/elon/data/downloads/（与服务端 data_dir 一致）
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
$Repo = "https://github.com/ElonQian1/Elon.git"

Write-Host "=== elon-node-agent 构建 + 发布 ===" -ForegroundColor Cyan

# ── 1. 在服务器上编译 Linux 版本 ─────────────────────────────────────────────
Write-Host "[1/4] 在服务器上编译 Linux 版本..." -ForegroundColor Yellow
$GitSha = (git rev-parse HEAD).Trim()
$RemoteBuild = @"
set -e
mkdir -p $RemoteDir
# 清理旧的 clone（若存在）
rm -rf /tmp/elon-node-build
git clone --depth 1 https://github.com/ElonQian1/Elon.git /tmp/elon-node-build
cd /tmp/elon-node-build/server
# 查找 cargo target（可能在 ~/.cache/elon/rust-target 或 target/）
export CARGO_TARGET_DIR=\$(cat .cargo/config.toml 2>/dev/null | grep 'target-dir' | sed 's/.*= *//' | tr -d '"' || echo 'target')
[ -z "\$CARGO_TARGET_DIR" ] && CARGO_TARGET_DIR="target"
RUSTFLAGS="-C target-cpu=x86-64" cargo build --release --bin $Bin
# 查找产物
BINARY=\$(find /root/.cache/elon/rust-target/release /tmp/elon-node-build/server/target/release -maxdepth 1 -name "$Bin" -type f 2>/dev/null | head -1)
if [ -z "\$BINARY" ]; then echo "ERROR: 找不到编译产物"; exit 1; fi
cp "\$BINARY" $RemoteDir/$Bin
chmod +x $RemoteDir/$Bin
rm -rf /tmp/elon-node-build
echo "Linux build OK: \$(stat -c '%s' $RemoteDir/$Bin) bytes"
"@

ssh -o ProxyCommand=none $Server $RemoteBuild
if ($LASTEXITCODE -ne 0) { throw "Linux 编译/上传失败" }

# ── 2. 在本机编译 Windows 版本 ───────────────────────────────────────────────
Write-Host "[2/4] 编译 Windows 版本..." -ForegroundColor Yellow
Push-Location (Join-Path $PSScriptRoot "..\server")
try {
    cargo build --release --bin $Bin --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Windows 编译失败" }
} finally {
    Pop-Location
}

# 查找 Windows exe（共享 target 或本地 target）
$WinBin = Get-ChildItem -Recurse -Filter "$Bin.exe" "d:\rust" -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "x86_64-pc-windows-msvc.release" } |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $WinBin) {
    $WinBin = Get-ChildItem -Recurse -Filter "$Bin.exe" (Join-Path $PSScriptRoot "..") -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "release" -and $_.FullName -notmatch "debug" } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $WinBin) { throw "Windows 二进制不存在，请检查 cargo build 产物路径" }
Write-Host "   Windows 二进制：$WinBin" -ForegroundColor DarkGray

# ── 3. 上传 Windows exe ───────────────────────────────────────────────────────
Write-Host "[3/4] 上传 Windows 二进制..." -ForegroundColor Yellow
ssh -o ProxyCommand=none $Server "mkdir -p $RemoteDir"
scp -o ProxyCommand=none $WinBin "${Server}:${RemoteDir}/${Bin}.exe"
if ($LASTEXITCODE -ne 0) { throw "Windows exe 上传失败" }

# ── 4. 验证下载地址 ──────────────────────────────────────────────────────────
Write-Host "[4/4] 验证下载地址..." -ForegroundColor Yellow
$BaseUrl = "http://43.139.149.158:8080"

$linuxStatus = (Invoke-WebRequest -Uri "$BaseUrl/downloads/$Bin" -Method Head -UseBasicParsing -ErrorAction SilentlyContinue).StatusCode
$winStatus   = (Invoke-WebRequest -Uri "$BaseUrl/downloads/$Bin.exe" -Method Head -UseBasicParsing -ErrorAction SilentlyContinue).StatusCode
Write-Host "  Linux  $BaseUrl/downloads/$Bin  → HTTP $linuxStatus"
Write-Host "  Windows $BaseUrl/downloads/$Bin.exe → HTTP $winStatus"

if ($linuxStatus -ne 200 -or $winStatus -ne 200) {
    Write-Host "⚠️  下载地址返回非 200，请检查服务端 /downloads/ 路由是否已部署" -ForegroundColor Yellow
} else {
    Write-Host ""
    Write-Host "✅ elon-node-agent 发布完成" -ForegroundColor Green
    Write-Host "   下载地址（Linux）:   $BaseUrl/downloads/$Bin"
    Write-Host "   下载地址（Windows）: $BaseUrl/downloads/$Bin.exe"
}
