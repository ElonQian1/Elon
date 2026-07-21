<#
.SYNOPSIS
    Finalize an AI task through one deterministic workflow gate.

.DESCRIPTION
    Verifies the task worktree is clean, runs the requested completion check,
    fast-forwards the tracked local main baseline even when unrelated untracked
    files exist, audits those files separately, and removes merged Codex task
    worktrees. Temporary files are only auto-removed from roots explicitly
    declared in .ai/workspace-policy.txt.
#>
param(
    [ValidateSet("CodePushed", "CodeSync", "AndroidFeature", "NodeAgent", "DocsOnly", "Server", "PcFrontend")]
    [string]$Kind = "CodePushed",

    [string]$TaskWorktree = "",

    [string]$TaskContract = "",

    [switch]$AllowLegacyNoTaskContract,

    [switch]$SkipArtifactCleanup,

    [switch]$SkipWorktreeCleanup
)

$ErrorActionPreference = "Stop"
$businessStatus = "not_checked"
$localMainStatus = "not_checked"
$taskWorktreeStatus = "not_checked"

. (Join-Path $PSScriptRoot 'ai-task-finish-contract.ps1')

function Invoke-NativeCapture {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = @(& $FilePath @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = $output
        Text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    }
}

function Invoke-GitCapture {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$GitArgs
    )
    return Invoke-NativeCapture -FilePath "git" -Arguments (@("-C", $RepoPath) + $GitArgs)
}

function Invoke-GitRequired {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$GitArgs
    )
    $result = Invoke-GitCapture -RepoPath $RepoPath -GitArgs $GitArgs
    if ($result.ExitCode -ne 0) {
        throw "git $($GitArgs -join ' ') failed in $RepoPath`: $($result.Text)"
    }
    return $result.Text
}

