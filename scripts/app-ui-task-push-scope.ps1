. (Join-Path $PSScriptRoot 'git-path-resolution.ps1')

function Get-ElonAppUiTaskPushScopeMarkerPath {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)

    $paths = Get-ElonRepositoryPathsFromRoot -RepoRoot $RepoRoot
    Join-Path $paths.GitDir 'elon-app-ui-push-scope.v1.json'
}

function Get-ElonAppUiTaskMarkerSha {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)

    $paths = Get-ElonRepositoryPathsFromRoot -RepoRoot $RepoRoot
    $markerPath = Join-Path $paths.GitDir 'elon-task-base.v1'
    if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) { return $null }
    $sha = ([System.IO.File]::ReadAllText($markerPath, [System.Text.Encoding]::ASCII)).Trim()
    if ($sha -notmatch '^[0-9a-f]{40}$') { throw "Invalid APP UI task base marker: $markerPath" }
    $sha
}

function Test-ElonGitAncestor {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$AncestorSha,
        [Parameter(Mandatory = $true)][string]$DescendantSha
    )

    & git -C $RepoRoot merge-base --is-ancestor $AncestorSha $DescendantSha 2>$null
    $LASTEXITCODE -eq 0
}

function Get-ElonPushChangedPaths {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$BaseSha,
        [Parameter(Mandatory = $true)][string]$HeadSha
    )

    if ($BaseSha -eq $HeadSha) { return @() }
    $paths = & git -C $RepoRoot -c core.quotePath=false diff --name-only $BaseSha $HeadSha 2>$null
    if ($LASTEXITCODE -ne 0) { throw "Unable to inspect successful push paths between $BaseSha and $HeadSha" }
    @($paths | ForEach-Object { $_.Trim() -replace '\\', '/' } | Where-Object { $_ } | Sort-Object -Unique)
}

function New-ElonAppUiPushScopeCandidate {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string[]]$GitArgs,
        [string]$RemoteName = 'origin'
    )

    if ($GitArgs.Count -lt 3 -or $GitArgs[0] -ne 'push') { return $null }
    $pushesHeadToMain = @($GitArgs | Where-Object { $_ -match '^HEAD:(?:refs/heads/)?main$' }).Count -gt 0
    if (-not $pushesHeadToMain) { return $null }

    $headSha = (& git -C $RepoRoot rev-parse HEAD 2>$null).Trim()
    $remoteRef = "refs/remotes/$RemoteName/main"
    $pushBaseSha = (& git -C $RepoRoot rev-parse --verify $remoteRef 2>$null).Trim()
    $taskBaseSha = Get-ElonAppUiTaskMarkerSha -RepoRoot $RepoRoot
    if (
        $headSha -notmatch '^[0-9a-f]{40}$' -or
        $pushBaseSha -notmatch '^[0-9a-f]{40}$' -or
        [string]::IsNullOrWhiteSpace($taskBaseSha)
    ) {
        return $null
    }
    if (-not (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha $pushBaseSha -DescendantSha $headSha)) {
        return $null
    }
    if (-not (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha $taskBaseSha -DescendantSha $pushBaseSha)) {
        return $null
    }

    [PSCustomObject]@{
        TaskBaseSha = $taskBaseSha
        ScopeBaseSha = $pushBaseSha
        HeadSha = $headSha
        ChangedPaths = @(Get-ElonPushChangedPaths -RepoRoot $RepoRoot -BaseSha $pushBaseSha -HeadSha $headSha)
        RemoteName = $RemoteName
        RemoteBranch = 'main'
    }
}

