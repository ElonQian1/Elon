<#
.SYNOPSIS
    elon cli 服务端 — 本地交叉编译 → 部署（版本号由服务器分配）
.DESCRIPTION
    新版业务流程（version-from-server，不再把版本号写进 git）：

      1. git fetch origin main + fast-forward only (业务代码 commit 必须先由 AI 自己 push)
      2. POST /api/release/claim          (服务器原子分配一个新的 versionName 给本机)
      3. cargo zigbuild --target musl     (设 ELON_BUILD_VERSION=<assignedVersionName> 注入产物)
      4. SHA 顺序检查 + flock CAS         (服务器若已有更新版本则作废本机产物)
      5. POST /api/release/finish         (汇报成功/失败，释放 in-flight 槽位)

    版本号永远不进 Cargo.toml / git。多 AI 并发不再因为"同时 bump 同一版本号"
    撞 rebase 死循环；脚本不再自动 commit/push 版本号。

    依赖（首次运行前手动安装一次即可）：
      1. zig 工具链：https://ziglang.org/download/  → 解压后加入 PATH
      2. cargo-zigbuild：cargo install cargo-zigbuild
      3. musl target：rustup target add x86_64-unknown-linux-musl
      4. OpenSSH 客户端（Windows 11 内置）

.PARAMETER SkipBuild
    跳过编译，用上次已有的产物直接重新部署（仅上传 + 重启）。注意：仍会调用 claim 拿新版本号，
    但 binary 是旧的，binary 内嵌的版本可能与本次 assignedVersionName 不符。
.PARAMETER SkipUpload
    只做本地编译，不上传不重启（用于本地验证 binary）。会调用 finish(success=false) 释放槽位。
.PARAMETER Force
    强制重新部署，跳过 SHA 顺序检查 + CAS（仅在确认要覆盖线上更新版本时使用）。
.PARAMETER SkipVersionBump
    [DEPRECATED] 旧流程参数，已无意义（版本号由服务器分配）。保留只为兼容，会打印一条提示。

.NOTES
    构建缓存优先级（高→低）：
      1. 机器级 User 环境变量 RUST_SERVER_MUSL_TARGET_DIR=D:\rust\shared\server-musl-target
         兼容旧名 RUST_MUSL_TARGET_DIR。
         多个 Rust 后端项目共享同一份 musl 增量编译产物，设置一次全局生效。
      2. 仓库根 .env.local 中 ELON_BUILD_TARGET_DIR=D:\rust\shared（父目录）
         脚本追加固定子目录名 elon-server-musl，适合只有本项目需要自定义的场景。
      3. 未设置时：%LOCALAPPDATA%\Elon\build-target\elon-server-musl（Windows 默认）

    并发安全模型（出现中止提示时参考）：
      T0  git fetch origin main + fast-forward ← 基于最新 main 编译
      T1  本地编译（几分钟到几十分钟）            ← 窗口期，其他 PC 可能抢先发布
      T2  服务器祖先检查：serverSha 是 localSha 的祖先 → 继续；否则 → 中止（exit 0）
      T3  flock 锁内 CAS 二次校验（最后防线）
      T4  替换 binary + 重启
      T5  HTTP 健康检查
      中止提示"部署已中止：服务器版本更新"是正常的并发保护，不是失败。
      直接验证线上版本；强制覆盖用 -Force 参数。

    共享脚本路径规则（调试脚本或写新脚本时注意）：
      绝不能在共享脚本里写死某台 PC 的盘符/用户名（如 E:\、C:\Users\Alice\）。
      机器差异走 .env.local（已在 .gitignore）或进程环境变量覆盖，
      可参考 .env.local.example 了解可配置项。
      CARGO_TARGET_DIR 不能是相对路径，否则随 cwd 漂移产生鬼影 target 目录。

    CPU 兼容规则：
      发布构建强制使用 CARGO_ENCODED_RUSTFLAGS="-C target-cpu=x86-64"。
      即使某台开发机的全局 Cargo config 写了 target-cpu=native，也不会污染服务器产物。

