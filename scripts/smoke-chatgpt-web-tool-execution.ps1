#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateSet("web_search", "deep_research", "image_generation", "canvas", "study_mode", "agent_mode")]
    [string]$ToolId = "web_search",
    [switch]$UserConfirmedAgentMode,
    [ValidateRange(30, 300)][int]$ReadyTimeoutSec = 120,
    [ValidateRange(30, 1200)][int]$ReplyTimeoutSec = 600,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

$toolSpecs = @{
    web_search = [pscustomobject]@{
        semantic = "web_search"
        case_id = "reversible/tool_execution_with_citations"
        prompt = "Use web search for the official OpenAI homepage, then include {marker} in the answer."
        expected_parts = @("citation")
    }
    deep_research = [pscustomobject]@{
        semantic = "deep_research"
        case_id = "reversible/composer_tool_execution/deep_research"
        prompt = "Research the official OpenAI homepage and briefly report the result with citations. Include {marker}."
        expected_parts = @("citation")
    }
    image_generation = [pscustomobject]@{
        semantic = "image_generation"
        case_id = "reversible/composer_tool_execution/image_generation"
        prompt = "Create a simple black circle on a white background. In the response text include {marker}."
        expected_parts = @("image")
    }
    canvas = [pscustomobject]@{
        semantic = "canvas"
        case_id = "reversible/composer_tool_execution/canvas"
        prompt = "Create a canvas containing one heading and the text {marker}."
        expected_parts = @("artifact", "interactive", "code")
    }
    study_mode = [pscustomobject]@{
        semantic = "study"
        case_id = "reversible/composer_tool_execution/study_mode"
        prompt = "Use study mode to ask one short question about 2 + 2, and include {marker}."
        expected_parts = @()
    }
    agent_mode = [pscustomobject]@{
        semantic = "agent"
        case_id = "supervised/composer_tool_execution/agent_mode"
        prompt = "Without opening external sites or taking actions, reply with {marker}."
        expected_parts = @()
    }
}
$toolSpec = $toolSpecs[$ToolId]
if ($ToolId -eq "agent_mode" -and -not $UserConfirmedAgentMode) {
    throw "CHATGPT_WEB_TOOL_EXECUTION_STATUS=user_action_required required_action=confirm_agent_mode"
}

function Invoke-ReceiptAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{},
        [ValidateRange(5, 300)][int]$TimeoutSec = $ReadyTimeoutSec
    )

    $dispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action $Action -Arguments $Arguments -TimeoutSec $TimeoutSec
    $requestId = [string]$dispatch.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action." }
    return Wait-ChatGptCommandReceipt `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $requestId -ExpectedAction $ExpectedAction `
        -TimeoutSec $TimeoutSec -PollIntervalSec $PollIntervalSec
}

