# 清理 ai-task-preflight 留下的孤儿 worktree
#
# 默认（dry-run）：只列出可清理 / 不可清理的项，不做任何修改。
# -Apply：实际执行清理。
# -Force：跳过"分支必须合并进 origin/main"的检查（不推荐）。
# -KeepLast N：保留最近 N 个 task worktree（按时间戳倒序）。
#
# 安全规则（默认所有条件都必须满足才会删除）：
#   1. worktree 路径形如 "<repo>-task-*"，或当前分支形如 "codex/*"
#   2. 工作树没有未提交 / 未跟踪文件
#   3. 当前所在分支已经合并进 origin/main（即没有未推送或未合并的工作）
#   4. 不是当前正在使用的 worktree
#   5. 创建时间超过 -MinAgeMinutes，避免清理刚创建、尚未来得及写入的并行任务
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File scripts\cleanup-task-worktrees.ps1            # 预览
#   powershell -ExecutionPolicy Bypass -File scripts\cleanup-task-worktrees.ps1 -Apply     # 执行

param(
    [switch]$Apply,
    [switch]$Force,
    [int]$KeepLast = 0,
    [int]$MinAgeMinutes = 60,
    [switch]$DeleteRemoteBranches,
    [string[]]$ExcludePath = @()
)

$ErrorActionPreference = "Stop"

function GitOutput {
    param([string[]]$GitArgs)
    $output = & git @GitArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($GitArgs -join ' ') failed: $output"
    }
    return ($output -join "`n").Trim()
}

function Get-GitFetchFailureHint {
    param([string]$Output)

    $text = if ($Output) { $Output } else { "" }
    if ($text -match '(Could not resolve host|Name or service not known|Temporary failure in name resolution)') {
        return "网络/DNS 无法解析 GitHub，请检查网络、DNS 或代理后重试。"
    }
    if ($text -match '(Failed to connect|Connection timed out|Connection reset|Connection refused|Operation timed out|HTTP/2 stream|early EOF|The remote end hung up unexpectedly)') {
        return "网络连接到 GitHub 不稳定或超时，通常是临时抖动；脚本已短重试但仍失败。"
    }
    if ($text -match '(Permission denied|Authentication failed|Repository not found|Could not read from remote repository|Host key verification failed|publickey)') {
        return "Git 远端认证或仓库权限异常，请检查 SSH key、GitHub 权限和 origin 地址。"
    }
    return "Git fetch 失败，原因未能自动分类；请查看原始输出。"
}

function Invoke-GitFetchWithRetry {
    param(
        [int]$Attempts = 3,
        [int]$DelaySeconds = 2
    )

    $lastOutput = ""
    for ($i = 1; $i -le $Attempts; $i++) {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & git fetch origin 2>&1
        } finally {
            $ErrorActionPreference = $oldPreference
        }
        $lastOutput = ($output -join "`n").Trim()
        if ($LASTEXITCODE -eq 0) {
            if ($i -gt 1) {
                Write-Host "GIT_FETCH_RETRY=success_after_$i"
            }
            return
        }
        $hint = Get-GitFetchFailureHint -Output $lastOutput
        Write-Host "GIT_FETCH_RETRY=attempt_$i/$Attempts failed: $hint" -ForegroundColor Yellow
        if ($i -lt $Attempts) { Start-Sleep -Seconds $DelaySeconds }
    }

    $finalHint = Get-GitFetchFailureHint -Output $lastOutput
    throw "git fetch origin failed after $Attempts attempts. $finalHint 原始输出：$lastOutput"
}

