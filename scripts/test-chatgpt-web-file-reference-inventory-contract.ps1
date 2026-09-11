#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$path = Join-Path $PSScriptRoot 'smoke-chatgpt-web-file-reference-inventory.ps1'
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null; $errors = $null
[System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count) { throw 'inventory_parse_failed' }
foreach ($required in @(
    'Assert-ChatGptWebSmokeTrustedDevice', 'Get-ChatGptWebSmokeUserReadiness',
    'Test-WebChatNativeChatSurfaceForeground', 'existing_work_in_progress',
    'Start-ChatGptWebSmokeAwakeLease', 'Stop-ChatGptWebSmokeAwakeLease',
    'Wait-ChatGptCommandReceipt', 'chatgpt_list_conversation_files',
    "[ValidateRange(1, 12)]", 'Select-Object -Skip $CandidateOffset -First $Limit',
    'Open $origin.social_chat.web_chat_conversation_path',
    '$after.input.text -eq $origin.input.text',
    '$report.inventory_completed = $true',
    '@($report.cases | Where-Object { -not $_.index_read }).Count -eq 0',
    '$report.passed -and $report.restored -and $report.awake_restored',
    'sent_messages = 0', 'downloaded_files = 0', 'content_exported = $false',
    'Select-Object method, path, status', 'protocol_dropped'
    'Invoke-AndroidSemanticAcceptance', "Ui 'current_settings'", "Ui 'files_refresh'", "Ui 'files_wait'", "Ui 'back'"
    '[switch]$VerifyRefreshInPlace', 'refresh_check_requires_native_menu', "Ui 'files_refresh_stable'",
    'refresh_receipt_unconfirmed', 'duplicate_pending_read'
)) {
    if (-not $source.Contains($required)) { throw "inventory_guard_missing: $required" }
}
foreach ($forbidden in @('chatgpt_send_message', 'chatgpt_download_', 'chatgpt_delete_',
    'chatgpt_attach_', 'uiautomator', 'screencap', 'pm clear', 'Clear-WebView')) {
    if ($source.Contains($forbidden)) { throw "inventory_write_or_export_forbidden: $forbidden" }
}
$passedAssignment = $source.IndexOf('$report.passed = $candidates.Count -gt 0')
if ($passedAssignment -lt 0 -or $source.Contains('$report.passed = $true')) {
    throw 'inventory_cannot_pass_empty_or_failed_cases'
}
Write-Output 'CHATGPT_FILE_REFERENCE_INVENTORY_CONTRACT=passed'
