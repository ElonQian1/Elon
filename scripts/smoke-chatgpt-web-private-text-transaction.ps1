#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 300)][int]$TimeoutSec = 120,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

function ConvertTo-PrivateTextProbeText {
    param([AllowNull()]$Value)

    return ([string]$Value).Trim() -replace '\\([_-])', '$1'
}

function Get-PrivateTextRouteKind {
    param([AllowEmptyString()][string]$Path)

    $clean = (([string]$Path -split '[?#]', 2)[0]).TrimEnd('/')
    if (-not $clean) { return "root" }
    if ($clean -match '^/c/[^/]+$') { return "conversation" }
    if ($clean -match '^/g/[^/]+$') { return "gizmo_root" }
    if ($clean -match '^/g/[^/]+/c/[^/]+$') { return "gizmo_conversation" }
    if ($clean -match '^/g/[^/]+/') {
        return "gizmo_other_$(@($clean.Trim('/') -split '/').Count)"
    }
    return "other_$(@($clean.Trim('/') -split '/').Count)"
}

function Wait-PrivateTextProbeReply {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Prompt,
        [Parameter(Mandatory = $true)][string]$ExpectedReply,
        [Parameter(Mandatory = $true)][string]$CommandDetail,
        [ValidateRange(10, 300)][int]$WaitTimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($WaitTimeoutSec)
    $diagnostic = [ordered]@{
        send_receipt_succeeded = $true
        command_detail = $CommandDetail
        provider_state = ""
        streaming = $false
        message_count = 0
        user_marker_matched = $false
        assistant_marker_matched = $false
    }
    do {
        if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $Runtime)) {
            throw "Private text transaction acceptance lost the production chat foreground."
        }
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" -MainState
        $diagnostic.provider_state = [string]$main.social_chat.web_chat_state
        $diagnostic.streaming = [bool]$main.social_chat.web_chat_streaming
        $messages = @($main.social_chat.messages)
        $diagnostic.message_count = $messages.Count
        $user = @($messages | Where-Object { [string]$_.role -eq "user" }) |
            Select-Object -Last 1
        $assistant = @($messages | Where-Object { [string]$_.role -eq "friend" }) |
            Select-Object -Last 1
        $diagnostic.user_marker_matched = [string]$user.content -eq $Prompt
        $diagnostic.assistant_marker_matched =
            (ConvertTo-PrivateTextProbeText $assistant.content) -eq $ExpectedReply
        if (
            [string]$main.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
            [string]$main.social_chat.web_chat_state -eq "ready" -and
            $main.social_chat.web_chat_streaming -ne $true -and
            [string]$user.content -eq $Prompt -and
            (ConvertTo-PrivateTextProbeText $assistant.content) -eq $ExpectedReply
        ) {
            return [pscustomobject]@{
                detail = $CommandDetail
            }
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the private text transaction probe: $($diagnostic | ConvertTo-Json -Compress)"
}

