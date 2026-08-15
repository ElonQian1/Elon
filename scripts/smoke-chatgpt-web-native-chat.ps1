#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 300)][int]$TimeoutSec = 120,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [switch]$SendProbe,
    [string]$ProbeMarker = "",
    [string]$Prompt = "",
    [string]$ExpectedReply = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

function Resolve-ChatGptWebNativeAdapterVersion {
    param([ValidateRange(0, 9999)][int]$RequestedVersion)

    if ($RequestedVersion -gt 0) { return $RequestedVersion }
    $root = Split-Path -Parent $PSScriptRoot
    $path = Join-Path $root `
        "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt"
    $source = Get-Content -LiteralPath $path -Raw
    $match = [regex]::Match($source, 'ADAPTER_VERSION\s*=\s*(\d+)')
    if (-not $match.Success) { throw "Unable to resolve ChatGPT Web adapter version." }
    return [int]$match.Groups[1].Value
}

function ConvertTo-ChatGptWebNativeProbeText {
    param([AllowNull()]$Value)

    return ([string]$Value).Trim() -replace '\\([_-])', '$1'
}

function Wait-ChatGptWebNativeProbeReply {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Prompt,
        [Parameter(Mandatory = $true)][string]$ExpectedReply,
        [ValidateRange(10, 300)][int]$WaitTimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($WaitTimeoutSec)
    do {
        if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $Runtime)) {
            throw "ChatGPT Web AI acceptance was interrupted because another app took the foreground."
        }
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
        $messages = @($state.social_chat.messages)
        $user = @($messages | Where-Object { [string]$_.role -eq "user" }) |
            Select-Object -Last 1
        $assistant = @($messages | Where-Object { [string]$_.role -eq "friend" }) |
            Select-Object -Last 1
        if (
            [string]$state.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
            [string]$state.social_chat.web_chat_state -eq "ready" -and
            $state.social_chat.web_chat_streaming -ne $true -and
            [string]$user.content -eq $Prompt -and
            (ConvertTo-ChatGptWebNativeProbeText $assistant.content) -eq
                (ConvertTo-ChatGptWebNativeProbeText $ExpectedReply)
        ) {
            return $state
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the ChatGPT Web AI probe reply."
}

if (-not $SendProbe -and ($ProbeMarker -or $Prompt -or $ExpectedReply)) {
    throw "Probe arguments require -SendProbe because the default ChatGPT smoke is read-only."
}
if ($ProbeMarker -and ($Prompt -or $ExpectedReply)) {
    throw "ProbeMarker cannot be combined with Prompt or ExpectedReply."
}
if ($ProbeMarker -and $ProbeMarker -notmatch '^[A-Za-z0-9_-]{8,120}$') {
    throw "ProbeMarker must be 8-120 ASCII letters, digits, underscores, or hyphens."
}
if ($SendProbe -and -not $ProbeMarker -and -not $Prompt -and -not $ExpectedReply) {
    $ProbeMarker = "ELON-CHATGPT-WEB-AI-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
}
if ($SendProbe -and ($Prompt -or $ExpectedReply) -and (-not $Prompt -or -not $ExpectedReply)) {
    throw "Prompt and ExpectedReply must be provided together."
}
if ($Prompt.Length -gt 400 -or $ExpectedReply.Length -gt 400) {
    throw "Prompt and ExpectedReply must each stay within 400 characters."
}
$probePrompt = if ($Prompt) { $Prompt } else { "Reply exactly with: $ProbeMarker" }
$probeExpectedReply = if ($ExpectedReply) { $ExpectedReply } else { $ProbeMarker }

$ExpectedAdapterVersion = Resolve-ChatGptWebNativeAdapterVersion $ExpectedAdapterVersion
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $origin = Open-WebChatNativeChatSurface -Runtime $runtime `
        -ProviderId "chatgpt_web" -TimeoutSec $TimeoutSec
    if ([int]$origin.social_chat.web_chat_adapter_version -ne $ExpectedAdapterVersion) {
        throw "Unexpected ChatGPT Web adapter version."
    }
    if ($origin.social_chat.web_chat_composer_ready -ne $true) {
        throw "ChatGPT Web AI composer is not ready."
    }

    $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "get_web_chat_navigation" -Arguments @{ offset = 0; limit = 1 }
    $report = [ordered]@{
        schema = "elon.chatgpt_web.native_chat_smoke.v1"
        mode = if ($SendProbe) { "send_probe" } else { "read_only" }
        provider_id = "chatgpt_web"
        adapter_version = $ExpectedAdapterVersion
        authenticated = $origin.social_chat.web_chat_authenticated -eq $true
        composer_ready = $true
        conversation_total = [int]$navigation.conversation_total
        project_total = [int]$navigation.project_total
        sent_messages = 0
        assistant_completed = $false
        probe_kind = if ($Prompt) { "custom_exact" } else { "marker_exact" }
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
                -Description "blank ChatGPT Web AI probe conversation" -Predicate {
                    param($state)
                    [string]$state.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
                        $state.social_chat.web_chat_composer_ready -eq $true -and
                        [int]$state.social_chat.message_count -eq 0
                } | Out-Null
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
                -Arguments @{ text = $probePrompt } | Out-Null
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input" | Out-Null
            Wait-ChatGptWebNativeProbeReply -Runtime $runtime -Prompt $probePrompt `
                -ExpectedReply $probeExpectedReply -WaitTimeoutSec $TimeoutSec | Out-Null
            $report.sent_messages = 1
            $report.assistant_completed = $true
        } finally {
            if ($originPath) {
                try {
                    Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                        -Action "open_web_chat_conversation" `
                        -Arguments @{ conversation_path = $originPath } | Out-Null
                    $restored = Wait-ChatGptWebSmokeState -Runtime $runtime `
                        -TimeoutSec $TimeoutSec `
                        -Description "original ChatGPT Web AI conversation" -Predicate {
                            param($state)
                            [string]$state.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
                                [string]$state.social_chat.web_chat_conversation_path -eq $originPath
                        }.GetNewClosure()
                    $report.original_conversation_restored = $null -ne $restored
                } catch {
                    $report.original_conversation_restored = $false
                    throw
                }
            } else {
                Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                    -Action "start_new_web_chat_conversation" | Out-Null
            }
        }
    }

    $report | ConvertTo-Json -Depth 6
    Write-Output "CHATGPT_WEB_NATIVE_CHAT_SMOKE_STATUS=passed mode=$($report.mode)"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