function Wait-ProductionSendReceipt {
    param(
        [Parameter(Mandatory = $true)][long]$AfterObservedAtMs
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReplyTimeoutSec)
    do {
        if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) {
            throw "ChatGPT tool execution lost the production chat foreground."
        }
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
        if (
            [string]$main.active_surface -ne "social_ai" -or
            [string]$main.social_chat.interaction_mode -ne "chat" -or
            [string]$main.social_chat.web_chat_provider_id -ne "chatgpt_web"
        ) {
            throw "ChatGPT tool execution left the production chat surface before send confirmation."
        }
        $receipt = $main.social_chat.web_chat_last_send_command
        if (
            $null -ne $receipt -and
            [string]$receipt.action -eq "send_prompt" -and
            [long]$receipt.observed_at_ms -gt $AfterObservedAtMs
        ) {
            if ($receipt.ok -ne $true) {
                $detail = ConvertTo-ChatGptWebSmokeSafeDiagnostic `
                    -Value $receipt.detail -MaxLength 160
                throw "ChatGPT production tool send failed: $detail"
            }
            return $receipt
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the ChatGPT production tool send receipt: $ToolId"
}

function Wait-ToolReply {
    param(
        [Parameter(Mandatory = $true)][int]$InitialMainMessageCount,
        [Parameter(Mandatory = $true)][int]$InitialAdapterMessageCount
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReplyTimeoutSec)
    $lastMainState = ""
    $lastMainMessageCount = 0
    $lastAdapterMessageCount = 0
    do {
        if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) {
            throw "ChatGPT tool execution lost the production chat foreground."
        }
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
        if (
            [string]$main.active_surface -ne "social_ai" -or
            [string]$main.social_chat.interaction_mode -ne "chat" -or
            [string]$main.social_chat.web_chat_provider_id -ne "chatgpt_web"
        ) {
            throw "ChatGPT tool execution left the production chat surface while awaiting completion."
        }
        $lastMainState = [string]$main.social_chat.web_chat_state
        $mainMessages = @($main.social_chat.messages)
        $lastMainMessageCount = $mainMessages.Count
        $lastMainMessage = $mainMessages | Select-Object -Last 1
        $mainCompleted =
            [string]$main.active_surface -eq "social_ai" -and
            [string]$main.social_chat.interaction_mode -eq "chat" -and
            [string]$main.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
            [string]$main.social_chat.web_chat_state -eq "ready" -and
            $main.social_chat.web_chat_streaming -ne $true -and
            $mainMessages.Count -ge ($InitialMainMessageCount + 2) -and
            [string]$lastMainMessage.role -eq "friend"
        if ($mainCompleted) {
            $adapter = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
            $adapterMessages = @($adapter.conversation.messages)
            $lastAdapterMessageCount = $adapterMessages.Count
            $lastAdapterMessage = $adapterMessages | Select-Object -Last 1
            if (
                [string]$adapter.surface -eq "chatgpt_web" -and
                [string]$adapter.bridge_state -eq "ready" -and
                $adapter.adapter_current -eq $true -and
                $adapter.streaming -ne $true -and
                $adapterMessages.Count -ge ($InitialAdapterMessageCount + 2) -and
                [string]$lastAdapterMessage.role -eq "assistant"
            ) {
                return $adapter
            }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT tool completion: $ToolId; " +
        "provider_state=$lastMainState main_messages=$lastMainMessageCount " +
        "adapter_messages=$lastAdapterMessageCount"
}

function Get-ToolOption {
    $lastError = ""
    foreach ($attempt in 1..3) {
        try {
            Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
                -ExpectedAction "list_composer_tools" `
                -Arguments @{ section = "tools" } `
                -TimeoutSec ([Math]::Min($ReadyTimeoutSec, 45)) | Out-Null
            $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
                -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
            $option = @($navigation.composer_sections.tools) |
                Where-Object { [string]$_.semantic -eq [string]$toolSpec.semantic } |
                Select-Object -First 1
            if ($null -ne $option) { return $option }
        } catch {
            $lastError = ConvertTo-ChatGptWebSmokeSafeDiagnostic `
                -Value $_.Exception.Message -MaxLength 120
        }
        if ($attempt -lt 3) {
            Close-ComposerMenu
            Start-Sleep -Seconds $runtime.poll_interval_sec
        }
    }
    $suffix = if ($lastError) { "; last_error=$lastError" } else { "" }
    throw "Requested ChatGPT composer tool was not observed after bounded refresh: $ToolId$suffix"
}

