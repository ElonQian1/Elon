#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "request-chatgpt-web-user-attention.ps1"
$source = Get-Content -LiteralPath $path -Raw
$runtime = Get-Content -LiteralPath (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1") -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) { throw "User-attention helper has parse errors." }

foreach ($required in @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Request-ChatGptWebSmokeUserAttention",
    "ExpectedHardwareSerial",
    "vibrator_manager",
    "continuation_requires_explicit_reply",
    "automatic_sensitive_action = `$false"
)) {
    if (-not ($source.Contains($required) -or $runtime.Contains($required))) {
        throw "User-attention helper is missing: $required"
    }
}
foreach ($forbidden in @("KEYCODE_WAKEUP", "pm clear", "removeAllCookies", "input text")) {
    if ($source.Contains($forbidden)) {
        throw "User-attention helper contains forbidden operation: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_USER_ATTENTION_CONTRACT=passed"
