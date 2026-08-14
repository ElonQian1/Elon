#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 300)][int]$TimeoutSec = 120,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [switch]$SendProbe,
    [string]$ProbeMarker = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

function Resolve-GoogleWebAdapterVersion {
    param([ValidateRange(0, 9999)][int]$RequestedVersion)

    if ($RequestedVersion -gt 0) { return $RequestedVersion }
    $root = Split-Path -Parent $PSScriptRoot
    $path = Join-Path $root `
        "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt"
    $source = Get-Content -LiteralPath $path -Raw
    $match = [regex]::Match($source, 'ADAPTER_VERSION\s*=\s*(\d+)')
    if (-not $match.Success) { throw "Unable to resolve Google Web adapter version." }
    return [int]$match.Groups[1].Value
}

function Wait-GoogleWebProbeReply {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Marker,
        [ValidateRange(10, 300)][int]$WaitTimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($WaitTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
        $messages = @($state.social_chat.messages)
        $user = @($messages | Where-Object { [string]$_.role -eq "user" }) |
            Select-Object -Last 1
        $assistant = @($messages | Where-Object { [string]$_.role -eq "friend" }) |
            Select-Object -Last 1
        if (
            [string]$state.social_chat.web_chat_provider_id -eq "google_web" -and
            [string]$state.social_chat.web_chat_state -eq "ready" -and
            [string]$user.content -like "*$Marker*" -and
            [string]$assistant.content -like "*$Marker*"
        ) {
            return $state
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the Google Web AI probe reply."
}

if (-not $SendProbe -and $ProbeMarker) {
    throw "ProbeMarker requires -SendProbe because the default Google smoke is read-only."
}
if ($SendProbe -and -not $ProbeMarker) {
    $ProbeMarker = "ELON-GOOGLE-WEB-AI-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
}
if ($ProbeMarker -and $ProbeMarker -notmatch '^[A-Za-z0-9_-]{8,120}$') {
    throw "ProbeMarker must be 8-120 ASCII letters, digits, underscores, or hyphens."
}

$ExpectedAdapterVersion = Resolve-GoogleWebAdapterVersion $ExpectedAdapterVersion
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $origin = Open-WebChatNativeChatSurface -Runtime $runtime `
        -ProviderId "google_web" -TimeoutSec $TimeoutSec
    if ([int]$origin.social_chat.web_chat_adapter_version -ne $ExpectedAdapterVersion) {
        throw "Unexpected Google Web adapter version."
    }
    if ($origin.social_chat.web_chat_composer_ready -ne $true) {
        throw "Google Web AI composer is not ready."
    }

    $report = [ordered]@{
        schema = "elon.google_web.native_chat_smoke.v1"
        mode = if ($SendProbe) { "send_probe" } else { "read_only" }
        provider_id = "google_web"
        adapter_version = $ExpectedAdapterVersion
        authenticated = $origin.social_chat.web_chat_authenticated -eq $true
        composer_ready = $true
        sent_messages = 0
        assistant_completed = $false
        original_conversation_restored = $true
        cleared_cookies = $false
        cleared_app_data = $false
        private_content_emitted = $false
    }

    if ($SendProbe) {
        $originPath = [string]$origin.social_chat.web_chat_conversation_path
        try {
            Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                -Action "start_new_web_chat_conversation" | Out-Null
            Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -Description "blank Google Web AI probe conversation" -Predicate {
                    param($state)
                    [string]$state.social_chat.web_chat_provider_id -eq "google_web" -and
                        $state.social_chat.web_chat_composer_ready -eq $true -and
                        [int]$state.social_chat.message_count -eq 0
                } | Out-Null
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
                -Arguments @{ text = "Reply exactly with: $ProbeMarker" } | Out-Null
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input" | Out-Null
            Wait-GoogleWebProbeReply -Runtime $runtime -Marker $ProbeMarker `
                -WaitTimeoutSec $TimeoutSec | Out-Null
            $report.sent_messages = 1
            $report.assistant_completed = $true
        } finally {
            if ($originPath) {
                Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                    -Action "open_web_chat_conversation" `
                    -Arguments @{ conversation_path = $originPath } | Out-Null
                $restored = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                    -Description "original Google Web AI conversation" -Predicate {
                        param($state)
                        [string]$state.social_chat.web_chat_provider_id -eq "google_web" -and
                            [string]$state.social_chat.web_chat_conversation_path -eq $originPath
                    }.GetNewClosure()
                $report.original_conversation_restored = $null -ne $restored
            } else {
                Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                    -Action "start_new_web_chat_conversation" | Out-Null
            }
        }
    }

    $report | ConvertTo-Json -Depth 6
    Write-Output "GOOGLE_WEB_NATIVE_CHAT_SMOKE_STATUS=passed mode=$($report.mode)"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
