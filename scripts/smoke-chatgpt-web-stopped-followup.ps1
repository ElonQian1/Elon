#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe',
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 306,
    [ValidateRange(20, 180)][int]$TimeoutSec = 120
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-stopped-turn-evidence.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$report = [ordered]@{ schema = 'elon.chatgpt.stopped_followup.v1'; passed = $false
    stage = 'prepare'; restored = $false; awake_restored = $false; private_content_emitted = $false }
$originalPath = ''; $started = $false
$firstPrompt = 'Begin your answer with ANSWER-START. Then write 200 numbered lines of basic arithmetic facts, one per line.'
$secondPrompt = 'Reply only with TURN-TWO-OK.'

function Read-Native {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    $state = Get-ChatGptWebNativeChatState -Runtime $runtime
    if ($state.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $state.social_chat.interaction_mode -ne 'chat') { throw 'native_surface_changed' }
    return $state
}

function Wait-Native([string]$Stage, [scriptblock]$Predicate) {
    $report.stage = $Stage
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = Read-Native
        if (& $Predicate $state) { return $state }
        Start-Sleep -Milliseconds 700
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'stage_timeout'
}

function Act([string]$Action, [hashtable]$Arguments = @{}) {
    $result = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action -Arguments $Arguments
    if ($result.control_ok -ne $true) { throw 'action_not_accepted' }
    return $result
}

function Send-Native([string]$Text) {
    $before = Read-Native
    $timestamp = [long]$before.social_chat.web_chat_last_send_command.observed_at_ms
    Act 'set_input_text' @{ text = $Text } | Out-Null
    Act 'send_input' | Out-Null
    $state = Wait-Native 'send_receipt' {
        param($s)
        [long]$s.social_chat.web_chat_last_send_command.observed_at_ms -gt $timestamp
    }
    $receipt = $state.social_chat.web_chat_last_send_command
    if ($receipt.ok -ne $true -or $receipt.detail -ne 'official_runtime_v1:accepted') {
        throw 'runtime_send_unconfirmed'
    }
}

function Wait-Blank {
    Wait-Native 'blank_native' { param($s)
        $s.social_chat.web_chat_composer_ready -eq $true -and
        @($s.social_chat.messages).Count -eq 0 -and -not $s.input.text
    } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
        -Description 'blank official snapshot' -Predicate { param($s)
            $s.bridge_state -eq 'ready' -and $s.adapter_current -eq $true -and
            [int]$s.conversation.message_count -eq 0 -and [int]$s.input.official_draft_length -eq 0
        } | Out-Null
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $origin = Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId 'chatgpt_web' -TimeoutSec $TimeoutSec
    $web = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool 'ui_state'
    if ([int]$origin.social_chat.web_chat_adapter_version -ne $ExpectedAdapterVersion) { throw 'adapter_mismatch' }
    if ($origin.social_chat.web_chat_streaming -or $origin.input.text -or
        [int]$web.input.official_draft_length -ne 0) { throw 'existing_work_in_progress' }
    $originalPath = [string]$origin.social_chat.web_chat_conversation_path
    if ($originalPath -in @('', '/') -and @($origin.social_chat.messages).Count -gt 0) {
        throw 'guest_history_cannot_be_restored'
    }
    $started = $true
    Act 'start_new_web_chat_conversation' | Out-Null
    Wait-Blank
    Send-Native $firstPrompt
    # A recognizable requested answer, not merely any assistant-role frame.
    $partial = Wait-Native 'public_answer_streaming' { param($s)
        $answer = @($s.social_chat.messages | Where-Object role -eq 'friend') | Select-Object -Last 1
        $s.social_chat.web_chat_streaming -eq $true -and ([string]$answer.content).Length -ge 80 -and
            ([string]$answer.content).StartsWith('ANSWER-START', [StringComparison]::Ordinal)
    }
    $partialText = [string](@($partial.social_chat.messages | Where-Object role -eq 'friend')[-1].content)
    $report.partial_characters = $partialText.Length
    $report.stage = 'stop_receipt'
    $stop = Act 'chatgpt_stop_generation'
    $result = Wait-ChatGptCommandReceipt -RequestId ([string]$stop.command_receipt.request_id) `
        -ExpectedAction 'stop_generation' -TimeoutSec $TimeoutSec -PollIntervalSec 1 -InvokeUiState {
            Read-Native | Out-Null
            Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool 'ui_state'
        }
    $report.stop_observed = $result.receipt.result.detail -eq 'official_runtime_v1:stop_observed'
    if (-not $report.stop_observed) { throw 'private_stop_unconfirmed' }
    Wait-Native 'stopped' { param($s)
        $s.social_chat.web_chat_streaming -eq $false -and $s.social_chat.web_chat_composer_ready -eq $true
    } | Out-Null
    Send-Native $secondPrompt
    $final = Wait-Native 'followup_complete' { param($s)
        $answer = @($s.social_chat.messages | Where-Object role -eq 'friend') | Select-Object -Last 1
        $s.social_chat.web_chat_streaming -eq $false -and
            ([string]$answer.content).Trim().TrimEnd('.') -ceq 'TURN-TWO-OK'
    }
    $report.continuity = Get-ChatGptStoppedTurnEvidence -State $final -FirstPrompt $firstPrompt `
        -SecondPrompt $secondPrompt -PartialReply $partialText -ExpectedReply 'TURN-TWO-OK'
    $web = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool 'ui_state'
    $report.empty_drafts = -not $final.input.text -and [int]$web.input.official_draft_length -eq 0
    $report.passed = $report.continuity.passed -and $report.empty_drafts
    $report.stage = 'complete'
} catch {
    # Preserve stage and failure separately from cleanup, without dumping state/text.
    $report.failure = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
} finally {
    $completedStage = $report.stage
    try {
        if ($started) {
            $current = Read-Native
            $web = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool 'ui_state'
            if ($current.social_chat.web_chat_streaming -or $current.input.text -or
                [int]$web.input.official_draft_length -ne 0) { throw 'active_work_preserved_no_cleanup_navigation' }
            if ($originalPath -notin @('', '/')) {
                $report.restored = Restore-WebChatNativeConversation -Runtime $runtime `
                    -ProviderId 'chatgpt_web' -ConversationPath $originalPath -TimeoutSec $TimeoutSec
            } else {
                Act 'start_new_web_chat_conversation' | Out-Null
                Wait-Blank
                $report.restored = $true
            }
        } else { $report.restored = $true }
    } catch { $report.cleanup_failure = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message }
    $report.stage = $completedStage
    $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime
}
$report.passed = $report.passed -and $report.restored -and $report.awake_restored
$report | ConvertTo-Json -Depth 5
if (-not $report.passed) { exit 1 }
