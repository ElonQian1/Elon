param(
    [switch]$CreateWorktree,
    [switch]$AlwaysCreateWorktree,
    [string]$BranchPrefix = "codex/task",
    [string]$WorktreeParent = "",
    [switch]$SkipAutoCleanup
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot 'ai-task-finish-contract.ps1')

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
            $status = (& git -C $mainPath status --porcelain=v1 --untracked-files=no)
            if ($LASTEXITCODE -ne 0) {
                Write-Host "MAIN_BASELINE_SYNC=blocked_status_failed:$mainPath"
                return
            }
            if (-not [string]::IsNullOrWhiteSpace(($status -join "`n"))) {
                Write-Host "MAIN_BASELINE_SYNC=blocked_tracked_changes:$mainPath"
                return
            }

            $untracked = @(& git -C $mainPath -c core.quotePath=false status --porcelain=v1 --untracked-files=all) |
                Where-Object { $_ -like "?? *" }
            if ($untracked.Count -gt 0) {
                Write-Host "MAIN_BASELINE_UNTRACKED=warning:$($untracked.Count)"
            } else {
                Write-Host "MAIN_BASELINE_UNTRACKED=clean"
            }

            $oldPreference = $ErrorActionPreference
            $ErrorActionPreference = "Continue"
            try {
                $mergeOutput = & git -C $mainPath merge --ff-only origin/main 2>&1
                $mergeExitCode = $LASTEXITCODE
            } finally {
                $ErrorActionPreference = $oldPreference
            }
            if ($mergeExitCode -ne 0) {
                Write-Host "MAIN_BASELINE_SYNC=failed:${mainPath}:$($mergeOutput -join ' ')"
                return
            }
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

function Invoke-GitFetchWithRetry {
    param(
        [string]$RepoPath = ".",
        [string[]]$GitArgs = @("fetch", "origin")
    )

    $result = Invoke-ElonGitHubGitWithProxyFallback -RepoPath $RepoPath -GitArgs $GitArgs -RemoteName "origin"
    Write-Host "GITHUB_SSH_ROUTE=$($result.Route)"
    if ($result.ExitCode -ne 0) {
        throw "git $($GitArgs -join ' ') failed. $($result.Hint)`n$($result.Text)"
    }
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
    Write-Host "RULE_OUTPUT=commands expected to exceed 200 lines must use scripts\invoke-ai-logged-command.ps1; never stream full successful build/test/publish logs into AI context."
    Write-Host "RULE_BEFORE_COMMIT=run scripts\check-source-size.ps1 and scripts\check-document-modularity.ps1 before git commit; pre-commit/pre-push repeat the document guard."
    Write-Host "RULE_PUSH=after commit run powershell -NoProfile -ExecutionPolicy Bypass -File scripts\direct-network.ps1 push origin HEAD:main; only a non-fast-forward rejection triggers fetch and rebase."
    $contractId = ''
    if ($State -ne 'blocked_needs_worktree' -and (Test-Path -LiteralPath $EditRoot)) {
        $contractId = New-AiTaskFinishContract -RepoPath $EditRoot
        Write-Host "FINISH_CONTRACT_SCHEMA=elon.ai_finish_contract.v1"
        Write-Host "FINISH_CONTRACT_ID=$contractId"
    }
    Write-Host "RULE_FINISH=after push run the exact FINISH_COMMAND_POWERSHELL; it validates the preflight identity, verifies origin/main, syncs main, audits artifacts, and cleans the task worktree."
    Write-Host "FINISH_COMMAND_POWERSHELL=powershell -NoProfile -ExecutionPolicy Bypass -File scripts\finish-ai-task.ps1 -Kind CodePushed -TaskWorktree `"$EditRoot`" -TaskContract $contractId"
    Write-Host "FINISH_COMMAND_SHELL=bash scripts/finish-ai-task.sh --kind CodePushed"
    Write-Host "AI_WORKFLOW_GUARD_END"
}

function Test-PcConversationWorktree {
    param(
        [string]$RepoRoot,
        [string]$Branch
    )

    $normalizedRepoRoot = ($RepoRoot -replace "\\", "/").TrimEnd("/")
    $isConversationPath = $normalizedRepoRoot -match '(^|/)conversation-worktrees/[^/]+/[^/]+(/|$)'
    $isSessionBranch = $Branch -match '^ai/session/[^/]+/[^/]+$'
    return $isConversationPath -or $isSessionBranch
}

function Resolve-AiTaskWorktreeRoot {
    param(
        [string]$RepoRoot,
        [string]$ExplicitParent
    )

    if (-not [string]::IsNullOrWhiteSpace($ExplicitParent)) {
        return [System.IO.Path]::GetFullPath($ExplicitParent)
    }
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_AI_WORKTREE_ROOT)) {
        if (-not [System.IO.Path]::IsPathRooted($env:ELON_AI_WORKTREE_ROOT)) {
            throw "ELON_AI_WORKTREE_ROOT must be an absolute path."
        }
        return [System.IO.Path]::GetFullPath($env:ELON_AI_WORKTREE_ROOT)
    }

    $driveRoot = [System.IO.Path]::GetPathRoot($RepoRoot)
    if ([string]::IsNullOrWhiteSpace($driveRoot)) {
        throw "Cannot determine a short worktree root for repository: $RepoRoot"
    }
    return Join-Path $driveRoot "wt"
}

function Lock-AiTaskWorktree {
    param([string]$RepoPath, [string]$WorktreePath)

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = @(& git -C $RepoPath worktree lock --reason "active Codex task; finish-ai-task unlocks" $WorktreePath 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    $text = ($output -join "`n")
    if ($exitCode -ne 0 -and $text -notmatch "already locked") {
        throw "Unable to lock active task worktree: $text"
    }
    Write-Host "WORKTREE_LOCKED=true"
}

