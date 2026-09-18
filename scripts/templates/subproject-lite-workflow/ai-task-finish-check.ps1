<#
.SYNOPSIS
    子项目轻量任务收尾校验：确认工作区干净、本地分支与 origin 一致（已推送）。
.DESCRIPTION
    在准备宣布"完成"前运行；不做 worktree 回收、不做发布判定，只确认代码
    真的已经进入远端主线，避免"改完就说完成但忘了 push"。
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
$statusShort = & git status --short
$isDirty = -not [string]::IsNullOrWhiteSpace(($statusShort -join "`n"))

Write-Host "REPO_ROOT=$repoRoot"
Write-Host "BRANCH=$branch"
Write-Host "DIRTY=$isDirty"

if ($isDirty) {
    Write-Host "Changed files:"
    $statusShort | ForEach-Object { Write-Host "  $_" }
    Write-Host "FINALIZABLE=false"
    Write-Host "REASON=工作区仍有未提交改动，先 commit 再 push"
    exit 0
}

$originUrl = (& git remote get-url origin 2>$null)
$hasOrigin = $LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($originUrl)
if (-not $hasOrigin) {
    Write-Host "FINALIZABLE=false"
    Write-Host "REASON=没有 origin remote，无法确认是否已推送"
    exit 0
}

& git fetch origin 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "git fetch origin failed" }

$localHead = GitOutput @("rev-parse", "HEAD")
$remoteRef = if ($branch) { "origin/$branch" } else { "origin/HEAD" }
& git rev-parse --verify $remoteRef *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "FINALIZABLE=false"
    Write-Host "REASON=远端没有对应分支 $remoteRef，请先 push"
    exit 0
}
$remoteHead = GitOutput @("rev-parse", $remoteRef)

if ($localHead -ne $remoteHead) {
    Write-Host "FINALIZABLE=false"
    Write-Host "REASON=本地 HEAD($($localHead.Substring(0,7))) 与 $remoteRef($($remoteHead.Substring(0,7))) 不一致，请先 push 或处理冲突"
    exit 0
}

Write-Host "FINALIZABLE=true"
Write-Host "HEAD=$($localHead.Substring(0,7))"
Write-Host "SYNCED_WITH=$remoteRef"