function Wait-ToolStructuralReply {
    $expectedPartTypes = @($toolSpec.expected_parts)
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Min($ReplyTimeoutSec, 180))
    $observedPartTypes = @()
    do {
        if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) {
            throw "ChatGPT tool execution lost the production chat foreground while awaiting rich output."
        }
        $replyState = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if (
            [string]$replyState.surface -ne "chatgpt_web" -or
            [string]$replyState.bridge_state -ne "ready" -or
            $replyState.adapter_current -ne $true
        ) {
            Start-Sleep -Seconds ([Math]::Max(2, $runtime.poll_interval_sec))
            continue
        }
        $messageCount = [int]$replyState.conversation.message_count
        if ($messageCount -lt 1) {
            Start-Sleep -Seconds ([Math]::Max(2, $runtime.poll_interval_sec))
            continue
        }
        $context = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_get_context" -Arguments @{
                message_offset = [Math]::Max(0, $messageCount - 1)
                message_limit = 1
            }
        $assistant = @($context.messages) | Select-Object -Last 1
        $observedPartTypes = @(
            $assistant.parts |
                ForEach-Object { [string]$_.type } |
                Where-Object { $_ } |
                Sort-Object -Unique
        )
        $matchingPartCount = @(
            $observedPartTypes | Where-Object { $_ -in $expectedPartTypes }
        ).Count
        if ($expectedPartTypes.Count -eq 0 -or $matchingPartCount -gt 0) {
            return [pscustomobject]@{
                reply = $replyState
                observed_part_types = $observedPartTypes
                matching_part_count = $matchingPartCount
            }
        }
        Start-Sleep -Seconds ([Math]::Max(2, $runtime.poll_interval_sec))
    } while ([DateTimeOffset]::UtcNow -lt $deadline)

    $safeObserved = if ($observedPartTypes.Count) {
        $observedPartTypes -join ","
    } else {
        "none"
    }
    throw "ChatGPT tool reply did not expose the expected structural output for $ToolId; " +
        "observed_parts=$safeObserved"
}

function Close-ComposerMenu {
    Invoke-ReceiptAction -Action "chatgpt_dismiss_composer_options" `
        -ExpectedAction "dismiss_composer_menu" `
        -TimeoutSec ([Math]::Min($ReadyTimeoutSec, 30)) | Out-Null
}

function Restore-ToolSelection {
    if (-not $enabledBySmoke) { return }
    $active = Get-ToolOption
    if ($active.selected -eq $true) {
        Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_composer_tool" -Arguments @{
                section = "tools"
                option_id = [string]$active.id
            } | Out-Null
    } else {
        Close-ComposerMenu
    }
}

function Restore-Origin {
    param(
        [AllowEmptyString()][string]$ConversationPath,
        [AllowEmptyString()][string]$InputText
    )

    $restored = if ($ConversationPath) {
        Restore-WebChatNativeConversation -Runtime $runtime `
            -ProviderId "chatgpt_web" -ConversationPath $ConversationPath `
            -TimeoutSec ([Math]::Min($ReadyTimeoutSec, 120))
    } else {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "start_new_web_chat_conversation" | Out-Null
        $true
    }
    if (-not $restored) { return $false }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "set_input_text" -Arguments @{ text = $InputText } | Out-Null
    return $true
}