.EXAMPLE
    .\scripts\publish-server.ps1                          # 正常流程（claim → 编译 → 部署 → finish）
    .\scripts\publish-server.ps1 -SkipBuild               # 只用上次产物重新部署
    .\scripts\publish-server.ps1 -SkipUpload              # 只本地编译，不部署
    .\scripts\publish-server.ps1 -Force                   # 强制覆盖线上版本（绕过祖先检查 + CAS）
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

function Import-LocalEnvFile {
    param([string]$Path)

    if (-not (Test-Path $Path)) { return }

    foreach ($line in Get-Content $Path -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) {
            continue
        }
        if ($trimmed -notmatch '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$') {
            continue
        }

        $name = $Matches[1]
        $value = $Matches[2].Trim()
        if ($value.Length -ge 2) {
            $first = $value.Substring(0, 1)
            $last = $value.Substring($value.Length - 1, 1)
            if (($first -eq '"' -and $last -eq '"') -or ($first -eq "'" -and $last -eq "'")) {
                $value = $value.Substring(1, $value.Length - 2)
            }
        }

        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

function Resolve-BuildTargetRoot {
    param([string]$RepoRoot)

    if (-not [string]::IsNullOrWhiteSpace($env:ELON_BUILD_TARGET_DIR)) {
        $root = $env:ELON_BUILD_TARGET_DIR.Trim()
    } elseif (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        $root = Join-Path $env:LOCALAPPDATA "Elon\build-target"
    } else {
        $root = Join-Path (Split-Path $RepoRoot -Parent) ".elon-build-target"
    }

    if (-not [System.IO.Path]::IsPathRooted($root)) {
        Write-Error "❌ ELON_BUILD_TARGET_DIR 必须是绝对路径，当前值: $root"
    }

    $fullPath = [System.IO.Path]::GetFullPath($root)
    $pathRoot = [System.IO.Path]::GetPathRoot($fullPath)
    if ($pathRoot -and -not (Test-Path $pathRoot)) {
        Write-Error "❌ 构建缓存目录所在盘符不存在: $fullPath。请在 .env.local 或环境变量中设置 ELON_BUILD_TARGET_DIR。"
    }

    New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
    return $fullPath
}

function Resolve-ServerMuslTargetDir {
    param([string]$RepoRoot)

    $targetVarName = $null
    $targetDir = $null
    if (-not [string]::IsNullOrWhiteSpace($env:RUST_SERVER_MUSL_TARGET_DIR)) {
        $targetVarName = "RUST_SERVER_MUSL_TARGET_DIR"
        $targetDir = $env:RUST_SERVER_MUSL_TARGET_DIR.Trim()
    } elseif (-not [string]::IsNullOrWhiteSpace($env:RUST_MUSL_TARGET_DIR)) {
        $targetVarName = "RUST_MUSL_TARGET_DIR"
        $targetDir = $env:RUST_MUSL_TARGET_DIR.Trim()
    }

    if ($targetDir) {
        if (-not [System.IO.Path]::IsPathRooted($targetDir)) {
            Write-Error "❌ $targetVarName 必须是绝对路径，当前值: $targetDir"
        }

        $fullPath = [System.IO.Path]::GetFullPath($targetDir)
        $pathRoot = [System.IO.Path]::GetPathRoot($fullPath)
        if ($pathRoot -and -not (Test-Path $pathRoot)) {
            Write-Error "❌ server musl 构建缓存目录所在盘符不存在: $fullPath"
        }

        New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
        return $fullPath
    }

    return (Join-Path (Resolve-BuildTargetRoot -RepoRoot $RepoRoot) "elon-server-musl")
}

function Get-CargoConfigCandidates {
    param([string]$RepoRoot)

    $paths = New-Object System.Collections.Generic.List[string]
    $dir = [System.IO.DirectoryInfo]::new($RepoRoot)
    while ($dir) {
        $paths.Add((Join-Path $dir.FullName ".cargo\config.toml"))
        $paths.Add((Join-Path $dir.FullName ".cargo\config"))
        $dir = $dir.Parent
    }

    $cargoHome = if (-not [string]::IsNullOrWhiteSpace($env:CARGO_HOME)) {
        $env:CARGO_HOME
    } else {
        Join-Path $HOME ".cargo"
    }
    $paths.Add((Join-Path $cargoHome "config.toml"))
    $paths.Add((Join-Path $cargoHome "config"))

    return $paths | Select-Object -Unique | Where-Object { Test-Path $_ }
}

function Find-NativeRustflagSources {
    param(
        [string]$RepoRoot,
        [string]$Target
    )

    $sources = New-Object System.Collections.Generic.List[string]
    $targetEnv = "CARGO_TARGET_$($Target.ToUpperInvariant().Replace('-', '_'))_RUSTFLAGS"
    $envNames = @(
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        $targetEnv
    )
    foreach ($name in $envNames) {
        $value = [Environment]::GetEnvironmentVariable($name, "Process")
        if ($value -match 'target-cpu\s*=?\s*"?native') {
            $sources.Add("env:$name")
        }
    }

    foreach ($path in Get-CargoConfigCandidates -RepoRoot $RepoRoot) {
        $content = Get-Content -LiteralPath $path -Raw -ErrorAction SilentlyContinue
        if ($content -match 'target-cpu\s*=?\s*"?native') {
            $sources.Add($path)
        }
    }

    return $sources
}

function Enable-PortableReleaseRustflags {
    param(
        [string]$RepoRoot,
        [string]$Target
    )

    $targetEnv = "CARGO_TARGET_$($Target.ToUpperInvariant().Replace('-', '_'))_RUSTFLAGS"
    $envNames = @(
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        $targetEnv
    )
    $saved = @{}
    foreach ($name in $envNames) {
        $saved[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }

    $nativeSources = @(Find-NativeRustflagSources -RepoRoot $RepoRoot -Target $Target)
    if ($nativeSources.Count -gt 0) {
        Write-Host "   ⚠️  检测到 target-cpu=native，发布脚本将忽略这些机器级 rustflags：" -ForegroundColor Yellow
        foreach ($source in $nativeSources) {
            Write-Host "      $source" -ForegroundColor Yellow
        }
    }

    # Cargo checks CARGO_ENCODED_RUSTFLAGS before RUSTFLAGS and config files.
    # Force a portable baseline for release artifacts so a Windows/Linux build
    # machine cannot emit CPU-specific instructions that crash on the server.
    $unitSeparator = [char]0x1f
    $env:CARGO_ENCODED_RUSTFLAGS = "-C${unitSeparator}target-cpu=x86-64"
    Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    Remove-Item Env:CARGO_BUILD_RUSTFLAGS -ErrorAction SilentlyContinue
    Remove-Item "Env:$targetEnv" -ErrorAction SilentlyContinue
    Write-Host "   ✅ Release rustflags: -C target-cpu=x86-64（屏蔽全局 target-cpu=native）" -ForegroundColor Green

    return $saved
}

function Restore-ReleaseRustflags {
    param([hashtable]$Saved)

    if (-not $Saved) { return }
    foreach ($name in $Saved.Keys) {
        if ($null -eq $Saved[$name]) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable($name, [string]$Saved[$name], "Process")
        }
    }
}

Import-LocalEnvFile (Join-Path $RepoRoot ".env.local")

# ─────────────────────────────────────────────────────────────
# Release API helper（与服务器 /api/release/{claim,heartbeat,finish} 通讯）
# ─────────────────────────────────────────────────────────────
$ServerHttpBase = "http://43.139.149.158:8080"
$ReleaseApiBase = "$ServerHttpBase/api/release"

function Invoke-ReleaseApi {
    param(
        [Parameter(Mandatory)] [string]$Endpoint,
        [object]$Body = $null,
        [int]$TimeoutSec = 20
    )
    $url = "$ReleaseApiBase/$Endpoint"
    $tmp = $null
    try {
        $curlArgs = @('--noproxy','*','-s','--max-time',$TimeoutSec,'-w','\n__HTTP_STATUS__:%{http_code}','-X','POST',$url)
        if ($Body) {
            $json = ($Body | ConvertTo-Json -Depth 6 -Compress)
            $tmp = [System.IO.Path]::GetTempFileName()
            [System.IO.File]::WriteAllText($tmp, $json, [System.Text.UTF8Encoding]::new($false))
            $curlArgs += @('-H','Content-Type: application/json; charset=utf-8','--data-binary',"@$tmp")
        }
        $raw = & curl.exe @curlArgs 2>&1
        $rawText = ($raw -join "`n")
        if ($LASTEXITCODE -ne 0) {
            throw "curl 调用失败 ($Endpoint, exit=$LASTEXITCODE): $rawText"
        }
        $statusLine = ($rawText -split "`n") | Where-Object { $_ -match '^__HTTP_STATUS__:' } | Select-Object -Last 1
        $bodyText = ($rawText -replace "(?s)\n?__HTTP_STATUS__:\d+\s*$","")
        $status = if ($statusLine) { [int]($statusLine -replace '^__HTTP_STATUS__:','') } else { 0 }
        if ($status -lt 200 -or $status -ge 300) {
            throw "release/$Endpoint HTTP ${status}: $bodyText"
        }
        if ([string]::IsNullOrWhiteSpace($bodyText)) { return $null }
        return ($bodyText | ConvertFrom-Json)
    } finally {
        if ($tmp -and (Test-Path $tmp)) { Remove-Item $tmp -Force -ErrorAction SilentlyContinue }
    }
}

function Invoke-HttpJson {
    param(
        [Parameter(Mandatory)] [string]$Url,
        [int]$TimeoutSec = 10
    )
    $raw = & curl.exe --noproxy '*' -s --max-time $TimeoutSec -w "`n__HTTP_STATUS__:%{http_code}" $Url 2>&1
    $rawText = ($raw -join "`n")
    if ($LASTEXITCODE -ne 0) {
        throw "curl GET 失败 (exit=$LASTEXITCODE): $rawText"
    }
    $statusLine = ($rawText -split "`n") | Where-Object { $_ -match '^__HTTP_STATUS__:' } | Select-Object -Last 1
    $bodyText = ($rawText -replace "(?s)\n?__HTTP_STATUS__:\d+\s*$","")
    $status = if ($statusLine) { [int]($statusLine -replace '^__HTTP_STATUS__:','') } else { 0 }
    if ($status -lt 200 -or $status -ge 300) {
        throw "HTTP ${status}: $bodyText"
    }
    if ([string]::IsNullOrWhiteSpace($bodyText)) { return $null }
    return ($bodyText | ConvertFrom-Json)
}

function Get-VersionSortKey {
    param([string]$VersionName)
    if ($VersionName -match '^(\d+)\.(\d+)\.(\d+)$') {
        return ('{0:D8}.{1:D8}.{2:D8}' -f [int]$Matches[1], [int]$Matches[2], [int]$Matches[3])
    }
    return '00000000.00000000.00000000'
}

function Get-ServerReleaseVersionBaseline {
    $candidates = @()
    $errors = @()

    try {
        $status = Invoke-HttpJson -Url "$ReleaseApiBase/status?kind=server" -TimeoutSec 10
        $name = [string]$status.lastPublishedVersionName
        if (-not [string]::IsNullOrWhiteSpace($name)) {
            $candidates += [pscustomobject]@{
                Source      = '/api/release/status'
                VersionName = $name
            }
        }
    } catch {
        $errors += "/api/release/status?kind=server: $_"
    }

    try {
        $live = Invoke-HttpJson -Url "$ServerHttpBase/api/server/version" -TimeoutSec 10
        $name = [string]$live.versionName
        if (-not [string]::IsNullOrWhiteSpace($name)) {
            $candidates += [pscustomobject]@{
                Source      = '/api/server/version'
                VersionName = $name
            }
        }
    } catch {
        $errors += "/api/server/version: $_"
    }

    if ($candidates.Count -eq 0) {
        foreach ($errorText in $errors) {
            Write-Warning "   ⚠️  后端版本基线读取失败：$errorText"
        }
        Write-Error "❌ 无法读取服务器后端版本基线；发布已停止，避免用 Cargo.toml 兜底版本发布。"
    }

    $selected = $candidates |
        Sort-Object -Property @{ Expression = { Get-VersionSortKey $_.VersionName }; Descending = $true } |
        Select-Object -First 1
    foreach ($candidate in $candidates) {
        if ($candidate.VersionName -ne $selected.VersionName) {
            Write-Warning "   ⚠️  服务器后端版本来源不一致：$($candidate.Source)=v$($candidate.VersionName)，最终采用 v$($selected.VersionName)"
        }
    }
    Write-Host "   ℹ️  服务器后端版本基线: v$($selected.VersionName) [$($selected.Source)]" -ForegroundColor DarkGray
    return $selected
}

# 全局状态：claim token，脚本失败/中止时用来调 finish 释放槽位
$script:ReleaseToken = $null
$script:ReleaseFinished = $false

function Complete-Release {
    param(
        [Parameter(Mandatory)] [bool]$Success,
        [string]$VersionName = '',
        [string]$Sha = '',
        [string]$ErrorMessage = ''
    )
    if (-not $script:ReleaseToken -or $script:ReleaseFinished) { return }
    try {
        $payload = @{
            kind  = 'server'
            token = $script:ReleaseToken
            success = $Success
        }
        if ($Success) {
            if ($VersionName) { $payload.versionName = $VersionName }
            if ($Sha)         { $payload.sha = $Sha }
        } else {
            if ($ErrorMessage) { $payload.errorMessage = $ErrorMessage }
        }
        Invoke-ReleaseApi -Endpoint 'finish' -Body $payload | Out-Null
        $script:ReleaseFinished = $true
    } catch {
        Write-Host "   ⚠️  release/finish 调用失败（不影响主流程）: $_" -ForegroundColor Yellow
    }
}

if ($SkipVersionBump) {
    Write-Host "ℹ️  -SkipVersionBump 已废弃（版本号现由服务器分配，不再写 Cargo.toml）。" -ForegroundColor DarkGray
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
# 1. git fetch + fast-forward（同步业务代码；版本号不再由本机改动）
# ─────────────────────────────────────────────────────────────
Write-Host "1⃣  同步最新代码..." -ForegroundColor Yellow
$dirty = (git -C $RepoRoot status --porcelain 2>$null) | Out-String
$dirty = $dirty.Trim()
if ($dirty) {
    Write-Host ""
    Write-Host "❌ 工作区不干净，请先 commit + push 业务改动再运行部署脚本：" -ForegroundColor Red
    Write-Host $dirty -ForegroundColor Yellow
    Write-Host ""
    Write-Error "工作区有未提交改动"
}

git -C $RepoRoot fetch origin main | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Error "git fetch origin main 失败" }

$localHead = (git -C $RepoRoot rev-parse HEAD).Trim()
$remoteHead = (git -C $RepoRoot rev-parse origin/main).Trim()

if ($localHead -ne $remoteHead) {
    git -C $RepoRoot merge-base --is-ancestor $localHead $remoteHead | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ℹ️  本地 HEAD 已包含在 origin/main 中，快进到最新 main：$($remoteHead.Substring(0, 7))" -ForegroundColor Cyan
        git -C $RepoRoot merge --ff-only origin/main | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Error "git merge --ff-only origin/main 失败" }
    } else {
        git -C $RepoRoot merge-base --is-ancestor $remoteHead $localHead | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Error "当前 HEAD 尚未进入 origin/main，禁止基于未推送提交部署。请先执行：git push origin HEAD:main"
        }
        Write-Error "当前 HEAD 与 origin/main 已分叉，发布脚本不会自动 rebase。请先完成代码合并并 push 后再运行。"
    }
}

