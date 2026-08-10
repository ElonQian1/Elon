$ErrorActionPreference = "Stop"

$smokePath = Join-Path $PSScriptRoot "smoke-chatgpt-web-feature-pages.ps1"
$runtimePath = Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1"
$policyPath = Join-Path $PSScriptRoot "chatgpt-web-feature-audit-policy.ps1"
$smoke = Get-Content -LiteralPath $smokePath -Raw
$runtime = Get-Content -LiteralPath $runtimePath -Raw

function Assert-Contains {
    param([string]$Source, [string]$Needle, [string]$Message)
    if (-not $Source.Contains($Needle)) { throw $Message }
}

foreach ($path in @($smokePath, $runtimePath, $policyPath)) {
    $tokens = $null
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile(
        $path,
        [ref]$tokens,
        [ref]$errors
    )
    if (@($errors).Count -gt 0) {
        throw "PowerShell parse failed for $path`: $($errors[0].Message)"
    }
}

Assert-Contains $runtime "physical USB serial" "Runtime must require an explicit physical USB serial."
Assert-Contains $runtime "Invoke-ElonNativeCommand" "Runtime must bound native adb commands."
Assert-Contains $runtime "function Invoke-ChatGptWebSmokeAdb" `
    "Runtime must expose a bounded USB-only adb command helper."
Assert-Contains $runtime "Verification is deferred" "Missing USB must be reported as deferred."
if ($runtime.Contains('adb connect') -or $runtime.Contains('connect",')) {
    throw "Feature-page smoke runtime must not create a wireless adb connection."
}
Assert-Contains $smoke '$safeKinds = @("library", "tasks", "apps", "projects", "gpts")' `
    "Feature-page smoke must constrain navigation to non-destructive page kinds."
Assert-Contains $smoke 'chatgpt_select_feature' "Feature-page smoke must use the stable MCP action."
Assert-Contains $smoke '-Action "chatgpt_get_navigation"' `
    "Feature-page smoke must read features from the dedicated navigation API."
Assert-Contains $smoke '$navigation.features | Where-Object { $null -ne $_ }' `
    "Feature-page smoke must not count a null navigation payload as one feature."
Assert-Contains $smoke 'command_receipt.request_id' "Feature-page smoke must await durable command receipts."
Assert-Contains $smoke 'Test-ChatGptWebFeatureMatrix' "Feature-page smoke must use shared structural policy."
Assert-Contains $smoke 'function Restore-Origin' "Feature-page smoke must restore the original page."
Assert-Contains $smoke 'Invoke-ChatGptWebSmokeAdb' `
    "Feature-page smoke must use the bounded USB helper for back navigation."
if ($smoke.Contains('chatgpt_new_conversation')) {
    throw "Feature-page smoke must not replace or discard the user's current conversation."
}

. $policyPath
$healthy = [pscustomobject]@{
    control_ok = $true
    ready_for_mcp = $true
    manifest = [pscustomobject]@{
        page_kind = "feature"
        compatibility = "healthy"
        controls_truncated = $false
        control_count = 12
        native_control_count = 10
        generic_control_count = 0
        unexpected_official_fallback_control_count = 0
    }
    unknown_semantics = @()
    unknown_capabilities = @()
    adaptation_review = [pscustomobject]@{ required = $false }
}
$healthyAudit = Test-ChatGptWebFeatureMatrix -Matrix $healthy
if (-not $healthyAudit.passed) { throw "Healthy feature matrix must pass." }

$drifted = $healthy | Select-Object *
$drifted.manifest = $healthy.manifest | Select-Object *
$drifted.manifest.generic_control_count = 2
$drifted.manifest.unexpected_official_fallback_control_count = 1
$drifted.adaptation_review = [pscustomobject]@{ required = $true }
$driftedAudit = Test-ChatGptWebFeatureMatrix -Matrix $drifted
if ($driftedAudit.passed) { throw "Drifted feature matrix must fail." }
foreach ($reason in @(
    "generic_controls_present",
    "unexpected_official_fallback_controls_present",
    "adaptation_review_required"
)) {
    if ($reason -notin @($driftedAudit.reasons)) {
        throw "Drifted feature matrix is missing reason: $reason"
    }
}

Write-Output "CHATGPT_FEATURE_PAGE_SMOKE_CONTRACT=passed"
