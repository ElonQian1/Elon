param(
    [Parameter(Mandatory = $true)]
    [string]$Changelog,
    [string]$TaskBaseSha = '',
    [string]$TaskScopeBaseSha = '',
    [Alias('BaseSha')]
    [string]$DeployedServerSha = '',
    [switch]$StaticRuntimePwa,
    [switch]$PlanOnly,
    [switch]$NoResume
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'app-ui-change-scope.ps1')
. (Join-Path $PSScriptRoot 'release-stage-receipt.ps1')

$repoRoot = (& git rev-parse --show-toplevel 2>&1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Run this script inside the repository.'
}
$repoRoot = $repoRoot.Trim()
$status = & git -C $repoRoot status --porcelain --untracked-files=all
if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect repository status.' }
if ($status) { throw 'Fast-lane publishing requires a clean committed worktree.' }

$headSha = (& git -C $repoRoot rev-parse HEAD).Trim()
$taskBase = Get-ElonAppUiTaskBaseSha -RepoRoot $repoRoot -ExplicitBaseSha $TaskBaseSha
if ($null -eq $taskBase) {
    throw 'APP UI task base is unavailable. Run ai-task-preflight before editing or pass -TaskBaseSha explicitly.'
}
if ([string]::IsNullOrWhiteSpace($DeployedServerSha)) {
    $DeployedServerSha = Get-ElonDeployedServerSha
}
if ([string]::IsNullOrWhiteSpace($DeployedServerSha)) {
    $DeployedServerSha = '0000000000000000000000000000000000000000'
}
$taskScopeBase = Get-ElonAppUiTaskScopeBaseSha `
    -RepoRoot $repoRoot -TaskBase $taskBase -HeadSha $headSha `
    -ExplicitScopeBaseSha $TaskScopeBaseSha
$scope = if ($null -ne $taskScopeBase.ChangedPaths) {
    Resolve-ElonAppUiChangeScope `
        -RepoRoot $repoRoot -BaseSha $taskScopeBase.Sha -HeadSha $headSha `
        -ChangedPaths @($taskScopeBase.ChangedPaths)
} else {
    Resolve-ElonAppUiChangeScope -RepoRoot $repoRoot -BaseSha $taskScopeBase.Sha -HeadSha $headSha
}
$deploymentDebt = Resolve-ElonAppUiChangeScope `
    -RepoRoot $repoRoot -BaseSha $DeployedServerSha -HeadSha $headSha
if ($StaticRuntimePwa -and $scope.MobilePwaMode -ne 'static_template') {
    throw "-StaticRuntimePwa cannot override task scope '$($scope.MobilePwaMode)'. Static publishing is allowed only for self-contained mobile PWA assets."
}
$receipt = New-ElonReleaseReceipt -RepoRoot $repoRoot -Kind 'app-ui' -SourceSha $headSha
$watch = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host 'APP_UI_RELEASE_POLICY=publish_before_optional_renderer'
Write-Host 'APP_UI_PUBLISH_ORDER=mobile_pwa_then_apk'
Write-Host 'APP_UI_RENDERER=skipped'
Write-Host "APP_UI_TASK_BASE_SHA=$($taskBase.Sha)"
Write-Host "APP_UI_TASK_BASE_SOURCE=$($taskBase.Source)"
Write-Host "APP_UI_TASK_SCOPE_BASE_SHA=$($taskScopeBase.Sha)"
Write-Host "APP_UI_TASK_SCOPE_BASE_SOURCE=$($taskScopeBase.Source)"
Write-Host "APP_UI_TASK_SCOPE_PATHS_SOURCE=$($taskScopeBase.Source)"
Write-Host "APP_UI_DEPLOYED_SERVER_SHA=$DeployedServerSha"
Write-Host "APP_UI_HEAD_SHA=$headSha"
Write-Host "APP_UI_MOBILE_PWA_MODE=$($scope.MobilePwaMode)"
Write-Host "APP_UI_SCOPE_REASON=$($scope.Reason)"
Write-Host "APP_UI_TASK_CHANGED_PATHS=$($scope.ChangedPaths.Count)"
Write-Host "APP_UI_DEPLOYMENT_DEBT_PATHS=$($deploymentDebt.ChangedPaths.Count)"
Write-Host "APP_UI_DEPLOYMENT_DEBT_MODE=$($deploymentDebt.MobilePwaMode)"
Write-Host 'APP_UI_SERVER_RELEASE=task_scope_driven'

if ($PlanOnly) {
    Write-Host 'APP_UI_PUBLISH_RESULT=planned'
    exit 0
}

$resumeMobilePwa = $false
if (-not $NoResume -and (Test-ElonReleaseStagePassed -Receipt $receipt -Stage 'mobile_pwa')) {
    try {
        & (Join-Path $PSScriptRoot 'publish-mobile-pwa-static.ps1') -VerifyOnly
        $resumeMobilePwa = $LASTEXITCODE -eq 0
        if ($resumeMobilePwa -and $scope.MobilePwaMode -eq 'full_server') {
            $serverNowSha = Get-ElonDeployedServerSha
            if ([string]::IsNullOrWhiteSpace($serverNowSha)) {
                $resumeMobilePwa = $false
            } else {
                & git -C $repoRoot merge-base --is-ancestor $headSha $serverNowSha 2>$null
                $resumeMobilePwa = $LASTEXITCODE -eq 0
            }
        }
    } catch {
        Write-Host "APP_UI_MOBILE_PWA_RESUME=stale reason=$($_.Exception.Message)"
    }
}
if ($resumeMobilePwa) {
    Write-Host 'RELEASE_STAGE=mobile_pwa status=resumed durationSeconds=0 message=receipt and remote artifact verified'
} elseif ($scope.MobilePwaMode -eq 'static_template') {
    Invoke-ElonReleaseStage -Receipt $receipt -Stage 'mobile_pwa' -SuccessMessage 'static template published' -Action {
        & (Join-Path $PSScriptRoot 'publish-mobile-pwa-static.ps1')
    }
} elseif ($scope.MobilePwaMode -eq 'full_server') {
    Invoke-ElonReleaseStage -Receipt $receipt -Stage 'mobile_pwa' -SuccessMessage 'server runtime and template published' -Action {
        & (Join-Path $PSScriptRoot 'publish-server.ps1') -SkipPcFrontend
        & (Join-Path $PSScriptRoot 'publish-mobile-pwa-static.ps1')
    }
} else {
    Set-ElonReleaseStageReceipt -Receipt $receipt -Stage 'mobile_pwa' -Status skipped `
        -Message 'no mobile PWA change'
}
Write-Host 'MOBILE_PWA_PUBLISH=passed'

# publish-apk.ps1 performs an online input-coverage check before claiming a
# version. Calling it on resume is cheap and protects against server rollback.
Invoke-ElonReleaseStage -Receipt $receipt -Stage 'apk' -SuccessMessage 'APK published or already covered' -Action {
    & (Join-Path $PSScriptRoot 'publish-apk.ps1') -Changelog $Changelog -AllowAdbVerificationDeferred
}

$watch.Stop()
Write-Host 'APK_PUBLISH=passed'
Write-Host "APP_UI_PUBLISH_DURATION_SECONDS=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
Write-Host 'APP_UI_PUBLISH_RESULT=passed'