function Wait-PrivateTextProductionSendReceipt {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][long]$AfterObservedAtMs,
        [ValidateRange(10, 300)][int]$WaitTimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($WaitTimeoutSec)
    do {
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" -MainState
        $receipt = $main.social_chat.web_chat_last_send_command
        if (
            $null -ne $receipt -and
            [string]$receipt.action -eq "send_prompt" -and
            [long]$receipt.observed_at_ms -gt $AfterObservedAtMs
        ) {
            if ($receipt.ok -ne $true) {
                $detail = ConvertTo-ChatGptWebSmokeSafeDiagnostic `
                    -Value $receipt.detail -MaxLength 160
                throw "ChatGPT production send command failed: $detail"
            }
            return $receipt
        }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the ChatGPT production send receipt."
}

function Invoke-PrivateTextProbe {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Marker,
        [ValidateRange(10, 300)][int]$WaitTimeoutSec
    )

    $prompt = "Reply exactly with: $Marker"
    $before = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state" -MainState
    $previousReceipt = $before.social_chat.web_chat_last_send_command
    $previousObservedAtMs = if ($null -ne $previousReceipt) {
        [long]$previousReceipt.observed_at_ms
    } else {
        0L
    }
    Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
        -Action "set_input_text" -Arguments @{ text = $prompt } | Out-Null
    $started = [DateTimeOffset]::UtcNow
    Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action "send_input" | Out-Null
    $sendReceipt = Wait-PrivateTextProductionSendReceipt -Runtime $Runtime `
        -AfterObservedAtMs $previousObservedAtMs -WaitTimeoutSec $WaitTimeoutSec
    $completed = Wait-PrivateTextProbeReply -Runtime $Runtime -Prompt $prompt `
        -ExpectedReply $Marker -CommandDetail ([string]$sendReceipt.detail) `
        -WaitTimeoutSec $WaitTimeoutSec
    $detail = [string]$completed.detail
    $transport = if ($detail -eq "private_text_v1:accepted") {
        "private_text_v1"
    } elseif ($detail -match '\[private_fallback:([^\]]+)\]') {
        "official_fallback:$($Matches[1])"
    } else {
        "official"
    }
    return [ordered]@{
        transport = $transport
        elapsed_ms = [long]([DateTimeOffset]::UtcNow - $started).TotalMilliseconds
        receipt_succeeded = $true
        stream_completed = $true
    }
}

$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion `
    -ExpectedAdapterVersion $ExpectedAdapterVersion
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
$originPath = ""
$originInputText = ""
$restored = $true
try {
    $origin = Open-WebChatNativeChatSurface -Runtime $runtime `
        -ProviderId "chatgpt_web" -TimeoutSec $TimeoutSec
    if ([int]$origin.social_chat.web_chat_adapter_version -ne $ExpectedAdapterVersion) {
        throw "Unexpected ChatGPT Web adapter version."
    }
    $originPath = [string]$origin.social_chat.web_chat_conversation_path
    $originInputText = [string]$origin.input.text
    Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "start_new_web_chat_conversation" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec -MainState `
        -Description "blank private text transaction conversation" -Predicate {
            param($state)
            [string]$state.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
                $state.social_chat.web_chat_composer_ready -eq $true -and
                [int]$state.social_chat.message_count -eq 0
        } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
        -Description "blank official private text transaction composer" -Predicate {
            param($state)
            [string]$state.surface -eq "chatgpt_web" -and
                [string]$state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                [int]$state.conversation.message_count -eq 0 -and
                [int]$state.input.official_draft_length -eq 0
        } | Out-Null

    $stamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $seed = Invoke-PrivateTextProbe -Runtime $runtime `
        -Marker "ELON-PRIVATE-TEXT-SEED-$stamp" -WaitTimeoutSec $TimeoutSec
    $seedState = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
    $seedRouteKind = Get-PrivateTextRouteKind `
        -Path ([string]$seedState.social_chat.web_chat_conversation_path)
    $direct = Invoke-PrivateTextProbe -Runtime $runtime `
        -Marker "ELON-PRIVATE-TEXT-DIRECT-$stamp" -WaitTimeoutSec $TimeoutSec
    $report = [ordered]@{
        schema = "elon.chatgpt_web.private_text_transaction_smoke.v1"
        provider_id = "chatgpt_web"
        production_surface = $true
        adapter_version = $ExpectedAdapterVersion
        seed_route_kind = $seedRouteKind
        seed = $seed
        direct = $direct
        original_conversation_restored = $true
        cleared_cookies = $false
        cleared_app_data = $false
        private_content_emitted = $false
    }
} finally {
    if ($originPath) {
        $restored = Restore-WebChatNativeConversation -Runtime $runtime `
            -ProviderId "chatgpt_web" -ConversationPath $originPath `
            -TimeoutSec ([Math]::Min($TimeoutSec, 120))
    } else {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "start_new_web_chat_conversation" | Out-Null
    }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "set_input_text" -Arguments @{ text = $originInputText } | Out-Null
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
if (-not $restored) { throw "Unable to restore the original ChatGPT Web AI conversation." }
$report.original_conversation_restored = $restored
$report | ConvertTo-Json -Depth 6
Write-Output "CHATGPT_WEB_PRIVATE_TEXT_TRANSACTION_SMOKE_STATUS=passed"
