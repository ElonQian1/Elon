param()

$ErrorActionPreference = "Stop"

function Invoke-Git {
    param(
        [string]$Path,
        [string[]]$GitArgs
    )

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & git -C $Path @GitArgs 2>&1
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    if ($LASTEXITCODE -ne 0) {
        throw "git -C `"$Path`" $($GitArgs -join ' ') failed: $($output -join "`n")"
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
        throw "$Message Missing: $Expected"
    }
}

$repoRoot = (& git rev-parse --show-toplevel 2>&1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Run this script inside the git repository."
}
$repoRoot = $repoRoot.Trim()

$preflightScript = Join-Path $repoRoot "scripts\ai-task-preflight.ps1"
$preflightSh = Join-Path $repoRoot "scripts\ai-task-preflight.sh"
$directNetworkScript = Join-Path $repoRoot "scripts\direct-network.ps1"
$finishContractScript = Join-Path $repoRoot "scripts\ai-task-finish-contract.ps1"
$cleanupScript = Join-Path $repoRoot "scripts\cleanup-task-worktrees.ps1"
$cleanupSh = Join-Path $repoRoot "scripts\cleanup-task-worktrees.sh"
$preflightContent = Get-Content -Raw -LiteralPath $preflightScript
$preflightShContent = Get-Content -Raw -LiteralPath $preflightSh
$cleanupContent = Get-Content -Raw -LiteralPath $cleanupScript
$cleanupShContent = Get-Content -Raw -LiteralPath $cleanupSh

$expectedPsNeedsWorktree = '$needsWorktree = -not $isPcConversationWorktree -and ($AlwaysCreateWorktree -or $isDirty -or ($behind -gt 0) -or $isMainBaseline)'
Assert-Contains $preflightContent $expectedPsNeedsWorktree "PowerShell preflight must not treat -CreateWorktree alone as a need for another worktree."
Assert-Contains $preflightShContent 'if [[ "$pc_conversation_worktree" -ne 1 && ( "$always_create_worktree" -eq 1 || "$dirty" -eq 1 || "$behind" -gt 0 || "$branch" == "main" ) ]]; then' "Shell preflight must not treat --create-worktree alone as a need for another worktree."
Assert-Contains $preflightContent "function Write-AiWorkflowGuard" "PowerShell preflight must print the AI workflow guard."
Assert-Contains $preflightShContent "write_ai_workflow_guard()" "Shell preflight must print the AI workflow guard."
Assert-Contains $preflightContent "EDIT_ROOT=" "PowerShell preflight must expose the only safe edit root."
Assert-Contains $preflightShContent "EDIT_ROOT=" "Shell preflight must expose the only safe edit root."
Assert-Contains $preflightContent "AUTO_CLEANUP=skipped_created_worktree" "PowerShell preflight must not clean up the worktree it just created."
Assert-Contains $preflightContent "worktree lock --reason" "PowerShell preflight must lock active Codex task worktrees."
Assert-Contains $preflightShContent "worktree lock --reason" "Shell preflight must lock active Codex task worktrees."
Assert-Contains $preflightContent "function Test-PcConversationWorktree" "PowerShell preflight must detect platform-created PC conversation worktrees."
Assert-Contains $preflightShContent "is_pc_conversation_worktree()" "Shell preflight must detect platform-created PC conversation worktrees."
Assert-Contains $preflightContent "PC_CONVERSATION_WORKTREE=true" "PowerShell preflight must expose when the current workspace is already a PC conversation worktree."
Assert-Contains $preflightShContent "PC_CONVERSATION_WORKTREE=true" "Shell preflight must expose when the current workspace is already a PC conversation worktree."
Assert-Contains $preflightContent "FINISH_COMMAND_POWERSHELL=" "PowerShell preflight must print the deterministic finish entry point."
Assert-Contains $preflightContent "FINISH_CONTRACT_ID=" "PowerShell preflight must issue an immutable finish contract."
Assert-Contains $preflightContent "-TaskContract" "PowerShell finish command must bind the preflight contract."
Assert-Contains $preflightShContent "FINISH_COMMAND_SHELL=" "Shell preflight must print the deterministic finish entry point."
Assert-Contains $preflightShContent "FINISH_CONTRACT_ID=" "Shell preflight must issue an immutable finish contract."
Assert-Contains $preflightShContent "--task-contract" "Shell finish command must bind the preflight contract."
Assert-Contains $preflightContent "--untracked-files=no" "PowerShell preflight must separate tracked main changes from untracked hygiene warnings."
Assert-Contains $preflightShContent "--untracked-files=no" "Shell preflight must separate tracked main changes from untracked hygiene warnings."
Assert-Contains $cleanupContent "Test-PlatformSessionWorktree" "PowerShell cleanup must recognize platform conversation worktrees."
Assert-Contains $cleanupContent "ai/session/*" "PowerShell cleanup must include platform session branches in safe cleanup candidates."
Assert-Contains $cleanupShContent "is_platform_session_worktree" "Shell cleanup must recognize platform conversation worktrees."
Assert-Contains $cleanupShContent "--min-age-minutes" "Shell cleanup must support the same recent-worktree protection knob."
Assert-Contains $cleanupContent '"locked"' "PowerShell cleanup must preserve Git-locked active worktrees."
Assert-Contains $cleanupShContent "locked)" "Shell cleanup must preserve Git-locked active worktrees."