function Normalize-PathText {
    param([string]$Path)
    $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    return $fullPath.Replace('\', '/')
}

function Get-GitWorktreeEntries {
    param([string]$RepoPath)

    $result = Invoke-GitCapture -RepoPath $RepoPath -GitArgs @("worktree", "list", "--porcelain")
    if ($result.ExitCode -ne 0) {
        throw "Unable to list git worktrees: $($result.Text)"
    }

    $entries = @()
    $current = @{}
    foreach ($line in $result.Output) {
        $lineText = [string]$line
        if ($lineText -eq "") {
            if ($current.Count -gt 0) {
                $entries += [pscustomobject]$current
                $current = @{}
            }
            continue
        }
        $kv = $lineText -split " ", 2
        switch ($kv[0]) {
            "worktree" { $current["Path"] = $kv[1] }
            "HEAD"     { $current["Head"] = $kv[1] }
            "branch"   { $current["Branch"] = ($kv[1] -replace "^refs/heads/", "") }
            "bare"     { $current["Bare"] = $true }
            "detached" { $current["Detached"] = $true }
        }
    }
    if ($current.Count -gt 0) {
        $entries += [pscustomobject]$current
    }
    return $entries
}

function Read-WorkspacePolicy {
    param([string]$RepoPath)

    $policyPath = Join-Path $RepoPath ".ai\workspace-policy.txt"
    if (-not (Test-Path -LiteralPath $policyPath)) {
        throw "Workspace policy is missing: $policyPath"
    }

    $temporaryRoots = @()
    $sourceExtensions = @()
    $generatedExtensions = @()
    foreach ($line in Get-Content -LiteralPath $policyPath) {
        $trimmed = $line.Trim()
        if ($trimmed -eq "" -or $trimmed.StartsWith("#")) { continue }
        $parts = $trimmed -split "\s+", 2
        if ($parts.Count -ne 2) {
            throw "Invalid workspace policy line: $line"
        }
        switch ($parts[0]) {
            "temporary-root" { $temporaryRoots += $parts[1].Trim() }
            "source-extension" { $sourceExtensions += $parts[1].Trim().ToLowerInvariant() }
            "generated-extension" { $generatedExtensions += $parts[1].Trim().ToLowerInvariant() }
            default { throw "Unknown workspace policy rule: $($parts[0])" }
        }
    }

    return [pscustomobject]@{
        TemporaryRoots = $temporaryRoots
        SourceExtensions = $sourceExtensions
        GeneratedExtensions = $generatedExtensions
    }
}

function Clear-DeclaredTemporaryRoots {
    param(
        [string]$RepoPath,
        $Policy,
        [string]$Label
    )

    if ($SkipArtifactCleanup) { return }
    $rootFull = [System.IO.Path]::GetFullPath($RepoPath).TrimEnd('\', '/')
    $rootPrefix = $rootFull + [System.IO.Path]::DirectorySeparatorChar

    foreach ($relativeRoot in $Policy.TemporaryRoots) {
        $normalizedRelative = ($relativeRoot -replace '/', [System.IO.Path]::DirectorySeparatorChar).TrimEnd('\', '/')
        if (
            [string]::IsNullOrWhiteSpace($normalizedRelative) -or
            [System.IO.Path]::IsPathRooted($normalizedRelative) -or
            ($normalizedRelative -split '[\\/]') -contains ".."
        ) {
            throw "Unsafe temporary-root policy value: $relativeRoot"
        }

        $candidate = [System.IO.Path]::GetFullPath((Join-Path $rootFull $normalizedRelative))
        if (-not $candidate.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Temporary root escaped repository boundary: $candidate"
        }

        $tracked = Invoke-GitRequired -RepoPath $RepoPath -GitArgs @("ls-files", "--", $relativeRoot)
        if (-not [string]::IsNullOrWhiteSpace($tracked)) {
            throw "Refusing to clean tracked files from declared temporary root '$relativeRoot'."
        }

        if (Test-Path -LiteralPath $candidate) {
            Remove-Item -LiteralPath $candidate -Recurse -Force
            Write-Host "ARTIFACT_CLEANUP=$Label`:$relativeRoot"
        }
    }
}

function Get-StatusLines {
    param(
        [string]$RepoPath,
        [switch]$TrackedOnly
    )
    $untrackedMode = if ($TrackedOnly) { "no" } else { "all" }
    $result = Invoke-GitCapture -RepoPath $RepoPath -GitArgs @(
        "-c", "core.quotePath=false", "status", "--porcelain=v1", "--untracked-files=$untrackedMode"
    )
    if ($result.ExitCode -ne 0) {
        throw "git status failed in $RepoPath`: $($result.Text)"
    }
    return @($result.Output | ForEach-Object { [string]$_ } | Where-Object { $_ -ne "" })
}

function Get-UntrackedPaths {
    param([string]$RepoPath)
    return @(Get-StatusLines -RepoPath $RepoPath | Where-Object { $_ -like "?? *" } | ForEach-Object { $_.Substring(3) })
}

function Get-ArtifactDisposition {
    param(
        [string]$Path,
        $Policy
    )
    $extension = [System.IO.Path]::GetExtension($Path).ToLowerInvariant()
    if ($Policy.SourceExtensions -contains $extension) {
        return "candidate_track"
    }
    if ($Policy.GeneratedExtensions -contains $extension) {
        return "candidate_temporary_or_precise_ignore"
    }
    return "owner_decision_required"
}

function Write-UntrackedAudit {
    param(
        [string]$RepoPath,
        $Policy,
        [string]$Prefix
    )
    $paths = @(Get-UntrackedPaths -RepoPath $RepoPath)
    if ($paths.Count -eq 0) {
        Write-Host "$Prefix=clean"
        return 0
    }

    Write-Host "$Prefix=warning:$($paths.Count)"
    foreach ($path in $paths) {
        $disposition = Get-ArtifactDisposition -Path $path -Policy $Policy
        Write-Host "$Prefix`_PATH=$path|$disposition"
    }
    return $paths.Count
}

function Invoke-GitFetchWithRetry {
    param(
        [string]$RepoPath,
        [int]$Attempts = 3
    )
    $lastText = ""
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        $result = Invoke-GitCapture -RepoPath $RepoPath -GitArgs @(
            "-c", "http.proxy=", "-c", "https.proxy=", "fetch", "origin", "main"
        )
        if ($result.ExitCode -eq 0) { return }
        $lastText = $result.Text
        Write-Host "GIT_FETCH_RETRY=attempt_$attempt/$Attempts"
        if ($attempt -lt $Attempts) { Start-Sleep -Seconds 2 }
    }
    throw "git fetch origin main failed after $Attempts attempts: $lastText"
}

function Invoke-CompletionCheck {
    param(
        [string]$RepoPath,
        [string]$CompletionKind
    )
    $checkScript = Join-Path $RepoPath "scripts\check-task-complete.ps1"
    if (-not (Test-Path -LiteralPath $checkScript)) {
        throw "Completion check script is missing: $checkScript"
    }

    $enginePath = [System.Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
    $result = Invoke-NativeCapture -FilePath $enginePath -Arguments @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $checkScript, "-Kind", $CompletionKind
    )
    $result.Output | ForEach-Object { Write-Host ([string]$_) }
    if ($result.ExitCode -ne 0) {
        throw "Completion check failed for kind $CompletionKind."
    }
}

try {
    $startPath = if ([string]::IsNullOrWhiteSpace($TaskWorktree)) { (Get-Location).Path } else { $TaskWorktree }
    if (-not (Test-Path -LiteralPath $startPath)) {
        throw "Task worktree does not exist: $startPath"
    }

    $taskRoot = (Invoke-GitRequired -RepoPath $startPath -GitArgs @("rev-parse", "--show-toplevel")).Trim()
    $taskRoot = [System.IO.Path]::GetFullPath($taskRoot)
    $taskBranch = (Invoke-GitRequired -RepoPath $taskRoot -GitArgs @("branch", "--show-current")).Trim()
    $taskLeaf = Split-Path -Leaf $taskRoot
    $requiresContract = $taskBranch -like "codex/*" -or $taskLeaf -match '-task-\d{8}-\d{6}'
    if ($requiresContract -and [string]::IsNullOrWhiteSpace($TaskContract) -and -not $AllowLegacyNoTaskContract) {
        throw "Managed task worktree requires the immutable TaskContract emitted by preflight."
    }
    if (-not [string]::IsNullOrWhiteSpace($TaskContract)) {
        $null = Assert-AiTaskFinishContract -RepoPath $taskRoot -ContractId $TaskContract
        Write-Host "FINISH_CONTRACT_STATUS=validated:$TaskContract"
    } elseif ($AllowLegacyNoTaskContract) {
        Write-Host "FINISH_CONTRACT_STATUS=legacy_override"
    }
    $policy = Read-WorkspacePolicy -RepoPath $taskRoot

    $directNetworkScript = Join-Path $taskRoot "scripts\direct-network.ps1"
    if (Test-Path -LiteralPath $directNetworkScript) {
        . $directNetworkScript
        if (Get-Command Set-ElonProjectDirectGitSsh -ErrorAction SilentlyContinue) {
            Set-ElonProjectDirectGitSsh
        }
    }

    Clear-DeclaredTemporaryRoots -RepoPath $taskRoot -Policy $policy -Label "task"
    $taskStatus = @(Get-StatusLines -RepoPath $taskRoot)
    if ($taskStatus.Count -gt 0) {
        Write-Host "TASK_WORKTREE_STATUS=dirty"
        foreach ($line in $taskStatus) {
            if ($line -like "?? *") {
                $path = $line.Substring(3)
                $disposition = Get-ArtifactDisposition -Path $path -Policy $policy
                Write-Host "TASK_UNRESOLVED_PATH=$path|$disposition"
            } else {
                Write-Host "TASK_UNRESOLVED_GIT=$line"
            }
        }
        throw "Task worktree is not clean. Track intentional source/tests, move disposable output under .ai-tmp/, or add a precise ignore rule for stable generated output."
    }
    $taskWorktreeStatus = "clean"

    Invoke-CompletionCheck -RepoPath $taskRoot -CompletionKind $Kind
    $businessStatus = "complete"
    $isPlatformManagedTask = $taskBranch -like "ai/session/*"

    $entries = @(Get-GitWorktreeEntries -RepoPath $taskRoot)
    $mainWorktree = $entries | Where-Object { $_.Branch -eq "main" -and $_.Path } | Select-Object -First 1
    if (-not $mainWorktree) {
        throw "No checked-out main worktree was found; local main baseline cannot be finalized."
    }
    $mainPath = [System.IO.Path]::GetFullPath([string]$mainWorktree.Path)

    Clear-DeclaredTemporaryRoots -RepoPath $mainPath -Policy $policy -Label "main"
    $mainTrackedStatus = @(Get-StatusLines -RepoPath $mainPath -TrackedOnly)
    $skipMainSync = $false
    if ($mainTrackedStatus.Count -gt 0) {
        $localMainStatus = "blocked_tracked_changes"
        $mainTrackedStatus | ForEach-Object { Write-Host "MAIN_TRACKED_CHANGE=$_" }
        if ($isPlatformManagedTask) {
            Write-Host "MAIN_BASELINE_SYNC=blocked_tracked_changes:$mainPath"
            $skipMainSync = $true
        } else {
            throw "The main baseline has tracked changes and cannot be fast-forwarded safely."
        }
    }

    if (-not $skipMainSync) {
        Invoke-GitFetchWithRetry -RepoPath $mainPath
        $merge = Invoke-GitCapture -RepoPath $mainPath -GitArgs @("merge", "--ff-only", "origin/main")
        if ($merge.ExitCode -ne 0) {
            $localMainStatus = "sync_failed"
            throw "The main baseline could not fast-forward. Git may be protecting an untracked same-path collision: $($merge.Text)"
        }

        $mainHead = (Invoke-GitRequired -RepoPath $mainPath -GitArgs @("rev-parse", "HEAD")).Trim()
        $originHead = (Invoke-GitRequired -RepoPath $mainPath -GitArgs @("rev-parse", "origin/main")).Trim()
        if ($mainHead -ne $originHead) {
            $localMainStatus = "not_current"
            throw "Local main is not the current fetched origin/main. main=$mainHead origin/main=$originHead"
        }
        $localMainStatus = "current:$($mainHead.Substring(0, 7))"
    }
    $null = Write-UntrackedAudit -RepoPath $mainPath -Policy $policy -Prefix "MAIN_UNTRACKED_STATUS"

    $taskNormalized = Normalize-PathText $taskRoot
    $mainNormalized = Normalize-PathText $mainPath
    $isManagedTaskWorktree = $taskBranch -like "codex/*" -or $taskLeaf -match '-task-\d{8}-\d{6}'

    if ($taskNormalized -eq $mainNormalized) {
        $taskWorktreeStatus = "main_baseline_not_applicable"
    } elseif ($taskBranch -like "ai/session/*") {
        $taskWorktreeStatus = "platform_managed"
    } elseif ($SkipWorktreeCleanup) {
        $taskWorktreeStatus = "skipped_by_option"
    } elseif ($isManagedTaskWorktree) {
        Set-Location -LiteralPath $mainPath
        $unlock = Invoke-GitCapture -RepoPath $mainPath -GitArgs @("worktree", "unlock", $taskRoot)
        if ($unlock.ExitCode -ne 0 -and $unlock.Text -notmatch "not locked") {
            throw "Unable to unlock completed task worktree: $($unlock.Text)"
        }
        $remove = Invoke-GitCapture -RepoPath $mainPath -GitArgs @("worktree", "remove", $taskRoot)

        $remaining = @(Get-GitWorktreeEntries -RepoPath $mainPath | Where-Object {
            $_.Path -and (Normalize-PathText ([string]$_.Path)) -eq $taskNormalized
        })
        if ($remaining.Count -gt 0) {
            $taskWorktreeStatus = "cleanup_failed"
            throw "Task worktree is still registered after targeted cleanup: $taskRoot. $($remove.Text)"
        }

        if (Test-Path -LiteralPath $taskRoot) {
            $residualFiles = @(Get-ChildItem -LiteralPath $taskRoot -Force -Recurse -File -ErrorAction Stop)
            $reparseEntries = @(Get-ChildItem -LiteralPath $taskRoot -Force -Recurse -ErrorAction Stop | Where-Object {
                ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
            })
            if ($residualFiles.Count -gt 0 -or $reparseEntries.Count -gt 0) {
                $taskWorktreeStatus = "cleanup_failed_residual_files"
                Write-Host "TASK_WORKTREE_RESIDUAL_PATH=$taskRoot"
                Write-Host "TASK_WORKTREE_REMOVE_OUTPUT=$($remove.Text)"
                throw "Targeted worktree cleanup left files or links behind; refusing to delete unknown residual content."
            }

            try {
                Remove-Item -LiteralPath $taskRoot -Recurse -Force -ErrorAction Stop
                $taskWorktreeStatus = "cleaned"
            } catch {
                $taskWorktreeStatus = "cleaned_registration_residual_empty_directory"
                Write-Host "TASK_WORKTREE_RESIDUAL_PATH=$taskRoot"
            }
        } else {
            $taskWorktreeStatus = "cleaned"
        }
    } else {
        $taskWorktreeStatus = "user_managed"
    }

    Write-Host "BUSINESS_STATUS=$businessStatus"
    Write-Host "LOCAL_MAIN_STATUS=$localMainStatus"
    Write-Host "TASK_WORKTREE_STATUS=$taskWorktreeStatus"
    Write-Host "FINALIZABLE=true"
    exit 0
} catch {
    Write-Host "BUSINESS_STATUS=$businessStatus"
    Write-Host "LOCAL_MAIN_STATUS=$localMainStatus"
    Write-Host "TASK_WORKTREE_STATUS=$taskWorktreeStatus"
    Write-Host "FINALIZABLE=false"
    Write-Host "FINISH_ERROR=$($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