function Normalize-WorktreePath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    try {
        return ([System.IO.Path]::GetFullPath($Path).TrimEnd('\','/') -replace '\\','/')
    } catch {
        return ($Path.TrimEnd('\','/') -replace '\\','/')
    }
}

$repoRoot = GitOutput @("rev-parse", "--show-toplevel")
Set-Location -LiteralPath $repoRoot

$repoLeaf = Split-Path -Leaf $repoRoot
$currentWorktree = Normalize-WorktreePath (Resolve-Path -LiteralPath ".").Path
$excludeSet = @{}
foreach ($path in $ExcludePath) {
    if ([string]::IsNullOrWhiteSpace($path)) { continue }
    $fullPath = if ([System.IO.Path]::IsPathRooted($path)) {
        [System.IO.Path]::GetFullPath($path)
    } else {
        [System.IO.Path]::GetFullPath((Join-Path $repoRoot $path))
    }
    $excludeSet[(Normalize-WorktreePath $fullPath)] = $true
}

# 同步远端，确保 origin/main 是最新的
Invoke-GitFetchWithRetry

# 解析所有已注册 worktree
$entries = @()
$current = @{}
foreach ($line in (& git worktree list --porcelain)) {
    if ($line -eq "") {
        if ($current.Count -gt 0) { $entries += [pscustomobject]$current; $current = @{} }
        continue
    }
    $kv = $line -split " ", 2
    switch ($kv[0]) {
        "worktree" { $current["Path"] = $kv[1] }
        "HEAD"     { $current["Head"] = $kv[1] }
        "branch"   { $current["Branch"] = ($kv[1] -replace "^refs/heads/","") }
        "bare"     { $current["Bare"] = $true }
        "detached" { $current["Detached"] = $true }
    }
}
if ($current.Count -gt 0) { $entries += [pscustomobject]$current }

# 只保留 AI 任务 worktree。
# 早期脚本只按 "<repo>-task-*" 目录名识别，导致 elon-win-client-*、
# elon-routeb-* 这类功能命名的 codex worktree 无法被自动回收。
$taskStampPattern = "\d{8}-\d{6}(-[A-Za-z0-9]+(-[A-Fa-f0-9]+)?)?"
$pattern = "^$([regex]::Escape($repoLeaf))-task-$taskStampPattern(-task-$taskStampPattern)?$"
$taskWorktrees = $entries | Where-Object {
    $_.Path -and (
        ((Split-Path -Leaf $_.Path) -match $pattern) -or
        ($_.Branch -and $_.Branch -like "codex/*")
    )
} | Sort-Object Path

if ($KeepLast -gt 0 -and $taskWorktrees.Count -gt $KeepLast) {
    $keep = $taskWorktrees | Sort-Object { Split-Path -Leaf $_.Path } -Descending | Select-Object -First $KeepLast
    $keepSet = @{}; foreach ($k in $keep) { $keepSet[$k.Path] = $true }
} else {
    $keepSet = @{}
}

$toRemove = @()
$kept = @()

foreach ($wt in $taskWorktrees) {
    $reasons = @()

    $normalized = Normalize-WorktreePath $wt.Path
    if ($normalized -ieq $currentWorktree) { $reasons += "当前正在使用" }
    if ($keepSet.ContainsKey($wt.Path))    { $reasons += "在 -KeepLast 保留范围内" }
    if ($excludeSet.ContainsKey($normalized)) { $reasons += "在 -ExcludePath 保护范围内" }

    if ($MinAgeMinutes -gt 0 -and (Test-Path -LiteralPath $wt.Path)) {
        $createdUtc = (Get-Item -LiteralPath $wt.Path -Force).CreationTimeUtc
        $ageMinutes = ([DateTime]::UtcNow - $createdUtc).TotalMinutes
        if ($ageMinutes -lt $MinAgeMinutes) {
            $reasons += "近期创建保护(MinAgeMinutes=$MinAgeMinutes, age=$([Math]::Round($ageMinutes, 1)))"
        }
    }

    if (-not (Test-Path -LiteralPath $wt.Path)) {
        $reasons += "目录已不存在（可 prune）"
    } else {
        # 检查脏状态
        $statusOut = (& git -C $wt.Path status --short 2>&1)
        if ($LASTEXITCODE -ne 0) {
            $reasons += "git status 失败: $statusOut"
        } elseif (-not [string]::IsNullOrWhiteSpace(($statusOut -join "`n"))) {
            $reasons += "有未提交/未跟踪改动"
        }

        # 检查分支是否合并进 origin/main
        if (-not $Force -and $wt.Branch) {
            & git merge-base --is-ancestor $wt.Branch origin/main 2>&1 | Out-Null
            if ($LASTEXITCODE -ne 0) {
                $reasons += "分支 $($wt.Branch) 尚未合并进 origin/main（用 -Force 跳过）"
            }
        }
    }

    if ($reasons.Count -eq 0) {
        $toRemove += $wt
    } else {
        $kept += [pscustomobject]@{ Worktree = $wt; Reasons = $reasons }
    }
}

Write-Host "=== 扫描结果 ===" -ForegroundColor Cyan
Write-Host "可清理: $($toRemove.Count) 个"
Write-Host "保留:   $($kept.Count) 个"
Write-Host ""

if ($toRemove.Count -gt 0) {
    Write-Host "[将被删除]" -ForegroundColor Yellow
    foreach ($wt in $toRemove) {
        Write-Host "  $($wt.Path)  ($($wt.Branch))"
    }
    Write-Host ""
}

if ($kept.Count -gt 0) {
    Write-Host "[保留]" -ForegroundColor Green
    foreach ($k in $kept) {
        Write-Host "  $($k.Worktree.Path)  ($($k.Worktree.Branch))"
        foreach ($r in $k.Reasons) { Write-Host "    - $r" }
    }
    Write-Host ""
}

if (-not $Apply) {
    Write-Host "预览模式。如要执行，请加 -Apply。" -ForegroundColor Cyan
    return
}

if ($toRemove.Count -eq 0) {
    Write-Host "无需清理。" -ForegroundColor Green
    & git worktree prune
    return
}

Write-Host "=== 开始清理 ===" -ForegroundColor Cyan
$removed = 0; $failed = 0
foreach ($wt in $toRemove) {
    try {
        Write-Host "removing $($wt.Path)" -ForegroundColor Yellow
        & git worktree remove --force $wt.Path 2>&1 | ForEach-Object { Write-Host "  $_" }
        if ($LASTEXITCODE -ne 0) { throw "worktree remove failed" }

        if ($wt.Branch) {
            $deleteFlag = if ($Force) { "-D" } else { "-d" }
            & git branch $deleteFlag $wt.Branch 2>&1 | ForEach-Object { Write-Host "  $_" }
            if ($DeleteRemoteBranches) {
                & git push origin --delete $wt.Branch 2>&1 | ForEach-Object { Write-Host "  $_" }
            }
        }
        $removed++
    } catch {
        Write-Host "  失败: $_" -ForegroundColor Red
        $failed++
    }
}

& git worktree prune

Write-Host ""
Write-Host "完成：清理 $removed 个，失败 $failed 个。" -ForegroundColor Green
