#!/usr/bin/env pwsh
# NOTE: Keep this file encoded as UTF-8 with BOM. Windows PowerShell 5.1 must
# read Chinese launcher filenames correctly.
<#
.SYNOPSIS
    构建并上传一龙 PC 节点客户端可执行文件到服务器。

.DESCRIPTION
    默认只完成本机 Windows 构建、验证、持久化安装候选与 post-terminal 激活安排，
    然后把 Linux 构建、跨平台上传、广播和握手交给持久异步 outbox。
    -SynchronousRemote 仅供 outbox worker 执行远端阶段。

.EXAMPLE
    .\scripts\publish-node-agent.ps1
    .\scripts\publish-node-agent.ps1 -Changelog "修复 Win 端自动更新后的恢复提示"
#>

param(
    [switch]$SkipBroadcast,
    [string]$AdminToken = "",
    [string]$Changelog = "",
    [int]$HandshakeWaitSec = 90,
    [switch]$SkipHandshakeWait,
    [switch]$RequireAllOnlineTargetBuild,
    [string]$ReplayPublishedSha = "",
    [switch]$SynchronousRemote,
    [string]$RemoteOutboxEventPath = "",
    [switch]$SkipLocalActivation
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot "direct-network.ps1")
. (Join-Path $PSScriptRoot "publish-server-pc-frontend.ps1")
. (Join-Path $PSScriptRoot "local-env.ps1")
. (Join-Path $PSScriptRoot "node-agent-release-contract.ps1")
. (Join-Path $PSScriptRoot "release-publish-lease.ps1")
. (Join-Path $PSScriptRoot "node-agent-publish-http.ps1")
. (Join-Path $PSScriptRoot "node-agent-publish-replay.ps1")
. (Join-Path $PSScriptRoot "node-agent-release-outbox.ps1")
. (Join-Path $PSScriptRoot "node-agent-local-activation.ps1")
. (Join-Path $PSScriptRoot "node-agent-publish-handshake.ps1")
$Server = "root@43.139.149.158"
$BaseUrl = "http://43.139.149.158:8080"
# data_dir = /opt/elon/data，downloads 子目录与 router.rs 中 state.data_dir.join("downloads") 一致
$RemoteDir = "/opt/elon/data/downloads"
$Bin = "elon-pc-node"
$WindowsClientPackageName = "elon-node-agent-windows.zip"
$RipgrepPackageName = "ripgrep-windows.zip"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$ServerDir = Join-Path $RepoRoot "server"
$ServerManifest = Join-Path $ServerDir "Cargo.toml"
$PcFrontendDir = Join-Path $RepoRoot "pc-frontend"
$PcDistDir = Join-Path $PcFrontendDir "dist"
$DesktopShellManifest = Join-Path $RepoRoot "desktop-shell\src-tauri\Cargo.toml"
$BrandIcon = Join-Path $RepoRoot "desktop-shell\src-tauri\icons\icon.ico"
$PublishLockPath = Join-Path `
    ([System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)) `
    "Elon\locks\node-agent-publish-v1.lock"
$PublishLock = Enter-NodeAgentPublishLock -Path $PublishLockPath
$script:NodeReleaseToken = $null
$script:NodeReleaseOwned = $false
$script:NodeReleaseFinished = $false
$script:NodeReleaseBatchId = ''
$script:NodeReleaseActiveStage = 'windows_node'
$script:NodeReleaseHeartbeat = $null
$script:NodeReleaseContext = $null

function Set-NodeAgentPublishPhase {
    param([string]$Phase, [string]$Status)
    if ($SynchronousRemote -and $null -ne $script:NodeReleaseContext) {
        Set-ElonReleasePhase -Context $script:NodeReleaseContext -Phase $Phase -Status $Status
    }
}

try {
    Import-ElonLocalEnvFile -Path (Join-Path $RepoRoot ".env.local")
    Write-Host "=== 一龙 PC 节点客户端构建 + 发布 ===" -ForegroundColor Cyan

function Resolve-NodeAgentChangelog {
    param(
        [string]$Explicit,
        [string]$Sha
    )

    $text = $Explicit
    if ([string]::IsNullOrWhiteSpace($text)) {
        $text = (git -C $RepoRoot log -1 --format=%s $Sha 2>$null | Select-Object -First 1)
    }
    $text = ([string]$text).Trim() -replace '\s+', ' '
    if ($text.Length -gt 240) {
        return $text.Substring(0, 237) + "..."
    }
    return $text
}

function Invoke-GitFetchMain {
    $result = Invoke-ElonGitHubGitWithProxyFallback -RepoPath $RepoRoot -GitArgs @("fetch", "origin", "main") -RemoteName "origin"
    if ($result.ExitCode -ne 0) { throw "git fetch origin main 失败，无法确认 PC 节点发布基线。$($result.Hint) $($result.Text)" }
    Write-Host "  GitHub SSH route: $($result.Route)" -ForegroundColor DarkGray
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

function Invoke-RemoteBash {
    param([Parameter(Mandatory = $true)][string]$Script)

    $result = Invoke-RemoteBashRaw -Script $Script
    if ($result.ExitCode -ne 0) {
        throw "服务器远程命令失败($($result.ExitCode))：$($result.Output)"
    }
    return $result.Output
}

function Invoke-RemoteBashRaw {
    param([Parameter(Mandatory = $true)][string]$Script)

    $normalizedScript = ConvertTo-RemoteBashScript -Script $Script
    $encoded = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($normalizedScript))
    $remoteCommand = "printf '%s' '$encoded' | base64 -d | bash"
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server $remoteCommand 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = ($output -join "`n").Trim()
    }
}