function Save-ElonAppUiTaskPushScope {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)]$Candidate
    )

    $markerPath = Get-ElonAppUiTaskPushScopeMarkerPath -RepoRoot $RepoRoot
    $scopeBaseSha = [string]$Candidate.ScopeBaseSha
    $changedPaths = @(
        $Candidate.ChangedPaths |
            ForEach-Object { ([string]$_).Trim() -replace '\\', '/' } |
            Where-Object { $_ } |
            Sort-Object -Unique
    )
    if (Test-Path -LiteralPath $markerPath -PathType Leaf) {
        try {
            $existing = Get-Content -Raw -LiteralPath $markerPath | ConvertFrom-Json
            if (
                [string]$existing.schema -eq 'elon.app_ui_task_push_scope.v1' -and
                [string]$existing.taskBaseSha -eq [string]$Candidate.TaskBaseSha -and
                (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha ([string]$existing.headSha) -DescendantSha ([string]$Candidate.HeadSha)) -and
                (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha ([string]$existing.scopeBaseSha) -DescendantSha ([string]$Candidate.HeadSha))
            ) {
                $scopeBaseSha = [string]$existing.scopeBaseSha
                $changedPaths = @(
                    @($existing.changedPaths) + @($Candidate.ChangedPaths) |
                        ForEach-Object { [string]$_ } |
                        Where-Object { $_ } |
                        Sort-Object -Unique
                )
            }
        } catch {
            Write-Warning "Ignoring stale APP UI push scope marker: $($_.Exception.Message)"
        }
    }

    $payload = [ordered]@{
        schema = 'elon.app_ui_task_push_scope.v1'
        taskBaseSha = [string]$Candidate.TaskBaseSha
        scopeBaseSha = $scopeBaseSha
        headSha = [string]$Candidate.HeadSha
        changedPaths = @($changedPaths)
        remoteName = [string]$Candidate.RemoteName
        remoteBranch = [string]$Candidate.RemoteBranch
        recordedAtUtc = ([datetime]::UtcNow).ToString('o')
    }
    $tempPath = "$markerPath.tmp-$PID-$([Guid]::NewGuid().ToString('N'))"
    try {
        [System.IO.File]::WriteAllText(
            $tempPath,
            (($payload | ConvertTo-Json -Depth 4) + "`n"),
            [System.Text.UTF8Encoding]::new($false)
        )
        Move-Item -LiteralPath $tempPath -Destination $markerPath -Force
    } finally {
        Remove-Item -LiteralPath $tempPath -Force -ErrorAction SilentlyContinue
    }
    Write-Host "APP_UI_PUSH_SCOPE_RECORDED=base:$scopeBaseSha head:$($Candidate.HeadSha) paths:$($changedPaths.Count)"
}

function Get-ElonAppUiTaskPushScope {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$TaskBaseSha,
        [Parameter(Mandatory = $true)][string]$HeadSha
    )

    $markerPath = Get-ElonAppUiTaskPushScopeMarkerPath -RepoRoot $RepoRoot
    if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) { return $null }
    try {
        $marker = Get-Content -Raw -LiteralPath $markerPath | ConvertFrom-Json
        if (
            [string]$marker.schema -ne 'elon.app_ui_task_push_scope.v1' -or
            [string]$marker.taskBaseSha -ne $TaskBaseSha -or
            [string]$marker.headSha -ne $HeadSha -or
            [string]$marker.remoteName -ne 'origin' -or
            [string]$marker.remoteBranch -ne 'main' -or
            [string]$marker.scopeBaseSha -notmatch '^[0-9a-f]{40}$' -or
            $null -eq $marker.changedPaths
        ) {
            return $null
        }
        if (-not (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha $TaskBaseSha -DescendantSha ([string]$marker.scopeBaseSha))) {
            return $null
        }
        if (-not (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha ([string]$marker.scopeBaseSha) -DescendantSha $HeadSha)) {
            return $null
        }
        [PSCustomObject]@{
            Sha = [string]$marker.scopeBaseSha
            Source = 'successful_push_marker'
            Path = $markerPath
            ChangedPaths = @(
                @($marker.changedPaths) |
                    ForEach-Object { [string]$_ } |
                    Where-Object { $_ } |
                    Sort-Object -Unique
            )
        }
    } catch {
        Write-Warning "Ignoring invalid APP UI push scope marker: $($_.Exception.Message)"
        return $null
    }
}
