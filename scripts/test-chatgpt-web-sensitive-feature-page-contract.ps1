#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-sensitive-feature-page.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) { throw "Sensitive feature smoke has parse errors." }

foreach ($required in @(
    '[ValidateSet("health", "finances")]',
    "UserConfirmedSensitiveFeature",
    "requires_user_confirmation",
    "user_confirmed = `$true",
    'health = "supervised/feature_page/health"',
    'finances = "supervised/feature_page/finances"',
    "Test-ChatGptWebFeatureMatrix",
    "Restore-Origin",
    "private_content_emitted = `$false",
    "mutations_invoked = 0",
    "sent_messages = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false"
)) {
    if (-not $source.Contains($required)) {
        throw "Sensitive feature smoke is missing: $required"
    }
}
foreach ($forbidden in @("pm clear", "removeAllCookies", "send_input")) {
    if ($source.Contains($forbidden)) {
        throw "Sensitive feature smoke contains forbidden operation: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_SENSITIVE_FEATURE_PAGE_CONTRACT=passed"