$repoRoot = GitOutput @("rev-parse", "--show-toplevel")
Set-Location -LiteralPath $repoRoot

$branch = GitOutput @("branch", "--show-current")
$originUrl = (& git remote get-url origin 2>$null)
$hasOrigin = $LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($originUrl)

if ($hasOrigin) {
    Invoke-GitFetchWithRetry -RepoPath $repoRoot -GitArgs @("fetch", "origin")
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

$isPcConversationWorktree = Test-PcConversationWorktree -RepoRoot $repoRoot -Branch $branch
if ($isPcConversationWorktree) {
    Write-Host "PC_CONVERSATION_WORKTREE=true"
}

if ($isDirty) {
    Write-Host "Changed files:"
    $statusShort | ForEach-Object { Write-Host "  $_" }
}

$isMainBaseline = $branch -eq "main"
$needsWorktree = -not $isPcConversationWorktree -and ($AlwaysCreateWorktree -or $isDirty -or ($behind -gt 0) -or $isMainBaseline)
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
    $parent = Resolve-AiTaskWorktreeRoot -RepoRoot $repoRoot -ExplicitParent $WorktreeParent
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $leaf = $uniqueSuffix
    $worktreePath = Join-Path $parent $leaf

    & git worktree add -b $newBranch $worktreePath origin/main
    if ($LASTEXITCODE -ne 0) {
        throw "git worktree add failed"
    }
    Lock-AiTaskWorktree -RepoPath $repoRoot -WorktreePath $worktreePath

    Write-Host "WORKTREE_CREATED=true"
    Write-Host "WORKTREE_BRANCH=$newBranch"
    Write-Host "WORKTREE_ROOT=$parent"
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
} elseif ($isPcConversationWorktree) {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=PC conversation worktree is already isolated; use the current workspace for direct edits."
    Write-AiWorkflowGuard -EditRoot $repoRoot -State "pc_conversation_worktree_ok"
} else {
    if ($branch -like "codex/*") {
        Lock-AiTaskWorktree -RepoPath $repoRoot -WorktreePath $repoRoot
    }
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
