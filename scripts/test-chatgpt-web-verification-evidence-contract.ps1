$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$runtime = Get-Content (Join-Path $repoRoot "scripts/chatgpt-web-smoke-runtime.ps1") -Raw
$expected = [ordered]@{
    "smoke-chatgpt-web-apk.ps1" = @(
        "safe/read_only_surface",
        "safe/authenticated_session",
        "safe/account_menu_structure",
        "reversible/send_probe"
    )
    "smoke-chatgpt-web-reversible-controls.ps1" = @("reversible/reversible_controls")
    "smoke-chatgpt-web-tool-execution.ps1" = @(
        "reversible/tool_execution_with_citations",
        "reversible/composer_tool_execution/deep_research",
        "reversible/composer_tool_execution/image_generation",
        "reversible/composer_tool_execution/canvas",
        "reversible/composer_tool_execution/study_mode",
        "supervised/composer_tool_execution/agent_mode"
    )
    "smoke-chatgpt-web-composer-controls.ps1" = @(
        "reversible/composer_controls",
        "reversible/composer_tool_discovery/deep_research",
        "reversible/composer_tool_discovery/image_generation",
        "reversible/composer_tool_discovery/canvas",
        "reversible/composer_tool_discovery/study_mode",
        "reversible/composer_tool_discovery/agent_mode"
    )
    "smoke-chatgpt-web-attachment-lifecycle.ps1" = @(
        "supervised/attachment_lifecycle"
    )
    "smoke-chatgpt-web-audio-lifecycle.ps1" = @(
        "supervised/dictation_transcription",
        "supervised/realtime_voice_round_trip"
    )
    "smoke-chatgpt-web-message-structure.ps1" = @("reversible/message_structure")
    "smoke-chatgpt-web-copy.ps1" = @("reversible/copy_receipt_without_content_readback")
    "smoke-chatgpt-web-regenerate.ps1" = @("reversible/regenerate_response")
    "smoke-chatgpt-web-message-actions.ps1" = @("safe/message_actions")
    "smoke-chatgpt-web-feature-pages.ps1" = @(
        "safe/feature_pages",
        "safe/feature_page/projects",
        "safe/feature_page/tasks",
        "safe/feature_page/library",
        "safe/feature_page/gpts",
        "safe/feature_page/apps",
        "safe/feature_page/work"
    )
    "smoke-chatgpt-web-settings.ps1" = @(
        "safe/settings_overlay_form_controls",
        "safe/settings_overlay_idempotent_form_controls"
    )
    "smoke-chatgpt-web-session-recovery.ps1" = @("safe/session_recovery")
    "smoke-chatgpt-web-conversation-management.ps1" = @(
        "safe/conversation_management_structure"
    )
    "smoke-chatgpt-web-long-running-stability.ps1" = @(
        "safe/session_long_running_stability"
    )
    "smoke-chatgpt-web-sensitive-feature-page.ps1" = @(
        "supervised/feature_page/health",
        "supervised/feature_page/finances"
    )
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
