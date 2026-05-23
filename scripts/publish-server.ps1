<#
.SYNOPSIS
    elon cli 服务端 — 本地交叉编译 → 部署
.DESCRIPTION
    本地用 cargo zigbuild 交叉编译到 x86_64-unknown-linux-musl，
    通过 git worktree 临时隔离（不把未提交改动带到服务器），
    scp 上传到服务器后重启并验证。

    依赖（首次运行前手动安装一次即可）：
      1. zig 工具链：https://ziglang.org/download/  → 解压后加入 PATH
      2. cargo-zigbuild：cargo install cargo-zigbuild
      3. musl target：rustup target add x86_64-unknown-linux-musl
      4. OpenSSH 客户端（Windows 11 内置）

.PARAMETER SkipBuild
    跳过编译，用上次已有的产物直接重新部署（仅上传 + 重启）。
.PARAMETER SkipUpload
    只做本地编译，不上传不重启（用于本地验证 binary）。
.PARAMETER Force
    强制重新部署，即使检测到服务器上已是相同 SHA。

.EXAMPLE
    .\scripts\publish-server.ps1                     # 正常流程
    .\scripts\publish-server.ps1 -SkipBuild          # 只用上次产物重新部署
    .\scripts\publish-server.ps1 -SkipUpload         # 只本地编译，不部署
