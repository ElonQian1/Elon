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

Assert-Contains $smoke "ExpectedHardwareSerial" `
    "Feature-page smoke must pin a wireless device to its hardware identity."
Assert-Contains $smoke "ExpectedAdapterVersion" `
    "Feature-page smoke must pin the expected adapter contract."
Assert-Contains $smoke "Assert-ChatGptWebSmokeAdapterVersion -State `$origin" `
    "Feature-page smoke must reject an unexpected adapter version."
Assert-Contains $smoke "Assert-ChatGptWebSmokeTrustedDevice" `
    "Feature-page smoke must verify the pinned physical device."
Assert-Contains $runtime "function Wait-ChatGptWebSmokeAuthenticatedReady" `
    "Runtime must centralize bounded bridge recovery before navigation."
Assert-Contains $runtime "function Open-ChatGptWebSmokeSurface" `
    "Runtime must centralize idempotent ChatGPT Web surface entry."
Assert-Contains $runtime 'Open-WebChatNativeChatSurface -Runtime $Runtime -ProviderId "chatgpt_web"' `
    "Runtime must enter the production ChatGPT friend-chat surface."
$readyFunctionStart = $runtime.IndexOf("function Wait-ChatGptWebSmokeAuthenticatedReady")
$readyFunctionEnd = $runtime.IndexOf("function ", $readyFunctionStart + 1)
if ($readyFunctionStart -lt 0 -or $readyFunctionEnd -le $readyFunctionStart) {
    throw "Runtime must expose a bounded authenticated bridge wait."
}
$readyFunction = $runtime.Substring($readyFunctionStart, $readyFunctionEnd - $readyFunctionStart)
if ($readyFunction.Contains('chatgpt_refresh')) {
    throw "Authenticated bridge recovery must preserve the warm identity page instead of reloading it."
}
Assert-Contains $readyFunction 'resumed authenticated ChatGPT Web bridge' `
    "Runtime must keep waiting for the existing bounded recovery coordinator."
Assert-Contains $readyFunction '-EnsureMainActivity | Out-Null' `
    "Runtime must rebind a warm MCP endpoint to the existing foreground task without reloading."
Assert-Contains $runtime '$state.adapter_current -eq $true' `
    "Runtime must accept only the current trusted adapter."
Assert-Contains $smoke "Wait-ChatGptWebSmokeAuthenticatedReady -Runtime `$runtime" `
    "Feature-page smoke must await a ready bridge before feature controls."
Assert-Contains $smoke "Open-ChatGptWebSmokeSurface -Runtime `$runtime" `
    "Feature-page smoke must use the idempotent surface entry helper."
if ($smoke -notmatch '(?s)CHATGPT_FEATURE_PAGE_PHASE phase=bootstrap.*Open-ChatGptWebSmokeSurface.*Wait-ChatGptWebSmokeAuthenticatedReady.*Assert-ChatGptWebSmokeAdapterVersion') {
    throw "Feature-page smoke must recover the production bridge before auditing features."
}
if ($smoke.Contains("chatgpt_select_view")) {
    throw "Feature-page smoke must not route through the retired test-page view switch."
}
Assert-Contains $smoke "function Get-RemainingSeconds" `
    "Feature-page smoke must enforce one deadline across nested bridge waits."
Assert-Contains $smoke "Get-RemainingSeconds -Deadline `$deadline" `
    "Feature-page smoke must pass only remaining time to nested waits."
Assert-Contains $runtime '($deadline - [DateTimeOffset]::UtcNow).TotalSeconds' `
    "Bridge recovery must consume the original timeout instead of resetting it."
Assert-Contains $smoke "TotalTimeoutSec = 180" `
    "Feature-page smoke must expose one bounded total acceptance budget."
Assert-Contains $smoke "function Get-StepDeadline" `
    "Feature-page steps must share the total acceptance deadline."
Assert-Contains $smoke "CHATGPT_FEATURE_PAGE_PHASE phase=bootstrap" `
    "Feature-page smoke must expose a content-free bootstrap progress marker."
Assert-Contains $smoke 'CHATGPT_FEATURE_PAGE_START kind=$kind' `
    "Feature-page smoke must identify the safe feature kind before navigation."
Assert-Contains $smoke 'failed_kinds=$failedKinds' `
    "Feature-page failures must identify only allowlisted feature kinds."
Assert-Contains $runtime "Invoke-ElonNativeCommand" "Runtime must bound native adb commands."
Assert-Contains $runtime "function Invoke-ChatGptWebSmokeAdb" `
    "Runtime must expose a bounded adb command helper."
Assert-Contains $runtime "Verification is deferred" "Missing device must be reported as deferred."
if ($runtime.Contains('adb connect') -or $runtime.Contains('connect",')) {
    throw "Feature-page smoke runtime must not create a wireless adb connection."
}
Assert-Contains $smoke '$safeFeatureCases = [ordered]@{' `
    "Feature-page smoke must define one ordered allowlist and evidence mapping."
foreach ($kind in @("images", "library", "tasks", "apps", "projects", "gpts", "work")) {
    Assert-Contains $smoke "$kind = `"safe/feature_page/$kind`"" `
        "Feature-page smoke must map $kind to its own verification case."
}
Assert-Contains $smoke '$safeKinds = @($safeFeatureCases.Keys)' `
    "Feature-page navigation must derive from the verification case allowlist."
if ($smoke -match '(?m)^\s*(?:health|finances)\s*=\s*"safe/feature_page/') {
    throw "Sensitive Health and Finances pages must not enter unattended feature-page smoke."
}
Assert-Contains $smoke 'chatgpt_select_feature' "Feature-page smoke must use the stable MCP action."
Assert-Contains $smoke '-Action "chatgpt_get_navigation"' `
    "Feature-page smoke must read features from the dedicated navigation API."
Assert-Contains $smoke '$navigation.features | Where-Object { $null -ne $_ }' `
    "Feature-page smoke must not count a null navigation payload as one feature."
Assert-Contains $smoke 'command_receipt.request_id' "Feature-page smoke must await durable command receipts."
Assert-Contains $smoke 'Test-ChatGptWebFeatureMatrix' "Feature-page smoke must use shared structural policy."
Assert-Contains $smoke '$safeFeatureCases[[string]$_.kind]' `
    "Feature-page smoke must register evidence for each page that passed its own audit."
Assert-Contains $smoke 'function Wait-FeatureMatrix' `
    "Feature-page smoke must wait for the routed page manifest before auditing it."
Assert-Contains $smoke '[string]$last.manifest.compatibility -eq "healthy"' `
    "Feature-page smoke must not audit a stale or still-loading manifest."
Assert-Contains $smoke '$matrix = Wait-FeatureMatrix -Kind $kind' `
    "Feature-page smoke must audit the settled current feature manifest."
Assert-Contains $smoke 'function Restore-Origin' "Feature-page smoke must restore the original page."
Assert-Contains $smoke 'chatgpt_open_conversation' `
    "Feature-page smoke must restore a saved conversation through the stable MCP route."
Assert-Contains $smoke 'conversation_path = $Path' `
    "Feature-page smoke must restore the exact original conversation path."
Assert-Contains $smoke '[int]$origin.input.text_length -gt 0' `
    "Feature-page smoke must preserve a non-empty draft instead of replacing it."
Assert-Contains $smoke 'Wait-CommandAndPage -RequestId $requestId -PageKind $PageKind' `
    "Feature-page smoke must await the durable restoration receipt and route."

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