$Sha      = (git -C $RepoRoot rev-parse --short HEAD).Trim()
$ShaBig   = (git -C $RepoRoot rev-parse HEAD).Trim()
$CargoTomlPath = Join-Path $ServerDir "Cargo.toml"
$FallbackVersion = [regex]::Match(
    (Get-Content $CargoTomlPath -Encoding UTF8 -Raw),
    '(?m)^version\s*=\s*"([^"]+)"'
).Groups[1].Value
Write-Host "   ✅ 最新 SHA: $Sha" -ForegroundColor Green
Write-Host "   ℹ️  Cargo.toml 兜底版本: v$FallbackVersion（不会被本次脚本修改）" -ForegroundColor DarkGray
$serverVersionBaseline = Get-ServerReleaseVersionBaseline
$ClaimCurrentVersion = [string]$serverVersionBaseline.VersionName

# ─────────────────────────────────────────────────────────────
# 1.5  从服务器原子分配新版本号（claim）
# ─────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "1.5⃣  向服务器申请新版本号..." -ForegroundColor Yellow
$builderId = "$env:COMPUTERNAME-$env:USERNAME"
if ([string]::IsNullOrWhiteSpace($builderId) -or $builderId -eq "-") {
    $builderId = "unknown-builder-" + ([Guid]::NewGuid().ToString().Substring(0,8))
}
$builderLabel = "publish-server.ps1 @ $builderId"

