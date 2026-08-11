. (Join-Path $PSScriptRoot 'git-path-resolution.ps1')
. (Join-Path $PSScriptRoot 'app-ui-task-push-scope.ps1')

function Get-ElonAppUiChangedPaths {
    param(
        [Parameter(Mandatory)] [string]$RepoRoot,
        [Parameter(Mandatory)] [string]$BaseSha,
        [Parameter(Mandatory)] [string]$HeadSha
    )

    @(Get-ElonPushChangedPaths -RepoRoot $RepoRoot -BaseSha $BaseSha -HeadSha $HeadSha)
}

function Get-ElonAppUiTaskBaseSha {
    param(
        [Parameter(Mandatory)] [string]$RepoRoot,
        [string]$ExplicitBaseSha = ''
    )

    $candidate = $ExplicitBaseSha.Trim()
    $source = 'explicit'
    if ([string]::IsNullOrWhiteSpace($candidate)) {
        $repositoryPaths = Get-ElonRepositoryPathsFromRoot -RepoRoot $RepoRoot
        $markerPath = Join-Path $repositoryPaths.GitDir 'elon-task-base.v1'
        if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) { return $null }
        $candidate = ([System.IO.File]::ReadAllText($markerPath, [System.Text.Encoding]::ASCII)).Trim()
        $source = 'preflight_marker'
    }
    if ($candidate -notmatch '^[0-9a-f]{40}$') {
        throw "Invalid APP UI task base SHA from $source."
    }
    & git -C $RepoRoot cat-file -e "$candidate^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) { throw "APP UI task base commit is unavailable: $candidate" }
    $headSha = (& git -C $RepoRoot rev-parse HEAD).Trim()
    & git -C $RepoRoot merge-base --is-ancestor $candidate $headSha 2>$null
    if ($LASTEXITCODE -ne 0) { throw "APP UI task base is not an ancestor of HEAD: $candidate" }
    [PSCustomObject]@{ Sha = $candidate; Source = $source }
}

function Get-ElonStaticMobilePwaInputPaths {
    @(
        'server/src/assets/web_page.html',
        'server/src/assets/project_plaza.css',
        'server/src/assets/project_plaza_cache.js',
        'server/src/assets/project_plaza.js',
        'server/src/assets/orbital_mobile_theme.css'
    )
}

function Get-ElonAppUiTaskScopeBaseSha {
    param(
        [Parameter(Mandatory)] [string]$RepoRoot,
        [Parameter(Mandatory)] $TaskBase,
        [Parameter(Mandatory)] [string]$HeadSha,
        [string]$ExplicitScopeBaseSha = ''
    )

    $candidate = $ExplicitScopeBaseSha.Trim()
    if (-not [string]::IsNullOrWhiteSpace($candidate)) {
        if ($candidate -notmatch '^[0-9a-f]{40}$') {
            throw 'Invalid explicit APP UI task scope base SHA.'
        }
        if (-not (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha $TaskBase.Sha -DescendantSha $candidate)) {
            throw 'Explicit APP UI task scope base is outside the preflight task history.'
        }
        if (-not (Test-ElonGitAncestor -RepoRoot $RepoRoot -AncestorSha $candidate -DescendantSha $HeadSha)) {
            throw 'Explicit APP UI task scope base is not an ancestor of HEAD.'
        }
        return [PSCustomObject]@{ Sha = $candidate; Source = 'explicit_scope_base'; ChangedPaths = $null }
    }

    $pushScope = Get-ElonAppUiTaskPushScope `
        -RepoRoot $RepoRoot -TaskBaseSha $TaskBase.Sha -HeadSha $HeadSha
    if ($null -ne $pushScope) { return $pushScope }
    [PSCustomObject]@{ Sha = $TaskBase.Sha; Source = $TaskBase.Source; ChangedPaths = $null }
}

function Resolve-ElonAppUiChangeScope {
    param(
        [Parameter(Mandatory)] [string]$RepoRoot,
        [Parameter(Mandatory)] [string]$BaseSha,
        [Parameter(Mandatory)] [string]$HeadSha,
        [string[]]$ChangedPaths
    )

    & git -C $RepoRoot cat-file -e "$BaseSha^{commit}" 2>$null
    $baseExists = $LASTEXITCODE -eq 0
    & git -C $RepoRoot cat-file -e "$HeadSha^{commit}" 2>$null
    $headExists = $LASTEXITCODE -eq 0
    $isAncestor = $false
    if ($baseExists -and $headExists) {
        & git -C $RepoRoot merge-base --is-ancestor $BaseSha $HeadSha 2>$null
        $isAncestor = $LASTEXITCODE -eq 0
    }
    if ($null -eq $ChangedPaths) {
        $ChangedPaths = if ($isAncestor) {
            @(Get-ElonAppUiChangedPaths -RepoRoot $RepoRoot -BaseSha $BaseSha -HeadSha $HeadSha)
        } else { @() }
    }

    $androidChanged = @($ChangedPaths | Where-Object { $_ -like 'android/*' }).Count -gt 0
    $staticPwaInputs = @(Get-ElonStaticMobilePwaInputPaths)
    $staticPwaChanges = @($ChangedPaths | Where-Object { $_ -in $staticPwaInputs })
    $webTemplateChanged = $staticPwaChanges.Count -gt 0
    $otherServerChanges = @($ChangedPaths | Where-Object {
        $_ -like 'server/*' -and $_ -notin $staticPwaInputs
    })
    $pcFrontendChanged = @($ChangedPaths | Where-Object { $_ -like 'pc-frontend/*' }).Count -gt 0

    $mobilePwaMode = if (-not $isAncestor) {
        'full_server'
    } elseif ($otherServerChanges.Count -gt 0 -or $pcFrontendChanged) {
        'full_server'
    } elseif ($webTemplateChanged) {
        'static_template'
    } else {
        'none'
    }

    $reason = switch ($mobilePwaMode) {
        'full_server' {
            if (-not $isAncestor) { 'base_sha_not_ancestor' }
            elseif ($otherServerChanges.Count -gt 0) { 'server_runtime_or_embedded_asset_changed' }
            else { 'pc_frontend_changed' }
        }
        'static_template' { 'only_runtime_mobile_pwa_assets_changed' }
        default { 'no_mobile_pwa_change' }
    }

    [PSCustomObject]@{
        BaseSha = $BaseSha
        HeadSha = $HeadSha
        ChangedPaths = @($ChangedPaths)
        AndroidChanged = $androidChanged
        WebTemplateChanged = $webTemplateChanged
        StaticPwaChanges = $staticPwaChanges
        OtherServerChanges = $otherServerChanges
        MobilePwaMode = $mobilePwaMode
        Reason = $reason
    }
}

function Get-ElonDeployedServerSha {
    param([string]$VersionUrl = 'http://43.139.149.158:8080/api/server/version')

    try {
        $response = Invoke-RestMethod -Uri $VersionUrl -TimeoutSec 10
        $sha = [string]$response.gitSha
        if ($sha -match '^[0-9a-f]{40}$') { return $sha }
    } catch {
        Write-Warning "Unable to read deployed server SHA: $_"
    }
    return $null
}
