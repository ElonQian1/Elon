$ErrorActionPreference = "Stop"

$repoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$finishScript = Join-Path $repoRoot "scripts\finish-ai-task.ps1"
$checkScript = Join-Path $repoRoot "scripts\check-task-complete.ps1"
$cleanupScript = Join-Path $repoRoot "scripts\cleanup-task-worktrees.ps1"
$directNetworkScript = Join-Path $repoRoot "scripts\direct-network.ps1"
$finishContractScript = Join-Path $repoRoot "scripts\ai-task-finish-contract.ps1"
$terminalFinalizationScript = Join-Path $repoRoot "scripts\ai-task-terminal-finalization.ps1"
$terminalFinalizationReceiptScript = Join-Path $repoRoot "scripts\ai-task-terminal-finalization-receipt.ps1"
$policyFile = Join-Path $repoRoot ".ai\workspace-policy.txt"

$finishSource = Get-Content -Raw -LiteralPath $finishScript
$terminalFinalizationSource = Get-Content -Raw -LiteralPath $terminalFinalizationScript
if (-not $terminalFinalizationSource.Contains("@('worktree', 'unlock'")) {
    throw "PowerShell finish must unlock its completed managed worktree before removal."
}
$finishShellSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "scripts\finish-ai-task.sh")
if (-not $finishShellSource.Contains('worktree unlock "$task_root"')) {
    throw "Shell finish must unlock its completed managed worktree before removal."
}
if (-not $terminalFinalizationSource.Contains('Get-AiTerminalLeaseObservation') -or
    -not $terminalFinalizationSource.Contains('intentionally adjacent to unlock') -or
    -not $finishShellSource.Contains('ai_finish_worktree_lease_reason')) {
    throw "Platform finish must inspect the exact immutable supervision lease before unlock."
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

function Invoke-GitCaptureResult {
    param([string]$Path, [string[]]$GitArgs)
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& git -C $Path @GitArgs 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    [pscustomobject]@{
        ExitCode = $exitCode
        Text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    }
}

function Assert-ReceiptBytesUnchanged {
    param([string]$Path, [string]$Before, [string]$Message)
    $after = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($Path))
    if ($after -ne $Before) { throw $Message }
}