function ConvertTo-RemoteBashScript {
    param([Parameter(Mandatory = $true)][string]$Script)

    return (($Script -replace "`r`n", "`n") -replace "`r", "`n")
}

function Test-RemoteNodeAgentAdminToken {
    $script = @'
set -eu
for env_file in /etc/elon-server.env /root/Elon/server/.env; do
  if [ -f "$env_file" ]; then
    set -a
    . "$env_file" >/dev/null 2>&1 || true
    set +a
  fi
done
test -n "${ADMIN_TOKEN:-}"
'@
    $result = Invoke-RemoteBashRaw -Script $script
    if ($result.ExitCode -eq 0) { return $true }
    if ($result.ExitCode -eq 1) { return $false }
    throw "无法检查服务器 ADMIN_TOKEN：$($result.Output)"
}

function Invoke-RemoteNodeAgentUpdateBroadcast {
    $script = @'
set -eu
for env_file in /etc/elon-server.env /root/Elon/server/.env; do
  if [ -f "$env_file" ]; then
    set -a
    . "$env_file" >/dev/null 2>&1 || true
    set +a
  fi
done
if [ -z "${ADMIN_TOKEN:-}" ]; then
  echo "ADMIN_TOKEN missing on server" >&2
  exit 2
fi
json=$(curl --noproxy '*' -fsS -X POST 'http://127.0.0.1:8080/api/admin/nodes/push-update' -H "Authorization: Bearer ${ADMIN_TOKEN}" -H 'Content-Type: application/json' --data '{}')
printf '%s' "$json" | base64 | tr -d '\n'
'@
    $raw = Invoke-RemoteBash -Script $script
    if ([string]::IsNullOrWhiteSpace($raw)) { return $null }
    try {
        return ConvertFrom-NodeAgentUtf8Base64Json -Value $raw
    } catch {
        # 广播已经在远端执行；响应解析失败不能安全重试 POST，只保留未知数量证据。
        Write-Host "  广播响应 JSON 解析失败（不影响广播本身）：$($_.Exception.Message)" -ForegroundColor DarkYellow
        return $null
    }
}

function Invoke-RemoteNodePublicDevHandshakeStatus {
    $script = @'
set -eu
for env_file in /etc/elon-server.env /root/Elon/server/.env; do
  if [ -f "$env_file" ]; then
    set -a
    . "$env_file" >/dev/null 2>&1 || true
    set +a
  fi
done
if [ -z "${ADMIN_TOKEN:-}" ]; then
  echo "ADMIN_TOKEN missing on server" >&2
  exit 2
fi
json=$(curl --noproxy '*' -fsS 'http://127.0.0.1:8080/api/admin/nodes/public-dev-handshake' -H "Authorization: Bearer ${ADMIN_TOKEN}")
printf '%s' "$json" | base64 | tr -d '\n'
'@
    $raw = Invoke-RemoteBash -Script $script
    if ([string]::IsNullOrWhiteSpace($raw)) { return $null }
    try {
        return ConvertFrom-NodeAgentUtf8Base64Json -Value $raw
    } catch {
        Write-Host "  握手诊断响应 JSON 解析失败，跳过本轮：$($_.Exception.Message)" -ForegroundColor DarkYellow
        return $null
    }
}

