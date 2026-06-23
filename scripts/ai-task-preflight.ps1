param(
    [switch]$CreateWorktree,
    [switch]$AlwaysCreateWorktree,
    [string]$BranchPrefix = "codex/task",
    [string]$WorktreeParent = "",
    [switch]$SkipAutoCleanup
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
    return "Git fetch 失败，原因未能自动分类；请查看下方原始输出。"
}

function Invoke-GitFetchWithRetry {
    param(
        [string[]]$GitArgs = @("fetch", "origin"),
        [int]$Attempts = 3,
        [int]$DelaySeconds = 2
    )

    $lastOutput = ""
    for ($i = 1; $i -le $Attempts; $i++) {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & git @GitArgs 2>&1
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
        if ($i -lt $Attempts) {
            Start-Sleep -Seconds $DelaySeconds
        }
    }

    $finalHint = Get-GitFetchFailureHint -Output $lastOutput
    throw "git $($GitArgs -join ' ') failed after $Attempts attempts. $finalHint`n原始输出：$lastOutput"
}

$repoRoot = GitOutput @("rev-parse", "--show-toplevel")
Set-Location -LiteralPath $repoRoot

$branch = GitOutput @("branch", "--show-current")
$originUrl = (& git remote get-url origin 2>$null)
$hasOrigin = $LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($originUrl)

if ($hasOrigin) {
    Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin")
}

$statusShort = (& git status --short)
$isDirty = -not [string]::IsNullOrWhiteSpace(($statusShort -join "`n"))
$behind = 0
$ahead = 0
if ($hasOrigin) {
    & git rev-parse --verify origin/main *> $null
    if ($LASTEXITCODE -eq 0) {
        $counts = GitOutput @("rev-list", "--left-right", "--count", "HEAD...origin/main")
        $parts = $counts -split "\s+"
        if ($parts.Length -ge 2) {
            $ahead = [int]$parts[0]
            $behind = [int]$parts[1]
        }
    }
}

Write-Host "REPO_ROOT=$repoRoot"
Write-Host "BRANCH=$branch"
Write-Host "DIRTY=$isDirty"
Write-Host "AHEAD=$ahead"
Write-Host "BEHIND=$behind"

if ($isDirty) {
    Write-Host "Changed files:"
    $statusShort | ForEach-Object { Write-Host "  $_" }
}

$needsWorktree = $AlwaysCreateWorktree -or $isDirty -or ($behind -gt 0)
$createdWorktree = $false
if (($CreateWorktree -or $AlwaysCreateWorktree) -and $needsWorktree) {
    if (-not $hasOrigin) {
        throw "Cannot create isolated worktree: origin remote is missing"
    }

    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $safePrefix = $BranchPrefix.TrimEnd("/")
    $newBranch = "$safePrefix-$stamp"
    $parent = if ([string]::IsNullOrWhiteSpace($WorktreeParent)) {
        Split-Path -Parent $repoRoot
    } else {
        $WorktreeParent
    }
    $leaf = "$(Split-Path -Leaf $repoRoot)-task-$stamp"
    $worktreePath = Join-Path $parent $leaf

    & git worktree add -b $newBranch $worktreePath origin/main
    if ($LASTEXITCODE -ne 0) {
        throw "git worktree add failed"
    }

    Write-Host "WORKTREE_CREATED=true"
    Write-Host "WORKTREE_BRANCH=$newBranch"
    Write-Host "WORKTREE_PATH=$worktreePath"
    Write-Host "NEXT=cd `"$worktreePath`""
    $createdWorktree = $true
} elseif ($needsWorktree) {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=Run powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree before editing."
} else {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=Workspace is clean and current enough for direct edits."
}

# ─────────────────────────────────────────────────────────────
# 自动清理已合并、工作树干净的孤儿 task worktree（防止累积）
# 仅删除满足"已合并到 origin/main + 无未提交内容 + 不是当前 worktree"的，
# 有未提交改动的会被自动保留。要禁用：-SkipAutoCleanup
# ─────────────────────────────────────────────────────────────
if ($createdWorktree -and -not $SkipAutoCleanup) {
    Write-Host "AUTO_CLEANUP=skipped_after_worktree_create"
} elseif (-not $SkipAutoCleanup) {
    $cleanupScript = Join-Path $repoRoot "scripts\cleanup-task-worktrees.ps1"
    if (Test-Path -LiteralPath $cleanupScript) {
        try {
            $cleanupOut = & powershell -NoProfile -ExecutionPolicy Bypass -File $cleanupScript -Apply 2>&1
            $removedLine = $cleanupOut | Select-String -Pattern "^完成：清理" | Select-Object -Last 1
            if ($removedLine) {
                Write-Host "AUTO_CLEANUP=$($removedLine.Line.Trim())"
            } else {
                Write-Host "AUTO_CLEANUP=skipped"
            }
        } catch {
            Write-Host "AUTO_CLEANUP=failed: $_"
        }
    }
}
