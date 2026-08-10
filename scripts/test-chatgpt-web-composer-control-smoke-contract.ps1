#requires -Version 5.1

$ErrorActionPreference = "Stop"
$script = Get-Content (Join-Path $PSScriptRoot "smoke-chatgpt-web-composer-controls.ps1") -Raw

$required = @(
    "Assert-ChatGptWebSmokeUsbDevice",
    "chatgpt_start_dictation",
    "chatgpt_cancel_dictation",
    "start_dictation",
    "cancel_dictation",
    "web_search",
    "chatgpt_select_composer_option",
    "set_input_text",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "original_state_restored = `$true",
    "CHATGPT_WEB_COMPOSER_CONTROL_SMOKE_STATUS=passed"
)
foreach ($token in $required) {
    if (-not $script.Contains($token)) { throw "Missing composer smoke contract token: $token" }
}

if ($script -match "send_input|ProbeMarker|Reply only with") {
    throw "Composer control smoke must not send ChatGPT messages."
}
Write-Output "CHATGPT_WEB_COMPOSER_CONTROL_SMOKE_CONTRACT=passed"
