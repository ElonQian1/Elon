function Test-GitCommitAncestor {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$Ancestor,
        [Parameter(Mandatory = $true)][string]$Descendant
    )

    & git -C $RepoPath rev-parse --verify "$Ancestor^{commit}" *> $null
    if ($LASTEXITCODE -ne 0) { return $false }
    & git -C $RepoPath rev-parse --verify "$Descendant^{commit}" *> $null
    if ($LASTEXITCODE -ne 0) { return $false }
    & git -C $RepoPath merge-base --is-ancestor $Ancestor $Descendant *> $null
    return $LASTEXITCODE -eq 0
}

function Get-AndroidTaskPublicationProvenance {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$TaskHead,
        [Parameter(Mandatory = $true)][string]$OriginMain,
        [Parameter(Mandatory = $true)][string]$PublishedSha
    )

    $taskPushed = Test-GitCommitAncestor -RepoPath $RepoPath `
        -Ancestor $TaskHead -Descendant $OriginMain
    $publishedContainsTask = Test-GitCommitAncestor -RepoPath $RepoPath `
        -Ancestor $TaskHead -Descendant $PublishedSha
    $publishedPrecedesTask = Test-GitCommitAncestor -RepoPath $RepoPath `
        -Ancestor $PublishedSha -Descendant $TaskHead
    $androidChanges = @()
    $sameAndroidInputs = $false
    if (-not $publishedContainsTask -and $publishedPrecedesTask) {
        $androidChanges = @(& git -C $RepoPath diff --name-only "$PublishedSha..$TaskHead" -- android 2>$null)
        $sameAndroidInputs = $LASTEXITCODE -eq 0 -and $androidChanges.Count -eq 0
    }
    $publishedExactTask = $TaskHead.StartsWith($PublishedSha) -or $PublishedSha.StartsWith($TaskHead)
    $coverageReason = if ($publishedExactTask) {
        "exact_task_commit"
    } elseif ($publishedContainsTask) {
        "deployed_descendant"
    } elseif ($sameAndroidInputs) {
        "same_android_inputs"
    } elseif ($publishedPrecedesTask) {
        "android_inputs_changed"
    } else {
        "diverged_history"
    }
    [pscustomobject]@{
        TaskPushed = $taskPushed
        PublishedContainsTask = $publishedContainsTask
        PublishedCoversTask = $publishedContainsTask -or $sameAndroidInputs
        PublishedExactTask = $publishedExactTask
        CoverageReason = $coverageReason
        ChangedAndroidPaths = $androidChanges
    }
}