function Invoke-NodePublicDevHandshakeStatus {
    param(
        [string]$Token,
        [bool]$UseRemoteToken
    )

    if (-not [string]::IsNullOrWhiteSpace($Token)) {
        return Invoke-NoProxyJson `
            -Uri "$BaseUrl/api/admin/nodes/public-dev-handshake" `
            -Method "Get" `
            -Headers @{ Authorization = "Bearer $Token" } `
            -TimeoutSec 15
    }

    if ($UseRemoteToken) {
        return Invoke-RemoteNodePublicDevHandshakeStatus
    }

    return $null
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

function Invoke-NodeAgentPcFrontendBuild {
    if (-not (Test-Path (Join-Path $PcFrontendDir "package.json"))) {
        throw "缺少 PC 前端工程：$PcFrontendDir"
    }
    if (-not (Get-Command "npm" -ErrorAction SilentlyContinue)) {
        throw "npm 不在 PATH，无法构建节点包内置 PC 工作台"
    }

    Write-Host "[2.4/4] 构建节点包内置 PC 工作台..." -ForegroundColor Yellow
    Push-Location $PcFrontendDir
    try {
        $lockFile = Join-Path $PcFrontendDir "package-lock.json"
        $nmDir = Join-Path $PcFrontendDir "node_modules"
        $nmInstalled = Join-Path $nmDir ".npm-installed-sha"
        $lockHash = if (Test-Path $lockFile) {
            (Get-FileHash $lockFile -Algorithm MD5).Hash
        } else { '' }
        $prevHash = if (Test-Path $nmInstalled) { Get-Content $nmInstalled -Raw } else { '' }
        $needInstall = (-not (Test-Path $nmDir)) -or ($lockHash -ne $prevHash.Trim())
        if ($needInstall) {
            Write-Host "   安装/更新前端依赖（npm ci）..." -ForegroundColor Gray
            $installExit = Invoke-LoggedCmd -Command "npm ci"
            if ($installExit -ne 0) { throw "npm ci 失败，exit=$installExit" }
            $lockHash | Set-Content $nmInstalled -NoNewline
        }
        Reset-PcFrontendBuildArtifacts -FrontendDir $PcFrontendDir
        $buildExit = Invoke-LoggedCmd -Command "npm run build"
        if ($buildExit -ne 0) { throw "npm run build 失败，exit=$buildExit" }
    } catch {
        $primaryBuildError = $_
        try {
            Reset-PcFrontendBuildArtifacts -FrontendDir $PcFrontendDir
            Invoke-PcFrontendLocalBuild -FrontendDir $PcFrontendDir
        } catch {
            try {
                Invoke-PcFrontendPnpmBuild -FrontendDir $PcFrontendDir
            } catch {
                throw "PC 前端构建失败：$primaryBuildError；fallback: $_"
            }
        }
    } finally {
        Pop-Location
    }
    if (-not (Test-Path (Join-Path $PcDistDir "index.html"))) {
        throw "PC 前端 dist 缺少 index.html：$PcDistDir"
    }
    Write-Host "   PC 工作台 dist 就绪: $PcDistDir" -ForegroundColor Green
}

function Resolve-RipgrepExe {
    $candidates = @()
    $cmd = Get-Command rg -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { $candidates += $cmd.Source }
    if ($env:LOCALAPPDATA) {
        $candidates += Join-Path $env:LOCALAPPDATA "ElonNode\tools\ripgrep\bin\rg.exe"
        $roots = @(
            (Join-Path $env:LOCALAPPDATA "OpenAI\Codex\bin"),
            (Join-Path $env:LOCALAPPDATA "ElonNode\tools\ripgrep")
        )
        foreach ($root in $roots) {
            if (-not (Test-Path -LiteralPath $root -PathType Container)) { continue }
            Get-ChildItem -LiteralPath $root -Recurse -Filter "rg.exe" -File -ErrorAction SilentlyContinue |
                ForEach-Object { $candidates += $_.FullName }
        }
    }
    foreach ($candidate in $candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and
            (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return [System.IO.Path]::GetFullPath($candidate)
        }
    }
    return $null
}

$env:CARGO_TARGET_DIR = Resolve-NodeAgentTargetDir

foreach ($requiredPath in @(
    $ServerManifest,
    (Join-Path $PcFrontendDir "package.json"),
    (Join-Path $RepoRoot "default-project-docs\files"),
    (Join-Path $RepoRoot "scripts\setup-node-env.ps1"),
    (Join-Path $ServerDir "src\node_agent_admin.html"),
    $DesktopShellManifest,
    $BrandIcon
)) {
    if (-not (Test-Path -LiteralPath $requiredPath)) {
        throw "发布目录不完整，缺少必要文件：$requiredPath。请在完整仓库 worktree 中运行。"
    }
}
Assert-NodeAgentBackgroundGitLaunchPolicy -RepoRoot $RepoRoot
$BroadcastAdminToken = ''
$UseRemoteAdminToken = $false
if ($SynchronousRemote) {
    $BroadcastAdminToken = Resolve-NodeAgentAdminToken -ExplicitToken $AdminToken
    if (-not $SkipBroadcast -and [string]::IsNullOrWhiteSpace($BroadcastAdminToken)) {
        if (Test-RemoteNodeAgentAdminToken) {
            $UseRemoteAdminToken = $true
            Write-Host "  本机未设置 ADMIN_TOKEN，将通过服务器本机环境广播更新（token 不回传）。" -ForegroundColor DarkGray
        } else {
            throw "缺少 ADMIN_TOKEN 或 ELON_ADMIN_TOKEN，且服务器环境未找到 ADMIN_TOKEN。若只想上传文件请显式传 -SkipBroadcast。"
        }
    }
}

# 解析真实 target 目录（可能被全局 .cargo/config.toml 的 target-dir 重定向到共享目录）
$meta = cargo metadata --manifest-path $ServerManifest --no-deps --format-version 1 --locked | ConvertFrom-Json
$TargetDir = $meta.target_directory
if (-not $TargetDir) { throw "无法解析 cargo target 目录" }
$PackageVersion = ($meta.packages | Where-Object { $_.name -eq "elon-server" } | Select-Object -First 1).version
if (-not $PackageVersion) { throw "无法解析一龙 PC 节点版本号" }
$ReplayOnlyRequested = -not [string]::IsNullOrWhiteSpace($ReplayPublishedSha)
if (-not $SynchronousRemote -and $ReplayOnlyRequested) {
    throw 'ReplayPublishedSha is a remote operation and requires -SynchronousRemote.'
}
if ($SynchronousRemote -and -not [string]::IsNullOrWhiteSpace($RemoteOutboxEventPath)) {
    $remoteEvent = Read-NodeAgentRemoteReleaseEvent -EventPath $RemoteOutboxEventPath
    $GitSha = [string]$remoteEvent.git_sha
    if ($GitSha -notmatch '^[0-9a-f]{40}$') { throw 'Remote outbox event has an invalid Git SHA.' }
    $headSha = (git -C $RepoRoot rev-parse HEAD).Trim()
    if ($headSha -ne $GitSha) { throw "Remote worker source identity mismatch: head=$headSha event=$GitSha" }
} elseif ($SynchronousRemote) {
    $GitSha = Resolve-NodeAgentPublishSha -RepoRoot $RepoRoot -ReplayPublishedSha $ReplayPublishedSha
} else {
    $GitSha = (git -C $RepoRoot rev-parse HEAD).Trim().ToLowerInvariant()
    $cachedMain = (git -C $RepoRoot rev-parse origin/main).Trim().ToLowerInvariant()
    if ($GitSha -ne $cachedMain) {
        throw "本机发布只接受已推送的缓存 origin/main：HEAD=$GitSha origin/main=$cachedMain。"
    }
    $dirty = @(git -C $RepoRoot status --porcelain=v1 --untracked-files=all)
    if ($dirty.Count -gt 0) { throw '本机发布要求任务 worktree clean，避免构建未提交身份。' }
}
$script:NodeReleaseBatchId = Get-ElonReleaseBatchId -Sha $GitSha
$ReleaseIdentity = Get-NodeAgentReleaseIdentity -Version $PackageVersion -GitSha $GitSha
$ReleaseChangelog = Resolve-NodeAgentChangelog -Explicit $Changelog -Sha $GitSha
$UseOutboxArtifacts = $SynchronousRemote -and -not [string]::IsNullOrWhiteSpace($RemoteOutboxEventPath)
$nodeBuilderId = "$env:COMPUTERNAME-$env:USERNAME"
if ([string]::IsNullOrWhiteSpace($nodeBuilderId) -or $nodeBuilderId -eq '-') {
    $nodeBuilderId = "unknown-node-builder-$([Guid]::NewGuid().ToString('N').Substring(0, 8))" }
if ($SynchronousRemote) {
    $nodeClaim = Enter-ElonNodeAgentPublishLease -ReleaseApiBase "$BaseUrl/api/release" `
        -Sha $GitSha -VersionName $PackageVersion -BuilderId $nodeBuilderId
    if (-not $nodeClaim) { return }
    $claimIsReplay = $nodeClaim.PSObject.Properties.Name -contains 'replayOnly' -and $nodeClaim.replayOnly
    if ($ReplayOnlyRequested -and -not $claimIsReplay) {
        throw 'ReplayPublishedSha was not already published/coalesced; refusing to build or upload.'
    }
    $script:NodeReleaseToken = [string]$nodeClaim.token
    $script:NodeReleaseOwned = -not [string]::IsNullOrWhiteSpace($script:NodeReleaseToken)
    if ($nodeClaim.PSObject.Properties.Name -contains 'batchId' -and -not [string]::IsNullOrWhiteSpace([string]$nodeClaim.batchId)) {
        $script:NodeReleaseBatchId = [string]$nodeClaim.batchId
    }
    if ($claimIsReplay) {
        Invoke-NodeAgentPublishReplay -GitSha $GitSha -PackageVersion $PackageVersion `
            -BatchId $script:NodeReleaseBatchId -ReleaseIdentity $ReleaseIdentity `
            -SkipBroadcast $SkipBroadcast -BroadcastAdminToken $BroadcastAdminToken `
            -UseRemoteAdminToken $UseRemoteAdminToken -HandshakeWaitSec $HandshakeWaitSec -BaseUrl $BaseUrl
        $script:NodeReleaseFinished = $true
        return
    }
    $script:NodeReleaseContext = New-ElonReleaseStageContext -ReleaseApiBase "$BaseUrl/api/release" -Kind 'node_agent' -Token $script:NodeReleaseToken -BatchId $script:NodeReleaseBatchId -Sha $GitSha -Stage 'windows_node'
    $script:NodeReleaseHeartbeat = Start-ElonReleaseContextHeartbeat -Context $script:NodeReleaseContext
}
Write-Host "  target 目录: $TargetDir" -ForegroundColor DarkGray
Write-Host "  发布基线: origin/main@$($GitSha.Substring(0, 7))" -ForegroundColor DarkGray
Write-Host "  发布身份: $ReleaseIdentity" -ForegroundColor DarkGray
if (-not [string]::IsNullOrWhiteSpace($ReleaseChangelog)) {
    Write-Host "  更新内容: $ReleaseChangelog" -ForegroundColor DarkGray
}

