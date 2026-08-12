$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-settings.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$parseErrors
)
if (@($parseErrors).Count -gt 0) {
    throw "ChatGPT settings smoke has PowerShell parse errors: $($parseErrors[0].Message)"
}

foreach ($required in @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    'ExpectedAdapterVersion = 65',
    'view_mode -notin @("official", "web")',
    '$state.bridge_state -eq "ready"',
    '$state.adapter_current -eq $true',
    'Wait-FirstControl -Semantic "profile" -Region "overlay"',
    'Wait-FirstControl -Semantic "logout" -Region "overlay"',
    'Wait-FirstControl -Semantic "settings" -Region "overlay"',
    'Invoke-ReceiptAction -Action "chatgpt_refresh_controls"',
    'Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime -Action $Action',
    '-ExpectedAction "snapshot_ui_manifest"',
    '[string]$_.role -eq "tab"',
    '[string]$_.role -eq "switch"',
    '[string]$_.semantic -ne "selection"',
    '[string]$_.semantic -ne "toggle"',
    'selected = $true',
    'idempotent_tab_selection = $true',
    'settings_already_open = $settingsAlreadyOpen',
    'changed_settings = $false',
    'function Invoke-ReadOnlyControlQuery',
    '[ValidateSet("chatgpt_find_controls", "chatgpt_get_capability_matrix")]',
    'if ($_.Exception.Message -notmatch "bridge_not_ready") { throw }',
    "function Restore-Origin",
    '$refreshAttempted = $false',
    'TotalSeconds -ge 10',
    'sent_messages = 0',
    'uploaded_attachments = 0',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    "CHATGPT_SETTINGS_SMOKE_STATUS=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT settings smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    "send_input",
    "chatgpt_set_control_text",
    "chatgpt_select_control_choice",
    "selected = `$false",
    "pm clear",
    "removeAllCookies",
    "chatgpt_remove_attachment",
    "chatgpt_start_dictation",
    "chatgpt_submit_dictation",
    "chatgpt_new_conversation"
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT settings smoke contains a forbidden side effect: $forbidden"
    }
}
if ($source.Contains("InitialWaitSec 1")) {
    throw "Settings navigation must not refresh during transient adapter reconnects."
}

Write-Output "CHATGPT_SETTINGS_SMOKE_CONTRACT=passed"
