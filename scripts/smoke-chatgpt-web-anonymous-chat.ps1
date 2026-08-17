#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 300)][int]$TimeoutSec = 90,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [switch]$SendProbe,
    [string]$ProbeMarker = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion

if (-not $SendProbe -and $ProbeMarker) {
    throw "ProbeMarker requires -SendProbe because the default anonymous smoke is read-only."
}
if ($SendProbe -and -not $ProbeMarker) {
    $ProbeMarker = "ELON-ANONYMOUS-WEB-AI-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
}
if ($ProbeMarker -and $ProbeMarker -notmatch '^[A-Za-z0-9_-]{8,120}$') {
    throw "ProbeMarker must be 8-120 ASCII letters, digits, underscores, or hyphens."
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
        -Description "anonymous ChatGPT Web composer" -Predicate {
            param($state)
            $state.surface -eq "chatgpt_web" -and
                $state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                $state.authenticated -eq $false -and
                $state.login_required -eq $false -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false
        }
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion

    $matrix = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_capability_matrix"
    if (
        $matrix.authenticated -ne $false -or
        $matrix.login_required -ne $false -or
        $matrix.chat_access_available -ne $true -or
        $matrix.ready_for_chat -ne $true
    ) {
        throw "Anonymous ChatGPT Web capability matrix is not ready for chat."
    }

    $report = [ordered]@{
        schema = "elon.chatgpt_web.anonymous_chat_smoke.v1"
        mode = if ($SendProbe) { "send_probe" } else { "read_only" }
        authenticated = $false
        login_required = $false
        composer_ready = $true
        sent_messages = 0
        assistant_completed = $false
        original_view_restored = $true
        cleared_cookies = $false
        cleared_app_data = $false
        private_content_emitted = $false
    }

    if ($SendProbe) {
        $checkpoint = Start-ChatGptWebSmokeIsolatedConversation -Runtime $runtime `
            -OriginState $origin -TimeoutSec $TimeoutSec
        try {
            Invoke-ChatGptWebSmokeReceiptAction -Runtime $runtime `
                -Action "set_input_text" -ExpectedAction "set_draft" `
                -Arguments @{ text = "Reply only with: $ProbeMarker" } `
                -TimeoutSec $TimeoutSec | Out-Null
            $beforeSend = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            $dispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
                -Action "send_input" -TimeoutSec $TimeoutSec
            $requestId = [string]$dispatch.command_receipt.request_id
            if (-not $requestId) { throw "Anonymous send_input returned no request id." }
            Wait-ChatGptProbeReply -RequestId $requestId -Marker $ProbeMarker `
                -AfterMs $beforeSend -TimeoutSec $TimeoutSec -PollIntervalSec 1 `
                -InvokeUiState {
                    Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
                } | Out-Null
            $report.sent_messages = 1
            $report.assistant_completed = $true
            Register-ChatGptWebVerificationCases -Runtime $runtime `
                -CaseIds @("reversible/anonymous_send_probe") `
                -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
        } finally {
            Restore-ChatGptWebSmokeOrigin -Runtime $runtime `
                -ConversationPath $checkpoint.origin_conversation_path `
                -TimeoutSec $TimeoutSec | Out-Null
        }
    }

    $report | ConvertTo-Json -Depth 8
    Write-Output "CHATGPT_WEB_ANONYMOUS_SMOKE_STATUS=passed mode=$($report.mode)"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
