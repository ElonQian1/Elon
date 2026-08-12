$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$runtime = Get-Content (Join-Path $repoRoot "scripts/chatgpt-web-smoke-runtime.ps1") -Raw
$expected = [ordered]@{
    "smoke-chatgpt-web-apk.ps1" = @("safe/read_only_surface", "reversible/send_probe")
    "smoke-chatgpt-web-reversible-controls.ps1" = @("reversible/reversible_controls")
    "smoke-chatgpt-web-tool-execution.ps1" = @("reversible/tool_execution_with_citations")
    "smoke-chatgpt-web-composer-controls.ps1" = @("reversible/composer_controls")
    "smoke-chatgpt-web-message-structure.ps1" = @("reversible/message_structure")
    "smoke-chatgpt-web-copy.ps1" = @("reversible/copy_receipt_without_content_readback")
    "smoke-chatgpt-web-regenerate.ps1" = @("reversible/regenerate_response")
    "smoke-chatgpt-web-message-actions.ps1" = @("safe/message_actions")
    "smoke-chatgpt-web-feature-pages.ps1" = @("safe/feature_pages", "safe/feature_pages_individual")
    "smoke-chatgpt-web-settings.ps1" = @(
        "safe/settings_overlay_form_controls",
        "safe/settings_overlay_idempotent_form_controls"
    )
    "smoke-chatgpt-web-session-recovery.ps1" = @("safe/session_recovery")
}

foreach ($token in @(
    "function Register-ChatGptWebVerificationCases",
    'Action "chatgpt_record_verification_cases"',
    "expected_adapter_version = `$ExpectedAdapterVersion",
    "verification_evidence.current_case_ids"
)) {
    if ($runtime -notlike "*$token*") {
        throw "ChatGPT Web smoke runtime is missing verification evidence token: $token"
    }
}

foreach ($entry in $expected.GetEnumerator()) {
    $content = Get-Content (Join-Path $repoRoot "scripts/$($entry.Key)") -Raw
    if ($content -notlike "*Register-ChatGptWebVerificationCases*") {
        throw "Smoke script does not register verification evidence: $($entry.Key)"
    }
    foreach ($caseId in $entry.Value) {
        if ($content -notlike "*$caseId*") {
            throw "Smoke script is missing verification case '$caseId': $($entry.Key)"
        }
    }
}

$adapterSource = Get-Content (Join-Path $repoRoot `
    "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt") -Raw
$adapterMatch = [regex]::Match($adapterSource, "ADAPTER_VERSION\s*=\s*(\d+)")
if (-not $adapterMatch.Success) { throw "Unable to read ChatGPT Web adapter version." }
$adapterVersion = [int]$adapterMatch.Groups[1].Value
Get-ChildItem (Join-Path $repoRoot "scripts") -Filter "*chatgpt-web*.ps1" | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    $matches = [regex]::Matches($content, "ExpectedAdapterVersion\s*=\s*(\d+)")
    foreach ($match in $matches) {
        if ([int]$match.Groups[1].Value -ne $adapterVersion) {
            throw "Stale ChatGPT Web adapter default in $($_.Name): $($match.Groups[1].Value)"
        }
    }
}

Write-Output "ChatGPT Web verification evidence contracts passed."
