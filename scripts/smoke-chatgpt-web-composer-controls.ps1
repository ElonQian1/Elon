#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [switch]$SkipDictation,
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 60,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Wait-CommandReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $receipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $receipt -and [string]$receipt.status -eq "failed") {
            throw "ChatGPT command failed: $ExpectedAction"
        }
        if (
            $null -ne $receipt -and
            [string]$receipt.status -eq "succeeded" -and
            [string]$receipt.expected_web_action -eq $ExpectedAction -and
            $receipt.result.ok -eq $true
        ) {
            return [pscustomobject]@{ state = $state; receipt = $receipt }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT command: $ExpectedAction"
}

function Invoke-ReceiptAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{}
    )

    $dispatched = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments -TimeoutSec $ReadyTimeoutSec
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
}

function Get-ComposerTools {
    Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
        -ExpectedAction "list_composer_tools" -Arguments @{ section = "tools" } | Out-Null
    $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
    return @($navigation.composer_sections.tools | Where-Object { $null -ne $_ })
}

function Get-WebSearchOption {
    $option = @(Get-ComposerTools) |
        Where-Object { [string]$_.semantic -eq "web_search" } |
        Select-Object -First 1
    if ($null -eq $option) { throw "ChatGPT web search tool is unavailable." }
    return $option
}

$composerToolDiscoveryCases = [ordered]@{
    deep_research = "reversible/composer_tool_discovery/deep_research"
    image_generation = "reversible/composer_tool_discovery/image_generation"
    canvas = "reversible/composer_tool_discovery/canvas"
    study = "reversible/composer_tool_discovery/study_mode"
    agent = "reversible/composer_tool_discovery/agent_mode"
}

Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
$origin = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
    -Description "authenticated ChatGPT composer" -Predicate {
        param($state)
        $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.authenticated -eq $true -and
            $state.composer_ready -eq $true -and
            (
                $SkipDictation -or
                $state.dictation_active -eq $true -or
                @($state.ui_manifest.controls | Where-Object {
                    [string]$_.semantic -eq "dictation" -and $_.enabled -eq $true
                }).Count -gt 0
            )
    }
Assert-ChatGptWebSmokeAdapterVersion -State $origin `
    -ExpectedAdapterVersion $ExpectedAdapterVersion

$originConversationPath = Get-ChatGptWebSmokeConversationPath `
    -Url ([string]$origin.conversation.url)
if ([int]$origin.input.text_length -gt 0) {
    throw "Composer control smoke will not replace a non-empty ChatGPT draft."
}
if (-not $originConversationPath -and [string]$origin.page_kind -ne "home") {
    throw "The current ChatGPT page has no safe MCP restoration route."
}
if ($origin.dictation_active -eq $true -or $origin.streaming -eq $true) {
    throw "Composer control smoke will not interrupt active ChatGPT work."
}