try {
    $claim = Invoke-ReleaseApi -Endpoint 'claim' -Body (@{
        kind              = 'server'
        sha               = $ShaBig
        builderId         = $builderId
        builderLabel      = $builderLabel
        bump              = 'patch'
        currentVersionName = $ClaimCurrentVersion
    })
} catch {
    Write-Error "❌ /api/release/claim 失败：$_"
}

if (-not $claim -or $claim.action -ne 'build') {
    Write-Error "❌ release/claim 返回非预期响应：$($claim | ConvertTo-Json -Compress)"
}

$script:ReleaseToken = [string]$claim.token
$AssignedVersion     = [string]$claim.assignedVersionName
if ([string]::IsNullOrWhiteSpace($AssignedVersion)) {
    Write-Error "❌ release/claim 未返回 assignedVersionName"
}
$InFlightCount = if ($claim.PSObject.Properties.Match('inFlightCount').Count) { [int]$claim.inFlightCount } else { 1 }
Write-Host "   ✅ 已分配版本号: v$AssignedVersion (token=$($script:ReleaseToken.Substring(0,8))..., in-flight=$InFlightCount)" -ForegroundColor Green

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
$TmpWorktree  = Join-Path (Split-Path $RepoRoot -Parent) "elon-build-$Sha"
# 优先使用机器级中性目录，让多个 Rust 后端共享同一份 server-musl target。
# 未配置 RUST_SERVER_MUSL_TARGET_DIR/RUST_MUSL_TARGET_DIR 时，保留旧的 ELON_BUILD_TARGET_DIR/elon-server-musl 兼容路径。
$BuildTargetDir = Resolve-ServerMuslTargetDir -RepoRoot $RepoRoot
$BuildTargetRoot = Split-Path $BuildTargetDir -Parent
$BuildBinDir  = [System.IO.Path]::Combine($BuildTargetDir, $Target, "release")
$Binary       = Join-Path $BuildBinDir "elon-server"
Write-Host "  构建缓存: $BuildTargetDir" -ForegroundColor Gray

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
    $savedReleaseRustflags = $null
    try {
        # Build outside the temporary worktree when a machine-specific cache is configured.
        $env:CARGO_TARGET_DIR = $BuildTargetDir
        $env:ELON_SERVER_GIT_SHA = $ShaBig
        $env:ELON_BUILD_VERSION  = $AssignedVersion
        $savedReleaseRustflags = Enable-PortableReleaseRustflags -RepoRoot $RepoRoot -Target $Target
        cargo zigbuild --release --target $Target --bin elon-server
        $cargoExitCode = $LASTEXITCODE
        Restore-ReleaseRustflags -Saved $savedReleaseRustflags
        $savedReleaseRustflags = $null
        if ($cargoExitCode -ne 0) {
            Remove-Item Env:ELON_SERVER_GIT_SHA -ErrorAction SilentlyContinue
            Remove-Item Env:ELON_BUILD_VERSION  -ErrorAction SilentlyContinue
            Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
            Pop-Location
            Remove-Worktree
            Complete-Release -Success $false -ErrorMessage "cargo zigbuild failed"
            Write-Error "❌ 编译失败"
        }
        Remove-Item Env:ELON_SERVER_GIT_SHA -ErrorAction SilentlyContinue
        Remove-Item Env:ELON_BUILD_VERSION  -ErrorAction SilentlyContinue
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } catch {
        Restore-ReleaseRustflags -Saved $savedReleaseRustflags
        Remove-Item Env:ELON_SERVER_GIT_SHA -ErrorAction SilentlyContinue
        Remove-Item Env:ELON_BUILD_VERSION  -ErrorAction SilentlyContinue
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
        Pop-Location -ErrorAction SilentlyContinue
        Remove-Worktree
        Complete-Release -Success $false -ErrorMessage "cargo zigbuild exception: $_"
        throw
    }
    Pop-Location

    if (-not (Test-Path $Binary)) {
        Remove-Worktree
        Complete-Release -Success $false -ErrorMessage "binary missing after build"
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
    Write-Host "   注入版本: v$AssignedVersion" -ForegroundColor Gray
    Remove-Worktree
    Complete-Release -Success $false -ErrorMessage "skip upload (local build only)"
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
    Complete-Release -Success $false -ErrorMessage "scp upload failed"
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
            Complete-Release -Success $false -ErrorMessage "superseded by server sha $serverSha"
            $shortServer = $serverSha.Substring(0, [Math]::Min(8, $serverSha.Length))
            Write-Host ""
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host "   ⚠️  部署已中止：服务器版本更新" -ForegroundColor Yellow
            Write-Host "   服务器当前: $shortServer（比本次 $Sha 更新）" -ForegroundColor Yellow
            Write-Host "   原因：另一个开发者已部署了更新版本，本次编译基于旧 commit。" -ForegroundColor Yellow
            Write-Host "   处理：本次代码若已 push，则发布交由后续最新 main；明确发布协调任务可重新运行，或用 -Force 强制覆盖。" -ForegroundColor Yellow
            Write-Host "   release/finish 已调用 (success=false)，分配的 v$AssignedVersion 已释放。" -ForegroundColor Yellow
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
    Complete-Release -Success $false -ErrorMessage "cas conflict inside flock: $lockResult"
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
    Write-Host "   ⚠️  部署已中止：CAS 冲突（锁内检测到并发部署）" -ForegroundColor Yellow
    Write-Host "   $lockResult" -ForegroundColor Yellow
    Write-Host "   处理：本次代码若已 push，则发布交由后续最新 main；明确发布协调任务可重新运行，或用 -Force 强制覆盖。" -ForegroundColor Yellow
    Write-Host "   release/finish 已调用 (success=false)，分配的 v$AssignedVersion 已释放。" -ForegroundColor Yellow
    Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
    exit 0
} elseif ($lockExit -ne 0) {
    Remove-Worktree
    Complete-Release -Success $false -ErrorMessage "deploy script failed: exit=$lockExit"
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
# 7. 清理工作树 + finish(success=true)
# ─────────────────────────────────────────────────────────────
Remove-Worktree
Complete-Release -Success $true -VersionName $AssignedVersion -Sha $ShaBig

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "   ✅ 部署完成！" -ForegroundColor Green
Write-Host "   版本:   v$AssignedVersion  (服务器分配，未写入 git)" -ForegroundColor Gray
Write-Host "   SHA:    $Sha" -ForegroundColor Gray
Write-Host "   服务:   http://43.139.149.158:8080/health" -ForegroundColor Gray
Write-Host "   版本接口: http://43.139.149.158:8080/api/server/version" -ForegroundColor Gray
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# ─────────────────────────────────────────────────────────────
# 8. 自动清理已合并的孤儿 task worktree（防累积）
# ─────────────────────────────────────────────────────────────
$cleanupScript = Join-Path $RepoRoot "scripts\cleanup-task-worktrees.ps1"
if (Test-Path -LiteralPath $cleanupScript) {
    try {
        $cleanupOut = & powershell -NoProfile -ExecutionPolicy Bypass -File $cleanupScript -Apply 2>&1
        $removedLine = $cleanupOut | Select-String -Pattern "^完成：清理" | Select-Object -Last 1
        if ($removedLine) {
            Write-Host "   $($removedLine.Line.Trim())（自动）" -ForegroundColor DarkGray
        }
    } catch {
        Write-Host "   ⚠️  自动清理 worktree 失败：$_" -ForegroundColor Yellow
    }
}
