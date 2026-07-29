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
Assert-Contains $validator 'server/src/assets/web_page.html'
Assert-Contains $validator 'FAST_LANE_RENDERER=skipped'
Assert-Contains $validator 'Start-Process'
Assert-Contains $publisher 'publish-server.ps1'
Assert-Contains $publisher 'publish-apk.ps1'
Assert-Contains $publisher 'APP_UI_RELEASE_POLICY=publish_before_optional_renderer'
if ($publisher.IndexOf('publish-server.ps1') -gt $publisher.IndexOf('publish-apk.ps1')) {
    throw "Mobile PWA/server must publish before APK."
}
Assert-Contains $workflow 'ADB'
Assert-Contains $workflow 'pc-frontend'
Assert-Contains $workflow 'publish Server/PWA'
Assert-Contains $workflow 'publish APK'
Assert-Contains $workflow 'optional Renderer verification'
Assert-Contains $workflow 'VERIFICATION_DEFERRED'
Assert-Contains $sharedContract 'APP_UI_RELEASE_POLICY=publish_before_optional_renderer'
Assert-Contains $sharedContract 'VERIFICATION_DEFERRED'
Assert-Contains $rendererWorkflow 'VERIFICATION_DEFERRED'
Assert-Contains $rendererWorkflow 'realDeviceRequired=true'
Assert-Contains $uiSkill 'must not block Server/PWA or APK publication or repository finish'

& node (Join-Path $repoRoot "scripts\check-mobile-pwa-source.js")
if ($LASTEXITCODE -ne 0) { throw "Mobile PWA source check failed." }

& powershell -NoProfile -ExecutionPolicy Bypass -File `
    (Join-Path $repoRoot "scripts\validate-app-ui-fast-lane.ps1") `
    -NoContractReason "workflow self-test" -PlanOnly
if ($LASTEXITCODE -ne 0) { throw "Fast-lane plan check failed." }

Write-Host "APP_UI_FAST_LANE_TESTS=passed"
