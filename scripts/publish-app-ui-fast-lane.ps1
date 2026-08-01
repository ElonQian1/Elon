param(
    [Parameter(Mandatory = $true)]
    [string]$Changelog,
    [string]$BaseSha = '',
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
if ([string]::IsNullOrWhiteSpace($BaseSha)) {
    $BaseSha = Get-ElonDeployedServerSha
}
if ([string]::IsNullOrWhiteSpace($BaseSha)) {
    $BaseSha = '0000000000000000000000000000000000000000'
}
$scope = Resolve-ElonAppUiChangeScope -RepoRoot $repoRoot -BaseSha $BaseSha -HeadSha $headSha
$receipt = New-ElonReleaseReceipt -RepoRoot $repoRoot -Kind 'app-ui' -SourceSha $headSha
$watch = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host 'APP_UI_RELEASE_POLICY=publish_before_optional_renderer'
Write-Host 'APP_UI_PUBLISH_ORDER=mobile_pwa_then_apk'
Write-Host 'APP_UI_RENDERER=skipped'
Write-Host "APP_UI_BASE_SHA=$BaseSha"
Write-Host "APP_UI_HEAD_SHA=$headSha"
Write-Host "APP_UI_MOBILE_PWA_MODE=$($scope.MobilePwaMode)"
Write-Host "APP_UI_SCOPE_REASON=$($scope.Reason)"
Write-Host "APP_UI_CHANGED_PATHS=$($scope.ChangedPaths.Count)"

if ($PlanOnly) {
    Write-Host 'APP_UI_PUBLISH_RESULT=planned'
    exit 0
}

if (-not $NoResume -and (Test-ElonReleaseStagePassed -Receipt $receipt -Stage 'mobile_pwa')) {
    Write-Host 'RELEASE_STAGE=mobile_pwa status=resumed durationSeconds=0 message=previous receipt passed'
} elseif ($scope.MobilePwaMode -eq 'static_template') {
    Invoke-ElonReleaseStage -Receipt $receipt -Stage 'mobile_pwa' -SuccessMessage 'static template published' -Action {
        & (Join-Path $PSScriptRoot 'publish-mobile-pwa-static.ps1')
    }
} elseif ($scope.MobilePwaMode -eq 'full_server') {
    Invoke-ElonReleaseStage -Receipt $receipt -Stage 'mobile_pwa' -SuccessMessage 'server runtime published' -Action {
        & (Join-Path $PSScriptRoot 'publish-server.ps1') -SkipPcFrontend
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
