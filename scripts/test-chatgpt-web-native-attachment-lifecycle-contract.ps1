$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-native-attachment-lifecycle.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
) | Out-Null
if (@($errors).Count -gt 0) {
    throw "Native attachment lifecycle smoke has PowerShell parse errors."
}

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) {
        throw "Native attachment lifecycle contract is missing: $Needle"
    }
}

foreach ($required in @(
    '"PrepareAndRemove", "StageForSend", "SendAndVerifyReply"',
    '$nativePredicate = $Predicate',
    '}.GetNewClosure()',
    '[switch]$UserConfirmedAttachmentSend',
    'fixed_ascii_text_v1',
    'Open-ChatGptWebNativeChatSurface',
    'start_new_web_chat_conversation',
    'Current non-empty ChatGPT Web AI conversation cannot be restored safely.',
    'web_chat_adapter_version -ne $ExpectedAdapterVersion',
    'Native attachment acceptance requires an authenticated ChatGPT Web session.',
    'stage_chatgpt_web_acceptance_attachment',
    'remove_chatgpt_web_acceptance_attachment',
    'Assert-FixtureState -State $staged -Expected $true',
    'Assert-FixtureState -State $removed -Expected $false',
    'Invoke-NativeAction -Action "set_input_text"',
    'Invoke-NativeAction -Action "send_input"',
    '$checkpoint.phase = "send_dispatching"',
    '$checkpoint.phase = "reply_requested"',
    'Attachment send outcome is ambiguous after interruption',
    'Native attachment upload failed; fixed fixture removed.',
    'web_chat_attachment_phase -eq "completed"',
    'web_chat_pending_attachment_count -eq 0',
    'Restore-Origin -Checkpoint $checkpoint',
    'Register-ChatGptWebVerificationCases',
    '-CaseIds @("supervised/attachment_lifecycle")',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'CHATGPT_WEB_NATIVE_ATTACHMENT_STATUS=passed'
)) {
    Assert-Contains $required
}

$confirmation = $source.IndexOf('if (-not $UserConfirmedAttachmentSend)')
$send = $source.IndexOf('Invoke-NativeAction -Action "send_input"')
if ($confirmation -lt 0 -or $send -le $confirmation) {
    throw "Native attachment upload must remain behind explicit user supervision."
}

foreach ($forbidden in @(
    'ACTION_OPEN_DOCUMENT',
    'ACTION_GET_CONTENT',
    'uiautomator',
    'input tap',
    'pm clear',
    'removeAllCookies',
    'Downloads',
    'DCIM',
    '.conversation.title'
)) {
    if ($source.Contains($forbidden)) {
        throw "Native attachment smoke contains forbidden file or device access: $forbidden"
    }
}

if ($source -match '(?m)^\s*exit\s+[1-9]') {
    throw "Native attachment smoke must fail through exceptions."
}
if (@([regex]::Matches($source, 'Invoke-NativeAction -Action "send_input"')).Count -ne 1) {
    throw "Native attachment smoke must dispatch exactly one supervised message."
}

$lineCount = @($source -split "`n").Count
if ($lineCount -gt 430) {
    throw "Native attachment lifecycle smoke exceeded its modular size budget: $lineCount"
}

Write-Output "CHATGPT_WEB_NATIVE_ATTACHMENT_CONTRACT=passed"
