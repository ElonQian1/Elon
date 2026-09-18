<#
.SYNOPSIS
    子项目轻量任务预检：同步 main 基线、报告脏/领先/落后状态。
.DESCRIPTION
    不建隔离 worktree，适合单代理顺序开发的子项目。任务开始前运行一次；
    如果发现工作区脏或落后，先按提示处理，再开始改代码。
#>
param()

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
    & git fetch origin 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "git fetch origin failed" }
}

$statusShort = & git status --short
$isDirty = -not [string]::IsNullOrWhiteSpace(($statusShort -join "`n"))

$ahead = 0; $behind = 0
if ($hasOrigin -and $branch -eq "main") {
    & git rev-parse --verify origin/main *> $null
    if ($LASTEXITCODE -eq 0) {
        $counts = GitOutput @("rev-list", "--left-right", "--count", "HEAD...origin/main")
        $parts = $counts -split "\s+"
        if ($parts.Length -ge 2) { $ahead = [int]$parts[0]; $behind = [int]$parts[1] }
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
    Write-Host "NEXT=有未提交改动来源不明时先确认归属，不要自动覆盖或丢弃；确认是自己的未完成工作可以继续编辑。"
    exit 0
}

if ($behind -gt 0) {
    if ($branch -ne "main") {
        Write-Host "NEXT=当前不在 main，且 main 落后 origin/main $behind 个提交；不要在此分支上 rebase/合并 main，先确认分支意图。"
        exit 0
    }
    $merge = & git merge --ff-only origin/main 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "SYNC_FAILED=true"
        Write-Host "DETAIL=$merge"
        throw "无法快进同步到 origin/main：$merge"
    }
    Write-Host "SYNC=fast_forwarded"
} else {
    Write-Host "SYNC=already_current"
}

Write-Host "NEXT=工作区干净且已同步，可以开始改动；每完成一个独立改动就 git add + commit + git push origin HEAD:main，不要攒到任务结束才一次性提交。"
