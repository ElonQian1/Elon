param(
    [Parameter(Mandatory = $true)]
    [string]$Changelog
)

$ErrorActionPreference = "Stop"
$repoRoot = (& git rev-parse --show-toplevel 2>&1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Run this script inside the repository."
}
$repoRoot = $repoRoot.Trim()

$status = & git -C $repoRoot status --porcelain --untracked-files=all
if ($LASTEXITCODE -ne 0) {
    throw "Unable to inspect repository status."
}
if ($status) {
    throw "Fast-lane publishing requires a clean committed worktree."
}

$watch = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "APP_UI_PUBLISH_ORDER=mobile_pwa_then_apk"
Write-Host "APP_UI_RENDERER=skipped"

& (Join-Path $PSScriptRoot "publish-server.ps1")
if ($LASTEXITCODE -ne 0) {
    throw "Mobile PWA/server publishing failed with exit code $LASTEXITCODE."
}
Write-Host "MOBILE_PWA_PUBLISH=passed"

& (Join-Path $PSScriptRoot "publish-apk.ps1") -Changelog $Changelog
if ($LASTEXITCODE -ne 0) {
    throw "APK publishing failed with exit code $LASTEXITCODE."
}

$watch.Stop()
Write-Host "APK_PUBLISH=passed"
Write-Host "APP_UI_PUBLISH_DURATION_SECONDS=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
Write-Host "APP_UI_PUBLISH_RESULT=passed"

