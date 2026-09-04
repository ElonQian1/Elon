$ErrorActionPreference = "Stop"

$source = Get-Content -LiteralPath (
    Join-Path $PSScriptRoot "smoke-chatgpt-web-tool-execution.ps1"
) -Raw

foreach ($required in @(
    'Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',
    '[ValidateSet("web_search", "deep_research", "image_generation", "canvas", "study_mode", "agent_mode")]',
    'CHATGPT_WEB_TOOL_EXECUTION_STATUS=user_action_required required_action=confirm_agent_mode',
    'Open-WebChatNativeChatSurface -Runtime $runtime',
    '-Action "start_new_web_chat_conversation"',
    'Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState',
    'Where-Object { [string]$_.semantic -eq [string]$toolSpec.semantic }',
    'Invoke-ReceiptAction -Action "chatgpt_select_composer_option"',
    'Invoke-ReceiptAction -Action "chatgpt_dismiss_composer_options"',
    '-ExpectedAction "dismiss_composer_menu"',
    'foreach ($attempt in 1..3)',
    'was not observed after bounded refresh',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"',
    'function Wait-ProductionSendReceipt',
    'web_chat_last_send_command',
    '[string]$beforeMain.active_surface -ne "social_ai"',
    '[string]$beforeMain.social_chat.web_chat_provider_id -ne "chatgpt_web"',
    'function Wait-ToolReply',
    'function Wait-ToolStructuralReply',
    '$mainMessages.Count -ge ($InitialMainMessageCount + 2)',
    '$adapterMessages.Count -ge ($InitialAdapterMessageCount + 2)',
    'observed_parts=$safeObserved',
    'expected_structural_part_types = @($toolSpec.expected_parts)',
    '-CaseIds @([string]$toolSpec.case_id)',
    '-ProductionSurface',
    'if (-not $toolStateRestored -and $enabledBySmoke)',
    'Restore-ToolSelection',
    'Restore-WebChatNativeConversation -Runtime $runtime',
    '-InputText $originInputText',
    'production_send_receipt_observed = $true',
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
    '@("shell", "input", "keyevent", "4")',
    '$send.command_receipt.request_id',
    'Invoke-ReceiptAction -Action "chatgpt_new_conversation"',
    'Invoke-ReceiptAction -Action "chatgpt_open_conversation"',
    'Write-Output $marker',
    'conversation_url =',
    'assistant_content ='
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT tool execution smoke leaks private or probe data: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_TOOL_EXECUTION_SMOKE_CONTRACT=passed"
