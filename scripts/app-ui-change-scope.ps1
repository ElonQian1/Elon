function Get-ElonAppUiChangedPaths {
    param(
        [Parameter(Mandatory)] [string]$RepoRoot,
        [Parameter(Mandatory)] [string]$BaseSha,
        [Parameter(Mandatory)] [string]$HeadSha
    )

    if ($BaseSha -eq $HeadSha) { return @() }
    $paths = & git -C $RepoRoot diff --name-only --diff-filter=ACMR $BaseSha $HeadSha 2>$null
    if ($LASTEXITCODE -ne 0) { throw "Unable to inspect changes between $BaseSha and $HeadSha" }
    @($paths | ForEach-Object { $_.Trim() -replace '\\', '/' } | Where-Object { $_ })
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
    $webTemplateChanged = $ChangedPaths -contains 'server/src/assets/web_page.html'
    $otherServerChanges = @($ChangedPaths | Where-Object {
        $_ -like 'server/*' -and $_ -ne 'server/src/assets/web_page.html'
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
        'static_template' { 'only_runtime_mobile_pwa_template_changed' }
        default { 'no_mobile_pwa_change' }
    }

    [PSCustomObject]@{
        BaseSha = $BaseSha
        HeadSha = $HeadSha
        ChangedPaths = @($ChangedPaths)
        AndroidChanged = $androidChanged
        WebTemplateChanged = $webTemplateChanged
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
