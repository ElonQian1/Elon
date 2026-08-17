#requires -Version 5.1

$ErrorActionPreference = "Stop"
$scriptPath = Join-Path $PSScriptRoot "smoke-chatgpt-web-copy.ps1"
$source = Get-Content -LiteralPath $scriptPath -Raw

function Assert-Contains([string]$Needle) {
    if (-not $source.Contains($Needle)) {
        throw "Copy smoke contract is missing: $Needle"
    }
}

Assert-Contains 'chatgpt_copy_last_response'
Assert-Contains 'elon.chatgpt_web.clipboard_receipt.v1'
Assert-Contains 'content_exported -ne $false'
Assert-Contains 'clipboard_content_read_back = $false'
Assert-Contains 'private_content_emitted = $false'
Assert-Contains 'original_conversation_restored = $true'
Assert-Contains 'production_surface_preserved = Test-ChatGptWebSmokeActivityForeground'
Assert-Contains 'cleared_cookies = $false'
Assert-Contains 'cleared_app_data = $false'

foreach ($forbidden in @('dumpsys clipboard', 'cmd clipboard', 'service call clipboard')) {
    if ($source.Contains($forbidden)) {
        throw "Copy smoke contract must not read clipboard content: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_COPY_SMOKE_CONTRACT=passed"
