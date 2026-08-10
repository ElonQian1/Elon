#requires -Version 5.1

$ErrorActionPreference = "Stop"
$smokePath = Join-Path $PSScriptRoot "smoke-chatgpt-web-reversible-controls.ps1"
$smoke = Get-Content -LiteralPath $smokePath -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $smokePath,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "PowerShell parse failed for $smokePath`: $($errors[0].Message)"
}

$required = @(
    "Assert-ChatGptWebSmokeUsbDevice",
    "chatgpt_list_composer_options",
    "chatgpt_select_composer_option",
    "chatgpt_refresh_controls",
    "chatgpt_find_controls",
    "chatgpt_set_control_expanded",
    "set_ui_control_expanded",
    "original_state_restored = `$modelRestored",
    "original_state_restored = `$disclosureRestored",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_STATUS=passed"
)
foreach ($token in $required) {
    if (-not $smoke.Contains($token)) {
        throw "Missing reversible control smoke contract token: $token"
    }
}
foreach ($forbidden in @("send_input", "chatgpt_new_conversation", "chatgpt_remove_attachment")) {
    if ($smoke.Contains($forbidden)) {
        throw "Reversible control smoke must not use unsafe action: $forbidden"
    }
}
Write-Output "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_CONTRACT=passed"
