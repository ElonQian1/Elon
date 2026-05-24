param(
    [switch]$CreateWorktree,
    [switch]$AlwaysCreateWorktree,
    [string]$BranchPrefix = "codex/task",
    [string]$WorktreeParent = ""
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

$repoRoot = GitOutput @("rev-parse", "--show-toplevel")
Set-Location -LiteralPath $repoRoot

$branch = GitOutput @("branch", "--show-current")
$originUrl = (& git remote get-url origin 2>$null)
$hasOrigin = $LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($originUrl)

if ($hasOrigin) {
    & git fetch origin
    if ($LASTEXITCODE -ne 0) {
        throw "git fetch origin failed"
    }
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
} elseif ($needsWorktree) {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=Run powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree before editing."
} else {
    Write-Host "WORKTREE_CREATED=false"
    Write-Host "NEXT=Workspace is clean and current enough for direct edits."
}