function Assert-DocumentContains {
    param(
        [string]$RelativePath,
        [string]$Snippet
    )

    $docPath = Join-Path $repoRoot $relativePath
    $docContent = Get-Content -Raw -LiteralPath $docPath
    Assert-Contains -Text $docContent -Expected $Snippet -Message "Workflow documentation is missing the required preflight/worktree rule in $RelativePath."
}

function Assert-DocumentDoesNotContain {
    param(
        [string]$RelativePath,
        [string]$Snippet
    )

    $docPath = Join-Path $repoRoot $RelativePath
    $docContent = Get-Content -Raw -LiteralPath $docPath
    if ($docContent.Contains($Snippet)) {
        throw "Workflow documentation still contains obsolete preflight/worktree guidance in $RelativePath. Forbidden: $Snippet"
    }
}

function Assert-DocumentTokenBudget {
    param(
        [string]$RelativePath,
        [int]$MaxApproxTokens
    )

    $docPath = Join-Path $repoRoot $RelativePath
    $docContent = Get-Content -Raw -LiteralPath $docPath
    $approxTokens = [int][Math]::Ceiling($docContent.Length / 4.0)
    if ($approxTokens -gt $MaxApproxTokens) {
        throw "Mandatory routing document exceeded its token budget: $RelativePath approxTokens=$approxTokens max=$MaxApproxTokens"
    }
}

function Assert-WorkflowFileDoesNotContain {
    param(
        [string]$RelativePath,
        [string]$Snippet,
        [string]$Reason
    )

    $docPath = Join-Path $repoRoot $RelativePath
    $docContent = Get-Content -Raw -LiteralPath $docPath
    if ($docContent.Contains($Snippet)) {
        throw "Workflow file contains obsolete release guidance in $RelativePath. Forbidden: $Snippet. $Reason"
    }
}

function Assert-WorkflowFileContains {
    param(
        [string]$RelativePath,
        [string]$Snippet,
        [string]$Reason
    )

    $docPath = Join-Path $repoRoot $RelativePath
    $docContent = Get-Content -Raw -LiteralPath $docPath
    Assert-Contains -Text $docContent -Expected $Snippet -Message "Workflow file is missing required release guidance in $RelativePath. $Reason"
}

function Invoke-PreflightAndAssertNoNestedWorktree {
    param(
        [string]$WorktreePath,
        [string]$WorktreeParent,
        [string]$ExpectedBranch,
        [string]$ExpectedState,
        [string]$Reason
    )

    New-Item -ItemType Directory -Path $WorktreeParent -Force | Out-Null
    $preflightArgs = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        (Join-Path $WorktreePath "scripts\ai-task-preflight.ps1"),
        "-CreateWorktree",
        "-BranchPrefix",
        "codex/preflight-test",
        "-WorktreeParent",
        $WorktreeParent,
        "-SkipAutoCleanup"
    )

    Push-Location -LiteralPath $WorktreePath
    try {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & powershell @preflightArgs 2>&1
        } finally {
            $ErrorActionPreference = $oldPreference
        }
    } finally {
        Pop-Location
    }

    $outputText = ($output -join "`n").Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "ai-task-preflight.ps1 failed in isolated fixture: $outputText"
    }

    Assert-Contains $outputText "BRANCH=$ExpectedBranch" "$Reason Fixture ran from the wrong branch."
    Assert-Contains $outputText "WORKTREE_CREATED=false" "$Reason must not create another worktree."
    Assert-Contains $outputText "AI_WORKFLOW_GUARD_BEGIN" "$Reason output must include the self-contained AI workflow guard."
    Assert-Contains $outputText "EDIT_ROOT=" "$Reason output must expose the safe edit root."
    Assert-Contains $outputText "EDIT_STATE=$ExpectedState" "$Reason must report the expected edit state."

    $createdChildren = @(Get-ChildItem -LiteralPath $WorktreeParent -Force)
    if ($createdChildren.Count -ne 0) {
        throw "$Reason unexpectedly created nested worktree entries under $WorktreeParent."
    }

    return $outputText
}

