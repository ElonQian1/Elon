#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [switch]$SkipDictation,
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 60,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 67
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

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

Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
    -EnsureMainActivity | Out-Null
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
$originViewMode = [string]$origin.view_mode

$dictationResult = [ordered]@{ skipped = $true; reason = "user_assisted_audio_capture" }
if (-not $SkipDictation) {
    $dictationStart = Invoke-ReceiptAction -Action "chatgpt_start_dictation" `
        -ExpectedAction "start_dictation"
    $active = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "active ChatGPT dictation" -RequireChatGptForeground `
        -Predicate { param($state) $state.dictation_active -eq $true }
    $dictationCancel = Invoke-ReceiptAction -Action "chatgpt_cancel_dictation" `
        -ExpectedAction "cancel_dictation"
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

Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
    -Arguments @{ view_mode = "native" } | Out-Null

$search = Get-WebSearchOption
$initialSearchSelected = $search.selected -eq $true
$restoreSearch = $false
try {
    $toggleOn = Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
        -ExpectedAction "select_composer_tool" -Arguments @{
            section = "tools"
            option_id = [string]$search.id
        }
    $restoreSearch = $true
    $toggled = Get-WebSearchOption
    if (($toggled.selected -eq $true) -eq $initialSearchSelected) {
        throw "ChatGPT web search selection did not toggle."
    }
    $toggleOff = Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
        -ExpectedAction "select_composer_tool" -Arguments @{
            section = "tools"
            option_id = [string]$toggled.id
        }
    $restoreSearch = $false
    $restored = Get-WebSearchOption
    if (($restored.selected -eq $true) -ne $initialSearchSelected) {
        throw "ChatGPT web search selection was not restored."
    }
} finally {
    if ($restoreSearch) {
        try {
            $current = Get-WebSearchOption
            if (($current.selected -eq $true) -ne $initialSearchSelected) {
                Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
                    -ExpectedAction "select_composer_tool" -Arguments @{
                        section = "tools"
                        option_id = [string]$current.id
                    } | Out-Null
            }
        } catch {
            Write-Warning "ChatGPT web search test state could not be restored automatically."
        }
    }
}

if ($originViewMode -in @("web", "native")) {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = $originViewMode } | Out-Null
}

[ordered]@{
    schema = "elon.chatgpt_web.composer_control_smoke.v1"
    passed = $true
    device_serial = $DeviceSerial
    sent_messages = 0
    uploaded_attachments = 0
    dictation = $dictationResult
    web_search = [ordered]@{
        enable_receipt = [string]$toggleOn.receipt.status
        toggled = $true
        disable_receipt = [string]$toggleOff.receipt.status
        original_state_restored = $true
    }
    original_view_mode_restored = $true
} | ConvertTo-Json -Depth 10
Write-Output "CHATGPT_WEB_COMPOSER_CONTROL_SMOKE_STATUS=passed"
