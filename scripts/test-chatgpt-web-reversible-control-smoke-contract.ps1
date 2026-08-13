#requires -Version 5.1

$ErrorActionPreference = "Stop"
$smokePath = Join-Path $PSScriptRoot "smoke-chatgpt-web-reversible-controls.ps1"
$runtimePath = Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1"
$smoke = Get-Content -LiteralPath $smokePath -Raw
$runtime = Get-Content -LiteralPath $runtimePath -Raw
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
    "ExpectedHardwareSerial",
    "ExpectedAdapterVersion",
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    "chatgpt_list_composer_options",
    "chatgpt_select_composer_option",
    "adapter_current -eq `$true",
    "Get-SelectableModels",
    "Test-SelectableModelLeaf",
    'kind -in @("menuitemradio", "option")',
    "Get-CachedComposerModels -RequireLeafChoices",
    "reasoning|thinking|effort|思考|推理|强度",
    "[Math]::Min(`$ReadyTimeoutSec, 15)",
    "Wait-SelectedModel",
    "requested model as selected",
    "expected=`$ExpectedLabel current=`$([string]`$current.conversation.current_model)",
    "Find-ModelByLabel",
    "IsNullOrWhiteSpace(`$originalModelLabel)",
    "Restore-ModelByLabel",
    "Wait-ViewMode",
    "Get-ConversationPathFromUrl",
    "Wait-BlankConversation",
    'Action "chatgpt_new_conversation"',
    'Action "chatgpt_open_conversation"',
    "original_conversation_restored = `$modelConversationRestored",
    'view_mode = "web"',
    "original_view_mode_restored = `$modelViewRestored",
    "`$options.Count -gt 0",
    "Timed out waiting for ChatGPT model options",
    "modelDiscoveryStage",
    "model entry finishes hydrating",
    "chatgpt_refresh_controls",
    "chatgpt_find_controls",
    "chatgpt_set_control_expanded",
    "set_ui_control_expanded",
    '[string]$_.region -eq "composer"',
    '[string]$_.semantic -in @("model", "attachment")',
    "original_state_restored = `$modelRestored",
    "original_state_restored = `$disclosureRestored",
    'semantic -eq "temporary_chat"',
    "Wait-TemporaryChatSelection",
    'ExpectedAction "invoke_ui_control"',
    '"reversible/temporary_chat_toggle"',
    "original_state_restored = `$temporaryChatRestored",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_STATUS=passed"
)
foreach ($token in $required) {
    if (-not $smoke.Contains($token)) {
        throw "Missing reversible control smoke contract token: $token"
    }
}
if ($smoke -notmatch '(?s)Get-CachedComposerModels -RequireLeafChoices.*?catch\s*\{\s*continue\s*\}') {
    throw "Reversible control smoke must continue past submenu choices without two leaves."
}
if ($smoke -notmatch "if \(\[string\]\`$_.label -match '\(\?i\)model\|模型'\) \{ 0 \}") {
    throw "Reversible control smoke must inspect the model submenu before adjacent capability submenus."
}
foreach ($token in @(
    "expected_hardware_serial",
    "read adb hardware identity",
    "Device hardware identity does not match the pinned target",
    "Assert-ChatGptWebSmokeUsbDevice",
    "mcp_bootstrapped = `$false",
    "HealthTimeoutSec = 15",
    "RequestTimeoutSec = 30",
    "AdbTimeoutSec = 8",
    "if (`$Runtime.mcp_bootstrapped) { `$params.NoBootstrap = `$true }",
    '$params.Remove("OpenAppOnFailure")',
    "Get-ChatGptWebSmokeMcpFailureDetail",
    "function Open-ChatGptWebSmokeSurface",
    'throw "APK MCP tool failed: $detail"',
    "<email>",
    "<redacted>"
)) {
    if (-not $runtime.Contains($token)) {
        throw "Missing trusted device runtime contract token: $token"
    }
}
if ($runtime.Contains('adb connect') -or $runtime.Contains('connect",')) {
    throw "Trusted device runtime must not create a wireless adb connection."
}
if ($runtime -notmatch '(?s)if \(\$EnsureMainActivity\) \{\s*\$params\.EnsureMainActivity = \$true\s*\$params\.OpenAppOnFailure = \$true\s*\}') {
    throw "Trusted runtime must relaunch MainActivity only for an explicit initial bootstrap."
}
. $runtimePath
$testExecutable = (Get-Process -Id $PID).Path
$safeDiagnostic = ConvertTo-ChatGptWebSmokeSafeDiagnostic `
    -Value "bridge failed for sample@example.test token=synthetic-secret"
if (
    $safeDiagnostic -notmatch '<email>' -or
    $safeDiagnostic -notmatch 'token=<redacted>' -or
    $safeDiagnostic -match 'synthetic-secret'
) {
    throw "Smoke diagnostics must redact synthetic credential-shaped values."
}
$failureResponse = [pscustomobject]@{
    result = [pscustomobject]@{
        structuredContent = [pscustomobject]@{
            action = "chatgpt_list_features"
            error_code = "bridge_not_ready"
        }
    }
}
$failureDetail = Get-ChatGptWebSmokeMcpFailureDetail `
    -Response $failureResponse -Tool "ui_control"
if ($failureDetail -ne "tool=ui_control action=chatgpt_list_features error=bridge_not_ready") {
    throw "Smoke diagnostics must retain safe action and error-code evidence."
}
$wirelessRejected = $false
try {
    New-ChatGptWebSmokeRuntime -Adb $testExecutable `
        -DeviceSerial "192.0.2.10:5555" | Out-Null
} catch {
    $wirelessRejected = $_.Exception.Message -match "expected hardware serial"
}
if (-not $wirelessRejected) { throw "Unpinned wireless smoke runtime must be rejected." }
$trusted = New-ChatGptWebSmokeRuntime -Adb $testExecutable `
    -DeviceSerial "192.0.2.10:5555" -ExpectedHardwareSerial "hardware-test"
if ($trusted.expected_hardware_serial -ne "hardware-test") {
    throw "Pinned wireless smoke runtime must retain the expected hardware identity."
}
$emulatorRejected = $false
try {
    New-ChatGptWebSmokeRuntime -Adb $testExecutable `
        -DeviceSerial "emulator-5554" | Out-Null
} catch {
    $emulatorRejected = $_.Exception.Message -match "emulator transport"
}
if (-not $emulatorRejected) { throw "Emulator smoke runtime must be rejected." }
foreach ($forbidden in @("send_input", "chatgpt_remove_attachment")) {
    if ($smoke.Contains($forbidden)) {
        throw "Reversible control smoke must not use unsafe action: $forbidden"
    }
}
Write-Output "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_CONTRACT=passed"
