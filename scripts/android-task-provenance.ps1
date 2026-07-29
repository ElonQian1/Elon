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
    [pscustomobject]@{
        TaskPushed = $taskPushed
        PublishedContainsTask = $publishedContainsTask
        PublishedExactTask = $TaskHead.StartsWith($PublishedSha) -or $PublishedSha.StartsWith($TaskHead)
    }
}
