#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-reversible-batch.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "PowerShell parse failed for $path`: $($errors[0].Message)"
}

$required = @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    'id = "reversible_controls"',
    'id = "composer_controls"',
    'id = "message_structure"',
    "SkipDictation = `$true",
    "user_supervised_remaining",
    "official_authentication",
    "attachment_lifecycle",
    "dictation_audio_capture",
    "realtime_voice",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_REVERSIBLE_ACCEPTANCE_BATCH_STATUS=passed"
)
foreach ($needle in $required) {
    if (-not $source.Contains($needle)) {
        throw "ChatGPT reversible batch contract is missing: $needle"
    }
}
foreach ($forbidden in @("-SendProbe", "pm clear", "removeAllCookies")) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT reversible batch contains a forbidden action: $forbidden"
    }
}

foreach ($childScript in @(
    "smoke-chatgpt-web-reversible-controls.ps1",
    "smoke-chatgpt-web-composer-controls.ps1",
    "smoke-chatgpt-web-message-structure.ps1"
)) {
    $childSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot $childScript) -Raw
    if ($childSource -match '(?m)^\s*exit\s+[1-9]') {
        throw "Reversible acceptance child must throw instead of terminating the batch: $childScript"
    }
}

Write-Output "CHATGPT_WEB_REVERSIBLE_ACCEPTANCE_BATCH_CONTRACT=passed"
