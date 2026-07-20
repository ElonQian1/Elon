$ErrorActionPreference = "Stop"

$repoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$finishScript = Join-Path $repoRoot "scripts\finish-ai-task.ps1"
$checkScript = Join-Path $repoRoot "scripts\check-task-complete.ps1"
$cleanupScript = Join-Path $repoRoot "scripts\cleanup-task-worktrees.ps1"
$directNetworkScript = Join-Path $repoRoot "scripts\direct-network.ps1"
$policyFile = Join-Path $repoRoot ".ai\workspace-policy.txt"

$finishSource = Get-Content -Raw -LiteralPath $finishScript
if (-not $finishSource.Contains('worktree", "unlock"')) {
    throw "PowerShell finish must unlock its completed managed worktree before removal."
}
$finishShellSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "scripts\finish-ai-task.sh")
if (-not $finishShellSource.Contains('worktree unlock "$task_root"')) {
    throw "Shell finish must unlock its completed managed worktree before removal."
}

$checkSource = Get-Content -Raw -LiteralPath $checkScript
$serverGateStart = $checkSource.IndexOf('if ($Kind -eq "Server" -or $Kind -eq "PcFrontend")')
$serverHealthStart = $checkSource.IndexOf('    try {', $serverGateStart)
if ($serverGateStart -lt 0 -or $serverHealthStart -lt 0) {
    throw "Server/PcFrontend completion gate could not be located."
}
$serverPushGate = $checkSource.Substring($serverGateStart, $serverHealthStart - $serverGateStart)
if (-not $serverPushGate.Contains('git merge-base --is-ancestor $head $originMain')) {
    throw "Server/PcFrontend completion must accept a task HEAD already contained in a newer origin/main."
}
if ($serverPushGate.Contains('$head -ne $originMain')) {
    throw "Server/PcFrontend completion must not chase unrelated commits that landed after deployment."
}

function Invoke-Git {
    param(
        [string]$Path,
        [string[]]$GitArgs
    )
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & git -C $Path @GitArgs 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    if ($exitCode -ne 0) {
        throw "git $($GitArgs -join ' ') failed in $Path`: $($output -join ' ')"
    }
    return ($output -join "`n").Trim()
}

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Expected,
        [string]$Message
    )
    if (-not $Text.Contains($Expected)) {
        throw "$Message Missing: $Expected`nActual output:`n$Text"
    }
}

function Invoke-Finish {
    param(
        [string]$WorktreePath,
        [switch]$ExpectFailure,
        [switch]$PerformCleanup
    )

    $finishArgs = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $WorktreePath "scripts\finish-ai-task.ps1"),
        "-Kind", "CodePushed",
        "-TaskWorktree", $WorktreePath
    )
    if (-not $PerformCleanup) {
        $finishArgs += "-SkipWorktreeCleanup"
    }

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell @finishArgs 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }

    $text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    if ($ExpectFailure -and $exitCode -eq 0) {
        throw "finish-ai-task unexpectedly succeeded.`n$text"
    }
    if (-not $ExpectFailure -and $exitCode -ne 0) {
        throw "finish-ai-task failed unexpectedly.`n$text"
    }
    return $text
}

