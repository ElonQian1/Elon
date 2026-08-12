#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 300)][int]$ReadyTimeoutSec = 120,
    [ValidateRange(30, 600)][int]$ReplyTimeoutSec = 240,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 65
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

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

function Get-WebSearchOption {
    Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
        -ExpectedAction "list_composer_tools" `
        -Arguments @{ section = "tools" } | Out-Null
    $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
    $option = @($navigation.composer_sections.tools) |
        Where-Object { [string]$_.semantic -eq "web_search" } |
        Select-Object -First 1
    if ($null -eq $option) { throw "ChatGPT web search tool is unavailable." }
    return $option
}

function Close-ComposerMenu {
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") `
        -TimeoutSec 5 -Label "close ChatGPT composer tool menu" | Out-Null
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

    $search = Get-WebSearchOption
    $enabledBySmoke = $search.selected -ne $true
    if ($enabledBySmoke) {
        Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_composer_tool" -Arguments @{
                section = "tools"
                option_id = [string]$search.id
            } | Out-Null
    }
    $selected = Get-WebSearchOption
    if ($selected.selected -ne $true) { throw "ChatGPT web search tool was not selected." }
    Close-ComposerMenu

    $marker = "ELON-CHATGPT-TOOL-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
        -Arguments @{
            text = "Use web search for the official OpenAI homepage and include $marker in the answer."
        } | Out-Null
    $beforeSend = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    $send = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"
    $sendRequestId = [string]$send.command_receipt.request_id
    if (-not $sendRequestId) { throw "ChatGPT tool prompt did not return a receipt id." }
    $reply = Wait-ChatGptProbeReply `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $sendRequestId -Marker $marker `
        -AfterMs ([long]$beforeSend.last_command.observed_at_ms) `
        -TimeoutSec $ReplyTimeoutSec -PollIntervalSec $PollIntervalSec
    $context = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_context" -Arguments @{
            message_offset = [Math]::Max(0, [int]$reply.conversation.message_count - 1)
            message_limit = 1
        }
    $assistant = @($context.messages) | Select-Object -Last 1
    $citationCount = @(
        $assistant.parts | Where-Object { [string]$_.type -eq "citation" }
    ).Count
    if ($citationCount -lt 1) { throw "ChatGPT tool reply did not expose citation structure." }

    if ($enabledBySmoke) {
        $active = Get-WebSearchOption
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
    Restore-Origin -ConversationPath $originPath -ViewMode $originMode
    $originRestored = $true
    $result = [ordered]@{
        schema = "elon.chatgpt_web.tool_execution_acceptance.v1"
        passed = $true
        adapter_version = [int]$reply.adapter_version
        isolated_conversation = $true
        tool_selection_observed = $true
        send_receipt_observed = $true
        assistant_completed = $true
        citation_count = $citationCount
        tool_state_restored = $true
        original_conversation_restored = $true
        original_view_mode_restored = $true
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    }
} finally {
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
