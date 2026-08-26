param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [ValidateRange(15, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(10, 90)][int]$ProbeTimeoutSec = 45
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion `
    -ExpectedAdapterVersion $ExpectedAdapterVersion -RepositoryRoot $repositoryRoot
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$originalProvider = ""
$originalChatPath = ""
$probeStartedAtMs = 0L
$probeCompletedAtMs = 0L
$privateRevisionBefore = 0L
$privateRevisionAfter = 0L
$streamObserved = $false
$watchdogFired = $false
$streamSettled = $false
$originalConversationRestored = $false
$originalProviderRestored = $false
$probePrompt = "Write 120 short numbered lines, one item per line."

function Get-ConversationPath {
    param([AllowNull()][string]$Url)

    try {
        $uri = [Uri]$Url
        if ($uri.Host -in @("chatgpt.com", "www.chatgpt.com") -and $uri.AbsolutePath -match '^/c/') {
            return $uri.AbsolutePath
        }
    } catch { }
    return ""
}

function Wait-Command {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    return Wait-ChatGptCommandReceipt -RequestId $RequestId -ExpectedAction $ExpectedAction `
        -TimeoutSec $TimeoutSec -PollIntervalSec 1 -InvokeUiState {
            Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        }
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    $readiness = Get-ChatGptWebSmokeUserReadiness -Runtime $runtime
    if ($readiness.ready -ne $true) {
        throw "ChatGPT watchdog acceptance requires an unlocked device."
    }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null

    $origin = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" `
        -EnsureMainActivity -MainState
    $originalProvider = [string]$origin.social_chat.web_chat_provider_id
    Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $ReadyTimeoutSec | Out-Null
    $ready = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec
    Assert-ChatGptWebSmokeAdapterVersion -State $ready `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if (
        [int]$ready.input.text_length -ne 0 -or
        [int]$ready.input.official_draft_length -ne 0
    ) {
        throw "ChatGPT watchdog acceptance refuses to overwrite an existing native draft."
    }
    $originalChatPath = Get-ConversationPath ([string]$ready.conversation.url)

    $newDispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_new_conversation" -TimeoutSec $ReadyTimeoutSec
    $newRequestId = [string]$newDispatch.command_receipt.request_id
    if ([string]::IsNullOrWhiteSpace($newRequestId)) {
        throw "New conversation did not return a command receipt."
    }
    Wait-Command -RequestId $newRequestId -ExpectedAction "new_conversation" `
        -TimeoutSec $ReadyTimeoutSec | Out-Null
    $blank = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "isolated blank ChatGPT watchdog conversation" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false -and
                [int]$state.input.text_length -eq 0 -and
                [int]$state.input.official_draft_length -eq 0 -and
                [int]$state.conversation.message_count -eq 0
        }
    $privateRevisionBefore = [long]$blank.private_stream_observer.revision

    $probeDispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_verify_private_stream_watchdog" -TimeoutSec $ReadyTimeoutSec
    $probeRequestId = [string]$probeDispatch.command_receipt.request_id
    if ([string]::IsNullOrWhiteSpace($probeRequestId)) {
        throw "Watchdog probe did not return a command receipt."
    }
    $probeStartedAtMs = [long]$probeDispatch.command_receipt.started_at_ms

    $draftDispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_set_page_input_text" -Arguments @{ text = $probePrompt } `
        -TimeoutSec $ReadyTimeoutSec
    if ($draftDispatch.control_ok -ne $true) {
        throw "Unable to stage the isolated watchdog prompt."
    }
    $draftRequestId = [string]$draftDispatch.command_receipt.request_id
    if (-not $draftRequestId) { throw "Page draft did not return a command receipt." }
    Wait-Command -RequestId $draftRequestId -ExpectedAction "set_draft" `
        -TimeoutSec $ReadyTimeoutSec | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 15 `
        -Description "isolated watchdog native draft" -Predicate {
            param($state)
            [int]$state.input.text_length -eq $probePrompt.Length -and
                [int]$state.input.official_draft_length -eq $probePrompt.Length
        }.GetNewClosure() | Out-Null

    $sendDispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_send_page_input" -TimeoutSec $ReadyTimeoutSec
    $sendRequestId = [string]$sendDispatch.command_receipt.request_id
    if ([string]::IsNullOrWhiteSpace($sendRequestId)) {
        throw "Streaming probe did not return a command receipt."
    }
    Wait-Command -RequestId $sendRequestId -ExpectedAction "send_prompt" `
        -TimeoutSec $ReadyTimeoutSec | Out-Null

    $streaming = Wait-ChatGptStreamingState -Expected $true -TimeoutSec 30 `
        -PrivateRevisionGreaterThan $privateRevisionBefore -InvokeUiState {
            Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        }
    $streamObserved = [bool]$streaming.streaming
    $privateRevisionAfter = [long]$streaming.private_stream_observer.revision

    $probeResult = Wait-Command -RequestId $probeRequestId `
        -ExpectedAction "verify_private_stream_watchdog" -TimeoutSec $ProbeTimeoutSec
    $probeCompletedAtMs = [long]$probeResult.receipt.completed_at_ms
    $watchdogFired = $probeResult.receipt.result.ok -eq $true -and
        $probeCompletedAtMs - $probeStartedAtMs -ge 3500

    $current = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    if ($current.streaming -eq $true) {
        $stopDispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
            -Action "chatgpt_stop_generation" -TimeoutSec $ReadyTimeoutSec
        $stopRequestId = [string]$stopDispatch.command_receipt.request_id
        Wait-Command -RequestId $stopRequestId -ExpectedAction "stop_generation" `
            -TimeoutSec 30 | Out-Null
    }
    $settled = Wait-ChatGptStreamingState -Expected $false -TimeoutSec 30 `
        -InvokeUiState {
            Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        }
    $streamSettled = $settled.streaming -eq $false
} finally {
    try {
        $current = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if ($current.streaming -eq $true) {
            Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
                -Action "chatgpt_stop_generation" -TimeoutSec 20 | Out-Null
        }
        if (
            [int]$current.input.text_length -eq $probePrompt.Length -or
            [int]$current.input.official_draft_length -eq $probePrompt.Length
        ) {
            $cleanupDispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
                -Action "chatgpt_set_page_input_text" -Arguments @{ text = "" } `
                -TimeoutSec 20
            $cleanupRequestId = [string]$cleanupDispatch.command_receipt.request_id
            if ($cleanupRequestId) {
                Wait-Command -RequestId $cleanupRequestId -ExpectedAction "set_draft" `
                    -TimeoutSec 20 | Out-Null
            }
        }
    } catch { }
    if ($originalChatPath) {
        $originalConversationRestored = Restore-WebChatNativeConversation -Runtime $runtime `
            -ProviderId "chatgpt_web" -ConversationPath $originalChatPath -TimeoutSec 45
    } else {
        $originalConversationRestored = $true
    }
    if ($originalProvider -in @("chatgpt_web", "google_web")) {
        try {
            Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId $originalProvider `
                -TimeoutSec $ReadyTimeoutSec | Out-Null
            $originalProviderRestored = $true
        } catch { }
    } else {
        $originalProviderRestored = $true
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime
}

$passed = $streamObserved -and $watchdogFired -and $streamSettled -and
    $originalConversationRestored -and $originalProviderRestored
$summary = [ordered]@{
    schema = "elon.chatgpt_web.private_stream_watchdog_acceptance.v1"
    passed = $passed
    adapter_version = $ExpectedAdapterVersion
    private_stream_observed = $privateRevisionAfter -gt $privateRevisionBefore
    native_streaming_observed = $streamObserved
    watchdog_elapsed_ms = [Math]::Max(0L, $probeCompletedAtMs - $probeStartedAtMs)
    private_stream_watchdog_fired = $watchdogFired
    native_streaming_settled = $streamSettled
    original_conversation_restored = $originalConversationRestored
    original_provider_restored = $originalProviderRestored
    official_request_replayed = $false
    cookies_cleared = $false
    app_data_cleared = $false
    private_content_emitted = $false
}
$summary | ConvertTo-Json -Depth 5
if (-not $passed) { exit 1 }