$LinuxBin = ''
$LinuxSha256 = ''
if ($SynchronousRemote) {
    # ── 1. 交叉编译 Linux musl 版本（只在异步 worker）────────────────────────
    Write-Host "[remote 1/5] 交叉编译 Linux x86_64-musl..." -ForegroundColor Yellow
    $script:NodeReleaseActiveStage = 'linux_build'
    Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'running'
    $PreviousNodeAgentGitSha = $env:ELON_NODE_AGENT_GIT_SHA
    try {
        $hasZigbuild = $null -ne (Get-Command "cargo-zigbuild" -ErrorAction SilentlyContinue)
        if (-not $hasZigbuild) { $hasZigbuild = $null -ne (cargo zigbuild --version 2>$null) }
        if (-not $hasZigbuild) {
            Write-Host "  安装 cargo-zigbuild..." -ForegroundColor Yellow
            cargo install cargo-zigbuild
            if ($LASTEXITCODE -ne 0) { throw "cargo-zigbuild 安装失败（需先安装 zig 并加入 PATH）" }
        }
        $unitSeparator = [char]0x1f
        $env:CARGO_ENCODED_RUSTFLAGS = "-C${unitSeparator}target-cpu=x86-64"
        $env:ELON_NODE_AGENT_GIT_SHA = $GitSha
        cargo zigbuild --manifest-path $ServerManifest --release --locked --bin $Bin --target x86_64-unknown-linux-musl
        if ($LASTEXITCODE -ne 0) { throw "Linux 编译失败" }
    } finally {
        Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
        if ($null -eq $PreviousNodeAgentGitSha) {
            Remove-Item Env:\ELON_NODE_AGENT_GIT_SHA -ErrorAction SilentlyContinue
        } else {
            $env:ELON_NODE_AGENT_GIT_SHA = $PreviousNodeAgentGitSha
        }
    }
    $LinuxBin = Join-Path $TargetDir "x86_64-unknown-linux-musl\release\$Bin"
    if (-not (Test-Path $LinuxBin)) { throw "Linux 二进制不存在：$LinuxBin" }
    $LinuxSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $LinuxBin).Hash.ToLowerInvariant()
    Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'succeeded'
} else {
    Write-Host '[local 1/4] 默认本机路径跳过 Linux；已交给持久异步 outbox。' -ForegroundColor DarkGray
}