$result = $null
$originPath = ""
$originInputText = ""
$originRestored = $false
$enabledBySmoke = $false
$toolStateRestored = $false
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $originMain = Open-WebChatNativeChatSurface -Runtime $runtime `
        -ProviderId "chatgpt_web" -TimeoutSec $ReadyTimeoutSec
    $origin = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    $originPath = [string]$originMain.social_chat.web_chat_conversation_path
    $originInputText = [string]$originMain.input.text

    Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "start_new_web_chat_conversation" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec -MainState `
        -Description "isolated blank production tool conversation" -Predicate {
            param($state)
            [string]$state.social_chat.web_chat_provider_id -eq "chatgpt_web" -and
                [string]$state.social_chat.web_chat_state -eq "ready" -and
                $state.social_chat.web_chat_composer_ready -eq $true -and
                [int]$state.social_chat.message_count -eq 0
        } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "isolated blank tool conversation" -Predicate {
            param($state)
            [string]$state.surface -eq "chatgpt_web" -and
                [string]$state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                [int]$state.conversation.message_count -eq 0 -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false -and
                [int]$state.input.official_draft_length -eq 0
        } | Out-Null

    $tool = Get-ToolOption
    $enabledBySmoke = $tool.selected -ne $true
    if ($enabledBySmoke) {
        Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_composer_tool" -Arguments @{
                section = "tools"
                option_id = [string]$tool.id
            } | Out-Null
    }
    $selected = Get-ToolOption
    if ($selected.selected -ne $true) { throw "Requested ChatGPT composer tool was not selected: $ToolId" }
    Close-ComposerMenu

    $marker = "ELON-CHATGPT-TOOL-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    $prompt = ([string]$toolSpec.prompt).Replace("{marker}", $marker)
    $beforeMain = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" -MainState
    if (
        [string]$beforeMain.active_surface -ne "social_ai" -or
        [string]$beforeMain.social_chat.interaction_mode -ne "chat" -or
        [string]$beforeMain.social_chat.web_chat_provider_id -ne "chatgpt_web"
    ) {
        throw "ChatGPT production tool surface changed before the prompt could be sent."
    }
    $previousReceipt = $beforeMain.social_chat.web_chat_last_send_command
    $previousReceiptAtMs = if ($null -ne $previousReceipt) {
        [long]$previousReceipt.observed_at_ms
    } else {
        0L
    }
    $beforeSend = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
        -Arguments @{ text = $prompt } | Out-Null
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input" | Out-Null
    Wait-ProductionSendReceipt -AfterObservedAtMs $previousReceiptAtMs | Out-Null
    $reply = Wait-ToolReply `
        -InitialMainMessageCount ([int]$beforeMain.social_chat.message_count) `
        -InitialAdapterMessageCount ([int]$beforeSend.conversation.message_count)
    $structuralReply = Wait-ToolStructuralReply
    $reply = $structuralReply.reply
    $observedPartTypes = @($structuralReply.observed_part_types)
    $matchingPartCount = [int]$structuralReply.matching_part_count

    Restore-ToolSelection
    $toolStateRestored = $true
    $originRestored = Restore-Origin -ConversationPath $originPath `
        -InputText $originInputText
    if (-not $originRestored) {
        throw "Unable to restore the original ChatGPT production conversation."
    }
    $result = [ordered]@{
        schema = "elon.chatgpt_web.tool_execution_acceptance.v3"
        passed = $true
        tool_id = $ToolId
        tool_semantic = [string]$toolSpec.semantic
        adapter_version = [int]$reply.adapter_version
        isolated_conversation = $true
        tool_selection_observed = $true
        send_receipt_observed = $true
        production_send_receipt_observed = $true
        assistant_completed = $true
        expected_structural_part_types = @($toolSpec.expected_parts)
        observed_structural_part_types = $observedPartTypes
        matching_structural_part_count = $matchingPartCount
        tool_state_restored = $toolStateRestored
        original_conversation_restored = $true
        production_surface_preserved = Test-ChatGptWebSmokeActivityForeground -Runtime $runtime
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    }
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "restored ChatGPT bridge before evidence registration" -Predicate {
            param($state)
            [string]$state.surface -eq "chatgpt_web" -and
                [string]$state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true
        } | Out-Null
    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @([string]$toolSpec.case_id) `
        -ExpectedAdapterVersion $ExpectedAdapterVersion -ProductionSurface | Out-Null
} finally {
    if (-not $toolStateRestored -and $enabledBySmoke) {
        try {
            Restore-ToolSelection
            $toolStateRestored = $true
        } catch {
            Write-Warning "Unable to restore the original ChatGPT tool selection after a failed smoke."
        }
    }
    if (-not $originRestored) {
        try {
            $originRestored = Restore-Origin -ConversationPath $originPath `
                -InputText $originInputText
        } catch {
            Write-Warning "Unable to restore the original ChatGPT view after a failed tool smoke."
        }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

$result | ConvertTo-Json -Depth 4
Write-Output "CHATGPT_WEB_TOOL_EXECUTION_ACCEPTANCE=passed"