Assert-DocumentContains -RelativePath "AGENTS.md" -Snippet "WF-START"
Assert-DocumentContains -RelativePath "AGENTS.md" -Snippet "FINALIZABLE=true"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "WF-START"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "WF-FINISH"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "scripts\finish-ai-task.ps1"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "FINALIZABLE=true"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "cleanup-task-worktrees.*"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "FINISH_COMMAND_*"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "candidate_track"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "ai/session/*"
Assert-DocumentDoesNotContain -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet 'applyTo: "**"'
Assert-DocumentContains -RelativePath "docs\ai-agent-workflow.md" -Snippet "origin/main"
Assert-DocumentContains -RelativePath "docs\ai-agent-workflow.md" -Snippet "finish-ai-task.ps1"
Assert-DocumentContains -RelativePath "AI_TASK_TEMPLATE.md" -Snippet "WF-REPORT"
Assert-DocumentContains -RelativePath ".github\prompts\elon-dev-task.prompt.md" -Snippet "WF-REPORT"
Assert-DocumentContains -RelativePath ".github\prompts\elon-apk-release.prompt.md" -Snippet "AndroidFeature"
Assert-DocumentContains -RelativePath ".github\agents\elon-implementer.agent.md" -Snippet "WF-START"
Assert-DocumentContains -RelativePath ".github\agents\elon-planner.agent.md" -Snippet "WF-REPORT"
Assert-DocumentContains -RelativePath ".github\agents\elon-reviewer.agent.md" -Snippet "FINALIZABLE"
Assert-DocumentContains -RelativePath ".github\skills\cloud-apk-dev\SKILL.md" -Snippet "WF-START"
Assert-DocumentTokenBudget -RelativePath ".github\copilot-instructions.md" -MaxApproxTokens 1100
Assert-DocumentTokenBudget -RelativePath "AGENTS.md" -MaxApproxTokens 750
Assert-DocumentTokenBudget -RelativePath "CODEX.md" -MaxApproxTokens 500
Assert-DocumentTokenBudget -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -MaxApproxTokens 1000
Assert-DocumentTokenBudget -RelativePath "AI_TASK_TEMPLATE.md" -MaxApproxTokens 400
$thinLifecycleAssets = @(
    "AI_TASK_TEMPLATE.md",
    ".github\prompts\elon-dev-task.prompt.md",
    ".github\prompts\elon-apk-release.prompt.md",
    ".github\agents\elon-implementer.agent.md",
    ".github\agents\elon-planner.agent.md",
    ".github\agents\elon-reviewer.agent.md",
    ".github\skills\cloud-apk-dev\SKILL.md"
)
foreach ($asset in $thinLifecycleAssets) {
    Assert-DocumentDoesNotContain -RelativePath $asset -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
    Assert-DocumentDoesNotContain -RelativePath $asset -Snippet "git pull --ff-only origin main"
}
$parallelPublishDiscussionDoc = Join-Path "docs" ([string]::Concat([char]0x5e76, [char]0x884c, [char]0x53d1, [char]0x5e03, [char]0x8ba8, [char]0x8bba, ".md"))
Assert-DocumentContains -RelativePath $parallelPublishDiscussionDoc -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
Assert-DocumentContains -RelativePath $parallelPublishDiscussionDoc -Snippet "WORKTREE_PATH"
Assert-DocumentContains -RelativePath $parallelPublishDiscussionDoc -Snippet "scripts\finish-ai-task.ps1"
Assert-DocumentDoesNotContain -RelativePath $parallelPublishDiscussionDoc -Snippet "git pull --ff-only origin main"
Assert-DocumentDoesNotContain -RelativePath $parallelPublishDiscussionDoc -Snippet "Stop-Process"
Assert-DocumentDoesNotContain -RelativePath $parallelPublishDiscussionDoc -Snippet "bb64a-session"
Assert-DocumentDoesNotContain -RelativePath $parallelPublishDiscussionDoc -Snippet "bb64a-deploy"