if ($UseOutboxArtifacts) {
    Write-Host '[remote 2/5] 复用本机已验证的不可变 Windows 产物...' -ForegroundColor Yellow
    $LinuxDownloadUrl = "$BaseUrl/api/node-agent/download/linux"
    $WindowsDownloadUrl = "$BaseUrl/api/node-agent/download/windows"
    $WindowsClientDownloadUrl = "$BaseUrl/api/node-agent/download/windows-client"
    $RipgrepDownloadUrl = "$BaseUrl/api/node-agent/download/ripgrep-windows"
    $WinBin = [string]$remoteEvent.artifacts.windows_exe
    $WindowsClientPackage = [string]$remoteEvent.artifacts.windows_client
    $RipgrepPackage = [string]$remoteEvent.artifacts.ripgrep
    $WinSha256 = Get-NodeAgentFileSha256 -Path $WinBin
    $WindowsClientSha256 = Get-NodeAgentFileSha256 -Path $WindowsClientPackage
    if ($WinSha256 -ne [string]$remoteEvent.artifacts.windows_exe_sha256 -or
        $WindowsClientSha256 -ne [string]$remoteEvent.artifacts.windows_client_sha256) {
        throw 'Remote worker durable Windows artifacts failed immutable SHA-256 verification.'
    }
    $RipgrepZipSha256 = ''
    $RipgrepZipFileSize = 0
    if (-not [string]::IsNullOrWhiteSpace($RipgrepPackage)) {
        $RipgrepZipSha256 = Get-NodeAgentFileSha256 -Path $RipgrepPackage
        if ($RipgrepZipSha256 -ne [string]$remoteEvent.artifacts.ripgrep_sha256) {
            throw 'Remote worker durable ripgrep artifact failed immutable SHA-256 verification.'
        }
        $RipgrepZipFileSize = (Get-Item -LiteralPath $RipgrepPackage).Length
    }
} else {
# ── 2. 编译 Windows 版本 ─────────────────────────────────────────────────────
Write-Host "[2/5] 编译 Windows 版本..." -ForegroundColor Yellow
$script:NodeReleaseActiveStage = 'windows_build'
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'running'
$PreviousNodeAgentGitSha = $env:ELON_NODE_AGENT_GIT_SHA
try {
    # 强制通用 CPU，避免全局 target-cpu=native 产出用户机器无法运行的指令。
    $unitSeparator = [char]0x1f
    $env:CARGO_ENCODED_RUSTFLAGS = "-C${unitSeparator}target-cpu=x86-64"
    $env:ELON_NODE_AGENT_GIT_SHA = $GitSha
    Write-Host "  Windows release rustflags: -C target-cpu=x86-64" -ForegroundColor DarkGray
    cargo build --manifest-path $ServerManifest --release --locked --bin $Bin
    if ($LASTEXITCODE -ne 0) { throw "Windows 编译失败" }
} finally {
    Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
    if ($null -eq $PreviousNodeAgentGitSha) {
        Remove-Item Env:\ELON_NODE_AGENT_GIT_SHA -ErrorAction SilentlyContinue
    } else {
        $env:ELON_NODE_AGENT_GIT_SHA = $PreviousNodeAgentGitSha
    }
}

$WinBin = Join-Path $TargetDir "release\$Bin.exe"
if (-not (Test-Path $WinBin)) { throw "Windows 二进制不存在：$WinBin" }
Assert-WindowsExecutableBrandIcon -ExecutablePath $WinBin -ExpectedIconPath $BrandIcon | Out-Null
$WinSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $WinBin).Hash.ToLowerInvariant()
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'succeeded'

