param(
    [switch]$CreateWorktree,
    [switch]$AlwaysCreateWorktree,
    [string]$BranchPrefix = "codex/task",
    [string]$WorktreeParent = "",
    [switch]$SkipAutoCleanup
)

$ErrorActionPreference = "Stop"

$directNetworkScript = Join-Path $PSScriptRoot "direct-network.ps1"
if (Test-Path -LiteralPath $directNetworkScript) {
    . $directNetworkScript
    Set-ElonProjectDirectNetwork
}

function GitOutput {
    param([string[]]$GitArgs)
    $output = & git @GitArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($GitArgs -join ' ') failed: $output"
    }
    return ($output -join "`n").Trim()
}

function GitOutputInPath {
    param(
        [string]$Path,
        [string[]]$GitArgs
    )
    $output = & git -C $Path @GitArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git -C `"$Path`" $($GitArgs -join ' ') failed: $output"
    }
    return ($output -join "`n").Trim()
}

function Get-GitWorktreeEntries {
    $entries = @()
    $current = @{}
    foreach ($line in (& git worktree list --porcelain)) {
        if ($line -eq "") {
            if ($current.Count -gt 0) {
                $entries += [pscustomobject]$current
                $current = @{}
            }
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
    return $entries
}

function Sync-LocalMainBaseline {
    param([bool]$HasOrigin)

    if (-not $HasOrigin) {
        Write-Host "MAIN_BASELINE_SYNC=skipped_no_origin"
        return
    }

    & git rev-parse --verify origin/main *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "MAIN_BASELINE_SYNC=skipped_no_origin_main"
        return
    }

    & git worktree prune *> $null

    $mainWorktree = Get-GitWorktreeEntries |
        Where-Object { $_.Branch -eq "main" -and $_.Path } |
        Select-Object -First 1

    if ($mainWorktree) {
        $mainPath = [string]$mainWorktree.Path
        $gitMarker = Join-Path $mainPath ".git"
        if (Test-Path -LiteralPath $gitMarker) {
            $status = (& git -C $mainPath status --porcelain=v1 --untracked-files=normal)
            if ($LASTEXITCODE -ne 0) {
                Write-Host "MAIN_BASELINE_SYNC=blocked_status_failed:$mainPath"
                return
            }
            if (-not [string]::IsNullOrWhiteSpace(($status -join "`n"))) {
                Write-Host "MAIN_BASELINE_SYNC=blocked_dirty:$mainPath"
                return
            }

            GitOutputInPath -Path $mainPath -GitArgs @("merge", "--ff-only", "origin/main") | Out-Null
            Write-Host "MAIN_BASELINE_SYNC=synced_worktree:$mainPath"
            return
        }
    }

    & git show-ref --verify --quiet refs/heads/main
    if ($LASTEXITCODE -eq 0) {
        GitOutput @("branch", "--force", "main", "origin/main") | Out-Null
        Write-Host "MAIN_BASELINE_SYNC=synced_ref"
    } else {
        GitOutput @("branch", "main", "origin/main") | Out-Null
        Write-Host "MAIN_BASELINE_SYNC=created_ref"
    }
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

function Write-AiWorkflowGuard {
    param(
        [string]$EditRoot,
        [string]$State
    )

    Write-Host "AI_WORKFLOW_GUARD_BEGIN"
    Write-Host "EDIT_ROOT=$EditRoot"
    Write-Host "EDIT_STATE=$State"
    Write-Host "RULE_MAIN_BASELINE=main checkout is sync-only; do not edit business files in main."
    Write-Host "RULE_BEFORE_EDIT=cd to EDIT_ROOT/WORKTREE_PATH and run git status --short --branch before editing."
    Write-Host "RULE_PUSH=after commit run git push origin HEAD:main, then scripts\check-task-complete.ps1 -Kind CodePushed."
    Write-Host "RULE_FINISH=after push sync the main baseline with git pull --ff-only and run scripts\cleanup-task-worktrees.ps1 -Apply."
    Write-Host "AI_WORKFLOW_GUARD_END"
}

$repoRoot = GitOutput @("rev-parse", "--show-toplevel")
Set-Location -LiteralPath $repoRoot

$branch = GitOutput @("branch", "--show-current")
$originUrl = (& git remote get-url origin 2>$null)
$hasOrigin = $LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($originUrl)

if ($hasOrigin) {
    if (Get-Command Set-ElonProjectDirectGitSsh -ErrorAction SilentlyContinue) {
        Set-ElonProjectDirectGitSsh
    }
    Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin")
    Sync-LocalMainBaseline -HasOrigin $true
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

$isMainBaseline = $branch -eq "main"
$needsWorktree = $AlwaysCreateWorktree -or $isDirty -or ($behind -gt 0) -or $isMainBaseline
$createdWorktree = $false
$createdWorktreePath = ""
if (($CreateWorktree -or $AlwaysCreateWorktree) -and $needsWorktree) {
    if (-not $hasOrigin) {
        throw "Cannot create isolated worktree: origin remote is missing"
    }

    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $shortGuid = ((New-Guid).Guid -replace "-", "").Substring(0, 8)
    $uniqueSuffix = "$PID-$shortGuid"
    $safePrefix = $BranchPrefix.TrimEnd("/")
    $newBranch = "$safePrefix-$stamp-$uniqueSuffix"
    $parent = if ([string]::IsNullOrWhiteSpace($WorktreeParent)) {
        Split-Path -Parent $repoRoot
    } else {
        $WorktreeParent
    }
    $leaf = "$(Split-Path -Leaf $repoRoot)-task-$stamp-$uniqueSuffix"
    $worktreePath = Join-Path $parent $leaf

    & git worktree add -b $newBranch $worktreePath origin/main
    if ($LASTEXITCODE -ne 0) {
        throw "git worktree add failed"
    }

    Write-Host "WORKTREE_CREATED=true"
    Write-Host "WORKTREE_BRANCH=$newBranch"
    Write-Host "WORKTREE_PATH=$worktreePath"
    Write-Host "WORKTREE_BASE=$(git rev-parse --short origin/main)"
    Write-Host "NEXT=cd `"$worktreePath`""
    Write-AiWorkflowGuard -EditRoot $worktreePath -State "created_worktree"
    $createdWorktree = $true
    $createdWorktreePath = $worktreePath
} elseif ($needsWorktree) {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=Run powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree before editing."
    Write-AiWorkflowGuard -EditRoot "BLOCKED_CREATE_WORKTREE_FIRST" -State "blocked_needs_worktree"
} else {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=Workspace is already isolated and current enough for direct edits."
    Write-AiWorkflowGuard -EditRoot $repoRoot -State "current_worktree_ok"
}

# ─────────────────────────────────────────────────────────────
# 自动清理已合并、工作树干净的孤儿 task worktree（防止累积）
# 仅删除满足"已合并到 origin/main + 无未提交内容 + 不是当前 worktree"的，
# 有未提交改动的会被自动保留。要禁用：-SkipAutoCleanup
# ─────────────────────────────────────────────────────────────
if (-not $SkipAutoCleanup -and $createdWorktree) {
    # A newly-created worktree starts clean and already merged with origin/main.
    # Running cleanup after creation can race with the just-emitted edit root, so
    # leave cleanup to task finish or the next preflight run.
    Write-Host "AUTO_CLEANUP=skipped_created_worktree"
} elseif (-not $SkipAutoCleanup) {
    $cleanupScript = Join-Path $repoRoot "scripts\cleanup-task-worktrees.ps1"
    if (Test-Path -LiteralPath $cleanupScript) {
        try {
            $cleanupArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $cleanupScript, "-Apply")
            $cleanupOut = & powershell @cleanupArgs 2>&1
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
