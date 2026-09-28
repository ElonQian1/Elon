param()

$ErrorActionPreference = "Stop"
$repoRoot = (& git rev-parse --show-toplevel 2>&1).Trim()
if ($LASTEXITCODE -ne 0) { throw "Run inside the repository." }

function Assert-Contains {
    param([string]$Text, [string]$Expected)
    if (-not $Text.Contains($Expected)) { throw "Missing required text: $Expected" }
}

$validator = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "scripts\validate-app-ui-fast-lane.ps1")
$publisher = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "scripts\publish-app-ui-fast-lane.ps1")
$workflow = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "docs\app-ui-fast-lane.md")
$sharedContract = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".github\copilot-instructions.md")
$rendererWorkflow = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "docs\android-real-renderer-ui-workflow.md")
$uiSkill = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".agents\skills\yilong-ui-design\SKILL.md")

Assert-Contains $validator ':app:testDebugUnitTest'
Assert-Contains $validator ':app:assembleDebug'
Assert-Contains $validator 'server/src/assets/*'
Assert-Contains $validator 'NoPwaImpactReason'
Assert-Contains $validator 'FAST_LANE_RENDERER=skipped'
Assert-Contains $validator 'Start-Process'
Assert-Contains $validator '[void]$androidProcess.Handle'
Assert-Contains $validator '[void]$pwaProcess.Handle'
Assert-Contains $publisher 'publish-server.ps1'
Assert-Contains $publisher 'publish-mobile-pwa-static.ps1'
Assert-Contains $publisher 'StaticRuntimePwa'
Assert-Contains $publisher 'TaskBaseSha'
Assert-Contains $publisher 'TaskScopeBaseSha'
Assert-Contains $publisher 'APP_UI_TASK_BASE_SHA'
Assert-Contains $publisher 'APP_UI_TASK_SCOPE_BASE_SHA'
Assert-Contains $publisher 'APP_UI_DEPLOYED_SERVER_SHA'
Assert-Contains $publisher 'APP_UI_DEPLOYMENT_DEBT_PATHS'
Assert-Contains $publisher "cannot override task scope"
Assert-Contains $publisher 'app-ui-change-scope.ps1'
Assert-Contains $publisher '-SkipPcFrontend'
Assert-Contains $publisher 'publish-apk.ps1'
Assert-Contains $publisher '-AllowAdbVerificationDeferred'
Assert-Contains $publisher 'APP_UI_RELEASE_POLICY=publish_before_optional_renderer'
Assert-Contains $publisher '-VerifyOnly'
Assert-Contains $publisher 'receipt and remote artifact verified'
$fullServerBranch = [regex]::Match(
    $publisher,
    '(?s)elseif \(\$scope\.MobilePwaMode -eq ''full_server''\).*?^\s*}\s*else\s*{',
    [System.Text.RegularExpressions.RegexOptions]::Multiline
).Value
Assert-Contains $fullServerBranch 'publish-server.ps1'
Assert-Contains $fullServerBranch 'publish-mobile-pwa-static.ps1'
if ($publisher.IndexOf('publish-server.ps1') -gt $publisher.IndexOf('publish-apk.ps1')) {
    throw "Mobile PWA/server must publish before APK."
}
Assert-Contains $workflow 'mobile-design-system-v2.md'
Assert-Contains $workflow 'NoPwaImpactReason'
Assert-Contains $workflow 'NoContractReason'
Assert-Contains $workflow 'publish-before-optional-renderer'
Assert-Contains $workflow 'invoke-ai-logged-command.ps1'
Assert-Contains $workflow 'AndroidFeature'
Assert-Contains $sharedContract 'APP_UI_RELEASE_POLICY=publish_before_optional_renderer'
Assert-Contains $sharedContract 'VERIFICATION_DEFERRED'
Assert-Contains $rendererWorkflow 'VERIFICATION_DEFERRED'
Assert-Contains $rendererWorkflow 'realDeviceRequired=true'
Assert-Contains $rendererWorkflow 'RENDERER_PREPARATION_ATTEMPTS=0'
Assert-Contains $uiSkill 'native runtime evidence precede formal publication'
Assert-Contains $publisher 'SYSTEM_UI_REFACTOR_REQUIRES_RUNTIME_EVIDENCE'
. (Join-Path $repoRoot 'scripts/mobile-ui-design-scope.ps1')
foreach ($path in @('docs/design/mobile-tokens-v2.json', 'android/app/src/main/res/values-night/themes.xml',
    'scripts/templates/mobile-design-v2.css')) {
    if (-not (Test-ElonSystemicMobileDesignChange -Paths @($path))) { throw "Systemic change entered fast lane: $path" }
}
if (Test-ElonSystemicMobileDesignChange -Paths @('android/app/src/main/res/drawable/ic_search.xml')) {
    throw 'An isolated icon correction was incorrectly classified as a system reconstruction.'
}
$postflight = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'scripts\apk-publish-postflight.ps1')
Assert-Contains $postflight 'apk-adb-autodeploy.ps1'
Assert-Contains $postflight 'Invoke-ElonApkAdbAutodeploy -ApkPath $ApkPath -ExpectedVersionCode $ExpectedVersionCode'
$lanDistClient = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'scripts\lan-dist-client.ps1')
Assert-Contains $lanDistClient '-RedirectStandardOutput "$LogFile.stdout"'

& node (Join-Path $repoRoot "scripts\check-mobile-pwa-source.js")
if ($LASTEXITCODE -ne 0) { throw "Mobile PWA source check failed." }

& powershell -NoProfile -ExecutionPolicy Bypass -File `
    (Join-Path $repoRoot 'scripts\test-app-ui-task-push-scope.ps1')
if ($LASTEXITCODE -ne 0) { throw 'APP UI task push scope tests failed.' }

& powershell -NoProfile -ExecutionPolicy Bypass -File `
    (Join-Path $repoRoot "scripts\validate-app-ui-fast-lane.ps1") `
    -NoContractReason "workflow self-test" -PlanOnly
if ($LASTEXITCODE -ne 0) { throw "Fast-lane plan check failed." }

Write-Host "APP_UI_FAST_LANE_TESTS=passed"