# ── 2.2 编译一龙桌面壳（elon-desktop，独立 Tauri crate）──────────────────────
Write-Host "[2.2/5] 编译一龙桌面壳 (elon-desktop)..." -ForegroundColor Yellow
$script:NodeReleaseActiveStage = 'desktop_shell_build'
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'running'
try {
    $unitSeparator = [char]0x1f
    $env:CARGO_ENCODED_RUSTFLAGS = "-C${unitSeparator}target-cpu=x86-64"
    cargo build --manifest-path $DesktopShellManifest --release --locked --bin elon-desktop
    if ($LASTEXITCODE -ne 0) { throw "elon-desktop 编译失败" }
} finally {
    Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
}
$DesktopShellBin = Join-Path $TargetDir "release\elon-desktop.exe"
if (-not (Test-Path $DesktopShellBin)) { throw "elon-desktop 二进制不存在：$DesktopShellBin" }
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'succeeded'

$script:NodeReleaseActiveStage = 'pc_frontend_bundle'
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'running'
Invoke-NodeAgentPcFrontendBuild
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'succeeded'

# ── 2.5 打包 Windows 客户端 ──────────────────────────────────────────────────
Write-Host "[2.5/5] 打包 Windows 客户端..." -ForegroundColor Yellow
$LinuxDownloadUrl = "$BaseUrl/api/node-agent/download/linux"
$WindowsDownloadUrl = "$BaseUrl/api/node-agent/download/windows"
$WindowsClientDownloadUrl = "$BaseUrl/api/node-agent/download/windows-client"
$RipgrepDownloadUrl = "$BaseUrl/api/node-agent/download/ripgrep-windows"
$LauncherDir = Join-Path $PSScriptRoot "node-agent-launcher"
$PackageRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-node-agent-windows-" + [Guid]::NewGuid().ToString("N"))
$PackageInternal = Join-Path $PackageRoot "_internal"
$WindowsClientPackage = Join-Path $TargetDir "release\$WindowsClientPackageName"
$RipgrepPackage = Join-Path $TargetDir "release\$RipgrepPackageName"
$RipgrepZipSha256 = ""
$RipgrepZipFileSize = 0
Write-Host "  打包可选绿色 ripgrep..." -ForegroundColor DarkGray
$RipgrepExe = Resolve-RipgrepExe
if ($RipgrepExe) {
    $RipgrepRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-ripgrep-windows-" + [Guid]::NewGuid().ToString("N"))
    $RipgrepBinDir = Join-Path $RipgrepRoot "bin"
    New-Item -ItemType Directory -Force -Path $RipgrepBinDir | Out-Null
    try {
        Copy-Item -LiteralPath $RipgrepExe -Destination (Join-Path $RipgrepBinDir "rg.exe") -Force
        Compress-ArchiveWithRetry -Path (Join-Path $RipgrepRoot "*") -DestinationPath $RipgrepPackage
        $RipgrepZipSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $RipgrepPackage).Hash.ToLowerInvariant()
        $RipgrepZipFileSize = (Get-Item -LiteralPath $RipgrepPackage).Length
        Write-Host "  ripgrep package sha256 = $RipgrepZipSha256" -ForegroundColor DarkGray
    } finally {
        Remove-Item -LiteralPath $RipgrepRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
} else {
    Write-Host "  未找到 rg.exe，跳过绿色 ripgrep 包；客户端修复时会 fallback 到 winget。" -ForegroundColor DarkYellow
}
New-Item -ItemType Directory -Force -Path $PackageRoot, $PackageInternal | Out-Null
try {
    $PackageClient = Join-Path $PackageRoot "一龙开发平台.exe"
    $PackageUninstall = Join-Path $PackageRoot "卸载一龙开发平台.exe"
    Copy-Item -LiteralPath $WinBin -Destination $PackageClient -Force
    Copy-Item -LiteralPath $WinBin -Destination $PackageUninstall -Force
    Assert-WindowsExecutableBrandIcon -ExecutablePath $PackageClient -ExpectedIconPath $BrandIcon | Out-Null
    Assert-WindowsExecutableBrandIcon -ExecutablePath $PackageUninstall -ExpectedIconPath $BrandIcon | Out-Null
    Copy-Item -LiteralPath $DesktopShellBin -Destination (Join-Path $PackageInternal "elon-desktop.exe") -Force
    foreach ($name in @('node-agent.env.example','README.txt')) { Copy-Item -LiteralPath (Join-Path $LauncherDir $name) -Destination (Join-Path $PackageInternal $name) -Force }
    foreach ($name in @('desktop-review-credential.ps1','new-desktop-review-ticket.ps1')) { Copy-Item -LiteralPath (Join-Path $RepoRoot "scripts\$name") -Destination (Join-Path $PackageInternal $name) -Force }
    $PackagePcDist = Join-Path $PackageInternal "pc-next-dist"
    New-Item -ItemType Directory -Force -Path $PackagePcDist | Out-Null
    Copy-Item -Path (Join-Path $PcDistDir "*") -Destination $PackagePcDist -Recurse -Force
    $PackageVersionInfo = [ordered]@{
        version = $PackageVersion
        gitSha = $GitSha
        changelog = $ReleaseChangelog
        updated_at = (Get-Date).ToString("o")
        downloadUrl = $WindowsDownloadUrl
        linuxDownloadUrl = $LinuxDownloadUrl
        windowsClientDownloadUrl = $WindowsClientDownloadUrl
        sha256 = $WinSha256
        fileSha256 = $WinSha256
        linuxSha256 = $LinuxSha256
        ripgrepZipUrl = $RipgrepDownloadUrl
        ripgrepZipSha256 = $RipgrepZipSha256
        ripgrepZipFileSize = [int64]$RipgrepZipFileSize
    }
    Write-Utf8NoBom `
        -Path (Join-Path $PackageInternal "node-agent-version.json") `
        -Content ($PackageVersionInfo | ConvertTo-Json -Depth 4)
    Compress-ArchiveWithRetry -Path (Join-Path $PackageRoot "*") -DestinationPath $WindowsClientPackage
} finally {
    Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
}
if (-not (Test-Path $WindowsClientPackage)) { throw "Windows 客户端压缩包不存在：$WindowsClientPackage" }
$WindowsClientSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $WindowsClientPackage).Hash.ToLowerInvariant()
}