$dictationResult = [ordered]@{ skipped = $true; reason = "user_assisted_audio_capture" }
$isolationRequested = $false
$originRestored = $false
$dictationStarted = $false
$restoreSearch = $false
try {
    $isolationRequested = $true
    $isolation = Start-ChatGptWebSmokeIsolatedConversation -Runtime $runtime `
        -OriginState $origin -TimeoutSec $ReadyTimeoutSec
    Assert-ChatGptWebSmokeAdapterVersion -State $isolation.isolated_state `
        -ExpectedAdapterVersion $ExpectedAdapterVersion

    if (-not $SkipDictation) {
        $dictationStart = Invoke-ReceiptAction -Action "chatgpt_start_dictation" `
            -ExpectedAction "start_dictation"
        $dictationStarted = $true
        $active = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "active ChatGPT dictation" -RequireChatGptForeground `
            -Predicate { param($state) $state.dictation_active -eq $true }
        $dictationCancel = Invoke-ReceiptAction -Action "chatgpt_cancel_dictation" `
            -ExpectedAction "cancel_dictation"
        $dictationStarted = $false
        $inactive = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "stopped ChatGPT dictation" -RequireChatGptForeground `
            -Predicate { param($state) $state.dictation_active -eq $false }
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
            -Arguments @{ text = "" } | Out-Null
        $dictationResult = [ordered]@{
            skipped = $false
            start_receipt = [string]$dictationStart.receipt.status
            active = [bool]$active.dictation_active
            cancel_receipt = [string]$dictationCancel.receipt.status
            stopped = -not [bool]$inactive.dictation_active
            input_cleared = $true
        }
    }

    $search = Get-WebSearchOption
    if ($search.selected -eq $true) {
        throw "The isolated ChatGPT conversation inherited an active web search tool."
    }
    try {
        $toggleOn = Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_composer_tool" -Arguments @{
                section = "tools"
                option_id = [string]$search.id
            }
        $restoreSearch = $true
        $toggled = Get-WebSearchOption
        if ($toggled.selected -ne $true) {
            throw "ChatGPT web search selection did not turn on."
        }
        $toggleOff = Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_composer_tool" -Arguments @{
                section = "tools"
                option_id = [string]$toggled.id
            }
        $restoreSearch = $false
        $restored = Get-WebSearchOption
        if ($restored.selected -eq $true) {
            throw "ChatGPT web search selection did not turn off."
        }
    } finally {
        if ($restoreSearch) {
            try {
                $current = Get-WebSearchOption
                if ($current.selected -eq $true) {
                    Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
                        -ExpectedAction "select_composer_tool" -Arguments @{
                            section = "tools"
                            option_id = [string]$current.id
                        } | Out-Null
                }
            } catch {
                Write-Warning "ChatGPT isolated web search state could not be reset automatically."
            }
        }
    }

    $observedTools = @(Get-ComposerTools)
    $observedToolSemantics = @(
        $observedTools |
            ForEach-Object { [string]$_.semantic } |
            Where-Object { $composerToolDiscoveryCases.Contains($_) } |
            Sort-Object -Unique
    )
} finally {
    if ($dictationStarted) {
        try {
            Invoke-ReceiptAction -Action "chatgpt_cancel_dictation" `
                -ExpectedAction "cancel_dictation" | Out-Null
        } catch {
            Write-Warning "ChatGPT isolated dictation could not be cancelled automatically."
        }
    }
    if ($isolationRequested) {
        Restore-ChatGptWebSmokeOrigin -Runtime $runtime `
            -ConversationPath $originConversationPath `
            -TimeoutSec $ReadyTimeoutSec | Out-Null
        $originRestored = $true
    }
}

$discoveryCases = @(
    $observedToolSemantics | ForEach-Object { $composerToolDiscoveryCases[$_] }
)

Register-ChatGptWebVerificationCases -Runtime $runtime `
    -CaseIds (@("reversible/composer_controls") + $discoveryCases) `
    -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null

[ordered]@{
    schema = "elon.chatgpt_web.composer_control_smoke.v2"
    passed = $true
    device_serial = $DeviceSerial
    sent_messages = 0
    uploaded_attachments = 0
    isolated_conversation = $true
    origin_restored = [bool]$originRestored
    dictation = $dictationResult
    web_search = [ordered]@{
        enable_receipt = [string]$toggleOn.receipt.status
        toggled = $true
        disable_receipt = [string]$toggleOff.receipt.status
        original_state_restored = $true
    }
    composer_tool_discovery = [ordered]@{
        observed_count = $observedToolSemantics.Count
        observed_ids = @(
            $composerToolDiscoveryCases.Keys |
                Where-Object { $_ -in $observedToolSemantics } |
                ForEach-Object {
                    switch ($_) {
                        "study" { "study_mode" }
                        "agent" { "agent_mode" }
                        default { $_ }
                    }
                }
        )
        executed_tools = 0
    }
    production_surface_preserved = Test-ChatGptWebSmokeActivityForeground -Runtime $runtime
} | ConvertTo-Json -Depth 10
Write-Output "CHATGPT_WEB_COMPOSER_CONTROL_SMOKE_STATUS=passed"
