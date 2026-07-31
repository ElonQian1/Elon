param()

$ErrorActionPreference = 'Stop'
$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Run this test inside the repository.'
}
$nativeCommandHelper = Join-Path $repoRoot 'scripts\native-command-timeout.ps1'
$releaseReceiptHelper = Join-Path $repoRoot 'scripts\release-stage-receipt.ps1'
$changeScopeHelper = Join-Path $repoRoot 'scripts\app-ui-change-scope.ps1'
$apkTransportHelper = Join-Path $repoRoot 'scripts\apk-publish-transport.ps1'
. $nativeCommandHelper
. $releaseReceiptHelper
. $changeScopeHelper
. $apkTransportHelper

if (-not (Get-Command New-ElonApkAtomicDeployScript -ErrorAction SilentlyContinue)) {
    throw 'APK publish transport functions were not loaded.'
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$success = Invoke-ElonNativeCommand -FilePath 'cmd.exe' -TimeoutSeconds 10 -Label 'native success' `
    -ArgumentList @('/d', '/s', '/c', 'echo native-ok')
Assert-True ($success.ExitCode -eq 0) 'Native command success exit code was not preserved.'
Assert-True ($success.Stdout.Contains('native-ok')) 'Native command stdout was not captured.'
$failure = Invoke-ElonNativeCommand -FilePath 'cmd.exe' -TimeoutSeconds 10 -Label 'native failure' `
    -ArgumentList @('/d', '/s', '/c', 'exit 7')
Assert-True ($failure.ExitCode -eq 7 -and -not $failure.TimedOut) `
    'Native command failure exit code was not preserved.'

$timeoutWatch = [System.Diagnostics.Stopwatch]::StartNew()
$timeout = Invoke-ElonNativeCommand -FilePath 'powershell.exe' -TimeoutSeconds 1 -Label 'native timeout' `
    -ArgumentList @('-NoProfile', '-Command', 'Start-Sleep -Seconds 30')
$timeoutWatch.Stop()
Assert-True ($timeout.ExitCode -eq 124 -and $timeout.TimedOut) 'Native command timeout was not classified.'
Assert-True ($timeoutWatch.Elapsed.TotalSeconds -lt 8) 'Native command timeout did not terminate promptly.'

$head = (& git -C $repoRoot rev-parse HEAD).Trim()
$staticScope = Resolve-ElonAppUiChangeScope -RepoRoot $repoRoot -BaseSha $head -HeadSha $head `
    -ChangedPaths @('android/app/src/main/res/layout/activity_main.xml', 'server/src/assets/web_page.html')
Assert-True ($staticScope.MobilePwaMode -eq 'static_template') 'Template-only UI change must use static PWA publishing.'
Assert-True $staticScope.AndroidChanged 'Android UI change was not detected.'

$serverScope = Resolve-ElonAppUiChangeScope -RepoRoot $repoRoot -BaseSha $head -HeadSha $head `
    -ChangedPaths @('server/src/web.rs', 'server/src/assets/web_page.html')
Assert-True ($serverScope.MobilePwaMode -eq 'full_server') 'Server runtime change must use full server publishing.'

$noneScope = Resolve-ElonAppUiChangeScope -RepoRoot $repoRoot -BaseSha $head -HeadSha $head `
    -ChangedPaths @('scripts/test-app-ui-fast-lane.ps1')
Assert-True ($noneScope.MobilePwaMode -eq 'none') 'Unrelated script change must not publish the mobile PWA.'

$receipt = New-ElonReleaseReceipt -RepoRoot $repoRoot -Kind 'optimization-test' -SourceSha $head
Set-ElonReleaseStageReceipt -Receipt $receipt -Stage 'sample' -Status passed -DurationSeconds 1.2 -Message 'ok'
Assert-True (Test-ElonReleaseStagePassed -Receipt $receipt -Stage 'sample') 'Passed receipt stage was not resumable.'
$receiptDocument = Read-ElonReleaseReceiptFile -Path $receipt.Path
Assert-True ($receiptDocument.schema -eq 'elon.release.receipt.v1') 'Release receipt schema is wrong.'
$global:LASTEXITCODE = 9
Invoke-ElonReleaseStage -Receipt $receipt -Stage 'powershell_only' -Action { $null = 1 + 1 }
Assert-True (Test-ElonReleaseStagePassed -Receipt $receipt -Stage 'powershell_only') `
    'A stale native exit code poisoned a PowerShell-only release stage.'

$apkPublisher = Get-Content -LiteralPath (Join-Path $repoRoot 'scripts\publish-apk.ps1') -Raw
Assert-True ($apkPublisher.Contains('Assert-RemoteApkArtifact')) 'APK publisher is missing server-side artifact verification.'
Assert-True ($apkPublisher.Contains('sha256      = $apkSha256')) 'APK version metadata is missing SHA-256.'
Assert-True (-not $apkPublisher.Contains('elon-remote-apk-')) 'APK publisher still downloads the full remote APK.'
$script:ServerDir = '/tmp/elon-app-test'
$deployScript = New-ElonApkAtomicDeployScript -ApkStage '/tmp/app.apk' -JsonStage '/tmp/version.json' `
    -ReleaseSha $head -ExpectedServerSha $head -ExpectedSha256 ('a' * 64)
Assert-True ($deployScript.Contains("awk '{print `$1}'")) 'APK staging hash command lost its shell field expression.'
Assert-True ($deployScript.Contains('EXPECTED_HASH=' + "'" + ('a' * 64) + "'")) 'APK expected hash was not injected.'

$serverPublisher = Get-Content -LiteralPath (Join-Path $repoRoot 'scripts\publish-server.ps1') -Raw
Assert-True (-not $serverPublisher.Contains("-Phase 'pc_frontend' -Status 'skipped'")) `
    'The server publisher uses a release-ledger status that the API does not accept.'

& powershell -NoProfile -ExecutionPolicy Bypass -File `
    (Join-Path $repoRoot 'scripts\publish-mobile-pwa-static.ps1') -PlanOnly
if ($LASTEXITCODE -ne 0) { throw 'Static mobile PWA plan failed.' }
$staticPublisher = Get-Content -LiteralPath (Join-Path $repoRoot 'scripts\publish-mobile-pwa-static.ps1') -Raw
Assert-True (-not $staticPublisher.Contains('Split-Path $RemotePath')) `
    'The static publisher treats a POSIX remote path as a Windows path.'

Write-Host 'RELEASE_WORKFLOW_OPTIMIZATION_TESTS=passed'