if (-not $SynchronousRemote) {
    $commonDir = (git -C $RepoRoot rev-parse --git-common-dir).Trim()
    if (-not [System.IO.Path]::IsPathRooted($commonDir)) {
        $commonDir = Join-Path $RepoRoot $commonDir
    }
    $commonDir = [System.IO.Path]::GetFullPath($commonDir)
    $outboxRoot = Get-NodeAgentReleaseOutboxRoot
    $outboxEvent = Add-NodeAgentRemoteReleaseEvent -OutboxRoot $outboxRoot -GitSha $GitSha `
        -Version $PackageVersion -ReleaseIdentity $ReleaseIdentity -Changelog $ReleaseChangelog `
        -WindowsExe $WinBin -WindowsClientPackage $WindowsClientPackage `
        -RipgrepPackage $(if (Test-Path -LiteralPath $RipgrepPackage -PathType Leaf) { $RipgrepPackage } else { '' }) `
        -GitCommonDir $commonDir
    $activationRoot = Get-NodeAgentLocalActivationRoot
    $localRelease = Register-NodeAgentVerifiedLocalRelease -StateRoot $activationRoot `
        -GitSha $GitSha -Version $PackageVersion -ReleaseIdentity $ReleaseIdentity `
        -WindowsClientPackage $WindowsClientPackage -WindowsClientSha256 $WindowsClientSha256
    $activatorPid = $null
    if (-not $SkipLocalActivation) {
        $activatorPid = Start-NodeAgentPostTerminalActivator -StateRoot $activationRoot
    }
    $workerPid = Start-NodeAgentRemoteReleaseWorker -OutboxRoot $outboxRoot
    Write-Host ''
    Write-Host '✅ 本机 Windows release 已验证并进入安全激活队列；远端阶段异步执行。' -ForegroundColor Green
    Write-Output 'NODE_AGENT_LOCAL_PREPARE_STATUS=complete'
    Write-Output 'NODE_AGENT_LOCAL_ACTIVATION_STATUS=restart_scheduled'
    Write-Output "NODE_AGENT_LOCAL_RELEASE_IDENTITY=$ReleaseIdentity"
    Write-Output "NODE_AGENT_LOCAL_RELEASE_STATE=$($localRelease.StatePath)"
    Write-Output "NODE_AGENT_LOCAL_ACTIVATOR_PID=$activatorPid"
    Write-Output 'NODE_AGENT_REMOTE_SYNC_STATE=pending'
    Write-Output "NODE_AGENT_REMOTE_OUTBOX_EVENT=$($outboxEvent.EventPath)"
    Write-Output "NODE_AGENT_REMOTE_WORKER_PID=$workerPid"
    Write-Output 'NODE_AGENT_LOCAL_SERVER_DEPENDENCY=none'
    $script:NodeReleaseFinished = $true
    return
}

# ── 3. 上传到服务器 ───────────────────────────────────────────────────────────
Write-Host "[3/5] 上传到服务器..." -ForegroundColor Yellow
$script:NodeReleaseActiveStage = 'artifact_upload'
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'running'
$currentReleaseSha = (git -C $RepoRoot rev-parse HEAD).Trim()
if ($currentReleaseSha -ne $GitSha) {
    throw "上传前当前 worktree HEAD 已改变：claim=$GitSha, current=$currentReleaseSha。不可替换固定发布 SHA。"
}
ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server "mkdir -p $RemoteDir"
scp -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $LinuxBin "${Server}:${RemoteDir}/${Bin}"
ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server "chmod +x ${RemoteDir}/${Bin}"
scp -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $WinBin "${Server}:${RemoteDir}/${Bin}.exe"
scp -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $WindowsClientPackage "${Server}:${RemoteDir}/${WindowsClientPackageName}"
if ($LASTEXITCODE -ne 0) { throw "上传失败" }
if (Test-Path -LiteralPath $RipgrepPackage -PathType Leaf) {
    scp -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $RipgrepPackage "${Server}:${RemoteDir}/${RipgrepPackageName}"
    if ($LASTEXITCODE -ne 0) { throw "上传 ripgrep 绿色包失败" }
}
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'succeeded'