#>
param(
    [switch]$SkipBuild,
    [switch]$SkipUpload,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# ─────────────────────────────────────────────────────────────
# 配置（修改这里以适应不同服务器）
# ─────────────────────────────────────────────────────────────
$Target      = "x86_64-unknown-linux-musl"
$Server      = "root@43.139.149.158"
$RemoteDir   = "/root/Elon"
$RemoteBin   = "$RemoteDir/server/target/release/elon-server"
$SshOpts     = @("-o", "ProxyCommand=none")  # 绕过本地 VPN 代理

# ─────────────────────────────────────────────────────────────
# 路径推导（基于 git 仓库根，兼容任意 PC、任意路径）
# ─────────────────────────────────────────────────────────────
# 先尝试从脚本所在目录解析仓库根，再 fallback 到当前目录
$gitRoot = git -C $PSScriptRoot rev-parse --show-toplevel 2>$null
if (-not $gitRoot) {
    $gitRoot = git rev-parse --show-toplevel 2>$null
}
if (-not $gitRoot) {
    Write-Error "❌ 当前目录不在 git 仓库中，请从仓库根或 scripts/ 目录运行本脚本。"
}
$RepoRoot  = $gitRoot.Trim()
$ServerDir = Join-Path $RepoRoot "server"

if (-not (Test-Path (Join-Path $ServerDir "Cargo.toml"))) {
    Write-Error "❌ 找不到 $ServerDir/Cargo.toml，请确认仓库结构。"
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "   elon cli 服务端  交叉编译 + 部署" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  仓库根: $RepoRoot" -ForegroundColor Gray
Write-Host "  目标:   $Target" -ForegroundColor Gray
Write-Host "  服务器: $Server" -ForegroundColor Gray
Write-Host ""

# ─────────────────────────────────────────────────────────────
# 1. git pull --rebase（避免 push 时冲突）
# ─────────────────────────────────────────────────────────────
Write-Host "1⃣  同步最新代码..." -ForegroundColor Yellow
git -C $RepoRoot pull --rebase origin main
if ($LASTEXITCODE -ne 0) { Write-Error "git pull --rebase 失败" }

$Sha      = (git -C $RepoRoot rev-parse --short HEAD).Trim()
$ShaBig   = (git -C $RepoRoot rev-parse HEAD).Trim()
Write-Host "   ✅ 最新 SHA: $Sha" -ForegroundColor Green

# ─────────────────────────────────────────────────────────────
# 2. 环境检查（仅 Build 时做）
# ─────────────────────────────────────────────────────────────
if (-not $SkipBuild) {
    # 检查 zig
    if (-not (Get-Command "zig" -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "❌ 未找到 zig！请先安装 zig 并加入 PATH：" -ForegroundColor Red
        Write-Host "   https://ziglang.org/download/" -ForegroundColor Yellow
        Write-Host "   下载后解压，将目录加入系统 PATH。" -ForegroundColor Yellow
        exit 1
    }
    $zigVer = (zig version 2>&1).Trim()
    Write-Host "   zig: $zigVer" -ForegroundColor Gray

    # 检查 cargo-zigbuild
    $hasZigbuild = $null -ne (Get-Command "cargo-zigbuild" -ErrorAction SilentlyContinue)
    if (-not $hasZigbuild) {
        # 也许作为 cargo subcommand 存在
        $hasZigbuild = (cargo zigbuild --version 2>$null) -ne $null
    }
    if (-not $hasZigbuild) {
        Write-Host "📦 安装 cargo-zigbuild..." -ForegroundColor Yellow
        cargo install cargo-zigbuild
        if ($LASTEXITCODE -ne 0) { Write-Error "cargo-zigbuild 安装失败" }
    }

    # 检查 musl target
    $targets = rustup target list --installed 2>$null
    if ($targets -notmatch [regex]::Escape($Target)) {
        Write-Host "📦 添加 rustup target $Target..." -ForegroundColor Yellow
        rustup target add $Target
        if ($LASTEXITCODE -ne 0) { Write-Error "rustup target add 失败" }
    }
}

# ─────────────────────────────────────────────────────────────
# 3. 编译（临时工作树 — 确保从干净 commit 构建）
# ─────────────────────────────────────────────────────────────
# 把编译产物放在临时工作树内的 target/，与其他项目完全隔离
$TmpWorktree  = Join-Path (Split-Path $RepoRoot -Parent) "elon-build-$Sha"
$BuildBinDir  = Join-Path $TmpWorktree "server" "target" $Target "release"
$Binary       = Join-Path $BuildBinDir "elon-server"

function Remove-Worktree {
    if (Test-Path $TmpWorktree) {
        Write-Host "   🧹 清理临时工作树..." -ForegroundColor Gray
        git -C $RepoRoot worktree remove $TmpWorktree --force 2>$null | Out-Null
    }
}

if (-not $SkipBuild) {
    # 清理残留工作树（上次异常中断可能遗留）
    Remove-Worktree

    Write-Host "2⃣  创建临时工作树（$Sha）..." -ForegroundColor Yellow
    git -C $RepoRoot worktree add --detach $TmpWorktree HEAD
    if ($LASTEXITCODE -ne 0) { Write-Error "git worktree add 失败" }

    $TmpServerDir = Join-Path $TmpWorktree "server"

    Write-Host "3⃣  交叉编译 → $Target..." -ForegroundColor Yellow
    Push-Location $TmpServerDir
    try {
        # 强制把产物输出到临时工作树内，不污染其他项目的 target 目录
        $env:CARGO_TARGET_DIR = [System.IO.Path]::GetFullPath((Join-Path $TmpServerDir "target"))
        cargo zigbuild --release --target $Target
        if ($LASTEXITCODE -ne 0) {
            Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
            Pop-Location
            Remove-Worktree
            Write-Error "❌ 编译失败"
        }
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } catch {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
        Pop-Location -ErrorAction SilentlyContinue
        Remove-Worktree
        throw
    }
    Pop-Location

    if (-not (Test-Path $Binary)) {
        Remove-Worktree
        Write-Error "❌ 编译产物不存在: $Binary"
    }

    $sizeKB = [math]::Round((Get-Item $Binary).Length / 1KB, 0)
    Write-Host "   ✅ 编译成功！产物 $([math]::Round($sizeKB/1024,1)) MB" -ForegroundColor Green
} else {
    Write-Host "2⃣  ⏩ 跳过编译（-SkipBuild）" -ForegroundColor Yellow
    if (-not (Test-Path $Binary)) {
        # SkipBuild 时找不到临时工作树的 binary，尝试从工作区 target 中找
        $cargoTargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $ServerDir "target" }
        $Binary = Join-Path $cargoTargetDir $Target "release" "elon-server"
        if (-not (Test-Path $Binary)) {
            Write-Error "❌ 找不到编译产物。请先不带 -SkipBuild 运行一次。"
        }
        Write-Host "   使用已有产物: $Binary" -ForegroundColor Gray
    }
}

if ($SkipUpload) {
    Write-Host ""
    Write-Host "✅ 本地编译完成（-SkipUpload，未部署）" -ForegroundColor Green
    Write-Host "   产物: $Binary" -ForegroundColor Gray
    Remove-Worktree
    exit 0
}

# ─────────────────────────────────────────────────────────────
# 4. 上传到服务器（staging 路径，原子替换）
# ─────────────────────────────────────────────────────────────
Write-Host "4⃣  上传 binary 到服务器..." -ForegroundColor Yellow
$stagingPath = "/tmp/elon-server-new"
scp @SshOpts $Binary "${Server}:${stagingPath}"
if ($LASTEXITCODE -ne 0) {
    Remove-Worktree
    Write-Error "❌ SCP 上传失败"
}
Write-Host "   ✅ 上传完成" -ForegroundColor Green

# ─────────────────────────────────────────────────────────────
# 5. 替换 binary + 重启服务
# ─────────────────────────────────────────────────────────────
Write-Host "5⃣  替换 binary 并重启服务..." -ForegroundColor Yellow
# 分步执行（避免 && 因 pkill 无进程时返回 1 导致整条命令失败）
ssh @SshOpts $Server "mkdir -p $(Split-Path $RemoteBin -Parent) 2>/dev/null; mv $stagingPath $RemoteBin; chmod +x $RemoteBin"
if ($LASTEXITCODE -ne 0) {
    Remove-Worktree
    Write-Error "❌ binary 替换失败"
}
ssh @SshOpts $Server "pkill -f elon-server 2>/dev/null; sleep 1; nohup $RemoteBin >> /root/elon-server.log 2>&1 & echo \$!"
# nohup 在后台运行，SSH 退出码不代表服务失败，这里不检查

Write-Host "   ✅ 服务重启指令已发送" -ForegroundColor Green

# ─────────────────────────────────────────────────────────────
# 6. 验证
# ─────────────────────────────────────────────────────────────
Write-Host "6⃣  等待服务启动（3 秒）..." -ForegroundColor Yellow
Start-Sleep 3

$health = curl.exe --noproxy '*' -s --max-time 10 "http://43.139.149.158:8080/health" 2>&1
if ($health -and $health.ToString().Trim() -ne "") {
    Write-Host "   ✅ 健康检查: $health" -ForegroundColor Green
} else {
    Write-Host "   ⚠️  健康检查无响应（服务可能还在启动中，手动确认：curl.exe --noproxy '*' http://43.139.149.158:8080/health）" -ForegroundColor Yellow
}

# ─────────────────────────────────────────────────────────────
# 7. 清理工作树
# ─────────────────────────────────────────────────────────────
Remove-Worktree

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "   ✅ 部署完成！" -ForegroundColor Green
Write-Host "   SHA:    $Sha" -ForegroundColor Gray
Write-Host "   服务:   http://43.139.149.158:8080/health" -ForegroundColor Gray
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
