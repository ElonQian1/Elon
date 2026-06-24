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

$expectedPsNeedsWorktree = '$needsWorktree = $CreateWorktree -or $AlwaysCreateWorktree -or $isDirty -or ($behind -gt 0) -or $isMainBaseline'
Assert-Contains $preflightContent $expectedPsNeedsWorktree "PowerShell preflight must honor explicit -CreateWorktree."
Assert-Contains $preflightShContent 'create_worktree" -eq 1' "Shell preflight must honor explicit --create-worktree."

function Assert-DocumentContains {
    param(
        [string]$RelativePath,
        [string]$Snippet
    )

    $docPath = Join-Path $repoRoot $relativePath
    $docContent = Get-Content -Raw -LiteralPath $docPath
    Assert-Contains -Text $docContent -Expected $Snippet -Message "Workflow documentation is missing the required preflight/worktree rule in $RelativePath."
}

Assert-DocumentContains -RelativePath "AGENTS.md" -Snippet "scripts\ai-task-preflight.ps1 -CreateWorktree"
Assert-DocumentContains -RelativePath ".github\copilot-instructions.md" -Snippet "WORKTREE_CREATED=true"
Assert-DocumentContains -RelativePath ".github\instructions\git-deploy-workflow.instructions.md" -Snippet "WORKTREE_PATH"
Assert-DocumentContains -RelativePath "docs\ai-agent-workflow.md" -Snippet "origin/main"

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
    Assert-Contains $outputText "WORKTREE_CREATED=true" "Explicit -CreateWorktree must create a new worktree even from a clean non-main worktree."
    Assert-Contains $outputText "WORKTREE_PATH=$createdWorktreeParent" "Created worktree should stay inside the test parent directory."

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
