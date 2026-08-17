$ErrorActionPreference = "Stop"

$source = Get-Content -LiteralPath (
    Join-Path $PSScriptRoot "smoke-chatgpt-web-tool-execution.ps1"
) -Raw

foreach ($required in @(
    'Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',
    '[ValidateSet("web_search", "deep_research", "image_generation", "canvas", "study_mode", "agent_mode")]',
    'CHATGPT_WEB_TOOL_EXECUTION_STATUS=user_action_required required_action=confirm_agent_mode',
    'Invoke-ReceiptAction -Action "chatgpt_new_conversation"',
    'Where-Object { [string]$_.semantic -eq [string]$toolSpec.semantic }',
    'Invoke-ReceiptAction -Action "chatgpt_select_composer_option"',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"',
    'function Wait-ToolReply',
    '$messages.Count -ge ($InitialMessageCount + 2)',
    'expected_structural_part_types = @($toolSpec.expected_parts)',
    '-CaseIds @([string]$toolSpec.case_id)',
    'if (-not $toolStateRestored -and $enabledBySmoke)',
    'Restore-ToolSelection',
    'Restore-Origin -ConversationPath $originPath',
    'production_surface_preserved = Test-ChatGptWebSmokeActivityForeground',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false'
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT tool execution smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    'chatgpt_select_view',
    'Write-Output $marker',
    'conversation_url =',
    'assistant_content ='
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT tool execution smoke leaks private or probe data: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_TOOL_EXECUTION_SMOKE_CONTRACT=passed"