$releaseWorkflowFiles = @(
    "AGENTS.md",
    ".github\copilot-instructions.md",
    ".github\instructions\git-deploy-workflow.instructions.md",
    ".github\prompts\elon-dev-task.prompt.md",
    ".github\prompts\elon-apk-release.prompt.md",
    ".github\skills\cloud-apk-dev\SKILL.md",
    "docs\ai-agent-workflow.md",
    $parallelPublishDiscussionDoc,
    "scripts\publish-apk.ps1",
    "scripts\publish-apk.sh",
    "scripts\publish-server.ps1",
    "scripts\publish-server.sh"
)

foreach ($releaseWorkflowFile in $releaseWorkflowFiles) {
    Assert-WorkflowFileDoesNotContain -RelativePath $releaseWorkflowFile -Snippet "git push origin master" -Reason "The project primary branch is main, and task worktrees must push current HEAD explicitly."
    Assert-WorkflowFileDoesNotContain -RelativePath $releaseWorkflowFile -Snippet "HEAD:master" -Reason "The project primary branch is main."
    Assert-WorkflowFileDoesNotContain -RelativePath $releaseWorkflowFile -Snippet "git push origin main" -Reason "Task worktrees are on codex/* branches; use git push origin HEAD:main so the current commit is pushed."
}

Assert-WorkflowFileContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "git push origin HEAD:main" -Reason "The current worktree commit must be pushed to origin/main explicitly."
Assert-WorkflowFileContains -RelativePath ".github\copilot-instructions.md" -Snippet "git push origin HEAD:main" -Reason "The global instruction must not depend on the local branch name."
Assert-WorkflowFileContains -RelativePath $parallelPublishDiscussionDoc -Snippet "git push origin HEAD:main" -Reason "Parallel workflow docs must show worktree-safe pushes."
Assert-WorkflowFileContains -RelativePath "docs\ai-agent-workflow.md" -Snippet "git push origin HEAD:main" -Reason "The long-form workflow must show worktree-safe pushes."
Assert-WorkflowFileContains -RelativePath ".github\prompts\elon-apk-release.prompt.md" -Snippet "release-only commit" -Reason "APK release prompts must explicitly reject release-only version commits."
Assert-WorkflowFileContains -RelativePath "scripts\publish-apk.ps1" -Snippet "Restore-GradleVersionFile" -Reason "APK publishing must restore claimed versions out of git."
Assert-WorkflowFileContains -RelativePath "scripts\publish-apk.sh" -Snippet "restore_gradle" -Reason "APK publishing must restore claimed versions out of git."
Assert-WorkflowFileDoesNotContain -RelativePath "scripts\publish-apk.ps1" -Snippet "release commit" -Reason "APK script logs should say source commit and no version commit, not imply a generated release commit."
Assert-WorkflowFileDoesNotContain -RelativePath "scripts\publish-apk.sh" -Snippet "release commit" -Reason "APK script logs should say source commit and no version commit, not imply a generated release commit."
Assert-WorkflowFileDoesNotContain -RelativePath "scripts\publish-apk.ps1" -Snippet "git commit -m" -Reason "APK publishing must not create release-only version commits."
Assert-WorkflowFileDoesNotContain -RelativePath "scripts\publish-apk.sh" -Snippet "git commit -m" -Reason "APK publishing must not create release-only version commits."
Assert-WorkflowFileDoesNotContain -RelativePath "scripts\publish-apk.ps1" -Snippet "git add " -Reason "APK publishing must not stage build.gradle version changes."
Assert-WorkflowFileDoesNotContain -RelativePath "scripts\publish-apk.sh" -Snippet "git add " -Reason "APK publishing must not stage build.gradle version changes."

