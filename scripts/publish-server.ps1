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
.PARAMETER SkipVersionBump
    跳过自动版本递增（手动已修改 MINOR/MAJOR 版本号时使用）。

.EXAMPLE
    .\scripts\publish-server.ps1                          # 正常流程（自动递增 PATCH）
    .\scripts\publish-server.ps1 -SkipVersionBump         # 跳过版本递增（手动控制版本号时）
    .\scripts\publish-server.ps1 -SkipBuild               # 只用上次产物重新部署
    .\scripts\publish-server.ps1 -SkipUpload              # 只本地编译，不部署
#>
param(
    [switch]$SkipBuild,
    [switch]$SkipUpload,
    [switch]$Force,
    [switch]$SkipVersionBump
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
$CargoTomlPath = Join-Path $ServerDir "Cargo.toml"
$ServerVersion = [regex]::Match(
    (Get-Content $CargoTomlPath -Encoding UTF8 -Raw),
    '(?m)^version\s*=\s*"([^"]+)"'
).Groups[1].Value
Write-Host "   ✅ 最新 SHA: $Sha" -ForegroundColor Green
Write-Host "   ✅ 后端版本: v$ServerVersion" -ForegroundColor Green

# ─────────────────────────────────────────────────────────────
# 1.5  自动递增 PATCH 版本号（仅 Build 且未指定 -SkipVersionBump 时）
# ─────────────────────────────────────────────────────────────
if (-not $SkipBuild -and -not $SkipVersionBump) {
    Write-Host ""
    Write-Host "1.5⃣  自动递增 PATCH 版本号..." -ForegroundColor Yellow
    $cargoContent = Get-Content $CargoTomlPath -Raw -Encoding UTF8
    $oldVersion   = [regex]::Match($cargoContent, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    $parts        = $oldVersion.Split('.')
    $parts[2]     = [string]([int]$parts[2] + 1)
    $newVersion   = $parts -join '.'
    # 只替换第一个 version = "..." 行（[package] 段，不误改依赖版本）
    $cargoContent = [regex]::Replace($cargoContent, '(?m)^(version\s*=\s*)"[^"]+"', "`${1}`"$newVersion`"", 1)
    Set-Content $CargoTomlPath $cargoContent -NoNewline -Encoding UTF8

    Write-Host "   📦 版本号递增: v$oldVersion → v$newVersion" -ForegroundColor Cyan

    git -C $RepoRoot add "server/Cargo.toml"
    git -C $RepoRoot commit -m "chore(server): bump version to $newVersion"
    if ($LASTEXITCODE -ne 0) { Write-Error "❌ git commit 版本号失败" }

    git -C $RepoRoot push origin main
    if ($LASTEXITCODE -ne 0) { Write-Error "❌ git push 版本号失败（存在冲突时请先 git pull --rebase 后重试）" }

    # 版本 commit 后 SHA 变了，重新获取
    $Sha           = (git -C $RepoRoot rev-parse --short HEAD).Trim()
    $ShaBig        = (git -C $RepoRoot rev-parse HEAD).Trim()
    $ServerVersion = $newVersion
    Write-Host "   ✅ 版本已提交推送 SHA: $Sha  v$ServerVersion" -ForegroundColor Green
} elseif ($SkipVersionBump) {
    Write-Host "1.5⃣  ⏩ 跳过版本递增（-SkipVersionBump）" -ForegroundColor Yellow
}

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
$BuildBinDir  = [System.IO.Path]::Combine($TmpWorktree, "server", "target", $Target, "release")
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
        $env:ELON_SERVER_GIT_SHA = $ShaBig
        cargo zigbuild --release --target $Target
        if ($LASTEXITCODE -ne 0) {
            Remove-Item Env:ELON_SERVER_GIT_SHA -ErrorAction SilentlyContinue
            Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
            Pop-Location
            Remove-Worktree
            Write-Error "❌ 编译失败"
        }
        Remove-Item Env:ELON_SERVER_GIT_SHA -ErrorAction SilentlyContinue
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } catch {
        Remove-Item Env:ELON_SERVER_GIT_SHA -ErrorAction SilentlyContinue
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
        $Binary = [System.IO.Path]::Combine($cargoTargetDir, $Target, "release", "elon-server")
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
# 4. 上传到服务器（staging 路径用 SHA 命名，避免并发部署互相覆盖）
# ─────────────────────────────────────────────────────────────
Write-Host "4⃣  上传 binary 到服务器..." -ForegroundColor Yellow
# 每次部署 staging 路径唯一（含 SHA），两个开发者同时部署不会互相覆盖 binary
$stagingPath = "/tmp/elon-server-$Sha"
scp @SshOpts $Binary "${Server}:${stagingPath}"
if ($LASTEXITCODE -ne 0) {
    Remove-Worktree
    Write-Error "❌ SCP 上传失败"
}
Write-Host "   ✅ 上传完成" -ForegroundColor Green

# ─────────────────────────────────────────────────────────────
# 4.5  SHA 顺序检查（防止旧版编译慢覆盖新版）
# ─────────────────────────────────────────────────────────────
if (-not $Force) {
    $deployedShaFile = "$RemoteDir/.deployed-sha"
    $serverSha = (ssh @SshOpts $Server "cat $deployedShaFile 2>/dev/null || echo ''").Trim()
    if ($serverSha -and $serverSha -ne $ShaBig) {
        # 检查服务器的 SHA 是否是我们的祖先（即我们更新）
        git -C $RepoRoot merge-base --is-ancestor $serverSha $ShaBig 2>$null | Out-Null
        if ($LASTEXITCODE -ne 0) {
            # 服务器 SHA 不是我们的祖先 → 服务器已有更新版本，拒绝回退
            ssh @SshOpts $Server "rm -f $stagingPath" 2>$null
            Remove-Worktree
            $shortServer = $serverSha.Substring(0, [Math]::Min(8, $serverSha.Length))
            Write-Host ""
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host "   ⚠️  部署已中止：服务器版本更新" -ForegroundColor Yellow
            Write-Host "   服务器当前: $shortServer（比本次 $Sha 更新）" -ForegroundColor Yellow
            Write-Host "   原因：另一个开发者已部署了更新版本，本次编译基于旧 commit。" -ForegroundColor Yellow
            Write-Host "   解决：git pull --rebase 后重新编译部署，或用 -Force 强制覆盖。" -ForegroundColor Yellow
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host ""
            exit 0
        }
    }
    Write-Host "   ✅ SHA 顺序检查通过（本次 $Sha 是最新版本）" -ForegroundColor Green
}

# ─────────────────────────────────────────────────────────────
# 5. 替换 binary + 重启服务（flock 互斥锁 + CAS 原子化）
# ─────────────────────────────────────────────────────────────
# 锁保护范围：CAS 校验 .deployed-sha + mv + restart + 写新 SHA。
# 即使两台 PC 都通过了步骤 4.5 的祖先检查，在锁内仍会重新比对
# .deployed-sha == EXPECTED（客户端进入锁前看到的服务器 SHA），
# 任何中途被别人抢先部署 → 退出码 42 → 本端拒绝覆盖。
Write-Host "5⃣  替换 binary 并重启服务（flock 互斥锁保护）..." -ForegroundColor Yellow
$remoteBinDir = Split-Path $RemoteBin -Parent
$expectedSha = if ($Force) { '__FORCE__' } elseif ($serverSha) { $serverSha } else { '' }
$lockScriptTemplate = @'
set -e
EXPECTED='__EXPECTED__'
NEW='__NEW__'
STAGING='__STAGING__'
DEST='__DEST__'
DEST_DIR='__DESTDIR__'
SHA_FILE='__SHAFILE__'
REMOTE_DIR='__REMOTEDIR__'
CURRENT=$(cat "$SHA_FILE" 2>/dev/null || echo '')
if [ "$EXPECTED" != "__FORCE__" ] && [ -n "$CURRENT" ] && [ "$CURRENT" != "$EXPECTED" ]; then
  echo "CAS_CONFLICT current=$CURRENT expected=$EXPECTED" >&2
  rm -f "$STAGING" 2>/dev/null || true
  exit 42
fi
mkdir -p "$DEST_DIR"
mv "$STAGING" "$DEST"
chmod +x "$DEST"
if systemctl is-enabled elon-server >/dev/null 2>&1; then
  systemctl restart elon-server
else
  pkill -f elon-server 2>/dev/null || true
  sleep 1
  fuser -k 8080/tcp 2>/dev/null || true
  sleep 1
  cd "$REMOTE_DIR" && nohup "$DEST" </dev/null >> /root/elon-server.log 2>&1 & disown
  sleep 2
fi
echo "$NEW" > "$SHA_FILE"
echo OK
'@
$lockScript = $lockScriptTemplate.
    Replace('__EXPECTED__', $expectedSha).
    Replace('__NEW__', $ShaBig).
    Replace('__STAGING__', $stagingPath).
    Replace('__DEST__', $RemoteBin).
    Replace('__DESTDIR__', $remoteBinDir).
    Replace('__SHAFILE__', "$RemoteDir/.deployed-sha").
    Replace('__REMOTEDIR__', $RemoteDir)

# 强制 LF 行尾，并用 base64 绕过 PowerShell stdin 自动加 \r\n 的问题
# （否则远端 bash 看到 "set -e\r" → "set: - : invalid option"）
$lockScriptLF = $lockScript -replace "`r`n", "`n"
$lockB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($lockScriptLF))
$lockResult = ssh @SshOpts $Server "flock -x -w 120 /tmp/elon-deploy.lock bash -c 'echo $lockB64 | base64 -d | bash'" 2>&1
$lockExit = $LASTEXITCODE
if ($lockExit -eq 42) {
    Remove-Worktree
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
    Write-Host "   ⚠️  部署已中止：CAS 冲突（锁内检测到并发部署）" -ForegroundColor Yellow
    Write-Host "   $lockResult" -ForegroundColor Yellow
    Write-Host "   解决：git pull --rebase 后重新部署，或用 -Force 强制覆盖。" -ForegroundColor Yellow
    Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
    exit 0
} elseif ($lockExit -ne 0) {
    Remove-Worktree
    Write-Error "❌ 锁内部署失败（exit=$lockExit）: $lockResult"
}
Write-Host "   ✅ 服务重启指令已发送（锁内完成 mv + restart + 写 SHA）" -ForegroundColor Green
Write-Host "   ✅ SHA 记录已写入服务器 (.deployed-sha = $Sha)" -ForegroundColor Green

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

$serverVersionResp = curl.exe --noproxy '*' -s --max-time 10 "http://43.139.149.158:8080/api/server/version" 2>&1
if ($serverVersionResp -and $serverVersionResp.ToString().Trim() -ne "") {
    Write-Host "   ✅ 后端版本接口: $serverVersionResp" -ForegroundColor Green
} else {
    Write-Host "   ⚠️  后端版本接口无响应（手动确认：curl.exe --noproxy '*' http://43.139.149.158:8080/api/server/version）" -ForegroundColor Yellow
}

# ─────────────────────────────────────────────────────────────
# 7. 清理工作树
# ─────────────────────────────────────────────────────────────
Remove-Worktree

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "   ✅ 部署完成！" -ForegroundColor Green
Write-Host "   版本:   v$ServerVersion" -ForegroundColor Gray
Write-Host "   SHA:    $Sha" -ForegroundColor Gray
Write-Host "   服务:   http://43.139.149.158:8080/health" -ForegroundColor Gray
Write-Host "   版本接口: http://43.139.149.158:8080/api/server/version" -ForegroundColor Gray
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