# ── 4. 验证下载地址 ──────────────────────────────────────────────────────────
Write-Host "[4/5] 验证下载地址..." -ForegroundColor Yellow

$size    = ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server "stat -c '%s' ${RemoteDir}/${Bin}"
$sizeWin = ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server "stat -c '%s' ${RemoteDir}/${Bin}.exe"
$sizeWinClient = ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server "stat -c '%s' ${RemoteDir}/${WindowsClientPackageName}"
if (Test-Path -LiteralPath $RipgrepPackage -PathType Leaf) {
    $RipgrepZipFileSize = [int64](ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $Server "stat -c '%s' ${RemoteDir}/${RipgrepPackageName}")
}
$VersionInfo = [ordered]@{
    version = $PackageVersion
    gitSha = $GitSha
    changelog = $ReleaseChangelog
    updated_at = (Get-Date).ToString("o")
    downloadUrl = $WindowsDownloadUrl
    linuxDownloadUrl = $LinuxDownloadUrl
    windowsClientDownloadUrl = $WindowsClientDownloadUrl
    ripgrepZipUrl = $RipgrepDownloadUrl
    sha256 = $WinSha256
    fileSha256 = $WinSha256
    linuxSha256 = $LinuxSha256
    windowsClientSha256 = $WindowsClientSha256
    ripgrepZipSha256 = $RipgrepZipSha256
    fileSize = [int64]$sizeWin
    linuxFileSize = [int64]$size
    windowsClientFileSize = [int64]$sizeWinClient
    ripgrepZipFileSize = [int64]$RipgrepZipFileSize
}
$VersionFile = New-TemporaryFile
try {
    Write-Utf8NoBom -Path $VersionFile -Content ($VersionInfo | ConvertTo-Json -Depth 4)
    scp -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 $VersionFile "${Server}:${RemoteDir}/node-agent-version.json"
    if ($LASTEXITCODE -ne 0) { throw "版本信息上传失败" }
} finally {
    Remove-Item -LiteralPath $VersionFile -Force -ErrorAction SilentlyContinue
}
Write-Host "  Linux  $Bin size = $size bytes" -ForegroundColor Green
Write-Host "  Linux  $Bin sha256 = $LinuxSha256" -ForegroundColor DarkGray
Write-Host "  Windows $Bin.exe size = $sizeWin bytes" -ForegroundColor Green
Write-Host "  Windows $Bin.exe sha256 = $WinSha256" -ForegroundColor DarkGray
Write-Host "  Windows client package size = $sizeWinClient bytes" -ForegroundColor Green
Write-Host "  Windows client package sha256 = $WindowsClientSha256" -ForegroundColor DarkGray
if ($RipgrepZipFileSize -gt 0) {
    Write-Host "  ripgrep package size = $RipgrepZipFileSize bytes" -ForegroundColor Green
}
Write-Host "  Version info gitSha = $GitSha" -ForegroundColor Green

# ── 5. 推送在线 Windows 节点更新 ──────────────────────────────────────────────
Write-Host "[5/5] 推送在线 Windows 节点更新..." -ForegroundColor Yellow
$script:NodeReleaseActiveStage = 'target_handshake'
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'running'
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

if (-not $SkipBroadcast) {
    Wait-NodePublicDevHandshake `
        -Token $BroadcastAdminToken `
        -UseRemoteToken $UseRemoteAdminToken `
        -TimeoutSec $HandshakeWaitSec `
        -TargetReleaseIdentity $ReleaseIdentity
}
Set-NodeAgentPublishPhase -Phase $script:NodeReleaseActiveStage -Status 'succeeded'

Write-Host ""
Write-Host "✅ 一龙 PC 节点客户端发布完成" -ForegroundColor Green
Write-Host "   下载地址（Linux）:   $LinuxDownloadUrl"
Write-Host "   下载地址（Windows）: $WindowsDownloadUrl"
Write-Host "   客户端包（Windows）: $WindowsClientDownloadUrl"
Write-Host "   ripgrep 绿色包:      $RipgrepDownloadUrl"
if ($script:NodeReleaseOwned) {
    Stop-ElonReleaseHeartbeat -HeartbeatJob $script:NodeReleaseHeartbeat
    $script:NodeReleaseHeartbeat = $null
    Complete-ElonReleaseContext -Context $script:NodeReleaseContext -Success $true -VersionName $PackageVersion
}
$script:NodeReleaseFinished = $true
} catch {
    try { if ($script:NodeReleaseOwned -and -not $script:NodeReleaseFinished) {
            Set-ElonReleasePhase -Context $script:NodeReleaseContext -Phase $script:NodeReleaseActiveStage -Status 'failed'
            Complete-ElonReleaseContext -Context $script:NodeReleaseContext -Success $false -ErrorMessage ($_ | Out-String)
        } } catch {}
    throw
} finally {
    if ($null -ne $script:NodeReleaseHeartbeat) {
        Stop-ElonReleaseHeartbeat -HeartbeatJob $script:NodeReleaseHeartbeat
    }
    Exit-NodeAgentPublishLock -Lock $PublishLock
}