$tempBase = [System.IO.Path]::GetTempPath()
$testRoot = Join-Path $tempBase ("elon-finish-workflow-test-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testRoot | Out-Null

try {
    $originPath = Join-Path $testRoot "origin.git"
    $mainRepo = Join-Path $testRoot "main"
    $taskWorktree = Join-Path $testRoot "task-worktree"
    $peerWorktree = Join-Path $testRoot "peer-worktree"
    $platformSessionWorktree = Join-Path $testRoot "conversation-worktrees\elon-self\cleanup-session"

    & git init --bare $originPath *> $null
    if ($LASTEXITCODE -ne 0) { throw "git init --bare failed" }
    & git init -b main $mainRepo *> $null
    if ($LASTEXITCODE -ne 0) { throw "git init main failed" }

    Invoke-Git $mainRepo @("config", "user.email", "finish-test@example.invalid") | Out-Null
    Invoke-Git $mainRepo @("config", "user.name", "finish-test") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $mainRepo "scripts") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $mainRepo ".ai") | Out-Null
    Copy-Item -LiteralPath $finishScript -Destination (Join-Path $mainRepo "scripts\finish-ai-task.ps1")
    Copy-Item -LiteralPath $checkScript -Destination (Join-Path $mainRepo "scripts\check-task-complete.ps1")
    Copy-Item -LiteralPath $cleanupScript -Destination (Join-Path $mainRepo "scripts\cleanup-task-worktrees.ps1")
    Copy-Item -LiteralPath $directNetworkScript -Destination (Join-Path $mainRepo "scripts\direct-network.ps1")
    Copy-Item -LiteralPath $policyFile -Destination (Join-Path $mainRepo ".ai\workspace-policy.txt")
    Set-Content -LiteralPath (Join-Path $mainRepo "README.md") -Value "finish workflow fixture`n" -Encoding UTF8

    Invoke-Git $mainRepo @("add", "README.md", "scripts", ".ai") | Out-Null
    Invoke-Git $mainRepo @("commit", "-m", "seed finish workflow fixture") | Out-Null
    Invoke-Git $mainRepo @("remote", "add", "origin", $originPath) | Out-Null
    Invoke-Git $mainRepo @("push", "-u", "origin", "main") | Out-Null
    Invoke-Git $mainRepo @("worktree", "add", "-b", "codex/finish-fixture", $taskWorktree, "origin/main") | Out-Null
    Invoke-Git $mainRepo @("worktree", "add", "-b", "codex/peer-fixture", $peerWorktree, "origin/main") | Out-Null
    New-Item -ItemType Directory -Path (Split-Path -Parent $platformSessionWorktree) -Force | Out-Null
    Invoke-Git $mainRepo @("worktree", "add", "-b", "ai/session/elon-self/cleanup-session", $platformSessionWorktree, "origin/main") | Out-Null
    Invoke-Git $taskWorktree @("config", "user.email", "finish-test@example.invalid") | Out-Null
    Invoke-Git $taskWorktree @("config", "user.name", "finish-test") | Out-Null

    Add-Content -LiteralPath (Join-Path $taskWorktree "README.md") -Value "task change"
    Invoke-Git $taskWorktree @("add", "README.md") | Out-Null
    Invoke-Git $taskWorktree @("commit", "-m", "finish workflow task change") | Out-Null
    Invoke-Git $taskWorktree @("push", "origin", "HEAD:main") | Out-Null

    # Global cleanup can run from another concurrent task immediately after a
    # new worktree is created. Both recent clean worktrees must survive that
    # scan even though their branches are already ancestors of origin/main.
    Invoke-Git $mainRepo @("worktree", "lock", "--reason", "active workflow fixture", $peerWorktree) | Out-Null
    Push-Location -LiteralPath $mainRepo
    try {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $recentCleanupOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $mainRepo "scripts\cleanup-task-worktrees.ps1") -Apply 2>&1
            $recentCleanupExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldPreference
        }
    } finally {
        Pop-Location
    }
    $recentCleanupText = (($recentCleanupOutput | ForEach-Object { [string]$_ }) -join "`n").Trim()
    if ($recentCleanupExitCode -ne 0) { throw "Recent-worktree cleanup fixture failed.`n$recentCleanupText" }
    Assert-Contains $recentCleanupText "MinAgeMinutes=60" "Global cleanup must protect newly created concurrent worktrees."
    $registeredAfterRecentCleanup = Invoke-Git $mainRepo @("worktree", "list", "--porcelain")
    if (-not $registeredAfterRecentCleanup.Contains("branch refs/heads/codex/finish-fixture")) { throw "Global cleanup removed the newly created task worktree.`n$recentCleanupText" }
    if (-not $registeredAfterRecentCleanup.Contains("branch refs/heads/codex/peer-fixture")) { throw "Global cleanup removed the newly created peer worktree.`n$recentCleanupText" }
    if (-not $registeredAfterRecentCleanup.Contains("branch refs/heads/ai/session/elon-self/cleanup-session")) { throw "Global cleanup removed the newly created platform session worktree.`n$recentCleanupText" }

    # A legacy untracked source-looking file must not block the tracked main
    # baseline from catching up, and must never be auto-added or deleted.
    $legacyPath = Join-Path $mainRepo "legacy-test.rs"
    Set-Content -LiteralPath $legacyPath -Value "diagnostic only" -Encoding UTF8
    $beforeMain = Invoke-Git $mainRepo @("rev-parse", "HEAD")
    $originMain = Invoke-Git $mainRepo @("rev-parse", "origin/main")
    if ($beforeMain -eq $originMain) { throw "Fixture main must start behind origin/main." }

    $successOutput = Invoke-Finish -WorktreePath $taskWorktree
    Assert-Contains $successOutput "LOCAL_MAIN_STATUS=current:" "Finish must report a current main baseline."
    Assert-Contains $successOutput "MAIN_UNTRACKED_STATUS_PATH=legacy-test.rs|candidate_track" "Finish must classify source-looking untracked files without mutating them."
    Assert-Contains $successOutput "FINALIZABLE=true" "Finish must allow final reporting after main is safely synchronized."

    $afterMain = Invoke-Git $mainRepo @("rev-parse", "HEAD")
    $originMain = Invoke-Git $mainRepo @("rev-parse", "origin/main")
    if ($afterMain -ne $originMain) { throw "Local main did not fast-forward to origin/main." }
    if (-not (Test-Path -LiteralPath $legacyPath)) { throw "Unknown main untracked file was deleted unexpectedly." }

    # Platform-managed conversation worktrees are already isolated from the
    # checked-out main baseline. Unknown tracked main edits must remain visible
    # and untouched, but they must not turn a clean, pushed platform task back
    # into an unfinished business result.
    Add-Content -LiteralPath (Join-Path $mainRepo "README.md") -Value "unknown platform-owned main edit"
    $platformFinishOutput = Invoke-Finish -WorktreePath $platformSessionWorktree
    Assert-Contains $platformFinishOutput "BUSINESS_STATUS=complete" "A platform session must retain its completed business state."
    Assert-Contains $platformFinishOutput "LOCAL_MAIN_STATUS=blocked_tracked_changes" "A platform session must report the dirty main baseline."
    Assert-Contains $platformFinishOutput "TASK_WORKTREE_STATUS=platform_managed" "A platform session must remain platform-managed."
    Assert-Contains $platformFinishOutput "FINALIZABLE=true" "Unknown main edits must not block a clean, pushed platform session."
    $dirtyMainContent = Get-Content -Raw -LiteralPath (Join-Path $mainRepo "README.md")
    if (-not $dirtyMainContent.Contains("unknown platform-owned main edit")) {
        throw "Platform finish mutated the unknown tracked main edit."
    }
    Invoke-Git $mainRepo @("restore", "--source=HEAD", "--", "README.md") | Out-Null

    # A same-path remote addition must be left to Git's overwrite protection.
    # The finish gate reports the already-complete business state separately
    # from the blocked local-main cleanup state.
    $mainCollisionPath = Join-Path $mainRepo "collision-test.rs"
    $taskCollisionPath = Join-Path $taskWorktree "collision-test.rs"
    Set-Content -LiteralPath $mainCollisionPath -Value "unknown local content" -Encoding UTF8
    Set-Content -LiteralPath $taskCollisionPath -Value "intentional tracked content" -Encoding UTF8
    Invoke-Git $taskWorktree @("add", "collision-test.rs") | Out-Null
    Invoke-Git $taskWorktree @("commit", "-m", "add collision fixture") | Out-Null
    Invoke-Git $taskWorktree @("push", "origin", "HEAD:main") | Out-Null

    $collisionOutput = Invoke-Finish -WorktreePath $taskWorktree -ExpectFailure
    Assert-Contains $collisionOutput "BUSINESS_STATUS=complete" "A local-main collision must not erase the completed remote business state."
    Assert-Contains $collisionOutput "LOCAL_MAIN_STATUS=sync_failed" "A same-path untracked collision must block only local-main synchronization."
    Assert-Contains $collisionOutput "FINALIZABLE=false" "A same-path collision must remain visible to the task owner."
    $collisionContent = Get-Content -Raw -LiteralPath $mainCollisionPath
    if (-not $collisionContent.Contains("unknown local content")) {
        throw "Same-path unknown main file was overwritten unexpectedly."
    }

    Remove-Item -LiteralPath $mainCollisionPath -Force
    $collisionRecoveryOutput = Invoke-Finish -WorktreePath $taskWorktree
    Assert-Contains $collisionRecoveryOutput "LOCAL_MAIN_STATUS=current:" "Finish must recover after the owner resolves a same-path collision."
    Assert-Contains $collisionRecoveryOutput "FINALIZABLE=true" "Resolved local-main collision must become finalizable."

    # Declared temporary roots are the only untracked content eligible for
    # automatic deletion.
    $temporaryRoot = Join-Path $taskWorktree ".ai-tmp"
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    Set-Content -LiteralPath (Join-Path $temporaryRoot "transient.log") -Value "temporary" -Encoding UTF8
    $tempOutput = Invoke-Finish -WorktreePath $taskWorktree
    Assert-Contains $tempOutput "ARTIFACT_CLEANUP=task:.ai-tmp/" "Declared task temporary root must be cleaned."
    if (Test-Path -LiteralPath $temporaryRoot) { throw "Declared temporary root still exists after finish." }

    # Untracked files outside the declared temporary root are a hard stop in
    # the task worktree and receive a deterministic disposition hint.
    $unresolvedPath = Join-Path $taskWorktree "new_behavior_test.rs"
    Set-Content -LiteralPath $unresolvedPath -Value "#[test]" -Encoding UTF8
    $failureOutput = Invoke-Finish -WorktreePath $taskWorktree -ExpectFailure
    Assert-Contains $failureOutput "TASK_UNRESOLVED_PATH=new_behavior_test.rs|candidate_track" "Source/test files must be classified as candidate_track."
    Assert-Contains $failureOutput "FINALIZABLE=false" "Dirty task worktree must block final reporting."

    Remove-Item -LiteralPath $unresolvedPath -Force
    $cleanupOutput = Invoke-Finish -WorktreePath $taskWorktree -PerformCleanup
    Assert-Contains $cleanupOutput "TASK_WORKTREE_STATUS=cleaned" "Unified finish must remove its merged Codex task worktree."
    Assert-Contains $cleanupOutput "FINALIZABLE=true" "Cleaned task worktree must be finalizable."
    $registered = Invoke-Git $mainRepo @("worktree", "list", "--porcelain")
    if ($registered.Contains($taskWorktree)) { throw "Task worktree is still registered after unified finish cleanup." }
    if (-not $registered.Contains("branch refs/heads/codex/peer-fixture")) { throw "Unified finish removed another agent's merged worktree." }

    Push-Location -LiteralPath $mainRepo
    try {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $platformCleanupOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $mainRepo "scripts\cleanup-task-worktrees.ps1") -Apply -MinAgeMinutes 0 2>&1
            $platformCleanupExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldPreference
        }
    } finally {
        Pop-Location
    }
    $platformCleanupText = (($platformCleanupOutput | ForEach-Object { [string]$_ }) -join "`n").Trim()
    if ($platformCleanupExitCode -ne 0) { throw "Platform-session cleanup fixture failed.`n$platformCleanupText" }
    Assert-Contains $platformCleanupText "ai/session/elon-self/cleanup-session" "Cleanup must include merged clean platform session worktrees."
    Assert-Contains $platformCleanupText "active workflow fixture" "Cleanup must preserve locked active task worktrees."
    $registeredAfterPlatformCleanup = Invoke-Git $mainRepo @("worktree", "list", "--porcelain")
    if ($registeredAfterPlatformCleanup.Contains("branch refs/heads/ai/session/elon-self/cleanup-session")) { throw "Platform session worktree is still registered after cleanup.`n$platformCleanupText" }
    if ($registeredAfterPlatformCleanup.Contains($platformSessionWorktree)) { throw "Platform session path is still registered after cleanup.`n$platformCleanupText" }
    if (-not $registeredAfterPlatformCleanup.Contains("branch refs/heads/codex/peer-fixture")) { throw "Locked peer worktree was removed by platform cleanup." }

    Write-Host "PASS ai-task-finish workflow guard"
} finally {
    $resolved = Resolve-Path -LiteralPath $testRoot -ErrorAction SilentlyContinue
    if ($resolved) {
        $resolvedPath = $resolved.Path
        $tempFullPath = [System.IO.Path]::GetFullPath($tempBase)
        $leaf = Split-Path -Leaf $resolvedPath
        if ($resolvedPath.StartsWith($tempFullPath, [StringComparison]::OrdinalIgnoreCase) -and $leaf.StartsWith("elon-finish-workflow-test-")) {
            Remove-Item -LiteralPath $resolvedPath -Recurse -Force
        } else {
            Write-Warning "Skip cleanup for unexpected test path: $resolvedPath"
        }
    }
}
