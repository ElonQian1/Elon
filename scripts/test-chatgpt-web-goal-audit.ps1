#requires -Version 5.1

$ErrorActionPreference = "Stop"
$scriptPath = Join-Path $PSScriptRoot "chatgpt-web-goal-audit.ps1"
$source = Get-Content -LiteralPath $scriptPath -Raw
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $scriptPath,
    [ref]$tokens,
    [ref]$parseErrors
)
if (@($parseErrors).Count -gt 0) {
    throw "ChatGPT Web goal audit has parse errors: $($parseErrors[0].Message)"
}

foreach ($required in @(
    'elon.chatgpt_web.goal_audit.v1',
    'requires_chatgpt_production_surface',
    'chatgpt_get_capability_matrix',
    'current_case_ids',
    'missing_scripted_case_ids',
    'missing_manual_case_ids',
    'pending_verification_feature_ids',
    'unknown_capability_count',
    'unknown_semantic_count',
    '-Tool "ui_state" -MainState',
    'private_content_emitted = $false',
    'CHATGPT_WEB_GOAL_AUDIT_STATUS='
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT Web goal audit is missing: $required"
    }
}
foreach ($forbidden in @(
    'social_chat.messages',
    'conversation_path',
    'conversation_title',
    'document.cookie',
    'removeAllCookies',
    'pm clear'
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT Web goal audit contains forbidden private or destructive behavior: $forbidden"
    }
}

$output = @(& $scriptPath -SelfTest)
if (($output -join "`n") -notmatch 'CHATGPT_WEB_GOAL_AUDIT_SELF_TEST=passed') {
    throw "ChatGPT Web goal audit self-test did not pass."
}

Write-Output "CHATGPT_WEB_GOAL_AUDIT_CONTRACT=passed"
