$ErrorActionPreference = "Stop"

$source = Get-Content -LiteralPath (
    Join-Path $PSScriptRoot "smoke-chatgpt-web-tool-execution.ps1"
) -Raw

foreach ($required in @(
    'Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',
    'Invoke-ReceiptAction -Action "chatgpt_new_conversation"',
    'Where-Object { [string]$_.semantic -eq "web_search" }',
    'Invoke-ReceiptAction -Action "chatgpt_select_composer_option"',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"',
    'Wait-ChatGptProbeReply',
    '[string]$_.type -eq "citation"',
    'Restore-Origin -ConversationPath $originPath -ViewMode $originMode',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false'
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT tool execution smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    'Write-Output $marker',
    'conversation_url =',
    'assistant_content ='
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT tool execution smoke leaks private or probe data: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_TOOL_EXECUTION_SMOKE_CONTRACT=passed"
