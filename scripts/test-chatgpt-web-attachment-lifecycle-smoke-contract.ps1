$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-attachment-lifecycle.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
) | Out-Null
if (@($errors).Count -gt 0) {
    throw "Attachment lifecycle smoke has PowerShell parse errors."
}

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) {
        throw "Attachment lifecycle smoke contract is missing: $Needle"
    }
}

foreach ($required in @(
    '[ValidateSet("Prepare", "OpenPicker", "VerifyAndRemove")]',
    'attachment_lifecycle_checkpoint.v1',
    'attachment_lifecycle_smoke.v1',
    'Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',
    'Wait-ChatGptWebSmokeAuthenticatedReady',
    'Start-ChatGptWebSmokeAwakeLease',
    'Stop-ChatGptWebSmokeAwakeLease',
    'conversation_binding_sha256',
    'device_binding_sha256',
    'Conversation message count changed after the checkpoint.',
    '[int]$State.input.text_length -ne 0',
    '[string]$_.semantic -eq "attachment_file"',
    '-Action "chatgpt_select_composer_option"',
    '-Action "chatgpt_remove_attachment"',
    '-ExpectedAction "remove_attachment"',
    '[string]$items[0].state -eq "ready"',
    '@($state.conversation.attachments).Count -eq 0',
    'Move-Item -LiteralPath $temporary -Destination $CheckpointPath -Force',
    'selected_local_files = 1',
    'final_attachment_count = 0',
    'message_count_unchanged = $true',
    'sent_messages = 0',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'private_content_emitted = $false',
    'CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=waiting_for_user_selection',
    'CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=passed'
)) {
    Assert-Contains $required
}

foreach ($forbidden in @(
    'send_input',
    'set_input_text',
    'pm clear',
    'removeAllCookies',
    'input tap',
    'KEYCODE_ENTER',
    'keyevent 66',
    '.conversation.title',
    '.attachments)[0].name'
)) {
    if ($source.Contains($forbidden)) {
        throw "Attachment lifecycle smoke contains forbidden data or action: $forbidden"
    }
}

if ($source -match '\.input\.text(?!_length)') {
    throw "Attachment lifecycle smoke must not read or emit draft text."
}

if ($source -match '(?m)^\s*exit\s+[1-9]') {
    throw "Attachment lifecycle smoke must fail through exceptions, not nested exit."
}

$lineCount = @($source -split "`n").Count
if ($lineCount -gt 420) {
    throw "Attachment lifecycle smoke exceeded its modular size budget: $lineCount"
}

Write-Output "CHATGPT_WEB_ATTACHMENT_LIFECYCLE_SMOKE_CONTRACT=passed"
