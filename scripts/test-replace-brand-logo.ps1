param()

$ErrorActionPreference = "Stop"
$output = & powershell -NoProfile -ExecutionPolicy Bypass -File `
    (Join-Path $PSScriptRoot "replace-brand-logo.ps1") -Check 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "replace-brand-logo -Check failed: $($output -join "`n")"
}
$text = $output -join "`n"
if (-not $text.Contains("BRAND_LOGO_STATUS=verified generated=24")) {
    throw "Brand logo verification did not cover all 24 generated artifacts: $text"
}
Write-Host "PASS replace-brand-logo"
