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
    "Assert-ChatGptWebSmokeTrustedDevice",
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
foreach ($token in @(
    "expected_hardware_serial",
    "read adb hardware identity",
    "Device hardware identity does not match the pinned target",
    "Assert-ChatGptWebSmokeUsbDevice"
)) {
    if (-not $runtime.Contains($token)) {
        throw "Missing trusted device runtime contract token: $token"
    }
}
if ($runtime.Contains('adb connect') -or $runtime.Contains('connect",')) {
    throw "Trusted device runtime must not create a wireless adb connection."
}
. $runtimePath
$wirelessRejected = $false
try {
    New-ChatGptWebSmokeRuntime -Adb "$PSHOME\powershell.exe" `
        -DeviceSerial "192.0.2.10:5555" | Out-Null
} catch {
    $wirelessRejected = $_.Exception.Message -match "expected hardware serial"
}
if (-not $wirelessRejected) { throw "Unpinned wireless smoke runtime must be rejected." }
$trusted = New-ChatGptWebSmokeRuntime -Adb "$PSHOME\powershell.exe" `
    -DeviceSerial "192.0.2.10:5555" -ExpectedHardwareSerial "hardware-test"
if ($trusted.expected_hardware_serial -ne "hardware-test") {
    throw "Pinned wireless smoke runtime must retain the expected hardware identity."
}
$emulatorRejected = $false
try {
    New-ChatGptWebSmokeRuntime -Adb "$PSHOME\powershell.exe" `
        -DeviceSerial "emulator-5554" | Out-Null
} catch {
    $emulatorRejected = $_.Exception.Message -match "emulator transport"
}
if (-not $emulatorRejected) { throw "Emulator smoke runtime must be rejected." }
foreach ($forbidden in @("send_input", "chatgpt_new_conversation", "chatgpt_remove_attachment")) {
    if ($smoke.Contains($forbidden)) {
        throw "Reversible control smoke must not use unsafe action: $forbidden"
    }
}
Write-Output "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_CONTRACT=passed"
