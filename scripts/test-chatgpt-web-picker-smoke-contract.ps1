$ErrorActionPreference = "Stop"

$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot "smoke-chatgpt-web-pickers.ps1") -Raw

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) { throw "Picker smoke contract is missing: $Needle" }
}

Assert-Contains '$cameraLabel = ([string][char]0x76F8) + [char]0x673A'
Assert-Contains '$photoLabel = ([string][char]0x7167) + [char]0x7247'
Assert-Contains '$fileLabel = ([string][char]0x6587) + [char]0x4EF6'
Assert-Contains 'chatgpt_select_composer_option'
Assert-Contains 'topResumedActivity='
Assert-Contains 'if ($null -eq $line) { return "" }'
Assert-Contains 'Restore-ChatGptActivity'
Assert-Contains '$pickerPredicate = {'
Assert-Contains '}.GetNewClosure()'
Assert-Contains 'Assert-ChatGptWebSmokeTrustedDevice'
Assert-Contains 'Assert-ChatGptWebSmokeAdapterVersion'
Assert-Contains 'Start-ChatGptWebSmokeAwakeLease'
Assert-Contains 'Stop-ChatGptWebSmokeAwakeLease'
Assert-Contains 'Wait-ChatGptWebSmokeAuthenticatedReady'
Assert-Contains 'Invoke-ChatGptWebSmokeAdb'
Assert-Contains '-EnsureMainActivity'
Assert-Contains 'command_receipt.request_id'
Assert-Contains '$state.command_requests'
Assert-Contains '-ExpectedAction "list_composer_tools"'
Assert-Contains 'selected_local_files = 0'
Assert-Contains 'uploaded_attachments = 0'
Assert-Contains 'sent_messages = 0'
Assert-Contains 'cleared_cookies = $false'
Assert-Contains 'cleared_app_data = $false'
Assert-Contains '@($state.conversation.attachments).Count -eq 0'

if (
    $source -match 'input\s+tap|input\s+text|KEYCODE_ENTER|keyevent\s+66' -or
    $source -match '(?m)^\s*exit\s+[1-9]' -or
    $source.Contains('collect_composer_tools')
) {
    throw "Picker smoke contains a forbidden legacy or side-effect path."
}

Write-Output "CHATGPT_WEB_PICKER_SMOKE_CONTRACT=passed"
