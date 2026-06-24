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
$preflightContent = Get-Content -Raw -LiteralPath $preflightScript
$preflightShContent = Get-Content -Raw -LiteralPath $preflightSh

$expectedPsNeedsWorktree = '$needsWorktree = $AlwaysCreateWorktree -or $isDirty -or ($behind -gt 0) -or $isMainBaseline'
Assert-Contains $preflightContent $expectedPsNeedsWorktree "PowerShell preflight must not treat -CreateWorktree alone as a need for another worktree."
Assert-Contains $preflightShContent 'if [[ "$always_create_worktree" -eq 1 || "$dirty" -eq 1 || "$behind" -gt 0 || "$branch" == "main" ]]; then' "Shell preflight must not treat --create-worktree alone as a need for another worktree."
Assert-Contains $preflightContent "function Write-AiWorkflowGuard" "PowerShell preflight must print the AI workflow guard."
Assert-Contains $preflightShContent "write_ai_workflow_guard()" "Shell preflight must print the AI workflow guard."
Assert-Contains $preflightContent "EDIT_ROOT=" "PowerShell preflight must expose the only safe edit root."
Assert-Contains $preflightShContent "EDIT_ROOT=" "Shell preflight must expose the only safe edit root."

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

Assert-DocumentContains -RelativePath "AGENTS.md" -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
Assert-DocumentContains -RelativePath "AGENTS.md" -Snippet "EDIT_ROOT"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "WORKTREE_CREATED=true"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "EDIT_ROOT"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "WORKTREE_PATH"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "EDIT_ROOT"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "nested worktree"
Assert-DocumentContains -RelativePath "docs\ai-agent-workflow.md" -Snippet "origin/main"
Assert-DocumentContains -RelativePath "docs\ai-agent-workflow.md" -Snippet "EDIT_ROOT"
Assert-DocumentContains -RelativePath "AI_TASK_TEMPLATE.md" -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
Assert-DocumentContains -RelativePath "AI_TASK_TEMPLATE.md" -Snippet "EDIT_ROOT"
Assert-DocumentContains -RelativePath ".github\prompts\elon-dev-task.prompt.md" -Snippet "WORKTREE_PATH"
Assert-DocumentContains -RelativePath ".github\prompts\elon-apk-release.prompt.md" -Snippet "WORKTREE_PATH"
Assert-DocumentContains -RelativePath ".github\agents\elon-implementer.agent.md" -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
Assert-DocumentContains -RelativePath ".github\agents\elon-planner.agent.md" -Snippet "ai-task-preflight"
Assert-DocumentContains -RelativePath ".github\agents\elon-reviewer.agent.md" -Snippet "release API"
Assert-DocumentContains -RelativePath ".github\skills\cloud-apk-dev\SKILL.md" -Snippet "WORKTREE_CREATED=true"
$parallelPublishDiscussionDoc = Join-Path "docs" ([string]::Concat([char]0x5e76, [char]0x884c, [char]0x53d1, [char]0x5e03, [char]0x8ba8, [char]0x8bba, ".md"))
Assert-DocumentContains -RelativePath $parallelPublishDiscussionDoc -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
Assert-DocumentContains -RelativePath $parallelPublishDiscussionDoc -Snippet "WORKTREE_PATH"
Assert-DocumentContains -RelativePath $parallelPublishDiscussionDoc -Snippet "scripts\cleanup-task-worktrees.ps1"
Assert-DocumentDoesNotContain -RelativePath $parallelPublishDiscussionDoc -Snippet "git fetch origin main"
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

    & git init --bare $originPath *> $null
    if ($LASTEXITCODE -ne 0) { throw "git init --bare failed" }

    & git init -b main $seedRepo *> $null
    if ($LASTEXITCODE -ne 0) { throw "git init -b main failed" }

    Invoke-Git $seedRepo @("config", "user.email", "preflight-test@example.invalid") | Out-Null
    Invoke-Git $seedRepo @("config", "user.name", "preflight-test") | Out-Null

    New-Item -ItemType Directory -Path (Join-Path $seedRepo "scripts") | Out-Null
    Copy-Item -LiteralPath $preflightScript -Destination (Join-Path $seedRepo "scripts\ai-task-preflight.ps1")
    Set-Content -LiteralPath (Join-Path $seedRepo "README.md") -Value "preflight workflow test`n" -Encoding UTF8

    Invoke-Git $seedRepo @("add", "README.md", "scripts/ai-task-preflight.ps1") | Out-Null
    Invoke-Git $seedRepo @("commit", "-m", "seed preflight workflow test") | Out-Null
    Invoke-Git $seedRepo @("remote", "add", "origin", $originPath) | Out-Null
    Invoke-Git $seedRepo @("push", "-u", "origin", "main") | Out-Null

    Invoke-Git $seedRepo @("worktree", "add", "-b", "codex/existing-clean", $existingWorktree, "origin/main") | Out-Null
    New-Item -ItemType Directory -Path $createdWorktreeParent | Out-Null

    $preflightArgs = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        (Join-Path $existingWorktree "scripts\ai-task-preflight.ps1"),
        "-CreateWorktree",
        "-BranchPrefix",
        "codex/preflight-test",
        "-WorktreeParent",
        $createdWorktreeParent,
        "-SkipAutoCleanup"
    )
    Push-Location -LiteralPath $existingWorktree
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

    Assert-Contains $outputText "BRANCH=codex/existing-clean" "Fixture did not run from the clean non-main worktree."
    Assert-Contains $outputText "DIRTY=False" "Fixture worktree should be clean."
    Assert-Contains $outputText "AHEAD=0" "Fixture should not be ahead of origin/main."
    Assert-Contains $outputText "BEHIND=0" "Fixture should not be behind origin/main."
    Assert-Contains $outputText "WORKTREE_CREATED=false" "Clean current non-main worktree must not create another worktree just because -CreateWorktree was passed."
    Assert-Contains $outputText "NEXT=Workspace is already isolated and current enough for direct edits." "Clean current non-main worktree should remain usable."
    Assert-Contains $outputText "AI_WORKFLOW_GUARD_BEGIN" "Preflight output must include the self-contained AI workflow guard."
    Assert-Contains $outputText "EDIT_ROOT=" "Preflight output must expose the safe edit root."
    Assert-Contains $outputText "EDIT_STATE=current_worktree_ok" "Clean current non-main worktree must be marked as directly editable."
    Assert-Contains $outputText "RULE_MAIN_BASELINE=main checkout is sync-only; do not edit business files in main." "Preflight guard must warn that main is a baseline only."

    $createdChildren = @(Get-ChildItem -LiteralPath $createdWorktreeParent -Force)
    if ($createdChildren.Count -ne 0) {
        throw "Clean current non-main worktree unexpectedly created nested worktree entries under $createdWorktreeParent."
    }

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
