function Test-ElonReleaseGitCommit {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$Sha
    )
    if ($Sha -notmatch '^[0-9a-f]{40}$') { return $false }
    git -C $RepoRoot cat-file -e "$Sha^{commit}" 2>$null
    return ($LASTEXITCODE -eq 0)
}

function Test-ElonReleaseGitAncestor {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$Ancestor,
        [Parameter(Mandatory)][string]$Descendant
    )
    if (-not (Test-ElonReleaseGitCommit -RepoRoot $RepoRoot -Sha $Ancestor) -or
        -not (Test-ElonReleaseGitCommit -RepoRoot $RepoRoot -Sha $Descendant)) {
        return $false
    }
    git -C $RepoRoot merge-base --is-ancestor $Ancestor $Descendant 2>$null
    return ($LASTEXITCODE -eq 0)
}

function Test-GitAncestor {
    param([string]$Ancestor, [string]$Descendant)
    return (Test-ElonReleaseGitAncestor -RepoRoot $script:RepoRoot -Ancestor $Ancestor -Descendant $Descendant)
}

function Get-ElonAndroidInputChanges {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$FromSha,
        [Parameter(Mandatory)][string]$ToSha
    )
    if (-not (Test-ElonReleaseGitCommit -RepoRoot $RepoRoot -Sha $FromSha) -or
        -not (Test-ElonReleaseGitCommit -RepoRoot $RepoRoot -Sha $ToSha)) {
        return @()
    }
    return @(git -C $RepoRoot diff --name-only "$FromSha..$ToSha" -- android 2>$null)
}

function Get-ElonApkInputCoverage {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$CandidateSha,
        [string]$DeployedSha
    )
    if (-not (Test-ElonReleaseGitCommit -RepoRoot $RepoRoot -Sha $CandidateSha) -or
        -not (Test-ElonReleaseGitCommit -RepoRoot $RepoRoot -Sha $DeployedSha)) {
        return [pscustomobject]@{
            Covered = $false; Reason = 'unknown_source'; ChangedPaths = @()
        }
    }
    if (Test-ElonReleaseGitAncestor -RepoRoot $RepoRoot -Ancestor $CandidateSha -Descendant $DeployedSha) {
        return [pscustomobject]@{
            Covered = $true; Reason = 'deployed_descendant'; ChangedPaths = @()
        }
    }
    if (-not (Test-ElonReleaseGitAncestor -RepoRoot $RepoRoot -Ancestor $DeployedSha -Descendant $CandidateSha)) {
        return [pscustomobject]@{
            Covered = $false; Reason = 'diverged_history'; ChangedPaths = @()
        }
    }
    $changes = @(Get-ElonAndroidInputChanges -RepoRoot $RepoRoot -FromSha $DeployedSha -ToSha $CandidateSha)
    return [pscustomobject]@{
        Covered = ($changes.Count -eq 0)
        Reason = if ($changes.Count -eq 0) { 'same_android_inputs' } else { 'android_inputs_changed' }
        ChangedPaths = $changes
    }
}

function Get-ElonApkBuildStartDecision {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$CandidateSha,
        [Parameter(Mandatory)][string]$CurrentMainSha
    )
    if ($CandidateSha -eq $CurrentMainSha) {
        return [pscustomobject]@{ Build = $true; Reason = 'current_main'; ChangedPaths = @() }
    }
    if (-not (Test-ElonReleaseGitAncestor -RepoRoot $RepoRoot -Ancestor $CandidateSha -Descendant $CurrentMainSha)) {
        return [pscustomobject]@{ Build = $false; Reason = 'not_main_ancestor'; ChangedPaths = @() }
    }
    $changes = @(Get-ElonAndroidInputChanges -RepoRoot $RepoRoot -FromSha $CandidateSha -ToSha $CurrentMainSha)
    return [pscustomobject]@{
        Build = ($changes.Count -eq 0)
        Reason = if ($changes.Count -eq 0) { 'same_android_inputs' } else { 'newer_android_generation' }
        ChangedPaths = $changes
    }
}

function Test-RemoteAdvanceSafeForApk {
    param([string]$BaseSha)
    Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin", "main") -FailureContext "无法判断远端前进是否影响 APK" -Quiet
    $changed = git -C $script:RepoRoot diff --name-only "$BaseSha..origin/main" 2>$null
    if (-not $changed) { return $true }
    foreach ($path in $changed) {
        if ($path -match '^android/' -or $path -match '^scripts/publish-apk') { return $false }
    }
    return $true
}
