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
        [hashtable]$Arguments = @{}
    )

    $dispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action $Action -Arguments $Arguments -TimeoutSec $ReadyTimeoutSec
    $requestId = [string]$dispatch.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action." }
    return Wait-ChatGptCommandReceipt `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $requestId -ExpectedAction $ExpectedAction `
        -TimeoutSec $ReadyTimeoutSec -PollIntervalSec $PollIntervalSec
}

function Wait-ToolReply {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][long]$AfterMs,
        [Parameter(Mandatory = $true)][int]$InitialMessageCount
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReplyTimeoutSec)
    $lastReceipt = $null
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $lastReceipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $lastReceipt -and [string]$lastReceipt.status -eq "failed") {
            throw "ChatGPT tool prompt failed: $ToolId"
        }
        $messages = @($state.conversation.messages)
        $lastMessage = $messages | Select-Object -Last 1
        if (
            $null -ne $lastReceipt -and
            [string]$lastReceipt.expected_web_action -eq "send_prompt" -and
            [string]$lastReceipt.status -eq "succeeded" -and
            $lastReceipt.result.ok -eq $true -and
            [long]$lastReceipt.completed_at_ms -gt $AfterMs -and
            $state.streaming -eq $false -and
            $messages.Count -ge ($InitialMessageCount + 2) -and
            [string]$lastMessage.role -eq "assistant"
        ) {
            return $state
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT tool completion: $ToolId; receipt=$($lastReceipt.status)"
}

function Get-ToolOption {
    Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
        -ExpectedAction "list_composer_tools" `
        -Arguments @{ section = "tools" } | Out-Null
    $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
    $option = @($navigation.composer_sections.tools) |
        Where-Object { [string]$_.semantic -eq [string]$toolSpec.semantic } |
        Select-Object -First 1
    if ($null -eq $option) { throw "Requested ChatGPT composer tool is unavailable: $ToolId" }
    return $option
}

function Close-ComposerMenu {
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") `
        -TimeoutSec 5 -Label "close ChatGPT composer tool menu" | Out-Null
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
        [Parameter(Mandatory = $true)][string]$ViewMode
    )

    if ($ConversationPath) {
        Invoke-ReceiptAction -Action "chatgpt_open_conversation" `
            -ExpectedAction "open_conversation" `
            -Arguments @{ conversation_path = $ConversationPath } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "original ChatGPT conversation restoration" -Predicate {
                param($state)
                [string]$state.conversation.url -like "*$ConversationPath*" -and
                    $state.bridge_state -eq "ready"
            }.GetNewClosure() | Out-Null
    } else {
        Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
            -ExpectedAction "new_conversation" | Out-Null
    }
    if ($ViewMode -in @("web", "native")) {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = $ViewMode } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "original ChatGPT view mode restoration" -Predicate {
                param($state)
                [string]$state.view_mode -eq $ViewMode
            }.GetNewClosure() | Out-Null
    }
}

$result = $null
$originPath = ""
$originMode = ""
$originRestored = $false
$enabledBySmoke = $false
$toolStateRestored = $false
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -InitialWaitSec 20
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    $originMode = [string]$origin.view_mode
    $originPath = [regex]::Match(
        [string]$origin.conversation.url,
        '/c/[A-Za-z0-9_-]{1,160}'
    ).Value

    Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
        -ExpectedAction "new_conversation" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "isolated blank tool conversation" -Predicate {
            param($state)
            [int]$state.conversation.message_count -eq 0 -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false
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
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
        -Arguments @{ text = $prompt } | Out-Null
    $beforeSend = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    $send = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"
    $sendRequestId = [string]$send.command_receipt.request_id
    if (-not $sendRequestId) { throw "ChatGPT tool prompt did not return a receipt id." }
    $reply = Wait-ToolReply -RequestId $sendRequestId `
        -AfterMs ([long]$beforeSend.last_command.observed_at_ms) `
        -InitialMessageCount ([int]$beforeSend.conversation.message_count)
    $context = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_context" -Arguments @{
            message_offset = [Math]::Max(0, [int]$reply.conversation.message_count - 1)
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
        $observedPartTypes | Where-Object { $_ -in @($toolSpec.expected_parts) }
    ).Count
    if (@($toolSpec.expected_parts).Count -gt 0 -and $matchingPartCount -lt 1) {
        throw "ChatGPT tool reply did not expose the expected structural output for $ToolId."
    }

    Restore-ToolSelection
    $toolStateRestored = $true
    Restore-Origin -ConversationPath $originPath -ViewMode $originMode
    $originRestored = $true
    $result = [ordered]@{
        schema = "elon.chatgpt_web.tool_execution_acceptance.v2"
        passed = $true
        tool_id = $ToolId
        tool_semantic = [string]$toolSpec.semantic
        adapter_version = [int]$reply.adapter_version
        isolated_conversation = $true
        tool_selection_observed = $true
        send_receipt_observed = $true
        assistant_completed = $true
        expected_structural_part_types = @($toolSpec.expected_parts)
        observed_structural_part_types = $observedPartTypes
        matching_structural_part_count = $matchingPartCount
        tool_state_restored = $toolStateRestored
        original_conversation_restored = $true
        original_view_mode_restored = $true
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    }
    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @([string]$toolSpec.case_id) `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
} finally {
    if (-not $toolStateRestored -and $enabledBySmoke) {
        try {
            Restore-ToolSelection
            $toolStateRestored = $true
        } catch {
            Write-Warning "Unable to restore the original ChatGPT tool selection after a failed smoke."
        }
    }
    if (-not $originRestored -and $originMode) {
        try {
            Restore-Origin -ConversationPath $originPath -ViewMode $originMode
        } catch {
            Write-Warning "Unable to restore the original ChatGPT view after a failed tool smoke."
        }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

$result | ConvertTo-Json -Depth 4
Write-Output "CHATGPT_WEB_TOOL_EXECUTION_ACCEPTANCE=passed"