$tempBase = [System.IO.Path]::GetTempPath()
$testRoot = Join-Path $tempBase ("elon-preflight-workflow-test-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testRoot | Out-Null

try {
    $originPath = Join-Path $testRoot "origin.git"
    $seedRepo = Join-Path $testRoot "seed"
    $existingWorktree = Join-Path $testRoot "existing-worktree"
    $createdWorktreeParent = Join-Path $testRoot "created"
    $conversationPathOnlyWorktree = Join-Path $testRoot "conversation-worktrees\demo-project\path-only"
    $conversationBranchOnlyWorktree = Join-Path $testRoot "conversation-worktrees\demo-project\branch-only"
    $conversationPathCreatedParent = Join-Path $testRoot "created-conversation-path"
    $conversationBranchCreatedParent = Join-Path $testRoot "created-conversation-branch"

    & git init --bare $originPath *> $null
    if ($LASTEXITCODE -ne 0) { throw "git init --bare failed" }

    & git init -b main $seedRepo *> $null
    if ($LASTEXITCODE -ne 0) { throw "git init -b main failed" }

    Invoke-Git $seedRepo @("config", "user.email", "preflight-test@example.invalid") | Out-Null
    Invoke-Git $seedRepo @("config", "user.name", "preflight-test") | Out-Null

    New-Item -ItemType Directory -Path (Join-Path $seedRepo "scripts") | Out-Null
    Copy-Item -LiteralPath $preflightScript -Destination (Join-Path $seedRepo "scripts\ai-task-preflight.ps1")
    Copy-Item -LiteralPath $directNetworkScript -Destination (Join-Path $seedRepo "scripts\direct-network.ps1")
    Copy-Item -LiteralPath $finishContractScript -Destination (Join-Path $seedRepo "scripts\ai-task-finish-contract.ps1")
    Set-Content -LiteralPath (Join-Path $seedRepo "README.md") -Value "preflight workflow test`n" -Encoding UTF8

    Invoke-Git $seedRepo @("add", "README.md", "scripts/ai-task-preflight.ps1", "scripts/direct-network.ps1", "scripts/ai-task-finish-contract.ps1") | Out-Null
    Invoke-Git $seedRepo @("commit", "-m", "seed preflight workflow test") | Out-Null
    Invoke-Git $seedRepo @("remote", "add", "origin", $originPath) | Out-Null
    Invoke-Git $seedRepo @("push", "-u", "origin", "main") | Out-Null

    Invoke-Git $seedRepo @("worktree", "add", "-b", "codex/existing-clean", $existingWorktree, "origin/main") | Out-Null
    New-Item -ItemType Directory -Path $createdWorktreeParent | Out-Null

    # Advance origin/main from the task worktree while leaving an unrelated
    # untracked file in the checked-out main baseline. Preflight must still
    # fast-forward tracked main and preserve the unknown file.
    $legacyMainFile = Join-Path $seedRepo "legacy-preflight-test.rs"
    Set-Content -LiteralPath $legacyMainFile -Value "legacy diagnostic" -Encoding UTF8
    Add-Content -LiteralPath (Join-Path $existingWorktree "README.md") -Value "task worktree advance"
    Invoke-Git $existingWorktree @("add", "README.md") | Out-Null
    Invoke-Git $existingWorktree @("commit", "-m", "advance origin from isolated task") | Out-Null
    Invoke-Git $existingWorktree @("push", "origin", "HEAD:main") | Out-Null

    $outputText = Invoke-PreflightAndAssertNoNestedWorktree `
        -WorktreePath $existingWorktree `
        -WorktreeParent $createdWorktreeParent `
        -ExpectedBranch "codex/existing-clean" `
        -ExpectedState "current_worktree_ok" `
        -Reason "Clean current non-main worktree"

    Assert-Contains $outputText "BRANCH=codex/existing-clean" "Fixture did not run from the clean non-main worktree."
    Assert-Contains $outputText "DIRTY=False" "Fixture worktree should be clean."
    Assert-Contains $outputText "AHEAD=0" "Fixture should not be ahead of origin/main."
    Assert-Contains $outputText "BEHIND=0" "Fixture should not be behind origin/main."
    Assert-Contains $outputText "NEXT=Workspace is already isolated and current enough for direct edits." "Clean current non-main worktree should remain usable."
    Assert-Contains $outputText "RULE_MAIN_BASELINE=main checkout is sync-only; do not edit business files in main." "Preflight guard must warn that main is a baseline only."
    Assert-Contains $outputText "MAIN_BASELINE_UNTRACKED=warning:1" "Untracked main files must be audited without blocking preflight sync."
    Assert-Contains $outputText "MAIN_BASELINE_SYNC=synced_worktree:" "Preflight must fast-forward tracked main despite unrelated untracked files."
    $mainAfterPreflight = Invoke-Git $seedRepo @("rev-parse", "HEAD")
    $originAfterPreflight = Invoke-Git $seedRepo @("rev-parse", "origin/main")
    if ($mainAfterPreflight -ne $originAfterPreflight) { throw "Preflight did not synchronize the checked-out main baseline." }
    if (-not (Test-Path -LiteralPath $legacyMainFile)) { throw "Preflight deleted an unknown main untracked file." }

    Invoke-Git $seedRepo @("worktree", "add", "-b", "codex/path-only", $conversationPathOnlyWorktree, "origin/main") | Out-Null
    Invoke-Git $seedRepo @("worktree", "add", "-b", "ai/session/demo-project/branch-only", $conversationBranchOnlyWorktree, "origin/main") | Out-Null
    Invoke-Git $seedRepo @("worktree", "lock", "--reason", "elon-supervision:branch-root", $conversationBranchOnlyWorktree) | Out-Null

    Set-Content -LiteralPath (Join-Path $seedRepo "README.md") -Value "preflight workflow test advanced`n" -Encoding UTF8
    Invoke-Git $seedRepo @("add", "README.md") | Out-Null
    Invoke-Git $seedRepo @("commit", "-m", "advance origin main for preflight workflow test") | Out-Null
    Invoke-Git $seedRepo @("push", "origin", "main") | Out-Null

    $conversationPathOutput = Invoke-PreflightAndAssertNoNestedWorktree `
        -WorktreePath $conversationPathOnlyWorktree `
        -WorktreeParent $conversationPathCreatedParent `
        -ExpectedBranch "codex/path-only" `
        -ExpectedState "pc_conversation_worktree_ok" `
        -Reason "PC conversation path worktree"
    Assert-Contains $conversationPathOutput "BEHIND=1" "PC conversation path fixture must be behind origin/main to prove the exception suppresses nested worktree creation."
    Assert-Contains $conversationPathOutput "PC_CONVERSATION_WORKTREE=true" "PC conversation path fixture must be detected from the worktree path."
    Assert-Contains $conversationPathOutput "NEXT=PC conversation worktree is already isolated; use the current workspace for direct edits." "PC conversation path fixture should remain usable in place."

    $conversationBranchOutput = Invoke-PreflightAndAssertNoNestedWorktree `
        -WorktreePath $conversationBranchOnlyWorktree `
        -WorktreeParent $conversationBranchCreatedParent `
        -ExpectedBranch "ai/session/demo-project/branch-only" `
        -ExpectedState "pc_conversation_worktree_ok" `
        -Reason "PC conversation branch worktree"
    Assert-Contains $conversationBranchOutput "BEHIND=1" "PC conversation branch fixture must be behind origin/main to prove the exception suppresses nested worktree creation."
    Assert-Contains $conversationBranchOutput "PC_CONVERSATION_WORKTREE=true" "PC conversation branch fixture must be detected from the branch name."
    Assert-Contains $conversationBranchOutput "NEXT=PC conversation worktree is already isolated; use the current workspace for direct edits." "PC conversation branch fixture should remain usable in place."

    Write-Host "PASS ai-task-preflight workflow guard"
} finally {
    $resolved = Resolve-Path -LiteralPath $testRoot -ErrorAction SilentlyContinue
    if ($resolved) {
        $resolvedPath = $resolved.Path
        $tempFullPath = [System.IO.Path]::GetFullPath($tempBase)
        $leaf = Split-Path -Leaf $resolvedPath
        if ($resolvedPath.StartsWith($tempFullPath, [StringComparison]::OrdinalIgnoreCase) -and $leaf.StartsWith("elon-preflight-workflow-test-")) {
            Remove-Item -LiteralPath $resolvedPath -Recurse -Force
        } else {
            Write-Warning "Skip cleanup for unexpected test path: $resolvedPath"
        }
    }
}

$finishWorkflowTest = Join-Path $repoRoot "scripts\test-ai-task-finish-workflow.ps1"
$oldPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $finishTestOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File $finishWorkflowTest 2>&1
    $finishTestExitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $oldPreference
}
$finishTestOutput | ForEach-Object { Write-Host ([string]$_) }
if ($finishTestExitCode -ne 0) {
    throw "Unified finish workflow guard failed."
}

$formatWorkflowTest = Join-Path $repoRoot "scripts\test-format-rust-workflow.ps1"
$oldPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $formatTestOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File $formatWorkflowTest 2>&1
    $formatTestExitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $oldPreference
}
$formatTestOutput | ForEach-Object { Write-Host ([string]$_) }
if ($formatTestExitCode -ne 0) {
    throw "Rust format workflow guard failed."
}

$githubSshNetworkTest = Join-Path $repoRoot "scripts\test-github-ssh-network.ps1"
$oldPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $githubSshTestOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File $githubSshNetworkTest 2>&1
    $githubSshTestExitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $oldPreference
}
$githubSshTestOutput | ForEach-Object { Write-Host ([string]$_) }
if ($githubSshTestExitCode -ne 0) {
    throw "GitHub SSH network fallback guard failed."
}