function Invoke-Finish {
    param(
        [string]$WorktreePath,
        [switch]$ExpectFailure,
        [switch]$PerformCleanup,
        [switch]$NoLegacy,
        [ValidateSet('None','Missing','Foreign','Reacquire')][string]$LeaseMutation = 'None',
        [ValidateSet('None','Missing','Foreign','Reacquire')][string]$InitialLeaseMutation = 'None',
        [switch]$TestFailAfterUnlock,
        [string]$ContractId = ''
    )

    $finishArgs = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $WorktreePath "scripts\finish-ai-task.ps1"),
        "-Kind", "CodePushed",
        "-TaskWorktree", $WorktreePath
    )
    if (-not [string]::IsNullOrWhiteSpace($ContractId)) {
        $finishArgs += @('-TaskContract', $ContractId)
    } elseif (-not $NoLegacy) {
        $finishArgs += "-AllowLegacyNoTaskContract"
    }
    if (-not $PerformCleanup) {
        $finishArgs += "-SkipWorktreeCleanup"
    }
    if ($LeaseMutation -ne 'None') {
        $finishArgs += @('-TestLeaseMutationAfterIdentity', $LeaseMutation)
    }
    if ($InitialLeaseMutation -ne 'None') {
        $finishArgs += @('-TestLeaseMutationBeforeFinalization', $InitialLeaseMutation)
    }
    if ($TestFailAfterUnlock) {
        $finishArgs += '-TestFailAfterPlatformUnlock'
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
    Copy-Item -LiteralPath $finishContractScript -Destination (Join-Path $mainRepo "scripts\ai-task-finish-contract.ps1")
    Copy-Item -LiteralPath $terminalFinalizationScript -Destination (Join-Path $mainRepo "scripts\ai-task-terminal-finalization.ps1")
    Copy-Item -LiteralPath $terminalFinalizationReceiptScript -Destination (Join-Path $mainRepo "scripts\ai-task-terminal-finalization-receipt.ps1")
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

    . (Join-Path $taskWorktree 'scripts\ai-task-finish-contract.ps1')
    $taskContractId = New-AiTaskFinishContract -RepoPath $taskWorktree
    $missingContractOutput = Invoke-Finish -WorktreePath $taskWorktree -ExpectFailure -NoLegacy
    Assert-Contains $missingContractOutput 'requires the immutable TaskContract' 'Managed task finish must fail closed without its preflight contract.'
    $successOutput = Invoke-Finish -WorktreePath $taskWorktree -ContractId $taskContractId
    Assert-Contains $successOutput "FINISH_CONTRACT_STATUS=validated:$taskContractId" "Finish must validate the exact preflight contract."
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
    Invoke-Git $mainRepo @("worktree", "lock", "--reason", "elon-supervision:platform-fixture", $platformSessionWorktree) | Out-Null
    . (Join-Path $platformSessionWorktree 'scripts\ai-task-finish-contract.ps1')
    . (Join-Path $platformSessionWorktree 'scripts\ai-task-terminal-finalization.ps1')
    $platformContractId = New-AiTaskFinishContract -RepoPath $platformSessionWorktree
    $platformReceiptPath = Get-AiTerminalFinalizationReceiptPath -TaskContract $platformContractId `
        -RootTaskId 'platform-fixture'

    Invoke-Git $mainRepo @("worktree", "unlock", $platformSessionWorktree) | Out-Null
    Invoke-Git $mainRepo @("worktree", "lock", "--reason", "elon-supervision:wrong-root", $platformSessionWorktree) | Out-Null
    $wrongRootOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure -ContractId $platformContractId
    Assert-Contains $wrongRootOutput "lease identity mismatch" "Platform finish must fail closed for a wrong-root supervision lease."
    Invoke-Git $mainRepo @("worktree", "unlock", $platformSessionWorktree) | Out-Null
    Invoke-Git $mainRepo @("worktree", "lock", "--reason", "foreign-workflow-lock", $platformSessionWorktree) | Out-Null
    $foreignLeaseOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure -ContractId $platformContractId
    Assert-Contains $foreignLeaseOutput "lease identity mismatch" "Platform finish must fail closed for an unknown foreign lease."
    Invoke-Git $mainRepo @("worktree", "unlock", $platformSessionWorktree) | Out-Null
    $missingLeaseOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure -ContractId $platformContractId
    Assert-Contains $missingLeaseOutput "requires exact/exact" "Receipt-null finish must reject initial/fresh missing lease observations."
    if (Test-Path -LiteralPath $platformReceiptPath) {
        throw 'Receipt-null missing lease failure persisted a receipt.'
    }
    Invoke-Git $mainRepo @("worktree", "lock", "--reason", "elon-supervision:platform-fixture", $platformSessionWorktree) | Out-Null

    foreach ($case in @(
        @{ Name = 'foreign'; Initial = 'Foreign'; Fresh = 'None' },
        @{ Name = 'exact-to-missing'; Initial = 'None'; Fresh = 'Missing' },
        @{ Name = 'same-root-reacquire'; Initial = 'None'; Fresh = 'Reacquire' }
    )) {
        $caseOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure `
            -ContractId $platformContractId -InitialLeaseMutation $case.Initial -LeaseMutation $case.Fresh
        Assert-Contains $caseOutput 'FINALIZABLE=false' "Receipt-null $($case.Name) mutation must fail closed."
        if (Test-Path -LiteralPath $platformReceiptPath) {
            throw "Receipt-null $($case.Name) mutation persisted a receipt."
        }
        $actual = Get-AiTaskWorktreeLeaseReason -RepoPath $platformSessionWorktree
        if (-not [string]::IsNullOrWhiteSpace($actual)) {
            Invoke-Git $mainRepo @('worktree', 'unlock', $platformSessionWorktree) | Out-Null
        }
        Invoke-Git $mainRepo @('worktree', 'lock', '--reason', 'elon-supervision:platform-fixture', $platformSessionWorktree) | Out-Null
    }

    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $platformReceiptPath)) | Out-Null
    [System.IO.File]::WriteAllText($platformReceiptPath, '{"schema":"wrong"}', [Text.UTF8Encoding]::new($false))
    $malformedBytes = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($platformReceiptPath))
    $malformedReceiptOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure -ContractId $platformContractId
    Assert-Contains $malformedReceiptOutput 'receipt schema fields' 'Malformed existing receipt must never be overwritten.'
    Assert-ReceiptBytesUnchanged $platformReceiptPath $malformedBytes 'Malformed receipt failure changed receipt bytes.'
    Remove-Item -LiteralPath $platformReceiptPath -Force

    function New-PreparedReceiptFixture {
        $currentLease = Get-AiTaskWorktreeLeaseReason -RepoPath $platformSessionWorktree
        if (-not [string]::IsNullOrWhiteSpace($currentLease)) {
            Invoke-Git $mainRepo @('worktree', 'unlock', $platformSessionWorktree) | Out-Null
        }
        Invoke-Git $mainRepo @('worktree', 'lock', '--reason', 'elon-supervision:platform-fixture', $platformSessionWorktree) | Out-Null
        if (Test-Path -LiteralPath $platformReceiptPath) { Remove-Item -LiteralPath $platformReceiptPath -Force }
        $validated = Assert-AiTaskFinishContract -RepoPath $platformSessionWorktree -ContractId $platformContractId
        $identity = Get-AiTerminalFinalizationIdentity -TaskRoot $platformSessionWorktree -BasePath $mainRepo `
            -TaskContract $platformContractId -ValidatedContract $validated
        $observation = Get-AiTerminalLeaseObservation -RepoPath $platformSessionWorktree
        $prepared = New-AiPreparedTerminalFinalizationReceipt -Identity $identity `
            -LeaseMarkerFingerprint $observation.MarkerFingerprint
        Write-AiTerminalFinalizationReceipt -Path $platformReceiptPath -Receipt $prepared
        [pscustomobject]@{
            Bytes = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($platformReceiptPath))
            FinalizationId = [string]$prepared.finalizationId
        }
    }

    foreach ($case in @(
        @{ Name = 'exact-to-missing'; Initial = 'None'; Fresh = 'Missing' },
        @{ Name = 'foreign-to-missing'; Initial = 'Foreign'; Fresh = 'Missing' },
        @{ Name = 'same-root-reacquired-to-missing'; Initial = 'Reacquire'; Fresh = 'Missing' },
        @{ Name = 'exact-marker-drift'; Initial = 'Reacquire'; Fresh = 'None' }
    )) {
        $preparedFixture = New-PreparedReceiptFixture
        $preparedFailure = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure `
            -ContractId $platformContractId -InitialLeaseMutation $case.Initial -LeaseMutation $case.Fresh
        Assert-Contains $preparedFailure 'FINALIZABLE=false' "Prepared $($case.Name) mutation must fail closed."
        Assert-ReceiptBytesUnchanged $platformReceiptPath $preparedFixture.Bytes "Prepared $($case.Name) mutation changed receipt bytes."
    }

    # Existing prepared + initial exact + fresh exact with the same durable
    # marker is the positive recovery path: this invocation owns the unlock.
    $preparedExact = New-PreparedReceiptFixture
    $preparedExactOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ContractId $platformContractId
    Assert-Contains $preparedExactOutput "TERMINAL_FINALIZATION_STATUS=prepared:$($preparedExact.FinalizationId)" 'Prepared exact/exact recovery must preserve its finalization id.'
    Assert-Contains $preparedExactOutput "TERMINAL_FINALIZATION_STATUS=completed:$($preparedExact.FinalizationId)" 'Prepared exact/exact recovery must complete the same finalization.'
    Assert-Contains $preparedExactOutput 'FINALIZABLE=true' 'Prepared exact/exact recovery must be finalizable.'
    $preparedExactReceipt = Read-AiTerminalFinalizationReceipt -Path $platformReceiptPath
    if ($preparedExactReceipt.state -ne 'completed' -or $preparedExactReceipt.finalizationId -ne $preparedExact.FinalizationId) {
        throw 'Prepared exact/exact recovery did not complete the original receipt.'
    }
    if (-not [string]::IsNullOrWhiteSpace((Get-AiTaskWorktreeLeaseReason -RepoPath $platformSessionWorktree))) {
        throw 'Prepared exact/exact recovery did not release the platform lease.'
    }
    $completedIdentity = [pscustomobject]@{ Fields = [ordered]@{
        taskContractId = [string]$preparedExactReceipt.taskContractId
        supervisionRootTaskId = [string]$preparedExactReceipt.supervisionRootTaskId
        worktree = [string]$preparedExactReceipt.worktree
        baseWorkspace = [string]$preparedExactReceipt.baseWorkspace
        gitDir = [string]$preparedExactReceipt.gitDir
        gitCommonDir = [string]$preparedExactReceipt.gitCommonDir
        branch = [string]$preparedExactReceipt.branch
        origin = [string]$preparedExactReceipt.origin
        finalHead = [string]$preparedExactReceipt.finalHead
    } }
    foreach ($invalidTerminalStatus in @('failed', 'canceled')) {
        $invalidBinding = ($preparedExactReceipt | ConvertTo-Json -Depth 8 -Compress | ConvertFrom-Json)
        $invalidBinding.taskId = 'fixture-task'
        $invalidBinding.completionEventId = 'fixture-event'
        $invalidBinding.terminalStatus = $invalidTerminalStatus
        $invalidBinding.boundAtUtc = [DateTime]::UtcNow.ToString('o')
        $rejected = $false
        try {
            Assert-AiTerminalFinalizationReceipt -Receipt $invalidBinding -Identity $completedIdentity `
                -TaskContract $platformContractId -RootTaskId 'platform-fixture'
        } catch {
            $rejected = $_.Exception.Message -like '*must be done*'
        }
        if (-not $rejected) {
            throw "Completed receipt accepted forbidden terminal binding: $invalidTerminalStatus"
        }
    }

    # Preserve the crash-after-unlock path: a receipt-null exact/exact run
    # persists prepared, verifies no lease, then a none/none replay completes.
    $null = New-PreparedReceiptFixture
    Remove-Item -LiteralPath $platformReceiptPath -Force
    $interruptedOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure `
        -ContractId $platformContractId -TestFailAfterUnlock
    Assert-Contains $interruptedOutput 'missing-lease verification' 'Unlock interruption must occur only after immediate missing-lease verification.'
    $interruptedReceipt = Read-AiTerminalFinalizationReceipt -Path $platformReceiptPath
    if ($null -eq $interruptedReceipt -or $interruptedReceipt.state -ne 'prepared') {
        throw 'Unlock interruption lost the prepared receipt.'
    }
    $interruptedFinalizationId = [string]$interruptedReceipt.finalizationId
    if (-not [string]::IsNullOrWhiteSpace((Get-AiTaskWorktreeLeaseReason -RepoPath $platformSessionWorktree))) {
        throw 'Unlock interruption unexpectedly retained a lease.'
    }
    $platformFinishOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ContractId $platformContractId
    Assert-Contains $platformFinishOutput "TERMINAL_FINALIZATION_STATUS=completed:$interruptedFinalizationId" 'Prepared none/none replay must complete the interrupted finalization.'
    Assert-Contains $platformFinishOutput "BUSINESS_STATUS=complete" "A platform session must retain its completed business state."
    Assert-Contains $platformFinishOutput "LOCAL_MAIN_STATUS=blocked_tracked_changes" "A platform session must report the dirty main baseline."
    Assert-Contains $platformFinishOutput "TASK_WORKTREE_STATUS=platform_managed" "A platform session must remain platform-managed."
    Assert-Contains $platformFinishOutput "TASK_WORKTREE_LEASE_STATUS=released" "A completed platform session must release its execution lease even when shared main is dirty."
    Assert-Contains $platformFinishOutput "FINALIZABLE=true" "Unknown main edits must not block a clean, pushed platform session."

    $completedBytes = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($platformReceiptPath))
    foreach ($case in @(
        @{ Name = 'exact-to-missing'; Initial = 'Reacquire'; Fresh = 'Missing' },
        @{ Name = 'foreign-to-missing'; Initial = 'Foreign'; Fresh = 'Missing' },
        @{ Name = 'missing-to-reacquired'; Initial = 'None'; Fresh = 'Reacquire' },
        @{ Name = 'exact-marker-drift'; Initial = 'Reacquire'; Fresh = 'None' }
    )) {
        $completedFailure = Invoke-Finish -WorktreePath $platformSessionWorktree -ExpectFailure `
            -ContractId $platformContractId -InitialLeaseMutation $case.Initial -LeaseMutation $case.Fresh
        Assert-Contains $completedFailure 'FINALIZABLE=false' "Completed $($case.Name) mutation must fail closed."
        Assert-ReceiptBytesUnchanged $platformReceiptPath $completedBytes "Completed $($case.Name) mutation changed receipt bytes."
        $actual = Get-AiTaskWorktreeLeaseReason -RepoPath $platformSessionWorktree
        if (-not [string]::IsNullOrWhiteSpace($actual)) {
            Invoke-Git $mainRepo @('worktree', 'unlock', $platformSessionWorktree) | Out-Null
        }
    }

    $completedReplayOutput = Invoke-Finish -WorktreePath $platformSessionWorktree -ContractId $platformContractId
    Assert-Contains $completedReplayOutput "TERMINAL_FINALIZATION_STATUS=completed:$interruptedFinalizationId" 'Completed none/none replay must be idempotent.'
    Assert-ReceiptBytesUnchanged $platformReceiptPath $completedBytes 'Completed replay changed receipt bytes.'

    $platformRegistration = Invoke-Git $mainRepo @("worktree", "list", "--porcelain")
    $platformEntry = ($platformRegistration -split "`n`n" | Where-Object { $_.Contains($platformSessionWorktree) }) -join "`n"
    if ($platformEntry -match '(?m)^locked(?: |$)') {
        throw "Platform finish left the completed task lease locked.`n$platformEntry"
    }
    $dirtyMainContent = Get-Content -Raw -LiteralPath (Join-Path $mainRepo "README.md")
    if (-not $dirtyMainContent.Contains("unknown platform-owned main edit")) {
        throw "Platform finish mutated the unknown tracked main edit."
    }
    Invoke-Git $mainRepo @("restore", "--source=HEAD", "--", "README.md") | Out-Null

    # A same-path remote addition must be left to Git's overwrite protection.
    # The finish gate reports the already-complete business state separately
    # from the blocked local-main cleanup state.
    Invoke-Git $peerWorktree @('config', 'user.email', 'finish-test@example.invalid') | Out-Null
    Invoke-Git $peerWorktree @('config', 'user.name', 'finish-test') | Out-Null
    Invoke-Git $peerWorktree @('fetch', 'origin') | Out-Null
    Invoke-Git $peerWorktree @('rebase', 'origin/main') | Out-Null
    Set-Content -LiteralPath (Join-Path $peerWorktree 'peer-advance.txt') -Value 'force a real non-fast-forward fixture' -Encoding UTF8
    Invoke-Git $peerWorktree @('add', 'peer-advance.txt') | Out-Null
    Invoke-Git $peerWorktree @('commit', '-m', 'advance collision fixture origin') | Out-Null
    Invoke-Git $peerWorktree @('push', 'origin', 'HEAD:main') | Out-Null

    $mainCollisionPath = Join-Path $mainRepo "collision-test.rs"
    $taskCollisionPath = Join-Path $taskWorktree "collision-test.rs"
    Set-Content -LiteralPath $mainCollisionPath -Value "unknown local content" -Encoding UTF8
    Set-Content -LiteralPath $taskCollisionPath -Value "intentional tracked content" -Encoding UTF8
    Invoke-Git $taskWorktree @("add", "collision-test.rs") | Out-Null
    Invoke-Git $taskWorktree @("commit", "-m", "add collision fixture") | Out-Null
    $rejectedPush = Invoke-GitCaptureResult $taskWorktree @('push', 'origin', 'HEAD:main')
    if ($rejectedPush.ExitCode -eq 0 -or $rejectedPush.Text -notmatch 'non-fast-forward|fetch first|rejected') {
        throw "Collision fixture did not produce a real non-fast-forward rejection.`n$($rejectedPush.Text)"
    }
    Invoke-Git $taskWorktree @('fetch', 'origin') | Out-Null
    Invoke-Git $taskWorktree @('rebase', 'origin/main') | Out-Null
    Invoke-Git $taskWorktree @('push', 'origin', 'HEAD:main') | Out-Null

    $collisionOutput = Invoke-Finish -WorktreePath $taskWorktree -ExpectFailure -ContractId $taskContractId
    Assert-Contains $collisionOutput "BUSINESS_STATUS=complete" "A local-main collision must not erase the completed remote business state."
    Assert-Contains $collisionOutput "LOCAL_MAIN_STATUS=sync_failed" "A same-path untracked collision must block only local-main synchronization."
    Assert-Contains $collisionOutput "FINALIZABLE=false" "A same-path collision must remain visible to the task owner."
    $collisionContent = Get-Content -Raw -LiteralPath $mainCollisionPath
    if (-not $collisionContent.Contains("unknown local content")) {
        throw "Same-path unknown main file was overwritten unexpectedly."
    }

    Remove-Item -LiteralPath $mainCollisionPath -Force
    $collisionRecoveryOutput = Invoke-Finish -WorktreePath $taskWorktree -ContractId $taskContractId
    Assert-Contains $collisionRecoveryOutput "LOCAL_MAIN_STATUS=current:" "Finish must recover after the owner resolves a same-path collision."
    Assert-Contains $collisionRecoveryOutput "FINALIZABLE=true" "Resolved local-main collision must become finalizable."

    # Declared temporary roots are the only untracked content eligible for
    # automatic deletion.
    $temporaryRoot = Join-Path $taskWorktree ".ai-tmp"
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    Set-Content -LiteralPath (Join-Path $temporaryRoot "transient.log") -Value "temporary" -Encoding UTF8
    $tempOutput = Invoke-Finish -WorktreePath $taskWorktree -ContractId $taskContractId
    Assert-Contains $tempOutput "ARTIFACT_CLEANUP=task:.ai-tmp/" "Declared task temporary root must be cleaned."
    if (Test-Path -LiteralPath $temporaryRoot) { throw "Declared temporary root still exists after finish." }

    # Untracked files outside the declared temporary root are a hard stop in
    # the task worktree and receive a deterministic disposition hint.
    $unresolvedPath = Join-Path $taskWorktree "new_behavior_test.rs"
    Set-Content -LiteralPath $unresolvedPath -Value "#[test]" -Encoding UTF8
    $failureOutput = Invoke-Finish -WorktreePath $taskWorktree -ExpectFailure -ContractId $taskContractId
    Assert-Contains $failureOutput "TASK_UNRESOLVED_PATH=new_behavior_test.rs|candidate_track" "Source/test files must be classified as candidate_track."
    Assert-Contains $failureOutput "FINALIZABLE=false" "Dirty task worktree must block final reporting."

    Remove-Item -LiteralPath $unresolvedPath -Force
    $cleanupOutput = Invoke-Finish -WorktreePath $taskWorktree -PerformCleanup -ContractId $taskContractId
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
    if (-not (Test-Path -LiteralPath $platformReceiptPath -PathType Leaf)) { throw 'Platform cleanup removed the durable terminal finalization receipt.' }
    Assert-ReceiptBytesUnchanged $platformReceiptPath $completedBytes 'Platform cleanup changed the durable terminal finalization receipt.'
    if (-not $registeredAfterPlatformCleanup.Contains("branch refs/heads/codex/peer-fixture")) { throw "Locked peer worktree was removed by platform cleanup." }

    Write-Host "PASS ai-task-finish workflow guard"
} finally {
    if (-not [string]::IsNullOrWhiteSpace([string]$platformReceiptPath) -and
        (Test-Path -LiteralPath $platformReceiptPath -PathType Leaf)) {
        Remove-Item -LiteralPath $platformReceiptPath -Force
    }
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
