$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-private-stream-watchdog.ps1"
$source = Get-Content -LiteralPath $path -Raw

function Assert-Contains([string]$Needle) {
    if (-not $source.Contains($Needle)) { throw "Missing watchdog smoke contract: $Needle" }
}

Assert-Contains 'chatgpt_verify_private_stream_watchdog'
Assert-Contains 'chatgpt_set_page_input_text'
Assert-Contains 'chatgpt_send_page_input'
Assert-Contains '[string]$ExpectedHardwareSerial = ""'
Assert-Contains 'Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime'
Assert-Contains 'PrivateRevisionGreaterThan $privateRevisionBefore'
Assert-Contains 'ExpectedAction "verify_private_stream_watchdog"'
Assert-Contains '$probeCompletedAtMs - $probeStartedAtMs -ge 3500'
Assert-Contains 'refuses to overwrite an existing native draft'
Assert-Contains '[int]$state.input.official_draft_length -eq $probePrompt.Length'
Assert-Contains '[int]$current.input.official_draft_length -eq $probePrompt.Length'
Assert-Contains 'Restore-WebChatNativeConversation'
Assert-Contains 'official_request_replayed = $false'
Assert-Contains 'cookies_cleared = $false'
Assert-Contains 'app_data_cleared = $false'
Assert-Contains 'private_content_emitted = $false'
if ($source -match 'pm\s+clear|clear\s+com\.elon\.app') {
    throw "Watchdog smoke must not clear application data."
}

Write-Output "ChatGPT private stream watchdog smoke contract passed."
